use std::collections::BTreeSet;

use nando_core::wave::{
    CandidateCubeField, CircuitSynthesisConfig, CircuitSynthesizer, FrozenSynthesizedCircuitSet,
    OperatorCircuit, OperatorCircuitSynthesisReport, OperatorGrokkingConfig,
};
use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use crate::{BackwardWave, VerifiedDeltaOutcome, VerifiedDeltaReceipt};

use super::{FrozenVersionSpaceEnvelopeV1, Ms3FrozenVersionSpaceStateV1};

pub const NORTH_STAR_CELLULAR_SUPPORT_SCHEMA_V1: &str = "nando.north-star-cellular-support.v1";
const MIN_INDEPENDENT_SUPPORT_RECEIPTS: usize = 3;
const MAX_CELLULAR_SUPPORT_REPORT_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NorthStarCellularSupportReportV1 {
    pub schema: String,
    pub report_root_sha256: String,
    pub frozen_contract_root_sha256: String,
    pub source_generation: u64,
    pub source_operator_fingerprint64: u64,
    pub synthesis_config_root_sha256: String,
    pub grokking_config_root_sha256: String,
    pub support_receipt_roots_sha256: Vec<String>,
    pub support_session_roots_sha256: Vec<String>,
    pub synthesized_circuit_roots_sha256: Vec<String>,
    pub emitted_fragments: usize,
    pub emitted_circuits: usize,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NorthStarCellularSupportV1 {
    pub report: NorthStarCellularSupportReportV1,
    pub synthesis: OperatorCircuitSynthesisReport,
    pub frozen_circuits: FrozenSynthesizedCircuitSet,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NorthStarCellularSupportErrorV1 {
    InvalidFrozenContract,
    InvalidReceipt,
    InsufficientIndependentEvidence,
    InvalidConfig,
    WaveRejected,
    SynthesisFailed,
    CircuitFreezeFailed,
    Serialization,
}

pub fn synthesize_north_star_cellular_support_v1(
    frozen: &FrozenVersionSpaceEnvelopeV1,
    source_generation: u64,
    source_operator_fingerprint64: u64,
    receipts: &[VerifiedDeltaReceipt],
    synthesis_config: CircuitSynthesisConfig,
    grokking_config: OperatorGrokkingConfig,
) -> Result<NorthStarCellularSupportV1, NorthStarCellularSupportErrorV1> {
    frozen
        .validate()
        .map_err(|_| NorthStarCellularSupportErrorV1::InvalidFrozenContract)?;
    if !matches!(
        frozen.contract.state,
        Ms3FrozenVersionSpaceStateV1::UniqueLawFrozen { .. }
    ) || source_generation == 0
        || source_operator_fingerprint64 == 0
    {
        return Err(NorthStarCellularSupportErrorV1::InvalidFrozenContract);
    }
    let receipt_roots = receipts
        .iter()
        .map(|receipt| receipt.receipt_sha256().to_owned())
        .collect::<BTreeSet<_>>();
    let session_roots = receipts
        .iter()
        .map(|receipt| receipt.session_id_sha256().to_owned())
        .collect::<BTreeSet<_>>();
    if receipts.len() < MIN_INDEPENDENT_SUPPORT_RECEIPTS
        || receipt_roots.len() != receipts.len()
        || session_roots.len() != receipts.len()
    {
        return Err(NorthStarCellularSupportErrorV1::InsufficientIndependentEvidence);
    }
    if receipts.iter().any(|receipt| {
        receipt.generation() != source_generation
            || receipt.operator_fingerprint64() != source_operator_fingerprint64
            || receipt.outcome() != VerifiedDeltaOutcome::Positive
            || !valid_nonzero_sha256(receipt.receipt_sha256())
            || !valid_nonzero_sha256(receipt.session_id_sha256())
    }) {
        return Err(NorthStarCellularSupportErrorV1::InvalidReceipt);
    }

    let mut support_field = CandidateCubeField::new(source_generation, grokking_config)
        .map_err(|_| NorthStarCellularSupportErrorV1::InvalidConfig)?;
    for receipt in receipts {
        BackwardWave::apply(&mut support_field, source_operator_fingerprint64, receipt)
            .map_err(|_| NorthStarCellularSupportErrorV1::WaveRejected)?;
    }
    let synthesis = CircuitSynthesizer::synthesize(support_field.waves(), synthesis_config)
        .map_err(|_| NorthStarCellularSupportErrorV1::SynthesisFailed)?;
    let frozen_circuits = FrozenSynthesizedCircuitSet::freeze(source_generation, &synthesis)
        .map_err(|_| NorthStarCellularSupportErrorV1::CircuitFreezeFailed)?;
    let synthesis_config_root_sha256 = synthesis_config_root(synthesis_config)?;
    let grokking_config_root_sha256 = grokking_config_root(grokking_config)?;
    let mut synthesized_circuit_roots_sha256 = synthesis
        .circuits
        .iter()
        .map(circuit_root)
        .collect::<Result<Vec<_>, _>>()?;
    synthesized_circuit_roots_sha256.sort();
    if synthesized_circuit_roots_sha256
        .windows(2)
        .any(|pair| pair[0] == pair[1])
    {
        return Err(NorthStarCellularSupportErrorV1::Serialization);
    }
    let mut report = NorthStarCellularSupportReportV1 {
        schema: NORTH_STAR_CELLULAR_SUPPORT_SCHEMA_V1.to_owned(),
        report_root_sha256: String::new(),
        frozen_contract_root_sha256: frozen.contract.contract_root_sha256.clone(),
        source_generation,
        source_operator_fingerprint64,
        synthesis_config_root_sha256,
        grokking_config_root_sha256,
        support_receipt_roots_sha256: receipt_roots.into_iter().collect(),
        support_session_roots_sha256: session_roots.into_iter().collect(),
        synthesized_circuit_roots_sha256,
        emitted_fragments: synthesis.fragments.emitted_fragments,
        emitted_circuits: synthesis.emitted_circuits,
        authority_ready: false,
        phase_mutation_allowed: false,
    };
    report.report_root_sha256 = report.expected_root()?;
    if !report.validate() {
        return Err(NorthStarCellularSupportErrorV1::Serialization);
    }
    Ok(NorthStarCellularSupportV1 {
        report,
        synthesis,
        frozen_circuits,
    })
}

impl NorthStarCellularSupportReportV1 {
    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == NORTH_STAR_CELLULAR_SUPPORT_SCHEMA_V1
            && valid_nonzero_sha256(&self.report_root_sha256)
            && valid_nonzero_sha256(&self.frozen_contract_root_sha256)
            && self.source_generation > 0
            && self.source_operator_fingerprint64 > 0
            && valid_nonzero_sha256(&self.synthesis_config_root_sha256)
            && valid_nonzero_sha256(&self.grokking_config_root_sha256)
            && sorted_unique_roots(&self.support_receipt_roots_sha256)
            && self.support_receipt_roots_sha256.len() >= MIN_INDEPENDENT_SUPPORT_RECEIPTS
            && sorted_unique_roots(&self.support_session_roots_sha256)
            && self.support_session_roots_sha256.len() == self.support_receipt_roots_sha256.len()
            && sorted_unique_roots(&self.synthesized_circuit_roots_sha256)
            && self.emitted_fragments >= self.support_receipt_roots_sha256.len()
            && self.emitted_circuits == self.synthesized_circuit_roots_sha256.len()
            && !self.authority_ready
            && !self.phase_mutation_allowed
            && self
                .expected_root()
                .is_ok_and(|root| root == self.report_root_sha256)
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, NorthStarCellularSupportErrorV1> {
        if !self.validate() {
            return Err(NorthStarCellularSupportErrorV1::Serialization);
        }
        let bytes =
            serde_cbor::to_vec(self).map_err(|_| NorthStarCellularSupportErrorV1::Serialization)?;
        if bytes.is_empty() || bytes.len() > MAX_CELLULAR_SUPPORT_REPORT_BYTES {
            return Err(NorthStarCellularSupportErrorV1::Serialization);
        }
        Ok(bytes)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, NorthStarCellularSupportErrorV1> {
        if bytes.is_empty() || bytes.len() > MAX_CELLULAR_SUPPORT_REPORT_BYTES {
            return Err(NorthStarCellularSupportErrorV1::Serialization);
        }
        let report: Self = serde_cbor::from_slice(bytes)
            .map_err(|_| NorthStarCellularSupportErrorV1::Serialization)?;
        if !report.validate() || report.canonical_bytes()? != bytes {
            return Err(NorthStarCellularSupportErrorV1::Serialization);
        }
        Ok(report)
    }

    fn expected_root(&self) -> Result<String, NorthStarCellularSupportErrorV1> {
        canonical_json_sha256(&(
            NORTH_STAR_CELLULAR_SUPPORT_SCHEMA_V1,
            self.frozen_contract_root_sha256.as_str(),
            self.source_generation,
            self.source_operator_fingerprint64,
            self.synthesis_config_root_sha256.as_str(),
            self.grokking_config_root_sha256.as_str(),
            &self.support_receipt_roots_sha256,
            &self.support_session_roots_sha256,
            &self.synthesized_circuit_roots_sha256,
            self.emitted_fragments,
            self.emitted_circuits,
            false,
            false,
        ))
        .map_err(|_| NorthStarCellularSupportErrorV1::Serialization)
    }
}

fn synthesis_config_root(
    config: CircuitSynthesisConfig,
) -> Result<String, NorthStarCellularSupportErrorV1> {
    canonical_json_sha256(&(
        "nando.north-star-circuit-synthesis-config.v1",
        config.max_circuits,
        config.max_fragments,
    ))
    .map_err(|_| NorthStarCellularSupportErrorV1::Serialization)
}

fn grokking_config_root(
    config: OperatorGrokkingConfig,
) -> Result<String, NorthStarCellularSupportErrorV1> {
    canonical_json_sha256(&(
        "nando.north-star-grokking-config.v1",
        config.max_circuits,
        config.max_waves,
        config.min_independent_surfaces,
        config.min_independent_sessions,
        config.min_relation_planes,
        config.coherence_floor.to_bits(),
        config.coherence_margin.to_bits(),
    ))
    .map_err(|_| NorthStarCellularSupportErrorV1::Serialization)
}

fn circuit_root(circuit: &OperatorCircuit) -> Result<String, NorthStarCellularSupportErrorV1> {
    let relations = circuit
        .relations()
        .iter()
        .map(|relation| {
            (
                relation.cell.plane,
                relation.cell.source_role,
                relation.cell.target_role,
                relation.state as i8,
                relation.phase_anchor.re.to_bits(),
                relation.phase_anchor.im.to_bits(),
            )
        })
        .collect::<Vec<_>>();
    canonical_json_sha256(&(
        "nando.north-star-synthesized-circuit.v1",
        circuit.role_count(),
        relations,
    ))
    .map_err(|_| NorthStarCellularSupportErrorV1::Serialization)
}

fn sorted_unique_roots(roots: &[String]) -> bool {
    !roots.is_empty()
        && roots.iter().all(|root| valid_nonzero_sha256(root))
        && roots.windows(2).all(|pair| pair[0] < pair[1])
}
