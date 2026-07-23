use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use notify::{EventKind, RecursiveMode, Watcher};

const POLICY_RECONCILE_INTERVAL: Duration = Duration::from_secs(1);

pub struct RuntimePolicyCache {
    mode_path: PathBuf,
    kill_switch_path: PathBuf,
    cpu_mode: AtomicBool,
    kill_switch: AtomicBool,
}

impl RuntimePolicyCache {
    #[must_use]
    pub fn load(mode_path: PathBuf, kill_switch_path: PathBuf) -> Self {
        let cache = Self {
            mode_path,
            kill_switch_path,
            cpu_mode: AtomicBool::new(false),
            kill_switch: AtomicBool::new(true),
        };
        cache.refresh();
        cache
    }

    #[must_use]
    pub fn cpu_mode(&self) -> bool {
        self.cpu_mode.load(Ordering::Acquire)
    }

    #[must_use]
    pub fn kill_switch(&self) -> bool {
        self.kill_switch.load(Ordering::Acquire)
    }

    fn refresh(&self) {
        self.cpu_mode
            .store(read_cpu_mode(&self.mode_path), Ordering::Release);
        self.kill_switch
            .store(self.kill_switch_path.exists(), Ordering::Release);
    }

    fn fail_closed(&self) {
        self.cpu_mode.store(false, Ordering::Release);
        self.kill_switch.store(true, Ordering::Release);
    }
}

pub fn spawn_runtime_policy_watch(cache: Arc<RuntimePolicyCache>) -> Result<(), String> {
    let (sender, receiver) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        let should_forward = match &event {
            Ok(event) => matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            ),
            Err(_) => true,
        };
        if should_forward {
            let _ = sender.send(event);
        }
    })
    .map_err(|error| format!("runtime_policy_watcher_create:{error}"))?;
    let parents = [cache.mode_path.parent(), cache.kill_switch_path.parent()]
        .into_iter()
        .flatten()
        .map(Path::to_path_buf)
        .collect::<BTreeSet<_>>();
    if parents.is_empty() {
        return Err("runtime_policy_parent_missing".to_owned());
    }
    for parent in parents {
        std::fs::create_dir_all(&parent)
            .map_err(|error| format!("runtime_policy_dir:{}:{error}", parent.display()))?;
        watcher
            .watch(&parent, RecursiveMode::NonRecursive)
            .map_err(|error| format!("runtime_policy_watch:{}:{error}", parent.display()))?;
    }
    cache.refresh();
    thread::Builder::new()
        .name("nando-runtime-policy".to_owned())
        .spawn(move || {
            let _watcher = watcher;
            loop {
                match receiver.recv_timeout(POLICY_RECONCILE_INTERVAL) {
                    Ok(Ok(_)) => cache.refresh(),
                    Ok(Err(_)) => cache.fail_closed(),
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // Atomic rename notifications may name only the temporary file.
                        cache.refresh();
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        cache.fail_closed();
                        break;
                    }
                }
            }
        })
        .map(|_| ())
        .map_err(|error| format!("runtime_policy_thread:{error}"))
}

fn read_cpu_mode(path: &Path) -> bool {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|value| {
            value
                .get("mode")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .is_some_and(|mode| mode == "CPU")
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use super::{RuntimePolicyCache, spawn_runtime_policy_watch};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn atomic_policy_replacement_reaches_runtime_cache() -> Result<(), Box<dyn Error>> {
        let root = unique_test_dir()?;
        fs::create_dir_all(&root)?;
        let mode_path = root.join("mode.json");
        let kill_switch_path = root.join("kill-switch");
        atomic_mode_write(&mode_path, "SHADOW")?;

        let cache = Arc::new(RuntimePolicyCache::load(
            mode_path.clone(),
            kill_switch_path.clone(),
        ));
        spawn_runtime_policy_watch(cache.clone()).map_err(std::io::Error::other)?;
        assert!(!cache.cpu_mode());
        assert!(!cache.kill_switch());

        atomic_mode_write(&mode_path, "CPU")?;
        assert!(wait_until(|| cache.cpu_mode()));

        fs::write(&kill_switch_path, b"blocked\n")?;
        assert!(wait_until(|| cache.kill_switch()));

        fs::remove_file(&kill_switch_path)?;
        assert!(wait_until(|| !cache.kill_switch()));

        atomic_mode_write(&mode_path, "SHADOW")?;
        assert!(wait_until(|| !cache.cpu_mode()));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn malformed_or_missing_mode_is_fail_closed() -> Result<(), Box<dyn Error>> {
        let root = unique_test_dir()?;
        fs::create_dir_all(&root)?;
        let mode_path = root.join("mode.json");
        let kill_switch_path = root.join("kill-switch");

        fs::write(&mode_path, br#"{"mode":"CPU"}"#)?;
        let cache = RuntimePolicyCache::load(mode_path.clone(), kill_switch_path);
        assert!(cache.cpu_mode());

        fs::write(&mode_path, b"not-json")?;
        cache.refresh();
        assert!(!cache.cpu_mode());

        fs::remove_file(&mode_path)?;
        cache.refresh();
        assert!(!cache.cpu_mode());

        fs::remove_dir_all(root)?;
        Ok(())
    }

    fn atomic_mode_write(path: &std::path::Path, mode: &str) -> Result<(), Box<dyn Error>> {
        let suffix = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp = path.with_file_name(format!(".mode.test.{suffix}.tmp"));
        fs::write(&temp, format!("{{\"mode\":\"{mode}\"}}\n"))?;
        fs::rename(temp, path)?;
        Ok(())
    }

    fn wait_until(predicate: impl Fn() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if predicate() {
                return true;
            }
            thread::sleep(Duration::from_millis(25));
        }
        predicate()
    }

    fn unique_test_dir() -> Result<std::path::PathBuf, Box<dyn Error>> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let suffix = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        Ok(std::env::temp_dir().join(format!(
            "nando-runtime-policy-{}-{now}-{suffix}",
            std::process::id()
        )))
    }
}
