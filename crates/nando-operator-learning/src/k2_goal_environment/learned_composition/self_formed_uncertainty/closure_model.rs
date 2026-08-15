use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    K2InquiryObservationModeV1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_CLOSURE_CENSUS_SCHEMA_V1, K2_UNCERTAINTY_CLOSURE_PLANNER_REQUEST_SCHEMA_V1,
    K2_UNCERTAINTY_COMPLETION_CANDIDATE_SCHEMA_V1, K2_UNCERTAINTY_CONFIRM_MODELS_V1,
    K2_UNCERTAINTY_MAX_COST_UNITS_V1, K2_UNCERTAINTY_MAX_PLAN_COST_UNITS_V1,
    K2_UNCERTAINTY_MAX_PLAN_RISK_UNITS_V1, K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1,
    K2_UNCERTAINTY_MAX_REPRESENTATIVES_V1, K2_UNCERTAINTY_MAX_RISK_UNITS_V1,
    K2_UNCERTAINTY_MIN_REPRESENTATIVES_V1, K2UncertaintyEligibilityDispositionV1,
    K2UncertaintyRawProbeDispositionV1, K2UncertaintySafetyDispositionV1, denied_authority_v1,
    require_denied_authority_v1, require_exact_len_v1, require_sorted_unique_v1,
    uncertainty_decode_v1, uncertainty_root_v1,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyClosureDispositionV1 {
    SingleProbe,
    TwoProbe,
    ClosureUnavailable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyClosurePlannerRequestV1 {
    pub schema: String,
    pub case_id_sha256: String,
    pub frontier_root_sha256: String,
    pub first_tournament_root_sha256: String,
    pub first_probe_root_sha256: String,
    pub representatives: Vec<K2UncertaintyRawProbeDispositionV1>,
    pub planner_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintyClosurePlannerRequestV1 {
    pub fn seal(
        case_id_sha256: String,
        frontier_root_sha256: String,
        first_tournament_root_sha256: String,
        first_probe_root_sha256: String,
        mut representatives: Vec<K2UncertaintyRawProbeDispositionV1>,
        planner_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        representatives.sort_by(|left, right| {
            left.probe
                .probe_root_sha256
                .cmp(&right.probe.probe_root_sha256)
        });
        let mut value = Self {
            schema: K2_UNCERTAINTY_CLOSURE_PLANNER_REQUEST_SCHEMA_V1.to_owned(),
            case_id_sha256,
            frontier_root_sha256,
            first_tournament_root_sha256,
            first_probe_root_sha256,
            representatives,
            planner_executable_sha256,
            authority: denied_authority_v1(),
            request_root_sha256: String::new(),
        };
        value.request_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.case_id_sha256,
            &self.frontier_root_sha256,
            &self.first_tournament_root_sha256,
            &self.first_probe_root_sha256,
            &self.planner_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        if self.representatives.len() < K2_UNCERTAINTY_MIN_REPRESENTATIVES_V1
            || self.representatives.len() > K2_UNCERTAINTY_MAX_REPRESENTATIVES_V1
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_closure_representative_count_invalid",
            ));
        }
        for representative in &self.representatives {
            representative.validate()?;
            if representative.probe.experiment_id_sha256 != self.case_id_sha256 {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_closure_representative_case_invalid",
                ));
            }
        }
        if self
            .representatives
            .windows(2)
            .any(|pair| pair[0].probe.probe_root_sha256 >= pair[1].probe.probe_root_sha256)
            || !self
                .representatives
                .iter()
                .any(|value| value.probe.probe_root_sha256 == self.first_probe_root_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_closure_representative_membership_invalid",
            ));
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CLOSURE_PLANNER_REQUEST_SCHEMA_V1
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_closure_planner_request_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CLOSURE_PLANNER_REQUEST_SCHEMA_V1,
            &self.case_id_sha256,
            &self.frontier_root_sha256,
            &self.first_tournament_root_sha256,
            &self.first_probe_root_sha256,
            &self.representatives,
            &self.planner_executable_sha256,
            &self.authority,
        ))
    }
}

