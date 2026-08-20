use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    K2CompositionErrorV1, K2CompositionFileEntryV1, K2CompositionLearnedEffectV1,
    K2CompositionResultV1, K2CompositionTreeManifestV1, K2InquiryObservationModeV1,
    K2InquiryWorldModelV1, inquiry_generated_probe_provenance_root_v1,
    inquiry_observable_outcome_root_v1,
};
use super::{
    K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1, K2_UNCERTAINTY_ORACLE_FRONTIER_RECEIPT_SCHEMA_V1,
    K2_UNCERTAINTY_RAW_PROBES_V1, K2UncertaintyDomainVocabularyV1,
    K2UncertaintyEligibilityDispositionV1, K2UncertaintyFrontierPageV1, K2UncertaintyFrontierV1,
    K2UncertaintyModelSetV1, K2UncertaintyOracleFrontierReceiptV1, K2UncertaintyProbeClassV1,
    K2UncertaintyProbeEquivalenceKeyV1, K2UncertaintyRawProbeDispositionV1,
    K2UncertaintySafetyDispositionV1,
};

pub struct K2UncertaintyReconstructedFrontierV1 {
    pub receipt: K2UncertaintyOracleFrontierReceiptV1,
    pub representatives: Vec<K2UncertaintyRawProbeDispositionV1>,
}

pub fn reconstruct_self_formed_oracle_frontier_v1(
    vocabulary: &K2UncertaintyDomainVocabularyV1,
    split_commitment_root_sha256: &str,
    model_set: &K2UncertaintyModelSetV1,
    world_models: &[K2InquiryWorldModelV1],
    pages: &[K2UncertaintyFrontierPageV1],
    frozen_frontier: &K2UncertaintyFrontierV1,
) -> K2CompositionResultV1<K2UncertaintyReconstructedFrontierV1> {
    vocabulary.validate()?;
    model_set.validate()?;
    for model in world_models {
        model.validate()?;
    }
    frozen_frontier.validate()?;
    let expected_page_count =
        K2_UNCERTAINTY_RAW_PROBES_V1.div_ceil(K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1);
    if pages.len() != expected_page_count
        || frozen_frontier.case_id_sha256 != vocabulary.case_id_sha256
        || frozen_frontier.model_set_root_sha256 != model_set.model_set_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_frontier_binding_invalid",
        ));
    }

    let mut raw = Vec::with_capacity(K2_UNCERTAINTY_RAW_PROBES_V1);
    let mut page_roots = Vec::with_capacity(pages.len());
    for (page_sequence, page) in pages.iter().enumerate() {
        page.validate()?;
        if page.page_sequence != page_sequence as u64
            || page.case_id_sha256 != vocabulary.case_id_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_frontier_page_binding_invalid",
            ));
        }
        page_roots.push(page.page_root_sha256.clone());
        raw.extend(page.dispositions.iter().cloned());
    }
    page_roots.sort();
    if page_roots != frozen_frontier.page_roots_sha256 || raw.len() != K2_UNCERTAINTY_RAW_PROBES_V1
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_frontier_page_denominator_invalid",
        ));
    }

    let mut roots = BTreeSet::new();
    let mut quotient = BTreeMap::<K2UncertaintyProbeEquivalenceKeyV1, Vec<String>>::new();
    let mut by_probe = BTreeMap::new();
    for (sequence, disposition) in raw.into_iter().enumerate() {
        disposition.validate()?;
        if disposition.raw_sequence != sequence as u64
            || !roots.insert(disposition.probe.probe_root_sha256.clone())
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_raw_probe_denominator_invalid",
            ));
        }
        let reconstructed = reconstruct_disposition_key_v1(
            vocabulary,
            split_commitment_root_sha256,
            world_models,
            &disposition,
        )?;
        if reconstructed != disposition.equivalence_key {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_equivalence_key_mismatch",
            ));
        }
        quotient
            .entry(reconstructed)
            .or_default()
            .push(disposition.probe.probe_root_sha256.clone());
        by_probe.insert(disposition.probe.probe_root_sha256.clone(), disposition);
    }

    let mut classes = Vec::with_capacity(quotient.len());
    for (key, mut members) in quotient {
        members.sort();
        let representative_probe_root_sha256 =
            members
                .first()
                .cloned()
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_oracle_probe_class_empty",
                ))?;
        let mut class = K2UncertaintyProbeClassV1 {
            schema: super::K2_UNCERTAINTY_PROBE_CLASS_SCHEMA_V1.to_owned(),
            equivalence_key: key,
            member_probe_roots_sha256: members,
            representative_probe_root_sha256,
            class_root_sha256: String::new(),
        };
        class.reseal()?;
        classes.push(class);
    }
    classes.sort_by(|left, right| left.class_root_sha256.cmp(&right.class_root_sha256));
    let mut representative_probe_roots_sha256 = classes
        .iter()
        .map(|class| class.representative_probe_root_sha256.clone())
        .collect::<Vec<_>>();
    representative_probe_roots_sha256.sort();
    if classes != frozen_frontier.classes
        || representative_probe_roots_sha256 != frozen_frontier.representative_probe_roots_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_frontier_partition_mismatch",
        ));
    }

    let mut representatives = Vec::with_capacity(representative_probe_roots_sha256.len());
    for root in &representative_probe_roots_sha256 {
        representatives.push(by_probe.remove(root).ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_representative_missing",
        ))?);
    }
    let classes_root_sha256 = super::uncertainty_root_v1(&(
        "nando.k2-self-formed-oracle-reconstructed-classes.v1",
        &classes,
    ))?;
    let mut receipt = K2UncertaintyOracleFrontierReceiptV1 {
        schema: K2_UNCERTAINTY_ORACLE_FRONTIER_RECEIPT_SCHEMA_V1.to_owned(),
        case_id_sha256: vocabulary.case_id_sha256.clone(),
        raw_probe_count: K2_UNCERTAINTY_RAW_PROBES_V1 as u64,
        raw_member_count: roots.len() as u64,
        duplicate_member_count: 0,
        unclassified_member_count: 0,
        class_count: classes.len() as u64,
        classes_root_sha256,
        representative_probe_roots_sha256,
        receipt_root_sha256: String::new(),
    };
    receipt.reseal()?;
    Ok(K2UncertaintyReconstructedFrontierV1 {
        receipt,
        representatives,
    })
}

