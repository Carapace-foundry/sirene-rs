use std::io::{self, BufRead, Write};
use std::time::Instant;

use sirene_core::CodeNaf;
use sirene_index::SireneIndex;
use sirene_matcher::{LocationQuery, SearchQuery};
use sirene_normalize::Normalizer;

use crate::args::InteractiveArgs;
use crate::error::{CliError, Result};

/// Runs an interactive REPL session allowing rapid fuzzy search queries with instant latency feedback.
pub fn execute(args: InteractiveArgs) -> Result<()> {
    if !args.index.exists() {
        return Err(CliError::FileNotFound(args.index));
    }

    println!("Loading index '{}'...", args.index.display());
    let index = SireneIndex::open_file(&args.index)?;
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    run_interactive(&mut stdin.lock(), &mut stdout, &index)
}

/// Runs the interactive loop with pluggable reader and writer for testing.
pub fn run_interactive<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    index: &SireneIndex,
) -> Result<()> {
    let normalizer = Normalizer::new();

    writeln!(
        writer,
        "======================================================="
    )?;
    writeln!(
        writer,
        "      SIRENE Interactive Search Engine (REPL)          "
    )?;
    writeln!(
        writer,
        "======================================================="
    )?;
    writeln!(
        writer,
        "Index: {} enterprises loaded in memory (mmap)",
        index.num_documents()
    )?;
    writeln!(writer, "Syntax:")?;
    writeln!(
        writer,
        "  - Type any enterprise name, acronym or SIREN/SIRET"
    )?;
    writeln!(
        writer,
        "  - Append '@<dep/postal>' for location (e.g. 'Boulangerie @33000')"
    )?;
    writeln!(
        writer,
        "  - Append '#<naf>' for activity (e.g. 'Tech Solutions #62.01Z')"
    )?;
    writeln!(
        writer,
        "  - Append '$<dirigeant>' for representative (e.g. 'Acme $Thomas Durand')"
    )?;
    writeln!(writer, "  - Type ':q' or ':quit' to exit\n")?;

    loop {
        write!(writer, "sirene> ")?;
        writer.flush()?;

        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break; // EOF
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        if input == ":q" || input == ":quit" || input == "exit" {
            writeln!(writer, "Exiting.")?;
            break;
        }

        // Parse inline filters (@location, #naf, $dirigeant)
        let mut query_parts = Vec::new();
        let mut location_str: Option<String> = None;
        let mut naf_str: Option<String> = None;
        let mut dir_str: Option<String> = None;

        for token in input.split_whitespace() {
            if let Some(stripped) = token.strip_prefix('@') {
                location_str = Some(stripped.to_string());
            } else if let Some(stripped) = token.strip_prefix('#') {
                naf_str = Some(stripped.to_string());
            } else if let Some(stripped) = token.strip_prefix('$') {
                dir_str = Some(stripped.to_string());
            } else {
                query_parts.push(token);
            }
        }

        let query_text = query_parts.join(" ");
        if query_text.is_empty() {
            continue;
        }

        let norm_query = normalizer.normalize_query(&query_text);
        let loc = location_str.as_deref().and_then(LocationQuery::parse);
        let naf = naf_str.as_deref().and_then(|s| CodeNaf::parse(s).ok());

        let search_query = SearchQuery {
            query: &norm_query,
            location: loc,
            naf,
            dirigeant: dir_str.as_deref(),
            strict_location: false,
        };

        let start = Instant::now();
        let results = index.search(&search_query, 5);
        let elapsed = start.elapsed();

        writeln!(
            writer,
            "-> Found {} results in {:.2?}:",
            results.len(),
            elapsed
        )?;

        for (i, res) in results.iter().enumerate() {
            let siret_str = res
                .siret
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("      -       ");

            let pivot_tag = if res.is_exact_pivot { " [EXACT]" } else { "" };

            writeln!(
                writer,
                "   [{}] {:<9} | {:<14} | {:<28} | Score: {:.3}{}",
                i + 1,
                res.siren.as_str(),
                siret_str,
                res.denomination,
                res.score,
                pivot_tag
            )?;
        }
        writeln!(writer)?;
    }

    Ok(())
}
