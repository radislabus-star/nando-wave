//! Frozen-denominator MS8 planner. It reports a portfolio but never admits it.

mod model;
mod selection;

pub use model::{
    COVERAGE_PORTFOLIO_SHADOW_SCHEMA_V1, CoverageCandidateCostV1, CoverageCandidateSafetyV1,
    CoverageIntentReceiptV1, CoveragePackageCandidateV1, CoveragePackageShadowV1,
    CoveragePortfolioConservationV1, CoveragePortfolioShadowErrorV1, CoveragePortfolioShadowV1,
    CoverageReceiptKindV1, CoverageSafetyVetoV1, FrozenCoverageDenominatorV1,
    SelectedCoveragePackageV1,
};
pub use selection::build_coverage_portfolio_shadow_v1;

#[cfg(test)]
mod tests;
