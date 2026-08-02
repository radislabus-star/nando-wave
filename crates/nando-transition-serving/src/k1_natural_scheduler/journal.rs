use super::*;

#[cfg(test)]
pub(super) fn restore_anchored_scheduler(
    config: &CertificationAuthorityConfigV1,
) -> Result<K1SchedulerLedgerV1, String> {
    restore_anchored_scheduler_for(config, K1SchedulerLaneV1::Mechanism)
}

pub(super) fn restore_anchored_scheduler_for(
    config: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
) -> Result<K1SchedulerLedgerV1, String> {
    let (ledger, last_event_root) = restore_signed_scheduler_journal_for(config, lane)?;
    let anchor = restore_scheduler_anchor_for(config, lane)?;
    if anchor.revision != ledger.revision
        || anchor.journal_event_root_sha256 != last_event_root
        || anchor.ledger_root_sha256 != ledger.ledger_root_sha256
    {
        return Err("k1_scheduler_rollback_detected".to_owned());
    }
    Ok(ledger)
}

pub(super) fn restore_signed_scheduler_journal_for(
    config: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
) -> Result<(K1SchedulerLedgerV1, String), String> {
    let mut ledger = K1SchedulerLedgerV1::empty().map_err(str::to_owned)?;
    let mut previous_root = scheduler_genesis_root();
    let directory = scheduler_journal_path_for(config, lane);
    let mut paths = match fs::read_dir(&directory) {
        Ok(entries) => entries
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("k1_scheduler_journal_list:{error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((ledger, previous_root));
        }
        Err(error) => return Err(format!("k1_scheduler_journal_open:{error}")),
    };
    paths.sort();
    let public_key = read_verifying_key(&config.authority_public_key_path)?;
    for path in paths {
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            return Err("k1_scheduler_journal_unknown_file".to_owned());
        }
        let signed: SignedSchedulerEventV1 = serde_json::from_slice(
            &fs::read(&path).map_err(|error| format!("k1_scheduler_journal_read:{error}"))?,
        )
        .map_err(|error| format!("k1_scheduler_journal_decode:{error}"))?;
        signed.validate_with_public_key(&public_key)?;
        if signed.event.sequence != ledger.revision.saturating_add(1)
            || signed.event.previous_event_root_sha256 != previous_root
        {
            return Err("k1_scheduler_journal_chain_invalid".to_owned());
        }
        let replayed = ledger
            .append(signed.event.payload.clone())
            .map_err(str::to_owned)?;
        if replayed != &signed.event
            || signed.resulting_ledger_root_sha256 != ledger.ledger_root_sha256
        {
            return Err("k1_scheduler_journal_ledger_mismatch".to_owned());
        }
        previous_root = signed.event.event_root_sha256;
    }
    Ok((ledger, previous_root))
}

pub(super) fn restore_scheduler_journal_prefix_for(
    config: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
    revision: u64,
) -> Result<(K1SchedulerLedgerV1, String), String> {
    let mut ledger = K1SchedulerLedgerV1::empty().map_err(str::to_owned)?;
    let mut previous_root = scheduler_genesis_root();
    let public_key = read_verifying_key(&config.authority_public_key_path)?;
    for sequence in 1..=revision {
        let path = scheduler_journal_path_for(config, lane).join(format!("{sequence:020}.json"));
        let signed: SignedSchedulerEventV1 = serde_json::from_slice(
            &fs::read(path).map_err(|error| format!("k1_scheduler_journal_read:{error}"))?,
        )
        .map_err(|error| format!("k1_scheduler_journal_decode:{error}"))?;
        signed.validate_with_public_key(&public_key)?;
        if signed.event.sequence != sequence
            || signed.event.previous_event_root_sha256 != previous_root
        {
            return Err("k1_scheduler_journal_chain_invalid".to_owned());
        }
        ledger
            .append(signed.event.payload.clone())
            .map_err(str::to_owned)?;
        if signed.resulting_ledger_root_sha256 != ledger.ledger_root_sha256 {
            return Err("k1_scheduler_journal_ledger_mismatch".to_owned());
        }
        previous_root = signed.event.event_root_sha256;
    }
    Ok((ledger, previous_root))
}

