use super::*;
use nando_operator_kernel::sha256_bytes;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CandidateIdentity {
    capture_generation_root_sha256: String,
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
            let capture_v2 = capture_generation_v2(&joined);
            let generator_eligible = capture_v2 && generator_eligible(&joined, &factorized);
            let cohort_index = cohort_rows.entry(identity.clone()).or_default();
            let support_overflow = generator_eligible && *cohort_index >= K1_MAX_SUPPORT_ROWS_V1;
            if generator_eligible {
                *cohort_index = cohort_index.saturating_add(1);
            }
            let safety_veto = !generator_eligible || support_overflow;
            let row = if capture_v2 {
                K1NaturalEvidenceRowV1::seal(
                    joined.join_root_sha256.clone(),
                    identity.capture_generation_root_sha256,
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
            } else {
                K1NaturalEvidenceRowV1::seal_legacy_v1(
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
                    true,
                )
            }
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
        capture_generation_root_sha256: joined.capture_generation_root_sha256.clone(),
        candidate_structural_root_sha256: factorized.applicability_shape_root_sha256.clone(),
        source_neutral_topology_root_sha256: source_neutral_topology_root_v1(&joined.topology)
            .map_err(str::to_owned)?,
        semantic_novelty_signature_root_sha256: canonical_json_sha256(&(
            K1_SEMANTIC_NOVELTY_SCHEMA_V1,
            consequence_type,
        ))
        .map_err(str::to_owned)?,
        consequence_type,
    })
}

fn capture_generation_v2(joined: &BlindThenRevealJoinedTransitionV1) -> bool {
    joined.schema == nando_operator_learning::multi_source::BLIND_THEN_REVEAL_JOIN_SCHEMA_V2
        && capture_generation_v2_roots(
            &joined.extractor_root_sha256,
            &joined.extractor_config_root_sha256,
            &joined.capture_generation_root_sha256,
        )
}

fn capture_generation_v2_roots(extractor: &str, config: &str, generation: &str) -> bool {
    extractor == sha256_bytes(b"nando.multi-source-extractor.v2")
        && config == sha256_bytes(b"nando.multi-source-extractor-config.v2")
        && generation
            == canonical_json_sha256(&(
                nando_operator_learning::multi_source::MULTI_SOURCE_CAPTURE_GENERATION_SCHEMA_V2,
                extractor,
                config,
            ))
            .unwrap_or_default()
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
            | PreActionShapeClassV1::CollectionPlusScalarMetadata
            | PreActionShapeClassV1::MultipleCollections
    ) && matches!(
        factorized.completed_effect,
        CompletedEffectFormV1::SingleRoleProjection
            | CompletedEffectFormV1::MultiRoleRendering
            | CompletedEffectFormV1::StatusValueBranch
            | CompletedEffectFormV1::CollectionTransform
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

#[cfg(test)]
mod tests {
    use super::capture_generation_v2_roots;
    use nando_operator_kernel::{canonical_json_sha256, sha256_bytes};

    #[test]
    fn legacy_capture_roots_are_diagnostic_only() {
        let extractor_v2 = sha256_bytes(b"nando.multi-source-extractor.v2");
        let config_v2 = sha256_bytes(b"nando.multi-source-extractor-config.v2");
        let generation_v2 = canonical_json_sha256(&(
            nando_operator_learning::multi_source::MULTI_SOURCE_CAPTURE_GENERATION_SCHEMA_V2,
            extractor_v2.as_str(),
            config_v2.as_str(),
        ))
        .expect("generation root");
        assert!(capture_generation_v2_roots(
            &extractor_v2,
            &config_v2,
            &generation_v2
        ));
        assert!(!capture_generation_v2_roots(
            &sha256_bytes(b"nando.multi-source-extractor.v1"),
            &sha256_bytes(b"nando.multi-source-extractor-config.v1"),
            &generation_v2
        ));
    }
}
