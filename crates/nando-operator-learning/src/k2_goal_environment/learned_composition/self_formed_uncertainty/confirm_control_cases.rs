use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    require_composition_root_v1,
};
use super::confirm_control_canary::{K2UncertaintyCanarySurfaceV1, scan_self_formed_r7k_canary_v1};
use super::{
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2_UNCERTAINTY_R7K_CONTROL_CASE_REQUEST_SCHEMA_V1,
    K2UncertaintyAuthorizationSlotLedgerV1, K2UncertaintyConfirmAttemptDescriptorV1,
    K2UncertaintyConfirmFinalVerifierRequestV1, K2UncertaintyConfirmGeneratorRequestV1,
    K2UncertaintyConfirmOwnerRequestV1, K2UncertaintyControlStdoutV1,
    K2UncertaintyDevelopmentRehearsalTerminalRequestV1,
    K2UncertaintyEvaluationResourceMeasurementsV1, K2UncertaintyEvaluationRouteReceiptV1,
    K2UncertaintyObservationVectorV2, K2UncertaintyOracleBaselineBatchReceiptV1,
    K2UncertaintyOracleBaselineCaseDescriptorV1, K2UncertaintyPrivateResolverRequestV1,
    K2UncertaintyR10AuthorizationReceiptV1, K2UncertaintyTerminalDispositionV1,
    denied_authority_v1, evaluate_self_formed_development_terminal_v1,
    load_self_formed_oracle_case_evidence_v1, require_denied_authority_v1,
    require_self_formed_public_coordinator_manifest_binding_v1, required_r10_authorization_text_v1,
    run_self_formed_k12_cleanup_control_v1, uncertainty_bytes_v1, uncertainty_decode_v1,
    uncertainty_root_v1,
};

const R7K_SESSION_ID_V1: &str = "019f4904-6810-74d3-9343-e7a29224a2fd";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR7kControlCaseRequestV1 {
    pub schema: String,
    pub control_id: String,
    pub experiment_root_sha256: String,
    pub freeze_root_sha256: String,
    pub scratch_root: String,
    pub subcase_roots_sha256: Vec<String>,
    pub target_source_root_sha256: String,
    pub adapter_source_root_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintyR7kControlCaseRequestV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_root_sha256,
            &self.freeze_root_sha256,
            &self.target_source_root_sha256,
            &self.adapter_source_root_sha256,
        ]
        .into_iter()
        .chain(self.subcase_roots_sha256.iter())
        {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_R7K_CONTROL_CASE_REQUEST_SCHEMA_V1
            || !matches!(
                self.control_id.as_str(),
                "K1" | "K2"
                    | "K3"
                    | "K4"
                    | "K5"
                    | "K6"
                    | "K7"
                    | "K8"
                    | "K9"
                    | "K10"
                    | "K11"
                    | "K12"
            )
            || self.scratch_root.is_empty()
            || self.subcase_roots_sha256.len() != expected_subcase_count_v1(&self.control_id)?
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_r7k_control_case_request_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_R7K_CONTROL_CASE_REQUEST_SCHEMA_V1,
            &self.control_id,
            &self.experiment_root_sha256,
            &self.freeze_root_sha256,
            &self.scratch_root,
            &self.subcase_roots_sha256,
            &self.target_source_root_sha256,
            &self.adapter_source_root_sha256,
            &self.authority,
        ))
    }
}

pub fn evaluate_self_formed_r7k_control_case_v1(
    request: &K2UncertaintyR7kControlCaseRequestV1,
) -> K2CompositionResultV1<K2UncertaintyControlStdoutV1> {
    request.validate()?;
    let disposition = match request.control_id.as_str() {
        "K1" => control_k1_v1(request)?,
        "K2" => control_k2_v1(request)?,
        "K3" => control_k3_v1(request)?,
        "K4" => control_k4_v1(request)?,
        "K5" => control_k5_v1(request)?,
        "K6" => control_k6_v1(request)?,
        "K7" => control_k7_v1(request)?,
        "K8" => control_k8_v1(request)?,
        "K9" => control_k9_v1(request)?,
        "K10" => control_k10_v1(request)?,
        "K11" => control_k11_v1(request)?,
        "K12" => control_k12_v1(request)?,
        _ => {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_r7k_control_case_not_implemented",
            ));
        }
    };
    Ok(K2UncertaintyControlStdoutV1 {
        control_id: request.control_id.clone(),
        disposition: disposition.to_owned(),
    })
}