pub fn decode_self_formed_closure_planner_request_v1(
    bytes: &[u8],
) -> K2CompositionResultV1<K2UncertaintyClosurePlannerRequestV1> {
    if bytes.len() > K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_protocol_bytes_exhausted",
        ));
    }
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| K2CompositionErrorV1::Invalid("self_formed_closure_protocol_invalid"))?;
    if json_contains_any_key_v1(
        &value,
        &[
            "mapping",
            "topology_family",
            "matched_pair_index",
            "private_case",
            "private_truth",
            "resolved_private_effect",
        ],
    ) {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_closure_private_input_forbidden",
        ));
    }
    if json_contains_any_key_v1(
        &value,
        &[
            "observed_outcome",
            "observation_vector",
            "post_manifest",
            "worker_outcome",
        ],
    ) {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_closure_post_outcome_input_forbidden",
        ));
    }
    let request: K2UncertaintyClosurePlannerRequestV1 = uncertainty_decode_v1(bytes)?;
    request.validate()?;
    Ok(request)
}

fn json_contains_any_key_v1(value: &serde_json::Value, forbidden: &[&str]) -> bool {
    match value {
        serde_json::Value::Object(object) => object.iter().any(|(key, value)| {
            forbidden.contains(&key.as_str()) || json_contains_any_key_v1(value, forbidden)
        }),
        serde_json::Value::Array(values) => values
            .iter()
            .any(|value| json_contains_any_key_v1(value, forbidden)),
        _ => false,
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCompletionCandidateV1 {
    pub schema: String,
    pub case_id_sha256: String,
    pub first_probe_root_sha256: String,
    pub second_probe_root_sha256: String,
    pub first_prediction_roots_sha256: Vec<String>,
    pub second_prediction_roots_sha256: Vec<String>,
    pub joint_pairwise_outcome_equal: [bool; 6],
    pub joint_partition_sizes: Vec<u64>,
    pub joint_minimax_eliminated: u64,
    pub joint_pair_separation: u64,
    pub cumulative_risk_units: u64,
    pub cumulative_cost_units: u64,
    pub eligible: bool,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub candidate_root_sha256: String,
}

impl K2UncertaintyCompletionCandidateV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.case_id_sha256,
            &self.first_probe_root_sha256,
            &self.second_probe_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_exact_len_v1(
            self.first_prediction_roots_sha256.len(),
            K2_UNCERTAINTY_CONFIRM_MODELS_V1,
            "self_formed_completion_first_predictions_invalid",
        )?;
        require_exact_len_v1(
            self.second_prediction_roots_sha256.len(),
            K2_UNCERTAINTY_CONFIRM_MODELS_V1,
            "self_formed_completion_second_predictions_invalid",
        )?;
        for root in self
            .first_prediction_roots_sha256
            .iter()
            .chain(&self.second_prediction_roots_sha256)
        {
            require_composition_root_v1(root)?;
        }
        let partition = closure_partition_sizes_v1(self.joint_pairwise_outcome_equal)?;
        let largest = partition.first().copied().unwrap_or_default();
        let pair_separation = 16_u64
            .checked_sub(
                partition
                    .iter()
                    .try_fold(0_u64, |sum, size| sum.checked_add(size.checked_mul(*size)?))
                    .ok_or(K2CompositionErrorV1::Invalid(
                        "self_formed_completion_pair_separation_overflow",
                    ))?,
            )
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_completion_pair_separation_invalid",
            ))?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_COMPLETION_CANDIDATE_SCHEMA_V1
            || self.first_probe_root_sha256 == self.second_probe_root_sha256
            || self.joint_partition_sizes != partition
            || self.joint_minimax_eliminated != 4_u64.saturating_sub(largest)
            || self.joint_pair_separation != pair_separation
            || self.cumulative_risk_units > K2_UNCERTAINTY_MAX_PLAN_RISK_UNITS_V1
            || self.cumulative_cost_units > K2_UNCERTAINTY_MAX_PLAN_COST_UNITS_V1
            || self.candidate_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_completion_candidate_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.candidate_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        completion_candidate_root_v1(self)
    }
}

