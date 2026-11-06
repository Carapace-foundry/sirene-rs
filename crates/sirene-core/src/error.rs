use thiserror::Error;

/// Global error type for `sirene-core`.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum Error {
    #[error("SIREN error: {0}")]
    Siren(#[from] SirenError),

    #[error("SIRET error: {0}")]
    Siret(#[from] SiretError),

    #[error("NIC error: {0}")]
    Nic(#[from] NicError),

    #[error("Code Commune error: {0}")]
    CodeCommune(#[from] CodeCommuneError),

    #[error("Code Postal error: {0}")]
    CodePostal(#[from] CodePostalError),

    #[error("Code NAF error: {0}")]
    CodeNaf(#[from] CodeNafError),

    #[error("Catégorie Juridique error: {0}")]
    CategorieJuridique(#[from] CategorieJuridiqueError),

    #[error("État Administratif error: {0}")]
    EtatAdministratif(#[from] EtatAdministratifError),
}

/// Errors occurring during SIREN parsing and validation.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum SirenError {
    #[error("Invalid SIREN length: expected 9 digits, got {actual}")]
    InvalidLength { actual: usize },

    #[error("Invalid character in SIREN at index {position}: expected digit '0'..'9', got '{ch}'")]
    NonDigitCharacter { position: usize, ch: char },

    #[error("Invalid SIREN Luhn checksum for '{siren}'")]
    InvalidLuhnChecksum { siren: String },
}

/// Errors occurring during SIRET parsing and validation.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum SiretError {
    #[error("Invalid SIRET length: expected 14 digits, got {actual}")]
    InvalidLength { actual: usize },

    #[error("Invalid character in SIRET at index {position}: expected digit '0'..'9', got '{ch}'")]
    NonDigitCharacter { position: usize, ch: char },

    #[error("Invalid SIRET Luhn checksum for '{siret}'")]
    InvalidLuhnChecksum { siret: String },

    #[error("Invalid La Poste SIRET checksum (sum of digits not multiple of 5) for '{siret}'")]
    InvalidLaPosteChecksum { siret: String },

    #[error("Embedded SIREN is invalid: {0}")]
    InvalidSiren(#[from] SirenError),
}

/// Errors occurring during NIC parsing.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum NicError {
    #[error("Invalid NIC length: expected 5 digits, got {actual}")]
    InvalidLength { actual: usize },

    #[error("Invalid character in NIC at index {position}: expected digit '0'..'9', got '{ch}'")]
    NonDigitCharacter { position: usize, ch: char },
}

/// Errors occurring during Code Commune (COG) parsing.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum CodeCommuneError {
    #[error("Invalid Code Commune length: expected 5 characters, got {actual}")]
    InvalidLength { actual: usize },

    #[error("Invalid Code Commune format '{input}': must be 5 digits or Corsica code (2A/2B followed by 3 digits)")]
    InvalidFormat { input: String },
}

/// Errors occurring during Code Postal parsing.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum CodePostalError {
    #[error("Invalid Code Postal length: expected 5 digits, got {actual}")]
    InvalidLength { actual: usize },

    #[error(
        "Invalid character in Code Postal at index {position}: expected digit '0'..'9', got '{ch}'"
    )]
    NonDigitCharacter { position: usize, ch: char },
}

/// Errors occurring during Code NAF parsing.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum CodeNafError {
    #[error("Invalid Code NAF format '{input}': expected 4 digits followed by 1 letter (e.g. '6201Z' or '62.01Z')")]
    InvalidFormat { input: String },
}

/// Errors occurring during Catégorie Juridique parsing.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum CategorieJuridiqueError {
    #[error("Invalid Catégorie Juridique length: expected 4 digits, got {actual}")]
    InvalidLength { actual: usize },

    #[error(
        "Invalid character in Catégorie Juridique at index {position}: expected digit, got '{ch}'"
    )]
    NonDigitCharacter { position: usize, ch: char },
}

/// Errors occurring during État Administratif parsing.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum EtatAdministratifError {
    #[error("Invalid État Administratif code '{code}': expected 'A' (actif) or 'C' (cessé)")]
    InvalidCode { code: String },
}

/// Specialized `Result` type for `sirene-core`.
pub type Result<T> = std::result::Result<T, Error>;
