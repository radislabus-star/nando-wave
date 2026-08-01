//! Idempotent cold-path actuator from MS3 future PASS to ordinary CPU proof.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use nando_operator_kernel::canonical_json_sha256;
use nando_operator_learning::multi_source::{
    Ms3FutureApplicabilityDispositionV1, Ms3IndependentFutureVerdictV1, TransportBindingLedgerV1,
};
use nando_response_actor::{
    Ms4ExactPackageWaveProofV1, Ms4ExternalAdmissionCandidateV1, ResponsePackageState,
    ResponseRegistry,
};
use serde::{Deserialize, Serialize};

use crate::live_economics::{PackageCpuCompletionReceiptV1, first_durable_package_completion};
use crate::{AppState, bounded_reason, unix_now, write_bytes_atomic};

const REPORT_SCHEMA_V1: &str = "nando.ms4-autonomous-closed-loop-report.v1";
const REPORT_SCHEMA_V2: &str = "nando.ms4-autonomous-closed-loop-report.v2";

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Ms4ClosedLoopStageV1 {
    #[default]
    WaitingForMs3,
    WaitingForRuntimeEvidence,
    WaitingForNegativeControl,
    CandidateSealed,
    ExternalAdmissionPending,
    OrdinaryCpuPending,
    Complete,
    Blocked,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Ms4ClosedLoopReportV1 {
    pub schema: String,
    pub report_root_sha256: String,
    pub generated_at_unix: u64,
    pub generation_sequence: u64,
    pub stage: Ms4ClosedLoopStageV1,
    pub blocker: String,
    pub frozen_envelope_root_sha256: Option<String>,
    pub future_envelope_root_sha256: Option<String>,
    pub candidate_root_sha256: Option<String>,
    pub package_id: Option<String>,
    #[serde(default)]
    pub exact_package_wave_proof_root_sha256: Option<String>,
    pub negative_controls: u64,
    pub external_admission_pass: bool,
    pub ordinary_cpu_receipt_root_sha256: Option<String>,
    #[serde(default)]
    pub ordinary_cpu_completion_root_sha256: Option<String>,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

impl Default for Ms4ClosedLoopReportV1 {
    fn default() -> Self {
        Self::seal(0, Ms4ClosedLoopStageV1::WaitingForMs3, "ms3_future_pending")
    }
}

impl Ms4ClosedLoopReportV1 {
    fn seal(generation_sequence: u64, stage: Ms4ClosedLoopStageV1, blocker: &str) -> Self {
        let mut report = Self {
            schema: REPORT_SCHEMA_V2.to_owned(),
            report_root_sha256: String::new(),
            generated_at_unix: unix_now(),
            generation_sequence,
            stage,
            blocker: blocker.to_owned(),
            frozen_envelope_root_sha256: None,
            future_envelope_root_sha256: None,
            candidate_root_sha256: None,
            package_id: None,
            exact_package_wave_proof_root_sha256: None,
            negative_controls: 0,
            external_admission_pass: false,
            ordinary_cpu_receipt_root_sha256: None,
            ordinary_cpu_completion_root_sha256: None,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        report.reseal();
        report
    }

    fn reseal(&mut self) {
        self.report_root_sha256 = self.expected_root().unwrap_or_default();
    }

    fn validate(&self) -> Result<(), String> {
        if !matches!(self.schema.as_str(), REPORT_SCHEMA_V1 | REPORT_SCHEMA_V2)
            || self.phase_mutation_allowed
            || self.report_root_sha256 != self.expected_root()?
        {
            return Err("ms4_report_invalid".to_owned());
        }
        if self.schema == REPORT_SCHEMA_V2
            && self.stage == Ms4ClosedLoopStageV1::Complete
            && (self.exact_package_wave_proof_root_sha256.is_none()
                || self.ordinary_cpu_receipt_root_sha256.is_none()
                || self.ordinary_cpu_completion_root_sha256.is_none())
        {
            return Err("ms4_report_completion_proof_missing".to_owned());
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, String> {
        if self.schema == REPORT_SCHEMA_V1 {
            return canonical_json_sha256(&(
                REPORT_SCHEMA_V1,
                self.generated_at_unix,
                self.generation_sequence,
                self.stage,
                self.blocker.as_str(),
                self.frozen_envelope_root_sha256.as_deref(),
                self.future_envelope_root_sha256.as_deref(),
                self.candidate_root_sha256.as_deref(),
                self.package_id.as_deref(),
                self.negative_controls,
                self.external_admission_pass,
                self.ordinary_cpu_receipt_root_sha256.as_deref(),
                self.authority_ready,
                false,
            ))
            .map_err(str::to_owned);
        }
        canonical_json_sha256(&(
            REPORT_SCHEMA_V2,
            self.generated_at_unix,
            self.generation_sequence,
            self.stage,
            self.blocker.as_str(),
            self.frozen_envelope_root_sha256.as_deref(),
            self.future_envelope_root_sha256.as_deref(),
            self.candidate_root_sha256.as_deref(),
            self.package_id.as_deref(),
            self.exact_package_wave_proof_root_sha256.as_deref(),
            self.negative_controls,
            self.external_admission_pass,
            self.ordinary_cpu_receipt_root_sha256.as_deref(),
            self.ordinary_cpu_completion_root_sha256.as_deref(),
            self.authority_ready,
            false,
        ))
        .map_err(str::to_owned)
    }
}

pub(super) fn restore_report(path: &Path) -> Result<Ms4ClosedLoopReportV1, String> {
    let status_path = path.join("status.json");
    let bytes = match fs::read(&status_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Ms4ClosedLoopReportV1::default());
        }
        Err(error) => return Err(format!("ms4_report_restore_read:{error}")),
    };
    let report: Ms4ClosedLoopReportV1 = serde_json::from_slice(&bytes)
        .map_err(|error| format!("ms4_report_restore_decode:{error}"))?;
    report.validate()?;
    Ok(report)
}

pub(super) fn advance(state: &AppState) -> Result<Ms4ClosedLoopReportV1, String> {
    match advance_inner(state) {
        Ok(report) => persist_report(state, report),
        Err(error) => {
            let generation = state
                .ms3_frozen_version_space
                .as_ref()
                .and_then(|runtime| runtime.lock().ok())
                .map_or(0, |runtime| runtime.generation_sequence());
            persist_report(
                state,
                Ms4ClosedLoopReportV1::seal(
                    generation,
                    Ms4ClosedLoopStageV1::Blocked,
                    &bounded_reason(&error),
                ),
            )
        }
    }
}

fn advance_inner(state: &AppState) -> Result<Ms4ClosedLoopReportV1, String> {
    let Some(runtime) = &state.ms3_frozen_version_space else {
        return Ok(Ms4ClosedLoopReportV1::default());
    };
    let (generation, frozen, future, applicability) = {
        let runtime = runtime
            .lock()
            .map_err(|_| "ms4_ms3_runtime_lock_poisoned".to_owned())?;
        (
            runtime.generation_sequence(),
            runtime.envelope().cloned(),
            runtime.independent_future().cloned(),
            runtime.applicability_ledger().cloned(),
        )
    };
    let Some(frozen) = frozen else {
        return Ok(Ms4ClosedLoopReportV1::seal(
            generation,
            Ms4ClosedLoopStageV1::WaitingForMs3,
            "unique_law_pending",
        ));
    };
    let mut report = Ms4ClosedLoopReportV1::seal(
        generation,
        Ms4ClosedLoopStageV1::WaitingForMs3,
        "independent_future_pending",
    );
    report.frozen_envelope_root_sha256 = Some(frozen.envelope_root_sha256.clone());
    let Some(future) = future else {
        report.reseal();
        return Ok(report);
    };
    report.future_envelope_root_sha256 = Some(future.envelope_root_sha256.clone());
    if future.receipt.verdict != Ms3IndependentFutureVerdictV1::Pass {
        report.stage = Ms4ClosedLoopStageV1::Blocked;
        report.blocker = "ms3_future_contradiction".to_owned();
        report.reseal();
        return Ok(report);
    }
    if future.receipt.client_route_status != Some(418)
        || future
            .receipt
            .client_route_receipt_root_sha256
            .as_deref()
            .is_none_or(str::is_empty)
    {
        report.stage = Ms4ClosedLoopStageV1::Blocked;
        report.blocker = "ms3_future_independent_route_proof_missing".to_owned();
        report.reseal();
        return Ok(report);
    }

    let candidate_path = state
        .config
        .ms4_closed_loop_path
        .join("candidates")
        .join(format!("{}.cbor", future.envelope_root_sha256));
    let candidate = if candidate_path.exists() {
        let bytes = std::fs::read(&candidate_path)
            .map_err(|error| format!("ms4_candidate_read:{error}"))?;
        let candidate = Ms4ExternalAdmissionCandidateV1::from_canonical_bytes(&bytes)
            .map_err(|error| format!("ms4_candidate_restore:{error}"))?;
        if candidate.future_envelope_root_sha256() != future.envelope_root_sha256 {
            return Err("ms4_candidate_future_rebound".to_owned());
        }
        candidate
    } else {
        let topology_archive = state
            .multi_source_topology_archive
            .as_ref()
            .ok_or_else(|| "ms4_topology_archive_missing".to_owned())?;
        let (support_topology, future_topology, negative_topologies, support_partition_topologies) = {
            let archive = topology_archive
                .lock()
                .map_err(|_| "ms4_topology_archive_lock_poisoned".to_owned())?;
            let support = archive.row_by_root(&frozen.contract.topology_root_sha256);
            let future_row = archive.row_by_root(&future.receipt.topology_root_sha256);
            let support_intent = support
                .as_ref()
                .map(|row| row.structure.turn_intent_id_sha256.as_str());
            let support_partition = archive
                .rows()
                .into_iter()
                .filter(|row| support_intent == Some(row.structure.turn_intent_id_sha256.as_str()))
                .collect::<Vec<_>>();
            let negatives = applicability
                .as_ref()
                .into_iter()
                .flat_map(|ledger| &ledger.events)
                .filter(|event| {
                    event.disposition
                        == Ms3FutureApplicabilityDispositionV1::StructurallyNotApplicable
                })
                .filter_map(|event| archive.row_by_root(&event.topology_root_sha256))
                .filter(|topology| {
                    topology
                        .bridge_sequence
                        .is_some_and(|sequence| sequence >= frozen.contract.future_min_sequence)
                        && topology
                            .session_lineage_sha256
                            .as_ref()
                            .is_some_and(|lineage| {
                                lineage != &frozen.contract.session_lineage_sha256
                                    && lineage != &future.receipt.session_lineage_sha256
                            })
                        && topology.structure.provider_bound_turn_identity
                        && topology.physical_order_proven
                })
                .take(64)
                .collect::<Vec<_>>();
            (support, future_row, negatives, support_partition)
        };
        let (Some(support_topology), Some(future_topology)) = (support_topology, future_topology)
        else {
            report.stage = Ms4ClosedLoopStageV1::WaitingForRuntimeEvidence;
            report.blocker = "ms4_bound_topology_pending".to_owned();
            report.reseal();
            return Ok(report);
        };
        if negative_topologies.is_empty() {
            report.stage = Ms4ClosedLoopStageV1::WaitingForNegativeControl;
            report.blocker = "ms4_post_freeze_negative_control_pending".to_owned();
            report.reseal();
            return Ok(report);
        }
        report.negative_controls = u64::try_from(negative_topologies.len()).unwrap_or(u64::MAX);
        let frame_archive = state
            .multi_source_frame_archive
            .as_ref()
            .ok_or_else(|| "ms4_frame_archive_missing".to_owned())?;
        let support_intents = support_partition_topologies
            .iter()
            .map(|row| row.structure.turn_intent_id_sha256.clone())
            .collect::<BTreeSet<_>>();
        let (support_frame, future_frame, support_partition_frames) = {
            let archive = frame_archive
                .lock()
                .map_err(|_| "ms4_frame_archive_lock_poisoned".to_owned())?;
            (
                archive.frame_by_root(&frozen.contract.frame_root_sha256),
                archive.frame_by_root(&future.receipt.completed_frame_root_sha256),
                archive.frames_for_intents(&support_intents),
            )
        };
        let terminals = state
            .terminal_receipt_archive
            .as_ref()
            .ok_or_else(|| "ms4_terminal_archive_missing".to_owned())?;
        let support_request_ids = support_partition_topologies
            .iter()
            .map(|row| row.structure.request_event_id_sha256.clone())
            .collect::<BTreeSet<_>>();
        let (support_terminal, future_terminal, support_partition_terminals) = {
            let archive = terminals
                .lock()
                .map_err(|_| "ms4_terminal_archive_lock_poisoned".to_owned())?;
            (
                archive.receipt_for_request(&frozen.contract.request_event_id_sha256),
                archive.receipt_for_request(&future_topology.structure.request_event_id_sha256),
                archive.receipts_for_requests(&support_request_ids),
            )
        };
        let parities = state
            .remote_evidence_spool
            .as_ref()
            .ok_or_else(|| "ms4_runtime_parity_spool_missing".to_owned())?;
        let (support_parity, future_parity, future_route_receipt) = {
            let spool = parities
                .lock()
                .map_err(|_| "ms4_runtime_parity_spool_lock_poisoned".to_owned())?;
            let route_receipts = spool.route_receipts_by_frame_root();
            (
                spool.runtime_parity_for_frame(&frozen.contract.frame_root_sha256),
                spool.runtime_parity_for_frame(&future.receipt.completed_frame_root_sha256),
                route_receipts
                    .get(&future.receipt.completed_frame_root_sha256)
                    .cloned(),
            )
        };
        let (
            Some(support_frame),
            Some(future_frame),
            Some(support_terminal),
            Some(future_terminal),
            Some(support_parity),
            Some(future_parity),
        ) = (
            support_frame,
            future_frame,
            support_terminal,
            future_terminal,
            support_parity,
            future_parity,
        )
        else {
            report.stage = Ms4ClosedLoopStageV1::WaitingForRuntimeEvidence;
            report.blocker = "ms4_bound_runtime_parity_pending".to_owned();
            report.reseal();
            return Ok(report);
        };
        let support_transport = TransportBindingLedgerV1::build(
            &support_partition_topologies,
            &support_partition_frames,
            &support_partition_terminals,
        );
        let Some(support_transport_binding) = support_transport
            .bound_for_topology(&frozen.contract.topology_root_sha256)
            .iter()
            .find(|bound| {
                bound.binding.binding_root_sha256 == frozen.contract.transport_binding_root_sha256
            })
            .map(|bound| bound.binding.clone())
        else {
            report.stage = Ms4ClosedLoopStageV1::WaitingForRuntimeEvidence;
            report.blocker = "ms4_support_transport_binding_pending".to_owned();
            report.reseal();
            return Ok(report);
        };
        let Some(future_route_receipt) = future_route_receipt else {
            report.stage = Ms4ClosedLoopStageV1::WaitingForRuntimeEvidence;
            report.blocker = "ms4_bound_future_route_receipt_pending".to_owned();
            report.reseal();
            return Ok(report);
        };
        let candidate = Ms4ExternalAdmissionCandidateV1::seal(
            frozen.clone(),
            future.clone(),
            support_topology,
            support_frame,
            support_terminal,
            Some(support_transport_binding),
            support_parity,
            future_topology,
            future_frame,
            future_terminal,
            Some(future_route_receipt),
            future_parity,
            negative_topologies,
        )
        .map_err(|error| format!("ms4_candidate_seal:{error}"))?;
        let bytes = candidate
            .canonical_bytes()
            .map_err(|error| format!("ms4_candidate_encode:{error}"))?;
        fs::create_dir_all(
            candidate_path
                .parent()
                .ok_or_else(|| "ms4_candidate_parent_missing".to_owned())?,
        )
        .map_err(|error| format!("ms4_candidate_parent_create:{error}"))?;
        write_bytes_atomic(&candidate_path, &bytes, "ms4-external-candidate")?;
        let restored = Ms4ExternalAdmissionCandidateV1::from_canonical_bytes(
            &std::fs::read(&candidate_path)
                .map_err(|error| format!("ms4_candidate_verify_read:{error}"))?,
        )
        .map_err(|error| format!("ms4_candidate_verify:{error}"))?;
        if restored != candidate {
            return Err("ms4_candidate_restart_parity_mismatch".to_owned());
        }
        candidate
    };

    let exact_wave_proof = candidate
        .exact_package_wave_proof()
        .map_err(|error| format!("ms4_exact_package_wave_proof:{error}"))?;
    persist_exact_package_wave_proof(
        &state.config.ms4_closed_loop_path,
        candidate.candidate_root_sha256(),
        &exact_wave_proof,
    )?;
    report.exact_package_wave_proof_root_sha256 = Some(exact_wave_proof.proof_root_sha256.clone());
    let package = candidate
        .admitted_package()
        .map_err(|error| format!("ms4_package_rebuild:{error}"))?;
    report.candidate_root_sha256 = Some(candidate.candidate_root_sha256().to_owned());
    report.package_id = Some(package.package_id.clone());
    report.negative_controls = package.anti_centers.len() as u64;
    let changed = state
        .ms4_external_candidate
        .write()
        .map_err(|_| "ms4_candidate_cache_lock_poisoned".to_owned())?
        .as_ref()
        .is_none_or(|existing| {
            existing.candidate_root_sha256() != candidate.candidate_root_sha256()
        });
    if changed {
        *state
            .ms4_external_candidate
            .write()
            .map_err(|_| "ms4_candidate_cache_lock_poisoned".to_owned())? = Some(candidate);
        if let Some(trigger) = state
            .authority_trigger
            .lock()
            .map_err(|_| "ms4_authority_trigger_lock_poisoned".to_owned())?
            .as_ref()
        {
            let _ = trigger.try_send(());
        }
    }

    report.stage = Ms4ClosedLoopStageV1::CandidateSealed;
    report.blocker = "external_admission_pending".to_owned();
    let admitted = package_is_admitted(state, &package.package_id)?;
    report.external_admission_pass = admitted;
    report.authority_ready = admitted;
    if admitted {
        report.stage = Ms4ClosedLoopStageV1::OrdinaryCpuPending;
        report.blocker = "ordinary_cpu_accept_pending".to_owned();
        if let Some(completion) = ordinary_cpu_completion(
            &state.config.ms4_ordinary_economics_path,
            &package.package_id,
        )? {
            let existing = state
                .ms4_closed_loop_report
                .read()
                .map_err(|_| "ms4_report_cache_lock_poisoned".to_owned())?
                .clone();
            let immutable_match = existing.schema == REPORT_SCHEMA_V2
                && existing.stage == Ms4ClosedLoopStageV1::Complete
                && existing.generation_sequence == report.generation_sequence
                && existing.candidate_root_sha256 == report.candidate_root_sha256
                && existing.package_id == report.package_id
                && existing.exact_package_wave_proof_root_sha256
                    == report.exact_package_wave_proof_root_sha256
                && existing.ordinary_cpu_receipt_root_sha256.as_deref()
                    == Some(completion.verification_receipt_root_sha256.as_str())
                && existing.ordinary_cpu_completion_root_sha256.as_deref()
                    == Some(completion.completion_root_sha256.as_str());
            if immutable_match {
                return Ok(existing);
            }
            report.stage = Ms4ClosedLoopStageV1::Complete;
            report.blocker.clear();
            report.ordinary_cpu_receipt_root_sha256 =
                Some(completion.verification_receipt_root_sha256);
            report.ordinary_cpu_completion_root_sha256 = Some(completion.completion_root_sha256);
        }
    } else {
        report.stage = Ms4ClosedLoopStageV1::ExternalAdmissionPending;
    }
    report.reseal();
    Ok(report)
}

fn package_is_admitted(state: &AppState, package_id: &str) -> Result<bool, String> {
    let registry: ResponseRegistry = match std::fs::read(&state.config.response_registry_path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("ms4_registry_decode:{error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("ms4_registry_read:{error}")),
    };
    registry.validate().map_err(str::to_owned)?;
    let active = registry.packages.iter().any(|package| {
        package.package_id == package_id && package.state == ResponsePackageState::Active
    });
    let cache_ready = state
        .response_cache
        .read()
        .is_ok_and(|cache| cache.ready && unix_now() <= cache.admission_expires_at_unix);
    Ok(active && cache_ready)
}

fn ordinary_cpu_completion(
    path: &Path,
    package_id: &str,
) -> Result<Option<PackageCpuCompletionReceiptV1>, String> {
    first_durable_package_completion(path, package_id)
}

fn persist_exact_package_wave_proof(
    root: &Path,
    candidate_root_sha256: &str,
    proof: &Ms4ExactPackageWaveProofV1,
) -> Result<(), String> {
    let proof_path = root
        .join("exact-package-wave-proofs")
        .join(format!("{candidate_root_sha256}.cbor"));
    let bytes = proof
        .canonical_bytes()
        .map_err(|error| format!("ms4_exact_wave_proof_encode:{error}"))?;
    if proof_path.exists() {
        let restored = Ms4ExactPackageWaveProofV1::from_canonical_bytes(
            &fs::read(&proof_path).map_err(|error| format!("ms4_exact_wave_proof_read:{error}"))?,
        )
        .map_err(|error| format!("ms4_exact_wave_proof_restore:{error}"))?;
        if restored != *proof {
            return Err("ms4_exact_wave_proof_rebound".to_owned());
        }
        return Ok(());
    }
    fs::create_dir_all(
        proof_path
            .parent()
            .ok_or_else(|| "ms4_exact_wave_proof_parent_missing".to_owned())?,
    )
    .map_err(|error| format!("ms4_exact_wave_proof_parent_create:{error}"))?;
    write_bytes_atomic(&proof_path, &bytes, "ms4-exact-package-wave-proof")?;
    let restored = Ms4ExactPackageWaveProofV1::from_canonical_bytes(
        &fs::read(&proof_path)
            .map_err(|error| format!("ms4_exact_wave_proof_verify_read:{error}"))?,
    )
    .map_err(|error| format!("ms4_exact_wave_proof_verify:{error}"))?;
    if restored != *proof {
        return Err("ms4_exact_wave_proof_restart_parity_mismatch".to_owned());
    }
    Ok(())
}

fn persist_report(
    state: &AppState,
    report: Ms4ClosedLoopReportV1,
) -> Result<Ms4ClosedLoopReportV1, String> {
    report.validate()?;
    if state
        .ms4_closed_loop_report
        .read()
        .map_err(|_| "ms4_report_cache_lock_poisoned".to_owned())?
        .eq(&report)
        && state
            .config
            .ms4_closed_loop_path
            .join("status.json")
            .exists()
    {
        return Ok(report);
    }
    fs::create_dir_all(&state.config.ms4_closed_loop_path)
        .map_err(|error| format!("ms4_report_parent_create:{error}"))?;
    let bytes =
        serde_json::to_vec(&report).map_err(|error| format!("ms4_report_encode:{error}"))?;
    write_bytes_atomic(
        &state.config.ms4_closed_loop_path.join("status.json"),
        &bytes,
        "ms4-closed-loop-report",
    )?;
    *state
        .ms4_closed_loop_report
        .write()
        .map_err(|_| "ms4_report_cache_lock_poisoned".to_owned())? = report.clone();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::live_economics::LiveEconomicsLedger;

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn test_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "nando-ms4-economics-{label}-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn completion_requires_a_durable_framed_v4_receipt() {
        let root = test_path("ordinary");
        let path = root.join("economics-live.json");
        let package_id = "ms4-natural-test";
        let intent = "a".repeat(64);
        let receipt_root = "b".repeat(64);
        let mut ledger = LiveEconomicsLedger::open(&root).expect("open ledger");
        ledger
            .observe_request(&intent, 321, true)
            .expect("ordinary request");
        assert_eq!(
            ordinary_cpu_completion(&path, package_id).expect("scan"),
            None
        );
        ledger
            .observe_verified_accept_with_receipt(
                &intent,
                321,
                Some(package_id),
                Some(&receipt_root),
            )
            .expect("verified accept");
        let completion = ordinary_cpu_completion(&path, package_id)
            .expect("scan")
            .expect("durable completion");
        assert_eq!(completion.verification_receipt_root_sha256, receipt_root);
        assert_eq!(completion.exact_input_tokens, 321);
        drop(ledger);
        let restarted = ordinary_cpu_completion(&path, package_id)
            .expect("restart scan")
            .expect("restart completion");
        assert_eq!(restarted, completion);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn completion_latches_first_receipt_and_rejects_mutable_snapshot_rebinding() {
        let root = test_path("v4-latch");
        let path = root.join("economics-live.json");
        let package_id = "ms4-natural-test";
        let first_intent = "c".repeat(64);
        let second_intent = "d".repeat(64);
        let first_root = "e".repeat(64);
        let second_root = "f".repeat(64);
        let mut ledger = LiveEconomicsLedger::open(&root).expect("open ledger");
        ledger
            .observe_request(&first_intent, 100, true)
            .expect("first request");
        ledger
            .observe_verified_accept_with_receipt(
                &first_intent,
                100,
                Some(package_id),
                Some(&first_root),
            )
            .expect("first accept");
        ledger
            .observe_request(&second_intent, 200, true)
            .expect("second request");
        ledger
            .observe_verified_accept_with_receipt(
                &second_intent,
                200,
                Some(package_id),
                Some(&second_root),
            )
            .expect("second accept");
        let completion = ordinary_cpu_completion(&path, package_id)
            .expect("scan")
            .expect("first completion");
        assert_eq!(completion.verification_receipt_root_sha256, first_root);

        let mut snapshot: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).expect("snapshot read"))
                .expect("snapshot decode");
        snapshot["verified_by_package"][package_id]["first_receipt_root_sha256"] =
            serde_json::Value::String(second_root);
        std::fs::write(&path, serde_json::to_vec(&snapshot).expect("tamper encode"))
            .expect("tamper snapshot");
        assert_eq!(
            ordinary_cpu_completion(&path, package_id).expect("tampered scan"),
            None
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn report_restore_accepts_v1_and_latches_complete_v2_roots() {
        let root = test_path("report-restore");
        std::fs::create_dir_all(&root).expect("report root");
        let mut legacy = Ms4ClosedLoopReportV1::seal(31, Ms4ClosedLoopStageV1::Complete, "");
        legacy.schema = REPORT_SCHEMA_V1.to_owned();
        legacy.candidate_root_sha256 = Some("a".repeat(64));
        legacy.package_id = Some("ms4-natural-test".to_owned());
        legacy.external_admission_pass = true;
        legacy.authority_ready = true;
        legacy.ordinary_cpu_receipt_root_sha256 = Some("b".repeat(64));
        legacy.reseal();
        std::fs::write(
            root.join("status.json"),
            serde_json::to_vec(&legacy).expect("legacy report encode"),
        )
        .expect("legacy report");
        assert_eq!(restore_report(&root).expect("legacy restore"), legacy);

        let mut current = Ms4ClosedLoopReportV1::seal(31, Ms4ClosedLoopStageV1::Complete, "");
        current.candidate_root_sha256 = Some("a".repeat(64));
        current.package_id = Some("ms4-natural-test".to_owned());
        current.exact_package_wave_proof_root_sha256 = Some("c".repeat(64));
        current.external_admission_pass = true;
        current.authority_ready = true;
        current.ordinary_cpu_receipt_root_sha256 = Some("b".repeat(64));
        current.ordinary_cpu_completion_root_sha256 = Some("d".repeat(64));
        current.reseal();
        current.validate().expect("current report");
        std::fs::write(
            root.join("status.json"),
            serde_json::to_vec(&current).expect("current report encode"),
        )
        .expect("current report");
        assert_eq!(restore_report(&root).expect("current restore"), current);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
