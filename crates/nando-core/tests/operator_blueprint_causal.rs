use nando_core::wave::{
    BlueprintBeamConfig, BlueprintFutureEvaluator, BlueprintPhaseControl, BoundedCircuitBeam,
    BoundedRoleAligner, Commitment256, FrozenOperatorBlueprintSet, LocalRelationFragment,
    PhaseCenterCell, RoleAlignmentConfig, StructuralRoleSignature, SurfaceFragmentBundle,
    TernaryRelationState,
};

fn digest(byte: u8) -> Commitment256 {
    [byte; 32]
}

fn phase(angle: f64) -> PhaseCenterCell {
    PhaseCenterCell {
        re: angle.cos(),
        im: angle.sin(),
    }
}

fn role(canonical_role: u8) -> StructuralRoleSignature {
    StructuralRoleSignature::new(canonical_role + 1, 1, canonical_role, 0, Vec::new())
}

fn partial_surface(
    lineage: u8,
    surface: u8,
    local_to_semantic: [u8; 3],
    plane: u8,
    semantic_source: u8,
    semantic_target: u8,
    angle: f64,
) -> SurfaceFragmentBundle {
    let local_source = local_to_semantic
        .iter()
        .position(|role| *role == semantic_source)
        .expect("source role") as u8;
    let local_target = local_to_semantic
        .iter()
        .position(|role| *role == semantic_target)
        .expect("target role") as u8;
    SurfaceFragmentBundle::new(
        digest(lineage),
        digest(surface),
        local_to_semantic.into_iter().map(role).collect::<Vec<_>>(),
        vec![LocalRelationFragment {
            plane,
            source_local_role: local_source,
            target_local_role: local_target,
            state: TernaryRelationState::Supported,
            phase_anchor: phase(angle),
        }],
        Vec::new(),
    )
    .expect("valid partial local surface")
}

fn support_surfaces() -> Vec<SurfaceFragmentBundle> {
    vec![
        partial_surface(1, 31, [2, 0, 1], 0, 0, 1, 0.0),
        partial_surface(2, 32, [1, 2, 0], 0, 0, 1, std::f64::consts::PI),
        partial_surface(3, 33, [0, 2, 1], 1, 1, 2, 0.7),
        partial_surface(4, 34, [2, 1, 0], 1, 1, 2, 0.7 + std::f64::consts::FRAC_PI_2),
        partial_surface(5, 35, [1, 0, 2], 2, 0, 2, 2.2),
        partial_surface(6, 36, [0, 1, 2], 2, 0, 2, 2.2 - std::f64::consts::FRAC_PI_2),
    ]
}

fn future_surfaces() -> Vec<SurfaceFragmentBundle> {
    vec![
        partial_surface(21, 51, [1, 0, 2], 0, 0, 1, 0.0),
        partial_surface(22, 52, [2, 1, 0], 1, 1, 2, 0.7),
        partial_surface(23, 53, [0, 2, 1], 2, 0, 2, 2.2),
    ]
}

#[test]
fn local_partial_graphs_create_competing_circuits_resolved_only_by_future_phase() {
    let support = support_surfaces();
    assert!(support.iter().all(|bundle| bundle.relations().len() == 1));
    let alignments = BoundedRoleAligner::align(&support, RoleAlignmentConfig::default());
    assert!(alignments.completion.is_complete());
    let synthesis =
        BoundedCircuitBeam::synthesize(&support, &alignments, BlueprintBeamConfig::default());
    assert!(synthesis.completion.is_complete());
    assert!(synthesis.blueprints.len() >= 3);

    let frozen =
        FrozenOperatorBlueprintSet::freeze(7, &support, BlueprintBeamConfig::default(), &synthesis)
            .expect("frozen X/Y/Z version space");
    drop(support);
    let future = future_surfaces();

    let full = BlueprintFutureEvaluator::evaluate(
        &frozen,
        &future,
        Default::default(),
        BlueprintPhaseControl::Full,
    );
    let winner = full
        .winner_fingerprint_sha256
        .expect("full phase crystallizes one blueprint");
    assert!(full.runner_up_margin >= 0.10);

    for control in [
        BlueprintPhaseControl::NoPhase,
        BlueprintPhaseControl::ShuffledPhase,
        BlueprintPhaseControl::MagnitudeOnly,
        BlueprintPhaseControl::MatchedRandomCenter,
    ] {
        let ablated =
            BlueprintFutureEvaluator::evaluate(&frozen, &future, Default::default(), control);
        assert_eq!(
            ablated.winner_fingerprint_sha256, None,
            "{control:?} must abstain"
        );
    }

    let restored = BlueprintFutureEvaluator::evaluate(
        &frozen,
        &future,
        Default::default(),
        BlueprintPhaseControl::Full,
    );
    assert_eq!(restored.winner_fingerprint_sha256, Some(winner));
}
