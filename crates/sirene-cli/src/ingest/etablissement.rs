use csv::{ReaderBuilder, StringRecord};
use std::collections::HashMap;
use std::io::Read;

use sirene_core::{CodeCommune, CodePostal, Siren, Siret};
use sirene_index::IndexDocument;

use crate::error::{CliError, Result};

/// Discovered column indices for `StockEtablissement` CSV schema.
#[derive(Debug, Default)]
struct ColumnIndices {
    siren: Option<usize>,
    nic: Option<usize>,
    siret: Option<usize>,
    siege: Option<usize>,
    code_commune: Option<usize>,
    code_postal: Option<usize>,
    enseigne: Option<usize>,
    denom_usuelle: Option<usize>,
}

impl ColumnIndices {
    fn find(header: &StringRecord) -> Self {
        let mut idx = Self::default();
        for (i, col) in header.iter().enumerate() {
            let col_clean = col.trim().to_ascii_lowercase();
            match col_clean.as_str() {
                "siren" => idx.siren = Some(i),
                "nic" => idx.nic = Some(i),
                "siret" => idx.siret = Some(i),
                "etablissementsiege" | "siege" => idx.siege = Some(i),
                "codecommuneetablissement" | "codecommune" => idx.code_commune = Some(i),
                "codepostaletablissement" | "codepostal" => idx.code_postal = Some(i),
                "enseigne1etablissement" | "enseigne" => idx.enseigne = Some(i),
                "denominationusuelleetablissement" | "denominationusuelle" => {
                    idx.denom_usuelle = Some(i)
                }
                _ => {}
            }
        }
        idx
    }
}

/// Enriches loaded `IndexDocument`s with SIRET and geographic coordinates from `StockEtablissement`.
pub fn enrich_with_etablissements<R: Read>(
    reader: R,
    documents: &mut [IndexDocument],
) -> Result<usize> {
    let mut siren_map: HashMap<Siren, usize> = HashMap::with_capacity(documents.len());
    for (i, doc) in documents.iter().enumerate() {
        siren_map.insert(doc.siren, i);
    }

    let mut csv_reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(reader);

    let headers = csv_reader.headers()?.clone();
    let col = ColumnIndices::find(&headers);

    let siren_idx = col
        .siren
        .ok_or_else(|| CliError::Custom("Required column 'siren' missing from CSV".to_string()))?;

    let mut record = StringRecord::new();
    let mut enriched_count = 0;
    // Track whether headquarters (siege) has been recorded for each doc
    let mut siege_recorded = vec![false; documents.len()];

    while csv_reader.read_record(&mut record)? {
        let siren_raw = match record.get(siren_idx) {
            Some(s) if !s.trim().is_empty() => s.trim(),
            _ => continue,
        };

        let siren = match Siren::parse(siren_raw) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let doc_idx = match siren_map.get(&siren) {
            Some(&idx) => idx,
            None => continue,
        };

        let is_siege = col
            .siege
            .and_then(|i| record.get(i))
            .map(|s| s.trim().eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        // If we already have the official headquarters, skip secondary branches
        if siege_recorded[doc_idx] && !is_siege {
            continue;
        }

        // Parse SIRET
        let siret = if let Some(s_idx) = col.siret {
            record.get(s_idx).and_then(|s| Siret::parse(s.trim()).ok())
        } else if let Some(n_idx) = col.nic {
            record.get(n_idx).and_then(|nic_str| {
                if nic_str.trim().len() == 5 {
                    let mut s = String::with_capacity(14);
                    s.push_str(siren.as_str());
                    s.push_str(nic_str.trim());
                    Siret::parse(&s).ok()
                } else {
                    None
                }
            })
        } else {
            None
        };

        // Parse Code Commune (COG)
        let code_commune = col
            .code_commune
            .and_then(|i| record.get(i))
            .and_then(|s| CodeCommune::parse(s.trim()).ok());

        // Parse Code Postal
        let code_postal = col
            .code_postal
            .and_then(|i| record.get(i))
            .and_then(|s| CodePostal::parse(s.trim()).ok());

        let doc = &mut documents[doc_idx];
        if siret.is_some() {
            doc.siret = siret;
        }
        if code_commune.is_some() {
            doc.code_commune = code_commune;
        }
        if code_postal.is_some() {
            doc.code_postal = code_postal;
        }

        // Attach trade name / enseigne if distinct from legal denomination
        let commercial_name = col
            .enseigne
            .and_then(|i| record.get(i))
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                col.denom_usuelle
                    .and_then(|i| record.get(i))
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
            });

        if let Some(com_name) = commercial_name {
            if !doc
                .denomination
                .to_ascii_uppercase()
                .contains(&com_name.to_ascii_uppercase())
            {
                doc.denomination.push_str(&format!(" ({})", com_name));
            }
        }

        if is_siege {
            siege_recorded[doc_idx] = true;
        }
        enriched_count += 1;
    }

    Ok(enriched_count)
}
