#[path = "authority.rs"]
mod authority;
#[path = "app/collection.rs"]
mod collection;
#[path = "app/io.rs"]
mod io;
#[path = "app/orchestration.rs"]
mod orchestration;
#[path = "app/parity.rs"]
mod parity;
#[path = "app/support.rs"]
mod support;

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use nando_response_actor::{
    AtomSource, AtomValueType, COLLECTION_EXTERNAL_VERIFIER_SCHEMA, CollectionSynthesisExample,
    FrameRepresentationPolicy, GROUNDED_RESPONSE_PACKAGE_PREFIX,
    RESPONSE_FUTURE_VERIFIER_RECEIPT_SCHEMA_V2, RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2,
    RESPONSE_RUNTIME_PARITY_RECEIPT_SET_SCHEMA_V1, RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1,
    ROUTING_REFINEMENT_VERSION, RelationAtom, RelationFrame, ResponseExecutionStatus,
    ResponseOperation, ResponsePackage, ResponsePackageOrigin, ResponsePackageProof,
    ResponsePackageState, ResponseRegistry, ResponseRelationObservation, ResponseShadowObservation,
    ResponseSupportFreezePolicy, ResponseSupportManifest, ResponseSupportManifestSet,
    ResponseValueSelector, SOURCE_VALUE_EXTERNAL_VERIFIER_SCHEMA,
    VALUE_PROJECTION_EXTERNAL_VERIFIER_SCHEMA, canonical_json_sha256, compile_response_registry,
    compile_source_neutral_quarantine_packages, evaluate_grounded_wave_causality_refs,
    execute_response, frame_matches_program_action_contract_with_grounding,
    freeze_source_neutral_support, freeze_source_neutral_support_with_policy, ground_roles,
    is_source_neutral_relation_frame, partition_teacher_training_families,
    relation_frame_phase_margin_micro, relation_frame_routes_to_package,
    relation_frame_routing_atom_ids, response_actor_program_digest,
    response_independent_verifier_program_digest, response_package_digest,
    response_package_lineage_id, response_program_external_verifier_schema,
    response_program_required_routing_atom_ids, response_support_manifest_digest,
    synthesize_response_operator, synthesize_unique_collection_program, verify_operator_structure,
    verify_response_independently,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use authority::{
    aggregate_causal_verdict, compile_runtime_registry, package_receipt_sets,
    response_authority_candidate,
};
// These bridges stay private to the miner application and do not extend the crate API.
use collection::*;
use io::*;
use orchestration::run_with_args;
use parity::*;
use support::*;

const SELF_TRAINING_MIN_VERIFIED_FUTURE_ROWS: usize = 64;
const SELF_TRAINING_MIN_VERIFIED_FUTURE_SESSIONS: usize = 6;
const SELF_TRAINING_RESERVED_FUTURE_SESSIONS: usize = 3;
const SELF_TRAINING_MIN_ROLLOVER_ROWS: usize = 32;

#[cfg(test)]
use nando_response_actor::{
    RESPONSE_AUTHORITY_SCHEMA_V2, RESPONSE_REGISTRY_SCHEMA_V6, ResponsePackageAuthorityBindingV2,
    response_registry_digest,
};

pub(super) fn main() {
    if let Err(error) = run() {
        eprintln!("nando-response-miner: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    run_with_args(&args)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
