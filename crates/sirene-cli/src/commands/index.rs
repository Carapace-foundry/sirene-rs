use std::time::Instant;

use sirene_index::IndexBuilder;

use crate::args::IndexArgs;
use crate::error::{CliError, Result};
use crate::ingest::{
    enrich_with_dirigeants, enrich_with_etablissements, open_streaming_reader, stream_unite_legale,
    UniteLegaleIngestOptions,
};

/// Executes the index command: parses CSV/archives, enriches records, builds and writes `.idx` binary.
pub fn execute(args: IndexArgs) -> Result<()> {
    let start_time = Instant::now();
    println!("=== SIRENE Index Builder ===");

    if !args.unite_legale.exists() {
        return Err(CliError::FileNotFound(args.unite_legale));
    }

    println!(
        "[1/4] Reading legal units from {}...",
        args.unite_legale.display()
    );
    let ul_reader = open_streaming_reader(&args.unite_legale).map_err(CliError::Custom)?;
    let ul_start = Instant::now();

    let mut documents = stream_unite_legale(
        ul_reader,
        UniteLegaleIngestOptions {
            active_only: args.active_only,
            limit: args.limit,
        },
    )?;

    let ul_elapsed = ul_start.elapsed().as_secs_f64();
    let ul_rate = if ul_elapsed > 0.0 {
        documents.len() as f64 / ul_elapsed
    } else {
        0.0
    };
    println!(
        "      -> Loaded {} enterprises in {:.2}s ({:.0} rec/s)",
        documents.len(),
        ul_elapsed,
        ul_rate
    );

    // Optional enrichment: StockEtablissement
    if let Some(ref etab_path) = args.etablissement {
        if !etab_path.exists() {
            return Err(CliError::FileNotFound(etab_path.clone()));
        }
        println!(
            "[2/4] Enriching addresses & SIRET from {}...",
            etab_path.display()
        );
        let etab_start = Instant::now();
        let etab_reader = open_streaming_reader(etab_path).map_err(CliError::Custom)?;
        let count = enrich_with_etablissements(etab_reader, &mut documents)?;
        println!(
            "      -> Enriched {} establishments in {:.2}s",
            count,
            etab_start.elapsed().as_secs_f64()
        );
    } else {
        println!("[2/4] Skipping establishment enrichment (no file provided)");
    }

    // Optional enrichment: RNE Dirigeants
    if let Some(ref dir_path) = args.dirigeants {
        if !dir_path.exists() {
            return Err(CliError::FileNotFound(dir_path.clone()));
        }
        println!(
            "[3/4] Enriching corporate leaders from {}...",
            dir_path.display()
        );
        let dir_start = Instant::now();
        let dir_reader = open_streaming_reader(dir_path).map_err(CliError::Custom)?;
        let count = enrich_with_dirigeants(dir_reader, &mut documents)?;
        println!(
            "      -> Attached {} corporate leaders in {:.2}s",
            count,
            dir_start.elapsed().as_secs_f64()
        );
    } else {
        println!("[3/4] Skipping corporate leader enrichment (no file provided)");
    }

    // Filter by department if requested (with Corsica 20 <-> 2A/2B equivalence)
    if let Some(ref dept) = args.department {
        let dept_upper = dept.trim().to_ascii_uppercase();
        let before_count = documents.len();
        documents.retain(|doc| {
            let matches_commune = doc
                .code_commune
                .as_ref()
                .map(|c| sirene_matcher::are_departments_matching(c.departement(), &dept_upper))
                .unwrap_or(false);
            let matches_postal = doc
                .code_postal
                .as_ref()
                .map(|p| sirene_matcher::are_departments_matching(p.departement(), &dept_upper))
                .unwrap_or(false);
            matches_commune || matches_postal
        });
        println!(
            "      -> Retained {}/{} records in department {}",
            documents.len(),
            before_count,
            dept_upper
        );
    }

    // Compile into compact binary index
    println!(
        "[4/4] Building compact FST & Roaring index ({} records)...",
        documents.len()
    );
    let build_start = Instant::now();
    let mut builder = IndexBuilder::new();
    for doc in documents {
        builder.add_document(doc);
    }

    let index_bytes = builder.build()?;
    let build_elapsed = build_start.elapsed().as_secs_f64();
    println!("      -> Compiled index in {:.2}s", build_elapsed);

    // Write output file
    std::fs::write(&args.output, &index_bytes)?;
    let total_elapsed = start_time.elapsed().as_secs_f64();
    let file_size_mb = index_bytes.len() as f64 / (1024.0 * 1024.0);

    println!(
        "\nSuccess: Index written to '{}' ({:.2} MB) in {:.2}s total.",
        args.output.display(),
        file_size_mb,
        total_elapsed
    );

    Ok(())
}
