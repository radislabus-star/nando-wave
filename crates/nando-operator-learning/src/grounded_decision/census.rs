use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::{
    AvailableActionContractsV1, GoalSatisfactionReceiptV1, GroundedDecisionEpisodeV1,
    GroundedDecisionMaterialV1, GroundedTransitionEpisodeV1, PreActionGoalBindingReceiptV1,
    SelectedActionSequenceV1, TypedGoalContractV1, distinct_valid_roots,
};

pub const GROUNDED_TRANSITION_PROJECTION_SNAPSHOT_SCHEMA_V1: &str =
    "nando.grounded-transition-projection-snapshot.v1";
pub const GROUNDED_DECISION_CENSUS_SCHEMA_V1: &str = "nando.grounded-decision-census.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionProjectionCensorReasonV1 {
    InvalidSourceReceipt,
    MissingCertifiedK1Binding,
    MissingPreActionTopology,
    AmbiguousPreActionTopology,
    MissingTransportBinding,
    AmbiguousTransportBinding,
    MissingVerifiedOutcome,
    IdentityMismatch,
    ProvenanceFailure,
    CapacityExhausted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GroundedTransitionProjectionSnapshotV1 {
    pub schema: String,
    pub snapshot_root_sha256: String,
    pub source_snapshot_root_sha256: String,
    pub transition_rows_scanned: u64,
    pub certified_k1_rows: u64,
    pub transition_rows_projected: u64,
    pub transition_rows_censored: u64,
    pub censor_counts: BTreeMap<TransitionProjectionCensorReasonV1, u64>,
    pub transition_episode_set_root_sha256: String,
    pub episodes: Vec<GroundedTransitionEpisodeV1>,
    pub raw_payloads_persisted: bool,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

impl GroundedTransitionProjectionSnapshotV1 {
    pub fn seal(
        source_snapshot_root_sha256: String,
        transition_rows_scanned: u64,
        certified_k1_rows: u64,
        mut episodes: Vec<GroundedTransitionEpisodeV1>,
        censor_counts: BTreeMap<TransitionProjectionCensorReasonV1, u64>,
    ) -> Result<Self, &'static str> {
        if !valid_nonzero_sha256(&source_snapshot_root_sha256)
            || certified_k1_rows > transition_rows_scanned
            || episodes.iter().any(|episode| episode.validate().is_err())
        {
            return Err("grounded_transition_projection_input_invalid");
        }
        episodes.sort_by(|left, right| left.episode_root_sha256.cmp(&right.episode_root_sha256));
        if episodes
            .windows(2)
            .any(|pair| pair[0].episode_root_sha256 == pair[1].episode_root_sha256)
        {
            return Err("grounded_transition_projection_duplicate_episode");
        }
        let transition_rows_projected =
            u64::try_from(episodes.len()).map_err(|_| "grounded_transition_projection_count")?;
        let transition_rows_censored = censor_counts
            .values()
            .try_fold(0_u64, |total, count| total.checked_add(*count))
            .ok_or("grounded_transition_projection_count")?;
        if transition_rows_projected.saturating_add(transition_rows_censored)
            != transition_rows_scanned
        {
            return Err("grounded_transition_projection_denominator_mismatch");
        }
        let episode_roots = episodes
            .iter()
            .map(|episode| episode.episode_root_sha256.as_str())
            .collect::<Vec<_>>();
        let transition_episode_set_root_sha256 =
            canonical_json_sha256(&("nando.grounded-transition-episode-set.v1", &episode_roots))?;
        let snapshot_root_sha256 = projection_snapshot_root(
            &source_snapshot_root_sha256,
            transition_rows_scanned,
            certified_k1_rows,
            transition_rows_projected,
            transition_rows_censored,
            &censor_counts,
            &transition_episode_set_root_sha256,
        )?;
        let snapshot = Self {
            schema: GROUNDED_TRANSITION_PROJECTION_SNAPSHOT_SCHEMA_V1.to_owned(),
            snapshot_root_sha256,
            source_snapshot_root_sha256,
            transition_rows_scanned,
            certified_k1_rows,
            transition_rows_projected,
            transition_rows_censored,
            censor_counts,
            transition_episode_set_root_sha256,
            episodes,
            raw_payloads_persisted: false,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        Ok(snapshot)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != GROUNDED_TRANSITION_PROJECTION_SNAPSHOT_SCHEMA_V1
            || !valid_nonzero_sha256(&self.snapshot_root_sha256)
            || !valid_nonzero_sha256(&self.source_snapshot_root_sha256)
            || !valid_nonzero_sha256(&self.transition_episode_set_root_sha256)
            || self.raw_payloads_persisted
            || self.authority_ready
            || self.phase_mutation_allowed
        {
            return Err("grounded_transition_projection_snapshot_invalid");
        }
        let restored = Self::seal(
            self.source_snapshot_root_sha256.clone(),
            self.transition_rows_scanned,
            self.certified_k1_rows,
            self.episodes.clone(),
            self.censor_counts.clone(),
        )?;
        if restored.snapshot_root_sha256 != self.snapshot_root_sha256
            || restored.transition_episode_set_root_sha256
                != self.transition_episode_set_root_sha256
            || restored.transition_rows_projected != self.transition_rows_projected
            || restored.transition_rows_censored != self.transition_rows_censored
            || restored.episodes != self.episodes
        {
            return Err("grounded_transition_projection_snapshot_root_mismatch");
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct DecisionEvidenceSurfaceV1 {
    pub transition: GroundedTransitionEpisodeV1,
    pub goal_contract: Option<TypedGoalContractV1>,
    pub goal_binding_receipt: Option<PreActionGoalBindingReceiptV1>,
    pub constraint_contract_root_sha256: Option<String>,
    pub available_actions: Option<AvailableActionContractsV1>,
    pub selected_action_sequence: Option<SelectedActionSequenceV1>,
    pub goal_satisfaction_receipt: Option<GoalSatisfactionReceiptV1>,
    pub provenance_verified: bool,
}

impl DecisionEvidenceSurfaceV1 {
    #[must_use]
    pub fn dynamics_only(transition: GroundedTransitionEpisodeV1) -> Self {
        Self {
            transition,
            goal_contract: None,
            goal_binding_receipt: None,
            constraint_contract_root_sha256: Some(absent_constraint_contract_root_v1()),
            available_actions: None,
            selected_action_sequence: None,
            goal_satisfaction_receipt: None,
            provenance_verified: true,
        }
    }
}

#[must_use]
pub fn absent_constraint_contract_root_v1() -> String {
    canonical_json_sha256(&("nando.explicit-absent-constraint-contract.v1", false))
        .expect("static absent constraint contract serializes")
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionCensusBlockerV1 {
    MissingGoal,
    MissingAlternative,
    MissingHorizon,
    MissingSatisfaction,
    ProvenanceFailure,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GroundedDecisionCensusV1 {
    pub schema: String,
    pub report_root_sha256: String,
    pub transition_projection_root_sha256: String,
    pub transition_rows_scanned: u64,
    pub transition_rows_projected: u64,
    pub transition_rows_censored: u64,
    pub transition_censor_counts: BTreeMap<TransitionProjectionCensorReasonV1, u64>,
    pub goal_bound: u64,
    pub alternative_bearing: u64,
    pub horizon_bound: u64,
    pub satisfaction_verifiable: u64,
    pub dynamics_only: u64,
    pub decision_episodes: u64,
    pub distinct_transition_lineages: u64,
    pub distinct_decision_lineages: u64,
    pub lineage_independent_episodes: u64,
    pub blocker_counts: BTreeMap<DecisionCensusBlockerV1, u64>,
    pub decision_episode_set_root_sha256: String,
    pub decision_episode_roots_sha256: Vec<String>,
    pub verdict: String,
    pub blocker: String,
    pub model_training_allowed: bool,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Serialize)]
struct GroundedDecisionCensusDigestV1<'a> {
    schema: &'static str,
    transition_projection_root_sha256: &'a str,
    transition_rows_scanned: u64,
    transition_rows_projected: u64,
    transition_rows_censored: u64,
    transition_censor_counts: &'a BTreeMap<TransitionProjectionCensorReasonV1, u64>,
    goal_bound: u64,
    alternative_bearing: u64,
    horizon_bound: u64,
    satisfaction_verifiable: u64,
    dynamics_only: u64,
    decision_episodes: u64,
    distinct_transition_lineages: u64,
    distinct_decision_lineages: u64,
    lineage_independent_episodes: u64,
    blocker_counts: &'a BTreeMap<DecisionCensusBlockerV1, u64>,
    decision_episode_set_root_sha256: &'a str,
    verdict: &'a str,
    blocker: &'a str,
    model_training_allowed: bool,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

pub fn build_grounded_decision_census_v1(
    projection: &GroundedTransitionProjectionSnapshotV1,
    surfaces: Vec<DecisionEvidenceSurfaceV1>,
) -> Result<GroundedDecisionCensusV1, &'static str> {
    projection.validate()?;
    let projected_roots = projection
        .episodes
        .iter()
        .map(|episode| episode.episode_root_sha256.as_str())
        .collect::<BTreeSet<_>>();
    if surfaces.len() != projection.episodes.len()
        || surfaces.iter().any(|surface| {
            surface.transition.validate().is_err()
                || !projected_roots.contains(surface.transition.episode_root_sha256.as_str())
        })
    {
        return Err("grounded_decision_census_surface_mismatch");
    }
    let surface_roots = surfaces
        .iter()
        .map(|surface| surface.transition.episode_root_sha256.as_str())
        .collect::<BTreeSet<_>>();
    if surface_roots.len() != surfaces.len() {
        return Err("grounded_decision_census_duplicate_surface");
    }

    let mut blocker_counts = BTreeMap::new();
    let mut goal_bound = 0_u64;
    let mut alternative_bearing = 0_u64;
    let mut horizon_bound = 0_u64;
    let mut satisfaction_verifiable = 0_u64;
    let mut dynamics_only = 0_u64;
    let mut decision_episodes = Vec::new();

    for surface in surfaces {
        let goal = valid_goal_binding(&surface);
        let horizon = surface.goal_contract.as_ref().is_some_and(|contract| {
            contract.validate().is_ok()
                && valid_nonzero_sha256(&contract.outcome_horizon_contract_root_sha256)
        });
        let alternative = valid_alternative_surface(&surface);
        let satisfaction = valid_satisfaction_surface(&surface);
        let provenance = valid_surface_provenance(&surface);

        if goal {
            goal_bound = goal_bound.saturating_add(1);
        } else {
            increment(&mut blocker_counts, DecisionCensusBlockerV1::MissingGoal);
        }
        if horizon {
            horizon_bound = horizon_bound.saturating_add(1);
        } else {
            increment(&mut blocker_counts, DecisionCensusBlockerV1::MissingHorizon);
        }
        if alternative {
            alternative_bearing = alternative_bearing.saturating_add(1);
        } else {
            increment(
                &mut blocker_counts,
                DecisionCensusBlockerV1::MissingAlternative,
            );
        }
        if satisfaction {
            satisfaction_verifiable = satisfaction_verifiable.saturating_add(1);
        } else {
            increment(
                &mut blocker_counts,
                DecisionCensusBlockerV1::MissingSatisfaction,
            );
        }
        if !provenance {
            increment(
                &mut blocker_counts,
                DecisionCensusBlockerV1::ProvenanceFailure,
            );
        }
        if !goal || !alternative {
            dynamics_only = dynamics_only.saturating_add(1);
        }

        if goal && horizon && alternative && satisfaction && provenance {
            decision_episodes.push(build_decision_episode(surface)?);
        }
    }

    decision_episodes.sort_by(|left, right| {
        left.decision_episode_root_sha256
            .cmp(&right.decision_episode_root_sha256)
    });
    let decision_episode_roots_sha256 = decision_episodes
        .iter()
        .map(|episode| episode.decision_episode_root_sha256.clone())
        .collect::<Vec<_>>();
    let decision_episode_set_root_sha256 = canonical_json_sha256(&(
        "nando.grounded-decision-episode-set.v1",
        &decision_episode_roots_sha256,
    ))?;
    let transition_lineages = distinct_valid_roots(
        projection
            .episodes
            .iter()
            .map(|episode| episode.lineage_root_sha256.clone()),
    );
    let decision_lineages = distinct_valid_roots(
        decision_episodes
            .iter()
            .map(|episode| episode.lineage_root_sha256.clone()),
    );
    let distinct_transition_lineages =
        u64::try_from(transition_lineages.len()).map_err(|_| "grounded_decision_census_count")?;
    let distinct_decision_lineages =
        u64::try_from(decision_lineages.len()).map_err(|_| "grounded_decision_census_count")?;
    let decision_episode_count =
        u64::try_from(decision_episodes.len()).map_err(|_| "grounded_decision_census_count")?;
    let lineage_independent_episodes = if distinct_decision_lineages >= 2 {
        decision_episode_count
    } else {
        0
    };
    let (verdict, blocker) = if decision_episode_count == 0 {
        (
            "EMPTY_DECISION_SURFACE".to_owned(),
            leading_blocker(&blocker_counts).to_owned(),
        )
    } else if distinct_decision_lineages < 2 {
        (
            "DECISION_SURFACE_LINEAGE_BLOCKED".to_owned(),
            "insufficient_independent_lineages".to_owned(),
        )
    } else {
        ("READY_FOR_BASELINES".to_owned(), String::new())
    };
    let model_training_allowed = verdict == "READY_FOR_BASELINES";
    let report_root_sha256 = decision_census_root(
        projection,
        goal_bound,
        alternative_bearing,
        horizon_bound,
        satisfaction_verifiable,
        dynamics_only,
        decision_episode_count,
        distinct_transition_lineages,
        distinct_decision_lineages,
        lineage_independent_episodes,
        &blocker_counts,
        &decision_episode_set_root_sha256,
        &verdict,
        &blocker,
        model_training_allowed,
    )?;
    let report = GroundedDecisionCensusV1 {
        schema: GROUNDED_DECISION_CENSUS_SCHEMA_V1.to_owned(),
        report_root_sha256,
        transition_projection_root_sha256: projection.snapshot_root_sha256.clone(),
        transition_rows_scanned: projection.transition_rows_scanned,
        transition_rows_projected: projection.transition_rows_projected,
        transition_rows_censored: projection.transition_rows_censored,
        transition_censor_counts: projection.censor_counts.clone(),
        goal_bound,
        alternative_bearing,
        horizon_bound,
        satisfaction_verifiable,
        dynamics_only,
        decision_episodes: decision_episode_count,
        distinct_transition_lineages,
        distinct_decision_lineages,
        lineage_independent_episodes,
        blocker_counts,
        decision_episode_set_root_sha256,
        decision_episode_roots_sha256,
        verdict,
        blocker,
        model_training_allowed,
        authority_ready: false,
        phase_mutation_allowed: false,
    };
    report.validate()?;
    Ok(report)
}

impl GroundedDecisionCensusV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != GROUNDED_DECISION_CENSUS_SCHEMA_V1
            || !valid_nonzero_sha256(&self.report_root_sha256)
            || !valid_nonzero_sha256(&self.transition_projection_root_sha256)
            || !valid_nonzero_sha256(&self.decision_episode_set_root_sha256)
            || self.transition_rows_projected > self.transition_rows_scanned
            || self
                .transition_rows_projected
                .saturating_add(self.transition_rows_censored)
                != self.transition_rows_scanned
            || self.transition_censor_counts.values().copied().sum::<u64>()
                != self.transition_rows_censored
            || self.goal_bound > self.transition_rows_projected
            || self.alternative_bearing > self.transition_rows_projected
            || self.horizon_bound > self.transition_rows_projected
            || self.satisfaction_verifiable > self.transition_rows_projected
            || self.dynamics_only > self.transition_rows_projected
            || self.decision_episodes
                != u64::try_from(self.decision_episode_roots_sha256.len()).unwrap_or(u64::MAX)
            || self.lineage_independent_episodes > self.decision_episodes
            || self.model_training_allowed != (self.verdict == "READY_FOR_BASELINES")
            || self.authority_ready
            || self.phase_mutation_allowed
            || self
                .decision_episode_roots_sha256
                .iter()
                .any(|root| !valid_nonzero_sha256(root))
            || canonical_json_sha256(&(
                "nando.grounded-decision-episode-set.v1",
                &self.decision_episode_roots_sha256,
            ))? != self.decision_episode_set_root_sha256
            || self.expected_root()? != self.report_root_sha256
        {
            return Err("grounded_decision_census_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&GroundedDecisionCensusDigestV1 {
            schema: GROUNDED_DECISION_CENSUS_SCHEMA_V1,
            transition_projection_root_sha256: &self.transition_projection_root_sha256,
            transition_rows_scanned: self.transition_rows_scanned,
            transition_rows_projected: self.transition_rows_projected,
            transition_rows_censored: self.transition_rows_censored,
            transition_censor_counts: &self.transition_censor_counts,
            goal_bound: self.goal_bound,
            alternative_bearing: self.alternative_bearing,
            horizon_bound: self.horizon_bound,
            satisfaction_verifiable: self.satisfaction_verifiable,
            dynamics_only: self.dynamics_only,
            decision_episodes: self.decision_episodes,
            distinct_transition_lineages: self.distinct_transition_lineages,
            distinct_decision_lineages: self.distinct_decision_lineages,
            lineage_independent_episodes: self.lineage_independent_episodes,
            blocker_counts: &self.blocker_counts,
            decision_episode_set_root_sha256: &self.decision_episode_set_root_sha256,
            verdict: &self.verdict,
            blocker: &self.blocker,
            model_training_allowed: self.model_training_allowed,
            authority_ready: self.authority_ready,
            phase_mutation_allowed: self.phase_mutation_allowed,
        })
    }
}

fn valid_goal_binding(surface: &DecisionEvidenceSurfaceV1) -> bool {
    let (Some(goal), Some(binding)) = (
        surface.goal_contract.as_ref(),
        surface.goal_binding_receipt.as_ref(),
    ) else {
        return false;
    };
    goal.validate().is_ok()
        && binding.validate().is_ok()
        && binding.goal_contract_root_sha256 == goal.goal_contract_root_sha256
        && binding.pre_action_observation_root_sha256
            == surface.transition.pre_action_state_root_sha256
}

fn valid_alternative_surface(surface: &DecisionEvidenceSurfaceV1) -> bool {
    let (Some(available), Some(selected)) = (
        surface.available_actions.as_ref(),
        surface.selected_action_sequence.as_ref(),
    ) else {
        return false;
    };
    available.validate().is_ok()
        && selected.validate().is_ok()
        && selected.action_contract_roots_sha256.iter().all(|root| {
            root == &available.abstain_contract_root_sha256
                || available.action_contract_roots_sha256.contains(root)
        })
        && available.has_meaningful_alternative(selected)
}

fn valid_satisfaction_surface(surface: &DecisionEvidenceSurfaceV1) -> bool {
    let (Some(goal), Some(receipt)) = (
        surface.goal_contract.as_ref(),
        surface.goal_satisfaction_receipt.as_ref(),
    ) else {
        return false;
    };
    goal.validate().is_ok()
        && receipt.validate().is_ok()
        && receipt.goal_contract_root_sha256 == goal.goal_contract_root_sha256
        && receipt.outcome_horizon_contract_root_sha256 == goal.outcome_horizon_contract_root_sha256
}

fn valid_surface_provenance(surface: &DecisionEvidenceSurfaceV1) -> bool {
    surface.provenance_verified
        && surface.transition.validate().is_ok()
        && surface
            .constraint_contract_root_sha256
            .as_deref()
            .is_some_and(valid_nonzero_sha256)
}

fn build_decision_episode(
    surface: DecisionEvidenceSurfaceV1,
) -> Result<GroundedDecisionEpisodeV1, &'static str> {
    let goal_contract = surface
        .goal_contract
        .ok_or("grounded_decision_goal_missing")?;
    let goal_binding_receipt = surface
        .goal_binding_receipt
        .ok_or("grounded_decision_goal_binding_missing")?;
    let constraint_contract_root_sha256 = surface
        .constraint_contract_root_sha256
        .ok_or("grounded_decision_constraint_missing")?;
    let available_actions = surface
        .available_actions
        .ok_or("grounded_decision_actions_missing")?;
    let selected_action_sequence = surface
        .selected_action_sequence
        .ok_or("grounded_decision_selected_action_missing")?;
    let goal_satisfaction_receipt = surface
        .goal_satisfaction_receipt
        .ok_or("grounded_decision_satisfaction_missing")?;
    let transition = surface.transition;
    let provenance_roots_sha256 = vec![
        transition.provenance_root_sha256.clone(),
        goal_contract.goal_contract_root_sha256.clone(),
        goal_binding_receipt.receipt_root_sha256.clone(),
        available_actions.contracts_root_sha256.clone(),
        selected_action_sequence.sequence_root_sha256.clone(),
        goal_satisfaction_receipt.receipt_root_sha256.clone(),
    ];
    GroundedDecisionEpisodeV1::seal(GroundedDecisionMaterialV1 {
        evidence_class: transition.evidence_class,
        pre_action_observation_root_sha256: transition.pre_action_state_root_sha256.clone(),
        goal_contract,
        goal_binding_receipt,
        constraint_contract_root_sha256,
        available_actions,
        selected_action_sequence,
        transitions: vec![transition.clone()],
        independent_verifier_root_sha256: goal_satisfaction_receipt
            .independent_verifier_root_sha256
            .clone(),
        goal_satisfaction_receipt,
        alternative_probe_manifest_root_sha256: None,
        lineage_root_sha256: transition.lineage_root_sha256,
        capture_generation_root_sha256: transition.capture_generation_root_sha256,
        disposition: transition.disposition,
        provenance_roots_sha256,
    })
}

fn increment(
    counts: &mut BTreeMap<DecisionCensusBlockerV1, u64>,
    blocker: DecisionCensusBlockerV1,
) {
    let count = counts.entry(blocker).or_default();
    *count = count.saturating_add(1);
}

fn leading_blocker(counts: &BTreeMap<DecisionCensusBlockerV1, u64>) -> &'static str {
    let ordered = [
        (
            DecisionCensusBlockerV1::MissingGoal,
            "missing_pre_action_goal",
        ),
        (
            DecisionCensusBlockerV1::MissingAlternative,
            "missing_meaningful_alternative",
        ),
        (
            DecisionCensusBlockerV1::MissingHorizon,
            "missing_frozen_outcome_horizon",
        ),
        (
            DecisionCensusBlockerV1::MissingSatisfaction,
            "missing_verified_goal_satisfaction",
        ),
        (
            DecisionCensusBlockerV1::ProvenanceFailure,
            "decision_provenance_failure",
        ),
    ];
    let mut best = None;
    let mut best_count = 0_u64;
    for (candidate, reason) in ordered {
        let count = counts.get(&candidate).copied().unwrap_or(0);
        if best.is_none() || count > best_count {
            best = Some(reason);
            best_count = count;
        }
    }
    best.unwrap_or("transition_surface_empty")
}

