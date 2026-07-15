use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use nando_transition_inducer::{
    LiveObservedTransition, LivePackageOrigin, LiveProfileRegistry, RawPhaseConfig,
    RawPhaseFamilyState, RawPhaseInducer, RawPhaseSurfaceState, TransitionTrace,
    evaluate_leave_one_surface_out, evaluate_support_query_transfer, import_package_with_origin,
    split_forward_adaptation_query, timestamp_unix_nanos, transition_family_key,
};
use serde_json::Value;

use super::{Config, stable_hash, structural_class_key};

const MIN_SURFACE_ROWS: usize = 12;
const MAX_ROWS_PER_SURFACE: usize = 64;
type TransferBatches = (Vec<Vec<TransitionTrace>>, Vec<Vec<TransitionTrace>>);

struct FrontierObservation {
    observed_surfaces: usize,
    eligible_surfaces: usize,
    support_rows: usize,
    already_induced: bool,
    minimum_surfaces: usize,
    surfaces: BTreeMap<String, RawPhaseSurfaceState>,
}

pub(super) fn induce_raw_phase_families(
    registry: &mut LiveProfileRegistry,
    traces: &[LiveObservedTransition],
    config: &Config,
) -> Result<(), String> {
    let mut families = BTreeMap::<String, BTreeMap<String, Vec<&LiveObservedTransition>>>::new();
    for trace in traces {
        families
            .entry(anonymous_family_key(trace))
            .or_default()
            .entry(structural_class_key(trace))
            .or_default()
            .push(trace);
    }
    for (family_key, surfaces) in families {
        let observed_surfaces = surfaces.len();
        let frontier = surface_frontier(&surfaces);
        let mut eligible = surfaces
            .iter()
            .filter(|(_, support)| support.len() >= MIN_SURFACE_ROWS)
            .map(|(surface_key, support)| {
                let mut support = support
                    .iter()
                    .map(|trace| TransitionTrace {
                        before: trace.before.clone(),
                        action: trace.action.clone(),
                        after: trace.after.clone(),
                    })
                    .collect::<Vec<_>>();
                if support.len() > MAX_ROWS_PER_SURFACE {
                    support = support.split_off(support.len() - MAX_ROWS_PER_SURFACE);
                }
                (surface_key.clone(), support)
            })
            .collect::<Vec<_>>();
        eligible.sort_by(|left, right| left.0.cmp(&right.0));
        let support_rows = eligible.iter().map(|(_, support)| support.len()).sum();
        let generation_key = raw_family_generation_key(&family_key, &eligible);
        let already_induced = registry.induced_raw_family_keys.contains(&generation_key);
        let minimum_surfaces = RawPhaseConfig::live_v1().min_cross_surface_support;
        update_observation_state(
            registry,
            &family_key,
            FrontierObservation {
                observed_surfaces,
                eligible_surfaces: eligible.len(),
                support_rows,
                already_induced,
                minimum_surfaces,
                surfaces: frontier,
            },
        );
        if already_induced || eligible.len() < minimum_surfaces {
            continue;
        }

        registry.telemetry.raw_phase_training_attempts = registry
            .telemetry
            .raw_phase_training_attempts
            .saturating_add(1);
        let meta = eligible
            .iter()
            .map(|(_, support)| support.clone())
            .collect::<Vec<_>>();
        let inducer = match RawPhaseInducer::train(&meta, RawPhaseConfig::live_v1()) {
            Ok(inducer) => inducer,
            Err(error) => {
                let state = family_state_mut(registry, &family_key)?;
                state.training_attempts = state.training_attempts.saturating_add(1);
                state.last_reason = format!("raw_phase_train:{error:?}");
                continue;
            }
        };
        update_training_state(registry, &family_key, &inducer)?;
        if !inducer.is_ready() {
            continue;
        }
        let transfer = match evaluate_leave_one_surface_out(&meta, RawPhaseConfig::live_v1()) {
            Ok(transfer) => transfer,
            Err(error) => {
                let state = family_state_mut(registry, &family_key)?;
                state.last_reason = format!("leave_one_surface_out:{error:?}");
                continue;
            }
        };
        {
            let state = family_state_mut(registry, &family_key)?;
            state.transfer_tested_surfaces = transfer.tested_surfaces;
            state.transfer_passed_surfaces = transfer.passed_surfaces;
            state.transfer_query_rows = transfer.query_rows;
            state.transfer_correct_executions = transfer.correct_executions;
            state.transfer_abstains = transfer.abstains;
            state.transfer_wrong_accepts = transfer.wrong_accepts;
            state.leave_one_surface_out_pass = transfer.leave_one_surface_out_pass;
            if !transfer.leave_one_surface_out_pass {
                state.stage = "circuit_formation".to_owned();
                state.last_reason = "awaiting_leave_one_surface_out_transfer".to_owned();
            }
        }
        if !transfer.leave_one_surface_out_pass {
            continue;
        }
        let (session_adaptation, session_query) = session_adaptation_query(&surfaces, &eligible)?;
        let session_transfer = evaluate_support_query_transfer(
            &session_adaptation,
            &session_query,
            RawPhaseConfig::live_v1(),
        )
        .map_err(|error| format!("session_transfer:{error:?}"))?;
        let (time_adaptation, time_query) = forward_time_adaptation_query(&surfaces, &eligible)?;
        let time_transfer = evaluate_support_query_transfer(
            &time_adaptation,
            &time_query,
            RawPhaseConfig::live_v1(),
        )
        .map_err(|error| format!("time_transfer:{error:?}"))?;
        {
            let state = family_state_mut(registry, &family_key)?;
            state.new_session_split_pass = session_transfer.leave_one_surface_out_pass;
            state.session_transfer_query_rows = session_transfer.query_rows;
            state.session_transfer_correct_executions = session_transfer.correct_executions;
            state.session_transfer_abstains = session_transfer.abstains;
            state.session_transfer_wrong_accepts = session_transfer.wrong_accepts;
            state.forward_time_split_pass = time_transfer.leave_one_surface_out_pass;
            state.time_transfer_query_rows = time_transfer.query_rows;
            state.time_transfer_correct_executions = time_transfer.correct_executions;
            state.time_transfer_abstains = time_transfer.abstains;
            state.time_transfer_wrong_accepts = time_transfer.wrong_accepts;
            if !state.new_session_split_pass || !state.forward_time_split_pass {
                state.stage = "circuit_formation".to_owned();
                state.last_reason = "awaiting_session_time_transfer".to_owned();
            }
        }
        if !session_transfer.leave_one_surface_out_pass || !time_transfer.leave_one_surface_out_pass
        {
            continue;
        }

        let mut package_ids = Vec::new();
        let mut induced_surface_packages = Vec::new();
        let mut induction_cpu_ns = 0u64;
        let mut last_error = None;
        let future_evidence_not_before_unix_nanos = surfaces
            .values()
            .flatten()
            .filter_map(|trace| timestamp_unix_nanos(&trace.timestamp))
            .max();
        for (surface_key, support) in &eligible {
            match inducer.induce(support) {
                Ok((package, metrics)) => {
                    induction_cpu_ns = induction_cpu_ns.saturating_add(metrics.induction_cpu_ns);
                    let source = PathBuf::from(format!("raw-live-family-{family_key}"));
                    let imported = import_package_with_origin(
                        registry,
                        &package,
                        &source,
                        &config.state_dir,
                        LivePackageOrigin::RawPhaseInduction,
                    )?;
                    if let Some(cutoff) = future_evidence_not_before_unix_nanos
                        && let Some(record) = registry.packages.get_mut(&package.package_id)
                    {
                        record.future_evidence_not_before_unix_nanos = cutoff;
                    }
                    package_ids.push(package.package_id.clone());
                    induced_surface_packages.push((surface_key.clone(), package.package_id));
                    if imported {
                        registry.telemetry.raw_phase_packages_induced = registry
                            .telemetry
                            .raw_phase_packages_induced
                            .saturating_add(1);
                    }
                }
                Err(error) => last_error = Some(format!("raw_phase_induce:{error:?}")),
            }
        }
        registry.telemetry.raw_phase_induction_cpu_ns = registry
            .telemetry
            .raw_phase_induction_cpu_ns
            .saturating_add(induction_cpu_ns);
        for (surface_key, _) in &induced_surface_packages {
            registry.induced_class_keys.insert(surface_key.clone());
        }
        let complete = package_ids.len() == eligible.len();
        if complete {
            registry
                .induced_raw_family_keys
                .insert(generation_key.clone());
        }
        let state = family_state_mut(registry, &family_key)?;
        state.induction_cpu_ns = state.induction_cpu_ns.saturating_add(induction_cpu_ns);
        state.package_ids.extend(package_ids);
        state.package_ids.sort();
        state.package_ids.dedup();
        for (surface_key, package_id) in induced_surface_packages {
            if let Some(surface) = state.surface_frontier.get_mut(&surface_key) {
                surface.circuit_covered = true;
                surface.package_id = Some(package_id);
                surface.last_reason = "raw_phase_package_entered_quarantine".to_owned();
            }
        }
        refresh_frontier_coverage(state);
        if complete {
            state.stage = "cleanup".to_owned();
            state.last_reason = "compact_packages_entered_quarantine".to_owned();
        } else {
            state.last_reason =
                last_error.unwrap_or_else(|| "raw_phase_partial_induction".to_owned());
        }
    }
    Ok(())
}

