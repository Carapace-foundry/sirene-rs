use byteorder::{ByteOrder, LittleEndian};
use fst::Map;
use roaring::RoaringBitmap;
use smallvec::SmallVec;
use std::collections::HashSet;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::sync::Arc;

use sirene_core::{
    CodeCommune, CodeNaf, CodePostal, EtatAdministratif, IdentifiantPivot, Siren, Siret,
};
use sirene_matcher::{trigrams, EntityMatcher, MatchCandidate, SearchQuery};
use sirene_normalize::Normalizer;

use crate::document::{IndexedDocumentView, SearchResult};
use crate::error::{IndexError, Result};
use crate::filters::FilterIndex;
use crate::storage::{IndexHeader, IndexStorage, StorageSlice};
use crate::trigram_index::TrigramIndex;

/// Zero-copy SIRENE search engine and index reader.
pub struct SireneIndex {
    storage: Arc<IndexStorage>,
    header: IndexHeader,
    fst_siren: Map<StorageSlice>,
    fst_terms: Map<StorageSlice>,
    term_postings: Vec<RoaringBitmap>,
    trigram_index: TrigramIndex,
    filters: FilterIndex,
    normalizer: Normalizer,
    matcher: EntityMatcher,
}

impl SireneIndex {
    /// Loads an index from an in-memory byte buffer (suitable for WASM allocations).
    pub fn from_vec(vec: Vec<u8>) -> Result<Self> {
        Self::init(IndexStorage::Owned(vec))
    }

    /// Loads an index from a static byte slice.
    pub fn from_static_bytes(bytes: &'static [u8]) -> Result<Self> {
        Self::init(IndexStorage::Static(bytes))
    }

