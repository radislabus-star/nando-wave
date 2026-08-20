use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::super::K2CompositionResultV1;
use super::*;

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn r7k_cleanup_pipeline_preserves_evidence_removes_disposable_and_publishes_development_only() {
    let fixture = CleanupFixture::new("pipeline");
    let (terminal, manifest, authorization) = fixture.prepare_authorization();
    let owner = fixture.execute_cleanup(&authorization);
    let cleanup = fixture.verify_cleanup(&manifest, &owner);
    let cleanup_after_restart = fixture.verify_cleanup(&manifest, &owner);
    assert_eq!(cleanup_after_restart, cleanup);
    let request = K2UncertaintyDevelopmentResultRequestV1::seal(
        fixture.control_root.to_string_lossy().into_owned(),
        terminal.clone(),
        cleanup.clone(),
        root("result-publisher"),
    )
    .expect("development result request");
    let result =
        publish_self_formed_development_result_v1(&request).expect("publish Development result");
    let result_after_restart =
        publish_self_formed_development_result_v1(&request).expect("restart Development result");
    assert_eq!(result_after_restart, result);

    assert_eq!(result.disposition, "DEVELOPMENT_REHEARSAL_COMPLETE");
    assert_eq!(
        result.terminal_receipt_root_sha256,
        terminal.receipt_root_sha256
    );
    assert_eq!(
        result.cleanup_receipt_root_sha256,
        cleanup.receipt_root_sha256
    );
    assert_eq!(cleanup.deleted_paths, 2);
    assert_eq!(cleanup.retained_paths, 2);
    assert_eq!(cleanup.unexpected_residue, 0);
    assert!(fixture.governed_root.join("retained.json").exists());
    assert!(fixture.governed_root.join("superseded.json").exists());
    assert!(!fixture.governed_root.join("scratch/temp.bin").exists());
    assert!(!fixture.governed_root.join("scratch").exists());
    assert_eq!(mode(&fixture.governed_root.join("retained.json")), 0o600);
}

#[test]
fn r7k_cleanup_restarts_every_semantic_durability_prefix_without_redeletion() {
    for target in 0..2 {
        for fault in [
            K2UncertaintyCleanupFaultV1::BeforeIntent { target },
            K2UncertaintyCleanupFaultV1::AfterIntent { target },
            K2UncertaintyCleanupFaultV1::AfterMutation { target },
            K2UncertaintyCleanupFaultV1::AfterParentFsync { target },
            K2UncertaintyCleanupFaultV1::AfterCompletion { target },
        ] {
            let fixture = CleanupFixture::new(&format!("restart-{target}-{fault:?}"));
            let (_, manifest, authorization) = fixture.prepare_authorization();
            let request = fixture.owner_request(&authorization);
            let error = execute_self_formed_cleanup_with_fault_v1(&request, fault)
                .expect_err("fault must interrupt cleanup");
            assert_eq!(
                error.to_string(),
                "k2_composition_invalid:self_formed_cleanup_injected_fault"
            );
            let owner = execute_self_formed_cleanup_v1(&request).expect("restart cleanup");
            let cleanup = fixture.verify_cleanup(&manifest, &owner);
            assert!(cleanup.cleanup_frozen);
            assert_eq!(cleanup.deleted_paths, 2);
        }
    }
}

#[test]
fn r7k_k12_rejects_retained_deletion_and_disposable_residue_independently() {
    let retained = CleanupFixture::new("k12-retained");
    let (_, retained_manifest, retained_authorization) = retained.prepare_authorization();
    let retained_owner = retained.execute_cleanup(&retained_authorization);
    fs::remove_file(retained.governed_root.join("retained.json"))
        .expect("delete retained evidence in negative clone");
    assert!(
        retained
            .verify_cleanup_result(&retained_manifest, &retained_owner)
            .is_err(),
        "retained evidence deletion must reject CleanupFrozen"
    );

    let residue = CleanupFixture::new("k12-residue");
    let (_, residue_manifest, residue_authorization) = residue.prepare_authorization();
    let residue_owner = residue.execute_cleanup(&residue_authorization);
    fs::create_dir(residue.governed_root.join("scratch"))
        .expect("restore disposable directory in negative clone");
    fs::set_permissions(
        residue.governed_root.join("scratch"),
        fs::Permissions::from_mode(0o700),
    )
    .expect("chmod restored disposable directory");
    fs::write(residue.governed_root.join("scratch/temp.bin"), b"temporary")
        .expect("restore disposable file");
    fs::set_permissions(
        residue.governed_root.join("scratch/temp.bin"),
        fs::Permissions::from_mode(0o600),
    )
    .expect("chmod restored disposable file");
    assert!(
        residue
            .verify_cleanup_result(&residue_manifest, &residue_owner)
            .is_err(),
        "disposable residue must reject CleanupFrozen"
    );
}

