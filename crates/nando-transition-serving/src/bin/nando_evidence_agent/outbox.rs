#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RouteBoundOutboxFrameV1 {
    schema: String,
    route_receipt_root_sha256: String,
    route_receipt: NandoRouteReceiptV1,
    frame: RelationFrame,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runtime_parity_case: Option<RuntimeParityCase>,
}

struct LocalEvidenceOutbox {
    directory: PathBuf,
    ledger: FramedCborLedger,
    frames: BTreeMap<String, nando_operator_learning::FramedRecordRef>,
    payload_bytes: u64,
}

struct OutboxSink {
    outbox: Arc<Mutex<LocalEvidenceOutbox>>,
    transport_censors: Arc<Mutex<TransportCensorLedger>>,
    route_receipts: Arc<Mutex<NandoRouteReceiptIndex>>,
    route_metrics: Arc<RouteBindingMetrics>,
}

#[derive(Default)]
struct RouteBindingMetrics {
    route_bound_frames: AtomicU64,
    route_unbound_frames: AtomicU64,
    route_receipt_refresh_failures: AtomicU64,
    transport_censored_frames: AtomicU64,
}

impl LocalEvidenceOutbox {
    fn open(directory: &Path) -> Result<Self, String> {
        let ledger = FramedCborLedger::open(directory, OUTBOX_PREFIX)?;
        let mut frames = BTreeMap::<String, nando_operator_learning::FramedRecordRef>::new();
        let mut payload_bytes = 0_u64;
        nando_operator_learning::visit_framed_cbor::<RouteBoundOutboxFrameV1, _>(
            directory,
            OUTBOX_PREFIX,
            |record, bound| {
                let sealed = bound.seal()?;
                let new_root = !frames.contains_key(&sealed.frame_root_sha256);
                if let Some(existing_ref) = frames.get(&sealed.frame_root_sha256) {
                    let existing = nando_operator_learning::read_framed_cbor_record::<
                        RouteBoundOutboxFrameV1,
                    >(directory, OUTBOX_PREFIX, existing_ref)?;
                    if existing == bound {
                        return Ok(());
                    }
                    if !existing.same_transport_binding(&bound)
                        || existing.runtime_parity_case.is_some()
                        || bound.runtime_parity_case.is_none()
                    {
                        return Err("evidence_agent_outbox_rebound".to_owned());
                    }
                }
                if new_root && frames.len() >= MAX_OUTBOX_FRAMES
                    || payload_bytes.saturating_add(u64::from(record.payload_bytes))
                        > MAX_OUTBOX_BYTES
                {
                    return Err("evidence_agent_outbox_budget".to_owned());
                }
                payload_bytes = payload_bytes.saturating_add(u64::from(record.payload_bytes));
                frames.insert(sealed.frame_root_sha256, record);
                Ok(())
            },
        )?;
        Ok(Self {
            directory: directory.to_owned(),
            ledger,
            frames,
            payload_bytes,
        })
    }

    #[cfg(test)]
    fn append(
        &mut self,
        frame: RelationFrame,
        route_receipt: NandoRouteReceiptV1,
        runtime_parity_case: Option<RuntimeParityCase>,
    ) -> Result<(), String> {
        let bound = RouteBoundOutboxFrameV1 {
            schema: ROUTE_BOUND_OUTBOX_SCHEMA_V1.to_owned(),
            route_receipt_root_sha256: route_receipt.receipt_root_sha256.clone(),
            route_receipt,
            frame,
            runtime_parity_case,
        };
        let sealed = bound.seal()?;
        self.append_sealed(bound, sealed)
    }

    fn append_sealed(
        &mut self,
        bound: RouteBoundOutboxFrameV1,
        sealed: RemoteEvidenceFrameV1,
    ) -> Result<(), String> {
        let new_root = !self.frames.contains_key(&sealed.frame_root_sha256);
        if let Some(existing_ref) = self.frames.get(&sealed.frame_root_sha256) {
            let existing = self.read_record(existing_ref)?;
            if existing == bound
                || existing.same_transport_binding(&bound)
                    && existing.runtime_parity_case.is_some()
                    && bound.runtime_parity_case.is_none()
            {
                return Ok(());
            }
            if !existing.same_transport_binding(&bound)
                || existing.runtime_parity_case.is_some()
                || bound.runtime_parity_case.is_none()
            {
                return Err("evidence_agent_outbox_rebound".to_owned());
            }
        }
        let bytes = frame_bytes(&bound)?;
        if new_root && self.frames.len() >= MAX_OUTBOX_FRAMES
            || self.payload_bytes.saturating_add(bytes) > MAX_OUTBOX_BYTES
        {
            return Err("evidence_agent_outbox_budget".to_owned());
        }
        let record = self.ledger.append(&bound)?;
        self.ledger.sync()?;
        self.payload_bytes = self.payload_bytes.saturating_add(bytes);
        self.frames.insert(sealed.frame_root_sha256, record);
        Ok(())
    }

    fn read_record(
        &self,
        record: &nando_operator_learning::FramedRecordRef,
    ) -> Result<RouteBoundOutboxFrameV1, String> {
        nando_operator_learning::read_framed_cbor_record(
            &self.directory,
            OUTBOX_PREFIX,
            record,
        )
    }

