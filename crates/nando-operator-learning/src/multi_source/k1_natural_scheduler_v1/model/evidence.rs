use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K1ConsequenceTypeV1 {
    Scalar,
    Record,
    Collection,
    Boolean,
    RenderedSequence,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K1NaturalEvidenceClassV1 {
    NaturalLive,
    Controlled,
    GeneratedMs5,
    GeneratedMs6,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1NaturalEvidenceRowV1 {
    pub schema: String,
    pub row_root_sha256: String,
    pub evidence_root_sha256: String,
    pub candidate_structural_root_sha256: String,
    pub source_neutral_topology_root_sha256: String,
    pub semantic_novelty_signature_root_sha256: String,
    pub lineage_root_sha256: String,
    pub consequence_type: K1ConsequenceTypeV1,
    pub evidence_class: K1NaturalEvidenceClassV1,
    pub capture_sequence: u64,
    pub contract_sequence: u64,
    pub input_tokens: u64,
    pub settled: bool,
    pub verified: bool,
    pub safety_veto: bool,
}

impl K1NaturalEvidenceRowV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        evidence_root_sha256: String,
        candidate_structural_root_sha256: String,
        source_neutral_topology_root_sha256: String,
        semantic_novelty_signature_root_sha256: String,
        lineage_root_sha256: String,
        consequence_type: K1ConsequenceTypeV1,
        evidence_class: K1NaturalEvidenceClassV1,
        capture_sequence: u64,
        contract_sequence: u64,
        input_tokens: u64,
        settled: bool,
        verified: bool,
        safety_veto: bool,
    ) -> Result<Self, &'static str> {
        let mut row = Self {
            schema: "nando.k1-natural-evidence-row.v1".to_owned(),
            row_root_sha256: String::new(),
            evidence_root_sha256,
            candidate_structural_root_sha256,
            source_neutral_topology_root_sha256,
            semantic_novelty_signature_root_sha256,
            lineage_root_sha256,
            consequence_type,
            evidence_class,
            capture_sequence,
            contract_sequence,
            input_tokens,
            settled,
            verified,
            safety_veto,
        };
        row.row_root_sha256 = row.expected_root()?;
        row.validate()?;
        Ok(row)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let roots = [
            self.row_root_sha256.as_str(),
            self.evidence_root_sha256.as_str(),
            self.candidate_structural_root_sha256.as_str(),
            self.source_neutral_topology_root_sha256.as_str(),
            self.semantic_novelty_signature_root_sha256.as_str(),
            self.lineage_root_sha256.as_str(),
        ];
        if self.schema != "nando.k1-natural-evidence-row.v1"
            || !roots.into_iter().all(valid_nonzero_sha256)
            || self.capture_sequence == 0
            || self.contract_sequence < self.capture_sequence
            || self.input_tokens == 0
            || (self.verified && !self.settled)
            || self.row_root_sha256 != self.expected_root()?
        {
            return Err("k1_natural_evidence_row_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            self.schema.as_str(),
            self.evidence_root_sha256.as_str(),
            self.candidate_structural_root_sha256.as_str(),
            self.source_neutral_topology_root_sha256.as_str(),
            self.semantic_novelty_signature_root_sha256.as_str(),
            self.lineage_root_sha256.as_str(),
            self.consequence_type,
            self.evidence_class,
            self.capture_sequence,
            self.contract_sequence,
            self.input_tokens,
            self.settled,
            self.verified,
            self.safety_veto,
        ))
    }
}
