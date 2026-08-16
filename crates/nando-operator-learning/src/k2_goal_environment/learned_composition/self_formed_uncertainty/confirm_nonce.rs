use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_sha256_bytes_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_CONFIRM_NONCE_RECEIPT_SCHEMA_V1, denied_authority_v1,
    require_denied_authority_v1, uncertainty_bytes_v1, uncertainty_decode_v1, uncertainty_root_v1,
};

pub const K2_UNCERTAINTY_CONFIRM_NONCE_RELATIVE_PATH_V1: &str = "private/confirm-nonce.bin";
pub const K2_UNCERTAINTY_CONFIRM_NONCE_RECEIPT_RELATIVE_PATH_V1: &str =
    "private/confirm-nonce-receipt.json";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyConfirmNonceReceiptV1 {
    pub schema: String,
    pub nonce_relative_path: String,
    pub nonce_commitment_sha256: String,
    pub byte_len: u64,
    pub mode: u32,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyConfirmNonceReceiptV1 {
    fn seal(nonce: &[u8; 32]) -> K2CompositionResultV1<Self> {
        let mut receipt = Self {
            schema: K2_UNCERTAINTY_CONFIRM_NONCE_RECEIPT_SCHEMA_V1.to_owned(),
            nonce_relative_path: K2_UNCERTAINTY_CONFIRM_NONCE_RELATIVE_PATH_V1.to_owned(),
            nonce_commitment_sha256: composition_sha256_bytes_v1(nonce),
            byte_len: nonce.len() as u64,
            mode: 0o400,
            authority: denied_authority_v1(),
            receipt_root_sha256: String::new(),
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.nonce_commitment_sha256)?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CONFIRM_NONCE_RECEIPT_SCHEMA_V1
            || self.nonce_relative_path != K2_UNCERTAINTY_CONFIRM_NONCE_RELATIVE_PATH_V1
            || self.byte_len != 32
            || self.mode != 0o400
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_nonce_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONFIRM_NONCE_RECEIPT_SCHEMA_V1,
            &self.nonce_relative_path,
            &self.nonce_commitment_sha256,
            self.byte_len,
            self.mode,
            &self.authority,
        ))
    }
}

pub fn persist_retained_confirm_nonce_v1(
    attempt_root: &Path,
    nonce: &[u8; 32],
) -> K2CompositionResultV1<K2UncertaintyConfirmNonceReceiptV1> {
    let private_root = attempt_root.join("private");
    fs::create_dir(&private_root)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_confirm_private_root"))?;
    fs::set_permissions(&private_root, fs::Permissions::from_mode(0o700))
        .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_confirm_private_root"))?;
    sync_directory_v1(attempt_root, "sync_self_formed_confirm_attempt_root")?;

    let receipt = K2UncertaintyConfirmNonceReceiptV1::seal(nonce)?;
    write_immutable_v1(
        &attempt_root.join(&receipt.nonce_relative_path),
        nonce,
        0o400,
        "create_self_formed_confirm_nonce",
    )?;
    sync_directory_v1(&private_root, "sync_self_formed_confirm_nonce_directory")?;
    write_immutable_v1(
        &attempt_root.join(K2_UNCERTAINTY_CONFIRM_NONCE_RECEIPT_RELATIVE_PATH_V1),
        &uncertainty_bytes_v1(&receipt)?,
        0o400,
        "create_self_formed_confirm_nonce_receipt",
    )?;
    sync_directory_v1(
        &private_root,
        "sync_self_formed_confirm_nonce_receipt_directory",
    )?;
    validate_retained_confirm_nonce_v1(attempt_root, &receipt)?;
    Ok(receipt)
}

pub fn load_retained_confirm_nonce_receipt_v1(
    attempt_root: &Path,
) -> K2CompositionResultV1<K2UncertaintyConfirmNonceReceiptV1> {
    let path = attempt_root.join(K2_UNCERTAINTY_CONFIRM_NONCE_RECEIPT_RELATIVE_PATH_V1);
    let bytes = fs::read(&path)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_confirm_nonce_receipt"))?;
    if file_mode_v1(&path)? != 0o400 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_nonce_receipt_mode_invalid",
        ));
    }
    let receipt = uncertainty_decode_v1(&bytes)?;
    validate_retained_confirm_nonce_v1(attempt_root, &receipt)?;
    Ok(receipt)
}

pub fn retained_confirm_nonce_observed_root_v1(
    attempt_root: &Path,
) -> K2CompositionResultV1<Option<String>> {
    let path = attempt_root.join(K2_UNCERTAINTY_CONFIRM_NONCE_RELATIVE_PATH_V1);
    match fs::read(path) {
        Ok(mut bytes) => {
            let root = uncertainty_root_v1(&(
                "nando.k2-self-formed-observed-retained-nonce.v1",
                composition_sha256_bytes_v1(&bytes),
                bytes.len() as u64,
            ));
            bytes.fill(0);
            std::hint::black_box(&mut bytes);
            Ok(Some(root?))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(K2CompositionErrorV1::Io(
            "inspect_self_formed_confirm_nonce",
        )),
    }
}

fn validate_retained_confirm_nonce_v1(
    attempt_root: &Path,
    receipt: &K2UncertaintyConfirmNonceReceiptV1,
) -> K2CompositionResultV1<()> {
    receipt.validate()?;
    let path = attempt_root.join(&receipt.nonce_relative_path);
    let metadata = fs::metadata(&path)
        .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_confirm_nonce"))?;
    let mut bytes =
        fs::read(&path).map_err(|_| K2CompositionErrorV1::Io("read_self_formed_confirm_nonce"))?;
    let content_sha256 = composition_sha256_bytes_v1(&bytes);
    bytes.fill(0);
    std::hint::black_box(&mut bytes);
    if !metadata.is_file()
        || metadata.len() != receipt.byte_len
        || file_mode_v1(&path)? != receipt.mode
        || content_sha256 != receipt.nonce_commitment_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_nonce_artifact_invalid",
        ));
    }
    Ok(())
}

fn write_immutable_v1(
    path: &Path,
    bytes: &[u8],
    mode: u32,
    reason: &'static str,
) -> K2CompositionResultV1<()> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(mode)
        .open(path)
        .map_err(|_| K2CompositionErrorV1::Io(reason))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_confirm_nonce_artifact"))?;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_confirm_nonce_artifact"))
}

fn file_mode_v1(path: &Path) -> K2CompositionResultV1<u32> {
    Ok(fs::metadata(path)
        .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_confirm_nonce_artifact"))?
        .permissions()
        .mode()
        & 0o777)
}

fn sync_directory_v1(path: &Path, reason: &'static str) -> K2CompositionResultV1<()> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io(reason))
}
