//! Durable active-generation pointer for autonomous MS3 failure rollover.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use nando_operator_learning::multi_source::{
    Ms3GenerationRegistryV1, Ms3IndependentFutureVerdictV1,
};
use serde::{Deserialize, Serialize};

use crate::ms3_frozen_version_space::Ms3FrozenVersionSpaceRuntime;
use crate::ms3_linked_frame_acquisition::Ms3LinkedFrameAcquisitionRuntime;
use crate::multi_source_topology_archive::MultiSourceTopologyArchive;

const LIFECYCLE_SCHEMA_V1: &str = "nando.ms3-generation-lifecycle.v1";
const LIFECYCLE_FILE: &str = "generation-lifecycle-v1.cbor";
const LEGACY_VERSION_SPACE_DIRECTORY: &str = "version-space-v1";
const GENERATIONS_DIRECTORY: &str = "generations";
const GENERATION_REGISTRY_FILE: &str = "generation-registry-v1.cbor";
const MAX_LIFECYCLE_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Ms3GenerationLifecycleManifestV1 {
    pub schema: String,
    pub manifest_root_sha256: String,
    pub active_generation_sequence: u64,
    pub acquisition_contract_root_sha256: String,
    pub topology_prefix_root_sha256: String,
    pub topology_watermark_rows: u64,
    pub predecessor_terminal_root_sha256: Option<String>,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

pub(super) struct Ms3GenerationLifecycleRuntime {
    root: PathBuf,
    registry_path: PathBuf,
    manifest_path: PathBuf,
    manifest: Ms3GenerationLifecycleManifestV1,
    max_new_topology_rows: u64,
    max_elapsed_seconds: u64,
}

pub(super) struct Ms3ActiveGenerationRuntimes {
    pub acquisition: Ms3LinkedFrameAcquisitionRuntime,
    pub frozen: Ms3FrozenVersionSpaceRuntime,
}

impl Ms3GenerationLifecycleRuntime {
    pub(super) fn open(
        root: &Path,
        topology_archive: &MultiSourceTopologyArchive,
        opened_at_unix: u64,
        max_new_topology_rows: u64,
        max_elapsed_seconds: u64,
    ) -> Result<(Self, Ms3ActiveGenerationRuntimes), String> {
        fs::create_dir_all(root).map_err(|error| format!("ms3_lifecycle_directory:{error}"))?;
        let registry_path = root
            .join(LEGACY_VERSION_SPACE_DIRECTORY)
            .join(GENERATION_REGISTRY_FILE);
        let manifest_path = root.join(LIFECYCLE_FILE);
        let restored_manifest = read_bounded(&manifest_path)?
            .map(|bytes| Ms3GenerationLifecycleManifestV1::from_canonical_bytes(&bytes))
            .transpose()?;
        let active_generation_sequence = match &restored_manifest {
            Some(manifest) => manifest.active_generation_sequence,
            None => infer_legacy_generation(root, &registry_path)?,
        };
        let (acquisition_path, version_space_path) =
            generation_paths(root, active_generation_sequence);
        let acquisition = Ms3LinkedFrameAcquisitionRuntime::open_generation(
            &acquisition_path,
            active_generation_sequence,
            topology_archive,
            opened_at_unix,
            max_new_topology_rows,
            max_elapsed_seconds,
        )?;
        let frozen = Ms3FrozenVersionSpaceRuntime::open(
            &version_space_path,
            &registry_path,
            active_generation_sequence,
            topology_archive.max_bridge_sequence(),
            opened_at_unix,
        )?;
        let manifest = match restored_manifest {
            Some(manifest) => manifest,
            None => {
                let predecessor_terminal_root_sha256 = predecessor_terminal_root(
                    frozen.generation_registry(),
                    active_generation_sequence,
                )?;
                let manifest = Ms3GenerationLifecycleManifestV1::seal(
                    active_generation_sequence,
                    acquisition.contract(),
                    predecessor_terminal_root_sha256,
                )?;
                write_atomic(&manifest_path, &manifest.canonical_bytes()?)?;
                manifest
            }
        };
        validate_runtime_binding(&manifest, &acquisition, &frozen)?;
        Ok((
            Self {
                root: root.to_path_buf(),
                registry_path,
                manifest_path,
                manifest,
                max_new_topology_rows,
                max_elapsed_seconds,
            },
            Ms3ActiveGenerationRuntimes {
                acquisition,
                frozen,
            },
        ))
    }

    pub(super) fn prepare_successor(
        &mut self,
        topology_archive: &MultiSourceTopologyArchive,
        opened_at_unix: u64,
        current_acquisition: &Ms3LinkedFrameAcquisitionRuntime,
        current_frozen: &Ms3FrozenVersionSpaceRuntime,
    ) -> Result<Ms3ActiveGenerationRuntimes, String> {
        validate_runtime_binding(&self.manifest, current_acquisition, current_frozen)?;
        let registry = current_frozen.generation_registry();
        let closure_capture_sequence = registry
            .generation_closure_capture_sequence(self.manifest.active_generation_sequence)
            .ok_or_else(|| "ms3_lifecycle_closure_sequence_missing".to_owned())?;
        let linked_failure =
            registry.linked_acquisition_failure(self.manifest.active_generation_sequence);
        let predecessor_terminal_root_sha256 = if let Some(failure) = linked_failure {
            failure.receipt_root_sha256.clone()
        } else {
            let current_entry = registry
                .generations
                .iter()
                .find(|entry| entry.generation_sequence == self.manifest.active_generation_sequence)
                .ok_or_else(|| "ms3_lifecycle_terminal_registry_missing".to_owned())?;
            if let Some(future) = current_frozen.independent_future() {
                if future.receipt.verdict != Ms3IndependentFutureVerdictV1::Contradiction {
                    return Err("ms3_lifecycle_successor_requires_terminal_failure".to_owned());
                }
                let terminal = current_entry
                    .terminal
                    .as_ref()
                    .ok_or_else(|| "ms3_lifecycle_terminal_registry_missing".to_owned())?;
                if terminal.verdict != Ms3IndependentFutureVerdictV1::Contradiction
                    || terminal.future_receipt_root_sha256 != future.receipt.receipt_root_sha256
                {
                    return Err("ms3_lifecycle_terminal_registry_mismatch".to_owned());
                }
                terminal.terminal_root_sha256.clone()
            } else if let Some(failure) = &current_entry.acquisition_failure {
                failure.receipt_root_sha256.clone()
            } else {
                return Err("ms3_lifecycle_successor_requires_terminal_failure".to_owned());
            }
        };
        let next_generation_sequence = self
            .manifest
            .active_generation_sequence
            .checked_add(1)
            .ok_or_else(|| "ms3_lifecycle_generation_overflow".to_owned())?;
        let (acquisition_path, version_space_path) =
            generation_paths(&self.root, next_generation_sequence);
        let successor_cursor_rows = if let Some(failure) = linked_failure {
            let terminal_report = current_acquisition
                .terminal_report()
                .ok_or_else(|| "ms3_lifecycle_successor_terminal_report_missing".to_owned())?;
            if terminal_report.acquisition_contract.contract_root_sha256
                != failure.acquisition_contract_root_sha256
                || terminal_report.report_root_sha256 != failure.acquisition_report_root_sha256
            {
                return Err("ms3_lifecycle_successor_terminal_report_binding_invalid".to_owned());
            }
            let report_cursor = current_acquisition
                .consumed_topology_cursor_rows()
                .ok_or_else(|| "ms3_lifecycle_successor_cursor_missing".to_owned())?;
            if failure.consumed_topology_cursor_rows > 0
                && failure.consumed_topology_cursor_rows != report_cursor
            {
                return Err("ms3_lifecycle_successor_cursor_binding_invalid".to_owned());
            }
            let cursor = usize::try_from(report_cursor)
                .map_err(|_| "ms3_lifecycle_successor_cursor_range".to_owned())?;
            if cursor > topology_archive.len()
                || topology_archive.bridge_sequence_at_cursor(cursor)? != closure_capture_sequence
            {
                return Err("ms3_lifecycle_successor_cursor_binding_invalid".to_owned());
            }
            cursor
        } else {
            topology_archive.cursor_after_bridge_sequence(closure_capture_sequence)?
        };
        let acquisition = Ms3LinkedFrameAcquisitionRuntime::open_generation_at_cursor(
            &acquisition_path,
            next_generation_sequence,
            topology_archive,
            Some(
                u64::try_from(successor_cursor_rows)
                    .map_err(|_| "ms3_lifecycle_successor_cursor_range".to_owned())?,
            ),
            opened_at_unix,
            self.max_new_topology_rows,
            self.max_elapsed_seconds,
        )?;
        let frozen = Ms3FrozenVersionSpaceRuntime::open(
            &version_space_path,
            &self.registry_path,
            next_generation_sequence,
            topology_archive.max_bridge_sequence(),
            opened_at_unix,
        )?;
        if frozen.envelope().is_some() || frozen.independent_future().is_some() {
            return Err("ms3_lifecycle_successor_not_fresh".to_owned());
        }
        let manifest = Ms3GenerationLifecycleManifestV1::seal(
            next_generation_sequence,
            acquisition.contract(),
            Some(predecessor_terminal_root_sha256),
        )?;
        validate_runtime_binding(&manifest, &acquisition, &frozen)?;
        write_atomic(&self.manifest_path, &manifest.canonical_bytes()?)?;
        self.manifest = manifest;
        Ok(Ms3ActiveGenerationRuntimes {
            acquisition,
            frozen,
        })
    }

    pub(super) const fn manifest(&self) -> &Ms3GenerationLifecycleManifestV1 {
        &self.manifest
    }
}

impl Ms3GenerationLifecycleManifestV1 {
    fn seal(
        active_generation_sequence: u64,
        acquisition: &nando_operator_learning::multi_source::Ms3LinkedFrameAcquisitionContractV1,
        predecessor_terminal_root_sha256: Option<String>,
    ) -> Result<Self, String> {
        let mut manifest = Self {
            schema: LIFECYCLE_SCHEMA_V1.to_owned(),
            manifest_root_sha256: String::new(),
            active_generation_sequence,
            acquisition_contract_root_sha256: acquisition.contract_root_sha256.clone(),
            topology_prefix_root_sha256: acquisition.topology_prefix_root_sha256.clone(),
            topology_watermark_rows: acquisition.topology_watermark_rows,
            predecessor_terminal_root_sha256,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        manifest.manifest_root_sha256 = manifest.expected_root()?;
        manifest
            .validate()
            .then_some(manifest)
            .ok_or_else(|| "ms3_lifecycle_manifest_invalid".to_owned())
    }

    fn validate(&self) -> bool {
        self.schema == LIFECYCLE_SCHEMA_V1
            && self.active_generation_sequence > 0
            && valid_nonzero_sha256(&self.manifest_root_sha256)
            && valid_nonzero_sha256(&self.acquisition_contract_root_sha256)
            && valid_nonzero_sha256(&self.topology_prefix_root_sha256)
            && (if self.active_generation_sequence == 1 {
                self.predecessor_terminal_root_sha256.is_none()
            } else {
                self.predecessor_terminal_root_sha256
                    .as_ref()
                    .is_some_and(|root| valid_nonzero_sha256(root))
            })
            && !self.authority_ready
            && !self.phase_mutation_allowed
            && self
                .expected_root()
                .is_ok_and(|root| root == self.manifest_root_sha256)
    }

    fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        if !self.validate() {
            return Err("ms3_lifecycle_manifest_invalid".to_owned());
        }
        let bytes = serde_cbor::to_vec(self)
            .map_err(|error| format!("ms3_lifecycle_manifest_encode:{error}"))?;
        if bytes.is_empty() || bytes.len() > MAX_LIFECYCLE_BYTES {
            return Err("ms3_lifecycle_state_budget".to_owned());
        }
        Ok(bytes)
    }

    fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > MAX_LIFECYCLE_BYTES {
            return Err("ms3_lifecycle_state_budget".to_owned());
        }
        let manifest: Self = serde_cbor::from_slice(bytes)
            .map_err(|error| format!("ms3_lifecycle_manifest_decode:{error}"))?;
        if !manifest.validate() || manifest.canonical_bytes()? != bytes {
            return Err("ms3_lifecycle_manifest_invalid".to_owned());
        }
        Ok(manifest)
    }

    fn expected_root(&self) -> Result<String, String> {
        canonical_json_sha256(&(
            LIFECYCLE_SCHEMA_V1,
            self.active_generation_sequence,
            self.acquisition_contract_root_sha256.as_str(),
            self.topology_prefix_root_sha256.as_str(),
            self.topology_watermark_rows,
            self.predecessor_terminal_root_sha256.as_deref(),
            false,
            false,
        ))
        .map_err(|error| format!("ms3_lifecycle_manifest_root:{error}"))
    }
}

