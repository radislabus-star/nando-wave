use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use nando_response_actor::{
    ECONOMICS_RECEIPT_SCHEMA_V1, EconomicsReceipt, FramedCborLedger, OnlineCollectionMiner,
    OnlineCollectionObservation, OnlineResponseMinerReport, OnlineResponseStream,
    OnlineResponseStreamStatus, ReducibilityClass, RelationFrame, RuntimeParityCase,
    TeacherTransition, read_framed_cbor, teacher_transition_from_completed,
};

const QUEUE_CAPACITY: usize = 4_096;
const INPUTS_PER_SYNTHESIS_SLICE: u64 = 64;
const CHECKPOINT_EVENTS: u64 = 4_096;
const CHECKPOINT_INTERVAL: Duration = Duration::from_secs(60);
const EXPERIMENTAL_SELF_TRAINING_WORK_ENABLED: bool = true;

#[derive(Default)]
struct MinerWorkerCounters {
    enqueued: AtomicU64,
    processed: AtomicU64,
    failed: AtomicU64,
    checkpoints: AtomicU64,
    synthesis_slices: AtomicU64,
    exact_checks: AtomicU64,
    replayed_records: AtomicU64,
    collection_processed: AtomicU64,
    collection_maintenance_slices: AtomicU64,
    collection_replayed_records: AtomicU64,
    opportunity_processed: AtomicU64,
    opportunity_replayed_records: AtomicU64,
    replay_rejected_records: AtomicU64,
    opportunity_dropped: AtomicU64,
    startup_replay_micros: AtomicU64,
    startup_replay_support_before: AtomicU64,
    startup_replay_support_after_teacher: AtomicU64,
    startup_replay_support_after_opportunity: AtomicU64,
    transition_last_micros: AtomicU64,
    transition_max_micros: AtomicU64,
    transition_total_micros: AtomicU64,
    collection_last_micros: AtomicU64,
    collection_max_micros: AtomicU64,
    collection_total_micros: AtomicU64,
    collection_maintenance_last_micros: AtomicU64,
    collection_maintenance_max_micros: AtomicU64,
    collection_maintenance_total_micros: AtomicU64,
    synthesis_last_micros: AtomicU64,
    synthesis_max_micros: AtomicU64,
    synthesis_total_micros: AtomicU64,
    checkpoint_last_micros: AtomicU64,
    checkpoint_max_micros: AtomicU64,
    checkpoint_total_micros: AtomicU64,
    opportunity_last_micros: AtomicU64,
    opportunity_max_micros: AtomicU64,
    opportunity_total_micros: AtomicU64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize)]
pub struct MinerWorkerStatus {
    pub queue_capacity: usize,
    pub inputs_per_synthesis_slice: u64,
    pub checkpoint_events: u64,
    pub checkpoint_interval_seconds: u64,
    pub enqueued: u64,
    pub processed: u64,
    pub failed: u64,
    pub checkpoints: u64,
    pub synthesis_slices: u64,
    pub exact_checks: u64,
    pub replayed_records: u64,
    pub collection_processed: u64,
    pub collection_maintenance_slices: u64,
    pub collection_replayed_records: u64,
    pub opportunity_processed: u64,
    pub opportunity_replayed_records: u64,
    pub replay_rejected_records: u64,
    pub opportunity_dropped: u64,
    pub queue_backlog_estimate: u64,
    pub startup_replay_micros: u64,
    pub startup_replay_support_before: u64,
    pub startup_replay_support_after_teacher: u64,
    pub startup_replay_support_after_opportunity: u64,
    pub transition_last_micros: u64,
    pub transition_max_micros: u64,
    pub transition_total_micros: u64,
    pub collection_last_micros: u64,
    pub collection_max_micros: u64,
    pub collection_total_micros: u64,
    pub collection_maintenance_last_micros: u64,
    pub collection_maintenance_max_micros: u64,
    pub collection_maintenance_total_micros: u64,
    pub synthesis_last_micros: u64,
    pub synthesis_max_micros: u64,
    pub synthesis_total_micros: u64,
    pub checkpoint_last_micros: u64,
    pub checkpoint_max_micros: u64,
    pub checkpoint_total_micros: u64,
    pub opportunity_last_micros: u64,
    pub opportunity_max_micros: u64,
    pub opportunity_total_micros: u64,
}

