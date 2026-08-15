use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    K2CompositionErrorV1, K2CompositionFileEntryV1, K2CompositionLearnedEffectV1,
    K2CompositionResultV1, K2CompositionTreeManifestV1, K2InquiryModelActionV1,
};
use super::{
    K2_UNCERTAINTY_ACTIONS_V1, K2_UNCERTAINTY_EFFECTS_PER_ACTION_V1, K2_UNCERTAINTY_RAW_PROBES_V1,
    K2_UNCERTAINTY_STATE_COUNT_V1, K2UncertaintyActionSurvivorsV1,
    K2UncertaintyConsistencyDispositionV1, K2UncertaintyConsistencySetV1,
    K2UncertaintyDomainVocabularyV1, K2UncertaintyEffectCandidateV1,
    K2UncertaintyLearnerResponseV1, K2UncertaintyPublicCaseV1, K2UncertaintySemanticClassV1,
    K2UncertaintySemanticSignatureV1, K2UncertaintySupportOutcomeV1, K2UncertaintySyntacticModelV1,
    K2UncertaintyTransitionReasonV1, uncertainty_root_v1,
};

pub(super) struct IndependentInductionV1 {
    pub states: Vec<K2CompositionTreeManifestV1>,
    pub effects: Vec<K2CompositionLearnedEffectV1>,
    pub syntactic_models: Vec<K2UncertaintySyntacticModelV1>,
}

pub(super) fn verify_induction_v1(
    public_case: &K2UncertaintyPublicCaseV1,
    learned: &K2UncertaintyLearnerResponseV1,
) -> K2CompositionResultV1<IndependentInductionV1> {
    public_case.validate()?;
    learned.validate()?;
    let effects = independent_effects_v1(&public_case.vocabulary)?;
    let mut dispositions = Vec::new();
    let mut action_survivors = Vec::with_capacity(K2_UNCERTAINTY_ACTIONS_V1);
    for action_root in &public_case.vocabulary.opaque_action_roots_sha256 {
        let observations = public_case
            .support
            .observations
            .iter()
            .filter(|row| row.opaque_action_root_sha256 == *action_root)
            .collect::<Vec<_>>();
        if observations.len() != 3 {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_final_support_count_invalid",
            ));
        }
        let mut survivors = Vec::new();
        for effect in &effects {
            let candidate = K2UncertaintyEffectCandidateV1::seal(effect.clone())?;
            let mut all_consistent = true;
            for observation in &observations {
                let predicted = independent_support_outcome_v1(&observation.pre_manifest, effect)?;
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
        public_case.vocabulary.case_id_sha256.clone(),
        public_case.support.support_root_sha256.clone(),
        dispositions,
    )?;
    if consistency != learned.consistency || action_survivors != learned.model_set.action_survivors
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_induction_consistency_mismatch",
        ));
    }

    let checked_product = action_survivors.iter().try_fold(1_u64, |count, action| {
        count
            .checked_mul(action.effects.len() as u64)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_final_model_count_overflow",
            ))
    })?;
    let mut syntactic_models = Vec::new();
    independent_materialize_models_v1(
        &action_survivors,
        0,
        &mut Vec::new(),
        &mut syntactic_models,
    )?;
    syntactic_models.sort_by(|left, right| left.syntax_root_sha256.cmp(&right.syntax_root_sha256));
    if checked_product != 4 || syntactic_models != learned.model_set.syntactic_models {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_syntactic_models_mismatch",
        ));
    }

    let states = independent_states_v1(&public_case.vocabulary)?;
    let mut signatures = Vec::with_capacity(syntactic_models.len());
    for model in &syntactic_models {
        let mut outcomes = Vec::with_capacity(K2_UNCERTAINTY_RAW_PROBES_V1);
        for action in &model.actions {
            for state in &states {
                outcomes.push(
                    independent_support_outcome_v1(state, &action.effect)?
                        .observable_outcome_root_sha256,
                );
            }
        }
        signatures.push(K2UncertaintySemanticSignatureV1::seal(
            model.syntax_root_sha256.clone(),
            outcomes,
        )?);
    }
    signatures.sort_by(|left, right| left.syntax_root_sha256.cmp(&right.syntax_root_sha256));
    let mut quotient = BTreeMap::<Vec<String>, Vec<String>>::new();
    for signature in &signatures {
        quotient
            .entry(signature.observable_outcome_roots_sha256.clone())
            .or_default()
            .push(signature.syntax_root_sha256.clone());
    }
    let mut classes = quotient
        .into_iter()
        .map(|(outcomes, members)| {
            K2UncertaintySemanticClassV1::seal(
                uncertainty_root_v1(&(
                    super::K2_UNCERTAINTY_SEMANTIC_SIGNATURE_SCHEMA_V1,
                    outcomes,
                ))?,
                members,
            )
        })
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    classes.sort_by(|left, right| left.class_root_sha256.cmp(&right.class_root_sha256));
    if signatures != learned.model_set.semantic_signatures
        || classes != learned.model_set.semantic_classes
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_semantic_quotient_mismatch",
        ));
    }
    let expected_classes = classes
        .iter()
        .map(|class| class.class_root_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let actual_classes = learned
        .world_models
        .iter()
        .map(|model| model.model_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    if expected_classes != actual_classes {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_world_model_class_mismatch",
        ));
    }
    for class in &classes {
        let syntax = syntactic_models
            .iter()
            .find(|model| model.syntax_root_sha256 == class.representative_syntax_root_sha256)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_final_representative_missing",
            ))?;
        let world = learned
            .world_models
            .iter()
            .find(|model| model.model_id_sha256 == class.class_root_sha256)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_final_world_model_missing",
            ))?;
        if world.actions != syntax.actions {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_final_world_model_actions_mismatch",
            ));
        }
    }
    Ok(IndependentInductionV1 {
        states,
        effects,
        syntactic_models,
    })
}

