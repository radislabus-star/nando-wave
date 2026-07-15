use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;

use notify::{EventKind, RecursiveMode, Watcher};

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
    let target_paths = [cache.mode_path.clone(), cache.kill_switch_path.clone()]
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        let should_forward = match &event {
            Ok(event) => {
                matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                ) && event.paths.iter().any(|path| target_paths.contains(path))
            }
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
            while let Ok(event) = receiver.recv() {
                if event.is_ok() {
                    cache.refresh();
                } else {
                    cache.fail_closed();
                }
            }
            cache.fail_closed();
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
