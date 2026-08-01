//! Durable pre-action controls for an independent post-center MS4 holdout.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use nando_operator_learning::{FramedCborLedger, read_framed_cbor, write_atomic_cbor};
use nando_response_actor::{
    Ms4ExternalAdmissionCandidateV1, ResponseExecutionStatus, ResponseExecutor, ResponsePackage,
    ResponsePhaseControlV1, response_execution_payload_digest,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::AppState;
use crate::live_economics::durable_package_completions;

const CONTRACT_SCHEMA: &str = "nando.ms4-exact-wave-holdout-contract.v1";
const PRECOMMIT_SCHEMA: &str = "nando.ms4-exact-wave-precommit.v1";
const ROOT_DIR: &str = "exact-wave-holdout-v1";
const ACTIVE_CONTRACT_FILE: &str = "active-contract.cbor";
const PRECOMMIT_DIR: &str = "precommits";
const PRECOMMIT_PREFIX: &str = "precommit";
const CONTRACT_WINDOW_SECONDS: u64 = 24 * 60 * 60;
const SETTLEMENT_GRACE_SECONDS: u64 = 5 * 60;
const MAX_HOLDOUT_ROWS: u64 = 256;
const MAX_PRECOMMITS: usize = 4_096;
const PROOF_SCHEMA: &str = "nando.ms4-independent-exact-wave-proof.v1";
const EVALUATION_SCHEMA: &str = "nando.ms4-exact-wave-holdout-evaluation.v1";
const MIN_INDEPENDENT_LINEAGES: u64 = 2;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Ms4ExactWaveHoldoutContractV1 {
    pub schema: String,
    pub contract_root_sha256: String,
    pub candidate_root_sha256: String,
    pub package_id: String,
    pub execution_payload_sha256: String,
    pub center_training_max_sequence: u64,
    pub contract_watermark: u64,
    pub holdout_min_sequence: u64,
    pub opened_at_unix: u64,
    pub deadline_unix: u64,
    pub settlement_grace_seconds: u64,
    pub max_holdout_rows: u64,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

impl Ms4ExactWaveHoldoutContractV1 {
    fn seal(
        candidate_root_sha256: &str,
        package_id: &str,
        execution_payload_sha256: &str,
        center_training_max_sequence: u64,
        contract_watermark: u64,
        opened_at_unix: u64,
    ) -> Result<Self, String> {
        let mut contract = Self {
            schema: CONTRACT_SCHEMA.to_owned(),
            contract_root_sha256: String::new(),
            candidate_root_sha256: candidate_root_sha256.to_owned(),
            package_id: package_id.to_owned(),
            execution_payload_sha256: execution_payload_sha256.to_owned(),
            center_training_max_sequence,
            contract_watermark,
            holdout_min_sequence: contract_watermark.saturating_add(1),
            opened_at_unix,
            deadline_unix: opened_at_unix.saturating_add(CONTRACT_WINDOW_SECONDS),
            settlement_grace_seconds: SETTLEMENT_GRACE_SECONDS,
            max_holdout_rows: MAX_HOLDOUT_ROWS,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        contract.contract_root_sha256 = contract.expected_root()?;
        contract.validate()?;
        Ok(contract)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != CONTRACT_SCHEMA
            || !valid_nonzero_sha256(&self.contract_root_sha256)
            || !valid_nonzero_sha256(&self.candidate_root_sha256)
            || self.package_id.is_empty()
            || !valid_nonzero_sha256(&self.execution_payload_sha256)
            || self.center_training_max_sequence == 0
            || self.contract_watermark < self.center_training_max_sequence
            || self.holdout_min_sequence != self.contract_watermark.saturating_add(1)
            || self.opened_at_unix == 0
            || self.deadline_unix != self.opened_at_unix.saturating_add(CONTRACT_WINDOW_SECONDS)
            || self.settlement_grace_seconds != SETTLEMENT_GRACE_SECONDS
            || self.max_holdout_rows != MAX_HOLDOUT_ROWS
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.contract_root_sha256 != self.expected_root()?
        {
            return Err("ms4_exact_wave_holdout_contract_invalid".to_owned());
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, String> {
        canonical_json_sha256(&(
            CONTRACT_SCHEMA,
            self.candidate_root_sha256.as_str(),
            self.package_id.as_str(),
            self.execution_payload_sha256.as_str(),
            self.center_training_max_sequence,
            self.contract_watermark,
            self.holdout_min_sequence,
            self.opened_at_unix,
            self.deadline_unix,
            self.settlement_grace_seconds,
            self.max_holdout_rows,
            false,
            false,
        ))
        .map_err(str::to_owned)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Ms4ExactWaveHoldoutStatusV1 {
    #[default]
    Collecting,
    Pass,
    Fail,
    AcquisitionFail,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Ms4ExactWaveControlScoreV1 {
    pub mode: String,
    pub correct_rows: u64,
    pub wrong_rows: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Ms4IndependentExactWaveProofV1 {
    pub schema: String,
    pub proof_root_sha256: String,
    pub contract_root_sha256: String,
    pub candidate_root_sha256: String,
    pub package_id: String,
    pub execution_payload_sha256: String,
    pub center_training_max_sequence: u64,
    pub holdout_min_sequence: u64,
    pub positive_precommit_roots_sha256: Vec<String>,
    pub positive_completion_roots_sha256: Vec<String>,
    pub negative_precommit_roots_sha256: Vec<String>,
    pub negative_binding_roots_sha256: Vec<String>,
    pub independent_lineages_sha256: Vec<String>,
    pub full: Ms4ExactWaveControlScoreV1,
    pub no_phase: Ms4ExactWaveControlScoreV1,
    pub shuffled_phase: Ms4ExactWaveControlScoreV1,
    pub magnitude_only: Ms4ExactWaveControlScoreV1,
    pub random_center: Ms4ExactWaveControlScoreV1,
    pub strict_all_ablation_pass: bool,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Ms4ExactWaveHoldoutEvaluationV1 {
    pub schema: String,
    pub contract_root_sha256: String,
    pub status: Ms4ExactWaveHoldoutStatusV1,
    pub blocker: String,
    pub scanned_topology_rows: u64,
    pub independent_topology_rows: u64,
    pub precommitted_rows: u64,
    pub settled_rows: u64,
    pub positive_holdout_rows: u64,
    pub phase_challenging_negative_rows: u64,
    pub precommit_missing_rows: u64,
    pub precommit_disqualified_rows: u64,
    pub settlement_pending_rows: u64,
    pub training_lineage_reuse_excluded_rows: u64,
    pub independent_lineages: u64,
    pub proof: Option<Ms4IndependentExactWaveProofV1>,
}

impl Ms4IndependentExactWaveProofV1 {
    #[allow(clippy::too_many_arguments)]
    fn seal(
        contract: &Ms4ExactWaveHoldoutContractV1,
        positive_precommit_roots_sha256: Vec<String>,
        positive_completion_roots_sha256: Vec<String>,
        negative_precommit_roots_sha256: Vec<String>,
        negative_binding_roots_sha256: Vec<String>,
        independent_lineages_sha256: Vec<String>,
        scores: BTreeMap<ResponsePhaseControlV1, Ms4ExactWaveControlScoreV1>,
    ) -> Result<Self, String> {
        let control = |mode| {
            scores
                .get(&mode)
                .cloned()
                .ok_or_else(|| "ms4_exact_wave_control_score_missing".to_owned())
        };
        let full = control(ResponsePhaseControlV1::Full)?;
        let no_phase = control(ResponsePhaseControlV1::NoPhase)?;
        let shuffled_phase = control(ResponsePhaseControlV1::ShuffledPhase)?;
        let magnitude_only = control(ResponsePhaseControlV1::MagnitudeOnly)?;
        let random_center = control(ResponsePhaseControlV1::RandomCenter)?;
        let strict_all_ablation_pass = full.wrong_rows == 0
            && [&no_phase, &shuffled_phase, &magnitude_only, &random_center]
                .into_iter()
                .all(|score| score.correct_rows < full.correct_rows);
        let mut proof = Self {
            schema: PROOF_SCHEMA.to_owned(),
            proof_root_sha256: String::new(),
            contract_root_sha256: contract.contract_root_sha256.clone(),
            candidate_root_sha256: contract.candidate_root_sha256.clone(),
            package_id: contract.package_id.clone(),
            execution_payload_sha256: contract.execution_payload_sha256.clone(),
            center_training_max_sequence: contract.center_training_max_sequence,
            holdout_min_sequence: contract.holdout_min_sequence,
            positive_precommit_roots_sha256,
            positive_completion_roots_sha256,
            negative_precommit_roots_sha256,
            negative_binding_roots_sha256,
            independent_lineages_sha256,
            full,
            no_phase,
            shuffled_phase,
            magnitude_only,
            random_center,
            strict_all_ablation_pass,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        proof.proof_root_sha256 = proof.expected_root()?;
        proof.validate()?;
        Ok(proof)
    }

    pub fn validate(&self) -> Result<(), String> {
        let positive_rows =
            u64::try_from(self.positive_precommit_roots_sha256.len()).unwrap_or(u64::MAX);
        let negative_rows =
            u64::try_from(self.negative_precommit_roots_sha256.len()).unwrap_or(u64::MAX);
        let total_rows = positive_rows.saturating_add(negative_rows);
        let roots = [
            &self.positive_precommit_roots_sha256,
            &self.positive_completion_roots_sha256,
            &self.negative_precommit_roots_sha256,
            &self.negative_binding_roots_sha256,
            &self.independent_lineages_sha256,
        ];
        let valid_score = |score: &Ms4ExactWaveControlScoreV1, mode: ResponsePhaseControlV1| {
            score.mode == mode.label()
                && score.correct_rows.saturating_add(score.wrong_rows) == total_rows
        };
        if self.schema != PROOF_SCHEMA
            || !valid_nonzero_sha256(&self.proof_root_sha256)
            || !valid_nonzero_sha256(&self.contract_root_sha256)
            || !valid_nonzero_sha256(&self.candidate_root_sha256)
            || self.package_id.is_empty()
            || !valid_nonzero_sha256(&self.execution_payload_sha256)
            || self.center_training_max_sequence == 0
            || self.holdout_min_sequence <= self.center_training_max_sequence
            || positive_rows == 0
            || negative_rows == 0
            || self.positive_completion_roots_sha256.len()
                != self.positive_precommit_roots_sha256.len()
            || self.negative_binding_roots_sha256.len()
                != self.negative_precommit_roots_sha256.len()
            || u64::try_from(self.independent_lineages_sha256.len()).unwrap_or(0)
                < MIN_INDEPENDENT_LINEAGES
            || roots.iter().any(|values| {
                values.iter().any(|root| !valid_nonzero_sha256(root))
                    || !values.windows(2).all(|pair| pair[0] < pair[1])
            })
            || !valid_score(&self.full, ResponsePhaseControlV1::Full)
            || !valid_score(&self.no_phase, ResponsePhaseControlV1::NoPhase)
            || !valid_score(&self.shuffled_phase, ResponsePhaseControlV1::ShuffledPhase)
            || !valid_score(&self.magnitude_only, ResponsePhaseControlV1::MagnitudeOnly)
            || !valid_score(&self.random_center, ResponsePhaseControlV1::RandomCenter)
            || self.full.correct_rows != total_rows
            || self.full.wrong_rows != 0
            || !self.strict_all_ablation_pass
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.proof_root_sha256 != self.expected_root()?
        {
            return Err("ms4_independent_exact_wave_proof_invalid".to_owned());
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, String> {
        canonical_json_sha256(&(
            PROOF_SCHEMA,
            (
                self.contract_root_sha256.as_str(),
                self.candidate_root_sha256.as_str(),
                self.package_id.as_str(),
                self.execution_payload_sha256.as_str(),
                self.center_training_max_sequence,
                self.holdout_min_sequence,
            ),
            (
                &self.positive_precommit_roots_sha256,
                &self.positive_completion_roots_sha256,
                &self.negative_precommit_roots_sha256,
                &self.negative_binding_roots_sha256,
                &self.independent_lineages_sha256,
            ),
            (
                &self.full,
                &self.no_phase,
                &self.shuffled_phase,
                &self.magnitude_only,
                &self.random_center,
            ),
            true,
            false,
            false,
        ))
        .map_err(str::to_owned)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Ms4ExactWaveControlPredictionV1 {
    pub mode: ResponsePhaseControlV1,
    pub executed: bool,
    pub reason: String,
    pub response_sha256: Option<String>,
    pub phase_margin_micro: Option<i64>,
    pub exact_actor_checks: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Ms4ExactWavePrecommitV1 {
    pub schema: String,
    pub precommit_root_sha256: String,
    pub contract_root_sha256: String,
    pub candidate_root_sha256: String,
    pub package_id: String,
    pub request_event_id_sha256: String,
    pub economics_intent_sha256: String,
    pub turn_intent_id_sha256: String,
    pub request_sha256: String,
    pub predicted_at_unix_nanos: u64,
    pub controls: Vec<Ms4ExactWaveControlPredictionV1>,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

impl Ms4ExactWavePrecommitV1 {
    fn seal(
        contract: &Ms4ExactWaveHoldoutContractV1,
        request_event_id_sha256: &str,
        economics_intent_sha256: &str,
        turn_intent_id_sha256: &str,
        request_sha256: &str,
        predicted_at_unix_nanos: u64,
        controls: Vec<Ms4ExactWaveControlPredictionV1>,
    ) -> Result<Self, String> {
        let mut receipt = Self {
            schema: PRECOMMIT_SCHEMA.to_owned(),
            precommit_root_sha256: String::new(),
            contract_root_sha256: contract.contract_root_sha256.clone(),
            candidate_root_sha256: contract.candidate_root_sha256.clone(),
            package_id: contract.package_id.clone(),
            request_event_id_sha256: request_event_id_sha256.to_owned(),
            economics_intent_sha256: economics_intent_sha256.to_owned(),
            turn_intent_id_sha256: turn_intent_id_sha256.to_owned(),
            request_sha256: request_sha256.to_owned(),
            predicted_at_unix_nanos,
            controls,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        receipt.precommit_root_sha256 = receipt.expected_root()?;
        receipt.validate(contract)?;
        Ok(receipt)
    }

    pub fn validate(&self, contract: &Ms4ExactWaveHoldoutContractV1) -> Result<(), String> {
        let modes = self
            .controls
            .iter()
            .map(|control| control.mode)
            .collect::<Vec<_>>();
        let expected_modes = [
            ResponsePhaseControlV1::Full,
            ResponsePhaseControlV1::NoPhase,
            ResponsePhaseControlV1::ShuffledPhase,
            ResponsePhaseControlV1::MagnitudeOnly,
            ResponsePhaseControlV1::RandomCenter,
        ];
        let controls_valid = self.controls.iter().all(|control| {
            !control.reason.is_empty()
                && control.reason.len() <= 512
                && control.exact_actor_checks <= 1
                && match (control.executed, control.response_sha256.as_deref()) {
                    (true, Some(root)) => valid_nonzero_sha256(root),
                    (false, None) => true,
                    _ => false,
                }
        });
        if self.schema != PRECOMMIT_SCHEMA
            || !valid_nonzero_sha256(&self.precommit_root_sha256)
            || self.contract_root_sha256 != contract.contract_root_sha256
            || self.candidate_root_sha256 != contract.candidate_root_sha256
            || self.package_id != contract.package_id
            || !valid_nonzero_sha256(&self.request_event_id_sha256)
            || !valid_nonzero_sha256(&self.economics_intent_sha256)
            || self.turn_intent_id_sha256.is_empty()
            || !valid_nonzero_sha256(&self.request_sha256)
            || self.predicted_at_unix_nanos / 1_000_000_000 < contract.opened_at_unix
            || modes != expected_modes
            || !controls_valid
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.precommit_root_sha256 != self.expected_root()?
        {
            return Err("ms4_exact_wave_precommit_invalid".to_owned());
        }
        Ok(())
    }

    pub fn control(
        &self,
        mode: ResponsePhaseControlV1,
    ) -> Option<&Ms4ExactWaveControlPredictionV1> {
        self.controls.iter().find(|control| control.mode == mode)
    }

    fn expected_root(&self) -> Result<String, String> {
        canonical_json_sha256(&(
            PRECOMMIT_SCHEMA,
            self.contract_root_sha256.as_str(),
            self.candidate_root_sha256.as_str(),
            self.package_id.as_str(),
            self.request_event_id_sha256.as_str(),
            self.economics_intent_sha256.as_str(),
            self.turn_intent_id_sha256.as_str(),
            self.request_sha256.as_str(),
            self.predicted_at_unix_nanos,
            &self.controls,
            false,
            false,
        ))
        .map_err(str::to_owned)
    }
}

pub(super) struct Ms4ExactWavePrecommitWriter {
    root: PathBuf,
    ledger: FramedCborLedger,
    seen_request_events_by_contract: BTreeMap<String, BTreeSet<String>>,
}

impl Ms4ExactWavePrecommitWriter {
    pub fn open(ms4_root: &Path) -> Result<Self, String> {
        let root = ms4_root.join(ROOT_DIR);
        let ledger_dir = root.join(PRECOMMIT_DIR);
        let existing = if ledger_dir.exists() {
            read_framed_cbor::<Ms4ExactWavePrecommitV1>(&ledger_dir, PRECOMMIT_PREFIX)?
        } else {
            Vec::new()
        };
        let mut seen_request_events_by_contract = BTreeMap::<String, BTreeSet<String>>::new();
        for receipt in existing {
            let inserted = seen_request_events_by_contract
                .entry(receipt.contract_root_sha256)
                .or_default()
                .insert(receipt.request_event_id_sha256);
            if !inserted {
                return Err("ms4_exact_wave_precommit_duplicate_request".to_owned());
            }
        }
        let ledger =
            FramedCborLedger::open_with_limits(&ledger_dir, PRECOMMIT_PREFIX, 64 * 1024 * 1024, 1)?;
        Ok(Self {
            root,
            ledger,
            seen_request_events_by_contract,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_pre_action(
        &mut self,
        executor: &ResponseExecutor,
        request_event_id_sha256: &str,
        economics_intent_sha256: &str,
        turn_intent_id_sha256: &str,
        request_sha256: &str,
        request_text: &str,
        provider_payload: &Value,
        predicted_at_unix_nanos: u64,
    ) -> Result<Option<Ms4ExactWavePrecommitV1>, String> {
        let Some(contract) = read_active_contract(&self.root)? else {
            return Ok(None);
        };
        let seen_request_events = self
            .seen_request_events_by_contract
            .entry(contract.contract_root_sha256.clone())
            .or_default();
        if seen_request_events.len() >= MAX_PRECOMMITS
            || seen_request_events.contains(request_event_id_sha256)
        {
            return Ok(None);
        }
        if predicted_at_unix_nanos / 1_000_000_000 > contract.deadline_unix {
            return Ok(None);
        }
        let full = control_prediction(
            executor,
            &contract.package_id,
            ResponsePhaseControlV1::Full,
            request_text,
            provider_payload,
        )?;
        let no_phase = control_prediction(
            executor,
            &contract.package_id,
            ResponsePhaseControlV1::NoPhase,
            request_text,
            provider_payload,
        )?;
        if !full.executed && !no_phase.executed {
            return Ok(None);
        }
        let mut controls = vec![full, no_phase];
        for mode in [
            ResponsePhaseControlV1::ShuffledPhase,
            ResponsePhaseControlV1::MagnitudeOnly,
            ResponsePhaseControlV1::RandomCenter,
        ] {
            controls.push(control_prediction(
                executor,
                &contract.package_id,
                mode,
                request_text,
                provider_payload,
            )?);
        }
        let receipt = Ms4ExactWavePrecommitV1::seal(
            &contract,
            request_event_id_sha256,
            economics_intent_sha256,
            turn_intent_id_sha256,
            request_sha256,
            predicted_at_unix_nanos,
            controls,
        )?;
        self.ledger.append(&receipt)?;
        self.ledger.sync()?;
        seen_request_events.insert(request_event_id_sha256.to_owned());
        Ok(Some(receipt))
    }
}

pub(super) fn ensure_holdout_contract(
    ms4_root: &Path,
    candidate_root_sha256: &str,
    package_id: &str,
    execution_payload_sha256: &str,
    center_training_max_sequence: u64,
    contract_watermark: u64,
    opened_at_unix: u64,
) -> Result<Ms4ExactWaveHoldoutContractV1, String> {
    let root = ms4_root.join(ROOT_DIR);
    if let Some(existing) = read_active_contract(&root)?
        && existing.candidate_root_sha256 == candidate_root_sha256
        && existing.package_id == package_id
        && existing.execution_payload_sha256 == execution_payload_sha256
    {
        return Ok(existing);
    }
    let contracts_dir = root.join("contracts");
    if !root.join(ACTIVE_CONTRACT_FILE).exists()
        && contracts_dir.exists()
        && fs::read_dir(&contracts_dir)
            .map_err(|error| format!("ms4_exact_wave_contract_scan:{error}"))?
            .next()
            .is_some()
    {
        return Err("ms4_exact_wave_active_contract_missing".to_owned());
    }
    let contract = Ms4ExactWaveHoldoutContractV1::seal(
        candidate_root_sha256,
        package_id,
        execution_payload_sha256,
        center_training_max_sequence,
        contract_watermark.max(center_training_max_sequence),
        opened_at_unix,
    )?;
    let contract_path = root
        .join("contracts")
        .join(format!("{}.cbor", contract.contract_root_sha256));
    write_atomic_cbor(&contract_path, &contract)?;
    write_atomic_cbor(&root.join(ACTIVE_CONTRACT_FILE), &contract)?;
    Ok(contract)
}

pub(super) fn read_precommits(
    ms4_root: &Path,
    contract: &Ms4ExactWaveHoldoutContractV1,
) -> Result<Vec<Ms4ExactWavePrecommitV1>, String> {
    let directory = ms4_root.join(ROOT_DIR).join(PRECOMMIT_DIR);
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut receipts = read_framed_cbor::<Ms4ExactWavePrecommitV1>(&directory, PRECOMMIT_PREFIX)?
        .into_iter()
        .filter(|receipt| receipt.contract_root_sha256 == contract.contract_root_sha256)
        .collect::<Vec<_>>();
    for receipt in &receipts {
        receipt.validate(contract)?;
    }
    receipts.sort_by(|left, right| {
        left.predicted_at_unix_nanos
            .cmp(&right.predicted_at_unix_nanos)
            .then_with(|| left.precommit_root_sha256.cmp(&right.precommit_root_sha256))
    });
    let mut by_request = BTreeMap::new();
    for receipt in &receipts {
        if by_request
            .insert(
                receipt.request_event_id_sha256.as_str(),
                receipt.precommit_root_sha256.as_str(),
            )
            .is_some()
        {
            return Err("ms4_exact_wave_precommit_request_rebound".to_owned());
        }
    }
    Ok(receipts)
}

pub(super) fn evaluate_holdout(
    state: &AppState,
    candidate: &Ms4ExternalAdmissionCandidateV1,
    package: &ResponsePackage,
    now_unix: u64,
) -> Result<Ms4ExactWaveHoldoutEvaluationV1, String> {
    let topology_archive = state
        .multi_source_topology_archive
        .as_ref()
        .ok_or_else(|| "ms4_exact_wave_topology_archive_missing".to_owned())?;
    let contract_watermark = topology_archive
        .lock()
        .map_err(|_| "ms4_exact_wave_topology_archive_lock_poisoned".to_owned())?
        .max_bridge_sequence();
    let execution_payload_sha256 =
        response_execution_payload_digest(package).map_err(str::to_owned)?;
    let contract = ensure_holdout_contract(
        &state.config.ms4_closed_loop_path,
        candidate.candidate_root_sha256(),
        &package.package_id,
        &execution_payload_sha256,
        candidate.center_training_max_sequence(),
        contract_watermark,
        now_unix,
    )?;
    let training_lineages = candidate.center_training_lineages();
    let mut post_center_rows = topology_archive
        .lock()
        .map_err(|_| "ms4_exact_wave_topology_archive_lock_poisoned".to_owned())?
        .rows()
        .into_iter()
        .filter(|row| {
            row.bridge_sequence
                .is_some_and(|sequence| sequence >= contract.holdout_min_sequence)
                && row
                    .captured_at_unix_ms
                    .is_some_and(|captured| captured / 1_000 <= contract.deadline_unix)
                && row.structure.provider_bound_turn_identity
                && row.physical_order_proven
        })
        .collect::<Vec<_>>();
    post_center_rows.sort_by_key(|row| row.bridge_sequence.unwrap_or(u64::MAX));
    let scanned_topology_rows = u64::try_from(post_center_rows.len()).unwrap_or(u64::MAX);
    let training_lineage_reuse_excluded_rows = post_center_rows
        .iter()
        .filter(|row| {
            row.session_lineage_sha256
                .as_ref()
                .is_some_and(|lineage| training_lineages.contains(lineage))
        })
        .count();
    let independent_rows = post_center_rows
        .into_iter()
        .filter(|row| {
            row.session_lineage_sha256
                .as_ref()
                .is_some_and(|lineage| !training_lineages.contains(lineage))
        })
        .take(usize::try_from(contract.max_holdout_rows).unwrap_or(usize::MAX))
        .collect::<Vec<_>>();
    let denominator_closed_at_unix = if independent_rows.len()
        >= usize::try_from(contract.max_holdout_rows).unwrap_or(usize::MAX)
    {
        independent_rows
            .last()
            .and_then(|row| row.captured_at_unix_ms)
            .map_or(contract.deadline_unix, |captured| captured / 1_000)
            .min(contract.deadline_unix)
    } else {
        contract.deadline_unix
    };
    let denominator_closed = u64::try_from(independent_rows.len()).unwrap_or(u64::MAX)
        >= contract.max_holdout_rows
        || now_unix >= contract.deadline_unix;
    let settlement_closed = denominator_closed
        && now_unix >= denominator_closed_at_unix.saturating_add(contract.settlement_grace_seconds);

    let precommits = read_precommits(&state.config.ms4_closed_loop_path, &contract)?;
    let precommits_by_request = precommits
        .iter()
        .map(|receipt| (receipt.request_event_id_sha256.as_str(), receipt))
        .collect::<BTreeMap<_, _>>();
    let completions = durable_package_completions(
        &state.config.ms4_ordinary_economics_path,
        &package.package_id,
    )?;
    let completions_by_intent = completions
        .iter()
        .filter(|receipt| receipt.accepted_at_unix >= contract.opened_at_unix)
        .map(|receipt| (receipt.intent_sha256.as_str(), receipt))
        .collect::<BTreeMap<_, _>>();
    let request_ids = independent_rows
        .iter()
        .map(|row| row.structure.request_event_id_sha256.clone())
        .collect::<BTreeSet<_>>();
    let intent_ids = independent_rows
        .iter()
        .map(|row| row.structure.turn_intent_id_sha256.clone())
        .collect::<BTreeSet<_>>();
    let terminals = state
        .terminal_receipt_archive
        .as_ref()
        .ok_or_else(|| "ms4_exact_wave_terminal_archive_missing".to_owned())?
        .lock()
        .map_err(|_| "ms4_exact_wave_terminal_archive_lock_poisoned".to_owned())?
        .receipts_for_requests(&request_ids);
    let frames = state
        .multi_source_frame_archive
        .as_ref()
        .ok_or_else(|| "ms4_exact_wave_frame_archive_missing".to_owned())?
        .lock()
        .map_err(|_| "ms4_exact_wave_frame_archive_lock_poisoned".to_owned())?
        .frames_for_intents(&intent_ids);
    let transport = nando_operator_learning::multi_source::TransportBindingLedgerV1::build(
        &independent_rows,
        &frames,
        &terminals,
    );
    let (route_receipts, parity_by_frame) = {
        let spool = state
            .remote_evidence_spool
            .as_ref()
            .ok_or_else(|| "ms4_exact_wave_remote_evidence_spool_missing".to_owned())?
            .lock()
            .map_err(|_| "ms4_exact_wave_remote_evidence_spool_lock_poisoned".to_owned())?;
        let route_receipts = spool.route_receipts_by_frame_root();
        let parity_by_frame = frames
            .iter()
            .filter_map(|frame| {
                let root = canonical_json_sha256(frame).ok()?;
                spool
                    .runtime_parity_for_frame(&root)
                    .map(|parity| (root, parity))
            })
            .collect::<BTreeMap<_, _>>();
        (route_receipts, parity_by_frame)
    };

    let mut scores = ResponsePhaseControlV1::ALL
        .into_iter()
        .map(|mode| {
            (
                mode,
                Ms4ExactWaveControlScoreV1 {
                    mode: mode.label().to_owned(),
                    correct_rows: 0,
                    wrong_rows: 0,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut positive_precommits = Vec::new();
    let mut positive_completions = Vec::new();
    let mut negative_precommits = Vec::new();
    let mut negative_bindings = Vec::new();
    let mut proof_lineages = BTreeSet::new();
    let mut precommit_missing_rows = 0_u64;
    let mut precommit_disqualified_rows = 0_u64;
    let mut settlement_pending_rows = 0_u64;
    let mut precommitted_rows = 0_u64;
    let mut settled_rows = 0_u64;

    for row in &independent_rows {
        let Some(precommit) = precommits_by_request
            .get(row.structure.request_event_id_sha256.as_str())
            .copied()
            .filter(|receipt| {
                receipt.turn_intent_id_sha256 == row.structure.turn_intent_id_sha256
                    && receipt.request_sha256 == row.structure.provider_capture_request_root_sha256
            })
        else {
            precommit_missing_rows = precommit_missing_rows.saturating_add(1);
            continue;
        };
        precommitted_rows = precommitted_rows.saturating_add(1);
        let full = precommit
            .control(ResponsePhaseControlV1::Full)
            .ok_or_else(|| "ms4_exact_wave_full_prediction_missing".to_owned())?;
        let no_phase = precommit
            .control(ResponsePhaseControlV1::NoPhase)
            .ok_or_else(|| "ms4_exact_wave_no_phase_prediction_missing".to_owned())?;
        if full.executed {
            let Some(completion) = completions_by_intent
                .get(precommit.economics_intent_sha256.as_str())
                .copied()
            else {
                settlement_pending_rows = settlement_pending_rows.saturating_add(1);
                continue;
            };
            let matching_bounds = transport
                .bound_for_topology(&row.commit.commitment_root_sha256)
                .iter()
                .filter(|bound| {
                    bound.binding.request_event_id_sha256 == row.structure.request_event_id_sha256
                        && bound.binding.turn_intent_id_sha256
                            == row.structure.turn_intent_id_sha256
                        && row.session_lineage_sha256.as_ref()
                            == Some(&bound.binding.session_lineage_sha256)
                })
                .cloned()
                .collect::<Vec<_>>();
            let Some(bound) = (matching_bounds.len() == 1).then(|| matching_bounds[0].clone())
            else {
                settlement_pending_rows = settlement_pending_rows.saturating_add(1);
                continue;
            };
            let frame_root = &bound.binding.completed_frame_root_sha256;
            let route_bound = route_receipts.get(frame_root).is_some_and(|receipt| {
                receipt.remote_status == 200
                    && receipt.request_body_sha256
                        == row.structure.provider_capture_request_root_sha256
                    && receipt.turn_intent_id_sha256 == row.structure.turn_intent_id_sha256
            });
            let Some(parity) = parity_by_frame.get(frame_root).filter(|_| route_bound) else {
                settlement_pending_rows = settlement_pending_rows.saturating_add(1);
                continue;
            };
            if precommit.predicted_at_unix_nanos >= bound.binding.action_observed_at_unix_nanos
                || precommit.predicted_at_unix_nanos
                    >= bound.binding.request_completed_at_unix_nanos
            {
                precommit_disqualified_rows = precommit_disqualified_rows.saturating_add(1);
                continue;
            }
            let Some(expected_response_sha256) = full.response_sha256.as_deref() else {
                return Err("ms4_exact_wave_full_response_missing".to_owned());
            };
            if canonical_json_sha256(&parity.expected_response).map_err(str::to_owned)?
                != expected_response_sha256
            {
                continue;
            }
            score_positive(&mut scores, precommit, expected_response_sha256)?;
            positive_precommits.push(precommit.precommit_root_sha256.clone());
            positive_completions.push(completion.completion_root_sha256.clone());
            if let Some(lineage) = &row.session_lineage_sha256 {
                proof_lineages.insert(lineage.clone());
            }
            settled_rows = settled_rows.saturating_add(1);
            continue;
        }
        if !no_phase.executed {
            continue;
        }
        let matching_bounds = transport
            .bound_for_topology(&row.commit.commitment_root_sha256)
            .iter()
            .filter(|bound| {
                bound.binding.request_event_id_sha256 == row.structure.request_event_id_sha256
                    && bound.binding.turn_intent_id_sha256 == row.structure.turn_intent_id_sha256
                    && row.session_lineage_sha256.as_ref()
                        == Some(&bound.binding.session_lineage_sha256)
            })
            .cloned()
            .collect::<Vec<_>>();
        let Some(bound) = (matching_bounds.len() == 1).then(|| matching_bounds[0].clone()) else {
            settlement_pending_rows = settlement_pending_rows.saturating_add(1);
            continue;
        };
        let frame_root = &bound.binding.completed_frame_root_sha256;
        let route_bound = route_receipts.get(frame_root).is_some_and(|receipt| {
            receipt.remote_status == 418
                && receipt.request_body_sha256 == row.structure.provider_capture_request_root_sha256
                && receipt.turn_intent_id_sha256 == row.structure.turn_intent_id_sha256
        });
        let Some(parity) = parity_by_frame.get(frame_root).filter(|_| route_bound) else {
            settlement_pending_rows = settlement_pending_rows.saturating_add(1);
            continue;
        };
        if precommit.predicted_at_unix_nanos >= bound.binding.action_observed_at_unix_nanos
            || precommit.predicted_at_unix_nanos >= bound.binding.request_completed_at_unix_nanos
        {
            precommit_disqualified_rows = precommit_disqualified_rows.saturating_add(1);
            continue;
        }
        settled_rows = settled_rows.saturating_add(1);
        let expected_response_sha256 =
            canonical_json_sha256(&parity.expected_response).map_err(str::to_owned)?;
        if no_phase.response_sha256.as_deref() == Some(expected_response_sha256.as_str()) {
            continue;
        }
        score_negative(&mut scores, precommit, &expected_response_sha256)?;
        negative_precommits.push(precommit.precommit_root_sha256.clone());
        negative_bindings.push(bound.binding.binding_root_sha256.clone());
        if let Some(lineage) = &row.session_lineage_sha256 {
            proof_lineages.insert(lineage.clone());
        }
    }

    sort_dedup(&mut positive_precommits);
    sort_dedup(&mut positive_completions);
    sort_dedup(&mut negative_precommits);
    sort_dedup(&mut negative_bindings);
    let independent_lineages = proof_lineages.into_iter().collect::<Vec<_>>();
    let proof_rows = u64::try_from(
        positive_precommits
            .len()
            .saturating_add(negative_precommits.len()),
    )
    .unwrap_or(u64::MAX);
    let ablation_pass = exact_ablation_pass(&scores, proof_rows)?;
    let proof = if !positive_precommits.is_empty()
        && !negative_precommits.is_empty()
        && u64::try_from(independent_lineages.len()).unwrap_or(0) >= MIN_INDEPENDENT_LINEAGES
        && ablation_pass
    {
        Some(Ms4IndependentExactWaveProofV1::seal(
            &contract,
            positive_precommits.clone(),
            positive_completions.clone(),
            negative_precommits.clone(),
            negative_bindings.clone(),
            independent_lineages.clone(),
            scores,
        )?)
    } else {
        None
    };
    if let Some(proof) = &proof {
        persist_proof(&state.config.ms4_closed_loop_path, proof)?;
    }
    let (status, blocker) = if proof.is_some() {
        (Ms4ExactWaveHoldoutStatusV1::Pass, String::new())
    } else if !denominator_closed || (settlement_pending_rows > 0 && !settlement_closed) {
        (
            Ms4ExactWaveHoldoutStatusV1::Collecting,
            "post_center_holdout_collecting".to_owned(),
        )
    } else if positive_precommits.is_empty()
        || negative_precommits.is_empty()
        || u64::try_from(independent_lineages.len()).unwrap_or(0) < MIN_INDEPENDENT_LINEAGES
    {
        (
            Ms4ExactWaveHoldoutStatusV1::AcquisitionFail,
            "post_center_phase_holdout_acquisition_fail".to_owned(),
        )
    } else {
        (
            Ms4ExactWaveHoldoutStatusV1::Fail,
            "independent_exact_package_ablation_failed".to_owned(),
        )
    };
    Ok(Ms4ExactWaveHoldoutEvaluationV1 {
        schema: EVALUATION_SCHEMA.to_owned(),
        contract_root_sha256: contract.contract_root_sha256,
        status,
        blocker,
        scanned_topology_rows,
        independent_topology_rows: u64::try_from(independent_rows.len()).unwrap_or(u64::MAX),
        precommitted_rows,
        settled_rows,
        positive_holdout_rows: u64::try_from(positive_precommits.len()).unwrap_or(u64::MAX),
        phase_challenging_negative_rows: u64::try_from(negative_precommits.len())
            .unwrap_or(u64::MAX),
        precommit_missing_rows,
        precommit_disqualified_rows,
        settlement_pending_rows,
        training_lineage_reuse_excluded_rows: u64::try_from(training_lineage_reuse_excluded_rows)
            .unwrap_or(u64::MAX),
        independent_lineages: u64::try_from(independent_lineages.len()).unwrap_or(u64::MAX),
        proof,
    })
}

fn exact_ablation_pass(
    scores: &BTreeMap<ResponsePhaseControlV1, Ms4ExactWaveControlScoreV1>,
    proof_rows: u64,
) -> Result<bool, String> {
    let full = scores
        .get(&ResponsePhaseControlV1::Full)
        .ok_or_else(|| "ms4_exact_wave_full_score_missing".to_owned())?;
    if proof_rows == 0 || full.correct_rows != proof_rows || full.wrong_rows != 0 {
        return Ok(false);
    }
    Ok([
        ResponsePhaseControlV1::NoPhase,
        ResponsePhaseControlV1::ShuffledPhase,
        ResponsePhaseControlV1::MagnitudeOnly,
        ResponsePhaseControlV1::RandomCenter,
    ]
    .into_iter()
    .all(|mode| {
        scores
            .get(&mode)
            .is_some_and(|score| score.correct_rows < full.correct_rows)
    }))
}

fn score_positive(
    scores: &mut BTreeMap<ResponsePhaseControlV1, Ms4ExactWaveControlScoreV1>,
    precommit: &Ms4ExactWavePrecommitV1,
    expected_response_sha256: &str,
) -> Result<(), String> {
    for mode in ResponsePhaseControlV1::ALL {
        let prediction = precommit
            .control(mode)
            .ok_or_else(|| "ms4_exact_wave_control_prediction_missing".to_owned())?;
        add_score(
            scores,
            mode,
            prediction.executed
                && prediction.response_sha256.as_deref() == Some(expected_response_sha256),
        )?;
    }
    Ok(())
}

fn score_negative(
    scores: &mut BTreeMap<ResponsePhaseControlV1, Ms4ExactWaveControlScoreV1>,
    precommit: &Ms4ExactWavePrecommitV1,
    expected_response_sha256: &str,
) -> Result<(), String> {
    for mode in ResponsePhaseControlV1::ALL {
        let prediction = precommit
            .control(mode)
            .ok_or_else(|| "ms4_exact_wave_control_prediction_missing".to_owned())?;
        add_score(
            scores,
            mode,
            !prediction.executed
                || prediction.response_sha256.as_deref() == Some(expected_response_sha256),
        )?;
    }
    Ok(())
}

fn add_score(
    scores: &mut BTreeMap<ResponsePhaseControlV1, Ms4ExactWaveControlScoreV1>,
    mode: ResponsePhaseControlV1,
    correct: bool,
) -> Result<(), String> {
    let score = scores
        .get_mut(&mode)
        .ok_or_else(|| "ms4_exact_wave_control_score_missing".to_owned())?;
    if correct {
        score.correct_rows = score.correct_rows.saturating_add(1);
    } else {
        score.wrong_rows = score.wrong_rows.saturating_add(1);
    }
    Ok(())
}

fn sort_dedup(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn persist_proof(root: &Path, proof: &Ms4IndependentExactWaveProofV1) -> Result<(), String> {
    proof.validate()?;
    let path = root
        .join("independent-exact-wave-proofs-v1")
        .join(format!("{}.cbor", proof.proof_root_sha256));
    if path.exists() {
        let restored: Ms4IndependentExactWaveProofV1 = serde_cbor::from_slice(
            &fs::read(&path).map_err(|error| format!("ms4_exact_wave_proof_read:{error}"))?,
        )
        .map_err(|error| format!("ms4_exact_wave_proof_decode:{error}"))?;
        restored.validate()?;
        return (restored == *proof)
            .then_some(())
            .ok_or_else(|| "ms4_exact_wave_proof_rebound".to_owned());
    }
    write_atomic_cbor(&path, proof)?;
    let restored: Ms4IndependentExactWaveProofV1 = serde_cbor::from_slice(
        &fs::read(&path).map_err(|error| format!("ms4_exact_wave_proof_verify_read:{error}"))?,
    )
    .map_err(|error| format!("ms4_exact_wave_proof_verify_decode:{error}"))?;
    restored.validate()?;
    (restored == *proof)
        .then_some(())
        .ok_or_else(|| "ms4_exact_wave_proof_restart_parity_mismatch".to_owned())
}

fn read_active_contract(root: &Path) -> Result<Option<Ms4ExactWaveHoldoutContractV1>, String> {
    let bytes = match fs::read(root.join(ACTIVE_CONTRACT_FILE)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("ms4_exact_wave_contract_read:{error}")),
    };
    let contract: Ms4ExactWaveHoldoutContractV1 = serde_cbor::from_slice(&bytes)
        .map_err(|error| format!("ms4_exact_wave_contract_decode:{error}"))?;
    contract.validate()?;
    Ok(Some(contract))
}

fn control_prediction(
    executor: &ResponseExecutor,
    package_id: &str,
    mode: ResponsePhaseControlV1,
    request_text: &str,
    provider_payload: &Value,
) -> Result<Ms4ExactWaveControlPredictionV1, String> {
    let execution =
        executor.execute_package_control_shadow(package_id, mode, request_text, provider_payload);
    let executed = execution.status == ResponseExecutionStatus::Executed;
    let response_sha256 = execution
        .response
        .as_ref()
        .map(canonical_json_sha256)
        .transpose()
        .map_err(str::to_owned)?;
    Ok(Ms4ExactWaveControlPredictionV1 {
        mode,
        executed,
        reason: execution.reason.chars().take(512).collect(),
        response_sha256,
        phase_margin_micro: execution.phase_margin_micro,
        exact_actor_checks: u64::try_from(execution.exact_actor_checks).unwrap_or(u64::MAX),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn holdout_contract_uses_a_post_freeze_watermark() {
        let root = std::env::temp_dir().join(format!(
            "nando-ms4-exact-wave-contract-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let contract = ensure_holdout_contract(
            &root,
            &"a".repeat(64),
            "package-a",
            &"b".repeat(64),
            41,
            57,
            1_700_000_000,
        )
        .expect("contract");
        assert_eq!(contract.center_training_max_sequence, 41);
        assert_eq!(contract.contract_watermark, 57);
        assert_eq!(contract.holdout_min_sequence, 58);
        assert!(!contract.authority_ready);
        assert!(!contract.phase_mutation_allowed);
        assert_eq!(
            ensure_holdout_contract(
                &root,
                &"a".repeat(64),
                "package-a",
                &"b".repeat(64),
                41,
                99,
                1_700_000_100,
            )
            .expect("restored"),
            contract
        );
        std::fs::remove_file(root.join(ROOT_DIR).join(ACTIVE_CONTRACT_FILE))
            .expect("remove active contract");
        assert_eq!(
            ensure_holdout_contract(
                &root,
                &"a".repeat(64),
                "package-a",
                &"b".repeat(64),
                41,
                100,
                1_700_000_200,
            ),
            Err("ms4_exact_wave_active_contract_missing".to_owned())
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn independent_proof_requires_every_real_control_to_degrade() {
        let scores = ResponsePhaseControlV1::ALL
            .into_iter()
            .map(|mode| {
                let correct_rows = if mode == ResponsePhaseControlV1::Full {
                    2
                } else {
                    1
                };
                (
                    mode,
                    Ms4ExactWaveControlScoreV1 {
                        mode: mode.label().to_owned(),
                        correct_rows,
                        wrong_rows: 2 - correct_rows,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(exact_ablation_pass(&scores, 2), Ok(true));

        let mut unchanged = scores;
        unchanged
            .get_mut(&ResponsePhaseControlV1::NoPhase)
            .expect("no-phase")
            .correct_rows = 2;
        unchanged
            .get_mut(&ResponsePhaseControlV1::NoPhase)
            .expect("no-phase")
            .wrong_rows = 0;
        assert_eq!(exact_ablation_pass(&unchanged, 2), Ok(false));
    }

    #[test]
    fn precommit_restart_rejects_duplicate_request_identity() {
        let root = std::env::temp_dir().join(format!(
            "nando-ms4-exact-wave-precommit-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let contract = ensure_holdout_contract(
            &root,
            &"a".repeat(64),
            "package-a",
            &"b".repeat(64),
            41,
            57,
            1_700_000_000,
        )
        .expect("contract");
        let controls = ResponsePhaseControlV1::ALL
            .into_iter()
            .map(|mode| Ms4ExactWaveControlPredictionV1 {
                mode,
                executed: false,
                reason: "rejected".to_owned(),
                response_sha256: None,
                phase_margin_micro: None,
                exact_actor_checks: 0,
            })
            .collect();
        let receipt = Ms4ExactWavePrecommitV1::seal(
            &contract,
            &"c".repeat(64),
            &"c".repeat(64),
            &"d".repeat(64),
            &"e".repeat(64),
            1_700_000_000_000_000_001,
            controls,
        )
        .expect("precommit");
        let mut writer = Ms4ExactWavePrecommitWriter::open(&root).expect("writer");
        writer.ledger.append(&receipt).expect("first append");
        writer.ledger.append(&receipt).expect("duplicate append");
        writer.ledger.sync().expect("sync");
        drop(writer);
        assert!(
            Ms4ExactWavePrecommitWriter::open(&root)
                .is_err_and(|error| error == "ms4_exact_wave_precommit_duplicate_request")
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
