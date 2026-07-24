//! Streaming tail and checkpoint persistence for the online response miner.
//!
//! This module moves bytes and snapshots; it does not decide operator authority.

use super::*;

impl OnlineResponseStream {
    #[must_use]
    pub const fn checkpoint_restored(&self) -> bool {
        self.checkpoint_restored
    }

    #[must_use]
    pub const fn source_offset(&self) -> u64 {
        self.source_offset
    }

    #[must_use]
    pub const fn source_lines(&self) -> u64 {
        self.source_lines
    }

    #[must_use]
    pub fn replay_support_parity_cases_total(&self) -> usize {
        self.miner.replay_support_parity_cases_total()
    }

    /// Restores only the bounded miner checkpoint. Live V2 evidence arrives
    /// through framed worker segments, so production never scans the legacy
    /// relation JSON ledger during startup.
    pub fn open_streaming(config: OnlineResponseTailConfig) -> Result<Self, String> {
        if let Some(parent) = config.report_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("online_report_dir:{}:{error}", parent.display()))?;
        }
        if let Some(parent) = config.checkpoint_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("online_checkpoint_dir:{}:{error}", parent.display()))?;
        }
        let checkpoint_owner = acquire_online_checkpoint_owner(&config.checkpoint_path)?;
        let restored = decode_online_checkpoint(&config.checkpoint_path)?;
        let checkpoint_restored = restored.is_some();
        let checkpoint_needs_rewrite = restored.as_ref().is_some_and(|checkpoint| {
            checkpoint.bucket_strategy_version < ONLINE_BUCKET_STRATEGY_VERSION
        });
        let miner = match restored {
            Some(checkpoint) => OnlineResponseMiner::from_checkpoint(checkpoint)?,
            None => OnlineResponseMiner::new(OnlineResponseMinerConfig::default())?,
        };
        let mut stream = Self {
            config,
            _checkpoint_owner: checkpoint_owner,
            miner,
            source_device: 0,
            source_inode: 0,
            source_offset: 0,
            source_lines: 0,
            parse_errors: 0,
            source_prefix_hasher: Sha256::new(),
            checkpoint_restored,
            events_since_checkpoint: 0,
            last_checkpoint: Instant::now(),
        };
        if checkpoint_needs_rewrite || !checkpoint_restored {
            stream.persist()?;
        } else {
            write_online_report(&stream.config.report_path, 0, 0, 0, true, &stream.miner)?;
        }
        Ok(stream)
    }

    pub fn open(config: OnlineResponseTailConfig) -> Result<Self, String> {
        if let Some(parent) = config.input_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("online_source_dir:{}:{error}", parent.display()))?;
        }
        if let Some(parent) = config.report_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("online_report_dir:{}:{error}", parent.display()))?;
        }
        if let Some(parent) = config.checkpoint_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("online_checkpoint_dir:{}:{error}", parent.display()))?;
        }
        let checkpoint_owner = acquire_online_checkpoint_owner(&config.checkpoint_path)?;
        let source = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&config.input_path)
            .map_err(|error| {
                format!("online_source_open:{}:{error}", config.input_path.display())
            })?;
        let metadata = source
            .metadata()
            .map_err(|error| format!("online_source_metadata:{error}"))?;
        let (source_device, source_inode) = source_identity(&metadata);
        let restored = load_online_checkpoint(
            &config.checkpoint_path,
            &config.input_path,
            source_device,
            source_inode,
        )?;
        let (
            mut miner,
            source_offset,
            mut source_lines,
            mut parse_errors,
            checkpoint_restored,
            checkpoint_needs_rewrite,
        ) = if let Some(checkpoint) =
            restored.filter(|checkpoint| checkpoint.source_offset <= metadata.len())
        {
            let offset = checkpoint.source_offset;
            let lines = checkpoint.source_lines;
            let errors = checkpoint.parse_errors;
            let needs_rewrite = checkpoint.bucket_strategy_version < ONLINE_BUCKET_STRATEGY_VERSION;
            (
                OnlineResponseMiner::from_checkpoint(checkpoint)?,
                offset,
                lines,
                errors,
                true,
                needs_rewrite,
            )
        } else {
            (
                OnlineResponseMiner::new(OnlineResponseMinerConfig::default())?,
                0,
                0,
                0,
                false,
                false,
            )
        };
        let mut reader = BufReader::new(source);
        reader
            .seek(SeekFrom::Start(source_offset))
            .map_err(|error| format!("online_source_seek:{error}"))?;
        let mut line = String::new();
        loop {
            line.clear();
            let position = reader
                .stream_position()
                .map_err(|error| format!("online_source_position:{error}"))?;
            let bytes = reader
                .read_line(&mut line)
                .map_err(|error| format!("online_source_read:{error}"))?;
            if bytes == 0 {
                let source_prefix_hasher = hash_source_prefix(&config.input_path, position)?;
                let mut stream = Self {
                    config,
                    _checkpoint_owner: checkpoint_owner,
                    miner,
                    source_device,
                    source_inode,
                    source_offset: position,
                    source_lines,
                    parse_errors,
                    source_prefix_hasher,
                    checkpoint_restored,
                    events_since_checkpoint: 0,
                    last_checkpoint: Instant::now(),
                };
                if !checkpoint_restored || checkpoint_needs_rewrite || position != source_offset {
                    stream.persist()?;
                } else {
                    write_online_report(
                        &stream.config.report_path,
                        stream.source_lines,
                        stream.parse_errors,
                        stream.source_offset,
                        true,
                        &stream.miner,
                    )?;
                }
                return Ok(stream);
            }
            if !line.ends_with('\n') {
                break;
            }
            source_lines = source_lines.saturating_add(1);
            match serde_json::from_str::<RelationFrame>(line.trim_end()) {
                Ok(frame) if checkpoint_restored => miner.observe_frame(frame)?,
                Ok(frame) => match miner.replay_chronological_frame(frame) {
                    Ok(()) => {}
                    Err(error) if error == "online_frame_id_content_conflict" => {
                        parse_errors = parse_errors.saturating_add(1);
                    }
                    Err(error) => return Err(error),
                },
                Err(_) => parse_errors = parse_errors.saturating_add(1),
            }
        }
        Err("online_source_partial_line_at_startup".to_owned())
    }

    /// Durably appends one canonical frame before applying it to mutable miner state.
    /// A failure after the append is recovered by replaying from the last checkpoint.
    pub fn ingest(
        &mut self,
        mut frame: RelationFrame,
    ) -> Result<OnlineResponseIngestResult, String> {
        canonicalize_online_frame(&mut frame);
        if self.miner.frame_disposition(&frame)? == FrameDisposition::Duplicate {
            return Ok(self.miner.ingest_result());
        }
        let mut bytes = crate::canonical_json_bytes(&frame)
            .map_err(|error| format!("online_audit_encode:{error}"))?;
        bytes.push(b'\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.input_path)
            .map_err(|error| format!("online_audit_open:{error}"))?;
        file.write_all(&bytes)
            .map_err(|error| format!("online_audit_write:{error}"))?;
        file.sync_data()
            .map_err(|error| format!("online_audit_sync:{error}"))?;
        self.source_prefix_hasher.update(&bytes);
        self.source_offset = file
            .metadata()
            .map_err(|error| format!("online_audit_metadata:{error}"))?
            .len();
        self.source_lines = self.source_lines.saturating_add(1);
        self.miner.observe_frame(frame)?;
        self.events_since_checkpoint = self.events_since_checkpoint.saturating_add(1);
        if self.events_since_checkpoint >= 64
            || self.last_checkpoint.elapsed() >= Duration::from_secs(5)
        {
            self.persist()?;
        }
        Ok(self.miner.ingest_result())
    }

    /// Applies a transition already made durable by the framed V2 worker.
    /// This path never appends to the legacy JSON relation ledger.
    pub fn apply_teacher_transition(
        &mut self,
        transition: crate::TeacherTransition,
    ) -> Result<OnlineResponseIngestResult, String> {
        self.miner.observe_teacher_transition(transition)?;
        self.events_since_checkpoint = self.events_since_checkpoint.saturating_add(1);
        Ok(self.miner.ingest_result())
    }

    pub fn run_self_training_work_slice(&mut self) -> usize {
        self.miner.self_training_v2.run_work_slice()
    }

    pub fn run_self_training_work_slice_with_progress(&mut self) -> (usize, bool) {
        self.miner.self_training_v2.run_work_slice_with_progress()
    }

    pub fn run_self_training_work_slice_for_signatures(
        &mut self,
        signatures: &BTreeSet<String>,
    ) -> usize {
        self.miner
            .self_training_v2
            .run_work_slice_for_signatures(signatures)
    }

    #[must_use]
    pub fn has_self_training_work(&self) -> bool {
        self.miner.self_training_v2.has_pending_work()
    }

    #[must_use]
    pub fn has_self_training_work_for_signatures(&self, signatures: &BTreeSet<String>) -> bool {
        self.miner
            .self_training_v2
            .has_pending_work_for_signatures(signatures)
    }

    #[must_use]
    pub fn semantic_law_evidence_audit(
        &self,
        signatures: &BTreeSet<String>,
    ) -> crate::SemanticLawEvidenceAudit {
        self.miner
            .self_training_v2
            .semantic_law_evidence_audit(signatures)
    }

    pub fn semantic_law_binding_evidence_report(
        &self,
        signatures: &BTreeSet<String>,
    ) -> Result<crate::BindingVersionSpaceReportV1, String> {
        self.miner
            .self_training_v2
            .semantic_law_binding_evidence_report(signatures)
    }

    pub fn persist_now(&mut self) -> Result<(), String> {
        self.persist()
    }

    #[must_use]
    pub fn report_path(&self) -> &Path {
        &self.config.report_path
    }

    /// Refreshes the liveness timestamp of an unchanged report without
    /// rebuilding miner diagnostics or rewriting the semantic checkpoint.
    pub fn refresh_report_heartbeat(&self) -> Result<(), String> {
        refresh_online_report_heartbeat(&self.config.report_path)
    }

    /// Refreshes an already-persisted report without acquiring the miner lock.
    pub fn refresh_report_heartbeat_at(path: &Path) -> Result<(), String> {
        refresh_online_report_heartbeat(path)
    }

    pub fn observe_ordinary_request(
        &mut self,
        intent_sha256: &str,
        input_tokens: u64,
        now_unix: u64,
    ) {
        self.miner
            .self_training_v2
            .observe_ordinary_request(intent_sha256, input_tokens, now_unix);
        self.events_since_checkpoint = self.events_since_checkpoint.saturating_add(1);
    }

    pub fn classify_ordinary_intent(
        &mut self,
        intent_sha256: &str,
        class: crate::ReducibilityClass,
        blocker: Option<&str>,
    ) {
        self.miner
            .self_training_v2
            .classify_intent(intent_sha256, class, blocker);
        self.events_since_checkpoint = self.events_since_checkpoint.saturating_add(1);
    }

    pub fn mark_verified_ordinary_intent(&mut self, intent_sha256: &str) {
        self.miner
            .self_training_v2
            .mark_verified_intent(intent_sha256);
        self.events_since_checkpoint = self.events_since_checkpoint.saturating_add(1);
    }

    pub fn mark_self_training_false_accept(&mut self, intent_sha256: &str) {
        self.miner.self_training_v2.mark_false_accept(intent_sha256);
        self.events_since_checkpoint = self.events_since_checkpoint.saturating_add(1);
    }

    pub fn mark_self_training_parity_failure(&mut self, intent_sha256: &str) {
        self.miner
            .self_training_v2
            .mark_parity_failure(intent_sha256);
        self.events_since_checkpoint = self.events_since_checkpoint.saturating_add(1);
    }

    /// Appends a replay batch with one durability barrier, then observes rows
    /// in source order. If the process stops after the append, normal checkpoint
    /// recovery observes the committed tail with the same semantics.
    pub fn ingest_batch<I>(&mut self, frames: I) -> Result<OnlineResponseIngestResult, String>
    where
        I: IntoIterator<Item = RelationFrame>,
    {
        let mut batch_ids = BTreeMap::<String, String>::new();
        let mut accepted = Vec::new();
        let mut bytes = Vec::new();
        for frame in frames {
            let digest = crate::relation_frame_learning_digest(&frame)
                .map_err(|error| format!("online_frame_digest:{error}"))?;
            match self.miner.frame_disposition(&frame)? {
                FrameDisposition::Duplicate => continue,
                FrameDisposition::New => {}
            }
            match batch_ids.get(&frame.frame_id_sha256) {
                Some(existing) if existing == &digest => continue,
                Some(_) => return Err("online_frame_id_content_conflict".to_owned()),
                None => {
                    batch_ids.insert(frame.frame_id_sha256.clone(), digest);
                }
            }
            let mut encoded = crate::canonical_json_bytes(&frame)
                .map_err(|error| format!("online_audit_encode:{error}"))?;
            encoded.push(b'\n');
            bytes.extend_from_slice(&encoded);
            accepted.push(frame);
        }
        if accepted.is_empty() {
            return Ok(self.miner.ingest_result());
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.input_path)
            .map_err(|error| format!("online_audit_open:{error}"))?;
        file.write_all(&bytes)
            .map_err(|error| format!("online_audit_write:{error}"))?;
        file.sync_data()
            .map_err(|error| format!("online_audit_sync:{error}"))?;
        self.source_prefix_hasher.update(&bytes);
        self.source_offset = file
            .metadata()
            .map_err(|error| format!("online_audit_metadata:{error}"))?
            .len();
        self.source_lines = self
            .source_lines
            .saturating_add(u64::try_from(accepted.len()).unwrap_or(u64::MAX));
        for frame in accepted {
            self.miner.observe_frame(frame)?;
        }
        self.persist()?;
        Ok(self.miner.ingest_result())
    }

    /// Trains from replayable source history without appending it to the live
    /// audit or claiming frozen-future evidence.
    pub fn train_replay_batch<I>(&mut self, frames: I) -> Result<OnlineResponseIngestResult, String>
    where
        I: IntoIterator<Item = RelationFrame>,
    {
        self.train_replay_cases_batch(frames.into_iter().map(|frame| (frame, None)))
    }

    /// Imports immutable history as support evidence while retaining an
    /// independently reconstructed runtime parity case when one is available.
    pub fn train_replay_cases_batch<I>(
        &mut self,
        cases: I,
    ) -> Result<OnlineResponseIngestResult, String>
    where
        I: IntoIterator<Item = (RelationFrame, Option<crate::RuntimeParityCase>)>,
    {
        let result = self.train_replay_cases_batch_buffered(cases)?;
        self.persist()?;
        Ok(result)
    }

    /// Imports support-only replay cases without an intermediate checkpoint.
    /// Callers must persist after their bounded synthesis work completes.
    pub fn train_replay_cases_batch_buffered<I>(
        &mut self,
        cases: I,
    ) -> Result<OnlineResponseIngestResult, String>
    where
        I: IntoIterator<Item = (RelationFrame, Option<crate::RuntimeParityCase>)>,
    {
        let mut imported_signatures = BTreeSet::new();
        for (frame, runtime_parity_case) in cases {
            let economics = (frame.estimated_input_tokens > 0).then(|| EconomicsReceipt {
                schema: ECONOMICS_RECEIPT_SCHEMA_V1.to_owned(),
                exact_input_tokens: frame.estimated_input_tokens,
                ordinary: false,
                controlled: false,
                replay: true,
                dedupe_eligible: true,
                provider_evidence_ref_sha256: frame.evidence_ref_sha256.clone(),
            });
            let mut transition = teacher_transition_from_completed(&frame, economics)
                .map_err(|error| format!("online_replay_teacher_transition:{error:?}"))?;
            transition.runtime_parity_case = runtime_parity_case;
            match self.miner.import_teacher_transition(transition) {
                Ok(Some(signature)) => {
                    imported_signatures.insert(signature);
                }
                Ok(None) => {}
                Err(error) if error == "online_frame_id_content_conflict" => {}
                Err(error) => return Err(error),
            }
        }
        self.miner
            .self_training_v2
            .prepare_incremental_replay_seed(imported_signatures);
        Ok(self.miner.ingest_result())
    }

    #[must_use]
    pub fn report(&self) -> OnlineResponseMinerReport {
        self.miner.report()
    }

    #[must_use]
    pub fn admission_candidates(&self) -> Vec<OnlineResponseAdmissionCandidate> {
        self.miner.admission_candidates()
    }

    #[must_use]
    pub fn crystallized_admission_candidates(&self) -> Vec<crate::LiveScalarAdmissionCandidate> {
        self.miner.crystallized_admission_candidates()
    }

    #[must_use]
    pub fn has_admission_candidates(&self) -> bool {
        !self.miner.admission_candidates().is_empty()
    }

    #[must_use]
    pub fn status(&self) -> OnlineResponseStreamStatus {
        let ingest = self.miner.ingest_result();
        let v2 = self.miner.self_training_v2.report(unix_now_seconds());
        let max_frozen_future_rows = v2
            .generations
            .iter()
            .map(|generation| generation.future_rows)
            .max()
            .unwrap_or(0);
        let v2_warm_bytes = v2.discovery.warm_bytes_estimate.saturating_add(
            v2.cegis
                .pools
                .iter()
                .map(|pool| pool.ast_nodes.saturating_mul(256))
                .sum::<usize>(),
        );
        let class_tokens = |class: &str| {
            v2.opportunity
                .classes
                .get(class)
                .map_or(0, |report| report.input_tokens)
        };
        OnlineResponseStreamStatus {
            checkpoint_restored: self.checkpoint_restored,
            rows_seen: ingest.rows_seen,
            rows_learned: ingest.rows_learned,
            bucket_count: ingest.bucket_count,
            candidate_bucket_count: ingest.candidate_bucket_count,
            false_accepts: ingest.false_accepts,
            warm_bytes_estimate: v2_warm_bytes,
            source_lines: self.source_lines,
            source_offset: self.source_offset,
            cegis_cohorts: v2.cegis.cohorts,
            cegis_winners: v2.cegis.winners,
            max_frozen_future_rows,
            signal_score_out_of_10: v2.signal_tree.overall_score_out_of_10,
            opportunity_ordinary_intents: v2.opportunity.ordinary_intents,
            opportunity_ordinary_tokens: v2.opportunity.ordinary_tokens,
            opportunity_verified_tokens: v2.opportunity.verified_tokens,
            opportunity_verified_share_milli: v2.opportunity.verified_token_share_milli,
            opportunity_executable_candidate_tokens: class_tokens("EXECUTABLE_CANDIDATE"),
            opportunity_missing_dsl_tokens: class_tokens("MISSING_DSL_PRIMITIVE"),
            opportunity_missing_verifier_tokens: class_tokens("MISSING_EXTERNAL_VERIFIER"),
            opportunity_insufficient_repetition_tokens: class_tokens("INSUFFICIENT_REPETITION"),
            opportunity_unexplored_multi_source_tokens: class_tokens("UNEXPLORED_MULTI_SOURCE"),
            opportunity_ambiguous_tokens: class_tokens("AMBIGUOUS_PRE_ACTION_STATE"),
            opportunity_non_deterministic_tokens: class_tokens("NON_DETERMINISTIC_OR_CREATIVE"),
            opportunity_unresolved_tokens: v2.opportunity.unresolved_tokens,
            opportunity_upper_bound_share_milli: v2
                .opportunity
                .optimistic_executable_upper_bound_share_milli,
            opportunity_accounting_identity_holds: v2.opportunity.classification_identity_holds
                && v2.opportunity.upper_bound_identity_holds,
            opportunity_m3_reachable: v2.opportunity.m3_reachable_under_upper_bound,
        }
    }

    pub fn persist(&mut self) -> Result<(), String> {
        let source_prefix_sha256 = format!("{:x}", self.source_prefix_hasher.clone().finalize());
        write_online_checkpoint(
            &self.config.checkpoint_path,
            self.source_device,
            self.source_inode,
            self.source_offset,
            &source_prefix_sha256,
            self.source_lines,
            self.parse_errors,
            &self.miner,
        )?;
        write_online_report(
            &self.config.report_path,
            self.source_lines,
            self.parse_errors,
            self.source_offset,
            self.checkpoint_restored,
            &self.miner,
        )?;
        self.events_since_checkpoint = 0;
        self.last_checkpoint = Instant::now();
        Ok(())
    }
}

