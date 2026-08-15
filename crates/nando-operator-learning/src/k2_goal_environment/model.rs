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

include!("model/domain.rs");
include!("model/protocol.rs");
include!("model/receipt.rs");
