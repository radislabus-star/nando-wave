use super::journal::*;
use super::*;

const K1_EPISTEMIC_FORK_SCHEMA_V1: &str = "nando.k1-epistemic-lane-fork.v1";
const K1_EPISTEMIC_FORK_FILE: &str = "k1-epistemic-lane-fork-v1.json";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct K1EpistemicLaneForkV1 {
    schema: String,
    fork_root_sha256: String,
    mechanism_ledger_revision: u64,
    mechanism_ledger_root_sha256: String,
    mechanism_projection_root_sha256: String,
    watched_candidate_freeze_root_sha256: String,
    watched_candidate_root_sha256: String,
    watched_identification_freeze_root_sha256: String,
    epistemic_prefix_revision: u64,
    epistemic_prefix_event_root_sha256: String,
    epistemic_prefix_ledger_root_sha256: String,
    signer_public_key_sha256: String,
    signature_ed25519_hex: String,
}

pub(super) fn ensure_epistemic_lane(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
) -> Result<(), String> {
    let fork_path = epistemic_fork_path(config)?;
    if fork_path.exists() {
        let existing = restore_epistemic_fork(config)?;
        validate_mechanism_source(config, &existing)?;
        validate_epistemic_prefix(config, &existing)?;
        return Ok(());
    }
    let mechanism = restore_anchored_scheduler_for(config, K1SchedulerLaneV1::Mechanism)?;
    let mechanism_projection = projection_for(&mechanism)?;
    let watched_candidate = mechanism_projection
        .active_candidate_freeze
        .as_ref()
        .ok_or_else(|| "k1_epistemic_fork_active_mechanism_candidate_missing".to_owned())?;
    let watched_identification = mechanism_projection
        .identification_freeze
        .as_ref()
        .ok_or_else(|| "k1_epistemic_fork_identification_missing".to_owned())?;
    let candidate_event = mechanism
        .events
        .iter()
        .find(|event| {
            matches!(
                &event.payload,
                K1SchedulerEventPayloadV1::CandidateFreeze(freeze)
                    if freeze.freeze_root_sha256 == watched_candidate.freeze_root_sha256
            )
        })
        .ok_or_else(|| "k1_epistemic_fork_candidate_event_missing".to_owned())?;
    let prefix_revision = candidate_event.sequence.saturating_sub(1);
    let (prefix, prefix_event_root) = restore_scheduler_journal_prefix_for(
        config,
        K1SchedulerLaneV1::Mechanism,
        prefix_revision,
    )?;
    let expected = K1EpistemicLaneForkV1::seal(
        &mechanism,
        &mechanism_projection,
        watched_candidate,
        watched_identification,
        &prefix,
        prefix_event_root,
        signing_key,
    )?;

    copy_prefix_events(config, prefix_revision)?;
    persist_scheduler_anchor_for(
        config,
        K1SchedulerLaneV1::Epistemic,
        signing_key,
        &prefix,
        &expected.epistemic_prefix_event_root_sha256,
    )?;
    persist_scheduler_cache_for(config, K1SchedulerLaneV1::Epistemic, &prefix)?;
    write_bytes_atomic(
        &fork_path,
        &serde_json::to_vec(&expected)
            .map_err(|error| format!("k1_epistemic_fork_encode:{error}"))?,
        "k1-epistemic-fork",
    )?;
    validate_epistemic_prefix(config, &expected)
}

