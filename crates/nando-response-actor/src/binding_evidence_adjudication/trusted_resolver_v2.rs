use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::canonical::{is_sha256, pretty_json_bytes, sha256_json};
use super::physical_trial_v2::{
    PhysicalTrialOutcomeV2, PhysicalTrialReceiptV2, validate_physical_trial_receipt_v2,
};
use super::wire::BindingAdjudicationErrorV1;

pub const TRUSTED_RESOLVED_BINDING_ROWS_SCHEMA_V2: &str = "nando.trusted-resolved-binding-rows.v2";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingEvidencePartitionV2 {
    Support,
    Future,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingTrialEvidenceLabelV2 {
    Positive,
    ApplicabilityNegative,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustedBindingResolverReceiptSourceV2 {
    ControlledFixture,
    ExternalIndependent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenBindingTrialRowV2 {
    pub frozen_row_root_sha256: String,
    pub frozen_graph_root_sha256: String,
    pub capture_root_sha256: String,
    pub partition: BindingEvidencePartitionV2,
    pub evidence_label: BindingTrialEvidenceLabelV2,
    pub relation_identity_sha256: String,
    pub protocol_facet_root_sha256: String,
    pub effect_invariant_root_sha256: String,
    pub physical_program_id_sha256: String,
    pub surface_root_sha256: String,
    pub receipt_source: TrustedBindingResolverReceiptSourceV2,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedBindingResolverInputV2 {
    pub frozen_rows: Vec<FrozenBindingTrialRowV2>,
    pub physical_trials: Vec<PhysicalTrialReceiptV2>,
    pub resolver_program_digest_sha256: String,
    pub external_manifest_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedResolvedBindingRowV2 {
    pub frozen_row_root_sha256: String,
    pub frozen_graph_root_sha256: String,
    pub capture_root_sha256: String,
    pub partition: BindingEvidencePartitionV2,
    pub evidence_label: BindingTrialEvidenceLabelV2,
    pub relation_identity_sha256: String,
    pub protocol_facet_root_sha256: String,
    pub effect_invariant_root_sha256: String,
    pub physical_program_id_sha256: String,
    pub surface_root_sha256: String,
    pub receipt_source: TrustedBindingResolverReceiptSourceV2,
    pub actor_program_digest_sha256: String,
    pub verifier_program_digest_sha256: String,
    pub candidate_action_digest_sha256: String,
    pub observed_delta_root_sha256: String,
    pub trial_outcome: PhysicalTrialOutcomeV2,
    pub physical_trial_receipt_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct TrustedResolvedBindingRowsWireV2 {
    schema: String,
    resolved_root_sha256: String,
    external_manifest_root_sha256: String,
    resolver_program_digest_sha256: String,
    rows: Vec<TrustedResolvedBindingRowV2>,
    support_rows: usize,
    future_rows: usize,
    censored_rows: usize,
    real_independent_receipts: usize,
    execution_authority: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedResolvedBindingRowsV2 {
    wire: TrustedResolvedBindingRowsWireV2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustedResolverV2Error {
    InvalidDigest,
    InvalidTrustRoot,
    MissingTrial,
    DuplicateRow,
    InvalidTrial,
    InvalidResolvedRows,
    Serialization,
}

impl From<BindingAdjudicationErrorV1> for TrustedResolverV2Error {
    fn from(value: BindingAdjudicationErrorV1) -> Self {
        match value {
            BindingAdjudicationErrorV1::Serialization => Self::Serialization,
            BindingAdjudicationErrorV1::InvalidDigest => Self::InvalidDigest,
            _ => Self::InvalidResolvedRows,
        }
    }
}

impl TrustedResolvedBindingRowsV2 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, TrustedResolverV2Error> {
        pretty_json_bytes(&self.wire).map_err(TrustedResolverV2Error::from)
    }

    pub fn from_canonical_bytes(
        bytes: &[u8],
        external_manifest_root_sha256: &str,
    ) -> Result<Self, TrustedResolverV2Error> {
        let wire: TrustedResolvedBindingRowsWireV2 = serde_json::from_slice(bytes)
            .map_err(|_| TrustedResolverV2Error::InvalidResolvedRows)?;
        let resolved = Self { wire };
        if resolved.canonical_bytes()? != bytes
            || resolved.external_manifest_root_sha256() != external_manifest_root_sha256
        {
            return Err(TrustedResolverV2Error::InvalidTrustRoot);
        }
        validate_resolved_rows_v2(&resolved)?;
        Ok(resolved)
    }

    pub fn resolved_root_sha256(&self) -> &str {
        &self.wire.resolved_root_sha256
    }

    pub fn external_manifest_root_sha256(&self) -> &str {
        &self.wire.external_manifest_root_sha256
    }

    pub fn resolver_program_digest_sha256(&self) -> &str {
        &self.wire.resolver_program_digest_sha256
    }

    pub fn rows(&self) -> &[TrustedResolvedBindingRowV2] {
        &self.wire.rows
    }

    pub fn support_rows(&self) -> usize {
        self.wire.support_rows
    }

    pub fn future_rows(&self) -> usize {
        self.wire.future_rows
    }

    pub fn censored_rows(&self) -> usize {
        self.wire.censored_rows
    }

    pub fn real_independent_receipts(&self) -> usize {
        self.wire.real_independent_receipts
    }

    pub fn execution_authority(&self) -> bool {
        self.wire.execution_authority
    }
}

pub fn resolve_trusted_binding_rows_v2(
    input: TrustedBindingResolverInputV2,
) -> Result<TrustedResolvedBindingRowsV2, TrustedResolverV2Error> {
    validate_resolver_input_roots_v2(&input)?;
    let expected_root = trusted_binding_resolver_manifest_root_v2(
        &input.frozen_rows,
        &input.physical_trials,
        &input.resolver_program_digest_sha256,
    )?;
    if expected_root != input.external_manifest_root_sha256 {
        return Err(TrustedResolverV2Error::InvalidTrustRoot);
    }
    let mut trial_by_row = BTreeMap::new();
    for trial in &input.physical_trials {
        validate_physical_trial_receipt_v2(trial)
            .map_err(|_| TrustedResolverV2Error::InvalidTrial)?;
        if trial_by_row
            .insert(trial.joined_roots.frozen_row_root_sha256.clone(), trial)
            .is_some()
        {
            return Err(TrustedResolverV2Error::DuplicateRow);
        }
    }

    let mut seen_rows = BTreeSet::new();
    let mut rows = Vec::with_capacity(input.frozen_rows.len());
    for frozen in input.frozen_rows {
        validate_frozen_row_v2(&frozen)?;
        if !seen_rows.insert(frozen.frozen_row_root_sha256.clone()) {
            return Err(TrustedResolverV2Error::DuplicateRow);
        }
        let trial = trial_by_row
            .get(&frozen.frozen_row_root_sha256)
            .ok_or(TrustedResolverV2Error::MissingTrial)?;
        if trial.joined_roots.frozen_graph_root_sha256 != frozen.frozen_graph_root_sha256
            || trial.joined_roots.capture_root_sha256 != frozen.capture_root_sha256
        {
            return Err(TrustedResolverV2Error::InvalidTrial);
        }
        rows.push(TrustedResolvedBindingRowV2 {
            frozen_row_root_sha256: frozen.frozen_row_root_sha256,
            frozen_graph_root_sha256: frozen.frozen_graph_root_sha256,
            capture_root_sha256: frozen.capture_root_sha256,
            partition: frozen.partition,
            evidence_label: frozen.evidence_label,
            relation_identity_sha256: frozen.relation_identity_sha256,
            protocol_facet_root_sha256: frozen.protocol_facet_root_sha256,
            effect_invariant_root_sha256: frozen.effect_invariant_root_sha256,
            physical_program_id_sha256: frozen.physical_program_id_sha256,
            surface_root_sha256: frozen.surface_root_sha256,
            receipt_source: frozen.receipt_source,
            actor_program_digest_sha256: trial.joined_roots.actor_program_digest_sha256.clone(),
            verifier_program_digest_sha256: trial
                .joined_roots
                .verifier_program_digest_sha256
                .clone(),
            candidate_action_digest_sha256: trial
                .joined_roots
                .candidate_action_digest_sha256
                .clone(),
            observed_delta_root_sha256: trial.joined_roots.observed_delta_root_sha256.clone(),
            trial_outcome: trial.outcome,
            physical_trial_receipt_sha256: trial.receipt_sha256.clone(),
        });
    }
    rows.sort_by(|left, right| {
        left.frozen_row_root_sha256
            .cmp(&right.frozen_row_root_sha256)
    });
    let support_rows = rows
        .iter()
        .filter(|row| row.partition == BindingEvidencePartitionV2::Support)
        .count();
    let future_rows = rows
        .iter()
        .filter(|row| row.partition == BindingEvidencePartitionV2::Future)
        .count();
    let censored_rows = rows
        .iter()
        .filter(|row| row.trial_outcome == PhysicalTrialOutcomeV2::Censored)
        .count();
    let real_independent_receipts = rows
        .iter()
        .filter(|row| {
            row.receipt_source == TrustedBindingResolverReceiptSourceV2::ExternalIndependent
        })
        .count();
    let mut wire = TrustedResolvedBindingRowsWireV2 {
        schema: TRUSTED_RESOLVED_BINDING_ROWS_SCHEMA_V2.to_owned(),
        resolved_root_sha256: String::new(),
        external_manifest_root_sha256: input.external_manifest_root_sha256,
        resolver_program_digest_sha256: input.resolver_program_digest_sha256,
        rows,
        support_rows,
        future_rows,
        censored_rows,
        real_independent_receipts,
        execution_authority: false,
    };
    wire.resolved_root_sha256 = trusted_resolved_rows_digest_v2(&wire)?;
    let resolved = TrustedResolvedBindingRowsV2 { wire };
    validate_resolved_rows_v2(&resolved)?;
    Ok(resolved)
}

pub fn trusted_binding_resolver_manifest_root_v2(
    frozen_rows: &[FrozenBindingTrialRowV2],
    physical_trials: &[PhysicalTrialReceiptV2],
    resolver_program_digest_sha256: &str,
) -> Result<String, TrustedResolverV2Error> {
    if !is_sha256(resolver_program_digest_sha256) {
        return Err(TrustedResolverV2Error::InvalidDigest);
    }
    let mut frozen_rows = frozen_rows.to_vec();
    frozen_rows.sort_by(|left, right| {
        left.frozen_row_root_sha256
            .cmp(&right.frozen_row_root_sha256)
    });
    let mut trial_roots = physical_trials
        .iter()
        .map(|trial| trial.receipt_sha256.as_str())
        .collect::<Vec<_>>();
    trial_roots.sort();
    sha256_json(&(
        TRUSTED_RESOLVED_BINDING_ROWS_SCHEMA_V2,
        &frozen_rows,
        trial_roots,
        resolver_program_digest_sha256,
    ))
    .map_err(TrustedResolverV2Error::from)
}

pub(crate) fn validate_resolved_rows_v2(
    resolved: &TrustedResolvedBindingRowsV2,
) -> Result<(), TrustedResolverV2Error> {
    let wire = &resolved.wire;
    if wire.schema != TRUSTED_RESOLVED_BINDING_ROWS_SCHEMA_V2
        || wire.execution_authority
        || wire.rows.is_empty()
        || !is_sha256(&wire.external_manifest_root_sha256)
        || !is_sha256(&wire.resolver_program_digest_sha256)
        || wire.resolved_root_sha256 != trusted_resolved_rows_digest_v2(wire)?
        || wire
            .rows
            .windows(2)
            .any(|pair| pair[0].frozen_row_root_sha256 >= pair[1].frozen_row_root_sha256)
    {
        return Err(TrustedResolverV2Error::InvalidResolvedRows);
    }
    let support_rows = wire
        .rows
        .iter()
        .filter(|row| row.partition == BindingEvidencePartitionV2::Support)
        .count();
    let future_rows = wire
        .rows
        .iter()
        .filter(|row| row.partition == BindingEvidencePartitionV2::Future)
        .count();
    let censored_rows = wire
        .rows
        .iter()
        .filter(|row| row.trial_outcome == PhysicalTrialOutcomeV2::Censored)
        .count();
    let real_independent_receipts = wire
        .rows
        .iter()
        .filter(|row| {
            row.receipt_source == TrustedBindingResolverReceiptSourceV2::ExternalIndependent
        })
        .count();
    if wire.support_rows != support_rows
        || wire.future_rows != future_rows
        || wire.censored_rows != censored_rows
        || wire.real_independent_receipts != real_independent_receipts
        || wire
            .rows
            .iter()
            .try_for_each(validate_resolved_row_v2)
            .is_err()
    {
        return Err(TrustedResolverV2Error::InvalidResolvedRows);
    }
    Ok(())
}

fn trusted_resolved_rows_digest_v2(
    wire: &TrustedResolvedBindingRowsWireV2,
) -> Result<String, TrustedResolverV2Error> {
    sha256_json(&(
        wire.schema.as_str(),
        wire.external_manifest_root_sha256.as_str(),
        wire.resolver_program_digest_sha256.as_str(),
        &wire.rows,
        wire.support_rows,
        wire.future_rows,
        wire.censored_rows,
        wire.real_independent_receipts,
        wire.execution_authority,
    ))
    .map_err(TrustedResolverV2Error::from)
}

fn validate_resolver_input_roots_v2(
    input: &TrustedBindingResolverInputV2,
) -> Result<(), TrustedResolverV2Error> {
    if input.frozen_rows.is_empty()
        || input.physical_trials.len() != input.frozen_rows.len()
        || !is_sha256(&input.resolver_program_digest_sha256)
        || !is_sha256(&input.external_manifest_root_sha256)
    {
        return Err(TrustedResolverV2Error::InvalidDigest);
    }
    Ok(())
}

fn validate_frozen_row_v2(row: &FrozenBindingTrialRowV2) -> Result<(), TrustedResolverV2Error> {
    let roots = [
        row.frozen_row_root_sha256.as_str(),
        row.frozen_graph_root_sha256.as_str(),
        row.capture_root_sha256.as_str(),
        row.relation_identity_sha256.as_str(),
        row.protocol_facet_root_sha256.as_str(),
        row.effect_invariant_root_sha256.as_str(),
        row.physical_program_id_sha256.as_str(),
        row.surface_root_sha256.as_str(),
    ];
    if roots.into_iter().all(is_sha256) {
        Ok(())
    } else {
        Err(TrustedResolverV2Error::InvalidDigest)
    }
}

fn validate_resolved_row_v2(
    row: &TrustedResolvedBindingRowV2,
) -> Result<(), TrustedResolverV2Error> {
    let roots = [
        row.frozen_row_root_sha256.as_str(),
        row.frozen_graph_root_sha256.as_str(),
        row.capture_root_sha256.as_str(),
        row.relation_identity_sha256.as_str(),
        row.protocol_facet_root_sha256.as_str(),
        row.effect_invariant_root_sha256.as_str(),
        row.physical_program_id_sha256.as_str(),
        row.surface_root_sha256.as_str(),
        row.actor_program_digest_sha256.as_str(),
        row.verifier_program_digest_sha256.as_str(),
        row.candidate_action_digest_sha256.as_str(),
        row.observed_delta_root_sha256.as_str(),
        row.physical_trial_receipt_sha256.as_str(),
    ];
    if roots.into_iter().all(is_sha256) {
        Ok(())
    } else {
        Err(TrustedResolverV2Error::InvalidDigest)
    }
}
