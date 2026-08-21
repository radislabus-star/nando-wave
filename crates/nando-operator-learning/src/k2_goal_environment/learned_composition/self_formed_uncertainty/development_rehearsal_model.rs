use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    require_composition_root_v1, valid_composition_path_v1,
};
use super::{
    K2_UNCERTAINTY_CONFIRM_CASES_V1, K2UncertaintyConfirmAttemptEventV1,
    K2UncertaintyConfirmAttemptModeV1, K2UncertaintyConfirmOwnerRequestV1,
    K2UncertaintyConfirmPipeReceiptV1, denied_authority_v1, require_denied_authority_v1,
    uncertainty_root_v1,
};

pub const K2_UNCERTAINTY_DEVELOPMENT_STORED_ARTIFACT_SCHEMA_V1: &str =
    "nando.k2-self-formed-development-rehearsal-stored-artifact.v1";
pub const K2_UNCERTAINTY_DEVELOPMENT_SPLIT_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-development-rehearsal-split-receipt.v1";
pub const K2_UNCERTAINTY_DEVELOPMENT_OWNER_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-development-rehearsal-owner-receipt.v1";
pub const K2_UNCERTAINTY_DEVELOPMENT_RECONSTRUCTION_DOMAIN_V1: &str =
    "nando.k2-self-formed-development-rehearsal-private-reconstruction.v1";
pub const K2_UNCERTAINTY_DEVELOPMENT_SPLIT_PATH_V1: &str = "development-split-receipt.json";
pub const K2_UNCERTAINTY_DEVELOPMENT_OWNER_PATH_V1: &str = "development-owner-receipt.json";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1 {
    PublicBatch,
    PublicDenominator,
    ResolverTable,
    FinalTruth,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyDevelopmentRehearsalStoredArtifactV1 {
    pub schema: String,
    pub mode: K2UncertaintyConfirmAttemptModeV1,
    pub kind: K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1,
    pub case_id_sha256: Option<String>,
    pub private_case_ordinal: Option<u64>,
    pub relative_path: String,
    pub unix_mode: u32,
    pub byte_len: u64,
    pub content_sha256: String,
    pub semantic_root_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub artifact_root_sha256: String,
}

impl K2UncertaintyDevelopmentRehearsalStoredArtifactV1 {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn seal(
        kind: K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1,
        case_id_sha256: Option<String>,
        private_case_ordinal: Option<u64>,
        relative_path: String,
        unix_mode: u32,
        byte_len: u64,
        content_sha256: String,
        semantic_root_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut artifact = Self {
            schema: K2_UNCERTAINTY_DEVELOPMENT_STORED_ARTIFACT_SCHEMA_V1.to_owned(),
            mode: K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal,
            kind,
            case_id_sha256,
            private_case_ordinal,
            relative_path,
            unix_mode,
            byte_len,
            content_sha256,
            semantic_root_sha256,
            authority: denied_authority_v1(),
            artifact_root_sha256: String::new(),
        };
        artifact.artifact_root_sha256 = artifact.expected_root()?;
        artifact.validate()?;
        Ok(artifact)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.content_sha256)?;
        require_composition_root_v1(&self.semantic_root_sha256)?;
        if let Some(case_id) = &self.case_id_sha256 {
            require_composition_root_v1(case_id)?;
        }
        require_denied_authority_v1(&self.authority)?;
        let public = matches!(
            self.kind,
            K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::PublicBatch
                | K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::PublicDenominator
        );
        let identity_valid = if public {
            self.case_id_sha256.is_none()
                && self.private_case_ordinal.is_none()
                && self.unix_mode == 0o600
                && self.relative_path
                    == match self.kind {
                        K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::PublicBatch => {
                            "public/public-batch.json"
                        }
                        _ => "public/denominator-receipt.json",
                    }
        } else {
            let case_id = self.case_id_sha256.as_deref().unwrap_or_default();
            self.private_case_ordinal
                .is_some_and(|ordinal| ordinal < K2_UNCERTAINTY_CONFIRM_CASES_V1 as u64)
                && self.unix_mode == 0o400
                && self.relative_path
                    == match self.kind {
                        K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::ResolverTable => {
                            format!("private/resolver/{case_id}.json")
                        }
                        _ => format!("private/final-truth/{case_id}.json"),
                    }
        };
        if self.schema != K2_UNCERTAINTY_DEVELOPMENT_STORED_ARTIFACT_SCHEMA_V1
            || self.mode != K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal
            || !identity_valid
            || !valid_composition_path_v1(&self.relative_path)
            || self.byte_len == 0
            || self.artifact_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_development_stored_artifact_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_DEVELOPMENT_STORED_ARTIFACT_SCHEMA_V1,
            self.mode,
            self.kind,
            &self.case_id_sha256,
            &self.private_case_ordinal,
            &self.relative_path,
            self.unix_mode,
            self.byte_len,
            &self.content_sha256,
            &self.semantic_root_sha256,
            &self.authority,
        ))
    }
}

