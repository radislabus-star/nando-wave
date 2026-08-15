use std::cmp::Ordering;
use std::io::{Read, Write};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    K2CompositionTreeManifestV1, composition_root_v1,
};
use super::model::{
    K2_REPRESENTATION_MAX_DEPTH_V1, K2_REPRESENTATION_MAX_PROTOCOL_BYTES_V1,
    K2_REPRESENTATION_POLICY_OUTCOME_SCHEMA_V1, K2RepresentationBeamLayerV1,
    K2RepresentationPolicyOutcomeV1, K2RepresentationPolicyRequestV1,
    K2RepresentationPolicyTraceV1, K2RepresentationProgramV1, apply_feature_transition_v1,
    extract_policy_features_v1, hidden_score_v1, representation_bytes_v1, representation_decode_v1,
    representation_executable_matches_v1,
};

#[derive(Clone)]
struct BeamCandidateV1 {
    program: K2RepresentationProgramV1,
    current: K2CompositionTreeManifestV1,
    cumulative_score: i64,
}

pub fn run_hidden_representation_policy_v1(
    request: &K2RepresentationPolicyRequestV1,
) -> K2CompositionResultV1<K2RepresentationPolicyOutcomeV1> {
    request.validate()?;
    let mut beam = Vec::<BeamCandidateV1>::new();
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
                if action_evaluations > request.maximum_action_evaluations {
                    return Err(K2CompositionErrorV1::Invalid(
                        "representation_policy_action_budget_exceeded",
                    ));
                }
                let features = extract_policy_features_v1(
                    &request.task,
                    &current,
                    &prefix,
                    &law.action_id_sha256,
                )?;
                let (hidden, action_score) = hidden_score_v1(&request.model, &features)?;
                let cumulative_score = prefix_score.saturating_add(action_score);
                let mut action_ids = prefix.clone();
                action_ids.push(law.action_id_sha256.clone());
                let program = K2RepresentationProgramV1::seal(action_ids)?;
                let transition = apply_feature_transition_v1(&current, &law.effect);
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
                    expanded.push(BeamCandidateV1 {
                        program,
                        current: next,
                        cumulative_score,
                    });
                }
            }
        }
        expanded.sort_by(compare_beam_candidates_v1);
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
        .min_by(|left, right| compare_beam_candidates_v1(left, right));
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

pub fn run_hidden_representation_policy_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_REPRESENTATION_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_representation_policy_stdin"))?;
    let request: K2RepresentationPolicyRequestV1 = representation_decode_v1(&input)?;
    representation_executable_matches_v1(&request.policy_executable_sha256)?;
    let outcome = run_hidden_representation_policy_v1(&request)?;
    std::io::stdout()
        .write_all(&representation_bytes_v1(&outcome)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_representation_policy_stdout"))
}

fn compare_beam_candidates_v1(left: &BeamCandidateV1, right: &BeamCandidateV1) -> Ordering {
    right
        .cumulative_score
        .cmp(&left.cumulative_score)
        .then_with(|| {
            left.program
                .program_root_sha256
                .cmp(&right.program.program_root_sha256)
        })
}
