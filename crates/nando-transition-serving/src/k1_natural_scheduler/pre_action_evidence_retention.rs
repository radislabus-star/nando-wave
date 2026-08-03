use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use nando_operator_kernel::canonical_json_sha256;
use serde::{Deserialize, Serialize};

use crate::operator_certification::CertificationAuthorityConfigV1;

const MAX_FILES: usize = 256;
const MAX_BYTES: u64 = 64 * 1024 * 1024;
const MAX_AGE_SECONDS: u64 = 30 * 24 * 60 * 60;
const INTENT_SCHEMA_V2: &str = "nando.k1-pre-action-evidence-cleanup-intent.v2";
const COMPLETION_SCHEMA_V2: &str = "nando.k1-pre-action-evidence-cleanup-completion.v2";
const RECEIPT_DIR: &str = "k1-pre-action-authority-evidence-cleanup-v2";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CleanupIntentV2 {
    schema: String,
    terminal_verdict_root_sha256: String,
    removal_capture_roots_sha256: Vec<String>,
    removal_bytes: u64,
    max_files: usize,
    max_bytes: u64,
    max_age_seconds: u64,
    planned_at_unix: u64,
    intent_root_sha256: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CleanupCompletionV2 {
    schema: String,
    terminal_verdict_root_sha256: String,
    intent_root_sha256: String,
    remaining_files: usize,
    remaining_bytes: u64,
    cleaned_at_unix: u64,
    completion_root_sha256: String,
}

struct EvidenceFile {
    capture_root: String,
    bytes: u64,
    modified_unix: u64,
}

pub(super) fn prune_after_terminal_verdict(
    config: &CertificationAuthorityConfigV1,
    terminal_verdict_root_sha256: &str,
) -> Result<(), String> {
    prune_root(&config.root, terminal_verdict_root_sha256)
}

fn prune_root(root: &Path, terminal_verdict_root_sha256: &str) -> Result<(), String> {
    let directory = root.join("k1-pre-action-authority-evidence-v1");
    let transition_dir = root.join(RECEIPT_DIR).join(terminal_verdict_root_sha256);
    let intent_path = transition_dir.join("intent.json");
    let completion_path = transition_dir.join("completion.json");
    if completion_path.exists() {
        validate_completion(&completion_path, terminal_verdict_root_sha256)?;
        return Ok(());
    }
    let intent = if intent_path.exists() {
        load_intent(&intent_path, terminal_verdict_root_sha256)?
    } else {
        let intent = plan_cleanup(&directory, terminal_verdict_root_sha256)?;
        fs::create_dir_all(&transition_dir)
            .map_err(|error| format!("k1_pre_action_retention_receipt_parent:{error}"))?;
        let bytes = serde_json::to_vec_pretty(&intent)
            .map_err(|error| format!("k1_pre_action_retention_intent_encode:{error}"))?;
        crate::write_bytes_atomic(&intent_path, &bytes, "k1-pre-action-retention-intent")?;
        intent
    };
    if directory.exists() {
        for capture_root in &intent.removal_capture_roots_sha256 {
            let path = directory.join(format!("{capture_root}.cbor"));
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("k1_pre_action_retention_remove:{error}")),
            }
        }
        fs::File::open(&directory)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("k1_pre_action_retention_directory_sync:{error}"))?;
    }
    let (remaining_files, remaining_bytes) = evidence_totals(&directory)?;
    let cleaned_at_unix = unix_now()?;
    let completion_root_sha256 = canonical_json_sha256(&(
        COMPLETION_SCHEMA_V2,
        terminal_verdict_root_sha256,
        intent.intent_root_sha256.as_str(),
        remaining_files,
        remaining_bytes,
        cleaned_at_unix,
    ))
    .map_err(|error| format!("k1_pre_action_retention_completion_root:{error}"))?;
    let completion = CleanupCompletionV2 {
        schema: COMPLETION_SCHEMA_V2.to_owned(),
        terminal_verdict_root_sha256: terminal_verdict_root_sha256.to_owned(),
        intent_root_sha256: intent.intent_root_sha256,
        remaining_files,
        remaining_bytes,
        cleaned_at_unix,
        completion_root_sha256,
    };
    let bytes = serde_json::to_vec_pretty(&completion)
        .map_err(|error| format!("k1_pre_action_retention_completion_encode:{error}"))?;
    crate::write_bytes_atomic(
        &completion_path,
        &bytes,
        "k1-pre-action-retention-completion",
    )
}

