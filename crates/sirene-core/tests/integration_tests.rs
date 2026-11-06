use sirene_core::{
    extract_pivot, CategorieJuridique, CodeCommune, CodeNaf, CodePostal, EtatAdministratif,
    IdentifiantPivot, Nic, Siren, Siret,
};

#[test]
fn test_siren_parsing_and_formatting() {
    let raw = "732 829 320";
    let siren = Siren::parse(raw).expect("Valid SIREN with spaces should parse");
    assert_eq!(siren.as_str(), "732829320");
    assert_eq!(siren.formatted(), "732 829 320");
    assert!(!siren.is_la_poste());

    let dotted = Siren::parse("732.829.320").expect("Dotted SIREN should parse");
    assert_eq!(dotted, siren);

    let dashed = Siren::parse("732-829-320").expect("Dashed SIREN should parse");
    assert_eq!(dashed, siren);

    let invalid_luhn = Siren::parse("732829321");
    assert!(invalid_luhn.is_err());

    let invalid_len = Siren::parse("73282932");
    assert!(invalid_len.is_err());
}

#[test]
fn test_nic_parsing() {
    let nic = Nic::parse("00074").expect("Valid NIC");
    assert_eq!(nic.as_str(), "00074");
    assert_eq!(
        Nic::parse("0007"),
        Err(sirene_core::NicError::InvalidLength { actual: 4 })
    );
}

#[test]
fn test_siret_and_nic_extraction() {
    let siret_str = "732 829 320 00074";
    let siret = Siret::parse(siret_str).expect("Valid SIRET should parse");

    assert_eq!(siret.as_str(), "73282932000074");
    assert_eq!(siret.formatted(), "732 829 320 00074");

    let siren = siret.siren();
    assert_eq!(siren.as_str(), "732829320");

    let nic = siret.nic();
    assert_eq!(nic.as_str(), "00074");

    let reconstructed = Siret::from_siren_and_nic(siren, nic).expect("Valid SIRET components");
    assert_eq!(reconstructed, siret);
}

#[test]
fn test_la_poste_siret_derogation() {
    // Siège (Standard Luhn valid)
    let siege = Siret::parse("356 000 000 00048").expect("La Poste siege is valid");
    assert!(siege.is_la_poste());

    // Établissement secondaire 38497 (fails Luhn, passes sum % 5 == 0)
    let secondary = Siret::parse("356 000 000 38497").expect("La Poste derogation valid");
    assert!(secondary.is_la_poste());
    assert!(secondary.is_la_poste_derogation());

    // Invalid La Poste SIRET (fails both)
    let bad = Siret::parse("356 000 000 38498");
    assert!(bad.is_err());
}

#[test]
fn test_code_commune_and_postal() {
    // Mainland Paris 1er
    let com_paris = CodeCommune::parse("75101").expect("Valid commune");
    assert_eq!(com_paris.departement(), "75");
    assert!(!com_paris.is_corse());
    assert!(!com_paris.is_outre_mer());

    // Corsica 2A
    let com_ajaccio = CodeCommune::parse("2A004").expect("Valid Corsica commune");
    assert_eq!(com_ajaccio.departement(), "2A");
    assert!(com_ajaccio.is_corse());

    // Corsica 2B
    let com_bastia = CodeCommune::parse("2B033").expect("Valid Corsica commune");
    assert_eq!(com_bastia.departement(), "2B");
    assert!(com_bastia.is_corse());

    // Overseas Guadeloupe Basse-Terre
    let com_bt = CodeCommune::parse("97105").expect("Valid DOM commune");
    assert_eq!(com_bt.departement(), "971");
    assert!(com_bt.is_outre_mer());

    // Postal code distinct from commune
    let cp_ajaccio = CodePostal::parse("20000").expect("Valid postal code");
    assert_eq!(cp_ajaccio.departement(), "20");
}

