use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionFileEntryV1,
    K2CompositionLearnedEffectV1, K2CompositionResultV1, K2CompositionTreeManifestV1,
    K2InquiryModelActionV1, K2InquiryWorldModelV1, composition_bytes_v1,
    composition_sha256_file_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_ACTIONS_V1, K2_UNCERTAINTY_EFFECTS_PER_ACTION_V1,
    K2_UNCERTAINTY_LEARNER_RESPONSE_SCHEMA_V1, K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1,
    K2_UNCERTAINTY_MODEL_SET_SCHEMA_V1, K2_UNCERTAINTY_RAW_MODEL_COUNT_V1,
    K2_UNCERTAINTY_STATE_COUNT_V1, K2UncertaintyActionSurvivorsV1,
    K2UncertaintyConsistencyDispositionV1, K2UncertaintyConsistencySetV1,
    K2UncertaintyEffectCandidateV1, K2UncertaintyLearnerRequestV1, K2UncertaintyModelSetV1,
    K2UncertaintySemanticClassV1, K2UncertaintySemanticSignatureV1, K2UncertaintySupportOutcomeV1,
    K2UncertaintySyntacticModelV1, K2UncertaintyTransitionReasonV1, denied_authority_v1,
    require_denied_authority_v1, require_exact_len_v1, uncertainty_decode_v1, uncertainty_root_v1,
};

const MAX_DEVELOPMENT_MATERIALIZED_MODELS_V1: usize = 6;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyLearnerResponseV1 {
    pub schema: String,
    pub learner_request_root_sha256: String,
    pub consistency: K2UncertaintyConsistencySetV1,
    pub model_set: K2UncertaintyModelSetV1,
    pub world_models: Vec<K2InquiryWorldModelV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub response_root_sha256: String,
}

impl K2UncertaintyLearnerResponseV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.learner_request_root_sha256)?;
        self.consistency.validate()?;
        self.model_set.validate()?;
        if self.consistency.consistency_root_sha256 != self.model_set.consistency_root_sha256
            || self.consistency.case_id_sha256 != self.model_set.case_id_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_learner_consistency_binding_invalid",
            ));
        }
        require_exact_len_v1(
            self.world_models.len(),
            self.model_set.semantic_classes.len(),
            "self_formed_world_model_count_invalid",
        )?;
        let classes = self
            .model_set
            .semantic_classes
            .iter()
            .map(|class| class.class_root_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let mut model_classes = BTreeSet::new();
        for model in &self.world_models {
            model.validate()?;
            if model.experiment_id_sha256 != self.model_set.case_id_sha256
                || model.common_evidence_root_sha256 != self.model_set.support_root_sha256
                || model.source_neutral_provenance_root_sha256
                    != self.model_set.model_set_root_sha256
                || !model_classes.insert(model.model_id_sha256.as_str())
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_world_model_binding_invalid",
                ));
            }
        }
        if classes != model_classes
            || self
                .world_models
                .windows(2)
                .any(|pair| pair[0].model_root_sha256 >= pair[1].model_root_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_world_models_not_canonical",
            ));
        }
        require_denied_authority_v1(&self.authority)?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_LEARNER_RESPONSE_SCHEMA_V1,
            &self.learner_request_root_sha256,
            &self.consistency,
            &self.model_set,
            &self.world_models,
            &self.authority,
        ))?;
        if self.schema != K2_UNCERTAINTY_LEARNER_RESPONSE_SCHEMA_V1
            || self.response_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_learner_response_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.authority = denied_authority_v1();
        self.response_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_LEARNER_RESPONSE_SCHEMA_V1,
            &self.learner_request_root_sha256,
            &self.consistency,
            &self.model_set,
            &self.world_models,
            &self.authority,
        ))?;
        self.validate()
    }
}