fn validate_mechanism_source(
    config: &CertificationAuthorityConfigV1,
    fork: &K1EpistemicLaneForkV1,
) -> Result<(), String> {
    let (ledger, _) = restore_scheduler_journal_prefix_for(
        config,
        K1SchedulerLaneV1::Mechanism,
        fork.mechanism_ledger_revision,
    )?;
    let projection = projection_for(&ledger)?;
    if ledger.ledger_root_sha256 != fork.mechanism_ledger_root_sha256
        || projection.projection_root_sha256 != fork.mechanism_projection_root_sha256
        || projection
            .active_candidate_freeze
            .as_ref()
            .is_none_or(|candidate| {
                candidate.freeze_root_sha256 != fork.watched_candidate_freeze_root_sha256
                    || candidate.candidate_root_sha256 != fork.watched_candidate_root_sha256
            })
        || projection
            .identification_freeze
            .as_ref()
            .is_none_or(|identification| {
                identification.freeze_root_sha256 != fork.watched_identification_freeze_root_sha256
            })
    {
        return Err("k1_epistemic_fork_mechanism_source_mismatch".to_owned());
    }
    Ok(())
}

pub(super) fn epistemic_exclusions(
    config: &CertificationAuthorityConfigV1,
) -> Result<BTreeSet<String>, String> {
    let fork = restore_epistemic_fork(config)?;
    Ok(BTreeSet::from([fork.watched_candidate_root_sha256]))
}

fn copy_prefix_events(
    config: &CertificationAuthorityConfigV1,
    prefix_revision: u64,
) -> Result<(), String> {
    let source = scheduler_journal_path_for(config, K1SchedulerLaneV1::Mechanism);
    let destination = scheduler_journal_path_for(config, K1SchedulerLaneV1::Epistemic);
    fs::create_dir_all(&destination)
        .map_err(|error| format!("k1_epistemic_fork_parent:{error}"))?;
    for sequence in 1..=prefix_revision {
        let name = format!("{sequence:020}.json");
        let source_path = source.join(&name);
        let destination_path = destination.join(&name);
        let bytes = fs::read(&source_path)
            .map_err(|error| format!("k1_epistemic_fork_source_read:{error}"))?;
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o640)
            .open(&destination_path)
        {
            Ok(mut file) => {
                file.write_all(&bytes)
                    .map_err(|error| format!("k1_epistemic_fork_write:{error}"))?;
                file.sync_all()
                    .map_err(|error| format!("k1_epistemic_fork_sync:{error}"))?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if fs::read(&destination_path)
                    .map_err(|error| format!("k1_epistemic_fork_existing_read:{error}"))?
                    != bytes
                {
                    return Err("k1_epistemic_fork_existing_event_mismatch".to_owned());
                }
            }
            Err(error) => return Err(format!("k1_epistemic_fork_create:{error}")),
        }
    }
    File::open(&destination)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("k1_epistemic_fork_dir_sync:{error}"))
}

fn validate_epistemic_prefix(
    config: &CertificationAuthorityConfigV1,
    fork: &K1EpistemicLaneForkV1,
) -> Result<(), String> {
    fork.validate_with_public_key(&read_verifying_key(&config.authority_public_key_path)?)?;
    let (prefix, event_root) = restore_scheduler_journal_prefix_for(
        config,
        K1SchedulerLaneV1::Epistemic,
        fork.epistemic_prefix_revision,
    )?;
    if prefix.ledger_root_sha256 != fork.epistemic_prefix_ledger_root_sha256
        || event_root != fork.epistemic_prefix_event_root_sha256
    {
        return Err("k1_epistemic_fork_prefix_mismatch".to_owned());
    }
    Ok(())
}

fn restore_epistemic_fork(
    config: &CertificationAuthorityConfigV1,
) -> Result<K1EpistemicLaneForkV1, String> {
    let receipt: K1EpistemicLaneForkV1 = serde_json::from_slice(
        &fs::read(epistemic_fork_path(config)?)
            .map_err(|error| format!("k1_epistemic_fork_read:{error}"))?,
    )
    .map_err(|error| format!("k1_epistemic_fork_decode:{error}"))?;
    receipt.validate_with_public_key(&read_verifying_key(&config.authority_public_key_path)?)?;
    Ok(receipt)
}

fn epistemic_fork_path(config: &CertificationAuthorityConfigV1) -> Result<PathBuf, String> {
    config
        .anchor_path
        .parent()
        .map(|parent| parent.join(K1_EPISTEMIC_FORK_FILE))
        .ok_or_else(|| "k1_epistemic_fork_parent_missing".to_owned())
}

