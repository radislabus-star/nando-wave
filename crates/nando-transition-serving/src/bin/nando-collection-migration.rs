use std::env;
use std::path::PathBuf;
use std::time::Duration;

use nando_response_actor::OnlineCollectionConfig;
use nando_transition_serving::session_backfill::run_collection_migration_pass;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let sessions_root = PathBuf::from(args.next().ok_or_else(usage)?);
    let migration_checkpoint = PathBuf::from(args.next().ok_or_else(usage)?);
    let collection_checkpoint = PathBuf::from(args.next().ok_or_else(usage)?);
    let max_seconds = args
        .next()
        .map(|value| value.parse::<u64>())
        .transpose()
        .map_err(|error| format!("max_seconds:{error}"))?
        .unwrap_or(25)
        .clamp(1, 30);
    if args.next().is_some() {
        return Err(usage());
    }
    let report = run_collection_migration_pass(
        &sessions_root,
        &migration_checkpoint,
        &collection_checkpoint,
        OnlineCollectionConfig::default(),
        Duration::from_secs(max_seconds),
    )?;
    println!(
        "{}",
        serde_json::to_string(&report)
            .map_err(|error| format!("collection_migration_report:{error}"))?
    );
    Ok(())
}

fn usage() -> String {
    "usage: nando-collection-migration <sessions-root> <migration-checkpoint> <collection-checkpoint> [max-seconds<=30]".to_owned()
}
