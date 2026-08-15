use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use nando_operator_learning::{
    K2_UNCERTAINTY_PLAN_DISPATCH_SCHEMA_V2, K2_UNCERTAINTY_PROBE_DISPATCH_ITEM_SCHEMA_V2,
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionTreeManifestV1,
    K2InquiryObserverRequestV1, K2InquiryWorkerRequestV1, K2UncertaintyCaseJournalFaultV2,
    K2UncertaintyCaseJournalPhaseV2, K2UncertaintyCaseJournalV2,
    K2UncertaintyCasePreverificationV2, K2UncertaintyClosureCensusV1,
    K2UncertaintyClosureDispositionV1, K2UncertaintyClosurePlanV1,
    K2UncertaintyClosurePlannerRequestV1, K2UncertaintyClosureVerificationReceiptV1,
    K2UncertaintyClosureVerificationRequestV1, K2UncertaintyPlanDispatchV2,
    K2UncertaintyProbeDispatchItemV2, K2UncertaintyRawProbeDispositionV1,
    K2UncertaintySafetyRequestV1, K2UncertaintyWorkspaceIdentityV2, composition_root_v1,
    composition_sha256_file_v1, plan_self_formed_uncertainty_closure_v1,
    self_formed_grammar_root_v1, uncertainty_bytes_v1, uncertainty_decode_v1,
    verify_self_formed_closure_independently_v1, verify_self_formed_private_safety_v1,
};

use super::fixture::{R7Fixture, root_hash};

pub(super) struct TwoProbeHarness {
    pub planner_request: K2UncertaintyClosurePlannerRequestV1,
    pub census: K2UncertaintyClosureCensusV1,
    pub verification_request: K2UncertaintyClosureVerificationRequestV1,
    pub case_preverification: K2UncertaintyCasePreverificationV2,
    pub dispatch: K2UncertaintyPlanDispatchV2,
}

pub fn run() {
    let fixture = R7Fixture::new();
    let representatives = representative_dispositions(&fixture);
    let actual_request = closure_request(&fixture, representatives.clone());
    let actual =
        plan_self_formed_uncertainty_closure_v1(&actual_request).expect("actual closure census");
    let expected_candidates = if actual.completion_required {
        actual.representative_count.saturating_sub(1)
    } else {
        0
    };
    assert_eq!(actual.candidate_count, expected_candidates);
    match actual.disposition {
        K2UncertaintyClosureDispositionV1::SingleProbe => {
            assert_eq!(actual.first_partition_sizes, [1, 1, 1, 1]);
            assert!(actual.selected_second_probe_root_sha256.is_none());
        }
        K2UncertaintyClosureDispositionV1::TwoProbe => {
            assert_eq!(
                actual.selected_joint_partition_sizes,
                Some(vec![1, 1, 1, 1])
            );
        }
        K2UncertaintyClosureDispositionV1::ClosureUnavailable => {
            panic!("development case has no bounded closure")
        }
    }
    let verifier = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-closure-verifier"));
    let verifier_sha256 = composition_sha256_file_v1(&verifier).expect("closure verifier sha");
    let (verification_request, verification_receipt, closure_plan) =
        verify_closure(&actual_request, &actual, verifier_sha256.clone());
    let process_receipt = run_verifier_process(&verifier, &verification_request);
    assert_eq!(verification_receipt, process_receipt);
    let case_v2 = K2UncertaintyCasePreverificationV2::seal(
        fixture.preverification.clone(),
        verification_request.clone(),
        verification_receipt.clone(),
        Some(closure_plan),
    )
    .expect("case V2 preverification");
    case_v2
        .validate()
        .expect("validate case V2 preverification");

    let mut tampered = actual.clone();
    tampered.first_partition_sizes.reverse();
    if tampered.first_partition_sizes == actual.first_partition_sizes {
        tampered.completion_required = !tampered.completion_required;
    }
    let tampered_request = K2UncertaintyClosureVerificationRequestV1::seal(
        verifier_sha256.clone(),
        actual_request.clone(),
        tampered,
    )
    .expect("structurally bound tampered census request");
    assert_error(
        verify_self_formed_closure_independently_v1(&tampered_request),
        "self_formed_closure_verification_census_mismatch",
        "self_formed_closure_verification_census_mismatch",
    );
    verify_verifier_source_independence();

    let manifests = distinct_manifests(&representatives);
    verify_single_probe(&fixture, &representatives, &manifests);
    let two_probe = build_two_probe_harness(&fixture, &verifier_sha256);
    verify_case_journal(&fixture, two_probe.dispatch);
    verify_unavailable_and_omission_rejection(
        &fixture,
        &representatives,
        &manifests,
        &verifier_sha256,
    );
}

