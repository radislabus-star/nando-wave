use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    K2CompositionTreeManifestV1, K2InquiryProbeV1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_CONFIRM_MODELS_V1, K2_UNCERTAINTY_EFFECT_ACCOUNTING_SCHEMA_V1,
    K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1, K2_UNCERTAINTY_FRONTIER_PAGE_SCHEMA_V1,
    K2_UNCERTAINTY_FRONTIER_SCHEMA_V1, K2_UNCERTAINTY_MAX_COST_UNITS_V1,
    K2_UNCERTAINTY_MAX_REPRESENTATIVES_V1, K2_UNCERTAINTY_MAX_RISK_UNITS_V1,
    K2_UNCERTAINTY_MIN_REPRESENTATIVES_V1, K2_UNCERTAINTY_PREDICTION_WITNESS_SCHEMA_V1,
    K2_UNCERTAINTY_PROBE_CLASS_SCHEMA_V1, K2_UNCERTAINTY_RAW_PREDICTIONS_V1,
    K2_UNCERTAINTY_RAW_PROBE_SCHEMA_V1, K2_UNCERTAINTY_RAW_PROBES_V1,
    K2_UNCERTAINTY_RESOURCE_TERMINAL_SCHEMA_V1, K2_UNCERTAINTY_RISK_COST_SCHEMA_V1,
    K2_UNCERTAINTY_ROBUST_ACCOUNTING_SCHEMA_V1, K2_UNCERTAINTY_STATE_COUNT_V1,
    require_denied_authority_v1, require_exact_len_v1, require_sorted_unique_v1,
    uncertainty_root_v1,
};

pub const K2_UNCERTAINTY_STATE_UNIVERSE_SCHEMA_V1: &str = "nando.k2-self-formed-state-universe.v1";
pub const K2_UNCERTAINTY_PROBE_EQUIVALENCE_KEY_SCHEMA_V1: &str =
    "nando.k2-self-formed-probe-equivalence-key.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyStateUniverseV1 {
    pub schema: String,
    pub vocabulary_root_sha256: String,
    pub manifests: Vec<K2CompositionTreeManifestV1>,
    pub universe_root_sha256: String,
}

