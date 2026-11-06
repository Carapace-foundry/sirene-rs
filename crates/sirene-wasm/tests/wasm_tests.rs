use sirene_core::{CodeCommune, CodeNaf, CodePostal, EtatAdministratif, Siren, Siret};
use sirene_index::{IndexBuilder, IndexDocument};
use sirene_wasm::types::{WasmPivot, WasmSearchOptions};
use sirene_wasm::{
    extract_pivot_native, format_siren, format_siret, is_la_poste_siren, jaro_winkler_similarity,
    normalize_query, score_denominations, validate_siren, validate_siret, SireneWasmEngine,
};

#[test]
fn test_standalone_wasm_utilities() {
    // 1. SIREN / SIRET validation
    assert!(validate_siren("732829320"));
    assert!(!validate_siren("732829321"));
    assert!(!validate_siren("invalid"));

    assert!(validate_siret("73282932000074"));
    // La Poste official derogation (sum % 5 == 0)
    assert!(validate_siret("35600000038497"));
    assert!(!validate_siret("73282932000075"));

    // 2. Formatting & La Poste detection
    assert_eq!(format_siren("732829320"), Some("732 829 320".to_string()));
    assert_eq!(
        format_siret("73282932000074"),
        Some("732 829 320 00074".to_string())
    );
    assert!(is_la_poste_siren("356000000"));
    assert!(!is_la_poste_siren("732829320"));

    // 3. Query normalization (legal noise stripping)
    let norm = normalize_query("Établissements Dupont Plomberie SASU");
    assert_eq!(norm, "dupont plomberie");

    // 3. String similarities
    let jw = jaro_winkler_similarity("Boucherie Dupont", "Boucherie Dupan");
    assert!(jw > 0.90);

    let denom_score = score_denominations("Dupont Plomberie SASU", "Plomberie Dupont");
    assert_eq!(denom_score, 1.0);

    // 4. Extract pivot
    let pivot_siren = extract_pivot_native("Recherche entreprise 732829320 sur Paris");
    assert_eq!(pivot_siren, Some(WasmPivot::Siren("732829320".to_string())));

    let pivot_siret = extract_pivot_native("Etablissement 73282932000074");
    assert_eq!(
        pivot_siret,
        Some(WasmPivot::Siret("73282932000074".to_string()))
    );

    let pivot_none = extract_pivot_native("Sans pivot numerique");
    assert_eq!(pivot_none, None);
}

#[test]
fn test_sirene_wasm_engine_search_and_lookup() {
    let mut builder = IndexBuilder::new();

    let siren1 = Siren::parse("732829320").expect("siren1");
    let siret1 = Siret::parse("73282932000074").expect("siret1");
    let com1 = CodeCommune::parse("75101").expect("com1");
    let cp1 = CodePostal::parse("75001").expect("cp1");
    let naf1 = CodeNaf::parse("47.22Z").expect("naf1");

    builder.add_document(IndexDocument {
        siren: siren1,
        siret: Some(siret1),
        denomination: "Boucherie Dupont".to_string(),
        code_commune: Some(com1),
        code_postal: Some(cp1),
        code_naf: Some(naf1),
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: vec!["Jean Dupont".to_string()],
    });

    let siren2 = Siren::parse("443061841").expect("siren2");
    let com2 = CodeCommune::parse("33063").expect("com2");
    let cp2 = CodePostal::parse("33000").expect("cp2");

    builder.add_document(IndexDocument {
        siren: siren2,
        siret: None,
        denomination: "Boulangerie Martin".to_string(),
        code_commune: Some(com2),
        code_postal: Some(cp2),
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: vec![],
    });

    let index_bytes = builder.build().expect("build index");
    let engine = SireneWasmEngine::load_from_slice(&index_bytes).expect("load wasm engine");

    assert_eq!(engine.num_documents(), 2);

    // Exact SIREN lookup
    let doc_siren = engine.find_by_siren_native(siren1).expect("find by siren");
    assert_eq!(doc_siren.siren, siren1);
    assert_eq!(doc_siren.denomination, "boucherie dupont");

    // Exact SIRET lookup
    let doc_siret = engine.find_by_siret_native(siret1).expect("find by siret");
    assert_eq!(doc_siret.siret, Some(siret1));

    // Non-existent SIREN lookup
    let siren_missing = Siren::parse("123456782").expect("missing siren");
    let doc_none = engine.find_by_siren_native(siren_missing);
    assert!(doc_none.is_none());

    // Get document by ID
    let doc0 = engine.get_document_native(0).expect("doc 0");
    assert_eq!(doc0.siren, siren1);

    let doc_out_of_bounds = engine.get_document_native(99);
    assert!(doc_out_of_bounds.is_none());

    // Fuzzy search
    let default_opts = WasmSearchOptions::default();
    let res = engine.search_native("Boucherie Dupan", &default_opts);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].siren, siren1);
    assert!(res[0].score > 0.85);

    // Fuzzy search with structured options
    let opts = WasmSearchOptions {
        location: Some("33000".to_string()),
        naf: None,
        dirigeant: None,
        top_k: Some(5),
        strict_location: Some(false),
    };
    let res_geo = engine.search_native("Boulangerie Martin", &opts);
    assert_eq!(res_geo.len(), 1);
    assert_eq!(res_geo[0].siren, siren2);

    let opts_full = WasmSearchOptions {
        location: Some("75001".to_string()),
        naf: Some("47.22Z".to_string()),
        dirigeant: Some("Jean Dupont".to_string()),
        top_k: Some(3),
        strict_location: Some(true),
    };
    let res_full = engine.search_native("Boucherie Dupont", &opts_full);
    assert_eq!(res_full.len(), 1);
    assert_eq!(res_full[0].siren, siren1);

    // Serde roundtrips
    let json_opts = serde_json::to_string(&opts_full).expect("serialize opts");
    let deser_opts: WasmSearchOptions = serde_json::from_str(&json_opts).expect("deserialize opts");
    assert_eq!(deser_opts.location, Some("75001".to_string()));
    assert_eq!(deser_opts.naf, Some("47.22Z".to_string()));

    let pivot = WasmPivot::Siren("732829320".to_string());
    let json_piv = serde_json::to_string(&pivot).expect("serialize pivot");
    let deser_piv: WasmPivot = serde_json::from_str(&json_piv).expect("deserialize pivot");
    assert_eq!(deser_piv, pivot);

    // Test loading
    assert!(SireneWasmEngine::load_from_slice(&index_bytes).is_ok());
    assert!(SireneWasmEngine::load_from_slice(b"invalid_bytes").is_err());
    assert_eq!(engine.num_documents(), 2);
}
