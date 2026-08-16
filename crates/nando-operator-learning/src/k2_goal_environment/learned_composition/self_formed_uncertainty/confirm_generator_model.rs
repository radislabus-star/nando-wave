use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_sha256_bytes_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_CONFIRM_GENERATOR_REQUEST_SCHEMA_V1,
    K2_UNCERTAINTY_CONFIRM_GENERATOR_RESPONSE_SCHEMA_V1,
    K2_UNCERTAINTY_DEVELOPMENT_SEED_COMMITMENT_V1, K2_UNCERTAINTY_PREREGISTRATION_V2_ROOT_V1,
    K2_UNCERTAINTY_PREREGISTRATION_V3_ROOT_V1, K2UncertaintyPrivateBatchV1,
    K2UncertaintyPublicBatchV1, K2UncertaintySplitV1, denied_authority_v1,
    require_denied_authority_v1, uncertainty_root_v1,
};

pub const K2_UNCERTAINTY_PREREGISTRATION_V4_ROOT_V1: &str =
    "1c40f6bb696257e4add212f7d2d8edadf30fefb20b0ed2507a6c6b0dadb051ce";
pub const K2_UNCERTAINTY_PREREGISTRATION_V5_ROOT_V1: &str =
    "075275b140c00b8e26bfbccebd014136bd39a0d7b23ae83f6d8292bd6d6b776d";
pub const K2_UNCERTAINTY_SUPERSEDED_CONFIRM_SEED_COMMITMENT_V1: &str =
    "ba7173ac286e3ac1e5eff1eb1fd510acc29a4454f25bbd6cddc8236cf1ebb988";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyConfirmGeneratorRequestV1 {
    pub schema: String,
    pub split: K2UncertaintySplitV1,
    pub nonce_bytes: Vec<u8>,
    pub nonce_commitment_sha256: String,
    pub preregistration_v2_root_sha256: String,
    pub preregistration_v3_root_sha256: String,
    pub preregistration_v4_root_sha256: String,
    pub preregistration_v5_root_sha256: String,
    pub successor_freeze_root_sha256: String,
    pub authorization_receipt_root_sha256: String,
    pub generator_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintyConfirmGeneratorRequestV1 {
    pub fn seal(
        nonce_bytes: Vec<u8>,
        successor_freeze_root_sha256: String,
        authorization_receipt_root_sha256: String,
        generator_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut request = Self {
            schema: K2_UNCERTAINTY_CONFIRM_GENERATOR_REQUEST_SCHEMA_V1.to_owned(),
            split: K2UncertaintySplitV1::Confirm,
            nonce_commitment_sha256: composition_sha256_bytes_v1(&nonce_bytes),
            nonce_bytes,
            preregistration_v2_root_sha256: K2_UNCERTAINTY_PREREGISTRATION_V2_ROOT_V1.to_owned(),
            preregistration_v3_root_sha256: K2_UNCERTAINTY_PREREGISTRATION_V3_ROOT_V1.to_owned(),
            preregistration_v4_root_sha256: K2_UNCERTAINTY_PREREGISTRATION_V4_ROOT_V1.to_owned(),
            preregistration_v5_root_sha256: K2_UNCERTAINTY_PREREGISTRATION_V5_ROOT_V1.to_owned(),
            successor_freeze_root_sha256,
            authorization_receipt_root_sha256,
            generator_executable_sha256,
            authority: denied_authority_v1(),
            request_root_sha256: String::new(),
        };
        request.request_root_sha256 = request.expected_root()?;
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.successor_freeze_root_sha256,
            &self.authorization_receipt_root_sha256,
            &self.generator_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        let commitment = composition_sha256_bytes_v1(&self.nonce_bytes);
        if self.schema != K2_UNCERTAINTY_CONFIRM_GENERATOR_REQUEST_SCHEMA_V1
            || self.split != K2UncertaintySplitV1::Confirm
            || self.nonce_bytes.len() != 32
            || self.nonce_commitment_sha256 != commitment
            || self.nonce_commitment_sha256 == K2_UNCERTAINTY_DEVELOPMENT_SEED_COMMITMENT_V1
            || self.nonce_commitment_sha256 == K2_UNCERTAINTY_SUPERSEDED_CONFIRM_SEED_COMMITMENT_V1
            || self.preregistration_v2_root_sha256 != K2_UNCERTAINTY_PREREGISTRATION_V2_ROOT_V1
            || self.preregistration_v3_root_sha256 != K2_UNCERTAINTY_PREREGISTRATION_V3_ROOT_V1
            || self.preregistration_v4_root_sha256 != K2_UNCERTAINTY_PREREGISTRATION_V4_ROOT_V1
            || self.preregistration_v5_root_sha256 != K2_UNCERTAINTY_PREREGISTRATION_V5_ROOT_V1
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_generator_request_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONFIRM_GENERATOR_REQUEST_SCHEMA_V1,
            self.split,
            &self.nonce_bytes,
            &self.nonce_commitment_sha256,
            &self.preregistration_v2_root_sha256,
            &self.preregistration_v3_root_sha256,
            &self.preregistration_v4_root_sha256,
            &self.preregistration_v5_root_sha256,
            &self.successor_freeze_root_sha256,
            &self.authorization_receipt_root_sha256,
            &self.generator_executable_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyConfirmGeneratorResponseV1 {
    pub schema: String,
    pub generator_request_root_sha256: String,
    pub public: K2UncertaintyPublicBatchV1,
    pub private: K2UncertaintyPrivateBatchV1,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub response_root_sha256: String,
}

impl K2UncertaintyConfirmGeneratorResponseV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.generator_request_root_sha256)?;
        self.public.validate()?;
        self.private.validate()?;
        require_denied_authority_v1(&self.authority)?;
        let split = self.public.cases.first().map(|case| case.vocabulary.split);
        if self.schema != K2_UNCERTAINTY_CONFIRM_GENERATOR_RESPONSE_SCHEMA_V1
            || split != Some(K2UncertaintySplitV1::Confirm)
            || self.public.split_commitment_root_sha256
                == K2_UNCERTAINTY_DEVELOPMENT_SEED_COMMITMENT_V1
            || self.public.split_commitment_root_sha256
                == K2_UNCERTAINTY_SUPERSEDED_CONFIRM_SEED_COMMITMENT_V1
            || self.public.experiment_id_sha256 != self.private.experiment_id_sha256
            || self.public.public_batch_root_sha256 != self.private.public_batch_root_sha256
            || self.response_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_generator_response_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.authority = denied_authority_v1();
        self.response_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONFIRM_GENERATOR_RESPONSE_SCHEMA_V1,
            &self.generator_request_root_sha256,
            &self.public,
            &self.private,
            &self.authority,
        ))
    }
}