#[derive(Clone)]
pub struct MinerWorkerHandle {
    sender: SyncSender<MinerCommand>,
    counters: Arc<MinerWorkerCounters>,
    response_report: Arc<std::sync::RwLock<Option<OnlineResponseMinerReport>>>,
    response_status: Arc<std::sync::RwLock<Option<OnlineResponseStreamStatus>>>,
}

enum MinerCommand {
    Transition(Box<TeacherTransition>),
    Collection(OnlineCollectionObservation),
    Opportunity(MinerOpportunityEvent),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
enum MinerOpportunityEvent {
    Request {
        intent_sha256: String,
        input_tokens: u64,
        now_unix: u64,
    },
    Classify {
        intent_sha256: String,
        class: ReducibilityClass,
        blocker: String,
    },
    Verified {
        intent_sha256: String,
    },
    ParityFailure {
        intent_sha256: String,
    },
    FalseAccept {
        intent_sha256: String,
    },
}

impl MinerWorkerHandle {
    pub fn submit_frame(&self, frame: RelationFrame) -> Result<(), String> {
        self.submit_frame_with_parity(frame, None)
    }

    pub fn submit_frame_with_parity(
        &self,
        frame: RelationFrame,
        runtime_parity_case: Option<RuntimeParityCase>,
    ) -> Result<(), String> {
        let economics = (frame.estimated_input_tokens > 0).then(|| EconomicsReceipt {
            schema: ECONOMICS_RECEIPT_SCHEMA_V1.to_owned(),
            exact_input_tokens: frame.estimated_input_tokens,
            ordinary: true,
            controlled: false,
            replay: false,
            dedupe_eligible: true,
            provider_evidence_ref_sha256: frame.evidence_ref_sha256.clone(),
        });
        let mut transition = teacher_transition_from_completed(&frame, economics)
            .map_err(|error| format!("miner_teacher_transition:{error:?}"))?;
        transition.runtime_parity_case = runtime_parity_case;
        self.submit_transition(transition)
    }

