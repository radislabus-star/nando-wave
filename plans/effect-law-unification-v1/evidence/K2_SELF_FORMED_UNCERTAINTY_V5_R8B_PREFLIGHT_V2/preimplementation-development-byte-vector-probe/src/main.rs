use std::cmp::Ordering;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use nando_operator_learning::{
    K2CompositionAuthorityBoundaryV1, K2UncertaintyConfirmAttemptModeV1,
    K2UncertaintyConfirmPipeReceiptV1, composition_sha256_bytes_v1, uncertainty_bytes_v1,
    uncertainty_decode_v1, uncertainty_root_v1,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

const SOURCE_COMMIT: &str = "bdcae5351c7de75f325b0ebe752804066823cc38";
const VECTOR_DOMAIN: &str = "nando.k2-self-formed-r8b-development-byte-vector.v1";
const ARTIFACT_SCHEMA: &str = "nando.k2-self-formed-development-rehearsal-stored-artifact.v1";
const SPLIT_SCHEMA: &str = "nando.k2-self-formed-development-rehearsal-split-receipt.v1";
const OWNER_SCHEMA: &str = "nando.k2-self-formed-development-rehearsal-owner-receipt.v1";
const RECONSTRUCTION_DOMAIN: &str =
    "nando.k2-self-formed-development-rehearsal-private-reconstruction.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
enum DevelopmentStoredArtifactKindV1 {
    PublicBatch,
    PublicDenominator,
    ResolverTable,
    FinalTruth,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct DevelopmentStoredArtifactV1 {
    schema: String,
    mode: K2UncertaintyConfirmAttemptModeV1,
    kind: DevelopmentStoredArtifactKindV1,
    case_id_sha256: Option<String>,
    private_case_ordinal: Option<u64>,
    relative_path: String,
    unix_mode: u32,
    byte_len: u64,
    content_sha256: String,
    semantic_root_sha256: String,
    authority: K2CompositionAuthorityBoundaryV1,
    artifact_root_sha256: String,
}

impl DevelopmentStoredArtifactV1 {
    fn seal(
        kind: DevelopmentStoredArtifactKindV1,
        case_id_sha256: Option<String>,
        private_case_ordinal: Option<u64>,
        relative_path: String,
        unix_mode: u32,
        label: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut artifact = Self {
            schema: ARTIFACT_SCHEMA.to_owned(),
            mode: K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal,
            kind,
            case_id_sha256,
            private_case_ordinal,
            relative_path,
            unix_mode,
            byte_len: 100 + label.len() as u64,
            content_sha256: vector_root(&format!("content-{label}"))?,
            semantic_root_sha256: vector_root(&format!("semantic-{label}"))?,
            authority: K2CompositionAuthorityBoundaryV1::denied(),
            artifact_root_sha256: String::new(),
        };
        artifact.artifact_root_sha256 = artifact.expected_root()?;
        Ok(artifact)
    }

    fn expected_root(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(uncertainty_root_v1(&(
            ARTIFACT_SCHEMA,
            self.mode,
            self.kind,
            &self.case_id_sha256,
            &self.private_case_ordinal,
            &self.relative_path,
            self.unix_mode,
            self.byte_len,
            &self.content_sha256,
            &self.semantic_root_sha256,
            &self.authority,
        ))?)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct DevelopmentSplitReceiptV1 {
    schema: String,
    mode: K2UncertaintyConfirmAttemptModeV1,
    attempt_root_sha256: String,
    owner_request_root_sha256: String,
    owner_executable_sha256: String,
    generator_executable_sha256: String,
    generator_request_root_sha256: String,
    generator_response_root_sha256: String,
    pipe_receipt: K2UncertaintyConfirmPipeReceiptV1,
    pipe_receipt_root_sha256: String,
    experiment_id_sha256: String,
    development_seed_commitment_sha256: String,
    public_batch_root_sha256: String,
    private_batch_root_sha256: String,
    public_denominator_root_sha256: String,
    artifacts: Vec<DevelopmentStoredArtifactV1>,
    private_reconstruction_root_sha256: String,
    authority: K2CompositionAuthorityBoundaryV1,
    split_receipt_root_sha256: String,
}

impl DevelopmentSplitReceiptV1 {
    fn expected_root(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(uncertainty_root_v1(&(
            SPLIT_SCHEMA,
            (
                self.mode,
                &self.attempt_root_sha256,
                &self.owner_request_root_sha256,
                &self.owner_executable_sha256,
                &self.generator_executable_sha256,
                &self.generator_request_root_sha256,
                &self.generator_response_root_sha256,
                &self.pipe_receipt,
            ),
            (
                &self.pipe_receipt_root_sha256,
                &self.experiment_id_sha256,
                &self.development_seed_commitment_sha256,
                &self.public_batch_root_sha256,
                &self.private_batch_root_sha256,
                &self.public_denominator_root_sha256,
                &self.artifacts,
                &self.private_reconstruction_root_sha256,
            ),
            &self.authority,
        ))?)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct DevelopmentOwnerReceiptV1 {
    schema: String,
    mode: K2UncertaintyConfirmAttemptModeV1,
    owner_request_root_sha256: String,
    attempt_root_sha256: String,
    owner_executable_sha256: String,
    generator_executable_sha256: String,
    generator_request_root_sha256: String,
    generator_response_root_sha256: String,
    public_batch_root_sha256: String,
    private_batch_root_sha256: String,
    split_receipt_root_sha256: String,
    pipe_receipt_root_sha256: String,
    cases_generated_event_root_sha256: String,
    generator_dispatch_count: u64,
    nonce_commitment_sha256: Option<String>,
    authorization_receipt_root_sha256: Option<String>,
    authorization_slot_claim_root_sha256: Option<String>,
    sealed_attempts: u64,
    authority: K2CompositionAuthorityBoundaryV1,
    receipt_root_sha256: String,
}

impl DevelopmentOwnerReceiptV1 {
    fn expected_root(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(uncertainty_root_v1(&(
            OWNER_SCHEMA,
            (
                self.mode,
                &self.owner_request_root_sha256,
                &self.attempt_root_sha256,
                &self.owner_executable_sha256,
                &self.generator_executable_sha256,
                &self.generator_request_root_sha256,
                &self.generator_response_root_sha256,
                &self.public_batch_root_sha256,
            ),
            (
                &self.private_batch_root_sha256,
                &self.split_receipt_root_sha256,
                &self.pipe_receipt_root_sha256,
                &self.cases_generated_event_root_sha256,
                self.generator_dispatch_count,
                &self.nonce_commitment_sha256,
                &self.authorization_receipt_root_sha256,
                &self.authorization_slot_claim_root_sha256,
            ),
            self.sealed_attempts,
            &self.authority,
        ))?)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let pipe_path = required_path(args.next(), "historical pipe fixture")?;
    let output_root = required_path(args.next(), "output root")?;
    if args.next().is_some() {
        return Err("expected exactly two arguments".into());
    }
    create_private_directory(&output_root)?;

    let pipe: K2UncertaintyConfirmPipeReceiptV1 = uncertainty_decode_v1(&fs::read(pipe_path)?)?;
    pipe.validate()?;

    let mut artifacts = development_artifacts()?;
    artifacts.sort_by(compare_artifacts);
    if artifacts.len() != 34 {
        return Err("Development artifact vector must contain 34 descriptors".into());
    }

    let private_case_roots = (0..16)
        .map(|ordinal| vector_root(&format!("private-case-{ordinal:02}")))
        .collect::<Result<Vec<_>, _>>()?;
    let private_reconstruction_root_sha256 = uncertainty_root_v1(&(
        RECONSTRUCTION_DOMAIN,
        &private_case_roots,
        vector_root("reconstructed-private-batch")?,
        vector_root("reconstructed-generator-response")?,
        785_577_u64,
        vector_root("canonical-generator-response-bytes")?,
    ))?;

    let mut split = DevelopmentSplitReceiptV1 {
        schema: SPLIT_SCHEMA.to_owned(),
        mode: K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal,
        attempt_root_sha256: vector_root("attempt")?,
        owner_request_root_sha256: vector_root("owner-request")?,
        owner_executable_sha256: vector_root("owner-executable")?,
        generator_executable_sha256: vector_root("generator-executable")?,
        generator_request_root_sha256: vector_root("generator-request")?,
        generator_response_root_sha256: vector_root("generator-response")?,
        pipe_receipt_root_sha256: pipe.receipt_root_sha256.clone(),
        pipe_receipt: pipe,
        experiment_id_sha256: vector_root("experiment")?,
        development_seed_commitment_sha256: vector_root("development-seed")?,
        public_batch_root_sha256: vector_root("public-batch")?,
        private_batch_root_sha256: vector_root("private-batch")?,
        public_denominator_root_sha256: vector_root("public-denominator")?,
        artifacts,
        private_reconstruction_root_sha256,
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        split_receipt_root_sha256: String::new(),
    };
    split.split_receipt_root_sha256 = split.expected_root()?;

    let mut owner = DevelopmentOwnerReceiptV1 {
        schema: OWNER_SCHEMA.to_owned(),
        mode: K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal,
        owner_request_root_sha256: split.owner_request_root_sha256.clone(),
        attempt_root_sha256: split.attempt_root_sha256.clone(),
        owner_executable_sha256: split.owner_executable_sha256.clone(),
        generator_executable_sha256: split.generator_executable_sha256.clone(),
        generator_request_root_sha256: split.generator_request_root_sha256.clone(),
        generator_response_root_sha256: split.generator_response_root_sha256.clone(),
        public_batch_root_sha256: split.public_batch_root_sha256.clone(),
        private_batch_root_sha256: split.private_batch_root_sha256.clone(),
        split_receipt_root_sha256: split.split_receipt_root_sha256.clone(),
        pipe_receipt_root_sha256: split.pipe_receipt_root_sha256.clone(),
        cases_generated_event_root_sha256: vector_root("cases-generated-event")?,
        generator_dispatch_count: 1,
        nonce_commitment_sha256: None,
        authorization_receipt_root_sha256: None,
        authorization_slot_claim_root_sha256: None,
        sealed_attempts: 0,
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    };
    owner.receipt_root_sha256 = owner.expected_root()?;

    let artifact_bytes = write_canonical(
        &output_root.join("development-stored-artifacts.vector.json"),
        &split.artifacts,
    )?;
    let split_bytes = write_canonical(
        &output_root.join("development-split-receipt.vector.json"),
        &split,
    )?;
    let owner_bytes = write_canonical(
        &output_root.join("development-owner-receipt.vector.json"),
        &owner,
    )?;

    let manifest = serde_json::json!({
        "schema": "nando.k2-self-formed-r8b-development-byte-known-answer.v1",
        "status": "preimplementation_mirror_only_no_authority",
        "source_commit": SOURCE_COMMIT,
        "artifact_count": split.artifacts.len(),
        "private_reconstruction_root_sha256": split.private_reconstruction_root_sha256,
        "split_receipt_root_sha256": split.split_receipt_root_sha256,
        "owner_receipt_root_sha256": owner.receipt_root_sha256,
        "artifact_vector_bytes": artifact_bytes.len(),
        "artifact_vector_bytes_sha256": composition_sha256_bytes_v1(&artifact_bytes),
        "split_receipt_bytes": split_bytes.len(),
        "split_receipt_bytes_sha256": composition_sha256_bytes_v1(&split_bytes),
        "owner_receipt_bytes": owner_bytes.len(),
        "owner_receipt_bytes_sha256": composition_sha256_bytes_v1(&owner_bytes),
        "denied_authority": true,
        "sealed_attempts": 0,
    });
    write_canonical(
        &output_root.join("development-byte-known-answer.manifest.json"),
        &manifest,
    )?;

    println!("artifact_count={}", split.artifacts.len());
    println!(
        "split_receipt_root_sha256={}",
        split.split_receipt_root_sha256
    );
    println!("owner_receipt_root_sha256={}", owner.receipt_root_sha256);
    Ok(())
}

fn development_artifacts() -> Result<Vec<DevelopmentStoredArtifactV1>, Box<dyn std::error::Error>> {
    let mut artifacts = vec![
        DevelopmentStoredArtifactV1::seal(
            DevelopmentStoredArtifactKindV1::PublicBatch,
            None,
            None,
            "public/public-batch.json".to_owned(),
            0o600,
            "public-batch",
        )?,
        DevelopmentStoredArtifactV1::seal(
            DevelopmentStoredArtifactKindV1::PublicDenominator,
            None,
            None,
            "public/denominator-receipt.json".to_owned(),
            0o600,
            "public-denominator",
        )?,
    ];
    for ordinal in 0..16_u64 {
        let case_id = vector_root(&format!("case-{ordinal:02}"))?;
        artifacts.push(DevelopmentStoredArtifactV1::seal(
            DevelopmentStoredArtifactKindV1::ResolverTable,
            Some(case_id.clone()),
            Some(ordinal),
            format!("private/resolver/{case_id}.json"),
            0o400,
            &format!("resolver-{ordinal:02}"),
        )?);
        artifacts.push(DevelopmentStoredArtifactV1::seal(
            DevelopmentStoredArtifactKindV1::FinalTruth,
            Some(case_id.clone()),
            Some(ordinal),
            format!("private/final-truth/{case_id}.json"),
            0o400,
            &format!("final-truth-{ordinal:02}"),
        )?);
    }
    Ok(artifacts)
}

fn compare_artifacts(
    left: &DevelopmentStoredArtifactV1,
    right: &DevelopmentStoredArtifactV1,
) -> Ordering {
    (
        left.kind,
        &left.case_id_sha256,
        &left.private_case_ordinal,
        &left.relative_path,
        &left.artifact_root_sha256,
    )
        .cmp(&(
            right.kind,
            &right.case_id_sha256,
            &right.private_case_ordinal,
            &right.relative_path,
            &right.artifact_root_sha256,
        ))
}

fn vector_root(label: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(uncertainty_root_v1(&(VECTOR_DOMAIN, label))?)
}

fn required_path(
    value: Option<std::ffi::OsString>,
    label: &'static str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    value.map(PathBuf::from).ok_or_else(|| label.into())
}

fn create_private_directory(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn write_canonical<T>(path: &Path, value: &T) -> Result<Vec<u8>, Box<dyn std::error::Error>>
where
    T: DeserializeOwned + PartialEq + Serialize,
{
    let bytes = uncertainty_bytes_v1(value)?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);

    let reopened_bytes = fs::read(path)?;
    let reopened: T = uncertainty_decode_v1(&reopened_bytes)?;
    if &reopened != value || uncertainty_bytes_v1(&reopened)? != bytes {
        return Err(format!("canonical roundtrip mismatch: {}", path.display()).into());
    }
    Ok(bytes)
}