pub fn learn_self_formed_uncertainty_v1(
    request: &K2UncertaintyLearnerRequestV1,
) -> K2CompositionResultV1<K2UncertaintyLearnerResponseV1> {
    request.validate()?;
    let effects = learner_enumerate_effects_v1(request)?;
    let mut dispositions = Vec::new();
    let mut action_survivors = Vec::with_capacity(K2_UNCERTAINTY_ACTIONS_V1);
    for action_root in &request.vocabulary.opaque_action_roots_sha256 {
        let observations = request
            .support
            .observations
            .iter()
            .filter(|row| &row.opaque_action_root_sha256 == action_root)
            .collect::<Vec<_>>();
        require_exact_len_v1(
            observations.len(),
            super::K2_UNCERTAINTY_SUPPORT_ROWS_PER_ACTION_V1,
            "self_formed_learner_action_support_invalid",
        )?;
        let mut survivors = Vec::new();
        for effect in &effects {
            let candidate = K2UncertaintyEffectCandidateV1::seal(effect.clone())?;
            let mut all_consistent = true;
            for observation in &observations {
                let predicted = learner_apply_effect_v1(&observation.pre_manifest, effect)?;
                let disposition = K2UncertaintyConsistencyDispositionV1::seal(
                    action_root.clone(),
                    candidate.clone(),
                    observation.observation_root_sha256.clone(),
                    predicted.observable_outcome_root_sha256,
                    observation.outcome.observable_outcome_root_sha256.clone(),
                )?;
                all_consistent &= disposition.consistent;
                dispositions.push(disposition);
            }
            if all_consistent {
                survivors.push(candidate);
            }
        }
        action_survivors.push(K2UncertaintyActionSurvivorsV1::seal(
            action_root.clone(),
            survivors,
        )?);
    }
    action_survivors.sort_by(|left, right| {
        left.opaque_action_root_sha256
            .cmp(&right.opaque_action_root_sha256)
    });
    let consistency = K2UncertaintyConsistencySetV1::seal(
        request.vocabulary.case_id_sha256.clone(),
        request.support.support_root_sha256.clone(),
        dispositions,
    )?;
    let checked_product_count = action_survivors.iter().try_fold(1_u64, |value, action| {
        value
            .checked_mul(action.effects.len() as u64)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_learner_model_count_overflow",
            ))
    })?;
    if checked_product_count == 0
        || checked_product_count > MAX_DEVELOPMENT_MATERIALIZED_MODELS_V1 as u64
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_learner_model_count_out_of_bounds",
        ));
    }
    let mut syntactic_models = Vec::with_capacity(checked_product_count as usize);
    materialize_models_v1(
        &action_survivors,
        0,
        &mut Vec::with_capacity(K2_UNCERTAINTY_ACTIONS_V1),
        &mut syntactic_models,
    )?;
    syntactic_models.sort_by(|left, right| left.syntax_root_sha256.cmp(&right.syntax_root_sha256));
    let states = learner_enumerate_states_v1(request)?;
    let mut signatures = Vec::with_capacity(syntactic_models.len());
    for model in &syntactic_models {
        signatures.push(semantic_signature_v1(model, &states)?);
    }
    signatures.sort_by(|left, right| left.syntax_root_sha256.cmp(&right.syntax_root_sha256));
    let mut quotient = BTreeMap::<Vec<String>, Vec<String>>::new();
    for signature in &signatures {
        quotient
            .entry(signature.observable_outcome_roots_sha256.clone())
            .or_default()
            .push(signature.syntax_root_sha256.clone());
    }
    let mut semantic_classes = quotient
        .into_iter()
        .map(|(outcomes, members)| {
            let signature_root = uncertainty_root_v1(&(
                super::K2_UNCERTAINTY_SEMANTIC_SIGNATURE_SCHEMA_V1,
                &outcomes,
            ))?;
            K2UncertaintySemanticClassV1::seal(signature_root, members)
        })
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    semantic_classes.sort_by(|left, right| left.class_root_sha256.cmp(&right.class_root_sha256));
    let mut model_set = K2UncertaintyModelSetV1 {
        schema: K2_UNCERTAINTY_MODEL_SET_SCHEMA_V1.to_owned(),
        case_id_sha256: request.vocabulary.case_id_sha256.clone(),
        vocabulary_root_sha256: request.vocabulary.vocabulary_root_sha256.clone(),
        support_root_sha256: request.support.support_root_sha256.clone(),
        consistency_root_sha256: consistency.consistency_root_sha256.clone(),
        raw_algebraic_model_count: K2_UNCERTAINTY_RAW_MODEL_COUNT_V1,
        action_survivors,
        checked_product_count,
        syntactic_models,
        semantic_signatures: signatures,
        semantic_classes,
        authority: denied_authority_v1(),
        model_set_root_sha256: String::new(),
    };
    model_set.reseal()?;
    let mut world_models = Vec::with_capacity(model_set.semantic_classes.len());
    for class in &model_set.semantic_classes {
        let syntax = model_set
            .syntactic_models
            .iter()
            .find(|model| model.syntax_root_sha256 == class.representative_syntax_root_sha256)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_learner_representative_missing",
            ))?;
        world_models.push(K2InquiryWorldModelV1::seal(
            request.vocabulary.case_id_sha256.clone(),
            class.class_root_sha256.clone(),
            request.support.support_root_sha256.clone(),
            model_set.model_set_root_sha256.clone(),
            syntax.actions.clone(),
        )?);
    }
    world_models.sort_by(|left, right| left.model_root_sha256.cmp(&right.model_root_sha256));
    let mut response = K2UncertaintyLearnerResponseV1 {
        schema: K2_UNCERTAINTY_LEARNER_RESPONSE_SCHEMA_V1.to_owned(),
        learner_request_root_sha256: request.request_root_sha256.clone(),
        consistency,
        model_set,
        world_models,
        authority: denied_authority_v1(),
        response_root_sha256: String::new(),
    };
    response.reseal()?;
    Ok(response)
}

pub fn run_self_formed_learner_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_learner_stdin"))?;
    let request: K2UncertaintyLearnerRequestV1 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_learner"))?;
    if composition_sha256_file_v1(&executable)? != request.learner_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_learner_executable_mismatch",
        ));
    }
    let response = learn_self_formed_uncertainty_v1(&request)?;
    std::io::stdout()
        .write_all(&composition_bytes_v1(&response)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_learner_stdout"))
}

