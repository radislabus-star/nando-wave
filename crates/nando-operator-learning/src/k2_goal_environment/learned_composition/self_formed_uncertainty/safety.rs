use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionLearnedEffectV1,
    K2CompositionResultV1, K2InquiryObservationModeV1, K2InquiryProbeV1,
    composition_sha256_file_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_MAX_COST_UNITS_V1, K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1,
    K2_UNCERTAINTY_MAX_RISK_UNITS_V1, K2_UNCERTAINTY_RISK_COST_SCHEMA_V1,
    K2_UNCERTAINTY_SAFETY_RECEIPT_SCHEMA_V1, K2_UNCERTAINTY_SAFETY_REQUEST_SCHEMA_V1,
    K2UncertaintyDomainVocabularyV1, K2UncertaintyEffectCandidateV1, K2UncertaintyRiskCostV1,
    denied_authority_v1, require_denied_authority_v1, uncertainty_bytes_v1, uncertainty_decode_v1,
    uncertainty_root_v1,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyPrivateSafetyDispositionV1 {
    Pass,
    GrammarVeto,
    ConfinementVeto,
    ManifestVeto,
    ObservationVeto,
    ReversibilityVeto,
    RiskVeto,
    CostVeto,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintySafetyRequestV1 {
    pub schema: String,
    pub selection_root_sha256: String,
    pub selected_probe: K2InquiryProbeV1,
    pub resolved_private_effect: K2CompositionLearnedEffectV1,
    pub vocabulary: K2UncertaintyDomainVocabularyV1,
    pub grammar_root_sha256: String,
    pub sandbox_root_sha256: String,
    pub safety_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintySafetyRequestV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        selection_root_sha256: String,
        selected_probe: K2InquiryProbeV1,
        resolved_private_effect: K2CompositionLearnedEffectV1,
        vocabulary: K2UncertaintyDomainVocabularyV1,
        grammar_root_sha256: String,
        sandbox_root_sha256: String,
        safety_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let authority = denied_authority_v1();
        let mut request = Self {
            schema: K2_UNCERTAINTY_SAFETY_REQUEST_SCHEMA_V1.to_owned(),
            selection_root_sha256,
            selected_probe,
            resolved_private_effect,
            vocabulary,
            grammar_root_sha256,
            sandbox_root_sha256,
            safety_executable_sha256,
            authority,
            request_root_sha256: String::new(),
        };
        request.request_root_sha256 = request.expected_root()?;
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.selection_root_sha256,
            &self.grammar_root_sha256,
            &self.sandbox_root_sha256,
            &self.safety_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.selected_probe.validate()?;
        self.resolved_private_effect.validate()?;
        self.vocabulary.validate()?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_SAFETY_REQUEST_SCHEMA_V1
            || self.selected_probe.experiment_id_sha256 != self.vocabulary.case_id_sha256
            || !self
                .vocabulary
                .opaque_action_roots_sha256
                .contains(&self.selected_probe.action_id_sha256)
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_safety_request_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_SAFETY_REQUEST_SCHEMA_V1,
            &self.selection_root_sha256,
            &self.selected_probe,
            &self.resolved_private_effect,
            &self.vocabulary,
            &self.grammar_root_sha256,
            &self.sandbox_root_sha256,
            &self.safety_executable_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintySafetyReceiptV1 {
    pub schema: String,
    pub safety_request_root_sha256: String,
    pub disposition: K2UncertaintyPrivateSafetyDispositionV1,
    pub selected_effect_accounting: Option<K2UncertaintyRiskCostV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintySafetyReceiptV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.safety_request_root_sha256)?;
        if let Some(accounting) = &self.selected_effect_accounting {
            accounting.validate()?;
        }
        require_denied_authority_v1(&self.authority)?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_SAFETY_RECEIPT_SCHEMA_V1,
            &self.safety_request_root_sha256,
            self.disposition,
            &self.selected_effect_accounting,
            &self.authority,
        ))?;
        if self.schema != K2_UNCERTAINTY_SAFETY_RECEIPT_SCHEMA_V1
            || (self.disposition == K2UncertaintyPrivateSafetyDispositionV1::Pass
                && self.selected_effect_accounting.is_none())
            || self.receipt_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_safety_receipt_invalid",
            ));
        }
        Ok(())
    }
}

