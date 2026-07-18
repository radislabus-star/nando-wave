use std::env;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

#[path = "../opportunity_scan.rs"]
mod opportunity_scan;

use opportunity_scan::{OpportunityScanStatus, spawn_opportunity_scan};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let sessions_root = PathBuf::from(args.next().ok_or_else(usage)?);
    let checkpoint_path = PathBuf::from(args.next().ok_or_else(usage)?);
    let max_seconds = args
        .next()
        .map(|value| value.parse::<u64>())
        .transpose()
        .map_err(|error| format!("max_seconds:{error}"))?
        .unwrap_or(55)
        .clamp(1, 55);
    if args.next().is_some() {
        return Err(usage());
    }

    let shared = Arc::new(RwLock::new(OpportunityScanStatus::default()));
    spawn_opportunity_scan(sessions_root, checkpoint_path, Arc::clone(&shared))?;
    let started = Instant::now();
    loop {
        thread::sleep(Duration::from_millis(100));
        let status = shared
            .read()
            .map_err(|_| "opportunity_scan_status_lock_poisoned".to_owned())?
            .clone();
        let complete = status.ready
            && !status.busy
            && status.source_files_complete == status.source_files_seen;
        if complete || started.elapsed() >= Duration::from_secs(max_seconds) {
            println!(
                "{}",
                serde_json::to_string(&status)
                    .map_err(|error| format!("opportunity_scan_report:{error}"))?
            );
            return if complete {
                Ok(())
            } else {
                Err("opportunity_scan_timeout".to_owned())
            };
        }
    }
}

fn usage() -> String {
    "usage: nando-opportunity-scan <sessions-root> <checkpoint-path> [max-seconds<=55]".to_owned()
}