pub fn run_online_response_tail(config: OnlineResponseTailConfig) -> Result<(), String> {
    if let Some(parent) = config.report_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("online_report_dir:{}:{error}", parent.display()))?;
    }
    if let Some(parent) = config.checkpoint_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("online_checkpoint_dir:{}:{error}", parent.display()))?;
    }
    let _checkpoint_owner = acquire_online_checkpoint_owner(&config.checkpoint_path)?;
    let source = OpenOptions::new()
        .read(true)
        .open(&config.input_path)
        .map_err(|error| format!("online_source_open:{}:{error}", config.input_path.display()))?;
    let source_metadata = source
        .metadata()
        .map_err(|error| format!("online_source_metadata:{error}"))?;
    let (mut source_device, mut source_inode) = source_identity(&source_metadata);
    let restored = load_online_checkpoint(
        &config.checkpoint_path,
        &config.input_path,
        source_device,
        source_inode,
    )?;
    let (mut miner, source_offset, mut source_lines, mut parse_errors, checkpoint_restored) =
        if let Some(checkpoint) =
            restored.filter(|checkpoint| checkpoint.source_offset <= source_metadata.len())
        {
            let checkpoint_offset = checkpoint.source_offset;
            let checkpoint_lines = checkpoint.source_lines;
            let checkpoint_parse_errors = checkpoint.parse_errors;
            (
                OnlineResponseMiner::from_checkpoint(checkpoint)?,
                checkpoint_offset,
                checkpoint_lines,
                checkpoint_parse_errors,
                true,
            )
        } else {
            (
                OnlineResponseMiner::new(OnlineResponseMinerConfig::default())?,
                0,
                0,
                0,
                false,
            )
        };
    let mut reader = BufReader::new(source);
    reader
        .seek(SeekFrom::Start(source_offset))
        .map_err(|error| format!("online_source_seek:{error}"))?;
    let mut source_prefix_hasher = hash_source_prefix(&config.input_path, source_offset)?;
    let mut line = String::new();
    let mut following = checkpoint_restored;
    let mut last_report = Instant::now();
    loop {
        line.clear();
        let position = reader
            .stream_position()
            .map_err(|error| format!("online_source_position:{error}"))?;
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("online_source_read:{error}"))?;
        if bytes == 0 {
            if !following || last_report.elapsed() >= Duration::from_secs(5) {
                following = true;
                write_online_checkpoint(
                    &config.checkpoint_path,
                    source_device,
                    source_inode,
                    position,
                    &format!("{:x}", source_prefix_hasher.clone().finalize()),
                    source_lines,
                    parse_errors,
                    &miner,
                )?;
                write_online_report(
                    &config.report_path,
                    source_lines,
                    parse_errors,
                    position,
                    checkpoint_restored,
                    &miner,
                )?;
                last_report = Instant::now();
            }
            let length = fs::metadata(&config.input_path)
                .map(|metadata| metadata.len())
                .unwrap_or(position);
            if length < position {
                let source = OpenOptions::new()
                    .read(true)
                    .open(&config.input_path)
                    .map_err(|error| format!("online_source_reopen:{error}"))?;
                let metadata = source
                    .metadata()
                    .map_err(|error| format!("online_source_reopen_metadata:{error}"))?;
                (source_device, source_inode) = source_identity(&metadata);
                reader = BufReader::new(source);
                source_prefix_hasher = Sha256::new();
                following = true;
            } else {
                thread::sleep(config.idle_sleep);
            }
            continue;
        }
        if !line.ends_with('\n') {
            reader
                .seek(SeekFrom::Start(position))
                .map_err(|error| format!("online_source_partial_rewind:{error}"))?;
            thread::sleep(config.idle_sleep);
            continue;
        }
        source_prefix_hasher.update(line.as_bytes());
        source_lines = source_lines.saturating_add(1);
        match serde_json::from_str::<RelationFrame>(line.trim_end()) {
            Ok(frame) if following => miner.observe_frame(frame)?,
            Ok(frame) => miner.replay_chronological_frame(frame)?,
            Err(_) => parse_errors = parse_errors.saturating_add(1),
        }
    }
}

