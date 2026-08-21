use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1, require_composition_root_v1,
};
use super::immutable_publication::decode_canonical_json_v1;
use super::{
    K2_UNCERTAINTY_R8B_CONTROL_WRAPPER_SCHEMA_V3, K2_UNCERTAINTY_R8B_EXECUTABLE_MANIFEST_SCHEMA_V2,
    K2_UNCERTAINTY_R8B_ORACLE_WRAPPER_SCHEMA_V3, K2_UNCERTAINTY_R8B_PRODUCER_REQUEST_SCHEMA_V2,
    K2_UNCERTAINTY_R8B_ROUTE_RECEIPT_SCHEMA_V2, K2_UNCERTAINTY_R8B_SUITE_RECEIPT_SCHEMA_V2,
    K2UncertaintyCleanupReceiptV1, K2UncertaintyControlEvaluationReceiptV1, K2UncertaintyDevelopmentResultReceiptV1,
    K2UncertaintyOracleBaselineBatchReceiptV1,
    K2UncertaintyR8BControlWrapperV3, K2UncertaintyR8BOracleWrapperV3, denied_authority_v1,
    require_denied_authority_v1, uncertainty_root_v1,
};

pub const K2_UNCERTAINTY_R8B_AUTHORIZATION_REQUEST_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-authorization-request.v2";
pub const K2_UNCERTAINTY_R8B_AUTHORIZATION_RECEIPT_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-authorization-receipt.v2";
pub const K2_UNCERTAINTY_R8B_AUTHORIZATION_REQUEST_SCHEMA_V3: &str =
    "nando.k2-self-formed-r8b-authorization-request.v3";
pub const K2_UNCERTAINTY_R8B_AUTHORIZATION_RECEIPT_SCHEMA_V3: &str =
    "nando.k2-self-formed-r8b-authorization-receipt.v3";

#[rustfmt::skip]
pub(super) const LINKED_ROLES_V2: [&str; 26] = [
    "M01_DEVELOPMENT_OWNER", "M02_GENERATOR",
    "M03_LEARNER", "M04_PROBE", "M05_SELECTOR", "M06_BASELINE",
    "M07_SELECTION_PREVERIFIER", "M08_CLOSURE_PLANNER", "M09_CLOSURE_VERIFIER",
    "M10_PUBLIC_COORDINATOR", "M11_PRIVATE_RESOLVER", "M12_SAFETY",
    "M13_WORKER", "M14_OBSERVER", "M15_FINAL_VERIFIER", "M16_ORACLE",
    "M17_CONTROL_EVALUATOR", "M18_TERMINAL_EVALUATOR", "M19_FRESH_CONTROL_CASE",
    "M20_CLEANUP_AUTHORIZER", "M21_CLEANUP_OWNER", "M22_CLEANUP_VERIFIER",
    "M23_DEVELOPMENT_RESULT_PUBLISHER", "M24_LINKED_RUNNER",
    "M25_R8B_AUTHORIZER", "M26_R8B_PUBLISHER",
];

#[rustfmt::skip]
pub(super) const SUITE_ROLES_V2: [&str; 5] = [
    "S01_CRATE_UNIT", "S02_RESTART", "S03_MODE_MATRIX",
    "S04_CLEANUP_NEGATIVE", "S05_AUTHORITY_PUBLICATION",
];

