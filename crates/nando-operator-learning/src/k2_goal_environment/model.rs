use std::collections::BTreeSet;
use std::fmt;

use nando_operator_kernel::{canonical_json_bytes, canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use crate::{LawLabSandboxExecutionV1, LawLabSandboxPurposeV1, LawLabSandboxRequestV1};

pub const K2_GOAL_ENVELOPE_SCHEMA_V1: &str = "nando.k2-goal-envelope.v1";
pub const K2_GOAL_PREDICATE_SCHEMA_V1: &str = "nando.k2-goal-predicate.v1";
pub const K2_ACTION_REF_SCHEMA_V1: &str = "nando.k2-k1-action-ref.v1";
pub const K2_VOCABULARY_SNAPSHOT_SCHEMA_V1: &str = "nando.k2-k1-vocabulary-snapshot.v1";
pub const K2_ALTERNATIVE_SET_SCHEMA_V1: &str = "nando.k2-alternative-set.v1";
pub const K2_BUDGET_SCHEMA_V1: &str = "nando.k2-goal-environment-budget.v1";
pub const K2_ORACLE_MANIFEST_SCHEMA_V1: &str = "nando.k2-exact-oracle-manifest.v1";
pub const K2_DECISION_FREEZE_SCHEMA_V1: &str = "nando.k2-decision-freeze.v1";
pub const K2_PREDICTION_SET_SCHEMA_V1: &str = "nando.k2-alternative-prediction-set.v1";
pub const K2_PREPARED_SELECTOR_SCHEMA_V1: &str = "nando.k2-prepared-capability-selector.v1";
pub const K2_SELECTION_RECEIPT_SCHEMA_V1: &str = "nando.k2-prepared-selection-receipt.v1";
pub const K2_LAW_LAB_BINDING_SCHEMA_V1: &str = "nando.k2-law-lab-binding.v1";
pub const K2_EXACT_ORACLE_REQUEST_SCHEMA_V1: &str = "nando.k2-exact-oracle-request.v1";
pub const K2_EXACT_ORACLE_OUTCOME_SCHEMA_V1: &str = "nando.k2-exact-oracle-outcome.v1";
pub const K2_EXACT_GOAL_RECEIPT_SCHEMA_V1: &str = "nando.k2-exact-goal-receipt.v1";
pub const K2_DECISION_OUTCOME_SCHEMA_V1: &str = "nando.k2-decision-outcome.v1";
pub const K2_EPISODE_SEAL_SCHEMA_V1: &str = "nando.k2-decision-episode-seal.v1";
pub const K2_AUTHORITY_BOUNDARY_SCHEMA_V1: &str = "nando.k2-authority-boundary.v1";

pub const K2_MAX_ALTERNATIVES_V1: usize = 16;
pub const K2_MAX_EVENTS_PER_EPISODE_V1: u64 = 16;
pub const K2_MAX_EVENT_BYTES_V1: u64 = 64 * 1024;
pub const K2_MAX_EPISODE_BYTES_V1: u64 = 1024 * 1024;
pub const K2_MAX_RETAINED_CAPABILITY_EPISODES_V1: u64 = 64;

pub type K2GoalEnvironmentResultV1<T> = Result<T, K2GoalEnvironmentErrorV1>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum K2GoalEnvironmentErrorV1 {
    Invalid(&'static str),
    Io(String),
    Sandbox(String),
    Serialization,
}

impl fmt::Display for K2GoalEnvironmentErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(reason) => formatter.write_str(reason),
            Self::Io(reason) => write!(formatter, "k2_goal_environment_io:{reason}"),
            Self::Sandbox(reason) => write!(formatter, "k2_goal_environment_sandbox:{reason}"),
            Self::Serialization => formatter.write_str("k2_goal_environment_serialization"),
        }
    }
}

impl std::error::Error for K2GoalEnvironmentErrorV1 {}

fn canonical_root<T: Serialize>(value: &T) -> K2GoalEnvironmentResultV1<String> {
    canonical_json_sha256(value).map_err(|_| K2GoalEnvironmentErrorV1::Serialization)
}

fn require_root(root: &str, reason: &'static str) -> K2GoalEnvironmentResultV1<()> {
    if valid_nonzero_sha256(root) {
        Ok(())
    } else {
        Err(K2GoalEnvironmentErrorV1::Invalid(reason))
    }
}

