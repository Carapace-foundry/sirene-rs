use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use sirene_core::{CodeNaf, Siren, Siret};
use sirene_index::SireneIndex;
use sirene_matcher::{LocationQuery, SearchQuery};
use sirene_normalize::Normalizer;

use crate::args::ServeArgs;
use crate::error::{CliError, Result};

// ============================================================================
// DINUM / INSEE Compliant REST JSON Models
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct DinumSiege {
    pub siret: Option<String>,
    pub code_postal: Option<String>,
    pub code_commune: Option<String>,
    pub etat_administratif: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DinumSearchResult {
    pub siren: String,
    pub nom_complet: String,
    pub nom_raison_sociale: String,
    pub sigle: Option<String>,
    pub activite_principale: Option<String>,
    pub etat_administratif: String,
    pub siege: DinumSiege,
    pub dirigeants: Vec<String>,
    pub matching_score: f32,
    pub is_exact_pivot: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DinumSearchResponse {
    pub results: Vec<DinumSearchResult>,
    pub total_results: usize,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DinumEnterpriseResponse {
    pub siren: String,
    pub nom_complet: String,
    pub nom_raison_sociale: String,
    pub activite_principale: Option<String>,
    pub etat_administratif: String,
    pub siege: DinumSiege,
    pub dirigeants: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DinumEtablissementResponse {
    pub siret: String,
    pub siren: String,
    pub nom_complet: String,
    pub activite_principale: Option<String>,
    pub etat_administratif: String,
    pub code_postal: Option<String>,
    pub code_commune: Option<String>,
    pub est_siege: bool,
    pub dirigeants: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub num_documents: u32,
}

// ============================================================================
// Lightweight HTTP Engine
// ============================================================================

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub query_params: HashMap<String, String>,
}

#[derive(Debug)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: &'static str,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn json(status_code: u16, status_text: &'static str, body: String) -> Self {
        Self {
            status_code,
            status_text,
            content_type: "application/json; charset=utf-8",
            body: body.into_bytes(),
        }
    }

    pub fn ok_json<T: Serialize>(value: &T) -> Self {
        match serde_json::to_string(value) {
            Ok(json) => Self::json(200, "OK", json),
            Err(e) => Self::internal_error(&format!("JSON serialization error: {}", e)),
        }
    }

    pub fn bad_request(message: &str) -> Self {
        let err = serde_json::json!({
            "error": "Bad Request",
            "message": message,
        });
        Self::json(400, "Bad Request", err.to_string())
    }

    pub fn not_found(message: &str) -> Self {
        let err = serde_json::json!({
            "error": "Not Found",
            "message": message,
        });
        Self::json(404, "Not Found", err.to_string())
    }

    pub fn method_not_allowed() -> Self {
        let err = serde_json::json!({
            "error": "Method Not Allowed",
            "message": "Only GET and OPTIONS methods are allowed on this endpoint",
        });
        Self::json(405, "Method Not Allowed", err.to_string())
    }

    pub fn internal_error(message: &str) -> Self {
        let err = serde_json::json!({
            "error": "Internal Server Error",
            "message": message,
        });
        Self::json(500, "Internal Server Error", err.to_string())
    }

    pub fn cors_preflight() -> Self {
        Self {
            status_code: 204,
            status_text: "No Content",
            content_type: "text/plain",
            body: Vec::new(),
        }
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let header = format!(
            "HTTP/1.1 {} {}\r\n\
             Content-Type: {}\r\n\
             Content-Length: {}\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Access-Control-Allow-Methods: GET, OPTIONS\r\n\
             Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
             Connection: close\r\n\
             \r\n",
            self.status_code,
            self.status_text,
            self.content_type,
            self.body.len(),
        );
        writer.write_all(header.as_bytes())?;
        if !self.body.is_empty() {
            writer.write_all(&self.body)?;
        }
        writer.flush()
    }
}

/// Decodes URL percent-encoded UTF-8 strings (`%20`, `+`, `%C3%A9`, etc.) with length safety.
pub fn percent_decode(input: &str) -> String {
    const MAX_INPUT_LEN: usize = 4096;
    let slice = if input.len() > MAX_INPUT_LEN {
        &input[..MAX_INPUT_LEN]
    } else {
        input
    };
    let bytes = slice.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
            {
                decoded.push(val);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            decoded.push(b' ');
            i += 1;
            continue;
        }
        decoded.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// Safely reads a line from a buffered reader up to `max_len` bytes, preventing Unbounded Read DoS.
fn read_bounded_line<R: BufRead>(
    reader: &mut R,
    buf: &mut String,
    max_len: usize,
) -> std::io::Result<usize> {
    buf.clear();
    let mut total = 0;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(total);
        }
        let (consumed, done) = match available.iter().position(|&b| b == b'\n') {
            Some(pos) => (pos + 1, true),
            None => (available.len(), false),
        };
        if total + consumed > max_len {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "HTTP line exceeds maximum allowed length (Line Bomb protection)",
            ));
        }
        let s = std::str::from_utf8(&available[..consumed])
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        buf.push_str(s);
        reader.consume(consumed);
        total += consumed;
        if done {
            return Ok(total);
        }
    }
}

