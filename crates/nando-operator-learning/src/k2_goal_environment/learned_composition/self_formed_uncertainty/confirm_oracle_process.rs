use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, composition_sha256_bytes_v1,
    composition_sha256_file_v1,
};
use super::{
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2UncertaintyBaselineSummaryV1,
    K2UncertaintyCasePreverificationV2, K2UncertaintyClosurePlanV1,
    K2UncertaintyConfirmFinalTruthCaseV1, K2UncertaintyConfirmFinalVerifierReceiptV1,
    K2UncertaintyFrontierPageV1, K2UncertaintyFrontierV1, K2UncertaintyModelSetV1,
    K2UncertaintyObservationVectorV2, K2UncertaintyOracleBaselineCaseDescriptorV1,
    K2UncertaintyOracleBaselineCaseReceiptV1, K2UncertaintyOracleCaseEvidenceManifestV1,
    K2UncertaintyOracleEvidenceEntryV1, K2UncertaintyOracleEvidenceKindV1,
    K2UncertaintyOraclePublicBindingsV1, evaluate_self_formed_oracle_case_v1,
    reconstruct_self_formed_oracle_frontier_v1, uncertainty_bytes_v1, uncertainty_decode_v1,
};

pub const K2_UNCERTAINTY_ORACLE_MANIFEST_FILE_V1: &str = "case-evidence-manifest.json";

pub struct K2UncertaintyLoadedOracleCaseEvidenceV1 {
    pub manifest: K2UncertaintyOracleCaseEvidenceManifestV1,
    pub public_bindings: K2UncertaintyOraclePublicBindingsV1,
    pub model_set: K2UncertaintyModelSetV1,
    pub frontier: K2UncertaintyFrontierV1,
    pub pages: Vec<K2UncertaintyFrontierPageV1>,
    pub closure_plan: K2UncertaintyClosurePlanV1,
    pub closure_preverification: K2UncertaintyCasePreverificationV2,
    pub baseline_summary: K2UncertaintyBaselineSummaryV1,
    pub observation_vector: K2UncertaintyObservationVectorV2,
    pub final_verifier_receipt: K2UncertaintyConfirmFinalVerifierReceiptV1,
    pub private_truth: K2UncertaintyConfirmFinalTruthCaseV1,
}

