//! Read-only S1A transition projection and S1B decision-surface census.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use nando_operator_admission::{OperatorCertificationEntryV1, OperatorCertificationLedgerV1};
use nando_operator_kernel::{RelationFrame, canonical_json_sha256, valid_nonzero_sha256};
use nando_operator_learning::multi_source::{
    PreActionTopologyAuditRowV1, TransportBindingFailureV1, TransportBindingLedgerV1,
};
use nando_operator_learning::{
    DecisionEvidenceSurfaceV1, GroundedDecisionCensusV1, GroundedEvidenceClassV1,
    GroundedTransitionEpisodeV1, GroundedTransitionMaterialV1,
    GroundedTransitionProjectionSnapshotV1, TransitionProjectionCensorReasonV1,
    TransitionTerminalDispositionV1, build_grounded_decision_census_v1, read_framed_cbor,
    write_atomic_cbor,
};
use serde::{Deserialize, Serialize};

use crate::ServingConfig;
use crate::live_economics::PackageCpuCompletionReceiptV1;
use crate::operator_certification::{CertificationAuthorityConfigV1, restore_anchored_ledger};
use crate::terminal_receipt_archive::read_terminal_receipts_for_requests;

const PROJECTION_FILE: &str = "grounded-transition-projection-v1.cbor";
const CENSUS_FILE: &str = "grounded-decision-census-v1.json";
const COMPLETION_PREFIX: &str = "completion";
const TOPOLOGY_PREFIX: &str = "multi-source-topology";
const FRAME_PREFIX: &str = "multi-source-frame";

#[derive(Clone, Debug)]
pub struct GroundedDecisionCensusConfigV1 {
    pub economics_snapshot_path: PathBuf,
    pub topology_archive_path: PathBuf,
    pub frame_archive_path: PathBuf,
    pub terminal_archive_path: PathBuf,
    pub output_directory: PathBuf,
    pub certification: CertificationAuthorityConfigV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GroundedDecisionCensusRunV1 {
    pub projection: GroundedTransitionProjectionSnapshotV1,
    pub census: GroundedDecisionCensusV1,
}

#[derive(Serialize)]
struct SourceSnapshotDigestV1<'a> {
    schema: &'static str,
    economics_snapshot_root_sha256: &'a str,
    certification_ledger_root_sha256: &'a str,
    completion_receipt_roots_sha256: &'a [String],
    topology_commitment_roots_sha256: &'a [String],
    completed_frame_roots_sha256: &'a [String],
    terminal_receipt_roots_sha256: &'a [String],
}

impl GroundedDecisionCensusConfigV1 {
    #[must_use]
    pub fn from_serving_config(config: &ServingConfig) -> Self {
        let state_dir = config
            .economics_path
            .parent()
            .unwrap_or_else(|| Path::new("/var/lib/nando-wave/transition"));
        Self {
            economics_snapshot_path: config.ms4_ordinary_economics_path.clone(),
            topology_archive_path: config.multi_source_topology_archive_path.clone(),
            frame_archive_path: config.multi_source_frame_archive_path.clone(),
            terminal_archive_path: config.terminal_receipt_archive_path.clone(),
            output_directory: state_dir.join("grounded-meaning-v1"),
            certification: CertificationAuthorityConfigV1::from_serving_config(config),
        }
    }
}

