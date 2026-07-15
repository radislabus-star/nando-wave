use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::thread;

use bytes::Bytes;
use serde::Serialize;
use serde_json::Value;
use tiktoken_rs::CoreBPE;

use crate::live_economics::LiveEconomicsLedger;

const ECONOMICS_QUEUE_CAPACITY: usize = 256;
const ECONOMICS_QUEUE_MAX_BODY_BYTES: u64 = 256 * 1024 * 1024;

enum EconomicsCommand {
    Request {
        intent_sha256: String,
        request_body: Bytes,
        eligible: bool,
    },
    Fallback {
        intent_sha256: String,
        stage: String,
        reason: String,
    },
    Verified {
        intent_sha256: String,
        input_tokens: u64,
    },
    ParityFailure {
        intent_sha256: String,
    },
    FalseAccept {
        intent_sha256: String,
    },
}

#[derive(Default)]
struct EconomicsWorkerCounters {
    ready: AtomicBool,
    enqueued: AtomicU64,
    processed: AtomicU64,
    failed: AtomicU64,
    dropped: AtomicU64,
    queued_body_bytes: AtomicU64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct EconomicsWorkerStatus {
    pub ready: bool,
    pub enqueued: u64,
    pub processed: u64,
    pub failed: u64,
    pub dropped: u64,
    pub queue_backlog_estimate: u64,
    pub queued_body_bytes: u64,
}

#[derive(Clone)]
pub struct EconomicsWorkerHandle {
    sender: SyncSender<EconomicsCommand>,
    counters: Arc<EconomicsWorkerCounters>,
}

impl EconomicsWorkerHandle {
    pub fn observe_request(
        &self,
        intent_sha256: String,
        request_body: Bytes,
        eligible: bool,
    ) -> Result<(), String> {
        self.submit(EconomicsCommand::Request {
            intent_sha256,
            request_body,
            eligible,
        })
    }

    pub fn observe_fallback(
        &self,
        intent_sha256: String,
        stage: String,
        reason: String,
    ) -> Result<(), String> {
        self.submit(EconomicsCommand::Fallback {
            intent_sha256,
            stage,
            reason,
        })
    }

    pub fn observe_verified(&self, intent_sha256: String, input_tokens: u64) -> Result<(), String> {
        self.submit(EconomicsCommand::Verified {
            intent_sha256,
            input_tokens,
        })
    }

    pub fn observe_parity_failure(&self, intent_sha256: String) -> Result<(), String> {
        self.submit(EconomicsCommand::ParityFailure { intent_sha256 })
    }

    pub fn observe_false_accept(&self, intent_sha256: String) -> Result<(), String> {
        self.submit(EconomicsCommand::FalseAccept { intent_sha256 })
    }

    #[must_use]
    pub fn status(&self) -> EconomicsWorkerStatus {
        let enqueued = self.counters.enqueued.load(Ordering::Relaxed);
        let processed = self.counters.processed.load(Ordering::Relaxed);
        let failed = self.counters.failed.load(Ordering::Relaxed);
        EconomicsWorkerStatus {
            ready: self.counters.ready.load(Ordering::Acquire),
            enqueued,
            processed,
            failed,
            dropped: self.counters.dropped.load(Ordering::Relaxed),
            queue_backlog_estimate: enqueued.saturating_sub(processed.saturating_add(failed)),
            queued_body_bytes: self.counters.queued_body_bytes.load(Ordering::Relaxed),
        }
    }

