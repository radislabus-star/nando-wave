use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use crate::multi_source::{SourceNeutralTopologyMotifEmbeddingV1, SourceNeutralTopologyMotifV1};

use super::{
    K1_NATURAL_EVIDENCE_ROW_SCHEMA_V1, K1_NATURAL_EVIDENCE_ROW_SCHEMA_V2,
    K1_NATURAL_EVIDENCE_ROW_SCHEMA_V3, K1_NATURAL_EVIDENCE_ROW_SCHEMA_V4,
};

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
    #[serde(default)]
    pub capture_generation_root_sha256: String,
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
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub complete_topology_root_sha256: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub motif_root_sha256: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub motif_embedding_manifest_root_sha256: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub motif_embeddings: Vec<SourceNeutralTopologyMotifEmbeddingV1>,
}

#[derive(Serialize)]
struct K1NaturalEvidenceDigestV4<'a> {
    schema: &'static str,
    evidence_root_sha256: &'a str,
    capture_generation_root_sha256: &'a str,
    candidate_structural_root_sha256: &'a str,
    source_neutral_topology_root_sha256: &'a str,
    semantic_novelty_signature_root_sha256: &'a str,
    lineage_root_sha256: &'a str,
    consequence_type: K1ConsequenceTypeV1,
    evidence_class: K1NaturalEvidenceClassV1,
    capture_sequence: u64,
    contract_sequence: u64,
    input_tokens: u64,
    settled: bool,
    verified: bool,
    safety_veto: bool,
    complete_topology_root_sha256: &'a str,
    motif_root_sha256: &'a str,
    motif_embedding_manifest_root_sha256: &'a str,
    motif_embeddings: &'a [SourceNeutralTopologyMotifEmbeddingV1],
}

