#[path = "../../../nando-operator-persistence/tests/f7_support/mod.rs"]
mod f7_support;

use f7_support::{FixtureV3, root};
use nando_operator_kernel::{RuntimeProjectionV3, Sha256CommitmentV3};
use nando_operator_learning::{
    GenerationCaptureCommitmentV3, GenerationCaptureIndexV3, GenerationShadowReceiptInputV3,
    GenerationShadowReceiptLedgerV3, GenerationShadowTerminalOutcomeV3, ProviderCaptureIndexV3,
    ProviderRequestCaptureInputV3, seal_provider_request_capture_v3,
};
use nando_operator_persistence::decode_generation_checkpoint_v3;

use super::*;

const RESOURCE_RECEIPT: &[u8] =
    include_bytes!("../../../../plans/effect-law-unification-v1/STOP_F8_0_RESOURCE_TRUTH.json");

struct AdmissionFixtureV3 {
    checkpoint: Box<[u8]>,
    generation_capture: Box<[u8]>,
    provider_capture: Box<[u8]>,
    shadow: Box<[u8]>,
    controls: Box<[u8]>,
}

impl AdmissionFixtureV3 {
    fn new() -> Self {
        let mut fixture = FixtureV3::new("f8c-external-reconstruction");
        fixture.append_support();
        fixture.freeze_and_append_future();
        let checkpoint = fixture.checkpoint(1);
        let restored = decode_generation_checkpoint_v3(&checkpoint).expect("checkpoint");
        let generation_capture = generation_capture(&restored)
            .canonical_bytes()
            .expect("generation capture");

        let (reserved, lease) = ProviderCaptureIndexV3::empty()
            .expect("empty capture")
            .reserve_next_lease()
            .expect("capture lease");
        let f6 = fixture.support_f6_receipt();
        let capture = seal_provider_request_capture_v3(ProviderRequestCaptureInputV3 {
            capture_sequence: lease.first_sequence(),
            capture_epoch_root: lease.epoch_root_sha256(),
            lineage_root_sha256: Sha256CommitmentV3::digest_bytes(b"independent-live-lineage"),
            request_root_sha256: Sha256CommitmentV3::from_hex(f6.request_sha256())
                .expect("request root"),
            projection: RuntimeProjectionV3::Responses,
            streaming: false,
            observed_at_unix_ms: 1_750_000_000_000,
        })
        .expect("capture");
        let provider_capture = reserved
            .append_batch(std::slice::from_ref(&capture))
            .expect("provider append");
        let provider_capture_bytes = provider_capture
            .canonical_bytes()
            .expect("provider capture bytes");
        let manifest = restored.generation().manifest();
        let mut shadow = GenerationShadowReceiptLedgerV3::new(
            manifest.generation_id_sha256().to_owned(),
            restored.publish_sequence(),
            restored.checkpoint_sha256().to_owned(),
        )
        .expect("shadow");
        shadow
            .append(
                &provider_capture,
                GenerationShadowReceiptInputV3 {
                    capture_receipt: &capture,
                    traffic_receipt_sha256: &root("f8c traffic receipt"),
                    traffic_generation_sequence: manifest.sequence(),
                    traffic_generation_id_sha256: manifest.generation_id_sha256(),
                    traffic_index_sha256: &manifest.components().dispatch_index_sha256,
                    traffic_request_sha256: f6.request_sha256(),
                    traffic_verdict_code: 1,
                    traffic_phase_report_sha256: Some(&root("f8c phase report")),
                    traffic_operator_receipt_sha256: Some(&root("f8c operator receipt")),
                    f6_receipt: Some(&f6),
                    outcome: GenerationShadowTerminalOutcomeV3::VerifiedPass,
                    parity_mismatch: false,
                },
            )
            .expect("shadow append");
        let traffic_receipt_set_sha256 = external_phase_control_traffic_set_sha256_v3(
            manifest.generation_id_sha256(),
            &shadow
                .receipts()
                .iter()
                .map(|receipt| receipt.traffic_receipt_sha256().to_owned())
                .collect::<Vec<_>>(),
        )
        .expect("traffic receipt set");
        let controls = phase_controls(
            manifest.generation_id_sha256(),
            &traffic_receipt_set_sha256,
            1,
            0,
        )
        .canonical_bytes()
        .expect("controls");
        Self {
            checkpoint,
            generation_capture,
            provider_capture: provider_capture_bytes,
            shadow: shadow.canonical_bytes().expect("shadow bytes"),
            controls,
        }
    }

