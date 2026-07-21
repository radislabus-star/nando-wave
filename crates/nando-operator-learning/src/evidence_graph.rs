use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use nando_operator_kernel::{canonical_json_bytes, canonical_json_sha256};

use crate::{CanonicalEventGraph, CanonicalEventNode, EvidenceEventTime};

pub const EVIDENCE_GRAPH_SCHEMA_V1: &str = "nando.evidence-graph.v1";
pub const EVIDENCE_GRAPH_RECEIPT_SCHEMA_V2: &str = "nando.evidence-graph-receipt.v2";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceGraphPolicy {
    pub max_events: usize,
    pub max_atoms: usize,
}

impl Default for EvidenceGraphPolicy {
    fn default() -> Self {
        Self {
            max_events: 1_024,
            max_atoms: 262_144,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EvidenceNodeRef {
    pub event_graph_sha256: String,
    pub node_index: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceNodeType {
    Null,
    Boolean,
    Number,
    String,
    ParsedJson,
    Array,
    Object,
    ObjectField,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "relation", rename_all = "snake_case")]
pub enum EvidenceGraphAtom {
    EventIncluded {
        event_graph_sha256: String,
        source_stream_sha256: String,
        source_offset: u64,
    },
    IntentMembership {
        event_graph_sha256: String,
        client_intent_id_sha256: String,
    },
    TypedNode {
        source: EvidenceNodeRef,
        node_type: EvidenceNodeType,
    },
    Cardinality {
        source: EvidenceNodeRef,
        count: usize,
    },
    CardinalityEquals {
        collection: EvidenceNodeRef,
        value: EvidenceNodeRef,
    },
    Contains {
        container: EvidenceNodeRef,
        member: EvidenceNodeRef,
    },
    FieldOf {
        record: EvidenceNodeRef,
        field: EvidenceNodeRef,
        value: EvidenceNodeRef,
    },
    MemberOf {
        member: EvidenceNodeRef,
        collection: EvidenceNodeRef,
    },
    ValueEquality {
        left: EvidenceNodeRef,
        right: EvidenceNodeRef,
    },
    DerivedFrom {
        derived: EvidenceNodeRef,
        source: EvidenceNodeRef,
    },
    TemporalBefore {
        predecessor_event_sha256: String,
        successor_event_sha256: String,
        session_id_sha256: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceGraph {
    pub schema: String,
    pub policy: EvidenceGraphPolicy,
    pub event_graph_sha256: Vec<String>,
    pub atoms: Vec<EvidenceGraphAtom>,
    pub graph_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceGraphRecord {
    pub schema: String,
    pub sequence: u64,
    pub previous_record_sha256: String,
    pub graph: EvidenceGraph,
    pub record_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct EvidenceGraphReceipt {
    schema: String,
    sequence: u64,
    previous_record_sha256: String,
    graph_sha256: String,
    event_set_sha256: String,
    event_graph_sha256: Vec<String>,
    event_count: usize,
    atom_count: usize,
    policy: EvidenceGraphPolicy,
    record_sha256: String,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceGraphStoreStatus {
    pub graph_total: u64,
    pub duplicate_graph_total: u64,
    pub recovered_partial_tail_bytes: u64,
}

pub struct DeterministicEvidenceGraphStore {
    path: PathBuf,
    next_sequence: u64,
    previous_record_sha256: String,
    seen_graphs: BTreeSet<[u8; 32]>,
    status: EvidenceGraphStoreStatus,
}

#[derive(Default)]
pub struct EvidenceGraphBuilder;

#[derive(Serialize)]
struct EvidenceGraphDigestMaterial<'a> {
    schema: &'static str,
    policy: EvidenceGraphPolicy,
    event_graph_sha256: &'a [String],
    atoms: &'a [EvidenceGraphAtom],
}

#[derive(Serialize)]
struct EvidenceGraphReceiptDigestMaterial<'a> {
    schema: &'static str,
    sequence: u64,
    previous_record_sha256: &'a str,
    graph_sha256: &'a str,
    event_set_sha256: &'a str,
    event_graph_sha256: &'a [String],
    event_count: usize,
    atom_count: usize,
    policy: EvidenceGraphPolicy,
}

impl DeterministicEvidenceGraphStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, String> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("evidence_graph_store_dir:{}:{error}", parent.display())
            })?;
        }
        let recovered_partial_tail_bytes = recover_partial_graph_tail(&path)?;
        let mut store = Self {
            path,
            next_sequence: 0,
            previous_record_sha256: "0".repeat(64),
            seen_graphs: BTreeSet::new(),
            status: EvidenceGraphStoreStatus {
                recovered_partial_tail_bytes,
                ..EvidenceGraphStoreStatus::default()
            },
        };
        store.replay()?;
        Ok(store)
    }

    pub fn append(&mut self, graph: EvidenceGraph) -> Result<bool, String> {
        verify_graph(&graph)?;
        let packed_graph_sha256 = decode_graph_sha256(&graph.graph_sha256)?;
        if self.seen_graphs.contains(&packed_graph_sha256) {
            self.status.duplicate_graph_total = self.status.duplicate_graph_total.saturating_add(1);
            return Ok(false);
        }
        let sequence = self.next_sequence;
        let event_count = graph.event_graph_sha256.len();
        let atom_count = graph.atoms.len();
        let event_set_sha256 = canonical_json_sha256(&graph.event_graph_sha256)
            .map_err(|error| format!("evidence_graph_event_set_digest:{error}"))?;
        let record_sha256 = canonical_json_sha256(&EvidenceGraphReceiptDigestMaterial {
            schema: EVIDENCE_GRAPH_RECEIPT_SCHEMA_V2,
            sequence,
            previous_record_sha256: &self.previous_record_sha256,
            graph_sha256: &graph.graph_sha256,
            event_set_sha256: &event_set_sha256,
            event_graph_sha256: &graph.event_graph_sha256,
            event_count,
            atom_count,
            policy: graph.policy,
        })
        .map_err(|error| format!("evidence_graph_receipt_digest:{error}"))?;
        let record = EvidenceGraphReceipt {
            schema: EVIDENCE_GRAPH_RECEIPT_SCHEMA_V2.to_owned(),
            sequence,
            previous_record_sha256: self.previous_record_sha256.clone(),
            graph_sha256: graph.graph_sha256,
            event_set_sha256,
            event_graph_sha256: graph.event_graph_sha256,
            event_count,
            atom_count,
            policy: graph.policy,
            record_sha256: record_sha256.clone(),
        };
        let mut bytes = canonical_json_bytes(&record)
            .map_err(|error| format!("evidence_graph_record_encode:{error}"))?;
        bytes.push(b'\n');
        let new_file = !self.path.exists();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| {
                format!("evidence_graph_store_open:{}:{error}", self.path.display())
            })?;
        file.write_all(&bytes)
            .map_err(|error| format!("evidence_graph_store_write:{error}"))?;
        file.sync_data()
            .map_err(|error| format!("evidence_graph_store_sync:{error}"))?;
        if new_file {
            sync_parent_directory(&self.path)?;
        }
        self.seen_graphs.insert(packed_graph_sha256);
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.previous_record_sha256 = record_sha256;
        self.status.graph_total = self.status.graph_total.saturating_add(1);
        Ok(true)
    }

