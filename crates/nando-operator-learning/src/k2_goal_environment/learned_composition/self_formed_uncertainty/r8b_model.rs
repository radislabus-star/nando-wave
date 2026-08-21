use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_R8B_EXECUTABLE_MANIFEST_SCHEMA_V2, K2_UNCERTAINTY_R8B_ROUTE_RECEIPT_SCHEMA_V2,
    K2_UNCERTAINTY_R8B_SUITE_RECEIPT_SCHEMA_V2, denied_authority_v1, require_denied_authority_v1,
    uncertainty_root_v1,
};

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
            Self::LinkedRoute | Self::ProductionSurvival => {
                K2_UNCERTAINTY_R8B_ROUTE_RECEIPT_SCHEMA_V2
            }
            Self::LinkedManifest | Self::SuiteManifest => {
                K2_UNCERTAINTY_R8B_EXECUTABLE_MANIFEST_SCHEMA_V2
            }
            Self::OracleCases => "nando.k2-self-formed-oracle-batch-receipt.v1",
            Self::FrozenControlScopes => K2_UNCERTAINTY_R8B_SUITE_RECEIPT_SCHEMA_V2,
            Self::LegacyControls
            | Self::V3Controls
            | Self::V4Controls
            | Self::FreshControlCases => "nando.k2-self-formed-control-receipt.v1",
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
        for root in self
            .source_roots_sha256
            .iter()
            .chain([&self.route_id_sha256, &self.producer_executable_sha256])
        {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        let distinct_sources = self
            .source_roots_sha256
            .iter()
            .collect::<BTreeSet<_>>()
            .len();
        let custom_schema = matches!(
            self.schema.as_str(),
            K2_UNCERTAINTY_R8B_ROUTE_RECEIPT_SCHEMA_V2 | K2_UNCERTAINTY_R8B_SUITE_RECEIPT_SCHEMA_V2
        );
        if !custom_schema
            || self.schema != self.kind.expected_schema()
            || self.source_roots_sha256.is_empty()
            || distinct_sources != self.source_roots_sha256.len()
            || (self.kind == K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes
                && distinct_sources != 4)
            || self.observed == 0
            || self
                .kind
                .required()
                .is_some_and(|required| self.observed != required)
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

fn invalid(reason: &'static str) -> K2CompositionErrorV1 {
    K2CompositionErrorV1::Invalid(reason)
}
