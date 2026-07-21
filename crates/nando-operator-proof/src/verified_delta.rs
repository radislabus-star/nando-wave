use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const TYPED_EXECUTION_STAGE_RECEIPT_SCHEMA_V1: &str = "nando.typed-execution-stage-receipt.v1";
pub const VERIFIED_DELTA_RECEIPT_SCHEMA_V1: &str = "nando.verified-delta-receipt.v1";
pub const VERIFIED_DELTA_MAX_RELATIONS: usize = 256;
const PHASE_MICRO_SCALE: f64 = 1_000_000.0;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TypedExecutionStage {
    RoleBinding,
    RelationEvaluation,
    Transform,
    Composition,
    Renderer,
    IndependentVerifier,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TypedExecutionStageReceipt {
    pub schema: String,
    pub stage: TypedExecutionStage,
    pub generation: u64,
    pub operator_fingerprint64: u64,
    pub surface_id_sha256: String,
    pub session_id_sha256: String,
    pub input_relation_sha256: String,
    pub predicted_relation_sha256: String,
    pub observed_relation_sha256: String,
    pub stage_payload_sha256: String,
    pub independently_observed: bool,
    pub accepted: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifiedDeltaOutcome {
    Positive,
    ApplicabilityNegative,
    HardContradiction,
    CensoredUnknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifiedDeltaRelationState {
    Opposed,
    Unresolved,
    Supported,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifiedDeltaRelation {
    pub plane: u8,
    pub source_role: u8,
    pub target_role: u8,
    pub state: VerifiedDeltaRelationState,
    pub phase_re_micro: i32,
    pub phase_im_micro: i32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifiedDeltaReceipt {
    schema: String,
    receipt_sha256: String,
    generation: u64,
    operator_fingerprint64: u64,
    surface_id_sha256: String,
    session_id_sha256: String,
    input_relation_sha256: String,
    predicted_relation_sha256: String,
    observed_relation_sha256: String,
    outcome: VerifiedDeltaOutcome,
    relations: Vec<VerifiedDeltaRelation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifiedDeltaError {
    WrongReceiptCount,
    InvalidSchema,
    InvalidDigest,
    DuplicateStage,
    MissingStage,
    MixedTraceIdentity,
    VerifierNotIndependent,
    OutcomeVerifierMismatch,
    PositiveResidualMismatch,
    EmptySemanticResidual,
    RelationCapacityExceeded,
    InvalidPhase,
}

impl VerifiedDeltaReceipt {
    pub fn from_typed_trace(
        receipts: Vec<TypedExecutionStageReceipt>,
        outcome: VerifiedDeltaOutcome,
        relations: Vec<VerifiedDeltaRelation>,
    ) -> Result<Self, VerifiedDeltaError> {
        if receipts.len() != TypedExecutionStage::ALL.len() {
            return Err(VerifiedDeltaError::WrongReceiptCount);
        }
        if relations.len() > VERIFIED_DELTA_MAX_RELATIONS {
            return Err(VerifiedDeltaError::RelationCapacityExceeded);
        }
        if outcome != VerifiedDeltaOutcome::CensoredUnknown && relations.is_empty() {
            return Err(VerifiedDeltaError::EmptySemanticResidual);
        }
        if relations.iter().any(|relation| {
            relation.phase_re_micro.unsigned_abs() > 1_000_000
                || relation.phase_im_micro.unsigned_abs() > 1_000_000
        }) {
            return Err(VerifiedDeltaError::InvalidPhase);
        }

        let mut by_stage = BTreeMap::new();
        for receipt in receipts {
            if receipt.schema != TYPED_EXECUTION_STAGE_RECEIPT_SCHEMA_V1 {
                return Err(VerifiedDeltaError::InvalidSchema);
            }
            if !all_digests_valid(&receipt) {
                return Err(VerifiedDeltaError::InvalidDigest);
            }
            if by_stage.insert(receipt.stage, receipt).is_some() {
                return Err(VerifiedDeltaError::DuplicateStage);
            }
        }
        if !TypedExecutionStage::ALL
            .iter()
            .all(|stage| by_stage.contains_key(stage))
        {
            return Err(VerifiedDeltaError::MissingStage);
        }

        let first = by_stage
            .values()
            .next()
            .ok_or(VerifiedDeltaError::MissingStage)?;
        let identity = TraceIdentity::from(first);
        if by_stage
            .values()
            .any(|receipt| TraceIdentity::from(receipt) != identity)
        {
            return Err(VerifiedDeltaError::MixedTraceIdentity);
        }
        let verifier = by_stage
            .get(&TypedExecutionStage::IndependentVerifier)
            .ok_or(VerifiedDeltaError::MissingStage)?;
        if !verifier.independently_observed {
            return Err(VerifiedDeltaError::VerifierNotIndependent);
        }
        match outcome {
            VerifiedDeltaOutcome::Positive | VerifiedDeltaOutcome::ApplicabilityNegative
                if !verifier.accepted =>
            {
                return Err(VerifiedDeltaError::OutcomeVerifierMismatch);
            }
            VerifiedDeltaOutcome::HardContradiction if verifier.accepted => {
                return Err(VerifiedDeltaError::OutcomeVerifierMismatch);
            }
            VerifiedDeltaOutcome::Positive
                if identity.predicted_relation_sha256 != identity.observed_relation_sha256 =>
            {
                return Err(VerifiedDeltaError::PositiveResidualMismatch);
            }
            _ => {}
        }

        let receipt_sha256 = receipt_digest(&identity, outcome, &relations);
        Ok(Self {
            schema: VERIFIED_DELTA_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_sha256,
            generation: identity.generation,
            operator_fingerprint64: identity.operator_fingerprint64,
            surface_id_sha256: identity.surface_id_sha256,
            session_id_sha256: identity.session_id_sha256,
            input_relation_sha256: identity.input_relation_sha256,
            predicted_relation_sha256: identity.predicted_relation_sha256,
            observed_relation_sha256: identity.observed_relation_sha256,
            outcome,
            relations,
        })
    }

    #[must_use]
    pub fn schema(&self) -> &str {
        &self.schema
    }

    #[must_use]
    pub fn receipt_sha256(&self) -> &str {
        &self.receipt_sha256
    }

    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    #[must_use]
    pub const fn operator_fingerprint64(&self) -> u64 {
        self.operator_fingerprint64
    }

    #[must_use]
    pub fn surface_id_sha256(&self) -> &str {
        &self.surface_id_sha256
    }

    #[must_use]
    pub fn session_id_sha256(&self) -> &str {
        &self.session_id_sha256
    }

    #[must_use]
    pub fn input_relation_sha256(&self) -> &str {
        &self.input_relation_sha256
    }

    #[must_use]
    pub const fn outcome(&self) -> VerifiedDeltaOutcome {
        self.outcome
    }

    #[must_use]
    pub fn relations(&self) -> &[VerifiedDeltaRelation] {
        &self.relations
    }

    #[must_use]
    pub fn phase_component(value_micro: i32) -> f64 {
        f64::from(value_micro) / PHASE_MICRO_SCALE
    }
}

impl TypedExecutionStage {
    pub const ALL: [Self; 6] = [
        Self::RoleBinding,
        Self::RelationEvaluation,
        Self::Transform,
        Self::Composition,
        Self::Renderer,
        Self::IndependentVerifier,
    ];
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct TraceIdentity {
    generation: u64,
    operator_fingerprint64: u64,
    surface_id_sha256: String,
    session_id_sha256: String,
    input_relation_sha256: String,
    predicted_relation_sha256: String,
    observed_relation_sha256: String,
}

impl From<&TypedExecutionStageReceipt> for TraceIdentity {
    fn from(receipt: &TypedExecutionStageReceipt) -> Self {
        Self {
            generation: receipt.generation,
            operator_fingerprint64: receipt.operator_fingerprint64,
            surface_id_sha256: receipt.surface_id_sha256.clone(),
            session_id_sha256: receipt.session_id_sha256.clone(),
            input_relation_sha256: receipt.input_relation_sha256.clone(),
            predicted_relation_sha256: receipt.predicted_relation_sha256.clone(),
            observed_relation_sha256: receipt.observed_relation_sha256.clone(),
        }
    }
}

fn all_digests_valid(receipt: &TypedExecutionStageReceipt) -> bool {
    [
        receipt.surface_id_sha256.as_str(),
        receipt.session_id_sha256.as_str(),
        receipt.input_relation_sha256.as_str(),
        receipt.predicted_relation_sha256.as_str(),
        receipt.observed_relation_sha256.as_str(),
        receipt.stage_payload_sha256.as_str(),
    ]
    .iter()
    .all(|digest| valid_sha256(digest))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn receipt_digest(
    identity: &TraceIdentity,
    outcome: VerifiedDeltaOutcome,
    relations: &[VerifiedDeltaRelation],
) -> String {
    let bytes = serde_json::to_vec(&(
        VERIFIED_DELTA_RECEIPT_SCHEMA_V1,
        identity,
        outcome,
        relations,
    ))
    .unwrap_or_default();
    format!("{:x}", Sha256::digest(bytes))
}
