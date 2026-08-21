use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::super::{K2CompositionErrorV1, K2CompositionResultV1, composition_sha256_bytes_v1};
use super::{
    K2_UNCERTAINTY_CONFIRM_CASES_V1, K2_UNCERTAINTY_CONFIRM_FINAL_TRUTH_SCHEMA_V1,
    K2_UNCERTAINTY_CONFIRM_PUBLIC_DENOMINATOR_SCHEMA_V1,
    K2_UNCERTAINTY_CONFIRM_RESOLVER_TABLE_SCHEMA_V1, K2_UNCERTAINTY_DEVELOPMENT_OWNER_PATH_V1,
    K2_UNCERTAINTY_DEVELOPMENT_SEED_COMMITMENT_V1, K2_UNCERTAINTY_DEVELOPMENT_SPLIT_PATH_V1,
    K2_UNCERTAINTY_GENERATOR_RESPONSE_SCHEMA_V1, K2_UNCERTAINTY_IMMUTABLE_MAX_BYTES_V1,
    K2_UNCERTAINTY_PRIVATE_BATCH_SCHEMA_V1, K2UncertaintyConfirmFinalTruthCaseV1,
    K2UncertaintyConfirmOwnerRequestV1, K2UncertaintyConfirmPipeReceiptV1,
    K2UncertaintyConfirmPublicDenominatorReceiptV1, K2UncertaintyConfirmResolverTableV1,
    K2UncertaintyDevelopmentRehearsalOwnerReceiptV1,
    K2UncertaintyDevelopmentRehearsalSplitReceiptV1,
    K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1,
    K2UncertaintyDevelopmentRehearsalStoredArtifactV1, K2UncertaintyGeneratorResponseV1,
    K2UncertaintyImmutablePublicationFaultV1, K2UncertaintyPrivateBatchV1,
    K2UncertaintyPrivateCaseV1, K2UncertaintyPublicBatchV1, create_private_directory_v1,
    denied_authority_v1, development_private_reconstruction_root_v1, inspect_immutable_file_v1,
    publish_immutable_file_v1, recover_linked_publication_temp_from_final_v1, uncertainty_bytes_v1,
    uncertainty_decode_v1, uncertainty_root_v1,
};

