use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionFileEntryV1,
    K2CompositionLearnedEffectV1, K2CompositionResultV1, K2CompositionTreeManifestV1,
    K2InquiryObservationModeV1, K2InquiryProbeV1, composition_sha256_file_v1,
    inquiry_generated_probe_provenance_root_v1, inquiry_observable_outcome_root_v1,
    require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_EFFECT_ACCOUNTING_SCHEMA_V1, K2_UNCERTAINTY_EFFECTS_PER_ACTION_V1,
    K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1, K2_UNCERTAINTY_FRONTIER_PAGE_SCHEMA_V1,
    K2_UNCERTAINTY_FRONTIER_SCHEMA_V1, K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1,
    K2_UNCERTAINTY_PREDICTION_WITNESS_SCHEMA_V1, K2_UNCERTAINTY_PROBE_CLASS_SCHEMA_V1,
    K2_UNCERTAINTY_PROBE_OUTPUT_SCHEMA_V1, K2_UNCERTAINTY_PROBE_REQUEST_SCHEMA_V1,
    K2_UNCERTAINTY_RAW_PREDICTIONS_V1, K2_UNCERTAINTY_RAW_PROBE_SCHEMA_V1,
    K2_UNCERTAINTY_RAW_PROBES_V1, K2_UNCERTAINTY_RISK_COST_SCHEMA_V1,
    K2_UNCERTAINTY_ROBUST_ACCOUNTING_SCHEMA_V1, K2UncertaintyDomainVocabularyV1,
    K2UncertaintyEffectAccountingV1, K2UncertaintyEffectCandidateV1,
    K2UncertaintyEligibilityDispositionV1, K2UncertaintyFrontierPageV1, K2UncertaintyFrontierV1,
    K2UncertaintyLearnerResponseV1, K2UncertaintyPredictionWitnessV1, K2UncertaintyProbeClassV1,
    K2UncertaintyProbeEquivalenceKeyV1, K2UncertaintyPublicCaseV1,
    K2UncertaintyRawProbeDispositionV1, K2UncertaintyRiskCostV1, K2UncertaintyRobustAccountingV1,
    K2UncertaintySafetyDispositionV1, K2UncertaintyStateUniverseV1, denied_authority_v1,
    publish_self_formed_probe_output_v1, require_denied_authority_v1, require_exact_len_v1,
    uncertainty_bytes_v1, uncertainty_decode_v1, uncertainty_root_v1,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyProbeRequestV1 {
    pub schema: String,
    pub public_case: K2UncertaintyPublicCaseV1,
    pub learner_response: K2UncertaintyLearnerResponseV1,
    pub split_commitment_root_sha256: String,
    pub probe_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintyProbeRequestV1 {
    pub fn seal(
        public_case: K2UncertaintyPublicCaseV1,
        learner_response: K2UncertaintyLearnerResponseV1,
        split_commitment_root_sha256: String,
        probe_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let authority = denied_authority_v1();
        let mut request = Self {
            schema: K2_UNCERTAINTY_PROBE_REQUEST_SCHEMA_V1.to_owned(),
            public_case,
            learner_response,
            split_commitment_root_sha256,
            probe_executable_sha256,
            authority,
            request_root_sha256: String::new(),
        };
        request.request_root_sha256 = request.expected_root()?;
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.public_case.validate()?;
        self.learner_response.validate()?;
        require_composition_root_v1(&self.split_commitment_root_sha256)?;
        require_composition_root_v1(&self.probe_executable_sha256)?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_PROBE_REQUEST_SCHEMA_V1
            || self.public_case.vocabulary.case_id_sha256
                != self.learner_response.model_set.case_id_sha256
            || self.public_case.vocabulary.vocabulary_root_sha256
                != self.learner_response.model_set.vocabulary_root_sha256
            || self.public_case.support.support_root_sha256
                != self.learner_response.model_set.support_root_sha256
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_probe_request_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_PROBE_REQUEST_SCHEMA_V1,
            &self.public_case,
            &self.learner_response,
            &self.split_commitment_root_sha256,
            &self.probe_executable_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyProbeOutputV1 {
    pub schema: String,
    pub probe_request_root_sha256: String,
    pub state_universe: K2UncertaintyStateUniverseV1,
    pub pages: Vec<K2UncertaintyFrontierPageV1>,
    pub frontier: K2UncertaintyFrontierV1,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub output_root_sha256: String,
}

impl K2UncertaintyProbeOutputV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.probe_request_root_sha256)?;
        self.state_universe.validate()?;
        self.frontier.validate()?;
        let expected_pages =
            K2_UNCERTAINTY_RAW_PROBES_V1.div_ceil(K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1);
        require_exact_len_v1(
            self.pages.len(),
            expected_pages,
            "self_formed_probe_output_page_count_invalid",
        )?;
        let mut page_roots = Vec::with_capacity(self.pages.len());
        let mut probe_roots = BTreeSet::new();
        for (sequence, page) in self.pages.iter().enumerate() {
            page.validate()?;
            if page.page_sequence != sequence as u64
                || page.case_id_sha256 != self.frontier.case_id_sha256
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_probe_output_page_binding_invalid",
                ));
            }
            page_roots.push(page.page_root_sha256.clone());
            for disposition in &page.dispositions {
                if !probe_roots.insert(disposition.probe.probe_root_sha256.as_str()) {
                    return Err(K2CompositionErrorV1::Invalid(
                        "self_formed_probe_output_duplicate_probe",
                    ));
                }
            }
        }
        page_roots.sort();
        let class_members = self
            .frontier
            .classes
            .iter()
            .flat_map(|class| class.member_probe_roots_sha256.iter())
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        require_denied_authority_v1(&self.authority)?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_PROBE_OUTPUT_SCHEMA_V1,
            &self.probe_request_root_sha256,
            &self.state_universe,
            self.pages
                .iter()
                .map(|page| &page.page_root_sha256)
                .collect::<Vec<_>>(),
            &self.frontier,
            &self.authority,
        ))?;
        if self.schema != K2_UNCERTAINTY_PROBE_OUTPUT_SCHEMA_V1
            || self.frontier.state_universe_root_sha256 != self.state_universe.universe_root_sha256
            || self.frontier.page_roots_sha256 != page_roots
            || probe_roots.len() != K2_UNCERTAINTY_RAW_PROBES_V1
            || probe_roots != class_members
            || self.output_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_probe_output_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.authority = denied_authority_v1();
        self.output_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_PROBE_OUTPUT_SCHEMA_V1,
            &self.probe_request_root_sha256,
            &self.state_universe,
            self.pages
                .iter()
                .map(|page| &page.page_root_sha256)
                .collect::<Vec<_>>(),
            &self.frontier,
            &self.authority,
        ))?;
        self.validate()
    }
}

