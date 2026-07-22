use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nando_operator_learning::{FramedCborLedger, read_framed_cbor, write_atomic_cbor};
use serde::{Deserialize, Serialize};
use serde_json::json;

const EVENT_SCHEMA: &str = "nando.live-economics-event.v3";
const SNAPSHOT_SCHEMA: &str = "nando.economics-snapshot.v3";
const CHECKPOINT_SCHEMA: &str = "nando.live-economics-checkpoint.v3";
const MINIMUM_M3_INTENTS: usize = 10_000;
const MINIMUM_M3_SECONDS: u64 = 24 * 60 * 60;
const REQUIRED_M3_WINDOWS: usize = 3;
const INPUT_TOKEN_ACCOUNTING_EXACT: bool = true;
const INPUT_TOKEN_ACCOUNTING_SCHEMA: &str = "normalized_request_json_o200k_tokens_v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EconomicsEvent {
    schema: String,
    kind: String,
    intent_sha256: String,
    input_tokens: u64,
    eligible: bool,
    timestamp_unix: u64,
    #[serde(default)]
    stage: String,
    #[serde(default)]
    reason: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct FallbackCounter {
    intents: u64,
    input_tokens: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EconomicsWindow {
    started_at_unix: u64,
    ended_at_unix: u64,
    ordinary_intents: u64,
    ordinary_tokens: u64,
    verified_intents: u64,
    verified_tokens: u64,
    token_share_milli: u64,
    false_accepts: u64,
    parity_failures: u64,
    #[serde(default)]
    pipeline_dropped: u64,
    #[serde(default)]
    unresolved_outcomes: u64,
    pass: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EconomicsCheckpoint {
    schema: String,
    epoch_started_at_unix: u64,
    eligible: BTreeMap<String, u64>,
    #[serde(default)]
    pending_opened_at: BTreeMap<String, u64>,
    ineligible: BTreeSet<String>,
    verified: BTreeMap<String, u64>,
    #[serde(default)]
    fallback_by_intent: BTreeMap<String, String>,
    #[serde(default)]
    fallback_tokens_by_intent: BTreeMap<String, u64>,
    fallback_reasons: BTreeMap<String, FallbackCounter>,
    completed_windows: Vec<EconomicsWindow>,
    dedupe_conflicts: u64,
    false_accepts: u64,
    parity_failures: u64,
    #[serde(default)]
    pipeline_dropped: u64,
    #[serde(default)]
    false_accept_intents: BTreeSet<String>,
    #[serde(default)]
    parity_failure_intents: BTreeSet<String>,
}

pub struct LiveEconomicsLedger {
    journal: FramedCborLedger,
    checkpoint_path: PathBuf,
    snapshot_path: PathBuf,
    epoch_started_at_unix: u64,
    eligible: BTreeMap<String, u64>,
    pending_opened_at: BTreeMap<String, u64>,
    ineligible: BTreeSet<String>,
    verified: BTreeMap<String, u64>,
    fallback_by_intent: BTreeMap<String, String>,
    fallback_tokens_by_intent: BTreeMap<String, u64>,
    fallback_reasons: BTreeMap<String, FallbackCounter>,
    completed_windows: Vec<EconomicsWindow>,
    dedupe_conflicts: u64,
    false_accepts: u64,
    parity_failures: u64,
    pipeline_dropped: u64,
    false_accept_intents: BTreeSet<String>,
    parity_failure_intents: BTreeSet<String>,
    events_since_checkpoint: u64,
    last_checkpoint: Instant,
    last_snapshot: Instant,
}

impl LiveEconomicsLedger {
    pub fn open(state_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(state_dir)
            .map_err(|error| format!("live_economics_dir:{}:{error}", state_dir.display()))?;
        let ledger_dir = state_dir.join("economics-events-v3");
        let checkpoint_path = state_dir.join("economics-live-v3.checkpoint");
        let snapshot_path = state_dir.join("economics-live.json");
        let checkpoint = fs::read(&checkpoint_path)
            .ok()
            .and_then(|bytes| serde_cbor::from_slice::<EconomicsCheckpoint>(&bytes).ok())
            .filter(|checkpoint| checkpoint.schema == CHECKPOINT_SCHEMA);
        let now = unix_now();
        let mut ledger = if let Some(checkpoint) = checkpoint {
            Self {
                journal: FramedCborLedger::open(&ledger_dir, "economics")?,
                checkpoint_path,
                snapshot_path,
                epoch_started_at_unix: checkpoint.epoch_started_at_unix,
                eligible: checkpoint.eligible,
                pending_opened_at: checkpoint.pending_opened_at,
                ineligible: checkpoint.ineligible,
                verified: checkpoint.verified,
                fallback_by_intent: checkpoint.fallback_by_intent,
                fallback_tokens_by_intent: checkpoint.fallback_tokens_by_intent,
                fallback_reasons: checkpoint.fallback_reasons,
                completed_windows: checkpoint.completed_windows,
                dedupe_conflicts: checkpoint.dedupe_conflicts,
                false_accepts: checkpoint.false_accepts,
                parity_failures: checkpoint.parity_failures,
                pipeline_dropped: checkpoint.pipeline_dropped,
                false_accept_intents: checkpoint.false_accept_intents,
                parity_failure_intents: checkpoint.parity_failure_intents,
                events_since_checkpoint: 0,
                last_checkpoint: Instant::now(),
                last_snapshot: Instant::now(),
            }
        } else {
            Self {
                journal: FramedCborLedger::open(&ledger_dir, "economics")?,
                checkpoint_path,
                snapshot_path,
                epoch_started_at_unix: now,
                eligible: BTreeMap::new(),
                pending_opened_at: BTreeMap::new(),
                ineligible: BTreeSet::new(),
                verified: BTreeMap::new(),
                fallback_by_intent: BTreeMap::new(),
                fallback_tokens_by_intent: BTreeMap::new(),
                fallback_reasons: BTreeMap::new(),
                completed_windows: Vec::new(),
                dedupe_conflicts: 0,
                false_accepts: 0,
                parity_failures: 0,
                pipeline_dropped: 0,
                false_accept_intents: BTreeSet::new(),
                parity_failure_intents: BTreeSet::new(),
                events_since_checkpoint: 0,
                last_checkpoint: Instant::now(),
                last_snapshot: Instant::now(),
            }
        };
        for event in read_framed_cbor::<EconomicsEvent>(&ledger_dir, "economics")? {
            if event.schema == EVENT_SCHEMA {
                ledger.apply(event);
            }
        }
        for intent_sha256 in ledger.eligible.keys() {
            if !ledger.verified.contains_key(intent_sha256)
                && !ledger.fallback_by_intent.contains_key(intent_sha256)
            {
                ledger
                    .pending_opened_at
                    .entry(intent_sha256.clone())
                    .or_insert(now);
            }
        }
        let interrupted_intents = ledger.pending_opened_at.keys().cloned().collect::<Vec<_>>();
        for intent_sha256 in interrupted_intents {
            ledger.observe_fallback(
                &intent_sha256,
                "runtime",
                "interrupted_before_terminal_outcome",
            )?;
        }
        ledger.persist_checkpoint()?;
        ledger.journal.compact_after_checkpoint()?;
        ledger.persist_snapshot()?;
        Ok(ledger)
    }

    pub fn observe_request(
        &mut self,
        intent_sha256: &str,
        input_tokens: u64,
        eligible: bool,
    ) -> Result<(), String> {
        if eligible {
            if let Some(previous) = self.eligible.get(intent_sha256) {
                if *previous != input_tokens {
                    self.dedupe_conflicts = self.dedupe_conflicts.saturating_add(1);
                }
                return self.maybe_persist(false);
            }
        } else if self.ineligible.contains(intent_sha256) {
            return self.maybe_persist(false);
        }
        self.record(EconomicsEvent {
            schema: EVENT_SCHEMA.to_owned(),
            kind: "request".to_owned(),
            intent_sha256: intent_sha256.to_owned(),
            input_tokens,
            eligible,
            timestamp_unix: unix_now(),
            stage: String::new(),
            reason: String::new(),
        })
    }

    pub fn observe_verified_accept(
        &mut self,
        intent_sha256: &str,
        input_tokens: u64,
    ) -> Result<(), String> {
        if self.ineligible.contains(intent_sha256) || self.verified.contains_key(intent_sha256) {
            return self.maybe_persist(false);
        }
        self.record(EconomicsEvent {
            schema: EVENT_SCHEMA.to_owned(),
            kind: "verified_accept".to_owned(),
            intent_sha256: intent_sha256.to_owned(),
            input_tokens,
            eligible: true,
            timestamp_unix: unix_now(),
            stage: String::new(),
            reason: String::new(),
        })
    }

    pub fn observe_fallback(
        &mut self,
        intent_sha256: &str,
        stage: &str,
        reason: &str,
    ) -> Result<(), String> {
        let input_tokens = self.eligible.get(intent_sha256).copied().unwrap_or(0);
        self.record(EconomicsEvent {
            schema: EVENT_SCHEMA.to_owned(),
            kind: "fallback".to_owned(),
            intent_sha256: intent_sha256.to_owned(),
            input_tokens,
            eligible: self.eligible.contains_key(intent_sha256),
            timestamp_unix: unix_now(),
            stage: bounded_label(stage),
            reason: bounded_label(reason),
        })
    }

    pub fn observe_parity_failure(&mut self, intent_sha256: &str) -> Result<(), String> {
        self.record(EconomicsEvent {
            schema: EVENT_SCHEMA.to_owned(),
            kind: "parity_failure".to_owned(),
            intent_sha256: intent_sha256.to_owned(),
            input_tokens: self.eligible.get(intent_sha256).copied().unwrap_or(0),
            eligible: true,
            timestamp_unix: unix_now(),
            stage: String::new(),
            reason: String::new(),
        })
    }

    pub fn observe_false_accept(&mut self, intent_sha256: &str) -> Result<(), String> {
        self.record(EconomicsEvent {
            schema: EVENT_SCHEMA.to_owned(),
            kind: "false_accept".to_owned(),
            intent_sha256: intent_sha256.to_owned(),
            input_tokens: self.eligible.get(intent_sha256).copied().unwrap_or(0),
            eligible: true,
            timestamp_unix: unix_now(),
            stage: String::new(),
            reason: String::new(),
        })
    }

    pub fn set_pipeline_dropped(&mut self, dropped: u64) {
        self.pipeline_dropped = self.pipeline_dropped.max(dropped);
    }

    fn record(&mut self, event: EconomicsEvent) -> Result<(), String> {
        let terminal = event.kind != "request";
        self.journal.append(&event)?;
        self.apply(event);
        self.events_since_checkpoint = self.events_since_checkpoint.saturating_add(1);
        if terminal {
            self.roll_window_if_mature();
        }
        self.maybe_persist(true)
    }

    fn apply(&mut self, event: EconomicsEvent) {
        self.epoch_started_at_unix = self.epoch_started_at_unix.min(event.timestamp_unix);
        match event.kind.as_str() {
            "request" if event.eligible => {
                self.eligible
                    .entry(event.intent_sha256.clone())
                    .or_insert(event.input_tokens);
                self.pending_opened_at
                    .entry(event.intent_sha256)
                    .or_insert(event.timestamp_unix);
            }
            "request" => {
                self.ineligible.insert(event.intent_sha256);
            }
            "verified_accept"
                if event.eligible && !self.ineligible.contains(&event.intent_sha256) =>
            {
                let exact_input_tokens = self
                    .eligible
                    .get(&event.intent_sha256)
                    .copied()
                    .unwrap_or(event.input_tokens);
                if let Some(previous) = self.fallback_by_intent.remove(&event.intent_sha256) {
                    let fallback_tokens = self
                        .fallback_tokens_by_intent
                        .remove(&event.intent_sha256)
                        .unwrap_or(exact_input_tokens);
                    decrement_fallback_counter(
                        &mut self.fallback_reasons,
                        &previous,
                        fallback_tokens,
                    );
                }
                self.eligible
                    .entry(event.intent_sha256.clone())
                    .or_insert(exact_input_tokens);
                self.verified
                    .entry(event.intent_sha256.clone())
                    .or_insert(exact_input_tokens);
                self.pending_opened_at.remove(&event.intent_sha256);
            }
            "fallback"
                if event.eligible
                    && !self.verified.contains_key(&event.intent_sha256)
                    && !self.fallback_by_intent.contains_key(&event.intent_sha256) =>
            {
                let key = format!("{}:{}", event.stage, event.reason);
                self.fallback_by_intent
                    .insert(event.intent_sha256.clone(), key.clone());
                self.fallback_tokens_by_intent
                    .insert(event.intent_sha256.clone(), event.input_tokens);
                let counter = self.fallback_reasons.entry(key).or_default();
                counter.intents = counter.intents.saturating_add(1);
                counter.input_tokens = counter.input_tokens.saturating_add(event.input_tokens);
                self.pending_opened_at.remove(&event.intent_sha256);
            }
            "false_accept" => {
                if self.false_accept_intents.insert(event.intent_sha256) {
                    self.false_accepts = self.false_accepts.saturating_add(1);
                }
            }
            "parity_failure" if self.parity_failure_intents.insert(event.intent_sha256) => {
                self.parity_failures = self.parity_failures.saturating_add(1);
            }
            _ => {}
        }
    }

    fn maybe_persist(&mut self, changed: bool) -> Result<(), String> {
        let checkpoint_due = changed
            && (self.events_since_checkpoint >= 64
                || self.last_checkpoint.elapsed() >= Duration::from_secs(5));
        if checkpoint_due {
            self.persist_checkpoint()?;
            self.journal.compact_after_checkpoint()?;
            self.events_since_checkpoint = 0;
            self.last_checkpoint = Instant::now();
        }
        if checkpoint_due || self.last_snapshot.elapsed() >= Duration::from_secs(1) {
            self.persist_snapshot()?;
            self.last_snapshot = Instant::now();
        }
        Ok(())
    }

    fn persist_checkpoint(&self) -> Result<(), String> {
        write_atomic_cbor(
            &self.checkpoint_path,
            &EconomicsCheckpoint {
                schema: CHECKPOINT_SCHEMA.to_owned(),
                epoch_started_at_unix: self.epoch_started_at_unix,
                eligible: self.eligible.clone(),
                pending_opened_at: self.pending_opened_at.clone(),
                ineligible: self.ineligible.clone(),
                verified: self.verified.clone(),
                fallback_by_intent: self.fallback_by_intent.clone(),
                fallback_tokens_by_intent: self.fallback_tokens_by_intent.clone(),
                fallback_reasons: self.fallback_reasons.clone(),
                completed_windows: self.completed_windows.clone(),
                dedupe_conflicts: self.dedupe_conflicts,
                false_accepts: self.false_accepts,
                parity_failures: self.parity_failures,
                pipeline_dropped: self.pipeline_dropped,
                false_accept_intents: self.false_accept_intents.clone(),
                parity_failure_intents: self.parity_failure_intents.clone(),
            },
        )
    }

    fn persist_snapshot(&self) -> Result<(), String> {
        let current = self.current_window(unix_now());
        let eligible_intents = self.eligible.len() as u64;
        let avoided_calls = self.verified.len() as u64;
        let terminal_fallbacks = self.fallback_by_intent.len() as u64;
        let in_flight_local_outcomes = self.pending_opened_at.len() as u64;
        let terminal_intents = avoided_calls.saturating_add(terminal_fallbacks);
        let unresolved_local_outcomes = eligible_intents
            .saturating_sub(terminal_intents)
            .saturating_sub(in_flight_local_outcomes);
        let verification_coverage = if avoided_calls == 0 { 0 } else { 1_000 };
        let hard_gate_pass = self.false_accepts == 0
            && self.parity_failures == 0
            && self.dedupe_conflicts == 0
            && self.pipeline_dropped == 0
            && unresolved_local_outcomes == 0
            && avoided_calls > 0
            && INPUT_TOKEN_ACCOUNTING_EXACT;
        let product_m3_pass = self.completed_windows.len() >= REQUIRED_M3_WINDOWS
            && self.completed_windows.iter().all(|window| window.pass);
        let snapshot = json!({
            "schema": SNAPSHOT_SCHEMA,
            "source": "rust_streaming_economics_v2",
            "generated_at_unix": unix_now(),
            "accounting_epoch_started_at_unix": self.epoch_started_at_unix,
            "dedupe_eligible_client_intents": eligible_intents,
            "terminal_client_intents": terminal_intents,
            "in_flight_local_outcomes": in_flight_local_outcomes,
            "dedupe_ineligible_client_intents": self.ineligible.len(),
            "unique_client_intents": eligible_intents,
            "global_input_tokens": current.ordinary_tokens,
            "input_token_accounting": INPUT_TOKEN_ACCOUNTING_SCHEMA,
            "input_token_accounting_exact": INPUT_TOKEN_ACCOUNTING_EXACT,
            "provider_billed_token_count_available_for_avoided_calls": false,
            "actual_local_accepts": avoided_calls,
            "verified_local_accepts": avoided_calls,
            "avoided_calls": avoided_calls,
            "avoided_input_tokens": current.verified_tokens,
            "call_saving_share_milli": ratio_milli(avoided_calls, terminal_intents),
            "input_token_saving_share_milli": current.token_share_milli,
            "verification_coverage_milli": verification_coverage,
            "false_accepts": self.false_accepts,
            "false_accept_evidence_available": true,
            "false_accept_evidence_reason": "rust_terminal_verifier_events",
            "runtime_parity_mismatches": self.parity_failures,
            "missing_evidence_receipts": 0,
            "unresolved_local_outcomes": unresolved_local_outcomes,
            "dedupe_conflicts": self.dedupe_conflicts,
            "pipeline_dropped": self.pipeline_dropped,
            "hard_gate_pass": hard_gate_pass,
            "product_m1_pass": avoided_calls >= 100 && current.token_share_milli >= 10,
            "m3_current_window_pass": current.pass,
            "product_m3_pass": product_m3_pass,
            "m3_required_consecutive_windows": REQUIRED_M3_WINDOWS,
            "m3_current_window": &current,
            "m3_blockers": m3_blockers(
                INPUT_TOKEN_ACCOUNTING_EXACT,
                self.pipeline_dropped,
                unresolved_local_outcomes,
                self.false_accepts,
                self.parity_failures,
            ),
            "completed_m3_windows": &self.completed_windows,
            "fallback_reasons": &self.fallback_reasons,
            "active_event_segments_bytes": self.journal.status().active_segment_bytes,
            "source_reconciliation": {
                "complete": unresolved_local_outcomes == 0 && self.dedupe_conflicts == 0,
                "blockers": if unresolved_local_outcomes == 0 && self.dedupe_conflicts == 0 {
                    Vec::<String>::new()
                } else {
                    vec!["unresolved_or_conflicting_intents".to_owned()]
                },
            },
            "boundary": "terminal ordinary deduplicated provider requests; in-flight requests remain outside completed economics windows; exact o200k tokenization of recursively key-sorted JSON request payloads; finalized Rust verifier receipts only; counterfactual provider-billed usage for avoided calls is unavailable and is not claimed",
        });
        atomic_json(&self.snapshot_path, &snapshot)
    }

    fn roll_window_if_mature(&mut self) {
        let now = unix_now();
        let terminal_intents = self
            .verified
            .len()
            .saturating_add(self.fallback_by_intent.len());
        if terminal_intents < MINIMUM_M3_INTENTS
            || now.saturating_sub(self.epoch_started_at_unix) < MINIMUM_M3_SECONDS
        {
            return;
        }
        let window = self.current_window(now);
        if window.unresolved_outcomes != 0 {
            return;
        }
        self.completed_windows.push(window);
        if self.completed_windows.len() > REQUIRED_M3_WINDOWS {
            let remove = self.completed_windows.len() - REQUIRED_M3_WINDOWS;
            self.completed_windows.drain(..remove);
        }
        self.epoch_started_at_unix = now;
        self.eligible
            .retain(|intent, _| self.pending_opened_at.contains_key(intent));
        self.ineligible.clear();
        self.verified.clear();
        self.fallback_by_intent.clear();
        self.fallback_tokens_by_intent.clear();
        self.fallback_reasons.clear();
        self.dedupe_conflicts = 0;
        self.false_accepts = 0;
        self.parity_failures = 0;
        self.pipeline_dropped = 0;
        self.false_accept_intents.clear();
        self.parity_failure_intents.clear();
        self.epoch_started_at_unix = self
            .pending_opened_at
            .values()
            .copied()
            .min()
            .unwrap_or(now);
    }

    fn current_window(&self, now: u64) -> EconomicsWindow {
        let ordinary_tokens = self
            .verified
            .values()
            .chain(self.fallback_tokens_by_intent.values())
            .copied()
            .sum::<u64>();
        let verified_tokens = self.verified.values().copied().sum::<u64>();
        let token_share_milli = ratio_milli(verified_tokens, ordinary_tokens);
        let terminal_intents = self
            .verified
            .len()
            .saturating_add(self.fallback_by_intent.len());
        let mature = terminal_intents >= MINIMUM_M3_INTENTS
            && now.saturating_sub(self.epoch_started_at_unix) >= MINIMUM_M3_SECONDS;
        let unresolved_outcomes = u64::try_from(self.eligible.len())
            .unwrap_or(u64::MAX)
            .saturating_sub(u64::try_from(terminal_intents).unwrap_or(u64::MAX))
            .saturating_sub(u64::try_from(self.pending_opened_at.len()).unwrap_or(u64::MAX));
        EconomicsWindow {
            started_at_unix: self.epoch_started_at_unix,
            ended_at_unix: now,
            ordinary_intents: terminal_intents as u64,
            ordinary_tokens,
            verified_intents: self.verified.len() as u64,
            verified_tokens,
            token_share_milli,
            false_accepts: self.false_accepts,
            parity_failures: self.parity_failures,
            pipeline_dropped: self.pipeline_dropped,
            unresolved_outcomes,
            pass: mature
                && token_share_milli >= 500
                && INPUT_TOKEN_ACCOUNTING_EXACT
                && self.false_accepts == 0
                && self.parity_failures == 0
                && self.pipeline_dropped == 0
                && unresolved_outcomes == 0
                && self.dedupe_conflicts == 0,
        }
    }
}

fn m3_blockers(
    token_accounting_exact: bool,
    pipeline_dropped: u64,
    unresolved_local_outcomes: u64,
    false_accepts: u64,
    parity_failures: u64,
) -> Vec<&'static str> {
    let mut blockers = Vec::new();
    if !token_accounting_exact {
        blockers.push("input_token_accounting_not_exact");
    }
    if pipeline_dropped != 0 {
        blockers.push("economics_pipeline_dropped");
    }
    if unresolved_local_outcomes != 0 {
        blockers.push("unresolved_local_outcomes");
    }
    if false_accepts != 0 {
        blockers.push("false_accepts_nonzero");
    }
    if parity_failures != 0 {
        blockers.push("runtime_parity_failures_nonzero");
    }
    blockers
}

fn atomic_json(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    let temporary = path.with_extension("tmp");
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    {
        let mut file = fs::File::create(&temporary)
            .map_err(|error| format!("live_economics_create:{}:{error}", temporary.display()))?;
        use std::io::Write as _;
        file.write_all(&bytes)
            .map_err(|error| format!("live_economics_write:{}:{error}", temporary.display()))?;
        file.sync_all()
            .map_err(|error| format!("live_economics_sync:{}:{error}", temporary.display()))?;
    }
    fs::rename(&temporary, path)
        .map_err(|error| format!("live_economics_rename:{}:{error}", path.display()))?;
    if let Some(parent) = path.parent() {
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("live_economics_parent_sync:{}:{error}", parent.display()))?;
    }
    Ok(())
}

