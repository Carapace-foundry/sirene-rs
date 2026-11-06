use byteorder::{LittleEndian, WriteBytesExt};
use fst::MapBuilder;
use roaring::RoaringBitmap;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use sirene_matcher::trigrams;
use sirene_normalize::Normalizer;

use crate::document::IndexDocument;
use crate::error::Result;
use crate::filters::FilterIndex;
use crate::storage::{IndexHeader, CURRENT_VERSION, HEADER_SIZE};
use crate::trigram_index::TrigramIndex;

/// Builder for constructing compact, zero-copy SIRENE indices.
#[derive(Default)]
pub struct IndexBuilder {
    documents: Vec<IndexDocument>,
    normalizer: Normalizer,
}

impl IndexBuilder {
    /// Creates a new `IndexBuilder`.
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
            normalizer: Normalizer::new(),
        }
    }

    /// Adds an enterprise document to the index.
    pub fn add_document(&mut self, doc: IndexDocument) -> u32 {
        let doc_id = self.documents.len() as u32;
        self.documents.push(doc);
        doc_id
    }

    /// Compiles all accumulated documents into a serialized binary index stream.
    pub fn build(self) -> Result<Vec<u8>> {
        let num_docs = self.documents.len() as u32;
        let mut out = vec![0u8; HEADER_SIZE];

        // 1. SIREN Table (fixed 9 bytes per doc)
        let offset_siren = out.len() as u64;
        for doc in &self.documents {
            out.extend_from_slice(doc.siren.as_bytes());
        }
        let len_siren = out.len() as u64 - offset_siren;

        // 2. SIRET Table (fixed 15 bytes per doc: 1 byte presence flag + 14 bytes)
        let offset_siret = out.len() as u64;
        for doc in &self.documents {
            if let Some(ref siret) = doc.siret {
                out.push(1);
                out.extend_from_slice(siret.as_bytes());
            } else {
                out.push(0);
                out.extend_from_slice(&[0u8; 14]);
            }
        }
        let len_siret = out.len() as u64 - offset_siret;

        // 3. Metadata Table (fixed 17 bytes per doc:
        //    commune [u8; 5], postal [u8; 5], naf [u8; 5], status [u8; 1], has_siret [u8; 1])
        let offset_meta = out.len() as u64;
        for doc in &self.documents {
            let com_bytes = doc
                .code_commune
                .as_ref()
                .map(|c| *c.as_bytes())
                .unwrap_or([b' '; 5]);
            let cp_bytes = doc
                .code_postal
                .as_ref()
                .map(|p| *p.as_bytes())
                .unwrap_or([b' '; 5]);
            let naf_bytes = doc
                .code_naf
                .as_ref()
                .map(|n| *n.as_bytes())
                .unwrap_or([b' '; 5]);
            let status = doc.etat_administratif.as_char() as u8;

            out.extend_from_slice(&com_bytes);
            out.extend_from_slice(&cp_bytes);
            out.extend_from_slice(&naf_bytes);
            out.push(status);
            out.push(if doc.siret.is_some() { 1 } else { 0 });
        }
        let len_meta = out.len() as u64 - offset_meta;

        // 4. Names blob and offsets
        let mut names_blob = Vec::new();
        let mut name_offsets = Vec::with_capacity(num_docs as usize);

        for doc in &self.documents {
            let norm_name = self.normalizer.normalize_for_indexing(&doc.denomination);
            let name_bytes = norm_name.as_bytes();
            let start = names_blob.len() as u32;
            let len = name_bytes.len() as u16;
            names_blob.extend_from_slice(name_bytes);
            name_offsets.push((start, len));
        }

        let offset_names_blob = out.len() as u64;
        out.extend_from_slice(&names_blob);
        let len_names_blob = out.len() as u64 - offset_names_blob;

        let offset_name_offsets = out.len() as u64;
        for (start, len) in name_offsets {
            out.write_u32::<LittleEndian>(start)?;
            out.write_u16::<LittleEndian>(len)?;
        }
        let len_name_offsets = out.len() as u64 - offset_name_offsets;

        // 5. FST SIREN Index
        let offset_fst_siren = out.len() as u64;
        {
            let mut siren_pairs: Vec<([u8; 9], u64)> = self
                .documents
                .iter()
                .enumerate()
                .map(|(i, d)| (*d.siren.as_bytes(), i as u64))
                .collect();
            siren_pairs.sort_by_key(|a| a.0);
            siren_pairs.dedup_by(|a, b| a.0 == b.0);

            let mut fst_builder = MapBuilder::new(&mut out)?;
            for (siren_bytes, doc_id) in siren_pairs {
                fst_builder.insert(siren_bytes, doc_id)?;
            }
            fst_builder.finish()?;
        }
        let len_fst_siren = out.len() as u64 - offset_fst_siren;

        // 6. FST Terms and Postings
        let mut term_postings: BTreeMap<String, RoaringBitmap> = BTreeMap::new();
        for (doc_id, doc) in self.documents.iter().enumerate() {
            let q = self.normalizer.normalize_query(&doc.denomination);
            for tok in q.tokens {
                term_postings.entry(tok).or_default().insert(doc_id as u32);
            }
        }

        let offset_fst_terms = out.len() as u64;
        {
            let mut fst_builder = MapBuilder::new(&mut out)?;
            for (term_idx, (term, _)) in term_postings.iter().enumerate() {
                fst_builder.insert(term.as_bytes(), term_idx as u64)?;
            }
            fst_builder.finish()?;
        }
        let len_fst_terms = out.len() as u64 - offset_fst_terms;

        let offset_term_postings = out.len() as u64;
        {
            out.write_u32::<LittleEndian>(term_postings.len() as u32)?;
            for bm in term_postings.values() {
                bm.serialize_into(&mut out)?;
            }
        }
        let len_term_postings = out.len() as u64 - offset_term_postings;

        // 7. Trigram Inverted Index
        let mut trigram_index = TrigramIndex::new();
        for (doc_id, doc) in self.documents.iter().enumerate() {
            let norm_name = self.normalizer.normalize_for_indexing(&doc.denomination);
            let tri = trigrams(&norm_name);
            trigram_index.add_document(doc_id as u32, &tri);
        }

        let offset_trigrams = out.len() as u64;
        trigram_index.serialize_into(&mut out)?;
        let len_trigrams = out.len() as u64 - offset_trigrams;

        // 8. Filters (Active & Department Bitsets)
        let mut filters = FilterIndex::new();
        for (doc_id, doc) in self.documents.iter().enumerate() {
            let is_active = doc.etat_administratif.is_actif();
            let c_dept = doc.code_commune.as_ref().map(|c| c.departement());
            let p_dept = doc.code_postal.as_ref().map(|p| p.departement());

            filters.add_document(doc_id as u32, is_active, c_dept);
            if let Some(pd) = p_dept {
                if Some(pd) != c_dept {
                    filters.add_department(doc_id as u32, pd);
                }
            }
        }

        let offset_filters = out.len() as u64;
        filters.serialize_into(&mut out)?;
        let len_filters = out.len() as u64 - offset_filters;

        // 9. Dirigeants Data
        let offset_dirigeants = out.len() as u64;
        {
            let mut dir_blob = Vec::new();
            let mut dir_offsets = Vec::with_capacity(self.documents.len());
            for doc in &self.documents {
                let cur_offset = dir_blob.len() as u32;
                dir_offsets.push(cur_offset);
                dir_blob.write_u8(doc.dirigeants.len().min(255) as u8)?;
                for dir in &doc.dirigeants {
                    let dir_norm = self.normalizer.normalize_for_indexing(dir);
                    let bytes = dir_norm.as_bytes();
                    let len = bytes.len().min(255) as u8;
                    dir_blob.write_u8(len)?;
                    dir_blob.extend_from_slice(&bytes[..len as usize]);
                }
            }
            for off in dir_offsets {
                out.write_u32::<LittleEndian>(off)?;
            }
            out.extend_from_slice(&dir_blob);
        }
        let len_dirigeants = out.len() as u64 - offset_dirigeants;

        // 10. Write final header into the beginning of the buffer
        let header = IndexHeader {
            version: CURRENT_VERSION,
            num_docs,
            siren_table: (offset_siren, len_siren),
            siret_table: (offset_siret, len_siret),
            metadata_table: (offset_meta, len_meta),
            names_blob: (offset_names_blob, len_names_blob),
            name_offsets: (offset_name_offsets, len_name_offsets),
            fst_siren: (offset_fst_siren, len_fst_siren),
            fst_terms: (offset_fst_terms, len_fst_terms),
            term_postings: (offset_term_postings, len_term_postings),
            trigram_data: (offset_trigrams, len_trigrams),
            filter_data: (offset_filters, len_filters),
            dirigeants_data: (offset_dirigeants, len_dirigeants),
        };

        let mut header_buf = Vec::with_capacity(HEADER_SIZE);
        header.write_to(&mut header_buf)?;
        out[0..HEADER_SIZE].copy_from_slice(&header_buf);

        Ok(out)
    }

    /// Writes the built index directly to a file.
    pub fn write_to_file<P: AsRef<Path>>(self, path: P) -> Result<()> {
        let bytes = self.build()?;
        let mut file = File::create(path)?;
        file.write_all(&bytes)?;
        Ok(())
    }
}