pub const K2_UNCERTAINTY_DEVELOPMENT_SPLIT_PUBLICATION_ID_V1: u64 = 34;
pub const K2_UNCERTAINTY_DEVELOPMENT_OWNER_PUBLICATION_ID_V1: u64 = 35;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2UncertaintyDevelopmentRehearsalFullSplitV1 {
    pub split: K2UncertaintyDevelopmentRehearsalSplitReceiptV1,
    pub public_batch: K2UncertaintyPublicBatchV1,
    pub public_denominator: K2UncertaintyConfirmPublicDenominatorReceiptV1,
    pub private_batch: K2UncertaintyPrivateBatchV1,
    pub generator_response: K2UncertaintyGeneratorResponseV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2UncertaintyDevelopmentRehearsalMetadataV1 {
    pub owner: K2UncertaintyDevelopmentRehearsalOwnerReceiptV1,
    pub split: K2UncertaintyDevelopmentRehearsalSplitReceiptV1,
    pub public_batch: K2UncertaintyPublicBatchV1,
    pub public_denominator: K2UncertaintyConfirmPublicDenominatorReceiptV1,
}

struct DevelopmentPayloadV1 {
    artifact: K2UncertaintyDevelopmentRehearsalStoredArtifactV1,
    bytes: Vec<u8>,
}

pub fn publish_development_rehearsal_split_v1(
    generated_root: &Path,
    owner_request: &K2UncertaintyConfirmOwnerRequestV1,
    owner_executable_sha256: String,
    response: &K2UncertaintyGeneratorResponseV1,
    response_bytes: &[u8],
    pipe_receipt: K2UncertaintyConfirmPipeReceiptV1,
) -> K2CompositionResultV1<K2UncertaintyDevelopmentRehearsalSplitReceiptV1> {
    publish_development_rehearsal_split_with_fault_v1(
        generated_root,
        owner_request,
        owner_executable_sha256,
        response,
        response_bytes,
        pipe_receipt,
        K2UncertaintyImmutablePublicationFaultV1::None,
    )
}

pub fn publish_development_rehearsal_split_with_fault_v1(
    generated_root: &Path,
    owner_request: &K2UncertaintyConfirmOwnerRequestV1,
    owner_executable_sha256: String,
    response: &K2UncertaintyGeneratorResponseV1,
    response_bytes: &[u8],
    pipe_receipt: K2UncertaintyConfirmPipeReceiptV1,
    fault: K2UncertaintyImmutablePublicationFaultV1,
) -> K2CompositionResultV1<K2UncertaintyDevelopmentRehearsalSplitReceiptV1> {
    owner_request.validate()?;
    response.validate()?;
    pipe_receipt.validate()?;
    let generator_request = owner_request.development_generator_request.as_ref().ok_or(
        K2CompositionErrorV1::Invalid("self_formed_development_generator_request_missing"),
    )?;
    generator_request.validate()?;
    let canonical_response = uncertainty_bytes_v1(response)?;
    let canonical_request = uncertainty_bytes_v1(generator_request)?;
    if response_bytes != canonical_response
        || response.generator_request_root_sha256 != generator_request.request_root_sha256
        || response.public.experiment_id_sha256 != owner_request.descriptor.experiment_id_sha256
        || pipe_receipt.generator_request_root_sha256 != generator_request.request_root_sha256
        || pipe_receipt.generator_executable_sha256
            != owner_request.descriptor.generator_executable_sha256
        || pipe_receipt.request_bytes != canonical_request.len() as u64
        || pipe_receipt.response_bytes != canonical_response.len() as u64
        || pipe_receipt.response_bytes_sha256 != composition_sha256_bytes_v1(&canonical_response)
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_pipe_response_binding_invalid",
        ));
    }

    create_development_directories_v1(generated_root)?;
    let denominator =
        development_denominator_v1(response, pipe_receipt.generator_executable_sha256.clone())?;
    let mut payloads = development_payloads_v1(response, &denominator)?;
    payloads.sort_by(|left, right| left.artifact.cmp(&right.artifact));
    let private_case_roots = response
        .private
        .cases
        .iter()
        .map(|case| case.private_case_root_sha256.clone())
        .collect::<Vec<_>>();
    let reconstruction_root = development_private_reconstruction_root_v1(
        &private_case_roots,
        &response.private.private_batch_root_sha256,
        &response.response_root_sha256,
        canonical_response.len() as u64,
        &composition_sha256_bytes_v1(&canonical_response),
    )?;
    let split = K2UncertaintyDevelopmentRehearsalSplitReceiptV1::seal(
        owner_request,
        owner_executable_sha256,
        generator_request.request_root_sha256.clone(),
        response.response_root_sha256.clone(),
        pipe_receipt,
        response.public.public_batch_root_sha256.clone(),
        response.private.private_batch_root_sha256.clone(),
        denominator.receipt_root_sha256.clone(),
        payloads
            .iter()
            .map(|value| value.artifact.clone())
            .collect(),
        reconstruction_root,
    )?;

    for (sequence, payload) in payloads.iter().enumerate() {
        publish_immutable_file_v1(
            generated_root,
            &payload.artifact.relative_path,
            &payload.bytes,
            payload.artifact.unix_mode,
            sequence as u64,
            fault,
        )?;
    }
    reconstruct_development_payloads_v1(generated_root, owner_request, &split)?;
    publish_immutable_file_v1(
        generated_root,
        K2_UNCERTAINTY_DEVELOPMENT_SPLIT_PATH_V1,
        &uncertainty_bytes_v1(&split)?,
        0o600,
        K2_UNCERTAINTY_DEVELOPMENT_SPLIT_PUBLICATION_ID_V1,
        fault,
    )?;
    let reopened = load_development_rehearsal_split_full_v1(generated_root, owner_request)?;
    if reopened.split != split {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_split_reopen_mismatch",
        ));
    }
    Ok(split)
}