    #[must_use]
    pub fn status(&self) -> EvidenceGraphStoreStatus {
        self.status
    }

    fn replay(&mut self) -> Result<(), String> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(format!(
                    "evidence_graph_store_read:{}:{error}",
                    self.path.display()
                ));
            }
        };
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|error| format!("evidence_graph_store_line:{error}"))?;
            if line.is_empty() {
                return Err("evidence_graph_store_empty_record".to_owned());
            }
            let record: EvidenceGraphReceipt = serde_json::from_str(&line)
                .map_err(|error| format!("evidence_graph_store_decode:{error}"))?;
            if record.schema != EVIDENCE_GRAPH_RECEIPT_SCHEMA_V2
                || record.sequence != self.next_sequence
                || record.previous_record_sha256 != self.previous_record_sha256
            {
                return Err("evidence_graph_store_chain_mismatch".to_owned());
            }
            if record.event_count == 0 || record.atom_count == 0 {
                return Err("evidence_graph_store_empty_receipt".to_owned());
            }
            let expected = canonical_json_sha256(&EvidenceGraphReceiptDigestMaterial {
                schema: EVIDENCE_GRAPH_RECEIPT_SCHEMA_V2,
                sequence: record.sequence,
                previous_record_sha256: &record.previous_record_sha256,
                graph_sha256: &record.graph_sha256,
                event_set_sha256: &record.event_set_sha256,
                event_graph_sha256: &record.event_graph_sha256,
                event_count: record.event_count,
                atom_count: record.atom_count,
                policy: record.policy,
            })
            .map_err(|error| format!("evidence_graph_receipt_digest:{error}"))?;
            if record.record_sha256 != expected {
                return Err("evidence_graph_store_record_digest_mismatch".to_owned());
            }
            if record.event_count != record.event_graph_sha256.len()
                || canonical_json_sha256(&record.event_graph_sha256)
                    .map_err(|error| format!("evidence_graph_event_set_digest:{error}"))?
                    != record.event_set_sha256
                || record
                    .event_graph_sha256
                    .iter()
                    .any(|digest| decode_graph_sha256(digest).is_err())
            {
                return Err("evidence_graph_store_event_set_mismatch".to_owned());
            }
            if !self
                .seen_graphs
                .insert(decode_graph_sha256(&record.graph_sha256)?)
            {
                return Err("evidence_graph_store_duplicate_graph".to_owned());
            }
            self.next_sequence = self.next_sequence.saturating_add(1);
            self.previous_record_sha256 = record.record_sha256;
            self.status.graph_total = self.status.graph_total.saturating_add(1);
        }
        Ok(())
    }
}

