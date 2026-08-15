use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionFileEntryV1,
    K2CompositionLearnedEffectV1, K2CompositionResultV1, K2CompositionTreeManifestV1,
    composition_root_v1,
};
use super::model::{
    K2_REPRESENTATION_BASELINE_OUTCOME_SCHEMA_V1, K2_REPRESENTATION_COMPLETE_PROGRAMS_V1,
    K2_REPRESENTATION_FEATURE_NAMES_V1, K2_REPRESENTATION_FEATURE_SCALE_V1,
    K2_REPRESENTATION_FEATURES_V1, K2_REPRESENTATION_MAX_DEPTH_V1,
    K2_REPRESENTATION_MAX_PROTOCOL_BYTES_V1, K2_REPRESENTATION_POLICY_OUTCOME_SCHEMA_V1,
    K2_REPRESENTATION_VERIFICATION_SCHEMA_V1, K2_REPRESENTATION_VERIFIER_REQUEST_SCHEMA_V1,
    K2RepresentationBaselineOutcomeV1, K2RepresentationBeamLayerV1,
    K2RepresentationFeatureVectorV1, K2RepresentationPolicyOutcomeV1,
    K2RepresentationPolicyRequestV1, K2RepresentationPolicyTraceV1, K2RepresentationProgramV1,
    K2RepresentationTaskV1, K2RepresentationVerificationReceiptV1,
    K2RepresentationVerifierRequestV1, representation_bytes_v1, representation_decode_v1,
    representation_executable_matches_v1,
};

#[derive(Clone)]
struct VerifierBeamCandidateV1 {
    program: K2RepresentationProgramV1,
    current: K2CompositionTreeManifestV1,
    cumulative_score: i64,
}

pub fn verify_hidden_representation_v1(
    request: &K2RepresentationVerifierRequestV1,
) -> K2CompositionResultV1<K2RepresentationVerificationReceiptV1> {
    validate_verifier_request_v1(request)?;
    let reconstructed_baseline = verifier_complete_baseline_v1(
        &request.policy_request.task,
        &request.baseline_outcome.request_root_sha256,
    )?;
    if reconstructed_baseline != request.baseline_outcome {
        return Err(K2CompositionErrorV1::Invalid(
            "representation_independent_baseline_mismatch",
        ));
    }
    let reconstructed_policy = verifier_reconstruct_policy_v1(&request.policy_request)?;
    if reconstructed_policy != request.policy_outcome {
        return Err(K2CompositionErrorV1::Invalid(
            "representation_independent_policy_mismatch",
        ));
    }
    let selected =
        request
            .policy_outcome
            .selected_program
            .as_ref()
            .ok_or(K2CompositionErrorV1::Invalid(
                "representation_verified_program_missing",
            ))?;
    let selected_is_minimum_satisfying = request
        .baseline_outcome
        .minimum_satisfying_programs
        .iter()
        .any(|program| program == selected);
    if !selected_is_minimum_satisfying
        || !request.policy_outcome.exact_goal_satisfied
        || request.policy_outcome.action_evaluations
            > request.policy_request.maximum_action_evaluations
    {
        return Err(K2CompositionErrorV1::Invalid(
            "representation_verified_selection_invalid",
        ));
    }
    let authority = K2CompositionAuthorityBoundaryV1::denied();
    let verification_root_sha256 = composition_root_v1(&(
        K2_REPRESENTATION_VERIFICATION_SCHEMA_V1,
        &request.policy_request.task.task_root_sha256,
        &request.policy_outcome.outcome_root_sha256,
        &request.baseline_outcome.outcome_root_sha256,
        K2_REPRESENTATION_COMPLETE_PROGRAMS_V1,
        request.policy_outcome.action_evaluations,
        selected_is_minimum_satisfying,
        request.policy_outcome.exact_goal_satisfied,
        &authority,
    ))?;
    Ok(K2RepresentationVerificationReceiptV1 {
        schema: K2_REPRESENTATION_VERIFICATION_SCHEMA_V1.to_owned(),
        task_root_sha256: request.policy_request.task.task_root_sha256.clone(),
        policy_outcome_root_sha256: request.policy_outcome.outcome_root_sha256.clone(),
        baseline_outcome_root_sha256: request.baseline_outcome.outcome_root_sha256.clone(),
        independently_reconstructed_programs: K2_REPRESENTATION_COMPLETE_PROGRAMS_V1,
        independently_reconstructed_evaluations: request.policy_outcome.action_evaluations,
        selected_is_minimum_satisfying,
        exact_goal_satisfied: request.policy_outcome.exact_goal_satisfied,
        authority,
        verification_root_sha256,
    })
}