pub(super) fn build_two_probe_harness(
    fixture: &R7Fixture,
    verifier_sha256: &str,
) -> TwoProbeHarness {
    let representatives = representative_dispositions(fixture);
    let manifests = distinct_manifests(&representatives);
    verify_two_probe_and_order_invariance(fixture, &representatives, &manifests, verifier_sha256)
}

fn verify_single_probe(
    fixture: &R7Fixture,
    representatives: &[K2UncertaintyRawProbeDispositionV1],
    manifests: &[K2CompositionTreeManifestV1],
) {
    let mut rewritten = representatives.to_vec();
    let first = first_representative_mut(fixture, &mut rewritten);
    rewrite_partition(first, [0, 1, 2, 3], manifests);
    let census = plan_self_formed_uncertainty_closure_v1(&closure_request(fixture, rewritten))
        .expect("single-probe closure census");
    assert_eq!(
        census.disposition,
        K2UncertaintyClosureDispositionV1::SingleProbe
    );
    assert_eq!(census.first_partition_sizes, [1, 1, 1, 1]);
    assert!(!census.completion_required);
    assert_eq!(census.candidate_count, 0);
    assert!(census.candidates.is_empty());
    assert!(census.selected_second_probe_root_sha256.is_none());
}

pub(super) fn verify_two_probe_and_order_invariance(
    fixture: &R7Fixture,
    representatives: &[K2UncertaintyRawProbeDispositionV1],
    manifests: &[K2CompositionTreeManifestV1],
    verifier_sha256: &str,
) -> TwoProbeHarness {
    let first_root = first_probe_root(fixture);
    let second_root = representatives
        .iter()
        .map(|value| &value.probe.probe_root_sha256)
        .find(|root| *root != first_root)
        .expect("second representative")
        .clone();
    let mut rewritten = representatives.to_vec();
    for representative in &mut rewritten {
        let groups = if representative.probe.probe_root_sha256 == second_root {
            [0, 1, 0, 1]
        } else {
            [0, 0, 1, 1]
        };
        rewrite_partition(representative, groups, manifests);
    }
    let request = closure_request(fixture, rewritten.clone());
    let census =
        plan_self_formed_uncertainty_closure_v1(&request).expect("two-probe closure census");
    assert_eq!(
        census.disposition,
        K2UncertaintyClosureDispositionV1::TwoProbe
    );
    assert_eq!(
        census.selected_second_probe_root_sha256,
        Some(second_root.clone())
    );
    assert_eq!(
        census.selected_joint_partition_sizes,
        Some(vec![1, 1, 1, 1])
    );
    assert_eq!(
        census.candidate_count,
        census.representative_count.saturating_sub(1)
    );
    let (verification_request, verification_receipt, plan) =
        verify_closure(&request, &census, verifier_sha256.to_owned());
    assert_eq!(plan.plan_length, 2);
    assert_eq!(plan.ordered_probe_roots_sha256[1], second_root);

    rewritten.reverse();
    let reordered = plan_self_formed_uncertainty_closure_v1(&closure_request(fixture, rewritten))
        .expect("order-invariant closure census");
    assert_eq!(census.census_root_sha256, reordered.census_root_sha256);
    assert_eq!(
        census.candidate_denominator_root_sha256,
        reordered.candidate_denominator_root_sha256
    );
    let case_preverification = K2UncertaintyCasePreverificationV2::seal(
        fixture.preverification.clone(),
        verification_request.clone(),
        verification_receipt.clone(),
        Some(plan.clone()),
    )
    .expect("two-probe case preverification");
    let dispatch = dispatch_for_plan(fixture, &request, &plan);
    TwoProbeHarness {
        planner_request: request,
        census,
        verification_request,
        case_preverification,
        dispatch,
    }
}

