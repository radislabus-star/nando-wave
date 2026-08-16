use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde::{Serialize, de::DeserializeOwned};

use super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, K2InquiryBaselineRequestV1, K2InquiryBaselinesV1,
    K2InquiryVerifierCommandV1, K2InquiryVerifierReceiptV1, composition_sha256_file_v1,
};
use super::{
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1,
    K2UncertaintyBatchPrecommitV2, K2UncertaintyCasePreverificationV2,
    K2UncertaintyClosureCensusV1, K2UncertaintyClosurePlanV1, K2UncertaintyClosurePlannerRequestV1,
    K2UncertaintyClosureVerificationReceiptV1, K2UncertaintyClosureVerificationRequestV1,
    K2UncertaintyConfirmDataMountV1, K2UncertaintyConfirmGuestExecutableV1,
    K2UncertaintyConfirmMountTargetV1, K2UncertaintyLearnerRequestV1,
    K2UncertaintyLearnerResponseV1, K2UncertaintyProbeArtifactsV1, K2UncertaintyProbeRequestV1,
    K2UncertaintyPublicCoordinatorRequestV1, K2UncertaintyPublicOwnerRoleV1,
    K2UncertaintyPublicOwnerV1, K2UncertaintyPublicPrecommitReceiptV1,
    K2UncertaintyPublicPreparedCaseV1, denied_authority_v1,
    preverify_self_formed_case_with_owner_v1, publish_self_formed_final_verifier_material_v2,
    publish_self_formed_public_case_v1, publish_self_formed_public_precommit_v1,
    reopen_self_formed_probe_output_v1, run_self_formed_confirm_sandbox_v1,
    run_self_formed_tournament_with_owners_v1, uncertainty_bytes_v1, uncertainty_decode_v1,
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
                    &[],
                    selector_request,
                    30,
                )
            },
            &mut |baseline_request: &K2InquiryBaselineRequestV1| {
                invoke_owner_v1::<_, K2InquiryBaselinesV1>(
                    K2UncertaintyConfirmGuestExecutableV1::Baseline,
                    baseline,
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
    if composition_sha256_file_v1(&executable)? != request.coordinator_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_public_coordinator_executable_mismatch",
        ));
    }
    let receipt =
        execute_self_formed_public_coordinator_v1(&request, Path::new(&request.output_root))?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_public_coordinator_stdout"))
}

fn invoke_owner_v1<I, O>(
    role: K2UncertaintyConfirmGuestExecutableV1,
    owner: &K2UncertaintyPublicOwnerV1,
    mounts: &[K2UncertaintyConfirmDataMountV1<'_>],
    input: &I,
    cpu_seconds: u64,
) -> K2CompositionResultV1<O>
where
    I: Serialize,
    O: DeserializeOwned + Serialize,
{
    let output = run_self_formed_confirm_sandbox_v1(
        role,
        Path::new(&owner.executable_path),
        &owner.executable_sha256,
        mounts,
        &uncertainty_bytes_v1(input)?,
        cpu_seconds,
    )?;
    uncertainty_decode_v1(&output)
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