#[test]
fn r7k_result_rejects_valid_foreign_terminal_root() {
    let fixture = CleanupFixture::new("foreign-terminal");
    let (_, manifest, authorization) = fixture.prepare_authorization();
    let owner = fixture.execute_cleanup(&authorization);
    let cleanup = fixture.verify_cleanup(&manifest, &owner);
    let foreign_terminal = K2UncertaintyTerminalEvaluationReceiptV1::seal(
        K2UncertaintyTerminalModeV1::DevelopmentRehearsal,
        root("foreign-development-terminal-request"),
        K2UncertaintyTerminalDispositionV1::DevelopmentRehearsalPass,
        "development_component_routes_complete".to_owned(),
        root("terminal-evaluator"),
    )
    .expect("valid foreign terminal");
    assert!(
        K2UncertaintyDevelopmentResultRequestV1::seal(
            fixture.control_root.to_string_lossy().into_owned(),
            foreign_terminal,
            cleanup,
            root("result-publisher"),
        )
        .is_err(),
        "a valid but foreign terminal root must not join CleanupFrozen"
    );
}

#[test]
fn r7k_cleanup_verifier_restarts_after_every_publication_boundary() {
    for fault in [
        K2UncertaintyCleanupVerifierFaultV1::AfterCensusPage { page: 0 },
        K2UncertaintyCleanupVerifierFaultV1::BeforeReceipt,
        K2UncertaintyCleanupVerifierFaultV1::AfterReceipt,
    ] {
        let fixture = CleanupFixture::new(&format!("verifier-{fault:?}"));
        let (_, manifest, authorization) = fixture.prepare_authorization();
        let owner = fixture.execute_cleanup(&authorization);
        let request = fixture.verify_request(&manifest, &owner);
        let error = verify_self_formed_cleanup_with_fault_v1(&request, fault)
            .expect_err("verifier fault must interrupt publication");
        assert_eq!(
            error.to_string(),
            "k2_composition_invalid:self_formed_cleanup_verifier_injected_fault"
        );
        let receipt = verify_self_formed_cleanup_v1(&request).expect("restart cleanup verifier");
        assert!(receipt.cleanup_frozen);
        assert_eq!(receipt.unexpected_residue, 0);
    }
}

#[test]
fn r7k_before_census_restarts_after_every_publication_boundary() {
    for fault in [
        K2UncertaintyCleanupManifestFaultV1::AfterPage { page: 0 },
        K2UncertaintyCleanupManifestFaultV1::BeforeDescriptor,
        K2UncertaintyCleanupManifestFaultV1::AfterDescriptor,
    ] {
        let fixture = CleanupFixture::new(&format!("before-census-{fault:?}"));
        let (manifest, pages) = fixture.census();
        let error = publish_self_formed_cleanup_manifest_with_fault_v1(
            &fixture.governed_root,
            &fixture.control_root,
            &manifest,
            &pages,
            fault,
        )
        .expect_err("manifest fault must interrupt publication");
        assert_eq!(
            error.to_string(),
            "k2_composition_invalid:self_formed_cleanup_manifest_injected_fault"
        );
        publish_self_formed_cleanup_manifest_v1(
            &fixture.governed_root,
            &fixture.control_root,
            &manifest,
            &pages,
        )
        .expect("restart before-census publication");
        assert_eq!(
            load_self_formed_cleanup_manifest_pages_v1(&fixture.control_root, &manifest)
                .expect("reload durable before-census pages"),
            pages
        );
    }
}

