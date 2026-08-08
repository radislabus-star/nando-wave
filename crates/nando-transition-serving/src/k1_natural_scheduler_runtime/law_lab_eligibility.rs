use nando_operator_learning::{
    LawLabContractV1, LawLabProbeDomainV1, LawLabSandboxAuthorityBoundaryV1,
    LawLabSandboxExecutorManifestV1, LawLabSandboxPurposeV1, LawLabSandboxRequestV1,
};
use serde::{Deserialize, Serialize};

use super::*;

const LAW_LAB_K1_ELIGIBILITY_REPORT_SCHEMA_V1: &str = "nando.law-lab-k1-eligibility-report.v1";
const LAW_LAB_ACTIVE_PROBE_PLAN_SCHEMA_V1: &str = "nando.law-lab-active-probe-plan.v1";
const LAW_LAB_ACTIVE_PROBE_OUTCOME_CONTRACT_V1: &str = "post_work_tree_root_sha256";

const NO_ACTIVE_CANDIDATE_BLOCKER: &str = "no_active_candidate_freeze";
const IDENTIFICATION_FREEZE_MISSING_BLOCKER: &str = "identification_freeze_missing";
const VERSION_SPACE_NOT_AMBIGUOUS_BLOCKER: &str = "version_space_not_ambiguous";
const PROBE_PREDICTION_PRECOMMIT_MISSING_BLOCKER: &str =
    "durable_probe_prediction_precommit_missing";
const ACTIVE_PROBE_CONTRACT_MISSING_BLOCKER: &str = "active_probe_execution_contract_missing";
const EXECUTOR_ATTESTATION_MISSING_BLOCKER: &str = "sandbox_executor_attestation_missing";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LawLabK1EligibilityStateV1 {
    NoEligibleLawLabProbe,
    AwaitingIdentificationFreeze,
    AwaitingProbePredictionPrecommit,
    AwaitingActiveProbeContract,
    AwaitingExecutorAttestation,
    ReadyForSandboxExecution,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LawLabK1EligibilityReportV1 {
    pub schema: String,
    pub report_root_sha256: String,
    pub generated_at_unix: u64,
    pub state: LawLabK1EligibilityStateV1,
    pub blocker: String,
    pub contract_root_sha256: String,
    pub scheduler_projection_root_sha256: String,
    pub scheduler_ledger_revision: u64,
    pub scheduler_ledger_root_sha256: String,
    pub scheduler_latest_event_root_sha256: String,
    pub active_candidate_freeze_root_sha256: Option<String>,
    pub natural_candidate_root_sha256: Option<String>,
    pub identification_freeze_root_sha256: Option<String>,
    pub current_version_space_root_sha256: Option<String>,
    pub current_semantic_class_roots_sha256: Vec<String>,
    pub pending_probe_receipt_root_sha256: Option<String>,
    pub precommitted_predictions_root_sha256: Option<String>,
    pub active_probe_request_root_sha256: Option<String>,
    pub active_probe_source_tree_root_sha256: Option<String>,
    pub active_probe_domain: Option<LawLabProbeDomainV1>,
    pub executor_manifest_root_sha256: Option<String>,
    pub latest_terminal_verdict_root_sha256: Option<String>,
    pub latest_terminal_blocker: String,
    pub multi_source_research_enabled: bool,
    pub sandbox_execution_allowed: bool,
    pub authority: LawLabSandboxAuthorityBoundaryV1,
}

#[derive(Serialize)]
struct LawLabK1EligibilityReportDigestV1<'a> {
    schema: &'static str,
    generated_at_unix: u64,
    state: LawLabK1EligibilityStateV1,
    blocker: &'a str,
    contract_root_sha256: &'a str,
    scheduler_projection_root_sha256: &'a str,
    scheduler_ledger_revision: u64,
    scheduler_ledger_root_sha256: &'a str,
    scheduler_latest_event_root_sha256: &'a str,
    active_candidate_freeze_root_sha256: Option<&'a str>,
    natural_candidate_root_sha256: Option<&'a str>,
    identification_freeze_root_sha256: Option<&'a str>,
    current_version_space_root_sha256: Option<&'a str>,
    current_semantic_class_roots_sha256: &'a [String],
    pending_probe_receipt_root_sha256: Option<&'a str>,
    precommitted_predictions_root_sha256: Option<&'a str>,
    active_probe_request_root_sha256: Option<&'a str>,
    active_probe_source_tree_root_sha256: Option<&'a str>,
    active_probe_domain: Option<LawLabProbeDomainV1>,
    executor_manifest_root_sha256: Option<&'a str>,
    latest_terminal_verdict_root_sha256: Option<&'a str>,
    latest_terminal_blocker: &'a str,
    multi_source_research_enabled: bool,
    sandbox_execution_allowed: bool,
    authority: &'a LawLabSandboxAuthorityBoundaryV1,
}

