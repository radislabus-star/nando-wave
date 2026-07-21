use super::*;
use crate::protocol_mode::{BoundedProtocolModeCandidateV2, compile_protocol_modes_v2};
use crate::{
    BindingCompletionStateV1, BindingEvidenceBudgetV1, BindingPredicateV1,
    BindingProtocolCompileVerdictV2, BindingProtocolCompilerErrorV2, BindingRequestRelationV1,
    BindingValueTypeV1, CanonicalEffectLawV3, FrozenCandidateRelationGraphV1,
    PreActionBindingContextV1, PreActionBindingSurfaceV1, ProtocolArgumentRoleSchemaV2,
    ProtocolArgumentRoleV2, ProtocolCapabilityContractV2, ProtocolConstantContractV2,
    ProtocolModeCompilerBudgetV2, ProtocolModeProgramV2, ProtocolModeSetV2,
    ProtocolRoleCardinalityV2, ProtocolSelectorProgramV2, ProtocolSourceRoleSchemaV2,
    ProtocolSourceRoleV2, ProtocolStructuralGuardV2, ProtocolTemporalCardinalityContractV2,
    ProtocolValueContractV2, TrustedResolvedBindingRowV2, compile_protocol_modes_for_effect_law_v3,
};
use serde_json::json;

fn root(seed: &str) -> String {
    sha256_bytes(seed.as_bytes())
}

fn actor_input(seed: &str, outcome: PhysicalActorOutcomeV2) -> PhysicalActorObservationInputV2 {
    PhysicalActorObservationInputV2 {
        frozen_row_root_sha256: root(&format!("{seed}:row")),
        frozen_graph_root_sha256: root(&format!("{seed}:graph")),
        capture_root_sha256: root(&format!("{seed}:capture")),
        pre_state_root_sha256: root(&format!("{seed}:pre")),
        actor_program_digest_sha256: root("actor-program-v2"),
        candidate_action_digest_sha256: root(&format!("{seed}:candidate-action")),
        observed_post_state_root_sha256: root(&format!("{seed}:post")),
        observed_delta_root_sha256: root(&format!("{seed}:delta")),
        actor_outcome: outcome,
    }
}

fn verifier_for(
    actor: &PhysicalActorObservationV2,
    outcome: IndependentTrialVerifierOutcomeV2,
) -> IndependentTrialVerifierReceiptV2 {
    let delta = if outcome == IndependentTrialVerifierOutcomeV2::Fail {
        root("verifier-disagrees-delta")
    } else {
        actor.observed_delta_root_sha256.clone()
    };
    verify_independent_physical_trial_v2(IndependentTrialVerifierInputV2 {
        actor_observation_sha256: actor.observation_sha256.clone(),
        independent_verifier_program_digest_sha256: root("independent-verifier-program-v2"),
        independently_recomputed_delta_root_sha256: delta,
        structural_invariant_roots_sha256: vec![root("invariant:b"), root("invariant:a")],
        outcome,
    })
    .expect("verifier receipt")
}

fn joined_roots(
    actor: &PhysicalActorObservationV2,
    verifier: &IndependentTrialVerifierReceiptV2,
) -> PhysicalTrialJoinedRootsV2 {
    PhysicalTrialJoinedRootsV2 {
        frozen_row_root_sha256: actor.frozen_row_root_sha256.clone(),
        frozen_graph_root_sha256: actor.frozen_graph_root_sha256.clone(),
        capture_root_sha256: actor.capture_root_sha256.clone(),
        pre_state_root_sha256: actor.pre_state_root_sha256.clone(),
        actor_program_digest_sha256: actor.actor_program_digest_sha256.clone(),
        verifier_program_digest_sha256: verifier.independent_verifier_program_digest_sha256.clone(),
        candidate_action_digest_sha256: actor.candidate_action_digest_sha256.clone(),
        observed_post_state_root_sha256: actor.observed_post_state_root_sha256.clone(),
        observed_delta_root_sha256: actor.observed_delta_root_sha256.clone(),
        actor_observation_sha256: actor.observation_sha256.clone(),
        verifier_receipt_sha256: verifier.verifier_receipt_sha256.clone(),
    }
}

fn pass_trial(seed: &str) -> PhysicalTrialReceiptV2 {
    let actor = observe_physical_actor_v2(actor_input(seed, PhysicalActorOutcomeV2::Applied))
        .expect("actor observation");
    let verifier = verifier_for(&actor, IndependentTrialVerifierOutcomeV2::Pass);
    seal_physical_trial_receipt_v2(joined_roots(&actor, &verifier), actor, verifier)
        .expect("sealed trial")
}

#[test]
fn matching_actor_and_verifier_roots_create_tamper_evident_trial() {
    let trial = pass_trial("positive-support");
    assert_eq!(trial.outcome, PhysicalTrialOutcomeV2::Pass);
    assert!(!trial.execution_authority);
    assert_ne!(
        trial.joined_roots.actor_program_digest_sha256,
        trial.joined_roots.verifier_program_digest_sha256
    );
    assert_eq!(
        trial
            .verifier_receipt
            .independently_recomputed_delta_root_sha256,
        trial.actor_observation.observed_delta_root_sha256
    );
}

