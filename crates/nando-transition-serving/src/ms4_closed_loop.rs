//! Idempotent cold-path actuator from MS3 future PASS to ordinary CPU proof.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

use nando_operator_kernel::canonical_json_sha256;
use nando_operator_learning::multi_source::{
    Ms3FutureApplicabilityDispositionV1, Ms3IndependentFutureVerdictV1, TransportBindingLedgerV1,
};
use nando_response_actor::{
    Ms4ExternalAdmissionCandidateV1, ResponsePackageState, ResponseRegistry,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{AppState, bounded_reason, unix_now, write_bytes_atomic};

const REPORT_SCHEMA_V1: &str = "nando.ms4-autonomous-closed-loop-report.v1";
const ECONOMICS_TAIL_BYTES: u64 = 16 * 1024 * 1024;

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
    pub negative_controls: u64,
    pub external_admission_pass: bool,
    pub ordinary_cpu_receipt_root_sha256: Option<String>,
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
            schema: REPORT_SCHEMA_V1.to_owned(),
            report_root_sha256: String::new(),
            generated_at_unix: unix_now(),
            generation_sequence,
            stage,
            blocker: blocker.to_owned(),
            frozen_envelope_root_sha256: None,
            future_envelope_root_sha256: None,
            candidate_root_sha256: None,
            package_id: None,
            negative_controls: 0,
            external_admission_pass: false,
            ordinary_cpu_receipt_root_sha256: None,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        report.reseal();
        report
    }

    fn reseal(&mut self) {
        self.report_root_sha256 = canonical_json_sha256(&(
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
        .unwrap_or_default();
    }
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
        if let Some(receipt_root) = ordinary_cpu_receipt_root(
            &state.config.ms4_ordinary_economics_path,
            &package.package_id,
        )? {
            report.stage = Ms4ClosedLoopStageV1::Complete;
            report.blocker.clear();
            report.ordinary_cpu_receipt_root_sha256 = Some(receipt_root);
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

fn ordinary_cpu_receipt_root(path: &Path, package_id: &str) -> Result<Option<String>, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("ms4_economics_open:{error}")),
    };
    let length = file
        .metadata()
        .map_err(|error| format!("ms4_economics_metadata:{error}"))?
        .len();
    let start = length.saturating_sub(ECONOMICS_TAIL_BYTES);
    file.seek(SeekFrom::Start(start))
        .map_err(|error| format!("ms4_economics_seek:{error}"))?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    if start > 0 {
        reader
            .read_line(&mut line)
            .map_err(|error| format!("ms4_economics_partial:{error}"))?;
        line.clear();
    }
    while reader
        .read_line(&mut line)
        .map_err(|error| format!("ms4_economics_read:{error}"))?
        != 0
    {
        let value = serde_json::from_str::<Value>(&line).ok();
        if value.as_ref().is_some_and(|row| {
            let intent_dedupe_eligible =
                row.get("intent_dedupe_eligible").and_then(Value::as_bool) == Some(true);
            let ordinary = row
                .get("ordinary")
                .and_then(Value::as_bool)
                .unwrap_or_else(|| {
                    intent_dedupe_eligible
                        && row.get("traffic_source").and_then(Value::as_str) == Some("ordinary")
                });
            let controlled = row
                .get("controlled")
                .and_then(Value::as_bool)
                .unwrap_or(!ordinary);
            row.get("schema").and_then(Value::as_str) == Some("nando.economics-terminal.v1")
                && row.get("package_id").and_then(Value::as_str) == Some(package_id)
                && intent_dedupe_eligible
                && ordinary
                && !controlled
                && matches!(
                    row.get("route").and_then(Value::as_str),
                    Some("local_response_actor" | "local_actor")
                )
                && row.get("provider_attempt_id").is_some_and(Value::is_null)
                && row.get("avoided_call").and_then(Value::as_bool) == Some(true)
                && row.get("upstream_socket_opened").and_then(Value::as_bool) == Some(false)
                && row.get("verification_status").and_then(Value::as_str) == Some("verified")
                && row
                    .get("input_tokens")
                    .and_then(Value::as_u64)
                    .is_some_and(|tokens| tokens > 0)
        }) {
            return value
                .as_ref()
                .map(canonical_json_sha256)
                .transpose()
                .map_err(str::to_owned);
        }
        line.clear();
    }
    Ok(None)
}

fn persist_report(
    state: &AppState,
    report: Ms4ClosedLoopReportV1,
) -> Result<Ms4ClosedLoopReportV1, String> {
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

    use serde_json::json;

    use super::*;

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn test_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "nando-ms4-economics-{label}-{}-{}.jsonl",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn row(package_id: &str) -> Value {
        json!({
            "schema": "nando.economics-terminal.v1",
            "package_id": package_id,
            "intent_dedupe_eligible": true,
            "ordinary": true,
            "controlled": false,
            "traffic_source": "ordinary",
            "route": "local_response_actor",
            "provider_attempt_id": null,
            "avoided_call": true,
            "upstream_socket_opened": false,
            "verification_status": "verified",
            "input_tokens": 100
        })
    }

    #[test]
    fn completion_requires_an_ordinary_verified_avoided_upstream_receipt() {
        let path = test_path("ordinary");
        let package_id = "ms4-natural-test";
        let mut controlled = row(package_id);
        controlled["ordinary"] = Value::Bool(false);
        controlled["controlled"] = Value::Bool(true);
        let mut upstream = row(package_id);
        upstream["upstream_socket_opened"] = Value::Bool(true);
        let lines = [controlled, upstream]
            .into_iter()
            .map(|value| serde_json::to_string(&value).expect("row"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, format!("{lines}\n")).expect("economics");
        assert_eq!(
            ordinary_cpu_receipt_root(&path, package_id).expect("scan"),
            None
        );

        let ordinary = row(package_id);
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("append");
        use std::io::Write;
        writeln!(
            file,
            "{}",
            serde_json::to_string(&ordinary).expect("ordinary row")
        )
        .expect("ordinary append");
        file.sync_all().expect("ordinary sync");
        assert_eq!(
            ordinary_cpu_receipt_root(&path, package_id).expect("scan"),
            Some(canonical_json_sha256(&ordinary).expect("receipt root"))
        );

        let legacy_path = test_path("legacy-ordinary");
        let mut legacy = row(package_id);
        legacy
            .as_object_mut()
            .expect("legacy row")
            .remove("ordinary");
        legacy
            .as_object_mut()
            .expect("legacy row")
            .remove("controlled");
        std::fs::write(
            &legacy_path,
            format!(
                "{}\n",
                serde_json::to_string(&legacy).expect("legacy row encode")
            ),
        )
        .expect("legacy economics");
        assert_eq!(
            ordinary_cpu_receipt_root(&legacy_path, package_id).expect("legacy scan"),
            Some(canonical_json_sha256(&legacy).expect("legacy receipt root"))
        );
        std::fs::remove_file(path).expect("cleanup");
        std::fs::remove_file(legacy_path).expect("legacy cleanup");
    }
}
