#[path = "authority.rs"]
mod authority;

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use nando_response_actor::{
    AtomSource, AtomValueType, COLLECTION_EXTERNAL_VERIFIER_SCHEMA, CollectionSynthesisExample,
    FrameRepresentationPolicy, GROUNDED_RESPONSE_PACKAGE_PREFIX,
    RESPONSE_FUTURE_VERIFIER_RECEIPT_SCHEMA_V2, RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2,
    RESPONSE_RUNTIME_PARITY_RECEIPT_SET_SCHEMA_V1, RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1,
    ROUTING_REFINEMENT_VERSION, RelationAtom, RelationFrame, ResponseExecutionStatus,
    ResponseOperation, ResponsePackage, ResponsePackageOrigin, ResponsePackageProof,
    ResponsePackageState, ResponseRegistry, ResponseRelationObservation, ResponseShadowObservation,
    ResponseSupportFreezePolicy, ResponseSupportManifest, ResponseSupportManifestSet,
    ResponseValueSelector, SOURCE_VALUE_EXTERNAL_VERIFIER_SCHEMA,
    VALUE_PROJECTION_EXTERNAL_VERIFIER_SCHEMA, canonical_json_sha256, compile_response_registry,
    compile_source_neutral_quarantine_packages, evaluate_grounded_wave_causality_refs,
    execute_response, frame_matches_program_action_contract_with_grounding,
    freeze_source_neutral_support, freeze_source_neutral_support_with_policy, ground_roles,
    is_source_neutral_relation_frame, partition_teacher_training_families,
    relation_frame_phase_margin_micro, relation_frame_routes_to_package,
    relation_frame_routing_atom_ids, response_actor_program_digest,
    response_independent_verifier_program_digest, response_package_digest,
    response_package_lineage_id, response_program_external_verifier_schema,
    response_program_required_routing_atom_ids, response_support_manifest_digest,
    synthesize_response_operator, synthesize_unique_collection_program, verify_operator_structure,
    verify_response_independently,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use authority::{
    aggregate_causal_verdict, compile_runtime_registry, package_receipt_sets,
    response_authority_candidate,
};

const SELF_TRAINING_MIN_VERIFIED_FUTURE_ROWS: usize = 64;
const SELF_TRAINING_MIN_VERIFIED_FUTURE_SESSIONS: usize = 6;
const SELF_TRAINING_RESERVED_FUTURE_SESSIONS: usize = 3;
const SELF_TRAINING_MIN_ROLLOVER_ROWS: usize = 32;

#[cfg(test)]
use nando_response_actor::{
    RESPONSE_AUTHORITY_SCHEMA_V2, RESPONSE_REGISTRY_SCHEMA_V6, ResponsePackageAuthorityBindingV2,
    response_registry_digest,
};

pub(super) fn main() {
    if let Err(error) = run() {
        eprintln!("nando-response-miner: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    run_with_args(&args)
}

fn run_with_args(args: &[PathBuf]) -> Result<(), String> {
    let cycle_started = Instant::now();
    let state = Path::new("/var/lib/nando-wave/transition");
    let relations_path = args
        .first()
        .cloned()
        .unwrap_or_else(|| state.join("response-relations.jsonl"));
    let shadows_path = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| state.join("response-shadows.jsonl"));
    let causal_path = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| state.join("response-wave-causal-proof.json"));
    let registry_path = args
        .get(3)
        .cloned()
        .unwrap_or_else(|| state.join("response-registry.json"));
    let status_path = args
        .get(4)
        .cloned()
        .unwrap_or_else(|| state.join("response-miner-status.json"));
    let relation_frames_path = args
        .get(5)
        .cloned()
        .unwrap_or_else(|| state.join("response-relation-frames.jsonl"));
    let support_manifests_path = args
        .get(6)
        .cloned()
        .unwrap_or_else(|| state.join("response-support-manifests.json"));
    let verifier_receipts_path = args
        .get(7)
        .cloned()
        .unwrap_or_else(|| state.join("response-verifier-receipts.json"));
    let grounded_causal_path = args
        .get(8)
        .cloned()
        .unwrap_or_else(|| state.join("response-grounded-wave-causal-proof.json"));
    let runtime_parity_receipts_path = args
        .get(9)
        .cloned()
        .unwrap_or_else(|| status_path.with_file_name("response-runtime-parity-receipts.json"));
    let input_fingerprint_sha256 = miner_input_fingerprint(&[
        &relations_path,
        &shadows_path,
        &causal_path,
        &relation_frames_path,
        &support_manifests_path,
    ])?;
    if refresh_idle_miner_status(
        &status_path,
        &input_fingerprint_sha256,
        cycle_started.elapsed().as_millis() as u64,
    )? {
        return Ok(());
    }
    let relations = read_json_lines::<ResponseRelationObservation>(&relations_path)?;
    let shadows = read_json_lines::<ResponseShadowObservation>(&shadows_path)?;
    let (raw_relation_frames, cold_collection_rows) =
        read_relation_frame_input(&relation_frames_path)?;
    let raw_relation_frame_rows = raw_relation_frames.len();
    let (
        unique_relation_frames,
        relation_frame_duplicate_rows,
        relation_frame_conflicting_duplicate_ids,
    ) = dedupe_relation_frames(raw_relation_frames);
    let unique_relation_frame_rows = unique_relation_frames.len();
    let relation_frames = unique_relation_frames
        .into_iter()
        .filter(is_source_neutral_relation_frame)
        .collect::<Vec<_>>();
    let grounded_family_by_frame_id = relation_frames
        .iter()
        .filter_map(|frame| {
            let hypotheses = ground_roles(frame);
            (hypotheses.len() == 1 && hypotheses[0].competing_binding_count == 0)
                .then(|| (frame.frame_id_sha256.clone(), hypotheses[0].frame_family_id))
        })
        .collect::<BTreeMap<_, _>>();
    let legacy_relation_frames_ignored =
        unique_relation_frame_rows.saturating_sub(relation_frames.len());
    let lifecycle_relation_frames = relation_frames
        .iter()
        .filter(|frame| project_status_response_shape_is_valid(frame))
        .cloned()
        .collect::<Vec<_>>();
    let ambiguous_frames = lifecycle_relation_frames
        .iter()
        .filter(|frame| !grounded_family_by_frame_id.contains_key(&frame.frame_id_sha256))
        .count();
    let grounded_families = partition_teacher_training_families(&lifecycle_relation_frames);
    let grounded_family_reports = grounded_families
        .iter()
        .map(|((family_id, _teacher_signature), frames)| grounded_family_report(*family_id, frames))
        .collect::<Vec<_>>();
    let mut synthesized_operators = Vec::new();
    let mut synthesis_failures = Vec::new();
    for ((family_id, teacher_signature), frames) in &grounded_families {
        match synthesize_response_operator(frames) {
            Ok(operator) => synthesized_operators.push(operator),
            Err(error) => synthesis_failures.push(serde_json::json!({
                "family_id": family_id,
                "teacher_signature_sha256": teacher_signature,
                "rows": frames.len(),
                "positive_rows": frames
                    .iter()
                    .filter(|frame| frame.verifier_label == Some(true))
                    .count(),
                "negative_rows": frames
                    .iter()
                    .filter(|frame| frame.verifier_label == Some(false))
                    .count(),
                "has_custom_tool_action": frames.iter().any(|frame| frame.atoms.iter().any(
                    |atom| matches!(atom, RelationAtom::ActionCustomTool { .. })
                )),
                "has_function_action": frames.iter().any(|frame| frame.atoms.iter().any(
                    |atom| matches!(atom, RelationAtom::ActionFunction { .. })
                )),
                "reason": error.code(),
            })),
        }
    }
    let synthesized_candidates = synthesized_operators.len();
    let generic_function_call_candidates = synthesized_operators
        .iter()
        .filter(|operator| {
            matches!(
                operator.candidate.program.operation,
                ResponseOperation::FunctionCallFromRoles { .. }
            )
        })
        .count();
    let value_projection_candidates = synthesized_operators
        .iter()
        .filter(|operator| {
            matches!(
                operator.candidate.program.operation,
                ResponseOperation::ProjectSelectedValue { .. }
            )
        })
        .count();
    let status_projection_candidates = synthesized_operators
        .iter()
        .filter(|operator| {
            matches!(
                operator.candidate.program.operation,
                ResponseOperation::ProjectStatus { .. }
            )
        })
        .count();
    let token_opportunity = token_opportunity_report(&relation_frames);
    let synthesis_exact_checks = synthesized_operators
        .iter()
        .map(|operator| operator.candidate.exact_checks as u64)
        .sum::<u64>();
    let synthesis_description_bytes = synthesized_operators
        .iter()
        .map(|operator| operator.candidate.description_length_bytes as u64)
        .sum::<u64>();
    let revision = read_registry_revision(&registry_path).saturating_add(1);
    let wave_causal_pass = causal_proof_passes(&causal_path);
    let mut support_manifests = read_json::<ResponseSupportManifestSet>(&support_manifests_path)
        .filter(|set| set.schema == "nando.response-support-manifest-set.v1")
        .unwrap_or_else(|| ResponseSupportManifestSet {
            schema: "nando.response-support-manifest-set.v1".to_owned(),
            manifests: Vec::new(),
        });
    let historical_support_manifest_count = support_manifests.manifests.len();
    let removed_manifests = compact_live_support_manifests(&mut support_manifests.manifests, 2);
    if !removed_manifests.is_empty() {
        archive_support_manifests(&support_manifests_path, &removed_manifests)?;
        atomic_write_json(&support_manifests_path, &support_manifests)?;
    }
    let discovered_manifests = freeze_source_neutral_support(
        &lifecycle_relation_frames,
        unix_now().saturating_mul(1_000_000_000),
        wave_causal_pass,
    );
    let known_packages = support_manifests
        .manifests
        .iter()
        .map(|manifest| manifest.package_id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let new_manifests = discovered_manifests
        .manifests
        .into_iter()
        .filter(|manifest| !known_packages.contains(&manifest.package_id))
        .collect::<Vec<_>>();
    if !new_manifests.is_empty() {
        support_manifests.manifests.extend(new_manifests);
        atomic_write_json(&support_manifests_path, &support_manifests)?;
    }
    let mut current_support_manifests =
        latest_grounded_support_manifests(&support_manifests.manifests);
    let collection_families = collection_families(&cold_collection_rows);
    let mut collection_manifests = support_manifests
        .manifests
        .iter()
        .filter(|manifest| manifest.package_id.starts_with("raw-phase-collection-"))
        .map(|manifest| (manifest.package_id.clone(), manifest.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut new_collection_manifests = Vec::new();
    for family in &collection_families {
        let Some(preliminary) = compile_collection_quarantine_package(family) else {
            continue;
        };
        if collection_manifests.contains_key(&preliminary.package_id) {
            continue;
        }
        if let Some(manifest) = build_collection_support_manifest(family, &preliminary) {
            collection_manifests.insert(manifest.package_id.clone(), manifest.clone());
            new_collection_manifests.push(manifest);
        }
    }
    if !new_collection_manifests.is_empty() {
        support_manifests.manifests.extend(new_collection_manifests);
        atomic_write_json(&support_manifests_path, &support_manifests)?;
    }
    current_support_manifests.extend(collection_manifests.values().cloned());
    let frozen_frame_ids = current_support_manifests
        .iter()
        .flat_map(|manifest| manifest.support_frame_ids.iter())
        .collect::<std::collections::BTreeSet<_>>();
    let frozen_support = relation_frames
        .iter()
        .filter(|frame| frozen_frame_ids.contains(&frame.frame_id_sha256))
        .cloned()
        .collect::<Vec<_>>();
    let legacy_shadow_registry =
        compile_response_registry(revision, &relations, &shadows, wave_causal_pass);
    let grounded_packages = current_support_manifests
        .iter()
        .filter(|manifest| {
            manifest
                .package_id
                .starts_with(GROUNDED_RESPONSE_PACKAGE_PREFIX)
        })
        .filter_map(|manifest| {
            let support_ids = manifest
                .support_frame_ids
                .iter()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>();
            let manifest_support = relation_frames
                .iter()
                .filter(|frame| support_ids.contains(frame.frame_id_sha256.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            let mut package =
                compile_source_neutral_quarantine_packages(&manifest_support, wave_causal_pass)
                    .into_iter()
                    .max_by_key(|package| package.proof.support_rows)?;
            package.package_id = manifest.package_id.clone();
            package.phase_centers =
                manifest_runtime_phase_centers(manifest, manifest_support.as_slice());
            package.anti_centers = manifest.learned_anti_center_atom_ids.clone();
            package.routing_predicates = manifest.selected_routing_predicates.clone();
            package
                .required_routing_atom_ids
                .extend(manifest.selected_routing_atom_ids.iter().copied());
            package.required_routing_atom_ids.sort_unstable();
            package.required_routing_atom_ids.dedup();
            Some(package)
        })
        .collect::<Vec<_>>();
    let mut grounded_by_package_id = BTreeMap::<String, ResponsePackage>::new();
    for package in grounded_packages {
        match grounded_by_package_id.entry(package.package_id.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(package);
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let incumbent = entry.get();
                let replace = package.proof.support_rows > incumbent.proof.support_rows
                    || (package.proof.support_rows == incumbent.proof.support_rows
                        && response_package_digest(&package).unwrap_or_default()
                            < response_package_digest(incumbent).unwrap_or_default());
                if replace {
                    entry.insert(package);
                }
            }
        }
    }
    let mut grounded_packages = grounded_by_package_id.into_values().collect::<Vec<_>>();
    let negative_frames = relation_frames
        .iter()
        .filter(|frame| {
            frame.verifier_label == Some(false)
                || !grounded_family_by_frame_id.contains_key(&frame.frame_id_sha256)
        })
        .collect::<Vec<_>>();
    let verifier_negative_frames = relation_frames
        .iter()
        .filter(|frame| frame.verifier_label == Some(false))
        .collect::<Vec<_>>();
    let mut causal_support_by_package = BTreeMap::<String, Vec<RelationFrame>>::new();
    let mut causal_future_by_package = BTreeMap::<String, Vec<RelationFrame>>::new();
    let mut causal_negatives_by_package = BTreeMap::<String, Vec<&RelationFrame>>::new();
    let mut missing_receipts_by_package = BTreeMap::<String, usize>::new();
    let mut verifier_receipts = Vec::new();
    let mut future_frames = 0_usize;
    let mut future_wrong = 0_usize;
    let mut missing_receipts = 0_usize;
    let mut post_freeze_rows = 0_usize;
    let mut support_session_reject_rows = 0_usize;
    let mut support_intent_reject_rows = 0_usize;
    let mut independent_post_freeze_rows = 0_usize;
    let mut reserved_session_rows = 0_usize;
    let mut new_session_rows = 0_usize;
    let mut route_mismatch_rows = 0_usize;
    let mut route_unbound_rows = 0_usize;
    let mut route_margin_below_rows = 0_usize;
    let mut route_margin_min_micro: Option<i64> = None;
    let mut route_margin_max_micro: Option<i64> = None;
    let mut routed_rows = 0_usize;
    let mut verifier_accepted_rows = 0_usize;
    let mut verifier_rejected_rows = 0_usize;
    let mut package_future_eligibility = BTreeMap::<String, Value>::new();
    let mut positive_route_mismatch_sessions_by_lineage =
        BTreeMap::<String, std::collections::BTreeSet<String>>::new();
    let current_grounded_package_ids = grounded_packages
        .iter()
        .map(|package| package.package_id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let matched_support_manifests = current_support_manifests
        .iter()
        .filter(|manifest| current_grounded_package_ids.contains(&manifest.package_id))
        .count();
    let unmatched_current_support_manifests = current_support_manifests
        .len()
        .saturating_sub(matched_support_manifests);
    let grounded_packages_without_manifest = current_grounded_package_ids
        .iter()
        .filter(|package_id| {
            !current_support_manifests
                .iter()
                .any(|manifest| manifest.package_id.as_str() == package_id.as_str())
        })
        .count();
    let latest_support_boundary_unix_nanos = current_support_manifests
        .iter()
        .map(|manifest| manifest.support_boundary_unix_nanos)
        .max();
    for manifest in &support_manifests.manifests {
        if !current_grounded_package_ids.contains(&manifest.package_id) {
            continue;
        }
        let support_ids = manifest
            .support_frame_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        let support_sessions = manifest
            .support_session_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        let support_intents = manifest
            .support_intent_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        let reserved_future_sessions = manifest
            .reserved_future_session_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        let package_support = relation_frames
            .iter()
            .filter(|frame| support_ids.contains(&frame.frame_id_sha256))
            .cloned()
            .collect::<Vec<_>>();
        let Ok(operator) = synthesize_response_operator(&package_support) else {
            continue;
        };
        let support_family_id = package_support.first().and_then(|frame| {
            grounded_family_by_frame_id
                .get(&frame.frame_id_sha256)
                .copied()
        });
        causal_support_by_package.insert(manifest.package_id.clone(), package_support.clone());
        let mut future_sessions = std::collections::BTreeSet::new();
        let mut future_surfaces = std::collections::BTreeSet::new();
        let mut package_future = 0_usize;
        let mut package_wrong = 0_usize;
        let mut package_missing = 0_usize;
        let mut package_post_freeze = 0_usize;
        let mut package_support_session_reject = 0_usize;
        let mut package_support_intent_reject = 0_usize;
        let mut package_independent = 0_usize;
        let mut package_route_mismatch = 0_usize;
        let mut package_route_unbound = 0_usize;
        let mut package_route_margin_below = 0_usize;
        let mut package_routed = 0_usize;
        let mut package_family_mismatch = 0_usize;
        let mut package_label_reject = 0_usize;
        let mut package_evidence_reject = 0_usize;
        let mut package_output_hash_reject = 0_usize;
        let mut package_shape_reject = 0_usize;
        let mut package_structure_reject = 0_usize;
        let mut package_execution_reject = 0_usize;
        let routing_package = grounded_packages
            .iter()
            .find(|package| package.package_id == manifest.package_id)
            .cloned()
            .ok_or_else(|| "grounded_manifest_package_missing".to_owned())?;
        let required_verifier_schema =
            response_program_external_verifier_schema(&routing_package.program).unwrap_or("");
        let verifier_id = required_verifier_schema
            .strip_suffix(".v1")
            .unwrap_or(required_verifier_schema);
        for (future_index, frame) in relation_frames
            .iter()
            .filter(|frame| frame.observed_at_unix_nanos > manifest.created_at_unix_nanos)
            .enumerate()
        {
            post_freeze_rows = post_freeze_rows.saturating_add(1);
            package_post_freeze = package_post_freeze.saturating_add(1);
            if support_sessions.contains(&frame.session_id_sha256) {
                support_session_reject_rows = support_session_reject_rows.saturating_add(1);
                package_support_session_reject = package_support_session_reject.saturating_add(1);
                continue;
            }
            if support_intents.contains(&frame.client_intent_id_sha256) {
                support_intent_reject_rows = support_intent_reject_rows.saturating_add(1);
                package_support_intent_reject = package_support_intent_reject.saturating_add(1);
                continue;
            }
            independent_post_freeze_rows = independent_post_freeze_rows.saturating_add(1);
            package_independent = package_independent.saturating_add(1);
            if reserved_future_sessions.contains(&frame.session_id_sha256) {
                reserved_session_rows = reserved_session_rows.saturating_add(1);
            } else {
                new_session_rows = new_session_rows.saturating_add(1);
            }
            let route_margin = relation_frame_phase_margin_micro(&routing_package, frame);
            if let Some(margin) = route_margin {
                route_margin_min_micro =
                    Some(route_margin_min_micro.map_or(margin, |value| value.min(margin)));
                route_margin_max_micro =
                    Some(route_margin_max_micro.map_or(margin, |value| value.max(margin)));
            }
            if !relation_frame_routes_to_package(&routing_package, frame) {
                route_mismatch_rows = route_mismatch_rows.saturating_add(1);
                package_route_mismatch = package_route_mismatch.saturating_add(1);
                if frame.verifier_label == Some(true)
                    && support_family_id.is_some()
                    && grounded_family_by_frame_id
                        .get(&frame.frame_id_sha256)
                        .copied()
                        == support_family_id
                {
                    positive_route_mismatch_sessions_by_lineage
                        .entry(manifest.lineage_id.clone())
                        .or_default()
                        .insert(frame.session_id_sha256.clone());
                }
                if route_margin.is_some() {
                    route_margin_below_rows = route_margin_below_rows.saturating_add(1);
                    package_route_margin_below = package_route_margin_below.saturating_add(1);
                } else {
                    route_unbound_rows = route_unbound_rows.saturating_add(1);
                    package_route_unbound = package_route_unbound.saturating_add(1);
                }
                continue;
            }
            routed_rows = routed_rows.saturating_add(1);
            package_routed = package_routed.saturating_add(1);
            let evidence_present = is_sha256(&frame.evidence_ref_sha256);
            let output_sha256 = action_value_sha256(frame);
            let output_hash_required = !matches!(
                &operator.candidate.program.operation,
                ResponseOperation::ProjectStatus { .. }
            );
            let family_matches =
                support_family_id.is_some() && relation_frame_family_id(frame) == support_family_id;
            let positive_label = frame.verifier_label == Some(true);
            let verifier_payload = parity_provider_payload(&routing_package, frame, future_index);
            let verifier_execution =
                execute_response(&routing_package.program, "", &verifier_payload);
            let independently_verified_output =
                verifier_execution.response.as_deref().filter(|response| {
                    routing_package.verifier.as_ref().is_some_and(|verifier| {
                        verify_response_independently(verifier, &verifier_payload, response).is_ok()
                    })
                });
            let canonical_evidence_sha256 = canonical_json_sha256(&verifier_payload).ok();
            let canonical_output_sha256 = independently_verified_output
                .and_then(|output| canonical_json_sha256(&output).ok());
            let accepted = positive_label
                && evidence_present
                && (!output_hash_required || output_sha256.is_some())
                && family_matches
                && project_status_response_shape_is_valid(frame)
                && verify_operator_structure(frame, &operator)
                && canonical_evidence_sha256.is_some()
                && canonical_output_sha256.is_some();
            package_label_reject =
                package_label_reject.saturating_add(usize::from(!positive_label));
            package_evidence_reject =
                package_evidence_reject.saturating_add(usize::from(!evidence_present));
            package_output_hash_reject = package_output_hash_reject
                .saturating_add(usize::from(output_hash_required && output_sha256.is_none()));
            package_family_mismatch =
                package_family_mismatch.saturating_add(usize::from(!family_matches));
            package_shape_reject = package_shape_reject
                .saturating_add(usize::from(!project_status_response_shape_is_valid(frame)));
            package_structure_reject = package_structure_reject
                .saturating_add(usize::from(!verify_operator_structure(frame, &operator)));
            package_execution_reject = package_execution_reject.saturating_add(usize::from(
                canonical_evidence_sha256.is_none() || canonical_output_sha256.is_none(),
            ));
            if accepted {
                verifier_accepted_rows = verifier_accepted_rows.saturating_add(1);
                causal_future_by_package
                    .entry(manifest.package_id.clone())
                    .or_default()
                    .push(frame.clone());
            } else {
                verifier_rejected_rows = verifier_rejected_rows.saturating_add(1);
                causal_negatives_by_package
                    .entry(manifest.package_id.clone())
                    .or_default()
                    .push(frame);
            }
            if accepted {
                package_future = package_future.saturating_add(1);
                future_sessions.insert(frame.session_id_sha256.clone());
                future_surfaces.extend(frame.atoms.iter().filter_map(|atom| match atom {
                    RelationAtom::ToolKind { value } => Some(value.clone()),
                    _ => None,
                }));
            }
            package_wrong = package_wrong.saturating_add(usize::from(!positive_label || !accepted));
            package_missing = package_missing.saturating_add(usize::from(!evidence_present));
            verifier_receipts.push(serde_json::json!({
                "schema": RESPONSE_FUTURE_VERIFIER_RECEIPT_SCHEMA_V2,
                "package_id": manifest.package_id,
                "registry_revision": revision,
                "candidate_id_sha256": operator.candidate.candidate_id_sha256,
                "frame_id_sha256": frame.frame_id_sha256,
                "session_id_sha256": frame.session_id_sha256,
                "client_intent_id_sha256": frame.client_intent_id_sha256,
                "verifier_id": verifier_id,
                "verifier_version": "v1",
                "actor_program_sha256": response_actor_program_digest(&routing_package.program)
                    .unwrap_or_default(),
                "independent_verifier_program_sha256": routing_package
                    .verifier
                    .as_ref()
                    .and_then(|verifier| response_independent_verifier_program_digest(verifier).ok())
                    .unwrap_or_default(),
                "evidence_sha256": canonical_evidence_sha256.unwrap_or_default(),
                "output_sha256": canonical_output_sha256.unwrap_or_default(),
                "verified_at_unix_nanos": frame.observed_at_unix_nanos,
                "observed_label": if positive_label { "positive" } else { "negative" },
                "accepted": accepted,
            }));
        }
        if let Some(package) = grounded_packages
            .iter_mut()
            .find(|package| package.package_id == manifest.package_id)
        {
            package.proof.future_rows = package_future;
            package.proof.wrong_accepts = package_wrong;
            package.proof.distinct_sessions = future_sessions.len();
            package.proof.distinct_surfaces = future_surfaces.len();
            package.proof.verifier_schema = if package_future > 0 && package_missing == 0 {
                required_verifier_schema.to_owned()
            } else {
                "source_neutral_structure_only.v1".to_owned()
            };
        }
        future_frames = future_frames.saturating_add(package_future);
        future_wrong = future_wrong.saturating_add(package_wrong);
        missing_receipts = missing_receipts.saturating_add(package_missing);
        missing_receipts_by_package.insert(manifest.package_id.clone(), package_missing);
        package_future_eligibility.insert(
            manifest.package_id.clone(),
            serde_json::json!({
                "lineage_id": manifest.lineage_id,
                "generation": manifest.generation,
                "created_at_unix_nanos": manifest.created_at_unix_nanos,
                "support_boundary_unix_nanos": manifest.support_boundary_unix_nanos,
                "post_freeze_rows": package_post_freeze,
                "support_session_reject_rows": package_support_session_reject,
                "support_intent_reject_rows": package_support_intent_reject,
                "independent_rows": package_independent,
                "route_mismatch_rows": package_route_mismatch,
                "route_unbound_rows": package_route_unbound,
                "route_margin_below_rows": package_route_margin_below,
                "routed_rows": package_routed,
                "accepted_rows": package_future,
                "rejected_rows": package_wrong,
                "rejection_reasons": {
                    "negative_label": package_label_reject,
                    "missing_evidence": package_evidence_reject,
                    "missing_output_hash": package_output_hash_reject,
                    "family_mismatch": package_family_mismatch,
                    "response_shape": package_shape_reject,
                    "operator_structure": package_structure_reject,
                    "execution_or_verifier": package_execution_reject,
                },
            }),
        );
    }
    let mut runtime_parity_checks = 0_usize;
    let mut runtime_parity_failures = 0_usize;
    let mut grounded_causal_reports = BTreeMap::new();
    let mut routing_indistinguishable_by_package = BTreeMap::new();
    let mut hard_negative_accepts_by_package = BTreeMap::new();
    let mut runtime_parity_checks_by_package = BTreeMap::new();
    let mut runtime_parity_receipts_by_package = BTreeMap::<String, Vec<Value>>::new();
    let mut routed_counterexamples_by_package = BTreeMap::<String, Vec<Value>>::new();
    let mut routed_counterexample_sessions_by_lineage =
        BTreeMap::<String, std::collections::BTreeSet<String>>::new();
    let collection_packages = collection_families
        .iter()
        .filter_map(|family| {
            let preliminary = compile_collection_quarantine_package(family)?;
            let manifest = collection_manifests.get(&preliminary.package_id)?;
            let package = compile_collection_package(family, Some(manifest))?;
            Some((package, manifest, family))
        })
        .collect::<Vec<_>>();
    for (package, manifest, family) in &collection_packages {
        let mut package = package.clone();
        let support_ids = manifest
            .support_frame_ids
            .iter()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        let support_sessions = manifest
            .support_session_ids
            .iter()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        let support_intents = manifest
            .support_intent_ids
            .iter()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        let support_frames = relation_frames
            .iter()
            .filter(|frame| support_ids.contains(frame.frame_id_sha256.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        let future_cold = family
            .iter()
            .filter(|row| {
                row.observed_at_unix_nanos > manifest.support_boundary_unix_nanos
                    && !support_sessions.contains(row.session_id_sha256.as_str())
                    && !support_intents.contains(row.client_intent_id_sha256.as_str())
            })
            .collect::<Vec<_>>();
        let future_ids = future_cold
            .iter()
            .map(|row| row.frame_id_sha256.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let future_frames_for_package = relation_frames
            .iter()
            .filter(|frame| future_ids.contains(frame.frame_id_sha256.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        causal_support_by_package.insert(package.package_id.clone(), support_frames);
        causal_future_by_package.insert(package.package_id.clone(), future_frames_for_package);
        missing_receipts_by_package.insert(package.package_id.clone(), 0);
        let actor_sha256 = response_actor_program_digest(&package.program).unwrap_or_default();
        let verifier_sha256 = package
            .verifier
            .as_ref()
            .and_then(|verifier| response_independent_verifier_program_digest(verifier).ok())
            .unwrap_or_default();
        for row in future_cold {
            let execution = execute_response(&package.program, "", &row.example.provider_payload);
            let accepted = execution.status == ResponseExecutionStatus::Executed
                && execution.response.as_deref() == Some(row.example.expected_response.as_str())
                && package.verifier.as_ref().is_some_and(|verifier| {
                    verify_response_independently(
                        verifier,
                        &row.example.provider_payload,
                        execution.response.as_deref().unwrap_or_default(),
                    )
                    .is_ok()
                });
            verifier_receipts.push(serde_json::json!({
                "schema": RESPONSE_FUTURE_VERIFIER_RECEIPT_SCHEMA_V2,
                "package_id": package.package_id,
                "registry_revision": revision,
                "candidate_id_sha256": canonical_json_sha256(&package.program).unwrap_or_default(),
                "frame_id_sha256": row.frame_id_sha256,
                "session_id_sha256": row.session_id_sha256,
                "client_intent_id_sha256": row.client_intent_id_sha256,
                "verifier_id": "collection_program_external_evidence",
                "verifier_version": "v1",
                "actor_program_sha256": actor_sha256,
                "independent_verifier_program_sha256": verifier_sha256,
                "evidence_sha256": canonical_json_sha256(&row.example.provider_payload).unwrap_or_default(),
                "output_sha256": execution.response.as_ref().and_then(|output| canonical_json_sha256(output).ok()).unwrap_or_default(),
                "verified_at_unix_nanos": row.observed_at_unix_nanos,
                "observed_label": "positive",
                "accepted": accepted,
            }));
        }
        let family_ids = family
            .iter()
            .map(|row| row.frame_id_sha256.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let frames_by_id = relation_frames
            .iter()
            .map(|frame| (frame.frame_id_sha256.as_str(), frame))
            .collect::<BTreeMap<_, _>>();
        let background_wrong = cold_collection_rows
            .iter()
            .filter(|row| {
                row.observed_at_unix_nanos > manifest.support_boundary_unix_nanos
                    && !family_ids.contains(row.frame_id_sha256.as_str())
                    && !support_sessions.contains(row.session_id_sha256.as_str())
                    && !support_intents.contains(row.client_intent_id_sha256.as_str())
            })
            .filter_map(|row| {
                let frame = frames_by_id.get(row.frame_id_sha256.as_str()).copied()?;
                let winner = collection_packages
                    .iter()
                    .filter_map(|(candidate, _, _)| {
                        relation_frame_phase_margin_micro(candidate, frame)
                            .map(|margin| (margin, candidate.package_id.as_str()))
                    })
                    .max_by(|left, right| left.0.cmp(&right.0).then_with(|| right.1.cmp(left.1)))?;
                (winner.1 == package.package_id).then_some((row, frame))
            })
            .filter(|(row, _)| {
                let execution =
                    execute_response(&package.program, "", &row.example.provider_payload);
                execution.status == ResponseExecutionStatus::Executed
                    && execution.response.as_deref() != Some(row.example.expected_response.as_str())
            })
            .map(|(_, frame)| frame)
            .collect::<Vec<_>>();
        package.proof.wrong_accepts = package
            .proof
            .wrong_accepts
            .saturating_add(background_wrong.len());
        future_wrong = future_wrong.saturating_add(background_wrong.len());
        causal_negatives_by_package
            .entry(package.package_id.clone())
            .or_default()
            .extend(background_wrong);
        grounded_packages.push(package);
    }
    for package in &mut grounded_packages {
        let support = causal_support_by_package
            .get(&package.package_id)
            .cloned()
            .unwrap_or_default();
        let future = causal_future_by_package
            .get(&package.package_id)
            .cloned()
            .unwrap_or_default();
        let negatives = causal_negatives_by_package
            .entry(package.package_id.clone())
            .or_default();
        negatives.extend(package_negative_frame_refs_with_grounding(
            package,
            &support,
            &relation_frames,
            &grounded_family_by_frame_id,
        ));
        dedupe_frame_refs(negatives);
        package
            .anti_centers
            .extend(learned_discriminating_anti_centers(&support, negatives));
        package.anti_centers.sort_unstable();
        package.anti_centers.dedup();
        let routed_negatives = negatives
            .iter()
            .copied()
            .filter(|frame| relation_frame_routes_to_package(package, frame))
            .collect::<Vec<_>>();
        let routing_indistinguishable = routed_negatives.len();
        routed_counterexamples_by_package.insert(
            package.package_id.clone(),
            routed_negatives
                .iter()
                .take(8)
                .map(|frame| routed_counterexample_summary(frame))
                .collect(),
        );
        if let Some(manifest) = current_support_manifests
            .iter()
            .find(|manifest| manifest.package_id == package.package_id)
        {
            let support_sessions = manifest
                .support_session_ids
                .iter()
                .collect::<std::collections::BTreeSet<_>>();
            let sessions = routed_negatives
                .iter()
                .filter(|frame| !support_sessions.contains(&frame.session_id_sha256))
                .map(|frame| frame.session_id_sha256.clone())
                .collect::<std::collections::BTreeSet<_>>();
            if !sessions.is_empty() {
                routed_counterexample_sessions_by_lineage
                    .entry(manifest.lineage_id.clone())
                    .or_default()
                    .extend(sessions);
            }
        }
        routing_indistinguishable_by_package
            .insert(package.package_id.clone(), routing_indistinguishable);
        let hard_negative_accepts = exact_package_hard_negative_accepts(package);
        hard_negative_accepts_by_package.insert(package.package_id.clone(), hard_negative_accepts);
        let (checks, failures, parity_receipts) = if matches!(
            package.program.operation,
            ResponseOperation::ComposeCollection { .. }
        ) {
            collection_runtime_parity(package, &cold_collection_rows, &future, revision)
        } else {
            exact_package_runtime_parity(package, &future, revision)
        };
        runtime_parity_checks_by_package.insert(package.package_id.clone(), checks);
        runtime_parity_receipts_by_package.insert(package.package_id.clone(), parity_receipts);
        package.proof.runtime_parity_failures = failures;
        runtime_parity_checks = runtime_parity_checks.saturating_add(checks);
        runtime_parity_failures = runtime_parity_failures.saturating_add(failures);
        let support_refs = support.iter().collect::<Vec<_>>();
        let future_refs = future.iter().collect::<Vec<_>>();
        let report =
            evaluate_grounded_wave_causality_refs(package, &support_refs, &future_refs, negatives);
        let exact_package_causal_pass = report.verdict == "PASS";
        grounded_causal_reports.insert(package.package_id.clone(), report);
        package.proof.wave_causal_pass = exact_package_causal_pass;
        let package_missing = missing_receipts_by_package
            .get(&package.package_id)
            .copied()
            .unwrap_or_default();
        let promotion_ready = package.proof.support_rows >= 32
            && package.proof.future_rows >= 32
            && package.proof.distinct_sessions >= 3
            && package.proof.distinct_surfaces >= 2
            && package.proof.wrong_accepts == 0
            && package.proof.runtime_parity_failures == 0
            && package.proof.exact_cache_overlap == 0
            && package_missing == 0
            && hard_negative_accepts == 0
            && routing_indistinguishable == 0
            && relation_frame_conflicting_duplicate_ids == 0
            && wave_causal_pass
            && package.proof.wave_causal_pass
            && exact_package_causal_pass
            && package.verifier.is_some()
            && response_program_external_verifier_schema(&package.program)
                .is_some_and(|schema| package.proof.verifier_schema == schema);
        package.state = if promotion_ready {
            ResponsePackageState::Active
        } else {
            ResponsePackageState::Quarantine
        };
    }
    let mut rollover_policy = ResponseSupportFreezePolicy::default();
    for (lineage_id, sessions) in &routed_counterexample_sessions_by_lineage {
        let Some(current) = current_support_manifests
            .iter()
            .find(|manifest| manifest.lineage_id == *lineage_id)
        else {
            continue;
        };
        if current.routing_refinement_version < ROUTING_REFINEMENT_VERSION {
            continue;
        }
        if sessions.is_empty() {
            continue;
        }
        rollover_policy.only_lineages.insert(lineage_id.clone());
        rollover_policy
            .forced_support_session_ids_by_lineage
            .insert(lineage_id.clone(), sessions.clone());
        rollover_policy
            .generation_by_lineage
            .insert(lineage_id.clone(), current.generation.saturating_add(1));
        rollover_policy
            .supersedes_package_id_by_lineage
            .insert(lineage_id.clone(), current.package_id.clone());
    }
    // A clean package can still overfit its routing refinement to volatile
    // support context. Positive rows from the same synthesized family are not
    // future accepts, but they are valid evidence for an immutable next
    // generation that must rediscover a transferable pre-action guard.
    for (lineage_id, sessions) in &positive_route_mismatch_sessions_by_lineage {
        let Some(current) = current_support_manifests
            .iter()
            .find(|manifest| manifest.lineage_id == *lineage_id)
        else {
            continue;
        };
        if current.routing_refinement_version < ROUTING_REFINEMENT_VERSION {
            continue;
        }
        if current.selected_routing_atom_ids.is_empty() {
            continue;
        }
        if sessions.is_empty() {
            continue;
        }
        rollover_policy.only_lineages.insert(lineage_id.clone());
        rollover_policy
            .forced_support_session_ids_by_lineage
            .entry(lineage_id.clone())
            .or_default()
            .extend(sessions.iter().cloned());
        rollover_policy
            .generation_by_lineage
            .insert(lineage_id.clone(), current.generation.saturating_add(1));
        rollover_policy
            .supersedes_package_id_by_lineage
            .insert(lineage_id.clone(), current.package_id.clone());
    }
    let mut self_training_rollover_sessions_by_lineage =
        BTreeMap::<String, std::collections::BTreeSet<String>>::new();
    for package in &grounded_packages {
        if package.state != ResponsePackageState::Active {
            continue;
        }
        let Some(current) = current_support_manifests
            .iter()
            .find(|manifest| manifest.package_id == package.package_id)
        else {
            continue;
        };
        let future = causal_future_by_package
            .get(&package.package_id)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let sessions = verified_future_sessions_for_self_training(future);
        if sessions.is_empty() {
            continue;
        }
        self_training_rollover_sessions_by_lineage
            .insert(current.lineage_id.clone(), sessions.clone());
        rollover_policy
            .only_lineages
            .insert(current.lineage_id.clone());
        rollover_policy
            .forced_support_session_ids_by_lineage
            .entry(current.lineage_id.clone())
            .or_default()
            .extend(sessions);
        rollover_policy.generation_by_lineage.insert(
            current.lineage_id.clone(),
            current.generation.saturating_add(1),
        );
        rollover_policy
            .supersedes_package_id_by_lineage
            .insert(current.lineage_id.clone(), current.package_id.clone());
    }
    let counterexample_rollover_lineages = rollover_policy.only_lineages.clone();
    let mut evidence_refresh_policy = ResponseSupportFreezePolicy::default();
    for package in &grounded_packages {
        let Some(current) = current_support_manifests
            .iter()
            .find(|manifest| manifest.package_id == package.package_id)
        else {
            continue;
        };
        let policy_migration = current.routing_refinement_version < ROUTING_REFINEMENT_VERSION;
        if !policy_migration
            && (package.state != ResponsePackageState::Quarantine
                || package.proof.future_rows != 0
                || current.reserved_future_session_ids.len() >= 3
                || counterexample_rollover_lineages.contains(&current.lineage_id))
        {
            continue;
        }
        evidence_refresh_policy
            .only_lineages
            .insert(current.lineage_id.clone());
        evidence_refresh_policy
            .forced_support_session_ids_by_lineage
            .insert(
                current.lineage_id.clone(),
                current.support_session_ids.iter().cloned().collect(),
            );
        if let Some(family_id) = relation_frames
            .iter()
            .filter(|frame| current.support_frame_ids.contains(&frame.frame_id_sha256))
            .find_map(relation_frame_family_id)
        {
            evidence_refresh_policy
                .forced_family_id_by_lineage
                .insert(current.lineage_id.clone(), family_id);
        }
        evidence_refresh_policy.generation_by_lineage.insert(
            current.lineage_id.clone(),
            current.generation.saturating_add(1),
        );
        evidence_refresh_policy
            .supersedes_package_id_by_lineage
            .insert(current.lineage_id.clone(), current.package_id.clone());
    }
    let rollover_candidates = if rollover_policy.only_lineages.is_empty() {
        ResponseSupportManifestSet {
            schema: "nando.response-support-manifest-set.v1".to_owned(),
            manifests: Vec::new(),
        }
    } else {
        freeze_source_neutral_support_with_policy(
            &relation_frames,
            unix_now().saturating_mul(1_000_000_000),
            wave_causal_pass,
            &rollover_policy,
        )
    };
    let known_packages = support_manifests
        .manifests
        .iter()
        .map(|manifest| manifest.package_id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let rollover_manifests = rollover_candidates
        .manifests
        .into_iter()
        .filter(|manifest| {
            !known_packages.contains(&manifest.package_id)
                && current_support_manifests
                    .iter()
                    .find(|current| current.lineage_id == manifest.lineage_id)
                    .is_some_and(|current| rollover_manifest_improves(current, manifest))
        })
        .collect::<Vec<_>>();
    let rollover_manifest_ids = rollover_manifests
        .iter()
        .map(|manifest| manifest.package_id.clone())
        .collect::<Vec<_>>();
    let evidence_refresh_candidates = if evidence_refresh_policy.only_lineages.is_empty() {
        ResponseSupportManifestSet {
            schema: "nando.response-support-manifest-set.v1".to_owned(),
            manifests: Vec::new(),
        }
    } else {
        freeze_source_neutral_support_with_policy(
            &relation_frames,
            unix_now().saturating_mul(1_000_000_000),
            wave_causal_pass,
            &evidence_refresh_policy,
        )
    };
    let evidence_refresh_candidate_count = evidence_refresh_candidates.manifests.len();
    let evidence_refresh_candidate_summaries = evidence_refresh_candidates
        .manifests
        .iter()
        .map(|manifest| {
            serde_json::json!({
                "package_id": manifest.package_id,
                "lineage_id": manifest.lineage_id,
                "generation": manifest.generation,
                "routing_refinement_version": manifest.routing_refinement_version,
                "support_rows": manifest.support_frame_ids.len(),
            })
        })
        .collect::<Vec<_>>();
    let evidence_refresh_manifests = evidence_refresh_candidates
        .manifests
        .into_iter()
        .filter(|candidate| {
            let matching_current = current_support_manifests
                .iter()
                .find(|current| current.lineage_id == candidate.lineage_id);
            let same_lineage_improvement = matching_current
                .is_some_and(|current| evidence_refresh_improves(current, candidate));
            let new_policy_lineage = candidate.routing_refinement_version
                == ROUTING_REFINEMENT_VERSION
                && candidate.support_frame_ids.len() >= 32
                && !evidence_refresh_policy.only_lineages.is_empty()
                && matching_current.is_none();
            !known_packages.contains(&candidate.package_id)
                && (same_lineage_improvement || new_policy_lineage)
        })
        .collect::<Vec<_>>();
    let evidence_refresh_manifest_ids = evidence_refresh_manifests
        .iter()
        .map(|manifest| manifest.package_id.clone())
        .collect::<Vec<_>>();
    if !rollover_manifests.is_empty() || !evidence_refresh_manifests.is_empty() {
        support_manifests.manifests.extend(rollover_manifests);
        support_manifests
            .manifests
            .extend(evidence_refresh_manifests);
        atomic_write_json(&support_manifests_path, &support_manifests)?;
    }
    let verifier_receipts_emitted = verifier_receipts.len();
    let verifier_receipts_accepted = verifier_receipts
        .iter()
        .filter(|receipt| receipt.get("accepted").and_then(Value::as_bool) == Some(true))
        .count();
    let verifier_coverage_state = verifier_coverage_state(future_frames, verifier_receipts_emitted);
    let future_verifier_receipt_packages = package_receipt_sets(
        revision,
        &verifier_receipts,
        "nando.response-future-verifier-package-receipts.v2",
    );
    atomic_write_value(
        &verifier_receipts_path,
        &serde_json::json!({
            "schema": RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2,
            "registry_revision": revision,
            "generated_at_unix": unix_now(),
            "packages": future_verifier_receipt_packages,
        }),
    )?;
    let runtime_parity_receipt_packages = runtime_parity_receipts_by_package
        .iter()
        .map(|(package_id, receipts)| {
            serde_json::json!({
                "schema": "nando.response-runtime-parity-package-receipts.v1",
                "package_id": package_id,
                "registry_revision": revision,
                "receipts": receipts,
            })
        })
        .collect::<Vec<_>>();
    atomic_write_value(
        &runtime_parity_receipts_path,
        &serde_json::json!({
            "schema": RESPONSE_RUNTIME_PARITY_RECEIPT_SET_SCHEMA_V1,
            "registry_revision": revision,
            "generated_at_unix": unix_now(),
            "packages": runtime_parity_receipt_packages,
        }),
    )?;
    let exact_package_verdict = aggregate_causal_verdict(
        grounded_packages.iter().map(|package| &package.package_id),
        &grounded_causal_reports,
    );
    let exact_package_count = grounded_packages.len();
    let cross_family_negative_accepts = hard_negative_accepts_by_package.values().sum::<usize>();
    let routing_indistinguishable_negative_frames =
        routing_indistinguishable_by_package.values().sum::<usize>();
    let exact_package_causal_pass = exact_package_verdict == "PASS";
    let causal_negative_frames = causal_negatives_by_package
        .values()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    let applicability_negative_frames = causal_negative_frames
        .iter()
        .filter(|frame| frame.verifier_label != Some(false))
        .collect::<Vec<_>>();
    atomic_write_value(
        &grounded_causal_path,
        &serde_json::json!({
            "schema": "nando.grounded-response-wave-causal-report-set.v1",
            "verdict": exact_package_verdict,
            "packages": grounded_causal_reports,
        }),
    )?;
    let registry = compile_runtime_registry(revision, grounded_packages);
    atomic_write_json(&registry_path, &registry)?;
    let response_authority_candidate = response_authority_candidate(
        &registry,
        &current_support_manifests,
        &grounded_causal_reports,
        &future_verifier_receipt_packages,
        &runtime_parity_receipt_packages,
    )?;
    let grounded_candidate = registry
        .packages
        .iter()
        .find(|package| package.package_id.starts_with("raw-phase-grounded-"));
    let grounded_promotion_blockers = grounded_candidate.map_or_else(
        || vec!["grounded_candidate_missing".to_owned()],
        |package| {
            let mut blockers = Vec::new();
            let package_missing_receipts = missing_receipts_by_package
                .get(&package.package_id)
                .copied()
                .unwrap_or_default();
            let package_causal_report = grounded_causal_reports.get(&package.package_id);
            let package_runtime_checks = runtime_parity_checks_by_package
                .get(&package.package_id)
                .copied()
                .unwrap_or_default();
            let package_hard_negative_accepts = hard_negative_accepts_by_package
                .get(&package.package_id)
                .copied()
                .unwrap_or_default();
            let package_routing_indistinguishable = routing_indistinguishable_by_package
                .get(&package.package_id)
                .copied()
                .unwrap_or_default();
            if package.proof.support_rows < 32 {
                blockers.push("support_rows_below_32".to_owned());
            }
            if package.proof.future_rows < 32 {
                blockers.push("future_rows_below_32".to_owned());
            }
            if package.proof.distinct_sessions < 3 {
                blockers.push("future_sessions_below_3".to_owned());
            }
            if package.proof.distinct_surfaces < 2 {
                blockers.push("future_surfaces_below_2".to_owned());
            }
            if package.proof.wrong_accepts != 0 {
                blockers.push("future_wrong_accepts_nonzero".to_owned());
            }
            if package_missing_receipts != 0 {
                blockers.push("missing_verifier_receipts".to_owned());
            }
            if package_causal_report.is_none_or(|report| report.verdict != "PASS") {
                blockers.push("exact_package_causal_ablation_not_pass".to_owned());
            }
            if !wave_causal_pass || !package.proof.wave_causal_pass {
                blockers.push("causal_proof_not_pass".to_owned());
            }
            if package_runtime_checks < package.proof.future_rows
                || package.proof.runtime_parity_failures != 0
            {
                blockers.push("runtime_parity_not_pass".to_owned());
            }
            if package_hard_negative_accepts != 0 {
                blockers.push("cross_family_negative_accepts_nonzero".to_owned());
            }
            if package_routing_indistinguishable != 0 {
                blockers.push("negative_unseparable_at_current_representation".to_owned());
            }
            if relation_frame_conflicting_duplicate_ids != 0 {
                blockers.push("conflicting_relation_frame_duplicate_ids".to_owned());
            }
            blockers
        },
    );
    let active = registry
        .packages
        .iter()
        .filter(|package| package.eligible_for_admission_candidate())
        .count();
    let quarantined = registry
        .packages
        .iter()
        .filter(|package| package.state == nando_response_actor::ResponsePackageState::Quarantine)
        .count();
    let grounded_active = registry
        .packages
        .iter()
        .filter(|package| {
            is_grounded_package(package) && package.eligible_for_admission_candidate()
        })
        .count();
    let grounded_quarantine = registry
        .packages
        .iter()
        .filter(|package| {
            is_grounded_package(package) && package.state == ResponsePackageState::Quarantine
        })
        .count();
    let legacy_named_active = legacy_shadow_registry
        .packages
        .iter()
        .filter(|package| {
            !is_grounded_package(package) && package.eligible_for_admission_candidate()
        })
        .count();
    let legacy_named_quarantine = legacy_shadow_registry
        .packages
        .iter()
        .filter(|package| {
            !is_grounded_package(package) && package.state == ResponsePackageState::Quarantine
        })
        .count();
    let nearest = registry
        .packages
        .iter()
        .filter(|package| {
            package.state == nando_response_actor::ResponsePackageState::Quarantine
                && package.proof.wrong_accepts == 0
        })
        .min_by_key(|package| {
            32_usize.saturating_sub(package.proof.support_rows)
                + 32_usize.saturating_sub(package.proof.future_rows)
                + 3_usize.saturating_sub(package.proof.distinct_sessions)
                + 2_usize.saturating_sub(package.proof.distinct_surfaces)
        });
    let nearest_grounded = registry
        .packages
        .iter()
        .filter(|package| {
            is_grounded_package(package)
                && package.state == ResponsePackageState::Quarantine
                && package.proof.wrong_accepts == 0
        })
        .min_by_key(promotion_debt);
    let nearest_legacy = legacy_shadow_registry
        .packages
        .iter()
        .filter(|package| {
            !is_grounded_package(package)
                && package.state == ResponsePackageState::Quarantine
                && package.proof.wrong_accepts == 0
        })
        .min_by_key(promotion_debt);
    let support_boundary_age_seconds = latest_support_boundary_unix_nanos
        .map(|boundary| unix_now().saturating_sub(boundary / 1_000_000_000));
    let grounded_manifest = grounded_candidate.and_then(|package| {
        current_support_manifests
            .iter()
            .find(|manifest| manifest.package_id == package.package_id)
    });
    let grounded_candidate_status = grounded_candidate.map(|package| {
        let package_causal_report = grounded_causal_reports.get(&package.package_id);
        let package_runtime_checks = runtime_parity_checks_by_package
            .get(&package.package_id)
            .copied()
            .unwrap_or_default();
        let package_hard_negative_accepts = hard_negative_accepts_by_package
            .get(&package.package_id)
            .copied()
            .unwrap_or_default();
        let package_routing_indistinguishable = routing_indistinguishable_by_package
            .get(&package.package_id)
            .copied()
            .unwrap_or_default();
        serde_json::json!({
            "package_id": package.package_id,
            "generation": "grounded_generic",
            "state": package_state_name(package.state),
            "program_operation": program_operation_name(&package.program.operation),
            "support_rows": package.proof.support_rows,
            "future_rows": package.proof.future_rows,
            "distinct_sessions": package.proof.distinct_sessions,
            "distinct_surfaces": package.proof.distinct_surfaces,
            "wrong_accepts": package.proof.wrong_accepts,
            "verifier_schema": package.proof.verifier_schema,
            "phase_center_atoms": package.phase_centers.len(),
            "anti_center_atoms": package.anti_centers.len(),
            "wave_margin_micro": package.wave_margin_micro,
            "exact_package_causal_verdict": package_causal_report
                .map_or("MISSING", |report| report.verdict.as_str()),
            "runtime_parity_checks": package_runtime_checks,
            "runtime_parity_failures": package.proof.runtime_parity_failures,
            "cross_family_negative_accepts": package_hard_negative_accepts,
            "routing_indistinguishable_negative_frames": package_routing_indistinguishable,
            "representation_state": if package_routing_indistinguishable == 0 {
                "SEPARABLE"
            } else {
                "UNSEPARABLE_AT_CURRENT_REPRESENTATION"
            },
            "manifest_attached": current_support_manifests
                .iter()
                .any(|manifest| manifest.package_id == package.package_id),
            "support_boundary_age_seconds": support_boundary_age_seconds,
            "split_state": grounded_manifest.map_or("MISSING", |manifest| {
                if manifest.selected_routing_atom_ids.is_empty()
                    && manifest.selected_routing_predicates.is_empty()
                {
                    "NO_CLEAN_PRE_ACTION_SPLIT"
                } else {
                    "CLEAN_CONTEXT_SUBCENTER"
                }
            }),
            "selected_routing_atoms": grounded_manifest
                .map_or(0, |manifest| manifest.selected_routing_atom_ids.len()),
            "selected_routing_predicates": grounded_manifest
                .map_or(&[][..], |manifest| manifest.selected_routing_predicates.as_slice()),
            "split_parent_support_rows": grounded_manifest
                .map_or(0, |manifest| manifest.split_parent_support_rows),
            "split_retained_support_rows": package.proof.support_rows,
            "split_negative_frames": grounded_manifest
                .map_or(0, |manifest| manifest.split_negative_frame_ids.len()),
            "holdout_negative_frames": grounded_manifest
                .map_or(0, |manifest| manifest.holdout_negative_frame_ids.len()),
            "reserved_future_sessions": grounded_manifest
                .map_or(0, |manifest| manifest.reserved_future_session_ids.len()),
            "promotion_blockers": grounded_promotion_blockers.clone(),
        })
    });
    let grounded_package_proofs = registry
        .packages
        .iter()
        .filter(|package| is_grounded_package(package))
        .map(|package| {
            serde_json::json!({
                "package_id": package.package_id,
                "state": package_state_name(package.state),
                "execution_authority": 0,
                "admission_candidate": package.eligible_for_admission_candidate(),
                "program_operation": program_operation_name(&package.program.operation),
                "support_rows": package.proof.support_rows,
                "future_rows": package.proof.future_rows,
                "wrong_accepts": package.proof.wrong_accepts,
                "verifier_schema": package.proof.verifier_schema,
                "causal_report": grounded_causal_reports.get(&package.package_id),
                "runtime_parity_checks": runtime_parity_checks_by_package
                    .get(&package.package_id)
                    .copied()
                    .unwrap_or_default(),
                "runtime_parity_failures": package.proof.runtime_parity_failures,
                "hard_negative_accepts": hard_negative_accepts_by_package
                    .get(&package.package_id)
                    .copied()
                    .unwrap_or_default(),
                "routing_indistinguishable_negatives": routing_indistinguishable_by_package
                    .get(&package.package_id)
                    .copied()
                    .unwrap_or_default(),
                "missing_verifier_receipts": missing_receipts_by_package
                    .get(&package.package_id)
                    .copied()
                    .unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>();
    let synthesis_status = serde_json::json!({
        "selected_candidates": synthesized_candidates,
        "generic_function_call_candidates": generic_function_call_candidates,
        "value_projection_candidates": value_projection_candidates,
        "status_projection_candidates": status_projection_candidates,
        "exact_checks": synthesis_exact_checks,
        "description_bytes": synthesis_description_bytes,
        "program_hint_authority_used": false,
        "manual_family_to_operator_mapping_used": false,
        "failed_families": synthesis_failures,
    });
    let verifier_coverage = serde_json::json!({
        "state": verifier_coverage_state,
        "required": future_frames,
        "emitted": verifier_receipts_emitted,
        "accepted": verifier_receipts_accepted,
        "missing": missing_receipts,
    });
    let package_generations = serde_json::json!({
        "grounded_generic": {
            "active": grounded_active,
            "quarantine": grounded_quarantine,
            "execution_authority": grounded_active,
        },
        "legacy_named": {
            "active": legacy_named_active,
            "quarantine": legacy_named_quarantine,
            "shadow_packages": legacy_shadow_registry.packages.len(),
            "present_in_runtime_registry": 0,
            "execution_authority": 0,
        },
    });
    let support_manifest_linkage = serde_json::json!({
        "current_manifests": current_support_manifests.len(),
        "matched_manifests": matched_support_manifests,
        "unmatched_current_manifests": unmatched_current_support_manifests,
        "grounded_packages_without_manifest": grounded_packages_without_manifest,
        "current_package_match": matched_support_manifests > 0
            && unmatched_current_support_manifests == 0
            && grounded_packages_without_manifest == 0,
        "latest_boundary_age_seconds": support_boundary_age_seconds,
    });
    let causal_proofs = serde_json::json!({
        "global_regression_pass": wave_causal_pass,
        "exact_package_pass": exact_package_causal_pass,
        "exact_package_verdict": exact_package_verdict,
        "exact_package_report": {
            "schema": "nando.grounded-response-wave-causal-report-set.v1",
            "verdict": exact_package_verdict,
            "package_count": exact_package_count,
            "pass_count": grounded_causal_reports.values()
                .filter(|report| report.verdict == "PASS")
                .count(),
            "watch_count": exact_package_count.saturating_sub(
                grounded_causal_reports.values()
                    .filter(|report| report.verdict == "PASS")
                    .count()
            ),
            "support_rows": grounded_causal_reports.values().map(|report| report.support_rows).sum::<usize>(),
            "future_rows": grounded_causal_reports.values().map(|report| report.future_rows).sum::<usize>(),
            "full_phase_correct": grounded_causal_reports.values().map(|report| report.full_phase_correct).sum::<usize>(),
            "shuffled_phase_correct": grounded_causal_reports.values().map(|report| report.shuffled_phase_correct).sum::<usize>(),
            "random_center_correct": grounded_causal_reports.values().map(|report| report.random_center_correct).sum::<usize>(),
            "negative_accepts": grounded_causal_reports.values().map(|report| report.negative_accepts).sum::<usize>(),
        },
        "packages": grounded_package_proofs,
        "family_reports": grounded_causal_reports,
    });
    let counterexample_rollover = serde_json::json!({
        "lineages_with_routed_future_counterexamples":
            routed_counterexample_sessions_by_lineage.len(),
        "next_generation_manifests_emitted": rollover_manifest_ids.len(),
        "next_generation_package_ids": rollover_manifest_ids,
        "routed_counterexamples_by_package": routed_counterexamples_by_package,
    });
    let self_training_rollover = serde_json::json!({
        "policy": "verified_future_dataset_aggregation_v1",
        "minimum_verified_future_rows": SELF_TRAINING_MIN_VERIFIED_FUTURE_ROWS,
        "minimum_verified_future_sessions": SELF_TRAINING_MIN_VERIFIED_FUTURE_SESSIONS,
        "reserved_future_sessions": SELF_TRAINING_RESERVED_FUTURE_SESSIONS,
        "minimum_rollover_rows": SELF_TRAINING_MIN_ROLLOVER_ROWS,
        "lineages": self_training_rollover_sessions_by_lineage.len(),
        "training_sessions_by_lineage": self_training_rollover_sessions_by_lineage,
        "generated_manifest_ids": rollover_manifest_ids,
    });
    let evidence_refresh_rollover = serde_json::json!({
        "eligible_lineages": evidence_refresh_policy.only_lineages.len(),
        "forced_family_ids": evidence_refresh_policy.forced_family_id_by_lineage,
        "candidate_manifests_before_filter": evidence_refresh_candidate_count,
        "candidates": evidence_refresh_candidate_summaries,
        "next_generation_manifests_emitted": evidence_refresh_manifest_ids.len(),
        "next_generation_package_ids": evidence_refresh_manifest_ids,
        "trigger": "quarantine_without_future_and_new_reservable_sessions",
        "authority_inherited": false,
    });
    let live_collection_packages = registry
        .packages
        .iter()
        .filter(|package| {
            matches!(
                package.program.operation,
                ResponseOperation::ComposeCollection { .. }
            )
        })
        .collect::<Vec<_>>();
    let mut status = serde_json::json!({
        "schema": "nando.response-miner-status.v1",
        "input_fingerprint_sha256": input_fingerprint_sha256,
        "cycle_mode": "full_recompute",
        "generated_at_unix": unix_now(),
        "revision": revision,
        "relations_total": relations.len(),
        "relation_frames_total": raw_relation_frame_rows,
        "grounded_role_families": grounded_families.len(),
        "ambiguous_relation_frames": ambiguous_frames,
        "synthesized_program_candidates": synthesized_candidates,
        "synthesis": synthesis_status,
        "collection_synthesis": {
            "cold_evidence_rows": cold_collection_rows.len(),
            "discovered_families": collection_families.len(),
            "candidate_present": !live_collection_packages.is_empty(),
            "package_ids": live_collection_packages.iter().map(|package| package.package_id.as_str()).collect::<Vec<_>>(),
            "operations": live_collection_packages.iter().map(|package| program_operation_name(&package.program.operation)).collect::<Vec<_>>(),
            "support_rows": live_collection_packages.iter().map(|package| package.proof.support_rows).sum::<usize>(),
            "future_rows": live_collection_packages.iter().map(|package| package.proof.future_rows).sum::<usize>(),
            "future_wrong_accepts": live_collection_packages.iter().map(|package| package.proof.wrong_accepts).sum::<usize>(),
            "distinct_sessions": live_collection_packages.iter().map(|package| package.proof.distinct_sessions).max().unwrap_or(0),
            "distinct_surfaces": live_collection_packages.iter().map(|package| package.proof.distinct_surfaces).max().unwrap_or(0),
            "wave_causal_pass": !live_collection_packages.is_empty() && live_collection_packages.iter().all(|package| package.proof.wave_causal_pass),
            "execution_authority": false,
        },
        "support_manifests": current_support_manifests.len(),
        "historical_support_manifests": historical_support_manifest_count,
        "live_support_manifests": support_manifests.manifests.len(),
        "frozen_support_frames": frozen_support.len(),
        "independent_future_frames": future_frames,
        "future_wrong_accepts": future_wrong,
        "missing_verifier_receipts": missing_receipts,
        "verifier_coverage": verifier_coverage,
        "negative_relation_frames": negative_frames.len(),
        "applicability_negative_frames": applicability_negative_frames.len(),
        "learned_anti_center_atoms": registry
            .packages
            .iter()
            .flat_map(|package| package.anti_centers.iter().copied())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        "grounded_promotion_ready": grounded_promotion_blockers.is_empty(),
        "grounded_promotion_blockers": grounded_promotion_blockers,
        "grounded_causal_verdict": exact_package_verdict,
        "causal_proofs": causal_proofs,
        "runtime_parity_checks": runtime_parity_checks,
        "runtime_parity_failures": runtime_parity_failures,
        "cross_family_negative_accepts": cross_family_negative_accepts,
        "future_shadows_total": shadows.len(),
        "future_eligibility": {
            "post_freeze_rows": post_freeze_rows,
            "support_session_reject_rows": support_session_reject_rows,
            "support_intent_reject_rows": support_intent_reject_rows,
            "independent_post_freeze_rows": independent_post_freeze_rows,
            "reserved_session_rows": reserved_session_rows,
            "new_session_rows": new_session_rows,
            "route_mismatch_rows": route_mismatch_rows,
            "route_unbound_rows": route_unbound_rows,
            "route_margin_below_rows": route_margin_below_rows,
            "route_margin_min_micro": route_margin_min_micro,
            "route_margin_max_micro": route_margin_max_micro,
            "routed_rows": routed_rows,
            "verifier_accepted_rows": verifier_accepted_rows,
            "verifier_rejected_rows": verifier_rejected_rows,
            "packages": package_future_eligibility,
        },
        "packages_total": registry.packages.len(),
        "active_packages": active,
        "quarantined_packages": quarantined,
        "package_generations": package_generations,
        "grounded_candidate": grounded_candidate_status,
        "support_manifest_linkage": support_manifest_linkage,
        "wave_causal_pass": wave_causal_pass,
        "nearest_clean_candidate": candidate_progress(nearest),
        "nearest_grounded_candidate": candidate_progress(nearest_grounded),
        "nearest_legacy_candidate": candidate_progress(nearest_legacy),
        "cycle_duration_ms": cycle_started.elapsed().as_millis() as u64,
        "automatic_lifecycle": true,
        "manual_profile_approval": false,
    });
    let status_object = status
        .as_object_mut()
        .ok_or_else(|| "response_miner_status_not_object".to_owned())?;
    status_object.insert(
        "counterexample_rollover".to_owned(),
        counterexample_rollover,
    );
    status_object.insert("self_training_rollover".to_owned(), self_training_rollover);
    status_object.insert(
        "evidence_refresh_rollover".to_owned(),
        evidence_refresh_rollover,
    );
    status_object.insert("execution_authority".to_owned(), Value::from(0));
    status_object.insert(
        "response_authority_candidate".to_owned(),
        response_authority_candidate,
    );
    status_object.insert(
        "response_authority_requires_composite_admission_v2".to_owned(),
        Value::Bool(true),
    );
    status_object.insert(
        "grounded_family_scoreboard".to_owned(),
        Value::Array(grounded_family_reports),
    );
    status_object.insert("token_opportunity".to_owned(), token_opportunity);
    status_object.insert(
        "source_neutral_relation_frames".to_owned(),
        Value::from(relation_frames.len() as u64),
    );
    status_object.insert(
        "relation_frames_unique".to_owned(),
        Value::from(unique_relation_frame_rows as u64),
    );
    status_object.insert(
        "relation_frame_duplicate_rows".to_owned(),
        Value::from(relation_frame_duplicate_rows as u64),
    );
    status_object.insert(
        "relation_frame_conflicting_duplicate_ids".to_owned(),
        Value::from(relation_frame_conflicting_duplicate_ids as u64),
    );
    status_object.insert(
        "legacy_relation_frames_ignored".to_owned(),
        Value::from(legacy_relation_frames_ignored as u64),
    );
    status_object.insert(
        "verifier_negative_relation_frames".to_owned(),
        Value::from(verifier_negative_frames.len() as u64),
    );
    status_object.insert(
        "causal_negative_relation_frames".to_owned(),
        Value::from(causal_negative_frames.len() as u64),
    );
    status_object.insert(
        "routing_indistinguishable_negative_frames".to_owned(),
        Value::from(routing_indistinguishable_negative_frames as u64),
    );
    status_object.insert(
        "representation_state".to_owned(),
        Value::from(if routing_indistinguishable_negative_frames == 0 {
            "SEPARABLE"
        } else {
            "UNSEPARABLE_AT_CURRENT_REPRESENTATION"
        }),
    );
    status_object.insert(
        "routing_split".to_owned(),
        serde_json::json!({
            "state": grounded_manifest.map_or("MISSING", |manifest| {
                if manifest.selected_routing_atom_ids.is_empty()
                    && manifest.selected_routing_predicates.is_empty()
                {
                    "NO_CLEAN_PRE_ACTION_SPLIT"
                } else {
                    "CLEAN_CONTEXT_SUBCENTER"
                }
            }),
            "selected_atoms": grounded_manifest
                .map_or(0, |manifest| manifest.selected_routing_atom_ids.len()),
            "selected_predicates": grounded_manifest
                .map_or(&[][..], |manifest| manifest.selected_routing_predicates.as_slice()),
            "parent_support_rows": grounded_manifest
                .map_or(0, |manifest| manifest.split_parent_support_rows),
            "retained_support_rows": grounded_candidate
                .map_or(0, |package| package.proof.support_rows),
            "negative_frames": grounded_manifest
                .map_or(0, |manifest| manifest.split_negative_frame_ids.len()),
            "holdout_negative_frames": grounded_manifest
                .map_or(0, |manifest| manifest.holdout_negative_frame_ids.len()),
            "manual_class_list_used": false,
            "target_or_action_label_used_at_runtime": false,
        }),
    );
    atomic_write_value(&status_path, &status)?;
    println!(
        "{}",
        serde_json::json!({
            "schema": "nando.response-miner-run.v1",
            "revision": revision,
            "relations": relations.len(),
            "future_shadows": shadows.len(),
            "wave_causal_pass": wave_causal_pass,
            "packages": registry.packages.len(),
            "active_packages": active,
            "registry_path": registry_path,
        })
    );
    Ok(())
}

fn dedupe_relation_frames(frames: Vec<RelationFrame>) -> (Vec<RelationFrame>, usize, usize) {
    let raw_rows = frames.len();
    let mut by_id = BTreeMap::new();
    let mut conflicting_ids = std::collections::BTreeSet::new();
    for frame in frames {
        if let Some(existing) = by_id.get(&frame.frame_id_sha256) {
            if existing != &frame {
                conflicting_ids.insert(frame.frame_id_sha256.clone());
            }
            continue;
        }
        by_id.insert(frame.frame_id_sha256.clone(), frame);
    }
    let unique = by_id.into_values().collect::<Vec<_>>();
    let duplicate_rows = raw_rows.saturating_sub(unique.len());
    (unique, duplicate_rows, conflicting_ids.len())
}

fn read_json_lines<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(path).map_err(|error| format!("open:{}:{error}", path.display()))?;
    let mut rows = Vec::new();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|error| format!("read:{}:{error}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        rows.push(serde_json::from_str(&line).map_err(|error| {
            format!(
                "parse:{}:{}:{error}",
                path.display(),
                index.saturating_add(1)
            )
        })?);
    }
    Ok(rows)
}

fn is_grounded_package(package: &ResponsePackage) -> bool {
    package.package_id.starts_with("raw-phase-grounded-")
}

fn promotion_debt(package: &&ResponsePackage) -> usize {
    32_usize.saturating_sub(package.proof.support_rows)
        + 32_usize.saturating_sub(package.proof.future_rows)
        + 3_usize.saturating_sub(package.proof.distinct_sessions)
        + 2_usize.saturating_sub(package.proof.distinct_surfaces)
}

fn candidate_progress(package: Option<&ResponsePackage>) -> Value {
    package.map_or(Value::Null, |package| {
        serde_json::json!({
            "package_id": package.package_id,
            "generation": if is_grounded_package(package) {
                "grounded_generic"
            } else {
                "legacy_named"
            },
            "support_rows": package.proof.support_rows,
            "future_rows": package.proof.future_rows,
            "distinct_sessions": package.proof.distinct_sessions,
            "distinct_surfaces": package.proof.distinct_surfaces,
            "wrong_accepts": package.proof.wrong_accepts,
            "support_gap": 32_usize.saturating_sub(package.proof.support_rows),
            "future_gap": 32_usize.saturating_sub(package.proof.future_rows),
            "session_gap": 3_usize.saturating_sub(package.proof.distinct_sessions),
            "surface_gap": 2_usize.saturating_sub(package.proof.distinct_surfaces),
        })
    })
}

const fn verifier_coverage_state(required: usize, emitted: usize) -> &'static str {
    if required == 0 {
        "NOT_EVALUATED"
    } else if emitted >= required {
        "COMPLETE"
    } else {
        "PARTIAL"
    }
}

const fn package_state_name(state: ResponsePackageState) -> &'static str {
    match state {
        ResponsePackageState::Quarantine => "quarantine",
        ResponsePackageState::Active => "active",
        ResponsePackageState::Revoked => "revoked",
    }
}

const fn program_operation_name(operation: &ResponseOperation) -> &'static str {
    match operation {
        ResponseOperation::UniqueConsensus { .. } => "unique_consensus",
        ResponseOperation::AdvancePlan { .. } => "advance_plan",
        ResponseOperation::FunctionCallFromRoles { .. } => "function_call_from_roles",
        ResponseOperation::CustomToolCallFromRoles { .. } => "custom_tool_call_from_roles",
        ResponseOperation::ProjectSelectedValue { .. } => "project_selected_value",
        ResponseOperation::ProjectStatus { .. } => "project_status",
        ResponseOperation::ComposeCollection { .. } => "compose_collection",
        ResponseOperation::CopyAfterPrefix { .. } => "copy_after_prefix",
        ResponseOperation::TestResultSummary { .. } => "test_result_summary",
        ResponseOperation::WaitOnYieldedCell { .. } => "wait_on_yielded_cell",
        ResponseOperation::WaitOnAnyYieldedCell { .. } => "wait_on_any_yielded_cell",
        ResponseOperation::WaitOnYieldedSurfaces { .. } => "wait_on_yielded_surfaces",
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ColdCollectionEvidence {
    schema: String,
    provider_payload: Value,
    expected_response: String,
}

#[derive(Clone, Debug)]
struct ColdCollectionRow {
    frame_id_sha256: String,
    session_id_sha256: String,
    client_intent_id_sha256: String,
    observed_at_unix_nanos: u64,
    surface_sha256: String,
    phase_valid: bool,
    request_phase_atom_ids: Vec<u64>,
    example: CollectionSynthesisExample,
}

fn cold_collection_rows(rows: &[Value]) -> Vec<ColdCollectionRow> {
    let mut output = Vec::new();
    for row in rows {
        let Some(cold_value) = row.get("cold_collection_example") else {
            continue;
        };
        let Ok(cold) = serde_json::from_value::<ColdCollectionEvidence>(cold_value.clone()) else {
            continue;
        };
        if cold.schema != "nando.response-collection-synthesis-example.v1"
            || canonical_json_sha256(&cold).ok().as_deref()
                != row.get("evidence_ref_sha256").and_then(Value::as_str)
        {
            continue;
        }
        let (Some(frame_id), Some(session_id), Some(intent_id), Some(observed_at)) = (
            row.get("frame_id_sha256").and_then(Value::as_str),
            row.get("session_id_sha256").and_then(Value::as_str),
            row.get("client_intent_id_sha256").and_then(Value::as_str),
            row.get("observed_at_unix_nanos").and_then(Value::as_u64),
        ) else {
            continue;
        };
        let Some(surface_sha256) = collection_surface_digest(&cold.provider_payload) else {
            continue;
        };
        output.push(ColdCollectionRow {
            frame_id_sha256: frame_id.to_owned(),
            session_id_sha256: session_id.to_owned(),
            client_intent_id_sha256: intent_id.to_owned(),
            observed_at_unix_nanos: observed_at,
            surface_sha256,
            phase_valid: row
                .get("atoms")
                .and_then(Value::as_array)
                .is_some_and(|atoms| {
                    atoms.iter().any(|atom| {
                        atom.get("kind").and_then(Value::as_str) == Some("collection_shape")
                    }) && atoms.iter().any(|atom| {
                        atom.get("kind").and_then(Value::as_str) == Some("completion_state")
                    })
                }),
            request_phase_atom_ids: row
                .get("atoms")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|atom| {
                    (atom.get("kind").and_then(Value::as_str) == Some("request_phase_atom"))
                        .then(|| atom.get("atom_id").and_then(Value::as_u64))
                        .flatten()
                })
                .collect(),
            example: CollectionSynthesisExample {
                provider_payload: cold.provider_payload,
                expected_response: cold.expected_response,
            },
        });
    }
    output.sort_by(|left, right| {
        (left.observed_at_unix_nanos, &left.frame_id_sha256)
            .cmp(&(right.observed_at_unix_nanos, &right.frame_id_sha256))
    });
    output.dedup_by(|left, right| left.frame_id_sha256 == right.frame_id_sha256);
    output
}

fn read_relation_frame_input(
    path: &Path,
) -> Result<(Vec<RelationFrame>, Vec<ColdCollectionRow>), String> {
    if !path.exists() {
        return Ok((Vec::new(), Vec::new()));
    }
    let file = fs::File::open(path).map_err(|error| format!("open:{}:{error}", path.display()))?;
    let mut frames = Vec::new();
    let mut cold = Vec::new();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|error| format!("read:{}:{error}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(&line)
            .map_err(|error| format!("parse:{}:{}:{error}", path.display(), index + 1))?;
        cold.extend(cold_collection_rows(std::slice::from_ref(&value)));
        frames.push(serde_json::from_value(value).map_err(|error| {
            format!(
                "relation_frame_parse:{}:{}:{error}",
                path.display(),
                index + 1
            )
        })?);
    }
    Ok((frames, cold))
}

fn collection_surface_digest(payload: &Value) -> Option<String> {
    let text = payload
        .get("input")?
        .as_array()?
        .last()?
        .get("output")?
        .as_str()?;
    let root = serde_json::from_str::<Value>(text).ok()?;
    let mut shape = root
        .as_object()?
        .iter()
        .filter_map(|(collection, value)| {
            let rows = value.as_array()?;
            let fields = rows
                .first()?
                .as_object()?
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            Some((collection.clone(), fields))
        })
        .collect::<Vec<_>>();
    shape.sort();
    canonical_json_sha256(&shape).ok()
}

fn collection_families(rows: &[ColdCollectionRow]) -> Vec<Vec<ColdCollectionRow>> {
    let mut shape_buckets = BTreeMap::<String, Vec<ColdCollectionRow>>::new();
    for row in rows {
        let key = serde_json::from_str::<Value>(&row.example.expected_response)
            .ok()
            .map_or_else(|| "plain_text".to_owned(), |value| value_shape(&value));
        shape_buckets.entry(key).or_default().push(row.clone());
    }
    shape_buckets
        .into_values()
        .flat_map(split_collection_bucket_by_behavior)
        .collect()
}

fn split_collection_bucket_by_behavior(
    mut rows: Vec<ColdCollectionRow>,
) -> Vec<Vec<ColdCollectionRow>> {
    const MAX_SEED_PAIRS: usize = 256;
    let mut families = Vec::new();
    while rows.len() >= 2 {
        if synthesize_unique_collection_program(
            &rows
                .iter()
                .map(|row| row.example.clone())
                .collect::<Vec<_>>(),
        )
        .is_ok()
        {
            families.push(rows);
            return families;
        }
        let mut candidates =
            BTreeMap::<String, nando_response_actor::SynthesizedCollectionProgram>::new();
        let mut seen_surfaces = std::collections::BTreeSet::new();
        let seed_indices = rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                seen_surfaces
                    .insert(row.surface_sha256.as_str())
                    .then_some(index)
            })
            .take(8)
            .collect::<Vec<_>>();
        let mut pairs = 0_usize;
        'outer: for (left_position, left) in seed_indices.iter().copied().enumerate() {
            for right in seed_indices.iter().copied().skip(left_position + 1) {
                if rows[left].surface_sha256 == rows[right].surface_sha256 {
                    continue;
                }
                pairs = pairs.saturating_add(1);
                if pairs > MAX_SEED_PAIRS {
                    break 'outer;
                }
                let support = [rows[left].example.clone(), rows[right].example.clone()];
                if let Ok(candidate) = synthesize_unique_collection_program(&support)
                    && let Ok(digest) = canonical_json_sha256(&candidate.program)
                {
                    candidates.entry(digest).or_insert(candidate);
                }
            }
        }
        let best = candidates
            .into_values()
            .filter_map(|candidate| {
                let covered = rows
                    .iter()
                    .enumerate()
                    .filter_map(|(index, row)| {
                        collection_candidate_covers(&candidate, row).then_some(index)
                    })
                    .collect::<Vec<_>>();
                (covered.len() >= 2).then_some((
                    covered.len(),
                    std::cmp::Reverse(candidate.description_length_bytes),
                    covered,
                ))
            })
            .max_by_key(|(coverage, description, _)| (*coverage, *description));
        let Some((_, _, covered)) = best else {
            break;
        };
        let covered = covered
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let mut family = Vec::new();
        let mut remainder = Vec::new();
        for (index, row) in rows.into_iter().enumerate() {
            if covered.contains(&index) {
                family.push(row);
            } else {
                remainder.push(row);
            }
        }
        families.push(family);
        rows = remainder;
    }
    if !rows.is_empty() {
        families.push(rows);
    }
    families
}

fn collection_candidate_covers(
    candidate: &nando_response_actor::SynthesizedCollectionProgram,
    row: &ColdCollectionRow,
) -> bool {
    let execution = execute_response(&candidate.program, "", &row.example.provider_payload);
    execution.status == ResponseExecutionStatus::Executed
        && execution.response.as_deref() == Some(row.example.expected_response.as_str())
        && verify_response_independently(
            &candidate.verifier,
            &row.example.provider_payload,
            execution.response.as_deref().unwrap_or_default(),
        )
        .is_ok()
}

fn value_shape(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(_) => "boolean".to_owned(),
        Value::Number(_) => "number".to_owned(),
        Value::String(_) => "string".to_owned(),
        Value::Array(values) => values.first().map_or_else(
            || "array:empty".to_owned(),
            |value| format!("array:{}", value_shape(value)),
        ),
        Value::Object(values) => {
            let mut shapes = values.values().map(value_shape).collect::<Vec<_>>();
            shapes.sort();
            format!("object:{}", shapes.join(","))
        }
    }
}

fn compile_collection_quarantine_package(rows: &[ColdCollectionRow]) -> Option<ResponsePackage> {
    compile_collection_package(rows, None)
}

fn compile_collection_package(
    rows: &[ColdCollectionRow],
    manifest: Option<&ResponseSupportManifest>,
) -> Option<ResponsePackage> {
    if rows.len() < 2 {
        return None;
    }
    let mut session_order = Vec::<String>::new();
    for row in rows {
        if !session_order.contains(&row.session_id_sha256) {
            session_order.push(row.session_id_sha256.clone());
        }
    }
    let reserved_sessions = if let Some(manifest) = manifest {
        manifest
            .reserved_future_session_ids
            .iter()
            .cloned()
            .collect()
    } else if session_order.len() >= 4 {
        session_order[session_order.len().saturating_sub(3)..]
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
    } else {
        std::collections::BTreeSet::new()
    };
    let support_ids = manifest.map(|manifest| {
        manifest
            .support_frame_ids
            .iter()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>()
    });
    let mut support = rows
        .iter()
        .filter(|row| {
            support_ids.as_ref().map_or_else(
                || !reserved_sessions.contains(&row.session_id_sha256),
                |ids| ids.contains(row.frame_id_sha256.as_str()),
            )
        })
        .collect::<Vec<_>>();
    if support.len() < 2 {
        support = rows.iter().collect();
    }
    let synthesized = synthesize_unique_collection_program(
        &support
            .iter()
            .map(|row| row.example.clone())
            .collect::<Vec<_>>(),
    )
    .ok()?;
    let support_sessions = support
        .iter()
        .map(|row| row.session_id_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let support_intents = support
        .iter()
        .map(|row| row.client_intent_id_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let boundary = manifest.map_or(0, |manifest| manifest.support_boundary_unix_nanos);
    let future = rows
        .iter()
        .filter(|row| {
            if manifest.is_some() {
                row.observed_at_unix_nanos > boundary
                    && !support_sessions.contains(row.session_id_sha256.as_str())
                    && !support_intents.contains(row.client_intent_id_sha256.as_str())
            } else {
                reserved_sessions.contains(&row.session_id_sha256)
            }
        })
        .collect::<Vec<_>>();
    let mut future_accepts = 0_usize;
    let mut wrong_accepts = 0_usize;
    for row in &future {
        let execution = execute_response(&synthesized.program, "", &row.example.provider_payload);
        if execution.status == ResponseExecutionStatus::Executed {
            if execution.response.as_deref() == Some(row.example.expected_response.as_str())
                && verify_response_independently(
                    &synthesized.verifier,
                    &row.example.provider_payload,
                    execution.response.as_deref().unwrap_or_default(),
                )
                .is_ok()
            {
                future_accepts = future_accepts.saturating_add(1);
            } else {
                wrong_accepts = wrong_accepts.saturating_add(1);
            }
        }
    }
    let required = response_program_required_routing_atom_ids(&synthesized.program);
    let digest = canonical_json_sha256(&synthesized.program).ok()?;
    let distinct_sessions = future
        .iter()
        .map(|row| row.session_id_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let distinct_surfaces = rows
        .iter()
        .map(|row| row.surface_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let causal_pass = future_accepts >= 32
        && wrong_accepts == 0
        && distinct_surfaces >= 2
        && future.iter().all(|row| row.phase_valid);
    let state = if manifest.is_some()
        && support.len() >= 32
        && future_accepts >= 32
        && distinct_sessions >= 3
        && distinct_surfaces >= 2
        && wrong_accepts == 0
        && causal_pass
    {
        ResponsePackageState::Active
    } else {
        ResponsePackageState::Quarantine
    };
    let phase_centers = manifest.map_or_else(
        || required.clone(),
        |manifest| manifest.learned_center_atom_ids.clone(),
    );
    Some(ResponsePackage {
        schema: "nando.response-package.v1".to_owned(),
        package_id: manifest.map_or_else(
            || {
                format!(
                    "raw-phase-collection-{}",
                    digest.get(..16).unwrap_or(&digest)
                )
            },
            |manifest| manifest.package_id.clone(),
        ),
        origin: ResponsePackageOrigin::GroundedSynthesis,
        state,
        program: synthesized.program,
        verifier: Some(synthesized.verifier),
        routing_predicates: Vec::new(),
        required_routing_atom_ids: required.clone(),
        phase_centers,
        anti_centers: Vec::new(),
        wave_margin_micro: 1,
        learned_wave_route: None,
        crystallized_operator: None,
        proof: ResponsePackageProof {
            support_rows: support.len(),
            future_rows: future_accepts,
            distinct_sessions,
            distinct_surfaces,
            wrong_accepts,
            runtime_parity_failures: 0,
            exact_cache_overlap: 0,
            wave_causal_pass: causal_pass,
            verifier_schema: COLLECTION_EXTERNAL_VERIFIER_SCHEMA.to_owned(),
        },
    })
}

fn build_collection_support_manifest(
    rows: &[ColdCollectionRow],
    package: &ResponsePackage,
) -> Option<ResponseSupportManifest> {
    if package.proof.support_rows < 32 || package.proof.distinct_surfaces < 2 {
        return None;
    }
    let mut session_order = Vec::<String>::new();
    for row in rows {
        if !session_order.contains(&row.session_id_sha256) {
            session_order.push(row.session_id_sha256.clone());
        }
    }
    if session_order.len() < 4 {
        return None;
    }
    let reserved = session_order[session_order.len() - 3..].to_vec();
    let reserved_set = reserved
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    let support = rows
        .iter()
        .filter(|row| !reserved_set.contains(row.session_id_sha256.as_str()))
        .collect::<Vec<_>>();
    let boundary = support.iter().map(|row| row.observed_at_unix_nanos).max()?;
    let mut request_counts = BTreeMap::<u64, usize>::new();
    for row in &support {
        for atom in &row.request_phase_atom_ids {
            *request_counts.entry(*atom).or_default() += 1;
        }
    }
    let minimum_request_support = support.len().saturating_mul(4).div_ceil(5).max(2);
    let mut learned_centers = package.required_routing_atom_ids.clone();
    learned_centers.extend(
        request_counts
            .into_iter()
            .filter_map(|(atom, count)| (count >= minimum_request_support).then_some(atom)),
    );
    learned_centers.sort_unstable();
    learned_centers.dedup();
    let mut manifest = ResponseSupportManifest {
        schema: RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1.to_owned(),
        package_id: package.package_id.clone(),
        lineage_id: response_package_lineage_id(
            &package.program,
            &package.required_routing_atom_ids,
        ),
        generation: 1,
        routing_refinement_version: ROUTING_REFINEMENT_VERSION,
        supersedes_package_id: None,
        created_at_unix_nanos: unix_now().saturating_mul(1_000_000_000),
        support_boundary_unix_nanos: boundary,
        support_frame_ids: support
            .iter()
            .map(|row| row.frame_id_sha256.clone())
            .collect(),
        support_session_ids: support
            .iter()
            .map(|row| row.session_id_sha256.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect(),
        support_intent_ids: support
            .iter()
            .map(|row| row.client_intent_id_sha256.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect(),
        reserved_future_session_ids: reserved,
        learned_center_atom_ids: learned_centers,
        learned_anti_center_atom_ids: Vec::new(),
        selected_routing_atom_ids: package.required_routing_atom_ids.clone(),
        selected_routing_predicates: Vec::new(),
        split_negative_frame_ids: Vec::new(),
        holdout_negative_frame_ids: Vec::new(),
        split_parent_support_rows: support.len(),
        manifest_sha256: String::new(),
    };
    manifest.manifest_sha256 = response_support_manifest_digest(&manifest).ok()?;
    Some(manifest)
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

fn miner_input_fingerprint(paths: &[&Path]) -> Result<String, String> {
    let rows = paths
        .iter()
        .map(|path| {
            let metadata = fs::metadata(path).ok();
            let modified_unix_nanos = metadata
                .as_ref()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |duration| duration.as_nanos());
            serde_json::json!({
                "path": path,
                "bytes": metadata.as_ref().map_or(0, fs::Metadata::len),
                "modified_unix_nanos": modified_unix_nanos,
            })
        })
        .collect::<Vec<_>>();
    canonical_json_sha256(&rows).map_err(str::to_owned)
}

fn refresh_idle_miner_status(
    status_path: &Path,
    input_fingerprint_sha256: &str,
    elapsed_ms: u64,
) -> Result<bool, String> {
    let Some(mut status) = read_json::<Value>(status_path) else {
        return Ok(false);
    };
    if status
        .get("input_fingerprint_sha256")
        .and_then(Value::as_str)
        != Some(input_fingerprint_sha256)
    {
        return Ok(false);
    }
    let Some(object) = status.as_object_mut() else {
        return Ok(false);
    };
    object.insert("generated_at_unix".to_owned(), Value::from(unix_now()));
    object.insert(
        "cycle_mode".to_owned(),
        Value::String("idle_no_input_change".to_owned()),
    );
    object.insert("cycle_duration_ms".to_owned(), Value::from(elapsed_ms));
    atomic_write_value(status_path, &status)?;
    Ok(true)
}

fn compact_live_support_manifests(
    manifests: &mut Vec<ResponseSupportManifest>,
    generations_per_lineage: usize,
) -> Vec<ResponseSupportManifest> {
    let mut by_lineage = BTreeMap::<String, Vec<usize>>::new();
    for (index, manifest) in manifests.iter().enumerate() {
        if manifest.package_id.starts_with("raw-phase-collection-") {
            continue;
        }
        by_lineage
            .entry(manifest.lineage_id.clone())
            .or_default()
            .push(index);
    }
    let mut keep = std::collections::BTreeSet::new();
    for (lineage_id, mut indices) in by_lineage {
        if lineage_id.is_empty() {
            continue;
        }
        indices.sort_by_key(|index| {
            let manifest = &manifests[*index];
            (
                manifest.generation,
                manifest.created_at_unix_nanos,
                manifest.package_id.clone(),
            )
        });
        keep.extend(
            indices
                .into_iter()
                .rev()
                .take(generations_per_lineage.max(1)),
        );
    }
    let mut retained = Vec::new();
    let mut removed = Vec::new();
    for (index, manifest) in manifests.drain(..).enumerate() {
        if manifest.package_id.starts_with("raw-phase-collection-") || keep.contains(&index) {
            retained.push(manifest);
        } else {
            removed.push(manifest);
        }
    }
    *manifests = retained;
    removed
}

fn archive_support_manifests(
    support_manifests_path: &Path,
    removed: &[ResponseSupportManifest],
) -> Result<(), String> {
    let archive_path =
        support_manifests_path.with_file_name("response-support-manifests.archive.jsonl");
    let mut known = std::collections::BTreeSet::new();
    if let Ok(file) = fs::File::open(&archive_path) {
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            if let Ok(manifest) = serde_json::from_str::<ResponseSupportManifest>(&line) {
                known.insert(manifest.package_id);
            }
        }
    }
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("archive_parent:{}:{error}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&archive_path)
        .map_err(|error| format!("archive_open:{}:{error}", archive_path.display()))?;
    for manifest in removed {
        if !known.insert(manifest.package_id.clone()) {
            continue;
        }
        serde_json::to_writer(&mut file, manifest)
            .map_err(|error| format!("archive_serialize:{error}"))?;
        file.write_all(b"\n")
            .map_err(|error| format!("archive_write:{error}"))?;
    }
    file.sync_all()
        .map_err(|error| format!("archive_sync:{error}"))
}

fn latest_grounded_support_manifests(
    manifests: &[ResponseSupportManifest],
) -> Vec<ResponseSupportManifest> {
    let mut latest = BTreeMap::<String, ResponseSupportManifest>::new();
    for manifest in manifests.iter().filter(|manifest| {
        manifest
            .package_id
            .starts_with(GROUNDED_RESPONSE_PACKAGE_PREFIX)
            && !manifest.lineage_id.is_empty()
    }) {
        let replace = latest.get(&manifest.lineage_id).is_none_or(|current| {
            (
                manifest.generation,
                manifest.created_at_unix_nanos,
                &manifest.package_id,
            ) > (
                current.generation,
                current.created_at_unix_nanos,
                &current.package_id,
            )
        });
        if replace {
            latest.insert(manifest.lineage_id.clone(), manifest.clone());
        }
    }
    latest.into_values().collect()
}

fn manifest_runtime_phase_centers(
    manifest: &ResponseSupportManifest,
    frames: &[RelationFrame],
) -> Vec<u64> {
    let mut centers = manifest.learned_center_atom_ids.clone();
    if !manifest.selected_routing_predicates.is_empty() {
        let support_ids = manifest
            .support_frame_ids
            .iter()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        let cardinality_atoms = frames
            .iter()
            .filter(|frame| support_ids.contains(frame.frame_id_sha256.as_str()))
            .flat_map(|frame| {
                let mut without_cardinalities = frame.clone();
                without_cardinalities
                    .atoms
                    .retain(|atom| !matches!(atom, RelationAtom::Cardinality { .. }));
                let non_cardinality_atoms = relation_frame_routing_atom_ids(&without_cardinalities)
                    .into_iter()
                    .collect::<std::collections::BTreeSet<_>>();
                relation_frame_routing_atom_ids(frame)
                    .into_iter()
                    .filter(move |atom| !non_cardinality_atoms.contains(atom))
            })
            .collect::<std::collections::BTreeSet<_>>();
        centers.retain(|atom| !cardinality_atoms.contains(atom));
        centers.extend(
            manifest
                .selected_routing_predicates
                .iter()
                .map(nando_response_actor::ResponseRoutingPredicate::phase_atom_id),
        );
    }
    centers.sort_unstable();
    centers.dedup();
    centers
}

fn read_registry_revision(path: &Path) -> u64 {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<ResponseRegistry>(&bytes).ok())
        .map_or(0, |registry| registry.revision)
}

#[cfg(test)]
fn package_negative_frame_refs<'a>(
    package: &ResponsePackage,
    support: &[RelationFrame],
    frames: &'a [RelationFrame],
) -> Vec<&'a RelationFrame> {
    let grounded_family_by_frame_id = frames
        .iter()
        .filter_map(|frame| {
            relation_frame_family_id(frame)
                .map(|family_id| (frame.frame_id_sha256.clone(), family_id))
        })
        .collect::<BTreeMap<_, _>>();
    package_negative_frame_refs_with_grounding(
        package,
        support,
        frames,
        &grounded_family_by_frame_id,
    )
}

fn package_negative_frame_refs_with_grounding<'a>(
    package: &ResponsePackage,
    support: &[RelationFrame],
    frames: &'a [RelationFrame],
    grounded_family_by_frame_id: &BTreeMap<String, u64>,
) -> Vec<&'a RelationFrame> {
    let support_family = support.first().and_then(|frame| {
        grounded_family_by_frame_id
            .get(&frame.frame_id_sha256)
            .copied()
    });
    let equivalent_action_event_ids = frames
        .iter()
        .filter(|frame| {
            frame_matches_program_action_contract_with_grounding(
                &package.program,
                frame,
                grounded_family_by_frame_id.contains_key(&frame.frame_id_sha256),
            )
        })
        .map(|frame| frame.event_id_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let expected_completion = match &package.program.operation {
        ResponseOperation::ProjectSelectedValue {
            completion_state, ..
        }
        | ResponseOperation::ProjectStatus {
            completion_state, ..
        }
        | ResponseOperation::ComposeCollection {
            completion_state, ..
        } => completion_state.as_str(),
        _ if response_program_external_verifier_schema(&package.program)
            == Some(SOURCE_VALUE_EXTERNAL_VERIFIER_SCHEMA) =>
        {
            "completed"
        }
        _ => "pending",
    };
    let expected_response_shape = match package.program.operation {
        ResponseOperation::CustomToolCallFromRoles { .. } => "custom_tool_call",
        ResponseOperation::ProjectSelectedValue { .. }
        | ResponseOperation::ProjectStatus { .. }
        | ResponseOperation::ComposeCollection { .. } => "assistant_message",
        _ => "function_call",
    };
    let representation_policy = FrameRepresentationPolicy::from_support(support);
    frames
        .iter()
        .filter(|frame| representation_policy.matches(frame))
        .filter(|frame| {
            let completion_mismatch = !frame.atoms.iter().any(|atom| {
                matches!(atom, RelationAtom::CompletionState { value } if value == expected_completion)
            });
            let response_shape_mismatch = !frame.atoms.iter().any(|atom| {
                matches!(atom, RelationAtom::ResponseShape { value } if value == expected_response_shape)
            });
            let cross_family_positive = frame.verifier_label == Some(true)
                && support_family.is_some()
                && grounded_family_by_frame_id
                    .get(&frame.frame_id_sha256)
                    .is_some_and(|family| Some(*family) != support_family);
            (frame.verifier_label == Some(false)
                && !equivalent_action_event_ids.contains(frame.event_id_sha256.as_str())
                && !frame_matches_program_action_contract_with_grounding(
                    &package.program,
                    frame,
                    grounded_family_by_frame_id.contains_key(&frame.frame_id_sha256),
                ))
                || completion_mismatch
                || response_shape_mismatch
                || cross_family_positive
        })
        .collect()
}

fn relation_frame_family_id(frame: &RelationFrame) -> Option<u64> {
    let hypotheses = ground_roles(frame);
    (hypotheses.len() == 1 && hypotheses[0].competing_binding_count == 0)
        .then(|| hypotheses[0].frame_family_id)
}

fn learned_discriminating_anti_centers(
    support: &[RelationFrame],
    negatives: &[&RelationFrame],
) -> Vec<u64> {
    let positive_union = support
        .iter()
        .flat_map(relation_frame_routing_atom_ids)
        .collect::<std::collections::BTreeSet<_>>();
    let mut negative_common = negatives
        .first()
        .map(|frame| {
            relation_frame_routing_atom_ids(frame)
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>()
        })
        .unwrap_or_default();
    for frame in negatives.iter().skip(1) {
        let atoms = relation_frame_routing_atom_ids(frame)
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        negative_common.retain(|atom| atoms.contains(atom));
    }
    negative_common
        .difference(&positive_union)
        .copied()
        .collect()
}

fn routed_counterexample_summary(frame: &RelationFrame) -> Value {
    let mut call_shape = "missing";
    let mut completion = "missing";
    let mut response_shape = "missing";
    let mut tool_kind = "missing";
    let mut action = "none".to_owned();
    let mut cardinalities = BTreeMap::new();
    for atom in &frame.atoms {
        match atom {
            RelationAtom::ObservationCallShape { value } => call_shape = value,
            RelationAtom::CompletionState { value } => completion = value,
            RelationAtom::ResponseShape { value } => response_shape = value,
            RelationAtom::ToolKind { value } => tool_kind = value,
            RelationAtom::ActionFunction { value } => action = format!("function:{value}"),
            RelationAtom::ActionCustomTool { value } => {
                action = format!("custom_tool:{value}");
            }
            RelationAtom::Cardinality { role, count } => {
                cardinalities.insert(role.clone(), *count);
            }
            _ => {}
        }
    }
    serde_json::json!({
        "frame_id_sha256": frame.frame_id_sha256,
        "session_id_sha256": frame.session_id_sha256,
        "verifier_label": frame.verifier_label,
        "observation_call_shape": call_shape,
        "completion_state": completion,
        "proof_only_next_response_shape": response_shape,
        "tool_kind": tool_kind,
        "proof_only_competing_action": action,
        "cardinalities": cardinalities,
    })
}

fn grounded_family_report(family_id: u64, frames: &[RelationFrame]) -> Value {
    let positive_rows = frames
        .iter()
        .filter(|frame| frame.verifier_label == Some(true))
        .count();
    let negative_rows = frames
        .iter()
        .filter(|frame| frame.verifier_label == Some(false))
        .count();
    let deduped_token_sum = |eligible: fn(&RelationFrame) -> bool| {
        frames
            .iter()
            .filter(|frame| eligible(frame))
            .fold(BTreeMap::<&str, u64>::new(), |mut by_event, frame| {
                by_event
                    .entry(frame.event_id_sha256.as_str())
                    .and_modify(|tokens| {
                        *tokens = (*tokens).max(frame.estimated_input_tokens);
                    })
                    .or_insert(frame.estimated_input_tokens);
                by_event
            })
            .into_values()
            .fold(0_u64, u64::saturating_add)
    };
    let total_estimated_input_tokens = deduped_token_sum(|_| true);
    let positive_estimated_input_tokens =
        deduped_token_sum(|frame| frame.verifier_label == Some(true));
    let sessions = frames
        .iter()
        .map(|frame| frame.session_id_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let surfaces = frames
        .iter()
        .flat_map(|frame| frame.atoms.iter())
        .filter_map(|atom| match atom {
            RelationAtom::ToolKind { value } => Some(value.as_str()),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let mut action_symbols = std::collections::BTreeSet::new();
    let mut selector_kinds = std::collections::BTreeSet::new();
    let mut selectors = std::collections::BTreeSet::new();
    let mut call_shapes = std::collections::BTreeSet::new();
    for frame in frames {
        for atom in &frame.atoms {
            match atom {
                RelationAtom::ActionFunction { value } => {
                    action_symbols.insert(format!("function:{value}"));
                }
                RelationAtom::ActionCustomTool { value } => {
                    action_symbols.insert(format!("custom_tool:{value}"));
                }
                RelationAtom::ActionValueProjection { format, renderer } => {
                    action_symbols.insert(format!(
                        "value_projection:{format:?}:{}",
                        if renderer.is_direct() {
                            "direct"
                        } else {
                            "template"
                        }
                    ));
                }
                RelationAtom::ObservationSelector { selector, .. } => {
                    selectors.insert(
                        serde_json::to_string(selector).unwrap_or_else(|_| "null".to_owned()),
                    );
                    selector_kinds.insert(match selector {
                        nando_response_actor::ResponseValueSelector::ContinuationHandle {
                            ..
                        } => "continuation_handle",
                        nando_response_actor::ResponseValueSelector::UniqueScalar { .. } => {
                            "unique_scalar"
                        }
                        nando_response_actor::ResponseValueSelector::UniqueTurnScalar { .. } => {
                            "unique_turn_scalar"
                        }
                        nando_response_actor::ResponseValueSelector::ContentLinePrefix {
                            ..
                        } => "content_line_prefix",
                        nando_response_actor::ResponseValueSelector::JsonField { .. } => {
                            "json_field"
                        }
                        nando_response_actor::ResponseValueSelector::JsonScalarOrdinal {
                            ..
                        } => "json_scalar_ordinal",
                        nando_response_actor::ResponseValueSelector::UniqueTurnJsonField {
                            ..
                        } => "unique_turn_json_field",
                        nando_response_actor::ResponseValueSelector::UniqueActiveTurnJsonField {
                            ..
                        } => "unique_active_turn_json_field",
                        nando_response_actor::ResponseValueSelector::RequestReferencedJsonField {
                            ..
                        } => "request_referenced_json_field",
                        nando_response_actor::ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
                            ..
                        } => "request_referenced_json_field_ordinal",
                        nando_response_actor::ResponseValueSelector::TurnOutputLine { .. } => {
                            "turn_output_line"
                        }
                        nando_response_actor::ResponseValueSelector::TurnOutputScalarOrdinal {
                            ..
                        } => "turn_output_scalar_ordinal",
                        nando_response_actor::ResponseValueSelector::LatestTurnOutputLine {
                            ..
                        } => "latest_turn_output_line",
                        nando_response_actor::ResponseValueSelector::LatestTurnOutputScalarOrdinal {
                            ..
                        } => "latest_turn_output_scalar_ordinal",
                        nando_response_actor::ResponseValueSelector::LatestTurnOutputScalarFromEnd {
                            ..
                        } => "latest_turn_output_scalar_from_end",
                        nando_response_actor::ResponseValueSelector::CommandOutputBody => {
                            "command_output_body"
                        }
                        nando_response_actor::ResponseValueSelector::RequestLastToken => {
                            "request_last_token"
                        }
                        nando_response_actor::ResponseValueSelector::RequestUniqueLiteral => {
                            "request_unique_literal"
                        }
                    });
                }
                RelationAtom::ObservationCallShape { value } => {
                    call_shapes.insert(value.as_str());
                }
                _ => {}
            }
        }
    }
    serde_json::json!({
        "family_id": family_id,
        "rows": frames.len(),
        "positive_rows": positive_rows,
        "negative_rows": negative_rows,
        "total_estimated_input_tokens": total_estimated_input_tokens,
        "positive_estimated_input_tokens": positive_estimated_input_tokens,
        "sessions": sessions,
        "surfaces": surfaces,
        "action_symbols": action_symbols,
        "selector_kinds": selector_kinds,
        "selectors": selectors
            .into_iter()
            .filter_map(|selector| serde_json::from_str::<Value>(&selector).ok())
            .collect::<Vec<_>>(),
        "observation_call_shapes": call_shapes,
        "support_floor_reached": positive_rows >= 32,
    })
}

fn token_opportunity_report(frames: &[RelationFrame]) -> Value {
    let mut by_event = BTreeMap::<&str, u64>::new();
    let mut positive_by_event = BTreeMap::<&str, u64>::new();
    for frame in frames {
        by_event
            .entry(frame.event_id_sha256.as_str())
            .and_modify(|tokens| *tokens = (*tokens).max(frame.estimated_input_tokens))
            .or_insert(frame.estimated_input_tokens);
        if frame.verifier_label == Some(true) {
            positive_by_event
                .entry(frame.event_id_sha256.as_str())
                .and_modify(|tokens| *tokens = (*tokens).max(frame.estimated_input_tokens))
                .or_insert(frame.estimated_input_tokens);
        }
    }
    let sum = |values: &BTreeMap<&str, u64>| values.values().copied().fold(0, u64::saturating_add);
    serde_json::json!({
        "dedupe_key": "event_id_sha256",
        "raw_rows": frames.len(),
        "deduplicated_events": by_event.len(),
        "deduplicated_input_tokens": sum(&by_event),
        "positive_deduplicated_events": positive_by_event.len(),
        "positive_deduplicated_input_tokens": sum(&positive_by_event),
    })
}

fn verified_future_sessions_for_self_training(
    future: &[RelationFrame],
) -> std::collections::BTreeSet<String> {
    if future.len() < SELF_TRAINING_MIN_VERIFIED_FUTURE_ROWS {
        return std::collections::BTreeSet::new();
    }
    let mut sessions = BTreeMap::<String, (u64, usize)>::new();
    for frame in future
        .iter()
        .filter(|frame| frame.verifier_label == Some(true))
    {
        sessions
            .entry(frame.session_id_sha256.clone())
            .and_modify(|(latest, rows)| {
                *latest = (*latest).max(frame.observed_at_unix_nanos);
                *rows = rows.saturating_add(1);
            })
            .or_insert((frame.observed_at_unix_nanos, 1));
    }
    if sessions.len() < SELF_TRAINING_MIN_VERIFIED_FUTURE_SESSIONS {
        return std::collections::BTreeSet::new();
    }
    let mut ordered = sessions.into_iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.1.0.cmp(&right.1.0).then_with(|| left.0.cmp(&right.0)));
    let training_limit = ordered
        .len()
        .saturating_sub(SELF_TRAINING_RESERVED_FUTURE_SESSIONS);
    let mut selected = std::collections::BTreeSet::new();
    let mut selected_rows = 0_usize;
    for (session, (_, rows)) in ordered.into_iter().take(training_limit) {
        selected.insert(session);
        selected_rows = selected_rows.saturating_add(rows);
        if selected_rows >= SELF_TRAINING_MIN_ROLLOVER_ROWS {
            break;
        }
    }
    if selected_rows < SELF_TRAINING_MIN_ROLLOVER_ROWS {
        return std::collections::BTreeSet::new();
    }
    selected
}

fn evidence_refresh_improves(
    current: &ResponseSupportManifest,
    candidate: &ResponseSupportManifest,
) -> bool {
    candidate.generation > current.generation
        && candidate.supersedes_package_id.as_deref() == Some(current.package_id.as_str())
        && candidate.support_frame_ids.len() >= 32
        && ((candidate.routing_refinement_version > current.routing_refinement_version)
            || (candidate.reserved_future_session_ids.len()
                > current.reserved_future_session_ids.len()
                && candidate.support_frame_ids != current.support_frame_ids))
}

fn rollover_manifest_improves(
    current: &ResponseSupportManifest,
    candidate: &ResponseSupportManifest,
) -> bool {
    if candidate.generation <= current.generation
        || candidate.supersedes_package_id.as_deref() != Some(current.package_id.as_str())
        || candidate.routing_refinement_version < current.routing_refinement_version
    {
        return false;
    }
    // Positive centers are re-estimated from the selected support rows and can
    // drift without changing what the package is allowed to execute. Treating
    // that drift as a new contract repeatedly moves the frozen-future boundary.
    let routing_contract_changed = candidate.learned_anti_center_atom_ids
        != current.learned_anti_center_atom_ids
        || candidate.selected_routing_atom_ids != current.selected_routing_atom_ids
        || candidate.selected_routing_predicates != current.selected_routing_predicates;
    let materially_more_support =
        candidate.support_frame_ids.len() >= current.support_frame_ids.len().saturating_add(32);
    routing_contract_changed || materially_more_support
}

fn dedupe_frame_refs(frames: &mut Vec<&RelationFrame>) {
    frames.sort_unstable_by_key(|frame| frame.frame_id_sha256.as_str());
    frames.dedup_by_key(|frame| frame.frame_id_sha256.as_str());
}

fn action_value_sha256(frame: &RelationFrame) -> Option<&str> {
    frame.atoms.iter().find_map(|atom| match atom {
        RelationAtom::TypedSlot {
            source: AtomSource::Action,
            value_sha256,
            ..
        } if is_sha256(value_sha256) => Some(value_sha256.as_str()),
        _ => None,
    })
}

fn project_status_response_shape_is_valid(frame: &RelationFrame) -> bool {
    if !frame
        .atoms
        .iter()
        .any(|atom| matches!(atom, RelationAtom::ActionStatusProjection { .. }))
    {
        return true;
    }
    frame
        .atoms
        .iter()
        .filter(|atom| matches!(atom, RelationAtom::ResponseShape { .. }))
        .count()
        == 1
        && frame.atoms.iter().any(|atom| {
            matches!(atom, RelationAtom::ResponseShape { value } if value == "assistant_message")
        })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn exact_package_runtime_parity(
    package: &nando_response_actor::ResponsePackage,
    frames: &[RelationFrame],
    registry_revision: u64,
) -> (usize, usize, Vec<Value>) {
    if frames.is_empty() {
        return (0, 0, Vec::new());
    }
    let Some(verifier) = package.verifier.as_ref() else {
        return (frames.len(), frames.len(), Vec::new());
    };
    let actor_program_sha256 = response_actor_program_digest(&package.program).unwrap_or_default();
    let verifier_program_sha256 =
        response_independent_verifier_program_digest(verifier).unwrap_or_default();
    let mut failures = 0_usize;
    let mut receipts = Vec::new();
    for (index, frame) in frames.iter().enumerate() {
        let payload = parity_provider_payload(package, frame, index);
        let execution = execute_response(&package.program, "", &payload);
        let independently_verified_output = execution
            .response
            .as_deref()
            .filter(|response| verify_response_independently(verifier, &payload, response).is_ok());
        let passed = relation_frame_routes_to_package(package, frame)
            && execution.status == ResponseExecutionStatus::Executed
            && independently_verified_output.is_some();
        if !passed {
            failures = failures.saturating_add(1);
            continue;
        }
        let Some(evidence_sha256) = canonical_json_sha256(&payload).ok() else {
            failures = failures.saturating_add(1);
            continue;
        };
        let Some(output_sha256) =
            independently_verified_output.and_then(|output| canonical_json_sha256(&output).ok())
        else {
            failures = failures.saturating_add(1);
            continue;
        };
        receipts.push(serde_json::json!({
            "schema": "nando.response-runtime-parity-receipt.v1",
            "package_id": package.package_id,
            "registry_revision": registry_revision,
            "frame_id_sha256": frame.frame_id_sha256,
            "actor_program_sha256": actor_program_sha256,
            "independent_verifier_program_sha256": verifier_program_sha256,
            "evidence_sha256": evidence_sha256,
            "output_sha256": output_sha256,
            "result": "pass",
        }));
    }
    (frames.len(), failures, receipts)
}

fn collection_runtime_parity(
    package: &ResponsePackage,
    rows: &[ColdCollectionRow],
    frames: &[RelationFrame],
    registry_revision: u64,
) -> (usize, usize, Vec<Value>) {
    let Some(verifier) = package.verifier.as_ref() else {
        return (frames.len(), frames.len(), Vec::new());
    };
    let by_frame = rows
        .iter()
        .map(|row| (row.frame_id_sha256.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let actor_sha256 = response_actor_program_digest(&package.program).unwrap_or_default();
    let verifier_sha256 =
        response_independent_verifier_program_digest(verifier).unwrap_or_default();
    let mut failures = 0_usize;
    let mut receipts = Vec::new();
    for frame in frames {
        let Some(row) = by_frame.get(frame.frame_id_sha256.as_str()) else {
            failures = failures.saturating_add(1);
            continue;
        };
        let execution = execute_response(&package.program, "", &row.example.provider_payload);
        let passed = relation_frame_routes_to_package(package, frame)
            && execution.status == ResponseExecutionStatus::Executed
            && execution.response.as_deref() == Some(row.example.expected_response.as_str())
            && verify_response_independently(
                verifier,
                &row.example.provider_payload,
                execution.response.as_deref().unwrap_or_default(),
            )
            .is_ok();
        if !passed {
            failures = failures.saturating_add(1);
            continue;
        }
        receipts.push(serde_json::json!({
            "schema": "nando.response-runtime-parity-receipt.v1",
            "package_id": package.package_id,
            "registry_revision": registry_revision,
            "frame_id_sha256": frame.frame_id_sha256,
            "actor_program_sha256": actor_sha256,
            "independent_verifier_program_sha256": verifier_sha256,
            "evidence_sha256": canonical_json_sha256(&row.example.provider_payload).unwrap_or_default(),
            "output_sha256": execution.response.as_ref().and_then(|output| canonical_json_sha256(output).ok()).unwrap_or_default(),
            "result": "pass",
        }));
    }
    (frames.len(), failures, receipts)
}

fn parity_provider_payload(
    package: &ResponsePackage,
    frame: &RelationFrame,
    index: usize,
) -> Value {
    let cardinality = |role: &str| {
        frame
            .atoms
            .iter()
            .find_map(|atom| match atom {
                RelationAtom::Cardinality {
                    role: atom_role,
                    count,
                } if atom_role == role => Some(*count as usize),
                _ => None,
            })
            .unwrap_or(0)
    };
    let calls = cardinality("turn_call_count_band").max(1);
    let projection_selector = match &package.program.operation {
        ResponseOperation::ProjectSelectedValue { selector, .. } => Some(selector),
        _ => None,
    };
    let status_selector = match &package.program.operation {
        ResponseOperation::ProjectStatus { selector, .. } => Some(selector),
        _ => None,
    };
    let source_value = frame.atoms.iter().find_map(|atom| match atom {
        RelationAtom::TypedSlot {
            value_type,
            source: AtomSource::Observation,
            ..
        } if projection_selector.is_some()
            || status_selector.is_some()
            || response_program_external_verifier_schema(&package.program)
                == Some(SOURCE_VALUE_EXTERNAL_VERIFIER_SCHEMA) =>
        {
            Some(match value_type {
                AtomValueType::Identifier => Value::String(format!("parity-{index}")),
                AtomValueType::String => Value::String(format!("parity value {index}")),
                AtomValueType::Integer => Value::from(index.saturating_add(100)),
                AtomValueType::Boolean => Value::Bool(index.is_multiple_of(2)),
                AtomValueType::Collection => Value::Null,
            })
        }
        _ => None,
    });
    let custom_tool = response_program_external_verifier_schema(&package.program)
        == Some(nando_response_actor::CUSTOM_TOOL_EXTERNAL_VERIFIER_SCHEMA);
    let outputs = cardinality("turn_output_count_band")
        .max(usize::from(source_value.is_some() || custom_tool));
    let pending = cardinality("turn_pending_count_band").min(outputs);
    let messages = cardinality("turn_message_count_band");
    let shapes = cardinality("turn_call_shape_count_band").max(1);
    let observation_call_shape = frame
        .atoms
        .iter()
        .find_map(|atom| match atom {
            RelationAtom::ObservationCallShape { value } => Some(value.as_str()),
            _ => None,
        })
        .unwrap_or("custom_tool_call");
    let request_content = if matches!(
        projection_selector,
        Some(ResponseValueSelector::RequestLastToken)
    ) {
        source_value.as_ref().map_or_else(
            || "runtime parity".to_owned(),
            |value| {
                format!(
                    "runtime parity {}",
                    value
                        .as_str()
                        .map_or_else(|| value.to_string(), str::to_owned)
                )
            },
        )
    } else if matches!(
        projection_selector,
        Some(ResponseValueSelector::RequestUniqueLiteral)
    ) {
        source_value.as_ref().map_or_else(
            || "runtime parity".to_owned(),
            |value| {
                format!(
                    "runtime parity '{}'",
                    value
                        .as_str()
                        .map_or_else(|| value.to_string(), str::to_owned)
                )
            },
        )
    } else if let Some(ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
        ordinal, ..
    }) = projection_selector.or(status_selector)
    {
        (0..=*ordinal).map(|index| format!("role_{index}")).fold(
            "runtime parity".to_owned(),
            |mut request, role| {
                request.push(' ');
                request.push_str(&role);
                request
            },
        )
    } else if matches!(
        projection_selector.or(status_selector),
        Some(ResponseValueSelector::RequestReferencedJsonField { .. })
    ) {
        "runtime parity selected".to_owned()
    } else {
        "runtime parity".to_owned()
    };
    let mut input = vec![serde_json::json!({
        "type": "message",
        "role": "user",
        "content": request_content,
    })];
    input.extend((0..messages).map(|_| {
        serde_json::json!({
            "type": "message",
            "role": "assistant",
            "content": "progress",
        })
    }));
    input.extend((0..calls).map(|call| {
        if observation_call_shape == "function_call" {
            serde_json::json!({
                "type": "function_call",
                "name": format!("shape-{}", call % shapes),
                "call_id": format!("parity-{index}-{call}"),
                "arguments": "{}",
            })
        } else {
            serde_json::json!({
                "type": "custom_tool_call",
                "name": format!("shape-{}", call % shapes),
                "call_id": format!("parity-{index}-{call}"),
                "input": "run",
            })
        }
    }));
    input.extend((0..outputs).map(|output| {
        let is_last = output + 1 == outputs;
        let is_pending = !custom_tool
            && projection_selector.is_none()
            && source_value.is_none()
            && (is_last || output + 1 < pending);
        let output_text = if is_last {
            source_value.as_ref().map(|value| {
                serde_json::to_string(&serde_json::json!({"value": value})).unwrap_or_default()
            })
        } else {
            None
        };
        let output_value = if let (true, Some(selector), Some(value)) =
            (is_last, projection_selector, source_value.as_ref())
        {
            parity_projection_output(selector, value)
        } else if let (true, Some(selector), Some(value)) =
            (is_last, status_selector, source_value.as_ref())
        {
            Value::String(parity_provider_output(selector, value))
        } else if custom_tool && is_last {
            serde_json::json!([{
                "type": "text",
                "text": format!("SESSION_ID={}", index.saturating_add(100)),
            }])
        } else if is_pending {
            Value::String(format!(
                "Script running with cell ID parity-{index}-{output}\n"
            ))
        } else {
            Value::String(output_text.unwrap_or_else(|| "completed".to_owned()))
        };
        serde_json::json!({
            "type": if observation_call_shape == "function_call" {
                "function_call_output"
            } else {
                "custom_tool_call_output"
            },
            "call_id": format!("parity-{index}-{}", output.min(calls.saturating_sub(1))),
            "output": output_value,
        })
    }));
    serde_json::json!({"input": input})
}

fn parity_provider_output(selector: &ResponseValueSelector, value: &Value) -> String {
    match selector {
        ResponseValueSelector::ContinuationHandle { .. } => {
            let value = value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned);
            format!("Script running with cell ID {value}")
        }
        ResponseValueSelector::UniqueScalar { .. }
        | ResponseValueSelector::UniqueTurnScalar { .. } => value.to_string(),
        ResponseValueSelector::ContentLinePrefix { prefix, .. } => {
            format!("{prefix}{value}")
        }
        ResponseValueSelector::JsonField { field, .. }
        | ResponseValueSelector::UniqueTurnJsonField { field, .. }
        | ResponseValueSelector::UniqueActiveTurnJsonField { field, .. } => {
            let mut object = serde_json::Map::new();
            object.insert(field.clone(), value.clone());
            serde_json::to_string(&Value::Object(object)).unwrap_or_else(|_| "{}".to_owned())
        }
        ResponseValueSelector::RequestReferencedJsonField { .. } => {
            serde_json::json!({"selected": value}).to_string()
        }
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal,
            value_type,
        } => parity_request_referenced_ordinal_output(*ordinal, *value_type, value),
        ResponseValueSelector::JsonScalarOrdinal {
            ordinal,
            value_type,
        } => parity_scalar_ordinal_output(*ordinal, *value_type, value),
        ResponseValueSelector::TurnOutputScalarOrdinal {
            scalar_ordinal,
            value_type,
            ..
        }
        | ResponseValueSelector::LatestTurnOutputScalarOrdinal {
            scalar_ordinal,
            value_type,
        } => parity_scalar_ordinal_output(*scalar_ordinal, *value_type, value),
        ResponseValueSelector::LatestTurnOutputScalarFromEnd {
            reverse_ordinal,
            value_type,
        } => parity_scalar_reverse_ordinal_output(*reverse_ordinal, *value_type, value),
        ResponseValueSelector::TurnOutputLine { line_index, .. }
        | ResponseValueSelector::LatestTurnOutputLine { line_index, .. } => {
            let value = value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned);
            std::iter::repeat_n("parity line", usize::from(*line_index))
                .chain(std::iter::once(value.as_str()))
                .collect::<Vec<_>>()
                .join("\n")
        }
        ResponseValueSelector::CommandOutputBody => {
            let value = value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned);
            format!("Script completed\nOutput:\n{value}")
        }
        ResponseValueSelector::RequestLastToken => value
            .as_str()
            .map_or_else(|| value.to_string(), str::to_owned),
        ResponseValueSelector::RequestUniqueLiteral => value
            .as_str()
            .map_or_else(|| value.to_string(), str::to_owned),
    }
}

fn parity_projection_output(
    selector: &nando_response_actor::ResponseValueSelector,
    value: &Value,
) -> Value {
    match selector {
        nando_response_actor::ResponseValueSelector::ContinuationHandle { .. } => {
            let value = value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned);
            Value::String(format!("Script running with cell ID {value}"))
        }
        nando_response_actor::ResponseValueSelector::UniqueScalar { .. }
        | nando_response_actor::ResponseValueSelector::UniqueTurnScalar { .. } => {
            Value::String(serde_json::to_string(value).unwrap_or_else(|_| "null".to_owned()))
        }
        nando_response_actor::ResponseValueSelector::ContentLinePrefix { prefix, .. } => {
            let value = value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned);
            Value::String(format!("{prefix}{value}"))
        }
        nando_response_actor::ResponseValueSelector::JsonField { field, .. }
        | nando_response_actor::ResponseValueSelector::UniqueTurnJsonField { field, .. }
        | nando_response_actor::ResponseValueSelector::UniqueActiveTurnJsonField {
            field, ..
        } => {
            let mut object = serde_json::Map::new();
            object.insert(field.clone(), value.clone());
            Value::String(
                serde_json::to_string(&Value::Object(object)).unwrap_or_else(|_| "null".to_owned()),
            )
        }
        nando_response_actor::ResponseValueSelector::RequestReferencedJsonField { .. } => {
            Value::String(serde_json::json!({"selected": value}).to_string())
        }
        nando_response_actor::ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal,
            value_type,
        } => Value::String(parity_request_referenced_ordinal_output(
            *ordinal,
            *value_type,
            value,
        )),
        nando_response_actor::ResponseValueSelector::JsonScalarOrdinal {
            ordinal,
            value_type,
        } => Value::String(parity_scalar_ordinal_output(*ordinal, *value_type, value)),
        nando_response_actor::ResponseValueSelector::TurnOutputScalarOrdinal {
            scalar_ordinal,
            value_type,
            ..
        }
        | nando_response_actor::ResponseValueSelector::LatestTurnOutputScalarOrdinal {
            scalar_ordinal,
            value_type,
        } => Value::String(parity_scalar_ordinal_output(
            *scalar_ordinal,
            *value_type,
            value,
        )),
        nando_response_actor::ResponseValueSelector::LatestTurnOutputScalarFromEnd {
            reverse_ordinal,
            value_type,
        } => Value::String(parity_scalar_reverse_ordinal_output(
            *reverse_ordinal,
            *value_type,
            value,
        )),
        nando_response_actor::ResponseValueSelector::TurnOutputLine { line_index, .. }
        | nando_response_actor::ResponseValueSelector::LatestTurnOutputLine {
            line_index, ..
        } => {
            let value = value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned);
            Value::String(
                std::iter::repeat_n("parity line", usize::from(*line_index))
                    .chain(std::iter::once(value.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        }
        nando_response_actor::ResponseValueSelector::CommandOutputBody => {
            let value = value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned);
            Value::String(format!("Script completed\nOutput:\n{value}"))
        }
        nando_response_actor::ResponseValueSelector::RequestLastToken => Value::String(
            value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned),
        ),
        nando_response_actor::ResponseValueSelector::RequestUniqueLiteral => Value::String(
            value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned),
        ),
    }
}

fn parity_request_referenced_ordinal_output(
    ordinal: u16,
    value_type: AtomValueType,
    value: &Value,
) -> String {
    let filler = match value_type {
        AtomValueType::String | AtomValueType::Identifier => Value::String(String::new()),
        AtomValueType::Integer => Value::from(0),
        AtomValueType::Boolean => Value::Bool(false),
        AtomValueType::Collection => Value::Null,
    };
    let mut object = serde_json::Map::new();
    for index in 0..=ordinal {
        object.insert(
            format!("role_{index}"),
            if index == ordinal {
                value.clone()
            } else {
                filler.clone()
            },
        );
    }
    serde_json::to_string(&Value::Object(object)).unwrap_or_else(|_| "{}".to_owned())
}

fn parity_scalar_ordinal_output(ordinal: u16, value_type: AtomValueType, value: &Value) -> String {
    let filler = match value_type {
        AtomValueType::String | AtomValueType::Identifier => Value::String(String::new()),
        AtomValueType::Integer => Value::from(0),
        AtomValueType::Boolean => Value::Bool(false),
        AtomValueType::Collection => Value::Null,
    };
    let mut values = vec![filler; usize::from(ordinal)];
    values.push(value.clone());
    serde_json::json!({"values": values}).to_string()
}

fn parity_scalar_reverse_ordinal_output(
    reverse_ordinal: u16,
    value_type: AtomValueType,
    value: &Value,
) -> String {
    let filler = match value_type {
        AtomValueType::String | AtomValueType::Identifier => Value::String(String::new()),
        AtomValueType::Integer => Value::from(0),
        AtomValueType::Boolean => Value::Bool(false),
        AtomValueType::Collection => Value::Null,
    };
    let mut values = vec![value.clone()];
    values.extend(std::iter::repeat_n(filler, usize::from(reverse_ordinal)));
    serde_json::to_string(&values).unwrap_or_default()
}

fn exact_package_hard_negative_accepts(package: &nando_response_actor::ResponsePackage) -> usize {
    let continuation_outputs = vec![
        Value::String("completed successfully".to_owned()),
        Value::String(
            "Script running with cell ID first\nScript running with cell ID second\n".to_owned(),
        ),
        Value::String("Script running with cell ID !!!\n".to_owned()),
        Value::String(String::new()),
    ];
    let source_value_outputs = vec![
        Value::String("{}".to_owned()),
        Value::String("[]".to_owned()),
        Value::String("{\"left\":1,\"right\":2}".to_owned()),
        Value::String("null".to_owned()),
        Value::String("1.5".to_owned()),
        Value::String("ambiguous\nmultiline".to_owned()),
    ];
    let status_outputs = vec![
        Value::String("{\"other\":0}".to_owned()),
        Value::String("{\"exit_code\":1000001}".to_owned()),
        Value::String("{\"exit_code\":-1}".to_owned()),
        Value::String("{\"exit_code\":true}".to_owned()),
        Value::String("{\"exit_code\":0}\n{\"exit_code\":1}".to_owned()),
        serde_json::json!([{"type":"unknown_text","text":"{\"exit_code\":0}"}]),
    ];
    let custom_tool_outputs = vec![
        serde_json::json!([]),
        serde_json::json!([{"type":"text","text":"completed"}]),
        serde_json::json!([
            {"type":"text","text":"SESSION_ID=1"},
            {"type":"text","text":"SESSION_ID=2"}
        ]),
        serde_json::json!([{"type":"text","text":"SESSION_ID=invalid integer"}]),
    ];
    let collection_outputs = vec![
        Value::String("{}".to_owned()),
        Value::String("{\"left\":[],\"right\":[]}".to_owned()),
        Value::String("{\"rows\":[]}".to_owned()),
        Value::String("{\"rows\":[{\"left\":\"keep\",\"right\":\"keep\",\"value\":1}]}".to_owned()),
        Value::String("{\"rows\":[{\"kind\":\"keep\"},{\"other\":\"keep\"}]}".to_owned()),
    ];
    let schema = response_program_external_verifier_schema(&package.program);
    let outputs = if matches!(
        schema,
        Some(SOURCE_VALUE_EXTERNAL_VERIFIER_SCHEMA | VALUE_PROJECTION_EXTERNAL_VERIFIER_SCHEMA)
    ) {
        source_value_outputs
    } else if schema == Some(COLLECTION_EXTERNAL_VERIFIER_SCHEMA) {
        collection_outputs
    } else if schema == Some("status_projection_external_evidence.v1") {
        status_outputs
    } else if schema == Some(nando_response_actor::CUSTOM_TOOL_EXTERNAL_VERIFIER_SCHEMA) {
        custom_tool_outputs
    } else {
        continuation_outputs
    };
    outputs
        .into_iter()
        .filter(|output| {
            let payload = serde_json::json!({
                "input": [{
                    "type": "function_call_output",
                    "output": output,
                }]
            });
            let execution = execute_response(&package.program, "", &payload);
            execution.response.as_deref().is_some_and(|response| {
                package.verifier.as_ref().is_some_and(|verifier| {
                    verify_response_independently(verifier, &payload, response).is_ok()
                })
            })
        })
        .count()
}

fn causal_proof_passes(path: &Path) -> bool {
    let Some(proof) = fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
    else {
        return false;
    };
    let number = |key: &str| proof.get(key).and_then(Value::as_u64);
    proof.get("schema").and_then(Value::as_str) == Some("nando.response-wave-causal-proof.v1")
        && proof.get("verdict").and_then(Value::as_str) == Some("PASS")
        && number("heldout_correct") == number("heldout_total")
        && number("heldout_total").is_some_and(|total| total >= 32)
        && number("full_phase_exact_checks")
            .zip(number("no_phase_exact_checks"))
            .is_some_and(|(full, no_phase)| full < no_phase)
        && number("full_phase_exact_checks")
            .zip(number("shuffled_phase_exact_checks"))
            .is_some_and(|(full, shuffled)| full < shuffled)
}

fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    atomic_write_value(
        path,
        &serde_json::to_value(value).map_err(|error| error.to_string())?,
    )
}

fn atomic_write_value(path: &Path, value: &Value) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("no_parent:{}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| format!("mkdir:{}:{error}", parent.display()))?;
    let temporary = parent.join(format!(
        ".{}.new",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("registry")
    ));
    let mut file = fs::File::create(&temporary)
        .map_err(|error| format!("create:{}:{error}", temporary.display()))?;
    serde_json::to_writer_pretty(&mut file, value)
        .map_err(|error| format!("serialize:{}:{error}", temporary.display()))?;
    file.write_all(b"\n")
        .map_err(|error| format!("write:{}:{error}", temporary.display()))?;
    file.sync_all()
        .map_err(|error| format!("sync:{}:{error}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|error| format!("rename:{}:{error}", path.display()))
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use nando_response_actor::{
        ProjectStatusMapping, ResponseExecutor, ResponseProgram, ResponseRegistry,
        ValueProjectionFormat,
    };
    use sha2::{Digest, Sha256};

    use super::*;

    static PROJECT_STATUS_LIFECYCLE_TEST_ID: std::sync::atomic::AtomicU64 =
        std::sync::atomic::AtomicU64::new(0);

    fn sha256_text(value: impl AsRef<[u8]>) -> String {
        format!("{:x}", Sha256::digest(value.as_ref()))
    }

    fn v7_project_status_frame(
        index: usize,
        session: usize,
        observed_at_unix_nanos: u64,
    ) -> RelationFrame {
        let value = if index.is_multiple_of(2) { "0" } else { "23" };
        RelationFrame {
            schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: sha256_text(format!("status-frame-{index}")),
            event_id_sha256: sha256_text(format!("status-event-{index}")),
            client_intent_id_sha256: sha256_text(format!("status-intent-{index}")),
            session_id_sha256: sha256_text(format!("status-session-{session}")),
            observed_at_unix_nanos,
            estimated_input_tokens: 64,
            extractor_version: "response-relation-extractor.v7".to_owned(),
            verifier_label: Some(true),
            atoms: vec![
                RelationAtom::ToolKind {
                    value: if session.is_multiple_of(2) {
                        "exec".to_owned()
                    } else {
                        "write_stdin".to_owned()
                    },
                },
                RelationAtom::ObservationCallShape {
                    value: "function_call".to_owned(),
                },
                RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
                RelationAtom::ResponseShape {
                    value: "assistant_message".to_owned(),
                },
                RelationAtom::TypedSlot {
                    slot_id: 1,
                    value_type: AtomValueType::Integer,
                    source: AtomSource::Observation,
                    value_sha256: sha256_text(value),
                },
                RelationAtom::UniqueSlot { slot_id: 1 },
                RelationAtom::ObservationSelector {
                    slot_id: 1,
                    selector: ResponseValueSelector::JsonField {
                        field: "exit_code".to_owned(),
                        value_type: AtomValueType::Integer,
                    },
                },
                RelationAtom::Cardinality {
                    role: "turn_call_count_band".to_owned(),
                    count: 1,
                },
                RelationAtom::Cardinality {
                    role: "turn_output_count_band".to_owned(),
                    count: 1,
                },
                RelationAtom::Cardinality {
                    role: "turn_pending_count_band".to_owned(),
                    count: 0,
                },
                RelationAtom::Cardinality {
                    role: "turn_message_count_band".to_owned(),
                    count: 0,
                },
                RelationAtom::Cardinality {
                    role: "turn_call_shape_count_band".to_owned(),
                    count: 1,
                },
                RelationAtom::ActionStatusProjection {
                    mapping: ProjectStatusMapping::ZeroIsSuccess,
                },
            ],
            evidence_ref_sha256: sha256_text(format!("status-evidence-{index}")),
        }
    }

    fn v7_cross_family_projection_frame(
        index: usize,
        session: usize,
        observed_at_unix_nanos: u64,
    ) -> RelationFrame {
        let mut frame = v7_project_status_frame(index, session, observed_at_unix_nanos);
        let observation_hash = frame.atoms.iter().find_map(|atom| match atom {
            RelationAtom::TypedSlot {
                source: AtomSource::Observation,
                value_sha256,
                ..
            } => Some(value_sha256.clone()),
            _ => None,
        });
        frame
            .atoms
            .retain(|atom| !matches!(atom, RelationAtom::ActionStatusProjection { .. }));
        frame.atoms.extend([
            RelationAtom::TypedSlot {
                slot_id: 2,
                value_type: AtomValueType::Integer,
                source: AtomSource::Action,
                value_sha256: observation_hash.expect("observation hash"),
            },
            RelationAtom::SlotEquality {
                left_slot: 1,
                right_slot: 2,
            },
            RelationAtom::ActionValueProjection {
                format: ValueProjectionFormat::CanonicalJson,
                renderer: nando_response_actor::CollectionOutputRenderer::Direct,
            },
        ]);
        frame.frame_id_sha256 = sha256_text(format!("projection-frame-{index}"));
        frame.event_id_sha256 = sha256_text(format!("projection-event-{index}"));
        frame.client_intent_id_sha256 = sha256_text(format!("projection-intent-{index}"));
        frame.evidence_ref_sha256 = sha256_text(format!("projection-evidence-{index}"));
        frame
    }

    fn write_relation_frames(path: &Path, frames: &[RelationFrame]) {
        let mut bytes = Vec::new();
        for frame in frames {
            serde_json::to_writer(&mut bytes, frame).expect("frame json");
            bytes.push(b'\n');
        }
        fs::write(path, bytes).expect("write relation frames");
    }

    #[test]
    fn zero_future_receipts_are_not_evaluated() {
        assert_eq!(verifier_coverage_state(0, 0), "NOT_EVALUATED");
        assert_eq!(verifier_coverage_state(3, 2), "PARTIAL");
        assert_eq!(verifier_coverage_state(3, 3), "COMPLETE");
    }

    #[test]
    fn relation_frame_replays_are_deduped_and_conflicts_are_counted() {
        let frame = RelationFrame {
            schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: "a".repeat(64),
            event_id_sha256: "b".repeat(64),
            client_intent_id_sha256: "c".repeat(64),
            session_id_sha256: "d".repeat(64),
            observed_at_unix_nanos: 1,
            estimated_input_tokens: 0,
            extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            atoms: vec![RelationAtom::CompletionState {
                value: "pending".to_owned(),
            }],
            evidence_ref_sha256: "e".repeat(64),
        };
        let mut conflicting = frame.clone();
        conflicting.verifier_label = Some(false);
        let (unique, duplicate_rows, conflicting_ids) =
            dedupe_relation_frames(vec![frame.clone(), frame, conflicting]);
        assert_eq!(unique.len(), 1);
        assert_eq!(duplicate_rows, 2);
        assert_eq!(conflicting_ids, 1);
    }

    #[test]
    fn grounded_family_scoreboard_reports_positive_and_total_token_opportunity() {
        let frame = |id: char, event: char, label: bool, tokens: u64| RelationFrame {
            schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: id.to_string().repeat(64),
            event_id_sha256: event.to_string().repeat(64),
            client_intent_id_sha256: "c".repeat(64),
            session_id_sha256: "d".repeat(64),
            observed_at_unix_nanos: 1,
            estimated_input_tokens: tokens,
            extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(label),
            atoms: vec![RelationAtom::ActionValueProjection {
                format: nando_response_actor::ValueProjectionFormat::PlainText,
                renderer: nando_response_actor::CollectionOutputRenderer::Direct,
            }],
            evidence_ref_sha256: "e".repeat(64),
        };
        let report = grounded_family_report(
            7,
            &[
                frame('1', 'a', true, 400),
                frame('2', 'a', true, 390),
                frame('3', 'b', false, 90),
                frame('4', 'b', false, 80),
            ],
        );
        assert_eq!(report["positive_estimated_input_tokens"], 400);
        assert_eq!(report["total_estimated_input_tokens"], 490);
        assert_eq!(
            report["action_symbols"][0],
            "value_projection:PlainText:direct"
        );
    }

    #[test]
    fn generic_operation_is_reported_separately_from_wait_templates() {
        let generic = ResponseOperation::FunctionCallFromRoles {
            function_name: "resume_job".to_owned(),
            selector: nando_response_actor::ResponseValueSelector::UniqueScalar {
                value_type: nando_response_actor::AtomValueType::Identifier,
            },
            arguments: Vec::new(),
        };
        assert_eq!(program_operation_name(&generic), "function_call_from_roles");
        assert_eq!(
            program_operation_name(&ResponseOperation::ProjectSelectedValue {
                selector: nando_response_actor::ResponseValueSelector::UniqueScalar {
                    value_type: nando_response_actor::AtomValueType::Integer,
                },
                format: nando_response_actor::ValueProjectionFormat::CanonicalJson,
                renderer: nando_response_actor::CollectionOutputRenderer::Direct,
                completion_state: "completed".to_owned(),
            }),
            "project_selected_value"
        );
        assert_eq!(
            program_operation_name(&ResponseOperation::WaitOnAnyYieldedCell {
                function_name: "wait".to_owned(),
                yield_time_ms: 1_000,
                max_tokens: 5_000,
            }),
            "wait_on_any_yielded_cell"
        );
        let status = ResponseProgram::project_status(
            ResponseValueSelector::JsonField {
                field: "exit_code".to_owned(),
                value_type: AtomValueType::Integer,
            },
            nando_response_actor::ProjectStatusMapping::ZeroIsSuccess,
            "completed",
        );
        assert_eq!(program_operation_name(&status.operation), "project_status");
        assert_eq!(
            response_program_external_verifier_schema(&status),
            Some("status_projection_external_evidence.v1")
        );
    }

    #[test]
    fn project_status_v7_frames_complete_automatic_support_future_and_causal_lifecycle() {
        let root = env::temp_dir().join(format!(
            "nando-response-miner-project-status-{}-{}",
            std::process::id(),
            PROJECT_STATUS_LIFECYCLE_TEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("test root");
        let args = [
            "relations.jsonl",
            "shadows.jsonl",
            "causal.json",
            "registry.json",
            "status.json",
            "frames.jsonl",
            "manifests.json",
            "receipts.json",
            "grounded-causal.json",
        ]
        .map(|name| root.join(name));
        atomic_write_value(
            &args[2],
            &serde_json::json!({
                "schema":"nando.response-wave-causal-proof.v1",
                "verdict":"PASS",
                "heldout_correct":32,
                "heldout_total":32,
                "full_phase_exact_checks":32,
                "no_phase_exact_checks":64,
                "shuffled_phase_exact_checks":64,
            }),
        )
        .expect("causal proof");

        let support = (0..32)
            .map(|index| v7_project_status_frame(index, index % 4, index as u64 + 1))
            .collect::<Vec<_>>();
        write_relation_frames(&args[5], &support);
        run_with_args(&args).expect("support cycle");

        let manifests: ResponseSupportManifestSet = read_json(&args[6]).expect("support manifests");
        assert_eq!(manifests.manifests.len(), 1);
        assert_eq!(manifests.manifests[0].support_frame_ids.len(), 32);
        let freeze_time = manifests.manifests[0].created_at_unix_nanos;

        let mut support_and_future = support;
        support_and_future.extend((32..64).map(|index| {
            v7_project_status_frame(
                index,
                100 + index % 4,
                freeze_time.saturating_add(index as u64 + 1),
            )
        }));
        write_relation_frames(&args[5], &support_and_future);
        run_with_args(&args).expect("future cycle");

        let registry: ResponseRegistry = read_json(&args[3]).expect("runtime registry");
        assert_eq!(registry.packages.len(), 1);
        let package = &registry.packages[0];
        assert!(matches!(
            package.program.operation,
            ResponseOperation::ProjectStatus { .. }
        ));
        assert_eq!(package.state, ResponsePackageState::Active);
        assert_eq!(package.proof.support_rows, 32);
        assert_eq!(package.proof.future_rows, 32);
        assert_eq!(package.proof.distinct_sessions, 4);
        assert_eq!(package.proof.distinct_surfaces, 2);
        assert_eq!(package.proof.wrong_accepts, 0);
        assert!(package.proof.wave_causal_pass);
        assert!(package.eligible_for_admission_candidate());
        assert!(!package.eligible_for_local_accept());

        let executor = ResponseExecutor::load(&args[3]).expect("miner-built candidate registry");
        assert_eq!(executor.active_package_count(), 0);
        assert_eq!(executor.diagnostic_package_count(), 1);
        let blocked = executor.execute(
            "",
            &serde_json::json!({
                "input":[{"type":"function_call_output","output":"{\"exit_code\":0}"}]
            }),
        );
        assert_eq!(blocked.status, ResponseExecutionStatus::Abstain);
        assert_eq!(blocked.reason, "execution_authority_missing");

        let execution = executor.execute_shadow(
            "",
            &serde_json::json!({
                "input":[
                    {"type":"message","role":"user","content":"status"},
                    {
                        "type":"function_call",
                        "name":"exec",
                        "call_id":"status-call",
                        "arguments":"{}"
                    },
                    {
                        "type":"function_call_output",
                        "call_id":"status-call",
                        "output":"{\"exit_code\":0}"
                    }
                ]
            }),
        );
        assert_eq!(
            execution.status,
            ResponseExecutionStatus::Executed,
            "{}",
            execution.reason
        );
        assert_eq!(execution.response.as_deref(), Some("success"));

        let status: Value = read_json(&args[4]).expect("miner status");
        assert_eq!(status["grounded_promotion_ready"], Value::Bool(true));
        assert_eq!(status["grounded_causal_verdict"], "PASS");
        assert_eq!(status["future_eligibility"]["verifier_accepted_rows"], 32);
        let receipts: Value = read_json(&args[7]).expect("verifier receipts");
        let packages = receipts["packages"].as_array().expect("receipt packages");
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0]["package_id"], package.package_id);
        let receipts = packages[0]["receipts"].as_array().expect("receipt rows");
        assert_eq!(receipts.len(), 32);
        assert!(receipts.iter().all(|receipt| {
            receipt["accepted"] == Value::Bool(true)
                && receipt["schema"] == RESPONSE_FUTURE_VERIFIER_RECEIPT_SCHEMA_V2
                && receipt["actor_program_sha256"]
                    .as_str()
                    .is_some_and(nando_response_actor::valid_nonzero_sha256)
                && receipt["independent_verifier_program_sha256"]
                    .as_str()
                    .is_some_and(nando_response_actor::valid_nonzero_sha256)
                && receipt["evidence_sha256"]
                    .as_str()
                    .is_some_and(nando_response_actor::valid_nonzero_sha256)
                && receipt["output_sha256"]
                    .as_str()
                    .is_some_and(nando_response_actor::valid_nonzero_sha256)
        }));

        let cross_family =
            v7_cross_family_projection_frame(64, 200, freeze_time.saturating_add(1_000));
        assert_eq!(
            relation_frame_routing_atom_ids(&support_and_future[0]),
            relation_frame_routing_atom_ids(&cross_family),
            "post-action family labels must not enter runtime routing atoms"
        );
        support_and_future.push(cross_family);
        write_relation_frames(&args[5], &support_and_future);
        run_with_args(&args).expect("cross-family negative cycle");
        let rejected_registry: ResponseRegistry = read_json(&args[3]).expect("rejected registry");
        assert_eq!(
            rejected_registry.packages[0].state,
            ResponsePackageState::Quarantine
        );
        assert_eq!(rejected_registry.packages[0].proof.future_rows, 32);
        assert_eq!(rejected_registry.packages[0].proof.wrong_accepts, 1);
        assert_eq!(
            ResponseExecutor::load(&args[3])
                .expect("valid quarantined registry")
                .active_package_count(),
            0
        );

        fs::remove_dir_all(root).expect("cleanup test root");
    }

    #[test]
    fn identical_pre_action_atoms_are_reported_as_unseparable() {
        let frame = RelationFrame {
            schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: "a".repeat(64),
            event_id_sha256: "b".repeat(64),
            client_intent_id_sha256: "c".repeat(64),
            session_id_sha256: "d".repeat(64),
            observed_at_unix_nanos: 1,
            estimated_input_tokens: 0,
            extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(false),
            atoms: vec![
                RelationAtom::CompletionState {
                    value: "pending".to_owned(),
                },
                RelationAtom::ResponseShape {
                    value: "function_call".to_owned(),
                },
            ],
            evidence_ref_sha256: "e".repeat(64),
        };
        let package = ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id: "test".to_owned(),
            origin: nando_response_actor::ResponsePackageOrigin::GroundedSynthesis,
            state: ResponsePackageState::Quarantine,
            program: nando_response_actor::ResponseProgram::wait_on_any_yielded_cell(),
            verifier: None,
            routing_predicates: Vec::new(),
            required_routing_atom_ids: Vec::new(),
            phase_centers: relation_frame_routing_atom_ids(&frame),
            anti_centers: Vec::new(),
            wave_margin_micro: 850_000,
            learned_wave_route: None,
            crystallized_operator: None,
            proof: nando_response_actor::ResponsePackageProof {
                support_rows: 0,
                future_rows: 0,
                distinct_sessions: 0,
                distinct_surfaces: 0,
                wrong_accepts: 0,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass: false,
                verifier_schema: String::new(),
            },
        };
        assert!(relation_frame_routes_to_package(&package, &frame));
        let mut legacy = package.clone();
        legacy.package_id = "legacy-template".to_owned();
        legacy.origin = ResponsePackageOrigin::LegacyTemplate;
        let runtime_registry = compile_runtime_registry(9, vec![legacy, package]);
        assert_eq!(runtime_registry.revision, 9);
        assert_eq!(runtime_registry.packages.len(), 1);
        assert_eq!(
            runtime_registry.packages[0].origin,
            ResponsePackageOrigin::GroundedSynthesis
        );
    }

    #[test]
    fn runtime_registry_preserves_distinct_phase_profiles_for_the_same_actor() {
        let base = ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id: "phase-a".to_owned(),
            origin: ResponsePackageOrigin::GroundedSynthesis,
            state: ResponsePackageState::Active,
            program: ResponseProgram::wait_on_yielded_cell(),
            verifier: None,
            routing_predicates: Vec::new(),
            required_routing_atom_ids: vec![1],
            phase_centers: vec![1, 2],
            anti_centers: Vec::new(),
            wave_margin_micro: 1,
            learned_wave_route: None,
            crystallized_operator: None,
            proof: ResponsePackageProof {
                support_rows: 32,
                future_rows: 32,
                distinct_sessions: 3,
                distinct_surfaces: 2,
                wrong_accepts: 0,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass: true,
                verifier_schema: String::new(),
            },
        };
        let mut distinct = base.clone();
        distinct.package_id = "phase-b".to_owned();
        distinct.phase_centers.push(3);
        let registry = compile_runtime_registry(1, vec![base, distinct]);
        assert_eq!(
            registry
                .packages
                .iter()
                .filter(|package| package.state == ResponsePackageState::Active)
                .count(),
            2
        );
    }

    #[test]
    fn causal_aggregate_cannot_pass_when_a_later_package_is_watch() {
        let report =
            |package_id: &str, verdict: &str| nando_response_actor::GroundedWaveCausalReport {
                schema: nando_response_actor::RESPONSE_EXACT_CAUSAL_PROOF_SCHEMA_V2.to_owned(),
                package_id: package_id.to_owned(),
                verdict: verdict.to_owned(),
                support_rows: 32,
                future_rows: 32,
                negative_rows: 1,
                full_phase_correct: 32,
                no_phase_correct: 32,
                shuffled_phase_correct: 0,
                random_center_correct: 0,
                magnitude_only_correct: 0,
                no_anti_center_correct: 32,
                negative_accepts: 0,
                no_phase_negative_accepts: 1,
                shuffled_negative_accepts: 0,
                random_center_negative_accepts: 0,
                magnitude_only_negative_accepts: 1,
                no_anti_center_negative_accepts: 0,
                full_margin_mean_micro: 1,
                shuffled_margin_mean_micro: 0,
                random_margin_mean_micro: 0,
                no_phase_exact_checks: 64,
                full_phase_exact_checks: 32,
            };
        let reports = BTreeMap::from([
            ("first".to_owned(), report("first", "PASS")),
            ("second".to_owned(), report("second", "WATCH")),
        ]);
        assert_eq!(
            aggregate_causal_verdict(["first", "second"], &reports),
            "WATCH"
        );
        assert_eq!(aggregate_causal_verdict(["first"], &reports), "PASS");
        assert_eq!(
            aggregate_causal_verdict(Vec::<String>::new(), &reports),
            "MISSING"
        );
    }

    #[test]
    fn package_evidence_is_partitioned_by_operator_family() {
        let frame =
            |completion: &str, value_type: AtomValueType, function: &str, frame_id: char| {
                RelationFrame {
                    schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
                    frame_id_sha256: frame_id.to_string().repeat(64),
                    event_id_sha256: "b".repeat(64),
                    client_intent_id_sha256: "c".repeat(64),
                    session_id_sha256: "d".repeat(64),
                    observed_at_unix_nanos: 1,
                    estimated_input_tokens: 0,
                    extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION
                        .to_owned(),
                    verifier_label: Some(true),
                    atoms: vec![
                        RelationAtom::ToolKind {
                            value: "source".to_owned(),
                        },
                        RelationAtom::CompletionState {
                            value: completion.to_owned(),
                        },
                        RelationAtom::ResponseShape {
                            value: "function_call".to_owned(),
                        },
                        RelationAtom::TypedSlot {
                            slot_id: 1,
                            value_type,
                            source: AtomSource::Observation,
                            value_sha256: "1".repeat(64),
                        },
                        RelationAtom::TypedSlot {
                            slot_id: 2,
                            value_type,
                            source: AtomSource::Action,
                            value_sha256: "1".repeat(64),
                        },
                        RelationAtom::UniqueSlot { slot_id: 1 },
                        RelationAtom::SlotEquality {
                            left_slot: 1,
                            right_slot: 2,
                        },
                        RelationAtom::ActionFunction {
                            value: function.to_owned(),
                        },
                        RelationAtom::ActionRoleArgument {
                            name: "value".to_owned(),
                            slot_id: 2,
                            value_type: Some(value_type),
                        },
                    ],
                    evidence_ref_sha256: "e".repeat(64),
                }
            };
        let source = frame("completed", AtomValueType::String, "route_result", '1');
        let wait = frame("pending", AtomValueType::Identifier, "wait", '2');
        let package = |program: nando_response_actor::ResponseProgram, support: &RelationFrame| {
            let required_routing_atom_ids =
                nando_response_actor::response_program_required_routing_atom_ids(&program);
            ResponsePackage {
                schema: "nando.response-package.v1".to_owned(),
                package_id: format!("package-{}", support.frame_id_sha256),
                origin: ResponsePackageOrigin::GroundedSynthesis,
                state: ResponsePackageState::Quarantine,
                program,
                verifier: None,
                routing_predicates: Vec::new(),
                required_routing_atom_ids,
                phase_centers: relation_frame_routing_atom_ids(support),
                anti_centers: Vec::new(),
                wave_margin_micro: 850_000,
                learned_wave_route: None,
                crystallized_operator: None,
                proof: nando_response_actor::ResponsePackageProof {
                    support_rows: 1,
                    future_rows: 0,
                    distinct_sessions: 1,
                    distinct_surfaces: 1,
                    wrong_accepts: 0,
                    runtime_parity_failures: 0,
                    exact_cache_overlap: 0,
                    wave_causal_pass: false,
                    verifier_schema: String::new(),
                },
            }
        };
        let source_package = package(
            nando_response_actor::ResponseProgram::function_call_from_roles(
                "route_result",
                nando_response_actor::ResponseValueSelector::UniqueScalar {
                    value_type: nando_response_actor::AtomValueType::String,
                },
                vec![nando_response_actor::ResponseArgument::Role {
                    name: "value".to_owned(),
                    role: nando_response_actor::SemanticRole::SourceValue,
                    value_type: Some(nando_response_actor::AtomValueType::String),
                }],
            ),
            &source,
        );
        let wait_package = package(
            nando_response_actor::ResponseProgram::function_call_from_roles(
                "wait",
                nando_response_actor::ResponseValueSelector::ContentLinePrefix {
                    prefix: "Script running with cell ID ".to_owned(),
                    value_type: nando_response_actor::AtomValueType::Identifier,
                },
                vec![nando_response_actor::ResponseArgument::Role {
                    name: "value".to_owned(),
                    role: nando_response_actor::SemanticRole::ContinuationHandle,
                    value_type: Some(nando_response_actor::AtomValueType::Identifier),
                }],
            ),
            &wait,
        );
        let frames = [source.clone(), wait.clone()];
        let source_negatives = package_negative_frame_refs(&source_package, &[source], &frames);
        let wait_negatives = package_negative_frame_refs(&wait_package, &[wait], &frames);
        assert_eq!(source_negatives.len(), 1);
        assert_eq!(source_negatives[0].frame_id_sha256, "2".repeat(64));
        assert_eq!(wait_negatives.len(), 1);
        assert_eq!(wait_negatives[0].frame_id_sha256, "1".repeat(64));
    }

    #[test]
    fn custom_tool_support_is_not_classified_as_its_own_negative() {
        let frame = RelationFrame {
            schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: "a".repeat(64),
            event_id_sha256: "b".repeat(64),
            client_intent_id_sha256: "c".repeat(64),
            session_id_sha256: "d".repeat(64),
            observed_at_unix_nanos: 1,
            estimated_input_tokens: 0,
            extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            atoms: vec![
                RelationAtom::ToolKind {
                    value: "exec".to_owned(),
                },
                RelationAtom::CompletionState {
                    value: "pending".to_owned(),
                },
                RelationAtom::ResponseShape {
                    value: "custom_tool_call".to_owned(),
                },
                RelationAtom::TypedSlot {
                    slot_id: 1,
                    value_type: AtomValueType::Integer,
                    source: AtomSource::Observation,
                    value_sha256: "1".repeat(64),
                },
                RelationAtom::TypedSlot {
                    slot_id: 2,
                    value_type: AtomValueType::Integer,
                    source: AtomSource::Action,
                    value_sha256: "1".repeat(64),
                },
                RelationAtom::UniqueSlot { slot_id: 1 },
                RelationAtom::ObservationSelector {
                    slot_id: 1,
                    selector: nando_response_actor::ResponseValueSelector::JsonField {
                        field: "session_id".to_owned(),
                        value_type: AtomValueType::Integer,
                    },
                },
                RelationAtom::SlotEquality {
                    left_slot: 1,
                    right_slot: 2,
                },
                RelationAtom::ActionCustomTool {
                    value: "exec".to_owned(),
                },
                RelationAtom::ActionInnerTool {
                    value: "write_stdin".to_owned(),
                },
                RelationAtom::ActionRoleArgument {
                    name: "session_id".to_owned(),
                    slot_id: 2,
                    value_type: Some(AtomValueType::Integer),
                },
                RelationAtom::ActionJsonResultProjection,
            ],
            evidence_ref_sha256: "e".repeat(64),
        };
        let program = nando_response_actor::ResponseProgram::custom_tool_call_from_roles(
            "exec",
            "write_stdin",
            nando_response_actor::ResponseValueSelector::JsonField {
                field: "session_id".to_owned(),
                value_type: AtomValueType::Integer,
            },
            vec![nando_response_actor::ResponseArgument::Role {
                name: "session_id".to_owned(),
                role: nando_response_actor::SemanticRole::ContinuationHandle,
                value_type: Some(AtomValueType::Integer),
            }],
            nando_response_actor::CustomToolResultProjection::JsonStringifyResult,
        );
        let required_routing_atom_ids =
            nando_response_actor::response_program_required_routing_atom_ids(&program);
        let package = ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id: "custom".to_owned(),
            origin: ResponsePackageOrigin::GroundedSynthesis,
            state: ResponsePackageState::Quarantine,
            program,
            verifier: None,
            routing_predicates: Vec::new(),
            required_routing_atom_ids,
            phase_centers: relation_frame_routing_atom_ids(&frame),
            anti_centers: Vec::new(),
            wave_margin_micro: 850_000,
            learned_wave_route: None,
            crystallized_operator: None,
            proof: nando_response_actor::ResponsePackageProof {
                support_rows: 1,
                future_rows: 0,
                distinct_sessions: 1,
                distinct_surfaces: 1,
                wrong_accepts: 0,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass: false,
                verifier_schema: String::new(),
            },
        };
        assert!(
            package_negative_frame_refs(
                &package,
                std::slice::from_ref(&frame),
                std::slice::from_ref(&frame),
            )
            .is_empty()
        );
    }

    #[test]
    fn evidence_refresh_requires_more_independent_sessions_without_inheriting_authority() {
        let manifest = |package_id: &str,
                        generation: u64,
                        supersedes_package_id: Option<&str>,
                        support_prefix: &str,
                        support_rows: usize,
                        reserved_sessions: usize| {
            ResponseSupportManifest {
                schema: "nando.response-support-manifest.v1".to_owned(),
                package_id: package_id.to_owned(),
                lineage_id: "lineage".to_owned(),
                generation,
                routing_refinement_version: ROUTING_REFINEMENT_VERSION,
                supersedes_package_id: supersedes_package_id.map(str::to_owned),
                created_at_unix_nanos: generation,
                support_boundary_unix_nanos: generation,
                support_frame_ids: (0..support_rows)
                    .map(|index| format!("{support_prefix}-{index}"))
                    .collect(),
                support_session_ids: vec![format!("support-{generation}")],
                support_intent_ids: vec![format!("intent-{generation}")],
                reserved_future_session_ids: (0..reserved_sessions)
                    .map(|index| format!("reserved-{index}"))
                    .collect(),
                learned_center_atom_ids: vec![1],
                learned_anti_center_atom_ids: Vec::new(),
                selected_routing_atom_ids: Vec::new(),
                selected_routing_predicates: Vec::new(),
                split_negative_frame_ids: Vec::new(),
                holdout_negative_frame_ids: Vec::new(),
                split_parent_support_rows: support_rows,
                manifest_sha256: format!("manifest-{generation}"),
            }
        };
        let current = manifest("g1", 1, None, "old", 32, 0);
        let improved = manifest("g2", 2, Some("g1"), "new", 32, 3);
        assert!(evidence_refresh_improves(&current, &improved));

        let same_support = manifest("g2", 2, Some("g1"), "old", 32, 3);
        assert!(!evidence_refresh_improves(&current, &same_support));
        let mut legacy_policy = current.clone();
        legacy_policy.routing_refinement_version = 0;
        let policy_migration = manifest("g2", 2, Some("g1"), "old", 32, 0);
        assert!(evidence_refresh_improves(&legacy_policy, &policy_migration));
        let undersized = manifest("g2", 2, Some("g1"), "new", 31, 3);
        assert!(!evidence_refresh_improves(&current, &undersized));
        let authority_mismatch = manifest("g2", 2, Some("other"), "new", 32, 3);
        assert!(!evidence_refresh_improves(&current, &authority_mismatch));
    }

    #[test]
    fn rollover_requires_a_new_route_contract_or_material_support_gain() {
        let manifest =
            |package_id: &str, generation: u64, support_rows: usize| ResponseSupportManifest {
                schema: "nando.response-support-manifest.v1".to_owned(),
                package_id: package_id.to_owned(),
                lineage_id: "lineage".to_owned(),
                generation,
                routing_refinement_version: ROUTING_REFINEMENT_VERSION,
                supersedes_package_id: (generation > 1).then(|| "g1".to_owned()),
                created_at_unix_nanos: generation,
                support_boundary_unix_nanos: generation,
                support_frame_ids: (0..support_rows)
                    .map(|index| format!("f-{index}"))
                    .collect(),
                support_session_ids: vec!["session".to_owned()],
                support_intent_ids: vec!["intent".to_owned()],
                reserved_future_session_ids: Vec::new(),
                learned_center_atom_ids: vec![1, 2],
                learned_anti_center_atom_ids: Vec::new(),
                selected_routing_atom_ids: Vec::new(),
                selected_routing_predicates: Vec::new(),
                split_negative_frame_ids: Vec::new(),
                holdout_negative_frame_ids: Vec::new(),
                split_parent_support_rows: support_rows,
                manifest_sha256: String::new(),
            };
        let current = manifest("g1", 1, 64);
        let repeated = manifest("g2", 2, 64);
        assert!(!rollover_manifest_improves(&current, &repeated));

        let mut center_drift = repeated.clone();
        center_drift.learned_center_atom_ids.push(3);
        assert!(!rollover_manifest_improves(&current, &center_drift));

        let expanded = manifest("g2", 2, 96);
        assert!(rollover_manifest_improves(&current, &expanded));

        let mut refined = repeated;
        refined.selected_routing_atom_ids.push(3);
        assert!(rollover_manifest_improves(&current, &refined));
    }

    #[test]
    fn token_opportunity_dedupes_replayed_events() {
        let frame = |id: char, tokens: u64, label| RelationFrame {
            schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: id.to_string().repeat(64),
            event_id_sha256: "event".repeat(12) + &id.to_string(),
            client_intent_id_sha256: "c".repeat(64),
            session_id_sha256: "d".repeat(64),
            observed_at_unix_nanos: 1,
            estimated_input_tokens: tokens,
            extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: label,
            atoms: Vec::new(),
            evidence_ref_sha256: "e".repeat(64),
        };
        let report = token_opportunity_report(&[
            frame('a', 100, Some(true)),
            frame('b', 90, Some(true)),
            frame('c', 50, Some(false)),
        ]);
        assert_eq!(report["deduplicated_events"], 3);
        assert_eq!(report["deduplicated_input_tokens"], 240);
    }

    #[test]
    fn verified_future_self_training_keeps_a_newer_frozen_exam() {
        let mut future = Vec::new();
        for session in 0_u64..6 {
            for row in 0_u64..11 {
                future.push(RelationFrame {
                    schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
                    frame_id_sha256: sha256_text(format!("frame-{session}-{row}")),
                    event_id_sha256: sha256_text(format!("event-{session}-{row}")),
                    client_intent_id_sha256: sha256_text(format!("intent-{session}-{row}")),
                    session_id_sha256: sha256_text(format!("session-{session}")),
                    observed_at_unix_nanos: session * 1_000 + row,
                    estimated_input_tokens: 10,
                    extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION
                        .to_owned(),
                    verifier_label: Some(true),
                    atoms: Vec::new(),
                    evidence_ref_sha256: sha256_text(format!("evidence-{session}-{row}")),
                });
            }
        }
        let selected = verified_future_sessions_for_self_training(&future);
        assert_eq!(selected.len(), 3);
        for session in 0_u64..3 {
            assert!(selected.contains(&sha256_text(format!("session-{session}"))));
        }
        for session in 3_u64..6 {
            assert!(!selected.contains(&sha256_text(format!("session-{session}"))));
        }
    }

    #[test]
    fn verified_future_self_training_rejects_small_or_unverified_evidence() {
        let frame = |session: u64, row: u64, label| RelationFrame {
            schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: sha256_text(format!("frame-{session}-{row}-{label:?}")),
            event_id_sha256: sha256_text(format!("event-{session}-{row}-{label:?}")),
            client_intent_id_sha256: sha256_text(format!("intent-{session}-{row}-{label:?}")),
            session_id_sha256: sha256_text(format!("session-{session}")),
            observed_at_unix_nanos: session * 1_000 + row,
            estimated_input_tokens: 10,
            extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: label,
            atoms: Vec::new(),
            evidence_ref_sha256: sha256_text(format!("evidence-{session}-{row}-{label:?}")),
        };
        let too_few_sessions = (0_u64..64)
            .map(|row| frame(row % 5, row, Some(true)))
            .collect::<Vec<_>>();
        assert!(verified_future_sessions_for_self_training(&too_few_sessions).is_empty());

        let unverified = (0_u64..66)
            .map(|row| frame(row % 6, row, Some(false)))
            .collect::<Vec<_>>();
        assert!(verified_future_sessions_for_self_training(&unverified).is_empty());
    }

    #[test]
    fn quarantined_registry_package_has_no_execution_authority() {
        let package = ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id: "quarantined".to_owned(),
            origin: ResponsePackageOrigin::GroundedSynthesis,
            state: ResponsePackageState::Quarantine,
            program: ResponseProgram::wait_on_yielded_cell(),
            verifier: None,
            routing_predicates: Vec::new(),
            required_routing_atom_ids: Vec::new(),
            phase_centers: vec![1],
            anti_centers: Vec::new(),
            wave_margin_micro: 1,
            learned_wave_route: None,
            crystallized_operator: None,
            proof: nando_response_actor::ResponsePackageProof {
                support_rows: 32,
                future_rows: 31,
                distinct_sessions: 3,
                distinct_surfaces: 2,
                wrong_accepts: 0,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass: true,
                verifier_schema: String::new(),
            },
        };
        let registry = compile_runtime_registry(4, vec![package]);
        let executor = ResponseExecutor::from_registry(registry).expect("valid v5 registry");
        assert_eq!(executor.active_package_count(), 0);
    }

    #[test]
    fn cold_collection_evidence_builds_generic_quarantine_without_authority() {
        let mut rows = Vec::new();
        for session in 0_u64..7 {
            let (collection, predicate, projected) = if session % 2 == 0 {
                ("rows", "kind", "value")
            } else {
                ("entries", "tag", "amount")
            };
            let count = if session < 4 { 10 } else { 11 };
            for ordinal in 0..count {
                let output = serde_json::json!({
                    (collection): [
                        {(predicate):"keep", (projected):3},
                        {(predicate):"drop", (projected):4},
                        {(predicate):"keep", (projected):5}
                    ]
                });
                rows.push(ColdCollectionRow {
                    frame_id_sha256: sha256_text(format!("frame-{session}-{ordinal}")),
                    session_id_sha256: sha256_text(format!("session-{session}")),
                    client_intent_id_sha256: sha256_text(format!("intent-{session}-{ordinal}")),
                    observed_at_unix_nanos: session * 1_000 + ordinal,
                    surface_sha256: sha256_text(format!("{collection}:{predicate}:{projected}")),
                    phase_valid: true,
                    request_phase_atom_ids: Vec::new(),
                    example: CollectionSynthesisExample {
                        provider_payload: serde_json::json!({
                            "input":[{"type":"function_call_output","output":output.to_string()}]
                        }),
                        expected_response: "[3,5]".to_owned(),
                    },
                });
            }
        }
        let package = compile_collection_quarantine_package(&rows).expect("collection package");
        assert_eq!(package.state, ResponsePackageState::Quarantine);
        assert_eq!(package.proof.support_rows, 40);
        assert_eq!(package.proof.future_rows, 33);
        assert_eq!(package.proof.distinct_sessions, 3);
        assert_eq!(package.proof.wrong_accepts, 0);
        assert!(package.proof.wave_causal_pass);
        assert!(matches!(
            package.program.operation,
            ResponseOperation::ComposeCollection { .. }
        ));
        assert!(!package.eligible_for_admission_candidate());
        let mut mixed = rows.clone();
        mixed.extend(rows.iter().cloned().map(|mut row| {
            row.frame_id_sha256 = sha256_text(format!("drop:{}", row.frame_id_sha256));
            row.client_intent_id_sha256 =
                sha256_text(format!("drop:{}", row.client_intent_id_sha256));
            row.example.expected_response = "[4]".to_owned();
            row
        }));
        let split = collection_families(&mixed);
        assert_eq!(split.len(), 2);
        assert!(split.iter().all(|family| family.len() == rows.len()));
        let manifest = build_collection_support_manifest(&rows, &package).expect("manifest");
        let future_package =
            compile_collection_package(&rows, Some(&manifest)).expect("future package");
        assert_eq!(future_package.state, ResponsePackageState::Active);
        assert!(future_package.eligible_for_admission_candidate());
        let relation_rows = rows
            .iter()
            .map(|row| RelationFrame {
                schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
                frame_id_sha256: row.frame_id_sha256.clone(),
                event_id_sha256: sha256_text(format!("event:{}", row.frame_id_sha256)),
                client_intent_id_sha256: row.client_intent_id_sha256.clone(),
                session_id_sha256: row.session_id_sha256.clone(),
                observed_at_unix_nanos: row.observed_at_unix_nanos,
                estimated_input_tokens: 10,
                extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION
                    .to_owned(),
                verifier_label: Some(true),
                atoms: vec![
                    RelationAtom::CollectionShape {
                        array_fields: 1,
                        row_fields: 2,
                    },
                    RelationAtom::ResponseShape {
                        value: "assistant_message".to_owned(),
                    },
                    RelationAtom::CompletionState {
                        value: "completed".to_owned(),
                    },
                ],
                evidence_ref_sha256: sha256_text(format!("evidence:{}", row.frame_id_sha256)),
            })
            .collect::<Vec<_>>();
        let support_ids = manifest
            .support_frame_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        let causal_support = relation_rows
            .iter()
            .filter(|frame| support_ids.contains(&frame.frame_id_sha256))
            .cloned()
            .collect::<Vec<_>>();
        let causal_future = relation_rows
            .iter()
            .filter(|frame| frame.observed_at_unix_nanos > manifest.support_boundary_unix_nanos)
            .cloned()
            .collect::<Vec<_>>();
        let causal = nando_response_actor::evaluate_grounded_wave_causality(
            &future_package,
            &causal_support,
            &causal_future,
            &[],
        );
        assert_eq!(causal.verdict, "PASS", "{causal:?}");
        let executor =
            ResponseExecutor::from_registry(compile_runtime_registry(9, vec![future_package]))
                .expect("diagnostic registry");
        assert_eq!(executor.active_package_count(), 0);
    }

    #[test]
    fn miner_cycle_builds_collection_manifest_receipts_and_authority_candidate() {
        let root = env::temp_dir().join(format!(
            "nando-collection-miner-cycle-{}-{}",
            std::process::id(),
            unix_now()
        ));
        fs::create_dir_all(&root).expect("root");
        let args = [
            "relations.jsonl",
            "shadows.jsonl",
            "causal.json",
            "registry.json",
            "status.json",
            "frames.jsonl",
            "manifests.json",
            "receipts.json",
            "grounded-causal.json",
            "parity.json",
        ]
        .map(|name| root.join(name));
        fs::write(&args[0], "").expect("relations");
        fs::write(&args[1], "").expect("shadows");
        atomic_write_value(
            &args[2],
            &serde_json::json!({
                "schema":"nando.response-wave-causal-proof.v1",
                "verdict":"PASS",
                "heldout_correct":32,
                "heldout_total":32,
                "full_phase_exact_checks":32,
                "no_phase_exact_checks":64,
                "shuffled_phase_exact_checks":64,
            }),
        )
        .expect("global causal");
        let mut lines = String::new();
        for session in 0_u64..7 {
            let (collection, predicate, projected) = if session % 2 == 0 {
                ("rows", "kind", "value")
            } else {
                ("entries", "tag", "amount")
            };
            let count = if session < 4 { 10 } else { 11 };
            for ordinal in 0..count {
                let first_value = 3 + session;
                let middle_value = 4 + session;
                let last_value = 5 + session;
                let output = serde_json::json!({
                    (collection): [
                        {(predicate):"keep", (projected):first_value},
                        {(predicate):"drop", (projected):middle_value},
                        {(predicate):"keep", (projected):last_value}
                    ]
                });
                let project_expected = format!("[{first_value},{last_value}]");
                for (family, expected_response, request_atom) in [
                    (
                        "project",
                        project_expected.as_str(),
                        13_665_181_768_394_347_299_u64,
                    ),
                    ("count", "2", 15_291_052_347_829_727_369_u64),
                ] {
                    let provider_payload = serde_json::json!({
                        "input":[{"type":"function_call_output","output":output.to_string()}]
                    });
                    let cold = ColdCollectionEvidence {
                        schema: "nando.response-collection-synthesis-example.v1".to_owned(),
                        provider_payload,
                        expected_response: expected_response.to_owned(),
                    };
                    let frame_id = sha256_text(format!("cycle-{family}-frame-{session}-{ordinal}"));
                    let row = serde_json::json!({
                        "schema": nando_response_actor::RELATION_FRAME_SCHEMA,
                        "frame_id_sha256": frame_id,
                        "event_id_sha256": sha256_text(format!("cycle-{family}-event-{session}-{ordinal}")),
                        "client_intent_id_sha256": sha256_text(format!("cycle-{family}-intent-{session}-{ordinal}")),
                        "session_id_sha256": sha256_text(format!("cycle-{family}-session-{session}")),
                        "observed_at_unix_nanos": session * 10_000 + ordinal as u64 * 2 + u64::from(family == "count") + 1,
                        "estimated_input_tokens": 100,
                        "extractor_version": nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION,
                        "verifier_label": true,
                        "atoms": [
                            {"kind":"collection_shape","array_fields":1,"row_fields":2},
                            {"kind":"request_phase_atom","atom_id":request_atom},
                            {"kind":"observation_call_shape","value":"function_call"},
                            {"kind":"response_shape","value":"assistant_message"},
                            {"kind":"completion_state","value":"completed"}
                        ],
                        "evidence_ref_sha256": canonical_json_sha256(&cold).expect("cold digest"),
                        "cold_collection_example": cold,
                    });
                    lines.push_str(&serde_json::to_string(&row).expect("row"));
                    lines.push('\n');
                }
            }
        }
        fs::write(&args[5], lines).expect("frames");
        run_with_args(&args).expect("miner cycle");
        let registry: ResponseRegistry = read_json(&args[3]).expect("registry");
        assert_eq!(registry.schema, RESPONSE_REGISTRY_SCHEMA_V6);
        assert_eq!(registry.packages.len(), 2);
        assert!(
            registry
                .packages
                .iter()
                .all(|package| package.state == ResponsePackageState::Active)
        );
        assert!(
            registry
                .packages
                .iter()
                .all(ResponsePackage::eligible_for_admission_candidate)
        );
        let manifests: ResponseSupportManifestSet = read_json(&args[6]).expect("manifests");
        assert_eq!(manifests.manifests.len(), 2);
        assert!(
            manifests
                .manifests
                .iter()
                .all(|manifest| manifest.support_frame_ids.len() == 40)
        );
        let status: Value = read_json(&args[4]).expect("status");
        assert_eq!(status["collection_synthesis"]["future_rows"], 66);
        assert_eq!(status["collection_synthesis"]["future_wrong_accepts"], 0);
        let bindings: Vec<ResponsePackageAuthorityBindingV2> =
            serde_json::from_value(status["response_authority_candidate"]["packages"].clone())
                .expect("authority bindings");
        assert_eq!(bindings.len(), 2);
        let now = unix_now();
        let gate_build = sha256_text("gate-build");
        let runtime_build = sha256_text("runtime-build");
        let admission = nando_response_actor::CompositeResponseAdmissionV2 {
            schema: nando_response_actor::COMPOSITE_ADMISSION_SCHEMA_V2.to_owned(),
            project_id: "nando-wave".to_owned(),
            generated_at_unix: now,
            expires_at_unix: now + 30,
            verdict: "PASS".to_owned(),
            eligible_for_local_accept: true,
            response_authority: nando_response_actor::ResponseAuthorityV2 {
                schema: RESPONSE_AUTHORITY_SCHEMA_V2.to_owned(),
                registry_schema: registry.schema.clone(),
                registry_revision: registry.revision,
                registry_sha256: response_registry_digest(&registry).expect("registry digest"),
                gate_build_sha256: gate_build.clone(),
                runtime_build_sha256: runtime_build.clone(),
                packages: bindings,
            },
        };
        let executor = ResponseExecutor::from_registry_with_admission(
            registry,
            admission,
            "nando-wave",
            &gate_build,
            &runtime_build,
            now,
            30,
        )
        .expect("authorized executor");
        let heldout = serde_json::json!({
            "input":[{"type":"function_call_output","output":serde_json::json!({
                "records":[
                    {"marker":"keep","score":3},
                    {"marker":"drop","score":4},
                    {"marker":"keep","score":5}
                ]
            }).to_string()}]
        });
        let execution = executor.execute("project", &heldout);
        assert_eq!(
            execution.status,
            ResponseExecutionStatus::Executed,
            "{}",
            execution.reason
        );
        assert_eq!(execution.response.as_deref(), Some("[3,5]"));
        let count_execution = executor.execute("count", &heldout);
        assert_eq!(
            count_execution.status,
            ResponseExecutionStatus::Executed,
            "{}",
            count_execution.reason
        );
        assert_eq!(count_execution.response.as_deref(), Some("2"));
        let projected = serde_json::json!({"output_text":"[3,5]"});
        let runtime_receipt = executor
            .finalize_runtime_receipt(
                &execution,
                &sha256_text("heldout-request"),
                "test-projector.v1",
                &sha256_text("test-projector"),
                &projected,
            )
            .expect("runtime receipt");
        assert_eq!(
            runtime_receipt.receipt.schema,
            nando_response_actor::RESPONSE_RUNTIME_RECEIPT_SCHEMA_V2
        );
        let negative_output = serde_json::json!({
            "records":[
                {"marker":"keep","score":3},
                {"marker":"drop","score":4},
                {"marker":"keep","score":5}
            ]
        });
        let negative_cold = ColdCollectionEvidence {
            schema: "nando.response-collection-synthesis-example.v1".to_owned(),
            provider_payload: serde_json::json!({
                "input":[{"type":"function_call_output","output":negative_output.to_string()}]
            }),
            expected_response: "[999]".to_owned(),
        };
        let negative_row = serde_json::json!({
            "schema": nando_response_actor::RELATION_FRAME_SCHEMA,
            "frame_id_sha256": sha256_text("drift-frame"),
            "event_id_sha256": sha256_text("drift-event"),
            "client_intent_id_sha256": sha256_text("drift-intent"),
            "session_id_sha256": sha256_text("drift-session"),
            "observed_at_unix_nanos": 100_000_u64,
            "estimated_input_tokens": 100,
            "extractor_version": nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION,
            "verifier_label": true,
            "atoms": [
                {"kind":"collection_shape","array_fields":1,"row_fields":2},
                {"kind":"request_phase_atom","atom_id":13_665_181_768_394_347_299_u64},
                {"kind":"observation_call_shape","value":"function_call"},
                {"kind":"response_shape","value":"assistant_message"},
                {"kind":"completion_state","value":"completed"}
            ],
            "evidence_ref_sha256": canonical_json_sha256(&negative_cold).expect("negative digest"),
            "cold_collection_example": negative_cold,
        });
        let mut frame_file = fs::OpenOptions::new()
            .append(true)
            .open(&args[5])
            .expect("frames append");
        writeln!(
            frame_file,
            "{}",
            serde_json::to_string(&negative_row).expect("negative row")
        )
        .expect("negative append");
        run_with_args(&args).expect("drift miner cycle");
        let drift_registry: ResponseRegistry = read_json(&args[3]).expect("drift registry");
        assert_eq!(
            drift_registry
                .packages
                .iter()
                .filter(|package| package.state == ResponsePackageState::Active)
                .count(),
            1
        );
        let demoted = drift_registry
            .packages
            .iter()
            .find(|package| package.proof.wrong_accepts > 0)
            .expect("demoted package");
        assert_eq!(demoted.state, ResponsePackageState::Quarantine);
        let drift_status: Value = read_json(&args[4]).expect("drift status");
        assert_eq!(
            drift_status["collection_synthesis"]["future_wrong_accepts"],
            1
        );
        assert_eq!(
            drift_status["response_authority_candidate"]["packages"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
