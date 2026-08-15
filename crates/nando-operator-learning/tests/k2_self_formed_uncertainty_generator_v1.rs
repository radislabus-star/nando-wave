use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use nando_operator_learning::{
    K2_UNCERTAINTY_ACTIONS_V1, K2_UNCERTAINTY_CONFIRM_CASES_V1, K2_UNCERTAINTY_SUPPORT_ROWS_V1,
    K2UncertaintyGeneratorRequestV1, K2UncertaintyGeneratorResponseV1, composition_sha256_file_v1,
    uncertainty_bytes_v1, uncertainty_decode_v1,
};

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
