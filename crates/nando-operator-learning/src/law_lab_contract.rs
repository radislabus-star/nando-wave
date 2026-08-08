//! Frozen authority-free contract for bounded active law identification.
//!
//! Real ordinary traffic owns candidate provenance and the later natural
//! holdout. Lab probes may eliminate hypotheses, but they can never grant
//! execution authority, K1 membership, economics credit, or phase mutation.

use nando_operator_kernel::{canonical_json_bytes, canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

pub const LAW_LAB_CONTRACT_SCHEMA_V1: &str = "nando.law-lab-contract.v1";
pub const LAW_LAB_CONTRACT_VERSION_V1: u64 = 1;

pub const LAW_LAB_MAX_ACTIVE_GENERATIONS_V1: u64 = 1;
pub const LAW_LAB_MAX_CANDIDATES_V1: u64 = 1;
pub const LAW_LAB_MAX_NATURAL_SUPPORT_ROWS_V1: u64 = 64;
pub const LAW_LAB_MAX_HYPOTHESES_V1: u64 = 32;
pub const LAW_LAB_MAX_PROBES_V1: u64 = 8;
pub const LAW_LAB_MAX_GENERATION_WALL_MS_V1: u64 = 900_000;
pub const LAW_LAB_MAX_PROBE_WALL_MS_V1: u64 = 5_000;
pub const LAW_LAB_MAX_PROBE_CPU_MS_V1: u64 = 3_000;
pub const LAW_LAB_MAX_MEMORY_BYTES_V1: u64 = 512 * 1024 * 1024;
pub const LAW_LAB_MAX_DISK_BYTES_V1: u64 = 256 * 1024 * 1024;
pub const LAW_LAB_MAX_INPUT_BYTES_V1: u64 = 8 * 1024 * 1024;
pub const LAW_LAB_MAX_OUTPUT_BYTES_V1: u64 = 2 * 1024 * 1024;
pub const LAW_LAB_MAX_PROCESSES_V1: u64 = 16;
pub const LAW_LAB_MAX_MODEL_CALLS_V1: u64 = 0;
pub const LAW_LAB_MAX_MODEL_TOKENS_V1: u64 = 0;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LawLabCandidateSourceV1 {
    OrdinaryTrafficResidual,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LawLabProbeEvidenceClassV1 {
    IsolatedExperiment,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LawLabPromotionEvidenceClassV1 {
    PostCandidateNaturalHoldout,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LawLabProbeSelectionRuleV1 {
    MaximumDistinguishingPartition,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LawLabIdentificationMachineV1 {
    OperatorIdentificationMachineV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LawLabProbeDomainV1 {
    Filesystem,
    Git,
    Sqlite,
    StructuredData,
    StructuredCli,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LawLabTerminalVerdictV1 {
    UniqueLawCandidate,
    NoDistinguishingProbe,
    NoIdentifiableLaw,
    SandboxVerificationFail,
    BudgetExhausted,
    SafetyVeto,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LawLabPhaseV1 {
    ContractFrozen,
    NaturalResidualBound,
    VersionSpaceFrozen,
    ProbeSelected,
    PredictionsPrecommitted,
    ProbeExecuted,
    OutcomeVerified,
    Terminal,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabTransitionV1 {
    pub from: LawLabPhaseV1,
    pub to: LawLabPhaseV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabEvidencePolicyV1 {
    pub candidate_source: LawLabCandidateSourceV1,
    pub probe_evidence_class: LawLabProbeEvidenceClassV1,
    pub promotion_evidence_class: LawLabPromotionEvidenceClassV1,
    pub real_traffic_binding_required: bool,
    pub generated_fixtures_may_seed_candidate: bool,
    pub teacher_outputs_may_seed_candidate: bool,
    pub lab_probe_may_satisfy_natural_holdout: bool,
    pub post_candidate_natural_holdout_required: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabHypothesisPolicyV1 {
    pub identification_machine: LawLabIdentificationMachineV1,
    pub parallel_identifier_allowed: bool,
    pub operator_blind: bool,
    pub program_hints_allowed: bool,
    pub source_identity_may_grant_semantic_authority: bool,
    pub stable_hash_is_tie_break_only: bool,
    pub exact_replay_required: bool,
    pub semantic_quotient_required: bool,
    pub version_space_must_be_frozen_before_probe: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabProbePolicyV1 {
    pub selection_rule: LawLabProbeSelectionRuleV1,
    pub predictions_precommitted: bool,
    pub every_surviving_hypothesis_must_predict: bool,
    pub independent_oracle_required: bool,
    pub candidate_program_may_act_as_oracle: bool,
    pub exact_outcome_required: bool,
    pub probe_pending_releases_generation: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabSafetyPolicyV1 {
    pub allowed_domains: Vec<LawLabProbeDomainV1>,
    pub network_enabled: bool,
    pub production_state_mount_allowed: bool,
    pub production_writes_allowed: bool,
    pub secrets_mounted: bool,
    pub host_pid_namespace_enabled: bool,
    pub arbitrary_host_paths_allowed: bool,
    pub shell_interpretation_allowed: bool,
    pub source_snapshot_read_only: bool,
    pub disposable_workspace_required: bool,
    pub deterministic_seed_required: bool,
    pub cleanup_receipt_required: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabBudgetV1 {
    pub maximum_active_generations: u64,
    pub maximum_candidates: u64,
    pub maximum_natural_support_rows: u64,
    pub maximum_hypotheses: u64,
    pub maximum_probes: u64,
    pub maximum_generation_wall_ms: u64,
    pub maximum_probe_wall_ms: u64,
    pub maximum_probe_cpu_ms: u64,
    pub maximum_memory_bytes: u64,
    pub maximum_disk_bytes: u64,
    pub maximum_input_bytes: u64,
    pub maximum_output_bytes: u64,
    pub maximum_processes: u64,
    pub maximum_model_calls: u64,
    pub maximum_model_tokens: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabTerminalPolicyV1 {
    pub verdicts: Vec<LawLabTerminalVerdictV1>,
    pub allowed_terminalizations: Vec<LawLabTerminalRuleV1>,
    pub exactly_one_terminal_receipt_required: bool,
    pub terminal_receipt_releases_generation: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabTerminalRuleV1 {
    pub from: LawLabPhaseV1,
    pub verdict: LawLabTerminalVerdictV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabLifecyclePolicyV1 {
    pub initial_phase: LawLabPhaseV1,
    pub terminal_phase: LawLabPhaseV1,
    pub allowed_transitions: Vec<LawLabTransitionV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabAuthorityBoundaryV1 {
    pub lab_may_emit_candidate: bool,
    pub lab_may_issue_law_certificate: bool,
    pub lab_may_activate_package: bool,
    pub lab_may_grant_execution_authority: bool,
    pub lab_may_enter_k1_registry: bool,
    pub lab_may_mutate_phase_memory: bool,
    pub lab_may_receive_product_economics_credit: bool,
    pub external_natural_certification_required: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabContractV1 {
    pub schema: String,
    pub contract_root_sha256: String,
    pub contract_version: u64,
    pub evidence_policy: LawLabEvidencePolicyV1,
    pub hypothesis_policy: LawLabHypothesisPolicyV1,
    pub probe_policy: LawLabProbePolicyV1,
    pub safety_policy: LawLabSafetyPolicyV1,
    pub budget: LawLabBudgetV1,
    pub terminal_policy: LawLabTerminalPolicyV1,
    pub lifecycle_policy: LawLabLifecyclePolicyV1,
    pub authority_boundary: LawLabAuthorityBoundaryV1,
}

#[derive(Serialize)]
struct LawLabContractDigestV1<'a> {
    schema: &'static str,
    contract_version: u64,
    evidence_policy: &'a LawLabEvidencePolicyV1,
    hypothesis_policy: &'a LawLabHypothesisPolicyV1,
    probe_policy: &'a LawLabProbePolicyV1,
    safety_policy: &'a LawLabSafetyPolicyV1,
    budget: LawLabBudgetV1,
    terminal_policy: &'a LawLabTerminalPolicyV1,
    lifecycle_policy: &'a LawLabLifecyclePolicyV1,
    authority_boundary: &'a LawLabAuthorityBoundaryV1,
}

impl LawLabBudgetV1 {
    #[must_use]
    pub const fn preregistered_v1() -> Self {
        Self {
            maximum_active_generations: LAW_LAB_MAX_ACTIVE_GENERATIONS_V1,
            maximum_candidates: LAW_LAB_MAX_CANDIDATES_V1,
            maximum_natural_support_rows: LAW_LAB_MAX_NATURAL_SUPPORT_ROWS_V1,
            maximum_hypotheses: LAW_LAB_MAX_HYPOTHESES_V1,
            maximum_probes: LAW_LAB_MAX_PROBES_V1,
            maximum_generation_wall_ms: LAW_LAB_MAX_GENERATION_WALL_MS_V1,
            maximum_probe_wall_ms: LAW_LAB_MAX_PROBE_WALL_MS_V1,
            maximum_probe_cpu_ms: LAW_LAB_MAX_PROBE_CPU_MS_V1,
            maximum_memory_bytes: LAW_LAB_MAX_MEMORY_BYTES_V1,
            maximum_disk_bytes: LAW_LAB_MAX_DISK_BYTES_V1,
            maximum_input_bytes: LAW_LAB_MAX_INPUT_BYTES_V1,
            maximum_output_bytes: LAW_LAB_MAX_OUTPUT_BYTES_V1,
            maximum_processes: LAW_LAB_MAX_PROCESSES_V1,
            maximum_model_calls: LAW_LAB_MAX_MODEL_CALLS_V1,
            maximum_model_tokens: LAW_LAB_MAX_MODEL_TOKENS_V1,
        }
    }
}

impl LawLabContractV1 {
    pub fn preregistered_v1() -> Result<Self, &'static str> {
        let mut contract = Self {
            schema: LAW_LAB_CONTRACT_SCHEMA_V1.to_owned(),
            contract_root_sha256: String::new(),
            contract_version: LAW_LAB_CONTRACT_VERSION_V1,
            evidence_policy: evidence_policy_v1(),
            hypothesis_policy: hypothesis_policy_v1(),
            probe_policy: probe_policy_v1(),
            safety_policy: safety_policy_v1(),
            budget: LawLabBudgetV1::preregistered_v1(),
            terminal_policy: terminal_policy_v1(),
            lifecycle_policy: lifecycle_policy_v1(),
            authority_boundary: authority_boundary_v1(),
        };
        contract.contract_root_sha256 = contract.expected_root()?;
        contract.validate()?;
        Ok(contract)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != LAW_LAB_CONTRACT_SCHEMA_V1
            || self.contract_version != LAW_LAB_CONTRACT_VERSION_V1
            || !valid_nonzero_sha256(&self.contract_root_sha256)
            || self.evidence_policy != evidence_policy_v1()
            || self.hypothesis_policy != hypothesis_policy_v1()
            || self.probe_policy != probe_policy_v1()
            || self.safety_policy != safety_policy_v1()
            || self.budget != LawLabBudgetV1::preregistered_v1()
            || self.terminal_policy != terminal_policy_v1()
            || self.lifecycle_policy != lifecycle_policy_v1()
            || self.authority_boundary != authority_boundary_v1()
            || self.contract_root_sha256 != self.expected_root()?
        {
            return Err("law_lab_contract_invalid");
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, &'static str> {
        self.validate()?;
        canonical_json_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        let contract: Self =
            serde_json::from_slice(bytes).map_err(|_| "law_lab_contract_decode_failed")?;
        contract.validate()?;
        if canonical_json_bytes(&contract)? != bytes {
            return Err("law_lab_contract_not_canonical");
        }
        Ok(contract)
    }

    pub fn allows_transition(
        &self,
        from: LawLabPhaseV1,
        to: LawLabPhaseV1,
    ) -> Result<bool, &'static str> {
        self.validate()?;
        Ok(self
            .lifecycle_policy
            .allowed_transitions
            .contains(&LawLabTransitionV1 { from, to }))
    }

    pub fn recognizes_terminal_verdict(
        &self,
        verdict: LawLabTerminalVerdictV1,
    ) -> Result<bool, &'static str> {
        self.validate()?;
        Ok(self.terminal_policy.verdicts.contains(&verdict))
    }

    pub fn allows_terminalization(
        &self,
        from: LawLabPhaseV1,
        verdict: LawLabTerminalVerdictV1,
    ) -> Result<bool, &'static str> {
        self.validate()?;
        Ok(self
            .terminal_policy
            .allowed_terminalizations
            .contains(&LawLabTerminalRuleV1 { from, verdict }))
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&LawLabContractDigestV1 {
            schema: LAW_LAB_CONTRACT_SCHEMA_V1,
            contract_version: LAW_LAB_CONTRACT_VERSION_V1,
            evidence_policy: &self.evidence_policy,
            hypothesis_policy: &self.hypothesis_policy,
            probe_policy: &self.probe_policy,
            safety_policy: &self.safety_policy,
            budget: self.budget,
            terminal_policy: &self.terminal_policy,
            lifecycle_policy: &self.lifecycle_policy,
            authority_boundary: &self.authority_boundary,
        })
    }
}

fn evidence_policy_v1() -> LawLabEvidencePolicyV1 {
    LawLabEvidencePolicyV1 {
        candidate_source: LawLabCandidateSourceV1::OrdinaryTrafficResidual,
        probe_evidence_class: LawLabProbeEvidenceClassV1::IsolatedExperiment,
        promotion_evidence_class: LawLabPromotionEvidenceClassV1::PostCandidateNaturalHoldout,
        real_traffic_binding_required: true,
        generated_fixtures_may_seed_candidate: false,
        teacher_outputs_may_seed_candidate: false,
        lab_probe_may_satisfy_natural_holdout: false,
        post_candidate_natural_holdout_required: true,
    }
}

fn hypothesis_policy_v1() -> LawLabHypothesisPolicyV1 {
    LawLabHypothesisPolicyV1 {
        identification_machine: LawLabIdentificationMachineV1::OperatorIdentificationMachineV1,
        parallel_identifier_allowed: false,
        operator_blind: true,
        program_hints_allowed: false,
        source_identity_may_grant_semantic_authority: false,
        stable_hash_is_tie_break_only: true,
        exact_replay_required: true,
        semantic_quotient_required: true,
        version_space_must_be_frozen_before_probe: true,
    }
}

fn probe_policy_v1() -> LawLabProbePolicyV1 {
    LawLabProbePolicyV1 {
        selection_rule: LawLabProbeSelectionRuleV1::MaximumDistinguishingPartition,
        predictions_precommitted: true,
        every_surviving_hypothesis_must_predict: true,
        independent_oracle_required: true,
        candidate_program_may_act_as_oracle: false,
        exact_outcome_required: true,
        probe_pending_releases_generation: false,
    }
}

fn safety_policy_v1() -> LawLabSafetyPolicyV1 {
    LawLabSafetyPolicyV1 {
        allowed_domains: vec![
            LawLabProbeDomainV1::Filesystem,
            LawLabProbeDomainV1::Git,
            LawLabProbeDomainV1::Sqlite,
            LawLabProbeDomainV1::StructuredData,
            LawLabProbeDomainV1::StructuredCli,
        ],
        network_enabled: false,
        production_state_mount_allowed: false,
        production_writes_allowed: false,
        secrets_mounted: false,
        host_pid_namespace_enabled: false,
        arbitrary_host_paths_allowed: false,
        shell_interpretation_allowed: false,
        source_snapshot_read_only: true,
        disposable_workspace_required: true,
        deterministic_seed_required: true,
        cleanup_receipt_required: true,
    }
}

fn terminal_policy_v1() -> LawLabTerminalPolicyV1 {
    use LawLabPhaseV1::{
        ContractFrozen, NaturalResidualBound, OutcomeVerified, PredictionsPrecommitted,
        ProbeExecuted, ProbeSelected, VersionSpaceFrozen,
    };
    use LawLabTerminalVerdictV1::{
        BudgetExhausted, NoDistinguishingProbe, NoIdentifiableLaw, SafetyVeto,
        SandboxVerificationFail, UniqueLawCandidate,
    };

    LawLabTerminalPolicyV1 {
        verdicts: vec![
            UniqueLawCandidate,
            NoDistinguishingProbe,
            NoIdentifiableLaw,
            SandboxVerificationFail,
            BudgetExhausted,
            SafetyVeto,
        ],
        allowed_terminalizations: vec![
            LawLabTerminalRuleV1 {
                from: VersionSpaceFrozen,
                verdict: UniqueLawCandidate,
            },
            LawLabTerminalRuleV1 {
                from: OutcomeVerified,
                verdict: UniqueLawCandidate,
            },
            LawLabTerminalRuleV1 {
                from: VersionSpaceFrozen,
                verdict: NoDistinguishingProbe,
            },
            LawLabTerminalRuleV1 {
                from: OutcomeVerified,
                verdict: NoDistinguishingProbe,
            },
            LawLabTerminalRuleV1 {
                from: VersionSpaceFrozen,
                verdict: NoIdentifiableLaw,
            },
            LawLabTerminalRuleV1 {
                from: OutcomeVerified,
                verdict: NoIdentifiableLaw,
            },
            LawLabTerminalRuleV1 {
                from: ProbeSelected,
                verdict: SandboxVerificationFail,
            },
            LawLabTerminalRuleV1 {
                from: PredictionsPrecommitted,
                verdict: SandboxVerificationFail,
            },
            LawLabTerminalRuleV1 {
                from: ProbeExecuted,
                verdict: SandboxVerificationFail,
            },
            LawLabTerminalRuleV1 {
                from: ContractFrozen,
                verdict: BudgetExhausted,
            },
            LawLabTerminalRuleV1 {
                from: NaturalResidualBound,
                verdict: BudgetExhausted,
            },
            LawLabTerminalRuleV1 {
                from: VersionSpaceFrozen,
                verdict: BudgetExhausted,
            },
            LawLabTerminalRuleV1 {
                from: ProbeSelected,
                verdict: BudgetExhausted,
            },
            LawLabTerminalRuleV1 {
                from: PredictionsPrecommitted,
                verdict: BudgetExhausted,
            },
            LawLabTerminalRuleV1 {
                from: ProbeExecuted,
                verdict: BudgetExhausted,
            },
            LawLabTerminalRuleV1 {
                from: OutcomeVerified,
                verdict: BudgetExhausted,
            },
            LawLabTerminalRuleV1 {
                from: ContractFrozen,
                verdict: SafetyVeto,
            },
            LawLabTerminalRuleV1 {
                from: NaturalResidualBound,
                verdict: SafetyVeto,
            },
            LawLabTerminalRuleV1 {
                from: VersionSpaceFrozen,
                verdict: SafetyVeto,
            },
            LawLabTerminalRuleV1 {
                from: ProbeSelected,
                verdict: SafetyVeto,
            },
            LawLabTerminalRuleV1 {
                from: PredictionsPrecommitted,
                verdict: SafetyVeto,
            },
            LawLabTerminalRuleV1 {
                from: ProbeExecuted,
                verdict: SafetyVeto,
            },
            LawLabTerminalRuleV1 {
                from: OutcomeVerified,
                verdict: SafetyVeto,
            },
        ],
        exactly_one_terminal_receipt_required: true,
        terminal_receipt_releases_generation: true,
    }
}

fn lifecycle_policy_v1() -> LawLabLifecyclePolicyV1 {
    use LawLabPhaseV1::{
        ContractFrozen, NaturalResidualBound, OutcomeVerified, PredictionsPrecommitted,
        ProbeExecuted, ProbeSelected, Terminal, VersionSpaceFrozen,
    };

    LawLabLifecyclePolicyV1 {
        initial_phase: ContractFrozen,
        terminal_phase: Terminal,
        allowed_transitions: vec![
            LawLabTransitionV1 {
                from: ContractFrozen,
                to: NaturalResidualBound,
            },
            LawLabTransitionV1 {
                from: NaturalResidualBound,
                to: VersionSpaceFrozen,
            },
            LawLabTransitionV1 {
                from: VersionSpaceFrozen,
                to: ProbeSelected,
            },
            LawLabTransitionV1 {
                from: ProbeSelected,
                to: PredictionsPrecommitted,
            },
            LawLabTransitionV1 {
                from: PredictionsPrecommitted,
                to: ProbeExecuted,
            },
            LawLabTransitionV1 {
                from: ProbeExecuted,
                to: OutcomeVerified,
            },
            LawLabTransitionV1 {
                from: OutcomeVerified,
                to: VersionSpaceFrozen,
            },
        ],
    }
}

fn authority_boundary_v1() -> LawLabAuthorityBoundaryV1 {
    LawLabAuthorityBoundaryV1 {
        lab_may_emit_candidate: true,
        lab_may_issue_law_certificate: false,
        lab_may_activate_package: false,
        lab_may_grant_execution_authority: false,
        lab_may_enter_k1_registry: false,
        lab_may_mutate_phase_memory: false,
        lab_may_receive_product_economics_credit: false,
        external_natural_certification_required: true,
    }
}
