use byteorder::ByteOrder;
use sirene_core::{CodeCommune, CodeNaf, CodePostal, EtatAdministratif, Siren, Siret};
use sirene_index::{IndexBuilder, IndexDocument, SireneIndex};
use sirene_matcher::{LocationQuery, SearchQuery};
use sirene_normalize::Normalizer;

#[test]
fn test_end_to_end_indexing_and_search() {
    let mut builder = IndexBuilder::new();

    // 1. Boucherie Dupont - Paris
    let siren1 = Siren::parse("732829320").expect("siren1");
    let com1 = CodeCommune::parse("75101").expect("com1");
    let cp1 = CodePostal::parse("75001").expect("cp1");
    let naf1 = CodeNaf::parse("47.22Z").expect("naf1");
    builder.add_document(IndexDocument {
        siren: siren1,
        siret: None,
        denomination: "Boucherie Dupont".to_string(),
        code_commune: Some(com1),
        code_postal: Some(cp1),
        code_naf: Some(naf1),
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: vec!["Jean Dupont".to_string()],
    });

    // 2. Boulangerie Martin - Bordeaux (33000)
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

    // 3. Boulangerie Martin - Lille (59000)
    let siren3 = Siren::parse("356000000").expect("siren3");
    let com3 = CodeCommune::parse("59350").expect("com3");
    let cp3 = CodePostal::parse("59000").expect("cp3");
    builder.add_document(IndexDocument {
        siren: siren3,
        siret: None,
        denomination: "Boulangerie Martin".to_string(),
        code_commune: Some(com3),
        code_postal: Some(cp3),
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: vec![],
    });

    // 4. Dupont Plomberie - Lyon
    let siren4 = Siren::parse("552120222").expect("siren4");
    let com4 = CodeCommune::parse("69381").expect("com4");
    let cp4 = CodePostal::parse("69001").expect("cp4");
    builder.add_document(IndexDocument {
        siren: siren4,
        siret: None,
        denomination: "Dupont Plomberie".to_string(),
        code_commune: Some(com4),
        code_postal: Some(cp4),
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: vec![],
    });

    // Build index in memory
    let index_bytes = builder.build().expect("build index");
    let index = SireneIndex::from_vec(index_bytes).expect("load index");
    assert_eq!(index.num_documents(), 4);

    let normalizer = Normalizer::new();

    // Test A: Exact SIREN lookup
    let found_doc_id = index.find_by_siren(siren1);
    assert_eq!(found_doc_id, Some(0));
    let doc = index.get_document(0).expect("doc 0");
    assert_eq!(doc.siren, siren1);
    assert_eq!(doc.denomination, "boucherie dupont");

    // Test B: Typo query "Boucherie Dupan"
    let q_typo = normalizer.normalize_query("Boucherie Dupan");
    let search_typo = SearchQuery {
        query: &q_typo,
        location: None,
        naf: None,
        dirigeant: None,
        strict_location: false,
    };
    let res_typo = index.search(&search_typo, 5);
    assert!(!res_typo.is_empty());
    assert_eq!(res_typo[0].siren, siren1);
    assert!(res_typo[0].score > 0.85);

    // Test C: Legal noise query "Dupont Plomberie SASU"
    let q_legal = normalizer.normalize_query("Dupont Plomberie SASU");
    let search_legal = SearchQuery {
        query: &q_legal,
        location: None,
        naf: None,
        dirigeant: None,
        strict_location: false,
    };
    let res_legal = index.search(&search_legal, 5);
    assert!(!res_legal.is_empty());
    assert_eq!(res_legal[0].siren, siren4);
    assert_eq!(res_legal[0].score, 1.0);

    // Test D: Geographic Disambiguation: "Boulangerie Martin" + 33000 (Bordeaux)
    let q_martin = normalizer.normalize_query("Boulangerie Martin");
    let loc_bordeaux = LocationQuery::parse("33000");
    let search_geo = SearchQuery {
        query: &q_martin,
        location: loc_bordeaux,
        naf: None,
        dirigeant: None,
        strict_location: false,
    };
    let res_geo = index.search(&search_geo, 5);
    assert!(res_geo.len() >= 2);
    // Bordeaux MUST be #1
    assert_eq!(res_geo[0].siren, siren2);
    // Lille MUST be ranked after Bordeaux
    assert_eq!(res_geo[1].siren, siren3);
    assert!(res_geo[0].score > res_geo[1].score);

    // Test D2: Strict geographic rejection
    let search_geo_strict = SearchQuery {
        query: &q_martin,
        location: LocationQuery::parse("33000"),
        naf: None,
        dirigeant: None,
        strict_location: true,
    };
    let res_geo_strict = index.search(&search_geo_strict, 5);
    assert_eq!(res_geo_strict.len(), 1);
    assert_eq!(res_geo_strict[0].siren, siren2);

    // Test E: Exact SIREN query shortcut
    let q_pivot = normalizer.normalize_query("732829320");
    let search_pivot = SearchQuery {
        query: &q_pivot,
        location: None,
        naf: None,
        dirigeant: None,
        strict_location: false,
    };
    let res_pivot = index.search(&search_pivot, 5);
    assert_eq!(res_pivot.len(), 1);
    assert!(res_pivot[0].is_exact_pivot);
    assert_eq!(res_pivot[0].siren, siren1);
    assert_eq!(res_pivot[0].score, 1.0);
}

