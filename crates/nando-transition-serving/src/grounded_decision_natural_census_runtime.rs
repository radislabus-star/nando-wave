use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use nando_operator_learning::{
    DecisionContractPrecommitV1, DurableGoalSatisfactionV1, DurableSelectedActionBindingV1,
    GroundedDecisionShadowCensorV1, S1C4_NATURAL_CENSUS_REPORT_SCHEMA_V1, S1c4ClassificationRowV1,
    S1c4NaturalCensusCursorV1, S1c4NaturalCensusReportV1, S1c4NaturalCensusStateV1,
    S1c4NaturalCensusVerdictV1, S1c4TerminalClassificationV1, s1c4_terminal_verdict_v1,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::grounded_decision_capture::{
    read_precommit_records, read_satisfaction_records, read_selected_records,
};
use crate::opportunity_bridge::{
    OpportunityWindowBoundaryV1, OpportunityWindowClosureV1, S1C4_WINDOW_BOUNDARY_FILE_V1,
};
use crate::{AppState, ServingConfig, sha256_file_streaming, unix_now};

pub const S1C4_CURSOR_FILE_V1: &str = "s1c4-natural-census-cursor-v1.json";
pub const S1C4_REPORT_FILE_V1: &str = "s1c4-natural-census-report-v1.json";
pub const S1C4_OPEN_REQUEST_FILE_V1: &str = "s1c4-open-request-v1.json";
pub const S1C4_OUTPUT_DIRECTORY_V1: &str = "s1c4-natural-census-v1";
const CONTROLLER_POLL: Duration = Duration::from_millis(100);
const MAX_SIDECAR_BYTES: u64 = 64 * 1024;

#[derive(Clone, Copy)]
struct SafetyCountersV1 {
    false_accepts: u64,
    parity_failures: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct S1c4OpenRequestV1 {
    schema: String,
    deployment_receipt_root_sha256: String,
}

struct CensusProjectionV1 {
    denominator_requests: u64,
    denominator_input_tokens: u64,
    classified_requests: u64,
    goal_bound: u64,
    alternative_bearing: u64,
    decision_episodes: u64,
    satisfied_episodes: u64,
    distinct_decision_lineages: u64,
    censor_counts: BTreeMap<GroundedDecisionShadowCensorV1, u64>,
    exact_join_complete: bool,
    source_complete: bool,
    last_denominator_sequence: u64,
    last_denominator_row_position: u64,
    last_classification_root: String,
}

#[derive(Clone, Copy)]
struct FrozenWindowBoundaryV1 {
    state: S1c4NaturalCensusStateV1,
    closes_at_unix: u64,
    quiescence_deadline_unix: u64,
    end_sequence: u64,
    end_ordinal: u64,
    end_input_tokens: u64,
}

pub(crate) fn open_s1c4_natural_census_cursor_v1(
    state: &AppState,
    deployment_receipt_root_sha256: String,
) -> Result<S1c4NaturalCensusCursorV1, String> {
    if !valid_nonzero_sha256(&deployment_receipt_root_sha256) {
        return Err("s1c4_deployment_receipt_root_invalid".to_owned());
    }
    let config = &state.config;
    let paths = census_paths(config)?;
    let cursor_path = paths.output_directory.join(S1C4_CURSOR_FILE_V1);
    if cursor_path.exists() {
        return Err("s1c4_cursor_already_exists".to_owned());
    }
    if paths.report_path.exists() {
        return Err("s1c4_report_exists_without_cursor".to_owned());
    }
    let shadow = state
        .grounded_decision_shadow
        .as_ref()
        .ok_or_else(|| "s1c4_shadow_runtime_missing".to_owned())?;
    let cursor = state
        .opportunity_bridge
        .with_durable_cursor(|opportunity, opportunity_inner| {
            if opportunity.last_sequence == 0 {
                return Err("s1c4_opportunity_checkpoint_empty".to_owned());
            }
            let classifications = shadow
                .classification
                .read_rows(&paths.classification_directory)?;
            let classification_root = classifications.last().map_or_else(
                nando_operator_learning::s1c4_classification_genesis_root_v1,
                |row| row.row_root_sha256.clone(),
            );
            let precommits = read_precommit_records(&paths.journal_directory)?;
            let selected = read_selected_records(&paths.journal_directory)?;
            let satisfaction = read_satisfaction_records(&paths.journal_directory)?;
            validate_decision_journals(&precommits, &selected, &satisfaction)?;
            let writer = shadow.classification.status();
            let safety = read_safety_counters(&paths.economics_path)?;
            let implementation_sha256 = sha256_file_streaming(&config.runtime_build_path)?;
            let opportunity_bridge_root_sha256 = canonical_json_sha256(&(
                "nando.s1c4-opportunity-durable-cursor.v1",
                opportunity.counter_started_after_sequence,
                opportunity.last_sequence,
                opportunity.durable_sequence,
                opportunity.request_events,
                opportunity.request_input_tokens,
            ))
            .map_err(str::to_owned)?;
            let cursor = S1c4NaturalCensusCursorV1::seal(
                implementation_sha256,
                deployment_receipt_root_sha256,
                opportunity_bridge_root_sha256,
                opportunity.counter_started_after_sequence,
                opportunity.last_sequence,
                opportunity.request_events,
                opportunity.request_input_tokens,
                u64::try_from(classifications.len()).map_err(|_| "s1c4_count_overflow")?,
                classification_root,
                u64::try_from(precommits.len()).map_err(|_| "s1c4_count_overflow")?,
                precommit_prefix_root(&precommits)?,
                u64::try_from(selected.len()).map_err(|_| "s1c4_count_overflow")?,
                selected_prefix_root(&selected)?,
                u64::try_from(satisfaction.len()).map_err(|_| "s1c4_count_overflow")?,
                satisfaction_prefix_root(&satisfaction)?,
                writer.queue_overflow,
                writer.writer_failures,
                writer.disconnected,
                writer.duplicate_rows,
                safety.false_accepts,
                safety.parity_failures,
                unix_now(),
            )
            .map_err(str::to_owned)?;
            classification_window_bridge_capture(&shadow.classification, &cursor)?;
            if let Err(error) = crate::opportunity_bridge::OpportunityBridgeRuntime::configure_request_deadline_capture_locked(
                opportunity_inner,
                cursor.cursor_root_sha256.clone(),
                cursor.deadline_at_unix,
                cursor
                    .opportunity_start_request_ordinal
                    .checked_add(cursor.maximum_request_events)
                    .ok_or_else(|| "s1c4_classification_window_overflow".to_owned())?,
                paths.deadline_boundary_path.clone(),
            ) {
                shadow.classification.disable_window();
                return Err(error);
            }
            if let Err(error) = write_new_json(&cursor_path, &cursor) {
                shadow.classification.disable_window();
                crate::opportunity_bridge::OpportunityBridgeRuntime::disable_request_deadline_capture_locked(
                    opportunity_inner,
                );
                return Err(error);
            }
            Ok(cursor)
        })?;
    Ok(cursor)
}

pub(crate) fn spawn_s1c4_natural_census_runtime(state: AppState) -> Result<(), String> {
    let Some(shadow) = state.grounded_decision_shadow.as_ref() else {
        return Ok(());
    };
    let paths = census_paths(&state.config)?;
    let cursor = read_json_bounded::<S1c4NaturalCensusCursorV1>(&paths.cursor_path)?;
    let terminal =
        read_json_bounded::<S1c4NaturalCensusReportV1>(&paths.report_path)?.is_some_and(|report| {
            report.validate().is_ok() && report.state == S1c4NaturalCensusStateV1::Terminal
        });
    if let Some(cursor) = cursor.filter(|cursor| cursor.validate().is_ok() && !terminal) {
        configure_classification_window(&state, &cursor, &paths)?;
    } else {
        shadow.classification.disable_window();
    }
    let classification = shadow.classification.clone();
    thread::Builder::new()
        .name("nando-s1c4-natural-census".to_owned())
        .spawn(move || {
            loop {
                if !paths.cursor_path.exists()
                    && let Err(error) = consume_open_request(&state, &paths)
                {
                    eprintln!("nando-s1c4-open: {error}");
                }
                if let Err(error) = advance_census(&state, &paths) {
                    eprintln!("nando-s1c4-census: {error}");
                    if let Err(publish_error) = publish_integrity_veto(&state, &paths, &error) {
                        eprintln!("nando-s1c4-veto: {publish_error}");
                    }
                }
                let terminal = read_json_bounded::<S1c4NaturalCensusReportV1>(&paths.report_path)
                    .ok()
                    .flatten()
                    .is_some_and(|report| {
                        report.validate().is_ok()
                            && report.state == S1c4NaturalCensusStateV1::Terminal
                    });
                if terminal {
                    classification.disable_window();
                    state.opportunity_bridge.disable_request_deadline_capture();
                }
                thread::sleep(CONTROLLER_POLL);
            }
        })
        .map(|_| ())
        .map_err(|error| format!("s1c4_controller_spawn:{error}"))
}

fn configure_classification_window(
    state: &AppState,
    cursor: &S1c4NaturalCensusCursorV1,
    paths: &CensusPathsV1,
) -> Result<(), String> {
    let classification = &state
        .grounded_decision_shadow
        .as_ref()
        .ok_or_else(|| "s1c4_shadow_runtime_missing".to_owned())?
        .classification;
    classification_window_bridge_capture(classification, cursor)?;
    state
        .opportunity_bridge
        .configure_request_deadline_capture(
            cursor.cursor_root_sha256.clone(),
            cursor.deadline_at_unix,
            cursor
                .opportunity_start_request_ordinal
                .checked_add(cursor.maximum_request_events)
                .ok_or_else(|| "s1c4_classification_window_overflow".to_owned())?,
            paths.deadline_boundary_path.clone(),
        )?;
    Ok(())
}

fn classification_window_bridge_capture(
    classification: &crate::grounded_decision_natural_census::S1c4ClassificationRuntimeV1,
    cursor: &S1c4NaturalCensusCursorV1,
) -> Result<(), String> {
    classification.configure_window(
        cursor.opportunity_start_request_ordinal,
        cursor
            .opportunity_start_request_ordinal
            .checked_add(cursor.maximum_request_events)
            .ok_or_else(|| "s1c4_classification_window_overflow".to_owned())?,
        cursor.deadline_at_unix,
    )
}

fn consume_open_request(state: &AppState, paths: &CensusPathsV1) -> Result<(), String> {
    let Some(request) = read_json_bounded::<S1c4OpenRequestV1>(&paths.open_request_path)? else {
        return Ok(());
    };
    if request.schema != "nando.s1c4-open-request.v1"
        || !valid_nonzero_sha256(&request.deployment_receipt_root_sha256)
    {
        return Err("s1c4_open_request_invalid".to_owned());
    }
    let shadow = state
        .grounded_decision_shadow
        .as_ref()
        .ok_or_else(|| "s1c4_shadow_runtime_missing".to_owned())?;
    let _journal_guard = shadow
        .journal
        .lock()
        .map_err(|_| "s1c4_journal_lock_poisoned".to_owned())?;
    let cursor = open_s1c4_natural_census_cursor_v1(state, request.deployment_receipt_root_sha256)?;
    cursor.validate().map_err(str::to_owned)?;
    fs::remove_file(&paths.open_request_path)
        .map_err(|error| format!("s1c4_open_request_remove:{error}"))?;
    File::open(&paths.output_directory)
        .and_then(|file| file.sync_all())
        .map_err(|error| format!("s1c4_open_request_parent_sync:{error}"))
}

fn advance_census(state: &AppState, paths: &CensusPathsV1) -> Result<(), String> {
    let Some(cursor) = read_json_bounded::<S1c4NaturalCensusCursorV1>(&paths.cursor_path)? else {
        return Ok(());
    };
    cursor.validate().map_err(str::to_owned)?;
    if let Some(report) = read_json_bounded::<S1c4NaturalCensusReportV1>(&paths.report_path)? {
        report.validate().map_err(str::to_owned)?;
        if report.cursor_root_sha256 != cursor.cursor_root_sha256 {
            return Err("s1c4_report_cursor_mismatch".to_owned());
        }
        if report.state == S1c4NaturalCensusStateV1::Terminal {
            return Ok(());
        }
    }

    let mut opportunity = state.opportunity_bridge.status();
    let writer = state
        .grounded_decision_shadow
        .as_ref()
        .ok_or_else(|| "s1c4_shadow_runtime_missing".to_owned())?
        .classification
        .status();
    let shadow = state
        .grounded_decision_shadow
        .as_ref()
        .ok_or_else(|| "s1c4_shadow_runtime_missing".to_owned())?;
    let rows = shadow
        .classification
        .read_rows(&paths.classification_directory)?;
    let previous_report = read_json_bounded::<S1c4NaturalCensusReportV1>(&paths.report_path)?;
    let now = unix_now();

    if opportunity.producer.counter_started_after_sequence
        != cursor.opportunity_counter_started_after_sequence
        || opportunity.producer.last_sequence < cursor.opportunity_start_sequence
        || opportunity.producer.request_events < cursor.opportunity_start_request_ordinal
        || opportunity.producer.request_input_tokens < cursor.opportunity_start_input_tokens
    {
        return Err("s1c4_opportunity_counter_epoch_changed".to_owned());
    }

    let deadline_boundary = state
        .opportunity_bridge
        .freeze_request_deadline_boundary(now)?;
    opportunity = state.opportunity_bridge.status();
    let frozen = frozen_window_boundary(
        &cursor,
        previous_report.as_ref(),
        deadline_boundary.as_ref(),
        &opportunity,
        now,
    )?;
    let state_value = frozen.state;
    let closes_at = frozen.closes_at_unix;
    let quiescence_deadline = frozen.quiescence_deadline_unix;
    let end_ordinal = frozen.end_ordinal;

    let _journal_guard = shadow
        .journal
        .lock()
        .map_err(|_| "s1c4_journal_lock_poisoned".to_owned())?;
    let projection = project_census(paths, &cursor, &rows, end_ordinal)?;
    let (projected_end_sequence, projected_end_tokens) =
        exact_window_boundary(&cursor, &projection)?;
    let (end_sequence, end_tokens) = if state_value == S1c4NaturalCensusStateV1::Quiescing {
        if projection.source_complete
            && (projected_end_sequence != frozen.end_sequence
                || projected_end_tokens != frozen.end_input_tokens)
        {
            return Err("s1c4_frozen_boundary_projection_mismatch".to_owned());
        }
        (frozen.end_sequence, frozen.end_input_tokens)
    } else {
        (projected_end_sequence, projected_end_tokens)
    };
    let safety = read_safety_counters(&paths.economics_path)?;
    let queue_overflow = writer
        .queue_overflow
        .saturating_sub(cursor.queue_overflow_start);
    let writer_failures = writer
        .writer_failures
        .saturating_sub(cursor.writer_failures_start)
        .saturating_add(
            writer
                .disconnected
                .saturating_sub(cursor.disconnected_start),
        );
    let duplicate_rows = writer
        .duplicate_rows
        .saturating_sub(cursor.duplicate_rows_start);
    let false_accepts = safety
        .false_accepts
        .saturating_sub(cursor.false_accepts_start);
    let parity_failures = safety
        .parity_failures
        .saturating_sub(cursor.parity_failures_start);
    let evidence_veto = queue_overflow != 0
        || writer_failures != 0
        || duplicate_rows != 0
        || false_accepts != 0
        || parity_failures != 0
        || !projection.exact_join_complete;
    let source_durable = projection.denominator_requests == 0
        || opportunity.producer.durable_sequence >= projection.last_denominator_sequence;
    let complete = state_value == S1c4NaturalCensusStateV1::Quiescing
        && projection.classified_requests == projection.denominator_requests
        && projection.source_complete
        && source_durable
        && writer.durable_rows >= projection.last_denominator_row_position;
    let terminal = state_value == S1c4NaturalCensusStateV1::Quiescing
        && (complete || now >= quiescence_deadline);
    let (report_state, verdict, blocker) = if terminal {
        let (verdict, blocker) = s1c4_terminal_verdict_v1(
            projection.denominator_requests,
            projection.classified_requests,
            projection.goal_bound,
            projection.alternative_bearing,
            projection.satisfied_episodes,
            projection.distinct_decision_lineages,
            projection
                .censor_counts
                .get(&GroundedDecisionShadowCensorV1::MissingExactGoal)
                .copied()
                .unwrap_or(0),
            evidence_veto || !complete,
        );
        (S1c4NaturalCensusStateV1::Terminal, verdict, blocker)
    } else if state_value == S1c4NaturalCensusStateV1::Quiescing {
        (
            S1c4NaturalCensusStateV1::Quiescing,
            S1c4NaturalCensusVerdictV1::Collecting,
            "awaiting_durable_classification_quiescence",
        )
    } else {
        (
            S1c4NaturalCensusStateV1::Collecting,
            S1c4NaturalCensusVerdictV1::Collecting,
            "finite_natural_window_open",
        )
    };

    let report = S1c4NaturalCensusReportV1::seal(S1c4NaturalCensusReportV1 {
        schema: S1C4_NATURAL_CENSUS_REPORT_SCHEMA_V1.to_owned(),
        report_root_sha256: String::new(),
        cursor_root_sha256: cursor.cursor_root_sha256.clone(),
        state: report_state,
        verdict,
        blocker: blocker.to_owned(),
        generated_at_unix: now,
        closes_at_unix: closes_at,
        quiescence_deadline_unix: quiescence_deadline,
        opportunity_end_sequence: end_sequence,
        opportunity_end_request_ordinal: end_ordinal,
        opportunity_end_input_tokens: end_tokens,
        denominator_requests: projection.denominator_requests,
        denominator_input_tokens: projection.denominator_input_tokens,
        classified_requests: projection.classified_requests,
        goal_bound: projection.goal_bound,
        alternative_bearing: projection.alternative_bearing,
        decision_episodes: projection.decision_episodes,
        satisfied_episodes: projection.satisfied_episodes,
        distinct_decision_lineages: projection.distinct_decision_lineages,
        censor_counts: projection.censor_counts,
        classification_rows_total: u64::try_from(rows.len()).unwrap_or(u64::MAX),
        classification_last_root_sha256: projection.last_classification_root,
        queue_overflow,
        writer_failures,
        duplicate_rows,
        false_accepts,
        parity_failures,
        source_complete: projection.source_complete && source_durable,
        exact_join_complete: projection.exact_join_complete,
        raw_payloads_persisted: false,
        k2_open: false,
        s2_started: false,
        model_training_allowed: false,
        package_activation_allowed: false,
        authority_ready: false,
        phase_mutation_allowed: false,
    })
    .map_err(str::to_owned)?;
    write_atomic_json(&paths.report_path, &report)
}

fn frozen_window_boundary(
    cursor: &S1c4NaturalCensusCursorV1,
    previous_report: Option<&S1c4NaturalCensusReportV1>,
    deadline_boundary: Option<&OpportunityWindowBoundaryV1>,
    opportunity: &crate::opportunity_bridge::OpportunityBridgeStatusV1,
    now: u64,
) -> Result<FrozenWindowBoundaryV1, String> {
    if let Some(report) =
        previous_report.filter(|report| report.state == S1c4NaturalCensusStateV1::Quiescing)
    {
        return Ok(FrozenWindowBoundaryV1 {
            state: S1c4NaturalCensusStateV1::Quiescing,
            closes_at_unix: report.closes_at_unix,
            quiescence_deadline_unix: report.quiescence_deadline_unix,
            end_sequence: report.opportunity_end_sequence,
            end_ordinal: report.opportunity_end_request_ordinal,
            end_input_tokens: report.opportunity_end_input_tokens,
        });
    }
    if let Some(boundary) = deadline_boundary {
        boundary.validate()?;
        let maximum_ordinal = cursor
            .opportunity_start_request_ordinal
            .checked_add(cursor.maximum_request_events)
            .ok_or_else(|| "s1c4_window_boundary_ordinal_overflow".to_owned())?;
        let closure_matches = match boundary.closure {
            OpportunityWindowClosureV1::RequestLimit => {
                boundary.opportunity_end_request_ordinal == maximum_ordinal
                    && boundary.closes_at_unix <= cursor.deadline_at_unix
            }
            OpportunityWindowClosureV1::TimeLimit => {
                boundary.opportunity_end_request_ordinal < maximum_ordinal
                    && boundary.closes_at_unix == cursor.deadline_at_unix
            }
        };
        if boundary.cursor_root_sha256 != cursor.cursor_root_sha256
            || boundary.opportunity_end_sequence < cursor.opportunity_start_sequence
            || boundary.opportunity_end_request_ordinal < cursor.opportunity_start_request_ordinal
            || boundary.opportunity_end_input_tokens < cursor.opportunity_start_input_tokens
            || !closure_matches
        {
            return Err("s1c4_window_boundary_cursor_mismatch".to_owned());
        }
        return Ok(FrozenWindowBoundaryV1 {
            state: S1c4NaturalCensusStateV1::Quiescing,
            closes_at_unix: boundary.closes_at_unix,
            quiescence_deadline_unix: now.saturating_add(cursor.quiescence_seconds),
            end_sequence: boundary.opportunity_end_sequence,
            end_ordinal: boundary.opportunity_end_request_ordinal,
            end_input_tokens: boundary.opportunity_end_input_tokens,
        });
    }
    if now > cursor.deadline_at_unix {
        return Err("s1c4_window_boundary_missing_after_deadline".to_owned());
    }
    Ok(FrozenWindowBoundaryV1 {
        state: S1c4NaturalCensusStateV1::Collecting,
        closes_at_unix: 0,
        quiescence_deadline_unix: 0,
        end_sequence: 0,
        end_ordinal: opportunity.producer.request_events,
        end_input_tokens: 0,
    })
}

fn publish_integrity_veto(
    state: &AppState,
    paths: &CensusPathsV1,
    blocker: &str,
) -> Result<(), String> {
    let Some(cursor) = read_json_bounded::<S1c4NaturalCensusCursorV1>(&paths.cursor_path)? else {
        return Ok(());
    };
    cursor.validate().map_err(str::to_owned)?;
    if read_json_bounded::<S1c4NaturalCensusReportV1>(&paths.report_path)?.is_some_and(|report| {
        report.validate().is_ok() && report.state == S1c4NaturalCensusStateV1::Terminal
    }) {
        return Ok(());
    }
    let opportunity = state.opportunity_bridge.status();
    let writer = state
        .grounded_decision_shadow
        .as_ref()
        .ok_or_else(|| "s1c4_shadow_runtime_missing".to_owned())?
        .classification
        .status();
    let safety = read_safety_counters(&paths.economics_path).unwrap_or(SafetyCountersV1 {
        false_accepts: cursor.false_accepts_start,
        parity_failures: cursor.parity_failures_start,
    });
    let classification_last_root_sha256 = if writer.durable_rows == writer.appended_rows
        && valid_nonzero_sha256(&writer.last_row_root_sha256)
    {
        writer.last_row_root_sha256
    } else {
        cursor.classification_start_root_sha256.clone()
    };
    let now = unix_now();
    let report = S1c4NaturalCensusReportV1::seal(S1c4NaturalCensusReportV1 {
        schema: S1C4_NATURAL_CENSUS_REPORT_SCHEMA_V1.to_owned(),
        report_root_sha256: String::new(),
        cursor_root_sha256: cursor.cursor_root_sha256.clone(),
        state: S1c4NaturalCensusStateV1::Terminal,
        verdict: S1c4NaturalCensusVerdictV1::Veto,
        blocker: blocker.to_owned(),
        generated_at_unix: now,
        closes_at_unix: now,
        quiescence_deadline_unix: now,
        opportunity_end_sequence: opportunity.producer.durable_sequence,
        opportunity_end_request_ordinal: opportunity.producer.request_events,
        opportunity_end_input_tokens: opportunity.producer.request_input_tokens,
        denominator_requests: opportunity
            .producer
            .request_events
            .saturating_sub(cursor.opportunity_start_request_ordinal),
        denominator_input_tokens: opportunity
            .producer
            .request_input_tokens
            .saturating_sub(cursor.opportunity_start_input_tokens),
        classified_requests: 0,
        goal_bound: 0,
        alternative_bearing: 0,
        decision_episodes: 0,
        satisfied_episodes: 0,
        distinct_decision_lineages: 0,
        censor_counts: BTreeMap::new(),
        classification_rows_total: writer.durable_rows,
        classification_last_root_sha256,
        queue_overflow: writer
            .queue_overflow
            .saturating_sub(cursor.queue_overflow_start),
        writer_failures: writer
            .writer_failures
            .saturating_sub(cursor.writer_failures_start)
            .saturating_add(
                writer
                    .disconnected
                    .saturating_sub(cursor.disconnected_start),
            ),
        duplicate_rows: writer
            .duplicate_rows
            .saturating_sub(cursor.duplicate_rows_start),
        false_accepts: safety
            .false_accepts
            .saturating_sub(cursor.false_accepts_start),
        parity_failures: safety
            .parity_failures
            .saturating_sub(cursor.parity_failures_start),
        source_complete: false,
        exact_join_complete: false,
        raw_payloads_persisted: false,
        k2_open: false,
        s2_started: false,
        model_training_allowed: false,
        package_activation_allowed: false,
        authority_ready: false,
        phase_mutation_allowed: false,
    })
    .map_err(str::to_owned)?;
    write_atomic_json(&paths.report_path, &report)
}

fn project_census(
    paths: &CensusPathsV1,
    cursor: &S1c4NaturalCensusCursorV1,
    rows: &[S1c4ClassificationRowV1],
    end_ordinal: u64,
) -> Result<CensusProjectionV1, String> {
    let start = usize::try_from(cursor.classification_start_rows)
        .map_err(|_| "s1c4_classification_cursor_overflow".to_owned())?;
    if start > rows.len()
        || (start > 0 && rows[start - 1].row_root_sha256 != cursor.classification_start_root_sha256)
    {
        return Err("s1c4_classification_prefix_changed".to_owned());
    }
    let denominator_requests = end_ordinal.saturating_sub(cursor.opportunity_start_request_ordinal);
    let mut suffix = rows
        .iter()
        .enumerate()
        .skip(start)
        .filter(|(_, row)| {
            row.opportunity_request_ordinal > cursor.opportunity_start_request_ordinal
                && row.opportunity_request_ordinal <= end_ordinal
        })
        .collect::<Vec<_>>();
    suffix.sort_by_key(|(_, row)| row.opportunity_request_ordinal);
    let mut source_complete = true;
    let mut denominator_input_tokens = 0_u64;
    let mut last_sequence = 0_u64;
    let mut last_denominator_row_position = 0_u64;
    let mut seen_sequences = BTreeSet::new();
    let mut censor_counts = BTreeMap::new();
    let mut decision_rows = Vec::new();
    for (index, (row_position, row)) in suffix.iter().enumerate() {
        let expected_ordinal = cursor
            .opportunity_start_request_ordinal
            .saturating_add(u64::try_from(index).unwrap_or(u64::MAX))
            .saturating_add(1);
        if row.opportunity_request_ordinal != expected_ordinal
            || !seen_sequences.insert(row.opportunity_sequence)
            || (last_sequence != 0 && row.opportunity_sequence <= last_sequence)
            || row.validate().is_err()
        {
            source_complete = false;
        }
        last_sequence = row.opportunity_sequence;
        last_denominator_row_position = last_denominator_row_position.max(
            u64::try_from(*row_position)
                .unwrap_or(u64::MAX)
                .saturating_add(1),
        );
        denominator_input_tokens =
            denominator_input_tokens.saturating_add(row.request_input_tokens);
        match &row.classification {
            S1c4TerminalClassificationV1::Censored { reason } => {
                *censor_counts.entry(*reason).or_insert(0_u64) = censor_counts
                    .get(reason)
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(1);
            }
            S1c4TerminalClassificationV1::DecisionRecorded {
                decision_precommit_root_sha256,
            } => decision_rows.push((*row, decision_precommit_root_sha256.as_str())),
        }
    }
    if u64::try_from(suffix.len()).unwrap_or(u64::MAX) != denominator_requests {
        source_complete = false;
    }

    let precommits = read_precommit_records(&paths.journal_directory)?;
    let selected = read_selected_records(&paths.journal_directory)?;
    let satisfactions = read_satisfaction_records(&paths.journal_directory)?;
    validate_decision_journals(&precommits, &selected, &satisfactions)?;
    let precommit_start = usize::try_from(cursor.precommit_start_rows)
        .map_err(|_| "s1c4_precommit_cursor_overflow".to_owned())?;
    let selected_start = usize::try_from(cursor.selected_action_start_rows)
        .map_err(|_| "s1c4_selected_cursor_overflow".to_owned())?;
    let satisfaction_start = usize::try_from(cursor.satisfaction_start_rows)
        .map_err(|_| "s1c4_satisfaction_cursor_overflow".to_owned())?;
    if precommit_start > precommits.len()
        || selected_start > selected.len()
        || satisfaction_start > satisfactions.len()
    {
        return Err("s1c4_journal_prefix_changed".to_owned());
    }
    if precommit_prefix_root(&precommits[..precommit_start])? != cursor.precommit_prefix_root_sha256
        || selected_prefix_root(&selected[..selected_start])?
            != cursor.selected_action_prefix_root_sha256
        || satisfaction_prefix_root(&satisfactions[..satisfaction_start])?
            != cursor.satisfaction_prefix_root_sha256
    {
        return Err("s1c4_journal_prefix_changed".to_owned());
    }
    let relevant_precommits = precommits[precommit_start..]
        .iter()
        .filter(|row| {
            suffix.iter().any(|(_, classification)| {
                classification.request_event_identity_root_sha256
                    == row.request_event_identity_root_sha256
            })
        })
        .collect::<Vec<_>>();
    let precommit_by_root = relevant_precommits
        .iter()
        .map(|row| (row.precommit_root_sha256.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let precommit_by_request = relevant_precommits
        .iter()
        .map(|row| (row.request_event_identity_root_sha256.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let selected_by_root = selected[selected_start..]
        .iter()
        .filter(|row| precommit_by_root.contains_key(row.receipt.precommit_root_sha256.as_str()))
        .map(|row| (row.receipt.precommit_root_sha256.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let satisfaction_by_root = satisfactions[satisfaction_start..]
        .iter()
        .filter(|row| precommit_by_root.contains_key(row.precommit_root_sha256.as_str()))
        .map(|row| (row.precommit_root_sha256.as_str(), row))
        .collect::<BTreeMap<_, _>>();

    let goal_bound = suffix
        .iter()
        .filter(|(_, row)| {
            !matches!(
                row.classification,
                S1c4TerminalClassificationV1::Censored {
                    reason: GroundedDecisionShadowCensorV1::MissingExactGoal
                        | GroundedDecisionShadowCensorV1::GoalInputInvalid
                        | GroundedDecisionShadowCensorV1::CaptureDisabled
                        | GroundedDecisionShadowCensorV1::IneligibleTrafficProvenance
                }
            )
        })
        .count();
    let alternative_bearing = suffix
        .iter()
        .filter(|(_, row)| {
            precommit_by_request
                .get(row.request_event_identity_root_sha256.as_str())
                .is_some_and(|precommit| precommit.available_action_count > 0)
        })
        .count();

    let mut exact_join_complete = precommit_by_request.len() == precommit_by_root.len();
    let mut satisfied_episodes = 0_u64;
    let mut lineages = BTreeSet::new();
    for (classification, precommit_root) in &decision_rows {
        let Some(precommit) = precommit_by_root.get(precommit_root) else {
            exact_join_complete = false;
            continue;
        };
        let Some(selected) = selected_by_root.get(precommit_root) else {
            exact_join_complete = false;
            continue;
        };
        let Some(satisfaction) = satisfaction_by_root.get(precommit_root) else {
            exact_join_complete = false;
            continue;
        };
        if precommit.request_event_identity_root_sha256
            != classification.request_event_identity_root_sha256
            || precommit.available_action_count == 0
            || selected.validate_join(precommit).is_err()
            || satisfaction.validate_join(precommit, selected).is_err()
        {
            exact_join_complete = false;
            continue;
        }
        if satisfaction.receipt.satisfied {
            satisfied_episodes = satisfied_episodes.saturating_add(1);
            lineages.insert(classification.session_lineage_root_sha256.as_str());
        }
    }
    for (root, selected) in &selected_by_root {
        if !precommit_by_root
            .get(root)
            .is_some_and(|precommit| selected.validate_join(precommit).is_ok())
        {
            exact_join_complete = false;
        }
    }
    for (root, satisfaction) in &satisfaction_by_root {
        if !precommit_by_root.get(root).is_some_and(|precommit| {
            selected_by_root
                .get(root)
                .is_some_and(|selected| satisfaction.validate_join(precommit, selected).is_ok())
        }) {
            exact_join_complete = false;
        }
    }

    Ok(CensusProjectionV1 {
        denominator_requests,
        denominator_input_tokens,
        classified_requests: u64::try_from(suffix.len()).unwrap_or(u64::MAX),
        goal_bound: u64::try_from(goal_bound).unwrap_or(u64::MAX),
        alternative_bearing: u64::try_from(alternative_bearing).unwrap_or(u64::MAX),
        decision_episodes: u64::try_from(decision_rows.len()).unwrap_or(u64::MAX),
        satisfied_episodes,
        distinct_decision_lineages: u64::try_from(lineages.len()).unwrap_or(u64::MAX),
        censor_counts,
        exact_join_complete,
        source_complete,
        last_denominator_sequence: last_sequence,
        last_denominator_row_position,
        last_classification_root: rows.last().map_or_else(
            nando_operator_learning::s1c4_classification_genesis_root_v1,
            |row| row.row_root_sha256.clone(),
        ),
    })
}

fn exact_window_boundary(
    cursor: &S1c4NaturalCensusCursorV1,
    projection: &CensusProjectionV1,
) -> Result<(u64, u64), String> {
    if !projection.source_complete
        || projection.classified_requests != projection.denominator_requests
    {
        return Ok((0, 0));
    }
    let end_tokens = cursor
        .opportunity_start_input_tokens
        .checked_add(projection.denominator_input_tokens)
        .ok_or_else(|| "s1c4_opportunity_token_boundary_overflow".to_owned())?;
    Ok((projection.last_denominator_sequence, end_tokens))
}

fn validate_decision_journals(
    precommits: &[DecisionContractPrecommitV1],
    selected: &[DurableSelectedActionBindingV1],
    satisfactions: &[DurableGoalSatisfactionV1],
) -> Result<(), String> {
    let mut precommit_by_root = BTreeMap::new();
    let mut precommit_request_roots = BTreeSet::new();
    for precommit in precommits {
        precommit.validate().map_err(str::to_owned)?;
        if !precommit_request_roots.insert(precommit.request_event_identity_root_sha256.as_str())
            || precommit_by_root
                .insert(precommit.precommit_root_sha256.as_str(), precommit)
                .is_some()
        {
            return Err("s1c4_precommit_duplicate".to_owned());
        }
    }

    let mut selected_by_precommit = BTreeMap::new();
    let mut selected_record_roots = BTreeSet::new();
    for record in selected {
        record.validate().map_err(str::to_owned)?;
        let precommit = precommit_by_root
            .get(record.receipt.precommit_root_sha256.as_str())
            .ok_or_else(|| "s1c4_selected_orphan".to_owned())?;
        record.validate_join(precommit).map_err(str::to_owned)?;
        if !selected_record_roots.insert(record.record_root_sha256.as_str())
            || selected_by_precommit
                .insert(record.receipt.precommit_root_sha256.as_str(), record)
                .is_some()
        {
            return Err("s1c4_selected_duplicate".to_owned());
        }
    }

    let mut satisfaction_precommits = BTreeSet::new();
    let mut satisfaction_record_roots = BTreeSet::new();
    for record in satisfactions {
        record.validate().map_err(str::to_owned)?;
        let precommit = precommit_by_root
            .get(record.precommit_root_sha256.as_str())
            .ok_or_else(|| "s1c4_satisfaction_orphan".to_owned())?;
        let selected = selected_by_precommit
            .get(record.precommit_root_sha256.as_str())
            .ok_or_else(|| "s1c4_satisfaction_selected_missing".to_owned())?;
        record
            .validate_join(precommit, selected)
            .map_err(str::to_owned)?;
        if !satisfaction_record_roots.insert(record.record_root_sha256.as_str())
            || !satisfaction_precommits.insert(record.precommit_root_sha256.as_str())
        {
            return Err("s1c4_satisfaction_duplicate".to_owned());
        }
    }
    Ok(())
}

fn precommit_prefix_root(rows: &[DecisionContractPrecommitV1]) -> Result<String, String> {
    journal_prefix_root(
        "nando.s1c4-precommit-prefix.v1",
        rows.iter().map(|row| row.precommit_root_sha256.as_str()),
    )
}

fn selected_prefix_root(rows: &[DurableSelectedActionBindingV1]) -> Result<String, String> {
    journal_prefix_root(
        "nando.s1c4-selected-prefix.v1",
        rows.iter().map(|row| row.record_root_sha256.as_str()),
    )
}

fn satisfaction_prefix_root(rows: &[DurableGoalSatisfactionV1]) -> Result<String, String> {
    journal_prefix_root(
        "nando.s1c4-satisfaction-prefix.v1",
        rows.iter().map(|row| row.record_root_sha256.as_str()),
    )
}

fn journal_prefix_root<'a>(
    schema: &str,
    roots: impl IntoIterator<Item = &'a str>,
) -> Result<String, String> {
    canonical_json_sha256(&(schema, roots.into_iter().collect::<Vec<_>>())).map_err(str::to_owned)
}

struct CensusPathsV1 {
    output_directory: PathBuf,
    cursor_path: PathBuf,
    open_request_path: PathBuf,
    report_path: PathBuf,
    classification_directory: PathBuf,
    deadline_boundary_path: PathBuf,
    journal_directory: PathBuf,
    economics_path: PathBuf,
}

fn census_paths(config: &ServingConfig) -> Result<CensusPathsV1, String> {
    let output_directory = s1c4_output_directory(&config.grounded_decision_journal_path);
    Ok(CensusPathsV1 {
        cursor_path: output_directory.join(S1C4_CURSOR_FILE_V1),
        open_request_path: output_directory.join(S1C4_OPEN_REQUEST_FILE_V1),
        report_path: output_directory.join(S1C4_REPORT_FILE_V1),
        classification_directory: output_directory.join("s1c4-classifications-v1"),
        deadline_boundary_path: output_directory.join(S1C4_WINDOW_BOUNDARY_FILE_V1),
        journal_directory: config.grounded_decision_journal_path.clone(),
        economics_path: config.ms4_ordinary_economics_path.clone(),
        output_directory,
    })
}

pub(crate) fn s1c4_output_directory(journal_directory: &Path) -> PathBuf {
    journal_directory.join(S1C4_OUTPUT_DIRECTORY_V1)
}

fn read_safety_counters(path: &Path) -> Result<SafetyCountersV1, String> {
    let value = read_json_bounded::<Value>(path)?
        .ok_or_else(|| "s1c4_economics_snapshot_missing".to_owned())?;
    Ok(SafetyCountersV1 {
        false_accepts: value
            .get("false_accepts")
            .and_then(Value::as_u64)
            .ok_or_else(|| "s1c4_false_accepts_missing".to_owned())?,
        parity_failures: value
            .get("runtime_parity_mismatches")
            .and_then(Value::as_u64)
            .ok_or_else(|| "s1c4_parity_failures_missing".to_owned())?,
    })
}

fn read_json_bounded<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Option<T>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("s1c4_sidecar_read:{error}")),
    };
    if bytes.is_empty() || u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_SIDECAR_BYTES {
        return Err("s1c4_sidecar_size_invalid".to_owned());
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| format!("s1c4_sidecar_decode:{error}"))
}

fn write_new_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "s1c4_sidecar_parent_missing".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| format!("s1c4_sidecar_mkdir:{error}"))?;
    let bytes =
        serde_json::to_vec_pretty(value).map_err(|error| format!("s1c4_sidecar_encode:{error}"))?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| format!("s1c4_cursor_create:{error}"))?;
    file.write_all(&bytes)
        .and_then(|()| file.write_all(b"\n"))
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("s1c4_cursor_write:{error}"))?;
    File::open(parent)
        .and_then(|file| file.sync_all())
        .map_err(|error| format!("s1c4_cursor_parent_sync:{error}"))
}

