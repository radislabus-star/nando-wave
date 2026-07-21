use super::*;

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
        effect_invariant_root_sha256: root("same-effect-invariant"),
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
            TrustedBindingResolverReceiptSourceV2::ControlledFixture,
        ),
        frozen_row(
            &future_positive,
            BindingEvidencePartitionV2::Future,
            BindingTrialEvidenceLabelV2::Positive,
            &relation,
            TrustedBindingResolverReceiptSourceV2::ControlledFixture,
        ),
        frozen_row(
            &support_negative,
            BindingEvidencePartitionV2::Support,
            BindingTrialEvidenceLabelV2::ApplicabilityNegative,
            &relation,
            TrustedBindingResolverReceiptSourceV2::ControlledFixture,
        ),
        frozen_row(
            &future_negative,
            BindingEvidencePartitionV2::Future,
            BindingTrialEvidenceLabelV2::ApplicabilityNegative,
            &relation,
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

fn mode_candidate(
    id: &str,
    effect_law_id: &str,
    relation: &str,
    action_class: &str,
    positive_rows: Vec<String>,
) -> BoundedProtocolModeCandidateV2 {
    BoundedProtocolModeCandidateV2 {
        candidate_id_sha256: root(id),
        effect_law_id_sha256: effect_law_id.to_owned(),
        relation_identity_sha256: relation.to_owned(),
        source_role_schema_root_sha256: root("source-role-schema"),
        selector_program_root_sha256: root(&format!("{id}:selector")),
        observed_emitted_types_root_sha256: root("observed-emitted-types"),
        capability_protocol_root_sha256: root("capability-protocol"),
        argument_role_schema_root_sha256: root("argument-role-schema"),
        constant_contract_root_sha256: root("constant-contract"),
        structural_guard_root_sha256: root("structural-guard"),
        temporal_cardinality_contract_root_sha256: root("temporal-cardinality"),
        action_class_root_sha256: action_class.to_owned(),
        covers_positive_rows_sha256: positive_rows,
        accepts_negative_rows_sha256: Vec::new(),
        wrong_action_rows_sha256: Vec::new(),
        verify_failed_rows_sha256: Vec::new(),
        search_exhausted: false,
    }
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
    let mode_set = compile_protocol_modes_v2(
        &evidence,
        &effect_law_id,
        vec![
            mode_candidate(
                "direct-mode",
                &effect_law_id,
                &relation,
                &action_class,
                positive_rows.clone(),
            ),
            mode_candidate(
                "wrapped-mode",
                &effect_law_id,
                &relation,
                &action_class,
                positive_rows.clone(),
            ),
        ],
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

    let mut accepts_negative = mode_candidate(
        "accepts-negative",
        &effect_law_id,
        &relation,
        &action_a,
        positive_rows.clone(),
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
    assert_eq!(set.negative_accepts, 1);

    let set = compile_protocol_modes_v2(
        &evidence,
        &effect_law_id,
        vec![
            mode_candidate(
                "action-a",
                &effect_law_id,
                &relation,
                &action_a,
                positive_rows.clone(),
            ),
            mode_candidate(
                "action-b",
                &effect_law_id,
                &relation,
                &action_b,
                positive_rows.clone(),
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
