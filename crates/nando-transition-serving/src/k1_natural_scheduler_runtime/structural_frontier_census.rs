use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use nando_operator_learning::multi_source::K1NaturalCohortCandidateV1;

use super::*;

pub(super) const STRUCTURAL_FRONTIER_CENSUS_SCHEMA_V2: &str = "nando.structural-frontier-census.v2";
const STRUCTURAL_FRONTIER_SOURCE_SCHEMA_V2: &str = "nando.structural-frontier-source.v2";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StructuralFrontierCensusV2 {
    pub schema: String,
    pub report_root_sha256: String,
    pub source_root_sha256: String,
    pub evidence_epoch_root_sha256: String,
    pub fixture_exclusion_root_sha256: String,
    pub active_protocol_mode_set_root_sha256: String,
    pub contract_watermark: u64,
    pub scheduler_state: K1NaturalSchedulerRuntimeStateV1,
    pub scheduler_blocker: String,
    pub scheduler_projection_root_sha256: String,
    pub active_candidate_freeze_root_sha256: Option<String>,
    pub verdict: String,
    pub blocker: String,
    pub join: MultiSourceJoinReportV1,
    pub topology_rows: u64,
    pub frame_rows: u64,
    pub joined_rows: u64,
    pub accepted_rows: u64,
    pub natural_rows: u64,
    pub cohorts_total: u64,
    pub readiness_pass_cohorts: u64,
    pub schedulable_ready_cohorts: u64,
    pub retained_cohorts: u64,
    pub completed_cohorts_excluded: u64,
    pub capacity_excluded_cohorts: u64,
    pub consequence_type_counts: BTreeMap<K1ConsequenceTypeV1, u64>,
    pub leading_candidate_root_sha256: Option<String>,
    pub catalog: K1NaturalCohortCatalogV1,
    pub queue: K1NaturalCandidateQueueV1,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Serialize)]
struct StructuralFrontierSourceDigestV2<'a> {
    schema: &'static str,
    join: &'a MultiSourceJoinReportV1,
    evidence_epoch_root_sha256: &'a str,
    catalog_root_sha256: &'a str,
    queue_root_sha256: &'a str,
    active_protocol_mode_set_root_sha256: &'a str,
    contract_watermark: u64,
    scheduler_state: K1NaturalSchedulerRuntimeStateV1,
    scheduler_blocker: &'a str,
    scheduler_projection_root_sha256: &'a str,
    identification_root_sha256: Option<&'a str>,
    transfer_lifecycle_root_sha256: Option<&'a str>,
}

#[derive(Serialize)]
struct StructuralFrontierReportDigestV2<'a> {
    schema: &'static str,
    source_root_sha256: &'a str,
    evidence_epoch_root_sha256: &'a str,
    fixture_exclusion_root_sha256: &'a str,
    active_protocol_mode_set_root_sha256: &'a str,
    contract_watermark: u64,
    scheduler_state: K1NaturalSchedulerRuntimeStateV1,
    scheduler_blocker: &'a str,
    scheduler_projection_root_sha256: &'a str,
    active_candidate_freeze_root_sha256: Option<&'a str>,
    verdict: &'a str,
    blocker: &'a str,
    join: &'a MultiSourceJoinReportV1,
    natural_rows: u64,
    cohorts_total: u64,
    readiness_pass_cohorts: u64,
    schedulable_ready_cohorts: u64,
    retained_cohorts: u64,
    completed_cohorts_excluded: u64,
    capacity_excluded_cohorts: u64,
    consequence_type_counts: &'a BTreeMap<K1ConsequenceTypeV1, u64>,
    leading_candidate_root_sha256: Option<&'a str>,
    catalog_root_sha256: &'a str,
    queue_root_sha256: &'a str,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

pub(super) fn source_root(
    prepared: &PreparedK1TickContextV1,
    runtime: &K1NaturalSchedulerRuntimeReportV1,
) -> Result<String, String> {
    canonical_json_sha256(&StructuralFrontierSourceDigestV2 {
        schema: STRUCTURAL_FRONTIER_SOURCE_SCHEMA_V2,
        join: &prepared.join_report,
        evidence_epoch_root_sha256: &prepared.evidence_epoch_root_sha256,
        catalog_root_sha256: &prepared.catalog.catalog_root_sha256,
        queue_root_sha256: &runtime.queue.queue_root_sha256,
        active_protocol_mode_set_root_sha256: &prepared.active_protocol_mode_set_root_sha256,
        contract_watermark: prepared.contract_watermark,
        scheduler_state: runtime.state,
        scheduler_blocker: &runtime.blocker,
        scheduler_projection_root_sha256: &runtime.projection.projection_root_sha256,
        identification_root_sha256: runtime
            .identification
            .as_ref()
            .map(|report| report.report_root_sha256.as_str()),
        transfer_lifecycle_root_sha256: runtime
            .transfer_lifecycle
            .as_ref()
            .map(|report| report.report_root_sha256.as_str()),
    })
    .map_err(str::to_owned)
}

