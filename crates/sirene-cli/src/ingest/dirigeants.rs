use csv::{ReaderBuilder, StringRecord};
use std::collections::HashMap;
use std::io::Read;

use sirene_core::Siren;
use sirene_index::IndexDocument;

use crate::error::{CliError, Result};

/// Discovered column indices for RNE Dirigeants CSV schema.
#[derive(Debug, Default)]
struct ColumnIndices {
    siren: Option<usize>,
    nom: Option<usize>,
    nom_usage: Option<usize>,
    prenoms: Option<usize>,
    denomination: Option<usize>,
}

impl ColumnIndices {
    fn find(header: &StringRecord) -> Self {
        let mut idx = Self::default();
        for (i, col) in header.iter().enumerate() {
            let col_clean = col.trim().to_ascii_lowercase();
            match col_clean.as_str() {
                "siren" => idx.siren = Some(i),
                "nom" | "nompatronymique" => idx.nom = Some(i),
                "nomusage" => idx.nom_usage = Some(i),
                "prenoms" | "prenom" | "prenom1" => idx.prenoms = Some(i),
                "denomination" | "denominationunitelegale" => idx.denomination = Some(i),
                _ => {}
            }
        }
        idx
    }
}

/// Enriches loaded `IndexDocument`s with corporate leaders and representatives from RNE INPI CSV.
pub fn enrich_with_dirigeants<R: Read>(
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
    let mut attached_count = 0;

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

        // Moral person dirigente
        let raw_denom = col
            .denomination
            .and_then(|i| record.get(i))
            .map(|s| s.trim())
            .unwrap_or("");

        let fullname = if !raw_denom.is_empty() {
            raw_denom.to_string()
        } else {
            // Physical person dirigente: prenom + nom
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
                .prenoms
                .and_then(|i| record.get(i))
                .map(|s| s.trim())
                .unwrap_or("");

            if !nom.is_empty() || !prenom.is_empty() {
                format!("{} {}", prenom, nom).trim().to_string()
            } else {
                continue;
            }
        };

        if !fullname.is_empty() {
            let doc = &mut documents[doc_idx];
            // Avoid duplicate leader names
            if !doc
                .dirigeants
                .iter()
                .any(|d| d.eq_ignore_ascii_case(&fullname))
            {
                doc.dirigeants.push(fullname);
                attached_count += 1;
            }
        }
    }

    Ok(attached_count)
}
