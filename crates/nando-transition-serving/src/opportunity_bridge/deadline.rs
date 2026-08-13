use std::fs::{self, File, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use nando_operator_learning::OpportunityBridgeEventV1;
use serde::{Deserialize, Serialize};

use super::BridgeInner;
use super::spool::{sync_directory, sync_pending_spool};

pub(crate) const S1C4_WINDOW_BOUNDARY_FILE_V1: &str = "s1c4-window-boundary-v1.json";
const WINDOW_BOUNDARY_SCHEMA_V1: &str = "nando.s1c4-window-boundary.v1";
const MAX_BOUNDARY_BYTES: u64 = 16 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum OpportunityWindowClosureV1 {
    RequestLimit,
    TimeLimit,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OpportunityWindowBoundaryV1 {
    pub schema: String,
    pub boundary_root_sha256: String,
    pub cursor_root_sha256: String,
    pub closure: OpportunityWindowClosureV1,
    pub closes_at_unix: u64,
    pub opportunity_end_sequence: u64,
    pub opportunity_end_request_ordinal: u64,
    pub opportunity_end_input_tokens: u64,
    pub frozen_at_unix: u64,
}

impl OpportunityWindowBoundaryV1 {
    #[allow(clippy::too_many_arguments)]
    fn seal(
        cursor_root_sha256: String,
        closure: OpportunityWindowClosureV1,
        closes_at_unix: u64,
        opportunity_end_sequence: u64,
        opportunity_end_request_ordinal: u64,
        opportunity_end_input_tokens: u64,
        frozen_at_unix: u64,
    ) -> Result<Self, String> {
        let mut boundary = Self {
            schema: WINDOW_BOUNDARY_SCHEMA_V1.to_owned(),
            boundary_root_sha256: String::new(),
            cursor_root_sha256,
            closure,
            closes_at_unix,
            opportunity_end_sequence,
            opportunity_end_request_ordinal,
            opportunity_end_input_tokens,
            frozen_at_unix,
        };
        boundary.boundary_root_sha256 = boundary.expected_root()?;
        boundary.validate()?;
        Ok(boundary)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.schema != WINDOW_BOUNDARY_SCHEMA_V1
            || !valid_nonzero_sha256(&self.boundary_root_sha256)
            || !valid_nonzero_sha256(&self.cursor_root_sha256)
            || self.closes_at_unix == 0
            || self.frozen_at_unix < self.closes_at_unix
            || self.opportunity_end_sequence == 0
            || self.opportunity_end_request_ordinal == 0
            || self.expected_root()? != self.boundary_root_sha256
        {
            return Err("s1c4_window_boundary_invalid".to_owned());
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, String> {
        canonical_json_sha256(&(
            WINDOW_BOUNDARY_SCHEMA_V1,
            self.cursor_root_sha256.as_str(),
            self.closure,
            self.closes_at_unix,
            self.opportunity_end_sequence,
            self.opportunity_end_request_ordinal,
            self.opportunity_end_input_tokens,
            self.frozen_at_unix,
        ))
        .map_err(str::to_owned)
    }
}

pub(super) struct OpportunityWindowCaptureV1 {
    cursor_root_sha256: String,
    deadline_at_unix: u64,
    maximum_request_ordinal: u64,
    boundary_path: PathBuf,
    boundary: Option<OpportunityWindowBoundaryV1>,
    failure: Option<String>,
}

pub(super) fn configure_window_capture(
    inner: &BridgeInner,
    cursor_root_sha256: String,
    deadline_at_unix: u64,
    maximum_request_ordinal: u64,
    boundary_path: PathBuf,
) -> Result<(), String> {
    if !valid_nonzero_sha256(&cursor_root_sha256)
        || deadline_at_unix == 0
        || maximum_request_ordinal == 0
    {
        return Err("s1c4_window_capture_invalid".to_owned());
    }
    let boundary = read_boundary(&boundary_path)?;
    if boundary
        .as_ref()
        .is_some_and(|boundary| boundary.cursor_root_sha256 != cursor_root_sha256)
    {
        return Err("s1c4_window_boundary_cursor_mismatch".to_owned());
    }
    let mut capture = OpportunityWindowCaptureV1 {
        cursor_root_sha256,
        deadline_at_unix,
        maximum_request_ordinal,
        boundary_path,
        boundary,
        failure: None,
    };
    if capture.boundary.is_none() {
        let current_ordinal = inner.producer.request_events.load(Ordering::Acquire);
        if current_ordinal > capture.maximum_request_ordinal {
            return Err("s1c4_window_request_limit_overshot".to_owned());
        }
        if current_ordinal == capture.maximum_request_ordinal {
            capture.boundary = Some(seal_current_prefix(
                inner,
                &capture,
                OpportunityWindowClosureV1::RequestLimit,
                unix_now()?,
                unix_now()?,
            )?);
        }
    }
    *inner
        .deadline_capture
        .lock()
        .map_err(|_| "s1c4_window_capture_lock_poisoned".to_owned())? = Some(capture);
    Ok(())
}

pub(super) fn disable_window_capture(inner: &BridgeInner) {
    if let Ok(mut capture) = inner.deadline_capture.lock() {
        *capture = None;
    }
}

pub(super) fn classify_request_before_persist(
    inner: &BridgeInner,
    event: &OpportunityBridgeEventV1,
) -> Result<bool, String> {
    if event.request_economics().is_none() {
        return Ok(false);
    }
    let now_unix = unix_now()?;
    let mut capture = inner
        .deadline_capture
        .lock()
        .map_err(|_| "s1c4_window_capture_lock_poisoned".to_owned())?;
    let Some(capture) = capture.as_mut() else {
        return Ok(false);
    };
    if let Some(error) = &capture.failure {
        return Err(error.clone());
    }
    if capture.boundary.is_some() {
        return Ok(false);
    }
    if now_unix <= capture.deadline_at_unix {
        return Ok(true);
    }
    let boundary = match seal_current_prefix(
        inner,
        capture,
        OpportunityWindowClosureV1::TimeLimit,
        capture.deadline_at_unix,
        now_unix,
    ) {
        Ok(boundary) => boundary,
        Err(error) => {
            capture.failure = Some(error.clone());
            return Err(error);
        }
    };
    capture.boundary = Some(boundary);
    Ok(false)
}

pub(super) fn freeze_request_limit_after_persist(
    inner: &BridgeInner,
    request_was_eligible: bool,
    request_observed_at_unix: u64,
) -> Result<Option<OpportunityWindowBoundaryV1>, String> {
    if !request_was_eligible {
        return Ok(None);
    }
    let mut capture = inner
        .deadline_capture
        .lock()
        .map_err(|_| "s1c4_window_capture_lock_poisoned".to_owned())?;
    let Some(capture) = capture.as_mut() else {
        return Ok(None);
    };
    if let Some(error) = &capture.failure {
        return Err(error.clone());
    }
    if capture.boundary.is_some()
        || inner.producer.request_events.load(Ordering::Acquire) < capture.maximum_request_ordinal
    {
        return Ok(None);
    }
    let boundary = match seal_current_prefix(
        inner,
        capture,
        OpportunityWindowClosureV1::RequestLimit,
        request_observed_at_unix,
        unix_now()?,
    ) {
        Ok(boundary) => boundary,
        Err(error) => {
            capture.failure = Some(error.clone());
            return Err(error);
        }
    };
    capture.boundary = Some(boundary.clone());
    Ok(Some(boundary))
}

pub(super) fn freeze_time_limit_if_due(
    inner: &BridgeInner,
    now_unix: u64,
) -> Result<Option<OpportunityWindowBoundaryV1>, String> {
    let mut capture = inner
        .deadline_capture
        .lock()
        .map_err(|_| "s1c4_window_capture_lock_poisoned".to_owned())?;
    let Some(capture) = capture.as_mut() else {
        return Ok(None);
    };
    if let Some(error) = &capture.failure {
        return Err(error.clone());
    }
    if let Some(boundary) = &capture.boundary {
        return Ok(Some(boundary.clone()));
    }
    if now_unix <= capture.deadline_at_unix {
        return Ok(None);
    }
    let boundary = match seal_current_prefix(
        inner,
        capture,
        OpportunityWindowClosureV1::TimeLimit,
        capture.deadline_at_unix,
        now_unix,
    ) {
        Ok(boundary) => boundary,
        Err(error) => {
            capture.failure = Some(error.clone());
            return Err(error);
        }
    };
    capture.boundary = Some(boundary.clone());
    Ok(Some(boundary))
}

fn seal_current_prefix(
    inner: &BridgeInner,
    capture: &OpportunityWindowCaptureV1,
    closure: OpportunityWindowClosureV1,
    closes_at_unix: u64,
    frozen_at_unix: u64,
) -> Result<OpportunityWindowBoundaryV1, String> {
    let previous_durable = inner.producer.durable_sequence.load(Ordering::Acquire);
    let end_sequence = inner.producer.last_sequence.load(Ordering::Acquire);
    sync_pending_spool(&inner.pending_dir, previous_durable, end_sequence)?;
    inner
        .producer
        .durable_sequence
        .store(end_sequence, Ordering::Release);
    let boundary = OpportunityWindowBoundaryV1::seal(
        capture.cursor_root_sha256.clone(),
        closure,
        closes_at_unix,
        end_sequence,
        inner.producer.request_events.load(Ordering::Acquire),
        inner.producer.request_input_tokens.load(Ordering::Acquire),
        frozen_at_unix,
    )?;
    write_new_boundary(&capture.boundary_path, &boundary)?;
    Ok(boundary)
}

pub(crate) fn read_boundary(path: &Path) -> Result<Option<OpportunityWindowBoundaryV1>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("s1c4_window_boundary_read:{error}")),
    };
    if bytes.is_empty() || u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_BOUNDARY_BYTES {
        return Err("s1c4_window_boundary_size_invalid".to_owned());
    }
    let boundary: OpportunityWindowBoundaryV1 = serde_json::from_slice(&bytes)
        .map_err(|error| format!("s1c4_window_boundary_decode:{error}"))?;
    boundary.validate()?;
    Ok(Some(boundary))
}

fn write_new_boundary(path: &Path, boundary: &OpportunityWindowBoundaryV1) -> Result<(), String> {
    boundary.validate()?;
    let parent = path
        .parent()
        .ok_or_else(|| "s1c4_window_boundary_parent_missing".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| format!("s1c4_window_boundary_mkdir:{error}"))?;
    let bytes = serde_json::to_vec_pretty(boundary)
        .map_err(|error| format!("s1c4_window_boundary_encode:{error}"))?;
    let temporary = path.with_extension("tmp");
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|error| format!("s1c4_window_boundary_create:{error}"))?;
    let result = file
        .write_all(&bytes)
        .and_then(|()| file.write_all(b"\n"))
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::rename(&temporary, path))
        .and_then(|()| File::open(parent).and_then(|directory| directory.sync_all()));
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(format!("s1c4_window_boundary_publish:{error}"));
    }
    sync_directory(parent)
}