#[test]
fn test_file_persistence_and_mmap() {
    let mut builder = IndexBuilder::new();
    let siren = Siren::parse("732829320").expect("siren");
    let com = CodeCommune::parse("75101").expect("com");

    builder.add_document(IndexDocument {
        siren,
        siret: None,
        denomination: "Acme Corporation".to_string(),
        code_commune: Some(com),
        code_postal: None,
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: vec![],
    });

    let temp_file = std::env::temp_dir().join("test_sirene_index.bin");
    builder.write_to_file(&temp_file).expect("write file");

    #[cfg(not(target_arch = "wasm32"))]
    {
        let index = SireneIndex::open_file(&temp_file).expect("open file via mmap");
        assert_eq!(index.num_documents(), 1);
        let doc = index.get_document(0).expect("doc 0");
        assert_eq!(doc.siren, siren);
        assert_eq!(doc.denomination, "acme corporation");
    }

    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_corrupted_header_detection_and_safe_rejection() {
    let mut builder = IndexBuilder::new();
    let siren = Siren::parse("732829320").expect("siren");
    builder.add_document(IndexDocument {
        siren,
        siret: None,
        denomination: "Test Entity".to_string(),
        code_commune: None,
        code_postal: None,
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: vec![],
    });

    let valid_bytes = builder.build().expect("build index");

    // 1. Invalid Magic
    let mut bad_magic = valid_bytes.clone();
    bad_magic[0..8].copy_from_slice(b"BADMAGIC");
    assert!(matches!(
        SireneIndex::from_vec(bad_magic),
        Err(sirene_index::IndexError::InvalidMagic { .. })
    ));

    // 2. Truncated file
    let truncated = valid_bytes[..50].to_vec();
    assert!(SireneIndex::from_vec(truncated).is_err());

    // 3. Corrupted section out of bounds (modify siren_table offset in header: offset 16 in header)
    let mut oob_section = valid_bytes.clone();
    // Offset for siren_table is at byte 16 (after 8 bytes magic + 4 version + 4 num_docs)
    // Write an out of bounds offset u64: 99999999
    byteorder::LittleEndian::write_u64(&mut oob_section[16..24], 99_999_999);
    assert!(matches!(
        SireneIndex::from_vec(oob_section),
        Err(sirene_index::IndexError::CorruptedIndex { .. })
    ));

    // 4. Mismatched table size vs num_docs (siren_table len at byte 24..32)
    let mut bad_table_len = valid_bytes.clone();
    byteorder::LittleEndian::write_u64(&mut bad_table_len[24..32], 50); // expected 9 bytes for 1 doc
    assert!(matches!(
        SireneIndex::from_vec(bad_table_len),
        Err(sirene_index::IndexError::CorruptedIndex { .. })
    ));
}

#[test]
fn test_corrupted_name_offset_safe_rejection() {
    let mut builder = IndexBuilder::new();
    let siren = Siren::parse("732829320").expect("siren");
    builder.add_document(IndexDocument {
        siren,
        siret: None,
        denomination: "Safe Bounds Entity".to_string(),
        code_commune: None,
        code_postal: None,
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: vec![],
    });

    let mut bytes = builder.build().expect("build index");
    let header = sirene_index::IndexHeader::read_from(&mut &bytes[..]).expect("read header");

    // Corrupt name_offsets for doc 0: point start offset way beyond names_blob len
    let no_offset = header.name_offsets.0 as usize;
    byteorder::LittleEndian::write_u32(&mut bytes[no_offset..no_offset + 4], 999_999);

    let index = SireneIndex::from_vec(bytes).expect("index loads because header passed");
    // get_document must safely return None without panicking
    let doc = index.get_document(0);
    assert_eq!(doc, None);
}

#[test]
fn test_secondary_siret_resolution_and_search() {
    let mut builder = IndexBuilder::new();
    let siren = Siren::parse("732829320").expect("valid siren");
    let siege_siret = Siret::parse("73282932000074").expect("siege siret");
    let secondary_siret = Siret::parse("73282932000082").expect("secondary siret");

    builder.add_document(IndexDocument {
        siren,
        siret: Some(siege_siret),
        denomination: "Entreprise Multi Etablissements".to_string(),
        code_commune: None,
        code_postal: None,
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: vec![],
    });

    let bytes = builder.build().expect("build index");
    let index = SireneIndex::from_vec(bytes).expect("load index");

    // 1. Exact match via headquarter SIRET
    let found_siege = index.find_by_siret(siege_siret);
    assert_eq!(found_siege, Some(0));

    // 2. Exact match via secondary establishment SIRET
    let found_secondary = index.find_by_siret(secondary_siret);
    assert_eq!(found_secondary, Some(0));

    // 3. Search query with secondary SIRET string
    let normalizer = Normalizer::new();
    let norm_q = normalizer.normalize_query("73282932000082");
    let search_q = SearchQuery {
        query: &norm_q,
        location: None,
        naf: None,
        dirigeant: None,
        strict_location: false,
    };
    let results = index.search(&search_q, 5);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].siren, siren);
    assert!(results[0].is_exact_pivot);
    assert_eq!(results[0].score, 1.0);
}

