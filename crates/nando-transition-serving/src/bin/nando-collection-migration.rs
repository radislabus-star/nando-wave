use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use std::time::Duration;

use nando_response_actor::{
    OnlineCollectionConfig, OnlineCollectionMiner, OnlineCollectionObservation,
    diagnose_response_dynamic_coverage, enumerate_source_neutral_response_programs,
    read_framed_cbor, response_program_authority_matches_example,
    response_program_exactly_matches_example, response_program_kind,
    response_program_matches_example,
};
use nando_transition_serving::session_backfill::{
    run_collection_migration_pass, run_collection_rehydration_pass,
};
use nando_transition_serving::verified_collection_observations_from_session;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let first = args.next().ok_or_else(usage)?;
    if first == "--diagnose-ledger" {
        let ledger = PathBuf::from(args.next().ok_or_else(usage)?);
        if args.next().is_some() {
            return Err(usage());
        }
        let observations =
            read_framed_cbor::<OnlineCollectionObservation>(&ledger, "collection-observation")?;
        println!("{}", diagnose_observations(observations));
        return Ok(());
    }
    if first == "--diagnose-session" {
        let session = PathBuf::from(args.next().ok_or_else(usage)?);
        if args.next().is_some() {
            return Err(usage());
        }
        let observations = verified_collection_observations_from_session(&session)?;
        println!("{}", diagnose_observations(observations));
        return Ok(());
    }
    if first == "--status-checkpoint" {
        let checkpoint = PathBuf::from(args.next().ok_or_else(usage)?);
        if args.next().is_some() {
            return Err(usage());
        }
        let miner = OnlineCollectionMiner::open(checkpoint, OnlineCollectionConfig::default())?;
        println!(
            "{}",
            serde_json::to_string(&miner.status())
                .map_err(|error| format!("collection_status_report:{error}"))?
        );
        return Ok(());
    }
    if first == "--diagnose-checkpoint" {
        let checkpoint = PathBuf::from(args.next().ok_or_else(usage)?);
        let bucket_id = args.next();
        if args.next().is_some() {
            return Err(usage());
        }
        let miner = OnlineCollectionMiner::open(checkpoint, OnlineCollectionConfig::default())?;
        let report = if let Some(bucket_id) = bucket_id {
            vec![
                miner
                    .consensus_diagnostic_for_bucket(&bucket_id)
                    .ok_or_else(|| format!("collection_diagnostic_bucket_missing:{bucket_id}"))?,
            ]
        } else {
            miner.consensus_diagnostics()
        };
        println!(
            "{}",
            serde_json::to_string(&report)
                .map_err(|error| format!("collection_diagnostic_report:{error}"))?
        );
        return Ok(());
    }
    let sessions_root = PathBuf::from(first);
    let migration_checkpoint = PathBuf::from(args.next().ok_or_else(usage)?);
    let collection_checkpoint = PathBuf::from(args.next().ok_or_else(usage)?);
    let max_seconds = args
        .next()
        .map(|value| value.parse::<u64>())
        .transpose()
        .map_err(|error| format!("max_seconds:{error}"))?
        .unwrap_or(25)
        .clamp(1, 30);
    let rehydrate_only = match args.next().as_deref() {
        None => false,
        Some("--rehydrate-only") => true,
        Some(_) => return Err(usage()),
    };
    if args.next().is_some() {
        return Err(usage());
    }
    let run = if rehydrate_only {
        run_collection_rehydration_pass
    } else {
        run_collection_migration_pass
    };
    let report = run(
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
    "usage: nando-collection-migration --diagnose-ledger <collection-ledger-dir>\n       nando-collection-migration --diagnose-session <session-jsonl>\n       nando-collection-migration --status-checkpoint <collection-checkpoint>\n       nando-collection-migration --diagnose-checkpoint <collection-checkpoint> [bucket-id]\n       nando-collection-migration <sessions-root> <migration-checkpoint> <collection-checkpoint> [max-seconds<=30] [--rehydrate-only]".to_owned()
}