#[allow(clippy::too_many_arguments)]
fn decision_census_root(
    projection: &GroundedTransitionProjectionSnapshotV1,
    goal_bound: u64,
    alternative_bearing: u64,
    horizon_bound: u64,
    satisfaction_verifiable: u64,
    dynamics_only: u64,
    decision_episodes: u64,
    distinct_transition_lineages: u64,
    distinct_decision_lineages: u64,
    lineage_independent_episodes: u64,
    blocker_counts: &BTreeMap<DecisionCensusBlockerV1, u64>,
    decision_episode_set_root_sha256: &str,
    verdict: &str,
    blocker: &str,
    model_training_allowed: bool,
) -> Result<String, &'static str> {
    canonical_json_sha256(&GroundedDecisionCensusDigestV1 {
        schema: GROUNDED_DECISION_CENSUS_SCHEMA_V1,
        transition_projection_root_sha256: &projection.snapshot_root_sha256,
        transition_rows_scanned: projection.transition_rows_scanned,
        transition_rows_projected: projection.transition_rows_projected,
        transition_rows_censored: projection.transition_rows_censored,
        transition_censor_counts: &projection.censor_counts,
        goal_bound,
        alternative_bearing,
        horizon_bound,
        satisfaction_verifiable,
        dynamics_only,
        decision_episodes,
        distinct_transition_lineages,
        distinct_decision_lineages,
        lineage_independent_episodes,
        blocker_counts,
        decision_episode_set_root_sha256,
        verdict,
        blocker,
        model_training_allowed,
        authority_ready: false,
        phase_mutation_allowed: false,
    })
}

fn projection_snapshot_root(
    source_snapshot_root_sha256: &str,
    transition_rows_scanned: u64,
    certified_k1_rows: u64,
    transition_rows_projected: u64,
    transition_rows_censored: u64,
    censor_counts: &BTreeMap<TransitionProjectionCensorReasonV1, u64>,
    transition_episode_set_root_sha256: &str,
) -> Result<String, &'static str> {
    canonical_json_sha256(&(
        GROUNDED_TRANSITION_PROJECTION_SNAPSHOT_SCHEMA_V1,
        source_snapshot_root_sha256,
        transition_rows_scanned,
        certified_k1_rows,
        transition_rows_projected,
        transition_rows_censored,
        censor_counts,
        transition_episode_set_root_sha256,
        false,
        false,
        false,
    ))
}
