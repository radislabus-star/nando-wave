use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionFileEntryV1,
    K2CompositionLearnedEffectV1, K2CompositionResultV1, K2CompositionTreeManifestV1,
    composition_root_v1,
};
use super::model::{
    K2_REPRESENTATION_BASELINE_OUTCOME_SCHEMA_V1, K2_REPRESENTATION_COMPLETE_PROGRAMS_V1,
    K2_REPRESENTATION_MAX_DEPTH_V1, K2_REPRESENTATION_MAX_PROTOCOL_BYTES_V1,
    K2RepresentationBaselineOutcomeV1, K2RepresentationBaselineRequestV1,
    K2RepresentationDecisionGroupV1, K2RepresentationProgramV1, K2RepresentationTaskV1,
    K2RepresentationTrainingCorpusV1, K2RepresentationTrainingRowV1, extract_policy_features_v1,
    representation_bytes_v1, representation_decode_v1, representation_executable_matches_v1,
};

pub fn complete_representation_baseline_v1(
    request: &K2RepresentationBaselineRequestV1,
) -> K2CompositionResultV1<K2RepresentationBaselineOutcomeV1> {
    request.validate()?;
    let action_ids = request
        .task
        .laws
        .iter()
        .map(|law| law.action_id_sha256.clone())
        .collect::<Vec<_>>();
    let programs = enumerate_all_programs_v1(&action_ids)?;
    if programs.len() as u64 != K2_REPRESENTATION_COMPLETE_PROGRAMS_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "representation_baseline_denominator_mismatch",
        ));
    }

    let mut candidate_roots = Vec::with_capacity(programs.len());
    let mut satisfying = Vec::new();
    let mut valid_programs = 0_u64;
    let mut inapplicable_programs = 0_u64;
    for program in programs {
        match baseline_predict_program_v1(&request.task, &program) {
            Ok(terminal) => {
                valid_programs += 1;
                let exact = terminal == request.task.goal.expected_terminal;
                candidate_roots.push(composition_root_v1(&(
                    "nando.k2-representation-baseline-candidate.v1",
                    &program,
                    "valid",
                    &terminal.tree_root_sha256,
                    exact,
                ))?);
                if exact {
                    satisfying.push(program);
                }
            }
            Err((step, reason)) => {
                inapplicable_programs += 1;
                candidate_roots.push(composition_root_v1(&(
                    "nando.k2-representation-baseline-candidate.v1",
                    &program,
                    "inapplicable",
                    step,
                    reason,
                ))?);
            }
        }
    }
    if valid_programs + inapplicable_programs != K2_REPRESENTATION_COMPLETE_PROGRAMS_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "representation_baseline_partition_mismatch",
        ));
    }
    let minimum_satisfying_depth = satisfying
        .iter()
        .map(K2RepresentationProgramV1::depth)
        .min()
        .ok_or(K2CompositionErrorV1::Invalid(
            "representation_baseline_goal_unreachable",
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
        task_root_sha256: request.task.task_root_sha256.clone(),
        request_root_sha256: request.request_root_sha256.clone(),
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

pub fn project_representation_training_corpus_v1(
    tasks: &[K2RepresentationTaskV1],
    baselines: &[K2RepresentationBaselineOutcomeV1],
) -> K2CompositionResultV1<K2RepresentationTrainingCorpusV1> {
    if tasks.len() != baselines.len() {
        return Err(K2CompositionErrorV1::Invalid(
            "representation_training_projection_count_mismatch",
        ));
    }
    let mut all_groups = Vec::new();
    let mut baseline_roots = Vec::new();
    for (task, baseline) in tasks.iter().zip(baselines) {
        if task.task_root_sha256 != baseline.task_root_sha256
            || baseline.complete_programs != K2_REPRESENTATION_COMPLETE_PROGRAMS_V1
            || baseline.minimum_satisfying_programs.is_empty()
        {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_training_baseline_invalid",
            ));
        }
        baseline_roots.push(baseline.outcome_root_sha256.clone());
        let mut prefixes = BTreeMap::<String, Vec<String>>::new();
        for program in &baseline.minimum_satisfying_programs {
            for prefix_len in 0..program.action_ids_sha256.len() {
                let prefix = program.action_ids_sha256[..prefix_len].to_vec();
                let root = composition_root_v1(&(
                    "nando.k2-representation-train-prefix.v1",
                    &task.task_root_sha256,
                    &prefix,
                ))?;
                prefixes.entry(root).or_insert(prefix);
            }
        }
        for prefix in prefixes.values() {
            let current = replay_prefix_v1(task, prefix)?;
            let positive_actions = baseline
                .minimum_satisfying_programs
                .iter()
                .filter(|program| program.action_ids_sha256.starts_with(prefix))
                .filter_map(|program| program.action_ids_sha256.get(prefix.len()))
                .cloned()
                .collect::<BTreeSet<_>>();
            let used = prefix.iter().collect::<BTreeSet<_>>();
            let mut rows = Vec::new();
            for law in &task.laws {
                if used.contains(&law.action_id_sha256) {
                    continue;
                }
                let features =
                    extract_policy_features_v1(task, &current, prefix, &law.action_id_sha256)?;
                rows.push(K2RepresentationTrainingRowV1::seal(
                    features,
                    positive_actions.contains(&law.action_id_sha256),
                )?);
            }
            if rows.iter().any(|row| row.positive_continuation)
                && rows.iter().any(|row| !row.positive_continuation)
            {
                all_groups.push(K2RepresentationDecisionGroupV1::seal(rows)?);
            }
        }
    }
    K2RepresentationTrainingCorpusV1::seal(baseline_roots, all_groups)
}