fn verify_unavailable_and_omission_rejection(
    fixture: &R7Fixture,
    representatives: &[K2UncertaintyRawProbeDispositionV1],
    manifests: &[K2CompositionTreeManifestV1],
    verifier_sha256: &str,
) {
    let mut rewritten = representatives.to_vec();
    for representative in &mut rewritten {
        rewrite_partition(representative, [0, 0, 1, 1], manifests);
    }
    let mut census = plan_self_formed_uncertainty_closure_v1(&closure_request(fixture, rewritten))
        .expect("unavailable closure census");
    assert_eq!(
        census.disposition,
        K2UncertaintyClosureDispositionV1::ClosureUnavailable
    );
    assert!(census.selected_second_probe_root_sha256.is_none());
    assert!(census.selected_joint_partition_sizes.is_none());
    assert_eq!(
        census.candidate_count,
        census.representative_count.saturating_sub(1)
    );
    let planner_request = closure_request(
        fixture,
        representatives_with_partition(representatives, [0, 0, 1, 1], manifests),
    );
    let verification_request = K2UncertaintyClosureVerificationRequestV1::seal(
        verifier_sha256.to_owned(),
        planner_request.clone(),
        census.clone(),
    )
    .expect("unavailable verification request");
    let verification = verify_self_formed_closure_independently_v1(&verification_request)
        .expect("independently verify unavailable census");
    assert_error(
        K2UncertaintyClosurePlanV1::seal(&planner_request, &census, &verification),
        "self_formed_closure_plan_unavailable",
        "self_formed_closure_plan_unavailable",
    );

    census.candidates.pop().expect("candidate to omit");
    census.candidate_count = census.candidate_count.saturating_sub(1);
    assert_error(
        census.reseal(),
        "self_formed_closure_candidates_not_canonical",
        "self_formed_closure_census_invalid",
    );
}

fn verify_closure(
    planner_request: &K2UncertaintyClosurePlannerRequestV1,
    census: &K2UncertaintyClosureCensusV1,
    verifier_sha256: String,
) -> (
    K2UncertaintyClosureVerificationRequestV1,
    K2UncertaintyClosureVerificationReceiptV1,
    K2UncertaintyClosurePlanV1,
) {
    let request = K2UncertaintyClosureVerificationRequestV1::seal(
        verifier_sha256,
        planner_request.clone(),
        census.clone(),
    )
    .expect("closure verification request");
    uncertainty_bytes_v1(&request).expect("closure request protocol budget");
    let receipt = verify_self_formed_closure_independently_v1(&request)
        .expect("independent closure verification");
    let plan = K2UncertaintyClosurePlanV1::seal(planner_request, census, &receipt)
        .expect("immutable closure plan");
    (request, receipt, plan)
}

fn run_verifier_process(
    executable: &Path,
    request: &K2UncertaintyClosureVerificationRequestV1,
) -> K2UncertaintyClosureVerificationReceiptV1 {
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn closure verifier");
    child
        .stdin
        .take()
        .expect("closure verifier stdin")
        .write_all(&uncertainty_bytes_v1(request).expect("closure verifier input"))
        .expect("write closure verifier input");
    let output = child.wait_with_output().expect("wait for closure verifier");
    assert!(
        output.status.success(),
        "closure verifier failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    uncertainty_decode_v1(&output.stdout).expect("decode closure verifier receipt")
}

fn verify_verifier_source_independence() {
    let source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "src/k2_goal_environment/learned_composition/self_formed_uncertainty/closure_verifier.rs",
    ))
    .expect("read closure verifier source");
    for forbidden in [
        "plan_self_formed_uncertainty_closure_v1",
        "compare_completion_candidates_v1",
        "closure_partition_sizes_v1",
        "closure_probe_eligible_v1",
    ] {
        assert!(
            !source.contains(forbidden),
            "closure verifier imports planner helper {forbidden}"
        );
    }
}