pub fn run_self_formed_r7k_control_case_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_r7k_control_case_stdin"))?;
    let request: K2UncertaintyR7kControlCaseRequestV1 = uncertainty_decode_v1(&input)?;
    let stdout = evaluate_self_formed_r7k_control_case_v1(&request)?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&stdout)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_r7k_control_case_stdout"))
}

pub fn new_self_formed_r7k_control_case_request_v1(
    control_id: String,
    experiment_root_sha256: String,
    freeze_root_sha256: String,
    scratch_root: String,
    subcase_roots_sha256: Vec<String>,
    target_source_root_sha256: String,
    adapter_source_root_sha256: String,
) -> K2CompositionResultV1<K2UncertaintyR7kControlCaseRequestV1> {
    let mut request = K2UncertaintyR7kControlCaseRequestV1 {
        schema: K2_UNCERTAINTY_R7K_CONTROL_CASE_REQUEST_SCHEMA_V1.to_owned(),
        control_id,
        experiment_root_sha256,
        freeze_root_sha256,
        scratch_root,
        subcase_roots_sha256,
        target_source_root_sha256,
        adapter_source_root_sha256,
        authority: denied_authority_v1(),
        request_root_sha256: String::new(),
    };
    request.request_root_sha256 = request.expected_root()?;
    request.validate()?;
    Ok(request)
}

fn control_k1_v1(
    request: &K2UncertaintyR7kControlCaseRequestV1,
) -> K2CompositionResultV1<&'static str> {
    let development_seed = fs::read(Path::new(&request.scratch_root).join("development-seed.bin"))
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_r7k_development_seed"))?;
    let error = require_rejection_v1(
        K2UncertaintyConfirmGeneratorRequestV1::seal(
            development_seed,
            request.freeze_root_sha256.clone(),
            root_v1("k1-authorization")?,
            root_v1("k1-generator")?,
        ),
        "self_formed_r7k_k1_unexpected_accept",
    )?;
    require_error_v1(
        &error,
        "k2_composition_invalid:self_formed_confirm_generator_request_invalid",
    )?;
    Ok("reused_development_commitment_rejected")
}

fn control_k2_v1(
    request: &K2UncertaintyR7kControlCaseRequestV1,
) -> K2CompositionResultV1<&'static str> {
    let scratch = Path::new(&request.scratch_root);
    let authorization = authorization_v1("k2-primary", "2026-08-20T12:00:00+03:00")?;
    let claim = K2UncertaintyAuthorizationSlotLedgerV1::open_or_create(&scratch.join("k2-ledger"))?
        .claim(&authorization, root_v1("k2-slot-owner")?)?;
    let owner_sha256 = root_v1("k2-confirm-owner")?;
    let generator_sha256 = root_v1("k2-generator")?;
    let descriptor = K2UncertaintyConfirmAttemptDescriptorV1::confirm(
        authorization.experiment_id_sha256.clone(),
        authorization.successor_freeze_root_sha256.clone(),
        authorization.executable_manifest_root_sha256.clone(),
        owner_sha256,
        generator_sha256,
        &authorization,
        &claim,
    )?;
    let mut valid = K2UncertaintyConfirmOwnerRequestV1::confirm(
        descriptor,
        scratch.to_string_lossy().into_owned(),
        "attempt".to_owned(),
        "k2-ledger".to_owned(),
        std::env::current_exe()
            .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_r7k_control_adapter"))?
            .to_string_lossy()
            .into_owned(),
        authorization,
        claim,
    )?;
    let mut missing = valid.clone();
    missing.authorization_receipt = None;
    missing.reseal_envelope_for_r7k_control_v1()?;
    let missing_error = require_rejection_v1(
        missing.validate(),
        "self_formed_r7k_k2_missing_authorization_unexpected_accept",
    )?;
    require_error_v1(
        &missing_error,
        "k2_composition_invalid:self_formed_confirm_owner_authorization_missing",
    )?;

    let foreign = authorization_v1("k2-foreign", "2026-08-20T12:01:00+03:00")?;
    valid.authorization_receipt = Some(foreign);
    valid.reseal_envelope_for_r7k_control_v1()?;
    let foreign_error = require_rejection_v1(
        valid.validate(),
        "self_formed_r7k_k2_foreign_authorization_unexpected_accept",
    )?;
    require_error_v1(
        &foreign_error,
        "k2_composition_invalid:self_formed_confirm_owner_authority_binding_invalid",
    )?;
    Ok("missing_or_foreign_authorization_rejected")
}

