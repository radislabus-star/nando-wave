use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionLearnedEffectV1,
    K2CompositionResultV1, K2InquiryModelActionV1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_ACTION_SURVIVORS_SCHEMA_V1, K2_UNCERTAINTY_ACTIONS_V1,
    K2_UNCERTAINTY_CONFIRM_MODELS_V1, K2_UNCERTAINTY_CONSISTENCY_DISPOSITIONS_V1,
    K2_UNCERTAINTY_CONSISTENCY_SCHEMA_V1, K2_UNCERTAINTY_EFFECTS_PER_ACTION_V1,
    K2_UNCERTAINTY_MODEL_SET_SCHEMA_V1, K2_UNCERTAINTY_RAW_MODEL_COUNT_V1,
    K2_UNCERTAINTY_RAW_PROBES_V1, K2_UNCERTAINTY_SEMANTIC_CLASS_SCHEMA_V1,
    K2_UNCERTAINTY_SEMANTIC_SIGNATURE_SCHEMA_V1, K2UncertaintySupportSetV1,
    require_denied_authority_v1, require_exact_len_v1, require_sorted_unique_v1,
    uncertainty_root_v1,
};

pub const K2_UNCERTAINTY_CONSISTENCY_SET_SCHEMA_V1: &str =
    "nando.k2-self-formed-consistency-set.v1";
pub const K2_UNCERTAINTY_SYNTACTIC_MODEL_SCHEMA_V1: &str =
    "nando.k2-self-formed-syntactic-model.v1";

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyEffectCandidateV1 {
    pub effect: K2CompositionLearnedEffectV1,
    pub effect_root_sha256: String,
}

impl K2UncertaintyEffectCandidateV1 {
    pub fn seal(effect: K2CompositionLearnedEffectV1) -> K2CompositionResultV1<Self> {
        effect.validate()?;
        let effect_root_sha256 =
            uncertainty_root_v1(&("nando.k2-self-formed-effect-candidate.v1", &effect))?;
        Ok(Self {
            effect,
            effect_root_sha256,
        })
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.effect.validate()?;
        let expected =
            uncertainty_root_v1(&("nando.k2-self-formed-effect-candidate.v1", &self.effect))?;
        if self.effect_root_sha256 != expected {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_effect_candidate_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyConsistencyDispositionV1 {
    pub schema: String,
    pub opaque_action_root_sha256: String,
    pub effect: K2UncertaintyEffectCandidateV1,
    pub support_observation_root_sha256: String,
    pub predicted_observable_outcome_root_sha256: String,
    pub observed_observable_outcome_root_sha256: String,
    pub consistent: bool,
    pub disposition_root_sha256: String,
}

impl K2UncertaintyConsistencyDispositionV1 {
    pub fn seal(
        opaque_action_root_sha256: String,
        effect: K2UncertaintyEffectCandidateV1,
        support_observation_root_sha256: String,
        predicted_observable_outcome_root_sha256: String,
        observed_observable_outcome_root_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let consistent =
            predicted_observable_outcome_root_sha256 == observed_observable_outcome_root_sha256;
        let disposition_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONSISTENCY_SCHEMA_V1,
            &opaque_action_root_sha256,
            &effect,
            &support_observation_root_sha256,
            &predicted_observable_outcome_root_sha256,
            &observed_observable_outcome_root_sha256,
            consistent,
        ))?;
        let disposition = Self {
            schema: K2_UNCERTAINTY_CONSISTENCY_SCHEMA_V1.to_owned(),
            opaque_action_root_sha256,
            effect,
            support_observation_root_sha256,
            predicted_observable_outcome_root_sha256,
            observed_observable_outcome_root_sha256,
            consistent,
            disposition_root_sha256,
        };
        disposition.validate()?;
        Ok(disposition)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.opaque_action_root_sha256,
            &self.support_observation_root_sha256,
            &self.predicted_observable_outcome_root_sha256,
            &self.observed_observable_outcome_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.effect.validate()?;
        let consistent = self.predicted_observable_outcome_root_sha256
            == self.observed_observable_outcome_root_sha256;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONSISTENCY_SCHEMA_V1,
            &self.opaque_action_root_sha256,
            &self.effect,
            &self.support_observation_root_sha256,
            &self.predicted_observable_outcome_root_sha256,
            &self.observed_observable_outcome_root_sha256,
            consistent,
        ))?;
        if self.schema != K2_UNCERTAINTY_CONSISTENCY_SCHEMA_V1
            || self.consistent != consistent
            || self.disposition_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_consistency_disposition_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyConsistencySetV1 {
    pub schema: String,
    pub case_id_sha256: String,
    pub support_root_sha256: String,
    pub dispositions: Vec<K2UncertaintyConsistencyDispositionV1>,
    pub consistency_root_sha256: String,
}

impl K2UncertaintyConsistencySetV1 {
    pub fn seal(
        case_id_sha256: String,
        support_root_sha256: String,
        mut dispositions: Vec<K2UncertaintyConsistencyDispositionV1>,
    ) -> K2CompositionResultV1<Self> {
        dispositions.sort_by(|left, right| {
            (
                &left.opaque_action_root_sha256,
                &left.effect.effect_root_sha256,
                &left.support_observation_root_sha256,
            )
                .cmp(&(
                    &right.opaque_action_root_sha256,
                    &right.effect.effect_root_sha256,
                    &right.support_observation_root_sha256,
                ))
        });
        let consistency_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONSISTENCY_SET_SCHEMA_V1,
            &case_id_sha256,
            &support_root_sha256,
            &dispositions,
        ))?;
        let set = Self {
            schema: K2_UNCERTAINTY_CONSISTENCY_SET_SCHEMA_V1.to_owned(),
            case_id_sha256,
            support_root_sha256,
            dispositions,
            consistency_root_sha256,
        };
        set.validate()?;
        Ok(set)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.case_id_sha256)?;
        require_composition_root_v1(&self.support_root_sha256)?;
        require_exact_len_v1(
            self.dispositions.len(),
            K2_UNCERTAINTY_CONSISTENCY_DISPOSITIONS_V1,
            "self_formed_consistency_denominator_invalid",
        )?;
        let mut keys = BTreeSet::new();
        for disposition in &self.dispositions {
            disposition.validate()?;
            if !keys.insert((
                &disposition.opaque_action_root_sha256,
                &disposition.effect.effect_root_sha256,
                &disposition.support_observation_root_sha256,
            )) {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_consistency_duplicate",
                ));
            }
        }
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONSISTENCY_SET_SCHEMA_V1,
            &self.case_id_sha256,
            &self.support_root_sha256,
            &self.dispositions,
        ))?;
        if self.schema != K2_UNCERTAINTY_CONSISTENCY_SET_SCHEMA_V1
            || self.consistency_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_consistency_set_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyActionSurvivorsV1 {
    pub schema: String,
    pub opaque_action_root_sha256: String,
    pub effects: Vec<K2UncertaintyEffectCandidateV1>,
    pub survivors_root_sha256: String,
}

