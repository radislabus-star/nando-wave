use nando_operator_kernel::{
    ResponseProgram, canonical_json_sha256, response_program_version_root_sha256,
    valid_nonzero_sha256,
};
use serde::{Deserialize, Serialize};

pub const K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1: &str =
    "nando.k1-durable-independent-future-prediction.v1";
const K1_FUTURE_PREDICTION_CONTRACT_SCHEMA_V1: &str = "nando.k1-future-prediction-contract.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1FuturePredictionContractV1 {
    pub schema: String,
    pub contract_root_sha256: String,
    pub candidate_freeze_root_sha256: String,
    pub identification_freeze_root_sha256: String,
    pub semantic_class_root_sha256: String,
    pub protocol_mode_root_sha256: String,
    pub canonical_program_root_sha256: String,
    pub canonical_program: ResponseProgram,
    pub sealed_at_unix_nanos: u64,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

impl K1FuturePredictionContractV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        candidate_freeze_root_sha256: String,
        identification_freeze_root_sha256: String,
        semantic_class_root_sha256: String,
        protocol_mode_root_sha256: String,
        canonical_program: ResponseProgram,
        sealed_at_unix_nanos: u64,
    ) -> Result<Self, &'static str> {
        canonical_program.validate()?;
        let canonical_program_root_sha256 =
            response_program_version_root_sha256(&canonical_program)?;
        let mut contract = Self {
            schema: K1_FUTURE_PREDICTION_CONTRACT_SCHEMA_V1.to_owned(),
            contract_root_sha256: String::new(),
            candidate_freeze_root_sha256,
            identification_freeze_root_sha256,
            semantic_class_root_sha256,
            protocol_mode_root_sha256,
            canonical_program_root_sha256,
            canonical_program,
            sealed_at_unix_nanos,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        contract.contract_root_sha256 = contract.expected_root()?;
        contract.validate()?;
        Ok(contract)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        self.canonical_program.validate()?;
        if self.schema != K1_FUTURE_PREDICTION_CONTRACT_SCHEMA_V1
            || ![
                self.contract_root_sha256.as_str(),
                self.candidate_freeze_root_sha256.as_str(),
                self.identification_freeze_root_sha256.as_str(),
                self.semantic_class_root_sha256.as_str(),
                self.protocol_mode_root_sha256.as_str(),
                self.canonical_program_root_sha256.as_str(),
            ]
            .into_iter()
            .all(valid_nonzero_sha256)
            || self.canonical_program_root_sha256
                != response_program_version_root_sha256(&self.canonical_program)?
            || self.sealed_at_unix_nanos == 0
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.contract_root_sha256 != self.expected_root()?
        {
            return Err("k1_future_prediction_contract_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            K1_FUTURE_PREDICTION_CONTRACT_SCHEMA_V1,
            self.candidate_freeze_root_sha256.as_str(),
            self.identification_freeze_root_sha256.as_str(),
            self.semantic_class_root_sha256.as_str(),
            self.protocol_mode_root_sha256.as_str(),
            self.canonical_program_root_sha256.as_str(),
            self.sealed_at_unix_nanos,
            false,
            false,
        ))
    }
}
