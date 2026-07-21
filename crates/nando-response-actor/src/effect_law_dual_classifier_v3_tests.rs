use std::collections::BTreeMap;

use super::super::tests::{FixtureSpec, SealedFixtureSet, sealed_set};
use super::*;
use crate::{
    CollectionOutputRenderer, ProjectStatusMapping, canonical_json_bytes,
    teacher_semantic_law_signature,
};

fn rows(sealed: &SealedFixtureSet) -> Vec<EffectLawDualClassificationRowV3> {
    sealed
        .transitions
        .iter()
        .cloned()
        .zip(sealed.observations.iter().cloned())
        .map(|(transition, observation)| {
            EffectLawDualClassificationRowV3::new(transition, observation)
        })
        .collect()
}

fn classify(
    sealed: &SealedFixtureSet,
    rows: &[EffectLawDualClassificationRowV3],
) -> EffectLawDualClassificationReportV3 {
    let dictionary = EffectLawDictionaryV3::builtin().expect("dictionary");
    let hypothesis = EffectQuotientHypothesisV3::physical_adapters_only().expect("hypothesis");
    EffectLawDualClassifierV3::new(&sealed.trusted, &dictionary, &hypothesis)
        .classify(rows)
        .expect("dual classification")
}

fn paired_specs(first_seed: u64, configure: impl Fn(&mut FixtureSpec)) -> Vec<FixtureSpec> {
    let mut direct = FixtureSpec::direct(first_seed);
    let mut wrapped = FixtureSpec::wrapped(first_seed + 1);
    configure(&mut direct);
    configure(&mut wrapped);
    vec![direct, wrapped]
}

fn v3_law_count(report: &EffectLawDualClassificationReportV3) -> usize {
    report.v3_law_to_legacy_v1_cohorts.len()
}

fn stop_f3_specs() -> Vec<FixtureSpec> {
    let mut specs = paired_specs(1, |_| {});
    specs.extend(paired_specs(3, |spec| spec.yield_time_ms = 2_000));
    specs.extend(paired_specs(5, |spec| spec.terminate = true));
    specs.extend(paired_specs(7, |spec| {
        spec.renderer = CollectionOutputRenderer::RenderTemplate {
            prefix: "[".to_owned(),
            suffix: "]".to_owned(),
        };
    }));
    specs.extend(paired_specs(9, |spec| {
        spec.temporal_successor
            .clone_from(&spec.temporal_predecessor);
    }));
    specs.extend(paired_specs(11, |spec| spec.extra_preserved_role = true));
    specs
}

#[test]
fn renamed_and_wrapped_physical_surfaces_share_one_v3_law() {
    let sealed = sealed_set(&paired_specs(1, |_| {}));
    let report = classify(&sealed, &rows(&sealed));

    assert_eq!(report.denominator, 2);
    assert_eq!(report.v3_classified_rows, 2);
    assert_eq!(v3_law_count(&report), 1);
    assert_eq!(report.three_dimensional_independence.len(), 1);
    assert_eq!(report.three_dimensional_independence[0].observations, 2);
    assert_eq!(report.three_dimensional_independence[0].episode_lineages, 2);
    assert_eq!(
        report.three_dimensional_independence[0].independent_surfaces,
        2
    );
    assert_eq!(
        report.three_dimensional_independence[0].physical_programs,
        2
    );
}

#[test]
fn different_typed_constants_split_v3_and_explain_the_v1_split() {
    let mut specs = paired_specs(1, |_| {});
    specs.extend(paired_specs(3, |spec| spec.yield_time_ms = 2_000));
    let sealed = sealed_set(&specs);
    let report = classify(&sealed, &rows(&sealed));

    assert_eq!(v3_law_count(&report), 2);
    assert!(report.effect_significant_splits.iter().any(|split| {
        split.pairwise_witnesses.iter().any(|witness| {
            witness
                .reasons
                .contains(&EffectLawDualClassificationReasonV3::TypedConstants)
        })
    }));
    assert!(report.unexplained_splits.is_empty());
}

#[test]
fn continue_and_terminate_are_distinct_v3_laws() {
    let mut specs = paired_specs(1, |_| {});
    specs.extend(paired_specs(3, |spec| spec.terminate = true));
    let sealed = sealed_set(&specs);
    let report = classify(&sealed, &rows(&sealed));

    assert_eq!(v3_law_count(&report), 2);
    assert!(report.unexplained_merges.is_empty());
    assert!(report.unexplained_splits.is_empty());
}

