use super::*;

pub(in crate::k1_natural_scheduler_runtime) fn frozen_support<'a>(
    bindings: &'a [EvidenceBinding],
    freeze: &K1NaturalCandidateFreezeV1,
) -> Result<Vec<&'a EvidenceBinding>, String> {
    let support = bindings
        .iter()
        .filter(|binding| {
            binding_matches_freeze(binding, freeze)
                && !binding.row.safety_veto
                && frozen_support_contains(binding.row.capture_sequence, freeze.support_watermark)
        })
        .collect::<Vec<_>>();
    let manifest = frozen_support_manifest(support.iter().map(|binding| &binding.row))?;
    if support.is_empty()
        || support.len() > K1_MAX_SUPPORT_ROWS_V1
        || manifest != freeze.evidence_manifest_root_sha256
    {
        return Err("k1_runtime_frozen_support_manifest_mismatch".to_owned());
    }
    Ok(support)
}

pub(in crate::k1_natural_scheduler_runtime) fn binding_matches_freeze(
    binding: &EvidenceBinding,
    freeze: &K1NaturalCandidateFreezeV1,
) -> bool {
    binding.row.candidate_structural_root_sha256 == freeze.candidate_structural_root_sha256
        && binding.row.source_neutral_topology_root_sha256
            == freeze.source_neutral_topology_root_sha256
        && binding.row.semantic_novelty_signature_root_sha256
            == freeze.semantic_novelty_signature_root_sha256
        && binding.row.consequence_type == freeze.consequence_type
}

pub(in crate::k1_natural_scheduler_runtime) fn identify_frozen_candidate(
    bindings: &[EvidenceBinding],
    frames: &[RelationFrame],
    active_protocol_mode_roots_sha256: &BTreeSet<String>,
    freeze: &K1NaturalCandidateFreezeV1,
    applied_roots: &BTreeSet<String>,
    trial_roots: &BTreeSet<String>,
) -> Result<MultiSourceT1IdentificationV3, String> {
    let mut selected = bindings
        .iter()
        .filter(|binding| {
            binding_matches_freeze(binding, freeze)
                && !binding.row.safety_veto
                && (frozen_support_contains(binding.row.capture_sequence, freeze.support_watermark)
                    || applied_roots.contains(&binding.joined.join_root_sha256)
                    || trial_roots.contains(&binding.joined.join_root_sha256))
        })
        .map(|binding| binding.joined.clone())
        .collect::<Vec<_>>();
    selected.sort_by(|left, right| {
        left.capture_sequence
            .cmp(&right.capture_sequence)
            .then_with(|| left.join_root_sha256.cmp(&right.join_root_sha256))
    });
    let epoch = canonical_json_sha256(&(
        "nando.k1-frozen-identification-evidence.v1",
        freeze.freeze_root_sha256.as_str(),
        selected
            .iter()
            .map(|row| row.join_root_sha256.as_str())
            .collect::<Vec<_>>(),
    ))
    .map_err(str::to_owned)?;
    let report = identify_multi_source_t1_operator_with_active_protocols_v1(
        &selected,
        frames,
        &BTreeSet::new(),
        active_protocol_mode_roots_sha256,
        epoch,
    );
    if !report.validate()
        || report.selected_shape_root_sha256.as_deref()
            != Some(freeze.candidate_structural_root_sha256.as_str())
    {
        return Err("k1_runtime_identification_report_invalid".to_owned());
    }
    Ok(report)
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
        MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V2.to_owned(),
        report.remaining_semantic_class_roots_sha256.clone(),
        quotient,
        probe_policy,
        K1_PREDICTION_SCHEMA_V1.to_owned(),
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
                && !consumed_roots.contains(&binding.joined.join_root_sha256)
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