impl Ord for K2UncertaintyDevelopmentRehearsalStoredArtifactV1 {
    fn cmp(&self, other: &Self) -> Ordering {
        (
            self.kind,
            &self.case_id_sha256,
            &self.private_case_ordinal,
            &self.relative_path,
            &self.artifact_root_sha256,
        )
            .cmp(&(
                other.kind,
                &other.case_id_sha256,
                &other.private_case_ordinal,
                &other.relative_path,
                &other.artifact_root_sha256,
            ))
    }
}

impl PartialOrd for K2UncertaintyDevelopmentRehearsalStoredArtifactV1 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyDevelopmentRehearsalSplitReceiptV1 {
    pub schema: String,
    pub mode: K2UncertaintyConfirmAttemptModeV1,
    pub attempt_root_sha256: String,
    pub owner_request_root_sha256: String,
    pub owner_executable_sha256: String,
    pub generator_executable_sha256: String,
    pub generator_request_root_sha256: String,
    pub generator_response_root_sha256: String,
    pub pipe_receipt: K2UncertaintyConfirmPipeReceiptV1,
    pub pipe_receipt_root_sha256: String,
    pub experiment_id_sha256: String,
    pub development_seed_commitment_sha256: String,
    pub public_batch_root_sha256: String,
    pub private_batch_root_sha256: String,
    pub public_denominator_root_sha256: String,
    pub artifacts: Vec<K2UncertaintyDevelopmentRehearsalStoredArtifactV1>,
    pub private_reconstruction_root_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub split_receipt_root_sha256: String,
}