fn diagnose_observations(observations: Vec<OnlineCollectionObservation>) -> serde_json::Value {
    let mut rows = 0_u64;
    let mut enumeration_errors = 0_u64;
    let mut no_candidates = 0_u64;
    let mut teacher_match_rows = 0_u64;
    let mut authority_match_rows = 0_u64;
    let mut exact_match_rows = 0_u64;
    let mut dynamic_partial_rows = 0_u64;
    let mut no_candidates_dynamic_zero = 0_u64;
    let mut no_candidates_dynamic_partial = 0_u64;
    let mut no_candidates_dynamic_full = 0_u64;
    let mut no_candidates_with_request_source = 0_u64;
    let mut no_candidates_with_tool_source = 0_u64;
    let mut no_candidates_matching_selectors = 0_u64;
    let mut no_candidates_response_at_most_512 = 0_u64;
    let mut no_candidates_response_over_512 = 0_u64;
    let mut no_candidates_static_at_most_512 = 0_u64;
    let mut no_candidates_short_dynamic_zero = 0_u64;
    let mut no_candidates_short_dynamic_partial = 0_u64;
    let mut no_candidates_short_partial_exact_surface = 0_u64;
    let mut short_partial_candidates_enumerated = 0_u64;
    let mut short_partial_policy_reasons = BTreeMap::<String, u64>::new();
    let mut short_partial_canonical_rejection_reasons = BTreeMap::<String, u64>::new();
    let mut candidates_enumerated = 0_u64;
    let mut policy_rejected_exact_matches = 0_u64;
    let mut policy_rejection_reasons = BTreeMap::<String, u64>::new();
    let mut canonical_rejection_reasons = BTreeMap::<String, u64>::new();
    let mut teacher_without_authority_rows = 0_u64;
    let mut teacher_kinds = BTreeMap::<String, u64>::new();
    let mut authority_kinds = BTreeMap::<String, u64>::new();
    for observation in observations {
        rows = rows.saturating_add(1);
        let coverage = diagnose_response_dynamic_coverage(&observation.example);
        if coverage.dynamic_bytes > 0 && coverage.dynamic_bytes < coverage.response_bytes {
            dynamic_partial_rows = dynamic_partial_rows.saturating_add(1);
        }
        let space = match enumerate_source_neutral_response_programs(&observation.example) {
            Ok(space) => space,
            Err(_) => {
                enumeration_errors = enumeration_errors.saturating_add(1);
                continue;
            }
        };
        candidates_enumerated = candidates_enumerated
            .saturating_add(u64::try_from(space.candidates_enumerated).unwrap_or(u64::MAX));
        policy_rejected_exact_matches = policy_rejected_exact_matches
            .saturating_add(u64::try_from(space.policy_rejected_exact_matches).unwrap_or(u64::MAX));
        for (reason, count) in &space.policy_rejection_reasons {
            *policy_rejection_reasons.entry(reason.clone()).or_default() +=
                u64::try_from(*count).unwrap_or(u64::MAX);
        }
        for (reason, count) in &space.canonical_rejection_reasons {
            *canonical_rejection_reasons
                .entry(reason.clone())
                .or_default() += u64::try_from(*count).unwrap_or(u64::MAX);
        }
        if space.programs.is_empty() {
            no_candidates = no_candidates.saturating_add(1);
            if coverage.dynamic_bytes == 0 {
                no_candidates_dynamic_zero = no_candidates_dynamic_zero.saturating_add(1);
            } else if coverage.dynamic_bytes < coverage.response_bytes {
                no_candidates_dynamic_partial = no_candidates_dynamic_partial.saturating_add(1);
            } else {
                no_candidates_dynamic_full = no_candidates_dynamic_full.saturating_add(1);
            }
            no_candidates_with_request_source = no_candidates_with_request_source
                .saturating_add(u64::from(coverage.request_dynamic_bytes > 0));
            no_candidates_with_tool_source = no_candidates_with_tool_source
                .saturating_add(u64::from(coverage.tool_dynamic_bytes > 0));
            no_candidates_matching_selectors = no_candidates_matching_selectors
                .saturating_add(u64::try_from(coverage.matching_selectors).unwrap_or(u64::MAX));
            if coverage.response_bytes <= 512 {
                no_candidates_response_at_most_512 =
                    no_candidates_response_at_most_512.saturating_add(1);
                no_candidates_short_dynamic_zero = no_candidates_short_dynamic_zero
                    .saturating_add(u64::from(coverage.dynamic_bytes == 0));
                no_candidates_short_dynamic_partial = no_candidates_short_dynamic_partial
                    .saturating_add(u64::from(
                        coverage.dynamic_bytes > 0
                            && coverage.dynamic_bytes < coverage.response_bytes,
                    ));
                no_candidates_short_partial_exact_surface =
                    no_candidates_short_partial_exact_surface.saturating_add(u64::from(
                        coverage.dynamic_bytes > 0
                            && coverage.dynamic_bytes < coverage.response_bytes
                            && coverage.exact_surface_required,
                    ));
                if coverage.dynamic_bytes > 0 && coverage.dynamic_bytes < coverage.response_bytes {
                    short_partial_candidates_enumerated = short_partial_candidates_enumerated
                        .saturating_add(
                            u64::try_from(space.candidates_enumerated).unwrap_or(u64::MAX),
                        );
                    for (reason, count) in &space.policy_rejection_reasons {
                        *short_partial_policy_reasons
                            .entry(reason.clone())
                            .or_default() += u64::try_from(*count).unwrap_or(u64::MAX);
                    }
                    for (reason, count) in &space.canonical_rejection_reasons {
                        *short_partial_canonical_rejection_reasons
                            .entry(reason.clone())
                            .or_default() += u64::try_from(*count).unwrap_or(u64::MAX);
                    }
                }
            } else {
                no_candidates_response_over_512 = no_candidates_response_over_512.saturating_add(1);
            }
            no_candidates_static_at_most_512 =
                no_candidates_static_at_most_512.saturating_add(u64::from(
                    coverage
                        .response_bytes
                        .saturating_sub(coverage.dynamic_bytes)
                        <= 512,
                ));
            continue;
        }
        let mut teacher = false;
        let mut authority = false;
        let mut exact = false;
        for program in &space.programs {
            if response_program_matches_example(program, &observation.example) {
                teacher = true;
                *teacher_kinds
                    .entry(format!("{:?}", response_program_kind(program)))
                    .or_default() += 1;
            }
            if response_program_authority_matches_example(program, &observation.example) {
                authority = true;
                *authority_kinds
                    .entry(format!("{:?}", response_program_kind(program)))
                    .or_default() += 1;
            }
            exact |= response_program_exactly_matches_example(program, &observation.example);
        }
        teacher_match_rows = teacher_match_rows.saturating_add(u64::from(teacher));
        authority_match_rows = authority_match_rows.saturating_add(u64::from(authority));
        exact_match_rows = exact_match_rows.saturating_add(u64::from(exact));
        teacher_without_authority_rows =
            teacher_without_authority_rows.saturating_add(u64::from(teacher && !authority));
    }
    serde_json::json!({
        "schema": "nando.collection-observation-diagnostic.v1",
        "rows": rows,
        "enumeration_errors": enumeration_errors,
        "no_candidates": no_candidates,
        "dynamic_partial_rows": dynamic_partial_rows,
        "no_candidates_dynamic_zero": no_candidates_dynamic_zero,
        "no_candidates_dynamic_partial": no_candidates_dynamic_partial,
        "no_candidates_dynamic_full": no_candidates_dynamic_full,
        "no_candidates_with_request_source": no_candidates_with_request_source,
        "no_candidates_with_tool_source": no_candidates_with_tool_source,
        "no_candidates_matching_selectors": no_candidates_matching_selectors,
        "no_candidates_response_at_most_512": no_candidates_response_at_most_512,
        "no_candidates_response_over_512": no_candidates_response_over_512,
        "no_candidates_static_at_most_512": no_candidates_static_at_most_512,
        "no_candidates_short_dynamic_zero": no_candidates_short_dynamic_zero,
        "no_candidates_short_dynamic_partial": no_candidates_short_dynamic_partial,
        "no_candidates_short_partial_exact_surface": no_candidates_short_partial_exact_surface,
        "short_partial_candidates_enumerated": short_partial_candidates_enumerated,
        "short_partial_policy_reasons": short_partial_policy_reasons,
        "short_partial_canonical_rejection_reasons": short_partial_canonical_rejection_reasons,
        "candidates_enumerated": candidates_enumerated,
        "policy_rejected_exact_matches": policy_rejected_exact_matches,
        "policy_rejection_reasons": policy_rejection_reasons,
        "canonical_rejection_reasons": canonical_rejection_reasons,
        "teacher_match_rows": teacher_match_rows,
        "authority_match_rows": authority_match_rows,
        "exact_match_rows": exact_match_rows,
        "teacher_without_authority_rows": teacher_without_authority_rows,
        "teacher_program_kinds": teacher_kinds,
        "authority_program_kinds": authority_kinds,
    })
}
