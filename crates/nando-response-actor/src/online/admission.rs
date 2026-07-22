//! Candidate assembly and fail-closed admission prechecks for online evidence.
//!
//! Final authority remains owned by `nando-operator-admission`.

use super::*;

pub(super) type RepairedAdmissionGuard = (Vec<u64>, Vec<RelationFrame>, Vec<RelationFrame>);

pub(super) fn repair_frozen_admission_guard(
    program: &crate::ResponseProgram,
    required_atom_ids: &[u64],
    support: &[RelationFrame],
    future: &[RelationFrame],
    negatives: &[RelationFrame],
) -> Result<RepairedAdmissionGuard, String> {
    if support.len() < 32 || future.len() < 32 {
        return Err(format!(
            "guard_evidence_below_gate:support={}:future={}",
            support.len(),
            future.len()
        ));
    }
    let mut base_required = required_atom_ids.to_vec();
    base_required.sort_unstable();
    base_required.dedup();
    let frame_matches = |frame: &RelationFrame, required: &[u64]| {
        let observed = relation_frame_online_routing_atom_ids(frame);
        required
            .iter()
            .all(|atom| observed.binary_search(atom).is_ok())
    };
    let routed_negatives = negatives
        .iter()
        .filter(|frame| frame_matches(frame, &base_required))
        .collect::<Vec<_>>();
    if routed_negatives.is_empty() {
        return Ok((base_required, support.to_vec(), future.to_vec()));
    }
    let applicable_negatives = routed_negatives
        .iter()
        .copied()
        .filter(|frame| crate::synthesis::program_runtime_applicable(program, frame))
        .collect::<Vec<_>>();
    if applicable_negatives.is_empty() {
        return Ok((base_required, support.to_vec(), future.to_vec()));
    }

    let support_atoms = support
        .iter()
        .map(relation_frame_online_routing_atom_ids)
        .collect::<Vec<_>>();
    let future_atoms = future
        .iter()
        .map(relation_frame_online_routing_atom_ids)
        .collect::<Vec<_>>();
    let negative_atoms = applicable_negatives
        .iter()
        .map(|frame| relation_frame_online_routing_atom_ids(frame))
        .collect::<Vec<_>>();
    let mut frequency = BTreeMap::<u64, usize>::new();
    for observed in support_atoms.iter().chain(&future_atoms) {
        for atom in observed {
            if base_required.binary_search(atom).is_err() {
                *frequency.entry(*atom).or_default() += 1;
            }
        }
    }
    let mut ranked_atoms = frequency.into_iter().collect::<Vec<_>>();
    ranked_atoms.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    ranked_atoms.truncate(64);
    let ranked_atoms = ranked_atoms
        .into_iter()
        .map(|(atom, _)| atom)
        .collect::<Vec<_>>();
    let mut predicates = ranked_atoms
        .iter()
        .map(|atom| vec![*atom])
        .collect::<Vec<_>>();
    for (left_index, left) in ranked_atoms.iter().enumerate() {
        for right in ranked_atoms.iter().skip(left_index.saturating_add(1)) {
            predicates.push(vec![*left, *right]);
        }
    }
    let Some(best) = predicates
        .into_iter()
        .filter_map(|predicate| {
            let mut combined = base_required.clone();
            combined.extend(predicate);
            combined.sort_unstable();
            combined.dedup();
            let support_rows = support_atoms
                .iter()
                .filter(|observed| {
                    combined
                        .iter()
                        .all(|atom| observed.binary_search(atom).is_ok())
                })
                .count();
            let future_rows = future_atoms
                .iter()
                .filter(|observed| {
                    combined
                        .iter()
                        .all(|atom| observed.binary_search(atom).is_ok())
                })
                .count();
            if support_rows < 32
                || future_rows < 32
                || negative_atoms.iter().any(|observed| {
                    combined
                        .iter()
                        .all(|atom| observed.binary_search(atom).is_ok())
                })
            {
                return None;
            }
            Some((combined, support_rows, future_rows))
        })
        .max_by(|left, right| {
            left.1
                .saturating_add(left.2)
                .cmp(&right.1.saturating_add(right.2))
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.cmp(&right.2))
                .then_with(|| right.0.len().cmp(&left.0.len()))
                .then_with(|| right.0.cmp(&left.0))
        })
    else {
        // CEGIS deliberately delegates a pre-action collision that no exact
        // guard can separate to the learned anti-center. The candidate builder
        // below must still prove that Wave rejects these negatives with zero
        // false accepts before external admission sees the package.
        return Ok((base_required, support.to_vec(), future.to_vec()));
    };
    let repaired_support = support
        .iter()
        .filter(|frame| frame_matches(frame, &best.0))
        .cloned()
        .collect::<Vec<_>>();
    let repaired_future = future
        .iter()
        .filter(|frame| frame_matches(frame, &best.0))
        .cloned()
        .collect::<Vec<_>>();
    Ok((best.0, repaired_support, repaired_future))
}

