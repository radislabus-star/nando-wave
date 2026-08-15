use std::collections::BTreeMap;

use nando_operator_learning::{
    K2CompositionFileEntryV1, K2CompositionLearnedEffectV1, K2CompositionTreeManifestV1,
    K2UncertaintyContentAtomV1, K2UncertaintyDomainVocabularyV1, K2UncertaintyLearnerRequestV1,
    K2UncertaintyLearnerResponseV1, K2UncertaintyPathAtomV1, K2UncertaintyPublicCaseV1,
    K2UncertaintySupportObservationV1, K2UncertaintySupportOutcomeV1, K2UncertaintySupportSetV1,
    learn_self_formed_uncertainty_v1,
};

use super::fixture::{R7Fixture, root_hash};
use super::ledger::ControlLedger;

pub fn run(fixture: &R7Fixture, ledger: &mut ControlLedger) {
    action_id_equivariance(fixture);
    ledger.pass("01", "opaque_action_id_permutation_equivariant");
    path_equivariance(fixture);
    ledger.pass("02", "path_bijection_equivariant");
    content_equivariance(fixture);
    ledger.pass("03", "content_token_bijection_equivariant");

    let mut shuffled = fixture.public_case.support.observations.clone();
    shuffled.reverse();
    let resealed = K2UncertaintySupportSetV1::seal(
        fixture.public_case.support.case_id_sha256.clone(),
        fixture.public_case.support.vocabulary_root_sha256.clone(),
        shuffled,
    )
    .expect("shuffle support reseal");
    assert_eq!(resealed, fixture.public_case.support);
    ledger.pass("04", "support_row_shuffle_invariant");

    let original_case_roots = fixture
        .generated
        .public
        .cases
        .iter()
        .map(|case| case.public_case_root_sha256.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let mut reordered = fixture.generated.public.clone();
    reordered.cases.reverse();
    reordered.reseal().expect("case-order reseal");
    assert_eq!(
        original_case_roots,
        reordered
            .cases
            .iter()
            .map(|case| case.public_case_root_sha256.clone())
            .collect()
    );
    ledger.pass("05", "case_order_shuffle_invariant");

    let mut pairs = BTreeMap::new();
    for private_case in &fixture.generated.private.cases {
        pairs
            .entry((private_case.topology_family, private_case.matched_pair))
            .or_insert_with(Vec::new)
            .push(private_case);
    }
    assert_eq!(pairs.len(), 8);
    for pair in pairs.values() {
        assert_eq!(pair.len(), 2);
        assert_ne!(pair[0].mapping, pair[1].mapping);
        let public = pair
            .iter()
            .map(|private| {
                fixture
                    .generated
                    .public
                    .cases
                    .iter()
                    .find(|case| case.vocabulary.case_id_sha256 == private.case_id_sha256)
                    .expect("matched public case")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            public[0].vocabulary.opaque_action_roots_sha256.len(),
            public[1].vocabulary.opaque_action_roots_sha256.len()
        );
        assert_eq!(
            public[0].vocabulary.path_atoms.len(),
            public[1].vocabulary.path_atoms.len()
        );
        assert_eq!(
            public[0].vocabulary.content_atoms.len(),
            public[1].vocabulary.content_atoms.len()
        );
        assert_eq!(
            public[0].support.observations.len(),
            public[1].support.observations.len()
        );
    }
    ledger.pass("06", "matched_geometry_private_truth_differs");
}

fn action_id_equivariance(fixture: &R7Fixture) {
    let old = &fixture.public_case.vocabulary.opaque_action_roots_sha256;
    let renamed = old
        .iter()
        .enumerate()
        .map(|(index, _)| root_hash(&format!("renamed-action-{index}")))
        .collect::<Vec<_>>();
    let map = old
        .iter()
        .cloned()
        .zip(renamed.iter().cloned())
        .collect::<BTreeMap<_, _>>();
    let vocabulary = reseal_vocabulary(
        fixture,
        renamed,
        fixture.public_case.vocabulary.path_atoms.clone(),
        fixture.public_case.vocabulary.content_atoms.clone(),
    );
    let observations = fixture
        .public_case
        .support
        .observations
        .iter()
        .map(|row| {
            K2UncertaintySupportObservationV1::seal(
                row.case_id_sha256.clone(),
                row.support_sequence,
                row.pre_manifest.clone(),
                map[&row.opaque_action_root_sha256].clone(),
                row.outcome.clone(),
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .expect("rename action observations");
    let transformed = case_from_parts(vocabulary, observations);
    let learned = learn_case(&transformed);
    let original = survivor_effects(&fixture.learned);
    let transformed_survivors = survivor_effects(&learned);
    for (old_action, new_action) in map {
        assert_eq!(original[&old_action], transformed_survivors[&new_action]);
    }
    assert_eq!(
        fixture.learned.model_set.semantic_classes.len(),
        learned.model_set.semantic_classes.len()
    );
}

fn path_equivariance(fixture: &R7Fixture) {
    let path_map = fixture
        .public_case
        .vocabulary
        .path_atoms
        .iter()
        .map(|atom| (atom.path.clone(), format!("renamed/p-{}", atom.ordinal)))
        .collect::<BTreeMap<_, _>>();
    let inverse = path_map
        .iter()
        .map(|(old, new)| (new.clone(), old.clone()))
        .collect::<BTreeMap<_, _>>();
    let paths = fixture
        .public_case
        .vocabulary
        .path_atoms
        .iter()
        .map(|atom| K2UncertaintyPathAtomV1::seal(atom.ordinal, path_map[&atom.path].clone()))
        .collect::<Result<Vec<_>, _>>()
        .expect("renamed path atoms");
    let vocabulary = reseal_vocabulary(
        fixture,
        fixture
            .public_case
            .vocabulary
            .opaque_action_roots_sha256
            .clone(),
        paths,
        fixture.public_case.vocabulary.content_atoms.clone(),
    );
    let observations = fixture
        .public_case
        .support
        .observations
        .iter()
        .map(|row| {
            K2UncertaintySupportObservationV1::seal(
                row.case_id_sha256.clone(),
                row.support_sequence,
                rename_manifest_paths(&row.pre_manifest, &path_map),
                row.opaque_action_root_sha256.clone(),
                K2UncertaintySupportOutcomeV1::seal(
                    row.outcome.transition_reason,
                    rename_manifest_paths(&row.outcome.post_manifest, &path_map),
                )?,
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .expect("rename path observations");
    let learned = learn_case(&case_from_parts(vocabulary, observations));
    let original = survivor_effects(&fixture.learned);
    let transformed = survivor_effects(&learned)
        .into_iter()
        .map(|(action, effects)| {
            let mut effects = effects
                .into_iter()
                .map(|effect| rename_effect_paths(effect, &inverse))
                .collect::<Vec<_>>();
            effects.sort();
            (action, effects)
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(original, transformed);
}

fn content_equivariance(fixture: &R7Fixture) {
    let contents = fixture
        .public_case
        .vocabulary
        .content_atoms
        .iter()
        .map(|atom| {
            K2UncertaintyContentAtomV1::seal(
                atom.ordinal,
                vec![0xA0_u8 + atom.ordinal; atom.byte_len as usize],
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .expect("renamed content atoms");
    let content_map = fixture
        .public_case
        .vocabulary
        .content_atoms
        .iter()
        .zip(&contents)
        .map(|(old, new)| (old.bytes_sha256.clone(), new.bytes_sha256.clone()))
        .collect::<BTreeMap<_, _>>();
    let vocabulary = reseal_vocabulary(
        fixture,
        fixture
            .public_case
            .vocabulary
            .opaque_action_roots_sha256
            .clone(),
        fixture.public_case.vocabulary.path_atoms.clone(),
        contents,
    );
    let observations = fixture
        .public_case
        .support
        .observations
        .iter()
        .map(|row| {
            K2UncertaintySupportObservationV1::seal(
                row.case_id_sha256.clone(),
                row.support_sequence,
                rename_manifest_contents(&row.pre_manifest, &content_map),
                row.opaque_action_root_sha256.clone(),
                K2UncertaintySupportOutcomeV1::seal(
                    row.outcome.transition_reason,
                    rename_manifest_contents(&row.outcome.post_manifest, &content_map),
                )?,
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .expect("rename content observations");
    let learned = learn_case(&case_from_parts(vocabulary, observations));
    assert_eq!(
        survivor_effects(&fixture.learned),
        survivor_effects(&learned)
    );
}

fn reseal_vocabulary(
    fixture: &R7Fixture,
    actions: Vec<String>,
    paths: Vec<K2UncertaintyPathAtomV1>,
    contents: Vec<K2UncertaintyContentAtomV1>,
) -> K2UncertaintyDomainVocabularyV1 {
    let source = &fixture.public_case.vocabulary;
    K2UncertaintyDomainVocabularyV1::seal(
        source.experiment_id_sha256.clone(),
        source.case_id_sha256.clone(),
        source.split,
        source.generator_schema_root_sha256.clone(),
        actions,
        paths,
        contents,
    )
    .expect("transformed vocabulary")
}

fn case_from_parts(
    vocabulary: K2UncertaintyDomainVocabularyV1,
    observations: Vec<K2UncertaintySupportObservationV1>,
) -> K2UncertaintyPublicCaseV1 {
    let support = K2UncertaintySupportSetV1::seal(
        vocabulary.case_id_sha256.clone(),
        vocabulary.vocabulary_root_sha256.clone(),
        observations,
    )
    .expect("transformed support");
    K2UncertaintyPublicCaseV1::seal(vocabulary, support).expect("transformed case")
}

fn learn_case(case: &K2UncertaintyPublicCaseV1) -> K2UncertaintyLearnerResponseV1 {
    learn_self_formed_uncertainty_v1(
        &K2UncertaintyLearnerRequestV1::seal(
            case.vocabulary.clone(),
            case.support.clone(),
            root_hash("transformed-learner"),
        )
        .expect("transformed learner request"),
    )
    .expect("transformed learner response")
}

fn survivor_effects(
    learned: &K2UncertaintyLearnerResponseV1,
) -> BTreeMap<String, Vec<K2CompositionLearnedEffectV1>> {
    learned
        .model_set
        .action_survivors
        .iter()
        .map(|survivors| {
            (
                survivors.opaque_action_root_sha256.clone(),
                survivors
                    .effects
                    .iter()
                    .map(|candidate| candidate.effect.clone())
                    .collect(),
            )
        })
        .collect()
}

fn rename_manifest_paths(
    manifest: &K2CompositionTreeManifestV1,
    paths: &BTreeMap<String, String>,
) -> K2CompositionTreeManifestV1 {
    K2CompositionTreeManifestV1::seal_entries(
        manifest
            .entries
            .iter()
            .map(|entry| K2CompositionFileEntryV1 {
                path: paths[&entry.path].clone(),
                content_sha256: entry.content_sha256.clone(),
                byte_len: entry.byte_len,
            })
            .collect(),
    )
    .expect("rename manifest paths")
}

fn rename_manifest_contents(
    manifest: &K2CompositionTreeManifestV1,
    contents: &BTreeMap<String, String>,
) -> K2CompositionTreeManifestV1 {
    K2CompositionTreeManifestV1::seal_entries(
        manifest
            .entries
            .iter()
            .map(|entry| K2CompositionFileEntryV1 {
                path: entry.path.clone(),
                content_sha256: contents[&entry.content_sha256].clone(),
                byte_len: entry.byte_len,
            })
            .collect(),
    )
    .expect("rename manifest contents")
}

fn rename_effect_paths(
    effect: K2CompositionLearnedEffectV1,
    paths: &BTreeMap<String, String>,
) -> K2CompositionLearnedEffectV1 {
    match effect {
        K2CompositionLearnedEffectV1::CopyFile {
            source_path,
            target_path,
        } => K2CompositionLearnedEffectV1::CopyFile {
            source_path: paths[&source_path].clone(),
            target_path: paths[&target_path].clone(),
        },
        K2CompositionLearnedEffectV1::RemoveFile { path } => {
            K2CompositionLearnedEffectV1::RemoveFile {
                path: paths[&path].clone(),
            }
        }
    }
}
