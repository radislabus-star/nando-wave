use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionLearnedEffectV1,
    K2CompositionResultV1, K2InquiryProbeV1, composition_sha256_file_v1,
    require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2_UNCERTAINTY_PRIVATE_RESOLVER_RECEIPT_SCHEMA_V1,
    K2_UNCERTAINTY_PRIVATE_RESOLVER_REQUEST_SCHEMA_V1, K2UncertaintyClosurePlanV1,
    K2UncertaintyConfirmResolverTableV1, denied_authority_v1, require_denied_authority_v1,
    uncertainty_bytes_v1, uncertainty_decode_v1, uncertainty_root_v1,
};

const RESOLVER_TABLE_PATH_V1: &str = "/private/resolver.json";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyPrivateResolverRequestV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub public_batch_root_sha256: String,
    pub batch_precommit_root_sha256: String,
    pub case_preverification_root_sha256: String,
    pub public_case_root_sha256: String,
    pub closure_plan: K2UncertaintyClosurePlanV1,
    pub probe_ordinal: u64,
    pub selected_probe: K2InquiryProbeV1,
    pub resolver_table_root_sha256: String,
    pub resolver_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintyPrivateResolverRequestV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        experiment_id_sha256: String,
        public_batch_root_sha256: String,
        batch_precommit_root_sha256: String,
        case_preverification_root_sha256: String,
        public_case_root_sha256: String,
        closure_plan: K2UncertaintyClosurePlanV1,
        probe_ordinal: u64,
        selected_probe: K2InquiryProbeV1,
        resolver_table_root_sha256: String,
        resolver_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_PRIVATE_RESOLVER_REQUEST_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            public_batch_root_sha256,
            batch_precommit_root_sha256,
            case_preverification_root_sha256,
            public_case_root_sha256,
            closure_plan,
            probe_ordinal,
            selected_probe,
            resolver_table_root_sha256,
            resolver_executable_sha256,
            authority: denied_authority_v1(),
            request_root_sha256: String::new(),
        };
        value.request_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_id_sha256,
            &self.public_batch_root_sha256,
            &self.batch_precommit_root_sha256,
            &self.case_preverification_root_sha256,
            &self.public_case_root_sha256,
            &self.resolver_table_root_sha256,
            &self.resolver_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.closure_plan.validate()?;
        self.selected_probe.validate()?;
        require_denied_authority_v1(&self.authority)?;
        let expected_probe = self
            .closure_plan
            .ordered_probe_roots_sha256
            .get(self.probe_ordinal as usize);
        if self.schema != K2_UNCERTAINTY_PRIVATE_RESOLVER_REQUEST_SCHEMA_V1
            || self.probe_ordinal >= self.closure_plan.plan_length
            || expected_probe != Some(&self.selected_probe.probe_root_sha256)
            || self.selected_probe.experiment_id_sha256 != self.closure_plan.case_id_sha256
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_private_resolver_request_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_PRIVATE_RESOLVER_REQUEST_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.public_batch_root_sha256,
            &self.batch_precommit_root_sha256,
            &self.case_preverification_root_sha256,
            &self.public_case_root_sha256,
            &self.closure_plan.plan_root_sha256,
            self.probe_ordinal,
            &self.selected_probe.probe_root_sha256,
            &self.selected_probe.action_id_sha256,
            &self.resolver_table_root_sha256,
            &self.resolver_executable_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyPrivateResolverReceiptV1 {
    pub schema: String,
    pub resolver_request_root_sha256: String,
    pub experiment_id_sha256: String,
    pub case_id_sha256: String,
    pub public_case_root_sha256: String,
    pub closure_plan_root_sha256: String,
    pub probe_ordinal: u64,
    pub selected_probe_root_sha256: String,
    pub selected_action_root_sha256: String,
    pub resolved_effect: K2CompositionLearnedEffectV1,
    pub resolved_effect_root_sha256: String,
    pub resolver_table_root_sha256: String,
    pub resolver_executable_sha256: String,
    pub exposed_effect_count: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyPrivateResolverReceiptV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.resolver_request_root_sha256,
            &self.experiment_id_sha256,
            &self.case_id_sha256,
            &self.public_case_root_sha256,
            &self.closure_plan_root_sha256,
            &self.selected_probe_root_sha256,
            &self.selected_action_root_sha256,
            &self.resolved_effect_root_sha256,
            &self.resolver_table_root_sha256,
            &self.resolver_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.resolved_effect.validate()?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_PRIVATE_RESOLVER_RECEIPT_SCHEMA_V1
            || self.exposed_effect_count != 1
            || self.resolved_effect_root_sha256 != uncertainty_root_v1(&self.resolved_effect)?
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_private_resolver_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_PRIVATE_RESOLVER_RECEIPT_SCHEMA_V1,
            &self.resolver_request_root_sha256,
            &self.experiment_id_sha256,
            &self.case_id_sha256,
            &self.public_case_root_sha256,
            &self.closure_plan_root_sha256,
            self.probe_ordinal,
            &self.selected_probe_root_sha256,
            &self.selected_action_root_sha256,
            &self.resolved_effect_root_sha256,
            &self.resolver_table_root_sha256,
            &self.resolver_executable_sha256,
            self.exposed_effect_count,
            &self.authority,
        ))
    }
}