fn decode_graph_sha256(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 {
        return Err("evidence_graph_digest_length".to_owned());
    }
    let mut digest = [0_u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&value[offset..offset + 2], 16)
            .map_err(|_| "evidence_graph_digest_hex".to_owned())?;
    }
    Ok(digest)
}

impl EvidenceGraphBuilder {
    pub fn build(
        events: &[CanonicalEventGraph],
        policy: EvidenceGraphPolicy,
    ) -> Result<EvidenceGraph, &'static str> {
        if policy.max_events == 0 || policy.max_atoms == 0 {
            return Err("evidence_graph_zero_budget");
        }
        if events.len() > policy.max_events {
            return Err("evidence_graph_event_budget_exceeded");
        }
        let mut ordered = events.iter().collect::<Vec<_>>();
        ordered.sort_by(|left, right| event_order_key(left).cmp(&event_order_key(right)));
        let mut atoms = Vec::new();
        let mut first_value = BTreeMap::<(EvidenceNodeType, String), EvidenceNodeRef>::new();
        let mut cardinalities = BTreeMap::<String, Vec<EvidenceNodeRef>>::new();
        let mut numbers = BTreeMap::<String, Vec<EvidenceNodeRef>>::new();
        let mut previous_by_session = BTreeMap::<String, String>::new();
        for event in &ordered {
            let path_indexes = event
                .nodes
                .iter()
                .enumerate()
                .map(|(index, node)| (node_path(node), index))
                .collect::<BTreeMap<_, _>>();
            push_atom(
                &mut atoms,
                policy,
                EvidenceGraphAtom::EventIncluded {
                    event_graph_sha256: event.graph_sha256.clone(),
                    source_stream_sha256: event.source_stream_sha256.clone(),
                    source_offset: event.source_offset,
                },
            )?;
            if let Some(intent) = &event.client_intent_id_sha256 {
                push_atom(
                    &mut atoms,
                    policy,
                    EvidenceGraphAtom::IntentMembership {
                        event_graph_sha256: event.graph_sha256.clone(),
                        client_intent_id_sha256: intent.clone(),
                    },
                )?;
            }
            if let Some(previous) = previous_by_session
                .insert(event.session_id_sha256.clone(), event.graph_sha256.clone())
            {
                push_atom(
                    &mut atoms,
                    policy,
                    EvidenceGraphAtom::TemporalBefore {
                        predecessor_event_sha256: previous,
                        successor_event_sha256: event.graph_sha256.clone(),
                        session_id_sha256: event.session_id_sha256.clone(),
                    },
                )?;
            }
            for (index, node) in event.nodes.iter().enumerate() {
                let node_index = u32::try_from(index).map_err(|_| "evidence_graph_node_index")?;
                let source = EvidenceNodeRef {
                    event_graph_sha256: event.graph_sha256.clone(),
                    node_index,
                };
                let node_type = node_type(node);
                push_atom(
                    &mut atoms,
                    policy,
                    EvidenceGraphAtom::TypedNode {
                        source: source.clone(),
                        node_type,
                    },
                )?;
                if let CanonicalEventNode::ParsedJson { source_path, .. } = node {
                    let source_index = path_indexes
                        .get(source_path.as_str())
                        .ok_or("evidence_graph_derived_source_missing")?;
                    push_atom(
                        &mut atoms,
                        policy,
                        EvidenceGraphAtom::DerivedFrom {
                            derived: source.clone(),
                            source: EvidenceNodeRef {
                                event_graph_sha256: event.graph_sha256.clone(),
                                node_index: u32::try_from(*source_index)
                                    .map_err(|_| "evidence_graph_node_index")?,
                            },
                        },
                    )?;
                }
                if let Some(count) = node_cardinality(node) {
                    push_atom(
                        &mut atoms,
                        policy,
                        EvidenceGraphAtom::Cardinality {
                            source: source.clone(),
                            count,
                        },
                    )?;
                    cardinalities
                        .entry(number_digest(count))
                        .or_default()
                        .push(source.clone());
                }
                if let Some(value_sha256) = node_value_sha256(node) {
                    let key = (node_type, value_sha256.to_owned());
                    if let Some(first) = first_value.get(&key) {
                        if first != &source {
                            push_atom(
                                &mut atoms,
                                policy,
                                EvidenceGraphAtom::ValueEquality {
                                    left: first.clone(),
                                    right: source.clone(),
                                },
                            )?;
                        }
                    } else {
                        first_value.insert(key, source.clone());
                    }
                }
                if matches!(node, CanonicalEventNode::Number { .. })
                    && let Some(value_sha256) = node_value_sha256(node)
                {
                    numbers
                        .entry(value_sha256.to_owned())
                        .or_default()
                        .push(source.clone());
                }
                push_structural_relations(&mut atoms, policy, event, node, &source, &path_indexes)?;
            }
        }
        for (digest, collections) in cardinalities {
            let Some(values) = numbers.get(&digest) else {
                continue;
            };
            for collection in &collections {
                for value in values {
                    push_atom(
                        &mut atoms,
                        policy,
                        EvidenceGraphAtom::CardinalityEquals {
                            collection: collection.clone(),
                            value: value.clone(),
                        },
                    )?;
                }
            }
        }
        let event_graph_sha256 = ordered
            .iter()
            .map(|event| event.graph_sha256.clone())
            .collect::<Vec<_>>();
        let graph_sha256 = canonical_json_sha256(&EvidenceGraphDigestMaterial {
            schema: EVIDENCE_GRAPH_SCHEMA_V1,
            policy,
            event_graph_sha256: &event_graph_sha256,
            atoms: &atoms,
        })?;
        Ok(EvidenceGraph {
            schema: EVIDENCE_GRAPH_SCHEMA_V1.to_owned(),
            policy,
            event_graph_sha256,
            atoms,
            graph_sha256,
        })
    }
}

