use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use nando_operator_learning::{FramedCborLedger, read_framed_cbor, write_atomic_cbor};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const EVENT_SCHEMA: &str = "nando.live-economics-event.v4";
const SNAPSHOT_SCHEMA: &str = "nando.economics-snapshot.v4";
const CHECKPOINT_SCHEMA: &str = "nando.live-economics-checkpoint.v4";
const PACKAGE_COMPLETION_SCHEMA: &str = "nando.package-cpu-completion-receipt.v1";
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
    #[serde(default)]
    package_id: String,
    #[serde(default)]
    verification_receipt_root_sha256: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct FallbackCounter {
    intents: u64,
    input_tokens: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct PackageVerifiedEconomics {
    ordinary_accepts: u64,
    exact_input_tokens: u64,
    #[serde(default)]
    first_accept_timestamp_unix: u64,
    #[serde(default)]
    first_exact_input_tokens: u64,
    #[serde(default)]
    first_receipt_root_sha256: String,
    last_accept_timestamp_unix: u64,
    last_receipt_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct VerifiedReceiptMetadata {
    package_id: String,
    exact_input_tokens: u64,
    accepted_at_unix: u64,
    receipt_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PackageCpuCompletionReceiptV1 {
    pub schema: String,
    pub completion_root_sha256: String,
    pub package_id: String,
    pub intent_sha256: String,
    pub exact_input_tokens: u64,
    pub accepted_at_unix: u64,
    pub verification_receipt_root_sha256: String,
}

impl PackageCpuCompletionReceiptV1 {
    fn seal(intent_sha256: &str, metadata: &VerifiedReceiptMetadata) -> Result<Self, String> {
        let mut receipt = Self {
            schema: PACKAGE_COMPLETION_SCHEMA.to_owned(),
            completion_root_sha256: String::new(),
            package_id: metadata.package_id.clone(),
            intent_sha256: intent_sha256.to_owned(),
            exact_input_tokens: metadata.exact_input_tokens,
            accepted_at_unix: metadata.accepted_at_unix,
            verification_receipt_root_sha256: metadata.receipt_root_sha256.clone(),
        };
        receipt.completion_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != PACKAGE_COMPLETION_SCHEMA
            || !valid_nonzero_sha256(&self.completion_root_sha256)
            || !valid_package_id(&self.package_id)
            || !valid_sha256_hex(&self.intent_sha256)
            || self.exact_input_tokens == 0
            || self.accepted_at_unix == 0
            || !valid_sha256_hex(&self.verification_receipt_root_sha256)
            || self.completion_root_sha256 != self.expected_root()?
        {
            return Err("package_cpu_completion_receipt_invalid".to_owned());
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, String> {
        canonical_json_sha256(&(
            PACKAGE_COMPLETION_SCHEMA,
            self.package_id.as_str(),
            self.intent_sha256.as_str(),
            self.exact_input_tokens,
            self.accepted_at_unix,
            self.verification_receipt_root_sha256.as_str(),
        ))
        .map_err(str::to_owned)
    }
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
    #[serde(default)]
    prior_epoch_schema: String,
    #[serde(default)]
    prior_epoch_ordinary_tokens: u64,
    #[serde(default)]
    prior_epoch_verified_tokens: u64,
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
    #[serde(default)]
    verified_receipt_by_intent: BTreeMap<String, VerifiedReceiptMetadata>,
    #[serde(default)]
    verified_by_package: BTreeMap<String, PackageVerifiedEconomics>,
}

pub struct LiveEconomicsLedger {
    journal: FramedCborLedger,
    completion_journal: FramedCborLedger,
    checkpoint_path: PathBuf,
    snapshot_path: PathBuf,
    prior_epoch_schema: String,
    prior_epoch_ordinary_tokens: u64,
    prior_epoch_verified_tokens: u64,
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
    verified_receipt_by_intent: BTreeMap<String, VerifiedReceiptMetadata>,
    verified_by_package: BTreeMap<String, PackageVerifiedEconomics>,
    completion_roots_by_key: BTreeMap<(String, String), String>,
    events_since_checkpoint: u64,
    last_checkpoint: Instant,
    last_snapshot: Instant,
}

impl LiveEconomicsLedger {
    pub fn open(state_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(state_dir)
            .map_err(|error| format!("live_economics_dir:{}:{error}", state_dir.display()))?;
        // V3 used a user-turn identity for multiple provider calls. V4 keeps the
        // old ledger immutable and starts the corrected request-event domain.
        let ledger_dir = state_dir.join("economics-events-v4");
        let completion_dir = state_dir.join("package-cpu-completions-v1");
        let checkpoint_path = state_dir.join("economics-live-v4.checkpoint");
        let snapshot_path = state_dir.join("economics-live.json");
        let checkpoint = fs::read(&checkpoint_path)
            .ok()
            .and_then(|bytes| serde_cbor::from_slice::<EconomicsCheckpoint>(&bytes).ok())
            .filter(|checkpoint| checkpoint.schema == CHECKPOINT_SCHEMA);
        let prior_epoch = read_prior_epoch_totals(state_dir);
        let now = unix_now();
        let mut ledger = if let Some(checkpoint) = checkpoint {
            let (prior_epoch_schema, prior_epoch_ordinary_tokens, prior_epoch_verified_tokens) =
                if checkpoint.prior_epoch_schema.is_empty() {
                    prior_epoch
                } else {
                    (
                        checkpoint.prior_epoch_schema,
                        checkpoint.prior_epoch_ordinary_tokens,
                        checkpoint.prior_epoch_verified_tokens,
                    )
                };
            Self {
                journal: FramedCborLedger::open(&ledger_dir, "economics")?,
                completion_journal: FramedCborLedger::open_with_limits(
                    &completion_dir,
                    "completion",
                    64 * 1024 * 1024,
                    1,
                )?,
                checkpoint_path,
                snapshot_path,
                prior_epoch_schema,
                prior_epoch_ordinary_tokens,
                prior_epoch_verified_tokens,
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
                verified_receipt_by_intent: checkpoint.verified_receipt_by_intent,
                verified_by_package: checkpoint.verified_by_package,
                completion_roots_by_key: BTreeMap::new(),
                events_since_checkpoint: 0,
                last_checkpoint: Instant::now(),
                last_snapshot: Instant::now(),
            }
        } else {
            Self {
                journal: FramedCborLedger::open(&ledger_dir, "economics")?,
                completion_journal: FramedCborLedger::open_with_limits(
                    &completion_dir,
                    "completion",
                    64 * 1024 * 1024,
                    1,
                )?,
                checkpoint_path,
                snapshot_path,
                prior_epoch_schema: prior_epoch.0,
                prior_epoch_ordinary_tokens: prior_epoch.1,
                prior_epoch_verified_tokens: prior_epoch.2,
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
                verified_receipt_by_intent: BTreeMap::new(),
                verified_by_package: BTreeMap::new(),
                completion_roots_by_key: BTreeMap::new(),
                events_since_checkpoint: 0,
                last_checkpoint: Instant::now(),
                last_snapshot: Instant::now(),
            }
        };
        for receipt in
            read_framed_cbor::<PackageCpuCompletionReceiptV1>(&completion_dir, "completion")?
        {
            receipt.validate()?;
            let key = (receipt.package_id.clone(), receipt.intent_sha256.clone());
            if ledger
                .completion_roots_by_key
                .insert(key, receipt.completion_root_sha256.clone())
                .is_some_and(|existing| existing != receipt.completion_root_sha256)
            {
                return Err("package_cpu_completion_receipt_conflict".to_owned());
            }
        }
        for event in read_framed_cbor::<EconomicsEvent>(&ledger_dir, "economics")? {
            if event.schema == EVENT_SCHEMA {
                ledger.apply(event);
            }
        }
        ledger.reconcile_false_accept_outcomes();
        ledger.rebuild_verified_by_package();
        ledger.backfill_package_completion_receipts()?;
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
            package_id: String::new(),
            verification_receipt_root_sha256: String::new(),
        })
    }

    #[cfg(test)]
    fn observe_verified_accept(
        &mut self,
        intent_sha256: &str,
        input_tokens: u64,
    ) -> Result<(), String> {
        self.observe_verified_accept_with_receipt(intent_sha256, input_tokens, None, None)
    }

    pub fn observe_verified_accept_with_receipt(
        &mut self,
        intent_sha256: &str,
        input_tokens: u64,
        package_id: Option<&str>,
        verification_receipt_root_sha256: Option<&str>,
    ) -> Result<(), String> {
        if self.ineligible.contains(intent_sha256) || self.verified.contains_key(intent_sha256) {
            return self.maybe_persist(false);
        }
        let package_id = package_id.unwrap_or_default();
        let verification_receipt_root_sha256 = verification_receipt_root_sha256.unwrap_or_default();
        if !package_id.is_empty()
            && (!valid_package_id(package_id)
                || !valid_sha256_hex(verification_receipt_root_sha256))
        {
            return Err("live_economics_invalid_verified_receipt_metadata".to_owned());
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
            package_id: package_id.to_owned(),
            verification_receipt_root_sha256: verification_receipt_root_sha256.to_owned(),
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
            package_id: String::new(),
            verification_receipt_root_sha256: String::new(),
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
            package_id: String::new(),
            verification_receipt_root_sha256: String::new(),
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
            package_id: String::new(),
            verification_receipt_root_sha256: String::new(),
        })
    }

    pub fn set_pipeline_dropped(&mut self, dropped: u64) {
        self.pipeline_dropped = self.pipeline_dropped.max(dropped);
    }

    fn record(&mut self, event: EconomicsEvent) -> Result<(), String> {
        let terminal = event.kind != "request";
        let completion_intent =
            (event.kind == "verified_accept").then(|| event.intent_sha256.clone());
        self.journal.append(&event)?;
        if terminal {
            self.journal.sync()?;
        }
        self.apply(event);
        if let Some(intent_sha256) = completion_intent {
            self.append_package_completion_receipt(&intent_sha256)?;
            self.persist_snapshot()?;
            self.last_snapshot = Instant::now();
        }
        self.events_since_checkpoint = self.events_since_checkpoint.saturating_add(1);
        if terminal {
            self.roll_window_if_mature();
        }
        self.maybe_persist(true)
    }

    fn backfill_package_completion_receipts(&mut self) -> Result<(), String> {
        let mut intents = self
            .verified_receipt_by_intent
            .iter()
            .map(|(intent, metadata)| {
                (
                    metadata.accepted_at_unix,
                    intent.clone(),
                    metadata.receipt_root_sha256.clone(),
                )
            })
            .collect::<Vec<_>>();
        intents.sort();
        for (_, intent_sha256, _) in intents {
            self.append_package_completion_receipt(&intent_sha256)?;
        }
        Ok(())
    }

    fn append_package_completion_receipt(&mut self, intent_sha256: &str) -> Result<(), String> {
        let Some(metadata) = self.verified_receipt_by_intent.get(intent_sha256) else {
            return Ok(());
        };
        let receipt = PackageCpuCompletionReceiptV1::seal(intent_sha256, metadata)?;
        let key = (receipt.package_id.clone(), receipt.intent_sha256.clone());
        if let Some(existing) = self.completion_roots_by_key.get(&key) {
            return if existing == &receipt.completion_root_sha256 {
                Ok(())
            } else {
                Err("package_cpu_completion_receipt_conflict".to_owned())
            };
        }
        self.completion_journal.append(&receipt)?;
        self.completion_journal.sync()?;
        self.completion_roots_by_key
            .insert(key, receipt.completion_root_sha256);
        Ok(())
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
                if self.verified.contains_key(&event.intent_sha256) {
                    self.pending_opened_at.remove(&event.intent_sha256);
                    return;
                }
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
                if valid_package_id(&event.package_id)
                    && valid_sha256_hex(&event.verification_receipt_root_sha256)
                {
                    self.verified_receipt_by_intent.insert(
                        event.intent_sha256.clone(),
                        VerifiedReceiptMetadata {
                            package_id: event.package_id.clone(),
                            exact_input_tokens,
                            accepted_at_unix: event.timestamp_unix,
                            receipt_root_sha256: event.verification_receipt_root_sha256.clone(),
                        },
                    );
                    let package = self
                        .verified_by_package
                        .entry(event.package_id)
                        .or_default();
                    if package.first_accept_timestamp_unix == 0
                        || event.timestamp_unix < package.first_accept_timestamp_unix
                    {
                        package.first_accept_timestamp_unix = event.timestamp_unix;
                        package.first_exact_input_tokens = exact_input_tokens;
                        package.first_receipt_root_sha256 =
                            event.verification_receipt_root_sha256.clone();
                    }
                    package.ordinary_accepts = package.ordinary_accepts.saturating_add(1);
                    package.exact_input_tokens = package
                        .exact_input_tokens
                        .saturating_add(exact_input_tokens);
                    if event.timestamp_unix >= package.last_accept_timestamp_unix {
                        package.last_accept_timestamp_unix = event.timestamp_unix;
                        package.last_receipt_root_sha256 = event.verification_receipt_root_sha256;
                    }
                }
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
                self.transition_verified_accept_to_false_accept(&event.intent_sha256);
            }
            "parity_failure" if self.parity_failure_intents.insert(event.intent_sha256) => {
                self.parity_failures = self.parity_failures.saturating_add(1);
            }
            _ => {}
        }
    }

    fn transition_verified_accept_to_false_accept(&mut self, intent_sha256: &str) -> bool {
        let existing_false_accept = self
            .fallback_by_intent
            .get(intent_sha256)
            .is_some_and(|reason| reason == "runtime:false_accept");
        let Some(input_tokens) = self.verified.remove(intent_sha256).or_else(|| {
            existing_false_accept
                .then(|| self.eligible.get(intent_sha256).copied())
                .flatten()
        }) else {
            return false;
        };
        if self.false_accept_intents.insert(intent_sha256.to_owned()) {
            self.false_accepts = self.false_accepts.saturating_add(1);
        }
        if !existing_false_accept {
            let reason = "runtime:false_accept".to_owned();
            self.fallback_by_intent
                .insert(intent_sha256.to_owned(), reason.clone());
            self.fallback_tokens_by_intent
                .insert(intent_sha256.to_owned(), input_tokens);
            let counter = self.fallback_reasons.entry(reason).or_default();
            counter.intents = counter.intents.saturating_add(1);
            counter.input_tokens = counter.input_tokens.saturating_add(input_tokens);
        }
        if self
            .verified_receipt_by_intent
            .remove(intent_sha256)
            .is_some()
        {
            self.rebuild_verified_by_package();
        }
        self.pending_opened_at.remove(intent_sha256);
        true
    }

    fn rebuild_verified_by_package(&mut self) {
        self.verified_by_package.clear();
        for metadata in self.verified_receipt_by_intent.values() {
            let package = self
                .verified_by_package
                .entry(metadata.package_id.clone())
                .or_default();
            if package.first_accept_timestamp_unix == 0
                || metadata.accepted_at_unix < package.first_accept_timestamp_unix
            {
                package.first_accept_timestamp_unix = metadata.accepted_at_unix;
                package.first_exact_input_tokens = metadata.exact_input_tokens;
                package.first_receipt_root_sha256 = metadata.receipt_root_sha256.clone();
            }
            package.ordinary_accepts = package.ordinary_accepts.saturating_add(1);
            package.exact_input_tokens = package
                .exact_input_tokens
                .saturating_add(metadata.exact_input_tokens);
            if metadata.accepted_at_unix >= package.last_accept_timestamp_unix {
                package.last_accept_timestamp_unix = metadata.accepted_at_unix;
                package.last_receipt_root_sha256 = metadata.receipt_root_sha256.clone();
            }
        }
    }

    fn reconcile_false_accept_outcomes(&mut self) {
        let recorded = std::mem::take(&mut self.false_accept_intents);
        self.false_accepts = 0;
        for intent_sha256 in recorded {
            self.transition_verified_accept_to_false_accept(&intent_sha256);
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
                prior_epoch_schema: self.prior_epoch_schema.clone(),
                prior_epoch_ordinary_tokens: self.prior_epoch_ordinary_tokens,
                prior_epoch_verified_tokens: self.prior_epoch_verified_tokens,
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
                verified_receipt_by_intent: self.verified_receipt_by_intent.clone(),
                verified_by_package: self.verified_by_package.clone(),
            },
        )
    }

    fn persist_snapshot(&self) -> Result<(), String> {
        let current = self.current_window(unix_now());
        let display_global_input_tokens = self
            .prior_epoch_ordinary_tokens
            .saturating_add(current.ordinary_tokens);
        let display_avoided_input_tokens = self
            .prior_epoch_verified_tokens
            .saturating_add(current.verified_tokens);
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
        let mut snapshot = json!({
            "schema": SNAPSHOT_SCHEMA,
            "source": "rust_streaming_economics_v3",
            "generated_at_unix": unix_now(),
            "accounting_epoch_started_at_unix": self.epoch_started_at_unix,
            "dedupe_eligible_request_events": eligible_intents,
            "terminal_request_events": terminal_intents,
            "in_flight_local_outcomes": in_flight_local_outcomes,
            "dedupe_ineligible_request_events": self.ineligible.len(),
            "unique_request_events": eligible_intents,
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
            "identity_domain": "request_event.v1",
            "boundary": "terminal ordinary provider request events; a user turn may contain multiple independently accounted model calls; in-flight requests remain outside completed economics windows; exact o200k tokenization of recursively key-sorted JSON request payloads; finalized Rust verifier receipts only; counterfactual provider-billed usage for avoided calls is unavailable and is not claimed",
        });
        if let Some(object) = snapshot.as_object_mut() {
            object.insert(
                "verified_by_package".to_owned(),
                serde_json::to_value(&self.verified_by_package)
                    .map_err(|error| format!("live_economics_package_snapshot:{error}"))?,
            );
            object.insert(
                "display_global_input_tokens".to_owned(),
                json!(display_global_input_tokens),
            );
            object.insert(
                "display_avoided_input_tokens".to_owned(),
                json!(display_avoided_input_tokens),
            );
            object.insert(
                "display_input_token_accounting_partitioned".to_owned(),
                json!(true),
            );
            object.insert(
                "display_prior_epoch_schema".to_owned(),
                json!(&self.prior_epoch_schema),
            );
        }
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

pub(super) fn first_durable_package_completion(
    snapshot_path: &Path,
    package_id: &str,
) -> Result<Option<PackageCpuCompletionReceiptV1>, String> {
    let receipts = durable_package_completions(snapshot_path, package_id)?;
    Ok(receipts.into_iter().next())
}

pub(super) fn durable_package_completions(
    snapshot_path: &Path,
    package_id: &str,
) -> Result<Vec<PackageCpuCompletionReceiptV1>, String> {
    let snapshot_bytes = match fs::read(snapshot_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("package_completion_snapshot_read:{error}")),
    };
    let snapshot: Value = serde_json::from_slice(&snapshot_bytes)
        .map_err(|error| format!("package_completion_snapshot_decode:{error}"))?;
    if snapshot.get("schema").and_then(Value::as_str) != Some(SNAPSHOT_SCHEMA) {
        return Ok(Vec::new());
    }
    let clean = snapshot.get("false_accepts").and_then(Value::as_u64) == Some(0)
        && snapshot
            .get("runtime_parity_mismatches")
            .and_then(Value::as_u64)
            == Some(0)
        && snapshot.get("pipeline_dropped").and_then(Value::as_u64) == Some(0);
    if !clean {
        return Ok(Vec::new());
    }
    let Some(package) = snapshot
        .get("verified_by_package")
        .and_then(Value::as_object)
        .and_then(|packages| packages.get(package_id))
    else {
        return Ok(Vec::new());
    };
    if !package
        .get("ordinary_accepts")
        .and_then(Value::as_u64)
        .is_some_and(|accepts| accepts > 0)
    {
        return Ok(Vec::new());
    }
    let state_dir = snapshot_path
        .parent()
        .ok_or_else(|| "package_completion_state_dir_missing".to_owned())?;
    let completion_dir = state_dir.join("package-cpu-completions-v1");
    if !completion_dir.exists() {
        return Ok(Vec::new());
    }
    let receipts =
        read_framed_cbor::<PackageCpuCompletionReceiptV1>(&completion_dir, "completion")?
            .into_iter()
            .filter(|receipt| receipt.package_id == package_id)
            .collect::<Vec<_>>();
    for receipt in &receipts {
        receipt.validate()?;
    }
    let Some(first) = receipts.first() else {
        return Ok(Vec::new());
    };
    let snapshot_matches = package
        .get("first_receipt_root_sha256")
        .and_then(Value::as_str)
        == Some(first.verification_receipt_root_sha256.as_str())
        && package
            .get("first_accept_timestamp_unix")
            .and_then(Value::as_u64)
            == Some(first.accepted_at_unix)
        && package
            .get("first_exact_input_tokens")
            .and_then(Value::as_u64)
            == Some(first.exact_input_tokens);
    if !snapshot_matches {
        return Ok(Vec::new());
    }
    Ok(receipts)
}

