use std::fs::File;
use std::io::{Read, Write};
use tempfile::tempdir;

use sirene_cli::args::{IndexArgs, InspectArgs, SearchArgs};
use sirene_cli::commands::{index, inspect, search};

// Sample INSEE CSV Fixtures
const SAMPLE_UNITE_LEGALE_CSV: &str = "\
siren,denominationUniteLegale,nomUniteLegale,nomUsageUniteLegale,prenom1UniteLegale,sigleUniteLegale,etatAdministratifUniteLegale,categorieJuridiqueUniteLegale,activitePrincipaleUniteLegale
732829320,Boucherie Dupont SAS,,,Dupont,BD,A,5710,47.22Z
443061841,Boulangerie Martin SARL,,,,,A,5498,10.71C
356000000,Boulangerie Martin EI,,Martin,Paul,,A,1000,10.71C
552120222,Dupont Plomberie SASU,,,,,A,5710,43.22A
800123457,Tech Solutions SAS,,,,,A,5710,62.01Z
900654328,Tech Solutions SARL,,,,,A,5498,62.02A
123456782,,Durand,,Thomas,,C,1000,68.20A
";

const SAMPLE_ETABLISSEMENT_CSV: &str = "\
siren,nic,siret,etablissementSiege,codeCommuneEtablissement,codePostalEtablissement
732829320,00018,73282932000018,true,75101,75001
443061841,00025,44306184100025,true,33063,33000
356000000,00010,35600000000010,true,59350,59000
552120222,00014,55212022200014,true,69381,69001
800123457,00011,80012345700011,true,75102,75002
900654328,00019,90065432800019,true,13055,13001
123456782,00012,12345678200012,true,75103,75003
";

const SAMPLE_DIRIGEANTS_CSV: &str = "\
siren,nom,nom_usage,prenoms,denomination
800123457,Durand,,Thomas,
900654328,Dupont,,Jacques,
";

#[test]
fn test_cli_indexing_and_search_pipeline() {
    let dir = tempdir().expect("temp dir");

    let ul_path = dir.path().join("StockUniteLegale_utf8.csv");
    let etab_path = dir.path().join("StockEtablissement_utf8.csv");
    let dir_path = dir.path().join("Dirigeants.csv");
    let idx_path = dir.path().join("sirene.idx");

    File::create(&ul_path)
        .and_then(|mut f| f.write_all(SAMPLE_UNITE_LEGALE_CSV.as_bytes()))
        .expect("write unite_legale");

    File::create(&etab_path)
        .and_then(|mut f| f.write_all(SAMPLE_ETABLISSEMENT_CSV.as_bytes()))
        .expect("write etablissement");

    File::create(&dir_path)
        .and_then(|mut f| f.write_all(SAMPLE_DIRIGEANTS_CSV.as_bytes()))
        .expect("write dirigeants");

    // 1. Index command
    let index_args = IndexArgs {
        unite_legale: ul_path,
        etablissement: Some(etab_path),
        dirigeants: Some(dir_path),
        output: idx_path.clone(),
        active_only: false,
        department: None,
        limit: None,
    };

    index::execute(index_args).expect("execute index");
    assert!(idx_path.exists());

    // 2. Inspect command
    let inspect_args = InspectArgs {
        index: idx_path.clone(),
    };
    inspect::execute(inspect_args).expect("execute inspect");

    // 3. Search test A: Typo tolerance "Boucherie Dupan" -> Boucherie Dupont
    let search_typo = SearchArgs {
        index: idx_path.clone(),
        query: "Boucherie Dupan".to_string(),
        location: None,
        naf: None,
        dirigeant: None,
        top_k: 5,
        json: false,
    };
    search::execute(search_typo).expect("search typo");

    // 4. Search test B: Geographic disambiguation: "Boulangerie Martin" + Bordeaux (33000)
    let search_geo = SearchArgs {
        index: idx_path.clone(),
        query: "Boulangerie Martin".to_string(),
        location: Some("33000".to_string()),
        naf: None,
        dirigeant: None,
        top_k: 5,
        json: true,
    };
    search::execute(search_geo).expect("search geo");

    // 5. Search test C: Dirigeant boost "Tech Solutions" + Thomas Durand
    let search_dir = SearchArgs {
        index: idx_path.clone(),
        query: "Tech Solutions".to_string(),
        location: None,
        naf: None,
        dirigeant: Some("Thomas Durand".to_string()),
        top_k: 5,
        json: false,
    };
    search::execute(search_dir).expect("search dir");

    // 6. Search test D: Exact SIREN pivot lookup
    let search_pivot = SearchArgs {
        index: idx_path.clone(),
        query: "732829320".to_string(),
        location: None,
        naf: None,
        dirigeant: None,
        top_k: 5,
        json: false,
    };
    search::execute(search_pivot).expect("search pivot");
}

