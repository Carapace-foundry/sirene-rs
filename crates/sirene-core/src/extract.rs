use crate::siren::Siren;
use crate::siret::Siret;

/// Pivot numeric identifier extracted from a search query or record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdentifiantPivot {
    Siren(Siren),
    Siret(Siret),
}

impl IdentifiantPivot {
    /// Returns the associated 9-digit `Siren`.
    #[inline]
    pub fn siren(&self) -> Siren {
        match self {
            Self::Siren(s) => *s,
            Self::Siret(s) => s.siren(),
        }
    }

    /// Returns the 14-digit `Siret` if this is an establishment identifier.
    #[inline]
    pub fn siret(&self) -> Option<Siret> {
        match self {
            Self::Siren(_) => None,
            Self::Siret(s) => Some(*s),
        }
    }

    /// Returns the string representation.
    #[inline]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Siren(s) => s.as_str(),
            Self::Siret(s) => s.as_str(),
        }
    }
}

/// Attempts to extract a strictly valid SIRET (14 digits) or SIREN (9 digits)
/// from a raw user query string, testing for exact short-circuit matches.
///
/// Handles inputs with spaces, dots, dashes, or slashes (e.g. `"732 829 320"`,
/// `"443.061.841.00047"`, or `"Entreprise 732829320"`).
pub fn extract_pivot(query: &str) -> Option<IdentifiantPivot> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return None;
    }

    // 1. Direct whole-query attempt (with whitespace/separators filtered)
    if let Ok(siret) = Siret::parse(trimmed) {
        return Some(IdentifiantPivot::Siret(siret));
    }
    if let Ok(siren) = Siren::parse(trimmed) {
        return Some(IdentifiantPivot::Siren(siren));
    }

    // 2. Scan query for digit sequences that might form a valid SIRET or SIREN
    let chars: Vec<(usize, char)> = trimmed.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].1.is_ascii_digit() {
            let start_byte = chars[i].0;
            let mut digit_count = 0;
            let mut j = i;
            let mut last_char_idx = i;

            while j < chars.len() {
                let (_, ch) = chars[j];
                if ch.is_ascii_digit() {
                    digit_count += 1;
                    last_char_idx = j;
                    j += 1;
                } else if (ch.is_whitespace() || ch == '.' || ch == '-' || ch == '_')
                    && j + 1 < chars.len()
                    && chars[j + 1].1.is_ascii_digit()
                {
                    // separator inside a digit cluster
                    j += 1;
                } else {
                    break;
                }
            }

            let end_byte = chars[last_char_idx].0 + chars[last_char_idx].1.len_utf8();
            let candidate = &trimmed[start_byte..end_byte];

            if digit_count == 14 {
                if let Ok(siret) = Siret::parse(candidate) {
                    return Some(IdentifiantPivot::Siret(siret));
                }
            } else if digit_count == 9 {
                if let Ok(siren) = Siren::parse(candidate) {
                    return Some(IdentifiantPivot::Siren(siren));
                }
            }

            i = j;
        } else {
            i += 1;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_direct() {
        let res = extract_pivot("732 829 320");
        assert!(matches!(res, Some(IdentifiantPivot::Siren(_))));
        if let Some(IdentifiantPivot::Siren(s)) = res {
            assert_eq!(s.as_str(), "732829320");
        }

        let res2 = extract_pivot("732.829.320.00074");
        assert!(matches!(res2, Some(IdentifiantPivot::Siret(_))));
    }

    #[test]
    fn test_extract_embedded() {
        let res = extract_pivot("Boulangerie Martin 732 829 320 Paris");
        assert!(matches!(res, Some(IdentifiantPivot::Siren(_))));

        let res2 = extract_pivot("Societe Dupont SIRET 73282932000074 fin");
        assert!(matches!(res2, Some(IdentifiantPivot::Siret(_))));
    }

    #[test]
    fn test_extract_invalid() {
        assert_eq!(extract_pivot("Boulangerie Martin"), None);
        // Invalid Luhn
        assert_eq!(extract_pivot("732829321"), None);
    }

    #[test]
    fn test_extract_unicode_whitespace() {
        // French non-breaking space \u{00A0}
        let res1 = extract_pivot("Entreprise 732\u{00a0}829\u{00a0}320 Paris");
        assert!(matches!(res1, Some(IdentifiantPivot::Siren(_))));
        if let Some(IdentifiantPivot::Siren(s)) = res1 {
            assert_eq!(s.as_str(), "732829320");
        }

        // Narrow non-breaking space \u{202F}
        let res2 = extract_pivot("SIRET: 732\u{202f}829\u{202f}320\u{202f}00074");
        assert!(matches!(res2, Some(IdentifiantPivot::Siret(_))));
        if let Some(IdentifiantPivot::Siret(s)) = res2 {
            assert_eq!(s.as_str(), "73282932000074");
        }
    }
}
