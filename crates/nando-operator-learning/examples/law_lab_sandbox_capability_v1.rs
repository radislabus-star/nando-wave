use std::error::Error;
use std::fs::{self, File};
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use nando_operator_learning::{
    LAW_LAB_MAX_INPUT_BYTES_V1, LawLabProbeDomainV1, LawLabSandboxAdapterV1,
    LawLabSandboxCapabilityCaseV1, LawLabSandboxCapabilityReportV1, LawLabSandboxConfigV1,
    LawLabSandboxExecutorManifestV1, LawLabSandboxOperationV1, LawLabSandboxPurposeV1,
    LawLabSandboxRequestInputV1, LawLabSandboxRequestV1, LawLabTreeManifestV1,
};

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() != 3 {
        return Err(io::Error::other(
            "usage: law_lab_sandbox_capability_v1 WORKER WORKER_SHA256 SCRATCH_PARENT",
        )
        .into());
    }
    let worker_path = PathBuf::from(&arguments[0]);
    let worker_sha256 = arguments[1]
        .to_str()
        .filter(|value| valid_nonzero_sha256(value))
        .ok_or_else(|| io::Error::other("worker sha256 invalid"))?
        .to_owned();
    let scratch_parent = PathBuf::from(&arguments[2]);
    let mut scratch = CapabilityScratchV1::create(&scratch_parent)?;

    let filesystem_source = scratch.seal_source(
        "filesystem",
        &[
            ("keep.txt", b"stable capability payload".as_slice()),
            ("remove.txt", b"remove capability payload".as_slice()),
        ],
    )?;
    let structured_source = scratch.seal_source(
        "structured-data",
        &[(
            "record.json",
            br#"{ "z": 3, "a": {"b": 2, "a": 1} }"#.as_slice(),
        )],
    )?;
    let adapter = LawLabSandboxAdapterV1::new(
        LawLabSandboxConfigV1::strict_generated_capability_self_test_v1(
            worker_path,
            worker_sha256,
            scratch.source_store.clone(),
            scratch.workspace_store.clone(),
        ),
    );
    let executor_manifest = adapter.executor_manifest()?;

    let filesystem_request = capability_request_v1(
        &executor_manifest,
        &filesystem_source,
        "filesystem",
        LawLabProbeDomainV1::Filesystem,
        vec![
            LawLabSandboxOperationV1::CopySourceFile {
                source_path: "keep.txt".to_owned(),
                work_path: "copies/keep.txt".to_owned(),
            },
            LawLabSandboxOperationV1::RemoveWorkPath {
                work_path: "remove.txt".to_owned(),
            },
        ],
    )?;
    let filesystem_execution = adapter.execute(&filesystem_request)?;

    let structured_request = capability_request_v1(
        &executor_manifest,
        &structured_source,
        "structured-data",
        LawLabProbeDomainV1::StructuredData,
        vec![LawLabSandboxOperationV1::CanonicalizeJsonFile {
            work_path: "record.json".to_owned(),
        }],
    )?;
    let structured_execution = adapter.execute(&structured_request)?;

    let fixture_cleanup_root_sha256 = scratch.cleanup()?;
    let report = LawLabSandboxCapabilityReportV1::seal(
        executor_manifest,
        LawLabSandboxCapabilityCaseV1 {
            request: filesystem_request,
            execution: filesystem_execution,
        },
        LawLabSandboxCapabilityCaseV1 {
            request: structured_request,
            execution: structured_execution,
        },
        fixture_cleanup_root_sha256,
    )?;
    std::io::stdout().write_all(&report.canonical_bytes()?)?;
    Ok(())
}

fn capability_request_v1(
    executor: &LawLabSandboxExecutorManifestV1,
    source: &LawLabTreeManifestV1,
    label: &str,
    domain: LawLabProbeDomainV1,
    operations: Vec<LawLabSandboxOperationV1>,
) -> Result<LawLabSandboxRequestV1, Box<dyn Error>> {
    Ok(LawLabSandboxRequestV1::seal(LawLabSandboxRequestInputV1 {
        executor_manifest_root_sha256: executor.manifest_root_sha256.clone(),
        worker_sha256: executor.worker_sha256.clone(),
        candidate_root_sha256: root_v1(&format!("generated-capability-candidate:{label}"))?,
        version_space_root_sha256: root_v1(&format!("generated-capability-version-space:{label}"))?,
        durable_prediction_ledger_root_sha256: root_v1(&format!(
            "generated-capability-external-ledger:{label}"
        ))?,
        probe_root_sha256: root_v1(&format!("generated-capability-probe:{label}"))?,
        source_tree_root_sha256: source.tree_root_sha256.clone(),
        deterministic_seed_sha256: root_v1(&format!("generated-capability-seed:{label}"))?,
        domain,
        purpose: LawLabSandboxPurposeV1::GeneratedCapabilitySelfTest,
        surviving_hypothesis_count: 2,
        precommitted_prediction_count: 2,
        operations,
    })?)
}

fn root_v1(value: &str) -> Result<String, Box<dyn Error>> {
    canonical_json_sha256(&value).map_err(|error| io::Error::other(error).into())
}

struct CapabilityScratchV1 {
    root: PathBuf,
    source_store: PathBuf,
    workspace_store: PathBuf,
    instance_root_sha256: String,
    cleaned: bool,
}

impl CapabilityScratchV1 {
    fn create(parent: &Path) -> Result<Self, Box<dyn Error>> {
        fs::create_dir_all(parent)?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let instance_root_sha256 = canonical_json_sha256(&(
            "nando.law-lab-sandbox-capability-fixture.v1",
            std::process::id(),
            timestamp,
        ))
        .map_err(io::Error::other)?;
        let root = parent.join(&instance_root_sha256);
        fs::create_dir(&root)?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        let source_store = root.join("sources");
        let workspace_store = root.join("workspaces");
        for path in [&source_store, &workspace_store] {
            fs::create_dir(path)?;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
        }
        Ok(Self {
            root,
            source_store,
            workspace_store,
            instance_root_sha256,
            cleaned: false,
        })
    }

    fn seal_source(
        &self,
        label: &str,
        files: &[(&str, &[u8])],
    ) -> Result<LawLabTreeManifestV1, Box<dyn Error>> {
        let staging = self.root.join(format!("staging-{label}"));
        fs::create_dir(&staging)?;
        fs::set_permissions(&staging, fs::Permissions::from_mode(0o700))?;
        for (relative_path, bytes) in files {
            let path = staging.join(relative_path);
            if let Some(parent) = path.parent()
                && parent != staging
            {
                fs::create_dir_all(parent)?;
            }
            let mut file = File::create(&path)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        }
        let manifest = LawLabTreeManifestV1::scan(&staging, LAW_LAB_MAX_INPUT_BYTES_V1)?;
        fs::rename(&staging, self.source_store.join(&manifest.tree_root_sha256))?;
        File::open(&self.source_store)?.sync_all()?;
        Ok(manifest)
    }

    fn cleanup(&mut self) -> Result<String, Box<dyn Error>> {
        fs::remove_dir_all(&self.root)?;
        if self.root.try_exists()? {
            return Err(io::Error::other("capability scratch cleanup failed").into());
        }
        self.cleaned = true;
        Ok(canonical_json_sha256(&(
            "nando.law-lab-sandbox-capability-fixture-cleanup.v1",
            self.instance_root_sha256.as_str(),
            true,
        ))
        .map_err(io::Error::other)?)
    }
}

impl Drop for CapabilityScratchV1 {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
