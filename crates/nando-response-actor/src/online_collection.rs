use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use nando_core::wave::{phase_coherence, phase_margin_to_micro, phase_vector_from_atom_ids};

use crate::{
    AstProgramKind, CollectionSynthesisExample, ResponseExecutionStatus, ResponsePackage,
    ResponsePackageOrigin, ResponsePackageProof, ResponsePackageState, ResponseProgram,
    canonical_json_sha256, enumerate_source_neutral_response_programs, execute_response,
    is_privacy_safe_online_response_program,
    package::{
        request_phase_atom_ids, response_pre_action_context_atom_ids,
        response_program_external_verifier_schema,
    },
    response_program_exactly_matches_example, response_program_kind,
    response_program_required_routing_atom_ids, source_neutral_verifier_for_program,
    verify_response_independently,
};

const ONLINE_COLLECTION_SCHEMA_V1: &str = "nando.online-collection-version-space.v1";
const ONLINE_COLLECTION_SCHEMA_V2: &str = "nando.online-collection-program-pools.v2";
const ONLINE_COLLECTION_SCHEMA_V3: &str = "nando.online-outcome-version-space.v3";
const ONLINE_COLLECTION_POOLING_STRATEGY_V3: u32 = 3;
const ONLINE_COLLECTION_CHECKPOINT_MAGIC_V2: &[u8; 4] = b"NCO2";
const ONLINE_COLLECTION_CHECKPOINT_MAGIC_V3: &[u8; 4] = b"NCO3";
const MAX_PERSISTED_PARITY_BYTES_PER_BUCKET: usize = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineCollectionConfig {
    pub support_rows: usize,
    pub future_rows: usize,
    pub max_buckets: usize,
    pub max_receipts_per_bucket: usize,
}