fn reconstruct_disposition_key_v1(
    vocabulary: &K2UncertaintyDomainVocabularyV1,
    split_commitment_root_sha256: &str,
    world_models: &[K2InquiryWorldModelV1],
    disposition: &K2UncertaintyRawProbeDispositionV1,
) -> K2CompositionResultV1<K2UncertaintyProbeEquivalenceKeyV1> {
    let probe = &disposition.probe;
    let provenance = inquiry_generated_probe_provenance_root_v1(
        &vocabulary.case_id_sha256,
        &vocabulary.generator_schema_root_sha256,
        split_commitment_root_sha256,
        &probe.probe_id_sha256,
        &probe.action_id_sha256,
    )?;
    let eligibility = if probe.generated_provenance_root_sha256 != provenance {
        K2UncertaintyEligibilityDispositionV1::NonGeneratedProvenance
    } else if !probe.reversible {
        K2UncertaintyEligibilityDispositionV1::NonReversible
    } else if probe.observation_mode != K2InquiryObservationModeV1::ExactImmediate {
        K2UncertaintyEligibilityDispositionV1::NonExactObservation
    } else if vocabulary
        .opaque_action_roots_sha256
        .binary_search(&probe.action_id_sha256)
        .is_err()
    {
        K2UncertaintyEligibilityDispositionV1::UnknownAction
    } else if disposition.robust_accounting.maximum_risk_units
        > vocabulary.budget.maximum_risk_units
    {
        K2UncertaintyEligibilityDispositionV1::RiskBudgetExceeded
    } else if disposition.robust_accounting.maximum_cost_units
        > vocabulary.budget.maximum_cost_units
    {
        K2UncertaintyEligibilityDispositionV1::CostBudgetExceeded
    } else {
        K2UncertaintyEligibilityDispositionV1::Eligible
    };
    disposition.robust_accounting.validate()?;
    let safety = if disposition.robust_accounting.maximum_risk_units
        > vocabulary.budget.maximum_risk_units
        || disposition.robust_accounting.maximum_cost_units > vocabulary.budget.maximum_cost_units
    {
        K2UncertaintySafetyDispositionV1::OverBudget
    } else {
        K2UncertaintySafetyDispositionV1::Pass
    };

    let mut outcome_roots = Vec::with_capacity(world_models.len());
    let mut all_applied = true;
    for model in world_models {
        let effect = model
            .effect(&probe.action_id_sha256)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_model_action_missing",
            ))?;
        let (applied, reason, post) = oracle_apply_effect_v1(&probe.initial_manifest, effect)?;
        all_applied &= applied;
        let outcome = inquiry_observable_outcome_root_v1(probe.observation_mode, &post)?;
        let observed = disposition
            .predictions
            .iter()
            .find(|prediction| prediction.model_root_sha256 == model.model_root_sha256)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_prediction_missing",
            ))?;
        if observed.transition_applied != applied
            || observed.transition_reason != reason
            || observed.predicted_post_manifest != post
            || observed.observable_outcome_root_sha256 != outcome
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_prediction_mismatch",
            ));
        }
        outcome_roots.push(outcome);
    }
    if outcome_roots.len() != 4 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_prediction_denominator_invalid",
        ));
    }
    let pairwise_outcome_equal = [
        outcome_roots[0] == outcome_roots[1],
        outcome_roots[0] == outcome_roots[2],
        outcome_roots[0] == outcome_roots[3],
        outcome_roots[1] == outcome_roots[2],
        outcome_roots[1] == outcome_roots[3],
        outcome_roots[2] == outcome_roots[3],
    ];
    let mut key = K2UncertaintyProbeEquivalenceKeyV1 {
        schema: super::K2_UNCERTAINTY_PROBE_EQUIVALENCE_KEY_SCHEMA_V1.to_owned(),
        pairwise_outcome_equal,
        eligibility,
        safety,
        risk_units: disposition.robust_accounting.maximum_risk_units,
        cost_units: disposition.robust_accounting.maximum_cost_units,
        applicability_hint: all_applied,
        dependency_hint: probe.initial_manifest.entries.len() >= 2,
        cleanup_hint: disposition.robust_accounting.maximum_risk_units <= 1,
        key_root_sha256: String::new(),
    };
    key.reseal()?;
    Ok(key)
}