fn update_observation_state(
    registry: &mut LiveProfileRegistry,
    family_key: &str,
    mut observation: FrontierObservation,
) {
    let state = registry
        .raw_phase_families
        .entry(family_key.to_owned())
        .or_insert_with(|| RawPhaseFamilyState {
            family_key: family_key.to_owned(),
            ..RawPhaseFamilyState::default()
        });
    for (surface_key, surface) in &mut observation.surfaces {
        if let Some(existing) = state.surface_frontier.get(surface_key) {
            surface.circuit_covered = existing.circuit_covered;
            surface.package_id.clone_from(&existing.package_id);
            if existing.circuit_covered {
                surface.last_reason = existing.last_reason.clone();
            }
        }
    }
    state.surface_frontier = observation.surfaces;
    state.observed_surfaces = observation.observed_surfaces;
    state.eligible_surfaces = observation.eligible_surfaces;
    state.support_rows = observation.support_rows;
    refresh_frontier_coverage(state);
    if observation.already_induced {
        state.stage = "cleanup".to_owned();
        state.phase_circuit_ready = true;
        state.last_reason = "current_surface_generation_already_induced".to_owned();
        for surface in state
            .surface_frontier
            .values_mut()
            .filter(|surface| surface.eligible_for_training)
        {
            surface.circuit_covered = true;
            if surface.last_reason == "awaiting_surface_support" {
                surface.last_reason = "current_surface_generation_already_induced".to_owned();
            }
        }
        refresh_frontier_coverage(state);
    } else if observation.eligible_surfaces < observation.minimum_surfaces {
        state.stage = "memorization".to_owned();
        state.phase_circuit_ready = false;
        state.last_reason = "awaiting_cross_surface_evidence".to_owned();
    }
}