fn dispatch_for_plan(
    fixture: &R7Fixture,
    planner_request: &K2UncertaintyClosurePlannerRequestV1,
    plan: &K2UncertaintyClosurePlanV1,
) -> K2UncertaintyPlanDispatchV2 {
    let grammar_root =
        self_formed_grammar_root_v1(&fixture.public_case.vocabulary).expect("private grammar root");
    let mut items = Vec::new();
    for (ordinal, probe_root) in plan.ordered_probe_roots_sha256.iter().enumerate() {
        let probe = planner_request
            .representatives
            .iter()
            .find(|value| &value.probe.probe_root_sha256 == probe_root)
            .expect("planned probe")
            .probe
            .clone();
        let effect = fixture
            .private_case
            .mapping
            .iter()
            .find(|value| value.opaque_action_root_sha256 == probe.action_id_sha256)
            .expect("private planned effect")
            .effect
            .clone();
        let safety_request = K2UncertaintySafetyRequestV1::seal(
            plan.plan_root_sha256.clone(),
            probe.clone(),
            effect.clone(),
            fixture.public_case.vocabulary.clone(),
            grammar_root.clone(),
            root_hash("r7c-sandbox"),
            root_hash("r7c-safety"),
        )
        .expect("plan safety request");
        let safety_receipt =
            verify_self_formed_private_safety_v1(&safety_request).expect("plan safety receipt");
        let worker_request = K2InquiryWorkerRequestV1::seal(
            plan.case_id_sha256.clone(),
            plan.plan_root_sha256.clone(),
            probe.probe_root_sha256.clone(),
            probe.action_id_sha256.clone(),
            root_hash("r7c-worker"),
            probe.initial_manifest.clone(),
            effect,
        )
        .expect("plan worker request");
        let observer_request = K2InquiryObserverRequestV1::seal(
            plan.case_id_sha256.clone(),
            probe.probe_root_sha256.clone(),
            root_hash("r7c-observer"),
        )
        .expect("plan observer request");
        let workspace_identity = K2UncertaintyWorkspaceIdentityV2::seal(
            plan.case_id_sha256.clone(),
            plan.plan_root_sha256.clone(),
            ordinal as u64,
        )
        .expect("workspace identity");
        let mut item = K2UncertaintyProbeDispatchItemV2 {
            schema: K2_UNCERTAINTY_PROBE_DISPATCH_ITEM_SCHEMA_V2.to_owned(),
            case_id_sha256: plan.case_id_sha256.clone(),
            closure_plan_root_sha256: plan.plan_root_sha256.clone(),
            probe_ordinal: ordinal as u64,
            initial_manifest_root_sha256: probe.initial_manifest.tree_root_sha256.clone(),
            selected_probe: probe,
            safety_request,
            safety_receipt,
            worker_request,
            observer_request,
            workspace_identity,
            authority: K2CompositionAuthorityBoundaryV1::denied(),
            item_root_sha256: String::new(),
        };
        item.reseal().expect("dispatch item");
        items.push(item);
    }
    assert_eq!(items.len(), 2);
    assert_ne!(
        items[0].workspace_identity.identity_root_sha256,
        items[1].workspace_identity.identity_root_sha256
    );
    let workspace_roots = items
        .iter()
        .map(|value| value.workspace_identity.identity_root_sha256.as_str())
        .collect::<Vec<_>>();
    let workspace_denominator_root_sha256 = composition_root_v1(&(
        "nando.k2-self-formed-workspace-denominator.v2",
        &plan.plan_root_sha256,
        workspace_roots,
    ))
    .expect("workspace denominator");
    let mut dispatch = K2UncertaintyPlanDispatchV2 {
        schema: K2_UNCERTAINTY_PLAN_DISPATCH_SCHEMA_V2.to_owned(),
        batch_precommit_root_sha256: root_hash("r7c-batch"),
        case_preverification_root_sha256: root_hash("r7c-case"),
        closure_plan: plan.clone(),
        items,
        workspace_denominator_root_sha256,
        all_requests_precommitted: true,
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        dispatch_root_sha256: String::new(),
    };
    dispatch.reseal().expect("plan dispatch");
    dispatch
}

