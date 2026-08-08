use std::fmt;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::manifest::{LawLabSandboxExecutorManifestV1, LawLabTreeManifestV1};
use crate::{
    LAW_LAB_MAX_HYPOTHESES_V1, LAW_LAB_MAX_OUTPUT_BYTES_V1, LawLabContractV1, LawLabPhaseV1,
    LawLabProbeDomainV1,
};

pub const LAW_LAB_SANDBOX_REQUEST_SCHEMA_V1: &str = "nando.law-lab-sandbox-request.v1";
pub const LAW_LAB_SANDBOX_WORKER_OUTCOME_SCHEMA_V1: &str =
    "nando.law-lab-sandbox-worker-outcome.v1";
pub const LAW_LAB_SANDBOX_RECEIPT_SCHEMA_V1: &str = "nando.law-lab-sandbox-receipt.v1";
pub const LAW_LAB_SANDBOX_CLEANUP_SCHEMA_V1: &str = "nando.law-lab-sandbox-cleanup.v1";
pub const LAW_LAB_SANDBOX_AUTHORITY_SCHEMA_V1: &str = "nando.law-lab-sandbox-authority-boundary.v1";
pub const LAW_LAB_SANDBOX_ISOLATION_SCHEMA_V1: &str = "nando.law-lab-sandbox-isolation.v1";
pub const LAW_LAB_SANDBOX_CAPABILITY_REPORT_SCHEMA_V1: &str =
    "nando.law-lab-sandbox-capability-report.v1";
pub const LAW_LAB_SANDBOX_WORKER_PROTOCOL_VERSION_V1: u64 = 1;
pub const LAW_LAB_SANDBOX_MAX_OPERATIONS_V1: usize = 64;
pub const LAW_LAB_SANDBOX_MAX_PATH_BYTES_V1: usize = 256;
pub const LAW_LAB_SANDBOX_MAX_VISIBLE_PIDS_V1: u64 = 2;

