use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use nando_operator_learning::{
    K2_UNCERTAINTY_ACTIONS_V1, K2_UNCERTAINTY_CONFIRM_CASES_V1,
    K2_UNCERTAINTY_DEVELOPMENT_SEED_COMMITMENT_V1, K2_UNCERTAINTY_GENERATOR_REQUEST_SCHEMA_V1,
    K2_UNCERTAINTY_GENERATOR_RESPONSE_SCHEMA_V1, K2_UNCERTAINTY_PREREGISTRATION_V2_ROOT_V1,
    K2_UNCERTAINTY_PREREGISTRATION_V3_ROOT_V1, K2_UNCERTAINTY_SUPPORT_ROWS_V1,
    K2CompositionAuthorityBoundaryV1, K2UncertaintyConfirmGeneratorRequestV1,
    K2UncertaintyConfirmGeneratorResponseV1, K2UncertaintyGeneratorRequestV1,
    K2UncertaintyGeneratorResponseV1, K2UncertaintySplitV1, composition_sha256_bytes_v1,
    composition_sha256_file_v1, generate_self_formed_confirm_batch_v1,
    generate_self_formed_development_batch_v1, uncertainty_bytes_v1, uncertainty_decode_v1,
    uncertainty_root_v1,
};

const HISTORICAL_GENERATOR_SHA256: &str =
    "929c51bf374ddc55a4a109977bbc987e3443ee1bf4317f873fb3aed21568652b";
const HISTORICAL_REQUEST_ROOT_SHA256: &str =
    "c285343b9aa9c5146a1f512cf3c2f412a0e538dd803665429b42035e81430588";
const HISTORICAL_RESPONSE_ROOT_SHA256: &str =
    "10264f4a25e3ad22156a30c49dc1f53aee1de5754d6dd0f1b77c54929b3531cf";
const HISTORICAL_PUBLIC_BATCH_ROOT_SHA256: &str =
    "9fbdd35627b2b5265b8e35274412e7dc2a0cce576066022d99b8c67f13b8ad8a";
const HISTORICAL_PRIVATE_BATCH_ROOT_SHA256: &str =
    "5ed5436d0c78e5e62f58bbb4efa34cda73a50af296fdf03dfab16014f1c274e5";
const HISTORICAL_PRIVATE_DENOMINATOR_ROOT_SHA256: &str =
    "011ea03be71e80d520101ce2e7b8897be2a2b88e4285d0c8ca905dbb715026dc";
const HISTORICAL_RESPONSE_BYTES_SHA256: &str =
    "26e509ae09fd68b97323e9e9d0ee1bce9bfee7b0dbd75be5aca2015675027c6e";

