use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};

use sha2::{Digest, Sha256};

use super::super::{
    K2CompositionErrorV1, K2CompositionFileEntryV1, K2CompositionLearnedEffectV1,
    K2CompositionResultV1, K2CompositionTreeManifestV1, composition_bytes_v1,
    composition_sha256_file_v1,
};
use super::{
    K2_UNCERTAINTY_ACTIONS_V1, K2_UNCERTAINTY_CONFIRM_CASES_V1,
    K2_UNCERTAINTY_GENERATOR_RESPONSE_SCHEMA_V1, K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1,
    K2_UNCERTAINTY_PATHS_V1, K2_UNCERTAINTY_PRIVATE_BATCH_SCHEMA_V1,
    K2_UNCERTAINTY_PRIVATE_CASE_SCHEMA_V1, K2_UNCERTAINTY_PUBLIC_BATCH_SCHEMA_V1,
    K2UncertaintyContentAtomV1, K2UncertaintyDomainVocabularyV1, K2UncertaintyGeneratorRequestV1,
    K2UncertaintyGeneratorResponseV1, K2UncertaintyPathAtomV1, K2UncertaintyPrivateBatchV1,
    K2UncertaintyPrivateCaseV1, K2UncertaintyPrivateMappingEntryV1, K2UncertaintyPublicBatchV1,
    K2UncertaintyPublicCaseV1, K2UncertaintySplitV1, K2UncertaintySupportObservationV1,
    K2UncertaintySupportOutcomeV1, K2UncertaintySupportSetV1, K2UncertaintyTopologyFamilyV1,
    K2UncertaintyTransitionReasonV1, denied_authority_v1, uncertainty_decode_v1,
    uncertainty_root_v1,
};

const GENERATOR_SCHEMA_V1: &str = "nando.k2-self-formed-deterministic-generator.v1";

pub fn generate_self_formed_development_batch_v1(
    request: &K2UncertaintyGeneratorRequestV1,
) -> K2CompositionResultV1<K2UncertaintyGeneratorResponseV1> {
    request.validate()?;
    let generator_schema_root_sha256 = uncertainty_root_v1(&(
        GENERATOR_SCHEMA_V1,
        &request.preregistration_v2_root_sha256,
        &request.preregistration_v3_root_sha256,
    ))?;
    let experiment_id_sha256 = uncertainty_root_v1(&(
        "nando.k2-self-formed-development-experiment.v1",
        &request.seed_commitment_sha256,
        &generator_schema_root_sha256,
    ))?;

    let mut public_cases = Vec::with_capacity(K2_UNCERTAINTY_CONFIRM_CASES_V1);
    let mut private_cases = Vec::with_capacity(K2_UNCERTAINTY_CONFIRM_CASES_V1);
    for case_index in 0..K2_UNCERTAINTY_CONFIRM_CASES_V1 {
        let generated = generate_case_v1(
            &request.seed_bytes,
            &experiment_id_sha256,
            &generator_schema_root_sha256,
            case_index,
        )?;
        public_cases.push(generated.0);
        private_cases.push(generated.1);
    }
    public_cases.sort_by_key(|case| {
        derivation_bytes_v1(
            &request.seed_bytes,
            "public-case-order",
            case.public_case_root_sha256.as_bytes(),
        )
    });
    private_cases.sort_by(|left, right| left.case_id_sha256.cmp(&right.case_id_sha256));

    let mut public = K2UncertaintyPublicBatchV1 {
        schema: K2_UNCERTAINTY_PUBLIC_BATCH_SCHEMA_V1.to_owned(),
        experiment_id_sha256: experiment_id_sha256.clone(),
        split_commitment_root_sha256: request.seed_commitment_sha256.clone(),
        cases: public_cases,
        authority: denied_authority_v1(),
        public_batch_root_sha256: String::new(),
    };
    public.reseal()?;
    for private_case in &mut private_cases {
        let public_case = public
            .cases
            .iter()
            .find(|case| case.vocabulary.case_id_sha256 == private_case.case_id_sha256)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_generator_public_case_missing",
            ))?;
        private_case.public_case_root_sha256 = public_case.public_case_root_sha256.clone();
        private_case.reseal()?;
    }
    let mut private = K2UncertaintyPrivateBatchV1 {
        schema: K2_UNCERTAINTY_PRIVATE_BATCH_SCHEMA_V1.to_owned(),
        experiment_id_sha256,
        public_batch_root_sha256: public.public_batch_root_sha256.clone(),
        cases: private_cases,
        expected_denominator_commitment_sha256: String::new(),
        private_batch_root_sha256: String::new(),
    };
    private.reseal()?;
    let mut response = K2UncertaintyGeneratorResponseV1 {
        schema: K2_UNCERTAINTY_GENERATOR_RESPONSE_SCHEMA_V1.to_owned(),
        generator_request_root_sha256: request.request_root_sha256.clone(),
        public,
        private,
        authority: denied_authority_v1(),
        response_root_sha256: String::new(),
    };
    response.reseal()?;
    Ok(response)
}