#[test]
fn test_cli_active_and_department_filters() {
    let dir = tempdir().expect("temp dir");

    let ul_path = dir.path().join("StockUniteLegale.csv");
    let etab_path = dir.path().join("StockEtablissement.csv");
    let idx_path = dir.path().join("paris_active.idx");

    File::create(&ul_path)
        .and_then(|mut f| f.write_all(SAMPLE_UNITE_LEGALE_CSV.as_bytes()))
        .expect("write unite_legale");

    File::create(&etab_path)
        .and_then(|mut f| f.write_all(SAMPLE_ETABLISSEMENT_CSV.as_bytes()))
        .expect("write etablissement");

    // Index active only + department 75 (Paris)
    let index_args = IndexArgs {
        unite_legale: ul_path,
        etablissement: Some(etab_path),
        dirigeants: None,
        output: idx_path.clone(),
        active_only: true,
        department: Some("75".to_string()),
        limit: None,
    };

    index::execute(index_args).expect("execute index with filters");

    let inspect_args = InspectArgs {
        index: idx_path.clone(),
    };
    inspect::execute(inspect_args).expect("execute inspect");

    // Only active entities in 75 should remain (Boucherie Dupont 732829320 and Tech Solutions 800123457)
    // Ceased entity 123456782 (even though in 75) should be excluded because active_only = true
    let index = sirene_index::SireneIndex::open_file(&idx_path).expect("open index");
    assert_eq!(index.num_documents(), 2);
}

#[test]
fn test_compressed_stream_ingest() {
    use flate2::write::GzEncoder;
    use flate2::Compression;

    let dir = tempdir().expect("temp dir");
    let gz_path = dir.path().join("StockUniteLegale.csv.gz");
    let idx_path = dir.path().join("sirene_gz.idx");

    {
        let file = File::create(&gz_path).expect("create gz file");
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder
            .write_all(SAMPLE_UNITE_LEGALE_CSV.as_bytes())
            .expect("compress csv");
        encoder.finish().expect("finish compression");
    }

    let index_args = IndexArgs {
        unite_legale: gz_path,
        etablissement: None,
        dirigeants: None,
        output: idx_path.clone(),
        active_only: false,
        department: None,
        limit: Some(3),
    };

    index::execute(index_args).expect("index from .gz");
    let index = sirene_index::SireneIndex::open_file(&idx_path).expect("open index");
    assert_eq!(index.num_documents(), 3);
}

