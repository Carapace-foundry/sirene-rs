use smallvec::SmallVec;

/// Computes the optimal string alignment / restricted Damerau-Levenshtein distance.
///
/// Accounts for:
/// - Insertion (cost 1)
/// - Deletion (cost 1)
/// - Substitution (cost 1)
/// - Adjacent transposition (cost 1)
///
/// Uses 3 rolling stack-allocated rows for strings up to length 64 (zero heap allocation).
pub fn damerau_levenshtein(s1: &str, s2: &str) -> usize {
    let b1 = s1.as_bytes();
    let b2 = s2.as_bytes();

    if b1 == b2 {
        return 0;
    }
    if b1.is_empty() {
        return b2.len();
    }
    if b2.is_empty() {
        return b1.len();
    }

    let _len1 = b1.len();
    let len2 = b2.len();

    // Use stack buffers for typical string lengths (<= 64)
    if len2 <= 64 {
        let mut r0 = [0usize; 65];
        let mut r1 = [0usize; 65];
        let mut r2 = [0usize; 65];

        for (j, val) in r1.iter_mut().enumerate().take(len2 + 1) {
            *val = j;
        }

        for (i, &c1) in b1.iter().enumerate() {
            r2[0] = i + 1;

            for (j, &c2) in b2.iter().enumerate() {
                let cost = if c1 == c2 { 0 } else { 1 };
                let mut d = (r1[j + 1] + 1).min(r2[j] + 1).min(r1[j] + cost);

                if i > 0 && j > 0 && c1 == b2[j - 1] && b1[i - 1] == c2 {
                    d = d.min(r0[j - 1] + 1);
                }
                r2[j + 1] = d;
            }

            r0 = r1;
            r1 = r2;
        }

        r1[len2]
    } else {
        // Fallback for long strings
        let mut r0: SmallVec<[usize; 128]> = smallvec::smallvec![0; len2 + 1];
        let mut r1: SmallVec<[usize; 128]> = (0..=len2).collect();
        let mut r2: SmallVec<[usize; 128]> = smallvec::smallvec![0; len2 + 1];

        for (i, &c1) in b1.iter().enumerate() {
            r2[0] = i + 1;

            for (j, &c2) in b2.iter().enumerate() {
                let cost = if c1 == c2 { 0 } else { 1 };
                let mut d = (r1[j + 1] + 1).min(r2[j] + 1).min(r1[j] + cost);

                if i > 0 && j > 0 && c1 == b2[j - 1] && b1[i - 1] == c2 {
                    d = d.min(r0[j - 1] + 1);
                }
                r2[j + 1] = d;
            }

            r0.copy_from_slice(&r1);
            r1.copy_from_slice(&r2);
        }

        r1[len2]
    }
}

/// Computes a normalized similarity score based on Damerau-Levenshtein distance in `[0.0, 1.0]`.
pub fn damerau_levenshtein_similarity(s1: &str, s2: &str) -> f32 {
    let max_len = s1.len().max(s2.len());
    if max_len == 0 {
        return 1.0;
    }

    let dist = damerau_levenshtein(s1, s2);
    (1.0 - (dist as f32 / max_len as f32)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damerau_levenshtein() {
        assert_eq!(damerau_levenshtein("durand", "durand"), 0);
        // Transposition
        assert_eq!(damerau_levenshtein("durand", "druand"), 1);
        // Substitution
        assert_eq!(damerau_levenshtein("durand", "durant"), 1);
        // Deletion
        assert_eq!(damerau_levenshtein("thomas", "tomas"), 1);
        // Empty
        assert_eq!(damerau_levenshtein("", "abc"), 3);
    }

    #[test]
    fn test_damerau_similarity() {
        assert_eq!(damerau_levenshtein_similarity("dupont", "dupont"), 1.0);
        assert_eq!(damerau_levenshtein_similarity("", ""), 1.0);
        let sim = damerau_levenshtein_similarity("durand", "druand");
        assert_eq!(sim, 5.0 / 6.0);
    }
}
