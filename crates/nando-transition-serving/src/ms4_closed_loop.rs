//! Idempotent cold-path actuator from MS3 future PASS to ordinary CPU proof.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use nando_operator_admission::{
    ExecutionCertificateStatusV1, ExecutionCertificateV1, K1VocabularyGateV1,
    LawCertificateStatusV1, LawCertificateV1, MechanismCertificateStatusV1, MechanismCertificateV1,
    OperatorCertificationEntryV1, OperatorMechanismClassV1,
};
use nando_operator_kernel::canonical_json_sha256;
use nando_operator_learning::multi_source::{
    Ms3FutureApplicabilityDispositionV1, Ms3IndependentFutureVerdictV1, TransportBindingLedgerV1,
};
use nando_response_actor::{
    Ms4ExternalAdmissionCandidateV1, Ms4InSamplePhaseAblationV1, ResponsePackage,
    ResponsePackageState, ResponseRegistry,
};
use serde::{Deserialize, Serialize};

use crate::live_economics::{PackageCpuCompletionReceiptV1, first_durable_package_completion};
use crate::{AppState, bounded_reason, unix_now, write_bytes_atomic};

const REPORT_SCHEMA_V1: &str = "nando.ms4-autonomous-closed-loop-report.v1";
const REPORT_SCHEMA_V2: &str = "nando.ms4-autonomous-closed-loop-report.v2";
const REPORT_SCHEMA_V3: &str = "nando.ms4-autonomous-closed-loop-report.v3";
const REPORT_SCHEMA_V4: &str = "nando.ms4-autonomous-closed-loop-report.v4";
const REPORT_SCHEMA_V5: &str = "nando.ms4-autonomous-closed-loop-report.v5";
const ROLE_TOPOLOGY_SCHEMA_V1: &str = "nando.operator-role-topology.v1";

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
    Revoked,
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
    pub certification_ledger_root_sha256: Option<String>,
    #[serde(default)]
    pub operator_certification: Option<OperatorCertificationEntryV1>,
    #[serde(default)]
    pub k1_vocabulary_gate: Option<K1VocabularyGateV1>,
    #[serde(default)]
    pub in_sample_phase_ablation_root_sha256: Option<String>,
    #[serde(default)]
    pub exact_package_wave_proof_root_sha256: Option<String>,
    pub negative_controls: u64,
    #[serde(default)]
    pub anti_center_atoms: u64,
    #[serde(default)]
    pub exact_wave_holdout_contract_root_sha256: Option<String>,
    #[serde(default)]
    pub exact_wave_status: crate::ms4_exact_wave_holdout::Ms4ExactWaveHoldoutStatusV1,
    #[serde(default)]
    pub exact_wave_blocker: String,
    #[serde(default)]
    pub exact_wave_scanned_topology_rows: u64,
    #[serde(default)]
    pub exact_wave_independent_topology_rows: u64,
    #[serde(default)]
    pub exact_wave_precommitted_rows: u64,
    #[serde(default)]
    pub exact_wave_precommit_disqualified_rows: u64,
    #[serde(default)]
    pub exact_wave_settled_rows: u64,
    #[serde(default)]
    pub exact_wave_positive_holdout_rows: u64,
    #[serde(default)]
    pub exact_wave_phase_challenging_negative_rows: u64,
    #[serde(default)]
    pub exact_wave_scored_rows: u64,
    #[serde(default)]
    pub exact_wave_counterexample_rows: u64,
    #[serde(default)]
    pub exact_wave_full_wrong_rows: u64,
    #[serde(default)]
    pub exact_wave_no_phase_not_worse_rows: u64,
    #[serde(default)]
    pub exact_wave_censored_rows: u64,
    #[serde(default)]
    pub exact_wave_precommit_missing_rows: u64,
    #[serde(default)]
    pub exact_wave_settlement_pending_rows: u64,
    #[serde(default)]
    pub exact_wave_censored_precommit_missing_rows: u64,
    #[serde(default)]
    pub exact_wave_censored_precommit_disqualified_rows: u64,
    #[serde(default)]
    pub exact_wave_censored_settlement_unavailable_rows: u64,
    #[serde(default)]
    pub exact_wave_censored_primary_controls_abstained_rows: u64,
    #[serde(default)]
    pub exact_wave_unscored_settled_rows: u64,
    #[serde(default)]
    pub exact_wave_independent_lineages: u64,
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
            schema: REPORT_SCHEMA_V5.to_owned(),
            report_root_sha256: String::new(),
            generated_at_unix: unix_now(),
            generation_sequence,
            stage,
            blocker: blocker.to_owned(),
            frozen_envelope_root_sha256: None,
            future_envelope_root_sha256: None,
            candidate_root_sha256: None,
            package_id: None,
            certification_ledger_root_sha256: None,
            operator_certification: None,
            k1_vocabulary_gate: None,
            in_sample_phase_ablation_root_sha256: None,
            exact_package_wave_proof_root_sha256: None,
            negative_controls: 0,
            anti_center_atoms: 0,
            exact_wave_holdout_contract_root_sha256: None,
            exact_wave_status: Default::default(),
            exact_wave_blocker: String::new(),
            exact_wave_scanned_topology_rows: 0,
            exact_wave_independent_topology_rows: 0,
            exact_wave_precommitted_rows: 0,
            exact_wave_precommit_disqualified_rows: 0,
            exact_wave_settled_rows: 0,
            exact_wave_positive_holdout_rows: 0,
            exact_wave_phase_challenging_negative_rows: 0,
            exact_wave_scored_rows: 0,
            exact_wave_counterexample_rows: 0,
            exact_wave_full_wrong_rows: 0,
            exact_wave_no_phase_not_worse_rows: 0,
            exact_wave_censored_rows: 0,
            exact_wave_precommit_missing_rows: 0,
            exact_wave_settlement_pending_rows: 0,
            exact_wave_censored_precommit_missing_rows: 0,
            exact_wave_censored_precommit_disqualified_rows: 0,
            exact_wave_censored_settlement_unavailable_rows: 0,
            exact_wave_censored_primary_controls_abstained_rows: 0,
            exact_wave_unscored_settled_rows: 0,
            exact_wave_independent_lineages: 0,
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
        if !matches!(
            self.schema.as_str(),
            REPORT_SCHEMA_V1
                | REPORT_SCHEMA_V2
                | REPORT_SCHEMA_V3
                | REPORT_SCHEMA_V4
                | REPORT_SCHEMA_V5
        ) || self.phase_mutation_allowed
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
        if matches!(
            self.schema.as_str(),
            REPORT_SCHEMA_V3 | REPORT_SCHEMA_V4 | REPORT_SCHEMA_V5
        ) && matches!(
            self.stage,
            Ms4ClosedLoopStageV1::Complete | Ms4ClosedLoopStageV1::Revoked
        ) && (self.ordinary_cpu_receipt_root_sha256.is_none()
            || self.ordinary_cpu_completion_root_sha256.is_none())
        {
            return Err("ms4_report_operational_completion_proof_missing".to_owned());
        }
        if matches!(
            self.schema.as_str(),
            REPORT_SCHEMA_V3 | REPORT_SCHEMA_V4 | REPORT_SCHEMA_V5
        ) && ((self.exact_wave_status
            == crate::ms4_exact_wave_holdout::Ms4ExactWaveHoldoutStatusV1::Pass)
            != self.exact_package_wave_proof_root_sha256.is_some())
        {
            return Err("ms4_report_exact_wave_status_invalid".to_owned());
        }
        let typed_censored_rows = self
            .exact_wave_censored_precommit_missing_rows
            .saturating_add(self.exact_wave_censored_precommit_disqualified_rows)
            .saturating_add(self.exact_wave_censored_settlement_unavailable_rows)
            .saturating_add(self.exact_wave_censored_primary_controls_abstained_rows);
        if matches!(self.schema.as_str(), REPORT_SCHEMA_V4 | REPORT_SCHEMA_V5)
            && self.exact_wave_status
                == crate::ms4_exact_wave_holdout::Ms4ExactWaveHoldoutStatusV1::Pass
            && (self.exact_wave_counterexample_rows != 0
                || self.exact_wave_unscored_settled_rows != 0
                || self.exact_wave_settlement_pending_rows != 0
                || self.exact_wave_censored_rows != typed_censored_rows
                || self.exact_wave_independent_topology_rows
                    != self
                        .exact_wave_scored_rows
                        .saturating_add(self.exact_wave_censored_rows))
        {
            return Err("ms4_report_exact_wave_denominator_incomplete".to_owned());
        }
        if self.schema == REPORT_SCHEMA_V5 {
            if self
                .operator_certification
                .as_ref()
                .is_some_and(|entry| entry.validate().is_err())
                || self
                    .k1_vocabulary_gate
                    .as_ref()
                    .is_some_and(|gate| gate.validate().is_err())
                || self.operator_certification.is_some()
                    != (self.certification_ledger_root_sha256.is_some()
                        && self.k1_vocabulary_gate.is_some())
                || self.operator_certification.as_ref().is_some_and(|entry| {
                    self.package_id.as_deref() != Some(entry.package_id.as_str())
                })
            {
                return Err("ms4_report_certification_projection_invalid".to_owned());
            }
            if self.stage == Ms4ClosedLoopStageV1::Complete
                && self.operator_certification.as_ref().is_none_or(|entry| {
                    entry.execution.status != ExecutionCertificateStatusV1::Pass
                        || !entry.product_registry_member
                })
            {
                return Err("ms4_report_execution_certificate_missing".to_owned());
            }
            if self.stage == Ms4ClosedLoopStageV1::Revoked
                && (self.authority_ready
                    || self.operator_certification.as_ref().is_none_or(|entry| {
                        entry.execution.status != ExecutionCertificateStatusV1::Revoked
                            || entry.product_registry_member
                            || entry.k1_unit_eligible
                            || entry.false_bad_apply == 0
                    }))
            {
                return Err("ms4_report_execution_revocation_missing".to_owned());
            }
            if self.exact_wave_status
                == crate::ms4_exact_wave_holdout::Ms4ExactWaveHoldoutStatusV1::Pass
                && self.operator_certification.as_ref().is_none_or(|entry| {
                    entry.mechanism.status != MechanismCertificateStatusV1::Pass
                        || entry.mechanism.classification != OperatorMechanismClassV1::WaveCausal
                })
            {
                return Err("ms4_report_wave_mechanism_certificate_missing".to_owned());
            }
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
        if self.schema == REPORT_SCHEMA_V2 {
            return canonical_json_sha256(&(
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
            .map_err(str::to_owned);
        }
        if self.schema == REPORT_SCHEMA_V3 {
            return canonical_json_sha256(&(
                REPORT_SCHEMA_V3,
                self.generated_at_unix,
                (
                    self.generation_sequence,
                    self.stage,
                    self.blocker.as_str(),
                    self.frozen_envelope_root_sha256.as_deref(),
                    self.future_envelope_root_sha256.as_deref(),
                    self.candidate_root_sha256.as_deref(),
                    self.package_id.as_deref(),
                ),
                (
                    self.in_sample_phase_ablation_root_sha256.as_deref(),
                    self.exact_package_wave_proof_root_sha256.as_deref(),
                    self.negative_controls,
                    self.anti_center_atoms,
                ),
                (
                    self.exact_wave_holdout_contract_root_sha256.as_deref(),
                    self.exact_wave_status,
                    self.exact_wave_blocker.as_str(),
                    self.exact_wave_scanned_topology_rows,
                    self.exact_wave_independent_topology_rows,
                    self.exact_wave_precommitted_rows,
                    self.exact_wave_precommit_disqualified_rows,
                    self.exact_wave_settled_rows,
                    self.exact_wave_positive_holdout_rows,
                    self.exact_wave_phase_challenging_negative_rows,
                    self.exact_wave_independent_lineages,
                ),
                (
                    self.external_admission_pass,
                    self.ordinary_cpu_receipt_root_sha256.as_deref(),
                    self.ordinary_cpu_completion_root_sha256.as_deref(),
                    self.authority_ready,
                ),
                false,
            ))
            .map_err(str::to_owned);
        }
        if self.schema == REPORT_SCHEMA_V4 {
            return canonical_json_sha256(&(
                REPORT_SCHEMA_V4,
                self.generated_at_unix,
                (
                    self.generation_sequence,
                    self.stage,
                    self.blocker.as_str(),
                    self.frozen_envelope_root_sha256.as_deref(),
                    self.future_envelope_root_sha256.as_deref(),
                    self.candidate_root_sha256.as_deref(),
                    self.package_id.as_deref(),
                ),
                (
                    self.in_sample_phase_ablation_root_sha256.as_deref(),
                    self.exact_package_wave_proof_root_sha256.as_deref(),
                    self.negative_controls,
                    self.anti_center_atoms,
                ),
                (
                    self.exact_wave_holdout_contract_root_sha256.as_deref(),
                    self.exact_wave_status,
                    self.exact_wave_blocker.as_str(),
                    self.exact_wave_scanned_topology_rows,
                    self.exact_wave_independent_topology_rows,
                    self.exact_wave_precommitted_rows,
                    self.exact_wave_precommit_disqualified_rows,
                    self.exact_wave_settled_rows,
                    self.exact_wave_positive_holdout_rows,
                    self.exact_wave_phase_challenging_negative_rows,
                    self.exact_wave_independent_lineages,
                ),
                (
                    self.exact_wave_scored_rows,
                    self.exact_wave_counterexample_rows,
                    self.exact_wave_full_wrong_rows,
                    self.exact_wave_no_phase_not_worse_rows,
                    self.exact_wave_censored_rows,
                    self.exact_wave_precommit_missing_rows,
                    self.exact_wave_settlement_pending_rows,
                    self.exact_wave_censored_precommit_missing_rows,
                    self.exact_wave_censored_precommit_disqualified_rows,
                    self.exact_wave_censored_settlement_unavailable_rows,
                    self.exact_wave_censored_primary_controls_abstained_rows,
                    self.exact_wave_unscored_settled_rows,
                ),
                (
                    self.external_admission_pass,
                    self.ordinary_cpu_receipt_root_sha256.as_deref(),
                    self.ordinary_cpu_completion_root_sha256.as_deref(),
                    self.authority_ready,
                ),
                false,
            ))
            .map_err(str::to_owned);
        }
        canonical_json_sha256(&(
            REPORT_SCHEMA_V5,
            self.generated_at_unix,
            (
                self.generation_sequence,
                self.stage,
                self.blocker.as_str(),
                self.frozen_envelope_root_sha256.as_deref(),
                self.future_envelope_root_sha256.as_deref(),
                self.candidate_root_sha256.as_deref(),
                self.package_id.as_deref(),
            ),
            (
                self.in_sample_phase_ablation_root_sha256.as_deref(),
                self.exact_package_wave_proof_root_sha256.as_deref(),
                self.negative_controls,
                self.anti_center_atoms,
            ),
            (
                self.exact_wave_holdout_contract_root_sha256.as_deref(),
                self.exact_wave_status,
                self.exact_wave_blocker.as_str(),
                self.exact_wave_scanned_topology_rows,
                self.exact_wave_independent_topology_rows,
                self.exact_wave_precommitted_rows,
                self.exact_wave_precommit_disqualified_rows,
                self.exact_wave_settled_rows,
                self.exact_wave_positive_holdout_rows,
                self.exact_wave_phase_challenging_negative_rows,
                self.exact_wave_independent_lineages,
            ),
            (
                self.exact_wave_scored_rows,
                self.exact_wave_counterexample_rows,
                self.exact_wave_full_wrong_rows,
                self.exact_wave_no_phase_not_worse_rows,
                self.exact_wave_censored_rows,
                self.exact_wave_precommit_missing_rows,
                self.exact_wave_settlement_pending_rows,
                self.exact_wave_censored_precommit_missing_rows,
                self.exact_wave_censored_precommit_disqualified_rows,
                self.exact_wave_censored_settlement_unavailable_rows,
                self.exact_wave_censored_primary_controls_abstained_rows,
                self.exact_wave_unscored_settled_rows,
            ),
            (
                self.external_admission_pass,
                self.ordinary_cpu_receipt_root_sha256.as_deref(),
                self.ordinary_cpu_completion_root_sha256.as_deref(),
                self.authority_ready,
            ),
            (
                self.certification_ledger_root_sha256.as_deref(),
                self.operator_certification
                    .as_ref()
                    .map(|entry| entry.entry_root_sha256.as_str()),
                self.k1_vocabulary_gate
                    .as_ref()
                    .map(|gate| gate.gate_root_sha256.as_str()),
            ),
            false,
        ))
        .map_err(str::to_owned)
    }
}

pub(super) fn restore_report(
    path: &Path,
    certification_config: Option<&crate::operator_certification::CertificationAuthorityConfigV1>,
) -> Result<Ms4ClosedLoopReportV1, String> {
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
    if let (Some(ledger_root), Some(entry), Some(gate)) = (
        report.certification_ledger_root_sha256.as_deref(),
        report.operator_certification.as_ref(),
        report.k1_vocabulary_gate.as_ref(),
    ) {
        let config = certification_config
            .ok_or_else(|| "operator_certification_config_missing".to_owned())?;
        crate::operator_certification::validate_projection(
            config,
            &crate::operator_certification::CertificationProjectionV1 {
                ledger_root_sha256: ledger_root.to_owned(),
                entry: entry.clone(),
                k1_vocabulary_gate: gate.clone(),
            },
        )?;
    }
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

    let in_sample_phase_ablation = candidate
        .in_sample_phase_ablation()
        .map_err(|error| format!("ms4_in_sample_phase_ablation:{error}"))?;
    persist_in_sample_phase_ablation(
        &state.config.ms4_closed_loop_path,
        candidate.candidate_root_sha256(),
        &in_sample_phase_ablation,
    )?;
    report.in_sample_phase_ablation_root_sha256 =
        Some(in_sample_phase_ablation.proof_root_sha256.clone());
    let package = candidate
        .admitted_package()
        .map_err(|error| format!("ms4_package_rebuild:{error}"))?;
    report.candidate_root_sha256 = Some(candidate.candidate_root_sha256().to_owned());
    report.package_id = Some(package.package_id.clone());
    report.negative_controls =
        u64::try_from(candidate.topology_negative_control_count()).unwrap_or(u64::MAX);
    report.anti_center_atoms = u64::try_from(package.anti_centers.len()).unwrap_or(u64::MAX);
    match crate::ms4_exact_wave_holdout::evaluate_holdout(state, &candidate, &package, unix_now()) {
        Ok(exact_wave) => {
            report.exact_wave_holdout_contract_root_sha256 =
                Some(exact_wave.contract_root_sha256.clone());
            report.exact_wave_status = exact_wave.status;
            report.exact_wave_blocker = exact_wave.blocker;
            report.exact_wave_scanned_topology_rows = exact_wave.scanned_topology_rows;
            report.exact_wave_independent_topology_rows = exact_wave.independent_topology_rows;
            report.exact_wave_precommitted_rows = exact_wave.precommitted_rows;
            report.exact_wave_precommit_disqualified_rows = exact_wave.precommit_disqualified_rows;
            report.exact_wave_settled_rows = exact_wave.settled_rows;
            report.exact_wave_positive_holdout_rows = exact_wave.positive_holdout_rows;
            report.exact_wave_phase_challenging_negative_rows =
                exact_wave.phase_challenging_negative_rows;
            report.exact_wave_scored_rows = exact_wave.scored_rows;
            report.exact_wave_counterexample_rows = exact_wave.counterexample_rows;
            report.exact_wave_full_wrong_rows = exact_wave.full_wrong_rows;
            report.exact_wave_no_phase_not_worse_rows = exact_wave.no_phase_not_worse_rows;
            report.exact_wave_censored_rows = exact_wave.censored_rows;
            report.exact_wave_precommit_missing_rows = exact_wave.precommit_missing_rows;
            report.exact_wave_settlement_pending_rows = exact_wave.settlement_pending_rows;
            report.exact_wave_censored_precommit_missing_rows =
                exact_wave.censored_precommit_missing_rows;
            report.exact_wave_censored_precommit_disqualified_rows =
                exact_wave.censored_precommit_disqualified_rows;
            report.exact_wave_censored_settlement_unavailable_rows =
                exact_wave.censored_settlement_unavailable_rows;
            report.exact_wave_censored_primary_controls_abstained_rows =
                exact_wave.censored_primary_controls_abstained_rows;
            report.exact_wave_unscored_settled_rows = exact_wave.unscored_settled_rows;
            report.exact_wave_independent_lineages = exact_wave.independent_lineages;
            report.exact_package_wave_proof_root_sha256 =
                exact_wave.proof.map(|proof| proof.proof_root_sha256);
        }
        Err(error) => {
            report.exact_wave_status =
                crate::ms4_exact_wave_holdout::Ms4ExactWaveHoldoutStatusV1::Fail;
            report.exact_wave_blocker = bounded_reason(&error);
        }
    }
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
            .map_err(|_| "ms4_candidate_cache_lock_poisoned".to_owned())? = Some(candidate.clone());
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
            let immutable_match = matches!(
                existing.schema.as_str(),
                REPORT_SCHEMA_V2 | REPORT_SCHEMA_V3 | REPORT_SCHEMA_V4 | REPORT_SCHEMA_V5
            ) && existing.stage == Ms4ClosedLoopStageV1::Complete
                && existing.generation_sequence == report.generation_sequence
                && existing.candidate_root_sha256 == report.candidate_root_sha256
                && existing.package_id == report.package_id
                && existing.ordinary_cpu_receipt_root_sha256.as_deref()
                    == Some(completion.verification_receipt_root_sha256.as_str())
                && existing.ordinary_cpu_completion_root_sha256.as_deref()
                    == Some(completion.completion_root_sha256.as_str());
            if immutable_match {
                report.ordinary_cpu_receipt_root_sha256 = existing.ordinary_cpu_receipt_root_sha256;
                report.ordinary_cpu_completion_root_sha256 =
                    existing.ordinary_cpu_completion_root_sha256;
            } else {
                report.ordinary_cpu_receipt_root_sha256 =
                    Some(completion.verification_receipt_root_sha256);
                report.ordinary_cpu_completion_root_sha256 =
                    Some(completion.completion_root_sha256);
            }
            report.stage = Ms4ClosedLoopStageV1::Complete;
            report.blocker.clear();
        }
    } else {
        report.stage = Ms4ClosedLoopStageV1::ExternalAdmissionPending;
    }
    let certification = certify_operator(state, &report, &candidate, &package)?;
    if certification.entry.execution.status == ExecutionCertificateStatusV1::Revoked {
        report.stage = Ms4ClosedLoopStageV1::Revoked;
        report.blocker = "runtime_false_bad_apply".to_owned();
        report.authority_ready = false;
    }
    report.certification_ledger_root_sha256 = Some(certification.ledger_root_sha256);
    report.operator_certification = Some(certification.entry);
    report.k1_vocabulary_gate = Some(certification.k1_vocabulary_gate);
    report.reseal();
    Ok(report)
}

