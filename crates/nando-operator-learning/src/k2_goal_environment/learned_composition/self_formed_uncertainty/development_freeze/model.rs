use serde::{Deserialize, Serialize};

use super::super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    require_composition_root_v1,
};
use super::super::{
    K2_UNCERTAINTY_CONFIRM_CASES_V1, K2_UNCERTAINTY_MAX_BATCH_WALL_MS_V1,
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1,
    denied_authority_v1, require_denied_authority_v1, uncertainty_root_v1,
};
use super::{
    K2_UNCERTAINTY_CONFIRM_READ_CAPABILITY_SCHEMA_V1,
    K2_UNCERTAINTY_DEVELOPMENT_FREEZE_INPUT_SCHEMA_V1, K2_UNCERTAINTY_DEVELOPMENT_FREEZE_SCHEMA_V1,
    K2_UNCERTAINTY_DEVELOPMENT_RESULT_SCHEMA_V1, K2_UNCERTAINTY_FROZEN_MANIFEST_SCHEMA_V1,
    PRODUCTION_DASHBOARD_SOURCE_SHA256_V1, PRODUCTION_SERVING_SOURCE_SHA256_V1,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyFrozenManifestKindV1 {
    Contract,
    Source,
    Executable,
    TestGate,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyFrozenManifestInputV1 {
    pub kind: K2UncertaintyFrozenManifestKindV1,
    pub entry_count: u64,
    pub byte_len: u64,
    pub content_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyFrozenManifestV1 {
    pub schema: String,
    pub kind: K2UncertaintyFrozenManifestKindV1,
    pub entry_count: u64,
    pub byte_len: u64,
    pub content_sha256: String,
    pub manifest_root_sha256: String,
}

impl K2UncertaintyFrozenManifestV1 {
    fn seal(input: &K2UncertaintyFrozenManifestInputV1) -> K2CompositionResultV1<Self> {
        input.validate()?;
        let manifest_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_FROZEN_MANIFEST_SCHEMA_V1,
            input.kind,
            input.entry_count,
            input.byte_len,
            &input.content_sha256,
        ))?;
        let value = Self {
            schema: K2_UNCERTAINTY_FROZEN_MANIFEST_SCHEMA_V1.to_owned(),
            kind: input.kind,
            entry_count: input.entry_count,
            byte_len: input.byte_len,
            content_sha256: input.content_sha256.clone(),
            manifest_root_sha256,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.content_sha256)?;
        require_composition_root_v1(&self.manifest_root_sha256)?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_FROZEN_MANIFEST_SCHEMA_V1,
            self.kind,
            self.entry_count,
            self.byte_len,
            &self.content_sha256,
        ))?;
        if self.schema != K2_UNCERTAINTY_FROZEN_MANIFEST_SCHEMA_V1
            || self.entry_count == 0
            || self.byte_len == 0
            || self.manifest_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_frozen_manifest_invalid",
            ));
        }
        Ok(())
    }
}