fn control_k8_v1(
    request: &K2UncertaintyR7kControlCaseRequestV1,
) -> K2CompositionResultV1<&'static str> {
    let ledger = K2UncertaintyAuthorizationSlotLedgerV1::open_or_create(
        &Path::new(&request.scratch_root).join("k8-ledger"),
    )?;
    let first = authorization_v1("k8", "2026-08-20T12:02:00+03:00")?;
    let claim = ledger.claim(&first, root_v1("k8-owner-a")?)?;
    let duplicate_receipt = require_rejection_v1(
        ledger.claim(&first, root_v1("k8-owner-b")?),
        "self_formed_r7k_k8_duplicate_receipt_unexpected_accept",
    )?;
    require_error_v1(
        &duplicate_receipt,
        "k2_composition_invalid:self_formed_authorization_receipt_already_used",
    )?;

    let second = authorization_v1("k8", "2026-08-20T12:03:00+03:00")?;
    let duplicate_slot = require_rejection_v1(
        ledger.claim(&second, root_v1("k8-owner-a")?),
        "self_formed_r7k_k8_duplicate_slot_unexpected_accept",
    )?;
    require_error_v1(
        &duplicate_slot,
        "k2_composition_invalid:self_formed_authorization_slot_already_claimed",
    )?;
    let identity_error = require_rejection_v1(
        K2UncertaintyConfirmAttemptDescriptorV1::confirm(
            second.experiment_id_sha256.clone(),
            second.successor_freeze_root_sha256.clone(),
            second.executable_manifest_root_sha256.clone(),
            root_v1("k8-confirm-owner")?,
            root_v1("k8-generator")?,
            &second,
            &claim,
        ),
        "self_formed_r7k_k8_identity_unexpected_accept",
    )?;
    require_error_v1(
        &identity_error,
        "k2_composition_invalid:self_formed_confirm_descriptor_authorization_mismatch",
    )?;
    Ok("duplicate_slot_attempt_or_nonce_rejected")
}

fn control_k7_v1(
    request: &K2UncertaintyR7kControlCaseRequestV1,
) -> K2CompositionResultV1<&'static str> {
    let coordinator = Path::new(&request.scratch_root).join("public-coordinator");
    let missing = require_rejection_v1(
        require_self_formed_public_coordinator_manifest_binding_v1(&coordinator, None),
        "self_formed_r7k_k7_unmanifested_coordinator_unexpected_accept",
    )?;
    require_error_v1(
        &missing,
        "k2_composition_invalid:self_formed_public_coordinator_manifest_entry_missing",
    )?;

    let mismatch = require_rejection_v1(
        require_self_formed_public_coordinator_manifest_binding_v1(
            &coordinator,
            Some(&root_v1("k7-foreign-coordinator")?),
        ),
        "self_formed_r7k_k7_mismatched_coordinator_unexpected_accept",
    )?;
    require_error_v1(
        &mismatch,
        "k2_composition_invalid:self_formed_public_coordinator_executable_mismatch",
    )?;
    Ok("coordinator_manifest_mismatch_rejected")
}