fn decrement_fallback_counter(
    counters: &mut BTreeMap<String, FallbackCounter>,
    key: &str,
    input_tokens: u64,
) {
    let remove = if let Some(counter) = counters.get_mut(key) {
        counter.intents = counter.intents.saturating_sub(1);
        counter.input_tokens = counter.input_tokens.saturating_sub(input_tokens);
        counter.intents == 0
    } else {
        false
    };
    if remove {
        counters.remove(key);
    }
}

fn bounded_label(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
        .take(96)
        .collect()
}

fn ratio_milli(numerator: u64, denominator: u64) -> u64 {
    numerator
        .saturating_mul(1_000)
        .checked_div(denominator)
        .unwrap_or(0)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ineligible_probe_never_enters_commercial_numerator_or_denominator() {
        let root = std::env::temp_dir().join(format!(
            "nando-live-economics-{}-{}",
            std::process::id(),
            unix_now()
        ));
        let mut ledger = LiveEconomicsLedger::open(&root).expect("open ledger");
        ledger
            .observe_request("controlled-probe", 65, false)
            .expect("request");
        ledger
            .observe_verified_accept("controlled-probe", 65)
            .expect("accept");
        let snapshot: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join("economics-live.json")).expect("read snapshot"),
        )
        .expect("snapshot");
        assert_eq!(snapshot["global_input_tokens"], 0);
        assert_eq!(snapshot["verified_local_accepts"], 0);
        let replayed = LiveEconomicsLedger::open(&root).expect("replay");
        assert!(replayed.eligible.is_empty());
        assert!(replayed.verified.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restart_accounts_interrupted_request_as_fallback_without_savings() {
        let root = std::env::temp_dir().join(format!(
            "nando-live-economics-interrupted-{}-{}",
            std::process::id(),
            unix_now()
        ));
        let mut ledger = LiveEconomicsLedger::open(&root).expect("open ledger");
        ledger
            .observe_request("interrupted-intent", 1_024, true)
            .expect("request");
        drop(ledger);

        let replayed = LiveEconomicsLedger::open(&root).expect("replay");
        assert_eq!(replayed.eligible.get("interrupted-intent"), Some(&1_024));
        assert!(replayed.verified.is_empty());
        assert_eq!(
            replayed.fallback_by_intent.get("interrupted-intent"),
            Some(&"runtime:interrupted_before_terminal_outcome".to_owned())
        );
        assert!(replayed.pending_opened_at.is_empty());
        let snapshot: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join("economics-live.json")).expect("read snapshot"),
        )
        .expect("snapshot");
        assert_eq!(snapshot["avoided_input_tokens"], 0);
        assert_eq!(snapshot["unresolved_local_outcomes"], 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn live_in_flight_request_stays_outside_completed_denominator() {
        let root = std::env::temp_dir().join(format!(
            "nando-live-economics-in-flight-{}-{}",
            std::process::id(),
            unix_now()
        ));
        let mut ledger = LiveEconomicsLedger::open(&root).expect("open ledger");
        ledger
            .observe_request("in-flight-intent", 2_048, true)
            .expect("request");
        ledger.persist_snapshot().expect("snapshot");
        let snapshot: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join("economics-live.json")).expect("read snapshot"),
        )
        .expect("snapshot");
        assert_eq!(snapshot["in_flight_local_outcomes"], 1);
        assert_eq!(snapshot["terminal_client_intents"], 0);
        assert_eq!(snapshot["global_input_tokens"], 0);
        assert_eq!(snapshot["unresolved_local_outcomes"], 0);
        assert_eq!(snapshot["source_reconciliation"]["complete"], true);
        let _ = fs::remove_dir_all(root);
    }
}