#[test]
fn actor_and_verifier_program_digest_swaps_are_rejected() {
    let actor =
        observe_physical_actor_v2(actor_input("program-swap", PhysicalActorOutcomeV2::Applied))
            .expect("actor observation");
    let verifier = verifier_for(&actor, IndependentTrialVerifierOutcomeV2::Pass);
    let mut roots = joined_roots(&actor, &verifier);
    roots.actor_program_digest_sha256 = root("foreign-actor-program");
    assert_eq!(
        seal_physical_trial_receipt_v2(roots, actor.clone(), verifier.clone()),
        Err(PhysicalTrialV2Error::InvalidJoinedRoots)
    );
    let mut roots = joined_roots(&actor, &verifier);
    roots.verifier_program_digest_sha256 = root("foreign-verifier-program");
    assert_eq!(
        seal_physical_trial_receipt_v2(roots, actor, verifier),
        Err(PhysicalTrialV2Error::InvalidJoinedRoots)
    );
}

#[test]
fn same_program_digest_cannot_self_verify() {
    let actor =
        observe_physical_actor_v2(actor_input("same-program", PhysicalActorOutcomeV2::Applied))
            .expect("actor observation");
    let verifier = verify_independent_physical_trial_v2(IndependentTrialVerifierInputV2 {
        actor_observation_sha256: actor.observation_sha256.clone(),
        independent_verifier_program_digest_sha256: actor.actor_program_digest_sha256.clone(),
        independently_recomputed_delta_root_sha256: actor.observed_delta_root_sha256.clone(),
        structural_invariant_roots_sha256: vec![root("same-program-invariant")],
        outcome: IndependentTrialVerifierOutcomeV2::Pass,
    })
    .expect("verifier receipt");
    assert_eq!(
        seal_physical_trial_receipt_v2(joined_roots(&actor, &verifier), actor, verifier),
        Err(PhysicalTrialV2Error::ProgramDigestNotIndependent)
    );
}

#[test]
fn changed_candidate_pre_post_or_delta_root_is_rejected() {
    let actor =
        observe_physical_actor_v2(actor_input("root-change", PhysicalActorOutcomeV2::Applied))
            .expect("actor observation");
    let verifier = verifier_for(&actor, IndependentTrialVerifierOutcomeV2::Pass);
    for mutate in [
        "frozen_graph_root_sha256",
        "capture_root_sha256",
        "candidate_action_digest_sha256",
        "pre_state_root_sha256",
        "observed_post_state_root_sha256",
        "observed_delta_root_sha256",
        "actor_observation_sha256",
        "verifier_receipt_sha256",
    ] {
        let mut roots = joined_roots(&actor, &verifier);
        match mutate {
            "frozen_graph_root_sha256" => roots.frozen_graph_root_sha256 = root("changed-graph"),
            "capture_root_sha256" => roots.capture_root_sha256 = root("changed-capture"),
            "candidate_action_digest_sha256" => {
                roots.candidate_action_digest_sha256 = root("changed-candidate")
            }
            "pre_state_root_sha256" => roots.pre_state_root_sha256 = root("changed-pre"),
            "observed_post_state_root_sha256" => {
                roots.observed_post_state_root_sha256 = root("changed-post")
            }
            "observed_delta_root_sha256" => {
                roots.observed_delta_root_sha256 = root("changed-delta")
            }
            "actor_observation_sha256" => {
                roots.actor_observation_sha256 = root("changed-actor-receipt")
            }
            "verifier_receipt_sha256" => {
                roots.verifier_receipt_sha256 = root("changed-verifier-receipt")
            }
            _ => unreachable!(),
        }
        assert_eq!(
            seal_physical_trial_receipt_v2(roots, actor.clone(), verifier.clone()),
            Err(PhysicalTrialV2Error::InvalidJoinedRoots)
        );
    }
}

#[test]
fn actor_or_verifier_receipt_mutation_is_rejected() {
    let actor = observe_physical_actor_v2(actor_input(
        "receipt-mutation",
        PhysicalActorOutcomeV2::Applied,
    ))
    .expect("actor observation");
    let verifier = verifier_for(&actor, IndependentTrialVerifierOutcomeV2::Pass);

    let mut changed_actor = actor.clone();
    changed_actor.actor_outcome = PhysicalActorOutcomeV2::Failed;
    assert_eq!(
        seal_physical_trial_receipt_v2(
            joined_roots(&actor, &verifier),
            changed_actor,
            verifier.clone()
        ),
        Err(PhysicalTrialV2Error::InvalidActor)
    );

    let mut changed_verifier = verifier.clone();
    changed_verifier.outcome = IndependentTrialVerifierOutcomeV2::Fail;
    assert_eq!(
        seal_physical_trial_receipt_v2(joined_roots(&actor, &verifier), actor, changed_verifier),
        Err(PhysicalTrialV2Error::InvalidVerifier)
    );
}

#[test]
fn verifier_disagreement_creates_fail_receipt_without_authority() {
    let actor = observe_physical_actor_v2(actor_input(
        "verifier-fail",
        PhysicalActorOutcomeV2::Applied,
    ))
    .expect("actor observation");
    let verifier = verifier_for(&actor, IndependentTrialVerifierOutcomeV2::Fail);
    let trial = seal_physical_trial_receipt_v2(joined_roots(&actor, &verifier), actor, verifier)
        .expect("fail receipt");
    assert_eq!(trial.outcome, PhysicalTrialOutcomeV2::Fail);
    assert!(!trial.execution_authority);
}

#[test]
fn unavailable_environment_is_censored_without_semantic_evidence() {
    let actor =
        observe_physical_actor_v2(actor_input("censored", PhysicalActorOutcomeV2::Censored))
            .expect("actor observation");
    let verifier = verifier_for(&actor, IndependentTrialVerifierOutcomeV2::Censored);
    let trial = seal_physical_trial_receipt_v2(joined_roots(&actor, &verifier), actor, verifier)
        .expect("censored receipt");
    assert_eq!(trial.outcome, PhysicalTrialOutcomeV2::Censored);
    assert!(!trial.execution_authority);
}

