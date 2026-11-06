#![deny(unsafe_code)]

use clap::Parser;
use sirene_cli::args::{Cli, Commands};
use sirene_cli::commands;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Index(args) => commands::index::execute(args),
        Commands::Search(args) => commands::search::execute(args),
        Commands::Interactive(args) => commands::interactive::execute(args),
        Commands::Inspect(args) => commands::inspect::execute(args),
        Commands::Serve(args) => commands::serve::execute(args),
        Commands::Match(args) => commands::match_batch::execute(args),
    };

    if let Err(err) = result {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}