impl K1EpistemicLaneForkV1 {
    #[allow(clippy::too_many_arguments)]
    fn seal(
        mechanism: &K1SchedulerLedgerV1,
        mechanism_projection: &K1SchedulerProjectionV1,
        watched_candidate: &K1NaturalCandidateFreezeV1,
        watched_identification: &K1IdentificationFreezeV1,
        prefix: &K1SchedulerLedgerV1,
        prefix_event_root_sha256: String,
        signing_key: &SigningKey,
    ) -> Result<Self, String> {
        let mut receipt = Self {
            schema: K1_EPISTEMIC_FORK_SCHEMA_V1.to_owned(),
            fork_root_sha256: String::new(),
            mechanism_ledger_revision: mechanism.revision,
            mechanism_ledger_root_sha256: mechanism.ledger_root_sha256.clone(),
            mechanism_projection_root_sha256: mechanism_projection.projection_root_sha256.clone(),
            watched_candidate_freeze_root_sha256: watched_candidate.freeze_root_sha256.clone(),
            watched_candidate_root_sha256: watched_candidate.candidate_root_sha256.clone(),
            watched_identification_freeze_root_sha256: watched_identification
                .freeze_root_sha256
                .clone(),
            epistemic_prefix_revision: prefix.revision,
            epistemic_prefix_event_root_sha256: prefix_event_root_sha256,
            epistemic_prefix_ledger_root_sha256: prefix.ledger_root_sha256.clone(),
            signer_public_key_sha256: verifying_key_sha256(&signing_key.verifying_key()),
            signature_ed25519_hex: String::new(),
        };
        receipt.fork_root_sha256 = receipt.expected_root()?;
        receipt.signature_ed25519_hex = sign_root(signing_key, &receipt.fork_root_sha256)?;
        receipt.validate_with_public_key(&signing_key.verifying_key())?;
        Ok(receipt)
    }

    fn validate_with_public_key(&self, public_key: &VerifyingKey) -> Result<(), String> {
        if self.schema != K1_EPISTEMIC_FORK_SCHEMA_V1
            || self.mechanism_ledger_revision <= self.epistemic_prefix_revision
            || ![
                self.fork_root_sha256.as_str(),
                self.mechanism_ledger_root_sha256.as_str(),
                self.mechanism_projection_root_sha256.as_str(),
                self.watched_candidate_freeze_root_sha256.as_str(),
                self.watched_candidate_root_sha256.as_str(),
                self.watched_identification_freeze_root_sha256.as_str(),
                self.epistemic_prefix_event_root_sha256.as_str(),
                self.epistemic_prefix_ledger_root_sha256.as_str(),
                self.signer_public_key_sha256.as_str(),
            ]
            .into_iter()
            .all(valid_nonzero_sha256)
            || self.signer_public_key_sha256 != verifying_key_sha256(public_key)
            || self.fork_root_sha256 != self.expected_root()?
        {
            return Err("k1_epistemic_fork_invalid".to_owned());
        }
        verify_root(
            public_key,
            &self.fork_root_sha256,
            &self.signature_ed25519_hex,
        )
    }

    fn expected_root(&self) -> Result<String, String> {
        canonical_json_sha256(&(
            K1_EPISTEMIC_FORK_SCHEMA_V1,
            self.mechanism_ledger_revision,
            self.mechanism_ledger_root_sha256.as_str(),
            self.mechanism_projection_root_sha256.as_str(),
            self.watched_candidate_freeze_root_sha256.as_str(),
            self.watched_candidate_root_sha256.as_str(),
            self.watched_identification_freeze_root_sha256.as_str(),
            self.epistemic_prefix_revision,
            self.epistemic_prefix_event_root_sha256.as_str(),
            self.epistemic_prefix_ledger_root_sha256.as_str(),
            self.signer_public_key_sha256.as_str(),
        ))
        .map_err(str::to_owned)
    }
}
