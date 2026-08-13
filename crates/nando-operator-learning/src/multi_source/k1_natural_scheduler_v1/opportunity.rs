use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    ResponseOperation, ResponseProgram, canonical_json_sha256,
    response_program_version_root_sha256, valid_nonzero_sha256,
};
use serde::{Deserialize, Serialize};

use crate::multi_source::NaturalT1ProgramArtifactV1;

pub const IDENTIFIER_RESOURCE_LIMITS_SCHEMA_V1: &str = "nando.identifier-resource-limits.v1";
pub const IDENTIFIER_SUPPORT_ROW_SCHEMA_V1: &str = "nando.identifier-support-row.v1";
pub const IDENTIFIER_SUPPORT_MANIFEST_SCHEMA_V1: &str = "nando.identifier-support-manifest.v1";
pub const EVIDENCE_SOURCE_SNAPSHOT_SCHEMA_V1: &str = "nando.evidence-source-snapshot.v1";
pub const RELEVANT_IDENTIFIER_ARTIFACT_PROJECTION_SCHEMA_V1: &str =
    "nando.relevant-identifier-artifact-projection.v1";
pub const IDENTIFIER_CAUSAL_INPUT_MANIFEST_SCHEMA_V1: &str =
    "nando.identifier-causal-input-manifest.v1";
