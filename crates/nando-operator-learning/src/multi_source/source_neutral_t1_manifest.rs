use nando_operator_kernel::{
    AtomValueType, CollectionOutputRenderer, CollectionProgramStep, CollectionScalarType,
    MultiSourceContainerClassV1, MultiSourceTemporalClassV1, PreActionMultiSourceTopologyV1,
    ResponseOperation, ResponseProgram, ResponseRenderSegment, ResponseValueSelector,
    canonical_json_sha256,
};
use serde::{Deserialize, Serialize};

use super::source_neutral_t1_binding::{role_for_witness, role_type_matches, witness_for_selector};

const MANIFEST_SCHEMA_V1: &str = "nando.pre-action-t1-input-binding-manifest.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreActionT1SelectorOriginV1 {
    CollectionStep,
    Renderer,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PreActionT1ConsumedInputV1 {
    CollectionSource {
        program_output_ordinal: Option<u16>,
        topology_source_ordinal: u16,
        local_role_id: u16,
        frozen_witness_sha256: String,
    },
    SelectedValue {
        origin: PreActionT1SelectorOriginV1,
        selector: ResponseValueSelector,
        local_role_id: u16,
        topology_source_ordinal: u16,
        frozen_witness_sha256: String,
    },
    ImplicitRequestValue {
        value_type: CollectionScalarType,
        local_role_id: u16,
        topology_source_ordinal: u16,
        frozen_witness_sha256: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreActionT1InputBindingManifestV1 {
    pub schema: String,
    pub inputs: Vec<PreActionT1ConsumedInputV1>,
}

impl PreActionT1InputBindingManifestV1 {
    pub fn root_sha256(&self) -> Result<String, &'static str> {
        canonical_json_sha256(self).map_err(|_| "pre_action_binding_commitment_failed")
    }
}

pub fn pre_action_t1_input_binding_manifest_v1(
    program: &ResponseProgram,
    topology: &PreActionMultiSourceTopologyV1,
) -> Result<PreActionT1InputBindingManifestV1, &'static str> {
    let ResponseOperation::ComposeCollection {
        steps, renderer, ..
    } = &program.operation
    else {
        return Err("collection_program_required");
    };
    let program_output_ordinal = steps.iter().find_map(|step| match step {
        CollectionProgramStep::SelectTurnOutput { output_ordinal } => Some(*output_ordinal),
        _ => None,
    });
    let topology_source_ordinal = program_output_ordinal
        .map(|ordinal| {
            ordinal
                .checked_sub(1)
                .ok_or("collection_output_ordinal_invalid")
        })
        .transpose()?;
    let collection_roles = topology
        .roles
        .iter()
        .filter(|role| role.container_class != MultiSourceContainerClassV1::Scalar)
        .filter(|role| {
            topology_source_ordinal.map_or(
                role.temporal_class == MultiSourceTemporalClassV1::Latest,
                |ordinal| role.source_ordinal == ordinal,
            )
        })
        .collect::<Vec<_>>();
    let [collection_role] = collection_roles.as_slice() else {
        return Err("collection_role_missing_or_ambiguous");
    };
    let collection_witness = unique_role_witness(topology, collection_role.local_role_id)
        .ok_or("collection_role_witness_missing_or_ambiguous")?;
    let mut inputs = vec![PreActionT1ConsumedInputV1::CollectionSource {
        program_output_ordinal,
        topology_source_ordinal: collection_role.source_ordinal,
        local_role_id: collection_role.local_role_id,
        frozen_witness_sha256: collection_witness.value_sha256.clone(),
    }];

    for step in steps {
        match step {
            CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { selector, .. } => {
                inputs.push(selector_binding(
                    selector,
                    PreActionT1SelectorOriginV1::CollectionStep,
                    topology,
                )?);
            }
            CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { value_type } => {
                inputs.push(implicit_request_binding(*value_type, topology)?);
            }
            _ => {}
        }
    }
    if let CollectionOutputRenderer::RenderSequence { segments } = renderer {
        for segment in segments {
            if let ResponseRenderSegment::Selected { selector, .. } = segment {
                inputs.push(selector_binding(
                    selector,
                    PreActionT1SelectorOriginV1::Renderer,
                    topology,
                )?);
            }
        }
    }
    inputs.sort();
    inputs.dedup();
    Ok(PreActionT1InputBindingManifestV1 {
        schema: MANIFEST_SCHEMA_V1.to_owned(),
        inputs,
    })
}

fn selector_binding(
    selector: &ResponseValueSelector,
    origin: PreActionT1SelectorOriginV1,
    topology: &PreActionMultiSourceTopologyV1,
) -> Result<PreActionT1ConsumedInputV1, &'static str> {
    let witness = witness_for_selector(selector, topology)
        .ok_or("collection_selector_role_missing_or_ambiguous")?;
    let role = role_for_witness(topology, witness)
        .filter(|role| role.container_class == MultiSourceContainerClassV1::Scalar)
        .ok_or("collection_selector_role_not_scalar")?;
    Ok(PreActionT1ConsumedInputV1::SelectedValue {
        origin,
        selector: selector.clone(),
        local_role_id: role.local_role_id,
        topology_source_ordinal: role.source_ordinal,
        frozen_witness_sha256: witness.value_sha256.clone(),
    })
}

fn implicit_request_binding(
    value_type: CollectionScalarType,
    topology: &PreActionMultiSourceTopologyV1,
) -> Result<PreActionT1ConsumedInputV1, &'static str> {
    let atom_type = match value_type {
        CollectionScalarType::String => AtomValueType::String,
        CollectionScalarType::Integer => AtomValueType::Integer,
        CollectionScalarType::Boolean => AtomValueType::Boolean,
    };
    let matches = topology
        .role_witnesses
        .iter()
        .filter_map(|witness| {
            let role = role_for_witness(topology, witness)?;
            (role.container_class == MultiSourceContainerClassV1::Scalar
                && role_type_matches(role.type_class, atom_type)
                && (witness.request_reference_ordinal.is_some()
                    || !witness.request_reference_ordinal_candidates.is_empty()))
            .then_some((role, witness))
        })
        .collect::<Vec<_>>();
    let [(role, witness)] = matches.as_slice() else {
        return Err("collection_implicit_request_role_missing_or_ambiguous");
    };
    Ok(PreActionT1ConsumedInputV1::ImplicitRequestValue {
        value_type,
        local_role_id: role.local_role_id,
        topology_source_ordinal: role.source_ordinal,
        frozen_witness_sha256: witness.value_sha256.clone(),
    })
}

fn unique_role_witness(
    topology: &PreActionMultiSourceTopologyV1,
    local_role_id: u16,
) -> Option<&nando_operator_kernel::MultiSourceRoleWitnessV1> {
    let mut witnesses = topology
        .role_witnesses
        .iter()
        .filter(|witness| witness.local_role_id == local_role_id);
    let witness = witnesses.next()?;
    witnesses.next().is_none().then_some(witness)
}
