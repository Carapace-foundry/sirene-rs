use sirene_core::{CodeCommune, CodePostal, EtatAdministratif, Siren};
use sirene_matcher::{EntityMatcher, LocationQuery, MatchCandidate, SearchQuery};
use sirene_normalize::Normalizer;

#[test]
fn test_ground_truth_typo_boucherie_dupan_dupont() {
    let normalizer = Normalizer::new();
    let query_norm = normalizer.normalize_query("Boucherie Dupan");
    let search = SearchQuery {
        query: &query_norm,
        location: None,
        naf: None,
        dirigeant: None,
        strict_location: false,
    };

    let siren = Siren::parse("732829320").expect("siren");
    let candidate = MatchCandidate {
        siren,
        siret: None,
        denomination: "boucherie dupont",
        tokens: &["boucherie", "dupont"],
        code_commune: None,
        code_postal: None,
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: &[],
    };

    let matcher = EntityMatcher::new();
    let scored = matcher.score_candidate(&search, &candidate);

    assert!(
        scored.score() > 0.90,
        "Typo Boucherie Dupan should score > 0.90 with Boucherie Dupont, got {}",
        scored.score()
    );
}

#[test]
fn test_ground_truth_sncf_voyageurs_acronym() {
    let normalizer = Normalizer::new();
    let query_norm = normalizer.normalize_query("SNCF Voyageurs");
    let search = SearchQuery {
        query: &query_norm,
        location: None,
        naf: None,
        dirigeant: None,
        strict_location: false,
    };

    let siren = Siren::parse("732829320").expect("siren");
    let candidate = MatchCandidate {
        siren,
        siret: None,
        denomination: "societe nationale chemins fer francais voyageurs",
        tokens: &[
            "societe",
            "nationale",
            "chemins",
            "fer",
            "francais",
            "voyageurs",
        ],
        code_commune: None,
        code_postal: None,
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: &[],
    };

    let matcher = EntityMatcher::new();
    let scored = matcher.score_candidate(&search, &candidate);

    assert!(
        scored.score() > 0.85,
        "Acronym SNCF Voyageurs should match developed form with score > 0.85, got {}",
        scored.score()
    );

    // Symmetric reverse test: Query developed form against candidate acronym
    let q_rev =
        normalizer.normalize_query("Société Nationale des Chemins de Fer Français Voyageurs");
    let search_rev = SearchQuery {
        query: &q_rev,
        location: None,
        naf: None,
        dirigeant: None,
        strict_location: false,
    };
    let candidate_rev = MatchCandidate {
        siren,
        siret: None,
        denomination: "sncf voyageurs",
        tokens: &["sncf", "voyageurs"],
        code_commune: None,
        code_postal: None,
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: &[],
    };
    let scored_rev = matcher.score_candidate(&search_rev, &candidate_rev);
    assert!(
        scored_rev.score() > 0.85,
        "Developed query should match acronym candidate with score > 0.85, got {}",
        scored_rev.score()
    );
}

#[test]
fn test_ground_truth_dupont_plomberie_sasu() {
    let normalizer = Normalizer::new();
    let query_norm = normalizer.normalize_query("Dupont Plomberie SASU");
    let search = SearchQuery {
        query: &query_norm,
        location: None,
        naf: None,
        dirigeant: None,
        strict_location: false,
    };

    let siren = Siren::parse("732829320").expect("siren");
    let candidate = MatchCandidate {
        siren,
        siret: None,
        denomination: "dupont plomberie",
        tokens: &["dupont", "plomberie"],
        code_commune: None,
        code_postal: None,
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: &[],
    };

    let matcher = EntityMatcher::new();
    let scored = matcher.score_candidate(&search, &candidate);

    assert!(
        scored.sub_scores.score_nom >= 0.99,
        "Dupont Plomberie SASU should perfectly match Dupont Plomberie, got {}",
        scored.sub_scores.score_nom
    );
}

