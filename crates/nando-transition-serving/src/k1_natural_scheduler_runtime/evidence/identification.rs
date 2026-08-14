use super::*;

pub(in crate::k1_natural_scheduler_runtime) fn frozen_support_count(
    bindings: &[EvidenceBinding],
    motif_archive: Option<&MotifEvidenceArchive>,
    freeze: &K1NaturalCandidateFreezeV1,
) -> Result<u64, String> {
    if matches!(
        freeze.schema.as_str(),
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6
            | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V7
            | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8
    ) {
        return frozen_motif_support_count(bindings, motif_archive, freeze);
    }
    let mut support = Vec::new();
    let mut projected_rows = Vec::new();
    for binding in bindings {
        let Some(row) = evidence_row_for_freeze(binding, freeze)? else {
            continue;
        };
        if row_identity_matches_freeze(&row, freeze)
            && frozen_row_is_eligible(&row, freeze)
            && frozen_support_contains(row.capture_sequence, freeze.support_watermark)
        {
            support.push(binding);
            projected_rows.push(row);
        }
    }
    let manifest = frozen_support_manifest(projected_rows.iter())?;
    if support.is_empty()
        || support.len() > K1_MAX_SUPPORT_ROWS_V1
        || manifest != freeze.evidence_manifest_root_sha256
    {
        let legacy_rows = bindings
            .iter()
            .filter(|binding| binding.row.schema == K1_NATURAL_EVIDENCE_ROW_SCHEMA_V1)
            .count();
        let generation_rows = bindings
            .iter()
            .filter(|binding| {
                evidence_row_for_freeze(binding, freeze)
                    .ok()
                    .flatten()
                    .is_some_and(|row| {
                        capture_generation_matches(
                            &row.schema,
                            &row.capture_generation_root_sha256,
                            &freeze.schema,
                            &freeze.capture_generation_root_sha256,
                        )
                    })
            })
            .count();
        let identity_rows = bindings
            .iter()
            .filter(|binding| binding_matches_freeze(binding, freeze))
            .count();
        return Err(format!(
            "k1_runtime_frozen_support_manifest_mismatch:generation={}:rows={}:bindings={}:legacy={}:generation_match={}:identity_match={}:actual={}:expected={}",
            freeze.generation_sequence,
            support.len(),
            bindings.len(),
            legacy_rows,
            generation_rows,
            identity_rows,
            manifest,
            freeze.evidence_manifest_root_sha256,
        ));
    }
    u64::try_from(support.len()).map_err(|_| "k1_runtime_frozen_support_count".to_owned())
}