pub const OPPORTUNITY_ROOT_SCHEMA_V1: &str = "nando.k1-exact-experiment-opportunity.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentifierResourceLimitsV1 {
    pub maximum_support_rows: u64,
    pub maximum_seed_programs: u64,
    pub maximum_semantic_classes: u64,
    pub raw_phase_cells: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentifierSupportRowV1 {
    pub schema: String,
    pub row_root_sha256: String,
    pub capture_sequence: u64,
    pub join_root_sha256: String,
    pub completed_frame_root_sha256: String,
    pub topology_root_sha256: String,
    pub capture_generation_root_sha256: String,
    pub motif_root_sha256: String,
    pub embedding_roots_sha256: Vec<String>,
    pub session_lineage_root_sha256: String,
    pub turn_intent_id_sha256: String,
    pub session_id_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentifierSupportManifestV1 {
    pub schema: String,
    pub manifest_root_sha256: String,
    pub candidate_structural_root_sha256: String,
    pub support_watermark: u64,
    pub rows: Vec<IdentifierSupportRowV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceSourceSnapshotV1 {
    pub schema: String,
    pub snapshot_root_sha256: String,
    pub topology_prefix_rows: u64,
    pub topology_prefix_root_sha256: String,
    pub frame_prefix_rows: u64,
    pub frame_prefix_root_sha256: String,
    pub join_builder_schema: String,
    pub motif_archive_root_sha256: String,
    pub motif_config_root_sha256: String,
    pub collection_checkpoint_root_sha256: String,
    pub active_protocol_mode_set_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RelevantIdentifierArtifactProjectionV1 {
    pub schema: String,
    pub projection_root_sha256: String,
    pub requested_support_identity_root_sha256: String,
    pub artifact_roots_sha256: Vec<String>,
    pub programs: BTreeMap<String, ResponseProgram>,
    pub predicted_typed_consequence_roots_sha256: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentifierCausalInputManifestV1 {
    pub schema: String,
    pub manifest_root_sha256: String,
    pub opportunity_root_sha256: String,
    pub candidate_structural_root_sha256: String,
    pub support_manifest_root_sha256: String,
    pub support_rows: u64,
    pub relevant_artifact_projection_root_sha256: String,
    pub candidate_generator_schema: String,
    pub discovery_basis_root_sha256: String,
    pub active_protocol_mode_set_root_sha256: String,
    pub resource_limits: IdentifierResourceLimitsV1,
    pub resource_limits_root_sha256: String,
}

impl IdentifierResourceLimitsV1 {
    pub fn seal(
        maximum_support_rows: u64,
        maximum_seed_programs: u64,
        maximum_semantic_classes: u64,
        raw_phase_cells: u64,
    ) -> Result<Self, &'static str> {
        let limits = Self {
            maximum_support_rows,
            maximum_seed_programs,
            maximum_semantic_classes,
            raw_phase_cells,
        };
        limits.validate()?;
        Ok(limits)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.maximum_support_rows == 0
            || self.maximum_seed_programs == 0
            || self.maximum_semantic_classes == 0
            || self.raw_phase_cells == 0
        {
            return Err("identifier_resource_limits_invalid");
        }
        Ok(())
    }

    pub fn root(&self) -> Result<String, &'static str> {
        self.validate()?;
        canonical_json_sha256(&(IDENTIFIER_RESOURCE_LIMITS_SCHEMA_V1, self))
    }
}

impl IdentifierSupportRowV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        capture_sequence: u64,
        join_root_sha256: String,
        completed_frame_root_sha256: String,
        topology_root_sha256: String,
        capture_generation_root_sha256: String,
        motif_root_sha256: String,
        mut embedding_roots_sha256: Vec<String>,
        session_lineage_root_sha256: String,
        turn_intent_id_sha256: String,
        session_id_sha256: String,
    ) -> Result<Self, &'static str> {
        embedding_roots_sha256.sort();
        embedding_roots_sha256.dedup();
        let mut row = Self {
            schema: IDENTIFIER_SUPPORT_ROW_SCHEMA_V1.to_owned(),
            row_root_sha256: String::new(),
            capture_sequence,
            join_root_sha256,
            completed_frame_root_sha256,
            topology_root_sha256,
            capture_generation_root_sha256,
            motif_root_sha256,
            embedding_roots_sha256,
            session_lineage_root_sha256,
            turn_intent_id_sha256,
            session_id_sha256,
        };
        row.row_root_sha256 = row.expected_root()?;
        row.validate()?;
        Ok(row)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != IDENTIFIER_SUPPORT_ROW_SCHEMA_V1
            || self.capture_sequence == 0
            || [
                self.row_root_sha256.as_str(),
                self.join_root_sha256.as_str(),
                self.completed_frame_root_sha256.as_str(),
                self.topology_root_sha256.as_str(),
                self.capture_generation_root_sha256.as_str(),
                self.motif_root_sha256.as_str(),
                self.session_lineage_root_sha256.as_str(),
                self.turn_intent_id_sha256.as_str(),
                self.session_id_sha256.as_str(),
            ]
            .into_iter()
            .any(|root| !valid_nonzero_sha256(root))
            || self.embedding_roots_sha256.is_empty()
            || !strict_roots(&self.embedding_roots_sha256)
            || self.row_root_sha256 != self.expected_root()?
        {
            return Err("identifier_support_row_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            IDENTIFIER_SUPPORT_ROW_SCHEMA_V1,
            self.capture_sequence,
            self.join_root_sha256.as_str(),
            self.completed_frame_root_sha256.as_str(),
            self.topology_root_sha256.as_str(),
            self.capture_generation_root_sha256.as_str(),
            self.motif_root_sha256.as_str(),
            &self.embedding_roots_sha256,
            self.session_lineage_root_sha256.as_str(),
            self.turn_intent_id_sha256.as_str(),
            self.session_id_sha256.as_str(),
        ))
    }
}

