use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_bytes_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_AUTHORIZATION_SLOT_CLAIM_SCHEMA_V1,
    K2_UNCERTAINTY_AUTHORIZATION_SLOT_KEY_SCHEMA_V1, K2_UNCERTAINTY_PREREGISTRATION_V2_ROOT_V1,
    K2_UNCERTAINTY_PREREGISTRATION_V3_ROOT_V1, K2_UNCERTAINTY_PREREGISTRATION_V4_ROOT_V1,
    K2_UNCERTAINTY_PREREGISTRATION_V5_ROOT_V1, K2_UNCERTAINTY_R10_AUTHORIZATION_RECEIPT_SCHEMA_V1,
    denied_authority_v1, require_denied_authority_v1, uncertainty_decode_v1, uncertainty_root_v1,
};

const RECEIPT_DIRECTORY_V1: &str = "receipts";
const SLOT_DIRECTORY_V1: &str = "slots";
const PENDING_DIRECTORY_V1: &str = "pending";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyAuthorizationSlotKeyV1 {
    pub schema: String,
    /// Frozen pre-nonce identity, distinct from the nonce-derived batch ID.
    pub experiment_id_sha256: String,
    pub successor_freeze_root_sha256: String,
    pub contract_aggregate_root_sha256: String,
    pub slot_key_root_sha256: String,
}

impl K2UncertaintyAuthorizationSlotKeyV1 {
    pub fn seal(
        experiment_id_sha256: String,
        successor_freeze_root_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut key = Self {
            schema: K2_UNCERTAINTY_AUTHORIZATION_SLOT_KEY_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            successor_freeze_root_sha256,
            contract_aggregate_root_sha256: k2_uncertainty_contract_aggregate_root_v1()?,
            slot_key_root_sha256: String::new(),
        };
        key.slot_key_root_sha256 = key.expected_root()?;
        key.validate()?;
        Ok(key)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.experiment_id_sha256)?;
        require_composition_root_v1(&self.successor_freeze_root_sha256)?;
        require_composition_root_v1(&self.contract_aggregate_root_sha256)?;
        if self.schema != K2_UNCERTAINTY_AUTHORIZATION_SLOT_KEY_SCHEMA_V1
            || self.contract_aggregate_root_sha256 != k2_uncertainty_contract_aggregate_root_v1()?
            || self.slot_key_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_authorization_slot_key_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_AUTHORIZATION_SLOT_KEY_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.successor_freeze_root_sha256,
            &self.contract_aggregate_root_sha256,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR10AuthorizationReceiptV1 {
    pub schema: String,
    pub exact_user_authorization_text: String,
    pub codex_session_id: String,
    pub authorized_at: String,
    pub experiment_id_sha256: String,
    pub successor_freeze_root_sha256: String,
    pub contract_aggregate_root_sha256: String,
    pub executable_manifest_root_sha256: String,
    pub maximum_attempts: u64,
    pub maximum_slot_claims: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyR10AuthorizationReceiptV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        exact_user_authorization_text: String,
        codex_session_id: String,
        authorized_at: String,
        experiment_id_sha256: String,
        successor_freeze_root_sha256: String,
        executable_manifest_root_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut receipt = Self {
            schema: K2_UNCERTAINTY_R10_AUTHORIZATION_RECEIPT_SCHEMA_V1.to_owned(),
            exact_user_authorization_text,
            codex_session_id,
            authorized_at,
            experiment_id_sha256,
            successor_freeze_root_sha256,
            contract_aggregate_root_sha256: k2_uncertainty_contract_aggregate_root_v1()?,
            executable_manifest_root_sha256,
            maximum_attempts: 1,
            maximum_slot_claims: 1,
            authority: denied_authority_v1(),
            receipt_root_sha256: String::new(),
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_id_sha256,
            &self.successor_freeze_root_sha256,
            &self.contract_aggregate_root_sha256,
            &self.executable_manifest_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        let expected_text = required_r10_authorization_text_v1(&self.successor_freeze_root_sha256)?;
        if self.schema != K2_UNCERTAINTY_R10_AUTHORIZATION_RECEIPT_SCHEMA_V1
            || self.exact_user_authorization_text != expected_text
            || !valid_session_id_v1(&self.codex_session_id)
            || !valid_timestamp_text_v1(&self.authorized_at)
            || self.contract_aggregate_root_sha256 != k2_uncertainty_contract_aggregate_root_v1()?
            || self.maximum_attempts != 1
            || self.maximum_slot_claims != 1
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_r10_authorization_receipt_invalid",
            ));
        }
        Ok(())
    }

    pub fn slot_key(&self) -> K2CompositionResultV1<K2UncertaintyAuthorizationSlotKeyV1> {
        self.validate()?;
        K2UncertaintyAuthorizationSlotKeyV1::seal(
            self.experiment_id_sha256.clone(),
            self.successor_freeze_root_sha256.clone(),
        )
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_R10_AUTHORIZATION_RECEIPT_SCHEMA_V1,
            &self.exact_user_authorization_text,
            &self.codex_session_id,
            &self.authorized_at,
            &self.experiment_id_sha256,
            &self.successor_freeze_root_sha256,
            &self.contract_aggregate_root_sha256,
            &self.executable_manifest_root_sha256,
            self.maximum_attempts,
            self.maximum_slot_claims,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyAuthorizationSlotClaimV1 {
    pub schema: String,
    pub slot_key: K2UncertaintyAuthorizationSlotKeyV1,
    pub authorization_receipt_root_sha256: String,
    pub owner_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub claim_root_sha256: String,
}

impl K2UncertaintyAuthorizationSlotClaimV1 {
    fn seal(
        slot_key: K2UncertaintyAuthorizationSlotKeyV1,
        authorization_receipt_root_sha256: String,
        owner_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut claim = Self {
            schema: K2_UNCERTAINTY_AUTHORIZATION_SLOT_CLAIM_SCHEMA_V1.to_owned(),
            slot_key,
            authorization_receipt_root_sha256,
            owner_executable_sha256,
            authority: denied_authority_v1(),
            claim_root_sha256: String::new(),
        };
        claim.claim_root_sha256 = claim.expected_root()?;
        claim.validate()?;
        Ok(claim)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.slot_key.validate()?;
        require_composition_root_v1(&self.authorization_receipt_root_sha256)?;
        require_composition_root_v1(&self.owner_executable_sha256)?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_AUTHORIZATION_SLOT_CLAIM_SCHEMA_V1
            || self.claim_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_authorization_slot_claim_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_AUTHORIZATION_SLOT_CLAIM_SCHEMA_V1,
            &self.slot_key,
            &self.authorization_receipt_root_sha256,
            &self.owner_executable_sha256,
            &self.authority,
        ))
    }
}