fn frozen_motif_support_count(
    bindings: &[EvidenceBinding],
    motif_archive: Option<&MotifEvidenceArchive>,
    freeze: &K1NaturalCandidateFreezeV1,
) -> Result<u64, String> {
    let archive = motif_archive.ok_or_else(|| "k1_motif_archive_missing".to_owned())?;
    archive.validate(bindings)?;
    if archive.disposition.enumeration_config_root_sha256
        != freeze.motif_enumeration_config_root_sha256
    {
        return Err("k1_runtime_frozen_motif_disposition_mismatch".to_owned());
    }
    let mut support = archive
        .occurrences
        .iter()
        .filter(|binding| {
            row_identity_matches_freeze(&binding.row, freeze)
                && frozen_row_is_eligible(&binding.row, freeze)
                && frozen_support_contains(binding.row.capture_sequence, freeze.support_watermark)
        })
        .collect::<Vec<_>>();
    support.sort_by(|left, right| {
        left.row
            .capture_sequence
            .cmp(&right.row.capture_sequence)
            .then_with(|| left.row.row_root_sha256.cmp(&right.row.row_root_sha256))
    });
    let manifest = canonical_json_sha256(&(
        "nando.k1-motif-evidence-manifest.v1",
        support
            .iter()
            .map(|binding| binding.row.row_root_sha256.as_str())
            .collect::<Vec<_>>(),
    ))
    .map_err(str::to_owned)?;
    let complete_topology_manifest = canonical_json_sha256(&(
        "nando.k1-motif-complete-topology-manifest.v1",
        support
            .iter()
            .map(|binding| binding.row.complete_topology_root_sha256.as_str())
            .collect::<Vec<_>>(),
    ))
    .map_err(str::to_owned)?;
    let embedding_manifest = canonical_json_sha256(&(
        "nando.k1-motif-candidate-embedding-manifest.v1",
        support
            .iter()
            .map(|binding| binding.row.motif_embedding_manifest_root_sha256.as_str())
            .collect::<Vec<_>>(),
    ))
    .map_err(str::to_owned)?;
    archive
        .candidate_supports
        .iter()
        .find(|receipt| {
            receipt.capture_generation_root_sha256 == freeze.capture_generation_root_sha256
                && receipt.motif_root_sha256 == freeze.candidate_structural_root_sha256
                && receipt.semantic_novelty_signature_root_sha256
                    == freeze.semantic_novelty_signature_root_sha256
                && receipt.consequence_type == freeze.consequence_type
        })
        .ok_or_else(|| "k1_runtime_frozen_motif_support_receipt_missing".to_owned())?;
    if support.is_empty()
        || support.len() > K1_MAX_SUPPORT_ROWS_V1
        || manifest != freeze.evidence_manifest_root_sha256
        || complete_topology_manifest != freeze.complete_topology_manifest_root_sha256
        || embedding_manifest != freeze.motif_embedding_manifest_root_sha256
    {
        return Err("k1_runtime_frozen_motif_support_manifest_mismatch".to_owned());
    }
    let support_count = support.len();
    for binding in support {
        archive.exact_occurrence(bindings, binding)?;
    }
    u64::try_from(support_count).map_err(|_| "k1_runtime_frozen_motif_support_count".to_owned())
}

pub(in crate::k1_natural_scheduler_runtime) fn binding_matches_freeze(
    binding: &EvidenceBinding,
    freeze: &K1NaturalCandidateFreezeV1,
) -> bool {
    evidence_row_for_freeze(binding, freeze)
        .ok()
        .flatten()
        .is_some_and(|row| row_identity_matches_freeze(&row, freeze))
}

fn binding_is_eligible_for_freeze(
    binding: &EvidenceBinding,
    freeze: &K1NaturalCandidateFreezeV1,
) -> bool {
    evidence_row_for_freeze(binding, freeze)
        .ok()
        .flatten()
        .is_some_and(|row| {
            row_identity_matches_freeze(&row, freeze) && frozen_row_is_eligible(&row, freeze)
        })
}

fn row_identity_matches_freeze(
    row: &K1NaturalEvidenceRowV1,
    freeze: &K1NaturalCandidateFreezeV1,
) -> bool {
    capture_generation_matches(
        &row.schema,
        &row.capture_generation_root_sha256,
        &freeze.schema,
        &freeze.capture_generation_root_sha256,
    ) && row.candidate_structural_root_sha256 == freeze.candidate_structural_root_sha256
        && row.source_neutral_topology_root_sha256 == freeze.source_neutral_topology_root_sha256
        && row.semantic_novelty_signature_root_sha256
            == freeze.semantic_novelty_signature_root_sha256
        && row.consequence_type == freeze.consequence_type
}

fn frozen_row_is_eligible(
    row: &K1NaturalEvidenceRowV1,
    freeze: &K1NaturalCandidateFreezeV1,
) -> bool {
    !row.safety_veto || freeze.schema == K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V1
}

#[cfg(test)]
fn historical_v1_evidence_row(
    row: &K1NaturalEvidenceRowV1,
) -> Result<K1NaturalEvidenceRowV1, String> {
    K1NaturalEvidenceRowV1::seal_legacy_v1(
        row.evidence_root_sha256.clone(),
        row.candidate_structural_root_sha256.clone(),
        row.source_neutral_topology_root_sha256.clone(),
        row.semantic_novelty_signature_root_sha256.clone(),
        row.lineage_root_sha256.clone(),
        row.consequence_type,
        row.evidence_class,
        row.capture_sequence,
        row.contract_sequence,
        row.input_tokens,
        row.settled,
        row.verified,
        false,
    )
    .map_err(str::to_owned)
}