impl K2UncertaintyStateUniverseV1 {
    pub fn seal(
        vocabulary_root_sha256: String,
        mut manifests: Vec<K2CompositionTreeManifestV1>,
    ) -> K2CompositionResultV1<Self> {
        manifests.sort_by(|left, right| left.tree_root_sha256.cmp(&right.tree_root_sha256));
        let universe_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_STATE_UNIVERSE_SCHEMA_V1,
            &vocabulary_root_sha256,
            &manifests,
        ))?;
        let universe = Self {
            schema: K2_UNCERTAINTY_STATE_UNIVERSE_SCHEMA_V1.to_owned(),
            vocabulary_root_sha256,
            manifests,
            universe_root_sha256,
        };
        universe.validate()?;
        Ok(universe)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.vocabulary_root_sha256)?;
        require_exact_len_v1(
            self.manifests.len(),
            K2_UNCERTAINTY_STATE_COUNT_V1,
            "self_formed_state_denominator_invalid",
        )?;
        for manifest in &self.manifests {
            manifest.validate()?;
        }
        if self
            .manifests
            .windows(2)
            .any(|pair| pair[0].tree_root_sha256 >= pair[1].tree_root_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_state_universe_not_canonical",
            ));
        }
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_STATE_UNIVERSE_SCHEMA_V1,
            &self.vocabulary_root_sha256,
            &self.manifests,
        ))?;
        if self.schema != K2_UNCERTAINTY_STATE_UNIVERSE_SCHEMA_V1
            || self.universe_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_state_universe_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyEligibilityDispositionV1 {
    Eligible,
    NonGeneratedProvenance,
    NonReversible,
    NonExactObservation,
    UnknownAction,
    RiskBudgetExceeded,
    CostBudgetExceeded,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintySafetyDispositionV1 {
    Pass,
    OutOfTree,
    Symlink,
    SpecialFile,
    UnknownOpcode,
    Ambiguous,
    Delayed,
    Malformed,
    OverBudget,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyRiskCostV1 {
    pub schema: String,
    pub read_entries: u64,
    pub written_or_removed_entries: u64,
    pub overwritten_existing_entries: u64,
    pub removed_existing_entries: u64,
    pub overwritten_bytes: u64,
    pub removed_bytes: u64,
    pub touched_bytes: u64,
    pub risk_units: u64,
    pub cost_units: u64,
    pub accounting_root_sha256: String,
}

impl K2UncertaintyRiskCostV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        let byte_risk = ceil_page_units_v1(
            self.overwritten_bytes
                .checked_add(self.removed_bytes)
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_risk_bytes_overflow",
                ))?,
        );
        let risk = self
            .overwritten_existing_entries
            .checked_add(self.removed_existing_entries)
            .and_then(|value| value.checked_add(byte_risk))
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_risk_units_overflow",
            ))?;
        let cost = 1_u64
            .checked_add(self.read_entries)
            .and_then(|value| value.checked_add(self.written_or_removed_entries))
            .and_then(|value| value.checked_add(ceil_page_units_v1(self.touched_bytes)))
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_cost_units_overflow",
            ))?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_RISK_COST_SCHEMA_V1,
            self.read_entries,
            self.written_or_removed_entries,
            self.overwritten_existing_entries,
            self.removed_existing_entries,
            self.overwritten_bytes,
            self.removed_bytes,
            self.touched_bytes,
            risk,
            cost,
        ))?;
        if self.schema != K2_UNCERTAINTY_RISK_COST_SCHEMA_V1
            || self.risk_units != risk
            || self.cost_units != cost
            || self.risk_units > K2_UNCERTAINTY_MAX_RISK_UNITS_V1
            || self.cost_units > K2_UNCERTAINTY_MAX_COST_UNITS_V1
            || self.accounting_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_risk_cost_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.risk_units = self
            .overwritten_existing_entries
            .checked_add(self.removed_existing_entries)
            .and_then(|value| {
                self.overwritten_bytes
                    .checked_add(self.removed_bytes)
                    .and_then(|bytes| value.checked_add(ceil_page_units_v1(bytes)))
            })
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_risk_units_overflow",
            ))?;
        self.cost_units = 1_u64
            .checked_add(self.read_entries)
            .and_then(|value| value.checked_add(self.written_or_removed_entries))
            .and_then(|value| value.checked_add(ceil_page_units_v1(self.touched_bytes)))
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_cost_units_overflow",
            ))?;
        self.accounting_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_RISK_COST_SCHEMA_V1,
            self.read_entries,
            self.written_or_removed_entries,
            self.overwritten_existing_entries,
            self.removed_existing_entries,
            self.overwritten_bytes,
            self.removed_bytes,
            self.touched_bytes,
            self.risk_units,
            self.cost_units,
        ))?;
        self.validate()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyEffectAccountingV1 {
    pub schema: String,
    pub effect_root_sha256: String,
    pub accounting: K2UncertaintyRiskCostV1,
    pub effect_accounting_root_sha256: String,
}

impl K2UncertaintyEffectAccountingV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.effect_root_sha256)?;
        self.accounting.validate()?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_EFFECT_ACCOUNTING_SCHEMA_V1,
            &self.effect_root_sha256,
            &self.accounting,
        ))?;
        if self.schema != K2_UNCERTAINTY_EFFECT_ACCOUNTING_SCHEMA_V1
            || self.effect_accounting_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_effect_accounting_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.effect_accounting_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_EFFECT_ACCOUNTING_SCHEMA_V1,
            &self.effect_root_sha256,
            &self.accounting,
        ))?;
        self.validate()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyRobustAccountingV1 {
    pub schema: String,
    pub effects: Vec<K2UncertaintyEffectAccountingV1>,
    pub maximum_risk_units: u64,
    pub maximum_cost_units: u64,
    pub robust_accounting_root_sha256: String,
}

