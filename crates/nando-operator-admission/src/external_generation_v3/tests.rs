#[path = "../../../nando-operator-persistence/tests/f7_support/mod.rs"]
mod f7_support;

use std::{env, fs, path::PathBuf, sync::Arc};

use f7_support::{FixtureV3, root};
use nando_operator_kernel::{RuntimeProjectionV3, Sha256CommitmentV3, sha256_bytes};
use nando_operator_learning::{
    GenerationCaptureCommitmentV3, GenerationCaptureIndexV3, GenerationShadowReceiptInputV3,
    GenerationShadowReceiptLedgerV3, GenerationShadowTerminalOutcomeV3, ProviderCaptureIndexV3,
    ProviderRequestCaptureInputV3, seal_provider_request_capture_v3,
};
use nando_operator_persistence::{
    GenerationCheckpointStoreV3, GenerationShadowReceiptStoreV3, ProviderCaptureStoreReaderV3,
    decode_generation_checkpoint_v3,
};
use nando_operator_runtime::{
    RuntimeContextBudgetV3, TrafficShadowGenerationV3, TrafficShadowInputV3, TrafficShadowSourceV3,
    TrafficShadowVerdictV3, execute_traffic_shadow_with_handoff_v3,
};

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
        Self::build(true)
    }

    fn watch() -> Self {
        Self::build(false)
    }

    fn build(matched: bool) -> Self {
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
        let f6 = matched.then(|| fixture.support_f6_receipt());
        let mut payload = f7_support::support_request_payload();
        if !matched {
            payload["tools"] = serde_json::json!([]);
        }
        let payload_bytes = serde_json::to_vec(&payload).expect("provider payload");
        let request_sha256 = sha256_bytes(&payload_bytes);
        if let Some(f6) = &f6 {
            assert_eq!(f6.request_sha256(), request_sha256);
        }
        let capture = seal_provider_request_capture_v3(ProviderRequestCaptureInputV3 {
            capture_sequence: lease.first_sequence(),
            capture_epoch_root: lease.epoch_root_sha256(),
            lineage_root_sha256: Sha256CommitmentV3::digest_bytes(b"independent-live-lineage"),
            request_root_sha256: Sha256CommitmentV3::from_hex(&request_sha256)
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
        let traffic_generation = Arc::new(
            TrafficShadowGenerationV3::from_restored_generation(restored.generation())
                .expect("traffic generation"),
        );
        let window_row_sha256 = capture.event_root_sha256().to_hex();
        let traffic_input = TrafficShadowInputV3::replayable(
            &window_row_sha256,
            &request_sha256,
            RuntimeProjectionV3::Responses,
            false,
            TrafficShadowSourceV3::Ordinary,
            "continue CellA17",
            &payload,
        )
        .expect("traffic input");
        let execution = execute_traffic_shadow_with_handoff_v3(
            traffic_generation,
            traffic_input,
            RuntimeContextBudgetV3::default(),
        );
        let traffic = execution.receipt();
        if matched {
            assert_eq!(traffic.verdict(), TrafficShadowVerdictV3::CompleteShadow);
            assert_eq!(
                execution
                    .actor_action()
                    .expect("actor action")
                    .physical_action_sha256(),
                f6.as_ref().expect("f6").actor_physical_action_sha256()
            );
        } else {
            assert_eq!(traffic.verdict(), TrafficShadowVerdictV3::AbstainDispatch);
            assert!(execution.phase_control_evidence().is_none());
        }
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
                    traffic_receipt_sha256: traffic.receipt_sha256(),
                    traffic_generation_sequence: traffic.generation_sequence(),
                    traffic_generation_id_sha256: traffic.generation_root_sha256(),
                    traffic_index_sha256: traffic.index_sha256(),
                    traffic_request_sha256: traffic.request_sha256(),
                    traffic_verdict_code: traffic.verdict() as u8,
                    traffic_phase_report_sha256: traffic.phase_report_sha256(),
                    traffic_operator_receipt_sha256: traffic.operator_shadow_receipt_sha256(),
                    phase_control_evidence: execution.phase_control_evidence(),
                    f6_receipt: f6.as_ref(),
                    outcome: if matched {
                        GenerationShadowTerminalOutcomeV3::VerifiedPass
                    } else {
                        GenerationShadowTerminalOutcomeV3::RuntimeAbstain
                    },
                    parity_mismatch: false,
                },
            )
            .expect("shadow append");
        let controls =
            derive_external_phase_control_receipt_v3(manifest.generation_id_sha256(), &shadow, 0)
                .expect("derived controls")
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
    let controls = AdmissionFixtureV3::watch().controls;
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
    let fixture = AdmissionFixtureV3::watch();
    let candidate = reconstruct_external_generation_admission_candidate_v3(fixture.input())
        .expect("watch candidate");
    assert_eq!(
        candidate.verdict(),
        ExternalGenerationAdmissionVerdictV3::WatchNoCausalGain
    );
}

