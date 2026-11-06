use sirene_core::{CodeCommune, CodeNaf, CodePostal, EtatAdministratif};

use crate::damerau::damerau_levenshtein_similarity;
use crate::jaro_winkler::jaro_winkler_similarity;
use crate::token_matcher::score_denomination;

/// Geographic search criteria for location scoring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocationQuery {
    /// Exact 5-character COG commune code (e.g. `"75101"`, `"2A004"`).
    Commune(CodeCommune),
    /// 5-digit postal routing code (e.g. `"75001"`, `"33000"`).
    Postal(CodePostal),
    /// Department code (2 characters for mainland/Corsica e.g. `"75"`, `"33"`, `"2A"`, or 3 for DOM e.g. `"971"`).
    Departement(String),
}

impl LocationQuery {
    /// Attempts to parse a location string into a `LocationQuery`.
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return None;
        }

        if let Ok(c) = CodeCommune::parse(trimmed) {
            return Some(Self::Commune(c));
        }
        if let Ok(p) = CodePostal::parse(trimmed) {
            return Some(Self::Postal(p));
        }
        if trimmed.len() == 2 || trimmed.len() == 3 {
            return Some(Self::Departement(trimmed.to_ascii_uppercase()));
        }
        None
    }
}

/// Breakdown of the multi-criteria score components.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SubScores {
    /// Corporate denomination score $S_{\text{nom}} \in [0.0, 1.0]$.
    pub score_nom: f32,
    /// Geographic location score $S_{\text{loc}} \in [0.0, 1.0]$.
    pub score_loc: f32,
    /// NAF activity code score $S_{\text{naf}} \in [0.0, 1.0]$.
    pub score_naf: f32,
    /// RNE corporate leader score $S_{\text{dir}} \in [0.0, 1.0]$.
    pub score_dir: f32,
    /// Administrative status bonus/penalty $B_{\text{actif}}$.
    pub bonus_actif: f32,
    /// Final composite score.
    pub final_score: f32,
}

/// Configurable weights for the multi-criteria composite scoring model.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScoringWeights {
    /// Weight for corporate denomination ($w_{\text{nom}}$). Default: 0.60.
    pub weight_nom: f32,
    /// Weight for geographic location ($w_{\text{loc}}$). Default: 0.20.
    pub weight_loc: f32,
    /// Weight for NAF activity code ($w_{\text{naf}}$). Default: 0.10.
    pub weight_naf: f32,
    /// Weight for RNE corporate representative ($w_{\text{dir}}$). Default: 0.10.
    pub weight_dir: f32,
    /// Ratio of Jaro-Winkler vs Unordered Tokens in $S_{\text{nom}}$ (default: 0.50).
    pub jw_name_ratio: f32,
    /// Bonus for active administrative status ('A'). Default: +0.05.
    pub bonus_active: f32,
    /// Penalty for closed / ceased entities ('C'). Default: 0.15.
    pub penalty_ceased: f32,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            weight_nom: 0.60,
            weight_loc: 0.20,
            weight_naf: 0.10,
            weight_dir: 0.10,
            jw_name_ratio: 0.50,
            bonus_active: 0.05,
            penalty_ceased: 0.15,
        }
    }
}

/// Returns `true` if two department codes match, taking into account
/// Corsica's dual territorial (2A/2B) and postal (20) department representations.
#[inline]
pub fn are_departments_matching(dept1: &str, dept2: &str) -> bool {
    if dept1.eq_ignore_ascii_case(dept2) {
        return true;
    }
    let is_corse1 = dept1.eq_ignore_ascii_case("20")
        || dept1.eq_ignore_ascii_case("2A")
        || dept1.eq_ignore_ascii_case("2B");
    let is_corse2 = dept2.eq_ignore_ascii_case("20")
        || dept2.eq_ignore_ascii_case("2A")
        || dept2.eq_ignore_ascii_case("2B");

    if is_corse1
        && is_corse2
        && (dept1.eq_ignore_ascii_case("20") || dept2.eq_ignore_ascii_case("20"))
    {
        return true;
    }
    false
}

