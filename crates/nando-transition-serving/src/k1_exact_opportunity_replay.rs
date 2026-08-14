use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use nando_operator_learning::multi_source::{
    EvidenceSourceSnapshotV1, K1DeficitSnapshotV1, K1NaturalCandidateQueueV1,
    MultiSourceT1IdentificationStateV1,
};
use serde::{Deserialize, Serialize};

use crate::k1_natural_scheduler::journal::restore_anchored_scheduler_for;
use crate::k1_natural_scheduler::projection::{exact_attempt_index_for, projection_for};
use crate::k1_natural_scheduler::{K1SchedulerLaneV1, K1SchedulerProjectionV1};
use crate::operator_certification::{
    CertificationAuthorityConfigV1, K1ExactAuthoritySourceConfigV1, restore_anchored_ledger,
};

const REPLAY_SCHEMA_V1: &str = "nando.k1-exact-opportunity-replay.v1";
const ATTEMPT_INDEX_MEASUREMENT_SCHEMA_V1: &str = "nando.k1-exact-attempt-index-measurement.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K1ExactReplayOutcomeClassV1 {
    DeterministicPreFuture,
    FutureContingent,
    OperationalRetryable,
    Unclassified,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1ExactOpportunityOutcomeV1 {
    pub opportunity_root_sha256: String,
    pub identifier_report_root_sha256: String,
    pub disposition_set_root_sha256: String,
    pub outcome: K1ExactReplayOutcomeClassV1,
    pub blocker: String,
    pub support_rows: u64,
    pub seed_programs: u64,
    pub accepted_programs: u64,
    pub semantic_classes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1ExactOpportunityReplayV1 {
    pub schema: String,
    pub replay_root_sha256: String,
    pub scheduler_revision: u64,
    pub scheduler_ledger_root_sha256: String,
    pub epistemic_registry_revision: u64,
    pub epistemic_registry_root_sha256: String,
    pub deficit_snapshot_root_sha256: String,
    pub topology_rows: u64,
    pub topology_root_sha256: String,
    pub frame_rows: u64,
    pub frame_root_sha256: String,
    pub collection_checkpoint_root_sha256: String,
    pub evidence_source_snapshot_root_sha256: String,
    pub catalog_root_sha256: String,
    pub catalog_candidates: u64,
    pub queue_root_sha256: String,
    pub queue_rows: u64,
    pub readiness_pass_rows: u64,
    pub exact_roots_considered: u64,
    pub exact_unseen_opportunities: u64,
    pub exact_attempted_deterministic_roots: u64,
    pub legacy_unbound_terminals: u64,
    pub deterministic_pre_future: u64,
    pub future_contingent: u64,
    pub operational_retryable: u64,
    pub unclassified: u64,
    pub blocker_histogram: BTreeMap<String, u64>,
    pub outcomes: Vec<K1ExactOpportunityOutcomeV1>,
    pub queue_bytes: u64,
    pub wire_logical_bytes: u64,
    pub wire_compressed_bytes: u64,
    pub wire_outer_bytes: u64,
    pub archive_object_bytes: u64,
    pub authority_events_appended: u64,
    pub false_accepts: u64,
    pub parity_failures: u64,
    pub law_2_proved: bool,
    pub k1_laws: u64,
    pub k1_minimum_laws: u64,
    pub quality: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1ExactAttemptIndexMeasurementV1 {
    pub schema: String,
    pub scheduler_revision: u64,
    pub scheduler_ledger_root_sha256: String,
    pub scheduler_restore_ns: u64,
    pub attempt_index_projection_ns: u64,
    pub deterministic_attempts: u64,
    pub legacy_unbound_terminals: u64,
    pub attempt_index_root_sha256: String,
    pub scheduler_journal_bytes: u64,
    pub scheduler_cache_bytes: u64,
    pub authority_events_appended: u64,
}

impl K1ExactOpportunityReplayV1 {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn seal(
        scheduler: &K1SchedulerProjectionV1,
        deficit: &K1DeficitSnapshotV1,
        source_heads: &crate::k1_natural_scheduler_runtime::ExactDurableSourceHeadsV1,
        source_snapshot: &EvidenceSourceSnapshotV1,
        queue: &K1NaturalCandidateQueueV1,
        catalog_candidates: u64,
        exact_roots_considered: u64,
        outcomes: Vec<K1ExactOpportunityOutcomeV1>,
        queue_bytes: u64,
        wire_logical_bytes: u64,
        wire_compressed_bytes: u64,
        wire_outer_bytes: u64,
        archive_object_bytes: u64,
    ) -> Result<Self, String> {
        let mut blocker_histogram = BTreeMap::new();
        let mut deterministic_pre_future = 0_u64;
        let mut future_contingent = 0_u64;
        let mut operational_retryable = 0_u64;
        let mut unclassified = 0_u64;
        for outcome in &outcomes {
            *blocker_histogram
                .entry(outcome.blocker.clone())
                .or_insert(0) += 1;
            match outcome.outcome {
                K1ExactReplayOutcomeClassV1::DeterministicPreFuture => {
                    deterministic_pre_future = deterministic_pre_future.saturating_add(1);
                }
                K1ExactReplayOutcomeClassV1::FutureContingent => {
                    future_contingent = future_contingent.saturating_add(1);
                }
                K1ExactReplayOutcomeClassV1::OperationalRetryable => {
                    operational_retryable = operational_retryable.saturating_add(1);
                }
                K1ExactReplayOutcomeClassV1::Unclassified => {
                    unclassified = unclassified.saturating_add(1);
                }
            }
        }
        let readiness_pass_rows = u64::try_from(
            queue
                .rows
                .iter()
                .filter(|row| row.score.readiness_rank == 1)
                .count(),
        )
        .map_err(|_| "k1_exact_replay_readiness_count".to_owned())?;
        let mut replay = Self {
            schema: REPLAY_SCHEMA_V1.to_owned(),
            replay_root_sha256: String::new(),
            scheduler_revision: scheduler.ledger_revision,
            scheduler_ledger_root_sha256: scheduler.ledger_root_sha256.clone(),
            epistemic_registry_revision: deficit.epistemic_registry_revision,
            epistemic_registry_root_sha256: deficit.epistemic_registry_root_sha256.clone(),
            deficit_snapshot_root_sha256: deficit.snapshot_root_sha256.clone(),
            topology_rows: source_heads.topology_rows,
            topology_root_sha256: source_heads.topology_root_sha256.clone(),
            frame_rows: source_heads.frame_rows,
            frame_root_sha256: source_heads.frame_root_sha256.clone(),
            collection_checkpoint_root_sha256: source_heads
                .collection_checkpoint_root_sha256
                .clone(),
            evidence_source_snapshot_root_sha256: source_snapshot.snapshot_root_sha256.clone(),
            catalog_root_sha256: queue.catalog_root_sha256.clone(),
            catalog_candidates,
            queue_root_sha256: queue.queue_root_sha256.clone(),
            queue_rows: u64::try_from(queue.rows.len())
                .map_err(|_| "k1_exact_replay_queue_count".to_owned())?,
            readiness_pass_rows,
            exact_roots_considered,
            exact_unseen_opportunities: queue.exact_unseen_opportunities,
            exact_attempted_deterministic_roots: queue.exact_attempted_deterministic_roots,
            legacy_unbound_terminals: queue.legacy_unbound_terminals,
            deterministic_pre_future,
            future_contingent,
            operational_retryable,
            unclassified,
            blocker_histogram,
            outcomes,
            queue_bytes,
            wire_logical_bytes,
            wire_compressed_bytes,
            wire_outer_bytes,
            archive_object_bytes,
            authority_events_appended: 0,
            false_accepts: 0,
            parity_failures: 0,
            law_2_proved: false,
            k1_laws: deficit.law_certificates,
            k1_minimum_laws: deficit.minimum_law_certificates,
            quality: "UNKNOWN".to_owned(),
        };
        replay.replay_root_sha256 =
            nando_operator_kernel::canonical_json_sha256(&replay).map_err(str::to_owned)?;
        Ok(replay)
    }
}

pub fn replay_frozen_snapshot(snapshot_root: &Path) -> Result<K1ExactOpportunityReplayV1, String> {
    let config = frozen_snapshot_config(snapshot_root)?;
    let scheduler = restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Epistemic)?;
    let scheduler_projection = projection_for(&scheduler)?;
    let exact_attempt_index = exact_attempt_index_for(&scheduler)?;
    let certification = restore_anchored_ledger(&config)?;
    let deficit = crate::k1_natural_scheduler::current_deficit_snapshot(&config)?;
    let active_protocols = crate::multi_source_live::known_epistemic_protocol_mode_roots(
        &config.response_registry_path,
        &certification,
    )?;
    crate::k1_natural_scheduler_runtime::replay_exact_opportunities_v1(
        &config,
        &deficit,
        &exact_attempt_index,
        &active_protocols,
        &scheduler_projection,
    )
}

pub fn measure_frozen_attempt_index(
    snapshot_root: &Path,
) -> Result<K1ExactAttemptIndexMeasurementV1, String> {
    let config = frozen_snapshot_config(snapshot_root)?;
    let restore_started = Instant::now();
    let scheduler = restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Epistemic)?;
    let scheduler_restore_ns = u64::try_from(restore_started.elapsed().as_nanos())
        .map_err(|_| "k1_exact_replay_restore_duration".to_owned())?;
    let projection_started = Instant::now();
    let index = exact_attempt_index_for(&scheduler)?;
    let attempt_index_projection_ns = u64::try_from(projection_started.elapsed().as_nanos())
        .map_err(|_| "k1_exact_replay_attempt_index_duration".to_owned())?;
    Ok(K1ExactAttemptIndexMeasurementV1 {
        schema: ATTEMPT_INDEX_MEASUREMENT_SCHEMA_V1.to_owned(),
        scheduler_revision: scheduler.revision,
        scheduler_ledger_root_sha256: scheduler.ledger_root_sha256,
        scheduler_restore_ns,
        attempt_index_projection_ns,
        deterministic_attempts: u64::try_from(index.deterministic_attempts.len())
            .map_err(|_| "k1_exact_replay_attempt_index_count".to_owned())?,
        legacy_unbound_terminals: index.legacy_unbound_terminals,
        attempt_index_root_sha256: index.index_root_sha256,
        scheduler_journal_bytes: regular_file_bytes(
            &config.root.join("k1-epistemic-scheduler-journal-v1"),
        )?,
        scheduler_cache_bytes: regular_file_bytes(
            &config.root.join("k1-epistemic-scheduler-ledger-v1.json"),
        )?,
        authority_events_appended: 0,
    })
}