fn roots_are_unique<'a>(roots: impl IntoIterator<Item = &'a str>) -> bool {
    let roots = roots.into_iter().collect::<Vec<_>>();
    roots.iter().copied().collect::<BTreeSet<_>>().len() == roots.len()
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2EvidenceProvenanceV1 {
    GeneratedCapabilitySelfTest,
    CertificateBoundK1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2GoalHorizonV1 {
    SingleSandboxTerminal,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2AuthorityBoundaryV1 {
    pub schema: String,
    pub law_certificate_issued: bool,
    pub package_activated: bool,
    pub execution_authority_granted: bool,
    pub k1_registry_mutated: bool,
    pub k2_claim_granted: bool,
    pub phase_memory_mutated: bool,
    pub product_economics_credited: bool,
    pub natural_holdout_satisfied: bool,
}

impl K2AuthorityBoundaryV1 {
    #[must_use]
    pub fn authority_free_v1() -> Self {
        Self {
            schema: K2_AUTHORITY_BOUNDARY_SCHEMA_V1.to_owned(),
            law_certificate_issued: false,
            package_activated: false,
            execution_authority_granted: false,
            k1_registry_mutated: false,
            k2_claim_granted: false,
            phase_memory_mutated: false,
            product_economics_credited: false,
            natural_holdout_satisfied: false,
        }
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        if self == &Self::authority_free_v1() {
            Ok(())
        } else {
            Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_authority_boundary_violated",
            ))
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2GoalEnvelopeV1 {
    pub schema: String,
    pub goal_envelope_root_sha256: String,
    pub provenance: K2EvidenceProvenanceV1,
    pub environment_root_sha256: String,
    pub goal_predicate_root_sha256: String,
    pub expected_terminal_tree_root_sha256: String,
    pub expected_goal_store_snapshot_root_sha256: String,
    pub constraints_root_sha256: String,
    pub oracle_contract_root_sha256: String,
    pub horizon: K2GoalHorizonV1,
    pub created_at_unix_ms: u64,
}

#[derive(Serialize)]
struct K2GoalEnvelopeDigestV1<'a> {
    schema: &'static str,
    provenance: K2EvidenceProvenanceV1,
    environment_root_sha256: &'a str,
    goal_predicate_root_sha256: &'a str,
    expected_terminal_tree_root_sha256: &'a str,
    expected_goal_store_snapshot_root_sha256: &'a str,
    constraints_root_sha256: &'a str,
    oracle_contract_root_sha256: &'a str,
    horizon: K2GoalHorizonV1,
    created_at_unix_ms: u64,
}

impl K2GoalEnvelopeV1 {
    pub fn seal(
        provenance: K2EvidenceProvenanceV1,
        environment_root_sha256: String,
        expected_terminal_tree_root_sha256: String,
        expected_goal_store_snapshot_root_sha256: String,
        constraints_root_sha256: String,
        oracle_contract_root_sha256: String,
        created_at_unix_ms: u64,
    ) -> K2GoalEnvironmentResultV1<Self> {
        let goal_predicate_root_sha256 = canonical_root(&(
            K2_GOAL_PREDICATE_SCHEMA_V1,
            "workspace_tree_root_equals",
            expected_terminal_tree_root_sha256.as_str(),
        ))?;
        let mut goal = Self {
            schema: K2_GOAL_ENVELOPE_SCHEMA_V1.to_owned(),
            goal_envelope_root_sha256: String::new(),
            provenance,
            environment_root_sha256,
            goal_predicate_root_sha256,
            expected_terminal_tree_root_sha256,
            expected_goal_store_snapshot_root_sha256,
            constraints_root_sha256,
            oracle_contract_root_sha256,
            horizon: K2GoalHorizonV1::SingleSandboxTerminal,
            created_at_unix_ms,
        };
        goal.goal_envelope_root_sha256 = goal.expected_root()?;
        goal.validate()?;
        Ok(goal)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        for (root, reason) in [
            (
                self.goal_envelope_root_sha256.as_str(),
                "k2_goal_root_invalid",
            ),
            (
                self.environment_root_sha256.as_str(),
                "k2_goal_environment_root_invalid",
            ),
            (
                self.expected_terminal_tree_root_sha256.as_str(),
                "k2_goal_expected_tree_root_invalid",
            ),
            (
                self.expected_goal_store_snapshot_root_sha256.as_str(),
                "k2_goal_store_snapshot_root_invalid",
            ),
            (
                self.constraints_root_sha256.as_str(),
                "k2_goal_constraints_root_invalid",
            ),
            (
                self.oracle_contract_root_sha256.as_str(),
                "k2_goal_oracle_contract_root_invalid",
            ),
        ] {
            require_root(root, reason)?;
        }
        let expected_predicate_root = canonical_root(&(
            K2_GOAL_PREDICATE_SCHEMA_V1,
            "workspace_tree_root_equals",
            self.expected_terminal_tree_root_sha256.as_str(),
        ))?;
        if self.schema != K2_GOAL_ENVELOPE_SCHEMA_V1
            || self.horizon != K2GoalHorizonV1::SingleSandboxTerminal
            || self.goal_predicate_root_sha256 != expected_predicate_root
            || self.goal_envelope_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_goal_envelope_invalid",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        self.validate()?;
        canonical_json_bytes(self).map_err(|_| K2GoalEnvironmentErrorV1::Serialization)
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&K2GoalEnvelopeDigestV1 {
            schema: K2_GOAL_ENVELOPE_SCHEMA_V1,
            provenance: self.provenance,
            environment_root_sha256: &self.environment_root_sha256,
            goal_predicate_root_sha256: &self.goal_predicate_root_sha256,
            expected_terminal_tree_root_sha256: &self.expected_terminal_tree_root_sha256,
            expected_goal_store_snapshot_root_sha256: &self
                .expected_goal_store_snapshot_root_sha256,
            constraints_root_sha256: &self.constraints_root_sha256,
            oracle_contract_root_sha256: &self.oracle_contract_root_sha256,
            horizon: self.horizon,
            created_at_unix_ms: self.created_at_unix_ms,
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2GoalEnvironmentBudgetV1 {
    pub maximum_alternatives: u64,
    pub maximum_probes: u64,
    pub maximum_events_per_episode: u64,
    pub maximum_event_bytes: u64,
    pub maximum_episode_bytes: u64,
    pub maximum_retained_capability_episodes: u64,
}

impl K2GoalEnvironmentBudgetV1 {
    #[must_use]
    pub const fn preregistered_v1() -> Self {
        Self {
            maximum_alternatives: K2_MAX_ALTERNATIVES_V1 as u64,
            maximum_probes: 1,
            maximum_events_per_episode: K2_MAX_EVENTS_PER_EPISODE_V1,
            maximum_event_bytes: K2_MAX_EVENT_BYTES_V1,
            maximum_episode_bytes: K2_MAX_EPISODE_BYTES_V1,
            maximum_retained_capability_episodes: K2_MAX_RETAINED_CAPABILITY_EPISODES_V1,
        }
    }

    pub fn root(&self) -> K2GoalEnvironmentResultV1<String> {
        if self != &Self::preregistered_v1() {
            return Err(K2GoalEnvironmentErrorV1::Invalid("k2_budget_invalid"));
        }
        canonical_root(&(K2_BUDGET_SCHEMA_V1, self))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2K1ActionRefInputV1 {
    pub provenance: K2EvidenceProvenanceV1,
    pub applicability_environment_root_sha256: String,
    pub applicability_receipt_root_sha256: String,
    pub operation_plan_root_sha256: String,
    pub predicted_consequence_root_sha256: String,
    pub fixture_effect_root_sha256: Option<String>,
    pub law_certificate_root_sha256: Option<String>,
    pub epistemic_registry_member_root_sha256: Option<String>,
    pub bundle_v4_root_sha256: Option<String>,
    pub execution_certificate_root_sha256: Option<String>,
    pub applicability_guard_root_sha256: Option<String>,
    pub effect_contract_root_sha256: Option<String>,
    pub semantic_class_root_sha256: Option<String>,
    pub role_topology_root_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2K1ActionRefV1 {
    pub schema: String,
    pub action_root_sha256: String,
    pub provenance: K2EvidenceProvenanceV1,
    pub applicability_environment_root_sha256: String,
    pub applicability_receipt_root_sha256: String,
    pub operation_plan_root_sha256: String,
    pub predicted_consequence_root_sha256: String,
    pub fixture_effect_root_sha256: Option<String>,
    pub law_certificate_root_sha256: Option<String>,
    pub epistemic_registry_member_root_sha256: Option<String>,
    pub bundle_v4_root_sha256: Option<String>,
    pub execution_certificate_root_sha256: Option<String>,
    pub applicability_guard_root_sha256: Option<String>,
    pub effect_contract_root_sha256: Option<String>,
    pub semantic_class_root_sha256: Option<String>,
    pub role_topology_root_sha256: Option<String>,
}

#[derive(Serialize)]
struct K2K1ActionRefDigestV1<'a> {
    schema: &'static str,
    provenance: K2EvidenceProvenanceV1,
    applicability_environment_root_sha256: &'a str,
    applicability_receipt_root_sha256: &'a str,
    operation_plan_root_sha256: &'a str,
    predicted_consequence_root_sha256: &'a str,
    fixture_effect_root_sha256: Option<&'a str>,
    law_certificate_root_sha256: Option<&'a str>,
    epistemic_registry_member_root_sha256: Option<&'a str>,
    bundle_v4_root_sha256: Option<&'a str>,
    execution_certificate_root_sha256: Option<&'a str>,
    applicability_guard_root_sha256: Option<&'a str>,
    effect_contract_root_sha256: Option<&'a str>,
    semantic_class_root_sha256: Option<&'a str>,
    role_topology_root_sha256: Option<&'a str>,
}

impl K2K1ActionRefV1 {
    pub fn seal(input: K2K1ActionRefInputV1) -> K2GoalEnvironmentResultV1<Self> {
        let mut action = Self {
            schema: K2_ACTION_REF_SCHEMA_V1.to_owned(),
            action_root_sha256: String::new(),
            provenance: input.provenance,
            applicability_environment_root_sha256: input.applicability_environment_root_sha256,
            applicability_receipt_root_sha256: input.applicability_receipt_root_sha256,
            operation_plan_root_sha256: input.operation_plan_root_sha256,
            predicted_consequence_root_sha256: input.predicted_consequence_root_sha256,
            fixture_effect_root_sha256: input.fixture_effect_root_sha256,
            law_certificate_root_sha256: input.law_certificate_root_sha256,
            epistemic_registry_member_root_sha256: input.epistemic_registry_member_root_sha256,
            bundle_v4_root_sha256: input.bundle_v4_root_sha256,
            execution_certificate_root_sha256: input.execution_certificate_root_sha256,
            applicability_guard_root_sha256: input.applicability_guard_root_sha256,
            effect_contract_root_sha256: input.effect_contract_root_sha256,
            semantic_class_root_sha256: input.semantic_class_root_sha256,
            role_topology_root_sha256: input.role_topology_root_sha256,
        };
        action.action_root_sha256 = action.expected_root()?;
        action.validate()?;
        Ok(action)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        for root in [
            self.action_root_sha256.as_str(),
            self.applicability_environment_root_sha256.as_str(),
            self.applicability_receipt_root_sha256.as_str(),
            self.operation_plan_root_sha256.as_str(),
            self.predicted_consequence_root_sha256.as_str(),
        ] {
            require_root(root, "k2_action_required_root_invalid")?;
        }
        let certificate_roots = [
            self.law_certificate_root_sha256.as_deref(),
            self.epistemic_registry_member_root_sha256.as_deref(),
            self.bundle_v4_root_sha256.as_deref(),
            self.execution_certificate_root_sha256.as_deref(),
            self.applicability_guard_root_sha256.as_deref(),
            self.effect_contract_root_sha256.as_deref(),
            self.semantic_class_root_sha256.as_deref(),
            self.role_topology_root_sha256.as_deref(),
        ];
        if certificate_roots
            .iter()
            .flatten()
            .any(|root| !valid_nonzero_sha256(root))
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_action_certificate_root_invalid",
            ));
        }
        let provenance_valid = match self.provenance {
            K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest => {
                self.fixture_effect_root_sha256
                    .as_deref()
                    .is_some_and(valid_nonzero_sha256)
                    && certificate_roots.iter().all(Option::is_none)
            }
            K2EvidenceProvenanceV1::CertificateBoundK1 => {
                self.fixture_effect_root_sha256.is_none()
                    && certificate_roots.iter().all(Option::is_some)
            }
        };
        if self.schema != K2_ACTION_REF_SCHEMA_V1
            || !provenance_valid
            || self.action_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid("k2_action_ref_invalid"));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&K2K1ActionRefDigestV1 {
            schema: K2_ACTION_REF_SCHEMA_V1,
            provenance: self.provenance,
            applicability_environment_root_sha256: &self.applicability_environment_root_sha256,
            applicability_receipt_root_sha256: &self.applicability_receipt_root_sha256,
            operation_plan_root_sha256: &self.operation_plan_root_sha256,
            predicted_consequence_root_sha256: &self.predicted_consequence_root_sha256,
            fixture_effect_root_sha256: self.fixture_effect_root_sha256.as_deref(),
            law_certificate_root_sha256: self.law_certificate_root_sha256.as_deref(),
            epistemic_registry_member_root_sha256: self
                .epistemic_registry_member_root_sha256
                .as_deref(),
            bundle_v4_root_sha256: self.bundle_v4_root_sha256.as_deref(),
            execution_certificate_root_sha256: self.execution_certificate_root_sha256.as_deref(),
            applicability_guard_root_sha256: self.applicability_guard_root_sha256.as_deref(),
            effect_contract_root_sha256: self.effect_contract_root_sha256.as_deref(),
            semantic_class_root_sha256: self.semantic_class_root_sha256.as_deref(),
            role_topology_root_sha256: self.role_topology_root_sha256.as_deref(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2K1VocabularySnapshotV1 {
    pub schema: String,
    pub snapshot_root_sha256: String,
    pub provenance: K2EvidenceProvenanceV1,
    pub epistemic_registry_revision: Option<u64>,
    pub epistemic_registry_root_sha256: Option<String>,
    pub actions: Vec<K2K1ActionRefV1>,
    pub captured_at_unix_ms: u64,
}

#[derive(Serialize)]
struct K2K1VocabularySnapshotDigestV1<'a> {
    schema: &'static str,
    provenance: K2EvidenceProvenanceV1,
    epistemic_registry_revision: Option<u64>,
    epistemic_registry_root_sha256: Option<&'a str>,
    actions: &'a [K2K1ActionRefV1],
    captured_at_unix_ms: u64,
}

impl K2K1VocabularySnapshotV1 {
    pub fn seal(
        provenance: K2EvidenceProvenanceV1,
        epistemic_registry_revision: Option<u64>,
        epistemic_registry_root_sha256: Option<String>,
        mut actions: Vec<K2K1ActionRefV1>,
        captured_at_unix_ms: u64,
    ) -> K2GoalEnvironmentResultV1<Self> {
        actions.sort_by(|left, right| left.action_root_sha256.cmp(&right.action_root_sha256));
        let mut snapshot = Self {
            schema: K2_VOCABULARY_SNAPSHOT_SCHEMA_V1.to_owned(),
            snapshot_root_sha256: String::new(),
            provenance,
            epistemic_registry_revision,
            epistemic_registry_root_sha256,
            actions,
            captured_at_unix_ms,
        };
        snapshot.snapshot_root_sha256 = snapshot.expected_root()?;
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        require_root(
            &self.snapshot_root_sha256,
            "k2_vocabulary_snapshot_root_invalid",
        )?;
        if self.actions.len() < 2 || self.actions.len() > K2_MAX_ALTERNATIVES_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_vocabulary_size_invalid",
            ));
        }
        for action in &self.actions {
            action.validate()?;
            if action.provenance != self.provenance {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_vocabulary_provenance_mismatch",
                ));
            }
        }
        if !self
            .actions
            .windows(2)
            .all(|pair| pair[0].action_root_sha256 < pair[1].action_root_sha256)
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_vocabulary_action_order_invalid",
            ));
        }
        let registry_valid = match self.provenance {
            K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest => {
                self.epistemic_registry_revision.is_none()
                    && self.epistemic_registry_root_sha256.is_none()
                    && roots_are_unique(
                        self.actions
                            .iter()
                            .filter_map(|action| action.fixture_effect_root_sha256.as_deref()),
                    )
                    && roots_are_unique(
                        self.actions
                            .iter()
                            .map(|action| action.predicted_consequence_root_sha256.as_str()),
                    )
            }
            K2EvidenceProvenanceV1::CertificateBoundK1 => {
                self.epistemic_registry_revision
                    .is_some_and(|revision| revision > 0)
                    && self
                        .epistemic_registry_root_sha256
                        .as_deref()
                        .is_some_and(valid_nonzero_sha256)
                    && roots_are_unique(
                        self.actions
                            .iter()
                            .filter_map(|action| action.law_certificate_root_sha256.as_deref()),
                    )
                    && roots_are_unique(
                        self.actions
                            .iter()
                            .filter_map(|action| action.semantic_class_root_sha256.as_deref()),
                    )
                    && roots_are_unique(
                        self.actions
                            .iter()
                            .filter_map(|action| action.effect_contract_root_sha256.as_deref()),
                    )
                    && roots_are_unique(
                        self.actions
                            .iter()
                            .map(|action| action.predicted_consequence_root_sha256.as_str()),
                    )
            }
        };
        if self.schema != K2_VOCABULARY_SNAPSHOT_SCHEMA_V1
            || !registry_valid
            || self.snapshot_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_vocabulary_snapshot_invalid",
            ));
        }
        Ok(())
    }

    pub fn action(&self, action_root_sha256: &str) -> Option<&K2K1ActionRefV1> {
        self.actions
            .iter()
            .find(|action| action.action_root_sha256 == action_root_sha256)
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&K2K1VocabularySnapshotDigestV1 {
            schema: K2_VOCABULARY_SNAPSHOT_SCHEMA_V1,
            provenance: self.provenance,
            epistemic_registry_revision: self.epistemic_registry_revision,
            epistemic_registry_root_sha256: self.epistemic_registry_root_sha256.as_deref(),
            actions: &self.actions,
            captured_at_unix_ms: self.captured_at_unix_ms,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2AlternativeV1 {
    pub action_root_sha256: String,
    pub applicability_environment_root_sha256: String,
    pub applicability_receipt_root_sha256: String,
    pub operation_plan_root_sha256: String,
    pub predicted_consequence_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2AlternativeSetV1 {
    pub schema: String,
    pub alternative_set_root_sha256: String,
    pub provenance: K2EvidenceProvenanceV1,
    pub vocabulary_snapshot_root_sha256: String,
    pub environment_root_sha256: String,
    pub alternatives: Vec<K2AlternativeV1>,
}

#[derive(Serialize)]
struct K2AlternativeSetDigestV1<'a> {
    schema: &'static str,
    provenance: K2EvidenceProvenanceV1,
    vocabulary_snapshot_root_sha256: &'a str,
    environment_root_sha256: &'a str,
    alternatives: &'a [K2AlternativeV1],
}

impl K2AlternativeSetV1 {
    pub fn seal(
        vocabulary: &K2K1VocabularySnapshotV1,
        environment_root_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        vocabulary.validate()?;
        let alternatives = vocabulary
            .actions
            .iter()
            .map(|action| K2AlternativeV1 {
                action_root_sha256: action.action_root_sha256.clone(),
                applicability_environment_root_sha256: action
                    .applicability_environment_root_sha256
                    .clone(),
                applicability_receipt_root_sha256: action.applicability_receipt_root_sha256.clone(),
                operation_plan_root_sha256: action.operation_plan_root_sha256.clone(),
                predicted_consequence_root_sha256: action.predicted_consequence_root_sha256.clone(),
            })
            .collect();
        let mut set = Self {
            schema: K2_ALTERNATIVE_SET_SCHEMA_V1.to_owned(),
            alternative_set_root_sha256: String::new(),
            provenance: vocabulary.provenance,
            vocabulary_snapshot_root_sha256: vocabulary.snapshot_root_sha256.clone(),
            environment_root_sha256,
            alternatives,
        };
        set.alternative_set_root_sha256 = set.expected_root()?;
        set.validate(vocabulary)?;
        Ok(set)
    }

    pub fn validate(&self, vocabulary: &K2K1VocabularySnapshotV1) -> K2GoalEnvironmentResultV1<()> {
        vocabulary.validate()?;
        require_root(
            &self.alternative_set_root_sha256,
            "k2_alternative_set_root_invalid",
        )?;
        require_root(
            &self.environment_root_sha256,
            "k2_alternative_environment_root_invalid",
        )?;
        if self.schema != K2_ALTERNATIVE_SET_SCHEMA_V1
            || self.provenance != vocabulary.provenance
            || self.vocabulary_snapshot_root_sha256 != vocabulary.snapshot_root_sha256
            || self.alternatives.len() != vocabulary.actions.len()
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_alternative_set_binding_invalid",
            ));
        }
        for (alternative, action) in self.alternatives.iter().zip(&vocabulary.actions) {
            if alternative.action_root_sha256 != action.action_root_sha256
                || alternative.applicability_environment_root_sha256 != self.environment_root_sha256
                || alternative.applicability_environment_root_sha256
                    != action.applicability_environment_root_sha256
                || alternative.applicability_receipt_root_sha256
                    != action.applicability_receipt_root_sha256
                || alternative.operation_plan_root_sha256 != action.operation_plan_root_sha256
                || alternative.predicted_consequence_root_sha256
                    != action.predicted_consequence_root_sha256
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_alternative_binding_invalid",
                ));
            }
        }
        if self.alternative_set_root_sha256 != self.expected_root()? {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_alternative_set_invalid",
            ));
        }
        Ok(())
    }

    pub fn alternative(&self, action_root_sha256: &str) -> Option<&K2AlternativeV1> {
        self.alternatives
            .iter()
            .find(|alternative| alternative.action_root_sha256 == action_root_sha256)
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&K2AlternativeSetDigestV1 {
            schema: K2_ALTERNATIVE_SET_SCHEMA_V1,
            provenance: self.provenance,
            vocabulary_snapshot_root_sha256: &self.vocabulary_snapshot_root_sha256,
            environment_root_sha256: &self.environment_root_sha256,
            alternatives: &self.alternatives,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2ExactOracleManifestV1 {
    pub schema: String,
    pub manifest_root_sha256: String,
    pub executable_sha256: String,
}

impl K2ExactOracleManifestV1 {
    pub fn seal(executable_sha256: String) -> K2GoalEnvironmentResultV1<Self> {
        require_root(&executable_sha256, "k2_oracle_executable_sha_invalid")?;
        let mut manifest = Self {
            schema: K2_ORACLE_MANIFEST_SCHEMA_V1.to_owned(),
            manifest_root_sha256: String::new(),
            executable_sha256,
        };
        manifest.manifest_root_sha256 = canonical_root(&(
            K2_ORACLE_MANIFEST_SCHEMA_V1,
            manifest.executable_sha256.as_str(),
        ))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        require_root(&self.executable_sha256, "k2_oracle_executable_sha_invalid")?;
        if self.schema != K2_ORACLE_MANIFEST_SCHEMA_V1
            || self.manifest_root_sha256
                != canonical_root(&(
                    K2_ORACLE_MANIFEST_SCHEMA_V1,
                    self.executable_sha256.as_str(),
                ))?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_oracle_manifest_invalid",
            ));
        }
        Ok(())
    }
}

