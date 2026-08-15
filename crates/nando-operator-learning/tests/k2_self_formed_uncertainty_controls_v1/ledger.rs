use std::collections::BTreeMap;

const EXPECTED: [(&str, &str); 36] = [
    ("01", "opaque_action_id_permutation_equivariant"),
    ("02", "path_bijection_equivariant"),
    ("03", "content_token_bijection_equivariant"),
    ("04", "support_row_shuffle_invariant"),
    ("05", "case_order_shuffle_invariant"),
    ("06", "matched_geometry_private_truth_differs"),
    ("07", "supplied_candidate_model_field_rejected"),
    ("08", "prepared_model_list_rejected"),
    ("09", "omitted_consistent_model_detected"),
    ("10", "extra_inconsistent_model_detected"),
    ("11", "support_row_removal_enlarges_model_set"),
    (
        "12",
        "support_outcome_mutation_changes_or_invalidates_model_set",
    ),
    ("13", "syntactic_duplicate_rejected_before_materialization"),
    ("14", "semantic_duplicate_cannot_increase_cardinality"),
    ("15", "probe_role_field_rejected"),
    ("16", "omitted_raw_probe_detected"),
    ("17", "extra_non_derived_probe_rejected"),
    ("18", "supplied_safety_cost_mismatch_rejected"),
    ("19", "out_of_tree_probe_vetoed"),
    ("20", "risk_formula_mutation_detected"),
    ("21", "cost_formula_mutation_detected"),
    ("22", "predecessor_scorer_mutation_detected"),
    ("23", "nonce_bytes_absent_from_public_requests"),
    ("24", "private_mapping_absent_from_public_requests"),
    ("25", "post_outcome_selection_rejected"),
    ("26", "dispatch_before_batch_barrier_rejected"),
    ("27", "same_identity_redispatch_rejected"),
    ("28", "observer_worker_stdout_field_rejected"),
    ("29", "final_verifier_forbidden_imports_absent"),
    ("30", "development_root_in_confirm_packet_rejected"),
    ("31", "authority_true_rejected"),
    ("32", "cleanup_before_publication_rejected"),
    ("T1", "absolute_post_root_quotient_rejected"),
    ("T2", "representative_top_k_rejected"),
    ("T3", "tournament_omission_or_duplicate_filler_rejected"),
    ("T4", "tournament_direct_winner_mismatch_rejected"),
];

const V4_EXPECTED: [(&str, &str); 16] = [
    ("J1", "private_completion_input_rejected"),
    ("J2", "post_outcome_completion_input_rejected"),
    ("J3", "omitted_completion_candidate_rejected"),
    ("J4", "duplicate_or_foreign_second_probe_rejected"),
    ("J5", "wrong_joint_and_matrix_rejected"),
    ("J6", "changed_predecessor_winner_rejected"),
    ("J7", "swapped_probe_ordinal_rejected"),
    ("J8", "shared_workspace_binding_rejected"),
    ("J9", "missing_second_observation_rejected"),
    ("J10", "cleanup_before_terminal_rejected"),
    ("J11", "unavailable_case_omission_rejected"),
    ("J12", "completion_count_or_membership_rejected"),
    ("J13", "non_global_second_winner_rejected"),
    ("J14", "cumulative_budget_excess_rejected"),
    ("J15", "invalid_crash_prefix_or_redispatch_rejected"),
    ("J16", "stored_joint_partition_drift_rejected"),
];

pub struct ControlLedger {
    passed: BTreeMap<&'static str, &'static str>,
}

pub struct V4ControlLedger {
    passed: BTreeMap<&'static str, &'static str>,
}

impl V4ControlLedger {
    pub fn new() -> Self {
        Self {
            passed: BTreeMap::new(),
        }
    }

    pub fn pass(&mut self, id: &'static str, disposition: &'static str) {
        assert!(
            self.passed.insert(id, disposition).is_none(),
            "duplicate {id}"
        );
    }

    pub fn finish(self) {
        let expected = V4_EXPECTED.into_iter().collect::<BTreeMap<_, _>>();
        assert_eq!(self.passed, expected);
        eprintln!("R7 V4 controls PASS: J1-J16");
    }
}

impl ControlLedger {
    pub fn new() -> Self {
        Self {
            passed: BTreeMap::new(),
        }
    }

    pub fn pass(&mut self, id: &'static str, disposition: &'static str) {
        assert!(
            self.passed.insert(id, disposition).is_none(),
            "duplicate {id}"
        );
    }

    pub fn finish(self) {
        let expected = EXPECTED.into_iter().collect::<BTreeMap<_, _>>();
        assert_eq!(self.passed, expected);
        eprintln!("R7 controls PASS: 32/32 plus T1-T4");
    }
}
