use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_DEVELOPMENT_TERMINAL_REQUEST_SCHEMA_V1, K2_UNCERTAINTY_MAX_BATCH_WALL_MS_V1,
    K2_UNCERTAINTY_MAX_CASE_WALL_MS_V1, K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1,
    K2_UNCERTAINTY_MAX_RESIDENT_BYTES_V1, K2_UNCERTAINTY_RESOURCE_MEASUREMENTS_SCHEMA_V1,
    K2_UNCERTAINTY_ROUTE_RECEIPT_SCHEMA_V1, K2_UNCERTAINTY_SEALED_TERMINAL_REQUEST_SCHEMA_V1,
    K2_UNCERTAINTY_TERMINAL_RECEIPT_SCHEMA_V1, K2UncertaintyControlEvaluationReceiptV1,
    K2UncertaintyOracleBaselineBatchReceiptV1, denied_authority_v1, require_denied_authority_v1,
    uncertainty_root_v1,
};

pub const K2_UNCERTAINTY_TERMINAL_ROUTE_IDS_V1: [&str; 5] = [
    "public_precommit",
    "case_execution",
    "final_verification",
    "oracle_evaluation",
    "control_evaluation",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyEvaluationRouteReceiptV1 {
    pub schema: String,
    pub route_id: String,
    pub producer_executable_sha256: String,
    pub consumer_executable_sha256: String,
    pub expected_events: u64,
    pub observed_events: u64,
    pub complete: bool,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyEvaluationRouteReceiptV1 {
    pub fn seal(
        route_id: String,
        producer_executable_sha256: String,
        consumer_executable_sha256: String,
        expected_events: u64,
        observed_events: u64,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_ROUTE_RECEIPT_SCHEMA_V1.to_owned(),
            route_id,
            producer_executable_sha256,
            consumer_executable_sha256,
            expected_events,
            observed_events,
            complete: expected_events == observed_events && expected_events > 0,
            authority: denied_authority_v1(),
            receipt_root_sha256: String::new(),
        };
        value.receipt_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.producer_executable_sha256)?;
        require_composition_root_v1(&self.consumer_executable_sha256)?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_ROUTE_RECEIPT_SCHEMA_V1
            || !K2_UNCERTAINTY_TERMINAL_ROUTE_IDS_V1.contains(&self.route_id.as_str())
            || self.producer_executable_sha256 == self.consumer_executable_sha256
            || self.expected_events == 0
            || self.complete != (self.expected_events == self.observed_events)
            || !self.complete
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_terminal_route_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_ROUTE_RECEIPT_SCHEMA_V1,
            &self.route_id,
            &self.producer_executable_sha256,
            &self.consumer_executable_sha256,
            self.expected_events,
            self.observed_events,
            self.complete,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyEvaluationResourceMeasurementsV1 {
    pub schema: String,
    pub peak_resident_bytes: u64,
    pub maximum_case_wall_ms: u64,
    pub batch_wall_ms: u64,
    pub maximum_protocol_bytes: u64,
    pub false_accepts: u64,
    pub forbidden_executions: u64,
    pub authority_promotions: u64,
    pub production_effects: u64,
    pub network_effects: u64,
    pub resource_violations: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyEvaluationResourceMeasurementsV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        peak_resident_bytes: u64,
        maximum_case_wall_ms: u64,
        batch_wall_ms: u64,
        maximum_protocol_bytes: u64,
        false_accepts: u64,
        forbidden_executions: u64,
        authority_promotions: u64,
        production_effects: u64,
        network_effects: u64,
        resource_violations: u64,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_RESOURCE_MEASUREMENTS_SCHEMA_V1.to_owned(),
            peak_resident_bytes,
            maximum_case_wall_ms,
            batch_wall_ms,
            maximum_protocol_bytes,
            false_accepts,
            forbidden_executions,
            authority_promotions,
            production_effects,
            network_effects,
            resource_violations,
            authority: denied_authority_v1(),
            receipt_root_sha256: String::new(),
        };
        value.receipt_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_RESOURCE_MEASUREMENTS_SCHEMA_V1
            || self.peak_resident_bytes > K2_UNCERTAINTY_MAX_RESIDENT_BYTES_V1
            || self.maximum_case_wall_ms > K2_UNCERTAINTY_MAX_CASE_WALL_MS_V1
            || self.batch_wall_ms > K2_UNCERTAINTY_MAX_BATCH_WALL_MS_V1
            || self.maximum_protocol_bytes >= K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 as u64
            || self.false_accepts != 0
            || self.forbidden_executions != 0
            || self.authority_promotions != 0
            || self.production_effects != 0
            || self.network_effects != 0
            || self.resource_violations != 0
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_terminal_resource_measurements_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_RESOURCE_MEASUREMENTS_SCHEMA_V1,
            self.peak_resident_bytes,
            self.maximum_case_wall_ms,
            self.batch_wall_ms,
            self.maximum_protocol_bytes,
            self.false_accepts,
            self.forbidden_executions,
            self.authority_promotions,
            self.production_effects,
            self.network_effects,
            self.resource_violations,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyDevelopmentRehearsalTerminalRequestV1 {
    pub schema: String,
    pub experiment_root_sha256: String,
    pub oracle_batch: K2UncertaintyOracleBaselineBatchReceiptV1,
    pub controls: Vec<K2UncertaintyControlEvaluationReceiptV1>,
    pub routes: Vec<K2UncertaintyEvaluationRouteReceiptV1>,
    pub resources: K2UncertaintyEvaluationResourceMeasurementsV1,
    pub sealed_attempts: u64,
    pub authorization_slots: u64,
    pub nonce_commitments: u64,
    pub scientific_verdict_requested: bool,
    pub terminal_evaluator_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintyDevelopmentRehearsalTerminalRequestV1 {
    pub fn seal(
        experiment_root_sha256: String,
        oracle_batch: K2UncertaintyOracleBaselineBatchReceiptV1,
        controls: Vec<K2UncertaintyControlEvaluationReceiptV1>,
        routes: Vec<K2UncertaintyEvaluationRouteReceiptV1>,
        resources: K2UncertaintyEvaluationResourceMeasurementsV1,
        terminal_evaluator_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_DEVELOPMENT_TERMINAL_REQUEST_SCHEMA_V1.to_owned(),
            experiment_root_sha256,
            oracle_batch,
            controls,
            routes,
            resources,
            sealed_attempts: 0,
            authorization_slots: 0,
            nonce_commitments: 0,
            scientific_verdict_requested: false,
            terminal_evaluator_executable_sha256,
            authority: denied_authority_v1(),
            request_root_sha256: String::new(),
        };
        value.reseal()?;
        Ok(value)
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.request_root_sha256 = self.expected_root()?;
        self.validate_envelope()
    }

    pub fn validate_envelope(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.experiment_root_sha256)?;
        require_composition_root_v1(&self.terminal_evaluator_executable_sha256)?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_DEVELOPMENT_TERMINAL_REQUEST_SCHEMA_V1
            || self.sealed_attempts != 0
            || self.authorization_slots != 0
            || self.nonce_commitments != 0
            || self.scientific_verdict_requested
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_development_terminal_envelope_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_DEVELOPMENT_TERMINAL_REQUEST_SCHEMA_V1,
            &self.experiment_root_sha256,
            &self.oracle_batch,
            &self.controls,
            &self.routes,
            &self.resources,
            self.sealed_attempts,
            self.authorization_slots,
            self.nonce_commitments,
            self.scientific_verdict_requested,
            &self.terminal_evaluator_executable_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintySealedProjectionV1 {
    pub experiment_root_sha256: String,
    pub freeze_root_sha256: String,
    pub attempt_root_sha256: String,
    pub authorization_slot_root_sha256: String,
    pub nonce_commitment_root_sha256: String,
    pub authorization_slot_count: u64,
    pub nonce_commitment_count: u64,
    pub sealed_attempt_count: u64,
    pub projection_root_sha256: String,
}

impl K2UncertaintySealedProjectionV1 {
    pub fn seal(
        experiment_root_sha256: String,
        freeze_root_sha256: String,
        attempt_root_sha256: String,
        authorization_slot_root_sha256: String,
        nonce_commitment_root_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            experiment_root_sha256,
            freeze_root_sha256,
            attempt_root_sha256,
            authorization_slot_root_sha256,
            nonce_commitment_root_sha256,
            authorization_slot_count: 1,
            nonce_commitment_count: 1,
            sealed_attempt_count: 1,
            projection_root_sha256: String::new(),
        };
        value.projection_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_root_sha256,
            &self.freeze_root_sha256,
            &self.attempt_root_sha256,
            &self.authorization_slot_root_sha256,
            &self.nonce_commitment_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        if self.authorization_slot_count != 1
            || self.nonce_commitment_count != 1
            || self.sealed_attempt_count != 1
            || self.projection_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_sealed_projection_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            "nando.k2-self-formed-sealed-projection.v1",
            &self.experiment_root_sha256,
            &self.freeze_root_sha256,
            &self.attempt_root_sha256,
            &self.authorization_slot_root_sha256,
            &self.nonce_commitment_root_sha256,
            self.authorization_slot_count,
            self.nonce_commitment_count,
            self.sealed_attempt_count,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintySealedTerminalRequestV1 {
    pub schema: String,
    pub sealed_projection: K2UncertaintySealedProjectionV1,
    pub oracle_batch: K2UncertaintyOracleBaselineBatchReceiptV1,
    pub controls: Vec<K2UncertaintyControlEvaluationReceiptV1>,
    pub routes: Vec<K2UncertaintyEvaluationRouteReceiptV1>,
    pub resources: K2UncertaintyEvaluationResourceMeasurementsV1,
    pub irreversible_dispatch_missing_results: u64,
    pub ambiguous_results: u64,
    pub terminal_evaluator_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintySealedTerminalRequestV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        sealed_projection: K2UncertaintySealedProjectionV1,
        oracle_batch: K2UncertaintyOracleBaselineBatchReceiptV1,
        controls: Vec<K2UncertaintyControlEvaluationReceiptV1>,
        routes: Vec<K2UncertaintyEvaluationRouteReceiptV1>,
        resources: K2UncertaintyEvaluationResourceMeasurementsV1,
        irreversible_dispatch_missing_results: u64,
        ambiguous_results: u64,
        terminal_evaluator_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_SEALED_TERMINAL_REQUEST_SCHEMA_V1.to_owned(),
            sealed_projection,
            oracle_batch,
            controls,
            routes,
            resources,
            irreversible_dispatch_missing_results,
            ambiguous_results,
            terminal_evaluator_executable_sha256,
            authority: denied_authority_v1(),
            request_root_sha256: String::new(),
        };
        value.reseal()?;
        Ok(value)
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.request_root_sha256 = self.expected_root()?;
        self.validate_envelope()
    }

    pub fn validate_envelope(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.terminal_evaluator_executable_sha256)?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_SEALED_TERMINAL_REQUEST_SCHEMA_V1
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_sealed_terminal_envelope_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_SEALED_TERMINAL_REQUEST_SCHEMA_V1,
            &self.sealed_projection,
            &self.oracle_batch,
            &self.controls,
            &self.routes,
            &self.resources,
            self.irreversible_dispatch_missing_results,
            self.ambiguous_results,
            &self.terminal_evaluator_executable_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyTerminalModeV1 {
    DevelopmentRehearsal,
    SealedAttempt,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum K2UncertaintyTerminalDispositionV1 {
    DevelopmentRehearsalPass,
    K2SelfFormedUncertaintyCapabilityPass,
    ScientificFail,
    InfrastructureFail,
    Indeterminate,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyTerminalEvaluationReceiptV1 {
    pub schema: String,
    pub mode: K2UncertaintyTerminalModeV1,
    pub request_root_sha256: String,
    pub disposition: K2UncertaintyTerminalDispositionV1,
    pub reason: String,
    pub evaluator_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyTerminalEvaluationReceiptV1 {
    pub(crate) fn seal(
        mode: K2UncertaintyTerminalModeV1,
        request_root_sha256: String,
        disposition: K2UncertaintyTerminalDispositionV1,
        reason: String,
        evaluator_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let authority = denied_authority_v1();
        let receipt_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_TERMINAL_RECEIPT_SCHEMA_V1,
            mode,
            &request_root_sha256,
            disposition,
            &reason,
            &evaluator_executable_sha256,
            &authority,
        ))?;
        let value = Self {
            schema: K2_UNCERTAINTY_TERMINAL_RECEIPT_SCHEMA_V1.to_owned(),
            mode,
            request_root_sha256,
            disposition,
            reason,
            evaluator_executable_sha256,
            authority,
            receipt_root_sha256,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.request_root_sha256)?;
        require_composition_root_v1(&self.evaluator_executable_sha256)?;
        require_denied_authority_v1(&self.authority)?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_TERMINAL_RECEIPT_SCHEMA_V1,
            self.mode,
            &self.request_root_sha256,
            self.disposition,
            &self.reason,
            &self.evaluator_executable_sha256,
            &self.authority,
        ))?;
        if self.schema != K2_UNCERTAINTY_TERMINAL_RECEIPT_SCHEMA_V1
            || self.reason.is_empty()
            || self.receipt_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_terminal_receipt_invalid",
            ));
        }
        Ok(())
    }
}

pub(crate) fn validate_terminal_routes_v1(
    routes: &[K2UncertaintyEvaluationRouteReceiptV1],
) -> K2CompositionResultV1<()> {
    if routes.len() != K2_UNCERTAINTY_TERMINAL_ROUTE_IDS_V1.len() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_terminal_route_denominator_invalid",
        ));
    }
    let mut ids = BTreeSet::new();
    for route in routes {
        route.validate()?;
        ids.insert(route.route_id.as_str());
    }
    if ids != K2_UNCERTAINTY_TERMINAL_ROUTE_IDS_V1.into_iter().collect() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_terminal_route_set_invalid",
        ));
    }
    Ok(())
}