    fn submit(&self, command: EconomicsCommand) -> Result<(), String> {
        let body_bytes = command_body_bytes(&command);
        if body_bytes > 0
            && self
                .counters
                .queued_body_bytes
                .fetch_update(Ordering::AcqRel, Ordering::Relaxed, |current| {
                    current
                        .checked_add(body_bytes)
                        .filter(|next| *next <= ECONOMICS_QUEUE_MAX_BODY_BYTES)
                })
                .is_err()
        {
            self.counters.dropped.fetch_add(1, Ordering::Relaxed);
            return Err("economics_worker_body_budget_exceeded".to_owned());
        }
        match self.sender.try_send(command) {
            Ok(()) => {
                self.counters.enqueued.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(mpsc::TrySendError::Full(_)) => {
                release_body_bytes(&self.counters, body_bytes);
                self.counters.dropped.fetch_add(1, Ordering::Relaxed);
                Err("economics_worker_queue_full".to_owned())
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                release_body_bytes(&self.counters, body_bytes);
                Err("economics_worker_stopped".to_owned())
            }
        }
    }
}

pub fn spawn_economics_worker(state_dir: PathBuf) -> Result<EconomicsWorkerHandle, String> {
    let (sender, receiver) = mpsc::sync_channel(ECONOMICS_QUEUE_CAPACITY);
    let counters = Arc::new(EconomicsWorkerCounters::default());
    let thread_counters = counters.clone();
    thread::Builder::new()
        .name("nando-live-economics".to_owned())
        .spawn(move || {
            let tokenizer = match tiktoken_rs::o200k_base() {
                Ok(tokenizer) => tokenizer,
                Err(error) => {
                    thread_counters.failed.fetch_add(1, Ordering::Relaxed);
                    eprintln!("nando-live-economics tokenizer: {error}");
                    return;
                }
            };
            let mut ledger = match LiveEconomicsLedger::open(&state_dir) {
                Ok(ledger) => ledger,
                Err(error) => {
                    thread_counters.failed.fetch_add(1, Ordering::Relaxed);
                    eprintln!("nando-live-economics open: {error}");
                    return;
                }
            };
            thread_counters.ready.store(true, Ordering::Release);
            while let Ok(command) = receiver.recv() {
                release_body_bytes(&thread_counters, command_body_bytes(&command));
                ledger.set_pipeline_dropped(thread_counters.dropped.load(Ordering::Relaxed));
                let result = match command {
                    EconomicsCommand::Request {
                        intent_sha256,
                        request_body,
                        eligible,
                    } => canonical_request_tokens(&tokenizer, &request_body).and_then(|tokens| {
                        ledger.observe_request(&intent_sha256, tokens, eligible)
                    }),
                    EconomicsCommand::Fallback {
                        intent_sha256,
                        stage,
                        reason,
                    } => ledger.observe_fallback(&intent_sha256, &stage, &reason),
                    EconomicsCommand::Verified {
                        intent_sha256,
                        input_tokens,
                    } => ledger.observe_verified_accept(&intent_sha256, input_tokens),
                    EconomicsCommand::ParityFailure { intent_sha256 } => {
                        ledger.observe_parity_failure(&intent_sha256)
                    }
                    EconomicsCommand::FalseAccept { intent_sha256 } => {
                        ledger.observe_false_accept(&intent_sha256)
                    }
                };
                match result {
                    Ok(()) => {
                        thread_counters.processed.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(error) => {
                        thread_counters.failed.fetch_add(1, Ordering::Relaxed);
                        eprintln!("nando-live-economics event: {error}");
                    }
                }
            }
            thread_counters.ready.store(false, Ordering::Release);
        })
        .map_err(|error| format!("economics_worker_spawn:{error}"))?;
    Ok(EconomicsWorkerHandle { sender, counters })
}

fn command_body_bytes(command: &EconomicsCommand) -> u64 {
    match command {
        EconomicsCommand::Request { request_body, .. } => {
            u64::try_from(request_body.len()).unwrap_or(u64::MAX)
        }
        EconomicsCommand::Fallback { .. }
        | EconomicsCommand::Verified { .. }
        | EconomicsCommand::ParityFailure { .. }
        | EconomicsCommand::FalseAccept { .. } => 0,
    }
}

fn release_body_bytes(counters: &EconomicsWorkerCounters, body_bytes: u64) {
    if body_bytes != 0 {
        let _ = counters.queued_body_bytes.fetch_update(
            Ordering::AcqRel,
            Ordering::Relaxed,
            |current| Some(current.saturating_sub(body_bytes)),
        );
    }
}

fn canonical_request_tokens(tokenizer: &CoreBPE, body: &[u8]) -> Result<u64, String> {
    let value = serde_json::from_slice::<Value>(body)
        .map_err(|error| format!("economics_request_json:{error}"))?;
    let normalized = normalized_json(value);
    let text = serde_json::to_string(&normalized)
        .map_err(|error| format!("economics_request_normalize:{error}"))?;
    u64::try_from(tokenizer.encode_ordinary(&text).len())
        .map_err(|_| "economics_request_token_count_overflow".to_owned())
}

fn normalized_json(value: Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.into_iter().map(normalized_json).collect()),
        Value::Object(values) => {
            let mut entries = values.into_iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, normalized_json(value)))
                    .collect(),
            )
        }
        scalar => scalar,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_accounting_is_stable_across_json_key_order_and_whitespace() {
        let tokenizer = tiktoken_rs::o200k_base().expect("o200k tokenizer");
        let left = br#"{"model":"gpt-5","input":[{"role":"user","content":"count this"}]}"#;
        let right = br#"{
            "input": [ { "content": "count this", "role": "user" } ],
            "model": "gpt-5"
        }"#;
        assert_eq!(
            canonical_request_tokens(&tokenizer, left),
            canonical_request_tokens(&tokenizer, right)
        );
    }
}