fn certify_operator(
    state: &AppState,
    report: &Ms4ClosedLoopReportV1,
    candidate: &Ms4ExternalAdmissionCandidateV1,
    package: &ResponsePackage,
) -> Result<crate::operator_certification::CertificationProjectionV1, String> {
    let bundle_id = candidate.canonical_bundle_id_sha256();
    let law_id = package
        .crystallized_operator
        .as_ref()
        .ok_or_else(|| "operator_certification_bundle_missing".to_owned())?
        .canonical_law_id_sha256()
        .map_err(|error| format!("operator_certification_law_id:{error:?}"))?
        .ok_or_else(|| "operator_certification_law_id_missing".to_owned())?;
    let role_topology_id = role_topology_id(package)?;
    let certification_config = certification_config(&state.config);
    let (false_bad_apply, live_safety_evidence) =
        crate::operator_certification::durable_false_bad_apply_evidence(
            &certification_config,
            &package.package_id,
        )?;

    let execution_pass = report.stage == Ms4ClosedLoopStageV1::Complete
        && report.external_admission_pass
        && report.ordinary_cpu_receipt_root_sha256.is_some()
        && report.ordinary_cpu_completion_root_sha256.is_some();
    let mut execution_evidence = vec![
        candidate.candidate_root_sha256().to_owned(),
        candidate.future_envelope_root_sha256().to_owned(),
    ];
    execution_evidence.extend(report.ordinary_cpu_receipt_root_sha256.iter().cloned());
    execution_evidence.extend(report.ordinary_cpu_completion_root_sha256.iter().cloned());
    execution_evidence.extend(live_safety_evidence);
    let execution_revoked = false_bad_apply > 0;
    let execution = ExecutionCertificateV1::seal(
        bundle_id,
        &package.package_id,
        if execution_revoked {
            ExecutionCertificateStatusV1::Revoked
        } else if execution_pass {
            ExecutionCertificateStatusV1::Pass
        } else {
            ExecutionCertificateStatusV1::Pending
        },
        execution_evidence,
        if execution_revoked {
            "runtime_false_bad_apply"
        } else if execution_pass {
            ""
        } else {
            "ordinary_cpu_completion_pending"
        },
    )
    .map_err(str::to_owned)?;

    let cleanup = crate::operator_certification::restore_cleanup_receipt(
        &certification_config,
        bundle_id,
        &package.package_id,
        candidate.candidate_root_sha256(),
    )?;
    let mut law_evidence = vec![
        report
            .frozen_envelope_root_sha256
            .clone()
            .ok_or_else(|| "operator_certification_frozen_root_missing".to_owned())?,
        candidate.future_envelope_root_sha256().to_owned(),
        candidate.candidate_root_sha256().to_owned(),
        package
            .proof
            .adaptive_identification
            .as_ref()
            .ok_or_else(|| "operator_certification_adaptive_proof_missing".to_owned())?
            .proof_root_sha256()
            .to_owned(),
    ];
    law_evidence.extend(
        cleanup
            .iter()
            .map(|receipt| receipt.receipt_root_sha256.clone()),
    );
    let cleanup_present = cleanup.is_some();
    let cleanup_root = cleanup.map(|receipt| receipt.receipt_root_sha256);
    let law = LawCertificateV1::seal(
        bundle_id,
        &package.package_id,
        if cleanup_present {
            LawCertificateStatusV1::Pass
        } else {
            LawCertificateStatusV1::Partial
        },
        law_evidence,
        cleanup_root,
        if cleanup_present {
            ""
        } else {
            "exact_memory_cleanup_receipt_missing"
        },
    )
    .map_err(str::to_owned)?;

    let (mechanism_status, mechanism_classification, mechanism_blocker) =
        match report.exact_wave_status {
            crate::ms4_exact_wave_holdout::Ms4ExactWaveHoldoutStatusV1::Pass => (
                MechanismCertificateStatusV1::Pass,
                OperatorMechanismClassV1::WaveCausal,
                "",
            ),
            crate::ms4_exact_wave_holdout::Ms4ExactWaveHoldoutStatusV1::Collecting => (
                MechanismCertificateStatusV1::Collecting,
                OperatorMechanismClassV1::Unresolved,
                if report.exact_wave_blocker.is_empty() {
                    "post_center_holdout_collecting"
                } else {
                    report.exact_wave_blocker.as_str()
                },
            ),
            crate::ms4_exact_wave_holdout::Ms4ExactWaveHoldoutStatusV1::Fail
            | crate::ms4_exact_wave_holdout::Ms4ExactWaveHoldoutStatusV1::AcquisitionFail => (
                MechanismCertificateStatusV1::Fail,
                OperatorMechanismClassV1::Unresolved,
                if report.exact_wave_blocker.is_empty() {
                    "wave_causal_not_proven"
                } else {
                    report.exact_wave_blocker.as_str()
                },
            ),
        };
    let mut mechanism_evidence = vec![candidate.candidate_root_sha256().to_owned()];
    mechanism_evidence.extend(
        report
            .exact_wave_holdout_contract_root_sha256
            .iter()
            .cloned(),
    );
    mechanism_evidence.extend(report.exact_package_wave_proof_root_sha256.iter().cloned());
    let mechanism = MechanismCertificateV1::seal(
        bundle_id,
        &package.package_id,
        mechanism_status,
        mechanism_classification,
        mechanism_evidence,
        mechanism_blocker,
    )
    .map_err(str::to_owned)?;

    let entry = OperatorCertificationEntryV1::seal(
        bundle_id,
        &package.package_id,
        &law_id,
        &role_topology_id,
        execution,
        law,
        mechanism,
        false_bad_apply,
    )
    .map_err(str::to_owned)?;
    crate::operator_certification::append_entry(&certification_config, entry)
}

