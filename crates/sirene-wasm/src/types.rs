use serde::{Deserialize, Serialize};

/// Optional parameters for WebAssembly search queries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WasmSearchOptions {
    /// Optional geographic location (postal code, commune COG, or department).
    pub location: Option<String>,

    /// Optional NAF activity code (e.g. "62.01Z").
    pub naf: Option<String>,

    /// Optional corporate leader / representative name.
    pub dirigeant: Option<String>,

    /// Maximum number of ranked results (default: 10).
    #[serde(alias = "top_k")]
    pub top_k: Option<usize>,

    /// If true, entities outside the specified location are strictly rejected.
    #[serde(alias = "strict_location")]
    pub strict_location: Option<bool>,
}

/// Extracted pivot identifier result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum WasmPivot {
    Siren(String),
    Siret(String),
}
