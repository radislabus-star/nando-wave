use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_CONFIRM_CASES_V1, K2_UNCERTAINTY_PUBLIC_CASE_ARTIFACT_SCHEMA_V1,
    K2_UNCERTAINTY_PUBLIC_COMPONENT_ARTIFACT_SCHEMA_V1,
    K2_UNCERTAINTY_PUBLIC_COORDINATOR_REQUEST_SCHEMA_V1, K2_UNCERTAINTY_PUBLIC_OWNER_SET_SCHEMA_V1,
    K2_UNCERTAINTY_PUBLIC_PRECOMMIT_RECEIPT_SCHEMA_V1,
    K2_UNCERTAINTY_PUBLIC_PREPARED_CASE_SCHEMA_V1, K2UncertaintyBatchPrecommitV2,
    K2UncertaintyCasePreverificationV1, K2UncertaintyCasePreverificationV2,
    K2UncertaintyClosureDispositionV1, K2UncertaintyConfirmPublicDenominatorReceiptV1,
    K2UncertaintyProbeArtifactsV1, K2UncertaintyProbeRequestV1, K2UncertaintyPublicBatchV1,
    denied_authority_v1, require_denied_authority_v1, require_exact_len_v1, uncertainty_root_v1,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyPublicOwnerRoleV1 {
    Learner,
    Probe,
    Selector,
    Baseline,
    SelectionPreverifier,
    ClosurePlanner,
    ClosureVerifier,
}

impl K2UncertaintyPublicOwnerRoleV1 {
    pub const ALL: [Self; 7] = [
        Self::Learner,
        Self::Probe,
        Self::Selector,
        Self::Baseline,
        Self::SelectionPreverifier,
        Self::ClosurePlanner,
        Self::ClosureVerifier,
    ];
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyPublicOwnerV1 {
    pub role: K2UncertaintyPublicOwnerRoleV1,
    pub executable_path: String,
    pub executable_sha256: String,
}

impl K2UncertaintyPublicOwnerV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.executable_sha256)?;
        let path = Path::new(&self.executable_path);
        if !path.is_absolute()
            || self.executable_path.as_bytes().contains(&0)
            || path.components().any(|part| {
                matches!(
                    part,
                    std::path::Component::ParentDir | std::path::Component::CurDir
                )
            })
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_public_owner_path_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyPublicOwnerSetV1 {
    pub schema: String,
    pub owners: Vec<K2UncertaintyPublicOwnerV1>,
    pub owner_set_root_sha256: String,
}