fn control_k3_v1(
    request: &K2UncertaintyR7kControlCaseRequestV1,
) -> K2CompositionResultV1<&'static str> {
    let scratch = Path::new(&request.scratch_root);
    let argv = std::env::args_os()
        .map(|value| std::os::unix::ffi::OsStrExt::as_bytes(value.as_os_str()).to_vec())
        .collect::<Vec<_>>();
    let environment = std::env::vars_os()
        .map(|(key, value)| {
            let mut entry = std::os::unix::ffi::OsStrExt::as_bytes(key.as_os_str()).to_vec();
            entry.push(b'=');
            entry.extend_from_slice(std::os::unix::ffi::OsStrExt::as_bytes(value.as_os_str()));
            entry
        })
        .collect::<Vec<_>>();
    let persisted = fs::read(scratch.join("persisted-generator-request.bin"))
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_r7k_k3_persisted_request"))?;
    let census = scan_self_formed_r7k_canary_v1(
        &argv,
        &environment,
        &[scratch],
        &persisted,
        &scratch.join("public"),
    )?;
    census.require_complete_hits(&[
        K2UncertaintyCanarySurfaceV1::Argv,
        K2UncertaintyCanarySurfaceV1::Environment,
        K2UncertaintyCanarySurfaceV1::PathComponent,
        K2UncertaintyCanarySurfaceV1::PersistedRequest,
    ])?;
    let error = require_rejection_v1(
        census.require_absent("self_formed_r7k_nonce_transport_canary_detected"),
        "self_formed_r7k_k3_nonce_transport_unexpected_accept",
    )?;
    require_error_v1(
        &error,
        "k2_composition_invalid:self_formed_r7k_nonce_transport_canary_detected",
    )?;
    Ok("nonce_transport_rejected")
}

fn control_k4_v1(
    request: &K2UncertaintyR7kControlCaseRequestV1,
) -> K2CompositionResultV1<&'static str> {
    let scratch = Path::new(&request.scratch_root);
    let census = scan_self_formed_r7k_canary_v1(&[], &[], &[], &[], &scratch.join("public"))?;
    census.require_complete_hits(&[K2UncertaintyCanarySurfaceV1::PublicFile])?;
    let error = require_rejection_v1(
        census.require_absent("self_formed_r7k_private_public_canary_detected"),
        "self_formed_r7k_k4_private_public_unexpected_accept",
    )?;
    require_error_v1(
        &error,
        "k2_composition_invalid:self_formed_r7k_private_public_canary_detected",
    )?;
    Ok("private_public_leakage_rejected")
}

fn control_k5_v1(
    request: &K2UncertaintyR7kControlCaseRequestV1,
) -> K2CompositionResultV1<&'static str> {
    let valid: K2UncertaintyPrivateResolverRequestV1 =
        decode_fixture_v1(request, "fixture-packet/resolver-request.json")?;
    let error = require_rejection_v1(
        K2UncertaintyPrivateResolverRequestV1::seal(
            valid.experiment_id_sha256,
            valid.public_batch_root_sha256,
            String::new(),
            valid.case_preverification_root_sha256,
            valid.public_case_root_sha256,
            valid.closure_plan,
            valid.probe_ordinal,
            valid.selected_probe,
            valid.resolver_table_root_sha256,
            valid.resolver_executable_sha256,
        ),
        "self_formed_r7k_k5_early_resolver_unexpected_accept",
    )?;
    require_error_v1(&error, "k2_composition_invalid:root_invalid")?;
    require_path_absent_v1(
        Path::new("/private/resolver.json"),
        "self_formed_r7k_k5_forbidden_resolver_mount_present",
    )?;
    Ok("early_private_resolver_rejected")
}