pub fn load_development_rehearsal_split_full_v1(
    generated_root: &Path,
    owner_request: &K2UncertaintyConfirmOwnerRequestV1,
) -> K2CompositionResultV1<K2UncertaintyDevelopmentRehearsalFullSplitV1> {
    let bytes = recover_linked_publication_temp_from_final_v1(
        generated_root,
        K2_UNCERTAINTY_DEVELOPMENT_SPLIT_PATH_V1,
        0o600,
        K2_UNCERTAINTY_IMMUTABLE_MAX_BYTES_V1,
        K2_UNCERTAINTY_DEVELOPMENT_SPLIT_PUBLICATION_ID_V1,
    )?;
    let split: K2UncertaintyDevelopmentRehearsalSplitReceiptV1 = uncertainty_decode_v1(&bytes)?;
    let result = reconstruct_development_payloads_v1(generated_root, owner_request, &split)?;
    require_exact_generated_tree_v1(generated_root, &split)?;
    Ok(result)
}

pub fn load_development_rehearsal_owner_full_v1(
    attempt_root: &Path,
    owner_request: &K2UncertaintyConfirmOwnerRequestV1,
) -> K2CompositionResultV1<(
    K2UncertaintyDevelopmentRehearsalOwnerReceiptV1,
    K2UncertaintyDevelopmentRehearsalFullSplitV1,
)> {
    let owner = load_development_owner_receipt_v1(attempt_root)?;
    let full =
        load_development_rehearsal_split_full_v1(&attempt_root.join("generated"), owner_request)?;
    validate_owner_split_binding_v1(&owner, &full.split, owner_request)?;
    Ok((owner, full))
}

pub fn load_development_rehearsal_owner_metadata_v1(
    attempt_root: &Path,
    owner_request: &K2UncertaintyConfirmOwnerRequestV1,
) -> K2CompositionResultV1<K2UncertaintyDevelopmentRehearsalMetadataV1> {
    let owner = load_development_owner_receipt_v1(attempt_root)?;
    let generated_root = attempt_root.join("generated");
    let split_bytes = recover_linked_publication_temp_from_final_v1(
        &generated_root,
        K2_UNCERTAINTY_DEVELOPMENT_SPLIT_PATH_V1,
        0o600,
        K2_UNCERTAINTY_IMMUTABLE_MAX_BYTES_V1,
        K2_UNCERTAINTY_DEVELOPMENT_SPLIT_PUBLICATION_ID_V1,
    )?;
    let split: K2UncertaintyDevelopmentRehearsalSplitReceiptV1 =
        uncertainty_decode_v1(&split_bytes)?;
    split.validate()?;
    validate_split_request_binding_v1(&split, owner_request)?;
    validate_owner_split_binding_v1(&owner, &split, owner_request)?;
    let public_batch: K2UncertaintyPublicBatchV1 = decode_public_artifact_v1(
        &generated_root,
        &split,
        K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::PublicBatch,
    )?;
    let public_denominator: K2UncertaintyConfirmPublicDenominatorReceiptV1 =
        decode_public_artifact_v1(
            &generated_root,
            &split,
            K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::PublicDenominator,
        )?;
    public_batch.validate()?;
    public_denominator.validate()?;
    if public_batch.public_batch_root_sha256 != split.public_batch_root_sha256
        || public_denominator.receipt_root_sha256 != split.public_denominator_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_public_binding_invalid",
        ));
    }
    for artifact in split.artifacts.iter().filter(|value| {
        matches!(
            value.kind,
            K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::ResolverTable
                | K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::FinalTruth
        )
    }) {
        let custody = inspect_immutable_file_v1(
            &generated_root,
            &artifact.relative_path,
            artifact.unix_mode,
            artifact.byte_len as usize,
        )?;
        if custody.byte_len != artifact.byte_len || custody.link_count != 1 {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_development_private_custody_invalid",
            ));
        }
    }
    require_exact_generated_tree_v1(&generated_root, &split)?;
    Ok(K2UncertaintyDevelopmentRehearsalMetadataV1 {
        owner,
        split,
        public_batch,
        public_denominator,
    })
}

