use smallvec::SmallVec;
use std::borrow::Cow;

use crate::legal::{expand_abbreviation, is_legal_form};
use crate::stopwords::{is_elision_prefix, is_stop_word};

/// Configuration options for text tokenization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenizerOptions {
    /// Whether to remove common French corporate legal forms (e.g. `"sarl"`, `"sas"`, `"sci"`).
    pub strip_legal_forms: bool,
    /// Whether to expand abbreviations (e.g. `"ste"` -> `"societe"`, `"btp"` -> `"batiment travaux publics"`).
    pub expand_abbreviations: bool,
    /// Whether to filter out French stop words (e.g. `"de"`, `"du"`, `"des"`, `"et"`).
    pub filter_stop_words: bool,
    /// Whether to filter out French elision prefixes (e.g. `"d"`, `"l"`, `"qu"`).
    pub filter_elisions: bool,
    /// Minimum token length to retain (default: 1).
    pub min_token_length: usize,
}

impl Default for TokenizerOptions {
    fn default() -> Self {
        Self {
            strip_legal_forms: true,
            expand_abbreviations: false,
            filter_stop_words: true,
            filter_elisions: true,
            min_token_length: 1,
        }
    }
}

/// High-performance stack-allocated tokenizer using `SmallVec`.
///
/// Designed to eliminate heap allocations for typical French enterprise names
/// (which usually comprise 1 to 8 words).
#[derive(Debug, Clone, Copy, Default)]
pub struct Tokenizer {
    options: TokenizerOptions,
}

impl Tokenizer {
    /// Creates a tokenizer with custom options.
    pub const fn new(options: TokenizerOptions) -> Self {
        Self { options }
    }

    /// Returns the current configuration options.
    pub const fn options(&self) -> &TokenizerOptions {
        &self.options
    }

    /// Tokenizes an ASCII-folded string into stack-allocated slices.
    ///
    /// Preserves original tokens as a fallback if strict filtering would otherwise
    /// leave the token list empty (e.g. for queries that only contain a legal form like `"SARL"`).
    pub fn tokenize<'a>(&self, text: &'a str) -> SmallVec<[&'a str; 8]> {
        let mut tokens: SmallVec<[&'a str; 8]> = SmallVec::new();
        let mut all_raw_tokens: SmallVec<[&'a str; 8]> = SmallVec::new();

        for word in text.split(' ') {
            if word.is_empty() {
                continue;
            }

            all_raw_tokens.push(word);

            if word.len() < self.options.min_token_length {
                continue;
            }

            if self.options.filter_elisions && is_elision_prefix(word) {
                continue;
            }

            if self.options.filter_stop_words && is_stop_word(word) {
                continue;
            }

            if self.options.strip_legal_forms && is_legal_form(word) {
                continue;
            }

            tokens.push(word);
        }

        // Safeguard: if filtering removed everything, preserve the raw non-empty tokens
        if tokens.is_empty() && !all_raw_tokens.is_empty() {
            return all_raw_tokens;
        }

        tokens
    }

    /// Tokenizes an ASCII-folded string, resolving abbreviations into full wording.
    pub fn tokenize_expanded<'a>(&self, text: &'a str) -> SmallVec<[Cow<'a, str>; 8]> {
        let mut tokens: SmallVec<[Cow<'a, str>; 8]> = SmallVec::new();
        let mut all_raw_tokens: SmallVec<[Cow<'a, str>; 8]> = SmallVec::new();

        for word in text.split(' ') {
            if word.is_empty() {
                continue;
            }

            all_raw_tokens.push(Cow::Borrowed(word));

            if self.options.filter_elisions && is_elision_prefix(word) {
                continue;
            }

            if self.options.filter_stop_words && is_stop_word(word) {
                continue;
            }

            if self.options.strip_legal_forms && is_legal_form(word) {
                continue;
            }

            if self.options.expand_abbreviations {
                if let Some(expanded) = expand_abbreviation(word) {
                    for part in expanded.split(' ') {
                        if !part.is_empty() {
                            tokens.push(Cow::Borrowed(part));
                        }
                    }
                    continue;
                }
            }

            if word.len() < self.options.min_token_length {
                continue;
            }

            tokens.push(Cow::Borrowed(word));
        }

        if tokens.is_empty() && !all_raw_tokens.is_empty() {
            return all_raw_tokens;
        }

        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_clean_distinctive() {
        let tokenizer = Tokenizer::default();
        let tokens = tokenizer.tokenize("dupont plomberie sasu");
        assert_eq!(tokens.as_slice(), &["dupont", "plomberie"]);
    }

    #[test]
    fn test_tokenize_complex_legal_noise() {
        let tokenizer = Tokenizer::default();
        let tokens = tokenizer.tokenize("ste d exploitation des ets dupont cie");
        assert_eq!(tokens.as_slice(), &["exploitation", "dupont"]);
    }

    #[test]
    fn test_tokenize_fallback_safeguard() {
        let tokenizer = Tokenizer::default();
        // Query that is ONLY a legal form
        let tokens = tokenizer.tokenize("sarl");
        assert_eq!(tokens.as_slice(), &["sarl"]);
    }

    #[test]
    fn test_tokenize_expanded() {
        let opts = TokenizerOptions {
            strip_legal_forms: false,
            expand_abbreviations: true,
            filter_stop_words: true,
            filter_elisions: true,
            min_token_length: 1,
        };
        let tokenizer = Tokenizer::new(opts);
        let tokens = tokenizer.tokenize_expanded("btp dupont");
        assert_eq!(
            tokens.as_slice(),
            &["batiment", "travaux", "publics", "dupont"]
        );
    }
}