impl K2UncertaintyRobustAccountingV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_exact_len_v1(
            self.effects.len(),
            super::K2_UNCERTAINTY_EFFECTS_PER_ACTION_V1,
            "self_formed_robust_accounting_effect_count_invalid",
        )?;
        for effect in &self.effects {
            effect.validate()?;
        }
        if self
            .effects
            .windows(2)
            .any(|pair| pair[0].effect_root_sha256 >= pair[1].effect_root_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_robust_accounting_not_canonical",
            ));
        }
        let risk = self
            .effects
            .iter()
            .map(|effect| effect.accounting.risk_units)
            .max()
            .unwrap_or(0);
        let cost = self
            .effects
            .iter()
            .map(|effect| effect.accounting.cost_units)
            .max()
            .unwrap_or(0);
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_ROBUST_ACCOUNTING_SCHEMA_V1,
            &self.effects,
            risk,
            cost,
        ))?;
        if self.schema != K2_UNCERTAINTY_ROBUST_ACCOUNTING_SCHEMA_V1
            || self.maximum_risk_units != risk
            || self.maximum_cost_units != cost
            || self.maximum_risk_units > K2_UNCERTAINTY_MAX_RISK_UNITS_V1
            || self.maximum_cost_units > K2_UNCERTAINTY_MAX_COST_UNITS_V1
            || self.robust_accounting_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_robust_accounting_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.effects.sort();
        self.maximum_risk_units = self
            .effects
            .iter()
            .map(|effect| effect.accounting.risk_units)
            .max()
            .unwrap_or(0);
        self.maximum_cost_units = self
            .effects
            .iter()
            .map(|effect| effect.accounting.cost_units)
            .max()
            .unwrap_or(0);
        self.robust_accounting_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_ROBUST_ACCOUNTING_SCHEMA_V1,
            &self.effects,
            self.maximum_risk_units,
            self.maximum_cost_units,
        ))?;
        self.validate()
    }
}

#[must_use]
pub const fn ceil_page_units_v1(bytes: u64) -> u64 {
    if bytes == 0 {
        0
    } else {
        1 + (bytes - 1) / 4096
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyPredictionWitnessV1 {
    pub schema: String,
    pub model_root_sha256: String,
    pub probe_root_sha256: String,
    pub transition_applied: bool,
    pub transition_reason: String,
    pub predicted_post_manifest: K2CompositionTreeManifestV1,
    pub observable_outcome_root_sha256: String,
    pub prediction_root_sha256: String,
}

impl K2UncertaintyPredictionWitnessV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.model_root_sha256)?;
        require_composition_root_v1(&self.probe_root_sha256)?;
        require_composition_root_v1(&self.observable_outcome_root_sha256)?;
        self.predicted_post_manifest.validate()?;
        let observable = uncertainty_root_v1(&(
            "nando.k2-inquiry-observable-exact-manifest.v1",
            &self.predicted_post_manifest,
        ))?;
        let reason_valid = if self.transition_applied {
            self.transition_reason == "applied"
        } else {
            matches!(
                self.transition_reason.as_str(),
                "copy_source_missing" | "remove_path_missing"
            )
        };
        if !reason_valid || self.observable_outcome_root_sha256 != observable {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_prediction_reason_invalid",
            ));
        }
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_PREDICTION_WITNESS_SCHEMA_V1,
            &self.model_root_sha256,
            &self.probe_root_sha256,
            self.transition_applied,
            &self.transition_reason,
            &self.predicted_post_manifest,
            &self.observable_outcome_root_sha256,
        ))?;
        if self.schema != K2_UNCERTAINTY_PREDICTION_WITNESS_SCHEMA_V1
            || self.prediction_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_prediction_witness_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.prediction_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_PREDICTION_WITNESS_SCHEMA_V1,
            &self.model_root_sha256,
            &self.probe_root_sha256,
            self.transition_applied,
            &self.transition_reason,
            &self.predicted_post_manifest,
            &self.observable_outcome_root_sha256,
        ))?;
        self.validate()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyProbeEquivalenceKeyV1 {
    pub schema: String,
    pub pairwise_outcome_equal: [bool; 6],
    pub eligibility: K2UncertaintyEligibilityDispositionV1,
    pub safety: K2UncertaintySafetyDispositionV1,
    pub risk_units: u64,
    pub cost_units: u64,
    pub applicability_hint: bool,
    pub dependency_hint: bool,
    pub cleanup_hint: bool,
    pub key_root_sha256: String,
}