pub(super) fn build_report(
    prepared: &PreparedK1TickContextV1,
    runtime: &K1NaturalSchedulerRuntimeReportV1,
) -> Result<StructuralFrontierCensusV2, String> {
    if runtime.catalog.catalog_root_sha256 != prepared.catalog.catalog_root_sha256
        || runtime.join != prepared.join_report
    {
        return Err("structural_frontier_runtime_context_mismatch".to_owned());
    }
    let source_root_sha256 = source_root(prepared, runtime)?;
    let consequence_type_counts = consequence_counts(&prepared.catalog.candidates);
    let readiness_pass_cohorts = count_candidates(&prepared.catalog.candidates, |candidate| {
        candidate.readiness.pass
    })?;
    let schedulable_ready_cohorts = u64::try_from(
        runtime
            .queue
            .rows
            .iter()
            .filter(|row| row.score.readiness_rank == 1)
            .count(),
    )
    .map_err(|_| "structural_frontier_count".to_owned())?;
    let retained_cohorts = u64::try_from(runtime.queue.rows.len())
        .map_err(|_| "structural_frontier_count".to_owned())?;
    let cohorts_total = u64::try_from(prepared.catalog.candidates.len())
        .map_err(|_| "structural_frontier_count".to_owned())?;
    let leading_candidate_root_sha256 = leading_candidate_root(&runtime.queue);
    let (verdict, blocker) = verdict_and_blocker(&prepared.catalog, &runtime.queue)?;
    let active_candidate_freeze_root_sha256 = runtime
        .projection
        .active_candidate_freeze
        .as_ref()
        .map(|freeze| freeze.freeze_root_sha256.clone());
    let mut report = StructuralFrontierCensusV2 {
        schema: STRUCTURAL_FRONTIER_CENSUS_SCHEMA_V2.to_owned(),
        report_root_sha256: String::new(),
        source_root_sha256,
        evidence_epoch_root_sha256: prepared.evidence_epoch_root_sha256.clone(),
        fixture_exclusion_root_sha256: prepared.catalog.fixture_exclusion_root_sha256.clone(),
        active_protocol_mode_set_root_sha256: prepared.active_protocol_mode_set_root_sha256.clone(),
        contract_watermark: prepared.contract_watermark,
        scheduler_state: runtime.state,
        scheduler_blocker: runtime.blocker.clone(),
        scheduler_projection_root_sha256: runtime.projection.projection_root_sha256.clone(),
        active_candidate_freeze_root_sha256,
        verdict,
        blocker,
        join: prepared.join_report.clone(),
        topology_rows: prepared.join_report.topology_rows,
        frame_rows: prepared.join_report.completed_frames,
        joined_rows: prepared.join_report.joined_rows,
        accepted_rows: prepared.join_report.accepted_rows,
        natural_rows: prepared.catalog.natural_rows,
        cohorts_total,
        readiness_pass_cohorts,
        schedulable_ready_cohorts,
        retained_cohorts,
        completed_cohorts_excluded: runtime.queue.completed_candidates_excluded,
        capacity_excluded_cohorts: runtime.queue.capacity_excluded_candidates,
        consequence_type_counts,
        leading_candidate_root_sha256,
        catalog: prepared.catalog.clone(),
        queue: runtime.queue.clone(),
        authority_ready: false,
        phase_mutation_allowed: false,
    };
    report.report_root_sha256 = report.expected_root()?;
    report.validate()?;
    Ok(report)
}

pub(super) fn publish_report(
    root: &Path,
    report: &StructuralFrontierCensusV2,
) -> Result<(), String> {
    report.validate()?;
    let bytes = serde_json::to_vec(report)
        .map_err(|error| format!("structural_frontier_encode:{error}"))?;
    publish_report_bytes(root, &report.report_root_sha256, &bytes)?;
    let latest = fs::read(root.join("latest.json"))
        .map_err(|error| format!("structural_frontier_latest_read:{error}"))?;
    if latest != bytes {
        return Err("structural_frontier_latest_bytes_mismatch".to_owned());
    }
    let restored: StructuralFrontierCensusV2 = serde_json::from_slice(&latest)
        .map_err(|error| format!("structural_frontier_latest_decode:{error}"))?;
    restored.validate()?;
    if restored.report_root_sha256 != report.report_root_sha256 {
        return Err("structural_frontier_latest_root_mismatch".to_owned());
    }
    Ok(())
}

