use smallvec::SmallVec;

/// Standard scaling factor for Winkler prefix bonus.
pub const WINKLER_PREFIX_SCALE: f32 = 0.1;

/// Maximum common prefix length rewarded by Winkler.
pub const MAX_WINKLER_PREFIX_LEN: usize = 4;

/// Computes the Jaro similarity between two strings, returning a value in `[0.0, 1.0]`.
///
/// Uses stack-allocated bitmasks for strings of length up to 64 to guarantee zero heap allocation.
pub fn jaro_similarity(s1: &str, s2: &str) -> f32 {
    let b1 = s1.as_bytes();
    let b2 = s2.as_bytes();

    if b1 == b2 {
        return 1.0;
    }
    if b1.is_empty() || b2.is_empty() {
        return 0.0;
    }

    let len1 = b1.len();
    let len2 = b2.len();

    let match_distance = (len1.max(len2) / 2).saturating_sub(1);

    // If both strings fit in 64 bytes, use stack bitmasks
    if len1 <= 64 && len2 <= 64 {
        let mut s1_mask: u64 = 0;
        let mut s2_mask: u64 = 0;
        let mut matches = 0usize;

        for (i, &c1) in b1.iter().enumerate() {
            let start = i.saturating_sub(match_distance);
            let end = (i + match_distance + 1).min(len2);

            for (j, &c2) in b2.iter().enumerate().take(end).skip(start) {
                if (s2_mask & (1u64 << j)) == 0 && c1 == c2 {
                    s1_mask |= 1u64 << i;
                    s2_mask |= 1u64 << j;
                    matches += 1;
                    break;
                }
            }
        }

        if matches == 0 {
            return 0.0;
        }

        // Count half-transpositions
        let mut transpositions = 0usize;
        let mut k = 0usize;

        for (i, &c1) in b1.iter().enumerate() {
            if (s1_mask & (1u64 << i)) != 0 {
                while (s2_mask & (1u64 << k)) == 0 {
                    k += 1;
                }
                if c1 != b2[k] {
                    transpositions += 1;
                }
                k += 1;
            }
        }

        let m = matches as f32;
        let t = (transpositions / 2) as f32;
        (m / (len1 as f32) + m / (len2 as f32) + (m - t) / m) / 3.0
    } else {
        // Fallback with SmallVec for longer strings
        let mut s1_matched: SmallVec<[bool; 128]> = smallvec::smallvec![false; len1];
        let mut s2_matched: SmallVec<[bool; 128]> = smallvec::smallvec![false; len2];
        let mut matches = 0usize;

        for (i, &c1) in b1.iter().enumerate() {
            let start = i.saturating_sub(match_distance);
            let end = (i + match_distance + 1).min(len2);

            for (j, &c2) in b2.iter().enumerate().take(end).skip(start) {
                if !s2_matched[j] && c1 == c2 {
                    s1_matched[i] = true;
                    s2_matched[j] = true;
                    matches += 1;
                    break;
                }
            }
        }

        if matches == 0 {
            return 0.0;
        }

        let mut transpositions = 0usize;
        let mut k = 0usize;

        for (i, &c1) in b1.iter().enumerate() {
            if s1_matched[i] {
                while !s2_matched[k] {
                    k += 1;
                }
                if c1 != b2[k] {
                    transpositions += 1;
                }
                k += 1;
            }
        }

        let m = matches as f32;
        let t = (transpositions / 2) as f32;
        (m / (len1 as f32) + m / (len2 as f32) + (m - t) / m) / 3.0
    }
}

/// Computes the Jaro-Winkler similarity between two strings, returning a value in `[0.0, 1.0]`.
///
/// Gives increased weight to matching prefixes up to 4 characters.
pub fn jaro_winkler_similarity(s1: &str, s2: &str) -> f32 {
    let jaro = jaro_similarity(s1, s2);
    if jaro <= 0.0 || jaro >= 1.0 {
        return jaro;
    }

    // Determine common prefix length up to MAX_WINKLER_PREFIX_LEN
    let prefix_len = s1
        .bytes()
        .zip(s2.bytes())
        .take(MAX_WINKLER_PREFIX_LEN)
        .take_while(|(c1, c2)| c1 == c2)
        .count();

    (jaro + (prefix_len as f32 * WINKLER_PREFIX_SCALE * (1.0 - jaro))).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_and_empty() {
        assert_eq!(jaro_winkler_similarity("", ""), 1.0);
        assert_eq!(jaro_winkler_similarity("dupont", "dupont"), 1.0);
        assert_eq!(jaro_winkler_similarity("dupont", ""), 0.0);
        assert_eq!(jaro_winkler_similarity("", "dupont"), 0.0);
    }

    #[test]
    fn test_standard_reference_cases() {
        // "MARTHA" vs "MARHTA"
        let jaro = jaro_similarity("MARTHA", "MARHTA");
        assert!((jaro - 0.944).abs() < 0.005);
        let jw = jaro_winkler_similarity("MARTHA", "MARHTA");
        assert!((jw - 0.961).abs() < 0.005);

        // "DWAYNE" vs "DUANE"
        let jaro_dwayne = jaro_similarity("DWAYNE", "DUANE");
        assert!((jaro_dwayne - 0.822).abs() < 0.005);
        let jw_dwayne = jaro_winkler_similarity("DWAYNE", "DUANE");
        assert!((jw_dwayne - 0.840).abs() < 0.005);
    }

    #[test]
    fn test_boucherie_dupan_dupont() {
        let sim = jaro_winkler_similarity("boucherie dupan", "boucherie dupont");
        assert!(sim > 0.92, "Expected high similarity for typo, got {}", sim);
    }
}
