use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;

use serde::{Serialize, de::DeserializeOwned};

use super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, K2InquiryBaselineRequestV1, K2InquiryBaselinesV1,
    K2InquirySelectionPrecommitV1, K2InquiryVerifierCommandV1, K2InquiryVerifierReceiptV1,
    composition_sha256_bytes_v1, composition_sha256_file_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2_UNCERTAINTY_R8B_PRODUCER_REQUEST_ENV_V3,
    K2_UNCERTAINTY_R8B_STDOUT_RECEIPT_PATH_V2, K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1,
    K2UncertaintyBatchPrecommitV2, K2UncertaintyCasePreverificationV2,
    K2UncertaintyClosureCensusV1, K2UncertaintyClosurePlanV1, K2UncertaintyClosurePlannerRequestV1,
    K2UncertaintyClosureVerificationReceiptV1, K2UncertaintyClosureVerificationRequestV1,
    K2UncertaintyConfirmDataMountV1, K2UncertaintyConfirmGuestExecutableV1,
    K2UncertaintyConfirmMountTargetV1, K2UncertaintyLearnerRequestV1,
    K2UncertaintyLearnerResponseV1, K2UncertaintyProbeArtifactsV1, K2UncertaintyProbeRequestV1,
    K2UncertaintyPublicCoordinatorRequestV1, K2UncertaintyPublicOwnerRoleV1,
    K2UncertaintyPublicOwnerV1, K2UncertaintyPublicPrecommitReceiptV1,
    K2UncertaintyPublicPreparedCaseV1, K2UncertaintyR8BExecutableIdentityV2,
    K2UncertaintyR8BExpectedOutcomeV3, K2UncertaintyR8BInvocationPlanV3,
    K2UncertaintyR8BLaunchKindV3, K2UncertaintyR8BLedgerWriterV2, K2UncertaintyR8BLedgerWriterV3,
    K2UncertaintyR8BProcessEventV3, K2UncertaintyR8BProducedReceiptV2,
    K2UncertaintyR8BProducerRequestV3, K2UncertaintyR8BToolIdentityV3, K2UncertaintyR8BToolRoleV3,
    K2UncertaintyR8BValidatedFactV3, K2UncertaintyR8BValidatorV3, denied_authority_v1,
    preverify_self_formed_case_with_owner_v1, publish_self_formed_final_verifier_material_v2,
    publish_self_formed_public_case_v1, publish_self_formed_public_precommit_v1,
    reopen_self_formed_probe_output_v1, run_self_formed_confirm_sandbox_measured_v1,
    run_self_formed_tournament_with_owners_v1, uncertainty_bytes_v1, uncertainty_decode_v1,
    uncertainty_root_v1, validate_self_formed_r8b_producer_request_v3,
};

