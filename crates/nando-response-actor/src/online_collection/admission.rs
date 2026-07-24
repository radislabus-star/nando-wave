//! Candidate packaging, frozen-future evaluation, and persistence.
//!
//! This owner emits candidates only; external admission remains authoritative.

use super::*;

impl OnlineCollectionMiner {
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
            let runtime_parity_cases = bucket
                .future
                .iter()
                .filter_map(|receipt| {
                    let example = bucket
                        .runtime_examples
                        .get(&receipt.evidence_graph_sha256)?;
                    let canonical_response =
                        independently_verified_authority_response(&package.program, example)?;
                    Some(crate::RuntimeParityCase {
                        evidence_ref_sha256: receipt.evidence_graph_sha256.clone(),
                        capture_receipt: None,
                        request_text: String::new(),
                        provider_payload: example.provider_payload.clone(),
                        expected_response: canonical_response,
                    })
                })
                .collect();
            let durable_runtime_parity_receipts = bucket
                .future
                .iter()
                .filter_map(|receipt| {
                    bucket
                        .durable_runtime_parity_receipts
                        .get(&receipt.evidence_graph_sha256)
                        .cloned()
                })
                .collect();
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
                runtime_parity_cases,
                durable_runtime_parity_receipts,
                archetype_id: bucket.archetype_id.clone(),
                identification_programs: bucket.programs.values().cloned().collect(),
                candidate_freeze: bucket.adaptive_candidate_freeze.clone(),
            });
        }
        candidates.sort_by(|left, right| left.package.package_id.cmp(&right.package.package_id));
        Ok(candidates)
    }

    pub(super) fn package_for_bucket(
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
        let required_routing_atom_ids = response_program_required_routing_atom_ids(&program);
        let phase_centers = bucket_phase_center_atom_ids(bucket);
        let anti_centers = self.anti_center_atom_ids(index);
        let route_sha256 =
            canonical_json_sha256(&(&required_routing_atom_ids, &phase_centers, &anti_centers))
                .map_err(str::to_owned)?;
        let wave_margin_micro = learned_wave_margin_micro(bucket, &phase_centers, &anti_centers);
        let adaptive_identification =
            bucket
                .adaptive_candidate_freeze
                .as_ref()
                .and_then(|freeze| {
                    let transfer_root = adaptive_transfer_proof_root(
                        &future_manifest_sha256,
                        program_sha256,
                        &program,
                        &bucket.support,
                        &bucket.future,
                        &bucket
                            .future
                            .iter()
                            .filter_map(|receipt| {
                                bucket
                                    .durable_runtime_parity_receipts
                                    .get(&receipt.evidence_graph_sha256)
                                    .cloned()
                            })
                            .collect::<Vec<_>>(),
                    )
                    .ok()?;
                    nando_operator_admission::seal_adaptive_identification_proof_v1(
                        nando_operator_admission::AdaptiveIdentificationProofInputV1 {
                            candidate_freeze_root_sha256: freeze.freeze_root_sha256().to_owned(),
                            semantic_class_id_sha256: freeze
                                .semantic_class_id()
                                .as_str()
                                .to_owned(),
                            canonical_program_root_sha256: freeze
                                .canonical_program_root_sha256()
                                .to_owned(),
                            applicability_scope_root_sha256: freeze
                                .applicability_scope_root_sha256()
                                .to_owned(),
                            transfer_proof_root_sha256: transfer_root,
                        },
                    )
                    .ok()
                });
        let distinct_sessions = bucket
            .support
            .iter()
            .chain(&bucket.future)
            .map(|receipt| receipt.session_id_sha256.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        let distinct_surfaces = bucket
            .support
            .iter()
            .chain(&bucket.future)
            .map(|receipt| receipt.evidence_graph_sha256.as_str())
            .collect::<BTreeSet<_>>()
            .len();
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
            crystallized_operator: None,
            proof: ResponsePackageProof {
                support_rows: bucket.support.len(),
                future_rows: bucket.future.len(),
                distinct_sessions,
                distinct_surfaces,
                wrong_accepts: bucket.wrong_accepts,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass,
                verifier_schema: verifier_schema.to_owned(),
                adaptive_identification,
            },
        };
        package.validate().map_err(str::to_owned)?;
        Ok(Some(package))
    }

    pub(super) fn anti_center_atom_ids(&self, index: usize) -> Vec<u64> {
        self.checkpoint.buckets[index]
            .learned_anti_atom_ids
            .iter()
            .copied()
            .take(32)
            .collect()
    }

    pub(super) fn learn_applicability_anti_atoms(
        &mut self,
        index: usize,
        negative: &OnlineCollectionReceipt,
    ) {
        let Some(bucket) = self.checkpoint.buckets.get(index) else {
            return;
        };
        let support_atoms = bucket
            .support
            .iter()
            .flat_map(|receipt| receipt.request_atom_ids.iter().copied())
            .collect::<BTreeSet<_>>();
        let candidates = negative
            .request_atom_ids
            .iter()
            .copied()
            .filter(|atom| !support_atoms.contains(atom))
            .collect::<BTreeSet<_>>();
        if candidates.is_empty() {
            return;
        }
        let bucket_id = bucket.bucket_id.clone();
        let evidence = self
            .checkpoint
            .applicability_negative_sessions
            .entry(bucket_id)
            .or_default();
        let learned = update_applicability_negative_sessions(
            evidence,
            candidates,
            &negative.session_id_sha256,
        );
        if let Some(bucket) = self.checkpoint.buckets.get_mut(index) {
            bucket.learned_anti_atom_ids.extend(learned);
        }
    }

    pub(super) fn collection_causal_report(
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
        let adaptive = bucket.adaptive_candidate_freeze.is_some();
        let distinct_sessions = bucket
            .support
            .iter()
            .chain(&bucket.future)
            .map(|receipt| receipt.session_id_sha256.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        let distinct_surfaces = bucket
            .support
            .iter()
            .chain(&bucket.future)
            .map(|receipt| receipt.evidence_graph_sha256.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        let evidence_gate = if adaptive {
            !bucket.support.is_empty()
                && !bucket.future.is_empty()
                && distinct_sessions >= 2
                && distinct_surfaces >= 2
        } else {
            bucket.support.len() >= 32
                && bucket.future.len() >= 32
                && distinct_receipt_sessions(&bucket.future) >= 3
                && distinct_receipt_layouts(&bucket.future) >= 2
        };
        let pass = evidence_gate
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

    pub(super) fn create_bucket(
        &mut self,
        programs: BTreeMap<String, ResponseProgram>,
        observation: &OnlineCollectionObservation,
        verifier_pass: bool,
    ) -> Result<(), String> {
        if programs.is_empty() {
            return Err("online_collection_empty_program_pool".to_owned());
        }
        if self.checkpoint.buckets.len() >= self.checkpoint.config.max_buckets {
            self.checkpoint.unsupported_total = self.checkpoint.unsupported_total.saturating_add(1);
            return Ok(());
        }
        let archetypes = programs
            .values()
            .map(response_program_archetype_id)
            .collect::<Result<BTreeSet<_>, _>>()?;
        if archetypes.len() != 1 {
            return Err("online_collection_mixed_archetype_bucket".to_owned());
        }
        let archetype_id = archetypes
            .into_iter()
            .next()
            .ok_or_else(|| "online_collection_archetype_missing".to_owned())?;
        let request_atoms = observation_request_atom_ids(observation);
        let program_digests = programs.keys().cloned().collect::<Vec<_>>();
        let bucket_id = collection_archetype_bucket_id(&archetype_id, &program_digests)?;
        if self
            .checkpoint
            .buckets
            .iter()
            .any(|bucket| bucket.bucket_id == bucket_id)
        {
            return Ok(());
        }
        let support = vec![receipt_with_program_atoms(
            observation,
            verifier_pass,
            &programs,
        )?];
        self.checkpoint.program_pool_receipts_total = self
            .checkpoint
            .program_pool_receipts_total
            .saturating_add(1);
        self.checkpoint.buckets.push(OnlineCollectionBucket {
            bucket_id,
            archetype_id,
            programs,
            common_request_atom_ids: request_atoms,
            support,
            future: Vec::new(),
            runtime_examples: BTreeMap::from([(
                observation.evidence_graph_sha256.clone(),
                observation.example.clone(),
            )]),
            durable_adapter_phase_atoms: BTreeMap::new(),
            durable_runtime_parity_receipts: BTreeMap::new(),
            adaptive_candidate_freeze: None,
            frozen_program_sha256: None,
            support_watermark_event_time_unix_nanos: None,
            support_manifest_sha256: None,
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        });
        if let Some(bucket) = self.checkpoint.buckets.last_mut() {
            refresh_durable_adapter_phase_atoms(bucket);
        }
        if self.checkpoint.config.proof_mode == OnlineCollectionProofMode::AdaptiveVersionSpace {
            self.maybe_freeze(self.checkpoint.buckets.len().saturating_sub(1))?;
        }
        Ok(())
    }

    pub(super) fn assign_archetype_programs(
        &mut self,
        programs: BTreeMap<String, ResponseProgram>,
        observation: &OnlineCollectionObservation,
        verifier_pass: bool,
        count_observation: bool,
    ) -> Result<(), String> {
        let groups = group_programs_by_archetype(programs)?;
        if count_observation && groups.len() > 1 {
            self.checkpoint.ambiguous_assignment_total =
                self.checkpoint.ambiguous_assignment_total.saturating_add(1);
        }
        let mut proof_refresh_used = false;
        for (archetype_id, programs) in groups {
            let target = self
                .checkpoint
                .buckets
                .iter()
                .enumerate()
                .filter(|(_, bucket)| bucket.frozen_program_sha256.is_none())
                .filter(|(_, bucket)| bucket.archetype_id == archetype_id)
                .filter(|(_, bucket)| {
                    let additional = programs
                        .keys()
                        .filter(|digest| !bucket.programs.contains_key(*digest))
                        .count();
                    bucket.programs.len().saturating_add(additional)
                        <= crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS
                })
                .max_by_key(|(_, bucket)| bucket.support.len())
                .map(|(index, _)| index);
            if let Some(index) = target {
                self.update_bucket(
                    index,
                    &programs,
                    observation,
                    verifier_pass,
                    !proof_refresh_used,
                )?;
                proof_refresh_used = true;
            } else {
                self.create_bucket(programs, observation, verifier_pass)?;
            }
        }
        Ok(())
    }

    pub(super) fn update_bucket(
        &mut self,
        index: usize,
        matching_programs: &BTreeMap<String, ResponseProgram>,
        observation: &OnlineCollectionObservation,
        verifier_pass: bool,
        refresh_proof: bool,
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
        for (digest, program) in matching_programs {
            bucket
                .programs
                .entry(digest.clone())
                .or_insert_with(|| program.clone());
        }
        let request_atoms = observation_request_atom_ids(observation);
        bucket
            .common_request_atom_ids
            .retain(|atom| request_atoms.contains(atom));
        if bucket.programs.is_empty() {
            return Err("online_collection_version_space_became_empty".to_owned());
        }
        merge_receipts(
            &mut bucket.support,
            vec![receipt_with_program_atoms(
                observation,
                verifier_pass,
                matching_programs,
            )?],
            self.checkpoint.config.max_receipts_per_bucket,
        );
        insert_runtime_example(
            bucket,
            observation,
            self.checkpoint.config.max_receipts_per_bucket,
        );
        refresh_durable_adapter_phase_atoms(bucket);
        self.normalize_bucket_receipts(index);
        let support_rows = self.checkpoint.buckets[index].support.len();
        if refresh_proof
            && (self.checkpoint.config.proof_mode
                == OnlineCollectionProofMode::AdaptiveVersionSpace
                || support_rows >= self.checkpoint.config.support_rows)
        {
            self.freeze_or_split(index)?;
        }
        Ok(())
    }

    pub(super) fn matching_unfrozen_buckets(
        &mut self,
        observation: &OnlineCollectionObservation,
    ) -> Result<Vec<(usize, BTreeSet<String>)>, String> {
        let request_atoms = observation_request_atom_ids(observation);
        let query = phase_vector_from_atom_ids(request_atoms.iter().copied(), 16);
        let total_unfrozen = self
            .checkpoint
            .buckets
            .iter()
            .filter(|bucket| bucket.frozen_program_sha256.is_none())
            .count();
        let mut ranked_buckets = self
            .checkpoint
            .buckets
            .iter()
            .enumerate()
            .filter(|(_, bucket)| bucket.frozen_program_sha256.is_none())
            .filter(|(_, bucket)| {
                !bucket
                    .learned_anti_atom_ids
                    .iter()
                    .any(|atom| request_atoms.contains(atom))
            })
            .map(|(index, bucket)| {
                let phase_centers = bucket_phase_center_atom_ids(bucket);
                let anti_centers = bucket
                    .learned_anti_atom_ids
                    .iter()
                    .copied()
                    .take(32)
                    .collect::<Vec<_>>();
                let positive = phase_vector_from_atom_ids(phase_centers.iter().copied(), 16);
                let negative = phase_vector_from_atom_ids(anti_centers.iter().copied(), 16);
                let margin = phase_margin_to_micro(
                    phase_coherence(&query, &positive) - phase_coherence(&query, &negative),
                )
                .unwrap_or(i64::MIN);
                let threshold = learned_wave_margin_micro(
                    bucket,
                    phase_centers.as_slice(),
                    anti_centers.as_slice(),
                );
                let common_match = bucket
                    .common_request_atom_ids
                    .iter()
                    .all(|atom| request_atoms.contains(atom));
                let overlap = phase_centers
                    .iter()
                    .filter(|atom| request_atoms.contains(atom))
                    .count();
                (
                    index,
                    margin >= threshold,
                    common_match,
                    margin,
                    overlap,
                    bucket.support.len(),
                    bucket.bucket_id.clone(),
                )
            })
            .collect::<Vec<_>>();
        ranked_buckets.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| right.2.cmp(&left.2))
                .then_with(|| right.3.cmp(&left.3))
                .then_with(|| right.4.cmp(&left.4))
                .then_with(|| right.5.cmp(&left.5))
                .then_with(|| left.6.cmp(&right.6))
        });
        ranked_buckets.truncate(MAX_UNFROZEN_ROUTE_BUCKETS);

        let mut checks = 0_u64;
        let scheduled = ranked_buckets.len() as u64;
        let mut matching = Vec::new();
        for (index, ..) in ranked_buckets {
            let bucket = &self.checkpoint.buckets[index];
            let mut ranked_programs = bucket
                .programs
                .iter()
                .map(|(digest, program)| {
                    let support_matches = bucket
                        .support
                        .iter()
                        .filter(|receipt| receipt.matched_program_sha256.contains(digest))
                        .count();
                    let routing_overlap = response_program_required_routing_atom_ids(program)
                        .iter()
                        .filter(|atom| request_atoms.contains(atom))
                        .count();
                    (digest, program, support_matches, routing_overlap)
                })
                .collect::<Vec<_>>();
            ranked_programs.sort_by(|left, right| {
                right
                    .2
                    .cmp(&left.2)
                    .then_with(|| right.3.cmp(&left.3))
                    .then_with(|| left.0.cmp(right.0))
            });
            let mut matching_programs = BTreeSet::new();
            for (digest, program, _, _) in ranked_programs
                .into_iter()
                .take(MAX_UNFROZEN_ROUTE_PROGRAMS)
            {
                checks = checks.saturating_add(1);
                if independently_verified_authority_response(program, &observation.example)
                    .is_some()
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
            .saturating_add(total_unfrozen.saturating_sub(scheduled as usize) as u64);
        self.checkpoint.exact_checks_total =
            self.checkpoint.exact_checks_total.saturating_add(checks);
        Ok(matching)
    }

    pub(super) fn evaluate_frozen_candidates(
        &mut self,
        observation: &OnlineCollectionObservation,
    ) -> Result<bool, String> {
        let mut verified_match = false;
        let mut verified_exact_teacher_match = false;
        let mut late_after_freeze = 0_u64;
        let mut future_intent_rejected = 0_u64;
        let mut route_candidates_considered = 0_u64;
        let mut route_anti_rejected = 0_u64;
        let mut route_phase_rejected = 0_u64;
        let mut route_verifier_rejected = 0_u64;
        let mut route_rejection_reasons = BTreeMap::<String, u64>::new();
        let mut route_witness_pending = 0_u64;
        let mut route_witness_resolved = 0_u64;
        let mut route_irreducible = 0_u64;
        let mut route_applicability_abstain = 0_u64;
        let mut future_accepted = 0_u64;
        let mut pending_subcenters = Vec::new();
        let mut pending_witness_successors = Vec::new();
        let mut witness_consumed = false;
        for index in 0..self.checkpoint.buckets.len() {
            let Some(program_sha256) = self.checkpoint.buckets[index].frozen_program_sha256.clone()
            else {
                continue;
            };
            route_candidates_considered = route_candidates_considered.saturating_add(1);
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
            if routed_receipt.request_atom_ids.iter().any(|atom| {
                self.checkpoint.buckets[index]
                    .learned_anti_atom_ids
                    .contains(atom)
            }) {
                route_anti_rejected = route_anti_rejected.saturating_add(1);
                continue;
            }
            if !receipt_routes_phase(&routed_receipt, &phase_centers, &anti_centers, threshold) {
                route_phase_rejected = route_phase_rejected.saturating_add(1);
                continue;
            }
            let authority_result = {
                let bucket = &self.checkpoint.buckets[index];
                let Some(program) = bucket.programs.get(&program_sha256) else {
                    return Err("online_collection_frozen_program_missing".to_owned());
                };
                independently_verified_authority_response_result(program, &observation.example)
                    .and_then(|response| {
                        // Actor/verifier agreement is necessary but not enough:
                        // frozen future must reproduce the completed teacher.
                        (response == observation.example.expected_response)
                            .then_some(response)
                            .ok_or("teacher_response_mismatch")
                    })
            };
            let rejection_reason = authority_rejection_reason(&authority_result);
            let verifier_pass = rejection_reason.is_none();
            if !verifier_pass {
                route_verifier_rejected = route_verifier_rejected.saturating_add(1);
                let reason = rejection_reason.unwrap_or("unknown_verifier_rejection");
                *route_rejection_reasons
                    .entry(reason.to_owned())
                    .or_default() += 1;
                let witness_decision = active_witness_decision(
                    &self.checkpoint.buckets[index],
                    &program_sha256,
                    observation,
                    self.checkpoint.config.max_receipts_per_bucket,
                )?;
                match witness_decision {
                    ActiveWitnessDecision::Successor {
                        bucket: successor,
                        resolved,
                    } => {
                        if resolved {
                            route_witness_resolved = route_witness_resolved.saturating_add(1);
                        } else {
                            route_witness_pending = route_witness_pending.saturating_add(1);
                        }
                        pending_witness_successors.push(*successor);
                        witness_consumed = true;
                        self.checkpoint.counterexamples_total =
                            self.checkpoint.counterexamples_total.saturating_add(1);
                        revoke_frozen_bucket(&mut self.checkpoint.buckets[index], &program_sha256);
                        self.checkpoint.revoked_candidates_total =
                            self.checkpoint.revoked_candidates_total.saturating_add(1);
                        continue;
                    }
                    ActiveWitnessDecision::Pending => {
                        route_witness_pending = route_witness_pending.saturating_add(1);
                        continue;
                    }
                    ActiveWitnessDecision::Irreducible => {
                        if !is_hard_teacher_counterexample(reason) {
                            self.learn_applicability_anti_atoms(index, &routed_receipt);
                            route_applicability_abstain =
                                route_applicability_abstain.saturating_add(1);
                            continue;
                        }
                    }
                }
                route_irreducible = route_irreducible.saturating_add(1);
                let bucket = &mut self.checkpoint.buckets[index];
                pending_subcenters.extend(counterexample_subcenters(
                    bucket,
                    &program_sha256,
                    &routed_receipt,
                )?);
                bucket.wrong_accepts = bucket.wrong_accepts.saturating_add(1);
                bucket.adaptive_candidate_freeze = None;
                bucket.frozen_program_sha256 = None;
                bucket.support_watermark_event_time_unix_nanos = None;
                bucket.support_manifest_sha256 = None;
                revoke_frozen_bucket(bucket, &program_sha256);
                self.checkpoint.counterexamples_total =
                    self.checkpoint.counterexamples_total.saturating_add(1);
                self.checkpoint.revoked_candidates_total =
                    self.checkpoint.revoked_candidates_total.saturating_add(1);
            } else {
                let authority_response = authority_result.ok();
                let bucket = &mut self.checkpoint.buckets[index];
                let Some(program) = bucket.programs.get(&program_sha256) else {
                    return Err("online_collection_frozen_program_missing".to_owned());
                };
                verified_exact_teacher_match |= authority_response.as_deref()
                    == Some(observation.example.expected_response.as_str());
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
                    let durable_parity = build_durable_runtime_parity_receipt(
                        program,
                        &observation.evidence_graph_sha256,
                        &observation.example,
                    )
                    .map_err(str::to_owned)?;
                    push_bounded(
                        &mut bucket.future,
                        routed_receipt,
                        self.checkpoint.config.max_receipts_per_bucket,
                    );
                    bucket
                        .durable_runtime_parity_receipts
                        .insert(observation.evidence_graph_sha256.clone(), durable_parity);
                    future_accepted = future_accepted.saturating_add(1);
                    let future_refs = bucket
                        .future
                        .iter()
                        .map(|receipt| receipt.evidence_graph_sha256.as_str())
                        .collect::<BTreeSet<_>>();
                    bucket
                        .durable_runtime_parity_receipts
                        .retain(|evidence_ref, _| future_refs.contains(evidence_ref.as_str()));
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
        pending_witness_successors.append(&mut pending_subcenters);
        pending_witness_successors.truncate(available);
        for subcenter in pending_witness_successors {
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
        self.checkpoint.frozen_route_candidates_considered_total = self
            .checkpoint
            .frozen_route_candidates_considered_total
            .saturating_add(route_candidates_considered);
        self.checkpoint.frozen_route_anti_rejected_total = self
            .checkpoint
            .frozen_route_anti_rejected_total
            .saturating_add(route_anti_rejected);
        self.checkpoint.frozen_route_phase_rejected_total = self
            .checkpoint
            .frozen_route_phase_rejected_total
            .saturating_add(route_phase_rejected);
        self.checkpoint.frozen_route_verifier_rejected_total = self
            .checkpoint
            .frozen_route_verifier_rejected_total
            .saturating_add(route_verifier_rejected);
        for (reason, count) in route_rejection_reasons {
            let total = self
                .checkpoint
                .frozen_route_rejection_reasons
                .entry(reason)
                .or_default();
            *total = total.saturating_add(count);
        }
        self.checkpoint.frozen_route_witness_pending_total = self
            .checkpoint
            .frozen_route_witness_pending_total
            .saturating_add(route_witness_pending);
        self.checkpoint.frozen_route_witness_resolved_total = self
            .checkpoint
            .frozen_route_witness_resolved_total
            .saturating_add(route_witness_resolved);
        self.checkpoint.frozen_route_irreducible_total = self
            .checkpoint
            .frozen_route_irreducible_total
            .saturating_add(route_irreducible);
        self.checkpoint.frozen_route_applicability_abstain_total = self
            .checkpoint
            .frozen_route_applicability_abstain_total
            .saturating_add(route_applicability_abstain);
        self.checkpoint.frozen_future_accepted_total = self
            .checkpoint
            .frozen_future_accepted_total
            .saturating_add(future_accepted);
        if verified_match {
            self.record_executable_observation(verified_exact_teacher_match, true);
        }
        Ok(verified_match || witness_consumed)
    }

    pub(super) fn maybe_freeze(&mut self, index: usize) -> Result<(), String> {
        let adaptive =
            self.checkpoint.config.proof_mode == OnlineCollectionProofMode::AdaptiveVersionSpace;
        let Some(bucket) = self.checkpoint.buckets.get_mut(index) else {
            return Ok(());
        };
        refresh_durable_adapter_phase_atoms(bucket);
        if !adaptive
            && bucket.support.len() >= self.checkpoint.config.support_rows
            && bucket.frozen_program_sha256.is_none()
            && bucket.support.iter().all(|receipt| receipt.verifier_pass)
            && let SupportConsensusCandidate::Ready(candidate) =
                support_consensus_candidate(bucket)?
        {
            let digest = canonical_json_sha256(&candidate).map_err(str::to_owned)?;
            if !candidate_authority_verified_on_support(bucket, &candidate) {
                return Err("online_collection_consensus_support_authority_unproven".to_owned());
            }
            for receipt in &mut bucket.support {
                receipt.matched_program_sha256 = vec![digest.clone()];
            }
            bucket.programs = BTreeMap::from([(digest, candidate)]);
        }
        if adaptive
            && bucket.frozen_program_sha256.is_none()
            && let Some(identification) = identify_collection_bucket(bucket)?
        {
            let program_sha256 = identification.program_sha256;
            let Some(program) = bucket.programs.get(&program_sha256) else {
                return Err("online_collection_adaptive_program_missing".to_owned());
            };
            if !candidate_authority_verified_on_support(bucket, program)
                || response_program_required_routing_atom_ids(program).is_empty()
            {
                return Err("online_collection_adaptive_support_authority_unproven".to_owned());
            }
            bucket.frozen_program_sha256 = Some(program_sha256);
            bucket.support_watermark_event_time_unix_nanos = bucket
                .support
                .iter()
                .filter_map(|receipt| receipt.event_time_unix_nanos)
                .max();
            bucket.adaptive_candidate_freeze = Some(identification.freeze);
            bucket.support_manifest_sha256 = Some(collection_support_manifest_digest(bucket)?);
            bucket.runtime_examples.clear();
            bucket.durable_adapter_phase_atoms.clear();
            return Ok(());
        }
        if !adaptive
            && bucket.support.len() >= self.checkpoint.config.support_rows
            && bucket.programs.len() == 1
            && bucket.support.iter().all(|receipt| receipt.verifier_pass)
            // A singleton version space is still only a hypothesis until its
            // complete teacher response is proven on every support receipt.
            && bucket
                .programs
                .values()
                .next()
                .is_some_and(|program| candidate_authority_verified_on_support(bucket, program))
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
            bucket.adaptive_candidate_freeze = None;
            bucket.runtime_examples.clear();
            bucket.durable_adapter_phase_atoms.clear();
        }
        Ok(())
    }

    pub(super) fn upgrade_legacy_frozen_identification(
        &mut self,
        index: usize,
    ) -> Result<bool, String> {
        let Some(bucket) = self.checkpoint.buckets.get(index) else {
            return Ok(false);
        };
        let Some(frozen_program_sha256) = bucket.frozen_program_sha256.as_deref() else {
            return Ok(false);
        };
        if bucket.adaptive_candidate_freeze.is_some()
            || bucket.support.is_empty()
            || bucket
                .support
                .iter()
                .any(|receipt| !receipt.verifier_pass || receipt.event_time_unix_nanos.is_none())
        {
            return Ok(false);
        }
        let expected_watermark = bucket
            .support
            .iter()
            .filter_map(|receipt| receipt.event_time_unix_nanos)
            .max();
        let expected_manifest = collection_support_manifest_digest(bucket)?;
        if bucket.support_watermark_event_time_unix_nanos != expected_watermark
            || bucket.support_manifest_sha256.as_deref() != Some(expected_manifest.as_str())
        {
            return Ok(false);
        }
        let Some(identification) = identify_collection_bucket(bucket)? else {
            return Ok(false);
        };
        if identification.program_sha256 != frozen_program_sha256 {
            return Ok(false);
        }
        let Some(program) = bucket.programs.get(frozen_program_sha256) else {
            return Ok(false);
        };
        if !candidate_authority_verified_on_support(bucket, program)
            || response_program_required_routing_atom_ids(program).is_empty()
        {
            return Ok(false);
        }

        // This migration seals only the already-proven identification basis.
        // It neither creates future evidence nor changes the frozen program.
        self.checkpoint.buckets[index].adaptive_candidate_freeze = Some(identification.freeze);
        Ok(true)
    }

    pub(super) fn freeze_or_split(&mut self, index: usize) -> Result<(), String> {
        if self.checkpoint.config.proof_mode == OnlineCollectionProofMode::AdaptiveVersionSpace {
            return self.maybe_freeze(index);
        }
        let law_subcenters = self
            .checkpoint
            .buckets
            .get(index)
            .filter(|bucket| {
                bucket.frozen_program_sha256.is_none()
                    && bucket.support.len() >= self.checkpoint.config.support_rows
            })
            .map(|bucket| {
                support_law_subcenters(
                    bucket,
                    self.checkpoint.config.support_rows,
                    self.checkpoint.config.max_receipts_per_bucket,
                )
            })
            .transpose()?
            .unwrap_or_default();
        let preferred = law_subcenters
            .iter()
            .find(|subcenter| {
                matches!(
                    support_consensus_candidate(subcenter),
                    Ok(SupportConsensusCandidate::Ready(_))
                )
            })
            .cloned();
        if let Some(subcenter) = preferred {
            let subcenter_index = if let Some(existing) = self
                .checkpoint
                .buckets
                .iter()
                .position(|candidate| candidate.bucket_id == subcenter.bucket_id)
            {
                if self.checkpoint.buckets[existing]
                    .frozen_program_sha256
                    .is_none()
                {
                    self.checkpoint.buckets[existing] = subcenter;
                }
                existing
            } else {
                if self.checkpoint.buckets.len() >= self.checkpoint.config.max_buckets {
                    return Ok(());
                }
                self.checkpoint.buckets.push(subcenter);
                self.checkpoint.cegis_subcenters_total =
                    self.checkpoint.cegis_subcenters_total.saturating_add(1);
                self.checkpoint.buckets.len().saturating_sub(1)
            };
            self.normalize_bucket_receipts(subcenter_index);
            self.maybe_freeze(subcenter_index)?;
            return Ok(());
        }
        self.maybe_freeze(index)?;
        let Some(bucket) = self.checkpoint.buckets.get(index) else {
            return Ok(());
        };
        let blocker = support_freeze_blocker(bucket, self.checkpoint.config.support_rows);
        if !support_blocker_requires_subcenter_split(blocker.as_deref()) {
            return Ok(());
        }
        let law_subcenters = support_law_subcenters(
            bucket,
            self.checkpoint.config.support_rows,
            self.checkpoint.config.max_receipts_per_bucket,
        )?;
        let mut subcenters = Vec::new();
        for law_subcenter in law_subcenters {
            if let Some(decidable) = maximal_decidable_support_subcenter(
                &law_subcenter,
                self.checkpoint.config.support_rows,
                self.checkpoint.config.max_receipts_per_bucket,
            )? {
                subcenters.push(decidable);
            }
            if let Some(decidable) = clean_pre_action_program_subcenter(
                &law_subcenter,
                self.checkpoint.config.support_rows,
                self.checkpoint.config.max_receipts_per_bucket,
            )? {
                subcenters.push(decidable);
            }
            subcenters.push(law_subcenter);
        }
        subcenters.extend(support_program_subcenters(
            bucket,
            self.checkpoint.config.support_rows,
            self.checkpoint.config.max_receipts_per_bucket,
        )?);
        let mut seen = BTreeSet::new();
        subcenters.retain(|subcenter| seen.insert(subcenter.bucket_id.clone()));
        let available = self
            .checkpoint
            .config
            .max_buckets
            .saturating_sub(self.checkpoint.buckets.len());
        subcenters.truncate(available.min(4));
        for subcenter in subcenters {
            if let Some(existing) = self
                .checkpoint
                .buckets
                .iter()
                .position(|candidate| candidate.bucket_id == subcenter.bucket_id)
            {
                if self.checkpoint.buckets[existing]
                    .frozen_program_sha256
                    .is_none()
                {
                    self.checkpoint.buckets[existing] = subcenter;
                    self.normalize_bucket_receipts(existing);
                    self.maybe_freeze(existing)?;
                }
                continue;
            }
            self.checkpoint.buckets.push(subcenter);
            let subcenter_index = self.checkpoint.buckets.len().saturating_sub(1);
            self.normalize_bucket_receipts(subcenter_index);
            self.maybe_freeze(subcenter_index)?;
            self.checkpoint.cegis_subcenters_total =
                self.checkpoint.cegis_subcenters_total.saturating_add(1);
        }
        Ok(())
    }

    pub(super) fn normalize_bucket_receipts(&mut self, index: usize) {
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

    pub(super) fn persist(&self) -> Result<(), String> {
        // Raw provider payloads are bounded working memory, never durable
        // evidence. Receipts and the intersected program pool are sufficient
        // to resume; explicit replay can rehydrate examples when required.
        let mut durable_checkpoint = self.checkpoint.clone();
        for bucket in &mut durable_checkpoint.buckets {
            bucket.runtime_examples.clear();
            if bucket.frozen_program_sha256.is_some() {
                bucket.durable_adapter_phase_atoms.clear();
            } else {
                let support_refs = bucket
                    .support
                    .iter()
                    .map(|receipt| receipt.evidence_graph_sha256.as_str())
                    .collect::<BTreeSet<_>>();
                bucket
                    .durable_adapter_phase_atoms
                    .retain(|evidence, _| support_refs.contains(evidence.as_str()));
            }
        }
        let payload = serde_cbor::to_vec(&durable_checkpoint)
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
