use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ReducibilityClass;

pub const OPPORTUNITY_BRIDGE_EVENT_SCHEMA_V1: &str = "nando.opportunity-bridge-event.v1";
pub const OPPORTUNITY_BRIDGE_MAX_EVENT_BYTES_V1: usize = 4 * 1024;
const MAX_BLOCKER_BYTES: usize = 120;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpportunityBridgeEventV1 {
    pub schema: String,
    pub event: OpportunityBridgeEventKindV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OpportunityBridgeEventKindV1 {
    Request {
        intent_sha256: String,
        input_tokens: u64,
        observed_at_unix: u64,
    },
    Classify {
        intent_sha256: String,
        class: ReducibilityClass,
        blocker: String,
    },
    Verified {
        intent_sha256: String,
    },
    ParityFailure {
        intent_sha256: String,
    },
    FalseAccept {
        intent_sha256: String,
    },
}

impl OpportunityBridgeEventV1 {
    #[must_use]
    pub fn request(intent_sha256: String, input_tokens: u64, observed_at_unix: u64) -> Self {
        Self {
            schema: OPPORTUNITY_BRIDGE_EVENT_SCHEMA_V1.to_owned(),
            event: OpportunityBridgeEventKindV1::Request {
                intent_sha256,
                input_tokens,
                observed_at_unix,
            },
        }
    }

    #[must_use]
    pub fn classify(intent_sha256: String, class: ReducibilityClass, blocker: String) -> Self {
        Self {
            schema: OPPORTUNITY_BRIDGE_EVENT_SCHEMA_V1.to_owned(),
            event: OpportunityBridgeEventKindV1::Classify {
                intent_sha256,
                class,
                blocker,
            },
        }
    }

    #[must_use]
    pub fn verified(intent_sha256: String) -> Self {
        Self {
            schema: OPPORTUNITY_BRIDGE_EVENT_SCHEMA_V1.to_owned(),
            event: OpportunityBridgeEventKindV1::Verified { intent_sha256 },
        }
    }

    #[must_use]
    pub fn parity_failure(intent_sha256: String) -> Self {
        Self {
            schema: OPPORTUNITY_BRIDGE_EVENT_SCHEMA_V1.to_owned(),
            event: OpportunityBridgeEventKindV1::ParityFailure { intent_sha256 },
        }
    }

    #[must_use]
    pub fn false_accept(intent_sha256: String) -> Self {
        Self {
            schema: OPPORTUNITY_BRIDGE_EVENT_SCHEMA_V1.to_owned(),
            event: OpportunityBridgeEventKindV1::FalseAccept { intent_sha256 },
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != OPPORTUNITY_BRIDGE_EVENT_SCHEMA_V1 {
            return Err("opportunity_bridge_schema_mismatch".to_owned());
        }
        if !valid_sha256(self.intent_sha256()) {
            return Err("opportunity_bridge_invalid_intent_sha256".to_owned());
        }
        match &self.event {
            OpportunityBridgeEventKindV1::Request {
                observed_at_unix, ..
            } if *observed_at_unix == 0 => Err("opportunity_bridge_invalid_observed_at".to_owned()),
            OpportunityBridgeEventKindV1::Classify { blocker, .. }
                if blocker.is_empty() || blocker.len() > MAX_BLOCKER_BYTES =>
            {
                Err("opportunity_bridge_invalid_blocker".to_owned())
            }
            _ => Ok(()),
        }
    }

    pub fn canonical_cbor(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let bytes = serde_cbor::to_vec(self)
            .map_err(|error| format!("opportunity_bridge_encode:{error}"))?;
        if bytes.len() > OPPORTUNITY_BRIDGE_MAX_EVENT_BYTES_V1 {
            return Err("opportunity_bridge_event_too_large".to_owned());
        }
        Ok(bytes)
    }

    pub fn from_canonical_cbor(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > OPPORTUNITY_BRIDGE_MAX_EVENT_BYTES_V1 {
            return Err("opportunity_bridge_event_size_invalid".to_owned());
        }
        let event: Self = serde_cbor::from_slice(bytes)
            .map_err(|error| format!("opportunity_bridge_decode:{error}"))?;
        event.validate()?;
        if event.canonical_cbor()?.as_slice() != bytes {
            return Err("opportunity_bridge_noncanonical_event".to_owned());
        }
        Ok(event)
    }

    pub fn canonical_sha256(&self) -> Result<String, String> {
        self.canonical_cbor().map(|bytes| hex_sha256(&bytes))
    }

    #[must_use]
    pub fn intent_sha256(&self) -> &str {
        match &self.event {
            OpportunityBridgeEventKindV1::Request { intent_sha256, .. }
            | OpportunityBridgeEventKindV1::Classify { intent_sha256, .. }
            | OpportunityBridgeEventKindV1::Verified { intent_sha256 }
            | OpportunityBridgeEventKindV1::ParityFailure { intent_sha256 }
            | OpportunityBridgeEventKindV1::FalseAccept { intent_sha256 } => intent_sha256,
        }
    }

    #[must_use]
    pub const fn request_economics(&self) -> Option<(u64, u64)> {
        match self.event {
            OpportunityBridgeEventKindV1::Request {
                input_tokens,
                observed_at_unix,
                ..
            } => Some((input_tokens, observed_at_unix)),
            _ => None,
        }
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent() -> String {
        "31".repeat(32)
    }

    #[test]
    fn event_round_trip_is_canonical_and_stable() {
        let event = OpportunityBridgeEventV1::classify(
            intent(),
            ReducibilityClass::ExecutableCandidate,
            "phase_route_missing".to_owned(),
        );
        let bytes = event.canonical_cbor().expect("canonical event");
        assert_eq!(
            OpportunityBridgeEventV1::from_canonical_cbor(&bytes).expect("decode"),
            event
        );
        assert_eq!(event.canonical_sha256().expect("digest").len(), 64);
    }

    #[test]
    fn malformed_identity_and_unbounded_blocker_are_rejected() {
        let malformed = OpportunityBridgeEventV1::verified("not-a-digest".to_owned());
        assert_eq!(
            malformed.validate().expect_err("invalid identity"),
            "opportunity_bridge_invalid_intent_sha256"
        );
        let unbounded = OpportunityBridgeEventV1::classify(
            intent(),
            ReducibilityClass::UnclassifiedBug,
            "x".repeat(MAX_BLOCKER_BYTES + 1),
        );
        assert_eq!(
            unbounded.validate().expect_err("unbounded blocker"),
            "opportunity_bridge_invalid_blocker"
        );
    }
}
