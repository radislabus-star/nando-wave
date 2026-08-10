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

#[derive(Default)]
struct CohortSupportReservoir {
    selected_rows: usize,
    selected_lineages: BTreeSet<String>,
}

enum PendingEvidenceBinding {
    Fixed {
        binding: Box<EvidenceBinding>,
        identity: CandidateIdentity,
        generator_eligible: bool,
    },
    Eligible {
        joined: Box<BlindThenRevealJoinedTransitionV1>,
        identity: CandidateIdentity,
    },
}

impl PendingEvidenceBinding {
    fn order_key(&self) -> (u64, &str) {
        match self {
            Self::Fixed { binding, .. } => {
                (binding.row.capture_sequence, binding.join_root_sha256())
            }
            Self::Eligible { joined, .. } => {
                (joined.capture_sequence, joined.join_root_sha256.as_str())
            }
        }
    }
}

pub(in crate::k1_natural_scheduler_runtime) struct EvidenceBindingAccumulator {
    pending: Vec<PendingEvidenceBinding>,
    retain_safety_payloads: bool,
}

impl EvidenceBindingAccumulator {
    pub(in crate::k1_natural_scheduler_runtime) fn new(retain_safety_payloads: bool) -> Self {
        Self {
            pending: Vec::new(),
            retain_safety_payloads,
        }
    }

    pub(in crate::k1_natural_scheduler_runtime) fn push(
        &mut self,
        joined: BlindThenRevealJoinedTransitionV1,
    ) -> Result<(), String> {
        let factorized = factor_multi_source_row_v1(&joined);
        let identity = candidate_identity(&joined, &factorized)?;
        let generator_eligible = generator_eligible(&joined, &factorized);
        if capture_generation_v2(&joined) && generator_eligible {
            self.pending.push(PendingEvidenceBinding::Eligible {
                joined: Box::new(joined),
                identity,
            });
        } else {
            self.pending.push(PendingEvidenceBinding::Fixed {
                binding: Box::new(seal_evidence_binding(
                    joined,
                    identity.clone(),
                    true,
                    self.retain_safety_payloads,
                )?),
                identity,
                generator_eligible,
            });
        }
        Ok(())
    }

    pub(in crate::k1_natural_scheduler_runtime) fn finish(
        mut self,
    ) -> Result<Vec<EvidenceBinding>, String> {
        self.pending
            .sort_by(|left, right| left.order_key().cmp(&right.order_key()));
        let mut cohort_reservoirs = BTreeMap::<CandidateIdentity, CohortSupportReservoir>::new();
        let mut bindings = Vec::with_capacity(self.pending.len());
        for pending in self.pending {
            match pending {
                PendingEvidenceBinding::Fixed {
                    binding,
                    identity,
                    generator_eligible,
                } => {
                    let _ = support_reservoir_overflow(
                        cohort_reservoirs.entry(identity).or_default(),
                        &binding.row.lineage_root_sha256,
                        generator_eligible,
                    );
                    bindings.push(*binding);
                }
                PendingEvidenceBinding::Eligible { joined, identity } => {
                    let support_overflow = support_reservoir_overflow(
                        cohort_reservoirs.entry(identity.clone()).or_default(),
                        &joined.session_lineage_sha256,
                        true,
                    );
                    bindings.push(seal_evidence_binding(
                        *joined,
                        identity,
                        support_overflow,
                        self.retain_safety_payloads,
                    )?);
                }
            }
        }
        Ok(bindings)
    }
}

#[cfg(test)]
pub(in crate::k1_natural_scheduler_runtime) fn build_evidence_bindings(
    joined_rows: Vec<BlindThenRevealJoinedTransitionV1>,
) -> Result<Vec<EvidenceBinding>, String> {
    let mut accumulator = EvidenceBindingAccumulator::new(true);
    for joined in joined_rows {
        accumulator.push(joined)?;
    }
    accumulator.finish()
}