fn verify_case_journal(fixture: &R7Fixture, dispatch: K2UncertaintyPlanDispatchV2) {
    let journal_root = fixture.root.join("r7c-happy-journal");
    let mut journal = K2UncertaintyCaseJournalV2::create(&journal_root, dispatch.clone())
        .expect("create R7C journal");
    assert_error(
        journal.freeze_observation_vector(
            root_hash("r7c-coordinator"),
            root_hash("r7c-vector-request-early"),
            root_hash("r7c-vector-early"),
            K2UncertaintyCaseJournalFaultV2::None,
        ),
        "self_formed_case_journal_observation_vector_order_v2_invalid",
        "self_formed_case_journal_observation_vector_order_v2_invalid",
    );
    journal
        .record_plan_dispatch(
            root_hash("r7c-coordinator"),
            K2UncertaintyCaseJournalFaultV2::None,
        )
        .expect("record whole plan dispatch");
    for ordinal in 0..dispatch.closure_plan.plan_length {
        let permit = journal
            .begin_probe_execution(ordinal, K2UncertaintyCaseJournalFaultV2::None)
            .expect("begin planned execution");
        journal
            .record_probe_observation(
                permit,
                root_hash(&format!("r7c-observation-{ordinal}")),
                K2UncertaintyCaseJournalFaultV2::None,
            )
            .expect("freeze planned observation");
    }
    assert_eq!(
        journal.projection().expect("happy projection").phase,
        K2UncertaintyCaseJournalPhaseV2::ReadyForObservationVector
    );
    journal
        .freeze_observation_vector(
            root_hash("r7c-coordinator"),
            root_hash("r7c-vector-request"),
            root_hash("r7c-vector"),
            K2UncertaintyCaseJournalFaultV2::None,
        )
        .expect("freeze observation vector");
    assert_error(
        journal.freeze_cleanup(
            root_hash("r7c-coordinator"),
            root_hash("r7c-cleanup-request-early"),
            root_hash("r7c-cleanup-receipt-early"),
            K2UncertaintyCaseJournalFaultV2::None,
        ),
        "self_formed_case_journal_cleanup_order_v2_invalid",
        "self_formed_case_journal_cleanup_order_v2_invalid",
    );
    journal
        .record_case_terminal(
            root_hash("r7c-final-verifier"),
            root_hash("r7c-final-request"),
            root_hash("r7c-final-receipt"),
            K2UncertaintyCaseJournalFaultV2::None,
        )
        .expect("record case terminal");
    journal
        .record_models_updated(
            root_hash("r7c-coordinator"),
            root_hash("r7c-outer-model-update"),
            root_hash("r7c-final-receipt"),
            K2UncertaintyCaseJournalFaultV2::None,
        )
        .expect("record models updated");
    journal
        .freeze_cleanup(
            root_hash("r7c-coordinator"),
            root_hash("r7c-cleanup-request"),
            root_hash("r7c-cleanup-receipt"),
            K2UncertaintyCaseJournalFaultV2::None,
        )
        .expect("freeze cleanup");
    let reopened = K2UncertaintyCaseJournalV2::reopen(&journal_root).expect("reopen happy journal");
    assert_eq!(journal.state(), reopened.state());
    assert_eq!(
        reopened.projection().expect("reopened projection").phase,
        K2UncertaintyCaseJournalPhaseV2::CleanupFrozen
    );

    let before_root = fixture.root.join("r7c-before-rename-journal");
    let mut before = K2UncertaintyCaseJournalV2::create(&before_root, dispatch.clone())
        .expect("create before-rename journal");
    assert_error(
        before.record_plan_dispatch(
            root_hash("r7c-coordinator"),
            K2UncertaintyCaseJournalFaultV2::BeforeRename,
        ),
        "self_formed_case_journal_v2_fault_before_rename",
        "self_formed_case_journal_v2_fault_before_rename",
    );
    let before_reopened =
        K2UncertaintyCaseJournalV2::reopen(&before_root).expect("reopen before-rename journal");
    assert_eq!(
        before_reopened
            .projection()
            .expect("before-rename projection")
            .phase,
        K2UncertaintyCaseJournalPhaseV2::AwaitingPlanDispatch
    );

    let after_root = fixture.root.join("r7c-after-rename-journal");
    let mut after = K2UncertaintyCaseJournalV2::create(&after_root, dispatch)
        .expect("create after-rename journal");
    after
        .record_plan_dispatch(
            root_hash("r7c-coordinator"),
            K2UncertaintyCaseJournalFaultV2::None,
        )
        .expect("record crash plan dispatch");
    assert_error(
        after.begin_probe_execution(0, K2UncertaintyCaseJournalFaultV2::AfterRename),
        "self_formed_case_journal_v2_fault_after_rename",
        "self_formed_case_journal_v2_fault_after_rename",
    );
    let mut after_reopened =
        K2UncertaintyCaseJournalV2::reopen(&after_root).expect("reopen after-rename journal");
    assert_eq!(
        after_reopened
            .projection()
            .expect("after-rename projection")
            .phase,
        K2UncertaintyCaseJournalPhaseV2::IndeterminateExecution { probe_ordinal: 0 }
    );
    assert_error(
        after_reopened.begin_probe_execution(0, K2UncertaintyCaseJournalFaultV2::None),
        "self_formed_case_journal_probe_redispatch_v2",
        "self_formed_case_journal_probe_redispatch_v2",
    );
    after_reopened
        .freeze_indeterminate_execution(
            root_hash("r7c-coordinator"),
            root_hash("r7c-indeterminate-terminal"),
            K2UncertaintyCaseJournalFaultV2::None,
        )
        .expect("freeze indeterminate terminal");
    assert_eq!(
        K2UncertaintyCaseJournalV2::reopen(&after_root)
            .expect("reopen terminal journal")
            .projection()
            .expect("terminal projection")
            .phase,
        K2UncertaintyCaseJournalPhaseV2::IndeterminateTerminal { probe_ordinal: 0 }
    );
}