fn evidence_row_for_freeze(
    binding: &EvidenceBinding,
    freeze: &K1NaturalCandidateFreezeV1,
) -> Result<Option<K1NaturalEvidenceRowV1>, String> {
    if matches!(
        freeze.schema.as_str(),
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6
            | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V7
            | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8
    ) {
        return Ok(exact_motif_occurrence_for_binding(binding, freeze)?.map(|value| value.row));
    }
    if matches!(
        freeze.schema.as_str(),
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V4 | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V5
    ) {
        return Ok(
            (binding.row.schema == K1_NATURAL_EVIDENCE_ROW_SCHEMA_V3).then(|| binding.row.clone())
        );
    }
    let Some(joined) = binding.joined.as_deref() else {
        return Ok(None);
    };
    let topology_root = candidate_topology_root(freeze, &joined.topology)?;
    let row = &binding.row;
    let projected = match freeze.schema.as_str() {
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V1 => K1NaturalEvidenceRowV1::seal_legacy_v1(
            row.evidence_root_sha256.clone(),
            row.candidate_structural_root_sha256.clone(),
            topology_root,
            row.semantic_novelty_signature_root_sha256.clone(),
            row.lineage_root_sha256.clone(),
            row.consequence_type,
            row.evidence_class,
            row.capture_sequence,
            row.contract_sequence,
            row.input_tokens,
            row.settled,
            row.verified,
            false,
        ),
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V2 | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V3 => {
            K1NaturalEvidenceRowV1::seal_legacy_v2(
                row.evidence_root_sha256.clone(),
                row.capture_generation_root_sha256.clone(),
                row.candidate_structural_root_sha256.clone(),
                topology_root,
                row.semantic_novelty_signature_root_sha256.clone(),
                row.lineage_root_sha256.clone(),
                row.consequence_type,
                row.evidence_class,
                row.capture_sequence,
                row.contract_sequence,
                row.input_tokens,
                row.settled,
                row.verified,
                row.safety_veto,
            )
        }
        _ => return Ok(None),
    }
    .map_err(str::to_owned)?;
    Ok(Some(projected))
}

fn exact_motif_occurrence_for_binding(
    binding: &EvidenceBinding,
    freeze: &K1NaturalCandidateFreezeV1,
) -> Result<Option<ExactMotifOccurrenceV1>, String> {
    let Some(joined) = binding.joined.as_deref() else {
        return Ok(None);
    };
    Ok(exact_motif_occurrences_for_joined(joined)?
        .into_iter()
        .find(|occurrence| row_identity_matches_freeze(&occurrence.row, freeze)))
}

