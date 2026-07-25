//! Checkpoint migration, program-pool normalization, and bounded evidence compaction.

use super::*;

pub(super) fn support_blocker_requires_subcenter_split(blocker: Option<&str>) -> bool {
    matches!(
        blocker,
        Some(
            "support_program_cover_empty"
                | "support_program_cover_incomplete"
                | "support_layout_adapter_unproven"
                | "support_phase_adapter_unproven"
                | "support_consensus_variant_budget_exceeded"
                | "support_consensus_authority_unproven"
        )
    )
}

pub(super) fn migrate_collection_keyed_layouts(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<(), String> {
    for bucket in &mut checkpoint.buckets {
        let layouts = bucket
            .runtime_examples
            .iter()
            .map(|(evidence_id, example)| {
                structural_layout_sha256(&example.provider_payload)
                    .map(|layout| (evidence_id.clone(), layout))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        for receipt in bucket.support.iter_mut().chain(bucket.future.iter_mut()) {
            if let Some(layout) = layouts.get(&receipt.evidence_graph_sha256) {
                receipt.layout_sha256.clone_from(layout);
            }
        }
    }
    Ok(())
}

pub(super) fn rotate_unbound_adaptive_collection_generations(
    checkpoint: &mut OnlineCollectionCheckpoint,
) {
    let mut empty_bucket_ids = BTreeSet::new();
    for bucket in &mut checkpoint.buckets {
        let has_unbound_authority_evidence = bucket
            .support
            .iter()
            .chain(&bucket.future)
            .any(|receipt| receipt.capture_binding.is_none());
        if !has_unbound_authority_evidence {
            continue;
        }
        checkpoint.unreplayable_support_discarded_total = checkpoint
            .unreplayable_support_discarded_total
            .saturating_add(
                u64::try_from(bucket.support.len().saturating_add(bucket.future.len()))
                    .unwrap_or(u64::MAX),
            );
        // Candidate programs remain a bounded hypothesis prior. Every field
        // that can influence authority is rebuilt from fresh capture-bound rows.
        bucket.common_request_atom_ids.clear();
        bucket.support.clear();
        bucket.future.clear();
        bucket.runtime_examples.clear();
        bucket.durable_adapter_phase_atoms.clear();
        bucket.durable_runtime_parity_receipts.clear();
        bucket.adaptive_candidate_freeze = None;
        bucket.frozen_program_sha256 = None;
        bucket.support_watermark_event_time_unix_nanos = None;
        bucket.support_manifest_sha256 = None;
        bucket.rejected_program_sha256.clear();
        bucket.learned_anti_atom_ids.clear();
        bucket.wrong_accepts = 0;
        checkpoint
            .structural_resynthesis_pending_bucket_ids
            .remove(&bucket.bucket_id);
        if bucket.programs.is_empty() {
            empty_bucket_ids.insert(bucket.bucket_id.clone());
        }
    }
    if !empty_bucket_ids.is_empty() {
        // A rejected-only legacy bucket has no hypothesis left after its
        // unbound evidence is retired, so it cannot seed a fresh generation.
        checkpoint
            .buckets
            .retain(|bucket| !empty_bucket_ids.contains(&bucket.bucket_id));
        checkpoint
            .applicability_negative_sessions
            .retain(|bucket_id, _| !empty_bucket_ids.contains(bucket_id));
    }
}

pub(super) fn repair_empty_adaptive_phase_seeds(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> bool {
    if checkpoint.config.proof_mode != OnlineCollectionProofMode::AdaptiveVersionSpace {
        return false;
    }
    let mut repaired = false;
    for bucket in &mut checkpoint.buckets {
        if bucket.support.is_empty() || !bucket.common_request_atom_ids.is_empty() {
            continue;
        }
        let Some(mut common) = bucket
            .support
            .first()
            .map(|receipt| durable_pre_action_atom_ids(bucket, receipt))
        else {
            continue;
        };
        for receipt in bucket.support.iter().skip(1) {
            let atoms = durable_pre_action_atom_ids(bucket, receipt);
            common.retain(|atom| atoms.contains(atom));
        }
        if !common.is_empty() {
            bucket.common_request_atom_ids = common;
            repaired = true;
        }
    }
    repaired
}

pub(super) fn repair_adaptive_frozen_routing_atoms(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<bool, String> {
    if checkpoint.config.proof_mode != OnlineCollectionProofMode::AdaptiveVersionSpace {
        return Ok(false);
    }
    let mut repaired = false;
    for bucket in &mut checkpoint.buckets {
        let Some(program_sha256) = bucket.frozen_program_sha256.clone() else {
            continue;
        };
        if bucket.adaptive_candidate_freeze.is_none() {
            continue;
        }
        let (identification, changed) = bind_frozen_program_routing_atoms(bucket, &program_sha256)?;
        if !changed {
            continue;
        }
        bucket.adaptive_candidate_freeze = Some(identification.freeze);
        bucket.support_manifest_sha256 = Some(collection_support_manifest_digest(bucket)?);
        repaired = true;
    }
    Ok(repaired)
}

pub(super) fn response_program_surface_priority(program: &ResponseProgram) -> u8 {
    let renderer = match &program.operation {
        crate::ResponseOperation::ProjectSelectedValue { renderer, .. }
        | crate::ResponseOperation::ProjectStatus { renderer, .. }
        | crate::ResponseOperation::ComposeCollection { renderer, .. } => renderer,
        _ => return 2,
    };
    u8::from(renderer.is_direct())
}

pub(super) fn decode_collection_checkpoint(
    bytes: &[u8],
) -> Result<OnlineCollectionCheckpoint, String> {
    if let Some(payload) = bytes.strip_prefix(ONLINE_COLLECTION_CHECKPOINT_MAGIC_V3) {
        return serde_cbor::from_slice(payload)
            .map_err(|error| format!("online_collection_checkpoint_decode_cbor:{error}"));
    }
    if let Some(payload) = bytes.strip_prefix(ONLINE_COLLECTION_CHECKPOINT_MAGIC_V2) {
        return serde_cbor::from_slice(payload)
            .map_err(|error| format!("online_collection_checkpoint_decode_cbor:{error}"));
    }
    serde_json::from_slice(bytes)
        .map_err(|error| format!("online_collection_checkpoint_decode_legacy_json:{error}"))
}

pub(super) fn migrate_collection_program_pools(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<(), String> {
    if checkpoint.schema != ONLINE_COLLECTION_SCHEMA_V1
        && checkpoint.schema != ONLINE_COLLECTION_SCHEMA_V2
        && checkpoint.schema != ONLINE_COLLECTION_SCHEMA_V3
    {
        return Err("online_collection_checkpoint_schema_unknown".to_owned());
    }
    let legacy_observations = checkpoint.observations_total;
    let legacy_buckets = checkpoint.buckets.len() as u64;
    let legacy_receipts = checkpoint
        .buckets
        .iter()
        .map(|bucket| bucket.support.len().saturating_add(bucket.future.len()) as u64)
        .sum::<u64>();

    // V1 accepted component matches. Those receipts cannot prove an exact CPU
    // response after the raw example has been intentionally discarded.
    checkpoint.schema = ONLINE_COLLECTION_SCHEMA_V3.to_owned();
    checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V3;
    checkpoint.observations_total = 0;
    checkpoint.duplicate_observations_total = 0;
    checkpoint.observed_evidence_graph_sha256.clear();
    checkpoint.unsupported_total = 0;
    checkpoint.ambiguous_assignment_total = 0;
    checkpoint.exact_checks_total = 0;
    checkpoint.candidates_enumerated_total = 0;
    checkpoint.full_enumerations_total = 0;
    checkpoint.version_space_intersection_checks_total = 0;
    checkpoint.guard_scheduled_buckets_total = 0;
    checkpoint.guard_pruned_buckets_total = 0;
    checkpoint.unsupported_expected_in_latest_output = 0;
    checkpoint.unsupported_expected_in_any_output = 0;
    checkpoint.unsupported_without_exact_source_span = 0;
    checkpoint.unsupported_with_scalar_overlap = 0;
    checkpoint.policy_rejected_exact_matches = 0;
    checkpoint.counterexamples_total = 0;
    checkpoint.cegis_subcenters_total = 0;
    checkpoint.revoked_candidates_total = 0;
    checkpoint.late_after_freeze_total = 0;
    checkpoint.future_intent_rejected_total = 0;
    checkpoint.frozen_route_candidates_considered_total = 0;
    checkpoint.frozen_route_anti_rejected_total = 0;
    checkpoint.frozen_route_phase_rejected_total = 0;
    checkpoint.frozen_route_verifier_rejected_total = 0;
    checkpoint.frozen_future_accepted_total = 0;
    checkpoint.exact_executable_observations_total = 0;
    checkpoint.teacher_only_observations_total = 0;
    checkpoint.program_pool_reuse_total = 0;
    checkpoint.program_pool_receipts_total = 0;
    checkpoint.legacy_partial_observations_discarded_total = checkpoint
        .legacy_partial_observations_discarded_total
        .saturating_add(legacy_observations);
    checkpoint.legacy_partial_buckets_discarded_total = checkpoint
        .legacy_partial_buckets_discarded_total
        .saturating_add(legacy_buckets);
    checkpoint.legacy_partial_receipts_discarded_total = checkpoint
        .legacy_partial_receipts_discarded_total
        .saturating_add(legacy_receipts);
    checkpoint.unreplayable_support_discarded_total = 0;
    checkpoint.buckets.clear();
    Ok(())
}

pub(super) fn migrate_collection_archetype_pools(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<(), String> {
    let mut rebuilt = Vec::new();
    for bucket in std::mem::take(&mut checkpoint.buckets) {
        if bucket.programs.is_empty() {
            continue;
        }
        if bucket.frozen_program_sha256.is_some() {
            let mut bucket = bucket;
            bucket.archetype_id = bucket
                .programs
                .values()
                .next()
                .map(response_program_archetype_id)
                .transpose()?
                .unwrap_or_else(|| format!("legacy-frozen:{}", bucket.bucket_id));
            let program_digests = bucket.programs.keys().cloned().collect::<Vec<_>>();
            for receipt in bucket.support.iter_mut().chain(bucket.future.iter_mut()) {
                if receipt.matched_program_sha256.is_empty() {
                    receipt.matched_program_sha256 = program_digests.clone();
                }
            }
            rebuilt.push(bucket);
            continue;
        }

        let mut groups = BTreeMap::<String, BTreeMap<String, ResponseProgram>>::new();
        for (digest, program) in &bucket.programs {
            groups
                .entry(response_program_archetype_id(program)?)
                .or_default()
                .insert(digest.clone(), program.clone());
        }
        for (archetype_id, programs) in groups {
            let mut migrated = bucket.clone();
            migrated.archetype_id = archetype_id.clone();
            migrated.programs =
                bounded_program_map(programs, crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS);
            let program_digests = migrated.programs.keys().cloned().collect::<Vec<_>>();
            for receipt in &mut migrated.support {
                receipt.matched_program_sha256 = program_digests.clone();
            }
            migrated.future.clear();
            migrated.runtime_examples.clear();
            migrated.durable_runtime_parity_receipts.clear();
            migrated.adaptive_candidate_freeze = None;
            migrated.frozen_program_sha256 = None;
            migrated.support_watermark_event_time_unix_nanos = None;
            migrated.support_manifest_sha256 = None;
            migrated.bucket_id =
                collection_archetype_bucket_id(&archetype_id, migrated.programs.keys())?;
            rebuilt.push(migrated);
        }
    }
    checkpoint.buckets = rebuilt;
    checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V4;
    Ok(())
}

pub(super) fn migrate_collection_exact_authority_pools(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<(), String> {
    let mut rebuilt = Vec::new();
    for bucket in std::mem::take(&mut checkpoint.buckets) {
        let mut groups = BTreeMap::<String, BTreeMap<String, ResponseProgram>>::new();
        for (digest, program) in &bucket.programs {
            groups
                .entry(response_program_archetype_id(program)?)
                .or_default()
                .insert(digest.clone(), program.clone());
        }
        for (archetype_id, programs) in groups {
            let mut migrated = bucket.clone();
            migrated.archetype_id = archetype_id.clone();
            migrated.programs =
                bounded_program_map(programs, crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS);
            checkpoint.unreplayable_support_discarded_total = checkpoint
                .unreplayable_support_discarded_total
                .saturating_add(migrated.support.len() as u64);
            migrated.support.clear();
            migrated.future.clear();
            migrated.runtime_examples.clear();
            migrated.durable_runtime_parity_receipts.clear();
            migrated.adaptive_candidate_freeze = None;
            migrated.frozen_program_sha256 = None;
            migrated.support_watermark_event_time_unix_nanos = None;
            migrated.support_manifest_sha256 = None;
            migrated.rejected_program_sha256.clear();
            migrated.learned_anti_atom_ids.clear();
            migrated.wrong_accepts = 0;
            migrated.bucket_id =
                collection_archetype_bucket_id(&archetype_id, migrated.programs.keys())?;
            rebuilt.push(migrated);
        }
    }
    checkpoint.buckets = rebuilt;
    checkpoint.exact_executable_observations_total = 0;
    checkpoint.teacher_only_observations_total = checkpoint.observations_total;
    checkpoint.unsupported_total = checkpoint.observations_total;
    checkpoint.ambiguous_assignment_total = 0;
    checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V5;
    Ok(())
}

pub(super) fn migrate_collection_renderer_consensus_pools(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<(), String> {
    type RankedProgram = (ResponseProgram, usize);
    type RendererPool = (
        BTreeMap<String, RankedProgram>,
        BTreeMap<String, CollectionSynthesisExample>,
    );

    let mut preserved = Vec::new();
    let mut pools = BTreeMap::<String, RendererPool>::new();
    let mut migrated_examples = BTreeSet::new();
    for bucket in std::mem::take(&mut checkpoint.buckets) {
        if bucket.frozen_program_sha256.is_some()
            || !bucket.support.is_empty()
            || !bucket.future.is_empty()
            || bucket.runtime_examples.is_empty()
        {
            preserved.push(bucket);
            continue;
        }

        let mut bucket_produced_exact = false;
        for (evidence_id, example) in &bucket.runtime_examples {
            let Ok(space) = enumerate_source_neutral_response_programs(example) else {
                continue;
            };
            for program in space.programs.into_iter().filter(|program| {
                crate::response_program_exactly_matches_example(program, example)
                    && is_privacy_safe_online_response_program(program)
            }) {
                let archetype_id = response_program_archetype_id(&program)?;
                let digest = canonical_json_sha256(&program).map_err(str::to_owned)?;
                let (programs, examples) = pools.entry(archetype_id).or_default();
                let ranked = programs.entry(digest).or_insert((program, 0));
                ranked.1 = ranked.1.saturating_add(1);
                examples
                    .entry(evidence_id.clone())
                    .or_insert_with(|| example.clone());
                migrated_examples.insert(evidence_id.clone());
                bucket_produced_exact = true;
            }
        }
        if !bucket_produced_exact {
            preserved.push(bucket);
        }
    }

    for (archetype_id, (programs, examples)) in pools {
        let mut ranked = programs.into_iter().collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .1
                .1
                .cmp(&left.1.1)
                .then_with(|| {
                    response_program_surface_priority(&right.1.0)
                        .cmp(&response_program_surface_priority(&left.1.0))
                })
                .then_with(|| left.0.cmp(&right.0))
        });
        let programs = ranked
            .into_iter()
            .take(crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS)
            .map(|(digest, (program, _))| (digest, program))
            .collect::<BTreeMap<_, _>>();
        if programs.is_empty() {
            continue;
        }
        let bucket_id = collection_archetype_bucket_id(&archetype_id, programs.keys())?;
        let mut runtime_examples = examples;
        trim_runtime_examples(
            &mut runtime_examples,
            checkpoint.config.max_receipts_per_bucket,
        );
        preserved.push(OnlineCollectionBucket {
            bucket_id,
            archetype_id,
            programs,
            common_request_atom_ids: BTreeSet::new(),
            support: Vec::new(),
            future: Vec::new(),
            runtime_examples,
            durable_adapter_phase_atoms: BTreeMap::new(),
            durable_runtime_parity_receipts: BTreeMap::new(),
            adaptive_candidate_freeze: None,
            frozen_program_sha256: None,
            support_watermark_event_time_unix_nanos: None,
            support_manifest_sha256: None,
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        });
    }
    checkpoint.buckets = preserved;
    checkpoint.renderer_consensus_migrated_examples_total = checkpoint
        .renderer_consensus_migrated_examples_total
        .saturating_add(migrated_examples.len() as u64);
    checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V6;
    Ok(())
}

pub(super) fn repair_collection_checkpoint_accounting(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> bool {
    let mut repaired = false;
    if checkpoint.pooling_strategy_version == ONLINE_COLLECTION_POOLING_STRATEGY_V5
        && checkpoint.teacher_only_observations_total == checkpoint.observations_total
        && checkpoint.exact_executable_observations_total == 0
        && checkpoint.unsupported_total == checkpoint.observations_total
        && checkpoint.ambiguous_assignment_total > 0
    {
        checkpoint.ambiguous_assignment_total = 0;
        repaired = true;
    }
    for bucket in &mut checkpoint.buckets {
        if bucket.frozen_program_sha256.is_none() && !bucket.support.is_empty() {
            let before = bucket.support.len();
            bucket.support.retain(|receipt| {
                receipt.verifier_pass
                    && (!receipt.matched_program_sha256.is_empty()
                        || !receipt.verified_semantic_program_sha256.is_empty())
            });
            let discarded = before.saturating_sub(bucket.support.len());
            if discarded > 0 {
                checkpoint.unreplayable_support_discarded_total = checkpoint
                    .unreplayable_support_discarded_total
                    .saturating_add(discarded as u64);
                repaired = true;
            }
        }
    }
    repaired
}

pub(super) fn response_program_archetype_id(program: &ResponseProgram) -> Result<String, String> {
    let material = match &program.operation {
        crate::ResponseOperation::UniqueConsensus { variants, .. } => {
            let mut archetypes = variants
                .iter()
                .map(|variant| response_program_archetype_id(&variant.program))
                .collect::<Result<BTreeSet<_>, _>>()?;
            if archetypes.len() != 1 {
                return Err("online_collection_consensus_archetype_mismatch".to_owned());
            }
            archetypes
                .pop_first()
                .ok_or_else(|| "online_collection_consensus_archetype_empty".to_owned())?
        }
        crate::ResponseOperation::ProjectSelectedValue { .. } => "project".to_owned(),
        crate::ResponseOperation::ProjectStatus { .. } => "status".to_owned(),
        crate::ResponseOperation::ComposeCollection { steps, .. } => {
            let has_filter = steps.iter().any(|step| {
                matches!(
                    step,
                    crate::CollectionProgramStep::FilterUniqueFieldEquals { .. }
                        | crate::CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { .. }
                        | crate::CollectionProgramStep::FilterFieldEquals { .. }
                )
            });
            let has_count = steps
                .iter()
                .any(|step| matches!(step, crate::CollectionProgramStep::Count));
            let has_aggregate = steps.iter().any(|step| {
                matches!(
                    step,
                    crate::CollectionProgramStep::AggregateUniqueIntegerField { .. }
                )
            });
            match (has_filter, has_count, has_aggregate) {
                (true, true, _) => "collection:compose_filter_count".to_owned(),
                (false, true, _) => "collection:count".to_owned(),
                (true, false, _) => "collection:filter".to_owned(),
                (false, false, true) => "collection:aggregate".to_owned(),
                (false, false, false) => "collection:compose".to_owned(),
            }
        }
        _ => return Err("online_collection_program_archetype_unsupported".to_owned()),
    };
    canonical_json_sha256(&("nando.collection-archetype.v1", material)).map_err(str::to_owned)
}

pub(super) fn group_programs_by_archetype(
    programs: BTreeMap<String, ResponseProgram>,
) -> Result<Vec<ArchetypeProgramPool>, String> {
    let mut groups = BTreeMap::<String, Vec<(String, ResponseProgram)>>::new();
    for (digest, program) in programs {
        groups
            .entry(response_program_archetype_id(&program)?)
            .or_default()
            .push((digest, program));
    }
    groups
        .into_iter()
        .map(|(archetype, variants)| {
            Ok((
                archetype,
                bounded_program_map(
                    variants.into_iter().collect(),
                    MAX_NEW_ADAPTERS_PER_OBSERVATION,
                ),
            ))
        })
        .collect()
}

pub(super) fn structural_programs_for_observation(
    observation: &OnlineCollectionObservation,
) -> Result<BTreeMap<String, ResponseProgram>, String> {
    let synthesis_example = compact_active_turn_synthesis_example(&observation.example)
        .unwrap_or_else(|| observation.example.clone());
    enumerate_source_neutral_structural_response_programs(&synthesis_example)
        .map_err(str::to_owned)?
        .into_iter()
        .filter(is_privacy_safe_online_response_program)
        .filter(|program| {
            independently_verified_authority_response(program, &observation.example).is_some()
        })
        .map(|program| {
            canonical_json_sha256(&program)
                .map(|digest| (digest, program))
                .map_err(str::to_owned)
        })
        .collect()
}

pub(super) fn compact_active_turn_synthesis_example(
    example: &CollectionSynthesisExample,
) -> Option<CollectionSynthesisExample> {
    let input = example.provider_payload.get("input")?.as_array()?;
    let last_user = input
        .iter()
        .rposition(|item| item.get("role").and_then(Value::as_str) == Some("user"))?;
    if last_user == 0 {
        return None;
    }
    let mut provider_payload = example.provider_payload.clone();
    provider_payload["input"] = Value::Array(input[last_user..].to_vec());
    Some(CollectionSynthesisExample {
        provider_payload,
        expected_response: example.expected_response.clone(),
    })
}

pub(super) fn bounded_program_map(
    programs: BTreeMap<String, ResponseProgram>,
    limit: usize,
) -> BTreeMap<String, ResponseProgram> {
    let mut variants = programs.into_iter().collect::<Vec<_>>();
    let preferred_dynamic = variants
        .iter()
        .filter(|(_, program)| canonical_dynamic_role_count(program) >= 2)
        .max_by(|(left_digest, left), (right_digest, right)| {
            canonical_dynamic_role_count(left)
                .cmp(&canonical_dynamic_role_count(right))
                .then_with(|| {
                    serde_json::to_vec(right)
                        .unwrap_or_default()
                        .len()
                        .cmp(&serde_json::to_vec(left).unwrap_or_default().len())
                })
                .then_with(|| right_digest.cmp(left_digest))
        })
        .cloned();
    variants.sort_by(|(left_digest, left), (right_digest, right)| {
        response_program_surface_priority(left)
            .cmp(&response_program_surface_priority(right))
            .then_with(|| {
                serde_json::to_vec(left)
                    .unwrap_or_default()
                    .len()
                    .cmp(&serde_json::to_vec(right).unwrap_or_default().len())
            })
            .then_with(|| left_digest.cmp(right_digest))
    });
    if let Some((preferred_digest, _)) = &preferred_dynamic {
        variants.retain(|(digest, _)| digest != preferred_digest);
        variants.truncate(limit.saturating_sub(1));
        if limit > 0 {
            variants.push(preferred_dynamic.expect("preferred dynamic program"));
        }
    } else {
        variants.truncate(limit);
    }
    variants.into_iter().collect()
}

pub(super) fn buckets_share_execution_law(
    left: &OnlineCollectionBucket,
    right: &OnlineCollectionBucket,
) -> bool {
    let left_laws = left
        .programs
        .values()
        .filter_map(|program| response_law_key(program).ok())
        .collect::<BTreeSet<_>>();
    right.programs.values().any(|program| {
        response_law_key(program)
            .ok()
            .is_some_and(|law| left_laws.contains(&law))
    })
}

pub(super) fn select_program_receipt_cover(
    programs: &BTreeMap<String, ResponseProgram>,
    receipts: &[OnlineCollectionReceipt],
    budget: usize,
) -> Option<BTreeSet<String>> {
    if programs.len() <= budget {
        return Some(programs.keys().cloned().collect());
    }
    let mut coverage = BTreeMap::<String, BTreeSet<usize>>::new();
    for (index, receipt) in receipts.iter().enumerate() {
        for digest in &receipt.matched_program_sha256 {
            if programs.contains_key(digest) {
                coverage.entry(digest.clone()).or_default().insert(index);
            }
        }
    }
    let mut uncovered = (0..receipts.len()).collect::<BTreeSet<_>>();
    let mut selected = BTreeSet::<String>::new();
    while !uncovered.is_empty() && selected.len() < budget {
        let next = coverage
            .iter()
            .filter(|(digest, _)| !selected.contains(*digest))
            .map(|(digest, covered)| {
                let gain = covered.intersection(&uncovered).count();
                let program = &programs[digest];
                let bytes = serde_json::to_vec(program).map_or(usize::MAX, |value| value.len());
                (
                    gain,
                    canonical_direct_response_program(program)
                        .is_ok_and(|canonical| is_source_neutral_response_program(&canonical)),
                    canonical_dynamic_role_count(program),
                    bytes,
                    digest,
                    covered,
                )
            })
            .filter(|(gain, _, _, _, _, _)| *gain > 0)
            .max_by(|left, right| {
                left.0
                    .cmp(&right.0)
                    .then_with(|| left.1.cmp(&right.1))
                    .then_with(|| left.2.cmp(&right.2))
                    .then_with(|| right.3.cmp(&left.3))
                    .then_with(|| right.4.cmp(left.4))
            })?;
        selected.insert(next.4.clone());
        for index in next.5 {
            uncovered.remove(index);
        }
    }
    if !uncovered.is_empty() {
        return None;
    }
    let mut remainder = programs
        .iter()
        .filter(|(digest, _)| !selected.contains(*digest))
        .map(|(digest, program)| {
            (
                coverage.get(digest).map_or(0, BTreeSet::len),
                canonical_direct_response_program(program)
                    .is_ok_and(|canonical| is_source_neutral_response_program(&canonical)),
                canonical_dynamic_role_count(program),
                serde_json::to_vec(program).map_or(usize::MAX, |value| value.len()),
                digest.clone(),
            )
        })
        .collect::<Vec<_>>();
    remainder.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| left.3.cmp(&right.3))
            .then_with(|| left.4.cmp(&right.4))
    });
    for (_, _, _, _, digest) in remainder {
        if selected.len() >= budget {
            break;
        }
        selected.insert(digest);
    }
    Some(selected)
}

pub(super) fn canonical_dynamic_role_count(program: &ResponseProgram) -> usize {
    let Ok(canonical) = canonical_direct_response_program(program) else {
        return 0;
    };
    let renderer = match canonical.operation {
        crate::ResponseOperation::ProjectSelectedValue { renderer, .. }
        | crate::ResponseOperation::ProjectStatus { renderer, .. }
        | crate::ResponseOperation::ComposeCollection { renderer, .. } => renderer,
        _ => return 0,
    };
    match renderer {
        crate::CollectionOutputRenderer::RenderSequence { segments } => segments
            .iter()
            .filter(|segment| !matches!(segment, crate::ResponseRenderSegment::Static { .. }))
            .count(),
        crate::CollectionOutputRenderer::Direct => 1,
        _ => 0,
    }
}

pub(super) fn collection_archetype_bucket_id<'a>(
    archetype_id: &str,
    program_digests: impl IntoIterator<Item = &'a String>,
) -> Result<String, String> {
    let digests = program_digests.into_iter().cloned().collect::<Vec<_>>();
    canonical_json_sha256(&("nando.collection-archetype-pool.v1", archetype_id, digests))
        .map_err(str::to_owned)
}

pub(super) fn insert_runtime_example(
    bucket: &mut OnlineCollectionBucket,
    observation: &OnlineCollectionObservation,
    limit: usize,
) {
    insert_runtime_example_for_evidence(
        bucket,
        &observation.evidence_graph_sha256,
        observation,
        limit,
    );
}

pub(super) fn insert_runtime_example_for_evidence(
    bucket: &mut OnlineCollectionBucket,
    evidence_id: &str,
    observation: &OnlineCollectionObservation,
    limit: usize,
) {
    let stored_example =
        compact_runtime_example(bucket, observation).unwrap_or_else(|| observation.example.clone());
    bucket
        .runtime_examples
        .insert(evidence_id.to_owned(), stored_example);
    trim_bucket_runtime_examples(bucket, limit);
}

pub(super) fn trim_bucket_runtime_examples(bucket: &mut OnlineCollectionBucket, limit: usize) {
    let best_law_key = best_bucket_law_key(bucket);
    while bucket.runtime_examples.len() > limit
        || persisted_runtime_example_bytes(&bucket.runtime_examples)
            > MAX_PERSISTED_PARITY_BYTES_PER_BUCKET
    {
        let candidate = bucket
            .runtime_examples
            .iter()
            .max_by(|(left_id, left), (right_id, right)| {
                let left_outside_best = best_law_key
                    .as_ref()
                    .is_some_and(|law_key| !receipt_supports_law(bucket, left_id, law_key));
                let right_outside_best = best_law_key
                    .as_ref()
                    .is_some_and(|law_key| !receipt_supports_law(bucket, right_id, law_key));
                left_outside_best
                    .cmp(&right_outside_best)
                    .then_with(|| {
                        persisted_runtime_example_size(left_id, left)
                            .cmp(&persisted_runtime_example_size(right_id, right))
                    })
                    .then_with(|| left_id.cmp(right_id))
            })
            .map(|(evidence_id, _)| evidence_id.clone());
        let Some(candidate) = candidate else {
            break;
        };
        bucket.runtime_examples.remove(&candidate);
    }
}

pub(super) fn refresh_durable_adapter_phase_atoms(bucket: &mut OnlineCollectionBucket) {
    let support_refs = bucket
        .support
        .iter()
        .map(|receipt| receipt.evidence_graph_sha256.as_str())
        .collect::<BTreeSet<_>>();
    bucket
        .durable_adapter_phase_atoms
        .retain(|evidence, _| support_refs.contains(evidence.as_str()));
    for atoms_by_program in bucket.durable_adapter_phase_atoms.values_mut() {
        atoms_by_program.retain(|program_sha256, _| bucket.programs.contains_key(program_sha256));
    }
    for (evidence, example) in &bucket.runtime_examples {
        if !support_refs.contains(evidence.as_str()) {
            continue;
        }
        let atoms_by_program = bucket
            .durable_adapter_phase_atoms
            .entry(evidence.clone())
            .or_default();
        for (program_sha256, program) in &bucket.programs {
            let mut atoms =
                crate::runtime::actor_adapter_phase_atom_ids(program, &example.provider_payload);
            atoms.sort_unstable();
            atoms.dedup();
            if atoms.is_empty() || atoms.len() > MAX_DURABLE_ADAPTER_PHASE_ATOMS {
                atoms_by_program.remove(program_sha256);
            } else {
                atoms_by_program.insert(program_sha256.clone(), atoms);
            }
        }
    }
    bucket
        .durable_adapter_phase_atoms
        .retain(|_, atoms_by_program| !atoms_by_program.is_empty());
}

pub(super) fn durable_adapter_phase_subset(
    bucket: &OnlineCollectionBucket,
    evidence_ids: &BTreeSet<String>,
    program_ids: &BTreeSet<String>,
) -> BTreeMap<String, BTreeMap<String, Vec<u64>>> {
    bucket
        .durable_adapter_phase_atoms
        .iter()
        .filter(|(evidence_id, _)| evidence_ids.contains(*evidence_id))
        .filter_map(|(evidence_id, atoms_by_program)| {
            let retained = atoms_by_program
                .iter()
                .filter(|(program_sha256, _)| program_ids.contains(*program_sha256))
                .map(|(program_sha256, atoms)| (program_sha256.clone(), atoms.clone()))
                .collect::<BTreeMap<_, _>>();
            (!retained.is_empty()).then(|| (evidence_id.clone(), retained))
        })
        .collect()
}

pub(super) fn best_bucket_law_key(bucket: &OnlineCollectionBucket) -> Option<Vec<u8>> {
    let digest_law_keys = bucket
        .programs
        .iter()
        .filter_map(|(digest, program)| {
            response_law_key(program)
                .ok()
                .map(|law_key| (digest.as_str(), law_key))
        })
        .collect::<BTreeMap<_, _>>();
    let mut support = BTreeMap::<Vec<u8>, usize>::new();
    for receipt in &bucket.support {
        let receipt_laws = receipt
            .matched_program_sha256
            .iter()
            .filter_map(|digest| digest_law_keys.get(digest.as_str()).cloned())
            .collect::<BTreeSet<_>>();
        for law_key in receipt_laws {
            *support.entry(law_key).or_default() += 1;
        }
    }
    support
        .into_iter()
        .max_by(|(left_key, left_rows), (right_key, right_rows)| {
            left_rows
                .cmp(right_rows)
                .then_with(|| right_key.cmp(left_key))
        })
        .map(|(law_key, _)| law_key)
}

pub(super) fn receipt_supports_law(
    bucket: &OnlineCollectionBucket,
    evidence_id: &str,
    law_key: &[u8],
) -> bool {
    bucket
        .support
        .iter()
        .find(|receipt| receipt.evidence_graph_sha256 == evidence_id)
        .is_some_and(|receipt| {
            receipt.matched_program_sha256.iter().any(|digest| {
                bucket
                    .programs
                    .get(digest)
                    .and_then(|program| response_law_key(program).ok())
                    .is_some_and(|candidate| candidate == law_key)
            })
        })
}

pub(super) fn compact_runtime_example(
    bucket: &OnlineCollectionBucket,
    observation: &OnlineCollectionObservation,
) -> Option<CollectionSynthesisExample> {
    let input = observation
        .example
        .provider_payload
        .get("input")?
        .as_array()?;
    let last_user = input
        .iter()
        .rposition(|item| item.get("role").and_then(Value::as_str) == Some("user"));
    let compact_input = input
        .iter()
        .enumerate()
        .filter(|(index, item)| {
            Some(*index) == last_user
                || matches!(
                    item.get("type").and_then(Value::as_str),
                    Some("function_call_output" | "custom_tool_call_output")
                )
        })
        .map(|(_, item)| item.clone())
        .collect::<Vec<_>>();
    if compact_input.is_empty() || compact_input.len() == input.len() {
        return None;
    }
    let compact = CollectionSynthesisExample {
        provider_payload: serde_json::json!({"input": compact_input}),
        expected_response: observation.example.expected_response.clone(),
    };
    let matched_digests = bucket
        .support
        .iter()
        .find(|receipt| receipt.evidence_graph_sha256 == observation.evidence_graph_sha256)
        .map(|receipt| &receipt.matched_program_sha256)?;
    if matched_digests.is_empty() {
        return None;
    }
    for digest in matched_digests {
        let program = bucket.programs.get(digest)?;
        let full_response =
            independently_verified_authority_response(program, &observation.example)?;
        let compact_response = independently_verified_authority_response(program, &compact)?;
        if compact_response != full_response {
            return None;
        }
    }
    Some(compact)
}

pub(super) fn trim_runtime_examples(
    examples: &mut BTreeMap<String, CollectionSynthesisExample>,
    limit: usize,
) {
    while examples.len() > limit
        || persisted_runtime_example_bytes(examples) > MAX_PERSISTED_PARITY_BYTES_PER_BUCKET
    {
        let Some(oldest) = examples.keys().next().cloned() else {
            break;
        };
        examples.remove(&oldest);
    }
}

pub(super) fn persisted_runtime_example_bytes(
    examples: &BTreeMap<String, CollectionSynthesisExample>,
) -> usize {
    examples
        .iter()
        .map(|(digest, example)| persisted_runtime_example_size(digest, example))
        .fold(0_usize, usize::saturating_add)
}

pub(super) fn persisted_runtime_example_size(
    digest: &str,
    example: &CollectionSynthesisExample,
) -> usize {
    digest
        .len()
        .saturating_add(serde_cbor::to_vec(example).map_or(0, |bytes| bytes.len()))
}

pub(super) enum UnsupportedSourceSpan {
    Latest,
    Earlier,
    Missing,
}

pub(super) fn unsupported_source_span(
    example: &CollectionSynthesisExample,
) -> UnsupportedSourceSpan {
    let outputs = example
        .provider_payload
        .get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| {
            matches!(
                item.get("type").and_then(Value::as_str),
                Some("function_call_output" | "custom_tool_call_output")
            )
        })
        .filter_map(|item| item.get("output").and_then(Value::as_str))
        .collect::<Vec<_>>();
    if outputs
        .last()
        .is_some_and(|output| output.contains(&example.expected_response))
    {
        UnsupportedSourceSpan::Latest
    } else if outputs
        .iter()
        .any(|output| output.contains(&example.expected_response))
    {
        UnsupportedSourceSpan::Earlier
    } else {
        UnsupportedSourceSpan::Missing
    }
}

pub(super) fn has_scalar_overlap(example: &CollectionSynthesisExample) -> bool {
    example
        .provider_payload
        .get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("output").and_then(Value::as_str))
        .any(|output| {
            let mut scalars = Vec::new();
            if let Ok(value) = serde_json::from_str::<Value>(output) {
                collect_scalar_strings(&value, &mut scalars);
            }
            scalars.extend(
                output
                    .split(|character: char| {
                        character.is_whitespace()
                            || matches!(character, ':' | '=' | ',' | ';' | '[' | ']' | '{' | '}')
                    })
                    .filter(|value| value.len() >= 2 && value.len() <= 128)
                    .map(str::to_owned),
            );
            scalars
                .iter()
                .any(|scalar| example.expected_response.contains(scalar))
        })
}

pub(super) fn collect_scalar_strings(value: &Value, output: &mut Vec<String>) {
    match value {
        Value::Null => {}
        Value::Bool(value) => output.push(value.to_string()),
        Value::Number(value) => output.push(value.to_string()),
        Value::String(value) if value.len() >= 2 && value.len() <= 128 => {
            output.push(value.clone());
        }
        Value::Array(values) => {
            for value in values {
                collect_scalar_strings(value, output);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                collect_scalar_strings(value, output);
            }
        }
        Value::String(_) => {}
    }
}