pub fn enumerate_self_formed_probe_frontier_v1(
    request: &K2UncertaintyProbeRequestV1,
) -> K2CompositionResultV1<K2UncertaintyProbeOutputV1> {
    request.validate()?;
    request
        .learner_response
        .model_set
        .require_confirm_cardinality()?;
    let states = probe_enumerate_states_v1(&request.public_case.vocabulary)?;
    let state_universe = K2UncertaintyStateUniverseV1::seal(
        request
            .public_case
            .vocabulary
            .vocabulary_root_sha256
            .clone(),
        states.clone(),
    )?;
    let effects = probe_enumerate_effects_v1(&request.public_case.vocabulary)?;
    let mut dispositions = Vec::with_capacity(K2_UNCERTAINTY_RAW_PROBES_V1);
    for (action_index, action_root) in request
        .public_case
        .vocabulary
        .opaque_action_roots_sha256
        .iter()
        .enumerate()
    {
        for (state_index, state) in states.iter().enumerate() {
            let raw_sequence = (action_index * states.len() + state_index) as u64;
            dispositions.push(build_probe_disposition_v1(
                request,
                &effects,
                state,
                action_root,
                raw_sequence,
            )?);
        }
    }
    require_exact_len_v1(
        dispositions.len(),
        K2_UNCERTAINTY_RAW_PROBES_V1,
        "self_formed_probe_raw_denominator_invalid",
    )?;

    let mut quotient = BTreeMap::<K2UncertaintyProbeEquivalenceKeyV1, Vec<String>>::new();
    for disposition in &dispositions {
        quotient
            .entry(disposition.equivalence_key.clone())
            .or_default()
            .push(disposition.probe.probe_root_sha256.clone());
    }
    let mut classes = Vec::with_capacity(quotient.len());
    for (key, mut members) in quotient {
        members.sort();
        let representative = members
            .first()
            .cloned()
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_probe_class_empty",
            ))?;
        let mut class = K2UncertaintyProbeClassV1 {
            schema: K2_UNCERTAINTY_PROBE_CLASS_SCHEMA_V1.to_owned(),
            equivalence_key: key,
            member_probe_roots_sha256: members,
            representative_probe_root_sha256: representative,
            class_root_sha256: String::new(),
        };
        class.reseal()?;
        classes.push(class);
    }
    classes.sort_by(|left, right| left.class_root_sha256.cmp(&right.class_root_sha256));

    let mut pages = Vec::new();
    for (page_sequence, chunk) in dispositions
        .chunks(K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1)
        .enumerate()
    {
        let mut page = K2UncertaintyFrontierPageV1 {
            schema: K2_UNCERTAINTY_FRONTIER_PAGE_SCHEMA_V1.to_owned(),
            case_id_sha256: request.public_case.vocabulary.case_id_sha256.clone(),
            page_sequence: page_sequence as u64,
            dispositions: chunk.to_vec(),
            page_root_sha256: String::new(),
        };
        page.reseal()?;
        pages.push(page);
    }
    let mut page_roots = pages
        .iter()
        .map(|page| page.page_root_sha256.clone())
        .collect::<Vec<_>>();
    page_roots.sort();
    let mut all_probe_roots = dispositions
        .iter()
        .map(|disposition| disposition.probe.probe_root_sha256.clone())
        .collect::<Vec<_>>();
    all_probe_roots.sort();
    let raw_probe_denominator_root_sha256 = uncertainty_root_v1(&(
        "nando.k2-self-formed-raw-probe-denominator.v1",
        &all_probe_roots,
    ))?;
    let mut representative_probe_roots_sha256 = classes
        .iter()
        .map(|class| class.representative_probe_root_sha256.clone())
        .collect::<Vec<_>>();
    representative_probe_roots_sha256.sort();
    let mut frontier = K2UncertaintyFrontierV1 {
        schema: K2_UNCERTAINTY_FRONTIER_SCHEMA_V1.to_owned(),
        case_id_sha256: request.public_case.vocabulary.case_id_sha256.clone(),
        model_set_root_sha256: request
            .learner_response
            .model_set
            .model_set_root_sha256
            .clone(),
        state_universe_root_sha256: state_universe.universe_root_sha256.clone(),
        raw_probe_count: K2_UNCERTAINTY_RAW_PROBES_V1 as u64,
        raw_prediction_count: K2_UNCERTAINTY_RAW_PREDICTIONS_V1 as u64,
        page_roots_sha256: page_roots,
        raw_probe_denominator_root_sha256,
        classes,
        representative_probe_roots_sha256,
        authority: denied_authority_v1(),
        frontier_root_sha256: String::new(),
    };
    frontier.reseal()?;
    let mut output = K2UncertaintyProbeOutputV1 {
        schema: K2_UNCERTAINTY_PROBE_OUTPUT_SCHEMA_V1.to_owned(),
        probe_request_root_sha256: request.request_root_sha256.clone(),
        state_universe,
        pages,
        frontier,
        authority: denied_authority_v1(),
        output_root_sha256: String::new(),
    };
    output.reseal()?;
    Ok(output)
}

