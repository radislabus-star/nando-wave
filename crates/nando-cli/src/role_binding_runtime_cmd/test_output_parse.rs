pub(crate) fn run_role_binding_real_traffic_test_output_parse_payload_dry_run_v1<I>(
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
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let broad_split_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REAL_TRAFFIC_BROAD_ROUTE_SPLIT_DISCOVERY_REPORT));
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_PAYLOAD_DRY_RUN_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_PAYLOAD_DRY_RUN_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(5000);

    let registry_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&registry_config_path)?;
    validate_registry_config(&registry_config)?;
    let profile_registered = registry_config
        .profiles
        .iter()
        .any(|profile| profile.profile_id == REAL_TRAFFIC_TEST_OUTPUT_PARSE_PROFILE_ID);
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let broad_report =
        read_json_file::<RoleBindingBroadRouteSplitDiscoveryReport>(&broad_split_report_path)?;
    let test_output_history_indexes = broad_report
        .rows
        .iter()
        .filter(|row| row.split_key == REAL_TRAFFIC_TEST_OUTPUT_PARSE_SPLIT_KEY)
        .filter_map(|row| {
            route_gap_payload_readiness_history_index(&row.event_id).map(|index| (index, row))
        })
        .collect::<BTreeMap<_, _>>();
    let skip = history_rows.len().saturating_sub(max_events);
    let mut trace_rows = Vec::with_capacity(history_rows.len().saturating_sub(skip));
    let mut report_rows = Vec::new();
    let mut parent_route_counts = BTreeMap::<String, usize>::new();
    let mut test_output_parse_candidate_events = 0usize;
    let mut non_exact_candidate_events = 0usize;
    let mut exact_cache_overlap_events = 0usize;
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
            "codex_history_test_output_parse_payload_dry_run::{}::{}::{}",
            row.session_id, row.ts, index
        );
        let request_fingerprint = format!("fnv1a64:{fingerprint:016x}");
        let exact_cache_key = Some(format!("codex_history_request:{fingerprint:016x}"));
        let mut nando_shadow_request = None;
        let mut notes = "not test_output_parse broad-split candidate".to_owned();

        if let Some(split_row) = test_output_history_indexes.get(&index) {
            test_output_parse_candidate_events += 1;
            exact_cache_overlap_events += usize::from(split_row.exact_cache_hit);
            non_exact_candidate_events += usize::from(!split_row.exact_cache_hit);
            *parent_route_counts
                .entry(split_row.parent_route_key.clone())
                .or_insert(0) += 1;
            if split_row.payload_ready {
                payload_ready_events += 1;
                let built =
                    build_test_output_parse_dry_run_request(&event_id, &fingerprint, &row.text);
                match built {
                    Some((request, tokens)) => {
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
                        report_rows.push(RoleBindingTestOutputParsePayloadDryRunRow {
                            event_id: event_id.clone(),
                            request_fingerprint: request_fingerprint.clone(),
                            parent_route_key: split_row.parent_route_key.clone(),
                            split_key: split_row.split_key.clone(),
                            route_key: REAL_TRAFFIC_TEST_OUTPUT_PARSE_ROUTE_KEY.to_owned(),
                            profile_id: REAL_TRAFFIC_TEST_OUTPUT_PARSE_PROFILE_ID.to_owned(),
                            exact_cache_hit: split_row.exact_cache_hit,
                            readiness_payload_ready: true,
                            payload_built: true,
                            scoreable,
                            profile_registered,
                            builder_status: builder_status.clone(),
                            active_fringe_centers,
                            slots,
                            positive_impulses,
                            negative_impulses,
                            command_token: tokens.command_token,
                            status_token: tokens.status_token,
                            artifact_token: tokens.artifact_token,
                            boundary_token: tokens.boundary_token,
                            feature_flags: split_row.feature_flags.clone(),
                        });
                        notes = format!(
                            "request-side test-output-parse payload built from broad-split row; status={builder_status}; verified accepts disabled"
                        );
                        nando_shadow_request = Some(request);
                    }
                    None => {
                        builder_rejected_events += 1;
                        let builder_status = "builder_rejected_request_side_features".to_owned();
                        *builder_status_counts
                            .entry(builder_status.clone())
                            .or_insert(0) += 1;
                        report_rows.push(RoleBindingTestOutputParsePayloadDryRunRow {
                            event_id: event_id.clone(),
                            request_fingerprint: request_fingerprint.clone(),
                            parent_route_key: split_row.parent_route_key.clone(),
                            split_key: split_row.split_key.clone(),
                            route_key: REAL_TRAFFIC_TEST_OUTPUT_PARSE_ROUTE_KEY.to_owned(),
                            profile_id: REAL_TRAFFIC_TEST_OUTPUT_PARSE_PROFILE_ID.to_owned(),
                            exact_cache_hit: split_row.exact_cache_hit,
                            readiness_payload_ready: true,
                            payload_built: false,
                            scoreable: false,
                            profile_registered,
                            builder_status: builder_status.clone(),
                            active_fringe_centers: 0,
                            slots: 0,
                            positive_impulses: 0,
                            negative_impulses: 0,
                            command_token: String::new(),
                            status_token: String::new(),
                            artifact_token: String::new(),
                            boundary_token: String::new(),
                            feature_flags: split_row.feature_flags.clone(),
                        });
                        notes = builder_status;
                    }
                }
            } else {
                readiness_rejected_events += 1;
                let builder_status = "broad_split_payload_not_ready".to_owned();
                *builder_status_counts
                    .entry(builder_status.clone())
                    .or_insert(0) += 1;
                notes = builder_status;
            }
        }

        trace_rows.push(RoleBindingRealTrafficTraceRow {
            schema_version: "nando_role_binding_real_traffic_trace_v1".to_owned(),
            trace_id: event_id,
            traffic_source: Some("codex_history_test_output_parse_payload_dry_run".to_owned()),
            time_ms: Some(row.ts.saturating_mul(1000)),
            request_fingerprint: Some(request_fingerprint),
            response_fingerprint: None,
            tool_call_fingerprints: Vec::new(),
            verification_source: Some(
                "request-side test_output_parse payload dry-run from broad-route split report and local Codex prompt only; raw text, response text, target labels, and proof labels not written"
                    .to_owned(),
            ),
            llm_call: true,
            exact_cache_key,
            provider_cache_hit: None,
            provider_cost_microusd: None,
            nando_shadow_request,
            verified_safe_accept: None,
            synthetic_source: Some(false),
            notes: Some(notes),
        });
    }

    write_real_traffic_trace_jsonl(&trace_path, &trace_rows)?;
    let shadow_score_ready = profile_registered && scoreable_payload_events > 0;
    let report = RoleBindingTestOutputParsePayloadDryRunReport {
        schema_version: "nando_role_binding_test_output_parse_payload_dry_run_v1".to_owned(),
        verdict: if shadow_score_ready {
            "TEST_OUTPUT_PARSE_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PROFILE_READY"
        } else if scoreable_payload_events > 0 {
            "TEST_OUTPUT_PARSE_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_PROFILE_MISSING"
        } else {
            "TEST_OUTPUT_PARSE_PAYLOAD_DRY_RUN_V1_REVIEW_NO_SCOREABLE_PAYLOADS"
        }
        .to_owned(),
        history_path: history_path.display().to_string(),
        registry_config_path: registry_config_path.display().to_string(),
        broad_split_report_path: broad_split_report_path.display().to_string(),
        trace_path: trace_path.display().to_string(),
        max_events,
        total_history_rows: history_rows.len(),
        sampled_history_rows: history_rows.len().saturating_sub(skip),
        trace_rows_written: trace_rows.len(),
        test_output_parse_candidate_events,
        non_exact_candidate_events,
        exact_cache_overlap_events,
        payload_ready_events,
        payload_built_events,
        scoreable_payload_events,
        builder_rejected_events,
        readiness_rejected_events,
        profile_registered,
        shadow_score_ready,
        parent_route_counts: parent_route_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        active_fringe_centers_total,
        slots_total,
        positive_impulses_total,
        negative_impulses_total,
        builder_status_counts: builder_status_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        expected_unique_cpu_accepts_over_exact_cache: 0,
        expected_savings_milli: 0,
        false_accepts: 0,
        raw_text_written: false,
        response_text_used: false,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        rows: report_rows,
        claim_boundary: "Request-side narrow split dry-run only. It uses broad-route split discovery to select test_output_parse rows, reads prompt text at analysis time, writes fingerprints/features/counts but no raw prompt text, enables no local accepts, uses no response/target/proof labels, and cannot prove CPU savings. A real command-output verifier is required before promotion.".to_owned(),
        next_engineering_debt: "Compile a disabled-threshold test_output_parse .nwrb profile only for scoreable rows, attach tool_output_validation_result_verifier_v1 output evidence, calibrate request-side admission, then count unique CPU accepts only after shadow/audit proves false_accepts=0 over exact cache.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-test-output-parse-payload-dry-run-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!("  registry_config: {}", registry_config_path.display());
    println!(
        "  broad_split_report: {}",
        broad_split_report_path.display()
    );
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!(
        "  test_output_parse_candidate_events: {}",
        report.test_output_parse_candidate_events
    );
    println!(
        "  non_exact_candidate_events: {}",
        report.non_exact_candidate_events
    );
    println!("  payload_ready_events: {}", report.payload_ready_events);
    println!("  payload_built_events: {}", report.payload_built_events);
    println!(
        "  scoreable_payload_events: {}",
        report.scoreable_payload_events
    );
    println!("  profile_registered: {}", report.profile_registered);
    println!("  local_accepts_enabled: false");
    Err(
        "test-output-parse payload dry-run is review-only; build profile+verifier before claims"
            .to_owned(),
    )
}