fn push_atom(
    atoms: &mut Vec<EvidenceGraphAtom>,
    policy: EvidenceGraphPolicy,
    atom: EvidenceGraphAtom,
) -> Result<(), &'static str> {
    if atoms.len() >= policy.max_atoms {
        return Err("evidence_graph_atom_budget_exceeded");
    }
    atoms.push(atom);
    Ok(())
}

fn verify_graph(graph: &EvidenceGraph) -> Result<(), String> {
    if graph.schema != EVIDENCE_GRAPH_SCHEMA_V1 {
        return Err("evidence_graph_schema_mismatch".to_owned());
    }
    let expected = canonical_json_sha256(&EvidenceGraphDigestMaterial {
        schema: EVIDENCE_GRAPH_SCHEMA_V1,
        policy: graph.policy,
        event_graph_sha256: &graph.event_graph_sha256,
        atoms: &graph.atoms,
    })
    .map_err(|error| format!("evidence_graph_digest:{error}"))?;
    if graph.graph_sha256 != expected {
        return Err("evidence_graph_digest_mismatch".to_owned());
    }
    Ok(())
}

fn recover_partial_graph_tail(path: &Path) -> Result<u64, String> {
    let mut file = match OpenOptions::new().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => {
            return Err(format!(
                "evidence_graph_store_recovery_open:{}:{error}",
                path.display()
            ));
        }
    };
    let length = file
        .metadata()
        .map_err(|error| format!("evidence_graph_store_recovery_metadata:{error}"))?
        .len();
    if length == 0 {
        return Ok(0);
    }
    file.seek(SeekFrom::End(-1))
        .map_err(|error| format!("evidence_graph_store_recovery_seek:{error}"))?;
    let mut last = [0_u8; 1];
    file.read_exact(&mut last)
        .map_err(|error| format!("evidence_graph_store_recovery_read:{error}"))?;
    if last[0] == b'\n' {
        return Ok(0);
    }
    let window = length.min(64 * 1024);
    file.seek(SeekFrom::Start(length - window))
        .map_err(|error| format!("evidence_graph_store_recovery_seek:{error}"))?;
    let mut tail = vec![0_u8; window as usize];
    file.read_exact(&mut tail)
        .map_err(|error| format!("evidence_graph_store_recovery_read:{error}"))?;
    let retained = tail
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(length - window, |index| length - window + index as u64 + 1);
    let removed = length.saturating_sub(retained);
    file.set_len(retained)
        .map_err(|error| format!("evidence_graph_store_recovery_truncate:{error}"))?;
    file.sync_data()
        .map_err(|error| format!("evidence_graph_store_recovery_sync:{error}"))?;
    Ok(removed)
}