impl K2UncertaintyPublicOwnerSetV1 {
    pub fn seal(mut owners: Vec<K2UncertaintyPublicOwnerV1>) -> K2CompositionResultV1<Self> {
        owners.sort();
        let mut value = Self {
            schema: K2_UNCERTAINTY_PUBLIC_OWNER_SET_SCHEMA_V1.to_owned(),
            owners,
            owner_set_root_sha256: String::new(),
        };
        value.owner_set_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_exact_len_v1(
            self.owners.len(),
            K2UncertaintyPublicOwnerRoleV1::ALL.len(),
            "self_formed_public_owner_count_invalid",
        )?;
        let mut roles = BTreeSet::new();
        let mut paths = BTreeSet::new();
        let mut roots = BTreeSet::new();
        for owner in &self.owners {
            owner.validate()?;
            if !roles.insert(owner.role)
                || !paths.insert(owner.executable_path.as_str())
                || !roots.insert(owner.executable_sha256.as_str())
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_public_owner_identity_duplicate",
                ));
            }
        }
        if roles != K2UncertaintyPublicOwnerRoleV1::ALL.into_iter().collect()
            || !self.owners.windows(2).all(|pair| pair[0] < pair[1])
            || self.schema != K2_UNCERTAINTY_PUBLIC_OWNER_SET_SCHEMA_V1
            || self.owner_set_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_public_owner_set_invalid",
            ));
        }
        Ok(())
    }

    pub fn owner(
        &self,
        role: K2UncertaintyPublicOwnerRoleV1,
    ) -> K2CompositionResultV1<&K2UncertaintyPublicOwnerV1> {
        self.owners
            .iter()
            .find(|owner| owner.role == role)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_public_owner_missing",
            ))
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(K2_UNCERTAINTY_PUBLIC_OWNER_SET_SCHEMA_V1, &self.owners))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyPublicCoordinatorRequestV1 {
    pub schema: String,
    pub public_batch: K2UncertaintyPublicBatchV1,
    pub public_denominator: K2UncertaintyConfirmPublicDenominatorReceiptV1,
    pub owner_set: K2UncertaintyPublicOwnerSetV1,
    pub output_root: String,
    pub coordinator_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintyPublicCoordinatorRequestV1 {
    pub fn seal(
        public_batch: K2UncertaintyPublicBatchV1,
        public_denominator: K2UncertaintyConfirmPublicDenominatorReceiptV1,
        owner_set: K2UncertaintyPublicOwnerSetV1,
        output_root: String,
        coordinator_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_PUBLIC_COORDINATOR_REQUEST_SCHEMA_V1.to_owned(),
            public_batch,
            public_denominator,
            owner_set,
            output_root,
            coordinator_executable_sha256,
            authority: denied_authority_v1(),
            request_root_sha256: String::new(),
        };
        value.request_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.public_batch.validate()?;
        self.public_denominator.validate()?;
        self.owner_set.validate()?;
        require_composition_root_v1(&self.coordinator_executable_sha256)?;
        require_denied_authority_v1(&self.authority)?;
        let output_root = Path::new(&self.output_root);
        if self.schema != K2_UNCERTAINTY_PUBLIC_COORDINATOR_REQUEST_SCHEMA_V1
            || self.public_batch.experiment_id_sha256
                != self.public_denominator.experiment_id_sha256
            || self.public_batch.public_batch_root_sha256
                != self.public_denominator.public_batch_root_sha256
            || !output_root.is_absolute()
            || output_root.components().any(|part| {
                matches!(
                    part,
                    std::path::Component::ParentDir | std::path::Component::CurDir
                )
            })
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_public_coordinator_request_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_PUBLIC_COORDINATOR_REQUEST_SCHEMA_V1,
            &self.public_batch.public_batch_root_sha256,
            &self.public_denominator.receipt_root_sha256,
            &self.owner_set.owner_set_root_sha256,
            &self.output_root,
            &self.coordinator_executable_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyPublicPreparedCaseV1 {
    pub schema: String,
    pub case_sequence: u64,
    pub probe_request: K2UncertaintyProbeRequestV1,
    pub probe_artifacts: K2UncertaintyProbeArtifactsV1,
    pub selection_preverification: K2UncertaintyCasePreverificationV1,
    pub preverification: K2UncertaintyCasePreverificationV2,
    pub prepared_case_root_sha256: String,
}

impl K2UncertaintyPublicPreparedCaseV1 {
    pub fn seal(
        case_sequence: u64,
        probe_request: K2UncertaintyProbeRequestV1,
        probe_artifacts: K2UncertaintyProbeArtifactsV1,
        selection_preverification: K2UncertaintyCasePreverificationV1,
        preverification: K2UncertaintyCasePreverificationV2,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_PUBLIC_PREPARED_CASE_SCHEMA_V1.to_owned(),
            case_sequence,
            probe_request,
            probe_artifacts,
            selection_preverification,
            preverification,
            prepared_case_root_sha256: String::new(),
        };
        value.prepared_case_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.probe_request.validate()?;
        self.probe_artifacts.validate()?;
        self.selection_preverification.validate()?;
        self.preverification.validate()?;
        let case_id = &self.probe_request.public_case.vocabulary.case_id_sha256;
        if self.schema != K2_UNCERTAINTY_PUBLIC_PREPARED_CASE_SCHEMA_V1
            || self.case_sequence >= K2_UNCERTAINTY_CONFIRM_CASES_V1 as u64
            || self.probe_artifacts.case_id_sha256 != *case_id
            || self.selection_preverification.case_id_sha256 != *case_id
            || self.preverification.selection_preverification != self.selection_preverification
            || self.preverification.closure_plan.is_none()
            || matches!(
                self.preverification
                    .closure_verification_receipt
                    .disposition,
                K2UncertaintyClosureDispositionV1::ClosureUnavailable
            )
            || self.prepared_case_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_public_prepared_case_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_PUBLIC_PREPARED_CASE_SCHEMA_V1,
            self.case_sequence,
            &self.probe_request.request_root_sha256,
            &self.probe_artifacts.artifacts_root_sha256,
            &self.selection_preverification.receipt_root_sha256,
            &self.preverification.receipt_root_sha256,
        ))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyPublicComponentKindV1 {
    ProbeRequest,
    ProbeArtifacts,
    SelectionPreverification,
    Preverification,
}

impl K2UncertaintyPublicComponentKindV1 {
    pub const ALL: [Self; 4] = [
        Self::ProbeRequest,
        Self::ProbeArtifacts,
        Self::SelectionPreverification,
        Self::Preverification,
    ];

    pub const fn file_name(self) -> &'static str {
        match self {
            Self::ProbeRequest => "probe-request.json",
            Self::ProbeArtifacts => "probe-artifacts.json",
            Self::SelectionPreverification => "selection-preverification.json",
            Self::Preverification => "preverification.json",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyPublicComponentArtifactV1 {
    pub schema: String,
    pub kind: K2UncertaintyPublicComponentKindV1,
    pub relative_path: String,
    pub content_sha256: String,
    pub byte_len: u64,
    pub mode: u32,
    pub semantic_root_sha256: String,
    pub artifact_root_sha256: String,
}

impl K2UncertaintyPublicComponentArtifactV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.content_sha256,
            &self.semantic_root_sha256,
            &self.artifact_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        if self.schema != K2_UNCERTAINTY_PUBLIC_COMPONENT_ARTIFACT_SCHEMA_V1
            || !self.relative_path.ends_with(self.kind.file_name())
            || self.byte_len == 0
            || self.byte_len > super::K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 as u64
            || self.mode != 0o600
            || self.artifact_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_public_component_artifact_invalid",
            ));
        }
        Ok(())
    }

    pub(crate) fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_PUBLIC_COMPONENT_ARTIFACT_SCHEMA_V1,
            self.kind,
            &self.relative_path,
            &self.content_sha256,
            self.byte_len,
            self.mode,
            &self.semantic_root_sha256,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyPublicCaseArtifactV1 {
    pub schema: String,
    pub case_sequence: u64,
    pub case_id_sha256: String,
    pub components: Vec<K2UncertaintyPublicComponentArtifactV1>,
    pub prepared_case_root_sha256: String,
    pub artifact_root_sha256: String,
}

