use std::collections::BTreeSet;
use std::path::PathBuf;

use nando_operator_kernel::{
    AtomSource, RelationAtom, ResponseOperation, ResponseValueSelector, canonical_json_sha256,
};
use nando_operator_learning::multi_source::factor_multi_source_row_v1;

use super::*;
use crate::multi_source_frame_archive::MultiSourceFrameArchive;
use crate::multi_source_topology_archive::MultiSourceTopologyArchive;

#[test]
#[ignore = "requires a disposable copy of live multi-source archives"]
fn dump_live_candidate_role_witness_hashes() {
    let root = PathBuf::from(
        std::env::var("NANDO_K1_LIVE_FIXTURE").expect("NANDO_K1_LIVE_FIXTURE directory"),
    );
    let candidate_root =
        std::env::var("NANDO_K1_CANDIDATE_ROOT").expect("NANDO_K1_CANDIDATE_ROOT");
    let topology_archive = MultiSourceTopologyArchive::open(
        &root.join("pre-action-topology-archive-v1"),
    )
    .expect("topology archive");
    let frame_archive =
        MultiSourceFrameArchive::open(&root.join("relation-frame-archive-v1"))
            .expect("frame archive");
    let topologies = topology_archive.shared_rows();
    let frames = frame_archive.shared_frames();
    let mut accumulator = EvidenceBindingAccumulator::new(true);
    let join_report = stream_multi_source_joins_from_iter(
        topologies.iter().map(|row| row.as_ref()),
        frames.iter().map(|frame| frame.as_ref()),
        |joined| accumulator.push(joined),
    )
    .expect("stream joins");
    let prepared = prepare_tick_context_from_bindings(
        join_report,
        accumulator.finish().expect("bindings"),
        &BTreeSet::new(),
        true,
    )
    .expect("prepared context");
    let candidate = prepared
        .catalog
        .candidates
        .iter()
        .find(|candidate| candidate.candidate_root_sha256 == candidate_root)
        .expect("candidate");
    let support = prepared
        .bindings
        .iter()
        .filter(|binding| {
            let row = &binding.row;
            row.candidate_structural_root_sha256 == candidate.candidate_structural_root_sha256
                && row.source_neutral_topology_root_sha256
                    == candidate.source_neutral_topology_root_sha256
                && row.semantic_novelty_signature_root_sha256
                    == candidate.semantic_novelty_signature_root_sha256
                && row.consequence_type == candidate.consequence_type
                && row.capture_generation_root_sha256
                    == candidate.capture_generation_root_sha256
                && row.capture_sequence <= candidate.last_capture_sequence
        })
        .collect::<Vec<_>>();
    assert_eq!(support.len(), candidate.evidence_rows as usize);

    let rows = support
        .iter()
        .map(|binding| {
            let joined = binding.joined();
            let frame = frames
                .iter()
                .find(|frame| {
                    canonical_json_sha256(frame.as_ref()).is_ok_and(|root| {
                        root == binding.completed_frame_root_sha256
                    })
                })
                .expect("completed frame");
            let selected_slots = frame
                .atoms
                .iter()
                .filter_map(|atom| match atom {
                    RelationAtom::ObservationSelector { slot_id, selector } => {
                        Some((*slot_id, selector))
                    }
                    _ => None,
                })
                .collect::<std::collections::BTreeMap<_, _>>();
            let observations = frame
                .atoms
                .iter()
                .filter_map(|atom| match atom {
                    RelationAtom::TypedSlot {
                        slot_id,
                        value_type,
                        source: AtomSource::Observation,
                        value_sha256,
                    } if selected_slots.contains_key(slot_id) => Some(serde_json::json!({
                        "slot_id": slot_id,
                        "value_type": value_type,
                        "value_sha256": value_sha256,
                        "selector": selector_summary(selected_slots[slot_id]),
                        "selector_root_sha256": canonical_json_sha256(selected_slots[slot_id])
                            .expect("selector root"),
                        "exact_witness_roles": joined
                            .topology
                            .role_witnesses
                            .iter()
                            .filter(|witness| witness.value_sha256 == *value_sha256)
                            .map(|witness| witness.local_role_id)
                            .collect::<Vec<_>>()
                    })),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let factorized = factor_multi_source_row_v1(joined);
            let physical_programs = nando_operator_learning::synthesis::
                enumerate_response_program_candidates(std::slice::from_ref(frame));
            let role_types = joined.topology.roles.iter().fold(
                std::collections::BTreeMap::<String, usize>::new(),
                |mut counts, role| {
                    *counts.entry(format!("{:?}", role.type_class)).or_default() += 1;
                    counts
                },
            );
            let role_containers = joined.topology.roles.iter().fold(
                std::collections::BTreeMap::<String, usize>::new(),
                |mut counts, role| {
                    *counts
                        .entry(format!("{:?}", role.container_class))
                        .or_default() += 1;
                    counts
                },
            );
            serde_json::json!({
                "capture_sequence": binding.row.capture_sequence,
                "evidence_root_sha256": binding.row.evidence_root_sha256,
                "topology_commitment_root_sha256": binding.topology_commitment_root_sha256,
                "completed_frame_root_sha256": binding.completed_frame_root_sha256,
                "pre_action_shape": factorized.pre_action_shape,
                "completed_effect": factorized.completed_effect,
                "roles": joined.topology.roles.len(),
                "role_witnesses": joined.topology.role_witnesses.len(),
                "role_types": role_types,
                "role_containers": role_containers,
                "physical_programs": physical_programs
                    .iter()
                    .map(|program| operation_summary(&program.operation))
                    .collect::<Vec<_>>(),
                "selected_observations": observations
            })
        })
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "candidate": candidate,
            "support_rows": rows
        }))
        .expect("diagnostic json")
    );
}

