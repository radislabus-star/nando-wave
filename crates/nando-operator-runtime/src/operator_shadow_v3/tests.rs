use nando_operator_kernel::{
    BoundProtocolActionInputV3, BoundProtocolArgumentInputV3, BoundProtocolValueV3,
    PROTOCOL_VM_HEADER_BYTES_V3, RuntimeCapabilityKindV3, build_bound_protocol_action_v3,
    canonical_json_sha256,
};

use super::bytecode::compile_vm_bytecode_v3;
use super::*;
use crate::{OperatorVmError, execute_protocol_vm_bytecode_v3};

fn root(label: &str) -> String {
    canonical_json_sha256(&label).expect("test root")
}

fn action(
    label: &str,
    capability_kind: RuntimeCapabilityKindV3,
    arguments: Vec<(&str, u16, BoundProtocolValueV3)>,
) -> nando_operator_kernel::BoundProtocolActionV3 {
    build_bound_protocol_action_v3(BoundProtocolActionInputV3 {
        index_sha256: root("index"),
        artifact_root_sha256: root("artifact"),
        mode_id_sha256: root("mode"),
        executable_mode_root_sha256: root("executable"),
        payload_root_sha256: root("payload"),
        effect_law_id_sha256: root("law"),
        action_class_root_sha256: root("action-class"),
        request_view_sha256: root("request"),
        mapping_sha256: root(label),
        capability_id: 7,
        capability_kind,
        physical_symbol: "continue_session".to_owned(),
        arguments: arguments
            .into_iter()
            .enumerate()
            .map(
                |(ordinal, (name, source_role_id, value))| BoundProtocolArgumentInputV3 {
                    argument_ordinal: ordinal as u16,
                    source_role_id,
                    physical_name: name.to_owned(),
                    value,
                },
            )
            .collect(),
    })
    .expect("bound action")
}

#[test]
fn actor_and_vm_execute_all_bound_value_types_with_exact_parity() {
    let action = action(
        "mapping-a",
        RuntimeCapabilityKindV3::Function,
        vec![
            ("boolean", 0, BoundProtocolValueV3::Boolean(true)),
            ("count", 1, BoundProtocolValueV3::Integer(17)),
            (
                "handle",
                2,
                BoundProtocolValueV3::Identifier("CellA17".to_owned()),
            ),
            (
                "message",
                3,
                BoundProtocolValueV3::String("continue".to_owned()),
            ),
        ],
    );
    let execution = execute_bound_protocol_shadow_v3(&action);

    assert_eq!(
        execution.receipt().verdict(),
        OperatorShadowVerdictV3::Complete
    );
    assert_eq!(execution.actor_output(), execution.vm_output());
    assert_eq!(
        execution.receipt().actor_output_sha256(),
        execution.receipt().vm_output_sha256()
    );
    assert!(execution.receipt().program_sha256().is_some());
    assert!(!execution.receipt().execution_authority());
    assert!(!execution.execution_authority());
}

#[test]
fn program_root_is_owned_by_the_selected_mapping() {
    let first = action(
        "mapping-a",
        RuntimeCapabilityKindV3::Function,
        vec![(
            "handle",
            0,
            BoundProtocolValueV3::String("CellA17".to_owned()),
        )],
    );
    let second = action(
        "mapping-b",
        RuntimeCapabilityKindV3::Function,
        vec![(
            "handle",
            0,
            BoundProtocolValueV3::String("CellA17".to_owned()),
        )],
    );
    assert_eq!(
        first.physical_action_sha256(),
        second.physical_action_sha256()
    );

    let first = execute_bound_protocol_shadow_v3(&first);
    let second = execute_bound_protocol_shadow_v3(&second);
    assert_ne!(
        first.receipt().program_sha256(),
        second.receipt().program_sha256()
    );
    assert_ne!(
        first.receipt().action_derivation_sha256(),
        second.receipt().action_derivation_sha256()
    );
}

#[test]
fn unknown_vm_opcode_fails_closed_without_actor_fallback() {
    let action = action(
        "unknown-opcode",
        RuntimeCapabilityKindV3::Function,
        vec![(
            "handle",
            0,
            BoundProtocolValueV3::String("CellA17".to_owned()),
        )],
    );
    let program =
        nando_operator_kernel::compile_bound_protocol_program_v3(&action).expect("bound program");
    let mut bytecode = compile_vm_bytecode_v3(&program)
        .expect("bytecode")
        .into_vec();
    bytecode[PROTOCOL_VM_HEADER_BYTES_V3] = 0x7e;

    assert_eq!(
        execute_protocol_vm_bytecode_v3(&program, &bytecode),
        Err(OperatorVmError::UnsupportedOpcode)
    );
}

#[test]
fn oversized_actor_output_abstains_before_vm_execution() {
    let action = action(
        "output-budget",
        RuntimeCapabilityKindV3::Function,
        vec![(
            "message",
            0,
            BoundProtocolValueV3::String("x".repeat(16_384)),
        )],
    );
    let execution = execute_bound_protocol_shadow_v3(&action);

    assert_eq!(
        execution.receipt().verdict(),
        OperatorShadowVerdictV3::AbstainProgramBudget
    );
    assert!(execution.vm_output().is_none());
    assert!(!execution.execution_authority());
}

#[test]
fn unsupported_custom_capability_abstains_without_program() {
    let action = action(
        "custom-tool",
        RuntimeCapabilityKindV3::Custom,
        vec![(
            "handle",
            0,
            BoundProtocolValueV3::String("CellA17".to_owned()),
        )],
    );
    let execution = execute_bound_protocol_shadow_v3(&action);

    assert_eq!(
        execution.receipt().verdict(),
        OperatorShadowVerdictV3::AbstainUnsupportedCapability
    );
    assert!(execution.program().is_none());
    assert!(execution.bytecode().is_none());
}

#[test]
fn truncated_bytecode_is_rejected() {
    let action = action(
        "truncated",
        RuntimeCapabilityKindV3::Function,
        vec![(
            "handle",
            0,
            BoundProtocolValueV3::String("CellA17".to_owned()),
        )],
    );
    let execution = execute_bound_protocol_shadow_v3(&action);
    let bytecode = execution.bytecode().expect("bytecode");

    assert_eq!(
        execute_protocol_vm_bytecode_v3(
            execution.program().expect("program"),
            &bytecode[..bytecode.len() - 1],
        ),
        Err(OperatorVmError::InvalidProgram)
    );
}

#[test]
fn bytecode_cannot_replace_its_externally_owned_program_root() {
    let action = action(
        "forged-root",
        RuntimeCapabilityKindV3::Function,
        vec![(
            "handle",
            0,
            BoundProtocolValueV3::String("CellA17".to_owned()),
        )],
    );
    let execution = execute_bound_protocol_shadow_v3(&action);
    let mut bytecode = execution.bytecode().expect("bytecode").to_vec();
    bytecode[48] ^= 0x01;

    assert_eq!(
        execute_protocol_vm_bytecode_v3(execution.program().expect("program"), &bytecode),
        Err(OperatorVmError::InvalidProgram)
    );
}
