use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use roaring::RoaringBitmap;
use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};

/// Inverted trigram index mapping 24-bit packed trigrams to compressed doc ID postings.
#[derive(Debug, Clone, Default)]
pub struct TrigramIndex {
    /// Mapping: packed trigram (`u32`) -> `RoaringBitmap` of matching document IDs.
    pub postings: BTreeMap<u32, RoaringBitmap>,
    /// Unique trigram count per document ID (for exact Jaccard denominator calculation).
    pub doc_trigram_counts: Vec<u16>,
}

impl TrigramIndex {
    /// Creates an empty `TrigramIndex`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a document and its set of unique packed trigrams.
    pub fn add_document(&mut self, doc_id: u32, trigrams: &[u32]) {
        let id_usize = doc_id as usize;
        if id_usize >= self.doc_trigram_counts.len() {
            self.doc_trigram_counts.resize(id_usize + 1, 0);
        }
        self.doc_trigram_counts[id_usize] = trigrams.len() as u16;

        for &tri in trigrams {
            self.postings.entry(tri).or_default().insert(doc_id);
        }
    }

    /// Pre-selects top candidate document IDs matching query trigrams via minimum Jaccard threshold.
    ///
    /// $$J(Q, D) = \frac{\vert Q \cap D \vert}{\vert Q \vert + \vert D \vert - \vert Q \cap D \vert}$$
    pub fn select_candidates(
        &self,
        query_trigrams: &[u32],
        min_jaccard: f32,
        filter_mask: Option<&RoaringBitmap>,
        max_candidates: usize,
    ) -> Vec<(u32, f32)> {
        if query_trigrams.is_empty() {
            return Vec::new();
        }

        let q_len = query_trigrams.len();
        let mut match_counts: HashMap<u32, usize> = HashMap::new();

        // Accumulate matching trigram frequencies per document
        for &tri in query_trigrams {
            if let Some(posting_bm) = self.postings.get(&tri) {
                if let Some(mask) = filter_mask {
                    let filtered = posting_bm & mask;
                    for doc_id in &filtered {
                        *match_counts.entry(doc_id).or_insert(0) += 1;
                    }
                } else {
                    for doc_id in posting_bm {
                        *match_counts.entry(doc_id).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut candidates: Vec<(u32, f32)> = Vec::with_capacity(match_counts.len());

        for (doc_id, matches) in match_counts {
            let doc_idx = doc_id as usize;
            let d_len = if doc_idx < self.doc_trigram_counts.len() {
                self.doc_trigram_counts[doc_idx] as usize
            } else {
                matches
            };

            let union = q_len + d_len - matches;
            let jaccard = if union > 0 {
                (matches as f32 / union as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };

            if jaccard >= min_jaccard {
                candidates.push((doc_id, jaccard));
            }
        }

        // Sort descending by Jaccard similarity
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if candidates.len() > max_candidates {
            candidates.truncate(max_candidates);
        }

        candidates
    }

    /// Serializes the trigram inverted index into a binary writer.
    pub fn serialize_into<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        // 1. Doc trigram counts
        w.write_u32::<LittleEndian>(self.doc_trigram_counts.len() as u32)?;
        for &count in &self.doc_trigram_counts {
            w.write_u16::<LittleEndian>(count)?;
        }

        // 2. Postings count
        w.write_u32::<LittleEndian>(self.postings.len() as u32)?;
        for (&tri, bm) in &self.postings {
            w.write_u32::<LittleEndian>(tri)?;
            bm.serialize_into(&mut *w)?;
        }

        Ok(())
    }

    /// Deserializes the trigram inverted index from a binary reader.
    pub fn deserialize_from<R: Read>(r: &mut R) -> std::io::Result<Self> {
        let num_counts = r.read_u32::<LittleEndian>()? as usize;
        let mut doc_trigram_counts = Vec::with_capacity(num_counts);
        for _ in 0..num_counts {
            doc_trigram_counts.push(r.read_u16::<LittleEndian>()?);
        }

        let num_postings = r.read_u32::<LittleEndian>()? as usize;
        let mut postings = BTreeMap::new();
        for _ in 0..num_postings {
            let tri = r.read_u32::<LittleEndian>()?;
            let bm = RoaringBitmap::deserialize_from(&mut *r)?;
            postings.insert(tri, bm);
        }

        Ok(Self {
            postings,
            doc_trigram_counts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sirene_matcher::trigrams;

    #[test]
    fn test_trigram_index_and_candidate_selection() {
        let mut idx = TrigramIndex::new();

        // Doc 0: "boucherie dupont"
        let t0 = trigrams("boucherie dupont");
        idx.add_document(0, &t0);

        // Doc 1: "boulangerie martin"
        let t1 = trigrams("boulangerie martin");
        idx.add_document(1, &t1);

        // Doc 2: "boucherie dupan"
        let t2 = trigrams("boucherie dupan");
        idx.add_document(2, &t2);

        // Query: "boucherie dupont"
        let q = trigrams("boucherie dupont");
        let cand = idx.select_candidates(&q, 0.40, None, 10);

        assert!(!cand.is_empty());
        // Doc 0 is exact match (Jaccard = 1.0)
        assert_eq!(cand[0].0, 0);
        assert_eq!(cand[0].1, 1.0);

        // Doc 2 (dupan) is also selected with high Jaccard
        let dupan_cand = cand.iter().find(|(id, _)| *id == 2);
        assert!(dupan_cand.is_some());
        assert!(dupan_cand.unwrap().1 > 0.60);
    }
}
