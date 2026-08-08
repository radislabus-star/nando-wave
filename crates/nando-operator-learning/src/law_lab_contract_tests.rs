use serde_json::Value;

use crate::law_lab_contract::{
    LAW_LAB_MAX_ACTIVE_GENERATIONS_V1, LAW_LAB_MAX_CANDIDATES_V1, LAW_LAB_MAX_HYPOTHESES_V1,
    LAW_LAB_MAX_MODEL_CALLS_V1, LAW_LAB_MAX_MODEL_TOKENS_V1, LAW_LAB_MAX_PROBES_V1,
    LawLabContractV1, LawLabIdentificationMachineV1, LawLabPhaseV1, LawLabTerminalVerdictV1,
};

const CHECKED_IN_CONTRACT_V1: &[u8] =
    include_bytes!("../../../plans/law-lab-v1/LAW_LAB_CONTRACT_V1.json");

#[test]
fn preregistered_contract_roundtrips_byte_identically() {
    let contract = LawLabContractV1::preregistered_v1().expect("contract");
    let bytes = contract.canonical_bytes().expect("canonical bytes");
    let restored = LawLabContractV1::from_canonical_bytes(&bytes).expect("restore");

    assert_eq!(restored, contract);
    assert_eq!(restored.canonical_bytes(), Ok(bytes));
}

#[test]
fn checked_in_contract_is_the_exact_generated_canonical_artifact() {
    let contract = LawLabContractV1::preregistered_v1().expect("contract");
    let generated = contract.canonical_bytes().expect("canonical bytes");

    assert_eq!(generated, CHECKED_IN_CONTRACT_V1);
    assert_eq!(
        LawLabContractV1::from_canonical_bytes(CHECKED_IN_CONTRACT_V1),
        Ok(contract)
    );
}

#[test]
fn budget_is_single_generation_bounded_and_model_free() {
    let contract = LawLabContractV1::preregistered_v1().expect("contract");

    assert_eq!(
        contract.budget.maximum_active_generations,
        LAW_LAB_MAX_ACTIVE_GENERATIONS_V1
    );
    assert_eq!(
        contract.budget.maximum_candidates,
        LAW_LAB_MAX_CANDIDATES_V1
    );
    assert_eq!(
        contract.budget.maximum_hypotheses,
        LAW_LAB_MAX_HYPOTHESES_V1
    );
    assert_eq!(contract.budget.maximum_probes, LAW_LAB_MAX_PROBES_V1);
    assert_eq!(
        contract.budget.maximum_model_calls,
        LAW_LAB_MAX_MODEL_CALLS_V1
    );
    assert_eq!(
        contract.budget.maximum_model_tokens,
        LAW_LAB_MAX_MODEL_TOKENS_V1
    );
}

#[test]
fn lab_probe_cannot_substitute_for_natural_evidence_or_authority() {
    let contract = LawLabContractV1::preregistered_v1().expect("contract");
    let evidence = &contract.evidence_policy;
    let authority = &contract.authority_boundary;
    let hypotheses = &contract.hypothesis_policy;

    assert_eq!(
        hypotheses.identification_machine,
        LawLabIdentificationMachineV1::OperatorIdentificationMachineV1
    );
    assert!(!hypotheses.parallel_identifier_allowed);
    assert!(hypotheses.operator_blind);
    assert!(hypotheses.exact_replay_required);
    assert!(hypotheses.semantic_quotient_required);
    assert!(evidence.real_traffic_binding_required);
    assert!(!evidence.generated_fixtures_may_seed_candidate);
    assert!(!evidence.teacher_outputs_may_seed_candidate);
    assert!(!evidence.lab_probe_may_satisfy_natural_holdout);
    assert!(evidence.post_candidate_natural_holdout_required);
    assert!(authority.lab_may_emit_candidate);
    assert!(!authority.lab_may_issue_law_certificate);
    assert!(!authority.lab_may_activate_package);
    assert!(!authority.lab_may_grant_execution_authority);
    assert!(!authority.lab_may_enter_k1_registry);
    assert!(!authority.lab_may_mutate_phase_memory);
    assert!(!authority.lab_may_receive_product_economics_credit);
    assert!(authority.external_natural_certification_required);
}

#[test]
fn lifecycle_requires_prediction_precommit_and_a_fresh_version_freeze() {
    let contract = LawLabContractV1::preregistered_v1().expect("contract");

    assert_eq!(
        contract.allows_transition(
            LawLabPhaseV1::ProbeSelected,
            LawLabPhaseV1::PredictionsPrecommitted
        ),
        Ok(true)
    );
    assert_eq!(
        contract.allows_transition(
            LawLabPhaseV1::PredictionsPrecommitted,
            LawLabPhaseV1::ProbeExecuted
        ),
        Ok(true)
    );
    assert_eq!(
        contract.allows_transition(LawLabPhaseV1::ProbeSelected, LawLabPhaseV1::ProbeExecuted),
        Ok(false)
    );
    assert_eq!(
        contract.allows_transition(
            LawLabPhaseV1::OutcomeVerified,
            LawLabPhaseV1::VersionSpaceFrozen
        ),
        Ok(true)
    );
    assert_eq!(
        contract.allows_transition(LawLabPhaseV1::OutcomeVerified, LawLabPhaseV1::ProbeSelected),
        Ok(false)
    );
    assert_eq!(
        contract.allows_transition(LawLabPhaseV1::Terminal, LawLabPhaseV1::ContractFrozen),
        Ok(false)
    );
    assert_eq!(
        contract.allows_transition(LawLabPhaseV1::VersionSpaceFrozen, LawLabPhaseV1::Terminal),
        Ok(false)
    );
}