fn surface_frontier(
    surfaces: &BTreeMap<String, Vec<&LiveObservedTransition>>,
) -> BTreeMap<String, RawPhaseSurfaceState> {
    surfaces
        .iter()
        .map(|(surface_key, traces)| {
            let sessions = traces
                .iter()
                .map(|trace| trace.source_session_id_sha256.as_str())
                .filter(|session| !session.is_empty())
                .collect::<BTreeSet<_>>();
            let first_timestamp = traces
                .iter()
                .map(|trace| trace.timestamp.as_str())
                .filter(|timestamp| !timestamp.is_empty())
                .min()
                .unwrap_or_default()
                .to_owned();
            let last_timestamp = traces
                .iter()
                .map(|trace| trace.timestamp.as_str())
                .filter(|timestamp| !timestamp.is_empty())
                .max()
                .unwrap_or_default()
                .to_owned();
            let eligible_for_training = traces.len() >= MIN_SURFACE_ROWS;
            (
                surface_key.clone(),
                RawPhaseSurfaceState {
                    surface_key: surface_key.clone(),
                    observed_rows: traces.len(),
                    observed_tokens: traces.iter().map(|trace| trace.total_tokens).sum(),
                    session_count: sessions.len(),
                    first_timestamp,
                    last_timestamp,
                    eligible_for_training,
                    circuit_covered: false,
                    package_id: None,
                    last_reason: if eligible_for_training {
                        "eligible_awaiting_circuit".to_owned()
                    } else {
                        "awaiting_surface_support".to_owned()
                    },
                },
            )
        })
        .collect()
}

