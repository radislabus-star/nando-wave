//! Bounded, fail-closed experiments for distinguishing competing laws.
//!
//! This crate is deliberately colder than the runtime. A probe may reduce a
//! version space and emit a content-addressed receipt, but it cannot create an
//! ACTIVE package or grant execution authority. The executor is intentionally
//! small and closed over two disposable environments: filesystem and Git.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const LAB_SCHEMA_V1: &str = "nando.experimental-lab.v1";
pub const LAB_RECEIPT_SCHEMA_V1: &str = "nando.experimental-lab-receipt.v1";
pub const LAW_CERTIFICATE_SCHEMA_V1: &str = "nando.law-certificate.v1";
pub const MAX_FILES: usize = 64;
pub const MAX_FILE_BYTES: usize = 64 * 1024;
pub const MAX_HYPOTHESES: usize = 64;
pub const MAX_PROBE_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentKind {
    Filesystem,
    Git,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Copy,
    Delete,
    GitRename,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InitialState {
    pub files: BTreeMap<String, Vec<u8>>,
    pub structured_reference_paths: Vec<String>,
}

impl InitialState {
    pub fn new(files: BTreeMap<String, Vec<u8>>) -> Result<Self, LabError> {
        Self::with_references(files, Vec::new())
    }

    pub fn with_references(
        files: BTreeMap<String, Vec<u8>>,
        structured_reference_paths: Vec<String>,
    ) -> Result<Self, LabError> {
        validate_files(&files)?;
        if structured_reference_paths.len() > MAX_FILES {
            return Err(LabError::BoundExceeded("structured reference paths"));
        }
        for path in &structured_reference_paths {
            validate_relative_path(path)?;
        }
        Ok(Self {
            files,
            structured_reference_paths,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Action {
    pub kind: ActionKind,
    pub source: String,
    pub destination: Option<String>,
}

impl Action {
    pub fn copy(source: impl Into<String>, destination: impl Into<String>) -> Self {
        Self {
            kind: ActionKind::Copy,
            source: source.into(),
            destination: Some(destination.into()),
        }
    }

    pub fn delete(source: impl Into<String>) -> Self {
        Self {
            kind: ActionKind::Delete,
            source: source.into(),
            destination: None,
        }
    }

    pub fn git_rename(source: impl Into<String>, destination: impl Into<String>) -> Self {
        Self {
            kind: ActionKind::GitRename,
            source: source.into(),
            destination: Some(destination.into()),
        }
    }

    fn validate(&self) -> Result<(), LabError> {
        validate_relative_path(&self.source)?;
        match self.kind {
            ActionKind::Copy | ActionKind::GitRename => {
                let destination = self
                    .destination
                    .as_deref()
                    .ok_or(LabError::InvalidAction("destination is required"))?;
                validate_relative_path(destination)?;
                if self.source == destination {
                    return Err(LabError::InvalidAction("source equals destination"));
                }
            }
            ActionKind::Delete => {
                if self.destination.is_some() {
                    return Err(LabError::InvalidAction("delete has no destination"));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct OutcomeVector {
    pub tree_changed: bool,
    pub index_changed: bool,
    pub structured_references_changed: bool,
    pub source_present: bool,
    pub destination_present: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hypothesis {
    pub id: String,
    pub prediction: OutcomeVector,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LabProbe {
    pub schema: String,
    pub id: String,
    pub environment: EnvironmentKind,
    pub initial_state: InitialState,
    pub action: Action,
    pub hypotheses: Vec<Hypothesis>,
    pub safety_budget: SafetyBudget,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SafetyBudget {
    pub max_files: usize,
    pub max_file_bytes: usize,
    pub max_wall_time_ms: u64,
    pub network_enabled: bool,
    pub production_mounts: bool,
}

impl Default for SafetyBudget {
    fn default() -> Self {
        Self {
            max_files: MAX_FILES,
            max_file_bytes: MAX_FILE_BYTES,
            max_wall_time_ms: 5_000,
            network_enabled: false,
            production_mounts: false,
        }
    }
}

impl LabProbe {
    pub fn new(
        id: impl Into<String>,
        environment: EnvironmentKind,
        initial_state: InitialState,
        action: Action,
        hypotheses: Vec<Hypothesis>,
    ) -> Result<Self, LabError> {
        let probe = Self {
            schema: LAB_SCHEMA_V1.to_owned(),
            id: id.into(),
            environment,
            initial_state,
            action,
            hypotheses,
            safety_budget: SafetyBudget::default(),
        };
        probe.validate()?;
        Ok(probe)
    }

    pub fn validate(&self) -> Result<(), LabError> {
        if self.schema != LAB_SCHEMA_V1 || self.id.trim().is_empty() {
            return Err(LabError::InvalidProbe("schema or id"));
        }
        self.action.validate()?;
        validate_files(&self.initial_state.files)?;
        if self.initial_state.files.len() > self.safety_budget.max_files {
            return Err(LabError::BoundExceeded("files"));
        }
        if self
            .initial_state
            .files
            .values()
            .any(|bytes| bytes.len() > self.safety_budget.max_file_bytes)
        {
            return Err(LabError::BoundExceeded("file bytes"));
        }
        if self.safety_budget.network_enabled || self.safety_budget.production_mounts {
            return Err(LabError::UnsafePolicy);
        }
        if self.safety_budget.max_wall_time_ms == 0 {
            return Err(LabError::UnsafePolicy);
        }
        if self.hypotheses.is_empty() || self.hypotheses.len() > MAX_HYPOTHESES {
            return Err(LabError::BoundExceeded("hypotheses"));
        }
        let mut ids = BTreeSet::new();
        for hypothesis in &self.hypotheses {
            if hypothesis.id.trim().is_empty() || !ids.insert(&hypothesis.id) {
                return Err(LabError::InvalidProbe("duplicate hypothesis id"));
            }
        }
        let expected_environment = match self.action.kind {
            ActionKind::GitRename => EnvironmentKind::Git,
            ActionKind::Copy | ActionKind::Delete => EnvironmentKind::Filesystem,
        };
        if self.environment != expected_environment {
            return Err(LabError::InvalidProbe("action/environment mismatch"));
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, LabError> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|_| LabError::CanonicalEncoding)
    }

    pub fn digest_sha256(&self) -> Result<String, LabError> {
        Ok(sha256(&self.canonical_bytes()?))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProbeSelection {
    pub probe_id: String,
    pub partition_sizes: Vec<usize>,
    pub information_gain_pairs: u64,
    pub semantic_novelty: u64,
    pub execution_cost: u64,
    pub safety_risk: u64,
    pub nondeterminism: u64,
    pub score: i128,
}

/// Selects a probe by predicted outcome partition, never by executing a
/// candidate hypothesis. The pair score rewards balanced separation.
pub fn select_probe(probes: &[LabProbe]) -> Result<ProbeSelection, LabError> {
    if probes.is_empty() {
        return Err(LabError::NoProbe);
    }
    let mut best: Option<ProbeSelection> = None;
    for probe in probes {
        probe.validate()?;
        let mut partitions: BTreeMap<OutcomeVector, usize> = BTreeMap::new();
        for hypothesis in &probe.hypotheses {
            *partitions.entry(hypothesis.prediction).or_default() += 1;
        }
        let total = probe.hypotheses.len() as u64;
        let sum_squares = partitions
            .values()
            .map(|size| (*size as u64).saturating_mul(*size as u64))
            .sum::<u64>();
        let information_gain_pairs = total
            .saturating_mul(total.saturating_sub(1))
            .saturating_sub(sum_squares.saturating_sub(total));
        let selection = ProbeSelection {
            probe_id: probe.id.clone(),
            partition_sizes: partitions.values().copied().collect(),
            information_gain_pairs,
            semantic_novelty: 1,
            execution_cost: 1,
            safety_risk: 0,
            nondeterminism: 0,
            score: i128::from(information_gain_pairs) * 100 + 10 - 1,
        };
        if best.as_ref().is_none_or(|current| {
            selection.score > current.score
                || (selection.score == current.score && selection.probe_id < current.probe_id)
        }) {
            best = Some(selection);
        }
    }
    best.ok_or(LabError::NoProbe)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservedState {
    pub tree_root_sha256: String,
    pub index_root_sha256: Option<String>,
    pub structured_references_root_sha256: Option<String>,
    pub outcome: OutcomeVector,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UniqueLawCandidate {
    pub law_id: String,
    pub hypothesis_id: String,
    pub prediction: OutcomeVector,
    pub lab_probe_digest_sha256: String,
    pub authority_granted: bool,
    pub active_package_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CleanupReceipt {
    pub cleanup_attempted: bool,
    pub cleanup_succeeded: bool,
    pub cleanup_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IndependentNaturalHoldoutReceipt {
    pub schema: String,
    pub source_ref: String,
    pub source_owner: String,
    pub source_record_sha256: String,
    pub observed: OutcomeVector,
}

impl IndependentNaturalHoldoutReceipt {
    pub fn new(
        source_ref: impl Into<String>,
        source_owner: impl Into<String>,
        source_record_sha256: impl Into<String>,
        observed: OutcomeVector,
    ) -> Result<Self, LabError> {
        let receipt = Self {
            schema: "nando.independent-natural-holdout-receipt.v1".to_owned(),
            source_ref: source_ref.into(),
            source_owner: source_owner.into(),
            source_record_sha256: source_record_sha256.into(),
            observed,
        };
        if receipt.source_ref.trim().is_empty()
            || receipt.source_ref.starts_with("lab/")
            || receipt.source_owner.trim().is_empty()
            || !is_sha256(&receipt.source_record_sha256)
        {
            return Err(LabError::InvalidLawCertificate);
        }
        Ok(receipt)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LabProbeReceipt {
    pub schema: String,
    pub probe_digest_sha256: String,
    pub executor: String,
    pub executor_version: String,
    pub observed: ObservedState,
    pub surviving_hypotheses: Vec<String>,
    pub unique_law_candidate: Option<UniqueLawCandidate>,
    pub cleanup: CleanupReceipt,
    pub lab_probe: bool,
    pub production_authority: bool,
    pub receipt_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LawCertificate {
    pub schema: String,
    pub law_id: String,
    pub lab_probe_digest_sha256: String,
    pub natural_holdout_ref: String,
    pub natural_holdout_record_sha256: String,
    pub natural_holdout_source_owner: String,
    pub natural_holdout_outcome: OutcomeVector,
    pub independent_source: bool,
    pub authority_granted: bool,
    pub active_package_allowed: bool,
    pub certificate_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LabError {
    InvalidProbe(&'static str),
    InvalidAction(&'static str),
    InvalidPath,
    BoundExceeded(&'static str),
    UnsafePolicy,
    CanonicalEncoding,
    NoProbe,
    ExecutorUnavailable(String),
    ExecutorFailed(String),
    NoUniqueLaw,
    NaturalHoldoutMismatch,
    NaturalHoldoutNotIndependent,
    InvalidLawCertificate,
}

impl fmt::Display for LabError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProbe(reason) => write!(formatter, "invalid probe: {reason}"),
            Self::InvalidAction(reason) => write!(formatter, "invalid action: {reason}"),
            Self::InvalidPath => formatter.write_str("invalid relative path"),
            Self::BoundExceeded(kind) => write!(formatter, "bound exceeded: {kind}"),
            Self::UnsafePolicy => formatter.write_str("unsafe laboratory policy"),
            Self::CanonicalEncoding => formatter.write_str("canonical encoding failed"),
            Self::NoProbe => formatter.write_str("no probe available"),
            Self::ExecutorUnavailable(reason) => {
                write!(formatter, "executor unavailable: {reason}")
            }
            Self::ExecutorFailed(reason) => write!(formatter, "executor failed: {reason}"),
            Self::NoUniqueLaw => formatter.write_str("probe did not identify a unique law"),
            Self::NaturalHoldoutMismatch => formatter.write_str("natural holdout mismatch"),
            Self::NaturalHoldoutNotIndependent => {
                formatter.write_str("natural holdout is not independent")
            }
            Self::InvalidLawCertificate => formatter.write_str("invalid law certificate"),
        }
    }
}

impl std::error::Error for LabError {}

pub fn execute_probe(probe: &LabProbe) -> Result<LabProbeReceipt, LabError> {
    probe.validate()?;
    let probe_digest = probe.digest_sha256()?;
    let workspace = DisposableWorkspace::create()?;
    let result = execute_in_workspace(probe, workspace.path());
    let cleanup = workspace.cleanup();
    let (observed, executor, executor_version) = result?;
    let surviving_hypotheses = probe
        .hypotheses
        .iter()
        .filter(|hypothesis| hypothesis.prediction == observed.outcome)
        .map(|hypothesis| hypothesis.id.clone())
        .collect::<Vec<_>>();
    let unique_law_candidate = if surviving_hypotheses.len() == 1 {
        let hypothesis_id = surviving_hypotheses[0].clone();
        let hypothesis = probe
            .hypotheses
            .iter()
            .find(|candidate| candidate.id == hypothesis_id)
            .ok_or(LabError::NoUniqueLaw)?;
        Some(UniqueLawCandidate {
            law_id: format!("{}::{hypothesis_id}", probe.id),
            hypothesis_id,
            prediction: hypothesis.prediction,
            lab_probe_digest_sha256: probe_digest.clone(),
            authority_granted: false,
            active_package_allowed: false,
        })
    } else {
        None
    };
    let mut receipt = LabProbeReceipt {
        schema: LAB_RECEIPT_SCHEMA_V1.to_owned(),
        probe_digest_sha256: probe_digest,
        executor,
        executor_version,
        observed,
        surviving_hypotheses,
        unique_law_candidate,
        cleanup,
        lab_probe: true,
        production_authority: false,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = receipt_digest(&receipt)?;
    Ok(receipt)
}

pub fn certify_natural_holdout(
    candidate: &UniqueLawCandidate,
    evidence: IndependentNaturalHoldoutReceipt,
) -> Result<LawCertificate, LabError> {
    if evidence.source_record_sha256 == candidate.lab_probe_digest_sha256 {
        return Err(LabError::NaturalHoldoutNotIndependent);
    }
    if evidence.observed != candidate.prediction {
        return Err(LabError::NaturalHoldoutMismatch);
    }
    let mut certificate = LawCertificate {
        schema: LAW_CERTIFICATE_SCHEMA_V1.to_owned(),
        law_id: candidate.law_id.clone(),
        lab_probe_digest_sha256: candidate.lab_probe_digest_sha256.clone(),
        natural_holdout_ref: evidence.source_ref,
        natural_holdout_record_sha256: evidence.source_record_sha256,
        natural_holdout_source_owner: evidence.source_owner,
        natural_holdout_outcome: evidence.observed,
        independent_source: true,
        authority_granted: false,
        active_package_allowed: false,
        certificate_sha256: String::new(),
    };
    certificate.certificate_sha256 = certificate_digest(&certificate)?;
    Ok(certificate)
}

pub fn git_rename_probe() -> Result<LabProbe, LabError> {
    let mut files = BTreeMap::new();
    files.insert("a.txt".to_owned(), b"hello\n".to_vec());
    files.insert("refs.json".to_owned(), br#"{"path":"a.txt"}\n"#.to_vec());
    LabProbe::new(
        "git-rename-law",
        EnvironmentKind::Git,
        InitialState::with_references(files, vec!["refs.json".to_owned()])?,
        Action::git_rename("a.txt", "b.txt"),
        vec![
            Hypothesis {
                id: "tree-only".to_owned(),
                prediction: OutcomeVector {
                    tree_changed: true,
                    index_changed: false,
                    structured_references_changed: false,
                    source_present: false,
                    destination_present: true,
                },
            },
            Hypothesis {
                id: "tree-and-index".to_owned(),
                prediction: OutcomeVector {
                    tree_changed: true,
                    index_changed: true,
                    structured_references_changed: false,
                    source_present: false,
                    destination_present: true,
                },
            },
            Hypothesis {
                id: "tree-index-and-refs".to_owned(),
                prediction: OutcomeVector {
                    tree_changed: true,
                    index_changed: true,
                    structured_references_changed: true,
                    source_present: false,
                    destination_present: true,
                },
            },
        ],
    )
}

pub fn filesystem_copy_probe() -> Result<LabProbe, LabError> {
    let mut files = BTreeMap::new();
    files.insert("source.txt".to_owned(), b"copy me\n".to_vec());
    LabProbe::new(
        "filesystem-copy-law",
        EnvironmentKind::Filesystem,
        InitialState::new(files)?,
        Action::copy("source.txt", "copy.txt"),
        vec![
            Hypothesis {
                id: "no-op".to_owned(),
                prediction: OutcomeVector {
                    source_present: true,
                    ..OutcomeVector::default()
                },
            },
            Hypothesis {
                id: "copy-preserves-source".to_owned(),
                prediction: OutcomeVector {
                    tree_changed: true,
                    source_present: true,
                    destination_present: true,
                    ..OutcomeVector::default()
                },
            },
            Hypothesis {
                id: "move-removes-source".to_owned(),
                prediction: OutcomeVector {
                    tree_changed: true,
                    source_present: false,
                    destination_present: true,
                    ..OutcomeVector::default()
                },
            },
        ],
    )
}

pub fn filesystem_delete_probe() -> Result<LabProbe, LabError> {
    let mut files = BTreeMap::new();
    files.insert("remove.txt".to_owned(), b"remove me\n".to_vec());
    LabProbe::new(
        "filesystem-delete-law",
        EnvironmentKind::Filesystem,
        InitialState::new(files)?,
        Action::delete("remove.txt"),
        vec![
            Hypothesis {
                id: "no-op".to_owned(),
                prediction: OutcomeVector {
                    source_present: true,
                    ..OutcomeVector::default()
                },
            },
            Hypothesis {
                id: "delete-removes-source".to_owned(),
                prediction: OutcomeVector {
                    tree_changed: true,
                    source_present: false,
                    ..OutcomeVector::default()
                },
            },
            Hypothesis {
                id: "truncate-keeps-source".to_owned(),
                prediction: OutcomeVector {
                    tree_changed: true,
                    source_present: true,
                    ..OutcomeVector::default()
                },
            },
        ],
    )
}

fn execute_in_workspace(
    probe: &LabProbe,
    root: &Path,
) -> Result<(ObservedState, String, String), LabError> {
    write_initial_state(root, &probe.initial_state)?;
    let (executor, executor_version) = match probe.environment {
        EnvironmentKind::Filesystem => ("nando-lab-filesystem".to_owned(), "std-fs-v1".to_owned()),
        EnvironmentKind::Git => {
            init_git(root, probe.safety_budget.max_wall_time_ms)?;
            (
                "nando-lab-git".to_owned(),
                git_version(root, probe.safety_budget.max_wall_time_ms)?,
            )
        }
    };
    let before_tree = tree_root(root)?;
    let before_index = if probe.environment == EnvironmentKind::Git {
        Some(git_index_root(root, probe.safety_budget.max_wall_time_ms)?)
    } else {
        None
    };
    let before_refs = references_root(root, &probe.initial_state.structured_reference_paths)?;
    execute_action(
        root,
        &probe.action,
        probe.environment,
        probe.safety_budget.max_wall_time_ms,
    )?;
    let after_tree = tree_root(root)?;
    let after_index = if probe.environment == EnvironmentKind::Git {
        Some(git_index_root(root, probe.safety_budget.max_wall_time_ms)?)
    } else {
        None
    };
    let after_refs = references_root(root, &probe.initial_state.structured_reference_paths)?;
    let source_present = root.join(&probe.action.source).is_file();
    let destination_present = probe
        .action
        .destination
        .as_deref()
        .map(|path| root.join(path).is_file())
        .unwrap_or(false);
    let outcome = OutcomeVector {
        tree_changed: before_tree != after_tree,
        index_changed: before_index != after_index,
        structured_references_changed: before_refs != after_refs,
        source_present,
        destination_present,
    };
    Ok((
        ObservedState {
            tree_root_sha256: after_tree,
            index_root_sha256: after_index,
            structured_references_root_sha256: after_refs,
            outcome,
        },
        executor,
        executor_version,
    ))
}

fn write_initial_state(root: &Path, state: &InitialState) -> Result<(), LabError> {
    for (path, bytes) in &state.files {
        let target = root.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        let mut file = File::create(target).map_err(io_error)?;
        file.write_all(bytes).map_err(io_error)?;
    }
    Ok(())
}

fn execute_action(
    root: &Path,
    action: &Action,
    environment: EnvironmentKind,
    max_wall_time_ms: u64,
) -> Result<(), LabError> {
    let source = root.join(&action.source);
    match action.kind {
        ActionKind::Copy => {
            let destination = root.join(
                action
                    .destination
                    .as_deref()
                    .ok_or(LabError::InvalidAction("copy destination missing"))?,
            );
            fs::copy(source, destination).map_err(io_error)?;
        }
        ActionKind::Delete => fs::remove_file(source).map_err(io_error)?,
        ActionKind::GitRename => {
            if environment != EnvironmentKind::Git {
                return Err(LabError::InvalidAction("git rename outside Git"));
            }
            let destination = action
                .destination
                .as_deref()
                .ok_or(LabError::InvalidAction("rename destination missing"))?;
            let output = run_command_bounded(
                "git",
                &["mv", &action.source, destination],
                root,
                max_wall_time_ms,
            )?;
            if !output.status.success() {
                return Err(LabError::ExecutorFailed(
                    String::from_utf8_lossy(&output.stderr).into_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn init_git(root: &Path, max_wall_time_ms: u64) -> Result<(), LabError> {
    run_git(root, ["init", "-q"], max_wall_time_ms)?;
    run_git(
        root,
        ["config", "user.email", "lab@example.invalid"],
        max_wall_time_ms,
    )?;
    run_git(root, ["config", "user.name", "Nando Lab"], max_wall_time_ms)?;
    run_git(root, ["add", "."], max_wall_time_ms)?;
    run_git(
        root,
        ["commit", "-qm", "initial lab state"],
        max_wall_time_ms,
    )
}

fn git_version(root: &Path, max_wall_time_ms: u64) -> Result<String, LabError> {
    let output =
        run_command_bounded("git", &["--version"], root, max_wall_time_ms).map_err(|error| {
            match error {
                LabError::ExecutorFailed(reason) => LabError::ExecutorUnavailable(reason),
                other => other,
            }
        })?;
    if !output.status.success() {
        return Err(LabError::ExecutorUnavailable(
            "git --version failed".to_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn run_git<const N: usize>(
    root: &Path,
    args: [&str; N],
    max_wall_time_ms: u64,
) -> Result<(), LabError> {
    let output = run_command_bounded("git", &args, root, max_wall_time_ms)?;
    if !output.status.success() {
        return Err(LabError::ExecutorFailed(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    Ok(())
}

fn git_index_root(root: &Path, max_wall_time_ms: u64) -> Result<String, LabError> {
    let output = run_command_bounded("git", &["ls-files", "--stage"], root, max_wall_time_ms)?;
    if !output.status.success() {
        return Err(LabError::ExecutorFailed(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    Ok(sha256(&output.stdout))
}

fn run_command_bounded(
    program: &str,
    args: &[&str],
    root: &Path,
    max_wall_time_ms: u64,
) -> Result<Output, LabError> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| LabError::ExecutorUnavailable(error.to_string()))?;
    let deadline = Instant::now() + Duration::from_millis(max_wall_time_ms);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child
                    .wait_with_output()
                    .map_err(|error| LabError::ExecutorFailed(error.to_string()));
            }
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(LabError::ExecutorFailed(
                    "wall time budget exceeded".to_owned(),
                ));
            }
            Ok(None) => sleep(Duration::from_millis(1)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(LabError::ExecutorFailed(error.to_string()));
            }
        }
    }
}

fn tree_root(root: &Path) -> Result<String, LabError> {
    let mut entries = Vec::new();
    collect_tree(root, root, &mut entries)?;
    entries.sort();
    let mut bytes = Vec::new();
    for (path, digest) in entries {
        bytes.extend_from_slice(path.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(digest.as_bytes());
        bytes.push(b'\n');
    }
    Ok(sha256(&bytes))
}

fn collect_tree(
    root: &Path,
    current: &Path,
    entries: &mut Vec<(String, String)>,
) -> Result<(), LabError> {
    let directory = fs::read_dir(current).map_err(io_error)?;
    for entry in directory {
        let entry = entry.map_err(io_error)?;
        let path = entry.path();
        let relative = path.strip_prefix(root).map_err(|_| LabError::InvalidPath)?;
        if relative.components().next() == Some(Component::Normal(".git".as_ref())) {
            continue;
        }
        if path.is_dir() {
            collect_tree(root, &path, entries)?;
        } else if path.is_file() {
            let bytes = fs::read(&path).map_err(io_error)?;
            if bytes.len() > MAX_PROBE_BYTES {
                return Err(LabError::BoundExceeded("observed file bytes"));
            }
            let path_string = relative.to_string_lossy().into_owned();
            entries.push((path_string, sha256(&bytes)));
        }
    }
    Ok(())
}

fn references_root(root: &Path, paths: &[String]) -> Result<Option<String>, LabError> {
    if paths.is_empty() {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    for path in paths {
        let value = fs::read(root.join(path)).map_err(io_error)?;
        bytes.extend_from_slice(path.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&value);
        bytes.push(b'\n');
    }
    Ok(Some(sha256(&bytes)))
}

fn validate_files(files: &BTreeMap<String, Vec<u8>>) -> Result<(), LabError> {
    if files.len() > MAX_FILES {
        return Err(LabError::BoundExceeded("files"));
    }
    for (path, bytes) in files {
        validate_relative_path(path)?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err(LabError::BoundExceeded("file bytes"));
        }
    }
    Ok(())
}

fn validate_relative_path(path: &str) -> Result<(), LabError> {
    let candidate = Path::new(path);
    if path.is_empty()
        || path.contains('\0')
        || candidate.is_absolute()
        || candidate
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        return Err(LabError::InvalidPath);
    }
    Ok(())
}

fn receipt_digest(receipt: &LabProbeReceipt) -> Result<String, LabError> {
    let mut value = receipt.clone();
    value.receipt_sha256.clear();
    serde_json::to_vec(&value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| LabError::CanonicalEncoding)
}

fn certificate_digest(certificate: &LawCertificate) -> Result<String, LabError> {
    let mut value = certificate.clone();
    value.certificate_sha256.clear();
    serde_json::to_vec(&value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| LabError::CanonicalEncoding)
}

fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn io_error(error: std::io::Error) -> LabError {
    LabError::ExecutorFailed(error.to_string())
}

struct DisposableWorkspace {
    path: PathBuf,
}

impl DisposableWorkspace {
    fn create() -> Result<Self, LabError> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| LabError::ExecutorFailed("clock before epoch".to_owned()))?
            .as_nanos();
        let path = std::env::temp_dir().join(format!("nando-lab-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).map_err(io_error)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn cleanup(self) -> CleanupReceipt {
        let cleanup_root_sha256 = sha256(self.path.to_string_lossy().as_bytes());
        let cleanup_succeeded = fs::remove_dir_all(&self.path).is_ok();
        CleanupReceipt {
            cleanup_attempted: true,
            cleanup_succeeded,
            cleanup_root_sha256,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_selector_prefers_balanced_partition() -> Result<(), LabError> {
        let balanced = git_rename_probe()?;
        let mut imbalanced = balanced.clone();
        imbalanced.id = "imbalanced".to_owned();
        imbalanced.hypotheses = vec![
            Hypothesis {
                id: "one".to_owned(),
                prediction: OutcomeVector::default(),
            },
            Hypothesis {
                id: "two".to_owned(),
                prediction: OutcomeVector::default(),
            },
            Hypothesis {
                id: "three".to_owned(),
                prediction: OutcomeVector::default(),
            },
        ];
        let selected = select_probe(&[imbalanced, balanced])?;
        assert_eq!(selected.probe_id, "git-rename-law");
        assert_eq!(selected.partition_sizes, vec![1, 1, 1]);
        Ok(())
    }

    #[test]
    fn git_probe_uses_real_git_and_stays_non_authoritative() -> Result<(), LabError> {
        let receipt = execute_probe(&git_rename_probe()?)?;
        assert_eq!(receipt.surviving_hypotheses, vec!["tree-and-index"]);
        let candidate = receipt
            .unique_law_candidate
            .as_ref()
            .ok_or(LabError::NoUniqueLaw)?;
        assert!(!candidate.authority_granted);
        assert!(!candidate.active_package_allowed);
        assert!(receipt.lab_probe);
        assert!(!receipt.production_authority);
        assert!(receipt.cleanup.cleanup_succeeded);
        Ok(())
    }

    #[test]
    fn two_filesystem_probes_open_two_more_laws() -> Result<(), LabError> {
        let copy = execute_probe(&filesystem_copy_probe()?)?;
        let delete = execute_probe(&filesystem_delete_probe()?)?;
        assert_eq!(copy.surviving_hypotheses, vec!["copy-preserves-source"]);
        assert_eq!(delete.surviving_hypotheses, vec!["delete-removes-source"]);
        Ok(())
    }

    #[test]
    fn natural_holdout_is_required_and_does_not_activate_package() -> Result<(), LabError> {
        let receipt = execute_probe(&git_rename_probe()?)?;
        let candidate = receipt.unique_law_candidate.ok_or(LabError::NoUniqueLaw)?;
        let certificate = certify_natural_holdout(
            &candidate,
            IndependentNaturalHoldoutReceipt::new(
                "natural/git-trajectory/001",
                "independent-collector",
                sha256(b"external-natural-record"),
                candidate.prediction,
            )?,
        )?;
        assert_eq!(certificate.schema, LAW_CERTIFICATE_SCHEMA_V1);
        assert!(!certificate.authority_granted);
        assert!(!certificate.active_package_allowed);
        assert!(
            IndependentNaturalHoldoutReceipt::new(
                "lab/same-probe-replay",
                "same-lab",
                sha256(b"replay"),
                candidate.prediction,
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn unsafe_paths_and_policy_fail_closed() -> Result<(), LabError> {
        let mut files = BTreeMap::new();
        files.insert("../escape".to_owned(), vec![1]);
        assert!(InitialState::new(files).is_err());
        let mut probe = filesystem_copy_probe()?;
        probe.safety_budget.network_enabled = true;
        assert!(probe.validate().is_err());
        Ok(())
    }
}