pub(crate) fn law_lab_eligibility_report(
    projection: &K1SchedulerProjectionV1,
    active_probe_request: Option<&LawLabSandboxRequestV1>,
    executor_manifest: Option<&LawLabSandboxExecutorManifestV1>,
    multi_source_research_enabled: bool,
    generated_at_unix: u64,
) -> Result<LawLabK1EligibilityReportV1, String> {
    let contract = LawLabContractV1::preregistered_v1().map_err(str::to_owned)?;
    let classes = current_classes(projection);
    let candidate = projection.active_candidate_freeze.as_ref();
    let identification = projection.identification_freeze.as_ref();
    let pending_probe = projection
        .latest_probe_round
        .as_ref()
        .filter(|receipt| receipt.state == K1ProbeRoundStateV1::ProbePending);
    let current_version_space_root_sha256 = projection
        .latest_probe_round
        .as_ref()
        .and_then(|receipt| match receipt.state {
            K1ProbeRoundStateV1::ProbePending => {
                Some(receipt.previous_version_space_root_sha256.clone())
            }
            K1ProbeRoundStateV1::OutcomeApplied | K1ProbeRoundStateV1::OutcomeCensored => {
                receipt.next_version_space_root_sha256.clone()
            }
        })
        .or_else(|| identification.map(|freeze| freeze.initial_version_space_root_sha256.clone()));

    if let Some(request) = active_probe_request {
        validate_active_probe_request(projection, &classes, pending_probe, request)?;
    }
    if let Some(manifest) = executor_manifest {
        manifest.validate().map_err(|error| error.to_string())?;
        let request = active_probe_request
            .ok_or_else(|| "law_lab_executor_manifest_without_request".to_owned())?;
        if request.executor_manifest_root_sha256 != manifest.manifest_root_sha256
            || request.worker_sha256 != manifest.worker_sha256
            || !manifest.supported_domains.contains(&request.domain)
        {
            return Err("law_lab_executor_manifest_binding_invalid".to_owned());
        }
    }

    let (state, blocker) = classify_eligibility(
        candidate.is_some(),
        identification.is_some(),
        classes.len(),
        pending_probe.is_some(),
        active_probe_request.is_some(),
        executor_manifest.is_some(),
    );
    let sandbox_execution_allowed = state == LawLabK1EligibilityStateV1::ReadyForSandboxExecution
        && multi_source_research_enabled;
    let latest_terminal = projection.latest_terminal_verdict.as_ref();
    let mut report = LawLabK1EligibilityReportV1 {
        schema: LAW_LAB_K1_ELIGIBILITY_REPORT_SCHEMA_V1.to_owned(),
        report_root_sha256: String::new(),
        generated_at_unix,
        state,
        blocker: blocker.to_owned(),
        contract_root_sha256: contract.contract_root_sha256,
        scheduler_projection_root_sha256: projection.projection_root_sha256.clone(),
        scheduler_ledger_revision: projection.ledger_revision,
        scheduler_ledger_root_sha256: projection.ledger_root_sha256.clone(),
        scheduler_latest_event_root_sha256: projection.latest_event_root_sha256.clone(),
        active_candidate_freeze_root_sha256: candidate
            .map(|freeze| freeze.freeze_root_sha256.clone()),
        natural_candidate_root_sha256: candidate.map(|freeze| freeze.candidate_root_sha256.clone()),
        identification_freeze_root_sha256: identification
            .map(|freeze| freeze.freeze_root_sha256.clone()),
        current_version_space_root_sha256,
        current_semantic_class_roots_sha256: classes,
        pending_probe_receipt_root_sha256: pending_probe
            .map(|receipt| receipt.receipt_root_sha256.clone()),
        precommitted_predictions_root_sha256: pending_probe
            .map(|receipt| receipt.precommitted_predictions_root_sha256.clone()),
        active_probe_request_root_sha256: active_probe_request
            .map(|request| request.request_root_sha256.clone()),
        active_probe_source_tree_root_sha256: active_probe_request
            .map(|request| request.source_tree_root_sha256.clone()),
        active_probe_domain: active_probe_request.map(|request| request.domain),
        executor_manifest_root_sha256: executor_manifest
            .map(|manifest| manifest.manifest_root_sha256.clone()),
        latest_terminal_verdict_root_sha256: latest_terminal
            .map(|receipt| receipt.verdict_root_sha256.clone()),
        latest_terminal_blocker: latest_terminal
            .map_or_else(String::new, |receipt| receipt.blocker.clone()),
        multi_source_research_enabled,
        sandbox_execution_allowed,
        authority: LawLabSandboxAuthorityBoundaryV1::authority_free_v1(),
    };
    report.report_root_sha256 = report.expected_root()?;
    report.validate()?;
    Ok(report)
}

