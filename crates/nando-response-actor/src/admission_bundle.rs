use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{OnlineCollectionAdmissionCandidate, OnlineResponseAdmissionCandidate};

pub const ONLINE_ADMISSION_CANDIDATE_BUNDLE_SCHEMA_V1: &str =
    "nando.online-admission-candidate-bundle.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeParityCase {
    pub evidence_ref_sha256: String,
    #[serde(default)]
    pub request_text: String,
    pub provider_payload: Value,
    pub expected_response: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OnlineAdmissionCandidateBundle {
    pub schema: String,
    pub project_id: String,
    pub revision: u64,
    pub relation_candidates: Vec<OnlineResponseAdmissionCandidate>,
    pub collection_candidates: Vec<OnlineCollectionAdmissionCandidate>,
}

impl OnlineAdmissionCandidateBundle {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != ONLINE_ADMISSION_CANDIDATE_BUNDLE_SCHEMA_V1 {
            return Err("online_admission_candidate_bundle_schema_invalid");
        }
        if self.project_id.is_empty() || self.project_id.len() > 128 || self.revision == 0 {
            return Err("online_admission_candidate_bundle_identity_invalid");
        }
        if self.relation_candidates.len() > 256 || self.collection_candidates.len() > 256 {
            return Err("online_admission_candidate_bundle_capacity_exceeded");
        }
        Ok(())
    }
}
