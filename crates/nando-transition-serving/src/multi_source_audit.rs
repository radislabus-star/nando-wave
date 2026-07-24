//! Read-only STOP-MS0 adapters for live evidence stores.
//!
//! The learning crate owns accounting and shape semantics. This module only
//! decodes owner-local stores and never mutates checkpoints or authority.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;

use nando_operator_kernel::{AtomSource, RelationAtom, RelationFrame};
use nando_operator_learning::multi_source::{
    CompletedEffectFormV1, CoverageOpportunitySnapshotV1, MultiSourceEvidenceAuditV1,
    MultiSourceJoinLedgerV1, MultiSourceJoinReportV1, PreActionShapeClassV1,
    RelationEvidenceAuditV1, build_coverage_opportunity_snapshot_v1,
    build_multi_source_evidence_audit_v1, factor_multi_source_row_v1,
};
use nando_operator_learning::opportunity::ReducibilityClass;
use sha2::{Digest, Sha256};

use crate::request_learning::RequestLearningIndex;

pub const MULTI_SOURCE_DISCOVERY_AUDIT_SCHEMA_V2: &str = "nando.multi-source-discovery-audit.v2";

#[derive(Clone, Debug, serde::Serialize)]
pub struct MultiSourceDiscoveryAuditV2 {
    pub schema: String,
    pub evidence: MultiSourceEvidenceAuditV1,
    pub join: MultiSourceJoinReportV1,
    pub factorized_rows: u64,
    pub t1_eligibility: T1EligibilityAuditV1,
    pub opportunity: CoverageOpportunitySnapshotV1,
    pub restart_byte_parity: bool,
    pub authority_ready: bool,
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct T1EligibilityAuditV1 {
    pub shape_and_effect_rows: u64,
    pub extraction_complete_rows: u64,
    pub witness_complete_rows: u64,
    pub eligible_rows: u64,
}

type RelationDataV1 = (
    BTreeMap<String, RelationEvidenceAuditV1>,
    Vec<RelationFrame>,
    u64,
    u64,
    String,
);

pub fn run_multi_source_discovery_audit_v2(
    opportunity_checkpoint: &Path,
    request_learning_checkpoint: &Path,
    relation_frames: &Path,
) -> Result<MultiSourceDiscoveryAuditV2, String> {
    let opportunity_bytes = fs::read(opportunity_checkpoint).map_err(|error| {
        format!(
            "multi_source_opportunity_checkpoint_read:{}:{error}",
            opportunity_checkpoint.display()
        )
    })?;
    let opportunities = nando_response_actor::read_opportunity_audit_rows_from_checkpoint_bytes_v1(
        &opportunity_bytes,
    )?;
    let retained_frames =
        nando_response_actor::read_retained_relation_frames_from_checkpoint_bytes_v1(
            &opportunity_bytes,
        )?;
    let request_bytes = fs::read(request_learning_checkpoint).map_err(|error| {
        format!(
            "multi_source_request_checkpoint_read:{}:{error}",
            request_learning_checkpoint.display()
        )
    })?;
    let (request_index, _) = RequestLearningIndex::from_checkpoint_cbor(&request_bytes)?;
    let request_snapshot = request_index.audit_snapshot_v1().map_err(str::to_owned)?;
    let mut relevant = opportunities
        .iter()
        .filter(|row| {
            row.authority_observed && row.class == ReducibilityClass::UnexploredMultiSource
        })
        .map(|row| row.intent_sha256.clone())
        .collect::<BTreeSet<_>>();
    relevant.extend(
        request_snapshot
            .topologies
            .iter()
            .map(|row| row.structure.turn_intent_id_sha256.clone()),
    );
    let (relations, historical_frames, rows_scanned, parse_errors, relation_sha256) =
        read_relation_data(relation_frames, &relevant)?;
    let frames = merge_relevant_frames(retained_frames, historical_frames, &relevant);
    let evidence = build_multi_source_evidence_audit_v1(
        opportunities,
        request_snapshot.clone(),
        relations,
        sha256_bytes(&opportunity_bytes),
        sha256_bytes(&request_bytes),
        relation_sha256.clone(),
        rows_scanned,
        parse_errors,
    );
    let join_ledger = MultiSourceJoinLedgerV1::build(&request_snapshot.topologies, &frames);
    let factorized = join_ledger
        .rows()
        .iter()
        .map(factor_multi_source_row_v1)
        .collect::<Vec<_>>();
    let t1_eligibility = join_ledger.rows().iter().zip(&factorized).fold(
        T1EligibilityAuditV1::default(),
        |mut audit, (joined, row)| {
            let shape_and_effect = matches!(
                row.pre_action_shape,
                PreActionShapeClassV1::SingleRoleProjection
                    | PreActionShapeClassV1::OneOutputManyScalarRoles
                    | PreActionShapeClassV1::ManyOutputsLatestRelevantRole
            ) && row.completed_effect
                == CompletedEffectFormV1::SingleRoleProjection;
            if shape_and_effect {
                audit.shape_and_effect_rows = audit.shape_and_effect_rows.saturating_add(1);
            }
            let extraction_complete = shape_and_effect
                && matches!(
                    joined.topology.extraction_status,
                    nando_operator_kernel::MultiSourceExtractionStatusV1::Complete
                );
            if extraction_complete {
                audit.extraction_complete_rows = audit.extraction_complete_rows.saturating_add(1);
            }
            let witness_complete = extraction_complete
                && joined.topology.role_witnesses.len() == joined.topology.roles.len();
            if witness_complete {
                audit.witness_complete_rows = audit.witness_complete_rows.saturating_add(1);
                audit.eligible_rows = audit.eligible_rows.saturating_add(1);
            }
            audit
        },
    );
    let evidence_epoch_sha256 = sha256_bytes(
        &serde_json::to_vec(&(
            evidence.opportunity_checkpoint_sha256.as_str(),
            evidence.request_learning_checkpoint_sha256.as_str(),
            relation_sha256.as_str(),
        ))
        .map_err(|error| format!("multi_source_epoch_encode:{error}"))?,
    );
    let opportunity = build_coverage_opportunity_snapshot_v1(
        &factorized,
        &BTreeSet::new(),
        evidence_epoch_sha256,
    );
    let report = MultiSourceDiscoveryAuditV2 {
        schema: MULTI_SOURCE_DISCOVERY_AUDIT_SCHEMA_V2.to_owned(),
        evidence,
        join: join_ledger.report(),
        factorized_rows: u64::try_from(factorized.len()).unwrap_or(u64::MAX),
        t1_eligibility,
        opportunity,
        restart_byte_parity: true,
        authority_ready: false,
    };
    let first = serde_json::to_vec(&report)
        .map_err(|error| format!("multi_source_report_encode:{error}"))?;
    let second = serde_json::to_vec(&report)
        .map_err(|error| format!("multi_source_report_encode:{error}"))?;
    if first != second {
        return Err("multi_source_report_restart_parity".to_owned());
    }
    Ok(report)
}

fn merge_relevant_frames(
    retained: Vec<RelationFrame>,
    historical: Vec<RelationFrame>,
    relevant: &BTreeSet<String>,
) -> Vec<RelationFrame> {
    let mut frames = BTreeMap::<String, RelationFrame>::new();
    for frame in historical.into_iter().chain(retained) {
        if relevant.contains(&frame.client_intent_id_sha256) {
            frames.insert(frame.frame_id_sha256.clone(), frame);
        }
    }
    let mut frames = frames.into_values().collect::<Vec<_>>();
    frames.sort_by(|left, right| {
        left.observed_at_unix_nanos
            .cmp(&right.observed_at_unix_nanos)
            .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
    });
    frames
}

pub fn run_multi_source_evidence_audit_v1(
    opportunity_checkpoint: &Path,
    request_learning_checkpoint: &Path,
    relation_frames: &Path,
) -> Result<MultiSourceEvidenceAuditV1, String> {
    let opportunity_bytes = fs::read(opportunity_checkpoint).map_err(|error| {
        format!(
            "multi_source_opportunity_checkpoint_read:{}:{error}",
            opportunity_checkpoint.display()
        )
    })?;
    let opportunities = nando_response_actor::read_opportunity_audit_rows_from_checkpoint_bytes_v1(
        &opportunity_bytes,
    )?;
    let request_bytes = fs::read(request_learning_checkpoint).map_err(|error| {
        format!(
            "multi_source_request_checkpoint_read:{}:{error}",
            request_learning_checkpoint.display()
        )
    })?;
    let (request_index, _) = RequestLearningIndex::from_checkpoint_cbor(&request_bytes)?;
    let request_snapshot = request_index.audit_snapshot_v1().map_err(str::to_owned)?;
    let relevant = opportunities
        .iter()
        .filter(|row| {
            row.authority_observed && row.class == ReducibilityClass::UnexploredMultiSource
        })
        .map(|row| row.intent_sha256.clone())
        .collect::<BTreeSet<_>>();
    let (relations, _, rows_scanned, parse_errors, relation_sha256) =
        read_relation_data(relation_frames, &relevant)?;
    Ok(build_multi_source_evidence_audit_v1(
        opportunities,
        request_snapshot,
        relations,
        sha256_bytes(&opportunity_bytes),
        sha256_bytes(&request_bytes),
        relation_sha256,
        rows_scanned,
        parse_errors,
    ))
}

fn read_relation_data(path: &Path, relevant: &BTreeSet<String>) -> Result<RelationDataV1, String> {
    let file = File::open(path).map_err(|error| {
        format!(
            "multi_source_relation_frames_open:{}:{error}",
            path.display()
        )
    })?;
    let mut summaries = BTreeMap::<String, RelationEvidenceAuditV1>::new();
    let mut relevant_frames = Vec::new();
    let mut rows = 0_u64;
    let mut parse_errors = 0_u64;
    let mut hasher = Sha256::new();
    let mut reader = BufReader::new(file);
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        let read = reader
            .read_until(b'\n', &mut bytes)
            .map_err(|error| format!("multi_source_relation_frames_read:{error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&bytes);
        rows = rows.saturating_add(1);
        let frame = match serde_json::from_slice::<RelationFrame>(&bytes) {
            Ok(frame) => frame,
            Err(_) => {
                parse_errors = parse_errors.saturating_add(1);
                continue;
            }
        };
        if !relevant.contains(&frame.client_intent_id_sha256) {
            continue;
        }
        let summary = summaries
            .entry(frame.client_intent_id_sha256.clone())
            .or_default();
        summary.frames = summary.frames.saturating_add(1);
        match frame.verifier_label {
            Some(true) => summary.positive = summary.positive.saturating_add(1),
            Some(false) => summary.negative = summary.negative.saturating_add(1),
            None => summary.unlabeled = summary.unlabeled.saturating_add(1),
        }
        collect_observation_roles(summary, frame.atoms.clone());
        relevant_frames.push(frame);
    }
    Ok((
        summaries,
        relevant_frames,
        rows,
        parse_errors,
        format!("{:x}", hasher.finalize()),
    ))
}

fn collect_observation_roles(summary: &mut RelationEvidenceAuditV1, atoms: Vec<RelationAtom>) {
    for atom in atoms {
        if let RelationAtom::TypedSlot {
            slot_id,
            value_type,
            source: AtomSource::Observation,
            ..
        } = atom
        {
            summary
                .observation_roles
                .insert((slot_id, format!("{value_type:?}")));
        }
    }
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
