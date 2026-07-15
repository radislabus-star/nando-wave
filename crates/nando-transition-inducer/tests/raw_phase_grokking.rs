use nando_transition_inducer::run_raw_phase_grokking_proof;

#[test]
fn raw_delta_feedback_forms_a_portable_phase_circuit() {
    let report = run_raw_phase_grokking_proof()
        .unwrap_or_else(|error| panic!("raw grokking proof failed: {error}"));
    assert!(report.verdicts.anonymous_atom_pass, "{report:#?}");
    assert!(report.verdicts.verifier_feedback_only_pass, "{report:#?}");
    assert!(report.verdicts.exact_cache_disjoint_pass, "{report:#?}");
    assert!(report.verdicts.delayed_transfer_pass, "{report:#?}");
    assert!(report.verdicts.circuit_formation_pass, "{report:#?}");
    assert!(report.verdicts.cleanup_pass, "{report:#?}");
    assert!(report.verdicts.phase_causal_pass, "{report:#?}");
    assert!(report.verdicts.full_execution_pass, "{report:#?}");
    assert!(report.verdicts.package_budget_pass, "{report:#?}");
    assert!(report.verdicts.overall_pass, "{report:#?}");
}