fn plan_cleanup(
    directory: &Path,
    terminal_verdict_root_sha256: &str,
) -> Result<CleanupIntentV2, String> {
    let now = unix_now()?;
    let mut files = evidence_files(directory)?;
    files.sort_by_key(|file| (file.modified_unix, file.capture_root.clone()));
    let mut remaining_bytes = files.iter().map(|file| file.bytes).sum::<u64>();
    let mut remaining_files = files.len();
    let mut removal_capture_roots_sha256 = Vec::new();
    let mut removal_bytes = 0_u64;
    for file in files {
        let expired = now.saturating_sub(file.modified_unix) > MAX_AGE_SECONDS;
        if !expired && remaining_files <= MAX_FILES && remaining_bytes <= MAX_BYTES {
            continue;
        }
        remaining_files = remaining_files.saturating_sub(1);
        remaining_bytes = remaining_bytes.saturating_sub(file.bytes);
        removal_bytes = removal_bytes.saturating_add(file.bytes);
        removal_capture_roots_sha256.push(file.capture_root);
    }
    removal_capture_roots_sha256.sort();
    let intent_root_sha256 = canonical_json_sha256(&(
        INTENT_SCHEMA_V2,
        terminal_verdict_root_sha256,
        &removal_capture_roots_sha256,
        removal_bytes,
        MAX_FILES,
        MAX_BYTES,
        MAX_AGE_SECONDS,
        now,
    ))
    .map_err(|error| format!("k1_pre_action_retention_intent_root:{error}"))?;
    Ok(CleanupIntentV2 {
        schema: INTENT_SCHEMA_V2.to_owned(),
        terminal_verdict_root_sha256: terminal_verdict_root_sha256.to_owned(),
        removal_capture_roots_sha256,
        removal_bytes,
        max_files: MAX_FILES,
        max_bytes: MAX_BYTES,
        max_age_seconds: MAX_AGE_SECONDS,
        planned_at_unix: now,
        intent_root_sha256,
    })
}

fn evidence_files(directory: &Path) -> Result<Vec<EvidenceFile>, String> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    fs::read_dir(directory)
        .map_err(|error| format!("k1_pre_action_retention_read_dir:{error}"))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension().and_then(|value| value.to_str()) == Some("cbor")).then_some(path)
        })
        .map(|path| {
            let metadata = fs::metadata(&path)
                .map_err(|error| format!("k1_pre_action_retention_metadata:{error}"))?;
            let modified_unix = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |duration| duration.as_secs());
            let capture_root = path
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or_else(|| "k1_pre_action_retention_file_name_invalid".to_owned())?
                .to_owned();
            Ok(EvidenceFile {
                capture_root,
                bytes: metadata.len(),
                modified_unix,
            })
        })
        .collect::<Result<Vec<_>, String>>()
}

fn evidence_totals(directory: &Path) -> Result<(usize, u64), String> {
    let files = evidence_files(directory)?;
    Ok((files.len(), files.iter().map(|file| file.bytes).sum()))
}

fn load_intent(path: &Path, terminal_root: &str) -> Result<CleanupIntentV2, String> {
    let intent: CleanupIntentV2 = serde_json::from_slice(
        &fs::read(path).map_err(|error| format!("k1_pre_action_retention_intent_read:{error}"))?,
    )
    .map_err(|error| format!("k1_pre_action_retention_intent_decode:{error}"))?;
    let expected = canonical_json_sha256(&(
        INTENT_SCHEMA_V2,
        intent.terminal_verdict_root_sha256.as_str(),
        &intent.removal_capture_roots_sha256,
        intent.removal_bytes,
        intent.max_files,
        intent.max_bytes,
        intent.max_age_seconds,
        intent.planned_at_unix,
    ))
    .map_err(|error| format!("k1_pre_action_retention_intent_root:{error}"))?;
    if intent.schema != INTENT_SCHEMA_V2
        || intent.terminal_verdict_root_sha256 != terminal_root
        || intent.intent_root_sha256 != expected
    {
        return Err("k1_pre_action_retention_intent_invalid".to_owned());
    }
    Ok(intent)
}