#[test]
fn test_code_naf() {
    let naf_dotted = CodeNaf::parse("62.01Z").expect("Valid NAF dotted");
    let naf_raw = CodeNaf::parse("6201z").expect("Valid NAF raw lowercase");

    assert_eq!(naf_dotted, naf_raw);
    assert_eq!(naf_dotted.division(), "62");
    assert_eq!(naf_dotted.groupe(), "620");
    assert_eq!(naf_dotted.classe(), "6201");
    assert_eq!(naf_dotted.sous_classe(), "6201Z");
    assert_eq!(naf_dotted.formatted(), "62.01Z");
}

#[test]
fn test_categorie_juridique() {
    let cat = CategorieJuridique::parse("5710").expect("Valid legal category");
    assert_eq!(cat.niveau1(), "5");
    assert_eq!(cat.niveau2(), "57");
    assert_eq!(cat.niveau3(), "5710");
    assert!(cat.libelle_sommaire().is_some());
}

#[test]
fn test_etat_administratif() {
    let actif = EtatAdministratif::parse("A").expect("Actif");
    assert!(actif.is_actif());
    assert_eq!(actif.as_char(), 'A');

    let cessee = EtatAdministratif::parse("Cessée").expect("Cessée");
    assert!(cessee.is_cesse());
    assert_eq!(cessee.as_char(), 'C');
}

#[test]
fn test_records_and_display_name() {
    use sirene_core::{DirigeantRecord, EtablissementRecord, UniteLegaleRecord};

    let siren = Siren::parse("732829320").expect("siren");
    let siret = Siret::parse("73282932000074").expect("siret");
    let nic = Nic::parse("00074").expect("nic");
    let naf = CodeNaf::parse("62.01Z").expect("naf");
    let cat = CategorieJuridique::parse("5710").expect("cat");
    let com = CodeCommune::parse("75101").expect("com");
    let cp = CodePostal::parse("75001").expect("cp");

    let ul_corp = UniteLegaleRecord {
        siren,
        denomination: Some("Dupont SAS".to_string()),
        nom_patronymique: None,
        prenoms: None,
        sigle: Some("DS".to_string()),
        etat_administratif: EtatAdministratif::Actif,
        categorie_juridique: Some(cat),
        activite_principale: Some(naf),
        tranche_effectifs: Some("11".to_string()),
        date_creation: Some("2020-01-01".to_string()),
    };
    assert_eq!(ul_corp.display_name(), "Dupont SAS");

    let ul_pers = UniteLegaleRecord {
        siren,
        denomination: None,
        nom_patronymique: Some("Martin".to_string()),
        prenoms: Some("Jean Paul".to_string()),
        sigle: None,
        etat_administratif: EtatAdministratif::Actif,
        categorie_juridique: None,
        activite_principale: None,
        tranche_effectifs: None,
        date_creation: None,
    };
    assert_eq!(ul_pers.display_name(), "Martin Jean Paul");

    let ul_empty = UniteLegaleRecord {
        siren,
        denomination: None,
        nom_patronymique: None,
        prenoms: None,
        sigle: None,
        etat_administratif: EtatAdministratif::Cesse,
        categorie_juridique: None,
        activite_principale: None,
        tranche_effectifs: None,
        date_creation: None,
    };
    assert_eq!(ul_empty.display_name(), "");

    let etab = EtablissementRecord {
        siret,
        siren,
        nic,
        etat_administratif: EtatAdministratif::Actif,
        code_commune: Some(com),
        code_postal: Some(cp),
        enseigne: Some("Boucherie Centrale".to_string()),
        denomination_usuelle: Some("Chez Dupont".to_string()),
        activite_principale: Some(naf),
        is_siege: true,
    };
    assert_eq!(etab.siret, siret);
    assert!(etab.is_siege);

    let dir = DirigeantRecord {
        siren,
        nom: "Durand".to_string(),
        prenoms: Some("Jacques".to_string()),
        qualite: Some("President".to_string()),
        date_naissance: Some("1975-06-15".to_string()),
        is_personne_morale: false,
    };
    assert_eq!(dir.nom, "Durand");
    assert!(!dir.is_personne_morale);
}

