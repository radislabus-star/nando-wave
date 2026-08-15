use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use nando_operator_learning::{
    K2_UNCERTAINTY_EFFECTS_PER_ACTION_V1, K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1,
    K2_UNCERTAINTY_RAW_PREDICTIONS_V1, K2_UNCERTAINTY_RAW_PROBES_V1, K2CompositionLearnedEffectV1,
    K2UncertaintyGeneratorRequestV1, K2UncertaintyGeneratorResponseV1,
    K2UncertaintyLearnerRequestV1, K2UncertaintyLearnerResponseV1,
    K2UncertaintyPrivateSafetyDispositionV1, K2UncertaintyProbeRequestV1,
    K2UncertaintySafetyRequestV1, composition_root_v1, composition_sha256_file_v1,
    enumerate_self_formed_probe_frontier_v1, self_formed_grammar_root_v1, uncertainty_bytes_v1,
    uncertainty_decode_v1, verify_self_formed_private_safety_v1,
};

#[test]
fn r4_frontier_exhausts_all_probes_and_private_safety_fails_closed() {
    let seed_path = std::env::var_os("NANDO_K2_DEVELOPMENT_SEED_PATH")
        .map(PathBuf::from)
        .expect("NANDO_K2_DEVELOPMENT_SEED_PATH is required for R4 test");
    let seed = fs::read(seed_path).expect("read development seed");
    let generator = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-generator"));
    let learner = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-learner"));
    let generated: K2UncertaintyGeneratorResponseV1 = run_process(
        &generator,
        &K2UncertaintyGeneratorRequestV1::development(
            seed,
            composition_sha256_file_v1(&generator).expect("generator hash"),
        )
        .expect("generator request"),
    );
    let learner_sha256 = composition_sha256_file_v1(&learner).expect("learner hash");
    let mut selected_families = BTreeSet::new();
    let selected_cases = generated
        .private
        .cases
        .iter()
        .filter(|case| case.matched_pair == 0 && selected_families.insert(case.topology_family))
        .collect::<Vec<_>>();
    assert_eq!(
        selected_cases.len(),
        4,
        "one case from every topology family"
    );

    let mut observed_class_counts = BTreeSet::new();
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
        let probe_request = K2UncertaintyProbeRequestV1::seal(
            public_case.clone(),
            learned,
            generated.public.split_commitment_root_sha256.clone(),
            root("probe-owner"),
        )
        .expect("probe request");
        uncertainty_bytes_v1(&probe_request).expect("probe request under protocol budget");
        let output = enumerate_self_formed_probe_frontier_v1(&probe_request)
            .expect("complete mechanical frontier");
        output.validate().expect("valid probe output");
        assert_eq!(
            output.frontier.raw_probe_count,
            K2_UNCERTAINTY_RAW_PROBES_V1 as u64
        );
        assert_eq!(
            output.frontier.raw_prediction_count,
            K2_UNCERTAINTY_RAW_PREDICTIONS_V1 as u64
        );
        assert_eq!(
            output.pages.len(),
            K2_UNCERTAINTY_RAW_PROBES_V1.div_ceil(K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1)
        );
        assert!(output.frontier.classes.len() >= 8);
        observed_class_counts.insert(output.frontier.classes.len());
        uncertainty_bytes_v1(&output.state_universe).expect("state universe protocol budget");
        uncertainty_bytes_v1(&output.frontier).expect("frontier protocol budget");
        for page in &output.pages {
            uncertainty_bytes_v1(page).expect("frontier page protocol budget");
            assert!(page.dispositions.iter().all(|disposition| {
                disposition.robust_accounting.effects.len() == K2_UNCERTAINTY_EFFECTS_PER_ACTION_V1
            }));
        }

        let representative = &output.frontier.representative_probe_roots_sha256[0];
        let disposition = output
            .pages
            .iter()
            .flat_map(|page| &page.dispositions)
            .find(|disposition| &disposition.probe.probe_root_sha256 == representative)
            .expect("representative disposition");
        let private_effect = private_case
            .mapping
            .iter()
            .find(|entry| entry.opaque_action_root_sha256 == disposition.probe.action_id_sha256)
            .expect("private effect")
            .effect
            .clone();
        let grammar_root =
            self_formed_grammar_root_v1(&public_case.vocabulary).expect("grammar root");
        let safety_request = K2UncertaintySafetyRequestV1::seal(
            root("selection"),
            disposition.probe.clone(),
            private_effect,
            public_case.vocabulary.clone(),
            grammar_root.clone(),
            root("sandbox"),
            root("safety-owner"),
        )
        .expect("safety request");
        let safety =
            verify_self_formed_private_safety_v1(&safety_request).expect("private safety receipt");
        assert_eq!(
            safety.disposition,
            K2UncertaintyPrivateSafetyDispositionV1::Pass
        );

        let foreign_effect = K2CompositionLearnedEffectV1::RemoveFile {
            path: "outside/frozen-vocabulary".to_owned(),
        };
        let veto_request = K2UncertaintySafetyRequestV1::seal(
            root("selection-veto"),
            disposition.probe.clone(),
            foreign_effect,
            public_case.vocabulary.clone(),
            grammar_root,
            root("sandbox-veto"),
            root("safety-owner"),
        )
        .expect("veto request");
        let veto = verify_self_formed_private_safety_v1(&veto_request)
            .expect("private safety veto receipt");
        assert_eq!(
            veto.disposition,
            K2UncertaintyPrivateSafetyDispositionV1::GrammarVeto
        );
    }
    assert!(!observed_class_counts.is_empty());
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
    composition_root_v1(&("nando.k2-self-formed-r4-test-root.v1", label)).expect("test root")
}
