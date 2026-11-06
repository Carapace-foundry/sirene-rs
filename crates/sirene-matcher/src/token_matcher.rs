use smallvec::SmallVec;

use crate::jaro_winkler::jaro_winkler_similarity;

/// Checks if an acronym token matches the initials of a sequence of words.
///
/// Example: `"sncf"` matches initials of `["societe", "nationale", "chemins", "fer"]`.
pub fn is_acronym_of(acronym: &str, words: &[&str]) -> bool {
    let acr_bytes = acronym.as_bytes();
    if acr_bytes.is_empty() || words.is_empty() {
        return false;
    }

    // Check full initials match
    if acr_bytes.len() == words.len() {
        let matches = acr_bytes
            .iter()
            .zip(words.iter())
            .all(|(&b, w)| !w.is_empty() && w.as_bytes()[0] == b);
        if matches {
            return true;
        }
    }

    // Check substring match if words has more tokens
    if words.len() >= acr_bytes.len() {
        for window in words.windows(acr_bytes.len()) {
            let matches = acr_bytes
                .iter()
                .zip(window.iter())
                .all(|(&b, w)| !w.is_empty() && w.as_bytes()[0] == b);
            if matches {
                return true;
            }
        }
    }

    false
}

/// Returns true if a byte represents an acronym connector letter corresponding
/// to omitted French stop words:
/// - `'d'`: de, du, des, d'
/// - `'e'`: et, en
/// - `'a'`: a, au, aux
/// - `'l'`: le, la, les, l'
#[inline]
pub fn is_acronym_connector_byte(b: u8) -> bool {
    matches!(b, b'd' | b'e' | b'a' | b'l')
}

/// Matches an acronym byte sequence against a sequence of words, tolerating
/// acronym connector letters that represent omitted French stop-words (e.g. "edf" vs ["electricite", "france"]).
fn matches_acronym_sequence(token_bytes: &[u8], window: &[&str]) -> bool {
    let mut t_idx = 0;
    let mut w_idx = 0;

    while t_idx < token_bytes.len() && w_idx < window.len() {
        let w = window[w_idx];
        if w.is_empty() {
            w_idx += 1;
            continue;
        }

        let first_byte = w.as_bytes()[0];
        if token_bytes[t_idx] == first_byte {
            t_idx += 1;
            w_idx += 1;
        } else if is_acronym_connector_byte(token_bytes[t_idx]) {
            t_idx += 1;
        } else {
            return false;
        }
    }

    while t_idx < token_bytes.len() && is_acronym_connector_byte(token_bytes[t_idx]) {
        t_idx += 1;
    }

    t_idx == token_bytes.len() && w_idx == window.len()
}

/// Checks if `token` is an acronym matching the initials of a window of words in `words`.
/// Returns the starting index and window length if found.
pub fn find_acronym_window(token: &str, words: &[&str]) -> Option<(usize, usize)> {
    let bytes = token.as_bytes();
    if bytes.len() < 2 || words.is_empty() {
        return None;
    }

    // 1. Direct window match (window size == token length)
    if words.len() >= bytes.len() {
        for (start_idx, window) in words.windows(bytes.len()).enumerate() {
            let is_match = bytes
                .iter()
                .zip(window.iter())
                .all(|(&b, w)| !w.is_empty() && w.as_bytes()[0] == b);
            if is_match {
                return Some((start_idx, bytes.len()));
            }
        }
    }

    // 2. Connector-tolerant matching for windows of length between 2 and bytes.len()
    let min_len = 2;
    let max_len = words.len().min(bytes.len());
    for win_len in (min_len..=max_len).rev() {
        for (start_idx, window) in words.windows(win_len).enumerate() {
            if matches_acronym_sequence(bytes, window) {
                return Some((start_idx, win_len));
            }
        }
    }

    // 3. Official expansion matching (e.g. "sncf" -> "societe nationale des chemins de fer")
    if let Some(expanded) = sirene_normalize::expand_abbreviation(token) {
        let exp_words: SmallVec<[&str; 8]> = expanded
            .split(' ')
            .filter(|w| {
                !w.is_empty()
                    && !sirene_normalize::is_stop_word(w)
                    && !sirene_normalize::is_legal_form(w)
            })
            .collect();
        if !exp_words.is_empty() && words.len() >= exp_words.len() {
            for (start_idx, window) in words.windows(exp_words.len()).enumerate() {
                if window == exp_words.as_slice() {
                    return Some((start_idx, exp_words.len()));
                }
            }
        }
    }

    None
}