pub(super) fn restore_scheduler_anchor_for(
    config: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
) -> Result<SchedulerAnchorV1, String> {
    let anchor: SchedulerAnchorV1 = serde_json::from_slice(
        &fs::read(scheduler_anchor_path_for(config, lane)?)
            .map_err(|error| format!("k1_scheduler_anchor_read:{error}"))?,
    )
    .map_err(|error| format!("k1_scheduler_anchor_decode:{error}"))?;
    anchor.validate_with_public_key(&read_verifying_key(&config.authority_public_key_path)?)?;
    Ok(anchor)
}

#[cfg(test)]
pub(super) fn persist_scheduler_event(
    config: &CertificationAuthorityConfigV1,
    event: &SignedSchedulerEventV1,
) -> Result<(), String> {
    persist_scheduler_event_for(config, K1SchedulerLaneV1::Mechanism, event)
}

pub(super) fn persist_scheduler_event_for(
    config: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
    event: &SignedSchedulerEventV1,
) -> Result<(), String> {
    let directory = scheduler_journal_path_for(config, lane);
    fs::create_dir_all(&directory)
        .map_err(|error| format!("k1_scheduler_journal_parent:{error}"))?;
    let path = directory.join(format!("{:020}.json", event.event.sequence));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o640)
        .open(path)
        .map_err(|error| format!("k1_scheduler_journal_create:{error}"))?;
    file.write_all(
        &serde_json::to_vec(event)
            .map_err(|error| format!("k1_scheduler_journal_encode:{error}"))?,
    )
    .map_err(|error| format!("k1_scheduler_journal_write:{error}"))?;
    file.sync_all()
        .map_err(|error| format!("k1_scheduler_journal_sync:{error}"))?;
    File::open(&directory)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("k1_scheduler_journal_dir_sync:{error}"))
}

#[cfg(test)]
pub(super) fn persist_scheduler_anchor(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    ledger: &K1SchedulerLedgerV1,
    last_event_root_sha256: &str,
) -> Result<(), String> {
    persist_scheduler_anchor_for(
        config,
        K1SchedulerLaneV1::Mechanism,
        signing_key,
        ledger,
        last_event_root_sha256,
    )
}

pub(super) fn persist_scheduler_anchor_for(
    config: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
    signing_key: &SigningKey,
    ledger: &K1SchedulerLedgerV1,
    last_event_root_sha256: &str,
) -> Result<(), String> {
    let anchor_path = scheduler_anchor_path_for(config, lane)?;
    let anchor_parent = anchor_path
        .parent()
        .ok_or_else(|| "k1_scheduler_anchor_parent_missing".to_owned())?;
    fs::create_dir_all(anchor_parent)
        .map_err(|error| format!("k1_scheduler_anchor_parent:{error}"))?;
    let anchor = SchedulerAnchorV1::seal(
        ledger.revision,
        last_event_root_sha256.to_owned(),
        ledger.ledger_root_sha256.clone(),
        signing_key,
    )?;
    write_bytes_atomic(
        &anchor_path,
        &serde_json::to_vec(&anchor)
            .map_err(|error| format!("k1_scheduler_anchor_encode:{error}"))?,
        "k1-scheduler-anchor",
    )
}

pub(super) fn persist_scheduler_cache_for(
    config: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
    ledger: &K1SchedulerLedgerV1,
) -> Result<(), String> {
    fs::create_dir_all(&config.root)
        .map_err(|error| format!("k1_scheduler_cache_parent:{error}"))?;
    write_bytes_atomic(
        &config.root.join(match lane {
            K1SchedulerLaneV1::Mechanism => K1_SCHEDULER_CACHE_FILE,
            K1SchedulerLaneV1::Epistemic => K1_EPISTEMIC_SCHEDULER_CACHE_FILE,
        }),
        &serde_json::to_vec(ledger)
            .map_err(|error| format!("k1_scheduler_cache_encode:{error}"))?,
        "k1-scheduler-cache",
    )
}