#[test]
fn test_zip_stream_ingest() {
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    let dir = tempdir().expect("temp dir");
    let zip_path = dir.path().join("StockUniteLegale.zip");
    let idx_path = dir.path().join("sirene_zip.idx");

    {
        let file = File::create(&zip_path).expect("create zip file");
        let mut zip = ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("StockUniteLegale_utf8.csv", options)
            .expect("start zip file entry");
        zip.write_all(SAMPLE_UNITE_LEGALE_CSV.as_bytes())
            .expect("write csv to zip");
        zip.finish().expect("finish zip");
    }

    let index_args = IndexArgs {
        unite_legale: zip_path,
        etablissement: None,
        dirigeants: None,
        output: idx_path.clone(),
        active_only: false,
        department: None,
        limit: Some(4),
    };

    index::execute(index_args).expect("index from .zip");
    let index = sirene_index::SireneIndex::open_file(&idx_path).expect("open index");
    assert_eq!(index.num_documents(), 4);

    // Stored ZIP compression method test
    let stored_zip_path = dir.path().join("stored.zip");
    {
        let file = File::create(&stored_zip_path).expect("create stored zip");
        let mut zip = ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("data.csv", options)
            .expect("start stored entry");
        zip.write_all(b"test line\n").expect("write data");
        zip.finish().expect("finish stored zip");
    }
    let mut reader =
        sirene_cli::ingest::open_streaming_reader(&stored_zip_path).expect("open stored zip");
    let mut content = String::new();
    reader.read_to_string(&mut content).expect("read content");
    assert_eq!(content, "test line\n");

    // Plain text reader test
    let plain_path = dir.path().join("plain.csv");
    File::create(&plain_path)
        .and_then(|mut f| f.write_all(b"plain data\n"))
        .expect("create plain");
    let mut plain_reader =
        sirene_cli::ingest::open_streaming_reader(&plain_path).expect("open plain");
    let mut plain_content = String::new();
    plain_reader
        .read_to_string(&mut plain_content)
        .expect("read plain content");
    assert_eq!(plain_content, "plain data\n");

    // Missing file test
    let missing_path = dir.path().join("nonexistent.csv");
    assert!(sirene_cli::ingest::open_streaming_reader(&missing_path).is_err());
}

#[test]
fn test_statut_diffusion_non_diffusible_and_enseigne_enrichment() {
    let dir = tempdir().expect("temp dir");
    let ul_path = dir.path().join("StockUniteLegale_nd.csv");
    let etab_path = dir.path().join("StockEtablissement_enseigne.csv");
    let idx_path = dir.path().join("sirene_nd.idx");

    let ul_csv = "\
siren,denominationUniteLegale,nomUniteLegale,prenom1UniteLegale,etatAdministratifUniteLegale,statutDiffusionUniteLegale
732829320,,Martin,Jean,A,N
";

    let etab_csv = "\
siren,nic,siret,etablissementSiege,codeCommuneEtablissement,codePostalEtablissement,enseigne1Etablissement
732829320,00074,73282932000074,true,75101,75001,Cyber Safe Solutions
";

    File::create(&ul_path)
        .and_then(|mut f| f.write_all(ul_csv.as_bytes()))
        .expect("write ul");

    File::create(&etab_path)
        .and_then(|mut f| f.write_all(etab_csv.as_bytes()))
        .expect("write etab");

    let index_args = IndexArgs {
        unite_legale: ul_path,
        etablissement: Some(etab_path),
        dirigeants: None,
        output: idx_path.clone(),
        active_only: false,
        department: None,
        limit: None,
    };

    index::execute(index_args).expect("execute index with ND and enseigne");

    let index = sirene_index::SireneIndex::open_file(&idx_path).expect("open index");
    assert_eq!(index.num_documents(), 1);

    let doc = index.get_document(0).expect("doc 0");
    // Verify patronymic "Martin Jean" is NOT exposed in denomination, but replaced by [ND] and enriched with enseigne
    assert!(doc.denomination.contains("[nd]") || doc.denomination.contains("nd"));
    assert!(doc.denomination.contains("cyber safe solutions"));
    assert!(!doc.denomination.contains("martin"));

    // Verify search by trade name (enseigne) matches the entity
    let normalizer = sirene_normalize::Normalizer::new();
    let norm_q = normalizer.normalize_query("Cyber Safe Solutions");
    let search_q = sirene_matcher::SearchQuery {
        query: &norm_q,
        location: None,
        naf: None,
        dirigeant: None,
        strict_location: false,
    };
    let res = index.search(&search_q, 5);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].siren.as_str(), "732829320");
    assert!(res[0].score > 0.8);
}