fn reconstruct_development_payloads_v1(
    generated_root: &Path,
    owner_request: &K2UncertaintyConfirmOwnerRequestV1,
    split: &K2UncertaintyDevelopmentRehearsalSplitReceiptV1,
) -> K2CompositionResultV1<K2UncertaintyDevelopmentRehearsalFullSplitV1> {
    split.validate()?;
    validate_split_request_binding_v1(split, owner_request)?;
    let public_batch: K2UncertaintyPublicBatchV1 = decode_public_artifact_v1(
        generated_root,
        split,
        K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::PublicBatch,
    )?;
    let denominator: K2UncertaintyConfirmPublicDenominatorReceiptV1 = decode_public_artifact_v1(
        generated_root,
        split,
        K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::PublicDenominator,
    )?;
    public_batch.validate()?;
    denominator.validate()?;
    let mut private_cases = Vec::with_capacity(K2_UNCERTAINTY_CONFIRM_CASES_V1);
    for ordinal in 0..K2_UNCERTAINTY_CONFIRM_CASES_V1 as u64 {
        let resolver_artifact = private_artifact_v1(
            split,
            K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::ResolverTable,
            ordinal,
        )?;
        let truth_artifact = private_artifact_v1(
            split,
            K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::FinalTruth,
            ordinal,
        )?;
        let resolver: K2UncertaintyConfirmResolverTableV1 = uncertainty_decode_v1(
            &read_artifact_v1(generated_root, resolver_artifact, ordinal + 2)?,
        )?;
        let truth: K2UncertaintyConfirmFinalTruthCaseV1 =
            uncertainty_decode_v1(&read_artifact_v1(
                generated_root,
                truth_artifact,
                ordinal + 2 + K2_UNCERTAINTY_CONFIRM_CASES_V1 as u64,
            )?)?;
        resolver.validate()?;
        truth.validate()?;
        let public_case = public_batch
            .cases
            .iter()
            .find(|case| case.vocabulary.case_id_sha256 == resolver.case_id_sha256)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_development_public_case_missing",
            ))?;
        if resolver.case_id_sha256 != truth.private_case.case_id_sha256
            || resolver.mapping != truth.private_case.mapping
            || resolver.public_case_root_sha256 != truth.private_case.public_case_root_sha256
            || resolver.case_id_sha256 != public_case.vocabulary.case_id_sha256
            || resolver.public_case_root_sha256 != public_case.public_case_root_sha256
            || resolver.resolver_table_root_sha256 != resolver_artifact.semantic_root_sha256
            || truth.final_truth_root_sha256 != truth_artifact.semantic_root_sha256
            || truth.expected_denominator_commitment_sha256
                != denominator.expected_denominator_commitment_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_development_private_binding_invalid",
            ));
        }
        private_cases.push(truth.private_case);
    }
    let private_batch =
        reconstructed_private_batch_v1(split, &public_batch, &denominator, private_cases)?;
    let response = reconstructed_response_v1(split, public_batch.clone(), private_batch.clone())?;
    let response_bytes = uncertainty_bytes_v1(&response)?;
    let request = owner_request.development_generator_request.as_ref().ok_or(
        K2CompositionErrorV1::Invalid("self_formed_development_generator_request_missing"),
    )?;
    let request_bytes = uncertainty_bytes_v1(request)?;
    let private_roots = private_batch
        .cases
        .iter()
        .map(|case| case.private_case_root_sha256.clone())
        .collect::<Vec<_>>();
    let reconstruction = development_private_reconstruction_root_v1(
        &private_roots,
        &private_batch.private_batch_root_sha256,
        &response.response_root_sha256,
        response_bytes.len() as u64,
        &composition_sha256_bytes_v1(&response_bytes),
    )?;
    if public_batch.public_batch_root_sha256 != split.public_batch_root_sha256
        || denominator.receipt_root_sha256 != split.public_denominator_root_sha256
        || private_batch.private_batch_root_sha256 != split.private_batch_root_sha256
        || response.response_root_sha256 != split.generator_response_root_sha256
        || split.pipe_receipt.request_bytes != request_bytes.len() as u64
        || split.pipe_receipt.response_bytes != response_bytes.len() as u64
        || split.pipe_receipt.response_bytes_sha256 != composition_sha256_bytes_v1(&response_bytes)
        || reconstruction != split.private_reconstruction_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_reconstruction_invalid",
        ));
    }
    Ok(K2UncertaintyDevelopmentRehearsalFullSplitV1 {
        split: split.clone(),
        public_batch,
        public_denominator: denominator,
        private_batch,
        generator_response: response,
    })
}