#[test]
fn r2_development_generator_is_deterministic_private_and_process_isolated() {
    let seed_path = std::env::var_os("NANDO_K2_DEVELOPMENT_SEED_PATH")
        .map(PathBuf::from)
        .expect("NANDO_K2_DEVELOPMENT_SEED_PATH is required for R2 test");
    let seed = fs::read(seed_path).expect("read development seed");
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-generator"));
    let executable_sha256 = composition_sha256_file_v1(&executable).expect("generator hash");
    let request = K2UncertaintyGeneratorRequestV1::development(seed.clone(), executable_sha256)
        .expect("development request");

    let mut child = Command::new(&executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn generator");
    child
        .stdin
        .as_mut()
        .expect("generator stdin")
        .write_all(&uncertainty_bytes_v1(&request).expect("request bytes"))
        .expect("write generator request");
    let output = child.wait_with_output().expect("generator output");
    assert!(
        output.status.success(),
        "generator failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let generated: K2UncertaintyGeneratorResponseV1 =
        uncertainty_decode_v1(&output.stdout).expect("generator response");
    generated.validate().expect("valid generator response");

    let repeated = run_again(&executable, &request);
    assert_eq!(generated, repeated, "generator bytes must be deterministic");
    assert_eq!(
        generated.public.cases.len(),
        K2_UNCERTAINTY_CONFIRM_CASES_V1
    );
    assert_eq!(
        generated.private.cases.len(),
        K2_UNCERTAINTY_CONFIRM_CASES_V1
    );
    for case in &generated.public.cases {
        assert_eq!(
            case.support.observations.len(),
            K2_UNCERTAINTY_SUPPORT_ROWS_V1
        );
        assert_eq!(
            case.vocabulary.opaque_action_roots_sha256.len(),
            K2_UNCERTAINTY_ACTIONS_V1
        );
    }

    let public_bytes = uncertainty_bytes_v1(&generated.public).expect("public bytes");
    let public_text = String::from_utf8(public_bytes).expect("public JSON");
    for forbidden in [
        "topology_family",
        "mapping",
        "expected_syntactic_model_count",
        "private_case_root_sha256",
        "seed_bytes",
        "true_model",
    ] {
        assert!(
            !public_text.contains(forbidden),
            "private generator field leaked: {forbidden}"
        );
    }
    assert!(
        !public_text
            .as_bytes()
            .windows(seed.len())
            .any(|window| window == seed)
    );

    let mut matched = BTreeMap::new();
    for case in &generated.private.cases {
        matched
            .entry((case.topology_family, case.matched_pair))
            .or_insert_with(Vec::new)
            .push(&case.mapping);
    }
    assert_eq!(matched.len(), 8);
    for pair in matched.values() {
        assert_eq!(pair.len(), 2);
        assert_ne!(
            pair[0], pair[1],
            "matched pair must differ in private truth"
        );
    }
}

#[test]
fn r7g_development_inner_roots_are_exact_and_outer_rebinding_is_identity_only() {
    let seed_path = std::env::var_os("NANDO_K2_DEVELOPMENT_SEED_PATH")
        .map(PathBuf::from)
        .expect("NANDO_K2_DEVELOPMENT_SEED_PATH is required for R7G parity test");
    let seed = fs::read(seed_path).expect("read development seed");
    let historical_request = K2UncertaintyGeneratorRequestV1::development(
        seed.clone(),
        HISTORICAL_GENERATOR_SHA256.into(),
    )
    .expect("historical request");
    let historical = generate_self_formed_development_batch_v1(&historical_request)
        .expect("historical response");
    assert_eq!(
        historical_request.request_root_sha256,
        HISTORICAL_REQUEST_ROOT_SHA256
    );
    assert_eq!(
        historical.response_root_sha256,
        HISTORICAL_RESPONSE_ROOT_SHA256
    );
    assert_eq!(
        historical.public.public_batch_root_sha256,
        HISTORICAL_PUBLIC_BATCH_ROOT_SHA256
    );
    assert_eq!(
        historical.private.private_batch_root_sha256,
        HISTORICAL_PRIVATE_BATCH_ROOT_SHA256
    );
    assert_eq!(
        historical.private.expected_denominator_commitment_sha256,
        HISTORICAL_PRIVATE_DENOMINATOR_ROOT_SHA256
    );
    assert_eq!(
        composition_sha256_bytes_v1(
            &uncertainty_bytes_v1(&historical).expect("historical response bytes")
        ),
        HISTORICAL_RESPONSE_BYTES_SHA256
    );

    let successor_executable_sha256 = composition_sha256_bytes_v1(b"r7g-successor-generator");
    let successor_request = K2UncertaintyGeneratorRequestV1::development(
        seed.clone(),
        successor_executable_sha256.clone(),
    )
    .expect("successor request");
    let successor =
        generate_self_formed_development_batch_v1(&successor_request).expect("successor response");
    assert_eq!(successor.public, historical.public);
    assert_eq!(successor.private, historical.private);
    assert_ne!(
        successor_request.request_root_sha256,
        historical_request.request_root_sha256
    );
    assert_ne!(
        successor.response_root_sha256,
        historical.response_root_sha256
    );

    let authority = K2CompositionAuthorityBoundaryV1::denied();
    let independently_rebuilt_request_root = uncertainty_root_v1(&(
        K2_UNCERTAINTY_GENERATOR_REQUEST_SCHEMA_V1,
        K2UncertaintySplitV1::Development,
        &seed,
        K2_UNCERTAINTY_DEVELOPMENT_SEED_COMMITMENT_V1,
        K2_UNCERTAINTY_PREREGISTRATION_V2_ROOT_V1,
        K2_UNCERTAINTY_PREREGISTRATION_V3_ROOT_V1,
        &successor_executable_sha256,
        &authority,
    ))
    .expect("independent request root");
    assert_eq!(
        successor_request.request_root_sha256,
        independently_rebuilt_request_root
    );
    let independently_rebuilt_response_root = uncertainty_root_v1(&(
        K2_UNCERTAINTY_GENERATOR_RESPONSE_SCHEMA_V1,
        &successor_request.request_root_sha256,
        &successor.public,
        &successor.private,
        &authority,
    ))
    .expect("independent response root");
    assert_eq!(
        successor.response_root_sha256,
        independently_rebuilt_response_root
    );
}

#[test]
fn r7g_confirm_schema_is_closed_split_preserving_and_process_dispatched() {
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-generator"));
    let executable_sha256 = composition_sha256_file_v1(&executable).expect("generator hash");
    let static_test_material = vec![0xA5; 32];
    let request = K2UncertaintyConfirmGeneratorRequestV1::seal(
        static_test_material,
        composition_sha256_bytes_v1(b"r7g-successor-freeze"),
        composition_sha256_bytes_v1(b"r7g-authorization-receipt"),
        executable_sha256,
    )
    .expect("static Confirm request");
    let generated =
        generate_self_formed_confirm_batch_v1(&request).expect("pure Confirm generation");
    assert!(
        generated
            .public
            .cases
            .iter()
            .all(|case| case.vocabulary.split == K2UncertaintySplitV1::Confirm)
    );
    assert_eq!(
        generated.public.split_commitment_root_sha256,
        request.nonce_commitment_sha256
    );
    assert_ne!(
        generated.public.public_batch_root_sha256,
        HISTORICAL_PUBLIC_BATCH_ROOT_SHA256
    );
    assert!(
        uncertainty_decode_v1::<K2UncertaintyGeneratorRequestV1>(
            &uncertainty_bytes_v1(&request).expect("Confirm request bytes")
        )
        .is_err()
    );

    let process_generated: K2UncertaintyConfirmGeneratorResponseV1 =
        run_confirm(&executable, &request);
    assert_eq!(process_generated, generated);
    let public_text =
        String::from_utf8(uncertainty_bytes_v1(&generated.public).expect("Confirm public bytes"))
            .expect("Confirm public JSON");
    for forbidden in [
        "nonce_bytes",
        "mapping",
        "topology_family",
        "authorization_receipt",
    ] {
        assert!(
            !public_text.contains(forbidden),
            "private Confirm field leaked: {forbidden}"
        );
    }
}

fn run_again(
    executable: &PathBuf,
    request: &K2UncertaintyGeneratorRequestV1,
) -> K2UncertaintyGeneratorResponseV1 {
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn repeated generator");
    child
        .stdin
        .as_mut()
        .expect("repeated generator stdin")
        .write_all(&uncertainty_bytes_v1(request).expect("repeated request bytes"))
        .expect("write repeated request");
    let output = child.wait_with_output().expect("repeated generator output");
    assert!(
        output.status.success(),
        "repeated generator failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    uncertainty_decode_v1(&output.stdout).expect("repeated generator response")
}

fn run_confirm(
    executable: &PathBuf,
    request: &K2UncertaintyConfirmGeneratorRequestV1,
) -> K2UncertaintyConfirmGeneratorResponseV1 {
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn Confirm generator");
    child
        .stdin
        .as_mut()
        .expect("Confirm generator stdin")
        .write_all(&uncertainty_bytes_v1(request).expect("Confirm request bytes"))
        .expect("write Confirm request");
    let output = child.wait_with_output().expect("Confirm generator output");
    assert!(
        output.status.success(),
        "Confirm generator failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    uncertainty_decode_v1(&output.stdout).expect("Confirm generator response")
}