pub(crate) fn completion_candidate_root_v1(
    value: &K2UncertaintyCompletionCandidateV1,
) -> K2CompositionResultV1<String> {
    uncertainty_root_v1(&(
        K2_UNCERTAINTY_COMPLETION_CANDIDATE_SCHEMA_V1,
        &value.case_id_sha256,
        &value.first_probe_root_sha256,
        &value.second_probe_root_sha256,
        &value.first_prediction_roots_sha256,
        &value.second_prediction_roots_sha256,
        value.joint_pairwise_outcome_equal,
        &value.joint_partition_sizes,
        value.joint_minimax_eliminated,
        value.joint_pair_separation,
        value.cumulative_risk_units,
        value.cumulative_cost_units,
        value.eligible,
        &value.authority,
    ))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyClosureCensusV1 {
    pub schema: String,
    pub planner_request_root_sha256: String,
    pub case_id_sha256: String,
    pub frontier_root_sha256: String,
    pub representative_probe_roots_sha256: Vec<String>,
    pub representative_count: u64,
    pub first_tournament_root_sha256: String,
    pub first_probe_root_sha256: String,
    pub first_pairwise_outcome_equal: [bool; 6],
    pub first_partition_sizes: Vec<u64>,
    pub completion_required: bool,
    pub second_probe_candidate_roots_sha256: Vec<String>,
    pub candidate_count: u64,
    pub membership_root_sha256: String,
    pub candidates: Vec<K2UncertaintyCompletionCandidateV1>,
    pub candidate_denominator_root_sha256: String,
    pub disposition: K2UncertaintyClosureDispositionV1,
    pub selected_second_probe_root_sha256: Option<String>,
    pub selected_joint_partition_sizes: Option<Vec<u64>>,
    pub planner_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub census_root_sha256: String,
}

#[derive(Serialize)]
struct K2UncertaintyClosureCensusRootPreimageV1<'a> {
    schema: &'static str,
    planner_request_root_sha256: &'a str,
    case_id_sha256: &'a str,
    frontier_root_sha256: &'a str,
    representative_probe_roots_sha256: &'a [String],
    representative_count: u64,
    first_tournament_root_sha256: &'a str,
    first_probe_root_sha256: &'a str,
    first_pairwise_outcome_equal: [bool; 6],
    first_partition_sizes: &'a [u64],
    completion_required: bool,
    second_probe_candidate_roots_sha256: &'a [String],
    candidate_count: u64,
    membership_root_sha256: &'a str,
    candidates: &'a [K2UncertaintyCompletionCandidateV1],
    candidate_denominator_root_sha256: &'a str,
    disposition: K2UncertaintyClosureDispositionV1,
    selected_second_probe_root_sha256: &'a Option<String>,
    selected_joint_partition_sizes: &'a Option<Vec<u64>>,
    planner_executable_sha256: &'a str,
    authority: &'a K2CompositionAuthorityBoundaryV1,
}