#[test]
fn r7k_authorization_and_result_restart_at_both_receipt_boundaries() {
    for fault in [
        K2UncertaintyCleanupAuthorizationFaultV1::BeforeReceipt,
        K2UncertaintyCleanupAuthorizationFaultV1::AfterReceipt,
    ] {
        let fixture = CleanupFixture::new(&format!("authorization-{fault:?}"));
        let terminal = fixture.terminal();
        let (manifest, pages) = fixture.census();
        publish_self_formed_cleanup_manifest_v1(
            &fixture.governed_root,
            &fixture.control_root,
            &manifest,
            &pages,
        )
        .expect("publish before census");
        let request = fixture.authorization_request(&terminal, &manifest);
        let error = authorize_self_formed_cleanup_with_fault_v1(&request, fault)
            .expect_err("authorization fault must interrupt publication");
        assert_eq!(
            error.to_string(),
            "k2_composition_invalid:self_formed_cleanup_authorization_injected_fault"
        );
        assert_eq!(
            authorize_self_formed_cleanup_v1(&request).expect("restart authorization"),
            authorize_self_formed_cleanup_v1(&request).expect("idempotent authorization")
        );
    }

    for fault in [
        K2UncertaintyDevelopmentResultFaultV1::BeforeReceipt,
        K2UncertaintyDevelopmentResultFaultV1::AfterReceipt,
    ] {
        let fixture = CleanupFixture::new(&format!("result-{fault:?}"));
        let (terminal, manifest, authorization) = fixture.prepare_authorization();
        let owner = fixture.execute_cleanup(&authorization);
        let cleanup = fixture.verify_cleanup(&manifest, &owner);
        let request = K2UncertaintyDevelopmentResultRequestV1::seal(
            fixture.control_root.to_string_lossy().into_owned(),
            terminal,
            cleanup,
            root("result-publisher"),
        )
        .expect("Development result request");
        let error = publish_self_formed_development_result_with_fault_v1(&request, fault)
            .expect_err("result fault must interrupt publication");
        assert_eq!(
            error.to_string(),
            "k2_composition_invalid:self_formed_development_result_injected_fault"
        );
        assert_eq!(
            publish_self_formed_development_result_v1(&request).expect("restart result"),
            publish_self_formed_development_result_v1(&request).expect("idempotent result")
        );
    }
}

struct CleanupFixture {
    root: PathBuf,
    governed_root: PathBuf,
    control_root: PathBuf,
}

