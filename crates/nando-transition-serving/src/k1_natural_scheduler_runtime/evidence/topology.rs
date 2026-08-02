use super::*;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CandidateIdentity {
    candidate_structural_root_sha256: String,
    source_neutral_topology_root_sha256: String,
    semantic_novelty_signature_root_sha256: String,
    consequence_type: K1ConsequenceTypeV1,
}

pub(in crate::k1_natural_scheduler_runtime) fn build_evidence_bindings(
    joined_rows: &[BlindThenRevealJoinedTransitionV1],
) -> Result<Vec<EvidenceBinding>, String> {
    let mut prepared = joined_rows
        .iter()
        .map(|joined| {
            let factorized = factor_multi_source_row_v1(joined);
            let identity = candidate_identity(joined, &factorized)?;
            Ok((joined.clone(), factorized, identity))
        })
        .collect::<Result<Vec<_>, String>>()?;
    prepared.sort_by(|left, right| {
        left.0
            .capture_sequence
            .cmp(&right.0.capture_sequence)
            .then_with(|| left.0.join_root_sha256.cmp(&right.0.join_root_sha256))
    });
    let mut cohort_rows = BTreeMap::<CandidateIdentity, usize>::new();
    prepared
        .into_iter()
        .map(|(joined, factorized, identity)| {
            let generator_eligible = generator_eligible(&joined, &factorized);
            let cohort_index = cohort_rows.entry(identity.clone()).or_default();
            let support_overflow = generator_eligible && *cohort_index >= K1_MAX_SUPPORT_ROWS_V1;
            if generator_eligible {
                *cohort_index = cohort_index.saturating_add(1);
            }
            let safety_veto = !generator_eligible || support_overflow;
            let row = K1NaturalEvidenceRowV1::seal(
                joined.join_root_sha256.clone(),
                identity.candidate_structural_root_sha256,
                identity.source_neutral_topology_root_sha256,
                identity.semantic_novelty_signature_root_sha256,
                joined.session_lineage_sha256.clone(),
                identity.consequence_type,
                K1NaturalEvidenceClassV1::NaturalLive,
                joined.capture_sequence,
                joined.capture_sequence,
                joined.input_tokens,
                true,
                joined.accepted,
                safety_veto,
            )
            .map_err(str::to_owned)?;
            Ok(EvidenceBinding { row, joined })
        })
        .collect()
}

fn candidate_identity(
    joined: &BlindThenRevealJoinedTransitionV1,
    factorized: &FactorizedMultiSourceRowV1,
) -> Result<CandidateIdentity, String> {
    let consequence_type = consequence_type(joined, factorized.completed_effect);
    Ok(CandidateIdentity {
        candidate_structural_root_sha256: factorized.applicability_shape_root_sha256.clone(),
        source_neutral_topology_root_sha256: source_neutral_topology_root(joined)?,
        semantic_novelty_signature_root_sha256: canonical_json_sha256(&(
            K1_SEMANTIC_NOVELTY_SCHEMA_V1,
            consequence_type,
        ))
        .map_err(str::to_owned)?,
        consequence_type,
    })
}

