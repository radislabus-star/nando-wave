use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

use nando_core::wave::{
    CandidateCubeField, CoherentOperatorCandidate, OperatorCircuit, OperatorCircuitRelation,
    OperatorCircuitStage, OperatorGrokkingConfig, OperatorGrokkingConsolidator,
    OperatorRelationCell, PhaseCenterCell, TernaryRelationState, VerifiedPartialRelationWave,
    VerifiedRelationSample, VerifiedWaveOutcome,
};

fn phase(angle: f64) -> PhaseCenterCell {
    PhaseCenterCell {
        re: angle.cos(),
        im: angle.sin(),
    }
}

fn cell(plane: u8, source_role: u8, target_role: u8) -> OperatorRelationCell {
    OperatorRelationCell {
        plane,
        source_role,
        target_role,
    }
}

fn circuit(anchors: [f64; 3]) -> OperatorCircuit {
    OperatorCircuit::new(
        3,
        [
            (cell(0, 0, 1), anchors[0]),
            (cell(0, 1, 2), anchors[1]),
            (cell(1, 0, 2), anchors[2]),
        ]
        .into_iter()
        .map(|(relation_cell, angle)| OperatorCircuitRelation {
            cell: relation_cell,
            state: TernaryRelationState::Supported,
            phase_anchor: phase(angle),
        })
        .collect(),
    )
    .expect("test circuit is connected")
}

fn wave(
    receipt: u64,
    surface: u64,
    relation_cell: OperatorRelationCell,
    observed_phase: PhaseCenterCell,
) -> VerifiedPartialRelationWave {
    VerifiedPartialRelationWave::new(
        receipt,
        surface,
        surface + 10_000,
        7,
        VerifiedWaveOutcome::Positive,
        vec![VerifiedRelationSample {
            cell: relation_cell,
            state: TernaryRelationState::Supported,
            phase: observed_phase,
        }],
    )
    .expect("verified partial wave")
}

fn run_arm(
    candidate_anchors: [f64; 3],
    observed_phases: [PhaseCenterCell; 3],
    surface_base: u64,
) -> Option<CoherentOperatorCandidate> {
    let mut field =
        CandidateCubeField::new(7, OperatorGrokkingConfig::default()).expect("valid bounded field");
    field
        .register_circuit(circuit(candidate_anchors))
        .expect("candidate circuit");
    field
        .register_circuit(circuit([FRAC_PI_4, -FRAC_PI_4, FRAC_PI_4]))
        .expect("matched competitor");
    for (index, relation_cell) in [cell(0, 0, 1), cell(0, 1, 2), cell(1, 0, 2)]
        .into_iter()
        .enumerate()
    {
        field
            .observe(wave(
                surface_base + index as u64,
                surface_base + index as u64,
                relation_cell,
                observed_phases[index],
            ))
            .expect("bounded wave");
    }
    let report = OperatorGrokkingConsolidator::consolidate(&field);
    report.candidate
}

#[test]
fn only_cross_plane_phase_coherence_forms_the_operator_circuit() {
    let law = [0.0, FRAC_PI_2, PI];
    let full_observed = [phase(0.0), phase(FRAC_PI_2), phase(PI)];

    let full = run_arm(law, full_observed, 100).expect("full phase forms circuit");
    let restored = run_arm(law, full_observed, 200).expect("restored phase forms circuit");
    assert_eq!(
        full.circuit.fingerprint64(),
        restored.circuit.fingerprint64()
    );

    let no_phase = run_arm(law, [PhaseCenterCell::default(); 3], 300);
    let shuffled = run_arm(law, [phase(FRAC_PI_2), phase(PI), phase(0.0)], 400);
    let magnitude_only = run_arm(law, [phase(0.0); 3], 500);
    let matched_random = run_arm([PI, PI, PI], full_observed, 600);

    assert!(no_phase.is_none());
    assert!(shuffled.is_none());
    assert!(magnitude_only.is_none());
    assert!(matched_random.is_none());
    assert!(full.coherence > 0.999);
    assert!(full.margin_over_runner_up > 0.5);
}

#[test]
fn transfer_survives_after_exact_support_field_is_dropped() {
    let law = [0.0, FRAC_PI_2, PI];
    let observed = [phase(0.0), phase(FRAC_PI_2), phase(PI)];
    let crystallized = run_arm(law, observed, 1_000).expect("support forms circuit");
    let fingerprint = crystallized.circuit.fingerprint64();

    // The candidate carries the circuit. No support wave or exact episode is
    // retained when the independent future field is constructed.
    let mut future =
        CandidateCubeField::new(7, OperatorGrokkingConfig::default()).expect("future field");
    future
        .register_circuit(crystallized.circuit)
        .expect("crystallized circuit only");
    for (index, relation_cell) in [cell(0, 0, 1), cell(0, 1, 2), cell(1, 0, 2)]
        .into_iter()
        .enumerate()
    {
        future
            .observe(wave(
                2_000 + index as u64,
                3_000 + index as u64,
                relation_cell,
                observed[index],
            ))
            .expect("independent future wave");
    }

    let future_report = OperatorGrokkingConsolidator::consolidate(&future);
    assert_eq!(future_report.stage, OperatorCircuitStage::CoherentCandidate);
    assert_eq!(
        future_report
            .candidate
            .expect("future transfer")
            .circuit
            .fingerprint64(),
        fingerprint
    );
}