impl K2UncertaintyClosureCensusV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.planner_request_root_sha256,
            &self.case_id_sha256,
            &self.frontier_root_sha256,
            &self.first_tournament_root_sha256,
            &self.first_probe_root_sha256,
            &self.membership_root_sha256,
            &self.candidate_denominator_root_sha256,
            &self.planner_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_sorted_unique_v1(
            &self.representative_probe_roots_sha256,
            "self_formed_closure_representative_roots_invalid",
        )?;
        if self.representative_count != self.representative_probe_roots_sha256.len() as u64
            || self.representative_count < K2_UNCERTAINTY_MIN_REPRESENTATIVES_V1 as u64
            || self.representative_count > K2_UNCERTAINTY_MAX_REPRESENTATIVES_V1 as u64
            || !self
                .representative_probe_roots_sha256
                .contains(&self.first_probe_root_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_closure_representative_denominator_invalid",
            ));
        }
        let first_partition = closure_partition_sizes_v1(self.first_pairwise_outcome_equal)?;
        let completion_required = first_partition.first().copied().unwrap_or_default() > 1;
        let expected_second_roots = if completion_required {
            self.representative_probe_roots_sha256
                .iter()
                .filter(|root| *root != &self.first_probe_root_sha256)
                .cloned()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        require_sorted_unique_or_empty_v1(
            &self.second_probe_candidate_roots_sha256,
            "self_formed_closure_second_roots_invalid",
        )?;
        for candidate in &self.candidates {
            candidate.validate()?;
        }
        if self
            .candidates
            .windows(2)
            .any(|pair| pair[0].second_probe_root_sha256 >= pair[1].second_probe_root_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_closure_candidates_not_canonical",
            ));
        }
        let actual_second_roots = self
            .candidates
            .iter()
            .map(|candidate| candidate.second_probe_root_sha256.clone())
            .collect::<Vec<_>>();
        let candidate_roots = self
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_root_sha256.clone())
            .collect::<Vec<_>>();
        let membership_root = uncertainty_root_v1(&(
            "nando.k2-self-formed-completion-membership.v1",
            &self.representative_probe_roots_sha256,
            &self.first_probe_root_sha256,
            &expected_second_roots,
        ))?;
        let denominator_root = uncertainty_root_v1(&(
            "nando.k2-self-formed-completion-denominator.v1",
            &candidate_roots,
        ))?;
        let selected = self
            .candidates
            .iter()
            .filter(|candidate| {
                candidate.eligible && candidate.joint_partition_sizes == [1_u64, 1, 1, 1]
            })
            .min_by(|left, right| compare_completion_candidates_v1(left, right));
        let expected_disposition = if !completion_required {
            K2UncertaintyClosureDispositionV1::SingleProbe
        } else if selected.is_some() {
            K2UncertaintyClosureDispositionV1::TwoProbe
        } else {
            K2UncertaintyClosureDispositionV1::ClosureUnavailable
        };
        let expected_selected =
            selected.map(|candidate| candidate.second_probe_root_sha256.clone());
        let expected_partition = selected.map(|candidate| candidate.joint_partition_sizes.clone());
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CLOSURE_CENSUS_SCHEMA_V1
            || self.first_partition_sizes != first_partition
            || self.completion_required != completion_required
            || self.second_probe_candidate_roots_sha256 != expected_second_roots
            || actual_second_roots != expected_second_roots
            || self.candidate_count != self.candidates.len() as u64
            || self.membership_root_sha256 != membership_root
            || self.candidate_denominator_root_sha256 != denominator_root
            || self.disposition != expected_disposition
            || self.selected_second_probe_root_sha256 != expected_selected
            || self.selected_joint_partition_sizes != expected_partition
            || self.census_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_closure_census_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.census_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        closure_census_root_v1(self)
    }
}

pub(crate) fn closure_census_root_v1(
    value: &K2UncertaintyClosureCensusV1,
) -> K2CompositionResultV1<String> {
    uncertainty_root_v1(&K2UncertaintyClosureCensusRootPreimageV1 {
        schema: K2_UNCERTAINTY_CLOSURE_CENSUS_SCHEMA_V1,
        planner_request_root_sha256: &value.planner_request_root_sha256,
        case_id_sha256: &value.case_id_sha256,
        frontier_root_sha256: &value.frontier_root_sha256,
        representative_probe_roots_sha256: &value.representative_probe_roots_sha256,
        representative_count: value.representative_count,
        first_tournament_root_sha256: &value.first_tournament_root_sha256,
        first_probe_root_sha256: &value.first_probe_root_sha256,
        first_pairwise_outcome_equal: value.first_pairwise_outcome_equal,
        first_partition_sizes: &value.first_partition_sizes,
        completion_required: value.completion_required,
        second_probe_candidate_roots_sha256: &value.second_probe_candidate_roots_sha256,
        candidate_count: value.candidate_count,
        membership_root_sha256: &value.membership_root_sha256,
        candidates: &value.candidates,
        candidate_denominator_root_sha256: &value.candidate_denominator_root_sha256,
        disposition: value.disposition,
        selected_second_probe_root_sha256: &value.selected_second_probe_root_sha256,
        selected_joint_partition_sizes: &value.selected_joint_partition_sizes,
        planner_executable_sha256: &value.planner_executable_sha256,
        authority: &value.authority,
    })
}