pub fn execute_self_formed_public_coordinator_v1(
    request: &K2UncertaintyPublicCoordinatorRequestV1,
    output_root: &Path,
) -> K2CompositionResultV1<K2UncertaintyPublicPrecommitReceiptV1> {
    request.validate()?;
    prepare_empty_output_root_v1(output_root)?;
    let learner = request
        .owner_set
        .owner(K2UncertaintyPublicOwnerRoleV1::Learner)?;
    let probe = request
        .owner_set
        .owner(K2UncertaintyPublicOwnerRoleV1::Probe)?;
    let selector = request
        .owner_set
        .owner(K2UncertaintyPublicOwnerRoleV1::Selector)?;
    let baseline = request
        .owner_set
        .owner(K2UncertaintyPublicOwnerRoleV1::Baseline)?;
    let selection_preverifier = request
        .owner_set
        .owner(K2UncertaintyPublicOwnerRoleV1::SelectionPreverifier)?;
    let closure_planner = request
        .owner_set
        .owner(K2UncertaintyPublicOwnerRoleV1::ClosurePlanner)?;
    let closure_verifier = request
        .owner_set
        .owner(K2UncertaintyPublicOwnerRoleV1::ClosureVerifier)?;

    let mut prepared = Vec::with_capacity(request.public_batch.cases.len());
    let mut artifacts = Vec::with_capacity(request.public_batch.cases.len());
    for (case_sequence, public_case) in request.public_batch.cases.iter().enumerate() {
        let learned: K2UncertaintyLearnerResponseV1 = invoke_owner_v1(
            K2UncertaintyConfirmGuestExecutableV1::Learner,
            learner,
            &public_case.vocabulary.case_id_sha256,
            &[],
            &K2UncertaintyLearnerRequestV1::seal(
                public_case.vocabulary.clone(),
                public_case.support.clone(),
                learner.executable_sha256.clone(),
            )?,
            60,
        )?;
        let probe_request = K2UncertaintyProbeRequestV1::seal(
            public_case.clone(),
            learned,
            request.public_batch.split_commitment_root_sha256.clone(),
            probe.executable_sha256.clone(),
        )?;
        let probe_root = output_root.join(format!("probes/case-{case_sequence:02}"));
        fs::create_dir_all(&probe_root)
            .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_public_probe_root"))?;
        fs::set_permissions(&probe_root, fs::Permissions::from_mode(0o700))
            .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_public_probe_root"))?;
        let probe_mount = [K2UncertaintyConfirmDataMountV1 {
            host_path: &probe_root,
            target: K2UncertaintyConfirmMountTargetV1::Output,
            writable: true,
        }];
        let probe_artifacts: K2UncertaintyProbeArtifactsV1 = invoke_owner_v1(
            K2UncertaintyConfirmGuestExecutableV1::Probe,
            probe,
            &public_case.vocabulary.case_id_sha256,
            &probe_mount,
            &probe_request,
            120,
        )?;
        let probe_output = reopen_self_formed_probe_output_v1(&probe_root, &probe_artifacts)?;
        let tournament = run_self_formed_tournament_with_owners_v1(
            public_case,
            &probe_request.learner_response,
            &probe_output,
            &request.public_batch.split_commitment_root_sha256,
            K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1,
            &selector.executable_sha256,
            &baseline.executable_sha256,
            &mut |selector_request| {
                invoke_owner_v1(
                    K2UncertaintyConfirmGuestExecutableV1::Selector,
                    selector,
                    &public_case.vocabulary.case_id_sha256,
                    &[],
                    selector_request,
                    30,
                )
            },
            &mut |baseline_request: &K2InquiryBaselineRequestV1| {
                invoke_owner_v1::<_, K2InquiryBaselinesV1>(
                    K2UncertaintyConfirmGuestExecutableV1::Baseline,
                    baseline,
                    &public_case.vocabulary.case_id_sha256,
                    &[],
                    baseline_request,
                    30,
                )
            },
        )?;
        let selection_preverification = preverify_self_formed_case_with_owner_v1(
            &tournament,
            &probe_artifacts,
            &baseline.executable_sha256,
            &selection_preverifier.executable_sha256,
            &mut |command: &K2InquiryVerifierCommandV1| {
                invoke_owner_v1::<_, K2InquiryVerifierReceiptV1>(
                    K2UncertaintyConfirmGuestExecutableV1::SelectionPreverifier,
                    selection_preverifier,
                    &public_case.vocabulary.case_id_sha256,
                    &[],
                    command,
                    30,
                )
            },
        )?;
        let representative_roots = probe_output
            .frontier
            .representative_probe_roots_sha256
            .iter()
            .collect::<BTreeSet<_>>();
        let representatives = probe_output
            .pages
            .iter()
            .flat_map(|page| &page.dispositions)
            .filter(|disposition| {
                representative_roots.contains(&disposition.probe.probe_root_sha256)
            })
            .cloned()
            .collect::<Vec<_>>();
        let planner_request = K2UncertaintyClosurePlannerRequestV1::seal(
            public_case.vocabulary.case_id_sha256.clone(),
            probe_output.frontier.frontier_root_sha256.clone(),
            selection_preverification
                .tournament
                .tournament_root_sha256
                .clone(),
            selection_preverification
                .tournament
                .tournament_winner_probe_root_sha256
                .clone(),
            representatives,
            closure_planner.executable_sha256.clone(),
        )?;
        let closure_census: K2UncertaintyClosureCensusV1 = invoke_owner_v1(
            K2UncertaintyConfirmGuestExecutableV1::ClosurePlanner,
            closure_planner,
            &public_case.vocabulary.case_id_sha256,
            &[],
            &planner_request,
            60,
        )?;
        let closure_request = K2UncertaintyClosureVerificationRequestV1::seal(
            closure_verifier.executable_sha256.clone(),
            planner_request,
            closure_census.clone(),
        )?;
        let closure_receipt: K2UncertaintyClosureVerificationReceiptV1 = invoke_owner_v1(
            K2UncertaintyConfirmGuestExecutableV1::ClosureVerifier,
            closure_verifier,
            &public_case.vocabulary.case_id_sha256,
            &[],
            &closure_request,
            60,
        )?;
        let closure_plan = K2UncertaintyClosurePlanV1::seal(
            &closure_request.planner_request,
            &closure_census,
            &closure_receipt,
        )?;
        let preverification = K2UncertaintyCasePreverificationV2::seal(
            selection_preverification.clone(),
            closure_request,
            closure_receipt,
            Some(closure_plan),
        )?;
        let value = K2UncertaintyPublicPreparedCaseV1::seal(
            case_sequence as u64,
            probe_request,
            probe_artifacts,
            selection_preverification,
            preverification,
        )?;
        artifacts.push(publish_self_formed_public_case_v1(output_root, &value)?);
        prepared.push(value);
    }

    let batch = K2UncertaintyBatchPrecommitV2::seal(
        request.public_batch.experiment_id_sha256.clone(),
        request
            .public_denominator
            .expected_denominator_commitment_sha256
            .clone(),
        &prepared
            .iter()
            .map(|case| case.preverification.clone())
            .collect::<Vec<_>>(),
        request
            .public_batch
            .cases
            .iter()
            .map(|case| case.vocabulary.case_id_sha256.clone())
            .collect(),
    )?;
    for case in &prepared {
        let probe_root = output_root.join(format!("probes/case-{:02}", case.case_sequence));
        publish_self_formed_final_verifier_material_v2(&probe_root, &batch, &case.preverification)?;
    }
    let mut receipt = K2UncertaintyPublicPrecommitReceiptV1 {
        schema: super::K2_UNCERTAINTY_PUBLIC_PRECOMMIT_RECEIPT_SCHEMA_V1.to_owned(),
        coordinator_request_root_sha256: request.request_root_sha256.clone(),
        experiment_id_sha256: request.public_batch.experiment_id_sha256.clone(),
        public_batch_root_sha256: request.public_batch.public_batch_root_sha256.clone(),
        public_denominator_root_sha256: request.public_denominator.receipt_root_sha256.clone(),
        owner_set_root_sha256: request.owner_set.owner_set_root_sha256.clone(),
        coordinator_executable_sha256: request.coordinator_executable_sha256.clone(),
        case_artifacts: artifacts,
        batch_precommit: batch,
        public_case_count: request.public_batch.cases.len() as u64,
        private_mount_count: 0,
        all_cases_precommitted: true,
        authority: denied_authority_v1(),
        receipt_root_sha256: String::new(),
    };
    receipt.reseal()?;
    publish_self_formed_public_precommit_v1(output_root, &receipt)?;
    Ok(receipt)
}

