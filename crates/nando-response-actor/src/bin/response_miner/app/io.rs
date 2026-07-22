//! Fail-closed proof loading and atomic report persistence.

use super::*;

pub(super) fn causal_proof_passes(path: &Path) -> bool {
    let Some(proof) = fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
    else {
        return false;
    };
    let number = |key: &str| proof.get(key).and_then(Value::as_u64);
    proof.get("schema").and_then(Value::as_str) == Some("nando.response-wave-causal-proof.v1")
        && proof.get("verdict").and_then(Value::as_str) == Some("PASS")
        && number("heldout_correct") == number("heldout_total")
        && number("heldout_total").is_some_and(|total| total >= 32)
        && number("full_phase_exact_checks")
            .zip(number("no_phase_exact_checks"))
            .is_some_and(|(full, no_phase)| full < no_phase)
        && number("full_phase_exact_checks")
            .zip(number("shuffled_phase_exact_checks"))
            .is_some_and(|(full, shuffled)| full < shuffled)
}

fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    atomic_write_value(
        path,
        &serde_json::to_value(value).map_err(|error| error.to_string())?,
    )
}

fn atomic_write_value(path: &Path, value: &Value) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("no_parent:{}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| format!("mkdir:{}:{error}", parent.display()))?;
    let temporary = parent.join(format!(
        ".{}.new",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("registry")
    ));
    let mut file = fs::File::create(&temporary)
        .map_err(|error| format!("create:{}:{error}", temporary.display()))?;
    serde_json::to_writer_pretty(&mut file, value)
        .map_err(|error| format!("serialize:{}:{error}", temporary.display()))?;
    file.write_all(b"\n")
        .map_err(|error| format!("write:{}:{error}", temporary.display()))?;
    file.sync_all()
        .map_err(|error| format!("sync:{}:{error}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|error| format!("rename:{}:{error}", path.display()))
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
