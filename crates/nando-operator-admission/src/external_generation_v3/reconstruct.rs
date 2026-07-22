use std::collections::BTreeSet;

use nando_operator_learning::{
    GenerationCaptureIndexV3, GenerationShadowReceiptLedgerV3, GenerationShadowTerminalOutcomeV3,
    ProviderCaptureIndexV3,
};
use nando_operator_persistence::{
    decode_generation_checkpoint_v3, join_generation_checkpoint_to_capture_index_v3,
};
use nando_operator_proof::independent_verifier_v3::IndependentVerifierVerdictV3;
use nando_operator_runtime::TrafficShadowGenerationV3;

use super::{
    ExternalGenerationAdmissionCandidateV3, ExternalGenerationAdmissionErrorV3,
    ExternalGenerationAdmissionInputV3, ExternalGenerationAdmissionVerdictV3,
    ExternalPhaseControlReceiptV3, ReconstructedCommitmentInputV3,
    external_phase_control_traffic_set_sha256_v3, resource::validate_resource_receipt_v3,
};

pub fn reconstruct_external_generation_admission_candidate_v3(
    input: ExternalGenerationAdmissionInputV3<'_>,
) -> Result<ExternalGenerationAdmissionCandidateV3, ExternalGenerationAdmissionErrorV3> {
    ensure_present(&input)?;
    let checkpoint = decode_generation_checkpoint_v3(input.generation_checkpoint_bytes)
        .map_err(|_| ExternalGenerationAdmissionErrorV3::InvalidGenerationCheckpoint)?;
    let generation_capture =
        GenerationCaptureIndexV3::from_canonical_bytes(input.generation_capture_index_bytes)
            .map_err(|_| ExternalGenerationAdmissionErrorV3::InvalidGenerationCaptureIndex)?;
    let joined = join_generation_checkpoint_to_capture_index_v3(checkpoint, &generation_capture)
        .map_err(|_| ExternalGenerationAdmissionErrorV3::CaptureJoinMismatch)?;
    let checkpoint = joined.into_checkpoint();
    let provider_capture =
        ProviderCaptureIndexV3::from_canonical_bytes(input.provider_capture_index_bytes)
            .map_err(|_| ExternalGenerationAdmissionErrorV3::InvalidProviderCaptureIndex)?;
    let shadow = GenerationShadowReceiptLedgerV3::from_canonical_bytes(input.shadow_ledger_bytes)
        .map_err(|_| ExternalGenerationAdmissionErrorV3::InvalidShadowLedger)?;
    let controls =
        ExternalPhaseControlReceiptV3::from_canonical_bytes(input.phase_control_receipt_bytes)?;
    let resource = validate_resource_receipt_v3(input.resource_receipt_bytes)?;

    let manifest = checkpoint.generation().manifest();
    if shadow.generation_id_sha256() != manifest.generation_id_sha256()
        || shadow.generation_publish_sequence() != checkpoint.publish_sequence()
        || shadow.generation_checkpoint_sha256() != checkpoint.checkpoint_sha256()
        || controls.generation_id_sha256() != manifest.generation_id_sha256()
    {
        return Err(ExternalGenerationAdmissionErrorV3::GenerationDrift);
    }
    let traffic_generation =
        TrafficShadowGenerationV3::from_restored_generation(checkpoint.generation())
            .map_err(|_| ExternalGenerationAdmissionErrorV3::GenerationDrift)?;
    let live = validate_live_shadow(
        &shadow,
        &provider_capture,
        &traffic_generation,
        manifest.components().artifact_set_sha256.as_str(),
        &frozen_lineages(&checkpoint),
    )?;
    let traffic_receipt_roots = shadow
        .receipts()
        .iter()
        .map(|receipt| receipt.traffic_receipt_sha256().to_owned())
        .collect::<Vec<_>>();
    let traffic_receipt_set_sha256 = external_phase_control_traffic_set_sha256_v3(
        manifest.generation_id_sha256(),
        &traffic_receipt_roots,
    )?;
    if controls.traffic_receipt_set_sha256() != traffic_receipt_set_sha256 {
        return Err(ExternalGenerationAdmissionErrorV3::ControlTrafficMismatch);
    }
    let accounting = checkpoint.ledger().accounting();
    let verdict = if controls.full_phase_gain() > 0 {
        ExternalGenerationAdmissionVerdictV3::ShadowReady
    } else {
        ExternalGenerationAdmissionVerdictV3::WatchNoCausalGain
    };
    ExternalGenerationAdmissionCandidateV3::from_reconstructed(ReconstructedCommitmentInputV3 {
        generation_id_sha256: manifest.generation_id_sha256().to_owned(),
        generation_checkpoint_sha256: checkpoint.checkpoint_sha256().to_owned(),
        generation_capture_index_sha256: generation_capture.index_sha256().to_owned(),
        provider_capture_index_sha256: provider_capture.index_sha256().to_hex(),
        shadow_ledger_sha256: shadow.ledger_sha256().to_owned(),
        artifact_set_sha256: manifest.components().artifact_set_sha256.clone(),
        dispatch_index_sha256: manifest.components().dispatch_index_sha256.clone(),
        support_evidence_sha256: checkpoint.evidence_root_sha256().to_owned(),
        future_partition_sha256: checkpoint
            .ledger()
            .future_partition_sha256()
            .map_err(|_| ExternalGenerationAdmissionErrorV3::InvalidGenerationCheckpoint)?,
        phase_control_receipt_sha256: controls.receipt_sha256().to_owned(),
        resource_receipt_sha256: resource.receipt_sha256,
        support_denominator: count(accounting.support_rows)?,
        future_denominator: count(accounting.future_rows)?,
        live_shadow_denominator: count(shadow.receipts().len())?,
        live_verified_passes: live.verified_passes,
        negative_denominator: live.rejects,
        censored_denominator: live.censored,
        verdict,
    })
}

