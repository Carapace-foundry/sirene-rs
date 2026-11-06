use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CliError>;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV parsing error: {0}")]
    Csv(#[from] csv::Error),

    #[error("Index error: {0}")]
    Index(#[from] sirene_index::IndexError),

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("{0}")]
    Custom(String),
}
