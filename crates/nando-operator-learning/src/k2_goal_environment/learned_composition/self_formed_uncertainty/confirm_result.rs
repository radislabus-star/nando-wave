use std::io::{Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_sha256_file_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_DEVELOPMENT_RESULT_RECEIPT_SCHEMA_V1,
    K2_UNCERTAINTY_DEVELOPMENT_RESULT_REQUEST_SCHEMA_V1, K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1,
    K2_UNCERTAINTY_SEALED_RESULT_RECEIPT_SCHEMA_V1, K2_UNCERTAINTY_SEALED_RESULT_REQUEST_SCHEMA_V1,
    K2UncertaintyCleanupReceiptV1, K2UncertaintyTerminalDispositionV1,
    K2UncertaintyTerminalEvaluationReceiptV1, K2UncertaintyTerminalModeV1, denied_authority_v1,
    publish_control_bytes_v1, require_denied_authority_v1, uncertainty_bytes_v1,
    uncertainty_decode_v1, uncertainty_root_v1,
};

const DEVELOPMENT_RESULT_FILE_V1: &str = "development-rehearsal-result.json";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum K2UncertaintyDevelopmentResultFaultV1 {
    None,
    BeforeReceipt,
    AfterReceipt,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyDevelopmentResultRequestV1 {
    pub schema: String,
    pub control_root: String,
    pub terminal_receipt: K2UncertaintyTerminalEvaluationReceiptV1,
    pub cleanup_receipt: K2UncertaintyCleanupReceiptV1,
    pub sealed_attempts: u64,
    pub authorization_slots: u64,
    pub confirm_nonce_commitment_sha256: Option<String>,
    pub publisher_executable_sha256: String,
    pub request_root_sha256: String,
}

impl K2UncertaintyDevelopmentResultRequestV1 {
    pub fn seal(
        control_root: String,
        terminal_receipt: K2UncertaintyTerminalEvaluationReceiptV1,
        cleanup_receipt: K2UncertaintyCleanupReceiptV1,
        publisher_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_DEVELOPMENT_RESULT_REQUEST_SCHEMA_V1.to_owned(),
            control_root,
            terminal_receipt,
            cleanup_receipt,
            sealed_attempts: 0,
            authorization_slots: 0,
            confirm_nonce_commitment_sha256: None,
            publisher_executable_sha256,
            request_root_sha256: String::new(),
        };
        value.request_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.terminal_receipt.validate()?;
        self.cleanup_receipt.validate()?;
        require_composition_root_v1(&self.publisher_executable_sha256)?;
        if self.schema != K2_UNCERTAINTY_DEVELOPMENT_RESULT_REQUEST_SCHEMA_V1
            || self.control_root.is_empty()
            || self.terminal_receipt.mode != K2UncertaintyTerminalModeV1::DevelopmentRehearsal
            || self.terminal_receipt.disposition
                != K2UncertaintyTerminalDispositionV1::DevelopmentRehearsalPass
            || self.cleanup_receipt.terminal_receipt_root_sha256
                != self.terminal_receipt.receipt_root_sha256
            || self.sealed_attempts != 0
            || self.authorization_slots != 0
            || self.confirm_nonce_commitment_sha256.is_some()
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_development_result_request_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_DEVELOPMENT_RESULT_REQUEST_SCHEMA_V1,
            &self.control_root,
            &self.terminal_receipt.receipt_root_sha256,
            &self.cleanup_receipt.receipt_root_sha256,
            self.sealed_attempts,
            self.authorization_slots,
            &self.confirm_nonce_commitment_sha256,
            &self.publisher_executable_sha256,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyDevelopmentResultReceiptV1 {
    pub schema: String,
    pub request_root_sha256: String,
    pub terminal_receipt_root_sha256: String,
    pub cleanup_receipt_root_sha256: String,
    pub disposition: String,
    pub publisher_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyDevelopmentResultReceiptV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.request_root_sha256,
            &self.terminal_receipt_root_sha256,
            &self.cleanup_receipt_root_sha256,
            &self.publisher_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_DEVELOPMENT_RESULT_RECEIPT_SCHEMA_V1
            || self.disposition != "DEVELOPMENT_REHEARSAL_COMPLETE"
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_development_result_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.receipt_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_DEVELOPMENT_RESULT_RECEIPT_SCHEMA_V1,
            &self.request_root_sha256,
            &self.terminal_receipt_root_sha256,
            &self.cleanup_receipt_root_sha256,
            &self.disposition,
            &self.publisher_executable_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintySealedResultRequestV1 {
    pub schema: String,
    pub terminal_receipt: K2UncertaintyTerminalEvaluationReceiptV1,
    pub cleanup_receipt: K2UncertaintyCleanupReceiptV1,
    pub publisher_executable_sha256: String,
}

impl K2UncertaintySealedResultRequestV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.terminal_receipt.validate()?;
        self.cleanup_receipt.validate()?;
        require_composition_root_v1(&self.publisher_executable_sha256)?;
        if self.schema != K2_UNCERTAINTY_SEALED_RESULT_REQUEST_SCHEMA_V1
            || self.terminal_receipt.mode != K2UncertaintyTerminalModeV1::SealedAttempt
            || self.cleanup_receipt.terminal_receipt_root_sha256
                != self.terminal_receipt.receipt_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_sealed_result_request_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintySealedResultReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
}

impl K2UncertaintySealedResultReceiptV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.receipt_root_sha256)?;
        if self.schema != K2_UNCERTAINTY_SEALED_RESULT_RECEIPT_SCHEMA_V1 {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_sealed_result_receipt_invalid",
            ));
        }
        Ok(())
    }
}

