use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    ProgramSemanticClassIdV1, canonical_json_sha256, valid_nonzero_sha256,
};
use serde::{Deserialize, Serialize};

pub const DISTINGUISHING_PROBE_MAX_CLASSES_V1: usize = 4_096;
pub const DISTINGUISHING_PROBE_MAX_CANDIDATES_V1: usize = 4_096;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSourceContractV1 {
    PassiveLiveTraffic,
    SealedDiscoveryCorpus,
    DevelopmentHarness,
    ExternalCausalInquiry,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProbeClassPredictionV1 {
    pub class_id: ProgramSemanticClassIdV1,
    pub outcome_partition_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DistinguishingProbeCandidateV1 {
    pub probe_root_sha256: String,
    pub observable_difference_root_sha256: String,
    pub source: EvidenceSourceContractV1,
    pub estimated_cost_units: u64,
    pub predictions: Vec<ProbeClassPredictionV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MissingEvidenceContractV1 {
    competing_class_roots_sha256: Vec<String>,
    probe_root_sha256: String,
    observable_difference_root_sha256: String,
    source: EvidenceSourceContractV1,
    expected_partition_gain: usize,
    estimated_cost_units: u64,
    stable_tie_break_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InquiryErrorV1 {
    InsufficientClasses,
    InvalidProbe,
    IncompletePredictions,
    NoDistinguishingProbe,
    BudgetExhausted,
    Serialization,
}

pub fn select_distinguishing_probe_v1(
    class_ids: &[ProgramSemanticClassIdV1],
    probes: &[DistinguishingProbeCandidateV1],
) -> Result<MissingEvidenceContractV1, InquiryErrorV1> {
    let class_set = class_ids.iter().cloned().collect::<BTreeSet<_>>();
    if class_set.len() < 2 {
        return Err(InquiryErrorV1::InsufficientClasses);
    }
    if class_set.len() > DISTINGUISHING_PROBE_MAX_CLASSES_V1
        || probes.len() > DISTINGUISHING_PROBE_MAX_CANDIDATES_V1
    {
        return Err(InquiryErrorV1::BudgetExhausted);
    }

    let mut best: Option<(usize, u64, String, &DistinguishingProbeCandidateV1)> = None;
    for probe in probes {
        validate_probe(probe, &class_set)?;
        let mut partitions = BTreeMap::<&str, usize>::new();
        for prediction in &probe.predictions {
            *partitions
                .entry(prediction.outcome_partition_root_sha256.as_str())
                .or_default() += 1;
        }
        let largest_partition = partitions
            .values()
            .copied()
            .max()
            .unwrap_or(class_set.len());
        let gain = class_set.len().saturating_sub(largest_partition);
        if gain == 0 {
            continue;
        }
        let tie_break = canonical_json_sha256(&(
            "nando.distinguishing-probe-tie-break.v1",
            probe.probe_root_sha256.as_str(),
            probe.observable_difference_root_sha256.as_str(),
            probe.source,
            probe.estimated_cost_units,
            &probe.predictions,
        ))
        .map_err(|_| InquiryErrorV1::Serialization)?;
        let replace = best
            .as_ref()
            .is_none_or(|(best_gain, best_cost, best_tie, _)| {
                let left = u128::from(gain as u64) * u128::from(*best_cost);
                let right = u128::from(*best_gain as u64) * u128::from(probe.estimated_cost_units);
                left > right || (left == right && tie_break < *best_tie)
            });
        if replace {
            best = Some((gain, probe.estimated_cost_units, tie_break, probe));
        }
    }

    let Some((gain, cost, tie_break, probe)) = best else {
        return Err(InquiryErrorV1::NoDistinguishingProbe);
    };
    Ok(MissingEvidenceContractV1 {
        competing_class_roots_sha256: class_set
            .iter()
            .map(|class_id| class_id.as_str().to_owned())
            .collect(),
        probe_root_sha256: probe.probe_root_sha256.clone(),
        observable_difference_root_sha256: probe.observable_difference_root_sha256.clone(),
        source: probe.source,
        expected_partition_gain: gain,
        estimated_cost_units: cost,
        stable_tie_break_sha256: tie_break,
    })
}

impl MissingEvidenceContractV1 {
    #[must_use]
    pub fn competing_class_roots_sha256(&self) -> &[String] {
        &self.competing_class_roots_sha256
    }

    #[must_use]
    pub fn probe_root_sha256(&self) -> &str {
        &self.probe_root_sha256
    }

    #[must_use]
    pub fn observable_difference_root_sha256(&self) -> &str {
        &self.observable_difference_root_sha256
    }

    #[must_use]
    pub const fn source(&self) -> EvidenceSourceContractV1 {
        self.source
    }

    #[must_use]
    pub const fn expected_partition_gain(&self) -> usize {
        self.expected_partition_gain
    }

    #[must_use]
    pub const fn estimated_cost_units(&self) -> u64 {
        self.estimated_cost_units
    }

    #[must_use]
    pub fn stable_tie_break_sha256(&self) -> &str {
        &self.stable_tie_break_sha256
    }
}

fn validate_probe(
    probe: &DistinguishingProbeCandidateV1,
    class_set: &BTreeSet<ProgramSemanticClassIdV1>,
) -> Result<(), InquiryErrorV1> {
    if probe.estimated_cost_units == 0
        || !valid_nonzero_sha256(&probe.probe_root_sha256)
        || !valid_nonzero_sha256(&probe.observable_difference_root_sha256)
        || probe
            .predictions
            .iter()
            .any(|prediction| !valid_nonzero_sha256(&prediction.outcome_partition_root_sha256))
    {
        return Err(InquiryErrorV1::InvalidProbe);
    }
    let predicted = probe
        .predictions
        .iter()
        .map(|prediction| prediction.class_id.clone())
        .collect::<BTreeSet<_>>();
    if predicted != *class_set || predicted.len() != probe.predictions.len() {
        return Err(InquiryErrorV1::IncompletePredictions);
    }
    Ok(())
}