impl K2UncertaintyFrozenManifestInputV1 {
    fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.content_sha256)?;
        if self.entry_count == 0 || self.byte_len == 0 {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_frozen_manifest_input_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyDevelopmentResultV1 {
    pub schema: String,
    pub package_tests_passed: u64,
    pub package_tests_failed: u64,
    pub package_tests_ignored: u64,
    pub legacy_controls_passed: u64,
    pub v3_controls_passed: u64,
    pub v4_controls_passed: u64,
    pub development_cases_passed: u64,
    pub one_probe_cases: u64,
    pub two_probe_cases: u64,
    pub independent_final_verifications_passed: u64,
    pub false_accepts: u64,
    pub maximum_final_request_bytes: u64,
    pub release_process_duration_ms: u64,
}

impl K2UncertaintyDevelopmentResultV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        let cases = K2_UNCERTAINTY_CONFIRM_CASES_V1 as u64;
        if self.schema != K2_UNCERTAINTY_DEVELOPMENT_RESULT_SCHEMA_V1
            || self.package_tests_passed == 0
            || self.package_tests_failed != 0
            || self.legacy_controls_passed != 32
            || self.v3_controls_passed != 4
            || self.v4_controls_passed != 16
            || self.development_cases_passed != cases
            || self.one_probe_cases != 8
            || self.two_probe_cases != 8
            || self.one_probe_cases + self.two_probe_cases != cases
            || self.independent_final_verifications_passed != cases
            || self.false_accepts != 0
            || self.maximum_final_request_bytes == 0
            || self.maximum_final_request_bytes > K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 as u64
            || self.release_process_duration_ms == 0
            || self.release_process_duration_ms > K2_UNCERTAINTY_MAX_BATCH_WALL_MS_V1
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_development_result_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyConfirmReadCapabilityV1 {
    pub schema: String,
    pub generator_executable_sha256: String,
    pub one_shot_file_descriptor_transport: bool,
    pub separate_r10_authorization_required: bool,
    pub interaction_performed: bool,
    pub sealed_execution_performed: bool,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub capability_root_sha256: String,
}

impl K2UncertaintyConfirmReadCapabilityV1 {
    fn seal(generator_executable_sha256: String) -> K2CompositionResultV1<Self> {
        require_composition_root_v1(&generator_executable_sha256)?;
        let authority = denied_authority_v1();
        let capability_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONFIRM_READ_CAPABILITY_SCHEMA_V1,
            &generator_executable_sha256,
            true,
            true,
            false,
            false,
            &authority,
        ))?;
        let value = Self {
            schema: K2_UNCERTAINTY_CONFIRM_READ_CAPABILITY_SCHEMA_V1.to_owned(),
            generator_executable_sha256,
            one_shot_file_descriptor_transport: true,
            separate_r10_authorization_required: true,
            interaction_performed: false,
            sealed_execution_performed: false,
            authority,
            capability_root_sha256,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.generator_executable_sha256)?;
        require_composition_root_v1(&self.capability_root_sha256)?;
        require_denied_authority_v1(&self.authority)?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONFIRM_READ_CAPABILITY_SCHEMA_V1,
            &self.generator_executable_sha256,
            self.one_shot_file_descriptor_transport,
            self.separate_r10_authorization_required,
            self.interaction_performed,
            self.sealed_execution_performed,
            &self.authority,
        ))?;
        if self.schema != K2_UNCERTAINTY_CONFIRM_READ_CAPABILITY_SCHEMA_V1
            || !self.one_shot_file_descriptor_transport
            || !self.separate_r10_authorization_required
            || self.interaction_performed
            || self.sealed_execution_performed
            || self.capability_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_read_capability_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyDevelopmentFreezeInputV1 {
    pub schema: String,
    pub frozen_commit_sha1: String,
    pub manifests: Vec<K2UncertaintyFrozenManifestInputV1>,
    pub development_result: K2UncertaintyDevelopmentResultV1,
    pub r8_receipt_sha256: String,
    pub selector_source_sha256: String,
    pub production_serving_source_sha256: String,
    pub production_dashboard_source_sha256: String,
    pub generator_executable_sha256: String,
    pub freeze_owner_executable_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyDevelopmentFreezeV1 {
    pub schema: String,
    pub frozen_commit_sha1: String,
    pub manifests: Vec<K2UncertaintyFrozenManifestV1>,
    pub development_result: K2UncertaintyDevelopmentResultV1,
    pub development_result_root_sha256: String,
    pub r8_receipt_sha256: String,
    pub selector_source_sha256: String,
    pub production_serving_source_sha256: String,
    pub production_dashboard_source_sha256: String,
    pub generator_executable_sha256: String,
    pub freeze_owner_executable_sha256: String,
    pub confirm_read_capability: K2UncertaintyConfirmReadCapabilityV1,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub freeze_root_sha256: String,
}

pub fn seal_self_formed_development_freeze_v1(
    input: &K2UncertaintyDevelopmentFreezeInputV1,
) -> K2CompositionResultV1<K2UncertaintyDevelopmentFreezeV1> {
    input.validate()?;
    let manifests = input
        .manifests
        .iter()
        .map(K2UncertaintyFrozenManifestV1::seal)
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    let development_result_root_sha256 = uncertainty_root_v1(&input.development_result)?;
    let confirm_read_capability =
        K2UncertaintyConfirmReadCapabilityV1::seal(input.generator_executable_sha256.clone())?;
    let authority = denied_authority_v1();
    let freeze_root_sha256 = uncertainty_root_v1(&(
        K2_UNCERTAINTY_DEVELOPMENT_FREEZE_SCHEMA_V1,
        &input.frozen_commit_sha1,
        &manifests,
        &development_result_root_sha256,
        &input.r8_receipt_sha256,
        &input.selector_source_sha256,
        &input.production_serving_source_sha256,
        &input.production_dashboard_source_sha256,
        &input.generator_executable_sha256,
        &input.freeze_owner_executable_sha256,
        &confirm_read_capability.capability_root_sha256,
        &authority,
    ))?;
    let value = K2UncertaintyDevelopmentFreezeV1 {
        schema: K2_UNCERTAINTY_DEVELOPMENT_FREEZE_SCHEMA_V1.to_owned(),
        frozen_commit_sha1: input.frozen_commit_sha1.clone(),
        manifests,
        development_result: input.development_result.clone(),
        development_result_root_sha256,
        r8_receipt_sha256: input.r8_receipt_sha256.clone(),
        selector_source_sha256: input.selector_source_sha256.clone(),
        production_serving_source_sha256: input.production_serving_source_sha256.clone(),
        production_dashboard_source_sha256: input.production_dashboard_source_sha256.clone(),
        generator_executable_sha256: input.generator_executable_sha256.clone(),
        freeze_owner_executable_sha256: input.freeze_owner_executable_sha256.clone(),
        confirm_read_capability,
        authority,
        freeze_root_sha256,
    };
    value.validate()?;
    Ok(value)
}

impl K2UncertaintyDevelopmentFreezeInputV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_commit_sha1_v1(&self.frozen_commit_sha1)?;
        require_exact_manifest_set_v1(&self.manifests)?;
        self.development_result.validate()?;
        for root in [
            &self.r8_receipt_sha256,
            &self.selector_source_sha256,
            &self.production_serving_source_sha256,
            &self.production_dashboard_source_sha256,
            &self.generator_executable_sha256,
            &self.freeze_owner_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        if self.schema != K2_UNCERTAINTY_DEVELOPMENT_FREEZE_INPUT_SCHEMA_V1
            || self.selector_source_sha256 != K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1
            || self.production_serving_source_sha256 != PRODUCTION_SERVING_SOURCE_SHA256_V1
            || self.production_dashboard_source_sha256 != PRODUCTION_DASHBOARD_SOURCE_SHA256_V1
            || self.generator_executable_sha256 == self.freeze_owner_executable_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_development_freeze_input_invalid",
            ));
        }
        Ok(())
    }
}