pub fn run_self_formed_probe_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_probe_stdin"))?;
    let request: K2UncertaintyProbeRequestV1 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_probe"))?;
    if composition_sha256_file_v1(&executable)? != request.probe_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_probe_executable_mismatch",
        ));
    }
    let output = enumerate_self_formed_probe_frontier_v1(&request)?;
    let receipt = publish_self_formed_probe_output_v1(Path::new("/out"), &output)?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_probe_stdout"))
}

fn build_probe_disposition_v1(
    request: &K2UncertaintyProbeRequestV1,
    effects: &[K2CompositionLearnedEffectV1],
    state: &K2CompositionTreeManifestV1,
    action_root: &str,
    raw_sequence: u64,
) -> K2CompositionResultV1<K2UncertaintyRawProbeDispositionV1> {
    let probe_id_sha256 = uncertainty_root_v1(&(
        "nando.k2-self-formed-mechanical-probe.v1",
        &request.public_case.vocabulary.case_id_sha256,
        raw_sequence,
        &state.tree_root_sha256,
        action_root,
    ))?;
    let robust_accounting = robust_accounting_v1(&request.public_case.vocabulary, state, effects)?;
    let mut raw_predictions = Vec::with_capacity(request.learner_response.world_models.len());
    for model in &request.learner_response.world_models {
        let action = model
            .effect(action_root)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_probe_model_action_missing",
            ))?;
        raw_predictions.push((
            model.model_root_sha256.clone(),
            probe_apply_effect_v1(state, action)?,
        ));
    }
    raw_predictions.sort_by(|left, right| left.0.cmp(&right.0));
    let applicability_hint = raw_predictions.iter().all(|(_, prediction)| prediction.0);
    let dependency_hint = state.entries.len() >= 2;
    let cleanup_hint = robust_accounting.maximum_risk_units <= 1;
    let provenance = inquiry_generated_probe_provenance_root_v1(
        &request.public_case.vocabulary.case_id_sha256,
        &request.public_case.vocabulary.generator_schema_root_sha256,
        &request.split_commitment_root_sha256,
        &probe_id_sha256,
        action_root,
    )?;
    let probe = K2InquiryProbeV1::seal(
        request.public_case.vocabulary.case_id_sha256.clone(),
        probe_id_sha256,
        action_root.to_owned(),
        state.clone(),
        true,
        K2InquiryObservationModeV1::ExactImmediate,
        robust_accounting.maximum_risk_units,
        robust_accounting.maximum_cost_units,
        applicability_hint,
        dependency_hint,
        cleanup_hint,
        provenance,
    )?;
    let mut predictions = Vec::with_capacity(raw_predictions.len());
    for (model_root_sha256, (applied, reason, post)) in raw_predictions {
        let observable_outcome_root_sha256 =
            inquiry_observable_outcome_root_v1(K2InquiryObservationModeV1::ExactImmediate, &post)?;
        let mut witness = K2UncertaintyPredictionWitnessV1 {
            schema: K2_UNCERTAINTY_PREDICTION_WITNESS_SCHEMA_V1.to_owned(),
            model_root_sha256,
            probe_root_sha256: probe.probe_root_sha256.clone(),
            transition_applied: applied,
            transition_reason: reason,
            predicted_post_manifest: post,
            observable_outcome_root_sha256,
            prediction_root_sha256: String::new(),
        };
        witness.reseal()?;
        predictions.push(witness);
    }
    let pairwise = prediction_equality_matrix_v1(&predictions)?;
    let mut key = K2UncertaintyProbeEquivalenceKeyV1 {
        schema: super::K2_UNCERTAINTY_PROBE_EQUIVALENCE_KEY_SCHEMA_V1.to_owned(),
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
    key.reseal()?;
    let mut disposition = K2UncertaintyRawProbeDispositionV1 {
        schema: K2_UNCERTAINTY_RAW_PROBE_SCHEMA_V1.to_owned(),
        raw_sequence,
        probe,
        predictions,
        robust_accounting,
        eligibility: K2UncertaintyEligibilityDispositionV1::Eligible,
        safety: K2UncertaintySafetyDispositionV1::Pass,
        equivalence_key: key,
        raw_probe_root_sha256: String::new(),
    };
    disposition.reseal()?;
    Ok(disposition)
}