fn refresh_frontier_coverage(state: &mut RawPhaseFamilyState) {
    state.frontier_observed_rows = state
        .surface_frontier
        .values()
        .map(|surface| surface.observed_rows)
        .sum();
    state.frontier_observed_tokens = state
        .surface_frontier
        .values()
        .map(|surface| surface.observed_tokens)
        .sum();
    state.frontier_covered_rows = state
        .surface_frontier
        .values()
        .filter(|surface| surface.circuit_covered)
        .map(|surface| surface.observed_rows)
        .sum();
    state.frontier_covered_tokens = state
        .surface_frontier
        .values()
        .filter(|surface| surface.circuit_covered)
        .map(|surface| surface.observed_tokens)
        .sum();
}

fn session_adaptation_query(
    surfaces: &BTreeMap<String, Vec<&LiveObservedTransition>>,
    eligible: &[(String, Vec<TransitionTrace>)],
) -> Result<TransferBatches, String> {
    let mut adaptation_batches = Vec::with_capacity(eligible.len());
    let mut query_batches = Vec::with_capacity(eligible.len());
    for (surface_key, _) in eligible {
        let traces = surfaces
            .get(surface_key)
            .ok_or_else(|| format!("session_surface_missing:{surface_key}"))?;
        let mut sessions = BTreeMap::<String, Vec<TransitionTrace>>::new();
        for trace in traces {
            if trace.source_session_id_sha256.is_empty() {
                continue;
            }
            sessions
                .entry(trace.source_session_id_sha256.clone())
                .or_default()
                .push(TransitionTrace {
                    before: trace.before.clone(),
                    action: trace.action.clone(),
                    after: trace.after.clone(),
                });
        }
        if sessions.len() < 2 {
            return Ok((Vec::new(), Vec::new()));
        }
        let heldout = sessions
            .keys()
            .next_back()
            .cloned()
            .ok_or_else(|| "session_heldout_missing".to_owned())?;
        let mut adaptation = Vec::new();
        let mut query = Vec::new();
        for (session, rows) in sessions {
            if session == heldout {
                query.extend(rows);
            } else {
                adaptation.extend(rows);
            }
        }
        adaptation_batches.push(adaptation);
        query_batches.push(query);
    }
    Ok((adaptation_batches, query_batches))
}

fn forward_time_adaptation_query(
    surfaces: &BTreeMap<String, Vec<&LiveObservedTransition>>,
    eligible: &[(String, Vec<TransitionTrace>)],
) -> Result<TransferBatches, String> {
    let mut adaptation_batches = Vec::with_capacity(eligible.len());
    let mut query_batches = Vec::with_capacity(eligible.len());
    for (surface_key, _) in eligible {
        let source = surfaces
            .get(surface_key)
            .ok_or_else(|| format!("time_surface_missing:{surface_key}"))?;
        let distinct_timestamps = source
            .iter()
            .map(|trace| trace.timestamp.as_str())
            .filter(|timestamp| !timestamp.is_empty())
            .collect::<BTreeSet<_>>();
        if distinct_timestamps.len() < 2 {
            return Ok((Vec::new(), Vec::new()));
        }
        let mut ordered = source.clone();
        ordered.sort_by(|left, right| {
            left.timestamp
                .cmp(&right.timestamp)
                .then_with(|| left.trace_id.cmp(&right.trace_id))
        });
        let traces = ordered
            .into_iter()
            .map(|trace| TransitionTrace {
                before: trace.before.clone(),
                action: trace.action.clone(),
                after: trace.after.clone(),
            })
            .collect::<Vec<_>>();
        let (adaptation, query) = split_forward_adaptation_query(&traces)
            .map_err(|error| format!("forward_time_split:{error:?}"))?;
        adaptation_batches.push(adaptation);
        query_batches.push(query);
    }
    Ok((adaptation_batches, query_batches))
}