pub fn publish_self_formed_development_result_v1(
    request: &K2UncertaintyDevelopmentResultRequestV1,
) -> K2CompositionResultV1<K2UncertaintyDevelopmentResultReceiptV1> {
    publish_self_formed_development_result_with_fault_v1(
        request,
        K2UncertaintyDevelopmentResultFaultV1::None,
    )
}

pub(crate) fn publish_self_formed_development_result_with_fault_v1(
    request: &K2UncertaintyDevelopmentResultRequestV1,
    fault: K2UncertaintyDevelopmentResultFaultV1,
) -> K2CompositionResultV1<K2UncertaintyDevelopmentResultReceiptV1> {
    request.validate()?;
    let mut receipt = K2UncertaintyDevelopmentResultReceiptV1 {
        schema: K2_UNCERTAINTY_DEVELOPMENT_RESULT_RECEIPT_SCHEMA_V1.to_owned(),
        request_root_sha256: request.request_root_sha256.clone(),
        terminal_receipt_root_sha256: request.terminal_receipt.receipt_root_sha256.clone(),
        cleanup_receipt_root_sha256: request.cleanup_receipt.receipt_root_sha256.clone(),
        disposition: "DEVELOPMENT_REHEARSAL_COMPLETE".to_owned(),
        publisher_executable_sha256: request.publisher_executable_sha256.clone(),
        authority: denied_authority_v1(),
        receipt_root_sha256: String::new(),
    };
    receipt.reseal()?;
    fail_development_result_at_v1(fault, K2UncertaintyDevelopmentResultFaultV1::BeforeReceipt)?;
    publish_control_bytes_v1(
        &Path::new(&request.control_root).join(DEVELOPMENT_RESULT_FILE_V1),
        &uncertainty_bytes_v1(&receipt)?,
    )?;
    fail_development_result_at_v1(fault, K2UncertaintyDevelopmentResultFaultV1::AfterReceipt)?;
    Ok(receipt)
}

fn fail_development_result_at_v1(
    actual: K2UncertaintyDevelopmentResultFaultV1,
    expected: K2UncertaintyDevelopmentResultFaultV1,
) -> K2CompositionResultV1<()> {
    if actual == expected {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_result_injected_fault",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum K2UncertaintyResultProcessRequestV1 {
    Development {
        request: K2UncertaintyDevelopmentResultRequestV1,
    },
    Sealed {
        request: K2UncertaintySealedResultRequestV1,
    },
}

pub fn run_self_formed_result_publisher_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_result_publisher_stdin"))?;
    let request: K2UncertaintyResultProcessRequestV1 = uncertainty_decode_v1(&input)?;
    match request {
        K2UncertaintyResultProcessRequestV1::Development { request } => {
            let executable = std::env::current_exe()
                .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_result_publisher"))?;
            if composition_sha256_file_v1(&executable)? != request.publisher_executable_sha256 {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_result_publisher_executable_mismatch",
                ));
            }
            let receipt = publish_self_formed_development_result_v1(&request)?;
            std::io::stdout()
                .write_all(&uncertainty_bytes_v1(&receipt)?)
                .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_result_stdout"))
        }
        K2UncertaintyResultProcessRequestV1::Sealed { request } => {
            request.validate()?;
            Err(K2CompositionErrorV1::Invalid(
                "self_formed_sealed_result_locked_until_r8b",
            ))
        }
    }
}
