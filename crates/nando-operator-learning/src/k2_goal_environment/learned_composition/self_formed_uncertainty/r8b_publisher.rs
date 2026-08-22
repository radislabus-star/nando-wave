use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_sha256_bytes_v1, composition_sha256_file_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2UncertaintyImmutablePublicationFaultV1,
    K2UncertaintyR8BAuthorizationReceiptV3, denied_authority_v1,
    immutable_publication_temp_relative_path_v1, publish_immutable_file_v1, read_immutable_file_v1,
    recover_linked_publication_temp_v1, require_denied_authority_v1, uncertainty_bytes_v1,
    uncertainty_decode_v1, uncertainty_root_v1,
};

pub const K2_UNCERTAINTY_R8B_PUBLICATION_REQUEST_SCHEMA_V3: &str =
    "nando.k2-self-formed-r8b-publication-request.v3";
pub const K2_UNCERTAINTY_R8B_PUBLICATION_RECEIPT_SCHEMA_V3: &str =
    "nando.k2-self-formed-r8b-publication-receipt.v3";
pub const K2_UNCERTAINTY_R8B_RECEIPT_PATH_V3: &str = "R8B_RECEIPT_V3.json";
pub const K2_UNCERTAINTY_R8B_PUBLICATION_ID_V3: u64 = 0;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BPublicationRequestV3 {
    pub schema: String,
    pub publication_root: String,
    pub authorization: K2UncertaintyR8BAuthorizationReceiptV3,
    pub publisher_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintyR8BPublicationRequestV3 {
    pub fn seal(
        publication_root: String,
        authorization: K2UncertaintyR8BAuthorizationReceiptV3,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_R8B_PUBLICATION_REQUEST_SCHEMA_V3.to_owned(),
            publication_root,
            publisher_executable_sha256: authorization.publisher_executable_sha256.clone(),
            authorization,
            authority: denied_authority_v1(),
            request_root_sha256: String::new(),
        };
        value.reseal()?;
        Ok(value)
    }

    fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.request_root_sha256.clear();
        self.request_root_sha256 = uncertainty_root_v1(self)?;
        self.validate()
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.authorization.validate()?;
        require_composition_root_v1(&self.publisher_executable_sha256)?;
        require_denied_authority_v1(&self.authority)?;
        let mut canonical = self.clone();
        canonical.request_root_sha256.clear();
        if self.schema != K2_UNCERTAINTY_R8B_PUBLICATION_REQUEST_SCHEMA_V3
            || !Path::new(&self.publication_root).is_absolute()
            || self.publisher_executable_sha256 != self.authorization.publisher_executable_sha256
            || self.request_root_sha256 != uncertainty_root_v1(&canonical)?
        {
            return Err(invalid("self_formed_r8b_publication_request_invalid"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BPublicationReceiptV3 {
    pub schema: String,
    pub request_root_sha256: String,
    pub authorization_receipt_root_sha256: String,
    pub relative_path: String,
    pub unix_mode: u32,
    pub byte_len: u64,
    pub content_sha256: String,
    pub publisher_executable_sha256: String,
    pub disposition: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyR8BPublicationReceiptV3 {
    fn seal(
        request: &K2UncertaintyR8BPublicationRequestV3,
        byte_len: u64,
        content_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_R8B_PUBLICATION_RECEIPT_SCHEMA_V3.to_owned(),
            request_root_sha256: request.request_root_sha256.clone(),
            authorization_receipt_root_sha256: request.authorization.receipt_root_sha256.clone(),
            relative_path: K2_UNCERTAINTY_R8B_RECEIPT_PATH_V3.to_owned(),
            unix_mode: 0o400,
            byte_len,
            content_sha256,
            publisher_executable_sha256: request.publisher_executable_sha256.clone(),
            disposition: "R8B_FROZEN".to_owned(),
            authority: denied_authority_v1(),
            receipt_root_sha256: String::new(),
        };
        value.receipt_root_sha256 = rooted_receipt_v3(&value)?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.request_root_sha256,
            &self.authorization_receipt_root_sha256,
            &self.content_sha256,
            &self.publisher_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_R8B_PUBLICATION_RECEIPT_SCHEMA_V3
            || self.relative_path != K2_UNCERTAINTY_R8B_RECEIPT_PATH_V3
            || self.unix_mode != 0o400
            || self.byte_len == 0
            || self.disposition != "R8B_FROZEN"
            || self.receipt_root_sha256 != rooted_receipt_v3(self)?
        {
            return Err(invalid("self_formed_r8b_publication_receipt_invalid"));
        }
        Ok(())
    }
}

pub fn publish_self_formed_r8b_v3(
    request: &K2UncertaintyR8BPublicationRequestV3,
) -> K2CompositionResultV1<K2UncertaintyR8BPublicationReceiptV3> {
    publish_self_formed_r8b_with_fault_v3(request, K2UncertaintyImmutablePublicationFaultV1::None)
}

pub(crate) fn publish_self_formed_r8b_with_fault_v3(
    request: &K2UncertaintyR8BPublicationRequestV3,
    fault: K2UncertaintyImmutablePublicationFaultV1,
) -> K2CompositionResultV1<K2UncertaintyR8BPublicationReceiptV3> {
    request.validate()?;
    let bytes = uncertainty_bytes_v1(&request.authorization)?;
    let published = publish_immutable_file_v1(
        Path::new(&request.publication_root),
        K2_UNCERTAINTY_R8B_RECEIPT_PATH_V3,
        &bytes,
        0o400,
        K2_UNCERTAINTY_R8B_PUBLICATION_ID_V3,
        fault,
    )?;
    if published.bytes != bytes
        || published.content_sha256 != composition_sha256_bytes_v1(&bytes)
        || published.unix_mode != 0o400
    {
        return Err(invalid("self_formed_r8b_published_bytes_invalid"));
    }
    K2UncertaintyR8BPublicationReceiptV3::seal(
        request,
        published.byte_len,
        published.content_sha256,
    )
}

pub fn recover_self_formed_r8b_publication_v3(
    request: &K2UncertaintyR8BPublicationRequestV3,
) -> K2CompositionResultV1<K2UncertaintyR8BPublicationReceiptV3> {
    request.validate()?;
    let root = Path::new(&request.publication_root);
    let bytes = uncertainty_bytes_v1(&request.authorization)?;
    let temporary = immutable_publication_temp_relative_path_v1(
        K2_UNCERTAINTY_R8B_RECEIPT_PATH_V3,
        K2_UNCERTAINTY_R8B_PUBLICATION_ID_V3,
    )?;
    if fs::symlink_metadata(root.join(temporary)).is_err() {
        return Err(invalid("self_formed_r8b_publication_recovery_temp_missing"));
    }
    recover_linked_publication_temp_v1(
        root,
        K2_UNCERTAINTY_R8B_RECEIPT_PATH_V3,
        &bytes,
        0o400,
        K2_UNCERTAINTY_R8B_PUBLICATION_ID_V3,
    )?;
    let published =
        read_immutable_file_v1(root, K2_UNCERTAINTY_R8B_RECEIPT_PATH_V3, 0o400, bytes.len())?;
    if published.bytes != bytes || published.content_sha256 != composition_sha256_bytes_v1(&bytes) {
        return Err(invalid("self_formed_r8b_recovered_bytes_invalid"));
    }
    K2UncertaintyR8BPublicationReceiptV3::seal(
        request,
        published.byte_len,
        published.content_sha256,
    )
}

pub fn run_self_formed_r8b_evidence_publisher_process_v3() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_r8b_publisher_stdin"))?;
    let request: K2UncertaintyR8BPublicationRequestV3 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_r8b_publisher"))?;
    if composition_sha256_file_v1(&executable)? != request.publisher_executable_sha256 {
        return Err(invalid("self_formed_r8b_publisher_executable_mismatch"));
    }
    let receipt = publish_self_formed_r8b_v3(&request)?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_r8b_publisher_stdout"))
}

fn rooted_receipt_v3(
    value: &K2UncertaintyR8BPublicationReceiptV3,
) -> K2CompositionResultV1<String> {
    let mut canonical = value.clone();
    canonical.receipt_root_sha256.clear();
    uncertainty_root_v1(&canonical)
}

fn invalid(reason: &'static str) -> K2CompositionErrorV1 {
    K2CompositionErrorV1::Invalid(reason)
}
