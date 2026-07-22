use std::{
    path::{Path, PathBuf},
    sync::{Arc, mpsc::Receiver},
    thread,
    time::Duration,
};

use nando_operator_learning::{
    GenerationShadowReceiptInputV3, GenerationShadowReceiptLedgerV3,
    GenerationShadowTerminalOutcomeV3, ProviderCaptureIndexV3,
};
use nando_operator_persistence::{GenerationShadowReceiptStoreV3, ProviderCaptureStoreV3};
use nando_operator_runtime::TrafficShadowReceiptV3;

use super::telemetry::GenerationShadowTelemetryV3;
use super::{
    GenerationShadowEvaluationVerdictV3, GenerationShadowRequestV3, GenerationShadowSnapshotV3,
    evaluation::{
        GenerationShadowEvaluationV3, evaluate_generation_shadow_request_with_evidence_v3,
    },
};

const CAPTURE_JOIN_RETRIES_V3: usize = 25;
const CAPTURE_JOIN_RETRY_DELAY_V3: Duration = Duration::from_millis(2);

pub(super) struct GenerationShadowWorkItemV3 {
    generation: Arc<GenerationShadowSnapshotV3>,
    request: GenerationShadowRequestV3,
}

impl GenerationShadowWorkItemV3 {
    pub(super) fn new(
        generation: Arc<GenerationShadowSnapshotV3>,
        request: GenerationShadowRequestV3,
    ) -> Self {
        Self {
            generation,
            request,
        }
    }
}

pub(super) fn run_generation_shadow_worker_v3(
    receiver: Receiver<GenerationShadowWorkItemV3>,
    telemetry: Arc<GenerationShadowTelemetryV3>,
    provider_capture_store_path: PathBuf,
    receipt_store_path: PathBuf,
) {
    let provider_capture_store = match ProviderCaptureStoreV3::open(provider_capture_store_path) {
        Ok(store) => store,
        Err(error) => {
            telemetry.blocked(&format!("generation_shadow_capture_store:{error:?}"));
            return;
        }
    };
    let mut durable = None;
    while let Ok(item) = receiver.recv() {
        let evaluation =
            evaluate_generation_shadow_request_with_evidence_v3(&item.generation, &item.request);
        telemetry.observe_evaluation(evaluation.receipt());
        match persist_evaluation(
            &provider_capture_store,
            &receipt_store_path,
            &mut durable,
            &item,
            &evaluation,
        ) {
            Ok(Some(ledger_sha256)) => telemetry.observe_durable_append(&ledger_sha256),
            Ok(None) => telemetry.observe_durable_censored(),
            Err(error) => {
                telemetry.blocked(&error);
                return;
            }
        }
    }
}

struct ActiveDurableLedgerV3 {
    generation_id_sha256: String,
    checkpoint_sha256: String,
    store: GenerationShadowReceiptStoreV3,
    ledger: GenerationShadowReceiptLedgerV3,
}

fn persist_evaluation(
    capture_store: &ProviderCaptureStoreV3,
    receipt_store_root: &Path,
    active: &mut Option<ActiveDurableLedgerV3>,
    item: &GenerationShadowWorkItemV3,
    evaluation: &GenerationShadowEvaluationV3,
) -> Result<Option<String>, String> {
    let Some(capture_receipt) = item.request.capture_receipt() else {
        return Ok(None);
    };
    let Some(traffic_receipt) = evaluation.traffic_receipt() else {
        return Ok(None);
    };
    validate_traffic_binding(
        &item.generation,
        capture_receipt,
        traffic_receipt,
        evaluation,
    )?;
    let Some(capture_index) = wait_for_durable_capture(capture_store, capture_receipt)? else {
        return Ok(None);
    };
    let manifest = item.generation.checkpoint().generation().manifest();
    let checkpoint_sha256 = item.generation.checkpoint().checkpoint_sha256();
    ensure_active_ledger(
        active,
        receipt_store_root,
        manifest.generation_id_sha256(),
        item.generation.checkpoint().publish_sequence(),
        checkpoint_sha256,
    )?;
    let durable = active
        .as_mut()
        .ok_or_else(|| "generation_shadow_ledger_unavailable".to_owned())?;
    let mut next = durable.ledger.clone();
    next.append(
        &capture_index,
        GenerationShadowReceiptInputV3 {
            capture_receipt,
            traffic_receipt_sha256: traffic_receipt.receipt_sha256(),
            traffic_generation_sequence: traffic_receipt.generation_sequence(),
            traffic_generation_id_sha256: traffic_receipt.generation_root_sha256(),
            traffic_index_sha256: traffic_receipt.index_sha256(),
            traffic_request_sha256: traffic_receipt.request_sha256(),
            traffic_verdict_code: traffic_receipt.verdict() as u8,
            traffic_phase_report_sha256: traffic_receipt.phase_report_sha256(),
            traffic_operator_receipt_sha256: traffic_receipt.operator_shadow_receipt_sha256(),
            f6_receipt: evaluation.verifier_receipt(),
            outcome: durable_outcome(evaluation),
            parity_mismatch: evaluation.receipt().parity_mismatch,
        },
    )
    .map_err(|error| format!("generation_shadow_ledger_append:{error:?}"))?;
    durable
        .store
        .publish(&next)
        .map_err(|error| format!("generation_shadow_ledger_publish:{error:?}"))?;
    durable.ledger = next;
    Ok(Some(durable.ledger.ledger_sha256().to_owned()))
}