fn frozen_snapshot_config(snapshot_root: &Path) -> Result<CertificationAuthorityConfigV1, String> {
    let snapshot_root = snapshot_root
        .canonicalize()
        .map_err(|error| format!("k1_exact_replay_snapshot_root:{error}"))?;
    if !snapshot_root.is_dir() {
        return Err("k1_exact_replay_snapshot_not_directory".to_owned());
    }
    let state = snapshot_root.join("state");
    let ms4 = state.join("multi-source-live-v2/ms4-closed-loop-v1");
    let anchor = snapshot_root.join("anchor");
    let public_key = snapshot_root.join("etc/authority-ed25519.pub");
    Ok(CertificationAuthorityConfigV1 {
        root: ms4,
        cleanup_receipts_path: snapshot_root.join("read-only-unused-cleanup"),
        anchor_path: anchor.join("operator-certification-anchor-v1.json"),
        authority_socket_path: snapshot_root.join("no-authority-socket"),
        authority_public_key_path: public_key.clone(),
        cleanup_public_key_path: public_key,
        response_registry_path: state.join("response-registry.json"),
        runtime_revocations_path: snapshot_root.join("read-only-unused-revocations.json"),
        k1_exact_sources: Some(K1ExactAuthoritySourceConfigV1 {
            topology_archive_path: state
                .join("multi-source-live-v2/pre-action-topology-archive-v1"),
            frame_archive_path: state.join("multi-source-live-v2/relation-frame-archive-v1"),
            collection_checkpoint_path: state
                .join("online-collection-program-pools-v37.checkpoint"),
            artifact_archive_path: snapshot_root.join("read-only-unused-artifact-archive"),
            scheduler_policy_path: snapshot_root.join("read-only-unused-policy.json"),
        }),
    })
}