#[test]
fn test_naf_hierarchy_and_sections() {
    use std::str::FromStr;

    // Test section mappings
    let naf_agri = CodeNaf::parse("01.11Z").unwrap();
    assert_eq!(naf_agri.section(), 'A');

    let naf_mine = CodeNaf::parse("05.10Z").unwrap();
    assert_eq!(naf_mine.section(), 'B');

    let naf_indus = CodeNaf::parse("10.71C").unwrap();
    assert_eq!(naf_indus.section(), 'C');

    let naf_elec = CodeNaf::parse("35.11Z").unwrap();
    assert_eq!(naf_elec.section(), 'D');

    let naf_eau = CodeNaf::parse("36.00Z").unwrap();
    assert_eq!(naf_eau.section(), 'E');

    let naf_btp = CodeNaf::parse("41.20A").unwrap();
    assert_eq!(naf_btp.section(), 'F');

    let naf_com = CodeNaf::parse("47.22Z").unwrap();
    assert_eq!(naf_com.section(), 'G');

    let naf_trans = CodeNaf::parse("49.41A").unwrap();
    assert_eq!(naf_trans.section(), 'H');

    let naf_hot = CodeNaf::parse("55.10Z").unwrap();
    assert_eq!(naf_hot.section(), 'I');

    let naf_info = CodeNaf::parse("62.01Z").unwrap();
    assert_eq!(naf_info.section(), 'J');
    assert_eq!(format!("{}", naf_info), "62.01Z");
    assert_eq!(naf_info.as_str(), "6201Z");
    assert_eq!(naf_info.as_bytes(), b"6201Z");

    let naf_fin = CodeNaf::parse("64.19Z").unwrap();
    assert_eq!(naf_fin.section(), 'K');

    let naf_immo = CodeNaf::parse("68.20A").unwrap();
    assert_eq!(naf_immo.section(), 'L');

    let naf_sci = CodeNaf::parse("69.10Z").unwrap();
    assert_eq!(naf_sci.section(), 'M');

    let naf_adm = CodeNaf::parse("78.10Z").unwrap();
    assert_eq!(naf_adm.section(), 'N');

    let naf_pub = CodeNaf::parse("84.11Z").unwrap();
    assert_eq!(naf_pub.section(), 'O');

    let naf_ens = CodeNaf::parse("85.10Z").unwrap();
    assert_eq!(naf_ens.section(), 'P');

    let naf_san = CodeNaf::parse("86.10Z").unwrap();
    assert_eq!(naf_san.section(), 'Q');

    let naf_art = CodeNaf::parse("90.01Z").unwrap();
    assert_eq!(naf_art.section(), 'R');

    let naf_ser = CodeNaf::parse("94.11Z").unwrap();
    assert_eq!(naf_ser.section(), 'S');

    let naf_men = CodeNaf::parse("97.00Z").unwrap();
    assert_eq!(naf_men.section(), 'T');

    let naf_extr = CodeNaf::parse("99.00Z").unwrap();
    assert_eq!(naf_extr.section(), 'U');

    // FromStr
    let parsed_from_str = CodeNaf::from_str("6201Z").unwrap();
    assert_eq!(parsed_from_str, naf_info);

    // new_unchecked
    let raw_naf = CodeNaf::new_unchecked(*b"6201Z");
    assert_eq!(raw_naf, naf_info);

    // Errors
    assert!(CodeNaf::parse("62").is_err());
    assert!(CodeNaf::parse("62011").is_err()); // digit instead of letter
    assert!(CodeNaf::parse("62A1Z").is_err()); // letter in digits
    assert!(CodeNaf::parse("620199").is_err());
}

