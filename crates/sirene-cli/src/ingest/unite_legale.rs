use csv::{ReaderBuilder, StringRecord};
use std::io::Read;

use sirene_core::{CodeNaf, EtatAdministratif, Siren};
use sirene_index::IndexDocument;

use crate::error::{CliError, Result};

/// Configuration options for streaming `StockUniteLegale` CSV ingestion.
#[derive(Debug, Clone, Default)]
pub struct UniteLegaleIngestOptions {
    /// Ingest only active enterprises (`etatAdministratif = 'A'`).
    pub active_only: bool,
    /// Maximum number of records to ingest.
    pub limit: Option<usize>,
}

/// Discovered column indices for `StockUniteLegale` CSV schema.
#[derive(Debug, Default)]
struct ColumnIndices {
    siren: Option<usize>,
    denomination: Option<usize>,
    nom: Option<usize>,
    nom_usage: Option<usize>,
    prenom: Option<usize>,
    sigle: Option<usize>,
    etat: Option<usize>,
    cat_juridique: Option<usize>,
    naf: Option<usize>,
    statut_diffusion: Option<usize>,
}

impl ColumnIndices {
    fn find(header: &StringRecord) -> Self {
        let mut idx = Self::default();
        for (i, col) in header.iter().enumerate() {
            let col_clean = col.trim().to_ascii_lowercase();
            match col_clean.as_str() {
                "siren" => idx.siren = Some(i),
                "denominationunitelegale" | "denomination" => idx.denomination = Some(i),
                "nomunitelegale" | "nom" => idx.nom = Some(i),
                "nomusageunitelegale" | "nomusage" => idx.nom_usage = Some(i),
                "prenom1unitelegale" | "prenom" | "prenom1" => idx.prenom = Some(i),
                "sigleunitelegale" | "sigle" => idx.sigle = Some(i),
                "etatadministratifunitelegale" | "etatadministratif" | "etat" => idx.etat = Some(i),
                "categoriejuridiqueunitelegale" | "categoriejuridique" => {
                    idx.cat_juridique = Some(i)
                }
                "activiteprincipaleunitelegale" | "activiteprincipale" | "naf" | "codenaf" => {
                    idx.naf = Some(i)
                }
                "statutdiffusionunitelegale" | "statutdiffusion" => idx.statut_diffusion = Some(i),
                _ => {}
            }
        }
        idx
    }
}

/// Streams raw `StockUniteLegale` CSV records and yields parsed `IndexDocument`s.
pub fn stream_unite_legale<R: Read>(
    reader: R,
    options: UniteLegaleIngestOptions,
) -> Result<Vec<IndexDocument>> {
    let mut csv_reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(reader);

    let headers = csv_reader.headers()?.clone();
    let col = ColumnIndices::find(&headers);

    let siren_idx = col
        .siren
        .ok_or_else(|| CliError::Custom("Required column 'siren' missing from CSV".to_string()))?;

    let capacity = options.limit.unwrap_or(10_000).min(100_000);
    let mut documents = Vec::with_capacity(capacity);
    let mut record = StringRecord::new();

    while csv_reader.read_record(&mut record)? {
        let siren_raw = match record.get(siren_idx) {
            Some(s) if !s.trim().is_empty() => s.trim(),
            _ => continue,
        };

        let siren = match Siren::parse(siren_raw) {
            Ok(s) => s,
            Err(_) => continue, // Skip invalid SIREN without aborting batch
        };

        // Resolve administrative status
        let etat = match col.etat.and_then(|i| record.get(i)) {
            Some(s) if s.trim().eq_ignore_ascii_case("C") => EtatAdministratif::Cesse,
            _ => EtatAdministratif::Actif,
        };

        if options.active_only && !etat.is_actif() {
            continue;
        }

        // Check legal diffusion status (Article A123-96 / GDPR)
        let is_diffusible = !matches!(
            col.statut_diffusion.and_then(|i| record.get(i)),
            Some(s) if s.trim().eq_ignore_ascii_case("N")
        );

        // Resolve corporate denomination
        let raw_denom = col
            .denomination
            .and_then(|i| record.get(i))
            .map(|s| s.trim())
            .unwrap_or("");

        let denomination = if !raw_denom.is_empty() {
            raw_denom.to_string()
        } else if !is_diffusible {
            // Natural person protected from public dissemination
            "[ND]".to_string()
        } else {
            // Natural person fallback: nom + prenom
            let nom = col
                .nom_usage
                .and_then(|i| record.get(i))
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    col.nom
                        .and_then(|i| record.get(i))
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                })
                .unwrap_or("");

            let prenom = col
                .prenom
                .and_then(|i| record.get(i))
                .map(|s| s.trim())
                .unwrap_or("");

            if !nom.is_empty() || !prenom.is_empty() {
                format!("{} {}", nom, prenom).trim().to_string()
            } else {
                continue; // Skip entities without a resolvable name
            }
        };

        // Append sigle if present and distinct
        let final_denomination = if let Some(sigle_idx) = col.sigle {
            if let Some(sigle) = record.get(sigle_idx).map(|s| s.trim()) {
                if !sigle.is_empty()
                    && !denomination
                        .to_ascii_uppercase()
                        .contains(&sigle.to_ascii_uppercase())
                {
                    format!("{} {}", denomination, sigle)
                } else {
                    denomination
                }
            } else {
                denomination
            }
        } else {
            denomination
        };

        // Parse optional NAF code
        let code_naf = col
            .naf
            .and_then(|i| record.get(i))
            .and_then(|s| CodeNaf::parse(s.trim()).ok());

        documents.push(IndexDocument {
            siren,
            siret: None,
            denomination: final_denomination,
            code_commune: None,
            code_postal: None,
            code_naf,
            etat_administratif: etat,
            dirigeants: Vec::new(),
        });

        if let Some(limit) = options.limit {
            if documents.len() >= limit {
                break;
            }
        }
    }

    Ok(documents)
}
