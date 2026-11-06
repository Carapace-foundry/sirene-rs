use sirene_core::IdentifiantPivot;
use sirene_normalize::{is_ascii_clean, to_ascii_folded, Normalizer, Tokenizer, TokenizerOptions};
use std::borrow::Cow;

#[test]
fn test_ground_truth_dupont_plomberie_sasu() {
    let normalizer = Normalizer::new();
    let q1 = normalizer.normalize_query("Dupont Plomberie SASU");
    let q2 = normalizer.normalize_query("Dupont Plomberie");

    assert_eq!(q1.distinctive_text(), q2.distinctive_text());
    assert_eq!(q1.distinctive_text(), "dupont plomberie");
    assert_eq!(q1.detected_legal_forms.as_slice(), &["sasu"]);
    assert!(q2.detected_legal_forms.is_empty());
}

#[test]
fn test_ground_truth_ste_d_exploitation_des_ets_dupont_et_cie() {
    let normalizer = Normalizer::new();
    let q = normalizer.normalize_query("STE D'EXPLOITATION DES ETS DUPONT & CIE");

    assert_eq!(q.distinctive_text(), "exploitation dupont");
    assert_eq!(q.tokens.as_slice(), &["exploitation", "dupont"]);
    assert_eq!(q.detected_legal_forms.as_slice(), &["ste", "ets", "cie"]);
}

#[test]
fn test_ground_truth_elision_and_stop_words() {
    let normalizer = Normalizer::new();
    let q = normalizer.normalize_query("L'Atelier d'Art Moderne");
    assert_eq!(q.tokens.as_slice(), &["atelier", "art", "moderne"]);
    assert_eq!(q.distinctive_text(), "atelier art moderne");
}

#[test]
fn test_unicode_transliteration_and_ligatures() {
    assert_eq!(
        to_ascii_folded("Épicerie & Café des Sœurs"),
        "epicerie cafe des soeurs"
    );
    assert_eq!(to_ascii_folded("Bœuf Bourguignon"), "boeuf bourguignon");
    assert_eq!(to_ascii_folded("EX-ÆQUO CONSULTING"), "ex aequo consulting");
    assert_eq!(to_ascii_folded("Ça va bien !"), "ca va bien");
}

#[test]
fn test_zero_allocation_borrowed_ascii() {
    let raw = "boucherie dupan";
    assert!(is_ascii_clean(raw));
    match to_ascii_folded(raw) {
        Cow::Borrowed(s) => assert_eq!(s, raw),
        Cow::Owned(_) => panic!("Should have borrowed input string"),
    }
}

#[test]
fn test_abbreviation_expansion_pipeline() {
    let opts = TokenizerOptions {
        strip_legal_forms: false,
        expand_abbreviations: true,
        filter_stop_words: true,
        filter_elisions: true,
        min_token_length: 1,
    };
    let normalizer = Normalizer::with_options(opts);
    let folded = normalizer.fold_ascii("ETS BTP MARTIN & CIE");
    let tokenizer = Tokenizer::new(opts);
    let tokens = tokenizer.tokenize_expanded(&folded);

    let token_strs: Vec<&str> = tokens.iter().map(|c| c.as_ref()).collect();
    assert_eq!(
        token_strs,
        vec![
            "etablissements",
            "batiment",
            "travaux",
            "publics",
            "martin",
            "compagnie"
        ]
    );
}

#[test]
fn test_query_with_pivot_extraction() {
    let normalizer = Normalizer::new();
    let q = normalizer.normalize_query("Dupont Plomberie SAS 732 829 320 Paris");

    assert!(q.is_exact_pivot());
    if let Some(IdentifiantPivot::Siren(siren)) = q.pivot {
        assert_eq!(siren.as_str(), "732829320");
    } else {
        panic!("Expected SIREN pivot");
    }
}

#[test]
fn test_normalize_edge_cases() {
    use sirene_normalize::{
        expand_abbreviation, is_elision_prefix, is_legal_form, is_noise_word, is_stop_word,
    };

    let normalizer = Normalizer::default();

    // 1. Empty string
    let empty_q = normalizer.normalize_query("");
    assert!(empty_q.tokens.is_empty());
    assert_eq!(empty_q.distinctive_text(), "");

    // 2. Query with only stop words (fallback safeguard preserves them rather than returning empty)
    let stop_only = normalizer.normalize_query("de du des");
    assert!(!stop_only.tokens.is_empty());

    // 3. Query with only legal noise (fallback safeguard preserves them)
    let legal_only = normalizer.normalize_query("SARL SAS");
    assert!(!legal_only.tokens.is_empty());

    // 4. Utility function checks
    assert_eq!(expand_abbreviation("unknown_abbr"), None);
    assert!(!is_legal_form("boucherie"));
    assert!(is_legal_form("sarl"));
    assert!(is_stop_word("de"));
    assert!(!is_stop_word("informatique"));
    assert!(is_noise_word("de"));
    assert!(!is_noise_word("boucherie"));
    assert!(is_elision_prefix("d"));
    assert!(is_elision_prefix("l"));
    assert!(!is_elision_prefix("xyz"));
}
