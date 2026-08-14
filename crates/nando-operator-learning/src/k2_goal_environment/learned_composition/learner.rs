use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};

use super::model::{
    K2_COMPOSITION_MAX_PROTOCOL_BYTES_V1, K2_COMPOSITION_SUPPORT_WORLDS_PER_ACTION_V1,
    K2CompositionErrorV1, K2CompositionFileEntryV1, K2CompositionLearnedEffectV1,
    K2CompositionLearnedLawSetV1, K2CompositionLearnedLawV1, K2CompositionLearningRequestV1,
    K2CompositionResultV1, K2CompositionSupportObservationV1, K2CompositionTreeManifestV1,
    composition_bytes_v1, composition_decode_v1, composition_sha256_file_v1,
};

pub fn learn_composition_effects_v1(
    request: &K2CompositionLearningRequestV1,
) -> K2CompositionResultV1<K2CompositionLearnedLawSetV1> {
    request.validate()?;
    let mut grouped = BTreeMap::<String, Vec<&K2CompositionSupportObservationV1>>::new();
    for observation in &request.observations {
        grouped
            .entry(observation.action_id_sha256.clone())
            .or_default()
            .push(observation);
    }
    let mut laws = Vec::with_capacity(grouped.len());
    for (action_id, observations) in grouped {
        if observations.len() != K2_COMPOSITION_SUPPORT_WORLDS_PER_ACTION_V1 {
            return Err(K2CompositionErrorV1::Invalid("insufficient_support"));
        }
        if observations
            .iter()
            .all(|observation| observation.before == observation.after)
        {
            return Err(K2CompositionErrorV1::Invalid("no_identifiable_effect"));
        }
        let candidates = enumerate_effect_candidates_v1(&observations)?;
        let survivors = candidates
            .iter()
            .filter(|candidate| {
                observations.iter().all(|observation| {
                    apply_learner_effect_v1(&observation.before, candidate)
                        .is_ok_and(|predicted| predicted == observation.after)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        let effect = match survivors.as_slice() {
            [effect] => effect.clone(),
            [] => return Err(K2CompositionErrorV1::Invalid("no_identifiable_effect")),
            _ => return Err(K2CompositionErrorV1::Invalid("ambiguous_effect")),
        };
        let observation_roots = observations
            .iter()
            .map(|observation| observation.observation_root_sha256.clone())
            .collect();
        laws.push(K2CompositionLearnedLawV1::seal(
            action_id,
            effect,
            observation_roots,
            candidates.len() as u64,
            candidates.len().saturating_sub(1) as u64,
        )?);
    }
    K2CompositionLearnedLawSetV1::seal(
        request.experiment_id_sha256.clone(),
        request.request_root_sha256.clone(),
        laws,
    )
}

fn enumerate_effect_candidates_v1(
    observations: &[&K2CompositionSupportObservationV1],
) -> K2CompositionResultV1<Vec<K2CompositionLearnedEffectV1>> {
    let paths = observations
        .iter()
        .flat_map(|observation| {
            observation
                .before
                .entries
                .iter()
                .chain(&observation.after.entries)
                .map(|entry| entry.path.clone())
        })
        .collect::<BTreeSet<_>>();
    if paths.is_empty() || paths.len() > 24 {
        return Err(K2CompositionErrorV1::Invalid(
            "effect_candidate_path_budget",
        ));
    }
    let mut candidates = paths
        .iter()
        .cloned()
        .map(|path| K2CompositionLearnedEffectV1::RemoveFile { path })
        .collect::<Vec<_>>();
    for source_path in &paths {
        for target_path in &paths {
            if source_path != target_path {
                candidates.push(K2CompositionLearnedEffectV1::CopyFile {
                    source_path: source_path.clone(),
                    target_path: target_path.clone(),
                });
            }
        }
    }
    candidates.sort();
    candidates.dedup();
    if candidates.len() > 576 {
        return Err(K2CompositionErrorV1::Invalid("effect_candidate_budget"));
    }
    Ok(candidates)
}

fn apply_learner_effect_v1(
    manifest: &K2CompositionTreeManifestV1,
    effect: &K2CompositionLearnedEffectV1,
) -> K2CompositionResultV1<K2CompositionTreeManifestV1> {
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
                .ok_or(K2CompositionErrorV1::Invalid("learner_copy_source_missing"))?;
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
                return Err(K2CompositionErrorV1::Invalid("learner_remove_path_missing"));
            }
        }
    }
    K2CompositionTreeManifestV1::seal_entries(entries.into_values().collect())
}

pub fn run_composition_effect_learner_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_COMPOSITION_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_learner_stdin"))?;
    let request: K2CompositionLearningRequestV1 = composition_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_learner_executable"))?;
    if composition_sha256_file_v1(&executable)? != request.learner_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid("learner_executable_mismatch"));
    }
    let outcome = learn_composition_effects_v1(&request)?;
    let output = composition_bytes_v1(&outcome)?;
    std::io::stdout()
        .write_all(&output)
        .map_err(|_| K2CompositionErrorV1::Io("write_learner_stdout"))
}
