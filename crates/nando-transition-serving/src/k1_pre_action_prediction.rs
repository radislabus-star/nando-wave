use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use nando_operator_kernel::{
    AtomValueType, ResponseOperation, canonical_json_sha256, response_program_version_root_sha256,
    sha256_bytes,
};
use nando_operator_learning::multi_source::K1PreActionExecutionReceiptV1;
use nando_response_actor::{
    ResponseExecutionStatus, execute_response, response_runtime_contract_sha256,
};

use crate::generation_shadow::GenerationShadowRequestV3;
use crate::k1_natural_scheduler::restore_projection;
use crate::operator_certification::CertificationAuthorityConfigV1;

const STORE_DIR: &str = "k1-pre-action-execution-receipts-v1";

pub(crate) fn precommit_if_applicable(
    config: &CertificationAuthorityConfigV1,
    request: &GenerationShadowRequestV3,
    turn_intent_id_sha256: &str,
) -> Result<Option<K1PreActionExecutionReceiptV1>, String> {
    let projection = restore_projection(config)?;
    let Some(contract) = projection.future_prediction_contract else {
        return Ok(None);
    };
    if !matches!(
        contract.canonical_program.operation,
        ResponseOperation::ComposeCollection { .. }
    ) {
        return Ok(None);
    }
    let capture = request
        .capture_receipt()
        .ok_or_else(|| "k1_pre_action_capture_receipt_missing".to_owned())?;
    if sha256_bytes(request.provider_payload_bytes()) != capture.request_root_sha256().to_hex() {
        return Err("k1_pre_action_request_digest_mismatch".to_owned());
    }
    let payload: serde_json::Value = serde_json::from_slice(request.provider_payload_bytes())
        .map_err(|_| "k1_pre_action_provider_payload_invalid".to_owned())?;
    let execution = execute_response(
        &contract.canonical_program,
        request.request_text(),
        &payload,
    );
    if execution.status != ResponseExecutionStatus::Executed {
        return Ok(None);
    }
    let response = execution
        .response
        .ok_or_else(|| "k1_pre_action_response_missing".to_owned())?;
    let program_root =
        response_program_version_root_sha256(&contract.canonical_program).map_err(str::to_owned)?;
    let response_value_type = typed_response_value(&response);
    let response_value_root = sha256_bytes(response.as_bytes());
    let consequence_root = nando_operator_learning::multi_source::typed_consequence_root_v1(
        response_value_type,
        &response_value_root,
    )
    .map_err(str::to_owned)?;
    let complete_binding_root = canonical_json_sha256(&(
        "nando.k1-complete-pre-action-input-binding.v1",
        program_root.as_str(),
        capture.request_root_sha256().to_hex(),
        sha256_bytes(request.request_text().as_bytes()),
        sha256_bytes(request.provider_payload_bytes()),
    ))
    .map_err(str::to_owned)?;
    let verifier_contract_root = canonical_json_sha256(&(
        "nando.k1-pre-action-execution-verifier-contract.v1",
        response_runtime_contract_sha256(),
        "exact_typed_consequence_root",
    ))
    .map_err(str::to_owned)?;
    let receipt = K1PreActionExecutionReceiptV1::seal(
        contract.contract_root_sha256,
        program_root,
        capture.event_root_sha256().to_hex(),
        capture.request_root_sha256().to_hex(),
        turn_intent_id_sha256.to_owned(),
        complete_binding_root,
        consequence_root,
        verifier_contract_root,
        capture.capture_sequence(),
        capture.observed_at_unix_ms(),
        unix_now_nanos()?,
    )
    .map_err(str::to_owned)?;
    persist_receipt(&config.root.join(STORE_DIR), &receipt)?;
    Ok(Some(receipt))
}

pub(crate) fn restore_for_request(
    config: &CertificationAuthorityConfigV1,
    request_root: &str,
    program_root: &str,
) -> Result<Option<K1PreActionExecutionReceiptV1>, String> {
    let path = receipt_path(&config.root.join(STORE_DIR), request_root, program_root);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("k1_pre_action_receipt_read:{error}")),
    };
    let receipt: K1PreActionExecutionReceiptV1 = serde_json::from_slice(&bytes)
        .map_err(|error| format!("k1_pre_action_receipt_decode:{error}"))?;
    receipt.validate().map_err(str::to_owned)?;
    if receipt.provider_capture_request_root_sha256 != request_root
        || receipt.canonical_program_root_sha256 != program_root
    {
        return Err("k1_pre_action_receipt_rebound".to_owned());
    }
    Ok(Some(receipt))
}

fn persist_receipt(
    directory: &Path,
    receipt: &K1PreActionExecutionReceiptV1,
) -> Result<(), String> {
    fs::create_dir_all(directory).map_err(|error| format!("k1_pre_action_mkdir:{error}"))?;
    let path = receipt_path(
        directory,
        &receipt.provider_capture_request_root_sha256,
        &receipt.canonical_program_root_sha256,
    );
    let bytes =
        serde_json::to_vec(receipt).map_err(|error| format!("k1_pre_action_encode:{error}"))?;
    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(mut file) => {
            file.write_all(&bytes)
                .and_then(|()| file.sync_all())
                .map_err(|error| format!("k1_pre_action_write:{error}"))?;
            fs::File::open(directory)
                .and_then(|dir| dir.sync_all())
                .map_err(|error| format!("k1_pre_action_dir_sync:{error}"))?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if fs::read(&path).map_err(|error| format!("k1_pre_action_existing:{error}"))? != bytes
            {
                return Err("k1_pre_action_receipt_replacement_forbidden".to_owned());
            }
        }
        Err(error) => return Err(format!("k1_pre_action_create:{error}")),
    }
    Ok(())
}

fn receipt_path(directory: &Path, request_root: &str, program_root: &str) -> PathBuf {
    directory.join(format!("{request_root}-{program_root}.json"))
}

fn typed_response_value(response: &str) -> AtomValueType {
    match serde_json::from_str::<serde_json::Value>(response) {
        Ok(serde_json::Value::Bool(_)) => AtomValueType::Boolean,
        Ok(serde_json::Value::Number(_)) => AtomValueType::Integer,
        Ok(serde_json::Value::Array(_) | serde_json::Value::Object(_)) => AtomValueType::Collection,
        _ => AtomValueType::String,
    }
}

fn unix_now_nanos() -> Result<u64, String> {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("k1_pre_action_clock:{error}"))?
            .as_nanos(),
    )
    .map_err(|_| "k1_pre_action_clock_overflow".to_owned())
}