#[test]
fn test_categorie_juridique_classification() {
    let cat_ei = CategorieJuridique::parse("1000").unwrap();
    assert!(cat_ei.is_personne_physique());
    assert!(!cat_ei.is_societe_commerciale());
    assert!(!cat_ei.is_personne_morale_droit_public());
    assert_eq!(format!("{}", cat_ei), "1000");
    assert_eq!(cat_ei.as_str(), "1000");

    let cat_sarl = CategorieJuridique::parse("5498").unwrap();
    assert!(!cat_sarl.is_personne_physique());
    assert!(cat_sarl.is_societe_commerciale());
    assert!(!cat_sarl.is_personne_morale_droit_public());

    let cat_pub = CategorieJuridique::parse("7120").unwrap();
    assert!(!cat_pub.is_personne_physique());
    assert!(!cat_pub.is_societe_commerciale());
    assert!(cat_pub.is_personne_morale_droit_public());

    let cat_raw = CategorieJuridique::new_unchecked(*b"5710");
    assert_eq!(cat_raw.as_bytes(), b"5710");

    assert!(CategorieJuridique::parse("571").is_err());
    assert!(CategorieJuridique::parse("57100").is_err());
    assert!(CategorieJuridique::parse("57A0").is_err());
}

#[test]
fn test_commune_and_postal_edge_cases() {
    let cp = CodePostal::parse("75001").unwrap();
    assert_eq!(format!("{}", cp), "75001");
    assert_eq!(cp.as_str(), "75001");
    assert_eq!(cp.as_bytes(), b"75001");
    assert_eq!(CodePostal::new_unchecked(*b"75001"), cp);
    assert!(CodePostal::parse("7500").is_err());
    assert!(CodePostal::parse("750011").is_err());
    assert!(CodePostal::parse("7500A").is_err());

    let com = CodeCommune::parse("75101").unwrap();
    assert_eq!(format!("{}", com), "75101");
    assert_eq!(com.as_str(), "75101");
    assert_eq!(com.as_bytes(), b"75101");
    assert_eq!(CodeCommune::new_unchecked(*b"75101"), com);
    assert!(CodeCommune::parse("7510").is_err());
    assert!(CodeCommune::parse("751011").is_err());
    assert!(CodeCommune::parse("7510!").is_err());

    let com_corse = CodeCommune::parse("2a004").unwrap();
    assert_eq!(com_corse.departement(), "2A");
    assert!(com_corse.is_corse());
    assert!(!com_corse.is_outre_mer());

    let cp_corse = CodePostal::parse("20167").unwrap();
    assert_eq!(cp_corse.departement(), "20");
}

#[test]
fn test_siren_and_siret_traits() {
    let s = Siren::parse_exact("732829320").unwrap();
    assert_eq!(s.as_bytes(), b"732829320");
    assert_eq!(format!("{}", s), "732829320");
    assert_eq!(Siren::new_unchecked(*b"732829320"), s);

    let srt = Siret::parse_exact("73282932000074").unwrap();
    assert_eq!(srt.as_bytes(), b"73282932000074");
    assert_eq!(format!("{}", srt), "73282932000074");
    assert_eq!(Siret::new_unchecked(*b"73282932000074"), srt);

    let nic = Nic::parse_exact("00074").unwrap();
    assert_eq!(nic.as_bytes(), b"00074");
    assert_eq!(format!("{}", nic), "00074");
    assert_eq!(Nic::new_unchecked(*b"00074"), nic);
}

