use serde::{Deserialize, Serialize};

use crate::{
    CrystallizedCollectionAdmissionCandidateV1, LiveScalarAdmissionCandidate,
    OnlineCollectionAdmissionCandidate, OnlineResponseAdmissionCandidate,
};

pub use nando_operator_admission::{
    DURABLE_RUNTIME_PARITY_RECEIPT_SCHEMA_V1, DurableRuntimeParityReceipt,
};
pub use nando_operator_learning::RuntimeParityCase;

pub const ONLINE_ADMISSION_CANDIDATE_BUNDLE_SCHEMA_V1: &str =
    "nando.online-admission-candidate-bundle.v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OnlineAdmissionCandidateBundle {
    pub schema: String,
    pub project_id: String,
    pub revision: u64,
    pub relation_candidates: Vec<OnlineResponseAdmissionCandidate>,
    pub collection_candidates: Vec<OnlineCollectionAdmissionCandidate>,
    #[serde(default)]
    pub crystallized_candidates: Vec<LiveScalarAdmissionCandidate>,
    #[serde(default)]
    pub crystallized_collection_candidates: Vec<CrystallizedCollectionAdmissionCandidateV1>,
}

impl OnlineAdmissionCandidateBundle {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != ONLINE_ADMISSION_CANDIDATE_BUNDLE_SCHEMA_V1 {
            return Err("online_admission_candidate_bundle_schema_invalid");
        }
        if self.project_id.is_empty() || self.project_id.len() > 128 || self.revision == 0 {
            return Err("online_admission_candidate_bundle_identity_invalid");
        }
        if self.relation_candidates.len() > 256
            || self.collection_candidates.len() > 256
            || self.crystallized_candidates.len() > 64
            || self.crystallized_collection_candidates.len() > 64
        {
            return Err("online_admission_candidate_bundle_capacity_exceeded");
        }
        for candidate in &self.crystallized_collection_candidates {
            candidate.validate()?;
        }
        Ok(())
    }
}