/// Parses key-value query parameters from a raw query string.
pub fn parse_query_params(query: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let val = parts.next().unwrap_or("");
        let decoded_key = percent_decode(key);
        let decoded_val = percent_decode(val);
        params.insert(decoded_key, decoded_val);
    }
    params
}

/// Parses an incoming HTTP 1.1 request from a buffered stream with strict DoS protection.
pub fn parse_http_request<R: BufRead>(reader: &mut R) -> std::io::Result<Option<HttpRequest>> {
    const MAX_LINE_LEN: usize = 8192;
    const MAX_HEADER_COUNT: usize = 100;
    const MAX_TOTAL_HEADERS_LEN: usize = 32768;

    let mut line = String::new();
    let bytes_read = read_bounded_line(reader, &mut line, MAX_LINE_LEN)?;
    if bytes_read == 0 {
        return Ok(None);
    }

    let line_trimmed = line.trim_end_matches(['\r', '\n']);
    let mut parts = line_trimmed.split_whitespace();
    let method = match parts.next() {
        Some(m) => m.to_uppercase(),
        None => return Ok(None),
    };
    let uri = match parts.next() {
        Some(u) => u.to_string(),
        None => return Ok(None),
    };

    // Consume remainder of headers safely (bounded count and total length)
    let mut header_count = 0;
    let mut total_headers_bytes = 0;
    loop {
        header_count += 1;
        if header_count > MAX_HEADER_COUNT {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Too many HTTP headers (Header Flood protection)",
            ));
        }
        let h_len = read_bounded_line(reader, &mut line, MAX_LINE_LEN)?;
        total_headers_bytes += h_len;
        if total_headers_bytes > MAX_TOTAL_HEADERS_LEN {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Total HTTP headers size exceeded (Header Size protection)",
            ));
        }
        if h_len == 0 || line == "\r\n" || line == "\n" {
            break;
        }
    }

    let mut uri_parts = uri.splitn(2, '?');
    let raw_path = uri_parts.next().unwrap_or("/");
    let path = percent_decode(raw_path);
    let query_str = uri_parts.next().unwrap_or("");
    let query_params = parse_query_params(query_str);

    Ok(Some(HttpRequest {
        method,
        path,
        query_params,
    }))
}

// ============================================================================
// Core Request Handler (DINUM / INSEE API routes)
// ============================================================================

