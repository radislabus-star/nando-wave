use std::collections::BTreeSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

use nando_operator_learning::{
    FramedCborLedger, S1C4_CLASSIFICATION_LEDGER_PREFIX_V1, S1C4_MAX_CLASSIFICATION_ROWS_V1,
    S1c4ClassificationRowV1, S1c4TerminalClassificationV1, read_framed_cbor,
    s1c4_classification_genesis_root_v1,
};
use serde::Serialize;

const CLASSIFICATION_QUEUE_CAPACITY: usize = 4096;
const CLASSIFICATION_SEGMENT_BYTES: u64 = 16 * 1024 * 1024;
const CLASSIFICATION_SYNC_EVERY_RECORDS: u32 = 32;
const CLASSIFICATION_SYNC_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Clone, Debug)]
pub struct S1c4ClassificationIngressV1 {
    pub opportunity_sequence: u64,
    pub opportunity_request_ordinal: u64,
    pub opportunity_event_root_sha256: String,
    pub request_input_tokens: u64,
    pub request_observed_at_unix: u64,
    pub request_event_identity_root_sha256: String,
    pub session_lineage_root_sha256: String,
    pub observed_at_unix_ms: u64,
    pub classification: S1c4TerminalClassificationV1,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct S1c4ClassificationWriterStatusV1 {
    pub ready: bool,
    pub recovered_rows: u64,
    pub enqueued_rows: u64,
    pub appended_rows: u64,
    pub durable_rows: u64,
    pub queue_overflow: u64,
    pub disconnected: u64,
    pub duplicate_rows: u64,
    pub writer_failures: u64,
    pub last_opportunity_sequence: u64,
    pub last_row_root_sha256: String,
    pub last_error: String,
}

#[derive(Clone)]
pub struct S1c4ClassificationRuntimeV1 {
    sender: SyncSender<S1c4ClassificationIngressV1>,
    telemetry: Arc<S1c4ClassificationTelemetryV1>,
    window_start_exclusive: Arc<AtomicU64>,
    window_end_inclusive: Arc<AtomicU64>,
    window_deadline_unix: Arc<AtomicU64>,
    scan_lock: Arc<Mutex<()>>,
}

struct S1c4ClassificationTelemetryV1 {
    ready: AtomicBool,
    recovered_rows: AtomicU64,
    enqueued_rows: AtomicU64,
    appended_rows: AtomicU64,
    durable_rows: AtomicU64,
    queue_overflow: AtomicU64,
    disconnected: AtomicU64,
    duplicate_rows: AtomicU64,
    writer_failures: AtomicU64,
    last_opportunity_sequence: AtomicU64,
    last_row_root_sha256: RwLock<String>,
    last_error: RwLock<String>,
}

struct RecoveredClassificationStateV1 {
    rows: u64,
    last_opportunity_sequence: u64,
    last_row_root_sha256: String,
    seen_sequences: BTreeSet<u64>,
    seen_request_ordinals: BTreeSet<u64>,
    seen_request_roots: BTreeSet<String>,
}

impl S1c4ClassificationRuntimeV1 {
    pub fn open(directory: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(directory)
            .map_err(|error| format!("s1c4_classification_directory_create:{error}"))?;
        let recovered = recover_classification_state(directory)?;
        let telemetry = Arc::new(S1c4ClassificationTelemetryV1 {
            ready: AtomicBool::new(false),
            recovered_rows: AtomicU64::new(recovered.rows),
            enqueued_rows: AtomicU64::new(0),
            appended_rows: AtomicU64::new(recovered.rows),
            durable_rows: AtomicU64::new(recovered.rows),
            queue_overflow: AtomicU64::new(0),
            disconnected: AtomicU64::new(0),
            duplicate_rows: AtomicU64::new(0),
            writer_failures: AtomicU64::new(0),
            last_opportunity_sequence: AtomicU64::new(recovered.last_opportunity_sequence),
            last_row_root_sha256: RwLock::new(recovered.last_row_root_sha256.clone()),
            last_error: RwLock::new(String::new()),
        });
        let mut ledger = FramedCborLedger::open_with_limits(
            directory,
            S1C4_CLASSIFICATION_LEDGER_PREFIX_V1,
            CLASSIFICATION_SEGMENT_BYTES,
            CLASSIFICATION_SYNC_EVERY_RECORDS,
        )?;
        let (sender, receiver) =
            sync_channel::<S1c4ClassificationIngressV1>(CLASSIFICATION_QUEUE_CAPACITY);
        let writer_telemetry = Arc::clone(&telemetry);
        let scan_lock = Arc::new(Mutex::new(()));
        let writer_scan_lock = Arc::clone(&scan_lock);
        thread::Builder::new()
            .name("nando-s1c4-classification-writer".to_owned())
            .spawn(move || {
                let mut previous_root = recovered.last_row_root_sha256;
                let mut seen_sequences = recovered.seen_sequences;
                let mut seen_request_ordinals = recovered.seen_request_ordinals;
                let mut seen_request_roots = recovered.seen_request_roots;
                writer_telemetry.ready.store(true, Ordering::Release);
                loop {
                    match receiver.recv_timeout(CLASSIFICATION_SYNC_INTERVAL) {
                        Ok(ingress) => {
                            let _scan_guard = match writer_scan_lock.lock() {
                                Ok(guard) => guard,
                                Err(_) => {
                                    writer_telemetry
                                        .writer_failures
                                        .fetch_add(1, Ordering::Relaxed);
                                    set_error(
                                        &writer_telemetry,
                                        "s1c4_classification_scan_lock_poisoned",
                                    );
                                    continue;
                                }
                            };
                            if !seen_sequences.insert(ingress.opportunity_sequence)
                                || !seen_request_ordinals
                                    .insert(ingress.opportunity_request_ordinal)
                                || !seen_request_roots
                                    .insert(ingress.request_event_identity_root_sha256.clone())
                            {
                                writer_telemetry
                                    .duplicate_rows
                                    .fetch_add(1, Ordering::Relaxed);
                                set_error(&writer_telemetry, "s1c4_classification_duplicate");
                                continue;
                            }
                            let row = match S1c4ClassificationRowV1::seal(
                                previous_root.clone(),
                                ingress.opportunity_sequence,
                                ingress.opportunity_request_ordinal,
                                ingress.opportunity_event_root_sha256,
                                ingress.request_input_tokens,
                                ingress.request_observed_at_unix,
                                ingress.request_event_identity_root_sha256,
                                ingress.session_lineage_root_sha256,
                                ingress.observed_at_unix_ms,
                                ingress.classification,
                            ) {
                                Ok(row) => row,
                                Err(error) => {
                                    writer_telemetry
                                        .writer_failures
                                        .fetch_add(1, Ordering::Relaxed);
                                    set_error(&writer_telemetry, error);
                                    continue;
                                }
                            };
                            if let Err(error) = ledger.append(&row) {
                                writer_telemetry
                                    .writer_failures
                                    .fetch_add(1, Ordering::Relaxed);
                                set_error(&writer_telemetry, &error);
                                continue;
                            }
                            previous_root = row.row_root_sha256.clone();
                            writer_telemetry
                                .appended_rows
                                .fetch_add(1, Ordering::Release);
                            writer_telemetry
                                .last_opportunity_sequence
                                .fetch_max(row.opportunity_sequence, Ordering::Release);
                            if let Ok(mut root) = writer_telemetry.last_row_root_sha256.write() {
                                *root = row.row_root_sha256;
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                    if let Err(error) = ledger.sync() {
                        writer_telemetry
                            .writer_failures
                            .fetch_add(1, Ordering::Relaxed);
                        set_error(&writer_telemetry, &error);
                    } else {
                        writer_telemetry.durable_rows.store(
                            writer_telemetry.appended_rows.load(Ordering::Acquire),
                            Ordering::Release,
                        );
                    }
                }
                let _ = ledger.sync();
                writer_telemetry.ready.store(false, Ordering::Release);
            })
            .map_err(|error| format!("s1c4_classification_writer_spawn:{error}"))?;
        Ok(Self {
            sender,
            telemetry,
            window_start_exclusive: Arc::new(AtomicU64::new(0)),
            window_end_inclusive: Arc::new(AtomicU64::new(0)),
            window_deadline_unix: Arc::new(AtomicU64::new(0)),
            scan_lock,
        })
    }

    pub fn try_submit_terminal(&self, ingress: S1c4ClassificationIngressV1) -> Result<(), String> {
        match self.sender.try_send(ingress) {
            Ok(()) => {
                self.telemetry.enqueued_rows.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(TrySendError::Full(_)) => {
                self.telemetry
                    .queue_overflow
                    .fetch_add(1, Ordering::Relaxed);
                set_error(&self.telemetry, "s1c4_classification_queue_full");
                Err("s1c4_classification_queue_full".to_owned())
            }
            Err(TrySendError::Disconnected(_)) => {
                self.telemetry.disconnected.fetch_add(1, Ordering::Relaxed);
                set_error(&self.telemetry, "s1c4_classification_writer_disconnected");
                Err("s1c4_classification_writer_disconnected".to_owned())
            }
        }
    }

    pub fn configure_window(
        &self,
        start_exclusive: u64,
        end_inclusive: u64,
        deadline_unix: u64,
    ) -> Result<(), String> {
        if end_inclusive <= start_exclusive || deadline_unix == 0 {
            return Err("s1c4_classification_window_invalid".to_owned());
        }
        self.window_start_exclusive
            .store(start_exclusive, Ordering::Relaxed);
        self.window_deadline_unix
            .store(deadline_unix, Ordering::Relaxed);
        self.window_end_inclusive
            .store(end_inclusive, Ordering::Release);
        Ok(())
    }

    pub fn disable_window(&self) {
        self.window_end_inclusive.store(0, Ordering::Release);
    }

    pub fn read_rows(&self, directory: &Path) -> Result<Vec<S1c4ClassificationRowV1>, String> {
        let _guard = self
            .scan_lock
            .lock()
            .map_err(|_| "s1c4_classification_scan_lock_poisoned".to_owned())?;
        let _ = recover_classification_state(directory)?;
        read_framed_cbor(directory, S1C4_CLASSIFICATION_LEDGER_PREFIX_V1)
    }

    #[must_use]
    pub fn eligible_ticket(&self, request_ordinal: u64, deadline_eligible: bool) -> bool {
        let end_inclusive = self.window_end_inclusive.load(Ordering::Acquire);
        let start_exclusive = self.window_start_exclusive.load(Ordering::Relaxed);
        end_inclusive > start_exclusive
            && request_ordinal > start_exclusive
            && request_ordinal <= end_inclusive
            && deadline_eligible
    }

    #[must_use]
    pub fn status(&self) -> S1c4ClassificationWriterStatusV1 {
        S1c4ClassificationWriterStatusV1 {
            ready: self.telemetry.ready.load(Ordering::Acquire),
            recovered_rows: self.telemetry.recovered_rows.load(Ordering::Relaxed),
            enqueued_rows: self.telemetry.enqueued_rows.load(Ordering::Relaxed),
            appended_rows: self.telemetry.appended_rows.load(Ordering::Acquire),
            durable_rows: self.telemetry.durable_rows.load(Ordering::Acquire),
            queue_overflow: self.telemetry.queue_overflow.load(Ordering::Relaxed),
            disconnected: self.telemetry.disconnected.load(Ordering::Relaxed),
            duplicate_rows: self.telemetry.duplicate_rows.load(Ordering::Relaxed),
            writer_failures: self.telemetry.writer_failures.load(Ordering::Relaxed),
            last_opportunity_sequence: self
                .telemetry
                .last_opportunity_sequence
                .load(Ordering::Acquire),
            last_row_root_sha256: self
                .telemetry
                .last_row_root_sha256
                .read()
                .map(|root| root.clone())
                .unwrap_or_default(),
            last_error: self
                .telemetry
                .last_error
                .read()
                .map(|error| error.clone())
                .unwrap_or_else(|_| "s1c4_classification_status_lock_poisoned".to_owned()),
        }
    }
}

fn recover_classification_state(
    directory: &Path,
) -> Result<RecoveredClassificationStateV1, String> {
    let rows = read_framed_cbor::<S1c4ClassificationRowV1>(
        directory,
        S1C4_CLASSIFICATION_LEDGER_PREFIX_V1,
    )?;
    if u64::try_from(rows.len()).unwrap_or(u64::MAX) > S1C4_MAX_CLASSIFICATION_ROWS_V1 {
        return Err("s1c4_classification_row_budget_exhausted".to_owned());
    }
    let mut previous_root = s1c4_classification_genesis_root_v1();
    let mut maximum_sequence = 0_u64;
    let mut seen_sequences = BTreeSet::new();
    let mut seen_request_ordinals = BTreeSet::new();
    let mut seen_request_roots = BTreeSet::new();
    for row in &rows {
        row.validate().map_err(str::to_owned)?;
        if row.previous_row_root_sha256 != previous_root
            || !seen_sequences.insert(row.opportunity_sequence)
            || !seen_request_ordinals.insert(row.opportunity_request_ordinal)
            || !seen_request_roots.insert(row.request_event_identity_root_sha256.clone())
        {
            return Err("s1c4_classification_recovery_identity_invalid".to_owned());
        }
        maximum_sequence = maximum_sequence.max(row.opportunity_sequence);
        previous_root = row.row_root_sha256.clone();
    }
    Ok(RecoveredClassificationStateV1 {
        rows: u64::try_from(rows.len()).unwrap_or(u64::MAX),
        last_opportunity_sequence: maximum_sequence,
        last_row_root_sha256: previous_root,
        seen_sequences,
        seen_request_ordinals,
        seen_request_roots,
    })
}

fn set_error(telemetry: &S1c4ClassificationTelemetryV1, error: &str) {
    if let Ok(mut last_error) = telemetry.last_error.write() {
        *last_error = error.to_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nando_operator_learning::GroundedDecisionShadowCensorV1;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn root(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    #[test]
    fn writer_recovers_exact_hash_chain_after_restart() {
        let directory = std::env::temp_dir().join(format!(
            "nando-s1c4-writer-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let runtime = S1c4ClassificationRuntimeV1::open(&directory).expect("runtime");
        runtime
            .configure_window(6, 7, 1_700_000_001)
            .expect("window");
        let request_root = root('2');
        let opportunity_event_root_sha256 =
            nando_operator_learning::OpportunityBridgeEventV1::request(
                request_root.clone(),
                42,
                1_700_000_000,
            )
            .canonical_sha256()
            .expect("event root");
        runtime
            .try_submit_terminal(S1c4ClassificationIngressV1 {
                opportunity_sequence: 11,
                opportunity_request_ordinal: 7,
                opportunity_event_root_sha256,
                request_input_tokens: 42,
                request_observed_at_unix: 1_700_000_000,
                request_event_identity_root_sha256: request_root,
                session_lineage_root_sha256: root('3'),
                observed_at_unix_ms: 4,
                classification: S1c4TerminalClassificationV1::Censored {
                    reason: GroundedDecisionShadowCensorV1::MissingExactGoal,
                },
            })
            .expect("submit");
        for _ in 0..100 {
            if runtime.status().durable_rows == 1 {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(runtime.status().durable_rows, 1);
        drop(runtime);
        thread::sleep(Duration::from_millis(20));
        let restored = S1c4ClassificationRuntimeV1::open(&directory).expect("restore");
        let status = restored.status();
        assert_eq!(status.recovered_rows, 1);
        assert_eq!(status.last_opportunity_sequence, 11);
        assert_ne!(
            status.last_row_root_sha256,
            s1c4_classification_genesis_root_v1()
        );
        drop(restored);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn ticket_started_before_close_can_submit_its_terminal_row_after_close() {
        let directory = std::env::temp_dir().join(format!(
            "nando-s1c4-terminal-after-close-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let runtime = S1c4ClassificationRuntimeV1::open(&directory).expect("runtime");
        runtime.disable_window();
        let request_root = root('4');
        let opportunity_event_root_sha256 =
            nando_operator_learning::OpportunityBridgeEventV1::request(
                request_root.clone(),
                7,
                1_700_000_000,
            )
            .canonical_sha256()
            .expect("event root");
        runtime
            .try_submit_terminal(S1c4ClassificationIngressV1 {
                opportunity_sequence: 12,
                opportunity_request_ordinal: 8,
                opportunity_event_root_sha256,
                request_input_tokens: 7,
                request_observed_at_unix: 1_700_000_000,
                request_event_identity_root_sha256: request_root,
                session_lineage_root_sha256: root('5'),
                observed_at_unix_ms: 5,
                classification: S1c4TerminalClassificationV1::Censored {
                    reason: GroundedDecisionShadowCensorV1::MissingExactGoal,
                },
            })
            .expect("terminal submit");
        for _ in 0..100 {
            if runtime.status().durable_rows == 1 {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(runtime.status().durable_rows, 1);
        drop(runtime);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn frozen_ticket_predicate_includes_exact_boundary_and_excludes_neighbors() {
        let directory = std::env::temp_dir().join(format!(
            "nando-s1c4-ticket-boundary-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let runtime = S1c4ClassificationRuntimeV1::open(&directory).expect("runtime");
        runtime
            .configure_window(100, 1_124, 1_700_000_100)
            .expect("window");
        assert!(!runtime.eligible_ticket(100, true));
        assert!(runtime.eligible_ticket(101, true));
        assert!(runtime.eligible_ticket(1_124, true));
        assert!(!runtime.eligible_ticket(1_125, true));
        assert!(!runtime.eligible_ticket(101, false));
        runtime.disable_window();
        assert!(!runtime.eligible_ticket(101, true));
        drop(runtime);
        let _ = std::fs::remove_dir_all(directory);
    }
}