fn validate_runtime_binding(
    manifest: &Ms3GenerationLifecycleManifestV1,
    acquisition: &Ms3LinkedFrameAcquisitionRuntime,
    frozen: &Ms3FrozenVersionSpaceRuntime,
) -> Result<(), String> {
    if !manifest.validate()
        || acquisition.generation_sequence() != manifest.active_generation_sequence
        || frozen.generation_sequence() != manifest.active_generation_sequence
        || acquisition.contract().contract_root_sha256 != manifest.acquisition_contract_root_sha256
        || acquisition.contract().topology_prefix_root_sha256
            != manifest.topology_prefix_root_sha256
        || acquisition.contract().topology_watermark_rows != manifest.topology_watermark_rows
        || predecessor_terminal_root(
            frozen.generation_registry(),
            manifest.active_generation_sequence,
        )? != manifest.predecessor_terminal_root_sha256
    {
        return Err("ms3_lifecycle_runtime_binding_invalid".to_owned());
    }
    Ok(())
}

fn predecessor_terminal_root(
    registry: &Ms3GenerationRegistryV1,
    generation_sequence: u64,
) -> Result<Option<String>, String> {
    if generation_sequence == 1 {
        return Ok(None);
    }
    registry
        .closure_root(generation_sequence - 1)
        .map(str::to_owned)
        .map(Some)
        .ok_or_else(|| "ms3_lifecycle_predecessor_missing".to_owned())
}

fn infer_legacy_generation(root: &Path, registry_path: &Path) -> Result<u64, String> {
    let generations_path = root.join(GENERATIONS_DIRECTORY);
    if generations_path.is_dir()
        && fs::read_dir(&generations_path)
            .map_err(|error| format!("ms3_lifecycle_generations_read:{error}"))?
            .next()
            .is_some()
    {
        return Err("ms3_lifecycle_manifest_missing".to_owned());
    }
    let registry = read_bounded(registry_path)?
        .map(|bytes| {
            Ms3GenerationRegistryV1::from_canonical_bytes(&bytes)
                .map_err(|error| format!("ms3_lifecycle_registry_restore:{error:?}"))
        })
        .transpose()?
        .unwrap_or_default();
    Ok(registry
        .generations
        .last()
        .map_or(1, |entry| entry.generation_sequence))
}

fn generation_paths(root: &Path, generation_sequence: u64) -> (PathBuf, PathBuf) {
    if generation_sequence == 1 {
        return (
            root.to_path_buf(),
            root.join(LEGACY_VERSION_SPACE_DIRECTORY),
        );
    }
    let generation_root = root
        .join(GENERATIONS_DIRECTORY)
        .join(format!("{generation_sequence:020}"));
    (
        generation_root.join("acquisition-v1"),
        generation_root.join(LEGACY_VERSION_SPACE_DIRECTORY),
    )
}

