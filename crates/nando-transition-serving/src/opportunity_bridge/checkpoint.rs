use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::spool::{PendingBridgeEvent, sync_directory};

const COUNTER_CHECKPOINT_SCHEMA_V1: &str = "nando.opportunity-bridge-counter-checkpoint.v1";
const COUNTER_CHECKPOINT_MAX_BYTES: u64 = 4 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(super) struct OpportunityBridgeCounterCheckpointV1 {
    pub(super) schema: String,
    pub(super) counter_started_after_sequence: u64,
    pub(super) last_sequence: u64,
    pub(super) events: u64,
    pub(super) request_events: u64,
    pub(super) request_input_tokens: u64,
}

impl OpportunityBridgeCounterCheckpointV1 {
    pub(super) fn empty(counter_started_after_sequence: u64) -> Self {
        Self {
            schema: COUNTER_CHECKPOINT_SCHEMA_V1.to_owned(),
            counter_started_after_sequence,
            last_sequence: counter_started_after_sequence,
            events: 0,
            request_events: 0,
            request_input_tokens: 0,
        }
    }

    pub(super) fn validate(&self) -> Result<(), String> {
        if self.schema != COUNTER_CHECKPOINT_SCHEMA_V1 {
            return Err("opportunity_bridge_counter_checkpoint_schema".to_owned());
        }
        if self.last_sequence < self.counter_started_after_sequence {
            return Err("opportunity_bridge_counter_checkpoint_sequence".to_owned());
        }
        if self.events
            > self
                .last_sequence
                .saturating_sub(self.counter_started_after_sequence)
            || self.request_events > self.events
            || (self.request_events == 0 && self.request_input_tokens != 0)
        {
            return Err("opportunity_bridge_counter_checkpoint_accounting".to_owned());
        }
        Ok(())
    }

    pub(super) fn with_pending(&self, pending: &[PendingBridgeEvent]) -> Result<Self, String> {
        self.validate()?;
        let mut next = self.clone();
        for row in pending {
            if row.sequence <= next.last_sequence {
                continue;
            }
            next.last_sequence = row.sequence;
            next.events = next.events.saturating_add(1);
            if let Some((input_tokens, _)) = row.event.request_economics() {
                next.request_events = next.request_events.saturating_add(1);
                next.request_input_tokens = next.request_input_tokens.saturating_add(input_tokens);
            }
        }
        next.validate()?;
        Ok(next)
    }
}

pub(super) fn load_counter_checkpoint(
    path: &Path,
) -> Result<Option<OpportunityBridgeCounterCheckpointV1>, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "opportunity_bridge_counter_checkpoint_open:{}:{error}",
                path.display()
            ));
        }
    };
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(COUNTER_CHECKPOINT_MAX_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("opportunity_bridge_counter_checkpoint_read:{error}"))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > COUNTER_CHECKPOINT_MAX_BYTES {
        return Err("opportunity_bridge_counter_checkpoint_budget".to_owned());
    }
    let checkpoint: OpportunityBridgeCounterCheckpointV1 = serde_json::from_slice(&bytes)
        .map_err(|error| format!("opportunity_bridge_counter_checkpoint_decode:{error}"))?;
    checkpoint.validate()?;
    Ok(Some(checkpoint))
}

pub(super) fn persist_counter_checkpoint(
    path: &Path,
    checkpoint: &OpportunityBridgeCounterCheckpointV1,
) -> Result<(), String> {
    checkpoint.validate()?;
    let bytes = serde_json::to_vec(checkpoint)
        .map_err(|error| format!("opportunity_bridge_counter_checkpoint_encode:{error}"))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > COUNTER_CHECKPOINT_MAX_BYTES {
        return Err("opportunity_bridge_counter_checkpoint_budget".to_owned());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "opportunity_bridge_counter_checkpoint_parent".to_owned())?;
    let temporary = parent.join("counter-checkpoint-v1.json.tmp");
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|error| format!("opportunity_bridge_counter_checkpoint_create:{error}"))?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("opportunity_bridge_counter_checkpoint_write:{error}"))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("opportunity_bridge_counter_checkpoint_publish:{error}"))?;
    sync_directory(parent)
}
