use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::r8b_process_authorizer::{
    closed_tree_paths_v2, descriptor_matches_entry_v2, load_identity_manifest_v2,
    read_closed_file_v2,
};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_sha256_file_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2_UNCERTAINTY_R8B_PACKET_MANIFEST_PATH_V2,
    K2_UNCERTAINTY_R8B_STDOUT_RECEIPT_PATH_V2, K2UncertaintyCleanupReceiptV1,
    K2UncertaintyControlEvaluationReceiptV1, K2UncertaintyDevelopmentResultReceiptV1,
    K2UncertaintyOracleBaselineBatchReceiptV1, K2UncertaintyR8BEvidenceKindV2,
    K2UncertaintyR8BExecutableManifestV2, K2UncertaintyR8BManifestClassV2,
    K2UncertaintyR8BMeasuredReceiptV2, K2UncertaintyR8BPacketEntryV2,
    K2UncertaintyR8BPacketManifestV2, denied_authority_v1, require_denied_authority_v1,
    uncertainty_bytes_v1, uncertainty_decode_v1, uncertainty_root_v1,
};

pub const K2_UNCERTAINTY_R8B_AUTHORIZATION_REQUEST_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-authorization-request.v2";
pub const K2_UNCERTAINTY_R8B_AUTHORIZATION_RECEIPT_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-authorization-receipt.v2";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BAuthorizationRequestV2 {
    pub schema: String,
    pub route_id_sha256: String,
    pub manifest_root_sha256: String,
    pub authorizer_executable_sha256: String,
    pub request_root_sha256: String,
}