#[test]
fn all_preregistered_verdicts_are_terminal_and_probe_pending_is_not_a_verdict() {
    let contract = LawLabContractV1::preregistered_v1().expect("contract");
    let expected = [
        LawLabTerminalVerdictV1::UniqueLawCandidate,
        LawLabTerminalVerdictV1::NoDistinguishingProbe,
        LawLabTerminalVerdictV1::NoIdentifiableLaw,
        LawLabTerminalVerdictV1::SandboxVerificationFail,
        LawLabTerminalVerdictV1::BudgetExhausted,
        LawLabTerminalVerdictV1::SafetyVeto,
    ];

    assert_eq!(contract.terminal_policy.verdicts, expected);
    assert!(
        contract
            .terminal_policy
            .exactly_one_terminal_receipt_required
    );
    assert!(
        contract
            .terminal_policy
            .terminal_receipt_releases_generation
    );
    assert!(!contract.probe_policy.probe_pending_releases_generation);
    for verdict in expected {
        assert_eq!(contract.recognizes_terminal_verdict(verdict), Ok(true));
    }
}

#[test]
fn terminal_verdict_is_bound_to_the_phase_that_proves_it() {
    let contract = LawLabContractV1::preregistered_v1().expect("contract");

    assert_eq!(
        contract.allows_terminalization(
            LawLabPhaseV1::VersionSpaceFrozen,
            LawLabTerminalVerdictV1::UniqueLawCandidate
        ),
        Ok(true)
    );
    assert_eq!(
        contract.allows_terminalization(
            LawLabPhaseV1::OutcomeVerified,
            LawLabTerminalVerdictV1::UniqueLawCandidate
        ),
        Ok(true)
    );
    for premature_phase in [
        LawLabPhaseV1::ContractFrozen,
        LawLabPhaseV1::NaturalResidualBound,
        LawLabPhaseV1::ProbeSelected,
        LawLabPhaseV1::PredictionsPrecommitted,
        LawLabPhaseV1::ProbeExecuted,
    ] {
        assert_eq!(
            contract.allows_terminalization(
                premature_phase,
                LawLabTerminalVerdictV1::UniqueLawCandidate
            ),
            Ok(false)
        );
    }
    assert_eq!(
        contract.allows_terminalization(
            LawLabPhaseV1::ProbeExecuted,
            LawLabTerminalVerdictV1::SandboxVerificationFail
        ),
        Ok(true)
    );
    assert_eq!(
        contract
            .allows_terminalization(LawLabPhaseV1::Terminal, LawLabTerminalVerdictV1::SafetyVeto),
        Ok(false)
    );
}

#[test]
fn modified_budget_or_authority_boundary_cannot_be_resealed_by_deserialization() {
    let contract = LawLabContractV1::preregistered_v1().expect("contract");
    let bytes = contract.canonical_bytes().expect("canonical bytes");
    let mut value: Value = serde_json::from_slice(&bytes).expect("json");

    value["budget"]["maximum_probes"] = Value::from(9_u64);
    let modified_budget = serde_json::to_vec(&value).expect("modified budget");
    assert_eq!(
        LawLabContractV1::from_canonical_bytes(&modified_budget),
        Err("law_lab_contract_invalid")
    );

    let mut value: Value = serde_json::from_slice(&bytes).expect("json");
    value["authority_boundary"]["lab_may_activate_package"] = Value::Bool(true);
    let modified_authority = serde_json::to_vec(&value).expect("modified authority");
    assert_eq!(
        LawLabContractV1::from_canonical_bytes(&modified_authority),
        Err("law_lab_contract_invalid")
    );
}

#[test]
fn noncanonical_or_unknown_contract_bytes_are_rejected() {
    let contract = LawLabContractV1::preregistered_v1().expect("contract");
    let bytes = contract.canonical_bytes().expect("canonical bytes");
    let mut padded = b" ".to_vec();
    padded.extend_from_slice(&bytes);
    assert_eq!(
        LawLabContractV1::from_canonical_bytes(&padded),
        Err("law_lab_contract_not_canonical")
    );

    let mut value: Value = serde_json::from_slice(&bytes).expect("json");
    value["unexpected"] = Value::Bool(true);
    let unknown = serde_json::to_vec(&value).expect("unknown field");
    assert_eq!(
        LawLabContractV1::from_canonical_bytes(&unknown),
        Err("law_lab_contract_decode_failed")
    );
}
