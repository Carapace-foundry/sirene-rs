#![deny(unsafe_code)]

//! `sirene-core`: Domain models, strictly validated SIREN/SIRET types and zero-allocation traits.
//!
//! Compliant with INSEE SIRENE V3 and INPI RNE specifications.

pub mod categorie;
pub mod commune;
pub mod error;
pub mod extract;
pub mod luhn;
pub mod naf;
pub mod nic;
pub mod record;
pub mod siren;
pub mod siret;
pub mod statut;

pub use categorie::CategorieJuridique;
pub use commune::{CodeCommune, CodePostal};
pub use error::{
    CategorieJuridiqueError, CodeCommuneError, CodeNafError, CodePostalError, Error,
    EtatAdministratifError, NicError, Result, SirenError, SiretError,
};
pub use extract::{extract_pivot, IdentifiantPivot};
pub use luhn::{
    is_la_poste_siren_bytes, luhn_checksum_ascii, sum_digits_multiple_of_5, validate_siren_bytes,
    validate_siret_bytes, LA_POSTE_SIREN,
};
pub use naf::CodeNaf;
pub use nic::Nic;
pub use record::{DirigeantRecord, EtablissementRecord, UniteLegaleRecord};
pub use siren::Siren;
pub use siret::Siret;
pub use statut::EtatAdministratif;
