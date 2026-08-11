use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use nando_operator_kernel::sha256_bytes;
use nando_operator_learning::{
    DecisionContractDurabilityReceiptV1, DecisionContractPrecommitV1, FramedCborLedger,
    MAX_DECISION_PRECOMMIT_BYTES_V1, read_framed_cbor,
};

pub const GROUNDED_DECISION_PRECOMMIT_LEDGER_PREFIX_V1: &str = "decision-precommit";
pub const GROUNDED_DECISION_PRECOMMIT_SEGMENT_BYTES_V1: u64 = 64 * 1024 * 1024;
pub const GROUNDED_DECISION_PRECOMMIT_QUOTA_BYTES_V1: u64 = 2 * 1024 * 1024 * 1024;

const FRAME_HEADER_BYTES: u64 = 12;

pub struct GroundedDecisionPrecommitJournalV1 {
    directory: PathBuf,
    ledger: FramedCborLedger,
    seen_request_roots: BTreeSet<String>,
    total_bytes: u64,
    quota_bytes: u64,
    poisoned: bool,
}

impl GroundedDecisionPrecommitJournalV1 {
    pub fn open(directory: &Path) -> Result<Self, String> {
        Self::open_with_budgets(
            directory,
            GROUNDED_DECISION_PRECOMMIT_SEGMENT_BYTES_V1,
            GROUNDED_DECISION_PRECOMMIT_QUOTA_BYTES_V1,
        )
    }

    fn open_with_budgets(
        directory: &Path,
        segment_bytes: u64,
        quota_bytes: u64,
    ) -> Result<Self, String> {
        let ledger = FramedCborLedger::open_with_limits(
            directory,
            GROUNDED_DECISION_PRECOMMIT_LEDGER_PREFIX_V1,
            segment_bytes,
            1,
        )?;
        let recovered = read_framed_cbor::<DecisionContractPrecommitV1>(
            directory,
            GROUNDED_DECISION_PRECOMMIT_LEDGER_PREFIX_V1,
        )?;
        let mut seen_request_roots = BTreeSet::new();
        for precommit in recovered {
            precommit.validate().map_err(str::to_owned)?;
            if !seen_request_roots.insert(precommit.request_event_identity_root_sha256) {
                return Err("grounded_decision_journal_duplicate_recovery".to_owned());
            }
        }
        let total_bytes = directory_bytes(directory)?;
        if total_bytes > quota_bytes {
            return Err("grounded_decision_journal_quota_exhausted".to_owned());
        }
        Ok(Self {
            directory: directory.to_owned(),
            ledger,
            seen_request_roots,
            total_bytes,
            quota_bytes,
            poisoned: false,
        })
    }

    pub fn append_precommit(
        &mut self,
        precommit: &DecisionContractPrecommitV1,
    ) -> Result<DecisionContractDurabilityReceiptV1, String> {
        if self.poisoned {
            return Err("grounded_decision_journal_poisoned".to_owned());
        }
        precommit.validate().map_err(str::to_owned)?;
        if self
            .seen_request_roots
            .contains(&precommit.request_event_identity_root_sha256)
        {
            return Err("grounded_decision_journal_duplicate_request".to_owned());
        }
        let payload = serde_cbor::to_vec(precommit)
            .map_err(|error| format!("grounded_decision_precommit_encode:{error}"))?;
        if payload.is_empty() || payload.len() > MAX_DECISION_PRECOMMIT_BYTES_V1 {
            return Err("grounded_decision_precommit_payload_budget".to_owned());
        }
        let payload_bytes = u32::try_from(payload.len())
            .map_err(|_| "grounded_decision_precommit_payload_budget".to_owned())?;
        let appended_bytes = FRAME_HEADER_BYTES.saturating_add(u64::from(payload_bytes));
        if self.total_bytes.saturating_add(appended_bytes) > self.quota_bytes {
            return Err("grounded_decision_journal_quota_exhausted".to_owned());
        }
        let frame = match self.ledger.append(precommit) {
            Ok(frame) => frame,
            Err(error) => {
                self.poisoned = true;
                return Err(error);
            }
        };
        if let Err(error) = self.ledger.sync() {
            self.poisoned = true;
            return Err(error);
        }
        let receipt = DecisionContractDurabilityReceiptV1::seal(
            precommit.precommit_root_sha256.clone(),
            frame.segment_id,
            frame.offset,
            payload_bytes,
            sha256_bytes(&payload),
        )
        .map_err(str::to_owned)?;
        self.total_bytes = self.total_bytes.saturating_add(appended_bytes);
        self.seen_request_roots
            .insert(precommit.request_event_identity_root_sha256.clone());
        Ok(receipt)
    }

