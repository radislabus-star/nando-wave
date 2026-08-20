use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, K2InquiryBaselineKindV1,
    inquiry_observable_outcome_root_v1,
};
use super::{
    K2_UNCERTAINTY_MAX_PLAN_COST_UNITS_V1, K2_UNCERTAINTY_MAX_PLAN_RISK_UNITS_V1,
    K2_UNCERTAINTY_ORACLE_BASELINE_RESULT_SCHEMA_V1, K2_UNCERTAINTY_ORACLE_CASE_RECEIPT_SCHEMA_V1,
    K2_UNCERTAINTY_ORACLE_ENUMERATION_SCHEMA_V1, K2_UNCERTAINTY_ORACLE_PLAN_RESULT_SCHEMA_V1,
    K2UncertaintyBaselineSummaryV1, K2UncertaintyClosurePlanV1,
    K2UncertaintyConfirmFinalTruthCaseV1, K2UncertaintyConfirmFinalVerifierReceiptV1,
    K2UncertaintyEligibilityDispositionV1, K2UncertaintyModelSetV1,
    K2UncertaintyObservationVectorV2, K2UncertaintyOracleBaselineCaseDescriptorV1,
    K2UncertaintyOracleBaselineCaseReceiptV1, K2UncertaintyOracleBaselineResultV1,
    K2UncertaintyOracleCaseEvidenceManifestV1, K2UncertaintyOracleEnumerationCensusV1,
    K2UncertaintyOracleFrontierReceiptV1, K2UncertaintyOraclePlanResultV1,
    K2UncertaintyRawProbeDispositionV1, K2UncertaintySafetyDispositionV1, denied_authority_v1,
    oracle_apply_effect_v1, uncertainty_root_v1,
};

#[derive(Clone)]
struct ProbeActualV1 {
    syntax_roots: BTreeSet<String>,
    actual_outcome_root_sha256: String,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum PlanRejectionV1 {
    Eligible,
    ProbeIneligible,
    RiskBudget,
    CostBudget,
}

#[derive(Serialize)]
struct EnumerationRowV1<'a> {
    ordered_probe_roots_sha256: &'a [String],
    source_ordinals: &'a [u64],
    rejection: PlanRejectionV1,
    residual_syntax_roots_sha256: &'a [String],
    residual_semantic_class_roots_sha256: &'a [String],
    true_class_retained: bool,
    risk_units: u64,
    cost_units: u64,
}

