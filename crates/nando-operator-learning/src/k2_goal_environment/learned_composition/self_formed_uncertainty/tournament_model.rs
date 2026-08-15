use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::super::{
    K2_INQUIRY_PRECOMMIT_SCHEMA_V1, K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1,
    K2CompositionResultV1, K2InquirySelectionPrecommitV1, K2InquirySelectorRequestV1,
    require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_DIRECT_WINNER_SCHEMA_V1, K2_UNCERTAINTY_MAX_REPRESENTATIVES_V1,
    K2_UNCERTAINTY_MAX_SELECTOR_REQUESTS_V1, K2_UNCERTAINTY_MIN_REPRESENTATIVES_V1,
    K2_UNCERTAINTY_SELECTOR_PROBES_V1, K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1,
    K2_UNCERTAINTY_TOURNAMENT_SCHEMA_V1, K2_UNCERTAINTY_TOURNAMENT_STEP_SCHEMA_V1,
    denied_authority_v1, require_denied_authority_v1, require_exact_len_v1,
    require_sorted_unique_v1, uncertainty_root_v1,
};

pub const K2_UNCERTAINTY_DIRECT_SCORE_SCHEMA_V1: &str = "nando.k2-self-formed-direct-score.v1";
pub const K2_UNCERTAINTY_TOURNAMENT_BATCH_SCHEMA_V1: &str =
    "nando.k2-self-formed-tournament-batch.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyTournamentStepKindV1 {
    Reduction,
    Final,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyTournamentStepV1 {
    pub schema: String,
    pub case_id_sha256: String,
    pub frontier_root_sha256: String,
    pub step_sequence: u64,
    pub kind: K2UncertaintyTournamentStepKindV1,
    pub active_probe_roots_sha256: Vec<String>,
    pub filler_probe_roots_sha256: Vec<String>,
    pub request: K2InquirySelectorRequestV1,
    pub precommit: K2InquirySelectionPrecommitV1,
    pub retained_probe_root_sha256: String,
    pub eliminated_probe_roots_sha256: Vec<String>,
    pub step_root_sha256: String,
}

