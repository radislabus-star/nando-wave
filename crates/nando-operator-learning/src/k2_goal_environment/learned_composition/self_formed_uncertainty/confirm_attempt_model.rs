use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    require_composition_root_v1, valid_composition_path_v1,
};
use super::{
    K2_UNCERTAINTY_CLASSIFIED_PATH_SCHEMA_V1, K2_UNCERTAINTY_CONFIRM_ATTEMPT_DESCRIPTOR_SCHEMA_V1,
    K2_UNCERTAINTY_CONFIRM_ATTEMPT_EVENT_SCHEMA_V1, K2UncertaintyAuthorizationSlotClaimV1,
    K2UncertaintyR10AuthorizationReceiptV1, denied_authority_v1,
    k2_uncertainty_contract_aggregate_root_v1, require_denied_authority_v1, uncertainty_root_v1,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyConfirmAttemptModeV1 {
    DevelopmentRehearsal,
    Confirm,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyConfirmAttemptDescriptorV1 {
    pub schema: String,
    pub mode: K2UncertaintyConfirmAttemptModeV1,
    /// Frozen pre-nonce identity, distinct from the nonce-derived batch ID.
    pub experiment_id_sha256: String,
    pub successor_freeze_root_sha256: String,
    pub contract_aggregate_root_sha256: String,
    pub executable_manifest_root_sha256: String,
    pub confirm_owner_executable_sha256: String,
    pub generator_executable_sha256: String,
    pub authorization_receipt_root_sha256: Option<String>,
    pub authorization_slot_claim_root_sha256: Option<String>,
    pub sealed_attempts: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub attempt_root_sha256: String,
}

impl K2UncertaintyConfirmAttemptDescriptorV1 {
    pub fn development_rehearsal(
        experiment_id_sha256: String,
        successor_freeze_root_sha256: String,
        executable_manifest_root_sha256: String,
        confirm_owner_executable_sha256: String,
        generator_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        Self::seal(
            K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal,
            experiment_id_sha256,
            successor_freeze_root_sha256,
            executable_manifest_root_sha256,
            confirm_owner_executable_sha256,
            generator_executable_sha256,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn confirm(
        experiment_id_sha256: String,
        successor_freeze_root_sha256: String,
        executable_manifest_root_sha256: String,
        confirm_owner_executable_sha256: String,
        generator_executable_sha256: String,
        authorization: &K2UncertaintyR10AuthorizationReceiptV1,
        slot_claim: &K2UncertaintyAuthorizationSlotClaimV1,
    ) -> K2CompositionResultV1<Self> {
        authorization.validate()?;
        slot_claim.validate()?;
        if authorization.experiment_id_sha256 != experiment_id_sha256
            || authorization.successor_freeze_root_sha256 != successor_freeze_root_sha256
            || authorization.executable_manifest_root_sha256 != executable_manifest_root_sha256
            || slot_claim.authorization_receipt_root_sha256 != authorization.receipt_root_sha256
            || slot_claim.slot_key != authorization.slot_key()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_descriptor_authorization_mismatch",
            ));
        }
        Self::seal(
            K2UncertaintyConfirmAttemptModeV1::Confirm,
            experiment_id_sha256,
            successor_freeze_root_sha256,
            executable_manifest_root_sha256,
            confirm_owner_executable_sha256,
            generator_executable_sha256,
            Some(authorization.receipt_root_sha256.clone()),
            Some(slot_claim.claim_root_sha256.clone()),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn seal(
        mode: K2UncertaintyConfirmAttemptModeV1,
        experiment_id_sha256: String,
        successor_freeze_root_sha256: String,
        executable_manifest_root_sha256: String,
        confirm_owner_executable_sha256: String,
        generator_executable_sha256: String,
        authorization_receipt_root_sha256: Option<String>,
        authorization_slot_claim_root_sha256: Option<String>,
    ) -> K2CompositionResultV1<Self> {
        let mut descriptor = Self {
            schema: K2_UNCERTAINTY_CONFIRM_ATTEMPT_DESCRIPTOR_SCHEMA_V1.to_owned(),
            mode,
            experiment_id_sha256,
            successor_freeze_root_sha256,
            contract_aggregate_root_sha256: k2_uncertainty_contract_aggregate_root_v1()?,
            executable_manifest_root_sha256,
            confirm_owner_executable_sha256,
            generator_executable_sha256,
            authorization_receipt_root_sha256,
            authorization_slot_claim_root_sha256,
            sealed_attempts: u64::from(mode == K2UncertaintyConfirmAttemptModeV1::Confirm),
            authority: denied_authority_v1(),
            attempt_root_sha256: String::new(),
        };
        descriptor.attempt_root_sha256 = descriptor.expected_root()?;
        descriptor.validate()?;
        Ok(descriptor)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_id_sha256,
            &self.successor_freeze_root_sha256,
            &self.contract_aggregate_root_sha256,
            &self.executable_manifest_root_sha256,
            &self.confirm_owner_executable_sha256,
            &self.generator_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        for root in [
            &self.authorization_receipt_root_sha256,
            &self.authorization_slot_claim_root_sha256,
        ]
        .into_iter()
        .flatten()
        {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        let mode_valid = match self.mode {
            K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal => {
                self.authorization_receipt_root_sha256.is_none()
                    && self.authorization_slot_claim_root_sha256.is_none()
                    && self.sealed_attempts == 0
            }
            K2UncertaintyConfirmAttemptModeV1::Confirm => {
                self.authorization_receipt_root_sha256.is_some()
                    && self.authorization_slot_claim_root_sha256.is_some()
                    && self.sealed_attempts == 1
            }
        };
        if self.schema != K2_UNCERTAINTY_CONFIRM_ATTEMPT_DESCRIPTOR_SCHEMA_V1
            || !mode_valid
            || self.contract_aggregate_root_sha256 != k2_uncertainty_contract_aggregate_root_v1()?
            || self.attempt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_attempt_descriptor_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONFIRM_ATTEMPT_DESCRIPTOR_SCHEMA_V1,
            self.mode,
            &self.experiment_id_sha256,
            &self.successor_freeze_root_sha256,
            &self.contract_aggregate_root_sha256,
            &self.executable_manifest_root_sha256,
            &self.confirm_owner_executable_sha256,
            &self.generator_executable_sha256,
            &self.authorization_receipt_root_sha256,
            &self.authorization_slot_claim_root_sha256,
            self.sealed_attempts,
            &self.authority,
        ))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyConfirmAttemptEventKindV1 {
    ArtifactsFrozen,
    NonceCreated,
    NonceCommitted,
    GeneratorDispatched,
    CasesGenerated,
    ModelSetsFrozen,
    ProbeSetsFrozen,
    SelectionsFrozen,
    AllCasesPrecommitted,
    ProbeDispatched,
    ProbeObserved,
    ObservationsFrozen,
    ModelsUpdated,
    ControlsFrozen,
    DevelopmentRehearsalTerminalFrozen,
    ScientificVerdictFrozen,
    NonceCreatedUncommitted,
    NonceCommittedUndispatched,
    GeneratorResultIndeterminate,
    CleanupFrozen,
}

impl K2UncertaintyConfirmAttemptEventKindV1 {
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::DevelopmentRehearsalTerminalFrozen
                | Self::ScientificVerdictFrozen
                | Self::NonceCreatedUncommitted
                | Self::NonceCommittedUndispatched
                | Self::GeneratorResultIndeterminate
                | Self::CleanupFrozen
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyConfirmAttemptEventV1 {
    pub schema: String,
    pub attempt_root_sha256: String,
    pub sequence: u64,
    pub previous_event_root_sha256: Option<String>,
    pub kind: K2UncertaintyConfirmAttemptEventKindV1,
    pub owner_executable_sha256: String,
    pub request_root_sha256: String,
    pub payload_root_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub event_root_sha256: String,
}

impl K2UncertaintyConfirmAttemptEventV1 {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn seal(
        attempt_root_sha256: String,
        sequence: u64,
        previous_event_root_sha256: Option<String>,
        kind: K2UncertaintyConfirmAttemptEventKindV1,
        owner_executable_sha256: String,
        request_root_sha256: String,
        payload_root_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut event = Self {
            schema: K2_UNCERTAINTY_CONFIRM_ATTEMPT_EVENT_SCHEMA_V1.to_owned(),
            attempt_root_sha256,
            sequence,
            previous_event_root_sha256,
            kind,
            owner_executable_sha256,
            request_root_sha256,
            payload_root_sha256,
            authority: denied_authority_v1(),
            event_root_sha256: String::new(),
        };
        event.event_root_sha256 = event.expected_root()?;
        event.validate()?;
        Ok(event)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.attempt_root_sha256,
            &self.owner_executable_sha256,
            &self.request_root_sha256,
            &self.payload_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        if let Some(previous) = &self.previous_event_root_sha256 {
            require_composition_root_v1(previous)?;
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CONFIRM_ATTEMPT_EVENT_SCHEMA_V1
            || (self.sequence == 0) != self.previous_event_root_sha256.is_none()
            || self.event_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_attempt_event_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONFIRM_ATTEMPT_EVENT_SCHEMA_V1,
            &self.attempt_root_sha256,
            self.sequence,
            &self.previous_event_root_sha256,
            self.kind,
            &self.owner_executable_sha256,
            &self.request_root_sha256,
            &self.payload_root_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyClassifiedPathDispositionV1 {
    RetainAlways,
    RetainSealedUntilPostResultReview,
    DeleteAfterTerminalAndObserverFsync,
    SupersededNeverUse,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyClassifiedPathV1 {
    pub schema: String,
    pub relative_path: String,
    pub disposition: K2UncertaintyClassifiedPathDispositionV1,
    pub content_sha256: String,
    pub byte_len: u64,
    pub mode: u32,
    pub path_root_sha256: String,
}

impl K2UncertaintyClassifiedPathV1 {
    pub fn seal(
        relative_path: String,
        disposition: K2UncertaintyClassifiedPathDispositionV1,
        content_sha256: String,
        byte_len: u64,
        mode: u32,
    ) -> K2CompositionResultV1<Self> {
        let mut path = Self {
            schema: K2_UNCERTAINTY_CLASSIFIED_PATH_SCHEMA_V1.to_owned(),
            relative_path,
            disposition,
            content_sha256,
            byte_len,
            mode,
            path_root_sha256: String::new(),
        };
        path.path_root_sha256 = path.expected_root()?;
        path.validate()?;
        Ok(path)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.content_sha256)?;
        if self.schema != K2_UNCERTAINTY_CLASSIFIED_PATH_SCHEMA_V1
            || !valid_composition_path_v1(&self.relative_path)
            || !matches!(self.mode, 0o400 | 0o500 | 0o600 | 0o700)
            || self.path_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_classified_path_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CLASSIFIED_PATH_SCHEMA_V1,
            &self.relative_path,
            self.disposition,
            &self.content_sha256,
            self.byte_len,
            self.mode,
        ))
    }
}