#[test]
#[ignore = "explicit F8-E immutable live-snapshot audit only"]
fn immutable_live_snapshot_reconstructs_shadow_ready_without_authority() {
    let generation_store =
        GenerationCheckpointStoreV3::open(required_path("NANDO_F8_LIVE_GENERATION_STORE"))
            .expect("generation store");
    let generation_restore = generation_store.restore().expect("generation restore");
    assert!(generation_restore.quarantined_files().is_empty());
    let checkpoint = generation_restore
        .checkpoint()
        .expect("generation checkpoint");

    let generation_capture = fs::read(required_path("NANDO_F8_LIVE_GENERATION_CAPTURE"))
        .expect("generation capture index");
    let provider_store =
        ProviderCaptureStoreReaderV3::open(required_path("NANDO_F8_LIVE_PROVIDER_CAPTURE_STORE"))
            .expect("provider capture store");
    let provider_restore = provider_store.restore().expect("provider capture restore");
    assert!(provider_restore.quarantined_files().is_empty());
    let provider = provider_restore.index().expect("provider capture index");

    // The audit runs on a stopped-and-copied snapshot, never against a live writer.
    let shadow_store =
        GenerationShadowReceiptStoreV3::open(required_path("NANDO_F8_LIVE_SHADOW_STORE"))
            .expect("shadow store");
    let shadow_restore = shadow_store.restore().expect("shadow restore");
    assert!(shadow_restore.quarantined_files().is_empty());
    let shadow = shadow_restore.ledger().expect("shadow ledger");
    let controls = derive_external_phase_control_receipt_v3(
        checkpoint.generation().manifest().generation_id_sha256(),
        shadow,
        0,
    )
    .expect("derived controls");
    let control_bytes = controls.canonical_bytes().expect("control bytes");
    assert!(controls.safety_zero());
    assert!(controls.full_phase_gain() > 0);

    let provider_bytes = provider.canonical_bytes().expect("provider bytes");
    let shadow_bytes = shadow.canonical_bytes().expect("shadow bytes");
    let candidate = reconstruct_external_generation_admission_candidate_v3(
        ExternalGenerationAdmissionInputV3 {
            generation_checkpoint_bytes: checkpoint.canonical_bytes(),
            generation_capture_index_bytes: &generation_capture,
            provider_capture_index_bytes: &provider_bytes,
            shadow_ledger_bytes: &shadow_bytes,
            phase_control_receipt_bytes: &control_bytes,
            resource_receipt_bytes: RESOURCE_RECEIPT,
        },
    )
    .expect("external reconstruction");
    assert_eq!(
        candidate.verdict(),
        ExternalGenerationAdmissionVerdictV3::ShadowReady
    );
    assert!(candidate.live_shadow_denominator() >= 2);
    assert_eq!(
        candidate.live_verified_passes(),
        candidate.live_shadow_denominator()
    );
    assert!(!candidate.execution_authority());
    let verified =
        verify_external_generation_submission_v3(&candidate, candidate.canonical_commitments())
            .expect("byte-identical submission");
    assert!(!verified.execution_authority());

    let output = required_path("NANDO_F8_LIVE_AUDIT_OUTPUT");
    assert!(
        !output.exists() || fs::read_dir(&output).expect("read output").next().is_none(),
        "refusing to overwrite a non-empty audit output"
    );
    fs::create_dir_all(&output).expect("create output");
    fs::write(
        output.join("external-phase-controls.v3.json"),
        &control_bytes,
    )
    .expect("write controls");
    fs::write(
        output.join("external-admission-candidate.v3.json"),
        candidate.canonical_commitments(),
    )
    .expect("write candidate");
    println!(
        "F8E_LIVE_AUDIT verdict=shadow_ready live={} verified={} gain={} authority=false commitments={}",
        candidate.live_shadow_denominator(),
        candidate.live_verified_passes(),
        controls.full_phase_gain(),
        candidate.commitments_sha256(),
    );
}

fn required_path(name: &str) -> PathBuf {
    PathBuf::from(env::var(name).unwrap_or_else(|_| panic!("{name} must be set")))
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