pub(super) fn build_subcenter_admission_candidate(
    config: OnlineResponseMinerConfig,
    bucket: &ResponseBucket,
    required_atom_ids: &[u64],
    support: Vec<RelationFrame>,
    future: Vec<RelationFrame>,
    negatives: Vec<RelationFrame>,
    proven: Option<ProvenAdmissionProgram<'_>>,
) -> Result<OnlineResponseAdmissionCandidate, String> {
    let parent_bucket_id = stable_action_signature_bucket_id(
        &bucket.teacher_action_symbol,
        &bucket.teacher_signature_sha256,
    );
    if support.len() < 32 || future.len() < 32 || negatives.is_empty() {
        trace_subcenter_build(parent_bucket_id, required_atom_ids, "evidence_below_gate");
        return Err(format!(
            "evidence_below_gate:support={}:future={}:negatives={}",
            support.len(),
            future.len(),
            negatives.len()
        ));
    }
    let calibration_events = config.calibration_events.min(negatives.len()).max(1);
    let (program, verifier, phase_rank, exact_checks) = if let Some(proven) = proven {
        if !support
            .iter()
            .all(|frame| crate::synthesis::program_is_consistent(proven.program, frame))
        {
            trace_subcenter_build(
                parent_bucket_id,
                required_atom_ids,
                "cegis_support_mismatch",
            );
            return Err("cegis_support_mismatch".to_owned());
        }
        let verifier = match crate::synthesis::compile_independent_verifier(proven.program) {
            Ok(verifier) => verifier,
            Err(error) => {
                trace_subcenter_build(parent_bucket_id, required_atom_ids, error.code());
                return Err(format!("verifier_compile:{}", error.code()));
            }
        };
        (
            proven.program.clone(),
            verifier,
            proven.phase_rank,
            proven.exact_checks,
        )
    } else {
        let synthesized = match synthesize_response_operator(&support) {
            Ok(synthesized) => synthesized,
            Err(error) => {
                trace_subcenter_build(parent_bucket_id, required_atom_ids, error.code());
                return Err(format!("synthesis:{}", error.code()));
            }
        };
        (
            synthesized.candidate.program,
            synthesized.verifier,
            synthesized.candidate.phase_rank,
            synthesized.candidate.exact_checks,
        )
    };
    let offered_future = future.len();
    let future = future
        .into_iter()
        .filter(|frame| {
            frame.verifier_label == Some(true)
                && crate::synthesis::program_is_consistent(&program, frame)
        })
        .collect::<Vec<_>>();
    if future.len() < 32 {
        trace_subcenter_build(
            parent_bucket_id,
            required_atom_ids,
            &format!(
                "consistent_future_below_gate:{}/{}",
                future.len(),
                offered_future
            ),
        );
        return Err(format!(
            "consistent_future_below_gate:{}/{}",
            future.len(),
            offered_future
        ));
    }
    let bucket_id = stable_subcenter_bucket_id(parent_bucket_id, required_atom_ids);
    let wave_config = PhaseCenterOnlineMinerConfig {
        cells: config.cells,
        min_bucket_events: config.min_bucket_events,
        threshold_floor_micro: config.threshold_floor_micro,
        calibration_events,
        max_buckets: 1,
    };
    let mut wave =
        PhaseCenterOnlineMiner::new(wave_config).map_err(|error| format!("wave_init:{error:?}"))?;
    let mut encoder = PhaseCenterAtomEncoder::new(config.cells)
        .map_err(|error| format!("wave_encoder_init:{error:?}"))?;
    for frame in &support {
        let wave_atoms = subcenter_wave_atom_ids(frame, required_atom_ids);
        wave.train_atom_ids(&mut encoder, bucket_id, wave_atoms, true)
            .map_err(|error| format!("wave_support_train:{error:?}"))?;
    }
    for frame in negatives.iter().take(calibration_events) {
        let wave_atoms = subcenter_wave_atom_ids(frame, required_atom_ids);
        wave.train_atom_ids(&mut encoder, bucket_id, wave_atoms, false)
            .map_err(|error| format!("wave_negative_train:{error:?}"))?;
    }
    for frame in negatives.iter().take(calibration_events) {
        let wave_atoms = subcenter_wave_atom_ids(frame, required_atom_ids);
        wave.observe_atom_ids(&mut encoder, bucket_id, wave_atoms, false, false, 0, 0)
            .map_err(|error| format!("wave_negative_calibration:{error:?}"))?;
    }
    for frame in &future {
        let wave_atoms = subcenter_wave_atom_ids(frame, required_atom_ids);
        let decision = wave
            .observe_atom_ids(
                &mut encoder,
                bucket_id,
                wave_atoms,
                true,
                false,
                frame.estimated_input_tokens,
                0,
            )
            .map_err(|error| format!("wave_future_observe:{error:?}"))?;
        if decision.unique_cpu_accept_over_exact_cache {
            break;
        }
    }
    let wave_bucket = wave
        .bucket(bucket_id)
        .ok_or_else(|| "wave_bucket_missing".to_owned())?;
    if wave_bucket.rejected
        || wave_bucket.false_accepts != 0
        || !wave_bucket.is_shadow_ready(config.min_bucket_events, calibration_events)
    {
        trace_subcenter_build(parent_bucket_id, required_atom_ids, "wave_not_shadow_ready");
        return Err("wave_not_shadow_ready".to_owned());
    }
    // This is only a proof candidate. The independent admission controller
    // replays frozen future, calibrates routing, runs causal ablations, and
    // checks runtime parity before granting execution authority.
    let wave_package = wave
        .shadow_ready_package_bytes(bucket_id)
        .map_err(|error| format!("wave_package:{error:?}"))?
        .ok_or_else(|| "wave_package_missing".to_owned())?;
    let distinct_sessions = support
        .iter()
        .map(|frame| frame.session_id_sha256.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let positive_tokens = support
        .iter()
        .chain(future.iter())
        .map(|frame| frame.estimated_input_tokens)
        .sum();
    Ok(OnlineResponseAdmissionCandidate {
        candidate: OnlineResponseCandidate {
            bucket_id,
            structural_family_id: stable_subcenter_family_id(
                bucket.structural_family_id,
                required_atom_ids,
            ),
            teacher_signature_sha256: bucket.teacher_signature_sha256.clone(),
            positive_rows: support.len().saturating_add(future.len()),
            negative_rows: bucket.negative_rows,
            positive_tokens,
            negative_tokens: bucket.negative_tokens,
            distinct_sessions,
            wave_threshold_micro: wave_package.threshold_micro,
            wave_runtime_bytes: wave_package.package_info.serialized_len,
            wave_runtime_fingerprint64: wave_package.package_info.fingerprint64,
            program,
            verifier,
            phase_rank,
            exact_checks,
        },
        wave_runtime_package: wave_package.package_bytes,
        support,
        future,
        negatives,
        required_routing_atom_ids: required_atom_ids.to_vec(),
        runtime_parity_cases: Vec::new(),
        semantic_alias_edges: Vec::new(),
        semantic_evidence_receipts: Vec::new(),
        semantic_evidence_root_sha256: String::new(),
    })
}

fn subcenter_wave_atom_ids(frame: &RelationFrame, _required_atom_ids: &[u64]) -> Vec<u64> {
    relation_frame_online_routing_atom_ids(frame)
}

#[derive(Clone, Copy)]
pub(super) struct ProvenAdmissionProgram<'a> {
    pub(super) program: &'a crate::ResponseProgram,
    pub(super) phase_rank: u32,
    pub(super) exact_checks: u32,
}

