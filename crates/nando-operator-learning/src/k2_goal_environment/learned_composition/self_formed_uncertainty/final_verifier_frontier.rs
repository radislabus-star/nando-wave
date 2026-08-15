use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    K2CompositionErrorV1, K2CompositionLearnedEffectV1, K2CompositionResultV1,
    K2CompositionTreeManifestV1, K2InquiryObservationModeV1, K2InquiryProbeV1,
};
use super::final_verifier_induction::{IndependentInductionV1, independent_apply_manifest_v1};
use super::{
    K2_UNCERTAINTY_EFFECT_ACCOUNTING_SCHEMA_V1, K2_UNCERTAINTY_PREDICTION_WITNESS_SCHEMA_V1,
    K2_UNCERTAINTY_PROBE_EQUIVALENCE_KEY_SCHEMA_V1, K2_UNCERTAINTY_RAW_PROBE_SCHEMA_V1,
    K2_UNCERTAINTY_RISK_COST_SCHEMA_V1, K2_UNCERTAINTY_ROBUST_ACCOUNTING_SCHEMA_V1,
    K2UncertaintyEffectAccountingV1, K2UncertaintyEffectCandidateV1,
    K2UncertaintyEligibilityDispositionV1, K2UncertaintyLearnerResponseV1,
    K2UncertaintyPredictionWitnessV1, K2UncertaintyProbeClassV1,
    K2UncertaintyProbeEquivalenceKeyV1, K2UncertaintyProbeOutputV1, K2UncertaintyPublicCaseV1,
    K2UncertaintyRawProbeDispositionV1, K2UncertaintyRiskCostV1, K2UncertaintyRobustAccountingV1,
    K2UncertaintySafetyDispositionV1, uncertainty_root_v1,
};

pub(super) struct IndependentFrontierV1 {
    pub representatives: BTreeMap<String, K2UncertaintyRawProbeDispositionV1>,
}

pub(super) fn verify_frontier_v1(
    public_case: &K2UncertaintyPublicCaseV1,
    learned: &K2UncertaintyLearnerResponseV1,
    induction: &IndependentInductionV1,
    output: &K2UncertaintyProbeOutputV1,
    split_commitment_root_sha256: &str,
) -> K2CompositionResultV1<IndependentFrontierV1> {
    output.validate()?;
    if output.state_universe.manifests != induction.states
        || output.state_universe.vocabulary_root_sha256
            != public_case.vocabulary.vocabulary_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_state_universe_mismatch",
        ));
    }
    let actual = output
        .pages
        .iter()
        .flat_map(|page| &page.dispositions)
        .collect::<Vec<_>>();
    let mut expected = Vec::with_capacity(1_792);
    for (action_index, action_root) in public_case
        .vocabulary
        .opaque_action_roots_sha256
        .iter()
        .enumerate()
    {
        for (state_index, state) in induction.states.iter().enumerate() {
            expected.push(independent_disposition_v1(
                public_case,
                learned,
                induction,
                split_commitment_root_sha256,
                action_root,
                state,
                (action_index * induction.states.len() + state_index) as u64,
            )?);
        }
    }
    if actual.len() != expected.len()
        || actual
            .iter()
            .zip(&expected)
            .any(|(actual, expected)| *actual != expected)
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_raw_frontier_mismatch",
        ));
    }

    let mut quotient = BTreeMap::new();
    for disposition in &expected {
        quotient
            .entry(disposition.equivalence_key.clone())
            .or_insert_with(Vec::new)
            .push(disposition.probe.probe_root_sha256.clone());
    }
    let mut classes = quotient
        .into_iter()
        .map(|(key, mut members)| {
            members.sort();
            let representative = members
                .first()
                .cloned()
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_final_probe_class_empty",
                ))?;
            let mut class = K2UncertaintyProbeClassV1 {
                schema: super::K2_UNCERTAINTY_PROBE_CLASS_SCHEMA_V1.to_owned(),
                equivalence_key: key,
                member_probe_roots_sha256: members,
                representative_probe_root_sha256: representative,
                class_root_sha256: String::new(),
            };
            class.reseal()?;
            Ok(class)
        })
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    classes.sort_by(|left, right| left.class_root_sha256.cmp(&right.class_root_sha256));
    if classes != output.frontier.classes {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_probe_quotient_mismatch",
        ));
    }
    let representative_roots = output
        .frontier
        .representative_probe_roots_sha256
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let representatives = expected
        .into_iter()
        .filter(|disposition| representative_roots.contains(&disposition.probe.probe_root_sha256))
        .map(|disposition| (disposition.probe.probe_root_sha256.clone(), disposition))
        .collect::<BTreeMap<_, _>>();
    if representatives.len() != representative_roots.len() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_representative_coverage_invalid",
        ));
    }
    Ok(IndependentFrontierV1 { representatives })
}