fn update_training_state(
    registry: &mut LiveProfileRegistry,
    family_key: &str,
    inducer: &RawPhaseInducer,
) -> Result<(), String> {
    let training = inducer.metrics();
    registry.telemetry.raw_phase_training_cpu_ns = registry
        .telemetry
        .raw_phase_training_cpu_ns
        .saturating_add(training.training_cpu_ns);
    let state = family_state_mut(registry, family_key)?;
    state.stage = if inducer.is_ready() {
        "circuit_formation".to_owned()
    } else {
        "memorization".to_owned()
    };
    state.verifier_positive_candidates = training.verifier_positive_candidates;
    state.verifier_negative_candidates = training.verifier_negative_candidates;
    state.compact_predicate_candidates = training.compact_predicate_candidates;
    state.discovered_predicates = training.discovered_predicates;
    state.predicate_confidence_milli = training.predicate_confidence_milli;
    state.phase_circuit_ready = training.phase_circuit_ready;
    state.training_attempts = state.training_attempts.saturating_add(1);
    state.training_cpu_ns = state
        .training_cpu_ns
        .saturating_add(training.training_cpu_ns);
    state.last_reason = if inducer.is_ready() {
        "raw_phase_circuit_ready".to_owned()
    } else {
        "raw_phase_predicates_not_identifiable".to_owned()
    };
    Ok(())
}

fn family_state_mut<'a>(
    registry: &'a mut LiveProfileRegistry,
    family_key: &str,
) -> Result<&'a mut RawPhaseFamilyState, String> {
    registry
        .raw_phase_families
        .get_mut(family_key)
        .ok_or_else(|| "raw_phase_family_state_missing".to_owned())
}

fn anonymous_family_key(trace: &LiveObservedTransition) -> String {
    transition_family_key(&trace.before).unwrap_or_else(|_| {
        let mut signature = String::new();
        append_anonymous_shape(&trace.before, &mut signature);
        format!("{:016x}", stable_hash(signature.as_bytes()))
    })
}

fn append_anonymous_shape(value: &Value, output: &mut String) {
    match value {
        Value::Null => output.push('0'),
        Value::Bool(_) => output.push('b'),
        Value::Number(_) => output.push('n'),
        Value::String(_) => output.push('s'),
        Value::Array(values) => {
            output.push('[');
            let mut shapes = values
                .iter()
                .map(|child| {
                    let mut shape = String::new();
                    append_anonymous_shape(child, &mut shape);
                    shape
                })
                .collect::<Vec<_>>();
            shapes.sort();
            shapes.dedup();
            for shape in shapes {
                output.push_str(&shape);
            }
            output.push(']');
        }
        Value::Object(object) => {
            output.push('{');
            let mut shapes = object
                .values()
                .map(|child| {
                    let mut shape = String::new();
                    append_anonymous_shape(child, &mut shape);
                    shape
                })
                .collect::<Vec<_>>();
            shapes.sort();
            shapes.dedup();
            for shape in shapes {
                output.push_str(&shape);
            }
            output.push('}');
        }
    }
}

fn raw_family_generation_key(
    family_key: &str,
    surfaces: &[(String, Vec<TransitionTrace>)],
) -> String {
    let mut bytes = family_key.as_bytes().to_vec();
    for (surface_key, _) in surfaces {
        bytes.extend_from_slice(surface_key.as_bytes());
    }
    format!("{family_key}:{:016x}", stable_hash(&bytes))
}
