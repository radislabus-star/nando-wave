use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use nando_operator_learning::{
    K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1, K2InquiryVerifierCommandV1,
    K2InquiryVerifierReceiptV1, K2UncertaintyCasePreverificationV1,
    K2UncertaintyGeneratorRequestV1, K2UncertaintyGeneratorResponseV1,
    K2UncertaintyLearnerRequestV1, K2UncertaintyLearnerResponseV1, K2UncertaintyPrivateCaseV1,
    K2UncertaintyProbeOutputV1, K2UncertaintyProbeRequestV1, K2UncertaintyPublicCaseV1,
    K2UncertaintyRawProbeDispositionV1, K2UncertaintyTournamentArtifactsV1, composition_root_v1,
    enumerate_self_formed_probe_frontier_v1, generate_self_formed_development_batch_v1,
    learn_self_formed_uncertainty_v1, preverify_self_formed_case_with_owner_v1,
    publish_self_formed_probe_output_v1, run_self_formed_tournament_v1,
    verify_inquiry_selection_v1,
};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct R7Fixture {
    pub root: PathBuf,
    pub generated: K2UncertaintyGeneratorResponseV1,
    pub public_case: K2UncertaintyPublicCaseV1,
    pub private_case: K2UncertaintyPrivateCaseV1,
    pub learner_request: K2UncertaintyLearnerRequestV1,
    pub learned: K2UncertaintyLearnerResponseV1,
    pub probe_request: K2UncertaintyProbeRequestV1,
    pub probe_output: K2UncertaintyProbeOutputV1,
    pub tournament: K2UncertaintyTournamentArtifactsV1,
    pub preverification: K2UncertaintyCasePreverificationV1,
}

impl R7Fixture {
    pub fn new() -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nando-k2-self-formed-r7-controls-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create R7 controls root");
        let seed_path = std::env::var_os("NANDO_K2_DEVELOPMENT_SEED_PATH")
            .map(PathBuf::from)
            .expect("NANDO_K2_DEVELOPMENT_SEED_PATH is required for R7 controls");
        let seed = fs::read(seed_path).expect("read development seed");
        let generator_request =
            K2UncertaintyGeneratorRequestV1::development(seed.clone(), root_hash("generator"))
                .expect("generator request");
        let generated = generate_self_formed_development_batch_v1(&generator_request)
            .expect("development batch");
        let public_case = generated.public.cases[0].clone();
        let private_case = generated
            .private
            .cases
            .iter()
            .find(|case| case.case_id_sha256 == public_case.vocabulary.case_id_sha256)
            .expect("private case")
            .clone();
        let learner_request = K2UncertaintyLearnerRequestV1::seal(
            public_case.vocabulary.clone(),
            public_case.support.clone(),
            root_hash("learner"),
        )
        .expect("learner request");
        let learned = learn_self_formed_uncertainty_v1(&learner_request).expect("learner response");
        let probe_request = K2UncertaintyProbeRequestV1::seal(
            public_case.clone(),
            learned.clone(),
            generated.public.split_commitment_root_sha256.clone(),
            root_hash("probe"),
        )
        .expect("probe request");
        let probe_output = enumerate_self_formed_probe_frontier_v1(&probe_request)
            .expect("complete probe frontier");
        let artifact_root = root.join("frontier");
        let probe_artifacts = publish_self_formed_probe_output_v1(&artifact_root, &probe_output)
            .expect("publish probe artifacts");
        let tournament = run_self_formed_tournament_v1(
            &public_case,
            &learned,
            &probe_output,
            &generated.public.split_commitment_root_sha256,
            K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1,
            &root_hash("selector"),
            &root_hash("baseline"),
        )
        .expect("complete frontier tournament");
        let preverification = preverify_self_formed_case_with_owner_v1(
            &tournament,
            &probe_artifacts,
            &root_hash("baseline"),
            &root_hash("preverifier"),
            &mut |command| match command {
                K2InquiryVerifierCommandV1::VerifySelection {
                    verifier_executable_sha256,
                    selector_request,
                    precommit,
                } => Ok(K2InquiryVerifierReceiptV1::Selection {
                    value: verify_inquiry_selection_v1(
                        verifier_executable_sha256.clone(),
                        selector_request,
                        precommit,
                    )?,
                }),
                K2InquiryVerifierCommandV1::VerifyOutcome { .. } => {
                    panic!("R7 preverification requested outcome route")
                }
            },
        )
        .expect("case preverification");
        Self {
            root,
            generated,
            public_case,
            private_case,
            learner_request,
            learned,
            probe_request,
            probe_output,
            tournament,
            preverification,
        }
    }

    pub fn selected_disposition(&self) -> &K2UncertaintyRawProbeDispositionV1 {
        let winner = &self
            .preverification
            .tournament
            .tournament_winner_probe_root_sha256;
        self.probe_output
            .pages
            .iter()
            .flat_map(|page| &page.dispositions)
            .find(|disposition| &disposition.probe.probe_root_sha256 == winner)
            .expect("selected disposition")
    }
}

impl Drop for R7Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn root_hash(label: &str) -> String {
    composition_root_v1(&("nando.k2-self-formed-r7-control.v1", label)).expect("control root")
}