#[allow(clippy::too_many_arguments)]
fn independent_disposition_v1(
    public_case: &K2UncertaintyPublicCaseV1,
    learned: &K2UncertaintyLearnerResponseV1,
    induction: &IndependentInductionV1,
    split_commitment_root_sha256: &str,
    action_root: &str,
    state: &K2CompositionTreeManifestV1,
    raw_sequence: u64,
) -> K2CompositionResultV1<K2UncertaintyRawProbeDispositionV1> {
    let probe_id_sha256 = uncertainty_root_v1(&(
        "nando.k2-self-formed-mechanical-probe.v1",
        &public_case.vocabulary.case_id_sha256,
        raw_sequence,
        &state.tree_root_sha256,
        action_root,
    ))?;
    let robust =
        independent_robust_accounting_v1(&public_case.vocabulary, state, &induction.effects)?;
    let provenance = uncertainty_root_v1(&(
        "nando.k2-inquiry-generated-probe-provenance.v1",
        &public_case.vocabulary.case_id_sha256,
        &public_case.vocabulary.generator_schema_root_sha256,
        split_commitment_root_sha256,
        &probe_id_sha256,
        action_root,
    ))?;
    let mut predictions = Vec::with_capacity(learned.world_models.len());
    for model in &learned.world_models {
        let effect = model
            .effect(action_root)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_final_model_action_missing",
            ))?;
        let (applied, reason, post) = independent_apply_manifest_v1(state, effect)?;
        let observable_outcome_root_sha256 =
            uncertainty_root_v1(&("nando.k2-inquiry-observable-exact-manifest.v1", &post))?;
        let witness = K2UncertaintyPredictionWitnessV1 {
            schema: K2_UNCERTAINTY_PREDICTION_WITNESS_SCHEMA_V1.to_owned(),
            model_root_sha256: model.model_root_sha256.clone(),
            probe_root_sha256: String::new(),
            transition_applied: applied,
            transition_reason: reason.to_owned(),
            predicted_post_manifest: post,
            observable_outcome_root_sha256,
            prediction_root_sha256: String::new(),
        };
        predictions.push(witness);
    }
    predictions.sort_by(|left, right| left.model_root_sha256.cmp(&right.model_root_sha256));
    let applicability_hint = predictions
        .iter()
        .all(|prediction| prediction.transition_applied);
    let dependency_hint = state.entries.len() >= 2;
    let cleanup_hint = robust.maximum_risk_units <= 1;
    let probe = K2InquiryProbeV1::seal(
        public_case.vocabulary.case_id_sha256.clone(),
        probe_id_sha256,
        action_root.to_owned(),
        state.clone(),
        true,
        K2InquiryObservationModeV1::ExactImmediate,
        robust.maximum_risk_units,
        robust.maximum_cost_units,
        applicability_hint,
        dependency_hint,
        cleanup_hint,
        provenance,
    )?;
    for prediction in &mut predictions {
        prediction.probe_root_sha256 = probe.probe_root_sha256.clone();
        prediction.reseal()?;
    }
    let roots = predictions
        .iter()
        .map(|prediction| prediction.observable_outcome_root_sha256.as_str())
        .collect::<Vec<_>>();
    let pairwise = [
        roots[0] == roots[1],
        roots[0] == roots[2],
        roots[0] == roots[3],
        roots[1] == roots[2],
        roots[1] == roots[3],
        roots[2] == roots[3],
    ];
    let mut equivalence_key = K2UncertaintyProbeEquivalenceKeyV1 {
        schema: K2_UNCERTAINTY_PROBE_EQUIVALENCE_KEY_SCHEMA_V1.to_owned(),
        pairwise_outcome_equal: pairwise,
        eligibility: K2UncertaintyEligibilityDispositionV1::Eligible,
        safety: K2UncertaintySafetyDispositionV1::Pass,
        risk_units: probe.risk_units,
        cost_units: probe.cost_units,
        applicability_hint,
        dependency_hint,
        cleanup_hint,
        key_root_sha256: String::new(),
    };
    equivalence_key.reseal()?;
    let mut disposition = K2UncertaintyRawProbeDispositionV1 {
        schema: K2_UNCERTAINTY_RAW_PROBE_SCHEMA_V1.to_owned(),
        raw_sequence,
        probe,
        predictions,
        robust_accounting: robust,
        eligibility: K2UncertaintyEligibilityDispositionV1::Eligible,
        safety: K2UncertaintySafetyDispositionV1::Pass,
        equivalence_key,
        raw_probe_root_sha256: String::new(),
    };
    disposition.reseal()?;
    Ok(disposition)
}