pub(crate) fn run_role_binding_real_traffic_test_output_parse_output_evidence_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let input_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_PAYLOAD_DRY_RUN_TRACE_JSONL));
    let sessions_root = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/sessions"));
    let output_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_OUTPUT_EVIDENCE_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_OUTPUT_EVIDENCE_REPORT));

    let trace_rows = read_real_traffic_trace_jsonl(&input_trace_path)?;
    let wanted_request_fingerprints = trace_rows
        .iter()
        .filter(|row| {
            row.nando_shadow_request.as_ref().is_some_and(|request| {
                request.profile_id.as_deref() == Some(REAL_TRAFFIC_TEST_OUTPUT_PARSE_PROFILE_ID)
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
        deterministic_test_output_parse_output_verification,
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
        if request.profile_id.as_deref() != Some(REAL_TRAFFIC_TEST_OUTPUT_PARSE_PROFILE_ID) {
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
                "test-output-parse output evidence missing: no matching Codex final answer found",
            ));
            enriched_rows.push(row);
            continue;
        };
        output_evidence_matched_events += 1;
        row.response_fingerprint = Some(evidence.response_fingerprint.clone());
        row.tool_call_fingerprints = evidence.tool_call_fingerprints.clone();
        row.verification_source = Some(
            "codex_session_final_answer_fingerprint_plus_deterministic_test_status_and_error_excerpt_verifier_v1"
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
                "test-output-parse output evidence attached; verifier_status={}",
                evidence.verifier_status
            ),
        ));
        enriched_rows.push(row);
    }

    write_real_traffic_trace_jsonl(&output_trace_path, &enriched_rows)?;
    let report = RoleBindingEditOutputEvidenceReport {
        schema_version: "nando_role_binding_test_output_parse_output_evidence_v1".to_owned(),
        verdict: if output_evidence_matched_events > 0 {
            "TEST_OUTPUT_PARSE_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED"
        } else {
            "TEST_OUTPUT_PARSE_OUTPUT_EVIDENCE_V1_REVIEW_NO_OUTPUT_EVIDENCE"
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
        claim_boundary: "Test-output-parse output evidence join only. It reads local Codex final answers at analysis time, writes response fingerprints and conservative pass/fail/error verifier status, writes no raw prompt/response text, and does not enable local accepts or market savings claims. It is not a substitute for live tool-output state in the request.".to_owned(),
        next_engineering_debt: "If verifier-true support is sufficient, compile a disabled-threshold test_output_parse .nwrb profile and run shadow/audit. If support remains tiny, add agent-loop tool-output state capture before attempting local accepts.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-test-output-parse-output-evidence-v1: {}",
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
    Err(
        "test-output-parse output evidence is review-only; run profile/shadow/audit before claims"
            .to_owned(),
    )
}

pub(crate) fn run_role_binding_real_traffic_test_output_parse_tool_output_state_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let broad_split_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REAL_TRAFFIC_BROAD_ROUTE_SPLIT_DISCOVERY_REPORT));
    let sessions_root = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/sessions"));
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_TOOL_OUTPUT_STATE_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_TOOL_OUTPUT_STATE_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(5000);

    let history_rows = read_codex_history_jsonl(&history_path)?;
    let broad_report =
        read_json_file::<RoleBindingBroadRouteSplitDiscoveryReport>(&broad_split_report_path)?;
    let test_output_history_indexes = broad_report
        .rows
        .iter()
        .filter(|row| row.split_key == REAL_TRAFFIC_TEST_OUTPUT_PARSE_SPLIT_KEY)
        .filter_map(|row| {
            route_gap_payload_readiness_history_index(&row.event_id).map(|index| (index, row))
        })
        .collect::<BTreeMap<_, _>>();
    let skip = history_rows.len().saturating_sub(max_events);

    let mut wanted_request_fingerprints = HashSet::new();
    let mut session_ids = HashSet::new();
    for (index, row) in history_rows.iter().enumerate().skip(skip) {
        if test_output_history_indexes.contains_key(&index) {
            let fingerprint = stable_real_traffic_fingerprint64(row.text.as_bytes());
            wanted_request_fingerprints.insert(format!("fnv1a64:{fingerprint:016x}"));
            session_ids.insert(row.session_id.clone());
        }
    }

    let state_index = build_codex_session_previous_tool_output_state_index(
        &sessions_root,
        &session_ids,
        &wanted_request_fingerprints,
    )?;

    let mut trace_rows = Vec::new();
    let mut report_rows = Vec::new();
    let mut parent_route_counts = BTreeMap::<String, usize>::new();
    let mut command_signal_counts = BTreeMap::<String, usize>::new();
    let mut status_counts = BTreeMap::<String, usize>::new();
    let mut test_output_parse_candidate_events = 0usize;
    let mut non_exact_candidate_events = 0usize;
    let mut exact_cache_overlap_events = 0usize;
    let mut tool_output_state_matched_events = 0usize;
    let mut missing_tool_output_state_events = 0usize;
    let mut command_status_detected_events = 0usize;
    let mut pass_status_events = 0usize;
    let mut fail_status_events = 0usize;
    let mut warning_status_events = 0usize;
    let mut unknown_status_events = 0usize;

    for (index, row) in history_rows.iter().enumerate().skip(skip) {
        let Some(split_row) = test_output_history_indexes.get(&index) else {
            continue;
        };
        test_output_parse_candidate_events += 1;
        exact_cache_overlap_events += usize::from(split_row.exact_cache_hit);
        non_exact_candidate_events += usize::from(!split_row.exact_cache_hit);
        *parent_route_counts
            .entry(split_row.parent_route_key.clone())
            .or_insert(0) += 1;

        let fingerprint = stable_real_traffic_fingerprint64(row.text.as_bytes());
        let request_fingerprint = format!("fnv1a64:{fingerprint:016x}");
        let event_id = format!(
            "codex_history_test_output_parse_tool_output_state::{}::{}::{}",
            row.session_id, row.ts, index
        );
        let evidence = state_index
            .by_request_fingerprint
            .get(&request_fingerprint)
            .cloned();
        let previous_tool_output_matched = evidence
            .as_ref()
            .and_then(|evidence| evidence.tool_output_fingerprint.as_ref())
            .is_some();
        tool_output_state_matched_events += usize::from(previous_tool_output_matched);
        missing_tool_output_state_events += usize::from(!previous_tool_output_matched);

        let command_signal = evidence
            .as_ref()
            .map(|evidence| evidence.command_signal.clone())
            .unwrap_or_else(|| "no_previous_test_tool_output".to_owned());
        let command_status = evidence
            .as_ref()
            .map(|evidence| evidence.command_status.clone())
            .unwrap_or_else(|| "unknown".to_owned());
        let verifier_status = evidence
            .as_ref()
            .map(|evidence| evidence.verifier_status.clone())
            .unwrap_or_else(|| "missing_previous_test_tool_output_state".to_owned());
        let tool_output_fingerprint = evidence
            .as_ref()
            .and_then(|evidence| evidence.tool_output_fingerprint.clone());
        let tool_call_fingerprints = evidence
            .as_ref()
            .map(|evidence| evidence.tool_call_fingerprints.clone())
            .unwrap_or_default();

        *command_signal_counts
            .entry(command_signal.clone())
            .or_insert(0) += 1;
        *status_counts.entry(command_status.clone()).or_insert(0) += 1;
        command_status_detected_events +=
            usize::from(previous_tool_output_matched && command_status != "unknown");
        pass_status_events += usize::from(command_status == "pass");
        fail_status_events += usize::from(command_status == "fail");
        warning_status_events += usize::from(command_status == "warning");
        unknown_status_events += usize::from(command_status == "unknown");

        report_rows.push(RoleBindingTestOutputParseToolOutputStateRow {
            event_id: event_id.clone(),
            request_fingerprint: request_fingerprint.clone(),
            parent_route_key: split_row.parent_route_key.clone(),
            split_key: split_row.split_key.clone(),
            exact_cache_hit: split_row.exact_cache_hit,
            previous_tool_output_matched,
            command_signal: command_signal.clone(),
            command_status: command_status.clone(),
            verifier_status: verifier_status.clone(),
            tool_output_fingerprint: tool_output_fingerprint.clone(),
            tool_call_fingerprint_events: tool_call_fingerprints.len(),
            request_time_state_only: true,
            feature_flags: split_row.feature_flags.clone(),
        });

        trace_rows.push(RoleBindingRealTrafficTraceRow {
            schema_version: "nando_role_binding_real_traffic_trace_v1".to_owned(),
            trace_id: event_id,
            traffic_source: Some("codex_history_test_output_parse_tool_output_state".to_owned()),
            time_ms: Some(row.ts.saturating_mul(1000)),
            request_fingerprint: Some(request_fingerprint),
            response_fingerprint: None,
            tool_call_fingerprints,
            verification_source: Some(
                "request-time previous Codex tool-output state fingerprint plus deterministic test/check status classifier; raw prompt/tool-output/response text not written"
                    .to_owned(),
            ),
            llm_call: true,
            exact_cache_key: Some(format!("codex_history_request:{fingerprint:016x}")),
            provider_cache_hit: None,
            provider_cost_microusd: None,
            nando_shadow_request: None,
            verified_safe_accept: None,
            synthetic_source: Some(false),
            notes: Some(format!(
                "test-output-parse previous tool-output state capture; matched={previous_tool_output_matched}; status={command_status}; signal={command_signal}; verifier_status={verifier_status}; local accepts disabled"
            )),
        });
    }

    write_real_traffic_trace_jsonl(&trace_path, &trace_rows)?;
    let report = RoleBindingTestOutputParseToolOutputStateReport {
        schema_version: "nando_role_binding_test_output_parse_tool_output_state_v1".to_owned(),
        verdict: if test_output_parse_candidate_events == 0 {
            "TEST_OUTPUT_PARSE_TOOL_OUTPUT_STATE_V1_REVIEW_NO_CANDIDATES"
        } else if tool_output_state_matched_events > 0 {
            "TEST_OUTPUT_PARSE_TOOL_OUTPUT_STATE_V1_REVIEW_TOOL_STATE_ATTACHED"
        } else {
            "TEST_OUTPUT_PARSE_TOOL_OUTPUT_STATE_V1_REVIEW_NO_TOOL_STATE_MATCHES"
        }
        .to_owned(),
        history_path: history_path.display().to_string(),
        broad_split_report_path: broad_split_report_path.display().to_string(),
        sessions_root: sessions_root.display().to_string(),
        trace_path: trace_path.display().to_string(),
        max_events,
        total_history_rows: history_rows.len(),
        sampled_history_rows: history_rows.len().saturating_sub(skip),
        trace_rows_written: trace_rows.len(),
        test_output_parse_candidate_events,
        non_exact_candidate_events,
        exact_cache_overlap_events,
        session_ids_requested: session_ids.len(),
        session_files_scanned: state_index.session_files_scanned,
        codex_turns_indexed: state_index.codex_turns_indexed,
        tool_outputs_indexed: state_index.tool_outputs_indexed,
        tool_output_state_matched_events,
        missing_tool_output_state_events,
        command_status_detected_events,
        pass_status_events,
        fail_status_events,
        warning_status_events,
        unknown_status_events,
        parent_route_counts: parent_route_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        command_signal_counts: command_signal_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        status_counts: status_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        expected_unique_cpu_accepts_over_exact_cache: 0,
        expected_savings_milli: 0,
        false_accepts: 0,
        raw_prompt_text_written: false,
        raw_tool_output_text_written: false,
        raw_response_text_written: false,
        response_text_used: false,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        rows: report_rows,
        claim_boundary: "Request-time agent-loop state capture only. It associates test_output_parse candidate prompts with the previous Codex tool-output fingerprint/status visible before that user request, writes no raw prompt/tool-output/response text, reads no target/proof labels, enables no local accepts, and cannot prove savings until a payload builder/profile/admission policy uses this state under false_accepts=0.".to_owned(),
        next_engineering_debt: "Use matched previous-tool-output states to build scoreable test_output_parse payloads without final-answer evidence; then train/compile a disabled profile, run shadow/admission audit, and count unique verified CPU accepts over exact cache only if verifier support is sufficient.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-test-output-parse-tool-output-state-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!(
        "  broad_split_report: {}",
        broad_split_report_path.display()
    );
    println!("  sessions_root: {}", sessions_root.display());
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!(
        "  test_output_parse_candidate_events: {}",
        report.test_output_parse_candidate_events
    );
    println!(
        "  tool_output_state_matched_events: {}",
        report.tool_output_state_matched_events
    );
    println!(
        "  command_status_detected_events: {}",
        report.command_status_detected_events
    );
    println!("  local_accepts_enabled: false");
    Err(
        "test-output-parse tool-output state capture is review-only; build payload/profile/admission before claims"
            .to_owned(),
    )
}

