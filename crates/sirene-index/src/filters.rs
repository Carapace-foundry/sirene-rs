use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use roaring::RoaringBitmap;
use std::collections::BTreeMap;
use std::io::{Read, Write};

/// Strict pre-filtering index based on compressed Roaring Bitmaps.
///
/// Executes SIMD binary intersections (bitset AND) for active state and department
/// before running any fuzzy matching.
#[derive(Debug, Clone, Default)]
pub struct FilterIndex {
    /// Bitset of all active enterprises (etatAdministratif = 'A').
    pub active_bitmap: RoaringBitmap,
    /// Bitset per department code (e.g. `"75"`, `"33"`, `"2A"`, `"971"`).
    pub dept_bitmaps: BTreeMap<String, RoaringBitmap>,
}

impl FilterIndex {
    /// Creates an empty `FilterIndex`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records document attributes for filter bitsets.
    pub fn add_document(&mut self, doc_id: u32, is_active: bool, dept: Option<&str>) {
        if is_active {
            self.active_bitmap.insert(doc_id);
        }
        if let Some(d) = dept {
            self.add_department(doc_id, d);
        }
    }

    /// Associates an additional department with a document (e.g. secondary postal or territorial routing).
    pub fn add_department(&mut self, doc_id: u32, dept: &str) {
        let key = dept.trim().to_ascii_uppercase();
        if !key.is_empty() {
            self.dept_bitmaps.entry(key).or_default().insert(doc_id);
        }
    }

    /// Evaluates requested strict filters into an intersected candidate mask.
    ///
    /// Returns `None` if no strict filters are active (meaning all documents are eligible).
    pub fn filter(
        &self,
        active_only: bool,
        department: Option<&str>,
        base_set: Option<&RoaringBitmap>,
    ) -> Option<RoaringBitmap> {
        let mut result: Option<RoaringBitmap> = base_set.cloned();

        if active_only {
            match result {
                Some(ref mut bm) => *bm &= &self.active_bitmap,
                None => result = Some(self.active_bitmap.clone()),
            }
        }

        if let Some(dept_str) = department {
            let dept_upper = dept_str.trim().to_ascii_uppercase();
            let dept_bm: Option<RoaringBitmap> = match dept_upper.as_str() {
                "20" => {
                    let mut union = RoaringBitmap::new();
                    if let Some(bm) = self.dept_bitmaps.get("20") {
                        union |= bm;
                    }
                    if let Some(bm) = self.dept_bitmaps.get("2A") {
                        union |= bm;
                    }
                    if let Some(bm) = self.dept_bitmaps.get("2B") {
                        union |= bm;
                    }
                    if union.is_empty() {
                        None
                    } else {
                        Some(union)
                    }
                }
                "2A" => {
                    let mut union = RoaringBitmap::new();
                    if let Some(bm) = self.dept_bitmaps.get("2A") {
                        union |= bm;
                    }
                    if let Some(bm) = self.dept_bitmaps.get("20") {
                        union |= bm;
                    }
                    if union.is_empty() {
                        None
                    } else {
                        Some(union)
                    }
                }
                "2B" => {
                    let mut union = RoaringBitmap::new();
                    if let Some(bm) = self.dept_bitmaps.get("2B") {
                        union |= bm;
                    }
                    if let Some(bm) = self.dept_bitmaps.get("20") {
                        union |= bm;
                    }
                    if union.is_empty() {
                        None
                    } else {
                        Some(union)
                    }
                }
                _ => self.dept_bitmaps.get(&dept_upper).cloned(),
            };

            match dept_bm {
                Some(bm) => match result {
                    Some(ref mut current) => *current &= &bm,
                    None => result = Some(bm),
                },
                None => {
                    // Department does not exist in index: zero candidates match
                    return Some(RoaringBitmap::new());
                }
            }
        }

        result
    }

    /// Serializes the filter bitmaps into a writer.
    pub fn serialize_into<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        // 1. Active bitmap
        self.active_bitmap.serialize_into(&mut *w)?;

        // 2. Dept bitmaps count
        w.write_u32::<LittleEndian>(self.dept_bitmaps.len() as u32)?;
        for (dept, bm) in &self.dept_bitmaps {
            let dept_bytes = dept.as_bytes();
            w.write_u8(dept_bytes.len() as u8)?;
            w.write_all(dept_bytes)?;
            bm.serialize_into(&mut *w)?;
        }

        Ok(())
    }

    /// Deserializes filter bitmaps from a reader.
    pub fn deserialize_from<R: Read>(r: &mut R) -> std::io::Result<Self> {
        let active_bitmap = RoaringBitmap::deserialize_from(&mut *r)?;

        let num_depts = r.read_u32::<LittleEndian>()?;
        let mut dept_bitmaps = BTreeMap::new();

        for _ in 0..num_depts {
            let len = r.read_u8()? as usize;
            let mut dept_bytes = vec![0u8; len];
            r.read_exact(&mut dept_bytes)?;
            let dept_str = String::from_utf8(dept_bytes).unwrap_or_default();
            let bm = RoaringBitmap::deserialize_from(&mut *r)?;
            dept_bitmaps.insert(dept_str, bm);
        }

        Ok(Self {
            active_bitmap,
            dept_bitmaps,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_index_operations() {
        let mut filters = FilterIndex::new();
        // Doc 0: Paris, active
        filters.add_document(0, true, Some("75"));
        // Doc 1: Paris, ceased
        filters.add_document(1, false, Some("75"));
        // Doc 2: Bordeaux, active
        filters.add_document(2, true, Some("33"));
        // Doc 3: Bordeaux, ceased
        filters.add_document(3, false, Some("33"));

        // Filter active only -> docs 0, 2
        let active = filters.filter(true, None, None).expect("active filter");
        assert_eq!(active.iter().collect::<Vec<_>>(), vec![0, 2]);

        // Filter dept 33 only -> docs 2, 3
        let bordeaux = filters
            .filter(false, Some("33"), None)
            .expect("bordeaux filter");
        assert_eq!(bordeaux.iter().collect::<Vec<_>>(), vec![2, 3]);

        // Filter active AND dept 33 -> doc 2 only
        let bordeaux_active = filters
            .filter(true, Some("33"), None)
            .expect("bordeaux active");
        assert_eq!(bordeaux_active.iter().collect::<Vec<_>>(), vec![2]);

        // Filter unknown dept -> empty
        let unknown = filters.filter(false, Some("99"), None).expect("unknown");
        assert!(unknown.is_empty());
    }

    #[test]
    fn test_filter_serialization() {
        let mut filters = FilterIndex::new();
        filters.add_document(10, true, Some("75"));
        filters.add_document(20, false, Some("33"));

        let mut buf = Vec::new();
        filters.serialize_into(&mut buf).expect("serialize");

        let loaded = FilterIndex::deserialize_from(&mut &buf[..]).expect("deserialize");
        assert_eq!(filters.active_bitmap, loaded.active_bitmap);
        assert_eq!(filters.dept_bitmaps.len(), loaded.dept_bitmaps.len());
    }
}