#[test]
fn test_sirene_serve_http_microservice() {
    use std::io::Read;
    use std::net::{TcpListener, TcpStream};
    use std::sync::Arc;
    use std::thread;

    let dir = tempdir().expect("temp dir");
    let ul_path = dir.path().join("StockUniteLegale.csv");
    let etab_path = dir.path().join("StockEtablissement.csv");
    let idx_path = dir.path().join("sirene_serve.idx");

    File::create(&ul_path)
        .and_then(|mut f| f.write_all(SAMPLE_UNITE_LEGALE_CSV.as_bytes()))
        .expect("write ul");

    File::create(&etab_path)
        .and_then(|mut f| f.write_all(SAMPLE_ETABLISSEMENT_CSV.as_bytes()))
        .expect("write etab");

    let index_args = IndexArgs {
        unite_legale: ul_path,
        etablissement: Some(etab_path),
        dirigeants: None,
        output: idx_path.clone(),
        active_only: false,
        department: None,
        limit: None,
    };
    index::execute(index_args).expect("execute index");

    let index = Arc::new(sirene_index::SireneIndex::open_file(&idx_path).expect("open index"));
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");

    // Spawn server thread handling 3 test requests
    let server_index = Arc::clone(&index);
    let server_handle = thread::spawn(move || {
        for _ in 0..3 {
            if let Ok((stream, _)) = listener.accept() {
                sirene_cli::commands::serve::handle_client(stream, &server_index);
            }
        }
    });

    // Helper to send HTTP GET and read response string
    let send_get = |uri: &str| -> String {
        let mut stream = TcpStream::connect(addr).expect("connect to server");
        let req = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            uri, addr
        );
        stream.write_all(req.as_bytes()).expect("write req");
        let mut resp = String::new();
        stream.read_to_string(&mut resp).expect("read resp");
        resp
    };

    // 1. Health check
    let resp_health = send_get("/health");
    assert!(resp_health.starts_with("HTTP/1.1 200 OK"));
    assert!(resp_health.contains("\"status\":\"ok\""));

    // 2. Search query via HTTP
    let resp_search = send_get("/search?q=Boucherie&code_postal=75001");
    assert!(resp_search.starts_with("HTTP/1.1 200 OK"));
    assert!(resp_search.contains("\"siren\":\"732829320\""));
    assert!(resp_search.contains("\"total_results\":1"));

    // 3. SIREN lookup via HTTP
    let resp_siren = send_get("/siren/732829320");
    assert!(resp_siren.starts_with("HTTP/1.1 200 OK"));
    assert!(resp_siren.contains("\"siren\":\"732829320\""));

    server_handle.join().expect("server join");
}

#[test]
fn test_cli_batch_csv_matching() {
    let dir = tempdir().expect("temp dir");

    let ul_path = dir.path().join("StockUniteLegale.csv");
    let etab_path = dir.path().join("StockEtablissement.csv");
    let idx_path = dir.path().join("sirene.idx");
    let input_csv = dir.path().join("clients.csv");
    let output_csv = dir.path().join("matched_clients.csv");

    File::create(&ul_path)
        .and_then(|mut f| f.write_all(SAMPLE_UNITE_LEGALE_CSV.as_bytes()))
        .expect("write ul");

    File::create(&etab_path)
        .and_then(|mut f| f.write_all(SAMPLE_ETABLISSEMENT_CSV.as_bytes()))
        .expect("write etab");

    // Index
    index::execute(IndexArgs {
        unite_legale: ul_path,
        etablissement: Some(etab_path),
        dirigeants: None,
        output: idx_path.clone(),
        active_only: true,
        department: None,
        limit: None,
    })
    .expect("index execute");

    // Create client batch input CSV
    let clients_data = "\
id,raison_sociale,code_postal,activite
1,Boucherie Dupont,75001,47.22Z
2,Tech Solutns,75002,62.01Z
3,Boulangerie Martin,59000,10.71C
4,Inconnu Entreprise,00000,99.99Z
5,Entreprise 732829320,75001,47.22Z
";
    File::create(&input_csv)
        .and_then(|mut f| f.write_all(clients_data.as_bytes()))
        .expect("write clients");

    // Run batch match
    sirene_cli::commands::match_batch::execute(sirene_cli::args::MatchArgs {
        index: idx_path,
        input: input_csv,
        col_nom: "raison_sociale".to_string(),
        col_location: Some("code_postal".to_string()),
        col_naf: Some("activite".to_string()),
        output: output_csv.clone(),
        threshold: 0.50,
    })
    .expect("batch match execute");

    // Verify output CSV
    let content = std::fs::read_to_string(&output_csv).expect("read output csv");
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 6); // header + 5 rows
    assert!(lines[0].contains("siren_match"));
    assert!(lines[0].contains("score_match"));
    assert!(lines[0].contains("is_exact_pivot"));

    // Row 1: Boucherie Dupont -> 732829320
    assert!(lines[1].contains("732829320"));
    // Row 2: Tech Solutns -> 800123457
    assert!(lines[2].contains("800123457"));
    // Row 3: Boulangerie Martin in 59 -> 356000000
    assert!(lines[3].contains("356000000"));
    // Row 4: Inconnu -> unmatched (empty score/siren)
    assert!(lines[4].contains("Inconnu Entreprise"));
    // Row 5: Exact pivot 732829320 -> is_exact_pivot true
    assert!(lines[5].contains("true"));
}

