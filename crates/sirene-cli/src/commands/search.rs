use std::time::Instant;

use sirene_core::CodeNaf;
use sirene_index::SireneIndex;
use sirene_matcher::{LocationQuery, SearchQuery};
use sirene_normalize::Normalizer;

use crate::args::SearchArgs;
use crate::error::{CliError, Result};

/// Executes a one-shot search command against a pre-built binary SIRENE index.
pub fn execute(args: SearchArgs) -> Result<()> {
    if !args.index.exists() {
        return Err(CliError::FileNotFound(args.index));
    }

    let index = SireneIndex::open_file(&args.index)?;
    let normalizer = Normalizer::new();
    let norm_query = normalizer.normalize_query(&args.query);

    let location = args.location.as_deref().and_then(LocationQuery::parse);
    let naf = args
        .naf
        .as_deref()
        .and_then(|s| CodeNaf::parse(s.trim()).ok());

    let search_query = SearchQuery {
        query: &norm_query,
        location,
        naf,
        dirigeant: args.dirigeant.as_deref(),
        strict_location: false,
    };

    let start = Instant::now();
    let results = index.search(&search_query, args.top_k);
    let elapsed = start.elapsed();

    if args.json {
        let json_output = serde_json::to_string_pretty(&results)
            .map_err(|e| CliError::Custom(format!("JSON serialization error: {}", e)))?;
        println!("{}", json_output);
    } else {
        println!(
            "Search Query: \"{}\" | Location: {:?} | NAF: {:?} | Dirigeant: {:?}",
            args.query, args.location, args.naf, args.dirigeant
        );
        println!(
            "Search completed in {:?} ({} result{}):\n",
            elapsed,
            results.len(),
            if results.len() == 1 { "" } else { "s" }
        );

        if results.is_empty() {
            println!("  No matching enterprise found.");
            return Ok(());
        }

        // Print header
        println!(
            "{:<4} | {:<9} | {:<14} | {:<32} | {:<6} | {:<5} | {:<5} | {:<5} | {:<5}",
            "Rank", "SIREN", "SIRET", "Denomination", "Score", "S_nom", "S_loc", "S_naf", "S_dir"
        );
        println!(
            "{:-<4}-+-{:-<9}-+-{:-<14}-+-{:-<32}-+-{:-<6}-+-{:-<5}-+-{:-<5}-+-{:-<5}-+-{:-<5}",
            "", "", "", "", "", "", "", "", ""
        );

        for (rank, res) in results.iter().enumerate() {
            let siret_str = res.siret.as_ref().map(|s| s.as_str()).unwrap_or("N/A");

            let denom_display = if res.denomination.chars().count() > 32 {
                let s: String = res.denomination.chars().take(29).collect();
                format!("{}...", s)
            } else {
                res.denomination.clone()
            };

            let pivot_tag = if res.is_exact_pivot { " [EXACT]" } else { "" };

            println!(
                "{:<4} | {:<9} | {:<14} | {:<32} | {:<6.3} | {:<5.2} | {:<5.2} | {:<5.2} | {:<5.2}{}",
                rank + 1,
                res.siren.as_str(),
                siret_str,
                denom_display,
                res.score,
                res.sub_scores.score_nom,
                res.sub_scores.score_loc,
                res.sub_scores.score_naf,
                res.sub_scores.score_dir,
                pivot_tag
            );
        }
        println!();
    }

    Ok(())
}