#[test]
fn test_ground_truth_geographic_disambiguation() {
    let normalizer = Normalizer::new();
    let query_norm = normalizer.normalize_query("Boulangerie Martin");
    let location_bordeaux = LocationQuery::parse("33000");

    let search = SearchQuery {
        query: &query_norm,
        location: location_bordeaux,
        naf: None,
        dirigeant: None,
        strict_location: false,
    };

    let siren1 = Siren::parse("732829320").expect("siren1");
    let com_bordeaux = CodeCommune::parse("33063").expect("bordeaux");
    let cp_bordeaux = CodePostal::parse("33000").expect("bordeaux cp");

    let candidate_bordeaux = MatchCandidate {
        siren: siren1,
        siret: None,
        denomination: "boulangerie martin",
        tokens: &["boulangerie", "martin"],
        code_commune: Some(com_bordeaux),
        code_postal: Some(cp_bordeaux),
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: &[],
    };

    let siren2 = Siren::parse("443061841").expect("siren2");
    let com_lille = CodeCommune::parse("59350").expect("lille");
    let cp_lille = CodePostal::parse("59000").expect("lille cp");

    let candidate_lille = MatchCandidate {
        siren: siren2,
        siret: None,
        denomination: "boulangerie martin",
        tokens: &["boulangerie", "martin"],
        code_commune: Some(com_lille),
        code_postal: Some(cp_lille),
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: &[],
    };

    let candidates = [candidate_lille.clone(), candidate_bordeaux.clone()];
    let matcher = EntityMatcher::new();
    let ranked = matcher.rank(&search, &candidates, 10);

    assert_eq!(ranked.len(), 2);
    // Bordeaux MUST be #1
    assert_eq!(ranked[0].candidate.siren, siren1);
    // Lille MUST be demoted
    assert_eq!(ranked[1].candidate.siren, siren2);
    assert!(
        ranked[0].score() > ranked[1].score(),
        "Bordeaux ({}) must score significantly higher than Lille ({})",
        ranked[0].score(),
        ranked[1].score()
    );
}

#[test]
fn test_ground_truth_dirigeants_rne_boost() {
    let normalizer = Normalizer::new();
    let query_norm = normalizer.normalize_query("Tech Solutions");

    let search = SearchQuery {
        query: &query_norm,
        location: None,
        naf: None,
        dirigeant: Some("thomas durand"),
        strict_location: false,
    };

    let siren1 = Siren::parse("732829320").expect("siren1");
    let candidate_with_dir = MatchCandidate {
        siren: siren1,
        siret: None,
        denomination: "tech solutions",
        tokens: &["tech", "solutions"],
        code_commune: None,
        code_postal: None,
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: &["thomas durand"],
    };

    let siren2 = Siren::parse("443061841").expect("siren2");
    let candidate_other_dir = MatchCandidate {
        siren: siren2,
        siret: None,
        denomination: "tech solutions",
        tokens: &["tech", "solutions"],
        code_commune: None,
        code_postal: None,
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: &["pierre dupont"],
    };

    let candidates = [candidate_other_dir, candidate_with_dir];
    let matcher = EntityMatcher::new();
    let ranked = matcher.rank(&search, &candidates, 10);

    assert_eq!(ranked[0].candidate.siren, siren1);
    assert!(ranked[0].score() > ranked[1].score());
}

#[test]
fn test_pivot_short_circuit() {
    let normalizer = Normalizer::new();
    let query_norm = normalizer.normalize_query("Tech Solutions 732 829 320 Paris");

    let search = SearchQuery {
        query: &query_norm,
        location: None,
        naf: None,
        dirigeant: None,
        strict_location: false,
    };

    let siren1 = Siren::parse("732829320").expect("siren1");
    let siren2 = Siren::parse("443061841").expect("siren2");

    let cand_match = MatchCandidate {
        siren: siren1,
        siret: None,
        denomination: "autre entreprise",
        tokens: &["autre", "entreprise"],
        code_commune: None,
        code_postal: None,
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: &[],
    };

    let cand_nom = MatchCandidate {
        siren: siren2,
        siret: None,
        denomination: "tech solutions",
        tokens: &["tech", "solutions"],
        code_commune: None,
        code_postal: None,
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: &[],
    };

    let candidates = [cand_nom, cand_match];
    let matcher = EntityMatcher::new();
    let ranked = matcher.rank(&search, &candidates, 10);

    // Exact SIREN match MUST be #1 with is_exact_pivot = true
    assert_eq!(ranked[0].candidate.siren, siren1);
    assert!(ranked[0].is_exact_pivot);
    assert_eq!(ranked[0].score(), 1.0);
}