pub fn build_identifier_support_manifest_v1(
    candidate_structural_root_sha256: String,
    support_watermark: u64,
    mut rows: Vec<IdentifierSupportRowV1>,
    maximum_support_rows: u64,
) -> Result<IdentifierSupportManifestV1, &'static str> {
    rows.sort_by(|left, right| {
        (
            left.capture_sequence,
            left.join_root_sha256.as_str(),
            left.motif_root_sha256.as_str(),
        )
            .cmp(&(
                right.capture_sequence,
                right.join_root_sha256.as_str(),
                right.motif_root_sha256.as_str(),
            ))
    });
    if !valid_nonzero_sha256(&candidate_structural_root_sha256)
        || support_watermark == 0
        || rows.is_empty()
        || u64::try_from(rows.len()).map_err(|_| "identifier_support_count")? > maximum_support_rows
        || rows.iter().any(|row| {
            row.validate().is_err()
                || row.capture_sequence > support_watermark
                || row.motif_root_sha256 != candidate_structural_root_sha256
        })
        || rows
            .windows(2)
            .any(|pair| pair[0].join_root_sha256 == pair[1].join_root_sha256)
    {
        return Err("identifier_support_manifest_invalid");
    }
    let manifest_root_sha256 = canonical_json_sha256(&(
        IDENTIFIER_SUPPORT_MANIFEST_SCHEMA_V1,
        candidate_structural_root_sha256.as_str(),
        support_watermark,
        rows.iter()
            .map(|row| row.row_root_sha256.as_str())
            .collect::<Vec<_>>(),
    ))?;
    Ok(IdentifierSupportManifestV1 {
        schema: IDENTIFIER_SUPPORT_MANIFEST_SCHEMA_V1.to_owned(),
        manifest_root_sha256,
        candidate_structural_root_sha256,
        support_watermark,
        rows,
    })
}

impl IdentifierSupportManifestV1 {
    pub fn validate(&self, maximum_support_rows: u64) -> Result<(), &'static str> {
        let rebuilt = build_identifier_support_manifest_v1(
            self.candidate_structural_root_sha256.clone(),
            self.support_watermark,
            self.rows.clone(),
            maximum_support_rows,
        )?;
        if &rebuilt != self {
            return Err("identifier_support_manifest_invalid");
        }
        Ok(())
    }
}

impl EvidenceSourceSnapshotV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        topology_prefix_rows: u64,
        topology_prefix_root_sha256: String,
        frame_prefix_rows: u64,
        frame_prefix_root_sha256: String,
        join_builder_schema: String,
        motif_archive_root_sha256: String,
        motif_config_root_sha256: String,
        collection_checkpoint_root_sha256: String,
        active_protocol_mode_set_root_sha256: String,
    ) -> Result<Self, &'static str> {
        let mut snapshot = Self {
            schema: EVIDENCE_SOURCE_SNAPSHOT_SCHEMA_V1.to_owned(),
            snapshot_root_sha256: String::new(),
            topology_prefix_rows,
            topology_prefix_root_sha256,
            frame_prefix_rows,
            frame_prefix_root_sha256,
            join_builder_schema,
            motif_archive_root_sha256,
            motif_config_root_sha256,
            collection_checkpoint_root_sha256,
            active_protocol_mode_set_root_sha256,
        };
        snapshot.snapshot_root_sha256 = snapshot.expected_root()?;
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != EVIDENCE_SOURCE_SNAPSHOT_SCHEMA_V1
            || self.topology_prefix_rows == 0
            || self.frame_prefix_rows == 0
            || self.join_builder_schema.is_empty()
            || [
                self.snapshot_root_sha256.as_str(),
                self.topology_prefix_root_sha256.as_str(),
                self.frame_prefix_root_sha256.as_str(),
                self.motif_archive_root_sha256.as_str(),
                self.motif_config_root_sha256.as_str(),
                self.collection_checkpoint_root_sha256.as_str(),
                self.active_protocol_mode_set_root_sha256.as_str(),
            ]
            .into_iter()
            .any(|root| !valid_nonzero_sha256(root))
            || self.snapshot_root_sha256 != self.expected_root()?
        {
            return Err("evidence_source_snapshot_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            EVIDENCE_SOURCE_SNAPSHOT_SCHEMA_V1,
            self.topology_prefix_rows,
            self.topology_prefix_root_sha256.as_str(),
            self.frame_prefix_rows,
            self.frame_prefix_root_sha256.as_str(),
            self.join_builder_schema.as_str(),
            self.motif_archive_root_sha256.as_str(),
            self.motif_config_root_sha256.as_str(),
            self.collection_checkpoint_root_sha256.as_str(),
            self.active_protocol_mode_set_root_sha256.as_str(),
        ))
    }
}