#[cfg(test)]
pub(super) fn scheduler_journal_path(config: &CertificationAuthorityConfigV1) -> PathBuf {
    scheduler_journal_path_for(config, K1SchedulerLaneV1::Mechanism)
}

pub(super) fn scheduler_journal_path_for(
    config: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
) -> PathBuf {
    config.root.join(match lane {
        K1SchedulerLaneV1::Mechanism => K1_SCHEDULER_JOURNAL_DIR,
        K1SchedulerLaneV1::Epistemic => K1_EPISTEMIC_SCHEDULER_JOURNAL_DIR,
    })
}

#[cfg(test)]
pub(super) fn scheduler_anchor_path(
    config: &CertificationAuthorityConfigV1,
) -> Result<PathBuf, String> {
    scheduler_anchor_path_for(config, K1SchedulerLaneV1::Mechanism)
}

pub(super) fn scheduler_anchor_path_for(
    config: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
) -> Result<PathBuf, String> {
    config
        .anchor_path
        .parent()
        .map(|parent| {
            parent.join(match lane {
                K1SchedulerLaneV1::Mechanism => K1_SCHEDULER_ANCHOR_FILE,
                K1SchedulerLaneV1::Epistemic => K1_EPISTEMIC_SCHEDULER_ANCHOR_FILE,
            })
        })
        .ok_or_else(|| "k1_scheduler_anchor_parent_missing".to_owned())
}

impl SignedSchedulerEventV1 {
    pub(super) fn seal(
        event: K1SchedulerEventV1,
        resulting_ledger_root_sha256: String,
        signing_key: &SigningKey,
    ) -> Result<Self, String> {
        let mut signed = Self {
            schema: K1_SCHEDULER_SIGNED_EVENT_SCHEMA_V1.to_owned(),
            signed_root_sha256: String::new(),
            event,
            resulting_ledger_root_sha256,
            signer_public_key_sha256: verifying_key_sha256(&signing_key.verifying_key()),
            signature_ed25519_hex: String::new(),
        };
        signed.signed_root_sha256 = signed.expected_root()?;
        signed.signature_ed25519_hex = sign_root(signing_key, &signed.signed_root_sha256)?;
        signed.validate_with_public_key(&signing_key.verifying_key())?;
        Ok(signed)
    }

    fn validate_with_public_key(&self, public_key: &VerifyingKey) -> Result<(), String> {
        self.event.validate().map_err(str::to_owned)?;
        if self.schema != K1_SCHEDULER_SIGNED_EVENT_SCHEMA_V1
            || !valid_nonzero_sha256(&self.signed_root_sha256)
            || !valid_nonzero_sha256(&self.resulting_ledger_root_sha256)
            || self.signer_public_key_sha256 != verifying_key_sha256(public_key)
            || self.signed_root_sha256 != self.expected_root()?
        {
            return Err("k1_scheduler_signed_event_invalid".to_owned());
        }
        verify_root(
            public_key,
            &self.signed_root_sha256,
            &self.signature_ed25519_hex,
        )
    }

    fn expected_root(&self) -> Result<String, String> {
        canonical_json_sha256(&(
            K1_SCHEDULER_SIGNED_EVENT_SCHEMA_V1,
            self.event.event_root_sha256.as_str(),
            self.resulting_ledger_root_sha256.as_str(),
            self.signer_public_key_sha256.as_str(),
        ))
        .map_err(str::to_owned)
    }
}

impl SchedulerAnchorV1 {
    fn seal(
        revision: u64,
        journal_event_root_sha256: String,
        ledger_root_sha256: String,
        signing_key: &SigningKey,
    ) -> Result<Self, String> {
        let mut anchor = Self {
            schema: K1_SCHEDULER_ANCHOR_SCHEMA_V1.to_owned(),
            anchor_root_sha256: String::new(),
            revision,
            journal_event_root_sha256,
            ledger_root_sha256,
            signer_public_key_sha256: verifying_key_sha256(&signing_key.verifying_key()),
            signature_ed25519_hex: String::new(),
        };
        anchor.anchor_root_sha256 = anchor.expected_root()?;
        anchor.signature_ed25519_hex = sign_root(signing_key, &anchor.anchor_root_sha256)?;
        anchor.validate_with_public_key(&signing_key.verifying_key())?;
        Ok(anchor)
    }

