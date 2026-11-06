use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// High-performance deterministic French SIRENE / RNE search and indexing engine.
#[derive(Debug, Parser)]
#[command(
    name = "sirene",
    version,
    about = "Ultra-fast, deterministic SIRENE / RNE fuzzy matching and indexing in Rust",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Ingest raw INSEE CSV / ZIP files and build a compact binary search index
    Index(IndexArgs),

    /// Search an indexed SIRENE database using fuzzy matching and multi-criteria scoring
    Search(SearchArgs),

    /// Launch an interactive real-time fuzzy search session
    Interactive(InteractiveArgs),

    /// Inspect binary index metadata, segment sizes, and document statistics
    Inspect(InspectArgs),

    /// Launch high-performance DINUM / INSEE REST HTTP API microservice
    Serve(ServeArgs),

    /// Batch reconcile and match an external CSV file against SIRENE (replaces Python record-linkage)
    Match(MatchArgs),
}

#[derive(Debug, Args)]
pub struct IndexArgs {
    /// Path to raw StockUniteLegale CSV, ZIP or GZ file
    #[arg(short = 'u', long = "unite-legale")]
    pub unite_legale: PathBuf,

    /// Optional path to raw StockEtablissement CSV, ZIP or GZ file (for address and SIRET enrichment)
    #[arg(short = 'e', long = "etablissement")]
    pub etablissement: Option<PathBuf>,

    /// Optional path to RNE Dirigeants CSV file
    #[arg(short = 'd', long = "dirigeants")]
    pub dirigeants: Option<PathBuf>,

    /// Output binary index file path
    #[arg(short = 'o', long = "output", default_value = "sirene.idx")]
    pub output: PathBuf,

    /// Only index active enterprises (etatAdministratif = 'A')
    #[arg(long = "active-only")]
    pub active_only: bool,

    /// Filter to index only entities in a specific department (e.g. "75", "33", "2A")
    #[arg(long = "department")]
    pub department: Option<String>,

    /// Maximum number of records to ingest (useful for testing or scoped extracts)
    #[arg(short = 'l', long = "limit")]
    pub limit: Option<usize>,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Path to binary index file
    #[arg(short = 'i', long = "index", default_value = "sirene.idx")]
    pub index: PathBuf,

    /// Query search text (business name, acronym, or SIREN/SIRET)
    pub query: String,

    /// Optional geographic location (department e.g. "75", postal code e.g. "75001", or commune COG)
    #[arg(short = 'l', long = "location")]
    pub location: Option<String>,

    /// Optional NAF activity code (e.g. "62.01Z")
    #[arg(long = "naf")]
    pub naf: Option<String>,

    /// Optional corporate representative / director name (e.g. "Thomas Durand")
    #[arg(long = "dirigeant")]
    pub dirigeant: Option<String>,

    /// Maximum number of ranked results to return
    #[arg(short = 'k', long = "top-k", default_value = "10")]
    pub top_k: usize,

    /// Output results as JSON
    #[arg(long = "json")]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct InteractiveArgs {
    /// Path to binary index file
    #[arg(short = 'i', long = "index", default_value = "sirene.idx")]
    pub index: PathBuf,
}

#[derive(Debug, Args)]
pub struct InspectArgs {
    /// Path to binary index file
    #[arg(short = 'i', long = "index", default_value = "sirene.idx")]
    pub index: PathBuf,
}

#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Path to binary index file
    #[arg(short = 'i', long = "index", default_value = "sirene.idx")]
    pub index: PathBuf,

    /// HTTP port to listen on
    #[arg(short = 'p', long = "port", default_value = "8080")]
    pub port: u16,

    /// Bind host address
    #[arg(short = 'b', long = "host", default_value = "0.0.0.0")]
    pub host: String,

    /// Number of worker threads (default: CPU cores count)
    #[arg(short = 'w', long = "workers")]
    pub workers: Option<usize>,
}

#[derive(Debug, Args)]
pub struct MatchArgs {
    /// Path to binary index file
    #[arg(short = 'i', long = "index", default_value = "sirene.idx")]
    pub index: PathBuf,

    /// Path to input CSV file to match (or "-" for stdin)
    #[arg(short = 'f', long = "input", default_value = "-")]
    pub input: PathBuf,

    /// Column name containing company/entity denomination (default: "nom")
    #[arg(long = "col-nom", default_value = "nom")]
    pub col_nom: String,

    /// Optional column name containing postal code or department (default: "code_postal")
    #[arg(long = "col-location")]
    pub col_location: Option<String>,

    /// Optional column name containing NAF activity code
    #[arg(long = "col-naf")]
    pub col_naf: Option<String>,

    /// Path to output matched CSV file (or "-" for stdout)
    #[arg(short = 'o', long = "output", default_value = "matched.csv")]
    pub output: PathBuf,

    /// Minimum score threshold in [0.0, 1.0] to accept a match
    #[arg(long = "threshold", default_value = "0.50")]
    pub threshold: f32,
}