fn representatives_with_partition(
    representatives: &[K2UncertaintyRawProbeDispositionV1],
    groups: [usize; 4],
    manifests: &[K2CompositionTreeManifestV1],
) -> Vec<K2UncertaintyRawProbeDispositionV1> {
    let mut rewritten = representatives.to_vec();
    for representative in &mut rewritten {
        rewrite_partition(representative, groups, manifests);
    }
    rewritten
}

fn closure_request(
    fixture: &R7Fixture,
    representatives: Vec<K2UncertaintyRawProbeDispositionV1>,
) -> K2UncertaintyClosurePlannerRequestV1 {
    K2UncertaintyClosurePlannerRequestV1::seal(
        fixture.public_case.vocabulary.case_id_sha256.clone(),
        fixture.probe_output.frontier.frontier_root_sha256.clone(),
        fixture.tournament.tournament.tournament_root_sha256.clone(),
        first_probe_root(fixture).clone(),
        representatives,
        root_hash("closure-planner"),
    )
    .expect("closure planner request")
}

fn representative_dispositions(fixture: &R7Fixture) -> Vec<K2UncertaintyRawProbeDispositionV1> {
    let expected = fixture
        .probe_output
        .frontier
        .representative_probe_roots_sha256
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut representatives = fixture
        .probe_output
        .pages
        .iter()
        .flat_map(|page| &page.dispositions)
        .filter(|value| expected.contains(&value.probe.probe_root_sha256))
        .cloned()
        .collect::<Vec<_>>();
    representatives.sort_by(|left, right| {
        left.probe
            .probe_root_sha256
            .cmp(&right.probe.probe_root_sha256)
    });
    assert_eq!(representatives.len(), expected.len());
    representatives
}

fn distinct_manifests(
    representatives: &[K2UncertaintyRawProbeDispositionV1],
) -> Vec<K2CompositionTreeManifestV1> {
    let manifests = representatives
        .iter()
        .flat_map(|value| &value.predictions)
        .map(|prediction| {
            (
                prediction.predicted_post_manifest.tree_root_sha256.clone(),
                prediction.predicted_post_manifest.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .take(4)
        .collect::<Vec<_>>();
    assert_eq!(manifests.len(), 4, "four distinct public outcomes required");
    manifests
}

fn rewrite_partition(
    disposition: &mut K2UncertaintyRawProbeDispositionV1,
    groups: [usize; 4],
    manifests: &[K2CompositionTreeManifestV1],
) {
    for (prediction, group) in disposition.predictions.iter_mut().zip(groups) {
        prediction.predicted_post_manifest = manifests[group].clone();
        prediction.observable_outcome_root_sha256 = composition_root_v1(&(
            "nando.k2-inquiry-observable-exact-manifest.v1",
            &prediction.predicted_post_manifest,
        ))
        .expect("observable root");
        prediction.reseal().expect("reseal prediction");
    }
    disposition.equivalence_key.pairwise_outcome_equal = [
        groups[0] == groups[1],
        groups[0] == groups[2],
        groups[0] == groups[3],
        groups[1] == groups[2],
        groups[1] == groups[3],
        groups[2] == groups[3],
    ];
    disposition
        .equivalence_key
        .reseal()
        .expect("reseal equivalence key");
    disposition.reseal().expect("reseal disposition");
}

fn first_representative_mut<'a>(
    fixture: &R7Fixture,
    representatives: &'a mut [K2UncertaintyRawProbeDispositionV1],
) -> &'a mut K2UncertaintyRawProbeDispositionV1 {
    let first = first_probe_root(fixture);
    representatives
        .iter_mut()
        .find(|value| &value.probe.probe_root_sha256 == first)
        .expect("first representative")
}

fn first_probe_root(fixture: &R7Fixture) -> &String {
    &fixture
        .tournament
        .tournament
        .tournament_winner_probe_root_sha256
}

fn assert_error<T>(result: Result<T, K2CompositionErrorV1>, primary: &str, fallback: &str) {
    let error = result.err().expect("invalid closure census accepted");
    let message = error.to_string();
    assert!(
        message.contains(primary) || message.contains(fallback),
        "wrong error: {error}"
    );
}
