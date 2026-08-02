use std::fmt;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use nando_operator_learning::{is_source_neutral_relation_frame, teacher_outcome_from_completed};
use serde::{Deserialize, Serialize};

use super::{
    REMOTE_EVIDENCE_FRAME_SCHEMA_V1, REMOTE_EVIDENCE_MAX_FRAME_BYTES_V1, RemoteEvidenceFrameV1,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteEvidenceFrameValidationBlockerV1 {
    ActionEventRootInvalid,
    FrameRootInvalid,
    FrameRootMismatch,
    ObservedAtMissing,
    ObservedAtMismatch,
    ParityEvidenceReferenceMismatch,
    ParityEncodeFailed,
    ParityExpectedResponseMissing,
    ParityFrameBudget,
    ParityRequestMissing,
    RouteConfirmationAfterFrame,
    RouteIntentMismatch,
    RouteReceiptInvalid,
    RouteReceiptRootInvalid,
    RouteReceiptRootMismatch,
    RouteReceiptRootMissing,
    RouteRequestAfterFrame,
    RouteSessionMismatch,
    SchemaInvalid,
    SessionRootInvalid,
    SourceSpecificFrame,
    TeacherOutcomeInvalid,
    TurnIntentRootInvalid,
    VerifierLabelNotAccepted,
    VerifierReceiptRootInvalid,
    VerifierReceiptRootMismatch,
    VerifierRejected,
}

impl RemoteEvidenceFrameValidationBlockerV1 {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ActionEventRootInvalid => "action_event_root_invalid",
            Self::FrameRootInvalid => "frame_root_invalid",
            Self::FrameRootMismatch => "frame_root_mismatch",
            Self::ObservedAtMissing => "observed_at_missing",
            Self::ObservedAtMismatch => "observed_at_mismatch",
            Self::ParityEvidenceReferenceMismatch => "parity_evidence_reference_mismatch",
            Self::ParityEncodeFailed => "parity_encode_failed",
            Self::ParityExpectedResponseMissing => "parity_expected_response_missing",
            Self::ParityFrameBudget => "parity_frame_budget",
            Self::ParityRequestMissing => "parity_request_missing",
            Self::RouteConfirmationAfterFrame => "route_confirmation_after_frame",
            Self::RouteIntentMismatch => "route_intent_mismatch",
            Self::RouteReceiptInvalid => "route_receipt_invalid",
            Self::RouteReceiptRootInvalid => "route_receipt_root_invalid",
            Self::RouteReceiptRootMismatch => "route_receipt_root_mismatch",
            Self::RouteReceiptRootMissing => "route_receipt_root_missing",
            Self::RouteRequestAfterFrame => "route_request_after_frame",
            Self::RouteSessionMismatch => "route_session_mismatch",
            Self::SchemaInvalid => "schema_invalid",
            Self::SessionRootInvalid => "session_root_invalid",
            Self::SourceSpecificFrame => "source_specific_frame",
            Self::TeacherOutcomeInvalid => "teacher_outcome_invalid",
            Self::TurnIntentRootInvalid => "turn_intent_root_invalid",
            Self::VerifierLabelNotAccepted => "verifier_label_not_accepted",
            Self::VerifierReceiptRootInvalid => "verifier_receipt_root_invalid",
            Self::VerifierReceiptRootMismatch => "verifier_receipt_root_mismatch",
            Self::VerifierRejected => "verifier_rejected",
        }
    }
}

impl fmt::Display for RemoteEvidenceFrameValidationBlockerV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RemoteEvidenceFrameSealErrorV1 {
    Censored(RemoteEvidenceFrameValidationBlockerV1),
    Fatal(String),
}

impl RemoteEvidenceFrameSealErrorV1 {
    #[must_use]
    pub const fn censor_blocker(&self) -> Option<RemoteEvidenceFrameValidationBlockerV1> {
        match self {
            Self::Censored(blocker) => Some(*blocker),
            Self::Fatal(_) => None,
        }
    }
}

impl fmt::Display for RemoteEvidenceFrameSealErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Censored(blocker) => {
                write!(formatter, "remote_evidence_frame_invalid:{blocker}")
            }
            Self::Fatal(error) => formatter.write_str(error),
        }
    }
}

impl std::error::Error for RemoteEvidenceFrameSealErrorV1 {}