fn write_atomic_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "s1c4_sidecar_parent_missing".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| format!("s1c4_sidecar_mkdir:{error}"))?;
    let temporary = parent.join(format!(".{S1C4_REPORT_FILE_V1}.{}.tmp", std::process::id()));
    let bytes =
        serde_json::to_vec_pretty(value).map_err(|error| format!("s1c4_sidecar_encode:{error}"))?;
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|error| format!("s1c4_report_create:{error}"))?;
        file.write_all(&bytes)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("s1c4_report_write:{error}"))?;
        fs::rename(&temporary, path).map_err(|error| format!("s1c4_report_publish:{error}"))?;
        File::open(parent)
            .and_then(|file| file.sync_all())
            .map_err(|error| format!("s1c4_report_parent_sync:{error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn output_directory_stays_inside_the_service_owned_journal() {
        let journal = Path::new("/state/grounded-meaning-v1/decision-contract-precommits-v1");
        let output = s1c4_output_directory(journal);
        assert_eq!(output, journal.join(S1C4_OUTPUT_DIRECTORY_V1));
        assert!(output.starts_with(journal));
        assert_ne!(output.parent(), journal.parent());
    }

    fn root(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    fn cursor() -> S1c4NaturalCensusCursorV1 {
        S1c4NaturalCensusCursorV1::seal(
            root('1'),
            root('2'),
            root('3'),
            10,
            20,
            12,
            300,
            0,
            nando_operator_learning::s1c4_classification_genesis_root_v1(),
            0,
            precommit_prefix_root(&[]).expect("precommit root"),
            0,
            selected_prefix_root(&[]).expect("selected root"),
            0,
            satisfaction_prefix_root(&[]).expect("satisfaction root"),
            0,
            0,
            0,
            0,
            0,
            0,
            1_700_000_000,
        )
        .expect("cursor")
    }

    fn projection(source_complete: bool) -> CensusProjectionV1 {
        CensusProjectionV1 {
            denominator_requests: 1024,
            denominator_input_tokens: 4096,
            classified_requests: 1024,
            goal_bound: 0,
            alternative_bearing: 0,
            decision_episodes: 0,
            satisfied_episodes: 0,
            distinct_decision_lineages: 0,
            censor_counts: BTreeMap::new(),
            exact_join_complete: true,
            source_complete,
            last_denominator_sequence: 2_048,
            last_denominator_row_position: 1_024,
            last_classification_root: root('4'),
        }
    }

    fn census_paths_for_test(label: &str) -> (PathBuf, CensusPathsV1) {
        let root = std::env::temp_dir().join(format!(
            "nando-s1c4-{label}-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let journal_directory = root.join("journals");
        std::fs::create_dir_all(&journal_directory).expect("journal directory");
        (
            root.clone(),
            CensusPathsV1 {
                output_directory: root.clone(),
                cursor_path: root.join(S1C4_CURSOR_FILE_V1),
                open_request_path: root.join(S1C4_OPEN_REQUEST_FILE_V1),
                report_path: root.join(S1C4_REPORT_FILE_V1),
                classification_directory: root.join("classifications"),
                deadline_boundary_path: root.join(S1C4_WINDOW_BOUNDARY_FILE_V1),
                journal_directory,
                economics_path: root.join("economics.json"),
            },
        )
    }

    fn censored_row(
        previous_root: String,
        sequence: u64,
        ordinal: u64,
        request_byte: char,
    ) -> S1c4ClassificationRowV1 {
        let request_root = root(request_byte);
        let event_root = nando_operator_learning::OpportunityBridgeEventV1::request(
            request_root.clone(),
            ordinal,
            1_700_000_000,
        )
        .canonical_sha256()
        .expect("event root");
        S1c4ClassificationRowV1::seal(
            previous_root,
            sequence,
            ordinal,
            event_root,
            ordinal,
            1_700_000_000,
            request_root,
            root('f'),
            ordinal,
            S1c4TerminalClassificationV1::Censored {
                reason: GroundedDecisionShadowCensorV1::MissingExactGoal,
            },
        )
        .expect("classification")
    }

    #[test]
    fn exact_1024_boundary_uses_the_last_denominator_request_projection() {
        assert_eq!(
            exact_window_boundary(&cursor(), &projection(true)).expect("boundary"),
            (2_048, 4_396)
        );
    }

    #[test]
    fn incomplete_suffix_cannot_publish_an_aggregate_boundary() {
        assert_eq!(
            exact_window_boundary(&cursor(), &projection(false)).expect("boundary"),
            (0, 0)
        );
    }

    #[test]
    fn prefix_roots_bind_order_and_reject_replacement() {
        let first = root('a');
        let second = root('b');
        let ordered = journal_prefix_root(
            "nando.s1c4-test-prefix.v1",
            [first.as_str(), second.as_str()],
        )
        .expect("ordered");
        let replaced = journal_prefix_root(
            "nando.s1c4-test-prefix.v1",
            [second.as_str(), first.as_str()],
        )
        .expect("replaced");
        assert_ne!(ordered, replaced);
    }

    #[test]
    fn out_of_order_request_completion_reconstructs_the_frozen_ordinal_order() {
        let (root_dir, paths) = census_paths_for_test("out-of-order");
        let second = censored_row(
            nando_operator_learning::s1c4_classification_genesis_root_v1(),
            22,
            14,
            'b',
        );
        let first = censored_row(second.row_root_sha256.clone(), 21, 13, 'a');
        let projection =
            project_census(&paths, &cursor(), &[second, first], 14).expect("projection");
        assert!(projection.source_complete);
        assert_eq!(projection.denominator_requests, 2);
        assert_eq!(projection.classified_requests, 2);
        assert_eq!(projection.last_denominator_sequence, 22);
        let _ = std::fs::remove_dir_all(root_dir);
    }

    #[test]
    fn missing_request_ordinal_is_visible_as_an_incomplete_source() {
        let (root_dir, paths) = census_paths_for_test("missing-ordinal");
        let row = censored_row(
            nando_operator_learning::s1c4_classification_genesis_root_v1(),
            22,
            14,
            'c',
        );
        let projection = project_census(&paths, &cursor(), &[row], 14).expect("projection");
        assert!(!projection.source_complete);
        assert_eq!(projection.denominator_requests, 2);
        assert_eq!(projection.classified_requests, 1);
        let _ = std::fs::remove_dir_all(root_dir);
    }

    #[test]
    fn integrity_veto_report_is_rooted_terminal_and_keeps_k2_closed() {
        let report = S1c4NaturalCensusReportV1::seal(S1c4NaturalCensusReportV1 {
            schema: S1C4_NATURAL_CENSUS_REPORT_SCHEMA_V1.to_owned(),
            report_root_sha256: String::new(),
            cursor_root_sha256: cursor().cursor_root_sha256,
            state: S1c4NaturalCensusStateV1::Terminal,
            verdict: S1c4NaturalCensusVerdictV1::Veto,
            blocker: "s1c4_journal_prefix_changed".to_owned(),
            generated_at_unix: 1_700_000_001,
            closes_at_unix: 1_700_000_001,
            quiescence_deadline_unix: 1_700_000_001,
            opportunity_end_sequence: 21,
            opportunity_end_request_ordinal: 13,
            opportunity_end_input_tokens: 301,
            denominator_requests: 1,
            denominator_input_tokens: 1,
            classified_requests: 0,
            goal_bound: 0,
            alternative_bearing: 0,
            decision_episodes: 0,
            satisfied_episodes: 0,
            distinct_decision_lineages: 0,
            censor_counts: BTreeMap::new(),
            classification_rows_total: 0,
            classification_last_root_sha256:
                nando_operator_learning::s1c4_classification_genesis_root_v1(),
            queue_overflow: 0,
            writer_failures: 0,
            duplicate_rows: 0,
            false_accepts: 0,
            parity_failures: 0,
            source_complete: false,
            exact_join_complete: false,
            raw_payloads_persisted: false,
            k2_open: false,
            s2_started: false,
            model_training_allowed: false,
            package_activation_allowed: false,
            authority_ready: false,
            phase_mutation_allowed: false,
        })
        .expect("report");
        report.validate().expect("valid report");
        assert_eq!(report.verdict, S1c4NaturalCensusVerdictV1::Veto);
        assert!(!report.k2_open);
        assert!(!report.authority_ready);
    }
}
