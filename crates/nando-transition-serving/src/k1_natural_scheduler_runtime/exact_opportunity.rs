use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use nando_operator_learning::multi_source::{
    CompletedEffectFormV1, EvidenceSourceSnapshotV1, ExactAttemptIndexV1,
    IdentifierCausalInputManifestV1, IdentifierResourceLimitsV1, IdentifierSupportManifestV1,
    IdentifierSupportRowV1, K1DeficitSnapshotV1, K1NaturalCandidateFreezeV1,
    K1NaturalCandidateQueueV1, K1NaturalCohortCandidateV1, MultiSourceT1IdentificationV3,
    NaturalT1ProgramArtifactV1, ProgramDispositionSetV1, RelevantIdentifierArtifactProjectionV1,
    build_identifier_causal_input_manifest_v1, build_identifier_support_manifest_v1,
    build_relevant_identifier_artifact_projection_v1, evaluate_program_dispositions_v1,
    natural_t1_discovery_basis_root_v4, source_neutral_topology_motif_config_root_v1,
};
use nando_response_actor::OnlineCollectionMiner;
use serde::{Deserialize, Serialize};

use super::{PreparedK1TickContextV1, generation_budget};
use crate::multi_source_frame_archive::MultiSourceFrameArchiveReadSnapshot;
use crate::multi_source_topology_archive::MultiSourceTopologyArchiveReadSnapshot;
use crate::operator_certification::CertificationAuthorityConfigV1;

const EXACT_JOIN_BUILDER_SCHEMA_V1: &str = "nando.multi-source-blind-then-reveal-join-builder.v1";
const EXACT_IDENTIFIER_MAX_SEED_PROGRAMS_V1: u64 = 4_096;
const EXACT_IDENTIFIER_MAX_SEMANTIC_CLASSES_V1: u64 = 4_096;
const EXACT_IDENTIFIER_RAW_PHASE_CELLS_V1: u64 = 16;
const EXACT_ARCHIVE_OBJECT_SCHEMA_V1: &str = "nando.k1-exact-identifier-archive-object.v1";
const EXACT_ARCHIVE_MANIFEST_SCHEMA_V1: &str = "nando.k1-identifier-artifact-archive-manifest.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExactIdentifierArchiveObjectV1 {
    schema: String,
    source_snapshot: EvidenceSourceSnapshotV1,
    support_manifest: IdentifierSupportManifestV1,
    relevant_projection: RelevantIdentifierArtifactProjectionV1,
    relevant_artifacts: Vec<NaturalT1ProgramArtifactV1>,
    causal_manifest: IdentifierCausalInputManifestV1,
    active_protocol_mode_roots_sha256: BTreeSet<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExactIdentifierArchiveManifestV1 {
    schema: String,
    manifest_root_sha256: String,
    object_root_sha256: String,
    source_snapshot_root_sha256: String,
    support_manifest_root_sha256: String,
    relevant_projection_root_sha256: String,
    causal_manifest_root_sha256: String,
}

impl ExactIdentifierArchiveManifestV1 {
    pub(crate) fn manifest_root_sha256(&self) -> &str {
        &self.manifest_root_sha256
    }

    pub(crate) fn object_root_sha256(&self) -> &str {
        &self.object_root_sha256
    }
}

pub(crate) struct RestoredExactIdentifierInputsV1 {
    prepared: PreparedK1TickContextV1,
    frames: Vec<nando_operator_kernel::RelationFrame>,
    pub(crate) artifacts: Vec<NaturalT1ProgramArtifactV1>,
    active_protocols: BTreeSet<String>,
    pub(crate) support: IdentifierSupportManifestV1,
    pub(crate) projection: RelevantIdentifierArtifactProjectionV1,
    causal: IdentifierCausalInputManifestV1,
}

pub(crate) struct ExactInitialIdentifierEvaluationV1 {
    pub(crate) report: MultiSourceT1IdentificationV3,
    pub(crate) dispositions: ProgramDispositionSetV1,
}

