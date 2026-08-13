use nando_operator_kernel::{canonical_json_bytes, canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use crate::multi_source::K1ConsequenceTypeV1;

use super::{
    AvailableActionContractsV1, GoalSatisfactionReceiptV1, PreActionGoalBindingReceiptV1,
    TypedGoalContractV1,
};

pub const TYPED_GOAL_PREDICATE_ARTIFACT_SCHEMA_V1: &str = "nando.typed-goal-predicate-artifact.v1";
pub const K1_ACTION_CONTRACT_PROJECTION_SCHEMA_V1: &str = "nando.k1-action-contract-projection.v1";
pub const OPAQUE_ACTION_EXECUTION_BINDING_SCHEMA_V1: &str =
    "nando.opaque-action-execution-binding.v1";
pub const DECISION_AUTHORITY_SNAPSHOT_SCHEMA_V1: &str = "nando.decision-authority-snapshot.v1";
pub const DECISION_CONTRACT_PRECOMMIT_SCHEMA_V1: &str = "nando.decision-contract-precommit.v1";
pub const DECISION_CONTRACT_PRECOMMIT_SCHEMA_V2: &str = "nando.decision-contract-precommit.v2";
pub const DECISION_CONTRACT_DURABILITY_RECEIPT_SCHEMA_V1: &str =
    "nando.decision-contract-durability-receipt.v1";
pub const SELECTED_ACTION_BINDING_RECEIPT_SCHEMA_V1: &str =
    "nando.selected-action-binding-receipt.v1";
pub const DURABLE_SELECTED_ACTION_BINDING_SCHEMA_V1: &str =
    "nando.durable-selected-action-binding.v1";
pub const DURABLE_GOAL_SATISFACTION_SCHEMA_V1: &str = "nando.durable-goal-satisfaction.v1";
pub const OPAQUE_ACTION_EXECUTION_BINDING_SET_SCHEMA_V1: &str =
    "nando.opaque-action-execution-binding-set.v1";

pub const MAX_TYPED_GOAL_PREDICATE_BYTES_V1: usize = 4 * 1024;
pub const MAX_DECISION_PRECOMMIT_BYTES_V1: usize = 32 * 1024;
pub const MAX_DECISION_ACTION_BINDINGS_V1: usize = 256;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GroundedDecisionShadowCensorV1 {
    CaptureDisabled,
    IneligibleTrafficProvenance,
    MissingExactGoal,
    GoalInputInvalid,
    AuthoritySnapshotUnavailable,
    AuthoritySnapshotMismatch,
    NoApplicableK1Action,
    ActionProjectionIncomplete,
    ActionCapacityExhausted,
    PrecommitSealFailed,
    PrecommitSyncFailed,
    SelectedActionNotK1,
    SelectedActionBindingFailed,
    SelectedActionSyncFailed,
    TerminalConsequenceUnavailable,
    IndependentVerifierUnavailable,
    GoalPredicateVerificationFailed,
    SatisfactionSyncFailed,
}

impl GroundedDecisionShadowCensorV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::CaptureDisabled => "CAPTURE_DISABLED",
            Self::IneligibleTrafficProvenance => "INELIGIBLE_TRAFFIC_PROVENANCE",
            Self::MissingExactGoal => "MISSING_EXACT_GOAL",
            Self::GoalInputInvalid => "GOAL_INPUT_INVALID",
            Self::AuthoritySnapshotUnavailable => "AUTHORITY_SNAPSHOT_UNAVAILABLE",
            Self::AuthoritySnapshotMismatch => "AUTHORITY_SNAPSHOT_MISMATCH",
            Self::NoApplicableK1Action => "NO_APPLICABLE_K1_ACTION",
            Self::ActionProjectionIncomplete => "ACTION_PROJECTION_INCOMPLETE",
            Self::ActionCapacityExhausted => "ACTION_CAPACITY_EXHAUSTED",
            Self::PrecommitSealFailed => "PRECOMMIT_SEAL_FAILED",
            Self::PrecommitSyncFailed => "PRECOMMIT_SYNC_FAILED",
            Self::SelectedActionNotK1 => "SELECTED_ACTION_NOT_K1",
            Self::SelectedActionBindingFailed => "SELECTED_ACTION_BINDING_FAILED",
            Self::SelectedActionSyncFailed => "SELECTED_ACTION_SYNC_FAILED",
            Self::TerminalConsequenceUnavailable => "TERMINAL_CONSEQUENCE_UNAVAILABLE",
            Self::IndependentVerifierUnavailable => "INDEPENDENT_VERIFIER_UNAVAILABLE",
            Self::GoalPredicateVerificationFailed => "GOAL_PREDICATE_VERIFICATION_FAILED",
            Self::SatisfactionSyncFailed => "SATISFACTION_SYNC_FAILED",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TypedGoalComparatorV1 {
    TypedValueRootEquals,
    RecordProjectionRootEquals,
    CollectionMultisetRootEquals,
    CollectionCountEquals,
    BooleanEquals,
    RenderedSequenceRootEquals,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypedGoalPredicateArtifactV1 {
    pub schema: String,
    pub artifact_root_sha256: String,
    pub comparator: TypedGoalComparatorV1,
    pub consequence_type: K1ConsequenceTypeV1,
    pub typed_target_root_sha256: String,
    pub independent_verifier_contract_root_sha256: String,
}

impl TypedGoalPredicateArtifactV1 {
    pub fn seal(
        comparator: TypedGoalComparatorV1,
        consequence_type: K1ConsequenceTypeV1,
        typed_target_root_sha256: String,
        independent_verifier_contract_root_sha256: String,
    ) -> Result<Self, &'static str> {
        let mut artifact = Self {
            schema: TYPED_GOAL_PREDICATE_ARTIFACT_SCHEMA_V1.to_owned(),
            artifact_root_sha256: String::new(),
            comparator,
            consequence_type,
            typed_target_root_sha256,
            independent_verifier_contract_root_sha256,
        };
        artifact.artifact_root_sha256 = artifact.expected_root()?;
        artifact.validate()?;
        Ok(artifact)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != TYPED_GOAL_PREDICATE_ARTIFACT_SCHEMA_V1
            || !comparator_matches_type(self.comparator, self.consequence_type)
            || !roots_valid([
                self.artifact_root_sha256.as_str(),
                self.typed_target_root_sha256.as_str(),
                self.independent_verifier_contract_root_sha256.as_str(),
            ])
            || self.expected_root()? != self.artifact_root_sha256
            || self.canonical_bytes()?.len() > MAX_TYPED_GOAL_PREDICATE_BYTES_V1
        {
            return Err("typed_goal_predicate_artifact_invalid");
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, &'static str> {
        canonical_json_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > MAX_TYPED_GOAL_PREDICATE_BYTES_V1 {
            return Err("typed_goal_predicate_artifact_too_large");
        }
        let artifact: Self =
            serde_json::from_slice(bytes).map_err(|_| "typed_goal_predicate_artifact_decode")?;
        artifact.validate()?;
        if artifact.canonical_bytes()? != bytes {
            return Err("typed_goal_predicate_artifact_noncanonical");
        }
        Ok(artifact)
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            TYPED_GOAL_PREDICATE_ARTIFACT_SCHEMA_V1,
            self.comparator,
            self.consequence_type,
            self.typed_target_root_sha256.as_str(),
            self.independent_verifier_contract_root_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1ActionContractProjectionV1 {
    pub schema: String,
    pub action_contract_root_sha256: String,
    pub semantic_law_id_sha256: String,
    pub role_topology_id_sha256: String,
    pub program_semantic_class_id_sha256: String,
    pub effect_contract_root_sha256: String,
    pub applicability_contract_root_sha256: String,
    pub verifier_contract_root_sha256: String,
    pub pinned_callee_set_root_sha256: String,
    pub consequence_type: K1ConsequenceTypeV1,
}

impl K1ActionContractProjectionV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        semantic_law_id_sha256: String,
        role_topology_id_sha256: String,
        program_semantic_class_id_sha256: String,
        effect_contract_root_sha256: String,
        applicability_contract_root_sha256: String,
        verifier_contract_root_sha256: String,
        pinned_callee_set_root_sha256: String,
        consequence_type: K1ConsequenceTypeV1,
    ) -> Result<Self, &'static str> {
        let mut projection = Self {
            schema: K1_ACTION_CONTRACT_PROJECTION_SCHEMA_V1.to_owned(),
            action_contract_root_sha256: String::new(),
            semantic_law_id_sha256,
            role_topology_id_sha256,
            program_semantic_class_id_sha256,
            effect_contract_root_sha256,
            applicability_contract_root_sha256,
            verifier_contract_root_sha256,
            pinned_callee_set_root_sha256,
            consequence_type,
        };
        projection.action_contract_root_sha256 = projection.expected_root()?;
        projection.validate()?;
        Ok(projection)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != K1_ACTION_CONTRACT_PROJECTION_SCHEMA_V1
            || !roots_valid([
                self.action_contract_root_sha256.as_str(),
                self.semantic_law_id_sha256.as_str(),
                self.role_topology_id_sha256.as_str(),
                self.program_semantic_class_id_sha256.as_str(),
                self.effect_contract_root_sha256.as_str(),
                self.applicability_contract_root_sha256.as_str(),
                self.verifier_contract_root_sha256.as_str(),
                self.pinned_callee_set_root_sha256.as_str(),
            ])
            || self.expected_root()? != self.action_contract_root_sha256
        {
            return Err("k1_action_contract_projection_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            K1_ACTION_CONTRACT_PROJECTION_SCHEMA_V1,
            self.semantic_law_id_sha256.as_str(),
            self.role_topology_id_sha256.as_str(),
            self.program_semantic_class_id_sha256.as_str(),
            self.effect_contract_root_sha256.as_str(),
            self.applicability_contract_root_sha256.as_str(),
            self.verifier_contract_root_sha256.as_str(),
            self.pinned_callee_set_root_sha256.as_str(),
            self.consequence_type,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OpaqueActionExecutionBindingV1 {
    pub schema: String,
    pub binding_root_sha256: String,
    pub action_contract_root_sha256: String,
    pub execution_payload_root_sha256: String,
    pub external_admission_package_binding_root_sha256: String,
    pub certification_entry_root_sha256: String,
    pub response_registry_root_sha256: String,
    pub response_registry_revision: u64,
    pub certification_ledger_root_sha256: String,
    pub certification_ledger_revision: u64,
}

impl OpaqueActionExecutionBindingV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        action_contract_root_sha256: String,
        execution_payload_root_sha256: String,
        external_admission_package_binding_root_sha256: String,
        certification_entry_root_sha256: String,
        response_registry_root_sha256: String,
        response_registry_revision: u64,
        certification_ledger_root_sha256: String,
        certification_ledger_revision: u64,
    ) -> Result<Self, &'static str> {
        let mut binding = Self {
            schema: OPAQUE_ACTION_EXECUTION_BINDING_SCHEMA_V1.to_owned(),
            binding_root_sha256: String::new(),
            action_contract_root_sha256,
            execution_payload_root_sha256,
            external_admission_package_binding_root_sha256,
            certification_entry_root_sha256,
            response_registry_root_sha256,
            response_registry_revision,
            certification_ledger_root_sha256,
            certification_ledger_revision,
        };
        binding.binding_root_sha256 = binding.expected_root()?;
        binding.validate()?;
        Ok(binding)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != OPAQUE_ACTION_EXECUTION_BINDING_SCHEMA_V1
            || self.response_registry_revision == 0
            || self.certification_ledger_revision == 0
            || !roots_valid([
                self.binding_root_sha256.as_str(),
                self.action_contract_root_sha256.as_str(),
                self.execution_payload_root_sha256.as_str(),
                self.external_admission_package_binding_root_sha256.as_str(),
                self.certification_entry_root_sha256.as_str(),
                self.response_registry_root_sha256.as_str(),
                self.certification_ledger_root_sha256.as_str(),
            ])
            || self.expected_root()? != self.binding_root_sha256
        {
            return Err("opaque_action_execution_binding_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            OPAQUE_ACTION_EXECUTION_BINDING_SCHEMA_V1,
            self.action_contract_root_sha256.as_str(),
            self.execution_payload_root_sha256.as_str(),
            self.external_admission_package_binding_root_sha256.as_str(),
            self.certification_entry_root_sha256.as_str(),
            self.response_registry_root_sha256.as_str(),
            self.response_registry_revision,
            self.certification_ledger_root_sha256.as_str(),
            self.certification_ledger_revision,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionAuthoritySnapshotV1 {
    pub schema: String,
    pub snapshot_root_sha256: String,
    pub response_registry_schema: String,
    pub response_registry_revision: u64,
    pub response_registry_root_sha256: String,
    pub external_admission_authority_root_sha256: String,
    pub certification_ledger_revision: u64,
    pub certification_ledger_root_sha256: String,
    pub k1_vocabulary_gate_root_sha256: String,
    pub runtime_contract_root_sha256: String,
}

impl DecisionAuthoritySnapshotV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        response_registry_schema: String,
        response_registry_revision: u64,
        response_registry_root_sha256: String,
        external_admission_authority_root_sha256: String,
        certification_ledger_revision: u64,
        certification_ledger_root_sha256: String,
        k1_vocabulary_gate_root_sha256: String,
        runtime_contract_root_sha256: String,
    ) -> Result<Self, &'static str> {
        let mut snapshot = Self {
            schema: DECISION_AUTHORITY_SNAPSHOT_SCHEMA_V1.to_owned(),
            snapshot_root_sha256: String::new(),
            response_registry_schema,
            response_registry_revision,
            response_registry_root_sha256,
            external_admission_authority_root_sha256,
            certification_ledger_revision,
            certification_ledger_root_sha256,
            k1_vocabulary_gate_root_sha256,
            runtime_contract_root_sha256,
        };
        snapshot.snapshot_root_sha256 = snapshot.expected_root()?;
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != DECISION_AUTHORITY_SNAPSHOT_SCHEMA_V1
            || self.response_registry_schema.is_empty()
            || self.response_registry_revision == 0
            || self.certification_ledger_revision == 0
            || !roots_valid([
                self.snapshot_root_sha256.as_str(),
                self.response_registry_root_sha256.as_str(),
                self.external_admission_authority_root_sha256.as_str(),
                self.certification_ledger_root_sha256.as_str(),
                self.k1_vocabulary_gate_root_sha256.as_str(),
                self.runtime_contract_root_sha256.as_str(),
            ])
            || self.expected_root()? != self.snapshot_root_sha256
        {
            return Err("decision_authority_snapshot_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            DECISION_AUTHORITY_SNAPSHOT_SCHEMA_V1,
            self.response_registry_schema.as_str(),
            self.response_registry_revision,
            self.response_registry_root_sha256.as_str(),
            self.external_admission_authority_root_sha256.as_str(),
            self.certification_ledger_revision,
            self.certification_ledger_root_sha256.as_str(),
            self.k1_vocabulary_gate_root_sha256.as_str(),
            self.runtime_contract_root_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactPreActionGoalInputV1 {
    pub predicate_artifact: TypedGoalPredicateArtifactV1,
    pub pre_action_goal_evidence_root_sha256: String,
    pub outcome_horizon_contract_root_sha256: String,
    pub observation_mask_root_sha256: String,
    pub feature_exclusion_root_sha256: String,
    pub binder_schema_root_sha256: String,
    pub pre_action_observation_root_sha256: String,
    pub independent_binder_root_sha256: String,
    pub frozen_at_sequence: u64,
    pub action_selection_not_before_sequence: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactPreActionGoalBindingV1 {
    pub predicate_artifact: TypedGoalPredicateArtifactV1,
    pub goal_contract: TypedGoalContractV1,
    pub binding_receipt: PreActionGoalBindingReceiptV1,
}

pub fn bind_exact_pre_action_goal_v1(
    input: ExactPreActionGoalInputV1,
) -> Result<ExactPreActionGoalBindingV1, &'static str> {
    input.predicate_artifact.validate()?;
    let goal_contract = TypedGoalContractV1::seal(
        input.pre_action_goal_evidence_root_sha256,
        input.predicate_artifact.artifact_root_sha256.clone(),
        input.outcome_horizon_contract_root_sha256,
        input.observation_mask_root_sha256,
        input.feature_exclusion_root_sha256,
        input
            .predicate_artifact
            .independent_verifier_contract_root_sha256
            .clone(),
        input.binder_schema_root_sha256,
        input.frozen_at_sequence,
    )?;
    let binding_receipt = PreActionGoalBindingReceiptV1::seal(
        &goal_contract,
        input.pre_action_observation_root_sha256,
        input.independent_binder_root_sha256,
        input.action_selection_not_before_sequence,
    )?;
    Ok(ExactPreActionGoalBindingV1 {
        predicate_artifact: input.predicate_artifact,
        goal_contract,
        binding_receipt,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionContractPrecommitInputV1 {
    pub request_event_identity_root_sha256: String,
    pub process_epoch_root_sha256: String,
    pub pre_action_observation_root_sha256: String,
    pub pre_action_topology_root_sha256: String,
    pub goal_contract: TypedGoalContractV1,
    pub goal_binding_receipt: PreActionGoalBindingReceiptV1,
    pub constraint_contract_root_sha256: String,
    pub authority_snapshot: DecisionAuthoritySnapshotV1,
    pub applicability_evaluator_schema: String,
    pub available_action_contracts_root_sha256: String,
    pub available_action_count: u32,
    pub opaque_execution_binding_set_root_sha256: String,
    pub journal_sequence: u64,
    pub action_selection_not_before_sequence: u64,
    pub precommit_monotonic_nanos: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionContractPrecommitV1 {
    pub schema: String,
    pub precommit_root_sha256: String,
    pub request_event_identity_root_sha256: String,
    pub process_epoch_root_sha256: String,
    pub pre_action_observation_root_sha256: String,
    pub pre_action_topology_root_sha256: String,
    pub typed_goal_contract_root_sha256: String,
    pub goal_binding_receipt_root_sha256: String,
    pub constraint_contract_root_sha256: String,
    pub outcome_horizon_contract_root_sha256: String,
    pub observation_mask_root_sha256: String,
    pub feature_exclusion_root_sha256: String,
    pub decision_authority_snapshot_root_sha256: String,
    pub response_registry_schema: String,
    pub response_registry_revision: u64,
    pub response_registry_root_sha256: String,
    pub external_admission_authority_root_sha256: String,
    pub certification_ledger_revision: u64,
    pub certification_ledger_root_sha256: String,
    pub k1_vocabulary_gate_root_sha256: String,
    pub applicability_evaluator_schema: String,
    pub runtime_contract_root_sha256: String,
    pub available_action_contracts_root_sha256: String,
    #[serde(default)]
    pub available_action_count: u32,
    pub opaque_execution_binding_set_root_sha256: String,
    pub journal_sequence: u64,
    pub action_selection_not_before_sequence: u64,
    pub precommit_monotonic_nanos: u64,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Serialize)]
struct DecisionContractPrecommitDigestV1<'a> {
    schema: &'static str,
    request_event_identity_root_sha256: &'a str,
    process_epoch_root_sha256: &'a str,
    pre_action_observation_root_sha256: &'a str,
    pre_action_topology_root_sha256: &'a str,
    typed_goal_contract_root_sha256: &'a str,
    goal_binding_receipt_root_sha256: &'a str,
    constraint_contract_root_sha256: &'a str,
    outcome_horizon_contract_root_sha256: &'a str,
    observation_mask_root_sha256: &'a str,
    feature_exclusion_root_sha256: &'a str,
    decision_authority_snapshot_root_sha256: &'a str,
    response_registry_schema: &'a str,
    response_registry_revision: u64,
    response_registry_root_sha256: &'a str,
    external_admission_authority_root_sha256: &'a str,
    certification_ledger_revision: u64,
    certification_ledger_root_sha256: &'a str,
    k1_vocabulary_gate_root_sha256: &'a str,
    applicability_evaluator_schema: &'a str,
    runtime_contract_root_sha256: &'a str,
    available_action_contracts_root_sha256: &'a str,
    opaque_execution_binding_set_root_sha256: &'a str,
    journal_sequence: u64,
    action_selection_not_before_sequence: u64,
    precommit_monotonic_nanos: u64,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

#[derive(Serialize)]
struct DecisionContractPrecommitDigestV2<'a> {
    schema: &'static str,
    request_event_identity_root_sha256: &'a str,
    process_epoch_root_sha256: &'a str,
    pre_action_observation_root_sha256: &'a str,
    pre_action_topology_root_sha256: &'a str,
    typed_goal_contract_root_sha256: &'a str,
    goal_binding_receipt_root_sha256: &'a str,
    constraint_contract_root_sha256: &'a str,
    outcome_horizon_contract_root_sha256: &'a str,
    observation_mask_root_sha256: &'a str,
    feature_exclusion_root_sha256: &'a str,
    decision_authority_snapshot_root_sha256: &'a str,
    response_registry_schema: &'a str,
    response_registry_revision: u64,
    response_registry_root_sha256: &'a str,
    external_admission_authority_root_sha256: &'a str,
    certification_ledger_revision: u64,
    certification_ledger_root_sha256: &'a str,
    k1_vocabulary_gate_root_sha256: &'a str,
    applicability_evaluator_schema: &'a str,
    runtime_contract_root_sha256: &'a str,
    available_action_contracts_root_sha256: &'a str,
    available_action_count: u32,
    opaque_execution_binding_set_root_sha256: &'a str,
    journal_sequence: u64,
    action_selection_not_before_sequence: u64,
    precommit_monotonic_nanos: u64,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

impl DecisionContractPrecommitV1 {
    pub fn seal(input: DecisionContractPrecommitInputV1) -> Result<Self, &'static str> {
        input.goal_contract.validate()?;
        input.goal_binding_receipt.validate()?;
        input.authority_snapshot.validate()?;
        if input.goal_binding_receipt.goal_contract_root_sha256
            != input.goal_contract.goal_contract_root_sha256
            || input
                .goal_binding_receipt
                .pre_action_observation_root_sha256
                != input.pre_action_observation_root_sha256
            || input
                .goal_binding_receipt
                .action_selection_not_before_sequence
                != input.action_selection_not_before_sequence
        {
            return Err("decision_contract_precommit_join_invalid");
        }
        let authority = input.authority_snapshot;
        let mut precommit = Self {
            schema: DECISION_CONTRACT_PRECOMMIT_SCHEMA_V2.to_owned(),
            precommit_root_sha256: String::new(),
            request_event_identity_root_sha256: input.request_event_identity_root_sha256,
            process_epoch_root_sha256: input.process_epoch_root_sha256,
            pre_action_observation_root_sha256: input.pre_action_observation_root_sha256,
            pre_action_topology_root_sha256: input.pre_action_topology_root_sha256,
            typed_goal_contract_root_sha256: input.goal_contract.goal_contract_root_sha256,
            goal_binding_receipt_root_sha256: input.goal_binding_receipt.receipt_root_sha256,
            constraint_contract_root_sha256: input.constraint_contract_root_sha256,
            outcome_horizon_contract_root_sha256: input
                .goal_contract
                .outcome_horizon_contract_root_sha256,
            observation_mask_root_sha256: input.goal_contract.observation_mask_root_sha256,
            feature_exclusion_root_sha256: input.goal_contract.feature_exclusion_root_sha256,
            decision_authority_snapshot_root_sha256: authority.snapshot_root_sha256,
            response_registry_schema: authority.response_registry_schema,
            response_registry_revision: authority.response_registry_revision,
            response_registry_root_sha256: authority.response_registry_root_sha256,
            external_admission_authority_root_sha256: authority
                .external_admission_authority_root_sha256,
            certification_ledger_revision: authority.certification_ledger_revision,
            certification_ledger_root_sha256: authority.certification_ledger_root_sha256,
            k1_vocabulary_gate_root_sha256: authority.k1_vocabulary_gate_root_sha256,
            applicability_evaluator_schema: input.applicability_evaluator_schema,
            runtime_contract_root_sha256: authority.runtime_contract_root_sha256,
            available_action_contracts_root_sha256: input.available_action_contracts_root_sha256,
            available_action_count: input.available_action_count,
            opaque_execution_binding_set_root_sha256: input
                .opaque_execution_binding_set_root_sha256,
            journal_sequence: input.journal_sequence,
            action_selection_not_before_sequence: input.action_selection_not_before_sequence,
            precommit_monotonic_nanos: input.precommit_monotonic_nanos,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        precommit.precommit_root_sha256 = precommit.expected_root()?;
        precommit.validate()?;
        Ok(precommit)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if !matches!(
            self.schema.as_str(),
            DECISION_CONTRACT_PRECOMMIT_SCHEMA_V1 | DECISION_CONTRACT_PRECOMMIT_SCHEMA_V2
        ) || self.response_registry_schema.is_empty()
            || self.applicability_evaluator_schema.is_empty()
            || self.response_registry_revision == 0
            || self.certification_ledger_revision == 0
            || self.journal_sequence == 0
            || self.action_selection_not_before_sequence <= self.journal_sequence
            || self.precommit_monotonic_nanos == 0
            || (self.schema == DECISION_CONTRACT_PRECOMMIT_SCHEMA_V2
                && self.available_action_count == 0)
            || (self.schema == DECISION_CONTRACT_PRECOMMIT_SCHEMA_V1
                && self.available_action_count != 0)
            || self.authority_ready
            || self.phase_mutation_allowed
            || !roots_valid([
                self.precommit_root_sha256.as_str(),
                self.request_event_identity_root_sha256.as_str(),
                self.process_epoch_root_sha256.as_str(),
                self.pre_action_observation_root_sha256.as_str(),
                self.pre_action_topology_root_sha256.as_str(),
                self.typed_goal_contract_root_sha256.as_str(),
                self.goal_binding_receipt_root_sha256.as_str(),
                self.constraint_contract_root_sha256.as_str(),
                self.outcome_horizon_contract_root_sha256.as_str(),
                self.observation_mask_root_sha256.as_str(),
                self.feature_exclusion_root_sha256.as_str(),
                self.decision_authority_snapshot_root_sha256.as_str(),
                self.response_registry_root_sha256.as_str(),
                self.external_admission_authority_root_sha256.as_str(),
                self.certification_ledger_root_sha256.as_str(),
                self.k1_vocabulary_gate_root_sha256.as_str(),
                self.runtime_contract_root_sha256.as_str(),
                self.available_action_contracts_root_sha256.as_str(),
                self.opaque_execution_binding_set_root_sha256.as_str(),
            ])
            || self.expected_root()? != self.precommit_root_sha256
            || canonical_json_bytes(self)?.len() > MAX_DECISION_PRECOMMIT_BYTES_V1
        {
            return Err("decision_contract_precommit_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        if self.schema == DECISION_CONTRACT_PRECOMMIT_SCHEMA_V1 {
            return canonical_json_sha256(&DecisionContractPrecommitDigestV1 {
                schema: DECISION_CONTRACT_PRECOMMIT_SCHEMA_V1,
                request_event_identity_root_sha256: &self.request_event_identity_root_sha256,
                process_epoch_root_sha256: &self.process_epoch_root_sha256,
                pre_action_observation_root_sha256: &self.pre_action_observation_root_sha256,
                pre_action_topology_root_sha256: &self.pre_action_topology_root_sha256,
                typed_goal_contract_root_sha256: &self.typed_goal_contract_root_sha256,
                goal_binding_receipt_root_sha256: &self.goal_binding_receipt_root_sha256,
                constraint_contract_root_sha256: &self.constraint_contract_root_sha256,
                outcome_horizon_contract_root_sha256: &self.outcome_horizon_contract_root_sha256,
                observation_mask_root_sha256: &self.observation_mask_root_sha256,
                feature_exclusion_root_sha256: &self.feature_exclusion_root_sha256,
                decision_authority_snapshot_root_sha256: &self
                    .decision_authority_snapshot_root_sha256,
                response_registry_schema: &self.response_registry_schema,
                response_registry_revision: self.response_registry_revision,
                response_registry_root_sha256: &self.response_registry_root_sha256,
                external_admission_authority_root_sha256: &self
                    .external_admission_authority_root_sha256,
                certification_ledger_revision: self.certification_ledger_revision,
                certification_ledger_root_sha256: &self.certification_ledger_root_sha256,
                k1_vocabulary_gate_root_sha256: &self.k1_vocabulary_gate_root_sha256,
                applicability_evaluator_schema: &self.applicability_evaluator_schema,
                runtime_contract_root_sha256: &self.runtime_contract_root_sha256,
                available_action_contracts_root_sha256: &self
                    .available_action_contracts_root_sha256,
                opaque_execution_binding_set_root_sha256: &self
                    .opaque_execution_binding_set_root_sha256,
                journal_sequence: self.journal_sequence,
                action_selection_not_before_sequence: self.action_selection_not_before_sequence,
                precommit_monotonic_nanos: self.precommit_monotonic_nanos,
                authority_ready: false,
                phase_mutation_allowed: false,
            });
        }
        canonical_json_sha256(&DecisionContractPrecommitDigestV2 {
            schema: DECISION_CONTRACT_PRECOMMIT_SCHEMA_V2,
            request_event_identity_root_sha256: &self.request_event_identity_root_sha256,
            process_epoch_root_sha256: &self.process_epoch_root_sha256,
            pre_action_observation_root_sha256: &self.pre_action_observation_root_sha256,
            pre_action_topology_root_sha256: &self.pre_action_topology_root_sha256,
            typed_goal_contract_root_sha256: &self.typed_goal_contract_root_sha256,
            goal_binding_receipt_root_sha256: &self.goal_binding_receipt_root_sha256,
            constraint_contract_root_sha256: &self.constraint_contract_root_sha256,
            outcome_horizon_contract_root_sha256: &self.outcome_horizon_contract_root_sha256,
            observation_mask_root_sha256: &self.observation_mask_root_sha256,
            feature_exclusion_root_sha256: &self.feature_exclusion_root_sha256,
            decision_authority_snapshot_root_sha256: &self.decision_authority_snapshot_root_sha256,
            response_registry_schema: &self.response_registry_schema,
            response_registry_revision: self.response_registry_revision,
            response_registry_root_sha256: &self.response_registry_root_sha256,
            external_admission_authority_root_sha256: &self
                .external_admission_authority_root_sha256,
            certification_ledger_revision: self.certification_ledger_revision,
            certification_ledger_root_sha256: &self.certification_ledger_root_sha256,
            k1_vocabulary_gate_root_sha256: &self.k1_vocabulary_gate_root_sha256,
            applicability_evaluator_schema: &self.applicability_evaluator_schema,
            runtime_contract_root_sha256: &self.runtime_contract_root_sha256,
            available_action_contracts_root_sha256: &self.available_action_contracts_root_sha256,
            available_action_count: self.available_action_count,
            opaque_execution_binding_set_root_sha256: &self
                .opaque_execution_binding_set_root_sha256,
            journal_sequence: self.journal_sequence,
            action_selection_not_before_sequence: self.action_selection_not_before_sequence,
            precommit_monotonic_nanos: self.precommit_monotonic_nanos,
            authority_ready: false,
            phase_mutation_allowed: false,
        })
    }

    #[cfg(test)]
    pub(crate) fn reseal_as_legacy_v1_for_test(&mut self) -> Result<(), &'static str> {
        self.schema = DECISION_CONTRACT_PRECOMMIT_SCHEMA_V1.to_owned();
        self.available_action_count = 0;
        self.precommit_root_sha256 = self.expected_root()?;
        self.validate()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionContractDurabilityReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub precommit_root_sha256: String,
    pub segment_id: u64,
    pub offset: u64,
    pub payload_bytes: u32,
    pub payload_sha256: String,
}

impl DecisionContractDurabilityReceiptV1 {
    pub fn seal(
        precommit_root_sha256: String,
        segment_id: u64,
        offset: u64,
        payload_bytes: u32,
        payload_sha256: String,
    ) -> Result<Self, &'static str> {
        let mut receipt = Self {
            schema: DECISION_CONTRACT_DURABILITY_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            precommit_root_sha256,
            segment_id,
            offset,
            payload_bytes,
            payload_sha256,
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != DECISION_CONTRACT_DURABILITY_RECEIPT_SCHEMA_V1
            || self.offset < 4
            || self.payload_bytes == 0
            || usize::try_from(self.payload_bytes).unwrap_or(usize::MAX)
                > MAX_DECISION_PRECOMMIT_BYTES_V1
            || !roots_valid([
                self.receipt_root_sha256.as_str(),
                self.precommit_root_sha256.as_str(),
                self.payload_sha256.as_str(),
            ])
            || self.expected_root()? != self.receipt_root_sha256
        {
            return Err("decision_contract_durability_receipt_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            DECISION_CONTRACT_DURABILITY_RECEIPT_SCHEMA_V1,
            self.precommit_root_sha256.as_str(),
            self.segment_id,
            self.offset,
            self.payload_bytes,
            self.payload_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedActionBindingReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub precommit_root_sha256: String,
    pub selected_action_contract_root_sha256: String,
    pub opaque_execution_binding_root_sha256: String,
    pub runtime_verification_receipt_root_sha256: String,
    pub selected_action_sequence: u64,
    pub selected_at_monotonic_nanos: u64,
    pub process_epoch_root_sha256: String,
}

impl SelectedActionBindingReceiptV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        precommit: &DecisionContractPrecommitV1,
        selected_action_contract_root_sha256: String,
        opaque_execution_binding_root_sha256: String,
        runtime_verification_receipt_root_sha256: String,
        selected_action_sequence: u64,
        selected_at_monotonic_nanos: u64,
        process_epoch_root_sha256: String,
    ) -> Result<Self, &'static str> {
        precommit.validate()?;
        if process_epoch_root_sha256 != precommit.process_epoch_root_sha256
            || selected_action_sequence < precommit.action_selection_not_before_sequence
            || selected_at_monotonic_nanos < precommit.precommit_monotonic_nanos
        {
            return Err("selected_action_binding_temporal_invalid");
        }
        let mut receipt = Self {
            schema: SELECTED_ACTION_BINDING_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            precommit_root_sha256: precommit.precommit_root_sha256.clone(),
            selected_action_contract_root_sha256,
            opaque_execution_binding_root_sha256,
            runtime_verification_receipt_root_sha256,
            selected_action_sequence,
            selected_at_monotonic_nanos,
            process_epoch_root_sha256,
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != SELECTED_ACTION_BINDING_RECEIPT_SCHEMA_V1
            || self.selected_action_sequence == 0
            || self.selected_at_monotonic_nanos == 0
            || !roots_valid([
                self.receipt_root_sha256.as_str(),
                self.precommit_root_sha256.as_str(),
                self.selected_action_contract_root_sha256.as_str(),
                self.opaque_execution_binding_root_sha256.as_str(),
                self.runtime_verification_receipt_root_sha256.as_str(),
                self.process_epoch_root_sha256.as_str(),
            ])
            || self.expected_root()? != self.receipt_root_sha256
        {
            return Err("selected_action_binding_receipt_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            SELECTED_ACTION_BINDING_RECEIPT_SCHEMA_V1,
            self.precommit_root_sha256.as_str(),
            self.selected_action_contract_root_sha256.as_str(),
            self.opaque_execution_binding_root_sha256.as_str(),
            self.runtime_verification_receipt_root_sha256.as_str(),
            self.selected_action_sequence,
            self.selected_at_monotonic_nanos,
            self.process_epoch_root_sha256.as_str(),
        ))
    }
}

pub fn opaque_action_execution_binding_set_root_v1(
    mut binding_roots_sha256: Vec<String>,
) -> Result<String, &'static str> {
    binding_roots_sha256.sort_unstable();
    binding_roots_sha256.dedup();
    if binding_roots_sha256.is_empty()
        || binding_roots_sha256.len() > MAX_DECISION_ACTION_BINDINGS_V1
        || binding_roots_sha256
            .iter()
            .any(|root| !valid_nonzero_sha256(root))
    {
        return Err("opaque_action_execution_binding_set_invalid");
    }
    canonical_json_sha256(&(
        OPAQUE_ACTION_EXECUTION_BINDING_SET_SCHEMA_V1,
        &binding_roots_sha256,
    ))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DurableSelectedActionBindingV1 {
    pub schema: String,
    pub record_root_sha256: String,
    pub receipt: SelectedActionBindingReceiptV1,
    pub action_projection: K1ActionContractProjectionV1,
    pub execution_binding: OpaqueActionExecutionBindingV1,
    pub available_actions: AvailableActionContractsV1,
    pub opaque_execution_binding_roots_sha256: Vec<String>,
    pub observed_consequence_root_sha256: String,
}

impl DurableSelectedActionBindingV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        precommit: &DecisionContractPrecommitV1,
        receipt: SelectedActionBindingReceiptV1,
        action_projection: K1ActionContractProjectionV1,
        execution_binding: OpaqueActionExecutionBindingV1,
        available_actions: AvailableActionContractsV1,
        mut opaque_execution_binding_roots_sha256: Vec<String>,
        observed_consequence_root_sha256: String,
    ) -> Result<Self, &'static str> {
        opaque_execution_binding_roots_sha256.sort_unstable();
        opaque_execution_binding_roots_sha256.dedup();
        let mut record = Self {
            schema: DURABLE_SELECTED_ACTION_BINDING_SCHEMA_V1.to_owned(),
            record_root_sha256: String::new(),
            receipt,
            action_projection,
            execution_binding,
            available_actions,
            opaque_execution_binding_roots_sha256,
            observed_consequence_root_sha256,
        };
        record.validate_join(precommit)?;
        record.record_root_sha256 = record.expected_root()?;
        record.validate_join(precommit)?;
        Ok(record)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        self.receipt.validate()?;
        self.action_projection.validate()?;
        self.execution_binding.validate()?;
        self.available_actions.validate()?;
        if self.schema != DURABLE_SELECTED_ACTION_BINDING_SCHEMA_V1
            || !valid_nonzero_sha256(&self.record_root_sha256)
            || !valid_nonzero_sha256(&self.observed_consequence_root_sha256)
            || self.action_projection.action_contract_root_sha256
                != self.execution_binding.action_contract_root_sha256
            || self.receipt.selected_action_contract_root_sha256
                != self.action_projection.action_contract_root_sha256
            || self.receipt.opaque_execution_binding_root_sha256
                != self.execution_binding.binding_root_sha256
            || !self
                .available_actions
                .action_contract_roots_sha256
                .contains(&self.action_projection.action_contract_root_sha256)
            || !self
                .opaque_execution_binding_roots_sha256
                .contains(&self.execution_binding.binding_root_sha256)
            || opaque_action_execution_binding_set_root_v1(
                self.opaque_execution_binding_roots_sha256.clone(),
            )
            .is_err()
            || self.expected_root()? != self.record_root_sha256
        {
            return Err("durable_selected_action_binding_invalid");
        }
        Ok(())
    }

    pub fn validate_join(
        &self,
        precommit: &DecisionContractPrecommitV1,
    ) -> Result<(), &'static str> {
        precommit.validate()?;
        if !self.record_root_sha256.is_empty() {
            self.validate()?;
        } else {
            self.receipt.validate()?;
            self.action_projection.validate()?;
            self.execution_binding.validate()?;
            self.available_actions.validate()?;
        }
        let binding_set_root = opaque_action_execution_binding_set_root_v1(
            self.opaque_execution_binding_roots_sha256.clone(),
        )?;
        if self.receipt.precommit_root_sha256 != precommit.precommit_root_sha256
            || self.receipt.process_epoch_root_sha256 != precommit.process_epoch_root_sha256
            || self.available_actions.contracts_root_sha256
                != precommit.available_action_contracts_root_sha256
            || binding_set_root != precommit.opaque_execution_binding_set_root_sha256
            || self.execution_binding.response_registry_root_sha256
                != precommit.response_registry_root_sha256
            || self.execution_binding.response_registry_revision
                != precommit.response_registry_revision
            || self.execution_binding.certification_ledger_root_sha256
                != precommit.certification_ledger_root_sha256
            || self.execution_binding.certification_ledger_revision
                != precommit.certification_ledger_revision
        {
            return Err("durable_selected_action_binding_join_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            DURABLE_SELECTED_ACTION_BINDING_SCHEMA_V1,
            self.receipt.receipt_root_sha256.as_str(),
            self.action_projection.action_contract_root_sha256.as_str(),
            self.execution_binding.binding_root_sha256.as_str(),
            self.available_actions.contracts_root_sha256.as_str(),
            &self.opaque_execution_binding_roots_sha256,
            self.observed_consequence_root_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DurableGoalSatisfactionV1 {
    pub schema: String,
    pub record_root_sha256: String,
    pub precommit_root_sha256: String,
    pub selected_action_receipt_root_sha256: String,
    pub goal_contract: TypedGoalContractV1,
    pub predicate_artifact: TypedGoalPredicateArtifactV1,
    pub receipt: GoalSatisfactionReceiptV1,
}

impl DurableGoalSatisfactionV1 {
    pub fn seal(
        precommit: &DecisionContractPrecommitV1,
        selected: &DurableSelectedActionBindingV1,
        goal_contract: TypedGoalContractV1,
        predicate_artifact: TypedGoalPredicateArtifactV1,
        receipt: GoalSatisfactionReceiptV1,
    ) -> Result<Self, &'static str> {
        let mut record = Self {
            schema: DURABLE_GOAL_SATISFACTION_SCHEMA_V1.to_owned(),
            record_root_sha256: String::new(),
            precommit_root_sha256: precommit.precommit_root_sha256.clone(),
            selected_action_receipt_root_sha256: selected.receipt.receipt_root_sha256.clone(),
            goal_contract,
            predicate_artifact,
            receipt,
        };
        record.validate_join(precommit, selected)?;
        record.record_root_sha256 = record.expected_root()?;
        record.validate_join(precommit, selected)?;
        Ok(record)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        self.goal_contract.validate()?;
        self.predicate_artifact.validate()?;
        self.receipt.validate()?;
        if self.schema != DURABLE_GOAL_SATISFACTION_SCHEMA_V1
            || !roots_valid([
                self.record_root_sha256.as_str(),
                self.precommit_root_sha256.as_str(),
                self.selected_action_receipt_root_sha256.as_str(),
            ])
            || self.goal_contract.typed_success_predicate_root_sha256
                != self.predicate_artifact.artifact_root_sha256
            || self.receipt.goal_contract_root_sha256
                != self.goal_contract.goal_contract_root_sha256
            || self.receipt.outcome_horizon_contract_root_sha256
                != self.goal_contract.outcome_horizon_contract_root_sha256
            || self.expected_root()? != self.record_root_sha256
        {
            return Err("durable_goal_satisfaction_invalid");
        }
        Ok(())
    }

    pub fn validate_join(
        &self,
        precommit: &DecisionContractPrecommitV1,
        selected: &DurableSelectedActionBindingV1,
    ) -> Result<(), &'static str> {
        precommit.validate()?;
        selected.validate_join(precommit)?;
        if !self.record_root_sha256.is_empty() {
            self.validate()?;
        } else {
            self.goal_contract.validate()?;
            self.predicate_artifact.validate()?;
            self.receipt.validate()?;
        }
        let expected_satisfied = verify_exact_goal_predicate_v1(
            &self.predicate_artifact,
            selected.action_projection.consequence_type,
            &selected.action_projection.verifier_contract_root_sha256,
            &selected.observed_consequence_root_sha256,
        )?;
        if self.precommit_root_sha256 != precommit.precommit_root_sha256
            || self.selected_action_receipt_root_sha256 != selected.receipt.receipt_root_sha256
            || self.goal_contract.goal_contract_root_sha256
                != precommit.typed_goal_contract_root_sha256
            || self.receipt.observed_consequence_root_sha256
                != selected.observed_consequence_root_sha256
            || self.receipt.independent_verifier_root_sha256
                != selected.receipt.runtime_verification_receipt_root_sha256
            || self.receipt.satisfied != expected_satisfied
        {
            return Err("durable_goal_satisfaction_join_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            DURABLE_GOAL_SATISFACTION_SCHEMA_V1,
            self.precommit_root_sha256.as_str(),
            self.selected_action_receipt_root_sha256.as_str(),
            self.goal_contract.goal_contract_root_sha256.as_str(),
            self.predicate_artifact.artifact_root_sha256.as_str(),
            self.receipt.receipt_root_sha256.as_str(),
        ))
    }
}

pub fn verify_exact_goal_predicate_v1(
    predicate: &TypedGoalPredicateArtifactV1,
    observed_consequence_type: K1ConsequenceTypeV1,
    verifier_contract_root_sha256: &str,
    observed_consequence_root_sha256: &str,
) -> Result<bool, &'static str> {
    predicate.validate()?;
    if observed_consequence_type != predicate.consequence_type
        || verifier_contract_root_sha256 != predicate.independent_verifier_contract_root_sha256
        || !valid_nonzero_sha256(observed_consequence_root_sha256)
    {
        return Err("exact_goal_predicate_verification_invalid");
    }
    Ok(predicate.typed_target_root_sha256 == observed_consequence_root_sha256)
}

fn comparator_matches_type(
    comparator: TypedGoalComparatorV1,
    consequence_type: K1ConsequenceTypeV1,
) -> bool {
    matches!(
        (comparator, consequence_type),
        (
            TypedGoalComparatorV1::TypedValueRootEquals
                | TypedGoalComparatorV1::CollectionCountEquals,
            K1ConsequenceTypeV1::Scalar
        ) | (
            TypedGoalComparatorV1::RecordProjectionRootEquals,
            K1ConsequenceTypeV1::Record
        ) | (
            TypedGoalComparatorV1::CollectionMultisetRootEquals,
            K1ConsequenceTypeV1::Collection
        ) | (
            TypedGoalComparatorV1::BooleanEquals,
            K1ConsequenceTypeV1::Boolean
        ) | (
            TypedGoalComparatorV1::RenderedSequenceRootEquals,
            K1ConsequenceTypeV1::RenderedSequence
        )
    )
}

fn roots_valid<'a>(roots: impl IntoIterator<Item = &'a str>) -> bool {
    roots.into_iter().all(valid_nonzero_sha256)
}
