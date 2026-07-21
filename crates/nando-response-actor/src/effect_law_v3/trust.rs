use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::{canonical_json_bytes, sha256_bytes, valid_nonzero_sha256};

pub(super) fn resolve_trusted_effect_evidence_set(
    manifest_bytes: &[u8],
    expected_manifest_root: &TrustedGenerationManifestRootV3,
) -> Result<TrustedEffectEvidenceSetV3, EffectLawV3Error> {
    let expected_manifest_root_sha256 = expected_manifest_root.0.as_str();
    if !valid_nonzero_sha256(expected_manifest_root_sha256)
        || sha256_bytes(manifest_bytes) != expected_manifest_root_sha256
    {
        return Err(EffectLawV3Error::InvalidTrustRoot);
    }
    let manifest: TrustedGenerationManifestWireV3 =
        serde_json::from_slice(manifest_bytes).map_err(|_| EffectLawV3Error::InvalidTrustRoot)?;
    if manifest.schema != TRUSTED_GENERATION_MANIFEST_SCHEMA_V3
        || canonical_json_bytes(&manifest).map_err(|_| EffectLawV3Error::Serialization)?
            != manifest_bytes
        || !valid_nonzero_sha256(&manifest.generation_id_sha256)
        || !valid_nonzero_sha256(&manifest.delta_verifier_root_sha256)
        || manifest.entries.is_empty()
        || manifest.entries.len() > MAX_OBSERVATIONS_V3
        || manifest.parity_receipts.len() != manifest.entries.len()
        || manifest.observed_states.len() != manifest.entries.len()
    {
        return Err(EffectLawV3Error::InvalidTrustRoot);
    }
    manifest
        .capture_index
        .validate()
        .map_err(|_| EffectLawV3Error::InvalidCaptureReceipt)?;
    if !strictly_ordered_by(&manifest.entries, |entry| {
        entry.evidence_ref_sha256.as_str()
    }) || !strictly_ordered_by(&manifest.parity_receipts, |receipt| {
        receipt.evidence_ref_sha256.as_str()
    }) || !strictly_ordered_by(&manifest.observed_states, |state| {
        state.evidence_ref_sha256.as_str()
    }) {
        return Err(EffectLawV3Error::InvalidTrustRoot);
    }

    let mut parity_by_evidence = BTreeMap::new();
    for receipt in manifest.parity_receipts {
        receipt
            .validate_sealed()
            .map_err(|_| EffectLawV3Error::InvalidParityReceipt)?;
        if parity_by_evidence
            .insert(receipt.evidence_ref_sha256.clone(), receipt)
            .is_some()
        {
            return Err(EffectLawV3Error::InvalidParityReceipt);
        }
    }
    let mut observed_state_by_evidence = BTreeMap::new();
    for state in manifest.observed_states {
        validate_independent_effect_state(&state)?;
        if observed_state_by_evidence
            .insert(state.evidence_ref_sha256.clone(), state)
            .is_some()
        {
            return Err(EffectLawV3Error::InvalidVerifierReceipt);
        }
    }
    let mut entry_by_evidence = BTreeMap::new();
    for entry in manifest.entries {
        validate_manifest_entry(&entry)?;
        let parity = parity_by_evidence
            .get(&entry.evidence_ref_sha256)
            .ok_or(EffectLawV3Error::InvalidTrustRoot)?;
        let observed = observed_state_by_evidence
            .get(&entry.evidence_ref_sha256)
            .ok_or(EffectLawV3Error::InvalidTrustRoot)?;
        if entry.parity_receipt_root_sha256 != parity.receipt_sha256
            || entry.physical_program_id != parity.program_sha256
            || entry.observed_state_root_sha256 != observed.receipt_sha256
        {
            return Err(EffectLawV3Error::InvalidTrustRoot);
        }
        if entry_by_evidence
            .insert(entry.evidence_ref_sha256.clone(), entry)
            .is_some()
        {
            return Err(EffectLawV3Error::InvalidTrustRoot);
        }
    }
    let delta_verifier_material = entry_by_evidence
        .keys()
        .map(|evidence_ref| {
            let parity = parity_by_evidence
                .get(evidence_ref)
                .expect("manifest parity was joined");
            let state = observed_state_by_evidence
                .get(evidence_ref)
                .expect("manifest state was joined");
            (
                evidence_ref.as_str(),
                parity.verifier_sha256.as_str(),
                state.observer_root_sha256.as_str(),
            )
        })
        .collect::<Vec<_>>();
    let expected_delta_verifier_root = evidence::sha256_serialized(&(
        VERIFIED_EFFECT_DELTA_RECEIPT_SCHEMA_V3,
        &delta_verifier_material,
    ))?;
    if expected_delta_verifier_root != manifest.delta_verifier_root_sha256 {
        return Err(EffectLawV3Error::InvalidTrustRoot);
    }
    let resolver_root_sha256 = evidence::sha256_serialized(&(
        TRUSTED_EFFECT_EVIDENCE_SET_SCHEMA_V3,
        manifest.generation_id_sha256.as_str(),
        expected_manifest_root_sha256,
        manifest.capture_index.index_sha256.as_str(),
        &delta_verifier_material,
    ))?;
    Ok(TrustedEffectEvidenceSetV3 {
        schema: TRUSTED_EFFECT_EVIDENCE_SET_SCHEMA_V3.to_owned(),
        generation_id_sha256: manifest.generation_id_sha256,
        trust_manifest_root_sha256: expected_manifest_root_sha256.to_owned(),
        delta_verifier_root_sha256: manifest.delta_verifier_root_sha256,
        resolver_root_sha256,
        capture_index: manifest.capture_index,
        parity_by_evidence,
        observed_state_by_evidence,
        entry_by_evidence,
    })
}