pub struct K2UncertaintyAuthorizationSlotLedgerV1 {
    root: PathBuf,
}

impl K2UncertaintyAuthorizationSlotLedgerV1 {
    pub fn open_or_create(root: &Path) -> K2CompositionResultV1<Self> {
        ensure_private_directory_v1(root)?;
        for child in [
            RECEIPT_DIRECTORY_V1,
            SLOT_DIRECTORY_V1,
            PENDING_DIRECTORY_V1,
        ] {
            ensure_private_directory_v1(&root.join(child))?;
        }
        sync_directory_v1(root, "sync_self_formed_slot_ledger_root")?;
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    pub fn open_existing(root: &Path) -> K2CompositionResultV1<Self> {
        let metadata = fs::symlink_metadata(root)
            .map_err(|_| K2CompositionErrorV1::Io("open_self_formed_slot_ledger"))?;
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || metadata.permissions().mode() & 0o777 != 0o700
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_slot_ledger_mode_invalid",
            ));
        }
        for child in [
            RECEIPT_DIRECTORY_V1,
            SLOT_DIRECTORY_V1,
            PENDING_DIRECTORY_V1,
        ] {
            let metadata = fs::symlink_metadata(root.join(child))
                .map_err(|_| K2CompositionErrorV1::Io("open_self_formed_slot_ledger_directory"))?;
            if metadata.file_type().is_symlink()
                || !metadata.is_dir()
                || metadata.permissions().mode() & 0o777 != 0o700
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_slot_ledger_directory_mode_invalid",
                ));
            }
        }
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    pub fn claim(
        &self,
        receipt: &K2UncertaintyR10AuthorizationReceiptV1,
        owner_executable_sha256: String,
    ) -> K2CompositionResultV1<K2UncertaintyAuthorizationSlotClaimV1> {
        receipt.validate()?;
        let claim = K2UncertaintyAuthorizationSlotClaimV1::seal(
            receipt.slot_key()?,
            receipt.receipt_root_sha256.clone(),
            owner_executable_sha256,
        )?;
        let bytes = composition_bytes_v1(&claim)?;
        let pending = self
            .root
            .join(PENDING_DIRECTORY_V1)
            .join(format!("{}.json", claim.claim_root_sha256));
        write_immutable_v1(&pending, &bytes, "create_self_formed_slot_pending")?;
        sync_directory_v1(
            &self.root.join(PENDING_DIRECTORY_V1),
            "sync_self_formed_slot_pending_directory",
        )?;

        let receipt_path = self
            .root
            .join(RECEIPT_DIRECTORY_V1)
            .join(format!("{}.json", receipt.receipt_root_sha256));
        link_unique_v1(
            &pending,
            &receipt_path,
            "self_formed_authorization_receipt_already_used",
            "link_self_formed_slot_receipt",
        )?;
        sync_directory_v1(
            &self.root.join(RECEIPT_DIRECTORY_V1),
            "sync_self_formed_slot_receipt_directory",
        )?;

        let slot_path = self
            .root
            .join(SLOT_DIRECTORY_V1)
            .join(format!("{}.json", claim.slot_key.slot_key_root_sha256));
        link_unique_v1(
            &pending,
            &slot_path,
            "self_formed_authorization_slot_already_claimed",
            "link_self_formed_slot_key",
        )?;
        sync_directory_v1(
            &self.root.join(SLOT_DIRECTORY_V1),
            "sync_self_formed_slot_key_directory",
        )?;
        claim.validate()?;
        Ok(claim)
    }

    pub fn read_slot_claim(
        &self,
        key: &K2UncertaintyAuthorizationSlotKeyV1,
    ) -> K2CompositionResultV1<K2UncertaintyAuthorizationSlotClaimV1> {
        key.validate()?;
        let path = self
            .root
            .join(SLOT_DIRECTORY_V1)
            .join(format!("{}.json", key.slot_key_root_sha256));
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_slot_claim"))?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.permissions().mode() & 0o777 != 0o400
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_authorization_slot_claim_mode_invalid",
            ));
        }
        let bytes =
            fs::read(path).map_err(|_| K2CompositionErrorV1::Io("read_self_formed_slot_claim"))?;
        let claim: K2UncertaintyAuthorizationSlotClaimV1 = uncertainty_decode_v1(&bytes)?;
        claim.validate()?;
        if &claim.slot_key != key {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_authorization_slot_binding_mismatch",
            ));
        }
        Ok(claim)
    }
}

