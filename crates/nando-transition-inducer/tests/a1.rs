use nando_transition_inducer::a1_lab::run_a1_proof;

#[test]
fn frozen_a1_transition_program_induction_passes_full_contract() {
    let report = run_a1_proof().unwrap_or_else(|error| panic!("A1 failed: {error}"));
    assert!(report.verdicts.core_pass, "{report:#?}");
    assert!(report.verdicts.full_contract_pass, "{report:#?}");
    assert!(report.verdicts.correctness_pass, "{report:#?}");
    assert!(report.verdicts.portability_pass, "{report:#?}");
    assert!(report.verdicts.wave_contribution_pass, "{report:#?}");
    assert!(report.verdicts.overall_pass, "{report:#?}");
}
