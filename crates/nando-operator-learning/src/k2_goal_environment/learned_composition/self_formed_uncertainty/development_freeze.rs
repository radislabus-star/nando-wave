mod model;
mod persistence;

#[cfg(test)]
mod tests;

pub use model::*;
pub use persistence::{
    publish_self_formed_development_freeze_v1, read_self_formed_development_freeze_v1,
    run_self_formed_development_freeze_process_v1,
};

pub const K2_UNCERTAINTY_DEVELOPMENT_FREEZE_INPUT_SCHEMA_V1: &str =
    "nando.k2-self-formed-development-freeze-input.v1";
pub const K2_UNCERTAINTY_FROZEN_MANIFEST_SCHEMA_V1: &str =
    "nando.k2-self-formed-frozen-manifest.v1";
pub const K2_UNCERTAINTY_DEVELOPMENT_RESULT_SCHEMA_V1: &str =
    "nando.k2-self-formed-development-result.v1";
pub const K2_UNCERTAINTY_CONFIRM_READ_CAPABILITY_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-read-capability.v1";
pub const K2_UNCERTAINTY_DEVELOPMENT_FREEZE_SCHEMA_V1: &str =
    "nando.k2-self-formed-development-freeze.v1";
pub const K2_UNCERTAINTY_DEVELOPMENT_FREEZE_ROOT_ENV_V1: &str = "NANDO_K2_DEVELOPMENT_FREEZE_ROOT";

const DEVELOPMENT_FREEZE_FILE_V1: &str = "development-freeze.json";
const PRODUCTION_SERVING_SOURCE_SHA256_V1: &str =
    "06de5229f16856df998ac3f71baae61bc4808183983aabca68146e28dc748949";
const PRODUCTION_DASHBOARD_SOURCE_SHA256_V1: &str =
    "2cb90e37d875b2ef4baab26d3abe235d4f6d7c29b19324e2962e16e8a1979f6c";