fn classify_eligibility(
    active_candidate: bool,
    identification_frozen: bool,
    semantic_class_count: usize,
    predictions_precommitted: bool,
    active_probe_contract_bound: bool,
    executor_attested: bool,
) -> (LawLabK1EligibilityStateV1, &'static str) {
    if !active_candidate {
        return (
            LawLabK1EligibilityStateV1::NoEligibleLawLabProbe,
            NO_ACTIVE_CANDIDATE_BLOCKER,
        );
    }
    if !identification_frozen {
        return (
            LawLabK1EligibilityStateV1::AwaitingIdentificationFreeze,
            IDENTIFICATION_FREEZE_MISSING_BLOCKER,
        );
    }
    if semantic_class_count < 2 {
        return (
            LawLabK1EligibilityStateV1::NoEligibleLawLabProbe,
            VERSION_SPACE_NOT_AMBIGUOUS_BLOCKER,
        );
    }
    if !predictions_precommitted {
        return (
            LawLabK1EligibilityStateV1::AwaitingProbePredictionPrecommit,
            PROBE_PREDICTION_PRECOMMIT_MISSING_BLOCKER,
        );
    }
    if !active_probe_contract_bound {
        return (
            LawLabK1EligibilityStateV1::AwaitingActiveProbeContract,
            ACTIVE_PROBE_CONTRACT_MISSING_BLOCKER,
        );
    }
    if !executor_attested {
        return (
            LawLabK1EligibilityStateV1::AwaitingExecutorAttestation,
            EXECUTOR_ATTESTATION_MISSING_BLOCKER,
        );
    }
    (LawLabK1EligibilityStateV1::ReadyForSandboxExecution, "")
}

fn validate_active_probe_request(
    projection: &K1SchedulerProjectionV1,
    classes: &[String],
    pending_probe: Option<&K1ProbeRoundReceiptV1>,
    request: &LawLabSandboxRequestV1,
) -> Result<(), String> {
    request.validate().map_err(|error| error.to_string())?;
    let candidate = projection
        .active_candidate_freeze
        .as_ref()
        .ok_or_else(|| NO_ACTIVE_CANDIDATE_BLOCKER.to_owned())?;
    let pending =
        pending_probe.ok_or_else(|| PROBE_PREDICTION_PRECOMMIT_MISSING_BLOCKER.to_owned())?;
    let class_count = u64::try_from(classes.len())
        .map_err(|_| "law_lab_semantic_class_count_overflow".to_owned())?;
    let prediction_count = u64::try_from(pending.class_partition_predictions.len())
        .map_err(|_| "law_lab_prediction_count_overflow".to_owned())?;
    let expected_probe_root_sha256 = active_probe_plan_root(request)?;
    if request.purpose != LawLabSandboxPurposeV1::ActiveDistinguishingProbe
        || request.candidate_root_sha256 != candidate.candidate_root_sha256
        || request.version_space_root_sha256 != pending.previous_version_space_root_sha256
        || request.durable_prediction_ledger_root_sha256 != projection.ledger_root_sha256
        || request.probe_root_sha256 != pending.selected_probe_root_sha256
        || request.probe_root_sha256 != expected_probe_root_sha256
        || request.surviving_hypothesis_count != class_count
        || request.precommitted_prediction_count != prediction_count
        || request.precommitted_prediction_count != request.surviving_hypothesis_count
    {
        return Err("law_lab_active_probe_binding_invalid".to_owned());
    }
    Ok(())
}