/// Routes and executes an incoming HTTP request against the binary SIRENE index.
pub fn handle_request(req: &HttpRequest, index: &SireneIndex) -> HttpResponse {
    if req.method == "OPTIONS" {
        return HttpResponse::cors_preflight();
    }

    if req.method != "GET" {
        return HttpResponse::method_not_allowed();
    }

    // Normalize path: strip trailing slash
    let mut path = req.path.as_str();
    if path.len() > 1 && path.ends_with('/') {
        path = &path[..path.len() - 1];
    }

    // Route: Health & Status
    if path.is_empty() || path == "/" || path == "/health" || path == "/status" {
        let health = HealthResponse {
            status: "ok".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            num_documents: index.num_documents(),
        };
        return HttpResponse::ok_json(&health);
    }

    // Route: DINUM / API Recherche Entreprises Search
    if path == "/search" || path == "/v3/search" || path == "/matching_etablissements" {
        let query_str = match req
            .query_params
            .get("q")
            .or_else(|| req.query_params.get("query"))
            .map(|s| s.trim())
        {
            Some(q) if !q.is_empty() => q,
            _ => return HttpResponse::bad_request("Le paramètre de recherche 'q' est requis."),
        };

        let location_str = req
            .query_params
            .get("code_postal")
            .or_else(|| req.query_params.get("cp"))
            .or_else(|| req.query_params.get("departement"))
            .or_else(|| req.query_params.get("dep"))
            .or_else(|| req.query_params.get("commune"))
            .or_else(|| req.query_params.get("code_commune"))
            .or_else(|| req.query_params.get("postcode"))
            .map(|s| s.as_str());

        let naf_str = req
            .query_params
            .get("naf")
            .or_else(|| req.query_params.get("activite_principale"))
            .or_else(|| req.query_params.get("code_naf"))
            .map(|s| s.as_str());

        let dirigeant_str = req
            .query_params
            .get("dirigeant")
            .or_else(|| req.query_params.get("nom_personne"))
            .or_else(|| req.query_params.get("representant"))
            .map(|s| s.as_str());

        let page: usize = req
            .query_params
            .get("page")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1)
            .max(1);

        let per_page: usize = req
            .query_params
            .get("per_page")
            .or_else(|| req.query_params.get("limite"))
            .or_else(|| req.query_params.get("limit"))
            .or_else(|| req.query_params.get("size"))
            .and_then(|s| s.parse().ok())
            .unwrap_or(10)
            .clamp(1, 100);

        let etat_filter = req
            .query_params
            .get("etat_administratif")
            .map(|s| s.trim().to_ascii_uppercase());

        let strict_location = req
            .query_params
            .get("strict_location")
            .map(|s| s == "true" || s == "1")
            .unwrap_or(false);

        let normalizer = Normalizer::new();
        let norm_query = normalizer.normalize_query(query_str);
        let location = location_str.and_then(LocationQuery::parse);
        let naf = naf_str.and_then(|s| CodeNaf::parse(s.trim()).ok());

        let search_query = SearchQuery {
            query: &norm_query,
            location,
            naf,
            dirigeant: dirigeant_str,
            strict_location,
        };

        // Query enough candidates to fulfill pagination (up to 200)
        let fetch_limit = (page.saturating_mul(per_page)).clamp(per_page, 200);
        let results = index.search(&search_query, fetch_limit);

        let dinum_results: Vec<DinumSearchResult> = results
            .into_iter()
            .filter_map(|res| {
                let view = index.get_document(res.doc_id);
                let etat = view
                    .as_ref()
                    .map(|v| v.etat_administratif.to_string())
                    .unwrap_or_else(|| "A".to_string());

                if let Some(ref req_etat) = etat_filter {
                    if !etat.eq_ignore_ascii_case(req_etat) {
                        return None;
                    }
                }

                let siege = DinumSiege {
                    siret: res.siret.as_ref().map(|s| s.to_string()),
                    code_postal: view
                        .as_ref()
                        .and_then(|v| v.code_postal.as_ref().map(|cp| cp.to_string())),
                    code_commune: view
                        .as_ref()
                        .and_then(|v| v.code_commune.as_ref().map(|cc| cc.to_string())),
                    etat_administratif: etat.clone(),
                };

                let activite = view
                    .as_ref()
                    .and_then(|v| v.code_naf.as_ref().map(|n| n.to_string()));
                let dirigeants = view
                    .as_ref()
                    .map(|v| v.dirigeants.iter().map(|d| d.to_string()).collect())
                    .unwrap_or_default();

                Some(DinumSearchResult {
                    siren: res.siren.to_string(),
                    nom_complet: res.denomination.clone(),
                    nom_raison_sociale: res.denomination,
                    sigle: None,
                    activite_principale: activite,
                    etat_administratif: etat,
                    siege,
                    dirigeants,
                    matching_score: res.score,
                    is_exact_pivot: res.is_exact_pivot,
                })
            })
            .collect();

        let total_results = dinum_results.len();
        let total_pages = total_results.div_ceil(per_page);

        let offset = (page.saturating_sub(1)) * per_page;
        let paginated_results: Vec<DinumSearchResult> = if offset < dinum_results.len() {
            dinum_results
                .into_iter()
                .skip(offset)
                .take(per_page)
                .collect()
        } else {
            Vec::new()
        };

        let response = DinumSearchResponse {
            results: paginated_results,
            total_results,
            page,
            per_page,
            total_pages,
        };

        return HttpResponse::ok_json(&response);
    }

    // Route: Direct SIREN lookup (/siren/{siren} or /v3/siren/{siren})
    if let Some(siren_part) = path
        .strip_prefix("/v3/siren/")
        .or_else(|| path.strip_prefix("/siren/"))
    {
        let raw_siren = siren_part.trim();
        let siren = match Siren::parse(raw_siren) {
            Ok(s) => s,
            Err(_) => {
                return HttpResponse::bad_request(&format!(
                    "SIREN invalide: '{}' (doit contenir 9 chiffres valides selon l'algorithme de Luhn)",
                    raw_siren
                ))
            }
        };

        let doc_id = match index.find_by_siren(siren) {
            Some(id) => id,
            None => {
                return HttpResponse::not_found(&format!(
                    "Aucune entreprise trouvée pour le SIREN {}",
                    siren
                ))
            }
        };

        let view = match index.get_document(doc_id) {
            Some(v) => v,
            None => return HttpResponse::internal_error("Document indexé introuvable"),
        };

        let siege = DinumSiege {
            siret: view.siret.as_ref().map(|s| s.to_string()),
            code_postal: view.code_postal.as_ref().map(|cp| cp.to_string()),
            code_commune: view.code_commune.as_ref().map(|cc| cc.to_string()),
            etat_administratif: view.etat_administratif.to_string(),
        };

        let resp = DinumEnterpriseResponse {
            siren: view.siren.to_string(),
            nom_complet: view.denomination.to_string(),
            nom_raison_sociale: view.denomination.to_string(),
            activite_principale: view.code_naf.as_ref().map(|n| n.to_string()),
            etat_administratif: view.etat_administratif.to_string(),
            siege,
            dirigeants: view.dirigeants.iter().map(|d| d.to_string()).collect(),
        };

        return HttpResponse::ok_json(&resp);
    }

    // Route: Direct SIRET lookup (/siret/{siret} or /v3/siret/{siret})
    if let Some(siret_part) = path
        .strip_prefix("/v3/siret/")
        .or_else(|| path.strip_prefix("/siret/"))
    {
        let raw_siret = siret_part.trim();
        let siret = match Siret::parse(raw_siret) {
            Ok(s) => s,
            Err(_) => {
                return HttpResponse::bad_request(&format!(
                    "SIRET invalide: '{}' (doit contenir 14 chiffres valides selon l'algorithme de Luhn)",
                    raw_siret
                ))
            }
        };

        let doc_id = match index.find_by_siret(siret) {
            Some(id) => id,
            None => {
                return HttpResponse::not_found(&format!(
                    "Aucun établissement trouvé pour le SIRET {}",
                    siret
                ))
            }
        };

        let view = match index.get_document(doc_id) {
            Some(v) => v,
            None => return HttpResponse::internal_error("Document indexé introuvable"),
        };

        let is_siege = view.siret.as_ref().map(|s| *s == siret).unwrap_or(true);
        let resp = DinumEtablissementResponse {
            siret: siret.to_string(),
            siren: view.siren.to_string(),
            nom_complet: view.denomination.to_string(),
            activite_principale: view.code_naf.as_ref().map(|n| n.to_string()),
            etat_administratif: view.etat_administratif.to_string(),
            code_postal: view.code_postal.as_ref().map(|cp| cp.to_string()),
            code_commune: view.code_commune.as_ref().map(|cc| cc.to_string()),
            est_siege: is_siege,
            dirigeants: view.dirigeants.iter().map(|d| d.to_string()).collect(),
        };

        return HttpResponse::ok_json(&resp);
    }

    HttpResponse::not_found(&format!("Route non trouvée: '{}'", path))
}

