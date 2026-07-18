use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

use nando_core::wave::{
    CandidateCubeField, CircuitSynthesisConfig, CircuitSynthesizer, FrozenFutureCircuitField,
    FrozenSynthesizedCircuitSet, OperatorCircuit, OperatorCircuitRelation, OperatorCircuitStage,
    OperatorGrokkingConfig, OperatorGrokkingConsolidator, OperatorRelationCell, PhaseCenterCell,
    TernaryRelationState, VerifiedPartialRelationWave, VerifiedRelationSample, VerifiedWaveOutcome,
};

fn phase(angle: f64) -> PhaseCenterCell {
    PhaseCenterCell {
        re: angle.cos(),
        im: angle.sin(),
    }
}

fn partial_wave(
    receipt_id: u64,
    surface_id: u64,
    generation: u64,
    cell: OperatorRelationCell,
    state: TernaryRelationState,
    sample_phase: PhaseCenterCell,
) -> VerifiedPartialRelationWave {
    VerifiedPartialRelationWave::new(
        receipt_id,
        surface_id,
        surface_id + 100,
        generation,
        VerifiedWaveOutcome::Positive,
        vec![VerifiedRelationSample {
            cell,
            state,
            phase: sample_phase,
        }],
    )
    .expect("valid partial wave")
}

fn cells() -> [(OperatorRelationCell, TernaryRelationState, PhaseCenterCell); 3] {
    [
        (
            OperatorRelationCell {
                plane: 0,
                source_role: 0,
                target_role: 1,
            },
            TernaryRelationState::Supported,
            phase(0.0),
        ),
        (
            OperatorRelationCell {
                plane: 1,
                source_role: 1,
                target_role: 2,
            },
            TernaryRelationState::Supported,
            phase(FRAC_PI_2),
        ),
        (
            OperatorRelationCell {
                plane: 2,
                source_role: 0,
                target_role: 2,
            },
            TernaryRelationState::Opposed,
            phase(PI),
        ),
    ]
}

fn future_field(
    frozen: &FrozenSynthesizedCircuitSet,
    phases: [PhaseCenterCell; 3],
) -> FrozenFutureCircuitField {
    let mut field = frozen
        .new_future_field(OperatorGrokkingConfig::default())
        .expect("future field");
    for (index, ((cell, state, _), sample_phase)) in cells().into_iter().zip(phases).enumerate() {
        field
            .observe(partial_wave(
                100 + index as u64,
                20 + index as u64,
                frozen.source_generation(),
                cell,
                state,
                sample_phase,
            ))
            .expect("disjoint future wave");
    }
    field
}

#[test]
fn unseen_circuit_is_synthesized_then_requires_future_phase_coherence() {
    let support = cells()
        .into_iter()
        .enumerate()
        .map(|(index, (cell, state, sample_phase))| {
            partial_wave(
                1 + index as u64,
                1 + index as u64,
                7,
                cell,
                state,
                sample_phase,
            )
        })
        .collect::<Vec<_>>();

    // The target circuit is not registered. Three partial support surfaces
    // create its topology and anchors, then are dropped before future proof.
    let synthesis = CircuitSynthesizer::synthesize(&support, CircuitSynthesisConfig::default())
        .expect("circuit synthesis");
    assert_eq!(synthesis.emitted_circuits, 1);
    assert!(support.iter().all(|wave| wave.samples.len() == 1));
    let target_fingerprint = synthesis.circuits[0].fingerprint64();
    let frozen = FrozenSynthesizedCircuitSet::freeze(7, &synthesis).expect("frozen topology set");
    drop(support);

    let mut reuse_guard = frozen
        .new_future_field(OperatorGrokkingConfig::default())
        .expect("future reuse guard");
    let (cell, state, sample_phase) = cells()[0];
    assert_eq!(
        reuse_guard.observe(partial_wave(1, 50, 7, cell, state, sample_phase)),
        Err(nando_core::wave::FrozenCircuitSetError::SupportReceiptReused)
    );

    let full_field = future_field(&frozen, [phase(0.0), phase(FRAC_PI_2), phase(PI)]);
    let full = OperatorGrokkingConsolidator::consolidate(full_field.candidate_field());
    assert_eq!(full.stage, OperatorCircuitStage::CoherentCandidate);
    assert_eq!(
        full.candidate
            .as_ref()
            .map(|candidate| candidate.circuit.fingerprint64()),
        Some(target_fingerprint)
    );

    let no_phase_field = future_field(&frozen, [PhaseCenterCell::default(); 3]);
    let no_phase = OperatorGrokkingConsolidator::consolidate(no_phase_field.candidate_field());
    assert!(no_phase.candidate.is_none());

    let shuffled_field = future_field(&frozen, [phase(FRAC_PI_2), phase(PI), phase(0.0)]);
    let shuffled = OperatorGrokkingConsolidator::consolidate(shuffled_field.candidate_field());
    assert!(shuffled.candidate.is_none());

    let magnitude_field = future_field(&frozen, [phase(0.0), phase(0.0), phase(0.0)]);
    let magnitude_only =
        OperatorGrokkingConsolidator::consolidate(magnitude_field.candidate_field());
    assert!(magnitude_only.candidate.is_none());

    let random_relations = frozen.circuits()[0]
        .relations()
        .iter()
        .map(|relation| OperatorCircuitRelation {
            phase_anchor: phase(FRAC_PI_4),
            ..*relation
        })
        .collect();
    let random_circuit = OperatorCircuit::new(3, random_relations).expect("random control");
    let mut random_field =
        CandidateCubeField::new(8, OperatorGrokkingConfig::default()).expect("random field");
    random_field
        .register_circuit(random_circuit)
        .expect("random circuit");
    for (index, (cell, state, sample_phase)) in cells().into_iter().enumerate() {
        random_field
            .observe(partial_wave(
                200 + index as u64,
                30 + index as u64,
                8,
                cell,
                state,
                sample_phase,
            ))
            .expect("random future wave");
    }
    let random = OperatorGrokkingConsolidator::consolidate(&random_field);
    assert!(random.candidate.is_none());

    let restored_field = future_field(&frozen, [phase(0.0), phase(FRAC_PI_2), phase(PI)]);
    let restored = OperatorGrokkingConsolidator::consolidate(restored_field.candidate_field());
    assert_eq!(restored.stage, OperatorCircuitStage::CoherentCandidate);
    assert_eq!(
        restored
            .candidate
            .as_ref()
            .map(|candidate| candidate.circuit.fingerprint64()),
        Some(target_fingerprint)
    );
}