fn prediction_equality_matrix_v1(
    predictions: &[K2UncertaintyPredictionWitnessV1],
) -> K2CompositionResultV1<[bool; 6]> {
    require_exact_len_v1(
        predictions.len(),
        super::K2_UNCERTAINTY_CONFIRM_MODELS_V1,
        "self_formed_prediction_matrix_count_invalid",
    )?;
    let roots = predictions
        .iter()
        .map(|prediction| prediction.observable_outcome_root_sha256.as_str())
        .collect::<Vec<_>>();
    Ok([
        roots[0] == roots[1],
        roots[0] == roots[2],
        roots[0] == roots[3],
        roots[1] == roots[2],
        roots[1] == roots[3],
        roots[2] == roots[3],
    ])
}

fn robust_accounting_v1(
    vocabulary: &K2UncertaintyDomainVocabularyV1,
    state: &K2CompositionTreeManifestV1,
    effects: &[K2CompositionLearnedEffectV1],
) -> K2CompositionResultV1<K2UncertaintyRobustAccountingV1> {
    let mut receipts = Vec::with_capacity(effects.len());
    for effect in effects {
        let candidate = K2UncertaintyEffectCandidateV1::seal(effect.clone())?;
        let mut receipt = K2UncertaintyEffectAccountingV1 {
            schema: K2_UNCERTAINTY_EFFECT_ACCOUNTING_SCHEMA_V1.to_owned(),
            effect_root_sha256: candidate.effect_root_sha256,
            accounting: effect_accounting_v1(vocabulary, state, effect)?,
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

fn effect_accounting_v1(
    vocabulary: &K2UncertaintyDomainVocabularyV1,
    state: &K2CompositionTreeManifestV1,
    effect: &K2CompositionLearnedEffectV1,
) -> K2CompositionResultV1<K2UncertaintyRiskCostV1> {
    let mut accounting = K2UncertaintyRiskCostV1 {
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
                require_content_v1(vocabulary, &source.content_sha256, source.byte_len)?;
                accounting.written_or_removed_entries = 1;
                accounting.touched_bytes = source.byte_len;
                if let Some(target) = state.entry(target_path) {
                    require_content_v1(vocabulary, &target.content_sha256, target.byte_len)?;
                    accounting.overwritten_existing_entries = 1;
                    accounting.overwritten_bytes = target.byte_len;
                    accounting.touched_bytes = accounting
                        .touched_bytes
                        .checked_add(target.byte_len)
                        .ok_or(K2CompositionErrorV1::Invalid(
                            "self_formed_accounting_bytes_overflow",
                        ))?;
                }
            }
        }
        K2CompositionLearnedEffectV1::RemoveFile { path } => {
            if let Some(existing) = state.entry(path) {
                require_content_v1(vocabulary, &existing.content_sha256, existing.byte_len)?;
                accounting.written_or_removed_entries = 1;
                accounting.removed_existing_entries = 1;
                accounting.removed_bytes = existing.byte_len;
                accounting.touched_bytes = existing.byte_len;
            }
        }
    }
    accounting.reseal()?;
    Ok(accounting)
}

fn require_content_v1(
    vocabulary: &K2UncertaintyDomainVocabularyV1,
    sha256: &str,
    byte_len: u64,
) -> K2CompositionResultV1<()> {
    let content = vocabulary
        .content_by_sha256(sha256)
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_accounting_content_missing",
        ))?;
    if content.byte_len != byte_len {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_accounting_content_length_mismatch",
        ));
    }
    Ok(())
}

