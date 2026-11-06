use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};
use std::sync::Arc;

use crate::error::IndexError;

pub const INDEX_MAGIC: &[u8; 8] = b"SIRENE01";
pub const CURRENT_VERSION: u32 = 1;
pub const HEADER_SIZE: usize = 192;

/// Memory storage abstraction supporting in-memory buffers, static slices, and mmap.
pub enum IndexStorage {
    Owned(Vec<u8>),
    Static(&'static [u8]),
    #[cfg(not(target_arch = "wasm32"))]
    Mmap(memmap2::Mmap),
}

impl IndexStorage {
    /// Returns a direct byte slice of the entire index data.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Owned(v) => v.as_slice(),
            Self::Static(s) => s,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Mmap(m) => m.as_ref(),
        }
    }
}

/// A validated reference-counted window into an `IndexStorage`.
///
/// Implements `AsRef<[u8]>` to allow zero-copy FSTs and readers to safely borrow
/// storage segments without lifetime forgery or unsafe pointer manipulation.
#[derive(Clone)]
pub struct StorageSlice {
    storage: Arc<IndexStorage>,
    start: usize,
    len: usize,
}

impl StorageSlice {
    /// Creates a bounds-checked storage slice.
    pub fn new(storage: Arc<IndexStorage>, start: usize, len: usize) -> Result<Self, IndexError> {
        let total_len = storage.as_bytes().len();
        let end = start
            .checked_add(len)
            .ok_or_else(|| IndexError::CorruptedIndex {
                message: format!(
                    "Storage slice offset overflow: start={}, len={}",
                    start, len
                ),
            })?;
        if end > total_len {
            return Err(IndexError::CorruptedIndex {
                message: format!(
                    "Storage slice out of bounds: range [{}..{}] exceeds storage size {}",
                    start, end, total_len
                ),
            });
        }
        Ok(Self {
            storage,
            start,
            len,
        })
    }
}

impl AsRef<[u8]> for StorageSlice {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.storage.as_bytes()[self.start..self.start + self.len]
    }
}

/// Header containing metadata table offsets and segment lengths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexHeader {
    pub version: u32,
    pub num_docs: u32,
    pub siren_table: (u64, u64),
    pub siret_table: (u64, u64),
    pub metadata_table: (u64, u64),
    pub names_blob: (u64, u64),
    pub name_offsets: (u64, u64),
    pub fst_siren: (u64, u64),
    pub fst_terms: (u64, u64),
    pub term_postings: (u64, u64),
    pub trigram_data: (u64, u64),
    pub filter_data: (u64, u64),
    pub dirigeants_data: (u64, u64),
}

impl IndexHeader {
    /// Validates all section offsets and segment lengths against total storage size
    /// and ensures table dimensions match the declared document count.
    pub fn validate(&self, total_size: usize) -> Result<(), IndexError> {
        let n_docs = self.num_docs as u64;

        let check_section = |name: &str, (offset, len): (u64, u64)| -> Result<(), IndexError> {
            let end = offset
                .checked_add(len)
                .ok_or_else(|| IndexError::CorruptedIndex {
                    message: format!(
                        "Header section '{}' offset overflow (offset={}, len={})",
                        name, offset, len
                    ),
                })?;
            if end > total_size as u64 {
                return Err(IndexError::CorruptedIndex {
                    message: format!(
                        "Header section '{}' out of bounds: [{}..{}] exceeds index file size {}",
                        name, offset, end, total_size
                    ),
                });
            }
            Ok(())
        };

        check_section("siren_table", self.siren_table)?;
        check_section("siret_table", self.siret_table)?;
        check_section("metadata_table", self.metadata_table)?;
        check_section("names_blob", self.names_blob)?;
        check_section("name_offsets", self.name_offsets)?;
        check_section("fst_siren", self.fst_siren)?;
        check_section("fst_terms", self.fst_terms)?;
        check_section("term_postings", self.term_postings)?;
        check_section("trigram_data", self.trigram_data)?;
        check_section("filter_data", self.filter_data)?;
        check_section("dirigeants_data", self.dirigeants_data)?;

        // Validate table sizes strictly against num_docs
        if self.siren_table.1 != n_docs * 9 {
            return Err(IndexError::CorruptedIndex {
                message: format!(
                    "Invalid SIREN table size: expected {} bytes for {} docs, found {}",
                    n_docs * 9,
                    n_docs,
                    self.siren_table.1
                ),
            });
        }

        if self.siret_table.1 != n_docs * 15 {
            return Err(IndexError::CorruptedIndex {
                message: format!(
                    "Invalid SIRET table size: expected {} bytes for {} docs, found {}",
                    n_docs * 15,
                    n_docs,
                    self.siret_table.1
                ),
            });
        }

        if self.metadata_table.1 != n_docs * 17 {
            return Err(IndexError::CorruptedIndex {
                message: format!(
                    "Invalid metadata table size: expected {} bytes for {} docs, found {}",
                    n_docs * 17,
                    n_docs,
                    self.metadata_table.1
                ),
            });
        }

        if self.name_offsets.1 != n_docs * 6 {
            return Err(IndexError::CorruptedIndex {
                message: format!(
                    "Invalid name offsets table size: expected {} bytes for {} docs, found {}",
                    n_docs * 6,
                    n_docs,
                    self.name_offsets.1
                ),
            });
        }

        Ok(())
    }

    pub fn write_to<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        w.write_all(INDEX_MAGIC)?;
        w.write_u32::<LittleEndian>(self.version)?;
        w.write_u32::<LittleEndian>(self.num_docs)?;

        let sections = [
            self.siren_table,
            self.siret_table,
            self.metadata_table,
            self.names_blob,
            self.name_offsets,
            self.fst_siren,
            self.fst_terms,
            self.term_postings,
            self.trigram_data,
            self.filter_data,
            self.dirigeants_data,
        ];

        for (offset, len) in sections {
            w.write_u64::<LittleEndian>(offset)?;
            w.write_u64::<LittleEndian>(len)?;
        }

        Ok(())
    }

    pub fn read_from<R: Read>(r: &mut R) -> Result<Self, IndexError> {
        let mut magic = [0u8; 8];
        r.read_exact(&mut magic)?;
        if &magic != INDEX_MAGIC {
            return Err(IndexError::InvalidMagic {
                expected: *INDEX_MAGIC,
                found: magic,
            });
        }

        let version = r.read_u32::<LittleEndian>()?;
        if version != CURRENT_VERSION {
            return Err(IndexError::UnsupportedVersion { version });
        }

        let num_docs = r.read_u32::<LittleEndian>()?;

        let mut read_section = || -> std::io::Result<(u64, u64)> {
            let offset = r.read_u64::<LittleEndian>()?;
            let len = r.read_u64::<LittleEndian>()?;
            Ok((offset, len))
        };

        Ok(Self {
            version,
            num_docs,
            siren_table: read_section()?,
            siret_table: read_section()?,
            metadata_table: read_section()?,
            names_blob: read_section()?,
            name_offsets: read_section()?,
            fst_siren: read_section()?,
            fst_terms: read_section()?,
            term_postings: read_section()?,
            trigram_data: read_section()?,
            filter_data: read_section()?,
            dirigeants_data: read_section()?,
        })
    }
}