impl K2UncertaintyR8BAuthorizationRequestV2 {
    pub fn seal(
        route_id_sha256: String,
        manifest_root_sha256: String,
        authorizer_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_R8B_AUTHORIZATION_REQUEST_SCHEMA_V2.to_owned(),
            route_id_sha256,
            manifest_root_sha256,
            authorizer_executable_sha256,
            request_root_sha256: String::new(),
        };
        value.request_root_sha256 = rooted_v2(&value)?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.route_id_sha256,
            &self.manifest_root_sha256,
            &self.authorizer_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        let mut canonical = self.clone();
        canonical.request_root_sha256.clear();
        if self.schema != K2_UNCERTAINTY_R8B_AUTHORIZATION_REQUEST_SCHEMA_V2
            || self.request_root_sha256 != uncertainty_root_v1(&canonical)?
        {
            return Err(invalid("self_formed_r8b_authorization_request_invalid"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BAuthorizationReceiptV2 {
    pub schema: String,
    pub request_root_sha256: String,
    pub tested_commit_sha256: String,
    pub route_id_sha256: String,
    pub manifest_root_sha256: String,
    pub linked_manifest_root_sha256: String,
    pub suite_manifest_root_sha256: String,
    pub process_ledger_root_sha256: String,
    pub entry_roots_sha256: Vec<String>,
    pub publisher_executable_sha256: String,
    pub disposition: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyR8BAuthorizationReceiptV2 {
    fn seal(
        request: &K2UncertaintyR8BAuthorizationRequestV2,
        manifest: &K2UncertaintyR8BPacketManifestV2,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_R8B_AUTHORIZATION_RECEIPT_SCHEMA_V2.to_owned(),
            request_root_sha256: request.request_root_sha256.clone(),
            tested_commit_sha256: manifest.tested_commit_sha256.clone(),
            route_id_sha256: manifest.route_id_sha256.clone(),
            manifest_root_sha256: manifest.manifest_root_sha256.clone(),
            linked_manifest_root_sha256: manifest.linked_manifest_root_sha256.clone(),
            suite_manifest_root_sha256: manifest.suite_manifest_root_sha256.clone(),
            process_ledger_root_sha256: manifest.process_ledger.ledger_root_sha256.clone(),
            entry_roots_sha256: manifest
                .entries
                .iter()
                .map(|entry| entry.entry_root_sha256.clone())
                .collect(),
            publisher_executable_sha256: manifest.publisher_executable_sha256.clone(),
            disposition: "R8B_FROZEN".to_owned(),
            authority: denied_authority_v1(),
            receipt_root_sha256: String::new(),
        };
        value.receipt_root_sha256 = rooted_v2(&value)?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.request_root_sha256,
            &self.tested_commit_sha256,
            &self.route_id_sha256,
            &self.manifest_root_sha256,
            &self.linked_manifest_root_sha256,
            &self.suite_manifest_root_sha256,
            &self.process_ledger_root_sha256,
            &self.publisher_executable_sha256,
        ]
        .into_iter()
        .chain(self.entry_roots_sha256.iter())
        {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        let mut canonical = self.clone();
        canonical.receipt_root_sha256.clear();
        if self.schema != K2_UNCERTAINTY_R8B_AUTHORIZATION_RECEIPT_SCHEMA_V2
            || self.entry_roots_sha256.is_empty()
            || self.disposition != "R8B_FROZEN"
            || self.receipt_root_sha256 != uncertainty_root_v1(&canonical)?
        {
            return Err(invalid("self_formed_r8b_authorization_receipt_invalid"));
        }
        Ok(())
    }
}

pub fn authorize_self_formed_r8b_v2(
    request: &K2UncertaintyR8BAuthorizationRequestV2,
    packet_root: &Path,
) -> K2CompositionResultV1<K2UncertaintyR8BAuthorizationReceiptV2> {
    request.validate()?;
    let manifest_bytes = read_closed_file_v2(
        &packet_root.join(K2_UNCERTAINTY_R8B_PACKET_MANIFEST_PATH_V2),
        None,
    )?;
    let manifest: K2UncertaintyR8BPacketManifestV2 = uncertainty_decode_v1(&manifest_bytes)?;
    manifest.validate()?;
    if manifest.route_id_sha256 != request.route_id_sha256
        || manifest.manifest_root_sha256 != request.manifest_root_sha256
    {
        return Err(invalid("self_formed_r8b_packet_request_binding_invalid"));
    }
    let observed_paths = closed_tree_paths_v2(packet_root)?;
    let expected_paths = manifest
        .entries
        .iter()
        .map(|entry| entry.relative_path.clone())
        .chain(std::iter::once(
            K2_UNCERTAINTY_R8B_PACKET_MANIFEST_PATH_V2.to_owned(),
        ))
        .collect::<BTreeSet<_>>();
    if observed_paths != expected_paths {
        return Err(invalid("self_formed_r8b_packet_path_set_invalid"));
    }

    let linked = load_identity_manifest_v2(
        packet_root,
        &manifest,
        K2UncertaintyR8BEvidenceKindV2::LinkedManifest,
        K2UncertaintyR8BManifestClassV2::Linked,
    )?;
    let suite = load_identity_manifest_v2(
        packet_root,
        &manifest,
        K2UncertaintyR8BEvidenceKindV2::SuiteManifest,
        K2UncertaintyR8BManifestClassV2::Suite,
    )?;
    if linked.manifest_root_sha256 != manifest.linked_manifest_root_sha256
        || suite.manifest_root_sha256 != manifest.suite_manifest_root_sha256
    {
        return Err(invalid("self_formed_r8b_manifest_root_binding_invalid"));
    }
    let identities = linked
        .identities
        .iter()
        .chain(&suite.identities)
        .map(|identity| (identity.role.as_str(), identity.sha256.as_str()))
        .collect::<BTreeMap<_, _>>();
    if identities.get("M25_R8B_AUTHORIZER") != Some(&request.authorizer_executable_sha256.as_str())
        || identities.get("M26_R8B_PUBLISHER")
            != Some(&manifest.publisher_executable_sha256.as_str())
    {
        return Err(invalid("self_formed_r8b_authority_identity_invalid"));
    }
    let mut consumed_descriptors = BTreeSet::new();
    for entry in &manifest.entries {
        verify_packet_entry_v2(packet_root, entry)?;
        if identities.get(entry.producer_role.as_str())
            != Some(&entry.producer_executable_sha256.as_str())
        {
            return Err(invalid("self_formed_r8b_entry_producer_identity_invalid"));
        }
        let parent_owned = matches!(
            entry.kind,
            K2UncertaintyR8BEvidenceKindV2::LinkedManifest
                | K2UncertaintyR8BEvidenceKindV2::SuiteManifest
                | K2UncertaintyR8BEvidenceKindV2::ProductionSurvival
        );
        if parent_owned {
            if entry.producer_role != "M24_LINKED_RUNNER"
                || entry.producer_event_root_sha256 != entry.semantic_root_sha256
            {
                return Err(invalid("self_formed_r8b_parent_observation_invalid"));
            }
        } else {
            if matches!(
                entry.producer_role.as_str(),
                "M25_R8B_AUTHORIZER" | "M26_R8B_PUBLISHER"
            ) {
                return Err(invalid("self_formed_r8b_future_outcome_in_packet"));
            }
            let event = manifest
                .process_ledger
                .finished_event(&entry.producer_event_root_sha256)
                .ok_or_else(|| invalid("self_formed_r8b_entry_process_event_missing"))?;
            if event.role != entry.producer_role
                || event.executable_sha256 != entry.producer_executable_sha256
                || event.normal_exit != Some(true)
                || event.exit_code != Some(0)
            {
                return Err(invalid("self_formed_r8b_entry_process_binding_invalid"));
            }
            let (index, descriptor) = event
                .produced_receipts
                .iter()
                .enumerate()
                .filter(|(_, descriptor)| descriptor_matches_entry_v2(descriptor, entry))
                .collect::<Vec<_>>()
                .as_slice()
                .first()
                .copied()
                .ok_or_else(|| invalid("self_formed_r8b_produced_receipt_missing"))?;
            if !consumed_descriptors.insert((event.event_root_sha256.as_str(), index))
                || event
                    .produced_receipts
                    .iter()
                    .filter(|candidate| descriptor_matches_entry_v2(candidate, entry))
                    .count()
                    != 1
                || (descriptor.relative_path == K2_UNCERTAINTY_R8B_STDOUT_RECEIPT_PATH_V2
                    && (event.stdout_byte_len != Some(descriptor.byte_len)
                        || event.stdout_sha256.as_ref() != Some(&descriptor.content_sha256)))
            {
                return Err(invalid("self_formed_r8b_produced_receipt_reused"));
            }
        }
    }
    K2UncertaintyR8BAuthorizationReceiptV2::seal(request, &manifest)
}

pub fn run_self_formed_r8b_authorizer_process_v2() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_r8b_authorizer_stdin"))?;
    let request: K2UncertaintyR8BAuthorizationRequestV2 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_r8b_authorizer"))?;
    if composition_sha256_file_v1(&executable)? != request.authorizer_executable_sha256 {
        return Err(invalid("self_formed_r8b_authorizer_executable_mismatch"));
    }
    let packet_root = std::env::current_dir()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_r8b_packet_root"))?;
    let receipt = authorize_self_formed_r8b_v2(&request, &packet_root)?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_r8b_authorizer_stdout"))
}

fn verify_packet_entry_v2(
    root: &Path,
    entry: &K2UncertaintyR8BPacketEntryV2,
) -> K2CompositionResultV1<()> {
    entry.validate()?;
    let bytes = read_closed_file_v2(&root.join(&entry.relative_path), Some(entry))?;
    match entry.kind {
        K2UncertaintyR8BEvidenceKindV2::LinkedRoute
        | K2UncertaintyR8BEvidenceKindV2::ProductionSurvival
        | K2UncertaintyR8BEvidenceKindV2::ConfirmCanonicalBytes
        | K2UncertaintyR8BEvidenceKindV2::DevelopmentKnownAnswers
        | K2UncertaintyR8BEvidenceKindV2::ModeMatrix
        | K2UncertaintyR8BEvidenceKindV2::ImmutablePublication
        | K2UncertaintyR8BEvidenceKindV2::ProcessRestart
        | K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes
        | K2UncertaintyR8BEvidenceKindV2::CleanupInterruption
        | K2UncertaintyR8BEvidenceKindV2::AggregatePublicationFaults => {
            let value: K2UncertaintyR8BMeasuredReceiptV2 = uncertainty_decode_v1(&bytes)?;
            value.validate()?;
            if value.kind != entry.kind
                || value.route_id_sha256 != entry.route_id_sha256
                || value.observed != entry.observed
                || value.producer_executable_sha256 != entry.producer_executable_sha256
                || value.receipt_root_sha256 != entry.semantic_root_sha256
            {
                return Err(invalid("self_formed_r8b_measured_entry_invalid"));
            }
        }
        K2UncertaintyR8BEvidenceKindV2::LinkedManifest
        | K2UncertaintyR8BEvidenceKindV2::SuiteManifest => {
            let value: K2UncertaintyR8BExecutableManifestV2 = uncertainty_decode_v1(&bytes)?;
            value.validate()?;
            if value.manifest_root_sha256 != entry.semantic_root_sha256 {
                return Err(invalid("self_formed_r8b_manifest_entry_invalid"));
            }
        }
        K2UncertaintyR8BEvidenceKindV2::OracleCases => {
            let value: K2UncertaintyOracleBaselineBatchReceiptV1 = uncertainty_decode_v1(&bytes)?;
            value.validate()?;
            if value.case_receipts.len() as u64 != entry.observed
                || value.receipt_root_sha256 != entry.semantic_root_sha256
            {
                return Err(invalid("self_formed_r8b_oracle_entry_invalid"));
            }
        }
        K2UncertaintyR8BEvidenceKindV2::LegacyControls
        | K2UncertaintyR8BEvidenceKindV2::V3Controls
        | K2UncertaintyR8BEvidenceKindV2::V4Controls
        | K2UncertaintyR8BEvidenceKindV2::FreshControlCases => {
            let value: K2UncertaintyControlEvaluationReceiptV1 = uncertainty_decode_v1(&bytes)?;
            value.validate()?;
            if !value.all_pass
                || (entry.kind != K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes
                    && value.passed != entry.observed)
                || value.receipt_root_sha256 != entry.semantic_root_sha256
            {
                return Err(invalid("self_formed_r8b_control_entry_invalid"));
            }
        }
        K2UncertaintyR8BEvidenceKindV2::CleanupTransaction => {
            let value: K2UncertaintyCleanupReceiptV1 = uncertainty_decode_v1(&bytes)?;
            value.validate()?;
            require_semantic_root_v2(&value.receipt_root_sha256, entry)?;
        }
        K2UncertaintyR8BEvidenceKindV2::DevelopmentResult => {
            let value: K2UncertaintyDevelopmentResultReceiptV1 = uncertainty_decode_v1(&bytes)?;
            value.validate()?;
            require_semantic_root_v2(&value.receipt_root_sha256, entry)?;
        }
    }
    Ok(())
}

fn require_semantic_root_v2(
    actual: &str,
    entry: &K2UncertaintyR8BPacketEntryV2,
) -> K2CompositionResultV1<()> {
    if actual == entry.semantic_root_sha256 {
        Ok(())
    } else {
        Err(invalid("self_formed_r8b_packet_semantic_root_mismatch"))
    }
}

fn rooted_v2<T>(value: &T) -> K2CompositionResultV1<String>
where
    T: Clone + Serialize + RootFieldV2,
{
    let mut canonical = value.clone();
    canonical.clear_root_v2();
    uncertainty_root_v1(&canonical)
}

trait RootFieldV2 {
    fn clear_root_v2(&mut self);
}

impl RootFieldV2 for K2UncertaintyR8BAuthorizationRequestV2 {
    fn clear_root_v2(&mut self) {
        self.request_root_sha256.clear();
    }
}

impl RootFieldV2 for K2UncertaintyR8BAuthorizationReceiptV2 {
    fn clear_root_v2(&mut self) {
        self.receipt_root_sha256.clear();
    }
}

fn invalid(reason: &'static str) -> K2CompositionErrorV1 {
    K2CompositionErrorV1::Invalid(reason)
}
