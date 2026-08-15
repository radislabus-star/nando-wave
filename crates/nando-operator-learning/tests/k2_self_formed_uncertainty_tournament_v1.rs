use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use nando_operator_learning::{
    K2_UNCERTAINTY_SELECTOR_PROBES_V1, K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1,
    K2InquiryBaselinesV1, K2InquirySelectionPrecommitV1, K2UncertaintyGeneratorRequestV1,
    K2UncertaintyGeneratorResponseV1, K2UncertaintyLearnerRequestV1,
    K2UncertaintyLearnerResponseV1, K2UncertaintyProbeRequestV1, composition_root_v1,
    composition_sha256_file_v1, enumerate_self_formed_probe_frontier_v1,
    run_self_formed_tournament_v1, uncertainty_bytes_v1, uncertainty_decode_v1,
};

#[test]
fn r5_complete_frontier_tournament_matches_direct_winner_and_frozen_predecessors() {
    let seed_path = std::env::var_os("NANDO_K2_DEVELOPMENT_SEED_PATH")
        .map(PathBuf::from)
        .expect("NANDO_K2_DEVELOPMENT_SEED_PATH is required for R5 test");
    let seed = fs::read(seed_path).expect("read development seed");
    let generator = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-generator"));
    let learner = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-learner"));
    let selector = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-selector"));
    let baseline = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-baseline"));
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/k2_goal_environment/learned_composition/active_inquiry/selector.rs");
    let baseline_source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/k2_goal_environment/learned_composition/active_inquiry/baseline.rs");
    assert_eq!(
        composition_sha256_file_v1(&source_root).expect("selector source hash"),
        K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1
    );
    assert_eq!(
        composition_sha256_file_v1(&baseline_source).expect("baseline source hash"),
        "febf3c09ae22de3bcf0989ce6aeb569925124a2c1b32277b1f5cb3083736974b"
    );

    let generated: K2UncertaintyGeneratorResponseV1 = run_process(
        &generator,
        &K2UncertaintyGeneratorRequestV1::development(
            seed,
            composition_sha256_file_v1(&generator).expect("generator hash"),
        )
        .expect("generator request"),
    );
    let learner_sha256 = composition_sha256_file_v1(&learner).expect("learner hash");
    let selector_sha256 = composition_sha256_file_v1(&selector).expect("selector hash");
    let baseline_sha256 = composition_sha256_file_v1(&baseline).expect("baseline hash");
    let mut selected_families = BTreeSet::new();
    let selected_cases = generated
        .private
        .cases
        .iter()
        .filter(|case| case.matched_pair == 0 && selected_families.insert(case.topology_family))
        .collect::<Vec<_>>();
    assert_eq!(selected_cases.len(), 4, "one case from every U1-U4 family");

    for private_case in selected_cases {
        let public_case = generated
            .public
            .cases
            .iter()
            .find(|case| case.vocabulary.case_id_sha256 == private_case.case_id_sha256)
            .expect("public case");
        let learned: K2UncertaintyLearnerResponseV1 = run_process(
            &learner,
            &K2UncertaintyLearnerRequestV1::seal(
                public_case.vocabulary.clone(),
                public_case.support.clone(),
                learner_sha256.clone(),
            )
            .expect("learner request"),
        );
        let output = enumerate_self_formed_probe_frontier_v1(
            &K2UncertaintyProbeRequestV1::seal(
                public_case.clone(),
                learned.clone(),
                generated.public.split_commitment_root_sha256.clone(),
                root("probe-owner"),
            )
            .expect("probe request"),
        )
        .expect("complete frontier");
        let artifacts = run_self_formed_tournament_v1(
            public_case,
            &learned,
            &output,
            &generated.public.split_commitment_root_sha256,
            K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1,
            &selector_sha256,
            &baseline_sha256,
        )
        .expect("exact complete-frontier tournament");
        artifacts.tournament.validate().expect("valid tournament");
        let representatives = output.frontier.representative_probe_roots_sha256.len();
        let expected_requests = representatives
            .saturating_sub(K2_UNCERTAINTY_SELECTOR_PROBES_V1)
            .div_ceil(K2_UNCERTAINTY_SELECTOR_PROBES_V1 - 1)
            + 1;
        assert_eq!(artifacts.steps.len(), expected_requests);
        assert_eq!(
            artifacts.tournament.direct_winner.scores.len(),
            representatives
        );
        assert_eq!(
            artifacts.tournament.tournament_winner_probe_root_sha256,
            artifacts
                .tournament
                .direct_winner
                .selected_probe_root_sha256
        );
        for (sequence, step) in artifacts.steps.iter().enumerate() {
            assert_eq!(step.step_sequence, sequence as u64);
            assert_eq!(
                step.frontier_root_sha256,
                output.frontier.frontier_root_sha256
            );
            let process_precommit: K2InquirySelectionPrecommitV1 =
                run_process(&selector, &step.request);
            assert_eq!(process_precommit, step.precommit);
        }
        assert_eq!(artifacts.baselines.len(), 4);
        for trace in &artifacts.baselines {
            assert_eq!(trace.requests.len(), trace.outcomes.len());
            for (request, expected) in trace.requests.iter().zip(&trace.outcomes) {
                let process_outcome: K2InquiryBaselinesV1 = run_process(&baseline, request);
                assert_eq!(&process_outcome, expected);
            }
        }
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

fn root(label: &str) -> String {
    composition_root_v1(&("nando.k2-self-formed-r5-test-root.v1", label)).expect("test root")
}
