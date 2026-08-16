use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_sha256_bytes_v1, require_composition_root_v1, valid_composition_path_v1,
};
use super::{
    K2_UNCERTAINTY_ACTIONS_V1, K2_UNCERTAINTY_CONFIRM_CASES_V1,
    K2_UNCERTAINTY_CONFIRM_FINAL_TRUTH_SCHEMA_V1, K2_UNCERTAINTY_CONFIRM_PRIVATE_SPLIT_SCHEMA_V1,
    K2_UNCERTAINTY_CONFIRM_PUBLIC_DENOMINATOR_SCHEMA_V1,
    K2_UNCERTAINTY_CONFIRM_RESOLVER_TABLE_SCHEMA_V1,
    K2_UNCERTAINTY_CONFIRM_SPLIT_RECEIPT_SCHEMA_V1,
    K2_UNCERTAINTY_CONFIRM_STORED_ARTIFACT_SCHEMA_V1, K2UncertaintyConfirmGeneratorResponseV1,
    K2UncertaintyPrivateCaseV1, K2UncertaintyPrivateMappingEntryV1, denied_authority_v1,
    require_denied_authority_v1, require_sorted_unique_v1, uncertainty_root_v1,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyConfirmStoredArtifactKindV1 {
    PublicBatch,
    PublicDenominator,
    ResolverTable,
    FinalTruth,
    PrivateSplit,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyConfirmStoredArtifactV1 {
    pub schema: String,
    pub kind: K2UncertaintyConfirmStoredArtifactKindV1,
    pub case_id_sha256: Option<String>,
    pub relative_path: String,
    pub mode: u32,
    pub byte_len: u64,
    pub content_sha256: String,
    pub semantic_root_sha256: String,
    pub artifact_root_sha256: String,
}

impl K2UncertaintyConfirmStoredArtifactV1 {
    pub(crate) fn seal(
        kind: K2UncertaintyConfirmStoredArtifactKindV1,
        case_id_sha256: Option<String>,
        relative_path: String,
        mode: u32,
        bytes: &[u8],
        semantic_root_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut artifact = Self {
            schema: K2_UNCERTAINTY_CONFIRM_STORED_ARTIFACT_SCHEMA_V1.to_owned(),
            kind,
            case_id_sha256,
            relative_path,
            mode,
            byte_len: bytes.len() as u64,
            content_sha256: composition_sha256_bytes_v1(bytes),
            semantic_root_sha256,
            artifact_root_sha256: String::new(),
        };
        artifact.artifact_root_sha256 = artifact.expected_root()?;
        artifact.validate()?;
        Ok(artifact)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.content_sha256)?;
        require_composition_root_v1(&self.semantic_root_sha256)?;
        if let Some(case_id) = &self.case_id_sha256 {
            require_composition_root_v1(case_id)?;
        }
        let mode = match self.kind {
            K2UncertaintyConfirmStoredArtifactKindV1::PublicBatch
            | K2UncertaintyConfirmStoredArtifactKindV1::PublicDenominator => 0o600,
            K2UncertaintyConfirmStoredArtifactKindV1::ResolverTable
            | K2UncertaintyConfirmStoredArtifactKindV1::FinalTruth
            | K2UncertaintyConfirmStoredArtifactKindV1::PrivateSplit => 0o400,
        };
        let case_binding = match self.kind {
            K2UncertaintyConfirmStoredArtifactKindV1::ResolverTable
            | K2UncertaintyConfirmStoredArtifactKindV1::FinalTruth => self.case_id_sha256.is_some(),
            _ => self.case_id_sha256.is_none(),
        };
        if self.schema != K2_UNCERTAINTY_CONFIRM_STORED_ARTIFACT_SCHEMA_V1
            || !valid_composition_path_v1(&self.relative_path)
            || self.mode != mode
            || self.byte_len == 0
            || !case_binding
            || self.artifact_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_stored_artifact_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONFIRM_STORED_ARTIFACT_SCHEMA_V1,
            self.kind,
            &self.case_id_sha256,
            &self.relative_path,
            self.mode,
            self.byte_len,
            &self.content_sha256,
            &self.semantic_root_sha256,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyConfirmPublicDenominatorReceiptV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub public_batch_root_sha256: String,
    pub expected_denominator_commitment_sha256: String,
    pub generator_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyConfirmPublicDenominatorReceiptV1 {
    pub(crate) fn seal(
        response: &K2UncertaintyConfirmGeneratorResponseV1,
        generator_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut receipt = Self {
            schema: K2_UNCERTAINTY_CONFIRM_PUBLIC_DENOMINATOR_SCHEMA_V1.to_owned(),
            experiment_id_sha256: response.public.experiment_id_sha256.clone(),
            public_batch_root_sha256: response.public.public_batch_root_sha256.clone(),
            expected_denominator_commitment_sha256: response
                .private
                .expected_denominator_commitment_sha256
                .clone(),
            generator_executable_sha256,
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
            &self.public_batch_root_sha256,
            &self.expected_denominator_commitment_sha256,
            &self.generator_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CONFIRM_PUBLIC_DENOMINATOR_SCHEMA_V1
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_public_denominator_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONFIRM_PUBLIC_DENOMINATOR_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.public_batch_root_sha256,
            &self.expected_denominator_commitment_sha256,
            &self.generator_executable_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyConfirmResolverTableV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub case_id_sha256: String,
    pub public_case_root_sha256: String,
    pub mapping: Vec<K2UncertaintyPrivateMappingEntryV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub resolver_table_root_sha256: String,
}

impl K2UncertaintyConfirmResolverTableV1 {
    pub(crate) fn seal(case: &K2UncertaintyPrivateCaseV1) -> K2CompositionResultV1<Self> {
        let mut table = Self {
            schema: K2_UNCERTAINTY_CONFIRM_RESOLVER_TABLE_SCHEMA_V1.to_owned(),
            experiment_id_sha256: case.experiment_id_sha256.clone(),
            case_id_sha256: case.case_id_sha256.clone(),
            public_case_root_sha256: case.public_case_root_sha256.clone(),
            mapping: case.mapping.clone(),
            authority: denied_authority_v1(),
            resolver_table_root_sha256: String::new(),
        };
        table.resolver_table_root_sha256 = table.expected_root()?;
        table.validate()?;
        Ok(table)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_id_sha256,
            &self.case_id_sha256,
            &self.public_case_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        for mapping in &self.mapping {
            mapping.validate()?;
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CONFIRM_RESOLVER_TABLE_SCHEMA_V1
            || self.mapping.len() != K2_UNCERTAINTY_ACTIONS_V1
            || !self.mapping.windows(2).all(|pair| pair[0] < pair[1])
            || self.resolver_table_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_resolver_table_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONFIRM_RESOLVER_TABLE_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.case_id_sha256,
            &self.public_case_root_sha256,
            &self.mapping,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyConfirmFinalTruthCaseV1 {
    pub schema: String,
    pub private_case: K2UncertaintyPrivateCaseV1,
    pub expected_denominator_commitment_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub final_truth_root_sha256: String,
}

impl K2UncertaintyConfirmFinalTruthCaseV1 {
    pub(crate) fn seal(
        private_case: K2UncertaintyPrivateCaseV1,
        expected_denominator_commitment_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut truth = Self {
            schema: K2_UNCERTAINTY_CONFIRM_FINAL_TRUTH_SCHEMA_V1.to_owned(),
            private_case,
            expected_denominator_commitment_sha256,
            authority: denied_authority_v1(),
            final_truth_root_sha256: String::new(),
        };
        truth.final_truth_root_sha256 = truth.expected_root()?;
        truth.validate()?;
        Ok(truth)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.private_case.validate()?;
        require_composition_root_v1(&self.expected_denominator_commitment_sha256)?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CONFIRM_FINAL_TRUTH_SCHEMA_V1
            || self.final_truth_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_final_truth_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONFIRM_FINAL_TRUTH_SCHEMA_V1,
            &self.private_case,
            &self.expected_denominator_commitment_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyConfirmPrivateSplitReceiptV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub generator_request_root_sha256: String,
    pub generator_response_root_sha256: String,
    pub artifacts: Vec<K2UncertaintyConfirmStoredArtifactV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub private_split_root_sha256: String,
}

impl K2UncertaintyConfirmPrivateSplitReceiptV1 {
    pub(crate) fn seal(
        response: &K2UncertaintyConfirmGeneratorResponseV1,
        mut artifacts: Vec<K2UncertaintyConfirmStoredArtifactV1>,
    ) -> K2CompositionResultV1<Self> {
        artifacts.sort();
        let mut receipt = Self {
            schema: K2_UNCERTAINTY_CONFIRM_PRIVATE_SPLIT_SCHEMA_V1.to_owned(),
            experiment_id_sha256: response.public.experiment_id_sha256.clone(),
            generator_request_root_sha256: response.generator_request_root_sha256.clone(),
            generator_response_root_sha256: response.response_root_sha256.clone(),
            artifacts,
            authority: denied_authority_v1(),
            private_split_root_sha256: String::new(),
        };
        receipt.private_split_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_id_sha256,
            &self.generator_request_root_sha256,
            &self.generator_response_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        for artifact in &self.artifacts {
            artifact.validate()?;
        }
        require_sorted_unique_v1(
            &self.artifacts,
            "self_formed_confirm_private_artifacts_invalid",
        )?;
        require_denied_authority_v1(&self.authority)?;
        let resolver = artifact_cases_v1(
            &self.artifacts,
            K2UncertaintyConfirmStoredArtifactKindV1::ResolverTable,
        );
        let truth = artifact_cases_v1(
            &self.artifacts,
            K2UncertaintyConfirmStoredArtifactKindV1::FinalTruth,
        );
        if self.schema != K2_UNCERTAINTY_CONFIRM_PRIVATE_SPLIT_SCHEMA_V1
            || self.artifacts.len() != K2_UNCERTAINTY_CONFIRM_CASES_V1 * 2
            || resolver.len() != K2_UNCERTAINTY_CONFIRM_CASES_V1
            || resolver != truth
            || self.private_split_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_private_split_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONFIRM_PRIVATE_SPLIT_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.generator_request_root_sha256,
            &self.generator_response_root_sha256,
            &self.artifacts,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyConfirmSplitReceiptV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub generator_request_root_sha256: String,
    pub generator_response_root_sha256: String,
    pub public_batch_root_sha256: String,
    pub public_denominator_root_sha256: String,
    pub private_split_root_sha256: String,
    pub artifacts: Vec<K2UncertaintyConfirmStoredArtifactV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub split_receipt_root_sha256: String,
}

impl K2UncertaintyConfirmSplitReceiptV1 {
    pub(crate) fn seal(
        response: &K2UncertaintyConfirmGeneratorResponseV1,
        denominator: &K2UncertaintyConfirmPublicDenominatorReceiptV1,
        private: &K2UncertaintyConfirmPrivateSplitReceiptV1,
        mut artifacts: Vec<K2UncertaintyConfirmStoredArtifactV1>,
    ) -> K2CompositionResultV1<Self> {
        artifacts.sort();
        let mut receipt = Self {
            schema: K2_UNCERTAINTY_CONFIRM_SPLIT_RECEIPT_SCHEMA_V1.to_owned(),
            experiment_id_sha256: response.public.experiment_id_sha256.clone(),
            generator_request_root_sha256: response.generator_request_root_sha256.clone(),
            generator_response_root_sha256: response.response_root_sha256.clone(),
            public_batch_root_sha256: response.public.public_batch_root_sha256.clone(),
            public_denominator_root_sha256: denominator.receipt_root_sha256.clone(),
            private_split_root_sha256: private.private_split_root_sha256.clone(),
            artifacts,
            authority: denied_authority_v1(),
            split_receipt_root_sha256: String::new(),
        };
        receipt.split_receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_id_sha256,
            &self.generator_request_root_sha256,
            &self.generator_response_root_sha256,
            &self.public_batch_root_sha256,
            &self.public_denominator_root_sha256,
            &self.private_split_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        for artifact in &self.artifacts {
            artifact.validate()?;
        }
        require_sorted_unique_v1(
            &self.artifacts,
            "self_formed_confirm_split_artifacts_invalid",
        )?;
        require_denied_authority_v1(&self.authority)?;
        let kinds = self
            .artifacts
            .iter()
            .map(|artifact| artifact.kind)
            .collect::<Vec<_>>();
        if self.schema != K2_UNCERTAINTY_CONFIRM_SPLIT_RECEIPT_SCHEMA_V1
            || kinds
                != [
                    K2UncertaintyConfirmStoredArtifactKindV1::PublicBatch,
                    K2UncertaintyConfirmStoredArtifactKindV1::PublicDenominator,
                    K2UncertaintyConfirmStoredArtifactKindV1::PrivateSplit,
                ]
            || self.split_receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_split_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONFIRM_SPLIT_RECEIPT_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.generator_request_root_sha256,
            &self.generator_response_root_sha256,
            &self.public_batch_root_sha256,
            &self.public_denominator_root_sha256,
            &self.private_split_root_sha256,
            &self.artifacts,
            &self.authority,
        ))
    }
}

fn artifact_cases_v1(
    artifacts: &[K2UncertaintyConfirmStoredArtifactV1],
    kind: K2UncertaintyConfirmStoredArtifactKindV1,
) -> BTreeSet<String> {
    artifacts
        .iter()
        .filter(|artifact| artifact.kind == kind)
        .filter_map(|artifact| artifact.case_id_sha256.clone())
        .collect()
}