#[test]
fn test_corsica_department_filtering_union() {
    let mut builder = IndexBuilder::new();

    // 1. Entity in Bastia (2B / Haute-Corse)
    let siren1 = Siren::parse("732829320").expect("siren1");
    let com1 = CodeCommune::parse("2B033").expect("bastia com");
    let cp1 = CodePostal::parse("20200").expect("bastia cp");
    builder.add_document(IndexDocument {
        siren: siren1,
        siret: None,
        denomination: "Bastia Commerce".to_string(),
        code_commune: Some(com1),
        code_postal: Some(cp1),
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: vec![],
    });

    // 2. Entity in Ajaccio (2A / Corse-du-Sud)
    let siren2 = Siren::parse("443061841").expect("siren2");
    let com2 = CodeCommune::parse("2A004").expect("ajaccio com");
    let cp2 = CodePostal::parse("20000").expect("ajaccio cp");
    builder.add_document(IndexDocument {
        siren: siren2,
        siret: None,
        denomination: "Ajaccio Restauration".to_string(),
        code_commune: Some(com2),
        code_postal: Some(cp2),
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: vec![],
    });

    // 3. Entity in Paris (75)
    let siren3 = Siren::parse("356000000").expect("siren3");
    let com3 = CodeCommune::parse("75101").expect("paris com");
    let cp3 = CodePostal::parse("75001").expect("paris cp");
    builder.add_document(IndexDocument {
        siren: siren3,
        siret: None,
        denomination: "Paris Services".to_string(),
        code_commune: Some(com3),
        code_postal: Some(cp3),
        code_naf: None,
        etat_administratif: EtatAdministratif::Actif,
        dirigeants: vec![],
    });

    let bytes = builder.build().expect("build index");
    let index = SireneIndex::from_vec(bytes).expect("load index");
    let filters = index.filters();

    // Query with department "20" (postal Corsica) should match both doc 0 (2B) and doc 1 (2A)
    let res_20 = filters.filter(false, Some("20"), None).expect("filter 20");
    assert!(res_20.contains(0));
    assert!(res_20.contains(1));
    assert!(!res_20.contains(2));

    // Query with department "2B" should match doc 0 (2B)
    let res_2b = filters.filter(false, Some("2B"), None).expect("filter 2B");
    assert!(res_2b.contains(0));
    assert!(!res_2b.contains(2));

    // Query with department "2A" should match doc 1 (2A)
    let res_2a = filters.filter(false, Some("2A"), None).expect("filter 2A");
    assert!(res_2a.contains(1));
    assert!(!res_2a.contains(2));
}

