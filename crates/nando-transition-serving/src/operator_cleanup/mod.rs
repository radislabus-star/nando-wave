mod authority;
mod protocol;
mod staging;

pub use authority::CleanupAuthorityRuntimeConfigV1;
pub use protocol::{
    BUNDLE_INPUT_SCHEMA_V1, BundleInputV1, CHALLENGE_SCHEMA_V1, CleanupChallengeV1,
    canonical_bundle_id, challenge_root, read_canonical_json, sha256, validate_bundle_input,
    validate_challenge, write_once,
};

pub(crate) use authority::{handle_authority_line, request_cleanup};
pub(crate) use protocol::{
    CLEANUP_AUTHORITY_REQUEST_SCHEMA_V1, CleanupAuthorityRequestV1, k1_package_candidate_root,
};
