use super::*;

const ROOT: &str = "abababababababababababababababababababababababababababababababab";

fn denominator(total: u64, target: u64) -> FrozenCoverageDenominatorV1 {
    FrozenCoverageDenominatorV1 {
        denominator_root_sha256: ROOT.to_owned(),
        total_verified_tokens: total,
        target_unique_verified_tokens: target,
    }
}

fn receipt(
    package_id: &str,
    intent_id: &str,
    verified_tokens: u64,
    kind: CoverageReceiptKindV1,
) -> CoverageIntentReceiptV1 {
    CoverageIntentReceiptV1 {
        denominator_root_sha256: ROOT.to_owned(),
        intent_id: intent_id.to_owned(),
        package_id: package_id.to_owned(),
        verified_tokens,
        kind,
    }
}

fn candidate(
    package_id: &str,
    cost: u64,
    receipts: Vec<CoverageIntentReceiptV1>,
) -> CoveragePackageCandidateV1 {
    CoveragePackageCandidateV1 {
        package_id: package_id.to_owned(),
        receipts,
        costs: CoverageCandidateCostV1 {
            learner_cost: cost,
            verifier_cost: 0,
            hot_bytes: 0,
        },
        safety: CoverageCandidateSafetyV1 {
            wrong_accepts: 0,
            parity_failures: 0,
            lease_valid: true,
            bundle_valid: true,
        },
    }
}

#[test]
fn overlap_is_deduped_from_greedy_marginal_gain() {
    let package_a = candidate(
        "package-a",
        2,
        vec![
            receipt(
                "package-a",
                "intent-1",
                60,
                CoverageReceiptKindV1::Projected,
            ),
            receipt(
                "package-a",
                "intent-2",
                20,
                CoverageReceiptKindV1::Projected,
            ),
            receipt("package-a", "intent-2", 20, CoverageReceiptKindV1::Actual),
        ],
    );
    let package_b = candidate(
        "package-b",
        1,
        vec![
            receipt(
                "package-b",
                "intent-1",
                60,
                CoverageReceiptKindV1::Projected,
            ),
            receipt(
                "package-b",
                "intent-3",
                30,
                CoverageReceiptKindV1::Projected,
            ),
            receipt("package-b", "intent-1", 60, CoverageReceiptKindV1::Actual),
            receipt("package-b", "intent-3", 30, CoverageReceiptKindV1::Actual),
        ],
    );

    let report =
        build_coverage_portfolio_shadow_v1(denominator(200, 100), vec![package_a, package_b])
            .expect("portfolio builds");

    assert_eq!(
        report
            .selected_packages
            .iter()
            .map(|package| (
                package.package_id.as_str(),
                package.marginal_unique_verified_tokens,
            ))
            .collect::<Vec<_>>(),
        vec![("package-b", 90), ("package-a", 20)]
    );
    assert_eq!(report.selected_projected_unique_verified_tokens, 110);
    assert_eq!(
        report
            .conservation
            .selected_projected_overlap_deduped_verified_tokens,
        60
    );
    assert_eq!(
        report
            .packages
            .iter()
            .find(|package| package.package_id == "package-a")
            .expect("package exists")
            .projection_error_verified_tokens,
        -60
    );
    assert!(report.validate());
}

#[test]
fn every_safety_failure_vetoes_before_scoring() {
    let mut unsafe_package = candidate(
        "package-unsafe",
        1,
        vec![receipt(
            "package-unsafe",
            "intent-unsafe",
            90,
            CoverageReceiptKindV1::Projected,
        )],
    );
    unsafe_package.safety = CoverageCandidateSafetyV1 {
        wrong_accepts: 1,
        parity_failures: 2,
        lease_valid: false,
        bundle_valid: false,
    };
    let safe_package = candidate(
        "package-safe",
        2,
        vec![receipt(
            "package-safe",
            "intent-safe",
            50,
            CoverageReceiptKindV1::Projected,
        )],
    );

    let report = build_coverage_portfolio_shadow_v1(
        denominator(200, 50),
        vec![unsafe_package, safe_package],
    )
    .expect("portfolio builds");

    assert_eq!(report.selected_packages[0].package_id, "package-safe");
    assert_eq!(
        report
            .packages
            .iter()
            .find(|package| package.package_id == "package-unsafe")
            .expect("package exists")
            .safety_vetoes,
        vec![
            CoverageSafetyVetoV1::WrongAccepts,
            CoverageSafetyVetoV1::ParityFailures,
            CoverageSafetyVetoV1::InvalidLease,
            CoverageSafetyVetoV1::InvalidBundle,
        ]
    );
}

