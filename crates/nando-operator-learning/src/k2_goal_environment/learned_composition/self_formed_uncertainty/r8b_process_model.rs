use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    require_composition_root_v1, valid_composition_path_v1,
};
use super::{
    K2UncertaintyR8BEvidenceKindV2, denied_authority_v1, require_denied_authority_v1,
    uncertainty_root_v1,
};

pub const K2_UNCERTAINTY_R8B_EXECUTABLE_MANIFEST_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-executable-manifest.v2";
pub const K2_UNCERTAINTY_R8B_PROCESS_EVENT_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-process-event.v2";
pub const K2_UNCERTAINTY_R8B_PROCESS_LEDGER_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-process-ledger.v2";
pub const K2_UNCERTAINTY_R8B_ROUTE_RECEIPT_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-route-receipt.v2";
pub const K2_UNCERTAINTY_R8B_SUITE_RECEIPT_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-suite-receipt.v2";
pub const K2_UNCERTAINTY_R8B_PACKET_ENTRY_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-packet-entry.v2";
pub const K2_UNCERTAINTY_R8B_PACKET_MANIFEST_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-packet-manifest.v2";
pub const K2_UNCERTAINTY_R8B_PACKET_MANIFEST_PATH_V2: &str = "aggregate-manifest.json";
pub const K2_UNCERTAINTY_R8B_PRODUCER_REQUEST_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-producer-request.v2";
pub const K2_UNCERTAINTY_R8B_STDOUT_RECEIPT_PATH_V2: &str = "stdout";
pub const K2_UNCERTAINTY_R8B_MAX_PRODUCED_RECEIPTS_V2: usize = 19;

#[rustfmt::skip]
const LINKED_ROLES_V2: [&str; 26] = [
    "M01_DEVELOPMENT_OWNER", "M02_GENERATOR",
    "M03_LEARNER", "M04_PROBE", "M05_SELECTOR", "M06_BASELINE",
    "M07_SELECTION_PREVERIFIER", "M08_CLOSURE_PLANNER", "M09_CLOSURE_VERIFIER",
    "M10_PUBLIC_COORDINATOR", "M11_PRIVATE_RESOLVER", "M12_SAFETY",
    "M13_WORKER", "M14_OBSERVER", "M15_FINAL_VERIFIER", "M16_ORACLE",
    "M17_CONTROL_EVALUATOR", "M18_TERMINAL_EVALUATOR", "M19_FRESH_CONTROL_CASE",
    "M20_CLEANUP_AUTHORIZER", "M21_CLEANUP_OWNER", "M22_CLEANUP_VERIFIER",
    "M23_DEVELOPMENT_RESULT_PUBLISHER", "M24_LINKED_RUNNER",
    "M25_R8B_AUTHORIZER", "M26_R8B_PUBLISHER",
];