fn write_online_report(
    path: &Path,
    source_lines: u64,
    parse_errors: u64,
    source_offset: u64,
    checkpoint_restored: bool,
    miner: &OnlineResponseMiner,
) -> Result<(), String> {
    let generated_at_unix_ms = unix_now_millis();
    let value = serde_json::json!({
        "schema": "nando.embedded-response-online-miner.v1",
        "generated_at_unix_ms": generated_at_unix_ms,
        "state_generated_at_unix_ms": generated_at_unix_ms,
        "source_lines": source_lines,
        "parse_errors": parse_errors,
        "source_offset": source_offset,
        "checkpoint_restored": checkpoint_restored,
        "tail_follow_active": true,
        "execution_authority": false,
        "miner": miner.report(),
    });
    write_online_report_value(path, &value)
}

fn refresh_online_report_heartbeat(path: &Path) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|error| format!("online_report_heartbeat_read:{error}"))?;
    let mut value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("online_report_heartbeat_decode:{error}"))?;
    if value.get("schema").and_then(serde_json::Value::as_str)
        != Some("nando.embedded-response-online-miner.v1")
        || !value.get("miner").is_some_and(serde_json::Value::is_object)
    {
        return Err("online_report_heartbeat_invalid_snapshot".to_owned());
    }
    let object = value
        .as_object_mut()
        .ok_or_else(|| "online_report_heartbeat_not_object".to_owned())?;
    let previous_generated_at = object
        .get("generated_at_unix_ms")
        .cloned()
        .unwrap_or_else(|| serde_json::json!(0));
    object
        .entry("state_generated_at_unix_ms")
        .or_insert(previous_generated_at);
    object.insert(
        "generated_at_unix_ms".to_owned(),
        serde_json::json!(unix_now_millis()),
    );
    write_online_report_value(path, &value)
}

