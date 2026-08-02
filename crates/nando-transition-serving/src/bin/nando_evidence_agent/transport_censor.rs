#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct TransportCensorReceiptV1 {
    schema: String,
    receipt_root_sha256: String,
    frame_root_sha256: String,
    route_receipt_root_sha256: String,
    session_id_sha256: String,
    turn_intent_id_sha256: String,
    action_event_id_sha256: String,
    observed_at_unix_nanos: u64,
    blocker: RemoteEvidenceFrameValidationBlockerV1,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

struct TransportCensorLedger {
    ledger: FramedCborLedger,
    receipts: BTreeMap<String, TransportCensorReceiptV1>,
    payload_bytes: u64,
}

impl TransportCensorReceiptV1 {
    fn seal(
        bound: &RouteBoundOutboxFrameV1,
        blocker: RemoteEvidenceFrameValidationBlockerV1,
    ) -> Result<Self, String> {
        let frame_root_sha256 = canonical_json_sha256(&bound.frame)
            .map_err(|error| format!("evidence_agent_censor_frame_root:{error}"))?;
        let mut receipt = Self {
            schema: TRANSPORT_CENSOR_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            frame_root_sha256,
            route_receipt_root_sha256: bound.route_receipt_root_sha256.clone(),
            session_id_sha256: bound.frame.session_id_sha256.clone(),
            turn_intent_id_sha256: bound.frame.client_intent_id_sha256.clone(),
            action_event_id_sha256: bound.frame.event_id_sha256.clone(),
            observed_at_unix_nanos: bound.frame.observed_at_unix_nanos,
            blocker,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        receipt
            .validate()
            .then_some(receipt)
            .ok_or_else(|| "evidence_agent_transport_censor_invalid".to_owned())
    }

    fn expected_root(&self) -> Result<String, String> {
        canonical_json_sha256(&(
            TRANSPORT_CENSOR_SCHEMA_V1,
            self.frame_root_sha256.as_str(),
            self.route_receipt_root_sha256.as_str(),
            self.session_id_sha256.as_str(),
            self.turn_intent_id_sha256.as_str(),
            self.action_event_id_sha256.as_str(),
            self.observed_at_unix_nanos,
            self.blocker,
            false,
            false,
        ))
        .map_err(|error| format!("evidence_agent_transport_censor_root:{error}"))
    }

    fn validate(&self) -> bool {
        self.schema == TRANSPORT_CENSOR_SCHEMA_V1
            && [
                self.receipt_root_sha256.as_str(),
                self.frame_root_sha256.as_str(),
                self.route_receipt_root_sha256.as_str(),
                self.session_id_sha256.as_str(),
                self.turn_intent_id_sha256.as_str(),
                self.action_event_id_sha256.as_str(),
            ]
            .into_iter()
            .all(valid_root)
            && self.observed_at_unix_nanos > 0
            && !self.authority_ready
            && !self.phase_mutation_allowed
            && self.expected_root().is_ok_and(|root| root == self.receipt_root_sha256)
    }
}

impl TransportCensorLedger {
    fn open(directory: &Path) -> Result<Self, String> {
        let ledger = FramedCborLedger::open(directory, TRANSPORT_CENSOR_PREFIX)?;
        let persisted =
            read_framed_cbor::<TransportCensorReceiptV1>(directory, TRANSPORT_CENSOR_PREFIX)?;
        let mut receipts = BTreeMap::<String, TransportCensorReceiptV1>::new();
        let mut payload_bytes = 0_u64;
        for receipt in persisted {
            if !receipt.validate() {
                return Err("evidence_agent_transport_censor_invalid".to_owned());
            }
            let bytes = frame_bytes(&receipt)?;
            if let Some(existing) = receipts.get(&receipt.receipt_root_sha256) {
                if existing == &receipt {
                    continue;
                }
                return Err("evidence_agent_transport_censor_rebound".to_owned());
            }
            if receipts.len() >= MAX_OUTBOX_FRAMES
                || payload_bytes.saturating_add(bytes) > MAX_OUTBOX_BYTES
            {
                return Err("evidence_agent_transport_censor_budget".to_owned());
            }
            payload_bytes = payload_bytes.saturating_add(bytes);
            receipts.insert(receipt.receipt_root_sha256.clone(), receipt);
        }
        Ok(Self {
            ledger,
            receipts,
            payload_bytes,
        })
    }

    fn append(
        &mut self,
        bound: &RouteBoundOutboxFrameV1,
        blocker: RemoteEvidenceFrameValidationBlockerV1,
    ) -> Result<bool, String> {
        let receipt = TransportCensorReceiptV1::seal(bound, blocker)?;
        if let Some(existing) = self.receipts.get(&receipt.receipt_root_sha256) {
            if existing == &receipt {
                return Ok(false);
            }
            return Err("evidence_agent_transport_censor_rebound".to_owned());
        }
        let bytes = frame_bytes(&receipt)?;
        if self.receipts.len() >= MAX_OUTBOX_FRAMES
            || self.payload_bytes.saturating_add(bytes) > MAX_OUTBOX_BYTES
        {
            return Err("evidence_agent_transport_censor_budget".to_owned());
        }
        self.ledger.append(&receipt)?;
        self.ledger.sync()?;
        self.payload_bytes = self.payload_bytes.saturating_add(bytes);
        self.receipts
            .insert(receipt.receipt_root_sha256.clone(), receipt);
        Ok(true)
    }

    fn len(&self) -> usize {
        self.receipts.len()
    }
}