/// Evaluates the geographic location score $S_{\text{loc}}$:
/// - `1.0` if exact match on INSEE commune code (COG).
/// - `0.8` if exact match on postal code.
/// - `0.5` if same department (first 2 digits / 2A/2B / DOM 3 digits, with Corsica 20<->2A/2B mapping).
/// - `0.0` otherwise.
pub fn score_localisation(
    query_loc: &LocationQuery,
    candidate_commune: Option<&CodeCommune>,
    candidate_postal: Option<&CodePostal>,
) -> f32 {
    match query_loc {
        LocationQuery::Commune(q_commune) => {
            if let Some(c_commune) = candidate_commune {
                if c_commune == q_commune {
                    return 1.0;
                }
            }
            if let Some(c_postal) = candidate_postal {
                if c_postal.as_str() == q_commune.as_str() {
                    return 0.8;
                }
            }
            // Check department match with Corsica tolerance
            let q_dept = q_commune.departement();
            if let Some(c_commune) = candidate_commune {
                if are_departments_matching(c_commune.departement(), q_dept) {
                    return 0.5;
                }
            }
            if let Some(c_postal) = candidate_postal {
                if are_departments_matching(c_postal.departement(), q_dept) {
                    return 0.5;
                }
            }
            0.0
        }
        LocationQuery::Postal(q_postal) => {
            if let Some(c_postal) = candidate_postal {
                if c_postal == q_postal {
                    return 0.8;
                }
            }
            if let Some(c_commune) = candidate_commune {
                if c_commune.as_str() == q_postal.as_str() {
                    return 1.0;
                }
            }
            // Check department match with Corsica tolerance
            let q_dept = q_postal.departement();
            if let Some(c_postal) = candidate_postal {
                if are_departments_matching(c_postal.departement(), q_dept) {
                    return 0.5;
                }
            }
            if let Some(c_commune) = candidate_commune {
                if are_departments_matching(c_commune.departement(), q_dept) {
                    return 0.5;
                }
            }
            0.0
        }
        LocationQuery::Departement(q_dept) => {
            if let Some(c_commune) = candidate_commune {
                if are_departments_matching(c_commune.departement(), q_dept) {
                    return 0.5;
                }
            }
            if let Some(c_postal) = candidate_postal {
                if are_departments_matching(c_postal.departement(), q_dept) {
                    return 0.5;
                }
            }
            0.0
        }
    }
}

/// Evaluates the NAF activity code score $S_{\text{naf}}$:
/// - `1.0` if exact match on NAF sub-class (e.g. `"6201Z"`).
/// - `0.6` if match on class (`"6201"`), group (`"620"`), or division (`"62"`).
/// - `0.0` otherwise.
pub fn score_activite(query_naf: &CodeNaf, candidate_naf: Option<&CodeNaf>) -> f32 {
    let candidate = match candidate_naf {
        Some(naf) => naf,
        None => return 0.0,
    };

    if candidate == query_naf {
        1.0
    } else if candidate.division() == query_naf.division()
        || candidate.groupe() == query_naf.groupe()
        || candidate.classe() == query_naf.classe()
    {
        0.6
    } else {
        0.0
    }
}

/// Evaluates the RNE corporate leader score $S_{\text{dir}}$:
/// Computes the maximum similarity (Jaro-Winkler / Damerau-Levenshtein) across all entity representatives.
pub fn score_dirigeants(query_dirigeant: &str, candidate_dirigeants: &[&str]) -> f32 {
    if query_dirigeant.is_empty() || candidate_dirigeants.is_empty() {
        return 0.0;
    }

    let mut max_sim = 0.0f32;
    for &dir in candidate_dirigeants {
        let jw = jaro_winkler_similarity(query_dirigeant, dir);
        let dam = damerau_levenshtein_similarity(query_dirigeant, dir);
        let sim = jw.max(dam);
        if sim > max_sim {
            max_sim = sim;
        }
        if max_sim >= 1.0 {
            break;
        }
    }

    max_sim
}

/// Computes the active administrative status bonus or penalty $B_{\text{actif}}$.
#[inline]
pub fn bonus_actif(etat: EtatAdministratif, bonus_active: f32, penalty_ceased: f32) -> f32 {
    match etat {
        EtatAdministratif::Actif => bonus_active,
        EtatAdministratif::Cesse => -penalty_ceased,
    }
}

/// Query input components used for multi-criteria composite scoring.
#[derive(Debug, Clone)]
pub struct QueryScoreInput<'a> {
    pub name: &'a str,
    pub tokens: &'a [&'a str],
    pub location: Option<&'a LocationQuery>,
    pub naf: Option<&'a CodeNaf>,
    pub dirigeant: Option<&'a str>,
}

/// Candidate input components used for multi-criteria composite scoring.
#[derive(Debug, Clone)]
pub struct CandidateScoreInput<'a> {
    pub name: &'a str,
    pub tokens: &'a [&'a str],
    pub commune: Option<&'a CodeCommune>,
    pub postal: Option<&'a CodePostal>,
    pub naf: Option<&'a CodeNaf>,
    pub dirigeants: &'a [&'a str],
    pub etat: EtatAdministratif,
}