fn regular_file_bytes(path: &Path) -> Result<u64, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("k1_exact_replay_measure_metadata:{error}"))?;
    if metadata.file_type().is_symlink() {
        return Err("k1_exact_replay_measure_symlink_forbidden".to_owned());
    }
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Err("k1_exact_replay_measure_path_type_invalid".to_owned());
    }
    let mut bytes = 0_u64;
    for entry in
        fs::read_dir(path).map_err(|error| format!("k1_exact_replay_measure_read_dir:{error}"))?
    {
        let entry = entry.map_err(|error| format!("k1_exact_replay_measure_entry:{error}"))?;
        bytes = bytes
            .checked_add(regular_file_bytes(&entry.path())?)
            .ok_or_else(|| "k1_exact_replay_measure_size_overflow".to_owned())?;
    }
    Ok(bytes)
}

pub(crate) fn classify_replay_outcome(
    state: MultiSourceT1IdentificationStateV1,
    blocker: &str,
    has_internal_unclassified: bool,
) -> K1ExactReplayOutcomeClassV1 {
    if has_internal_unclassified {
        K1ExactReplayOutcomeClassV1::Unclassified
    } else if nando_operator_learning::multi_source::deterministic_initial_blocker_v1(blocker) {
        K1ExactReplayOutcomeClassV1::DeterministicPreFuture
    } else if matches!(
        state,
        MultiSourceT1IdentificationStateV1::Ambiguous
            | MultiSourceT1IdentificationStateV1::FrozenAwaitingIndependentFuture
            | MultiSourceT1IdentificationStateV1::FutureContradiction
            | MultiSourceT1IdentificationStateV1::TransferReady
    ) {
        K1ExactReplayOutcomeClassV1::FutureContingent
    } else {
        K1ExactReplayOutcomeClassV1::OperationalRetryable
    }
}