impl K2UncertaintyDevelopmentRehearsalSplitReceiptV1 {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn seal(
        request: &K2UncertaintyConfirmOwnerRequestV1,
        owner_executable_sha256: String,
        generator_request_root_sha256: String,
        generator_response_root_sha256: String,
        pipe_receipt: K2UncertaintyConfirmPipeReceiptV1,
        public_batch_root_sha256: String,
        private_batch_root_sha256: String,
        public_denominator_root_sha256: String,
        mut artifacts: Vec<K2UncertaintyDevelopmentRehearsalStoredArtifactV1>,
        private_reconstruction_root_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let generator_request =
            request
                .development_generator_request
                .as_ref()
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_development_generator_request_missing",
                ))?;
        artifacts.sort();
        let mut receipt = Self {
            schema: K2_UNCERTAINTY_DEVELOPMENT_SPLIT_RECEIPT_SCHEMA_V1.to_owned(),
            mode: K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal,
            attempt_root_sha256: request.descriptor.attempt_root_sha256.clone(),
            owner_request_root_sha256: request.request_root_sha256.clone(),
            owner_executable_sha256,
            generator_executable_sha256: request.descriptor.generator_executable_sha256.clone(),
            generator_request_root_sha256,
            generator_response_root_sha256,
            pipe_receipt_root_sha256: pipe_receipt.receipt_root_sha256.clone(),
            pipe_receipt,
            experiment_id_sha256: request.descriptor.experiment_id_sha256.clone(),
            development_seed_commitment_sha256: generator_request.seed_commitment_sha256.clone(),
            public_batch_root_sha256,
            private_batch_root_sha256,
            public_denominator_root_sha256,
            artifacts,
            private_reconstruction_root_sha256,
            authority: denied_authority_v1(),
            split_receipt_root_sha256: String::new(),
        };
        receipt.split_receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.attempt_root_sha256,
            &self.owner_request_root_sha256,
            &self.owner_executable_sha256,
            &self.generator_executable_sha256,
            &self.generator_request_root_sha256,
            &self.generator_response_root_sha256,
            &self.pipe_receipt_root_sha256,
            &self.experiment_id_sha256,
            &self.development_seed_commitment_sha256,
            &self.public_batch_root_sha256,
            &self.private_batch_root_sha256,
            &self.public_denominator_root_sha256,
            &self.private_reconstruction_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.pipe_receipt.validate()?;
        for artifact in &self.artifacts {
            artifact.validate()?;
        }
        require_denied_authority_v1(&self.authority)?;
        let public_batches = self
            .artifacts
            .iter()
            .filter(|value| {
                value.kind == K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::PublicBatch
            })
            .count();
        let denominators = self
            .artifacts
            .iter()
            .filter(|value| {
                value.kind
                    == K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::PublicDenominator
            })
            .count();
        let resolver_ordinals = private_ordinals_v1(
            &self.artifacts,
            K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::ResolverTable,
        );
        let truth_ordinals = private_ordinals_v1(
            &self.artifacts,
            K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::FinalTruth,
        );
        let expected_ordinals = (0..K2_UNCERTAINTY_CONFIRM_CASES_V1 as u64).collect::<Vec<_>>();
        if self.schema != K2_UNCERTAINTY_DEVELOPMENT_SPLIT_RECEIPT_SCHEMA_V1
            || self.mode != K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal
            || self.pipe_receipt_root_sha256 != self.pipe_receipt.receipt_root_sha256
            || self.pipe_receipt.generator_executable_sha256 != self.generator_executable_sha256
            || self.pipe_receipt.generator_request_root_sha256 != self.generator_request_root_sha256
            || self.artifacts.len() != 34
            || !self.artifacts.windows(2).all(|pair| pair[0] < pair[1])
            || public_batches != 1
            || denominators != 1
            || resolver_ordinals != expected_ordinals
            || truth_ordinals != expected_ordinals
            || self.split_receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_development_split_receipt_invalid",
            ));
        }
        Ok(())
    }

    pub(super) fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_DEVELOPMENT_SPLIT_RECEIPT_SCHEMA_V1,
            (
                self.mode,
                &self.attempt_root_sha256,
                &self.owner_request_root_sha256,
                &self.owner_executable_sha256,
                &self.generator_executable_sha256,
                &self.generator_request_root_sha256,
                &self.generator_response_root_sha256,
                &self.pipe_receipt,
            ),
            (
                &self.pipe_receipt_root_sha256,
                &self.experiment_id_sha256,
                &self.development_seed_commitment_sha256,
                &self.public_batch_root_sha256,
                &self.private_batch_root_sha256,
                &self.public_denominator_root_sha256,
                &self.artifacts,
                &self.private_reconstruction_root_sha256,
            ),
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyDevelopmentRehearsalOwnerReceiptV1 {
    pub schema: String,
    pub mode: K2UncertaintyConfirmAttemptModeV1,
    pub owner_request_root_sha256: String,
    pub attempt_root_sha256: String,
    pub owner_executable_sha256: String,
    pub generator_executable_sha256: String,
    pub generator_request_root_sha256: String,
    pub generator_response_root_sha256: String,
    pub public_batch_root_sha256: String,
    pub private_batch_root_sha256: String,
    pub split_receipt_root_sha256: String,
    pub pipe_receipt_root_sha256: String,
    pub cases_generated_event_root_sha256: String,
    pub generator_dispatch_count: u64,
    pub nonce_commitment_sha256: Option<String>,
    pub authorization_receipt_root_sha256: Option<String>,
    pub authorization_slot_claim_root_sha256: Option<String>,
    pub sealed_attempts: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyDevelopmentRehearsalOwnerReceiptV1 {
    pub(crate) fn seal(
        request: &K2UncertaintyConfirmOwnerRequestV1,
        split: &K2UncertaintyDevelopmentRehearsalSplitReceiptV1,
        cases_generated_event: &K2UncertaintyConfirmAttemptEventV1,
        generator_dispatch_count: u64,
    ) -> K2CompositionResultV1<Self> {
        let mut receipt = Self {
            schema: K2_UNCERTAINTY_DEVELOPMENT_OWNER_RECEIPT_SCHEMA_V1.to_owned(),
            mode: K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal,
            owner_request_root_sha256: request.request_root_sha256.clone(),
            attempt_root_sha256: request.descriptor.attempt_root_sha256.clone(),
            owner_executable_sha256: split.owner_executable_sha256.clone(),
            generator_executable_sha256: split.generator_executable_sha256.clone(),
            generator_request_root_sha256: split.generator_request_root_sha256.clone(),
            generator_response_root_sha256: split.generator_response_root_sha256.clone(),
            public_batch_root_sha256: split.public_batch_root_sha256.clone(),
            private_batch_root_sha256: split.private_batch_root_sha256.clone(),
            split_receipt_root_sha256: split.split_receipt_root_sha256.clone(),
            pipe_receipt_root_sha256: split.pipe_receipt_root_sha256.clone(),
            cases_generated_event_root_sha256: cases_generated_event.event_root_sha256.clone(),
            generator_dispatch_count,
            nonce_commitment_sha256: None,
            authorization_receipt_root_sha256: None,
            authorization_slot_claim_root_sha256: None,
            sealed_attempts: 0,
            authority: denied_authority_v1(),
            receipt_root_sha256: String::new(),
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.owner_request_root_sha256,
            &self.attempt_root_sha256,
            &self.owner_executable_sha256,
            &self.generator_executable_sha256,
            &self.generator_request_root_sha256,
            &self.generator_response_root_sha256,
            &self.public_batch_root_sha256,
            &self.private_batch_root_sha256,
            &self.split_receipt_root_sha256,
            &self.pipe_receipt_root_sha256,
            &self.cases_generated_event_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_DEVELOPMENT_OWNER_RECEIPT_SCHEMA_V1
            || self.mode != K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal
            || self.generator_dispatch_count != 1
            || self.nonce_commitment_sha256.is_some()
            || self.authorization_receipt_root_sha256.is_some()
            || self.authorization_slot_claim_root_sha256.is_some()
            || self.sealed_attempts != 0
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_development_owner_receipt_invalid",
            ));
        }
        Ok(())
    }

    pub(super) fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_DEVELOPMENT_OWNER_RECEIPT_SCHEMA_V1,
            (
                self.mode,
                &self.owner_request_root_sha256,
                &self.attempt_root_sha256,
                &self.owner_executable_sha256,
                &self.generator_executable_sha256,
                &self.generator_request_root_sha256,
                &self.generator_response_root_sha256,
                &self.public_batch_root_sha256,
            ),
            (
                &self.private_batch_root_sha256,
                &self.split_receipt_root_sha256,
                &self.pipe_receipt_root_sha256,
                &self.cases_generated_event_root_sha256,
                self.generator_dispatch_count,
                &self.nonce_commitment_sha256,
                &self.authorization_receipt_root_sha256,
                &self.authorization_slot_claim_root_sha256,
            ),
            self.sealed_attempts,
            &self.authority,
        ))
    }
}

