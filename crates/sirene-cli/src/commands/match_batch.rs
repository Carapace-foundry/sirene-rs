use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::time::Instant;

use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use sirene_core::CodeNaf;
use sirene_index::SireneIndex;
use sirene_matcher::{LocationQuery, SearchQuery};
use sirene_normalize::Normalizer;

use crate::args::MatchArgs;
use crate::error::{CliError, Result};

/// Executes high-throughput batch reconciliation matching of an external CSV against SIRENE.
///
/// Designed as a direct drop-in replacement for legacy Python Record Linkage / Dedupe scripts.
/// Supports both file paths and Unix pipes (using "-" for stdin/stdout).
pub fn execute(args: MatchArgs) -> Result<()> {
    let is_stdin = args.input.as_os_str() == "-";
    let is_stdout = args.output.as_os_str() == "-";

    if !args.index.exists() {
        return Err(CliError::FileNotFound(args.index));
    }
    if !is_stdin && !args.input.exists() {
        return Err(CliError::FileNotFound(args.input));
    }

    if !is_stdout {
        println!(
            "Loading SIRENE binary index from {}...",
            args.index.display()
        );
    }
    let index = SireneIndex::open_file(&args.index)?;
    let normalizer = Normalizer::new();

    let reader: Box<dyn std::io::Read> = if is_stdin {
        Box::new(std::io::stdin())
    } else {
        if !is_stdout {
            println!("Opening input file {}...", args.input.display());
        }
        Box::new(File::open(&args.input)?)
    };

    let mut csv_reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(BufReader::with_capacity(256 * 1024, reader));

    let headers = csv_reader.headers()?.clone();

    // Identify target column indices
    let nom_col_lower = args.col_nom.trim().to_ascii_lowercase();
    let nom_idx = headers
        .iter()
        .position(|col| col.trim().to_ascii_lowercase() == nom_col_lower)
        .ok_or_else(|| {
            CliError::Custom(format!(
                "Required denomination column '{}' not found in CSV header",
                args.col_nom
            ))
        })?;

    let loc_idx = args.col_location.as_ref().and_then(|col_name| {
        let lower = col_name.trim().to_ascii_lowercase();
        headers
            .iter()
            .position(|col| col.trim().to_ascii_lowercase() == lower)
    });

    let naf_idx = args.col_naf.as_ref().and_then(|col_name| {
        let lower = col_name.trim().to_ascii_lowercase();
        headers
            .iter()
            .position(|col| col.trim().to_ascii_lowercase() == lower)
    });

    // Prepare output CSV
    let mut out_headers = headers.clone();
    out_headers.push_field("siren_match");
    out_headers.push_field("siret_match");
    out_headers.push_field("denomination_match");
    out_headers.push_field("score_match");
    out_headers.push_field("is_exact_pivot");

    let writer: Box<dyn std::io::Write> = if is_stdout {
        Box::new(std::io::stdout())
    } else {
        Box::new(File::create(&args.output)?)
    };

    let mut csv_writer = WriterBuilder::new()
        .has_headers(true)
        .from_writer(BufWriter::with_capacity(256 * 1024, writer));

    csv_writer.write_record(&out_headers)?;

    if !is_stdout {
        println!(
            "Reconciling records against {} indexed enterprises (threshold: {:.2})...",
            index.num_documents(),
            args.threshold
        );
    }

    let start_time = Instant::now();
    let mut total_records = 0usize;
    let mut matched_records = 0usize;
    let mut record = StringRecord::new();

    while csv_reader.read_record(&mut record)? {
        total_records += 1;

        let query_raw = record.get(nom_idx).unwrap_or("").trim();
        let loc_str = loc_idx.and_then(|i| record.get(i)).map(|s| s.trim());
        let naf_str = naf_idx.and_then(|i| record.get(i)).map(|s| s.trim());

        let mut out_record = record.clone();

        if query_raw.is_empty() {
            out_record.push_field("");
            out_record.push_field("");
            out_record.push_field("");
            out_record.push_field("0.0");
            out_record.push_field("false");
            csv_writer.write_record(&out_record)?;
            continue;
        }

        let norm_query = normalizer.normalize_query(query_raw);
        let loc = loc_str.and_then(LocationQuery::parse);
        let naf = naf_str.and_then(|s| CodeNaf::parse(s).ok());

        let search_query = SearchQuery {
            query: &norm_query,
            location: loc,
            naf,
            dirigeant: None,
            strict_location: false,
        };

        let results = index.search(&search_query, 1);

        if let Some(best) = results.first() {
            if best.score >= args.threshold {
                matched_records += 1;
                out_record.push_field(best.siren.as_str());
                out_record.push_field(best.siret.as_ref().map(|s| s.as_str()).unwrap_or(""));
                out_record.push_field(&best.denomination);
                out_record.push_field(&format!("{:.3}", best.score));
                out_record.push_field(if best.is_exact_pivot { "true" } else { "false" });
            } else {
                out_record.push_field("");
                out_record.push_field("");
                out_record.push_field("");
                out_record.push_field(&format!("{:.3}", best.score));
                out_record.push_field("false");
            }
        } else {
            out_record.push_field("");
            out_record.push_field("");
            out_record.push_field("");
            out_record.push_field("0.0");
            out_record.push_field("false");
        }

        csv_writer.write_record(&out_record)?;

        if total_records % 25_000 == 0 {
            let elapsed = start_time.elapsed().as_secs_f64();
            let rate = if elapsed > 0.0 {
                total_records as f64 / elapsed
            } else {
                0.0
            };
            if !is_stdout {
                println!(
                    "  Processed {:>7} rows... ({:>5} matches, {:.0} rows/s)",
                    total_records, matched_records, rate
                );
            }
        }
    }

    csv_writer.flush()?;

    let total_elapsed = start_time.elapsed().as_secs_f64();
    let avg_rate = if total_elapsed > 0.0 {
        total_records as f64 / total_elapsed
    } else {
        0.0
    };

    if !is_stdout {
        println!("\n=== Reconciliation Complete ===");
        println!("Total Rows:      {}", total_records);
        println!(
            "Matched:         {} ({:.1}%)",
            matched_records,
            if total_records > 0 {
                (matched_records as f64 / total_records as f64) * 100.0
            } else {
                0.0
            }
        );
        println!("Elapsed Time:    {:.2}s", total_elapsed);
        println!("Processing Rate: {:.0} records/sec", avg_rate);
        println!("Output File:     {}", args.output.display());
    }

    Ok(())
}