#[test]
fn test_active_vs_ceased_status_penalty() {
    let normalizer = Normalizer::new();
    let query_norm = normalizer.normalize_query("Restaurant Le Gourmet");
    let search = SearchQuery {
        query: &query_norm,
        location: None,
        naf: None,
        dirigeant: None,
        strict_location: false,
    };

    let siren1 = Siren::parse("732829320").expect("siren1");
    let siren2 = Siren::parse("443061841").expect("siren2");

    let cand_active = MatchCandidate {
        siren: siren1,
        siret: None,
        denomination: "restaurant le gourmet",
        tokens: &["restaurant", "gourmet"],
        code_commune: None,
        code_postal: None,
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: &[],
    };

    let cand_ceased = MatchCandidate {
        siren: siren2,
        siret: None,
        denomination: "restaurant le gourmet",
        tokens: &["restaurant", "gourmet"],
        code_commune: None,
        code_postal: None,
        code_naf: None,
        etat_administratif: EtatAdministratif::Cesse,
        dirigeants: &[],
    };

    let matcher = EntityMatcher::new();
    let s_act = matcher.score_candidate(&search, &cand_active);
    let s_ces = matcher.score_candidate(&search, &cand_ceased);

    assert!(s_act.score() > s_ces.score());
    assert!(s_ces.sub_scores.bonus_actif < 0.0);
}

#[test]
fn test_ground_truth_edf_acronym_with_connector_word() {
    let normalizer = Normalizer::new();
    let query_norm = normalizer.normalize_query("EDF Renouvelables");
    let search = SearchQuery {
        query: &query_norm,
        location: None,
        naf: None,
        dirigeant: None,
        strict_location: false,
    };

    let siren = Siren::parse("732829320").expect("siren");
    let candidate = MatchCandidate {
        siren,
        siret: None,
        denomination: "electricite france renouvelables",
        tokens: &["electricite", "france", "renouvelables"],
        code_commune: None,
        code_postal: None,
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: &[],
    };

    let matcher = EntityMatcher::new();
    let scored = matcher.score_candidate(&search, &candidate);

    assert!(
        scored.score() > 0.85,
        "Acronym EDF should match Electricite de France with connector word tolerance, got {}",
        scored.score()
    );
}

#[test]
fn test_ground_truth_corse_2a_2b_and_postal_20_matching() {
    use sirene_core::{CodeCommune, CodePostal};
    use sirene_matcher::{score_localisation, LocationQuery};

    let com_ajaccio = CodeCommune::parse("2A004").expect("2A commune");
    let cp_ajaccio = CodePostal::parse("20000").expect("20 postal");
    let com_bastia = CodeCommune::parse("2B033").expect("2B commune");
    let cp_bastia = CodePostal::parse("20200").expect("20 postal bastia");

    // Query by department 20 matches candidate in 2A and 2B
    let q_dep20 = LocationQuery::Departement("20".to_string());
    assert_eq!(
        score_localisation(&q_dep20, Some(&com_ajaccio), Some(&cp_ajaccio)),
        0.5
    );
    assert_eq!(
        score_localisation(&q_dep20, Some(&com_bastia), Some(&cp_bastia)),
        0.5
    );

    // Query by department 2A matches postal 20000
    let q_dep2a = LocationQuery::Departement("2A".to_string());
    assert_eq!(
        score_localisation(&q_dep2a, Some(&com_ajaccio), Some(&cp_ajaccio)),
        0.5
    );

    // Query by Ajaccio commune (2A004) matches postal 20000 department
    let q_com2a = LocationQuery::Commune(com_ajaccio);
    assert_eq!(score_localisation(&q_com2a, None, Some(&cp_ajaccio)), 0.5);
}