pub fn run_self_formed_generator_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_generator_stdin"))?;
    let request: K2UncertaintyGeneratorRequestV1 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_generator"))?;
    if composition_sha256_file_v1(&executable)? != request.generator_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_generator_executable_mismatch",
        ));
    }
    let response = generate_self_formed_development_batch_v1(&request)?;
    std::io::stdout()
        .write_all(&composition_bytes_v1(&response)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_generator_stdout"))
}

fn generate_case_v1(
    seed: &[u8],
    experiment_id_sha256: &str,
    generator_schema_root_sha256: &str,
    case_index: usize,
) -> K2CompositionResultV1<(K2UncertaintyPublicCaseV1, K2UncertaintyPrivateCaseV1)> {
    let case_context = format!("case-{case_index}");
    let case_id_sha256 = uncertainty_root_v1(&(
        "nando.k2-self-formed-development-case.v1",
        experiment_id_sha256,
        case_index,
        derivation_bytes_v1(seed, "case-id", case_context.as_bytes()),
    ))?;
    let mut path_ordinals = (0..K2_UNCERTAINTY_PATHS_V1).collect::<Vec<_>>();
    deterministic_permute_v1(seed, &format!("{case_context}-paths"), &mut path_ordinals);
    let paths = path_ordinals
        .iter()
        .enumerate()
        .map(|(ordinal, source)| {
            K2UncertaintyPathAtomV1::seal(
                ordinal as u8,
                format!("generated/case-{case_index}/p-{source}"),
            )
        })
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    let mut contents = (0..super::K2_UNCERTAINTY_CONTENTS_V1)
        .map(|source| {
            derivation_bytes_v1(
                seed,
                &format!("{case_context}-content-{source}"),
                experiment_id_sha256.as_bytes(),
            )
            .to_vec()
        })
        .collect::<Vec<_>>();
    deterministic_permute_v1(seed, &format!("{case_context}-contents"), &mut contents);
    let contents = contents
        .into_iter()
        .enumerate()
        .map(|(ordinal, bytes)| K2UncertaintyContentAtomV1::seal(ordinal as u8, bytes))
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    let mut actions = (0..K2_UNCERTAINTY_ACTIONS_V1)
        .map(|ordinal| {
            uncertainty_root_v1(&(
                "nando.k2-self-formed-opaque-action.v1",
                &case_id_sha256,
                ordinal,
                derivation_bytes_v1(
                    seed,
                    &format!("{case_context}-action-{ordinal}"),
                    experiment_id_sha256.as_bytes(),
                ),
            ))
        })
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    actions.sort();
    let vocabulary = K2UncertaintyDomainVocabularyV1::seal(
        experiment_id_sha256.to_owned(),
        case_id_sha256.clone(),
        K2UncertaintySplitV1::Development,
        generator_schema_root_sha256.to_owned(),
        actions.clone(),
        paths,
        contents,
    )?;
    let states = enumerate_states_v1(&vocabulary)?;
    let effects = enumerate_effects_v1(&vocabulary)?;
    let family = family_for_case_v1(case_index);
    let needed_sizes = match family {
        K2UncertaintyTopologyFamilyV1::U1SingleFour
        | K2UncertaintyTopologyFamilyV1::U3SingleFourCost => {
            [1_usize, 4].into_iter().collect::<BTreeSet<_>>()
        }
        K2UncertaintyTopologyFamilyV1::U2DoubleTwo
        | K2UncertaintyTopologyFamilyV1::U4DoubleTwoRisk => {
            [1_usize, 2].into_iter().collect::<BTreeSet<_>>()
        }
    };
    let mut witnesses = BTreeMap::new();
    for size in needed_sizes {
        let witness = find_support_witness_v1(
            seed,
            &format!("{case_context}-survivors-{size}"),
            &states,
            &effects,
            size,
        )?;
        witnesses.insert(size, witness);
    }

    let local_index = case_index % 4;
    let matched_pair = (local_index / 2) as u8;
    let pair_member = local_index % 2;
    let ambiguous_slots = ambiguous_action_slots_v1(family);
    let mut pending = Vec::with_capacity(super::K2_UNCERTAINTY_SUPPORT_ROWS_V1);
    let mut mapping = Vec::with_capacity(K2_UNCERTAINTY_ACTIONS_V1);
    for (action_slot, action_root) in actions.iter().enumerate() {
        let target_size = if ambiguous_slots.contains(&action_slot) {
            if ambiguous_slots.len() == 1 { 4 } else { 2 }
        } else {
            1
        };
        let witness = witnesses
            .get(&target_size)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_generator_support_witness_missing",
            ))?;
        let true_index = if target_size == 1 {
            0
        } else {
            (pair_member + usize::from(matched_pair) + action_slot) % target_size
        };
        let true_effect = effects[witness.effect_indices[true_index]].clone();
        mapping.push(K2UncertaintyPrivateMappingEntryV1 {
            opaque_action_root_sha256: action_root.clone(),
            effect: true_effect.clone(),
        });
        for state_index in witness.state_indices {
            let pre = states[state_index].clone();
            let outcome = apply_generated_effect_v1(&pre, &true_effect)?;
            pending.push(PendingSupportV1 {
                action_root_sha256: action_root.clone(),
                pre_manifest: pre,
                outcome,
            });
        }
    }
    pending.sort_by_key(|row| {
        derivation_bytes_v1(
            seed,
            &format!("{case_context}-support-order"),
            format!(
                "{}:{}:{}",
                row.action_root_sha256,
                row.pre_manifest.tree_root_sha256,
                row.outcome.outcome_root_sha256
            )
            .as_bytes(),
        )
    });
    let observations = pending
        .into_iter()
        .enumerate()
        .map(|(sequence, row)| {
            K2UncertaintySupportObservationV1::seal(
                case_id_sha256.clone(),
                sequence as u64,
                row.pre_manifest,
                row.action_root_sha256,
                row.outcome,
            )
        })
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    let support = K2UncertaintySupportSetV1::seal(
        case_id_sha256.clone(),
        vocabulary.vocabulary_root_sha256.clone(),
        observations,
    )?;
    let public = K2UncertaintyPublicCaseV1::seal(vocabulary, support)?;
    mapping.sort();
    let mut private = K2UncertaintyPrivateCaseV1 {
        schema: K2_UNCERTAINTY_PRIVATE_CASE_SCHEMA_V1.to_owned(),
        experiment_id_sha256: experiment_id_sha256.to_owned(),
        case_id_sha256,
        public_case_root_sha256: public.public_case_root_sha256.clone(),
        topology_family: family,
        matched_pair,
        mapping,
        expected_syntactic_model_count: 4,
        private_case_root_sha256: String::new(),
    };
    private.reseal()?;
    Ok((public, private))
}

