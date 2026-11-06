//! French legal forms and institutional abbreviations dictionary.
//! Conforms to DINUM (Annuaire des Entreprises / api-recherche-entreprises) standards.

/// Returns `true` if the normalized lowercase token is a recognized French legal form.
///
/// These terms are considered "legal noise" in corporate name searches and are stripped
/// from distinctive matching tokens (e.g. `"Dupont Plomberie SASU"` -> `"Dupont Plomberie"`).
#[inline]
pub fn is_legal_form(token: &str) -> bool {
    matches!(
        token,
        "sarl"
            | "sarlu"
            | "sas"
            | "snc"
            | "sci"
            | "scpi"
            | "selarl"
            | "selarlu"
            | "selas"
            | "selasu"
            | "sel"
            | "sa"
            | "eurl"
            | "sasu"
            | "ei"
            | "gie"
            | "geie"
            | "association"
            | "asso"
            | "ste"
            | "stes"
            | "societe"
            | "societes"
            | "ets"
            | "etabl"
            | "etabls"
            | "etablissement"
            | "etablissements"
            | "eirl"
            | "scop"
            | "scic"
            | "sem"
            | "semop"
            | "scm"
            | "selafa"
            | "earl"
            | "gaec"
            | "scea"
            | "scp"
            | "spfpl"
            | "sca"
            | "scs"
            | "sica"
            | "gcs"
            | "cooperative"
            | "mutuelle"
            | "fondation"
            | "syndicat"
            | "cie"
            | "cies"
            | "compagnie"
            | "compagnies"
            | "epic"
            | "epa"
            | "gip"
    )
}

/// Resolves common French corporate and institutional abbreviations into their full forms.
///
/// Rules:
/// - `STE` -> `SOCIETE`
/// - `ETS` / `ETABL` -> `ETABLISSEMENTS`
/// - `CIE` -> `COMPAGNIE`
/// - `BTP` -> `BATIMENT TRAVAUX PUBLICS`
#[inline]
pub fn expand_abbreviation(token: &str) -> Option<&'static str> {
    match token {
        "ste" => Some("societe"),
        "stes" => Some("societes"),
        "ets" | "etabl" | "etabls" => Some("etablissements"),
        "cie" => Some("compagnie"),
        "cies" => Some("compagnies"),
        "btp" => Some("batiment travaux publics"),
        "asso" => Some("association"),
        "sncf" => Some("societe nationale des chemins de fer francais"),
        "ratp" => Some("regie autonome des transports parisiens"),
        "edf" => Some("electricite de france"),
        "gdf" => Some("gaz de france"),
        "urssaf" => Some(
            "union de recouvrement des cotisations de securite sociale et d allocations familiales",
        ),
        "cpam" => Some("caisse primaire d assurance maladie"),
        "caf" => Some("caisse d allocations familiales"),
        "carsat" => Some("caisse d assurance retraite et de la sante au travail"),
        "chu" => Some("centre hospitalier universitaire"),
        "ch" => Some("centre hospitalier"),
        "cci" => Some("chambre de commerce et d industrie"),
        "cma" => Some("chambre de metiers et de l artisanat"),
        "sdis" => Some("service departemental d incendie et de secours"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legal_forms_detection() {
        assert!(is_legal_form("sarl"));
        assert!(is_legal_form("sas"));
        assert!(is_legal_form("sasu"));
        assert!(is_legal_form("sci"));
        assert!(is_legal_form("selas"));
        assert!(is_legal_form("earl"));
        assert!(is_legal_form("gaec"));
        assert!(is_legal_form("association"));
        assert!(is_legal_form("gie"));
        assert!(is_legal_form("ste"));
        assert!(is_legal_form("ets"));

        assert!(!is_legal_form("dupont"));
        assert!(!is_legal_form("plomberie"));
        assert!(!is_legal_form("boucherie"));
    }

    #[test]
    fn test_abbreviation_expansion() {
        assert_eq!(expand_abbreviation("ste"), Some("societe"));
        assert_eq!(expand_abbreviation("ets"), Some("etablissements"));
        assert_eq!(expand_abbreviation("etabl"), Some("etablissements"));
        assert_eq!(expand_abbreviation("cie"), Some("compagnie"));
        assert_eq!(expand_abbreviation("btp"), Some("batiment travaux publics"));
        assert_eq!(expand_abbreviation("dupont"), None);
    }
}