#[test]
fn physical_trial_json_roundtrip_is_byte_identical() {
    let trial = pass_trial("roundtrip");
    let bytes = trial.canonical_bytes().expect("trial bytes");
    let restored = PhysicalTrialReceiptV2::from_canonical_bytes(&bytes).expect("restore");
    assert_eq!(restored, trial);
    assert_eq!(restored.canonical_bytes().expect("restored bytes"), bytes);
}

#[test]
fn v2_sources_do_not_depend_on_v1_fixture_truth() {
    let sources = [
        include_str!("binding_evidence_adjudication/physical_actor_observation_v2.rs"),
        include_str!("binding_evidence_adjudication/independent_trial_verifier_v2.rs"),
        include_str!("binding_evidence_adjudication/physical_trial_v2.rs"),
    ]
    .join("\n");
    for forbidden in [
        "support_scene",
        "future_scene",
        "intervention_id",
        "BindingPhysicalLabelReceiptV1",
        "BindingPhysicalLabelReceiptSetV1",
        "expected_action_equivalence",
    ] {
        assert!(
            !sources.contains(forbidden),
            "forbidden V2 fixture dependency: {forbidden}"
        );
    }
}

fn frozen_row(
    trial: &PhysicalTrialReceiptV2,
    partition: BindingEvidencePartitionV2,
    evidence_label: BindingTrialEvidenceLabelV2,
    relation: &str,
    effect_invariant_root_sha256: &str,
    source: TrustedBindingResolverReceiptSourceV2,
) -> FrozenBindingTrialRowV2 {
    FrozenBindingTrialRowV2 {
        frozen_row_root_sha256: trial.joined_roots.frozen_row_root_sha256.clone(),
        frozen_graph_root_sha256: trial.joined_roots.frozen_graph_root_sha256.clone(),
        capture_root_sha256: trial.joined_roots.capture_root_sha256.clone(),
        partition,
        evidence_label,
        relation_identity_sha256: relation.to_owned(),
        protocol_facet_root_sha256: root(&format!(
            "protocol-facet:{}",
            trial.joined_roots.frozen_row_root_sha256
        )),
        effect_invariant_root_sha256: effect_invariant_root_sha256.to_owned(),
        physical_program_id_sha256: root("physical-program-family"),
        surface_root_sha256: root(&format!(
            "surface:{}",
            trial.joined_roots.frozen_graph_root_sha256
        )),
        receipt_source: source,
    }
}

fn abstain_trial(seed: &str) -> PhysicalTrialReceiptV2 {
    let actor = observe_physical_actor_v2(actor_input(seed, PhysicalActorOutcomeV2::Abstained))
        .expect("actor observation");
    let verifier = verifier_for(&actor, IndependentTrialVerifierOutcomeV2::Abstain);
    seal_physical_trial_receipt_v2(joined_roots(&actor, &verifier), actor, verifier)
        .expect("abstain trial")
}

fn controlled_resolved_fixture() -> (
    TrustedResolvedBindingRowsV2,
    String,
    Vec<String>,
    Vec<String>,
) {
    controlled_resolved_fixture_with_effect_invariant(&root("same-effect-invariant"))
}

fn controlled_resolved_fixture_with_effect_invariant(
    effect_invariant_root_sha256: &str,
) -> (
    TrustedResolvedBindingRowsV2,
    String,
    Vec<String>,
    Vec<String>,
) {
    let relation = root("parent-action-to-capability-instance-v2");
    let support_positive = pass_trial("support-positive");
    let future_positive = pass_trial("future-positive");
    let support_negative = abstain_trial("support-negative");
    let future_negative = abstain_trial("future-negative");
    let trials = vec![
        support_positive.clone(),
        future_positive.clone(),
        support_negative.clone(),
        future_negative.clone(),
    ];
    let rows = vec![
        frozen_row(
            &support_positive,
            BindingEvidencePartitionV2::Support,
            BindingTrialEvidenceLabelV2::Positive,
            &relation,
            effect_invariant_root_sha256,
            TrustedBindingResolverReceiptSourceV2::ControlledFixture,
        ),
        frozen_row(
            &future_positive,
            BindingEvidencePartitionV2::Future,
            BindingTrialEvidenceLabelV2::Positive,
            &relation,
            effect_invariant_root_sha256,
            TrustedBindingResolverReceiptSourceV2::ControlledFixture,
        ),
        frozen_row(
            &support_negative,
            BindingEvidencePartitionV2::Support,
            BindingTrialEvidenceLabelV2::ApplicabilityNegative,
            &relation,
            effect_invariant_root_sha256,
            TrustedBindingResolverReceiptSourceV2::ControlledFixture,
        ),
        frozen_row(
            &future_negative,
            BindingEvidencePartitionV2::Future,
            BindingTrialEvidenceLabelV2::ApplicabilityNegative,
            &relation,
            effect_invariant_root_sha256,
            TrustedBindingResolverReceiptSourceV2::ControlledFixture,
        ),
    ];
    let positive_rows = rows
        .iter()
        .filter(|row| row.evidence_label == BindingTrialEvidenceLabelV2::Positive)
        .map(|row| row.frozen_row_root_sha256.clone())
        .collect::<Vec<_>>();
    let negative_rows = rows
        .iter()
        .filter(|row| row.evidence_label == BindingTrialEvidenceLabelV2::ApplicabilityNegative)
        .map(|row| row.frozen_row_root_sha256.clone())
        .collect::<Vec<_>>();
    let resolver_program = root("trusted-resolver-v2");
    let external_manifest_root =
        trusted_binding_resolver_manifest_root_v2(&rows, &trials, &resolver_program)
            .expect("resolver manifest root");
    let resolved = resolve_trusted_binding_rows_v2(TrustedBindingResolverInputV2 {
        frozen_rows: rows,
        physical_trials: trials,
        resolver_program_digest_sha256: resolver_program,
        external_manifest_root_sha256: external_manifest_root,
    })
    .expect("trusted resolved rows");
    (resolved, relation, positive_rows, negative_rows)
}