pub struct K2DecisionFreezeInputV1<'a> {
    pub episode_id_sha256: String,
    pub goal: &'a K2GoalEnvelopeV1,
    pub vocabulary: &'a K2K1VocabularySnapshotV1,
    pub alternatives: &'a K2AlternativeSetV1,
    pub budget: K2GoalEnvironmentBudgetV1,
    pub selector_contract_root_sha256: String,
    pub selector_executable_sha256: String,
    pub oracle_manifest: &'a K2ExactOracleManifestV1,
    pub sandbox_worker_sha256: String,
    pub deterministic_seed_sha256: String,
    pub observed_registry_revision: Option<u64>,
    pub observed_registry_root_sha256: Option<String>,
    pub frozen_at_unix_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2DecisionFreezeV1 {
    pub schema: String,
    pub decision_freeze_root_sha256: String,
    pub episode_id_sha256: String,
    pub provenance: K2EvidenceProvenanceV1,
    pub goal_envelope_root_sha256: String,
    pub vocabulary_snapshot_root_sha256: String,
    pub alternative_set_root_sha256: String,
    pub initial_environment_root_sha256: String,
    pub selector_contract_root_sha256: String,
    pub selector_executable_sha256: String,
    pub oracle_manifest_root_sha256: String,
    pub oracle_executable_sha256: String,
    pub sandbox_worker_sha256: String,
    pub budget_root_sha256: String,
    pub deterministic_seed_sha256: String,
    pub previous_journal_entry_root_sha256: Option<String>,
    pub frozen_at_unix_ms: u64,
    pub authority: K2AuthorityBoundaryV1,
}