fn write_online_report_value(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(&value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut file = File::create(&temporary)
        .map_err(|error| format!("online_report_create:{}:{error}", temporary.display()))?;
    file.write_all(&bytes)
        .map_err(|error| format!("online_report_write:{error}"))?;
    file.sync_all()
        .map_err(|error| format!("online_report_sync:{error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("online_report_rename:{error}"))?;
    sync_parent_directory(path, "online_report_dir_sync")
}

fn unix_now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn load_online_checkpoint(
    path: &Path,
    source_path: &Path,
    source_device: u64,
    source_inode: u64,
) -> Result<Option<OnlineResponseCheckpoint>, String> {
    let Some(checkpoint) = decode_online_checkpoint(path)? else {
        return Ok(None);
    };
    if checkpoint.source_device != source_device || checkpoint.source_inode != source_inode {
        return Ok(None);
    }
    let actual = format!(
        "{:x}",
        hash_source_prefix(source_path, checkpoint.source_offset)?.finalize()
    );
    if checkpoint.source_prefix_sha256.len() != 64 || checkpoint.source_prefix_sha256 != actual {
        return Err("online_checkpoint_source_prefix_mismatch".to_owned());
    }
    Ok(Some(checkpoint))
}

fn decode_online_checkpoint(path: &Path) -> Result<Option<OnlineResponseCheckpoint>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("online_checkpoint_read:{}:{error}", path.display())),
    };
    let checkpoint: OnlineResponseCheckpoint =
        if let Some(payload) = bytes.strip_prefix(ONLINE_CHECKPOINT_MAGIC_V3) {
            serde_cbor::from_slice(payload)
                .map_err(|error| format!("online_checkpoint_decode:{error}"))?
        } else {
            serde_json::from_slice(&bytes)
                .map_err(|error| format!("online_checkpoint_legacy_decode:{error}"))?
        };
    if !matches!(
        checkpoint.schema.as_str(),
        "nando.online-response-checkpoint.v1"
            | "nando.online-response-checkpoint.v2"
            | "nando.online-response-checkpoint.v3"
    ) {
        return Ok(None);
    }
    Ok(Some(checkpoint))
}