#[test]
fn trusted_resolver_rejects_tampered_manifest_root() {
    let relation = root("resolver-tamper-relation");
    let trial = pass_trial("resolver-tamper");
    let trials = vec![trial.clone()];
    let rows = vec![frozen_row(
        &trial,
        BindingEvidencePartitionV2::Support,
        BindingTrialEvidenceLabelV2::Positive,
        &relation,
        &root("resolver-tamper-invariant"),
        TrustedBindingResolverReceiptSourceV2::ControlledFixture,
    )];
    let resolver_program = root("trusted-resolver-v2");
    let external_manifest_root =
        trusted_binding_resolver_manifest_root_v2(&rows, &trials, &resolver_program)
            .expect("resolver manifest root");
    let mut tampered = rows.clone();
    tampered[0].surface_root_sha256 = root("tampered-surface");
    assert_eq!(
        resolve_trusted_binding_rows_v2(TrustedBindingResolverInputV2 {
            frozen_rows: tampered,
            physical_trials: trials,
            resolver_program_digest_sha256: resolver_program,
            external_manifest_root_sha256: external_manifest_root,
        }),
        Err(TrustedResolverV2Error::InvalidTrustRoot)
    );
}

#[test]
fn capability_is_opaque_and_report_does_not_become_authority() {
    let (resolved, relation, _, _) = controlled_resolved_fixture();
    let outcome =
        adjudicate_binding_law_evidence_v2(&resolved, &relation).expect("adjudication outcome");
    let BindingAdjudicationOutcomeV2::Accepted(evidence) = outcome else {
        panic!("controlled evidence should produce a scoped capability");
    };
    assert_eq!(
        evidence.evidence_scope(),
        AcceptedBindingEvidenceScopeV2::ControlledFixture
    );
    assert!(!evidence.production_admissible());
    assert!(!evidence.execution_authority());
    assert!(evidence.capability_root_sha256().len() == 64);
    assert!(!evidence.report().execution_authority);
    let source = include_str!("binding_evidence_adjudication/binding_law_evidence_v2.rs");
    assert!(
        !source.contains("Deserialize, Eq, PartialEq)]\npub struct AcceptedBindingLawEvidenceV2")
    );
    assert!(!source.contains("pub fn new("));
}

#[test]
fn protocol_mode_compiler_has_separate_owner() {
    let proof_module = include_str!("binding_evidence_adjudication/mod.rs");
    let compiler_module = include_str!("protocol_mode.rs");
    assert!(!proof_module.contains("protocol_mode_compiler"));
    assert!(!proof_module.contains("compile_protocol_modes_v2"));
    assert!(compiler_module.contains("fn exact_cover_protocol_modes_v2"));
}

fn mode_candidate(
    id: &str,
    effect_law_id: &str,
    relation: &str,
    action_class: &str,
    protocol_facet_root: &str,
    effect_invariant_root: &str,
    claimed_positive_rows: Vec<String>,
) -> BoundedProtocolModeCandidateV2 {
    let selector_program = ProtocolSelectorProgramV2 {
        predicates: Vec::new(),
        max_action_classes: 1,
    };
    let selector_program_root_sha256 =
        protocol_component_root("selector-program", &selector_program);
    let program = ProtocolModeProgramV2 {
        source_role_schema: ProtocolSourceRoleSchemaV2 {
            roles: vec![ProtocolSourceRoleV2 {
                role_id: 0,
                value_type: BindingValueTypeV1::Integer,
                cardinality: ProtocolRoleCardinalityV2::OneActionClass,
            }],
        },
        selector_program,
        value_contract: ProtocolValueContractV2 {
            observed: BindingValueTypeV1::Integer,
            emitted: BindingValueTypeV1::Integer,
        },
        capability_contract: ProtocolCapabilityContractV2 {
            protocol_facet_root_sha256: protocol_facet_root.to_owned(),
            physical_program_ids_sha256: vec![root(&format!("{id}:physical-program"))],
        },
        argument_role_schema: ProtocolArgumentRoleSchemaV2 {
            roles: vec![ProtocolArgumentRoleV2 {
                argument_ordinal: 0,
                source_role_id: 0,
            }],
        },
        constant_contract: ProtocolConstantContractV2 {
            semantic_constants_sha256: Vec::new(),
            protocol_noop_constants_sha256: Vec::new(),
            execution_budget_roots_sha256: Vec::new(),
            transport_default_roots_sha256: Vec::new(),
        },
        structural_guard: ProtocolStructuralGuardV2 {
            relation_identity_sha256: relation.to_owned(),
            effect_invariant_root_sha256: effect_invariant_root.to_owned(),
            selector_program_root_sha256,
        },
        temporal_cardinality_contract: ProtocolTemporalCardinalityContractV2 {
            completion_states: Vec::new(),
            temporal_distances: Vec::new(),
            event_candidate_cardinalities: Vec::new(),
            require_unique_action_class: true,
        },
    };
    BoundedProtocolModeCandidateV2 {
        candidate_id_sha256: root(id),
        effect_law_id_sha256: effect_law_id.to_owned(),
        relation_identity_sha256: relation.to_owned(),
        protocol_facet_root_sha256: protocol_facet_root.to_owned(),
        effect_invariant_root_sha256: effect_invariant_root.to_owned(),
        source_role_schema_root_sha256: protocol_component_root(
            "source-role-schema",
            &program.source_role_schema,
        ),
        selector_program_root_sha256: protocol_component_root(
            "selector-program",
            &program.selector_program,
        ),
        observed_emitted_types_root_sha256: protocol_component_root(
            "observed-emitted-types",
            &program.value_contract,
        ),
        capability_protocol_root_sha256: protocol_component_root(
            "capability-protocol",
            &program.capability_contract,
        ),
        argument_role_schema_root_sha256: protocol_component_root(
            "argument-role-schema",
            &program.argument_role_schema,
        ),
        constant_contract_root_sha256: protocol_component_root(
            "constant-contract",
            &program.constant_contract,
        ),
        structural_guard_root_sha256: protocol_component_root(
            "structural-guard",
            &program.structural_guard,
        ),
        temporal_cardinality_contract_root_sha256: protocol_component_root(
            "temporal-cardinality",
            &program.temporal_cardinality_contract,
        ),
        action_class_root_sha256: action_class.to_owned(),
        program,
        covers_positive_rows_sha256: claimed_positive_rows,
        accepts_negative_rows_sha256: Vec::new(),
        wrong_action_rows_sha256: Vec::new(),
        verify_failed_rows_sha256: Vec::new(),
        search_exhausted: false,
    }
}

