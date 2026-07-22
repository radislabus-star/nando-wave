//! Support manifests, anti-centers, evidence accounting, and progress reports.

use super::*;

pub(super) fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
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