    /// Memory-maps an index file from disk (native targets).
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(unsafe_code)]
    pub fn open_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        // SAFETY:
        // The file descriptor is opened read-only and remains valid for the lifetime
        // of the mapping. All segment offsets, table sizes, and string slices are validated
        // against the mapped memory bounds in `IndexHeader::validate` and `StorageSlice::new`
        // before any slice or struct is decoded.
        let mmap = unsafe { memmap2::Mmap::map(&file)? };
        Self::init(IndexStorage::Mmap(mmap))
    }

    /// Internal initializer unpacking segments and zero-copy FST structures with strict bounds verification.
    fn init(storage: IndexStorage) -> Result<Self> {
        let storage = Arc::new(storage);
        let bytes = storage.as_bytes();
        let header = IndexHeader::read_from(&mut &bytes[..])?;
        header.validate(bytes.len())?;

        // Extract safe bounds-checked storage slices for FSTs (no lifetime forgery or unsafe)
        let fst_siren_slice = StorageSlice::new(
            Arc::clone(&storage),
            header.fst_siren.0 as usize,
            header.fst_siren.1 as usize,
        )?;
        let fst_siren = Map::new(fst_siren_slice)?;

        let fst_terms_slice = StorageSlice::new(
            Arc::clone(&storage),
            header.fst_terms.0 as usize,
            header.fst_terms.1 as usize,
        )?;
        let fst_terms = Map::new(fst_terms_slice)?;

        // Deserializing term postings
        let start_tp = header.term_postings.0 as usize;
        let len_tp = header.term_postings.1 as usize;
        let mut tp_cursor = &bytes[start_tp..start_tp + len_tp];
        if tp_cursor.len() < 4 {
            return Err(IndexError::CorruptedIndex {
                message: "Truncated term postings segment".to_string(),
            });
        }
        let num_terms = LittleEndian::read_u32(tp_cursor) as usize;
        tp_cursor = &tp_cursor[4..];
        let mut term_postings = Vec::with_capacity(num_terms);
        for _ in 0..num_terms {
            term_postings.push(RoaringBitmap::deserialize_from(&mut tp_cursor)?);
        }

        // Deserializing trigram index
        let start_tri = header.trigram_data.0 as usize;
        let len_tri = header.trigram_data.1 as usize;
        let mut tri_cursor = &bytes[start_tri..start_tri + len_tri];
        let trigram_index = TrigramIndex::deserialize_from(&mut tri_cursor)?;

        // Deserializing filter index
        let start_filt = header.filter_data.0 as usize;
        let len_filt = header.filter_data.1 as usize;
        let mut filt_cursor = &bytes[start_filt..start_filt + len_filt];
        let filters = FilterIndex::deserialize_from(&mut filt_cursor)?;

        Ok(Self {
            storage,
            header,
            fst_siren,
            fst_terms,
            term_postings,
            trigram_index,
            filters,
            normalizer: Normalizer::new(),
            matcher: EntityMatcher::new(),
        })
    }

    /// Returns the total number of indexed documents.
    #[inline]
    pub fn num_documents(&self) -> u32 {
        self.header.num_docs
    }

    /// Returns a reference to the index header.
    #[inline]
    pub fn header(&self) -> &IndexHeader {
        &self.header
    }

    /// Returns a reference to the filter index.
    #[inline]
    pub fn filters(&self) -> &FilterIndex {
        &self.filters
    }

    /// Returns a reference to the trigram index.
    #[inline]
    pub fn trigram_index(&self) -> &TrigramIndex {
        &self.trigram_index
    }

    /// Retrieves an instantaneous exact match by 9-digit SIREN via the FST index.
    pub fn find_by_siren(&self, siren: Siren) -> Option<u32> {
        self.fst_siren
            .get(siren.as_str().as_bytes())
            .map(|id| id as u32)
    }

    /// Retrieves an exact match by 14-digit SIRET.
    pub fn find_by_siret(&self, siret: Siret) -> Option<u32> {
        let siren = siret.siren();
        let doc_id = self.find_by_siren(siren)?;
        let view = self.get_document(doc_id)?;
        if view.siret == Some(siret) || view.siren == siren {
            Some(doc_id)
        } else {
            None
        }
    }

    /// Decodes a zero-copy document view for a given document ID.
    pub fn get_document(&self, doc_id: u32) -> Option<IndexedDocumentView<'_>> {
        if doc_id >= self.header.num_docs {
            return None;
        }

        let bytes = self.storage.as_bytes();
        let idx = doc_id as usize;

        // 1. SIREN (9 bytes)
        let s_offset = self.header.siren_table.0 as usize + idx * 9;
        let mut s_bytes = [0u8; 9];
        s_bytes.copy_from_slice(&bytes[s_offset..s_offset + 9]);
        let siren = Siren::new_unchecked(s_bytes);

        // 2. SIRET (15 bytes: flag + 14 digits)
        let srt_offset = self.header.siret_table.0 as usize + idx * 15;
        let has_siret = bytes[srt_offset] == 1;
        let siret = if has_siret {
            let mut srt_bytes = [0u8; 14];
            srt_bytes.copy_from_slice(&bytes[srt_offset + 1..srt_offset + 15]);
            Some(Siret::new_unchecked(srt_bytes))
        } else {
            None
        };

        // 3. Metadata (17 bytes: commune 5, postal 5, naf 5, status 1, has_siret 1)
        let m_offset = self.header.metadata_table.0 as usize + idx * 17;
        let com_raw = &bytes[m_offset..m_offset + 5];
        let code_commune = if com_raw[0] != b' ' {
            let mut arr = [0u8; 5];
            arr.copy_from_slice(com_raw);
            Some(CodeCommune::new_unchecked(arr))
        } else {
            None
        };

        let cp_raw = &bytes[m_offset + 5..m_offset + 10];
        let code_postal = if cp_raw[0] != b' ' {
            let mut arr = [0u8; 5];
            arr.copy_from_slice(cp_raw);
            Some(CodePostal::new_unchecked(arr))
        } else {
            None
        };

        let naf_raw = &bytes[m_offset + 10..m_offset + 15];
        let code_naf = if naf_raw[0] != b' ' {
            let mut arr = [0u8; 5];
            arr.copy_from_slice(naf_raw);
            Some(CodeNaf::new_unchecked(arr))
        } else {
            None
        };

        let status_byte = bytes[m_offset + 15];
        let etat_administratif = if status_byte == b'C' {
            EtatAdministratif::Cesse
        } else {
            EtatAdministratif::Actif
        };

        // 4. Denomination
        let no_offset = self.header.name_offsets.0 as usize + idx * 6;
        let name_start = LittleEndian::read_u32(&bytes[no_offset..no_offset + 4]) as usize;
        let name_len = LittleEndian::read_u16(&bytes[no_offset + 4..no_offset + 6]) as usize;

        let names_blob_len = self.header.names_blob.1 as usize;
        if !name_start
            .checked_add(name_len)
            .is_some_and(|end| end <= names_blob_len)
        {
            return None;
        }

        let nb_offset = self.header.names_blob.0 as usize + name_start;
        let denom_bytes = &bytes[nb_offset..nb_offset + name_len];
        let denomination = std::str::from_utf8(denom_bytes).unwrap_or_default();

        // 5. Dirigeants
        let d_start = self.header.dirigeants_data.0 as usize;
        let d_len = self.header.dirigeants_data.1 as usize;
        let d_end = d_start + d_len;
        let mut dirigeants = Vec::new();
        let min_table_len = self.header.num_docs as usize * 4;
        if d_len >= min_table_len {
            let off_pos = d_start + idx * 4;
            if off_pos + 4 <= d_end {
                let rel_offset = LittleEndian::read_u32(&bytes[off_pos..off_pos + 4]) as usize;
                let blob_start = d_start + min_table_len;
                let mut ptr = blob_start.saturating_add(rel_offset);
                if ptr < d_end {
                    let count = bytes[ptr] as usize;
                    ptr += 1;
                    for _ in 0..count {
                        if ptr >= d_end {
                            break;
                        }
                        let len = bytes[ptr] as usize;
                        ptr += 1;
                        if ptr.checked_add(len).is_some_and(|end| end <= d_end) {
                            if let Ok(s) = std::str::from_utf8(&bytes[ptr..ptr + len]) {
                                dirigeants.push(s);
                            }
                            ptr += len;
                        }
                    }
                }
            }
        }

        Some(IndexedDocumentView {
            doc_id,
            siren,
            siret,
            denomination,
            code_commune,
            code_postal,
            code_naf,
            etat_administratif,
            dirigeants,
        })
    }

    /// Performs a full end-to-end search:
    /// 1. Pivot short-circuit (exact SIREN/SIRET).
    /// 2. Roaring Bitmap strict pre-filtering (department / active state).
    /// 3. Candidate pre-selection via FST term intersection and Trigram Jaccard.
    /// 4. Fine multi-criteria ranking via `sirene-matcher`.
    pub fn search(&self, search: &SearchQuery, top_k: usize) -> Vec<SearchResult> {
        // Step 1: Immediate Pivot Short-Circuit (Section 4.1)
        if let Some(pivot) = search.query.pivot {
            let direct_doc_id = match pivot {
                IdentifiantPivot::Siren(s) => self.find_by_siren(s),
                IdentifiantPivot::Siret(s) => self.find_by_siret(s),
            };

            if let Some(doc_id) = direct_doc_id {
                if let Some(doc_view) = self.get_document(doc_id) {
                    let token_vec: SmallVec<[&str; 8]> = doc_view
                        .denomination
                        .split(' ')
                        .filter(|s| !s.is_empty())
                        .collect();

                    let cand = MatchCandidate {
                        siren: doc_view.siren,
                        siret: doc_view.siret,
                        denomination: doc_view.denomination,
                        tokens: &token_vec,
                        code_commune: doc_view.code_commune,
                        code_postal: doc_view.code_postal,
                        code_naf: doc_view.code_naf,
                        etat_administratif: doc_view.etat_administratif,
                        dirigeants: &doc_view.dirigeants,
                    };

                    let scored = self.matcher.score_candidate(search, &cand);
                    let result_siret = match pivot {
                        IdentifiantPivot::Siret(s) => Some(s),
                        IdentifiantPivot::Siren(_) => doc_view.siret,
                    };
                    return vec![SearchResult {
                        doc_id,
                        siren: doc_view.siren,
                        siret: result_siret,
                        denomination: doc_view.denomination.to_string(),
                        score: 1.0,
                        sub_scores: scored.sub_scores,
                        is_exact_pivot: true,
                    }];
                }
            }
        }

        // Step 2: Bitset Pre-Filtering
        let dept_str = if search.strict_location {
            search.location.as_ref().map(|loc| match loc {
                sirene_matcher::LocationQuery::Departement(d) => d.as_str(),
                sirene_matcher::LocationQuery::Postal(p) => p.departement(),
                sirene_matcher::LocationQuery::Commune(c) => c.departement(),
            })
        } else {
            None
        };

        let filter_mask = self.filters.filter(false, dept_str, None);

        // Step 3: Candidate Pre-Selection
        let mut candidate_ids: HashSet<u32> = HashSet::new();

        // 3a. Exact term matching via FST with multi-token AND intersection priority
        let mut term_bms = Vec::new();
        for tok in &search.query.tokens {
            if let Some(term_idx) = self.fst_terms.get(tok.as_bytes()) {
                if let Some(bm) = self.term_postings.get(term_idx as usize) {
                    term_bms.push(bm);
                }
            } else if let Some(expanded) = sirene_normalize::expand_abbreviation(tok) {
                for part in expanded.split(' ') {
                    if let Some(term_idx) = self.fst_terms.get(part.as_bytes()) {
                        if let Some(bm) = self.term_postings.get(term_idx as usize) {
                            term_bms.push(bm);
                        }
                    }
                }
            }
        }

        // Priority 1: Documents matching ALL search tokens (AND intersection)
        if term_bms.len() >= 2 {
            let mut and_bm = term_bms[0].clone();
            for bm in &term_bms[1..] {
                and_bm &= *bm;
            }
            if let Some(ref mask) = filter_mask {
                and_bm &= mask;
            }
            for id in and_bm.iter().take(200) {
                candidate_ids.insert(id);
            }
        }

        // Priority 2: Term postings union (for single token or to supplement multi-token)
        if candidate_ids.len() < 100 {
            for bm in &term_bms {
                let matching_bm = match filter_mask {
                    Some(ref mask) => *bm & mask,
                    None => (*bm).clone(),
                };
                for id in matching_bm.iter().take(100) {
                    candidate_ids.insert(id);
                    if candidate_ids.len() >= 250 {
                        break;
                    }
                }
            }
        }

        // 3b. Trigram Jaccard matching
        let clean_name = self.normalizer.normalize_for_indexing(&search.query.raw);
        let q_trigrams = trigrams(&clean_name);
        let trigram_candidates =
            self.trigram_index
                .select_candidates(&q_trigrams, 0.20, filter_mask.as_ref(), 150);

        for (id, _) in trigram_candidates {
            candidate_ids.insert(id);
        }

        // Step 4: Multi-criteria Scoring & Ranking
        let mut scored_results: Vec<SearchResult> = Vec::with_capacity(candidate_ids.len());

        for doc_id in candidate_ids {
            if let Some(doc_view) = self.get_document(doc_id) {
                let token_vec: SmallVec<[&str; 8]> = doc_view
                    .denomination
                    .split(' ')
                    .filter(|s| !s.is_empty())
                    .collect();

                let cand = MatchCandidate {
                    siren: doc_view.siren,
                    siret: doc_view.siret,
                    denomination: doc_view.denomination,
                    tokens: &token_vec,
                    code_commune: doc_view.code_commune,
                    code_postal: doc_view.code_postal,
                    code_naf: doc_view.code_naf,
                    etat_administratif: doc_view.etat_administratif,
                    dirigeants: &doc_view.dirigeants,
                };

                let scored = self.matcher.score_candidate(search, &cand);
                scored_results.push(SearchResult {
                    doc_id,
                    siren: doc_view.siren,
                    siret: doc_view.siret,
                    denomination: doc_view.denomination.to_string(),
                    score: scored.score(),
                    sub_scores: scored.sub_scores,
                    is_exact_pivot: scored.is_exact_pivot,
                });
            }
        }

        // Sort descending by score
        scored_results.sort_by(|a, b| {
            b.is_exact_pivot.cmp(&a.is_exact_pivot).then_with(|| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        if scored_results.len() > top_k {
            scored_results.truncate(top_k);
        }

        scored_results
    }
}
