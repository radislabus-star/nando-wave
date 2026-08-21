use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::super::composition_sha256_bytes_v1;
use super::*;

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn development_known_answer_vectors_are_byte_identical() {
    let root = vector_root_v1();
    let artifact_bytes =
        fs::read(root.join("development-stored-artifacts.vector.json")).expect("artifact vector");
    let artifacts: Vec<K2UncertaintyDevelopmentRehearsalStoredArtifactV1> =
        uncertainty_decode_v1(&artifact_bytes).expect("decode artifacts");
    assert_eq!(artifacts.len(), 34);
    assert!(artifacts.iter().all(|value| value.validate().is_ok()));
    assert_eq!(
        uncertainty_bytes_v1(&artifacts).expect("artifact bytes"),
        artifact_bytes
    );
    assert_eq!(
        composition_sha256_bytes_v1(&artifact_bytes),
        "5e9538235b1a036a4dd150cfb08f3c912df4b384ae484ddb2da044f31357093a"
    );

    let split_bytes =
        fs::read(root.join("development-split-receipt.vector.json")).expect("split vector");
    let split: K2UncertaintyDevelopmentRehearsalSplitReceiptV1 =
        uncertainty_decode_v1(&split_bytes).expect("decode split");
    assert_eq!(
        split.expected_root().expect("split root"),
        split.split_receipt_root_sha256
    );
    assert_eq!(
        uncertainty_bytes_v1(&split).expect("split bytes"),
        split_bytes
    );
    assert_eq!(
        split.private_reconstruction_root_sha256,
        "6f4f865654612db327dcad1503790e151e881e8d888abc2a5e16df0545bad8ef"
    );
    assert_eq!(
        split.split_receipt_root_sha256,
        "12199a9d2bdbe3172b17e571bbd056f45723e23c3765564da59794eb804c67e5"
    );

    let owner_bytes =
        fs::read(root.join("development-owner-receipt.vector.json")).expect("owner vector");
    let owner: K2UncertaintyDevelopmentRehearsalOwnerReceiptV1 =
        uncertainty_decode_v1(&owner_bytes).expect("decode owner");
    assert_eq!(
        owner.expected_root().expect("owner root"),
        owner.receipt_root_sha256
    );
    assert_eq!(
        uncertainty_bytes_v1(&owner).expect("owner bytes"),
        owner_bytes
    );
    assert_eq!(
        owner.receipt_root_sha256,
        "0b413483f55c213604cca3bac9821fea79d0067692e4abb0b89dd1e193c6f4c3"
    );
}

#[test]
fn immutable_publication_covers_72_boundaries() {
    for publication_id in 0..36_u64 {
        let relative = format!("object-{publication_id:02}.json");
        let bytes = format!("{{\"publication_id\":{publication_id}}}").into_bytes();

        let before = TestDirectoryV1::new();
        assert!(
            publish_immutable_file_v1(
                before.path(),
                &relative,
                &bytes,
                0o400,
                publication_id,
                K2UncertaintyImmutablePublicationFaultV1::BeforePublish(publication_id),
            )
            .is_err()
        );
        assert!(!before.path().join(&relative).exists());
        assert!(
            fs::read_dir(before.path())
                .expect("before directory")
                .next()
                .is_none()
        );

        let after = TestDirectoryV1::new();
        assert!(
            publish_immutable_file_v1(
                after.path(),
                &relative,
                &bytes,
                0o400,
                publication_id,
                K2UncertaintyImmutablePublicationFaultV1::AfterPublish(publication_id),
            )
            .is_err()
        );
        recover_linked_publication_temp_v1(after.path(), &relative, &bytes, 0o400, publication_id)
            .expect("recover after-publish fault");
        let recovered = read_immutable_file_v1(after.path(), &relative, 0o400, bytes.len())
            .expect("read recovered final");
        assert_eq!(recovered.bytes, bytes);
    }
}

#[test]
fn immutable_reader_rejects_symlink_and_foreign_hard_link() {
    let root = TestDirectoryV1::new();
    let outside = root.path().join("outside.json");
    fs::write(&outside, b"outside").expect("outside bytes");
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o400)).expect("outside mode");
    std::os::unix::fs::symlink(&outside, root.path().join("symlink.json")).expect("create symlink");
    assert!(read_immutable_file_v1(root.path(), "symlink.json", 0o400, 32).is_err());

    fs::hard_link(&outside, root.path().join("linked.json")).expect("create hard link");
    assert!(read_immutable_file_v1(root.path(), "linked.json", 0o400, 32).is_err());
}

fn vector_root_v1() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("plans/effect-law-unification-v1/evidence")
        .join("K2_SELF_FORMED_UNCERTAINTY_V5_R8B_PREFLIGHT_V2")
        .join("preimplementation-development-byte-vectors")
}

struct TestDirectoryV1(PathBuf);

impl TestDirectoryV1 {
    fn new() -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "nando-r8b-development-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create test directory");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
            .expect("chmod test directory");
        let path = fs::canonicalize(path).expect("canonical test directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectoryV1 {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700));
        for entry in walk_files_v1(&self.0) {
            let _ = fs::set_permissions(entry, fs::Permissions::from_mode(0o600));
        }
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn walk_files_v1(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(path) = pending.pop() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    pending.push(path);
                } else {
                    files.push(path);
                }
            }
        }
    }
    files
}