// Only the generation owner inside this crate may turn an externally stored
// commitment into a trust capability. Evidence producers never receive this
// constructor and therefore cannot bless a manifest they just recomputed.
#[cfg(test)]
pub(super) fn pin_trusted_generation_manifest_root(
    root_sha256: &str,
) -> Result<TrustedGenerationManifestRootV3, EffectLawV3Error> {
    if !valid_nonzero_sha256(root_sha256) {
        return Err(EffectLawV3Error::InvalidTrustRoot);
    }
    Ok(TrustedGenerationManifestRootV3(root_sha256.to_owned()))
}

fn effect_law_bundle_root_digest(
    bundle_bytes: &[u8],
    trust_manifest_root_sha256: &str,
    dictionary_root_sha256: &str,
    quotient_hypothesis_root_sha256: &str,
    canonicalizer_version: u16,
) -> Result<String, EffectLawV3Error> {
    evidence::sha256_serialized(&(
        TRUSTED_EFFECT_LAW_BUNDLE_ROOT_SCHEMA_V3,
        trust_manifest_root_sha256,
        dictionary_root_sha256,
        quotient_hypothesis_root_sha256,
        canonicalizer_version,
        sha256_bytes(bundle_bytes),
    ))
}

pub(super) fn validate_effect_law_bundle_root(
    bundle_bytes: &[u8],
    trusted: &TrustedEffectEvidenceSetV3,
    expected: &TrustedEffectLawBundleRootV3,
) -> Result<(), EffectLawV3Error> {
    if expected.schema != TRUSTED_EFFECT_LAW_BUNDLE_ROOT_SCHEMA_V3
        || expected.canonicalizer_version != EFFECT_LAW_IR_VERSION_V3
        || !valid_nonzero_sha256(&expected.bundle_root_sha256)
        || !valid_nonzero_sha256(&expected.trust_manifest_root_sha256)
        || !valid_nonzero_sha256(&expected.dictionary_root_sha256)
        || !valid_nonzero_sha256(&expected.quotient_hypothesis_root_sha256)
        || expected.trust_manifest_root_sha256 != trusted.trust_manifest_root_sha256
        || effect_law_bundle_root_digest(
            bundle_bytes,
            &expected.trust_manifest_root_sha256,
            &expected.dictionary_root_sha256,
            &expected.quotient_hypothesis_root_sha256,
            expected.canonicalizer_version,
        )? != expected.bundle_root_sha256
    {
        return Err(EffectLawV3Error::InvalidTrustRoot);
    }
    Ok(())
}