#[derive(Serialize)]
struct K2DecisionFreezeDigestV1<'a> {
    schema: &'static str,
    episode_id_sha256: &'a str,
    provenance: K2EvidenceProvenanceV1,
    goal_envelope_root_sha256: &'a str,
    vocabulary_snapshot_root_sha256: &'a str,
    alternative_set_root_sha256: &'a str,
    initial_environment_root_sha256: &'a str,
    selector_contract_root_sha256: &'a str,
    selector_executable_sha256: &'a str,
    oracle_manifest_root_sha256: &'a str,
    oracle_executable_sha256: &'a str,
    sandbox_worker_sha256: &'a str,
    budget_root_sha256: &'a str,
    deterministic_seed_sha256: &'a str,
    previous_journal_entry_root_sha256: Option<&'a str>,
    frozen_at_unix_ms: u64,
    authority: &'a K2AuthorityBoundaryV1,
}

impl K2DecisionFreezeV1 {
    pub fn seal(input: K2DecisionFreezeInputV1<'_>) -> K2GoalEnvironmentResultV1<Self> {
        input.goal.validate()?;
        input.vocabulary.validate()?;
        input.alternatives.validate(input.vocabulary)?;
        input.oracle_manifest.validate()?;
        let budget_root_sha256 = input.budget.root()?;
        if input.goal.provenance != input.vocabulary.provenance
            || input.goal.provenance != input.alternatives.provenance
            || input.goal.environment_root_sha256 != input.alternatives.environment_root_sha256
            || input.goal.oracle_contract_root_sha256 != input.oracle_manifest.manifest_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_decision_input_binding_invalid",
            ));
        }
        match input.vocabulary.provenance {
            K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest => {
                if input.observed_registry_revision.is_some()
                    || input.observed_registry_root_sha256.is_some()
                {
                    return Err(K2GoalEnvironmentErrorV1::Invalid(
                        "k2_fixture_registry_binding_forbidden",
                    ));
                }
            }
            K2EvidenceProvenanceV1::CertificateBoundK1 => {
                if input.observed_registry_revision != input.vocabulary.epistemic_registry_revision
                    || input.observed_registry_root_sha256
                        != input.vocabulary.epistemic_registry_root_sha256
                {
                    return Err(K2GoalEnvironmentErrorV1::Invalid(
                        "k2_registry_stale_before_freeze",
                    ));
                }
            }
        }
        for root in [
            input.episode_id_sha256.as_str(),
            input.selector_contract_root_sha256.as_str(),
            input.selector_executable_sha256.as_str(),
            input.sandbox_worker_sha256.as_str(),
            input.deterministic_seed_sha256.as_str(),
        ] {
            require_root(root, "k2_decision_input_root_invalid")?;
        }
        if !roots_are_unique([
            input.selector_executable_sha256.as_str(),
            input.oracle_manifest.executable_sha256.as_str(),
            input.sandbox_worker_sha256.as_str(),
        ]) {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_executable_identity_not_independent",
            ));
        }
        let mut freeze = Self {
            schema: K2_DECISION_FREEZE_SCHEMA_V1.to_owned(),
            decision_freeze_root_sha256: String::new(),
            episode_id_sha256: input.episode_id_sha256,
            provenance: input.goal.provenance,
            goal_envelope_root_sha256: input.goal.goal_envelope_root_sha256.clone(),
            vocabulary_snapshot_root_sha256: input.vocabulary.snapshot_root_sha256.clone(),
            alternative_set_root_sha256: input.alternatives.alternative_set_root_sha256.clone(),
            initial_environment_root_sha256: input.goal.environment_root_sha256.clone(),
            selector_contract_root_sha256: input.selector_contract_root_sha256,
            selector_executable_sha256: input.selector_executable_sha256,
            oracle_manifest_root_sha256: input.oracle_manifest.manifest_root_sha256.clone(),
            oracle_executable_sha256: input.oracle_manifest.executable_sha256.clone(),
            sandbox_worker_sha256: input.sandbox_worker_sha256,
            budget_root_sha256,
            deterministic_seed_sha256: input.deterministic_seed_sha256,
            previous_journal_entry_root_sha256: None,
            frozen_at_unix_ms: input.frozen_at_unix_ms,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        freeze.decision_freeze_root_sha256 = freeze.expected_root()?;
        freeze.validate(
            input.goal,
            input.vocabulary,
            input.alternatives,
            &input.budget,
            input.oracle_manifest,
        )?;
        Ok(freeze)
    }

    pub fn validate(
        &self,
        goal: &K2GoalEnvelopeV1,
        vocabulary: &K2K1VocabularySnapshotV1,
        alternatives: &K2AlternativeSetV1,
        budget: &K2GoalEnvironmentBudgetV1,
        oracle_manifest: &K2ExactOracleManifestV1,
    ) -> K2GoalEnvironmentResultV1<()> {
        goal.validate()?;
        vocabulary.validate()?;
        alternatives.validate(vocabulary)?;
        oracle_manifest.validate()?;
        self.authority.validate()?;
        for root in [
            self.decision_freeze_root_sha256.as_str(),
            self.episode_id_sha256.as_str(),
            self.selector_contract_root_sha256.as_str(),
            self.selector_executable_sha256.as_str(),
            self.sandbox_worker_sha256.as_str(),
            self.deterministic_seed_sha256.as_str(),
        ] {
            require_root(root, "k2_decision_root_invalid")?;
        }
        if self.schema != K2_DECISION_FREEZE_SCHEMA_V1
            || self.provenance != goal.provenance
            || self.provenance != vocabulary.provenance
            || self.goal_envelope_root_sha256 != goal.goal_envelope_root_sha256
            || self.vocabulary_snapshot_root_sha256 != vocabulary.snapshot_root_sha256
            || self.alternative_set_root_sha256 != alternatives.alternative_set_root_sha256
            || self.initial_environment_root_sha256 != goal.environment_root_sha256
            || self.initial_environment_root_sha256 != alternatives.environment_root_sha256
            || self.oracle_manifest_root_sha256 != oracle_manifest.manifest_root_sha256
            || self.oracle_executable_sha256 != oracle_manifest.executable_sha256
            || goal.oracle_contract_root_sha256 != oracle_manifest.manifest_root_sha256
            || self.budget_root_sha256 != budget.root()?
            || self.previous_journal_entry_root_sha256.is_some()
            || !roots_are_unique([
                self.selector_executable_sha256.as_str(),
                self.oracle_executable_sha256.as_str(),
                self.sandbox_worker_sha256.as_str(),
            ])
            || self.decision_freeze_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_decision_freeze_invalid",
            ));
        }
        Ok(())
    }

    pub(crate) fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.decision_freeze_root_sha256.as_str(),
            self.episode_id_sha256.as_str(),
            self.goal_envelope_root_sha256.as_str(),
            self.vocabulary_snapshot_root_sha256.as_str(),
            self.alternative_set_root_sha256.as_str(),
            self.initial_environment_root_sha256.as_str(),
            self.selector_contract_root_sha256.as_str(),
            self.selector_executable_sha256.as_str(),
            self.oracle_manifest_root_sha256.as_str(),
            self.oracle_executable_sha256.as_str(),
            self.sandbox_worker_sha256.as_str(),
            self.budget_root_sha256.as_str(),
            self.deterministic_seed_sha256.as_str(),
        ] {
            require_root(root, "k2_persisted_decision_root_invalid")?;
        }
        if self.schema != K2_DECISION_FREEZE_SCHEMA_V1
            || self.previous_journal_entry_root_sha256.is_some()
            || !roots_are_unique([
                self.selector_executable_sha256.as_str(),
                self.oracle_executable_sha256.as_str(),
                self.sandbox_worker_sha256.as_str(),
            ])
            || self.decision_freeze_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_persisted_decision_freeze_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&K2DecisionFreezeDigestV1 {
            schema: K2_DECISION_FREEZE_SCHEMA_V1,
            episode_id_sha256: &self.episode_id_sha256,
            provenance: self.provenance,
            goal_envelope_root_sha256: &self.goal_envelope_root_sha256,
            vocabulary_snapshot_root_sha256: &self.vocabulary_snapshot_root_sha256,
            alternative_set_root_sha256: &self.alternative_set_root_sha256,
            initial_environment_root_sha256: &self.initial_environment_root_sha256,
            selector_contract_root_sha256: &self.selector_contract_root_sha256,
            selector_executable_sha256: &self.selector_executable_sha256,
            oracle_manifest_root_sha256: &self.oracle_manifest_root_sha256,
            oracle_executable_sha256: &self.oracle_executable_sha256,
            sandbox_worker_sha256: &self.sandbox_worker_sha256,
            budget_root_sha256: &self.budget_root_sha256,
            deterministic_seed_sha256: &self.deterministic_seed_sha256,
            previous_journal_entry_root_sha256: self.previous_journal_entry_root_sha256.as_deref(),
            frozen_at_unix_ms: self.frozen_at_unix_ms,
            authority: &self.authority,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2AlternativePredictionV1 {
    pub action_root_sha256: String,
    pub predicted_terminal_tree_root_sha256: String,
    pub predicted_goal_satisfied: bool,
    pub prediction_evidence_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2AlternativePredictionSetV1 {
    pub schema: String,
    pub prediction_set_root_sha256: String,
    pub decision_freeze_root_sha256: String,
    pub provenance: K2EvidenceProvenanceV1,
    pub predictor_schema: String,
    pub predictor_executable_sha256: String,
    pub goal_envelope_root_sha256: String,
    pub vocabulary_snapshot_root_sha256: String,
    pub alternative_set_root_sha256: String,
    pub creation_sequence: u64,
    pub learned: bool,
    pub predictions: Vec<K2AlternativePredictionV1>,
}

#[derive(Serialize)]
struct K2AlternativePredictionSetDigestV1<'a> {
    schema: &'static str,
    decision_freeze_root_sha256: &'a str,
    provenance: K2EvidenceProvenanceV1,
    predictor_schema: &'a str,
    predictor_executable_sha256: &'a str,
    goal_envelope_root_sha256: &'a str,
    vocabulary_snapshot_root_sha256: &'a str,
    alternative_set_root_sha256: &'a str,
    creation_sequence: u64,
    learned: bool,
    predictions: &'a [K2AlternativePredictionV1],
}

impl K2AlternativePredictionSetV1 {
    pub fn prepared_capability_v1(
        freeze: &K2DecisionFreezeV1,
        goal: &K2GoalEnvelopeV1,
        vocabulary: &K2K1VocabularySnapshotV1,
        alternatives: &K2AlternativeSetV1,
        budget: &K2GoalEnvironmentBudgetV1,
        oracle_manifest: &K2ExactOracleManifestV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        freeze.validate(goal, vocabulary, alternatives, budget, oracle_manifest)?;
        if freeze.provenance != K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_certificate_bound_runtime_closed",
            ));
        }
        let predictions = alternatives
            .alternatives
            .iter()
            .map(|alternative| {
                let predicted_goal_satisfied = alternative.predicted_consequence_root_sha256
                    == goal.expected_terminal_tree_root_sha256;
                let prediction_evidence_root_sha256 = canonical_root(&(
                    K2_PREPARED_SELECTOR_SCHEMA_V1,
                    freeze.decision_freeze_root_sha256.as_str(),
                    alternative.action_root_sha256.as_str(),
                    alternative.predicted_consequence_root_sha256.as_str(),
                    predicted_goal_satisfied,
                ))?;
                Ok(K2AlternativePredictionV1 {
                    action_root_sha256: alternative.action_root_sha256.clone(),
                    predicted_terminal_tree_root_sha256: alternative
                        .predicted_consequence_root_sha256
                        .clone(),
                    predicted_goal_satisfied,
                    prediction_evidence_root_sha256,
                })
            })
            .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
        let mut set = Self {
            schema: K2_PREDICTION_SET_SCHEMA_V1.to_owned(),
            prediction_set_root_sha256: String::new(),
            decision_freeze_root_sha256: freeze.decision_freeze_root_sha256.clone(),
            provenance: freeze.provenance,
            predictor_schema: K2_PREPARED_SELECTOR_SCHEMA_V1.to_owned(),
            predictor_executable_sha256: freeze.selector_executable_sha256.clone(),
            goal_envelope_root_sha256: goal.goal_envelope_root_sha256.clone(),
            vocabulary_snapshot_root_sha256: vocabulary.snapshot_root_sha256.clone(),
            alternative_set_root_sha256: alternatives.alternative_set_root_sha256.clone(),
            creation_sequence: 1,
            learned: false,
            predictions,
        };
        set.prediction_set_root_sha256 = set.expected_root()?;
        set.validate(freeze, goal, vocabulary, alternatives)?;
        Ok(set)
    }

    pub fn validate(
        &self,
        freeze: &K2DecisionFreezeV1,
        goal: &K2GoalEnvelopeV1,
        vocabulary: &K2K1VocabularySnapshotV1,
        alternatives: &K2AlternativeSetV1,
    ) -> K2GoalEnvironmentResultV1<()> {
        if self.schema != K2_PREDICTION_SET_SCHEMA_V1
            || self.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
            || self.provenance != K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest
            || self.provenance != freeze.provenance
            || self.predictor_schema != K2_PREPARED_SELECTOR_SCHEMA_V1
            || self.predictor_executable_sha256 != freeze.selector_executable_sha256
            || self.goal_envelope_root_sha256 != goal.goal_envelope_root_sha256
            || self.vocabulary_snapshot_root_sha256 != vocabulary.snapshot_root_sha256
            || self.alternative_set_root_sha256 != alternatives.alternative_set_root_sha256
            || self.creation_sequence != 1
            || self.learned
            || self.predictions.len() != alternatives.alternatives.len()
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_prediction_set_binding_invalid",
            ));
        }
        for (prediction, alternative) in self.predictions.iter().zip(&alternatives.alternatives) {
            require_root(
                &prediction.prediction_evidence_root_sha256,
                "k2_prediction_evidence_root_invalid",
            )?;
            let expected_satisfaction = alternative.predicted_consequence_root_sha256
                == goal.expected_terminal_tree_root_sha256;
            if prediction.action_root_sha256 != alternative.action_root_sha256
                || prediction.predicted_terminal_tree_root_sha256
                    != alternative.predicted_consequence_root_sha256
                || prediction.predicted_goal_satisfied != expected_satisfaction
                || prediction.prediction_evidence_root_sha256
                    != canonical_root(&(
                        K2_PREPARED_SELECTOR_SCHEMA_V1,
                        freeze.decision_freeze_root_sha256.as_str(),
                        alternative.action_root_sha256.as_str(),
                        alternative.predicted_consequence_root_sha256.as_str(),
                        expected_satisfaction,
                    ))?
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid("k2_prediction_invalid"));
            }
        }
        if self.prediction_set_root_sha256 != self.expected_root()? {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_prediction_set_invalid",
            ));
        }
        Ok(())
    }

    pub(crate) fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        for root in [
            self.prediction_set_root_sha256.as_str(),
            self.decision_freeze_root_sha256.as_str(),
            self.predictor_executable_sha256.as_str(),
            self.goal_envelope_root_sha256.as_str(),
            self.vocabulary_snapshot_root_sha256.as_str(),
            self.alternative_set_root_sha256.as_str(),
        ] {
            require_root(root, "k2_persisted_prediction_root_invalid")?;
        }
        if self.schema != K2_PREDICTION_SET_SCHEMA_V1
            || self.provenance != K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest
            || self.predictor_schema != K2_PREPARED_SELECTOR_SCHEMA_V1
            || self.creation_sequence != 1
            || self.learned
            || self.predictions.len() < 2
            || self.predictions.len() > K2_MAX_ALTERNATIVES_V1
            || !roots_are_unique(
                self.predictions
                    .iter()
                    .map(|prediction| prediction.action_root_sha256.as_str()),
            )
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_persisted_prediction_set_invalid",
            ));
        }
        for prediction in &self.predictions {
            for root in [
                prediction.action_root_sha256.as_str(),
                prediction.predicted_terminal_tree_root_sha256.as_str(),
                prediction.prediction_evidence_root_sha256.as_str(),
            ] {
                require_root(root, "k2_persisted_prediction_entry_root_invalid")?;
            }
            if prediction.prediction_evidence_root_sha256
                != canonical_root(&(
                    K2_PREPARED_SELECTOR_SCHEMA_V1,
                    self.decision_freeze_root_sha256.as_str(),
                    prediction.action_root_sha256.as_str(),
                    prediction.predicted_terminal_tree_root_sha256.as_str(),
                    prediction.predicted_goal_satisfied,
                ))?
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_persisted_prediction_entry_invalid",
                ));
            }
        }
        if self.prediction_set_root_sha256 != self.expected_root()? {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_persisted_prediction_set_root_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&K2AlternativePredictionSetDigestV1 {
            schema: K2_PREDICTION_SET_SCHEMA_V1,
            decision_freeze_root_sha256: &self.decision_freeze_root_sha256,
            provenance: self.provenance,
            predictor_schema: &self.predictor_schema,
            predictor_executable_sha256: &self.predictor_executable_sha256,
            goal_envelope_root_sha256: &self.goal_envelope_root_sha256,
            vocabulary_snapshot_root_sha256: &self.vocabulary_snapshot_root_sha256,
            alternative_set_root_sha256: &self.alternative_set_root_sha256,
            creation_sequence: self.creation_sequence,
            learned: self.learned,
            predictions: &self.predictions,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2PreparedSelectionReceiptV1 {
    pub schema: String,
    pub selection_root_sha256: String,
    pub decision_freeze_root_sha256: String,
    pub prediction_set_root_sha256: String,
    pub selected_action_root_sha256: String,
    pub learned: bool,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2PreparedSelectionReceiptV1 {
    pub fn select(
        freeze: &K2DecisionFreezeV1,
        predictions: &K2AlternativePredictionSetV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        if freeze.provenance != K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest
            || predictions.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
            || predictions.learned
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_prepared_selector_scope_invalid",
            ));
        }
        let satisfying = predictions
            .predictions
            .iter()
            .filter(|prediction| prediction.predicted_goal_satisfied)
            .collect::<Vec<_>>();
        if satisfying.len() != 1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid("k2_no_unique_selection"));
        }
        let mut receipt = Self {
            schema: K2_SELECTION_RECEIPT_SCHEMA_V1.to_owned(),
            selection_root_sha256: String::new(),
            decision_freeze_root_sha256: freeze.decision_freeze_root_sha256.clone(),
            prediction_set_root_sha256: predictions.prediction_set_root_sha256.clone(),
            selected_action_root_sha256: satisfying[0].action_root_sha256.clone(),
            learned: false,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        receipt.selection_root_sha256 = receipt.expected_root()?;
        receipt.validate(freeze, predictions)?;
        Ok(receipt)
    }

    pub fn validate(
        &self,
        freeze: &K2DecisionFreezeV1,
        predictions: &K2AlternativePredictionSetV1,
    ) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        let satisfying = predictions
            .predictions
            .iter()
            .filter(|prediction| prediction.predicted_goal_satisfied)
            .collect::<Vec<_>>();
        if self.schema != K2_SELECTION_RECEIPT_SCHEMA_V1
            || self.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
            || self.prediction_set_root_sha256 != predictions.prediction_set_root_sha256
            || satisfying.len() != 1
            || self.selected_action_root_sha256 != satisfying[0].action_root_sha256
            || self.learned
            || self.selection_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_selection_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&(
            K2_SELECTION_RECEIPT_SCHEMA_V1,
            self.decision_freeze_root_sha256.as_str(),
            self.prediction_set_root_sha256.as_str(),
            self.selected_action_root_sha256.as_str(),
            self.learned,
            &self.authority,
        ))
    }
}

