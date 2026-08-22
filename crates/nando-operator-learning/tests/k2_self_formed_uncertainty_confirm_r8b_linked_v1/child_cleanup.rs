use super::*;

pub(super) struct CleanupOutputV2 {
    pub(super) cleanup: K2UncertaintyCleanupReceiptV1,
    pub(super) result: K2UncertaintyDevelopmentResultReceiptV1,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn complete_cleanup_v2(
    ledger: &mut DurableProcessLedgerV2,
    binaries: &LinkedBinariesV2,
    governed: &Path,
    control: &Path,
    owner_request: &nando_operator_learning::K2UncertaintyConfirmOwnerRequestV1,
    development: &K2UncertaintyDevelopmentRehearsalOwnerReceiptV1,
    terminal: &K2UncertaintyTerminalEvaluationReceiptV1,
    candidate: &Path,
) -> CleanupOutputV2 {
    let registry = actual_tree_registry_v2(governed, development);
    let m24_sha = composition_sha256_file_v1(
        &std::env::current_exe().expect("M24 cleanup census executable"),
    )
    .expect("M24 cleanup census SHA-256");
    let (manifest, pages) = census_self_formed_cleanup_artifacts_v1(
        governed,
        owner_request.descriptor.experiment_id_sha256.clone(),
        registry,
        m24_sha,
    )
    .expect("linked cleanup census");
    publish_self_formed_cleanup_manifest_v1(governed, control, &manifest, &pages)
        .expect("publish linked cleanup census");

    let m20 = binaries.get("M20_CLEANUP_AUTHORIZER");
    let auth_request = K2UncertaintyCleanupAuthorizationRequestV1::seal(
        "/control".to_owned(),
        owner_request.descriptor.experiment_id_sha256.clone(),
        terminal.clone(),
        manifest.clone(),
        root_v1("linked-cleanup-journal"),
        root_v1("linked-observer-event"),
        root_v1("linked-terminal-event"),
        m20.sha256.clone(),
    )
    .expect("M20 request");
    let authorization = run_cleanup_recorded_v2::<_, K2UncertaintyCleanupAuthorizationReceiptV1>(
        ledger,
        m20,
        "C16",
        K2UncertaintyR7kCleanupGuestV1::Authorizer,
        None,
        control,
        &auth_request,
        None,
    );
    let m21 = binaries.get("M21_CLEANUP_OWNER");
    let cleanup_owner_request = K2UncertaintyCleanupOwnerRequestV1::seal(
        "/governed".to_owned(),
        "/control".to_owned(),
        authorization,
        m21.sha256.clone(),
    )
    .expect("M21 request");
    let cleanup_owner = run_cleanup_recorded_v2::<_, K2UncertaintyCleanupOwnerReceiptV1>(
        ledger,
        m21,
        "C17",
        K2UncertaintyR7kCleanupGuestV1::Owner,
        Some(governed),
        control,
        &cleanup_owner_request,
        None,
    );
    let m22 = binaries.get("M22_CLEANUP_VERIFIER");
    let verify_request = K2UncertaintyCleanupVerifyRequestV1::seal(
        "/governed".to_owned(),
        "/control".to_owned(),
        manifest,
        cleanup_owner,
        m22.sha256.clone(),
    )
    .expect("M22 request");
    let cleanup = run_cleanup_recorded_v2::<_, K2UncertaintyCleanupReceiptV1>(
        ledger,
        m22,
        "C18",
        K2UncertaintyR7kCleanupGuestV1::Verifier,
        Some(governed),
        control,
        &verify_request,
        Some((
            candidate,
            "linked/cleanup.json",
            K2UncertaintyR8BEvidenceKindV2::CleanupTransaction,
        )),
    );

    let m23 = binaries.get("M23_DEVELOPMENT_RESULT_PUBLISHER");
    let result_request = K2UncertaintyResultProcessRequestV1::Development {
        request: K2UncertaintyDevelopmentResultRequestV1::seal(
            "/control".to_owned(),
            terminal.clone(),
            cleanup.clone(),
            m23.sha256.clone(),
        )
        .expect("M23 request"),
    };
    let result = run_cleanup_recorded_v2::<_, K2UncertaintyDevelopmentResultReceiptV1>(
        ledger,
        m23,
        "C20",
        K2UncertaintyR7kCleanupGuestV1::ResultPublisher,
        None,
        control,
        &result_request,
        Some((
            candidate,
            "linked/development-result.json",
            K2UncertaintyR8BEvidenceKindV2::DevelopmentResult,
        )),
    );
    CleanupOutputV2 { cleanup, result }
}

#[allow(clippy::too_many_arguments)]
fn run_cleanup_recorded_v2<I, O>(
    ledger: &mut DurableProcessLedgerV2,
    binary: &BinaryV2,
    stage: &str,
    role: K2UncertaintyR7kCleanupGuestV1,
    governed: Option<&Path>,
    control: &Path,
    request: &I,
    authority_output: Option<(&Path, &str, K2UncertaintyR8BEvidenceKindV2)>,
) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let input = uncertainty_bytes_v1(request).expect("cleanup process input");
    let started = ledger.start_bound(
        stage,
        None,
        None,
        binary,
        nando_operator_learning::uncertainty_root_v1(request)
            .expect("cleanup request semantic root"),
        nando_operator_learning::composition_sha256_bytes_v1(&input),
    );
    let outcome = nando_operator_learning::run_self_formed_r7k_cleanup_sandbox_measured_v1(
        role,
        &binary.path,
        &binary.sha256,
        governed,
        control,
        &input,
        60,
    )
    .unwrap_or_else(|error| ledger.fail_launch_bound(&started, error));
    let output = sandbox_output_v2(outcome);
    if !output.status.success() {
        ledger.fail_unexpected_bound(&started, &output, format!("{} cleanup failed", binary.role));
    }
    let value: O =
        nando_operator_learning::uncertainty_decode_v1(&output.stdout).unwrap_or_else(|error| {
            ledger.fail_unexpected_bound(
                &started,
                &output,
                format!("decode linked cleanup receipt: {error}"),
            )
        });
    let (schema, root) =
        typed_json_identity_v2(&output.stdout).expect("typed linked cleanup receipt");
    let outputs = authority_output
        .map(|(candidate, relative, kind)| {
            persist_candidate_v2(candidate, relative, &value);
            attested_evidence_output_v3(
                binary,
                relative,
                kind,
                &output.stdout,
                &schema,
                root.clone(),
            )
        })
        .into_iter()
        .collect();
    ledger.finish_bound(
        &started,
        &output,
        schema,
        root,
        K2UncertaintyR8BValidatedFactV3::None,
        outputs,
    );
    value
}