fn validate_completion(path: &Path, terminal_root: &str) -> Result<(), String> {
    let completion: CleanupCompletionV2 = serde_json::from_slice(
        &fs::read(path)
            .map_err(|error| format!("k1_pre_action_retention_completion_read:{error}"))?,
    )
    .map_err(|error| format!("k1_pre_action_retention_completion_decode:{error}"))?;
    let expected = canonical_json_sha256(&(
        COMPLETION_SCHEMA_V2,
        completion.terminal_verdict_root_sha256.as_str(),
        completion.intent_root_sha256.as_str(),
        completion.remaining_files,
        completion.remaining_bytes,
        completion.cleaned_at_unix,
    ))
    .map_err(|error| format!("k1_pre_action_retention_completion_root:{error}"))?;
    if completion.schema != COMPLETION_SCHEMA_V2
        || completion.terminal_verdict_root_sha256 != terminal_root
        || completion.completion_root_sha256 != expected
    {
        return Err("k1_pre_action_retention_completion_invalid".to_owned());
    }
    Ok(())
}

fn unix_now() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| format!("k1_pre_action_retention_clock:{error}"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use nando_operator_kernel::sha256_bytes;

    use super::{MAX_FILES, plan_cleanup, prune_root};

    #[test]
    fn terminal_cleanup_is_bounded_and_receipted() {
        let root = std::env::temp_dir().join(format!(
            "nando-k1-evidence-retention-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let evidence = root.join("k1-pre-action-authority-evidence-v1");
        fs::create_dir_all(&evidence).expect("evidence directory");
        for index in 0..=MAX_FILES {
            fs::write(
                evidence.join(format!(
                    "{}.cbor",
                    sha256_bytes(index.to_string().as_bytes())
                )),
                [0_u8; 8],
            )
            .expect("evidence file");
        }
        let terminal = sha256_bytes(b"terminal");
        prune_root(&root, &terminal).expect("terminal cleanup");
        assert_eq!(
            fs::read_dir(&evidence).expect("remaining evidence").count(),
            MAX_FILES
        );
        assert!(
            root.join("k1-pre-action-authority-evidence-cleanup-v2")
                .join(&terminal)
                .join("completion.json")
                .is_file()
        );
        prune_root(&root, &terminal).expect("idempotent cleanup retry");
        fs::remove_dir_all(root).expect("cleanup test root");
    }

    #[test]
    fn partial_cleanup_resumes_from_durable_intent() {
        let root = std::env::temp_dir().join(format!(
            "nando-k1-evidence-retention-resume-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let evidence = root.join("k1-pre-action-authority-evidence-v1");
        fs::create_dir_all(&evidence).expect("evidence directory");
        for index in 0..=MAX_FILES {
            fs::write(
                evidence.join(format!(
                    "{}.cbor",
                    sha256_bytes(index.to_string().as_bytes())
                )),
                [0_u8; 8],
            )
            .expect("evidence file");
        }
        let terminal = sha256_bytes(b"terminal-resume");
        let intent = plan_cleanup(&evidence, &terminal).expect("cleanup intent");
        let transition = root
            .join("k1-pre-action-authority-evidence-cleanup-v2")
            .join(&terminal);
        fs::create_dir_all(&transition).expect("transition directory");
        fs::write(
            transition.join("intent.json"),
            serde_json::to_vec_pretty(&intent).expect("encode intent"),
        )
        .expect("persist intent");
        let first = intent
            .removal_capture_roots_sha256
            .first()
            .expect("planned removal");
        fs::remove_file(evidence.join(format!("{first}.cbor"))).expect("partial removal");

        prune_root(&root, &terminal).expect("resume cleanup");
        assert_eq!(
            fs::read_dir(&evidence).expect("remaining evidence").count(),
            MAX_FILES
        );
        assert!(transition.join("completion.json").is_file());
        fs::remove_dir_all(root).expect("cleanup test root");
    }
}