pub fn resolve_self_formed_private_effect_v1(
    request: &K2UncertaintyPrivateResolverRequestV1,
    table: &K2UncertaintyConfirmResolverTableV1,
) -> K2CompositionResultV1<K2UncertaintyPrivateResolverReceiptV1> {
    request.validate()?;
    table.validate()?;
    if table.experiment_id_sha256 != request.experiment_id_sha256
        || table.case_id_sha256 != request.closure_plan.case_id_sha256
        || table.public_case_root_sha256 != request.public_case_root_sha256
        || table.resolver_table_root_sha256 != request.resolver_table_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_private_resolver_table_binding_invalid",
        ));
    }
    let mapping = table
        .mapping
        .iter()
        .find(|mapping| {
            mapping.opaque_action_root_sha256 == request.selected_probe.action_id_sha256
        })
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_private_resolver_action_missing",
        ))?;
    let resolved_effect_root_sha256 = uncertainty_root_v1(&mapping.effect)?;
    let authority = denied_authority_v1();
    let receipt_root_sha256 = uncertainty_root_v1(&(
        K2_UNCERTAINTY_PRIVATE_RESOLVER_RECEIPT_SCHEMA_V1,
        &request.request_root_sha256,
        &request.experiment_id_sha256,
        &request.closure_plan.case_id_sha256,
        &request.public_case_root_sha256,
        &request.closure_plan.plan_root_sha256,
        request.probe_ordinal,
        &request.selected_probe.probe_root_sha256,
        &request.selected_probe.action_id_sha256,
        &resolved_effect_root_sha256,
        &request.resolver_table_root_sha256,
        &request.resolver_executable_sha256,
        1_u64,
        &authority,
    ))?;
    let receipt = K2UncertaintyPrivateResolverReceiptV1 {
        schema: K2_UNCERTAINTY_PRIVATE_RESOLVER_RECEIPT_SCHEMA_V1.to_owned(),
        resolver_request_root_sha256: request.request_root_sha256.clone(),
        experiment_id_sha256: request.experiment_id_sha256.clone(),
        case_id_sha256: request.closure_plan.case_id_sha256.clone(),
        public_case_root_sha256: request.public_case_root_sha256.clone(),
        closure_plan_root_sha256: request.closure_plan.plan_root_sha256.clone(),
        probe_ordinal: request.probe_ordinal,
        selected_probe_root_sha256: request.selected_probe.probe_root_sha256.clone(),
        selected_action_root_sha256: request.selected_probe.action_id_sha256.clone(),
        resolved_effect: mapping.effect.clone(),
        resolved_effect_root_sha256,
        resolver_table_root_sha256: request.resolver_table_root_sha256.clone(),
        resolver_executable_sha256: request.resolver_executable_sha256.clone(),
        exposed_effect_count: 1,
        authority,
        receipt_root_sha256,
    };
    receipt.validate()?;
    Ok(receipt)
}

pub fn run_self_formed_private_resolver_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_private_resolver_stdin"))?;
    let request: K2UncertaintyPrivateResolverRequestV1 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_private_resolver"))?;
    if composition_sha256_file_v1(&executable)? != request.resolver_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_private_resolver_executable_mismatch",
        ));
    }
    let table_path = Path::new(RESOLVER_TABLE_PATH_V1);
    let metadata = fs::symlink_metadata(table_path)
        .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_private_resolver_table"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.permissions().mode() & 0o777 != 0o400
        || metadata.len() > K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 as u64
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_private_resolver_table_file_invalid",
        ));
    }
    let table: K2UncertaintyConfirmResolverTableV1 = uncertainty_decode_v1(
        &fs::read(table_path)
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_private_resolver_table"))?,
    )?;
    let receipt = resolve_self_formed_private_effect_v1(&request, &table)?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_private_resolver_stdout"))
}