fn actual_tree_registry_v2(
    governed: &Path,
    owner: &K2UncertaintyDevelopmentRehearsalOwnerReceiptV1,
) -> Vec<K2UncertaintyCleanupRegistryEntryV1> {
    let mut pending = vec![governed.to_path_buf()];
    let mut registry = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("read linked governed tree") {
            let path = entry.expect("linked governed entry").path();
            let relative = path
                .strip_prefix(governed)
                .expect("linked governed relative path")
                .to_string_lossy()
                .into_owned();
            let disposable = path.is_file()
                && (relative.starts_with("generated/private/resolver/")
                    || relative.starts_with("generated/private/final-truth/"));
            registry.push(K2UncertaintyCleanupRegistryEntryV1 {
                relative_path: relative,
                artifact_kind: if disposable {
                    K2UncertaintyCleanupArtifactKindV1::DisposableWorkspace
                } else {
                    K2UncertaintyCleanupArtifactKindV1::RetainedEvidence
                },
                producer_executable_sha256: owner.owner_executable_sha256.clone(),
                producing_journal_event_root_sha256: owner
                    .cases_generated_event_root_sha256
                    .clone(),
            });
            if path.is_dir() {
                pending.push(path);
            }
        }
    }
    registry.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    registry
}

pub(super) fn public_owner_set_v2(binaries: &LinkedBinariesV2) -> K2UncertaintyPublicOwnerSetV1 {
    let roles = [
        (K2UncertaintyPublicOwnerRoleV1::Learner, "M03_LEARNER"),
        (K2UncertaintyPublicOwnerRoleV1::Probe, "M04_PROBE"),
        (K2UncertaintyPublicOwnerRoleV1::Selector, "M05_SELECTOR"),
        (K2UncertaintyPublicOwnerRoleV1::Baseline, "M06_BASELINE"),
        (
            K2UncertaintyPublicOwnerRoleV1::SelectionPreverifier,
            "M07_SELECTION_PREVERIFIER",
        ),
        (
            K2UncertaintyPublicOwnerRoleV1::ClosurePlanner,
            "M08_CLOSURE_PLANNER",
        ),
        (
            K2UncertaintyPublicOwnerRoleV1::ClosureVerifier,
            "M09_CLOSURE_VERIFIER",
        ),
    ];
    K2UncertaintyPublicOwnerSetV1::seal(
        roles
            .into_iter()
            .map(|(role, name)| {
                let binary = binaries.get(name);
                K2UncertaintyPublicOwnerV1 {
                    role,
                    executable_path: binary.path.to_string_lossy().into_owned(),
                    executable_sha256: binary.sha256.clone(),
                }
            })
            .collect(),
    )
    .expect("linked public owner set")
}

pub(super) fn persist_candidate_v2<T: serde::Serialize>(root: &Path, relative: &str, value: &T) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create candidate parent");
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .expect("chmod candidate parent");
    }
    write_new_read_only_v2(
        &path,
        &uncertainty_bytes_v1(value).expect("candidate canonical bytes"),
    );
}

pub(super) fn publish_child_owned_v2(
    root: &Path,
    oracle: &K2UncertaintyR8BOracleWrapperV3,
    controls: &K2UncertaintyR8BControlWrapperV3,
    linked: &K2UncertaintyR8BMeasuredReceiptV2,
) {
    for (sequence, relative, bytes) in [
        (
            0_u64,
            "linked/oracle-batch.json",
            uncertainty_bytes_v1(oracle).expect("oracle batch bytes"),
        ),
        (
            1,
            "linked/control-scopes.json",
            uncertainty_bytes_v1(controls).expect("control census bytes"),
        ),
        (
            2,
            "linked/route.json",
            uncertainty_bytes_v1(linked).expect("linked route bytes"),
        ),
    ] {
        let parent = root
            .join(relative)
            .parent()
            .expect("child output parent")
            .to_path_buf();
        fs::create_dir_all(&parent).expect("create child output parent");
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700))
            .expect("chmod child output parent");
        publish_immutable_file_v1(
            root,
            relative,
            &bytes,
            0o400,
            sequence,
            K2UncertaintyImmutablePublicationFaultV1::None,
        )
        .expect("publish child-owned receipt");
    }
    File::open(root)
        .expect("open child output root")
        .sync_all()
        .expect("fsync child output root");
}
