use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

use super::super::{K2CompositionErrorV1, K2CompositionResultV1, composition_sha256_bytes_v1};
use super::{
    K2_UNCERTAINTY_CONFIRM_CASES_V1, K2UncertaintyConfirmFinalTruthCaseV1,
    K2UncertaintyConfirmGeneratorRequestV1, K2UncertaintyConfirmGeneratorResponseV1,
    K2UncertaintyConfirmPrivateSplitReceiptV1, K2UncertaintyConfirmPublicDenominatorReceiptV1,
    K2UncertaintyConfirmResolverTableV1, K2UncertaintyConfirmSplitReceiptV1,
    K2UncertaintyConfirmStoredArtifactKindV1, K2UncertaintyConfirmStoredArtifactV1,
    K2UncertaintyPublicBatchV1, uncertainty_bytes_v1, uncertainty_decode_v1,
};

const SPLIT_RECEIPT_PATH_V1: &str = "split-receipt.json";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum K2UncertaintyConfirmArtifactFaultV1 {
    None,
    BeforeRename(u64),
    AfterRename(u64),
}

pub fn publish_confirm_generator_split_v1(
    root: &Path,
    request: &K2UncertaintyConfirmGeneratorRequestV1,
    response: &K2UncertaintyConfirmGeneratorResponseV1,
) -> K2CompositionResultV1<K2UncertaintyConfirmSplitReceiptV1> {
    publish_confirm_generator_split_with_fault_v1(
        root,
        request,
        response,
        K2UncertaintyConfirmArtifactFaultV1::None,
    )
}

pub fn publish_confirm_generator_split_with_fault_v1(
    root: &Path,
    request: &K2UncertaintyConfirmGeneratorRequestV1,
    response: &K2UncertaintyConfirmGeneratorResponseV1,
    fault: K2UncertaintyConfirmArtifactFaultV1,
) -> K2CompositionResultV1<K2UncertaintyConfirmSplitReceiptV1> {
    request.validate()?;
    response.validate()?;
    if response.generator_request_root_sha256 != request.request_root_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_split_request_mismatch",
        ));
    }
    create_split_directories_v1(root)?;

    let denominator = K2UncertaintyConfirmPublicDenominatorReceiptV1::seal(
        response,
        request.generator_executable_sha256.clone(),
    )?;
    let mut private_files = Vec::with_capacity(K2_UNCERTAINTY_CONFIRM_CASES_V1 * 2);
    for case in &response.private.cases {
        let resolver = K2UncertaintyConfirmResolverTableV1::seal(case)?;
        let resolver_bytes = uncertainty_bytes_v1(&resolver)?;
        let resolver_artifact = K2UncertaintyConfirmStoredArtifactV1::seal(
            K2UncertaintyConfirmStoredArtifactKindV1::ResolverTable,
            Some(case.case_id_sha256.clone()),
            format!("private/resolver/{}.json", case.case_id_sha256),
            0o400,
            &resolver_bytes,
            resolver.resolver_table_root_sha256.clone(),
        )?;
        private_files.push((resolver_artifact, resolver_bytes));

        let truth = K2UncertaintyConfirmFinalTruthCaseV1::seal(
            case.clone(),
            response
                .private
                .expected_denominator_commitment_sha256
                .clone(),
        )?;
        let truth_bytes = uncertainty_bytes_v1(&truth)?;
        let truth_artifact = K2UncertaintyConfirmStoredArtifactV1::seal(
            K2UncertaintyConfirmStoredArtifactKindV1::FinalTruth,
            Some(case.case_id_sha256.clone()),
            format!("private/final-truth/{}.json", case.case_id_sha256),
            0o400,
            &truth_bytes,
            truth.final_truth_root_sha256.clone(),
        )?;
        private_files.push((truth_artifact, truth_bytes));
    }
    private_files.sort_by(|left, right| left.0.cmp(&right.0));
    let private = K2UncertaintyConfirmPrivateSplitReceiptV1::seal(
        response,
        private_files
            .iter()
            .map(|(artifact, _)| artifact.clone())
            .collect(),
    )?;

    let public_bytes = uncertainty_bytes_v1(&response.public)?;
    let denominator_bytes = uncertainty_bytes_v1(&denominator)?;
    let private_bytes = uncertainty_bytes_v1(&private)?;
    let mut top_files = [
        (
            K2UncertaintyConfirmStoredArtifactV1::seal(
                K2UncertaintyConfirmStoredArtifactKindV1::PublicBatch,
                None,
                "public/public-batch.json".to_owned(),
                0o600,
                &public_bytes,
                response.public.public_batch_root_sha256.clone(),
            )?,
            public_bytes,
        ),
        (
            K2UncertaintyConfirmStoredArtifactV1::seal(
                K2UncertaintyConfirmStoredArtifactKindV1::PublicDenominator,
                None,
                "public/denominator-receipt.json".to_owned(),
                0o600,
                &denominator_bytes,
                denominator.receipt_root_sha256.clone(),
            )?,
            denominator_bytes,
        ),
        (
            K2UncertaintyConfirmStoredArtifactV1::seal(
                K2UncertaintyConfirmStoredArtifactKindV1::PrivateSplit,
                None,
                "private/private-split-receipt.json".to_owned(),
                0o400,
                &private_bytes,
                private.private_split_root_sha256.clone(),
            )?,
            private_bytes,
        ),
    ];
    top_files.sort_by(|left, right| left.0.cmp(&right.0));
    let receipt = K2UncertaintyConfirmSplitReceiptV1::seal(
        response,
        &denominator,
        &private,
        top_files
            .iter()
            .map(|(artifact, _)| artifact.clone())
            .collect(),
    )?;

    let mut sequence = 0_u64;
    for (artifact, bytes) in top_files.iter().take(2) {
        atomic_write_split_artifact_v1(root, artifact, bytes, sequence, fault)?;
        sequence += 1;
    }
    for (artifact, bytes) in &private_files {
        atomic_write_split_artifact_v1(root, artifact, bytes, sequence, fault)?;
        sequence += 1;
    }
    let private_top = top_files
        .iter()
        .find(|(artifact, _)| {
            artifact.kind == K2UncertaintyConfirmStoredArtifactKindV1::PrivateSplit
        })
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_private_split_artifact_missing",
        ))?;
    atomic_write_split_artifact_v1(root, &private_top.0, &private_top.1, sequence, fault)?;
    sequence += 1;
    atomic_write_path_v1(
        root,
        SPLIT_RECEIPT_PATH_V1,
        &uncertainty_bytes_v1(&receipt)?,
        0o600,
        sequence,
        fault,
    )?;
    validate_confirm_generator_split_v1(root, &receipt)?;
    Ok(receipt)
}

