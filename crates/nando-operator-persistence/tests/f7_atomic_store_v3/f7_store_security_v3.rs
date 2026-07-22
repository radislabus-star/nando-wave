use std::fs;

use crate::f7_support::FixtureV3;
use nando_operator_persistence::{GENERATION_STORE_SLOT_A_FILE_V3, GenerationCheckpointStoreV3};

#[cfg(unix)]
#[test]
fn symlink_slot_is_quarantined_without_touching_its_target() {
    use std::os::unix::fs::symlink;

    let fixture = FixtureV3::new("symlink");
    fs::create_dir_all(&fixture.directory).expect("directory");
    let outside = fixture.directory.with_extension("outside");
    fs::write(&outside, b"do-not-read-or-change").expect("outside");
    symlink(
        &outside,
        fixture.directory.join(GENERATION_STORE_SLOT_A_FILE_V3),
    )
    .expect("symlink");

    let store = GenerationCheckpointStoreV3::open(&fixture.directory).expect("store");
    let restored = store.restore().expect("restore");
    assert!(restored.checkpoint().is_none());
    assert_eq!(restored.quarantined_files().len(), 1);
    assert_eq!(
        fs::read(&outside).expect("outside bytes"),
        b"do-not-read-or-change"
    );
    fs::remove_file(outside).expect("outside cleanup");
}

#[cfg(unix)]
#[test]
fn broken_temporary_symlink_is_quarantined_before_publish() {
    use std::os::unix::fs::symlink;

    let mut fixture = FixtureV3::new("broken-temporary-symlink");
    fixture.append_support();
    fs::create_dir_all(&fixture.directory).expect("directory");
    symlink(
        fixture.directory.join("missing-target"),
        fixture
            .directory
            .join(format!(".{GENERATION_STORE_SLOT_A_FILE_V3}.new")),
    )
    .expect("broken temporary symlink");

    let store = GenerationCheckpointStoreV3::open(&fixture.directory).expect("store");
    let restored = store.restore().expect("restore");
    assert!(restored.checkpoint().is_none());
    assert_eq!(restored.quarantined_files().len(), 1);
    store
        .publish(&fixture.checkpoint(1))
        .expect("publish after quarantine");
}