#[test]
fn test_similarity_edge_cases() {
    use sirene_core::CodeNaf;
    use sirene_matcher::qgram::unpack_trigram;
    use sirene_matcher::{
        are_departments_matching, bonus_actif, damerau_levenshtein, damerau_levenshtein_similarity,
        jaccard_similarity, jaro_similarity, jaro_winkler_similarity, pack_trigram, score_activite,
        score_dirigeants, trigrams,
    };

    // Damerau
    assert_eq!(damerau_levenshtein("", ""), 0);
    assert_eq!(damerau_levenshtein("a", ""), 1);
    assert_eq!(damerau_levenshtein("", "abc"), 3);
    assert_eq!(damerau_levenshtein("abc", "abc"), 0);
    assert_eq!(damerau_levenshtein("ab", "ba"), 1); // transposition
    assert_eq!(damerau_levenshtein("ca", "abc"), 3);
    assert_eq!(damerau_levenshtein_similarity("", ""), 1.0);
    assert_eq!(damerau_levenshtein_similarity("a", ""), 0.0);
    assert!(damerau_levenshtein_similarity("dupont", "dupan") > 0.6);

    // Jaro-Winkler
    assert_eq!(jaro_similarity("", ""), 1.0);
    assert_eq!(jaro_similarity("a", ""), 0.0);
    assert_eq!(jaro_similarity("", "b"), 0.0);
    assert_eq!(jaro_winkler_similarity("", ""), 1.0);
    assert_eq!(jaro_winkler_similarity("a", ""), 0.0);
    assert_eq!(jaro_winkler_similarity("same", "same"), 1.0);
    assert_eq!(jaro_winkler_similarity("abc", "xyz"), 0.0);

    // Q-gram
    assert_eq!(jaccard_similarity("", ""), 1.0);
    assert_eq!(jaccard_similarity("a", ""), 0.0);
    assert_eq!(jaccard_similarity("test", "test"), 1.0);
    assert_eq!(jaccard_similarity("abc", "xyz"), 0.0);
    let tri = trigrams("abc");
    assert!(!tri.is_empty());
    let packed = pack_trigram(b'a', b'b', b'c');
    assert_eq!(unpack_trigram(packed), [b'a', b'b', b'c']);

    // NAF scoring
    let naf1 = CodeNaf::parse("62.01Z").unwrap();
    let naf_same_classe = CodeNaf::parse("62.01A").unwrap();
    let naf_same_groupe = CodeNaf::parse("62.02A").unwrap();
    let naf_same_div = CodeNaf::parse("62.20Z").unwrap();
    let naf_diff = CodeNaf::parse("47.11D").unwrap();

    assert_eq!(score_activite(&naf1, Some(&naf1)), 1.0);
    assert_eq!(score_activite(&naf1, Some(&naf_same_classe)), 0.6);
    assert_eq!(score_activite(&naf1, Some(&naf_same_groupe)), 0.6);
    assert_eq!(score_activite(&naf1, Some(&naf_same_div)), 0.6);
    assert_eq!(score_activite(&naf1, Some(&naf_diff)), 0.0);
    assert_eq!(score_activite(&naf1, None), 0.0);

    // Dirigeants scoring
    let dirs: &[&str] = &["Thomas Durand", "Jacques Dupont"];
    assert_eq!(score_dirigeants("Thomas Durand", dirs), 1.0);
    assert!(score_dirigeants("Thomas", dirs) > 0.6);
    assert!(score_dirigeants("Inconnu", dirs) < 0.5);
    assert_eq!(score_dirigeants("", dirs), 0.0);
    assert_eq!(score_dirigeants("Thomas", &[]), 0.0);

    // Departments matching
    assert!(are_departments_matching("75", "75"));
    assert!(are_departments_matching("20", "2A"));
    assert!(are_departments_matching("2B", "20"));
    assert!(!are_departments_matching("75", "33"));

    // Bonus actif
    assert_eq!(bonus_actif(EtatAdministratif::Actif, 0.05, 0.15), 0.05);
    assert_eq!(bonus_actif(EtatAdministratif::Cesse, 0.05, 0.15), -0.15);

    // Long string fallbacks (> 64 characters)
    let s_long1 = "abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz0123456789extra1";
    let s_long2 = "abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz0123456789extra2";
    let dl_long = damerau_levenshtein(s_long1, s_long2);
    assert_eq!(dl_long, 1);
    let dl_long_sim = damerau_levenshtein_similarity(s_long1, s_long2);
    assert!(dl_long_sim > 0.95);

    let jaro_long = jaro_similarity(s_long1, s_long2);
    assert!(jaro_long > 0.95);
    let jw_long = jaro_winkler_similarity(s_long1, s_long2);
    assert!(jw_long > 0.95);

    let s_all_a = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let s_all_b = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    assert_eq!(jaro_similarity(s_all_a, s_all_b), 0.0);
}
