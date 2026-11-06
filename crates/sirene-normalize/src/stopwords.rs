//! French stop words and elision prefixes filter.
//! Conforms to DINUM (Annuaire des Entreprises / api-recherche-entreprises) standards.

/// Returns `true` if the normalized lowercase token is an isolated French elision prefix.
///
/// Handles tokens resulting from splitting elided words:
/// `d'`, `l'`, `m'`, `n'`, `s'`, `t'`, `qu'`, `c'`, `j'`.
#[inline]
pub fn is_elision_prefix(token: &str) -> bool {
    matches!(token, "d" | "l" | "m" | "n" | "s" | "t" | "qu" | "c" | "j")
}

/// Returns `true` if the token is a French stop word without distinctive business search value.
///
/// List conforms to DINUM guidelines:
/// `de`, `du`, `des`, `et`, `le`, `la`, `les`, `pour`, `en`, `par`, `un`, `une`, `au`, `aux`, `sur`, etc.
#[inline]
pub fn is_stop_word(token: &str) -> bool {
    matches!(
        token,
        "de" | "du"
            | "des"
            | "et"
            | "le"
            | "la"
            | "les"
            | "pour"
            | "en"
            | "par"
            | "un"
            | "une"
            | "au"
            | "aux"
            | "sur"
            | "dans"
            | "avec"
            | "chez"
            | "a"
            | "ce"
            | "cet"
            | "cette"
            | "ces"
            | "son"
            | "sa"
            | "ses"
            | "leur"
            | "leurs"
            | "sous"
            | "sans"
            | "vers"
    )
}

/// Returns `true` if the token is either a stop word or an elision prefix.
#[inline]
pub fn is_noise_word(token: &str) -> bool {
    is_elision_prefix(token) || is_stop_word(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elision_prefixes() {
        assert!(is_elision_prefix("d"));
        assert!(is_elision_prefix("l"));
        assert!(is_elision_prefix("qu"));
        assert!(!is_elision_prefix("art"));
        assert!(!is_elision_prefix("boulangerie"));
    }

    #[test]
    fn test_stop_words() {
        assert!(is_stop_word("de"));
        assert!(is_stop_word("du"));
        assert!(is_stop_word("des"));
        assert!(is_stop_word("et"));
        assert!(is_stop_word("le"));
        assert!(is_stop_word("la"));
        assert!(is_stop_word("les"));
        assert!(is_stop_word("pour"));
        assert!(is_stop_word("en"));
        assert!(is_stop_word("par"));

        assert!(!is_stop_word("dupont"));
        assert!(!is_stop_word("martin"));
    }
}