fn sync_parent_directory(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            format!(
                "evidence_graph_store_parent_sync:{}:{error}",
                parent.display()
            )
        })
}

fn event_order_key(event: &CanonicalEventGraph) -> (u64, &str, u64, &str) {
    let event_time = match event.event_time {
        EvidenceEventTime::Known { unix_nanos } => unix_nanos,
        EvidenceEventTime::Unknown => u64::MAX,
    };
    (
        event_time,
        event.source_stream_sha256.as_str(),
        event.source_offset,
        event.graph_sha256.as_str(),
    )
}

const fn node_type(node: &CanonicalEventNode) -> EvidenceNodeType {
    match node {
        CanonicalEventNode::Null { .. } => EvidenceNodeType::Null,
        CanonicalEventNode::Boolean { .. } => EvidenceNodeType::Boolean,
        CanonicalEventNode::Number { .. } => EvidenceNodeType::Number,
        CanonicalEventNode::String { .. } => EvidenceNodeType::String,
        CanonicalEventNode::ParsedJson { .. } => EvidenceNodeType::ParsedJson,
        CanonicalEventNode::Array { .. } => EvidenceNodeType::Array,
        CanonicalEventNode::Object { .. } => EvidenceNodeType::Object,
        CanonicalEventNode::ObjectField { .. } => EvidenceNodeType::ObjectField,
    }
}

const fn node_cardinality(node: &CanonicalEventNode) -> Option<usize> {
    match node {
        CanonicalEventNode::Array { len, .. } | CanonicalEventNode::Object { len, .. } => {
            Some(*len)
        }
        _ => None,
    }
}

fn node_value_sha256(node: &CanonicalEventNode) -> Option<&str> {
    match node {
        CanonicalEventNode::Number { value_sha256, .. }
        | CanonicalEventNode::String { value_sha256, .. } => Some(value_sha256),
        CanonicalEventNode::Boolean { value, .. } => {
            Some(if *value { "bool:true" } else { "bool:false" })
        }
        CanonicalEventNode::Null { .. } => Some("null"),
        CanonicalEventNode::Array { .. }
        | CanonicalEventNode::Object { .. }
        | CanonicalEventNode::ObjectField { .. }
        | CanonicalEventNode::ParsedJson { .. } => None,
    }
}

fn node_path(node: &CanonicalEventNode) -> &str {
    match node {
        CanonicalEventNode::Null { path }
        | CanonicalEventNode::Boolean { path, .. }
        | CanonicalEventNode::Number { path, .. }
        | CanonicalEventNode::String { path, .. }
        | CanonicalEventNode::ParsedJson { path, .. }
        | CanonicalEventNode::Array { path, .. }
        | CanonicalEventNode::Object { path, .. }
        | CanonicalEventNode::ObjectField { path, .. } => path,
    }
}