/// Computes the comprehensive multi-criteria composite score.
///
/// Renormalizes weights when optional search criteria (location, NAF, director) are not requested.
pub fn compute_composite_score(
    weights: &ScoringWeights,
    query: &QueryScoreInput,
    candidate: &CandidateScoreInput,
) -> SubScores {
    // 1. Denomination score S_nom
    let score_nom = score_denomination(
        query.name,
        query.tokens,
        candidate.name,
        candidate.tokens,
        weights.jw_name_ratio,
    );

    // 2. Location score S_loc
    let (score_loc, has_loc) = match query.location {
        Some(loc) => (
            score_localisation(loc, candidate.commune, candidate.postal),
            true,
        ),
        None => (0.0, false),
    };

    // 3. NAF score S_naf
    let (score_naf, has_naf) = match query.naf {
        Some(naf) => (score_activite(naf, candidate.naf), true),
        None => (0.0, false),
    };

    // 4. Dirigeant score S_dir
    let (score_dir, has_dir) = match query.dirigeant {
        Some(dir) => (score_dirigeants(dir, candidate.dirigeants), true),
        None => (0.0, false),
    };

    // 5. Status bonus B_actif
    let b_actif = bonus_actif(candidate.etat, weights.bonus_active, weights.penalty_ceased);

    // Compute active weight sum for dynamic renormalization
    let mut total_weight = weights.weight_nom;
    if has_loc {
        total_weight += weights.weight_loc;
    }
    if has_naf {
        total_weight += weights.weight_naf;
    }
    if has_dir {
        total_weight += weights.weight_dir;
    }

    let mut weighted_sum = weights.weight_nom * score_nom;
    if has_loc {
        weighted_sum += weights.weight_loc * score_loc;
    }
    if has_naf {
        weighted_sum += weights.weight_naf * score_naf;
    }
    if has_dir {
        weighted_sum += weights.weight_dir * score_dir;
    }

    let base_score = if total_weight > 0.0 {
        weighted_sum / total_weight
    } else {
        score_nom
    };

    let final_score = (base_score + b_actif).clamp(0.0, 1.0);

    SubScores {
        score_nom,
        score_loc,
        score_naf,
        score_dir,
        bonus_actif: b_actif,
        final_score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_scoring_hierarchy() {
        let com_paris = CodeCommune::parse("75101").expect("commune");
        let cp_paris = CodePostal::parse("75001").expect("postal");
        let com_lille = CodeCommune::parse("59350").expect("lille");
        let cp_lille = CodePostal::parse("59000").expect("lille cp");

        // Exact COG
        let q_com = LocationQuery::Commune(com_paris);
        assert_eq!(
            score_localisation(&q_com, Some(&com_paris), Some(&cp_paris)),
            1.0
        );

        // Exact postal
        let q_cp = LocationQuery::Postal(cp_paris);
        assert_eq!(
            score_localisation(&q_cp, Some(&com_paris), Some(&cp_paris)),
            0.8
        );

        // Department match (75 vs 75)
        let q_dept = LocationQuery::Departement("75".to_string());
        assert_eq!(
            score_localisation(&q_dept, Some(&com_paris), Some(&cp_paris)),
            0.5
        );

        // Divergence (Paris vs Lille)
        assert_eq!(
            score_localisation(&q_com, Some(&com_lille), Some(&cp_lille)),
            0.0
        );
    }

    #[test]
    fn test_naf_scoring() {
        let naf1 = CodeNaf::parse("62.01Z").expect("naf");
        let naf2 = CodeNaf::parse("62.02A").expect("same division/group");
        let naf3 = CodeNaf::parse("47.11D").expect("different division");

        assert_eq!(score_activite(&naf1, Some(&naf1)), 1.0);
        assert_eq!(score_activite(&naf1, Some(&naf2)), 0.6);
        assert_eq!(score_activite(&naf1, Some(&naf3)), 0.0);
    }

    #[test]
    fn test_dirigeants_scoring() {
        let dirs = ["thomas durand", "marie dupont"];
        let score = score_dirigeants("thomas durand", &dirs);
        assert_eq!(score, 1.0);

        let typo_score = score_dirigeants("tomas durant", &dirs);
        assert!(typo_score > 0.85);

        let mismatch_score = score_dirigeants("alain bernard", &dirs);
        assert!(mismatch_score < 0.60);
        assert!(mismatch_score < typo_score);
    }
}