// ============================================================================
// Multi-threaded TCP Connection Handler
// ============================================================================

pub fn handle_client(mut stream: TcpStream, index: &SireneIndex) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

    let mut reader = BufReader::new(&stream);
    match parse_http_request(&mut reader) {
        Ok(Some(req)) => {
            let resp = handle_request(&req, index);
            let _ = resp.write_to(&mut stream);
        }
        Ok(None) => {}
        Err(_) => {
            let resp = HttpResponse::bad_request("Requête HTTP non conforme");
            let _ = resp.write_to(&mut stream);
        }
    }
}

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    _id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<Sender<Job>>,
}

impl ThreadPool {
    fn new(size: usize) -> Self {
        let (sender, receiver) = channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);
        for _id in 0..size {
            let receiver = Arc::clone(&receiver);
            let thread = thread::spawn(move || loop {
                let job = {
                    let lock = match receiver.lock() {
                        Ok(l) => l,
                        Err(_) => break,
                    };
                    match lock.recv() {
                        Ok(j) => j,
                        Err(_) => break,
                    }
                };
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
            });
            workers.push(Worker {
                _id,
                thread: Some(thread),
            });
        }

        Self {
            workers,
            sender: Some(sender),
        }
    }

    fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        if let Some(ref sender) = self.sender {
            let _ = sender.send(Box::new(f));
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

// ============================================================================
// Command Execution Entrypoint
// ============================================================================

/// Starts the high-performance DINUM/INSEE HTTP API microservice.
pub fn execute(args: ServeArgs) -> Result<()> {
    if !args.index.exists() {
        return Err(CliError::FileNotFound(args.index));
    }

    println!("Loading SIRENE binary index from {:?}...", args.index);
    let index = Arc::new(SireneIndex::open_file(&args.index)?);
    let num_docs = index.num_documents();

    let workers_count = args
        .workers
        .unwrap_or_else(|| {
            thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        })
        .max(1);

    let bind_addr = format!("{}:{}", args.host, args.port);
    let listener = TcpListener::bind(&bind_addr)?;

    println!(
        "\n===================================================================\n\
         Sirene REST Microservice running on http://{}\n\
         Indexed documents: {}\n\
         Worker threads:    {}\n\
         Endpoints:\n\
           GET /search?q={{query}}&code_postal={{cp}}&departement={{dep}}&limite={{k}}\n\
           GET /siren/{{siren}}\n\
           GET /siret/{{siret}}\n\
           GET /health\n\
         ===================================================================\n",
        bind_addr, num_docs, workers_count
    );

    let pool = ThreadPool::new(workers_count);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let index_clone = Arc::clone(&index);
                pool.execute(move || {
                    handle_client(stream, &index_clone);
                });
            }
            Err(e) => {
                eprintln!("Connection accept error: {}", e);
            }
        }
    }

    Ok(())
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use sirene_core::{CodeCommune, CodeNaf, CodePostal, EtatAdministratif, Siren, Siret};
    use sirene_index::{IndexBuilder, IndexDocument};

    fn make_test_index() -> SireneIndex {
        let mut builder = IndexBuilder::new();
        let siren = Siren::parse("732829320").expect("siren");
        let siret = Siret::parse("73282932000074").expect("siret");
        let cp = CodePostal::parse("75001").expect("cp");
        let com = CodeCommune::parse("75101").expect("com");
        let naf = CodeNaf::parse("47.22Z").expect("naf");

        builder.add_document(IndexDocument {
            siren,
            siret: Some(siret),
            denomination: "Boucherie Dupont SAS".to_string(),
            code_commune: Some(com),
            code_postal: Some(cp),
            code_naf: Some(naf),
            etat_administratif: EtatAdministratif::Actif,
            dirigeants: vec!["Jean Dupont".to_string()],
        });

        let bytes = builder.build().expect("build index");
        SireneIndex::from_vec(bytes).expect("load index")
    }

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("hello+world"), "hello world");
        assert_eq!(percent_decode("Boucherie%20Dupont"), "Boucherie Dupont");
        assert_eq!(percent_decode("%C3%89lectricit%C3%A9"), "Électricité");
    }

    #[test]
    fn test_parse_http_request() {
        let raw =
            b"GET /search?q=dupont&code_postal=75001 HTTP/1.1\r\nHost: localhost:8080\r\n\r\n";
        let mut reader = std::io::Cursor::new(raw);
        let req = parse_http_request(&mut reader)
            .expect("parse")
            .expect("some req");

        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/search");
        assert_eq!(req.query_params.get("q"), Some(&"dupont".to_string()));
        assert_eq!(
            req.query_params.get("code_postal"),
            Some(&"75001".to_string())
        );
    }

    #[test]
    fn test_handle_health() {
        let index = make_test_index();
        let req = HttpRequest {
            method: "GET".to_string(),
            path: "/health".to_string(),
            query_params: HashMap::new(),
        };
        let resp = handle_request(&req, &index);
        assert_eq!(resp.status_code, 200);
        let body_str = std::str::from_utf8(&resp.body).expect("utf8");
        assert!(body_str.contains("\"status\":\"ok\""));
        assert!(body_str.contains("\"num_documents\":1"));
    }

    #[test]
    fn test_handle_search() {
        let index = make_test_index();
        let mut params = HashMap::new();
        params.insert("q".to_string(), "Boucherie Dupan".to_string());
        let req = HttpRequest {
            method: "GET".to_string(),
            path: "/search".to_string(),
            query_params: params,
        };
        let resp = handle_request(&req, &index);
        assert_eq!(resp.status_code, 200);
        let body_str = std::str::from_utf8(&resp.body).expect("utf8");
        assert!(body_str.contains("\"siren\":\"732829320\""));
        assert!(body_str.contains("\"total_results\":1"));
    }

    #[test]
    fn test_handle_siren_lookup() {
        let index = make_test_index();
        // 1. Valid found
        let req_found = HttpRequest {
            method: "GET".to_string(),
            path: "/siren/732829320".to_string(),
            query_params: HashMap::new(),
        };
        let resp = handle_request(&req_found, &index);
        assert_eq!(resp.status_code, 200);
        let body_str = std::str::from_utf8(&resp.body).expect("utf8");
        assert!(body_str.contains("\"siren\":\"732829320\""));
        assert!(body_str.contains("\"jean dupont\""));

        // 2. Valid not found (valid SIREN 443061841)
        let req_not_found = HttpRequest {
            method: "GET".to_string(),
            path: "/siren/443061841".to_string(),
            query_params: HashMap::new(),
        };
        let resp_nf = handle_request(&req_not_found, &index);
        assert_eq!(resp_nf.status_code, 404);

        // 3. Invalid SIREN
        let req_invalid = HttpRequest {
            method: "GET".to_string(),
            path: "/siren/12345".to_string(),
            query_params: HashMap::new(),
        };
        let resp_inv = handle_request(&req_invalid, &index);
        assert_eq!(resp_inv.status_code, 400);
    }

    #[test]
    fn test_handle_siret_lookup() {
        let index = make_test_index();
        // 1. Valid found
        let req_found = HttpRequest {
            method: "GET".to_string(),
            path: "/siret/73282932000074".to_string(),
            query_params: HashMap::new(),
        };
        let resp = handle_request(&req_found, &index);
        assert_eq!(resp.status_code, 200);
        let body_str = std::str::from_utf8(&resp.body).expect("utf8");
        assert!(body_str.contains("\"siret\":\"73282932000074\""));
        assert!(body_str.contains("\"est_siege\":true"));

        // 2. Invalid SIRET
        let req_invalid = HttpRequest {
            method: "GET".to_string(),
            path: "/siret/73282932000099".to_string(),
            query_params: HashMap::new(),
        };
        let resp_inv = handle_request(&req_invalid, &index);
        assert_eq!(resp_inv.status_code, 400);
    }

    #[test]
    fn test_cors_preflight() {
        let index = make_test_index();
        let req = HttpRequest {
            method: "OPTIONS".to_string(),
            path: "/search".to_string(),
            query_params: HashMap::new(),
        };
        let resp = handle_request(&req, &index);
        assert_eq!(resp.status_code, 204);
    }

    #[test]
    fn test_line_bomb_rejection() {
        // Line bomb: single line of 10,000 characters without \n (exceeding 8192 bytes limit)
        let huge_line = vec![b'A'; 10_000];
        let mut reader = std::io::Cursor::new(huge_line);
        let result = parse_http_request(&mut reader);
        assert!(result.is_err(), "Must reject oversized HTTP lines");
    }

    #[test]
    fn test_pagination_and_total_pages() {
        let index = make_test_index();
        let mut params = HashMap::new();
        params.insert("q".to_string(), "Boucherie".to_string());
        params.insert("page".to_string(), "1".to_string());
        params.insert("per_page".to_string(), "5".to_string());
        let req = HttpRequest {
            method: "GET".to_string(),
            path: "/search".to_string(),
            query_params: params,
        };
        let resp = handle_request(&req, &index);
        assert_eq!(resp.status_code, 200);
        let body_str = std::str::from_utf8(&resp.body).expect("utf8");
        assert!(body_str.contains("\"total_pages\":1"));
        assert!(body_str.contains("\"page\":1"));
        assert!(body_str.contains("\"per_page\":5"));
    }

    #[test]
    fn test_handle_search_filters_and_missing_q() {
        let index = make_test_index();

        // 1. Missing 'q'
        let req_no_q = HttpRequest {
            method: "GET".to_string(),
            path: "/search".to_string(),
            query_params: HashMap::new(),
        };
        let resp = handle_request(&req_no_q, &index);
        assert_eq!(resp.status_code, 400);

        // 2. Search with all filters
        let mut params = HashMap::new();
        params.insert("q".to_string(), "Boucherie".to_string());
        params.insert("departement".to_string(), "75".to_string());
        params.insert("commune".to_string(), "75101".to_string());
        params.insert("activite_principale".to_string(), "47.22Z".to_string());
        params.insert("dirigeant".to_string(), "Jean Dupont".to_string());
        params.insert("etat_administratif".to_string(), "A".to_string());
        params.insert("strict_location".to_string(), "true".to_string());
        let req_filtered = HttpRequest {
            method: "GET".to_string(),
            path: "/search".to_string(),
            query_params: params,
        };
        let resp_filt = handle_request(&req_filtered, &index);
        assert_eq!(resp_filt.status_code, 200);

        // 3. Search with ceased filter (should exclude active document)
        let mut params_ceased = HashMap::new();
        params_ceased.insert("q".to_string(), "Boucherie".to_string());
        params_ceased.insert("etat_administratif".to_string(), "C".to_string());
        let req_ceased = HttpRequest {
            method: "GET".to_string(),
            path: "/search".to_string(),
            query_params: params_ceased,
        };
        let resp_c = handle_request(&req_ceased, &index);
        assert_eq!(resp_c.status_code, 200);
        let body_str = std::str::from_utf8(&resp_c.body).expect("utf8");
        assert!(body_str.contains("\"total_results\":0"));
    }

    #[test]
    fn test_http_methods_and_unknown_routes() {
        let index = make_test_index();

        // POST method not allowed
        let req_post = HttpRequest {
            method: "POST".to_string(),
            path: "/search".to_string(),
            query_params: HashMap::new(),
        };
        let resp_post = handle_request(&req_post, &index);
        assert_eq!(resp_post.status_code, 405);

        // Unknown route
        let req_unknown = HttpRequest {
            method: "GET".to_string(),
            path: "/nonexistent/api".to_string(),
            query_params: HashMap::new(),
        };
        let resp_unk = handle_request(&req_unknown, &index);
        assert_eq!(resp_unk.status_code, 404);

        // SIRET valid but not found
        let req_siret_nf = HttpRequest {
            method: "GET".to_string(),
            path: "/siret/44306184100021".to_string(),
            query_params: HashMap::new(),
        };
        let resp_siret = handle_request(&req_siret_nf, &index);
        assert_eq!(resp_siret.status_code, 404);
    }

    #[test]
    fn test_header_flood_and_size_protections() {
        // Header flood: > 100 headers
        let mut flood = String::from("GET /health HTTP/1.1\r\n");
        for i in 0..110 {
            flood.push_str(&format!("X-Header-{}: test\r\n", i));
        }
        flood.push_str("\r\n");
        let mut reader = std::io::Cursor::new(flood.as_bytes());
        assert!(parse_http_request(&mut reader).is_err());

        // Header size protection: total headers > 32768 bytes
        let mut big_headers = String::from("GET /health HTTP/1.1\r\n");
        let val = "A".repeat(1000);
        for i in 0..40 {
            big_headers.push_str(&format!("X-Big-{}: {}\r\n", i, val));
        }
        big_headers.push_str("\r\n");
        let mut reader2 = std::io::Cursor::new(big_headers.as_bytes());
        assert!(parse_http_request(&mut reader2).is_err());
    }
}
