use sirene_core::{CodeNaf, Siren, Siret};
use sirene_index::{IndexedDocumentView, SearchResult, SireneIndex};
use sirene_matcher::{LocationQuery, SearchQuery};
use sirene_normalize::Normalizer;
use wasm_bindgen::prelude::*;

use crate::types::WasmSearchOptions;

/// Main search engine instance embedded in WebAssembly memory.
#[wasm_bindgen]
pub struct SireneWasmEngine {
    index: SireneIndex,
    normalizer: Normalizer,
}

impl SireneWasmEngine {
    /// Loads an index from a byte buffer directly in Rust memory.
    pub fn load_from_slice(bytes: &[u8]) -> Result<SireneWasmEngine, sirene_index::IndexError> {
        let index = SireneIndex::from_vec(bytes.to_vec())?;
        Ok(SireneWasmEngine {
            index,
            normalizer: Normalizer::new(),
        })
    }

    /// Internal search returning native Rust search results.
    pub fn search_native(&self, query: &str, options: &WasmSearchOptions) -> Vec<SearchResult> {
        let norm_query = self.normalizer.normalize_query(query);
        let loc = options.location.as_deref().and_then(LocationQuery::parse);
        let naf = options
            .naf
            .as_deref()
            .and_then(|s| CodeNaf::parse(s.trim()).ok());

        let search_query = SearchQuery {
            query: &norm_query,
            location: loc,
            naf,
            dirigeant: options.dirigeant.as_deref(),
            strict_location: options.strict_location.unwrap_or(false),
        };

        let top_k = options.top_k.unwrap_or(10);
        self.index.search(&search_query, top_k)
    }

    /// Internal document lookup.
    pub fn get_document_native(&self, doc_id: u32) -> Option<IndexedDocumentView<'_>> {
        self.index.get_document(doc_id)
    }

    /// Internal SIREN lookup.
    pub fn find_by_siren_native(&self, siren: Siren) -> Option<IndexedDocumentView<'_>> {
        let doc_id = self.index.find_by_siren(siren)?;
        self.index.get_document(doc_id)
    }

    /// Internal SIRET lookup.
    pub fn find_by_siret_native(&self, siret: Siret) -> Option<IndexedDocumentView<'_>> {
        let doc_id = self.index.find_by_siret(siret)?;
        self.index.get_document(doc_id)
    }
}

#[wasm_bindgen]
impl SireneWasmEngine {
    /// Instantiates the search engine by loading a serialized binary index from a byte array.
    ///
    /// Accepts a `Uint8Array` in JavaScript / TypeScript.
    #[wasm_bindgen(js_name = load)]
    pub fn load(bytes: &[u8]) -> Result<SireneWasmEngine, JsValue> {
        Self::load_from_slice(bytes)
            .map_err(|e| JsValue::from_str(&format!("Failed to load SIRENE index: {}", e)))
    }

    /// Searches the indexed enterprise database using fuzzy matching and multi-criteria scoring.
    ///
    /// Returns an array of search results with composite scores and subscores breakdown.
    #[wasm_bindgen(js_name = search)]
    pub fn search(&self, query: &str, options: JsValue) -> Result<JsValue, JsValue> {
        let opts: WasmSearchOptions = if options.is_undefined() || options.is_null() {
            WasmSearchOptions::default()
        } else {
            serde_wasm_bindgen::from_value(options)
                .map_err(|e| JsValue::from_str(&format!("Invalid search options: {}", e)))?
        };

        let results = self.search_native(query, &opts);
        serde_wasm_bindgen::to_value(&results)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Instant exact resolution by 9-digit SIREN via the embedded FST index.
    #[wasm_bindgen(js_name = findBySiren)]
    pub fn find_by_siren(&self, siren_str: &str) -> Result<JsValue, JsValue> {
        let siren = match Siren::parse(siren_str.trim()) {
            Ok(s) => s,
            Err(_) => return Ok(JsValue::NULL),
        };

        match self.find_by_siren_native(siren) {
            Some(doc) => serde_wasm_bindgen::to_value(&doc)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
            None => Ok(JsValue::NULL),
        }
    }

    /// Instant exact resolution by 14-digit SIRET via the embedded FST index.
    #[wasm_bindgen(js_name = findBySiret)]
    pub fn find_by_siret(&self, siret_str: &str) -> Result<JsValue, JsValue> {
        let siret = match Siret::parse(siret_str.trim()) {
            Ok(s) => s,
            Err(_) => return Ok(JsValue::NULL),
        };

        match self.find_by_siret_native(siret) {
            Some(doc) => serde_wasm_bindgen::to_value(&doc)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
            None => Ok(JsValue::NULL),
        }
    }

    /// Decodes a zero-copy document view for a given internal document ID.
    #[wasm_bindgen(js_name = getDocument)]
    pub fn get_document(&self, doc_id: u32) -> Result<JsValue, JsValue> {
        match self.get_document_native(doc_id) {
            Some(doc) => serde_wasm_bindgen::to_value(&doc)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
            None => Ok(JsValue::NULL),
        }
    }

    /// Returns the total number of indexed enterprises.
    #[wasm_bindgen(js_name = numDocuments)]
    pub fn num_documents(&self) -> u32 {
        self.index.num_documents()
    }
}
