use std::collections::BTreeSet;
use std::path::PathBuf;

use nando_operator_kernel::{AtomSource, RelationAtom, canonical_json_sha256};

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
                    RelationAtom::ObservationSelector { slot_id, .. } => Some(*slot_id),
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            let observations = frame
                .atoms
                .iter()
                .filter_map(|atom| match atom {
                    RelationAtom::TypedSlot {
                        slot_id,
                        value_type,
                        source: AtomSource::Observation,
                        value_sha256,
                    } if selected_slots.contains(slot_id) => Some(serde_json::json!({
                        "slot_id": slot_id,
                        "value_type": value_type,
                        "value_sha256": value_sha256,
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
            serde_json::json!({
                "capture_sequence": binding.row.capture_sequence,
                "evidence_root_sha256": binding.row.evidence_root_sha256,
                "topology_commitment_root_sha256": binding.topology_commitment_root_sha256,
                "completed_frame_root_sha256": binding.completed_frame_root_sha256,
                "roles": joined.topology.roles,
                "role_witnesses": joined.topology.role_witnesses,
                "relations": joined.topology.relations,
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