impl K2UncertaintyTournamentStepV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.case_id_sha256)?;
        require_composition_root_v1(&self.frontier_root_sha256)?;
        require_composition_root_v1(&self.retained_probe_root_sha256)?;
        self.request.validate()?;
        validate_precommit_v1(&self.precommit)?;
        if self.precommit.selector_request_root_sha256 != self.request.request_root_sha256
            || self.precommit.public_case_root_sha256 != self.request.public_case.case_root_sha256
            || self.precommit.selected_probe_root_sha256 != self.retained_probe_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_tournament_precommit_binding_invalid",
            ));
        }
        require_sorted_unique_v1(
            &self.active_probe_roots_sha256,
            "self_formed_tournament_active_roots_invalid",
        )?;
        require_sorted_unique_v1(
            &self.filler_probe_roots_sha256,
            "self_formed_tournament_filler_roots_invalid",
        )?;
        for root in self
            .active_probe_roots_sha256
            .iter()
            .chain(&self.filler_probe_roots_sha256)
            .chain(&self.eliminated_probe_roots_sha256)
        {
            require_composition_root_v1(root)?;
        }
        let active = self
            .active_probe_roots_sha256
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let fillers = self
            .filler_probe_roots_sha256
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if !active.is_disjoint(&fillers)
            || !active.contains(&self.retained_probe_root_sha256)
            || fillers.contains(&self.retained_probe_root_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_tournament_active_filler_invalid",
            ));
        }
        let request_roots = self
            .request
            .public_case
            .probes
            .iter()
            .map(|probe| probe.probe_root_sha256.clone())
            .collect::<BTreeSet<_>>();
        let submitted = active.union(&fillers).cloned().collect::<BTreeSet<_>>();
        if request_roots != submitted || request_roots.len() != K2_UNCERTAINTY_SELECTOR_PROBES_V1 {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_tournament_request_members_invalid",
            ));
        }
        let expected_eliminated = active
            .iter()
            .filter(|root| *root != &self.retained_probe_root_sha256)
            .cloned()
            .collect::<Vec<_>>();
        match self.kind {
            K2UncertaintyTournamentStepKindV1::Reduction => {
                if !fillers.is_empty()
                    || active.len() != K2_UNCERTAINTY_SELECTOR_PROBES_V1
                    || self.eliminated_probe_roots_sha256 != expected_eliminated
                {
                    return Err(K2CompositionErrorV1::Invalid(
                        "self_formed_tournament_reduction_invalid",
                    ));
                }
            }
            K2UncertaintyTournamentStepKindV1::Final => {
                if active.is_empty()
                    || active.len() > K2_UNCERTAINTY_SELECTOR_PROBES_V1
                    || active.len() + fillers.len() != K2_UNCERTAINTY_SELECTOR_PROBES_V1
                    || !self.eliminated_probe_roots_sha256.is_empty()
                {
                    return Err(K2CompositionErrorV1::Invalid(
                        "self_formed_tournament_final_invalid",
                    ));
                }
            }
        }
        let expected = self.expected_root()?;
        if self.schema != K2_UNCERTAINTY_TOURNAMENT_STEP_SCHEMA_V1
            || self.step_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_tournament_step_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.step_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_TOURNAMENT_STEP_SCHEMA_V1,
            &self.case_id_sha256,
            &self.frontier_root_sha256,
            self.step_sequence,
            self.kind,
            &self.active_probe_roots_sha256,
            &self.filler_probe_roots_sha256,
            &self.request,
            &self.precommit,
            &self.retained_probe_root_sha256,
            &self.eliminated_probe_roots_sha256,
        ))
    }
}

fn validate_precommit_v1(precommit: &K2InquirySelectionPrecommitV1) -> K2CompositionResultV1<()> {
    require_composition_root_v1(&precommit.selector_request_root_sha256)?;
    require_composition_root_v1(&precommit.public_case_root_sha256)?;
    require_composition_root_v1(&precommit.selected_probe_root_sha256)?;
    require_denied_authority_v1(&precommit.authority)?;
    let expected = uncertainty_root_v1(&(
        K2_INQUIRY_PRECOMMIT_SCHEMA_V1,
        &precommit.selector_request_root_sha256,
        &precommit.public_case_root_sha256,
        &precommit.evaluations,
        &precommit.selected_probe_root_sha256,
        precommit.exact_best_ties,
        &precommit.authority,
    ))?;
    if precommit.schema != K2_INQUIRY_PRECOMMIT_SCHEMA_V1
        || precommit.precommit_root_sha256 != expected
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_tournament_precommit_invalid",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyDirectScoreV1 {
    pub schema: String,
    pub probe_root_sha256: String,
    pub eligible: bool,
    pub minimax_eliminated: u64,
    pub pair_separation: u64,
    pub risk_units: u64,
    pub cost_units: u64,
    pub score_root_sha256: String,
}

impl K2UncertaintyDirectScoreV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.probe_root_sha256)?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_DIRECT_SCORE_SCHEMA_V1,
            &self.probe_root_sha256,
            self.eligible,
            self.minimax_eliminated,
            self.pair_separation,
            self.risk_units,
            self.cost_units,
        ))?;
        if self.schema != K2_UNCERTAINTY_DIRECT_SCORE_SCHEMA_V1
            || self.score_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_direct_score_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.score_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_DIRECT_SCORE_SCHEMA_V1,
            &self.probe_root_sha256,
            self.eligible,
            self.minimax_eliminated,
            self.pair_separation,
            self.risk_units,
            self.cost_units,
        ))?;
        self.validate()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyDirectWinnerV1 {
    pub schema: String,
    pub case_id_sha256: String,
    pub frontier_root_sha256: String,
    pub scores: Vec<K2UncertaintyDirectScoreV1>,
    pub selected_probe_root_sha256: String,
    pub direct_winner_root_sha256: String,
}

