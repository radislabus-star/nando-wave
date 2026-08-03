mod container_role;
mod extractor;
mod relations;

const REQUEST_REFERENCED_FLAG_V2: u16 = 1;

pub(crate) use extractor::extract_pre_action_multi_source_topology_v2;
