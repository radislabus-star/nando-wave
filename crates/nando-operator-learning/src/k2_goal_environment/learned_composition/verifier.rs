use std::collections::BTreeMap;
use std::io::{Read, Write};

use super::model::{
    K2_COMPOSITION_MAX_PROTOCOL_BYTES_V1, K2_COMPOSITION_ORACLE_OUTCOME_SCHEMA_V1,
    K2_COMPOSITION_ORACLE_REQUEST_SCHEMA_V1, K2_COMPOSITION_PLAN_VERIFICATION_SCHEMA_V1,
    K2_COMPOSITION_PLANNER_OUTCOME_SCHEMA_V1, K2_COMPOSITION_PROGRAMS_PER_ROUTE_V1,
    K2CompositionAuthorityBoundaryV1, K2CompositionCandidateV1, K2CompositionDependencyEdgeV1,
    K2CompositionErrorV1, K2CompositionFileEntryV1, K2CompositionLearnedEffectV1,
    K2CompositionOracleOutcomeV1, K2CompositionOracleRequestV1,
    K2CompositionPlanVerificationReceiptV1, K2CompositionPlannerOutcomeV1,
    K2CompositionPlanningRequestV1, K2CompositionProgramDispositionV1, K2CompositionProgramV1,
    K2CompositionResultV1, K2CompositionSemanticClassV1, K2CompositionTreeManifestV1,
    composition_bytes_v1, composition_decode_v1, composition_root_v1,
};

pub fn verify_composition_plan_v1(
    request: &K2CompositionPlanningRequestV1,
    outcome: &K2CompositionPlannerOutcomeV1,
) -> K2CompositionResultV1<K2CompositionPlanVerificationReceiptV1> {
    request.validate()?;
    outcome.authority.validate()?;
    if outcome.schema != K2_COMPOSITION_PLANNER_OUTCOME_SCHEMA_V1
        || outcome.experiment_id_sha256 != request.experiment_id_sha256
        || outcome.request_root_sha256 != request.request_root_sha256
        || outcome.budget_rejected_programs != 0
    {
        return Err(K2CompositionErrorV1::Invalid(
            "planner_outcome_binding_invalid",
        ));
    }
    let mut resealed = outcome.clone();
    let supplied_root = resealed.outcome_root_sha256.clone();
    resealed.reseal()?;
    if resealed.outcome_root_sha256 != supplied_root {
        return Err(K2CompositionErrorV1::Invalid(
            "planner_outcome_root_mismatch",
        ));
    }

    let action_ids = request
        .law_set
        .laws
        .iter()
        .map(|law| law.action_id_sha256.clone())
        .collect::<Vec<_>>();
    let mut raw_programs = Vec::new();
    for first in &action_ids {
        raw_programs.push(vec![first.clone()]);
        for second in &action_ids {
            if second == first {
                continue;
            }
            raw_programs.push(vec![first.clone(), second.clone()]);
            for third in &action_ids {
                if third != first && third != second {
                    raw_programs.push(vec![first.clone(), second.clone(), third.clone()]);
                }
            }
        }
    }
    if raw_programs.len() != K2_COMPOSITION_PROGRAMS_PER_ROUTE_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "verifier_program_denominator_invalid",
        ));
    }
    let mut candidates = Vec::with_capacity(raw_programs.len());
    for raw in raw_programs {
        let program = K2CompositionProgramV1::seal(raw)?;
        let mut current = request.target.entries.clone();
        let mut failure = None;
        for (step, action_id) in program.action_ids_sha256.iter().enumerate() {
            let law = request
                .law_set
                .law(action_id)
                .ok_or(K2CompositionErrorV1::Invalid("verifier_action_unknown"))?;
            match verifier_apply_effect_v1(&current, &law.effect) {
                Ok(next) => current = next,
                Err(reason) => {
                    failure = Some((step as u64, reason.to_owned()));
                    break;
                }
            }
        }
        let disposition = if let Some((step, reason)) = failure {
            K2CompositionProgramDispositionV1::InapplicableAtStep { step, reason }
        } else {
            let terminal = K2CompositionTreeManifestV1::seal_entries(current)?;
            let exact_goal_satisfied = terminal == request.goal.expected_terminal;
            K2CompositionProgramDispositionV1::ValidPrediction {
                terminal,
                exact_goal_satisfied,
            }
        };
        candidates.push(K2CompositionCandidateV1::seal(program, disposition)?);
    }
    candidates.sort_by(|left, right| {
        left.program
            .program_root_sha256
            .cmp(&right.program.program_root_sha256)
    });
    if candidates != outcome.candidates {
        return Err(K2CompositionErrorV1::Invalid("planner_parity_failure"));
    }

    let classes = verifier_semantic_classes_v1(&candidates)?;
    if classes != outcome.semantic_classes {
        return Err(K2CompositionErrorV1::Invalid("quotient_mismatch"));
    }
    let dependency_edges = verifier_dependency_edges_v1(request);
    if dependency_edges != outcome.dependency_edges {
        return Err(K2CompositionErrorV1::Invalid(
            "dependency_topology_mismatch",
        ));
    }
    let normalized_topology_root_sha256 = verifier_topology_root_v1(request, &dependency_edges)?;
    if normalized_topology_root_sha256 != outcome.normalized_topology_root_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "normalized_topology_mismatch",
        ));
    }

    let valid = candidates
        .iter()
        .filter(|candidate| {
            matches!(
                candidate.disposition,
                K2CompositionProgramDispositionV1::ValidPrediction { .. }
            )
        })
        .count() as u64;
    if valid != outcome.valid_programs
        || candidates.len() as u64 - valid != outcome.inapplicable_programs
    {
        return Err(K2CompositionErrorV1::Invalid(
            "program_disposition_count_mismatch",
        ));
    }
    let satisfying = classes
        .iter()
        .filter(|class| class.exact_goal_satisfied)
        .collect::<Vec<_>>();
    let [selected] = satisfying.as_slice() else {
        return Err(K2CompositionErrorV1::Invalid(
            "verifier_satisfying_class_invalid",
        ));
    };
    if selected.class_root_sha256 != outcome.selected_class_root_sha256
        || selected.depth != 3
        || outcome.minimum_satisfying_depth != 3
        || outcome.satisfying_strict_prefixes != 0
        || !selected
            .member_program_roots_sha256
            .contains(&outcome.selected_program.program_root_sha256)
    {
        return Err(K2CompositionErrorV1::Invalid(
            "selected_program_verification_failed",
        ));
    }

    let authority = K2CompositionAuthorityBoundaryV1::denied();
    let verification_root_sha256 = composition_root_v1(&(
        K2_COMPOSITION_PLAN_VERIFICATION_SCHEMA_V1,
        &request.experiment_id_sha256,
        &outcome.outcome_root_sha256,
        candidates.len() as u64,
        classes.len() as u64,
        true,
        true,
        &authority,
    ))?;
    Ok(K2CompositionPlanVerificationReceiptV1 {
        schema: K2_COMPOSITION_PLAN_VERIFICATION_SCHEMA_V1.to_owned(),
        experiment_id_sha256: request.experiment_id_sha256.clone(),
        planner_outcome_root_sha256: outcome.outcome_root_sha256.clone(),
        independently_verified_candidates: candidates.len() as u64,
        independently_verified_classes: classes.len() as u64,
        minimum_depth_verified: true,
        strict_prefixes_verified: true,
        authority,
        verification_root_sha256,
    })
}