fn probe_enumerate_states_v1(
    vocabulary: &K2UncertaintyDomainVocabularyV1,
) -> K2CompositionResultV1<Vec<K2CompositionTreeManifestV1>> {
    vocabulary.validate()?;
    let mut manifests = Vec::with_capacity(super::K2_UNCERTAINTY_STATE_COUNT_V1);
    for encoded in 0..super::K2_UNCERTAINTY_STATE_COUNT_V1 {
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

fn probe_enumerate_effects_v1(
    vocabulary: &K2UncertaintyDomainVocabularyV1,
) -> K2CompositionResultV1<Vec<K2CompositionLearnedEffectV1>> {
    vocabulary.validate()?;
    let mut effects = Vec::with_capacity(K2_UNCERTAINTY_EFFECTS_PER_ACTION_V1);
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
    require_exact_len_v1(
        effects.len(),
        K2_UNCERTAINTY_EFFECTS_PER_ACTION_V1,
        "self_formed_probe_effect_count_invalid",
    )?;
    Ok(effects)
}

fn probe_apply_effect_v1(
    manifest: &K2CompositionTreeManifestV1,
    effect: &K2CompositionLearnedEffectV1,
) -> K2CompositionResultV1<(bool, String, K2CompositionTreeManifestV1)> {
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