pub fn build_relevant_identifier_artifact_projection_v1(
    support: &IdentifierSupportManifestV1,
    artifacts: &[NaturalT1ProgramArtifactV1],
) -> Result<RelevantIdentifierArtifactProjectionV1, &'static str> {
    let identities = support
        .rows
        .iter()
        .map(|row| {
            (
                row.turn_intent_id_sha256.as_str(),
                row.session_id_sha256.as_str(),
            )
        })
        .collect::<BTreeSet<_>>();
    let requested_support_identity_root_sha256 = canonical_json_sha256(&(
        "nando.identifier-support-identities.v1",
        identities.iter().copied().collect::<Vec<_>>(),
    ))?;
    let mut artifact_roots_sha256 = Vec::new();
    let mut programs = BTreeMap::new();
    let mut predicted_typed_consequence_roots_sha256 = BTreeMap::new();
    for artifact in artifacts {
        artifact.validate()?;
        if !identities.contains(&(
            artifact.turn_intent_id_sha256.as_str(),
            artifact.session_id_sha256.as_str(),
        )) {
            continue;
        }
        let mut contributed = false;
        for root in &artifact.hypothesis_program_roots_sha256 {
            let program = artifact
                .programs
                .get(root)
                .ok_or("relevant_identifier_artifact_program_missing")?;
            if !matches!(
                program.operation,
                ResponseOperation::ComposeCollection { .. }
            ) {
                continue;
            }
            match programs.insert(root.clone(), program.clone()) {
                Some(existing) if existing != *program => {
                    return Err("relevant_identifier_artifact_program_conflict");
                }
                _ => {}
            }
            if let Some(prediction) = artifact.predicted_typed_consequence_roots_sha256.get(root) {
                match predicted_typed_consequence_roots_sha256
                    .insert(root.clone(), prediction.clone())
                {
                    Some(existing) if existing != *prediction => {
                        return Err("relevant_identifier_artifact_prediction_conflict");
                    }
                    _ => {}
                }
            }
            contributed = true;
        }
        if contributed {
            artifact_roots_sha256.push(artifact.artifact_root_sha256.clone());
        }
    }
    artifact_roots_sha256.sort();
    artifact_roots_sha256.dedup();
    if programs.iter().any(|(root, program)| {
        response_program_version_root_sha256(program).as_deref() != Ok(root.as_str())
    }) {
        return Err("relevant_identifier_artifact_program_invalid");
    }
    let projection_root_sha256 = canonical_json_sha256(&(
        RELEVANT_IDENTIFIER_ARTIFACT_PROJECTION_SCHEMA_V1,
        requested_support_identity_root_sha256.as_str(),
        &artifact_roots_sha256,
        &programs,
        &predicted_typed_consequence_roots_sha256,
    ))?;
    Ok(RelevantIdentifierArtifactProjectionV1 {
        schema: RELEVANT_IDENTIFIER_ARTIFACT_PROJECTION_SCHEMA_V1.to_owned(),
        projection_root_sha256,
        requested_support_identity_root_sha256,
        artifact_roots_sha256,
        programs,
        predicted_typed_consequence_roots_sha256,
    })
}

impl RelevantIdentifierArtifactProjectionV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != RELEVANT_IDENTIFIER_ARTIFACT_PROJECTION_SCHEMA_V1
            || !valid_nonzero_sha256(&self.requested_support_identity_root_sha256)
            || !strict_roots_or_empty(&self.artifact_roots_sha256)
            || self.programs.iter().any(|(root, program)| {
                response_program_version_root_sha256(program).as_deref() != Ok(root.as_str())
            })
            || self
                .predicted_typed_consequence_roots_sha256
                .iter()
                .any(|(root, value)| {
                    !self.programs.contains_key(root) || !valid_nonzero_sha256(value)
                })
            || self.projection_root_sha256
                != canonical_json_sha256(&(
                    RELEVANT_IDENTIFIER_ARTIFACT_PROJECTION_SCHEMA_V1,
                    self.requested_support_identity_root_sha256.as_str(),
                    &self.artifact_roots_sha256,
                    &self.programs,
                    &self.predicted_typed_consequence_roots_sha256,
                ))?
        {
            return Err("relevant_identifier_artifact_projection_invalid");
        }
        Ok(())
    }
}

