use super::*;
use crate::phase_streaming_cmd::{
    DEFAULT_CELLS, DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS,
    DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER,
    DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_PROFILES_PER_WORKER,
    DEFAULT_PHASE_CENTER_SHADOW_MAX_PROFILES_PER_ROUTE,
    DEFAULT_PHASE_CENTER_SHADOW_MAX_ROUTE_TOP_K,
    DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS, ForbiddenFlags, json_at, json_i64,
    json_string, json_string_vec, json_u64, read_json_value, stable_fingerprint, write_json_file,
};

pub(crate) fn run_phase_stream_hot_path_daemon_numeric_false_accept_split_audit_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let source_report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_HOT_PATH_DAEMON_NUMERIC_FUTURE_PACKAGE_AUDIT_REPORT)
    });
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_HOT_PATH_DAEMON_NUMERIC_FALSE_ACCEPT_SPLIT_AUDIT_REPORT)
    });
    let top_k = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid top_k value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(24)
        .max(1);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let source = read_json_value(&source_report_path)?;
    let cells = json_u64(&source, &["cells"]).unwrap_or(DEFAULT_CELLS as u64) as usize;
    let min_bucket_events = json_u64(&source, &["min_bucket_events"])
        .unwrap_or(DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS as u64)
        as usize;
    let candidate_bucket_id = json_u64(&source, &["candidate_bucket_id"])
        .ok_or_else(|| "false-accept split audit source missing candidate_bucket_id".to_owned())?
        as u32;
    let candidate_route_id = json_u64(&source, &["candidate_route_id"])
        .ok_or_else(|| "false-accept split audit source missing candidate_route_id".to_owned())?
        as u32;
    let candidate_package_path = json_string(&source, &["candidate_package_path"])
        .map(PathBuf::from)
        .ok_or_else(|| {
            "false-accept split audit source missing candidate_package_path".to_owned()
        })?;
    let threshold_micro = json_i64(&source, &["threshold_micro"])
        .ok_or_else(|| "false-accept split audit source missing threshold_micro".to_owned())?;
    let freeze_after_append_events =
        json_u64(&source, &["freeze_after_append_events"]).unwrap_or_default() as usize;
    let watermark_trace_paths = json_string_vec(json_at(&source, &["watermark_trace_paths"]))
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let append_trace_paths = json_string_vec(json_at(&source, &["append_trace_paths"]))
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if watermark_trace_paths.is_empty() || append_trace_paths.is_empty() {
        return Err("false-accept split audit needs watermark and append trace paths".to_owned());
    }

    let package_bytes = std::fs::read(&candidate_package_path).map_err(|error| {
        format!(
            "failed to read false-accept split package '{}': {error}",
            candidate_package_path.display()
        )
    })?;
    let offload_runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &package_bytes,
        PhaseCenterOffloadPolicy::new(threshold_micro)
            .map_err(|error| format!("false-accept split package policy error: {error:?}"))?,
    )
    .map_err(|error| format!("false-accept split package load error: {error:?}"))?;
    let hot_runtime = PhaseCenterHotRuntime::from_flat_runtime(
        offload_runtime.runtime(),
        &[candidate_bucket_id],
        &[threshold_micro],
    )
    .map_err(|error| format!("false-accept split hot runtime build error: {error:?}"))?;
    let route_plan = hot_runtime
        .route_plan_from_profile_ids(candidate_route_id, [candidate_bucket_id])
        .map_err(|error| format!("false-accept split route plan error: {error:?}"))?
        .ok_or_else(|| "false-accept split hot route has no profiles".to_owned())?;
    let route_table = PhaseCenterHotRouteTable::from_plans([route_plan])
        .map_err(|error| format!("false-accept split route table error: {error:?}"))?;
    let mut worker = PhaseCenterHotWorker::new(hot_runtime, route_table)
        .map_err(|error| format!("false-accept split hot worker error: {error:?}"))?;

    let mut store = PhaseCenterLiveOperatorStore::new(PhaseCenterLiveOperatorStoreConfig {
        miner: PhaseCenterOnlineMinerConfig {
            cells,
            min_bucket_events,
            threshold_floor_micro: 1,
            calibration_events: min_bucket_events,
            max_buckets: 16_384,
        },
        memory: PhaseCenterOperatorMemoryConfig {
            max_hot_profiles_per_worker: DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_PROFILES_PER_WORKER,
            max_hot_bytes_per_worker: DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER,
            max_warm_profiles_per_process:
                DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS,
            max_profiles_per_route: DEFAULT_PHASE_CENTER_SHADOW_MAX_PROFILES_PER_ROUTE,
            max_route_top_k: DEFAULT_PHASE_CENTER_SHADOW_MAX_ROUTE_TOP_K,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .map_err(|error| format!("failed to create false-accept split store: {error:?}"))?;
    let mut encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("failed to create false-accept split encoder: {error:?}"))?;
    let mut bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut watermark_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut watermark_total_rows = 0usize;
    let mut watermark_parsed_rows = 0usize;
    let mut watermark_skipped_no_verifier_label = 0usize;
    let mut watermark_skipped_no_safe_atoms = 0usize;
    for watermark_trace_path in &watermark_trace_paths {
        let file = File::open(watermark_trace_path).map_err(|error| {
            format!(
                "failed to open false-accept split watermark trace '{}': {error}",
                watermark_trace_path.display()
            )
        })?;
        live_store_observe_live_loop_budget_events(
            &watermark_trace_path.display().to_string(),
            io::BufReader::new(file),
            &mut store,
            &mut encoder,
            &mut bucket_policy,
            &mut exact_cache_keys_seen,
            &mut watermark_events,
            &mut watermark_total_rows,
            &mut watermark_parsed_rows,
            &mut watermark_skipped_no_verifier_label,
            &mut watermark_skipped_no_safe_atoms,
        )?;
    }

    let mut append_events = Vec::<LiveStoreParsedAtomEventWithAtoms>::new();
    let mut append_total_rows = 0usize;
    let mut append_parsed_rows = 0usize;
    let mut append_skipped_no_verifier_label = 0usize;
    let mut append_skipped_no_safe_atoms = 0usize;
    for append_trace_path in &append_trace_paths {
        let file = File::open(append_trace_path).map_err(|error| {
            format!(
                "failed to open false-accept split append trace '{}': {error}",
                append_trace_path.display()
            )
        })?;
        for (line_index, line) in io::BufReader::new(file).lines().enumerate() {
            let line = line.map_err(|error| {
                format!(
                    "failed to read false-accept split append trace '{}' line {}: {error}",
                    append_trace_path.display(),
                    line_index + 1
                )
            })?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            append_total_rows += 1;
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse false-accept split append trace '{}' line {}: {error}",
                    append_trace_path.display(),
                    line_index + 1
                )
            })?;
            let Some(verified_safe_accept) = row
                .get("verified_safe_accept")
                .and_then(serde_json::Value::as_bool)
            else {
                append_skipped_no_verifier_label += 1;
                continue;
            };
            let safe_atoms = live_store_safe_atoms(&row);
            if safe_atoms.is_empty() {
                append_skipped_no_safe_atoms += 1;
                continue;
            }
            let Some(event) = live_store_atom_event_from_row(
                &row,
                verified_safe_accept,
                &bucket_policy,
                &mut exact_cache_keys_seen,
            ) else {
                append_skipped_no_safe_atoms += 1;
                continue;
            };
            append_events.push(LiveStoreParsedAtomEventWithAtoms { event, safe_atoms });
            append_parsed_rows += 1;
        }
    }

    let mut eval = LiveStorePreparedHotPackEval::default();
    let mut reference_encoder = PhaseCenterAtomEncoder::new(cells).map_err(|error| {
        format!("failed to create false-accept split reference encoder: {error:?}")
    })?;
    let mut atom_accumulators = BTreeMap::<u64, LiveStoreFalseAcceptAtomAccumulator>::new();
    let mut runtime_margin_parity_checks = 0usize;
    let mut runtime_margin_parity_mismatches = 0usize;
    let mut runtime_decision_parity_mismatches = 0usize;
    let mut future_matching_bucket_events = 0usize;
    let mut latencies = Vec::new();
    let future_events = append_events
        .get(freeze_after_append_events..)
        .unwrap_or_default();
    for row in future_events {
        let event = &row.event;
        if event.bucket_id != candidate_bucket_id {
            continue;
        }
        future_matching_bucket_events += 1;
        let Some(route_index) = worker.resolve_route_index(event.route_id) else {
            runtime_decision_parity_mismatches += 1;
            continue;
        };
        let reference_vector = reference_encoder
            .encode_atom_ids(event.atom_ids.iter().copied())
            .map_err(|error| format!("false-accept split reference encode error: {error:?}"))?;
        let phase_vector = reference_vector.to_vec();
        let reference_margin_micro = offload_runtime
            .runtime()
            .score_vector_margin_micro(0, &phase_vector)
            .map_err(|error| format!("false-accept split reference score error: {error:?}"))?;
        let reference_score_candidate = reference_margin_micro >= threshold_micro;
        let prepared_row = LiveStorePreparedMemoryRow::new(
            route_index,
            event.atom_ids.clone(),
            phase_vector,
            event.hot_request_evidence(),
        );
        let started = Instant::now();
        let decisions = worker
            .score_prepared_row_with_evidence(&prepared_row, &mut eval)
            .map_err(|error| format!("false-accept split prepared score error: {error:?}"))?;
        latencies.push(started.elapsed().as_nanos());
        let mut matched_profile = false;
        for decision in decisions {
            if decision.profile_id != candidate_bucket_id {
                continue;
            }
            matched_profile = true;
            runtime_margin_parity_checks += 1;
            if decision.margin_micro != reference_margin_micro {
                runtime_margin_parity_mismatches += 1;
            }
            if decision.score_candidate != reference_score_candidate {
                runtime_decision_parity_mismatches += 1;
            }
            if !decision.score_candidate {
                continue;
            }
            for atom in &row.safe_atoms {
                let atom_id = stable_fingerprint(["live_store_atom", atom.as_str()]);
                let accumulator = atom_accumulators.entry(atom_id).or_insert_with(|| {
                    LiveStoreFalseAcceptAtomAccumulator {
                        atom_id,
                        atom: atom.clone(),
                        ..LiveStoreFalseAcceptAtomAccumulator::default()
                    }
                });
                accumulator.score_candidate_events += 1;
                if event.verified_safe_accept && !event.exact_cache_hit {
                    accumulator.unique_cpu_accepts_over_exact_cache += 1;
                    accumulator.tokens_saved =
                        accumulator.tokens_saved.saturating_add(event.tokens);
                    accumulator.cost_saved_microusd = accumulator
                        .cost_saved_microusd
                        .saturating_add(event.cost_microusd);
                }
                if !event.verified_safe_accept {
                    accumulator.false_accepts += 1;
                }
            }
        }
        if !matched_profile {
            runtime_decision_parity_mismatches += 1;
        }
    }
    latencies.sort_unstable();

    let min_clean_support = min_bucket_events.max(2);
    let mut atom_reports = atom_accumulators
        .into_values()
        .map(|accumulator| {
            let false_accept_milli = live_store_milli(
                accumulator.false_accepts as u64,
                accumulator.score_candidate_events as u64,
            );
            let refinement_blocker =
                live_store_false_accept_split_atom_refinement_blocker(&accumulator.atom);
            let clean_split_candidate = accumulator.false_accepts == 0
                && accumulator.unique_cpu_accepts_over_exact_cache > 0
                && accumulator.score_candidate_events >= min_clean_support;
            PhaseStreamHotPathDaemonNumericFalseAcceptAtomReport {
                atom_id: accumulator.atom_id,
                atom: accumulator.atom,
                score_candidate_events: accumulator.score_candidate_events,
                unique_cpu_accepts_over_exact_cache: accumulator
                    .unique_cpu_accepts_over_exact_cache,
                tokens_saved: accumulator.tokens_saved,
                cost_saved_microusd: accumulator.cost_saved_microusd,
                false_accepts: accumulator.false_accepts,
                false_accept_milli,
                clean_split_candidate,
                stageable_clean_split_candidate: clean_split_candidate
                    && refinement_blocker == "none",
                refinement_blocker,
            }
        })
        .collect::<Vec<_>>();
    let mut clean_atoms = atom_reports
        .iter()
        .filter(|atom| atom.clean_split_candidate)
        .cloned()
        .collect::<Vec<_>>();
    clean_atoms.sort_by(|left, right| {
        right
            .unique_cpu_accepts_over_exact_cache
            .cmp(&left.unique_cpu_accepts_over_exact_cache)
            .then_with(|| right.tokens_saved.cmp(&left.tokens_saved))
            .then_with(|| left.atom.cmp(&right.atom))
    });
    let clean_atom_candidate_count = clean_atoms.len();
    let best_clean_atom_unique_cpu_accepts_over_exact_cache = clean_atoms
        .first()
        .map_or(0, |atom| atom.unique_cpu_accepts_over_exact_cache);
    let best_clean_atom_tokens_saved = clean_atoms.first().map_or(0, |atom| atom.tokens_saved);
    let best_clean_atom_cost_saved_microusd = clean_atoms
        .first()
        .map_or(0, |atom| atom.cost_saved_microusd);
    let mut stageable_clean_atoms = clean_atoms
        .iter()
        .filter(|atom| atom.stageable_clean_split_candidate)
        .cloned()
        .collect::<Vec<_>>();
    let stageable_clean_atom_candidate_count = stageable_clean_atoms.len();
    let best_stageable_clean_atom_unique_cpu_accepts_over_exact_cache = stageable_clean_atoms
        .first()
        .map_or(0, |atom| atom.unique_cpu_accepts_over_exact_cache);
    let best_stageable_clean_atom_tokens_saved = stageable_clean_atoms
        .first()
        .map_or(0, |atom| atom.tokens_saved);
    let best_stageable_clean_atom_cost_saved_microusd = stageable_clean_atoms
        .first()
        .map_or(0, |atom| atom.cost_saved_microusd);
    stageable_clean_atoms.truncate(top_k);
    clean_atoms.truncate(top_k);

    atom_reports.sort_by(|left, right| {
        right
            .false_accepts
            .cmp(&left.false_accepts)
            .then_with(|| {
                right
                    .score_candidate_events
                    .cmp(&left.score_candidate_events)
            })
            .then_with(|| left.atom.cmp(&right.atom))
    });
    atom_reports.truncate(top_k);

    let p99_latency_ns = live_store_latency_percentile(&latencies, 99);
    let p99_budget_ns = 1_000;
    let split_diagnostic_ready = future_matching_bucket_events > 0
        && eval.score_candidate_events > 0
        && runtime_margin_parity_checks == eval.score_events
        && runtime_margin_parity_mismatches == 0
        && runtime_decision_parity_mismatches == 0
        && p99_latency_ns <= p99_budget_ns;
    let blocker = if split_diagnostic_ready {
        "none".to_owned()
    } else if future_matching_bucket_events == 0 {
        "false_accept_split_no_matching_future_events".to_owned()
    } else if eval.score_candidate_events == 0 {
        "false_accept_split_no_score_candidates".to_owned()
    } else if runtime_margin_parity_checks != eval.score_events
        || runtime_margin_parity_mismatches != 0
        || runtime_decision_parity_mismatches != 0
    {
        "false_accept_split_runtime_parity_failed".to_owned()
    } else if p99_latency_ns > p99_budget_ns {
        "false_accept_split_hot_path_p99_budget_exceeded".to_owned()
    } else {
        "false_accept_split_audit_failed".to_owned()
    };

    let report = PhaseStreamHotPathDaemonNumericFalseAcceptSplitAuditReport {
        report_kind: "phase_stream_hot_path_daemon_numeric_false_accept_split_audit_v1",
        mode: "cold_false_accept_atom_split_diagnostic_no_runtime_mutation",
        source_future_audit_report_path: source_report_path.display().to_string(),
        candidate_package_path: candidate_package_path.display().to_string(),
        append_trace_paths: append_trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        cells,
        min_bucket_events,
        top_k,
        append_total_rows,
        append_parsed_rows,
        append_skipped_no_verifier_label,
        append_skipped_no_safe_atoms,
        candidate_route_id,
        candidate_bucket_id,
        threshold_micro,
        freeze_after_append_events,
        future_events_after_freeze: future_events.len(),
        future_matching_bucket_events,
        score_events: eval.score_events,
        score_candidate_events: eval.score_candidate_events,
        unique_cpu_accepts_over_exact_cache: eval.unique_cpu_accepts_over_exact_cache,
        tokens_saved: eval.tokens_saved,
        cost_saved_microusd: eval.cost_saved_microusd,
        false_accepts: eval.false_accepts,
        clean_atom_candidate_count,
        stageable_clean_atom_candidate_count,
        best_clean_atom_unique_cpu_accepts_over_exact_cache,
        best_clean_atom_tokens_saved,
        best_clean_atom_cost_saved_microusd,
        best_stageable_clean_atom_unique_cpu_accepts_over_exact_cache,
        best_stageable_clean_atom_tokens_saved,
        best_stageable_clean_atom_cost_saved_microusd,
        top_clean_atoms: clean_atoms,
        top_stageable_clean_atoms: stageable_clean_atoms,
        top_false_atoms: atom_reports,
        runtime_margin_parity_checks,
        runtime_margin_parity_mismatches,
        runtime_decision_parity_mismatches,
        p50_latency_ns: live_store_latency_percentile(&latencies, 50),
        p90_latency_ns: live_store_latency_percentile(&latencies, 90),
        p99_latency_ns,
        max_latency_ns: latencies.last().copied().unwrap_or(0),
        p99_budget_ns,
        split_diagnostic_ready,
        product_promotion_enabled: false,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        verdict: if split_diagnostic_ready {
            "HOT_PATH_DAEMON_NUMERIC_FALSE_ACCEPT_SPLIT_AUDIT_PASS"
        } else {
            "HOT_PATH_DAEMON_NUMERIC_FALSE_ACCEPT_SPLIT_AUDIT_WATCH"
        },
        blocker,
        boundary: "false-accept split audit only: reloads an existing fresh-future .nwpc candidate, scores future matching rows, and ranks observable atom ids that separate clean score-candidate subsets from false accepts; it does not tune thresholds, compile packages, promote, write serving profiles, enable local_accept, allow market claims, or use legacy nwrb/role-binding paths",
    };
    write_json_file(&report_path, &report)?;
    println!("phase_stream_hot_path_daemon_numeric_false_accept_split_audit_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  score_candidate_events: {}",
        report.score_candidate_events
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!(
        "  clean_atom_candidate_count: {}",
        report.clean_atom_candidate_count
    );
    println!(
        "  stageable_clean_atom_candidate_count: {}",
        report.stageable_clean_atom_candidate_count
    );
    println!(
        "  best_clean_atom_unique_cpu_accepts_over_exact_cache: {}",
        report.best_clean_atom_unique_cpu_accepts_over_exact_cache
    );
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}