pub fn run_self_formed_public_coordinator_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_public_coordinator_stdin"))?;
    let request: K2UncertaintyPublicCoordinatorRequestV1 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_public_coordinator"))?;
    require_self_formed_public_coordinator_manifest_binding_v1(
        &executable,
        Some(&request.coordinator_executable_sha256),
    )?;
    let receipt =
        execute_self_formed_public_coordinator_v1(&request, Path::new(&request.output_root))?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_public_coordinator_stdout"))
}

pub fn require_self_formed_public_coordinator_manifest_binding_v1(
    executable: &Path,
    manifest_sha256: Option<&str>,
) -> K2CompositionResultV1<()> {
    let manifest_sha256 = manifest_sha256.ok_or(K2CompositionErrorV1::Invalid(
        "self_formed_public_coordinator_manifest_entry_missing",
    ))?;
    require_composition_root_v1(manifest_sha256)?;
    if composition_sha256_file_v1(executable)? != manifest_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_public_coordinator_executable_mismatch",
        ));
    }
    Ok(())
}

fn invoke_owner_v1<I, O>(
    role: K2UncertaintyConfirmGuestExecutableV1,
    owner: &K2UncertaintyPublicOwnerV1,
    case_id_sha256: &str,
    mounts: &[K2UncertaintyConfirmDataMountV1<'_>],
    input: &I,
    cpu_seconds: u64,
) -> K2CompositionResultV1<O>
where
    I: Serialize,
    O: DeserializeOwned + Serialize,
{
    let request_root = uncertainty_root_v1(input)?;
    let input = uncertainty_bytes_v1(input)?;
    let (stage_id, child_role) = public_child_identity_v2(role)?;
    let executable = Path::new(&owner.executable_path);
    let metadata = fs::metadata(executable)
        .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_r8b_public_child"))?;
    let current = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_r8b_m10"))?;
    let ledger = K2UncertaintyR8BLedgerWriterV2::from_environment(
        "M10_PUBLIC_COORDINATOR",
        composition_sha256_file_v1(&current)?,
        vec![K2UncertaintyR8BExecutableIdentityV2 {
            role: child_role.to_owned(),
            canonical_path: owner.executable_path.clone(),
            byte_len: metadata.len(),
            unix_mode: metadata.permissions().mode() & 0o7777,
            sha256: owner.executable_sha256.clone(),
        }],
    )?;
    let stdin_sha256 = composition_sha256_bytes_v1(&input);
    let started_v3 = start_dynamic_invocation_v3(
        stage_id,
        child_role,
        case_id_sha256,
        &owner.executable_sha256,
        &request_root,
        &stdin_sha256,
    )?;
    let started = ledger
        .as_ref()
        .map(|writer| {
            writer.child_started(
                stage_id,
                None,
                None,
                child_role,
                executable,
                request_root.clone(),
                stdin_sha256,
                monotonic_ns_v2(),
            )
        })
        .transpose()?;
    let outcome = run_self_formed_confirm_sandbox_measured_v1(
        role,
        executable,
        &owner.executable_sha256,
        mounts,
        &input,
        cpu_seconds,
    )?;
    if !outcome.normal_exit || outcome.exit_code != 0 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_sandbox_child_failed",
        ));
    }
    let (receipt_schema, semantic_root) = validate_public_child_output_v2(role, &outcome.stdout)?;
    let fact = validated_public_fact_v3(role, mounts, &outcome.stdout)?;
    let value = uncertainty_decode_v1(&outcome.stdout)?;
    if let Some((writer, started)) = started_v3 {
        writer.success(
            &started,
            &outcome.stdout,
            &outcome.stderr,
            receipt_schema.clone(),
            semantic_root.clone(),
            fact,
            Vec::new(),
            monotonic_ns_v2(),
        )?;
    }
    if let (Some(writer), Some(started)) = (&ledger, &started) {
        writer.child_finished(
            started,
            &outcome.stdout,
            &outcome.stderr,
            vec![K2UncertaintyR8BProducedReceiptV2 {
                relative_path: K2_UNCERTAINTY_R8B_STDOUT_RECEIPT_PATH_V2.to_owned(),
                byte_len: outcome.stdout.len() as u64,
                unix_mode: 0,
                content_sha256: composition_sha256_bytes_v1(&outcome.stdout),
                receipt_schema,
                semantic_root_sha256: semantic_root,
            }],
            monotonic_ns_v2(),
        )?;
    }
    Ok(value)
}

