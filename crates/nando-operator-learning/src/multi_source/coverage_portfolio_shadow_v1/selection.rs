use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::valid_nonzero_sha256;

use super::model::{
    COVERAGE_PORTFOLIO_SHADOW_SCHEMA_V1, CoverageCandidateCostV1, CoverageCandidateSafetyV1,
    CoverageIntentReceiptV1, CoveragePackageCandidateV1, CoveragePackageShadowV1,
    CoveragePortfolioConservationV1, CoveragePortfolioShadowErrorV1, CoveragePortfolioShadowV1,
    CoverageReceiptKindV1, CoverageSafetyVetoV1, FrozenCoverageDenominatorV1,
    SelectedCoveragePackageV1,
};

#[derive(Clone)]
struct IndexedCandidate {
    package_id: String,
    projected: BTreeMap<String, u64>,
    actual: BTreeMap<String, u64>,
    costs: CoverageCandidateCostV1,
    total_cost: u64,
    vetoes: Vec<CoverageSafetyVetoV1>,
}

pub fn build_coverage_portfolio_shadow_v1(
    denominator: FrozenCoverageDenominatorV1,
    candidates: Vec<CoveragePackageCandidateV1>,
) -> Result<CoveragePortfolioShadowV1, CoveragePortfolioShadowErrorV1> {
    validate_denominator(&denominator)?;
    let indexed = index_candidates(&denominator, candidates)?;
    let observed = collect_unique_tokens(
        indexed
            .iter()
            .flat_map(|candidate| candidate.projected.iter().chain(&candidate.actual)),
    )?;
    let observed_unique_verified_tokens = checked_token_sum(observed.values().copied())?;
    if observed_unique_verified_tokens > denominator.total_verified_tokens {
        return Err(
            CoveragePortfolioShadowErrorV1::ReceiptMassExceedsDenominator {
                observed: observed_unique_verified_tokens,
                denominator: denominator.total_verified_tokens,
            },
        );
    }

    let (selected_packages, selected_intents) = select_packages(&denominator, &indexed)?;
    let selected_ids = selected_packages
        .iter()
        .map(|package| package.package_id.clone())
        .collect::<BTreeSet<_>>();
    let selected_actual = collect_unique_tokens(
        indexed
            .iter()
            .filter(|candidate| selected_ids.contains(&candidate.package_id))
            .flat_map(|candidate| candidate.actual.iter()),
    )?;
    let selected_projected_unique_verified_tokens =
        checked_token_sum(selected_intents.values().copied())?;
    let selected_actual_unique_verified_tokens =
        checked_token_sum(selected_actual.values().copied())?;

    let conservation = build_conservation(
        &denominator,
        &indexed,
        &selected_ids,
        observed_unique_verified_tokens,
        selected_projected_unique_verified_tokens,
        selected_actual_unique_verified_tokens,
    )?;
    let selected_by_id = selected_packages
        .iter()
        .map(|package| {
            (
                package.package_id.as_str(),
                (
                    package.selection_order,
                    package.marginal_unique_verified_tokens,
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let packages = indexed
        .iter()
        .map(|candidate| {
            let projected_verified_tokens =
                checked_token_sum(candidate.projected.values().copied())?;
            let actual_verified_tokens = checked_token_sum(candidate.actual.values().copied())?;
            let selected = selected_by_id.get(candidate.package_id.as_str()).copied();
            Ok(CoveragePackageShadowV1 {
                package_id: candidate.package_id.clone(),
                costs: candidate.costs.clone(),
                total_bounded_cost: candidate.total_cost,
                projected_verified_tokens,
                actual_verified_tokens,
                projection_error_verified_tokens: i128::from(actual_verified_tokens)
                    - i128::from(projected_verified_tokens),
                safety_vetoes: candidate.vetoes.clone(),
                selected_order: selected.map(|(order, _)| order),
                selected_marginal_unique_verified_tokens: selected
                    .map_or(0, |(_, marginal)| marginal),
            })
        })
        .collect::<Result<Vec<_>, CoveragePortfolioShadowErrorV1>>()?;
    let target_reached =
        selected_projected_unique_verified_tokens >= denominator.target_unique_verified_tokens;
    let mut report = CoveragePortfolioShadowV1 {
        schema: COVERAGE_PORTFOLIO_SHADOW_SCHEMA_V1.to_owned(),
        portfolio_root_sha256: String::new(),
        denominator,
        packages,
        selected_packages,
        selected_projected_unique_verified_tokens,
        selected_actual_unique_verified_tokens,
        target_reached,
        conservation,
        authority_ready: false,
    };
    report.portfolio_root_sha256 = report.expected_root()?;
    Ok(report)
}

fn validate_denominator(
    denominator: &FrozenCoverageDenominatorV1,
) -> Result<(), CoveragePortfolioShadowErrorV1> {
    if !valid_nonzero_sha256(&denominator.denominator_root_sha256) {
        return Err(CoveragePortfolioShadowErrorV1::EmptyDenominatorRoot);
    }
    if denominator.total_verified_tokens == 0 {
        return Err(CoveragePortfolioShadowErrorV1::ZeroDenominator);
    }
    if denominator.target_unique_verified_tokens > denominator.total_verified_tokens {
        return Err(CoveragePortfolioShadowErrorV1::TargetExceedsDenominator {
            target: denominator.target_unique_verified_tokens,
            denominator: denominator.total_verified_tokens,
        });
    }
    Ok(())
}

fn index_candidates(
    denominator: &FrozenCoverageDenominatorV1,
    candidates: Vec<CoveragePackageCandidateV1>,
) -> Result<Vec<IndexedCandidate>, CoveragePortfolioShadowErrorV1> {
    let mut package_ids = BTreeSet::new();
    let mut intent_tokens = BTreeMap::<String, u64>::new();
    let mut indexed = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        if candidate.package_id.is_empty() {
            return Err(CoveragePortfolioShadowErrorV1::EmptyPackageId);
        }
        if !package_ids.insert(candidate.package_id.clone()) {
            return Err(CoveragePortfolioShadowErrorV1::DuplicatePackageId {
                package_id: candidate.package_id,
            });
        }
        let Some(total_cost) = candidate.costs.total_bounded_cost() else {
            return Err(CoveragePortfolioShadowErrorV1::InvalidCandidateCost {
                package_id: candidate.package_id,
            });
        };
        let mut projected = BTreeMap::new();
        let mut actual = BTreeMap::new();
        for receipt in candidate.receipts {
            validate_receipt(&candidate.package_id, denominator, &receipt)?;
            let destination = match receipt.kind {
                CoverageReceiptKindV1::Projected => &mut projected,
                CoverageReceiptKindV1::Actual => &mut actual,
            };
            if destination.contains_key(&receipt.intent_id) {
                return Err(CoveragePortfolioShadowErrorV1::DuplicateIntentReceipt {
                    package_id: candidate.package_id,
                    intent_id: receipt.intent_id,
                    kind: receipt.kind,
                });
            }
            if let Some(expected) = intent_tokens.get(&receipt.intent_id)
                && *expected != receipt.verified_tokens
            {
                return Err(CoveragePortfolioShadowErrorV1::IntentTokenMismatch {
                    intent_id: receipt.intent_id,
                    expected: *expected,
                    actual: receipt.verified_tokens,
                });
            }
            intent_tokens
                .entry(receipt.intent_id.clone())
                .or_insert(receipt.verified_tokens);
            destination.insert(receipt.intent_id, receipt.verified_tokens);
        }
        indexed.push(IndexedCandidate {
            package_id: candidate.package_id,
            projected,
            actual,
            costs: candidate.costs,
            total_cost,
            vetoes: safety_vetoes(&candidate.safety),
        });
    }
    indexed.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    Ok(indexed)
}

fn validate_receipt(
    package_id: &str,
    denominator: &FrozenCoverageDenominatorV1,
    receipt: &CoverageIntentReceiptV1,
) -> Result<(), CoveragePortfolioShadowErrorV1> {
    if receipt.package_id != package_id {
        return Err(CoveragePortfolioShadowErrorV1::ReceiptPackageMismatch {
            candidate_package_id: package_id.to_owned(),
            receipt_package_id: receipt.package_id.clone(),
        });
    }
    if receipt.denominator_root_sha256 != denominator.denominator_root_sha256 {
        return Err(CoveragePortfolioShadowErrorV1::DenominatorRootMismatch {
            package_id: package_id.to_owned(),
            intent_id: receipt.intent_id.clone(),
        });
    }
    if receipt.intent_id.is_empty() {
        return Err(CoveragePortfolioShadowErrorV1::EmptyIntentId {
            package_id: package_id.to_owned(),
        });
    }
    if receipt.verified_tokens == 0 {
        return Err(CoveragePortfolioShadowErrorV1::ZeroVerifiedTokens {
            package_id: package_id.to_owned(),
            intent_id: receipt.intent_id.clone(),
        });
    }
    Ok(())
}

fn safety_vetoes(safety: &CoverageCandidateSafetyV1) -> Vec<CoverageSafetyVetoV1> {
    let mut vetoes = Vec::new();
    if safety.wrong_accepts != 0 {
        vetoes.push(CoverageSafetyVetoV1::WrongAccepts);
    }
    if safety.parity_failures != 0 {
        vetoes.push(CoverageSafetyVetoV1::ParityFailures);
    }
    if !safety.lease_valid {
        vetoes.push(CoverageSafetyVetoV1::InvalidLease);
    }
    if !safety.bundle_valid {
        vetoes.push(CoverageSafetyVetoV1::InvalidBundle);
    }
    vetoes
}

fn select_packages(
    denominator: &FrozenCoverageDenominatorV1,
    candidates: &[IndexedCandidate],
) -> Result<(Vec<SelectedCoveragePackageV1>, BTreeMap<String, u64>), CoveragePortfolioShadowErrorV1>
{
    let mut remaining = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| candidate.vetoes.is_empty().then_some(index))
        .collect::<BTreeSet<_>>();
    let mut selected_intents = BTreeMap::<String, u64>::new();
    let mut selected = Vec::new();
    let mut cumulative = 0_u64;

    while cumulative < denominator.target_unique_verified_tokens {
        let mut best = None::<(usize, u64)>;
        for index in &remaining {
            let candidate = &candidates[*index];
            let gain = marginal_gain(&candidate.projected, &selected_intents)?;
            if gain == 0 {
                continue;
            }
            let replace = best.is_none_or(|(best_index, best_gain)| {
                let best_candidate = &candidates[best_index];
                match ratio_cmp(
                    gain,
                    candidate.total_cost,
                    best_gain,
                    best_candidate.total_cost,
                ) {
                    Ordering::Greater => true,
                    Ordering::Equal => candidate.package_id < best_candidate.package_id,
                    Ordering::Less => false,
                }
            });
            if replace {
                best = Some((*index, gain));
            }
        }
        let Some((index, gain)) = best else {
            break;
        };
        remaining.remove(&index);
        let candidate = &candidates[index];
        for (intent_id, tokens) in &candidate.projected {
            selected_intents.entry(intent_id.clone()).or_insert(*tokens);
        }
        cumulative = cumulative
            .checked_add(gain)
            .ok_or(CoveragePortfolioShadowErrorV1::TokenTotalOverflow)?;
        selected.push(SelectedCoveragePackageV1 {
            package_id: candidate.package_id.clone(),
            selection_order: u64::try_from(selected.len())
                .map_err(|_| CoveragePortfolioShadowErrorV1::TokenTotalOverflow)?,
            marginal_unique_verified_tokens: gain,
            cumulative_unique_verified_tokens: cumulative,
            total_bounded_cost: candidate.total_cost,
        });
    }
    Ok((selected, selected_intents))
}

fn marginal_gain(
    projected: &BTreeMap<String, u64>,
    selected: &BTreeMap<String, u64>,
) -> Result<u64, CoveragePortfolioShadowErrorV1> {
    checked_token_sum(
        projected.iter().filter_map(|(intent_id, tokens)| {
            (!selected.contains_key(intent_id)).then_some(*tokens)
        }),
    )
}

fn ratio_cmp(left_gain: u64, left_cost: u64, right_gain: u64, right_cost: u64) -> Ordering {
    (u128::from(left_gain) * u128::from(right_cost))
        .cmp(&(u128::from(right_gain) * u128::from(left_cost)))
}

fn collect_unique_tokens<'a>(
    receipts: impl IntoIterator<Item = (&'a String, &'a u64)>,
) -> Result<BTreeMap<String, u64>, CoveragePortfolioShadowErrorV1> {
    let mut unique = BTreeMap::new();
    for (intent_id, tokens) in receipts {
        if let Some(expected) = unique.get(intent_id)
            && expected != tokens
        {
            return Err(CoveragePortfolioShadowErrorV1::IntentTokenMismatch {
                intent_id: intent_id.clone(),
                expected: *expected,
                actual: *tokens,
            });
        }
        unique.entry(intent_id.clone()).or_insert(*tokens);
    }
    Ok(unique)
}

fn build_conservation(
    denominator: &FrozenCoverageDenominatorV1,
    candidates: &[IndexedCandidate],
    selected_ids: &BTreeSet<String>,
    observed_unique_verified_tokens: u64,
    selected_projected_unique_verified_tokens: u64,
    selected_actual_unique_verified_tokens: u64,
) -> Result<CoveragePortfolioConservationV1, CoveragePortfolioShadowErrorV1> {
    let candidate_projected_gross_verified_tokens = checked_token_sum(
        candidates
            .iter()
            .flat_map(|candidate| candidate.projected.values().copied()),
    )?;
    let candidate_projected_unique = collect_unique_tokens(
        candidates
            .iter()
            .flat_map(|candidate| candidate.projected.iter()),
    )?;
    let candidate_projected_unique_verified_tokens =
        checked_token_sum(candidate_projected_unique.values().copied())?;
    let selected_projected_gross_verified_tokens = checked_token_sum(
        candidates
            .iter()
            .filter(|candidate| selected_ids.contains(&candidate.package_id))
            .flat_map(|candidate| candidate.projected.values().copied()),
    )?;
    let selected_actual_gross_verified_tokens = checked_token_sum(
        candidates
            .iter()
            .filter(|candidate| selected_ids.contains(&candidate.package_id))
            .flat_map(|candidate| candidate.actual.values().copied()),
    )?;
    let denominator_unrepresented_verified_tokens = denominator
        .total_verified_tokens
        .checked_sub(observed_unique_verified_tokens)
        .ok_or(
            CoveragePortfolioShadowErrorV1::ReceiptMassExceedsDenominator {
                observed: observed_unique_verified_tokens,
                denominator: denominator.total_verified_tokens,
            },
        )?;
    let candidate_projected_overlap_deduped_verified_tokens =
        candidate_projected_gross_verified_tokens
            .checked_sub(candidate_projected_unique_verified_tokens)
            .ok_or(CoveragePortfolioShadowErrorV1::TokenTotalOverflow)?;
    let selected_projected_overlap_deduped_verified_tokens =
        selected_projected_gross_verified_tokens
            .checked_sub(selected_projected_unique_verified_tokens)
            .ok_or(CoveragePortfolioShadowErrorV1::TokenTotalOverflow)?;
    let selected_actual_overlap_deduped_verified_tokens = selected_actual_gross_verified_tokens
        .checked_sub(selected_actual_unique_verified_tokens)
        .ok_or(CoveragePortfolioShadowErrorV1::TokenTotalOverflow)?;

    Ok(CoveragePortfolioConservationV1 {
        denominator_verified_tokens: denominator.total_verified_tokens,
        observed_unique_verified_tokens,
        denominator_unrepresented_verified_tokens,
        candidate_projected_gross_verified_tokens,
        candidate_projected_unique_verified_tokens,
        candidate_projected_overlap_deduped_verified_tokens,
        selected_projected_gross_verified_tokens,
        selected_projected_unique_verified_tokens,
        selected_projected_overlap_deduped_verified_tokens,
        selected_actual_gross_verified_tokens,
        selected_actual_unique_verified_tokens,
        selected_actual_overlap_deduped_verified_tokens,
        denominator_conservation_holds: observed_unique_verified_tokens
            .checked_add(denominator_unrepresented_verified_tokens)
            == Some(denominator.total_verified_tokens),
        candidate_projection_conservation_holds: candidate_projected_unique_verified_tokens
            .checked_add(candidate_projected_overlap_deduped_verified_tokens)
            == Some(candidate_projected_gross_verified_tokens),
        selected_projection_conservation_holds: selected_projected_unique_verified_tokens
            .checked_add(selected_projected_overlap_deduped_verified_tokens)
            == Some(selected_projected_gross_verified_tokens),
        selected_actual_conservation_holds: selected_actual_unique_verified_tokens
            .checked_add(selected_actual_overlap_deduped_verified_tokens)
            == Some(selected_actual_gross_verified_tokens),
    })
}

fn checked_token_sum(
    tokens: impl IntoIterator<Item = u64>,
) -> Result<u64, CoveragePortfolioShadowErrorV1> {
    tokens.into_iter().try_fold(0_u64, |sum, tokens| {
        sum.checked_add(tokens)
            .ok_or(CoveragePortfolioShadowErrorV1::TokenTotalOverflow)
    })
}