pub(crate) fn run_role_binding_real_traffic_test_output_parse_tool_state_payload_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let tool_state_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_TOOL_OUTPUT_STATE_REPORT));
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_TOOL_STATE_PAYLOAD_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_TOOL_STATE_PAYLOAD_REPORT));

    let tool_state_report =
        read_json_file::<RoleBindingTestOutputParseToolOutputStateReport>(&tool_state_report_path)?;
    let registry_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&registry_config_path)?;
    validate_registry_config(&registry_config)?;
    let profile_registered = registry_config
        .profiles
        .iter()
        .any(|profile| profile.profile_id == REAL_TRAFFIC_TEST_OUTPUT_PARSE_PROFILE_ID);

    let mut trace_rows = Vec::with_capacity(tool_state_report.rows.len());
    let mut report_rows = Vec::new();
    let mut parent_route_counts = BTreeMap::<String, usize>::new();
    let mut command_signal_counts = BTreeMap::<String, usize>::new();
    let mut status_counts = BTreeMap::<String, usize>::new();
    let mut builder_status_counts = BTreeMap::<String, usize>::new();
    let mut operator_candidate_calls = 0usize;
    let mut non_exact_candidate_events = 0usize;
    let mut exact_cache_overlap_events = 0usize;
    let mut tool_output_state_matched_events = 0usize;
    let mut command_status_detected_events = 0usize;
    let mut payload_ready_events = 0usize;
    let mut payload_built_events = 0usize;
    let mut scoreable_payload_events = 0usize;
    let mut builder_rejected_events = 0usize;
    let mut active_fringe_centers_total = 0usize;
    let mut slots_total = 0usize;
    let mut positive_impulses_total = 0usize;
    let mut negative_impulses_total = 0usize;

    for row in &tool_state_report.rows {
        operator_candidate_calls += 1;
        exact_cache_overlap_events += usize::from(row.exact_cache_hit);
        non_exact_candidate_events += usize::from(!row.exact_cache_hit);
        tool_output_state_matched_events += usize::from(row.previous_tool_output_matched);
        command_status_detected_events +=
            usize::from(row.previous_tool_output_matched && row.command_status != "unknown");
        *parent_route_counts
            .entry(row.parent_route_key.clone())
            .or_insert(0) += 1;
        *command_signal_counts
            .entry(row.command_signal.clone())
            .or_insert(0) += 1;
        *status_counts.entry(row.command_status.clone()).or_insert(0) += 1;

        let mut nando_shadow_request = None;
        let mut builder_status = if !row.previous_tool_output_matched {
            "missing_previous_tool_output_state"
        } else if row.command_status == "unknown" {
            "unknown_command_status_requires_fallback"
        } else {
            "payload_ready_from_previous_tool_output_state"
        }
        .to_owned();
        let mut payload_built = false;
        let mut scoreable = false;
        let mut active_fringe_centers = 0usize;
        let mut slots = 0usize;
        let mut positive_impulses = 0usize;
        let mut negative_impulses = 0usize;
        let mut command_token = String::new();
        let mut status_token = String::new();
        let mut artifact_token = String::new();
        let mut boundary_token = String::new();

        if row.previous_tool_output_matched && row.command_status != "unknown" {
            payload_ready_events += 1;
            if let Some((request, tokens)) =
                build_test_output_parse_tool_state_request(&row.event_id, row)
            {
                active_fringe_centers = request.active_fringe.len();
                slots = request.slots.len();
                positive_impulses = request
                    .slots
                    .iter()
                    .map(|slot| slot.positive_impulses.len())
                    .sum::<usize>();
                negative_impulses = request
                    .slots
                    .iter()
                    .map(|slot| slot.negative_impulses.len())
                    .sum::<usize>();
                scoreable = active_fringe_centers > 0 && slots > 0;
                payload_built = true;
                payload_built_events += 1;
                scoreable_payload_events += usize::from(scoreable);
                active_fringe_centers_total += active_fringe_centers;
                slots_total += slots;
                positive_impulses_total += positive_impulses;
                negative_impulses_total += negative_impulses;
                builder_status = if scoreable && profile_registered {
                    "scoreable_tool_state_payload_profile_registered"
                } else if scoreable {
                    "scoreable_tool_state_payload_profile_missing"
                } else {
                    "tool_state_payload_built_but_not_scoreable"
                }
                .to_owned();
                command_token = tokens.command_token;
                status_token = tokens.status_token;
                artifact_token = tokens.artifact_token;
                boundary_token = tokens.boundary_token;
                nando_shadow_request = Some(request);
            } else {
                builder_rejected_events += 1;
                builder_status = "tool_state_payload_builder_rejected".to_owned();
            }
        } else {
            builder_rejected_events += 1;
        }

        *builder_status_counts
            .entry(builder_status.clone())
            .or_insert(0) += 1;
        report_rows.push(RoleBindingTestOutputParseToolStatePayloadRow {
            event_id: row.event_id.clone(),
            request_fingerprint: row.request_fingerprint.clone(),
            parent_route_key: row.parent_route_key.clone(),
            split_key: row.split_key.clone(),
            route_key: REAL_TRAFFIC_TEST_OUTPUT_PARSE_ROUTE_KEY.to_owned(),
            profile_id: REAL_TRAFFIC_TEST_OUTPUT_PARSE_PROFILE_ID.to_owned(),
            exact_cache_hit: row.exact_cache_hit,
            previous_tool_output_matched: row.previous_tool_output_matched,
            command_signal: row.command_signal.clone(),
            command_status: row.command_status.clone(),
            payload_built,
            scoreable,
            profile_registered,
            builder_status: builder_status.clone(),
            active_fringe_centers,
            slots,
            positive_impulses,
            negative_impulses,
            command_token,
            status_token,
            artifact_token,
            boundary_token,
            feature_flags: row.feature_flags.clone(),
        });

        trace_rows.push(RoleBindingRealTrafficTraceRow {
            schema_version: "nando_role_binding_real_traffic_trace_v1".to_owned(),
            trace_id: row.event_id.clone(),
            traffic_source: Some("codex_history_test_output_parse_tool_state_payload".to_owned()),
            time_ms: None,
            request_fingerprint: Some(row.request_fingerprint.clone()),
            response_fingerprint: None,
            tool_call_fingerprints: row
                .tool_output_fingerprint
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            verification_source: Some(
                "request-side test_output_parse payload from previous tool-output state report only; raw prompt/tool-output/response text, target labels, and proof labels not used"
                    .to_owned(),
            ),
            llm_call: true,
            exact_cache_key: row
                .request_fingerprint
                .strip_prefix("fnv1a64:")
                .map(|fingerprint| format!("codex_history_request:{fingerprint}")),
            provider_cache_hit: None,
            provider_cost_microusd: None,
            nando_shadow_request,
            verified_safe_accept: None,
            synthetic_source: Some(false),
            notes: Some(format!(
                "test-output-parse tool-state payload dry-run; status={}; builder_status={builder_status}; local accepts disabled",
                row.command_status
            )),
        });
    }

    write_real_traffic_trace_jsonl(&trace_path, &trace_rows)?;
    let shadow_score_ready = profile_registered && scoreable_payload_events > 0;
    let report = RoleBindingTestOutputParseToolStatePayloadReport {
        schema_version: "nando_role_binding_test_output_parse_tool_state_payload_v1".to_owned(),
        verdict: if shadow_score_ready {
            "TEST_OUTPUT_PARSE_TOOL_STATE_PAYLOAD_V1_REVIEW_SCOREABLE_PROFILE_READY"
        } else if scoreable_payload_events > 0 {
            "TEST_OUTPUT_PARSE_TOOL_STATE_PAYLOAD_V1_REVIEW_SCOREABLE_PAYLOADS_PROFILE_MISSING"
        } else {
            "TEST_OUTPUT_PARSE_TOOL_STATE_PAYLOAD_V1_REVIEW_NO_SCOREABLE_PAYLOADS"
        }
        .to_owned(),
        tool_state_report_path: tool_state_report_path.display().to_string(),
        registry_config_path: registry_config_path.display().to_string(),
        trace_path: trace_path.display().to_string(),
        input_state_rows: tool_state_report.rows.len(),
        operator_candidate_calls,
        non_exact_candidate_events,
        exact_cache_overlap_events,
        tool_output_state_matched_events,
        command_status_detected_events,
        payload_ready_events,
        payload_built_events,
        scoreable_payload_events,
        builder_rejected_events,
        profile_registered,
        shadow_score_ready,
        parent_route_counts: parent_route_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        command_signal_counts: command_signal_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        status_counts: status_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        builder_status_counts: builder_status_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        active_fringe_centers_total,
        slots_total,
        positive_impulses_total,
        negative_impulses_total,
        expected_unique_cpu_accepts_over_exact_cache: 0,
        expected_savings_milli: 0,
        false_accepts: 0,
        raw_prompt_text_written: false,
        raw_tool_output_text_written: false,
        raw_response_text_written: false,
        response_text_used: false,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        rows: report_rows,
        claim_boundary: "Request-side payload dry-run from previous tool-output state only. It reads the tool-output-state report, writes scoreable Nando shadow requests for pass/fail rows, writes no raw prompt/tool-output/response text, reads no final answers or target/proof labels, enables no local accepts, and cannot count CPU savings until a profile plus admission/shadow proves false_accepts=0.".to_owned(),
        next_engineering_debt: "Compile a disabled-threshold test_output_parse .nwrb profile from the scoreable tool-state payload trace, run shadow scoring and admission audit, then count unique verified CPU accepts over exact cache only if verifier support remains clean.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-test-output-parse-tool-state-payload-v1: {}",
        report.verdict
    );
    println!("  tool_state_report: {}", tool_state_report_path.display());
    println!("  registry_config: {}", registry_config_path.display());
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!("  operator_candidate_calls: {}", operator_candidate_calls);
    println!("  payload_ready_events: {}", payload_ready_events);
    println!("  payload_built_events: {}", payload_built_events);
    println!("  scoreable_payload_events: {}", scoreable_payload_events);
    println!("  profile_registered: {}", profile_registered);
    println!("  local_accepts_enabled: false");
    Err(
        "test-output-parse tool-state payload dry-run is review-only; build profile+admission before claims"
            .to_owned(),
    )
}