#[allow(clippy::too_many_arguments)]
pub fn evaluate_self_formed_oracle_case_v1(
    descriptor: &K2UncertaintyOracleBaselineCaseDescriptorV1,
    manifest: &K2UncertaintyOracleCaseEvidenceManifestV1,
    model_set: &K2UncertaintyModelSetV1,
    representatives: &[K2UncertaintyRawProbeDispositionV1],
    reconstructed_frontier: K2UncertaintyOracleFrontierReceiptV1,
    closure_plan: &K2UncertaintyClosurePlanV1,
    baselines: &K2UncertaintyBaselineSummaryV1,
    observations: &K2UncertaintyObservationVectorV2,
    final_verifier: &K2UncertaintyConfirmFinalVerifierReceiptV1,
    private_truth: &K2UncertaintyConfirmFinalTruthCaseV1,
) -> K2CompositionResultV1<K2UncertaintyOracleBaselineCaseReceiptV1> {
    descriptor.validate()?;
    manifest.validate()?;
    model_set.validate()?;
    closure_plan.validate()?;
    baselines.validate()?;
    observations.validate()?;
    final_verifier.validate()?;
    private_truth.validate()?;
    if manifest.manifest_root_sha256 != descriptor.case_evidence_manifest_root_sha256
        || model_set.case_id_sha256 != descriptor.case_id_sha256
        || closure_plan.case_id_sha256 != descriptor.case_id_sha256
        || closure_plan.plan_root_sha256 != descriptor.closure_plan_root_sha256
        || baselines.case_id_sha256 != descriptor.case_id_sha256
        || baselines.summary_root_sha256 != descriptor.baseline_summary_root_sha256
        || observations.case_id_sha256 != descriptor.case_id_sha256
        || observations.vector_root_sha256 != descriptor.observation_vector_root_sha256
        || final_verifier.verification.case_id_sha256 != descriptor.case_id_sha256
        || final_verifier.receipt_root_sha256 != descriptor.final_verifier_receipt_root_sha256
        || private_truth.private_case.case_id_sha256 != descriptor.case_id_sha256
        || private_truth.final_truth_root_sha256 != descriptor.private_truth_artifact_root_sha256
        || final_verifier.final_truth_root_sha256 != private_truth.final_truth_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_case_binding_invalid",
        ));
    }

    let (true_syntax_root, true_class_root, syntax_to_class) =
        derive_true_class_v1(model_set, private_truth)?;
    let initial_syntax = model_set
        .syntactic_models
        .iter()
        .map(|model| model.syntax_root_sha256.clone())
        .collect::<BTreeSet<_>>();
    let mut actuals = BTreeMap::new();
    for probe in representatives {
        let actual = evaluate_probe_v1(model_set, private_truth, probe)?;
        actuals.insert(probe.probe.probe_root_sha256.clone(), actual);
    }

    let model_guided = evaluate_plan_v1(
        &closure_plan.ordered_probe_roots_sha256,
        representatives,
        &actuals,
        &initial_syntax,
        &syntax_to_class,
        &true_class_root,
    )?;
    if observations.ordered_observable_outcome_roots_sha256.len()
        != closure_plan.ordered_probe_roots_sha256.len()
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_observation_denominator_invalid",
        ));
    }
    for ((probe_root, observed_root), execution) in closure_plan
        .ordered_probe_roots_sha256
        .iter()
        .zip(&observations.ordered_observable_outcome_roots_sha256)
        .zip(&observations.executions)
    {
        let actual = actuals
            .get(probe_root)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_model_guided_probe_missing",
            ))?;
        if &actual.actual_outcome_root_sha256 != observed_root
            || &execution.selected_probe_root_sha256 != probe_root
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_observation_parity_invalid",
            ));
        }
    }

    let (enumeration, oracle) = enumerate_oracle_v1(
        representatives,
        &actuals,
        &initial_syntax,
        &syntax_to_class,
        &true_class_root,
    )?;
    let baseline_results = evaluate_baselines_v1(
        baselines,
        representatives,
        &actuals,
        &initial_syntax,
        &syntax_to_class,
        &true_class_root,
    )?;
    let oracle_equality = model_guided.residual_semantic_class_roots_sha256.len() == 1
        && model_guided.true_class_retained
        && oracle.residual_semantic_class_roots_sha256.len() == 1
        && oracle.true_class_retained;
    let mut receipt = K2UncertaintyOracleBaselineCaseReceiptV1 {
        schema: K2_UNCERTAINTY_ORACLE_CASE_RECEIPT_SCHEMA_V1.to_owned(),
        case_sequence: descriptor.case_sequence,
        case_id_sha256: descriptor.case_id_sha256.clone(),
        descriptor_root_sha256: descriptor.descriptor_root()?,
        evidence_manifest_root_sha256: manifest.manifest_root_sha256.clone(),
        reconstructed_frontier,
        exact_plan_denominator: enumeration.expected_plan_count,
        enumeration,
        true_syntax_root_sha256: true_syntax_root,
        true_semantic_class_root_sha256: true_class_root,
        model_guided,
        model_guided_observation_parity: true,
        oracle,
        oracle_equality,
        baselines: baseline_results,
        final_verifier_receipt_root_sha256: final_verifier.receipt_root_sha256.clone(),
        false_accepts: final_verifier.verification.false_accepts,
        evaluator_executable_sha256: descriptor.oracle_evaluator_executable_sha256.clone(),
        authority: denied_authority_v1(),
        receipt_root_sha256: String::new(),
    };
    receipt.reseal()?;
    Ok(receipt)
}

fn derive_true_class_v1(
    model_set: &K2UncertaintyModelSetV1,
    truth: &K2UncertaintyConfirmFinalTruthCaseV1,
) -> K2CompositionResultV1<(String, String, BTreeMap<String, String>)> {
    let mapping = truth
        .private_case
        .mapping
        .iter()
        .map(|entry| (entry.opaque_action_root_sha256.as_str(), &entry.effect))
        .collect::<BTreeMap<_, _>>();
    let matches = model_set
        .syntactic_models
        .iter()
        .filter(|model| {
            model.actions.iter().all(|action| {
                mapping.get(action.action_id_sha256.as_str()).copied() == Some(&action.effect)
            })
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_true_syntax_not_unique",
        ));
    }
    let mut syntax_to_class = BTreeMap::new();
    for class in &model_set.semantic_classes {
        for syntax in &class.syntax_member_roots_sha256 {
            if syntax_to_class
                .insert(syntax.clone(), class.class_root_sha256.clone())
                .is_some()
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_oracle_semantic_partition_duplicate",
                ));
            }
        }
    }
    let syntax = matches[0].syntax_root_sha256.clone();
    let class = syntax_to_class
        .get(&syntax)
        .cloned()
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_true_class_missing",
        ))?;
    Ok((syntax, class, syntax_to_class))
}