pub fn load_self_formed_oracle_case_evidence_v1(
    root: &Path,
    descriptor: &K2UncertaintyOracleBaselineCaseDescriptorV1,
) -> K2CompositionResultV1<K2UncertaintyLoadedOracleCaseEvidenceV1> {
    descriptor.validate()?;
    if !root.is_dir() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_evidence_root_invalid",
        ));
    }
    let manifest_path = root.join(K2_UNCERTAINTY_ORACLE_MANIFEST_FILE_V1);
    require_regular_read_only_file_v1(&manifest_path)?;
    let manifest_bytes = read_bounded_v1(&manifest_path)?;
    let manifest: K2UncertaintyOracleCaseEvidenceManifestV1 =
        uncertainty_decode_v1(&manifest_bytes)?;
    manifest.validate()?;
    if manifest.case_id_sha256 != descriptor.case_id_sha256
        || manifest.manifest_root_sha256 != descriptor.case_evidence_manifest_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_evidence_manifest_binding_invalid",
        ));
    }

    let expected_paths = manifest
        .entries
        .iter()
        .map(|entry| entry.relative_path.clone())
        .chain(std::iter::once(
            K2_UNCERTAINTY_ORACLE_MANIFEST_FILE_V1.to_owned(),
        ))
        .collect::<BTreeSet<_>>();
    let observed_paths = collect_files_v1(root)?;
    if observed_paths != expected_paths {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_evidence_tree_not_closed",
        ));
    }
    for entry in &manifest.entries {
        verify_entry_v1(root, entry)?;
    }

    let public_bindings = decode_single_v1::<K2UncertaintyOraclePublicBindingsV1, _>(
        root,
        &manifest,
        K2UncertaintyOracleEvidenceKindV1::PublicBindings,
        |value| value.bindings_root_sha256.as_str(),
    )?;
    let model_set = decode_single_v1::<K2UncertaintyModelSetV1, _>(
        root,
        &manifest,
        K2UncertaintyOracleEvidenceKindV1::ModelSet,
        |value| value.model_set_root_sha256.as_str(),
    )?;
    let frontier = decode_single_v1::<K2UncertaintyFrontierV1, _>(
        root,
        &manifest,
        K2UncertaintyOracleEvidenceKindV1::FrontierCensus,
        |value| value.frontier_root_sha256.as_str(),
    )?;
    let closure_plan = decode_single_v1::<K2UncertaintyClosurePlanV1, _>(
        root,
        &manifest,
        K2UncertaintyOracleEvidenceKindV1::ClosurePlan,
        |value| value.plan_root_sha256.as_str(),
    )?;
    let closure_preverification = decode_single_v1::<K2UncertaintyCasePreverificationV2, _>(
        root,
        &manifest,
        K2UncertaintyOracleEvidenceKindV1::ClosurePreverification,
        |value| value.receipt_root_sha256.as_str(),
    )?;
    let baseline_summary = decode_single_v1::<K2UncertaintyBaselineSummaryV1, _>(
        root,
        &manifest,
        K2UncertaintyOracleEvidenceKindV1::BaselineSummary,
        |value| value.summary_root_sha256.as_str(),
    )?;
    let observation_vector = decode_single_v1::<K2UncertaintyObservationVectorV2, _>(
        root,
        &manifest,
        K2UncertaintyOracleEvidenceKindV1::ObservationVector,
        |value| value.vector_root_sha256.as_str(),
    )?;
    let final_verifier_receipt = decode_single_v1::<K2UncertaintyConfirmFinalVerifierReceiptV1, _>(
        root,
        &manifest,
        K2UncertaintyOracleEvidenceKindV1::FinalVerifierReceipt,
        |value| value.receipt_root_sha256.as_str(),
    )?;
    let private_truth = decode_single_v1::<K2UncertaintyConfirmFinalTruthCaseV1, _>(
        root,
        &manifest,
        K2UncertaintyOracleEvidenceKindV1::PrivateTruth,
        |value| value.final_truth_root_sha256.as_str(),
    )?;
    let mut page_entries = manifest
        .entries
        .iter()
        .filter(|entry| entry.kind == K2UncertaintyOracleEvidenceKindV1::FrontierPage)
        .collect::<Vec<_>>();
    page_entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let mut pages = Vec::with_capacity(page_entries.len());
    for entry in page_entries {
        let bytes = read_bounded_v1(&root.join(&entry.relative_path))?;
        let page: K2UncertaintyFrontierPageV1 = uncertainty_decode_v1(&bytes)?;
        page.validate()?;
        if page.page_root_sha256 != entry.semantic_root_sha256 {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_frontier_page_semantic_root_mismatch",
            ));
        }
        pages.push(page);
    }

    public_bindings.validate()?;
    model_set.validate()?;
    frontier.validate()?;
    closure_plan.validate()?;
    closure_preverification.validate()?;
    baseline_summary.validate()?;
    observation_vector.validate()?;
    final_verifier_receipt.validate()?;
    private_truth.validate()?;
    validate_descriptor_bindings_v1(
        descriptor,
        &public_bindings,
        &model_set,
        &frontier,
        &closure_plan,
        &closure_preverification,
        &baseline_summary,
        &observation_vector,
        &final_verifier_receipt,
        &private_truth,
    )?;
    Ok(K2UncertaintyLoadedOracleCaseEvidenceV1 {
        manifest,
        public_bindings,
        model_set,
        frontier,
        pages,
        closure_plan,
        closure_preverification,
        baseline_summary,
        observation_vector,
        final_verifier_receipt,
        private_truth,
    })
}

