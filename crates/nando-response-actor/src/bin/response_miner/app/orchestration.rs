//! One bounded miner cycle. Domain helpers live in sibling owner modules.

use super::*;

pub(super) fn run_with_args(args: &[PathBuf]) -> Result<(), String> {
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
    let current_support_manifests = latest_grounded_support_manifests(&support_manifests.manifests);
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
        package.state = ResponsePackageState::Active;
        let promotion_ready = package.admission_candidate_blocker().is_none()
            && package_missing == 0
            && hard_negative_accepts == 0
            && routing_indistinguishable == 0
            && relation_frame_conflicting_duplicate_ids == 0
            && wave_causal_pass
            && exact_package_causal_pass
            && package.proof.wave_causal_pass;
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
                && candidate.support_frame_ids.len() >= LEGACY_CONTROL_SUPPORT_ROWS
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
            let mut admission_candidate = package.clone();
            admission_candidate.state = ResponsePackageState::Active;
            if let Some(blocker) = admission_candidate.admission_candidate_blocker() {
                blockers.push(blocker.to_owned());
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
        .min_by_key(promotion_debt);
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
            "authority_owner": "online_collection_miner",
            "route": "version_space -> freeze -> independent_future -> external_admission",
            "legacy_batch_builder_enabled": false,
            "candidate_present": false,
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