impl K1NaturalEvidenceRowV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        evidence_root_sha256: String,
        capture_generation_root_sha256: String,
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
        Self::seal_with_schema(
            K1_NATURAL_EVIDENCE_ROW_SCHEMA_V3,
            evidence_root_sha256,
            capture_generation_root_sha256,
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
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn seal_legacy_v2(
        evidence_root_sha256: String,
        capture_generation_root_sha256: String,
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
        Self::seal_with_schema(
            K1_NATURAL_EVIDENCE_ROW_SCHEMA_V2,
            evidence_root_sha256,
            capture_generation_root_sha256,
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
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn seal_with_schema(
        schema: &'static str,
        evidence_root_sha256: String,
        capture_generation_root_sha256: String,
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
            schema: schema.to_owned(),
            row_root_sha256: String::new(),
            evidence_root_sha256,
            capture_generation_root_sha256,
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
            complete_topology_root_sha256: String::new(),
            motif_root_sha256: String::new(),
            motif_embedding_manifest_root_sha256: String::new(),
            motif_embeddings: Vec::new(),
        };
        row.row_root_sha256 = row.expected_root()?;
        row.validate()?;
        Ok(row)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn seal_legacy_v1(
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
            schema: K1_NATURAL_EVIDENCE_ROW_SCHEMA_V1.to_owned(),
            row_root_sha256: String::new(),
            evidence_root_sha256,
            capture_generation_root_sha256: String::new(),
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
            complete_topology_root_sha256: String::new(),
            motif_root_sha256: String::new(),
            motif_embedding_manifest_root_sha256: String::new(),
            motif_embeddings: Vec::new(),
        };
        row.row_root_sha256 = row.expected_root()?;
        row.validate()?;
        Ok(row)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn seal_motif_v4(
        evidence_root_sha256: String,
        capture_generation_root_sha256: String,
        complete_topology_root_sha256: String,
        motif: &SourceNeutralTopologyMotifV1,
        semantic_novelty_signature_root_sha256: String,
        lineage_root_sha256: String,
        consequence_type: K1ConsequenceTypeV1,
        capture_sequence: u64,
        contract_sequence: u64,
        input_tokens: u64,
        settled: bool,
        verified: bool,
        safety_veto: bool,
    ) -> Result<Self, &'static str> {
        motif.validate()?;
        let motif_embedding_manifest_root_sha256 = canonical_json_sha256(&(
            "nando.k1-motif-embedding-manifest.v1",
            motif.motif_root_sha256.as_str(),
            motif
                .embeddings
                .iter()
                .map(|embedding| embedding.embedding_root_sha256.as_str())
                .collect::<Vec<_>>(),
        ))?;
        let mut row = Self {
            schema: K1_NATURAL_EVIDENCE_ROW_SCHEMA_V4.to_owned(),
            row_root_sha256: String::new(),
            evidence_root_sha256,
            capture_generation_root_sha256,
            candidate_structural_root_sha256: motif.motif_root_sha256.clone(),
            source_neutral_topology_root_sha256: motif.motif_root_sha256.clone(),
            semantic_novelty_signature_root_sha256,
            lineage_root_sha256,
            consequence_type,
            evidence_class: K1NaturalEvidenceClassV1::NaturalLive,
            capture_sequence,
            contract_sequence,
            input_tokens,
            settled,
            verified,
            safety_veto,
            complete_topology_root_sha256,
            motif_root_sha256: motif.motif_root_sha256.clone(),
            motif_embedding_manifest_root_sha256,
            motif_embeddings: motif.embeddings.clone(),
        };
        row.row_root_sha256 = row.expected_root()?;
        row.validate()?;
        Ok(row)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let common_roots = [
            self.row_root_sha256.as_str(),
            self.evidence_root_sha256.as_str(),
            self.candidate_structural_root_sha256.as_str(),
            self.source_neutral_topology_root_sha256.as_str(),
            self.semantic_novelty_signature_root_sha256.as_str(),
            self.lineage_root_sha256.as_str(),
        ];
        let legacy_motif_fields_empty = self.complete_topology_root_sha256.is_empty()
            && self.motif_root_sha256.is_empty()
            && self.motif_embedding_manifest_root_sha256.is_empty()
            && self.motif_embeddings.is_empty();
        let schema_valid = match self.schema.as_str() {
            K1_NATURAL_EVIDENCE_ROW_SCHEMA_V1 => {
                self.capture_generation_root_sha256.is_empty() && legacy_motif_fields_empty
            }
            K1_NATURAL_EVIDENCE_ROW_SCHEMA_V2 | K1_NATURAL_EVIDENCE_ROW_SCHEMA_V3 => {
                valid_nonzero_sha256(&self.capture_generation_root_sha256)
                    && legacy_motif_fields_empty
            }
            K1_NATURAL_EVIDENCE_ROW_SCHEMA_V4 => self.validate_motif_fields(),
            _ => false,
        };
        if !schema_valid
            || !common_roots.into_iter().all(valid_nonzero_sha256)
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

    fn validate_motif_fields(&self) -> bool {
        valid_nonzero_sha256(&self.capture_generation_root_sha256)
            && valid_nonzero_sha256(&self.complete_topology_root_sha256)
            && self.motif_root_sha256 == self.candidate_structural_root_sha256
            && self.motif_root_sha256 == self.source_neutral_topology_root_sha256
            && valid_nonzero_sha256(&self.motif_root_sha256)
            && valid_nonzero_sha256(&self.motif_embedding_manifest_root_sha256)
            && !self.motif_embeddings.is_empty()
            && self.motif_embeddings.iter().all(|embedding| {
                embedding.ambient_topology_root_sha256 == self.complete_topology_root_sha256
                    && embedding.validate(&self.motif_root_sha256).is_ok()
            })
            && self
                .motif_embeddings
                .windows(2)
                .all(|pair| pair[0].embedding_root_sha256 < pair[1].embedding_root_sha256)
            && canonical_json_sha256(&(
                "nando.k1-motif-embedding-manifest.v1",
                self.motif_root_sha256.as_str(),
                self.motif_embeddings
                    .iter()
                    .map(|embedding| embedding.embedding_root_sha256.as_str())
                    .collect::<Vec<_>>(),
            ))
            .as_deref()
                == Ok(self.motif_embedding_manifest_root_sha256.as_str())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        if self.schema == K1_NATURAL_EVIDENCE_ROW_SCHEMA_V1 {
            return canonical_json_sha256(&(
                K1_NATURAL_EVIDENCE_ROW_SCHEMA_V1,
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
            ));
        }
        if self.schema == K1_NATURAL_EVIDENCE_ROW_SCHEMA_V4 {
            return canonical_json_sha256(&K1NaturalEvidenceDigestV4 {
                schema: K1_NATURAL_EVIDENCE_ROW_SCHEMA_V4,
                evidence_root_sha256: &self.evidence_root_sha256,
                capture_generation_root_sha256: &self.capture_generation_root_sha256,
                candidate_structural_root_sha256: &self.candidate_structural_root_sha256,
                source_neutral_topology_root_sha256: &self.source_neutral_topology_root_sha256,
                semantic_novelty_signature_root_sha256: &self
                    .semantic_novelty_signature_root_sha256,
                lineage_root_sha256: &self.lineage_root_sha256,
                consequence_type: self.consequence_type,
                evidence_class: self.evidence_class,
                capture_sequence: self.capture_sequence,
                contract_sequence: self.contract_sequence,
                input_tokens: self.input_tokens,
                settled: self.settled,
                verified: self.verified,
                safety_veto: self.safety_veto,
                complete_topology_root_sha256: &self.complete_topology_root_sha256,
                motif_root_sha256: &self.motif_root_sha256,
                motif_embedding_manifest_root_sha256: &self.motif_embedding_manifest_root_sha256,
                motif_embeddings: &self.motif_embeddings,
            });
        }
        canonical_json_sha256(&(
            self.schema.as_str(),
            self.evidence_root_sha256.as_str(),
            self.capture_generation_root_sha256.as_str(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn root(value: u64) -> String {
        format!("{value:064x}")
    }

    #[test]
    fn historical_v1_root_excludes_capture_generation_provenance() {
        let row = K1NaturalEvidenceRowV1::seal_legacy_v1(
            root(1),
            root(2),
            root(3),
            root(4),
            root(5),
            K1ConsequenceTypeV1::Scalar,
            K1NaturalEvidenceClassV1::NaturalLive,
            7,
            9,
            11,
            true,
            true,
            true,
        )
        .expect("legacy row");
        assert_eq!(
            row.row_root_sha256,
            "48a51dd4da6c70cf71293b5cf813e5ce068400ffa1711f0cce109c47d6437251"
        );

        let mut encoded = serde_json::to_value(&row).expect("encode legacy row");
        encoded
            .as_object_mut()
            .expect("row object")
            .remove("capture_generation_root_sha256");
        let decoded: K1NaturalEvidenceRowV1 =
            serde_json::from_value(encoded).expect("decode historical row");
        decoded.validate().expect("validate historical row");
        assert_eq!(decoded.row_root_sha256, row.row_root_sha256);
    }

    #[test]
    fn historical_v2_root_remains_decodable_and_stable() {
        let row = K1NaturalEvidenceRowV1::seal_legacy_v2(
            root(1),
            root(2),
            root(3),
            root(4),
            root(5),
            root(6),
            K1ConsequenceTypeV1::Scalar,
            K1NaturalEvidenceClassV1::NaturalLive,
            7,
            9,
            11,
            true,
            true,
            false,
        )
        .expect("v2 row");
        row.validate().expect("valid v2 row");
        assert_eq!(
            row.row_root_sha256,
            "cd140fc6e5b52a193fc91d89776cca46c777b7dd6509c836265b3734bf3358e0"
        );
        let bytes = serde_json::to_vec(&row).expect("encode v2 row");
        let restored: K1NaturalEvidenceRowV1 =
            serde_json::from_slice(&bytes).expect("decode v2 row");
        restored.validate().expect("validate v2 row");
        assert_eq!(serde_json::to_vec(&restored).expect("re-encode v2"), bytes);
    }

    #[test]
    fn historical_v3_root_remains_stable() {
        let row = K1NaturalEvidenceRowV1::seal(
            root(1),
            root(2),
            root(3),
            root(4),
            root(5),
            root(6),
            K1ConsequenceTypeV1::Scalar,
            K1NaturalEvidenceClassV1::NaturalLive,
            7,
            9,
            11,
            true,
            true,
            false,
        )
        .expect("v3 row");
        assert_eq!(
            row.row_root_sha256,
            "93032aaa89448433cff94f43e134c915535024c83adfbf89efb56e8765af07e3"
        );
        row.validate().expect("valid v3 row");
    }
}
