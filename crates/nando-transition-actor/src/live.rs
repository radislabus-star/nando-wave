use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{ExecutionStatus, SurfaceAdapter, TransitionProgram, execute_surface};

pub const PACKAGE_SCHEMA: &str = "nando.transition-package.v1";
pub const REQUEST_SCHEMA: &str = "nando.transition-request.v1";
pub const RESPONSE_SCHEMA: &str = "nando.transition-response.v1";

#[derive(Clone, Debug, Deserialize)]
pub struct TransitionPackage {
    pub schema: String,
    pub package_id: String,
    pub programs: BTreeMap<String, TransitionProgram>,
    pub adapters: BTreeMap<String, SurfaceAdapter>,
}

impl TransitionPackage {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != PACKAGE_SCHEMA {
            return Err("unsupported_package_schema".to_owned());
        }
        if self.package_id.is_empty() {
            return Err("empty_package_id".to_owned());
        }
        if self.programs.is_empty() {
            return Err("package_programs_missing".to_owned());
        }
        if self.adapters.is_empty() {
            return Err("package_adapters_missing".to_owned());
        }
        for program in self.programs.values() {
            program
                .validate()
                .map_err(|reason| format!("invalid_program:{reason}"))?;
        }
        for (adapter_id, adapter) in &self.adapters {
            if adapter.name != *adapter_id {
                return Err(format!("adapter_id_mismatch:{adapter_id}"));
            }
            adapter
                .validate()
                .map_err(|error| format!("invalid_adapter:{adapter_id}:{error}"))?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct TransitionRequest {
    pub schema: String,
    pub package_id: String,
    pub operator_id: String,
    pub adapter_id: String,
    pub before: Value,
    pub action: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveExecutionResult {
    pub local_accept: bool,
    pub verifier_ok: bool,
    pub false_accepts: u64,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
}

impl LiveExecutionResult {
    #[must_use]
    pub fn decline(reason: impl Into<String>) -> Self {
        Self {
            local_accept: false,
            verifier_ok: false,
            false_accepts: 0,
            reason: reason.into(),
            route: None,
            response: None,
        }
    }
}

#[must_use]
pub fn execute_live_request(
    package: &TransitionPackage,
    request: &TransitionRequest,
) -> LiveExecutionResult {
    if let Err(reason) = package.validate() {
        return LiveExecutionResult::decline(format!("package:{reason}"));
    }
    if request.schema != REQUEST_SCHEMA {
        return LiveExecutionResult::decline("unsupported_request_schema");
    }
    if request.package_id != package.package_id {
        return LiveExecutionResult::decline("package_id_mismatch");
    }
    let Some(program) = package.programs.get(&request.operator_id) else {
        return LiveExecutionResult::decline("operator_not_registered");
    };
    let Some(adapter) = package.adapters.get(&request.adapter_id) else {
        return LiveExecutionResult::decline("adapter_not_registered");
    };

    let result = execute_surface(program, adapter, &request.before, &request.action);
    if result.status != ExecutionStatus::Executed {
        let boundary = match result.status {
            ExecutionStatus::Abstain => "actor_abstain",
            ExecutionStatus::VerifyFailed => "actor_verify_failed",
            ExecutionStatus::Executed => "actor_output_missing",
        };
        return LiveExecutionResult::decline(format!("{boundary}:{}", result.reason));
    }
    let Some(after) = result.concrete_after else {
        return LiveExecutionResult::decline("actor_output_missing");
    };
    let route = format!(
        "typed_transition:{}:{}:{}",
        package.package_id, request.operator_id, request.adapter_id
    );
    let response = json!({
        "schema": RESPONSE_SCHEMA,
        "status": "executed",
        "package_id": package.package_id,
        "operator_id": request.operator_id,
        "adapter_id": request.adapter_id,
        "after": after,
        "proof": result.proof,
    });
    let response = match serde_json::to_string(&response) {
        Ok(response) => response,
        Err(error) => {
            return LiveExecutionResult::decline(format!("response_serialization:{error}"));
        }
    };
    LiveExecutionResult {
        local_accept: true,
        verifier_ok: true,
        false_accepts: 0,
        reason: "verified_transition_execution".to_owned(),
        route: Some(route),
        response: Some(response),
    }
}