fn family_for_case_v1(case_index: usize) -> K2UncertaintyTopologyFamilyV1 {
    match case_index / 4 {
        0 => K2UncertaintyTopologyFamilyV1::U1SingleFour,
        1 => K2UncertaintyTopologyFamilyV1::U2DoubleTwo,
        2 => K2UncertaintyTopologyFamilyV1::U3SingleFourCost,
        _ => K2UncertaintyTopologyFamilyV1::U4DoubleTwoRisk,
    }
}

fn ambiguous_action_slots_v1(family: K2UncertaintyTopologyFamilyV1) -> BTreeSet<usize> {
    match family {
        K2UncertaintyTopologyFamilyV1::U1SingleFour => BTreeSet::from([0]),
        K2UncertaintyTopologyFamilyV1::U2DoubleTwo => BTreeSet::from([0, 1]),
        K2UncertaintyTopologyFamilyV1::U3SingleFourCost => BTreeSet::from([2]),
        K2UncertaintyTopologyFamilyV1::U4DoubleTwoRisk => BTreeSet::from([2, 3]),
    }
}

struct PendingSupportV1 {
    action_root_sha256: String,
    pre_manifest: K2CompositionTreeManifestV1,
    outcome: K2UncertaintySupportOutcomeV1,
}

struct SupportWitnessV1 {
    effect_indices: Vec<usize>,
    state_indices: [usize; 3],
}

