use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::canonical_json_sha256;
use serde::{Deserialize, Serialize};

use super::{AuditMassV1, FactorizedMultiSourceRowV1, MultiSourceReasonV1, PreActionShapeClassV1};

pub const COVERAGE_OPPORTUNITY_SNAPSHOT_SCHEMA_V1: &str = "nando.coverage-opportunity-snapshot.v1";
pub const COVERAGE_OPPORTUNITY_MAX_ROWS_V1: usize = 256;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MarginalShapeOpportunityV1 {
    pub applicability_shape_root_sha256: String,
    pub reason: MultiSourceReasonV1,
    pub pre_action_shape: PreActionShapeClassV1,
    pub total: AuditMassV1,
    pub already_active: AuditMassV1,
    pub marginal: AuditMassV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoverageOpportunitySnapshotV1 {
    pub schema: String,
    pub snapshot_root_sha256: String,
    pub evidence_epoch_sha256: String,
    pub total: AuditMassV1,
    pub already_active: AuditMassV1,
    pub unresolved: AuditMassV1,
    pub token_conservation_holds: bool,
    pub duplicate_marginal_purchase: u64,
    pub rows: Vec<MarginalShapeOpportunityV1>,
    pub authority_ready: bool,
}

#[derive(Clone)]
struct IntentPortfolioRow {
    tokens: u64,
    shape_root: String,
    reason: MultiSourceReasonV1,
    shape: PreActionShapeClassV1,
}

#[must_use]
pub fn build_coverage_opportunity_snapshot_v1(
    rows: &[FactorizedMultiSourceRowV1],
    active_intents: &BTreeSet<String>,
    evidence_epoch_sha256: String,
) -> CoverageOpportunitySnapshotV1 {
    let mut by_intent = BTreeMap::<String, Vec<&FactorizedMultiSourceRowV1>>::new();
    for row in rows {
        by_intent
            .entry(row.turn_intent_id_sha256.clone())
            .or_default()
            .push(row);
    }
    let intents = by_intent
        .into_iter()
        .map(|(intent, rows)| (intent, collapse_intent(rows)))
        .collect::<BTreeMap<_, _>>();
    let mut shape_rows = BTreeMap::<
        (String, MultiSourceReasonV1, PreActionShapeClassV1),
        MarginalShapeOpportunityV1,
    >::new();
    let mut total = AuditMassV1::default();
    let mut already_active = AuditMassV1::default();
    let mut unresolved = AuditMassV1::default();
    for (intent, row) in intents {
        total.intents = total.intents.saturating_add(1);
        total.input_tokens = total.input_tokens.saturating_add(row.tokens);
        let active = active_intents.contains(&intent);
        let entry = shape_rows
            .entry((row.shape_root.clone(), row.reason, row.shape))
            .or_insert_with(|| MarginalShapeOpportunityV1 {
                applicability_shape_root_sha256: row.shape_root,
                reason: row.reason,
                pre_action_shape: row.shape,
                total: AuditMassV1::default(),
                already_active: AuditMassV1::default(),
                marginal: AuditMassV1::default(),
            });
        add_mass(&mut entry.total, row.tokens);
        if active {
            add_mass(&mut entry.already_active, row.tokens);
            add_mass(&mut already_active, row.tokens);
        } else {
            add_mass(&mut entry.marginal, row.tokens);
            add_mass(&mut unresolved, row.tokens);
        }
    }
    let mut rows = shape_rows.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .marginal
            .input_tokens
            .cmp(&left.marginal.input_tokens)
            .then_with(|| {
                left.applicability_shape_root_sha256
                    .cmp(&right.applicability_shape_root_sha256)
            })
    });
    rows.truncate(COVERAGE_OPPORTUNITY_MAX_ROWS_V1);
    let token_conservation_holds = total.intents
        == already_active.intents.saturating_add(unresolved.intents)
        && total.input_tokens
            == already_active
                .input_tokens
                .saturating_add(unresolved.input_tokens);
    let mut snapshot = CoverageOpportunitySnapshotV1 {
        schema: COVERAGE_OPPORTUNITY_SNAPSHOT_SCHEMA_V1.to_owned(),
        snapshot_root_sha256: String::new(),
        evidence_epoch_sha256,
        total,
        already_active,
        unresolved,
        token_conservation_holds,
        duplicate_marginal_purchase: 0,
        rows,
        authority_ready: false,
    };
    snapshot.snapshot_root_sha256 = snapshot.expected_root();
    snapshot
}

impl CoverageOpportunitySnapshotV1 {
    #[must_use]
    pub fn expected_root(&self) -> String {
        canonical_json_sha256(&(
            COVERAGE_OPPORTUNITY_SNAPSHOT_SCHEMA_V1,
            self.evidence_epoch_sha256.as_str(),
            &self.total,
            &self.already_active,
            &self.unresolved,
            self.token_conservation_holds,
            self.duplicate_marginal_purchase,
            &self.rows,
            false,
        ))
        .expect("coverage snapshot serializes")
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == COVERAGE_OPPORTUNITY_SNAPSHOT_SCHEMA_V1
            && !self.authority_ready
            && self.token_conservation_holds
            && self.duplicate_marginal_purchase == 0
            && self.rows.len() <= COVERAGE_OPPORTUNITY_MAX_ROWS_V1
            && self.snapshot_root_sha256 == self.expected_root()
    }
}

fn collapse_intent(rows: Vec<&FactorizedMultiSourceRowV1>) -> IntentPortfolioRow {
    let tokens = rows.iter().map(|row| row.input_tokens).max().unwrap_or(0);
    let shapes = rows
        .iter()
        .map(|row| {
            (
                row.applicability_shape_root_sha256.as_str(),
                row.reason,
                row.pre_action_shape,
            )
        })
        .collect::<BTreeSet<_>>();
    if let Some((shape_root, reason, shape)) = shapes.iter().next().copied()
        && shapes.len() == 1
    {
        return IntentPortfolioRow {
            tokens,
            shape_root: shape_root.to_owned(),
            reason,
            shape,
        };
    }
    IntentPortfolioRow {
        tokens,
        shape_root: canonical_json_sha256(&("nando.multi-source-mixed-intent-shape.v1", &shapes))
            .expect("mixed shape serializes"),
        reason: MultiSourceReasonV1::MultipleOutputParts,
        shape: PreActionShapeClassV1::Unresolved,
    }
}

fn add_mass(mass: &mut AuditMassV1, tokens: u64) {
    mass.intents = mass.intents.saturating_add(1);
    mass.input_tokens = mass.input_tokens.saturating_add(tokens);
}