#[test]
fn test_error_display_implementations() {
    use sirene_core::{
        CategorieJuridiqueError, CodeCommuneError, CodeNafError, CodePostalError,
        EtatAdministratifError, NicError, SirenError, SiretError,
    };

    let err_siren = SirenError::InvalidLength { actual: 8 };
    assert!(format!("{}", err_siren).contains("8"));
    let err_siren2 = SirenError::NonDigitCharacter {
        position: 2,
        ch: 'A',
    };
    assert!(format!("{}", err_siren2).contains('A'));
    let err_siren3 = SirenError::InvalidLuhnChecksum {
        siren: "732829321".to_string(),
    };
    assert!(format!("{}", err_siren3).contains("732829321"));

    let err_siret = SiretError::InvalidLength { actual: 12 };
    assert!(format!("{}", err_siret).contains("12"));
    let err_siret2 = SiretError::NonDigitCharacter {
        position: 4,
        ch: 'B',
    };
    assert!(format!("{}", err_siret2).contains('B'));
    let err_siret3 = SiretError::InvalidLuhnChecksum {
        siret: "73282932000075".to_string(),
    };
    assert!(format!("{}", err_siret3).contains("73282932000075"));
    let err_siret4 = SiretError::InvalidLaPosteChecksum {
        siret: "35600000038498".to_string(),
    };
    assert!(format!("{}", err_siret4).contains("35600000038498"));

    let err_com = CodeCommuneError::InvalidLength { actual: 3 };
    assert!(format!("{}", err_com).contains("3"));
    let err_com2 = CodeCommuneError::InvalidFormat {
        input: "7510!".to_string(),
    };
    assert!(format!("{}", err_com2).contains("7510!"));

    let err_cp = CodePostalError::InvalidLength { actual: 4 };
    assert!(format!("{}", err_cp).contains("4"));
    let err_cp2 = CodePostalError::NonDigitCharacter {
        position: 0,
        ch: 'X',
    };
    assert!(format!("{}", err_cp2).contains('X'));

    let err_naf = CodeNafError::InvalidFormat {
        input: "62".to_string(),
    };
    assert!(format!("{}", err_naf).contains("62"));

    let err_cat = CategorieJuridiqueError::InvalidLength { actual: 2 };
    assert!(format!("{}", err_cat).contains("2"));
    let err_cat2 = CategorieJuridiqueError::NonDigitCharacter {
        position: 1,
        ch: 'Z',
    };
    assert!(format!("{}", err_cat2).contains('Z'));

    let err_etat = EtatAdministratifError::InvalidCode {
        code: "XYZ".to_string(),
    };
    assert!(format!("{}", err_etat).contains("XYZ"));

    let err_nic = NicError::InvalidLength { actual: 3 };
    assert!(format!("{}", err_nic).contains("3"));
    let err_nic2 = NicError::NonDigitCharacter {
        position: 1,
        ch: '?',
    };
    assert!(format!("{}", err_nic2).contains('?'));
}

