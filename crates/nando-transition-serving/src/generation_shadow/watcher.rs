use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::UNIX_EPOCH;

use super::{GenerationShadowConfigV3, GenerationShadowRuntimeV3};

pub(super) fn run_generation_shadow_watcher_v3(
    runtime: Arc<GenerationShadowRuntimeV3>,
    config: GenerationShadowConfigV3,
) {
    let mut previous = None;
    loop {
        let current = source_fingerprint(&config);
        if previous.as_ref() != Some(&current) {
            let _ = runtime.reconcile_once();
            previous = Some(current);
        }
        thread::sleep(config.poll_interval);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceFingerprintV3 {
    capture: Option<(u64, u128)>,
    slot_a: Option<(u64, u128)>,
    slot_b: Option<(u64, u128)>,
}

fn source_fingerprint(config: &GenerationShadowConfigV3) -> SourceFingerprintV3 {
    SourceFingerprintV3 {
        capture: file_fingerprint(&config.capture_index_path),
        slot_a: file_fingerprint(
            &config
                .store_path
                .join(nando_operator_persistence::GENERATION_STORE_SLOT_A_FILE_V3),
        ),
        slot_b: file_fingerprint(
            &config
                .store_path
                .join(nando_operator_persistence::GENERATION_STORE_SLOT_B_FILE_V3),
        ),
    }
}

fn file_fingerprint(path: &Path) -> Option<(u64, u128)> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    Some((metadata.len(), modified))
}