fn evaluate_probe_v1(
    model_set: &K2UncertaintyModelSetV1,
    truth: &K2UncertaintyConfirmFinalTruthCaseV1,
    disposition: &K2UncertaintyRawProbeDispositionV1,
) -> K2CompositionResultV1<ProbeActualV1> {
    let private_effect = truth
        .private_case
        .mapping
        .iter()
        .find(|entry| entry.opaque_action_root_sha256 == disposition.probe.action_id_sha256)
        .map(|entry| &entry.effect)
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_private_action_missing",
        ))?;
    let (_, _, private_post) =
        oracle_apply_effect_v1(&disposition.probe.initial_manifest, private_effect)?;
    let actual_outcome_root_sha256 =
        inquiry_observable_outcome_root_v1(disposition.probe.observation_mode, &private_post)?;
    let mut syntax_roots = BTreeSet::new();
    for model in &model_set.syntactic_models {
        let effect = model
            .actions
            .iter()
            .find(|action| action.action_id_sha256 == disposition.probe.action_id_sha256)
            .map(|action| &action.effect)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_model_action_missing",
            ))?;
        let (_, _, post) = oracle_apply_effect_v1(&disposition.probe.initial_manifest, effect)?;
        let outcome =
            inquiry_observable_outcome_root_v1(disposition.probe.observation_mode, &post)?;
        if outcome == actual_outcome_root_sha256 {
            syntax_roots.insert(model.syntax_root_sha256.clone());
        }
    }
    if syntax_roots.is_empty() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_actual_outcome_eliminates_truth",
        ));
    }
    Ok(ProbeActualV1 {
        syntax_roots,
        actual_outcome_root_sha256,
    })
}

fn evaluate_plan_v1(
    roots: &[String],
    representatives: &[K2UncertaintyRawProbeDispositionV1],
    actuals: &BTreeMap<String, ProbeActualV1>,
    initial_syntax: &BTreeSet<String>,
    syntax_to_class: &BTreeMap<String, String>,
    true_class_root: &str,
) -> K2CompositionResultV1<K2UncertaintyOraclePlanResultV1> {
    if roots.is_empty() || roots.len() > 2 || roots.len() == 2 && roots[0] == roots[1] {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_plan_roots_invalid",
        ));
    }
    let mut residual = initial_syntax.clone();
    let mut risk = 0_u64;
    let mut cost = 0_u64;
    for root in roots {
        let probe = representatives
            .iter()
            .find(|probe| &probe.probe.probe_root_sha256 == root)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_plan_probe_missing",
            ))?;
        let actual = actuals.get(root).ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_plan_actual_missing",
        ))?;
        residual = residual
            .intersection(&actual.syntax_roots)
            .cloned()
            .collect();
        risk = risk
            .checked_add(probe.probe.risk_units)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_risk_overflow",
            ))?;
        cost = cost
            .checked_add(probe.probe.cost_units)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_cost_overflow",
            ))?;
    }
    let classes = residual
        .iter()
        .map(|syntax| {
            syntax_to_class
                .get(syntax)
                .cloned()
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_oracle_residual_class_missing",
                ))
        })
        .collect::<K2CompositionResultV1<BTreeSet<_>>>()?;
    let mut result = K2UncertaintyOraclePlanResultV1 {
        schema: K2_UNCERTAINTY_ORACLE_PLAN_RESULT_SCHEMA_V1.to_owned(),
        ordered_probe_roots_sha256: roots.to_vec(),
        residual_syntax_roots_sha256: residual.into_iter().collect(),
        residual_semantic_class_roots_sha256: classes.iter().cloned().collect(),
        true_class_retained: classes.contains(true_class_root),
        cumulative_risk_units: risk,
        cumulative_cost_units: cost,
        result_root_sha256: String::new(),
    };
    result.reseal()?;
    Ok(result)
}