pub fn build_identifier_causal_input_manifest_v1(
    support: &IdentifierSupportManifestV1,
    relevant_artifacts: &RelevantIdentifierArtifactProjectionV1,
    candidate_generator_schema: String,
    discovery_basis_root_sha256: String,
    active_protocol_mode_set_root_sha256: String,
    resource_limits: IdentifierResourceLimitsV1,
) -> Result<IdentifierCausalInputManifestV1, &'static str> {
    resource_limits.validate()?;
    support.validate(resource_limits.maximum_support_rows)?;
    relevant_artifacts.validate()?;
    if candidate_generator_schema.is_empty()
        || !valid_nonzero_sha256(&discovery_basis_root_sha256)
        || !valid_nonzero_sha256(&active_protocol_mode_set_root_sha256)
    {
        return Err("identifier_causal_input_manifest_invalid");
    }
    let support_rows = u64::try_from(support.rows.len()).map_err(|_| "identifier_support_count")?;
    let resource_limits_root_sha256 = resource_limits.root()?;
    let manifest_root_sha256 = canonical_json_sha256(&(
        IDENTIFIER_CAUSAL_INPUT_MANIFEST_SCHEMA_V1,
        support.candidate_structural_root_sha256.as_str(),
        support.manifest_root_sha256.as_str(),
        support_rows,
        relevant_artifacts.projection_root_sha256.as_str(),
        candidate_generator_schema.as_str(),
        discovery_basis_root_sha256.as_str(),
        active_protocol_mode_set_root_sha256.as_str(),
        resource_limits,
        resource_limits_root_sha256.as_str(),
    ))?;
    let opportunity_root_sha256 =
        canonical_json_sha256(&(OPPORTUNITY_ROOT_SCHEMA_V1, manifest_root_sha256.as_str()))?;
    Ok(IdentifierCausalInputManifestV1 {
        schema: IDENTIFIER_CAUSAL_INPUT_MANIFEST_SCHEMA_V1.to_owned(),
        manifest_root_sha256,
        opportunity_root_sha256,
        candidate_structural_root_sha256: support.candidate_structural_root_sha256.clone(),
        support_manifest_root_sha256: support.manifest_root_sha256.clone(),
        support_rows,
        relevant_artifact_projection_root_sha256: relevant_artifacts.projection_root_sha256.clone(),
        candidate_generator_schema,
        discovery_basis_root_sha256,
        active_protocol_mode_set_root_sha256,
        resource_limits,
        resource_limits_root_sha256,
    })
}

impl IdentifierCausalInputManifestV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.resource_limits.validate()?;
        if self.schema != IDENTIFIER_CAUSAL_INPUT_MANIFEST_SCHEMA_V1
            || self.support_rows == 0
            || self.support_rows > self.resource_limits.maximum_support_rows
            || self.candidate_generator_schema.is_empty()
            || [
                self.manifest_root_sha256.as_str(),
                self.opportunity_root_sha256.as_str(),
                self.candidate_structural_root_sha256.as_str(),
                self.support_manifest_root_sha256.as_str(),
                self.relevant_artifact_projection_root_sha256.as_str(),
                self.discovery_basis_root_sha256.as_str(),
                self.active_protocol_mode_set_root_sha256.as_str(),
                self.resource_limits_root_sha256.as_str(),
            ]
            .into_iter()
            .any(|root| !valid_nonzero_sha256(root))
            || self.resource_limits_root_sha256 != self.resource_limits.root()?
            || self.manifest_root_sha256
                != canonical_json_sha256(&(
                    IDENTIFIER_CAUSAL_INPUT_MANIFEST_SCHEMA_V1,
                    self.candidate_structural_root_sha256.as_str(),
                    self.support_manifest_root_sha256.as_str(),
                    self.support_rows,
                    self.relevant_artifact_projection_root_sha256.as_str(),
                    self.candidate_generator_schema.as_str(),
                    self.discovery_basis_root_sha256.as_str(),
                    self.active_protocol_mode_set_root_sha256.as_str(),
                    self.resource_limits,
                    self.resource_limits_root_sha256.as_str(),
                ))?
            || self.opportunity_root_sha256
                != canonical_json_sha256(&(
                    OPPORTUNITY_ROOT_SCHEMA_V1,
                    self.manifest_root_sha256.as_str(),
                ))?
        {
            return Err("identifier_causal_input_manifest_invalid");
        }
        Ok(())
    }
}