#[test]
fn test_serde_and_conversions() {
    use std::str::FromStr;

    let siren = Siren::parse("732829320").unwrap();
    let json_siren = serde_json::to_string(&siren).unwrap();
    assert_eq!(json_siren, "\"732829320\"");
    let de_siren: Siren = serde_json::from_str(&json_siren).unwrap();
    assert_eq!(de_siren, siren);

    let siret = Siret::parse("73282932000074").unwrap();
    let json_siret = serde_json::to_string(&siret).unwrap();
    assert_eq!(json_siret, "\"73282932000074\"");
    let de_siret: Siret = serde_json::from_str(&json_siret).unwrap();
    assert_eq!(de_siret, siret);

    let nic = Nic::parse("00074").unwrap();
    let json_nic = serde_json::to_string(&nic).unwrap();
    assert_eq!(json_nic, "\"00074\"");
    let de_nic: Nic = serde_json::from_str(&json_nic).unwrap();
    assert_eq!(de_nic, nic);

    let com = CodeCommune::parse("75101").unwrap();
    let json_com = serde_json::to_string(&com).unwrap();
    assert_eq!(json_com, "\"75101\"");
    let de_com: CodeCommune = serde_json::from_str(&json_com).unwrap();
    assert_eq!(de_com, com);

    let cp = CodePostal::parse("75001").unwrap();
    let json_cp = serde_json::to_string(&cp).unwrap();
    assert_eq!(json_cp, "\"75001\"");
    let de_cp: CodePostal = serde_json::from_str(&json_cp).unwrap();
    assert_eq!(de_cp, cp);

    let naf = CodeNaf::parse("62.01Z").unwrap();
    let json_naf = serde_json::to_string(&naf).unwrap();
    assert_eq!(json_naf, "\"62.01Z\"");
    let de_naf: CodeNaf = serde_json::from_str(&json_naf).unwrap();
    assert_eq!(de_naf, naf);

    let cat = CategorieJuridique::parse("5710").unwrap();
    let json_cat = serde_json::to_string(&cat).unwrap();
    assert_eq!(json_cat, "\"5710\"");
    let de_cat: CategorieJuridique = serde_json::from_str(&json_cat).unwrap();
    assert_eq!(de_cat, cat);

    let etat = EtatAdministratif::Actif;
    let json_etat = serde_json::to_string(&etat).unwrap();
    assert_eq!(json_etat, "\"A\"");
    let de_etat: EtatAdministratif = serde_json::from_str(&json_etat).unwrap();
    assert_eq!(de_etat, etat);

    // TryFrom and traits
    assert_eq!(Siren::try_from("732829320").unwrap(), siren);
    assert_eq!(Siren::try_from(b"732829320".as_slice()).unwrap(), siren);
    assert_eq!(Siren::try_from(*b"732829320").unwrap(), siren);
    assert_eq!(Siren::from_str("732829320").unwrap(), siren);
    assert!(Siren::try_from(b"7328293".as_slice()).is_err());
    assert_eq!(&*siren, "732829320");
    let s_str: &str = siren.as_ref();
    assert_eq!(s_str, "732829320");
    let b_slice: &[u8] = siren.as_ref();
    assert_eq!(b_slice, b"732829320");
    assert!(format!("{:?}", siren).contains("Siren"));

    assert_eq!(Siret::try_from("73282932000074").unwrap(), siret);
    assert_eq!(
        Siret::try_from(b"73282932000074".as_slice()).unwrap(),
        siret
    );
    assert_eq!(Siret::try_from(*b"73282932000074").unwrap(), siret);
    assert_eq!(Siret::from_str("73282932000074").unwrap(), siret);
    assert!(Siret::try_from(b"7328293".as_slice()).is_err());
    assert_eq!(&*siret, "73282932000074");
    let srt_str: &str = siret.as_ref();
    assert_eq!(srt_str, "73282932000074");
    let srt_b: &[u8] = siret.as_ref();
    assert_eq!(srt_b, b"73282932000074");
    assert!(format!("{:?}", siret).contains("Siret"));

    assert_eq!(Nic::try_from("00074").unwrap(), nic);
    assert_eq!(Nic::from_str("00074").unwrap(), nic);
    assert_eq!(&*nic, "00074");
    let nic_str: &str = nic.as_ref();
    assert_eq!(nic_str, "00074");
    let nic_b: &[u8] = nic.as_ref();
    assert_eq!(nic_b, b"00074");
    assert!(format!("{:?}", nic).contains("Nic"));

    assert_eq!(CodeCommune::try_from("75101").unwrap(), com);
    assert_eq!(CodeCommune::from_str("75101").unwrap(), com);
    assert_eq!(&*com, "75101");
    let com_str: &str = com.as_ref();
    assert_eq!(com_str, "75101");
    let com_b: &[u8] = com.as_ref();
    assert_eq!(com_b, b"75101");
    assert!(format!("{:?}", com).contains("CodeCommune"));

    assert_eq!(CodePostal::try_from("75001").unwrap(), cp);
    assert_eq!(CodePostal::from_str("75001").unwrap(), cp);
    assert_eq!(&*cp, "75001");
    let cp_str: &str = cp.as_ref();
    assert_eq!(cp_str, "75001");
    let cp_b: &[u8] = cp.as_ref();
    assert_eq!(cp_b, b"75001");
    assert!(format!("{:?}", cp).contains("CodePostal"));

    assert_eq!(CodeNaf::from_str("6201Z").unwrap(), naf);
    assert!(format!("{:?}", naf).contains("CodeNaf"));

    assert_eq!(CategorieJuridique::try_from("5710").unwrap(), cat);
    assert_eq!(CategorieJuridique::from_str("5710").unwrap(), cat);
    assert_eq!(&*cat, "5710");
    let cat_str: &str = cat.as_ref();
    assert_eq!(cat_str, "5710");
    let cat_b: &[u8] = cat.as_ref();
    assert_eq!(cat_b, b"5710");
    assert!(format!("{:?}", cat).contains("CategorieJuridique"));

    assert_eq!(
        EtatAdministratif::try_from('A').unwrap(),
        EtatAdministratif::Actif
    );
    assert_eq!(
        EtatAdministratif::try_from('C').unwrap(),
        EtatAdministratif::Cesse
    );
    assert!(EtatAdministratif::try_from('X').is_err());
    assert_eq!(
        EtatAdministratif::try_from(b'A').unwrap(),
        EtatAdministratif::Actif
    );
    assert_eq!(
        EtatAdministratif::try_from(b'C').unwrap(),
        EtatAdministratif::Cesse
    );
    assert!(EtatAdministratif::try_from(b'Z').is_err());
    assert_eq!(
        EtatAdministratif::from_str("A").unwrap(),
        EtatAdministratif::Actif
    );
    assert_eq!(format!("{:?}", EtatAdministratif::Actif), "Actif");
    assert_eq!(format!("{}", EtatAdministratif::Actif), "A");
    assert_eq!(format!("{}", EtatAdministratif::Cesse), "C");
}