fn control_k6_v1(
    request: &K2UncertaintyR7kControlCaseRequestV1,
) -> K2CompositionResultV1<&'static str> {
    let valid: K2UncertaintyConfirmFinalVerifierRequestV1 =
        decode_fixture_v1(request, "fixture-packet/final-request.json")?;
    let mut executions = valid.observation_vector.executions.clone();
    if executions.pop().is_none() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_r7k_k6_fixture_vector_empty",
        ));
    }
    let error = require_rejection_v1(
        K2UncertaintyObservationVectorV2::seal(&valid.dispatch, executions),
        "self_formed_r7k_k6_partial_vector_unexpected_accept",
    )?;
    require_error_v1(
        &error,
        "k2_composition_invalid:self_formed_observation_vector_dispatch_v2_invalid",
    )?;
    require_path_absent_v1(
        Path::new("/private/final-truth.json"),
        "self_formed_r7k_k6_forbidden_final_truth_mount_present",
    )?;
    Ok("early_final_truth_rejected")
}

fn control_k9_v1(
    request: &K2UncertaintyR7kControlCaseRequestV1,
) -> K2CompositionResultV1<&'static str> {
    let oracle_batch: K2UncertaintyOracleBaselineBatchReceiptV1 =
        decode_fixture_v1(request, "fixture-packet/oracle-batch.json")?;
    let mut omitted_case = oracle_batch.clone();
    if omitted_case.case_receipts.pop().is_none() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_r7k_k9_fixture_batch_empty",
        ));
    }
    let count_error = require_rejection_v1(
        omitted_case.validate(),
        "self_formed_r7k_k9_partial_batch_unexpected_accept",
    )?;
    require_error_v1(
        &count_error,
        "k2_composition_invalid:self_formed_oracle_batch_case_count_invalid",
    )?;

    let routes: Vec<K2UncertaintyEvaluationRouteReceiptV1> =
        decode_fixture_v1(request, "fixture-packet/routes.json")?;
    let resources: K2UncertaintyEvaluationResourceMeasurementsV1 =
        decode_fixture_v1(request, "fixture-packet/resources.json")?;
    let terminal_request = K2UncertaintyDevelopmentRehearsalTerminalRequestV1::seal(
        request.experiment_root_sha256.clone(),
        oracle_batch,
        Vec::new(),
        routes,
        resources,
        root_v1("k9-terminal-evaluator")?,
    )?;
    let terminal = evaluate_self_formed_development_terminal_v1(&terminal_request)?;
    if terminal.disposition != K2UncertaintyTerminalDispositionV1::InfrastructureFail
        || terminal.reason != "development_evidence_invalid"
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_r7k_k9_terminal_conjunct_unexpected_accept",
        ));
    }
    Ok("partial_terminal_denominator_rejected")
}

fn control_k10_v1(
    request: &K2UncertaintyR7kControlCaseRequestV1,
) -> K2CompositionResultV1<&'static str> {
    let one_probe: K2UncertaintyOracleBaselineCaseDescriptorV1 =
        decode_fixture_v1(request, "fixture-packet/one-probe-descriptor.json")?;
    let mut two_probe: K2UncertaintyOracleBaselineCaseDescriptorV1 =
        decode_fixture_v1(request, "fixture-packet/two-probe-descriptor.json")?;
    two_probe.closure_plan_root_sha256 = one_probe.closure_plan_root_sha256;
    let error = require_rejection_v1(
        load_self_formed_oracle_case_evidence_v1(
            &Path::new(&request.scratch_root).join("oracle-case-00"),
            &two_probe,
        ),
        "self_formed_r7k_k10_one_probe_substitution_unexpected_accept",
    )?;
    require_error_v1(
        &error,
        "k2_composition_invalid:self_formed_oracle_descriptor_evidence_binding_invalid",
    )?;
    Ok("one_probe_oracle_substitution_rejected")
}