fn verifier_apply_effect_v1(
    current: &[K2CompositionFileEntryV1],
    effect: &K2CompositionLearnedEffectV1,
) -> Result<Vec<K2CompositionFileEntryV1>, &'static str> {
    let mut next = current.to_vec();
    match effect {
        K2CompositionLearnedEffectV1::CopyFile {
            source_path,
            target_path,
        } => {
            let source = next
                .iter()
                .find(|entry| &entry.path == source_path)
                .cloned()
                .ok_or("copy_source_missing")?;
            next.retain(|entry| &entry.path != target_path);
            next.push(K2CompositionFileEntryV1 {
                path: target_path.clone(),
                content_sha256: source.content_sha256,
                byte_len: source.byte_len,
            });
        }
        K2CompositionLearnedEffectV1::RemoveFile { path } => {
            let before = next.len();
            next.retain(|entry| &entry.path != path);
            if next.len() == before {
                return Err("remove_path_missing");
            }
        }
    }
    next.sort();
    Ok(next)
}

fn verifier_semantic_classes_v1(
    candidates: &[K2CompositionCandidateV1],
) -> K2CompositionResultV1<Vec<K2CompositionSemanticClassV1>> {
    let mut grouped = BTreeMap::<(u64, Vec<String>, String), (Vec<String>, bool)>::new();
    for candidate in candidates {
        if let K2CompositionProgramDispositionV1::ValidPrediction {
            terminal,
            exact_goal_satisfied,
        } = &candidate.disposition
        {
            let mut actions = candidate.program.action_ids_sha256.clone();
            actions.sort();
            let group = grouped
                .entry((
                    candidate.program.depth(),
                    actions,
                    terminal.tree_root_sha256.clone(),
                ))
                .or_insert_with(|| (Vec::new(), *exact_goal_satisfied));
            if group.1 != *exact_goal_satisfied {
                return Err(K2CompositionErrorV1::Invalid(
                    "verifier_class_goal_conflict",
                ));
            }
            group.0.push(candidate.program.program_root_sha256.clone());
        }
    }
    let mut classes = Vec::with_capacity(grouped.len());
    for ((depth, actions, terminal_root), (members, satisfies)) in grouped {
        classes.push(K2CompositionSemanticClassV1::seal(
            depth,
            actions,
            terminal_root,
            members,
            satisfies,
        )?);
    }
    classes.sort_by(|left, right| left.class_root_sha256.cmp(&right.class_root_sha256));
    Ok(classes)
}