fn enumerate_oracle_v1(
    representatives: &[K2UncertaintyRawProbeDispositionV1],
    actuals: &BTreeMap<String, ProbeActualV1>,
    initial_syntax: &BTreeSet<String>,
    syntax_to_class: &BTreeMap<String, String>,
    true_class_root: &str,
) -> K2CompositionResultV1<(
    K2UncertaintyOracleEnumerationCensusV1,
    K2UncertaintyOraclePlanResultV1,
)> {
    let n = representatives.len() as u64;
    let expected = n.checked_mul(n).ok_or(K2CompositionErrorV1::Invalid(
        "self_formed_oracle_plan_count_overflow",
    ))?;
    let mut enumerated = 0_u64;
    let mut eligible = 0_u64;
    let mut rejected_probe_ineligible = 0_u64;
    let mut rejected_risk_budget = 0_u64;
    let mut rejected_cost_budget = 0_u64;
    let mut chain = uncertainty_root_v1(&("nando.k2-self-formed-oracle-enumeration-chain.v1", n))?;
    let mut winner: Option<K2UncertaintyOraclePlanResultV1> = None;

    for first in 0..representatives.len() {
        process_plan_v1(
            &[first],
            representatives,
            actuals,
            initial_syntax,
            syntax_to_class,
            true_class_root,
            &mut chain,
            &mut enumerated,
            &mut eligible,
            &mut rejected_probe_ineligible,
            &mut rejected_risk_budget,
            &mut rejected_cost_budget,
            &mut winner,
        )?;
        for second in 0..representatives.len() {
            if first == second {
                continue;
            }
            process_plan_v1(
                &[first, second],
                representatives,
                actuals,
                initial_syntax,
                syntax_to_class,
                true_class_root,
                &mut chain,
                &mut enumerated,
                &mut eligible,
                &mut rejected_probe_ineligible,
                &mut rejected_risk_budget,
                &mut rejected_cost_budget,
                &mut winner,
            )?;
        }
    }
    let mut census = K2UncertaintyOracleEnumerationCensusV1 {
        schema: K2_UNCERTAINTY_ORACLE_ENUMERATION_SCHEMA_V1.to_owned(),
        representative_count: n,
        expected_plan_count: expected,
        enumerated,
        eligible,
        rejected_probe_ineligible,
        rejected_risk_budget,
        rejected_cost_budget,
        enumeration_chain_root_sha256: chain,
        census_root_sha256: String::new(),
    };
    census.reseal()?;
    Ok((
        census,
        winner.ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_eligible_plan_missing",
        ))?,
    ))
}

#[allow(clippy::too_many_arguments)]
fn process_plan_v1(
    indexes: &[usize],
    representatives: &[K2UncertaintyRawProbeDispositionV1],
    actuals: &BTreeMap<String, ProbeActualV1>,
    initial_syntax: &BTreeSet<String>,
    syntax_to_class: &BTreeMap<String, String>,
    true_class_root: &str,
    chain: &mut String,
    enumerated: &mut u64,
    eligible: &mut u64,
    rejected_probe_ineligible: &mut u64,
    rejected_risk_budget: &mut u64,
    rejected_cost_budget: &mut u64,
    winner: &mut Option<K2UncertaintyOraclePlanResultV1>,
) -> K2CompositionResultV1<()> {
    let selected = indexes
        .iter()
        .map(|index| &representatives[*index])
        .collect::<Vec<_>>();
    let roots = selected
        .iter()
        .map(|probe| probe.probe.probe_root_sha256.clone())
        .collect::<Vec<_>>();
    let ordinals = selected
        .iter()
        .map(|probe| probe.raw_sequence)
        .collect::<Vec<_>>();
    let risk = selected
        .iter()
        .try_fold(0_u64, |total, probe| {
            total.checked_add(probe.probe.risk_units)
        })
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_risk_overflow",
        ))?;
    let cost = selected
        .iter()
        .try_fold(0_u64, |total, probe| {
            total.checked_add(probe.probe.cost_units)
        })
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_cost_overflow",
        ))?;
    let rejection = if selected.iter().any(|probe| {
        probe.eligibility != K2UncertaintyEligibilityDispositionV1::Eligible
            || probe.safety != K2UncertaintySafetyDispositionV1::Pass
            || !probe.probe.reversible
    }) {
        *rejected_probe_ineligible += 1;
        PlanRejectionV1::ProbeIneligible
    } else if risk > K2_UNCERTAINTY_MAX_PLAN_RISK_UNITS_V1 {
        *rejected_risk_budget += 1;
        PlanRejectionV1::RiskBudget
    } else if cost > K2_UNCERTAINTY_MAX_PLAN_COST_UNITS_V1 {
        *rejected_cost_budget += 1;
        PlanRejectionV1::CostBudget
    } else {
        *eligible += 1;
        PlanRejectionV1::Eligible
    };
    let result = evaluate_plan_v1(
        &roots,
        representatives,
        actuals,
        initial_syntax,
        syntax_to_class,
        true_class_root,
    )?;
    let row = EnumerationRowV1 {
        ordered_probe_roots_sha256: &roots,
        source_ordinals: &ordinals,
        rejection,
        residual_syntax_roots_sha256: &result.residual_syntax_roots_sha256,
        residual_semantic_class_roots_sha256: &result.residual_semantic_class_roots_sha256,
        true_class_retained: result.true_class_retained,
        risk_units: risk,
        cost_units: cost,
    };
    *chain = uncertainty_root_v1(&(
        "nando.k2-self-formed-oracle-enumeration-row.v1",
        &*chain,
        row,
    ))?;
    *enumerated = enumerated
        .checked_add(1)
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_enumeration_overflow",
        ))?;
    if matches!(rejection, PlanRejectionV1::Eligible)
        && winner
            .as_ref()
            .is_none_or(|current| plan_rank_v1(&result) < plan_rank_v1(current))
    {
        *winner = Some(result);
    }
    Ok(())
}

