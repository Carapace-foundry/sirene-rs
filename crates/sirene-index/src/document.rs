use sirene_core::{CodeCommune, CodeNaf, CodePostal, EtatAdministratif, Siren, Siret};
use sirene_matcher::SubScores;

/// An unindexed enterprise document submitted to `IndexBuilder`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IndexDocument {
    pub siren: Siren,
    pub siret: Option<Siret>,
    pub denomination: String,
    pub code_commune: Option<CodeCommune>,
    pub code_postal: Option<CodePostal>,
    pub code_naf: Option<CodeNaf>,
    pub etat_administratif: EtatAdministratif,
    pub dirigeants: Vec<String>,
}

/// Zero-copy view of an indexed enterprise document directly referencing index storage slices.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IndexedDocumentView<'a> {
    pub doc_id: u32,
    pub siren: Siren,
    pub siret: Option<Siret>,
    pub denomination: &'a str,
    pub code_commune: Option<CodeCommune>,
    pub code_postal: Option<CodePostal>,
    pub code_naf: Option<CodeNaf>,
    pub etat_administratif: EtatAdministratif,
    pub dirigeants: Vec<&'a str>,
}

/// An evaluated search result produced by `SireneIndex::search`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SearchResult {
    pub doc_id: u32,
    pub siren: Siren,
    pub siret: Option<Siret>,
    pub denomination: String,
    pub score: f32,
    pub sub_scores: SubScores,
    pub is_exact_pivot: bool,
}