#[test]
fn test_cli_args_parsing() {
    use clap::Parser;
    use sirene_cli::args::{Cli, Commands};

    // 1. Search args
    let cli = Cli::try_parse_from([
        "sirene", "search", "Dupont", "-l", "75001", "--naf", "62.01Z", "--json",
    ])
    .unwrap();
    if let Commands::Search(args) = cli.command {
        assert_eq!(args.query, "Dupont");
        assert_eq!(args.location.as_deref(), Some("75001"));
        assert_eq!(args.naf.as_deref(), Some("62.01Z"));
        assert!(args.json);
    } else {
        panic!("Expected Search command");
    }

    // 2. Index args
    let cli_idx = Cli::try_parse_from([
        "sirene",
        "index",
        "-u",
        "stock.csv",
        "-e",
        "etab.csv",
        "-d",
        "dir.csv",
        "--active-only",
        "--department",
        "75",
        "-l",
        "100",
    ])
    .unwrap();
    if let Commands::Index(args) = cli_idx.command {
        assert_eq!(args.unite_legale.to_str().unwrap(), "stock.csv");
        assert!(args.active_only);
        assert_eq!(args.department.as_deref(), Some("75"));
        assert_eq!(args.limit, Some(100));
    } else {
        panic!("Expected Index command");
    }

    // 3. Serve args
    let cli_srv = Cli::try_parse_from([
        "sirene",
        "serve",
        "-p",
        "9090",
        "-b",
        "127.0.0.1",
        "-w",
        "8",
    ])
    .unwrap();
    if let Commands::Serve(args) = cli_srv.command {
        assert_eq!(args.port, 9090);
        assert_eq!(args.host, "127.0.0.1");
        assert_eq!(args.workers, Some(8));
    } else {
        panic!("Expected Serve command");
    }

    // 4. Interactive args
    let cli_repl = Cli::try_parse_from(["sirene", "interactive", "-i", "my.idx"]).unwrap();
    assert!(matches!(cli_repl.command, Commands::Interactive(_)));

    // 5. Inspect args
    let cli_insp = Cli::try_parse_from(["sirene", "inspect", "-i", "my.idx"]).unwrap();
    assert!(matches!(cli_insp.command, Commands::Inspect(_)));

    // 6. Match args
    let cli_match = Cli::try_parse_from([
        "sirene",
        "match",
        "-i",
        "my.idx",
        "-f",
        "-",
        "-o",
        "-",
        "--threshold",
        "0.75",
    ])
    .unwrap();
    if let Commands::Match(args) = cli_match.command {
        assert_eq!(args.input.to_str().unwrap(), "-");
        assert_eq!(args.output.to_str().unwrap(), "-");
        assert_eq!(args.threshold, 0.75);
    } else {
        panic!("Expected Match command");
    }
}