#[test]
fn equal_ratios_tie_break_by_package_id_independent_of_input_order() {
    let package_a = candidate(
        "package-a",
        2,
        vec![receipt(
            "package-a",
            "intent-a",
            20,
            CoverageReceiptKindV1::Projected,
        )],
    );
    let package_b = candidate(
        "package-b",
        1,
        vec![receipt(
            "package-b",
            "intent-b",
            10,
            CoverageReceiptKindV1::Projected,
        )],
    );

    let forward = build_coverage_portfolio_shadow_v1(
        denominator(100, 15),
        vec![package_b.clone(), package_a.clone()],
    )
    .expect("portfolio builds");
    let reverse =
        build_coverage_portfolio_shadow_v1(denominator(100, 15), vec![package_a, package_b])
            .expect("portfolio builds");

    assert_eq!(forward, reverse);
    assert_eq!(forward.selected_packages.len(), 1);
    assert_eq!(forward.selected_packages[0].package_id, "package-a");
}

#[test]
fn zero_total_cost_is_invalid() {
    let zero_cost = candidate(
        "package-zero",
        0,
        vec![receipt(
            "package-zero",
            "intent-a",
            10,
            CoverageReceiptKindV1::Projected,
        )],
    );

    assert_eq!(
        build_coverage_portfolio_shadow_v1(denominator(100, 10), vec![zero_cost]),
        Err(CoveragePortfolioShadowErrorV1::InvalidCandidateCost {
            package_id: "package-zero".to_owned(),
        })
    );
}

#[test]
fn denominator_mismatch_and_duplicate_intent_are_rejected() {
    let mut wrong_root = receipt(
        "package-a",
        "intent-a",
        10,
        CoverageReceiptKindV1::Projected,
    );
    wrong_root.denominator_root_sha256 = "other-root".to_owned();
    assert_eq!(
        build_coverage_portfolio_shadow_v1(
            denominator(100, 10),
            vec![candidate("package-a", 1, vec![wrong_root])],
        ),
        Err(CoveragePortfolioShadowErrorV1::DenominatorRootMismatch {
            package_id: "package-a".to_owned(),
            intent_id: "intent-a".to_owned(),
        })
    );

    let duplicate = receipt(
        "package-b",
        "intent-b",
        10,
        CoverageReceiptKindV1::Projected,
    );
    assert_eq!(
        build_coverage_portfolio_shadow_v1(
            denominator(100, 10),
            vec![candidate(
                "package-b",
                1,
                vec![duplicate.clone(), duplicate],
            )],
        ),
        Err(CoveragePortfolioShadowErrorV1::DuplicateIntentReceipt {
            package_id: "package-b".to_owned(),
            intent_id: "intent-b".to_owned(),
            kind: CoverageReceiptKindV1::Projected,
        })
    );
}

#[test]
fn selection_stops_after_the_frozen_target_is_reached() {
    let package_a = candidate(
        "package-a",
        1,
        vec![receipt(
            "package-a",
            "intent-a",
            40,
            CoverageReceiptKindV1::Projected,
        )],
    );
    let package_b = candidate(
        "package-b",
        1,
        vec![receipt(
            "package-b",
            "intent-b",
            30,
            CoverageReceiptKindV1::Projected,
        )],
    );
    let package_c = candidate(
        "package-c",
        10,
        vec![receipt(
            "package-c",
            "intent-c",
            100,
            CoverageReceiptKindV1::Projected,
        )],
    );

    let report = build_coverage_portfolio_shadow_v1(
        denominator(200, 60),
        vec![package_c, package_b, package_a],
    )
    .expect("portfolio builds");

    assert_eq!(
        report
            .selected_packages
            .iter()
            .map(|package| package.package_id.as_str())
            .collect::<Vec<_>>(),
        vec!["package-a", "package-b"]
    );
    assert_eq!(report.selected_projected_unique_verified_tokens, 70);
    assert!(report.target_reached);
}

#[test]
fn shadow_report_never_grants_authority() {
    let report = build_coverage_portfolio_shadow_v1(
        denominator(100, 10),
        vec![candidate(
            "package-a",
            1,
            vec![receipt(
                "package-a",
                "intent-a",
                10,
                CoverageReceiptKindV1::Projected,
            )],
        )],
    )
    .expect("portfolio builds");

    assert!(!report.authority_ready);
    assert!(report.validate());
    let mut forged = report;
    forged.authority_ready = true;
    assert!(!forged.validate());
}