fn unix_now() -> Result<u64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "s1c4_window_clock_before_epoch".to_owned())
        .map(|duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    use super::*;
    use crate::opportunity_bridge::OpportunityBridgeRuntime;

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_directory(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nando-s1c4-window-{label}-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn root(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    #[test]
    fn request_limit_freezes_the_exact_nth_request_and_excludes_the_next() {
        let bridge_root = temp_directory("request-limit-bridge");
        let boundary_path =
            temp_directory("request-limit-boundary").join(S1C4_WINDOW_BOUNDARY_FILE_V1);
        let bridge = OpportunityBridgeRuntime::new(
            bridge_root.clone(),
            true,
            false,
            Duration::from_millis(10),
        )
        .expect("bridge");
        bridge
            .configure_request_deadline_capture(
                root('1'),
                unix_now().expect("clock").saturating_add(600),
                2,
                boundary_path.clone(),
            )
            .expect("capture");
        let first = bridge
            .submit(OpportunityBridgeEventV1::request(
                root('2'),
                11,
                unix_now().expect("clock"),
            ))
            .expect("first");
        let second = bridge
            .submit(OpportunityBridgeEventV1::request(
                root('3'),
                13,
                unix_now().expect("clock"),
            ))
            .expect("second");
        let third = bridge
            .submit(OpportunityBridgeEventV1::request(
                root('4'),
                17,
                unix_now().expect("clock"),
            ))
            .expect("third");
        assert!(first.s1c4_deadline_eligible);
        assert!(second.s1c4_deadline_eligible);
        assert!(!third.s1c4_deadline_eligible);
        let boundary = read_boundary(&boundary_path)
            .expect("boundary read")
            .expect("boundary");
        assert_eq!(boundary.closure, OpportunityWindowClosureV1::RequestLimit);
        assert_eq!(boundary.opportunity_end_sequence, second.sequence);
        assert_eq!(boundary.opportunity_end_request_ordinal, 2);
        assert_eq!(boundary.opportunity_end_input_tokens, 24);
        drop(bridge);
        let _ = fs::remove_dir_all(bridge_root);
        let _ = fs::remove_dir_all(boundary_path.parent().expect("parent"));
    }

    #[test]
    fn time_boundary_is_durable_and_restored_without_polling_drift() {
        let bridge_root = temp_directory("time-limit-bridge");
        let boundary_path =
            temp_directory("time-limit-boundary").join(S1C4_WINDOW_BOUNDARY_FILE_V1);
        let cursor_root = root('5');
        let bridge = OpportunityBridgeRuntime::new(
            bridge_root.clone(),
            true,
            false,
            Duration::from_millis(10),
        )
        .expect("bridge");
        let deadline = unix_now().expect("clock").saturating_add(10);
        bridge
            .configure_request_deadline_capture(
                cursor_root.clone(),
                deadline,
                100,
                boundary_path.clone(),
            )
            .expect("capture");
        let receipt = bridge
            .submit(OpportunityBridgeEventV1::request(
                root('6'),
                19,
                unix_now().expect("clock"),
            ))
            .expect("request");
        let boundary = bridge
            .freeze_request_deadline_boundary(deadline.saturating_add(1))
            .expect("freeze")
            .expect("boundary");
        assert_eq!(boundary.closure, OpportunityWindowClosureV1::TimeLimit);
        assert_eq!(boundary.opportunity_end_sequence, receipt.sequence);
        assert_eq!(boundary.opportunity_end_request_ordinal, 1);
        drop(bridge);

        let restored = OpportunityBridgeRuntime::new(
            bridge_root.clone(),
            true,
            false,
            Duration::from_millis(10),
        )
        .expect("restored bridge");
        restored
            .configure_request_deadline_capture(cursor_root, deadline, 100, boundary_path.clone())
            .expect("restore capture");
        let restored_boundary = restored
            .freeze_request_deadline_boundary(deadline.saturating_add(2))
            .expect("restored freeze")
            .expect("restored boundary");
        assert_eq!(restored_boundary, boundary);
        drop(restored);
        let _ = fs::remove_dir_all(bridge_root);
        let _ = fs::remove_dir_all(boundary_path.parent().expect("parent"));
    }
}
