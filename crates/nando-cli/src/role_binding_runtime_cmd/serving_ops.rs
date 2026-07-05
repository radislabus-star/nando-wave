pub(crate) fn run_role_binding_real_traffic_serving_ops_payload_dry_run_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GIT_CONTROL_PROFILE_REGISTRY_CONFIG));
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_PAYLOAD_DRY_RUN_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_PAYLOAD_DRY_RUN_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(1000);

    let registry_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&registry_config_path)?;
    validate_registry_config(&registry_config)?;
    let profile_registered = registry_config
        .profiles
        .iter()
        .any(|profile| profile.profile_id == REAL_TRAFFIC_SERVING_OPS_PROFILE_ID);
    let route_catalog = CodexHistoryRouteCatalog::from_registry(&registry_config)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let skip = history_rows.len().saturating_sub(max_events);
    let mut trace_rows = Vec::with_capacity(history_rows.len().saturating_sub(skip));
    let mut report_rows = Vec::new();
    let mut serving_ops_candidate_events = 0usize;
    let mut payload_ready_events = 0usize;
    let mut payload_built_events = 0usize;
    let mut scoreable_payload_events = 0usize;
    let mut builder_rejected_events = 0usize;
    let mut readiness_rejected_events = 0usize;
    let mut active_fringe_centers_total = 0usize;
    let mut slots_total = 0usize;
    let mut positive_impulses_total = 0usize;
    let mut negative_impulses_total = 0usize;
    let mut builder_status_counts = BTreeMap::<String, usize>::new();

    for (index, row) in history_rows.iter().enumerate().skip(skip) {
        let fingerprint = stable_real_traffic_fingerprint64(row.text.as_bytes());
        let event_id = format!(
            "codex_history_serving_ops_payload_dry_run::{}::{}::{}",
            row.session_id, row.ts, index
        );
        let request_fingerprint = format!("fnv1a64:{fingerprint:016x}");
        let exact_cache_key = Some(format!("codex_history_request:{fingerprint:016x}"));
        let mut nando_shadow_request = None;
        let mut notes = "not serving_ops route-gap candidate".to_owned();

        if route_catalog.classify_request_text(&row.text).is_none()
            && route_gap_family_key(&row.text) == REAL_TRAFFIC_SERVING_OPS_ROUTE_KEY
        {
            serving_ops_candidate_events += 1;
            let readiness =
                analyze_route_gap_payload_readiness(REAL_TRAFFIC_SERVING_OPS_ROUTE_KEY, &row.text);
            if readiness.payload_ready {
                payload_ready_events += 1;
                let built = build_serving_ops_dry_run_request(&event_id, &fingerprint, &row.text);
                match built {
                    Some(request) => {
                        let active_fringe_centers = request.active_fringe.len();
                        let slots = request.slots.len();
                        let positive_impulses = request
                            .slots
                            .iter()
                            .map(|slot| slot.positive_impulses.len())
                            .sum::<usize>();
                        let negative_impulses = request
                            .slots
                            .iter()
                            .map(|slot| slot.negative_impulses.len())
                            .sum::<usize>();
                        let scoreable = active_fringe_centers > 0 && slots > 0;
                        payload_built_events += 1;
                        scoreable_payload_events += usize::from(scoreable);
                        active_fringe_centers_total += active_fringe_centers;
                        slots_total += slots;
                        positive_impulses_total += positive_impulses;
                        negative_impulses_total += negative_impulses;
                        let builder_status = if scoreable && profile_registered {
                            "scoreable_payload_built_profile_registered"
                        } else if scoreable {
                            "scoreable_payload_built_profile_missing"
                        } else {
                            "payload_built_but_not_scoreable"
                        }
                        .to_owned();
                        *builder_status_counts
                            .entry(builder_status.clone())
                            .or_insert(0) += 1;
                        report_rows.push(RoleBindingServingOpsPayloadDryRunRow {
                            event_id: event_id.clone(),
                            request_fingerprint: request_fingerprint.clone(),
                            route_key: REAL_TRAFFIC_SERVING_OPS_ROUTE_KEY.to_owned(),
                            profile_id: REAL_TRAFFIC_SERVING_OPS_PROFILE_ID.to_owned(),
                            readiness_payload_ready: true,
                            payload_built: true,
                            scoreable,
                            profile_registered,
                            builder_status: builder_status.clone(),
                            active_fringe_centers,
                            slots,
                            positive_impulses,
                            negative_impulses,
                        });
                        notes = format!(
                            "request-side serving-ops payload built; status={builder_status}; server mutations and verified accepts disabled"
                        );
                        nando_shadow_request = Some(request);
                    }
                    None => {
                        builder_rejected_events += 1;
                        let builder_status = "builder_rejected_request_side_features".to_owned();
                        *builder_status_counts
                            .entry(builder_status.clone())
                            .or_insert(0) += 1;
                        report_rows.push(RoleBindingServingOpsPayloadDryRunRow {
                            event_id: event_id.clone(),
                            request_fingerprint: request_fingerprint.clone(),
                            route_key: REAL_TRAFFIC_SERVING_OPS_ROUTE_KEY.to_owned(),
                            profile_id: REAL_TRAFFIC_SERVING_OPS_PROFILE_ID.to_owned(),
                            readiness_payload_ready: true,
                            payload_built: false,
                            scoreable: false,
                            profile_registered,
                            builder_status: builder_status.clone(),
                            active_fringe_centers: 0,
                            slots: 0,
                            positive_impulses: 0,
                            negative_impulses: 0,
                        });
                        notes = builder_status;
                    }
                }
            } else {
                readiness_rejected_events += 1;
                let builder_status = "readiness_rejected".to_owned();
                *builder_status_counts
                    .entry(builder_status.clone())
                    .or_insert(0) += 1;
                notes = format!(
                    "serving_ops route-gap candidate rejected by readiness gate: {}",
                    readiness.missing_reasons.join(",")
                );
            }
        }

        trace_rows.push(RoleBindingRealTrafficTraceRow {
            schema_version: "nando_role_binding_real_traffic_trace_v1".to_owned(),
            trace_id: event_id,
            traffic_source: Some("codex_history_local_serving_ops_payload_dry_run".to_owned()),
            time_ms: Some(row.ts.saturating_mul(1000)),
            request_fingerprint: Some(request_fingerprint),
            response_fingerprint: None,
            tool_call_fingerprints: Vec::new(),
            verification_source: Some(
                "request-side serving-ops payload dry-run from local Codex prompt only; raw text, response text, target labels, proof labels, and server mutations are not used"
                    .to_owned(),
            ),
            llm_call: true,
            exact_cache_key,
            provider_cache_hit: None,
            provider_cost_microusd: None,
            token_cost: RoleBindingTraceTokenCostFields::default(),
            nando_shadow_request,
            verified_safe_accept: None,
            synthetic_source: Some(false),
            notes: Some(notes),
        });
    }

    write_real_traffic_trace_jsonl(&trace_path, &trace_rows)?;
    let shadow_score_ready = profile_registered && scoreable_payload_events > 0;
    let report = RoleBindingServingOpsPayloadDryRunReport {
        schema_version: "nando_role_binding_serving_ops_payload_dry_run_v1".to_owned(),
        verdict: if shadow_score_ready {
            "SERVING_OPS_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PROFILE_READY"
        } else if scoreable_payload_events > 0 {
            "SERVING_OPS_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_PROFILE_MISSING"
        } else {
            "SERVING_OPS_PAYLOAD_DRY_RUN_V1_REVIEW_NO_SCOREABLE_PAYLOADS"
        }
        .to_owned(),
        history_path: history_path.display().to_string(),
        registry_config_path: registry_config_path.display().to_string(),
        trace_path: trace_path.display().to_string(),
        max_events,
        total_history_rows: history_rows.len(),
        trace_rows_written: trace_rows.len(),
        serving_ops_candidate_events,
        payload_ready_events,
        payload_built_events,
        scoreable_payload_events,
        builder_rejected_events,
        readiness_rejected_events,
        profile_registered,
        shadow_score_ready,
        active_fringe_centers_total,
        slots_total,
        positive_impulses_total,
        negative_impulses_total,
        builder_status_counts: builder_status_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        raw_text_written: false,
        response_text_used: false,
        target_labels_used: false,
        proof_labels_used: false,
        server_mutation_enabled: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        rows: report_rows,
        claim_boundary: "Request-side dry-run payload builder only. It emits active_fringe/slots for serving_ops route-gap rows from prompt text only, writes no raw prompt text, performs no server/daemon mutation, keeps verified_safe_accept=None and expect_local_operator=false, and cannot prove savings.".to_owned(),
        next_engineering_debt: "Compile a serving_ops .nwrb scoring profile, rerun shadow, then attach service_health_metric_verifier_v1 before any local accept, daemon action, or market savings claim.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-serving-ops-payload-dry-run-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!("  registry_config: {}", registry_config_path.display());
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!(
        "  serving_ops_candidate_events: {}",
        report.serving_ops_candidate_events
    );
    println!("  payload_ready_events: {}", report.payload_ready_events);
    println!("  payload_built_events: {}", report.payload_built_events);
    println!(
        "  scoreable_payload_events: {}",
        report.scoreable_payload_events
    );
    println!("  profile_registered: {}", report.profile_registered);
    println!("  server_mutation_enabled: false");
    println!("  local_accepts_enabled: false");
    Err(
        "serving-ops payload dry-run is review-only; build profile+verifier before claims"
            .to_owned(),
    )
}