pub(super) struct SelfTrainingAdmissionEvaluation {
    pub(super) ready_cohorts: usize,
    pub(super) candidates: Vec<OnlineResponseAdmissionCandidate>,
    pub(super) blockers: Vec<OnlineResponseAdmissionBlockerReport>,
}

fn trace_subcenter_build(parent_bucket_id: u32, atom_ids: &[u64], reason: &str) {
    if std::env::var_os("NANDO_ONLINE_ADMISSION_TRACE").is_some() {
        eprintln!(
            "online_subcenter_build parent={parent_bucket_id} atoms={atom_ids:?} blocker={reason}"
        );
    }
}

fn stable_action_signature_bucket_id(action: &str, signature: &str) -> u32 {
    let digest = Sha256::digest(
        serde_json::to_vec(&("nando.online-action-signature.v1", action, signature))
            .unwrap_or_default(),
    );
    u32::from_be_bytes(digest[..4].try_into().unwrap_or([0; 4]))
}

fn stable_subcenter_bucket_id(parent_bucket_id: u32, atom_ids: &[u64]) -> u32 {
    let digest = Sha256::digest(
        serde_json::to_vec(&("nando.online-subcenter.v1", parent_bucket_id, atom_ids))
            .unwrap_or_default(),
    );
    u32::from_be_bytes(digest[..4].try_into().unwrap_or([0; 4]))
}