pub fn development_private_reconstruction_root_v1(
    ordered_private_case_roots: &[String],
    private_batch_root_sha256: &str,
    generator_response_root_sha256: &str,
    response_bytes: u64,
    response_bytes_sha256: &str,
) -> K2CompositionResultV1<String> {
    if ordered_private_case_roots.len() != K2_UNCERTAINTY_CONFIRM_CASES_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_reconstruction_count_invalid",
        ));
    }
    for root in ordered_private_case_roots
        .iter()
        .map(String::as_str)
        .chain([
            private_batch_root_sha256,
            generator_response_root_sha256,
            response_bytes_sha256,
        ])
    {
        require_composition_root_v1(root)?;
    }
    if response_bytes == 0 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_reconstruction_bytes_invalid",
        ));
    }
    uncertainty_root_v1(&(
        K2_UNCERTAINTY_DEVELOPMENT_RECONSTRUCTION_DOMAIN_V1,
        ordered_private_case_roots,
        private_batch_root_sha256,
        generator_response_root_sha256,
        response_bytes,
        response_bytes_sha256,
    ))
}

fn private_ordinals_v1(
    artifacts: &[K2UncertaintyDevelopmentRehearsalStoredArtifactV1],
    kind: K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1,
) -> Vec<u64> {
    let mut values = artifacts
        .iter()
        .filter(|value| value.kind == kind)
        .filter_map(|value| value.private_case_ordinal)
        .collect::<Vec<_>>();
    values.sort_unstable();
    values
}
