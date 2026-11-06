use thiserror::Error;

/// Error type for `sirene-index`.
#[derive(Debug, Error)]
pub enum IndexError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("FST error: {0}")]
    Fst(#[from] fst::Error),

    #[error("Invalid index header magic: expected {expected:?}, found {found:?}")]
    InvalidMagic { expected: [u8; 8], found: [u8; 8] },

    #[error("Unsupported index format version: {version}")]
    UnsupportedVersion { version: u32 },

    #[error("Corrupted index data: {message}")]
    CorruptedIndex { message: String },

    #[error("Document not found with ID {doc_id}")]
    DocumentNotFound { doc_id: u32 },

    #[error("SIREN error: {0}")]
    Siren(#[from] sirene_core::SirenError),

    #[error("SIRET error: {0}")]
    Siret(#[from] sirene_core::SiretError),

    #[error("Code Commune error: {0}")]
    CodeCommune(#[from] sirene_core::CodeCommuneError),

    #[error("Code Postal error: {0}")]
    CodePostal(#[from] sirene_core::CodePostalError),

    #[error("Code NAF error: {0}")]
    CodeNaf(#[from] sirene_core::CodeNafError),
}

/// Specialized Result for index operations.
pub type Result<T> = std::result::Result<T, IndexError>;