#[allow(clippy::too_many_arguments)]
fn start_dynamic_invocation_v3(
    stage: &str,
    target_role: &str,
    case_id_sha256: &str,
    target_sha256: &str,
    request_root_sha256: &str,
    stdin_sha256: &str,
) -> K2CompositionResultV1<
    Option<(
        K2UncertaintyR8BLedgerWriterV3,
        K2UncertaintyR8BProcessEventV3,
    )>,
> {
    let Some(path) = std::env::var_os(K2_UNCERTAINTY_R8B_PRODUCER_REQUEST_ENV_V3) else {
        return Ok(None);
    };
    let bytes = fs::read(path)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_r8b_v3_m10_request"))?;
    let request: K2UncertaintyR8BProducerRequestV3 = uncertainty_decode_v1(&bytes)?;
    validate_self_formed_r8b_producer_request_v3(&request)?;
    if uncertainty_bytes_v1(&request)? != bytes {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_r8b_v3_m10_request_bytes_invalid",
        ));
    }
    let writer = K2UncertaintyR8BLedgerWriterV3::attach_request(&request)?;
    let summary = writer.summary()?;
    let ordinal = summary
        .invocations
        .iter()
        .filter(|row| {
            row.request_owner_role == "M10_PUBLIC_COORDINATOR"
                && row.target_role == target_role
                && row.case_id_sha256.as_deref() == Some(case_id_sha256)
        })
        .count() as u64;
    let owner_sha256 = composition_sha256_file_v1(
        &std::env::current_exe()
            .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_r8b_m10"))?,
    )?;
    let invocation = K2UncertaintyR8BInvocationPlanV3 {
        invocation_id_sha256: uncertainty_root_v1(&(
            "nando.r8b.m10-invocation.v3",
            &request.route_id_sha256,
            case_id_sha256,
            target_role,
            ordinal,
            request_root_sha256,
        ))?,
        parent_invocation_id_sha256: None,
        request_owner_role: "M10_PUBLIC_COORDINATOR".to_owned(),
        request_owner_executable_sha256: owner_sha256,
        target_role: target_role.to_owned(),
        target_executable_sha256: target_sha256.to_owned(),
        launch_kind: K2UncertaintyR8BLaunchKindV3::BwrapPrlimitMediated,
        tool_chain: [
            (K2UncertaintyR8BToolRoleV3::Bwrap, "/usr/bin/bwrap"),
            (K2UncertaintyR8BToolRoleV3::Prlimit, "/usr/bin/prlimit"),
        ]
        .into_iter()
        .map(|(role, path)| {
            Ok(K2UncertaintyR8BToolIdentityV3 {
                role,
                canonical_path: path.to_owned(),
                sha256: composition_sha256_file_v1(Path::new(path))?,
            })
        })
        .collect::<K2CompositionResultV1<Vec<_>>>()?,
        stage: stage.to_owned(),
        case_id_sha256: Some(case_id_sha256.to_owned()),
        probe_ordinal: Some(ordinal),
        expected_outcome: K2UncertaintyR8BExpectedOutcomeV3::AuthoritySuccess,
        expected_exit_predicate: None,
        validator: if target_role == "M04_PROBE" {
            K2UncertaintyR8BValidatorV3::RepresentativeCount
        } else {
            K2UncertaintyR8BValidatorV3::ConcreteReceipt
        },
    };
    let started = writer.request(
        invocation,
        request_root_sha256.to_owned(),
        stdin_sha256.to_owned(),
        monotonic_ns_v2(),
    )?;
    Ok(Some((writer, started)))
}

