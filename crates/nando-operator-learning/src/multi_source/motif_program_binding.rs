use nando_operator_kernel::{
    PreActionMultiSourceTopologyV1, ResponseProgram, canonical_json_sha256,
    response_program_version_root_sha256,
};
use serde::{Deserialize, Serialize};

use super::{
    SourceNeutralTopologyMotifV1, pre_action_t1_binding_root, pre_action_t1_consumed_role_ids_v1,
};

pub const PRE_ACTION_T1_MOTIF_BINDING_SCHEMA_V1: &str = "nando.pre-action-t1-motif-binding.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreActionT1MotifBindingV1 {
    pub schema: String,
    pub binding_root_sha256: String,
    pub program_root_sha256: String,
    pub pre_action_binding_root_sha256: String,
    pub ambient_topology_root_sha256: String,
    pub motif_root_sha256: String,
    pub embedding_root_sha256: String,
    pub consumed_local_role_ids: Vec<u16>,
}

impl PreActionT1MotifBindingV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != PRE_ACTION_T1_MOTIF_BINDING_SCHEMA_V1
            || self.consumed_local_role_ids.is_empty()
            || !self
                .consumed_local_role_ids
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            || self.binding_root_sha256 != self.expected_root()?
        {
            return Err("pre_action_t1_motif_binding_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            PRE_ACTION_T1_MOTIF_BINDING_SCHEMA_V1,
            self.program_root_sha256.as_str(),
            self.pre_action_binding_root_sha256.as_str(),
            self.ambient_topology_root_sha256.as_str(),
            self.motif_root_sha256.as_str(),
            self.embedding_root_sha256.as_str(),
            self.consumed_local_role_ids.as_slice(),
        ))
    }
}

pub fn bind_pre_action_t1_program_to_motif_v1(
    program: &ResponseProgram,
    topology: &PreActionMultiSourceTopologyV1,
    motif: &SourceNeutralTopologyMotifV1,
) -> Result<PreActionT1MotifBindingV1, &'static str> {
    topology.validate()?;
    motif.validate()?;
    let ambient_topology_root_sha256 = canonical_json_sha256(topology)?;
    let consumed_local_role_ids = pre_action_t1_consumed_role_ids_v1(program, topology)?;
    let embedding = motif
        .embeddings
        .iter()
        .filter(|embedding| {
            embedding.ambient_topology_root_sha256 == ambient_topology_root_sha256
                && consumed_local_role_ids
                    .iter()
                    .all(|role_id| embedding.local_role_ids.binary_search(role_id).is_ok())
        })
        .min_by(|left, right| left.embedding_root_sha256.cmp(&right.embedding_root_sha256))
        .ok_or("program_consumed_roles_outside_frozen_motif")?;
    let mut binding = PreActionT1MotifBindingV1 {
        schema: PRE_ACTION_T1_MOTIF_BINDING_SCHEMA_V1.to_owned(),
        binding_root_sha256: String::new(),
        program_root_sha256: response_program_version_root_sha256(program)?,
        pre_action_binding_root_sha256: pre_action_t1_binding_root(program, topology)?,
        ambient_topology_root_sha256,
        motif_root_sha256: motif.motif_root_sha256.clone(),
        embedding_root_sha256: embedding.embedding_root_sha256.clone(),
        consumed_local_role_ids,
    };
    binding.binding_root_sha256 = binding.expected_root()?;
    binding.validate()?;
    Ok(binding)
}