fn development_payloads_v1(
    response: &K2UncertaintyGeneratorResponseV1,
    denominator: &K2UncertaintyConfirmPublicDenominatorReceiptV1,
) -> K2CompositionResultV1<Vec<DevelopmentPayloadV1>> {
    let public_bytes = uncertainty_bytes_v1(&response.public)?;
    let denominator_bytes = uncertainty_bytes_v1(denominator)?;
    let mut payloads = vec![
        payload_v1(
            K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::PublicBatch,
            None,
            None,
            "public/public-batch.json".to_owned(),
            0o600,
            public_bytes,
            response.public.public_batch_root_sha256.clone(),
        )?,
        payload_v1(
            K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::PublicDenominator,
            None,
            None,
            "public/denominator-receipt.json".to_owned(),
            0o600,
            denominator_bytes,
            denominator.receipt_root_sha256.clone(),
        )?,
    ];
    for (ordinal, case) in response.private.cases.iter().enumerate() {
        let resolver = development_resolver_v1(case)?;
        payloads.push(payload_v1(
            K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::ResolverTable,
            Some(case.case_id_sha256.clone()),
            Some(ordinal as u64),
            format!("private/resolver/{}.json", case.case_id_sha256),
            0o400,
            uncertainty_bytes_v1(&resolver)?,
            resolver.resolver_table_root_sha256,
        )?);
        let truth = development_truth_v1(
            case.clone(),
            denominator.expected_denominator_commitment_sha256.clone(),
        )?;
        payloads.push(payload_v1(
            K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::FinalTruth,
            Some(case.case_id_sha256.clone()),
            Some(ordinal as u64),
            format!("private/final-truth/{}.json", case.case_id_sha256),
            0o400,
            uncertainty_bytes_v1(&truth)?,
            truth.final_truth_root_sha256,
        )?);
    }
    Ok(payloads)
}

#[allow(clippy::too_many_arguments)]
fn payload_v1(
    kind: K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1,
    case_id: Option<String>,
    ordinal: Option<u64>,
    relative_path: String,
    unix_mode: u32,
    bytes: Vec<u8>,
    semantic_root: String,
) -> K2CompositionResultV1<DevelopmentPayloadV1> {
    let artifact = K2UncertaintyDevelopmentRehearsalStoredArtifactV1::seal(
        kind,
        case_id,
        ordinal,
        relative_path,
        unix_mode,
        bytes.len() as u64,
        composition_sha256_bytes_v1(&bytes),
        semantic_root,
    )?;
    Ok(DevelopmentPayloadV1 { artifact, bytes })
}

