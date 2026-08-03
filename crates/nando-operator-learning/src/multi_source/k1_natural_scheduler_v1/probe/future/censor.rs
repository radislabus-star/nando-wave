use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

const K1_FUTURE_PREDICTION_CENSOR_RECEIPT_SCHEMA_V1: &str =
    "nando.k1-future-prediction-censor-receipt.v1";
pub const K1_MISSING_COMPLETED_FRAME_BLOCKER_V1: &str = "CENSORED_MISSING_COMPLETED_FRAME";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1FuturePredictionCensorReceiptV1 {
    pub schema: String,
    pub censor_root_sha256: String,
    pub prediction_root_sha256: String,
    pub topology_commitment_root_sha256: String,
    pub prediction_capture_sequence: u64,
    pub request_event_id_sha256: String,
    pub terminal_receipt_root_sha256: String,
    pub terminal_completed_at_unix_nanos: u64,
    pub fence_topology_commitment_root_sha256: String,
    pub fence_request_event_id_sha256: String,
    pub session_lineage_sha256: String,
    pub fence_capture_sequence: u64,
    pub fence_captured_at_unix_nanos: u64,
    pub blocker: String,
    pub censored_at_unix_nanos: u64,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

impl K1FuturePredictionCensorReceiptV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal_missing_completed_frame(
        prediction_root_sha256: String,
        topology_commitment_root_sha256: String,
        prediction_capture_sequence: u64,
        request_event_id_sha256: String,
        terminal_receipt_root_sha256: String,
        terminal_completed_at_unix_nanos: u64,
        fence_topology_commitment_root_sha256: String,
        fence_request_event_id_sha256: String,
        session_lineage_sha256: String,
        fence_capture_sequence: u64,
        fence_captured_at_unix_nanos: u64,
        censored_at_unix_nanos: u64,
    ) -> Result<Self, &'static str> {
        let mut receipt = Self {
            schema: K1_FUTURE_PREDICTION_CENSOR_RECEIPT_SCHEMA_V1.to_owned(),
            censor_root_sha256: String::new(),
            prediction_root_sha256,
            topology_commitment_root_sha256,
            prediction_capture_sequence,
            request_event_id_sha256,
            terminal_receipt_root_sha256,
            terminal_completed_at_unix_nanos,
            fence_topology_commitment_root_sha256,
            fence_request_event_id_sha256,
            session_lineage_sha256,
            fence_capture_sequence,
            fence_captured_at_unix_nanos,
            blocker: K1_MISSING_COMPLETED_FRAME_BLOCKER_V1.to_owned(),
            censored_at_unix_nanos,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        receipt.censor_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != K1_FUTURE_PREDICTION_CENSOR_RECEIPT_SCHEMA_V1
            || ![
                self.censor_root_sha256.as_str(),
                self.prediction_root_sha256.as_str(),
                self.topology_commitment_root_sha256.as_str(),
                self.request_event_id_sha256.as_str(),
                self.terminal_receipt_root_sha256.as_str(),
                self.fence_topology_commitment_root_sha256.as_str(),
                self.fence_request_event_id_sha256.as_str(),
                self.session_lineage_sha256.as_str(),
            ]
            .into_iter()
            .all(valid_nonzero_sha256)
            || self.prediction_capture_sequence == 0
            || self.fence_capture_sequence <= self.prediction_capture_sequence
            || self.terminal_completed_at_unix_nanos == 0
            || self.fence_captured_at_unix_nanos <= self.terminal_completed_at_unix_nanos
            || self.censored_at_unix_nanos < self.fence_captured_at_unix_nanos
            || self.request_event_id_sha256 == self.fence_request_event_id_sha256
            || self.topology_commitment_root_sha256 == self.fence_topology_commitment_root_sha256
            || self.blocker != K1_MISSING_COMPLETED_FRAME_BLOCKER_V1
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.censor_root_sha256 != self.expected_root()?
        {
            return Err("k1_future_prediction_censor_receipt_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            K1_FUTURE_PREDICTION_CENSOR_RECEIPT_SCHEMA_V1,
            self.prediction_root_sha256.as_str(),
            self.topology_commitment_root_sha256.as_str(),
            self.prediction_capture_sequence,
            self.request_event_id_sha256.as_str(),
            self.terminal_receipt_root_sha256.as_str(),
            self.terminal_completed_at_unix_nanos,
            self.fence_topology_commitment_root_sha256.as_str(),
            self.fence_request_event_id_sha256.as_str(),
            self.session_lineage_sha256.as_str(),
            self.fence_capture_sequence,
            self.fence_captured_at_unix_nanos,
            self.blocker.as_str(),
            self.censored_at_unix_nanos,
            false,
            false,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nando_operator_kernel::sha256_bytes;

    fn root(label: &str) -> String {
        sha256_bytes(label.as_bytes())
    }

    #[test]
    fn missing_frame_censor_requires_a_later_same_lineage_fence_shape() {
        let receipt = K1FuturePredictionCensorReceiptV1::seal_missing_completed_frame(
            root("prediction"),
            root("topology"),
            7,
            root("request"),
            root("terminal"),
            100,
            root("fence-topology"),
            root("fence-request"),
            root("lineage"),
            8,
            101,
            102,
        )
        .expect("censor receipt");

        assert_eq!(receipt.blocker, K1_MISSING_COMPLETED_FRAME_BLOCKER_V1);
        assert!(!receipt.authority_ready);
        assert!(!receipt.phase_mutation_allowed);
        assert_eq!(receipt.validate(), Ok(()));

        let mut invalid = receipt;
        invalid.fence_capture_sequence = invalid.prediction_capture_sequence;
        assert_eq!(
            invalid.validate(),
            Err("k1_future_prediction_censor_receipt_invalid")
        );
    }
}