#[allow(clippy::too_many_arguments)]
pub(in crate::k1_natural_scheduler_runtime) fn identify_frozen_candidate(
    bindings: &[EvidenceBinding],
    motif_archive: Option<&MotifEvidenceArchive>,
    frames: &[RelationFrame],
    active_protocol_mode_roots_sha256: &BTreeSet<String>,
    candidate_artifacts: &[NaturalT1ProgramArtifactV1],
    freeze: &K1NaturalCandidateFreezeV1,
    applied_roots: &BTreeSet<String>,
    trial_roots: &BTreeSet<String>,
) -> Result<MultiSourceT1IdentificationV3, String> {
    validate_installed_discovery_basis(freeze)?;
    let (mut selected, motifs, occurrence_row_roots) = if matches!(
        freeze.schema.as_str(),
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6
            | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V7
            | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8
    ) {
        let (selected, motifs, row_roots) = frozen_motif_identification_evidence(
            bindings,
            motif_archive,
            freeze,
            applied_roots,
            trial_roots,
        )?;
        (selected, Some(motifs), row_roots)
    } else {
        let selected = bindings
            .iter()
            .filter(|binding| {
                binding_is_eligible_for_freeze(binding, freeze)
                    && (frozen_support_contains(
                        binding.row.capture_sequence,
                        freeze.support_watermark,
                    ) || applied_roots.contains(binding.join_root_sha256())
                        || trial_roots.contains(binding.join_root_sha256()))
            })
            .map(|binding| binding.joined().clone())
            .collect::<Vec<_>>();
        (selected, None, Vec::new())
    };
    let selected_motifs = motifs;
    if let Some(motifs) = selected_motifs.as_ref() {
        if motifs.len() != selected.len() {
            return Err("k1_runtime_frozen_motif_alignment_mismatch".to_owned());
        }
    } else {
        selected.sort_by(|left, right| {
            left.capture_sequence
                .cmp(&right.capture_sequence)
                .then_with(|| left.join_root_sha256.cmp(&right.join_root_sha256))
        });
    }
    let artifact_roots = candidate_artifacts
        .iter()
        .filter(|artifact| {
            selected.iter().any(|row| {
                row.turn_intent_id_sha256 == artifact.turn_intent_id_sha256
                    && row.session_id_sha256 == artifact.session_id_sha256
            })
        })
        .map(|artifact| artifact.artifact_root_sha256.as_str())
        .collect::<Vec<_>>();
    let identification_domain_root = identification_domain_root(freeze)?;
    let epoch = if let Some(motifs) = selected_motifs.as_ref() {
        canonical_json_sha256(&(
            "nando.k1-frozen-motif-identification-evidence.v1",
            identification_domain_root,
            occurrence_row_roots,
            selected
                .iter()
                .zip(motifs)
                .map(|(row, motif)| {
                    (
                        row.join_root_sha256.as_str(),
                        motif.motif_root_sha256.as_str(),
                        motif
                            .embeddings
                            .iter()
                            .map(|embedding| embedding.embedding_root_sha256.as_str())
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>(),
            artifact_roots,
        ))
    } else {
        canonical_json_sha256(&(
            "nando.k1-frozen-identification-evidence.v1",
            identification_domain_root,
            selected
                .iter()
                .map(|row| row.join_root_sha256.as_str())
                .collect::<Vec<_>>(),
            artifact_roots,
        ))
    }
    .map_err(str::to_owned)?;
    let contract = FrozenRawPhaseT1ContractV1 {
        frozen_domain_root_sha256: identification_domain_root,
        support_watermark: freeze.support_watermark,
        candidate_generator_schema: &freeze.generator_schema,
    };
    let report = selected_motifs.as_ref().map_or_else(
        || {
            identify_multi_source_t1_operator_with_frozen_raw_phase_v1(
                &selected,
                frames,
                &BTreeSet::new(),
                active_protocol_mode_roots_sha256,
                candidate_artifacts,
                contract,
                epoch.clone(),
            )
        },
        |motifs| {
            identify_multi_source_t1_operator_with_frozen_motif_v1(
                &selected,
                motifs,
                frames,
                &BTreeSet::new(),
                active_protocol_mode_roots_sha256,
                candidate_artifacts,
                contract,
                epoch.clone(),
            )
        },
    );
    if !report.validate()
        || !selected_shape_is_compatible(
            report.selected_shape_root_sha256.as_deref(),
            &freeze.candidate_structural_root_sha256,
        )
    {
        return Err("k1_runtime_identification_report_invalid".to_owned());
    }
    Ok(report)
}

pub(in crate::k1_natural_scheduler_runtime) fn frozen_support_completed_frame_roots(
    bindings: &[EvidenceBinding],
    motif_archive: Option<&MotifEvidenceArchive>,
    freeze: &K1NaturalCandidateFreezeV1,
) -> Result<BTreeSet<String>, String> {
    let roots = if matches!(
        freeze.schema.as_str(),
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6
            | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V7
            | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8
    ) {
        let archive = motif_archive.ok_or_else(|| "k1_motif_archive_missing".to_owned())?;
        archive.validate(bindings)?;
        archive
            .occurrences
            .iter()
            .filter(|binding| {
                row_identity_matches_freeze(&binding.row, freeze)
                    && frozen_row_is_eligible(&binding.row, freeze)
                    && frozen_support_contains(
                        binding.row.capture_sequence,
                        freeze.support_watermark,
                    )
            })
            .map(|binding| {
                archive
                    .joined_for(bindings, binding)
                    .map(|joined| joined.completed_frame_root_sha256.clone())
            })
            .collect::<Result<BTreeSet<_>, _>>()?
    } else {
        bindings
            .iter()
            .filter(|binding| {
                binding_is_eligible_for_freeze(binding, freeze)
                    && frozen_support_contains(
                        binding.row.capture_sequence,
                        freeze.support_watermark,
                    )
            })
            .map(|binding| binding.completed_frame_root_sha256.clone())
            .collect()
    };
    if roots.is_empty() {
        return Err("k1_runtime_frozen_support_frames_missing".to_owned());
    }
    Ok(roots)
}

type FrozenMotifIdentificationEvidence = (
    Vec<BlindThenRevealJoinedTransitionV1>,
    Vec<SourceNeutralTopologyMotifV1>,
    Vec<String>,
);

pub(in crate::k1_natural_scheduler_runtime) fn frozen_motif_identification_evidence(
    bindings: &[EvidenceBinding],
    motif_archive: Option<&MotifEvidenceArchive>,
    freeze: &K1NaturalCandidateFreezeV1,
    applied_roots: &BTreeSet<String>,
    trial_roots: &BTreeSet<String>,
) -> Result<FrozenMotifIdentificationEvidence, String> {
    let archive = motif_archive.ok_or_else(|| "k1_motif_archive_missing".to_owned())?;
    archive.validate(bindings)?;
    let mut selected = BTreeMap::<
        (u64, String),
        (
            BlindThenRevealJoinedTransitionV1,
            SourceNeutralTopologyMotifV1,
            String,
        ),
    >::new();
    for binding in archive.occurrences.iter().filter(|binding| {
        row_identity_matches_freeze(&binding.row, freeze)
            && frozen_row_is_eligible(&binding.row, freeze)
            && frozen_support_contains(binding.row.capture_sequence, freeze.support_watermark)
    }) {
        let exact = archive.exact_occurrence(bindings, binding)?;
        let joined = archive.joined_for(bindings, binding)?.clone();
        selected.insert(
            (joined.capture_sequence, joined.join_root_sha256.clone()),
            (joined, exact.motif, exact.row.row_root_sha256),
        );
    }
    for binding in bindings.iter().filter(|binding| {
        applied_roots.contains(binding.join_root_sha256())
            || trial_roots.contains(binding.join_root_sha256())
    }) {
        let Some(exact) = exact_motif_occurrence_for_binding(binding, freeze)? else {
            continue;
        };
        if !frozen_row_is_eligible(&exact.row, freeze) {
            continue;
        }
        let joined = binding.joined().clone();
        selected.insert(
            (joined.capture_sequence, joined.join_root_sha256.clone()),
            (joined, exact.motif, exact.row.row_root_sha256),
        );
    }
    let mut joined = Vec::with_capacity(selected.len());
    let mut motifs = Vec::with_capacity(selected.len());
    let mut row_roots = Vec::with_capacity(selected.len());
    for (_, (row, motif, row_root)) in selected {
        joined.push(row);
        motifs.push(motif);
        row_roots.push(row_root);
    }
    Ok((joined, motifs, row_roots))
}

fn capture_generation_matches(
    row_schema: &str,
    row_generation: &str,
    freeze_schema: &str,
    freeze_generation: &str,
) -> bool {
    match freeze_schema {
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V1 => {
            row_schema == K1_NATURAL_EVIDENCE_ROW_SCHEMA_V1
                && row_generation.is_empty()
                && freeze_generation.is_empty()
        }
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V2 | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V3 => {
            row_schema == K1_NATURAL_EVIDENCE_ROW_SCHEMA_V2
                && !row_generation.is_empty()
                && row_generation == freeze_generation
        }
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V4 | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V5 => {
            row_schema == K1_NATURAL_EVIDENCE_ROW_SCHEMA_V3
                && !row_generation.is_empty()
                && row_generation == freeze_generation
        }
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6
        | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V7
        | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8 => {
            row_schema == K1_NATURAL_EVIDENCE_ROW_SCHEMA_V4
                && !row_generation.is_empty()
                && row_generation == freeze_generation
        }
        _ => false,
    }
}

fn validate_installed_discovery_basis(freeze: &K1NaturalCandidateFreezeV1) -> Result<(), String> {
    validate_installed_discovery_basis_fields(&freeze.schema, &freeze.discovery_basis_root_sha256)
}

fn validate_installed_discovery_basis_fields(
    freeze_schema: &str,
    discovery_basis_root_sha256: &str,
) -> Result<(), String> {
    let installed = match freeze_schema {
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V3 => {
            natural_t1_discovery_basis_root_v1().map_err(str::to_owned)?
        }
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V4 => {
            natural_t1_discovery_basis_root_v2().map_err(str::to_owned)?
        }
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V5 => {
            natural_t1_discovery_basis_root_v3().map_err(str::to_owned)?
        }
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6
        | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V7
        | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8 => {
            natural_t1_discovery_basis_root_v4().map_err(str::to_owned)?
        }
        _ => return Ok(()),
    };
    if discovery_basis_root_sha256 != installed {
        return Err("k1_runtime_discovery_basis_unsupported".to_owned());
    }
    Ok(())
}

fn identification_domain_root(freeze: &K1NaturalCandidateFreezeV1) -> Result<&str, String> {
    if freeze.schema != K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8 {
        return Ok(freeze.freeze_root_sha256.as_str());
    }
    let manifest = freeze
        .identifier_causal_input_manifest
        .as_deref()
        .ok_or_else(|| "k1_exact_identifier_causal_manifest_missing".to_owned())?;
    manifest.validate().map_err(str::to_owned)?;
    Ok(manifest.opportunity_root_sha256.as_str())
}

fn selected_shape_is_compatible(selected: Option<&str>, frozen: &str) -> bool {
    selected.is_none_or(|selected| selected == frozen)
}

fn frozen_support_contains(capture_sequence: u64, support_watermark: u64) -> bool {
    capture_sequence <= support_watermark
}

fn frozen_support_manifest<'a>(
    rows: impl IntoIterator<Item = &'a K1NaturalEvidenceRowV1>,
) -> Result<String, String> {
    let mut rows = rows.into_iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        left.capture_sequence
            .cmp(&right.capture_sequence)
            .then_with(|| left.row_root_sha256.cmp(&right.row_root_sha256))
    });
    canonical_json_sha256(&(
        "nando.k1-natural-evidence-manifest.v1",
        rows.iter()
            .map(|row| row.row_root_sha256.as_str())
            .collect::<Vec<_>>(),
    ))
    .map_err(str::to_owned)
}

