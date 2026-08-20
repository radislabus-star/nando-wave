use std::io::{Read, Write};

use super::super::{K2CompositionErrorV1, K2CompositionResultV1, composition_sha256_file_v1};
use super::{
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2UncertaintyControlScopeV1,
    K2UncertaintyDevelopmentRehearsalTerminalRequestV1, K2UncertaintySealedTerminalRequestV1,
    K2UncertaintyTerminalDispositionV1, K2UncertaintyTerminalEvaluationReceiptV1,
    K2UncertaintyTerminalModeV1, uncertainty_bytes_v1, uncertainty_decode_v1,
    validate_terminal_routes_v1,
};

pub fn evaluate_self_formed_development_terminal_v1(
    request: &K2UncertaintyDevelopmentRehearsalTerminalRequestV1,
) -> K2CompositionResultV1<K2UncertaintyTerminalEvaluationReceiptV1> {
    request.validate_envelope()?;
    let infrastructure = validate_common_v1(
        &request.oracle_batch,
        &request.controls,
        &request.routes,
        &request.resources,
        &[
            K2UncertaintyControlScopeV1::SuccessorStaticLegacy,
            K2UncertaintyControlScopeV1::SuccessorStaticV3,
            K2UncertaintyControlScopeV1::SuccessorStaticV4,
            K2UncertaintyControlScopeV1::DevelopmentRehearsalV5,
        ],
    );
    let (disposition, reason) = if infrastructure.is_err() {
        (
            K2UncertaintyTerminalDispositionV1::InfrastructureFail,
            "development_evidence_invalid",
        )
    } else if !scientific_conjuncts_v1(&request.oracle_batch) {
        (
            K2UncertaintyTerminalDispositionV1::ScientificFail,
            "development_scientific_conjunct_false",
        )
    } else {
        (
            K2UncertaintyTerminalDispositionV1::DevelopmentRehearsalPass,
            "development_component_routes_complete",
        )
    };
    K2UncertaintyTerminalEvaluationReceiptV1::seal(
        K2UncertaintyTerminalModeV1::DevelopmentRehearsal,
        request.request_root_sha256.clone(),
        disposition,
        reason.to_owned(),
        request.terminal_evaluator_executable_sha256.clone(),
    )
}

pub fn evaluate_self_formed_sealed_terminal_v1(
    request: &K2UncertaintySealedTerminalRequestV1,
) -> K2CompositionResultV1<K2UncertaintyTerminalEvaluationReceiptV1> {
    request.validate_envelope()?;
    let (disposition, reason) =
        if request.irreversible_dispatch_missing_results > 0 || request.ambiguous_results > 0 {
            (
                K2UncertaintyTerminalDispositionV1::Indeterminate,
                "durable_dispatch_result_missing_or_ambiguous",
            )
        } else if request.sealed_projection.validate().is_err()
            || validate_common_v1(
                &request.oracle_batch,
                &request.controls,
                &request.routes,
                &request.resources,
                &[
                    K2UncertaintyControlScopeV1::SuccessorStaticLegacy,
                    K2UncertaintyControlScopeV1::SuccessorStaticV3,
                    K2UncertaintyControlScopeV1::SuccessorStaticV4,
                    K2UncertaintyControlScopeV1::SealedAttemptV5,
                ],
            )
            .is_err()
        {
            (
                K2UncertaintyTerminalDispositionV1::InfrastructureFail,
                "sealed_infrastructure_conjunct_false",
            )
        } else if !scientific_conjuncts_v1(&request.oracle_batch) {
            (
                K2UncertaintyTerminalDispositionV1::ScientificFail,
                "sealed_scientific_conjunct_false",
            )
        } else {
            (
                K2UncertaintyTerminalDispositionV1::K2SelfFormedUncertaintyCapabilityPass,
                "sealed_capability_conjuncts_complete",
            )
        };
    K2UncertaintyTerminalEvaluationReceiptV1::seal(
        K2UncertaintyTerminalModeV1::SealedAttempt,
        request.request_root_sha256.clone(),
        disposition,
        reason.to_owned(),
        request.terminal_evaluator_executable_sha256.clone(),
    )
}

fn validate_common_v1(
    oracle: &super::K2UncertaintyOracleBaselineBatchReceiptV1,
    controls: &[super::K2UncertaintyControlEvaluationReceiptV1],
    routes: &[super::K2UncertaintyEvaluationRouteReceiptV1],
    resources: &super::K2UncertaintyEvaluationResourceMeasurementsV1,
    expected_scopes: &[K2UncertaintyControlScopeV1],
) -> K2CompositionResultV1<()> {
    oracle.validate()?;
    if controls.len() != expected_scopes.len() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_terminal_control_scope_count_invalid",
        ));
    }
    for (receipt, expected) in controls.iter().zip(expected_scopes) {
        receipt.validate()?;
        if receipt.scope != *expected {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_terminal_control_scope_invalid",
            ));
        }
    }
    validate_terminal_routes_v1(routes)?;
    resources.validate()?;
    if oracle.false_accepts != 0 || resources.false_accepts != 0 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_terminal_false_accepts_nonzero",
        ));
    }
    Ok(())
}

fn scientific_conjuncts_v1(oracle: &super::K2UncertaintyOracleBaselineBatchReceiptV1) -> bool {
    oracle.oracle_equal_cases == 16
        && oracle.true_class_retained_cases == 16
        && oracle
            .aggregates
            .iter()
            .all(|aggregate| aggregate.aggregate_superiority && aggregate.threshold_pass)
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum K2UncertaintyTerminalProcessRequestV1 {
    Development {
        request: K2UncertaintyDevelopmentRehearsalTerminalRequestV1,
    },
    Sealed {
        request: K2UncertaintySealedTerminalRequestV1,
    },
}

pub fn run_self_formed_terminal_evaluator_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_terminal_stdin"))?;
    let request: K2UncertaintyTerminalProcessRequestV1 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_terminal_evaluator"))?;
    let executable_sha256 = composition_sha256_file_v1(&executable)?;
    let receipt = match request {
        K2UncertaintyTerminalProcessRequestV1::Development { request } => {
            if request.terminal_evaluator_executable_sha256 != executable_sha256 {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_terminal_evaluator_executable_mismatch",
                ));
            }
            evaluate_self_formed_development_terminal_v1(&request)?
        }
        K2UncertaintyTerminalProcessRequestV1::Sealed { request } => {
            if request.terminal_evaluator_executable_sha256 != executable_sha256 {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_terminal_evaluator_executable_mismatch",
                ));
            }
            evaluate_self_formed_sealed_terminal_v1(&request)?
        }
    };
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_terminal_stdout"))
}
