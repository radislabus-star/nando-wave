use nando_core::wave::{
    BlueprintBeamConfig, BlueprintFutureEvaluator, BlueprintPhaseControl, BoundedCircuitBeam,
    BoundedRoleAligner, Commitment256, FrozenOperatorBlueprintSet, LocalRelationFragment,
    PhaseCenterCell, RoleAlignmentConfig, StructuralRoleSignature, SurfaceFragmentBundle,
    TernaryRelationState, TypedProgramAtom,
};
use nando_response_actor::{
    CrystallizationParityReceipt, CrystallizedOperator, TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR,
    TRANSFORM_VALUE_INTEGER,
};
use serde_json::json;

fn digest(byte: u8) -> Commitment256 {
    [byte; 32]
}

fn phase(angle: f64) -> PhaseCenterCell {
    PhaseCenterCell {
        re: angle.cos(),
        im: angle.sin(),
    }
}

fn role() -> StructuralRoleSignature {
    // Type and temporal metadata deliberately cannot reveal semantic identity.
    StructuralRoleSignature::new(1, 1, 0, 0, vec![0, 1, 2])
}

fn local_role(local_to_semantic: [u8; 3], semantic: u8) -> u8 {
    local_to_semantic
        .iter()
        .position(|role| *role == semantic)
        .expect("semantic role exists") as u8
}

fn partial_surface(
    lineage: u8,
    local_to_semantic: [u8; 3],
    plane: u8,
    semantic_source: u8,
    semantic_target: u8,
    angle: f64,
) -> SurfaceFragmentBundle {
    let source = local_role(local_to_semantic, semantic_source);
    let target = local_role(local_to_semantic, semantic_target);
    SurfaceFragmentBundle::new(
        digest(lineage),
        digest(lineage.saturating_add(80)),
        vec![role(), role(), role()],
        vec![LocalRelationFragment {
            plane,
            source_local_role: source,
            target_local_role: target,
            state: TernaryRelationState::Supported,
            phase_anchor: phase(angle),
        }],
        vec![TypedProgramAtom {
            opcode: TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR,
            output_local_role: local_role(local_to_semantic, 2),
            source_a_local_role: local_role(local_to_semantic, 0),
            source_b_local_role: local_role(local_to_semantic, 0),
            parameter: TRANSFORM_VALUE_INTEGER,
            flags: 0,
        }],
    )
    .expect("valid partial surface")
}

fn complete_future_surface(lineage: u8, local_to_semantic: [u8; 3]) -> SurfaceFragmentBundle {
    let relations = [(0, 0, 1, 0.0), (1, 1, 2, 0.7), (2, 0, 2, 2.2)]
        .into_iter()
        .map(|(plane, source, target, angle)| LocalRelationFragment {
            plane,
            source_local_role: local_role(local_to_semantic, source),
            target_local_role: local_role(local_to_semantic, target),
            state: TernaryRelationState::Supported,
            phase_anchor: phase(angle),
        })
        .collect();
    SurfaceFragmentBundle::new(
        digest(lineage),
        digest(lineage.saturating_add(80)),
        vec![role(), role(), role()],
        relations,
        Vec::new(),
    )
    .expect("valid complete future surface")
}

#[test]
fn symmetric_partial_waves_crystallize_and_execute_only_with_full_phase() {
    let support = vec![
        partial_surface(1, [2, 0, 1], 0, 0, 1, 0.0),
        partial_surface(2, [0, 2, 1], 1, 1, 2, 0.7),
        partial_surface(3, [1, 0, 2], 2, 0, 2, 2.2),
    ];
    assert!(support.iter().all(|bundle| bundle.relations().len() == 1));

    let alignments = BoundedRoleAligner::align(&support, RoleAlignmentConfig::default());
    assert!(alignments.completion.is_complete());
    assert!(alignments.symmetric_branches > 0);
    let synthesis =
        BoundedCircuitBeam::synthesize(&support, &alignments, BlueprintBeamConfig::default());
    assert!(synthesis.completion.is_complete());
    assert!(synthesis.blueprints.len() >= 2);
    assert!(
        synthesis
            .blueprints
            .iter()
            .all(|blueprint| !blueprint.transform_program().is_empty())
    );

    let frozen = FrozenOperatorBlueprintSet::freeze(11, &support, Default::default(), &synthesis)
        .expect("complete competing version space");
    drop(support);
    let future = vec![
        complete_future_surface(21, [1, 0, 2]),
        complete_future_surface(22, [2, 1, 0]),
        complete_future_surface(23, [0, 2, 1]),
    ];

    let full = BlueprintFutureEvaluator::evaluate(
        &frozen,
        &future,
        Default::default(),
        BlueprintPhaseControl::Full,
    );
    let winner = full
        .winner_fingerprint_sha256
        .unwrap_or_else(|| panic!("full phase must select one topology: {full:#?}"));
    for control in [
        BlueprintPhaseControl::NoPhase,
        BlueprintPhaseControl::ShuffledPhase,
        BlueprintPhaseControl::MagnitudeOnly,
        BlueprintPhaseControl::MatchedRandomCenter,
    ] {
        let ablated =
            BlueprintFutureEvaluator::evaluate(&frozen, &future, Default::default(), control);
        assert_eq!(ablated.winner_fingerprint_sha256, None, "{control:?}");
    }

    let mut future_window = frozen.future_window();
    for bundle in &future {
        future_window
            .admit_lineage(bundle)
            .expect("independent future lineage");
    }
    let receipts = future
        .iter()
        .enumerate()
        .map(|(index, bundle)| {
            let value = 7 + index;
            CrystallizationParityReceipt {
                future_lineage_sha256: *bundle.lineage_sha256(),
                request_text: String::new(),
                provider_payload: json!({
                    "input": [{
                        "type":"function_call_output",
                        "output": format!("{{\"total\":{value}}}")
                    }]
                }),
                expected_response: value.to_string(),
            }
        })
        .collect::<Vec<_>>();
    let operator = CrystallizedOperator::crystallize(&future_window, &full, &receipts)
        .expect("causal winner crystallizes");

    assert_eq!(operator.blueprint_sha256(), &winner);
    assert_eq!(operator.verified_future_lineages().len(), 3);
    assert_eq!(
        operator
            .execute_verified("", &receipts[0].provider_payload)
            .as_deref(),
        Ok("7")
    );
}