impl K2UncertaintyPublicCaseArtifactV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [&self.case_id_sha256, &self.prepared_case_root_sha256] {
            require_composition_root_v1(root)?;
        }
        for component in &self.components {
            component.validate()?;
        }
        let kinds = self
            .components
            .iter()
            .map(|component| component.kind)
            .collect::<BTreeSet<_>>();
        if self.schema != K2_UNCERTAINTY_PUBLIC_CASE_ARTIFACT_SCHEMA_V1
            || self.case_sequence >= K2_UNCERTAINTY_CONFIRM_CASES_V1 as u64
            || self.components.len() != K2UncertaintyPublicComponentKindV1::ALL.len()
            || kinds
                != K2UncertaintyPublicComponentKindV1::ALL
                    .into_iter()
                    .collect()
            || !self.components.windows(2).all(|pair| pair[0] < pair[1])
            || self.artifact_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_public_case_artifact_invalid",
            ));
        }
        Ok(())
    }

    pub(crate) fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_PUBLIC_CASE_ARTIFACT_SCHEMA_V1,
            self.case_sequence,
            &self.case_id_sha256,
            &self.components,
            &self.prepared_case_root_sha256,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyPublicPrecommitReceiptV1 {
    pub schema: String,
    pub coordinator_request_root_sha256: String,
    pub experiment_id_sha256: String,
    pub public_batch_root_sha256: String,
    pub public_denominator_root_sha256: String,
    pub owner_set_root_sha256: String,
    pub coordinator_executable_sha256: String,
    pub case_artifacts: Vec<K2UncertaintyPublicCaseArtifactV1>,
    pub batch_precommit: K2UncertaintyBatchPrecommitV2,
    pub public_case_count: u64,
    pub private_mount_count: u64,
    pub all_cases_precommitted: bool,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyPublicPrecommitReceiptV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.coordinator_request_root_sha256,
            &self.experiment_id_sha256,
            &self.public_batch_root_sha256,
            &self.public_denominator_root_sha256,
            &self.owner_set_root_sha256,
            &self.coordinator_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.batch_precommit.validate()?;
        require_exact_len_v1(
            self.case_artifacts.len(),
            K2_UNCERTAINTY_CONFIRM_CASES_V1,
            "self_formed_public_precommit_artifact_count_invalid",
        )?;
        let mut ids = BTreeSet::new();
        for (sequence, artifact) in self.case_artifacts.iter().enumerate() {
            artifact.validate()?;
            if artifact.case_sequence != sequence as u64
                || !ids.insert(artifact.case_id_sha256.as_str())
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_public_precommit_artifact_sequence_invalid",
                ));
            }
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_PUBLIC_PRECOMMIT_RECEIPT_SCHEMA_V1
            || self.batch_precommit.experiment_id_sha256 != self.experiment_id_sha256
            || self.public_case_count != K2_UNCERTAINTY_CONFIRM_CASES_V1 as u64
            || self.private_mount_count != 0
            || !self.all_cases_precommitted
            || !self.batch_precommit.dispatch_permitted
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_public_precommit_receipt_invalid",
            ));
        }
        Ok(())
    }

    pub(crate) fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.receipt_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_PUBLIC_PRECOMMIT_RECEIPT_SCHEMA_V1,
            &self.coordinator_request_root_sha256,
            &self.experiment_id_sha256,
            &self.public_batch_root_sha256,
            &self.public_denominator_root_sha256,
            &self.owner_set_root_sha256,
            &self.coordinator_executable_sha256,
            &self.case_artifacts,
            &self.batch_precommit.batch_root_sha256,
            self.public_case_count,
            self.private_mount_count,
            self.all_cases_precommitted,
            &self.authority,
        ))
    }
}