pub(crate) struct RestoredExactOpportunityV1 {
    pub(crate) source_heads: ExactDurableSourceHeadsV1,
    pub(crate) source_snapshot: EvidenceSourceSnapshotV1,
    pub(crate) catalog: nando_operator_learning::multi_source::ValidatedK1NaturalCohortCatalogV1,
    pub(crate) queue: K1NaturalCandidateQueueV1,
    pub(crate) support_manifests_by_candidate: BTreeMap<String, IdentifierSupportManifestV1>,
    pub(crate) artifact_projections_by_candidate:
        BTreeMap<String, RelevantIdentifierArtifactProjectionV1>,
    pub(crate) causal_manifests_by_candidate: BTreeMap<String, IdentifierCausalInputManifestV1>,
    pub(crate) contract_watermark: u64,
    pub(crate) active_protocol_mode_set_root_sha256: String,
    pub(crate) artifacts: Vec<NaturalT1ProgramArtifactV1>,
    pub(crate) active_protocols: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExactDurableSourceHeadsV1 {
    pub(crate) topology_rows: u64,
    pub(crate) topology_root_sha256: String,
    pub(crate) frame_rows: u64,
    pub(crate) frame_root_sha256: String,
    pub(crate) collection_checkpoint_root_sha256: String,
}

pub(crate) fn restore_exact_durable_source_heads_v1(
    config: &CertificationAuthorityConfigV1,
) -> Result<ExactDurableSourceHeadsV1, String> {
    let sources = config
        .k1_exact_sources
        .as_ref()
        .ok_or_else(|| "k1_exact_authority_sources_not_configured".to_owned())?;
    let topology = MultiSourceTopologyArchiveReadSnapshot::read(&sources.topology_archive_path)?;
    let frames = MultiSourceFrameArchiveReadSnapshot::read(&sources.frame_archive_path)?;
    let collection = OnlineCollectionMiner::open_read_only(&sources.collection_checkpoint_path)?;
    Ok(ExactDurableSourceHeadsV1 {
        topology_rows: u64::try_from(topology.len())
            .map_err(|_| "k1_exact_topology_count".to_owned())?,
        topology_root_sha256: topology.prefix_root_sha256().to_owned(),
        frame_rows: u64::try_from(frames.len()).map_err(|_| "k1_exact_frame_count".to_owned())?,
        frame_root_sha256: frames.prefix_root_sha256().to_owned(),
        collection_checkpoint_root_sha256: collection.checkpoint_root_sha256()?.to_owned(),
    })
}

pub(crate) fn restore_exact_opportunity_v1(
    config: &CertificationAuthorityConfigV1,
    deficit: &K1DeficitSnapshotV1,
    exact_attempt_index: &ExactAttemptIndexV1,
    active_protocols: &BTreeSet<String>,
) -> Result<RestoredExactOpportunityV1, String> {
    let sources = config
        .k1_exact_sources
        .as_ref()
        .ok_or_else(|| "k1_exact_authority_sources_not_configured".to_owned())?;
    let topology = MultiSourceTopologyArchiveReadSnapshot::read(&sources.topology_archive_path)?;
    let frames = MultiSourceFrameArchiveReadSnapshot::read(&sources.frame_archive_path)?;
    let collection = OnlineCollectionMiner::open_read_only(&sources.collection_checkpoint_path)?;
    let source_heads = ExactDurableSourceHeadsV1 {
        topology_rows: u64::try_from(topology.len())
            .map_err(|_| "k1_exact_topology_count".to_owned())?,
        topology_root_sha256: topology.prefix_root_sha256().to_owned(),
        frame_rows: u64::try_from(frames.len()).map_err(|_| "k1_exact_frame_count".to_owned())?,
        frame_root_sha256: frames.prefix_root_sha256().to_owned(),
        collection_checkpoint_root_sha256: collection.checkpoint_root_sha256()?.to_owned(),
    };
    let mut accumulator = super::EvidenceBindingAccumulator::new(true);
    let join_report = nando_operator_learning::multi_source::stream_multi_source_joins_from_iter(
        topology.rows().iter().map(|row| row.as_ref()),
        frames.frames().iter().map(|frame| frame.as_ref()),
        |joined| accumulator.push(joined),
    )?;
    let prepared = super::prepare_tick_context_from_bindings(
        join_report,
        accumulator.finish()?,
        active_protocols,
    )?;
    let artifacts = collection.natural_t1_program_artifacts()?;
    let projection = build_exact_opportunity_projection_v1(
        &prepared,
        deficit,
        &artifacts,
        exact_attempt_index,
        &source_heads,
    )?;
    if restore_exact_durable_source_heads_v1(config)? != source_heads {
        return Err("STALE_BEFORE_FREEZE".to_owned());
    }
    Ok(RestoredExactOpportunityV1 {
        source_heads,
        source_snapshot: projection.source_snapshot,
        catalog: prepared.motif_catalog,
        queue: projection.queue,
        support_manifests_by_candidate: projection.support_manifests_by_candidate,
        artifact_projections_by_candidate: projection.artifact_projections_by_candidate,
        causal_manifests_by_candidate: projection.causal_manifests_by_candidate,
        contract_watermark: prepared.contract_watermark,
        active_protocol_mode_set_root_sha256: prepared.active_protocol_mode_set_root_sha256,
        artifacts,
        active_protocols: active_protocols.clone(),
    })
}

struct ExactOpportunityProjectionV1 {
    source_snapshot: EvidenceSourceSnapshotV1,
    queue: K1NaturalCandidateQueueV1,
    support_manifests_by_candidate: BTreeMap<String, IdentifierSupportManifestV1>,
    artifact_projections_by_candidate: BTreeMap<String, RelevantIdentifierArtifactProjectionV1>,
    causal_manifests_by_candidate: BTreeMap<String, IdentifierCausalInputManifestV1>,
}

fn build_exact_opportunity_projection_v1(
    prepared: &PreparedK1TickContextV1,
    deficit: &K1DeficitSnapshotV1,
    artifacts: &[NaturalT1ProgramArtifactV1],
    exact_attempt_index: &ExactAttemptIndexV1,
    sources: &ExactDurableSourceHeadsV1,
) -> Result<ExactOpportunityProjectionV1, String> {
    let motif_archive = prepared
        .motif_archive
        .as_ref()
        .ok_or_else(|| "k1_exact_motif_archive_missing".to_owned())?;
    motif_archive.validate(&prepared.bindings)?;
    let catalog = &prepared.motif_catalog;
    let ordinary_queue = catalog
        .build_candidate_queue_with_exclusions(
            deficit,
            &BTreeSet::new(),
            prepared.contract_watermark,
        )
        .map_err(str::to_owned)?;
    let source_snapshot = EvidenceSourceSnapshotV1::seal(
        sources.topology_rows,
        sources.topology_root_sha256.clone(),
        sources.frame_rows,
        sources.frame_root_sha256.clone(),
        EXACT_JOIN_BUILDER_SCHEMA_V1.to_owned(),
        motif_archive.archive_root_sha256.clone(),
        source_neutral_topology_motif_config_root_v1().map_err(str::to_owned)?,
        sources.collection_checkpoint_root_sha256.clone(),
        prepared.active_protocol_mode_set_root_sha256.clone(),
    )
    .map_err(str::to_owned)?;
    let resource_limits = IdentifierResourceLimitsV1::seal(
        generation_budget().maximum_support_rows,
        EXACT_IDENTIFIER_MAX_SEED_PROGRAMS_V1,
        EXACT_IDENTIFIER_MAX_SEMANTIC_CLASSES_V1,
        EXACT_IDENTIFIER_RAW_PHASE_CELLS_V1,
    )
    .map_err(str::to_owned)?;
    let discovery_basis_root_sha256 =
        natural_t1_discovery_basis_root_v4().map_err(str::to_owned)?;
    let mut support_manifests_by_candidate = BTreeMap::new();
    let mut artifact_projections_by_candidate = BTreeMap::new();
    let mut causal_manifests_by_candidate = BTreeMap::new();
    for row in ordinary_queue
        .rows
        .iter()
        .filter(|row| row.score.readiness_rank == 1)
    {
        let candidate = catalog
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_root_sha256 == row.candidate_root_sha256)
            .ok_or_else(|| "k1_exact_queue_candidate_missing".to_owned())?;
        let support =
            build_support_manifest(prepared, candidate, resource_limits.maximum_support_rows)?;
        let relevant = build_relevant_identifier_artifact_projection_v1(&support, artifacts)
            .map_err(str::to_owned)?;
        let causal = build_identifier_causal_input_manifest_v1(
            &support,
            &relevant,
            candidate.generator_schema.clone(),
            discovery_basis_root_sha256.clone(),
            prepared.active_protocol_mode_set_root_sha256.clone(),
            resource_limits,
        )
        .map_err(str::to_owned)?;
        support_manifests_by_candidate.insert(row.candidate_root_sha256.clone(), support);
        artifact_projections_by_candidate.insert(row.candidate_root_sha256.clone(), relevant);
        causal_manifests_by_candidate.insert(row.candidate_root_sha256.clone(), causal);
    }
    let queue = ordinary_queue
        .bind_exact_opportunities_v4(
            exact_attempt_index,
            source_snapshot.snapshot_root_sha256.clone(),
            &causal_manifests_by_candidate,
        )
        .map_err(str::to_owned)?;
    Ok(ExactOpportunityProjectionV1 {
        source_snapshot,
        queue,
        support_manifests_by_candidate,
        artifact_projections_by_candidate,
        causal_manifests_by_candidate,
    })
}