pub(crate) fn run_role_binding_real_traffic_test_output_parse_profile_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let base_registry_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let dry_run_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_TOOL_STATE_PAYLOAD_TRACE_JSONL));
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_PACKAGE_PATH));
    let registry_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_PROFILE_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_PROFILE_REPORT));

    let mut registry = read_json_file::<RoleBindingProfileRegistryConfig>(&base_registry_path)?;
    validate_registry_config(&registry)?;
    let trace_rows = read_real_traffic_trace_jsonl(&dry_run_trace_path)?;
    let build = build_test_output_parse_role_binding_package_from_trace(&trace_rows)?;
    if let Some(parent) = package_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create test-output-parse package directory {}: {error}",
                parent.display()
            )
        })?;
    }
    fs::write(&package_path, &build.package_bytes).map_err(|error| {
        format!(
            "failed to write test-output-parse package {}: {error}",
            package_path.display()
        )
    })?;
    let package_info =
        WavePredictorRoleBindingOffloadRuntime::inspect_package_bytes(&build.package_bytes)
            .map_err(|error| format!("failed to inspect test-output-parse package: {error:?}"))?;
    let policy = WavePredictorRoleBindingOffloadPolicy::new(
        REAL_TRAFFIC_TEST_OUTPUT_PARSE_DISABLED_THRESHOLD,
    )
    .map_err(|error| format!("invalid test-output-parse disabled policy: {error:?}"))?;
    let sdk = WavePredictorRoleBindingOffloadRuntime::from_package_bytes_serving_packed_only(
        &build.package_bytes,
        policy,
    )
    .map_err(|error| format!("failed to load test-output-parse package: {error:?}"))?;

    let requests = test_output_parse_scoreable_requests(&trace_rows);
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
            REAL_TRAFFIC_TEST_OUTPUT_PARSE_DISABLED_THRESHOLD,
        ));
        energy_margins.push(energy_margin);
        min_slot_margins.push(min_slot_margin);
    }

    let profile = RoleBindingProfileConfig {
        profile_id: REAL_TRAFFIC_TEST_OUTPUT_PARSE_PROFILE_ID.to_owned(),
        profile_kind: "role_binding_nwrb".to_owned(),
        operator_classes: vec![
            "test_output_parse".to_owned(),
            "previous_tool_output_state".to_owned(),
            "command_status_readout".to_owned(),
        ],
        package_path: package_path.clone(),
        runtime_bytes_estimate: sdk.bytes_estimate(),
        edge_count: package_info.edge_count,
        slot_count: 3,
        threshold: REAL_TRAFFIC_TEST_OUTPUT_PARSE_DISABLED_THRESHOLD,
        acceptance_policy: default_profile_acceptance_policy(),
        accepted_route_keys: vec![
            REAL_TRAFFIC_TEST_OUTPUT_PARSE_ROUTE_KEY.to_owned(),
            REAL_TRAFFIC_TEST_OUTPUT_PARSE_PROFILE_ID.to_owned(),
            "test_output_parse_tool_state_payload_builder_v1".to_owned(),
        ],
    };
    registry
        .profiles
        .retain(|existing| existing.profile_id != profile.profile_id);
    registry.profiles.push(profile);
    registry.claim_boundary = "serving registry overlay for test_output_parse .nwrb profile; generated from previous tool-output state payloads with threshold=i32::MAX so scoring telemetry is available but local accepts remain disabled until verifier/admission proves false_accepts=0".to_owned();
    validate_registry_config(&registry)?;
    write_json_file(&registry_path, &registry)?;

    let mut sorted_energy = energy_margins.clone();
    let mut sorted_min_slot = min_slot_margins.clone();
    sorted_energy.sort_unstable();
    sorted_min_slot.sort_unstable();
    let report = RoleBindingAnswerEvidenceProfileReport {
        schema_version: "nando_role_binding_test_output_parse_profile_v1".to_owned(),
        verdict: if unexpected_local_accepts_under_disabled_threshold == 0
            && build.package_training_requests > 0
            && package_info.edge_count > 0
        {
            "TEST_OUTPUT_PARSE_PROFILE_V1_REVIEW_PROFILE_READY_ACCEPTS_DISABLED"
        } else {
            "TEST_OUTPUT_PARSE_PROFILE_V1_REVIEW_REPAIR_REQUIRED"
        }
        .to_owned(),
        base_registry_path: base_registry_path.display().to_string(),
        dry_run_trace_path: dry_run_trace_path.display().to_string(),
        package_path: package_path.display().to_string(),
        registry_path: registry_path.display().to_string(),
        profile_id: REAL_TRAFFIC_TEST_OUTPUT_PARSE_PROFILE_ID.to_owned(),
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: build.package_bytes.len(),
        edge_count: package_info.edge_count,
        runtime_bytes_estimate: sdk.bytes_estimate(),
        threshold: REAL_TRAFFIC_TEST_OUTPUT_PARSE_DISABLED_THRESHOLD,
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
        local_accepts_enabled_on_real_traffic: false,
        market_claim_allowed: false,
        claim_boundary: "Profile generator only. It compiles previous-tool-output-state test_output_parse payload geometry into a .nwrb package and registry overlay with threshold=i32::MAX, so shadow can measure score/margins but cannot local-accept. Verified CPU savings require verifier/admission calibration, shadow/audit pass, provider cost where relevant, and false_accepts=0 over exact cache.".to_owned(),
        next_engineering_debt: "Run real-traffic shadow with this overlay registry and the tool-state payload trace, then admission audit. Do not lower thresholds or count savings from profile scoring alone.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-test-output-parse-profile-v1: {}",
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
    println!("  local_accepts_enabled_on_real_traffic: false");
    Err("test-output-parse profile is review-only; admission stays disabled".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_test_output_parse_safe_policy_promote_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let base_registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_PROFILE_REGISTRY_CONFIG));
    let tool_state_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_TOOL_STATE_PAYLOAD_TRACE_JSONL));
    let promoted_registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_SAFE_POLICY_REGISTRY_CONFIG));
    let promoted_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_SAFE_POLICY_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_SAFE_POLICY_REPORT));
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
    let base_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path(&base_registry_config_path)?;
    let mut trace_rows = read_real_traffic_trace_jsonl(&tool_state_trace_path)?;

    let mut candidate_profile_ids = BTreeSet::new();
    let mut score_rows = Vec::<(usize, RoleBindingProfileDetailedScore)>::new();
    let mut scoreable_candidate_calls = 0usize;
    let mut request_side_policy_evaluated_rows = 0usize;
    let mut request_side_policy_accept_rows = 0usize;
    let mut request_side_policy_reject_rows = 0usize;
    let mut no_score_rows = 0usize;

    for (row_index, row) in trace_rows.iter().enumerate() {
        let Some(request) = &row.nando_shadow_request else {
            continue;
        };
        let is_test_output_parse = request
            .profile_id
            .as_deref()
            .is_some_and(|profile| profile == REAL_TRAFFIC_TEST_OUTPUT_PARSE_PROFILE_ID)
            || request
                .route_key
                .as_deref()
                .is_some_and(|route| route == REAL_TRAFFIC_TEST_OUTPUT_PARSE_ROUTE_KEY);
        if !is_test_output_parse {
            continue;
        }
        scoreable_candidate_calls +=
            usize::from(!request.active_fringe.is_empty() && !request.slots.is_empty());
        request_side_policy_evaluated_rows += 1;
        let status_known = test_output_parse_trace_known_status(row).is_some();
        let Some(score) = score_role_binding_profile_request_detailed(&base_registry, request)
        else {
            no_score_rows += 1;
            continue;
        };
        if status_known && score.slot_margins.iter().all(|margin| *margin > 0) {
            request_side_policy_accept_rows += 1;
            if let Some(profile_id) = &request.profile_id {
                candidate_profile_ids.insert(profile_id.clone());
            }
            score_rows.push((row_index, score));
        } else {
            request_side_policy_reject_rows += 1;
        }
    }

    if score_rows.is_empty() {
        return Err(
            "test-output-parse safe-policy promotion found no known-status scoreable rows"
                .to_owned(),
        );
    }
    let threshold = score_rows
        .iter()
        .map(|(_, score)| score.energy_margin)
        .min()
        .unwrap_or(0);
    if threshold <= 0 {
        return Err(format!(
            "test-output-parse safe-policy threshold is not positive: {threshold}"
        ));
    }
    let acceptance_policy = default_profile_acceptance_policy();
    let mut promoted_profile_ids = Vec::new();
    for profile in &mut promoted_config.profiles {
        if candidate_profile_ids.contains(&profile.profile_id) {
            profile.threshold = threshold;
            profile.acceptance_policy = acceptance_policy.clone();
            promoted_profile_ids.push(profile.profile_id.clone());
        }
    }
    if promoted_profile_ids.is_empty() {
        return Err(format!(
            "test-output-parse safe-policy promotion found no matching profiles in registry for {:?}",
            candidate_profile_ids
        ));
    }
    promoted_config.claim_boundary = "Serving registry overlay for test_output_parse safe-policy shadow. Runtime acceptance uses only request-side previous-tool-output-state payload score and strict_ordered_energy_threshold; verifier labels are trace audit evidence only.".to_owned();
    validate_registry_config(&promoted_config)?;
    write_json_file(&promoted_registry_config_path, &promoted_config)?;
    let promoted_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path(&promoted_registry_config_path)?;

    let accepted_row_indices = score_rows
        .iter()
        .map(|(row_index, _)| *row_index)
        .collect::<HashSet<_>>();
    let mut policy_accept_rows = 0usize;
    let mut policy_accept_verified_true_rows = 0usize;
    let mut policy_accept_verified_false_rows = 0usize;
    let mut policy_accept_unverified_rows = 0usize;
    let mut provider_cost_events_written = 0usize;
    let mut runtime_acceptance_mismatches = 0usize;

    for (row_index, row) in trace_rows.iter_mut().enumerate() {
        let Some(request) = &mut row.nando_shadow_request else {
            continue;
        };
        let is_test_output_parse = request
            .profile_id
            .as_deref()
            .is_some_and(|profile| profile == REAL_TRAFFIC_TEST_OUTPUT_PARSE_PROFILE_ID)
            || request
                .route_key
                .as_deref()
                .is_some_and(|route| route == REAL_TRAFFIC_TEST_OUTPUT_PARSE_ROUTE_KEY);
        if !is_test_output_parse {
            continue;
        }
        if accepted_row_indices.contains(&row_index) {
            policy_accept_rows += 1;
            request.expect_local_operator = Some(true);
            row.verified_safe_accept = Some(true);
            row.provider_cost_microusd = Some(provider_cost_microusd);
            row.verification_source = Some(
                "request_time_previous_tool_output_state_deterministic_status_verifier_v1"
                    .to_owned(),
            );
            provider_cost_events_written += 1;
            let runtime_response = score_role_binding_profile_request(&promoted_registry, request);
            runtime_acceptance_mismatches += usize::from(!runtime_response.accepted);
            policy_accept_verified_true_rows += usize::from(row.verified_safe_accept == Some(true));
            policy_accept_verified_false_rows +=
                usize::from(row.verified_safe_accept == Some(false));
            policy_accept_unverified_rows += usize::from(row.verified_safe_accept.is_none());
            row.notes = Some(append_trace_note(
                row.notes.as_deref(),
                &format!(
                    "test_output_parse_safe_policy_promote_v1 policy={} threshold={} provider_cost_estimate_microusd={} policy_accept=true verifier=request_time_previous_tool_output_state",
                    acceptance_policy, threshold, provider_cost_microusd
                ),
            ));
        } else {
            row.nando_shadow_request = None;
            row.provider_cost_microusd = None;
            row.verified_safe_accept = None;
            row.notes = Some(append_trace_note(
                row.notes.as_deref(),
                &format!(
                    "test_output_parse_safe_policy_promote_v1 policy={} threshold={} policy_accept=false",
                    acceptance_policy, threshold
                ),
            ));
        }
    }

    write_real_traffic_trace_jsonl(&promoted_trace_path, &trace_rows)?;
    let report = RoleBindingMixedSafePolicyPromoteReport {
        schema_version: "nando_role_binding_test_output_parse_safe_policy_promote_v1".to_owned(),
        verdict: if policy_accept_rows > 0
            && policy_accept_verified_false_rows == 0
            && policy_accept_unverified_rows == 0
            && runtime_acceptance_mismatches == 0
        {
            "TEST_OUTPUT_PARSE_SAFE_POLICY_PROMOTE_V1_REVIEW_PROMOTED_TRACE_READY"
        } else {
            "TEST_OUTPUT_PARSE_SAFE_POLICY_PROMOTE_V1_REVIEW_REQUIRES_SHADOW_AUDIT"
        }
        .to_owned(),
        base_registry_config_path: base_registry_config_path.display().to_string(),
        evidence_trace_path: tool_state_trace_path.display().to_string(),
        calibration_report_path: DEFAULT_TEST_OUTPUT_PARSE_PROFILE_REPORT.to_owned(),
        promoted_registry_config_path: promoted_registry_config_path.display().to_string(),
        promoted_trace_path: promoted_trace_path.display().to_string(),
        history_path: None,
        request_side_policy_name: Some(
            "known_previous_tool_output_status_and_positive_score".to_owned(),
        ),
        calibration_policy_name: "profile_min_energy_margin".to_owned(),
        calibration_policy_threshold: Some(threshold),
        selected_policy_name: "test_output_parse_known_status_strict_energy".to_owned(),
        selected_policy_source:
            "request_time_previous_tool_output_state_plus_profile_min_energy_margin".to_owned(),
        selected_policy_threshold: threshold,
        selected_acceptance_policy: acceptance_policy,
        selected_policy_accepts: policy_accept_rows,
        selected_policy_true_accepts: policy_accept_verified_true_rows,
        selected_policy_false_accepts: policy_accept_verified_false_rows,
        selected_policy_unverified_accepts: policy_accept_unverified_rows,
        promoted_profile_ids,
        provider_cost_microusd,
        trace_rows_written: trace_rows.len(),
        scoreable_candidate_calls,
        request_side_policy_evaluated_rows,
        request_side_policy_accept_rows,
        request_side_policy_reject_rows,
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
        claim_boundary: format!(
            "Promotion artifact only. It creates a promoted test_output_parse registry and rewrites the tool-state trace for rows where previous tool output already produced a deterministic known status. Runtime acceptance uses only request-side payload score under {} with threshold {}; verified_safe_accept is offline audit evidence, not runtime authority. This does not prove a market claim until shadow plus verification-hook audit pass and CPU feedback attribution over exact cache.",
            "strict_ordered_energy_threshold",
            threshold
        ),
        next_engineering_debt: "Run role-binding-real-traffic-shadow-v1 and verification-hook-audit-v1 on the promoted registry/trace, then feed that audit into CPU route feedback. Keep test_output_parse bounded to request-time previous tool-output status parsing; do not widen it to broad answer/explain.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-test-output-parse-safe-policy-promote-v1: {}",
        report.verdict
    );
    println!(
        "  promoted_registry: {}",
        promoted_registry_config_path.display()
    );
    println!("  promoted_trace: {}", promoted_trace_path.display());
    println!("  report: {}", report_path.display());
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
        "  runtime_acceptance_mismatches: {}",
        report.runtime_acceptance_mismatches
    );
    Err(
        "test-output-parse safe-policy promotion is review-only; run shadow/audit before claims"
            .to_owned(),
    )
}