fn certification_config(
    config: &crate::ServingConfig,
) -> crate::operator_certification::CertificationAuthorityConfigV1 {
    crate::operator_certification::CertificationAuthorityConfigV1 {
        root: config.ms4_closed_loop_path.clone(),
        cleanup_receipts_path: config.operator_cleanup_receipts_path.clone(),
        anchor_path: config.operator_certification_anchor_path.clone(),
        authority_socket_path: config.operator_certification_authority_socket_path.clone(),
        authority_public_key_path: config
            .operator_certification_authority_public_key_path
            .clone(),
        cleanup_public_key_path: config.operator_cleanup_verifier_public_key_path.clone(),
        response_registry_path: config.response_registry_path.clone(),
        runtime_revocations_path: config.runtime_package_revocations_path.clone(),
    }
}

fn role_topology_id(package: &ResponsePackage) -> Result<String, String> {
    let restored = package
        .crystallized_operator
        .as_ref()
        .ok_or_else(|| "operator_role_topology_bundle_missing".to_owned())?
        .restore_verified()
        .map_err(|_| "operator_role_topology_restore_failed".to_owned())?;
    canonical_json_sha256(&(
        ROLE_TOPOLOGY_SCHEMA_V1,
        restored.role_graph().topology_commitment_sha256(),
    ))
    .map_err(str::to_owned)
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

fn persist_in_sample_phase_ablation(
    root: &Path,
    candidate_root_sha256: &str,
    proof: &Ms4InSamplePhaseAblationV1,
) -> Result<(), String> {
    let proof_path = root
        .join("in-sample-phase-ablations")
        .join(format!("{candidate_root_sha256}.cbor"));
    let bytes = proof
        .canonical_bytes()
        .map_err(|error| format!("ms4_exact_wave_proof_encode:{error}"))?;
    if proof_path.exists() {
        let restored = Ms4InSamplePhaseAblationV1::from_canonical_bytes(
            &fs::read(&proof_path).map_err(|error| format!("ms4_exact_wave_proof_read:{error}"))?,
        )
        .map_err(|error| format!("ms4_exact_wave_proof_restore:{error}"))?;
        if restored != *proof {
            return Err("ms4_in_sample_phase_ablation_rebound".to_owned());
        }
        return Ok(());
    }
    fs::create_dir_all(
        proof_path
            .parent()
            .ok_or_else(|| "ms4_exact_wave_proof_parent_missing".to_owned())?,
    )
    .map_err(|error| format!("ms4_exact_wave_proof_parent_create:{error}"))?;
    write_bytes_atomic(&proof_path, &bytes, "ms4-in-sample-phase-ablation")?;
    let restored = Ms4InSamplePhaseAblationV1::from_canonical_bytes(
        &fs::read(&proof_path)
            .map_err(|error| format!("ms4_exact_wave_proof_verify_read:{error}"))?,
    )
    .map_err(|error| format!("ms4_exact_wave_proof_verify:{error}"))?;
    if restored != *proof {
        return Err("ms4_in_sample_phase_ablation_restart_parity_mismatch".to_owned());
    }
    Ok(())
}

fn persist_report(
    state: &AppState,
    report: Ms4ClosedLoopReportV1,
) -> Result<Ms4ClosedLoopReportV1, String> {
    report.validate()?;
    let existing = state
        .ms4_closed_loop_report
        .read()
        .map_err(|_| "ms4_report_cache_lock_poisoned".to_owned())?
        .clone();
    if (existing == report || same_report_payload(&existing, &report))
        && state
            .config
            .ms4_closed_loop_path
            .join("status.json")
            .exists()
    {
        return Ok(existing);
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

fn same_report_payload(left: &Ms4ClosedLoopReportV1, right: &Ms4ClosedLoopReportV1) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    left.generated_at_unix = 0;
    right.generated_at_unix = 0;
    left.report_root_sha256.clear();
    right.report_root_sha256.clear();
    left == right
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
    fn report_restore_accepts_legacy_and_v3_separates_operational_from_exact_wave() {
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
        assert_eq!(restore_report(&root, None).expect("legacy restore"), legacy);

        let mut current = Ms4ClosedLoopReportV1::seal(31, Ms4ClosedLoopStageV1::Complete, "");
        current.schema = REPORT_SCHEMA_V4.to_owned();
        current.candidate_root_sha256 = Some("a".repeat(64));
        current.package_id = Some("ms4-natural-test".to_owned());
        current.in_sample_phase_ablation_root_sha256 = Some("c".repeat(64));
        current.external_admission_pass = true;
        current.authority_ready = true;
        current.ordinary_cpu_receipt_root_sha256 = Some("b".repeat(64));
        current.ordinary_cpu_completion_root_sha256 = Some("d".repeat(64));
        current.exact_wave_status =
            crate::ms4_exact_wave_holdout::Ms4ExactWaveHoldoutStatusV1::Collecting;
        current.exact_wave_blocker = "post_center_holdout_collecting".to_owned();
        current.reseal();
        current.validate().expect("current report");
        assert!(current.exact_package_wave_proof_root_sha256.is_none());
        std::fs::write(
            root.join("status.json"),
            serde_json::to_vec(&current).expect("current report encode"),
        )
        .expect("current report");
        assert_eq!(
            restore_report(&root, None).expect("current restore"),
            current
        );

        let mut uncertified_v5 = current.clone();
        uncertified_v5.schema = REPORT_SCHEMA_V5.to_owned();
        uncertified_v5.reseal();
        assert_eq!(
            uncertified_v5.validate(),
            Err("ms4_report_execution_certificate_missing".to_owned())
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn revoked_report_preserves_completion_proof_and_closes_authority() {
        let bundle = "a".repeat(64);
        let package = "ms4-natural-revoked";
        let execution = ExecutionCertificateV1::seal(
            &bundle,
            package,
            ExecutionCertificateStatusV1::Revoked,
            vec!["b".repeat(64)],
            "runtime_false_bad_apply",
        )
        .expect("execution");
        let law = LawCertificateV1::seal(
            &bundle,
            package,
            LawCertificateStatusV1::Partial,
            vec!["c".repeat(64)],
            None,
            "exact_memory_cleanup_receipt_missing",
        )
        .expect("law");
        let mechanism = MechanismCertificateV1::seal(
            &bundle,
            package,
            MechanismCertificateStatusV1::Collecting,
            OperatorMechanismClassV1::Unresolved,
            vec!["d".repeat(64)],
            "post_center_holdout_collecting",
        )
        .expect("mechanism");
        let entry = OperatorCertificationEntryV1::seal(
            &bundle,
            package,
            &"e".repeat(64),
            &"f".repeat(64),
            execution,
            law,
            mechanism,
            1,
        )
        .expect("entry");
        let mut ledger =
            nando_operator_admission::OperatorCertificationLedgerV1::empty().expect("ledger");
        ledger.append(entry.clone()).expect("append");

        let mut report = Ms4ClosedLoopReportV1::seal(
            31,
            Ms4ClosedLoopStageV1::Revoked,
            "runtime_false_bad_apply",
        );
        report.package_id = Some(package.to_owned());
        report.external_admission_pass = true;
        report.authority_ready = false;
        report.ordinary_cpu_receipt_root_sha256 = Some("1".repeat(64));
        report.ordinary_cpu_completion_root_sha256 = Some("2".repeat(64));
        report.certification_ledger_root_sha256 = Some(ledger.ledger_root_sha256.clone());
        report.operator_certification = Some(entry);
        report.k1_vocabulary_gate = Some(ledger.k1_vocabulary_gate().expect("k1 gate"));
        report.reseal();
        report.validate().expect("revoked report");

        report.authority_ready = true;
        report.reseal();
        assert_eq!(
            report.validate(),
            Err("ms4_report_execution_revocation_missing".to_owned())
        );
    }
}