fn control_k11_v1(
    request: &K2UncertaintyR7kControlCaseRequestV1,
) -> K2CompositionResultV1<&'static str> {
    let oracle_batch: K2UncertaintyOracleBaselineBatchReceiptV1 =
        decode_fixture_v1(request, "fixture-packet/oracle-batch.json")?;
    let mut omitted_case = oracle_batch.clone();
    if omitted_case.case_receipts.pop().is_none() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_r7k_k11_fixture_batch_empty",
        ));
    }
    let case_count_error = require_rejection_v1(
        omitted_case.validate(),
        "self_formed_r7k_k11_partial_batch_unexpected_accept",
    )?;
    require_error_v1(
        &case_count_error,
        "k2_composition_invalid:self_formed_oracle_batch_case_count_invalid",
    )?;

    let mut omitted_baseline =
        oracle_batch
            .case_receipts
            .first()
            .cloned()
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_r7k_k11_fixture_case_missing",
            ))?;
    if omitted_baseline.baselines.pop().is_none() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_r7k_k11_fixture_baseline_empty",
        ));
    }
    let baseline_error = require_rejection_v1(
        omitted_baseline.validate(),
        "self_formed_r7k_k11_partial_baseline_unexpected_accept",
    )?;
    require_error_v1(
        &baseline_error,
        "k2_composition_invalid:self_formed_oracle_case_baseline_count_invalid",
    )?;
    Ok("baseline_denominator_omission_rejected")
}

fn control_k12_v1(
    request: &K2UncertaintyR7kControlCaseRequestV1,
) -> K2CompositionResultV1<&'static str> {
    run_self_formed_k12_cleanup_control_v1(
        &Path::new(&request.scratch_root).join("k12"),
        request.experiment_root_sha256.clone(),
        root_v1("k12-cleanup-owner")?,
    )?;
    Ok("cleanup_retention_or_residue_violation_rejected")
}

fn authorization_v1(
    label: &str,
    authorized_at: &str,
) -> K2CompositionResultV1<K2UncertaintyR10AuthorizationReceiptV1> {
    let successor = root_v1(&format!("successor-{label}"))?;
    K2UncertaintyR10AuthorizationReceiptV1::seal(
        required_r10_authorization_text_v1(&successor)?,
        R7K_SESSION_ID_V1.to_owned(),
        authorized_at.to_owned(),
        root_v1(&format!("experiment-{label}"))?,
        successor,
        root_v1(&format!("manifest-{label}"))?,
    )
}

fn require_error_v1(error: &K2CompositionErrorV1, expected: &str) -> K2CompositionResultV1<()> {
    if error.to_string() != expected {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_r7k_control_typed_error_mismatch",
        ));
    }
    Ok(())
}

fn require_rejection_v1<T>(
    result: K2CompositionResultV1<T>,
    unexpected_accept: &'static str,
) -> K2CompositionResultV1<K2CompositionErrorV1> {
    match result {
        Ok(_) => Err(K2CompositionErrorV1::Invalid(unexpected_accept)),
        Err(error) => Ok(error),
    }
}

fn decode_fixture_v1<T: DeserializeOwned + Serialize>(
    request: &K2UncertaintyR7kControlCaseRequestV1,
    relative_path: &str,
) -> K2CompositionResultV1<T> {
    let bytes = fs::read(Path::new(&request.scratch_root).join(relative_path))
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_r7k_control_fixture"))?;
    uncertainty_decode_v1(&bytes)
}

fn require_path_absent_v1(path: &Path, present_error: &'static str) -> K2CompositionResultV1<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(K2CompositionErrorV1::Invalid(present_error)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(K2CompositionErrorV1::Io(
            "stat_self_formed_r7k_forbidden_mount",
        )),
    }
}

fn expected_subcase_count_v1(control_id: &str) -> K2CompositionResultV1<usize> {
    match control_id {
        "K1" | "K4" | "K10" => Ok(1),
        "K2" | "K5" | "K6" | "K7" | "K8" | "K9" | "K11" | "K12" => Ok(2),
        "K3" => Ok(4),
        _ => Err(K2CompositionErrorV1::Invalid(
            "self_formed_r7k_control_id_invalid",
        )),
    }
}

fn root_v1(label: &str) -> K2CompositionResultV1<String> {
    uncertainty_root_v1(&("nando.k2-self-formed-r7k-control.v1", label))
}
