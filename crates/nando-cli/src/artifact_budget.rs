use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering as AtomicOrdering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

const DEFAULT_ROOT: &str = "target/nando-wave";
const DEFAULT_SOFT_LIMIT_MIB: u64 = 1_024;
const DEFAULT_HARD_LIMIT_MIB: u64 = 2_048;
const DEFAULT_MAX_ARTIFACTS: usize = 4_096;
const WATCHDOG_INTERVAL: Duration = Duration::from_secs(5);

pub(crate) struct ArtifactBudgetGuard {
    root: PathBuf,
    soft_limit_bytes: u64,
    max_artifacts: usize,
    stop: Arc<AtomicBool>,
    watchdog: Option<JoinHandle<()>>,
}

impl ArtifactBudgetGuard {
    pub(crate) fn start() -> Self {
        let root = std::env::var_os("NANDO_ARTIFACT_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_ROOT));
        let soft_limit_bytes = env_mib("NANDO_ARTIFACT_SOFT_LIMIT_MIB", DEFAULT_SOFT_LIMIT_MIB);
        let hard_limit_bytes =
            env_mib("NANDO_ARTIFACT_HARD_LIMIT_MIB", DEFAULT_HARD_LIMIT_MIB).max(soft_limit_bytes);
        let max_artifacts = std::env::var("NANDO_ARTIFACT_MAX_COUNT")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_MAX_ARTIFACTS);

        report_prune(enforce_budget(&root, soft_limit_bytes, max_artifacts));

        let stop = Arc::new(AtomicBool::new(false));
        let watchdog_stop = Arc::clone(&stop);
        let watchdog_root = root.clone();
        let watchdog = thread::Builder::new()
            .name("nando-artifact-budget".to_owned())
            .spawn(move || {
                loop {
                    thread::park_timeout(WATCHDOG_INTERVAL);
                    if watchdog_stop.load(AtomicOrdering::Relaxed) {
                        break;
                    }
                    let Ok(bytes) = directory_size(&watchdog_root) else {
                        continue;
                    };
                    if bytes <= hard_limit_bytes {
                        continue;
                    }
                    eprintln!(
                        "artifact_budget_exceeded root={} bytes={} hard_limit_bytes={}",
                        watchdog_root.display(),
                        bytes,
                        hard_limit_bytes
                    );
                    report_prune(enforce_budget(
                        &watchdog_root,
                        soft_limit_bytes,
                        max_artifacts,
                    ));
                    std::process::exit(74);
                }
            })
            .ok();

        Self {
            root,
            soft_limit_bytes,
            max_artifacts,
            stop,
            watchdog,
        }
    }
}

impl Drop for ArtifactBudgetGuard {
    fn drop(&mut self) {
        self.stop.store(true, AtomicOrdering::Relaxed);
        if let Some(watchdog) = self.watchdog.take() {
            watchdog.thread().unpark();
            let _ = watchdog.join();
        }
        report_prune(enforce_budget(
            &self.root,
            self.soft_limit_bytes,
            self.max_artifacts,
        ));
    }
}

#[derive(Debug)]
struct ArtifactUnit {
    path: PathBuf,
    bytes: u64,
    modified: SystemTime,
}

#[derive(Debug)]
struct PruneReport {
    removed_artifacts: usize,
    removed_bytes: u64,
    remaining_bytes: u64,
}

fn env_mib(name: &str, default_mib: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default_mib)
        .saturating_mul(1024 * 1024)
}

fn enforce_budget(
    root: &Path,
    soft_limit_bytes: u64,
    max_artifacts: usize,
) -> Result<PruneReport, String> {
    if !validate_root(root)? {
        return Ok(PruneReport {
            removed_artifacts: 0,
            removed_bytes: 0,
            remaining_bytes: 0,
        });
    }

    let mut units = artifact_units(root)?;
    let mut remaining_bytes = units
        .iter()
        .fold(0_u64, |total, unit| total.saturating_add(unit.bytes));
    units.sort_by(|left, right| {
        left.modified
            .cmp(&right.modified)
            .then_with(|| left.path.cmp(&right.path))
    });

    let mut removed_artifacts = 0_usize;
    let mut removed_bytes = 0_u64;
    let mut retained_artifacts = units.len();
    for unit in units {
        if remaining_bytes <= soft_limit_bytes && retained_artifacts <= max_artifacts {
            break;
        }
        // Keep one completed artifact if it fits below the hard watchdog limit.
        if retained_artifacts <= 1 {
            break;
        }
        remove_artifact(&unit.path)?;
        remaining_bytes = remaining_bytes.saturating_sub(unit.bytes);
        removed_bytes = removed_bytes.saturating_add(unit.bytes);
        removed_artifacts = removed_artifacts.saturating_add(1);
        retained_artifacts = retained_artifacts.saturating_sub(1);
    }

    remove_empty_namespaces(root)?;
    Ok(PruneReport {
        removed_artifacts,
        removed_bytes,
        remaining_bytes,
    })
}