pub(in crate::k1_natural_scheduler_runtime) fn identification_can_freeze(
    report: &MultiSourceT1IdentificationV3,
) -> bool {
    !report.remaining_semantic_class_roots_sha256.is_empty()
        && report.support_manifest_root_sha256.is_some()
        && matches!(
            report.state,
            MultiSourceT1IdentificationStateV1::Ambiguous
                | MultiSourceT1IdentificationStateV1::FrozenAwaitingIndependentFuture
                | MultiSourceT1IdentificationStateV1::TransferReady
        )
}

pub(in crate::k1_natural_scheduler_runtime) fn seal_identification_freeze(
    candidate: &K1NaturalCandidateFreezeV1,
    report: &MultiSourceT1IdentificationV3,
    prediction_schema: &str,
) -> Result<K1IdentificationFreezeV1, String> {
    let quotient = canonical_json_sha256(&(
        K1_SEMANTIC_QUOTIENT_SCHEMA_V1,
        report.selected_shape_root_sha256.as_deref(),
        report.selected_protocol_mode_root_sha256.as_deref(),
        &report.remaining_semantic_class_roots_sha256,
    ))
    .map_err(str::to_owned)?;
    let probe_policy = canonical_json_sha256(&(
        K1_PROBE_POLICY_SCHEMA_V1,
        report
            .passive_probe
            .as_ref()
            .map(|probe| probe.probe_root_sha256.as_str()),
        report
            .passive_probe
            .as_ref()
            .map(|probe| probe.precommitted_predictions_root_sha256.as_str()),
        &report.remaining_semantic_class_roots_sha256,
    ))
    .map_err(str::to_owned)?;
    K1IdentificationFreezeV1::seal(
        candidate,
        report
            .support_manifest_root_sha256
            .clone()
            .ok_or_else(|| "k1_runtime_support_manifest_missing".to_owned())?,
        candidate.generator_schema.clone(),
        report.remaining_semantic_class_roots_sha256.clone(),
        quotient,
        probe_policy,
        prediction_schema.to_owned(),
    )
    .map_err(str::to_owned)
}

