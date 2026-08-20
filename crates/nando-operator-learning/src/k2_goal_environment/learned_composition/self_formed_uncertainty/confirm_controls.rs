use std::io::{Read, Write};

use super::super::{K2CompositionErrorV1, K2CompositionResultV1, composition_sha256_file_v1};
use super::{
    K2_UNCERTAINTY_CONTROL_RECEIPT_SCHEMA_V1, K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1,
    K2UncertaintyControlEvaluationReceiptV1, K2UncertaintyControlEvaluationRequestV1,
    K2UncertaintyControlScopeV1, K2UncertaintyControlStdoutV1, denied_authority_v1,
    uncertainty_bytes_v1, uncertainty_decode_v1,
};

pub fn evaluate_self_formed_controls_v1(
    request: &K2UncertaintyControlEvaluationRequestV1,
) -> K2CompositionResultV1<K2UncertaintyControlEvaluationReceiptV1> {
    request.validate()?;
    for (ordinal, outcome) in request.outcomes.iter().enumerate() {
        let (expected_id, expected_disposition) = expected_control_v1(request.scope, ordinal)?;
        let decoded: K2UncertaintyControlStdoutV1 = uncertainty_decode_v1(&outcome.stdout_bytes)?;
        if outcome.control_id != expected_id
            || outcome.decoded_disposition != expected_disposition
            || decoded.control_id != expected_id
            || decoded.disposition != expected_disposition
            || !outcome.normal_exit
            || outcome.exit_code != 0
            || outcome.timed_out
            || outcome.panicked
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_control_process_predicate_failed",
            ));
        }
    }
    let mut receipt = K2UncertaintyControlEvaluationReceiptV1 {
        schema: K2_UNCERTAINTY_CONTROL_RECEIPT_SCHEMA_V1.to_owned(),
        scope: request.scope,
        request_root_sha256: request.request_root_sha256.clone(),
        experiment_root_sha256: request.experiment_root_sha256.clone(),
        freeze_root_sha256: request.freeze_root_sha256.clone(),
        attempt_root_sha256: request.attempt_root_sha256.clone(),
        outcomes: request.outcomes.clone(),
        passed: request.outcomes.len() as u64,
        expected: request.scope.expected_count() as u64,
        all_pass: true,
        evaluator_executable_sha256: request.evaluator_executable_sha256.clone(),
        authority: denied_authority_v1(),
        receipt_root_sha256: String::new(),
    };
    receipt.reseal()?;
    Ok(receipt)
}

pub fn expected_self_formed_control_v1(
    scope: K2UncertaintyControlScopeV1,
    ordinal: usize,
) -> K2CompositionResultV1<(String, String)> {
    expected_control_v1(scope, ordinal)
}

fn expected_control_v1(
    scope: K2UncertaintyControlScopeV1,
    ordinal: usize,
) -> K2CompositionResultV1<(String, String)> {
    if ordinal >= scope.expected_count() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_control_ordinal_invalid",
        ));
    }
    let value = match scope {
        K2UncertaintyControlScopeV1::SuccessorStaticLegacy => {
            (format!("legacy-{:02}", ordinal + 1), "pass".to_owned())
        }
        K2UncertaintyControlScopeV1::SuccessorStaticV3 => {
            (format!("v3-{:02}", ordinal + 1), "pass".to_owned())
        }
        K2UncertaintyControlScopeV1::SuccessorStaticV4 => {
            (format!("v4-{:02}", ordinal + 1), "pass".to_owned())
        }
        K2UncertaintyControlScopeV1::DevelopmentRehearsalV5
        | K2UncertaintyControlScopeV1::SealedAttemptV5 => {
            let dispositions = [
                "reused_development_commitment_rejected",
                "missing_or_foreign_authorization_rejected",
                "nonce_transport_rejected",
                "private_public_leakage_rejected",
                "early_private_resolver_rejected",
                "early_final_truth_rejected",
                "coordinator_manifest_mismatch_rejected",
                "duplicate_slot_attempt_or_nonce_rejected",
                "partial_terminal_denominator_rejected",
                "one_probe_oracle_substitution_rejected",
                "baseline_denominator_omission_rejected",
                "cleanup_retention_or_residue_violation_rejected",
            ];
            (
                format!("K{}", ordinal + 1),
                dispositions[ordinal].to_owned(),
            )
        }
    };
    Ok(value)
}

pub fn run_self_formed_control_evaluator_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_control_stdin"))?;
    let request: K2UncertaintyControlEvaluationRequestV1 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_control_evaluator"))?;
    if composition_sha256_file_v1(&executable)? != request.evaluator_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_control_evaluator_executable_mismatch",
        ));
    }
    let receipt = evaluate_self_formed_controls_v1(&request)?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_control_stdout"))
}
