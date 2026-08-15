struct TestEnvironmentV1 {
    root: PathBuf,
    journal_store: PathBuf,
    workspace_store: PathBuf,
}

impl TestEnvironmentV1 {
    fn new(label: &str) -> Self {
        let sequence = TEST_SEQUENCE_V1.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nando-k2-inquiry-{label}-{}-{sequence}",
            std::process::id()
        ));
        let journal_store = root.join("journals");
        let workspace_store = root.join("workspaces");
        fs::create_dir_all(&journal_store).expect("create journal store");
        fs::create_dir_all(&workspace_store).expect("create workspace store");
        Self {
            root,
            journal_store,
            workspace_store,
        }
    }
}

impl Drop for TestEnvironmentV1 {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

struct ProcessBinariesV1 {
    selector: PathBuf,
    baseline: PathBuf,
    verifier: PathBuf,
    worker: PathBuf,
    observer: PathBuf,
    selector_sha256: String,
    baseline_sha256: String,
    verifier_sha256: String,
    worker_sha256: String,
    observer_sha256: String,
}

impl ProcessBinariesV1 {
    fn from_cargo() -> Self {
        let selector = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-selector"));
        let baseline = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-baseline"));
        let verifier = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-verifier"));
        let worker = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-worker"));
        let observer = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-observer"));
        Self {
            selector_sha256: composition_sha256_file_v1(&selector).expect("selector sha"),
            baseline_sha256: composition_sha256_file_v1(&baseline).expect("baseline sha"),
            verifier_sha256: composition_sha256_file_v1(&verifier).expect("verifier sha"),
            worker_sha256: composition_sha256_file_v1(&worker).expect("worker sha"),
            observer_sha256: composition_sha256_file_v1(&observer).expect("observer sha"),
            selector,
            baseline,
            verifier,
            worker,
            observer,
        }
    }

    fn assert_pairwise_distinct(&self) {
        let orchestrator = std::env::current_exe().expect("orchestrator executable");
        let roots = [
            self.selector_sha256.clone(),
            self.baseline_sha256.clone(),
            self.verifier_sha256.clone(),
            self.worker_sha256.clone(),
            self.observer_sha256.clone(),
            composition_sha256_file_v1(&orchestrator).expect("orchestrator sha"),
        ];
        assert_eq!(roots.iter().collect::<BTreeSet<_>>().len(), roots.len());
    }
}

fn run_isolated_protocol_v1<T, U>(executable: &Path, request: &T) -> U
where
    T: Serialize,
    U: serde::de::DeserializeOwned + Serialize,
{
    let guest = "/nando/bin/process";
    let mut command = Command::new("/usr/bin/bwrap");
    command.args([
        "--unshare-all",
        "--die-with-parent",
        "--new-session",
        "--cap-drop",
        "ALL",
        "--clearenv",
    ]);
    for path in ["/usr", "/lib", "/lib64"] {
        if Path::new(path).exists() {
            command.args(["--ro-bind", path, path]);
        }
    }
    command
        .args(["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp"])
        .args(["--dir", "/nando", "--dir", "/nando/bin"])
        .arg("--ro-bind")
        .arg(executable)
        .arg(guest)
        .args(["--setenv", "HOME", "/tmp", "--setenv", "LANG", "C"])
        .args(["--", "/usr/bin/prlimit", "--cpu=10:10"])
        .args(["--as=536870912:536870912", "--nproc=32:32"])
        .args(["--fsize=33554432:33554432", "--", guest])
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn isolated inquiry process");
    child
        .stdin
        .take()
        .expect("protocol stdin")
        .write_all(&composition_bytes_v1(request).expect("protocol request bytes"))
        .expect("write protocol request");
    let mut stdout = child.stdout.take().expect("protocol stdout");
    let mut stderr = child.stderr.take().expect("protocol stderr");
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .read_to_end(&mut bytes)
            .expect("read protocol stdout");
        bytes
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr
            .read_to_end(&mut bytes)
            .expect("read protocol stderr");
        bytes
    });
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll inquiry process") {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "isolated inquiry process timed out"
        );
        thread::sleep(Duration::from_millis(5));
    };
    let stdout = stdout_reader.join().expect("join stdout reader");
    let stderr = stderr_reader.join().expect("join stderr reader");
    assert!(
        status.success(),
        "isolated inquiry process failed: {}",
        String::from_utf8_lossy(&stderr)
    );
    composition_decode_v1(&stdout).expect("decode isolated inquiry output")
}
