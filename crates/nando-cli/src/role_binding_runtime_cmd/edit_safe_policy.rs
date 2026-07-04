// Included from ../role_binding_runtime_cmd.rs to keep private runtime types in one
// Rust module while cutting the edit-route admission/safe-policy commands
// out of the monolithic runtime command file.

pub(crate) fn run_role_binding_real_traffic_edit_safe_policy_promote_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let base_registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let evidence_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_OUTPUT_EVIDENCE_TRACE_JSONL));
    let calibration_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_LOCAL_ACCEPT_CALIBRATION_REPORT));
    let promoted_registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_SAFE_POLICY_REGISTRY_CONFIG));
    let promoted_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_SAFE_POLICY_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_SAFE_POLICY_REPORT));
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
    let Some(calibration_policy) = select_supported_mixed_safe_policy(&calibration) else {
        return Err(
            "edit calibration report has no supported safe policy candidate for runtime promotion"
                .to_owned(),
        );
    };
    let base_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path(&base_registry_config_path)?;
    let policy = select_mixed_promotion_policy_from_evidence(
        &base_registry,
        &trace_rows,
        calibration_policy,
        "edit_marker_length",
    )?;
    let threshold = policy.threshold;
    let acceptance_policy = "energy_threshold_only".to_owned();
    let edit_profile_ids = trace_rows
        .iter()
        .filter_map(|row| row.nando_shadow_request.as_ref())
        .filter(|request| {
            request
                .route_key
                .as_deref()
                .is_some_and(|route| route.contains("edit_marker_length"))
        })
        .filter_map(|request| request.profile_id.clone())
        .collect::<BTreeSet<_>>();
    if edit_profile_ids.is_empty() {
        return Err("edit safe policy promotion found no edit profile ids in trace".to_owned());
    }
    let mut promoted_profile_ids = Vec::new();
    for profile in &mut promoted_config.profiles {
        if edit_profile_ids.contains(&profile.profile_id) {
            profile.threshold = threshold;
            profile.acceptance_policy = acceptance_policy.clone();
            promoted_profile_ids.push(profile.profile_id.clone());
        }
    }
    if promoted_profile_ids.is_empty() {
        return Err(format!(
            "edit safe policy promotion found no matching profiles in registry for {:?}",
            edit_profile_ids
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
        let is_edit = request
            .route_key
            .as_deref()
            .is_some_and(|route| route.contains("edit_marker_length"));
        if !is_edit {
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
        let policy_accept = profile_accepts_score(
            &acceptance_policy,
            strict_ordered_pass,
            score.energy_margin,
            score.slot_margins.first().copied().unwrap_or(0),
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
            "{}; edit_safe_policy_promote_v1 policy={} threshold={} provider_cost_estimate_microusd={} policy_accept={}",
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
        schema_version: "nando_role_binding_edit_safe_policy_promote_v1".to_owned(),
        verdict: if policy_accept_rows > 0
            && policy_accept_verified_false_rows == 0
            && policy_accept_unverified_rows == 0
            && runtime_acceptance_mismatches == 0
        {
            "EDIT_SAFE_POLICY_PROMOTE_V1_REVIEW_PROMOTED_TRACE_READY"
        } else {
            "EDIT_SAFE_POLICY_PROMOTE_V1_REVIEW_REQUIRES_SHADOW_AUDIT"
        }
        .to_owned(),
        base_registry_config_path: base_registry_config_path.display().to_string(),
        evidence_trace_path: evidence_trace_path.display().to_string(),
        calibration_report_path: calibration_report_path.display().to_string(),
        promoted_registry_config_path: promoted_registry_config_path.display().to_string(),
        promoted_trace_path: promoted_trace_path.display().to_string(),
        history_path: None,
        request_side_policy_name: None,
        calibration_policy_name: calibration_policy.policy_name.clone(),
        calibration_policy_threshold: calibration_policy.threshold,
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
        claim_boundary: "Promotion artifact only. It creates a promoted serving registry with an explicit edit-route acceptance policy and rewrites a shadow trace with provider-cost estimates. Offline labels/evidence may choose the threshold, but serving uses only request-side score >= threshold. It does not prove market savings until role-binding-real-traffic-shadow-v1 and verification-hook audit pass with false_accepts=0 and unverified_shadow_accepts=0.".to_owned(),
        next_engineering_debt: "Run role-binding-real-traffic-shadow-v1 and verification-hook-audit-v1 on the promoted edit registry/trace. Only a shadow PASS with provider cost, non-synthetic rows, and false_accepts=0 can advance verified CPU routability.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-edit-safe-policy-promote-v1: {}",
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
    Err("edit safe policy promotion is review-only; run shadow/audit before claims".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_edit_safe_policy_promote_v2<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let base_registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let evidence_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_OUTPUT_EVIDENCE_TRACE_JSONL));
    let admission_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_ADMISSION_CALIBRATION_REPORT));
    let promoted_registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_SAFE_POLICY_V2_REGISTRY_CONFIG));
    let promoted_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_SAFE_POLICY_V2_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_SAFE_POLICY_V2_REPORT));
    let provider_cost_microusd = args
        .next()
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| format!("invalid provider_cost_microusd '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(100);
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));

    let mut promoted_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&base_registry_config_path)?;
    validate_registry_config(&promoted_config)?;
    let admission_report =
        read_json_file::<RoleBindingEditAdmissionCalibrationReport>(&admission_report_path)?;
    let mut trace_rows = read_real_traffic_trace_jsonl(&evidence_trace_path)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let history_by_fingerprint = history_rows
        .iter()
        .map(|row| {
            (
                format!(
                    "fnv1a64:{:016x}",
                    stable_real_traffic_fingerprint64(row.text.as_bytes())
                ),
                row.text.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let Some(admission_policy) = select_supported_edit_admission_policy(&admission_report) else {
        return Err(
            "edit admission report has no robust request-side policy candidate for v2 promotion"
                .to_owned(),
        );
    };

    let base_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path(&base_registry_config_path)?;
    let policy = select_edit_admission_promotion_policy_from_evidence(
        &base_registry,
        &trace_rows,
        &history_by_fingerprint,
        admission_policy,
    )?;
    let threshold = policy.threshold;
    let acceptance_policy = "energy_threshold_only".to_owned();

    let edit_profile_ids = trace_rows
        .iter()
        .filter_map(|row| row.nando_shadow_request.as_ref())
        .filter(|request| {
            request
                .route_key
                .as_deref()
                .is_some_and(|route| route.contains("edit_marker_length"))
        })
        .filter_map(|request| request.profile_id.clone())
        .collect::<BTreeSet<_>>();
    if edit_profile_ids.is_empty() {
        return Err("edit safe policy v2 promotion found no edit profile ids in trace".to_owned());
    }
    let mut promoted_profile_ids = Vec::new();
    for profile in &mut promoted_config.profiles {
        if edit_profile_ids.contains(&profile.profile_id) {
            profile.threshold = threshold;
            profile.acceptance_policy = acceptance_policy.clone();
            promoted_profile_ids.push(profile.profile_id.clone());
        }
    }
    if promoted_profile_ids.is_empty() {
        return Err(format!(
            "edit safe policy v2 promotion found no matching profiles in registry for {:?}",
            edit_profile_ids
        ));
    }
    validate_registry_config(&promoted_config)?;
    write_json_file(&promoted_registry_config_path, &promoted_config)?;
    let promoted_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path(&promoted_registry_config_path)?;

    let mut scoreable_candidate_calls = 0usize;
    let mut request_side_policy_evaluated_rows = 0usize;
    let mut request_side_policy_accept_rows = 0usize;
    let mut request_side_policy_reject_rows = 0usize;
    let mut history_prompt_missing_rows = 0usize;
    let mut policy_accept_rows = 0usize;
    let mut policy_accept_verified_true_rows = 0usize;
    let mut policy_accept_verified_false_rows = 0usize;
    let mut policy_accept_unverified_rows = 0usize;
    let mut provider_cost_events_written = 0usize;
    let mut runtime_acceptance_mismatches = 0usize;
    let mut no_score_rows = 0usize;

    for row in &mut trace_rows {
        let is_edit = row
            .nando_shadow_request
            .as_ref()
            .and_then(|request| request.route_key.as_deref())
            .is_some_and(|route| route.contains("edit_marker_length"));
        if !is_edit {
            continue;
        }
        let scoreable = row
            .nando_shadow_request
            .as_ref()
            .is_some_and(|request| !request.active_fringe.is_empty() && !request.slots.is_empty());
        scoreable_candidate_calls += usize::from(scoreable);

        let request_fingerprint = row.request_fingerprint.clone().unwrap_or_default();
        let Some(prompt_text) = history_by_fingerprint.get(&request_fingerprint) else {
            history_prompt_missing_rows += 1;
            row.nando_shadow_request = None;
            row.provider_cost_microusd = None;
            row.verified_safe_accept = None;
            row.notes = Some(format!(
                "{}; edit_safe_policy_promote_v2 request_policy={} policy_accept=false reason=history_prompt_missing",
                row.notes
                    .clone()
                    .unwrap_or_else(|| "real_codex_trace".to_owned()),
                admission_policy.policy_name
            ));
            continue;
        };
        request_side_policy_evaluated_rows += 1;
        let features = extract_edit_admission_features(prompt_text);
        let request_policy_accept =
            edit_admission_policy_accepts(&admission_policy.policy_name, &features)
                .unwrap_or(false);
        if !request_policy_accept {
            request_side_policy_reject_rows += 1;
            row.nando_shadow_request = None;
            row.provider_cost_microusd = None;
            row.verified_safe_accept = None;
            row.notes = Some(format!(
                "{}; edit_safe_policy_promote_v2 request_policy={} provider_cost_estimate_microusd={} policy_accept=false",
                row.notes
                    .clone()
                    .unwrap_or_else(|| "real_codex_trace".to_owned()),
                admission_policy.policy_name,
                provider_cost_microusd
            ));
            continue;
        }

        request_side_policy_accept_rows += 1;
        row.provider_cost_microusd = Some(provider_cost_microusd);
        provider_cost_events_written += 1;
        let Some(request) = &mut row.nando_shadow_request else {
            no_score_rows += 1;
            continue;
        };
        let Some(score) = score_role_binding_profile_request_detailed(&promoted_registry, request)
        else {
            no_score_rows += 1;
            request.expect_local_operator = Some(false);
            continue;
        };
        let strict_ordered_pass = score.slot_margins.iter().all(|margin| *margin > 0);
        let policy_accept = profile_accepts_score(
            &acceptance_policy,
            strict_ordered_pass,
            score.energy_margin,
            score.slot_margins.first().copied().unwrap_or(0),
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
            "{}; edit_safe_policy_promote_v2 request_policy={} runtime_policy={} threshold={} provider_cost_estimate_microusd={} policy_accept={}",
            row.notes
                .clone()
                .unwrap_or_else(|| "real_codex_trace".to_owned()),
            admission_policy.policy_name,
            acceptance_policy,
            threshold,
            provider_cost_microusd,
            policy_accept
        ));
    }

    write_real_traffic_trace_jsonl(&promoted_trace_path, &trace_rows)?;
    let report = RoleBindingMixedSafePolicyPromoteReport {
        schema_version: "nando_role_binding_edit_safe_policy_promote_v2".to_owned(),
        verdict: if policy_accept_rows > 0
            && policy_accept_verified_false_rows == 0
            && policy_accept_unverified_rows == 0
            && runtime_acceptance_mismatches == 0
        {
            "EDIT_SAFE_POLICY_PROMOTE_V2_REVIEW_PROMOTED_TRACE_READY"
        } else {
            "EDIT_SAFE_POLICY_PROMOTE_V2_REVIEW_REQUIRES_SHADOW_AUDIT"
        }
        .to_owned(),
        base_registry_config_path: base_registry_config_path.display().to_string(),
        evidence_trace_path: evidence_trace_path.display().to_string(),
        calibration_report_path: admission_report_path.display().to_string(),
        promoted_registry_config_path: promoted_registry_config_path.display().to_string(),
        promoted_trace_path: promoted_trace_path.display().to_string(),
        history_path: Some(history_path.display().to_string()),
        request_side_policy_name: Some(admission_policy.policy_name.clone()),
        calibration_policy_name: "edit_admission_robust_request_side_policy_candidate".to_owned(),
        calibration_policy_threshold: None,
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
        request_side_policy_evaluated_rows,
        request_side_policy_accept_rows,
        request_side_policy_reject_rows,
        history_prompt_missing_rows,
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
        claim_boundary: "Promotion artifact only. It creates a promoted edit-route registry from a robust request-side admission candidate and rewrites a full-window shadow trace. Runtime uses prompt-derived feature admission plus score >= threshold; verifier labels choose/evaluate the policy but are not runtime authority. It does not prove market savings until role-binding-real-traffic-shadow-v1 and verification-hook audit pass with false_accepts=0 and unverified_shadow_accepts=0.".to_owned(),
        next_engineering_debt: "Run role-binding-real-traffic-shadow-v1 and verification-hook-audit-v1 on the v2 promoted edit registry/trace, then feed that audit into CPU route feedback and catalog. Keep market claims disabled unless the full CPU80 window reports verified non-synthetic accepts.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-edit-safe-policy-promote-v2: {}",
        report.verdict
    );
    println!(
        "  promoted_registry: {}",
        promoted_registry_config_path.display()
    );
    println!("  promoted_trace: {}", promoted_trace_path.display());
    println!("  report: {}", report_path.display());
    println!(
        "  request_side_policy_name: {}",
        admission_policy.policy_name
    );
    println!(
        "  selected_policy_threshold: {}",
        report.selected_policy_threshold
    );
    println!(
        "  request_side_policy_accept_rows: {}",
        report.request_side_policy_accept_rows
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
    Err("edit safe policy v2 promotion is review-only; run shadow/audit before claims".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_edit_admission_calibration_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let evidence_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_OUTPUT_EVIDENCE_TRACE_JSONL));
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_ADMISSION_CALIBRATION_REPORT));

    let trace_rows = read_real_traffic_trace_jsonl(&evidence_trace_path)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let history_by_fingerprint = history_rows
        .iter()
        .map(|row| {
            (
                format!(
                    "fnv1a64:{:016x}",
                    stable_real_traffic_fingerprint64(row.text.as_bytes())
                ),
                row.text.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut rows = Vec::new();
    let mut hook_ready_rows = 0usize;
    let mut label_true_rows = 0usize;
    let mut label_false_rows = 0usize;
    let mut history_prompt_missing_rows = 0usize;

    for trace in &trace_rows {
        let Some(label) = trace.verified_safe_accept else {
            continue;
        };
        if trace.nando_shadow_request.is_none() {
            continue;
        }
        hook_ready_rows += 1;
        label_true_rows += usize::from(label);
        label_false_rows += usize::from(!label);
        let request_fingerprint = trace.request_fingerprint.clone().unwrap_or_default();
        let Some(prompt_text) = history_by_fingerprint.get(&request_fingerprint) else {
            history_prompt_missing_rows += 1;
            continue;
        };
        let features = extract_edit_admission_features(prompt_text);
        rows.push(RoleBindingEditAdmissionCalibrationRow {
            trace_id: trace.trace_id.clone(),
            request_fingerprint: trace.request_fingerprint.clone(),
            response_fingerprint: trace.response_fingerprint.clone(),
            verifier_label: label,
            features,
        });
    }

    let minimum_true_support = 2usize;
    let policies = edit_admission_policy_reports(&rows, minimum_true_support);
    let robust_safe_policy_found = policies.iter().any(|policy| policy.robust_safe);
    let singleton_safe_policy_found = policies.iter().any(|policy| policy.singleton_safe);
    let best_robust_true_accepts = policies
        .iter()
        .filter(|policy| policy.robust_safe)
        .map(|policy| policy.true_accepts)
        .max()
        .unwrap_or(0);
    let best_singleton_true_accepts = policies
        .iter()
        .filter(|policy| policy.singleton_safe)
        .map(|policy| policy.true_accepts)
        .max()
        .unwrap_or(0);
    let feature_counts = edit_admission_feature_counts(&rows);
    let report = RoleBindingEditAdmissionCalibrationReport {
        schema_version: "nando_role_binding_edit_admission_calibration_v1".to_owned(),
        verdict: if robust_safe_policy_found {
            "EDIT_ADMISSION_CALIBRATION_V1_REVIEW_ROBUST_POLICY_CANDIDATE_FOUND"
        } else if singleton_safe_policy_found {
            "EDIT_ADMISSION_CALIBRATION_V1_REVIEW_SINGLETON_ONLY_NO_ROBUST_POLICY"
        } else {
            "EDIT_ADMISSION_CALIBRATION_V1_REVIEW_NO_SAFE_POLICY"
        }
        .to_owned(),
        evidence_trace_path: evidence_trace_path.display().to_string(),
        history_path: history_path.display().to_string(),
        hook_ready_rows,
        rows_with_prompt_features: rows.len(),
        history_prompt_missing_rows,
        label_true_rows,
        label_false_rows,
        minimum_true_support,
        robust_safe_policy_found,
        singleton_safe_policy_found,
        best_robust_true_accepts,
        best_singleton_true_accepts,
        feature_counts,
        policies,
        rows,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        response_text_used_for_features: false,
        target_labels_used_for_runtime: false,
        proof_labels_used_for_runtime: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Admission calibration only. It reads real request text at analysis time, writes only fingerprints/features/counts, uses verification labels only to evaluate candidate request-side gates, enables no local accepts, and cannot be used as market savings claim.".to_owned(),
        next_engineering_debt: if robust_safe_policy_found {
            "Promote the robust admission candidate only through a separate shadow trace rewrite and false_accepts=0 gate; do not count singleton policies as product proof.".to_owned()
        } else {
            "Current real edit request-side features do not provide a robust safe admission gate. Leave edit local accepts disabled and either improve edit features with more real evidence or build the conditional/mixed payload builders.".to_owned()
        },
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-edit-admission-calibration-v1: {}",
        report.verdict
    );
    println!("  evidence_trace: {}", evidence_trace_path.display());
    println!("  history: {}", history_path.display());
    println!("  report: {}", report_path.display());
    println!("  hook_ready_rows: {}", report.hook_ready_rows);
    println!(
        "  rows_with_prompt_features: {}",
        report.rows_with_prompt_features
    );
    println!("  label_true_rows: {}", report.label_true_rows);
    println!("  label_false_rows: {}", report.label_false_rows);
    println!(
        "  robust_safe_policy_found: {}",
        report.robust_safe_policy_found
    );
    println!(
        "  singleton_safe_policy_found: {}",
        report.singleton_safe_policy_found
    );
    println!(
        "  best_robust_true_accepts: {}",
        report.best_robust_true_accepts
    );
    Err("edit admission calibration is review-only".to_owned())
}