impl Default for OnlineCollectionConfig {
    fn default() -> Self {
        Self {
            support_rows: 32,
            future_rows: 32,
            max_buckets: 1_024,
            max_receipts_per_bucket: 128,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineCollectionObservation {
    pub evidence_graph_sha256: String,
    pub client_intent_id_sha256: String,
    pub session_id_sha256: String,
    pub event_time_unix_nanos: Option<u64>,
    pub estimated_input_tokens: u64,
    pub example: CollectionSynthesisExample,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineCollectionReceipt {
    pub evidence_graph_sha256: String,
    pub client_intent_id_sha256: String,
    pub session_id_sha256: String,
    pub event_time_unix_nanos: Option<u64>,
    pub layout_sha256: String,
    pub estimated_input_tokens: u64,
    pub verifier_pass: bool,
    #[serde(default)]
    pub request_atom_ids: Vec<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineCollectionWaveCausalReport {
    pub schema: String,
    pub package_id: String,
    pub verdict: String,
    pub support_rows: usize,
    pub future_rows: usize,
    pub full_phase_correct: usize,
    pub no_phase_correct: usize,
    pub shuffled_phase_correct: usize,
    pub random_center_correct: usize,
    pub no_anti_center_correct: usize,
    pub full_phase_exact_checks: usize,
    pub no_phase_exact_checks: usize,
    pub shuffled_phase_exact_checks: usize,
    pub random_center_exact_checks: usize,
    pub no_anti_center_exact_checks: usize,
    pub wrong_accepts: usize,
    pub wave_margin_micro: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineCollectionAdmissionCandidate {
    pub package: ResponsePackage,
    pub bucket_id: String,
    pub program_sha256: String,
    pub support_watermark_event_time_unix_nanos: u64,
    pub support_manifest_sha256: String,
    pub future_manifest_sha256: String,
    pub causal_report: OnlineCollectionWaveCausalReport,
    pub support_receipts: Vec<OnlineCollectionReceipt>,
    pub future_receipts: Vec<OnlineCollectionReceipt>,
    #[serde(default)]
    pub runtime_parity_cases: Vec<crate::RuntimeParityCase>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineCollectionBucketStatus {
    pub bucket_id: String,
    pub version_space_size: usize,
    pub support_rows: usize,
    pub future_rows: usize,
    pub future_sessions: usize,
    pub future_layouts: usize,
    pub wrong_accepts: usize,
    pub frozen: bool,
    pub candidate_program_sha256: Option<String>,
    pub candidate_program_kind: Option<String>,
    pub rejected_programs: usize,
    pub learned_anti_atoms: usize,
    pub common_request_atoms: usize,
    pub support_tokens: u64,
    pub future_tokens: u64,
    pub support_watermark_event_time_unix_nanos: Option<u64>,
    pub support_manifest_sha256: Option<String>,
    pub future_manifest_sha256: Option<String>,
    pub runtime_parity_cases: usize,
    pub admission_blocker: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineCollectionStatus {
    pub pooling_strategy_version: u32,
    pub observations_total: u64,
    pub duplicate_observations_total: u64,
    pub unsupported_total: u64,
    pub ambiguous_assignment_total: u64,
    pub exact_checks_total: u64,
    pub candidates_enumerated_total: u64,
    pub full_enumerations_total: u64,
    pub version_space_intersection_checks_total: u64,
    pub guard_scheduled_buckets_total: u64,
    pub guard_pruned_buckets_total: u64,
    pub unsupported_expected_in_latest_output: u64,
    pub unsupported_expected_in_any_output: u64,
    pub unsupported_without_exact_source_span: u64,
    pub unsupported_with_scalar_overlap: u64,
    pub policy_rejected_exact_matches: u64,
    pub counterexamples_total: u64,
    pub cegis_subcenters_total: u64,
    pub revoked_candidates_total: u64,
    pub late_after_freeze_total: u64,
    pub future_intent_rejected_total: u64,
    pub exact_executable_observations_total: u64,
    pub program_pool_reuse_total: u64,
    pub program_pool_receipts_total: u64,
    pub legacy_partial_observations_discarded_total: u64,
    pub legacy_partial_buckets_discarded_total: u64,
    pub legacy_partial_receipts_discarded_total: u64,
    pub buckets_total: usize,
    pub frozen_buckets_total: usize,
    pub pre_admission_ready_buckets_total: usize,
    pub support_receipts_unique_total: usize,
    pub future_receipts_unique_total: usize,
    pub support_tokens_unique_total: u64,
    pub future_tokens_unique_total: u64,
    pub wrong_accepts_total: usize,
    pub runtime_parity_cases_total: usize,
    pub frozen_program_kinds: BTreeMap<String, usize>,
    pub buckets: Vec<OnlineCollectionBucketStatus>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct OnlineCollectionBucket {
    bucket_id: String,
    programs: BTreeMap<String, ResponseProgram>,
    #[serde(default)]
    common_request_atom_ids: BTreeSet<u64>,
    support: Vec<OnlineCollectionReceipt>,
    future: Vec<OnlineCollectionReceipt>,
    #[serde(default)]
    runtime_examples: BTreeMap<String, CollectionSynthesisExample>,
    frozen_program_sha256: Option<String>,
    #[serde(default)]
    support_watermark_event_time_unix_nanos: Option<u64>,
    #[serde(default)]
    support_manifest_sha256: Option<String>,
    #[serde(default)]
    rejected_program_sha256: BTreeSet<String>,
    #[serde(default)]
    learned_anti_atom_ids: BTreeSet<u64>,
    wrong_accepts: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct OnlineCollectionCheckpoint {
    schema: String,
    #[serde(default)]
    pooling_strategy_version: u32,
    config: OnlineCollectionConfig,
    observations_total: u64,
    #[serde(default)]
    duplicate_observations_total: u64,
    #[serde(default)]
    observed_evidence_graph_sha256: BTreeSet<String>,
    unsupported_total: u64,
    ambiguous_assignment_total: u64,
    exact_checks_total: u64,
    candidates_enumerated_total: u64,
    #[serde(default)]
    full_enumerations_total: u64,
    #[serde(default)]
    version_space_intersection_checks_total: u64,
    #[serde(default)]
    guard_scheduled_buckets_total: u64,
    #[serde(default)]
    guard_pruned_buckets_total: u64,
    #[serde(default)]
    unsupported_expected_in_latest_output: u64,
    #[serde(default)]
    unsupported_expected_in_any_output: u64,
    #[serde(default)]
    unsupported_without_exact_source_span: u64,
    #[serde(default)]
    unsupported_with_scalar_overlap: u64,
    #[serde(default)]
    policy_rejected_exact_matches: u64,
    #[serde(default)]
    counterexamples_total: u64,
    #[serde(default)]
    cegis_subcenters_total: u64,
    #[serde(default)]
    revoked_candidates_total: u64,
    #[serde(default)]
    late_after_freeze_total: u64,
    #[serde(default)]
    future_intent_rejected_total: u64,
    #[serde(default)]
    exact_executable_observations_total: u64,
    #[serde(default)]
    program_pool_reuse_total: u64,
    #[serde(default)]
    program_pool_receipts_total: u64,
    #[serde(default)]
    legacy_partial_observations_discarded_total: u64,
    #[serde(default)]
    legacy_partial_buckets_discarded_total: u64,
    #[serde(default)]
    legacy_partial_receipts_discarded_total: u64,
    buckets: Vec<OnlineCollectionBucket>,
}

#[derive(Serialize)]
struct CollectionSupportManifestMaterial<'a> {
    schema: &'static str,
    bucket_id: &'a str,
    program_sha256: &'a str,
    watermark_event_time_unix_nanos: u64,
    receipts: &'a [OnlineCollectionReceipt],
}

#[derive(Serialize)]
struct CollectionFutureManifestMaterial<'a> {
    schema: &'static str,
    support_manifest_sha256: &'a str,
    receipts: &'a [OnlineCollectionReceipt],
}

pub struct OnlineCollectionMiner {
    path: PathBuf,
    checkpoint: OnlineCollectionCheckpoint,
}

impl OnlineCollectionMiner {
    pub fn open(path: impl Into<PathBuf>, config: OnlineCollectionConfig) -> Result<Self, String> {
        validate_config(config)?;
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "online_collection_checkpoint_dir:{}:{error}",
                    parent.display()
                )
            })?;
        }
        let mut checkpoint = if path.exists() {
            decode_collection_checkpoint(&fs::read(&path).map_err(|error| {
                format!(
                    "online_collection_checkpoint_read:{}:{error}",
                    path.display()
                )
            })?)?
        } else {
            OnlineCollectionCheckpoint {
                schema: ONLINE_COLLECTION_SCHEMA_V3.to_owned(),
                pooling_strategy_version: ONLINE_COLLECTION_POOLING_STRATEGY_V3,
                config,
                observations_total: 0,
                duplicate_observations_total: 0,
                observed_evidence_graph_sha256: BTreeSet::new(),
                unsupported_total: 0,
                ambiguous_assignment_total: 0,
                exact_checks_total: 0,
                candidates_enumerated_total: 0,
                full_enumerations_total: 0,
                version_space_intersection_checks_total: 0,
                guard_scheduled_buckets_total: 0,
                guard_pruned_buckets_total: 0,
                unsupported_expected_in_latest_output: 0,
                unsupported_expected_in_any_output: 0,
                unsupported_without_exact_source_span: 0,
                unsupported_with_scalar_overlap: 0,
                policy_rejected_exact_matches: 0,
                counterexamples_total: 0,
                cegis_subcenters_total: 0,
                revoked_candidates_total: 0,
                late_after_freeze_total: 0,
                future_intent_rejected_total: 0,
                exact_executable_observations_total: 0,
                program_pool_reuse_total: 0,
                program_pool_receipts_total: 0,
                legacy_partial_observations_discarded_total: 0,
                legacy_partial_buckets_discarded_total: 0,
                legacy_partial_receipts_discarded_total: 0,
                buckets: Vec::new(),
            }
        };
        let migrated = checkpoint.schema != ONLINE_COLLECTION_SCHEMA_V3
            || checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V3;
        if migrated {
            migrate_collection_program_pools(&mut checkpoint)?;
        }
        validate_checkpoint(&checkpoint, config)?;
        let mut miner = Self { path, checkpoint };
        for index in 0..miner.checkpoint.buckets.len() {
            miner.normalize_bucket_receipts(index);
            miner.maybe_freeze(index)?;
        }
        if migrated {
            miner.persist()?;
        }
        Ok(miner)
    }

    pub fn observe(&mut self, observation: OnlineCollectionObservation) -> Result<(), String> {
        self.observe_with_persistence(observation, true)
    }

    pub fn observe_buffered(
        &mut self,
        observation: OnlineCollectionObservation,
    ) -> Result<(), String> {
        self.observe_with_persistence(observation, false)
    }

    pub fn flush(&self) -> Result<(), String> {
        self.persist()
    }

    fn observe_with_persistence(
        &mut self,
        observation: OnlineCollectionObservation,
        durable: bool,
    ) -> Result<(), String> {
        validate_observation(&observation)?;
        if self
            .checkpoint
            .observed_evidence_graph_sha256
            .contains(&observation.evidence_graph_sha256)
        {
            self.checkpoint.duplicate_observations_total = self
                .checkpoint
                .duplicate_observations_total
                .saturating_add(1);
            return self.persist_if(durable);
        }
        let evidence_graph_sha256 = observation.evidence_graph_sha256.clone();
        self.checkpoint.observations_total = self.checkpoint.observations_total.saturating_add(1);
        if self.evaluate_frozen_candidates(&observation)? {
            return self.persist_new_observation(evidence_graph_sha256, durable);
        }
        let matching_existing = self.matching_unfrozen_buckets(&observation)?;
        match matching_existing.as_slice() {
            [(index, matching_programs)] => {
                self.checkpoint.exact_executable_observations_total = self
                    .checkpoint
                    .exact_executable_observations_total
                    .saturating_add(1);
                self.update_bucket(*index, matching_programs, &observation)?;
                return self.persist_new_observation(evidence_graph_sha256, durable);
            }
            [_, _, ..] => {
                self.checkpoint.exact_executable_observations_total = self
                    .checkpoint
                    .exact_executable_observations_total
                    .saturating_add(1);
                self.checkpoint.ambiguous_assignment_total =
                    self.checkpoint.ambiguous_assignment_total.saturating_add(1);
                for (index, matching_programs) in matching_existing.iter().cloned() {
                    self.update_bucket(index, &matching_programs, &observation)?;
                }
                return self.persist_new_observation(evidence_graph_sha256, durable);
            }
            [] => {}
        }
        self.checkpoint.full_enumerations_total =
            self.checkpoint.full_enumerations_total.saturating_add(1);
        let version_space = enumerate_source_neutral_response_programs(&observation.example)
            .map_err(|error| format!("online_collection_synthesis:{error}"))?;
        self.checkpoint.exact_checks_total = self
            .checkpoint
            .exact_checks_total
            .saturating_add(version_space.exact_checks as u64);
        self.checkpoint.candidates_enumerated_total = self
            .checkpoint
            .candidates_enumerated_total
            .saturating_add(version_space.candidates_enumerated as u64);
        self.checkpoint.policy_rejected_exact_matches = self
            .checkpoint
            .policy_rejected_exact_matches
            .saturating_add(version_space.policy_rejected_exact_matches as u64);
        let programs = version_space
            .programs
            .into_iter()
            .filter(|program| {
                response_program_exactly_matches_example(program, &observation.example)
            })
            .filter(is_privacy_safe_online_response_program)
            .map(|program| {
                canonical_json_sha256(&program)
                    .map(|digest| (digest, program))
                    .map_err(str::to_owned)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        if programs.is_empty() {
            self.checkpoint.unsupported_total = self.checkpoint.unsupported_total.saturating_add(1);
            match unsupported_source_span(&observation.example) {
                UnsupportedSourceSpan::Latest => {
                    self.checkpoint.unsupported_expected_in_latest_output = self
                        .checkpoint
                        .unsupported_expected_in_latest_output
                        .saturating_add(1);
                }
                UnsupportedSourceSpan::Earlier => {
                    self.checkpoint.unsupported_expected_in_any_output = self
                        .checkpoint
                        .unsupported_expected_in_any_output
                        .saturating_add(1);
                }
                UnsupportedSourceSpan::Missing => {
                    self.checkpoint.unsupported_without_exact_source_span = self
                        .checkpoint
                        .unsupported_without_exact_source_span
                        .saturating_add(1);
                }
            }
            if has_scalar_overlap(&observation.example) {
                self.checkpoint.unsupported_with_scalar_overlap = self
                    .checkpoint
                    .unsupported_with_scalar_overlap
                    .saturating_add(1);
            }
            return self.persist_new_observation(evidence_graph_sha256, durable);
        }
        self.checkpoint.exact_executable_observations_total = self
            .checkpoint
            .exact_executable_observations_total
            .saturating_add(1);
        let matching = self
            .checkpoint
            .buckets
            .iter()
            .enumerate()
            .filter(|(_, bucket)| bucket.programs.keys().any(|key| programs.contains_key(key)))
            .filter(|(_, bucket)| bucket.frozen_program_sha256.is_none())
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        match matching.as_slice() {
            [] => self.create_bucket(programs, &observation)?,
            [index] => {
                let matching_programs = programs.keys().cloned().collect::<BTreeSet<_>>();
                self.update_bucket(*index, &matching_programs, &observation)?;
            }
            _ => {
                self.checkpoint.ambiguous_assignment_total =
                    self.checkpoint.ambiguous_assignment_total.saturating_add(1);
                for index in matching.iter().copied() {
                    let matching_programs = self.checkpoint.buckets[index]
                        .programs
                        .keys()
                        .filter(|digest| programs.contains_key(*digest))
                        .cloned()
                        .collect::<BTreeSet<_>>();
                    self.update_bucket(index, &matching_programs, &observation)?;
                }
            }
        }
        self.persist_new_observation(evidence_graph_sha256, durable)
    }

    #[must_use]
    pub fn status(&self) -> OnlineCollectionStatus {
        let mut support_receipts = BTreeMap::new();
        let mut future_receipts = BTreeMap::new();
        let mut runtime_parity_receipts = BTreeSet::new();
        for bucket in &self.checkpoint.buckets {
            for receipt in &bucket.support {
                support_receipts
                    .entry(receipt.evidence_graph_sha256.clone())
                    .or_insert(receipt.estimated_input_tokens);
            }
            for receipt in &bucket.future {
                future_receipts
                    .entry(receipt.evidence_graph_sha256.clone())
                    .or_insert(receipt.estimated_input_tokens);
                if bucket
                    .runtime_examples
                    .contains_key(&receipt.evidence_graph_sha256)
                {
                    runtime_parity_receipts.insert(receipt.evidence_graph_sha256.clone());
                }
            }
        }
        let mut buckets = self
            .checkpoint
            .buckets
            .iter()
            .map(bucket_status)
            .collect::<Vec<_>>();
        buckets.sort_by(|left, right| left.bucket_id.cmp(&right.bucket_id));
        let frozen_buckets_total = buckets.iter().filter(|bucket| bucket.frozen).count();
        let pre_admission_ready_buckets_total = buckets
            .iter()
            .filter(|bucket| bucket.admission_blocker.is_none())
            .count();
        let wrong_accepts_total = buckets.iter().map(|bucket| bucket.wrong_accepts).sum();
        let mut frozen_program_kinds = BTreeMap::new();
        for kind in buckets
            .iter()
            .filter_map(|bucket| bucket.candidate_program_kind.as_ref())
        {
            *frozen_program_kinds.entry(kind.clone()).or_insert(0) += 1;
        }
        OnlineCollectionStatus {
            pooling_strategy_version: self.checkpoint.pooling_strategy_version,
            observations_total: self.checkpoint.observations_total,
            duplicate_observations_total: self.checkpoint.duplicate_observations_total,
            unsupported_total: self.checkpoint.unsupported_total,
            ambiguous_assignment_total: self.checkpoint.ambiguous_assignment_total,
            exact_checks_total: self.checkpoint.exact_checks_total,
            candidates_enumerated_total: self.checkpoint.candidates_enumerated_total,
            full_enumerations_total: self.checkpoint.full_enumerations_total,
            version_space_intersection_checks_total: self
                .checkpoint
                .version_space_intersection_checks_total,
            guard_scheduled_buckets_total: self.checkpoint.guard_scheduled_buckets_total,
            guard_pruned_buckets_total: self.checkpoint.guard_pruned_buckets_total,
            unsupported_expected_in_latest_output: self
                .checkpoint
                .unsupported_expected_in_latest_output,
            unsupported_expected_in_any_output: self.checkpoint.unsupported_expected_in_any_output,
            unsupported_without_exact_source_span: self
                .checkpoint
                .unsupported_without_exact_source_span,
            unsupported_with_scalar_overlap: self.checkpoint.unsupported_with_scalar_overlap,
            policy_rejected_exact_matches: self.checkpoint.policy_rejected_exact_matches,
            counterexamples_total: self.checkpoint.counterexamples_total,
            cegis_subcenters_total: self.checkpoint.cegis_subcenters_total,
            revoked_candidates_total: self.checkpoint.revoked_candidates_total,
            late_after_freeze_total: self.checkpoint.late_after_freeze_total,
            future_intent_rejected_total: self.checkpoint.future_intent_rejected_total,
            exact_executable_observations_total: self
                .checkpoint
                .exact_executable_observations_total,
            program_pool_reuse_total: self.checkpoint.program_pool_reuse_total,
            program_pool_receipts_total: self.checkpoint.program_pool_receipts_total,
            legacy_partial_observations_discarded_total: self
                .checkpoint
                .legacy_partial_observations_discarded_total,
            legacy_partial_buckets_discarded_total: self
                .checkpoint
                .legacy_partial_buckets_discarded_total,
            legacy_partial_receipts_discarded_total: self
                .checkpoint
                .legacy_partial_receipts_discarded_total,
            buckets_total: buckets.len(),
            frozen_buckets_total,
            pre_admission_ready_buckets_total,
            support_receipts_unique_total: support_receipts.len(),
            future_receipts_unique_total: future_receipts.len(),
            support_tokens_unique_total: support_receipts.values().copied().sum(),
            future_tokens_unique_total: future_receipts.values().copied().sum(),
            wrong_accepts_total,
            runtime_parity_cases_total: runtime_parity_receipts.len(),
            frozen_program_kinds,
            buckets,
        }
    }

    fn persist_new_observation(
        &mut self,
        evidence_graph_sha256: String,
        durable: bool,
    ) -> Result<(), String> {
        self.checkpoint
            .observed_evidence_graph_sha256
            .insert(evidence_graph_sha256.clone());
        if let Err(error) = self.persist_if(durable) {
            self.checkpoint
                .observed_evidence_graph_sha256
                .remove(&evidence_graph_sha256);
            return Err(error);
        }
        Ok(())
    }

    fn persist_if(&self, durable: bool) -> Result<(), String> {
        if durable { self.persist() } else { Ok(()) }
    }

    pub fn quarantine_packages(&self) -> Result<Vec<ResponsePackage>, String> {
        let mut packages = Vec::new();
        for (index, bucket) in self.checkpoint.buckets.iter().enumerate() {
            if let Some(package) = self.package_for_bucket(index, bucket, false)? {
                packages.push(package);
            }
        }
        packages.sort_by(|left, right| left.package_id.cmp(&right.package_id));
        Ok(packages)
    }

    pub fn admission_candidates(&self) -> Result<Vec<OnlineCollectionAdmissionCandidate>, String> {
        let mut candidates = Vec::new();
        for (index, bucket) in self.checkpoint.buckets.iter().enumerate() {
            let Some(mut package) = self.package_for_bucket(index, bucket, false)? else {
                continue;
            };
            let causal_report = self.collection_causal_report(bucket, &package)?;
            if causal_report.verdict != "PASS" {
                continue;
            }
            package.state = ResponsePackageState::Active;
            package.proof.wave_causal_pass = true;
            package.wave_margin_micro = causal_report.wave_margin_micro;
            if !package.eligible_for_admission_candidate() {
                continue;
            }
            candidates.push(OnlineCollectionAdmissionCandidate {
                package,
                bucket_id: bucket.bucket_id.clone(),
                program_sha256: bucket
                    .frozen_program_sha256
                    .clone()
                    .ok_or_else(|| "online_collection_frozen_program_missing".to_owned())?,
                support_watermark_event_time_unix_nanos: bucket
                    .support_watermark_event_time_unix_nanos
                    .ok_or_else(|| "online_collection_support_watermark_missing".to_owned())?,
                support_manifest_sha256: bucket
                    .support_manifest_sha256
                    .clone()
                    .ok_or_else(|| "online_collection_support_manifest_missing".to_owned())?,
                future_manifest_sha256: collection_future_manifest_digest(bucket)?,
                causal_report,
                support_receipts: bucket.support.clone(),
                future_receipts: bucket.future.clone(),
                runtime_parity_cases: bucket
                    .future
                    .iter()
                    .filter_map(|receipt| {
                        bucket
                            .runtime_examples
                            .get(&receipt.evidence_graph_sha256)
                            .map(|example| crate::RuntimeParityCase {
                                evidence_ref_sha256: receipt.evidence_graph_sha256.clone(),
                                request_text: String::new(),
                                provider_payload: example.provider_payload.clone(),
                                expected_response: example.expected_response.clone(),
                            })
                    })
                    .collect(),
            });
        }
        candidates.sort_by(|left, right| left.package.package_id.cmp(&right.package.package_id));
        Ok(candidates)
    }

    fn package_for_bucket(
        &self,
        index: usize,
        bucket: &OnlineCollectionBucket,
        wave_causal_pass: bool,
    ) -> Result<Option<ResponsePackage>, String> {
        let Some(program_sha256) = &bucket.frozen_program_sha256 else {
            return Ok(None);
        };
        let Some(support_manifest_sha256) = &bucket.support_manifest_sha256 else {
            return Ok(None);
        };
        if bucket.wrong_accepts > 0 {
            return Ok(None);
        }
        let future_manifest_sha256 = collection_future_manifest_digest(bucket)?;
        let program = bucket
            .programs
            .get(program_sha256)
            .ok_or_else(|| "online_collection_frozen_program_missing".to_owned())?
            .clone();
        let verifier = source_neutral_verifier_for_program(&program).map_err(str::to_owned)?;
        let verifier_schema = response_program_external_verifier_schema(&program)
            .ok_or_else(|| "online_collection_external_verifier_schema_missing".to_owned())?;
        let mut required_routing_atom_ids = response_program_required_routing_atom_ids(&program);
        required_routing_atom_ids.extend(bucket.common_request_atom_ids.iter().copied());
        required_routing_atom_ids.sort_unstable();
        required_routing_atom_ids.dedup();
        let phase_centers = bucket_phase_center_atom_ids(bucket);
        let anti_centers = self.anti_center_atom_ids(index);
        let route_sha256 =
            canonical_json_sha256(&(&required_routing_atom_ids, &phase_centers, &anti_centers))
                .map_err(str::to_owned)?;
        let wave_margin_micro = learned_wave_margin_micro(bucket, &phase_centers, &anti_centers);
        let package = ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id: format!(
                "evidence-collection-{}-{}-{}-{}",
                &program_sha256[..8],
                &support_manifest_sha256[..8],
                &future_manifest_sha256[..8],
                &route_sha256[..8]
            ),
            origin: ResponsePackageOrigin::GroundedSynthesis,
            state: ResponsePackageState::Quarantine,
            program,
            verifier: Some(verifier),
            routing_predicates: Vec::new(),
            required_routing_atom_ids,
            phase_centers,
            anti_centers,
            wave_margin_micro,
            learned_wave_route: None,
            proof: ResponsePackageProof {
                support_rows: bucket.support.len(),
                future_rows: bucket.future.len(),
                distinct_sessions: distinct_receipt_sessions(&bucket.future),
                distinct_surfaces: distinct_receipt_layouts(&bucket.future),
                wrong_accepts: bucket.wrong_accepts,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass,
                verifier_schema: verifier_schema.to_owned(),
            },
        };
        package.validate().map_err(str::to_owned)?;
        Ok(Some(package))
    }

    fn anti_center_atom_ids(&self, index: usize) -> Vec<u64> {
        self.checkpoint.buckets[index]
            .learned_anti_atom_ids
            .iter()
            .copied()
            .take(32)
            .collect()
    }

    fn collection_causal_report(
        &self,
        bucket: &OnlineCollectionBucket,
        package: &ResponsePackage,
    ) -> Result<OnlineCollectionWaveCausalReport, String> {
        let threshold = package.wave_margin_micro;
        let full = phase_vector_from_atom_ids(package.phase_centers.iter().copied(), 16);
        let anti = phase_vector_from_atom_ids(package.anti_centers.iter().copied(), 16);
        let shuffled = phase_vector_from_atom_ids(
            package
                .phase_centers
                .iter()
                .map(|atom| atom.rotate_left(17) ^ 0x9e37_79b9_7f4a_7c15),
            16,
        );
        let random = phase_vector_from_atom_ids(
            package
                .phase_centers
                .iter()
                .map(|atom| atom.wrapping_mul(0xd6e8_feb8_6659_fd93) ^ 0xa5a5_a5a5_a5a5_a5a5),
            16,
        );
        let no_anti = phase_vector_from_atom_ids(std::iter::empty::<u64>(), 16);
        let routes = |receipt: &OnlineCollectionReceipt,
                      center: &[nando_core::wave::PhaseCenterCell],
                      anti_center: &[nando_core::wave::PhaseCenterCell],
                      hard_anti_atoms: &[u64]| {
            if !package
                .required_routing_atom_ids
                .iter()
                .all(|atom| receipt.request_atom_ids.binary_search(atom).is_ok())
            {
                return false;
            }
            if hard_anti_atoms
                .iter()
                .any(|atom| receipt.request_atom_ids.binary_search(atom).is_ok())
            {
                return false;
            }
            let query = phase_vector_from_atom_ids(receipt.request_atom_ids.iter().copied(), 16);
            phase_margin_to_micro(
                phase_coherence(&query, center) - phase_coherence(&query, anti_center),
            )
            .is_ok_and(|margin| margin >= threshold)
        };
        let full_phase_correct = bucket
            .future
            .iter()
            .filter(|receipt| routes(receipt, &full, &anti, &package.anti_centers))
            .count();
        let shuffled_phase_correct = bucket
            .future
            .iter()
            .filter(|receipt| routes(receipt, &shuffled, &anti, &package.anti_centers))
            .count();
        let random_center_correct = bucket
            .future
            .iter()
            .filter(|receipt| routes(receipt, &random, &anti, &package.anti_centers))
            .count();
        let no_anti_center_correct = bucket
            .future
            .iter()
            .filter(|receipt| routes(receipt, &full, &no_anti, &[]))
            .count();
        let no_phase_candidates = self
            .checkpoint
            .buckets
            .iter()
            .map(|candidate| candidate.programs.len().max(1))
            .sum::<usize>()
            .max(1);
        let full_phase_exact_checks = full_phase_correct;
        let no_phase_exact_checks = bucket.future.len().saturating_mul(no_phase_candidates);
        let pass = bucket.support.len() >= 32
            && bucket.future.len() >= 32
            && distinct_receipt_sessions(&bucket.future) >= 3
            && distinct_receipt_layouts(&bucket.future) >= 2
            && bucket.wrong_accepts == 0
            && full_phase_correct == bucket.future.len()
            && full_phase_exact_checks < no_phase_exact_checks
            && full_phase_correct > shuffled_phase_correct
            && full_phase_correct > random_center_correct;
        Ok(OnlineCollectionWaveCausalReport {
            schema: "nando.online-collection-wave-causal-report.v1".to_owned(),
            package_id: package.package_id.clone(),
            verdict: if pass { "PASS" } else { "WATCH" }.to_owned(),
            support_rows: bucket.support.len(),
            future_rows: bucket.future.len(),
            full_phase_correct,
            no_phase_correct: bucket.future.len(),
            shuffled_phase_correct,
            random_center_correct,
            no_anti_center_correct,
            full_phase_exact_checks,
            no_phase_exact_checks,
            shuffled_phase_exact_checks: shuffled_phase_correct,
            random_center_exact_checks: random_center_correct,
            no_anti_center_exact_checks: no_anti_center_correct,
            wrong_accepts: bucket.wrong_accepts,
            wave_margin_micro: threshold,
        })
    }

    fn create_bucket(
        &mut self,
        programs: BTreeMap<String, ResponseProgram>,
        observation: &OnlineCollectionObservation,
    ) -> Result<(), String> {
        let request_atoms = observation_request_atom_ids(observation);
        let layout_sha256 = structural_layout_sha256(&observation.example.provider_payload)?;
        for (program_sha256, program) in programs {
            if self.checkpoint.buckets.len() >= self.checkpoint.config.max_buckets {
                self.checkpoint.unsupported_total =
                    self.checkpoint.unsupported_total.saturating_add(1);
                break;
            }
            let base_bucket_id = canonical_json_sha256(&(
                "nando.collection-program-pool.v2",
                program_sha256.as_str(),
            ))
            .map_err(str::to_owned)?;
            let bucket_id = if self
                .checkpoint
                .buckets
                .iter()
                .any(|bucket| bucket.bucket_id == base_bucket_id)
            {
                canonical_json_sha256(&(
                    "nando.collection-program-subcenter-seed.v2",
                    program_sha256.as_str(),
                    &request_atoms,
                    layout_sha256.as_str(),
                ))
                .map_err(str::to_owned)?
            } else {
                base_bucket_id
            };
            if self
                .checkpoint
                .buckets
                .iter()
                .any(|bucket| bucket.bucket_id == bucket_id)
            {
                continue;
            }
            let programs = BTreeMap::from([(program_sha256, program)]);
            let support = vec![receipt_with_program_atoms(observation, true, &programs)?];
            self.checkpoint.program_pool_receipts_total = self
                .checkpoint
                .program_pool_receipts_total
                .saturating_add(1);
            self.checkpoint.buckets.push(OnlineCollectionBucket {
                bucket_id,
                programs,
                common_request_atom_ids: request_atoms.clone(),
                support,
                future: Vec::new(),
                runtime_examples: BTreeMap::from([(
                    observation.evidence_graph_sha256.clone(),
                    observation.example.clone(),
                )]),
                frozen_program_sha256: None,
                support_watermark_event_time_unix_nanos: None,
                support_manifest_sha256: None,
                rejected_program_sha256: BTreeSet::new(),
                learned_anti_atom_ids: BTreeSet::new(),
                wrong_accepts: 0,
            });
            let index = self.checkpoint.buckets.len().saturating_sub(1);
            self.maybe_freeze(index)?;
        }
        Ok(())
    }

    fn update_bucket(
        &mut self,
        index: usize,
        matching_programs: &BTreeSet<String>,
        observation: &OnlineCollectionObservation,
    ) -> Result<(), String> {
        if index >= self.checkpoint.buckets.len() {
            return Err("online_collection_bucket_missing".to_owned());
        }
        self.checkpoint.program_pool_reuse_total =
            self.checkpoint.program_pool_reuse_total.saturating_add(1);
        self.checkpoint.program_pool_receipts_total = self
            .checkpoint
            .program_pool_receipts_total
            .saturating_add(1);
        let bucket = self
            .checkpoint
            .buckets
            .get_mut(index)
            .expect("index checked above");
        bucket
            .programs
            .retain(|digest, _| matching_programs.contains(digest));
        let request_atoms = observation_request_atom_ids(observation);
        bucket
            .common_request_atom_ids
            .retain(|atom| request_atoms.contains(atom));
        if bucket.programs.is_empty() {
            return Err("online_collection_version_space_became_empty".to_owned());
        }
        push_bounded(
            &mut bucket.support,
            receipt_with_program_atoms(observation, true, &bucket.programs)?,
            self.checkpoint.config.max_receipts_per_bucket,
        );
        self.normalize_bucket_receipts(index);
        self.maybe_freeze(index)?;
        Ok(())
    }

    fn matching_unfrozen_buckets(
        &mut self,
        observation: &OnlineCollectionObservation,
    ) -> Result<Vec<(usize, BTreeSet<String>)>, String> {
        let mut checks = 0_u64;
        let mut scheduled = 0_u64;
        let mut pruned = 0_u64;
        let mut matching = Vec::new();
        let request_atoms = observation_request_atom_ids(observation);
        for (index, bucket) in self.checkpoint.buckets.iter().enumerate() {
            if bucket.frozen_program_sha256.is_some() {
                continue;
            }
            if !bucket.common_request_atom_ids.is_empty()
                && !bucket
                    .common_request_atom_ids
                    .iter()
                    .all(|atom| request_atoms.contains(atom))
            {
                pruned = pruned.saturating_add(1);
                continue;
            }
            scheduled = scheduled.saturating_add(1);
            let mut matching_programs = BTreeSet::new();
            for (digest, program) in &bucket.programs {
                checks = checks.saturating_add(1);
                if !response_program_exactly_matches_example(program, &observation.example) {
                    continue;
                }
                let execution =
                    execute_response(program, "", &observation.example.provider_payload);
                let Some(response) = execution.response.as_deref() else {
                    continue;
                };
                let verifier =
                    source_neutral_verifier_for_program(program).map_err(str::to_owned)?;
                if verify_response_independently(
                    &verifier,
                    &observation.example.provider_payload,
                    response,
                )
                .is_ok()
                {
                    matching_programs.insert(digest.clone());
                }
            }
            if !matching_programs.is_empty() {
                matching.push((index, matching_programs));
            }
        }
        self.checkpoint.version_space_intersection_checks_total = self
            .checkpoint
            .version_space_intersection_checks_total
            .saturating_add(checks);
        self.checkpoint.guard_scheduled_buckets_total = self
            .checkpoint
            .guard_scheduled_buckets_total
            .saturating_add(scheduled);
        self.checkpoint.guard_pruned_buckets_total = self
            .checkpoint
            .guard_pruned_buckets_total
            .saturating_add(pruned);
        self.checkpoint.exact_checks_total =
            self.checkpoint.exact_checks_total.saturating_add(checks);
        Ok(matching)
    }

    fn evaluate_frozen_candidates(
        &mut self,
        observation: &OnlineCollectionObservation,
    ) -> Result<bool, String> {
        let mut verified_match = false;
        let mut late_after_freeze = 0_u64;
        let mut future_intent_rejected = 0_u64;
        let mut pending_subcenters = Vec::new();
        for index in 0..self.checkpoint.buckets.len() {
            let Some(program_sha256) = self.checkpoint.buckets[index].frozen_program_sha256.clone()
            else {
                continue;
            };
            let phase_centers = bucket_phase_center_atom_ids(&self.checkpoint.buckets[index]);
            let anti_centers = self.anti_center_atom_ids(index);
            let threshold = learned_wave_margin_micro(
                &self.checkpoint.buckets[index],
                &phase_centers,
                &anti_centers,
            );
            let routed_receipt = receipt_with_program_atoms(
                observation,
                true,
                &self.checkpoint.buckets[index].programs,
            )?;
            if !self.checkpoint.buckets[index]
                .common_request_atom_ids
                .iter()
                .all(|atom| routed_receipt.request_atom_ids.binary_search(atom).is_ok())
            {
                continue;
            }
            if routed_receipt.request_atom_ids.iter().any(|atom| {
                self.checkpoint.buckets[index]
                    .learned_anti_atom_ids
                    .contains(atom)
            }) {
                continue;
            }
            if !receipt_routes_phase(&routed_receipt, &phase_centers, &anti_centers, threshold) {
                continue;
            }
            let bucket = &mut self.checkpoint.buckets[index];
            let Some(program) = bucket.programs.get(&program_sha256) else {
                return Err("online_collection_frozen_program_missing".to_owned());
            };
            let execution = execute_response(program, "", &observation.example.provider_payload);
            if execution.status != ResponseExecutionStatus::Executed {
                continue;
            }
            let verifier_pass =
                response_program_exactly_matches_example(program, &observation.example)
                    && execution.response.as_deref().is_some_and(|response| {
                        source_neutral_verifier_for_program(program).is_ok_and(|verifier| {
                            verify_response_independently(
                                &verifier,
                                &observation.example.provider_payload,
                                response,
                            )
                            .is_ok()
                        })
                    });
            if !verifier_pass {
                let support_atoms = bucket
                    .support
                    .iter()
                    .flat_map(|receipt| receipt.request_atom_ids.iter().copied())
                    .collect::<BTreeSet<_>>();
                let learned = routed_receipt
                    .request_atom_ids
                    .iter()
                    .copied()
                    .filter(|atom| !support_atoms.contains(atom))
                    .take(32)
                    .collect::<Vec<_>>();
                bucket.learned_anti_atom_ids.extend(learned.iter().copied());
                if !learned.is_empty() {
                    self.checkpoint.counterexamples_total =
                        self.checkpoint.counterexamples_total.saturating_add(1);
                    continue;
                }
                pending_subcenters.extend(counterexample_subcenters(
                    bucket,
                    &program_sha256,
                    &routed_receipt,
                )?);
                bucket.wrong_accepts = bucket.wrong_accepts.saturating_add(1);
                bucket.frozen_program_sha256 = None;
                bucket.support_watermark_event_time_unix_nanos = None;
                bucket.support_manifest_sha256 = None;
                bucket.programs.remove(&program_sha256);
                bucket.rejected_program_sha256.insert(program_sha256);
                self.checkpoint.counterexamples_total =
                    self.checkpoint.counterexamples_total.saturating_add(1);
                self.checkpoint.revoked_candidates_total =
                    self.checkpoint.revoked_candidates_total.saturating_add(1);
            } else {
                let support_intents = bucket
                    .support
                    .iter()
                    .map(|receipt| receipt.client_intent_id_sha256.as_str())
                    .collect::<BTreeSet<_>>();
                let after_watermark = observation.event_time_unix_nanos.is_some_and(|event_time| {
                    bucket
                        .support_watermark_event_time_unix_nanos
                        .is_some_and(|watermark| event_time > watermark)
                });
                let intent_is_new =
                    !support_intents.contains(observation.client_intent_id_sha256.as_str());
                if after_watermark && intent_is_new {
                    push_bounded(
                        &mut bucket.future,
                        routed_receipt,
                        self.checkpoint.config.max_receipts_per_bucket,
                    );
                    insert_runtime_example(
                        bucket,
                        observation,
                        self.checkpoint
                            .config
                            .future_rows
                            .min(self.checkpoint.config.max_receipts_per_bucket),
                    );
                } else if !after_watermark {
                    late_after_freeze = late_after_freeze.saturating_add(1);
                } else {
                    future_intent_rejected = future_intent_rejected.saturating_add(1);
                }
                verified_match = true;
            }
        }
        let available = self
            .checkpoint
            .config
            .max_buckets
            .saturating_sub(self.checkpoint.buckets.len());
        pending_subcenters.truncate(available);
        for subcenter in pending_subcenters {
            if self
                .checkpoint
                .buckets
                .iter()
                .any(|bucket| bucket.bucket_id == subcenter.bucket_id)
            {
                continue;
            }
            self.checkpoint.buckets.push(subcenter);
            self.checkpoint.cegis_subcenters_total =
                self.checkpoint.cegis_subcenters_total.saturating_add(1);
        }
        self.checkpoint.late_after_freeze_total = self
            .checkpoint
            .late_after_freeze_total
            .saturating_add(late_after_freeze);
        self.checkpoint.future_intent_rejected_total = self
            .checkpoint
            .future_intent_rejected_total
            .saturating_add(future_intent_rejected);
        Ok(verified_match)
    }

    fn maybe_freeze(&mut self, index: usize) -> Result<(), String> {
        let Some(bucket) = self.checkpoint.buckets.get_mut(index) else {
            return Ok(());
        };
        if bucket.support.len() >= self.checkpoint.config.support_rows
            && bucket.programs.len() == 1
            && !bucket_program_atom_ids(bucket).is_empty()
            && bucket
                .support
                .iter()
                .all(|receipt| receipt.event_time_unix_nanos.is_some())
        {
            bucket.frozen_program_sha256 = bucket.programs.keys().next().cloned();
            bucket.support_watermark_event_time_unix_nanos = bucket
                .support
                .iter()
                .filter_map(|receipt| receipt.event_time_unix_nanos)
                .max();
            bucket.support_manifest_sha256 = Some(collection_support_manifest_digest(bucket)?);
            bucket.runtime_examples.clear();
        }
        Ok(())
    }

    fn normalize_bucket_receipts(&mut self, index: usize) {
        let Some(bucket) = self.checkpoint.buckets.get_mut(index) else {
            return;
        };
        let atoms = bucket_program_atom_ids(bucket);
        for receipt in bucket.support.iter_mut().chain(bucket.future.iter_mut()) {
            receipt.request_atom_ids.extend(atoms.iter().copied());
            receipt.request_atom_ids.sort_unstable();
            receipt.request_atom_ids.dedup();
        }
    }

    fn persist(&self) -> Result<(), String> {
        let payload = serde_cbor::to_vec(&self.checkpoint)
            .map_err(|error| format!("online_collection_checkpoint_encode:{error}"))?;
        let mut bytes = Vec::with_capacity(
            ONLINE_COLLECTION_CHECKPOINT_MAGIC_V3
                .len()
                .saturating_add(payload.len()),
        );
        bytes.extend_from_slice(ONLINE_COLLECTION_CHECKPOINT_MAGIC_V3);
        bytes.extend_from_slice(&payload);
        let temporary = self.path.with_extension("tmp");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|error| {
                format!(
                    "online_collection_checkpoint_create:{}:{error}",
                    temporary.display()
                )
            })?;
        file.write_all(&bytes)
            .map_err(|error| format!("online_collection_checkpoint_write:{error}"))?;
        file.sync_data()
            .map_err(|error| format!("online_collection_checkpoint_sync:{error}"))?;
        fs::rename(&temporary, &self.path)
            .map_err(|error| format!("online_collection_checkpoint_rename:{error}"))?;
        sync_parent(&self.path)
    }
}

fn decode_collection_checkpoint(bytes: &[u8]) -> Result<OnlineCollectionCheckpoint, String> {
    if let Some(payload) = bytes.strip_prefix(ONLINE_COLLECTION_CHECKPOINT_MAGIC_V3) {
        return serde_cbor::from_slice(payload)
            .map_err(|error| format!("online_collection_checkpoint_decode_cbor:{error}"));
    }
    if let Some(payload) = bytes.strip_prefix(ONLINE_COLLECTION_CHECKPOINT_MAGIC_V2) {
        return serde_cbor::from_slice(payload)
            .map_err(|error| format!("online_collection_checkpoint_decode_cbor:{error}"));
    }
    serde_json::from_slice(bytes)
        .map_err(|error| format!("online_collection_checkpoint_decode_legacy_json:{error}"))
}

fn migrate_collection_program_pools(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<(), String> {
    if checkpoint.schema != ONLINE_COLLECTION_SCHEMA_V1
        && checkpoint.schema != ONLINE_COLLECTION_SCHEMA_V2
        && checkpoint.schema != ONLINE_COLLECTION_SCHEMA_V3
    {
        return Err("online_collection_checkpoint_schema_unknown".to_owned());
    }
    let legacy_observations = checkpoint.observations_total;
    let legacy_buckets = checkpoint.buckets.len() as u64;
    let legacy_receipts = checkpoint
        .buckets
        .iter()
        .map(|bucket| bucket.support.len().saturating_add(bucket.future.len()) as u64)
        .sum::<u64>();

    // V1 accepted component matches. Those receipts cannot prove an exact CPU
    // response after the raw example has been intentionally discarded.
    checkpoint.schema = ONLINE_COLLECTION_SCHEMA_V3.to_owned();
    checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V3;
    checkpoint.observations_total = 0;
    checkpoint.duplicate_observations_total = 0;
    checkpoint.observed_evidence_graph_sha256.clear();
    checkpoint.unsupported_total = 0;
    checkpoint.ambiguous_assignment_total = 0;
    checkpoint.exact_checks_total = 0;
    checkpoint.candidates_enumerated_total = 0;
    checkpoint.full_enumerations_total = 0;
    checkpoint.version_space_intersection_checks_total = 0;
    checkpoint.guard_scheduled_buckets_total = 0;
    checkpoint.guard_pruned_buckets_total = 0;
    checkpoint.unsupported_expected_in_latest_output = 0;
    checkpoint.unsupported_expected_in_any_output = 0;
    checkpoint.unsupported_without_exact_source_span = 0;
    checkpoint.unsupported_with_scalar_overlap = 0;
    checkpoint.policy_rejected_exact_matches = 0;
    checkpoint.counterexamples_total = 0;
    checkpoint.cegis_subcenters_total = 0;
    checkpoint.revoked_candidates_total = 0;
    checkpoint.late_after_freeze_total = 0;
    checkpoint.future_intent_rejected_total = 0;
    checkpoint.exact_executable_observations_total = 0;
    checkpoint.program_pool_reuse_total = 0;
    checkpoint.program_pool_receipts_total = 0;
    checkpoint.legacy_partial_observations_discarded_total = checkpoint
        .legacy_partial_observations_discarded_total
        .saturating_add(legacy_observations);
    checkpoint.legacy_partial_buckets_discarded_total = checkpoint
        .legacy_partial_buckets_discarded_total
        .saturating_add(legacy_buckets);
    checkpoint.legacy_partial_receipts_discarded_total = checkpoint
        .legacy_partial_receipts_discarded_total
        .saturating_add(legacy_receipts);
    checkpoint.buckets.clear();
    Ok(())
}

fn insert_runtime_example(
    bucket: &mut OnlineCollectionBucket,
    observation: &OnlineCollectionObservation,
    limit: usize,
) {
    bucket.runtime_examples.insert(
        observation.evidence_graph_sha256.clone(),
        observation.example.clone(),
    );
    while bucket.runtime_examples.len() > limit {
        let Some(oldest) = bucket.runtime_examples.keys().next().cloned() else {
            break;
        };
        bucket.runtime_examples.remove(&oldest);
    }
    while persisted_runtime_example_bytes(&bucket.runtime_examples)
        > MAX_PERSISTED_PARITY_BYTES_PER_BUCKET
    {
        let Some(oldest) = bucket.runtime_examples.keys().next().cloned() else {
            break;
        };
        bucket.runtime_examples.remove(&oldest);
    }
}

fn persisted_runtime_example_bytes(
    examples: &BTreeMap<String, CollectionSynthesisExample>,
) -> usize {
    examples
        .iter()
        .map(|(digest, example)| {
            digest
                .len()
                .saturating_add(serde_cbor::to_vec(example).map_or(0, |bytes| bytes.len()))
        })
        .fold(0_usize, usize::saturating_add)
}

enum UnsupportedSourceSpan {
    Latest,
    Earlier,
    Missing,
}

fn unsupported_source_span(example: &CollectionSynthesisExample) -> UnsupportedSourceSpan {
    let outputs = example
        .provider_payload
        .get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| {
            matches!(
                item.get("type").and_then(Value::as_str),
                Some("function_call_output" | "custom_tool_call_output")
            )
        })
        .filter_map(|item| item.get("output").and_then(Value::as_str))
        .collect::<Vec<_>>();
    if outputs
        .last()
        .is_some_and(|output| output.contains(&example.expected_response))
    {
        UnsupportedSourceSpan::Latest
    } else if outputs
        .iter()
        .any(|output| output.contains(&example.expected_response))
    {
        UnsupportedSourceSpan::Earlier
    } else {
        UnsupportedSourceSpan::Missing
    }
}

fn has_scalar_overlap(example: &CollectionSynthesisExample) -> bool {
    example
        .provider_payload
        .get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("output").and_then(Value::as_str))
        .any(|output| {
            let mut scalars = Vec::new();
            if let Ok(value) = serde_json::from_str::<Value>(output) {
                collect_scalar_strings(&value, &mut scalars);
            }
            scalars.extend(
                output
                    .split(|character: char| {
                        character.is_whitespace()
                            || matches!(character, ':' | '=' | ',' | ';' | '[' | ']' | '{' | '}')
                    })
                    .filter(|value| value.len() >= 2 && value.len() <= 128)
                    .map(str::to_owned),
            );
            scalars
                .iter()
                .any(|scalar| example.expected_response.contains(scalar))
        })
}

fn collect_scalar_strings(value: &Value, output: &mut Vec<String>) {
    match value {
        Value::Null => {}
        Value::Bool(value) => output.push(value.to_string()),
        Value::Number(value) => output.push(value.to_string()),
        Value::String(value) if value.len() >= 2 && value.len() <= 128 => {
            output.push(value.clone());
        }
        Value::Array(values) => {
            for value in values {
                collect_scalar_strings(value, output);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                collect_scalar_strings(value, output);
            }
        }
        Value::String(_) => {}
    }
}

fn validate_config(config: OnlineCollectionConfig) -> Result<(), String> {
    if config.support_rows == 0
        || config.future_rows == 0
        || config.max_buckets == 0
        || config.max_receipts_per_bucket < config.support_rows.max(config.future_rows)
    {
        return Err("online_collection_invalid_config".to_owned());
    }
    Ok(())
}

fn counterexample_subcenters(
    bucket: &OnlineCollectionBucket,
    program_sha256: &str,
    negative: &OnlineCollectionReceipt,
) -> Result<Vec<OnlineCollectionBucket>, String> {
    let Some(program) = bucket.programs.get(program_sha256) else {
        return Ok(Vec::new());
    };
    let negative_atoms = negative
        .request_atom_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut frequencies = BTreeMap::<u64, usize>::new();
    for receipt in &bucket.support {
        for atom in &receipt.request_atom_ids {
            if !negative_atoms.contains(atom) {
                *frequencies.entry(*atom).or_default() += 1;
            }
        }
    }
    let mut atoms = frequencies.into_iter().collect::<Vec<_>>();
    atoms.sort_by(|(left_atom, left_rows), (right_atom, right_rows)| {
        right_rows
            .cmp(left_rows)
            .then_with(|| left_atom.cmp(right_atom))
    });
    let mut seen_partitions = BTreeSet::new();
    let mut output = Vec::new();
    for (atom, rows) in atoms {
        if rows < 8 || output.len() >= 4 {
            continue;
        }
        let support = bucket
            .support
            .iter()
            .filter(|receipt| receipt.request_atom_ids.binary_search(&atom).is_ok())
            .cloned()
            .collect::<Vec<_>>();
        let partition = support
            .iter()
            .map(|receipt| receipt.evidence_graph_sha256.as_str())
            .collect::<Vec<_>>();
        let partition_sha256 = canonical_json_sha256(&partition).map_err(str::to_owned)?;
        if !seen_partitions.insert(partition_sha256.clone()) {
            continue;
        }
        let mut common_request_atom_ids = support.first().map_or_else(BTreeSet::new, |receipt| {
            receipt.request_atom_ids.iter().copied().collect()
        });
        for receipt in support.iter().skip(1) {
            common_request_atom_ids
                .retain(|candidate| receipt.request_atom_ids.binary_search(candidate).is_ok());
        }
        if !common_request_atom_ids.contains(&atom) {
            continue;
        }
        let bucket_id = canonical_json_sha256(&(
            "nando.collection-cegis-subcenter.v1",
            program_sha256,
            atom,
            partition_sha256,
        ))
        .map_err(str::to_owned)?;
        let support_ids = support
            .iter()
            .map(|receipt| receipt.evidence_graph_sha256.clone())
            .collect::<BTreeSet<_>>();
        output.push(OnlineCollectionBucket {
            bucket_id,
            programs: BTreeMap::from([(program_sha256.to_owned(), program.clone())]),
            common_request_atom_ids,
            support,
            future: Vec::new(),
            runtime_examples: bucket
                .runtime_examples
                .iter()
                .filter(|(id, _)| support_ids.contains(*id))
                .map(|(id, example)| (id.clone(), example.clone()))
                .collect(),
            frozen_program_sha256: None,
            support_watermark_event_time_unix_nanos: None,
            support_manifest_sha256: None,
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        });
    }
    Ok(output)
}

fn validate_checkpoint(
    checkpoint: &OnlineCollectionCheckpoint,
    config: OnlineCollectionConfig,
) -> Result<(), String> {
    if checkpoint.schema != ONLINE_COLLECTION_SCHEMA_V3
        || checkpoint.pooling_strategy_version != ONLINE_COLLECTION_POOLING_STRATEGY_V3
        || checkpoint.config != config
    {
        return Err("online_collection_checkpoint_contract_mismatch".to_owned());
    }
    if checkpoint
        .observed_evidence_graph_sha256
        .iter()
        .any(|digest| !is_sha256(digest))
        || checkpoint.observed_evidence_graph_sha256.len()
            > usize::try_from(checkpoint.observations_total).unwrap_or(usize::MAX)
    {
        return Err("online_collection_checkpoint_observation_index_invalid".to_owned());
    }
    for bucket in &checkpoint.buckets {
        if (bucket.programs.is_empty()
            && (bucket.rejected_program_sha256.is_empty()
                || bucket.frozen_program_sha256.is_some()))
            || (bucket.frozen_program_sha256.is_some()
                && (bucket.support_watermark_event_time_unix_nanos.is_none()
                    || bucket.support.iter().any(|receipt| {
                        receipt.event_time_unix_nanos.is_none_or(|event_time| {
                            bucket
                                .support_watermark_event_time_unix_nanos
                                .is_some_and(|watermark| event_time > watermark)
                        })
                    })
                    || collection_support_manifest_digest(bucket).ok().as_ref()
                        != bucket.support_manifest_sha256.as_ref()))
            || (bucket.frozen_program_sha256.is_none() && bucket.support_manifest_sha256.is_some())
            || bucket.programs.iter().any(|(digest, program)| {
                canonical_json_sha256(program).ok().as_ref() != Some(digest)
                    || program.validate().is_err()
                    || !is_privacy_safe_online_response_program(program)
            })
        {
            return Err("online_collection_checkpoint_program_invalid".to_owned());
        }
    }
    Ok(())
}

fn validate_observation(observation: &OnlineCollectionObservation) -> Result<(), String> {
    if !is_sha256(&observation.evidence_graph_sha256)
        || !is_sha256(&observation.client_intent_id_sha256)
        || !is_sha256(&observation.session_id_sha256)
    {
        return Err("online_collection_observation_identity_invalid".to_owned());
    }
    Ok(())
}

fn receipt(
    observation: &OnlineCollectionObservation,
    verifier_pass: bool,
) -> Result<OnlineCollectionReceipt, String> {
    Ok(OnlineCollectionReceipt {
        evidence_graph_sha256: observation.evidence_graph_sha256.clone(),
        client_intent_id_sha256: observation.client_intent_id_sha256.clone(),
        session_id_sha256: observation.session_id_sha256.clone(),
        event_time_unix_nanos: observation.event_time_unix_nanos,
        layout_sha256: structural_layout_sha256(&observation.example.provider_payload)?,
        estimated_input_tokens: observation.estimated_input_tokens,
        verifier_pass,
        request_atom_ids: observation_request_atom_ids(observation)
            .into_iter()
            .collect(),
    })
}

fn receipt_with_program_atoms(
    observation: &OnlineCollectionObservation,
    verifier_pass: bool,
    programs: &BTreeMap<String, ResponseProgram>,
) -> Result<OnlineCollectionReceipt, String> {
    let mut value = receipt(observation, verifier_pass)?;
    value
        .request_atom_ids
        .extend(common_program_atom_ids(programs));
    value.request_atom_ids.sort_unstable();
    value.request_atom_ids.dedup();
    Ok(value)
}

fn common_program_atom_ids(programs: &BTreeMap<String, ResponseProgram>) -> BTreeSet<u64> {
    let mut programs = programs.values();
    let Some(first) = programs.next() else {
        return BTreeSet::new();
    };
    let mut common = response_program_required_routing_atom_ids(first)
        .into_iter()
        .collect::<BTreeSet<_>>();
    for program in programs {
        let atoms = response_program_required_routing_atom_ids(program)
            .into_iter()
            .collect::<BTreeSet<_>>();
        common.retain(|atom| atoms.contains(atom));
    }
    common
}

fn bucket_program_atom_ids(bucket: &OnlineCollectionBucket) -> BTreeSet<u64> {
    common_program_atom_ids(&bucket.programs)
}

fn bucket_phase_center_atom_ids(bucket: &OnlineCollectionBucket) -> Vec<u64> {
    const MAX_LEARNED_REQUEST_ATOMS: usize = 32;
    let program_atoms = bucket_program_atom_ids(bucket);
    let minimum_frequency = bucket.support.len().saturating_div(8).max(2);
    let mut frequencies = BTreeMap::<u64, usize>::new();
    for receipt in &bucket.support {
        for atom in &receipt.request_atom_ids {
            if !program_atoms.contains(atom) {
                *frequencies.entry(*atom).or_default() += 1;
            }
        }
    }
    let mut learned = frequencies
        .into_iter()
        .filter(|(_, frequency)| *frequency >= minimum_frequency)
        .collect::<Vec<_>>();
    learned.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    learned.truncate(MAX_LEARNED_REQUEST_ATOMS);
    let mut atoms = program_atoms.into_iter().collect::<Vec<_>>();
    atoms.extend(learned.into_iter().map(|(atom, _)| atom));
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}

fn distinct_receipt_sessions(receipts: &[OnlineCollectionReceipt]) -> usize {
    receipts
        .iter()
        .map(|receipt| &receipt.session_id_sha256)
        .collect::<BTreeSet<_>>()
        .len()
}

fn distinct_receipt_layouts(receipts: &[OnlineCollectionReceipt]) -> usize {
    receipts
        .iter()
        .map(|receipt| &receipt.layout_sha256)
        .collect::<BTreeSet<_>>()
        .len()
}

fn learned_wave_margin_micro(
    bucket: &OnlineCollectionBucket,
    phase_centers: &[u64],
    anti_centers: &[u64],
) -> i64 {
    let positive = phase_vector_from_atom_ids(phase_centers.iter().copied(), 16);
    let negative = phase_vector_from_atom_ids(anti_centers.iter().copied(), 16);
    bucket
        .support
        .iter()
        .filter_map(|receipt| {
            let query = phase_vector_from_atom_ids(receipt.request_atom_ids.iter().copied(), 16);
            phase_margin_to_micro(
                phase_coherence(&query, &positive) - phase_coherence(&query, &negative),
            )
            .ok()
        })
        .min()
        .map(|minimum| minimum.saturating_mul(9).saturating_div(10).max(1))
        .unwrap_or(1)
}

fn receipt_routes_phase(
    receipt: &OnlineCollectionReceipt,
    phase_centers: &[u64],
    anti_centers: &[u64],
    threshold: i64,
) -> bool {
    let query = phase_vector_from_atom_ids(receipt.request_atom_ids.iter().copied(), 16);
    let positive = phase_vector_from_atom_ids(phase_centers.iter().copied(), 16);
    let negative = phase_vector_from_atom_ids(anti_centers.iter().copied(), 16);
    phase_margin_to_micro(phase_coherence(&query, &positive) - phase_coherence(&query, &negative))
        .is_ok_and(|margin| margin >= threshold)
}

fn structural_layout_sha256(value: &Value) -> Result<String, String> {
    canonical_json_sha256(&structural_layout(value)).map_err(str::to_owned)
}

fn collection_support_manifest_digest(bucket: &OnlineCollectionBucket) -> Result<String, String> {
    let program_sha256 = bucket
        .frozen_program_sha256
        .as_deref()
        .ok_or_else(|| "online_collection_support_program_missing".to_owned())?;
    let watermark_event_time_unix_nanos = bucket
        .support_watermark_event_time_unix_nanos
        .ok_or_else(|| "online_collection_support_watermark_missing".to_owned())?;
    canonical_json_sha256(&CollectionSupportManifestMaterial {
        schema: "nando.collection-support-manifest.v1",
        bucket_id: &bucket.bucket_id,
        program_sha256,
        watermark_event_time_unix_nanos,
        receipts: &bucket.support,
    })
    .map_err(str::to_owned)
}

fn collection_future_manifest_digest(bucket: &OnlineCollectionBucket) -> Result<String, String> {
    let support_manifest_sha256 = bucket
        .support_manifest_sha256
        .as_deref()
        .ok_or_else(|| "online_collection_support_manifest_missing".to_owned())?;
    canonical_json_sha256(&CollectionFutureManifestMaterial {
        schema: "nando.collection-future-manifest.v1",
        support_manifest_sha256,
        receipts: &bucket.future,
    })
    .map_err(str::to_owned)
}

pub fn online_collection_support_manifest_digest(
    candidate: &OnlineCollectionAdmissionCandidate,
) -> Result<String, String> {
    canonical_json_sha256(&CollectionSupportManifestMaterial {
        schema: "nando.collection-support-manifest.v1",
        bucket_id: &candidate.bucket_id,
        program_sha256: &candidate.program_sha256,
        watermark_event_time_unix_nanos: candidate.support_watermark_event_time_unix_nanos,
        receipts: &candidate.support_receipts,
    })
    .map_err(str::to_owned)
}

pub fn online_collection_future_manifest_digest(
    candidate: &OnlineCollectionAdmissionCandidate,
) -> Result<String, String> {
    canonical_json_sha256(&CollectionFutureManifestMaterial {
        schema: "nando.collection-future-manifest.v1",
        support_manifest_sha256: &candidate.support_manifest_sha256,
        receipts: &candidate.future_receipts,
    })
    .map_err(str::to_owned)
}

fn observation_request_atom_ids(observation: &OnlineCollectionObservation) -> BTreeSet<u64> {
    let mut atoms: BTreeSet<u64> = observation
        .example
        .provider_payload
        .get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .rev()
        .find(|item| item.get("role").and_then(Value::as_str) == Some("user"))
        .and_then(|item| item.get("content"))
        .and_then(request_content_text)
        .map(|text| request_phase_atom_ids(&text).into_iter().collect())
        .unwrap_or_default();
    atoms.extend(response_pre_action_context_atom_ids(
        &observation.example.provider_payload,
    ));
    atoms
}

fn request_content_text(content: &Value) -> Option<String> {
    if let Some(text) = content.as_str() {
        return (!text.is_empty()).then(|| text.to_owned());
    }
    let text = content
        .as_array()?
        .iter()
        .filter(|part| {
            matches!(
                part.get("type").and_then(Value::as_str),
                Some("text" | "input_text" | "output_text")
            )
        })
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("");
    (!text.is_empty()).then_some(text)
}

fn structural_layout(value: &Value) -> Value {
    match value {
        Value::Null => Value::String("null".to_owned()),
        Value::Bool(_) => Value::String("bool".to_owned()),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            Value::String("integer".to_owned())
        }
        Value::Number(_) => Value::String("number".to_owned()),
        Value::String(value) => serde_json::from_str::<Value>(value)
            .ok()
            .filter(|parsed| !matches!(parsed, Value::String(_)))
            .map_or_else(
                || Value::String("string".to_owned()),
                |parsed| structural_layout(&parsed),
            ),
        Value::Array(values) => Value::Array(values.iter().map(structural_layout).collect()),
        Value::Object(values) => {
            let mut shapes = values.values().map(structural_layout).collect::<Vec<_>>();
            shapes.sort_by_cached_key(|shape| serde_json::to_vec(shape).unwrap_or_default());
            Value::Array(shapes)
        }
    }
}

fn bucket_status(bucket: &OnlineCollectionBucket) -> OnlineCollectionBucketStatus {
    let future_sessions = distinct_receipt_sessions(&bucket.future);
    let future_layouts = distinct_receipt_layouts(&bucket.future);
    let runtime_parity_cases = bucket
        .future
        .iter()
        .filter(|receipt| {
            bucket
                .runtime_examples
                .contains_key(&receipt.evidence_graph_sha256)
        })
        .count();
    let admission_blocker = if bucket.frozen_program_sha256.is_none() {
        Some("support_or_program_not_frozen".to_owned())
    } else if bucket.future.len() < 32 {
        Some("future_rows_below_32".to_owned())
    } else if future_sessions < 3 {
        Some("future_sessions_below_3".to_owned())
    } else if future_layouts < 2 {
        Some("future_layouts_below_2".to_owned())
    } else if bucket.wrong_accepts > 0 {
        Some("wrong_accepts_nonzero".to_owned())
    } else if runtime_parity_cases < 32 {
        Some("runtime_parity_cases_below_32".to_owned())
    } else {
        None
    };
    OnlineCollectionBucketStatus {
        bucket_id: bucket.bucket_id.clone(),
        version_space_size: bucket.programs.len(),
        support_rows: bucket.support.len(),
        future_rows: bucket.future.len(),
        future_sessions,
        future_layouts,
        wrong_accepts: bucket.wrong_accepts,
        frozen: bucket.frozen_program_sha256.is_some(),
        candidate_program_sha256: bucket.frozen_program_sha256.clone(),
        candidate_program_kind: bucket
            .frozen_program_sha256
            .as_ref()
            .and_then(|digest| bucket.programs.get(digest))
            .map(response_program_kind_code)
            .map(str::to_owned),
        rejected_programs: bucket.rejected_program_sha256.len(),
        learned_anti_atoms: bucket.learned_anti_atom_ids.len(),
        common_request_atoms: bucket.common_request_atom_ids.len(),
        support_tokens: bucket
            .support
            .iter()
            .map(|receipt| receipt.estimated_input_tokens)
            .sum(),
        future_tokens: bucket
            .future
            .iter()
            .map(|receipt| receipt.estimated_input_tokens)
            .sum(),
        support_watermark_event_time_unix_nanos: bucket.support_watermark_event_time_unix_nanos,
        support_manifest_sha256: bucket.support_manifest_sha256.clone(),
        future_manifest_sha256: bucket
            .frozen_program_sha256
            .as_ref()
            .and_then(|_| collection_future_manifest_digest(bucket).ok()),
        runtime_parity_cases,
        admission_blocker,
    }
}

fn response_program_kind_code(program: &ResponseProgram) -> &'static str {
    match response_program_kind(program) {
        AstProgramKind::FunctionCall => "function_call",
        AstProgramKind::CustomToolCall => "custom_tool_call",
        AstProgramKind::Project => "project",
        AstProgramKind::Status => "status",
        AstProgramKind::Collection => "collection",
        AstProgramKind::Legacy => "legacy",
    }
}

fn push_bounded<T>(values: &mut Vec<T>, value: T, max: usize) {
    if values.len() == max {
        values.remove(0);
    }
    values.push(value);
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sync_parent(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("online_collection_checkpoint_parent_sync:{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn observation(index: usize, expected: &str) -> OnlineCollectionObservation {
        let field = format!("surface_{index}");
        let base = i64::try_from(index).unwrap_or_default().saturating_mul(10);
        OnlineCollectionObservation {
            evidence_graph_sha256: format!("{index:064x}"),
            client_intent_id_sha256: format!("{:064x}", index + 10_000),
            session_id_sha256: format!("{:064x}", index % 4 + 20_000),
            event_time_unix_nanos: Some(index as u64),
            estimated_input_tokens: 100,
            example: CollectionSynthesisExample {
                provider_payload: json!({
                    "input":[
                        {"type":"message","role":"user","content":[{
                            "type":"input_text",
                            "text":format!("Count records for batch {index}")
                        }]},
                        {"type":"function_call_output","output":json!({
                            field: [
                                {"value":base + 1},
                                {"value":base + 2},
                                {"value":base + 3}
                            ]
                        }).to_string()}
                    ]
                }),
                expected_response: expected.to_owned(),
            },
        }
    }

    fn routed_observation(
        index: usize,
        expected: &str,
        prompt: &str,
        alternate_layout: bool,
    ) -> OnlineCollectionObservation {
        let mut observation = observation(index, expected);
        observation.example.provider_payload["input"][0]["content"][0]["text"] =
            Value::String(prompt.to_owned());
        if alternate_layout {
            let output = observation.example.provider_payload["input"][1]["output"]
                .as_str()
                .expect("tool output");
            let mut parsed = serde_json::from_str::<Value>(output).expect("tool json");
            parsed["layout_marker"] = Value::Bool(true);
            observation.example.provider_payload["input"][1]["output"] =
                Value::String(parsed.to_string());
        }
        observation
    }

    fn multi_output_observation(index: usize) -> OnlineCollectionObservation {
        let total_field = format!("total_surface_{index}");
        let status_field = format!("status_surface_{index}");
        let total = index.saturating_add(40);
        let status = format!("ready-{index}");
        let total_output = if index.is_multiple_of(2) {
            json!({"wrapper": {(total_field.clone()): total}})
        } else {
            json!({(total_field): total})
        };
        let status_output = if index.is_multiple_of(2) {
            json!({"result": {(status_field.clone()): status}})
        } else {
            json!({(status_field): status})
        };
        OnlineCollectionObservation {
            evidence_graph_sha256: format!("{:064x}", index + 30_000),
            client_intent_id_sha256: format!("{:064x}", index + 40_000),
            session_id_sha256: format!("{:064x}", index % 4 + 50_000),
            event_time_unix_nanos: Some(index as u64),
            estimated_input_tokens: 1_000,
            example: CollectionSynthesisExample {
                provider_payload: json!({
                    "input":[
                        {"type":"message","role":"user","content":[{
                            "type":"input_text","text":"Summarize the verified result"
                        }]},
                        {"type":"function_call_output","call_id":format!("a-{index}"),"output":total_output.to_string()},
                        {"type":"function_call_output","call_id":format!("b-{index}"),"output":status_output.to_string()}
                    ]
                }),
                expected_response: format!("Total: {total}; status: ready-{index}."),
            },
        }
    }

    #[test]
    fn version_space_freezes_then_collects_independent_future_without_raw_examples() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let path = root.join("checkpoint.json");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        };
        let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
        for index in 1..=4 {
            miner.observe(observation(index, "3")).expect("observe");
        }
        let support_only_package_id = miner
            .quarantine_packages()
            .expect("support package")
            .into_iter()
            .next()
            .expect("frozen package")
            .package_id;
        let mut late = observation(9, "3");
        late.event_time_unix_nanos = Some(3);
        miner.observe(late).expect("late after freeze");
        let mut leaked_intent = observation(10, "3");
        leaked_intent.client_intent_id_sha256 = observation(1, "3").client_intent_id_sha256;
        miner
            .observe(leaked_intent)
            .expect("support intent after freeze");
        for index in 5..=8 {
            miner.observe(observation(index, "3")).expect("future");
        }
        miner
            .observe(observation(8, "3"))
            .expect("duplicate observation");
        let status = miner.status();
        assert_eq!(status.observations_total, 10);
        assert_eq!(status.duplicate_observations_total, 1);
        assert_eq!(status.late_after_freeze_total, 1);
        assert_eq!(status.future_intent_rejected_total, 1);
        assert_eq!(status.full_enumerations_total, 2);
        assert_eq!(status.version_space_intersection_checks_total, 2);
        assert_eq!(status.guard_scheduled_buckets_total, 2);
        assert_eq!(status.guard_pruned_buckets_total, 1);
        assert_eq!(status.buckets.len(), 1);
        assert!(status.buckets[0].frozen);
        assert_eq!(status.buckets[0].support_rows, 4);
        assert_eq!(status.buckets[0].future_rows, 4);
        assert_eq!(status.buckets[0].wrong_accepts, 0);
        assert!(status.buckets[0].support_manifest_sha256.is_some());
        assert!(status.buckets[0].future_manifest_sha256.is_some());
        let packages = miner.quarantine_packages().expect("packages");
        assert_eq!(packages.len(), 1);
        assert_ne!(packages[0].package_id, support_only_package_id);
        assert_eq!(packages[0].state, ResponsePackageState::Quarantine);
        assert!(!packages[0].eligible_for_admission_candidate());
        drop(miner);
        let durable = fs::read(&path).expect("checkpoint");
        assert!(durable.starts_with(ONLINE_COLLECTION_CHECKPOINT_MAGIC_V3));
        assert!(
            !durable
                .windows(b"surface_".len())
                .any(|row| row == b"surface_")
        );
        assert!(
            !durable
                .windows(b"provider_payload".len())
                .any(|row| row == b"provider_payload")
        );
        let restored = OnlineCollectionMiner::open(&path, config).expect("restart");
        let mut durable_status = status;
        durable_status.buckets[0].runtime_parity_cases = 0;
        durable_status.runtime_parity_cases_total = 0;
        assert_eq!(restored.status(), durable_status);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn semantic_program_pool_survives_field_renames_and_collects_future() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-outcome-multi-source-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 32,
            max_receipts_per_bucket: 16,
        };
        let mut miner =
            OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
        for index in 1..=4 {
            miner
                .observe(multi_output_observation(index))
                .expect("support");
        }
        for index in 5..=8 {
            miner
                .observe(multi_output_observation(index))
                .expect("future");
        }
        let package = miner
            .quarantine_packages()
            .expect("packages")
            .into_iter()
            .find(|package| {
                matches!(
                    &package.program.operation,
                    crate::ResponseOperation::ProjectSelectedValue {
                        selector: crate::ResponseValueSelector::UniqueTurnScalar { .. },
                        renderer: crate::CollectionOutputRenderer::RenderSequence { .. },
                        ..
                    }
                )
            })
            .expect("portable multi-output package");
        assert_eq!(package.proof.support_rows, 4);
        assert_eq!(package.proof.future_rows, 4);
        assert_eq!(package.proof.wrong_accepts, 0);
        assert!(package.proof.distinct_surfaces >= 2);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn multi_output_semantic_program_reaches_external_admission() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-outcome-admission-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let mut miner = OnlineCollectionMiner::open(
            root.join("checkpoint.cbor"),
            OnlineCollectionConfig::default(),
        )
        .expect("miner");
        for index in 1..=32 {
            miner
                .observe(multi_output_observation(index))
                .expect("support");
        }
        let mut competing = multi_output_observation(10_000);
        competing.example.provider_payload["input"][0]["content"][0]["text"] =
            Value::String("Emit an alternate verified template".to_owned());
        competing.example.expected_response = "Result: 10040; status: ready-10000.".to_owned();
        miner.observe(competing).expect("competing family");
        for index in 33..=64 {
            miner
                .observe(multi_output_observation(index))
                .expect("future");
        }
        let status = miner.status();
        assert_eq!(status.pooling_strategy_version, 3);
        assert!(status.exact_executable_observations_total >= 32);
        assert!(status.program_pool_receipts_total >= 32);
        assert!(status.frozen_buckets_total >= 1);
        assert!(status.pre_admission_ready_buckets_total >= 1);
        assert!(status.support_receipts_unique_total >= 32);
        assert!(status.future_receipts_unique_total >= 32);
        assert_eq!(status.wrong_accepts_total, 0);
        assert!(status.runtime_parity_cases_total >= 32);
        assert!(
            status
                .frozen_program_kinds
                .get("project")
                .copied()
                .unwrap_or(0)
                >= 1
        );
        let candidates = miner.admission_candidates().expect("candidates");
        let causal_reports = miner
            .checkpoint
            .buckets
            .iter()
            .enumerate()
            .filter_map(|(index, bucket)| {
                miner
                    .package_for_bucket(index, bucket, false)
                    .ok()
                    .flatten()
                    .and_then(|package| miner.collection_causal_report(bucket, &package).ok())
            })
            .collect::<Vec<_>>();
        let candidate_blockers = miner
            .checkpoint
            .buckets
            .iter()
            .enumerate()
            .filter_map(|(index, bucket)| {
                let mut package = miner.package_for_bucket(index, bucket, false).ok()??;
                let causal = miner.collection_causal_report(bucket, &package).ok()?;
                package.state = ResponsePackageState::Active;
                package.proof.wave_causal_pass = causal.verdict == "PASS";
                package.wave_margin_micro = causal.wave_margin_micro;
                Some(package.admission_candidate_blocker())
            })
            .collect::<Vec<_>>();
        let candidate = candidates
            .iter()
            .find(|candidate| {
                matches!(
                    &candidate.package.program.operation,
                    crate::ResponseOperation::ProjectSelectedValue {
                        selector: crate::ResponseValueSelector::UniqueTurnScalar { .. },
                        renderer: crate::CollectionOutputRenderer::RenderSequence { .. },
                        ..
                    }
                )
            })
            .unwrap_or_else(|| {
                panic!(
                    "admission-ready multi-output candidate: {:#?}\ncausal={causal_reports:#?}\nblockers={candidate_blockers:#?}",
                    miner.status()
                )
            });
        assert_eq!(candidate.causal_report.verdict, "PASS");
        assert_eq!(candidate.future_receipts.len(), 32);
        assert_eq!(candidate.runtime_parity_cases.len(), 32);
        let snapshot = crate::build_online_collection_admission_snapshot(
            std::slice::from_ref(candidate),
            "project",
            1,
            100,
            60,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("admission")
        .expect("authorized snapshot");
        assert_eq!(snapshot.registry.packages.len(), 1);
        assert!(snapshot.admission.eligible_for_local_accept);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn counterexample_learns_anti_center_then_revokes_only_when_unseparable() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-counterexample-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let path = root.join("checkpoint.json");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        };
        let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
        for index in 1..=4 {
            miner.observe(observation(index, "3")).expect("support");
        }
        miner
            .observe(observation(5, "not-three"))
            .expect("counterexample");
        let status = miner.status();
        assert_eq!(status.counterexamples_total, 1);
        assert_eq!(status.revoked_candidates_total, 0);
        assert_eq!(status.buckets.len(), 1);
        assert!(status.buckets[0].frozen);
        assert_eq!(status.buckets[0].wrong_accepts, 0);
        assert!(status.buckets[0].learned_anti_atoms > 0);