fn development_denominator_v1(
    response: &K2UncertaintyGeneratorResponseV1,
    generator_executable_sha256: String,
) -> K2CompositionResultV1<K2UncertaintyConfirmPublicDenominatorReceiptV1> {
    let authority = denied_authority_v1();
    let mut value = K2UncertaintyConfirmPublicDenominatorReceiptV1 {
        schema: K2_UNCERTAINTY_CONFIRM_PUBLIC_DENOMINATOR_SCHEMA_V1.to_owned(),
        experiment_id_sha256: response.public.experiment_id_sha256.clone(),
        public_batch_root_sha256: response.public.public_batch_root_sha256.clone(),
        expected_denominator_commitment_sha256: response
            .private
            .expected_denominator_commitment_sha256
            .clone(),
        generator_executable_sha256,
        authority,
        receipt_root_sha256: String::new(),
    };
    value.receipt_root_sha256 = uncertainty_root_v1(&(
        K2_UNCERTAINTY_CONFIRM_PUBLIC_DENOMINATOR_SCHEMA_V1,
        &value.experiment_id_sha256,
        &value.public_batch_root_sha256,
        &value.expected_denominator_commitment_sha256,
        &value.generator_executable_sha256,
        &value.authority,
    ))?;
    value.validate()?;
    Ok(value)
}

fn development_resolver_v1(
    case: &K2UncertaintyPrivateCaseV1,
) -> K2CompositionResultV1<K2UncertaintyConfirmResolverTableV1> {
    let authority = denied_authority_v1();
    let mut value = K2UncertaintyConfirmResolverTableV1 {
        schema: K2_UNCERTAINTY_CONFIRM_RESOLVER_TABLE_SCHEMA_V1.to_owned(),
        experiment_id_sha256: case.experiment_id_sha256.clone(),
        case_id_sha256: case.case_id_sha256.clone(),
        public_case_root_sha256: case.public_case_root_sha256.clone(),
        mapping: case.mapping.clone(),
        authority,
        resolver_table_root_sha256: String::new(),
    };
    value.resolver_table_root_sha256 = uncertainty_root_v1(&(
        K2_UNCERTAINTY_CONFIRM_RESOLVER_TABLE_SCHEMA_V1,
        &value.experiment_id_sha256,
        &value.case_id_sha256,
        &value.public_case_root_sha256,
        &value.mapping,
        &value.authority,
    ))?;
    value.validate()?;
    Ok(value)
}

fn development_truth_v1(
    private_case: K2UncertaintyPrivateCaseV1,
    denominator: String,
) -> K2CompositionResultV1<K2UncertaintyConfirmFinalTruthCaseV1> {
    let authority = denied_authority_v1();
    let mut value = K2UncertaintyConfirmFinalTruthCaseV1 {
        schema: K2_UNCERTAINTY_CONFIRM_FINAL_TRUTH_SCHEMA_V1.to_owned(),
        private_case,
        expected_denominator_commitment_sha256: denominator,
        authority,
        final_truth_root_sha256: String::new(),
    };
    value.final_truth_root_sha256 = uncertainty_root_v1(&(
        K2_UNCERTAINTY_CONFIRM_FINAL_TRUTH_SCHEMA_V1,
        &value.private_case,
        &value.expected_denominator_commitment_sha256,
        &value.authority,
    ))?;
    value.validate()?;
    Ok(value)
}

fn reconstructed_private_batch_v1(
    split: &K2UncertaintyDevelopmentRehearsalSplitReceiptV1,
    public: &K2UncertaintyPublicBatchV1,
    denominator: &K2UncertaintyConfirmPublicDenominatorReceiptV1,
    cases: Vec<K2UncertaintyPrivateCaseV1>,
) -> K2CompositionResultV1<K2UncertaintyPrivateBatchV1> {
    let value = K2UncertaintyPrivateBatchV1 {
        schema: K2_UNCERTAINTY_PRIVATE_BATCH_SCHEMA_V1.to_owned(),
        experiment_id_sha256: split.experiment_id_sha256.clone(),
        public_batch_root_sha256: public.public_batch_root_sha256.clone(),
        cases,
        expected_denominator_commitment_sha256: denominator
            .expected_denominator_commitment_sha256
            .clone(),
        private_batch_root_sha256: split.private_batch_root_sha256.clone(),
    };
    value.validate()?;
    Ok(value)
}