fn verifier_dependency_edges_v1(
    request: &K2CompositionPlanningRequestV1,
) -> Vec<K2CompositionDependencyEdgeV1> {
    let mut edges = Vec::new();
    for left in &request.law_set.laws {
        for right in &request.law_set.laws {
            if left.action_id_sha256 == right.action_id_sha256 {
                continue;
            }
            let writes = left.effect.write_paths();
            let reads = right.effect.read_paths();
            for path in writes {
                if reads.contains(&path) {
                    edges.push(K2CompositionDependencyEdgeV1 {
                        writer_action_id_sha256: left.action_id_sha256.clone(),
                        reader_action_id_sha256: right.action_id_sha256.clone(),
                        path,
                    });
                }
            }
        }
    }
    edges.sort_by(|left, right| {
        (
            &left.writer_action_id_sha256,
            &left.reader_action_id_sha256,
            &left.path,
        )
            .cmp(&(
                &right.writer_action_id_sha256,
                &right.reader_action_id_sha256,
                &right.path,
            ))
    });
    edges
}

fn verifier_topology_root_v1(
    request: &K2CompositionPlanningRequestV1,
    edges: &[K2CompositionDependencyEdgeV1],
) -> K2CompositionResultV1<String> {
    let mut signature = Vec::new();
    for law in &request.law_set.laws {
        let incoming = edges
            .iter()
            .filter(|edge| edge.reader_action_id_sha256 == law.action_id_sha256)
            .count() as u64;
        let outgoing = edges
            .iter()
            .filter(|edge| edge.writer_action_id_sha256 == law.action_id_sha256)
            .count() as u64;
        signature.push((incoming, outgoing));
    }
    signature.sort();
    composition_root_v1(&("nando.k2-composition-source-neutral-topology.v1", signature))
}

pub fn evaluate_exact_composition_goal_v1(
    request: &K2CompositionOracleRequestV1,
) -> K2CompositionResultV1<K2CompositionOracleOutcomeV1> {
    request.observed_terminal.validate()?;
    request.goal.validate()?;
    request.authority.validate()?;
    let expected_request_root = composition_root_v1(&(
        K2_COMPOSITION_ORACLE_REQUEST_SCHEMA_V1,
        &request.experiment_id_sha256,
        &request.observed_terminal,
        &request.goal,
        &request.authority,
    ))?;
    if request.schema != K2_COMPOSITION_ORACLE_REQUEST_SCHEMA_V1
        || expected_request_root != request.request_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid("oracle_request_invalid"));
    }
    let exact_goal_satisfied = request.observed_terminal == request.goal.expected_terminal;
    let authority = K2CompositionAuthorityBoundaryV1::denied();
    let observed_tree_root_sha256 = request.observed_terminal.tree_root_sha256.clone();
    let expected_tree_root_sha256 = request.goal.expected_terminal.tree_root_sha256.clone();
    let outcome_root_sha256 = composition_root_v1(&(
        K2_COMPOSITION_ORACLE_OUTCOME_SCHEMA_V1,
        &request.request_root_sha256,
        exact_goal_satisfied,
        &observed_tree_root_sha256,
        &expected_tree_root_sha256,
        &authority,
    ))?;
    Ok(K2CompositionOracleOutcomeV1 {
        schema: K2_COMPOSITION_ORACLE_OUTCOME_SCHEMA_V1.to_owned(),
        request_root_sha256: request.request_root_sha256.clone(),
        exact_goal_satisfied,
        observed_tree_root_sha256,
        expected_tree_root_sha256,
        authority,
        outcome_root_sha256,
    })
}

pub fn run_composition_exact_oracle_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_COMPOSITION_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_oracle_stdin"))?;
    let request: K2CompositionOracleRequestV1 = composition_decode_v1(&input)?;
    let outcome = evaluate_exact_composition_goal_v1(&request)?;
    let output = composition_bytes_v1(&outcome)?;
    std::io::stdout()
        .write_all(&output)
        .map_err(|_| K2CompositionErrorV1::Io("write_oracle_stdout"))
}