pub(crate) fn closure_probe_eligible_v1(disposition: &K2UncertaintyRawProbeDispositionV1) -> bool {
    disposition.eligibility == K2UncertaintyEligibilityDispositionV1::Eligible
        && disposition.safety == K2UncertaintySafetyDispositionV1::Pass
        && disposition.probe.reversible
        && disposition.probe.observation_mode == K2InquiryObservationModeV1::ExactImmediate
        && disposition.probe.risk_units <= K2_UNCERTAINTY_MAX_RISK_UNITS_V1
        && disposition.probe.cost_units <= K2_UNCERTAINTY_MAX_COST_UNITS_V1
}

pub(crate) fn compare_completion_candidates_v1(
    left: &K2UncertaintyCompletionCandidateV1,
    right: &K2UncertaintyCompletionCandidateV1,
) -> std::cmp::Ordering {
    right
        .joint_minimax_eliminated
        .cmp(&left.joint_minimax_eliminated)
        .then_with(|| right.joint_pair_separation.cmp(&left.joint_pair_separation))
        .then_with(|| left.cumulative_risk_units.cmp(&right.cumulative_risk_units))
        .then_with(|| left.cumulative_cost_units.cmp(&right.cumulative_cost_units))
        .then_with(|| {
            left.second_probe_root_sha256
                .cmp(&right.second_probe_root_sha256)
        })
}

pub(crate) fn closure_partition_sizes_v1(equal: [bool; 6]) -> K2CompositionResultV1<Vec<u64>> {
    let pairs = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    let mut parent = [0_usize, 1, 2, 3];
    for ((left, right), same) in pairs.into_iter().zip(equal) {
        if same {
            let left_root = closure_find_root_v1(&mut parent, left);
            let right_root = closure_find_root_v1(&mut parent, right);
            parent[right_root] = left_root;
        }
    }
    for ((left, right), same) in pairs.into_iter().zip(equal) {
        let equivalent =
            closure_find_root_v1(&mut parent, left) == closure_find_root_v1(&mut parent, right);
        if equivalent != same {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_completion_equality_nontransitive",
            ));
        }
    }
    let mut counts = [0_u64; 4];
    for index in 0..4 {
        let root = closure_find_root_v1(&mut parent, index);
        counts[root] = counts[root].saturating_add(1);
    }
    let mut sizes = counts
        .into_iter()
        .filter(|count| *count > 0)
        .collect::<Vec<_>>();
    sizes.sort_by(|left, right| right.cmp(left));
    Ok(sizes)
}

fn closure_find_root_v1(parent: &mut [usize; 4], value: usize) -> usize {
    if parent[value] != value {
        let next = parent[value];
        parent[value] = closure_find_root_v1(parent, next);
    }
    parent[value]
}

fn require_sorted_unique_or_empty_v1<T: Ord>(
    values: &[T],
    reason: &'static str,
) -> K2CompositionResultV1<()> {
    if values.is_empty() || values.windows(2).all(|pair| pair[0] < pair[1]) {
        Ok(())
    } else {
        Err(K2CompositionErrorV1::Invalid(reason))
    }
}

pub(crate) fn prediction_roots_v1(disposition: &K2UncertaintyRawProbeDispositionV1) -> Vec<String> {
    disposition
        .predictions
        .iter()
        .map(|prediction| prediction.prediction_root_sha256.clone())
        .collect()
}

pub(crate) fn checked_plan_budget_v1(
    first: u64,
    second: u64,
    limit: u64,
    reason: &'static str,
) -> K2CompositionResultV1<u64> {
    let total = first
        .checked_add(second)
        .ok_or(K2CompositionErrorV1::Invalid(reason))?;
    if total > limit {
        return Err(K2CompositionErrorV1::Invalid(reason));
    }
    Ok(total)
}

pub(crate) fn representative_root_set_v1(
    request: &K2UncertaintyClosurePlannerRequestV1,
) -> BTreeSet<String> {
    request
        .representatives
        .iter()
        .map(|value| value.probe.probe_root_sha256.clone())
        .collect()
}