fn build_support_manifest(
    prepared: &PreparedK1TickContextV1,
    candidate: &K1NaturalCohortCandidateV1,
    maximum_support_rows: u64,
) -> Result<IdentifierSupportManifestV1, String> {
    build_support_manifest_for_identity(
        prepared,
        &candidate.capture_generation_root_sha256,
        &candidate.candidate_structural_root_sha256,
        &candidate.semantic_novelty_signature_root_sha256,
        candidate.consequence_type,
        candidate.last_capture_sequence,
        maximum_support_rows,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_support_manifest_for_identity(
    prepared: &PreparedK1TickContextV1,
    capture_generation_root_sha256: &str,
    candidate_structural_root_sha256: &str,
    semantic_novelty_signature_root_sha256: &str,
    consequence_type: nando_operator_learning::multi_source::K1ConsequenceTypeV1,
    support_watermark: u64,
    maximum_support_rows: u64,
) -> Result<IdentifierSupportManifestV1, String> {
    let archive = prepared
        .motif_archive
        .as_ref()
        .ok_or_else(|| "k1_exact_motif_archive_missing".to_owned())?;
    let mut rows = Vec::new();
    for binding in archive.occurrences.iter().filter(|binding| {
        binding.row.capture_generation_root_sha256 == capture_generation_root_sha256
            && binding.row.motif_root_sha256 == candidate_structural_root_sha256
            && binding.row.semantic_novelty_signature_root_sha256
                == semantic_novelty_signature_root_sha256
            && binding.row.consequence_type == consequence_type
            && binding.row.capture_sequence <= support_watermark
    }) {
        let exact = archive.exact_occurrence(&prepared.bindings, binding)?;
        let joined = archive.joined_for(&prepared.bindings, binding)?;
        let embedding_roots_sha256 = exact
            .motif
            .embeddings
            .iter()
            .map(|embedding| embedding.embedding_root_sha256.clone())
            .collect();
        rows.push(
            IdentifierSupportRowV1::seal(
                binding.row.capture_sequence,
                binding.row.evidence_root_sha256.clone(),
                joined.completed_frame_root_sha256.clone(),
                binding.row.complete_topology_root_sha256.clone(),
                binding.row.capture_generation_root_sha256.clone(),
                binding.row.motif_root_sha256.clone(),
                embedding_roots_sha256,
                binding.row.lineage_root_sha256.clone(),
                joined.turn_intent_id_sha256.clone(),
                joined.session_id_sha256.clone(),
            )
            .map_err(str::to_owned)?,
        );
    }
    build_identifier_support_manifest_v1(
        candidate_structural_root_sha256.to_owned(),
        support_watermark,
        rows,
        maximum_support_rows,
    )
    .map_err(str::to_owned)
}

pub(crate) fn build_exact_identifier_archive_v1(
    source_snapshot: EvidenceSourceSnapshotV1,
    support_manifest: IdentifierSupportManifestV1,
    relevant_projection: RelevantIdentifierArtifactProjectionV1,
    all_artifacts: &[NaturalT1ProgramArtifactV1],
    causal_manifest: IdentifierCausalInputManifestV1,
    active_protocol_mode_roots_sha256: BTreeSet<String>,
) -> Result<(ExactIdentifierArchiveManifestV1, Vec<u8>), String> {
    source_snapshot.validate().map_err(str::to_owned)?;
    support_manifest
        .validate(causal_manifest.resource_limits.maximum_support_rows)
        .map_err(str::to_owned)?;
    relevant_projection.validate().map_err(str::to_owned)?;
    causal_manifest.validate().map_err(str::to_owned)?;
    if causal_manifest.support_manifest_root_sha256 != support_manifest.manifest_root_sha256
        || causal_manifest.relevant_artifact_projection_root_sha256
            != relevant_projection.projection_root_sha256
        || causal_manifest.active_protocol_mode_set_root_sha256
            != source_snapshot.active_protocol_mode_set_root_sha256
    {
        return Err("k1_exact_archive_binding_invalid".to_owned());
    }
    let relevant_roots = relevant_projection
        .artifact_roots_sha256
        .iter()
        .collect::<BTreeSet<_>>();
    let mut relevant_artifacts = all_artifacts
        .iter()
        .filter(|artifact| relevant_roots.contains(&artifact.artifact_root_sha256))
        .cloned()
        .collect::<Vec<_>>();
    relevant_artifacts
        .sort_by(|left, right| left.artifact_root_sha256.cmp(&right.artifact_root_sha256));
    if relevant_artifacts.len() != relevant_roots.len()
        || relevant_artifacts
            .iter()
            .any(|artifact| artifact.validate().is_err())
        || crate::k1_natural_scheduler::duplicate_cohorts::known_epistemic_protocol_mode_set_root(
            &active_protocol_mode_roots_sha256,
        )? != source_snapshot.active_protocol_mode_set_root_sha256
    {
        return Err("k1_exact_archive_payload_invalid".to_owned());
    }
    let object = ExactIdentifierArchiveObjectV1 {
        schema: EXACT_ARCHIVE_OBJECT_SCHEMA_V1.to_owned(),
        source_snapshot,
        support_manifest,
        relevant_projection,
        relevant_artifacts,
        causal_manifest,
        active_protocol_mode_roots_sha256,
    };
    validate_archive_object(&object)?;
    let object_bytes = serde_cbor::to_vec(&object)
        .map_err(|error| format!("k1_exact_archive_object_encode:{error}"))?;
    let object_root_sha256 = nando_operator_kernel::sha256_bytes(&object_bytes);
    let manifest_root_sha256 = nando_operator_kernel::canonical_json_sha256(&(
        EXACT_ARCHIVE_MANIFEST_SCHEMA_V1,
        object_root_sha256.as_str(),
        object.source_snapshot.snapshot_root_sha256.as_str(),
        object.support_manifest.manifest_root_sha256.as_str(),
        object.relevant_projection.projection_root_sha256.as_str(),
        object.causal_manifest.manifest_root_sha256.as_str(),
    ))
    .map_err(str::to_owned)?;
    Ok((
        ExactIdentifierArchiveManifestV1 {
            schema: EXACT_ARCHIVE_MANIFEST_SCHEMA_V1.to_owned(),
            manifest_root_sha256,
            object_root_sha256,
            source_snapshot_root_sha256: object.source_snapshot.snapshot_root_sha256,
            support_manifest_root_sha256: object.support_manifest.manifest_root_sha256,
            relevant_projection_root_sha256: object.relevant_projection.projection_root_sha256,
            causal_manifest_root_sha256: object.causal_manifest.manifest_root_sha256,
        },
        object_bytes,
    ))
}

pub(crate) fn restore_exact_identifier_inputs_v1(
    config: &CertificationAuthorityConfigV1,
    freeze: &K1NaturalCandidateFreezeV1,
) -> Result<RestoredExactIdentifierInputsV1, String> {
    let sources = config
        .k1_exact_sources
        .as_ref()
        .ok_or_else(|| "k1_exact_authority_sources_not_configured".to_owned())?;
    let (manifest, object) = restore_exact_identifier_archive_object_v1(
        &sources.artifact_archive_path,
        &freeze.identifier_artifact_archive_manifest_root_sha256,
    )?;
    if manifest.source_snapshot_root_sha256 != object.source_snapshot.snapshot_root_sha256
        || manifest.support_manifest_root_sha256 != object.support_manifest.manifest_root_sha256
        || manifest.relevant_projection_root_sha256
            != object.relevant_projection.projection_root_sha256
        || manifest.causal_manifest_root_sha256 != object.causal_manifest.manifest_root_sha256
        || freeze.evidence_source_snapshot_root_sha256
            != object.source_snapshot.snapshot_root_sha256
        || freeze.identifier_causal_input_manifest.as_deref() != Some(&object.causal_manifest)
    {
        return Err("k1_exact_archive_freeze_binding_invalid".to_owned());
    }
    let topology = MultiSourceTopologyArchiveReadSnapshot::read(&sources.topology_archive_path)?;
    let frames = MultiSourceFrameArchiveReadSnapshot::read(&sources.frame_archive_path)?;
    let topology_prefix = topology.verified_prefix(
        object.source_snapshot.topology_prefix_rows,
        &object.source_snapshot.topology_prefix_root_sha256,
    )?;
    let frame_prefix = frames.verified_prefix(
        object.source_snapshot.frame_prefix_rows,
        &object.source_snapshot.frame_prefix_root_sha256,
    )?;
    let mut accumulator = super::EvidenceBindingAccumulator::new(true);
    let join_report = nando_operator_learning::multi_source::stream_multi_source_joins_from_iter(
        topology_prefix.iter().map(|row| row.as_ref()),
        frame_prefix.iter().map(|frame| frame.as_ref()),
        |joined| accumulator.push(joined),
    )?;
    let prepared = super::prepare_tick_context_from_bindings(
        join_report,
        accumulator.finish()?,
        &object.active_protocol_mode_roots_sha256,
    )?;
    if object.source_snapshot.join_builder_schema != EXACT_JOIN_BUILDER_SCHEMA_V1
        || object.source_snapshot.motif_archive_root_sha256
            != prepared
                .motif_archive
                .as_ref()
                .ok_or_else(|| "k1_exact_motif_archive_missing".to_owned())?
                .archive_root_sha256
        || object.source_snapshot.motif_config_root_sha256
            != source_neutral_topology_motif_config_root_v1().map_err(str::to_owned)?
        || object.source_snapshot.active_protocol_mode_set_root_sha256
            != prepared.active_protocol_mode_set_root_sha256
    {
        return Err("k1_exact_archive_source_parity_failed".to_owned());
    }
    let rebuilt_support = build_support_manifest_for_identity(
        &prepared,
        &freeze.capture_generation_root_sha256,
        &freeze.candidate_structural_root_sha256,
        &freeze.semantic_novelty_signature_root_sha256,
        freeze.consequence_type,
        freeze.support_watermark,
        object.causal_manifest.resource_limits.maximum_support_rows,
    )?;
    if rebuilt_support != object.support_manifest {
        return Err("k1_exact_archive_support_parity_failed".to_owned());
    }
    Ok(RestoredExactIdentifierInputsV1 {
        prepared,
        frames: frame_prefix
            .iter()
            .map(|frame| frame.as_ref().clone())
            .collect(),
        artifacts: object.relevant_artifacts,
        active_protocols: object.active_protocol_mode_roots_sha256,
        support: object.support_manifest,
        projection: object.relevant_projection,
        causal: object.causal_manifest,
    })
}

fn restore_exact_identifier_archive_object_v1(
    archive_root: &Path,
    manifest_root: &str,
) -> Result<
    (
        ExactIdentifierArchiveManifestV1,
        ExactIdentifierArchiveObjectV1,
    ),
    String,
> {
    let manifest_path = archive_root
        .join("manifests")
        .join(format!("{manifest_root}.json"));
    let manifest_bytes = std::fs::read(&manifest_path)
        .map_err(|error| format!("k1_exact_archive_manifest_read:{error}"))?;
    let manifest: ExactIdentifierArchiveManifestV1 = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("k1_exact_archive_manifest_decode:{error}"))?;
    validate_archive_manifest(&manifest)?;
    if manifest.manifest_root_sha256 != *manifest_root
        || nando_operator_kernel::canonical_json_bytes(&manifest).map_err(str::to_owned)?
            != manifest_bytes
    {
        return Err("k1_exact_archive_manifest_rebind".to_owned());
    }
    let object_path = archive_root
        .join("objects")
        .join(format!("{}.cbor", manifest.object_root_sha256));
    let object_bytes = std::fs::read(&object_path)
        .map_err(|error| format!("k1_exact_archive_object_read:{error}"))?;
    if nando_operator_kernel::sha256_bytes(&object_bytes) != manifest.object_root_sha256 {
        return Err("k1_exact_archive_object_rebind".to_owned());
    }
    let object: ExactIdentifierArchiveObjectV1 = serde_cbor::from_slice(&object_bytes)
        .map_err(|error| format!("k1_exact_archive_object_decode:{error}"))?;
    validate_archive_object(&object)?;
    Ok((manifest, object))
}

pub(crate) fn evaluate_exact_initial_identifier_v1(
    inputs: &RestoredExactIdentifierInputsV1,
    freeze: &K1NaturalCandidateFreezeV1,
) -> Result<ExactInitialIdentifierEvaluationV1, String> {
    if inputs.causal.opportunity_root_sha256
        != freeze
            .identifier_causal_input_manifest
            .as_deref()
            .ok_or_else(|| "k1_exact_identifier_causal_manifest_missing".to_owned())?
            .opportunity_root_sha256
        || inputs.support.support_watermark != freeze.support_watermark
        || inputs.projection.projection_root_sha256
            != inputs.causal.relevant_artifact_projection_root_sha256
    {
        return Err("k1_exact_identifier_input_binding_invalid".to_owned());
    }
    let report = super::identify_frozen_candidate(
        &inputs.prepared.bindings,
        inputs.prepared.motif_archive.as_ref(),
        &inputs.frames,
        &inputs.active_protocols,
        &inputs.artifacts,
        freeze,
        &BTreeSet::new(),
        &BTreeSet::new(),
    )?;
    let dispositions = exact_seed_dispositions(inputs, freeze)?;
    Ok(ExactInitialIdentifierEvaluationV1 {
        report,
        dispositions,
    })
}

fn exact_seed_dispositions(
    inputs: &RestoredExactIdentifierInputsV1,
    freeze: &K1NaturalCandidateFreezeV1,
) -> Result<ProgramDispositionSetV1, String> {
    let archive = inputs
        .prepared
        .motif_archive
        .as_ref()
        .ok_or_else(|| "k1_exact_motif_archive_missing".to_owned())?;
    let representative = archive
        .occurrences
        .iter()
        .filter(|binding| {
            binding.row.capture_generation_root_sha256 == freeze.capture_generation_root_sha256
                && binding.row.motif_root_sha256 == freeze.candidate_structural_root_sha256
                && binding.row.semantic_novelty_signature_root_sha256
                    == freeze.semantic_novelty_signature_root_sha256
                && binding.row.consequence_type == freeze.consequence_type
                && binding.row.capture_sequence <= freeze.support_watermark
        })
        .min_by(|left, right| {
            (
                left.row.capture_sequence,
                left.row.evidence_root_sha256.as_str(),
            )
                .cmp(&(
                    right.row.capture_sequence,
                    right.row.evidence_root_sha256.as_str(),
                ))
        })
        .ok_or_else(|| "k1_exact_identifier_support_missing".to_owned())?;
    let exact = archive.exact_occurrence(&inputs.prepared.bindings, representative)?;
    let joined = archive.joined_for(&inputs.prepared.bindings, representative)?;
    let frame_by_root = inputs
        .frames
        .iter()
        .filter_map(|frame| {
            nando_operator_kernel::canonical_json_sha256(frame)
                .ok()
                .map(|root| (root, frame))
        })
        .collect::<BTreeMap<_, _>>();
    let frame = frame_by_root
        .get(&joined.completed_frame_root_sha256)
        .ok_or_else(|| "k1_exact_identifier_frame_missing".to_owned())?;
    let effect =
        nando_operator_learning::multi_source::factor_multi_source_row_v1(joined).completed_effect;
    let programs = if effect == CompletedEffectFormV1::CollectionTransform {
        inputs.projection.programs.clone()
    } else {
        nando_operator_learning::multi_source::enumerate_source_neutral_t1_candidates(
            joined, frame,
        )?
    };
    evaluate_program_dispositions_v1(&programs, &joined.topology, &exact.motif)
        .map(|(dispositions, _)| dispositions)
        .map_err(str::to_owned)
}

fn validate_archive_object(object: &ExactIdentifierArchiveObjectV1) -> Result<(), String> {
    object.source_snapshot.validate().map_err(str::to_owned)?;
    object.causal_manifest.validate().map_err(str::to_owned)?;
    object
        .support_manifest
        .validate(object.causal_manifest.resource_limits.maximum_support_rows)
        .map_err(str::to_owned)?;
    object
        .relevant_projection
        .validate()
        .map_err(str::to_owned)?;
    if object.schema != EXACT_ARCHIVE_OBJECT_SCHEMA_V1
        || object.causal_manifest.support_manifest_root_sha256
            != object.support_manifest.manifest_root_sha256
        || object
            .causal_manifest
            .relevant_artifact_projection_root_sha256
            != object.relevant_projection.projection_root_sha256
        || object.causal_manifest.active_protocol_mode_set_root_sha256
            != object.source_snapshot.active_protocol_mode_set_root_sha256
        || object
            .relevant_artifacts
            .iter()
            .any(|artifact| artifact.validate().is_err())
    {
        return Err("k1_exact_archive_object_invalid".to_owned());
    }
    let roots = object
        .relevant_artifacts
        .iter()
        .map(|artifact| artifact.artifact_root_sha256.as_str())
        .collect::<Vec<_>>();
    if !roots.windows(2).all(|pair| pair[0] < pair[1])
        || roots
            != object
                .relevant_projection
                .artifact_roots_sha256
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
    {
        return Err("k1_exact_archive_artifact_projection_mismatch".to_owned());
    }
    Ok(())
}

fn validate_archive_manifest(manifest: &ExactIdentifierArchiveManifestV1) -> Result<(), String> {
    let expected = nando_operator_kernel::canonical_json_sha256(&(
        EXACT_ARCHIVE_MANIFEST_SCHEMA_V1,
        manifest.object_root_sha256.as_str(),
        manifest.source_snapshot_root_sha256.as_str(),
        manifest.support_manifest_root_sha256.as_str(),
        manifest.relevant_projection_root_sha256.as_str(),
        manifest.causal_manifest_root_sha256.as_str(),
    ))
    .map_err(str::to_owned)?;
    if manifest.schema != EXACT_ARCHIVE_MANIFEST_SCHEMA_V1
        || manifest.manifest_root_sha256 != expected
    {
        return Err("k1_exact_archive_manifest_invalid".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use nando_operator_kernel::canonical_json_sha256;
    use nando_operator_learning::multi_source::{
        IdentifierResourceLimitsV1, IdentifierSupportRowV1,
        build_identifier_causal_input_manifest_v1, build_identifier_support_manifest_v1,
        build_relevant_identifier_artifact_projection_v1,
    };

    use super::*;

    fn root(value: u64) -> String {
        format!("{value:064x}")
    }

    fn archive_fixture() -> (
        EvidenceSourceSnapshotV1,
        IdentifierSupportManifestV1,
        RelevantIdentifierArtifactProjectionV1,
        IdentifierCausalInputManifestV1,
        BTreeSet<String>,
    ) {
        let active_protocols = BTreeSet::new();
        let active_root =
            crate::k1_natural_scheduler::duplicate_cohorts::known_epistemic_protocol_mode_set_root(
                &active_protocols,
            )
            .expect("active root");
        let support = build_identifier_support_manifest_v1(
            root(1),
            1,
            vec![
                IdentifierSupportRowV1::seal(
                    1,
                    root(2),
                    root(3),
                    root(4),
                    root(5),
                    root(1),
                    vec![root(6)],
                    root(7),
                    root(8),
                    root(9),
                )
                .expect("row"),
            ],
            64,
        )
        .expect("support");
        let projection =
            build_relevant_identifier_artifact_projection_v1(&support, &[]).expect("projection");
        let causal = build_identifier_causal_input_manifest_v1(
            &support,
            &projection,
            "nando.operator-blind-version-space-generator.v4".to_owned(),
            root(10),
            active_root.clone(),
            IdentifierResourceLimitsV1::seal(64, 4_096, 4_096, 16).expect("limits"),
        )
        .expect("causal");
        let source = EvidenceSourceSnapshotV1::seal(
            1,
            root(11),
            1,
            root(12),
            EXACT_JOIN_BUILDER_SCHEMA_V1.to_owned(),
            root(13),
            source_neutral_topology_motif_config_root_v1().expect("motif config"),
            root(14),
            active_root,
        )
        .expect("source");
        (source, support, projection, causal, active_protocols)
    }

    #[test]
    fn archive_object_binds_complete_frozen_inputs_and_rejects_tamper() {
        let (source, support, projection, causal, active_protocols) = archive_fixture();
        let (manifest, bytes) = build_exact_identifier_archive_v1(
            source.clone(),
            support.clone(),
            projection.clone(),
            &[],
            causal.clone(),
            active_protocols,
        )
        .expect("archive");
        validate_archive_manifest(&manifest).expect("manifest");
        assert_eq!(
            manifest.object_root_sha256,
            nando_operator_kernel::sha256_bytes(&bytes)
        );
        let object: ExactIdentifierArchiveObjectV1 =
            serde_cbor::from_slice(&bytes).expect("object decode");
        validate_archive_object(&object).expect("object");
        assert_eq!(object.source_snapshot, source);
        assert_eq!(object.support_manifest, support);
        assert_eq!(object.relevant_projection, projection);
        assert_eq!(object.causal_manifest, causal);

        let mut tampered = object;
        tampered.causal_manifest.support_manifest_root_sha256 =
            canonical_json_sha256(&("tampered", 1_u64)).expect("tampered root");
        assert_eq!(
            validate_archive_object(&tampered),
            Err("identifier_causal_input_manifest_invalid".to_owned())
        );
    }

    #[test]
    fn archive_restore_rejects_missing_and_tampered_halves() {
        let (source, support, projection, causal, active_protocols) = archive_fixture();
        let (manifest, object_bytes) = build_exact_identifier_archive_v1(
            source,
            support,
            projection,
            &[],
            causal,
            active_protocols,
        )
        .expect("archive");
        let root_dir =
            std::env::temp_dir().join(format!("nando-exact-archive-fault-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root_dir);
        std::fs::create_dir_all(root_dir.join("objects")).expect("object dir");
        std::fs::create_dir_all(root_dir.join("manifests")).expect("manifest dir");
        let object_path = root_dir
            .join("objects")
            .join(format!("{}.cbor", manifest.object_root_sha256));
        let manifest_path = root_dir
            .join("manifests")
            .join(format!("{}.json", manifest.manifest_root_sha256));

        std::fs::write(&object_path, &object_bytes).expect("orphan object");
        assert!(
            restore_exact_identifier_archive_object_v1(&root_dir, &manifest.manifest_root_sha256)
                .expect_err("object without manifest")
                .starts_with("k1_exact_archive_manifest_read:")
        );

        let manifest_bytes =
            nando_operator_kernel::canonical_json_bytes(&manifest).expect("canonical manifest");
        std::fs::write(&manifest_path, &manifest_bytes).expect("manifest");
        std::fs::remove_file(&object_path).expect("remove object");
        assert!(
            restore_exact_identifier_archive_object_v1(&root_dir, &manifest.manifest_root_sha256)
                .expect_err("manifest without object")
                .starts_with("k1_exact_archive_object_read:")
        );

        std::fs::write(&object_path, b"tampered-object").expect("tampered object");
        assert_eq!(
            restore_exact_identifier_archive_object_v1(&root_dir, &manifest.manifest_root_sha256)
                .expect_err("tampered object"),
            "k1_exact_archive_object_rebind"
        );

        std::fs::write(&object_path, &object_bytes).expect("restore object");
        let mut tampered_manifest = manifest_bytes;
        tampered_manifest.push(b' ');
        std::fs::write(&manifest_path, tampered_manifest).expect("tampered manifest");
        assert_eq!(
            restore_exact_identifier_archive_object_v1(&root_dir, &manifest.manifest_root_sha256)
                .expect_err("noncanonical manifest"),
            "k1_exact_archive_manifest_rebind"
        );
        std::fs::remove_dir_all(root_dir).expect("cleanup");
    }
}