pub fn run_grounded_decision_census_v1(
    config: &GroundedDecisionCensusConfigV1,
) -> Result<GroundedDecisionCensusRunV1, String> {
    let economics_safety_root_before =
        read_clean_economics_safety_root(&config.economics_snapshot_path)?;
    let certification_before = restore_anchored_ledger(&config.certification)?;
    let certified_packages = certified_k1_packages(&certification_before)?;
    let completion_directory = config
        .economics_snapshot_path
        .parent()
        .ok_or_else(|| "grounded_decision_economics_parent_missing".to_owned())?
        .join("package-cpu-completions-v1");
    let completions = read_framed_cbor::<PackageCpuCompletionReceiptV1>(
        &completion_directory,
        COMPLETION_PREFIX,
    )?;
    for receipt in &completions {
        receipt.validate()?;
    }

    let certified_completions = completions
        .iter()
        .filter(|receipt| certified_packages.contains_key(receipt.package_id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let certified_request_ids = certified_completions
        .iter()
        .map(|receipt| receipt.intent_sha256.clone())
        .collect::<BTreeSet<_>>();

    let topology_rows = read_framed_cbor::<PreActionTopologyAuditRowV1>(
        &config.topology_archive_path,
        TOPOLOGY_PREFIX,
    )?;
    let relevant_topologies = topology_rows
        .into_iter()
        .filter(|row| certified_request_ids.contains(&row.structure.request_event_id_sha256))
        .collect::<Vec<_>>();
    let relevant_turn_intents = relevant_topologies
        .iter()
        .map(|row| row.structure.turn_intent_id_sha256.clone())
        .collect::<BTreeSet<_>>();
    let frame_rows = read_framed_cbor::<RelationFrame>(&config.frame_archive_path, FRAME_PREFIX)?;
    let relevant_frames = frame_rows
        .into_iter()
        .filter(|frame| relevant_turn_intents.contains(&frame.client_intent_id_sha256))
        .collect::<Vec<_>>();
    let relevant_request_ids = relevant_topologies
        .iter()
        .map(|row| row.structure.request_event_id_sha256.clone())
        .collect::<BTreeSet<_>>();
    let terminals =
        read_terminal_receipts_for_requests(&config.terminal_archive_path, &relevant_request_ids)?;
    let binding_ledger =
        TransportBindingLedgerV1::build(&relevant_topologies, &relevant_frames, &terminals);

    let source_snapshot_root_sha256 = source_snapshot_root(
        &economics_safety_root_before,
        &certification_before,
        &completions,
        &relevant_topologies,
        &relevant_frames,
        &terminals,
    )?;
    let (episodes, censor_counts) = project_transitions(
        &completions,
        &certified_packages,
        &relevant_topologies,
        &binding_ledger,
        &certification_before.ledger_root_sha256,
    )?;
    let transition_rows_scanned =
        u64::try_from(completions.len()).map_err(|_| "grounded_decision_count")?;
    let certified_k1_rows =
        u64::try_from(certified_completions.len()).map_err(|_| "grounded_decision_count")?;
    let projection = GroundedTransitionProjectionSnapshotV1::seal(
        source_snapshot_root_sha256,
        transition_rows_scanned,
        certified_k1_rows,
        episodes,
        censor_counts,
    )
    .map_err(str::to_owned)?;
    let surfaces = projection
        .episodes
        .iter()
        .cloned()
        .map(DecisionEvidenceSurfaceV1::dynamics_only)
        .collect::<Vec<_>>();
    let census = build_grounded_decision_census_v1(&projection, surfaces).map_err(str::to_owned)?;

    let economics_safety_root_after =
        read_clean_economics_safety_root(&config.economics_snapshot_path)?;
    let certification_after = restore_anchored_ledger(&config.certification)?;
    if economics_safety_root_after != economics_safety_root_before
        || certification_after.ledger_root_sha256 != certification_before.ledger_root_sha256
    {
        return Err("grounded_decision_source_changed_during_scan".to_owned());
    }

    persist_run(config, &projection, &census)?;
    Ok(GroundedDecisionCensusRunV1 { projection, census })
}

fn project_transitions(
    completions: &[PackageCpuCompletionReceiptV1],
    certified_packages: &BTreeMap<&str, &OperatorCertificationEntryV1>,
    topologies: &[PreActionTopologyAuditRowV1],
    binding_ledger: &TransportBindingLedgerV1,
    certification_ledger_root_sha256: &str,
) -> Result<
    (
        Vec<GroundedTransitionEpisodeV1>,
        BTreeMap<TransitionProjectionCensorReasonV1, u64>,
    ),
    String,
> {
    let mut topologies_by_request = BTreeMap::<&str, Vec<&PreActionTopologyAuditRowV1>>::new();
    for topology in topologies {
        topologies_by_request
            .entry(topology.structure.request_event_id_sha256.as_str())
            .or_default()
            .push(topology);
    }
    let mut episodes = Vec::new();
    let mut censors = BTreeMap::new();
    for completion in completions {
        let Some(certification) = certified_packages
            .get(completion.package_id.as_str())
            .copied()
        else {
            increment_censor(
                &mut censors,
                TransitionProjectionCensorReasonV1::MissingCertifiedK1Binding,
            );
            continue;
        };
        let same_request = topologies_by_request
            .get(completion.intent_sha256.as_str())
            .map(Vec::as_slice)
            .unwrap_or_default();
        let topology = match same_request {
            [] => {
                increment_censor(
                    &mut censors,
                    TransitionProjectionCensorReasonV1::MissingPreActionTopology,
                );
                continue;
            }
            [topology] => *topology,
            _ => {
                increment_censor(
                    &mut censors,
                    TransitionProjectionCensorReasonV1::AmbiguousPreActionTopology,
                );
                continue;
            }
        };
        let bound = binding_ledger.bound_for_topology(&topology.commit.commitment_root_sha256);
        let transition = match bound {
            [] => {
                let reason = match binding_ledger
                    .failure_for_topology(&topology.commit.commitment_root_sha256)
                {
                    Some(TransportBindingFailureV1::CapacityExhausted) => {
                        TransitionProjectionCensorReasonV1::CapacityExhausted
                    }
                    Some(TransportBindingFailureV1::IdentityMismatch) => {
                        TransitionProjectionCensorReasonV1::IdentityMismatch
                    }
                    _ if binding_ledger
                        .join_rejection_for_topology(&topology.commit.commitment_root_sha256)
                        .is_some() =>
                    {
                        TransitionProjectionCensorReasonV1::IdentityMismatch
                    }
                    _ => TransitionProjectionCensorReasonV1::MissingTransportBinding,
                };
                increment_censor(&mut censors, reason);
                continue;
            }
            [transition] => transition,
            _ => {
                increment_censor(
                    &mut censors,
                    TransitionProjectionCensorReasonV1::AmbiguousTransportBinding,
                );
                continue;
            }
        };
        if !transition.joined.accepted {
            increment_censor(
                &mut censors,
                TransitionProjectionCensorReasonV1::MissingVerifiedOutcome,
            );
            continue;
        }
        if transition.binding.request_event_id_sha256 != completion.intent_sha256
            || transition.joined.topology_commitment_root_sha256
                != topology.commit.commitment_root_sha256
            || transition.joined.session_lineage_sha256 != transition.binding.session_lineage_sha256
        {
            increment_censor(
                &mut censors,
                TransitionProjectionCensorReasonV1::IdentityMismatch,
            );
            continue;
        }
        if !valid_nonzero_sha256(&transition.joined.capture_generation_root_sha256)
            || !valid_nonzero_sha256(&transition.joined.join_root_sha256)
            || !valid_nonzero_sha256(&transition.joined.verifier_receipt_root_sha256)
        {
            increment_censor(
                &mut censors,
                TransitionProjectionCensorReasonV1::ProvenanceFailure,
            );
            continue;
        }
        let episode = GroundedTransitionEpisodeV1::seal(GroundedTransitionMaterialV1 {
            evidence_class: GroundedEvidenceClassV1::Natural,
            pre_action_state_root_sha256: topology
                .structure
                .provider_capture_request_root_sha256
                .clone(),
            observed_constraint_root_sha256: None,
            grounded_role_environment_root_sha256: topology.commit.topology_root_sha256.clone(),
            k1_law_id_sha256: certification.semantic_law_id_sha256.clone(),
            bundle_id_sha256: certification.bundle_id_sha256.clone(),
            action_binding_root_sha256: transition.binding.binding_root_sha256.clone(),
            verified_delta_root_sha256: completion.completion_root_sha256.clone(),
            post_action_state_root_sha256: transition.joined.completed_frame_root_sha256.clone(),
            independent_verifier_root_sha256: completion.verification_receipt_root_sha256.clone(),
            lineage_root_sha256: transition.binding.session_lineage_sha256.clone(),
            capture_generation_root_sha256: transition
                .joined
                .capture_generation_root_sha256
                .clone(),
            disposition: TransitionTerminalDispositionV1::Positive,
            provenance_roots_sha256: vec![
                certification_ledger_root_sha256.to_owned(),
                certification.entry_root_sha256.clone(),
                completion.completion_root_sha256.clone(),
                topology.commit.commitment_root_sha256.clone(),
                transition.binding.terminal_receipt_root_sha256.clone(),
                transition.binding.binding_root_sha256.clone(),
                transition.joined.join_root_sha256.clone(),
                transition.joined.verifier_receipt_root_sha256.clone(),
            ],
        })
        .map_err(str::to_owned)?;
        episodes.push(episode);
    }
    Ok((episodes, censors))
}

fn certified_k1_packages(
    ledger: &OperatorCertificationLedgerV1,
) -> Result<BTreeMap<&str, &OperatorCertificationEntryV1>, String> {
    let packages = ledger
        .latest_entries()
        .into_iter()
        .filter(|entry| {
            entry.k1_unit_eligible
                && entry.epistemic_registry_member
                && entry.product_registry_member
                && entry.false_bad_apply == 0
        })
        .map(|entry| (entry.package_id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    if packages.is_empty() {
        return Err("grounded_decision_certified_k1_basis_empty".to_owned());
    }
    Ok(packages)
}

fn read_clean_economics_safety_root(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("grounded_decision_economics_read:{error}"))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("grounded_decision_economics_decode:{error}"))?;
    if value
        .get("false_accepts")
        .and_then(serde_json::Value::as_u64)
        != Some(0)
        || value
            .get("runtime_parity_mismatches")
            .and_then(serde_json::Value::as_u64)
            != Some(0)
        || value
            .get("pipeline_dropped")
            .and_then(serde_json::Value::as_u64)
            != Some(0)
    {
        return Err("grounded_decision_economics_not_clean".to_owned());
    }
    canonical_json_sha256(&(
        "nando.grounded-decision-economics-safety.v1",
        value.get("schema").and_then(serde_json::Value::as_str),
        0_u64,
        0_u64,
        0_u64,
    ))
    .map_err(str::to_owned)
}

fn source_snapshot_root(
    economics_snapshot_root_sha256: &str,
    certification: &OperatorCertificationLedgerV1,
    completions: &[PackageCpuCompletionReceiptV1],
    topologies: &[PreActionTopologyAuditRowV1],
    frames: &[RelationFrame],
    terminals: &[nando_operator_learning::multi_source::TransportTerminalReceiptV1],
) -> Result<String, String> {
    let mut completion_receipt_roots_sha256 = completions
        .iter()
        .map(|receipt| receipt.completion_root_sha256.clone())
        .collect::<Vec<_>>();
    let mut topology_commitment_roots_sha256 = topologies
        .iter()
        .map(|row| row.commit.commitment_root_sha256.clone())
        .collect::<Vec<_>>();
    let mut completed_frame_roots_sha256 = frames
        .iter()
        .map(canonical_json_sha256)
        .collect::<Result<Vec<_>, _>>()
        .map_err(str::to_owned)?;
    let mut terminal_receipt_roots_sha256 = terminals
        .iter()
        .map(|receipt| receipt.receipt_root_sha256.clone())
        .collect::<Vec<_>>();
    for roots in [
        &mut completion_receipt_roots_sha256,
        &mut topology_commitment_roots_sha256,
        &mut completed_frame_roots_sha256,
        &mut terminal_receipt_roots_sha256,
    ] {
        roots.sort();
        roots.dedup();
    }
    canonical_json_sha256(&SourceSnapshotDigestV1 {
        schema: "nando.grounded-decision-source-snapshot.v1",
        economics_snapshot_root_sha256,
        certification_ledger_root_sha256: &certification.ledger_root_sha256,
        completion_receipt_roots_sha256: &completion_receipt_roots_sha256,
        topology_commitment_roots_sha256: &topology_commitment_roots_sha256,
        completed_frame_roots_sha256: &completed_frame_roots_sha256,
        terminal_receipt_roots_sha256: &terminal_receipt_roots_sha256,
    })
    .map_err(str::to_owned)
}

fn persist_run(
    config: &GroundedDecisionCensusConfigV1,
    projection: &GroundedTransitionProjectionSnapshotV1,
    census: &GroundedDecisionCensusV1,
) -> Result<(), String> {
    fs::create_dir_all(&config.output_directory)
        .map_err(|error| format!("grounded_decision_output_dir:{error}"))?;
    let projection_path = config.output_directory.join(PROJECTION_FILE);
    let projection_bytes = serde_cbor::to_vec(projection)
        .map_err(|error| format!("grounded_decision_projection_encode:{error}"))?;
    let restored_projection: GroundedTransitionProjectionSnapshotV1 =
        serde_cbor::from_slice(&projection_bytes)
            .map_err(|error| format!("grounded_decision_projection_restart_decode:{error}"))?;
    restored_projection.validate().map_err(str::to_owned)?;
    if restored_projection != *projection {
        return Err("grounded_decision_projection_restart_parity".to_owned());
    }
    if fs::read(&projection_path).ok().as_deref() != Some(projection_bytes.as_slice()) {
        write_atomic_cbor(&projection_path, projection)?;
    }

    let census_path = config.output_directory.join(CENSUS_FILE);
    let mut census_bytes = serde_json::to_vec_pretty(census)
        .map_err(|error| format!("grounded_decision_census_encode:{error}"))?;
    census_bytes.push(b'\n');
    let restored_census: GroundedDecisionCensusV1 = serde_json::from_slice(&census_bytes)
        .map_err(|error| format!("grounded_decision_census_restart_decode:{error}"))?;
    restored_census.validate().map_err(str::to_owned)?;
    if restored_census != *census {
        return Err("grounded_decision_census_restart_parity".to_owned());
    }
    if fs::read(&census_path).ok().as_deref() != Some(census_bytes.as_slice()) {
        crate::write_bytes_atomic(&census_path, &census_bytes, "grounded-decision-census")?;
    }
    Ok(())
}

fn increment_censor(
    counts: &mut BTreeMap<TransitionProjectionCensorReasonV1, u64>,
    reason: TransitionProjectionCensorReasonV1,
) {
    let count = counts.entry(reason).or_default();
    *count = count.saturating_add(1);
}