pub struct K2LawLabBindingInputV1<'a> {
    pub freeze: &'a K2DecisionFreezeV1,
    pub goal: &'a K2GoalEnvelopeV1,
    pub vocabulary: &'a K2K1VocabularySnapshotV1,
    pub alternatives: &'a K2AlternativeSetV1,
    pub predictions: &'a K2AlternativePredictionSetV1,
    pub selection: &'a K2PreparedSelectionReceiptV1,
    pub request: &'a LawLabSandboxRequestV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LawLabBindingV1 {
    pub schema: String,
    pub binding_root_sha256: String,
    pub episode_id_sha256: String,
    pub decision_freeze_root_sha256: String,
    pub goal_envelope_root_sha256: String,
    pub vocabulary_snapshot_root_sha256: String,
    pub alternative_set_root_sha256: String,
    pub prediction_set_root_sha256: String,
    pub selected_action_root_sha256: String,
    pub law_lab_request_root_sha256: String,
    pub source_tree_root_sha256: String,
    pub executor_manifest_root_sha256: String,
    pub worker_sha256: String,
    pub deterministic_seed_sha256: String,
    pub budget_root_sha256: String,
    pub operation_plan_root_sha256: String,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2LawLabBindingV1 {
    pub fn seal(input: K2LawLabBindingInputV1<'_>) -> K2GoalEnvironmentResultV1<Self> {
        input
            .request
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        input.selection.validate(input.freeze, input.predictions)?;
        input.predictions.validate(
            input.freeze,
            input.goal,
            input.vocabulary,
            input.alternatives,
        )?;
        let selected = input
            .alternatives
            .alternative(&input.selection.selected_action_root_sha256)
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_selected_alternative_missing",
            ))?;
        let operation_plan_root_sha256 = canonical_root(&input.request.operations)?;
        let request_binding_valid = input.request.purpose
            == LawLabSandboxPurposeV1::GeneratedCapabilitySelfTest
            && input.request.candidate_root_sha256 == input.freeze.episode_id_sha256
            && input.request.version_space_root_sha256
                == input.alternatives.alternative_set_root_sha256
            && input.request.durable_prediction_ledger_root_sha256
                == input.predictions.prediction_set_root_sha256
            && input.request.probe_root_sha256 == input.selection.selection_root_sha256
            && input.request.deterministic_seed_sha256 == input.freeze.deterministic_seed_sha256
            && input.request.worker_sha256 == input.freeze.sandbox_worker_sha256
            && input.request.surviving_hypothesis_count
                == input.alternatives.alternatives.len() as u64
            && input.request.precommitted_prediction_count
                == input.predictions.predictions.len() as u64
            && operation_plan_root_sha256 == selected.operation_plan_root_sha256;
        if !request_binding_valid {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_law_lab_request_binding_invalid",
            ));
        }
        let mut binding = Self {
            schema: K2_LAW_LAB_BINDING_SCHEMA_V1.to_owned(),
            binding_root_sha256: String::new(),
            episode_id_sha256: input.freeze.episode_id_sha256.clone(),
            decision_freeze_root_sha256: input.freeze.decision_freeze_root_sha256.clone(),
            goal_envelope_root_sha256: input.goal.goal_envelope_root_sha256.clone(),
            vocabulary_snapshot_root_sha256: input.vocabulary.snapshot_root_sha256.clone(),
            alternative_set_root_sha256: input.alternatives.alternative_set_root_sha256.clone(),
            prediction_set_root_sha256: input.predictions.prediction_set_root_sha256.clone(),
            selected_action_root_sha256: input.selection.selected_action_root_sha256.clone(),
            law_lab_request_root_sha256: input.request.request_root_sha256.clone(),
            source_tree_root_sha256: input.request.source_tree_root_sha256.clone(),
            executor_manifest_root_sha256: input.request.executor_manifest_root_sha256.clone(),
            worker_sha256: input.request.worker_sha256.clone(),
            deterministic_seed_sha256: input.request.deterministic_seed_sha256.clone(),
            budget_root_sha256: input.freeze.budget_root_sha256.clone(),
            operation_plan_root_sha256,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        binding.binding_root_sha256 = binding.expected_root()?;
        binding.validate(input)?;
        Ok(binding)
    }

    pub fn validate(&self, input: K2LawLabBindingInputV1<'_>) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        input
            .request
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        let selected = input
            .alternatives
            .alternative(&input.selection.selected_action_root_sha256)
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_selected_alternative_missing",
            ))?;
        let operation_plan_root = canonical_root(&input.request.operations)?;
        if self.schema != K2_LAW_LAB_BINDING_SCHEMA_V1
            || self.episode_id_sha256 != input.freeze.episode_id_sha256
            || self.decision_freeze_root_sha256 != input.freeze.decision_freeze_root_sha256
            || self.goal_envelope_root_sha256 != input.goal.goal_envelope_root_sha256
            || self.vocabulary_snapshot_root_sha256 != input.vocabulary.snapshot_root_sha256
            || self.alternative_set_root_sha256 != input.alternatives.alternative_set_root_sha256
            || self.prediction_set_root_sha256 != input.predictions.prediction_set_root_sha256
            || self.selected_action_root_sha256 != input.selection.selected_action_root_sha256
            || self.law_lab_request_root_sha256 != input.request.request_root_sha256
            || self.source_tree_root_sha256 != input.request.source_tree_root_sha256
            || self.executor_manifest_root_sha256 != input.request.executor_manifest_root_sha256
            || self.worker_sha256 != input.request.worker_sha256
            || self.deterministic_seed_sha256 != input.request.deterministic_seed_sha256
            || self.budget_root_sha256 != input.freeze.budget_root_sha256
            || self.operation_plan_root_sha256 != selected.operation_plan_root_sha256
            || self.operation_plan_root_sha256 != operation_plan_root
            || self.binding_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_law_lab_binding_invalid",
            ));
        }
        Ok(())
    }

    pub(crate) fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.binding_root_sha256.as_str(),
            self.episode_id_sha256.as_str(),
            self.decision_freeze_root_sha256.as_str(),
            self.goal_envelope_root_sha256.as_str(),
            self.vocabulary_snapshot_root_sha256.as_str(),
            self.alternative_set_root_sha256.as_str(),
            self.prediction_set_root_sha256.as_str(),
            self.selected_action_root_sha256.as_str(),
            self.law_lab_request_root_sha256.as_str(),
            self.source_tree_root_sha256.as_str(),
            self.executor_manifest_root_sha256.as_str(),
            self.worker_sha256.as_str(),
            self.deterministic_seed_sha256.as_str(),
            self.budget_root_sha256.as_str(),
            self.operation_plan_root_sha256.as_str(),
        ] {
            require_root(root, "k2_persisted_law_lab_binding_root_invalid")?;
        }
        if self.schema != K2_LAW_LAB_BINDING_SCHEMA_V1
            || self.binding_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_persisted_law_lab_binding_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&(
            K2_LAW_LAB_BINDING_SCHEMA_V1,
            self.episode_id_sha256.as_str(),
            self.decision_freeze_root_sha256.as_str(),
            self.goal_envelope_root_sha256.as_str(),
            self.vocabulary_snapshot_root_sha256.as_str(),
            self.alternative_set_root_sha256.as_str(),
            self.prediction_set_root_sha256.as_str(),
            self.selected_action_root_sha256.as_str(),
            self.law_lab_request_root_sha256.as_str(),
            self.source_tree_root_sha256.as_str(),
            self.executor_manifest_root_sha256.as_str(),
            self.worker_sha256.as_str(),
            self.deterministic_seed_sha256.as_str(),
            self.budget_root_sha256.as_str(),
            self.operation_plan_root_sha256.as_str(),
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2ExactOracleRequestV1 {
    pub schema: String,
    pub request_root_sha256: String,
    pub oracle_manifest_root_sha256: String,
    pub goal_predicate_root_sha256: String,
    pub law_lab_binding_root_sha256: String,
    pub law_lab_receipt_root_sha256: String,
    pub expected_terminal_tree_root_sha256: String,
    pub observed_terminal_tree_root_sha256: String,
}

impl K2ExactOracleRequestV1 {
    pub fn seal(
        goal: &K2GoalEnvelopeV1,
        freeze: &K2DecisionFreezeV1,
        binding: &K2LawLabBindingV1,
        execution: &LawLabSandboxExecutionV1,
        oracle_manifest: &K2ExactOracleManifestV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        goal.validate()?;
        oracle_manifest.validate()?;
        if goal.goal_envelope_root_sha256 != freeze.goal_envelope_root_sha256
            || binding.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
            || execution.receipt.request_root_sha256 != binding.law_lab_request_root_sha256
            || oracle_manifest.manifest_root_sha256 != freeze.oracle_manifest_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_exact_oracle_request_binding_invalid",
            ));
        }
        let mut request = Self {
            schema: K2_EXACT_ORACLE_REQUEST_SCHEMA_V1.to_owned(),
            request_root_sha256: String::new(),
            oracle_manifest_root_sha256: oracle_manifest.manifest_root_sha256.clone(),
            goal_predicate_root_sha256: goal.goal_predicate_root_sha256.clone(),
            law_lab_binding_root_sha256: binding.binding_root_sha256.clone(),
            law_lab_receipt_root_sha256: execution.receipt.receipt_root_sha256.clone(),
            expected_terminal_tree_root_sha256: goal.expected_terminal_tree_root_sha256.clone(),
            observed_terminal_tree_root_sha256: execution.receipt.post_tree_root_sha256.clone(),
        };
        request.request_root_sha256 = request.expected_root()?;
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        for root in [
            self.request_root_sha256.as_str(),
            self.oracle_manifest_root_sha256.as_str(),
            self.goal_predicate_root_sha256.as_str(),
            self.law_lab_binding_root_sha256.as_str(),
            self.law_lab_receipt_root_sha256.as_str(),
            self.expected_terminal_tree_root_sha256.as_str(),
            self.observed_terminal_tree_root_sha256.as_str(),
        ] {
            require_root(root, "k2_exact_oracle_request_root_invalid")?;
        }
        if self.schema != K2_EXACT_ORACLE_REQUEST_SCHEMA_V1
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_exact_oracle_request_invalid",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        self.validate()?;
        canonical_json_bytes(self).map_err(|_| K2GoalEnvironmentErrorV1::Serialization)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> K2GoalEnvironmentResultV1<Self> {
        let request: Self = serde_json::from_slice(bytes).map_err(|_| {
            K2GoalEnvironmentErrorV1::Invalid("k2_exact_oracle_request_decode_failed")
        })?;
        request.validate()?;
        if request.canonical_bytes()? != bytes {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_exact_oracle_request_not_canonical",
            ));
        }
        Ok(request)
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&(
            K2_EXACT_ORACLE_REQUEST_SCHEMA_V1,
            self.oracle_manifest_root_sha256.as_str(),
            self.goal_predicate_root_sha256.as_str(),
            self.law_lab_binding_root_sha256.as_str(),
            self.law_lab_receipt_root_sha256.as_str(),
            self.expected_terminal_tree_root_sha256.as_str(),
            self.observed_terminal_tree_root_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2ExactOracleOutcomeV1 {
    pub schema: String,
    pub outcome_root_sha256: String,
    pub request_root_sha256: String,
    pub expected_terminal_tree_root_sha256: String,
    pub observed_terminal_tree_root_sha256: String,
    pub goal_satisfied: bool,
}

impl K2ExactOracleOutcomeV1 {
    pub fn evaluate(request: &K2ExactOracleRequestV1) -> K2GoalEnvironmentResultV1<Self> {
        request.validate()?;
        let mut outcome = Self {
            schema: K2_EXACT_ORACLE_OUTCOME_SCHEMA_V1.to_owned(),
            outcome_root_sha256: String::new(),
            request_root_sha256: request.request_root_sha256.clone(),
            expected_terminal_tree_root_sha256: request.expected_terminal_tree_root_sha256.clone(),
            observed_terminal_tree_root_sha256: request.observed_terminal_tree_root_sha256.clone(),
            goal_satisfied: request.expected_terminal_tree_root_sha256
                == request.observed_terminal_tree_root_sha256,
        };
        outcome.outcome_root_sha256 = outcome.expected_root()?;
        outcome.validate(request)?;
        Ok(outcome)
    }

    pub fn validate(&self, request: &K2ExactOracleRequestV1) -> K2GoalEnvironmentResultV1<()> {
        request.validate()?;
        if self.schema != K2_EXACT_ORACLE_OUTCOME_SCHEMA_V1
            || self.request_root_sha256 != request.request_root_sha256
            || self.expected_terminal_tree_root_sha256 != request.expected_terminal_tree_root_sha256
            || self.observed_terminal_tree_root_sha256 != request.observed_terminal_tree_root_sha256
            || self.goal_satisfied
                != (self.expected_terminal_tree_root_sha256
                    == self.observed_terminal_tree_root_sha256)
            || self.outcome_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_exact_oracle_outcome_invalid",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        canonical_json_bytes(self).map_err(|_| K2GoalEnvironmentErrorV1::Serialization)
    }

    pub fn from_canonical_bytes(
        bytes: &[u8],
        request: &K2ExactOracleRequestV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        let outcome: Self = serde_json::from_slice(bytes).map_err(|_| {
            K2GoalEnvironmentErrorV1::Invalid("k2_exact_oracle_outcome_decode_failed")
        })?;
        outcome.validate(request)?;
        if outcome.canonical_bytes()? != bytes {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_exact_oracle_outcome_not_canonical",
            ));
        }
        Ok(outcome)
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&(
            K2_EXACT_ORACLE_OUTCOME_SCHEMA_V1,
            self.request_root_sha256.as_str(),
            self.expected_terminal_tree_root_sha256.as_str(),
            self.observed_terminal_tree_root_sha256.as_str(),
            self.goal_satisfied,
        ))
    }
}

