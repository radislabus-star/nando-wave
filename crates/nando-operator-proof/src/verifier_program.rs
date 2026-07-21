use std::collections::BTreeMap;

use nando_operator_kernel::{
    ResponseArgument, ResponseOperation, ResponseProgram, SemanticRole, VerifierConsensusVariant,
    VerifierProgram,
};

pub(crate) fn source_neutral_verifier_for_program(
    program: &ResponseProgram,
) -> Result<VerifierProgram, &'static str> {
    match &program.operation {
        ResponseOperation::UniqueConsensus {
            variants,
            adapter_wave,
        } => Ok(VerifierProgram::UniqueConsensus {
            variants: variants
                .iter()
                .map(|variant| {
                    source_neutral_verifier_for_program(&variant.program).map(|verifier| {
                        VerifierConsensusVariant {
                            verifier,
                            allowed_layout_sha256: variant.allowed_layout_sha256.clone(),
                            required_request_atom_ids: variant.required_request_atom_ids.clone(),
                        }
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
            adapter_wave: adapter_wave.clone(),
        }),
        ResponseOperation::AdvancePlan { function_name } => Ok(VerifierProgram::AdvancePlan {
            function_name: function_name.clone(),
            require_explicit_tool_success: true,
            require_canonical_plan: true,
        }),
        ResponseOperation::FunctionCallFromRoles {
            function_name,
            selector,
            arguments,
        } => {
            let mut role_arguments = BTreeMap::new();
            let mut role_argument_types = BTreeMap::new();
            let mut integer_arguments = BTreeMap::new();
            let mut string_arguments = BTreeMap::new();
            let mut boolean_arguments = BTreeMap::new();
            for argument in arguments {
                match argument {
                    ResponseArgument::Role {
                        name,
                        role,
                        value_type,
                    } => {
                        role_arguments.insert(name.clone(), *role);
                        if let Some(value_type) = value_type {
                            role_argument_types.insert(name.clone(), *value_type);
                        }
                    }
                    ResponseArgument::Integer { name, value } => {
                        integer_arguments.insert(name.clone(), *value);
                    }
                    ResponseArgument::String { name, value } => {
                        string_arguments.insert(name.clone(), value.clone());
                    }
                    ResponseArgument::Boolean { name, value } => {
                        boolean_arguments.insert(name.clone(), *value);
                    }
                }
            }
            if role_arguments.is_empty() {
                return Err("source_neutral_call_verifier");
            }
            let pending = role_arguments
                .values()
                .any(|role| *role == SemanticRole::ContinuationHandle);
            Ok(VerifierProgram::FunctionCallFromRoles {
                function_name: function_name.clone(),
                selector: selector.clone(),
                role_arguments,
                role_argument_types,
                integer_arguments,
                string_arguments,
                boolean_arguments,
                require_pending_state: pending,
                require_unique_handle: pending,
            })
        }
        ResponseOperation::CustomToolCallFromRoles {
            custom_tool_name,
            inner_tool_name,
            selector,
            arguments,
            projection,
        } => Ok(VerifierProgram::CustomToolCallFromRoles {
            custom_tool_name: custom_tool_name.clone(),
            inner_tool_name: inner_tool_name.clone(),
            selector: selector.clone(),
            arguments: arguments.clone(),
            projection: projection.clone(),
            require_pending_state: true,
            require_unique_handle: true,
        }),
        ResponseOperation::ProjectSelectedValue {
            selector,
            format,
            renderer,
            completion_state,
        } => Ok(VerifierProgram::ProjectSelectedValue {
            selector: selector.clone(),
            format: *format,
            renderer: renderer.clone(),
            completion_state: completion_state.clone(),
            require_unique_value: true,
        }),
        ResponseOperation::ProjectStatus {
            selector,
            mapping,
            renderer,
            completion_state,
        } => Ok(VerifierProgram::ProjectStatus {
            selector: selector.clone(),
            mapping: *mapping,
            renderer: renderer.clone(),
            completion_state: completion_state.clone(),
            require_unique_value: true,
        }),
        ResponseOperation::ComposeCollection {
            steps,
            format,
            renderer,
            completion_state,
            max_items,
        } => Ok(VerifierProgram::ComposeCollection {
            steps: steps.clone(),
            format: *format,
            renderer: renderer.clone(),
            completion_state: completion_state.clone(),
            max_items: *max_items,
        }),
        ResponseOperation::CopyAfterPrefix { .. }
        | ResponseOperation::TestResultSummary { .. }
        | ResponseOperation::WaitOnYieldedCell { .. }
        | ResponseOperation::WaitOnAnyYieldedCell { .. }
        | ResponseOperation::WaitOnYieldedSurfaces { .. } => {
            Err("source_neutral_verifier_program_kind")
        }
    }
}