impl K2UncertaintyActionSurvivorsV1 {
    pub fn seal(
        opaque_action_root_sha256: String,
        mut effects: Vec<K2UncertaintyEffectCandidateV1>,
    ) -> K2CompositionResultV1<Self> {
        effects.sort();
        let survivors_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_ACTION_SURVIVORS_SCHEMA_V1,
            &opaque_action_root_sha256,
            &effects,
        ))?;
        let survivors = Self {
            schema: K2_UNCERTAINTY_ACTION_SURVIVORS_SCHEMA_V1.to_owned(),
            opaque_action_root_sha256,
            effects,
            survivors_root_sha256,
        };
        survivors.validate()?;
        Ok(survivors)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.opaque_action_root_sha256)?;
        if self.effects.is_empty() || self.effects.len() > K2_UNCERTAINTY_EFFECTS_PER_ACTION_V1 {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_action_survivor_count_invalid",
            ));
        }
        for effect in &self.effects {
            effect.validate()?;
        }
        require_sorted_unique_v1(&self.effects, "self_formed_action_survivors_not_unique")?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_ACTION_SURVIVORS_SCHEMA_V1,
            &self.opaque_action_root_sha256,
            &self.effects,
        ))?;
        if self.schema != K2_UNCERTAINTY_ACTION_SURVIVORS_SCHEMA_V1
            || self.survivors_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_action_survivors_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintySyntacticModelV1 {
    pub schema: String,
    pub actions: Vec<K2InquiryModelActionV1>,
    pub syntax_root_sha256: String,
}