pub fn evaluate_loaded_self_formed_oracle_case_v1(
    descriptor: &K2UncertaintyOracleBaselineCaseDescriptorV1,
    evidence: &K2UncertaintyLoadedOracleCaseEvidenceV1,
) -> K2CompositionResultV1<K2UncertaintyOracleBaselineCaseReceiptV1> {
    let reconstructed = reconstruct_self_formed_oracle_frontier_v1(
        &evidence
            .public_bindings
            .probe_request
            .public_case
            .vocabulary,
        &evidence
            .public_bindings
            .probe_request
            .split_commitment_root_sha256,
        &evidence.model_set,
        &evidence
            .public_bindings
            .probe_request
            .learner_response
            .world_models,
        &evidence.pages,
        &evidence.frontier,
    )?;
    evaluate_self_formed_oracle_case_v1(
        descriptor,
        &evidence.manifest,
        &evidence.model_set,
        &reconstructed.representatives,
        reconstructed.receipt,
        &evidence.closure_plan,
        &evidence.baseline_summary,
        &evidence.observation_vector,
        &evidence.final_verifier_receipt,
        &evidence.private_truth,
    )
}

pub fn run_self_formed_oracle_baseline_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_oracle_stdin"))?;
    let descriptor: K2UncertaintyOracleBaselineCaseDescriptorV1 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_oracle_evaluator"))?;
    if composition_sha256_file_v1(&executable)? != descriptor.oracle_evaluator_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_evaluator_executable_mismatch",
        ));
    }
    let root = std::env::current_dir()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_oracle_evidence_root"))?;
    let evidence = load_self_formed_oracle_case_evidence_v1(&root, &descriptor)?;
    let receipt = evaluate_loaded_self_formed_oracle_case_v1(&descriptor, &evidence)?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_oracle_stdout"))
}

fn verify_entry_v1(
    root: &Path,
    entry: &K2UncertaintyOracleEvidenceEntryV1,
) -> K2CompositionResultV1<()> {
    entry.validate()?;
    let path = root.join(&entry.relative_path);
    require_regular_read_only_file_v1(&path)?;
    let bytes = read_bounded_v1(&path)?;
    if bytes.len() as u64 != entry.byte_len
        || composition_sha256_bytes_v1(&bytes) != entry.content_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_evidence_content_mismatch",
        ));
    }
    Ok(())
}

fn decode_single_v1<T, F>(
    root: &Path,
    manifest: &K2UncertaintyOracleCaseEvidenceManifestV1,
    kind: K2UncertaintyOracleEvidenceKindV1,
    semantic_root: F,
) -> K2CompositionResultV1<T>
where
    T: serde::de::DeserializeOwned + SerializeValue,
    F: FnOnce(&T) -> &str,
{
    let entry = manifest.entry(kind)?;
    let bytes = read_bounded_v1(&root.join(&entry.relative_path))?;
    let value: T = uncertainty_decode_v1(&bytes)?;
    if semantic_root(&value) != entry.semantic_root_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_evidence_semantic_root_mismatch",
        ));
    }
    Ok(value)
}

trait SerializeValue: serde::Serialize {}
impl<T: serde::Serialize> SerializeValue for T {}