    fn selected_unseen(
        &self,
        seen: &BTreeSet<String>,
        limit: usize,
        max_payload_bytes: usize,
    ) -> Result<Vec<(String, RouteBoundOutboxFrameV1)>, String> {
        let mut selected = Vec::new();
        let mut selected_bytes = 0_usize;
        for (root, record) in self.frames.iter().filter(|(root, _)| !seen.contains(*root)) {
            if selected.len() >= limit {
                break;
            }
            let record_bytes = usize::try_from(record.payload_bytes)
                .map_err(|_| "evidence_agent_outbox_frame_budget".to_owned())?;
            if !selected.is_empty()
                && selected_bytes.saturating_add(record_bytes) > max_payload_bytes
            {
                break;
            }
            selected_bytes = selected_bytes.saturating_add(record_bytes);
            selected.push((root.clone(), self.read_record(record)?));
        }
        Ok(selected)
    }

    fn compact_all(&mut self) -> Result<(), String> {
        self.ledger.compact_after_checkpoint()?;
        self.frames.clear();
        self.payload_bytes = 0;
        Ok(())
    }

    #[cfg(test)]
    fn materialized_frames(&self) -> Result<Vec<RouteBoundOutboxFrameV1>, String> {
        self.frames
            .values()
            .map(|record| self.read_record(record))
            .collect()
    }
}

impl OutboxSink {
    fn append_route_bound_frame(
        &self,
        frame: RelationFrame,
        route_receipt: NandoRouteReceiptV1,
        runtime_parity_case: Option<RuntimeParityCase>,
    ) -> Result<(), String> {
        let bound = RouteBoundOutboxFrameV1::new(frame, route_receipt, runtime_parity_case);
        let sealed = match bound.seal_classified() {
            Ok(sealed) => sealed,
            Err(error) => {
                let Some(blocker) = error.censor_blocker() else {
                    return Err(error.to_string());
                };
                self.transport_censors
                    .lock()
                    .map_err(|_| "evidence_agent_transport_censor_lock_poisoned".to_owned())?
                    .append(&bound, blocker)?;
                self.route_metrics
                    .transport_censored_frames
                    .fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "nando-evidence-agent: transport frame censored blocker={} frame={}",
                    blocker.code(),
                    bound.frame.frame_id_sha256
                );
                return Ok(());
            }
        };
        self.outbox
            .lock()
            .map_err(|_| "evidence_agent_outbox_lock_poisoned".to_owned())?
            .append_sealed(bound, sealed)?;
        self.route_metrics
            .route_bound_frames
            .fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

impl VerifiedRelationFrameSink for OutboxSink {
    fn append_verified_frame_with_parity(
        &self,
        frame: RelationFrame,
        runtime_parity_case: Option<RuntimeParityCase>,
    ) -> Result<(), String> {
        let route_receipt = {
            let mut receipts = self
                .route_receipts
                .lock()
                .map_err(|_| "evidence_agent_route_receipt_lock_poisoned".to_owned())?;
            if let Err(error) = receipts.refresh() {
                self.route_metrics
                    .route_receipt_refresh_failures
                    .fetch_add(1, Ordering::Relaxed);
                return Err(error);
            }
            receipts.receipt_for_frame(
                &frame.client_intent_id_sha256,
                &frame.session_id_sha256,
                frame.observed_at_unix_nanos,
            )?
        };
        let Some(route_receipt) = route_receipt else {
            self.route_metrics
                .route_unbound_frames
                .fetch_add(1, Ordering::Relaxed);
            return Ok(());
        };
        self.append_route_bound_frame(frame, route_receipt, runtime_parity_case)
    }
}

impl RouteBoundOutboxFrameV1 {
    fn new(
        frame: RelationFrame,
        route_receipt: NandoRouteReceiptV1,
        runtime_parity_case: Option<RuntimeParityCase>,
    ) -> Self {
        Self {
            schema: ROUTE_BOUND_OUTBOX_SCHEMA_V1.to_owned(),
            route_receipt_root_sha256: route_receipt.receipt_root_sha256.clone(),
            route_receipt,
            frame,
            runtime_parity_case,
        }
    }

    fn seal(&self) -> Result<RemoteEvidenceFrameV1, String> {
        self.seal_classified().map_err(|error| error.to_string())
    }

    fn seal_classified(
        &self,
    ) -> Result<RemoteEvidenceFrameV1, RemoteEvidenceFrameSealErrorV1> {
        if self.schema != ROUTE_BOUND_OUTBOX_SCHEMA_V1 {
            return Err(RemoteEvidenceFrameSealErrorV1::Fatal(
                "evidence_agent_outbox_schema_invalid".to_owned(),
            ));
        }
        RemoteEvidenceFrameV1::seal_route_bound_with_parity_classified(
            self.frame.clone(),
            self.route_receipt.clone(),
            self.runtime_parity_case.clone(),
        )
    }

    fn same_transport_binding(&self, other: &Self) -> bool {
        self.schema == other.schema
            && self.route_receipt_root_sha256 == other.route_receipt_root_sha256
            && self.route_receipt == other.route_receipt
            && self.frame == other.frame
    }
}

fn frame_bytes<T: Serialize>(frame: &T) -> Result<u64, String> {
    let bytes = serde_cbor::to_vec(frame)
        .map_err(|error| format!("evidence_agent_outbox_encode:{error}"))?;
    if bytes.len() > MAX_FRAME_BYTES {
        return Err("evidence_agent_outbox_frame_budget".to_owned());
    }
    Ok(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
}