#[rustfmt::skip]
const SUITE_ROLES_V2: [&str; 5] = [
    "S01_CRATE_UNIT", "S02_RESTART", "S03_MODE_MATRIX",
    "S04_CLEANUP_NEGATIVE", "S05_AUTHORITY_PUBLICATION",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BProducerRequestV2 {
    pub schema: String,
    pub route_id_sha256: String,
    pub producer_role: String,
    pub producer_executable_sha256: String,
    pub test_selector: String,
    pub allowed_relative_paths: Vec<String>,
    pub exclusive_output_directory: String,
    pub request_root_sha256: String,
}

impl K2UncertaintyR8BProducerRequestV2 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.schema = K2_UNCERTAINTY_R8B_PRODUCER_REQUEST_SCHEMA_V2.to_owned();
        self.allowed_relative_paths.sort();
        self.request_root_sha256 = self.expected_root()?;
        self.validate()
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.route_id_sha256)?;
        require_composition_root_v1(&self.producer_executable_sha256)?;
        let paths = self.allowed_relative_paths.iter().collect::<BTreeSet<_>>();
        if self.schema != K2_UNCERTAINTY_R8B_PRODUCER_REQUEST_SCHEMA_V2
            || self.producer_role.is_empty()
            || self.test_selector.is_empty()
            || self.allowed_relative_paths.is_empty()
            || paths.len() != self.allowed_relative_paths.len()
            || !self
                .allowed_relative_paths
                .iter()
                .all(|path| valid_composition_path_v1(path))
            || !Path::new(&self.exclusive_output_directory).is_absolute()
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(invalid("self_formed_r8b_producer_request_invalid"));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        let mut value = self.clone();
        value.request_root_sha256.clear();
        uncertainty_root_v1(&value)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyR8BManifestClassV2 {
    Linked,
    Suite,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BExecutableIdentityV2 {
    pub role: String,
    pub canonical_path: String,
    pub byte_len: u64,
    pub unix_mode: u32,
    pub sha256: String,
}

impl K2UncertaintyR8BExecutableIdentityV2 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.sha256)?;
        if !Path::new(&self.canonical_path).is_absolute()
            || self.byte_len == 0
            || self.unix_mode & 0o111 == 0
        {
            return Err(invalid("self_formed_r8b_executable_identity_invalid"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BExecutableManifestV2 {
    pub schema: String,
    pub class: K2UncertaintyR8BManifestClassV2,
    pub identities: Vec<K2UncertaintyR8BExecutableIdentityV2>,
    pub manifest_root_sha256: String,
}

impl K2UncertaintyR8BExecutableManifestV2 {
    pub fn seal(
        class: K2UncertaintyR8BManifestClassV2,
        mut identities: Vec<K2UncertaintyR8BExecutableIdentityV2>,
    ) -> K2CompositionResultV1<Self> {
        identities.sort_by(|left, right| left.role.cmp(&right.role));
        let mut value = Self {
            schema: K2_UNCERTAINTY_R8B_EXECUTABLE_MANIFEST_SCHEMA_V2.to_owned(),
            class,
            identities,
            manifest_root_sha256: String::new(),
        };
        value.manifest_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        let expected = match self.class {
            K2UncertaintyR8BManifestClassV2::Linked => LINKED_ROLES_V2.as_slice(),
            K2UncertaintyR8BManifestClassV2::Suite => SUITE_ROLES_V2.as_slice(),
        };
        let roles = self
            .identities
            .iter()
            .map(|identity| identity.role.as_str())
            .collect::<Vec<_>>();
        let paths = self
            .identities
            .iter()
            .map(|identity| identity.canonical_path.as_str())
            .collect::<BTreeSet<_>>();
        let hashes = self
            .identities
            .iter()
            .map(|identity| identity.sha256.as_str())
            .collect::<BTreeSet<_>>();
        for identity in &self.identities {
            identity.validate()?;
        }
        if self.schema != K2_UNCERTAINTY_R8B_EXECUTABLE_MANIFEST_SCHEMA_V2
            || roles != expected
            || paths.len() != expected.len()
            || hashes.len() != expected.len()
            || self.manifest_root_sha256 != self.expected_root()?
        {
            return Err(invalid("self_formed_r8b_executable_manifest_invalid"));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        let mut value = self.clone();
        value.manifest_root_sha256.clear();
        uncertainty_root_v1(&value)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyR8BProcessEventKindV2 {
    ChildStarted,
    ChildFinished,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BProducedReceiptV2 {
    pub relative_path: String,
    pub byte_len: u64,
    pub unix_mode: u32,
    pub content_sha256: String,
    pub receipt_schema: String,
    pub semantic_root_sha256: String,
}

impl K2UncertaintyR8BProducedReceiptV2 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.content_sha256)?;
        require_composition_root_v1(&self.semantic_root_sha256)?;
        let stdout = self.relative_path == K2_UNCERTAINTY_R8B_STDOUT_RECEIPT_PATH_V2;
        if (!stdout && !valid_composition_path_v1(&self.relative_path))
            || self.byte_len == 0
            || self.receipt_schema.is_empty()
            || (stdout && self.unix_mode != 0)
            || (!stdout && self.unix_mode != 0o400)
        {
            return Err(invalid("self_formed_r8b_produced_receipt_invalid"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BProcessEventV2 {
    pub schema: String,
    pub sequence: u64,
    pub previous_event_root_sha256: Option<String>,
    pub kind: K2UncertaintyR8BProcessEventKindV2,
    pub route_id_sha256: String,
    pub stage_id: String,
    pub case_id_sha256: Option<String>,
    pub probe_ordinal: Option<u64>,
    pub writer_role: String,
    pub writer_executable_sha256: String,
    pub role: String,
    pub executable_sha256: String,
    pub request_root_sha256: String,
    pub stdin_sha256: String,
    pub started_event_root_sha256: Option<String>,
    pub normal_exit: Option<bool>,
    pub exit_code: Option<i32>,
    pub stdout_byte_len: Option<u64>,
    pub stdout_sha256: Option<String>,
    pub produced_receipts: Vec<K2UncertaintyR8BProducedReceiptV2>,
    pub stderr_byte_len: Option<u64>,
    pub stderr_sha256: Option<String>,
    pub started_monotonic_ns: u64,
    pub finished_monotonic_ns: Option<u64>,
    pub event_root_sha256: String,
}

impl K2UncertaintyR8BProcessEventV2 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.schema = K2_UNCERTAINTY_R8B_PROCESS_EVENT_SCHEMA_V2.to_owned();
        self.event_root_sha256 = self.expected_root()?;
        self.validate()
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            Some(&self.route_id_sha256),
            Some(&self.executable_sha256),
            Some(&self.request_root_sha256),
            Some(&self.stdin_sha256),
            Some(&self.writer_executable_sha256),
            self.previous_event_root_sha256.as_ref(),
            self.started_event_root_sha256.as_ref(),
            self.stdout_sha256.as_ref(),
            self.stderr_sha256.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            require_composition_root_v1(root)?;
        }
        let started = self.kind == K2UncertaintyR8BProcessEventKindV2::ChildStarted;
        let finish_fields = [
            self.normal_exit.is_some(),
            self.exit_code.is_some(),
            self.stdout_byte_len.is_some(),
            self.stdout_sha256.is_some(),
            !self.produced_receipts.is_empty(),
            self.stderr_byte_len.is_some(),
            self.stderr_sha256.is_some(),
            self.finished_monotonic_ns.is_some(),
            self.started_event_root_sha256.is_some(),
        ];
        if self.schema != K2_UNCERTAINTY_R8B_PROCESS_EVENT_SCHEMA_V2
            || self.stage_id.is_empty()
            || self.writer_role.is_empty()
            || self.role.is_empty()
            || (self.sequence == 0) != self.previous_event_root_sha256.is_none()
            || (started && finish_fields.iter().any(|present| *present))
            || (!started && finish_fields.iter().any(|present| !*present))
            || self.produced_receipts.len() > K2_UNCERTAINTY_R8B_MAX_PRODUCED_RECEIPTS_V2
            || !valid_produced_receipts_v2(&self.produced_receipts)
            || self
                .finished_monotonic_ns
                .is_some_and(|finished| finished < self.started_monotonic_ns)
            || self.event_root_sha256 != self.expected_root()?
        {
            return Err(invalid("self_formed_r8b_process_event_invalid"));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        let mut value = self.clone();
        value.event_root_sha256.clear();
        uncertainty_root_v1(&value)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BProcessLedgerV2 {
    pub schema: String,
    pub route_id_sha256: String,
    pub events: Vec<K2UncertaintyR8BProcessEventV2>,
    pub ledger_root_sha256: String,
}

impl K2UncertaintyR8BProcessLedgerV2 {
    pub fn seal(
        route_id_sha256: String,
        events: Vec<K2UncertaintyR8BProcessEventV2>,
    ) -> K2CompositionResultV1<Self> {
        Self::seal_prefix(route_id_sha256, events, true)
    }

    pub fn seal_natural_prefix(
        route_id_sha256: String,
        events: Vec<K2UncertaintyR8BProcessEventV2>,
    ) -> K2CompositionResultV1<Self> {
        Self::seal_prefix(route_id_sha256, events, false)
    }

    fn seal_prefix(
        route_id_sha256: String,
        events: Vec<K2UncertaintyR8BProcessEventV2>,
        require_complete: bool,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_R8B_PROCESS_LEDGER_SCHEMA_V2.to_owned(),
            route_id_sha256,
            events,
            ledger_root_sha256: String::new(),
        };
        value.ledger_root_sha256 = value.expected_root()?;
        value.validate_prefix(require_complete)?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.validate_prefix(true)
    }

    pub fn validate_natural_prefix(&self) -> K2CompositionResultV1<()> {
        self.validate_prefix(false)
    }

    fn validate_prefix(&self, require_complete: bool) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.route_id_sha256)?;
        let mut starts = BTreeMap::new();
        let mut finished = BTreeSet::new();
        let mut previous = None;
        for (sequence, event) in self.events.iter().enumerate() {
            event.validate()?;
            if event.sequence != sequence as u64
                || event.previous_event_root_sha256 != previous
                || event.route_id_sha256 != self.route_id_sha256
            {
                return Err(invalid("self_formed_r8b_process_ledger_chain_invalid"));
            }
            match event.kind {
                K2UncertaintyR8BProcessEventKindV2::ChildStarted => {
                    starts.insert(event.event_root_sha256.clone(), event);
                }
                K2UncertaintyR8BProcessEventKindV2::ChildFinished => {
                    let root = event
                        .started_event_root_sha256
                        .as_ref()
                        .ok_or_else(|| invalid("self_formed_r8b_process_start_missing"))?;
                    let start = starts
                        .get(root)
                        .ok_or_else(|| invalid("self_formed_r8b_process_start_foreign"))?;
                    if !finished.insert(root.clone()) || !same_invocation_v2(start, event) {
                        return Err(invalid("self_formed_r8b_process_finish_invalid"));
                    }
                }
            }
            previous = Some(event.event_root_sha256.clone());
        }
        if self.schema != K2_UNCERTAINTY_R8B_PROCESS_LEDGER_SCHEMA_V2
            || (require_complete && starts.len() != finished.len())
            || self.ledger_root_sha256 != self.expected_root()?
        {
            return Err(invalid("self_formed_r8b_process_ledger_incomplete"));
        }
        Ok(())
    }

    pub fn finished_event(&self, root: &str) -> Option<&K2UncertaintyR8BProcessEventV2> {
        self.events.iter().find(|event| {
            event.kind == K2UncertaintyR8BProcessEventKindV2::ChildFinished
                && event.event_root_sha256 == root
        })
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        let mut value = self.clone();
        value.ledger_root_sha256.clear();
        uncertainty_root_v1(&value)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BPacketEntryV2 {
    pub schema: String,
    pub kind: K2UncertaintyR8BEvidenceKindV2,
    pub relative_path: String,
    pub byte_len: u64,
    pub unix_mode: u32,
    pub content_sha256: String,
    pub receipt_schema: String,
    pub semantic_root_sha256: String,
    pub producer_role: String,
    pub producer_executable_sha256: String,
    pub producer_event_root_sha256: String,
    pub route_id_sha256: String,
    pub observed: u64,
    pub disposition: String,
    pub entry_root_sha256: String,
}

impl K2UncertaintyR8BPacketEntryV2 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.schema = K2_UNCERTAINTY_R8B_PACKET_ENTRY_SCHEMA_V2.to_owned();
        self.entry_root_sha256 = self.expected_root()?;
        self.validate()
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.content_sha256,
            &self.semantic_root_sha256,
            &self.producer_executable_sha256,
            &self.producer_event_root_sha256,
            &self.route_id_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        if self.schema != K2_UNCERTAINTY_R8B_PACKET_ENTRY_SCHEMA_V2
            || !valid_composition_path_v1(&self.relative_path)
            || self.relative_path == K2_UNCERTAINTY_R8B_PACKET_MANIFEST_PATH_V2
            || self.byte_len == 0
            || self.unix_mode != 0o400
            || self.receipt_schema != self.kind.expected_schema()
            || self.producer_role.is_empty()
            || self.observed == 0
            || self.disposition != "PASS"
            || self.entry_root_sha256 != self.expected_root()?
        {
            return Err(invalid("self_formed_r8b_packet_entry_invalid"));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        let mut value = self.clone();
        value.entry_root_sha256.clear();
        uncertainty_root_v1(&value)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BPacketManifestV2 {
    pub schema: String,
    pub tested_commit_sha256: String,
    pub route_id_sha256: String,
    pub linked_manifest_root_sha256: String,
    pub suite_manifest_root_sha256: String,
    pub process_ledger: K2UncertaintyR8BProcessLedgerV2,
    pub entries: Vec<K2UncertaintyR8BPacketEntryV2>,
    pub false_accepts: u64,
    pub sealed_attempts: u64,
    pub authorization_slots: u64,
    pub child_network_calls: u64,
    pub production_mutations: u64,
    pub publisher_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub manifest_root_sha256: String,
}

impl K2UncertaintyR8BPacketManifestV2 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.schema = K2_UNCERTAINTY_R8B_PACKET_MANIFEST_SCHEMA_V2.to_owned();
        self.authority = denied_authority_v1();
        self.entries.sort_by(|left, right| {
            (left.kind, &left.relative_path).cmp(&(right.kind, &right.relative_path))
        });
        self.manifest_root_sha256 = self.expected_root()?;
        self.validate()
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.tested_commit_sha256,
            &self.route_id_sha256,
            &self.linked_manifest_root_sha256,
            &self.suite_manifest_root_sha256,
            &self.publisher_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.process_ledger.validate()?;
        require_denied_authority_v1(&self.authority)?;
        let mut paths = BTreeSet::new();
        let mut observed = BTreeMap::new();
        for entry in &self.entries {
            entry.validate()?;
            if entry.route_id_sha256 != self.route_id_sha256 || !paths.insert(&entry.relative_path)
            {
                return Err(invalid("self_formed_r8b_packet_entry_set_invalid"));
            }
            *observed.entry(entry.kind).or_insert(0_u64) += entry.observed;
        }
        let kinds = observed.keys().copied().collect::<BTreeSet<_>>();
        let expected = K2UncertaintyR8BEvidenceKindV2::ALL
            .into_iter()
            .collect::<BTreeSet<_>>();
        let denominators_match = observed.iter().all(|(kind, count)| {
            kind.required()
                .map_or(*count > 0, |required| *count == required)
        });
        if self.schema != K2_UNCERTAINTY_R8B_PACKET_MANIFEST_SCHEMA_V2
            || self.process_ledger.route_id_sha256 != self.route_id_sha256
            || kinds != expected
            || !denominators_match
            || self.false_accepts != 0
            || self.sealed_attempts != 0
            || self.authorization_slots != 0
            || self.child_network_calls != 0
            || self.production_mutations != 0
            || self.manifest_root_sha256 != self.expected_root()?
        {
            return Err(invalid("self_formed_r8b_packet_manifest_invalid"));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        let mut value = self.clone();
        value.manifest_root_sha256.clear();
        uncertainty_root_v1(&value)
    }
}

fn same_invocation_v2(
    start: &K2UncertaintyR8BProcessEventV2,
    finish: &K2UncertaintyR8BProcessEventV2,
) -> bool {
    start.route_id_sha256 == finish.route_id_sha256
        && start.stage_id == finish.stage_id
        && start.case_id_sha256 == finish.case_id_sha256
        && start.probe_ordinal == finish.probe_ordinal
        && start.writer_role == finish.writer_role
        && start.writer_executable_sha256 == finish.writer_executable_sha256
        && start.role == finish.role
        && start.executable_sha256 == finish.executable_sha256
        && start.request_root_sha256 == finish.request_root_sha256
        && start.stdin_sha256 == finish.stdin_sha256
        && start.started_monotonic_ns == finish.started_monotonic_ns
}

fn valid_produced_receipts_v2(receipts: &[K2UncertaintyR8BProducedReceiptV2]) -> bool {
    let mut paths = BTreeSet::new();
    let mut roots = BTreeSet::new();
    receipts.iter().all(|receipt| {
        receipt.validate().is_ok()
            && paths.insert(receipt.relative_path.as_str())
            && roots.insert(receipt.semantic_root_sha256.as_str())
    })
}

fn invalid(reason: &'static str) -> K2CompositionErrorV1 {
    K2CompositionErrorV1::Invalid(reason)
}