fn active_probe_plan_root(request: &LawLabSandboxRequestV1) -> Result<String, String> {
    canonical_json_sha256(&(
        LAW_LAB_ACTIVE_PROBE_PLAN_SCHEMA_V1,
        request.contract_root_sha256.as_str(),
        request.candidate_root_sha256.as_str(),
        request.version_space_root_sha256.as_str(),
        request.source_tree_root_sha256.as_str(),
        request.deterministic_seed_sha256.as_str(),
        request.domain,
        request.surviving_hypothesis_count,
        LAW_LAB_ACTIVE_PROBE_OUTCOME_CONTRACT_V1,
        &request.operations,
    ))
    .map_err(str::to_owned)
}

impl LawLabK1EligibilityReportV1 {
    fn validate(&self) -> Result<(), String> {
        let required_roots = [
            self.report_root_sha256.as_str(),
            self.contract_root_sha256.as_str(),
            self.scheduler_projection_root_sha256.as_str(),
            self.scheduler_ledger_root_sha256.as_str(),
            self.scheduler_latest_event_root_sha256.as_str(),
        ];
        let optional_roots = [
            self.active_candidate_freeze_root_sha256.as_deref(),
            self.natural_candidate_root_sha256.as_deref(),
            self.identification_freeze_root_sha256.as_deref(),
            self.current_version_space_root_sha256.as_deref(),
            self.pending_probe_receipt_root_sha256.as_deref(),
            self.precommitted_predictions_root_sha256.as_deref(),
            self.active_probe_request_root_sha256.as_deref(),
            self.active_probe_source_tree_root_sha256.as_deref(),
            self.executor_manifest_root_sha256.as_deref(),
            self.latest_terminal_verdict_root_sha256.as_deref(),
        ];
        let (expected_state, expected_blocker) = classify_eligibility(
            self.active_candidate_freeze_root_sha256.is_some(),
            self.identification_freeze_root_sha256.is_some(),
            self.current_semantic_class_roots_sha256.len(),
            self.pending_probe_receipt_root_sha256.is_some()
                && self.precommitted_predictions_root_sha256.is_some(),
            self.active_probe_request_root_sha256.is_some()
                && self.active_probe_source_tree_root_sha256.is_some()
                && self.active_probe_domain.is_some(),
            self.executor_manifest_root_sha256.is_some(),
        );
        if self.schema != LAW_LAB_K1_ELIGIBILITY_REPORT_SCHEMA_V1
            || !required_roots.into_iter().all(valid_nonzero_sha256)
            || optional_roots
                .into_iter()
                .flatten()
                .any(|root| !valid_nonzero_sha256(root))
            || self
                .current_semantic_class_roots_sha256
                .iter()
                .any(|root| !valid_nonzero_sha256(root))
            || !self
                .current_semantic_class_roots_sha256
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            || self.active_candidate_freeze_root_sha256.is_some()
                != self.natural_candidate_root_sha256.is_some()
            || self.pending_probe_receipt_root_sha256.is_some()
                != self.precommitted_predictions_root_sha256.is_some()
            || self.active_probe_request_root_sha256.is_some()
                != self.active_probe_source_tree_root_sha256.is_some()
            || self.active_probe_request_root_sha256.is_some() != self.active_probe_domain.is_some()
            || self.executor_manifest_root_sha256.is_some()
                && self.active_probe_request_root_sha256.is_none()
            || self.state != expected_state
            || self.blocker != expected_blocker
            || self.sandbox_execution_allowed
                != (self.state == LawLabK1EligibilityStateV1::ReadyForSandboxExecution
                    && self.multi_source_research_enabled)
            || self.authority.validate().is_err()
            || self.report_root_sha256 != self.expected_root()?
        {
            return Err("law_lab_k1_eligibility_report_invalid".to_owned());
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, String> {
        canonical_json_sha256(&LawLabK1EligibilityReportDigestV1 {
            schema: LAW_LAB_K1_ELIGIBILITY_REPORT_SCHEMA_V1,
            generated_at_unix: self.generated_at_unix,
            state: self.state,
            blocker: &self.blocker,
            contract_root_sha256: &self.contract_root_sha256,
            scheduler_projection_root_sha256: &self.scheduler_projection_root_sha256,
            scheduler_ledger_revision: self.scheduler_ledger_revision,
            scheduler_ledger_root_sha256: &self.scheduler_ledger_root_sha256,
            scheduler_latest_event_root_sha256: &self.scheduler_latest_event_root_sha256,
            active_candidate_freeze_root_sha256: self
                .active_candidate_freeze_root_sha256
                .as_deref(),
            natural_candidate_root_sha256: self.natural_candidate_root_sha256.as_deref(),
            identification_freeze_root_sha256: self.identification_freeze_root_sha256.as_deref(),
            current_version_space_root_sha256: self.current_version_space_root_sha256.as_deref(),
            current_semantic_class_roots_sha256: &self.current_semantic_class_roots_sha256,
            pending_probe_receipt_root_sha256: self.pending_probe_receipt_root_sha256.as_deref(),
            precommitted_predictions_root_sha256: self
                .precommitted_predictions_root_sha256
                .as_deref(),
            active_probe_request_root_sha256: self.active_probe_request_root_sha256.as_deref(),
            active_probe_source_tree_root_sha256: self
                .active_probe_source_tree_root_sha256
                .as_deref(),
            active_probe_domain: self.active_probe_domain,
            executor_manifest_root_sha256: self.executor_manifest_root_sha256.as_deref(),
            latest_terminal_verdict_root_sha256: self
                .latest_terminal_verdict_root_sha256
                .as_deref(),
            latest_terminal_blocker: &self.latest_terminal_blocker,
            multi_source_research_enabled: self.multi_source_research_enabled,
            sandbox_execution_allowed: self.sandbox_execution_allowed,
            authority: &self.authority,
        })
        .map_err(str::to_owned)
    }
}

#[cfg(test)]
mod tests {
    use nando_operator_learning::{LawLabSandboxOperationV1, LawLabSandboxRequestInputV1};

    use super::*;

    fn root(label: &str) -> String {
        canonical_json_sha256(&label).expect("root")
    }

    fn inactive_projection() -> K1SchedulerProjectionV1 {
        K1SchedulerProjectionV1 {
            schema: "nando.k1-scheduler-projection.v1".to_owned(),
            projection_root_sha256: root("projection"),
            ledger_revision: 91,
            ledger_root_sha256: root("ledger"),
            latest_event_root_sha256: root("latest-event"),
            completed_generations: 44,
            completed_candidate_roots_sha256: Vec::new(),
            next_generation_sequence: 45,
            active_candidate_freeze: None,
            identification_freeze: None,
            future_prediction_contract: None,
            future_predictions: Vec::new(),
            future_prediction_censors: Vec::new(),
            future_outcomes: Vec::new(),
            latest_probe_round: None,
            completed_probe_rounds: 0,
            latest_applied_outcome: None,
            consumed_outcome_roots_sha256: Vec::new(),
            applied_outcome_roots_sha256: Vec::new(),
            remaining_probe_budget: None,
            latest_terminal_verdict: None,
            pending_terminal_transfer: None,
            latest_transfer_settlement: None,
            authority_ready: false,
            phase_mutation_allowed: false,
        }
    }

    fn active_request(work_path: &str) -> LawLabSandboxRequestV1 {
        let mut input = LawLabSandboxRequestInputV1 {
            executor_manifest_root_sha256: root("executor"),
            worker_sha256: root("worker"),
            candidate_root_sha256: root("candidate"),
            version_space_root_sha256: root("version-space"),
            durable_prediction_ledger_root_sha256: root("ledger"),
            probe_root_sha256: root("placeholder-probe"),
            source_tree_root_sha256: root("source-tree"),
            deterministic_seed_sha256: root("seed"),
            domain: LawLabProbeDomainV1::Filesystem,
            purpose: LawLabSandboxPurposeV1::ActiveDistinguishingProbe,
            surviving_hypothesis_count: 2,
            precommitted_prediction_count: 2,
            operations: vec![LawLabSandboxOperationV1::CopySourceFile {
                source_path: "input.txt".to_owned(),
                work_path: work_path.to_owned(),
            }],
        };
        let preliminary = LawLabSandboxRequestV1::seal(input.clone()).expect("preliminary");
        input.probe_root_sha256 = active_probe_plan_root(&preliminary).expect("probe root");
        LawLabSandboxRequestV1::seal(input).expect("request")
    }

    #[test]
    fn no_active_generation_is_not_an_eligible_law_lab_probe() {
        assert_eq!(
            classify_eligibility(false, false, 0, false, false, false),
            (
                LawLabK1EligibilityStateV1::NoEligibleLawLabProbe,
                NO_ACTIVE_CANDIDATE_BLOCKER,
            )
        );
    }

    #[test]
    fn probe_requires_ambiguity_and_durable_prediction_precommit() {
        assert_eq!(
            classify_eligibility(true, true, 1, false, false, false),
            (
                LawLabK1EligibilityStateV1::NoEligibleLawLabProbe,
                VERSION_SPACE_NOT_AMBIGUOUS_BLOCKER,
            )
        );
        assert_eq!(
            classify_eligibility(true, true, 2, false, false, false),
            (
                LawLabK1EligibilityStateV1::AwaitingProbePredictionPrecommit,
                PROBE_PREDICTION_PRECOMMIT_MISSING_BLOCKER,
            )
        );
    }

    #[test]
    fn precommitted_probe_still_requires_a_typed_execution_contract() {
        assert_eq!(
            classify_eligibility(true, true, 2, true, false, false),
            (
                LawLabK1EligibilityStateV1::AwaitingActiveProbeContract,
                ACTIVE_PROBE_CONTRACT_MISSING_BLOCKER,
            )
        );
        assert_eq!(
            classify_eligibility(true, true, 2, true, true, false),
            (
                LawLabK1EligibilityStateV1::AwaitingExecutorAttestation,
                EXECUTOR_ATTESTATION_MISSING_BLOCKER,
            )
        );
        assert_eq!(
            classify_eligibility(true, true, 2, true, true, true),
            (LawLabK1EligibilityStateV1::ReadyForSandboxExecution, "")
        );
    }

    #[test]
    fn inactive_projection_seals_an_authority_free_waiting_report() {
        let report =
            law_lab_eligibility_report(&inactive_projection(), None, None, false, 1_786_202_400)
                .expect("report");
        assert_eq!(
            report.state,
            LawLabK1EligibilityStateV1::NoEligibleLawLabProbe
        );
        assert_eq!(report.blocker, NO_ACTIVE_CANDIDATE_BLOCKER);
        assert!(!report.sandbox_execution_allowed);
        assert!(!report.authority.execution_authority_granted);
        assert_eq!(report.scheduler_ledger_revision, 91);
        report.validate().expect("valid report");

        let mut tampered = report;
        tampered.sandbox_execution_allowed = true;
        assert!(tampered.validate().is_err());
    }

    #[test]
    fn active_probe_root_precommits_typed_operations() {
        let first = active_request("copy-a.txt");
        let second = active_request("copy-b.txt");
        assert_eq!(
            first.probe_root_sha256,
            active_probe_plan_root(&first).expect("first root")
        );
        assert_ne!(first.probe_root_sha256, second.probe_root_sha256);
    }
}
