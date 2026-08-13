use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::GroundedDecisionShadowCensorV1;
use crate::OpportunityBridgeEventV1;

pub const S1C4_CLASSIFICATION_ROW_SCHEMA_V1: &str = "nando.s1c4-classification-row.v1";
pub const S1C4_CLASSIFICATION_LEDGER_PREFIX_V1: &str = "s1c4-classification";
pub const S1C4_MAX_CLASSIFICATION_ROWS_V1: u64 = 4096;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum S1c4TerminalClassificationV1 {
    DecisionRecorded {
        decision_precommit_root_sha256: String,
    },
    Censored {
        reason: GroundedDecisionShadowCensorV1,
    },
}

impl S1c4TerminalClassificationV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::DecisionRecorded {
                decision_precommit_root_sha256,
            } if !valid_nonzero_sha256(decision_precommit_root_sha256) => {
                Err("s1c4_decision_precommit_root_invalid")
            }
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct S1c4ClassificationRowV1 {
    pub schema: String,
    pub row_root_sha256: String,
    pub previous_row_root_sha256: String,
    pub opportunity_sequence: u64,
    pub opportunity_request_ordinal: u64,
    pub opportunity_event_root_sha256: String,
    pub request_input_tokens: u64,
    pub request_observed_at_unix: u64,
    pub request_event_identity_root_sha256: String,
    pub session_lineage_root_sha256: String,
    pub observed_at_unix_ms: u64,
    pub classification: S1c4TerminalClassificationV1,
}

#[derive(Serialize)]
struct S1c4ClassificationRowDigestV1<'a> {
    schema: &'static str,
    previous_row_root_sha256: &'a str,
    opportunity_sequence: u64,
    opportunity_request_ordinal: u64,
    opportunity_event_root_sha256: &'a str,
    request_input_tokens: u64,
    request_observed_at_unix: u64,
    request_event_identity_root_sha256: &'a str,
    session_lineage_root_sha256: &'a str,
    observed_at_unix_ms: u64,
    classification: &'a S1c4TerminalClassificationV1,
}

impl S1c4ClassificationRowV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        previous_row_root_sha256: String,
        opportunity_sequence: u64,
        opportunity_request_ordinal: u64,
        opportunity_event_root_sha256: String,
        request_input_tokens: u64,
        request_observed_at_unix: u64,
        request_event_identity_root_sha256: String,
        session_lineage_root_sha256: String,
        observed_at_unix_ms: u64,
        classification: S1c4TerminalClassificationV1,
    ) -> Result<Self, &'static str> {
        classification.validate()?;
        if !valid_nonzero_sha256(&previous_row_root_sha256)
            || opportunity_sequence == 0
            || opportunity_request_ordinal == 0
            || !valid_nonzero_sha256(&opportunity_event_root_sha256)
            || request_observed_at_unix == 0
            || !valid_nonzero_sha256(&request_event_identity_root_sha256)
            || !valid_nonzero_sha256(&session_lineage_root_sha256)
            || observed_at_unix_ms == 0
        {
            return Err("s1c4_classification_input_invalid");
        }
        let mut row = Self {
            schema: S1C4_CLASSIFICATION_ROW_SCHEMA_V1.to_owned(),
            row_root_sha256: String::new(),
            previous_row_root_sha256,
            opportunity_sequence,
            opportunity_request_ordinal,
            opportunity_event_root_sha256,
            request_input_tokens,
            request_observed_at_unix,
            request_event_identity_root_sha256,
            session_lineage_root_sha256,
            observed_at_unix_ms,
            classification,
        };
        row.row_root_sha256 = row.expected_root()?;
        row.validate()?;
        Ok(row)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        self.classification.validate()?;
        let request_event = OpportunityBridgeEventV1::request(
            self.request_event_identity_root_sha256.clone(),
            self.request_input_tokens,
            self.request_observed_at_unix,
        );
        if self.schema != S1C4_CLASSIFICATION_ROW_SCHEMA_V1
            || !valid_nonzero_sha256(&self.row_root_sha256)
            || !valid_nonzero_sha256(&self.previous_row_root_sha256)
            || self.opportunity_sequence == 0
            || self.opportunity_request_ordinal == 0
            || !valid_nonzero_sha256(&self.opportunity_event_root_sha256)
            || self.request_observed_at_unix == 0
            || !valid_nonzero_sha256(&self.request_event_identity_root_sha256)
            || !valid_nonzero_sha256(&self.session_lineage_root_sha256)
            || self.observed_at_unix_ms == 0
            || request_event
                .canonical_sha256()
                .map_err(|_| "s1c4_classification_request_projection_invalid")?
                != self.opportunity_event_root_sha256
            || self.expected_root()? != self.row_root_sha256
        {
            return Err("s1c4_classification_row_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&S1c4ClassificationRowDigestV1 {
            schema: S1C4_CLASSIFICATION_ROW_SCHEMA_V1,
            previous_row_root_sha256: &self.previous_row_root_sha256,
            opportunity_sequence: self.opportunity_sequence,
            opportunity_request_ordinal: self.opportunity_request_ordinal,
            opportunity_event_root_sha256: &self.opportunity_event_root_sha256,
            request_input_tokens: self.request_input_tokens,
            request_observed_at_unix: self.request_observed_at_unix,
            request_event_identity_root_sha256: &self.request_event_identity_root_sha256,
            session_lineage_root_sha256: &self.session_lineage_root_sha256,
            observed_at_unix_ms: self.observed_at_unix_ms,
            classification: &self.classification,
        })
    }
}

#[must_use]
pub fn s1c4_classification_genesis_root_v1() -> String {
    canonical_json_sha256(&("nando.s1c4-classification-genesis.v1", 0_u64))
        .expect("static S1C-4 genesis serializes")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    #[test]
    fn classification_row_is_rooted_and_tamper_evident() {
        let request_root = root('2');
        let request_event_root =
            OpportunityBridgeEventV1::request(request_root.clone(), 42, 1_700_000_000)
                .canonical_sha256()
                .expect("event root");
        let row = S1c4ClassificationRowV1::seal(
            s1c4_classification_genesis_root_v1(),
            7,
            5,
            request_event_root,
            42,
            1_700_000_000,
            request_root,
            root('3'),
            9,
            S1c4TerminalClassificationV1::Censored {
                reason: GroundedDecisionShadowCensorV1::MissingExactGoal,
            },
        )
        .expect("row");
        row.validate().expect("valid row");
        let mut forged = row;
        forged.opportunity_sequence = 8;
        assert_eq!(forged.validate(), Err("s1c4_classification_row_invalid"));
    }

    #[test]
    fn decision_classification_requires_a_valid_precommit_root() {
        assert_eq!(
            S1c4TerminalClassificationV1::DecisionRecorded {
                decision_precommit_root_sha256: String::new(),
            }
            .validate(),
            Err("s1c4_decision_precommit_root_invalid")
        );
    }
}