impl K2UncertaintyDevelopmentFreezeV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_commit_sha1_v1(&self.frozen_commit_sha1)?;
        let manifest_inputs = self
            .manifests
            .iter()
            .map(|manifest| {
                manifest.validate()?;
                Ok(K2UncertaintyFrozenManifestInputV1 {
                    kind: manifest.kind,
                    entry_count: manifest.entry_count,
                    byte_len: manifest.byte_len,
                    content_sha256: manifest.content_sha256.clone(),
                })
            })
            .collect::<K2CompositionResultV1<Vec<_>>>()?;
        require_exact_manifest_set_v1(&manifest_inputs)?;
        self.development_result.validate()?;
        self.confirm_read_capability.validate()?;
        require_denied_authority_v1(&self.authority)?;
        for root in [
            &self.development_result_root_sha256,
            &self.r8_receipt_sha256,
            &self.selector_source_sha256,
            &self.production_serving_source_sha256,
            &self.production_dashboard_source_sha256,
            &self.generator_executable_sha256,
            &self.freeze_owner_executable_sha256,
            &self.freeze_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        let result_root = uncertainty_root_v1(&self.development_result)?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_DEVELOPMENT_FREEZE_SCHEMA_V1,
            &self.frozen_commit_sha1,
            &self.manifests,
            &self.development_result_root_sha256,
            &self.r8_receipt_sha256,
            &self.selector_source_sha256,
            &self.production_serving_source_sha256,
            &self.production_dashboard_source_sha256,
            &self.generator_executable_sha256,
            &self.freeze_owner_executable_sha256,
            &self.confirm_read_capability.capability_root_sha256,
            &self.authority,
        ))?;
        if self.schema != K2_UNCERTAINTY_DEVELOPMENT_FREEZE_SCHEMA_V1
            || self.development_result_root_sha256 != result_root
            || self.selector_source_sha256 != K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1
            || self.production_serving_source_sha256 != PRODUCTION_SERVING_SOURCE_SHA256_V1
            || self.production_dashboard_source_sha256 != PRODUCTION_DASHBOARD_SOURCE_SHA256_V1
            || self.confirm_read_capability.generator_executable_sha256
                != self.generator_executable_sha256
            || self.freeze_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_development_freeze_invalid",
            ));
        }
        Ok(())
    }
}

fn require_exact_manifest_set_v1(
    manifests: &[K2UncertaintyFrozenManifestInputV1],
) -> K2CompositionResultV1<()> {
    let expected = [
        K2UncertaintyFrozenManifestKindV1::Contract,
        K2UncertaintyFrozenManifestKindV1::Source,
        K2UncertaintyFrozenManifestKindV1::Executable,
        K2UncertaintyFrozenManifestKindV1::TestGate,
    ];
    if manifests.len() != expected.len() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_manifest_set_invalid",
        ));
    }
    for (manifest, kind) in manifests.iter().zip(expected) {
        manifest.validate()?;
        if manifest.kind != kind {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_development_manifest_set_invalid",
            ));
        }
    }
    Ok(())
}

fn require_commit_sha1_v1(value: &str) -> K2CompositionResultV1<()> {
    if value.len() == 40
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && value.bytes().any(|byte| byte != b'0')
    {
        Ok(())
    } else {
        Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_commit_invalid",
        ))
    }
}
