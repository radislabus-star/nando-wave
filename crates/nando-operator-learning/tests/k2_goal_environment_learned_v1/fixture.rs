struct LearnedFixtureV1 {
    root: PathBuf,
    source_store: PathBuf,
    workspace_store: PathBuf,
    private_store: PathBuf,
    learned_journal_store: PathBuf,
    v1_journal_store: PathBuf,
    support: K2SupportWorldSetV1,
    target_pre: LawLabTreeManifestV1,
    target_expected: LawLabTreeManifestV1,
    target_goal_store_snapshot_root_sha256: String,
}

impl LearnedFixtureV1 {
    fn new() -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let parent = std::env::var_os("NANDO_K2_GOAL_TEST_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .expect("current directory")
                    .join("target/k2-goal-environment-learned-tests")
            });
        fs::create_dir_all(&parent).expect("test parent");
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).expect("parent mode");
        let fixture_root = parent.join(format!("{}-{sequence}", std::process::id()));
        fs::create_dir(&fixture_root).expect("fixture root");
        fs::set_permissions(&fixture_root, fs::Permissions::from_mode(0o700)).expect("root mode");
        let source_store = fixture_root.join("sources");
        let workspace_store = fixture_root.join("workspaces");
        let private_store = fixture_root.join("private");
        let learned_journal_store = fixture_root.join("learned-journals");
        let v1_journal_store = fixture_root.join("v1-journals");
        for path in [
            &source_store,
            &workspace_store,
            &private_store,
            &learned_journal_store,
            &v1_journal_store,
        ] {
            fs::create_dir(path).expect("fixture directory");
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).expect("directory mode");
        }
        let mut worlds = Vec::new();
        for ordinal in 0..3_u64 {
            let staging = fixture_root.join(format!("support-{ordinal}"));
            fs::create_dir(&staging).expect("support staging");
            fs::set_permissions(&staging, fs::Permissions::from_mode(0o700))
                .expect("support staging mode");
            write_file(
                &staging.join("input.bin"),
                &vec![b'a' + ordinal as u8; 11 + ordinal as usize * 6],
            );
            write_file(
                &staging.join("obsolete.bin"),
                &vec![b'k' + ordinal as u8; 7 + ordinal as usize * 8],
            );
            match ordinal {
                0 => write_file(&staging.join("distractor-a.txt"), b"a"),
                1 => {
                    fs::create_dir(staging.join("nested")).expect("nested");
                    fs::set_permissions(staging.join("nested"), fs::Permissions::from_mode(0o700))
                        .expect("nested mode");
                    write_file(&staging.join("nested/distractor-b.txt"), b"bb");
                }
                _ => {
                    write_file(&staging.join("distractor-c.txt"), b"ccc");
                    write_file(&staging.join("distractor-d.txt"), b"dddd");
                }
            }
            let manifest = LawLabTreeManifestV1::scan(&staging, K2_LEARNED_MAX_TREE_BYTES_V1)
                .expect("support manifest");
            fs::rename(&staging, source_store.join(&manifest.tree_root_sha256))
                .expect("seal support source");
            worlds.push(
                K2SupportWorldV1::seal(
                    ordinal,
                    manifest,
                    root(&format!("support-provenance-{ordinal}")),
                )
                .expect("support world"),
            );
        }
        let support = K2SupportWorldSetV1::seal(worlds).expect("support world set");

        let target_staging = fixture_root.join("target-staging");
        fs::create_dir(&target_staging).expect("target staging");
        fs::set_permissions(&target_staging, fs::Permissions::from_mode(0o700))
            .expect("target staging mode");
        let target_input = vec![b'z'; 37];
        write_file(&target_staging.join("input.bin"), &target_input);
        write_file(&target_staging.join("obsolete.bin"), &[b'y'; 41]);
        fs::create_dir(target_staging.join("target-nested")).expect("target nested");
        fs::set_permissions(
            target_staging.join("target-nested"),
            fs::Permissions::from_mode(0o700),
        )
        .expect("target nested mode");
        write_file(
            &target_staging.join("target-nested/distractor-e.txt"),
            b"eeeee",
        );
        write_file(&target_staging.join("distractor-f.txt"), b"ffffff");
        let target_pre = LawLabTreeManifestV1::scan(&target_staging, K2_LEARNED_MAX_TREE_BYTES_V1)
            .expect("target pre manifest");
        fs::rename(
            &target_staging,
            source_store.join(&target_pre.tree_root_sha256),
        )
        .expect("seal target source");

        let expected_store = fixture_root.join("target-expected");
        fs::create_dir(&expected_store).expect("expected store");
        write_file(&expected_store.join("input.bin"), &target_input);
        write_file(&expected_store.join("selected.bin"), &target_input);
        write_file(&expected_store.join("obsolete.bin"), &[b'y'; 41]);
        fs::create_dir(expected_store.join("target-nested")).expect("expected nested");
        write_file(
            &expected_store.join("target-nested/distractor-e.txt"),
            b"eeeee",
        );
        write_file(&expected_store.join("distractor-f.txt"), b"ffffff");
        let target_expected =
            LawLabTreeManifestV1::scan(&expected_store, K2_LEARNED_MAX_TREE_BYTES_V1)
                .expect("target expected manifest");
        let target_goal_store_snapshot_root_sha256 =
            canonical_json_sha256(&target_expected).expect("goal store snapshot");
        Self {
            root: fixture_root,
            source_store,
            workspace_store,
            private_store,
            learned_journal_store,
            v1_journal_store,
            support,
            target_pre,
            target_expected,
            target_goal_store_snapshot_root_sha256,
        }
    }
}

impl Drop for LearnedFixtureV1 {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_file(path: &Path, bytes: &[u8]) {
    let mut file = File::create(path).expect("fixture file");
    file.write_all(bytes).expect("fixture write");
    file.sync_all().expect("fixture sync");
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).expect("fixture mode");
}

#[test]
fn frozen_roots_are_sha256_and_authority_is_false() {
    let authority = K2AuthorityBoundaryV1::authority_free_v1();
    authority.validate().expect("authority-free boundary");
    assert!(valid_nonzero_sha256(&root("learned-unit-root")));
}