pub fn load_confirm_generator_split_receipt_v1(
    root: &Path,
) -> K2CompositionResultV1<K2UncertaintyConfirmSplitReceiptV1> {
    let path = root.join(SPLIT_RECEIPT_PATH_V1);
    let bytes = fs::read(&path)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_confirm_split_receipt"))?;
    if file_mode_v1(&path)? != 0o600 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_split_receipt_mode_invalid",
        ));
    }
    let receipt = uncertainty_decode_v1(&bytes)?;
    validate_confirm_generator_split_v1(root, &receipt)?;
    Ok(receipt)
}

pub fn validate_confirm_generator_split_v1(
    root: &Path,
    receipt: &K2UncertaintyConfirmSplitReceiptV1,
) -> K2CompositionResultV1<()> {
    receipt.validate()?;
    let public: K2UncertaintyPublicBatchV1 = decode_top_artifact_v1(
        root,
        receipt,
        K2UncertaintyConfirmStoredArtifactKindV1::PublicBatch,
    )?;
    let denominator: K2UncertaintyConfirmPublicDenominatorReceiptV1 = decode_top_artifact_v1(
        root,
        receipt,
        K2UncertaintyConfirmStoredArtifactKindV1::PublicDenominator,
    )?;
    let private: K2UncertaintyConfirmPrivateSplitReceiptV1 = decode_top_artifact_v1(
        root,
        receipt,
        K2UncertaintyConfirmStoredArtifactKindV1::PrivateSplit,
    )?;
    public.validate()?;
    denominator.validate()?;
    private.validate()?;
    if public.experiment_id_sha256 != receipt.experiment_id_sha256
        || public.public_batch_root_sha256 != receipt.public_batch_root_sha256
        || denominator.receipt_root_sha256 != receipt.public_denominator_root_sha256
        || private.private_split_root_sha256 != receipt.private_split_root_sha256
        || private.generator_request_root_sha256 != receipt.generator_request_root_sha256
        || private.generator_response_root_sha256 != receipt.generator_response_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_split_binding_invalid",
        ));
    }
    validate_private_artifacts_v1(root, &public, &denominator, &private)
}

fn decode_top_artifact_v1<T: serde::de::DeserializeOwned + serde::Serialize>(
    root: &Path,
    receipt: &K2UncertaintyConfirmSplitReceiptV1,
    kind: K2UncertaintyConfirmStoredArtifactKindV1,
) -> K2CompositionResultV1<T> {
    let artifact = receipt
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == kind)
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_top_artifact_missing",
        ))?;
    let bytes = read_stored_file_v1(root, artifact)?;
    uncertainty_decode_v1(&bytes)
}