pub fn run_hidden_representation_verifier_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_REPRESENTATION_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_representation_verifier_stdin"))?;
    let request: K2RepresentationVerifierRequestV1 = representation_decode_v1(&input)?;
    representation_executable_matches_v1(&request.verifier_executable_sha256)?;
    let receipt = verify_hidden_representation_v1(&request)?;
    std::io::stdout()
        .write_all(&representation_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_representation_verifier_stdout"))
}

fn validate_verifier_request_v1(
    request: &K2RepresentationVerifierRequestV1,
) -> K2CompositionResultV1<()> {
    request.policy_request.validate()?;
    request.authority.validate()?;
    let expected = composition_root_v1(&(
        K2_REPRESENTATION_VERIFIER_REQUEST_SCHEMA_V1,
        &request.verifier_executable_sha256,
        &request.policy_request,
        &request.policy_outcome,
        &request.baseline_outcome,
        &request.authority,
    ))?;
    if request.schema != K2_REPRESENTATION_VERIFIER_REQUEST_SCHEMA_V1
        || request.policy_request.task.task_root_sha256 != request.policy_outcome.task_root_sha256
        || request.policy_request.task.task_root_sha256 != request.baseline_outcome.task_root_sha256
        || expected != request.request_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "representation_verifier_request_invalid",
        ));
    }
    Ok(())
}

fn verifier_complete_baseline_v1(
    task: &K2RepresentationTaskV1,
    request_root_sha256: &str,
) -> K2CompositionResultV1<K2RepresentationBaselineOutcomeV1> {
    let action_ids = task
        .laws
        .iter()
        .map(|law| law.action_id_sha256.clone())
        .collect::<Vec<_>>();
    let mut programs = Vec::new();
    verifier_extend_programs_v1(&action_ids, &mut Vec::new(), &mut programs)?;
    if programs.len() as u64 != K2_REPRESENTATION_COMPLETE_PROGRAMS_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "representation_verifier_denominator_mismatch",
        ));
    }
    let mut candidate_roots = Vec::with_capacity(programs.len());
    let mut satisfying = Vec::new();
    let mut valid_programs = 0_u64;
    let mut inapplicable_programs = 0_u64;
    for program in programs {
        let mut current = task.initial.clone();
        let mut failure = None;
        for (step, action_id) in program.action_ids_sha256.iter().enumerate() {
            let law = task.law(action_id).ok_or(K2CompositionErrorV1::Invalid(
                "representation_verifier_action_unknown",
            ))?;
            match verifier_apply_effect_v1(&current, &law.effect) {
                Ok(next) => current = next,
                Err(reason) => {
                    failure = Some((step as u64, reason));
                    break;
                }
            }
        }
        if let Some((step, reason)) = failure {
            inapplicable_programs += 1;
            candidate_roots.push(composition_root_v1(&(
                "nando.k2-representation-baseline-candidate.v1",
                &program,
                "inapplicable",
                step,
                reason,
            ))?);
        } else {
            valid_programs += 1;
            let exact = current == task.goal.expected_terminal;
            candidate_roots.push(composition_root_v1(&(
                "nando.k2-representation-baseline-candidate.v1",
                &program,
                "valid",
                &current.tree_root_sha256,
                exact,
            ))?);
            if exact {
                satisfying.push(program);
            }
        }
    }
    let minimum_satisfying_depth = satisfying
        .iter()
        .map(K2RepresentationProgramV1::depth)
        .min()
        .ok_or(K2CompositionErrorV1::Invalid(
            "representation_verifier_goal_unreachable",
        ))?;
    let satisfying_strict_prefixes = satisfying
        .iter()
        .filter(|program| program.depth() < minimum_satisfying_depth)
        .count() as u64;
    satisfying.retain(|program| program.depth() == minimum_satisfying_depth);
    satisfying.sort_by(|left, right| left.program_root_sha256.cmp(&right.program_root_sha256));
    candidate_roots.sort();
    let candidate_set_root_sha256 =
        composition_root_v1(&("nando.k2-representation-candidate-set.v1", &candidate_roots))?;
    let mut outcome = K2RepresentationBaselineOutcomeV1 {
        schema: K2_REPRESENTATION_BASELINE_OUTCOME_SCHEMA_V1.to_owned(),
        task_root_sha256: task.task_root_sha256.clone(),
        request_root_sha256: request_root_sha256.to_owned(),
        complete_programs: K2_REPRESENTATION_COMPLETE_PROGRAMS_V1,
        valid_programs,
        inapplicable_programs,
        candidate_set_root_sha256,
        minimum_satisfying_depth,
        minimum_satisfying_programs: satisfying,
        satisfying_strict_prefixes,
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        outcome_root_sha256: String::new(),
    };
    outcome.reseal()?;
    Ok(outcome)
}