fn strict_roots(roots: &[String]) -> bool {
    !roots.is_empty() && strict_roots_or_empty(roots)
}

fn strict_roots_or_empty(roots: &[String]) -> bool {
    roots.iter().all(|root| valid_nonzero_sha256(root))
        && roots.windows(2).all(|pair| pair[0] < pair[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(value: u64) -> String {
        format!("{value:064x}")
    }

    fn support() -> IdentifierSupportManifestV1 {
        build_identifier_support_manifest_v1(
            root(9),
            2,
            vec![
                IdentifierSupportRowV1::seal(
                    2,
                    root(2),
                    root(12),
                    root(22),
                    root(32),
                    root(9),
                    vec![root(42)],
                    root(52),
                    root(62),
                    root(72),
                )
                .expect("row 2"),
                IdentifierSupportRowV1::seal(
                    1,
                    root(1),
                    root(11),
                    root(21),
                    root(31),
                    root(9),
                    vec![root(41)],
                    root(51),
                    root(61),
                    root(71),
                )
                .expect("row 1"),
            ],
            64,
        )
        .expect("support")
    }

    #[test]
    fn support_manifest_is_canonical_and_exact() {
        let left = support();
        let right = build_identifier_support_manifest_v1(
            left.candidate_structural_root_sha256.clone(),
            left.support_watermark,
            left.rows.iter().cloned().rev().collect(),
            64,
        )
        .expect("reordered");
        assert_eq!(left, right);
        let mut changed = left.rows.clone();
        changed[0].completed_frame_root_sha256 = root(99);
        changed[0].row_root_sha256 = changed[0].expected_root().expect("row root");
        let changed =
            build_identifier_support_manifest_v1(root(9), 2, changed, 64).expect("changed support");
        assert_ne!(left.manifest_root_sha256, changed.manifest_root_sha256);
    }

    #[test]
    fn empty_relevant_projection_is_rooted_and_stable() {
        let support = support();
        let projection =
            build_relevant_identifier_artifact_projection_v1(&support, &[]).expect("empty");
        projection.validate().expect("valid empty projection");
        assert!(projection.programs.is_empty());
        assert!(valid_nonzero_sha256(&projection.projection_root_sha256));
    }

    #[test]
    fn opportunity_excludes_receipt_metadata_and_changes_on_causal_input() {
        let support = support();
        let artifacts =
            build_relevant_identifier_artifact_projection_v1(&support, &[]).expect("projection");
        let limits = IdentifierResourceLimitsV1::seal(64, 4096, 4096, 16).expect("limits");
        let first = build_identifier_causal_input_manifest_v1(
            &support,
            &artifacts,
            "generator-v4".to_owned(),
            root(81),
            root(82),
            limits,
        )
        .expect("first");
        first.validate().expect("valid first");
        // There is intentionally no generation, timestamp, queue, score, cost,
        // token estimate, or future deadline parameter in this pure builder.
        let repeated = build_identifier_causal_input_manifest_v1(
            &support,
            &artifacts,
            "generator-v4".to_owned(),
            root(81),
            root(82),
            limits,
        )
        .expect("repeated");
        assert_eq!(first, repeated);

        let changed = build_identifier_causal_input_manifest_v1(
            &support,
            &artifacts,
            "generator-v4".to_owned(),
            root(81),
            root(83),
            limits,
        )
        .expect("changed mode set");
        assert_ne!(
            first.opportunity_root_sha256,
            changed.opportunity_root_sha256
        );
    }
}