fn artifact_units(root: &Path) -> Result<Vec<ArtifactUnit>, String> {
    let mut units = Vec::new();
    for entry in read_dir(root)? {
        let entry =
            entry.map_err(|error| format!("artifact_read_entry:{}:{error}", root.display()))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("artifact_metadata:{}:{error}", path.display()))?;
        if metadata.is_dir() {
            for child in read_dir(&path)? {
                let child = child
                    .map_err(|error| format!("artifact_read_entry:{}:{error}", path.display()))?;
                units.push(measure_unit(child.path())?);
            }
        } else {
            units.push(measure_unit(path)?);
        }
    }
    Ok(units)
}

fn measure_unit(path: PathBuf) -> Result<ArtifactUnit, String> {
    let (bytes, modified) = measure_path(&path)?;
    Ok(ArtifactUnit {
        path,
        bytes,
        modified,
    })
}

fn measure_path(path: &Path) -> Result<(u64, SystemTime), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("artifact_metadata:{}:{error}", path.display()))?;
    let mut modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    if !metadata.is_dir() {
        return Ok((metadata.len(), modified));
    }

    let mut bytes = 0_u64;
    for entry in read_dir(path)? {
        let entry =
            entry.map_err(|error| format!("artifact_read_entry:{}:{error}", path.display()))?;
        let (child_bytes, child_modified) = measure_path(&entry.path())?;
        bytes = bytes.saturating_add(child_bytes);
        if child_modified.cmp(&modified) == Ordering::Greater {
            modified = child_modified;
        }
    }
    Ok((bytes, modified))
}

fn directory_size(root: &Path) -> Result<u64, String> {
    if !validate_root(root)? {
        return Ok(0);
    }
    measure_path(root).map(|(bytes, _)| bytes)
}

fn validate_root(root: &Path) -> Result<bool, String> {
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("artifact_root_metadata:{}:{error}", root.display())),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!(
            "artifact_root_not_real_directory:{}",
            root.display()
        ));
    }
    Ok(true)
}

fn read_dir(path: &Path) -> Result<fs::ReadDir, String> {
    fs::read_dir(path).map_err(|error| format!("artifact_read_dir:{}:{error}", path.display()))
}

fn remove_artifact(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("artifact_remove_metadata:{}:{error}", path.display()))?;
    if metadata.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("artifact_remove_dir:{}:{error}", path.display()))
    } else {
        fs::remove_file(path)
            .map_err(|error| format!("artifact_remove_file:{}:{error}", path.display()))
    }
}

fn remove_empty_namespaces(root: &Path) -> Result<(), String> {
    for entry in read_dir(root)? {
        let entry =
            entry.map_err(|error| format!("artifact_read_entry:{}:{error}", root.display()))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("artifact_metadata:{}:{error}", path.display()))?;
        if metadata.is_dir() {
            let mut children = read_dir(&path)?;
            if children.next().is_none() {
                fs::remove_dir(&path)
                    .map_err(|error| format!("artifact_remove_empty:{}:{error}", path.display()))?;
            }
        }
    }
    Ok(())
}

fn report_prune(result: Result<PruneReport, String>) {
    match result {
        Ok(report) if report.removed_artifacts > 0 => eprintln!(
            "artifact_budget_pruned artifacts={} bytes={} remaining_bytes={}",
            report.removed_artifacts, report.removed_bytes, report.remaining_bytes
        ),
        Ok(_) => {}
        Err(error) => eprintln!("artifact_budget_error {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nando-artifact-budget-{label}-{}",
            std::process::id()
        ))
    }

    #[test]
    fn prunes_oldest_units_to_byte_budget() -> Result<(), String> {
        let root = test_root("prune");
        let namespace = root.join("streaming");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&namespace).map_err(|error| error.to_string())?;
        fs::write(namespace.join("a-old"), [0_u8; 8]).map_err(|error| error.to_string())?;
        fs::write(namespace.join("b-new"), [0_u8; 8]).map_err(|error| error.to_string())?;

        let report = enforce_budget(&root, 8, 10)?;
        assert_eq!(report.removed_artifacts, 1);
        assert_eq!(report.remaining_bytes, 8);
        assert!(!namespace.join("a-old").exists());
        assert!(namespace.join("b-new").exists());
        fs::remove_dir_all(root).map_err(|error| error.to_string())
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlink_root() -> Result<(), String> {
        use std::os::unix::fs::symlink;

        let real = test_root("real");
        let link = test_root("link");
        let _ = fs::remove_dir_all(&real);
        let _ = fs::remove_file(&link);
        fs::create_dir_all(&real).map_err(|error| error.to_string())?;
        symlink(&real, &link).map_err(|error| error.to_string())?;

        let result = enforce_budget(&link, 8, 10);
        assert!(result.is_err());
        fs::remove_file(link).map_err(|error| error.to_string())?;
        fs::remove_dir_all(real).map_err(|error| error.to_string())
    }
}