impl K2UncertaintyProbeEquivalenceKeyV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_PROBE_EQUIVALENCE_KEY_SCHEMA_V1,
            self.pairwise_outcome_equal,
            self.eligibility,
            self.safety,
            self.risk_units,
            self.cost_units,
            self.applicability_hint,
            self.dependency_hint,
            self.cleanup_hint,
        ))?;
        if self.schema != K2_UNCERTAINTY_PROBE_EQUIVALENCE_KEY_SCHEMA_V1
            || self.risk_units > K2_UNCERTAINTY_MAX_RISK_UNITS_V1
            || self.cost_units > K2_UNCERTAINTY_MAX_COST_UNITS_V1
            || self.key_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_probe_equivalence_key_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.key_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_PROBE_EQUIVALENCE_KEY_SCHEMA_V1,
            self.pairwise_outcome_equal,
            self.eligibility,
            self.safety,
            self.risk_units,
            self.cost_units,
            self.applicability_hint,
            self.dependency_hint,
            self.cleanup_hint,
        ))?;
        self.validate()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyRawProbeDispositionV1 {
    pub schema: String,
    pub raw_sequence: u64,
    pub probe: K2InquiryProbeV1,
    pub predictions: Vec<K2UncertaintyPredictionWitnessV1>,
    pub robust_accounting: K2UncertaintyRobustAccountingV1,
    pub eligibility: K2UncertaintyEligibilityDispositionV1,
    pub safety: K2UncertaintySafetyDispositionV1,
    pub equivalence_key: K2UncertaintyProbeEquivalenceKeyV1,
    pub raw_probe_root_sha256: String,
}

impl K2UncertaintyRawProbeDispositionV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.probe.validate()?;
        self.robust_accounting.validate()?;
        self.equivalence_key.validate()?;
        require_exact_len_v1(
            self.predictions.len(),
            K2_UNCERTAINTY_CONFIRM_MODELS_V1,
            "self_formed_probe_prediction_count_invalid",
        )?;
        for prediction in &self.predictions {
            prediction.validate()?;
            if prediction.probe_root_sha256 != self.probe.probe_root_sha256 {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_probe_prediction_binding_invalid",
                ));
            }
        }
        if self
            .predictions
            .windows(2)
            .any(|pair| pair[0].model_root_sha256 >= pair[1].model_root_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_probe_prediction_order_invalid",
            ));
        }
        let outcomes = self
            .predictions
            .iter()
            .map(|prediction| prediction.observable_outcome_root_sha256.as_str())
            .collect::<Vec<_>>();
        let pairwise = [
            outcomes[0] == outcomes[1],
            outcomes[0] == outcomes[2],
            outcomes[0] == outcomes[3],
            outcomes[1] == outcomes[2],
            outcomes[1] == outcomes[3],
            outcomes[2] == outcomes[3],
        ];
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_RAW_PROBE_SCHEMA_V1,
            self.raw_sequence,
            &self.probe,
            &self.predictions,
            &self.robust_accounting,
            self.eligibility,
            self.safety,
            &self.equivalence_key,
        ))?;
        if self.schema != K2_UNCERTAINTY_RAW_PROBE_SCHEMA_V1
            || self.raw_sequence >= K2_UNCERTAINTY_RAW_PROBES_V1 as u64
            || self.equivalence_key.pairwise_outcome_equal != pairwise
            || self.equivalence_key.eligibility != self.eligibility
            || self.equivalence_key.safety != self.safety
            || self.equivalence_key.risk_units != self.probe.risk_units
            || self.equivalence_key.cost_units != self.probe.cost_units
            || self.equivalence_key.applicability_hint != self.probe.applicability_hint
            || self.equivalence_key.dependency_hint != self.probe.dependency_hint
            || self.equivalence_key.cleanup_hint != self.probe.cleanup_hint
            || self.robust_accounting.maximum_risk_units != self.probe.risk_units
            || self.robust_accounting.maximum_cost_units != self.probe.cost_units
            || self.raw_probe_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_raw_probe_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.raw_probe_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_RAW_PROBE_SCHEMA_V1,
            self.raw_sequence,
            &self.probe,
            &self.predictions,
            &self.robust_accounting,
            self.eligibility,
            self.safety,
            &self.equivalence_key,
        ))?;
        self.validate()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyFrontierPageV1 {
    pub schema: String,
    pub case_id_sha256: String,
    pub page_sequence: u64,
    pub dispositions: Vec<K2UncertaintyRawProbeDispositionV1>,
    pub page_root_sha256: String,
}