    fn input(&self) -> ExternalGenerationAdmissionInputV3<'_> {
        ExternalGenerationAdmissionInputV3 {
            generation_checkpoint_bytes: &self.checkpoint,
            generation_capture_index_bytes: &self.generation_capture,
            provider_capture_index_bytes: &self.provider_capture,
            shadow_ledger_bytes: &self.shadow,
            phase_control_receipt_bytes: &self.controls,
            resource_receipt_bytes: RESOURCE_RECEIPT,
        }
    }
}

#[test]
fn immutable_inputs_reconstruct_one_no_authority_shadow_candidate() {
    let fixture = AdmissionFixtureV3::new();
    let candidate =
        reconstruct_external_generation_admission_candidate_v3(fixture.input()).expect("candidate");
    assert_eq!(
        candidate.verdict(),
        ExternalGenerationAdmissionVerdictV3::ShadowReady
    );
    assert_eq!(candidate.support_denominator(), 1);
    assert_eq!(candidate.future_denominator(), 1);
    assert_eq!(candidate.live_shadow_denominator(), 1);
    assert_eq!(candidate.live_verified_passes(), 1);
    assert!(!candidate.execution_authority());

    let verified =
        verify_external_generation_submission_v3(&candidate, candidate.canonical_commitments())
            .expect("byte-identical submission");
    assert_eq!(
        verified.commitments_sha256(),
        candidate.commitments_sha256()
    );
    assert!(!verified.execution_authority());
}

#[test]
fn missing_unknown_and_drifted_inputs_fail_closed() {
    let fixture = AdmissionFixtureV3::new();
    let missing = ExternalGenerationAdmissionInputV3 {
        phase_control_receipt_bytes: &[],
        ..fixture.input()
    };
    assert_eq!(
        reconstruct_external_generation_admission_candidate_v3(missing).err(),
        Some(ExternalGenerationAdmissionErrorV3::MissingInput)
    );

    let mut drifted_shadow = fixture.shadow.to_vec();
    *drifted_shadow.last_mut().expect("last byte") ^= 1;
    let drifted = ExternalGenerationAdmissionInputV3 {
        shadow_ledger_bytes: &drifted_shadow,
        ..fixture.input()
    };
    assert_eq!(
        reconstruct_external_generation_admission_candidate_v3(drifted).err(),
        Some(ExternalGenerationAdmissionErrorV3::InvalidShadowLedger)
    );

    let mut unknown_resource: serde_json::Value =
        serde_json::from_slice(RESOURCE_RECEIPT).expect("resource json");
    unknown_resource["schema"] = serde_json::Value::String("unknown.resource.v9".to_owned());
    let unknown_resource = serde_json::to_vec(&unknown_resource).expect("resource bytes");
    let unknown = ExternalGenerationAdmissionInputV3 {
        resource_receipt_bytes: &unknown_resource,
        ..fixture.input()
    };
    assert_eq!(
        reconstruct_external_generation_admission_candidate_v3(unknown).err(),
        Some(ExternalGenerationAdmissionErrorV3::UnknownSchema)
    );
}

#[test]
fn commitment_submission_tampering_is_blocked() {
    let fixture = AdmissionFixtureV3::new();
    let candidate =
        reconstruct_external_generation_admission_candidate_v3(fixture.input()).expect("candidate");
    let mut tampered = candidate.canonical_commitments().to_vec();
    *tampered.last_mut().expect("last byte") ^= 1;
    assert_eq!(
        verify_external_generation_submission_v3(&candidate, &tampered).err(),
        Some(ExternalGenerationAdmissionErrorV3::CommitmentDrift)
    );
}