pub(super) fn valid_r8b_role_v3(role: &str) -> bool {
    LINKED_ROLES_V2.contains(&role) || SUITE_ROLES_V2.contains(&role)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BAuthorizationRequestV2 {
    pub schema: String,
    pub route_id_sha256: String,
    pub manifest_root_sha256: String,
    pub authorizer_executable_sha256: String,
    pub request_root_sha256: String,
}

impl K2UncertaintyR8BAuthorizationRequestV2 {
    pub fn seal(
        route_id_sha256: String,
        manifest_root_sha256: String,
        authorizer_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_R8B_AUTHORIZATION_REQUEST_SCHEMA_V2.to_owned(),
            route_id_sha256,
            manifest_root_sha256,
            authorizer_executable_sha256,
            request_root_sha256: String::new(),
        };
        let mut canonical = value.clone();
        canonical.request_root_sha256.clear();
        value.request_root_sha256 = uncertainty_root_v1(&canonical)?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [&self.route_id_sha256, &self.manifest_root_sha256, &self.authorizer_executable_sha256] {
            require_composition_root_v1(root)?;
        }
        let mut canonical = self.clone();
        canonical.request_root_sha256.clear();
        if self.schema != K2_UNCERTAINTY_R8B_AUTHORIZATION_REQUEST_SCHEMA_V2
            || self.request_root_sha256 != uncertainty_root_v1(&canonical)?
        {
            return Err(invalid("self_formed_r8b_authorization_request_invalid"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BAuthorizationReceiptV2 {
    pub schema: String,
    pub request_root_sha256: String,
    pub tested_commit_sha256: String,
    pub route_id_sha256: String,
    pub manifest_root_sha256: String,
    pub linked_manifest_root_sha256: String,
    pub suite_manifest_root_sha256: String,
    pub process_ledger_root_sha256: String,
    pub entry_roots_sha256: Vec<String>,
    pub publisher_executable_sha256: String,
    pub disposition: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyR8BAuthorizationReceiptV2 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.request_root_sha256,
            &self.tested_commit_sha256,
            &self.route_id_sha256,
            &self.manifest_root_sha256,
            &self.linked_manifest_root_sha256,
            &self.suite_manifest_root_sha256,
            &self.process_ledger_root_sha256,
            &self.publisher_executable_sha256,
        ]
        .into_iter()
        .chain(self.entry_roots_sha256.iter())
        {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        let mut canonical = self.clone();
        canonical.receipt_root_sha256.clear();
        if self.schema != K2_UNCERTAINTY_R8B_AUTHORIZATION_RECEIPT_SCHEMA_V2
            || self.entry_roots_sha256.is_empty()
            || self.disposition != "R8B_FROZEN"
            || self.receipt_root_sha256 != uncertainty_root_v1(&canonical)?
        {
            return Err(invalid("self_formed_r8b_authorization_receipt_invalid"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BAuthorizationRequestV3 {
    pub schema: String,
    pub route_id_sha256: String,
    pub manifest_root_sha256: String,
    pub authorizer_executable_sha256: String,
    pub request_root_sha256: String,
}

impl K2UncertaintyR8BAuthorizationRequestV3 {
    pub fn seal(
        route_id_sha256: String,
        manifest_root_sha256: String,
        authorizer_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_R8B_AUTHORIZATION_REQUEST_SCHEMA_V3.to_owned(),
            route_id_sha256,
            manifest_root_sha256,
            authorizer_executable_sha256,
            request_root_sha256: String::new(),
        };
        value.request_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [&self.route_id_sha256, &self.manifest_root_sha256, &self.authorizer_executable_sha256] {
            require_composition_root_v1(root)?;
        }
        if self.schema != K2_UNCERTAINTY_R8B_AUTHORIZATION_REQUEST_SCHEMA_V3
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(invalid("self_formed_r8b_authorization_request_v3_invalid"));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        let mut value = self.clone();
        value.request_root_sha256.clear();
        uncertainty_root_v1(&value)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BAuthorizationReceiptV3 {
    pub schema: String,
    pub request_root_sha256: String,
    pub route_id_sha256: String,
    pub manifest_root_sha256: String,
    pub c08_projection_root_sha256: String,
    pub resource_receipt_root_sha256: String,
    pub ledger_seal_root_sha256: String,
    pub packet_member_roots_sha256: Vec<String>,
    pub publisher_executable_sha256: String,
    pub disposition: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyR8BAuthorizationReceiptV3 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.request_root_sha256,
            &self.route_id_sha256,
            &self.manifest_root_sha256,
            &self.c08_projection_root_sha256,
            &self.resource_receipt_root_sha256,
            &self.ledger_seal_root_sha256,
            &self.publisher_executable_sha256,
        ]
        .into_iter()
        .chain(self.packet_member_roots_sha256.iter())
        {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        let mut canonical = self.clone();
        canonical.receipt_root_sha256.clear();
        if self.schema != K2_UNCERTAINTY_R8B_AUTHORIZATION_RECEIPT_SCHEMA_V3
            || self.packet_member_roots_sha256.len() != 22
            || self.packet_member_roots_sha256.windows(2).any(|pair| pair[0] >= pair[1])
            || self.disposition != "R8B_FROZEN"
            || self.receipt_root_sha256 != uncertainty_root_v1(&canonical)?
        {
            return Err(invalid("self_formed_r8b_authorization_receipt_v3_invalid"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BExecutableIdentityV2 {
    pub role: String,
    pub canonical_path: String,
    pub byte_len: u64,
    pub unix_mode: u32,
    pub sha256: String,
}

impl K2UncertaintyR8BExecutableIdentityV2 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.sha256)?;
        if !Path::new(&self.canonical_path).is_absolute() || self.byte_len == 0 || self.unix_mode & 0o111 == 0 {
            return Err(invalid("self_formed_r8b_executable_identity_invalid"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyR8BManifestClassV2 {
    Linked,
    Suite,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BExecutableManifestV2 {
    pub schema: String,
    pub class: K2UncertaintyR8BManifestClassV2,
    pub identities: Vec<K2UncertaintyR8BExecutableIdentityV2>,
    pub manifest_root_sha256: String,
}

impl K2UncertaintyR8BExecutableManifestV2 {
    pub fn seal(
        class: K2UncertaintyR8BManifestClassV2,
        mut identities: Vec<K2UncertaintyR8BExecutableIdentityV2>,
    ) -> K2CompositionResultV1<Self> {
        identities.sort_by(|left, right| left.role.cmp(&right.role));
        let mut value = Self {
            schema: K2_UNCERTAINTY_R8B_EXECUTABLE_MANIFEST_SCHEMA_V2.to_owned(),
            class,
            identities,
            manifest_root_sha256: String::new(),
        };
        value.manifest_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        let expected = match self.class {
            K2UncertaintyR8BManifestClassV2::Linked => LINKED_ROLES_V2.as_slice(),
            K2UncertaintyR8BManifestClassV2::Suite => SUITE_ROLES_V2.as_slice(),
        };
        let roles = self.identities.iter().map(|identity| identity.role.as_str()).collect::<Vec<_>>();
        let paths = self.identities.iter().map(|identity| identity.canonical_path.as_str()).collect::<BTreeSet<_>>();
        let hashes = self.identities.iter().map(|identity| identity.sha256.as_str()).collect::<BTreeSet<_>>();
        for identity in &self.identities {
            identity.validate()?;
        }
        if self.schema != K2_UNCERTAINTY_R8B_EXECUTABLE_MANIFEST_SCHEMA_V2
            || roles != expected
            || paths.len() != expected.len()
            || hashes.len() != expected.len()
            || self.manifest_root_sha256 != self.expected_root()?
        {
            return Err(invalid("self_formed_r8b_executable_manifest_invalid"));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        let mut value = self.clone();
        value.manifest_root_sha256.clear();
        uncertainty_root_v1(&value)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BProducerRequestV2 {
    pub schema: String,
    pub route_id_sha256: String,
    pub producer_role: String,
    pub producer_executable_sha256: String,
    pub test_selector: String,
    pub allowed_relative_paths: Vec<String>,
    pub exclusive_output_directory: String,
    pub request_root_sha256: String,
}

impl K2UncertaintyR8BProducerRequestV2 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.schema = K2_UNCERTAINTY_R8B_PRODUCER_REQUEST_SCHEMA_V2.to_owned();
        self.allowed_relative_paths.sort();
        self.request_root_sha256 = self.expected_root()?;
        self.validate()
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.route_id_sha256)?;
        require_composition_root_v1(&self.producer_executable_sha256)?;
        let paths = self.allowed_relative_paths.iter().collect::<BTreeSet<_>>();
        if self.schema != K2_UNCERTAINTY_R8B_PRODUCER_REQUEST_SCHEMA_V2
            || self.producer_role.is_empty()
            || self.test_selector.is_empty()
            || self.allowed_relative_paths.is_empty()
            || paths.len() != self.allowed_relative_paths.len()
            || !self.allowed_relative_paths.iter().all(|path| super::super::valid_composition_path_v1(path))
            || !Path::new(&self.exclusive_output_directory).is_absolute()
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(invalid("self_formed_r8b_producer_request_invalid"));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        let mut value = self.clone();
        value.request_root_sha256.clear();
        uncertainty_root_v1(&value)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyR8BEvidenceKindV2 {
    ConfirmCanonicalBytes,
    DevelopmentKnownAnswers,
    ModeMatrix,
    ImmutablePublication,
    ProcessRestart,
    LinkedRoute,
    OracleCases,
    FrozenControlScopes,
    LegacyControls,
    V3Controls,
    V4Controls,
    CleanupTransaction,
    CleanupInterruption,
    DevelopmentResult,
    LinkedManifest,
    SuiteManifest,
    FreshControlCases,
    ProductionSurvival,
    AggregatePublicationFaults,
}

impl K2UncertaintyR8BEvidenceKindV2 {
    pub const ALL: [Self; 19] = [
        Self::ConfirmCanonicalBytes,
        Self::DevelopmentKnownAnswers,
        Self::ModeMatrix,
        Self::ImmutablePublication,
        Self::ProcessRestart,
        Self::LinkedRoute,
        Self::OracleCases,
        Self::FrozenControlScopes,
        Self::LegacyControls,
        Self::V3Controls,
        Self::V4Controls,
        Self::CleanupTransaction,
        Self::CleanupInterruption,
        Self::DevelopmentResult,
        Self::LinkedManifest,
        Self::SuiteManifest,
        Self::FreshControlCases,
        Self::ProductionSurvival,
        Self::AggregatePublicationFaults,
    ];

    pub const fn required(self) -> Option<u64> {
        match self {
            Self::ConfirmCanonicalBytes => None,
            Self::DevelopmentKnownAnswers => Some(3),
            Self::ModeMatrix => Some(20),
            Self::ImmutablePublication => Some(72),
            Self::ProcessRestart => Some(7),
            Self::LinkedRoute
            | Self::CleanupTransaction
            | Self::CleanupInterruption
            | Self::DevelopmentResult
            | Self::ProductionSurvival => Some(1),
            Self::OracleCases => Some(16),
            Self::FrozenControlScopes | Self::V3Controls => Some(4),
            Self::LegacyControls => Some(32),
            Self::V4Controls => Some(16),
            Self::LinkedManifest => Some(26),
            Self::SuiteManifest => Some(5),
            Self::FreshControlCases => Some(12),
            Self::AggregatePublicationFaults => Some(2),
        }
    }

    pub const fn expected_schema(self) -> &'static str {
        match self {
            Self::LinkedRoute | Self::ProductionSurvival => K2_UNCERTAINTY_R8B_ROUTE_RECEIPT_SCHEMA_V2,
            Self::LinkedManifest | Self::SuiteManifest => K2_UNCERTAINTY_R8B_EXECUTABLE_MANIFEST_SCHEMA_V2,
            Self::OracleCases => "nando.k2-self-formed-oracle-batch-receipt.v1",
            Self::FrozenControlScopes => K2_UNCERTAINTY_R8B_SUITE_RECEIPT_SCHEMA_V2,
            Self::LegacyControls | Self::V3Controls | Self::V4Controls | Self::FreshControlCases => {
                "nando.k2-self-formed-control-receipt.v1"
            }
            Self::CleanupTransaction => "nando.k2-self-formed-cleanup-receipt.v1",
            Self::DevelopmentResult => "nando.k2-self-formed-development-result-receipt.v1",
            _ => K2_UNCERTAINTY_R8B_SUITE_RECEIPT_SCHEMA_V2,
        }
    }

    pub const fn expected_root_field(self) -> &'static str {
        match self {
            Self::LinkedManifest | Self::SuiteManifest => "manifest_root_sha256",
            _ => "receipt_root_sha256",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BMeasuredReceiptV2 {
    pub schema: String,
    pub kind: K2UncertaintyR8BEvidenceKindV2,
    pub route_id_sha256: String,
    pub source_roots_sha256: Vec<String>,
    pub observed: u64,
    pub metrics: BTreeMap<String, u64>,
    pub false_accepts: u64,
    pub sealed_attempts: u64,
    pub production_mutations: u64,
    pub producer_executable_sha256: String,
    pub disposition: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyR8BMeasuredReceiptV2 {
    pub fn seal(
        kind: K2UncertaintyR8BEvidenceKindV2,
        route_id_sha256: String,
        source_roots_sha256: Vec<String>,
        observed: u64,
        metrics: BTreeMap<String, u64>,
        producer_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: String::new(),
            kind,
            route_id_sha256,
            source_roots_sha256,
            observed,
            metrics,
            false_accepts: 0,
            sealed_attempts: 0,
            production_mutations: 0,
            producer_executable_sha256,
            disposition: "PASS".to_owned(),
            authority: denied_authority_v1(),
            receipt_root_sha256: String::new(),
        };
        value.reseal()?;
        Ok(value)
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.schema = self.kind.expected_schema().to_owned();
        self.authority = denied_authority_v1();
        self.receipt_root_sha256 = self.expected_root()?;
        self.validate()
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in self.source_roots_sha256.iter().chain([&self.route_id_sha256, &self.producer_executable_sha256]) {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        let distinct_sources = self.source_roots_sha256.iter().collect::<BTreeSet<_>>().len();
        let custom_schema = matches!(
            self.schema.as_str(),
            K2_UNCERTAINTY_R8B_ROUTE_RECEIPT_SCHEMA_V2 | K2_UNCERTAINTY_R8B_SUITE_RECEIPT_SCHEMA_V2
        );
        if !custom_schema
            || self.schema != self.kind.expected_schema()
            || self.source_roots_sha256.is_empty()
            || distinct_sources != self.source_roots_sha256.len()
            || (self.kind == K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes && distinct_sources != 4)
            || self.observed == 0
            || self.kind.required().is_some_and(|required| self.observed != required)
            || self.false_accepts != 0
            || self.sealed_attempts != 0
            || self.production_mutations != 0
            || self.disposition != "PASS"
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(invalid("self_formed_r8b_measured_receipt_invalid"));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        let mut value = self.clone();
        value.receipt_root_sha256.clear();
        uncertainty_root_v1(&value)
    }
}

pub fn validate_self_formed_r8b_oracle_wrapper_v3(
    value: &K2UncertaintyR8BOracleWrapperV3,
) -> K2CompositionResultV1<()> {
    value.batch.validate()?;
    validate_wrapper_roots_v3(&value.completion_event_roots_sha256, &value.receipt_roots_sha256, 16)?;
    let expected =
        value.batch.case_receipts.iter().map(|row| row.receipt_root_sha256.as_str()).collect::<BTreeSet<_>>();
    let mut canonical = value.clone();
    canonical.receipt_root_sha256.clear();
    if value.schema != K2_UNCERTAINTY_R8B_ORACLE_WRAPPER_SCHEMA_V3
        || value.receipt_roots_sha256.iter().map(String::as_str).collect::<BTreeSet<_>>() != expected
        || value.receipt_root_sha256 != uncertainty_root_v1(&canonical)?
    {
        return Err(invalid("self_formed_r8b_oracle_wrapper_invalid"));
    }
    Ok(())
}

pub fn seal_self_formed_r8b_oracle_wrapper_v3(
    batch: K2UncertaintyOracleBaselineBatchReceiptV1,
    completion_event_roots_sha256: Vec<String>,
    receipt_roots_sha256: Vec<String>,
) -> K2CompositionResultV1<K2UncertaintyR8BOracleWrapperV3> {
    let mut value = K2UncertaintyR8BOracleWrapperV3 {
        schema: K2_UNCERTAINTY_R8B_ORACLE_WRAPPER_SCHEMA_V3.to_owned(), batch,
        completion_event_roots_sha256, receipt_roots_sha256, receipt_root_sha256: String::new(),
    };
    value.receipt_root_sha256 = uncertainty_root_v1(&value)?;
    validate_self_formed_r8b_oracle_wrapper_v3(&value)?;
    Ok(value)
}

pub fn validate_self_formed_r8b_control_wrapper_v3(
    value: &K2UncertaintyR8BControlWrapperV3,
) -> K2CompositionResultV1<()> {
    value.census.validate()?;
    validate_wrapper_roots_v3(&value.completion_event_roots_sha256, &value.receipt_roots_sha256, 4)?;
    let mut canonical = value.clone();
    canonical.receipt_root_sha256.clear();
    if value.schema != K2_UNCERTAINTY_R8B_CONTROL_WRAPPER_SCHEMA_V3
        || value.census.kind != K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes
        || value.census.source_roots_sha256 != value.receipt_roots_sha256
        || value.receipt_root_sha256 != uncertainty_root_v1(&canonical)?
    {
        return Err(invalid("self_formed_r8b_control_wrapper_invalid"));
    }
    Ok(())
}

pub fn seal_self_formed_r8b_control_wrapper_v3(
    census: K2UncertaintyR8BMeasuredReceiptV2,
    completion_event_roots_sha256: Vec<String>,
    receipt_roots_sha256: Vec<String>,
) -> K2CompositionResultV1<K2UncertaintyR8BControlWrapperV3> {
    let mut value = K2UncertaintyR8BControlWrapperV3 {
        schema: K2_UNCERTAINTY_R8B_CONTROL_WRAPPER_SCHEMA_V3.to_owned(), census,
        completion_event_roots_sha256, receipt_roots_sha256, receipt_root_sha256: String::new(),
    };
    value.receipt_root_sha256 = uncertainty_root_v1(&value)?;
    validate_self_formed_r8b_control_wrapper_v3(&value)?;
    Ok(value)
}

fn validate_wrapper_roots_v3(events: &[String], receipts: &[String], required: usize) -> K2CompositionResultV1<()> {
    for roots in [events, receipts] {
        roots.iter().try_for_each(|root| require_composition_root_v1(root))?;
        if roots.len() != required || roots.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(invalid("self_formed_r8b_root_vector_invalid"));
        }
    }
    if !events.iter().collect::<BTreeSet<_>>().is_disjoint(&receipts.iter().collect()) {
        return Err(invalid("self_formed_r8b_dual_root_domain_invalid"));
    }
    Ok(())
}

pub(super) type K2UncertaintyR8BEvidenceViewV3 = (String, String, u64, Option<String>, Option<Vec<String>>);

pub(super) fn decode_self_formed_r8b_evidence_view_v3(
    kind: K2UncertaintyR8BEvidenceKindV2,
    bytes: &[u8],
    route_id_sha256: &str,
) -> K2CompositionResultV1<K2UncertaintyR8BEvidenceViewV3> {
    let (schema, semantic_root_sha256, observed, producer_executable_sha256, source_roots_sha256) = match kind {
        K2UncertaintyR8BEvidenceKindV2::LegacyControls
        | K2UncertaintyR8BEvidenceKindV2::V3Controls
        | K2UncertaintyR8BEvidenceKindV2::V4Controls
        | K2UncertaintyR8BEvidenceKindV2::FreshControlCases => {
            let value: K2UncertaintyControlEvaluationReceiptV1 = decode_canonical_json_v1(bytes)?;
            value.validate()?;
            (value.schema, value.receipt_root_sha256, value.passed, Some(value.evaluator_executable_sha256), None)
        }
        K2UncertaintyR8BEvidenceKindV2::CleanupTransaction => {
            let value: K2UncertaintyCleanupReceiptV1 = decode_canonical_json_v1(bytes)?;
            value.validate()?;
            (value.schema, value.receipt_root_sha256, 1, None, None)
        }
        K2UncertaintyR8BEvidenceKindV2::DevelopmentResult => {
            let value: K2UncertaintyDevelopmentResultReceiptV1 = decode_canonical_json_v1(bytes)?;
            value.validate()?;
            (value.schema, value.receipt_root_sha256, 1, None, None)
        }
        K2UncertaintyR8BEvidenceKindV2::OracleCases
        | K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes
        | K2UncertaintyR8BEvidenceKindV2::LinkedManifest
        | K2UncertaintyR8BEvidenceKindV2::SuiteManifest => {
            return Err(invalid("self_formed_r8b_v3_special_evidence_redecoded"));
        }
        _ => {
            let value: K2UncertaintyR8BMeasuredReceiptV2 = decode_canonical_json_v1(bytes)?;
            value.validate()?;
            if value.kind != kind || value.route_id_sha256 != route_id_sha256 {
                return Err(invalid("self_formed_r8b_v3_measured_evidence_invalid"));
            }
            (
                value.schema,
                value.receipt_root_sha256,
                value.observed,
                Some(value.producer_executable_sha256),
                Some(value.source_roots_sha256),
            )
        }
    };
    Ok((schema, semantic_root_sha256, observed, producer_executable_sha256, source_roots_sha256))
}

fn invalid(reason: &'static str) -> K2CompositionErrorV1 {
    K2CompositionErrorV1::Invalid(reason)
}