#[allow(clippy::too_many_arguments)]
fn write_online_checkpoint(
    path: &Path,
    source_device: u64,
    source_inode: u64,
    source_offset: u64,
    source_prefix_sha256: &str,
    source_lines: u64,
    parse_errors: u64,
    miner: &OnlineResponseMiner,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("online_checkpoint_dir:{}:{error}", parent.display()))?;
    }
    let bytes = miner.checkpoint_bytes(
        source_device,
        source_inode,
        source_offset,
        source_prefix_sha256,
        source_lines,
        parse_errors,
    )?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|error| format!("online_checkpoint_create:{}:{error}", temporary.display()))?;
    file.write_all(&bytes)
        .map_err(|error| format!("online_checkpoint_write:{error}"))?;
    file.sync_all()
        .map_err(|error| format!("online_checkpoint_sync:{error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("online_checkpoint_rename:{error}"))?;
    sync_parent_directory(path, "online_checkpoint_dir_sync")
}

fn acquire_online_checkpoint_owner(checkpoint_path: &Path) -> Result<File, String> {
    let lock_path = checkpoint_path.with_extension("owner.lock");
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(&lock_path)
        .map_err(|error| {
            format!(
                "online_checkpoint_owner_open:{}:{error}",
                lock_path.display()
            )
        })?;
    file.try_lock().map_err(|error| {
        format!(
            "online_checkpoint_owned:{}:{error}",
            checkpoint_path.display()
        )
    })?;
    Ok(file)
}

fn hash_source_prefix(path: &Path, prefix_len: u64) -> Result<Sha256, String> {
    let mut file = File::open(path)
        .map_err(|error| format!("online_source_hash_open:{}:{error}", path.display()))?;
    if file
        .metadata()
        .map_err(|error| format!("online_source_hash_metadata:{error}"))?
        .len()
        < prefix_len
    {
        return Err("online_source_hash_prefix_beyond_end".to_owned());
    }
    let mut hasher = Sha256::new();
    let mut remaining = prefix_len;
    let mut buffer = [0_u8; 64 * 1024];
    while remaining > 0 {
        let limit = usize::try_from(remaining.min(buffer.len() as u64)).unwrap_or(buffer.len());
        let read = file
            .read(&mut buffer[..limit])
            .map_err(|error| format!("online_source_hash_read:{error}"))?;
        if read == 0 {
            return Err("online_source_hash_unexpected_eof".to_owned());
        }
        hasher.update(&buffer[..read]);
        remaining = remaining.saturating_sub(read as u64);
    }
    Ok(hasher)
}

fn sync_parent_directory(path: &Path, error_prefix: &str) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("{error_prefix}:{}:{error}", parent.display()))
}

#[cfg(unix)]
fn source_identity(metadata: &fs::Metadata) -> (u64, u64) {
    (metadata.dev(), metadata.ino())
}

#[cfg(not(unix))]
fn source_identity(_metadata: &fs::Metadata) -> (u64, u64) {
    (0, 0)
}