fn stable_subcenter_family_id(parent_family_id: u64, atom_ids: &[u64]) -> u64 {
    let digest = Sha256::digest(
        serde_json::to_vec(&(
            "nando.online-subcenter-family.v1",
            parent_family_id,
            atom_ids,
        ))
        .unwrap_or_default(),
    );
    u64::from_be_bytes(digest[..8].try_into().unwrap_or([0; 8]))
}

pub(super) fn online_admission_precheck(
    bucket: &ResponseBucket,
    learned_wave_route: Option<&crate::LearnedWaveRoute>,
) -> String {
    let negatives = bucket
        .negatives
        .iter()
        .chain(bucket.future_negatives.iter())
        .map(SharedRelationFrame::materialize)
        .collect::<Vec<_>>();
    let (support, mut frozen_future, required_routing_atom_ids) =
        clean_admission_partition(bucket, &negatives);
    let training = support
        .iter()
        .chain(negatives.iter())
        .cloned()
        .collect::<Vec<_>>();
    let mut packages = crate::compile_source_neutral_quarantine_packages(&training, true);
    if packages.len() != 1 {
        return format!("package_compile_count:{}", packages.len());
    }
    let package = &mut packages[0];
    frozen_future
        .retain(|frame| crate::frame_matches_program_action_contract(&package.program, frame));
    crate::lifecycle::apply_clean_routing_refinement(package, &support, &negatives);
    let _ = required_routing_atom_ids;
    let mut refined_support = support
        .iter()
        .filter(|frame| crate::package::relation_frame_matches_package_guard(package, frame))
        .cloned()
        .collect::<Vec<_>>();
    if refined_support.len() < 32 {
        return format!("refined_support_below_32:{}", refined_support.len());
    }
    if let Some(route) = learned_wave_route {
        package.wave_margin_micro = route.threshold_micro;
        package.learned_wave_route = Some(route.clone());
        let guard_relevant_negatives = negatives
            .iter()
            .filter(|frame| crate::package::relation_frame_matches_package_guard(package, frame))
            .cloned()
            .collect::<Vec<_>>();
        if !crate::online_admission::ensure_support_separating_learned_route(
            package,
            &refined_support,
            &guard_relevant_negatives,
        ) {
            return "learned_wave_overlap:no_support_only_separating_route".to_owned();
        }
        (refined_support, frozen_future) = crate::online_admission::phase_clean_support_future(
            package,
            &refined_support,
            &frozen_future,
            &negatives,
        );
        if refined_support.len() < 32 || frozen_future.len() < 32 {
            return format!(
                "phase_clean_rows_below_32:support={}:future={}",
                refined_support.len(),
                frozen_future.len()
            );
        }
    }
    let routed_future = frozen_future
        .iter()
        .filter(|frame| crate::relation_frame_routes_to_package(package, frame))
        .cloned()
        .collect::<Vec<_>>();
    if routed_future.len() < 32 {
        return format!("routed_future_below_32:{}", routed_future.len());
    }
    let future_wrong = routed_future
        .iter()
        .filter(|frame| {
            frame.verifier_label != Some(true)
                || !crate::frame_matches_program_action_contract(&package.program, frame)
        })
        .count();
    if future_wrong != 0 {
        return format!("future_action_contract_mismatch:{future_wrong}");
    }
    let negative_accepts = bucket
        .negatives
        .iter()
        .chain(bucket.future_negatives.iter())
        .filter(|frame| crate::relation_frame_routes_to_package(package, frame))
        .count();
    if negative_accepts != 0 {
        return format!("negative_routes_to_package:{negative_accepts}");
    }
    let causal = crate::evaluate_grounded_wave_causality(
        package,
        &refined_support,
        &routed_future,
        &negatives,
    );
    if causal.verdict != "PASS" {
        return format!(
            "causal_{}:full={}/{}:negative_accepts={}",
            causal.verdict.to_ascii_lowercase(),
            causal.full_phase_correct,
            causal.future_rows,
            causal.negative_accepts
        );
    }
    "pass_to_snapshot".to_owned()
}