impl K2UncertaintyDirectWinnerV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.case_id_sha256)?;
        require_composition_root_v1(&self.frontier_root_sha256)?;
        if self.scores.len() < K2_UNCERTAINTY_MIN_REPRESENTATIVES_V1
            || self.scores.len() > K2_UNCERTAINTY_MAX_REPRESENTATIVES_V1
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_direct_score_denominator_invalid",
            ));
        }
        for score in &self.scores {
            score.validate()?;
        }
        if self
            .scores
            .windows(2)
            .any(|pair| pair[0].probe_root_sha256 >= pair[1].probe_root_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_direct_scores_not_canonical",
            ));
        }
        let selected = self
            .scores
            .iter()
            .filter(|score| score.eligible)
            .min_by(|left, right| compare_direct_scores_v1(left, right))
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_direct_winner_missing",
            ))?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_DIRECT_WINNER_SCHEMA_V1,
            &self.case_id_sha256,
            &self.frontier_root_sha256,
            &self.scores,
            &self.selected_probe_root_sha256,
        ))?;
        if self.schema != K2_UNCERTAINTY_DIRECT_WINNER_SCHEMA_V1
            || self.selected_probe_root_sha256 != selected.probe_root_sha256
            || self.direct_winner_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_direct_winner_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.direct_winner_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_DIRECT_WINNER_SCHEMA_V1,
            &self.case_id_sha256,
            &self.frontier_root_sha256,
            &self.scores,
            &self.selected_probe_root_sha256,
        ))?;
        self.validate()
    }
}

fn compare_direct_scores_v1(
    left: &K2UncertaintyDirectScoreV1,
    right: &K2UncertaintyDirectScoreV1,
) -> std::cmp::Ordering {
    right
        .minimax_eliminated
        .cmp(&left.minimax_eliminated)
        .then_with(|| right.pair_separation.cmp(&left.pair_separation))
        .then_with(|| left.risk_units.cmp(&right.risk_units))
        .then_with(|| left.cost_units.cmp(&right.cost_units))
        .then_with(|| left.probe_root_sha256.cmp(&right.probe_root_sha256))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyTournamentV1 {
    pub schema: String,
    pub case_id_sha256: String,
    pub frontier_root_sha256: String,
    pub representative_count: u64,
    pub selector_source_sha256: String,
    pub selector_executable_sha256: String,
    pub step_roots_sha256: Vec<String>,
    pub request_count: u64,
    pub adapted_prediction_count: u64,
    pub tournament_winner_probe_root_sha256: String,
    pub direct_winner: K2UncertaintyDirectWinnerV1,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub tournament_root_sha256: String,
}

impl K2UncertaintyTournamentV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.case_id_sha256,
            &self.frontier_root_sha256,
            &self.selector_source_sha256,
            &self.selector_executable_sha256,
            &self.tournament_winner_probe_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.direct_winner.validate()?;
        let expected_requests = self
            .representative_count
            .saturating_sub(K2_UNCERTAINTY_SELECTOR_PROBES_V1 as u64)
            .div_ceil((K2_UNCERTAINTY_SELECTOR_PROBES_V1 - 1) as u64)
            + 1;
        if self.representative_count < K2_UNCERTAINTY_MIN_REPRESENTATIVES_V1 as u64
            || self.representative_count > K2_UNCERTAINTY_MAX_REPRESENTATIVES_V1 as u64
            || self.request_count != expected_requests
            || self.request_count > K2_UNCERTAINTY_MAX_SELECTOR_REQUESTS_V1 as u64
            || self.step_roots_sha256.len() as u64 != self.request_count
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_tournament_denominator_invalid",
            ));
        }
        for root in &self.step_roots_sha256 {
            require_composition_root_v1(root)?;
        }
        if self.step_roots_sha256.iter().collect::<BTreeSet<_>>().len()
            != self.step_roots_sha256.len()
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_tournament_step_roots_invalid",
            ));
        }
        let adapted = self
            .request_count
            .checked_mul(K2_UNCERTAINTY_SELECTOR_PROBES_V1 as u64)
            .and_then(|value| value.checked_mul(4))
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_tournament_prediction_overflow",
            ))?;
        require_denied_authority_v1(&self.authority)?;
        let expected = self.expected_root()?;
        if self.tournament_winner_probe_root_sha256 != self.direct_winner.selected_probe_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_tournament_direct_winner_mismatch",
            ));
        }
        if self.schema != K2_UNCERTAINTY_TOURNAMENT_SCHEMA_V1
            || self.direct_winner.case_id_sha256 != self.case_id_sha256
            || self.direct_winner.frontier_root_sha256 != self.frontier_root_sha256
            || self.direct_winner.scores.len() as u64 != self.representative_count
            || self.selector_source_sha256 != K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1
            || self.adapted_prediction_count != adapted
            || self.tournament_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_tournament_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.tournament_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_TOURNAMENT_SCHEMA_V1,
            &self.case_id_sha256,
            &self.frontier_root_sha256,
            self.representative_count,
            &self.selector_source_sha256,
            &self.selector_executable_sha256,
            &self.step_roots_sha256,
            self.request_count,
            self.adapted_prediction_count,
            &self.tournament_winner_probe_root_sha256,
            &self.direct_winner,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyTournamentBatchV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub tournaments: Vec<K2UncertaintyTournamentV1>,
    pub representative_count: u64,
    pub request_count: u64,
    pub adapted_prediction_count: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub batch_root_sha256: String,
}

