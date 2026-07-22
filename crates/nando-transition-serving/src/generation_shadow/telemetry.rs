use std::sync::RwLock;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use super::{
    GenerationShadowEvaluationReceiptV3, GenerationShadowEvaluationVerdictV3,
    GenerationShadowStatusV3, GenerationShadowSubmitVerdictV3,
};

const PHASE_DISABLED: u8 = 0;
const PHASE_EMPTY: u8 = 1;
const PHASE_LOADING: u8 = 2;
const PHASE_READY: u8 = 3;
const PHASE_BLOCKED: u8 = 4;

#[derive(Clone, Debug, Default)]
struct LoadedIdentityV3 {
    generation_sequence: u64,
    generation_id_sha256: String,
    publish_sequence: u64,
    checkpoint_sha256: String,
    capture_index_sha256: String,
    last_error: String,
}

pub(super) struct GenerationShadowTelemetryV3 {
    enabled: bool,
    phase: AtomicU8,
    identity: RwLock<LoadedIdentityV3>,
    load_attempts: AtomicU64,
    load_successes: AtomicU64,
    load_failures: AtomicU64,
    submitted: AtomicU64,
    enqueued: AtomicU64,
    censored: AtomicU64,
    evaluated: AtomicU64,
    verified: AtomicU64,
    runtime_abstains: AtomicU64,
    runtime_rejects: AtomicU64,
    verifier_abstains: AtomicU64,
    verifier_rejects: AtomicU64,
    parity_mismatches: AtomicU64,
}

impl GenerationShadowTelemetryV3 {
    pub(super) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            phase: AtomicU8::new(if enabled { PHASE_EMPTY } else { PHASE_DISABLED }),
            identity: RwLock::new(LoadedIdentityV3::default()),
            load_attempts: AtomicU64::new(0),
            load_successes: AtomicU64::new(0),
            load_failures: AtomicU64::new(0),
            submitted: AtomicU64::new(0),
            enqueued: AtomicU64::new(0),
            censored: AtomicU64::new(0),
            evaluated: AtomicU64::new(0),
            verified: AtomicU64::new(0),
            runtime_abstains: AtomicU64::new(0),
            runtime_rejects: AtomicU64::new(0),
            verifier_abstains: AtomicU64::new(0),
            verifier_rejects: AtomicU64::new(0),
            parity_mismatches: AtomicU64::new(0),
        }
    }

    pub(super) fn loading(&self) {
        self.load_attempts.fetch_add(1, Ordering::Relaxed);
        self.phase.store(PHASE_LOADING, Ordering::Release);
    }

    pub(super) fn ready(
        &self,
        generation_sequence: u64,
        generation_id_sha256: &str,
        publish_sequence: u64,
        checkpoint_sha256: &str,
        capture_index_sha256: &str,
    ) {
        if let Ok(mut identity) = self.identity.write() {
            *identity = LoadedIdentityV3 {
                generation_sequence,
                generation_id_sha256: generation_id_sha256.to_owned(),
                publish_sequence,
                checkpoint_sha256: checkpoint_sha256.to_owned(),
                capture_index_sha256: capture_index_sha256.to_owned(),
                last_error: String::new(),
            };
        }
        self.load_successes.fetch_add(1, Ordering::Relaxed);
        self.phase.store(PHASE_READY, Ordering::Release);
    }

    pub(super) fn empty(&self) {
        if let Ok(mut identity) = self.identity.write() {
            *identity = LoadedIdentityV3::default();
        }
        self.phase.store(PHASE_EMPTY, Ordering::Release);
    }

    pub(super) fn blocked(&self, error: &str) {
        if let Ok(mut identity) = self.identity.write() {
            identity.last_error = error.to_owned();
        }
        self.load_failures.fetch_add(1, Ordering::Relaxed);
        self.phase.store(PHASE_BLOCKED, Ordering::Release);
    }

    pub(super) fn observe_submit(&self, verdict: GenerationShadowSubmitVerdictV3) {
        self.submitted.fetch_add(1, Ordering::Relaxed);
        match verdict {
            GenerationShadowSubmitVerdictV3::Enqueued => {
                self.enqueued.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                self.censored.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub(super) fn observe_evaluation(&self, receipt: &GenerationShadowEvaluationReceiptV3) {
        self.evaluated.fetch_add(1, Ordering::Relaxed);
        match receipt.verdict {
            GenerationShadowEvaluationVerdictV3::Verified => {
                self.verified.fetch_add(1, Ordering::Relaxed);
            }
            GenerationShadowEvaluationVerdictV3::RuntimeAbstain
            | GenerationShadowEvaluationVerdictV3::InvalidRequest => {
                self.runtime_abstains.fetch_add(1, Ordering::Relaxed);
            }
            GenerationShadowEvaluationVerdictV3::RuntimeReject => {
                self.runtime_rejects.fetch_add(1, Ordering::Relaxed);
            }
            GenerationShadowEvaluationVerdictV3::VerifierAbstain => {
                self.verifier_abstains.fetch_add(1, Ordering::Relaxed);
            }
            GenerationShadowEvaluationVerdictV3::VerifierReject => {
                self.verifier_rejects.fetch_add(1, Ordering::Relaxed);
            }
        }
        if receipt.parity_mismatch {
            self.parity_mismatches.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub(super) fn snapshot(&self) -> GenerationShadowStatusV3 {
        let identity = self
            .identity
            .read()
            .map(|identity| identity.clone())
            .unwrap_or_default();
        GenerationShadowStatusV3 {
            enabled: self.enabled,
            phase: phase_name(self.phase.load(Ordering::Acquire)).to_owned(),
            generation_sequence: identity.generation_sequence,
            generation_id_sha256: identity.generation_id_sha256,
            publish_sequence: identity.publish_sequence,
            checkpoint_sha256: identity.checkpoint_sha256,
            capture_index_sha256: identity.capture_index_sha256,
            load_attempts: self.load_attempts.load(Ordering::Relaxed),
            load_successes: self.load_successes.load(Ordering::Relaxed),
            load_failures: self.load_failures.load(Ordering::Relaxed),
            submitted: self.submitted.load(Ordering::Relaxed),
            enqueued: self.enqueued.load(Ordering::Relaxed),
            censored: self.censored.load(Ordering::Relaxed),
            evaluated: self.evaluated.load(Ordering::Relaxed),
            verified: self.verified.load(Ordering::Relaxed),
            runtime_abstains: self.runtime_abstains.load(Ordering::Relaxed),
            runtime_rejects: self.runtime_rejects.load(Ordering::Relaxed),
            verifier_abstains: self.verifier_abstains.load(Ordering::Relaxed),
            verifier_rejects: self.verifier_rejects.load(Ordering::Relaxed),
            false_accepts: 0,
            parity_mismatches: self.parity_mismatches.load(Ordering::Relaxed),
            local_accepts: 0,
            last_error: identity.last_error,
            execution_authority: false,
        }
    }
}

const fn phase_name(phase: u8) -> &'static str {
    match phase {
        PHASE_EMPTY => "empty",
        PHASE_LOADING => "loading",
        PHASE_READY => "ready_shadow",
        PHASE_BLOCKED => "blocked",
        _ => "disabled",
    }
}