fn push_structural_relations(
    atoms: &mut Vec<EvidenceGraphAtom>,
    policy: EvidenceGraphPolicy,
    event: &CanonicalEventGraph,
    node: &CanonicalEventNode,
    source: &EvidenceNodeRef,
    path_indexes: &BTreeMap<&str, usize>,
) -> Result<(), &'static str> {
    let path = node_path(node);
    if let Some(value_path) = path.strip_suffix("#field") {
        let Some((record_path, _)) = value_path.rsplit_once(".f") else {
            return Err("evidence_graph_field_parent_missing");
        };
        let record = node_ref(event, path_indexes, record_path)?;
        let value = node_ref(event, path_indexes, value_path)?;
        push_atom(
            atoms,
            policy,
            EvidenceGraphAtom::FieldOf {
                record: record.clone(),
                field: source.clone(),
                value,
            },
        )?;
        return push_atom(
            atoms,
            policy,
            EvidenceGraphAtom::Contains {
                container: record,
                member: source.clone(),
            },
        );
    }
    if let Some(open) = path.rfind('[')
        && path.ends_with(']')
    {
        let parent_path = &path[..open];
        if let Ok(collection) = node_ref(event, path_indexes, parent_path) {
            push_atom(
                atoms,
                policy,
                EvidenceGraphAtom::Contains {
                    container: collection.clone(),
                    member: source.clone(),
                },
            )?;
            return push_atom(
                atoms,
                policy,
                EvidenceGraphAtom::MemberOf {
                    member: source.clone(),
                    collection,
                },
            );
        }
    }
    if let Some((parent_path, _)) = path.rsplit_once(".f")
        && let Ok(parent) = node_ref(event, path_indexes, parent_path)
    {
        push_atom(
            atoms,
            policy,
            EvidenceGraphAtom::Contains {
                container: parent,
                member: source.clone(),
            },
        )?;
    }
    Ok(())
}

fn node_ref(
    event: &CanonicalEventGraph,
    path_indexes: &BTreeMap<&str, usize>,
    path: &str,
) -> Result<EvidenceNodeRef, &'static str> {
    let index = path_indexes
        .get(path)
        .ok_or("evidence_graph_structural_source_missing")?;
    Ok(EvidenceNodeRef {
        event_graph_sha256: event.graph_sha256.clone(),
        node_index: u32::try_from(*index).map_err(|_| "evidence_graph_node_index")?,
    })
}