pub(super) fn validate_effect_law_bundle_identity(
    law: &CanonicalEffectLawV3,
    expected: &TrustedEffectLawBundleRootV3,
) -> Result<(), EffectLawV3Error> {
    if law.ir_version() != expected.canonicalizer_version
        || law.dictionary_root_sha256() != expected.dictionary_root_sha256
        || law.quotient_hypothesis_root_sha256() != expected.quotient_hypothesis_root_sha256
    {
        return Err(EffectLawV3Error::InvalidTrustRoot);
    }
    Ok(())
}

// The production generation owner deliberately has no constructor in F2.
// Tests pin the original immutable bundle before exercising forged restarts.
#[cfg(test)]
pub(super) fn pin_trusted_effect_law_bundle_root(
    bundle: &EffectLawRestartBundleV3,
    trusted: &TrustedEffectEvidenceSetV3,
) -> Result<TrustedEffectLawBundleRootV3, EffectLawV3Error> {
    let bytes = bundle.canonical_bytes()?;
    let dictionary_root_sha256 = bundle.law.dictionary_root_sha256().to_owned();
    let quotient_hypothesis_root_sha256 = bundle.law.quotient_hypothesis_root_sha256().to_owned();
    let canonicalizer_version = bundle.law.ir_version();
    let bundle_root_sha256 = effect_law_bundle_root_digest(
        &bytes,
        &trusted.trust_manifest_root_sha256,
        &dictionary_root_sha256,
        &quotient_hypothesis_root_sha256,
        canonicalizer_version,
    )?;
    Ok(TrustedEffectLawBundleRootV3 {
        schema: TRUSTED_EFFECT_LAW_BUNDLE_ROOT_SCHEMA_V3.to_owned(),
        bundle_root_sha256,
        trust_manifest_root_sha256: trusted.trust_manifest_root_sha256.clone(),
        dictionary_root_sha256,
        quotient_hypothesis_root_sha256,
        canonicalizer_version,
    })
}

fn strictly_ordered_by<T>(items: &[T], key: impl Fn(&T) -> &str) -> bool {
    items.windows(2).all(|pair| key(&pair[0]) < key(&pair[1]))
}

fn validate_independent_effect_state(
    state: &IndependentEffectStateV3,
) -> Result<(), EffectLawV3Error> {
    if state.schema != INDEPENDENT_EFFECT_STATE_SCHEMA_V3
        || !valid_nonzero_sha256(&state.evidence_ref_sha256)
        || !valid_nonzero_sha256(&state.before_atoms_root_sha256)
        || !valid_nonzero_sha256(&state.actor_response_sha256)
        || !valid_nonzero_sha256(&state.observer_root_sha256)
        || !valid_nonzero_sha256(&state.receipt_sha256)
        || state.effect_atoms.is_empty()
        || state.effect_atoms.len() > MAX_EFFECT_ATOMS_V3
        || state.effect_atoms.windows(2).any(|pair| pair[0] >= pair[1])
        || independent_effect_state_digest(state)? != state.receipt_sha256
    {
        return Err(EffectLawV3Error::InvalidVerifierReceipt);
    }
    Ok(())
}

pub(super) fn independent_effect_state_digest(
    state: &IndependentEffectStateV3,
) -> Result<String, EffectLawV3Error> {
    evidence::sha256_serialized(&(
        state.schema.as_str(),
        state.evidence_ref_sha256.as_str(),
        state.before_atoms_root_sha256.as_str(),
        state.actor_response_sha256.as_str(),
        &state.effect_atoms,
        state.observer_root_sha256.as_str(),
    ))
}

fn validate_manifest_entry(
    entry: &TrustedGenerationEvidenceEntryV3,
) -> Result<(), EffectLawV3Error> {
    if [
        entry.evidence_ref_sha256.as_str(),
        entry.transition_sha256.as_str(),
        entry.episode_lineage_sha256.as_str(),
        entry.surface_root_sha256.as_str(),
        entry.physical_program_id.as_str(),
        entry.capture_receipt_root_sha256.as_str(),
        entry.parity_receipt_root_sha256.as_str(),
        entry.observed_state_root_sha256.as_str(),
    ]
    .into_iter()
    .any(|value| !valid_nonzero_sha256(value))
    {
        return Err(EffectLawV3Error::InvalidTrustRoot);
    }
    Ok(())
}

