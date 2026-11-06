use sirene_index::SireneIndex;

use crate::args::InspectArgs;
use crate::error::{CliError, Result};

/// Inspects binary `.idx` index file structure, section layout, and document statistics.
pub fn execute(args: InspectArgs) -> Result<()> {
    if !args.index.exists() {
        return Err(CliError::FileNotFound(args.index));
    }

    let file_metadata = std::fs::metadata(&args.index)?;
    let total_file_size = file_metadata.len();

    let index = SireneIndex::open_file(&args.index)?;
    let header = index.header();
    let filters = index.filters();
    let trigrams = index.trigram_index();

    let active_count = filters.active_bitmap.len();
    let ceased_count = (header.num_docs as u64).saturating_sub(active_count);
    let avg_doc_bytes = if header.num_docs > 0 {
        total_file_size as f64 / header.num_docs as f64
    } else {
        0.0
    };

    println!("=======================================================");
    println!("             SIRENE Index Inspection Report            ");
    println!("=======================================================");
    println!("File:             {}", args.index.display());
    println!(
        "Total Size:       {} bytes ({:.2} MB)",
        total_file_size,
        total_file_size as f64 / (1024.0 * 1024.0)
    );
    println!("Format Version:   {}", header.version);
    println!("Total Documents:  {}", header.num_docs);
    println!(
        "Active ('A'):     {} ({:.1}%)",
        active_count,
        if header.num_docs > 0 {
            active_count as f64 / header.num_docs as f64 * 100.0
        } else {
            0.0
        }
    );
    println!(
        "Ceased ('C'):     {} ({:.1}%)",
        ceased_count,
        if header.num_docs > 0 {
            ceased_count as f64 / header.num_docs as f64 * 100.0
        } else {
            0.0
        }
    );
    println!("Avg Doc Size:     {:.1} bytes/document", avg_doc_bytes);
    println!(
        "Indexed Depts:    {} distinct departments",
        filters.dept_bitmaps.len()
    );
    println!("Distinct Trigrams:{}", trigrams.postings.len());
    println!();

    println!("--------------------------------------------------------------------------------");
    println!(
        "{:<22} | {:<12} | {:<14} | {:<8}",
        "Index Section", "Offset (B)", "Length (B)", "Share (%)"
    );
    println!("-----------------------+--------------+----------------+---------");

    let sections: [(&str, (u64, u64)); 12] = [
        ("Header", (0, 192)),
        ("SIREN Table", header.siren_table),
        ("SIRET Table", header.siret_table),
        ("Metadata Table", header.metadata_table),
        ("Names Blob", header.names_blob),
        ("Name Offsets", header.name_offsets),
        ("FST SIREN", header.fst_siren),
        ("FST Terms", header.fst_terms),
        ("Term Postings", header.term_postings),
        ("Trigram Index", header.trigram_data),
        ("Filter Bitmaps", header.filter_data),
        ("Dirigeants Data", header.dirigeants_data),
    ];

    for (name, (offset, len)) in sections {
        let pct = if total_file_size > 0 {
            (len as f64 / total_file_size as f64) * 100.0
        } else {
            0.0
        };
        println!(
            "{:<22} | {:<12} | {:<14} | {:<7.2}%",
            name, offset, len, pct
        );
    }
    println!("--------------------------------------------------------------------------------\n");

    Ok(())
}