    pub fn recover_precommits(&self) -> Result<Vec<DecisionContractPrecommitV1>, String> {
        let records: Vec<DecisionContractPrecommitV1> = read_framed_cbor(
            &self.directory,
            GROUNDED_DECISION_PRECOMMIT_LEDGER_PREFIX_V1,
        )?;
        for record in &records {
            record.validate().map_err(str::to_owned)?;
        }
        Ok(records)
    }

    #[must_use]
    pub const fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    #[must_use]
    pub const fn poisoned(&self) -> bool {
        self.poisoned
    }
}

fn directory_bytes(directory: &Path) -> Result<u64, String> {
    let mut total = 0_u64;
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("grounded_decision_journal_read_dir:{error}"))?
    {
        let entry = entry.map_err(|error| format!("grounded_decision_journal_entry:{error}"))?;
        let metadata = entry
            .metadata()
            .map_err(|error| format!("grounded_decision_journal_metadata:{error}"))?;
        if metadata.is_file() {
            total = total.saturating_add(metadata.len());
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    use nando_operator_learning::multi_source::K1ConsequenceTypeV1;
    use nando_operator_learning::{
        AvailableActionContractsV1, DecisionAuthoritySnapshotV1, DecisionContractPrecommitInputV1,
        ExactPreActionGoalInputV1, TypedGoalComparatorV1, TypedGoalPredicateArtifactV1,
        bind_exact_pre_action_goal_v1,
    };

    use super::*;

    fn root(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    fn temp_directory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "nando-grounded-decision-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn precommit(request_root: String, sequence: u64) -> DecisionContractPrecommitV1 {
        let predicate = TypedGoalPredicateArtifactV1::seal(
            TypedGoalComparatorV1::TypedValueRootEquals,
            K1ConsequenceTypeV1::Scalar,
            root('a'),
            root('b'),
        )
        .expect("predicate");
        let goal = bind_exact_pre_action_goal_v1(ExactPreActionGoalInputV1 {
            predicate_artifact: predicate,
            pre_action_goal_evidence_root_sha256: root('c'),
            outcome_horizon_contract_root_sha256: root('d'),
            observation_mask_root_sha256: root('e'),
            feature_exclusion_root_sha256: root('f'),
            binder_schema_root_sha256: root('1'),
            pre_action_observation_root_sha256: root('2'),
            independent_binder_root_sha256: root('3'),
            frozen_at_sequence: sequence,
            action_selection_not_before_sequence: sequence + 2,
        })
        .expect("goal");
        let authority = DecisionAuthoritySnapshotV1::seal(
            "nando.response-registry.v6".to_owned(),
            7,
            root('4'),
            root('5'),
            8,
            root('6'),
            root('7'),
            root('8'),
        )
        .expect("authority");
        let actions =
            AvailableActionContractsV1::seal(vec![root('9')], root('a')).expect("actions");
        DecisionContractPrecommitV1::seal(DecisionContractPrecommitInputV1 {
            request_event_identity_root_sha256: request_root,
            process_epoch_root_sha256: root('b'),
            pre_action_observation_root_sha256: root('2'),
            pre_action_topology_root_sha256: root('c'),
            goal_contract: goal.goal_contract,
            goal_binding_receipt: goal.binding_receipt,
            constraint_contract_root_sha256: root('d'),
            authority_snapshot: authority,
            applicability_evaluator_schema: "nando.response-pre-action-evaluator.v1".to_owned(),
            available_action_contracts_root_sha256: actions.contracts_root_sha256,
            opaque_execution_binding_set_root_sha256: root('e'),
            journal_sequence: sequence + 1,
            action_selection_not_before_sequence: sequence + 2,
            precommit_monotonic_nanos: sequence + 3,
        })
        .expect("precommit")
    }

    #[test]
    fn append_sync_receipt_and_restart_recovery_are_exact() {
        let root_dir = temp_directory("restart");
        let precommit = precommit(root('f'), 10);
        let receipt = {
            let mut journal = GroundedDecisionPrecommitJournalV1::open(&root_dir).expect("journal");
            let receipt = journal.append_precommit(&precommit).expect("append");
            receipt.validate().expect("receipt");
            assert_eq!(
                journal.recover_precommits().expect("recover"),
                vec![precommit.clone()]
            );
            receipt
        };
        let restored = GroundedDecisionPrecommitJournalV1::open(&root_dir).expect("reopen");
        assert_eq!(
            restored.recover_precommits().expect("recover"),
            vec![precommit]
        );
        assert!(receipt.offset >= 4);
        fs::remove_dir_all(root_dir).expect("cleanup");
    }

    #[test]
    fn duplicate_request_and_quota_fail_closed() {
        let root_dir = temp_directory("guards");
        let first = precommit(root('f'), 20);
        let one_record_quota = 4_u64.saturating_add(FRAME_HEADER_BYTES).saturating_add(
            u64::try_from(serde_cbor::to_vec(&first).expect("payload").len())
                .expect("payload size"),
        );
        let mut journal = GroundedDecisionPrecommitJournalV1::open_with_budgets(
            &root_dir,
            GROUNDED_DECISION_PRECOMMIT_SEGMENT_BYTES_V1,
            one_record_quota,
        )
        .expect("journal");
        journal.append_precommit(&first).expect("first");
        assert_eq!(
            journal.append_precommit(&first).expect_err("duplicate"),
            "grounded_decision_journal_duplicate_request"
        );
        let second = precommit(root('e'), 30);
        assert_eq!(
            journal.append_precommit(&second).expect_err("quota"),
            "grounded_decision_journal_quota_exhausted"
        );
        fs::remove_dir_all(root_dir).expect("cleanup");
    }

    #[test]
    fn partial_tail_is_removed_without_replaying_an_action() {
        let root_dir = temp_directory("partial-tail");
        let precommit = precommit(root('f'), 40);
        {
            let mut journal = GroundedDecisionPrecommitJournalV1::open(&root_dir).expect("journal");
            journal.append_precommit(&precommit).expect("append");
        }
        let segment = fs::read_dir(&root_dir)
            .expect("segments")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.is_file())
            .expect("segment");
        OpenOptions::new()
            .append(true)
            .open(segment)
            .expect("open tail")
            .write_all(&[7, 0, 0, 0, 1, 2, 3])
            .expect("write tail");
        let restored = GroundedDecisionPrecommitJournalV1::open(&root_dir).expect("reopen");
        assert_eq!(
            restored.recover_precommits().expect("recover"),
            vec![precommit]
        );
        fs::remove_dir_all(root_dir).expect("cleanup");
    }

    #[test]
    fn persisted_precommit_contains_no_raw_marker() {
        let root_dir = temp_directory("raw-marker");
        let marker = "S1C_RAW_REQUEST_MARKER_MUST_NOT_PERSIST";
        let mut journal = GroundedDecisionPrecommitJournalV1::open(&root_dir).expect("journal");
        journal
            .append_precommit(&precommit(root('f'), 50))
            .expect("append");
        drop(journal);
        for entry in fs::read_dir(&root_dir).expect("segments") {
            let bytes = fs::read(entry.expect("entry").path()).expect("bytes");
            assert!(
                !bytes
                    .windows(marker.len())
                    .any(|window| window == marker.as_bytes())
            );
        }
        fs::remove_dir_all(root_dir).expect("cleanup");
    }

    #[test]
    #[ignore = "isolated remote release S1C durability and rotation gate"]
    fn durable_sync_path_stays_within_budget_and_rotates_exactly() {
        const RECORDS: usize = 1_024;
        let root_dir = temp_directory("sync-gate");
        let mut journal = GroundedDecisionPrecommitJournalV1::open_with_budgets(
            &root_dir,
            1024 * 1024,
            GROUNDED_DECISION_PRECOMMIT_QUOTA_BYTES_V1,
        )
        .expect("journal");
        let mut samples = Vec::with_capacity(RECORDS);
        for index in 0..RECORDS {
            let request_root = sha256_bytes(format!("s1c-sync-request-{index}").as_bytes());
            let sequence = 1_000_u64.saturating_add((index as u64).saturating_mul(10));
            let record = precommit(request_root, sequence);
            let started = Instant::now();
            journal.append_precommit(&record).expect("append");
            samples.push(started.elapsed().as_nanos());
        }
        let p99 = percentile_ns(&samples, 99);
        let hard_max = samples.iter().copied().max().unwrap_or(u128::MAX);
        let segment_count = fs::read_dir(&root_dir)
            .expect("segments")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_file())
            .count();
        assert!(segment_count >= 2, "journal did not rotate");
        assert_eq!(
            journal.recover_precommits().expect("recover").len(),
            RECORDS
        );
        println!(
            "S1C_SYNC_LATENCY p99_ns={p99} hard_max_ns={hard_max} records={RECORDS} segments={segment_count}"
        );
        assert!(p99 <= 5_000_000, "sync p99 exceeded 5 ms");
        assert!(hard_max <= 20_000_000, "sync hard ceiling exceeded 20 ms");
        fs::remove_dir_all(root_dir).expect("cleanup");
    }

    fn percentile_ns(samples: &[u128], percentile: usize) -> u128 {
        let mut sorted = samples.to_vec();
        sorted.sort_unstable();
        let index = sorted
            .len()
            .saturating_mul(percentile)
            .div_ceil(100)
            .saturating_sub(1)
            .min(sorted.len().saturating_sub(1));
        sorted[index]
    }
}