impl K2UncertaintyTournamentBatchV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.experiment_id_sha256)?;
        require_exact_len_v1(
            self.tournaments.len(),
            super::K2_UNCERTAINTY_CONFIRM_CASES_V1,
            "self_formed_tournament_batch_case_count_invalid",
        )?;
        let mut cases = BTreeSet::new();
        let mut representatives = 0_u64;
        let mut requests = 0_u64;
        let mut predictions = 0_u64;
        for tournament in &self.tournaments {
            tournament.validate()?;
            if !cases.insert(&tournament.case_id_sha256) {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_tournament_batch_duplicate_case",
                ));
            }
            representatives = representatives
                .checked_add(tournament.representative_count)
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_tournament_batch_count_overflow",
                ))?;
            requests = requests.checked_add(tournament.request_count).ok_or(
                K2CompositionErrorV1::Invalid("self_formed_tournament_batch_count_overflow"),
            )?;
            predictions = predictions
                .checked_add(tournament.adapted_prediction_count)
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_tournament_batch_count_overflow",
                ))?;
        }
        require_denied_authority_v1(&self.authority)?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_TOURNAMENT_BATCH_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.tournaments,
            representatives,
            requests,
            predictions,
            &self.authority,
        ))?;
        if self.schema != K2_UNCERTAINTY_TOURNAMENT_BATCH_SCHEMA_V1
            || self.representative_count != representatives
            || self.request_count != requests
            || self.adapted_prediction_count != predictions
            || self.batch_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_tournament_batch_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.authority = denied_authority_v1();
        self.representative_count = self
            .tournaments
            .iter()
            .map(|value| value.representative_count)
            .sum();
        self.request_count = self
            .tournaments
            .iter()
            .map(|value| value.request_count)
            .sum();
        self.adapted_prediction_count = self
            .tournaments
            .iter()
            .map(|value| value.adapted_prediction_count)
            .sum();
        self.batch_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_TOURNAMENT_BATCH_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.tournaments,
            self.representative_count,
            self.request_count,
            self.adapted_prediction_count,
            &self.authority,
        ))?;
        self.validate()
    }
}