#[test]
fn test_cli_interactive_repl_session() {
    let dir = tempdir().expect("temp dir");
    let ul_path = dir.path().join("StockUniteLegale.csv");
    let etab_path = dir.path().join("StockEtablissement.csv");
    let idx_path = dir.path().join("sirene.idx");

    File::create(&ul_path)
        .and_then(|mut f| f.write_all(SAMPLE_UNITE_LEGALE_CSV.as_bytes()))
        .expect("write ul");
    File::create(&etab_path)
        .and_then(|mut f| f.write_all(SAMPLE_ETABLISSEMENT_CSV.as_bytes()))
        .expect("write etab");

    index::execute(IndexArgs {
        unite_legale: ul_path,
        etablissement: Some(etab_path),
        dirigeants: None,
        output: idx_path.clone(),
        active_only: false,
        department: None,
        limit: None,
    })
    .expect("index execute");

    let index = sirene_index::SireneIndex::open_file(&idx_path).expect("open index");

    // Mock interactive input: empty line, valid query with @loc, #naf, $dir, and quit
    let input = b"\nBoucherie Dupont @75001\nTech Solutions #62.01Z\nAcme $Durand\n:q\n";
    let mut reader = std::io::BufReader::new(&input[..]);
    let mut output = Vec::new();

    sirene_cli::commands::interactive::run_interactive(&mut reader, &mut output, &index)
        .expect("run interactive");

    let out_str = String::from_utf8(output).expect("valid utf8");
    assert!(out_str.contains("SIRENE Interactive Search Engine"));
    assert!(out_str.to_lowercase().contains("boucherie dupont"));
    assert!(out_str.contains("Exiting."));
}

#[test]
fn test_cli_match_errors_and_missing_files() {
    use sirene_cli::args::MatchArgs;
    use sirene_cli::commands::match_batch;
    use std::path::PathBuf;

    // Missing index file
    let bad_idx = MatchArgs {
        index: PathBuf::from("nonexistent_index.idx"),
        input: PathBuf::from("-"),
        col_nom: "nom".to_string(),
        col_location: None,
        col_naf: None,
        output: PathBuf::from("-"),
        threshold: 0.5,
    };
    assert!(match_batch::execute(bad_idx).is_err());

    // Valid index, missing input file
    let dir = tempdir().expect("temp dir");
    let ul_path = dir.path().join("StockUniteLegale.csv");
    let idx_path = dir.path().join("sirene.idx");

    File::create(&ul_path)
        .and_then(|mut f| f.write_all(SAMPLE_UNITE_LEGALE_CSV.as_bytes()))
        .expect("write ul");

    index::execute(IndexArgs {
        unite_legale: ul_path,
        etablissement: None,
        dirigeants: None,
        output: idx_path.clone(),
        active_only: false,
        department: None,
        limit: None,
    })
    .expect("index");

    let bad_input = MatchArgs {
        index: idx_path.clone(),
        input: PathBuf::from("nonexistent_input.csv"),
        col_nom: "nom".to_string(),
        col_location: None,
        col_naf: None,
        output: PathBuf::from("-"),
        threshold: 0.5,
    };
    assert!(match_batch::execute(bad_input).is_err());

    // Missing target column in CSV
    let csv_file = dir.path().join("bad_cols.csv");
    File::create(&csv_file)
        .and_then(|mut f| f.write_all(b"id,other_col\n1,test\n"))
        .expect("write csv");

    let bad_col = MatchArgs {
        index: idx_path,
        input: csv_file,
        col_nom: "denomination_manquante".to_string(),
        col_location: None,
        col_naf: None,
        output: PathBuf::from("-"),
        threshold: 0.5,
    };
    assert!(match_batch::execute(bad_col).is_err());
}

#[test]
fn test_cli_search_and_inspect_missing_file() {
    use std::path::PathBuf;

    let res_search = search::execute(SearchArgs {
        index: PathBuf::from("nonexistent.idx"),
        query: "test".to_string(),
        location: None,
        naf: None,
        dirigeant: None,
        top_k: 5,
        json: false,
    });
    assert!(res_search.is_err());

    let res_inspect = inspect::execute(InspectArgs {
        index: PathBuf::from("nonexistent.idx"),
    });
    assert!(res_inspect.is_err());
}