pub fn k2_uncertainty_contract_aggregate_root_v1() -> K2CompositionResultV1<String> {
    uncertainty_root_v1(&(
        "nando.k2-self-formed-contract-aggregate.v1",
        K2_UNCERTAINTY_PREREGISTRATION_V2_ROOT_V1,
        K2_UNCERTAINTY_PREREGISTRATION_V3_ROOT_V1,
        K2_UNCERTAINTY_PREREGISTRATION_V4_ROOT_V1,
        K2_UNCERTAINTY_PREREGISTRATION_V5_ROOT_V1,
    ))
}

pub fn required_r10_authorization_text_v1(
    successor_freeze_root_sha256: &str,
) -> K2CompositionResultV1<String> {
    require_composition_root_v1(successor_freeze_root_sha256)?;
    Ok(format!(
        "Authorize R10: execute exactly one sealed scientific attempt for successor freeze root {successor_freeze_root_sha256} under the V2-V5 contract."
    ))
}

fn valid_session_id_v1(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if [8, 13, 18, 23].contains(&index) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

fn valid_timestamp_text_v1(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.is_ascii()
        && value.bytes().all(|byte| !byte.is_ascii_control())
}

fn ensure_private_directory_v1(path: &Path) -> K2CompositionResultV1<()> {
    fs::create_dir_all(path)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_slot_ledger_directory"))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_slot_ledger_directory"))
}

fn write_immutable_v1(
    path: &Path,
    bytes: &[u8],
    create_error: &'static str,
) -> K2CompositionResultV1<()> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o400)
        .open(path)
        .map_err(|error| {
            if error.kind() == ErrorKind::AlreadyExists {
                K2CompositionErrorV1::Invalid("self_formed_slot_claim_already_attempted")
            } else {
                K2CompositionErrorV1::Io(create_error)
            }
        })?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_slot_claim"))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o400))
        .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_slot_claim"))
}

fn link_unique_v1(
    source: &Path,
    destination: &Path,
    exists_reason: &'static str,
    io_reason: &'static str,
) -> K2CompositionResultV1<()> {
    fs::hard_link(source, destination).map_err(|error| {
        if error.kind() == ErrorKind::AlreadyExists {
            K2CompositionErrorV1::Invalid(exists_reason)
        } else {
            K2CompositionErrorV1::Io(io_reason)
        }
    })
}

fn sync_directory_v1(path: &Path, reason: &'static str) -> K2CompositionResultV1<()> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io(reason))
}