struct LiveShadowAccountingV3 {
    verified_passes: u32,
    rejects: u32,
    censored: u32,
}

fn validate_live_shadow(
    shadow: &GenerationShadowReceiptLedgerV3,
    provider_capture: &ProviderCaptureIndexV3,
    traffic_generation: &TrafficShadowGenerationV3,
    artifact_set_sha256: &str,
    frozen_lineages: &BTreeSet<String>,
) -> Result<LiveShadowAccountingV3, ExternalGenerationAdmissionErrorV3> {
    let mut accounting = LiveShadowAccountingV3 {
        verified_passes: 0,
        rejects: 0,
        censored: 0,
    };
    for receipt in shadow.receipts() {
        let Some(capture) = provider_capture.find_exact(
            receipt.capture_sequence(),
            receipt.capture_event_sha256(),
            receipt.request_sha256(),
            receipt.capture_receipt_sha256(),
        ) else {
            return Err(ExternalGenerationAdmissionErrorV3::CaptureJoinMismatch);
        };
        if frozen_lineages.contains(&capture.lineage_root_sha256().to_hex()) {
            return Err(ExternalGenerationAdmissionErrorV3::CaptureJoinMismatch);
        }
        if receipt.traffic_generation_sequence() != traffic_generation.sequence()
            || receipt.traffic_generation_id_sha256() != traffic_generation.generation_root_sha256()
            || receipt.traffic_index_sha256() != traffic_generation.index_sha256()
            || receipt.traffic_request_sha256() != receipt.request_sha256().to_hex()
            || receipt.raw_payloads_persisted() != 0
            || receipt.local_accepts() != 0
            || receipt.execution_authority()
        {
            return Err(ExternalGenerationAdmissionErrorV3::GenerationDrift);
        }
        if receipt.parity_mismatch() {
            return Err(ExternalGenerationAdmissionErrorV3::RuntimeParityMismatch);
        }
        validate_live_verifier(receipt, artifact_set_sha256)?;
        match receipt.outcome() {
            GenerationShadowTerminalOutcomeV3::VerifiedPass => accounting.verified_passes += 1,
            GenerationShadowTerminalOutcomeV3::RuntimeReject
            | GenerationShadowTerminalOutcomeV3::VerifierReject => accounting.rejects += 1,
            GenerationShadowTerminalOutcomeV3::Censored => accounting.censored += 1,
            GenerationShadowTerminalOutcomeV3::RuntimeAbstain
            | GenerationShadowTerminalOutcomeV3::VerifierAbstain => {}
        }
    }
    Ok(accounting)
}

fn frozen_lineages(
    checkpoint: &nando_operator_persistence::RestoredGenerationCheckpointV3,
) -> BTreeSet<String> {
    checkpoint
        .ledger()
        .support()
        .iter()
        .chain(checkpoint.ledger().future())
        .map(|record| record.observation().lineage_root_sha256().to_owned())
        .collect()
}

fn validate_live_verifier(
    receipt: &nando_operator_learning::GenerationShadowReceiptV3,
    artifact_set_sha256: &str,
) -> Result<(), ExternalGenerationAdmissionErrorV3> {
    match (receipt.outcome(), receipt.verifier_receipt()) {
        (GenerationShadowTerminalOutcomeV3::VerifiedPass, Some(verifier)) => {
            if verifier.verdict() != IndependentVerifierVerdictV3::Verified
                || verifier.artifact_set_sha256() != artifact_set_sha256
                || verifier.request_sha256() != receipt.request_sha256().to_hex()
                || verifier.actor_physical_action_sha256()
                    != receipt.actor_action_sha256().unwrap_or_default()
                || verifier.actor_output_sha256()
                    != receipt.actor_output_sha256().unwrap_or_default()
                || verifier.receipt_sha256()
                    != receipt.verifier_receipt_sha256().unwrap_or_default()
                || receipt.semantic_updates() != 1
            {
                return Err(ExternalGenerationAdmissionErrorV3::RuntimeParityMismatch);
            }
        }
        (GenerationShadowTerminalOutcomeV3::VerifierAbstain, Some(verifier))
            if verifier.verdict() != IndependentVerifierVerdictV3::Verified
                && receipt.semantic_updates() == 0 => {}
        (GenerationShadowTerminalOutcomeV3::VerifierReject, Some(verifier))
            if verifier.verdict() != IndependentVerifierVerdictV3::Verified
                && receipt.semantic_updates() == 0 => {}
        (
            GenerationShadowTerminalOutcomeV3::RuntimeAbstain
            | GenerationShadowTerminalOutcomeV3::RuntimeReject
            | GenerationShadowTerminalOutcomeV3::Censored,
            None,
        ) if receipt.semantic_updates() == 0 => {}
        _ => return Err(ExternalGenerationAdmissionErrorV3::RuntimeParityMismatch),
    }
    Ok(())
}

fn ensure_present(
    input: &ExternalGenerationAdmissionInputV3<'_>,
) -> Result<(), ExternalGenerationAdmissionErrorV3> {
    [
        input.generation_checkpoint_bytes,
        input.generation_capture_index_bytes,
        input.provider_capture_index_bytes,
        input.shadow_ledger_bytes,
        input.phase_control_receipt_bytes,
        input.resource_receipt_bytes,
    ]
    .into_iter()
    .all(|bytes| !bytes.is_empty())
    .then_some(())
    .ok_or(ExternalGenerationAdmissionErrorV3::MissingInput)
}

fn count(value: usize) -> Result<u32, ExternalGenerationAdmissionErrorV3> {
    u32::try_from(value).map_err(|_| ExternalGenerationAdmissionErrorV3::Serialization)
}