pub(in crate::k1_natural_scheduler_runtime) fn extend_evidence_bindings(
    bindings: &mut Vec<EvidenceBinding>,
    retain_safety_payloads: bool,
    mut joined_rows: Vec<BlindThenRevealJoinedTransitionV1>,
) -> Result<(), String> {
    joined_rows.sort_by(|left, right| {
        left.capture_sequence
            .cmp(&right.capture_sequence)
            .then_with(|| left.join_root_sha256.cmp(&right.join_root_sha256))
    });
    if let (Some(previous), Some(next)) = (bindings.last(), joined_rows.first())
        && (next.capture_sequence, next.join_root_sha256.as_str())
            <= (previous.row.capture_sequence, previous.join_root_sha256())
    {
        return Err("k1_incremental_evidence_out_of_order".to_owned());
    }

    let mut cohort_reservoirs = BTreeMap::<CandidateIdentity, CohortSupportReservoir>::new();
    for binding in bindings.iter().filter(|binding| !binding.row.safety_veto) {
        let reservoir = cohort_reservoirs
            .entry(candidate_identity_from_row(&binding.row))
            .or_default();
        reservoir.selected_rows = reservoir.selected_rows.saturating_add(1);
        reservoir
            .selected_lineages
            .insert(binding.row.lineage_root_sha256.clone());
    }

    for joined in joined_rows {
        let factorized = factor_multi_source_row_v1(&joined);
        let identity = candidate_identity(&joined, &factorized)?;
        let capture_v2 = capture_generation_v2(&joined);
        let generator_eligible = generator_eligible(&joined, &factorized);
        let support_overflow = support_reservoir_overflow(
            cohort_reservoirs.entry(identity.clone()).or_default(),
            &joined.session_lineage_sha256,
            generator_eligible,
        );
        bindings.push(seal_evidence_binding(
            joined,
            identity,
            !capture_v2 || !generator_eligible || support_overflow,
            retain_safety_payloads,
        )?);
    }
    Ok(())
}

fn seal_evidence_binding(
    joined: BlindThenRevealJoinedTransitionV1,
    identity: CandidateIdentity,
    safety_veto: bool,
    retain_safety_payloads: bool,
) -> Result<EvidenceBinding, String> {
    let capture_v2 = capture_generation_v2(&joined);
    let completed_frame_root_sha256 = joined.completed_frame_root_sha256.clone();
    let topology_commitment_root_sha256 = joined.topology_commitment_root_sha256.clone();
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
    let joined = (!safety_veto || retain_safety_payloads).then(|| Box::new(joined));
    Ok(EvidenceBinding {
        row,
        joined,
        completed_frame_root_sha256,
        topology_commitment_root_sha256,
    })
}

fn candidate_identity_from_row(row: &K1NaturalEvidenceRowV1) -> CandidateIdentity {
    CandidateIdentity {
        capture_generation_root_sha256: row.capture_generation_root_sha256.clone(),
        candidate_structural_root_sha256: row.candidate_structural_root_sha256.clone(),
        source_neutral_topology_root_sha256: row.source_neutral_topology_root_sha256.clone(),
        semantic_novelty_signature_root_sha256: row.semantic_novelty_signature_root_sha256.clone(),
        consequence_type: row.consequence_type,
    }
}

