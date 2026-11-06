use smallvec::SmallVec;

/// Packs 3 ASCII bytes into a compact 24-bit `u32` integer.
#[inline]
pub const fn pack_trigram(b0: u8, b1: u8, b2: u8) -> u32 {
    (b0 as u32) | ((b1 as u32) << 8) | ((b2 as u32) << 16)
}

/// Unpacks a 24-bit `u32` trigram back into 3 bytes.
#[inline]
pub const fn unpack_trigram(val: u32) -> [u8; 3] {
    [val as u8, (val >> 8) as u8, (val >> 16) as u8]
}

/// Extracts a sorted, deduplicated set of 3-grams (trigrams) from an input string.
///
/// Uses `SmallVec` on the stack (up to 16 trigrams, covering strings up to 18 characters)
/// to avoid heap allocations for common words.
pub fn trigrams(s: &str) -> SmallVec<[u32; 16]> {
    let bytes = s.as_bytes();
    let mut out: SmallVec<[u32; 16]> = SmallVec::new();

    if bytes.is_empty() {
        return out;
    }

    if bytes.len() < 3 {
        // Pad shorter strings
        let b0 = bytes[0];
        let b1 = if bytes.len() > 1 { bytes[1] } else { b' ' };
        out.push(pack_trigram(b0, b1, b' '));
        return out;
    }

    for window in bytes.windows(3) {
        out.push(pack_trigram(window[0], window[1], window[2]));
    }

    out.sort_unstable();
    out.dedup();
    out
}

/// Computes the Jaccard similarity coefficient between two pre-sorted trigram sets:
///
/// $$J(A, B) = \frac{\vert A \cap B \vert}{\vert A \cup B \vert}$$
///
/// Runs in O(N + M) time via a two-pointer merge with zero memory allocation.
pub fn jaccard_similarity_sets(a: &[u32], b: &[u32]) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let mut i = 0usize;
    let mut j = 0usize;
    let mut intersection = 0usize;
    let mut union = 0usize;

    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            intersection += 1;
            union += 1;
            i += 1;
            j += 1;
        } else if a[i] < b[j] {
            union += 1;
            i += 1;
        } else {
            union += 1;
            j += 1;
        }
    }

    union += (a.len() - i) + (b.len() - j);

    if union == 0 {
        0.0
    } else {
        (intersection as f32 / union as f32).clamp(0.0, 1.0)
    }
}

/// Computes the trigram Jaccard similarity between two strings in `[0.0, 1.0]`.
pub fn jaccard_similarity(s1: &str, s2: &str) -> f32 {
    if s1 == s2 {
        return 1.0;
    }
    if s1.is_empty() || s2.is_empty() {
        return 0.0;
    }

    let t1 = trigrams(s1);
    let t2 = trigrams(s2);
    jaccard_similarity_sets(&t1, &t2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigram_extraction() {
        let tri = trigrams("dupont");
        // "dup", "upo", "pon", "ont" -> 4 unique trigrams
        assert_eq!(tri.len(), 4);
    }

    #[test]
    fn test_jaccard_similarity() {
        assert_eq!(jaccard_similarity("dupont", "dupont"), 1.0);
        assert_eq!(jaccard_similarity("", ""), 1.0);
        assert_eq!(jaccard_similarity("dupont", ""), 0.0);

        // "boucherie dupan" vs "boucherie dupont"
        let j = jaccard_similarity("boucherie dupan", "boucherie dupont");
        assert!(j > 0.65, "Expected Jaccard > 0.65 for typo, got {}", j);
    }
}