pub(in crate::k1_natural_scheduler_runtime) fn current_classes(
    projection: &K1SchedulerProjectionV1,
) -> Vec<String> {
    projection.latest_probe_round.as_ref().map_or_else(
        || {
            projection
                .identification_freeze
                .as_ref()
                .map_or_else(Vec::new, |freeze| {
                    freeze.initial_semantic_class_roots_sha256.clone()
                })
        },
        |receipt| match receipt.state {
            K1ProbeRoundStateV1::ProbePending => {
                receipt.previous_semantic_class_roots_sha256.clone()
            }
            K1ProbeRoundStateV1::OutcomeApplied | K1ProbeRoundStateV1::OutcomeCensored => {
                receipt.next_semantic_class_roots_sha256.clone()
            }
        },
    )
}

pub(in crate::k1_natural_scheduler_runtime) fn k1_predictions(
    probe: &PassiveT1ProbeContractV1,
) -> Vec<K1ProbeClassPredictionV1> {
    let mut predictions = probe
        .class_partition_predictions
        .iter()
        .map(|prediction| K1ProbeClassPredictionV1 {
            class_id: prediction.class_id.as_str().to_owned(),
            outcome_partition_root_sha256: prediction.outcome_partition_root_sha256.clone(),
        })
        .collect::<Vec<_>>();
    predictions.sort();
    predictions
}