impl CleanupFixture {
    fn new(label: &str) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nando-k2-r7k-{label}-{}-{sequence}",
            std::process::id()
        ));
        let governed_root = root.join("governed");
        let control_root = root.join("control");
        create_private_directory(&governed_root);
        create_private_directory(&control_root);
        create_private_file(&governed_root.join("retained.json"), b"retained");
        create_private_file(&governed_root.join("superseded.json"), b"superseded");
        create_private_directory(&governed_root.join("scratch"));
        create_private_file(&governed_root.join("scratch/temp.bin"), b"temporary");
        Self {
            root,
            governed_root,
            control_root,
        }
    }

    fn prepare_authorization(
        &self,
    ) -> (
        K2UncertaintyTerminalEvaluationReceiptV1,
        K2UncertaintyCleanupManifestV1,
        K2UncertaintyCleanupAuthorizationReceiptV1,
    ) {
        let terminal = self.terminal();
        let (manifest, pages) = self.census();
        publish_self_formed_cleanup_manifest_v1(
            &self.governed_root,
            &self.control_root,
            &manifest,
            &pages,
        )
        .expect("publish before census");
        let request = self.authorization_request(&terminal, &manifest);
        let authorization = authorize_self_formed_cleanup_v1(&request).expect("authorize cleanup");
        (terminal, manifest, authorization)
    }

    fn terminal(&self) -> K2UncertaintyTerminalEvaluationReceiptV1 {
        K2UncertaintyTerminalEvaluationReceiptV1::seal(
            K2UncertaintyTerminalModeV1::DevelopmentRehearsal,
            root("development-terminal-request"),
            K2UncertaintyTerminalDispositionV1::DevelopmentRehearsalPass,
            "development_component_routes_complete".to_owned(),
            root("terminal-evaluator"),
        )
        .expect("Development terminal receipt")
    }

    fn authorization_request(
        &self,
        terminal: &K2UncertaintyTerminalEvaluationReceiptV1,
        manifest: &K2UncertaintyCleanupManifestV1,
    ) -> K2UncertaintyCleanupAuthorizationRequestV1 {
        K2UncertaintyCleanupAuthorizationRequestV1::seal(
            self.control_root.to_string_lossy().into_owned(),
            root("experiment"),
            terminal.clone(),
            manifest.clone(),
            root("journal-projection"),
            root("observer-durable"),
            root("terminal-durable"),
            root("cleanup-authorizer"),
        )
        .expect("cleanup authorization request")
    }

    fn census(
        &self,
    ) -> (
        K2UncertaintyCleanupManifestV1,
        Vec<K2UncertaintyCleanupManifestPageV1>,
    ) {
        let registry = [
            (
                "scratch/temp.bin",
                K2UncertaintyCleanupArtifactKindV1::DisposableWorkspace,
            ),
            (
                "retained.json",
                K2UncertaintyCleanupArtifactKindV1::RetainedEvidence,
            ),
            (
                "scratch",
                K2UncertaintyCleanupArtifactKindV1::DisposableWorkspace,
            ),
            (
                "superseded.json",
                K2UncertaintyCleanupArtifactKindV1::SupersededEvidence,
            ),
        ]
        .into_iter()
        .map(|(path, kind)| K2UncertaintyCleanupRegistryEntryV1 {
            relative_path: path.to_owned(),
            artifact_kind: kind,
            producer_executable_sha256: root(&format!("producer-{path}")),
            producing_journal_event_root_sha256: root(&format!("journal-{path}")),
        })
        .collect();
        census_self_formed_cleanup_artifacts_v1(
            &self.governed_root,
            root("experiment"),
            registry,
            root("census-executable"),
        )
        .expect("before census")
    }

    fn owner_request(
        &self,
        authorization: &K2UncertaintyCleanupAuthorizationReceiptV1,
    ) -> K2UncertaintyCleanupOwnerRequestV1 {
        K2UncertaintyCleanupOwnerRequestV1::seal(
            self.governed_root.to_string_lossy().into_owned(),
            self.control_root.to_string_lossy().into_owned(),
            authorization.clone(),
            root("cleanup-owner"),
        )
        .expect("cleanup owner request")
    }

    fn execute_cleanup(
        &self,
        authorization: &K2UncertaintyCleanupAuthorizationReceiptV1,
    ) -> K2UncertaintyCleanupOwnerReceiptV1 {
        execute_self_formed_cleanup_v1(&self.owner_request(authorization)).expect("execute cleanup")
    }

    fn verify_cleanup(
        &self,
        manifest: &K2UncertaintyCleanupManifestV1,
        owner: &K2UncertaintyCleanupOwnerReceiptV1,
    ) -> K2UncertaintyCleanupReceiptV1 {
        self.verify_cleanup_result(manifest, owner)
            .expect("verify cleanup")
    }

    fn verify_cleanup_result(
        &self,
        manifest: &K2UncertaintyCleanupManifestV1,
        owner: &K2UncertaintyCleanupOwnerReceiptV1,
    ) -> K2CompositionResultV1<K2UncertaintyCleanupReceiptV1> {
        verify_self_formed_cleanup_v1(&self.verify_request(manifest, owner))
    }

    fn verify_request(
        &self,
        manifest: &K2UncertaintyCleanupManifestV1,
        owner: &K2UncertaintyCleanupOwnerReceiptV1,
    ) -> K2UncertaintyCleanupVerifyRequestV1 {
        K2UncertaintyCleanupVerifyRequestV1::seal(
            self.governed_root.to_string_lossy().into_owned(),
            self.control_root.to_string_lossy().into_owned(),
            manifest.clone(),
            owner.clone(),
            root("cleanup-verifier"),
        )
        .expect("cleanup verifier request")
    }
}

impl Drop for CleanupFixture {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("R7K failed fixture retained at {}", self.root.display());
        } else {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

fn create_private_directory(path: &Path) {
    fs::create_dir_all(path).expect("create private directory");
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).expect("chmod private directory");
}

fn create_private_file(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).expect("write private file");
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).expect("chmod private file");
}

fn mode(path: &Path) -> u32 {
    fs::metadata(path)
        .expect("file metadata")
        .permissions()
        .mode()
        & 0o7777
}

fn root(label: &str) -> String {
    uncertainty_root_v1(&("nando.k2-self-formed-r7k-test.v1", label)).expect("test root")
}
