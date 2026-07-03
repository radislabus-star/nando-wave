use nando_core::{
    WavePredictorActiveCenter, WavePredictorHebbianConfig, WavePredictorHebbianField,
    WavePredictorRoleBindingEvalTask, WavePredictorRoleBindingOffloadAction,
    WavePredictorRoleBindingOffloadPolicy, WavePredictorRoleBindingOffloadRuntime,
    WavePredictorRoleBindingPackageError,
};

const ACTION_BASE: u32 = 4_096;
const ROLE_BASE: u32 = 8_192;
const ROLE_STRIDE: u32 = 4_096;
const ACTION_CENTER: u32 = ACTION_BASE + 7;
const LANE_ID: u16 = 123;
const WRONG_LANE_ID: u16 = 124;
const OUTPUT_SLOT: u8 = 2;
const SOURCE_SLOT: u8 = 1;

fn role_center(slot_id: u8, lane_id: u16) -> u32 {
    ROLE_BASE + u32::from(slot_id) * ROLE_STRIDE + u32::from(lane_id)
}

fn build_public_sdk_fixture() -> (Vec<u8>, Vec<WavePredictorActiveCenter>) {
    let config = WavePredictorHebbianConfig {
        state_delta_binding_action_base: Some(ACTION_BASE),
        state_delta_binding_action_count: 4_096,
        state_delta_binding_role_base: Some(ROLE_BASE),
        state_delta_binding_role_stride: ROLE_STRIDE,
        state_delta_binding_role_count: 4,
        weight_limit: 1_024,
        ..WavePredictorHebbianConfig::default()
    };
    let mut field =
        WavePredictorHebbianField::new(ROLE_BASE as usize + ROLE_STRIDE as usize * 4, config);
    let active_fringe = vec![
        WavePredictorActiveCenter {
            center_id: ACTION_CENTER,
            strength: 3,
        },
        WavePredictorActiveCenter {
            center_id: role_center(SOURCE_SLOT, LANE_ID),
            strength: 2,
        },
    ];

    let changed =
        field.adjust_state_delta_role_binding(LANE_ID, 1, &active_fringe, Some(OUTPUT_SLOT), 7);
    assert_eq!(changed, 1);

    let package_bytes = field
        .compile_flat_role_binding_table()
        .to_bytes()
        .expect("role-binding package serializes");
    (package_bytes, active_fringe)
}

#[test]
fn public_sdk_loads_role_binding_package_and_routes_local_vs_fallback() {
    let (package_bytes, active_fringe) = build_public_sdk_fixture();
    let inspected = WavePredictorRoleBindingOffloadRuntime::inspect_package_bytes(&package_bytes)
        .expect("sdk inspects package bytes");
    assert_eq!(inspected.serialized_len, package_bytes.len());
    assert_eq!(inspected.edge_count, 1);
    assert!(inspected.fingerprint64 != 0);

    let policy = WavePredictorRoleBindingOffloadPolicy::new(1).expect("valid policy");
    let sdk = WavePredictorRoleBindingOffloadRuntime::from_package_bytes(&package_bytes, policy)
        .expect("sdk loads package bytes");

    assert_eq!(sdk.policy(), policy);
    assert_eq!(sdk.package_info(), inspected);
    assert_eq!(sdk.edge_count(), 1);
    assert!(sdk.bytes_estimate() > 0);

    let local_task = WavePredictorRoleBindingEvalTask {
        target_lane_id: LANE_ID,
        target_signed_strength: 1,
        wrong_lane_id: WRONG_LANE_ID,
        wrong_signed_strength: 1,
        active_fringe: &active_fringe,
        binding_output_slot: Some(OUTPUT_SLOT),
        expect_local_operator: true,
    };
    let fallback_task = WavePredictorRoleBindingEvalTask {
        target_lane_id: WRONG_LANE_ID,
        target_signed_strength: 1,
        wrong_lane_id: LANE_ID,
        wrong_signed_strength: 1,
        active_fringe: &active_fringe,
        binding_output_slot: Some(OUTPUT_SLOT),
        expect_local_operator: false,
    };
    let tasks = vec![local_task, fallback_task];

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
    assert_eq!(margins.len(), 2);
    assert!(margins[0] > 0);
    assert!(margins[1] < 0);
    assert_eq!(
        decisions[0].action,
        WavePredictorRoleBindingOffloadAction::LocalOperator
    );
    assert_eq!(
        decisions[1].action,
        WavePredictorRoleBindingOffloadAction::FallbackToLlm
    );
    assert_eq!(summary.calls, 2);
    assert_eq!(summary.local_operator_calls, 1);
    assert_eq!(summary.fallback_to_llm_calls, 1);
    assert_eq!(summary.false_local_accepts, 0);
}

#[test]
fn public_sdk_rejects_invalid_role_binding_package_bytes_and_policy() {
    assert_eq!(
        WavePredictorRoleBindingOffloadPolicy::new(0),
        Err(WavePredictorRoleBindingPackageError::InvalidPolicy)
    );
    assert_eq!(
        WavePredictorRoleBindingOffloadRuntime::inspect_package_bytes(b"not-a-package"),
        Err(WavePredictorRoleBindingPackageError::InvalidRuntimePackage)
    );
    assert_eq!(
        WavePredictorRoleBindingOffloadRuntime::from_package_bytes(
            b"not-a-package",
            WavePredictorRoleBindingOffloadPolicy::default_conservative(),
        ),
        Err(WavePredictorRoleBindingPackageError::InvalidRuntimePackage)
    );
}