#[test]
fn renderer_status_and_temporal_changes_split_v3() {
    let mut specs = paired_specs(1, |_| {});
    specs.extend(paired_specs(3, |spec| {
        spec.renderer = CollectionOutputRenderer::RenderTemplate {
            prefix: "[".to_owned(),
            suffix: "]".to_owned(),
        };
    }));
    specs.extend(paired_specs(5, |spec| {
        spec.status_mapping = ProjectStatusMapping::ZeroIsPass;
    }));
    specs.extend(paired_specs(7, |spec| {
        spec.temporal_successor
            .clone_from(&spec.temporal_predecessor);
    }));
    let sealed = sealed_set(&specs);
    let report = classify(&sealed, &rows(&sealed));

    assert_eq!(v3_law_count(&report), 4);
    assert!(report.effect_significant_splits.iter().any(|split| {
        split.pairwise_witnesses.iter().any(|witness| {
            witness
                .reasons
                .contains(&EffectLawDualClassificationReasonV3::TemporalCardinality)
        })
    }));
    assert!(report.unexplained_splits.is_empty());
}

#[test]
fn preserved_frame_change_splits_v3() {
    let mut specs = paired_specs(1, |_| {});
    specs.extend(paired_specs(3, |spec| spec.extra_preserved_role = true));
    let sealed = sealed_set(&specs);
    let report = classify(&sealed, &rows(&sealed));

    assert_eq!(v3_law_count(&report), 2);
    assert!(report.unexplained_merges.is_empty());
    assert!(report.unexplained_splits.is_empty());
}

#[test]
fn row_order_shuffle_is_byte_identical() {
    let sealed = sealed_set(&stop_f3_specs());
    let ordered = rows(&sealed);
    let mut shuffled = ordered.clone();
    shuffled.reverse();

    let ordered_report = classify(&sealed, &ordered);
    let shuffled_report = classify(&sealed, &shuffled);
    assert_eq!(
        ordered_report.canonical_bytes().expect("ordered bytes"),
        shuffled_report.canonical_bytes().expect("shuffled bytes")
    );
}

#[test]
fn foreign_and_tampered_trust_are_rejected_and_accounted() {
    let trusted_set = sealed_set(&paired_specs(1, |_| {}));
    let foreign_set = sealed_set(&paired_specs(3, |_| {}));
    let foreign_report = classify(&trusted_set, &rows(&foreign_set));
    assert_eq!(foreign_report.trust_failures.len(), 2);
    assert_eq!(foreign_report.accounted_rows, foreign_report.denominator);
    assert_eq!(foreign_report.legacy_v1_attempted_rows, 0);
    assert_eq!(foreign_report.v3_attempted_rows, 0);
    assert_eq!(
        foreign_report.verdict,
        EffectLawDualClassificationVerdictV3::Watch
    );

    let mut tampered_rows = rows(&trusted_set);
    tampered_rows[0].observation.transition_sha256 = crate::sha256_bytes(b"tampered");
    let tampered_report = classify(&trusted_set, &tampered_rows);
    assert_eq!(tampered_report.trust_failures.len(), 1);
    assert_eq!(tampered_report.trusted_rows, 1);
}

#[test]
fn same_trusted_input_denominator_reaches_v1_and_v3_once() {
    let sealed = sealed_set(&stop_f3_specs());
    let input = rows(&sealed);
    let report = classify(&sealed, &input);

    assert_eq!(report.denominator, input.len());
    assert_eq!(report.accounted_rows, input.len());
    assert_eq!(report.trusted_rows, input.len());
    assert_eq!(report.legacy_v1_attempted_rows, input.len());
    assert_eq!(report.v3_attempted_rows, input.len());
    assert_eq!(report.legacy_v1_classified_rows, input.len());
    assert_eq!(report.v3_classified_rows, input.len());
    assert!(report.unknown_censored_rows.is_empty());
    assert!(report.trust_failures.is_empty());
    assert_eq!(report.pairwise_discrepancies_expected, 6);
    assert_eq!(report.pairwise_discrepancies_accounted, 6);
    assert!(!report.execution_authority);

    let mut expected = input
        .iter()
        .map(|row| {
            let transition_sha256 =
                evidence::sha256_serialized(&row.transition).expect("transition digest");
            let row_sha256 = evidence::sha256_serialized(&(
                "nando.effect-law-dual-classification-row.v1-v3",
                transition_sha256.as_str(),
                row.observation.observation_sha256.as_str(),
            ))
            .expect("row digest");
            let signature =
                teacher_semantic_law_signature(&row.transition.as_training_relation_frame());
            (row_sha256, signature)
        })
        .collect::<Vec<_>>();
    expected.sort_by(|left, right| left.0.cmp(&right.0));
    let expected_refs = expected
        .iter()
        .map(|(row, signature)| (row.as_str(), signature.as_deref()))
        .collect::<Vec<_>>();
    assert_eq!(
        report.legacy_v1_route_root_sha256,
        evidence::sha256_serialized(&expected_refs).expect("legacy route root")
    );
}