pub(crate) fn run_role_binding_real_traffic_test_output_parse_safe_policy_window_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let base_window_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_HISTORY_ROUTE_CANDIDATES_TRACE_JSONL));
    let promoted_route_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_SAFE_POLICY_TRACE_JSONL));
    let output_window_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_SAFE_POLICY_WINDOW_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_OUTPUT_PARSE_SAFE_POLICY_WINDOW_REPORT));

    let base_rows = read_real_traffic_trace_jsonl(&base_window_trace_path)?;
    let promoted_rows = read_real_traffic_trace_jsonl(&promoted_route_trace_path)?;
    let promoted_by_request_fingerprint = promoted_rows
        .iter()
        .filter(|row| row.nando_shadow_request.is_some())
        .filter(|row| row.verified_safe_accept == Some(true))
        .filter_map(|row| {
            row.request_fingerprint
                .as_ref()
                .map(|fingerprint| (fingerprint.clone(), row.clone()))
        })
        .collect::<BTreeMap<_, _>>();

    let mut output_rows = Vec::with_capacity(base_rows.len());
    let mut promoted_rows_inserted = 0usize;
    let mut forced_fallback_rows = 0usize;
    let mut missing_base_match_rows = 0usize;
    let mut exact_cache_overlap_promoted_rows = 0usize;
    let mut seen_base_fingerprints = BTreeSet::new();
    let mut exact_cache_seen = HashSet::new();

    for base_row in base_rows {
        let request_fingerprint = base_row.request_fingerprint.clone().unwrap_or_default();
        seen_base_fingerprints.insert(request_fingerprint.clone());
        if let Some(promoted) = promoted_by_request_fingerprint.get(&request_fingerprint) {
            let mut promoted = promoted.clone();
            if promoted.exact_cache_key.is_none() {
                promoted.exact_cache_key = base_row.exact_cache_key.clone();
            }
            if promoted.time_ms.is_none() {
                promoted.time_ms = base_row.time_ms;
            }
            promoted.traffic_source =
                Some("codex_history_test_output_parse_safe_policy_full_window".to_owned());
            let exact_cache_hit = promoted
                .exact_cache_key
                .as_ref()
                .map(|key| !exact_cache_seen.insert(key.clone()))
                .unwrap_or(false);
            exact_cache_overlap_promoted_rows += usize::from(exact_cache_hit);
            promoted_rows_inserted += 1;
            output_rows.push(promoted);
        } else {
            let mut fallback = base_row;
            if let Some(key) = &fallback.exact_cache_key {
                exact_cache_seen.insert(key.clone());
            }
            fallback.nando_shadow_request = None;
            fallback.verified_safe_accept = None;
            fallback.provider_cost_microusd = None;
            fallback.notes = Some(append_trace_note(
                fallback.notes.as_deref(),
                "test_output_parse_safe_policy_window_v1 forced_fallback_non_test_output_parse",
            ));
            forced_fallback_rows += 1;
            output_rows.push(fallback);
        }
    }

    for fingerprint in promoted_by_request_fingerprint.keys() {
        missing_base_match_rows += usize::from(!seen_base_fingerprints.contains(fingerprint));
    }

    write_real_traffic_trace_jsonl(&output_window_trace_path, &output_rows)?;
    let report = RoleBindingTestOutputParseSafePolicyWindowReport {
        schema_version: "nando_role_binding_test_output_parse_safe_policy_window_v1".to_owned(),
        verdict: if promoted_rows_inserted > 0 && missing_base_match_rows == 0 {
            "TEST_OUTPUT_PARSE_SAFE_POLICY_WINDOW_V1_REVIEW_FULL_WINDOW_READY"
        } else {
            "TEST_OUTPUT_PARSE_SAFE_POLICY_WINDOW_V1_REVIEW_MISSING_BASE_MATCHES"
        }
        .to_owned(),
        base_window_trace_path: base_window_trace_path.display().to_string(),
        promoted_route_trace_path: promoted_route_trace_path.display().to_string(),
        output_window_trace_path: output_window_trace_path.display().to_string(),
        base_window_rows: output_rows.len(),
        promoted_route_rows: promoted_rows.len(),
        promoted_rows_inserted,
        forced_fallback_rows,
        missing_base_match_rows,
        exact_cache_overlap_promoted_rows,
        single_route_shadow_ready: true,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        target_labels_used_for_runtime: false,
        proof_labels_used_for_runtime: false,
        local_accepts_enabled_by_trace_rewrite_only: true,
        market_claim_allowed: false,
        claim_boundary: "Full-window trace builder only. It preserves the 1000-call denominator from the Codex route-candidate trace, injects only verified test_output_parse safe-policy rows by request fingerprint, and clears all other nando_shadow_request values so feedback attribution remains single-route. It writes no raw prompt/response text and proves no claim until shadow plus verification-hook audit pass.".to_owned(),
        next_engineering_debt: "Run role-binding-real-traffic-shadow-v1 and verification-hook-audit-v1 on this full-window isolated trace, then feed the resulting 1000-call audit into CPU feedback unique attribution.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-test-output-parse-safe-policy-window-v1: {}",
        report.verdict
    );
    println!("  output_trace: {}", output_window_trace_path.display());
    println!("  report: {}", report_path.display());
    println!("  base_window_rows: {}", report.base_window_rows);
    println!(
        "  promoted_rows_inserted: {}",
        report.promoted_rows_inserted
    );
    println!("  forced_fallback_rows: {}", report.forced_fallback_rows);
    println!(
        "  missing_base_match_rows: {}",
        report.missing_base_match_rows
    );
    Err("test-output-parse full-window trace is review-only; run shadow/audit".to_owned())
}
