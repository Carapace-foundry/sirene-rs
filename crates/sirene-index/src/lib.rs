#![deny(unsafe_code)]

//! `sirene-index`: Compact inverted index, FST dictionaries, Roaring Bitmaps, and zero-copy mmap/WASM engine.
//!
//! Compliant with INSEE SIRENE V3 and DINUM specifications:
//! - Finite State Transducers (`fst`) for sub-microsecond exact SIREN lookup and term dictionaries.
//! - Roaring Bitmaps (`roaring`) for compressed posting lists and SIMD strict filters (active, department).
//! - 24-bit packed trigram inverted index with Jaccard pre-filtering.
//! - Memory-mapped file I/O for native server deployments (`memmap2`) and direct byte slices for WebAssembly.

pub mod builder;
pub mod document;
pub mod error;
pub mod filters;
pub mod reader;
pub mod storage;
pub mod trigram_index;

pub use builder::IndexBuilder;
pub use document::{IndexDocument, IndexedDocumentView, SearchResult};
pub use error::{IndexError, Result};
pub use filters::FilterIndex;
pub use reader::SireneIndex;
pub use storage::{IndexHeader, IndexStorage};
pub use trigram_index::TrigramIndex;