pub(crate) fn run_role_binding_real_traffic_serving_ops_profile_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let base_registry_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GIT_CONTROL_PROFILE_REGISTRY_CONFIG));
    let dry_run_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_PAYLOAD_DRY_RUN_TRACE_JSONL));
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_PACKAGE_PATH));
    let registry_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_PROFILE_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_PROFILE_REPORT));

    let mut registry = read_json_file::<RoleBindingProfileRegistryConfig>(&base_registry_path)?;
    validate_registry_config(&registry)?;
    let trace_rows = read_real_traffic_trace_jsonl(&dry_run_trace_path)?;
    let build = build_serving_ops_role_binding_package_from_trace(&trace_rows)?;
    if let Some(parent) = package_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create serving-ops package directory {}: {error}",
                parent.display()
            )
        })?;
    }
    fs::write(&package_path, &build.package_bytes).map_err(|error| {
        format!(
            "failed to write serving-ops package {}: {error}",
            package_path.display()
        )
    })?;
    let package_info =
        WavePredictorRoleBindingOffloadRuntime::inspect_package_bytes(&build.package_bytes)
            .map_err(|error| format!("failed to inspect serving-ops package: {error:?}"))?;
    let policy =
        WavePredictorRoleBindingOffloadPolicy::new(REAL_TRAFFIC_SERVING_OPS_DISABLED_THRESHOLD)
            .map_err(|error| format!("invalid serving-ops disabled policy: {error:?}"))?;
    let sdk = WavePredictorRoleBindingOffloadRuntime::from_package_bytes_serving_packed_only(
        &build.package_bytes,
        policy,
    )
    .map_err(|error| format!("failed to load serving-ops package: {error:?}"))?;

    let requests = serving_ops_scoreable_requests(&trace_rows);
    let mut energy_margins = Vec::with_capacity(requests.len());
    let mut min_slot_margins = Vec::with_capacity(requests.len());
    let mut positive_margin_rows = 0usize;
    let mut strict_ordered_pass_rows = 0usize;
    let mut unexpected_local_accepts_under_disabled_threshold = 0usize;
    for request in &requests {
        let prepared = sdk.prepare_active_fringe_from_iter(
            request
                .active_fringe
                .iter()
                .map(|active| (active.center_id, active.strength)),
        );
        let mut energy_margin = 0i32;
        let mut min_slot_margin = i32::MAX;
        let mut first_slot_margin = 0i32;
        let mut strict_ordered_pass = true;
        for (slot_index, slot) in request.slots.iter().enumerate() {
            let (positive_score, negative_score) =
                score_role_binding_profile_slot(&sdk, &prepared, slot);
            let slot_margin = positive_score - negative_score;
            if slot_index == 0 {
                first_slot_margin = slot_margin;
            }
            energy_margin = energy_margin.saturating_add(slot_margin);
            min_slot_margin = min_slot_margin.min(slot_margin);
            strict_ordered_pass &= slot_margin > 0;
        }
        if min_slot_margin == i32::MAX {
            continue;
        }
        positive_margin_rows += usize::from(energy_margin > 0);
        strict_ordered_pass_rows += usize::from(strict_ordered_pass);
        unexpected_local_accepts_under_disabled_threshold += usize::from(profile_accepts_score(
            &default_profile_acceptance_policy(),
            strict_ordered_pass,
            energy_margin,
            first_slot_margin,
            REAL_TRAFFIC_SERVING_OPS_DISABLED_THRESHOLD,
        ));
        energy_margins.push(energy_margin);
        min_slot_margins.push(min_slot_margin);
    }

    let profile = RoleBindingProfileConfig {
        profile_id: REAL_TRAFFIC_SERVING_OPS_PROFILE_ID.to_owned(),
        profile_kind: "role_binding_nwrb".to_owned(),
        operator_classes: vec![
            "serving_ops".to_owned(),
            "service_health".to_owned(),
            "route_gap".to_owned(),
        ],
        package_path: package_path.clone(),
        runtime_bytes_estimate: sdk.bytes_estimate(),
        edge_count: package_info.edge_count,
        slot_count: 3,
        threshold: REAL_TRAFFIC_SERVING_OPS_DISABLED_THRESHOLD,
        acceptance_policy: default_profile_acceptance_policy(),
        accepted_route_keys: vec![
            REAL_TRAFFIC_SERVING_OPS_ROUTE_KEY.to_owned(),
            REAL_TRAFFIC_SERVING_OPS_PROFILE_ID.to_owned(),
            "serving_ops_metric_payload_builder_v1".to_owned(),
        ],
    };
    registry
        .profiles
        .retain(|existing| existing.profile_id != profile.profile_id);
    registry.profiles.push(profile);
    registry.claim_boundary = "serving registry overlay for serving-ops .nwrb profile; generated from request-side dry-run payloads with threshold=i32::MAX so scoring telemetry is available but local accepts and server mutations remain disabled until service-health verification exists".to_owned();
    validate_registry_config(&registry)?;
    write_json_file(&registry_path, &registry)?;

    let mut sorted_energy = energy_margins.clone();
    let mut sorted_min_slot = min_slot_margins.clone();
    sorted_energy.sort_unstable();
    sorted_min_slot.sort_unstable();
    let report = RoleBindingServingOpsProfileReport {
        schema_version: "nando_role_binding_serving_ops_profile_v1".to_owned(),
        verdict: if unexpected_local_accepts_under_disabled_threshold == 0
            && build.package_training_requests > 0
            && package_info.edge_count > 0
        {
            "SERVING_OPS_PROFILE_V1_REVIEW_PROFILE_READY_ACCEPTS_DISABLED"
        } else {
            "SERVING_OPS_PROFILE_V1_REVIEW_REPAIR_REQUIRED"
        }
        .to_owned(),
        base_registry_path: base_registry_path.display().to_string(),
        dry_run_trace_path: dry_run_trace_path.display().to_string(),
        package_path: package_path.display().to_string(),
        registry_path: registry_path.display().to_string(),
        profile_id: REAL_TRAFFIC_SERVING_OPS_PROFILE_ID.to_owned(),
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: build.package_bytes.len(),
        edge_count: package_info.edge_count,
        runtime_bytes_estimate: sdk.bytes_estimate(),
        threshold: REAL_TRAFFIC_SERVING_OPS_DISABLED_THRESHOLD,
        trace_rows_read: trace_rows.len(),
        scoreable_payload_events: requests.len(),
        package_training_requests: build.package_training_requests,
        positive_updates: build.positive_updates,
        negative_updates: build.negative_updates,
        changed_edges: build.changed_edges,
        positive_margin_rows,
        strict_ordered_pass_rows,
        unexpected_local_accepts_under_disabled_threshold,
        median_energy_margin: percentile_i32_sorted(&sorted_energy, 50),
        p10_energy_margin: percentile_i32_sorted(&sorted_energy, 10),
        min_energy_margin: sorted_energy.first().copied().unwrap_or(0),
        median_min_slot_margin: percentile_i32_sorted(&sorted_min_slot, 50),
        p10_min_slot_margin: percentile_i32_sorted(&sorted_min_slot, 10),
        min_slot_margin: sorted_min_slot.first().copied().unwrap_or(0),
        raw_text_written: false,
        response_text_used: false,
        target_labels_used: false,
        proof_labels_used: false,
        server_mutation_enabled: false,
        local_accepts_enabled_on_real_traffic: false,
        market_claim_allowed: false,
        claim_boundary: "Serving-ops profile compilation only. The .nwrb package can measure real score/margins over request-side service/metric/status payloads, but it cannot restart daemons, local-accept, or prove savings without service_health_metric_verifier_v1 and false_accepts=0.".to_owned(),
        next_engineering_debt: "Run serving-ops shadow with this overlay registry, then attach service_health_metric_verifier_v1 before threshold calibration, local accept, or daemon/server action paths.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-serving-ops-profile-v1: {}",
        report.verdict
    );
    println!("  base_registry: {}", base_registry_path.display());
    println!("  dry_run_trace: {}", dry_run_trace_path.display());
    println!("  package: {}", package_path.display());
    println!("  registry: {}", registry_path.display());
    println!("  report: {}", report_path.display());
    println!("  edge_count: {}", report.edge_count);
    println!(
        "  scoreable_payload_events: {}",
        report.scoreable_payload_events
    );
    println!("  median_energy_margin: {}", report.median_energy_margin);
    println!(
        "  unexpected_local_accepts_under_disabled_threshold: {}",
        report.unexpected_local_accepts_under_disabled_threshold
    );
    println!("  server_mutation_enabled: false");
    println!("  local_accepts_enabled_on_real_traffic: false");
    Err("serving-ops profile is review-only; attach verifier before claims".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_serving_ops_output_evidence_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let input_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_PAYLOAD_DRY_RUN_TRACE_JSONL));
    let sessions_root = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/sessions"));
    let output_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_OUTPUT_EVIDENCE_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_OUTPUT_EVIDENCE_REPORT));

    let trace_rows = read_real_traffic_trace_jsonl(&input_trace_path)?;
    let wanted_request_fingerprints = trace_rows
        .iter()
        .filter(|row| {
            row.nando_shadow_request.as_ref().is_some_and(|request| {
                request.profile_id.as_deref() == Some(REAL_TRAFFIC_SERVING_OPS_PROFILE_ID)
            })
        })
        .filter_map(|row| row.request_fingerprint.as_deref())
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    let session_ids = trace_rows
        .iter()
        .filter(|row| row.nando_shadow_request.is_some())
        .filter_map(|row| codex_history_session_id_from_trace_id(&row.trace_id))
        .collect::<HashSet<_>>();
    let session_index = build_codex_session_output_evidence_index(
        &sessions_root,
        &session_ids,
        &wanted_request_fingerprints,
        deterministic_serving_ops_output_verification,
    )?;

    let mut enriched_rows = Vec::with_capacity(trace_rows.len());
    let mut operator_candidate_calls = 0usize;
    let mut scoreable_candidate_calls = 0usize;
    let mut output_evidence_matched_events = 0usize;
    let mut deterministic_verification_events = 0usize;
    let mut verified_true_events = 0usize;
    let mut verified_false_events = 0usize;
    let mut no_session_output_match_events = 0usize;
    let mut verifier_not_applicable_events = 0usize;

    for mut row in trace_rows {
        let Some(request) = &row.nando_shadow_request else {
            enriched_rows.push(row);
            continue;
        };
        operator_candidate_calls += 1;
        scoreable_candidate_calls +=
            usize::from(!request.active_fringe.is_empty() && !request.slots.is_empty());
        if request.profile_id.as_deref() != Some(REAL_TRAFFIC_SERVING_OPS_PROFILE_ID) {
            enriched_rows.push(row);
            continue;
        }
        let request_fingerprint = row.request_fingerprint.clone().unwrap_or_default();
        let Some(evidence) = session_index
            .by_request_fingerprint
            .get(&request_fingerprint)
        else {
            no_session_output_match_events += 1;
            row.notes = Some(append_trace_note(
                row.notes.as_deref(),
                "serving-ops output evidence missing: no matching Codex final answer found",
            ));
            enriched_rows.push(row);
            continue;
        };
        output_evidence_matched_events += 1;
        row.response_fingerprint = Some(evidence.response_fingerprint.clone());
        row.verification_source = Some(
            "codex_session_final_answer_fingerprint_plus_deterministic_service_health_metric_verifier_v1"
                .to_owned(),
        );
        row.verified_safe_accept = Some(evidence.verified_safe_accept);
        deterministic_verification_events += usize::from(evidence.verifier_applicable);
        verifier_not_applicable_events += usize::from(!evidence.verifier_applicable);
        verified_true_events += usize::from(evidence.verified_safe_accept);
        verified_false_events += usize::from(!evidence.verified_safe_accept);
        row.notes = Some(append_trace_note(
            row.notes.as_deref(),
            &format!(
                "serving-ops output evidence attached; verifier_status={}",
                evidence.verifier_status
            ),
        ));
        enriched_rows.push(row);
    }

    write_real_traffic_trace_jsonl(&output_trace_path, &enriched_rows)?;
    let report = RoleBindingEditOutputEvidenceReport {
        schema_version: "nando_role_binding_serving_ops_output_evidence_v1".to_owned(),
        verdict: if output_evidence_matched_events > 0 {
            "SERVING_OPS_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED"
        } else {
            "SERVING_OPS_OUTPUT_EVIDENCE_V1_REVIEW_NO_OUTPUT_EVIDENCE"
        }
        .to_owned(),
        input_trace_path: input_trace_path.display().to_string(),
        sessions_root: sessions_root.display().to_string(),
        output_trace_path: output_trace_path.display().to_string(),
        total_trace_rows: enriched_rows.len(),
        operator_candidate_calls,
        scoreable_candidate_calls,
        session_ids_requested: session_ids.len(),
        session_files_scanned: session_index.session_files_scanned,
        codex_turns_indexed: session_index.codex_turns_indexed,
        output_evidence_matched_events,
        no_session_output_match_events,
        deterministic_verification_events,
        verifier_not_applicable_events,
        verified_true_events,
        verified_false_events,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        response_text_used_for_verification: true,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Serving-ops output evidence join only. It reads local Codex final answers at analysis time, writes response fingerprints and deterministic service/metric/status verification results, writes no raw prompt/response text, performs no server mutation, and does not enable local accepts or market savings claims.".to_owned(),
        next_engineering_debt: "Run shadow analysis and verification-hook audit over the serving-ops evidence trace, then calibrate local accept only if verifier-true support is sufficient and false_accepts remain 0.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-serving-ops-output-evidence-v1: {}",
        report.verdict
    );
    println!("  input_trace: {}", input_trace_path.display());
    println!("  sessions_root: {}", sessions_root.display());
    println!("  output_trace: {}", output_trace_path.display());
    println!("  report: {}", report_path.display());
    println!(
        "  output_evidence_matched_events: {}",
        report.output_evidence_matched_events
    );
    println!("  verified_true_events: {}", report.verified_true_events);
    println!("  verified_false_events: {}", report.verified_false_events);
    println!("  raw_response_text_written: false");
    println!("  server_mutation_enabled: false");
    Err("serving-ops output evidence is review-only; run shadow/audit before claims".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_serving_ops_local_accept_calibration_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_PROFILE_REGISTRY_CONFIG));
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_OUTPUT_EVIDENCE_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_LOCAL_ACCEPT_CALIBRATION_REPORT));

    let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&registry_config_path)?;
    let trace_rows = read_real_traffic_trace_jsonl(&trace_path)?;
    let mut scored_rows = Vec::new();
    let mut hook_ready_rows = 0usize;
    let mut label_true_rows = 0usize;
    let mut label_false_rows = 0usize;
    let mut no_score_rows = 0usize;

    for row in &trace_rows {
        let Some(label) = row.verified_safe_accept else {
            continue;
        };
        let Some(request) = &row.nando_shadow_request else {
            continue;
        };
        if request.profile_id.as_deref() != Some(REAL_TRAFFIC_SERVING_OPS_PROFILE_ID) {
            continue;
        }
        hook_ready_rows += 1;
        label_true_rows += usize::from(label);
        label_false_rows += usize::from(!label);
        let Some(score) = score_role_binding_profile_request_detailed(&registry, request) else {
            no_score_rows += 1;
            continue;
        };
        let current_response = score_role_binding_profile_request(&registry, request);
        let service_slot_margin = score.slot_margins.first().copied().unwrap_or(0);
        let metric_slot_margin = score.slot_margins.get(1).copied().unwrap_or(0);
        scored_rows.push(RoleBindingEditLocalAcceptCalibrationRow {
            trace_id: row.trace_id.clone(),
            request_fingerprint: row.request_fingerprint.clone(),
            response_fingerprint: row.response_fingerprint.clone(),
            verifier_label: label,
            production_accepted: current_response.accepted,
            production_fallback_reason: current_response.fallback_reason,
            energy_margin: score.energy_margin,
            min_slot_margin: score.min_slot_margin,
            marker_slot_margin: service_slot_margin,
            end_slot_margin: metric_slot_margin,
            slot_count: score.slot_margins.len(),
        });
    }

    let current_policy =
        evaluate_edit_calibration_policy("current_disabled_profile_policy", &scored_rows, |row| {
            row.production_accepted
        });
    let energy_positive_policy =
        evaluate_edit_calibration_policy("energy_positive_no_slot_order", &scored_rows, |row| {
            row.energy_margin >= 1
        });
    let strict_positive_policy = evaluate_edit_calibration_policy(
        "strict_positive_slots_and_energy_positive",
        &scored_rows,
        |row| row.min_slot_margin > 0 && row.energy_margin >= 1,
    );
    let service_slot_policy =
        evaluate_edit_calibration_policy("service_slot_positive_only", &scored_rows, |row| {
            row.marker_slot_margin > 0 && row.energy_margin >= 1
        });
    let metric_slot_policy =
        evaluate_edit_calibration_policy("metric_slot_positive_only", &scored_rows, |row| {
            row.end_slot_margin > 0 && row.energy_margin >= 1
        });
    let best_energy_threshold_policy =
        best_single_threshold_policy("best_energy_margin_threshold", &scored_rows, |row| {
            row.energy_margin
        });
    let best_min_slot_threshold_policy =
        best_single_threshold_policy("best_min_slot_margin_threshold", &scored_rows, |row| {
            row.min_slot_margin
        });
    let best_service_slot_threshold_policy =
        best_single_threshold_policy("best_service_slot_margin_threshold", &scored_rows, |row| {
            row.marker_slot_margin
        });
    let best_metric_slot_threshold_policy =
        best_single_threshold_policy("best_metric_slot_margin_threshold", &scored_rows, |row| {
            row.end_slot_margin
        });
    let margin_collision_diagnostics = planning_margin_collision_diagnostics(&scored_rows);
    let request_side_margin_only_accepts_all_true_without_false = margin_collision_diagnostics
        .iter()
        .any(|diagnostic| diagnostic.safe_accepts_all_true_rows);
    let policies = vec![
        current_policy,
        energy_positive_policy,
        strict_positive_policy,
        service_slot_policy,
        metric_slot_policy,
        best_energy_threshold_policy,
        best_min_slot_threshold_policy,
        best_service_slot_threshold_policy,
        best_metric_slot_threshold_policy,
    ];
    let safe_policy_found = policies
        .iter()
        .any(|policy| policy.false_accepts == 0 && policy.true_accepts > 0);
    let best_safe_true_accepts = policies
        .iter()
        .filter(|policy| policy.false_accepts == 0)
        .map(|policy| policy.true_accepts)
        .max()
        .unwrap_or(0);
    let report = RoleBindingEditLocalAcceptCalibrationReport {
        schema_version: "nando_role_binding_serving_ops_local_accept_calibration_v1".to_owned(),
        verdict: if safe_policy_found {
            "SERVING_OPS_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_SAFE_POLICY_CANDIDATE_FOUND"
        } else {
            "SERVING_OPS_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_NO_SAFE_READOUT_POLICY"
        }
        .to_owned(),
        registry_config_path: registry_config_path.display().to_string(),
        trace_path: trace_path.display().to_string(),
        hook_ready_rows,
        scored_rows: scored_rows.len(),
        label_true_rows,
        label_false_rows,
        no_score_rows,
        safe_policy_found,
        best_safe_true_accepts,
        policies,
        rows: scored_rows,
        margin_collision_diagnostics,
        request_side_margin_only_accepts_all_true_without_false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Serving-ops calibration only. It evaluates score/readout policies against deterministic service-health/metric verifier labels, writes fingerprints and margins only, performs no server mutation, enables no local accepts, and cannot be used as a market savings claim.".to_owned(),
        next_engineering_debt: if safe_policy_found {
            "Promote only through a separate serving-ops safe-policy artifact, then rerun shadow/audit with provider cost, false_accepts=0, and unverified_shadow_accepts=0 before counting savings or touching daemons.".to_owned()
        } else {
            "Do not lower the serving-ops threshold. Current score geometry does not separate verifier-true from verifier-false rows; improve request-side admission, payload features, or tool-output verification before enabling local accepts.".to_owned()
        },
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-serving-ops-local-accept-calibration-v1: {}",
        report.verdict
    );
    println!("  registry_config: {}", registry_config_path.display());
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!("  hook_ready_rows: {}", report.hook_ready_rows);
    println!("  label_true_rows: {}", report.label_true_rows);
    println!("  label_false_rows: {}", report.label_false_rows);
    println!("  safe_policy_found: {}", report.safe_policy_found);
    println!(
        "  best_safe_true_accepts: {}",
        report.best_safe_true_accepts
    );
    Err("serving-ops local accept calibration is review-only".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_serving_ops_safe_policy_promote_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let base_registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_PROFILE_REGISTRY_CONFIG));
    let evidence_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_OUTPUT_EVIDENCE_TRACE_JSONL));
    let calibration_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_LOCAL_ACCEPT_CALIBRATION_REPORT));
    let promoted_registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_SAFE_POLICY_REGISTRY_CONFIG));
    let promoted_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_SAFE_POLICY_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SERVING_OPS_SAFE_POLICY_REPORT));
    let provider_cost_microusd = args
        .next()
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| format!("invalid provider_cost_microusd '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(100);

    let mut promoted_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&base_registry_config_path)?;
    validate_registry_config(&promoted_config)?;
    let calibration =
        read_json_file::<RoleBindingEditLocalAcceptCalibrationReport>(&calibration_report_path)?;
    let mut trace_rows = read_real_traffic_trace_jsonl(&evidence_trace_path)?;
    let base_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path(&base_registry_config_path)?;
    let metric_slot_candidate =
        if let Some(calibration_policy) = select_supported_serving_ops_safe_policy(&calibration) {
            Some((
                calibration_policy.policy_name.clone(),
                calibration_policy.threshold,
                select_serving_ops_promotion_policy_from_evidence(
                    &base_registry,
                    &trace_rows,
                    calibration_policy,
                )?,
            ))
        } else {
            None
        };
    let metric_slot_market_safe = metric_slot_candidate
        .as_ref()
        .is_some_and(|(_, _, policy)| {
            policy.policy_name == "market_safe_metric_slot_margin_threshold"
                && policy.true_accepts >= DEFAULT_REAL_TRAFFIC_MIN_SAFE_POLICY_TRUE_SUPPORT
                && policy.false_accepts == 0
                && policy.unverified_accepts == 0
        });
    let (calibration_policy_name, calibration_policy_threshold, policy, acceptance_policy) =
        if metric_slot_market_safe {
            let (calibration_policy_name, calibration_policy_threshold, policy) =
                metric_slot_candidate.expect("metric-slot candidate exists when market safe");
            (
                calibration_policy_name,
                calibration_policy_threshold,
                policy,
                PROFILE_ACCEPTANCE_POLICY_SECOND_SLOT_THRESHOLD.to_owned(),
            )
        } else {
            let Some(calibration_policy) = select_supported_mixed_safe_policy(&calibration) else {
                return Err(
                "serving-ops calibration report has neither a market-safe second-slot policy nor a supported safe energy-threshold fallback"
                    .to_owned(),
            );
            };
            (
                calibration_policy.policy_name.clone(),
                calibration_policy.threshold,
                select_mixed_promotion_policy_from_evidence(
                    &base_registry,
                    &trace_rows,
                    calibration_policy,
                    REAL_TRAFFIC_SERVING_OPS_ROUTE_KEY,
                )?,
                "energy_threshold_only".to_owned(),
            )
        };
    let threshold = policy.threshold;
    let serving_ops_profile_ids = trace_rows
        .iter()
        .filter_map(|row| row.nando_shadow_request.as_ref())
        .filter(|request| {
            request
                .route_key
                .as_deref()
                .is_some_and(|route| route == REAL_TRAFFIC_SERVING_OPS_ROUTE_KEY)
        })
        .filter_map(|request| request.profile_id.clone())
        .collect::<BTreeSet<_>>();
    if serving_ops_profile_ids.is_empty() {
        return Err(
            "serving-ops safe policy promotion found no serving_ops profile ids in trace"
                .to_owned(),
        );
    }
    let mut promoted_profile_ids = Vec::new();
    for profile in &mut promoted_config.profiles {
        if serving_ops_profile_ids.contains(&profile.profile_id) {
            profile.threshold = threshold;
            profile.acceptance_policy = acceptance_policy.clone();
            promoted_profile_ids.push(profile.profile_id.clone());
        }
    }
    if promoted_profile_ids.is_empty() {
        return Err(format!(
            "serving-ops safe policy promotion found no matching profiles in registry for {:?}",
            serving_ops_profile_ids
        ));
    }
    validate_registry_config(&promoted_config)?;
    write_json_file(&promoted_registry_config_path, &promoted_config)?;
    let promoted_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path(&promoted_registry_config_path)?;

    let mut scoreable_candidate_calls = 0usize;
    let mut policy_accept_rows = 0usize;
    let mut policy_accept_verified_true_rows = 0usize;
    let mut policy_accept_verified_false_rows = 0usize;
    let mut policy_accept_unverified_rows = 0usize;
    let mut provider_cost_events_written = 0usize;
    let mut runtime_acceptance_mismatches = 0usize;
    let mut no_score_rows = 0usize;

    for row in &mut trace_rows {
        let Some(request) = &mut row.nando_shadow_request else {
            continue;
        };
        let is_serving_ops = request
            .route_key
            .as_deref()
            .is_some_and(|route| route == REAL_TRAFFIC_SERVING_OPS_ROUTE_KEY);
        if !is_serving_ops {
            continue;
        }
        scoreable_candidate_calls +=
            usize::from(!request.active_fringe.is_empty() && !request.slots.is_empty());
        row.provider_cost_microusd = Some(provider_cost_microusd);
        provider_cost_events_written += 1;

        let Some(score) = score_role_binding_profile_request_detailed(&promoted_registry, request)
        else {
            no_score_rows += 1;
            request.expect_local_operator = Some(false);
            continue;
        };
        let strict_ordered_pass = score.slot_margins.iter().all(|margin| *margin > 0);
        let policy_accept = profile_accepts_score_with_second_slot(
            &acceptance_policy,
            strict_ordered_pass,
            score.energy_margin,
            score.slot_margins.first().copied().unwrap_or(0),
            score.slot_margins.get(1).copied(),
            threshold,
        );
        request.expect_local_operator = Some(policy_accept);
        if policy_accept {
            policy_accept_rows += 1;
            policy_accept_verified_true_rows += usize::from(row.verified_safe_accept == Some(true));
            policy_accept_verified_false_rows +=
                usize::from(row.verified_safe_accept == Some(false));
            policy_accept_unverified_rows += usize::from(row.verified_safe_accept.is_none());
        }
        let runtime_response = score_role_binding_profile_request(&promoted_registry, request);
        runtime_acceptance_mismatches += usize::from(runtime_response.accepted != policy_accept);
        row.notes = Some(format!(
            "{}; serving_ops_safe_policy_promote_v1 policy={} threshold={} provider_cost_estimate_microusd={} policy_accept={}",
            row.notes
                .clone()
                .unwrap_or_else(|| "real_codex_trace".to_owned()),
            acceptance_policy,
            threshold,
            provider_cost_microusd,
            policy_accept
        ));
    }

    write_real_traffic_trace_jsonl(&promoted_trace_path, &trace_rows)?;
    let report = RoleBindingMixedSafePolicyPromoteReport {
        schema_version: "nando_role_binding_serving_ops_safe_policy_promote_v1".to_owned(),
        verdict: if policy_accept_rows > 0
            && policy_accept_verified_false_rows == 0
            && policy_accept_unverified_rows == 0
            && runtime_acceptance_mismatches == 0
        {
            "SERVING_OPS_SAFE_POLICY_PROMOTE_V1_REVIEW_PROMOTED_TRACE_READY"
        } else {
            "SERVING_OPS_SAFE_POLICY_PROMOTE_V1_REVIEW_REQUIRES_SHADOW_AUDIT"
        }
        .to_owned(),
        base_registry_config_path: base_registry_config_path.display().to_string(),
        evidence_trace_path: evidence_trace_path.display().to_string(),
        calibration_report_path: calibration_report_path.display().to_string(),
        promoted_registry_config_path: promoted_registry_config_path.display().to_string(),
        promoted_trace_path: promoted_trace_path.display().to_string(),
        history_path: None,
        request_side_policy_name: None,
        calibration_policy_name,
        calibration_policy_threshold,
        selected_policy_name: policy.policy_name.clone(),
        selected_policy_source: policy.selection_source.clone(),
        selected_policy_threshold: threshold,
        selected_acceptance_policy: acceptance_policy,
        selected_policy_accepts: policy.accepts,
        selected_policy_true_accepts: policy.true_accepts,
        selected_policy_false_accepts: policy.false_accepts,
        selected_policy_unverified_accepts: policy.unverified_accepts,
        promoted_profile_ids,
        provider_cost_microusd,
        trace_rows_written: trace_rows.len(),
        scoreable_candidate_calls,
        request_side_policy_evaluated_rows: 0,
        request_side_policy_accept_rows: 0,
        request_side_policy_reject_rows: 0,
        history_prompt_missing_rows: 0,
        policy_accept_rows,
        policy_accept_verified_true_rows,
        policy_accept_verified_false_rows,
        policy_accept_unverified_rows,
        provider_cost_events_written,
        no_score_rows,
        runtime_acceptance_mismatches,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        target_labels_used_for_runtime: false,
        proof_labels_used_for_runtime: false,
        market_claim_allowed: false,
        claim_boundary: "Promotion artifact only. It creates a promoted serving-ops registry and rewrites an evidence-backed shadow trace with provider-cost estimates. Offline labels/evidence choose the threshold, but serving uses only request-side payload score >= threshold. It performs no server mutation and does not prove market savings until shadow plus verification-hook audit pass with false_accepts=0 and unverified_shadow_accepts=0.".to_owned(),
        next_engineering_debt: "Run role-binding-real-traffic-shadow-v1 and verification-hook-audit-v1 on the promoted serving-ops registry/trace, then feed that safe-policy audit into CPU route feedback. Server/daemon actions stay disabled until a separate action executor gate exists.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-serving-ops-safe-policy-promote-v1: {}",
        report.verdict
    );
    println!(
        "  promoted_registry: {}",
        promoted_registry_config_path.display()
    );
    println!("  promoted_trace: {}", promoted_trace_path.display());
    println!("  report: {}", report_path.display());
    println!("  selected_policy_name: {}", report.selected_policy_name);
    println!(
        "  selected_policy_source: {}",
        report.selected_policy_source
    );
    println!(
        "  selected_policy_threshold: {}",
        report.selected_policy_threshold
    );
    println!("  policy_accept_rows: {}", report.policy_accept_rows);
    println!(
        "  policy_accept_verified_true_rows: {}",
        report.policy_accept_verified_true_rows
    );
    println!(
        "  policy_accept_verified_false_rows: {}",
        report.policy_accept_verified_false_rows
    );
    println!(
        "  policy_accept_unverified_rows: {}",
        report.policy_accept_unverified_rows
    );
    Err(
        "serving-ops safe policy promotion is review-only; run shadow/audit before claims"
            .to_owned(),
    )
}