pub(crate) fn unclassified_replay_outcome(
    opportunity_root_sha256: String,
    error: &str,
    support_rows: u64,
) -> Result<K1ExactOpportunityOutcomeV1, String> {
    let blocker = if !error.is_empty()
        && error.len() <= 128
        && error
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        error
    } else {
        "internal_unclassified"
    };
    let identifier_report_root_sha256 = nando_operator_kernel::canonical_json_sha256(&(
        "nando.k1-exact-replay-unclassified-identifier.v1",
        opportunity_root_sha256.as_str(),
        blocker,
    ))
    .map_err(str::to_owned)?;
    let disposition_set_root_sha256 = nando_operator_kernel::canonical_json_sha256(&(
        "nando.k1-exact-replay-unclassified-disposition.v1",
        opportunity_root_sha256.as_str(),
        blocker,
    ))
    .map_err(str::to_owned)?;
    Ok(K1ExactOpportunityOutcomeV1 {
        opportunity_root_sha256,
        identifier_report_root_sha256,
        disposition_set_root_sha256,
        outcome: K1ExactReplayOutcomeClassV1::Unclassified,
        blocker: blocker.to_owned(),
        support_rows,
        seed_programs: 0,
        accepted_programs: 0,
        semantic_classes: 0,
    })
}

#[must_use]
pub fn default_snapshot_path(root: impl Into<PathBuf>) -> PathBuf {
    root.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_outcome_classes_do_not_promote_operational_failures() {
        assert_eq!(
            classify_replay_outcome(
                MultiSourceT1IdentificationStateV1::CandidateGenerationEmpty,
                "motif_program_candidates_empty",
                false,
            ),
            K1ExactReplayOutcomeClassV1::DeterministicPreFuture
        );
        assert_eq!(
            classify_replay_outcome(
                MultiSourceT1IdentificationStateV1::Ambiguous,
                "multiple_semantic_classes_require_distinguishing_evidence",
                false,
            ),
            K1ExactReplayOutcomeClassV1::FutureContingent
        );
        assert_eq!(
            classify_replay_outcome(
                MultiSourceT1IdentificationStateV1::InvalidEvidence,
                "runtime_binding_failed",
                false,
            ),
            K1ExactReplayOutcomeClassV1::OperationalRetryable
        );
    }

    #[test]
    fn replay_receipt_serialization_excludes_private_payload_fields() {
        let source = include_str!("k1_exact_opportunity_replay.rs");
        for forbidden in [
            "request_text",
            "provider_payload",
            "session_id_sha256",
            "turn_intent_id_sha256",
            "private_key",
        ] {
            assert!(!source.contains(&format!("pub {forbidden}:")));
        }
    }

    #[test]
    fn candidate_local_errors_are_rooted_and_private_text_is_not_retained() {
        let outcome =
            unclassified_replay_outcome("1".repeat(64), "source_neutral_self_replay_failed", 8)
                .expect("stable outcome");
        assert_eq!(outcome.outcome, K1ExactReplayOutcomeClassV1::Unclassified);
        assert_eq!(outcome.blocker, "source_neutral_self_replay_failed");
        assert_eq!(outcome.support_rows, 8);

        let private =
            unclassified_replay_outcome("2".repeat(64), "/private/session/path: decode failed", 4)
                .expect("redacted outcome");
        assert_eq!(private.blocker, "internal_unclassified");
        assert!(
            !serde_json::to_string(&private)
                .expect("encode")
                .contains("/private")
        );
    }
}
