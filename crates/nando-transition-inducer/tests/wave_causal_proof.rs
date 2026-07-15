use nando_transition_inducer::run_wave_causal_proof;

#[test]
fn wave_phase_causally_reduces_a2_program_search() {
    let report =
        run_wave_causal_proof().unwrap_or_else(|error| panic!("causal proof failed: {error}"));
    assert!(report.verdicts.full_execution_pass, "{report:#?}");
    assert!(report.verdicts.phase_causal_pass, "{report:#?}");
    assert!(report.verdicts.relational_atoms_causal_pass, "{report:#?}");
    assert!(report.verdicts.anti_center_causal_pass, "{report:#?}");
    assert!(report.verdicts.clustered_anti_center_pass, "{report:#?}");
    assert!(report.verdicts.core_causal_pass, "{report:#?}");
    assert!(report.verdicts.strict_all_ablation_pass, "{report:#?}");
    assert!(report.verdicts.formation_final_pass, "{report:#?}");
    assert!(!report.formation_verdict.is_empty(), "{report:#?}");
}