        let mut unseparable = observation(6, "not-three");
        unseparable.example.provider_payload["input"][0]["content"][0]["text"] =
            Value::String("Count records for batch 1".to_owned());
        miner.observe(unseparable).expect("unseparable");
        let status = miner.status();
        assert_eq!(status.revoked_candidates_total, 1);
        assert!(!status.buckets[0].frozen);
        assert_eq!(status.buckets[0].wrong_accepts, 1);
        assert_eq!(status.buckets[0].rejected_programs, 1);
        assert!(miner.quarantine_packages().expect("packages").is_empty());
        drop(miner);
        OnlineCollectionMiner::open(&path, config).expect("restart");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn collection_wave_ablation_builds_external_admission_after_frozen_future() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-admission-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let config = OnlineCollectionConfig::default();
        let mut miner =
            OnlineCollectionMiner::open(root.join("checkpoint.json"), config).expect("miner");
        for index in 1..=32 {
            miner
                .observe(routed_observation(
                    index,
                    "3",
                    "Count records in the verified collection",
                    false,
                ))
                .expect("support");
        }
        miner
            .observe(routed_observation(
                10_000,
                "Rows: 3",
                "Summarize the selected payload",
                false,
            ))
            .expect("competing family");
        for index in 33..=64 {
            miner
                .observe(routed_observation(
                    index,
                    "3",
                    "Count records in the verified collection",
                    index % 2 == 0,
                ))
                .expect("future");
        }
        let candidates = miner.admission_candidates().expect("candidates");
        assert_eq!(candidates.len(), 1, "{:#?}", miner.status());
        let candidate = &candidates[0];
        assert_eq!(candidate.causal_report.verdict, "PASS");
        assert_eq!(candidate.causal_report.full_phase_correct, 32);
        assert!(
            candidate.causal_report.full_phase_exact_checks
                < candidate.causal_report.no_phase_exact_checks
        );
        assert_eq!(candidate.package.state, ResponsePackageState::Active);
        let snapshot = crate::build_online_collection_admission_snapshot(
            &candidates,
            "project",
            1,
            100,
            60,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("admission")
        .expect("authorized snapshot");
        assert_eq!(snapshot.registry.packages.len(), 1);
        assert!(snapshot.admission.eligible_for_local_accept);
        fs::remove_dir_all(root).expect("cleanup");
    }
}