fn protocol_component_root<T: serde::Serialize>(label: &str, value: &T) -> String {
    crate::canonical_json_sha256(&(crate::PROTOCOL_MODE_SET_SCHEMA_V2, label, value))
        .expect("protocol component digest")
}

fn scalar_binding_graph(
    seed: &str,
    target: u64,
    request_mentions_target: bool,
    wrapped: bool,
    ambiguous: bool,
) -> FrozenCandidateRelationGraphV1 {
    let previous = target.saturating_sub(1);
    let request = if ambiguous {
        format!("continue with {previous} and {target}")
    } else if request_mentions_target {
        format!("continue with {target}")
    } else {
        "continue with another value".to_owned()
    };
    let payload = if ambiguous {
        json!({
            "first_surface": previous,
            "second_surface": target
        })
    } else if wrapped {
        json!({
            "renamed_envelope": {
                "renamed_events": [
                    {"renamed_value": previous},
                    {"renamed_value": target}
                ]
            }
        })
    } else {
        json!({
            "events": [
                {"value": previous},
                {"value": target}
            ]
        })
    };
    PreActionBindingSurfaceV1::capture(
        root(&format!("{seed}:graph-row")),
        root(&format!("{seed}:graph-evidence")),
        &request,
        &payload,
        PreActionBindingContextV1 {
            call_shape_count: 1,
            capability_count: 1,
            completion_state: BindingCompletionStateV1::Unresolved,
            temporal_relation_count: 1,
            cardinality_relation_count: 1,
            topology_neighborhood_root_sha256: root("f4r2-context-topology"),
        },
        BindingEvidenceBudgetV1::default(),
    )
    .expect("structural pre-action surface")
    .candidate_relation_graph(BindingEvidenceBudgetV1::default())
    .expect("structural candidate graph")
    .freeze()
    .expect("frozen structural graph")
}

fn scalar_trial_for_graph(
    seed: &str,
    graph: &FrozenCandidateRelationGraphV1,
    target: u64,
    positive: bool,
) -> PhysicalTrialReceiptV2 {
    let mut input = actor_input(
        seed,
        if positive {
            PhysicalActorOutcomeV2::Applied
        } else {
            PhysicalActorOutcomeV2::Abstained
        },
    );
    input.frozen_graph_root_sha256 = graph.graph_root_sha256.clone();
    input.candidate_action_digest_sha256 =
        crate::canonical_json_sha256(&target).expect("target action digest");
    let actor = observe_physical_actor_v2(input).expect("structural actor observation");
    let verifier = verifier_for(
        &actor,
        if positive {
            IndependentTrialVerifierOutcomeV2::Pass
        } else {
            IndependentTrialVerifierOutcomeV2::Abstain
        },
    );
    seal_physical_trial_receipt_v2(joined_roots(&actor, &verifier), actor, verifier)
        .expect("structural physical trial")
}

#[allow(clippy::too_many_arguments)]
fn push_structural_case(
    graphs: &mut Vec<FrozenCandidateRelationGraphV1>,
    trials: &mut Vec<PhysicalTrialReceiptV2>,
    rows: &mut Vec<FrozenBindingTrialRowV2>,
    seed: &str,
    target: u64,
    request_mentions_target: bool,
    wrapped: bool,
    ambiguous: bool,
    partition: BindingEvidencePartitionV2,
    label: BindingTrialEvidenceLabelV2,
    relation: &str,
    effect_invariant: &str,
    facet: &str,
    physical_program: &str,
) {
    let positive = label == BindingTrialEvidenceLabelV2::Positive;
    let graph = scalar_binding_graph(seed, target, request_mentions_target, wrapped, ambiguous);
    let trial = scalar_trial_for_graph(seed, &graph, target, positive);
    rows.push(FrozenBindingTrialRowV2 {
        frozen_row_root_sha256: trial.joined_roots.frozen_row_root_sha256.clone(),
        frozen_graph_root_sha256: graph.graph_root_sha256.clone(),
        capture_root_sha256: trial.joined_roots.capture_root_sha256.clone(),
        partition,
        evidence_label: label,
        relation_identity_sha256: relation.to_owned(),
        protocol_facet_root_sha256: facet.to_owned(),
        effect_invariant_root_sha256: effect_invariant.to_owned(),
        physical_program_id_sha256: physical_program.to_owned(),
        surface_root_sha256: graph.graph_root_sha256.clone(),
        receipt_source: TrustedBindingResolverReceiptSourceV2::ControlledFixture,
    });
    trials.push(trial);
    graphs.push(graph);
}