impl K2UncertaintySyntacticModelV1 {
    pub fn seal(mut actions: Vec<K2InquiryModelActionV1>) -> K2CompositionResultV1<Self> {
        actions.sort();
        let syntax_root_sha256 =
            uncertainty_root_v1(&(K2_UNCERTAINTY_SYNTACTIC_MODEL_SCHEMA_V1, &actions))?;
        let model = Self {
            schema: K2_UNCERTAINTY_SYNTACTIC_MODEL_SCHEMA_V1.to_owned(),
            actions,
            syntax_root_sha256,
        };
        model.validate()?;
        Ok(model)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_exact_len_v1(
            self.actions.len(),
            K2_UNCERTAINTY_ACTIONS_V1,
            "self_formed_syntactic_model_action_count_invalid",
        )?;
        for action in &self.actions {
            action.validate()?;
        }
        require_sorted_unique_v1(&self.actions, "self_formed_syntactic_model_actions_invalid")?;
        let expected =
            uncertainty_root_v1(&(K2_UNCERTAINTY_SYNTACTIC_MODEL_SCHEMA_V1, &self.actions))?;
        if self.schema != K2_UNCERTAINTY_SYNTACTIC_MODEL_SCHEMA_V1
            || self.syntax_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_syntactic_model_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintySemanticSignatureV1 {
    pub schema: String,
    pub syntax_root_sha256: String,
    pub observable_outcome_roots_sha256: Vec<String>,
    pub semantic_signature_root_sha256: String,
}

impl K2UncertaintySemanticSignatureV1 {
    pub fn seal(
        syntax_root_sha256: String,
        observable_outcome_roots_sha256: Vec<String>,
    ) -> K2CompositionResultV1<Self> {
        let semantic_signature_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_SEMANTIC_SIGNATURE_SCHEMA_V1,
            &observable_outcome_roots_sha256,
        ))?;
        let signature = Self {
            schema: K2_UNCERTAINTY_SEMANTIC_SIGNATURE_SCHEMA_V1.to_owned(),
            syntax_root_sha256,
            observable_outcome_roots_sha256,
            semantic_signature_root_sha256,
        };
        signature.validate()?;
        Ok(signature)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.syntax_root_sha256)?;
        require_exact_len_v1(
            self.observable_outcome_roots_sha256.len(),
            K2_UNCERTAINTY_RAW_PROBES_V1,
            "self_formed_semantic_signature_denominator_invalid",
        )?;
        for root in &self.observable_outcome_roots_sha256 {
            require_composition_root_v1(root)?;
        }
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_SEMANTIC_SIGNATURE_SCHEMA_V1,
            &self.observable_outcome_roots_sha256,
        ))?;
        if self.schema != K2_UNCERTAINTY_SEMANTIC_SIGNATURE_SCHEMA_V1
            || self.semantic_signature_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_semantic_signature_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintySemanticClassV1 {
    pub schema: String,
    pub semantic_signature_root_sha256: String,
    pub syntax_member_roots_sha256: Vec<String>,
    pub representative_syntax_root_sha256: String,
    pub class_root_sha256: String,
}

impl K2UncertaintySemanticClassV1 {
    pub fn seal(
        semantic_signature_root_sha256: String,
        mut syntax_member_roots_sha256: Vec<String>,
    ) -> K2CompositionResultV1<Self> {
        syntax_member_roots_sha256.sort();
        let representative_syntax_root_sha256 =
            syntax_member_roots_sha256
                .first()
                .cloned()
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_semantic_class_empty",
                ))?;
        let class_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_SEMANTIC_CLASS_SCHEMA_V1,
            &semantic_signature_root_sha256,
            &syntax_member_roots_sha256,
            &representative_syntax_root_sha256,
        ))?;
        let class = Self {
            schema: K2_UNCERTAINTY_SEMANTIC_CLASS_SCHEMA_V1.to_owned(),
            semantic_signature_root_sha256,
            syntax_member_roots_sha256,
            representative_syntax_root_sha256,
            class_root_sha256,
        };
        class.validate()?;
        Ok(class)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.semantic_signature_root_sha256)?;
        require_sorted_unique_v1(
            &self.syntax_member_roots_sha256,
            "self_formed_semantic_class_members_invalid",
        )?;
        for root in &self.syntax_member_roots_sha256 {
            require_composition_root_v1(root)?;
        }
        let representative =
            self.syntax_member_roots_sha256
                .first()
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_semantic_class_empty",
                ))?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_SEMANTIC_CLASS_SCHEMA_V1,
            &self.semantic_signature_root_sha256,
            &self.syntax_member_roots_sha256,
            &self.representative_syntax_root_sha256,
        ))?;
        if self.schema != K2_UNCERTAINTY_SEMANTIC_CLASS_SCHEMA_V1
            || &self.representative_syntax_root_sha256 != representative
            || self.class_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_semantic_class_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyModelSetV1 {
    pub schema: String,
    pub case_id_sha256: String,
    pub vocabulary_root_sha256: String,
    pub support_root_sha256: String,
    pub consistency_root_sha256: String,
    pub raw_algebraic_model_count: u64,
    pub action_survivors: Vec<K2UncertaintyActionSurvivorsV1>,
    pub checked_product_count: u64,
    pub syntactic_models: Vec<K2UncertaintySyntacticModelV1>,
    pub semantic_signatures: Vec<K2UncertaintySemanticSignatureV1>,
    pub semantic_classes: Vec<K2UncertaintySemanticClassV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub model_set_root_sha256: String,
}