fn validated_public_fact_v3(
    role: K2UncertaintyConfirmGuestExecutableV1,
    mounts: &[K2UncertaintyConfirmDataMountV1<'_>],
    stdout: &[u8],
) -> K2CompositionResultV1<K2UncertaintyR8BValidatedFactV3> {
    if role != K2UncertaintyConfirmGuestExecutableV1::Probe {
        return Ok(K2UncertaintyR8BValidatedFactV3::None);
    }
    let artifacts: K2UncertaintyProbeArtifactsV1 = uncertainty_decode_v1(stdout)?;
    artifacts.validate()?;
    let root = mounts
        .first()
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_r8b_v3_probe_mount_missing",
        ))?
        .host_path;
    let output = reopen_self_formed_probe_output_v1(root, &artifacts)?;
    let count = output.frontier.representative_probe_roots_sha256.len() as u64;
    if !(8..=1792).contains(&count) {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_r8b_v3_representative_count_invalid",
        ));
    }
    Ok(K2UncertaintyR8BValidatedFactV3::RepresentativeCount { count })
}

fn public_child_identity_v2(
    role: K2UncertaintyConfirmGuestExecutableV1,
) -> K2CompositionResultV1<(&'static str, &'static str)> {
    match role {
        K2UncertaintyConfirmGuestExecutableV1::Learner => Ok(("C03", "M03_LEARNER")),
        K2UncertaintyConfirmGuestExecutableV1::Probe => Ok(("C04", "M04_PROBE")),
        K2UncertaintyConfirmGuestExecutableV1::Selector => Ok(("C05", "M05_SELECTOR")),
        K2UncertaintyConfirmGuestExecutableV1::Baseline => Ok(("C06", "M06_BASELINE")),
        K2UncertaintyConfirmGuestExecutableV1::SelectionPreverifier => {
            Ok(("C07", "M07_SELECTION_PREVERIFIER"))
        }
        K2UncertaintyConfirmGuestExecutableV1::ClosurePlanner => Ok(("C08", "M08_CLOSURE_PLANNER")),
        K2UncertaintyConfirmGuestExecutableV1::ClosureVerifier => {
            Ok(("C09", "M09_CLOSURE_VERIFIER"))
        }
        _ => Err(K2CompositionErrorV1::Invalid(
            "self_formed_r8b_m10_child_role_invalid",
        )),
    }
}

