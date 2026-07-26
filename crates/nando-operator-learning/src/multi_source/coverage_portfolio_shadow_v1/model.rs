use std::error::Error;
use std::fmt;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

pub const COVERAGE_PORTFOLIO_SHADOW_SCHEMA_V1: &str = "nando.coverage-portfolio-shadow.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FrozenCoverageDenominatorV1 {
    pub denominator_root_sha256: String,
    pub total_verified_tokens: u64,
    pub target_unique_verified_tokens: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageReceiptKindV1 {
    Projected,
    Actual,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoverageIntentReceiptV1 {
    pub denominator_root_sha256: String,
    pub intent_id: String,
    pub package_id: String,
    pub verified_tokens: u64,
    pub kind: CoverageReceiptKindV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoverageCandidateCostV1 {
    pub learner_cost: u64,
    pub verifier_cost: u64,
    pub hot_bytes: u64,
}

impl CoverageCandidateCostV1 {
    #[must_use]
    pub fn total_bounded_cost(&self) -> Option<u64> {
        let total = self
            .learner_cost
            .checked_add(self.verifier_cost)?
            .checked_add(self.hot_bytes)?;
        (total > 0).then_some(total)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoverageCandidateSafetyV1 {
    pub wrong_accepts: u64,
    pub parity_failures: u64,
    pub lease_valid: bool,
    pub bundle_valid: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageSafetyVetoV1 {
    WrongAccepts,
    ParityFailures,
    InvalidLease,
    InvalidBundle,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoveragePackageCandidateV1 {
    pub package_id: String,
    pub receipts: Vec<CoverageIntentReceiptV1>,
    pub costs: CoverageCandidateCostV1,
    pub safety: CoverageCandidateSafetyV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SelectedCoveragePackageV1 {
    pub package_id: String,
    pub selection_order: u64,
    pub marginal_unique_verified_tokens: u64,
    pub cumulative_unique_verified_tokens: u64,
    pub total_bounded_cost: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoveragePackageShadowV1 {
    pub package_id: String,
    pub costs: CoverageCandidateCostV1,
    pub total_bounded_cost: u64,
    pub projected_verified_tokens: u64,
    pub actual_verified_tokens: u64,
    // Signed actual minus projected mass.
    pub projection_error_verified_tokens: i128,
    pub safety_vetoes: Vec<CoverageSafetyVetoV1>,
    pub selected_order: Option<u64>,
    pub selected_marginal_unique_verified_tokens: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoveragePortfolioConservationV1 {
    pub denominator_verified_tokens: u64,
    pub observed_unique_verified_tokens: u64,
    pub denominator_unrepresented_verified_tokens: u64,
    pub candidate_projected_gross_verified_tokens: u64,
    pub candidate_projected_unique_verified_tokens: u64,
    pub candidate_projected_overlap_deduped_verified_tokens: u64,
    pub selected_projected_gross_verified_tokens: u64,
    pub selected_projected_unique_verified_tokens: u64,
    pub selected_projected_overlap_deduped_verified_tokens: u64,
    pub selected_actual_gross_verified_tokens: u64,
    pub selected_actual_unique_verified_tokens: u64,
    pub selected_actual_overlap_deduped_verified_tokens: u64,
    pub denominator_conservation_holds: bool,
    pub candidate_projection_conservation_holds: bool,
    pub selected_projection_conservation_holds: bool,
    pub selected_actual_conservation_holds: bool,
}

impl CoveragePortfolioConservationV1 {
    #[must_use]
    pub fn all_hold(&self) -> bool {
        self.denominator_conservation_holds
            && self.candidate_projection_conservation_holds
            && self.selected_projection_conservation_holds
            && self.selected_actual_conservation_holds
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoveragePortfolioShadowV1 {
    pub schema: String,
    pub portfolio_root_sha256: String,
    pub denominator: FrozenCoverageDenominatorV1,
    pub packages: Vec<CoveragePackageShadowV1>,
    pub selected_packages: Vec<SelectedCoveragePackageV1>,
    pub selected_projected_unique_verified_tokens: u64,
    pub selected_actual_unique_verified_tokens: u64,
    pub target_reached: bool,
    pub conservation: CoveragePortfolioConservationV1,
    pub authority_ready: bool,
}

impl CoveragePortfolioShadowV1 {
    pub fn expected_root(&self) -> Result<String, CoveragePortfolioShadowErrorV1> {
        canonical_json_sha256(&(
            COVERAGE_PORTFOLIO_SHADOW_SCHEMA_V1,
            &self.denominator,
            &self.packages,
            &self.selected_packages,
            self.selected_projected_unique_verified_tokens,
            self.selected_actual_unique_verified_tokens,
            self.target_reached,
            &self.conservation,
            false,
        ))
        .map_err(|_| CoveragePortfolioShadowErrorV1::Serialization)
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == COVERAGE_PORTFOLIO_SHADOW_SCHEMA_V1
            && valid_nonzero_sha256(&self.denominator.denominator_root_sha256)
            && !self.authority_ready
            && self.conservation.all_hold()
            && self.selected_projected_unique_verified_tokens
                == self.conservation.selected_projected_unique_verified_tokens
            && self.selected_actual_unique_verified_tokens
                == self.conservation.selected_actual_unique_verified_tokens
            && self.target_reached
                == (self.selected_projected_unique_verified_tokens
                    >= self.denominator.target_unique_verified_tokens)
            && selection_is_consistent(self)
            && self
                .expected_root()
                .is_ok_and(|expected| self.portfolio_root_sha256 == expected)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoveragePortfolioShadowErrorV1 {
    EmptyDenominatorRoot,
    ZeroDenominator,
    TargetExceedsDenominator {
        target: u64,
        denominator: u64,
    },
    EmptyPackageId,
    DuplicatePackageId {
        package_id: String,
    },
    InvalidCandidateCost {
        package_id: String,
    },
    EmptyIntentId {
        package_id: String,
    },
    ReceiptPackageMismatch {
        candidate_package_id: String,
        receipt_package_id: String,
    },
    DenominatorRootMismatch {
        package_id: String,
        intent_id: String,
    },
    ZeroVerifiedTokens {
        package_id: String,
        intent_id: String,
    },
    DuplicateIntentReceipt {
        package_id: String,
        intent_id: String,
        kind: CoverageReceiptKindV1,
    },
    IntentTokenMismatch {
        intent_id: String,
        expected: u64,
        actual: u64,
    },
    TokenTotalOverflow,
    Serialization,
    ReceiptMassExceedsDenominator {
        observed: u64,
        denominator: u64,
    },
}

impl fmt::Display for CoveragePortfolioShadowErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for CoveragePortfolioShadowErrorV1 {}

fn selection_is_consistent(report: &CoveragePortfolioShadowV1) -> bool {
    let mut cumulative = 0_u64;
    for (index, selected) in report.selected_packages.iter().enumerate() {
        if selected.selection_order != u64::try_from(index).unwrap_or(u64::MAX)
            || selected.marginal_unique_verified_tokens == 0
        {
            return false;
        }
        let Some(next) = cumulative.checked_add(selected.marginal_unique_verified_tokens) else {
            return false;
        };
        cumulative = next;
        if selected.cumulative_unique_verified_tokens != cumulative {
            return false;
        }
        let Some(package) = report
            .packages
            .iter()
            .find(|package| package.package_id == selected.package_id)
        else {
            return false;
        };
        if package.selected_order != Some(selected.selection_order)
            || package.selected_marginal_unique_verified_tokens
                != selected.marginal_unique_verified_tokens
            || !package.safety_vetoes.is_empty()
        {
            return false;
        }
    }
    cumulative == report.selected_projected_unique_verified_tokens
        && report
            .packages
            .windows(2)
            .all(|window| window[0].package_id < window[1].package_id)
}