/// Computes an unordered soft token overlap score between two token lists.
///
/// For each token in `tokens1`, finds the highest Jaro-Winkler match in `tokens2`,
/// and vice-versa, computing a symmetric bidirectional average.
///
/// Handles:
/// - Inverted words (e.g. `["plomberie", "dupont"]` vs `["dupont", "plomberie"]` -> 1.0).
/// - Misspellings within individual words.
/// - Acronyms / initialisms (e.g. `"sncf"` vs `"societe", "nationale", "chemins", "fer"`).
pub fn token_overlap_similarity(tokens1: &[&str], tokens2: &[&str]) -> f32 {
    if tokens1.is_empty() && tokens2.is_empty() {
        return 1.0;
    }
    if tokens1.is_empty() || tokens2.is_empty() {
        return 0.0;
    }
    if tokens1 == tokens2 {
        return 1.0;
    }

    // Stack coverage tracking for tokens1 and tokens2
    let mut covered1: SmallVec<[f32; 16]> = smallvec::smallvec![0.0; tokens1.len()];
    let mut covered2: SmallVec<[f32; 16]> = smallvec::smallvec![0.0; tokens2.len()];

    // 1. Check if any t1 in tokens1 is an acronym matching a window in tokens2
    for (i, &t1) in tokens1.iter().enumerate() {
        if let Some((start2, len2)) = find_acronym_window(t1, tokens2) {
            covered1[i] = covered1[i].max(0.95);
            for cov in covered2.iter_mut().skip(start2).take(len2) {
                *cov = (*cov).max(0.95);
            }
        }
    }

    // 2. Check if any t2 in tokens2 is an acronym matching a window in tokens1
    for (j, &t2) in tokens2.iter().enumerate() {
        if let Some((start1, len1)) = find_acronym_window(t2, tokens1) {
            covered2[j] = covered2[j].max(0.95);
            for cov in covered1.iter_mut().skip(start1).take(len1) {
                *cov = (*cov).max(0.95);
            }
        }
    }

    let mut score_1 = 0.0f32;
    for (i, &t1) in tokens1.iter().enumerate() {
        let mut max_sim = covered1[i];
        let mut best_j = None;
        for (j, &t2) in tokens2.iter().enumerate() {
            let sim = jaro_winkler_similarity(t1, t2);
            if sim > max_sim {
                max_sim = sim;
                best_j = Some(j);
            }
            if max_sim >= 1.0 {
                break;
            }
        }
        score_1 += max_sim;
        if let Some(j) = best_j {
            if j < covered2.len() && covered2[j] < max_sim {
                covered2[j] = max_sim;
            }
        }
    }
    score_1 /= tokens1.len() as f32;

    // Evaluate coverage of tokens2
    let mut score_2 = 0.0f32;
    for (j, &t2) in tokens2.iter().enumerate() {
        let mut max_sim = covered2[j];
        for &t1 in tokens1 {
            let sim = jaro_winkler_similarity(t2, t1);
            if sim > max_sim {
                max_sim = sim;
            }
            if max_sim >= 1.0 {
                break;
            }
        }
        score_2 += max_sim;
    }
    score_2 /= tokens2.len() as f32;

    ((score_1 + score_2) / 2.0).clamp(0.0, 1.0)
}

/// Computes the composite denomination similarity $S_{\text{nom}}$ combining
/// full-string Jaro-Winkler distance and unordered token overlap.
///
/// $$S_{\text{nom}} = w_{\text{jw}} \cdot S_{\text{jw}} + (1 - w_{\text{jw}}) \cdot S_{\text{tokens}}$$
pub fn score_denomination(
    s1: &str,
    tokens1: &[&str],
    s2: &str,
    tokens2: &[&str],
    jw_weight: f32,
) -> f32 {
    if tokens1 == tokens2 || (s1 == s2 && !s1.is_empty()) {
        return 1.0;
    }

    let s_jw = jaro_winkler_similarity(s1, s2);
    let s_tok = token_overlap_similarity(tokens1, tokens2);

    let weight = jw_weight.clamp(0.0, 1.0);
    let combined = weight * s_jw + (1.0 - weight) * s_tok;
    combined.max(s_tok).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inverted_tokens_tolerance() {
        let t1 = ["voyageurs", "sncf"];
        let t2 = ["sncf", "voyageurs"];
        let sim = token_overlap_similarity(&t1, &t2);
        assert_eq!(sim, 1.0);

        let denom_sim = score_denomination("voyageurs sncf", &t1, "sncf voyageurs", &t2, 0.5);
        assert_eq!(denom_sim, 1.0);
    }

    #[test]
    fn test_acronym_matching() {
        let t1 = ["sncf", "voyageurs"];
        let t2 = [
            "societe",
            "nationale",
            "chemins",
            "fer",
            "francais",
            "voyageurs",
        ];
        let sim = token_overlap_similarity(&t1, &t2);
        assert!(sim > 0.85, "Expected sim > 0.85, got {}", sim);
    }

    #[test]
    fn test_dupont_plomberie_token_matching() {
        let t1 = ["dupont", "plomberie"];
        let t2 = ["dupont", "plomberie"];
        assert_eq!(token_overlap_similarity(&t1, &t2), 1.0);
    }
}