fn find_support_witness_v1(
    seed: &[u8],
    context: &str,
    states: &[K2CompositionTreeManifestV1],
    effects: &[K2CompositionLearnedEffectV1],
    target_size: usize,
) -> K2CompositionResultV1<SupportWitnessV1> {
    let mut pairs = Vec::new();
    for left in 0..states.len() {
        for right in left + 1..states.len() {
            pairs.push((left, right));
        }
    }
    let pair_start = derivation_index_v1(seed, context, b"pair", pairs.len());
    for pair_offset in 0..pairs.len() {
        let (left, right) = pairs[(pair_start + pair_offset) % pairs.len()];
        let mut groups = BTreeMap::<(String, String), Vec<usize>>::new();
        for (effect_index, effect) in effects.iter().enumerate() {
            let left_outcome = apply_generated_effect_v1(&states[left], effect)?;
            let right_outcome = apply_generated_effect_v1(&states[right], effect)?;
            groups
                .entry((
                    left_outcome.observable_outcome_root_sha256,
                    right_outcome.observable_outcome_root_sha256,
                ))
                .or_default()
                .push(effect_index);
        }
        let candidates = groups
            .into_values()
            .filter(|group| group.len() == target_size)
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            continue;
        }
        let group_start = derivation_index_v1(
            seed,
            context,
            format!("group-{left}-{right}").as_bytes(),
            candidates.len(),
        );
        for group_offset in 0..candidates.len() {
            let group = &candidates[(group_start + group_offset) % candidates.len()];
            let state_start = derivation_index_v1(
                seed,
                context,
                format!("third-{left}-{right}").as_bytes(),
                states.len(),
            );
            for state_offset in 0..states.len() {
                let third = (state_start + state_offset) % states.len();
                if third == left || third == right {
                    continue;
                }
                let mut outcomes = BTreeSet::new();
                for effect_index in group {
                    outcomes.insert(
                        apply_generated_effect_v1(&states[third], &effects[*effect_index])?
                            .observable_outcome_root_sha256,
                    );
                }
                if outcomes.len() == 1 {
                    return Ok(SupportWitnessV1 {
                        effect_indices: group.clone(),
                        state_indices: [left, right, third],
                    });
                }
            }
        }
    }
    Err(K2CompositionErrorV1::Invalid(
        "self_formed_generator_support_geometry_unavailable",
    ))
}

pub fn enumerate_self_formed_states_v1(
    vocabulary: &K2UncertaintyDomainVocabularyV1,
) -> K2CompositionResultV1<Vec<K2CompositionTreeManifestV1>> {
    enumerate_states_v1(vocabulary)
}