pub const LAW_LAB_SANDBOX_FORBIDDEN_PATHS_V1: [&str; 6] = [
    "/home",
    "/root",
    "/etc",
    "/run",
    "/var/lib/nando-wave",
    "/proc/1/root/home",
];

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LawLabSandboxPurposeV1 {
    ActiveDistinguishingProbe,
    GeneratedCapabilitySelfTest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum LawLabSandboxOperationV1 {
    CopySourceFile {
        source_path: String,
        work_path: String,
    },
    RemoveWorkPath {
        work_path: String,
    },
    CanonicalizeJsonFile {
        work_path: String,
    },
}

impl LawLabSandboxOperationV1 {
    pub(crate) fn validate(&self) -> Result<(), LawLabSandboxErrorV1> {
        match self {
            Self::CopySourceFile {
                source_path,
                work_path,
            } => {
                validate_relative_path_v1(source_path)?;
                validate_relative_path_v1(work_path)?;
                if source_path == work_path {
                    return Err(LawLabSandboxErrorV1::OperationConflict);
                }
            }
            Self::RemoveWorkPath { work_path } | Self::CanonicalizeJsonFile { work_path } => {
                validate_relative_path_v1(work_path)?;
            }
        }
        Ok(())
    }

    pub(crate) fn mutation_path(&self) -> &str {
        match self {
            Self::CopySourceFile { work_path, .. }
            | Self::RemoveWorkPath { work_path }
            | Self::CanonicalizeJsonFile { work_path } => work_path,
        }
    }

    pub(crate) fn is_valid_for_domain(&self, domain: LawLabProbeDomainV1) -> bool {
        matches!(
            (domain, self),
            (
                LawLabProbeDomainV1::Filesystem,
                Self::CopySourceFile { .. } | Self::RemoveWorkPath { .. }
            ) | (
                LawLabProbeDomainV1::StructuredData,
                Self::CanonicalizeJsonFile { .. }
            )
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LawLabSandboxRequestInputV1 {
    pub executor_manifest_root_sha256: String,
    pub worker_sha256: String,
    pub candidate_root_sha256: String,
    pub version_space_root_sha256: String,
    pub durable_prediction_ledger_root_sha256: String,
    pub probe_root_sha256: String,
    pub source_tree_root_sha256: String,
    pub deterministic_seed_sha256: String,
    pub domain: LawLabProbeDomainV1,
    pub purpose: LawLabSandboxPurposeV1,
    pub surviving_hypothesis_count: u64,
    pub precommitted_prediction_count: u64,
    pub operations: Vec<LawLabSandboxOperationV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabSandboxRequestV1 {
    pub schema: String,
    pub request_root_sha256: String,
    pub contract_root_sha256: String,
    pub executor_manifest_root_sha256: String,
    pub worker_sha256: String,
    pub candidate_root_sha256: String,
    pub version_space_root_sha256: String,
    pub durable_prediction_ledger_root_sha256: String,
    pub probe_root_sha256: String,
    pub source_tree_root_sha256: String,
    pub deterministic_seed_sha256: String,
    pub phase: LawLabPhaseV1,
    pub domain: LawLabProbeDomainV1,
    pub purpose: LawLabSandboxPurposeV1,
    pub surviving_hypothesis_count: u64,
    pub precommitted_prediction_count: u64,
    pub operations: Vec<LawLabSandboxOperationV1>,
}

#[derive(Serialize)]
struct LawLabSandboxRequestDigestV1<'a> {
    schema: &'static str,
    contract_root_sha256: &'a str,
    executor_manifest_root_sha256: &'a str,
    worker_sha256: &'a str,
    candidate_root_sha256: &'a str,
    version_space_root_sha256: &'a str,
    durable_prediction_ledger_root_sha256: &'a str,
    probe_root_sha256: &'a str,
    source_tree_root_sha256: &'a str,
    deterministic_seed_sha256: &'a str,
    phase: LawLabPhaseV1,
    domain: LawLabProbeDomainV1,
    purpose: LawLabSandboxPurposeV1,
    surviving_hypothesis_count: u64,
    precommitted_prediction_count: u64,
    operations: &'a [LawLabSandboxOperationV1],
}

impl LawLabSandboxRequestV1 {
    pub fn seal(input: LawLabSandboxRequestInputV1) -> Result<Self, LawLabSandboxErrorV1> {
        let contract = LawLabContractV1::preregistered_v1()
            .map_err(|_| LawLabSandboxErrorV1::ContractInvalid)?;
        let mut request = Self {
            schema: LAW_LAB_SANDBOX_REQUEST_SCHEMA_V1.to_owned(),
            request_root_sha256: String::new(),
            contract_root_sha256: contract.contract_root_sha256,
            executor_manifest_root_sha256: input.executor_manifest_root_sha256,
            worker_sha256: input.worker_sha256,
            candidate_root_sha256: input.candidate_root_sha256,
            version_space_root_sha256: input.version_space_root_sha256,
            durable_prediction_ledger_root_sha256: input.durable_prediction_ledger_root_sha256,
            probe_root_sha256: input.probe_root_sha256,
            source_tree_root_sha256: input.source_tree_root_sha256,
            deterministic_seed_sha256: input.deterministic_seed_sha256,
            phase: LawLabPhaseV1::PredictionsPrecommitted,
            domain: input.domain,
            purpose: input.purpose,
            surviving_hypothesis_count: input.surviving_hypothesis_count,
            precommitted_prediction_count: input.precommitted_prediction_count,
            operations: input.operations,
        };
        request.request_root_sha256 = request.expected_root()?;
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> Result<(), LawLabSandboxErrorV1> {
        let contract = LawLabContractV1::preregistered_v1()
            .map_err(|_| LawLabSandboxErrorV1::ContractInvalid)?;
        if self.schema != LAW_LAB_SANDBOX_REQUEST_SCHEMA_V1
            || self.contract_root_sha256 != contract.contract_root_sha256
            || self.phase != LawLabPhaseV1::PredictionsPrecommitted
            || self.operations.is_empty()
            || self.operations.len() > LAW_LAB_SANDBOX_MAX_OPERATIONS_V1
            || self.surviving_hypothesis_count == 0
            || self.surviving_hypothesis_count > LAW_LAB_MAX_HYPOTHESES_V1
            || self.precommitted_prediction_count != self.surviving_hypothesis_count
            || [
                self.request_root_sha256.as_str(),
                self.executor_manifest_root_sha256.as_str(),
                self.worker_sha256.as_str(),
                self.candidate_root_sha256.as_str(),
                self.version_space_root_sha256.as_str(),
                self.durable_prediction_ledger_root_sha256.as_str(),
                self.probe_root_sha256.as_str(),
                self.source_tree_root_sha256.as_str(),
                self.deterministic_seed_sha256.as_str(),
            ]
            .into_iter()
            .any(|root| !valid_nonzero_sha256(root))
        {
            return Err(LawLabSandboxErrorV1::InvalidRequest);
        }
        if !matches!(
            self.domain,
            LawLabProbeDomainV1::Filesystem | LawLabProbeDomainV1::StructuredData
        ) {
            return Err(LawLabSandboxErrorV1::UnsupportedDomain);
        }
        for operation in &self.operations {
            operation.validate()?;
            if !operation.is_valid_for_domain(self.domain) {
                return Err(LawLabSandboxErrorV1::DomainOperationMismatch);
            }
        }
        for (index, left) in self.operations.iter().enumerate() {
            for right in self.operations.iter().skip(index + 1) {
                if paths_overlap_v1(left.mutation_path(), right.mutation_path()) {
                    return Err(LawLabSandboxErrorV1::OperationConflict);
                }
            }
        }
        if self.request_root_sha256 != self.expected_root()? {
            return Err(LawLabSandboxErrorV1::RequestRootMismatch);
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, LawLabSandboxErrorV1> {
        self.validate()?;
        nando_operator_kernel::canonical_json_bytes(self)
            .map_err(|_| LawLabSandboxErrorV1::Serialization)
    }

    fn expected_root(&self) -> Result<String, LawLabSandboxErrorV1> {
        canonical_json_sha256(&LawLabSandboxRequestDigestV1 {
            schema: LAW_LAB_SANDBOX_REQUEST_SCHEMA_V1,
            contract_root_sha256: &self.contract_root_sha256,
            executor_manifest_root_sha256: &self.executor_manifest_root_sha256,
            worker_sha256: &self.worker_sha256,
            candidate_root_sha256: &self.candidate_root_sha256,
            version_space_root_sha256: &self.version_space_root_sha256,
            durable_prediction_ledger_root_sha256: &self.durable_prediction_ledger_root_sha256,
            probe_root_sha256: &self.probe_root_sha256,
            source_tree_root_sha256: &self.source_tree_root_sha256,
            deterministic_seed_sha256: &self.deterministic_seed_sha256,
            phase: self.phase,
            domain: self.domain,
            purpose: self.purpose,
            surviving_hypothesis_count: self.surviving_hypothesis_count,
            precommitted_prediction_count: self.precommitted_prediction_count,
            operations: &self.operations,
        })
        .map_err(|_| LawLabSandboxErrorV1::Serialization)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabSandboxEnvironmentEntryV1 {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabSandboxIsolationAttestationV1 {
    pub schema: String,
    pub attestation_root_sha256: String,
    pub ipv4_non_loopback_route_entries: u64,
    pub ipv6_non_loopback_route_entries: u64,
    pub visible_pid_count: u64,
    pub no_new_privileges: bool,
    pub source_write_blocked: bool,
    pub forbidden_paths_absent: bool,
    pub forbidden_paths_checked: Vec<String>,
    pub environment: Vec<LawLabSandboxEnvironmentEntryV1>,
}

#[derive(Serialize)]
struct LawLabSandboxIsolationDigestV1<'a> {
    schema: &'static str,
    ipv4_non_loopback_route_entries: u64,
    ipv6_non_loopback_route_entries: u64,
    visible_pid_count: u64,
    no_new_privileges: bool,
    source_write_blocked: bool,
    forbidden_paths_absent: bool,
    forbidden_paths_checked: &'a [String],
    environment: &'a [LawLabSandboxEnvironmentEntryV1],
}

impl LawLabSandboxIsolationAttestationV1 {
    pub(crate) fn seal(
        ipv4_non_loopback_route_entries: u64,
        ipv6_non_loopback_route_entries: u64,
        visible_pid_count: u64,
        no_new_privileges: bool,
        source_write_blocked: bool,
        forbidden_paths_absent: bool,
        environment: Vec<LawLabSandboxEnvironmentEntryV1>,
    ) -> Result<Self, LawLabSandboxErrorV1> {
        let mut attestation = Self {
            schema: LAW_LAB_SANDBOX_ISOLATION_SCHEMA_V1.to_owned(),
            attestation_root_sha256: String::new(),
            ipv4_non_loopback_route_entries,
            ipv6_non_loopback_route_entries,
            visible_pid_count,
            no_new_privileges,
            source_write_blocked,
            forbidden_paths_absent,
            forbidden_paths_checked: LAW_LAB_SANDBOX_FORBIDDEN_PATHS_V1
                .into_iter()
                .map(str::to_owned)
                .collect(),
            environment,
        };
        attestation.attestation_root_sha256 = attestation.expected_root()?;
        attestation.validate()?;
        Ok(attestation)
    }

    pub fn validate(&self) -> Result<(), LawLabSandboxErrorV1> {
        if self.schema != LAW_LAB_SANDBOX_ISOLATION_SCHEMA_V1
            || self.ipv4_non_loopback_route_entries != 0
            || self.ipv6_non_loopback_route_entries != 0
            || self.visible_pid_count == 0
            || self.visible_pid_count > LAW_LAB_SANDBOX_MAX_VISIBLE_PIDS_V1
            || !self.no_new_privileges
            || !self.source_write_blocked
            || !self.forbidden_paths_absent
            || self.forbidden_paths_checked
                != LAW_LAB_SANDBOX_FORBIDDEN_PATHS_V1
                    .into_iter()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            || self.environment != deterministic_environment_v1()
            || self.attestation_root_sha256 != self.expected_root()?
        {
            return Err(LawLabSandboxErrorV1::IsolationVerificationFailed);
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, LawLabSandboxErrorV1> {
        canonical_json_sha256(&LawLabSandboxIsolationDigestV1 {
            schema: LAW_LAB_SANDBOX_ISOLATION_SCHEMA_V1,
            ipv4_non_loopback_route_entries: self.ipv4_non_loopback_route_entries,
            ipv6_non_loopback_route_entries: self.ipv6_non_loopback_route_entries,
            visible_pid_count: self.visible_pid_count,
            no_new_privileges: self.no_new_privileges,
            source_write_blocked: self.source_write_blocked,
            forbidden_paths_absent: self.forbidden_paths_absent,
            forbidden_paths_checked: &self.forbidden_paths_checked,
            environment: &self.environment,
        })
        .map_err(|_| LawLabSandboxErrorV1::Serialization)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabSandboxOperationResultV1 {
    pub ordinal: u64,
    pub operation_root_sha256: String,
    pub effect_root_sha256: String,
    pub bytes_written: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabSandboxWorkerOutcomeV1 {
    pub schema: String,
    pub outcome_root_sha256: String,
    pub exact_outcome_root_sha256: String,
    pub request_root_sha256: String,
    pub worker_protocol_version: u64,
    pub worker_sha256: String,
    pub source_manifest: LawLabTreeManifestV1,
    pub pre_work_manifest: LawLabTreeManifestV1,
    pub post_work_manifest: LawLabTreeManifestV1,
    pub operation_results: Vec<LawLabSandboxOperationResultV1>,
    pub output_bytes_written: u64,
    pub isolation: LawLabSandboxIsolationAttestationV1,
}

#[derive(Serialize)]
struct LawLabSandboxWorkerOutcomeDigestV1<'a> {
    schema: &'static str,
    exact_outcome_root_sha256: &'a str,
    request_root_sha256: &'a str,
    worker_protocol_version: u64,
    worker_sha256: &'a str,
    source_manifest: &'a LawLabTreeManifestV1,
    pre_work_manifest: &'a LawLabTreeManifestV1,
    post_work_manifest: &'a LawLabTreeManifestV1,
    operation_results: &'a [LawLabSandboxOperationResultV1],
    output_bytes_written: u64,
    isolation: &'a LawLabSandboxIsolationAttestationV1,
}

pub(crate) struct LawLabSandboxWorkerOutcomeInputV1<'a> {
    pub request: &'a LawLabSandboxRequestV1,
    pub worker_sha256: String,
    pub source_manifest: LawLabTreeManifestV1,
    pub pre_work_manifest: LawLabTreeManifestV1,
    pub post_work_manifest: LawLabTreeManifestV1,
    pub operation_results: Vec<LawLabSandboxOperationResultV1>,
    pub output_bytes_written: u64,
    pub isolation: LawLabSandboxIsolationAttestationV1,
}

impl LawLabSandboxWorkerOutcomeV1 {
    pub(crate) fn seal(
        input: LawLabSandboxWorkerOutcomeInputV1<'_>,
    ) -> Result<Self, LawLabSandboxErrorV1> {
        let exact_outcome_root_sha256 = canonical_json_sha256(&(
            LAW_LAB_SANDBOX_WORKER_OUTCOME_SCHEMA_V1,
            "exact-outcome",
            input.request.request_root_sha256.as_str(),
            input.post_work_manifest.tree_root_sha256.as_str(),
            &input.operation_results,
        ))
        .map_err(|_| LawLabSandboxErrorV1::Serialization)?;
        let mut outcome = Self {
            schema: LAW_LAB_SANDBOX_WORKER_OUTCOME_SCHEMA_V1.to_owned(),
            outcome_root_sha256: String::new(),
            exact_outcome_root_sha256,
            request_root_sha256: input.request.request_root_sha256.clone(),
            worker_protocol_version: LAW_LAB_SANDBOX_WORKER_PROTOCOL_VERSION_V1,
            worker_sha256: input.worker_sha256,
            source_manifest: input.source_manifest,
            pre_work_manifest: input.pre_work_manifest,
            post_work_manifest: input.post_work_manifest,
            operation_results: input.operation_results,
            output_bytes_written: input.output_bytes_written,
            isolation: input.isolation,
        };
        outcome.outcome_root_sha256 = outcome.expected_root()?;
        outcome.validate(input.request)?;
        Ok(outcome)
    }

    pub fn validate(&self, request: &LawLabSandboxRequestV1) -> Result<(), LawLabSandboxErrorV1> {
        request.validate()?;
        self.source_manifest.validate()?;
        self.pre_work_manifest.validate()?;
        self.post_work_manifest.validate()?;
        self.isolation.validate()?;
        let summed_output_bytes = self
            .operation_results
            .iter()
            .try_fold(0_u64, |sum, result| sum.checked_add(result.bytes_written))
            .ok_or(LawLabSandboxErrorV1::WorkerOutcomeInvalid)?;
        if self.schema != LAW_LAB_SANDBOX_WORKER_OUTCOME_SCHEMA_V1
            || self.worker_protocol_version != LAW_LAB_SANDBOX_WORKER_PROTOCOL_VERSION_V1
            || self.request_root_sha256 != request.request_root_sha256
            || self.worker_sha256 != request.worker_sha256
            || self.source_manifest.tree_root_sha256 != request.source_tree_root_sha256
            || self.pre_work_manifest.tree_root_sha256 != request.source_tree_root_sha256
            || self.operation_results.len() != request.operations.len()
            || self.output_bytes_written > LAW_LAB_MAX_OUTPUT_BYTES_V1
            || self
                .operation_results
                .iter()
                .enumerate()
                .any(|(index, result)| {
                    result.ordinal != index as u64
                        || !valid_nonzero_sha256(&result.operation_root_sha256)
                        || !valid_nonzero_sha256(&result.effect_root_sha256)
                })
            || self.output_bytes_written != summed_output_bytes
            || self.exact_outcome_root_sha256
                != canonical_json_sha256(&(
                    LAW_LAB_SANDBOX_WORKER_OUTCOME_SCHEMA_V1,
                    "exact-outcome",
                    request.request_root_sha256.as_str(),
                    self.post_work_manifest.tree_root_sha256.as_str(),
                    &self.operation_results,
                ))
                .map_err(|_| LawLabSandboxErrorV1::Serialization)?
            || self.outcome_root_sha256 != self.expected_root()?
        {
            return Err(LawLabSandboxErrorV1::WorkerOutcomeInvalid);
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, LawLabSandboxErrorV1> {
        canonical_json_sha256(&LawLabSandboxWorkerOutcomeDigestV1 {
            schema: LAW_LAB_SANDBOX_WORKER_OUTCOME_SCHEMA_V1,
            exact_outcome_root_sha256: &self.exact_outcome_root_sha256,
            request_root_sha256: &self.request_root_sha256,
            worker_protocol_version: self.worker_protocol_version,
            worker_sha256: &self.worker_sha256,
            source_manifest: &self.source_manifest,
            pre_work_manifest: &self.pre_work_manifest,
            post_work_manifest: &self.post_work_manifest,
            operation_results: &self.operation_results,
            output_bytes_written: self.output_bytes_written,
            isolation: &self.isolation,
        })
        .map_err(|_| LawLabSandboxErrorV1::Serialization)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabSandboxCleanupProofV1 {
    pub schema: String,
    pub cleanup_root_sha256: String,
    pub workspace_instance_sha256: String,
    pub removed: bool,
    pub verified_absent: bool,
}

impl LawLabSandboxCleanupProofV1 {
    pub(crate) fn seal(workspace_instance_sha256: String) -> Result<Self, LawLabSandboxErrorV1> {
        let mut proof = Self {
            schema: LAW_LAB_SANDBOX_CLEANUP_SCHEMA_V1.to_owned(),
            cleanup_root_sha256: String::new(),
            workspace_instance_sha256,
            removed: true,
            verified_absent: true,
        };
        proof.cleanup_root_sha256 = proof.expected_root()?;
        proof.validate()?;
        Ok(proof)
    }

    pub fn validate(&self) -> Result<(), LawLabSandboxErrorV1> {
        if self.schema != LAW_LAB_SANDBOX_CLEANUP_SCHEMA_V1
            || !valid_nonzero_sha256(&self.workspace_instance_sha256)
            || !self.removed
            || !self.verified_absent
            || self.cleanup_root_sha256 != self.expected_root()?
        {
            return Err(LawLabSandboxErrorV1::CleanupVerificationFailed);
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, LawLabSandboxErrorV1> {
        canonical_json_sha256(&(
            LAW_LAB_SANDBOX_CLEANUP_SCHEMA_V1,
            self.workspace_instance_sha256.as_str(),
            self.removed,
            self.verified_absent,
        ))
        .map_err(|_| LawLabSandboxErrorV1::Serialization)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabSandboxAuthorityBoundaryV1 {
    pub schema: String,
    pub prediction_commitments_written: bool,
    pub natural_holdout_satisfied: bool,
    pub law_certificate_issued: bool,
    pub execution_authority_granted: bool,
    pub package_activated: bool,
    pub k1_registry_mutated: bool,
    pub phase_memory_mutated: bool,
    pub economics_credit_granted: bool,
}

impl LawLabSandboxAuthorityBoundaryV1 {
    #[must_use]
    pub fn authority_free_v1() -> Self {
        Self {
            schema: LAW_LAB_SANDBOX_AUTHORITY_SCHEMA_V1.to_owned(),
            prediction_commitments_written: false,
            natural_holdout_satisfied: false,
            law_certificate_issued: false,
            execution_authority_granted: false,
            package_activated: false,
            k1_registry_mutated: false,
            phase_memory_mutated: false,
            economics_credit_granted: false,
        }
    }

    pub fn validate(&self) -> Result<(), LawLabSandboxErrorV1> {
        if self != &Self::authority_free_v1() {
            return Err(LawLabSandboxErrorV1::AuthorityBoundaryViolated);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabSandboxReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub contract_root_sha256: String,
    pub executor_manifest_root_sha256: String,
    pub request_root_sha256: String,
    pub candidate_root_sha256: String,
    pub version_space_root_sha256: String,
    pub durable_prediction_ledger_root_sha256: String,
    pub probe_root_sha256: String,
    pub purpose: LawLabSandboxPurposeV1,
    pub domain: LawLabProbeDomainV1,
    pub worker_outcome_root_sha256: String,
    pub exact_outcome_root_sha256: String,
    pub source_tree_root_sha256: String,
    pub post_tree_root_sha256: String,
    pub isolation_attestation_root_sha256: String,
    pub cleanup: LawLabSandboxCleanupProofV1,
    pub authority: LawLabSandboxAuthorityBoundaryV1,
}

#[derive(Serialize)]
struct LawLabSandboxReceiptDigestV1<'a> {
    schema: &'static str,
    contract_root_sha256: &'a str,
    executor_manifest_root_sha256: &'a str,
    request_root_sha256: &'a str,
    candidate_root_sha256: &'a str,
    version_space_root_sha256: &'a str,
    durable_prediction_ledger_root_sha256: &'a str,
    probe_root_sha256: &'a str,
    purpose: LawLabSandboxPurposeV1,
    domain: LawLabProbeDomainV1,
    worker_outcome_root_sha256: &'a str,
    exact_outcome_root_sha256: &'a str,
    source_tree_root_sha256: &'a str,
    post_tree_root_sha256: &'a str,
    isolation_attestation_root_sha256: &'a str,
    cleanup: &'a LawLabSandboxCleanupProofV1,
    authority: &'a LawLabSandboxAuthorityBoundaryV1,
}

impl LawLabSandboxReceiptV1 {
    pub(crate) fn seal(
        request: &LawLabSandboxRequestV1,
        outcome: &LawLabSandboxWorkerOutcomeV1,
        cleanup: LawLabSandboxCleanupProofV1,
    ) -> Result<Self, LawLabSandboxErrorV1> {
        let mut receipt = Self {
            schema: LAW_LAB_SANDBOX_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            contract_root_sha256: request.contract_root_sha256.clone(),
            executor_manifest_root_sha256: request.executor_manifest_root_sha256.clone(),
            request_root_sha256: request.request_root_sha256.clone(),
            candidate_root_sha256: request.candidate_root_sha256.clone(),
            version_space_root_sha256: request.version_space_root_sha256.clone(),
            durable_prediction_ledger_root_sha256: request
                .durable_prediction_ledger_root_sha256
                .clone(),
            probe_root_sha256: request.probe_root_sha256.clone(),
            purpose: request.purpose,
            domain: request.domain,
            worker_outcome_root_sha256: outcome.outcome_root_sha256.clone(),
            exact_outcome_root_sha256: outcome.exact_outcome_root_sha256.clone(),
            source_tree_root_sha256: outcome.source_manifest.tree_root_sha256.clone(),
            post_tree_root_sha256: outcome.post_work_manifest.tree_root_sha256.clone(),
            isolation_attestation_root_sha256: outcome.isolation.attestation_root_sha256.clone(),
            cleanup,
            authority: LawLabSandboxAuthorityBoundaryV1::authority_free_v1(),
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate(request, outcome)?;
        Ok(receipt)
    }

    pub fn validate(
        &self,
        request: &LawLabSandboxRequestV1,
        outcome: &LawLabSandboxWorkerOutcomeV1,
    ) -> Result<(), LawLabSandboxErrorV1> {
        request.validate()?;
        outcome.validate(request)?;
        self.cleanup.validate()?;
        self.authority.validate()?;
        if self.schema != LAW_LAB_SANDBOX_RECEIPT_SCHEMA_V1
            || self.contract_root_sha256 != request.contract_root_sha256
            || self.executor_manifest_root_sha256 != request.executor_manifest_root_sha256
            || self.request_root_sha256 != request.request_root_sha256
            || self.candidate_root_sha256 != request.candidate_root_sha256
            || self.version_space_root_sha256 != request.version_space_root_sha256
            || self.durable_prediction_ledger_root_sha256
                != request.durable_prediction_ledger_root_sha256
            || self.probe_root_sha256 != request.probe_root_sha256
            || self.purpose != request.purpose
            || self.domain != request.domain
            || self.worker_outcome_root_sha256 != outcome.outcome_root_sha256
            || self.exact_outcome_root_sha256 != outcome.exact_outcome_root_sha256
            || self.source_tree_root_sha256 != outcome.source_manifest.tree_root_sha256
            || self.post_tree_root_sha256 != outcome.post_work_manifest.tree_root_sha256
            || self.isolation_attestation_root_sha256 != outcome.isolation.attestation_root_sha256
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(LawLabSandboxErrorV1::ReceiptInvalid);
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, LawLabSandboxErrorV1> {
        canonical_json_sha256(&LawLabSandboxReceiptDigestV1 {
            schema: LAW_LAB_SANDBOX_RECEIPT_SCHEMA_V1,
            contract_root_sha256: &self.contract_root_sha256,
            executor_manifest_root_sha256: &self.executor_manifest_root_sha256,
            request_root_sha256: &self.request_root_sha256,
            candidate_root_sha256: &self.candidate_root_sha256,
            version_space_root_sha256: &self.version_space_root_sha256,
            durable_prediction_ledger_root_sha256: &self.durable_prediction_ledger_root_sha256,
            probe_root_sha256: &self.probe_root_sha256,
            purpose: self.purpose,
            domain: self.domain,
            worker_outcome_root_sha256: &self.worker_outcome_root_sha256,
            exact_outcome_root_sha256: &self.exact_outcome_root_sha256,
            source_tree_root_sha256: &self.source_tree_root_sha256,
            post_tree_root_sha256: &self.post_tree_root_sha256,
            isolation_attestation_root_sha256: &self.isolation_attestation_root_sha256,
            cleanup: &self.cleanup,
            authority: &self.authority,
        })
        .map_err(|_| LawLabSandboxErrorV1::Serialization)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabSandboxExecutionV1 {
    pub receipt: LawLabSandboxReceiptV1,
    pub worker_outcome: LawLabSandboxWorkerOutcomeV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabSandboxCapabilityCaseV1 {
    pub request: LawLabSandboxRequestV1,
    pub execution: LawLabSandboxExecutionV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabSandboxCapabilityReportV1 {
    pub schema: String,
    pub report_root_sha256: String,
    pub contract_root_sha256: String,
    pub executor_manifest: LawLabSandboxExecutorManifestV1,
    pub filesystem_case: LawLabSandboxCapabilityCaseV1,
    pub structured_data_case: LawLabSandboxCapabilityCaseV1,
    pub verified_backends: Vec<LawLabProbeDomainV1>,
    pub unimplemented_backends: Vec<LawLabProbeDomainV1>,
    pub generated_fixtures_only: bool,
    pub generated_fixtures_may_seed_candidate: bool,
    pub natural_holdout_satisfied: bool,
    pub source_fixtures_removed: bool,
    pub fixture_cleanup_root_sha256: String,
    pub authority: LawLabSandboxAuthorityBoundaryV1,
}

#[derive(Serialize)]
struct LawLabSandboxCapabilityReportDigestV1<'a> {
    schema: &'static str,
    contract_root_sha256: &'a str,
    executor_manifest: &'a LawLabSandboxExecutorManifestV1,
    filesystem_case: &'a LawLabSandboxCapabilityCaseV1,
    structured_data_case: &'a LawLabSandboxCapabilityCaseV1,
    verified_backends: &'a [LawLabProbeDomainV1],
    unimplemented_backends: &'a [LawLabProbeDomainV1],
    generated_fixtures_only: bool,
    generated_fixtures_may_seed_candidate: bool,
    natural_holdout_satisfied: bool,
    source_fixtures_removed: bool,
    fixture_cleanup_root_sha256: &'a str,
    authority: &'a LawLabSandboxAuthorityBoundaryV1,
}

impl LawLabSandboxCapabilityReportV1 {
    pub fn seal(
        executor_manifest: LawLabSandboxExecutorManifestV1,
        filesystem_case: LawLabSandboxCapabilityCaseV1,
        structured_data_case: LawLabSandboxCapabilityCaseV1,
        fixture_cleanup_root_sha256: String,
    ) -> Result<Self, LawLabSandboxErrorV1> {
        let contract = LawLabContractV1::preregistered_v1()
            .map_err(|_| LawLabSandboxErrorV1::ContractInvalid)?;
        let mut report = Self {
            schema: LAW_LAB_SANDBOX_CAPABILITY_REPORT_SCHEMA_V1.to_owned(),
            report_root_sha256: String::new(),
            contract_root_sha256: contract.contract_root_sha256,
            executor_manifest,
            filesystem_case,
            structured_data_case,
            verified_backends: vec![
                LawLabProbeDomainV1::Filesystem,
                LawLabProbeDomainV1::StructuredData,
            ],
            unimplemented_backends: vec![
                LawLabProbeDomainV1::Git,
                LawLabProbeDomainV1::Sqlite,
                LawLabProbeDomainV1::StructuredCli,
            ],
            generated_fixtures_only: true,
            generated_fixtures_may_seed_candidate: false,
            natural_holdout_satisfied: false,
            source_fixtures_removed: true,
            fixture_cleanup_root_sha256,
            authority: LawLabSandboxAuthorityBoundaryV1::authority_free_v1(),
        };
        report.report_root_sha256 = report.expected_root()?;
        report.validate()?;
        Ok(report)
    }

    pub fn validate(&self) -> Result<(), LawLabSandboxErrorV1> {
        let contract = LawLabContractV1::preregistered_v1()
            .map_err(|_| LawLabSandboxErrorV1::ContractInvalid)?;
        self.executor_manifest.validate()?;
        self.filesystem_case.execution.receipt.validate(
            &self.filesystem_case.request,
            &self.filesystem_case.execution.worker_outcome,
        )?;
        self.structured_data_case.execution.receipt.validate(
            &self.structured_data_case.request,
            &self.structured_data_case.execution.worker_outcome,
        )?;
        self.authority.validate()?;
        if self.schema != LAW_LAB_SANDBOX_CAPABILITY_REPORT_SCHEMA_V1
            || self.contract_root_sha256 != contract.contract_root_sha256
            || self.executor_manifest.manifest_root_sha256
                != self.filesystem_case.request.executor_manifest_root_sha256
            || self.executor_manifest.manifest_root_sha256
                != self
                    .structured_data_case
                    .request
                    .executor_manifest_root_sha256
            || self.filesystem_case.request.domain != LawLabProbeDomainV1::Filesystem
            || self.structured_data_case.request.domain != LawLabProbeDomainV1::StructuredData
            || self.filesystem_case.request.purpose
                != LawLabSandboxPurposeV1::GeneratedCapabilitySelfTest
            || self.structured_data_case.request.purpose
                != LawLabSandboxPurposeV1::GeneratedCapabilitySelfTest
            || self.verified_backends
                != [
                    LawLabProbeDomainV1::Filesystem,
                    LawLabProbeDomainV1::StructuredData,
                ]
            || self.unimplemented_backends
                != [
                    LawLabProbeDomainV1::Git,
                    LawLabProbeDomainV1::Sqlite,
                    LawLabProbeDomainV1::StructuredCli,
                ]
            || !self.generated_fixtures_only
            || self.generated_fixtures_may_seed_candidate
            || self.natural_holdout_satisfied
            || !self.source_fixtures_removed
            || !valid_nonzero_sha256(&self.fixture_cleanup_root_sha256)
            || self.report_root_sha256 != self.expected_root()?
        {
            return Err(LawLabSandboxErrorV1::ReceiptInvalid);
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, LawLabSandboxErrorV1> {
        self.validate()?;
        nando_operator_kernel::canonical_json_bytes(self)
            .map_err(|_| LawLabSandboxErrorV1::Serialization)
    }

    fn expected_root(&self) -> Result<String, LawLabSandboxErrorV1> {
        canonical_json_sha256(&LawLabSandboxCapabilityReportDigestV1 {
            schema: LAW_LAB_SANDBOX_CAPABILITY_REPORT_SCHEMA_V1,
            contract_root_sha256: &self.contract_root_sha256,
            executor_manifest: &self.executor_manifest,
            filesystem_case: &self.filesystem_case,
            structured_data_case: &self.structured_data_case,
            verified_backends: &self.verified_backends,
            unimplemented_backends: &self.unimplemented_backends,
            generated_fixtures_only: self.generated_fixtures_only,
            generated_fixtures_may_seed_candidate: self.generated_fixtures_may_seed_candidate,
            natural_holdout_satisfied: self.natural_holdout_satisfied,
            source_fixtures_removed: self.source_fixtures_removed,
            fixture_cleanup_root_sha256: &self.fixture_cleanup_root_sha256,
            authority: &self.authority,
        })
        .map_err(|_| LawLabSandboxErrorV1::Serialization)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LawLabSandboxErrorV1 {
    ContractInvalid,
    InvalidRequest,
    RequestRootMismatch,
    UnsupportedDomain,
    DomainOperationMismatch,
    UnsafePath,
    OperationConflict,
    Serialization,
    Io,
    InvalidTree,
    TreeBudgetExceeded,
    SourceSnapshotMissing,
    SourceManifestMismatch,
    ExecutorManifestInvalid,
    ExecutorManifestMismatch,
    ToolUntrusted,
    WorkerHashMismatch,
    WorkerProtocolFailed,
    WorkerOutputTooLarge,
    WorkerOutcomeInvalid,
    IsolationVerificationFailed,
    IndependentVerificationFailed,
    ProcessFailed,
    TimedOut,
    CleanupVerificationFailed,
    AuthorityBoundaryViolated,
    ReceiptInvalid,
}

impl fmt::Display for LawLabSandboxErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ContractInvalid => "law_lab_sandbox_contract_invalid",
            Self::InvalidRequest => "law_lab_sandbox_request_invalid",
            Self::RequestRootMismatch => "law_lab_sandbox_request_root_mismatch",
            Self::UnsupportedDomain => "law_lab_sandbox_domain_unsupported",
            Self::DomainOperationMismatch => "law_lab_sandbox_domain_operation_mismatch",
            Self::UnsafePath => "law_lab_sandbox_path_unsafe",
            Self::OperationConflict => "law_lab_sandbox_operation_conflict",
            Self::Serialization => "law_lab_sandbox_serialization_failed",
            Self::Io => "law_lab_sandbox_io_failed",
            Self::InvalidTree => "law_lab_sandbox_tree_invalid",
            Self::TreeBudgetExceeded => "law_lab_sandbox_tree_budget_exceeded",
            Self::SourceSnapshotMissing => "law_lab_sandbox_source_snapshot_missing",
            Self::SourceManifestMismatch => "law_lab_sandbox_source_manifest_mismatch",
            Self::ExecutorManifestInvalid => "law_lab_sandbox_executor_manifest_invalid",
            Self::ExecutorManifestMismatch => "law_lab_sandbox_executor_manifest_mismatch",
            Self::ToolUntrusted => "law_lab_sandbox_tool_untrusted",
            Self::WorkerHashMismatch => "law_lab_sandbox_worker_hash_mismatch",
            Self::WorkerProtocolFailed => "law_lab_sandbox_worker_protocol_failed",
            Self::WorkerOutputTooLarge => "law_lab_sandbox_worker_output_too_large",
            Self::WorkerOutcomeInvalid => "law_lab_sandbox_worker_outcome_invalid",
            Self::IsolationVerificationFailed => "law_lab_sandbox_isolation_verification_failed",
            Self::IndependentVerificationFailed => {
                "law_lab_sandbox_independent_verification_failed"
            }
            Self::ProcessFailed => "law_lab_sandbox_process_failed",
            Self::TimedOut => "law_lab_sandbox_timed_out",
            Self::CleanupVerificationFailed => "law_lab_sandbox_cleanup_verification_failed",
            Self::AuthorityBoundaryViolated => "law_lab_sandbox_authority_boundary_violated",
            Self::ReceiptInvalid => "law_lab_sandbox_receipt_invalid",
        })
    }
}

impl std::error::Error for LawLabSandboxErrorV1 {}

pub(crate) fn validate_relative_path_v1(path: &str) -> Result<(), LawLabSandboxErrorV1> {
    if path.is_empty()
        || path.len() > LAW_LAB_SANDBOX_MAX_PATH_BYTES_V1
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains('\0')
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(LawLabSandboxErrorV1::UnsafePath);
    }
    Ok(())
}

pub(crate) fn paths_overlap_v1(left: &str, right: &str) -> bool {
    left == right
        || left
            .strip_prefix(right)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || right
            .strip_prefix(left)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

pub(crate) fn deterministic_environment_v1() -> Vec<LawLabSandboxEnvironmentEntryV1> {
    [
        ("LANG", "C"),
        ("LC_ALL", "C"),
        ("PATH", "/usr/bin"),
        ("PWD", "/work"),
        ("TZ", "UTC"),
    ]
    .into_iter()
    .map(|(name, value)| LawLabSandboxEnvironmentEntryV1 {
        name: name.to_owned(),
        value: value.to_owned(),
    })
    .collect()
}