#[test]
fn test_siren_siret_parsing_branches() {
    // Siren with more than 9 digits
    assert!(Siren::parse("7328293200").is_err());
    // Siren with non-digit character
    assert!(Siren::parse("732A29320").is_err());

    // Siret with more than 14 digits
    assert!(Siret::parse("732829320000740").is_err());
    // Siret with non-digit character
    assert!(Siret::parse("7328293200007A").is_err());
    // Siret with less than 14 digits
    assert!(Siret::parse("7328293200007").is_err());

    // Nic with non-digit
    assert!(Nic::parse("0007A").is_err());
    assert!(Nic::parse("000740").is_err());

    // CodeCommune with non-alphanumeric
    assert!(CodeCommune::parse("75 01").is_err());
    assert!(CodeCommune::parse("7501").is_err());
    assert!(CodeCommune::parse("750100").is_err());

    // CodePostal with non-digit
    assert!(CodePostal::parse("7500A").is_err());
    assert!(CodePostal::parse("7500").is_err());
    assert!(CodePostal::parse("750010").is_err());
}

#[test]
fn test_extract_pivot_shortcut() {
    let query1 = "732 829 320";
    let p1 = extract_pivot(query1);
    assert!(matches!(p1, Some(IdentifiantPivot::Siren(_))));
    let pivot1 = p1.unwrap();
    assert_eq!(pivot1.siren(), Siren::parse("732829320").unwrap());
    assert_eq!(pivot1.siret(), None);
    assert_eq!(pivot1.as_str(), "732829320");

    let query2 = "Recherche entreprise 73282932000074 à Paris";
    let p2 = extract_pivot(query2);
    assert!(matches!(p2, Some(IdentifiantPivot::Siret(_))));
    let pivot2 = p2.unwrap();
    assert_eq!(pivot2.siren(), Siren::parse("732829320").unwrap());
    assert_eq!(
        pivot2.siret(),
        Some(Siret::parse("73282932000074").unwrap())
    );
    assert_eq!(pivot2.as_str(), "73282932000074");

    let query3 = "Boucherie Dupan 75001";
    let p3 = extract_pivot(query3);
    assert_eq!(p3, None);

    assert_eq!(extract_pivot(""), None);
    assert_eq!(extract_pivot("   "), None);
    assert_eq!(extract_pivot("1234567890"), None);
    assert_eq!(extract_pivot("1234"), None);
}