pub(super) fn entry<'a>(
    trusted: &'a TrustedEffectEvidenceSetV3,
    evidence_ref_sha256: &str,
) -> Result<
    (
        &'a TrustedGenerationEvidenceEntryV3,
        &'a DurableRuntimeParityReceipt,
        &'a IndependentEffectStateV3,
    ),
    EffectLawV3Error,
> {
    if trusted.schema != TRUSTED_EFFECT_EVIDENCE_SET_SCHEMA_V3
        || !valid_nonzero_sha256(&trusted.generation_id_sha256)
        || !valid_nonzero_sha256(&trusted.trust_manifest_root_sha256)
        || !valid_nonzero_sha256(&trusted.delta_verifier_root_sha256)
        || !valid_nonzero_sha256(&trusted.resolver_root_sha256)
    {
        return Err(EffectLawV3Error::InvalidTrustRoot);
    }
    Ok((
        trusted
            .entry_by_evidence
            .get(evidence_ref_sha256)
            .ok_or(EffectLawV3Error::InvalidTrustRoot)?,
        trusted
            .parity_by_evidence
            .get(evidence_ref_sha256)
            .ok_or(EffectLawV3Error::InvalidTrustRoot)?,
        trusted
            .observed_state_by_evidence
            .get(evidence_ref_sha256)
            .ok_or(EffectLawV3Error::InvalidTrustRoot)?,
    ))
}

pub(super) fn validate_restart_proofs(
    proofs: &[ObservationCanonicalProofV3],
    trusted: &TrustedEffectEvidenceSetV3,
) -> Result<EffectLawIndependenceV3, EffectLawV3Error> {
    let mut observations = BTreeSet::new();
    let mut evidence_refs = BTreeSet::new();
    let mut episode_lineages = BTreeSet::new();
    let mut surface_roots = BTreeSet::new();
    let mut physical_program_ids = BTreeSet::new();
    for proof in proofs {
        let (entry, parity, observed) = entry(trusted, &proof.evidence_ref_sha256)?;
        if proof.transition_sha256 != entry.transition_sha256
            || proof.episode_lineage_sha256 != entry.episode_lineage_sha256
            || proof.surface_root_sha256 != entry.surface_root_sha256
            || proof.physical_program_id != entry.physical_program_id
            || proof.capture_receipt_root_sha256 != entry.capture_receipt_root_sha256
            || proof.parity_receipt_root_sha256 != parity.receipt_sha256
            || proof.verifier_root_sha256 != parity.verifier_sha256
            || proof.observed_state_root_sha256 != observed.receipt_sha256
            || proof.resolver_root_sha256 != trusted.resolver_root_sha256
            || proof.trust_manifest_root_sha256 != trusted.trust_manifest_root_sha256
            || proof.delta_verifier_root_sha256 != trusted.delta_verifier_root_sha256
            || !valid_nonzero_sha256(&proof.verified_delta_receipt_root_sha256)
            || !observations.insert(proof.observation_sha256.as_str())
            || !evidence_refs.insert(proof.evidence_ref_sha256.as_str())
        {
            return Err(EffectLawV3Error::InvalidRestartBundle);
        }
        episode_lineages.insert(proof.episode_lineage_sha256.as_str());
        surface_roots.insert(proof.surface_root_sha256.as_str());
        physical_program_ids.insert(proof.physical_program_id.as_str());
    }
    let independence = EffectLawIndependenceV3 {
        observations: observations.len(),
        episode_lineages: episode_lineages.len(),
        surface_roots: surface_roots.len(),
        physical_program_ids: physical_program_ids.len(),
    };
    if independence.observations != proofs.len()
        || independence.episode_lineages < 2
        || independence.surface_roots < 2
        || independence.physical_program_ids < 2
    {
        return Err(EffectLawV3Error::InvalidRestartBundle);
    }
    Ok(independence)
}
