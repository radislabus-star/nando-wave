mod admission_bundle;
mod authority;
mod backward_wave;
mod causal;
mod cegis;
mod collection_synthesis;
mod contracts;
mod crystallized_operator;
mod decidability;
mod effect_graph;
mod evidence;
mod evidence_graph;
mod family_discovery;
mod grounding;
mod lifecycle;
mod online;
mod online_admission;
mod online_checkpoint;
mod online_collection;
mod online_state;
mod online_subcenter;
mod operator_generation;
mod operator_live_shadow;
mod opportunity;
mod outcome_example;
mod output_graph;
mod package;
mod program;
mod rollover;
mod runtime;
mod semantic_alias;
mod synthesis;
mod teacher_join;
mod training_types;
mod transferable_operator_v2;
mod verified_delta;
mod verifier;
mod version_space;

pub use admission_bundle::{
    DURABLE_RUNTIME_PARITY_RECEIPT_SCHEMA_V1, DurableRuntimeParityReceipt,
    ONLINE_ADMISSION_CANDIDATE_BUNDLE_SCHEMA_V1, OnlineAdmissionCandidateBundle, RuntimeParityCase,
};
pub use authority::{
    COMPOSITE_ADMISSION_SCHEMA_V2, CompositeResponseAdmissionV2, FinalizedPostVerifierReceiptV1,
    FinalizedRuntimeVerificationReceiptV2, PostVerifierReceiptV1, RESPONSE_AUTHORITY_SCHEMA_V2,
    RESPONSE_EXACT_CAUSAL_PROOF_SCHEMA_V2, RESPONSE_EXECUTION_PAYLOAD_SCHEMA_V1,
    RESPONSE_FUTURE_VERIFIER_RECEIPT_SCHEMA_V2, RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2,
    RESPONSE_POST_VERIFIER_ADMISSION_SCHEMA_V1, RESPONSE_POST_VERIFIER_RECEIPT_SCHEMA_V1,
    RESPONSE_PROOF_RECEIPT_BINDING_SCHEMA_V1, RESPONSE_REGISTRY_SCHEMA_V6,
    RESPONSE_RUNTIME_CONTRACT_SCHEMA_V1, RESPONSE_RUNTIME_PARITY_RECEIPT_SET_SCHEMA_V1,
    RESPONSE_RUNTIME_RECEIPT_SCHEMA_V2, RESPONSE_SEMANTIC_ALIAS_PROOF_SCHEMA_V1,
    RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1, ResponseAuthorityV2, ResponsePackageAuthorityBindingV2,
    RuntimeVerificationReceiptV2, RuntimeVerificationResultV2, canonical_json_bytes,
    canonical_json_sha256, finalize_post_verifier_receipt, response_actor_program_digest,
    response_execution_payload_digest, response_independent_verifier_program_digest,
    response_package_digest, response_proof_receipts_digest, response_registry_digest,
    response_runtime_contract_sha256, sha256_bytes, valid_nonzero_sha256,
};
pub use backward_wave::{BackwardWave, BackwardWaveError, BackwardWaveUpdate};
pub use causal::{
    GroundedWaveCausalReport, evaluate_grounded_wave_causality,
    evaluate_grounded_wave_causality_refs,
};
pub use cegis::{
    CegisCoordinator, CegisCounterexample, CegisPoolReport, CegisReport, CegisWinner,
    CounterexampleKind, RepairAction,
};
pub use collection_synthesis::{
    CollectionSynthesisExample, CollectionVersionSpace, ResponseCoverageDiagnostic,
    SynthesizedCollectionProgram, collection_verifier_for_program,
    diagnose_response_dynamic_coverage, enumerate_source_neutral_collection_programs,
    enumerate_source_neutral_response_programs, is_learned_bounded_response_program,
    is_privacy_safe_online_response_program, is_source_neutral_collection_program,
    is_source_neutral_response_program, response_program_authority_matches_example,
    response_program_exactly_matches_example, response_program_matches_example,
    source_neutral_verifier_for_program, synthesize_collection_program,
    synthesize_unique_collection_program,
};
pub use contracts::{
    AtomSource, AtomValueType, FrozenSplitError, GuardCandidate, INGRESS_EVENT_SCHEMA,
    IngressEvent, PROGRAM_CANDIDATE_SCHEMA, RELATION_FRAME_SCHEMA, ROLE_HYPOTHESIS_SCHEMA,
    RelationAtom, RelationFrame, ResponseProgramCandidate, ResponseValueSelector, RoleHypothesis,
    SemanticRole, TrafficClass, VERIFIER_RECEIPT_SCHEMA, VerifierConsensusVariant, VerifierProgram,
    VerifierReceipt, validate_frozen_future_split,
};
pub use crystallized_operator::{
    BoundCrystallizedOperator, BoundRoleEnvironment, CrystallizationParityReceipt,
    CrystallizedFeedbackError, CrystallizedOperator, CrystallizedOperatorError,
    ExecutableParitySeal, RuntimeRoleAnchor, RuntimeSurfaceEvidence, TRANSFORM_FLAG_CANONICAL_JSON,
    TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR, TRANSFORM_ROLE_NONE, TRANSFORM_VALUE_BOOLEAN,
    TRANSFORM_VALUE_IDENTIFIER, TRANSFORM_VALUE_INTEGER, TRANSFORM_VALUE_STRING,
    VerifiedCrystallizedOperator, VerifiedOperatorRestartBundle,
};
pub use decidability::{CpuDecidability, CpuDecidabilityClass, classify_cpu_decidability};
pub use effect_graph::{
    EFFECT_GRAPH_SCHEMA_V1, EffectEdge, EffectEdgeKind, EffectGraph, EffectGraphBuilder,
    EffectGraphCompleteness, EffectGraphPolicy, EffectNode, EffectNodeKind, EffectOperationKind,
    EffectSource,
};
pub use evidence::{
    CanonicalEventGraph, CanonicalEventNode, DeterministicEvidenceLedger,
    EVIDENCE_LEDGER_SCHEMA_V1, EVIDENCE_POLICY_VERSION, EvidenceAccounting, EvidenceEventTime,
    EvidenceIngestOutcome, EvidenceKey, EvidenceLedgerRecord, EvidencePolicyV1, EvidenceRejection,
    RawEvidenceEnvelope, canonicalize_evidence_envelope, evidence_payload_sha256,
    evidence_session_id_sha256,
};
pub use evidence_graph::{
    DeterministicEvidenceGraphStore, EvidenceGraph, EvidenceGraphAtom, EvidenceGraphBuilder,
    EvidenceGraphPolicy, EvidenceGraphRecord, EvidenceGraphStoreStatus, EvidenceNodeRef,
    EvidenceNodeType,
};
pub use family_discovery::{
    CrossSurfaceFamilyDiscovery, FamilyDiscoveryConfig, FamilyDiscoveryReport, RuntimeInvariant,
    TeacherPoolReport, TeacherPoolSnapshot,
};
pub use grounding::{
    SOURCE_NEUTRAL_EXTRACTOR_VERSION, SourceNeutralTrace, TraceSlot, extract_relation_frame,
    ground_roles, is_source_neutral_relation_frame, relation_frame_online_family_id,
    relation_frame_structural_family_id,
};
pub use lifecycle::{
    FrameRepresentationPolicy, GROUNDED_RESPONSE_PACKAGE_PREFIX, ROUTING_REFINEMENT_VERSION,
    ResponseProgramHint, ResponseRelationObservation, ResponseShadowObservation,
    ResponseSupportFreezePolicy, ResponseSupportManifest, ResponseSupportManifestSet,
    compile_response_registry, compile_source_neutral_quarantine_packages,
    frame_matches_program_action_contract, frame_matches_program_action_contract_with_grounding,
    frame_representation_matches_support, freeze_source_neutral_support,
    freeze_source_neutral_support_with_policy, grounded_response_package_id,
    response_package_lineage_id, response_support_manifest_digest,
};
pub use online::{
    OnlineResponseActionFamilyReport, OnlineResponseAdmissionCandidate, OnlineResponseBucketReport,
    OnlineResponseCandidate, OnlineResponseIngestResult, OnlineResponseMiner,
    OnlineResponseMinerConfig, OnlineResponseMinerReport, OnlineResponseStream,
    OnlineResponseStreamStatus, OnlineResponseTailConfig, run_online_response_tail,
};
pub use online_admission::{
    OnlineAdmissionSnapshot, build_crystallized_admission_snapshot,
    build_durable_runtime_parity_receipt, build_online_admission_snapshot,
    build_online_collection_admission_snapshot, merge_online_admission_snapshots,
};
pub use online_checkpoint::{
    FramedCborLedger, FramedLedgerStatus, FramedRecordRef, read_framed_cbor, write_atomic_cbor,
};
pub use online_collection::{
    LegacyReplayRehydrationStats, OnlineCollectionAdmissionCandidate, OnlineCollectionBucketStatus,
    OnlineCollectionConfig, OnlineCollectionConsensusDiagnostic, OnlineCollectionMiner,
    OnlineCollectionObservation, OnlineCollectionReceipt, OnlineCollectionRehydrationHint,
    OnlineCollectionStatus, OnlineCollectionWaveCausalReport,
    online_collection_future_manifest_digest, online_collection_support_manifest_digest,
};
pub use online_state::{
    MinerSignalStageReport, MinerSignalTreeReport, SELF_TRAINING_STATE_SCHEMA_V2,
    SELF_TRAINING_STATE_SCHEMA_V3, SelfTrainingAdmissionCohort, SelfTrainingGenerationReport,
    SelfTrainingStateReport, StreamingSelfTrainingState,
};
pub use operator_generation::{
    AdmissionReadyOperatorGeneration, OperatorGenerationError, OperatorGenerationFirewall,
};
pub use operator_live_shadow::{
    LiveScalarAdmissionCandidate, LiveScalarCircuitSample, LiveScalarShadowBlocker,
    LiveScalarShadowReport, LiveScalarShadowState, extract_live_scalar_circuit_sample,
};
pub use opportunity::{
    M3WindowReport, OPPORTUNITY_BOARD_SCHEMA_V2, OPPORTUNITY_BOARD_SCHEMA_V3, OpportunityBoard,
    OpportunityBoardConfig, OpportunityBoardReport, OpportunityClassReport, ReducibilityClass,
    TeacherOpportunityReport,
};
pub use outcome_example::{
    COMPLETED_TURN_EXAMPLE_SCHEMA_V1, COMPLETED_TURN_EXAMPLE_SCHEMA_V2, CompletedTurnExample,
    CompletedTurnRuntime, CompletedTurnTeacher, TrainingTarget, TurnCompletionReason,
};
pub use output_graph::{
    OutputGraph, OutputGraphSegment, OutputValueCandidate, OutputValueSource, build_output_graph,
};
pub use package::{
    COLLECTION_EXTERNAL_VERIFIER_SCHEMA, CONTINUATION_EXTERNAL_VERIFIER_SCHEMA,
    CUSTOM_TOOL_EXTERNAL_VERIFIER_SCHEMA, LearnedWaveRoute, LearnedWaveSubcenter, ResponseExecutor,
    ResponsePackage, ResponsePackageOrigin, ResponsePackageProof, ResponsePackageState,
    ResponseRegistry, ResponseRoutingComparison, ResponseRoutingPredicate, RoutedResponseExecution,
    SOURCE_VALUE_EXTERNAL_VERIFIER_SCHEMA, VALUE_PROJECTION_EXTERNAL_VERIFIER_SCHEMA,
    provider_tool_capability_atom_ids, relation_frame_online_routing_atom_ids,
    relation_frame_phase_atom_ids, relation_frame_phase_margin_micro,
    relation_frame_required_observable_atom_ids, relation_frame_routes_to_package,
    relation_frame_routing_atom_ids, request_phase_atom_ids,
    response_program_external_verifier_schema, response_program_required_routing_atom_ids,
};
pub(crate) use package::{response_pre_action_context_atom_ids, stable_atom_id};
pub use program::{
    CollectionAggregateOperation, CollectionOutputRenderer, CollectionProgramStep,
    CollectionScalarType, CustomToolResultProjection, MAX_PROJECT_STATUS_CODE,
    ProjectStatusMapping, ProjectStatusValue, RequestTemplateMarker, ResponseAdapterWaveConsensus,
    ResponseAdapterWaveRoute, ResponseAdapterWaveSubcenter, ResponseArgument,
    ResponseConsensusVariant, ResponseOperation, ResponseProgram, ResponseRenderSegment,
    ResponseScalarLiteral, ValueProjectionFormat,
};
pub use rollover::{
    FROZEN_PARTITION_VERSION, FrozenGeneration, RolloverPolicy, freeze_generation,
    generation_monotonically_improves, refresh_frozen_generation, successor_generation,
};
pub use runtime::{ResponseExecution, ResponseExecutionStatus, execute_response};
pub use semantic_alias::{
    SEMANTIC_ALIAS_GRAPH_SCHEMA_V1, SemanticAliasEdge, SemanticAliasGraph, SemanticAliasReport,
    SemanticAliasState, SemanticEffectEvidence,
};
pub use synthesis::{
    SynthesisError, SynthesizedResponseOperator, partition_teacher_training_families,
    synthesize_response_operator, verify_operator_structure,
};
pub use teacher_join::{
    TeacherJoin, TeacherJoinKey, TeacherJoinRejection, TeacherJoinReport, teacher_action_ast,
    teacher_action_symbol, teacher_join_key, teacher_outcome_from_completed,
    teacher_program_signature, teacher_program_signature_from_action_atoms,
    teacher_semantic_law_signature, teacher_transfer_family_signature,
    teacher_transition_from_completed,
};
pub use training_types::{
    ECONOMICS_RECEIPT_SCHEMA_V1, EconomicsReceipt, RUNTIME_FRAME_SCHEMA_V1, RuntimeFrame,
    TEACHER_OUTCOME_SCHEMA_V1, TEACHER_TRANSITION_SCHEMA_V1, TeacherActionAst, TeacherOutcome,
    TeacherTransition, TeacherVerifierEvidence, relation_atom_is_teacher_only,
    relation_frame_learning_digest,
};
pub use transferable_operator_v2::{
    ProvenTransferableOperatorV2, ShadowTransferableOperatorV2, TransferableOperatorV2Error,
};
pub use verified_delta::{
    TYPED_EXECUTION_STAGE_RECEIPT_SCHEMA_V1, TypedExecutionStage, TypedExecutionStageReceipt,
    VERIFIED_DELTA_MAX_RELATIONS, VERIFIED_DELTA_RECEIPT_SCHEMA_V1, VerifiedDeltaError,
    VerifiedDeltaOutcome, VerifiedDeltaReceipt, VerifiedDeltaRelation, VerifiedDeltaRelationState,
};
pub use verifier::{ResponseVerificationError, verify_response, verify_response_independently};
pub use version_space::{
    AstNodeId, AstProgramKind, InternedProgram, VersionSpaceArena, VersionSpaceConfig,
    VersionSpaceReport, response_program_depth, response_program_kind,
};

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;

    fn relation_frame(
        frame: &str,
        intent: &str,
        session: &str,
        observed_at_unix_nanos: u64,
    ) -> RelationFrame {
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: frame.repeat(64),
            event_id_sha256: "e".repeat(64),
            client_intent_id_sha256: intent.repeat(64),
            session_id_sha256: session.repeat(64),
            observed_at_unix_nanos,
            estimated_input_tokens: 0,
            extractor_version: "source-neutral-v1".to_owned(),
            verifier_label: None,
            atoms: vec![RelationAtom::CompletionState {
                value: "completed".to_owned(),
            }],
            evidence_ref_sha256: "a".repeat(64),
        }
    }

    #[test]
    fn p0_contracts_roundtrip_without_raw_content_fields() {
        let ingress = IngressEvent {
            schema: INGRESS_EVENT_SCHEMA.to_owned(),
            event_id_sha256: "a".repeat(64),
            client_intent_id_sha256: "b".repeat(64),
            session_id_sha256: "c".repeat(64),
            parent_event_id_sha256: None,
            observed_at_unix_nanos: 10,
            traffic_class: TrafficClass::Ordinary,
            request_shape_sha256: "d".repeat(64),
            evidence_ref_sha256: Some("e".repeat(64)),
            input_tokens: 42,
        };
        let encoded = serde_json::to_vec(&ingress).expect("serialize ingress");
        let decoded: IngressEvent = serde_json::from_slice(&encoded).expect("parse ingress");
        assert_eq!(decoded, ingress);
        let text = String::from_utf8(encoded).expect("utf8");
        for forbidden in [
            "raw_request",
            "raw_response",
            "prompt",
            "tool_output",
            "secret",
        ] {
            assert!(!text.contains(forbidden));
        }

        let candidate = ResponseProgramCandidate {
            schema: PROGRAM_CANDIDATE_SCHEMA.to_owned(),
            candidate_id_sha256: "f".repeat(64),
            role_hypothesis_id_sha256: "1".repeat(64),
            program: ResponseProgram::wait_on_yielded_surfaces(["service_observation"]),
            guard: GuardCandidate {
                required_atom_indices: vec![0],
                forbidden_atom_indices: vec![],
                require_unique_selector: true,
                max_evidence_age_ms: 30_000,
            },
            phase_rank: 0,
            exact_checks: 1,
            description_length_bytes: 128,
        };
        let roundtrip: ResponseProgramCandidate =
            serde_json::from_value(serde_json::to_value(&candidate).expect("candidate value"))
                .expect("candidate roundtrip");
        assert_eq!(roundtrip, candidate);

        let roles = RoleHypothesis {
            schema: ROLE_HYPOTHESIS_SCHEMA.to_owned(),
            hypothesis_id_sha256: "2".repeat(64),
            frame_family_id: 7,
            bindings: BTreeMap::from([(SemanticRole::ContinuationHandle, 0)]),
            competing_binding_count: 0,
            description_length_bytes: 32,
        };
        assert_eq!(
            serde_json::from_value::<RoleHypothesis>(
                serde_json::to_value(&roles).expect("roles value")
            )
            .expect("roles roundtrip"),
            roles
        );
    }

    #[test]
    fn p0_frozen_future_rejects_session_and_intent_leakage() {
        let support = vec![relation_frame("1", "2", "3", 10)];
        let future = vec![relation_frame("4", "5", "6", 20)];
        assert_eq!(validate_frozen_future_split(&support, &future, 10), Ok(()));

        let same_session = vec![relation_frame("4", "5", "3", 20)];
        assert_eq!(
            validate_frozen_future_split(&support, &same_session, 10),
            Err(FrozenSplitError::SessionLeakage)
        );
        let same_intent = vec![relation_frame("4", "2", "6", 20)];
        assert_eq!(
            validate_frozen_future_split(&support, &same_intent, 10),
            Err(FrozenSplitError::IntentLeakage)
        );
    }

    fn continuation_trace(
        observation_slot: u16,
        action_slot: u16,
        reverse_layout: bool,
    ) -> SourceNeutralTrace {
        let mut slots = vec![
            TraceSlot {
                slot_id: observation_slot,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Observation,
                value_sha256: "a".repeat(64),
            },
            TraceSlot {
                slot_id: action_slot,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Action,
                value_sha256: "a".repeat(64),
            },
        ];
        if reverse_layout {
            slots.reverse();
        }
        SourceNeutralTrace {
            event_id_sha256: "1".repeat(64),
            client_intent_id_sha256: "2".repeat(64),
            session_id_sha256: "3".repeat(64),
            observed_at_unix_nanos: 10,
            verifier_label: Some(true),
            evidence_ref_sha256: "4".repeat(64),
            tool_kind: "process_cell".to_owned(),
            observation_call_shape: "function_call".to_owned(),
            observation_selector: ResponseValueSelector::ContentLinePrefix {
                prefix: "Script running with cell ID ".to_owned(),
                value_type: AtomValueType::Identifier,
            },
            completion_state: "pending".to_owned(),
            output_status: None,
            response_shape: "function_call".to_owned(),
            slots,
            equal_slots: vec![(observation_slot, action_slot)],
            unique_slots: vec![observation_slot],
            action_function: Some("wait".to_owned()),
            action_role_arguments: vec![("cell_id".to_owned(), action_slot)],
            action_integer_arguments: vec![
                ("yield_time_ms".to_owned(), 1_000),
                ("max_tokens".to_owned(), 5_000),
            ],
        }
    }

    fn scalar_transfer_trace(
        observation_slot: u16,
        action_slot: u16,
        reverse_layout: bool,
        tool_kind: &str,
    ) -> SourceNeutralTrace {
        let mut slots = vec![
            TraceSlot {
                slot_id: observation_slot,
                value_type: AtomValueType::String,
                source: AtomSource::Observation,
                value_sha256: "5".repeat(64),
            },
            TraceSlot {
                slot_id: action_slot,
                value_type: AtomValueType::String,
                source: AtomSource::Action,
                value_sha256: "5".repeat(64),
            },
        ];
        if reverse_layout {
            slots.reverse();
        }
        SourceNeutralTrace {
            event_id_sha256: "6".repeat(64),
            client_intent_id_sha256: "7".repeat(64),
            session_id_sha256: "8".repeat(64),
            observed_at_unix_nanos: 20,
            verifier_label: Some(true),
            evidence_ref_sha256: "9".repeat(64),
            tool_kind: tool_kind.to_owned(),
            observation_call_shape: "function_call".to_owned(),
            observation_selector: ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::String,
            },
            completion_state: "completed".to_owned(),
            output_status: None,
            response_shape: "function_call".to_owned(),
            slots,
            equal_slots: vec![(observation_slot, action_slot)],
            unique_slots: vec![observation_slot],
            action_function: Some("route_result".to_owned()),
            action_role_arguments: vec![("result".to_owned(), action_slot)],
            action_integer_arguments: vec![("limit".to_owned(), 4)],
        }
    }

    fn custom_continuation_frame(
        frame_marker: char,
        session_marker: char,
        observation_slot: u16,
        action_slot: u16,
        tool_kind: &str,
    ) -> RelationFrame {
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: frame_marker.to_string().repeat(64),
            event_id_sha256: frame_marker.to_ascii_uppercase().to_string().repeat(64),
            client_intent_id_sha256: frame_marker.to_string().repeat(64),
            session_id_sha256: session_marker.to_string().repeat(64),
            observed_at_unix_nanos: 30,
            estimated_input_tokens: 0,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            atoms: vec![
                RelationAtom::ToolKind {
                    value: tool_kind.to_owned(),
                },
                RelationAtom::ObservationCallShape {
                    value: "custom_tool_call".to_owned(),
                },
                RelationAtom::CompletionState {
                    value: "pending".to_owned(),
                },
                RelationAtom::ResponseShape {
                    value: "custom_tool_call".to_owned(),
                },
                RelationAtom::ClientCapabilityAtom {
                    atom_id: package::stable_atom_id("client_capability:custom:exec"),
                },
                RelationAtom::TypedSlot {
                    slot_id: observation_slot,
                    value_type: AtomValueType::Integer,
                    source: AtomSource::Observation,
                    value_sha256: "5".repeat(64),
                },
                RelationAtom::UniqueSlot {
                    slot_id: observation_slot,
                },
                RelationAtom::ObservationSelector {
                    slot_id: observation_slot,
                    selector: ResponseValueSelector::ContentLinePrefix {
                        prefix: "SESSION_ID=".to_owned(),
                        value_type: AtomValueType::Integer,
                    },
                },
                RelationAtom::TypedSlot {
                    slot_id: action_slot,
                    value_type: AtomValueType::Integer,
                    source: AtomSource::Action,
                    value_sha256: "5".repeat(64),
                },
                RelationAtom::SlotEquality {
                    left_slot: observation_slot,
                    right_slot: action_slot,
                },
                RelationAtom::ActionCustomTool {
                    value: "exec".to_owned(),
                },
                RelationAtom::ActionInnerTool {
                    value: "write_stdin".to_owned(),
                },
                RelationAtom::ActionRoleArgument {
                    name: "session_id".to_owned(),
                    slot_id: action_slot,
                    value_type: None,
                },
                RelationAtom::ActionStringArgument {
                    name: "chars".to_owned(),
                    value: String::new(),
                },
                RelationAtom::ActionIntegerArgument {
                    name: "yield_time_ms".to_owned(),
                    value: 30_000,
                },
                RelationAtom::ActionIntegerArgument {
                    name: "max_output_tokens".to_owned(),
                    value: 12_000,
                },
                RelationAtom::ActionResultProjection {
                    output_field: "output".to_owned(),
                    continuation_field: "session_id".to_owned(),
                    continuation_prefix: "SESSION_ID=".to_owned(),
                },
                RelationAtom::Cardinality {
                    role: "turn_call_count_band".to_owned(),
                    count: 1,
                },
                RelationAtom::Cardinality {
                    role: "turn_output_count_band".to_owned(),
                    count: 1,
                },
                RelationAtom::Cardinality {
                    role: "turn_pending_count_band".to_owned(),
                    count: 0,
                },
                RelationAtom::Cardinality {
                    role: "turn_message_count_band".to_owned(),
                    count: 0,
                },
                RelationAtom::Cardinality {
                    role: "turn_call_shape_count_band".to_owned(),
                    count: 1,
                },
            ],
            evidence_ref_sha256: "f".repeat(64),
        }
    }

    #[test]
    fn p1_role_grounding_transfers_across_slot_renames_and_layouts() {
        let mut renamed_surface = continuation_trace(101, 4, true);
        renamed_surface.tool_kind = "unseen_runtime_surface".to_owned();
        let frames = [
            extract_relation_frame(&continuation_trace(7, 9, false)),
            extract_relation_frame(&renamed_surface),
        ];
        let mut family_id = None;
        for frame in frames {
            let hypotheses = ground_roles(&frame);
            assert_eq!(hypotheses.len(), 1);
            assert!(
                hypotheses[0]
                    .bindings
                    .contains_key(&SemanticRole::ContinuationHandle)
            );
            assert_eq!(hypotheses[0].competing_binding_count, 0);
            assert_eq!(
                *family_id.get_or_insert(hypotheses[0].frame_family_id),
                hypotheses[0].frame_family_id
            );
            let serialized = serde_json::to_string(&frame).expect("frame json");
            assert!(!serialized.contains("field_name"));
            assert!(!serialized.contains("program_hint"));
        }
    }

    #[test]
    fn p1_role_swap_and_ambiguity_do_not_gain_authority() {
        let mut swapped = continuation_trace(7, 9, false);
        swapped.slots[0].source = AtomSource::Action;
        swapped.slots[1].source = AtomSource::Observation;
        assert!(ground_roles(&extract_relation_frame(&swapped)).is_empty());

        let mut ambiguous = continuation_trace(7, 9, false);
        ambiguous.slots.extend([
            TraceSlot {
                slot_id: 11,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Observation,
                value_sha256: "b".repeat(64),
            },
            TraceSlot {
                slot_id: 12,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Action,
                value_sha256: "b".repeat(64),
            },
        ]);
        ambiguous.equal_slots.push((11, 12));
        let hypotheses = ground_roles(&extract_relation_frame(&ambiguous));
        assert_eq!(hypotheses.len(), 2);
        assert!(
            hypotheses
                .iter()
                .all(|hypothesis| hypothesis.competing_binding_count == 1)
        );
    }

    #[test]
    fn p2_synthesizes_program_guard_and_independent_verifier_from_frames() {
        let frames = [
            extract_relation_frame(&continuation_trace(7, 9, false)),
            extract_relation_frame(&continuation_trace(101, 4, true)),
        ];
        let operator = synthesize_response_operator(&frames).expect("synthesize operator");
        assert!(matches!(
            operator.candidate.program.operation,
            ResponseOperation::FunctionCallFromRoles { .. }
        ));
        assert!(operator.candidate.guard.require_unique_selector);
        assert!(operator.candidate.exact_checks >= frames.len() as u32);
        assert!(
            frames
                .iter()
                .all(|frame| verify_operator_structure(frame, &operator))
        );
        let serialized = serde_json::to_string(&operator).expect("operator json");
        assert!(!serialized.contains("program_hint"));
        assert!(!serialized.contains("field_name"));
    }

    #[test]
    fn p2_discovers_unseen_function_symbols_arguments_and_constants() {
        for (function, role_name, delay_name, delay, budget_name, budget) in [
            ("resume_job", "job_ref", "delay_ms", 750, "budget", 4_000),
            ("poll_task", "task_key", "poll_ms", 1_250, "limit", 6_000),
            (
                "continue_run",
                "run_token",
                "interval_ms",
                2_000,
                "token_cap",
                8_000,
            ),
        ] {
            let mut traces = [
                continuation_trace(7, 9, false),
                continuation_trace(101, 4, true),
            ];
            for (index, trace) in traces.iter_mut().enumerate() {
                trace.action_function = Some(function.to_owned());
                trace.action_role_arguments =
                    vec![(role_name.to_owned(), if index == 0 { 9 } else { 4 })];
                trace.action_integer_arguments = vec![
                    (delay_name.to_owned(), delay),
                    (budget_name.to_owned(), budget),
                ];
            }
            let frames = traces.map(|trace| extract_relation_frame(&trace));
            let operator = synthesize_response_operator(&frames).expect("generic synthesis");
            let ResponseOperation::FunctionCallFromRoles {
                function_name,
                arguments,
                ..
            } = &operator.candidate.program.operation
            else {
                panic!("generic program required");
            };
            assert_eq!(function_name, function);
            assert!(arguments.contains(&ResponseArgument::Role {
                name: role_name.to_owned(),
                role: SemanticRole::ContinuationHandle,
                value_type: None,
            }));
            assert!(arguments.contains(&ResponseArgument::Integer {
                name: delay_name.to_owned(),
                value: delay,
            }));
            assert!(arguments.contains(&ResponseArgument::Integer {
                name: budget_name.to_owned(),
                value: budget,
            }));
            let payload = json!({
                "input": [{
                    "type": "function_call_output",
                    "output": "Script running with cell ID unseen-handle\n",
                }]
            });
            let execution = execute_response(&operator.candidate.program, "", &payload);
            assert_eq!(execution.status, ResponseExecutionStatus::Executed);
            let response = execution.response.expect("generic response");
            assert!(verify_response_independently(&operator.verifier, &payload, &response).is_ok());
            let mutated = response.replace(function, "wrong_function");
            assert!(verify_response_independently(&operator.verifier, &payload, &mutated).is_err());
        }
    }

    #[test]
    fn p2_transfers_completed_scalar_roles_and_abstains_on_ambiguity() {
        let mut frames = [
            extract_relation_frame(&scalar_transfer_trace(3, 8, false, "json_tool")),
            extract_relation_frame(&scalar_transfer_trace(101, 4, true, "unseen_tool")),
        ];
        for frame in &mut frames {
            frame.atoms.extend([
                RelationAtom::Cardinality {
                    role: "turn_call_count_band".to_owned(),
                    count: 1,
                },
                RelationAtom::Cardinality {
                    role: "turn_output_count_band".to_owned(),
                    count: 1,
                },
                RelationAtom::Cardinality {
                    role: "turn_pending_count_band".to_owned(),
                    count: 0,
                },
                RelationAtom::Cardinality {
                    role: "turn_message_count_band".to_owned(),
                    count: 0,
                },
                RelationAtom::Cardinality {
                    role: "turn_call_shape_count_band".to_owned(),
                    count: 1,
                },
            ]);
        }
        for frame in &frames {
            let hypotheses = ground_roles(frame);
            assert_eq!(hypotheses.len(), 1);
            assert!(
                hypotheses[0]
                    .bindings
                    .contains_key(&SemanticRole::SourceValue)
            );
            assert!(
                hypotheses[0]
                    .bindings
                    .contains_key(&SemanticRole::TargetValue)
            );
        }

        let operator = synthesize_response_operator(&frames).expect("scalar operator synthesis");
        assert!(matches!(
            operator.candidate.program.operation,
            ResponseOperation::FunctionCallFromRoles { ref arguments, .. }
                if arguments.contains(&ResponseArgument::Role {
                    name: "result".to_owned(),
                    role: SemanticRole::SourceValue,
                    value_type: None,
                })
        ));
        let payload = json!({
            "input": [
                {
                    "type": "function_call",
                    "name": "source_tool",
                    "call_id": "fresh-call",
                    "arguments": "{}",
                },
                {
                    "type": "function_call_output",
                    "call_id": "fresh-call",
                    "output": "{\"nested\":{\"value\":\"fresh value\"}}",
                }
            ]
        });
        let execution = execute_response(&operator.candidate.program, "", &payload);
        assert_eq!(execution.status, ResponseExecutionStatus::Executed);
        let response = execution.response.expect("scalar function call");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&response).expect("response json"),
            json!({
                "name": "route_result",
                "arguments": {"result": "fresh value", "limit": 4},
            })
        );
        assert!(verify_response(&operator.candidate.program, "", &payload, &response).is_ok());
        assert!(verify_response_independently(&operator.verifier, &payload, &response).is_ok());
        assert!(
            verify_response_independently(
                &operator.verifier,
                &payload,
                &response.replace("fresh value", "wrong value"),
            )
            .is_err()
        );

        let ambiguous = json!({
            "input": [{
                "type": "function_call_output",
                "output": "{\"left\":\"one\",\"right\":\"two\"}",
            }]
        });
        assert_eq!(
            execute_response(&operator.candidate.program, "", &ambiguous).status,
            ResponseExecutionStatus::Abstain
        );

        let mut package = compile_source_neutral_quarantine_packages(&frames, true).remove(0);
        package.state = ResponsePackageState::Active;
        package.proof = ResponsePackageProof {
            support_rows: 32,
            future_rows: 32,
            distinct_sessions: 3,
            distinct_surfaces: 2,
            wrong_accepts: 0,
            runtime_parity_failures: 0,
            exact_cache_overlap: 0,
            wave_causal_pass: true,
            verifier_schema: SOURCE_VALUE_EXTERNAL_VERIFIER_SCHEMA.to_owned(),
        };
        assert!(package.eligible_for_admission_candidate());
        let executor = ResponseExecutor::from_registry(ResponseRegistry {
            schema: "nando.response-registry.v5".to_owned(),
            revision: 2,
            packages: vec![package],
        })
        .expect("scalar runtime registry");
        let routed = executor.execute_shadow("", &payload);
        assert_eq!(
            routed.status,
            ResponseExecutionStatus::Executed,
            "{routed:?}"
        );
        assert_eq!(
            executor.execute_shadow("", &ambiguous).status,
            ResponseExecutionStatus::Abstain
        );
    }

    #[test]
    fn p2_synthesizes_and_executes_typed_custom_tool_continuation() {
        let frames = [
            custom_continuation_frame('1', 'a', 3, 8, "exec_command"),
            custom_continuation_frame('2', 'b', 101, 4, "unseen_process_surface"),
        ];
        for frame in &frames {
            let hypotheses = ground_roles(frame);
            assert_eq!(hypotheses.len(), 1);
            assert!(
                hypotheses[0]
                    .bindings
                    .contains_key(&SemanticRole::ContinuationHandle)
            );
        }
        let operator = synthesize_response_operator(&frames).expect("custom tool synthesis");
        assert!(matches!(
            operator.candidate.program.operation,
            ResponseOperation::CustomToolCallFromRoles { .. }
        ));
        let payload = json!({
            "tools": [{"type": "custom", "name": "exec"}],
            "input": [
                {
                    "type": "custom_tool_call",
                    "name": "exec",
                    "call_id": "call-1",
                    "input": "source",
                },
                {
                    "type": "custom_tool_call_output",
                    "call_id": "call-1",
                    "output": [
                        {"type":"text","text":"still running"},
                        {"type":"text","text":"SESSION_ID=99123"}
                    ],
                }
            ]
        });
        let execution = execute_response(&operator.candidate.program, "", &payload);
        assert_eq!(
            execution.status,
            ResponseExecutionStatus::Executed,
            "{execution:?}"
        );
        let response = execution.response.expect("custom tool response");
        let value = serde_json::from_str::<serde_json::Value>(&response).expect("response json");
        assert_eq!(value["kind"], "custom_tool_call");
        assert_eq!(value["name"], "exec");
        assert!(value["input"].as_str().is_some_and(|source| {
            source.contains("tools.write_stdin")
                && source.contains("\"session_id\":99123")
                && source.contains("SESSION_ID=")
        }));
        assert!(verify_response_independently(&operator.verifier, &payload, &response).is_ok());
        let mutated = response.replace("write_stdin", "dangerous_tool");
        assert!(verify_response_independently(&operator.verifier, &payload, &mutated).is_err());

        let ambiguous = json!({
            "input": [{
                "type":"custom_tool_call_output",
                "output":[
                    {"type":"text","text":"SESSION_ID=1"},
                    {"type":"text","text":"SESSION_ID=2"}
                ]
            }]
        });
        assert_eq!(
            execute_response(&operator.candidate.program, "", &ambiguous).status,
            ResponseExecutionStatus::Abstain
        );

        let mut package = compile_source_neutral_quarantine_packages(&frames, true).remove(0);
        let mut changed_target_shape = frames[0].clone();
        for atom in &mut changed_target_shape.atoms {
            if let RelationAtom::ResponseShape { value } = atom {
                *value = "function_call".to_owned();
            }
        }
        assert_eq!(
            relation_frame_routing_atom_ids(&frames[0]),
            relation_frame_routing_atom_ids(&changed_target_shape)
        );
        let mut changed_target_family = frames.clone();
        for frame in &mut changed_target_family {
            for atom in &mut frame.atoms {
                if let RelationAtom::ResponseShape { value } = atom {
                    *value = "function_call".to_owned();
                }
            }
        }
        let target_changed_package =
            compile_source_neutral_quarantine_packages(&changed_target_family, true).remove(0);
        assert_eq!(target_changed_package.package_id, package.package_id);
        let mut changed_observation_shape = frames[0].clone();
        for atom in &mut changed_observation_shape.atoms {
            if let RelationAtom::ObservationCallShape { value } = atom {
                *value = "function_call".to_owned();
            }
        }
        assert!(!relation_frame_routes_to_package(
            &package,
            &changed_observation_shape
        ));
        let mut changed_observation_family = frames.clone();
        for frame in &mut changed_observation_family {
            for atom in &mut frame.atoms {
                if let RelationAtom::ObservationCallShape { value } = atom {
                    *value = "function_call".to_owned();
                }
            }
        }
        let observation_changed_package =
            compile_source_neutral_quarantine_packages(&changed_observation_family, true).remove(0);
        assert_ne!(observation_changed_package.package_id, package.package_id);
        let mut missing_observation_shape = frames.clone();
        for frame in &mut missing_observation_shape {
            frame
                .atoms
                .retain(|atom| !matches!(atom, RelationAtom::ObservationCallShape { .. }));
        }
        assert!(
            compile_source_neutral_quarantine_packages(&missing_observation_shape, true).is_empty()
        );
        package.state = ResponsePackageState::Active;
        package.proof = ResponsePackageProof {
            support_rows: 32,
            future_rows: 32,
            distinct_sessions: 3,
            distinct_surfaces: 2,
            wrong_accepts: 0,
            runtime_parity_failures: 0,
            exact_cache_overlap: 0,
            wave_causal_pass: true,
            verifier_schema: CUSTOM_TOOL_EXTERNAL_VERIFIER_SCHEMA.to_owned(),
        };
        assert!(package.eligible_for_admission_candidate());
        let executor = ResponseExecutor::from_registry(ResponseRegistry {
            schema: "nando.response-registry.v5".to_owned(),
            revision: 3,
            packages: vec![package],
        })
        .expect("custom tool runtime registry");
        let routed = executor.execute_shadow("", &payload);
        assert_eq!(
            routed.status,
            ResponseExecutionStatus::Executed,
            "{routed:?}"
        );
        assert_eq!(
            routed.verifier_schema.as_deref(),
            Some(CUSTOM_TOOL_EXTERNAL_VERIFIER_SCHEMA)
        );
        assert_eq!(
            executor.execute_shadow("", &ambiguous).status,
            ResponseExecutionStatus::Abstain
        );
    }

    #[test]
    fn p2_synthesizes_live_json_result_custom_tool_projection() {
        let mut frames = [
            custom_continuation_frame('3', 'c', 3, 8, "exec_command"),
            custom_continuation_frame('4', 'd', 101, 4, "unseen_process_surface"),
        ];
        for atom in &mut frames[1].atoms {
            if let RelationAtom::ActionIntegerArgument { name, value } = atom
                && name == "max_output_tokens"
            {
                *value = 3_000;
            }
        }
        for frame in &mut frames {
            let observation_slot = frame
                .atoms
                .iter()
                .find_map(|atom| match atom {
                    RelationAtom::ObservationSelector { slot_id, .. } => Some(*slot_id),
                    _ => None,
                })
                .expect("observation selector slot");
            frame.atoms.retain(|atom| {
                !matches!(
                    atom,
                    RelationAtom::ObservationSelector { .. }
                        | RelationAtom::ActionResultProjection { .. }
                )
            });
            frame.atoms.push(RelationAtom::ObservationSelector {
                slot_id: observation_slot,
                selector: ResponseValueSelector::JsonField {
                    field: "session_id".to_owned(),
                    value_type: AtomValueType::Integer,
                },
            });
            frame.atoms.push(RelationAtom::ActionJsonResultProjection);
        }
        let operator = synthesize_response_operator(&frames).expect("json result synthesis");
        assert!(matches!(
            operator.candidate.program.operation,
            ResponseOperation::CustomToolCallFromRoles {
                projection: CustomToolResultProjection::JsonStringifyResult,
                ..
            }
        ));
        let payload = json!({
            "input": [{
                "type": "custom_tool_call_output",
                "output": [{
                    "type": "input_text",
                    "text": "{\"output\":\"running\",\"session_id\":99123,\"wall_time_seconds\":10.0}"
                }]
            }]
        });
        let execution = execute_response(&operator.candidate.program, "", &payload);
        assert_eq!(execution.status, ResponseExecutionStatus::Executed);
        let response = execution.response.expect("custom tool response");
        let value = serde_json::from_str::<serde_json::Value>(&response).expect("response json");
        assert!(value["input"].as_str().is_some_and(|source| {
            source.contains("tools.write_stdin")
                && source.contains("\"session_id\":99123")
                && source.contains("\"max_output_tokens\":3000")
                && source.contains("text(JSON.stringify(r))")
        }));
        assert!(verify_response_independently(&operator.verifier, &payload, &response).is_ok());
        let mutated = response.replace("JSON.stringify(r)", "r.output");
        assert!(verify_response_independently(&operator.verifier, &payload, &mutated).is_err());
    }

    #[test]
    fn p3_grounded_packages_enter_quarantine_without_future_authority() {
        let mut renamed_surface = continuation_trace(101, 4, true);
        renamed_surface.tool_kind = "unseen_runtime_surface".to_owned();
        let frames = [
            extract_relation_frame(&continuation_trace(7, 9, false)),
            extract_relation_frame(&renamed_surface),
        ];
        let packages = compile_source_neutral_quarantine_packages(&frames, true);
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].state, ResponsePackageState::Quarantine);
        assert_eq!(packages[0].proof.support_rows, 2);
        assert_eq!(packages[0].proof.future_rows, 0);
        assert_eq!(packages[0].proof.distinct_surfaces, 2);
        assert_eq!(packages[0].phase_centers.len(), 6);
        assert!(packages[0].anti_centers.is_empty());
        assert!(!packages[0].eligible_for_admission_candidate());

        let mut extended = frames.to_vec();
        let mut later = continuation_trace(77, 88, false);
        later.tool_kind = "third_surface".to_owned();
        later.session_id_sha256 = "9".repeat(64);
        later.observed_at_unix_nanos = 20;
        extended.push(extract_relation_frame(&later));
        let extended_packages = compile_source_neutral_quarantine_packages(&extended, true);
        assert_eq!(extended_packages[0].package_id, packages[0].package_id);

        let undersized = freeze_source_neutral_support(&frames, 30, true);
        assert!(undersized.manifests.is_empty());
        let enough = (0..32_u64)
            .map(|index| {
                let mut frame = frames[(index as usize) % frames.len()].clone();
                frame.frame_id_sha256 = format!("{:064x}", index + 100);
                frame.event_id_sha256 = format!("{:064x}", index + 200);
                frame.client_intent_id_sha256 = format!("{:064x}", index + 300);
                frame.observed_at_unix_nanos = index + 10;
                frame
            })
            .collect::<Vec<_>>();
        let frozen = freeze_source_neutral_support(&enough, 50, true);
        assert_eq!(frozen.manifests.len(), 1);
        assert_eq!(frozen.manifests[0].support_frame_ids.len(), 32);
        assert_eq!(frozen.manifests[0].support_boundary_unix_nanos, 41);
        assert_eq!(frozen.manifests[0].created_at_unix_nanos, 50);
        assert_eq!(
            frozen.manifests[0].learned_center_atom_ids,
            packages[0].phase_centers
        );
        let unchanged = frozen.clone();
        assert_eq!(unchanged, frozen);
    }

    #[test]
    fn p3_freeze_reserves_recent_sessions_without_starving_support() {
        let mut frames = Vec::new();
        for session in 0..10_u64 {
            for row in 0..10_u64 {
                let index = session * 10 + row;
                let mut trace = continuation_trace(7, 9, false);
                trace.event_id_sha256 = format!("{index:064x}");
                trace.client_intent_id_sha256 = format!("{:064x}", index + 100);
                trace.session_id_sha256 = format!("{:064x}", session + 1);
                trace.observed_at_unix_nanos = index + 1;
                let mut frame = extract_relation_frame(&trace);
                frame.frame_id_sha256 = format!("{:064x}", index + 1_000);
                frame.verifier_label = Some(true);
                frame.atoms.push(RelationAtom::Cardinality {
                    role: "turn_message_count_band".to_owned(),
                    count: if index < 35 { 1 } else { 16 },
                });
                frames.push(frame);
            }
        }

        let mut support_negative = frames[0].clone();
        support_negative.frame_id_sha256 = "a".repeat(64);
        support_negative.event_id_sha256 = "b".repeat(64);
        support_negative.client_intent_id_sha256 = "c".repeat(64);
        support_negative.verifier_label = Some(false);
        support_negative.atoms.retain(|atom| {
            !matches!(
                atom,
                RelationAtom::TypedSlot {
                    source: AtomSource::Action,
                    ..
                } | RelationAtom::SlotEquality { .. }
                    | RelationAtom::ActionFunction { .. }
                    | RelationAtom::ActionRoleArgument { .. }
                    | RelationAtom::ActionIntegerArgument { .. }
                    | RelationAtom::Cardinality { .. }
            )
        });
        support_negative.atoms.push(RelationAtom::Cardinality {
            role: "turn_message_count_band".to_owned(),
            count: 4,
        });
        frames.push(support_negative);

        let mut holdout_negative = frames[90].clone();
        holdout_negative.frame_id_sha256 = "d".repeat(64);
        holdout_negative.event_id_sha256 = "e".repeat(64);
        holdout_negative.client_intent_id_sha256 = "f".repeat(64);
        holdout_negative.verifier_label = Some(false);
        holdout_negative.atoms.retain(|atom| {
            !matches!(
                atom,
                RelationAtom::TypedSlot {
                    source: AtomSource::Action,
                    ..
                } | RelationAtom::SlotEquality { .. }
                    | RelationAtom::ActionFunction { .. }
                    | RelationAtom::ActionRoleArgument { .. }
                    | RelationAtom::ActionIntegerArgument { .. }
                    | RelationAtom::Cardinality { .. }
            )
        });
        holdout_negative.atoms.push(RelationAtom::Cardinality {
            role: "turn_message_count_band".to_owned(),
            count: 16,
        });
        frames.push(holdout_negative);

        let frozen = freeze_source_neutral_support(&frames, 1_000, true);
        let manifest = &frozen.manifests[0];
        assert_eq!(manifest.support_session_ids.len(), 7);
        assert_eq!(manifest.reserved_future_session_ids.len(), 3);
        assert_eq!(manifest.split_parent_support_rows, 70);
        assert!(manifest.support_frame_ids.len() >= 32);
        assert_eq!(manifest.split_negative_frame_ids.len(), 1);
        assert_eq!(manifest.holdout_negative_frame_ids.len(), 1);
        assert!(
            manifest
                .reserved_future_session_ids
                .contains(&format!("{:064x}", 10))
        );
        assert!(
            !manifest
                .support_session_ids
                .contains(&format!("{:064x}", 10))
        );

        let support_ids = manifest
            .support_frame_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        let support = frames
            .iter()
            .filter(|frame| support_ids.contains(&frame.frame_id_sha256))
            .cloned()
            .collect::<Vec<_>>();
        let mut package = compile_source_neutral_quarantine_packages(&support, true).remove(0);
        package.phase_centers = manifest.learned_center_atom_ids.clone();
        package.routing_predicates = manifest.selected_routing_predicates.clone();
        assert!(relation_frame_routes_to_package(
            &package,
            frames.last().expect("counterexample")
        ));

        let forced_session = manifest.reserved_future_session_ids[0].clone();
        let policy = ResponseSupportFreezePolicy {
            forced_family_id_by_lineage: std::collections::BTreeMap::new(),
            forced_support_session_ids_by_lineage: std::collections::BTreeMap::from([(
                manifest.lineage_id.clone(),
                std::collections::BTreeSet::from([forced_session.clone()]),
            )]),
            generation_by_lineage: std::collections::BTreeMap::from([(
                manifest.lineage_id.clone(),
                2,
            )]),
            supersedes_package_id_by_lineage: std::collections::BTreeMap::from([(
                manifest.lineage_id.clone(),
                manifest.package_id.clone(),
            )]),
            only_lineages: std::collections::BTreeSet::from([manifest.lineage_id.clone()]),
        };
        let rolled = freeze_source_neutral_support_with_policy(&frames, 2_000, true, &policy);
        assert_eq!(rolled.manifests.len(), 1);
        assert_eq!(rolled.manifests[0].generation, 2);
        assert_eq!(
            rolled.manifests[0].supersedes_package_id.as_deref(),
            Some(manifest.package_id.as_str())
        );
        assert_ne!(rolled.manifests[0].package_id, manifest.package_id);
        assert!(
            rolled.manifests[0]
                .support_session_ids
                .contains(&forced_session)
        );
    }

    #[test]
    fn p3_negative_driven_split_freezes_only_the_clean_context_subcenter() {
        let mut frames = Vec::new();
        for index in 0..45_u64 {
            let mut trace = continuation_trace(7, 9, false);
            trace.event_id_sha256 = format!("{index:064x}");
            trace.client_intent_id_sha256 = format!("{:064x}", index + 100);
            trace.session_id_sha256 = format!("{:064x}", index / 15 + 1);
            trace.observed_at_unix_nanos = index + 1;
            let mut frame = extract_relation_frame(&trace);
            frame.frame_id_sha256 = format!("{:064x}", index + 1_000);
            frame.verifier_label = Some(true);
            frame.atoms.push(RelationAtom::Cardinality {
                role: "active_pending_handle_count_band".to_owned(),
                count: if index % 15 < 12 { 1 } else { 2 },
            });
            frames.push(frame);
        }
        for index in 0..4_u64 {
            let mut negative = frames[index as usize].clone();
            negative.frame_id_sha256 = format!("{:064x}", index + 2_000);
            negative.event_id_sha256 = format!("{:064x}", index + 3_000);
            negative.client_intent_id_sha256 = format!("{:064x}", index + 4_000);
            negative.verifier_label = Some(false);
            negative.atoms.retain(|atom| {
                !matches!(
                    atom,
                    RelationAtom::TypedSlot {
                        source: AtomSource::Action,
                        ..
                    } | RelationAtom::SlotEquality { .. }
                        | RelationAtom::ActionFunction { .. }
                        | RelationAtom::ActionRoleArgument { .. }
                        | RelationAtom::ActionIntegerArgument { .. }
                        | RelationAtom::Cardinality { .. }
                )
            });
            negative.atoms.push(RelationAtom::Cardinality {
                role: "active_pending_handle_count_band".to_owned(),
                count: 2,
            });
            frames.push(negative);
        }

        let frozen = freeze_source_neutral_support(&frames, 100, true);
        assert_eq!(frozen.manifests.len(), 1);
        let manifest = &frozen.manifests[0];
        assert!(
            manifest
                .package_id
                .starts_with(GROUNDED_RESPONSE_PACKAGE_PREFIX)
        );
        assert_eq!(manifest.split_parent_support_rows, 45);
        assert_eq!(manifest.support_frame_ids.len(), 36);
        assert!(manifest.selected_routing_atom_ids.is_empty());
        assert_eq!(manifest.selected_routing_predicates.len(), 1);
        assert_eq!(
            manifest.selected_routing_predicates[0].comparison,
            ResponseRoutingComparison::AtMost
        );
        assert_eq!(manifest.split_negative_frame_ids.len(), 4);
    }

    #[test]
    fn p3_capture_and_runtime_share_the_pre_action_context_contract() {
        let payload = json!({"input":[
            {"type":"custom_tool_call","name":"old","call_id":"old-call","input":"ignored"},
            {"type":"custom_tool_call_output","call_id":"old-call","output":"old output"},
            {"type":"message","role":"user","content":"new turn"},
            {"type":"custom_tool_call","name":"exec","call_id":"exec-1","input":"await tools.exec_command({cmd:'cargo test'})"},
            {"type":"custom_tool_call_output","call_id":"exec-1","output":"Script running with cell ID 859\n"}
        ]});
        let actual = package::response_pre_action_context_atom_ids(&payload)
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let expected = [
            ("turn_call_count_band", 1),
            ("turn_output_count_band", 1),
            ("turn_pending_count_band", 1),
            ("turn_message_count_band", 0),
            ("turn_call_shape_count_band", 1),
        ]
        .into_iter()
        .map(|(role, count)| package::stable_atom_id(&format!("cardinality:{role}:{count}")))
        .chain(std::iter::once(package::stable_atom_id(
            "tool_kind:custom_tool_call:exec",
        )))
        .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn p3_miner_learns_an_ordinal_guard_used_by_the_hot_runtime() {
        let mut frames = Vec::new();
        for index in 0..71_u64 {
            let mut trace = continuation_trace(7, 9, false);
            trace.event_id_sha256 = format!("{index:064x}");
            trace.client_intent_id_sha256 = format!("{:064x}", index + 100);
            trace.session_id_sha256 = "1".repeat(64);
            trace.observed_at_unix_nanos = index + 1;
            let mut frame = extract_relation_frame(&trace);
            frame.frame_id_sha256 = format!("{:064x}", index + 1_000);
            frame.verifier_label = Some(true);
            frame.atoms.push(RelationAtom::Cardinality {
                role: "turn_call_shape_count_band".to_owned(),
                count: match index {
                    0..14 => 1,
                    14..28 => 2,
                    28..42 => 4,
                    _ => 8,
                },
            });
            frames.push(frame);
        }
        for index in 0..4_u64 {
            let mut negative = frames[index as usize].clone();
            negative.frame_id_sha256 = format!("{:064x}", index + 2_000);
            negative.event_id_sha256 = format!("{:064x}", index + 3_000);
            negative.client_intent_id_sha256 = format!("{:064x}", index + 4_000);
            negative.verifier_label = Some(false);
            negative.atoms.retain(|atom| {
                !matches!(
                    atom,
                    RelationAtom::TypedSlot {
                        source: AtomSource::Action,
                        ..
                    } | RelationAtom::SlotEquality { .. }
                        | RelationAtom::ActionFunction { .. }
                        | RelationAtom::ActionRoleArgument { .. }
                        | RelationAtom::ActionIntegerArgument { .. }
                        | RelationAtom::Cardinality { .. }
                )
            });
            negative.atoms.push(RelationAtom::Cardinality {
                role: "turn_call_shape_count_band".to_owned(),
                count: if index == 3 { 32 } else { 8 },
            });
            frames.push(negative);
        }

        let frozen = freeze_source_neutral_support(&frames, 100, true);
        let manifest = &frozen.manifests[0];
        assert_eq!(manifest.split_parent_support_rows, 71);
        assert_eq!(manifest.support_frame_ids.len(), 42);
        assert!(manifest.selected_routing_atom_ids.is_empty());
        assert_eq!(
            manifest.selected_routing_predicates,
            vec![ResponseRoutingPredicate {
                role: "turn_call_shape_count_band".to_owned(),
                comparison: ResponseRoutingComparison::AtMost,
                threshold: 4,
                allowed_counts: Vec::new(),
            }]
        );

        let support_ids = manifest
            .support_frame_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        let support = frames
            .iter()
            .filter(|frame| support_ids.contains(&frame.frame_id_sha256))
            .cloned()
            .collect::<Vec<_>>();
        let mut package = compile_source_neutral_quarantine_packages(&support, true).remove(0);
        package.phase_centers = manifest.learned_center_atom_ids.clone();
        package.routing_predicates = manifest.selected_routing_predicates.clone();
        assert!(relation_frame_routes_to_package(&package, &support[0]));
        assert!(!relation_frame_routes_to_package(
            &package,
            frames.last().expect("negative")
        ));

        package.state = ResponsePackageState::Active;
        package.proof = ResponsePackageProof {
            support_rows: 42,
            future_rows: 32,
            distinct_sessions: 3,
            distinct_surfaces: 2,
            wrong_accepts: 0,
            runtime_parity_failures: 0,
            exact_cache_overlap: 0,
            wave_causal_pass: true,
            verifier_schema: "continue_handle_external_evidence.v1".to_owned(),
        };
        let executor = ResponseExecutor::from_registry(ResponseRegistry {
            schema: "nando.response-registry.v5".to_owned(),
            revision: 1,
            packages: vec![package],
        })
        .expect("runtime registry");
        let payload = |call_shapes: usize| {
            let mut input = vec![json!({"type":"message","role":"user","content":"turn"})];
            input.extend((1..call_shapes).flat_map(|index| {
                [
                    json!({"type":"function_call","name":format!("shape-{index}"),"call_id":format!("call-{index}"),"arguments":"{}"}),
                    json!({"type":"function_call_output","call_id":format!("call-{index}"),"output":"completed"}),
                ]
            }));
            input.extend([
                json!({"type":"function_call","name":"exec","call_id":"exec-1","arguments":"{}"}),
                json!({"type":"function_call_output","call_id":"exec-1","output":"Script running with cell ID 859\n"}),
            ]);
            json!({"input": input})
        };
        assert_eq!(
            executor.execute_shadow("", &payload(4)).status,
            ResponseExecutionStatus::Executed
        );
        assert_eq!(
            executor.execute_shadow("", &payload(8)).status,
            ResponseExecutionStatus::Abstain
        );
    }

    #[test]
    fn p3_miner_unites_disjoint_clean_bands_without_accepting_the_dirty_band() {
        let mut frames = Vec::new();
        for index in 0..100_u64 {
            let mut trace = continuation_trace(7, 9, false);
            trace.event_id_sha256 = format!("{index:064x}");
            trace.client_intent_id_sha256 = format!("{:064x}", index + 100);
            trace.session_id_sha256 = "1".repeat(64);
            trace.observed_at_unix_nanos = index + 1;
            let mut frame = extract_relation_frame(&trace);
            frame.frame_id_sha256 = format!("{:064x}", index + 1_000);
            frame.verifier_label = Some(true);
            frame.atoms.push(RelationAtom::Cardinality {
                role: "active_pending_handle_count_band".to_owned(),
                count: match index {
                    0..40 => 1,
                    40..60 => 4,
                    _ => 16,
                },
            });
            frames.push(frame);
        }
        for index in 0..4_u64 {
            let mut negative = frames[(40 + index) as usize].clone();
            negative.frame_id_sha256 = format!("{:064x}", index + 2_000);
            negative.event_id_sha256 = format!("{:064x}", index + 3_000);
            negative.client_intent_id_sha256 = format!("{:064x}", index + 4_000);
            negative.verifier_label = Some(false);
            negative.atoms.retain(|atom| {
                !matches!(
                    atom,
                    RelationAtom::TypedSlot {
                        source: AtomSource::Action,
                        ..
                    } | RelationAtom::SlotEquality { .. }
                        | RelationAtom::ActionFunction { .. }
                        | RelationAtom::ActionRoleArgument { .. }
                        | RelationAtom::ActionIntegerArgument { .. }
                )
            });
            frames.push(negative);
        }

        let frozen = freeze_source_neutral_support(&frames, 1_000, true);
        let manifest = &frozen.manifests[0];
        assert_eq!(manifest.split_parent_support_rows, 100);
        assert_eq!(manifest.support_frame_ids.len(), 80);
        assert_eq!(
            manifest.selected_routing_predicates,
            vec![ResponseRoutingPredicate {
                role: "active_pending_handle_count_band".to_owned(),
                comparison: ResponseRoutingComparison::OneOf,
                threshold: 0,
                allowed_counts: vec![1, 16],
            }]
        );

        let support_ids = manifest
            .support_frame_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        let support = frames
            .iter()
            .filter(|frame| support_ids.contains(&frame.frame_id_sha256))
            .cloned()
            .collect::<Vec<_>>();
        let mut package = compile_source_neutral_quarantine_packages(&support, true).remove(0);
        package.phase_centers = manifest.learned_center_atom_ids.clone();
        package.routing_predicates = manifest.selected_routing_predicates.clone();
        assert!(relation_frame_routes_to_package(&package, &frames[0]));
        assert!(!relation_frame_routes_to_package(&package, &frames[40]));
        assert!(relation_frame_routes_to_package(&package, &frames[99]));
    }

    #[test]
    fn p2_verifier_mutations_and_ambiguous_roles_are_killed() {
        let frame = extract_relation_frame(&continuation_trace(7, 9, false));
        let operator = synthesize_response_operator(std::slice::from_ref(&frame))
            .expect("synthesize operator");
        let mut missing_unique = frame.clone();
        missing_unique
            .atoms
            .retain(|atom| !matches!(atom, RelationAtom::UniqueSlot { .. }));
        assert!(!verify_operator_structure(&missing_unique, &operator));

        let mut ambiguous = continuation_trace(7, 9, false);
        ambiguous.slots.extend([
            TraceSlot {
                slot_id: 11,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Observation,
                value_sha256: "b".repeat(64),
            },
            TraceSlot {
                slot_id: 12,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Action,
                value_sha256: "b".repeat(64),
            },
        ]);
        ambiguous.equal_slots.push((11, 12));
        assert_eq!(
            synthesize_response_operator(&[extract_relation_frame(&ambiguous)]),
            Err(SynthesisError::AmbiguousRoles)
        );
    }

    #[test]
    fn copy_after_prefix_transfers_unseen_fillers() {
        let program = ResponseProgram::copy_after_prefix(["reply exactly:", "respond exactly:"]);
        for (request, expected) in [
            ("Reply exactly: ALPHA", "ALPHA"),
            ("Respond exactly: beta-42", "beta-42"),
            ("Reply exactly: новый ответ", "новый ответ"),
        ] {
            let execution = execute_response(&program, request, &json!({}));
            assert_eq!(execution.status, ResponseExecutionStatus::Executed);
            assert_eq!(execution.response.as_deref(), Some(expected));
            assert!(execution.verification_receipt_id.is_none());
        }
    }

    #[test]
    fn copy_after_prefix_abstains_on_unsupported_or_ambiguous_requests() {
        let program = ResponseProgram::copy_after_prefix(["reply exactly:"]);
        for request in [
            "explain this",
            "Reply exactly:",
            "prefix Reply exactly: hidden",
        ] {
            let execution = execute_response(&program, request, &json!({}));
            assert_eq!(execution.status, ResponseExecutionStatus::Abstain);
        }
    }

    #[test]
    fn test_summary_requires_narrow_intent_and_grounded_tool_output() {
        let program = ResponseProgram::test_result_summary(
            ["run tests", "check tests"],
            ["fix", "implement", "change", "continue"],
        );
        let payload = json!({
            "input":[{
                "type":"function_call_output",
                "output":"running 3 tests\ntest result: ok. 3 passed; 0 failed"
            }]
        });
        let execution = execute_response(&program, "Run tests", &payload);
        assert_eq!(execution.status, ResponseExecutionStatus::Executed);
        assert_eq!(execution.response.as_deref(), Some("Validation passed."));

        let broad = execute_response(&program, "Run tests and fix failures", &payload);
        assert_eq!(broad.status, ResponseExecutionStatus::Abstain);
    }

    #[test]
    fn test_summary_reads_codex_structured_wait_output() {
        let program = ResponseProgram::test_result_summary(
            ["run tests", "cargo test"],
            ["fix", "implement", "change", "continue"],
        );
        let payload = json!({"input":[{
            "type":"function_call_output",
            "output":[
                {"type":"input_text","text":"Chunk ID: x\nWall time: 0.1 seconds\nProcess exited with code 0\nFinal output:"},
                {"type":"input_text","text":"running 12 tests\ntest result: ok. 12 passed; 0 failed"}
            ]
        }]});
        let execution = execute_response(&program, "Run cargo test", &payload);
        assert_eq!(execution.status, ResponseExecutionStatus::Executed);
        assert_eq!(execution.response.as_deref(), Some("Validation passed."));
    }

    #[test]
    fn verifier_rejects_actor_output_mutation() {
        let program = ResponseProgram::copy_after_prefix(["reply exactly:"]);
        assert!(verify_response(&program, "Reply exactly: OK", &json!({}), "WRONG").is_err());
    }

    #[test]
    fn wait_actor_builds_verified_function_call_from_yielded_cell() {
        let program = ResponseProgram::wait_on_yielded_cell();
        let payload = json!({
            "input":[
            {"type":"function_call","name":"exec","call_id":"exec-1","arguments":"{\"cmd\":\"cargo test\"}"},
            {
                "type":"function_call_output",
                "call_id":"exec-1",
                "output":"Script running with cell ID 859\nWall time 10.0 seconds\nOutput:\n"
            }]
        });
        let execution = execute_response(&program, "", &payload);
        assert_eq!(execution.status, ResponseExecutionStatus::Executed);
        let response: serde_json::Value =
            serde_json::from_str(execution.response.as_deref().expect("response")).expect("json");
        assert_eq!(response.pointer("/arguments/cell_id"), Some(&json!("859")));
        assert_eq!(response.get("name"), Some(&json!("wait")));
    }

    #[test]
    fn wait_actor_rejects_stale_yield_evidence() {
        let program = ResponseProgram::wait_on_yielded_cell();
        let payload = json!({"input":[
            {"type":"function_call","name":"exec","call_id":"exec-1","arguments":"{\"cmd\":\"cargo test\"}"},
            {"type":"function_call_output","call_id":"exec-1","output":"Script running with cell ID 859\nWall time 10.0 seconds\nOutput:\n"},
            {"type":"message","role":"user","content":"new task"}
        ]});
        let execution = execute_response(&program, "new task", &payload);
        assert_eq!(execution.status, ResponseExecutionStatus::Abstain);
    }

    #[test]
    fn wait_actor_accepts_codex_custom_tool_surface() {
        let program = ResponseProgram::wait_on_yielded_cell();
        let payload = json!({"input":[
            {"type":"custom_tool_call","name":"exec","call_id":"exec-2","input":"await tools.exec_command({cmd:'cargo test'})"},
            {"type":"custom_tool_call_output","call_id":"exec-2","output":"Script running with cell ID 974\nWall time 10.0 seconds\nOutput:\n"}
        ]});
        let execution = execute_response(&program, "", &payload);
        assert_eq!(execution.status, ResponseExecutionStatus::Executed);
        assert!(
            execution
                .response
                .as_deref()
                .is_some_and(|value| value.contains("974"))
        );
    }

    #[test]
    fn any_wait_actor_transfers_to_non_build_yielded_cells() {
        let program = ResponseProgram::wait_on_any_yielded_cell();
        let payload = json!({"input":[
            {"type":"custom_tool_call","name":"exec","call_id":"exec-3","input":"await tools.exec_command({cmd:'journalctl -f'})"},
            {"type":"custom_tool_call_output","call_id":"exec-3","output":"Script running with cell ID service-42\nWall time 10.0 seconds\nOutput:\n"}
        ]});
        let execution = execute_response(&program, "", &payload);
        assert_eq!(execution.status, ResponseExecutionStatus::Executed);
        assert!(
            execution
                .response
                .as_deref()
                .is_some_and(|value| value.contains("service-42"))
        );
    }

    #[test]
    fn surfaced_wait_actor_abstains_outside_its_semantic_guard() {
        let program = ResponseProgram::wait_on_yielded_surfaces(["service_observation"]);
        let service = json!({"input":[
            {"type":"custom_tool_call","call_id":"exec-4","input":"await tools.exec_command({cmd:'journalctl -f'})"},
            {"type":"custom_tool_call_output","call_id":"exec-4","output":"Script running with cell ID service-43\n"}
        ]});
        assert_eq!(
            execute_response(&program, "", &service).status,
            ResponseExecutionStatus::Executed
        );
        let network = json!({"input":[
            {"type":"custom_tool_call","call_id":"exec-5","input":"await tools.exec_command({cmd:'ping host'})"},
            {"type":"custom_tool_call_output","call_id":"exec-5","output":"Script running with cell ID network-1\n"}
        ]});
        assert_eq!(
            execute_response(&program, "", &network).status,
            ResponseExecutionStatus::Abstain
        );
    }

    #[test]
    fn programs_are_compact() {
        let copy = ResponseProgram::copy_after_prefix(["reply exactly:", "respond exactly:"]);
        let test = ResponseProgram::test_result_summary(
            ["run tests", "check tests"],
            ["fix", "implement", "change"],
        );
        assert!(serde_json::to_vec(&copy).expect("serialize").len() <= 512);
        assert!(serde_json::to_vec(&test).expect("serialize").len() <= 512);
    }

    #[test]
    fn response_registry_version_skew_fails_closed() {
        let error = ResponseExecutor::from_registry(ResponseRegistry {
            schema: "nando.response-registry.v3".to_owned(),
            revision: 1,
            packages: Vec::new(),
        })
        .expect_err("stale serving binaries must reject the new registry contract");
        assert_eq!(error, "unsupported_registry_schema");
    }

    #[test]
    fn v5_external_registries_reject_private_selectors_but_allow_session_id() {
        static TEST_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let registry = |selector: ResponseValueSelector| {
            let program = ResponseProgram::project_status(
                selector.clone(),
                ProjectStatusMapping::ZeroIsSuccess,
                "completed",
            );
            let required_routing_atom_ids = response_program_required_routing_atom_ids(&program);
            ResponseRegistry {
                schema: "nando.response-registry.v5".to_owned(),
                revision: 1,
                packages: vec![ResponsePackage {
                    schema: "nando.response-package.v1".to_owned(),
                    package_id: "external-status".to_owned(),
                    origin: ResponsePackageOrigin::GroundedSynthesis,
                    state: ResponsePackageState::Active,
                    program,
                    verifier: Some(VerifierProgram::ProjectStatus {
                        selector,
                        mapping: ProjectStatusMapping::ZeroIsSuccess,
                        renderer: CollectionOutputRenderer::Direct,
                        completion_state: "completed".to_owned(),
                        require_unique_value: true,
                    }),
                    routing_predicates: Vec::new(),
                    required_routing_atom_ids: required_routing_atom_ids.clone(),
                    phase_centers: required_routing_atom_ids,
                    anti_centers: Vec::new(),
                    wave_margin_micro: 1,
                    learned_wave_route: None,
                    crystallized_operator: None,
                    proof: ResponsePackageProof {
                        support_rows: 32,
                        future_rows: 32,
                        distinct_sessions: 3,
                        distinct_surfaces: 2,
                        wrong_accepts: 0,
                        runtime_parity_failures: 0,
                        exact_cache_overlap: 0,
                        wave_causal_pass: true,
                        verifier_schema: "status_projection_external_evidence.v1".to_owned(),
                    },
                }],
            }
        };
        let root = std::env::temp_dir().join(format!(
            "nando-response-v5-selector-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).expect("test root");

        for (name, selector, expected) in [
            (
                "token",
                ResponseValueSelector::JsonField {
                    field: "api_token".to_owned(),
                    value_type: AtomValueType::Integer,
                },
                "invalid_selector_field",
            ),
            (
                "private-key",
                ResponseValueSelector::ContentLinePrefix {
                    prefix: "PRIVATE_KEY=".to_owned(),
                    value_type: AtomValueType::Integer,
                },
                "invalid_selector_prefix",
            ),
        ] {
            let path = root.join(format!("{name}.json"));
            std::fs::write(
                &path,
                serde_json::to_vec(&registry(selector)).expect("registry json"),
            )
            .expect("write registry");
            assert_eq!(
                ResponseExecutor::load(&path).expect_err("private selector must fail closed"),
                expected
            );
        }

        let benign_path = root.join("session-id.json");
        std::fs::write(
            &benign_path,
            serde_json::to_vec(&registry(ResponseValueSelector::JsonField {
                field: "session_id".to_owned(),
                value_type: AtomValueType::Integer,
            }))
            .expect("registry json"),
        )
        .expect("write registry");
        assert_eq!(
            ResponseExecutor::load(&benign_path)
                .expect("benign external registry")
                .active_package_count(),
            0
        );
        assert_eq!(
            ResponseExecutor::load(&benign_path)
                .expect("benign diagnostic registry")
                .diagnostic_package_count(),
            1
        );

        std::fs::remove_dir_all(root).expect("cleanup test root");
    }

    #[test]
    fn registry_excludes_fixture_and_unproven_packages() {
        let program = ResponseProgram::function_call_from_roles(
            "wait",
            ResponseValueSelector::ContentLinePrefix {
                prefix: "Script running with cell ID ".to_owned(),
                value_type: AtomValueType::Identifier,
            },
            vec![
                ResponseArgument::Role {
                    name: "cell_id".to_owned(),
                    role: SemanticRole::ContinuationHandle,
                    value_type: None,
                },
                ResponseArgument::Integer {
                    name: "yield_time_ms".to_owned(),
                    value: 1_000,
                },
                ResponseArgument::Integer {
                    name: "max_tokens".to_owned(),
                    value: 5_000,
                },
            ],
        );
        let required_routing_atom_ids = response_program_required_routing_atom_ids(&program);
        let proven = ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id: "proven".to_owned(),
            origin: ResponsePackageOrigin::GroundedSynthesis,
            state: ResponsePackageState::Active,
            program,
            verifier: Some(VerifierProgram::FunctionCallFromRoles {
                function_name: "wait".to_owned(),
                selector: ResponseValueSelector::ContentLinePrefix {
                    prefix: "Script running with cell ID ".to_owned(),
                    value_type: AtomValueType::Identifier,
                },
                role_arguments: BTreeMap::from([(
                    "cell_id".to_owned(),
                    SemanticRole::ContinuationHandle,
                )]),
                role_argument_types: BTreeMap::new(),
                integer_arguments: BTreeMap::from([
                    ("yield_time_ms".to_owned(), 1_000),
                    ("max_tokens".to_owned(), 5_000),
                ]),
                string_arguments: BTreeMap::new(),
                boolean_arguments: BTreeMap::new(),
                require_pending_state: true,
                require_unique_handle: true,
            }),
            routing_predicates: Vec::new(),
            required_routing_atom_ids,
            phase_centers: package::response_phase_atom_ids_for_grounded_function_call(),
            anti_centers: vec![2],
            wave_margin_micro: 100_000,
            learned_wave_route: None,
            crystallized_operator: None,
            proof: ResponsePackageProof {
                support_rows: 32,
                future_rows: 32,
                distinct_sessions: 3,
                distinct_surfaces: 2,
                wrong_accepts: 0,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass: true,
                verifier_schema: "continue_handle_external_evidence.v1".to_owned(),
            },
        };
        let mut fixture = proven.clone();
        fixture.package_id = "fixture".to_owned();
        fixture.origin = ResponsePackageOrigin::ImportedFixture;
        let mut unproven = proven.clone();
        unproven.package_id = "unproven".to_owned();
        unproven.proof.future_rows = 0;
        let executor = ResponseExecutor::from_registry(ResponseRegistry {
            schema: "nando.response-registry.v5".to_owned(),
            revision: 7,
            packages: vec![fixture, unproven, proven],
        })
        .expect("registry");
        assert_eq!(executor.revision(), 7);
        assert_eq!(executor.active_package_count(), 0);
        assert_eq!(executor.diagnostic_package_count(), 1);
        let result = executor.execute_shadow(
            "",
            &json!({"input":[{
                "type":"function_call_output",
                "call_id":"exec-1",
                "output":"Script running with cell ID 859\nWall time 10.0 seconds\nOutput:\n"
            }]}),
        );
        assert_eq!(result.status, ResponseExecutionStatus::Executed);
        assert_eq!(result.package_id.as_deref(), Some("proven"));
        assert_eq!(result.exact_actor_checks, 1);
        assert!(result.phase_margin_micro.is_some());
    }

    #[test]
    fn legacy_lifecycle_can_report_shadow_active_without_execution_authority() {
        let relations = (0..32)
            .map(|index| ResponseRelationObservation {
                schema: "nando.response-relation-observation.v1".to_owned(),
                relation_id: format!("relation-{index}"),
                observed_at: format!("{index:03}"),
                relation: "outcome_equals_request_suffix".to_owned(),
                program_hint: lifecycle::ResponseProgramHint {
                    op: "copy_after_prefix".to_owned(),
                    prefix: if index % 2 == 0 {
                        "reply exactly:".to_owned()
                    } else {
                        "respond exactly:".to_owned()
                    },
                },
                source_session_id_sha256: format!("session-{}", index % 3),
                source_turn_id_sha256: format!("turn-{index}"),
                surface_id_sha256: format!("surface-{}", index % 2),
                verifier_ok: true,
                guard_schema: String::new(),
            })
            .collect::<Vec<_>>();
        let quarantine = compile_response_registry(1, &relations, &[], true);
        assert_eq!(
            quarantine.packages[0].state,
            ResponsePackageState::Quarantine
        );
        let package_id = quarantine.packages[0].package_id.clone();
        let shadows = (0..32)
            .map(|index| ResponseShadowObservation {
                schema: "nando.response-shadow-observation.v1".to_owned(),
                package_id: package_id.clone(),
                observed_at: format!("future-{index:03}"),
                source_session_id_sha256: format!("future-session-{}", index % 3),
                surface_id_sha256: format!("surface-{}", index % 2),
                matched_guard: true,
                verifier_ok: true,
            })
            .collect::<Vec<_>>();
        let active = compile_response_registry(2, &relations, &shadows, true);
        assert_eq!(active.packages[0].state, ResponsePackageState::Active);
        assert_eq!(
            active.packages[0].origin,
            ResponsePackageOrigin::LegacyTemplate
        );
        let executor = ResponseExecutor::from_registry(active).expect("active registry");
        assert_eq!(executor.active_package_count(), 0);
    }

    #[test]
    fn legacy_templates_never_enter_l2_execution_router() {
        let packages = (0..16)
            .map(|index| {
                let prefix = if index == 7 {
                    "reply exactly:".to_owned()
                } else {
                    format!("route-{index}:")
                };
                ResponsePackage {
                    schema: "nando.response-package.v1".to_owned(),
                    package_id: format!("package-{index}"),
                    origin: ResponsePackageOrigin::LegacyTemplate,
                    state: ResponsePackageState::Active,
                    program: ResponseProgram::copy_after_prefix([prefix.clone()]),
                    verifier: None,
                    routing_predicates: Vec::new(),
                    required_routing_atom_ids: Vec::new(),
                    phase_centers: package::response_phase_atom_ids_for_prefix(&prefix),
                    anti_centers: vec![],
                    wave_margin_micro: 850_000,
                    learned_wave_route: None,
                    crystallized_operator: None,
                    proof: ResponsePackageProof {
                        support_rows: 32,
                        future_rows: 32,
                        distinct_sessions: 3,
                        distinct_surfaces: 2,
                        wrong_accepts: 0,
                        runtime_parity_failures: 0,
                        exact_cache_overlap: 0,
                        wave_causal_pass: true,
                        verifier_schema: "response_actor_independent_verifier.v1".to_owned(),
                    },
                }
            })
            .collect();
        let executor = ResponseExecutor::from_registry(ResponseRegistry {
            schema: "nando.response-registry.v5".to_owned(),
            revision: 1,
            packages,
        })
        .expect("registry");
        assert_eq!(executor.active_package_count(), 0);
        let result = executor.execute("Reply exactly: HELDOUT", &json!({}));
        assert_eq!(result.status, ResponseExecutionStatus::Abstain);
        assert_eq!(result.package_id, None);
        assert_eq!(result.exact_actor_checks, 0);
    }

    #[test]
    fn lifecycle_wrong_future_accept_blocks_activation() {
        let relations = (0..32)
            .map(|index| ResponseRelationObservation {
                schema: "nando.response-relation-observation.v1".to_owned(),
                relation_id: format!("relation-{index}"),
                observed_at: format!("{index:03}"),
                relation: "outcome_equals_request_suffix".to_owned(),
                program_hint: lifecycle::ResponseProgramHint {
                    op: "copy_after_prefix".to_owned(),
                    prefix: "reply exactly:".to_owned(),
                },
                source_session_id_sha256: format!("session-{}", index % 3),
                source_turn_id_sha256: format!("turn-{index}"),
                surface_id_sha256: format!("surface-{}", index % 2),
                verifier_ok: true,
                guard_schema: String::new(),
            })
            .collect::<Vec<_>>();
        let quarantine = compile_response_registry(1, &relations, &[], true);
        let package_id = quarantine.packages[0].package_id.clone();
        let shadows = (0..32)
            .map(|index| ResponseShadowObservation {
                schema: "nando.response-shadow-observation.v1".to_owned(),
                package_id: package_id.clone(),
                observed_at: format!("future-{index:03}"),
                source_session_id_sha256: format!("future-session-{}", index % 3),
                surface_id_sha256: format!("surface-{}", index % 2),
                matched_guard: true,
                verifier_ok: index != 31,
            })
            .collect::<Vec<_>>();
        let blocked = compile_response_registry(2, &relations, &shadows, true);
        assert_eq!(blocked.packages[0].state, ResponsePackageState::Quarantine);
        assert_eq!(blocked.packages[0].proof.wrong_accepts, 1);
    }

    fn selected_value_frame(
        marker: char,
        value_type: AtomValueType,
        selector: ResponseValueSelector,
        format: ValueProjectionFormat,
        source_hash: &str,
        target_hash: &str,
        extractor_version: &str,
    ) -> RelationFrame {
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: marker.to_string().repeat(64),
            event_id_sha256: marker.to_ascii_uppercase().to_string().repeat(64),
            client_intent_id_sha256: marker.to_string().repeat(64),
            session_id_sha256: marker.to_string().repeat(64),
            observed_at_unix_nanos: 100,
            estimated_input_tokens: 0,
            extractor_version: extractor_version.to_owned(),
            verifier_label: Some(true),
            atoms: vec![
                RelationAtom::ToolKind {
                    value: "observed_tool".to_owned(),
                },
                RelationAtom::ObservationCallShape {
                    value: "function_call".to_owned(),
                },
                RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
                RelationAtom::ResponseShape {
                    value: "assistant_message".to_owned(),
                },
                RelationAtom::TypedSlot {
                    slot_id: 7,
                    value_type,
                    source: AtomSource::Observation,
                    value_sha256: source_hash.to_owned(),
                },
                RelationAtom::ObservationSelector {
                    slot_id: 7,
                    selector,
                },
                RelationAtom::TypedSlot {
                    slot_id: 11,
                    value_type,
                    source: AtomSource::Action,
                    value_sha256: target_hash.to_owned(),
                },
                RelationAtom::SlotEquality {
                    left_slot: 7,
                    right_slot: 11,
                },
                RelationAtom::UniqueSlot { slot_id: 7 },
                RelationAtom::ActionValueProjection {
                    format,
                    renderer: CollectionOutputRenderer::Direct,
                },
            ],
            evidence_ref_sha256: "e".repeat(64),
        }
    }

    #[test]
    fn project_selected_value_plain_text_is_a_verified_assistant_message() {
        let frame = selected_value_frame(
            'p',
            AtomValueType::String,
            ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::String,
            },
            ValueProjectionFormat::PlainText,
            &"1".repeat(64),
            &"1".repeat(64),
            SOURCE_NEUTRAL_EXTRACTOR_VERSION,
        );
        let operator = synthesize_response_operator(&[frame]).expect("projection synthesis");
        assert!(matches!(
            operator.candidate.program.operation,
            ResponseOperation::ProjectSelectedValue { .. }
        ));
        let payload = json!({"input":[{
            "type":"function_call_output", "output":"source neutral value"
        }]});
        let execution = execute_response(&operator.candidate.program, "", &payload);
        assert_eq!(execution.status, ResponseExecutionStatus::Executed);
        assert_eq!(execution.response.as_deref(), Some("source neutral value"));
        assert!(
            verify_response_independently(
                &operator.verifier,
                &payload,
                execution.response.as_deref().expect("response")
            )
            .is_ok()
        );
    }

    #[test]
    fn project_selected_value_template_is_learned_and_verified_independently() {
        let mut frame = selected_value_frame(
            't',
            AtomValueType::Identifier,
            ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::Identifier,
            },
            ValueProjectionFormat::PlainText,
            &"7".repeat(64),
            &"7".repeat(64),
            SOURCE_NEUTRAL_EXTRACTOR_VERSION,
        );
        let renderer = CollectionOutputRenderer::RenderTemplate {
            prefix: "Result: ".to_owned(),
            suffix: ".".to_owned(),
        };
        for atom in &mut frame.atoms {
            if let RelationAtom::ActionValueProjection {
                renderer: observed, ..
            } = atom
            {
                *observed = renderer.clone();
            }
        }
        let operator = synthesize_response_operator(&[frame]).expect("template synthesis");
        let payload = json!({"input":[{
            "type":"function_call_output", "output":"ready"
        }]});
        let execution = execute_response(&operator.candidate.program, "", &payload);
        assert_eq!(
            execution.status,
            ResponseExecutionStatus::Executed,
            "{}",
            execution.reason
        );
        assert_eq!(execution.response.as_deref(), Some("Result: ready."));
        assert!(
            verify_response_independently(
                &operator.verifier,
                &payload,
                execution.response.as_deref().expect("response")
            )
            .is_ok()
        );
        assert!(verify_response_independently(&operator.verifier, &payload, "ready").is_err());
    }

    #[test]
    fn multi_claim_renderer_is_learned_and_verified_without_stored_values() {
        let mut frame = selected_value_frame(
            'm',
            AtomValueType::Integer,
            ResponseValueSelector::JsonField {
                field: "count".to_owned(),
                value_type: AtomValueType::Integer,
            },
            ValueProjectionFormat::PlainText,
            &"8".repeat(64),
            &"8".repeat(64),
            SOURCE_NEUTRAL_EXTRACTOR_VERSION,
        );
        let renderer = CollectionOutputRenderer::RenderSequence {
            segments: vec![
                ResponseRenderSegment::Static {
                    text: "Count: ".to_owned(),
                },
                ResponseRenderSegment::Primary,
                ResponseRenderSegment::Static {
                    text: "; status: ".to_owned(),
                },
                ResponseRenderSegment::Selected {
                    selector: ResponseValueSelector::JsonField {
                        field: "status".to_owned(),
                        value_type: AtomValueType::String,
                    },
                    format: ValueProjectionFormat::PlainText,
                },
                ResponseRenderSegment::Static {
                    text: ".".to_owned(),
                },
            ],
        };
        for atom in &mut frame.atoms {
            if let RelationAtom::ActionValueProjection {
                renderer: observed, ..
            } = atom
            {
                *observed = renderer.clone();
            }
        }
        let operator = synthesize_response_operator(&[frame]).expect("sequence synthesis");
        let payload = json!({"input":[{
            "type":"function_call_output", "output":"{\"count\":3,\"status\":\"passed\"}"
        }]});
        let execution = execute_response(&operator.candidate.program, "", &payload);
        assert_eq!(
            execution.status,
            ResponseExecutionStatus::Executed,
            "{}",
            execution.reason
        );
        assert_eq!(
            execution.response.as_deref(),
            Some("Count: 3; status: passed.")
        );
        assert!(
            verify_response_independently(
                &operator.verifier,
                &payload,
                execution.response.as_deref().expect("response")
            )
            .is_ok()
        );
        assert!(
            verify_response_independently(
                &operator.verifier,
                &payload,
                "Count: 3; status: failed."
            )
            .is_err()
        );
    }

    #[test]
    fn turn_output_line_sequence_replays_positionally_and_abstains_when_missing() {
        let program = ResponseProgram::project_selected_value(
            ResponseValueSelector::TurnOutputLine {
                output_ordinal: 1,
                line_index: 0,
                value_type: AtomValueType::String,
            },
            ValueProjectionFormat::PlainText,
            "completed",
        )
        .with_value_renderer(CollectionOutputRenderer::RenderSequence {
            segments: vec![
                ResponseRenderSegment::Static {
                    text: "Result: ".to_owned(),
                },
                ResponseRenderSegment::Primary,
                ResponseRenderSegment::Static {
                    text: "; status: ".to_owned(),
                },
                ResponseRenderSegment::Selected {
                    selector: ResponseValueSelector::TurnOutputLine {
                        output_ordinal: 1,
                        line_index: 1,
                        value_type: AtomValueType::String,
                    },
                    format: ValueProjectionFormat::PlainText,
                },
                ResponseRenderSegment::Static {
                    text: ".".to_owned(),
                },
            ],
        });
        let payload = json!({"input":[
            {"type":"message", "role":"user", "content":"check"},
            {"type":"function_call_output", "output":"apt is blocked\nchrome hold"}
        ]});
        let execution = execute_response(&program, "", &payload);
        assert_eq!(
            execution.status,
            ResponseExecutionStatus::Executed,
            "{}",
            execution.reason
        );
        assert_eq!(
            execution.response.as_deref(),
            Some("Result: apt is blocked; status: chrome hold.")
        );
        assert!(
            verify_response(
                &program,
                "",
                &payload,
                execution.response.as_deref().expect("response")
            )
            .is_ok()
        );
        let missing = json!({"input":[
            {"type":"message", "role":"user", "content":"check"},
            {"type":"function_call_output", "output":"apt is blocked"}
        ]});
        assert_eq!(
            execute_response(&program, "", &missing).status,
            ResponseExecutionStatus::Abstain
        );
    }

    #[test]
    fn project_selected_value_canonical_json_scalar_is_exact() {
        let frame = selected_value_frame(
            'j',
            AtomValueType::Integer,
            ResponseValueSelector::JsonField {
                field: "selected".to_owned(),
                value_type: AtomValueType::Integer,
            },
            ValueProjectionFormat::CanonicalJson,
            &"2".repeat(64),
            &"2".repeat(64),
            SOURCE_NEUTRAL_EXTRACTOR_VERSION,
        );
        let operator = synthesize_response_operator(&[frame]).expect("json projection synthesis");
        let payload = json!({"input":[{
            "type":"function_call_output", "output":"{\"selected\":42}"
        }]});
        let execution = execute_response(&operator.candidate.program, "", &payload);
        assert_eq!(execution.response.as_deref(), Some("42"));
    }

    #[test]
    fn project_selected_value_ambiguity_type_hash_and_staleness_abstain() {
        let selector = ResponseValueSelector::ContentLinePrefix {
            prefix: "value=".to_owned(),
            value_type: AtomValueType::Integer,
        };
        let good = selected_value_frame(
            'a',
            AtomValueType::Integer,
            selector.clone(),
            ValueProjectionFormat::PlainText,
            &"3".repeat(64),
            &"3".repeat(64),
            SOURCE_NEUTRAL_EXTRACTOR_VERSION,
        );
        let operator = synthesize_response_operator(&[good]).expect("projection synthesis");
        for payload in [
            json!({"input":[{"type":"function_call_output","output":"value=1\nvalue=2"}]}),
            json!({"input":[{"type":"function_call_output","output":"value=not-an-integer"}]}),
            json!({"input":[{"type":"function_call_output","output":"value=1"},{"type":"message","role":"user","content":"new turn"}]}),
        ] {
            assert_eq!(
                execute_response(&operator.candidate.program, "", &payload).status,
                ResponseExecutionStatus::Abstain
            );
        }
        let hash_mismatch = selected_value_frame(
            'h',
            AtomValueType::Integer,
            selector,
            ValueProjectionFormat::PlainText,
            &"4".repeat(64),
            &"5".repeat(64),
            SOURCE_NEUTRAL_EXTRACTOR_VERSION,
        );
        assert_eq!(
            synthesize_response_operator(&[hash_mismatch]),
            Err(SynthesisError::NoConsistentProgram)
        );
    }

    #[test]
    fn project_selected_value_multiline_and_non_scalar_abstain() {
        let multiline = ResponseProgram::project_selected_value(
            ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::String,
            },
            ValueProjectionFormat::PlainText,
            "completed",
        );
        assert_eq!(
            execute_response(
                &multiline,
                "",
                &json!({"input":[{"type":"function_call_output","output":"first\nsecond"}]})
            )
            .status,
            ResponseExecutionStatus::Abstain
        );
        let object = ResponseProgram::project_selected_value(
            ResponseValueSelector::JsonField {
                field: "selected".to_owned(),
                value_type: AtomValueType::String,
            },
            ValueProjectionFormat::CanonicalJson,
            "completed",
        );
        assert_eq!(
            execute_response(
                &object,
                "",
                &json!({"input":[{"type":"function_call_output","output":"{\"selected\":{\"nested\":true}}"}]})
            )
            .status,
            ResponseExecutionStatus::Abstain
        );
    }

    #[test]
    fn project_selected_value_independent_verifier_rejects_mutation() {
        let frame = selected_value_frame(
            'm',
            AtomValueType::Boolean,
            ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::Boolean,
            },
            ValueProjectionFormat::CanonicalJson,
            &"6".repeat(64),
            &"6".repeat(64),
            SOURCE_NEUTRAL_EXTRACTOR_VERSION,
        );
        let operator = synthesize_response_operator(&[frame]).expect("projection synthesis");
        let payload = json!({"input":[{"type":"function_call_output","output":"true"}]});
        assert!(verify_response_independently(&operator.verifier, &payload, "false").is_err());
    }

    #[test]
    fn project_selected_value_packages_and_diagnostics_do_not_persist_secret() {
        let secret = "PRIVATE-SCALAR-7b6d2";
        let frame = selected_value_frame(
            's',
            AtomValueType::String,
            ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::String,
            },
            ValueProjectionFormat::PlainText,
            &"7".repeat(64),
            &"7".repeat(64),
            SOURCE_NEUTRAL_EXTRACTOR_VERSION,
        );
        let package = compile_source_neutral_quarantine_packages(&[frame], true).remove(0);
        let registry = ResponseRegistry {
            schema: "nando.response-registry.v5".to_owned(),
            revision: 1,
            packages: vec![package.clone()],
        };
        assert!(
            !String::from_utf8(serde_json::to_vec(&package).expect("package bytes"))
                .expect("utf8")
                .contains(secret)
        );
        assert!(
            !String::from_utf8(serde_json::to_vec(&registry).expect("registry bytes"))
                .expect("utf8")
                .contains(secret)
        );
        let rejected = execute_response(
            &package.program,
            "",
            &json!({"input":[{"type":"function_call_output","output":format!("{secret}\nsecond")}]}),
        );
        assert!(!rejected.reason.contains(secret));
        assert!(rejected.verification_receipt_id.is_none());
    }

    fn project_status_program(selector: ResponseValueSelector) -> ResponseProgram {
        ResponseProgram::project_status(selector, ProjectStatusMapping::ZeroIsSuccess, "completed")
    }

    fn project_status_verifier(selector: ResponseValueSelector) -> VerifierProgram {
        VerifierProgram::ProjectStatus {
            selector,
            mapping: ProjectStatusMapping::ZeroIsSuccess,
            renderer: CollectionOutputRenderer::Direct,
            completion_state: "completed".to_owned(),
            require_unique_value: true,
        }
    }

    #[test]
    fn project_status_maps_zero_and_bounded_nonzero_to_exact_canonical_text() {
        let cases = [
            (
                ResponseValueSelector::UniqueScalar {
                    value_type: AtomValueType::Integer,
                },
                "0".to_owned(),
                "7".to_owned(),
            ),
            (
                ResponseValueSelector::ContentLinePrefix {
                    prefix: "exit_code=".to_owned(),
                    value_type: AtomValueType::Integer,
                },
                "exit_code=0".to_owned(),
                format!("exit_code={MAX_PROJECT_STATUS_CODE}"),
            ),
            (
                ResponseValueSelector::JsonField {
                    field: "exit_code".to_owned(),
                    value_type: AtomValueType::Integer,
                },
                "{\"exit_code\":0}".to_owned(),
                "{\"exit_code\":23}".to_owned(),
            ),
        ];
        for (selector, zero_output, nonzero_output) in cases {
            let program = project_status_program(selector.clone());
            let verifier = project_status_verifier(selector);
            for (tool_output, expected) in [
                (zero_output.as_str(), "success"),
                (nonzero_output.as_str(), "failure"),
            ] {
                let payload = json!({
                    "input": [{"type":"function_call_output","output":tool_output}]
                });
                let execution = execute_response(&program, "", &payload);
                assert_eq!(execution.status, ResponseExecutionStatus::Executed);
                assert_eq!(execution.response.as_deref(), Some(expected));
                assert_eq!(
                    execution.response.as_deref().map(str::len),
                    Some(expected.len())
                );
                assert!(execution.verification_receipt_id.is_none());
                assert!(verify_response(&program, "", &payload, expected).is_ok());
                assert!(verify_response_independently(&verifier, &payload, expected).is_ok());
            }
        }
    }

    #[test]
    fn project_status_content_parts_use_the_exact_actor_and_verifier_allowlist() {
        let selector = ResponseValueSelector::JsonField {
            field: "exit_code".to_owned(),
            value_type: AtomValueType::Integer,
        };
        let program = project_status_program(selector.clone());
        let verifier = project_status_verifier(selector);
        for part_type in ["text", "input_text", "output_text"] {
            let payload = json!({
                "input":[{
                    "type":"function_call_output",
                    "output":[{"type":part_type,"text":"{\"exit_code\":0}"}]
                }]
            });
            let execution = execute_response(&program, "", &payload);
            assert_eq!(execution.status, ResponseExecutionStatus::Executed);
            assert_eq!(execution.response.as_deref(), Some("success"));
            assert!(verify_response_independently(&verifier, &payload, "success").is_ok());
        }

        for output in [
            json!([{"type":"unknown_text","text":"{\"exit_code\":0}"}]),
            json!([
                {"type":"output_text","text":"{\"exit_code\":0}"},
                {"type":"image_text","text":"ignored by permissive parsers"}
            ]),
            json!([{"type":"text","content":"{\"exit_code\":0}"}]),
        ] {
            let payload = json!({
                "input":[{"type":"function_call_output","output":output}]
            });
            assert_eq!(
                execute_response(&program, "", &payload).status,
                ResponseExecutionStatus::Abstain
            );
            assert!(verify_response_independently(&verifier, &payload, "success").is_err());
        }
    }

    #[test]
    fn project_status_abstains_on_missing_multiple_non_integer_unbounded_and_stale_evidence() {
        let program = project_status_program(ResponseValueSelector::ContentLinePrefix {
            prefix: "exit_code=".to_owned(),
            value_type: AtomValueType::Integer,
        });
        for payload in [
            json!({"input":[{"type":"function_call_output","output":"no selected value"}]}),
            json!({"input":[{"type":"function_call_output","output":"exit_code=0\nexit_code=1"}]}),
            json!({"input":[{"type":"function_call_output","output":"exit_code=-1"}]}),
            json!({"input":[{"type":"function_call_output","output":format!("exit_code={}", MAX_PROJECT_STATUS_CODE + 1)}]}),
            json!({"input":[{"type":"function_call_output","output":"exit_code=true"}]}),
            json!({"input":[{"type":"function_call_output","output":"exit_code=success"}]}),
            json!({"input":[{"type":"function_call_output","output":"exit_code=completed successfully"}]}),
            json!({"input":[{"type":"function_call_output","output":"exit_code=1.0"}]}),
            json!({"input":[{"type":"function_call_output","output":"exit_code=0"},{"type":"message","role":"user","content":"new turn"}]}),
            json!({"input":[]}),
        ] {
            let execution = execute_response(&program, "", &payload);
            assert_eq!(execution.status, ResponseExecutionStatus::Abstain);
            assert!(execution.response.is_none());
            assert!(execution.verification_receipt_id.is_none());
        }
    }

    #[test]
    fn project_status_unique_scalar_rejects_enum_bool_prose_and_ambiguous_structures() {
        let program = project_status_program(ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::Integer,
        });
        for output in [
            "\"success\"",
            "true",
            "process exited successfully",
            "{\"left\":0,\"right\":1}",
            "[]",
            "null",
        ] {
            let execution = execute_response(
                &program,
                "",
                &json!({"input":[{"type":"custom_tool_call_output","output":output}]}),
            );
            assert_eq!(execution.status, ResponseExecutionStatus::Abstain);
        }
    }

    #[test]
    fn project_status_contract_and_relation_atom_roundtrip_canonically() {
        let selector = ResponseValueSelector::JsonField {
            field: "exit_code".to_owned(),
            value_type: AtomValueType::Integer,
        };
        let program = project_status_program(selector.clone());
        let verifier = project_status_verifier(selector);
        let atom = RelationAtom::ActionStatusProjection {
            mapping: ProjectStatusMapping::ZeroIsSuccess,
        };

        let program_json = serde_json::to_value(&program).expect("program json");
        assert_eq!(
            program_json.pointer("/operation/op"),
            Some(&json!("project_status"))
        );
        assert_eq!(
            program_json.pointer("/operation/mapping"),
            Some(&json!("zero_is_success"))
        );
        assert_eq!(
            serde_json::from_value::<ResponseProgram>(program_json).expect("program roundtrip"),
            program
        );

        let verifier_json = serde_json::to_value(&verifier).expect("verifier json");
        assert_eq!(verifier_json.get("kind"), Some(&json!("project_status")));
        assert_eq!(
            serde_json::from_value::<VerifierProgram>(verifier_json).expect("verifier roundtrip"),
            verifier
        );

        let atom_json = serde_json::to_value(&atom).expect("atom json");
        assert_eq!(
            atom_json.get("kind"),
            Some(&json!("action_status_projection"))
        );
        assert_eq!(
            serde_json::from_value::<RelationAtom>(atom_json).expect("atom roundtrip"),
            atom
        );
        assert_eq!(
            serde_json::to_string(&ProjectStatusValue::Success).expect("status json"),
            "\"success\""
        );
        assert_eq!(
            serde_json::to_string(&ProjectStatusValue::Failure).expect("status json"),
            "\"failure\""
        );
    }

    #[test]
    fn project_status_verifier_rejects_wrong_mapping_output_and_stale_evidence() {
        let selector = ResponseValueSelector::JsonField {
            field: "exit_code".to_owned(),
            value_type: AtomValueType::Integer,
        };
        let verifier = project_status_verifier(selector);
        let zero = json!({
            "input":[{"type":"function_call_output","output":"{\"exit_code\":0}"}]
        });
        let nonzero = json!({
            "input":[{"type":"function_call_output","output":"{\"exit_code\":9}"}]
        });
        assert!(verify_response_independently(&verifier, &zero, "failure").is_err());
        assert!(verify_response_independently(&verifier, &nonzero, "success").is_err());

        let stale = json!({
            "input":[
                {"type":"function_call_output","output":"{\"exit_code\":0}"},
                {"type":"message","role":"assistant","content":"already consumed"}
            ]
        });
        assert!(verify_response_independently(&verifier, &stale, "success").is_err());
    }

    #[test]
    fn v5_frames_and_existing_function_custom_tool_families_remain_compatible() {
        let mut function_frame = extract_relation_frame(&scalar_transfer_trace(3, 8, false, "v5"));
        function_frame.extractor_version = "response-relation-extractor.v5".to_owned();
        assert!(synthesize_response_operator(&[function_frame]).is_ok());

        let function = extract_relation_frame(&continuation_trace(2, 9, false));
        assert!(matches!(
            synthesize_response_operator(&[function])
                .expect("function synthesis")
                .candidate
                .program
                .operation,
            ResponseOperation::FunctionCallFromRoles { .. }
        ));
        let custom = custom_continuation_frame('z', 'y', 4, 12, "custom");
        assert!(matches!(
            synthesize_response_operator(&[custom])
                .expect("custom synthesis")
                .candidate
                .program
                .operation,
            ResponseOperation::CustomToolCallFromRoles { .. }
        ));
    }

    #[test]
    fn generic_collection_program_projects_filters_counts_and_composes_in_order() {
        let payload = json!({
            "input":[{"type":"function_call_output","output":
                "{\"rows\":[{\"kind\":\"keep\",\"value\":3},{\"kind\":\"drop\",\"value\":4},{\"kind\":\"keep\",\"value\":5}]}"
            }]
        });
        let steps = vec![
            CollectionProgramStep::SelectField {
                field: "rows".to_owned(),
            },
            CollectionProgramStep::FilterFieldEquals {
                field: "kind".to_owned(),
                value: ResponseScalarLiteral::String("keep".to_owned()),
            },
            CollectionProgramStep::ProjectField {
                field: "value".to_owned(),
            },
        ];
        let projection = ResponseProgram::compose_collection(
            steps.clone(),
            ValueProjectionFormat::CanonicalJson,
            "completed",
        );
        let executed = execute_response(&projection, "", &payload);
        assert_eq!(executed.status, ResponseExecutionStatus::Executed);
        assert_eq!(executed.response.as_deref(), Some("[3,5]"));

        let count_steps = vec![
            steps[0].clone(),
            steps[1].clone(),
            CollectionProgramStep::Count,
        ];
        let count = ResponseProgram::compose_collection(
            count_steps.clone(),
            ValueProjectionFormat::PlainText,
            "completed",
        );
        let executed = execute_response(&count, "", &payload);
        assert_eq!(executed.response.as_deref(), Some("2"));
        let verifier = VerifierProgram::ComposeCollection {
            steps: count_steps,
            format: ValueProjectionFormat::PlainText,
            renderer: CollectionOutputRenderer::Direct,
            completion_state: "completed".to_owned(),
            max_items: 1_024,
        };
        assert!(verify_response_independently(&verifier, &payload, "2").is_ok());
        assert!(verify_response_independently(&verifier, &payload, "3").is_err());
    }

    #[test]
    fn generic_collection_program_fails_closed_on_shape_order_and_budget() {
        let payload = json!({
            "input":[{"type":"function_call_output","output":"{\"rows\":[{\"kind\":\"keep\"}]}"}]
        });
        let wrong_order = ResponseProgram::compose_collection(
            vec![
                CollectionProgramStep::Count,
                CollectionProgramStep::SelectField {
                    field: "rows".to_owned(),
                },
            ],
            ValueProjectionFormat::CanonicalJson,
            "completed",
        );
        assert_eq!(
            execute_response(&wrong_order, "", &payload).status,
            ResponseExecutionStatus::Abstain
        );
        let missing = ResponseProgram::compose_collection(
            vec![CollectionProgramStep::SelectField {
                field: "missing".to_owned(),
            }],
            ValueProjectionFormat::CanonicalJson,
            "completed",
        );
        assert_eq!(
            execute_response(&missing, "", &payload).status,
            ResponseExecutionStatus::Abstain
        );
        let mut over_budget = ResponseProgram::compose_collection(
            vec![CollectionProgramStep::SelectField {
                field: "rows".to_owned(),
            }],
            ValueProjectionFormat::CanonicalJson,
            "completed",
        );
        let ResponseOperation::ComposeCollection { max_items, .. } = &mut over_budget.operation
        else {
            unreachable!();
        };
        *max_items = 0;
        assert_eq!(
            execute_response(&over_budget, "", &payload).status,
            ResponseExecutionStatus::Abstain
        );
    }
}