fn clean_admission_partition(
    bucket: &ResponseBucket,
    negatives: &[RelationFrame],
) -> (Vec<RelationFrame>, Vec<RelationFrame>, Vec<u64>) {
    clean_admission_partition_for_ids(bucket, negatives, None)
}

pub(super) fn clean_admission_partition_for_ids(
    bucket: &ResponseBucket,
    negatives: &[RelationFrame],
    eligible_ids: Option<&BTreeSet<String>>,
) -> (Vec<RelationFrame>, Vec<RelationFrame>, Vec<u64>) {
    let mut positives = bucket
        .positives
        .iter()
        .chain(bucket.future_positives.iter())
        .filter(|frame| {
            eligible_ids.is_none_or(|eligible| eligible.contains(&frame.frame_id_sha256))
        })
        .map(SharedRelationFrame::materialize)
        .collect::<Vec<_>>();
    positives.sort_by(|left, right| {
        left.observed_at_unix_nanos
            .cmp(&right.observed_at_unix_nanos)
            .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
    });
    let mut seen_events = BTreeSet::new();
    positives.retain(|frame| seen_events.insert(frame.event_id_sha256.clone()));
    let positive_atoms = positives
        .iter()
        .map(relation_frame_online_routing_atom_ids)
        .collect::<Vec<_>>();
    let collision_atoms = negatives
        .iter()
        .map(relation_frame_online_routing_atom_ids)
        .filter(|negative| positive_atoms.iter().any(|positive| positive == negative))
        .collect::<Vec<_>>();
    positives.retain(|frame| {
        let atoms = relation_frame_online_routing_atom_ids(frame);
        collision_atoms.iter().all(|collision| collision != &atoms)
    });

    let mut counts = BTreeMap::<u64, usize>::new();
    for frame in &positives {
        for atom in relation_frame_online_routing_atom_ids(frame) {
            if collision_atoms
                .iter()
                .all(|collision| collision.binary_search(&atom).is_err())
            {
                *counts.entry(atom).or_default() += 1;
            }
        }
    }
    let separator = counts
        .into_iter()
        .filter(|(_, count)| *count >= 64)
        .max_by(|(left_atom, left_count), (right_atom, right_count)| {
            left_count
                .cmp(right_count)
                .then_with(|| right_atom.cmp(left_atom))
        })
        .map(|(atom, _)| atom);
    if let Some(separator) = separator {
        positives.retain(|frame| {
            relation_frame_online_routing_atom_ids(frame)
                .binary_search(&separator)
                .is_ok()
        });
    }
    let support = positives.iter().take(32).cloned().collect::<Vec<_>>();
    let watermark = support
        .iter()
        .map(|frame| frame.observed_at_unix_nanos)
        .max()
        .unwrap_or(0);
    let support_sessions = support
        .iter()
        .map(|frame| frame.session_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let support_intents = support
        .iter()
        .map(|frame| frame.client_intent_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let support_events = support
        .iter()
        .map(|frame| frame.event_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let future = positives
        .into_iter()
        .skip(support.len())
        .filter(|frame| frame.observed_at_unix_nanos > watermark)
        .filter(|frame| !support_sessions.contains(frame.session_id_sha256.as_str()))
        .filter(|frame| !support_intents.contains(frame.client_intent_id_sha256.as_str()))
        .filter(|frame| !support_events.contains(frame.event_id_sha256.as_str()))
        .collect::<Vec<_>>();
    let mut required = bucket.exact_guard_atom_ids.clone();
    if let Some(separator) = separator {
        required.push(separator);
    }
    required.sort_unstable();
    required.dedup();
    (support, future, required)
}
