//! Durable active-generation pointer for autonomous MS3 contradiction rollover.

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
        let future = current_frozen
            .independent_future()
            .ok_or_else(|| "ms3_lifecycle_terminal_future_missing".to_owned())?;
        if future.receipt.verdict != Ms3IndependentFutureVerdictV1::Contradiction {
            return Err("ms3_lifecycle_successor_requires_contradiction".to_owned());
        }
        let terminal = current_frozen
            .generation_registry()
            .generations
            .last()
            .filter(|entry| entry.generation_sequence == self.manifest.active_generation_sequence)
            .and_then(|entry| entry.terminal.as_ref())
            .ok_or_else(|| "ms3_lifecycle_terminal_registry_missing".to_owned())?;
        if terminal.verdict != Ms3IndependentFutureVerdictV1::Contradiction
            || terminal.future_receipt_root_sha256 != future.receipt.receipt_root_sha256
        {
            return Err("ms3_lifecycle_terminal_registry_mismatch".to_owned());
        }
        let next_generation_sequence = self
            .manifest
            .active_generation_sequence
            .checked_add(1)
            .ok_or_else(|| "ms3_lifecycle_generation_overflow".to_owned())?;
        let (acquisition_path, version_space_path) =
            generation_paths(&self.root, next_generation_sequence);
        let acquisition = Ms3LinkedFrameAcquisitionRuntime::open_generation(
            &acquisition_path,
            next_generation_sequence,
            topology_archive,
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
            Some(terminal.terminal_root_sha256.clone()),
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
        .generations
        .iter()
        .find(|entry| entry.generation_sequence.saturating_add(1) == generation_sequence)
        .and_then(|entry| entry.terminal.as_ref())
        .filter(|terminal| terminal.verdict == Ms3IndependentFutureVerdictV1::Contradiction)
        .map(|terminal| Some(terminal.terminal_root_sha256.clone()))
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
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use nando_operator_kernel::{
        AtomSource, AtomValueType, LEARNING_REQUEST_STRUCTURE_SCHEMA_V2,
        MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceEvidenceOriginV1,
        MultiSourceExtractionStatusV1, MultiSourceRelationEdgeV1, MultiSourceRelationKindV1,
        MultiSourceRoleNodeV1, MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1,
        MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1, PreActionTopologyCommitV1,
        RELATION_FRAME_SCHEMA, RelationAtom, RelationFrame, sha256_bytes,
    };
    use nando_operator_learning::{
        SOURCE_NEUTRAL_EXTRACTOR_VERSION,
        multi_source::{
            Ms3VersionSpaceVersionsV1, PreActionTopologyAuditRowV1, TransportBindingLedgerV1,
            TransportTerminalReceiptV1, prepare_ms3_frozen_version_space_v1,
            seal_ms3_independent_future_v1,
        },
    };

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
            request_phase_atom_ids: vec![1],
            pre_action_context_atom_ids: vec![2],
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
        pass: bool,
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
        let (topology_archive, mut lifecycle, generation_one) = terminal_generation(&root, false);
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
    fn prepared_successor_is_idempotent_when_manifest_publish_did_not_happen() {
        let root = test_root("retry");
        let (topology_archive, mut lifecycle, generation_one) = terminal_generation(&root, false);
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
    fn future_pass_does_not_create_a_successor() {
        let root = test_root("pass");
        let (topology_archive, mut lifecycle, generation_one) = terminal_generation(&root, true);
        let result = lifecycle.prepare_successor(
            &topology_archive,
            200,
            &generation_one.acquisition,
            &generation_one.frozen,
        );
        assert!(matches!(
            result,
            Err(ref error) if error == "ms3_lifecycle_successor_requires_contradiction"
        ));
        assert_eq!(lifecycle.manifest().active_generation_sequence, 1);
        assert!(!root.join(GENERATIONS_DIRECTORY).exists());
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