pub(super) fn independent_effects_v1(
    vocabulary: &K2UncertaintyDomainVocabularyV1,
) -> K2CompositionResultV1<Vec<K2CompositionLearnedEffectV1>> {
    vocabulary.validate()?;
    let mut effects = Vec::new();
    for source in &vocabulary.path_atoms {
        for target in &vocabulary.path_atoms {
            if source.path != target.path {
                effects.push(K2CompositionLearnedEffectV1::CopyFile {
                    source_path: source.path.clone(),
                    target_path: target.path.clone(),
                });
            }
        }
    }
    for path in &vocabulary.path_atoms {
        effects.push(K2CompositionLearnedEffectV1::RemoveFile {
            path: path.path.clone(),
        });
    }
    effects.sort();
    if effects.len() != K2_UNCERTAINTY_EFFECTS_PER_ACTION_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_effect_denominator_invalid",
        ));
    }
    Ok(effects)
}

pub(super) fn independent_states_v1(
    vocabulary: &K2UncertaintyDomainVocabularyV1,
) -> K2CompositionResultV1<Vec<K2CompositionTreeManifestV1>> {
    let mut manifests = Vec::with_capacity(K2_UNCERTAINTY_STATE_COUNT_V1);
    for encoded in 0..K2_UNCERTAINTY_STATE_COUNT_V1 {
        let mut value = encoded;
        let mut entries = Vec::new();
        for path in &vocabulary.path_atoms {
            let state = value % 4;
            value /= 4;
            if state > 0 {
                let content = &vocabulary.content_atoms[state - 1];
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

pub(super) fn independent_apply_manifest_v1(
    manifest: &K2CompositionTreeManifestV1,
    effect: &K2CompositionLearnedEffectV1,
) -> K2CompositionResultV1<(bool, &'static str, K2CompositionTreeManifestV1)> {
    let mut entries = manifest
        .entries
        .iter()
        .cloned()
        .map(|entry| (entry.path.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    let (applied, reason) = match effect {
        K2CompositionLearnedEffectV1::CopyFile {
            source_path,
            target_path,
        } => match entries.get(source_path).cloned() {
            Some(mut source) => {
                source.path = target_path.clone();
                entries.insert(target_path.clone(), source);
                (true, "applied")
            }
            None => (false, "copy_source_missing"),
        },
        K2CompositionLearnedEffectV1::RemoveFile { path } => {
            if entries.remove(path).is_some() {
                (true, "applied")
            } else {
                (false, "remove_path_missing")
            }
        }
    };
    Ok((
        applied,
        reason,
        K2CompositionTreeManifestV1::seal_entries(entries.into_values().collect())?,
    ))
}

fn independent_support_outcome_v1(
    manifest: &K2CompositionTreeManifestV1,
    effect: &K2CompositionLearnedEffectV1,
) -> K2CompositionResultV1<K2UncertaintySupportOutcomeV1> {
    let (applied, reason, post) = independent_apply_manifest_v1(manifest, effect)?;
    let reason = match (applied, reason) {
        (true, "applied") => K2UncertaintyTransitionReasonV1::Applied,
        (false, "copy_source_missing") => K2UncertaintyTransitionReasonV1::CopySourceMissing,
        (false, "remove_path_missing") => K2UncertaintyTransitionReasonV1::RemovePathMissing,
        _ => {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_final_transition_reason_invalid",
            ));
        }
    };
    K2UncertaintySupportOutcomeV1::seal(reason, post)
}

fn independent_materialize_models_v1(
    survivors: &[K2UncertaintyActionSurvivorsV1],
    action_index: usize,
    current: &mut Vec<K2InquiryModelActionV1>,
    output: &mut Vec<K2UncertaintySyntacticModelV1>,
) -> K2CompositionResultV1<()> {
    if action_index == survivors.len() {
        output.push(K2UncertaintySyntacticModelV1::seal(current.clone())?);
        return Ok(());
    }
    for effect in &survivors[action_index].effects {
        current.push(K2InquiryModelActionV1 {
            action_id_sha256: survivors[action_index].opaque_action_root_sha256.clone(),
            effect: effect.effect.clone(),
        });
        independent_materialize_models_v1(survivors, action_index + 1, current, output)?;
        current.pop();
    }
    Ok(())
}
