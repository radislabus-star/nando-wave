use std::collections::BTreeMap;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::{FrozenVersionSpaceEnvelopeV1, Ms3FrozenVersionSpaceStateV1};

pub const NORTH_STAR_PROOF_CONTRACT_SCHEMA_V1: &str = "nando.north-star-proof-contract.v1";
pub const NORTH_STAR_PROOF_REPORT_SCHEMA_V1: &str = "nando.north-star-proof-report.v1";
pub const NORTH_STAR_REQUIRED_SEEDS_V1: usize = 5;
pub const NORTH_STAR_MIN_PASSING_SEEDS_V1: usize = 4;
const MAX_NORTH_STAR_CONTRACT_BYTES: usize = 1024 * 1024;
const MAX_NORTH_STAR_REPORT_BYTES: usize = 16 * 1024 * 1024;

const REQUIRED_ARMS: [NorthStarProofArmV1; 7] = [
    NorthStarProofArmV1::CellularWaveEnsemble,
    NorthStarProofArmV1::EqualBudgetMonolith,
    NorthStarProofArmV1::ExactStructuralSearch,
    NorthStarProofArmV1::NoPhase,
    NorthStarProofArmV1::ShuffledPhase,
    NorthStarProofArmV1::MagnitudeOnly,
    NorthStarProofArmV1::RandomCenter,
];

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NorthStarProofArmV1 {
    CellularWaveEnsemble,
    EqualBudgetMonolith,
    ExactStructuralSearch,
    NoPhase,
    ShuffledPhase,
    MagnitudeOnly,
    RandomCenter,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NorthStarBudgetV1 {
    pub total_memory_bytes: u64,
    pub hot_memory_bytes: u64,
    pub max_support_rows: u64,
    pub max_future_rows: u64,
    pub max_exact_checks: u64,
    pub max_cpu_nanos: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NorthStarProofThresholdsV1 {
    pub min_primary_gain_milli: i64,
    pub min_key_ablation_drop_milli: i64,
    pub min_key_to_non_key_ratio_milli: i64,
    pub min_passing_seeds: usize,
    pub required_seeds: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NorthStarProofContractV1 {
    pub schema: String,
    pub contract_root_sha256: String,
    pub ms3_generation_sequence: u64,
    pub frozen_envelope_root_sha256: String,
    pub frozen_contract_root_sha256: String,
    pub support_rows_root_sha256: String,
    pub grammar_root_sha256: String,
    pub quotient_root_sha256: String,
    pub class_predictions_root_sha256: String,
    pub compiler_version: String,
    pub vm_abi: String,
    pub verifier_schema: String,
    pub future_min_sequence: u64,
    pub arms: Vec<NorthStarProofArmV1>,
    pub seeds: Vec<u64>,
    pub budget: NorthStarBudgetV1,
    pub thresholds: NorthStarProofThresholdsV1,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NorthStarArmMetricsV1 {
    pub arm: NorthStarProofArmV1,
    pub budget_root_sha256: String,
    pub experiment_report_root_sha256: String,
    pub future_rows_root_sha256: String,
    pub snapshot_root_sha256: String,
    pub primary_score_milli: i64,
    pub future_rows: u64,
    pub correct_executions: u64,
    pub wrong_accepts: u64,
    pub runtime_parity_failures: u64,
    pub verifier_coverage_milli: u16,
    pub exact_checks: u64,
    pub memory_bytes: u64,
    pub cpu_nanos: u64,
    pub circuit_formed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NorthStarProofSeedReceiptV1 {
    pub seed: u64,
    pub receipt_root_sha256: String,
    pub arms: Vec<NorthStarArmMetricsV1>,
    pub delayed_transition_observed: bool,
    pub exact_memory_cleanup_observed: bool,
    pub key_ablation_drop_milli: i64,
    pub non_key_ablation_drop_milli: i64,
    pub snapshot_restore_exact: bool,
    pub snapshot_cold_start_gain_milli: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NorthStarSeedConditionsV1 {
    pub delayed_transition_observed: bool,
    pub exact_memory_cleanup_observed: bool,
    pub key_ablation_drop_milli: i64,
    pub non_key_ablation_drop_milli: i64,
    pub snapshot_restore_exact: bool,
    pub snapshot_cold_start_gain_milli: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NorthStarProofVerdictV1 {
    Pass,
    Watch,
    Fail,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NorthStarProofReportV1 {
    pub schema: String,
    pub report_root_sha256: String,
    pub contract_root_sha256: String,
    pub seed_receipts: Vec<NorthStarProofSeedReceiptV1>,
    pub passing_seeds: usize,
    pub support_future_overlap: u64,
    pub remote_restore_observed: bool,
    pub exact_authority_removed: bool,
    pub verdict: NorthStarProofVerdictV1,
    pub blocker: String,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NorthStarProofErrorV1 {
    InvalidFrozenContract,
    InvalidContract,
    InvalidSeedReceipt,
    Serialization,
}

impl Default for NorthStarProofThresholdsV1 {
    fn default() -> Self {
        Self {
            min_primary_gain_milli: 30,
            min_key_ablation_drop_milli: 50,
            min_key_to_non_key_ratio_milli: 2_000,
            min_passing_seeds: NORTH_STAR_MIN_PASSING_SEEDS_V1,
            required_seeds: NORTH_STAR_REQUIRED_SEEDS_V1,
        }
    }
}

impl NorthStarBudgetV1 {
    #[must_use]
    pub fn validate(&self) -> bool {
        self.total_memory_bytes > 0
            && self.hot_memory_bytes > 0
            && self.hot_memory_bytes <= self.total_memory_bytes
            && self.max_support_rows > 0
            && self.max_future_rows > 0
            && self.max_exact_checks > 0
            && self.max_cpu_nanos > 0
    }

    pub fn root_sha256(&self) -> Result<String, NorthStarProofErrorV1> {
        self.validate()
            .then_some(())
            .ok_or(NorthStarProofErrorV1::InvalidContract)?;
        canonical_json_sha256(&("nando.north-star-budget.v1", self))
            .map_err(|_| NorthStarProofErrorV1::Serialization)
    }
}

impl NorthStarProofContractV1 {
    pub fn seal(
        ms3_generation_sequence: u64,
        frozen: &FrozenVersionSpaceEnvelopeV1,
        seeds: Vec<u64>,
        budget: NorthStarBudgetV1,
    ) -> Result<Self, NorthStarProofErrorV1> {
        frozen
            .validate()
            .map_err(|_| NorthStarProofErrorV1::InvalidFrozenContract)?;
        if !matches!(
            frozen.contract.state,
            Ms3FrozenVersionSpaceStateV1::UniqueLawFrozen { .. }
        ) {
            return Err(NorthStarProofErrorV1::InvalidFrozenContract);
        }
        let mut contract = Self {
            schema: NORTH_STAR_PROOF_CONTRACT_SCHEMA_V1.to_owned(),
            contract_root_sha256: String::new(),
            ms3_generation_sequence,
            frozen_envelope_root_sha256: frozen.envelope_root_sha256.clone(),
            frozen_contract_root_sha256: frozen.contract.contract_root_sha256.clone(),
            support_rows_root_sha256: frozen.contract.support_rows_root_sha256.clone(),
            grammar_root_sha256: frozen.contract.grammar_root_sha256.clone(),
            quotient_root_sha256: frozen.contract.quotient_root_sha256.clone(),
            class_predictions_root_sha256: frozen.contract.class_predictions_root_sha256.clone(),
            compiler_version: frozen.contract.compiler_version.clone(),
            vm_abi: frozen.contract.vm_abi.clone(),
            verifier_schema: frozen.contract.verifier_schema.clone(),
            future_min_sequence: frozen.contract.future_min_sequence,
            arms: REQUIRED_ARMS.to_vec(),
            seeds,
            budget,
            thresholds: NorthStarProofThresholdsV1::default(),
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        contract.contract_root_sha256 = contract.expected_root()?;
        contract
            .validate()
            .then_some(contract)
            .ok_or(NorthStarProofErrorV1::InvalidContract)
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == NORTH_STAR_PROOF_CONTRACT_SCHEMA_V1
            && self.ms3_generation_sequence > 0
            && [
                &self.contract_root_sha256,
                &self.frozen_envelope_root_sha256,
                &self.frozen_contract_root_sha256,
                &self.support_rows_root_sha256,
                &self.grammar_root_sha256,
                &self.quotient_root_sha256,
                &self.class_predictions_root_sha256,
            ]
            .into_iter()
            .all(|root| valid_nonzero_sha256(root))
            && !self.compiler_version.is_empty()
            && !self.vm_abi.is_empty()
            && !self.verifier_schema.is_empty()
            && self.future_min_sequence > 0
            && self.arms == REQUIRED_ARMS
            && self.seeds.len() == NORTH_STAR_REQUIRED_SEEDS_V1
            && self.seeds.windows(2).all(|pair| pair[0] < pair[1])
            && self.budget.validate()
            && self.thresholds == NorthStarProofThresholdsV1::default()
            && !self.authority_ready
            && !self.phase_mutation_allowed
            && self
                .expected_root()
                .is_ok_and(|root| root == self.contract_root_sha256)
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, NorthStarProofErrorV1> {
        if !self.validate() {
            return Err(NorthStarProofErrorV1::InvalidContract);
        }
        let bytes = serde_cbor::to_vec(self).map_err(|_| NorthStarProofErrorV1::Serialization)?;
        if bytes.is_empty() || bytes.len() > MAX_NORTH_STAR_CONTRACT_BYTES {
            return Err(NorthStarProofErrorV1::Serialization);
        }
        Ok(bytes)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, NorthStarProofErrorV1> {
        if bytes.is_empty() || bytes.len() > MAX_NORTH_STAR_CONTRACT_BYTES {
            return Err(NorthStarProofErrorV1::InvalidContract);
        }
        let contract: Self =
            serde_cbor::from_slice(bytes).map_err(|_| NorthStarProofErrorV1::InvalidContract)?;
        if !contract.validate() || contract.canonical_bytes()? != bytes {
            return Err(NorthStarProofErrorV1::InvalidContract);
        }
        Ok(contract)
    }

    fn expected_root(&self) -> Result<String, NorthStarProofErrorV1> {
        canonical_json_sha256(&(
            NORTH_STAR_PROOF_CONTRACT_SCHEMA_V1,
            (
                self.ms3_generation_sequence,
                self.frozen_envelope_root_sha256.as_str(),
                self.frozen_contract_root_sha256.as_str(),
                self.support_rows_root_sha256.as_str(),
                self.grammar_root_sha256.as_str(),
                self.quotient_root_sha256.as_str(),
                self.class_predictions_root_sha256.as_str(),
                self.compiler_version.as_str(),
                self.vm_abi.as_str(),
                self.verifier_schema.as_str(),
                self.future_min_sequence,
            ),
            (
                &self.arms,
                &self.seeds,
                &self.budget,
                &self.thresholds,
                false,
                false,
            ),
        ))
        .map_err(|_| NorthStarProofErrorV1::Serialization)
    }
}

impl NorthStarProofSeedReceiptV1 {
    pub fn seal(
        contract: &NorthStarProofContractV1,
        seed: u64,
        arms: Vec<NorthStarArmMetricsV1>,
        conditions: NorthStarSeedConditionsV1,
    ) -> Result<Self, NorthStarProofErrorV1> {
        let mut receipt = Self {
            seed,
            receipt_root_sha256: String::new(),
            arms,
            delayed_transition_observed: conditions.delayed_transition_observed,
            exact_memory_cleanup_observed: conditions.exact_memory_cleanup_observed,
            key_ablation_drop_milli: conditions.key_ablation_drop_milli,
            non_key_ablation_drop_milli: conditions.non_key_ablation_drop_milli,
            snapshot_restore_exact: conditions.snapshot_restore_exact,
            snapshot_cold_start_gain_milli: conditions.snapshot_cold_start_gain_milli,
        };
        receipt.receipt_root_sha256 =
            seed_receipt_root(contract.contract_root_sha256.as_str(), &receipt)?;
        validate_seed_receipt(contract, &receipt)
            .then_some(receipt)
            .ok_or(NorthStarProofErrorV1::InvalidSeedReceipt)
    }

    #[must_use]
    pub fn validate(&self, contract: &NorthStarProofContractV1) -> bool {
        validate_seed_receipt(contract, self)
    }
}

impl NorthStarProofReportV1 {
    #[must_use]
    pub fn validate(&self, contract: &NorthStarProofContractV1) -> bool {
        if !contract.validate()
            || self.schema != NORTH_STAR_PROOF_REPORT_SCHEMA_V1
            || self.contract_root_sha256 != contract.contract_root_sha256
            || !valid_nonzero_sha256(&self.report_root_sha256)
            || self.authority_ready
            || self.phase_mutation_allowed
            || !self
                .seed_receipts
                .iter()
                .all(|receipt| receipt.validate(contract))
        {
            return false;
        }
        evaluate_north_star_proof_v1(
            contract,
            self.seed_receipts.clone(),
            self.support_future_overlap,
            self.remote_restore_observed,
            self.exact_authority_removed,
        )
        .is_ok_and(|expected| expected == *self)
    }

    pub fn canonical_bytes(
        &self,
        contract: &NorthStarProofContractV1,
    ) -> Result<Vec<u8>, NorthStarProofErrorV1> {
        if !self.validate(contract) {
            return Err(NorthStarProofErrorV1::InvalidSeedReceipt);
        }
        let bytes = serde_cbor::to_vec(self).map_err(|_| NorthStarProofErrorV1::Serialization)?;
        if bytes.is_empty() || bytes.len() > MAX_NORTH_STAR_REPORT_BYTES {
            return Err(NorthStarProofErrorV1::Serialization);
        }
        Ok(bytes)
    }

    pub fn from_canonical_bytes(
        bytes: &[u8],
        contract: &NorthStarProofContractV1,
    ) -> Result<Self, NorthStarProofErrorV1> {
        if bytes.is_empty() || bytes.len() > MAX_NORTH_STAR_REPORT_BYTES {
            return Err(NorthStarProofErrorV1::InvalidSeedReceipt);
        }
        let report: Self =
            serde_cbor::from_slice(bytes).map_err(|_| NorthStarProofErrorV1::InvalidSeedReceipt)?;
        if !report.validate(contract) || report.canonical_bytes(contract)? != bytes {
            return Err(NorthStarProofErrorV1::InvalidSeedReceipt);
        }
        Ok(report)
    }
}

pub fn evaluate_north_star_proof_v1(
    contract: &NorthStarProofContractV1,
    mut seed_receipts: Vec<NorthStarProofSeedReceiptV1>,
    support_future_overlap: u64,
    remote_restore_observed: bool,
    exact_authority_removed: bool,
) -> Result<NorthStarProofReportV1, NorthStarProofErrorV1> {
    if !contract.validate() {
        return Err(NorthStarProofErrorV1::InvalidContract);
    }
    seed_receipts.sort_by_key(|receipt| receipt.seed);
    if seed_receipts.len() > contract.seeds.len()
        || seed_receipts
            .windows(2)
            .any(|pair| pair[0].seed == pair[1].seed)
        || !seed_receipts
            .iter()
            .all(|receipt| validate_seed_receipt(contract, receipt))
    {
        return Err(NorthStarProofErrorV1::InvalidSeedReceipt);
    }
    let passing_seeds = seed_receipts
        .iter()
        .filter(|receipt| seed_passes(contract, receipt))
        .count();
    let complete = seed_receipts.len() == contract.seeds.len();
    let safe = seed_receipts.iter().all(seed_is_safe);
    let proof_pass = complete
        && safe
        && passing_seeds >= contract.thresholds.min_passing_seeds
        && support_future_overlap == 0
        && remote_restore_observed
        && exact_authority_removed;
    let (verdict, blocker) = if proof_pass {
        (NorthStarProofVerdictV1::Pass, String::new())
    } else if !safe {
        (
            NorthStarProofVerdictV1::Fail,
            "north_star_safety_failure".to_owned(),
        )
    } else if !complete {
        (
            NorthStarProofVerdictV1::Watch,
            "north_star_seed_receipts_pending".to_owned(),
        )
    } else if support_future_overlap > 0 {
        (
            NorthStarProofVerdictV1::Fail,
            "north_star_support_future_overlap".to_owned(),
        )
    } else if !remote_restore_observed {
        (
            NorthStarProofVerdictV1::Fail,
            "north_star_remote_restore_missing".to_owned(),
        )
    } else if !exact_authority_removed {
        (
            NorthStarProofVerdictV1::Fail,
            "north_star_exact_authority_not_removed".to_owned(),
        )
    } else {
        (
            NorthStarProofVerdictV1::Fail,
            "north_star_seed_threshold_not_met".to_owned(),
        )
    };
    let mut report = NorthStarProofReportV1 {
        schema: NORTH_STAR_PROOF_REPORT_SCHEMA_V1.to_owned(),
        report_root_sha256: String::new(),
        contract_root_sha256: contract.contract_root_sha256.clone(),
        seed_receipts,
        passing_seeds,
        support_future_overlap,
        remote_restore_observed,
        exact_authority_removed,
        verdict,
        blocker,
        authority_ready: false,
        phase_mutation_allowed: false,
    };
    report.report_root_sha256 = report.expected_root()?;
    Ok(report)
}

impl NorthStarProofReportV1 {
    fn expected_root(&self) -> Result<String, NorthStarProofErrorV1> {
        canonical_json_sha256(&(
            NORTH_STAR_PROOF_REPORT_SCHEMA_V1,
            self.contract_root_sha256.as_str(),
            &self.seed_receipts,
            self.passing_seeds,
            self.support_future_overlap,
            self.remote_restore_observed,
            self.exact_authority_removed,
            self.verdict,
            self.blocker.as_str(),
            false,
            false,
        ))
        .map_err(|_| NorthStarProofErrorV1::Serialization)
    }
}

fn validate_seed_receipt(
    contract: &NorthStarProofContractV1,
    receipt: &NorthStarProofSeedReceiptV1,
) -> bool {
    let budget_root = match contract.budget.root_sha256() {
        Ok(root) => root,
        Err(_) => return false,
    };
    let arms = receipt
        .arms
        .iter()
        .map(|metrics| metrics.arm)
        .collect::<Vec<_>>();
    contract.seeds.binary_search(&receipt.seed).is_ok()
        && valid_nonzero_sha256(&receipt.receipt_root_sha256)
        && arms == REQUIRED_ARMS
        && receipt.arms.iter().all(|metrics| {
            metrics.budget_root_sha256 == budget_root
                && valid_nonzero_sha256(&metrics.experiment_report_root_sha256)
                && valid_nonzero_sha256(&metrics.future_rows_root_sha256)
                && valid_nonzero_sha256(&metrics.snapshot_root_sha256)
                && metrics.verifier_coverage_milli <= 1_000
                && metrics.memory_bytes <= contract.budget.total_memory_bytes
                && metrics.exact_checks <= contract.budget.max_exact_checks
                && metrics.cpu_nanos <= contract.budget.max_cpu_nanos
                && metrics.future_rows <= contract.budget.max_future_rows
        })
        && seed_receipt_root(contract.contract_root_sha256.as_str(), receipt)
            .is_ok_and(|root| receipt.receipt_root_sha256 == root)
}

fn seed_receipt_root(
    contract_root: &str,
    receipt: &NorthStarProofSeedReceiptV1,
) -> Result<String, NorthStarProofErrorV1> {
    canonical_json_sha256(&(
        "nando.north-star-proof-seed-receipt.v1",
        contract_root,
        receipt.seed,
        &receipt.arms,
        receipt.delayed_transition_observed,
        receipt.exact_memory_cleanup_observed,
        receipt.key_ablation_drop_milli,
        receipt.non_key_ablation_drop_milli,
        receipt.snapshot_restore_exact,
        receipt.snapshot_cold_start_gain_milli,
    ))
    .map_err(|_| NorthStarProofErrorV1::Serialization)
}

fn seed_is_safe(receipt: &NorthStarProofSeedReceiptV1) -> bool {
    receipt.arms.iter().all(|metrics| {
        metrics.wrong_accepts == 0
            && metrics.runtime_parity_failures == 0
            && metrics.verifier_coverage_milli == 1_000
            && metrics.correct_executions == metrics.future_rows
    })
}

fn seed_passes(contract: &NorthStarProofContractV1, receipt: &NorthStarProofSeedReceiptV1) -> bool {
    let metrics = receipt
        .arms
        .iter()
        .map(|arm| (arm.arm, arm))
        .collect::<BTreeMap<_, _>>();
    let Some(full) = metrics.get(&NorthStarProofArmV1::CellularWaveEnsemble) else {
        return false;
    };
    let Some(monolith) = metrics.get(&NorthStarProofArmV1::EqualBudgetMonolith) else {
        return false;
    };
    let Some(exact) = metrics.get(&NorthStarProofArmV1::ExactStructuralSearch) else {
        return false;
    };
    let controls = [
        NorthStarProofArmV1::NoPhase,
        NorthStarProofArmV1::ShuffledPhase,
        NorthStarProofArmV1::MagnitudeOnly,
        NorthStarProofArmV1::RandomCenter,
    ];
    let control_degraded = controls.into_iter().all(|arm| {
        metrics.get(&arm).is_some_and(|control| {
            !control.circuit_formed
                || full.primary_score_milli - control.primary_score_milli
                    >= contract.thresholds.min_primary_gain_milli
        })
    });
    let key_ratio = if receipt.non_key_ablation_drop_milli <= 0 {
        i64::MAX
    } else {
        receipt.key_ablation_drop_milli.saturating_mul(1_000) / receipt.non_key_ablation_drop_milli
    };
    full.circuit_formed
        && full.primary_score_milli - monolith.primary_score_milli
            >= contract.thresholds.min_primary_gain_milli
        && full.primary_score_milli - exact.primary_score_milli
            >= contract.thresholds.min_primary_gain_milli
        && receipt.key_ablation_drop_milli >= contract.thresholds.min_key_ablation_drop_milli
        && receipt.non_key_ablation_drop_milli >= 0
        && key_ratio >= contract.thresholds.min_key_to_non_key_ratio_milli
        && receipt.delayed_transition_observed
        && receipt.exact_memory_cleanup_observed
        && receipt.snapshot_restore_exact
        && receipt.snapshot_cold_start_gain_milli >= contract.thresholds.min_primary_gain_milli
        && control_degraded
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn metrics(arm: NorthStarProofArmV1, score: i64, formed: bool) -> NorthStarArmMetricsV1 {
        NorthStarArmMetricsV1 {
            arm,
            budget_root_sha256: "a".repeat(64),
            experiment_report_root_sha256: "b".repeat(64),
            future_rows_root_sha256: "c".repeat(64),
            snapshot_root_sha256: "d".repeat(64),
            primary_score_milli: score,
            future_rows: 10,
            correct_executions: 10,
            wrong_accepts: 0,
            runtime_parity_failures: 0,
            verifier_coverage_milli: 1_000,
            exact_checks: 10,
            memory_bytes: 1_024,
            cpu_nanos: 1_000,
            circuit_formed: formed,
        }
    }

    #[test]
    fn required_arms_are_unique_and_stable() {
        assert_eq!(
            REQUIRED_ARMS.into_iter().collect::<BTreeSet<_>>().len(),
            REQUIRED_ARMS.len()
        );
    }

    #[test]
    fn unsafe_seed_never_passes() {
        let receipt = NorthStarProofSeedReceiptV1 {
            seed: 1,
            receipt_root_sha256: "b".repeat(64),
            arms: REQUIRED_ARMS
                .into_iter()
                .map(|arm| metrics(arm, 1_000, true))
                .collect(),
            delayed_transition_observed: true,
            exact_memory_cleanup_observed: true,
            key_ablation_drop_milli: 100,
            non_key_ablation_drop_milli: 10,
            snapshot_restore_exact: true,
            snapshot_cold_start_gain_milli: 100,
        };
        let mut receipt = receipt;
        receipt.arms[0].wrong_accepts = 1;
        assert!(!seed_is_safe(&receipt));
    }
}