#[test]
fn test_sirene_index_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<SireneIndex>();
}

#[test]
fn test_storage_slice_and_header_error_branches() {
    use sirene_index::storage::{IndexHeader, IndexStorage, StorageSlice, CURRENT_VERSION};
    use sirene_index::IndexError;
    use std::sync::Arc;

    // 1. Storage slice out of bounds
    let storage = Arc::new(IndexStorage::Owned(vec![0u8; 100]));
    let err_slice = StorageSlice::new(Arc::clone(&storage), 50, 60);
    assert!(err_slice.is_err());
    let err_overflow = StorageSlice::new(Arc::clone(&storage), usize::MAX, 10);
    assert!(err_overflow.is_err());
    let ok_slice = StorageSlice::new(storage, 10, 20).unwrap();
    assert_eq!(ok_slice.as_ref().len(), 20);

    // 2. IndexHeader validation error branches
    let bad_header_version = IndexHeader {
        version: CURRENT_VERSION + 1,
        num_docs: 0,
        siren_table: (0, 0),
        siret_table: (0, 0),
        metadata_table: (0, 0),
        names_blob: (0, 0),
        name_offsets: (0, 0),
        fst_siren: (0, 0),
        fst_terms: (0, 0),
        term_postings: (0, 0),
        trigram_data: (0, 0),
        filter_data: (0, 0),
        dirigeants_data: (0, 0),
    };
    let mut buf = Vec::new();
    bad_header_version.write_to(&mut buf).unwrap();
    let read_res = IndexHeader::read_from(&mut &buf[..]);
    assert!(matches!(
        read_res,
        Err(IndexError::UnsupportedVersion { .. })
    ));

    // 3. IndexError Display tests
    let e_magic = IndexError::InvalidMagic {
        expected: *b"SIRENE01",
        found: *b"BADMAGIC",
    };
    assert!(format!("{}", e_magic).contains("Invalid index header magic"));

    let e_ver = IndexError::UnsupportedVersion { version: 99 };
    assert!(format!("{}", e_ver).contains("99"));

    let e_corr = IndexError::CorruptedIndex {
        message: "corrupted block".to_string(),
    };
    assert!(format!("{}", e_corr).contains("corrupted block"));

    let e_doc = IndexError::DocumentNotFound { doc_id: 123 };
    assert!(format!("{}", e_doc).contains("123"));

    let e_io = IndexError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file not found",
    ));
    assert!(format!("{}", e_io).contains("file not found"));
}
