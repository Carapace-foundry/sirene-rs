use std::borrow::Cow;
use unicode_normalization::UnicodeNormalization;

/// Checks if a string slice is already canonical ASCII lowercase:
/// - Only contains ASCII lowercase letters `'a'..'z'`, digits `'0'..'9'`, and single spaces.
/// - No leading or trailing spaces.
/// - No consecutive spaces.
#[inline]
pub fn is_ascii_clean(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return true;
    }
    if bytes[0] == b' ' || bytes[bytes.len() - 1] == b' ' {
        return false;
    }

    let mut prev_space = false;
    for &b in bytes {
        match b {
            b'a'..=b'z' | b'0'..=b'9' => {
                prev_space = false;
            }
            b' ' => {
                if prev_space {
                    return false;
                }
                prev_space = true;
            }
            _ => return false,
        }
    }
    true
}

/// Translates a character into an ASCII equivalent, expanding French ligatures,
/// lowercasing, and mapping punctuation to space.
#[inline]
fn map_char_to_ascii(c: char, out: &mut String) {
    match c {
        // French ligatures
        'œ' | 'Œ' => out.push_str("oe"),
        'æ' | 'Æ' => out.push_str("ae"),

        // German sharp S
        'ß' => out.push_str("ss"),

        // ASCII alphanumeric
        'a'..='z' | '0'..='9' => out.push(c),
        'A'..='Z' => out.push(c.to_ascii_lowercase()),

        // Apostrophes & quotes
        '\'' | '`' | '’' | '‘' | 'ʻ' | 'ʼ' | '"' | '«' | '»' | '“' | '”' => {
            out.push(' ')
        }

        // Common punctuation & separators
        '-' | '_' | '.' | ',' | ';' | ':' | '/' | '\\' | '|' | '(' | ')' | '[' | ']' | '{'
        | '}' | '<' | '>' | '!' | '?' | '+' | '=' | '*' | '&' | '%' | '$' | '€' | '#' | '@'
        | '~' | '^' => out.push(' '),

        // All whitespace
        c if c.is_whitespace() => out.push(' '),

        // Other Unicode chars (handled via decomposition or fallback)
        other => {
            // Apply NFKD decomposition for accents/diacritics
            for decomposed in other.to_lowercase().nfkd() {
                // Skip combining diacritical marks (accents, cedillas, tildes, trema...)
                if ('\u{0300}'..='\u{036F}').contains(&decomposed) {
                    continue;
                }
                if decomposed.is_ascii_alphanumeric() {
                    out.push(decomposed);
                } else if decomposed.is_whitespace() || decomposed.is_ascii_punctuation() {
                    out.push(' ');
                }
            }
        }
    }
}

/// Performs complete Unicode diacritics folding, ASCII transliteration, and space normalization.
///
/// Guarantees:
/// - Absolute suppression of accents and diacritics (É, È, Ê, Ë -> e, Ç -> c, Œ -> oe, Æ -> ae).
/// - Systemic lowercasing.
/// - Replacement of all punctuation with single normalized spaces.
/// - Zero heap allocation if the input string is already canonical ASCII lowercase (`Cow::Borrowed`).
pub fn to_ascii_folded(input: &str) -> Cow<'_, str> {
    if is_ascii_clean(input) {
        return Cow::Borrowed(input);
    }

    let mut buf = String::with_capacity(input.len());
    for c in input.chars() {
        map_char_to_ascii(c, &mut buf);
    }

    // Collapse multiple consecutive spaces and trim edges
    let mut normalized = String::with_capacity(buf.len());
    let mut in_space = true; // initially true to strip leading spaces

    for c in buf.chars() {
        if c == ' ' {
            if !in_space {
                normalized.push(' ');
                in_space = true;
            }
        } else {
            normalized.push(c);
            in_space = false;
        }
    }

    // Strip trailing space if present
    if normalized.ends_with(' ') {
        normalized.pop();
    }

    Cow::Owned(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_allocation_when_already_clean() {
        let clean = "boucherie martin";
        match to_ascii_folded(clean) {
            Cow::Borrowed(b) => assert_eq!(b, clean),
            Cow::Owned(_) => panic!("Expected Borrowed for already clean string"),
        }
    }

    #[test]
    fn test_accents_and_diacritics_folding() {
        assert_eq!(to_ascii_folded("Société Électrique"), "societe electrique");
        assert_eq!(
            to_ascii_folded("À l'Entrepôt Créatif"),
            "a l entrepot creatif"
        );
        assert_eq!(to_ascii_folded("GARÇON & CIE"), "garcon cie");
    }

    #[test]
    fn test_ligatures_folding() {
        assert_eq!(to_ascii_folded("Bœuf"), "boeuf");
        assert_eq!(to_ascii_folded("Cœur de Ville"), "coeur de ville");
        assert_eq!(to_ascii_folded("EX ÆQUO"), "ex aequo");
    }

    #[test]
    fn test_punctuation_and_spaces_normalization() {
        assert_eq!(
            to_ascii_folded("  Dupont - Plomberie...  SASU / Paris  "),
            "dupont plomberie sasu paris"
        );
    }
}
