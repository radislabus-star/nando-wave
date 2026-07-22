//! Cold collection evidence, source-neutral grouping, and package compilation.

use super::*;

pub(super) fn dedupe_relation_frames(
    frames: Vec<RelationFrame>,
) -> (Vec<RelationFrame>, usize, usize) {
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

pub(super) fn read_json_lines<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>, String> {
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

pub(super) fn is_grounded_package(package: &ResponsePackage) -> bool {
    package.package_id.starts_with("raw-phase-grounded-")
}

pub(super) fn promotion_debt(package: &&ResponsePackage) -> usize {
    32_usize.saturating_sub(package.proof.support_rows)
        + 32_usize.saturating_sub(package.proof.future_rows)
        + 3_usize.saturating_sub(package.proof.distinct_sessions)
        + 2_usize.saturating_sub(package.proof.distinct_surfaces)
}

pub(super) fn candidate_progress(package: Option<&ResponsePackage>) -> Value {
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

pub(super) const fn verifier_coverage_state(required: usize, emitted: usize) -> &'static str {
    if required == 0 {
        "NOT_EVALUATED"
    } else if emitted >= required {
        "COMPLETE"
    } else {
        "PARTIAL"
    }
}

pub(super) const fn package_state_name(state: ResponsePackageState) -> &'static str {
    match state {
        ResponsePackageState::Quarantine => "quarantine",
        ResponsePackageState::Active => "active",
        ResponsePackageState::Revoked => "revoked",
    }
}

pub(super) const fn program_operation_name(operation: &ResponseOperation) -> &'static str {
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
pub(super) struct ColdCollectionEvidence {
    pub(super) schema: String,
    pub(super) provider_payload: Value,
    pub(super) expected_response: String,
}

#[derive(Clone, Debug)]
pub(super) struct ColdCollectionRow {
    pub(super) frame_id_sha256: String,
    pub(super) session_id_sha256: String,
    pub(super) client_intent_id_sha256: String,
    pub(super) observed_at_unix_nanos: u64,
    pub(super) surface_sha256: String,
    pub(super) phase_valid: bool,
    pub(super) request_phase_atom_ids: Vec<u64>,
    pub(super) example: CollectionSynthesisExample,
}

pub(super) fn cold_collection_rows(rows: &[Value]) -> Vec<ColdCollectionRow> {
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

pub(super) fn read_relation_frame_input(
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

pub(super) fn collection_surface_digest(payload: &Value) -> Option<String> {
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

pub(super) fn collection_families(rows: &[ColdCollectionRow]) -> Vec<Vec<ColdCollectionRow>> {
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

pub(super) fn split_collection_bucket_by_behavior(
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

pub(super) fn collection_candidate_covers(
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

pub(super) fn value_shape(value: &Value) -> String {
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

pub(super) fn compile_collection_quarantine_package(
    rows: &[ColdCollectionRow],
) -> Option<ResponsePackage> {
    compile_collection_package(rows, None)
}

pub(super) fn compile_collection_package(
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

pub(super) fn build_collection_support_manifest(
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