fn source_neutral_topology_root(
    joined: &BlindThenRevealJoinedTransitionV1,
) -> Result<String, String> {
    let role_index = joined
        .topology
        .roles
        .iter()
        .enumerate()
        .map(|(index, role)| (role.local_role_id, index))
        .collect::<BTreeMap<_, _>>();
    let roles = joined
        .topology
        .roles
        .iter()
        .map(|role| {
            (
                role.type_class,
                role.container_class,
                role.cardinality_class,
                role.temporal_class,
                role.depth_bucket,
                role.structural_flags,
            )
        })
        .collect::<Vec<_>>();
    let relations = joined
        .topology
        .relations
        .iter()
        .map(|edge| {
            Ok((
                edge.relation,
                *role_index
                    .get(&edge.source_role_id)
                    .ok_or_else(|| "k1_runtime_topology_source_role_missing".to_owned())?,
                *role_index
                    .get(&edge.target_role_id)
                    .ok_or_else(|| "k1_runtime_topology_target_role_missing".to_owned())?,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    canonical_json_sha256(&(
        K1_SOURCE_NEUTRAL_TOPOLOGY_SCHEMA_V1,
        joined.topology.grounded_output_count,
        joined.topology.output_part_count,
        roles,
        relations,
    ))
    .map_err(str::to_owned)
}

fn consequence_type(
    joined: &BlindThenRevealJoinedTransitionV1,
    effect: CompletedEffectFormV1,
) -> K1ConsequenceTypeV1 {
    match effect {
        CompletedEffectFormV1::StatusValueBranch => K1ConsequenceTypeV1::Boolean,
        CompletedEffectFormV1::CollectionTransform => K1ConsequenceTypeV1::Collection,
        CompletedEffectFormV1::MultiRoleRendering => K1ConsequenceTypeV1::RenderedSequence,
        CompletedEffectFormV1::CrossOutputComposition => K1ConsequenceTypeV1::Record,
        CompletedEffectFormV1::SingleRoleProjection => {
            if joined
                .topology
                .roles
                .iter()
                .any(|role| !matches!(role.container_class, MultiSourceContainerClassV1::Scalar))
            {
                K1ConsequenceTypeV1::Collection
            } else if joined
                .topology
                .roles
                .iter()
                .any(|role| role.type_class == MultiSourceTypeClassV1::Boolean)
            {
                K1ConsequenceTypeV1::Boolean
            } else if joined
                .topology
                .roles
                .iter()
                .any(|role| role.type_class == MultiSourceTypeClassV1::Object)
            {
                K1ConsequenceTypeV1::Record
            } else {
                K1ConsequenceTypeV1::Scalar
            }
        }
        CompletedEffectFormV1::Unexplained => K1ConsequenceTypeV1::Record,
    }
}

fn generator_eligible(
    joined: &BlindThenRevealJoinedTransitionV1,
    factorized: &FactorizedMultiSourceRowV1,
) -> bool {
    matches!(
        factorized.pre_action_shape,
        PreActionShapeClassV1::SingleRoleProjection
            | PreActionShapeClassV1::OneOutputManyScalarRoles
            | PreActionShapeClassV1::ManyOutputsLatestRelevantRole
            | PreActionShapeClassV1::CrossOutputDependency
    ) && matches!(
        factorized.completed_effect,
        CompletedEffectFormV1::SingleRoleProjection
            | CompletedEffectFormV1::MultiRoleRendering
            | CompletedEffectFormV1::CrossOutputComposition
    ) && factorized.reason != MultiSourceReasonV1::Censored
        && matches!(
            joined.topology.extraction_status,
            MultiSourceExtractionStatusV1::Complete
        )
        && joined.topology.role_witnesses.len() == joined.topology.roles.len()
}

pub(in crate::k1_natural_scheduler_runtime) fn evidence_epoch_root(
    bindings: &[EvidenceBinding],
) -> Result<String, String> {
    canonical_json_sha256(&(
        "nando.k1-natural-evidence-epoch.v1",
        bindings
            .iter()
            .map(|binding| binding.row.row_root_sha256.as_str())
            .collect::<Vec<_>>(),
    ))
    .map_err(str::to_owned)
}

pub(in crate::k1_natural_scheduler_runtime) fn fixture_exclusion_root() -> Result<String, String> {
    canonical_json_sha256(&(
        K1_FIXTURE_EXCLUSION_SCHEMA_V1,
        "controlled_traffic_excluded_before_provider_capture",
        "generated_ms5_authority_forbidden",
        "generated_ms6_authority_forbidden",
        "fresh_live_provider_bound_verified_join_required",
    ))
    .map_err(str::to_owned)
}

pub(in crate::k1_natural_scheduler_runtime) fn generation_budget() -> K1GenerationBudgetV1 {
    K1GenerationBudgetV1 {
        maximum_support_rows: K1_MAX_SUPPORT_ROWS_V1 as u64,
        maximum_probe_rounds: K1_MAX_PROBE_ROUNDS_V1,
        maximum_probe_cost_units: K1_MAX_PROBE_COST_UNITS_V1,
        maximum_generation_seconds: K1_MAX_GENERATION_SECONDS_V1,
    }
}