fn verifier_extend_programs_v1(
    action_ids: &[String],
    prefix: &mut Vec<String>,
    output: &mut Vec<K2RepresentationProgramV1>,
) -> K2CompositionResultV1<()> {
    if prefix.len() >= K2_REPRESENTATION_MAX_DEPTH_V1 as usize {
        return Ok(());
    }
    for action_id in action_ids {
        if prefix.contains(action_id) {
            continue;
        }
        prefix.push(action_id.clone());
        output.push(K2RepresentationProgramV1::seal(prefix.clone())?);
        verifier_extend_programs_v1(action_ids, prefix, output)?;
        prefix.pop();
    }
    Ok(())
}

fn verifier_reconstruct_policy_v1(
    request: &K2RepresentationPolicyRequestV1,
) -> K2CompositionResultV1<K2RepresentationPolicyOutcomeV1> {
    let mut beam = Vec::<VerifierBeamCandidateV1>::new();
    let mut trace = Vec::new();
    let mut layers = Vec::new();
    let mut action_evaluations = 0_u64;
    let mut exact_score_ties = 0_u64;
    for depth in 1..=K2_REPRESENTATION_MAX_DEPTH_V1 {
        let prefixes = if depth == 1 {
            vec![(Vec::<String>::new(), request.task.initial.clone(), 0_i64)]
        } else {
            beam.iter()
                .map(|candidate| {
                    (
                        candidate.program.action_ids_sha256.clone(),
                        candidate.current.clone(),
                        candidate.cumulative_score,
                    )
                })
                .collect::<Vec<_>>()
        };
        let mut expanded = Vec::new();
        for (prefix, current, prefix_score) in prefixes {
            for law in &request.task.laws {
                if prefix.contains(&law.action_id_sha256) {
                    continue;
                }
                action_evaluations += 1;
                let features =
                    verifier_features_v1(&request.task, &current, &prefix, &law.action_id_sha256)?;
                let (hidden, action_score) = verifier_hidden_score_v1(&request.model, &features);
                let cumulative_score = prefix_score.saturating_add(action_score);
                let mut action_ids = prefix.clone();
                action_ids.push(law.action_id_sha256.clone());
                let program = K2RepresentationProgramV1::seal(action_ids)?;
                let transition = verifier_apply_effect_v1(&current, &law.effect);
                let (applicable, resulting_tree_root_sha256) = match &transition {
                    Ok(next) => (true, Some(next.tree_root_sha256.clone())),
                    Err(_) => (false, None),
                };
                let prefix_program_root_sha256 = if prefix.is_empty() {
                    None
                } else {
                    Some(K2RepresentationProgramV1::seal(prefix.clone())?.program_root_sha256)
                };
                let trace_root_sha256 = composition_root_v1(&(
                    "nando.k2-representation-policy-trace.v1",
                    depth,
                    &prefix_program_root_sha256,
                    &law.action_id_sha256,
                    &features,
                    &hidden,
                    action_score,
                    cumulative_score,
                    applicable,
                    &program.program_root_sha256,
                    &resulting_tree_root_sha256,
                ))?;
                trace.push(K2RepresentationPolicyTraceV1 {
                    depth,
                    prefix_program_root_sha256,
                    action_id_sha256: law.action_id_sha256.clone(),
                    features,
                    hidden,
                    action_score,
                    cumulative_score,
                    applicable,
                    resulting_program_root_sha256: program.program_root_sha256.clone(),
                    resulting_tree_root_sha256,
                    trace_root_sha256,
                });
                if let Ok(next) = transition {
                    expanded.push(VerifierBeamCandidateV1 {
                        program,
                        current: next,
                        cumulative_score,
                    });
                }
            }
        }
        expanded.sort_by(verifier_compare_beam_v1);
        exact_score_ties += expanded
            .windows(2)
            .filter(|pair| pair[0].cumulative_score == pair[1].cumulative_score)
            .count() as u64;
        expanded.truncate(request.beam_width as usize);
        let retained_program_roots_sha256 = expanded
            .iter()
            .map(|candidate| candidate.program.program_root_sha256.clone())
            .collect::<Vec<_>>();
        let layer_root_sha256 = composition_root_v1(&(
            "nando.k2-representation-beam-layer.v1",
            depth,
            &retained_program_roots_sha256,
        ))?;
        layers.push(K2RepresentationBeamLayerV1 {
            depth,
            retained_program_roots_sha256,
            layer_root_sha256,
        });
        beam = expanded;
        if beam.is_empty() {
            break;
        }
    }
    let selected = beam
        .iter()
        .filter(|candidate| candidate.current == request.task.goal.expected_terminal)
        .min_by(|left, right| verifier_compare_beam_v1(left, right));
    let (selected_program, selected_terminal, exact_goal_satisfied) = match selected {
        Some(candidate) => (
            Some(candidate.program.clone()),
            Some(candidate.current.clone()),
            true,
        ),
        None => (None, None, false),
    };
    let mut outcome = K2RepresentationPolicyOutcomeV1 {
        schema: K2_REPRESENTATION_POLICY_OUTCOME_SCHEMA_V1.to_owned(),
        task_root_sha256: request.task.task_root_sha256.clone(),
        request_root_sha256: request.request_root_sha256.clone(),
        selected_program,
        selected_terminal,
        exact_goal_satisfied,
        action_evaluations,
        exact_score_ties,
        trace,
        layers,
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        outcome_root_sha256: String::new(),
    };
    outcome.reseal()?;
    Ok(outcome)
}

