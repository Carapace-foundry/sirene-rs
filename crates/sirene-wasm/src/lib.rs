#![deny(unsafe_code)]

//! Lightweight WebAssembly bindings for SIRENE / RNE fuzzy matching and search engine.

pub mod engine;
pub mod types;

pub use engine::SireneWasmEngine;
pub use types::{WasmPivot, WasmSearchOptions};

use sirene_core::{Siren, Siret};
use sirene_normalize::Normalizer;
use smallvec::SmallVec;
use wasm_bindgen::prelude::*;

/// Validates a 9-digit SIREN according to the official Luhn modulo 10 algorithm.
#[wasm_bindgen(js_name = validateSiren)]
pub fn validate_siren(siren: &str) -> bool {
    Siren::parse(siren).is_ok()
}

/// Validates a 14-digit SIRET according to the Luhn algorithm with La Poste derogation.
#[wasm_bindgen(js_name = validateSiret)]
pub fn validate_siret(siret: &str) -> bool {
    Siret::parse(siret).is_ok()
}

/// Normalizes raw enterprise queries (Unicode folding, diacritic stripping, legal noise removal, stop words).
#[wasm_bindgen(js_name = normalizeQuery)]
pub fn normalize_query(query: &str) -> String {
    let normalizer = Normalizer::new();
    normalizer.normalize_for_indexing(query)
}

/// Native helper to detect and extract pivot SIREN / SIRET.
pub fn extract_pivot_native(query: &str) -> Option<WasmPivot> {
    match sirene_core::extract_pivot(query) {
        Some(sirene_core::IdentifiantPivot::Siren(s)) => Some(WasmPivot::Siren(s.to_string())),
        Some(sirene_core::IdentifiantPivot::Siret(s)) => Some(WasmPivot::Siret(s.to_string())),
        None => None,
    }
}

/// Detects and extracts a valid SIREN (9 digits) or SIRET (14 digits) pivot identifier from free text.
#[wasm_bindgen(js_name = extractPivot)]
pub fn extract_pivot(query: &str) -> Result<JsValue, JsValue> {
    match extract_pivot_native(query) {
        Some(pivot) => serde_wasm_bindgen::to_value(&pivot)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
        None => Ok(JsValue::NULL),
    }
}

/// Formats a 9-digit SIREN into standard 3-3-3 spaced format (e.g. "732 829 320").
#[wasm_bindgen(js_name = formatSiren)]
pub fn format_siren(siren: &str) -> Option<String> {
    Siren::parse(siren).ok().map(|s| s.formatted())
}

/// Formats a 14-digit SIRET into standard 3-3-3-5 spaced format (e.g. "732 829 320 00074").
#[wasm_bindgen(js_name = formatSiret)]
pub fn format_siret(siret: &str) -> Option<String> {
    Siret::parse(siret).ok().map(|s| s.formatted())
}

/// Checks if a SIREN number belongs to La Poste (SIREN 356 000 000).
#[wasm_bindgen(js_name = isLaPosteSiren)]
pub fn is_la_poste_siren(siren: &str) -> bool {
    Siren::parse(siren)
        .map(|s| s.is_la_poste())
        .unwrap_or(false)
}

/// Calculates Jaro-Winkler string similarity between two strings.
#[wasm_bindgen(js_name = jaroWinklerSimilarity)]
pub fn jaro_winkler_similarity(s1: &str, s2: &str) -> f32 {
    sirene_matcher::jaro_winkler_similarity(s1, s2)
}

/// Evaluates corporate denomination score (Jaro-Winkler + unordered token overlap).
#[wasm_bindgen(js_name = scoreDenominations)]
pub fn score_denominations(query: &str, candidate: &str) -> f32 {
    let normalizer = Normalizer::new();
    let q_norm = normalizer.normalize_for_indexing(query);
    let c_norm = normalizer.normalize_for_indexing(candidate);
    let q_tokens: SmallVec<[&str; 8]> = q_norm.split(' ').filter(|s| !s.is_empty()).collect();
    let c_tokens: SmallVec<[&str; 8]> = c_norm.split(' ').filter(|s| !s.is_empty()).collect();
    sirene_matcher::score_denomination(&q_norm, &q_tokens, &c_norm, &c_tokens, 0.50)
}