pub(in crate::k1_natural_scheduler_runtime) fn validate_pending_probe(
    pending: &K1ProbeRoundReceiptV1,
    probe: &PassiveT1ProbeContractV1,
) -> Result<(), String> {
    if pending.selected_probe_root_sha256 != probe.probe_root_sha256
        || pending.observable_difference_root_sha256 != probe.observable_difference_root_sha256
        || pending.precommitted_predictions_root_sha256
            != probe.precommitted_predictions_root_sha256
        || pending.class_partition_predictions != k1_predictions(probe)
    {
        return Err("k1_runtime_pending_probe_replay_mismatch".to_owned());
    }
    Ok(())
}

pub(in crate::k1_natural_scheduler_runtime) fn predicted_partition(
    probe: &PassiveT1ProbeContractV1,
    classes: &[String],
) -> bool {
    let classes = classes.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let mut partitions = BTreeMap::<&str, BTreeSet<&str>>::new();
    for prediction in &probe.class_partition_predictions {
        partitions
            .entry(prediction.outcome_partition_root_sha256.as_str())
            .or_default()
            .insert(prediction.class_id.as_str());
    }
    partitions
        .into_values()
        .any(|partition| partition == classes)
}

pub(in crate::k1_natural_scheduler_runtime) fn next_future_binding<'a>(
    bindings: &'a [EvidenceBinding],
    freeze: &K1NaturalCandidateFreezeV1,
    consumed_roots: &BTreeSet<String>,
    minimum_capture_sequence: u64,
    excluded_lineages: Option<&BTreeSet<String>>,
) -> Option<&'a EvidenceBinding> {
    bindings
        .iter()
        .filter(|binding| {
            binding_matches_freeze(binding, freeze)
                && !binding.row.safety_veto
                && binding.row.verified
                && binding.row.capture_sequence >= freeze.future_min_sequence
                && binding.row.capture_sequence >= minimum_capture_sequence
                && !consumed_roots.contains(binding.join_root_sha256())
                && excluded_lineages
                    .is_none_or(|lineages| !lineages.contains(&binding.row.lineage_root_sha256))
        })
        .min_by(|left, right| {
            left.row
                .capture_sequence
                .cmp(&right.row.capture_sequence)
                .then_with(|| left.row.row_root_sha256.cmp(&right.row.row_root_sha256))
        })
}

#[cfg(test)]
mod tests;
