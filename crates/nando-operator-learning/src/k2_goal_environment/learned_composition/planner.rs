use std::collections::BTreeMap;
use std::io::{Read, Write};

use super::model::{
    K2_COMPOSITION_MAX_PROTOCOL_BYTES_V1, K2_COMPOSITION_PLANNER_OUTCOME_SCHEMA_V1,
    K2_COMPOSITION_PROGRAMS_PER_ROUTE_V1, K2CompositionAuthorityBoundaryV1,
    K2CompositionCandidateV1, K2CompositionDependencyEdgeV1, K2CompositionErrorV1,
    K2CompositionFileEntryV1, K2CompositionLearnedEffectV1, K2CompositionPlannerOutcomeV1,
    K2CompositionPlanningRequestV1, K2CompositionProgramDispositionV1, K2CompositionProgramV1,
    K2CompositionResultV1, K2CompositionSemanticClassV1, K2CompositionTreeManifestV1,
    composition_bytes_v1, composition_decode_v1, composition_root_v1, composition_sha256_file_v1,
};

pub fn plan_learned_composition_v1(
    request: &K2CompositionPlanningRequestV1,
) -> K2CompositionResultV1<K2CompositionPlannerOutcomeV1> {
    request.validate()?;
    let action_ids = request
        .law_set
        .laws
        .iter()
        .map(|law| law.action_id_sha256.clone())
        .collect::<Vec<_>>();
    let programs = enumerate_programs_v1(&action_ids, request.maximum_depth as usize)?;
    if programs.len() != K2_COMPOSITION_PROGRAMS_PER_ROUTE_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "program_denominator_mismatch",
        ));
    }

    let mut candidates = Vec::with_capacity(programs.len());
    for program in programs {
        let disposition = predict_program_v1(request, &program)?;
        candidates.push(K2CompositionCandidateV1::seal(program, disposition)?);
    }
    candidates.sort_by(|left, right| {
        left.program
            .program_root_sha256
            .cmp(&right.program.program_root_sha256)
    });

    let valid_programs = candidates
        .iter()
        .filter(|candidate| {
            matches!(
                candidate.disposition,
                K2CompositionProgramDispositionV1::ValidPrediction { .. }
            )
        })
        .count() as u64;
    let inapplicable_programs = candidates.len() as u64 - valid_programs;
    let semantic_classes = semantic_quotient_v1(&candidates)?;
    let satisfying = semantic_classes
        .iter()
        .filter(|class| class.exact_goal_satisfied)
        .collect::<Vec<_>>();
    let [selected_class] = satisfying.as_slice() else {
        return Err(K2CompositionErrorV1::Invalid(
            "unique_satisfying_class_missing",
        ));
    };
    let minimum_satisfying_depth = selected_class.depth;
    let satisfying_strict_prefixes = candidates
        .iter()
        .filter(|candidate| {
            candidate.program.depth() < minimum_satisfying_depth
                && matches!(
                    candidate.disposition,
                    K2CompositionProgramDispositionV1::ValidPrediction {
                        exact_goal_satisfied: true,
                        ..
                    }
                )
        })
        .count() as u64;
    if minimum_satisfying_depth != request.maximum_depth || satisfying_strict_prefixes != 0 {
        return Err(K2CompositionErrorV1::Invalid(
            "minimum_composition_depth_invalid",
        ));
    }
    let representative_root = selected_class
        .member_program_roots_sha256
        .first()
        .ok_or(K2CompositionErrorV1::Invalid("satisfying_class_empty"))?;
    let selected_program = candidates
        .iter()
        .find(|candidate| &candidate.program.program_root_sha256 == representative_root)
        .map(|candidate| candidate.program.clone())
        .ok_or(K2CompositionErrorV1::Invalid("selected_program_missing"))?;
    let selected_class_root_sha256 = selected_class.class_root_sha256.clone();
    let dependency_edges = dependency_edges_v1(request)?;
    let normalized_topology_root_sha256 = normalized_topology_root_v1(request, &dependency_edges)?;

    let mut outcome = K2CompositionPlannerOutcomeV1 {
        schema: K2_COMPOSITION_PLANNER_OUTCOME_SCHEMA_V1.to_owned(),
        experiment_id_sha256: request.experiment_id_sha256.clone(),
        request_root_sha256: request.request_root_sha256.clone(),
        candidates,
        semantic_classes,
        dependency_edges,
        normalized_topology_root_sha256,
        selected_class_root_sha256,
        selected_program,
        valid_programs,
        inapplicable_programs,
        budget_rejected_programs: 0,
        minimum_satisfying_depth,
        satisfying_strict_prefixes,
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        outcome_root_sha256: String::new(),
    };
    outcome.reseal()?;
    Ok(outcome)
}

fn enumerate_programs_v1(
    action_ids: &[String],
    maximum_depth: usize,
) -> K2CompositionResultV1<Vec<K2CompositionProgramV1>> {
    let mut programs = Vec::new();
    let mut prefix = Vec::new();
    extend_programs_v1(action_ids, maximum_depth, &mut prefix, &mut programs)?;
    Ok(programs)
}