fn validate_traffic_binding(
    generation: &GenerationShadowSnapshotV3,
    capture: &nando_operator_learning::ProviderRequestCaptureReceiptV3,
    traffic: &TrafficShadowReceiptV3,
    evaluation: &GenerationShadowEvaluationV3,
) -> Result<(), String> {
    let manifest = generation.checkpoint().generation().manifest();
    if traffic.request_sha256() != capture.request_root_sha256().to_hex()
        || traffic.window_row_sha256() != capture.event_root_sha256().to_hex()
        || traffic.generation_sequence() != manifest.sequence()
        || traffic.generation_root_sha256() != manifest.generation_id_sha256()
        || traffic.receipt_sha256() != evaluation.receipt().traffic_receipt_sha256
        || traffic.raw_payloads_persisted() != 0
        || traffic.local_accepts() != 0
        || traffic.execution_authority()
    {
        return Err("generation_shadow_traffic_binding_mismatch".to_owned());
    }
    Ok(())
}

fn wait_for_durable_capture(
    store: &ProviderCaptureStoreV3,
    capture: &nando_operator_learning::ProviderRequestCaptureReceiptV3,
) -> Result<Option<ProviderCaptureIndexV3>, String> {
    for attempt in 0..CAPTURE_JOIN_RETRIES_V3 {
        let restored = store
            .restore()
            .map_err(|error| format!("generation_shadow_capture_restore:{error:?}"))?;
        if let Some(index) = restored.index()
            && index.contains_exact(
                capture.capture_sequence(),
                capture.event_root_sha256(),
                capture.request_root_sha256(),
                capture.receipt_sha256(),
            )
        {
            return Ok(Some(index.clone()));
        }
        if attempt + 1 < CAPTURE_JOIN_RETRIES_V3 {
            thread::sleep(CAPTURE_JOIN_RETRY_DELAY_V3);
        }
    }
    Ok(None)
}

fn ensure_active_ledger(
    active: &mut Option<ActiveDurableLedgerV3>,
    root: &Path,
    generation_id_sha256: &str,
    generation_publish_sequence: u64,
    checkpoint_sha256: &str,
) -> Result<(), String> {
    if active.as_ref().is_some_and(|current| {
        current.generation_id_sha256 == generation_id_sha256
            && current.checkpoint_sha256 == checkpoint_sha256
    }) {
        return Ok(());
    }
    let store = GenerationShadowReceiptStoreV3::open(
        root.join(generation_id_sha256).join(checkpoint_sha256),
    )
    .map_err(|error| format!("generation_shadow_receipt_store:{error:?}"))?;
    let restored = store
        .restore()
        .map_err(|error| format!("generation_shadow_receipt_restore:{error:?}"))?;
    let ledger = match restored.ledger() {
        Some(ledger) => {
            if ledger.generation_id_sha256() != generation_id_sha256
                || ledger.generation_publish_sequence() != generation_publish_sequence
                || ledger.generation_checkpoint_sha256() != checkpoint_sha256
            {
                return Err("generation_shadow_receipt_foreign_generation".to_owned());
            }
            ledger.clone()
        }
        None => GenerationShadowReceiptLedgerV3::new(
            generation_id_sha256.to_owned(),
            generation_publish_sequence,
            checkpoint_sha256.to_owned(),
        )
        .map_err(|error| format!("generation_shadow_receipt_new:{error:?}"))?,
    };
    *active = Some(ActiveDurableLedgerV3 {
        generation_id_sha256: generation_id_sha256.to_owned(),
        checkpoint_sha256: checkpoint_sha256.to_owned(),
        store,
        ledger,
    });
    Ok(())
}

const fn durable_outcome(
    evaluation: &GenerationShadowEvaluationV3,
) -> GenerationShadowTerminalOutcomeV3 {
    match evaluation.receipt().verdict {
        GenerationShadowEvaluationVerdictV3::Verified => {
            GenerationShadowTerminalOutcomeV3::VerifiedPass
        }
        GenerationShadowEvaluationVerdictV3::RuntimeAbstain => {
            GenerationShadowTerminalOutcomeV3::RuntimeAbstain
        }
        GenerationShadowEvaluationVerdictV3::RuntimeReject => {
            GenerationShadowTerminalOutcomeV3::RuntimeReject
        }
        GenerationShadowEvaluationVerdictV3::VerifierAbstain => {
            GenerationShadowTerminalOutcomeV3::VerifierAbstain
        }
        GenerationShadowEvaluationVerdictV3::VerifierReject
            if evaluation.verifier_receipt().is_some() =>
        {
            GenerationShadowTerminalOutcomeV3::VerifierReject
        }
        GenerationShadowEvaluationVerdictV3::VerifierReject
        | GenerationShadowEvaluationVerdictV3::InvalidRequest => {
            GenerationShadowTerminalOutcomeV3::Censored
        }
    }
}
