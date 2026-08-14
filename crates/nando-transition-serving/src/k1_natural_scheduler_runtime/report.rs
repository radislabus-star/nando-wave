use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn runtime_report(
    generated_at_unix: u64,
    lane: K1SchedulerLaneV1,
    state: K1NaturalSchedulerRuntimeStateV1,
    blocker: String,
    projection: K1SchedulerProjectionV1,
    join: MultiSourceJoinReportV1,
    catalog: K1NaturalCohortCatalogV1,
    queue: K1NaturalCandidateQueueV1,
    identification: Option<MultiSourceT1IdentificationV3>,
    frozen_evidence_rows: u64,
    future_eligible_rows: u64,
) -> Result<K1NaturalSchedulerRuntimeReportV1, String> {
    let mut report = K1NaturalSchedulerRuntimeReportV1 {
        schema: K1_RUNTIME_REPORT_SCHEMA_V1.to_owned(),
        report_root_sha256: String::new(),
        generated_at_unix,
        lane,
        state,
        blocker,
        projection,
        join,
        catalog,
        queue,
        identification,
        transfer_lifecycle: None,
        frozen_evidence_rows,
        future_eligible_rows,
        exact_wake_status: None,
        authority_ready: false,
        phase_mutation_allowed: false,
    };
    report.report_root_sha256 = report.expected_root()?;
    report.validate()?;
    Ok(report)
}

impl K1NaturalSchedulerRuntimeReportV1 {
    fn expected_root(&self) -> Result<String, String> {
        canonical_json_sha256(&RuntimeReportDigestV1 {
            schema: K1_RUNTIME_REPORT_SCHEMA_V1,
            generated_at_unix: self.generated_at_unix,
            lane: self.lane,
            state: self.state,
            blocker: &self.blocker,
            projection_root_sha256: &self.projection.projection_root_sha256,
            join: &self.join,
            catalog_root_sha256: &self.catalog.catalog_root_sha256,
            queue_root_sha256: &self.queue.queue_root_sha256,
            identification_root_sha256: self
                .identification
                .as_ref()
                .map(|report| report.report_root_sha256.as_str()),
            transfer_lifecycle_root_sha256: self
                .transfer_lifecycle
                .as_ref()
                .map(|report| report.report_root_sha256.as_str()),
            frozen_evidence_rows: self.frozen_evidence_rows,
            future_eligible_rows: self.future_eligible_rows,
            exact_wake_status_root_sha256: self
                .exact_wake_status
                .as_ref()
                .map(|status| status.status_root_sha256.as_str()),
            authority_ready: false,
            phase_mutation_allowed: false,
        })
        .map_err(str::to_owned)
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != K1_RUNTIME_REPORT_SCHEMA_V1
            || self.generated_at_unix == 0
            || self.authority_ready
            || self.phase_mutation_allowed
            || self
                .exact_wake_status
                .as_ref()
                .is_some_and(|status| status.validate().is_err())
            || self
                .identification
                .as_ref()
                .is_some_and(|report| !report.validate())
            || self
                .transfer_lifecycle
                .as_ref()
                .is_some_and(|report| !valid_nonzero_sha256(&report.report_root_sha256))
            || self.report_root_sha256 != self.expected_root()?
        {
            return Err("k1_runtime_report_invalid".to_owned());
        }
        Ok(())
    }

    pub(super) fn attach_transfer_lifecycle(
        &mut self,
        lifecycle: K1TransferLifecycleReportV1,
    ) -> Result<(), String> {
        self.blocker = lifecycle.blocker.clone();
        self.transfer_lifecycle = Some(lifecycle);
        self.report_root_sha256 = self.expected_root()?;
        self.validate()
    }

    pub(super) fn attach_exact_wake_status(
        &mut self,
        status: crate::k1_natural_scheduler::K1ExactWakeStatusV1,
    ) -> Result<(), String> {
        status.validate()?;
        let replace_runtime_blocker = match status.decision {
            crate::k1_natural_scheduler::K1ExactWakeDecisionV1::WriterInactive => {
                if self.state == K1NaturalSchedulerRuntimeStateV1::WaitingForEvidence {
                    self.state = K1NaturalSchedulerRuntimeStateV1::WriterInactive;
                    true
                } else {
                    false
                }
            }
            crate::k1_natural_scheduler::K1ExactWakeDecisionV1::WaitingForEvidence => {
                self.state = K1NaturalSchedulerRuntimeStateV1::WaitingForEvidence;
                true
            }
            crate::k1_natural_scheduler::K1ExactWakeDecisionV1::WaitingForNovelEvidence => {
                self.state = K1NaturalSchedulerRuntimeStateV1::WaitingForNovelEvidence;
                true
            }
            crate::k1_natural_scheduler::K1ExactWakeDecisionV1::ResearchBudgetCooldown => {
                self.state = K1NaturalSchedulerRuntimeStateV1::ResearchBudgetCooldown;
                true
            }
            crate::k1_natural_scheduler::K1ExactWakeDecisionV1::CandidateFrozen
            | crate::k1_natural_scheduler::K1ExactWakeDecisionV1::ActiveGeneration => false,
            crate::k1_natural_scheduler::K1ExactWakeDecisionV1::K1VocabularyOpen => {
                self.state = K1NaturalSchedulerRuntimeStateV1::K1VocabularyOpen;
                true
            }
        };
        if replace_runtime_blocker {
            self.blocker = status.blocker.clone();
        }
        self.exact_wake_status = Some(status);
        self.report_root_sha256 = self.expected_root()?;
        self.validate()
    }
}
