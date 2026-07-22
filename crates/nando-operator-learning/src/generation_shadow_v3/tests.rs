use nando_operator_kernel::{RuntimeProjectionV3, Sha256CommitmentV3};

use crate::{
    ProviderCaptureIndexV3, ProviderRequestCaptureInputV3, ProviderRequestCaptureReceiptV3,
    seal_provider_request_capture_v3,
};

use super::*;

fn root(label: &str) -> String {
    Sha256CommitmentV3::digest_bytes(label.as_bytes()).to_hex()
}

fn capture(
    sequence: u64,
    epoch: Sha256CommitmentV3,
    label: &str,
) -> ProviderRequestCaptureReceiptV3 {
    seal_provider_request_capture_v3(ProviderRequestCaptureInputV3 {
        capture_sequence: sequence,
        capture_epoch_root: epoch,
        lineage_root_sha256: Sha256CommitmentV3::digest_bytes(
            format!("lineage-{label}").as_bytes(),
        ),
        request_root_sha256: Sha256CommitmentV3::digest_bytes(
            format!("request-{label}").as_bytes(),
        ),
        projection: RuntimeProjectionV3::Responses,
        streaming: true,
        observed_at_unix_ms: 1_750_000_000_000 + sequence,
    })
    .expect("capture")
}

fn reserved_captures(
    count: usize,
) -> (ProviderCaptureIndexV3, Vec<ProviderRequestCaptureReceiptV3>) {
    let (reserved, lease) = ProviderCaptureIndexV3::empty()
        .expect("empty")
        .reserve_next_lease()
        .expect("lease");
    let receipts = (1..=count)
        .map(|sequence| {
            capture(
                sequence as u64,
                lease.epoch_root_sha256(),
                &sequence.to_string(),
            )
        })
        .collect::<Vec<_>>();
    let index = reserved.append_batch(&receipts).expect("append");
    (index, receipts)
}

#[test]
fn censored_shadow_receipts_roundtrip_without_semantic_authority() {
    let (index, captures) = reserved_captures(2);
    let mut ledger =
        GenerationShadowReceiptLedgerV3::new(root("generation"), 7, root("checkpoint"))
            .expect("ledger");
    for (offset, capture) in captures.iter().enumerate() {
        ledger
            .append(
                &index,
                GenerationShadowReceiptInputV3 {
                    capture_receipt: capture,
                    traffic_receipt_sha256: &root(&format!("traffic-{offset}")),
                    traffic_generation_sequence: 3,
                    traffic_generation_id_sha256: &root("generation"),
                    traffic_index_sha256: &root("index"),
                    traffic_request_sha256: &capture.request_root_sha256().to_hex(),
                    traffic_verdict_code: 2,
                    traffic_phase_report_sha256: None,
                    traffic_operator_receipt_sha256: None,
                    f6_receipt: None,
                    outcome: GenerationShadowTerminalOutcomeV3::Censored,
                    parity_mismatch: false,
                },
            )
            .expect("append");
    }
    let bytes = ledger.canonical_bytes().expect("bytes");
    let restored = GenerationShadowReceiptLedgerV3::from_canonical_bytes(&bytes).expect("restore");
    assert_eq!(restored, ledger);
    assert_eq!(restored.publish_sequence(), 2);
    assert_eq!(restored.raw_payloads_persisted(), 0);
    assert!(!restored.execution_authority());
    assert!(
        restored
            .receipts()
            .iter()
            .all(|receipt| receipt.semantic_updates() == 0)
    );
}

#[test]
fn ledger_blocks_missing_capture_duplicate_and_foreign_generation_transition() {
    let (index, captures) = reserved_captures(2);
    let mut ledger =
        GenerationShadowReceiptLedgerV3::new(root("generation"), 7, root("checkpoint"))
            .expect("ledger");
    let input = GenerationShadowReceiptInputV3 {
        capture_receipt: &captures[0],
        traffic_receipt_sha256: &root("traffic"),
        traffic_generation_sequence: 3,
        traffic_generation_id_sha256: &root("generation"),
        traffic_index_sha256: &root("index"),
        traffic_request_sha256: &captures[0].request_root_sha256().to_hex(),
        traffic_verdict_code: 2,
        traffic_phase_report_sha256: None,
        traffic_operator_receipt_sha256: None,
        f6_receipt: None,
        outcome: GenerationShadowTerminalOutcomeV3::RuntimeAbstain,
        parity_mismatch: false,
    };
    ledger.append(&index, input.clone()).expect("append");
    assert_eq!(
        ledger.append(&index, input),
        Err(GenerationShadowLedgerErrorV3::NonMonotonicCapture)
    );

    let empty = ProviderCaptureIndexV3::empty().expect("empty");
    let mut missing =
        GenerationShadowReceiptLedgerV3::new(root("generation"), 7, root("checkpoint"))
            .expect("ledger");
    assert_eq!(
        missing.append(
            &empty,
            GenerationShadowReceiptInputV3 {
                capture_receipt: &captures[1],
                traffic_receipt_sha256: &root("traffic-2"),
                traffic_generation_sequence: 3,
                traffic_generation_id_sha256: &root("generation"),
                traffic_index_sha256: &root("index"),
                traffic_request_sha256: &captures[1].request_root_sha256().to_hex(),
                traffic_verdict_code: 2,
                traffic_phase_report_sha256: None,
                traffic_operator_receipt_sha256: None,
                f6_receipt: None,
                outcome: GenerationShadowTerminalOutcomeV3::Censored,
                parity_mismatch: false,
            }
        ),
        Err(GenerationShadowLedgerErrorV3::InvalidCaptureJoin)
    );

    let bytes = ledger.canonical_bytes().expect("bytes");
    let mut foreign =
        GenerationShadowReceiptLedgerV3::from_canonical_bytes(&bytes).expect("restore");
    foreign.generation_id_sha256 = root("foreign");
    assert_eq!(
        foreign.validate_extension_from(&ledger),
        Err(GenerationShadowLedgerErrorV3::InvalidGeneration)
    );
}

#[test]
fn verified_pass_cannot_be_relabelled_without_an_f6_receipt() {
    let (index, captures) = reserved_captures(1);
    let mut ledger =
        GenerationShadowReceiptLedgerV3::new(root("generation"), 7, root("checkpoint"))
            .expect("ledger");
    assert_eq!(
        ledger.append(
            &index,
            GenerationShadowReceiptInputV3 {
                capture_receipt: &captures[0],
                traffic_receipt_sha256: &root("traffic"),
                traffic_generation_sequence: 3,
                traffic_generation_id_sha256: &root("generation"),
                traffic_index_sha256: &root("index"),
                traffic_request_sha256: &captures[0].request_root_sha256().to_hex(),
                traffic_verdict_code: 1,
                traffic_phase_report_sha256: None,
                traffic_operator_receipt_sha256: None,
                f6_receipt: None,
                outcome: GenerationShadowTerminalOutcomeV3::VerifiedPass,
                parity_mismatch: false,
            }
        ),
        Err(GenerationShadowLedgerErrorV3::OutcomeMismatch)
    );
}

#[test]
fn provider_capture_exact_join_checks_all_commitments() {
    let (index, captures) = reserved_captures(1);
    let capture = &captures[0];
    assert!(index.contains_exact(
        capture.capture_sequence(),
        capture.event_root_sha256(),
        capture.request_root_sha256(),
        capture.receipt_sha256(),
    ));
    assert!(!index.contains_exact(
        capture.capture_sequence(),
        capture.event_root_sha256(),
        Sha256CommitmentV3::digest_bytes(b"wrong"),
        capture.receipt_sha256(),
    ));
}