fn extend_programs_v1(
    action_ids: &[String],
    maximum_depth: usize,
    prefix: &mut Vec<String>,
    programs: &mut Vec<K2CompositionProgramV1>,
) -> K2CompositionResultV1<()> {
    if prefix.len() >= maximum_depth {
        return Ok(());
    }
    for action_id in action_ids {
        if prefix.contains(action_id) {
            continue;
        }
        prefix.push(action_id.clone());
        programs.push(K2CompositionProgramV1::seal(prefix.clone())?);
        extend_programs_v1(action_ids, maximum_depth, prefix, programs)?;
        prefix.pop();
    }
    Ok(())
}

fn predict_program_v1(
    request: &K2CompositionPlanningRequestV1,
    program: &K2CompositionProgramV1,
) -> K2CompositionResultV1<K2CompositionProgramDispositionV1> {
    let mut current = request.target.clone();
    for (step, action_id) in program.action_ids_sha256.iter().enumerate() {
        let law = request
            .law_set
            .law(action_id)
            .ok_or(K2CompositionErrorV1::Invalid("program_action_unknown"))?;
        match apply_planner_effect_v1(&current, &law.effect) {
            Ok(next) => current = next,
            Err(reason) => {
                return Ok(K2CompositionProgramDispositionV1::InapplicableAtStep {
                    step: step as u64,
                    reason: reason.to_owned(),
                });
            }
        }
    }
    let exact_goal_satisfied = current == request.goal.expected_terminal;
    Ok(K2CompositionProgramDispositionV1::ValidPrediction {
        terminal: current,
        exact_goal_satisfied,
    })
}

fn apply_planner_effect_v1(
    manifest: &K2CompositionTreeManifestV1,
    effect: &K2CompositionLearnedEffectV1,
) -> Result<K2CompositionTreeManifestV1, &'static str> {
    let mut entries = manifest
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
                .ok_or("copy_source_missing")?;
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
                return Err("remove_path_missing");
            }
        }
    }
    K2CompositionTreeManifestV1::seal_entries(entries.into_values().collect())
        .map_err(|_| "predicted_manifest_invalid")
}

fn semantic_quotient_v1(
    candidates: &[K2CompositionCandidateV1],
) -> K2CompositionResultV1<Vec<K2CompositionSemanticClassV1>> {
    let mut groups = BTreeMap::<(u64, Vec<String>, String), (Vec<String>, bool)>::new();
    for candidate in candidates {
        let K2CompositionProgramDispositionV1::ValidPrediction {
            terminal,
            exact_goal_satisfied,
        } = &candidate.disposition
        else {
            continue;
        };
        let mut multiset = candidate.program.action_ids_sha256.clone();
        multiset.sort();
        let key = (
            candidate.program.depth(),
            multiset,
            terminal.tree_root_sha256.clone(),
        );
        let value = groups
            .entry(key)
            .or_insert((Vec::new(), *exact_goal_satisfied));
        if value.1 != *exact_goal_satisfied {
            return Err(K2CompositionErrorV1::Invalid("quotient_goal_inconsistent"));
        }
        value.0.push(candidate.program.program_root_sha256.clone());
    }
    let mut classes = groups
        .into_iter()
        .map(|((depth, multiset, terminal_root), (members, satisfies))| {
            K2CompositionSemanticClassV1::seal(depth, multiset, terminal_root, members, satisfies)
        })
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    classes.sort_by(|left, right| left.class_root_sha256.cmp(&right.class_root_sha256));
    Ok(classes)
}

fn dependency_edges_v1(
    request: &K2CompositionPlanningRequestV1,
) -> K2CompositionResultV1<Vec<K2CompositionDependencyEdgeV1>> {
    let mut edges = Vec::new();
    for writer in &request.law_set.laws {
        for reader in &request.law_set.laws {
            if writer.action_id_sha256 == reader.action_id_sha256 {
                continue;
            }
            let write_paths = writer.effect.write_paths();
            let read_paths = reader.effect.read_paths();
            for path in write_paths.intersection(&read_paths) {
                edges.push(K2CompositionDependencyEdgeV1 {
                    writer_action_id_sha256: writer.action_id_sha256.clone(),
                    reader_action_id_sha256: reader.action_id_sha256.clone(),
                    path: path.clone(),
                });
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
    Ok(edges)
}

fn normalized_topology_root_v1(
    request: &K2CompositionPlanningRequestV1,
    edges: &[K2CompositionDependencyEdgeV1],
) -> K2CompositionResultV1<String> {
    let mut signatures = request
        .law_set
        .laws
        .iter()
        .map(|law| {
            let incoming = edges
                .iter()
                .filter(|edge| edge.reader_action_id_sha256 == law.action_id_sha256)
                .count() as u64;
            let outgoing = edges
                .iter()
                .filter(|edge| edge.writer_action_id_sha256 == law.action_id_sha256)
                .count() as u64;
            (incoming, outgoing)
        })
        .collect::<Vec<_>>();
    signatures.sort();
    composition_root_v1(&(
        "nando.k2-composition-source-neutral-topology.v1",
        signatures,
    ))
}

pub fn run_composition_planner_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_COMPOSITION_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_planner_stdin"))?;
    let request: K2CompositionPlanningRequestV1 = composition_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_planner_executable"))?;
    if composition_sha256_file_v1(&executable)? != request.planner_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid("planner_executable_mismatch"));
    }
    let outcome = plan_learned_composition_v1(&request)?;
    let output = composition_bytes_v1(&outcome)?;
    std::io::stdout()
        .write_all(&output)
        .map_err(|_| K2CompositionErrorV1::Io("write_planner_stdout"))
}