#[allow(clippy::too_many_arguments)]
fn validate_descriptor_bindings_v1(
    descriptor: &K2UncertaintyOracleBaselineCaseDescriptorV1,
    bindings: &K2UncertaintyOraclePublicBindingsV1,
    model_set: &K2UncertaintyModelSetV1,
    frontier: &K2UncertaintyFrontierV1,
    closure_plan: &K2UncertaintyClosurePlanV1,
    closure_preverification: &K2UncertaintyCasePreverificationV2,
    baseline: &K2UncertaintyBaselineSummaryV1,
    observations: &K2UncertaintyObservationVectorV2,
    final_verifier: &K2UncertaintyConfirmFinalVerifierReceiptV1,
    truth: &K2UncertaintyConfirmFinalTruthCaseV1,
) -> K2CompositionResultV1<()> {
    let public_case = &bindings.probe_request.public_case;
    if descriptor.experiment_id_sha256 != bindings.experiment_id_sha256
        || descriptor.public_batch_root_sha256 != bindings.public_batch_root_sha256
        || descriptor.batch_precommit_root_sha256 != bindings.batch_precommit_root_sha256
        || descriptor.all_cases_precommitted_root_sha256
            != bindings.all_cases_precommitted_root_sha256
        || descriptor.case_sequence != bindings.case_sequence
        || descriptor.case_id_sha256 != public_case.vocabulary.case_id_sha256
        || descriptor.public_case_root_sha256 != public_case.public_case_root_sha256
        || descriptor.prepared_case_root_sha256 != bindings.prepared_case_root_sha256
        || descriptor.closure_plan_root_sha256 != closure_plan.plan_root_sha256
        || descriptor.baseline_summary_root_sha256 != baseline.summary_root_sha256
        || descriptor.observation_vector_root_sha256 != observations.vector_root_sha256
        || descriptor.final_verifier_receipt_root_sha256 != final_verifier.receipt_root_sha256
        || descriptor.private_truth_artifact_root_sha256 != truth.final_truth_root_sha256
        || bindings.probe_request.learner_response.model_set != *model_set
        || bindings.frontier_root_sha256 != frontier.frontier_root_sha256
        || bindings.closure_preverification_root_sha256
            != closure_preverification.receipt_root_sha256
        || bindings.selection_preverification_root_sha256
            != closure_preverification
                .selection_preverification
                .receipt_root_sha256
        || bindings.baseline_summary_root_sha256 != baseline.summary_root_sha256
        || closure_preverification.closure_plan.as_ref() != Some(closure_plan)
        || closure_preverification
            .selection_preverification
            .baseline_summary
            != *baseline
        || final_verifier.verification.observation_vector_root_sha256
            != observations.vector_root_sha256
        || truth.private_case.public_case_root_sha256 != public_case.public_case_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_descriptor_evidence_binding_invalid",
        ));
    }
    Ok(())
}

fn require_regular_read_only_file_v1(path: &Path) -> K2CompositionResultV1<()> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_oracle_evidence"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.permissions().mode() & 0o777 != 0o400
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_evidence_mode_invalid",
        ));
    }
    Ok(())
}

fn read_bounded_v1(path: &Path) -> K2CompositionResultV1<Vec<u8>> {
    let file = fs::File::open(path)
        .map_err(|_| K2CompositionErrorV1::Io("open_self_formed_oracle_evidence"))?;
    let mut bytes = Vec::new();
    file.take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_oracle_evidence"))?;
    if bytes.len() > K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_evidence_file_too_large",
        ));
    }
    Ok(bytes)
}

fn collect_files_v1(root: &Path) -> K2CompositionResultV1<BTreeSet<String>> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = BTreeSet::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_oracle_directory"))?
        {
            let entry =
                entry.map_err(|_| K2CompositionErrorV1::Io("read_self_formed_oracle_entry"))?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_oracle_entry"))?;
            if metadata.file_type().is_symlink() {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_oracle_evidence_symlink_forbidden",
                ));
            }
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() {
                files.insert(relative_string_v1(root, &path)?);
            } else {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_oracle_evidence_special_file_forbidden",
                ));
            }
        }
    }
    Ok(files)
}

fn relative_string_v1(root: &Path, path: &Path) -> K2CompositionResultV1<String> {
    path.strip_prefix(root)
        .map_err(|_| K2CompositionErrorV1::Invalid("self_formed_oracle_path_escape"))?
        .to_str()
        .map(str::to_owned)
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_path_non_utf8",
        ))
}
