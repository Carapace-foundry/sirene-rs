use crate::categorie::CategorieJuridique;
use crate::commune::{CodeCommune, CodePostal};
use crate::naf::CodeNaf;
use crate::nic::Nic;
use crate::siren::Siren;
use crate::siret::Siret;
use crate::statut::EtatAdministratif;

/// Legal unit record (Unité Légale) from INSEE SIRENE V3 (`StockUniteLegale`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UniteLegaleRecord {
    pub siren: Siren,
    pub denomination: Option<String>,
    pub nom_patronymique: Option<String>,
    pub prenoms: Option<String>,
    pub sigle: Option<String>,
    pub etat_administratif: EtatAdministratif,
    pub categorie_juridique: Option<CategorieJuridique>,
    pub activite_principale: Option<CodeNaf>,
    pub tranche_effectifs: Option<String>,
    pub date_creation: Option<String>,
}

impl UniteLegaleRecord {
    /// Returns the primary corporate name or natural person full name.
    pub fn display_name(&self) -> String {
        if let Some(ref denom) = self.denomination {
            denom.clone()
        } else {
            let mut parts = Vec::with_capacity(2);
            if let Some(ref nom) = self.nom_patronymique {
                parts.push(nom.as_str());
            }
            if let Some(ref prenoms) = self.prenoms {
                parts.push(prenoms.as_str());
            }
            parts.join(" ")
        }
    }
}

/// Establishment record (Établissement) from INSEE SIRENE V3 (`StockEtablissement`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EtablissementRecord {
    pub siret: Siret,
    pub siren: Siren,
    pub nic: Nic,
    pub etat_administratif: EtatAdministratif,
    pub code_commune: Option<CodeCommune>,
    pub code_postal: Option<CodePostal>,
    pub enseigne: Option<String>,
    pub denomination_usuelle: Option<String>,
    pub activite_principale: Option<CodeNaf>,
    pub is_siege: bool,
}

/// Corporate leader / representative from the RNE (Registre National des Entreprises - INPI).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DirigeantRecord {
    pub siren: Siren,
    pub nom: String,
    pub prenoms: Option<String>,
    pub qualite: Option<String>,
    pub date_naissance: Option<String>,
    pub is_personne_morale: bool,
}
