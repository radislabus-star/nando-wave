use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::K1_MOTIF_SOURCE_DISPOSITION_SCHEMA_V1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K1MotifSourceDispositionClassV1 {
    MotifRetained,
    MotifSupportOverflow,
    CensoredEmptyOrIncompleteTopology,
    CensoredMotifEnumerationBudget,
    CensoredInvalidEmbedding,
    FixtureOrControlledExcluded,
    SafetyVeto,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1MotifSourceDispositionV1 {
    pub schema: String,
    pub disposition_root_sha256: String,
    pub evidence_root_sha256: String,
    pub complete_topology_root_sha256: String,
    pub capture_sequence: u64,
    pub class: K1MotifSourceDispositionClassV1,
    pub enumerated_occurrences: u64,
    pub retained_occurrences: u64,
    pub overflow_occurrences: u64,
    pub occurrence_manifest_root_sha256: String,
}

#[derive(Serialize)]
struct SourceDispositionDigestV1<'a> {
    schema: &'static str,
    evidence_root_sha256: &'a str,
    complete_topology_root_sha256: &'a str,
    capture_sequence: u64,
    class: K1MotifSourceDispositionClassV1,
    enumerated_occurrences: u64,
    retained_occurrences: u64,
    overflow_occurrences: u64,
    occurrence_manifest_root_sha256: &'a str,
}

impl K1MotifSourceDispositionV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        evidence_root_sha256: String,
        complete_topology_root_sha256: String,
        capture_sequence: u64,
        class: K1MotifSourceDispositionClassV1,
        enumerated_occurrences: u64,
        retained_occurrences: u64,
        overflow_occurrences: u64,
        occurrence_manifest_root_sha256: String,
    ) -> Result<Self, &'static str> {
        let mut receipt = Self {
            schema: K1_MOTIF_SOURCE_DISPOSITION_SCHEMA_V1.to_owned(),
            disposition_root_sha256: String::new(),
            evidence_root_sha256,
            complete_topology_root_sha256,
            capture_sequence,
            class,
            enumerated_occurrences,
            retained_occurrences,
            overflow_occurrences,
            occurrence_manifest_root_sha256,
        };
        receipt.disposition_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let counts_valid = match self.class {
            K1MotifSourceDispositionClassV1::MotifRetained => {
                self.enumerated_occurrences > 0
                    && self.retained_occurrences > 0
                    && self
                        .retained_occurrences
                        .saturating_add(self.overflow_occurrences)
                        == self.enumerated_occurrences
            }
            K1MotifSourceDispositionClassV1::MotifSupportOverflow => {
                self.enumerated_occurrences > 0
                    && self.retained_occurrences == 0
                    && self.overflow_occurrences == self.enumerated_occurrences
            }
            K1MotifSourceDispositionClassV1::CensoredEmptyOrIncompleteTopology
            | K1MotifSourceDispositionClassV1::CensoredMotifEnumerationBudget
            | K1MotifSourceDispositionClassV1::CensoredInvalidEmbedding
            | K1MotifSourceDispositionClassV1::FixtureOrControlledExcluded
            | K1MotifSourceDispositionClassV1::SafetyVeto => {
                self.enumerated_occurrences == 0
                    && self.retained_occurrences == 0
                    && self.overflow_occurrences == 0
            }
        };
        if self.schema != K1_MOTIF_SOURCE_DISPOSITION_SCHEMA_V1
            || ![
                self.disposition_root_sha256.as_str(),
                self.evidence_root_sha256.as_str(),
                self.complete_topology_root_sha256.as_str(),
                self.occurrence_manifest_root_sha256.as_str(),
            ]
            .into_iter()
            .all(valid_nonzero_sha256)
            || self.capture_sequence == 0
            || !counts_valid
            || self.disposition_root_sha256 != self.expected_root()?
        {
            return Err("k1_motif_source_disposition_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&SourceDispositionDigestV1 {
            schema: K1_MOTIF_SOURCE_DISPOSITION_SCHEMA_V1,
            evidence_root_sha256: &self.evidence_root_sha256,
            complete_topology_root_sha256: &self.complete_topology_root_sha256,
            capture_sequence: self.capture_sequence,
            class: self.class,
            enumerated_occurrences: self.enumerated_occurrences,
            retained_occurrences: self.retained_occurrences,
            overflow_occurrences: self.overflow_occurrences,
            occurrence_manifest_root_sha256: &self.occurrence_manifest_root_sha256,
        })
    }
}
