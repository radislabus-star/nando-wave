use nando_core::{
    PHASE_CENTER_HOT_RUNTIME_PACKAGE_MAGIC, PhaseCenterCompiler, PhaseCenterEvalTask,
    PhaseCenterFlatRecord, PhaseCenterFlatRuntime, PhaseCenterHotPackagePolicyDefaults,
    PhaseCenterHotRequest, PhaseCenterHotRouteTable, PhaseCenterHotRuntime,
    PhaseCenterHotRuntimePackage, PhaseCenterOffloadAction, PhaseCenterOffloadPolicy,
    PhaseCenterOffloadRuntime, PhaseCenterRuntimeError, PhaseCenterVerifierBinding,
    phase_vector_from_atom_ids, phase_vector_from_atoms,
};

#[test]
fn public_sdk_loads_package_and_routes_local_vs_fallback() {
    let mut compiler = PhaseCenterCompiler::new(16, 1).expect("compiler builds");
    compiler
        .add_positive_atoms(0, ["action:normalize", "field:status", "write:canonical"])
        .expect("positive atoms");
    compiler
        .add_negative_atoms(0, ["action:normalize", "field:status", "write:wrong"])
        .expect("negative atoms");
    let package_bytes = compiler
        .compile()
        .expect("runtime compiles")
        .to_bytes()
        .expect("runtime serializes");

    let inspected =
        PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes).expect("sdk inspects");
    assert_eq!(inspected.cells, 16);
    assert_eq!(inspected.record_count, 1);
    assert_eq!(inspected.serialized_len, package_bytes.len());
    assert!(inspected.fingerprint64 != 0);

    let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
    let sdk = PhaseCenterOffloadRuntime::from_package_bytes(&package_bytes, policy)
        .expect("sdk loads package bytes");

    assert_eq!(sdk.policy(), policy);
    assert_eq!(sdk.package_info(), inspected);
    assert_eq!(sdk.cells(), 16);
    assert_eq!(sdk.record_count(), 1);
    assert!(sdk.bytes_estimate() > 0);

    let correct =
        phase_vector_from_atoms(["action:normalize", "field:status", "write:canonical"], 16);
    let wrong = phase_vector_from_atoms(["action:normalize", "field:status", "write:wrong"], 16);
    let tasks = vec![
        PhaseCenterEvalTask {
            center_index: 0,
            correct_vec: correct.clone().into_boxed_slice(),
            wrong_vec: wrong.clone().into_boxed_slice(),
        },
        PhaseCenterEvalTask {
            center_index: 0,
            correct_vec: wrong.into_boxed_slice(),
            wrong_vec: correct.into_boxed_slice(),
        },
    ];

    let mut decisions = Vec::with_capacity(4);
    let mut margins = Vec::with_capacity(4);
    let decision_capacity = decisions.capacity();
    let margin_capacity = margins.capacity();
    let summary = sdk
        .offload_summary_into(&tasks, &mut decisions, &mut margins)
        .expect("sdk routes tasks");

    assert_eq!(decisions.capacity(), decision_capacity);
    assert_eq!(margins.capacity(), margin_capacity);
    assert_eq!(decisions.len(), 2);
    assert_eq!(decisions[0].action, PhaseCenterOffloadAction::LocalOperator);
    assert_eq!(decisions[1].action, PhaseCenterOffloadAction::FallbackToLlm);
    assert_eq!(summary.calls, 2);
    assert_eq!(summary.local_operator_calls, 1);
    assert_eq!(summary.fallback_to_llm_calls, 1);
    assert_eq!(summary.false_local_accepts, 0);
}

#[test]
fn public_sdk_rejects_invalid_package_bytes() {
    let policy = PhaseCenterOffloadPolicy::default_conservative();
    assert_eq!(
        PhaseCenterOffloadRuntime::inspect_package_bytes(b"not-a-package"),
        Err(PhaseCenterRuntimeError::InvalidRuntimePackage)
    );
    assert_eq!(
        PhaseCenterOffloadRuntime::from_package_bytes(b"not-a-package", policy),
        Err(PhaseCenterRuntimeError::InvalidRuntimePackage)
    );
}

#[test]
fn public_sdk_loads_hot_package_into_numeric_worker() {
    let atom_ids = [42_u64, 7, 9];
    let wrong_atom_ids = [42_u64, 7, 99];
    let positive = phase_vector_from_atom_ids(atom_ids, 16);
    let negative = phase_vector_from_atom_ids(wrong_atom_ids, 16);
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.into_boxed_slice(),
            negative_center: negative.into_boxed_slice(),
        }],
    )
    .expect("valid flat runtime");
    let hot =
        PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1]).expect("hot runtime builds");
    let route_plan = hot
        .route_plan_from_profile_ids(11, [42])
        .expect("route plan builds")
        .expect("route plan exists");
    let route_table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("route table");
    let verifier_binding = PhaseCenterVerifierBinding {
        verifier_id: 11,
        verifier_version: 1,
        verifier_input_kind_id: 22,
        verifier_evidence_source_id: 33,
        false_accept_threshold: 0,
    };
    let policy_defaults = PhaseCenterHotPackagePolicyDefaults {
        local_accept_enabled: false,
        require_verifier: true,
        require_false_accepts_zero: true,
        shadow_only: true,
        min_margin_threshold_micro: 1,
    };
    let package = PhaseCenterHotRuntimePackage::from_runtime(
        hot,
        route_table,
        verifier_binding,
        policy_defaults,
    )
    .expect("hot package builds");
    let package_bytes = package.to_bytes().expect("hot package serializes");
    let inspected =
        PhaseCenterHotRuntimePackage::inspect_bytes(&package_bytes).expect("hot package inspects");
    assert_eq!(inspected.magic, PHASE_CENTER_HOT_RUNTIME_PACKAGE_MAGIC);
    assert_eq!(inspected.profile_count, 1);
    assert_eq!(inspected.route_count, 1);
    assert_eq!(inspected.route_profile_edges, 1);
    assert_ne!(inspected.fingerprint64, 0);
    assert!(inspected.hot_bytes_estimate > 0);
    assert!(!inspected.server_policy_allows_local_accept());

    let loaded =
        PhaseCenterHotRuntimePackage::from_bytes(&package_bytes).expect("hot package loads");
    assert_eq!(loaded.info, inspected);
    let mut worker = loaded.into_worker().expect("worker loads");
    let route_index = worker.resolve_route_index(11).expect("route index");
    let decisions = worker
        .score_atom_ids(PhaseCenterHotRequest::new(route_index, &atom_ids))
        .expect("worker scores");
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].profile_id, 42);
    assert!(decisions[0].score_candidate);
    assert!(decisions[0].verifier_required);
    assert!(!decisions[0].local_accept);
}