fn reconstructed_response_v1(
    split: &K2UncertaintyDevelopmentRehearsalSplitReceiptV1,
    public: K2UncertaintyPublicBatchV1,
    private: K2UncertaintyPrivateBatchV1,
) -> K2CompositionResultV1<K2UncertaintyGeneratorResponseV1> {
    let value = K2UncertaintyGeneratorResponseV1 {
        schema: K2_UNCERTAINTY_GENERATOR_RESPONSE_SCHEMA_V1.to_owned(),
        generator_request_root_sha256: split.generator_request_root_sha256.clone(),
        public,
        private,
        authority: denied_authority_v1(),
        response_root_sha256: split.generator_response_root_sha256.clone(),
    };
    value.validate()?;
    Ok(value)
}

fn decode_public_artifact_v1<T: serde::de::DeserializeOwned + serde::Serialize>(
    root: &Path,
    split: &K2UncertaintyDevelopmentRehearsalSplitReceiptV1,
    kind: K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1,
) -> K2CompositionResultV1<T> {
    let artifact = split
        .artifacts
        .iter()
        .find(|value| value.kind == kind)
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_development_public_artifact_missing",
        ))?;
    let sequence =
        u64::from(kind == K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::PublicDenominator);
    uncertainty_decode_v1(&read_artifact_v1(root, artifact, sequence)?)
}

fn private_artifact_v1(
    split: &K2UncertaintyDevelopmentRehearsalSplitReceiptV1,
    kind: K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1,
    ordinal: u64,
) -> K2CompositionResultV1<&K2UncertaintyDevelopmentRehearsalStoredArtifactV1> {
    split
        .artifacts
        .iter()
        .find(|value| value.kind == kind && value.private_case_ordinal == Some(ordinal))
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_development_private_artifact_missing",
        ))
}

fn read_artifact_v1(
    root: &Path,
    artifact: &K2UncertaintyDevelopmentRehearsalStoredArtifactV1,
    publication_id: u64,
) -> K2CompositionResultV1<Vec<u8>> {
    let bytes = recover_linked_publication_temp_from_final_v1(
        root,
        &artifact.relative_path,
        artifact.unix_mode,
        artifact.byte_len as usize,
        publication_id,
    )?;
    if bytes.len() as u64 != artifact.byte_len
        || composition_sha256_bytes_v1(&bytes) != artifact.content_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_artifact_bytes_invalid",
        ));
    }
    Ok(bytes)
}

fn validate_split_request_binding_v1(
    split: &K2UncertaintyDevelopmentRehearsalSplitReceiptV1,
    request: &K2UncertaintyConfirmOwnerRequestV1,
) -> K2CompositionResultV1<()> {
    request.validate()?;
    let generator_request =
        request
            .development_generator_request
            .as_ref()
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_development_generator_request_missing",
            ))?;
    if split.attempt_root_sha256 != request.descriptor.attempt_root_sha256
        || split.owner_request_root_sha256 != request.request_root_sha256
        || split.owner_executable_sha256 != request.descriptor.confirm_owner_executable_sha256
        || split.generator_executable_sha256 != request.descriptor.generator_executable_sha256
        || split.generator_request_root_sha256 != generator_request.request_root_sha256
        || split.experiment_id_sha256 != request.descriptor.experiment_id_sha256
        || split.development_seed_commitment_sha256 != generator_request.seed_commitment_sha256
        || split.development_seed_commitment_sha256 != K2_UNCERTAINTY_DEVELOPMENT_SEED_COMMITMENT_V1
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_split_request_binding_invalid",
        ));
    }
    Ok(())
}