impl StructuralFrontierCensusV2 {
    fn validate(&self) -> Result<(), String> {
        self.catalog.validate().map_err(str::to_owned)?;
        self.queue.validate().map_err(str::to_owned)?;
        let consequence_type_counts = consequence_counts(&self.catalog.candidates);
        let readiness_pass_cohorts = count_candidates(&self.catalog.candidates, |candidate| {
            candidate.readiness.pass
        })?;
        let schedulable_ready_cohorts = u64::try_from(
            self.queue
                .rows
                .iter()
                .filter(|row| row.score.readiness_rank == 1)
                .count(),
        )
        .map_err(|_| "structural_frontier_count".to_owned())?;
        let retained_cohorts = u64::try_from(self.queue.rows.len())
            .map_err(|_| "structural_frontier_count".to_owned())?;
        let cohorts_total = u64::try_from(self.catalog.candidates.len())
            .map_err(|_| "structural_frontier_count".to_owned())?;
        let (verdict, blocker) = verdict_and_blocker(&self.catalog, &self.queue)?;
        let roots_valid = [
            self.report_root_sha256.as_str(),
            self.source_root_sha256.as_str(),
            self.evidence_epoch_root_sha256.as_str(),
            self.fixture_exclusion_root_sha256.as_str(),
            self.active_protocol_mode_set_root_sha256.as_str(),
            self.scheduler_projection_root_sha256.as_str(),
        ]
        .into_iter()
        .all(valid_nonzero_sha256);
        let optional_roots_valid = self
            .active_candidate_freeze_root_sha256
            .as_deref()
            .into_iter()
            .chain(self.leading_candidate_root_sha256.as_deref())
            .all(valid_nonzero_sha256);
        if self.schema != STRUCTURAL_FRONTIER_CENSUS_SCHEMA_V2
            || !roots_valid
            || !optional_roots_valid
            || self.catalog.evidence_epoch_root_sha256 != self.evidence_epoch_root_sha256
            || self.catalog.fixture_exclusion_root_sha256 != self.fixture_exclusion_root_sha256
            || self.queue.catalog_root_sha256 != self.catalog.catalog_root_sha256
            || self.queue.catalog_candidates != cohorts_total
            || self.join.topology_rows != self.topology_rows
            || self.join.completed_frames != self.frame_rows
            || self.join.joined_rows != self.joined_rows
            || self.join.accepted_rows != self.accepted_rows
            || self.catalog.natural_rows != self.natural_rows
            || cohorts_total != self.cohorts_total
            || readiness_pass_cohorts != self.readiness_pass_cohorts
            || schedulable_ready_cohorts != self.schedulable_ready_cohorts
            || retained_cohorts != self.retained_cohorts
            || self.queue.completed_candidates_excluded != self.completed_cohorts_excluded
            || self.queue.capacity_excluded_candidates != self.capacity_excluded_cohorts
            || consequence_type_counts != self.consequence_type_counts
            || leading_candidate_root(&self.queue) != self.leading_candidate_root_sha256
            || verdict != self.verdict
            || blocker != self.blocker
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.report_root_sha256 != self.expected_root()?
        {
            return Err("structural_frontier_report_invalid".to_owned());
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, String> {
        canonical_json_sha256(&StructuralFrontierReportDigestV2 {
            schema: STRUCTURAL_FRONTIER_CENSUS_SCHEMA_V2,
            source_root_sha256: &self.source_root_sha256,
            evidence_epoch_root_sha256: &self.evidence_epoch_root_sha256,
            fixture_exclusion_root_sha256: &self.fixture_exclusion_root_sha256,
            active_protocol_mode_set_root_sha256: &self.active_protocol_mode_set_root_sha256,
            contract_watermark: self.contract_watermark,
            scheduler_state: self.scheduler_state,
            scheduler_blocker: &self.scheduler_blocker,
            scheduler_projection_root_sha256: &self.scheduler_projection_root_sha256,
            active_candidate_freeze_root_sha256: self
                .active_candidate_freeze_root_sha256
                .as_deref(),
            verdict: &self.verdict,
            blocker: &self.blocker,
            join: &self.join,
            natural_rows: self.natural_rows,
            cohorts_total: self.cohorts_total,
            readiness_pass_cohorts: self.readiness_pass_cohorts,
            schedulable_ready_cohorts: self.schedulable_ready_cohorts,
            retained_cohorts: self.retained_cohorts,
            completed_cohorts_excluded: self.completed_cohorts_excluded,
            capacity_excluded_cohorts: self.capacity_excluded_cohorts,
            consequence_type_counts: &self.consequence_type_counts,
            leading_candidate_root_sha256: self.leading_candidate_root_sha256.as_deref(),
            catalog_root_sha256: &self.catalog.catalog_root_sha256,
            queue_root_sha256: &self.queue.queue_root_sha256,
            authority_ready: false,
            phase_mutation_allowed: false,
        })
        .map_err(str::to_owned)
    }
}

fn consequence_counts(
    candidates: &[K1NaturalCohortCandidateV1],
) -> BTreeMap<K1ConsequenceTypeV1, u64> {
    let mut counts = BTreeMap::new();
    for candidate in candidates {
        let count = counts.entry(candidate.consequence_type).or_insert(0u64);
        *count = count.saturating_add(1);
    }
    counts
}

fn count_candidates(
    candidates: &[K1NaturalCohortCandidateV1],
    predicate: impl Fn(&K1NaturalCohortCandidateV1) -> bool,
) -> Result<u64, String> {
    u64::try_from(
        candidates
            .iter()
            .filter(|candidate| predicate(candidate))
            .count(),
    )
    .map_err(|_| "structural_frontier_count".to_owned())
}

fn leading_candidate_root(queue: &K1NaturalCandidateQueueV1) -> Option<String> {
    queue
        .first_readiness_pass()
        .or_else(|| queue.rows.first())
        .map(|row| row.candidate_root_sha256.clone())
}

fn verdict_and_blocker(
    catalog: &K1NaturalCohortCatalogV1,
    queue: &K1NaturalCandidateQueueV1,
) -> Result<(String, String), String> {
    if queue.first_readiness_pass().is_some() {
        return Ok(("ready_cohort".to_owned(), String::new()));
    }
    let blocker = if catalog.natural_rows == 0 {
        "natural_live_rows_missing".to_owned()
    } else if catalog.candidates.is_empty() {
        "candidate_generation_empty".to_owned()
    } else if queue.rows.is_empty() {
        "all_exact_cohort_identities_excluded".to_owned()
    } else {
        let leading_root = leading_candidate_root(queue)
            .ok_or_else(|| "structural_frontier_leading_candidate_missing".to_owned())?;
        let candidate = catalog
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_root_sha256 == leading_root)
            .ok_or_else(|| "structural_frontier_queue_candidate_missing".to_owned())?;
        if candidate.readiness.pass {
            "readiness_pass_candidate_outside_recency_window".to_owned()
        } else {
            candidate.readiness.blocker.clone()
        }
    };
    Ok(("collecting".to_owned(), blocker))
}

fn publish_report_bytes(root: &Path, report_root: &str, bytes: &[u8]) -> Result<(), String> {
    if !valid_nonzero_sha256(report_root) {
        return Err("structural_frontier_publish_root_invalid".to_owned());
    }
    let reports = root.join("reports");
    fs::create_dir_all(&reports)
        .map_err(|error| format!("structural_frontier_reports_create:{error}"))?;
    fs::File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("structural_frontier_root_sync:{error}"))?;
    let immutable = reports.join(format!("{report_root}.json"));
    if immutable.exists() {
        let existing = fs::read(&immutable)
            .map_err(|error| format!("structural_frontier_immutable_read:{error}"))?;
        if existing != bytes {
            return Err("structural_frontier_immutable_mismatch".to_owned());
        }
    } else {
        crate::write_bytes_atomic(&immutable, bytes, "structural-frontier-report")?;
    }
    let latest = root.join("latest.json");
    if fs::read(&latest).ok().as_deref() != Some(bytes) {
        crate::write_bytes_atomic(&latest, bytes, "structural-frontier-latest")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn immutable_report_and_latest_pointer_are_byte_identical() {
        let report_root = canonical_json_sha256(&("frontier-test", 1u64)).expect("root");
        let root = std::env::temp_dir().join(format!(
            "nando-structural-frontier-census-{}-{report_root}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let bytes = br#"{"schema":"frontier-test"}"#;

        publish_report_bytes(&root, &report_root, bytes).expect("first publish");
        publish_report_bytes(&root, &report_root, bytes).expect("idempotent publish");

        assert_eq!(fs::read(root.join("latest.json")).expect("latest"), bytes);
        assert_eq!(
            fs::read(root.join("reports").join(format!("{report_root}.json"))).expect("immutable"),
            bytes
        );
        assert_eq!(
            publish_report_bytes(&root, &report_root, b"different"),
            Err("structural_frontier_immutable_mismatch".to_owned())
        );
        fs::remove_dir_all(root).expect("cleanup test root");
    }
}