    pub fn submit_transition(&self, transition: TeacherTransition) -> Result<(), String> {
        self.sender
            .send(MinerCommand::Transition(Box::new(transition)))
            .map_err(|_| "miner_worker_stopped".to_owned())?;
        self.counters.enqueued.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn submit_collection(
        &self,
        observation: OnlineCollectionObservation,
    ) -> Result<(), String> {
        self.sender
            .send(MinerCommand::Collection(observation))
            .map_err(|_| "miner_worker_stopped".to_owned())?;
        self.counters.enqueued.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn submit_opportunity_request(
        &self,
        intent_sha256: String,
        input_tokens: u64,
        now_unix: u64,
    ) -> Result<(), String> {
        self.try_submit_opportunity(MinerOpportunityEvent::Request {
            intent_sha256,
            input_tokens,
            now_unix,
        })
    }

    pub fn submit_opportunity_classification(
        &self,
        intent_sha256: String,
        class: ReducibilityClass,
        blocker: String,
    ) -> Result<(), String> {
        self.try_submit_opportunity(MinerOpportunityEvent::Classify {
            intent_sha256,
            class,
            blocker,
        })
    }

    pub fn submit_opportunity_verified(&self, intent_sha256: String) -> Result<(), String> {
        self.try_submit_opportunity(MinerOpportunityEvent::Verified { intent_sha256 })
    }

    pub fn submit_opportunity_parity_failure(&self, intent_sha256: String) -> Result<(), String> {
        self.try_submit_opportunity(MinerOpportunityEvent::ParityFailure { intent_sha256 })
    }

    pub fn submit_opportunity_false_accept(&self, intent_sha256: String) -> Result<(), String> {
        self.try_submit_opportunity(MinerOpportunityEvent::FalseAccept { intent_sha256 })
    }

    fn try_submit_opportunity(&self, event: MinerOpportunityEvent) -> Result<(), String> {
        match self.sender.try_send(MinerCommand::Opportunity(event)) {
            Ok(()) => {
                self.counters.enqueued.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(mpsc::TrySendError::Full(_)) => {
                self.counters
                    .opportunity_dropped
                    .fetch_add(1, Ordering::Relaxed);
                Err("miner_worker_opportunity_queue_full".to_owned())
            }
            Err(mpsc::TrySendError::Disconnected(_)) => Err("miner_worker_stopped".to_owned()),
        }
    }

    #[must_use]
    pub fn status(&self) -> MinerWorkerStatus {
        let enqueued = self.counters.enqueued.load(Ordering::Relaxed);
        let processed = self.counters.processed.load(Ordering::Relaxed);
        let failed = self.counters.failed.load(Ordering::Relaxed);
        MinerWorkerStatus {
            queue_capacity: QUEUE_CAPACITY,
            inputs_per_synthesis_slice: INPUTS_PER_SYNTHESIS_SLICE,
            checkpoint_events: CHECKPOINT_EVENTS,
            checkpoint_interval_seconds: CHECKPOINT_INTERVAL.as_secs(),
            enqueued,
            processed,
            failed,
            checkpoints: self.counters.checkpoints.load(Ordering::Relaxed),
            synthesis_slices: self.counters.synthesis_slices.load(Ordering::Relaxed),
            exact_checks: self.counters.exact_checks.load(Ordering::Relaxed),
            replayed_records: self.counters.replayed_records.load(Ordering::Relaxed),
            collection_processed: self.counters.collection_processed.load(Ordering::Relaxed),
            collection_maintenance_slices: self
                .counters
                .collection_maintenance_slices
                .load(Ordering::Relaxed),
            collection_replayed_records: self
                .counters
                .collection_replayed_records
                .load(Ordering::Relaxed),
            opportunity_processed: self.counters.opportunity_processed.load(Ordering::Relaxed),
            opportunity_replayed_records: self
                .counters
                .opportunity_replayed_records
                .load(Ordering::Relaxed),
            replay_rejected_records: self
                .counters
                .replay_rejected_records
                .load(Ordering::Relaxed),
            opportunity_dropped: self.counters.opportunity_dropped.load(Ordering::Relaxed),
            queue_backlog_estimate: enqueued.saturating_sub(processed.saturating_add(failed)),
            startup_replay_micros: self.counters.startup_replay_micros.load(Ordering::Relaxed),
            startup_replay_support_before: self
                .counters
                .startup_replay_support_before
                .load(Ordering::Relaxed),
            startup_replay_support_after_teacher: self
                .counters
                .startup_replay_support_after_teacher
                .load(Ordering::Relaxed),
            startup_replay_support_after_opportunity: self
                .counters
                .startup_replay_support_after_opportunity
                .load(Ordering::Relaxed),
            transition_last_micros: self.counters.transition_last_micros.load(Ordering::Relaxed),
            transition_max_micros: self.counters.transition_max_micros.load(Ordering::Relaxed),
            transition_total_micros: self
                .counters
                .transition_total_micros
                .load(Ordering::Relaxed),
            collection_last_micros: self.counters.collection_last_micros.load(Ordering::Relaxed),
            collection_max_micros: self.counters.collection_max_micros.load(Ordering::Relaxed),
            collection_total_micros: self
                .counters
                .collection_total_micros
                .load(Ordering::Relaxed),
            collection_maintenance_last_micros: self
                .counters
                .collection_maintenance_last_micros
                .load(Ordering::Relaxed),
            collection_maintenance_max_micros: self
                .counters
                .collection_maintenance_max_micros
                .load(Ordering::Relaxed),
            collection_maintenance_total_micros: self
                .counters
                .collection_maintenance_total_micros
                .load(Ordering::Relaxed),
            synthesis_last_micros: self.counters.synthesis_last_micros.load(Ordering::Relaxed),
            synthesis_max_micros: self.counters.synthesis_max_micros.load(Ordering::Relaxed),
            synthesis_total_micros: self.counters.synthesis_total_micros.load(Ordering::Relaxed),
            checkpoint_last_micros: self.counters.checkpoint_last_micros.load(Ordering::Relaxed),
            checkpoint_max_micros: self.counters.checkpoint_max_micros.load(Ordering::Relaxed),
            checkpoint_total_micros: self
                .counters
                .checkpoint_total_micros
                .load(Ordering::Relaxed),
            opportunity_last_micros: self
                .counters
                .opportunity_last_micros
                .load(Ordering::Relaxed),
            opportunity_max_micros: self.counters.opportunity_max_micros.load(Ordering::Relaxed),
            opportunity_total_micros: self
                .counters
                .opportunity_total_micros
                .load(Ordering::Relaxed),
        }
    }

    #[must_use]
    pub fn response_report(&self) -> Option<OnlineResponseMinerReport> {
        self.response_report
            .read()
            .ok()
            .and_then(|report| report.clone())
    }

    #[must_use]
    pub fn response_status(&self) -> Option<OnlineResponseStreamStatus> {
        self.response_status.read().ok().and_then(|status| *status)
    }
}

pub fn spawn_miner_worker(
    miner: Arc<Mutex<OnlineResponseStream>>,
    collection_miner: Arc<Mutex<OnlineCollectionMiner>>,
    state_dir: PathBuf,
    authority_trigger: Option<SyncSender<()>>,
) -> Result<MinerWorkerHandle, String> {
    let startup_started = Instant::now();
    let teacher_ledger_dir = state_dir.join("response-teacher-segments-v2");
    let collection_ledger_dir = state_dir.join("response-collection-segments-v2");
    let opportunity_ledger_dir = state_dir.join("response-opportunity-segments-v2");
    fs::create_dir_all(&teacher_ledger_dir)
        .map_err(|error| format!("miner_worker_ledger_dir:{error}"))?;
    fs::create_dir_all(&collection_ledger_dir)
        .map_err(|error| format!("miner_worker_collection_ledger_dir:{error}"))?;
    fs::create_dir_all(&opportunity_ledger_dir)
        .map_err(|error| format!("miner_worker_opportunity_ledger_dir:{error}"))?;
    let teacher_replay =
        read_framed_cbor::<TeacherTransition>(&teacher_ledger_dir, "teacher-transition")?;
    let collection_replay = read_framed_cbor::<OnlineCollectionObservation>(
        &collection_ledger_dir,
        "collection-observation",
    )?;
    let opportunity_replay =
        read_framed_cbor::<MinerOpportunityEvent>(&opportunity_ledger_dir, "opportunity")?;
    let startup_replay_support_before = miner
        .lock()
        .map_err(|_| "miner_worker_initial_support_lock_poisoned".to_owned())?
        .replay_support_parity_cases_total();
    let mut replay_rejected_records = 0_u64;
    if !teacher_replay.is_empty() {
        let mut stream = miner
            .lock()
            .map_err(|_| "miner_worker_replay_lock_poisoned".to_owned())?;
        for transition in &teacher_replay {
            if let Err(error) = stream.apply_teacher_transition(transition.clone()) {
                replay_rejected_records = replay_rejected_records.saturating_add(1);
                eprintln!("nando-response-miner-v2 replay rejected: {error}");
            }
        }
        stream.persist_now()?;
    }
    let startup_replay_support_after_teacher = miner
        .lock()
        .map_err(|_| "miner_worker_teacher_support_lock_poisoned".to_owned())?
        .replay_support_parity_cases_total();
    if !collection_replay.is_empty() {
        let mut collection = collection_miner
            .lock()
            .map_err(|_| "miner_worker_collection_replay_lock_poisoned".to_owned())?;
        for observation in &collection_replay {
            if let Err(error) = collection.observe_buffered(observation.clone()) {
                replay_rejected_records = replay_rejected_records.saturating_add(1);
                eprintln!("nando-response-miner-v2 collection replay rejected: {error}");
            }
        }
        collection.flush()?;
    }
    if !opportunity_replay.is_empty() {
        let mut stream = miner
            .lock()
            .map_err(|_| "miner_worker_opportunity_replay_lock_poisoned".to_owned())?;
        for event in &opportunity_replay {
            apply_opportunity_event(&mut stream, event.clone());
        }
        stream.persist_now()?;
    }
    let startup_replay_support_after_opportunity = miner
        .lock()
        .map_err(|_| "miner_worker_opportunity_support_lock_poisoned".to_owned())?
        .replay_support_parity_cases_total();
    let mut teacher_ledger = FramedCborLedger::open(&teacher_ledger_dir, "teacher-transition")?;
    let mut collection_ledger =
        FramedCborLedger::open(&collection_ledger_dir, "collection-observation")?;
    let mut opportunity_ledger = FramedCborLedger::open(&opportunity_ledger_dir, "opportunity")?;
    if !teacher_replay.is_empty() {
        teacher_ledger.compact_after_checkpoint()?;
    }
    if !collection_replay.is_empty() {
        collection_ledger.compact_after_checkpoint()?;
    }
    if !opportunity_replay.is_empty() {
        opportunity_ledger.compact_after_checkpoint()?;
    }
    let (sender, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
    let counters = Arc::new(MinerWorkerCounters::default());
    counters
        .startup_replay_micros
        .store(elapsed_micros(startup_started), Ordering::Relaxed);
    counters.startup_replay_support_before.store(
        u64::try_from(startup_replay_support_before).unwrap_or(u64::MAX),
        Ordering::Relaxed,
    );
    counters.startup_replay_support_after_teacher.store(
        u64::try_from(startup_replay_support_after_teacher).unwrap_or(u64::MAX),
        Ordering::Relaxed,
    );
    counters.startup_replay_support_after_opportunity.store(
        u64::try_from(startup_replay_support_after_opportunity).unwrap_or(u64::MAX),
        Ordering::Relaxed,
    );
    counters.replayed_records.store(
        u64::try_from(teacher_replay.len()).unwrap_or(u64::MAX),
        Ordering::Relaxed,
    );
    counters.collection_replayed_records.store(
        u64::try_from(collection_replay.len()).unwrap_or(u64::MAX),
        Ordering::Relaxed,
    );
    counters.opportunity_replayed_records.store(
        u64::try_from(opportunity_replay.len()).unwrap_or(u64::MAX),
        Ordering::Relaxed,
    );
    counters
        .replay_rejected_records
        .store(replay_rejected_records, Ordering::Relaxed);
    let initial_synthesis_pending = EXPERIMENTAL_SELF_TRAINING_WORK_ENABLED
        && miner
            .lock()
            .map_err(|_| "miner_worker_initial_work_lock_poisoned".to_owned())?
            .has_self_training_work();
    let initial_collection_maintenance_pending = collection_miner
        .lock()
        .map_err(|_| "miner_worker_initial_collection_work_lock_poisoned".to_owned())?
        .has_structural_resynthesis_work();
    let (initial_report, initial_status) = {
        let stream = miner
            .lock()
            .map_err(|_| "miner_worker_initial_report_lock_poisoned".to_owned())?;
        (stream.report(), stream.status())
    };
    let response_report = Arc::new(std::sync::RwLock::new(Some(initial_report)));
    let response_status = Arc::new(std::sync::RwLock::new(Some(initial_status)));
    let thread_counters = Arc::clone(&counters);
    let thread_response_report = Arc::clone(&response_report);
    let thread_response_status = Arc::clone(&response_status);
    thread::Builder::new()
        .name("nando-response-miner-v2".to_owned())
        .spawn(move || {
            let mut events_since_checkpoint = 0_u64;
            let mut inputs_since_synthesis = 0_u64;
            let mut last_checkpoint = Instant::now();
            let mut synthesis_pending = initial_synthesis_pending;
            let mut collection_maintenance_pending = initial_collection_maintenance_pending;
            let mut prefer_collection_maintenance = initial_collection_maintenance_pending;
            loop {
                let command = if synthesis_pending || collection_maintenance_pending {
                    match receiver.try_recv() {
                        Ok(command) => Some(command),
                        Err(TryRecvError::Empty) => None,
                        Err(TryRecvError::Disconnected) => break,
                    }
                } else if events_since_checkpoint > 0 {
                    let timeout = CHECKPOINT_INTERVAL
                        .checked_sub(last_checkpoint.elapsed())
                        .unwrap_or(Duration::ZERO);
                    match receiver.recv_timeout(timeout) {
                        Ok(command) => Some(command),
                        Err(RecvTimeoutError::Timeout) => None,
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                } else {
                    match receiver.recv() {
                        Ok(command) => Some(command),
                        Err(_) => break,
                    }
                };

                let input_was_available = command.is_some();
                match command {
                    Some(MinerCommand::Transition(transition)) => {
                        let started = Instant::now();
                        let result = teacher_ledger.append(&transition).and_then(|_| {
                            miner
                                .lock()
                                .map_err(|_| "miner_worker_lock_poisoned".to_owned())?
                                .apply_teacher_transition(*transition)
                                .map(|_| ())
                        });
                        record_timing(
                            &thread_counters.transition_last_micros,
                            &thread_counters.transition_max_micros,
                            &thread_counters.transition_total_micros,
                            elapsed_micros(started),
                        );
                        if let Err(error) = result {
                            thread_counters.failed.fetch_add(1, Ordering::Relaxed);
                            eprintln!("nando-response-miner-v2 event error: {error}");
                            continue;
                        }
                        thread_counters.processed.fetch_add(1, Ordering::Relaxed);
                        events_since_checkpoint = events_since_checkpoint.saturating_add(1);
                        synthesis_pending = EXPERIMENTAL_SELF_TRAINING_WORK_ENABLED;
                        if let Some(trigger) = &authority_trigger {
                            let _ = trigger.try_send(());
                        }
                    }
                    Some(MinerCommand::Collection(observation)) => {
                        let started = Instant::now();
                        let result = collection_ledger.append(&observation).and_then(|_| {
                            collection_miner
                                .lock()
                                .map_err(|_| "miner_worker_collection_lock_poisoned".to_owned())?
                                .observe_buffered(observation)
                        });
                        record_timing(
                            &thread_counters.collection_last_micros,
                            &thread_counters.collection_max_micros,
                            &thread_counters.collection_total_micros,
                            elapsed_micros(started),
                        );
                        if let Err(error) = result {
                            thread_counters.failed.fetch_add(1, Ordering::Relaxed);
                            eprintln!("nando-response-miner-v2 collection error: {error}");
                            continue;
                        }
                        thread_counters.processed.fetch_add(1, Ordering::Relaxed);
                        thread_counters
                            .collection_processed
                            .fetch_add(1, Ordering::Relaxed);
                        events_since_checkpoint = events_since_checkpoint.saturating_add(1);
                        if let Some(trigger) = &authority_trigger {
                            let _ = trigger.try_send(());
                        }
                    }
                    Some(MinerCommand::Opportunity(event)) => {
                        let started = Instant::now();
                        let result = opportunity_ledger.append(&event).and_then(|_| {
                            let mut stream = miner
                                .lock()
                                .map_err(|_| "miner_worker_opportunity_lock_poisoned".to_owned())?;
                            apply_opportunity_event(&mut stream, event);
                            Ok(())
                        });
                        record_timing(
                            &thread_counters.opportunity_last_micros,
                            &thread_counters.opportunity_max_micros,
                            &thread_counters.opportunity_total_micros,
                            elapsed_micros(started),
                        );
                        if let Err(error) = result {
                            thread_counters.failed.fetch_add(1, Ordering::Relaxed);
                            eprintln!("nando-response-miner-v2 opportunity error: {error}");
                            continue;
                        }
                        thread_counters.processed.fetch_add(1, Ordering::Relaxed);
                        thread_counters
                            .opportunity_processed
                            .fetch_add(1, Ordering::Relaxed);
                        events_since_checkpoint = events_since_checkpoint.saturating_add(1);
                    }
                    None => {}
                }

                if input_was_available {
                    inputs_since_synthesis = inputs_since_synthesis.saturating_add(1);
                }
                let slice_due =
                    !input_was_available || inputs_since_synthesis >= INPUTS_PER_SYNTHESIS_SLICE;
                let run_collection_maintenance = collection_maintenance_pending
                    && slice_due
                    && (!synthesis_pending || prefer_collection_maintenance);
                let run_synthesis = synthesis_pending && slice_due && !run_collection_maintenance;
                if run_synthesis {
                    let started = Instant::now();
                    let (checks, synthesis_work_remains) =
                        miner.lock().ok().map_or((0, false), |mut stream| {
                            let checks = stream.run_self_training_work_slice();
                            let synthesis_work_remains = stream.has_self_training_work();
                            (checks, synthesis_work_remains)
                        });
                    record_timing(
                        &thread_counters.synthesis_last_micros,
                        &thread_counters.synthesis_max_micros,
                        &thread_counters.synthesis_total_micros,
                        elapsed_micros(started),
                    );
                    inputs_since_synthesis = 0;
                    synthesis_pending = synthesis_work_remains;
                    prefer_collection_maintenance = collection_maintenance_pending;
                    if checks > 0 {
                        thread_counters
                            .synthesis_slices
                            .fetch_add(1, Ordering::Relaxed);
                        thread_counters.exact_checks.fetch_add(
                            u64::try_from(checks).unwrap_or(u64::MAX),
                            Ordering::Relaxed,
                        );
                        events_since_checkpoint = events_since_checkpoint.saturating_add(1);
                        if let Some(trigger) = &authority_trigger {
                            let _ = trigger.try_send(());
                        }
                        thread::sleep(Duration::from_millis(1));
                    }
                } else if run_collection_maintenance {
                    let started = Instant::now();
                    let result = collection_miner
                        .lock()
                        .map_err(|_| "miner_worker_collection_maintenance_lock_poisoned".to_owned())
                        .and_then(|mut collection| {
                            let programs_added =
                                collection.run_structural_resynthesis_work_slice()?;
                            Ok((programs_added, collection.has_structural_resynthesis_work()))
                        });
                    record_timing(
                        &thread_counters.collection_maintenance_last_micros,
                        &thread_counters.collection_maintenance_max_micros,
                        &thread_counters.collection_maintenance_total_micros,
                        elapsed_micros(started),
                    );
                    inputs_since_synthesis = 0;
                    prefer_collection_maintenance = false;
                    match result {
                        Ok((programs_added, work_remains)) => {
                            collection_maintenance_pending = work_remains;
                            thread_counters
                                .collection_maintenance_slices
                                .fetch_add(1, Ordering::Relaxed);
                            events_since_checkpoint = events_since_checkpoint.saturating_add(1);
                            if programs_added > 0
                                && let Some(trigger) = &authority_trigger
                            {
                                let _ = trigger.try_send(());
                            }
                            thread::sleep(Duration::from_millis(1));
                        }
                        Err(error) => {
                            collection_maintenance_pending =
                                collection_miner.lock().is_ok_and(|collection| {
                                    collection.has_structural_resynthesis_work()
                                });
                            thread_counters.failed.fetch_add(1, Ordering::Relaxed);
                            eprintln!("nando-response-miner-v2 collection maintenance: {error}");
                        }
                    }
                }

                if events_since_checkpoint >= CHECKPOINT_EVENTS
                    || (events_since_checkpoint > 0
                        && last_checkpoint.elapsed() >= CHECKPOINT_INTERVAL)
                {
                    let started = Instant::now();
                    let persisted = miner
                        .lock()
                        .map_err(|_| "miner_worker_checkpoint_lock_poisoned".to_owned())
                        .and_then(|mut stream| stream.persist_now())
                        .and_then(|()| {
                            collection_miner
                                .lock()
                                .map_err(|_| {
                                    "miner_worker_collection_checkpoint_lock_poisoned".to_owned()
                                })?
                                .flush()
                        })
                        .and_then(|()| teacher_ledger.compact_after_checkpoint())
                        .and_then(|()| collection_ledger.compact_after_checkpoint())
                        .and_then(|()| opportunity_ledger.compact_after_checkpoint());
                    record_timing(
                        &thread_counters.checkpoint_last_micros,
                        &thread_counters.checkpoint_max_micros,
                        &thread_counters.checkpoint_total_micros,
                        elapsed_micros(started),
                    );
                    match persisted {
                        Ok(()) => {
                            if let Ok(stream) = miner.lock() {
                                let report = stream.report();
                                let status = stream.status();
                                if let Ok(mut published) = thread_response_report.write() {
                                    *published = Some(report);
                                }
                                if let Ok(mut published) = thread_response_status.write() {
                                    *published = Some(status);
                                }
                            }
                            thread_counters.checkpoints.fetch_add(1, Ordering::Relaxed);
                            events_since_checkpoint = 0;
                            last_checkpoint = Instant::now();
                            if let Some(trigger) = &authority_trigger {
                                let _ = trigger.try_send(());
                            }
                        }
                        Err(error) => {
                            thread_counters.failed.fetch_add(1, Ordering::Relaxed);
                            last_checkpoint = Instant::now();
                            eprintln!("nando-response-miner-v2 checkpoint error: {error}");
                        }
                    }
                }
            }
            let _ = miner
                .lock()
                .map_err(|_| "miner_worker_shutdown_lock_poisoned".to_owned())
                .and_then(|mut stream| stream.persist_now());
            let _ = collection_miner
                .lock()
                .map_err(|_| "miner_worker_collection_shutdown_lock_poisoned".to_owned())
                .and_then(|collection| collection.flush());
            let _ = teacher_ledger.sync();
            let _ = collection_ledger.sync();
            let _ = opportunity_ledger.sync();
        })
        .map_err(|error| format!("miner_worker_spawn:{error}"))?;
    Ok(MinerWorkerHandle {
        sender,
        counters,
        response_report,
        response_status,
    })
}

fn apply_opportunity_event(stream: &mut OnlineResponseStream, event: MinerOpportunityEvent) {
    match event {
        MinerOpportunityEvent::Request {
            intent_sha256,
            input_tokens,
            now_unix,
        } => stream.observe_ordinary_request(&intent_sha256, input_tokens, now_unix),
        MinerOpportunityEvent::Classify {
            intent_sha256,
            class,
            blocker,
        } => stream.classify_ordinary_intent(&intent_sha256, class, Some(&blocker)),
        MinerOpportunityEvent::Verified { intent_sha256 } => {
            stream.mark_verified_ordinary_intent(&intent_sha256);
        }
        MinerOpportunityEvent::ParityFailure { intent_sha256 } => {
            stream.mark_self_training_parity_failure(&intent_sha256);
        }
        MinerOpportunityEvent::FalseAccept { intent_sha256 } => {
            stream.mark_self_training_false_accept(&intent_sha256);
        }
    }
}

fn elapsed_micros(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX)
}

fn record_timing(last: &AtomicU64, maximum: &AtomicU64, total: &AtomicU64, micros: u64) {
    last.store(micros, Ordering::Relaxed);
    maximum.fetch_max(micros, Ordering::Relaxed);
    let _ = total.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        Some(current.saturating_add(micros))
    });
}