impl K2UncertaintyModelSetV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.case_id_sha256,
            &self.vocabulary_root_sha256,
            &self.support_root_sha256,
            &self.consistency_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_exact_len_v1(
            self.action_survivors.len(),
            K2_UNCERTAINTY_ACTIONS_V1,
            "self_formed_action_survivor_denominator_invalid",
        )?;
        for survivors in &self.action_survivors {
            survivors.validate()?;
        }
        if self
            .action_survivors
            .windows(2)
            .any(|pair| pair[0].opaque_action_root_sha256 >= pair[1].opaque_action_root_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_action_survivor_order_invalid",
            ));
        }
        let product =
            self.action_survivors
                .iter()
                .try_fold(1_u64, |value, action| {
                    value.checked_mul(action.effects.len() as u64).ok_or(
                        K2CompositionErrorV1::Invalid("self_formed_model_product_overflow"),
                    )
                })?;
        if self.raw_algebraic_model_count != K2_UNCERTAINTY_RAW_MODEL_COUNT_V1
            || self.checked_product_count != product
            || self.syntactic_models.len() as u64 != product
            || self.semantic_signatures.len() != self.syntactic_models.len()
            || self.semantic_classes.is_empty()
            || self.semantic_classes.len() > self.syntactic_models.len()
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_model_set_denominator_invalid",
            ));
        }
        for model in &self.syntactic_models {
            model.validate()?;
        }
        if self
            .syntactic_models
            .windows(2)
            .any(|pair| pair[0].syntax_root_sha256 >= pair[1].syntax_root_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_syntactic_models_not_canonical",
            ));
        }
        for signature in &self.semantic_signatures {
            signature.validate()?;
        }
        if self
            .semantic_signatures
            .windows(2)
            .any(|pair| pair[0].syntax_root_sha256 >= pair[1].syntax_root_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_semantic_signatures_not_canonical",
            ));
        }
        let model_roots = self
            .syntactic_models
            .iter()
            .map(|model| model.syntax_root_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let signature_roots = self
            .semantic_signatures
            .iter()
            .map(|signature| signature.syntax_root_sha256.as_str())
            .collect::<BTreeSet<_>>();
        if model_roots != signature_roots {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_semantic_signature_model_binding_invalid",
            ));
        }
        let signature_by_syntax = self
            .semantic_signatures
            .iter()
            .map(|signature| {
                (
                    signature.syntax_root_sha256.as_str(),
                    signature.semantic_signature_root_sha256.as_str(),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut class_members = BTreeSet::new();
        for class in &self.semantic_classes {
            class.validate()?;
            for member in &class.syntax_member_roots_sha256 {
                if signature_by_syntax.get(member.as_str()).copied()
                    != Some(class.semantic_signature_root_sha256.as_str())
                    || !class_members.insert(member.as_str())
                {
                    return Err(K2CompositionErrorV1::Invalid(
                        "self_formed_semantic_class_partition_invalid",
                    ));
                }
            }
        }
        if class_members != model_roots
            || self
                .semantic_classes
                .windows(2)
                .any(|pair| pair[0].class_root_sha256 >= pair[1].class_root_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_semantic_classes_not_canonical",
            ));
        }
        require_denied_authority_v1(&self.authority)?;
        let expected = self.expected_root()?;
        if self.schema != K2_UNCERTAINTY_MODEL_SET_SCHEMA_V1
            || self.model_set_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_model_set_invalid",
            ));
        }
        Ok(())
    }

    pub fn require_confirm_cardinality(&self) -> K2CompositionResultV1<()> {
        if self.syntactic_models.len() == K2_UNCERTAINTY_CONFIRM_MODELS_V1
            && self.semantic_classes.len() == K2_UNCERTAINTY_CONFIRM_MODELS_V1
        {
            Ok(())
        } else {
            Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_model_cardinality_invalid",
            ))
        }
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.model_set_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_MODEL_SET_SCHEMA_V1,
            &self.case_id_sha256,
            &self.vocabulary_root_sha256,
            &self.support_root_sha256,
            &self.consistency_root_sha256,
            self.raw_algebraic_model_count,
            &self.action_survivors,
            self.checked_product_count,
            &self.syntactic_models,
            &self.semantic_signatures,
            &self.semantic_classes,
            &self.authority,
        ))
    }
}

pub fn validate_model_set_against_support_v1(
    model_set: &K2UncertaintyModelSetV1,
    support: &K2UncertaintySupportSetV1,
) -> K2CompositionResultV1<()> {
    model_set.validate()?;
    support.validate()?;
    if model_set.case_id_sha256 != support.case_id_sha256
        || model_set.support_root_sha256 != support.support_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_model_support_binding_invalid",
        ));
    }
    Ok(())
}