impl RemoteEvidenceFrameV1 {
    #[must_use]
    pub fn validation_blocker(&self) -> Option<RemoteEvidenceFrameValidationBlockerV1> {
        if self.schema != REMOTE_EVIDENCE_FRAME_SCHEMA_V1 {
            return Some(RemoteEvidenceFrameValidationBlockerV1::SchemaInvalid);
        }
        if self.frame.verifier_label != Some(true) {
            return Some(RemoteEvidenceFrameValidationBlockerV1::VerifierLabelNotAccepted);
        }
        if !is_source_neutral_relation_frame(&self.frame) {
            return Some(RemoteEvidenceFrameValidationBlockerV1::SourceSpecificFrame);
        }
        if self.observed_at_unix_nanos == 0 {
            return Some(RemoteEvidenceFrameValidationBlockerV1::ObservedAtMissing);
        }
        if !valid_nonzero_sha256(&self.frame_root_sha256) {
            return Some(RemoteEvidenceFrameValidationBlockerV1::FrameRootInvalid);
        }
        if !valid_nonzero_sha256(&self.verifier_receipt_root_sha256) {
            return Some(RemoteEvidenceFrameValidationBlockerV1::VerifierReceiptRootInvalid);
        }
        if !valid_nonzero_sha256(&self.session_id_sha256) {
            return Some(RemoteEvidenceFrameValidationBlockerV1::SessionRootInvalid);
        }
        if !valid_nonzero_sha256(&self.turn_intent_id_sha256) {
            return Some(RemoteEvidenceFrameValidationBlockerV1::TurnIntentRootInvalid);
        }
        if !valid_nonzero_sha256(&self.action_event_id_sha256) {
            return Some(RemoteEvidenceFrameValidationBlockerV1::ActionEventRootInvalid);
        }
        if self
            .route_receipt_root_sha256
            .as_deref()
            .is_some_and(|root| !valid_nonzero_sha256(root))
        {
            return Some(RemoteEvidenceFrameValidationBlockerV1::RouteReceiptRootInvalid);
        }
        if let Some(blocker) = self.route_binding_blocker() {
            return Some(blocker);
        }
        if self.session_id_sha256 != self.frame.session_id_sha256 {
            return Some(RemoteEvidenceFrameValidationBlockerV1::SessionRootInvalid);
        }
        if self.turn_intent_id_sha256 != self.frame.client_intent_id_sha256 {
            return Some(RemoteEvidenceFrameValidationBlockerV1::TurnIntentRootInvalid);
        }
        if self.action_event_id_sha256 != self.frame.event_id_sha256 {
            return Some(RemoteEvidenceFrameValidationBlockerV1::ActionEventRootInvalid);
        }
        if self.observed_at_unix_nanos != self.frame.observed_at_unix_nanos {
            return Some(RemoteEvidenceFrameValidationBlockerV1::ObservedAtMismatch);
        }
        if let Some(parity) = &self.runtime_parity_case {
            if parity.evidence_ref_sha256 != self.frame.frame_id_sha256 {
                return Some(
                    RemoteEvidenceFrameValidationBlockerV1::ParityEvidenceReferenceMismatch,
                );
            }
            if parity.request_text.is_empty() {
                return Some(RemoteEvidenceFrameValidationBlockerV1::ParityRequestMissing);
            }
            if parity.expected_response.is_empty() {
                return Some(RemoteEvidenceFrameValidationBlockerV1::ParityExpectedResponseMissing);
            }
            match serde_cbor::to_vec(parity) {
                Err(_) => {
                    return Some(RemoteEvidenceFrameValidationBlockerV1::ParityEncodeFailed);
                }
                Ok(bytes) if bytes.len() > REMOTE_EVIDENCE_MAX_FRAME_BYTES_V1 => {
                    return Some(RemoteEvidenceFrameValidationBlockerV1::ParityFrameBudget);
                }
                Ok(_) => {}
            }
        }
        match canonical_json_sha256(&self.frame) {
            Ok(root) if root == self.frame_root_sha256 => {}
            Ok(_) => return Some(RemoteEvidenceFrameValidationBlockerV1::FrameRootMismatch),
            Err(_) => return Some(RemoteEvidenceFrameValidationBlockerV1::FrameRootInvalid),
        }
        let outcome = match teacher_outcome_from_completed(&self.frame) {
            Ok(outcome) => outcome,
            Err(_) => {
                return Some(RemoteEvidenceFrameValidationBlockerV1::TeacherOutcomeInvalid);
            }
        };
        if !outcome.verifier.accepted {
            return Some(RemoteEvidenceFrameValidationBlockerV1::VerifierRejected);
        }
        match canonical_json_sha256(&outcome.verifier) {
            Ok(root) if root == self.verifier_receipt_root_sha256 => None,
            Ok(_) => Some(RemoteEvidenceFrameValidationBlockerV1::VerifierReceiptRootMismatch),
            Err(_) => Some(RemoteEvidenceFrameValidationBlockerV1::VerifierReceiptRootInvalid),
        }
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.validation_blocker().is_none()
    }

    #[must_use]
    pub fn is_route_bound(&self) -> bool {
        self.route_receipt.is_some() && self.route_binding_blocker().is_none()
    }

    fn route_binding_blocker(&self) -> Option<RemoteEvidenceFrameValidationBlockerV1> {
        let (Some(root), Some(receipt)) = (
            self.route_receipt_root_sha256.as_deref(),
            self.route_receipt.as_ref(),
        ) else {
            return match (
                self.route_receipt_root_sha256.as_deref(),
                self.route_receipt.as_ref(),
            ) {
                (None, None) | (Some(_), None) => None,
                (None, Some(_)) => {
                    Some(RemoteEvidenceFrameValidationBlockerV1::RouteReceiptRootMissing)
                }
                (Some(_), Some(_)) => unreachable!(),
            };
        };
        if !receipt.validate() {
            return Some(RemoteEvidenceFrameValidationBlockerV1::RouteReceiptInvalid);
        }
        if root != receipt.receipt_root_sha256 {
            return Some(RemoteEvidenceFrameValidationBlockerV1::RouteReceiptRootMismatch);
        }
        if receipt.turn_intent_id_sha256 != self.frame.client_intent_id_sha256 {
            return Some(RemoteEvidenceFrameValidationBlockerV1::RouteIntentMismatch);
        }
        if receipt.session_id_sha256 != self.frame.session_id_sha256 {
            return Some(RemoteEvidenceFrameValidationBlockerV1::RouteSessionMismatch);
        }
        if receipt.request_observed_at_unix_nanos > self.frame.observed_at_unix_nanos {
            return Some(RemoteEvidenceFrameValidationBlockerV1::RouteRequestAfterFrame);
        }
        if receipt.route_confirmed_at_unix_nanos > self.frame.observed_at_unix_nanos {
            return Some(RemoteEvidenceFrameValidationBlockerV1::RouteConfirmationAfterFrame);
        }
        None
    }
}
