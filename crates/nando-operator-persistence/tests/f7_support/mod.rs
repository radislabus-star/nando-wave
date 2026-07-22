#[allow(dead_code, unused_imports)]
#[path = "../../../nando-operator-proof/tests/f6_support/mod.rs"]
mod f6_support;

use std::{fs, path::PathBuf};

use f6_support::{finish_handoff_v3, handoff_v3, request_payload_v3};
use nando_operator_kernel::{
    GenerationEvidencePartitionV3, OperatorGenerationComponentRootsV3, RuntimeProjectionV3,
    canonical_json_sha256, executable_artifact_set_sha256_v3, seal_operator_generation_manifest_v3,
};
use nando_operator_learning::{GenerationEvidenceLedgerV3, GenerationLearningOutcomeV3};
use nando_operator_persistence::{
    GenerationCheckpointReceiptRefV3, encode_generation_checkpoint_v3,
};
use nando_operator_proof::{
    generation_receipt_v3::{
        GenerationVerifierReceiptInputV3, GenerationVerifierReceiptV3,
        seal_generation_verifier_receipt_v3,
    },
    independent_verifier_v3::{
        IndependentVerifierArtifactSetV3, IndependentVerifierBudgetV3, IndependentVerifierInputV3,
        IndependentVerifierReceiptV3, verify_operator_result_v3,
    },
};
use nando_operator_runtime::{
    compile_structural_dispatch_index_v3, encode_operator_generation_restart_bundle_v3,
};

pub struct FixtureV3 {
    pub directory: PathBuf,
    pub manifest: nando_operator_kernel::OperatorGenerationManifestV3,
    pub bundle: Box<[u8]>,
    pub ledger: GenerationEvidenceLedgerV3,
    receipts: Vec<(IndependentVerifierReceiptV3, GenerationVerifierReceiptV3)>,
    support: f6_support::F5HandoffV3,
}

impl Drop for FixtureV3 {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

impl FixtureV3 {
    pub fn new(label: &str) -> Self {
        Self::new_generation(label, 1, None, "actor")
    }

    pub fn new_generation(
        label: &str,
        sequence: u64,
        parent_generation_id_sha256: Option<String>,
        actor_label: &str,
    ) -> Self {
        let support = handoff_v3("continue_session", "handle", "CellA17", &[51]);
        let index = compile_structural_dispatch_index_v3(&support.artifacts).expect("index");
        let manifest = seal_operator_generation_manifest_v3(
            sequence,
            parent_generation_id_sha256,
            OperatorGenerationComponentRootsV3 {
                artifact_set_sha256: executable_artifact_set_sha256_v3(&support.artifacts)
                    .expect("artifact set"),
                dispatch_index_sha256: index.index_sha256().to_owned(),
                actor_program_sha256: root(actor_label),
                renderer_program_sha256: root("renderer"),
                verifier_contract_sha256: root("verifier"),
                capability_contract_sha256: root("capability"),
                resource_budget_sha256: root("budget"),
            },
        )
        .expect("manifest");
        let bundle = encode_operator_generation_restart_bundle_v3(&manifest, &support.artifacts)
            .expect("bundle");
        let directory = std::env::temp_dir().join(format!(
            "nando-f7d-{label}-{}-{}",
            std::process::id(),
            root(label)
        ));
        let ledger = GenerationEvidenceLedgerV3::new(&manifest);
        Self {
            directory,
            manifest,
            bundle,
            ledger,
            receipts: Vec::new(),
            support,
        }
    }

    pub fn append_support(&mut self) {
        let f6 = verify(&self.support, &self.support.actor_output);
        let envelope = seal_generation_verifier_receipt_v3(
            &self.manifest,
            GenerationVerifierReceiptInputV3 {
                partition: GenerationEvidencePartitionV3::Support,
                capture_sequence: 1,
                support_watermark_next_sequence: 10,
                support_freeze_sha256: None,
                lineage_root_sha256: root("support lineage"),
                event_root_sha256: root("support event"),
            },
            &f6,
        )
        .expect("support envelope");
        self.ledger
            .append_generation_verifier_receipt(
                &envelope,
                GenerationLearningOutcomeV3::VerifiedPass,
            )
            .expect("support append");
        self.receipts.push((f6, envelope));
    }

    pub fn freeze_and_append_future(&mut self) {
        let freeze_sha256 = self
            .ledger
            .freeze_support(10, root("support watermark"))
            .expect("freeze")
            .freeze_sha256()
            .to_owned();
        let future = finish_handoff_v3(
            self.support.artifacts.clone(),
            "continue TaskB22".to_owned(),
            request_payload_v3("resume_task", "ticket", "TaskB22"),
        );
        let f6 = verify(&future, &future.actor_output);
        let envelope = seal_generation_verifier_receipt_v3(
            &self.manifest,
            GenerationVerifierReceiptInputV3 {
                partition: GenerationEvidencePartitionV3::Future,
                capture_sequence: 10,
                support_watermark_next_sequence: 10,
                support_freeze_sha256: Some(freeze_sha256),
                lineage_root_sha256: root("future lineage"),
                event_root_sha256: root("future event"),
            },
            &f6,
        )
        .expect("future envelope");
        self.ledger
            .append_generation_verifier_receipt(
                &envelope,
                GenerationLearningOutcomeV3::VerifiedPass,
            )
            .expect("future append");
        self.receipts.push((f6, envelope));
    }

    pub fn checkpoint(&self, publish_sequence: u64) -> Box<[u8]> {
        let refs = self
            .receipts
            .iter()
            .map(
                |(f6_receipt, generation_receipt)| GenerationCheckpointReceiptRefV3 {
                    f6_receipt,
                    generation_receipt,
                },
            )
            .collect::<Vec<_>>();
        encode_generation_checkpoint_v3(publish_sequence, &self.bundle, &self.ledger, &refs)
            .expect("checkpoint")
    }
}

pub fn root(label: &str) -> String {
    canonical_json_sha256(&label).expect("root")
}

fn verify(handoff: &f6_support::F5HandoffV3, output: &str) -> IndependentVerifierReceiptV3 {
    let artifact_set =
        IndependentVerifierArtifactSetV3::new(&handoff.artifacts).expect("artifact set");
    let input = IndependentVerifierInputV3::new(
        &handoff.request_sha256,
        RuntimeProjectionV3::Responses,
        &handoff.payload_bytes,
        &artifact_set,
        &handoff.action,
        output,
    )
    .expect("input");
    verify_operator_result_v3(&input, IndependentVerifierBudgetV3::default()).expect("receipt")
}