    fn validate_with_public_key(&self, public_key: &VerifyingKey) -> Result<(), String> {
        if self.schema != K1_SCHEDULER_ANCHOR_SCHEMA_V1
            || !valid_nonzero_sha256(&self.anchor_root_sha256)
            || !valid_nonzero_sha256(&self.journal_event_root_sha256)
            || !valid_nonzero_sha256(&self.ledger_root_sha256)
            || self.signer_public_key_sha256 != verifying_key_sha256(public_key)
            || self.anchor_root_sha256 != self.expected_root()?
        {
            return Err("k1_scheduler_anchor_invalid".to_owned());
        }
        verify_root(
            public_key,
            &self.anchor_root_sha256,
            &self.signature_ed25519_hex,
        )
    }

    fn expected_root(&self) -> Result<String, String> {
        canonical_json_sha256(&(
            K1_SCHEDULER_ANCHOR_SCHEMA_V1,
            self.revision,
            self.journal_event_root_sha256.as_str(),
            self.ledger_root_sha256.as_str(),
            self.signer_public_key_sha256.as_str(),
        ))
        .map_err(str::to_owned)
    }
}

pub(super) fn payload_root(payload: &K1SchedulerEventPayloadV1) -> &str {
    match payload {
        K1SchedulerEventPayloadV1::CandidateFreeze(value) => &value.freeze_root_sha256,
        K1SchedulerEventPayloadV1::IdentificationFreeze(value) => &value.freeze_root_sha256,
        K1SchedulerEventPayloadV1::FuturePredictionContract(value) => &value.contract_root_sha256,
        K1SchedulerEventPayloadV1::FuturePrediction(value) => &value.prediction_root_sha256,
        K1SchedulerEventPayloadV1::FutureOutcome(value) => &value.outcome_root_sha256,
        K1SchedulerEventPayloadV1::ProbeRound(value) => &value.receipt_root_sha256,
        K1SchedulerEventPayloadV1::TerminalVerdict(value) => &value.verdict_root_sha256,
        K1SchedulerEventPayloadV1::TransferSettlement(value) => &value.settlement_root_sha256,
    }
}

pub(super) fn scheduler_genesis_root() -> String {
    format!(
        "{:x}",
        Sha256::digest(b"nando.k1-natural-scheduler-journal-genesis.v1")
    )
}

pub(super) fn verifying_key_sha256(key: &VerifyingKey) -> String {
    format!("{:x}", Sha256::digest(key.as_bytes()))
}

pub(super) fn sign_root(signing_key: &SigningKey, root_sha256: &str) -> Result<String, String> {
    let root = decode_hex_array::<32>(root_sha256)
        .ok_or_else(|| "k1_scheduler_signed_root_invalid".to_owned())?;
    Ok(encode_hex(&signing_key.sign(&root).to_bytes()))
}

pub(super) fn verify_root(
    public_key: &VerifyingKey,
    root_sha256: &str,
    signature_hex: &str,
) -> Result<(), String> {
    let root = decode_hex_array::<32>(root_sha256)
        .ok_or_else(|| "k1_scheduler_signed_root_invalid".to_owned())?;
    let signature = Signature::from_bytes(
        &decode_hex_array::<64>(signature_hex)
            .ok_or_else(|| "k1_scheduler_signature_invalid".to_owned())?,
    );
    public_key
        .verify(&root, &signature)
        .map_err(|_| "k1_scheduler_signature_invalid".to_owned())
}

fn decode_hex_array<const N: usize>(value: &str) -> Option<[u8; N]> {
    if value.len() != N.saturating_mul(2) {
        return None;
    }
    let mut output = [0_u8; N];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        output[index] = hex_digit(pair[0])?.checked_mul(16)? | hex_digit(pair[1])?;
    }
    Some(output)
}

fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

pub(super) fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}
