//! Canonical evidence normalization and bounded bucket storage.

use super::*;

fn normalize_online_completion_state(frame: &mut RelationFrame) {
    let continuation_pending = frame.atoms.iter().any(|atom| {
        matches!(
            atom,
            RelationAtom::ObservationSelector {
                selector: crate::ResponseValueSelector::ContentLinePrefix { prefix, .. },
                ..
            } if prefix == "Script running with cell ID "
                || prefix == "Process running with session ID "
        )
    });
    if continuation_pending {
        for atom in &mut frame.atoms {
            if let RelationAtom::CompletionState { value } = atom {
                *value = "pending".to_owned();
            }
        }
    }
}

pub(super) fn canonicalize_online_frame(frame: &mut RelationFrame) {
    normalize_online_completion_state(frame);
    reconstruct_online_client_capability(frame);
    frame.atoms.sort();
    frame.atoms.dedup();
}

pub(super) fn reconstruct_online_client_capability(frame: &mut RelationFrame) {
    if frame.atoms.iter().any(|atom| {
        matches!(
            atom,
            RelationAtom::ClientCapabilityAtom { .. }
                | RelationAtom::ReconstructedClientCapabilityAtom { .. }
        )
    }) {
        return;
    }
    let capability = frame.atoms.iter().find_map(|atom| match atom {
        RelationAtom::ActionFunction { value } => Some(crate::package::stable_atom_id(&format!(
            "client_capability:function:{value}"
        ))),
        RelationAtom::ActionCustomTool { value } => Some(crate::package::stable_atom_id(&format!(
            "client_capability:custom:{value}"
        ))),
        _ => None,
    });
    if let Some(atom_id) = capability {
        frame
            .atoms
            .push(RelationAtom::ReconstructedClientCapabilityAtom { atom_id });
    }
}

pub(super) fn response_operation_name(program: &ResponseProgram) -> &'static str {
    match &program.operation {
        crate::ResponseOperation::AdvancePlan { .. } => "advance_plan",
        crate::ResponseOperation::FunctionCallFromRoles { function_name, .. } => {
            if function_name == "write_stdin" {
                "write_stdin"
            } else {
                "function_call_from_roles"
            }
        }
        crate::ResponseOperation::CustomToolCallFromRoles {
            inner_tool_name, ..
        } => {
            if inner_tool_name == "write_stdin" {
                "write_stdin"
            } else {
                "custom_tool_call_from_roles"
            }
        }
        crate::ResponseOperation::ProjectSelectedValue { .. } => "project_selected_value",
        crate::ResponseOperation::ProjectStatus { .. } => "project_status",
        crate::ResponseOperation::ComposeCollection { .. } => "compose_collection",
        _ => "other",
    }
}

pub(super) fn push_bounded<T>(rows: &mut VecDeque<T>, row: T, limit: usize) {
    if rows.len() == limit {
        rows.pop_front();
    }
    rows.push_back(row);
}

pub(super) fn intern_bucket_evidence(
    buckets: &mut BTreeMap<u32, ResponseBucket>,
) -> Result<(), String> {
    let mut arena = BTreeMap::<(String, String), SharedRelationFrame>::new();
    for bucket in buckets.values_mut() {
        for rows in [
            &mut bucket.positives,
            &mut bucket.negatives,
            &mut bucket.future_positives,
            &mut bucket.future_negatives,
        ] {
            for frame in rows {
                let digest = crate::relation_frame_learning_digest(frame.as_frame())
                    .map_err(|error| format!("online_frame_digest:{error}"))?;
                let key = (frame.frame_id_sha256.clone(), digest);
                if let Some(shared) = arena.get(&key) {
                    *frame = shared.clone();
                } else {
                    arena.insert(key, frame.clone());
                }
            }
        }
    }
    Ok(())
}

pub(super) fn canonical_runtime_parity_key(frame: &RelationFrame) -> String {
    let digest = Sha256::digest(
        serde_json::to_vec(&(
            "nando.future-runtime-parity.v1",
            &frame.evidence_ref_sha256,
            &frame.event_id_sha256,
            &frame.session_id_sha256,
        ))
        .unwrap_or_default(),
    );
    format!("{:020}:{digest:x}", frame.observed_at_unix_nanos)
}

