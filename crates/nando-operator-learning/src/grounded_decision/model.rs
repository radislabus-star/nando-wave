use std::collections::BTreeSet;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

pub const GROUNDED_TRANSITION_EPISODE_SCHEMA_V1: &str = "nando.grounded-transition-episode.v1";
pub const TYPED_GOAL_CONTRACT_SCHEMA_V1: &str = "nando.typed-goal-contract.v1";
pub const PRE_ACTION_GOAL_BINDING_RECEIPT_SCHEMA_V1: &str =
    "nando.pre-action-goal-binding-receipt.v1";
pub const AVAILABLE_ACTION_CONTRACTS_SCHEMA_V1: &str = "nando.available-action-contracts.v1";
pub const SELECTED_ACTION_SEQUENCE_SCHEMA_V1: &str = "nando.selected-action-sequence.v1";
pub const GOAL_SATISFACTION_RECEIPT_SCHEMA_V1: &str = "nando.goal-satisfaction-receipt.v1";
pub const GROUNDED_DECISION_EPISODE_SCHEMA_V1: &str = "nando.grounded-decision-episode.v1";

const MAX_PROVENANCE_ROOTS: usize = 32;
const MAX_AVAILABLE_ACTIONS: usize = 256;
const MAX_SELECTED_ACTIONS: usize = 16;
const MAX_TRANSITIONS_PER_DECISION: usize = 64;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GroundedEvidenceClassV1 {
    Natural,
    Lab,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionTerminalDispositionV1 {
    Positive,
    ApplicabilityNegative,
    HardContradiction,
    CensoredUnknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GroundedTransitionMaterialV1 {
    pub evidence_class: GroundedEvidenceClassV1,
    pub pre_action_state_root_sha256: String,
    pub observed_constraint_root_sha256: Option<String>,
    pub grounded_role_environment_root_sha256: String,
    pub k1_law_id_sha256: String,
    pub bundle_id_sha256: String,
    pub action_binding_root_sha256: String,
    pub verified_delta_root_sha256: String,
    pub post_action_state_root_sha256: String,
    pub independent_verifier_root_sha256: String,
    pub lineage_root_sha256: String,
    pub capture_generation_root_sha256: String,
    pub disposition: TransitionTerminalDispositionV1,
    pub provenance_roots_sha256: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GroundedTransitionEpisodeV1 {
    pub schema: String,
    pub episode_root_sha256: String,
    pub evidence_class: GroundedEvidenceClassV1,
    pub pre_action_state_root_sha256: String,
    pub observed_constraint_root_sha256: Option<String>,
    pub grounded_role_environment_root_sha256: String,
    pub k1_law_id_sha256: String,
    pub bundle_id_sha256: String,
    pub action_binding_root_sha256: String,
    pub verified_delta_root_sha256: String,
    pub post_action_state_root_sha256: String,
    pub independent_verifier_root_sha256: String,
    pub lineage_root_sha256: String,
    pub capture_generation_root_sha256: String,
    pub disposition: TransitionTerminalDispositionV1,
    pub provenance_root_sha256: String,
}

impl GroundedTransitionEpisodeV1 {
    pub fn seal(mut material: GroundedTransitionMaterialV1) -> Result<Self, &'static str> {
        canonical_roots(&mut material.provenance_roots_sha256, MAX_PROVENANCE_ROOTS)?;
        validate_transition_material(&material)?;
        let provenance_root_sha256 = canonical_json_sha256(&(
            "nando.grounded-transition-provenance.v1",
            &material.provenance_roots_sha256,
        ))?;
        let episode_root_sha256 = transition_episode_root(&material, &provenance_root_sha256)?;
        let episode = Self {
            schema: GROUNDED_TRANSITION_EPISODE_SCHEMA_V1.to_owned(),
            episode_root_sha256,
            evidence_class: material.evidence_class,
            pre_action_state_root_sha256: material.pre_action_state_root_sha256,
            observed_constraint_root_sha256: material.observed_constraint_root_sha256,
            grounded_role_environment_root_sha256: material.grounded_role_environment_root_sha256,
            k1_law_id_sha256: material.k1_law_id_sha256,
            bundle_id_sha256: material.bundle_id_sha256,
            action_binding_root_sha256: material.action_binding_root_sha256,
            verified_delta_root_sha256: material.verified_delta_root_sha256,
            post_action_state_root_sha256: material.post_action_state_root_sha256,
            independent_verifier_root_sha256: material.independent_verifier_root_sha256,
            lineage_root_sha256: material.lineage_root_sha256,
            capture_generation_root_sha256: material.capture_generation_root_sha256,
            disposition: material.disposition,
            provenance_root_sha256,
        };
        episode.validate()?;
        Ok(episode)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != GROUNDED_TRANSITION_EPISODE_SCHEMA_V1
            || !valid_nonzero_sha256(&self.episode_root_sha256)
            || !valid_optional_root(self.observed_constraint_root_sha256.as_deref())
            || !all_roots_valid([
                self.pre_action_state_root_sha256.as_str(),
                self.grounded_role_environment_root_sha256.as_str(),
                self.k1_law_id_sha256.as_str(),
                self.bundle_id_sha256.as_str(),
                self.action_binding_root_sha256.as_str(),
                self.verified_delta_root_sha256.as_str(),
                self.post_action_state_root_sha256.as_str(),
                self.independent_verifier_root_sha256.as_str(),
                self.lineage_root_sha256.as_str(),
                self.capture_generation_root_sha256.as_str(),
                self.provenance_root_sha256.as_str(),
            ])
        {
            return Err("grounded_transition_episode_invalid");
        }
        let material = GroundedTransitionMaterialV1 {
            evidence_class: self.evidence_class,
            pre_action_state_root_sha256: self.pre_action_state_root_sha256.clone(),
            observed_constraint_root_sha256: self.observed_constraint_root_sha256.clone(),
            grounded_role_environment_root_sha256: self
                .grounded_role_environment_root_sha256
                .clone(),
            k1_law_id_sha256: self.k1_law_id_sha256.clone(),
            bundle_id_sha256: self.bundle_id_sha256.clone(),
            action_binding_root_sha256: self.action_binding_root_sha256.clone(),
            verified_delta_root_sha256: self.verified_delta_root_sha256.clone(),
            post_action_state_root_sha256: self.post_action_state_root_sha256.clone(),
            independent_verifier_root_sha256: self.independent_verifier_root_sha256.clone(),
            lineage_root_sha256: self.lineage_root_sha256.clone(),
            capture_generation_root_sha256: self.capture_generation_root_sha256.clone(),
            disposition: self.disposition,
            provenance_roots_sha256: Vec::new(),
        };
        if transition_episode_root(&material, &self.provenance_root_sha256)?
            != self.episode_root_sha256
        {
            return Err("grounded_transition_episode_root_mismatch");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypedGoalContractV1 {
    pub schema: String,
    pub goal_contract_root_sha256: String,
    pub pre_action_goal_evidence_root_sha256: String,
    pub typed_success_predicate_root_sha256: String,
    pub outcome_horizon_contract_root_sha256: String,
    pub observation_mask_root_sha256: String,
    pub feature_exclusion_root_sha256: String,
    pub independent_goal_verifier_root_sha256: String,
    pub binder_schema_root_sha256: String,
    pub frozen_at_sequence: u64,
}

impl TypedGoalContractV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        pre_action_goal_evidence_root_sha256: String,
        typed_success_predicate_root_sha256: String,
        outcome_horizon_contract_root_sha256: String,
        observation_mask_root_sha256: String,
        feature_exclusion_root_sha256: String,
        independent_goal_verifier_root_sha256: String,
        binder_schema_root_sha256: String,
        frozen_at_sequence: u64,
    ) -> Result<Self, &'static str> {
        let mut contract = Self {
            schema: TYPED_GOAL_CONTRACT_SCHEMA_V1.to_owned(),
            goal_contract_root_sha256: String::new(),
            pre_action_goal_evidence_root_sha256,
            typed_success_predicate_root_sha256,
            outcome_horizon_contract_root_sha256,
            observation_mask_root_sha256,
            feature_exclusion_root_sha256,
            independent_goal_verifier_root_sha256,
            binder_schema_root_sha256,
            frozen_at_sequence,
        };
        contract.goal_contract_root_sha256 = contract.expected_root()?;
        contract.validate()?;
        Ok(contract)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != TYPED_GOAL_CONTRACT_SCHEMA_V1
            || self.frozen_at_sequence == 0
            || !all_roots_valid([
                self.goal_contract_root_sha256.as_str(),
                self.pre_action_goal_evidence_root_sha256.as_str(),
                self.typed_success_predicate_root_sha256.as_str(),
                self.outcome_horizon_contract_root_sha256.as_str(),
                self.observation_mask_root_sha256.as_str(),
                self.feature_exclusion_root_sha256.as_str(),
                self.independent_goal_verifier_root_sha256.as_str(),
                self.binder_schema_root_sha256.as_str(),
            ])
            || self.expected_root()? != self.goal_contract_root_sha256
        {
            return Err("typed_goal_contract_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            TYPED_GOAL_CONTRACT_SCHEMA_V1,
            self.pre_action_goal_evidence_root_sha256.as_str(),
            self.typed_success_predicate_root_sha256.as_str(),
            self.outcome_horizon_contract_root_sha256.as_str(),
            self.observation_mask_root_sha256.as_str(),
            self.feature_exclusion_root_sha256.as_str(),
            self.independent_goal_verifier_root_sha256.as_str(),
            self.binder_schema_root_sha256.as_str(),
            self.frozen_at_sequence,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreActionGoalBindingReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub goal_contract_root_sha256: String,
    pub pre_action_observation_root_sha256: String,
    pub independent_binder_root_sha256: String,
    pub frozen_at_sequence: u64,
    pub action_selection_not_before_sequence: u64,
}

impl PreActionGoalBindingReceiptV1 {
    pub fn seal(
        goal_contract: &TypedGoalContractV1,
        pre_action_observation_root_sha256: String,
        independent_binder_root_sha256: String,
        action_selection_not_before_sequence: u64,
    ) -> Result<Self, &'static str> {
        goal_contract.validate()?;
        let mut receipt = Self {
            schema: PRE_ACTION_GOAL_BINDING_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            goal_contract_root_sha256: goal_contract.goal_contract_root_sha256.clone(),
            pre_action_observation_root_sha256,
            independent_binder_root_sha256,
            frozen_at_sequence: goal_contract.frozen_at_sequence,
            action_selection_not_before_sequence,
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != PRE_ACTION_GOAL_BINDING_RECEIPT_SCHEMA_V1
            || self.frozen_at_sequence == 0
            || self.action_selection_not_before_sequence <= self.frozen_at_sequence
            || !all_roots_valid([
                self.receipt_root_sha256.as_str(),
                self.goal_contract_root_sha256.as_str(),
                self.pre_action_observation_root_sha256.as_str(),
                self.independent_binder_root_sha256.as_str(),
            ])
            || self.expected_root()? != self.receipt_root_sha256
        {
            return Err("pre_action_goal_binding_receipt_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            PRE_ACTION_GOAL_BINDING_RECEIPT_SCHEMA_V1,
            self.goal_contract_root_sha256.as_str(),
            self.pre_action_observation_root_sha256.as_str(),
            self.independent_binder_root_sha256.as_str(),
            self.frozen_at_sequence,
            self.action_selection_not_before_sequence,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AvailableActionContractsV1 {
    pub schema: String,
    pub contracts_root_sha256: String,
    pub action_contract_roots_sha256: Vec<String>,
    pub abstain_contract_root_sha256: String,
}

impl AvailableActionContractsV1 {
    pub fn seal(
        mut action_contract_roots_sha256: Vec<String>,
        abstain_contract_root_sha256: String,
    ) -> Result<Self, &'static str> {
        canonical_roots(&mut action_contract_roots_sha256, MAX_AVAILABLE_ACTIONS)?;
        if action_contract_roots_sha256.is_empty()
            || !valid_nonzero_sha256(&abstain_contract_root_sha256)
            || action_contract_roots_sha256.contains(&abstain_contract_root_sha256)
        {
            return Err("available_action_contracts_invalid");
        }
        let contracts_root_sha256 = canonical_json_sha256(&(
            AVAILABLE_ACTION_CONTRACTS_SCHEMA_V1,
            &action_contract_roots_sha256,
            abstain_contract_root_sha256.as_str(),
        ))?;
        Ok(Self {
            schema: AVAILABLE_ACTION_CONTRACTS_SCHEMA_V1.to_owned(),
            contracts_root_sha256,
            action_contract_roots_sha256,
            abstain_contract_root_sha256,
        })
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let sealed = Self::seal(
            self.action_contract_roots_sha256.clone(),
            self.abstain_contract_root_sha256.clone(),
        )?;
        if self.schema != AVAILABLE_ACTION_CONTRACTS_SCHEMA_V1
            || sealed.contracts_root_sha256 != self.contracts_root_sha256
            || sealed.action_contract_roots_sha256 != self.action_contract_roots_sha256
        {
            return Err("available_action_contracts_invalid");
        }
        Ok(())
    }

    pub fn has_meaningful_alternative(&self, selected: &SelectedActionSequenceV1) -> bool {
        self.validate().is_ok()
            && selected.validate().is_ok()
            && self
                .action_contract_roots_sha256
                .iter()
                .any(|root| !selected.action_contract_roots_sha256.contains(root))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedActionSequenceV1 {
    pub schema: String,
    pub sequence_root_sha256: String,
    pub action_contract_roots_sha256: Vec<String>,
}

impl SelectedActionSequenceV1 {
    pub fn seal(action_contract_roots_sha256: Vec<String>) -> Result<Self, &'static str> {
        if action_contract_roots_sha256.is_empty()
            || action_contract_roots_sha256.len() > MAX_SELECTED_ACTIONS
            || action_contract_roots_sha256
                .iter()
                .any(|root| !valid_nonzero_sha256(root))
        {
            return Err("selected_action_sequence_invalid");
        }
        let sequence_root_sha256 = canonical_json_sha256(&(
            SELECTED_ACTION_SEQUENCE_SCHEMA_V1,
            &action_contract_roots_sha256,
        ))?;
        Ok(Self {
            schema: SELECTED_ACTION_SEQUENCE_SCHEMA_V1.to_owned(),
            sequence_root_sha256,
            action_contract_roots_sha256,
        })
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let sealed = Self::seal(self.action_contract_roots_sha256.clone())?;
        if self.schema != SELECTED_ACTION_SEQUENCE_SCHEMA_V1
            || sealed.sequence_root_sha256 != self.sequence_root_sha256
        {
            return Err("selected_action_sequence_invalid");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GoalSatisfactionReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub goal_contract_root_sha256: String,
    pub outcome_horizon_contract_root_sha256: String,
    pub observed_consequence_root_sha256: String,
    pub independent_verifier_root_sha256: String,
    pub satisfied: bool,
}

impl GoalSatisfactionReceiptV1 {
    pub fn seal(
        goal_contract: &TypedGoalContractV1,
        observed_consequence_root_sha256: String,
        independent_verifier_root_sha256: String,
        satisfied: bool,
    ) -> Result<Self, &'static str> {
        goal_contract.validate()?;
        let mut receipt = Self {
            schema: GOAL_SATISFACTION_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            goal_contract_root_sha256: goal_contract.goal_contract_root_sha256.clone(),
            outcome_horizon_contract_root_sha256: goal_contract
                .outcome_horizon_contract_root_sha256
                .clone(),
            observed_consequence_root_sha256,
            independent_verifier_root_sha256,
            satisfied,
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != GOAL_SATISFACTION_RECEIPT_SCHEMA_V1
            || !all_roots_valid([
                self.receipt_root_sha256.as_str(),
                self.goal_contract_root_sha256.as_str(),
                self.outcome_horizon_contract_root_sha256.as_str(),
                self.observed_consequence_root_sha256.as_str(),
                self.independent_verifier_root_sha256.as_str(),
            ])
            || self.expected_root()? != self.receipt_root_sha256
        {
            return Err("goal_satisfaction_receipt_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            GOAL_SATISFACTION_RECEIPT_SCHEMA_V1,
            self.goal_contract_root_sha256.as_str(),
            self.outcome_horizon_contract_root_sha256.as_str(),
            self.observed_consequence_root_sha256.as_str(),
            self.independent_verifier_root_sha256.as_str(),
            self.satisfied,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct GroundedDecisionMaterialV1 {
    pub evidence_class: GroundedEvidenceClassV1,
    pub pre_action_observation_root_sha256: String,
    pub goal_contract: TypedGoalContractV1,
    pub goal_binding_receipt: PreActionGoalBindingReceiptV1,
    pub constraint_contract_root_sha256: String,
    pub available_actions: AvailableActionContractsV1,
    pub selected_action_sequence: SelectedActionSequenceV1,
    pub transitions: Vec<GroundedTransitionEpisodeV1>,
    pub goal_satisfaction_receipt: GoalSatisfactionReceiptV1,
    pub alternative_probe_manifest_root_sha256: Option<String>,
    pub independent_verifier_root_sha256: String,
    pub lineage_root_sha256: String,
    pub capture_generation_root_sha256: String,
    pub disposition: TransitionTerminalDispositionV1,
    pub provenance_roots_sha256: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GroundedDecisionEpisodeV1 {
    pub schema: String,
    pub decision_episode_root_sha256: String,
    pub evidence_class: GroundedEvidenceClassV1,
    pub pre_action_observation_root_sha256: String,
    pub typed_goal_contract_root_sha256: String,
    pub goal_binding_receipt_root_sha256: String,
    pub constraint_contract_root_sha256: String,
    pub observation_mask_root_sha256: String,
    pub available_action_contracts_root_sha256: String,
    pub selected_action_or_sequence_root_sha256: String,
    pub frozen_outcome_horizon_contract_root_sha256: String,
    pub transition_episode_roots_sha256: Vec<String>,
    pub verified_delta_sequence_root_sha256: String,
    pub goal_satisfaction_receipt_root_sha256: String,
    pub alternative_probe_manifest_root_sha256: Option<String>,
    pub independent_verifier_root_sha256: String,
    pub lineage_root_sha256: String,
    pub capture_generation_root_sha256: String,
    pub disposition: TransitionTerminalDispositionV1,
    pub provenance_root_sha256: String,
}

#[derive(Serialize)]
struct GroundedDecisionEpisodeDigestV1<'a> {
    schema: &'static str,
    evidence_class: GroundedEvidenceClassV1,
    pre_action_observation_root_sha256: &'a str,
    typed_goal_contract_root_sha256: &'a str,
    goal_binding_receipt_root_sha256: &'a str,
    constraint_contract_root_sha256: &'a str,
    observation_mask_root_sha256: &'a str,
    available_action_contracts_root_sha256: &'a str,
    selected_action_or_sequence_root_sha256: &'a str,
    frozen_outcome_horizon_contract_root_sha256: &'a str,
    transition_episode_roots_sha256: &'a [String],
    verified_delta_sequence_root_sha256: &'a str,
    goal_satisfaction_receipt_root_sha256: &'a str,
    alternative_probe_manifest_root_sha256: Option<&'a str>,
    independent_verifier_root_sha256: &'a str,
    lineage_root_sha256: &'a str,
    capture_generation_root_sha256: &'a str,
    disposition: TransitionTerminalDispositionV1,
    provenance_root_sha256: &'a str,
}

impl GroundedDecisionEpisodeV1 {
    pub fn seal(mut material: GroundedDecisionMaterialV1) -> Result<Self, &'static str> {
        validate_decision_material(&material)?;
        canonical_roots(&mut material.provenance_roots_sha256, MAX_PROVENANCE_ROOTS)?;
        let transition_episode_roots_sha256 = material
            .transitions
            .iter()
            .map(|transition| transition.episode_root_sha256.clone())
            .collect::<Vec<_>>();
        let verified_delta_roots_sha256 = material
            .transitions
            .iter()
            .map(|transition| transition.verified_delta_root_sha256.clone())
            .collect::<Vec<_>>();
        let verified_delta_sequence_root_sha256 = canonical_json_sha256(&(
            "nando.verified-delta-sequence.v1",
            &verified_delta_roots_sha256,
        ))?;
        let provenance_root_sha256 = canonical_json_sha256(&(
            "nando.grounded-decision-provenance.v1",
            &material.provenance_roots_sha256,
        ))?;
        let mut episode = Self {
            schema: GROUNDED_DECISION_EPISODE_SCHEMA_V1.to_owned(),
            decision_episode_root_sha256: String::new(),
            evidence_class: material.evidence_class,
            pre_action_observation_root_sha256: material.pre_action_observation_root_sha256,
            typed_goal_contract_root_sha256: material.goal_contract.goal_contract_root_sha256,
            goal_binding_receipt_root_sha256: material.goal_binding_receipt.receipt_root_sha256,
            constraint_contract_root_sha256: material.constraint_contract_root_sha256,
            observation_mask_root_sha256: material.goal_contract.observation_mask_root_sha256,
            available_action_contracts_root_sha256: material
                .available_actions
                .contracts_root_sha256,
            selected_action_or_sequence_root_sha256: material
                .selected_action_sequence
                .sequence_root_sha256,
            frozen_outcome_horizon_contract_root_sha256: material
                .goal_contract
                .outcome_horizon_contract_root_sha256,
            transition_episode_roots_sha256,
            verified_delta_sequence_root_sha256,
            goal_satisfaction_receipt_root_sha256: material
                .goal_satisfaction_receipt
                .receipt_root_sha256,
            alternative_probe_manifest_root_sha256: material.alternative_probe_manifest_root_sha256,
            independent_verifier_root_sha256: material.independent_verifier_root_sha256,
            lineage_root_sha256: material.lineage_root_sha256,
            capture_generation_root_sha256: material.capture_generation_root_sha256,
            disposition: material.disposition,
            provenance_root_sha256,
        };
        episode.decision_episode_root_sha256 = episode.expected_root()?;
        episode.validate()?;
        Ok(episode)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != GROUNDED_DECISION_EPISODE_SCHEMA_V1
            || self.transition_episode_roots_sha256.is_empty()
            || self.transition_episode_roots_sha256.len() > MAX_TRANSITIONS_PER_DECISION
            || !valid_optional_root(self.alternative_probe_manifest_root_sha256.as_deref())
            || self
                .transition_episode_roots_sha256
                .iter()
                .any(|root| !valid_nonzero_sha256(root))
            || !all_roots_valid([
                self.decision_episode_root_sha256.as_str(),
                self.pre_action_observation_root_sha256.as_str(),
                self.typed_goal_contract_root_sha256.as_str(),
                self.goal_binding_receipt_root_sha256.as_str(),
                self.constraint_contract_root_sha256.as_str(),
                self.observation_mask_root_sha256.as_str(),
                self.available_action_contracts_root_sha256.as_str(),
                self.selected_action_or_sequence_root_sha256.as_str(),
                self.frozen_outcome_horizon_contract_root_sha256.as_str(),
                self.verified_delta_sequence_root_sha256.as_str(),
                self.goal_satisfaction_receipt_root_sha256.as_str(),
                self.independent_verifier_root_sha256.as_str(),
                self.lineage_root_sha256.as_str(),
                self.capture_generation_root_sha256.as_str(),
                self.provenance_root_sha256.as_str(),
            ])
            || self.expected_root()? != self.decision_episode_root_sha256
        {
            return Err("grounded_decision_episode_invalid");
        }
        if self.evidence_class == GroundedEvidenceClassV1::Natural
            && self.alternative_probe_manifest_root_sha256.is_some()
        {
            return Err("natural_decision_episode_lab_probe_forbidden");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&GroundedDecisionEpisodeDigestV1 {
            schema: GROUNDED_DECISION_EPISODE_SCHEMA_V1,
            evidence_class: self.evidence_class,
            pre_action_observation_root_sha256: &self.pre_action_observation_root_sha256,
            typed_goal_contract_root_sha256: &self.typed_goal_contract_root_sha256,
            goal_binding_receipt_root_sha256: &self.goal_binding_receipt_root_sha256,
            constraint_contract_root_sha256: &self.constraint_contract_root_sha256,
            observation_mask_root_sha256: &self.observation_mask_root_sha256,
            available_action_contracts_root_sha256: &self.available_action_contracts_root_sha256,
            selected_action_or_sequence_root_sha256: &self.selected_action_or_sequence_root_sha256,
            frozen_outcome_horizon_contract_root_sha256: &self
                .frozen_outcome_horizon_contract_root_sha256,
            transition_episode_roots_sha256: &self.transition_episode_roots_sha256,
            verified_delta_sequence_root_sha256: &self.verified_delta_sequence_root_sha256,
            goal_satisfaction_receipt_root_sha256: &self.goal_satisfaction_receipt_root_sha256,
            alternative_probe_manifest_root_sha256: self
                .alternative_probe_manifest_root_sha256
                .as_deref(),
            independent_verifier_root_sha256: &self.independent_verifier_root_sha256,
            lineage_root_sha256: &self.lineage_root_sha256,
            capture_generation_root_sha256: &self.capture_generation_root_sha256,
            disposition: self.disposition,
            provenance_root_sha256: &self.provenance_root_sha256,
        })
    }
}

fn validate_transition_material(
    material: &GroundedTransitionMaterialV1,
) -> Result<(), &'static str> {
    if !valid_optional_root(material.observed_constraint_root_sha256.as_deref())
        || !all_roots_valid([
            material.pre_action_state_root_sha256.as_str(),
            material.grounded_role_environment_root_sha256.as_str(),
            material.k1_law_id_sha256.as_str(),
            material.bundle_id_sha256.as_str(),
            material.action_binding_root_sha256.as_str(),
            material.verified_delta_root_sha256.as_str(),
            material.post_action_state_root_sha256.as_str(),
            material.independent_verifier_root_sha256.as_str(),
            material.lineage_root_sha256.as_str(),
            material.capture_generation_root_sha256.as_str(),
        ])
        || material.provenance_roots_sha256.is_empty()
    {
        return Err("grounded_transition_material_invalid");
    }
    Ok(())
}

fn transition_episode_root(
    material: &GroundedTransitionMaterialV1,
    provenance_root_sha256: &str,
) -> Result<String, &'static str> {
    canonical_json_sha256(&(
        GROUNDED_TRANSITION_EPISODE_SCHEMA_V1,
        material.evidence_class,
        material.pre_action_state_root_sha256.as_str(),
        material.observed_constraint_root_sha256.as_deref(),
        material.grounded_role_environment_root_sha256.as_str(),
        material.k1_law_id_sha256.as_str(),
        material.bundle_id_sha256.as_str(),
        material.action_binding_root_sha256.as_str(),
        material.verified_delta_root_sha256.as_str(),
        material.post_action_state_root_sha256.as_str(),
        material.independent_verifier_root_sha256.as_str(),
        material.lineage_root_sha256.as_str(),
        material.capture_generation_root_sha256.as_str(),
        material.disposition,
        provenance_root_sha256,
    ))
}

fn validate_decision_material(material: &GroundedDecisionMaterialV1) -> Result<(), &'static str> {
    material.goal_contract.validate()?;
    material.goal_binding_receipt.validate()?;
    material.available_actions.validate()?;
    material.selected_action_sequence.validate()?;
    material.goal_satisfaction_receipt.validate()?;
    if material.transitions.is_empty()
        || material.transitions.len() > MAX_TRANSITIONS_PER_DECISION
        || material
            .transitions
            .iter()
            .any(|transition| transition.validate().is_err())
        || material
            .transitions
            .iter()
            .any(|transition| transition.evidence_class != material.evidence_class)
        || !all_roots_valid([
            material.pre_action_observation_root_sha256.as_str(),
            material.constraint_contract_root_sha256.as_str(),
            material.independent_verifier_root_sha256.as_str(),
            material.lineage_root_sha256.as_str(),
            material.capture_generation_root_sha256.as_str(),
        ])
        || !valid_optional_root(material.alternative_probe_manifest_root_sha256.as_deref())
        || material.goal_binding_receipt.goal_contract_root_sha256
            != material.goal_contract.goal_contract_root_sha256
        || material
            .goal_binding_receipt
            .pre_action_observation_root_sha256
            != material.pre_action_observation_root_sha256
        || material.goal_satisfaction_receipt.goal_contract_root_sha256
            != material.goal_contract.goal_contract_root_sha256
        || material
            .goal_satisfaction_receipt
            .outcome_horizon_contract_root_sha256
            != material.goal_contract.outcome_horizon_contract_root_sha256
        || !selected_actions_available(
            &material.available_actions,
            &material.selected_action_sequence,
        )
        || !material
            .available_actions
            .has_meaningful_alternative(&material.selected_action_sequence)
        || material
            .transitions
            .iter()
            .any(|transition| transition.lineage_root_sha256 != material.lineage_root_sha256)
        || material.transitions.iter().any(|transition| {
            transition.capture_generation_root_sha256 != material.capture_generation_root_sha256
        })
        || material.provenance_roots_sha256.is_empty()
    {
        return Err("grounded_decision_material_invalid");
    }
    if material.evidence_class == GroundedEvidenceClassV1::Natural
        && material.alternative_probe_manifest_root_sha256.is_some()
    {
        return Err("natural_decision_episode_lab_probe_forbidden");
    }
    Ok(())
}

fn selected_actions_available(
    available: &AvailableActionContractsV1,
    selected: &SelectedActionSequenceV1,
) -> bool {
    selected.action_contract_roots_sha256.iter().all(|root| {
        root == &available.abstain_contract_root_sha256
            || available.action_contract_roots_sha256.contains(root)
    })
}

fn canonical_roots(roots: &mut Vec<String>, max: usize) -> Result<(), &'static str> {
    if roots.is_empty() || roots.len() > max || roots.iter().any(|root| !valid_nonzero_sha256(root))
    {
        return Err("root_set_invalid");
    }
    roots.sort();
    roots.dedup();
    if roots.is_empty() || roots.len() > max {
        return Err("root_set_invalid");
    }
    Ok(())
}

fn all_roots_valid<'a>(roots: impl IntoIterator<Item = &'a str>) -> bool {
    roots.into_iter().all(valid_nonzero_sha256)
}

fn valid_optional_root(root: Option<&str>) -> bool {
    root.is_none_or(valid_nonzero_sha256)
}

pub(crate) fn distinct_valid_roots(roots: impl IntoIterator<Item = String>) -> BTreeSet<String> {
    roots
        .into_iter()
        .filter(|root| valid_nonzero_sha256(root))
        .collect()
}