fn validate_private_artifacts_v1(
    root: &Path,
    public: &K2UncertaintyPublicBatchV1,
    denominator: &K2UncertaintyConfirmPublicDenominatorReceiptV1,
    private: &K2UncertaintyConfirmPrivateSplitReceiptV1,
) -> K2CompositionResultV1<()> {
    for artifact in &private.artifacts {
        let bytes = read_stored_file_v1(root, artifact)?;
        let case_id = artifact
            .case_id_sha256
            .as_ref()
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_private_case_id_missing",
            ))?;
        let public_case = public
            .cases
            .iter()
            .find(|case| &case.vocabulary.case_id_sha256 == case_id)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_private_public_case_missing",
            ))?;
        match artifact.kind {
            K2UncertaintyConfirmStoredArtifactKindV1::ResolverTable => {
                let value: K2UncertaintyConfirmResolverTableV1 = uncertainty_decode_v1(&bytes)?;
                value.validate()?;
                if value.case_id_sha256 != *case_id
                    || value.public_case_root_sha256 != public_case.public_case_root_sha256
                    || value.resolver_table_root_sha256 != artifact.semantic_root_sha256
                {
                    return Err(K2CompositionErrorV1::Invalid(
                        "self_formed_confirm_resolver_binding_invalid",
                    ));
                }
            }
            K2UncertaintyConfirmStoredArtifactKindV1::FinalTruth => {
                let value: K2UncertaintyConfirmFinalTruthCaseV1 = uncertainty_decode_v1(&bytes)?;
                value.validate()?;
                if value.private_case.case_id_sha256 != *case_id
                    || value.private_case.public_case_root_sha256
                        != public_case.public_case_root_sha256
                    || value.expected_denominator_commitment_sha256
                        != denominator.expected_denominator_commitment_sha256
                    || value.final_truth_root_sha256 != artifact.semantic_root_sha256
                {
                    return Err(K2CompositionErrorV1::Invalid(
                        "self_formed_confirm_truth_binding_invalid",
                    ));
                }
            }
            _ => {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_confirm_private_artifact_kind_invalid",
                ));
            }
        }
    }
    Ok(())
}

fn create_split_directories_v1(root: &Path) -> K2CompositionResultV1<()> {
    DirBuilder::new()
        .mode(0o700)
        .create(root)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_confirm_split_root"))?;
    fs::set_permissions(root, fs::Permissions::from_mode(0o700))
        .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_confirm_split_root"))?;
    for relative in [
        "public",
        "private",
        "private/resolver",
        "private/final-truth",
    ] {
        let path = root.join(relative);
        DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_confirm_split_directory"))?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
            .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_confirm_split_directory"))?;
    }
    sync_directory_v1(root)
}

fn atomic_write_split_artifact_v1(
    root: &Path,
    artifact: &K2UncertaintyConfirmStoredArtifactV1,
    bytes: &[u8],
    sequence: u64,
    fault: K2UncertaintyConfirmArtifactFaultV1,
) -> K2CompositionResultV1<()> {
    artifact.validate()?;
    if artifact.byte_len != bytes.len() as u64
        || artifact.content_sha256 != composition_sha256_bytes_v1(bytes)
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_artifact_bytes_invalid",
        ));
    }
    atomic_write_path_v1(
        root,
        &artifact.relative_path,
        bytes,
        artifact.mode,
        sequence,
        fault,
    )
}

fn atomic_write_path_v1(
    root: &Path,
    relative_path: &str,
    bytes: &[u8],
    mode: u32,
    sequence: u64,
    fault: K2UncertaintyConfirmArtifactFaultV1,
) -> K2CompositionResultV1<()> {
    let path = root.join(relative_path);
    if path.exists() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_artifact_identity_exists",
        ));
    }
    let parent = path.parent().ok_or(K2CompositionErrorV1::Invalid(
        "self_formed_confirm_artifact_parent_missing",
    ))?;
    let name =
        path.file_name()
            .and_then(|value| value.to_str())
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_artifact_name_invalid",
            ))?;
    let temporary = parent.join(format!(".{name}.{sequence}.tmp"));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(mode)
        .open(&temporary)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_confirm_artifact_temp"))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_confirm_artifact_temp"))?;
    fs::set_permissions(&temporary, fs::Permissions::from_mode(mode))
        .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_confirm_artifact_temp"))?;
    if fault == K2UncertaintyConfirmArtifactFaultV1::BeforeRename(sequence) {
        let _ = fs::remove_file(&temporary);
        return Err(K2CompositionErrorV1::Io(
            "self_formed_confirm_artifact_fault_before_rename",
        ));
    }
    fs::rename(&temporary, &path)
        .map_err(|_| K2CompositionErrorV1::Io("rename_self_formed_confirm_artifact"))?;
    sync_directory_v1(parent)?;
    if fault == K2UncertaintyConfirmArtifactFaultV1::AfterRename(sequence) {
        return Err(K2CompositionErrorV1::Io(
            "self_formed_confirm_artifact_fault_after_rename",
        ));
    }
    Ok(())
}

fn read_stored_file_v1(
    root: &Path,
    artifact: &K2UncertaintyConfirmStoredArtifactV1,
) -> K2CompositionResultV1<Vec<u8>> {
    artifact.validate()?;
    let path = root.join(&artifact.relative_path);
    let bytes = fs::read(&path)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_confirm_stored_file"))?;
    if bytes.len() as u64 != artifact.byte_len
        || composition_sha256_bytes_v1(&bytes) != artifact.content_sha256
        || file_mode_v1(&path)? != artifact.mode
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_stored_file_invalid",
        ));
    }
    Ok(bytes)
}

fn file_mode_v1(path: &Path) -> K2CompositionResultV1<u32> {
    Ok(fs::metadata(path)
        .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_confirm_stored_file"))?
        .permissions()
        .mode()
        & 0o777)
}

fn sync_directory_v1(path: &Path) -> K2CompositionResultV1<()> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_confirm_split_directory"))
}
