use std::collections::BTreeSet;

use nando_operator_kernel::{canonical_json_bytes, canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::ExternalGenerationAdmissionErrorV3;

pub const EXTERNAL_PHASE_CONTROL_RECEIPT_SCHEMA_V3: &str =
    "nando.external-phase-control-receipt.v3.f8d";
pub const EXTERNAL_PHASE_CONTROL_TRAFFIC_SET_SCHEMA_V3: &str =
    "nando.external-phase-control-traffic-set.v3.f8d";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPhaseControlV3 {
    Full,
    NoPhase,
    ShuffledPhase,
    MagnitudeOnly,
    MatchedRandomCenter,
    NoWave,
}

impl ExternalPhaseControlV3 {
    pub const ALL: [Self; 6] = [
        Self::Full,
        Self::NoPhase,
        Self::ShuffledPhase,
        Self::MagnitudeOnly,
        Self::MatchedRandomCenter,
        Self::NoWave,
    ];
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalPhaseControlObservationV3 {
    control: ExternalPhaseControlV3,
    traffic_receipt_set_sha256: String,
    correct_actions: u32,
    wrong_actions: u32,
    exact_checks: u32,
    selected_actions: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalPhaseControlReceiptV3 {
    schema: String,
    generation_id_sha256: String,
    observations: Vec<ExternalPhaseControlObservationV3>,
    full_phase_gain: u32,
    false_accepts: u32,
    parity_mismatches: u32,
    restart_mismatches: u32,
    censored_semantic_updates: u32,
    support_future_overlap: u32,
    receipt_sha256: String,
    execution_authority: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalPhaseControlObservationInputV3 {
    pub control: ExternalPhaseControlV3,
    pub traffic_receipt_set_sha256: String,
    pub correct_actions: u32,
    pub wrong_actions: u32,
    pub exact_checks: u32,
    pub selected_actions: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalPhaseControlReceiptInputV3 {
    pub generation_id_sha256: String,
    pub observations: Vec<ExternalPhaseControlObservationInputV3>,
    pub false_accepts: u32,
    pub parity_mismatches: u32,
    pub restart_mismatches: u32,
    pub censored_semantic_updates: u32,
    pub support_future_overlap: u32,
}

pub fn seal_external_phase_control_receipt_v3(
    input: ExternalPhaseControlReceiptInputV3,
) -> Result<ExternalPhaseControlReceiptV3, ExternalGenerationAdmissionErrorV3> {
    if !valid_nonzero_sha256(&input.generation_id_sha256)
        || input.observations.len() != ExternalPhaseControlV3::ALL.len()
    {
        return Err(ExternalGenerationAdmissionErrorV3::InvalidPhaseControlReceipt);
    }
    let observations = input
        .observations
        .into_iter()
        .map(|observation| ExternalPhaseControlObservationV3 {
            control: observation.control,
            traffic_receipt_set_sha256: observation.traffic_receipt_set_sha256,
            correct_actions: observation.correct_actions,
            wrong_actions: observation.wrong_actions,
            exact_checks: observation.exact_checks,
            selected_actions: observation.selected_actions,
        })
        .collect::<Vec<_>>();
    validate_observations(&observations)?;
    let full = observations
        .iter()
        .find(|observation| observation.control == ExternalPhaseControlV3::Full)
        .ok_or(ExternalGenerationAdmissionErrorV3::InvalidPhaseControlReceipt)?;
    let best_control = observations
        .iter()
        .filter(|observation| observation.control != ExternalPhaseControlV3::Full)
        .map(|observation| observation.correct_actions)
        .max()
        .unwrap_or_default();
    let full_phase_gain = full.correct_actions.saturating_sub(best_control);
    let mut receipt = ExternalPhaseControlReceiptV3 {
        schema: EXTERNAL_PHASE_CONTROL_RECEIPT_SCHEMA_V3.to_owned(),
        generation_id_sha256: input.generation_id_sha256,
        observations,
        full_phase_gain,
        false_accepts: input.false_accepts,
        parity_mismatches: input.parity_mismatches,
        restart_mismatches: input.restart_mismatches,
        censored_semantic_updates: input.censored_semantic_updates,
        support_future_overlap: input.support_future_overlap,
        receipt_sha256: String::new(),
        execution_authority: false,
    };
    receipt.receipt_sha256 = control_digest(&receipt)?;
    validate_control_receipt(&receipt)?;
    Ok(receipt)
}

impl ExternalPhaseControlReceiptV3 {
    pub fn canonical_bytes(&self) -> Result<Box<[u8]>, ExternalGenerationAdmissionErrorV3> {
        validate_control_receipt(self)?;
        canonical_json_bytes(self)
            .map(Vec::into_boxed_slice)
            .map_err(|_| ExternalGenerationAdmissionErrorV3::Serialization)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, ExternalGenerationAdmissionErrorV3> {
        let receipt: Self = serde_json::from_slice(bytes)
            .map_err(|_| ExternalGenerationAdmissionErrorV3::InvalidPhaseControlReceipt)?;
        validate_control_receipt(&receipt)?;
        if receipt.canonical_bytes()?.as_ref() != bytes {
            return Err(ExternalGenerationAdmissionErrorV3::InvalidPhaseControlReceipt);
        }
        Ok(receipt)
    }

    #[must_use]
    pub fn generation_id_sha256(&self) -> &str {
        &self.generation_id_sha256
    }

    #[must_use]
    pub fn receipt_sha256(&self) -> &str {
        &self.receipt_sha256
    }

    #[must_use]
    pub const fn full_phase_gain(&self) -> u32 {
        self.full_phase_gain
    }

    #[must_use]
    pub fn traffic_receipt_set_sha256(&self) -> &str {
        &self.observations[0].traffic_receipt_set_sha256
    }

    #[must_use]
    pub const fn safety_zero(&self) -> bool {
        self.false_accepts == 0
            && self.parity_mismatches == 0
            && self.restart_mismatches == 0
            && self.censored_semantic_updates == 0
            && self.support_future_overlap == 0
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

fn validate_observations(
    observations: &[ExternalPhaseControlObservationV3],
) -> Result<(), ExternalGenerationAdmissionErrorV3> {
    let controls = observations
        .iter()
        .map(|observation| observation.control)
        .collect::<BTreeSet<_>>();
    let traffic_receipt_set_sha256 = observations
        .first()
        .map(|observation| observation.traffic_receipt_set_sha256.as_str());
    if controls != ExternalPhaseControlV3::ALL.into_iter().collect()
        || traffic_receipt_set_sha256.is_none()
        || observations.iter().any(|observation| {
            !valid_nonzero_sha256(&observation.traffic_receipt_set_sha256)
                || Some(observation.traffic_receipt_set_sha256.as_str())
                    != traffic_receipt_set_sha256
                || observation
                    .correct_actions
                    .saturating_add(observation.wrong_actions)
                    != observation.selected_actions
                || observation.selected_actions > observation.exact_checks
        })
    {
        return Err(ExternalGenerationAdmissionErrorV3::InvalidPhaseControlReceipt);
    }
    Ok(())
}

fn validate_control_receipt(
    receipt: &ExternalPhaseControlReceiptV3,
) -> Result<(), ExternalGenerationAdmissionErrorV3> {
    if receipt.schema != EXTERNAL_PHASE_CONTROL_RECEIPT_SCHEMA_V3
        || !valid_nonzero_sha256(&receipt.generation_id_sha256)
        || !valid_nonzero_sha256(&receipt.receipt_sha256)
        || receipt.execution_authority
        || !receipt.safety_zero()
    {
        return Err(ExternalGenerationAdmissionErrorV3::InvalidPhaseControlReceipt);
    }
    validate_observations(&receipt.observations)?;
    let full = receipt
        .observations
        .iter()
        .find(|observation| observation.control == ExternalPhaseControlV3::Full)
        .ok_or(ExternalGenerationAdmissionErrorV3::InvalidPhaseControlReceipt)?;
    let best_control = receipt
        .observations
        .iter()
        .filter(|observation| observation.control != ExternalPhaseControlV3::Full)
        .map(|observation| observation.correct_actions)
        .max()
        .unwrap_or_default();
    if receipt.full_phase_gain != full.correct_actions.saturating_sub(best_control)
        || control_digest(receipt)? != receipt.receipt_sha256
    {
        return Err(ExternalGenerationAdmissionErrorV3::InvalidPhaseControlReceipt);
    }
    Ok(())
}

pub fn external_phase_control_traffic_set_sha256_v3(
    generation_id_sha256: &str,
    traffic_receipt_roots: &[String],
) -> Result<String, ExternalGenerationAdmissionErrorV3> {
    if !valid_nonzero_sha256(generation_id_sha256) || traffic_receipt_roots.is_empty() {
        return Err(ExternalGenerationAdmissionErrorV3::InvalidPhaseControlReceipt);
    }
    let mut roots = traffic_receipt_roots.to_vec();
    roots.sort_unstable();
    if roots.iter().any(|root| !valid_nonzero_sha256(root))
        || roots.windows(2).any(|pair| pair[0] == pair[1])
    {
        return Err(ExternalGenerationAdmissionErrorV3::InvalidPhaseControlReceipt);
    }
    canonical_json_sha256(&(
        EXTERNAL_PHASE_CONTROL_TRAFFIC_SET_SCHEMA_V3,
        generation_id_sha256,
        roots,
    ))
    .map_err(|_| ExternalGenerationAdmissionErrorV3::Serialization)
}

fn control_digest(
    receipt: &ExternalPhaseControlReceiptV3,
) -> Result<String, ExternalGenerationAdmissionErrorV3> {
    canonical_json_sha256(&(
        EXTERNAL_PHASE_CONTROL_RECEIPT_SCHEMA_V3,
        receipt.generation_id_sha256.as_str(),
        &receipt.observations,
        receipt.full_phase_gain,
        receipt.false_accepts,
        receipt.parity_mismatches,
        receipt.restart_mismatches,
        receipt.censored_semantic_updates,
        receipt.support_future_overlap,
        false,
    ))
    .map_err(|_| ExternalGenerationAdmissionErrorV3::Serialization)
}