pub fn run_representation_baseline_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_REPRESENTATION_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_representation_baseline_stdin"))?;
    let request: K2RepresentationBaselineRequestV1 = representation_decode_v1(&input)?;
    representation_executable_matches_v1(&request.baseline_executable_sha256)?;
    let outcome = complete_representation_baseline_v1(&request)?;
    std::io::stdout()
        .write_all(&representation_bytes_v1(&outcome)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_representation_baseline_stdout"))
}

fn enumerate_all_programs_v1(
    action_ids: &[String],
) -> K2CompositionResultV1<Vec<K2RepresentationProgramV1>> {
    let mut programs = Vec::new();
    let mut prefix = Vec::new();
    extend_programs_v1(action_ids, &mut prefix, &mut programs)?;
    Ok(programs)
}

fn extend_programs_v1(
    action_ids: &[String],
    prefix: &mut Vec<String>,
    programs: &mut Vec<K2RepresentationProgramV1>,
) -> K2CompositionResultV1<()> {
    if prefix.len() >= K2_REPRESENTATION_MAX_DEPTH_V1 as usize {
        return Ok(());
    }
    for action_id in action_ids {
        if prefix.contains(action_id) {
            continue;
        }
        prefix.push(action_id.clone());
        programs.push(K2RepresentationProgramV1::seal(prefix.clone())?);
        extend_programs_v1(action_ids, prefix, programs)?;
        prefix.pop();
    }
    Ok(())
}

fn baseline_predict_program_v1(
    task: &K2RepresentationTaskV1,
    program: &K2RepresentationProgramV1,
) -> Result<K2CompositionTreeManifestV1, (u64, &'static str)> {
    let mut current = task.initial.clone();
    for (step, action_id) in program.action_ids_sha256.iter().enumerate() {
        let law = task
            .law(action_id)
            .ok_or((step as u64, "baseline_action_unknown"))?;
        current = baseline_apply_effect_v1(&current, &law.effect)
            .map_err(|reason| (step as u64, reason))?;
    }
    Ok(current)
}

fn baseline_apply_effect_v1(
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

fn replay_prefix_v1(
    task: &K2RepresentationTaskV1,
    prefix: &[String],
) -> K2CompositionResultV1<K2CompositionTreeManifestV1> {
    let mut current = task.initial.clone();
    for action_id in prefix {
        let law = task.law(action_id).ok_or(K2CompositionErrorV1::Invalid(
            "representation_prefix_action_unknown",
        ))?;
        current = baseline_apply_effect_v1(&current, &law.effect).map_err(|_| {
            K2CompositionErrorV1::Invalid("representation_positive_prefix_inapplicable")
        })?;
    }
    Ok(current)
}