fn structural_f4r2_fixture(
    effect_law: &CanonicalEffectLawV3,
    ambiguous_positive: bool,
) -> (
    AcceptedBindingLawEvidenceV2,
    Vec<FrozenCandidateRelationGraphV1>,
) {
    let relation = root("parent-action-to-capability-instance-f4r2");
    let facet_a = root("protocol-facet-wait");
    let facet_b = root("protocol-facet-write-stdin");
    let program_a = root("physical-program-wait");
    let program_b = root("physical-program-write-stdin");
    let mut graphs = Vec::new();
    let mut trials = Vec::new();
    let mut rows = Vec::new();
    for (seed, target, wrapped, partition, facet, program) in [
        (
            "a-support-positive",
            41,
            false,
            BindingEvidencePartitionV2::Support,
            &facet_a,
            &program_a,
        ),
        (
            "a-future-positive",
            42,
            true,
            BindingEvidencePartitionV2::Future,
            &facet_a,
            &program_a,
        ),
        (
            "b-support-positive",
            51,
            true,
            BindingEvidencePartitionV2::Support,
            &facet_b,
            &program_b,
        ),
        (
            "b-future-positive",
            52,
            false,
            BindingEvidencePartitionV2::Future,
            &facet_b,
            &program_b,
        ),
    ] {
        push_structural_case(
            &mut graphs,
            &mut trials,
            &mut rows,
            seed,
            target,
            true,
            wrapped,
            ambiguous_positive,
            partition,
            BindingTrialEvidenceLabelV2::Positive,
            &relation,
            effect_law.effect_invariant_root_sha256(),
            facet,
            program,
        );
    }
    for (seed, target, wrapped, partition, facet, program) in [
        (
            "a-support-negative",
            43,
            false,
            BindingEvidencePartitionV2::Support,
            &facet_a,
            &program_a,
        ),
        (
            "a-future-negative",
            44,
            true,
            BindingEvidencePartitionV2::Future,
            &facet_a,
            &program_a,
        ),
        (
            "b-support-negative",
            53,
            true,
            BindingEvidencePartitionV2::Support,
            &facet_b,
            &program_b,
        ),
        (
            "b-future-negative",
            54,
            false,
            BindingEvidencePartitionV2::Future,
            &facet_b,
            &program_b,
        ),
    ] {
        push_structural_case(
            &mut graphs,
            &mut trials,
            &mut rows,
            seed,
            target,
            false,
            wrapped,
            false,
            partition,
            BindingTrialEvidenceLabelV2::ApplicabilityNegative,
            &relation,
            effect_law.effect_invariant_root_sha256(),
            facet,
            program,
        );
    }
    let resolver_program = root("trusted-structural-resolver-f4r2");
    let external_manifest_root =
        trusted_binding_resolver_manifest_root_v2(&rows, &trials, &resolver_program)
            .expect("structural resolver manifest");
    let resolved = resolve_trusted_binding_rows_v2(TrustedBindingResolverInputV2 {
        frozen_rows: rows,
        physical_trials: trials,
        resolver_program_digest_sha256: resolver_program,
        external_manifest_root_sha256: external_manifest_root,
    })
    .expect("trusted structural rows");
    let outcome =
        adjudicate_binding_law_evidence_v2(&resolved, &relation).expect("structural adjudication");
    let BindingAdjudicationOutcomeV2::Accepted(evidence) = outcome else {
        panic!("structural evidence should be accepted");
    };
    (evidence, graphs)
}

fn rows_with_label(
    evidence: &AcceptedBindingLawEvidenceV2,
    label: BindingTrialEvidenceLabelV2,
) -> Vec<&TrustedResolvedBindingRowV2> {
    evidence
        .rows()
        .iter()
        .filter(|row| row.evidence_label == label)
        .collect::<Vec<_>>()
}