fn selector_kind(selector: &ResponseValueSelector) -> &'static str {
    match selector {
        ResponseValueSelector::ContinuationHandle { .. } => "continuation_handle",
        ResponseValueSelector::UniqueScalar { .. } => "unique_scalar",
        ResponseValueSelector::UniqueTurnScalar { .. } => "unique_turn_scalar",
        ResponseValueSelector::ContentLinePrefix { .. } => "content_line_prefix",
        ResponseValueSelector::JsonField { .. } => "json_field",
        ResponseValueSelector::JsonScalarOrdinal { .. } => "json_scalar_ordinal",
        ResponseValueSelector::UniqueTurnJsonField { .. } => "unique_turn_json_field",
        ResponseValueSelector::UniqueActiveTurnJsonField { .. } => {
            "unique_active_turn_json_field"
        }
        ResponseValueSelector::RequestReferencedJsonField { .. } => {
            "request_referenced_json_field"
        }
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal { .. } => {
            "request_referenced_json_field_ordinal"
        }
        ResponseValueSelector::TurnOutputLine { .. } => "turn_output_line",
        ResponseValueSelector::TurnOutputScalarOrdinal { .. } => "turn_output_scalar_ordinal",
        ResponseValueSelector::LatestTurnOutputLine { .. } => "latest_turn_output_line",
        ResponseValueSelector::LatestTurnOutputScalarOrdinal { .. } => {
            "latest_turn_output_scalar_ordinal"
        }
        ResponseValueSelector::LatestTurnOutputScalarFromEnd { .. } => {
            "latest_turn_output_scalar_from_end"
        }
        ResponseValueSelector::CommandOutputBody => "command_output_body",
        ResponseValueSelector::RequestLastToken => "request_last_token",
        ResponseValueSelector::RequestUniqueLiteral => "request_unique_literal",
    }
}

fn selector_summary(selector: &ResponseValueSelector) -> serde_json::Value {
    match selector {
        ResponseValueSelector::JsonField { field, .. }
        | ResponseValueSelector::UniqueTurnJsonField { field, .. }
        | ResponseValueSelector::UniqueActiveTurnJsonField { field, .. } => serde_json::json!({
            "kind": selector_kind(selector),
            "field": field,
        }),
        _ => serde_json::json!({"kind": selector_kind(selector)}),
    }
}

fn operation_summary(operation: &ResponseOperation) -> serde_json::Value {
    match operation {
        ResponseOperation::ProjectSelectedValue { selector, .. } => serde_json::json!({
            "op": "project_selected_value",
            "selector_kind": selector_kind(selector)
        }),
        ResponseOperation::ProjectStatus {
            selector, mapping, ..
        } => serde_json::json!({
            "op": "project_status",
            "selector_kind": selector_kind(selector),
            "mapping": mapping
        }),
        ResponseOperation::ComposeCollection { steps, .. } => serde_json::json!({
            "op": "compose_collection",
            "steps": steps.len()
        }),
        ResponseOperation::FunctionCallFromRoles { selector, .. } => serde_json::json!({
            "op": "function_call_from_roles",
            "selector_kind": selector_kind(selector)
        }),
        ResponseOperation::CustomToolCallFromRoles { selector, .. } => serde_json::json!({
            "op": "custom_tool_call_from_roles",
            "selector": selector_summary(selector)
        }),
        ResponseOperation::UniqueConsensus { variants, .. } => serde_json::json!({
            "op": "unique_consensus",
            "variants": variants.len()
        }),
        ResponseOperation::AdvancePlan { .. } => serde_json::json!({"op": "advance_plan"}),
        ResponseOperation::CopyAfterPrefix { .. } => {
            serde_json::json!({"op": "copy_after_prefix"})
        }
        ResponseOperation::TestResultSummary { .. } => {
            serde_json::json!({"op": "test_result_summary"})
        }
        ResponseOperation::WaitOnYieldedCell { .. } => {
            serde_json::json!({"op": "wait_on_yielded_cell"})
        }
        ResponseOperation::WaitOnAnyYieldedCell { .. } => {
            serde_json::json!({"op": "wait_on_any_yielded_cell"})
        }
        ResponseOperation::WaitOnYieldedSurfaces { .. } => {
            serde_json::json!({"op": "wait_on_yielded_surfaces"})
        }
    }
}
