use smallvec::SmallVec;
use std::borrow::Cow;

use sirene_core::{extract_pivot, IdentifiantPivot};

use crate::ascii::to_ascii_folded;
use crate::legal::is_legal_form;
use crate::tokenizer::{Tokenizer, TokenizerOptions};

/// A fully analyzed, normalized search query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedQuery {
    /// Raw query string as provided by the user.
    pub raw: String,
    /// Transliterated ASCII lowercase string with normalized punctuation.
    pub folded: String,
    /// Stack-allocated distinctive tokens (legal noise and stop words stripped).
    pub tokens: SmallVec<[String; 8]>,
    /// Legal forms identified in the query (e.g. `"sasu"`).
    pub detected_legal_forms: SmallVec<[String; 2]>,
    /// Exact SIREN or SIRET numeric key if detected in the query.
    pub pivot: Option<IdentifiantPivot>,
}

impl NormalizedQuery {
    /// Returns `true` if the query can be short-circuited via an exact SIREN/SIRET lookup.
    #[inline]
    pub fn is_exact_pivot(&self) -> bool {
        self.pivot.is_some()
    }

    /// Returns the distinctive tokens joined by a single space.
    pub fn distinctive_text(&self) -> String {
        self.tokens.join(" ")
    }
}

/// The unified SIRENE / RNE normalization engine conforming to DINUM standards.
#[derive(Debug, Clone, Copy, Default)]
pub struct Normalizer {
    tokenizer: Tokenizer,
}

impl Normalizer {
    /// Creates a default `Normalizer` with standard DINUM legal noise and stop word stripping.
    pub fn new() -> Self {
        Self {
            tokenizer: Tokenizer::default(),
        }
    }

    /// Creates a `Normalizer` with custom tokenizer options.
    pub const fn with_options(options: TokenizerOptions) -> Self {
        Self {
            tokenizer: Tokenizer::new(options),
        }
    }

    /// Translates text into folded ASCII lowercase without punctuation.
    ///
    /// Returns `Cow::Borrowed` with zero allocation if the input is already clean ASCII.
    #[inline]
    pub fn fold_ascii<'a>(&self, text: &'a str) -> Cow<'a, str> {
        to_ascii_folded(text)
    }

    /// Prepares corporate name text for compact indexing (e.g. FST / trigram dictionary).
    ///
    /// Applies ASCII folding, strips legal noise, and normalizes spacing.
    pub fn normalize_for_indexing(&self, text: &str) -> String {
        let folded = to_ascii_folded(text);
        let tokens = self.tokenizer.tokenize(&folded);
        tokens.join(" ")
    }

    /// Fully analyzes and normalizes an input search query string.
    pub fn normalize_query(&self, raw: &str) -> NormalizedQuery {
        let pivot = extract_pivot(raw);
        let folded_cow = to_ascii_folded(raw);
        let folded = folded_cow.into_owned();

        let mut detected_legal_forms = SmallVec::new();
        for word in folded.split(' ') {
            if is_legal_form(word) {
                detected_legal_forms.push(word.to_string());
            }
        }

        let token_slices = self.tokenizer.tokenize(&folded);
        let tokens: SmallVec<[String; 8]> =
            token_slices.into_iter().map(|s| s.to_string()).collect();

        NormalizedQuery {
            raw: raw.to_string(),
            folded,
            tokens,
            detected_legal_forms,
            pivot,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalizer_dupont_plomberie_sasu() {
        let normalizer = Normalizer::new();
        let q1 = normalizer.normalize_query("Dupont Plomberie SASU");
        let q2 = normalizer.normalize_query("Dupont Plomberie");

        assert_eq!(q1.distinctive_text(), q2.distinctive_text());
        assert_eq!(q1.tokens.as_slice(), &["dupont", "plomberie"]);
        assert_eq!(q1.detected_legal_forms.as_slice(), &["sasu"]);
        assert_eq!(q1.pivot, None);
    }

    #[test]
    fn test_normalizer_with_pivot_siren() {
        let normalizer = Normalizer::new();
        let q = normalizer.normalize_query("Société Générale 552 120 222 Paris");
        assert!(q.is_exact_pivot());
        if let Some(IdentifiantPivot::Siren(s)) = q.pivot {
            assert_eq!(s.as_str(), "552120222");
        }
    }
}
