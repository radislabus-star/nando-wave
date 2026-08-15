use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use nando_operator_learning::{
    K2_UNCERTAINTY_CONFIRM_MODELS_V1, K2_UNCERTAINTY_CONSISTENCY_DISPOSITIONS_V1,
    K2_UNCERTAINTY_RAW_PROBES_V1, K2InquiryModelActionV1, K2UncertaintyGeneratorRequestV1,
    K2UncertaintyGeneratorResponseV1, K2UncertaintyLearnerRequestV1,
    K2UncertaintyLearnerResponseV1, K2UncertaintySyntacticModelV1, composition_sha256_file_v1,
    uncertainty_bytes_v1, uncertainty_decode_v1,
};

#[test]
fn r3_learner_forms_complete_four_class_set_from_public_support_only() {
    let seed_path = std::env::var_os("NANDO_K2_DEVELOPMENT_SEED_PATH")
        .map(PathBuf::from)
        .expect("NANDO_K2_DEVELOPMENT_SEED_PATH is required for R3 test");
    let seed = fs::read(seed_path).expect("read development seed");
    let generator = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-generator"));
    let learner = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-learner"));
    let generator_request = K2UncertaintyGeneratorRequestV1::development(
        seed,
        composition_sha256_file_v1(&generator).expect("generator hash"),
    )
    .expect("generator request");
    let generated: K2UncertaintyGeneratorResponseV1 = run_process(&generator, &generator_request);
    let learner_sha256 = composition_sha256_file_v1(&learner).expect("learner hash");

    for public_case in &generated.public.cases {
        let request = K2UncertaintyLearnerRequestV1::seal(
            public_case.vocabulary.clone(),
            public_case.support.clone(),
            learner_sha256.clone(),
        )
        .expect("learner request");
        let request_text =
            String::from_utf8(uncertainty_bytes_v1(&request).expect("request bytes"))
                .expect("request JSON");
        for forbidden in [
            "topology_family",
            "private_case_root_sha256",
            "expected_syntactic_model_count",
            "mapping",
            "true_model",
        ] {
            assert!(
                !request_text.contains(forbidden),
                "learner leak: {forbidden}"
            );
        }

        let learned: K2UncertaintyLearnerResponseV1 = run_process(&learner, &request);
        learned.validate().expect("learner response");
        learned
            .model_set
            .require_confirm_cardinality()
            .expect("four syntactic and semantic models");
        assert_eq!(
            learned.consistency.dispositions.len(),
            K2_UNCERTAINTY_CONSISTENCY_DISPOSITIONS_V1
        );
        assert_eq!(
            learned.model_set.semantic_signatures.len(),
            K2_UNCERTAINTY_CONFIRM_MODELS_V1
        );
        assert!(
            learned
                .model_set
                .semantic_signatures
                .iter()
                .all(|signature| signature.observable_outcome_roots_sha256.len()
                    == K2_UNCERTAINTY_RAW_PROBES_V1)
        );
        assert_eq!(learned.world_models.len(), K2_UNCERTAINTY_CONFIRM_MODELS_V1);

        let private_case = generated
            .private
            .cases
            .iter()
            .find(|case| case.case_id_sha256 == public_case.vocabulary.case_id_sha256)
            .expect("private case");
        let true_syntax = K2UncertaintySyntacticModelV1::seal(
            private_case
                .mapping
                .iter()
                .map(|entry| K2InquiryModelActionV1 {
                    action_id_sha256: entry.opaque_action_root_sha256.clone(),
                    effect: entry.effect.clone(),
                })
                .collect(),
        )
        .expect("private syntax");
        assert!(
            learned
                .model_set
                .syntactic_models
                .iter()
                .any(|model| model.syntax_root_sha256 == true_syntax.syntax_root_sha256)
        );
    }
}

fn run_process<I, O>(executable: &Path, input: &I) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn process");
    child
        .stdin
        .as_mut()
        .expect("process stdin")
        .write_all(&uncertainty_bytes_v1(input).expect("input bytes"))
        .expect("write input");
    let output = child.wait_with_output().expect("process output");
    assert!(
        output.status.success(),
        "process failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    uncertainty_decode_v1(&output.stdout).expect("canonical process response")
}
