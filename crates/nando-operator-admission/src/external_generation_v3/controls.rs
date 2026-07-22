use std::collections::BTreeSet;

use nando_operator_kernel::{
    RuntimePhaseControlKindV3, RuntimePhaseSelectionV3, canonical_json_bytes,
    canonical_json_sha256, valid_nonzero_sha256,
};
use nando_operator_learning::{GenerationShadowReceiptLedgerV3, GenerationShadowTerminalOutcomeV3};
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

pub fn derive_external_phase_control_receipt_v3(
    generation_id_sha256: &str,
    shadow: &GenerationShadowReceiptLedgerV3,
    support_future_overlap: u32,
) -> Result<ExternalPhaseControlReceiptV3, ExternalGenerationAdmissionErrorV3> {
    if !valid_nonzero_sha256(generation_id_sha256)
        || shadow.generation_id_sha256() != generation_id_sha256
        || shadow.receipts().is_empty()
    {
        return Err(ExternalGenerationAdmissionErrorV3::InvalidPhaseControlReceipt);
    }
    let traffic_roots = shadow
        .receipts()
        .iter()
        .map(|receipt| receipt.traffic_receipt_sha256().to_owned())
        .collect::<Vec<_>>();
    let traffic_receipt_set_sha256 =
        external_phase_control_traffic_set_sha256_v3(generation_id_sha256, &traffic_roots)?;
    let mut observations = ExternalPhaseControlV3::ALL
        .into_iter()
        .map(|control| ExternalPhaseControlObservationV3 {
            control,
            traffic_receipt_set_sha256: traffic_receipt_set_sha256.clone(),
            correct_actions: 0,
            wrong_actions: 0,
            exact_checks: 0,
            selected_actions: 0,
        })
        .collect::<Vec<_>>();
    let mut parity_mismatches = 0_u32;
    let mut false_accepts = 0_u32;
    let mut censored_semantic_updates = 0_u32;
    for receipt in shadow.receipts() {
        parity_mismatches = checked_add(parity_mismatches, u32::from(receipt.parity_mismatch()))?;
        false_accepts = checked_add(false_accepts, u32::from(receipt.local_accepts()))?;
        if receipt.outcome() == GenerationShadowTerminalOutcomeV3::Censored {
            censored_semantic_updates = checked_add(
                censored_semantic_updates,
                u32::from(receipt.semantic_updates()),
            )?;
        }
        let expected = (receipt.outcome() == GenerationShadowTerminalOutcomeV3::VerifiedPass)
            .then(|| receipt.actor_action_sha256())
            .flatten();
        let Some(evidence) = receipt.phase_control_evidence() else {
            continue;
        };
        for observation in evidence.observations() {
            let control = runtime_control(observation.control());
            let aggregate = observations
                .iter_mut()
                .find(|aggregate| aggregate.control == control)
                .ok_or(ExternalGenerationAdmissionErrorV3::InvalidPhaseControlReceipt)?;
            aggregate.exact_checks =
                checked_add(aggregate.exact_checks, observation.exact_action_checks())?;
            if observation.selection() != RuntimePhaseSelectionV3::Selected {
                continue;
            }
            aggregate.selected_actions = checked_add(aggregate.selected_actions, 1)?;
            if observation.selected_physical_action_sha256() == expected {
                aggregate.correct_actions = checked_add(aggregate.correct_actions, 1)?;
            } else {
                aggregate.wrong_actions = checked_add(aggregate.wrong_actions, 1)?;
            }
        }
    }
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
        generation_id_sha256: generation_id_sha256.to_owned(),
        observations,
        full_phase_gain,
        false_accepts,
        parity_mismatches,
        restart_mismatches: 0,
        censored_semantic_updates,
        support_future_overlap,
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
    pub fn safety_zero(&self) -> bool {
        self.false_accepts == 0
            && self.parity_mismatches == 0
            && self.restart_mismatches == 0
            && self.censored_semantic_updates == 0
            && self.support_future_overlap == 0
            && observations_have_no_wrong_actions(&self.observations)
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

fn observations_have_no_wrong_actions(observations: &[ExternalPhaseControlObservationV3]) -> bool {
    observations
        .iter()
        .all(|observation| observation.wrong_actions == 0)
}

const fn runtime_control(control: RuntimePhaseControlKindV3) -> ExternalPhaseControlV3 {
    match control {
        RuntimePhaseControlKindV3::Full => ExternalPhaseControlV3::Full,
        RuntimePhaseControlKindV3::NoPhase => ExternalPhaseControlV3::NoPhase,
        RuntimePhaseControlKindV3::ShuffledPhase => ExternalPhaseControlV3::ShuffledPhase,
        RuntimePhaseControlKindV3::MagnitudeOnly => ExternalPhaseControlV3::MagnitudeOnly,
        RuntimePhaseControlKindV3::MatchedRandomCenter => {
            ExternalPhaseControlV3::MatchedRandomCenter
        }
    }
}

fn checked_add(left: u32, right: u32) -> Result<u32, ExternalGenerationAdmissionErrorV3> {
    left.checked_add(right)
        .ok_or(ExternalGenerationAdmissionErrorV3::Serialization)
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