#[test]
fn end_to_end_controlled_evidence_reaches_unique_safe_protocol_mode_set() {
    let (resolved, relation, positive_rows, _) = controlled_resolved_fixture();
    let outcome =
        adjudicate_binding_law_evidence_v2(&resolved, &relation).expect("adjudication outcome");
    let BindingAdjudicationOutcomeV2::Accepted(evidence) = outcome else {
        panic!("controlled evidence should produce capability");
    };
    let effect_law_id = root("effect-law-v3");
    let action_class = root("unique-action-class");
    let positive_views = rows_with_label(&evidence, BindingTrialEvidenceLabelV2::Positive);
    assert_eq!(positive_views.len(), 2);
    let mode_a = mode_candidate(
        "direct-mode",
        &effect_law_id,
        &relation,
        &action_class,
        &positive_views[0].protocol_facet_root_sha256,
        &positive_views[0].effect_invariant_root_sha256,
        positive_rows.clone(),
    );
    let mode_b = mode_candidate(
        "wrapped-mode",
        &effect_law_id,
        &relation,
        &action_class,
        &positive_views[1].protocol_facet_root_sha256,
        &positive_views[1].effect_invariant_root_sha256,
        Vec::new(),
    );
    let single_mode_set = compile_protocol_modes_v2(
        &evidence,
        &effect_law_id,
        vec![mode_a.clone()],
        ProtocolModeCompilerBudgetV2::default(),
    )
    .expect("compile single mode");
    assert_eq!(
        single_mode_set.verdict,
        BindingProtocolCompileVerdictV2::Abstain
    );
    assert_eq!(single_mode_set.positive_rows_covered, 1);

    let mode_set = compile_protocol_modes_v2(
        &evidence,
        &effect_law_id,
        vec![mode_a, mode_b],
        ProtocolModeCompilerBudgetV2::default(),
    )
    .expect("compile protocol modes");
    assert_eq!(
        mode_set.verdict,
        BindingProtocolCompileVerdictV2::ProtocolModeSet
    );
    assert_eq!(mode_set.modes.len(), 2);
    assert_eq!(mode_set.positive_rows, 2);
    assert_eq!(mode_set.positive_rows_covered, 2);
    assert_eq!(mode_set.wrong_actions, 0);
    assert_eq!(mode_set.verify_failed, 0);
    assert_eq!(mode_set.negative_accepts, 0);
    assert!(!mode_set.search_exhausted);
    assert_eq!(mode_set.action_equivalence_classes, 1);
    assert!(mode_set.all_surviving_covers_action_equivalent);
    assert!(!mode_set.production_admissible);
    assert!(!mode_set.execution_authority);
    let bytes = mode_set.canonical_bytes().expect("mode bytes");
    assert_eq!(bytes, mode_set.canonical_bytes().expect("mode bytes again"));
}

#[test]
fn typed_effect_law_entrypoint_binds_action_equivalence() {
    let effect_law = crate::effect_law_v3::test_only_canonical_effect_law_v3("f4r-typed-law");
    let (evidence, graph_views) = structural_f4r2_fixture(&effect_law, false);
    let mode_set = compile_protocol_modes_for_effect_law_v3(
        &evidence,
        &effect_law,
        &graph_views,
        ProtocolModeCompilerBudgetV2::default(),
    )
    .expect("typed compile");
    assert_eq!(
        mode_set.verdict,
        BindingProtocolCompileVerdictV2::ProtocolModeSet
    );
    assert_eq!(mode_set.modes.len(), 2);
    assert!(
        mode_set
            .modes
            .iter()
            .all(|mode| mode.covered_positive_rows_sha256.len() == 2)
    );
    assert!(
        mode_set.modes.iter().all(
            |mode| mode.action_class_root_sha256 == effect_law.action_equivalence_root_sha256()
        )
    );
    assert!(mode_set.modes.iter().all(|mode| {
        mode.program.selector_program.predicates
            == vec![BindingPredicateV1::RequestRelation {
                value: BindingRequestRelationV1::Mentioned,
            }]
    }));
    let bytes = mode_set.canonical_bytes().expect("canonical mode set");
    let restored = ProtocolModeSetV2::from_canonical_bytes(&bytes).expect("restart mode set");
    assert_eq!(restored, mode_set);
    assert_eq!(restored.canonical_bytes().expect("restart bytes"), bytes);

    let incompatible_law =
        crate::effect_law_v3::test_only_canonical_effect_law_v3("f4r-incompatible-law");
    let rejected = compile_protocol_modes_for_effect_law_v3(
        &evidence,
        &incompatible_law,
        &graph_views,
        ProtocolModeCompilerBudgetV2::default(),
    )
    .expect("typed incompatible compile");
    assert_eq!(rejected.verdict, BindingProtocolCompileVerdictV2::Abstain);
    assert_eq!(rejected.wrong_actions, 0);
    assert_eq!(rejected.positive_rows_covered, 0);
}

#[test]
fn canonical_f4_rejects_missing_or_tampered_graph_payload() {
    let effect_law = crate::effect_law_v3::test_only_canonical_effect_law_v3("f4r-graph-seal");
    let (evidence, graph_views) = structural_f4r2_fixture(&effect_law, false);
    assert_eq!(
        compile_protocol_modes_for_effect_law_v3(
            &evidence,
            &effect_law,
            &graph_views[..graph_views.len() - 1],
            ProtocolModeCompilerBudgetV2::default(),
        ),
        Err(BindingProtocolCompilerErrorV2::InvalidGraphView)
    );
    let mut extra = graph_views.clone();
    extra.push(scalar_binding_graph(
        "f4r-extra-graph",
        99,
        true,
        false,
        false,
    ));
    assert_eq!(
        compile_protocol_modes_for_effect_law_v3(
            &evidence,
            &effect_law,
            &extra,
            ProtocolModeCompilerBudgetV2::default(),
        ),
        Err(BindingProtocolCompilerErrorV2::InvalidGraphView)
    );
    let mut tampered = graph_views;
    tampered[0].graph.context.capability_count = 2;
    assert_eq!(
        compile_protocol_modes_for_effect_law_v3(
            &evidence,
            &effect_law,
            &tampered,
            ProtocolModeCompilerBudgetV2::default(),
        ),
        Err(BindingProtocolCompilerErrorV2::InvalidGraphView)
    );
}

#[test]
fn canonical_f4_abstains_when_structural_roles_remain_symmetric() {
    let effect_law = crate::effect_law_v3::test_only_canonical_effect_law_v3("f4r-symmetric");
    let (evidence, graph_views) = structural_f4r2_fixture(&effect_law, true);
    let mode_set = compile_protocol_modes_for_effect_law_v3(
        &evidence,
        &effect_law,
        &graph_views,
        ProtocolModeCompilerBudgetV2::default(),
    )
    .expect("symmetric compile");
    assert_eq!(mode_set.verdict, BindingProtocolCompileVerdictV2::Abstain);
    assert!(mode_set.modes.is_empty());
    assert_eq!(mode_set.positive_rows, 4);
    assert_eq!(mode_set.positive_rows_covered, 0);
    assert!(mode_set.wrong_actions > 0);
    assert!(!mode_set.execution_authority);
}