fn support_reservoir_overflow(
    reservoir: &mut CohortSupportReservoir,
    lineage_root_sha256: &str,
    generator_eligible: bool,
) -> bool {
    if !generator_eligible {
        return false;
    }
    if reservoir.selected_rows >= K1_MAX_SUPPORT_ROWS_V1 {
        return true;
    }

    let lineage_already_selected = reservoir.selected_lineages.contains(lineage_root_sha256);
    let selected_lineages = u64::try_from(reservoir.selected_lineages.len()).unwrap_or(u64::MAX);
    let reserved_lineage_slots =
        usize::try_from(K1_CANDIDATE_READINESS_MIN_LINEAGES_V1.saturating_sub(selected_lineages))
            .unwrap_or(K1_MAX_SUPPORT_ROWS_V1);
    let existing_lineage_limit = K1_MAX_SUPPORT_ROWS_V1.saturating_sub(reserved_lineage_slots);
    if lineage_already_selected
        && reserved_lineage_slots > 0
        && reservoir.selected_rows >= existing_lineage_limit
    {
        return true;
    }

    reservoir.selected_rows = reservoir.selected_rows.saturating_add(1);
    reservoir
        .selected_lineages
        .insert(lineage_root_sha256.to_owned());
    false
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
    let supported = [
        (
            sha256_bytes(b"nando.multi-source-extractor.v2"),
            sha256_bytes(b"nando.multi-source-extractor-config.v2"),
        ),
        (
            sha256_bytes(b"nando.multi-source-extractor.v3"),
            sha256_bytes(b"nando.multi-source-extractor-config.v3"),
        ),
    ]
    .into_iter()
    .any(|roots| roots.0 == extractor && roots.1 == config);
    supported
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
    use super::*;
    use crate::k1_natural_scheduler_runtime::K1_MAX_SUPPORT_ROWS_V1;
    use nando_operator_kernel::{
        MultiSourceCardinalityClassV1, MultiSourceRoleNodeV1, MultiSourceRoleWitnessV1,
        MultiSourceTemporalClassV1, PreActionMultiSourceTopologyV1, canonical_json_sha256,
        sha256_bytes,
    };
    use nando_operator_learning::multi_source::CompletedEffectAtomV1;

    fn root(value: u64) -> String {
        format!("{value:064x}")
    }

    fn joined(sequence: u64, lineage: u64) -> BlindThenRevealJoinedTransitionV1 {
        let extractor = sha256_bytes(b"nando.multi-source-extractor.v2");
        let config = sha256_bytes(b"nando.multi-source-extractor-config.v2");
        let generation = canonical_json_sha256(&(
            nando_operator_learning::multi_source::MULTI_SOURCE_CAPTURE_GENERATION_SCHEMA_V2,
            extractor.as_str(),
            config.as_str(),
        ))
        .expect("generation");
        BlindThenRevealJoinedTransitionV1 {
            schema: nando_operator_learning::multi_source::BLIND_THEN_REVEAL_JOIN_SCHEMA_V2
                .to_owned(),
            join_root_sha256: root(100 + sequence),
            capture_sequence: sequence,
            turn_intent_id_sha256: root(200 + sequence),
            request_event_id_sha256: root(300 + sequence),
            action_event_id_sha256: root(400 + sequence),
            session_lineage_sha256: root(lineage),
            session_id_sha256: root(500 + sequence),
            topology_commitment_root_sha256: root(600 + sequence),
            extractor_root_sha256: extractor,
            extractor_config_root_sha256: config,
            capture_generation_root_sha256: generation,
            pre_action_record_root_sha256: root(700 + sequence),
            completed_frame_root_sha256: root(800 + sequence),
            physical_action_root_sha256: root(900 + sequence),
            semantic_action_root_sha256: root(1_000 + sequence),
            effect_atoms: vec![CompletedEffectAtomV1::ValueProjection],
            verifier_receipt_root_sha256: root(1_100 + sequence),
            input_tokens: 10 + sequence,
            captured_at_unix_ms: 1_000 + sequence,
            completed_at_unix_nanos: (1_001 + sequence) * 1_000_000,
            accepted: true,
            topology: PreActionMultiSourceTopologyV1 {
                extraction_status: MultiSourceExtractionStatusV1::Complete,
                grounded_output_count: 1,
                output_part_count: 1,
                roles: vec![MultiSourceRoleNodeV1 {
                    local_role_id: 0,
                    source_ordinal: 0,
                    value_ordinal: 0,
                    type_class: MultiSourceTypeClassV1::Number,
                    container_class: MultiSourceContainerClassV1::Scalar,
                    cardinality_class: MultiSourceCardinalityClassV1::One,
                    temporal_class: MultiSourceTemporalClassV1::Latest,
                    depth_bucket: 1,
                    structural_flags: 1,
                }],
                role_witnesses: vec![MultiSourceRoleWitnessV1 {
                    local_role_id: 0,
                    value_sha256: root(1_200 + sequence),
                    request_reference_ordinal: None,
                    request_reference_ordinal_candidates: Vec::new(),
                }],
                relations: Vec::new(),
            },
        }
    }

    #[test]
    fn support_reservoir_preserves_a_slot_for_an_independent_lineage() {
        let mut reservoir = CohortSupportReservoir::default();
        for _ in 0..K1_MAX_SUPPORT_ROWS_V1 - 1 {
            assert!(!support_reservoir_overflow(
                &mut reservoir,
                "lineage-a",
                true
            ));
        }

        assert!(support_reservoir_overflow(
            &mut reservoir,
            "lineage-a",
            true
        ));
        assert!(!support_reservoir_overflow(
            &mut reservoir,
            "lineage-b",
            true
        ));
        assert_eq!(reservoir.selected_rows, K1_MAX_SUPPORT_ROWS_V1);
        assert_eq!(reservoir.selected_lineages.len(), 2);
        assert!(support_reservoir_overflow(
            &mut reservoir,
            "lineage-c",
            true
        ));
    }

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
        let extractor_v3 = sha256_bytes(b"nando.multi-source-extractor.v3");
        let config_v3 = sha256_bytes(b"nando.multi-source-extractor-config.v3");
        let generation_v3 = canonical_json_sha256(&(
            nando_operator_learning::multi_source::MULTI_SOURCE_CAPTURE_GENERATION_SCHEMA_V2,
            extractor_v3.as_str(),
            config_v3.as_str(),
        ))
        .expect("current generation root");
        assert!(capture_generation_v2_roots(
            &extractor_v3,
            &config_v3,
            &generation_v3
        ));
        assert!(!capture_generation_v2_roots(
            &sha256_bytes(b"nando.multi-source-extractor.v1"),
            &sha256_bytes(b"nando.multi-source-extractor-config.v1"),
            &generation_v2
        ));
    }

    #[test]
    fn incremental_evidence_matches_full_oracle_for_ordered_delta() {
        let first = joined(1, 1);
        let second = joined(2, 2);
        let oracle = build_evidence_bindings(vec![first.clone(), second.clone()]).expect("oracle");
        let mut incremental = build_evidence_bindings(vec![first]).expect("initial");
        extend_evidence_bindings(&mut incremental, true, vec![second]).expect("extend");
        assert_eq!(incremental, oracle);
        assert_eq!(
            evidence_epoch_root(&incremental).expect("incremental root"),
            evidence_epoch_root(&oracle).expect("oracle root")
        );
    }

    #[test]
    fn incremental_evidence_rejects_out_of_order_delta() {
        let mut incremental = build_evidence_bindings(vec![joined(2, 2)]).expect("initial");
        assert_eq!(
            extend_evidence_bindings(&mut incremental, true, vec![joined(1, 1)])
                .expect_err("fallback"),
            "k1_incremental_evidence_out_of_order"
        );
    }

    #[test]
    fn compact_bindings_preserve_roots_and_drop_only_veto_payloads() {
        let joined_rows = (1..=K1_MAX_SUPPORT_ROWS_V1 + 1)
            .map(|sequence| joined(sequence as u64, (sequence % 2) as u64 + 1))
            .collect::<Vec<_>>();
        let full = build_evidence_bindings(joined_rows.clone()).expect("full bindings");
        let mut accumulator = EvidenceBindingAccumulator::new(false);
        for row in joined_rows {
            accumulator.push(row).expect("compact row");
        }
        let compact = accumulator.finish().expect("compact bindings");

        assert_eq!(
            compact
                .iter()
                .map(|binding| &binding.row)
                .collect::<Vec<_>>(),
            full.iter().map(|binding| &binding.row).collect::<Vec<_>>()
        );
        assert_eq!(
            evidence_epoch_root(&compact).expect("compact root"),
            evidence_epoch_root(&full).expect("full root")
        );
        assert!(
            compact
                .iter()
                .filter(|binding| binding.row.safety_veto)
                .all(|binding| !binding.payload_retained())
        );
        assert!(
            compact
                .iter()
                .filter(|binding| !binding.row.safety_veto)
                .all(EvidenceBinding::payload_retained)
        );
        assert!(full.iter().all(EvidenceBinding::payload_retained));
    }
}