pub fn verify_self_formed_private_safety_v1(
    request: &K2UncertaintySafetyRequestV1,
) -> K2CompositionResultV1<K2UncertaintySafetyReceiptV1> {
    request.validate()?;
    let effects = safety_enumerate_effects_v1(&request.vocabulary)?;
    let effect_roots = effects
        .iter()
        .map(|effect| K2UncertaintyEffectCandidateV1::seal(effect.clone()))
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    let grammar_root =
        uncertainty_root_v1(&("nando.k2-self-formed-effect-grammar.v1", &effect_roots))?;
    let allowed_paths = request
        .vocabulary
        .path_atoms
        .iter()
        .map(|atom| atom.path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let read_paths = request.resolved_private_effect.read_paths();
    let write_paths = request.resolved_private_effect.write_paths();
    let disposition = if request.grammar_root_sha256 != grammar_root
        || !effects.contains(&request.resolved_private_effect)
    {
        K2UncertaintyPrivateSafetyDispositionV1::GrammarVeto
    } else if read_paths
        .iter()
        .chain(write_paths.iter())
        .any(|path| !allowed_paths.contains(path.as_str()))
    {
        K2UncertaintyPrivateSafetyDispositionV1::ConfinementVeto
    } else if request
        .selected_probe
        .initial_manifest
        .entries
        .iter()
        .any(|entry| {
            !allowed_paths.contains(entry.path.as_str())
                || request
                    .vocabulary
                    .content_by_sha256(&entry.content_sha256)
                    .is_none_or(|content| content.byte_len != entry.byte_len)
        })
    {
        K2UncertaintyPrivateSafetyDispositionV1::ManifestVeto
    } else if request.selected_probe.observation_mode != K2InquiryObservationModeV1::ExactImmediate
    {
        K2UncertaintyPrivateSafetyDispositionV1::ObservationVeto
    } else if !request.selected_probe.reversible {
        K2UncertaintyPrivateSafetyDispositionV1::ReversibilityVeto
    } else if request.selected_probe.risk_units > K2_UNCERTAINTY_MAX_RISK_UNITS_V1 {
        K2UncertaintyPrivateSafetyDispositionV1::RiskVeto
    } else if request.selected_probe.cost_units > K2_UNCERTAINTY_MAX_COST_UNITS_V1 {
        K2UncertaintyPrivateSafetyDispositionV1::CostVeto
    } else {
        K2UncertaintyPrivateSafetyDispositionV1::Pass
    };
    let selected_effect_accounting = if matches!(
        disposition,
        K2UncertaintyPrivateSafetyDispositionV1::Pass
            | K2UncertaintyPrivateSafetyDispositionV1::RiskVeto
            | K2UncertaintyPrivateSafetyDispositionV1::CostVeto
    ) {
        Some(safety_accounting_v1(
            &request.selected_probe,
            &request.resolved_private_effect,
        )?)
    } else {
        None
    };
    let authority = denied_authority_v1();
    let receipt_root_sha256 = uncertainty_root_v1(&(
        K2_UNCERTAINTY_SAFETY_RECEIPT_SCHEMA_V1,
        &request.request_root_sha256,
        disposition,
        &selected_effect_accounting,
        &authority,
    ))?;
    let receipt = K2UncertaintySafetyReceiptV1 {
        schema: K2_UNCERTAINTY_SAFETY_RECEIPT_SCHEMA_V1.to_owned(),
        safety_request_root_sha256: request.request_root_sha256.clone(),
        disposition,
        selected_effect_accounting,
        authority,
        receipt_root_sha256,
    };
    receipt.validate()?;
    Ok(receipt)
}

pub fn run_self_formed_safety_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_safety_stdin"))?;
    let request: K2UncertaintySafetyRequestV1 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_safety"))?;
    if composition_sha256_file_v1(&executable)? != request.safety_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_safety_executable_mismatch",
        ));
    }
    let receipt = verify_self_formed_private_safety_v1(&request)?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_safety_stdout"))
}

pub fn self_formed_grammar_root_v1(
    vocabulary: &K2UncertaintyDomainVocabularyV1,
) -> K2CompositionResultV1<String> {
    let candidates = safety_enumerate_effects_v1(vocabulary)?
        .into_iter()
        .map(K2UncertaintyEffectCandidateV1::seal)
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    uncertainty_root_v1(&("nando.k2-self-formed-effect-grammar.v1", candidates))
}

fn safety_enumerate_effects_v1(
    vocabulary: &K2UncertaintyDomainVocabularyV1,
) -> K2CompositionResultV1<Vec<K2CompositionLearnedEffectV1>> {
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
    Ok(effects)
}

fn safety_accounting_v1(
    probe: &K2InquiryProbeV1,
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
            if let Some(source) = probe.initial_manifest.entry(source_path) {
                accounting.written_or_removed_entries = 1;
                accounting.touched_bytes = source.byte_len;
                if let Some(target) = probe.initial_manifest.entry(target_path) {
                    accounting.overwritten_existing_entries = 1;
                    accounting.overwritten_bytes = target.byte_len;
                    accounting.touched_bytes = accounting
                        .touched_bytes
                        .checked_add(target.byte_len)
                        .ok_or(K2CompositionErrorV1::Invalid(
                            "self_formed_safety_accounting_overflow",
                        ))?;
                }
            }
        }
        K2CompositionLearnedEffectV1::RemoveFile { path } => {
            if let Some(existing) = probe.initial_manifest.entry(path) {
                accounting.written_or_removed_entries = 1;
                accounting.removed_existing_entries = 1;
                accounting.removed_bytes = existing.byte_len;
                accounting.touched_bytes = existing.byte_len;
            }
        }
    }
    accounting.reseal()?;
    if accounting.risk_units > probe.risk_units || accounting.cost_units > probe.cost_units {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_safety_robust_bound_invalid",
        ));
    }
    Ok(accounting)
}