pub struct K2ExactGoalEvaluationInputV1<'a> {
    pub freeze: &'a K2DecisionFreezeV1,
    pub goal: &'a K2GoalEnvelopeV1,
    pub vocabulary: &'a K2K1VocabularySnapshotV1,
    pub alternatives: &'a K2AlternativeSetV1,
    pub predictions: &'a K2AlternativePredictionSetV1,
    pub selection: &'a K2PreparedSelectionReceiptV1,
    pub binding: &'a K2LawLabBindingV1,
    pub request: &'a LawLabSandboxRequestV1,
    pub execution: &'a LawLabSandboxExecutionV1,
    pub oracle_manifest: &'a K2ExactOracleManifestV1,
    pub oracle_request: &'a K2ExactOracleRequestV1,
    pub oracle_outcome: &'a K2ExactOracleOutcomeV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2ExactGoalReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub provenance: K2EvidenceProvenanceV1,
    pub decision_freeze_root_sha256: String,
    pub law_lab_binding_root_sha256: String,
    pub law_lab_receipt_root_sha256: String,
    pub oracle_manifest_root_sha256: String,
    pub oracle_request_root_sha256: String,
    pub oracle_outcome_root_sha256: String,
    pub expected_terminal_tree_root_sha256: String,
    pub observed_terminal_tree_root_sha256: String,
    pub goal_satisfied: bool,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2ExactGoalReceiptV1 {
    pub fn evaluate(input: K2ExactGoalEvaluationInputV1<'_>) -> K2GoalEnvironmentResultV1<Self> {
        input.binding.validate(K2LawLabBindingInputV1 {
            freeze: input.freeze,
            goal: input.goal,
            vocabulary: input.vocabulary,
            alternatives: input.alternatives,
            predictions: input.predictions,
            selection: input.selection,
            request: input.request,
        })?;
        input
            .execution
            .receipt
            .validate(input.request, &input.execution.worker_outcome)
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        input.oracle_manifest.validate()?;
        input.oracle_request.validate()?;
        input.oracle_outcome.validate(input.oracle_request)?;
        if input.oracle_manifest.manifest_root_sha256 != input.freeze.oracle_manifest_root_sha256
            || input.oracle_manifest.executable_sha256 != input.freeze.oracle_executable_sha256
            || input.execution.receipt.request_root_sha256
                != input.binding.law_lab_request_root_sha256
            || input.oracle_request.oracle_manifest_root_sha256
                != input.oracle_manifest.manifest_root_sha256
            || input.oracle_request.goal_predicate_root_sha256
                != input.goal.goal_predicate_root_sha256
            || input.oracle_request.law_lab_binding_root_sha256 != input.binding.binding_root_sha256
            || input.oracle_request.law_lab_receipt_root_sha256
                != input.execution.receipt.receipt_root_sha256
            || input.oracle_request.expected_terminal_tree_root_sha256
                != input.goal.expected_terminal_tree_root_sha256
            || input.oracle_request.observed_terminal_tree_root_sha256
                != input.execution.receipt.post_tree_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_exact_oracle_binding_invalid",
            ));
        }
        let observed_terminal_tree_root_sha256 = input
            .oracle_outcome
            .observed_terminal_tree_root_sha256
            .clone();
        let goal_satisfied = input.oracle_outcome.goal_satisfied;
        let mut receipt = Self {
            schema: K2_EXACT_GOAL_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            provenance: input.freeze.provenance,
            decision_freeze_root_sha256: input.freeze.decision_freeze_root_sha256.clone(),
            law_lab_binding_root_sha256: input.binding.binding_root_sha256.clone(),
            law_lab_receipt_root_sha256: input.execution.receipt.receipt_root_sha256.clone(),
            oracle_manifest_root_sha256: input.oracle_manifest.manifest_root_sha256.clone(),
            oracle_request_root_sha256: input.oracle_request.request_root_sha256.clone(),
            oracle_outcome_root_sha256: input.oracle_outcome.outcome_root_sha256.clone(),
            expected_terminal_tree_root_sha256: input
                .goal
                .expected_terminal_tree_root_sha256
                .clone(),
            observed_terminal_tree_root_sha256,
            goal_satisfied,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate(input)?;
        Ok(receipt)
    }

    pub fn validate(
        &self,
        input: K2ExactGoalEvaluationInputV1<'_>,
    ) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        input.oracle_outcome.validate(input.oracle_request)?;
        let expected_satisfaction =
            self.expected_terminal_tree_root_sha256 == self.observed_terminal_tree_root_sha256;
        if self.schema != K2_EXACT_GOAL_RECEIPT_SCHEMA_V1
            || self.provenance != K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest
            || self.provenance != input.freeze.provenance
            || self.decision_freeze_root_sha256 != input.freeze.decision_freeze_root_sha256
            || self.law_lab_binding_root_sha256 != input.binding.binding_root_sha256
            || self.law_lab_receipt_root_sha256 != input.execution.receipt.receipt_root_sha256
            || self.oracle_manifest_root_sha256 != input.oracle_manifest.manifest_root_sha256
            || self.oracle_request_root_sha256 != input.oracle_request.request_root_sha256
            || self.oracle_outcome_root_sha256 != input.oracle_outcome.outcome_root_sha256
            || self.expected_terminal_tree_root_sha256
                != input.goal.expected_terminal_tree_root_sha256
            || self.observed_terminal_tree_root_sha256
                != input.execution.receipt.post_tree_root_sha256
            || self.goal_satisfied != expected_satisfaction
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_exact_goal_receipt_invalid",
            ));
        }
        Ok(())
    }

    pub(crate) fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.receipt_root_sha256.as_str(),
            self.decision_freeze_root_sha256.as_str(),
            self.law_lab_binding_root_sha256.as_str(),
            self.law_lab_receipt_root_sha256.as_str(),
            self.oracle_manifest_root_sha256.as_str(),
            self.oracle_request_root_sha256.as_str(),
            self.oracle_outcome_root_sha256.as_str(),
            self.expected_terminal_tree_root_sha256.as_str(),
            self.observed_terminal_tree_root_sha256.as_str(),
        ] {
            require_root(root, "k2_persisted_exact_goal_root_invalid")?;
        }
        if self.schema != K2_EXACT_GOAL_RECEIPT_SCHEMA_V1
            || self.provenance != K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest
            || self.goal_satisfied
                != (self.expected_terminal_tree_root_sha256
                    == self.observed_terminal_tree_root_sha256)
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_persisted_exact_goal_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&(
            K2_EXACT_GOAL_RECEIPT_SCHEMA_V1,
            self.provenance,
            self.decision_freeze_root_sha256.as_str(),
            self.law_lab_binding_root_sha256.as_str(),
            self.law_lab_receipt_root_sha256.as_str(),
            self.oracle_manifest_root_sha256.as_str(),
            self.oracle_request_root_sha256.as_str(),
            self.oracle_outcome_root_sha256.as_str(),
            self.expected_terminal_tree_root_sha256.as_str(),
            self.observed_terminal_tree_root_sha256.as_str(),
            self.goal_satisfied,
            &self.authority,
        ))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2DecisionTerminalVerdictV1 {
    CapabilityPass,
    LabGoalSatisfied,
    LabGoalNotSatisfied,
    InsufficientK1Vocabulary,
    CertificateBoundRuntimeClosed,
    StaleBeforeFreeze,
    NoMeaningfulAlternatives,
    NoUniqueSelection,
    SandboxVerificationFail,
    OracleMismatch,
    BudgetExhausted,
    SafetyVeto,
    IndeterminateAfterCrash,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2DecisionOutcomeReceiptV1 {
    pub schema: String,
    pub outcome_root_sha256: String,
    pub provenance: K2EvidenceProvenanceV1,
    pub decision_freeze_root_sha256: String,
    pub prediction_set_root_sha256: String,
    pub law_lab_binding_root_sha256: String,
    pub sandbox_receipt_root_sha256: String,
    pub exact_goal_receipt_root_sha256: String,
    pub terminal_verdict: K2DecisionTerminalVerdictV1,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2DecisionOutcomeReceiptV1 {
    pub fn capability_pass(
        freeze: &K2DecisionFreezeV1,
        predictions: &K2AlternativePredictionSetV1,
        binding: &K2LawLabBindingV1,
        execution: &LawLabSandboxExecutionV1,
        exact_goal: &K2ExactGoalReceiptV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        if freeze.provenance != K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest
            || predictions.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
            || binding.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
            || exact_goal.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
            || exact_goal.law_lab_binding_root_sha256 != binding.binding_root_sha256
            || exact_goal.law_lab_receipt_root_sha256 != execution.receipt.receipt_root_sha256
            || !exact_goal.goal_satisfied
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_capability_outcome_inputs_invalid",
            ));
        }
        let mut outcome = Self {
            schema: K2_DECISION_OUTCOME_SCHEMA_V1.to_owned(),
            outcome_root_sha256: String::new(),
            provenance: freeze.provenance,
            decision_freeze_root_sha256: freeze.decision_freeze_root_sha256.clone(),
            prediction_set_root_sha256: predictions.prediction_set_root_sha256.clone(),
            law_lab_binding_root_sha256: binding.binding_root_sha256.clone(),
            sandbox_receipt_root_sha256: execution.receipt.receipt_root_sha256.clone(),
            exact_goal_receipt_root_sha256: exact_goal.receipt_root_sha256.clone(),
            terminal_verdict: K2DecisionTerminalVerdictV1::CapabilityPass,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        outcome.outcome_root_sha256 = outcome.expected_root()?;
        outcome.validate()?;
        Ok(outcome)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.outcome_root_sha256.as_str(),
            self.decision_freeze_root_sha256.as_str(),
            self.prediction_set_root_sha256.as_str(),
            self.law_lab_binding_root_sha256.as_str(),
            self.sandbox_receipt_root_sha256.as_str(),
            self.exact_goal_receipt_root_sha256.as_str(),
        ] {
            require_root(root, "k2_outcome_root_invalid")?;
        }
        if self.schema != K2_DECISION_OUTCOME_SCHEMA_V1
            || self.provenance != K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest
            || self.terminal_verdict != K2DecisionTerminalVerdictV1::CapabilityPass
            || self.outcome_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_decision_outcome_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&(
            K2_DECISION_OUTCOME_SCHEMA_V1,
            self.provenance,
            self.decision_freeze_root_sha256.as_str(),
            self.prediction_set_root_sha256.as_str(),
            self.law_lab_binding_root_sha256.as_str(),
            self.sandbox_receipt_root_sha256.as_str(),
            self.exact_goal_receipt_root_sha256.as_str(),
            self.terminal_verdict,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2DecisionEpisodeSealV1 {
    pub schema: String,
    pub seal_root_sha256: String,
    pub episode_id_sha256: String,
    pub outcome_root_sha256: String,
    pub terminal_event_root_sha256: String,
    pub final_projection_root_sha256: String,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2DecisionEpisodeSealV1 {
    pub fn derive(
        episode_id_sha256: String,
        outcome_root_sha256: String,
        terminal_event_root_sha256: String,
        final_projection_root_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        for root in [
            episode_id_sha256.as_str(),
            outcome_root_sha256.as_str(),
            terminal_event_root_sha256.as_str(),
            final_projection_root_sha256.as_str(),
        ] {
            require_root(root, "k2_episode_seal_input_root_invalid")?;
        }
        let mut seal = Self {
            schema: K2_EPISODE_SEAL_SCHEMA_V1.to_owned(),
            seal_root_sha256: String::new(),
            episode_id_sha256,
            outcome_root_sha256,
            terminal_event_root_sha256,
            final_projection_root_sha256,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        seal.seal_root_sha256 = seal.expected_root()?;
        seal.validate()?;
        Ok(seal)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.seal_root_sha256.as_str(),
            self.episode_id_sha256.as_str(),
            self.outcome_root_sha256.as_str(),
            self.terminal_event_root_sha256.as_str(),
            self.final_projection_root_sha256.as_str(),
        ] {
            require_root(root, "k2_episode_seal_root_invalid")?;
        }
        if self.schema != K2_EPISODE_SEAL_SCHEMA_V1
            || self.seal_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid("k2_episode_seal_invalid"));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&(
            K2_EPISODE_SEAL_SCHEMA_V1,
            self.episode_id_sha256.as_str(),
            self.outcome_root_sha256.as_str(),
            self.terminal_event_root_sha256.as_str(),
            self.final_projection_root_sha256.as_str(),
            &self.authority,
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum K2CertificateBoundRuntimeStatusV1 {
    InsufficientK1Vocabulary,
    CertificateBoundRuntimeClosed,
}

#[must_use]
pub const fn k2_certificate_bound_runtime_status_v1(
    genuine_k1_action_count: usize,
) -> K2CertificateBoundRuntimeStatusV1 {
    if genuine_k1_action_count < 2 {
        K2CertificateBoundRuntimeStatusV1::InsufficientK1Vocabulary
    } else {
        K2CertificateBoundRuntimeStatusV1::CertificateBoundRuntimeClosed
    }
}