fn independent_robust_accounting_v1(
    vocabulary: &super::K2UncertaintyDomainVocabularyV1,
    state: &K2CompositionTreeManifestV1,
    effects: &[K2CompositionLearnedEffectV1],
) -> K2CompositionResultV1<K2UncertaintyRobustAccountingV1> {
    let mut receipts = Vec::with_capacity(effects.len());
    for effect in effects {
        let candidate = K2UncertaintyEffectCandidateV1::seal(effect.clone())?;
        let mut receipt = K2UncertaintyEffectAccountingV1 {
            schema: K2_UNCERTAINTY_EFFECT_ACCOUNTING_SCHEMA_V1.to_owned(),
            effect_root_sha256: candidate.effect_root_sha256,
            accounting: independent_accounting_v1(vocabulary, state, effect)?,
            effect_accounting_root_sha256: String::new(),
        };
        receipt.reseal()?;
        receipts.push(receipt);
    }
    let mut robust = K2UncertaintyRobustAccountingV1 {
        schema: K2_UNCERTAINTY_ROBUST_ACCOUNTING_SCHEMA_V1.to_owned(),
        effects: receipts,
        maximum_risk_units: 0,
        maximum_cost_units: 0,
        robust_accounting_root_sha256: String::new(),
    };
    robust.reseal()?;
    Ok(robust)
}

pub(super) fn independent_accounting_v1(
    vocabulary: &super::K2UncertaintyDomainVocabularyV1,
    state: &K2CompositionTreeManifestV1,
    effect: &K2CompositionLearnedEffectV1,
) -> K2CompositionResultV1<K2UncertaintyRiskCostV1> {
    let mut value = K2UncertaintyRiskCostV1 {
        schema: K2_UNCERTAINTY_RISK_COST_SCHEMA_V1.to_owned(),
        read_entries: 1,
        written_or_removed_entries: 0,
        overwritten_existing_entries: 0,
        removed_existing_entries: 0,
        overwritten_bytes: 0,
        removed_bytes: 0,
        touched_bytes: 0,
        risk_units: 0,
        cost_units: 0,
        accounting_root_sha256: String::new(),
    };
    match effect {
        K2CompositionLearnedEffectV1::CopyFile {
            source_path,
            target_path,
        } => {
            if let Some(source) = state.entry(source_path) {
                independent_require_content_v1(vocabulary, source)?;
                value.written_or_removed_entries = 1;
                value.touched_bytes = source.byte_len;
                if let Some(target) = state.entry(target_path) {
                    independent_require_content_v1(vocabulary, target)?;
                    value.overwritten_existing_entries = 1;
                    value.overwritten_bytes = target.byte_len;
                    value.touched_bytes = value.touched_bytes.checked_add(target.byte_len).ok_or(
                        K2CompositionErrorV1::Invalid(
                            "self_formed_final_accounting_bytes_overflow",
                        ),
                    )?;
                }
            }
        }
        K2CompositionLearnedEffectV1::RemoveFile { path } => {
            if let Some(existing) = state.entry(path) {
                independent_require_content_v1(vocabulary, existing)?;
                value.written_or_removed_entries = 1;
                value.removed_existing_entries = 1;
                value.removed_bytes = existing.byte_len;
                value.touched_bytes = existing.byte_len;
            }
        }
    }
    value.reseal()?;
    Ok(value)
}

fn independent_require_content_v1(
    vocabulary: &super::K2UncertaintyDomainVocabularyV1,
    entry: &super::super::K2CompositionFileEntryV1,
) -> K2CompositionResultV1<()> {
    if vocabulary
        .content_by_sha256(&entry.content_sha256)
        .is_none_or(|content| content.byte_len != entry.byte_len)
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_accounting_content_mismatch",
        ));
    }
    Ok(())
}