fn read_bounded(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("ms3_lifecycle_open:{}:{error}", path.display())),
    };
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(u64::try_from(MAX_LIFECYCLE_BYTES + 1).unwrap_or(u64::MAX))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("ms3_lifecycle_read:{}:{error}", path.display()))?;
    if bytes.is_empty() || bytes.len() > MAX_LIFECYCLE_BYTES {
        return Err("ms3_lifecycle_state_budget".to_owned());
    }
    Ok(Some(bytes))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() || bytes.len() > MAX_LIFECYCLE_BYTES {
        return Err("ms3_lifecycle_state_budget".to_owned());
    }
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("ms3_lifecycle_write_open:{error}"))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("ms3_lifecycle_write:{error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("ms3_lifecycle_rename:{error}"))?;
    File::open(
        path.parent()
            .ok_or_else(|| "ms3_lifecycle_parent_missing".to_owned())?,
    )
    .and_then(|directory| directory.sync_all())
    .map_err(|error| format!("ms3_lifecycle_directory_sync:{error}"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use nando_operator_kernel::{
        AtomSource, AtomValueType, LEARNING_REQUEST_STRUCTURE_SCHEMA_V2,
        MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceEvidenceOriginV1,
        MultiSourceExtractionStatusV1, MultiSourceRelationEdgeV1, MultiSourceRelationKindV1,
        MultiSourceRoleNodeV1, MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1,
        MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1, PreActionTopologyCommitV1,
        RELATION_FRAME_SCHEMA, RelationAtom, RelationFrame, canonical_json_sha256, sha256_bytes,
    };
    use nando_operator_learning::{
        RuntimeParityCase, SOURCE_NEUTRAL_EXTRACTOR_VERSION,
        multi_source::{
            Ms3VersionSpaceVersionsV1, PreActionTopologyAuditRowV1, TransportBindingLedgerV1,
            TransportTerminalReceiptV1, prepare_ms3_frozen_version_space_v1,
            seal_ms3_independent_future_v1,
        },
    };
    use nando_operator_runtime::response_pre_action_context_atom_ids;
    use nando_response_actor::{
        Ms4ExternalAdmissionCandidateV1, ResponseExecutionStatus, ResponseExecutor,
        build_ms4_external_admission_snapshot, request_phase_atom_ids,
    };
    use serde_json::json;

    use super::*;

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn test_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nando-ms3-lifecycle-{label}-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn hash(label: &str) -> String {
        sha256_bytes(label.as_bytes())
    }

    fn runtime_payload() -> serde_json::Value {
        json!({
            "input": [
                {
                    "type": "message",
                    "role": "user",
                    "content": "Return opaque"
                },
                {
                    "type": "function_call_output",
                    "output": "{\"opaque\":7}"
                }
            ]
        })
    }

    fn topology(
        label: &str,
        request_event: &str,
        session: &str,
        capture_sequence: u64,
        captured_at_unix_ms: u64,
    ) -> PreActionTopologyAuditRowV1 {
        let action_event = request_event.replacen("request", "action", 1);
        let topology = PreActionMultiSourceTopologyV1 {
            extraction_status: MultiSourceExtractionStatusV1::Complete,
            grounded_output_count: 1,
            output_part_count: 1,
            roles: vec![MultiSourceRoleNodeV1 {
                local_role_id: 0,
                source_ordinal: 0,
                value_ordinal: 0,
                type_class: MultiSourceTypeClassV1::Number,
                container_class: MultiSourceContainerClassV1::Scalar,
                cardinality_class: MultiSourceCardinalityClassV1::One,
                temporal_class: MultiSourceTemporalClassV1::Latest,
                depth_bucket: 1,
                structural_flags: 1,
            }],
            role_witnesses: vec![MultiSourceRoleWitnessV1 {
                local_role_id: 0,
                value_sha256: hash(&format!("value:{action_event}")),
                request_reference_ordinal: Some(0),
                request_reference_ordinal_candidates: Vec::new(),
            }],
            relations: vec![MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::RequestReferencesRole,
                source_role_id: 0,
                target_role_id: 0,
            }],
        };
        let structure = nando_operator_kernel::LearningRequestStructureV2 {
            schema: LEARNING_REQUEST_STRUCTURE_SCHEMA_V2.to_owned(),
            turn_intent_id_sha256: hash(label),
            request_event_id_sha256: hash(request_event),
            provider_bound_turn_identity: true,
            session_lineage_roots_sha256: vec![hash(session)],
            request_phase_atom_ids: request_phase_atom_ids("Return opaque"),
            pre_action_context_atom_ids: response_pre_action_context_atom_ids(&runtime_payload()),
            capability_atom_ids: vec![3],
            estimated_input_tokens: 100,
            provider_payload_bytes: 400,
            provider_capture_request_root_sha256: hash(&format!("request:{capture_sequence}")),
            decidability_reason_code: "pre_action_pending".to_owned(),
            topology,
        };
        let commit = PreActionTopologyCommitV1::seal(
            &structure,
            MultiSourceEvidenceOriginV1::FreshLive,
            hash("extractor"),
            hash("config"),
            capture_sequence,
        )
        .expect("topology commit");
        PreActionTopologyAuditRowV1 {
            bridge_epoch_sha256: hash("bridge"),
            bridge_sequence: Some(capture_sequence),
            record_sha256: Some(hash(&format!("record:{capture_sequence}"))),
            capture_epoch_sha256: Some(hash("capture-epoch")),
            capture_event_sha256: Some(hash(&format!("capture-event:{capture_sequence}"))),
            capture_receipt_sha256: Some(hash(&format!("receipt:{capture_sequence}"))),
            captured_at_unix_ms: Some(captured_at_unix_ms),
            session_lineage_sha256: Some(hash(session)),
            physical_order_proven: true,
            structure,
            commit,
        }
    }

    fn completed_frame(
        label: &str,
        action_event: &str,
        session: &str,
        observed_at_unix_ms: u64,
        verifier_label: bool,
    ) -> RelationFrame {
        let value_root = hash(&format!("value:{action_event}"));
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: hash(&format!("frame:{action_event}")),
            event_id_sha256: hash(action_event),
            client_intent_id_sha256: hash(label),
            session_id_sha256: hash(session),
            observed_at_unix_nanos: observed_at_unix_ms.saturating_mul(1_000_000),
            estimated_input_tokens: 100,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(verifier_label),
            atoms: vec![
                RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
                RelationAtom::TypedSlot {
                    slot_id: 7,
                    value_type: AtomValueType::Integer,
                    source: AtomSource::Observation,
                    value_sha256: value_root.clone(),
                },
                RelationAtom::UniqueSlot { slot_id: 7 },
                RelationAtom::ObservationSelector {
                    slot_id: 7,
                    selector: nando_operator_kernel::ResponseValueSelector::JsonField {
                        field: "opaque".to_owned(),
                        value_type: AtomValueType::Integer,
                    },
                },
                RelationAtom::TypedSlot {
                    slot_id: 11,
                    value_type: AtomValueType::Integer,
                    source: AtomSource::Action,
                    value_sha256: value_root,
                },
                RelationAtom::SlotEquality {
                    left_slot: 7,
                    right_slot: 11,
                },
                RelationAtom::ActionFunction {
                    value: "transport_a".to_owned(),
                },
                RelationAtom::ActionRoleArgument {
                    name: "value".to_owned(),
                    slot_id: 11,
                    value_type: Some(AtomValueType::Integer),
                },
            ],
            evidence_ref_sha256: hash(&format!("evidence:{action_event}")),
        }
    }

    fn terminal(
        request_event: &str,
        started_at_unix_ms: u64,
        completed_at_unix_ms: u64,
    ) -> TransportTerminalReceiptV1 {
        TransportTerminalReceiptV1::seal(
            hash(request_event),
            started_at_unix_ms.saturating_mul(1_000_000),
            completed_at_unix_ms.saturating_mul(1_000_000),
            200,
        )
        .expect("terminal receipt")
    }

    fn terminal_generation(
        root: &Path,
        future_verifier_label: Option<bool>,
    ) -> (
        MultiSourceTopologyArchive,
        Ms3GenerationLifecycleRuntime,
        Ms3ActiveGenerationRuntimes,
    ) {
        let opened_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock");
        let opened_at_unix = opened_at.as_secs();
        let opened_at_ms = u64::try_from(opened_at.as_millis()).expect("milliseconds");
        let topology_root = root.join("topologies");
        let mut topology_archive =
            MultiSourceTopologyArchive::open(&topology_root).expect("topology archive");
        let (lifecycle, mut runtimes) = Ms3GenerationLifecycleRuntime::open(
            root,
            &topology_archive,
            opened_at_unix,
            256,
            86_400,
        )
        .expect("lifecycle");

        let support_topology = topology(
            "support",
            "request-support",
            "support-lineage",
            1,
            opened_at_ms,
        );
        let support_frame = completed_frame(
            "support",
            "action-support",
            "support-lineage",
            opened_at_ms.saturating_add(500),
            true,
        );
        let support_terminal = terminal(
            "request-support",
            opened_at_ms.saturating_sub(10),
            opened_at_ms.saturating_add(100),
        );
        topology_archive
            .append(&support_topology)
            .expect("support topology");
        let report = runtimes
            .acquisition
            .evaluate(
                opened_at_unix,
                vec![support_topology.clone()],
                vec![support_frame.clone()],
                vec![support_terminal.clone()],
            )
            .expect("terminal acquisition");
        assert!(report.is_terminal());
        let binding = TransportBindingLedgerV1::build(
            std::slice::from_ref(&support_topology),
            std::slice::from_ref(&support_frame),
            std::slice::from_ref(&support_terminal),
        );
        let bound = &binding.bound_for_topology(&support_topology.commit.commitment_root_sha256)[0];
        let prepared = prepare_ms3_frozen_version_space_v1(&report, bound, &support_frame)
            .expect("prepared version space");
        runtimes
            .frozen
            .freeze(
                prepared,
                1,
                Ms3VersionSpaceVersionsV1 {
                    compiler_version: "test-compiler.v1".to_owned(),
                    vm_abi: "test-vm.v1".to_owned(),
                },
                opened_at_unix,
            )
            .expect("frozen law");
        let Some(pass) = future_verifier_label else {
            return (topology_archive, lifecycle, runtimes);
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock");
        let now_ms = u64::try_from(now.as_millis()).expect("milliseconds");
        let now_nanos = u64::try_from(now.as_nanos()).expect("nanoseconds");
        let future_topology = topology("future", "request-future", "future-lineage", 2, now_ms);
        topology_archive
            .append(&future_topology)
            .expect("future topology");
        assert!(
            runtimes
                .frozen
                .observe_topology(&future_topology, now_nanos)
                .expect("durable prediction")
        );
        let prediction = runtimes
            .frozen
            .predictions()
            .into_iter()
            .next()
            .expect("prediction");
        let (applicability_root, durable_at) = runtimes
            .frozen
            .prediction_commitment(&prediction.prediction_root_sha256)
            .expect("durable applicability");
        let future_frame = completed_frame(
            "future",
            "action-future",
            "future-lineage",
            now_ms.saturating_add(10_000),
            pass,
        );
        let future_terminal = terminal(
            "request-future",
            now_ms.saturating_sub(1),
            now_ms.saturating_add(11_000),
        );
        let future_binding = TransportBindingLedgerV1::build(
            std::slice::from_ref(&future_topology),
            std::slice::from_ref(&future_frame),
            std::slice::from_ref(&future_terminal),
        );
        let future_bound =
            &future_binding.bound_for_topology(&future_topology.commit.commitment_root_sha256)[0];
        let frozen = runtimes.frozen.envelope().expect("frozen envelope");
        let future = seal_ms3_independent_future_v1(
            frozen,
            &prediction,
            &applicability_root,
            durable_at,
            future_bound,
            &future_frame,
        )
        .expect("independent future");
        assert_eq!(
            future.receipt.verdict,
            if pass {
                Ms3IndependentFutureVerdictV1::Pass
            } else {
                Ms3IndependentFutureVerdictV1::Contradiction
            }
        );
        runtimes
            .frozen
            .seal_independent_future(future)
            .expect("terminal future");
        (topology_archive, lifecycle, runtimes)
    }

    fn runtime_parity(frame: &RelationFrame) -> RuntimeParityCase {
        RuntimeParityCase {
            evidence_ref_sha256: frame.frame_id_sha256.clone(),
            capture_receipt: None,
            request_text: "Return opaque".to_owned(),
            provider_payload: runtime_payload(),
            expected_response: serde_json::to_string(&json!({
                "name": "transport_a",
                "arguments": {"value": 7}
            }))
            .expect("expected response"),
        }
    }

    fn negative_topology(sequence: u64, captured_at_unix_ms: u64) -> PreActionTopologyAuditRowV1 {
        let mut row = topology(
            "negative",
            "request-negative",
            "negative-lineage",
            sequence,
            captured_at_unix_ms,
        );
        row.structure.request_phase_atom_ids = vec![7];
        row.structure.pre_action_context_atom_ids = vec![8];
        row.structure.topology.role_witnesses.clear();
        row.commit = PreActionTopologyCommitV1::seal(
            &row.structure,
            MultiSourceEvidenceOriginV1::FreshLive,
            hash("extractor"),
            hash("config"),
            sequence,
        )
        .expect("negative topology commit");
        row
    }

    #[test]
    fn ms4_external_candidate_rebuilds_authority_without_trusting_candidate_flags() {
        let root = test_root("ms4-external-admission");
        let (topology_archive, _lifecycle, runtimes) = terminal_generation(&root, Some(true));
        let frozen = runtimes.frozen.envelope().cloned().expect("frozen law");
        let future = runtimes
            .frozen
            .independent_future()
            .cloned()
            .expect("future pass");
        let support_topology = topology_archive
            .row_by_root(&frozen.contract.topology_root_sha256)
            .expect("support topology");
        let future_topology = topology_archive
            .row_by_root(&future.receipt.topology_root_sha256)
            .expect("future topology");
        let support_ms = support_topology.captured_at_unix_ms.expect("support time");
        let future_ms = future_topology.captured_at_unix_ms.expect("future time");
        let support_frame = completed_frame(
            "support",
            "action-support",
            "support-lineage",
            support_ms.saturating_add(500),
            true,
        );
        let future_frame = completed_frame(
            "future",
            "action-future",
            "future-lineage",
            future_ms.saturating_add(10_000),
            true,
        );
        assert_eq!(
            canonical_json_sha256(&support_frame).expect("support root"),
            frozen.contract.frame_root_sha256
        );
        assert_eq!(
            canonical_json_sha256(&future_frame).expect("future root"),
            future.receipt.completed_frame_root_sha256
        );
        let support_terminal = terminal(
            "request-support",
            support_ms.saturating_sub(10),
            support_ms.saturating_add(100),
        );
        let future_terminal = terminal(
            "request-future",
            future_ms.saturating_sub(1),
            future_ms.saturating_add(11_000),
        );
        let negative = negative_topology(
            frozen.contract.future_min_sequence.saturating_add(10),
            future_ms.saturating_add(20_000),
        );
        let candidate = Ms4ExternalAdmissionCandidateV1::seal(
            frozen.clone(),
            future.clone(),
            support_topology.clone(),
            support_frame.clone(),
            support_terminal.clone(),
            runtime_parity(&support_frame),
            future_topology.clone(),
            future_frame.clone(),
            future_terminal.clone(),
            runtime_parity(&future_frame),
            vec![negative.clone()],
        )
        .expect("external candidate");
        let bytes = candidate.canonical_bytes().expect("candidate bytes");
        assert_eq!(
            Ms4ExternalAdmissionCandidateV1::from_canonical_bytes(&bytes)
                .expect("candidate restart"),
            candidate
        );
        let package = candidate.admitted_package().expect("admitted package");
        assert!(package.proof.wave_causal_pass);
        assert!(!package.phase_centers.is_empty());
        assert!(!package.anti_centers.is_empty());
        assert_eq!(package.admission_candidate_blocker(), None);

        let gate_root = hash("gate");
        let runtime_root = hash("runtime");
        let snapshot = build_ms4_external_admission_snapshot(
            std::slice::from_ref(&candidate),
            "nando-wave",
            1,
            100,
            30,
            &gate_root,
            &runtime_root,
        )
        .expect("external rebuild")
        .expect("admission snapshot");
        let executor = ResponseExecutor::from_registry_with_admission(
            snapshot.registry,
            snapshot.admission,
            "nando-wave",
            &gate_root,
            &runtime_root,
            100,
            30,
        )
        .expect("independent executor");
        let execution = executor.execute("Return opaque", &runtime_payload());
        assert_eq!(execution.status, ResponseExecutionStatus::Executed);
        assert_eq!(
            execution.response.as_deref(),
            Some(runtime_parity(&support_frame).expected_response.as_str())
        );

        assert_eq!(
            Ms4ExternalAdmissionCandidateV1::seal(
                frozen.clone(),
                future.clone(),
                support_topology.clone(),
                support_frame.clone(),
                support_terminal.clone(),
                runtime_parity(&support_frame),
                future_topology.clone(),
                future_frame.clone(),
                future_terminal.clone(),
                runtime_parity(&future_frame),
                Vec::new(),
            )
            .expect_err("negative proof is mandatory"),
            "ms4_external_negative_control_missing"
        );
        let mut rebound_parity = runtime_parity(&support_frame);
        rebound_parity.evidence_ref_sha256 = hash("wrong frame");
        assert_eq!(
            Ms4ExternalAdmissionCandidateV1::seal(
                frozen,
                future,
                support_topology,
                support_frame,
                support_terminal,
                rebound_parity,
                future_topology,
                future_frame.clone(),
                future_terminal,
                runtime_parity(&future_frame),
                vec![negative],
            )
            .expect_err("runtime parity cannot rebound"),
            "ms4_external_runtime_parity_binding_invalid"
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn first_generation_uses_legacy_paths_and_manifest_is_canonical() {
        let root = test_root("first");
        let topology_root = root.join("topologies");
        let topology_archive =
            MultiSourceTopologyArchive::open(&topology_root).expect("topology archive");
        let (lifecycle, runtimes) =
            Ms3GenerationLifecycleRuntime::open(&root, &topology_archive, 100, 4, 60)
                .expect("lifecycle");
        assert_eq!(lifecycle.manifest().active_generation_sequence, 1);
        assert_eq!(runtimes.acquisition.generation_sequence(), 1);
        assert_eq!(runtimes.frozen.generation_sequence(), 1);
        assert!(!lifecycle.manifest().authority_ready);
        assert!(!lifecycle.manifest().phase_mutation_allowed);
        let bytes = std::fs::read(root.join(LIFECYCLE_FILE)).expect("manifest bytes");
        assert_eq!(
            Ms3GenerationLifecycleManifestV1::from_canonical_bytes(&bytes)
                .expect("manifest restore"),
            lifecycle.manifest().clone()
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn missing_manifest_cannot_reinfer_a_started_successor() {
        let root = test_root("missing");
        std::fs::create_dir_all(
            root.join(GENERATIONS_DIRECTORY)
                .join("00000000000000000002"),
        )
        .expect("generation directory");
        let topology_root = root.join("topologies");
        let topology_archive =
            MultiSourceTopologyArchive::open(&topology_root).expect("topology archive");
        assert_eq!(
            Ms3GenerationLifecycleRuntime::open(&root, &topology_archive, 100, 4, 60)
                .err()
                .expect("fail closed"),
            "ms3_lifecycle_manifest_missing"
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn contradiction_rolls_to_fresh_generation_and_restarts_without_evidence_loss() {
        let root = test_root("rollover");
        let (topology_archive, mut lifecycle, generation_one) =
            terminal_generation(&root, Some(false));
        let legacy_contract =
            std::fs::read(root.join("contract-v1.cbor")).expect("legacy acquisition contract");
        let legacy_report =
            std::fs::read(root.join("terminal-report-v1.cbor")).expect("legacy terminal report");
        let legacy_envelope = std::fs::read(
            root.join(LEGACY_VERSION_SPACE_DIRECTORY)
                .join("frozen-version-space-v1.cbor"),
        )
        .expect("legacy frozen envelope");
        let legacy_future = std::fs::read(
            root.join(LEGACY_VERSION_SPACE_DIRECTORY)
                .join("independent-future-v1.cbor"),
        )
        .expect("legacy future");

        let generation_two = lifecycle
            .prepare_successor(
                &topology_archive,
                200,
                &generation_one.acquisition,
                &generation_one.frozen,
            )
            .expect("successor");
        assert_eq!(lifecycle.manifest().active_generation_sequence, 2);
        assert_eq!(generation_two.acquisition.generation_sequence(), 2);
        assert_eq!(generation_two.frozen.generation_sequence(), 2);
        assert_eq!(
            generation_two
                .acquisition
                .contract()
                .topology_watermark_rows,
            2
        );
        assert!(generation_two.frozen.envelope().is_none());
        assert!(generation_two.frozen.independent_future().is_none());
        assert!(generation_two.frozen.predictions().is_empty());
        assert!(!lifecycle.manifest().authority_ready);
        assert!(!lifecycle.manifest().phase_mutation_allowed);
        assert_eq!(
            std::fs::read(root.join("contract-v1.cbor")).expect("legacy acquisition contract"),
            legacy_contract
        );
        assert_eq!(
            std::fs::read(root.join("terminal-report-v1.cbor")).expect("legacy terminal report"),
            legacy_report
        );
        assert_eq!(
            std::fs::read(
                root.join(LEGACY_VERSION_SPACE_DIRECTORY)
                    .join("frozen-version-space-v1.cbor")
            )
            .expect("legacy frozen envelope"),
            legacy_envelope
        );
        assert_eq!(
            std::fs::read(
                root.join(LEGACY_VERSION_SPACE_DIRECTORY)
                    .join("independent-future-v1.cbor")
            )
            .expect("legacy future"),
            legacy_future
        );

        let (restored, restored_generation) =
            Ms3GenerationLifecycleRuntime::open(&root, &topology_archive, 300, 256, 86_400)
                .expect("restart");
        assert_eq!(restored.manifest(), lifecycle.manifest());
        assert_eq!(restored_generation.acquisition.generation_sequence(), 2);
        assert_eq!(
            restored_generation.acquisition.contract(),
            generation_two.acquisition.contract()
        );
        assert!(restored_generation.frozen.envelope().is_none());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn reused_linked_lineage_closes_immutably_and_opens_fresh_successor() {
        let root = test_root("linked-evidence-reuse");
        let (mut topology_archive, mut lifecycle, generation_one) =
            terminal_generation(&root, Some(false));
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock");
        let mut generation_two = lifecycle
            .prepare_successor(
                &topology_archive,
                now.as_secs(),
                &generation_one.acquisition,
                &generation_one.frozen,
            )
            .expect("generation two");
        let now_ms = u64::try_from(now.as_millis()).expect("milliseconds");
        let reused_topology = topology(
            "reused-support",
            "request-reused-support",
            "support-lineage",
            3,
            now_ms,
        );
        let reused_frame = completed_frame(
            "reused-support",
            "action-reused-support",
            "support-lineage",
            now_ms.saturating_add(500),
            true,
        );
        let reused_terminal = terminal(
            "request-reused-support",
            now_ms.saturating_sub(10),
            now_ms.saturating_add(100),
        );
        topology_archive
            .append(&reused_topology)
            .expect("reused topology remains in denominator");
        let report = generation_two
            .acquisition
            .evaluate(
                now.as_secs(),
                vec![reused_topology],
                vec![reused_frame],
                vec![reused_terminal],
            )
            .expect("legacy terminal reused report");
        assert!(report.is_terminal());
        assert!(report.receipts.iter().all(|receipt| {
            generation_two
                .frozen
                .generation_registry()
                .linked_evidence_was_used(receipt)
        }));
        let terminal_report_path = generation_paths(&root, 2).0.join("terminal-report-v1.cbor");
        let terminal_report_bytes =
            std::fs::read(&terminal_report_path).expect("terminal report bytes");
        let closure = generation_two
            .frozen
            .seal_linked_evidence_reuse(&report, topology_archive.max_bridge_sequence())
            .expect("durable evidence reuse closure");
        assert_eq!(
            closure.blocker,
            nando_operator_learning::multi_source::MS3_LINKED_EVIDENCE_REUSE
        );
        assert!(!closure.authority_ready);
        assert!(!closure.phase_mutation_allowed);

        let generation_three = lifecycle
            .prepare_successor(
                &topology_archive,
                now.as_secs().saturating_add(1),
                &generation_two.acquisition,
                &generation_two.frozen,
            )
            .expect("generation three");
        assert_eq!(generation_three.acquisition.generation_sequence(), 3);
        assert_eq!(
            generation_three
                .acquisition
                .contract()
                .topology_watermark_rows,
            3
        );
        assert_eq!(
            std::fs::read(&terminal_report_path).expect("terminal report after successor"),
            terminal_report_bytes
        );
        assert!(!lifecycle.manifest().authority_ready);
        assert!(!lifecycle.manifest().phase_mutation_allowed);

        let (restored, restored_generation) = Ms3GenerationLifecycleRuntime::open(
            &root,
            &topology_archive,
            now.as_secs().saturating_add(2),
            256,
            86_400,
        )
        .expect("restart");
        assert_eq!(restored.manifest(), lifecycle.manifest());
        assert_eq!(restored_generation.acquisition.generation_sequence(), 3);
        assert_eq!(
            restored_generation
                .frozen
                .generation_registry()
                .linked_acquisition_failure(2),
            Some(&closure)
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn prepared_successor_is_idempotent_when_manifest_publish_did_not_happen() {
        let root = test_root("retry");
        let (topology_archive, mut lifecycle, generation_one) =
            terminal_generation(&root, Some(false));
        let generation_one_manifest =
            std::fs::read(root.join(LIFECYCLE_FILE)).expect("generation one manifest");
        let first_successor = lifecycle
            .prepare_successor(
                &topology_archive,
                200,
                &generation_one.acquisition,
                &generation_one.frozen,
            )
            .expect("first successor");
        let first_contract = first_successor.acquisition.contract().clone();
        let first_manifest = lifecycle.manifest().clone();

        write_atomic(&root.join(LIFECYCLE_FILE), &generation_one_manifest)
            .expect("restore old active pointer");
        let (mut restored_old, restored_generation_one) =
            Ms3GenerationLifecycleRuntime::open(&root, &topology_archive, 300, 256, 86_400)
                .expect("restore old generation");
        assert_eq!(restored_old.manifest().active_generation_sequence, 1);
        let retried = restored_old
            .prepare_successor(
                &topology_archive,
                400,
                &restored_generation_one.acquisition,
                &restored_generation_one.frozen,
            )
            .expect("retry successor");
        assert_eq!(retried.acquisition.contract(), &first_contract);
        assert_eq!(restored_old.manifest(), &first_manifest);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn exhausted_applicability_gate_rolls_to_fresh_generation_and_restarts() {
        let root = test_root("applicability-failure-rollover");
        let (topology_archive, mut lifecycle, mut generation_one) =
            terminal_generation(&root, None);
        let failure = generation_one
            .frozen
            .seal_applicability_acquisition_failure(u64::MAX)
            .expect("durable acquisition failure");
        assert_eq!(failure.generation_sequence, 1);
        assert_eq!(failure.independent_topologies, 0);
        assert!(!failure.authority_ready);
        assert!(!failure.phase_mutation_allowed);
        assert_eq!(
            generation_one
                .frozen
                .seal_applicability_acquisition_failure(u64::MAX)
                .expect("idempotent acquisition failure"),
            failure
        );

        let generation_two = lifecycle
            .prepare_successor(
                &topology_archive,
                200,
                &generation_one.acquisition,
                &generation_one.frozen,
            )
            .expect("fresh successor");
        assert_eq!(lifecycle.manifest().active_generation_sequence, 2);
        assert_eq!(
            lifecycle.manifest().predecessor_terminal_root_sha256,
            Some(failure.receipt_root_sha256)
        );
        assert_eq!(generation_two.acquisition.generation_sequence(), 2);
        assert!(generation_two.frozen.envelope().is_none());

        let (restored, restored_generation) =
            Ms3GenerationLifecycleRuntime::open(&root, &topology_archive, 300, 256, 86_400)
                .expect("restart");
        assert_eq!(restored.manifest(), lifecycle.manifest());
        assert_eq!(restored_generation.acquisition.generation_sequence(), 2);
        assert!(restored_generation.frozen.envelope().is_none());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn exhausted_linked_acquisition_rolls_forward_and_preserves_generation_sequence() {
        let root = test_root("linked-acquisition-failure-rollover");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock");
        let opened_at_unix = now.as_secs();
        let opened_at_ms = u64::try_from(now.as_millis()).expect("milliseconds");
        let topology_root = root.join("topologies");
        let mut topology_archive =
            MultiSourceTopologyArchive::open(&topology_root).expect("topology archive");
        let (mut lifecycle, mut generation_one) = Ms3GenerationLifecycleRuntime::open(
            &root,
            &topology_archive,
            opened_at_unix,
            1,
            86_400,
        )
        .expect("generation one");
        let unsettled = topology(
            "unsettled",
            "request-unsettled",
            "unsettled-lineage",
            1,
            opened_at_ms,
        );
        topology_archive
            .append(&unsettled)
            .expect("unsettled topology");
        let unlinked = topology(
            "unlinked",
            "request-unlinked",
            "unlinked-lineage",
            2,
            opened_at_ms.saturating_add(100),
        );
        topology_archive
            .append(&unlinked)
            .expect("unlinked topology");
        let settled_frame = completed_frame(
            "unlinked",
            "action-unlinked",
            "unlinked-lineage",
            opened_at_ms.saturating_add(120),
            true,
        );
        let settled_frame_root =
            nando_operator_kernel::canonical_json_sha256(&settled_frame).expect("frame root");
        let failed = generation_one
            .acquisition
            .evaluate_with_route_bound_evidence(
                opened_at_unix,
                vec![unsettled, unlinked],
                vec![settled_frame],
                vec![terminal(
                    "request-unlinked",
                    opened_at_ms.saturating_add(90),
                    opened_at_ms.saturating_add(110),
                )],
                &BTreeSet::from([settled_frame_root.clone()]),
                &BTreeSet::from([settled_frame_root]),
            )
            .expect("terminal acquisition failure");
        assert_eq!(
            failed.verdict,
            nando_operator_learning::multi_source::Ms3LinkedFrameAcquisitionVerdictV1::AcquisitionFail
        );
        let failure = generation_one
            .frozen
            .seal_linked_acquisition_failure(&failed, topology_archive.max_bridge_sequence())
            .expect("durable linked acquisition failure");
        assert_eq!(failure.generation_sequence, 1);
        assert_eq!(failed.candidate_topology_rows, 2);
        assert_eq!(failed.eligible_topology_rows, 1);
        assert_eq!(failed.route_settlement_pending_rows, 1);
        assert!(!failure.authority_ready);
        assert!(!failure.phase_mutation_allowed);
        assert!(
            generation_one
                .frozen
                .seal_linked_acquisition_failure(
                    &failed,
                    topology_archive.max_bridge_sequence().saturating_add(100),
                )
                .is_err(),
            "closure cannot move after the denominator was frozen"
        );

        let support = topology(
            "support-two",
            "request-support-two",
            "support-two-lineage",
            3,
            opened_at_ms.saturating_add(1_000),
        );
        topology_archive
            .append(&support)
            .expect("overflow support topology");
        let mut generation_two = lifecycle
            .prepare_successor(
                &topology_archive,
                opened_at_unix.saturating_add(1),
                &generation_one.acquisition,
                &generation_one.frozen,
            )
            .expect("generation two");
        assert_eq!(lifecycle.manifest().active_generation_sequence, 2);
        assert_eq!(
            lifecycle.manifest().predecessor_terminal_root_sha256,
            Some(failure.receipt_root_sha256.clone())
        );
        assert_eq!(generation_two.acquisition.generation_sequence(), 2);
        assert_eq!(
            generation_two
                .acquisition
                .contract()
                .topology_watermark_rows,
            2,
            "successor starts at the consumed cursor, not the archive tail"
        );
        assert!(generation_two.frozen.envelope().is_none());

        let frame = completed_frame(
            "support-two",
            "action-support-two",
            "support-two-lineage",
            opened_at_ms.saturating_add(1_500),
            true,
        );
        let support_terminal = terminal(
            "request-support-two",
            opened_at_ms.saturating_add(900),
            opened_at_ms.saturating_add(1_100),
        );
        let report = generation_two
            .acquisition
            .evaluate(
                opened_at_unix.saturating_add(1),
                vec![support.clone()],
                vec![frame.clone()],
                vec![support_terminal.clone()],
            )
            .expect("linked acquisition");
        let ledger = TransportBindingLedgerV1::build(
            std::slice::from_ref(&support),
            std::slice::from_ref(&frame),
            std::slice::from_ref(&support_terminal),
        );
        let bound = &ledger.bound_for_topology(&support.commit.commitment_root_sha256)[0];
        let prepared = prepare_ms3_frozen_version_space_v1(&report, bound, &frame)
            .expect("prepared version space");
        generation_two
            .frozen
            .freeze(
                prepared,
                topology_archive.max_bridge_sequence(),
                Ms3VersionSpaceVersionsV1 {
                    compiler_version: "test-compiler.v1".to_owned(),
                    vm_abi: "test-vm.v1".to_owned(),
                },
                opened_at_unix.saturating_add(1),
            )
            .expect("generation two freeze");
        assert_eq!(
            generation_two
                .frozen
                .generation_registry()
                .generations
                .last()
                .expect("frozen generation")
                .generation_sequence,
            2
        );
        assert_eq!(
            generation_two
                .frozen
                .generation_registry()
                .linked_acquisition_failure(1),
            Some(&failure)
        );

        let (restored, restored_generation) = Ms3GenerationLifecycleRuntime::open(
            &root,
            &topology_archive,
            opened_at_unix.saturating_add(2),
            256,
            86_400,
        )
        .expect("restart");
        assert_eq!(restored.manifest(), lifecycle.manifest());
        assert_eq!(restored_generation.frozen.generation_sequence(), 2);
        assert!(restored_generation.frozen.envelope().is_some());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn capture_gap_repair_closes_generation_before_fresh_successor() {
        let root = test_root("capture-gap-repair-rollover");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock");
        let opened_at_unix = now.as_secs();
        let opened_at_ms = u64::try_from(now.as_millis()).expect("milliseconds");
        let topology_root = root.join("topologies");
        let mut topology_archive =
            MultiSourceTopologyArchive::open(&topology_root).expect("topology archive");
        let (mut lifecycle, mut generation_one) = Ms3GenerationLifecycleRuntime::open(
            &root,
            &topology_archive,
            opened_at_unix,
            256,
            86_400,
        )
        .expect("generation one");
        let mut support = topology(
            "capture-gap",
            "request-capture-gap",
            "capture-gap-lineage",
            1,
            opened_at_ms,
        );
        support.structure.topology.role_witnesses[0].value_sha256 =
            hash("pre-action-role-omitted-by-old-extractor");
        support.commit = PreActionTopologyCommitV1::seal(
            &support.structure,
            MultiSourceEvidenceOriginV1::FreshLive,
            hash("old-extractor"),
            hash("config"),
            1,
        )
        .expect("re-sealed topology commit");
        topology_archive.append(&support).expect("support topology");
        let frame = completed_frame(
            "capture-gap",
            "action-capture-gap",
            "capture-gap-lineage",
            opened_at_ms.saturating_add(500),
            true,
        );
        let support_terminal = terminal(
            "request-capture-gap",
            opened_at_ms,
            opened_at_ms.saturating_add(250),
        );
        let report = generation_one
            .acquisition
            .evaluate(
                opened_at_unix,
                vec![support],
                vec![frame],
                vec![support_terminal],
            )
            .expect("linked capture gap");
        assert!(report.validate(), "capture-gap report remains canonical");
        assert_eq!(
            report.verdict,
            nando_operator_learning::multi_source::Ms3LinkedFrameAcquisitionVerdictV1::LinkedFrameObserved
        );
        assert!(report.receipts.iter().all(|receipt| {
            receipt.gap_class
                == Some(
                    nando_operator_learning::multi_source::RepresentationGapClassV1::CaptureGapA,
                )
        }));

        let closure = generation_one
            .frozen
            .seal_linked_capture_gap_repair(&report, topology_archive.max_bridge_sequence())
            .expect("durable capture-gap closure");
        assert_eq!(
            closure.blocker,
            nando_operator_learning::multi_source::MS3_CAPTURE_GAP_REPAIR_REQUIRED
        );
        let generation_two = lifecycle
            .prepare_successor(
                &topology_archive,
                opened_at_unix.saturating_add(1),
                &generation_one.acquisition,
                &generation_one.frozen,
            )
            .expect("fresh successor");
        assert_eq!(generation_two.acquisition.generation_sequence(), 2);
        assert_eq!(
            generation_two
                .acquisition
                .contract()
                .topology_watermark_rows,
            1
        );
        assert!(generation_two.frozen.envelope().is_none());
        assert!(!lifecycle.manifest().authority_ready);
        assert!(!lifecycle.manifest().phase_mutation_allowed);

        let (restored, restored_generation) = Ms3GenerationLifecycleRuntime::open(
            &root,
            &topology_archive,
            opened_at_unix.saturating_add(2),
            256,
            86_400,
        )
        .expect("restart");
        assert_eq!(restored.manifest(), lifecycle.manifest());
        assert_eq!(restored_generation.acquisition.generation_sequence(), 2);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn future_pass_does_not_create_a_successor() {
        let root = test_root("pass");
        let (topology_archive, mut lifecycle, generation_one) =
            terminal_generation(&root, Some(true));
        let result = lifecycle.prepare_successor(
            &topology_archive,
            200,
            &generation_one.acquisition,
            &generation_one.frozen,
        );
        assert!(matches!(
            result,
            Err(ref error) if error == "ms3_lifecycle_successor_requires_terminal_failure"
        ));
        assert_eq!(lifecycle.manifest().active_generation_sequence, 1);
        assert!(!root.join(GENERATIONS_DIRECTORY).exists());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires NANDO_MS3_TERMINAL_V1_FIXTURE pointing to a disposable live-state copy"]
    fn terminal_v1_fixture_uses_its_bound_report_cursor_without_reordering_archive() {
        let fixture = PathBuf::from(
            std::env::var("NANDO_MS3_TERMINAL_V1_FIXTURE").expect("fixture directory"),
        );
        let topology_archive =
            MultiSourceTopologyArchive::open(&fixture.join("pre-action-topology-archive-v1"))
                .expect("topology archive");
        let learning_root = fixture.join("linked-frame-acquisition-v1");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_secs();
        let (mut lifecycle, generation) = Ms3GenerationLifecycleRuntime::open(
            &learning_root,
            &topology_archive,
            now,
            256,
            86_400,
        )
        .expect("terminal lifecycle");
        let active_generation = lifecycle.manifest().active_generation_sequence;
        let failure = generation
            .frozen
            .generation_registry()
            .linked_acquisition_failure(active_generation)
            .expect("linked acquisition failure");
        assert_eq!(
            failure.consumed_topology_cursor_rows, 0,
            "fixture must exercise the persisted V1 receipt"
        );
        let expected_cursor = generation
            .acquisition
            .consumed_topology_cursor_rows()
            .expect("bound terminal report cursor");
        let successor = lifecycle
            .prepare_successor(
                &topology_archive,
                now.saturating_add(1),
                &generation.acquisition,
                &generation.frozen,
            )
            .expect("successor from V1 receipt");
        assert_eq!(
            successor.acquisition.generation_sequence(),
            active_generation.saturating_add(1)
        );
        assert_eq!(
            successor.acquisition.contract().topology_watermark_rows,
            expected_cursor
        );
        assert!(!successor.frozen.generation_registry().authority_ready);
        assert!(
            !successor
                .frozen
                .generation_registry()
                .phase_mutation_allowed
        );
    }

    #[test]
    #[ignore = "requires NANDO_MS3_G16_FIXTURE pointing to a disposable live-state copy"]
    fn frozen_g16_fixture_censors_unattributed_rows_without_losing_overflow() {
        let fixture =
            PathBuf::from(std::env::var("NANDO_MS3_G16_FIXTURE").expect("fixture directory"));
        let topology_root = fixture.join("pre-action-topology-archive-v1");
        let topology_archive =
            MultiSourceTopologyArchive::open(&topology_root).expect("topology archive");
        let learning_root = fixture.join("linked-frame-acquisition-v1");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_secs();
        let (mut lifecycle, mut generation) = Ms3GenerationLifecycleRuntime::open(
            &learning_root,
            &topology_archive,
            now,
            256,
            86_400,
        )
        .expect("G16 lifecycle");
        assert_eq!(lifecycle.manifest().active_generation_sequence, 16);
        let watermark = generation.acquisition.contract().topology_watermark_rows;
        let new_topologies = topology_archive
            .rows_after(usize::try_from(watermark).expect("watermark range"))
            .expect("G16 topology tail");
        let request_ids = new_topologies
            .iter()
            .map(|row| row.structure.request_event_id_sha256.clone())
            .collect::<BTreeSet<_>>();
        let intent_ids = new_topologies
            .iter()
            .map(|row| row.structure.turn_intent_id_sha256.clone())
            .collect::<BTreeSet<_>>();
        let frame_archive = crate::multi_source_frame_archive::MultiSourceFrameArchive::open(
            &fixture.join("relation-frame-archive-v1"),
        )
        .expect("frame archive");
        let terminal_archive = crate::terminal_receipt_archive::TerminalReceiptArchive::open(
            &fixture.join("terminal-receipt-archive-v1"),
        )
        .expect("terminal archive");
        let frames = frame_archive.frames_for_intents(&intent_ids);
        let terminals = terminal_archive.receipts_for_requests(&request_ids);
        let used_evidence_roots = generation
            .frozen
            .generation_registry()
            .used_evidence_roots();
        let report = generation
            .acquisition
            .evaluate_excluding_used_evidence(
                now,
                new_topologies,
                frames,
                terminals,
                &used_evidence_roots,
            )
            .expect("G16 terminal report");
        eprintln!("G16 replay report: {report:#?}");

        assert_eq!(report.raw_scanned_topology_rows, 256);
        assert_eq!(report.eligible_topology_rows, 155);
        assert_eq!(report.terminal_receipt_rows, 155);
        assert_eq!(report.censored_unattributed_rows, 12);
        assert_eq!(report.censored_topology_rows, 89);
        assert_eq!(report.linked_frame_rows, 0);
        assert_eq!(
            report.verdict,
            nando_operator_learning::multi_source::Ms3LinkedFrameAcquisitionVerdictV1::CensoredIneligibleProbe
        );
        assert!(!report.authority_ready);
        assert!(!report.phase_update_allowed);

        let consumed_cursor = report.consumed_topology_cursor_rows;
        let consumed_sequence = report.consumed_capture_sequence;
        let overflow_rows = u64::try_from(topology_archive.len())
            .expect("archive rows")
            .saturating_sub(consumed_cursor);
        assert!(overflow_rows > 0, "fixture must contain post-G16 overflow");
        generation
            .frozen
            .seal_ineligible_probe_censor(&report, consumed_sequence)
            .expect("durable G16 censor");
        let generation_17 = lifecycle
            .prepare_successor(
                &topology_archive,
                now.saturating_add(1),
                &generation.acquisition,
                &generation.frozen,
            )
            .expect("G17 successor");
        assert_eq!(generation_17.acquisition.generation_sequence(), 17);
        assert_eq!(
            generation_17.acquisition.contract().topology_watermark_rows,
            consumed_cursor
        );
        assert_eq!(
            u64::try_from(topology_archive.len())
                .expect("archive rows")
                .saturating_sub(generation_17.acquisition.contract().topology_watermark_rows),
            overflow_rows
        );

        let (restored, restored_generation) = Ms3GenerationLifecycleRuntime::open(
            &learning_root,
            &topology_archive,
            now.saturating_add(2),
            256,
            86_400,
        )
        .expect("restart after G16 rollover");
        assert_eq!(restored.manifest(), lifecycle.manifest());
        assert_eq!(restored_generation.acquisition.generation_sequence(), 17);
        assert!(
            !restored_generation
                .frozen
                .generation_registry()
                .authority_ready
        );
        assert!(
            !restored_generation
                .frozen
                .generation_registry()
                .phase_mutation_allowed
        );
    }

    #[test]
    #[ignore = "requires NANDO_MS3_G25_FIXTURE pointing to a disposable live-state copy"]
    fn frozen_g25_fixture_closes_pre_route_receipt_epoch_into_v3() {
        let fixture =
            PathBuf::from(std::env::var("NANDO_MS3_G25_FIXTURE").expect("fixture directory"));
        let topology_root = fixture.join("pre-action-topology-archive-v1");
        let topology_archive =
            MultiSourceTopologyArchive::open(&topology_root).expect("topology archive");
        let learning_root = fixture.join("linked-frame-acquisition-v1");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_secs();
        let (mut lifecycle, mut generation) = Ms3GenerationLifecycleRuntime::open(
            &learning_root,
            &topology_archive,
            now,
            256,
            86_400,
        )
        .expect("G25 lifecycle");
        assert_eq!(lifecycle.manifest().active_generation_sequence, 25);
        assert_eq!(
            generation.acquisition.contract().schema,
            nando_operator_learning::multi_source::MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V2
        );

        let watermark = generation.acquisition.contract().topology_watermark_rows;
        let new_topologies = topology_archive
            .rows_after(usize::try_from(watermark).expect("watermark range"))
            .expect("G25 topology tail");
        let request_ids = new_topologies
            .iter()
            .map(|row| row.structure.request_event_id_sha256.clone())
            .collect::<BTreeSet<_>>();
        let intent_ids = new_topologies
            .iter()
            .map(|row| row.structure.turn_intent_id_sha256.clone())
            .collect::<BTreeSet<_>>();
        let frame_archive = crate::multi_source_frame_archive::MultiSourceFrameArchive::open(
            &fixture.join("relation-frame-archive-v1"),
        )
        .expect("frame archive");
        let terminal_archive = crate::terminal_receipt_archive::TerminalReceiptArchive::open(
            &fixture.join("terminal-receipt-archive-v1"),
        )
        .expect("terminal archive");
        let frames = frame_archive.frames_for_intents(&intent_ids);
        let terminals = terminal_archive.receipts_for_requests(&request_ids);
        let used_evidence_roots = generation
            .frozen
            .generation_registry()
            .used_evidence_roots();
        let report = generation
            .acquisition
            .evaluate_excluding_used_evidence(
                now,
                new_topologies,
                frames,
                terminals,
                &used_evidence_roots,
            )
            .expect("G25 epoch closure report");

        assert_eq!(report.raw_scanned_topology_rows, 264);
        assert_eq!(report.eligible_topology_rows, 66);
        assert_eq!(report.terminal_receipt_rows, 66);
        assert_eq!(report.censored_topology_rows, 198);
        assert_eq!(
            report
                .ineligible_reason_counts
                .get(
                    &nando_operator_learning::multi_source::MultiSourceJoinCensoredReasonV1::TerminalReceiptUnavailable
                ),
            Some(&180)
        );
        assert_eq!(
            report.ineligible_reason_counts.get(
                &nando_operator_learning::multi_source::MultiSourceJoinCensoredReasonV1::TopologyCensored
            ),
            Some(&18)
        );
        assert_eq!(
            report
                .ineligible_reason_counts
                .values()
                .copied()
                .sum::<u64>(),
            198
        );
        assert_eq!(report.linked_frame_rows, 0);
        assert_eq!(
            report.verdict,
            nando_operator_learning::multi_source::Ms3LinkedFrameAcquisitionVerdictV1::CensoredPreRouteReceiptEpoch
        );
        assert_eq!(
            report.blocker,
            nando_operator_learning::multi_source::MS3_CENSORED_PRE_ROUTE_RECEIPT_EPOCH
        );
        assert!(!report.authority_ready);
        assert!(!report.phase_update_allowed);

        let consumed_cursor = report.consumed_topology_cursor_rows;
        let consumed_sequence = report.consumed_capture_sequence;
        let closure_receipt = generation
            .frozen
            .seal_ineligible_probe_censor(&report, consumed_sequence)
            .expect("durable G25 epoch censor");
        assert_eq!(closure_receipt.censored_pre_route_receipt_rows, 180);
        assert_eq!(closure_receipt.censored_topology_rows, 198);
        let generation_26 = lifecycle
            .prepare_successor(
                &topology_archive,
                now.saturating_add(1),
                &generation.acquisition,
                &generation.frozen,
            )
            .expect("G26 successor");
        assert_eq!(generation_26.acquisition.generation_sequence(), 26);
        assert_eq!(
            generation_26.acquisition.contract().schema,
            nando_operator_learning::multi_source::MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V3
        );
        assert_eq!(
            generation_26.acquisition.contract().topology_watermark_rows,
            consumed_cursor
        );
        assert!(!generation_26.frozen.generation_registry().authority_ready);
        assert!(
            !generation_26
                .frozen
                .generation_registry()
                .phase_mutation_allowed
        );

        let (restored, restored_generation) = Ms3GenerationLifecycleRuntime::open(
            &learning_root,
            &topology_archive,
            now.saturating_add(2),
            256,
            86_400,
        )
        .expect("restart after G25 rollover");
        assert_eq!(restored.manifest(), lifecycle.manifest());
        assert_eq!(restored_generation.acquisition.generation_sequence(), 26);
        assert_eq!(
            restored_generation.acquisition.contract(),
            generation_26.acquisition.contract()
        );
    }
}