fn number_digest(value: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"nando.number-value.v1");
    hasher.update([0]);
    hasher.update(value.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DeterministicEvidenceLedger, EvidenceIngestOutcome, EvidencePolicyV1, RawEvidenceEnvelope,
    };
    use std::fs;

    fn event(offset: u64, payload: &[u8]) -> CanonicalEventGraph {
        let path = std::env::temp_dir().join(format!(
            "nando-evidence-graph-{}-{offset}-{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let _ = fs::remove_file(&path);
        let checkpoint_path = path.with_extension("checkpoint.json");
        let _ = fs::remove_file(&checkpoint_path);
        let mut ledger =
            DeterministicEvidenceLedger::open(&path, EvidencePolicyV1::default()).expect("ledger");
        let record = ledger
            .ingest(RawEvidenceEnvelope {
                source_stream_id: "stream".to_owned(),
                source_offset: offset,
                event_id: format!("event-{offset}"),
                session_id: "session".to_owned(),
                client_intent_id: Some("intent".to_owned()),
                call_id: Some(format!("call-{offset}")),
                output_ordinal: Some(offset as u32),
                event_time_unix_nanos: Some(offset),
                schema_version: 1,
                payload: payload.to_vec(),
            })
            .expect("ingest");
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(checkpoint_path);
        match record.outcome {
            EvidenceIngestOutcome::Normalized { graph } => graph,
            _ => panic!("normalized graph"),
        }
    }

    #[test]
    fn reducer_preserves_multi_output_order_cardinality_and_equality() {
        let first = event(1, br#"{"rows":[{"id":7},{"id":8}]}"#);
        let second = event(2, br#"{"selected":7}"#);
        let graph = EvidenceGraphBuilder::build(&[second, first], EvidenceGraphPolicy::default())
            .expect("evidence graph");
        assert_eq!(graph.event_graph_sha256.len(), 2);
        assert!(
            graph
                .atoms
                .iter()
                .any(|atom| matches!(atom, EvidenceGraphAtom::TemporalBefore { .. }))
        );
        assert!(
            graph
                .atoms
                .iter()
                .any(|atom| matches!(atom, EvidenceGraphAtom::Cardinality { count: 2, .. }))
        );
        assert!(
            graph
                .atoms
                .iter()
                .any(|atom| matches!(atom, EvidenceGraphAtom::ValueEquality { .. }))
        );
    }

    #[test]
    fn every_relation_atom_carries_explicit_source_reference() {
        let event = event(1, br#"{"items":[true,false]}"#);
        let graph = EvidenceGraphBuilder::build(&[event], EvidenceGraphPolicy::default())
            .expect("evidence graph");
        assert!(!graph.atoms.is_empty());
        assert!(graph.atoms.iter().all(|atom| match atom {
            EvidenceGraphAtom::EventIncluded {
                event_graph_sha256, ..
            }
            | EvidenceGraphAtom::IntentMembership {
                event_graph_sha256, ..
            } => {
                !event_graph_sha256.is_empty()
            }
            EvidenceGraphAtom::TypedNode { source, .. }
            | EvidenceGraphAtom::Cardinality { source, .. } => {
                !source.event_graph_sha256.is_empty()
            }
            EvidenceGraphAtom::CardinalityEquals { collection, value } => {
                !collection.event_graph_sha256.is_empty() && !value.event_graph_sha256.is_empty()
            }
            EvidenceGraphAtom::Contains { container, member } => {
                !container.event_graph_sha256.is_empty() && !member.event_graph_sha256.is_empty()
            }
            EvidenceGraphAtom::FieldOf {
                record,
                field,
                value,
            } => {
                !record.event_graph_sha256.is_empty()
                    && !field.event_graph_sha256.is_empty()
                    && !value.event_graph_sha256.is_empty()
            }
            EvidenceGraphAtom::MemberOf { member, collection } => {
                !member.event_graph_sha256.is_empty() && !collection.event_graph_sha256.is_empty()
            }
            EvidenceGraphAtom::ValueEquality { left, right } => {
                !left.event_graph_sha256.is_empty() && !right.event_graph_sha256.is_empty()
            }
            EvidenceGraphAtom::DerivedFrom { derived, source } => {
                !derived.event_graph_sha256.is_empty() && !source.event_graph_sha256.is_empty()
            }
            EvidenceGraphAtom::TemporalBefore {
                predecessor_event_sha256,
                successor_event_sha256,
                ..
            } => !predecessor_event_sha256.is_empty() && !successor_event_sha256.is_empty(),
        }));
    }

    #[test]
    fn graph_store_is_hash_chained_idempotent_and_restart_safe() {
        let root = std::env::temp_dir().join(format!(
            "nando-evidence-graph-store-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let path = root.join("graphs.jsonl");
        let graph = EvidenceGraphBuilder::build(
            &[event(7, br#"{"items":[1,2,3]}"#)],
            EvidenceGraphPolicy::default(),
        )
        .expect("graph");
        let mut store = DeterministicEvidenceGraphStore::open(&path).expect("store");
        assert!(store.append(graph.clone()).expect("first append"));
        assert!(!store.append(graph).expect("duplicate append"));
        assert_eq!(
            store.status(),
            EvidenceGraphStoreStatus {
                graph_total: 1,
                duplicate_graph_total: 1,
                recovered_partial_tail_bytes: 0,
            }
        );
        drop(store);

        let restored = DeterministicEvidenceGraphStore::open(&path).expect("restored");
        assert_eq!(restored.status().graph_total, 1);
        assert_eq!(restored.status().duplicate_graph_total, 0);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn graph_store_recovers_only_incomplete_tail_and_rejects_committed_corruption() {
        let root = std::env::temp_dir().join(format!(
            "nando-evidence-graph-corruption-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let path = root.join("graphs.jsonl");
        let graph = EvidenceGraphBuilder::build(
            &[event(8, br#"{"value":8}"#)],
            EvidenceGraphPolicy::default(),
        )
        .expect("graph");
        let mut store = DeterministicEvidenceGraphStore::open(&path).expect("store");
        store.append(graph).expect("append");
        drop(store);
        OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("tail open")
            .write_all(b"partial")
            .expect("tail write");
        let recovered = DeterministicEvidenceGraphStore::open(&path).expect("recover");
        assert_eq!(recovered.status().recovered_partial_tail_bytes, 7);
        drop(recovered);

        let mut bytes = fs::read(&path).expect("bytes");
        let index = bytes
            .iter()
            .position(|byte| *byte == b'0')
            .expect("digest byte");
        bytes[index] = b'1';
        fs::write(&path, bytes).expect("corrupt");
        assert!(DeterministicEvidenceGraphStore::open(&path).is_err());
        fs::remove_dir_all(root).expect("cleanup");
    }
}
