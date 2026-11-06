use sirene_core::{CodeCommune, CodeNaf, CodePostal, EtatAdministratif, Siren, Siret};
use sirene_normalize::NormalizedQuery;

use crate::scoring::{
    compute_composite_score, CandidateScoreInput, LocationQuery, QueryScoreInput, ScoringWeights,
    SubScores,
};

/// An enterprise candidate evaluated by the scoring engine.
#[derive(Debug, Clone)]
pub struct MatchCandidate<'a> {
    pub siren: Siren,
    pub siret: Option<Siret>,
    pub denomination: &'a str,
    pub tokens: &'a [&'a str],
    pub code_commune: Option<CodeCommune>,
    pub code_postal: Option<CodePostal>,
    pub code_naf: Option<CodeNaf>,
    pub etat_administratif: EtatAdministratif,
    pub dirigeants: &'a [&'a str],
}

/// A parsed search query with optional multi-criteria filters.
#[derive(Debug, Clone)]
pub struct SearchQuery<'a> {
    pub query: &'a NormalizedQuery,
    pub location: Option<LocationQuery>,
    pub naf: Option<CodeNaf>,
    pub dirigeant: Option<&'a str>,
    pub strict_location: bool,
}

impl<'a> SearchQuery<'a> {
    /// Creates a basic search query from a normalized query.
    pub fn new(query: &'a NormalizedQuery) -> Self {
        Self {
            query,
            location: None,
            naf: None,
            dirigeant: None,
            strict_location: false,
        }
    }
}

/// A candidate scored and ranked by the matching engine.
#[derive(Debug, Clone)]
pub struct ScoredCandidate<'a> {
    pub candidate: &'a MatchCandidate<'a>,
    pub sub_scores: SubScores,
    pub is_exact_pivot: bool,
}

impl ScoredCandidate<'_> {
    /// Returns the final matching score.
    #[inline]
    pub fn score(&self) -> f32 {
        self.sub_scores.final_score
    }
}

/// High-level entity matcher executing multi-criteria scoring and candidate ranking.
#[derive(Debug, Clone, Copy, Default)]
pub struct EntityMatcher {
    weights: ScoringWeights,
    min_threshold: f32,
}

impl EntityMatcher {
    /// Creates a new `EntityMatcher` with default weights.
    pub fn new() -> Self {
        Self {
            weights: ScoringWeights::default(),
            min_threshold: 0.0,
        }
    }

    /// Creates a matcher with custom weights and minimum score threshold.
    pub const fn with_weights(weights: ScoringWeights, min_threshold: f32) -> Self {
        Self {
            weights,
            min_threshold,
        }
    }

    /// Evaluates a single candidate against a search query.
    pub fn score_candidate<'a>(
        &self,
        search: &SearchQuery,
        candidate: &'a MatchCandidate<'a>,
    ) -> ScoredCandidate<'a> {
        // 1. Pivot short circuit (Section 4.1): if query contains a valid SIREN or SIRET matching the candidate
        if let Some(pivot) = search.query.pivot {
            let is_match = match pivot {
                sirene_core::IdentifiantPivot::Siren(s) => s == candidate.siren,
                sirene_core::IdentifiantPivot::Siret(s) => {
                    if let Some(c_siret) = candidate.siret {
                        s == c_siret || s.siren() == candidate.siren
                    } else {
                        s.siren() == candidate.siren
                    }
                }
            };

            if is_match {
                return ScoredCandidate {
                    candidate,
                    sub_scores: SubScores {
                        score_nom: 1.0,
                        score_loc: 1.0,
                        score_naf: 1.0,
                        score_dir: 1.0,
                        bonus_actif: if candidate.etat_administratif.is_actif() {
                            self.weights.bonus_active
                        } else {
                            -self.weights.penalty_ceased
                        },
                        final_score: 1.0,
                    },
                    is_exact_pivot: true,
                };
            }
        }

        // 2. Multi-criteria composite scoring
        let query_tokens: smallvec::SmallVec<[&str; 8]> =
            search.query.tokens.iter().map(|s| s.as_str()).collect();

        let distinctive = search.query.distinctive_text();
        let query_name = if distinctive.is_empty() {
            &search.query.folded
        } else {
            &distinctive
        };

        let q_input = QueryScoreInput {
            name: query_name,
            tokens: &query_tokens,
            location: search.location.as_ref(),
            naf: search.naf.as_ref(),
            dirigeant: search.dirigeant,
        };

        let c_input = CandidateScoreInput {
            name: candidate.denomination,
            tokens: candidate.tokens,
            commune: candidate.code_commune.as_ref(),
            postal: candidate.code_postal.as_ref(),
            naf: candidate.code_naf.as_ref(),
            dirigeants: candidate.dirigeants,
            etat: candidate.etat_administratif,
        };

        let sub_scores = compute_composite_score(&self.weights, &q_input, &c_input);

        ScoredCandidate {
            candidate,
            sub_scores,
            is_exact_pivot: false,
        }
    }

    /// Scores and ranks a collection of candidates, returning up to `top_k` results
    /// exceeding the minimum score threshold sorted in descending order of score.
    pub fn rank<'a>(
        &self,
        search: &SearchQuery,
        candidates: &'a [MatchCandidate<'a>],
        top_k: usize,
    ) -> Vec<ScoredCandidate<'a>> {
        let mut scored: Vec<ScoredCandidate<'a>> = candidates
            .iter()
            .map(|c| self.score_candidate(search, c))
            .filter(|sc| sc.score() >= self.min_threshold)
            .collect();

        // Exact pivots always come first, followed by descending final_score
        scored.sort_by(|a, b| {
            b.is_exact_pivot.cmp(&a.is_exact_pivot).then_with(|| {
                b.score()
                    .partial_cmp(&a.score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        if scored.len() > top_k {
            scored.truncate(top_k);
        }

        scored
    }
}
