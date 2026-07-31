//! Source-neutral multi-source discovery contracts.

mod audit;
mod coverage_portfolio_shadow_v1;
mod factorizer;
mod failure_corpus;
mod frozen_version_space;
mod identification;
mod join;
mod linked_frame_acquisition;
mod live_snapshot;
mod marginal;
mod ms3_generation_registry_v1;
mod north_star_cellular_support_v1;
mod north_star_proof_v1;
mod representation_gap;
mod source_neutral_t1;
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
    MultiSourceReasonV1, PreActionShapeClassV1, factor_multi_source_row_v1,
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
    MULTI_SOURCE_T1_IDENTIFICATION_SCHEMA_V3, MULTI_SOURCE_T1_PROOF_BASIS_SCHEMA_V1,
    MultiSourceT1IdentificationStateV1, MultiSourceT1IdentificationV3, MultiSourceT1ProofBasisV1,
    PassiveT1ProbeContractV1, active_t1_protocol_mode_root_v1,
    identify_multi_source_t1_operator_v1,
    identify_multi_source_t1_operator_with_active_protocols_v1,
};
pub use join::{
    BLIND_THEN_REVEAL_JOIN_SCHEMA_V1, BlindThenRevealJoinedTransitionV1, CompletedEffectAtomV1,
    MULTI_SOURCE_JOIN_MAX_ROWS_V1, MultiSourceJoinCensoredReasonV1, MultiSourceJoinLedgerV1,
    MultiSourceJoinReportV1, ObservedTeacherActionRefV1, VerifiedOutcomeReceiptRefV1,
    validate_pre_action_topology_join_eligibility_v1,
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
pub use source_neutral_t1::pre_action_t1_binding_root;
pub use transport_binding::{
    REQUEST_ACTION_BINDING_SCHEMA_V1, RequestActionBindingV1, TRANSPORT_BOUND_JOIN_MAX_ROWS_V1,
    TRANSPORT_TERMINAL_RECEIPT_SCHEMA_V1, TransportBindingFailureV1, TransportBindingLedgerV1,
    TransportBoundJoinedTransitionV1, TransportJoinRejectionV1, TransportTerminalReceiptV1,
    bind_independent_fallback_transition_v1,
};

#[cfg(test)]
mod tests;