fn enumerate_states_v1(
    vocabulary: &K2UncertaintyDomainVocabularyV1,
) -> K2CompositionResultV1<Vec<K2CompositionTreeManifestV1>> {
    vocabulary.validate()?;
    let mut manifests = Vec::with_capacity(super::K2_UNCERTAINTY_STATE_COUNT_V1);
    for encoded in 0..super::K2_UNCERTAINTY_STATE_COUNT_V1 {
        let mut value = encoded;
        let mut entries = Vec::new();
        for path in &vocabulary.path_atoms {
            let state = value % 4;
            value /= 4;
            if state > 0 {
                let content = &vocabulary.content_atoms[state - 1];
                entries.push(K2CompositionFileEntryV1 {
                    path: path.path.clone(),
                    content_sha256: content.bytes_sha256.clone(),
                    byte_len: content.byte_len,
                });
            }
        }
        manifests.push(K2CompositionTreeManifestV1::seal_entries(entries)?);
    }
    manifests.sort_by(|left, right| left.tree_root_sha256.cmp(&right.tree_root_sha256));
    Ok(manifests)
}

pub fn enumerate_self_formed_effects_v1(
    vocabulary: &K2UncertaintyDomainVocabularyV1,
) -> K2CompositionResultV1<Vec<K2CompositionLearnedEffectV1>> {
    enumerate_effects_v1(vocabulary)
}

fn enumerate_effects_v1(
    vocabulary: &K2UncertaintyDomainVocabularyV1,
) -> K2CompositionResultV1<Vec<K2CompositionLearnedEffectV1>> {
    vocabulary.validate()?;
    let mut effects = Vec::with_capacity(super::K2_UNCERTAINTY_EFFECTS_PER_ACTION_V1);
    for source in &vocabulary.path_atoms {
        for target in &vocabulary.path_atoms {
            if source.path != target.path {
                effects.push(K2CompositionLearnedEffectV1::CopyFile {
                    source_path: source.path.clone(),
                    target_path: target.path.clone(),
                });
            }
        }
    }
    for path in &vocabulary.path_atoms {
        effects.push(K2CompositionLearnedEffectV1::RemoveFile {
            path: path.path.clone(),
        });
    }
    effects.sort();
    Ok(effects)
}

pub(crate) fn apply_generated_effect_v1(
    manifest: &K2CompositionTreeManifestV1,
    effect: &K2CompositionLearnedEffectV1,
) -> K2CompositionResultV1<K2UncertaintySupportOutcomeV1> {
    manifest.validate()?;
    effect.validate()?;
    let mut entries = manifest
        .entries
        .iter()
        .cloned()
        .map(|entry| (entry.path.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    let reason = match effect {
        K2CompositionLearnedEffectV1::CopyFile {
            source_path,
            target_path,
        } => match entries.get(source_path).cloned() {
            Some(mut source) => {
                source.path = target_path.clone();
                entries.insert(target_path.clone(), source);
                K2UncertaintyTransitionReasonV1::Applied
            }
            None => K2UncertaintyTransitionReasonV1::CopySourceMissing,
        },
        K2CompositionLearnedEffectV1::RemoveFile { path } => {
            if entries.remove(path).is_some() {
                K2UncertaintyTransitionReasonV1::Applied
            } else {
                K2UncertaintyTransitionReasonV1::RemovePathMissing
            }
        }
    };
    let post_manifest = K2CompositionTreeManifestV1::seal_entries(entries.into_values().collect())?;
    K2UncertaintySupportOutcomeV1::seal(reason, post_manifest)
}

fn deterministic_permute_v1<T>(seed: &[u8], context: &str, values: &mut [T]) {
    for cursor in (1..values.len()).rev() {
        let selected =
            derivation_index_v1(seed, context, &(cursor as u64).to_le_bytes(), cursor + 1);
        values.swap(cursor, selected);
    }
}

fn derivation_index_v1(seed: &[u8], context: &str, value: &[u8], denominator: usize) -> usize {
    let bytes = derivation_bytes_v1(seed, context, value);
    let mut prefix = [0_u8; 8];
    prefix.copy_from_slice(&bytes[..8]);
    (u64::from_le_bytes(prefix) as usize) % denominator
}

fn derivation_bytes_v1(seed: &[u8], context: &str, value: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"nando.k2-self-formed-derivation.v1\0");
    hash.update((seed.len() as u64).to_le_bytes());
    hash.update(seed);
    hash.update((context.len() as u64).to_le_bytes());
    hash.update(context.as_bytes());
    hash.update((value.len() as u64).to_le_bytes());
    hash.update(value);
    hash.finalize().into()
}