impl K2UncertaintyFrontierPageV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.case_id_sha256)?;
        if self.dispositions.is_empty()
            || self.dispositions.len() > K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_frontier_page_size_invalid",
            ));
        }
        let first = self.page_sequence * K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1 as u64;
        for (offset, disposition) in self.dispositions.iter().enumerate() {
            disposition.validate()?;
            if disposition.raw_sequence != first + offset as u64 {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_frontier_page_sequence_invalid",
                ));
            }
        }
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_FRONTIER_PAGE_SCHEMA_V1,
            &self.case_id_sha256,
            self.page_sequence,
            &self.dispositions,
        ))?;
        if self.schema != K2_UNCERTAINTY_FRONTIER_PAGE_SCHEMA_V1
            || self.page_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_frontier_page_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.page_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_FRONTIER_PAGE_SCHEMA_V1,
            &self.case_id_sha256,
            self.page_sequence,
            &self.dispositions,
        ))?;
        self.validate()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyProbeClassV1 {
    pub schema: String,
    pub equivalence_key: K2UncertaintyProbeEquivalenceKeyV1,
    pub member_probe_roots_sha256: Vec<String>,
    pub representative_probe_root_sha256: String,
    pub class_root_sha256: String,
}

impl K2UncertaintyProbeClassV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.equivalence_key.validate()?;
        require_sorted_unique_v1(
            &self.member_probe_roots_sha256,
            "self_formed_probe_class_members_invalid",
        )?;
        for root in &self.member_probe_roots_sha256 {
            require_composition_root_v1(root)?;
        }
        let representative =
            self.member_probe_roots_sha256
                .first()
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_probe_class_empty",
                ))?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_PROBE_CLASS_SCHEMA_V1,
            &self.equivalence_key,
            &self.member_probe_roots_sha256,
            &self.representative_probe_root_sha256,
        ))?;
        if self.schema != K2_UNCERTAINTY_PROBE_CLASS_SCHEMA_V1
            || &self.representative_probe_root_sha256 != representative
            || self.class_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_probe_class_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.class_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_PROBE_CLASS_SCHEMA_V1,
            &self.equivalence_key,
            &self.member_probe_roots_sha256,
            &self.representative_probe_root_sha256,
        ))?;
        self.validate()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyFrontierV1 {
    pub schema: String,
    pub case_id_sha256: String,
    pub model_set_root_sha256: String,
    pub state_universe_root_sha256: String,
    pub raw_probe_count: u64,
    pub raw_prediction_count: u64,
    pub page_roots_sha256: Vec<String>,
    pub raw_probe_denominator_root_sha256: String,
    pub classes: Vec<K2UncertaintyProbeClassV1>,
    pub representative_probe_roots_sha256: Vec<String>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub frontier_root_sha256: String,
}

