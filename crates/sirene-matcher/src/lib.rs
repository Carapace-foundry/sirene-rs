#![deny(unsafe_code)]

//! `sirene-matcher`: High-performance similarity algorithms and multi-criteria composite scoring.
//!
//! Compliant with DINUM (Annuaire des Entreprises) and INSEE SIRENE / INPI RNE specifications:
//! - Jaro-Winkler string similarity with stack bitmasks for zero heap allocation.
//! - Damerau-Levenshtein distance with adjacent transposition support.
//! - Compact 24-bit $q$-gram packing and linear-time Jaccard similarity.
//! - Soft token matching with word inversion tolerance and acronym detection.
//! - Multi-criteria scoring formula:
//!   $$\text{Score}_{\text{final}} = w_{\text{nom}} \cdot S_{\text{nom}} + w_{\text{loc}} \cdot S_{\text{loc}} + w_{\text{naf}} \cdot S_{\text{naf}} + w_{\text{dir}} \cdot S_{\text{dir}} + B_{\text{actif}}$$
//! - Pivot short-circuiting on exact SIREN / SIRET detection.

pub mod candidate;
pub mod damerau;
pub mod jaro_winkler;
pub mod qgram;
pub mod scoring;
pub mod token_matcher;

pub use candidate::{EntityMatcher, MatchCandidate, ScoredCandidate, SearchQuery};
pub use damerau::{damerau_levenshtein, damerau_levenshtein_similarity};
pub use jaro_winkler::{jaro_similarity, jaro_winkler_similarity};
pub use qgram::{jaccard_similarity, jaccard_similarity_sets, pack_trigram, trigrams};
pub use scoring::{
    are_departments_matching, bonus_actif, compute_composite_score, score_activite,
    score_dirigeants, score_localisation, CandidateScoreInput, LocationQuery, QueryScoreInput,
    ScoringWeights, SubScores,
};
pub use token_matcher::{is_acronym_of, score_denomination, token_overlap_similarity};