fn validate_owner_split_binding_v1(
    owner: &K2UncertaintyDevelopmentRehearsalOwnerReceiptV1,
    split: &K2UncertaintyDevelopmentRehearsalSplitReceiptV1,
    request: &K2UncertaintyConfirmOwnerRequestV1,
) -> K2CompositionResultV1<()> {
    owner.validate()?;
    if owner.owner_request_root_sha256 != request.request_root_sha256
        || owner.attempt_root_sha256 != split.attempt_root_sha256
        || owner.owner_executable_sha256 != split.owner_executable_sha256
        || owner.generator_executable_sha256 != split.generator_executable_sha256
        || owner.generator_request_root_sha256 != split.generator_request_root_sha256
        || owner.generator_response_root_sha256 != split.generator_response_root_sha256
        || owner.public_batch_root_sha256 != split.public_batch_root_sha256
        || owner.private_batch_root_sha256 != split.private_batch_root_sha256
        || owner.split_receipt_root_sha256 != split.split_receipt_root_sha256
        || owner.pipe_receipt_root_sha256 != split.pipe_receipt_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_owner_split_binding_invalid",
        ));
    }
    Ok(())
}

fn load_development_owner_receipt_v1(
    attempt_root: &Path,
) -> K2CompositionResultV1<K2UncertaintyDevelopmentRehearsalOwnerReceiptV1> {
    let bytes = recover_linked_publication_temp_from_final_v1(
        attempt_root,
        K2_UNCERTAINTY_DEVELOPMENT_OWNER_PATH_V1,
        0o600,
        K2_UNCERTAINTY_IMMUTABLE_MAX_BYTES_V1,
        K2_UNCERTAINTY_DEVELOPMENT_OWNER_PUBLICATION_ID_V1,
    )?;
    let owner: K2UncertaintyDevelopmentRehearsalOwnerReceiptV1 = uncertainty_decode_v1(&bytes)?;
    owner.validate()?;
    Ok(owner)
}

fn create_development_directories_v1(root: &Path) -> K2CompositionResultV1<()> {
    create_private_directory_v1(root)?;
    for relative in [
        "public",
        "private",
        "private/resolver",
        "private/final-truth",
    ] {
        create_private_directory_v1(&root.join(relative))?;
    }
    Ok(())
}

fn require_exact_generated_tree_v1(
    root: &Path,
    split: &K2UncertaintyDevelopmentRehearsalSplitReceiptV1,
) -> K2CompositionResultV1<()> {
    require_names_v1(
        root,
        ["development-split-receipt.json", "private", "public"],
    )?;
    require_names_v1(
        &root.join("public"),
        ["denominator-receipt.json", "public-batch.json"],
    )?;
    require_names_v1(&root.join("private"), ["final-truth", "resolver"])?;
    for (directory, kind) in [
        (
            "private/resolver",
            K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::ResolverTable,
        ),
        (
            "private/final-truth",
            K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::FinalTruth,
        ),
    ] {
        let expected = split
            .artifacts
            .iter()
            .filter(|value| value.kind == kind)
            .map(|value| {
                Path::new(&value.relative_path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect::<BTreeSet<_>>();
        require_names_set_v1(&root.join(directory), &expected)?;
    }
    Ok(())
}

fn require_names_v1<const N: usize>(path: &Path, names: [&str; N]) -> K2CompositionResultV1<()> {
    require_names_set_v1(path, &names.into_iter().map(str::to_owned).collect())
}

fn require_names_set_v1(path: &Path, expected: &BTreeSet<String>) -> K2CompositionResultV1<()> {
    let observed = fs::read_dir(path)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_development_directory"))?
        .map(|entry| {
            entry
                .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_development_entry"))?
                .file_name()
                .into_string()
                .map_err(|_| {
                    K2CompositionErrorV1::Invalid("self_formed_development_entry_name_invalid")
                })
        })
        .collect::<K2CompositionResultV1<BTreeSet<_>>>()?;
    if &observed != expected {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_directory_census_invalid",
        ));
    }
    Ok(())
}