impl K2UncertaintyFrontierV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.case_id_sha256,
            &self.model_set_root_sha256,
            &self.state_universe_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        let page_count =
            K2_UNCERTAINTY_RAW_PROBES_V1.div_ceil(K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1);
        require_exact_len_v1(
            self.page_roots_sha256.len(),
            page_count,
            "self_formed_frontier_page_denominator_invalid",
        )?;
        require_sorted_unique_v1(
            &self.page_roots_sha256,
            "self_formed_frontier_page_roots_invalid",
        )?;
        for root in &self.page_roots_sha256 {
            require_composition_root_v1(root)?;
        }
        if self.classes.len() < K2_UNCERTAINTY_MIN_REPRESENTATIVES_V1
            || self.classes.len() > K2_UNCERTAINTY_MAX_REPRESENTATIVES_V1
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_frontier_class_count_invalid",
            ));
        }
        let mut all_members = BTreeSet::new();
        let mut representatives = Vec::with_capacity(self.classes.len());
        let mut class_keys = BTreeSet::new();
        for class in &self.classes {
            class.validate()?;
            if !class_keys.insert(&class.equivalence_key) {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_frontier_duplicate_class",
                ));
            }
            representatives.push(class.representative_probe_root_sha256.clone());
            for member in &class.member_probe_roots_sha256 {
                if !all_members.insert(member.clone()) {
                    return Err(K2CompositionErrorV1::Invalid(
                        "self_formed_frontier_duplicate_member",
                    ));
                }
            }
        }
        representatives.sort();
        let all_member_roots = all_members.into_iter().collect::<Vec<_>>();
        let denominator_root = uncertainty_root_v1(&(
            "nando.k2-self-formed-raw-probe-denominator.v1",
            &all_member_roots,
        ))?;
        require_denied_authority_v1(&self.authority)?;
        let expected = self.expected_root()?;
        if self.schema != K2_UNCERTAINTY_FRONTIER_SCHEMA_V1
            || self.raw_probe_count != K2_UNCERTAINTY_RAW_PROBES_V1 as u64
            || self.raw_prediction_count != K2_UNCERTAINTY_RAW_PREDICTIONS_V1 as u64
            || all_member_roots.len() != K2_UNCERTAINTY_RAW_PROBES_V1
            || self.raw_probe_denominator_root_sha256 != denominator_root
            || self.representative_probe_roots_sha256 != representatives
            || self.frontier_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_frontier_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.frontier_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_FRONTIER_SCHEMA_V1,
            &self.case_id_sha256,
            &self.model_set_root_sha256,
            &self.state_universe_root_sha256,
            self.raw_probe_count,
            self.raw_prediction_count,
            &self.page_roots_sha256,
            &self.raw_probe_denominator_root_sha256,
            &self.classes,
            &self.representative_probe_roots_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyResourceTerminalKindV1 {
    CountOverflow,
    ProtocolBytesExhausted,
    ResidentMemoryExhausted,
    CaseDeadlineExceeded,
    BatchDeadlineExceeded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyResourceTerminalV1 {
    pub schema: String,
    pub case_id_sha256: Option<String>,
    pub kind: K2UncertaintyResourceTerminalKindV1,
    pub measured: u64,
    pub limit: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub terminal_root_sha256: String,
}

impl K2UncertaintyResourceTerminalV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        if let Some(case) = &self.case_id_sha256 {
            require_composition_root_v1(case)?;
        }
        require_denied_authority_v1(&self.authority)?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_RESOURCE_TERMINAL_SCHEMA_V1,
            &self.case_id_sha256,
            self.kind,
            self.measured,
            self.limit,
            &self.authority,
        ))?;
        if self.schema != K2_UNCERTAINTY_RESOURCE_TERMINAL_SCHEMA_V1
            || self.measured <= self.limit
            || self.terminal_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_resource_terminal_invalid",
            ));
        }
        Ok(())
    }
}
