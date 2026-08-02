use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn runtime_report(
    generated_at_unix: u64,
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
}