fn verifier_features_v1(
    task: &K2RepresentationTaskV1,
    current: &K2CompositionTreeManifestV1,
    used_action_ids: &[String],
    action_id: &str,
) -> K2CompositionResultV1<K2RepresentationFeatureVectorV1> {
    let law = task.law(action_id).ok_or(K2CompositionErrorV1::Invalid(
        "representation_verifier_feature_action_missing",
    ))?;
    let current_entries = current
        .entries
        .iter()
        .map(|entry| (entry.path.clone(), entry.clone()))
        .collect::<BTreeMap<_, _>>();
    let goal_entries = task
        .goal
        .expected_terminal
        .entries
        .iter()
        .map(|entry| (entry.path.clone(), entry.clone()))
        .collect::<BTreeMap<_, _>>();
    let used = used_action_ids.iter().collect::<BTreeSet<_>>();
    let remaining = task
        .laws
        .iter()
        .filter(|other| {
            other.action_id_sha256 != action_id && !used.contains(&other.action_id_sha256)
        })
        .collect::<Vec<_>>();
    let mut values = vec![0_i64; K2_REPRESENTATION_FEATURES_V1];
    values[0] = K2_REPRESENTATION_FEATURE_SCALE_V1;
    values[13] = ((K2_REPRESENTATION_MAX_DEPTH_V1.saturating_sub(used.len() as u64))
        * K2_REPRESENTATION_FEATURE_SCALE_V1 as u64
        / K2_REPRESENTATION_MAX_DEPTH_V1) as i64;
    match &law.effect {
        K2CompositionLearnedEffectV1::CopyFile {
            source_path,
            target_path,
        } => {
            values[2] = K2_REPRESENTATION_FEATURE_SCALE_V1;
            let source = current_entries.get(source_path);
            let goal_target = goal_entries.get(target_path);
            values[1] = verifier_bool_v1(source.is_some());
            values[4] = verifier_bool_v1(goal_target.is_some());
            values[5] = verifier_bool_v1(verifier_same_file_value_v1(source, goal_target));
            values[6] = verifier_bool_v1(
                source.is_some()
                    && !verifier_same_file_value_v1(source, current_entries.get(target_path)),
            );
            let consumed = remaining
                .iter()
                .any(|other| other.effect.read_paths().contains(target_path));
            values[7] = verifier_bool_v1(consumed);
            values[8] = verifier_bool_v1(
                source.is_none()
                    && remaining
                        .iter()
                        .any(|other| other.effect.write_paths().contains(source_path)),
            );
            values[12] = verifier_bool_v1(goal_target.is_none() && !consumed);
        }
        K2CompositionLearnedEffectV1::RemoveFile { path } => {
            values[3] = K2_REPRESENTATION_FEATURE_SCALE_V1;
            values[1] = verifier_bool_v1(current_entries.contains_key(path));
            values[9] = verifier_bool_v1(!goal_entries.contains_key(path));
            values[10] = verifier_bool_v1(goal_entries.contains_key(path));
            values[11] = verifier_bool_v1(
                remaining
                    .iter()
                    .any(|other| other.effect.read_paths().contains(path)),
            );
        }
    }
    let feature_root_sha256 = composition_root_v1(&(
        "nando.k2-representation-feature-vector.v1",
        K2_REPRESENTATION_FEATURE_NAMES_V1,
        &values,
    ))?;
    Ok(K2RepresentationFeatureVectorV1 {
        schema: "nando.k2-representation-feature-vector.v1".to_owned(),
        values,
        feature_root_sha256,
    })
}