fn learner_enumerate_effects_v1(
    request: &K2UncertaintyLearnerRequestV1,
) -> K2CompositionResultV1<Vec<K2CompositionLearnedEffectV1>> {
    let mut effects = Vec::with_capacity(K2_UNCERTAINTY_EFFECTS_PER_ACTION_V1);
    for source in &request.vocabulary.path_atoms {
        for target in &request.vocabulary.path_atoms {
            if source.path != target.path {
                effects.push(K2CompositionLearnedEffectV1::CopyFile {
                    source_path: source.path.clone(),
                    target_path: target.path.clone(),
                });
            }
        }
    }
    for path in &request.vocabulary.path_atoms {
        effects.push(K2CompositionLearnedEffectV1::RemoveFile {
            path: path.path.clone(),
        });
    }
    effects.sort();
    require_exact_len_v1(
        effects.len(),
        K2_UNCERTAINTY_EFFECTS_PER_ACTION_V1,
        "self_formed_learner_effect_denominator_invalid",
    )?;
    Ok(effects)
}

fn learner_enumerate_states_v1(
    request: &K2UncertaintyLearnerRequestV1,
) -> K2CompositionResultV1<Vec<K2CompositionTreeManifestV1>> {
    let mut manifests = Vec::with_capacity(K2_UNCERTAINTY_STATE_COUNT_V1);
    for encoded in 0..K2_UNCERTAINTY_STATE_COUNT_V1 {
        let mut value = encoded;
        let mut entries = Vec::new();
        for path in &request.vocabulary.path_atoms {
            let state = value % 4;
            value /= 4;
            if state > 0 {
                let content = &request.vocabulary.content_atoms[state - 1];
                entries.push(K2CompositionFileEntryV1 {
                    path: path.path.clone(),
                    content_sha256: content.bytes_sha256.clone(),
                    byte_len: content.byte_len,
                });
            }
        }
        manifests.push(K2CompositionTreeManifestV1::seal_entries(entries)?);
    }
    manifests.sort_by(|left, right| left.tree_root_sha256.cmp(&right.tree_root_sha256));
    Ok(manifests)
}

fn materialize_models_v1(
    survivors: &[K2UncertaintyActionSurvivorsV1],
    action_index: usize,
    current: &mut Vec<K2InquiryModelActionV1>,
    models: &mut Vec<K2UncertaintySyntacticModelV1>,
) -> K2CompositionResultV1<()> {
    if action_index == survivors.len() {
        models.push(K2UncertaintySyntacticModelV1::seal(current.clone())?);
        return Ok(());
    }
    let action = &survivors[action_index];
    for effect in &action.effects {
        current.push(K2InquiryModelActionV1 {
            action_id_sha256: action.opaque_action_root_sha256.clone(),
            effect: effect.effect.clone(),
        });
        materialize_models_v1(survivors, action_index + 1, current, models)?;
        current.pop();
    }
    Ok(())
}

fn semantic_signature_v1(
    model: &K2UncertaintySyntacticModelV1,
    states: &[K2CompositionTreeManifestV1],
) -> K2CompositionResultV1<K2UncertaintySemanticSignatureV1> {
    let mut outcomes = Vec::with_capacity(super::K2_UNCERTAINTY_RAW_PROBES_V1);
    for action in &model.actions {
        for state in states {
            outcomes.push(
                learner_apply_effect_v1(state, &action.effect)?.observable_outcome_root_sha256,
            );
        }
    }
    K2UncertaintySemanticSignatureV1::seal(model.syntax_root_sha256.clone(), outcomes)
}

fn learner_apply_effect_v1(
    manifest: &K2CompositionTreeManifestV1,
    effect: &K2CompositionLearnedEffectV1,
) -> K2CompositionResultV1<K2UncertaintySupportOutcomeV1> {
    manifest.validate()?;
    effect.validate()?;
    let mut entries = manifest
        .entries
        .iter()
        .cloned()
        .map(|entry| (entry.path.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    let reason = match effect {
        K2CompositionLearnedEffectV1::CopyFile {
            source_path,
            target_path,
        } => match entries.get(source_path).cloned() {
            Some(mut source) => {
                source.path = target_path.clone();
                entries.insert(target_path.clone(), source);
                K2UncertaintyTransitionReasonV1::Applied
            }
            None => K2UncertaintyTransitionReasonV1::CopySourceMissing,
        },
        K2CompositionLearnedEffectV1::RemoveFile { path } => {
            if entries.remove(path).is_some() {
                K2UncertaintyTransitionReasonV1::Applied
            } else {
                K2UncertaintyTransitionReasonV1::RemovePathMissing
            }
        }
    };
    K2UncertaintySupportOutcomeV1::seal(
        reason,
        K2CompositionTreeManifestV1::seal_entries(entries.into_values().collect())?,
    )
}
