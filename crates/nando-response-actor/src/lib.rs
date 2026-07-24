mod admission_bundle;
mod authority;
mod backward_wave;
mod binding_evidence;
mod binding_evidence_adjudication;
mod binding_evidence_capture_owner;
mod binding_evidence_future_capture;
mod binding_evidence_preregistration;
mod capture_provenance;
mod capture_transition_binding_archive_reader;
mod causal;
mod cegis;
mod collection_synthesis;
mod contracts;
mod crystallized_collection_candidate;
mod crystallized_operator;
mod decidability;
mod effect_graph;
mod effect_law;
mod effect_law_v3;
mod evidence;
mod evidence_graph;
mod executable_protocol_mode;
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
mod operator_vm;
mod operator_vm_compiler;
mod opportunity;
mod outcome_example;
mod output_graph;
mod package;
mod program;
mod protocol_mode;
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
pub use binding_evidence::{
    BINDING_VERSION_SPACE_REPORT_SCHEMA_V1, BindingBaselineOutcomeV1, BindingCallLineageV1,
    BindingCandidateFeaturesV1, BindingCandidateNodeV1, BindingCandidateRelationEdgeV1,
    BindingCandidateRelationKindV1, BindingCapabilityClassV1, BindingCompletionStateV1,
    BindingDistinguishingProbeV1, BindingEvaluationLabelV1, BindingEvidenceBudgetV1,
    BindingEvidenceErrorV1, BindingHypothesisScoreV1, BindingPredicateV1, BindingRequestRelationV1,
    BindingRowAccountingV1, BindingSourceEventClassV1, BindingTieV1, BindingValueTypeV1,
    BindingVersionSpaceReportV1, BindingVersionSpaceVerdictV1, CANDIDATE_RELATION_GRAPH_SCHEMA_V1,
    CandidateRelationGraphV1, EXPECTED_BINDING_RECEIPT_SCHEMA_V1, ExpectedBindingReceiptV1,
    FROZEN_CANDIDATE_RELATION_GRAPH_SCHEMA_V1, FrozenCandidateRelationGraphV1,
    MAX_BINDING_CANDIDATES_PER_ROW_V1, MAX_BINDING_HYPOTHESES_V1, MAX_BINDING_JSON_NODES_V1,
    MAX_BINDING_PREDICATES_PER_HYPOTHESIS_V1, MAX_BINDING_RECENT_EVENTS_V1,
    MAX_BINDING_RELATION_EDGES_PER_ROW_V1, MAX_BINDING_REPORT_HYPOTHESES_V1,
    MAX_BINDING_REPORT_TIES_V1, MAX_BINDING_TEXT_BYTES_V1, PRE_ACTION_BINDING_SURFACE_SCHEMA_V1,
    PreActionBindingContextV1, PreActionBindingSurfaceV1, evaluate_binding_version_space_v1,
};
pub use binding_evidence_adjudication::{
    AcceptedBindingEvidenceScopeV2, AcceptedBindingLawEvidenceV2,
    BINDING_ADJUDICATION_REPORT_SCHEMA_V1, BINDING_ADJUDICATION_REPORT_SCHEMA_V2,
    BINDING_EXTERNAL_LABEL_TRUST_SCHEMA_V1, BINDING_PHYSICAL_LABEL_RECEIPT_SCHEMA_V1,
    BINDING_PHYSICAL_LABEL_SET_SCHEMA_V1, BindingAdjudicationErrorV1, BindingAdjudicationOutcomeV2,
    BindingAdjudicationReportV2, BindingCausalAdjudicationReportV1, BindingEvidencePartitionV2,
    BindingExternalLabelTrustReceiptV1, BindingHypothesisAdjudicationStatusV1,
    BindingInterventionAdjudicationV1, BindingLawEvidenceV2Error, BindingObservedCandidateV1,
    BindingObservedParentV1, BindingObservedRelationV1, BindingPhysicalActorOutcomeV1,
    BindingPhysicalCandidateTrialV1, BindingPhysicalLabelReceiptSetV1,
    BindingPhysicalLabelReceiptV1, BindingPhysicalRelationStateV1, BindingTrialEvidenceLabelV2,
    FrozenBindingTrialRowV2, INDEPENDENT_TRIAL_VERIFIER_RECEIPT_SCHEMA_V2,
    IndependentTrialVerifierInputV2, IndependentTrialVerifierOutcomeV2,
    IndependentTrialVerifierReceiptV2, PHYSICAL_ACTOR_OBSERVATION_SCHEMA_V2,
    PHYSICAL_TRIAL_RECEIPT_SCHEMA_V2, PhysicalActorObservationInputV2, PhysicalActorObservationV2,
    PhysicalActorOutcomeV2, PhysicalTrialJoinedRootsV2, PhysicalTrialOutcomeV2,
    PhysicalTrialReceiptV2, PhysicalTrialV2Error, TRUSTED_RESOLVED_BINDING_ROWS_SCHEMA_V2,
    TrustedBindingResolverInputV2, TrustedBindingResolverReceiptSourceV2,
    TrustedResolvedBindingRowV2, TrustedResolvedBindingRowsV2, TrustedResolverV2Error,
    adjudicate_binding_hypotheses_v1, adjudicate_binding_law_evidence_v2,
    build_binding_label_manifest_v1, observe_frozen_binding_labels_v1, observe_physical_actor_v2,
    resolve_trusted_binding_rows_v2, seal_binding_external_label_trust_v1,
    seal_physical_trial_receipt_v2, trusted_binding_resolver_manifest_root_v2,
    verify_independent_physical_trial_v2,
};
pub use binding_evidence_capture_owner::{
    BINDING_SUPPORT_CAPTURE_BATCH_SCHEMA_V1, BINDING_SUPPORT_CAPTURE_ROW_SCHEMA_V1,
    BINDING_SUPPORT_FREEZE_REPORT_SCHEMA_V1, BINDING_SUPPORT_FREEZE_SCHEMA_V1,
    BindingSupportCaptureBatchV1, BindingSupportCaptureErrorV1, BindingSupportCaptureOwnerV1,
    BindingSupportCaptureRowV1, BindingSupportFreezeReportV1, BindingSupportFreezeV1,
    MAX_BINDING_SUPPORT_CAPTURE_ROWS_V1, MIN_BINDING_SUPPORT_CAPTURE_ROWS_V1,
};
pub use binding_evidence_future_capture::{
    BINDING_FUTURE_ACQUISITION_PROTOCOL_SCHEMA_V1, BINDING_FUTURE_CAPTURE_BATCH_SCHEMA_V1,
    BINDING_FUTURE_CAPTURE_FREEZE_SCHEMA_V1, BINDING_FUTURE_CAPTURE_ROW_SCHEMA_V1,
    BindingFutureAcquisitionProtocolV1, BindingFutureCaptureBatchV1, BindingFutureCaptureErrorV1,
    BindingFutureCaptureFreezeV1, BindingFutureCaptureInputV1, BindingFutureCaptureOwnerV1,
    BindingFutureCaptureReportV1, BindingFutureCaptureSlotV1, BindingFutureChallengeContractV1,
    BindingFutureSourceContractV1, binding_future_acquisition_protocol_v1,
};
pub use binding_evidence_preregistration::{
    BINDING_CAPTURE_WATERMARK_SCHEMA_V1, BINDING_EVIDENCE_PREREGISTRATION_SCHEMA_V1,
    BINDING_LABEL_ENVELOPE_SCHEMA_V1, BINDING_LABEL_MANIFEST_SCHEMA_V1,
    BindingCaptureReceiptEntryV1, BindingCausalHypothesisKindV1, BindingCausalHypothesisStatusV1,
    BindingCausalHypothesisV1, BindingCausalInterventionV1, BindingEvidencePartitionV1,
    BindingEvidencePreregistrationV1, BindingInterventionPredictionV1,
    BindingLabelObservationSourceV1, BindingLineageSplitContractV1, BindingPreregistrationErrorV1,
    BindingTrustedLabelContractV1, MAX_BINDING_LABEL_ENVELOPES_V1,
    MIN_BINDING_SESSION_LINEAGES_PER_PARTITION_V1, TrustedBindingCaptureWatermarkRootV1,
    TrustedBindingLabelManifestRootV1, TrustedBindingLabelSetV1,
    UntrustedBindingCaptureWatermarkV1, UntrustedBindingLabelEnvelopeV1,
    UntrustedBindingLabelManifestV1, binding_evidence_preregistration_v1,
    resolve_trusted_binding_label_set_v1,
};
pub use capture_provenance::{
    CAPTURE_COMMITMENT_INDEX_SCHEMA_V1, CAPTURE_EVIDENCE_RECEIPT_SCHEMA_V1,
    CAPTURE_TRANSITION_BINDING_SCHEMA_V1, CaptureCommitmentArchiveReader, CaptureCommitmentIndex,
    CaptureEvidenceReceipt, CaptureRecordCommitment, CaptureTransitionBinding,
    CaptureTransitionBindingArchiveReader, MAX_CAPTURE_COMMITMENT_INDEX_RECORDS,
    MAX_CAPTURE_RECEIPT_RECORDS, verify_crystallized_capture_provenance,
    verify_crystallized_capture_provenance_durable,
    verify_crystallized_collection_capture_provenance_durable,
};
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
    is_source_neutral_response_program, is_transfer_bound_response_program,
    response_program_authority_matches_example, response_program_dynamic_value_root_sha256,
    response_program_exactly_matches_example, response_program_matches_example,
    response_program_requires_static_frame_transfer, source_neutral_verifier_for_program,
    synthesize_collection_program, synthesize_unique_collection_program,
};
pub use contracts::{
    AtomSource, AtomValueType, FrozenSplitError, GuardCandidate, INGRESS_EVENT_SCHEMA,
    IngressEvent, PROGRAM_CANDIDATE_SCHEMA, RELATION_FRAME_SCHEMA, ROLE_HYPOTHESIS_SCHEMA,
    RelationAtom, RelationFrame, ResponseProgramCandidate, ResponseValueSelector, RoleHypothesis,
    SemanticRole, TrafficClass, VERIFIER_RECEIPT_SCHEMA, VerifierConsensusVariant, VerifierProgram,
    VerifierReceipt, validate_frozen_future_split,
};
pub use crystallized_collection_candidate::{
    CRYSTALLIZED_COLLECTION_ADMISSION_CANDIDATE_SCHEMA_V1,
    CrystallizedCollectionAdmissionCandidateV1,
};
pub use crystallized_operator::{
    BoundCrystallizedOperator, BoundRoleEnvironment, CrystallizationParityReceipt,
    CrystallizedFeedbackError, CrystallizedOperator, CrystallizedOperatorError,
    ExecutableParitySeal, RuntimeRoleAnchor, RuntimeSurfaceEvidence, TRANSFORM_FLAG_CANONICAL_JSON,
    TRANSFORM_OPCODE_COUNT_COLLECTION, TRANSFORM_OPCODE_FILTER_REQUEST_VALUE,
    TRANSFORM_OPCODE_PROJECT_STATUS, TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR, TRANSFORM_ROLE_NONE,
    TRANSFORM_STATUS_ZERO_IS_OK, TRANSFORM_STATUS_ZERO_IS_PASS, TRANSFORM_STATUS_ZERO_IS_SUCCESS,
    TRANSFORM_STATUS_ZERO_IS_TRUE, TRANSFORM_VALUE_BOOLEAN, TRANSFORM_VALUE_COLLECTION,
    TRANSFORM_VALUE_IDENTIFIER, TRANSFORM_VALUE_INTEGER, TRANSFORM_VALUE_STRING,
    VerifiedCrystallizedOperator, VerifiedOperatorRestartBundle, crystallization_raw_input_sha256,
};
pub use decidability::{CpuDecidability, CpuDecidabilityClass, classify_cpu_decidability};
pub use effect_graph::{
    EFFECT_GRAPH_SCHEMA_V1, EffectEdge, EffectEdgeKind, EffectGraph, EffectGraphBuilder,
    EffectGraphCompleteness, EffectGraphPolicy, EffectNode, EffectNodeKind, EffectOperationKind,
    EffectSource,
};
pub use effect_law::{
    CANONICAL_EFFECT_LAW_SCHEMA_V2, CanonicalEffectLawV2, CanonicalEffectTopologyV2,
    CanonicalNodeMappingEntryV2, CanonicalizedEffectLawV2, EFFECT_NODE_COLLECTION,
    EFFECT_NODE_OPERATION, EFFECT_NODE_SCALAR, EFFECT_OBSERVATION_SCHEMA_V2,
    EFFECT_OPCODE_ASSERT_CONSTANT, EFFECT_OPCODE_COMPOSE, EFFECT_OPCODE_CONSUME,
    EFFECT_OPCODE_COPY, EFFECT_OPCODE_EQUAL, EFFECT_OPCODE_PRESERVE, EFFECT_OPCODE_PRODUCE,
    EFFECT_OPCODE_REQUIRE, EFFECT_OPERATION_CALL, EFFECT_OPERATION_PLAN_ADVANCE,
    EFFECT_OPERATION_PROJECT, EFFECT_OPERATION_STATUS, EFFECT_QUOTIENT_REPORT_SCHEMA_V2,
    EFFECT_VALUE_BOOLEAN, EFFECT_VALUE_COLLECTION, EFFECT_VALUE_IDENTIFIER, EFFECT_VALUE_INTEGER,
    EFFECT_VALUE_OPAQUE_SCALAR, EFFECT_VALUE_OPERATION, EFFECT_VALUE_STRING, EffectClauseV2,
    EffectLawDictionaryRootsV2, EffectLawError, EffectLawId, EffectLawProgramV2,
    EffectLawQuotientReportV2, EffectOpcodeV2, EffectRoleV2, EffectTopologyEdgeV2,
    EffectTopologyNodeV2, EffectValueTypeV2, EvidenceBoundEffectObservationV2,
    ObservationNodeMappingV2, PhysicalEffectArgumentV2, PreservedFrameContractV2,
    ProtocolModeDifferenceV2, RoleCardinalityV2, RoleRefV2, TypedConstantCommitmentV2,
    observe_effect_transition_v2, search_effect_law_quotient_v2,
};
pub use effect_law_v3::{
    CANONICAL_EFFECT_LAW_SCHEMA_V3, CanonicalEffectEdgeV3, CanonicalEffectLawCandidateV3,
    CanonicalEffectLawV3, CanonicalEffectNodeV3, CanonicalNodeMappingV3, CanonicalRelationClauseV3,
    EFFECT_ATOM_ACTION_RELATION, EFFECT_ATOM_CARDINALITY, EFFECT_ATOM_PHYSICAL_SURFACE,
    EFFECT_ATOM_POSTCONDITION, EFFECT_ATOM_PRECONDITION, EFFECT_ATOM_RENDERER,
    EFFECT_ATOM_TEMPORAL, EFFECT_DELTA_CONTRACT_SCHEMA_V3,
    EFFECT_LAW_DUAL_CLASSIFICATION_REPORT_SCHEMA_V3, EFFECT_LAW_RESTART_BUNDLE_SCHEMA_V3,
    EFFECT_OBSERVATION_CANDIDATE_SCHEMA_V3, EFFECT_QUOTIENT_HYPOTHESIS_SCHEMA_V3,
    EFFECT_REL_CONSTANT, EFFECT_REL_CONSUME, EFFECT_REL_COPY, EFFECT_REL_EQUAL, EFFECT_REL_REQUIRE,
    EffectDeltaContractV3, EffectDictionaryEntryV3, EffectLawDictionaryV3,
    EffectLawDualClassificationDiscrepancyDirectionV3, EffectLawDualClassificationDiscrepancyV3,
    EffectLawDualClassificationDiscrepancyWitnessV3, EffectLawDualClassificationMapV3,
    EffectLawDualClassificationReasonV3, EffectLawDualClassificationReportV3,
    EffectLawDualClassificationRowReportV3, EffectLawDualClassificationRowStatusV3,
    EffectLawDualClassificationRowV3, EffectLawDualClassificationVerdictV3,
    EffectLawDualClassifierV3, EffectLawDualIndependenceReportV3, EffectLawIdV3,
    EffectLawIndependenceV3, EffectLawQuotientReportV3, EffectLawRestartBundleV3, EffectLawV3Error,
    EffectObservationCandidateV3, EffectQuotientHypothesisV3, ExactEffectAtomV3,
    INDEPENDENT_EFFECT_STATE_SCHEMA_V3, ObservationCanonicalProofV3, PROTOCOL_FACET_SCHEMA_V3,
    SEALED_EFFECT_OBSERVATION_SCHEMA_V3, SealedEffectObservationV3,
    TRUSTED_EFFECT_EVIDENCE_SET_SCHEMA_V3, TRUSTED_EFFECT_LAW_BUNDLE_ROOT_SCHEMA_V3,
    TRUSTED_GENERATION_MANIFEST_SCHEMA_V3, TrustedEffectEvidenceSetV3,
    TrustedEffectLawBundleRootV3, TrustedGenerationManifestRootV3,
    VERIFIED_EFFECT_DELTA_RECEIPT_SCHEMA_V3, VerifiedEffectDeltaReceiptV3,
    observe_effect_transition_v3, resolve_trusted_effect_evidence_set_v3,
    seal_effect_observation_v3, search_effect_law_quotient_v3,
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
pub use executable_protocol_mode::{
    EXECUTABLE_PROTOCOL_MODE_ARTIFACT_SCHEMA_V3, ExecutableProtocolModeArtifactV3,
    ExecutableProtocolModeErrorV3, ExecutableProtocolModeV3, PROTOCOL_FACET_PAYLOAD_SCHEMA_V3,
    ProtocolCapabilityArgumentV3, ProtocolCapabilityKindV3, ProtocolDefaultSemanticsV3,
    ProtocolFacetEvidenceInputV3, ProtocolFacetPayloadV3, ProtocolPhysicalSymbolSourceV3,
    compile_executable_protocol_mode_artifact_v3,
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
    OnlineAdmissionCandidateRejection, OnlineAdmissionEvaluation, OnlineAdmissionSnapshot,
    build_crystallized_admission_snapshot, build_durable_runtime_parity_receipt,
    build_online_admission_evaluation, build_online_admission_snapshot,
    build_online_collection_admission_snapshot, merge_online_admission_snapshots,
    merge_with_active_online_admission, merge_with_proven_active_online_admission,
    reissue_unrevoked_active_online_admission, remove_runtime_revoked_online_admission,
};
pub use online_checkpoint::{
    FramedCborLedger, FramedLedgerStatus, FramedRecordRef, read_framed_cbor, write_atomic_cbor,
};
pub use online_collection::{
    LegacyReplayRehydrationStats, OnlineCollectionAdmissionCandidate, OnlineCollectionBucketStatus,
    OnlineCollectionConfig, OnlineCollectionConsensusDiagnostic, OnlineCollectionMiner,
    OnlineCollectionObservation, OnlineCollectionProofMode, OnlineCollectionReceipt,
    OnlineCollectionRehydrationHint, OnlineCollectionStatus, OnlineCollectionWaveCausalReport,
    online_collection_adaptive_transfer_proof_digest, online_collection_candidate_freeze,
    online_collection_future_manifest_digest, online_collection_support_manifest_digest,
};
pub use online_state::{
    MinerSignalStageReport, MinerSignalTreeReport, SELF_TRAINING_STATE_SCHEMA_V2,
    SELF_TRAINING_STATE_SCHEMA_V3, SELF_TRAINING_STATE_SCHEMA_V4, SELF_TRAINING_STATE_SCHEMA_V5,
    SEMANTIC_EVIDENCE_RECEIPT_SCHEMA_V1, SEMANTIC_LAW_EVIDENCE_AUDIT_SCHEMA_V1,
    SelfTrainingAdmissionCohort, SelfTrainingGenerationReport, SelfTrainingStateReport,
    SemanticEvidenceOutcome, SemanticEvidenceReceipt, SemanticLawActorAudit,
    SemanticLawActorReplayOutcome, SemanticLawEvidenceAudit, SemanticLawEvidenceAuditRow,
    SemanticLawSelectorCandidate, SemanticLawValueOccurrence, StreamingSelfTrainingState,
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
#[cfg(test)]
pub(crate) use package::stable_atom_id;
pub use package::{
    COLLECTION_EXTERNAL_VERIFIER_SCHEMA, CONTINUATION_EXTERNAL_VERIFIER_SCHEMA,
    CUSTOM_TOOL_EXTERNAL_VERIFIER_SCHEMA, LearnedWaveRoute, LearnedWaveSubcenter, ResponseExecutor,
    ResponsePackage, ResponsePackageOrigin, ResponsePackageProof, ResponsePackageState,
    ResponseRegistry, ResponseRoutingComparison, ResponseRoutingPredicate, RoutedResponseExecution,
    SOURCE_VALUE_EXTERNAL_VERIFIER_SCHEMA, STATUS_PROJECTION_EXTERNAL_VERIFIER_SCHEMA,
    VALUE_PROJECTION_EXTERNAL_VERIFIER_SCHEMA, provider_tool_capability_atom_ids,
    relation_frame_online_routing_atom_ids, relation_frame_phase_atom_ids,
    relation_frame_phase_margin_micro, relation_frame_required_observable_atom_ids,
    relation_frame_routes_to_package, relation_frame_routing_atom_ids, request_phase_atom_ids,
    response_program_external_verifier_schema, response_program_required_routing_atom_ids,
};
pub use program::{
    CollectionAggregateOperation, CollectionOutputRenderer, CollectionProgramStep,
    CollectionScalarType, CustomToolResultProjection, MAX_PROJECT_STATUS_CODE,
    MAX_RESPONSE_RENDER_DYNAMIC_SEGMENTS, MAX_RESPONSE_RENDER_SEGMENTS,
    MAX_RESPONSE_STATIC_TEXT_BYTES, ProjectStatusMapping, ProjectStatusValue,
    RequestTemplateMarker, ResponseAdapterWaveConsensus, ResponseAdapterWaveRoute,
    ResponseAdapterWaveSubcenter, ResponseArgument, ResponseConsensusVariant, ResponseOperation,
    ResponseProgram, ResponseRenderSegment, ResponseScalarLiteral, ValueProjectionFormat,
    collection_static_text_rejection_reason,
};
pub use protocol_mode::{
    BindingProtocolCompileVerdictV2, BindingProtocolCompilerErrorV2, PROTOCOL_MODE_SET_SCHEMA_V2,
    ProtocolArgumentRoleSchemaV2, ProtocolArgumentRoleV2, ProtocolCapabilityContractV2,
    ProtocolConstantContractV2, ProtocolModeCompilerBudgetV2, ProtocolModeProgramV2,
    ProtocolModeSetV2, ProtocolModeV2, ProtocolRoleCardinalityV2, ProtocolSelectorProgramV2,
    ProtocolSourceRoleSchemaV2, ProtocolSourceRoleV2, ProtocolStructuralGuardV2,
    ProtocolTemporalCardinalityContractV2, ProtocolValueContractV2,
    compile_protocol_modes_for_effect_law_v3,
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
pub use verifier::{
    ResponseVerificationError, verify_response, verify_response_independently,
    verify_response_independently_with_request,
};
pub use version_space::{
    AstNodeId, AstProgramKind, InternedProgram, VersionSpaceArena, VersionSpaceConfig,
    VersionSpaceReport, response_program_depth, response_program_kind,
};

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
