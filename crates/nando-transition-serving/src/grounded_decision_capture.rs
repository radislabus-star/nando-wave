use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use nando_operator_kernel::sha256_bytes;
use nando_operator_learning::{
    DecisionContractDurabilityReceiptV1, DecisionContractPrecommitV1, DurableGoalSatisfactionV1,
    DurableSelectedActionBindingV1, FramedCborLedger, MAX_DECISION_PRECOMMIT_BYTES_V1,
    read_framed_cbor,
};

pub const GROUNDED_DECISION_PRECOMMIT_LEDGER_PREFIX_V1: &str = "decision-precommit";
pub const GROUNDED_DECISION_SELECTED_LEDGER_PREFIX_V1: &str = "selected-action-binding";
pub const GROUNDED_DECISION_SATISFACTION_LEDGER_PREFIX_V1: &str = "goal-satisfaction";
pub const GROUNDED_DECISION_PRECOMMIT_SEGMENT_BYTES_V1: u64 = 64 * 1024 * 1024;
pub const GROUNDED_DECISION_PRECOMMIT_QUOTA_BYTES_V1: u64 = 2 * 1024 * 1024 * 1024;

const FRAME_HEADER_BYTES: u64 = 12;

pub struct GroundedDecisionPrecommitJournalV1 {
    directory: PathBuf,
    precommit_ledger: FramedCborLedger,
    selected_ledger: FramedCborLedger,
    satisfaction_ledger: FramedCborLedger,
    seen_request_roots: BTreeSet<String>,
    precommits_by_root: BTreeMap<String, DecisionContractPrecommitV1>,
    selected_by_precommit_root: BTreeMap<String, DurableSelectedActionBindingV1>,
    satisfaction_by_precommit_root: BTreeMap<String, DurableGoalSatisfactionV1>,
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
        let precommit_ledger = FramedCborLedger::open_with_limits(
            directory,
            GROUNDED_DECISION_PRECOMMIT_LEDGER_PREFIX_V1,
            segment_bytes,
            1,
        )?;
        let selected_ledger = FramedCborLedger::open_with_limits(
            directory,
            GROUNDED_DECISION_SELECTED_LEDGER_PREFIX_V1,
            segment_bytes,
            1,
        )?;
        let satisfaction_ledger = FramedCborLedger::open_with_limits(
            directory,
            GROUNDED_DECISION_SATISFACTION_LEDGER_PREFIX_V1,
            segment_bytes,
            1,
        )?;
        let recovered = read_framed_cbor::<DecisionContractPrecommitV1>(
            directory,
            GROUNDED_DECISION_PRECOMMIT_LEDGER_PREFIX_V1,
        )?;
        let mut seen_request_roots = BTreeSet::new();
        let mut precommits_by_root = BTreeMap::new();
        for precommit in recovered {
            precommit.validate().map_err(str::to_owned)?;
            if !seen_request_roots.insert(precommit.request_event_identity_root_sha256.clone())
                || precommits_by_root
                    .insert(precommit.precommit_root_sha256.clone(), precommit)
                    .is_some()
            {
                return Err("grounded_decision_journal_duplicate_recovery".to_owned());
            }
        }
        let mut selected_by_precommit_root = BTreeMap::new();
        for selected in read_framed_cbor::<DurableSelectedActionBindingV1>(
            directory,
            GROUNDED_DECISION_SELECTED_LEDGER_PREFIX_V1,
        )? {
            let precommit = precommits_by_root
                .get(&selected.receipt.precommit_root_sha256)
                .ok_or_else(|| "grounded_decision_selected_precommit_missing".to_owned())?;
            selected.validate_join(precommit).map_err(str::to_owned)?;
            if selected_by_precommit_root
                .insert(precommit.precommit_root_sha256.clone(), selected)
                .is_some()
            {
                return Err("grounded_decision_selected_replay".to_owned());
            }
        }
        let mut satisfaction_by_precommit_root = BTreeMap::new();
        for satisfaction in read_framed_cbor::<DurableGoalSatisfactionV1>(
            directory,
            GROUNDED_DECISION_SATISFACTION_LEDGER_PREFIX_V1,
        )? {
            let precommit = precommits_by_root
                .get(&satisfaction.precommit_root_sha256)
                .ok_or_else(|| "grounded_decision_satisfaction_precommit_missing".to_owned())?;
            let selected = selected_by_precommit_root
                .get(&satisfaction.precommit_root_sha256)
                .ok_or_else(|| "grounded_decision_satisfaction_selected_missing".to_owned())?;
            satisfaction
                .validate_join(precommit, selected)
                .map_err(str::to_owned)?;
            if satisfaction_by_precommit_root
                .insert(precommit.precommit_root_sha256.clone(), satisfaction)
                .is_some()
            {
                return Err("grounded_decision_satisfaction_replay".to_owned());
            }
        }
        let total_bytes = directory_bytes(directory)?;
        if total_bytes > quota_bytes {
            return Err("grounded_decision_journal_quota_exhausted".to_owned());
        }
        Ok(Self {
            directory: directory.to_owned(),
            precommit_ledger,
            selected_ledger,
            satisfaction_ledger,
            seen_request_roots,
            precommits_by_root,
            selected_by_precommit_root,
            satisfaction_by_precommit_root,
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
        let frame = match self.precommit_ledger.append(precommit) {
            Ok(frame) => frame,
            Err(error) => {
                self.poisoned = true;
                return Err(error);
            }
        };
        if let Err(error) = self.precommit_ledger.sync() {
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
        self.precommits_by_root
            .insert(precommit.precommit_root_sha256.clone(), precommit.clone());
        Ok(receipt)
    }

    pub fn append_selected_action(
        &mut self,
        selected: &DurableSelectedActionBindingV1,
    ) -> Result<(), String> {
        if self.poisoned {
            return Err("grounded_decision_journal_poisoned".to_owned());
        }
        let precommit = self
            .precommits_by_root
            .get(&selected.receipt.precommit_root_sha256)
            .ok_or_else(|| "grounded_decision_selected_precommit_missing".to_owned())?;
        selected.validate_join(precommit).map_err(str::to_owned)?;
        if self
            .selected_by_precommit_root
            .contains_key(&precommit.precommit_root_sha256)
        {
            return Err("grounded_decision_selected_replay".to_owned());
        }
        let appended_bytes = append_synced_record(
            &mut self.selected_ledger,
            selected,
            self.total_bytes,
            self.quota_bytes,
        )
        .inspect_err(|_| self.poisoned = true)?;
        self.total_bytes = self.total_bytes.saturating_add(appended_bytes);
        self.selected_by_precommit_root
            .insert(precommit.precommit_root_sha256.clone(), selected.clone());
        Ok(())
    }

    pub fn append_goal_satisfaction(
        &mut self,
        satisfaction: &DurableGoalSatisfactionV1,
    ) -> Result<(), String> {
        if self.poisoned {
            return Err("grounded_decision_journal_poisoned".to_owned());
        }
        let precommit = self
            .precommits_by_root
            .get(&satisfaction.precommit_root_sha256)
            .ok_or_else(|| "grounded_decision_satisfaction_precommit_missing".to_owned())?;
        let selected = self
            .selected_by_precommit_root
            .get(&satisfaction.precommit_root_sha256)
            .ok_or_else(|| "grounded_decision_satisfaction_selected_missing".to_owned())?;
        satisfaction
            .validate_join(precommit, selected)
            .map_err(str::to_owned)?;
        if self
            .satisfaction_by_precommit_root
            .contains_key(&precommit.precommit_root_sha256)
        {
            return Err("grounded_decision_satisfaction_replay".to_owned());
        }
        let appended_bytes = append_synced_record(
            &mut self.satisfaction_ledger,
            satisfaction,
            self.total_bytes,
            self.quota_bytes,
        )
        .inspect_err(|_| self.poisoned = true)?;
        self.total_bytes = self.total_bytes.saturating_add(appended_bytes);
        self.satisfaction_by_precommit_root.insert(
            precommit.precommit_root_sha256.clone(),
            satisfaction.clone(),
        );
        Ok(())
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

    pub fn recover_selected_actions(&self) -> Result<Vec<DurableSelectedActionBindingV1>, String> {
        read_framed_cbor(&self.directory, GROUNDED_DECISION_SELECTED_LEDGER_PREFIX_V1)
    }

    pub fn recover_goal_satisfactions(&self) -> Result<Vec<DurableGoalSatisfactionV1>, String> {
        read_framed_cbor(
            &self.directory,
            GROUNDED_DECISION_SATISFACTION_LEDGER_PREFIX_V1,
        )
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

fn append_synced_record<T: serde::Serialize>(
    ledger: &mut FramedCborLedger,
    record: &T,
    total_bytes: u64,
    quota_bytes: u64,
) -> Result<u64, String> {
    let payload = serde_cbor::to_vec(record)
        .map_err(|error| format!("grounded_decision_record_encode:{error}"))?;
    if payload.is_empty() || payload.len() > MAX_DECISION_PRECOMMIT_BYTES_V1 {
        return Err("grounded_decision_record_payload_budget".to_owned());
    }
    let payload_bytes =
        u64::try_from(payload.len()).map_err(|_| "grounded_decision_record_payload_budget")?;
    let appended_bytes = FRAME_HEADER_BYTES.saturating_add(payload_bytes);
    if total_bytes.saturating_add(appended_bytes) > quota_bytes {
        return Err("grounded_decision_journal_quota_exhausted".to_owned());
    }
    ledger.append(record)?;
    ledger.sync()?;
    Ok(appended_bytes)
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
        DurableGoalSatisfactionV1, DurableSelectedActionBindingV1, ExactPreActionGoalInputV1,
        GoalSatisfactionReceiptV1, K1ActionContractProjectionV1, OpaqueActionExecutionBindingV1,
        SelectedActionBindingReceiptV1, TypedGoalComparatorV1, TypedGoalPredicateArtifactV1,
        bind_exact_pre_action_goal_v1, opaque_action_execution_binding_set_root_v1,
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

    fn joined_records(
        sequence: u64,
    ) -> (
        DecisionContractPrecommitV1,
        DurableSelectedActionBindingV1,
        DurableGoalSatisfactionV1,
    ) {
        let predicate = TypedGoalPredicateArtifactV1::seal(
            TypedGoalComparatorV1::TypedValueRootEquals,
            K1ConsequenceTypeV1::Scalar,
            root('a'),
            root('b'),
        )
        .expect("predicate");
        let goal = bind_exact_pre_action_goal_v1(ExactPreActionGoalInputV1 {
            predicate_artifact: predicate.clone(),
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
        let projection = K1ActionContractProjectionV1::seal(
            root('1'),
            root('2'),
            root('3'),
            root('4'),
            root('5'),
            root('b'),
            root('6'),
            K1ConsequenceTypeV1::Scalar,
        )
        .expect("projection");
        let binding = OpaqueActionExecutionBindingV1::seal(
            projection.action_contract_root_sha256.clone(),
            root('7'),
            root('8'),
            root('9'),
            authority.response_registry_root_sha256.clone(),
            authority.response_registry_revision,
            authority.certification_ledger_root_sha256.clone(),
            authority.certification_ledger_revision,
        )
        .expect("binding");
        let actions = AvailableActionContractsV1::seal(
            vec![projection.action_contract_root_sha256.clone()],
            root('f'),
        )
        .expect("actions");
        let binding_roots = vec![binding.binding_root_sha256.clone()];
        let precommit = DecisionContractPrecommitV1::seal(DecisionContractPrecommitInputV1 {
            request_event_identity_root_sha256: sha256_bytes(
                format!("s1c-joined-request-{sequence}").as_bytes(),
            ),
            process_epoch_root_sha256: root('a'),
            pre_action_observation_root_sha256: root('2'),
            pre_action_topology_root_sha256: root('c'),
            goal_contract: goal.goal_contract.clone(),
            goal_binding_receipt: goal.binding_receipt,
            constraint_contract_root_sha256: root('d'),
            authority_snapshot: authority,
            applicability_evaluator_schema: "nando.response-pre-action-evaluator.v1".to_owned(),
            available_action_contracts_root_sha256: actions.contracts_root_sha256.clone(),
            opaque_execution_binding_set_root_sha256: opaque_action_execution_binding_set_root_v1(
                binding_roots.clone(),
            )
            .expect("binding set"),
            journal_sequence: sequence + 1,
            action_selection_not_before_sequence: sequence + 2,
            precommit_monotonic_nanos: sequence + 3,
        })
        .expect("precommit");
        let selected_receipt = SelectedActionBindingReceiptV1::seal(
            &precommit,
            projection.action_contract_root_sha256.clone(),
            binding.binding_root_sha256.clone(),
            root('e'),
            sequence + 2,
            sequence + 4,
            precommit.process_epoch_root_sha256.clone(),
        )
        .expect("selected receipt");
        let selected = DurableSelectedActionBindingV1::seal(
            &precommit,
            selected_receipt,
            projection,
            binding,
            actions,
            binding_roots,
            root('a'),
        )
        .expect("selected");
        let satisfaction_receipt = GoalSatisfactionReceiptV1::seal(
            &goal.goal_contract,
            root('a'),
            selected
                .receipt
                .runtime_verification_receipt_root_sha256
                .clone(),
            true,
        )
        .expect("satisfaction receipt");
        let satisfaction = DurableGoalSatisfactionV1::seal(
            &precommit,
            &selected,
            goal.goal_contract,
            predicate,
            satisfaction_receipt,
        )
        .expect("satisfaction");
        (precommit, selected, satisfaction)
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
        let one_record_quota = 12_u64.saturating_add(FRAME_HEADER_BYTES).saturating_add(
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
    fn ordered_three_ledger_join_is_restart_exact_and_replay_closed() {
        let root_dir = temp_directory("three-ledger-restart");
        let (precommit, selected, satisfaction) = joined_records(60);
        {
            let mut journal = GroundedDecisionPrecommitJournalV1::open(&root_dir).expect("journal");
            assert_eq!(
                journal.append_selected_action(&selected),
                Err("grounded_decision_selected_precommit_missing".to_owned())
            );
            journal.append_precommit(&precommit).expect("precommit");
            assert_eq!(
                journal.append_goal_satisfaction(&satisfaction),
                Err("grounded_decision_satisfaction_selected_missing".to_owned())
            );
            journal.append_selected_action(&selected).expect("selected");
            assert_eq!(
                journal.append_selected_action(&selected),
                Err("grounded_decision_selected_replay".to_owned())
            );
            journal
                .append_goal_satisfaction(&satisfaction)
                .expect("satisfaction");
            assert_eq!(
                journal.append_goal_satisfaction(&satisfaction),
                Err("grounded_decision_satisfaction_replay".to_owned())
            );
        }
        let restored = GroundedDecisionPrecommitJournalV1::open(&root_dir).expect("restart");
        assert_eq!(
            restored.recover_precommits().expect("precommits"),
            vec![precommit]
        );
        assert_eq!(
            restored.recover_selected_actions().expect("selected"),
            vec![selected]
        );
        assert_eq!(
            restored
                .recover_goal_satisfactions()
                .expect("satisfactions"),
            vec![satisfaction]
        );
        assert_eq!(
            fs::read_dir(&root_dir)
                .expect("segments")
                .filter_map(Result::ok)
                .filter(|entry| entry.path().is_file())
                .count(),
            3
        );
        fs::remove_dir_all(root_dir).expect("cleanup");
    }

    #[test]
    fn combined_quota_poison_blocks_later_ledgers() {
        let root_dir = temp_directory("combined-quota");
        let (precommit, selected, satisfaction) = joined_records(70);
        let precommit_bytes = u64::try_from(
            serde_cbor::to_vec(&precommit)
                .expect("encode precommit")
                .len(),
        )
        .expect("precommit length fits u64");
        let selected_bytes = u64::try_from(
            serde_cbor::to_vec(&selected)
                .expect("encode selected action")
                .len(),
        )
        .expect("selected action length fits u64");
        let quota = 12_u64
            .saturating_add(FRAME_HEADER_BYTES)
            .saturating_add(precommit_bytes)
            .saturating_add(FRAME_HEADER_BYTES)
            .saturating_add(selected_bytes)
            .saturating_sub(1);
        let mut journal = GroundedDecisionPrecommitJournalV1::open_with_budgets(
            &root_dir,
            GROUNDED_DECISION_PRECOMMIT_SEGMENT_BYTES_V1,
            quota,
        )
        .expect("journal");
        journal.append_precommit(&precommit).expect("precommit");
        assert_eq!(
            journal.append_selected_action(&selected),
            Err("grounded_decision_journal_quota_exhausted".to_owned())
        );
        assert!(journal.poisoned());
        assert_eq!(
            journal.append_goal_satisfaction(&satisfaction),
            Err("grounded_decision_journal_poisoned".to_owned())
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

    #[test]
    #[ignore = "isolated remote release S1C-2 three-ledger sync resource gate"]
    fn three_ledger_sync_path_stays_within_eligible_budget() {
        const RECORDS: usize = 256;
        let root_dir = temp_directory("three-ledger-sync-gate");
        let mut journal = GroundedDecisionPrecommitJournalV1::open_with_budgets(
            &root_dir,
            GROUNDED_DECISION_PRECOMMIT_SEGMENT_BYTES_V1,
            GROUNDED_DECISION_PRECOMMIT_QUOTA_BYTES_V1,
        )
        .expect("journal");
        let mut samples = Vec::with_capacity(RECORDS);
        for index in 0..RECORDS {
            let sequence = 10_000_u64.saturating_add((index as u64).saturating_mul(10));
            let (precommit, selected, satisfaction) = joined_records(sequence);
            let started = Instant::now();
            journal.append_precommit(&precommit).expect("precommit");
            journal.append_selected_action(&selected).expect("selected");
            journal
                .append_goal_satisfaction(&satisfaction)
                .expect("satisfaction");
            samples.push(started.elapsed().as_nanos());
        }
        let p99 = percentile_ns(&samples, 99);
        let hard_max = samples.iter().copied().max().unwrap_or(u128::MAX);
        println!("S1C2_SYNC_LATENCY p99_ns={p99} hard_max_ns={hard_max} records={RECORDS}");
        assert!(p99 <= 5_000_000, "three-ledger sync p99 exceeded 5 ms");
        assert!(
            hard_max <= 20_000_000,
            "three-ledger sync hard ceiling exceeded 20 ms"
        );
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