pub(super) fn push_session_diverse_future(
    rows: &mut VecDeque<SharedRelationFrame>,
    row: SharedRelationFrame,
    limit: usize,
) {
    if limit == 0 {
        return;
    }
    if rows.len() < limit {
        rows.push_back(row);
        return;
    }

    let incoming_session = row.session_id_sha256.as_str();
    let replacement = rows
        .iter()
        .position(|existing| existing.session_id_sha256 == incoming_session)
        .or_else(|| {
            rows.iter().enumerate().find_map(|(index, existing)| {
                let copies = rows
                    .iter()
                    .filter(|candidate| candidate.session_id_sha256 == existing.session_id_sha256)
                    .count();
                (copies > 1).then_some(index)
            })
        })
        .unwrap_or(0);
    rows.remove(replacement);
    rows.push_back(row);
}

pub(super) fn trim_session_diverse_future(rows: &mut VecDeque<SharedRelationFrame>, limit: usize) {
    if rows.len() <= limit {
        return;
    }
    let mut bounded = VecDeque::with_capacity(limit);
    for row in std::mem::take(rows) {
        push_session_diverse_future(&mut bounded, row, limit);
    }
    *rows = bounded;
}

pub(super) fn update_cardinality_bounds(bucket: &mut ResponseBucket, frame: &RelationFrame) {
    for atom in &frame.atoms {
        let RelationAtom::Cardinality { role, count } = atom else {
            continue;
        };
        bucket
            .positive_cardinality_bounds
            .entry(role.clone())
            .and_modify(|(minimum, maximum)| {
                *minimum = (*minimum).min(*count);
                *maximum = (*maximum).max(*count);
            })
            .or_insert((*count, *count));
    }
    if bucket.positive_cardinality_signatures.len() < 64
        && let Some(signature) = cardinality_signature(frame)
    {
        bucket.positive_cardinality_signatures.insert(signature);
    }
}

pub(super) fn cardinality_guard_matches(bucket: &ResponseBucket, frame: &RelationFrame) -> bool {
    if !bucket.exact_guard_atom_ids.is_empty() {
        let observed = exact_guard_atom_ids(frame);
        if !bucket
            .exact_guard_atom_ids
            .iter()
            .all(|required| observed.binary_search(required).is_ok())
        {
            return false;
        }
    }
    if bucket.positive_cardinality_bounds.is_empty() {
        return true;
    }
    if !bucket.positive_cardinality_signatures.is_empty()
        && cardinality_signature(frame)
            .is_none_or(|signature| !bucket.positive_cardinality_signatures.contains(&signature))
    {
        return false;
    }
    let observed = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::Cardinality { role, count } => Some((role.as_str(), *count)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    bucket
        .positive_cardinality_bounds
        .iter()
        .all(|(role, (minimum, maximum))| {
            observed
                .get(role.as_str())
                .is_some_and(|count| count >= minimum && count <= maximum)
        })
}

pub(super) fn update_exact_guard(bucket: &mut ResponseBucket, frame: &RelationFrame) {
    let observed = exact_guard_atom_ids(frame);
    if bucket.positives.is_empty() {
        bucket.exact_guard_atom_ids = observed;
    } else {
        bucket
            .exact_guard_atom_ids
            .retain(|required| observed.binary_search(required).is_ok());
    }
}

pub(super) fn recompute_exact_guard(bucket: &mut ResponseBucket) {
    let mut positives = bucket
        .positives
        .iter()
        .chain(bucket.future_positives.iter());
    let Some(first) = positives.next() else {
        bucket.exact_guard_atom_ids.clear();
        return;
    };
    let mut required = exact_guard_atom_ids(first);
    for frame in positives {
        let observed = exact_guard_atom_ids(frame);
        required.retain(|atom| observed.binary_search(atom).is_ok());
    }
    bucket.exact_guard_atom_ids = required;
}

fn exact_guard_atom_ids(frame: &RelationFrame) -> Vec<u64> {
    let filtered = RelationFrame {
        atoms: frame
            .atoms
            .iter()
            .filter(|atom| !matches!(atom, RelationAtom::Cardinality { .. }))
            .cloned()
            .collect(),
        ..frame.clone()
    };
    relation_frame_online_routing_atom_ids(&filtered)
}

fn cardinality_signature(frame: &RelationFrame) -> Option<String> {
    let mut parts = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::Cardinality { role, count } => Some(format!("{role}={count}")),
            _ => None,
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }
    parts.sort();
    Some(parts.join("|"))
}