fn validate_public_child_output_v2(
    role: K2UncertaintyConfirmGuestExecutableV1,
    bytes: &[u8],
) -> K2CompositionResultV1<(String, String)> {
    match role {
        K2UncertaintyConfirmGuestExecutableV1::Learner => {
            let value: K2UncertaintyLearnerResponseV1 = uncertainty_decode_v1(bytes)?;
            value.validate()?;
            Ok((value.schema, value.response_root_sha256))
        }
        K2UncertaintyConfirmGuestExecutableV1::Probe => {
            let value: K2UncertaintyProbeArtifactsV1 = uncertainty_decode_v1(bytes)?;
            value.validate()?;
            Ok((value.schema, value.artifacts_root_sha256))
        }
        K2UncertaintyConfirmGuestExecutableV1::Selector => {
            let value: K2InquirySelectionPrecommitV1 = uncertainty_decode_v1(bytes)?;
            let mut resealed = value.clone();
            resealed.reseal()?;
            if value != resealed {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_r8b_selector_output_invalid",
                ));
            }
            Ok((value.schema, value.precommit_root_sha256))
        }
        K2UncertaintyConfirmGuestExecutableV1::Baseline => {
            let value: K2InquiryBaselinesV1 = uncertainty_decode_v1(bytes)?;
            let mut resealed = value.clone();
            resealed.reseal()?;
            if value != resealed {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_r8b_baseline_output_invalid",
                ));
            }
            Ok((value.schema, value.baselines_root_sha256))
        }
        K2UncertaintyConfirmGuestExecutableV1::SelectionPreverifier => {
            let value: K2InquiryVerifierReceiptV1 = uncertainty_decode_v1(bytes)?;
            match value {
                K2InquiryVerifierReceiptV1::Selection { value } => {
                    let mut resealed = value.clone();
                    resealed.reseal()?;
                    if value != resealed {
                        return Err(K2CompositionErrorV1::Invalid(
                            "self_formed_r8b_preverifier_output_invalid",
                        ));
                    }
                    Ok((value.schema, value.receipt_root_sha256))
                }
                K2InquiryVerifierReceiptV1::Outcome { .. } => Err(K2CompositionErrorV1::Invalid(
                    "self_formed_r8b_preverifier_output_invalid",
                )),
            }
        }
        K2UncertaintyConfirmGuestExecutableV1::ClosurePlanner => {
            let value: K2UncertaintyClosureCensusV1 = uncertainty_decode_v1(bytes)?;
            value.validate()?;
            Ok((value.schema, value.census_root_sha256))
        }
        K2UncertaintyConfirmGuestExecutableV1::ClosureVerifier => {
            let value: K2UncertaintyClosureVerificationReceiptV1 = uncertainty_decode_v1(bytes)?;
            value.validate()?;
            Ok((value.schema, value.receipt_root_sha256))
        }
        _ => Err(K2CompositionErrorV1::Invalid(
            "self_formed_r8b_m10_child_role_invalid",
        )),
    }
}

fn monotonic_ns_v2() -> u64 {
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    ORIGIN.get_or_init(Instant::now).elapsed().as_nanos() as u64
}

fn prepare_empty_output_root_v1(root: &Path) -> K2CompositionResultV1<()> {
    if !root.is_absolute() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_public_output_root_invalid",
        ));
    }
    fs::create_dir_all(root)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_public_output_root"))?;
    fs::set_permissions(root, fs::Permissions::from_mode(0o700))
        .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_public_output_root"))?;
    if fs::read_dir(root)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_public_output_root"))?
        .next()
        .is_some()
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_public_output_root_not_empty",
        ));
    }
    Ok(())
}

#[allow(dead_code)]
fn _path_from_owner_v1(owner: &K2UncertaintyPublicOwnerV1) -> PathBuf {
    PathBuf::from(&owner.executable_path)
}