#[test]
fn canonical_f4_restart_rejects_tampered_selector_program() {
    let effect_law = crate::effect_law_v3::test_only_canonical_effect_law_v3("f4r-restart-tamper");
    let (evidence, graph_views) = structural_f4r2_fixture(&effect_law, false);
    let mode_set = compile_protocol_modes_for_effect_law_v3(
        &evidence,
        &effect_law,
        &graph_views,
        ProtocolModeCompilerBudgetV2::default(),
    )
    .expect("typed compile");
    let mut value = serde_json::to_value(mode_set).expect("mode set value");
    value["modes"][0]["program"]["selector_program"]["max_action_classes"] = json!(2);
    let mut bytes = serde_json::to_vec_pretty(&value).expect("tampered canonical bytes");
    bytes.push(b'\n');
    assert!(ProtocolModeSetV2::from_canonical_bytes(&bytes).is_err());
}

#[test]
fn f4_abstains_on_negative_accept_or_competing_action_or_exhausted_search() {
    let (resolved, relation, positive_rows, negative_rows) = controlled_resolved_fixture();
    let outcome =
        adjudicate_binding_law_evidence_v2(&resolved, &relation).expect("adjudication outcome");
    let BindingAdjudicationOutcomeV2::Accepted(evidence) = outcome else {
        panic!("controlled evidence should produce capability");
    };
    let effect_law_id = root("effect-law-v3");
    let action_a = root("action-a");
    let action_b = root("action-b");
    let positive_views = rows_with_label(&evidence, BindingTrialEvidenceLabelV2::Positive);
    let negative_views = rows_with_label(
        &evidence,
        BindingTrialEvidenceLabelV2::ApplicabilityNegative,
    );

    let mut accepts_negative = mode_candidate(
        "accepts-negative",
        &effect_law_id,
        &relation,
        &action_a,
        &negative_views[0].protocol_facet_root_sha256,
        &negative_views[0].effect_invariant_root_sha256,
        Vec::new(),
    );
    accepts_negative.accepts_negative_rows_sha256 = vec![negative_rows[0].clone()];
    let set = compile_protocol_modes_v2(
        &evidence,
        &effect_law_id,
        vec![accepts_negative],
        ProtocolModeCompilerBudgetV2::default(),
    )
    .expect("compile negative accept");
    assert_eq!(set.verdict, BindingProtocolCompileVerdictV2::Abstain);
    assert_eq!(set.negative_accepts, 1);

    let mut accepts_unknown_negative = mode_candidate(
        "accepts-unknown-negative",
        &effect_law_id,
        &relation,
        &action_a,
        &positive_views[0].protocol_facet_root_sha256,
        &positive_views[0].effect_invariant_root_sha256,
        positive_rows.clone(),
    );
    accepts_unknown_negative.accepts_negative_rows_sha256 = vec![root("unknown-negative-row")];
    let set = compile_protocol_modes_v2(
        &evidence,
        &effect_law_id,
        vec![accepts_unknown_negative],
        ProtocolModeCompilerBudgetV2::default(),
    )
    .expect("compile unknown negative accept");
    assert_eq!(set.verdict, BindingProtocolCompileVerdictV2::Abstain);
    assert_eq!(set.negative_accepts, 0);
    assert_eq!(set.positive_rows_covered, 1);

    let set = compile_protocol_modes_v2(
        &evidence,
        &effect_law_id,
        vec![
            mode_candidate(
                "action-a",
                &effect_law_id,
                &relation,
                &action_a,
                &positive_views[0].protocol_facet_root_sha256,
                &positive_views[0].effect_invariant_root_sha256,
                Vec::new(),
            ),
            mode_candidate(
                "action-a-2",
                &effect_law_id,
                &relation,
                &action_a,
                &positive_views[1].protocol_facet_root_sha256,
                &positive_views[1].effect_invariant_root_sha256,
                Vec::new(),
            ),
            mode_candidate(
                "action-b",
                &effect_law_id,
                &relation,
                &action_b,
                &positive_views[0].protocol_facet_root_sha256,
                &positive_views[0].effect_invariant_root_sha256,
                Vec::new(),
            ),
            mode_candidate(
                "action-b-2",
                &effect_law_id,
                &relation,
                &action_b,
                &positive_views[1].protocol_facet_root_sha256,
                &positive_views[1].effect_invariant_root_sha256,
                Vec::new(),
            ),
        ],
        ProtocolModeCompilerBudgetV2::default(),
    )
    .expect("compile competing actions");
    assert_eq!(set.verdict, BindingProtocolCompileVerdictV2::Abstain);
    assert_eq!(set.action_equivalence_classes, 2);

    let mut exhausted = mode_candidate(
        "exhausted",
        &effect_law_id,
        &relation,
        &action_a,
        &positive_views[0].protocol_facet_root_sha256,
        &positive_views[0].effect_invariant_root_sha256,
        positive_rows,
    );
    exhausted.search_exhausted = true;
    let set = compile_protocol_modes_v2(
        &evidence,
        &effect_law_id,
        vec![exhausted],
        ProtocolModeCompilerBudgetV2::default(),
    )
    .expect("compile exhausted");
    assert_eq!(set.verdict, BindingProtocolCompileVerdictV2::Abstain);
    assert!(set.search_exhausted);
}