fn plan_rank_v1(plan: &K2UncertaintyOraclePlanResultV1) -> (usize, usize, u64, u64, &[String]) {
    (
        plan.residual_semantic_class_roots_sha256.len(),
        plan.ordered_probe_roots_sha256.len(),
        plan.cumulative_risk_units,
        plan.cumulative_cost_units,
        &plan.ordered_probe_roots_sha256,
    )
}

fn evaluate_baselines_v1(
    summary: &K2UncertaintyBaselineSummaryV1,
    representatives: &[K2UncertaintyRawProbeDispositionV1],
    actuals: &BTreeMap<String, ProbeActualV1>,
    initial_syntax: &BTreeSet<String>,
    syntax_to_class: &BTreeMap<String, String>,
    true_class_root: &str,
) -> K2CompositionResultV1<Vec<K2UncertaintyOracleBaselineResultV1>> {
    let initial_classes = initial_syntax
        .iter()
        .map(|syntax| {
            syntax_to_class
                .get(syntax)
                .cloned()
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_oracle_initial_class_missing",
                ))
        })
        .collect::<K2CompositionResultV1<BTreeSet<_>>>()?;
    let mut results = Vec::with_capacity(4);
    for decision in &summary.decisions {
        let (classes, retained, risk, cost) =
            if let Some(root) = &decision.selected_probe_root_sha256 {
                let plan = evaluate_plan_v1(
                    std::slice::from_ref(root),
                    representatives,
                    actuals,
                    initial_syntax,
                    syntax_to_class,
                    true_class_root,
                )?;
                (
                    plan.residual_semantic_class_roots_sha256.len() as u64,
                    plan.true_class_retained,
                    plan.cumulative_risk_units,
                    plan.cumulative_cost_units,
                )
            } else {
                (
                    initial_classes.len() as u64,
                    initial_classes.contains(true_class_root),
                    0,
                    0,
                )
            };
        let mut result = K2UncertaintyOracleBaselineResultV1 {
            schema: K2_UNCERTAINTY_ORACLE_BASELINE_RESULT_SCHEMA_V1.to_owned(),
            kind: decision.kind,
            selected_probe_root_sha256: decision.selected_probe_root_sha256.clone(),
            residual_semantic_classes: classes,
            true_class_retained: retained,
            risk_units: risk,
            cost_units: cost,
            result_root_sha256: String::new(),
        };
        result.reseal()?;
        results.push(result);
    }
    let expected = [
        K2InquiryBaselineKindV1::Passive,
        K2InquiryBaselineKindV1::StableHash,
        K2InquiryBaselineKindV1::CheapestFirst,
        K2InquiryBaselineKindV1::ExplicitHeuristic,
    ];
    if results.iter().map(|result| result.kind).ne(expected) {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_baseline_order_invalid",
        ));
    }
    Ok(results)
}