fn verifier_hidden_score_v1(
    model: &super::model::K2MeaningPolicySnapshotV1,
    features: &K2RepresentationFeatureVectorV1,
) -> (Vec<i64>, i64) {
    let hidden = model
        .encoder_weights
        .iter()
        .map(|row| {
            row.iter()
                .zip(&features.values)
                .map(|(weight, value)| weight.saturating_mul(*value))
                .sum::<i64>()
                .saturating_div(K2_REPRESENTATION_FEATURE_SCALE_V1)
                .clamp(0, 1_000_000)
        })
        .collect::<Vec<_>>();
    let score = model
        .output_weights
        .iter()
        .zip(&hidden)
        .map(|(weight, value)| weight.saturating_mul(*value))
        .sum();
    (hidden, score)
}

fn verifier_apply_effect_v1(
    current: &K2CompositionTreeManifestV1,
    effect: &K2CompositionLearnedEffectV1,
) -> Result<K2CompositionTreeManifestV1, &'static str> {
    let mut entries = current
        .entries
        .iter()
        .cloned()
        .map(|entry| (entry.path.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    match effect {
        K2CompositionLearnedEffectV1::CopyFile {
            source_path,
            target_path,
        } => {
            let source = entries
                .get(source_path)
                .cloned()
                .ok_or("baseline_copy_source_missing")?;
            entries.insert(
                target_path.clone(),
                K2CompositionFileEntryV1 {
                    path: target_path.clone(),
                    content_sha256: source.content_sha256,
                    byte_len: source.byte_len,
                },
            );
        }
        K2CompositionLearnedEffectV1::RemoveFile { path } => {
            if entries.remove(path).is_none() {
                return Err("baseline_remove_path_missing");
            }
        }
    }
    K2CompositionTreeManifestV1::seal_entries(entries.into_values().collect())
        .map_err(|_| "baseline_manifest_invalid")
}

fn verifier_compare_beam_v1(
    left: &VerifierBeamCandidateV1,
    right: &VerifierBeamCandidateV1,
) -> Ordering {
    right
        .cumulative_score
        .cmp(&left.cumulative_score)
        .then_with(|| {
            left.program
                .program_root_sha256
                .cmp(&right.program.program_root_sha256)
        })
}

fn verifier_same_file_value_v1(
    left: Option<&K2CompositionFileEntryV1>,
    right: Option<&K2CompositionFileEntryV1>,
) -> bool {
    matches!(
        (left, right),
        (Some(left), Some(right))
            if left.content_sha256 == right.content_sha256 && left.byte_len == right.byte_len
    )
}

const fn verifier_bool_v1(value: bool) -> i64 {
    if value {
        K2_REPRESENTATION_FEATURE_SCALE_V1
    } else {
        0
    }
}
