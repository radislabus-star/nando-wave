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
    let targets = package_proof_targets(package);
    targets.0.saturating_sub(package.proof.support_rows)
        + targets.1.saturating_sub(package.proof.future_rows)
        + targets.2.saturating_sub(package.proof.distinct_sessions)
        + targets.3.saturating_sub(package.proof.distinct_surfaces)
}

pub(super) fn candidate_progress(package: Option<&ResponsePackage>) -> Value {
    package.map_or(Value::Null, |package| {
        let targets = package_proof_targets(package);
        let mut admission_candidate = package.clone();
        admission_candidate.state = ResponsePackageState::Active;
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
            "proof_mode": if package.proof.adaptive_identification.is_some() {
                "adaptive_identification"
            } else {
                "legacy_control"
            },
            "admission_blocker": admission_candidate.admission_candidate_blocker(),
            "support_gap": targets.0.saturating_sub(package.proof.support_rows),
            "future_gap": targets.1.saturating_sub(package.proof.future_rows),
            "session_gap": targets.2.saturating_sub(package.proof.distinct_sessions),
            "surface_gap": targets.3.saturating_sub(package.proof.distinct_surfaces),
        })
    })
}

fn package_proof_targets(package: &ResponsePackage) -> (usize, usize, usize, usize) {
    if package
        .proof
        .adaptive_identification
        .as_ref()
        .is_some_and(|proof| proof.validate().is_ok())
    {
        (1, 1, 2, 2)
    } else {
        (
            LEGACY_CONTROL_SUPPORT_ROWS,
            LEGACY_CONTROL_FUTURE_ROWS,
            LEGACY_CONTROL_MIN_SESSIONS,
            LEGACY_CONTROL_MIN_SURFACES,
        )
    }
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
        let Some(frame_id) = row.get("frame_id_sha256").and_then(Value::as_str) else {
            continue;
        };
        output.push(ColdCollectionRow {
            frame_id_sha256: frame_id.to_owned(),
            example: CollectionSynthesisExample {
                provider_payload: cold.provider_payload,
                expected_response: cold.expected_response,
            },
        });
    }
    output.sort_by(|left, right| left.frame_id_sha256.cmp(&right.frame_id_sha256));
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
