use std::collections::BTreeSet;

use nando_operator_kernel::{
    ExecutableProtocolModeArtifactV3, canonical_json_sha256,
    validate_executable_protocol_mode_artifact_v3,
};
use serde::Serialize;

use super::dispatch_index::StructuralDispatchBitIndexV3;
use super::encoding::compile_mode_graph_v3;
use super::{
    CompiledProtocolModeV3, F5C_MAX_INDEXED_MODES_V3, ModeToRoleErrorV3, StructuralDispatchIndexV3,
};

#[derive(Serialize)]
struct CompiledModeCommitmentV3<'a> {
    artifact_root_sha256: &'a str,
    mode_id_sha256: &'a str,
    executable_mode_root_sha256: &'a str,
    payload_root_sha256: &'a str,
    effect_law_id_sha256: &'a str,
    source_value_type: nando_operator_kernel::BindingValueTypeV1,
    source_roles: &'a [(u16, nando_operator_kernel::BindingValueTypeV1)],
    capability_kind: nando_operator_kernel::ProtocolCapabilityKindV3,
    capability_argument_types: &'a [nando_operator_kernel::BindingValueTypeV1],
    capability_arguments: &'a [nando_operator_kernel::ProtocolCapabilityArgumentV3],
    role_graph_sha256: String,
    relation_program_fingerprint64: u64,
    constraints: &'a [super::constraint::CompiledConstraintV3],
}

pub fn compile_structural_dispatch_index_v3(
    artifacts: &[ExecutableProtocolModeArtifactV3],
) -> Result<StructuralDispatchIndexV3, ModeToRoleErrorV3> {
    let mut artifacts = artifacts.iter().collect::<Vec<_>>();
    artifacts.sort_by(|left, right| left.artifact_sha256().cmp(right.artifact_sha256()));
    let mut artifact_roots = BTreeSet::new();
    let mut mode_ids = BTreeSet::new();
    let mut modes = Vec::new();

    for artifact in artifacts {
        if !artifact_roots.insert(artifact.artifact_sha256()) {
            return Err(ModeToRoleErrorV3::DuplicateArtifact);
        }
        validate_artifact(artifact)?;
        for (executable, mode) in artifact
            .modes()
            .iter()
            .zip(artifact.source_mode_set().modes.iter())
        {
            if executable.source_mode_id_sha256() != mode.mode_id_sha256
                || !mode_ids.insert(mode.mode_id_sha256.as_str())
            {
                return Err(ModeToRoleErrorV3::DuplicateMode);
            }
            if modes.len() >= F5C_MAX_INDEXED_MODES_V3 {
                return Err(ModeToRoleErrorV3::IndexBudgetExhausted);
            }
            let source_value_type = mode
                .program
                .source_role_schema
                .roles
                .first()
                .ok_or(ModeToRoleErrorV3::InvalidSelector)?
                .value_type;
            let source_roles = mode
                .program
                .source_role_schema
                .roles
                .iter()
                .map(|role| (role.role_id, role.value_type))
                .collect::<Vec<_>>()
                .into_boxed_slice();
            let (role_graph, relation_program, constraints) = compile_mode_graph_v3(mode)?;
            modes.push(CompiledProtocolModeV3 {
                artifact_root_sha256: artifact.artifact_sha256().to_owned(),
                mode_id_sha256: mode.mode_id_sha256.clone(),
                executable_mode_root_sha256: executable.executable_mode_root_sha256().to_owned(),
                payload_root_sha256: executable.payload().payload_root_sha256().to_owned(),
                effect_law_id_sha256: mode.effect_law_id_sha256.clone(),
                action_class_root_sha256: mode.action_class_root_sha256.clone(),
                source_value_type,
                source_roles,
                capability_kind: executable.payload().capability_kind(),
                capability_argument_types: executable
                    .payload()
                    .arguments()
                    .iter()
                    .map(|argument| argument.value_type())
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                capability_arguments: executable.payload().arguments().to_vec().into_boxed_slice(),
                role_graph,
                relation_program,
                constraints,
            });
        }
    }

    let index_sha256 = index_digest(&modes)?;
    let dispatch_bits = StructuralDispatchBitIndexV3::build(&modes);
    Ok(StructuralDispatchIndexV3 {
        index_sha256,
        modes: modes.into_boxed_slice(),
        dispatch_bits,
    })
}

fn validate_artifact(artifact: &ExecutableProtocolModeArtifactV3) -> Result<(), ModeToRoleErrorV3> {
    validate_executable_protocol_mode_artifact_v3(artifact)
        .map_err(|_| ModeToRoleErrorV3::InvalidArtifact)?;
    let bytes = artifact
        .canonical_bytes()
        .map_err(|_| ModeToRoleErrorV3::Serialization)?;
    let decoded =
        ExecutableProtocolModeArtifactV3::from_canonical_bytes(&bytes, artifact.artifact_sha256())
            .map_err(|_| ModeToRoleErrorV3::InvalidArtifact)?;
    if decoded != *artifact {
        return Err(ModeToRoleErrorV3::InvalidArtifact);
    }
    Ok(())
}

fn index_digest(modes: &[CompiledProtocolModeV3]) -> Result<String, ModeToRoleErrorV3> {
    let material = modes
        .iter()
        .map(|mode| {
            Ok(CompiledModeCommitmentV3 {
                artifact_root_sha256: &mode.artifact_root_sha256,
                mode_id_sha256: &mode.mode_id_sha256,
                executable_mode_root_sha256: &mode.executable_mode_root_sha256,
                payload_root_sha256: &mode.payload_root_sha256,
                effect_law_id_sha256: &mode.effect_law_id_sha256,
                source_value_type: mode.source_value_type,
                source_roles: &mode.source_roles,
                capability_kind: mode.capability_kind,
                capability_argument_types: &mode.capability_argument_types,
                capability_arguments: &mode.capability_arguments,
                role_graph_sha256: role_graph_digest(mode)?,
                relation_program_fingerprint64: mode.relation_program.fingerprint64(),
                constraints: &mode.constraints,
            })
        })
        .collect::<Result<Vec<_>, ModeToRoleErrorV3>>()?;
    canonical_json_sha256(&("nando.f5c.structural-dispatch-index.v3", material))
        .map_err(|_| ModeToRoleErrorV3::Serialization)
}

fn role_graph_digest(mode: &CompiledProtocolModeV3) -> Result<String, ModeToRoleErrorV3> {
    let roles = mode
        .role_graph
        .canonical_roles()
        .iter()
        .map(|role| {
            (
                role.type_class(),
                role.cardinality_class(),
                role.temporal_position(),
                role.constraint_mask(),
                role.neighboring_relation_planes(),
            )
        })
        .collect::<Vec<_>>();
    canonical_json_sha256(&roles).map_err(|_| ModeToRoleErrorV3::Serialization)
}
