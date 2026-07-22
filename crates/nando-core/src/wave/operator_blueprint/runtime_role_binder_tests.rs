use super::*;

fn digest(byte: u8) -> Commitment256 {
    [byte; 32]
}

fn bounded_mapping_space(
    first_class_roles: usize,
    second_class_roles: usize,
) -> (RoleGraph, SurfaceFragmentBundle) {
    let first = StructuralRoleSignature::new(11, 1, 0, 0, Vec::new());
    let second = StructuralRoleSignature::new(12, 1, 0, 0, Vec::new());
    let canonical_roles = std::iter::repeat_n(first.clone(), first_class_roles)
        .chain(std::iter::repeat_n(second.clone(), second_class_roles))
        .collect::<Vec<_>>();
    let graph = RoleGraph::from_canonical_roles(canonical_roles).expect("bounded role graph");
    let bundle = SurfaceFragmentBundle::new(
        digest(41),
        digest(42),
        vec![first, second],
        Vec::new(),
        Vec::new(),
    )
    .expect("bounded mapping bundle");
    (graph, bundle)
}

#[test]
fn exact_mapping_cap_is_complete_only_when_no_frontier_remains() {
    let (graph, bundle) = bounded_mapping_space(8, 8);
    let (mappings, complete) = future_role_mappings(&bundle, &graph, 64);

    assert_eq!(mappings.len(), 64);
    assert!(complete);
}

#[test]
fn mapping_beyond_exact_cap_is_exhausted_without_truncation_claim() {
    let (graph, bundle) = bounded_mapping_space(9, 8);
    let (mappings, complete) = future_role_mappings(&bundle, &graph, 64);

    assert_eq!(mappings.len(), 64);
    assert!(!complete);
}

#[test]
fn runtime_binding_reports_all_structural_mappings_before_phase_winners() {
    let context = StructuralRoleSignature::new(9, 1, 0, 0, vec![0]);
    let source = StructuralRoleSignature::new(1, 1, 1, 0, vec![0]);
    let graph =
        RoleGraph::from_canonical_roles(vec![context.clone(), source.clone(), source.clone()])
            .expect("phase role graph");
    let circuit = OperatorCircuit::new(
        3,
        vec![
            OperatorCircuitRelation {
                cell: OperatorRelationCell {
                    plane: 0,
                    source_role: 0,
                    target_role: 1,
                },
                state: TernaryRelationState::Supported,
                phase_anchor: PhaseCenterCell { re: 1.0, im: 0.0 },
            },
            OperatorCircuitRelation {
                cell: OperatorRelationCell {
                    plane: 0,
                    source_role: 0,
                    target_role: 2,
                },
                state: TernaryRelationState::Supported,
                phase_anchor: PhaseCenterCell { re: 0.0, im: 1.0 },
            },
        ],
    )
    .expect("phase relation circuit");
    let bundle = SurfaceFragmentBundle::new(
        digest(43),
        digest(44),
        vec![context, source.clone(), source],
        vec![
            LocalRelationFragment {
                plane: 0,
                source_local_role: 0,
                target_local_role: 1,
                state: TernaryRelationState::Supported,
                phase_anchor: PhaseCenterCell { re: 1.0, im: 0.0 },
            },
            LocalRelationFragment {
                plane: 0,
                source_local_role: 0,
                target_local_role: 2,
                state: TernaryRelationState::Supported,
                phase_anchor: PhaseCenterCell { re: 0.0, im: 1.0 },
            },
        ],
        Vec::new(),
    )
    .expect("phase runtime bundle");

    let report = RuntimeRoleBinder::bind(&graph, &circuit, &bundle, 64);
    assert!(report.completion().is_complete());
    assert_eq!(report.structural_mappings().len(), 2);
    assert_eq!(report.phase_winner_mappings().len(), 1);
    assert_eq!(report.mappings(), report.phase_winner_mappings());
    assert_eq!(report.phase_runner_up_fit_fixed(), Some(0));
    assert_eq!(report.phase_margin_fixed(), Some(2_000_000_000));
}
