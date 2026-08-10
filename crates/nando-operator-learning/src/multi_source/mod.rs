//! Source-neutral multi-source discovery contracts.

pub use nando_client_evidence::NandoRouteReceiptV1;

mod audit;
mod coverage_portfolio_shadow_v1;
mod factorizer;
mod failure_corpus;
mod frozen_version_space;
mod identification;
mod join;
mod k1_natural_scheduler_v1;
mod linked_frame_acquisition;
mod live_snapshot;
mod marginal;
mod ms3_generation_registry_v1;
mod natural_program_artifact;
mod natural_vocabulary_census;
mod north_star_cellular_support_v1;
mod north_star_proof_v1;
mod representation_gap;
mod source_neutral_t1;
mod source_neutral_t1_binding;
mod source_neutral_t1_manifest;
mod transport_binding;

pub use audit::{
    AuditMassV1, MULTI_SOURCE_EVIDENCE_AUDIT_SCHEMA_V1, MissingEvidenceFieldV1,
    MultiSourceEvidenceAuditV1, MultiSourceShapeAuditV1, PreActionTopologyAuditRowV1,
    RelationEvidenceAuditV1, RequestStructureAuditRowV1, RequestStructureAuditSnapshotV1,
    build_multi_source_evidence_audit_v1,
};
pub use coverage_portfolio_shadow_v1::{
    COVERAGE_PORTFOLIO_SHADOW_SCHEMA_V1, CoverageCandidateCostV1, CoverageCandidateSafetyV1,
    CoverageIntentReceiptV1, CoveragePackageCandidateV1, CoveragePackageShadowV1,
    CoveragePortfolioConservationV1, CoveragePortfolioShadowErrorV1, CoveragePortfolioShadowV1,
    CoverageReceiptKindV1, CoverageSafetyVetoV1, FrozenCoverageDenominatorV1,
    SelectedCoveragePackageV1, build_coverage_portfolio_shadow_v1,
};
pub use factorizer::{
    CompletedEffectFormV1, FactorizedMultiSourceRowV1, MULTI_SOURCE_FACTORIZED_ROW_SCHEMA_V1,
    MultiSourceReasonV1, PreActionShapeClassV1, SOURCE_NEUTRAL_TOPOLOGY_QUOTIENT_SCHEMA_V2,
    factor_multi_source_row_v1, pre_action_applicability_shape_root_v1,
    source_neutral_topology_quotient_root_v2, source_neutral_topology_root_v1,
};
pub use failure_corpus::{
    MS3_FAILURE_CORPUS_MAX_ROWS_V1, MS3_FAILURE_CORPUS_SCHEMA_V1, Ms3FailureCorpusRowV1,
    Ms3FailureCorpusV1, Ms3FailureDispositionV1, build_ms3_failure_corpus_v1,
};
pub use frozen_version_space::{
    FrozenVersionSpaceContractV1, FrozenVersionSpaceEnvelopeV1,
    MS3_FROZEN_VERSION_SPACE_CONTRACT_SCHEMA_V1, MS3_FROZEN_VERSION_SPACE_CONTRACT_SCHEMA_V2,
    MS3_FROZEN_VERSION_SPACE_ENVELOPE_SCHEMA_V1, MS3_FUTURE_APPLICABILITY_ACQUISITION_FAIL,
    MS3_FUTURE_APPLICABILITY_CONTRACT_SCHEMA_V1, MS3_FUTURE_APPLICABILITY_EVENT_SCHEMA_V1,
    MS3_FUTURE_APPLICABILITY_LEDGER_SCHEMA_V1,
    MS3_FUTURE_APPLICABILITY_MAX_INDEPENDENT_TOPOLOGIES_V1,
    MS3_FUTURE_APPLICABILITY_REPORT_SCHEMA_V1, MS3_FUTURE_APPLICABILITY_WINDOW_SECONDS_V1,
    MS3_FUTURE_PREDICTION_SCHEMA_V1, MS3_INDEPENDENT_FUTURE_ENVELOPE_SCHEMA_V1,
    MS3_INDEPENDENT_FUTURE_RECEIPT_SCHEMA_V1, MS3_PRE_FREEZE_BUFFER_EXCLUDED,
    Ms3CompletedFrameCaptureFenceV1, Ms3FrozenVersionSpaceErrorV1, Ms3FrozenVersionSpaceStateV1,
    Ms3FutureApplicabilityContractV1, Ms3FutureApplicabilityDispositionV1,
    Ms3FutureApplicabilityEventV1, Ms3FutureApplicabilityLedgerV1, Ms3FutureApplicabilityReportV1,
    Ms3FutureApplicabilityV1, Ms3FutureApplicabilityVerdictV1, Ms3FuturePredictionV1,
    Ms3IndependentFutureEnvelopeV1, Ms3IndependentFutureReceiptV1, Ms3IndependentFutureVerdictV1,
    Ms3VersionSpaceVersionsV1, Ms3ZeroClassReasonV1, PreparedMs3VersionSpaceV1,
    classify_ms3_unique_law_v1, predict_ms3_unique_law_v1, prepare_ms3_frozen_version_space_v1,
    prepare_ms3_frozen_version_space_with_denominator_v1, seal_ms3_independent_future_v1,
    seal_ms3_independent_future_with_route_receipt_v1,
};
pub use identification::{
    FrozenRawPhaseT1ContractV1, MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V2,
    MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V3, MULTI_SOURCE_T1_IDENTIFICATION_SCHEMA_V3,
    MULTI_SOURCE_T1_PROOF_BASIS_SCHEMA_V1, MultiSourceT1IdentificationStateV1,
    MultiSourceT1IdentificationV3, MultiSourceT1ProofBasisV1, NATURAL_T1_DISCOVERY_BASIS_SCHEMA_V1,
    NATURAL_T1_DISCOVERY_BASIS_SCHEMA_V2, NATURAL_T1_DISCOVERY_BASIS_SCHEMA_V3,
    NATURAL_T1_K0_CURRICULUM_SCHEMA_V1, NATURAL_T1_KNOWN_PROTOCOL_MODE_SET_SCHEMA_V1,
    NATURAL_T1_VERIFIER_SEMANTICS_SCHEMA_V1, PassiveT1ProbeContractV1,
    RAW_PHASE_EXECUTABLE_BLUEPRINT_BUILDER_V1, RAW_PHASE_EXECUTABLE_BLUEPRINT_ENVELOPE_SCHEMA_V1,
    RAW_PHASE_EXECUTABLE_EVIDENCE_SCHEMA_V1, RAW_PHASE_SELECTED_EXECUTABLE_RECEIPT_SCHEMA_V1,
    RAW_PHASE_T1_HYPOTHESIS_ENVELOPE_SCHEMA_V1, RAW_PHASE_T1_HYPOTHESIS_GENERATOR_V1,
    RawPhaseExecutableBlueprintDispositionV1, RawPhaseExecutableBlueprintEnvelopeV1,
    RawPhaseExecutableBlueprintExclusionV1, RawPhaseExecutableEvidenceV1,
    RawPhaseRebuiltExecutableBlueprintV1, RawPhaseSelectedExecutableReceiptV1,
    RawPhaseT1HypothesisEnvelopeV1, RawPhaseT1HypothesisScoreV1, active_t1_protocol_mode_root_v1,
    identify_multi_source_t1_operator_v1,
    identify_multi_source_t1_operator_with_active_protocols_v1,
    identify_multi_source_t1_operator_with_candidate_artifacts_v1,
    identify_multi_source_t1_operator_with_frozen_raw_phase_v1, natural_t1_discovery_basis_root_v1,
    natural_t1_discovery_basis_root_v2, natural_t1_discovery_basis_root_v3,
    raw_phase_executable_runtime_selectors_v1, raw_phase_executable_surface_bundle_v1,
    rebuild_raw_phase_selected_executable_v1, seal_raw_phase_t1_hypothesis_envelope_v1,
};
pub use join::{
    BLIND_THEN_REVEAL_JOIN_SCHEMA_V1, BLIND_THEN_REVEAL_JOIN_SCHEMA_V2,
    BlindThenRevealJoinedTransitionV1, CompletedEffectAtomV1,
    MULTI_SOURCE_CAPTURE_GENERATION_SCHEMA_V1, MULTI_SOURCE_CAPTURE_GENERATION_SCHEMA_V2,
    MULTI_SOURCE_JOIN_MAX_ROWS_V1, MultiSourceJoinCensoredReasonV1, MultiSourceJoinLedgerV1,
    MultiSourceJoinReportV1, ObservedTeacherActionRefV1, PreparedMultiSourceJoinFrameV1,
    VerifiedOutcomeReceiptRefV1, join_prepared_multi_source_frame_v1,
    prepare_multi_source_join_frame_v1, stream_multi_source_joins_from_iter,
    validate_pre_action_topology_join_eligibility_v1,
};
pub use k1_natural_scheduler_v1::{
    K1_CANDIDATE_READINESS_MIN_LINEAGES_V1, K1_CANDIDATE_READINESS_MIN_SETTLED_ROWS_V1,
    K1_CANDIDATE_READINESS_MIN_VERIFIED_ROWS_V1, K1_DEFICIT_SNAPSHOT_SCHEMA_V1,
    K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1, K1_IDENTIFICATION_FREEZE_SCHEMA_V1,
    K1_MISSING_COMPLETED_FRAME_BLOCKER_V1, K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V1,
    K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V2, K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V3,
    K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V4, K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V5,
    K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V1, K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V1,
    K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V2, K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V3,
    K1_NATURAL_COHORT_CATALOG_SCHEMA_V1, K1_NATURAL_EVIDENCE_ROW_SCHEMA_V1,
    K1_NATURAL_EVIDENCE_ROW_SCHEMA_V2, K1_NATURAL_EVIDENCE_ROW_SCHEMA_V3,
    K1_PROBE_ROUND_RECEIPT_SCHEMA_V1, K1_SCHEDULER_SCHEMA_V1, K1CandidateReadinessV1,
    K1CandidateScoreV1, K1ConsequenceTypeV1, K1DeficitSnapshotV1, K1FutureOutcomeReceiptV1,
    K1FuturePredictionCensorReceiptV1, K1FuturePredictionContractV1, K1FuturePredictionReceiptV1,
    K1GenerationBudgetV1, K1GenerationTerminalVerdictV1, K1GenerationVerdictClassV1,
    K1IdentificationFreezeV1, K1NaturalCandidateFreezeV1, K1NaturalCandidateQueueRowV1,
    K1NaturalCandidateQueueV1, K1NaturalCohortCandidateV1, K1NaturalCohortCatalogV1,
    K1NaturalEvidenceClassV1, K1NaturalEvidenceRowV1, K1PreActionExecutionReceiptV1,
    K1ProbeBudgetRemainingV1, K1ProbeClassPredictionV1, K1ProbeRoundReceiptV1, K1ProbeRoundStateV1,
    K1SchedulerEventPayloadV1, K1SchedulerEventV1, K1SchedulerLedgerV1, K1TransferSettlementV1,
    build_k1_natural_candidate_queue_v1, build_k1_natural_candidate_queue_with_exclusions_v1,
    build_k1_natural_cohort_catalog_v1, observed_typed_consequence_root_v1,
    typed_consequence_root_v1,
};
pub use linked_frame_acquisition::{
    MS3_CENSORED_INELIGIBLE_PROBE, MS3_CENSORED_PRE_ROUTE_RECEIPT_EPOCH,
    MS3_CENSORED_UNATTRIBUTED_PROBE, MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V1,
    MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V2,
    MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V3, MS3_LINKED_FRAME_ACQUISITION_FAIL,
    MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V1, MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V2,
    MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V3, MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V4,
    MS3_LINKED_FRAME_ELIGIBILITY_POLICY_V1, MS3_LINKED_FRAME_ELIGIBILITY_POLICY_V2,
    MS3_LINKED_FRAME_RECEIPT_SCHEMA_V1, MS3_RECEIPT_LAG_SLO_SECONDS_V1,
    MS3_SCIENTIFIC_DENOMINATOR_ENVELOPE_SCHEMA_V1, MS3_SCIENTIFIC_DENOMINATOR_RECEIPT_SCHEMA_V1,
    MS3_SCIENTIFIC_TOPOLOGY_SETTLEMENT_SCHEMA_V1, Ms3AcquisitionTopologySelectionV1,
    Ms3CandidateSettlementClassV1, Ms3LinkedFrameAcquisitionContractV1,
    Ms3LinkedFrameAcquisitionReportV1, Ms3LinkedFrameAcquisitionVerdictV1, Ms3LinkedFrameReceiptV1,
    Ms3ScientificDenominatorEnvelopeV1, Ms3ScientificDenominatorReceiptV1,
    Ms3ScientificDenominatorReconstructionV1, Ms3ScientificTopologySettlementV1,
    REPRESENTATION_GAP_CLASSIFIER_VERSION_V1,
    build_ms3_linked_frame_acquisition_report_excluding_used_evidence_v1,
    build_ms3_linked_frame_acquisition_report_v1,
    build_ms3_linked_frame_acquisition_report_with_route_bound_evidence_v1,
    build_ms3_scientific_denominator_receipt_v1, close_ms3_pre_route_receipt_epoch_v1,
    select_ms3_linked_frame_acquisition_topologies_v1,
    select_ms3_linked_frame_acquisition_topologies_with_route_bound_evidence_v1,
    validate_ms3_scientific_denominator_evidence_v1,
};
pub use live_snapshot::{
    LIVE_MULTI_SOURCE_DISCOVERY_SNAPSHOT_SCHEMA_V3, LiveMultiSourceDiscoveryBlockerV1,
    LiveMultiSourceDiscoverySnapshotV3, build_live_multi_source_discovery_snapshot_v3,
    build_live_multi_source_discovery_snapshot_with_active_protocols_v3,
};
pub use marginal::{
    COVERAGE_OPPORTUNITY_MAX_ROWS_V1, COVERAGE_OPPORTUNITY_SNAPSHOT_SCHEMA_V1,
    CoverageOpportunitySnapshotV1, MarginalShapeOpportunityV1,
    build_coverage_opportunity_snapshot_v1,
};
pub use ms3_generation_registry_v1::{
    MS3_CAPTURE_GAP_REPAIR_REQUIRED, MS3_GENERATION_ACQUISITION_FAILURE_SCHEMA_V1,
    MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V1,
    MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V2,
    MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V3,
    MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V4, MS3_GENERATION_REGISTRY_SCHEMA_V1,
    MS3_GENERATION_TERMINAL_SCHEMA_V1, MS3_LINKED_EVIDENCE_REUSE,
    Ms3GenerationAcquisitionFailureReceiptV1, Ms3GenerationEntryV1,
    Ms3GenerationLinkedAcquisitionFailureReceiptV1, Ms3GenerationRegistryErrorV1,
    Ms3GenerationRegistryV1, Ms3GenerationTerminalReceiptV1,
};
pub use natural_program_artifact::{
    NATURAL_T1_PROGRAM_ARTIFACT_MAX_PROGRAMS_V1, NATURAL_T1_PROGRAM_ARTIFACT_SCHEMA_V1,
    NaturalT1ProgramArtifactV1,
};
pub use natural_vocabulary_census::{
    NATURAL_VOCABULARY_CENSUS_SCHEMA_V1, NaturalVocabularyCensusV1,
    NaturalVocabularyCensusVerdictV1, NaturalVocabularyFormCensusV1,
    NaturalVocabularyOperationFormV1, build_natural_vocabulary_census_v1,
};
pub use north_star_cellular_support_v1::{
    NORTH_STAR_CELLULAR_SUPPORT_SCHEMA_V1, NorthStarCellularSupportErrorV1,
    NorthStarCellularSupportReportV1, NorthStarCellularSupportV1,
    synthesize_north_star_cellular_support_v1,
};
pub use north_star_proof_v1::{
    NORTH_STAR_MIN_PASSING_SEEDS_V1, NORTH_STAR_PROOF_CONTRACT_SCHEMA_V1,
    NORTH_STAR_PROOF_REPORT_SCHEMA_V1, NORTH_STAR_REQUIRED_SEEDS_V1, NorthStarArmMetricsV1,
    NorthStarBudgetV1, NorthStarProofArmV1, NorthStarProofContractV1, NorthStarProofErrorV1,
    NorthStarProofReportV1, NorthStarProofSeedReceiptV1, NorthStarProofThresholdsV1,
    NorthStarProofVerdictV1, NorthStarSeedConditionsV1, evaluate_north_star_proof_v1,
};
pub use representation_gap::{
    REPRESENTATION_GAP_ADJUDICATION_SCHEMA_V1, REPRESENTATION_GAP_REPORT_SCHEMA_V1,
    RepresentationGapAdjudicationReportV1, RepresentationGapAdjudicationV1,
    RepresentationGapClassV1, build_representation_gap_adjudication_report_v1,
};
pub use source_neutral_t1::{pre_action_t1_binding_root, t1_program_is_consistent};
pub use source_neutral_t1_manifest::{
    PreActionT1ConsumedInputV1, PreActionT1InputBindingManifestV1, PreActionT1SelectorOriginV1,
    pre_action_t1_input_binding_manifest_v1,
};
pub use transport_binding::{
    REQUEST_ACTION_BINDING_SCHEMA_V1, RequestActionBindingV1, TRANSPORT_BOUND_JOIN_MAX_ROWS_V1,
    TRANSPORT_TERMINAL_RECEIPT_SCHEMA_V1, TransportBindingFailureV1, TransportBindingLedgerV1,
    TransportBoundJoinedTransitionV1, TransportJoinRejectionV1, TransportTerminalReceiptV1,
    bind_independent_fallback_transition_v1, validate_request_action_binding_v1,
};

#[cfg(test)]
mod tests;