pub(crate) fn oracle_apply_effect_v1(
    manifest: &K2CompositionTreeManifestV1,
    effect: &K2CompositionLearnedEffectV1,
) -> K2CompositionResultV1<(bool, String, K2CompositionTreeManifestV1)> {
    manifest.validate()?;
    effect.validate()?;
    let mut entries = manifest
        .entries
        .iter()
        .cloned()
        .map(|entry| (entry.path.clone(), entry))
        .collect::<BTreeMap<_, K2CompositionFileEntryV1>>();
    let (applied, reason) = match effect {
        K2CompositionLearnedEffectV1::CopyFile {
            source_path,
            target_path,
        } => match entries.get(source_path).cloned() {
            Some(mut source) => {
                source.path.clone_from(target_path);
                entries.insert(target_path.clone(), source);
                (true, "applied".to_owned())
            }
            None => (false, "copy_source_missing".to_owned()),
        },
        K2CompositionLearnedEffectV1::RemoveFile { path } => {
            if entries.remove(path).is_some() {
                (true, "applied".to_owned())
            } else {
                (false, "remove_path_missing".to_owned())
            }
        }
    };
    Ok((
        applied,
        reason,
        K2CompositionTreeManifestV1::seal_entries(entries.into_values().collect())?,
    ))
}