#[test]
fn stop_f3_fixture_report_is_gate_clean() {
    let sealed = sealed_set(&stop_f3_specs());
    let report = classify(&sealed, &rows(&sealed));

    assert_eq!(report.verdict, EffectLawDualClassificationVerdictV3::Pass);
    assert_eq!(report.denominator, 12);
    assert_eq!(report.accounted_rows, 12);
    assert!(report.unexplained_merges.is_empty());
    assert!(report.unexplained_splits.is_empty());
    assert_eq!(report.pairwise_discrepancies_expected, 6);
    assert_eq!(report.pairwise_discrepancies_accounted, 6);
    assert!(!report.execution_authority);
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("STOP-F3 report JSON")
    );
}

fn synthetic_classified_facets(
    legacy_v1_signature_sha256: &str,
    protocol_facet_root_sha256: &str,
    effect_facet_root_sha256: &str,
) -> ClassifiedLawFacets {
    ClassifiedLawFacets {
        legacy_v1_signature_sha256: legacy_v1_signature_sha256.to_owned(),
        facets: LawFacets {
            grouping_key_sha256: "group".to_owned(),
            protocol_facet_root_sha256: protocol_facet_root_sha256.to_owned(),
            physical_program_id_sha256: "program".to_owned(),
            independent_surface_root_sha256: protocol_facet_root_sha256.to_owned(),
            episode_lineage_sha256: legacy_v1_signature_sha256.to_owned(),
            effect_invariant_root_sha256: effect_facet_root_sha256.to_owned(),
            typed_constants_root_sha256: "constants".to_owned(),
            completion_status_renderer_root_sha256: "completion".to_owned(),
            temporal_cardinality_root_sha256: "temporal".to_owned(),
            preserved_frame_root_sha256: "preserved".to_owned(),
            effect_facet_root_sha256: effect_facet_root_sha256.to_owned(),
        },
    }
}

#[test]
fn aggregate_protocol_roots_do_not_explain_every_v1_pair() {
    let law_id = "shared-v3-law".to_owned();
    let facets_by_law = BTreeMap::from([(
        law_id.clone(),
        vec![
            synthetic_classified_facets("v1-a", "protocol-1", "effect"),
            synthetic_classified_facets("v1-b", "protocol-1", "effect"),
            synthetic_classified_facets("v1-c", "protocol-2", "effect"),
        ],
    )]);
    let signatures = vec!["v1-a".to_owned(), "v1-b".to_owned(), "v1-c".to_owned()];
    let witnesses = merge_pairwise_witnesses(&law_id, &signatures, &facets_by_law)
        .expect("pairwise merge witnesses");

    assert_eq!(witnesses.len(), 3);
    let same_protocol_pair = witnesses
        .iter()
        .find(|witness| witness.left_class_sha256 == "v1-a" && witness.right_class_sha256 == "v1-b")
        .expect("A/B witness");
    assert!(same_protocol_pair.effect_facets_identical);
    assert!(!same_protocol_pair.protocol_facets_distinct);
    assert!(!same_protocol_pair.explained);
    assert!(witnesses.iter().all(|witness| !witness.explained));
}

#[test]
fn protocol_only_merge_candidate_remains_watch_without_label_free_fixture() {
    assert!(!PROTOCOL_ONLY_MERGE_FIXTURE_PROVEN_V3);
    let law_id = "shared-v3-law".to_owned();
    let facets_by_law = BTreeMap::from([(
        law_id.clone(),
        vec![
            synthetic_classified_facets("v1-a", "protocol-1", "effect"),
            synthetic_classified_facets("v1-b", "protocol-2", "effect"),
        ],
    )]);
    let signatures = vec!["v1-a".to_owned(), "v1-b".to_owned()];
    let witnesses = merge_pairwise_witnesses(&law_id, &signatures, &facets_by_law)
        .expect("pairwise merge witness");

    assert_eq!(witnesses.len(), 1);
    assert!(witnesses[0].effect_facets_identical);
    assert!(witnesses[0].protocol_facets_distinct);
    assert!(
        witnesses[0]
            .reasons
            .contains(&EffectLawDualClassificationReasonV3::ProtocolFacet)
    );
    assert!(!witnesses[0].supporting_fixture_proven);
    assert!(!witnesses[0].explained);
}

#[test]
fn checked_in_stop_f3_json_matches_generated_canonical_report() {
    let sealed = sealed_set(&stop_f3_specs());
    let report = classify(&sealed, &rows(&sealed));
    let generated =
        canonical_json_bytes(&serde_json::to_value(report).expect("generated report value"))
            .expect("generated canonical report");
    let checked_in_value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../plans/effect-law-unification-v1/STOP_F3_DUAL_CLASSIFICATION_V1_V3.json"
    ))
    .expect("checked-in STOP-F3 JSON");
    let checked_in = canonical_json_bytes(&checked_in_value).expect("checked-in canonical report");

    assert_eq!(generated, checked_in);
}
