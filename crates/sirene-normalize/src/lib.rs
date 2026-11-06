#![deny(unsafe_code)]

//! `sirene-normalize`: Deterministic French lexical normalization and legal noise removal pipeline.
//!
//! Compliant with DINUM (Annuaire des Entreprises / api-recherche-entreprises) standards:
//! - Complete Unicode diacritic folding & ASCII lowercasing (with zero-allocation fast path).
//! - French legal form stripping (`SARL`, `SAS`, `SCI`, `SELARL`, `SA`, `EURL`, `SASU`, etc.).
//! - Abbreviation resolution (`STE` -> `SOCIETE`, `ETS` -> `ETABLISSEMENTS`, `CIE` -> `COMPAGNIE`, `BTP` -> `BATIMENT TRAVAUX PUBLICS`).
//! - Elision prefixes (`d'`, `l'`, `qu'`, etc.) and stop-word filtering (`de`, `du`, `des`, `et`, `le`, `la`, `les`, etc.).
//! - Stack-allocated token buffers via `SmallVec`.

pub mod ascii;
pub mod legal;
pub mod pipeline;
pub mod stopwords;
pub mod tokenizer;

pub use ascii::{is_ascii_clean, to_ascii_folded};
pub use legal::{expand_abbreviation, is_legal_form};
pub use pipeline::{NormalizedQuery, Normalizer};
pub use stopwords::{is_elision_prefix, is_noise_word, is_stop_word};
pub use tokenizer::{Tokenizer, TokenizerOptions};
