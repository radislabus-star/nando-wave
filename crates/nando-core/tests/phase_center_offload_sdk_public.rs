use nando_core::{
    PhaseCenterCompiler, PhaseCenterEvalTask, PhaseCenterOffloadAction, PhaseCenterOffloadPolicy,
    PhaseCenterOffloadRuntime, PhaseCenterRuntimeError, phase_vector_from_atoms,
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
