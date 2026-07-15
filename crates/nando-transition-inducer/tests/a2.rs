use nando_transition_inducer::a2_lab::run_a2_proof;

#[test]
fn frozen_a2_four_operator_weak_observation_contract_passes() {
    let report = run_a2_proof().unwrap_or_else(|error| panic!("A2 failed: {error}"));
    assert!(report.verdicts.operator_expansion_pass, "{report:#?}");
    assert!(report.verdicts.weak_observability_pass, "{report:#?}");
    assert!(report.verdicts.wave_contribution_pass, "{report:#?}");
    assert!(report.verdicts.overall_pass, "{report:#?}");
}
