use std::collections::BTreeMap;

use nando_operator_kernel::{ResponseProgram, VerifierProgram};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CollectionSynthesisExample {
    pub provider_payload: Value,
    pub expected_response: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SynthesizedCollectionProgram {
    pub program: ResponseProgram,
    pub verifier: VerifierProgram,
    pub exact_checks: usize,
    pub candidates_enumerated: usize,
    pub description_length_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionVersionSpace {
    pub programs: Vec<ResponseProgram>,
    pub exact_checks: usize,
    pub candidates_enumerated: usize,
    pub policy_rejected_exact_matches: usize,
    pub policy_rejection_reasons: BTreeMap<String, usize>,
    pub canonical_rejection_reasons: BTreeMap<String, usize>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResponseCoverageDiagnostic {
    pub response_bytes: usize,
    pub dynamic_bytes: usize,
    pub request_dynamic_bytes: usize,
    pub tool_dynamic_bytes: usize,
    pub matching_selectors: usize,
    pub exact_surface_required: bool,
}