fn read_prior_epoch_totals(state_dir: &Path) -> (String, u64, u64) {
    let path = state_dir.join("economics-live-v3.checkpoint");
    let Some(checkpoint) = fs::read(path)
        .ok()
        .and_then(|bytes| serde_cbor::from_slice::<EconomicsCheckpoint>(&bytes).ok())
        .filter(|checkpoint| checkpoint.schema == "nando.live-economics-checkpoint.v3")
    else {
        return (String::new(), 0, 0);
    };
    let verified_tokens = checkpoint.verified.values().copied().sum::<u64>();
    let ordinary_tokens = verified_tokens.saturating_add(
        checkpoint
            .fallback_tokens_by_intent
            .values()
            .copied()
            .sum::<u64>(),
    );
    (checkpoint.schema, ordinary_tokens, verified_tokens)
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

fn valid_package_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn valid_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
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
        assert_eq!(snapshot["terminal_request_events"], 0);
        assert_eq!(snapshot["global_input_tokens"], 0);
        assert_eq!(snapshot["unresolved_local_outcomes"], 0);
        assert_eq!(snapshot["source_reconciliation"]["complete"], true);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn independent_request_events_do_not_conflict_inside_one_user_turn() {
        let root = std::env::temp_dir().join(format!(
            "nando-live-economics-request-events-{}-{}",
            std::process::id(),
            unix_now()
        ));
        let mut ledger = LiveEconomicsLedger::open(&root).expect("open ledger");
        ledger
            .observe_request("request-event-a", 1_024, true)
            .expect("first request");
        ledger
            .observe_request("request-event-b", 2_048, true)
            .expect("second request");

        assert_eq!(ledger.eligible.len(), 2);
        assert_eq!(ledger.dedupe_conflicts, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn false_accept_removes_cpu_savings_and_replays_as_fallback() {
        let root = std::env::temp_dir().join(format!(
            "nando-live-economics-false-accept-{}-{}",
            std::process::id(),
            unix_now()
        ));
        let mut ledger = LiveEconomicsLedger::open(&root).expect("open ledger");
        ledger
            .observe_request("false-accepted-intent", 2_048, true)
            .expect("request");
        ledger
            .observe_verified_accept("false-accepted-intent", 2_048)
            .expect("verified");
        ledger
            .observe_false_accept("unknown-intent")
            .expect("unknown report");
        ledger
            .observe_false_accept("false-accepted-intent")
            .expect("false accept");
        ledger.persist_snapshot().expect("snapshot");

        let snapshot: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join("economics-live.json")).expect("read snapshot"),
        )
        .expect("snapshot");
        assert_eq!(snapshot["verified_local_accepts"], 0);
        assert_eq!(snapshot["avoided_input_tokens"], 0);
        assert_eq!(snapshot["global_input_tokens"], 2_048);
        assert_eq!(snapshot["false_accepts"], 1);
        assert_eq!(
            snapshot["fallback_reasons"]["runtime:false_accept"]["intents"],
            1
        );

        drop(ledger);
        let replayed = LiveEconomicsLedger::open(&root).expect("replay");
        assert!(replayed.verified.is_empty());
        assert_eq!(replayed.false_accepts, 1);
        assert_eq!(
            replayed
                .fallback_by_intent
                .get("false-accepted-intent")
                .map(String::as_str),
            Some("runtime:false_accept")
        );
        assert!(!replayed.false_accept_intents.contains("unknown-intent"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn corrected_epoch_keeps_legacy_totals_only_for_dashboard_continuity() {
        let root = std::env::temp_dir().join(format!(
            "nando-live-economics-partitioned-display-{}-{}",
            std::process::id(),
            unix_now()
        ));
        fs::create_dir_all(&root).expect("test root");
        let legacy = EconomicsCheckpoint {
            schema: "nando.live-economics-checkpoint.v3".to_owned(),
            prior_epoch_schema: String::new(),
            prior_epoch_ordinary_tokens: 0,
            prior_epoch_verified_tokens: 0,
            epoch_started_at_unix: unix_now(),
            eligible: BTreeMap::new(),
            pending_opened_at: BTreeMap::new(),
            ineligible: BTreeSet::new(),
            verified: BTreeMap::from([("legacy-local".to_owned(), 100)]),
            fallback_by_intent: BTreeMap::from([(
                "legacy-provider".to_owned(),
                "provider:fallback".to_owned(),
            )]),
            fallback_tokens_by_intent: BTreeMap::from([("legacy-provider".to_owned(), 900)]),
            fallback_reasons: BTreeMap::new(),
            completed_windows: Vec::new(),
            dedupe_conflicts: 0,
            false_accepts: 0,
            parity_failures: 0,
            pipeline_dropped: 0,
            false_accept_intents: BTreeSet::new(),
            parity_failure_intents: BTreeSet::new(),
            verified_receipt_by_intent: BTreeMap::new(),
            verified_by_package: BTreeMap::new(),
        };
        write_atomic_cbor(&root.join("economics-live-v3.checkpoint"), &legacy)
            .expect("legacy checkpoint");

        let mut ledger = LiveEconomicsLedger::open(&root).expect("open corrected epoch");
        ledger
            .observe_request("request-event-v4", 250, true)
            .expect("request");
        ledger
            .observe_verified_accept("request-event-v4", 250)
            .expect("verified");
        ledger.persist_snapshot().expect("snapshot");
        let snapshot: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join("economics-live.json")).expect("read snapshot"),
        )
        .expect("snapshot");

        assert_eq!(snapshot["global_input_tokens"], 250);
        assert_eq!(snapshot["avoided_input_tokens"], 250);
        assert_eq!(snapshot["display_global_input_tokens"], 1_250);
        assert_eq!(snapshot["display_avoided_input_tokens"], 350);
        assert_eq!(
            snapshot["display_prior_epoch_schema"],
            "nando.live-economics-checkpoint.v3"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn package_receipt_and_exact_tokens_survive_restart_and_false_accept() {
        let root = std::env::temp_dir().join(format!(
            "nando-live-economics-package-{}-{}",
            std::process::id(),
            unix_now()
        ));
        let package_id = "ms4-natural-test";
        let intent_sha256 = "b".repeat(64);
        let receipt_root = "a".repeat(64);
        let mut ledger = LiveEconomicsLedger::open(&root).expect("open ledger");
        ledger
            .observe_request(&intent_sha256, 321, true)
            .expect("request");
        ledger
            .observe_verified_accept_with_receipt(
                &intent_sha256,
                999,
                Some(package_id),
                Some(&receipt_root),
            )
            .expect("verified");
        ledger.persist_checkpoint().expect("checkpoint");
        ledger.persist_snapshot().expect("snapshot");

        let snapshot: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join("economics-live.json")).expect("read snapshot"),
        )
        .expect("snapshot");
        assert_eq!(
            snapshot["verified_by_package"][package_id]["ordinary_accepts"],
            1
        );
        assert_eq!(
            snapshot["verified_by_package"][package_id]["exact_input_tokens"],
            321
        );
        assert_eq!(
            snapshot["verified_by_package"][package_id]["last_receipt_root_sha256"],
            receipt_root
        );
        assert_eq!(
            snapshot["verified_by_package"][package_id]["first_receipt_root_sha256"],
            receipt_root
        );

        drop(ledger);
        let mut replayed = LiveEconomicsLedger::open(&root).expect("restart");
        assert_eq!(
            replayed
                .verified_by_package
                .get(package_id)
                .map(|package| package.exact_input_tokens),
            Some(321)
        );
        replayed
            .observe_false_accept(&intent_sha256)
            .expect("false accept");
        replayed.persist_snapshot().expect("false snapshot");
        let snapshot: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join("economics-live.json")).expect("read false snapshot"),
        )
        .expect("false snapshot");
        assert!(snapshot["verified_by_package"][package_id].is_null());
        let _ = fs::remove_dir_all(root);
    }
}