#[test]
fn phase_controls_cannot_claim_a_different_traffic_set() {
    let fixture = AdmissionFixtureV3::new();
    let checkpoint = decode_generation_checkpoint_v3(&fixture.checkpoint).expect("checkpoint");
    let controls = phase_controls(
        checkpoint.generation().manifest().generation_id_sha256(),
        &root("foreign traffic set"),
        1,
        0,
    )
    .canonical_bytes()
    .expect("foreign controls");
    let input = ExternalGenerationAdmissionInputV3 {
        phase_control_receipt_bytes: &controls,
        ..fixture.input()
    };
    assert_eq!(
        reconstruct_external_generation_admission_candidate_v3(input).err(),
        Some(ExternalGenerationAdmissionErrorV3::ControlTrafficMismatch)
    );
}

#[test]
fn equal_phase_controls_remain_watch() {
    let fixture = AdmissionFixtureV3::new();
    let checkpoint = decode_generation_checkpoint_v3(&fixture.checkpoint).expect("checkpoint");
    let controls = phase_controls(
        checkpoint.generation().manifest().generation_id_sha256(),
        ExternalPhaseControlReceiptV3::from_canonical_bytes(&fixture.controls)
            .expect("existing controls")
            .traffic_receipt_set_sha256(),
        1,
        1,
    )
    .canonical_bytes()
    .expect("controls");
    let input = ExternalGenerationAdmissionInputV3 {
        phase_control_receipt_bytes: &controls,
        ..fixture.input()
    };
    let candidate =
        reconstruct_external_generation_admission_candidate_v3(input).expect("watch candidate");
    assert_eq!(
        candidate.verdict(),
        ExternalGenerationAdmissionVerdictV3::WatchNoCausalGain
    );
}

fn phase_controls(
    generation_id_sha256: &str,
    traffic_receipt_set_sha256: &str,
    full_correct: u32,
    control_correct: u32,
) -> ExternalPhaseControlReceiptV3 {
    seal_external_phase_control_receipt_v3(ExternalPhaseControlReceiptInputV3 {
        generation_id_sha256: generation_id_sha256.to_owned(),
        observations: ExternalPhaseControlV3::ALL
            .into_iter()
            .map(|control| ExternalPhaseControlObservationInputV3 {
                control,
                traffic_receipt_set_sha256: traffic_receipt_set_sha256.to_owned(),
                correct_actions: if control == ExternalPhaseControlV3::Full {
                    full_correct
                } else {
                    control_correct
                },
                wrong_actions: 0,
                exact_checks: 1,
                selected_actions: if control == ExternalPhaseControlV3::Full {
                    full_correct
                } else {
                    control_correct
                },
            })
            .collect(),
        false_accepts: 0,
        parity_mismatches: 0,
        restart_mismatches: 0,
        censored_semantic_updates: 0,
        support_future_overlap: 0,
    })
    .expect("phase controls")
}

fn generation_capture(
    checkpoint: &nando_operator_persistence::RestoredGenerationCheckpointV3,
) -> GenerationCaptureIndexV3 {
    GenerationCaptureIndexV3::new(
        checkpoint
            .receipts()
            .iter()
            .enumerate()
            .map(|(index, pair)| {
                let receipt = pair.generation_receipt();
                GenerationCaptureCommitmentV3::new(
                    receipt.capture_sequence(),
                    root(&format!("f8c capture {index}")),
                    receipt.lineage_root_sha256().to_owned(),
                    receipt.event_root_sha256().to_owned(),
                    receipt.f6_request_sha256().to_owned(),
                )
                .expect("generation capture")
            })
            .collect(),
    )
    .expect("generation capture index")
}
