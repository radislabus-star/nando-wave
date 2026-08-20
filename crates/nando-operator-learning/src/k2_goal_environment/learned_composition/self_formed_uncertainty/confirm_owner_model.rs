use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    require_composition_root_v1, valid_composition_path_v1,
};
use super::{
    K2_UNCERTAINTY_CONFIRM_OWNER_RECEIPT_SCHEMA_V1, K2_UNCERTAINTY_CONFIRM_OWNER_REQUEST_SCHEMA_V1,
    K2_UNCERTAINTY_CONFIRM_PIPE_RECEIPT_SCHEMA_V1, K2UncertaintyAuthorizationSlotClaimV1,
    K2UncertaintyConfirmAttemptDescriptorV1, K2UncertaintyConfirmAttemptModeV1,
    K2UncertaintyGeneratorRequestV1, K2UncertaintyR10AuthorizationReceiptV1, denied_authority_v1,
    require_denied_authority_v1, uncertainty_root_v1,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyConfirmOwnerRequestV1 {
    pub schema: String,
    pub descriptor: K2UncertaintyConfirmAttemptDescriptorV1,
    pub lab_root: String,
    pub attempt_relative_path: String,
    pub slot_ledger_relative_path: Option<String>,
    pub generator_executable_path: String,
    pub development_generator_request: Option<K2UncertaintyGeneratorRequestV1>,
    pub authorization_receipt: Option<K2UncertaintyR10AuthorizationReceiptV1>,
    pub authorization_slot_claim: Option<K2UncertaintyAuthorizationSlotClaimV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintyConfirmOwnerRequestV1 {
    pub(crate) fn reseal_envelope_for_r7k_control_v1(&mut self) -> K2CompositionResultV1<()> {
        self.request_root_sha256 = self.expected_root()?;
        Ok(())
    }

    pub fn development_rehearsal(
        descriptor: K2UncertaintyConfirmAttemptDescriptorV1,
        lab_root: String,
        attempt_relative_path: String,
        generator_executable_path: String,
        development_generator_request: K2UncertaintyGeneratorRequestV1,
    ) -> K2CompositionResultV1<Self> {
        Self::seal(
            descriptor,
            lab_root,
            attempt_relative_path,
            None,
            generator_executable_path,
            Some(development_generator_request),
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn confirm(
        descriptor: K2UncertaintyConfirmAttemptDescriptorV1,
        lab_root: String,
        attempt_relative_path: String,
        slot_ledger_relative_path: String,
        generator_executable_path: String,
        authorization_receipt: K2UncertaintyR10AuthorizationReceiptV1,
        authorization_slot_claim: K2UncertaintyAuthorizationSlotClaimV1,
    ) -> K2CompositionResultV1<Self> {
        Self::seal(
            descriptor,
            lab_root,
            attempt_relative_path,
            Some(slot_ledger_relative_path),
            generator_executable_path,
            None,
            Some(authorization_receipt),
            Some(authorization_slot_claim),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn seal(
        descriptor: K2UncertaintyConfirmAttemptDescriptorV1,
        lab_root: String,
        attempt_relative_path: String,
        slot_ledger_relative_path: Option<String>,
        generator_executable_path: String,
        development_generator_request: Option<K2UncertaintyGeneratorRequestV1>,
        authorization_receipt: Option<K2UncertaintyR10AuthorizationReceiptV1>,
        authorization_slot_claim: Option<K2UncertaintyAuthorizationSlotClaimV1>,
    ) -> K2CompositionResultV1<Self> {
        let mut request = Self {
            schema: K2_UNCERTAINTY_CONFIRM_OWNER_REQUEST_SCHEMA_V1.to_owned(),
            descriptor,
            lab_root,
            attempt_relative_path,
            slot_ledger_relative_path,
            generator_executable_path,
            development_generator_request,
            authorization_receipt,
            authorization_slot_claim,
            authority: denied_authority_v1(),
            request_root_sha256: String::new(),
        };
        request.request_root_sha256 = request.expected_root()?;
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.descriptor.validate()?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CONFIRM_OWNER_REQUEST_SCHEMA_V1
            || !absolute_clean_path_v1(&self.lab_root)
            || !absolute_clean_path_v1(&self.generator_executable_path)
            || !valid_composition_path_v1(&self.attempt_relative_path)
            || self.attempt_relative_path.contains('/')
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_owner_request_invalid",
            ));
        }
        match self.descriptor.mode {
            K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal => {
                let generator_request = self.development_generator_request.as_ref().ok_or(
                    K2CompositionErrorV1::Invalid(
                        "self_formed_development_owner_generator_request_missing",
                    ),
                )?;
                generator_request.validate()?;
                if self.slot_ledger_relative_path.is_some()
                    || self.authorization_receipt.is_some()
                    || self.authorization_slot_claim.is_some()
                    || generator_request.generator_executable_sha256
                        != self.descriptor.generator_executable_sha256
                {
                    return Err(K2CompositionErrorV1::Invalid(
                        "self_formed_development_owner_binding_invalid",
                    ));
                }
            }
            K2UncertaintyConfirmAttemptModeV1::Confirm => {
                let ledger_path = self.slot_ledger_relative_path.as_deref().ok_or(
                    K2CompositionErrorV1::Invalid("self_formed_confirm_owner_ledger_missing"),
                )?;
                if !valid_composition_path_v1(ledger_path)
                    || ledger_path.contains('/')
                    || self.development_generator_request.is_some()
                {
                    return Err(K2CompositionErrorV1::Invalid(
                        "self_formed_confirm_owner_mode_invalid",
                    ));
                }
                let authorization =
                    self.authorization_receipt
                        .as_ref()
                        .ok_or(K2CompositionErrorV1::Invalid(
                            "self_formed_confirm_owner_authorization_missing",
                        ))?;
                let claim =
                    self.authorization_slot_claim
                        .as_ref()
                        .ok_or(K2CompositionErrorV1::Invalid(
                            "self_formed_confirm_owner_claim_missing",
                        ))?;
                authorization.validate()?;
                claim.validate()?;
                if self.descriptor.authorization_receipt_root_sha256.as_deref()
                    != Some(authorization.receipt_root_sha256.as_str())
                    || self
                        .descriptor
                        .authorization_slot_claim_root_sha256
                        .as_deref()
                        != Some(claim.claim_root_sha256.as_str())
                    || claim.authorization_receipt_root_sha256 != authorization.receipt_root_sha256
                    || claim.slot_key != authorization.slot_key()?
                {
                    return Err(K2CompositionErrorV1::Invalid(
                        "self_formed_confirm_owner_authority_binding_invalid",
                    ));
                }
            }
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONFIRM_OWNER_REQUEST_SCHEMA_V1,
            &self.descriptor,
            &self.lab_root,
            &self.attempt_relative_path,
            &self.slot_ledger_relative_path,
            &self.generator_executable_path,
            &self.development_generator_request,
            &self.authorization_receipt,
            &self.authorization_slot_claim,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyConfirmPipeReceiptV1 {
    pub schema: String,
    pub generator_executable_sha256: String,
    pub generator_request_root_sha256: String,
    pub request_bytes: u64,
    pub response_bytes: u64,
    pub response_bytes_sha256: String,
    pub child_invocations: u64,
    pub stdin_send_operations: u64,
    pub argv_secret_matches: u64,
    pub environment_secret_matches: u64,
    pub path_secret_matches: u64,
    pub output_secret_matches: u64,
    pub request_artifact_writes: u64,
    pub log_writes: u64,
    pub network_namespace_isolated: bool,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyConfirmPipeReceiptV1 {
    pub(crate) fn seal(
        generator_executable_sha256: String,
        generator_request_root_sha256: String,
        request_bytes: u64,
        response_bytes: u64,
        response_bytes_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut receipt = Self {
            schema: K2_UNCERTAINTY_CONFIRM_PIPE_RECEIPT_SCHEMA_V1.to_owned(),
            generator_executable_sha256,
            generator_request_root_sha256,
            request_bytes,
            response_bytes,
            response_bytes_sha256,
            child_invocations: 1,
            stdin_send_operations: 1,
            argv_secret_matches: 0,
            environment_secret_matches: 0,
            path_secret_matches: 0,
            output_secret_matches: 0,
            request_artifact_writes: 0,
            log_writes: 0,
            network_namespace_isolated: true,
            authority: denied_authority_v1(),
            receipt_root_sha256: String::new(),
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.generator_executable_sha256,
            &self.generator_request_root_sha256,
            &self.response_bytes_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CONFIRM_PIPE_RECEIPT_SCHEMA_V1
            || self.request_bytes == 0
            || self.response_bytes == 0
            || self.child_invocations != 1
            || self.stdin_send_operations != 1
            || self.argv_secret_matches != 0
            || self.environment_secret_matches != 0
            || self.path_secret_matches != 0
            || self.output_secret_matches != 0
            || self.request_artifact_writes != 0
            || self.log_writes != 0
            || !self.network_namespace_isolated
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_pipe_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONFIRM_PIPE_RECEIPT_SCHEMA_V1,
            &self.generator_executable_sha256,
            &self.generator_request_root_sha256,
            self.request_bytes,
            self.response_bytes,
            &self.response_bytes_sha256,
            self.child_invocations,
            self.stdin_send_operations,
            self.argv_secret_matches,
            self.environment_secret_matches,
            self.path_secret_matches,
            self.output_secret_matches,
            self.request_artifact_writes,
            self.log_writes,
            self.network_namespace_isolated,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyConfirmOwnerReceiptV1 {
    pub schema: String,
    pub owner_request_root_sha256: String,
    pub attempt_root_sha256: String,
    pub mode: K2UncertaintyConfirmAttemptModeV1,
    pub confirm_owner_executable_sha256: String,
    pub generator_executable_sha256: String,
    pub generator_request_root_sha256: String,
    pub generator_response_root_sha256: String,
    pub public_batch_root_sha256: String,
    pub private_batch_root_sha256: String,
    pub split_receipt_root_sha256: Option<String>,
    pub nonce_commitment_sha256: Option<String>,
    pub pipe_receipt: K2UncertaintyConfirmPipeReceiptV1,
    pub journal_last_event_root_sha256: String,
    pub generator_dispatch_count: u64,
    pub sealed_attempts: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyConfirmOwnerReceiptV1 {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn seal(
        request: &K2UncertaintyConfirmOwnerRequestV1,
        generator_response_root_sha256: String,
        public_batch_root_sha256: String,
        private_batch_root_sha256: String,
        generator_request_root_sha256: String,
        split_receipt_root_sha256: Option<String>,
        nonce_commitment_sha256: Option<String>,
        pipe_receipt: K2UncertaintyConfirmPipeReceiptV1,
        journal_last_event_root_sha256: String,
        generator_dispatch_count: u64,
    ) -> K2CompositionResultV1<Self> {
        let mut receipt = Self {
            schema: K2_UNCERTAINTY_CONFIRM_OWNER_RECEIPT_SCHEMA_V1.to_owned(),
            owner_request_root_sha256: request.request_root_sha256.clone(),
            attempt_root_sha256: request.descriptor.attempt_root_sha256.clone(),
            mode: request.descriptor.mode,
            confirm_owner_executable_sha256: request
                .descriptor
                .confirm_owner_executable_sha256
                .clone(),
            generator_executable_sha256: request.descriptor.generator_executable_sha256.clone(),
            generator_request_root_sha256,
            generator_response_root_sha256,
            public_batch_root_sha256,
            private_batch_root_sha256,
            split_receipt_root_sha256,
            nonce_commitment_sha256,
            pipe_receipt,
            journal_last_event_root_sha256,
            generator_dispatch_count,
            sealed_attempts: request.descriptor.sealed_attempts,
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
            &self.confirm_owner_executable_sha256,
            &self.generator_executable_sha256,
            &self.generator_request_root_sha256,
            &self.generator_response_root_sha256,
            &self.public_batch_root_sha256,
            &self.private_batch_root_sha256,
            &self.journal_last_event_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.pipe_receipt.validate()?;
        for root in [
            &self.split_receipt_root_sha256,
            &self.nonce_commitment_sha256,
        ]
        .into_iter()
        .flatten()
        {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        let mode_valid = match self.mode {
            K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal => {
                self.split_receipt_root_sha256.is_none()
                    && self.nonce_commitment_sha256.is_none()
                    && self.sealed_attempts == 0
            }
            K2UncertaintyConfirmAttemptModeV1::Confirm => {
                self.split_receipt_root_sha256.is_some()
                    && self.nonce_commitment_sha256.is_some()
                    && self.sealed_attempts == 1
            }
        };
        if self.schema != K2_UNCERTAINTY_CONFIRM_OWNER_RECEIPT_SCHEMA_V1
            || !mode_valid
            || self.generator_dispatch_count != 1
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_owner_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONFIRM_OWNER_RECEIPT_SCHEMA_V1,
            (
                &self.owner_request_root_sha256,
                &self.attempt_root_sha256,
                self.mode,
                &self.confirm_owner_executable_sha256,
                &self.generator_executable_sha256,
                &self.generator_request_root_sha256,
                &self.generator_response_root_sha256,
                &self.public_batch_root_sha256,
            ),
            (
                &self.private_batch_root_sha256,
                &self.split_receipt_root_sha256,
                &self.nonce_commitment_sha256,
                &self.pipe_receipt,
                &self.journal_last_event_root_sha256,
                self.generator_dispatch_count,
                self.sealed_attempts,
            ),
            &self.authority,
        ))
    }
}

fn absolute_clean_path_v1(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4096
        && !value
            .bytes()
            .any(|byte| byte == 0 || byte.is_ascii_control())
        && Path::new(value).is_absolute()
}
