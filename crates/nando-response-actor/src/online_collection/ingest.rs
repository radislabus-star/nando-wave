//! Online observation ingestion, replay rehydration, and bounded miner state updates.

use super::*;

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
                pooling_strategy_version: ONLINE_COLLECTION_POOLING_STRATEGY_V36,
                structural_resynthesis_pending_bucket_ids: BTreeSet::new(),
                structural_resynthesis_completed_buckets_total: 0,
                structural_resynthesis_failed_buckets_total: 0,
                config,
                observations_total: 0,
                duplicate_observations_total: 0,
                observed_evidence_graph_sha256: BTreeSet::new(),
                unsupported_total: 0,
                synthesis_error_total: 0,
                privacy_rejected_observations_total: 0,
                unsupported_dynamic_zero_total: 0,
                unsupported_dynamic_partial_total: 0,
                unsupported_dynamic_full_total: 0,
                unsupported_partial_with_request_source_total: 0,
                unsupported_partial_with_tool_source_total: 0,
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
                policy_rejection_reasons: BTreeMap::new(),
                counterexamples_total: 0,
                cegis_subcenters_total: 0,
                revoked_candidates_total: 0,
                late_after_freeze_total: 0,
                future_intent_rejected_total: 0,
                frozen_route_candidates_considered_total: 0,
                frozen_route_anti_rejected_total: 0,
                frozen_route_phase_rejected_total: 0,
                frozen_route_verifier_rejected_total: 0,
                frozen_route_rejection_reasons: BTreeMap::new(),
                frozen_route_witness_pending_total: 0,
                frozen_route_witness_resolved_total: 0,
                frozen_route_irreducible_total: 0,
                frozen_route_applicability_abstain_total: 0,
                frozen_future_accepted_total: 0,
                exact_executable_observations_total: 0,
                semantic_executable_observations_total: 0,
                teacher_only_observations_total: 0,
                program_pool_reuse_total: 0,
                program_pool_receipts_total: 0,
                renderer_consensus_migrated_examples_total: 0,
                legacy_partial_observations_discarded_total: 0,
                legacy_partial_buckets_discarded_total: 0,
                legacy_partial_receipts_discarded_total: 0,
                unreplayable_support_discarded_total: 0,
                applicability_negative_sessions: BTreeMap::new(),
                buckets: Vec::new(),
            }
        };
        let legacy_migrated = checkpoint.schema != ONLINE_COLLECTION_SCHEMA_V3
            || checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V3;
        if legacy_migrated {
            migrate_collection_program_pools(&mut checkpoint)?;
        }
        let archetype_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V4;
        if archetype_migrated {
            migrate_collection_archetype_pools(&mut checkpoint)?;
        }
        let exact_authority_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V5;
        if exact_authority_migrated {
            migrate_collection_exact_authority_pools(&mut checkpoint)?;
        }
        let renderer_consensus_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V6;
        if renderer_consensus_migrated {
            migrate_collection_renderer_consensus_pools(&mut checkpoint)?;
        }
        let invariant_wave_center_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V7;
        if invariant_wave_center_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V7;
        }
        let active_witness_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V8;
        if active_witness_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V8;
            migrate_collection_active_witness_pools(&mut checkpoint)?;
            checkpoint.frozen_route_candidates_considered_total = 0;
            checkpoint.frozen_route_anti_rejected_total = 0;
            checkpoint.frozen_route_phase_rejected_total = 0;
            checkpoint.frozen_route_verifier_rejected_total = 0;
            checkpoint.frozen_route_witness_pending_total = 0;
            checkpoint.frozen_route_witness_resolved_total = 0;
            checkpoint.frozen_route_irreducible_total = 0;
            checkpoint.frozen_future_accepted_total = 0;
            checkpoint.late_after_freeze_total = 0;
            checkpoint.future_intent_rejected_total = 0;
        }
        let exact_teacher_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V9;
        if exact_teacher_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V9;
            checkpoint.frozen_route_candidates_considered_total = 0;
            checkpoint.frozen_route_anti_rejected_total = 0;
            checkpoint.frozen_route_phase_rejected_total = 0;
            checkpoint.frozen_route_verifier_rejected_total = 0;
            checkpoint.frozen_route_rejection_reasons.clear();
            checkpoint.frozen_route_witness_pending_total = 0;
            checkpoint.frozen_route_witness_resolved_total = 0;
            checkpoint.frozen_route_irreducible_total = 0;
            checkpoint.frozen_future_accepted_total = 0;
            checkpoint.late_after_freeze_total = 0;
            checkpoint.future_intent_rejected_total = 0;
            for bucket in &mut checkpoint.buckets {
                bucket.future.clear();
                bucket.durable_runtime_parity_receipts.clear();
            }
        }
        let typed_negative_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V10;
        if typed_negative_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V10;
            checkpoint.frozen_route_candidates_considered_total = 0;
            checkpoint.frozen_route_anti_rejected_total = 0;
            checkpoint.frozen_route_phase_rejected_total = 0;
            checkpoint.frozen_route_verifier_rejected_total = 0;
            checkpoint.frozen_route_rejection_reasons.clear();
            checkpoint.frozen_route_witness_pending_total = 0;
            checkpoint.frozen_route_witness_resolved_total = 0;
            checkpoint.frozen_route_irreducible_total = 0;
            checkpoint.frozen_route_applicability_abstain_total = 0;
            checkpoint.frozen_future_accepted_total = 0;
            checkpoint.late_after_freeze_total = 0;
            checkpoint.future_intent_rejected_total = 0;
            for bucket in &mut checkpoint.buckets {
                bucket.learned_anti_atom_ids.clear();
            }
        }
        let exact_receipt_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V12;
        if exact_receipt_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V12;
            migrate_collection_exact_receipts(&mut checkpoint)?;
            checkpoint.frozen_route_candidates_considered_total = 0;
            checkpoint.frozen_route_anti_rejected_total = 0;
            checkpoint.frozen_route_phase_rejected_total = 0;
            checkpoint.frozen_route_verifier_rejected_total = 0;
            checkpoint.frozen_route_rejection_reasons.clear();
            checkpoint.frozen_route_witness_pending_total = 0;
            checkpoint.frozen_route_witness_resolved_total = 0;
            checkpoint.frozen_route_irreducible_total = 0;
            checkpoint.frozen_route_applicability_abstain_total = 0;
            checkpoint.frozen_future_accepted_total = 0;
            checkpoint.late_after_freeze_total = 0;
            checkpoint.future_intent_rejected_total = 0;
        }
        let law_quotient_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V13;
        if law_quotient_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V13;
        }
        let keyed_layout_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V14;
        if keyed_layout_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V14;
            migrate_collection_keyed_layouts(&mut checkpoint)?;
        }
        let adapter_intersection_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V15;
        if adapter_intersection_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V15;
        }
        let phase_adapter_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V16;
        if phase_adapter_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V16;
        }
        let decidable_recovery_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V17;
        if decidable_recovery_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V17;
        }
        let relational_role_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V18;
        if relational_role_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V18;
            migrate_collection_relational_role_programs(&mut checkpoint)?;
        }
        let replayable_support_revalidated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V19;
        if replayable_support_revalidated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V19;
        }
        let consensus_policy_reconsidered =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V20;
        if consensus_policy_reconsidered {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V20;
        }
        let structural_resynthesis_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V21;
        if structural_resynthesis_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V21;
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
        }
        let selector_law_quotient_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V22;
        if selector_law_quotient_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V22;
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
        }
        let semantic_adapter_wave_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V23;
        if semantic_adapter_wave_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V23;
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
        }
        let expanded_adapter_library_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V24;
        if expanded_adapter_library_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V24;
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
        }
        let relational_adapter_path_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V25;
        if relational_adapter_path_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V25;
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
        }
        let lexical_adapter_wave_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V26;
        if lexical_adapter_wave_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V26;
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
        }
        let adapter_wave_proof_refresh_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V27;
        if adapter_wave_proof_refresh_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V27;
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
        }
        let turn_output_adapter_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V28;
        if turn_output_adapter_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V28;
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
        }
        let concrete_adapter_law_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V29;
        if concrete_adapter_law_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V29;
        }
        let canonical_alignment_refresh_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V31;
        if canonical_alignment_refresh_migrated {
            // New canonical alignment is applied only to newly observed or
            // explicitly replayed support. Restart must not duplicate buckets
            // or silently reclassify retained evidence.
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V31;
        }
        let durable_phase_adapter_refresh_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V32;
        if durable_phase_adapter_refresh_migrated {
            // V32 can reconsider retained support without raw provider data:
            // routing atoms are recovered from durable pre-action receipts.
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V32;
        }
        let durable_law_subcenter_refresh_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V33;
        if durable_law_subcenter_refresh_migrated {
            // Matched program digests are exact teacher proofs, so V33 can
            // recover law subcenters without retaining provider payloads.
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V33;
        }
        let exact_subcenter_dedup_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V34;
        if exact_subcenter_dedup_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V34;
        }
        let durable_adapter_phase_evidence_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V35;
        if durable_adapter_phase_evidence_migrated {
            // Hash-only V34 checkpoints cannot manufacture actor phase atoms.
            // Queue retained support for bounded replay; only matching real
            // evidence may populate the compact V35 proof field.
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V35;
        }
        let adaptive_identification_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V36;
        if adaptive_identification_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V36;
        }
        let accounting_repaired = repair_collection_checkpoint_accounting(&mut checkpoint);
        validate_checkpoint(&checkpoint, config)?;
        let mut miner = Self { path, checkpoint };
        if replayable_support_revalidated {
            miner.revalidate_replayable_support_buffered()?;
        }
        let pre_v17_migrated = legacy_migrated
            || archetype_migrated
            || exact_authority_migrated
            || renderer_consensus_migrated
            || invariant_wave_center_migrated
            || active_witness_migrated
            || exact_teacher_migrated
            || typed_negative_migrated
            || exact_receipt_migrated
            || law_quotient_migrated
            || keyed_layout_migrated
            || adapter_intersection_migrated
            || phase_adapter_migrated
            || accounting_repaired;
        let checkpoint_migrated = pre_v17_migrated
            || decidable_recovery_migrated
            || relational_role_migrated
            || replayable_support_revalidated
            || consensus_policy_reconsidered
            || structural_resynthesis_migrated
            || selector_law_quotient_migrated
            || turn_output_adapter_migrated
            || concrete_adapter_law_migrated
            || canonical_alignment_refresh_migrated
            || durable_phase_adapter_refresh_migrated
            || durable_law_subcenter_refresh_migrated
            || exact_subcenter_dedup_migrated
            || durable_adapter_phase_evidence_migrated
            || adaptive_identification_migrated;
        if checkpoint_migrated {
            if exact_subcenter_dedup_migrated {
                miner.deduplicate_exact_unfrozen_buckets()?;
            }
            if pre_v17_migrated {
                miner.merge_converged_unfrozen_buckets()?;
            }
            let migration_indices = if adaptive_identification_migrated
                && miner.checkpoint.config.proof_mode
                    == OnlineCollectionProofMode::AdaptiveVersionSpace
            {
                miner
                    .checkpoint
                    .buckets
                    .iter()
                    .enumerate()
                    .filter(|(_, bucket)| {
                        bucket.frozen_program_sha256.is_none() && !bucket.support.is_empty()
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>()
            } else if pre_v17_migrated {
                (0..miner.checkpoint.buckets.len()).collect::<Vec<_>>()
            } else if durable_phase_adapter_refresh_migrated
                || durable_law_subcenter_refresh_migrated
                || exact_subcenter_dedup_migrated
            {
                miner
                    .checkpoint
                    .buckets
                    .iter()
                    .enumerate()
                    .filter(|(_, bucket)| {
                        bucket.frozen_program_sha256.is_none()
                            && bucket.support.len() >= miner.checkpoint.config.support_rows
                            && bucket.programs.len() > 1
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>()
            } else if consensus_policy_reconsidered
                || structural_resynthesis_migrated
                || selector_law_quotient_migrated
                || canonical_alignment_refresh_migrated
            {
                Vec::new()
            } else {
                miner
                    .checkpoint
                    .buckets
                    .iter()
                    .enumerate()
                    .filter(|(_, bucket)| {
                        bucket.frozen_program_sha256.is_none()
                            && bucket.support.len() >= miner.checkpoint.config.support_rows
                            && bucket.programs.len() > 1
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>()
            };
            for index in migration_indices {
                miner.normalize_bucket_receipts(index);
                if adaptive_identification_migrated
                    && miner.checkpoint.config.proof_mode
                        == OnlineCollectionProofMode::AdaptiveVersionSpace
                {
                    miner.maybe_freeze(index)?;
                } else {
                    miner.freeze_or_split(index)?;
                }
            }
            miner.persist()?;
        }
        Ok(miner)
    }

    pub fn observe(&mut self, observation: OnlineCollectionObservation) -> Result<(), String> {
        self.observe_with_persistence(observation, true, false)
    }

    pub fn observe_buffered(
        &mut self,
        observation: OnlineCollectionObservation,
    ) -> Result<(), String> {
        self.observe_with_persistence(observation, false, false)
    }

    pub fn observe_replay_training_buffered(
        &mut self,
        observation: OnlineCollectionObservation,
    ) -> Result<(), String> {
        self.observe_with_persistence(observation, false, true)
    }

    pub fn rehydrate_replay_training_buffered(
        &mut self,
        observation: OnlineCollectionObservation,
    ) -> Result<(), String> {
        validate_observation(&observation)?;
        if !self
            .checkpoint
            .observed_evidence_graph_sha256
            .contains(&observation.evidence_graph_sha256)
        {
            return Ok(());
        }
        let evidence_id = observation.evidence_graph_sha256.as_str();
        let indices = self
            .checkpoint
            .buckets
            .iter()
            .enumerate()
            .filter(|(_, bucket)| {
                bucket.frozen_program_sha256.is_none()
                    && !bucket.runtime_examples.contains_key(evidence_id)
                    && bucket
                        .support
                        .iter()
                        .any(|receipt| receipt.evidence_graph_sha256 == evidence_id)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        for index in indices {
            let verified = self.checkpoint.buckets[index]
                .support
                .iter()
                .find(|receipt| receipt.evidence_graph_sha256 == evidence_id)
                .is_some_and(|receipt| {
                    receipt.matched_program_sha256.iter().any(|digest| {
                        self.checkpoint.buckets[index]
                            .programs
                            .get(digest)
                            .is_some_and(|program| {
                                independently_verified_teacher_match(program, &observation.example)
                            })
                    })
                });
            if !verified {
                continue;
            }
            let layout_sha256 = structural_layout_sha256(&observation.example.provider_payload)?;
            for receipt in self.checkpoint.buckets[index]
                .support
                .iter_mut()
                .filter(|receipt| receipt.evidence_graph_sha256 == evidence_id)
            {
                receipt.layout_sha256.clone_from(&layout_sha256);
            }
            insert_runtime_example(
                &mut self.checkpoint.buckets[index],
                &observation,
                self.checkpoint.config.max_receipts_per_bucket,
            );
            refresh_durable_adapter_phase_atoms(&mut self.checkpoint.buckets[index]);
            self.freeze_or_split(index)?;
        }
        Ok(())
    }

    pub fn rehydrate_legacy_replay_training_buffered(
        &mut self,
        observation: OnlineCollectionObservation,
        source_session_identities: &BTreeSet<String>,
    ) -> Result<LegacyReplayRehydrationStats, String> {
        validate_observation(&observation)?;
        let layout_sha256 = structural_layout_sha256(&observation.example.provider_payload)?;
        let mut stats = LegacyReplayRehydrationStats::default();
        let indices = self
            .checkpoint
            .buckets
            .iter()
            .enumerate()
            .filter_map(|(index, bucket)| {
                if bucket.frozen_program_sha256.is_some() {
                    return None;
                }
                let mut matches = Vec::new();
                for receipt in &bucket.support {
                    if !source_session_identities.contains(&receipt.session_id_sha256)
                        || bucket
                            .runtime_examples
                            .contains_key(&receipt.evidence_graph_sha256)
                    {
                        continue;
                    }
                    stats.session_receipts = stats.session_receipts.saturating_add(1);
                    let event_matches = match (
                        receipt.event_time_unix_nanos,
                        observation.event_time_unix_nanos,
                    ) {
                        (Some(left), Some(right)) => left == right,
                        (None, None) => true,
                        _ => false,
                    };
                    if !event_matches {
                        continue;
                    }
                    stats.event_time_matches = stats.event_time_matches.saturating_add(1);
                    if receipt.estimated_input_tokens != observation.estimated_input_tokens {
                        continue;
                    }
                    stats.token_matches = stats.token_matches.saturating_add(1);
                    if !receipt.matched_program_sha256.iter().any(|digest| {
                        bucket.programs.get(digest).is_some_and(|program| {
                            independently_verified_teacher_match(program, &observation.example)
                        })
                    }) {
                        continue;
                    }
                    stats.verifier_matches = stats.verifier_matches.saturating_add(1);
                    let layout_matches = receipt.layout_sha256 == layout_sha256;
                    if layout_matches {
                        stats.layout_matches = stats.layout_matches.saturating_add(1);
                    }
                    matches.push((receipt.evidence_graph_sha256.clone(), layout_matches));
                }
                let layout_matches = matches
                    .iter()
                    .filter(|(_, layout_matches)| *layout_matches)
                    .map(|(evidence_id, _)| evidence_id.clone())
                    .collect::<Vec<_>>();
                let selected = if layout_matches.len() == 1 {
                    Some(layout_matches[0].clone())
                } else if layout_matches.is_empty() && matches.len() == 1 {
                    Some(matches[0].0.clone())
                } else {
                    None
                };
                if selected.is_none() && !matches.is_empty() {
                    stats.ambiguous_matches = stats.ambiguous_matches.saturating_add(1);
                }
                selected.map(|evidence_id| (index, evidence_id))
            })
            .collect::<Vec<_>>();
        for (index, evidence_id) in indices {
            insert_runtime_example_for_evidence(
                &mut self.checkpoint.buckets[index],
                &evidence_id,
                &observation,
                self.checkpoint.config.max_receipts_per_bucket,
            );
            self.freeze_or_split(index)?;
            stats.attached_receipts = stats.attached_receipts.saturating_add(1);
        }
        Ok(stats)
    }

    pub fn revalidate_replayable_support_buffered(&mut self) -> Result<u64, String> {
        self.revalidate_support_buffered(false, true)
    }

    pub(super) fn revalidate_support_buffered(
        &mut self,
        structural_only: bool,
        refresh_proof: bool,
    ) -> Result<u64, String> {
        let initial_bucket_count = self.checkpoint.buckets.len();
        let mut links_added = 0_u64;
        for index in 0..initial_bucket_count {
            links_added = links_added.saturating_add(self.revalidate_bucket_support(
                index,
                structural_only,
                refresh_proof,
            )?);
        }
        Ok(links_added)
    }

    pub(super) fn revalidate_bucket_support(
        &mut self,
        index: usize,
        structural_only: bool,
        refresh_proof: bool,
    ) -> Result<u64, String> {
        let Some(bucket) = self.checkpoint.buckets.get(index) else {
            return Ok(0);
        };
        if bucket.frozen_program_sha256.is_some()
            || (structural_only
                && !bucket
                    .programs
                    .values()
                    .any(|program| canonical_dynamic_role_count(program) >= 2))
        {
            return Ok(0);
        }
        let links = {
            let mut links = Vec::new();
            for (receipt_index, receipt) in bucket.support.iter().enumerate() {
                let Some(example) = bucket.runtime_examples.get(&receipt.evidence_graph_sha256)
                else {
                    continue;
                };
                let has_retained_match = receipt
                    .matched_program_sha256
                    .iter()
                    .any(|digest| bucket.programs.contains_key(digest));
                for (digest, program) in &bucket.programs {
                    if !receipt.matched_program_sha256.contains(digest)
                        && (!structural_only
                            || !has_retained_match
                            || canonical_dynamic_role_count(program) >= 2)
                        && independently_verified_teacher_match(program, example)
                    {
                        links.push((receipt_index, digest.clone()));
                    }
                }
            }
            links
        };
        let links_added = u64::try_from(links.len()).unwrap_or(u64::MAX);
        if !links.is_empty() {
            let bucket = &mut self.checkpoint.buckets[index];
            for (receipt_index, digest) in links {
                bucket.support[receipt_index]
                    .matched_program_sha256
                    .push(digest);
            }
            for receipt in &mut bucket.support {
                receipt.matched_program_sha256.sort();
                receipt.matched_program_sha256.dedup();
            }
        }
        self.normalize_bucket_receipts(index);
        if refresh_proof {
            self.freeze_or_split(index)?;
        }
        Ok(links_added)
    }

    #[must_use]
    pub fn has_structural_resynthesis_work(&self) -> bool {
        !self
            .checkpoint
            .structural_resynthesis_pending_bucket_ids
            .is_empty()
    }

    pub fn run_structural_resynthesis_work_slice(&mut self) -> Result<u64, String> {
        let Some(bucket_id) = self
            .checkpoint
            .structural_resynthesis_pending_bucket_ids
            .pop_first()
        else {
            return Ok(0);
        };
        let Some(index) = self
            .checkpoint
            .buckets
            .iter()
            .position(|bucket| bucket.bucket_id == bucket_id)
        else {
            self.checkpoint
                .structural_resynthesis_completed_buckets_total = self
                .checkpoint
                .structural_resynthesis_completed_buckets_total
                .saturating_add(1);
            return Ok(0);
        };
        let result = self.resynthesize_bucket_structural_programs(index);
        match result {
            Ok(programs_added) => {
                self.freeze_or_split(index)?;
                self.checkpoint
                    .structural_resynthesis_completed_buckets_total = self
                    .checkpoint
                    .structural_resynthesis_completed_buckets_total
                    .saturating_add(1);
                Ok(programs_added)
            }
            Err(error) => {
                self.checkpoint.structural_resynthesis_failed_buckets_total = self
                    .checkpoint
                    .structural_resynthesis_failed_buckets_total
                    .saturating_add(1);
                Err(error)
            }
        }
    }

    pub(super) fn resynthesize_bucket_structural_programs(
        &mut self,
        index: usize,
    ) -> Result<u64, String> {
        let bucket = self
            .checkpoint
            .buckets
            .get(index)
            .ok_or_else(|| "online_collection_resynthesis_bucket_missing".to_owned())?;
        if bucket.frozen_program_sha256.is_some()
            || bucket.support.len() < self.checkpoint.config.support_rows
        {
            return Ok(0);
        }
        let archetype_id = bucket.archetype_id.clone();
        let mut seeds = bucket
            .support
            .iter()
            .filter_map(|receipt| {
                let example = bucket
                    .runtime_examples
                    .get(&receipt.evidence_graph_sha256)?;
                let coverage = diagnose_response_dynamic_coverage(example);
                (coverage.matching_selectors >= 2).then_some((
                    coverage.matching_selectors,
                    coverage.tool_dynamic_bytes,
                    coverage.dynamic_bytes,
                    example.expected_response.len(),
                    receipt.evidence_graph_sha256.clone(),
                    receipt.clone(),
                    example.clone(),
                ))
            })
            .collect::<Vec<_>>();
        seeds.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| right.1.cmp(&left.1))
                .then_with(|| right.2.cmp(&left.2))
                .then_with(|| right.3.cmp(&left.3))
                .then_with(|| left.4.cmp(&right.4))
        });
        seeds.truncate(MAX_STRUCTURAL_RESYNTHESIS_SEEDS_PER_BUCKET);

        let mut programs = BTreeMap::new();
        for (_, _, _, _, _, receipt, example) in seeds {
            let observation = OnlineCollectionObservation {
                evidence_graph_sha256: receipt.evidence_graph_sha256,
                client_intent_id_sha256: receipt.client_intent_id_sha256,
                session_id_sha256: receipt.session_id_sha256,
                event_time_unix_nanos: receipt.event_time_unix_nanos,
                estimated_input_tokens: receipt.estimated_input_tokens,
                example,
            };
            for (digest, program) in structural_programs_for_observation(&observation)? {
                if response_program_archetype_id(&program)? == archetype_id {
                    programs.insert(digest, program);
                }
            }
        }
        let programs_added = {
            let bucket = &mut self.checkpoint.buckets[index];
            let added = programs
                .keys()
                .filter(|digest| !bucket.programs.contains_key(*digest))
                .count();
            bucket.programs.extend(programs);
            bucket.programs = bounded_program_map(
                std::mem::take(&mut bucket.programs),
                crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS,
            );
            bucket.bucket_id =
                collection_archetype_bucket_id(&bucket.archetype_id, bucket.programs.keys())?;
            u64::try_from(added).unwrap_or(u64::MAX)
        };
        self.revalidate_bucket_support(index, true, true)?;
        Ok(programs_added)
    }

    pub fn flush(&self) -> Result<(), String> {
        self.persist()
    }

    pub(super) fn observe_with_persistence(
        &mut self,
        observation: OnlineCollectionObservation,
        durable: bool,
        support_only: bool,
    ) -> Result<(), String> {
        validate_observation(&observation)?;
        let already_observed = self
            .checkpoint
            .observed_evidence_graph_sha256
            .contains(&observation.evidence_graph_sha256);
        if already_observed && !support_only {
            self.checkpoint.duplicate_observations_total = self
                .checkpoint
                .duplicate_observations_total
                .saturating_add(1);
            return self.persist_if(durable);
        }
        let count_observation = !already_observed;
        let evidence_graph_sha256 = observation.evidence_graph_sha256.clone();
        if count_observation {
            self.checkpoint.observations_total =
                self.checkpoint.observations_total.saturating_add(1);
        }
        let frozen_match = !support_only && self.evaluate_frozen_candidates(&observation)?;
        if frozen_match {
            return self.persist_new_observation(evidence_graph_sha256, durable, false);
        }
        let matching_existing = self.matching_unfrozen_buckets(&observation)?;
        match matching_existing.as_slice() {
            [(index, matching_programs)] => {
                let matching_programs = self.checkpoint.buckets[*index]
                    .programs
                    .iter()
                    .filter(|(digest, _)| matching_programs.contains(*digest))
                    .map(|(digest, program)| (digest.clone(), program.clone()))
                    .collect::<BTreeMap<_, _>>();
                let exact_match = matching_programs.values().any(|program| {
                    response_program_exactly_matches_example(program, &observation.example)
                });
                self.record_executable_observation(exact_match, count_observation);
                self.update_bucket(*index, &matching_programs, &observation, true, true)?;
                let structural_programs = structural_programs_for_observation(&observation)?;
                if !structural_programs.is_empty() {
                    self.assign_archetype_programs(structural_programs, &observation, true, false)?;
                }
                return self.persist_new_observation(evidence_graph_sha256, durable, true);
            }
            [_, _, ..] => {
                let exact_match = matching_existing.iter().any(|(index, matching_programs)| {
                    self.checkpoint.buckets[*index]
                        .programs
                        .iter()
                        .filter(|(digest, _)| matching_programs.contains(*digest))
                        .any(|(_, program)| {
                            response_program_exactly_matches_example(program, &observation.example)
                        })
                });
                self.record_executable_observation(exact_match, count_observation);
                if count_observation {
                    self.checkpoint.ambiguous_assignment_total =
                        self.checkpoint.ambiguous_assignment_total.saturating_add(1);
                }
                for (index, matching_programs) in matching_existing.iter().cloned() {
                    let matching_programs = self.checkpoint.buckets[index]
                        .programs
                        .iter()
                        .filter(|(digest, _)| matching_programs.contains(*digest))
                        .map(|(digest, program)| (digest.clone(), program.clone()))
                        .collect::<BTreeMap<_, _>>();
                    self.update_bucket(index, &matching_programs, &observation, true, true)?;
                }
                let structural_programs = structural_programs_for_observation(&observation)?;
                if !structural_programs.is_empty() {
                    self.assign_archetype_programs(structural_programs, &observation, true, false)?;
                }
                return self.persist_new_observation(evidence_graph_sha256, durable, true);
            }
            [] => {}
        }
        self.checkpoint.full_enumerations_total =
            self.checkpoint.full_enumerations_total.saturating_add(1);
        let synthesis_example = compact_active_turn_synthesis_example(&observation.example)
            .unwrap_or_else(|| observation.example.clone());
        let coverage = diagnose_response_dynamic_coverage(&synthesis_example);
        let source_span = unsupported_source_span(&synthesis_example);
        let scalar_overlap = has_scalar_overlap(&synthesis_example);
        let version_space = match enumerate_source_neutral_response_programs_with_coverage(
            &synthesis_example,
            Some(coverage),
        ) {
            Ok(version_space) => version_space,
            Err(_) => {
                if count_observation {
                    self.checkpoint.synthesis_error_total =
                        self.checkpoint.synthesis_error_total.saturating_add(1);
                    self.checkpoint.unsupported_total =
                        self.checkpoint.unsupported_total.saturating_add(1);
                }
                return self.persist_new_observation(evidence_graph_sha256, durable, false);
            }
        };
        self.checkpoint.exact_checks_total = self
            .checkpoint
            .exact_checks_total
            .saturating_add(version_space.exact_checks as u64);
        self.checkpoint.candidates_enumerated_total = self
            .checkpoint
            .candidates_enumerated_total
            .saturating_add(version_space.candidates_enumerated as u64);
        if count_observation {
            self.checkpoint.policy_rejected_exact_matches = self
                .checkpoint
                .policy_rejected_exact_matches
                .saturating_add(version_space.policy_rejected_exact_matches as u64);
            for (reason, count) in &version_space.policy_rejection_reasons {
                let total = self
                    .checkpoint
                    .policy_rejection_reasons
                    .entry(reason.clone())
                    .or_default();
                *total = total.saturating_add(*count as u64);
            }
        }
        let exact_programs = version_space
            .programs
            .iter()
            .filter(|program| {
                crate::response_program_exactly_matches_example(program, &observation.example)
            })
            .cloned()
            .collect::<Vec<_>>();
        let exact_program_count = exact_programs.len();
        let teacher_programs = version_space
            .programs
            .into_iter()
            .filter(|program| {
                response_program_authority_matches_example(program, &observation.example)
            })
            .collect::<Vec<_>>();
        let exact_programs = exact_programs
            .into_iter()
            .filter(is_privacy_safe_online_response_program)
            .filter(|program| {
                independently_verified_authority_response(program, &observation.example).is_some()
            })
            .map(|program| {
                canonical_json_sha256(&program)
                    .map(|digest| (digest, program))
                    .map_err(str::to_owned)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let teacher_programs = teacher_programs
            .into_iter()
            .filter(is_privacy_safe_online_response_program)
            .filter(|program| {
                independently_verified_authority_response(program, &observation.example).is_some()
            })
            .map(|program| {
                canonical_json_sha256(&program)
                    .map(|digest| (digest, program))
                    .map_err(str::to_owned)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let exact_verified = !exact_programs.is_empty();
        // Keep canonical semantic operators alongside surface-specific exact
        // renderers so repeated behavior can converge across response styles.
        let programs = teacher_programs;
        if programs.is_empty() {
            if count_observation {
                self.checkpoint.unsupported_total =
                    self.checkpoint.unsupported_total.saturating_add(1);
                if exact_program_count > 0 {
                    self.checkpoint.privacy_rejected_observations_total = self
                        .checkpoint
                        .privacy_rejected_observations_total
                        .saturating_add(1);
                }
                if coverage.dynamic_bytes == 0 {
                    self.checkpoint.unsupported_dynamic_zero_total = self
                        .checkpoint
                        .unsupported_dynamic_zero_total
                        .saturating_add(1);
                } else if coverage.dynamic_bytes < coverage.response_bytes {
                    self.checkpoint.unsupported_dynamic_partial_total = self
                        .checkpoint
                        .unsupported_dynamic_partial_total
                        .saturating_add(1);
                    if coverage.request_dynamic_bytes > 0 {
                        self.checkpoint
                            .unsupported_partial_with_request_source_total = self
                            .checkpoint
                            .unsupported_partial_with_request_source_total
                            .saturating_add(1);
                    }
                    if coverage.tool_dynamic_bytes > 0 {
                        self.checkpoint.unsupported_partial_with_tool_source_total = self
                            .checkpoint
                            .unsupported_partial_with_tool_source_total
                            .saturating_add(1);
                    }
                } else {
                    self.checkpoint.unsupported_dynamic_full_total = self
                        .checkpoint
                        .unsupported_dynamic_full_total
                        .saturating_add(1);
                }
                match source_span {
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
                if scalar_overlap {
                    self.checkpoint.unsupported_with_scalar_overlap = self
                        .checkpoint
                        .unsupported_with_scalar_overlap
                        .saturating_add(1);
                }
            }
            return self.persist_new_observation(evidence_graph_sha256, durable, false);
        }
        if count_observation {
            if exact_verified {
                self.checkpoint.exact_executable_observations_total = self
                    .checkpoint
                    .exact_executable_observations_total
                    .saturating_add(1);
            } else {
                self.checkpoint.semantic_executable_observations_total = self
                    .checkpoint
                    .semantic_executable_observations_total
                    .saturating_add(1);
            }
        }
        if support_only
            && self.checkpoint.buckets.iter().any(|bucket| {
                bucket.frozen_program_sha256.is_some()
                    && bucket.programs.keys().any(|key| programs.contains_key(key))
            })
        {
            return self.persist_new_observation(evidence_graph_sha256, durable, false);
        }
        self.assign_archetype_programs(programs, &observation, true, count_observation)?;
        self.persist_new_observation(evidence_graph_sha256, durable, true)
    }

    pub(super) fn record_executable_observation(
        &mut self,
        exact_teacher_match: bool,
        count_observation: bool,
    ) {
        if !count_observation {
            return;
        }
        if exact_teacher_match {
            self.checkpoint.exact_executable_observations_total = self
                .checkpoint
                .exact_executable_observations_total
                .saturating_add(1);
        } else {
            self.checkpoint.semantic_executable_observations_total = self
                .checkpoint
                .semantic_executable_observations_total
                .saturating_add(1);
        }
    }

    #[must_use]
    pub fn consensus_diagnostics(&self) -> Vec<OnlineCollectionConsensusDiagnostic> {
        self.checkpoint
            .buckets
            .iter()
            .filter(|bucket| {
                bucket.frozen_program_sha256.is_none()
                    && bucket.support.len() >= self.checkpoint.config.support_rows
            })
            .map(|bucket| {
                consensus_diagnostic(
                    bucket,
                    self.checkpoint.config.support_rows,
                    self.checkpoint.config.max_receipts_per_bucket,
                )
            })
            .collect()
    }

    #[must_use]
    pub fn consensus_diagnostic_for_bucket(
        &self,
        bucket_id: &str,
    ) -> Option<OnlineCollectionConsensusDiagnostic> {
        self.checkpoint
            .buckets
            .iter()
            .find(|bucket| bucket.bucket_id == bucket_id)
            .map(|bucket| {
                consensus_diagnostic(
                    bucket,
                    self.checkpoint.config.support_rows,
                    self.checkpoint.config.max_receipts_per_bucket,
                )
            })
    }

    pub fn status(&self) -> OnlineCollectionStatus {
        let mut support_receipts = BTreeMap::new();
        let mut future_receipts = BTreeMap::new();
        let mut runtime_parity_receipts = BTreeSet::new();
        let mut durable_adapter_phase_evidence = BTreeSet::new();
        let mut durable_adapter_phase_pairs = 0_usize;
        for bucket in &self.checkpoint.buckets {
            for (evidence_id, atoms_by_program) in &bucket.durable_adapter_phase_atoms {
                durable_adapter_phase_evidence.insert(evidence_id.clone());
                durable_adapter_phase_pairs =
                    durable_adapter_phase_pairs.saturating_add(atoms_by_program.len());
            }
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
                    || bucket
                        .durable_runtime_parity_receipts
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
            .map(|bucket| {
                bucket_status(
                    bucket,
                    self.checkpoint.config.proof_mode,
                    self.checkpoint.config.support_rows,
                )
            })
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
        let accounted_ambiguous_total = self.checkpoint.ambiguous_assignment_total;
        let accounted_executable_total = self
            .checkpoint
            .exact_executable_observations_total
            .saturating_add(self.checkpoint.semantic_executable_observations_total)
            .saturating_sub(accounted_ambiguous_total);
        let classified = self
            .checkpoint
            .exact_executable_observations_total
            .saturating_add(self.checkpoint.semantic_executable_observations_total)
            .saturating_add(self.checkpoint.unsupported_total);
        let legacy_unclassified_observations_total = self
            .checkpoint
            .observations_total
            .saturating_sub(classified);
        let accounted_irreducible_total = self
            .checkpoint
            .unsupported_total
            .saturating_add(legacy_unclassified_observations_total);
        OnlineCollectionStatus {
            pooling_strategy_version: self.checkpoint.pooling_strategy_version,
            durable_adapter_phase_evidence_rows: durable_adapter_phase_evidence.len(),
            durable_adapter_phase_pairs,
            structural_resynthesis_pending_buckets: self
                .checkpoint
                .structural_resynthesis_pending_bucket_ids
                .len(),
            structural_resynthesis_completed_buckets_total: self
                .checkpoint
                .structural_resynthesis_completed_buckets_total,
            structural_resynthesis_failed_buckets_total: self
                .checkpoint
                .structural_resynthesis_failed_buckets_total,
            observations_total: self.checkpoint.observations_total,
            duplicate_observations_total: self.checkpoint.duplicate_observations_total,
            unsupported_total: self.checkpoint.unsupported_total,
            synthesis_error_total: self.checkpoint.synthesis_error_total,
            privacy_rejected_observations_total: self
                .checkpoint
                .privacy_rejected_observations_total,
            unsupported_dynamic_zero_total: self.checkpoint.unsupported_dynamic_zero_total,
            unsupported_dynamic_partial_total: self.checkpoint.unsupported_dynamic_partial_total,
            unsupported_dynamic_full_total: self.checkpoint.unsupported_dynamic_full_total,
            unsupported_partial_with_request_source_total: self
                .checkpoint
                .unsupported_partial_with_request_source_total,
            unsupported_partial_with_tool_source_total: self
                .checkpoint
                .unsupported_partial_with_tool_source_total,
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
            policy_rejection_reasons: self.checkpoint.policy_rejection_reasons.clone(),
            counterexamples_total: self.checkpoint.counterexamples_total,
            cegis_subcenters_total: self.checkpoint.cegis_subcenters_total,
            revoked_candidates_total: self.checkpoint.revoked_candidates_total,
            late_after_freeze_total: self.checkpoint.late_after_freeze_total,
            future_intent_rejected_total: self.checkpoint.future_intent_rejected_total,
            frozen_route_candidates_considered_total: self
                .checkpoint
                .frozen_route_candidates_considered_total,
            frozen_route_anti_rejected_total: self.checkpoint.frozen_route_anti_rejected_total,
            frozen_route_phase_rejected_total: self.checkpoint.frozen_route_phase_rejected_total,
            frozen_route_verifier_rejected_total: self
                .checkpoint
                .frozen_route_verifier_rejected_total,
            frozen_route_rejection_reasons: self.checkpoint.frozen_route_rejection_reasons.clone(),
            frozen_route_rejection_accounting_complete: self
                .checkpoint
                .frozen_route_rejection_reasons
                .values()
                .copied()
                .sum::<u64>()
                == self.checkpoint.frozen_route_verifier_rejected_total,
            frozen_route_witness_pending_total: self.checkpoint.frozen_route_witness_pending_total,
            frozen_route_witness_resolved_total: self
                .checkpoint
                .frozen_route_witness_resolved_total,
            frozen_route_irreducible_total: self.checkpoint.frozen_route_irreducible_total,
            frozen_route_applicability_abstain_total: self
                .checkpoint
                .frozen_route_applicability_abstain_total,
            frozen_route_verifier_accounting_complete: self
                .checkpoint
                .frozen_route_verifier_rejected_total
                == self
                    .checkpoint
                    .frozen_route_witness_pending_total
                    .saturating_add(self.checkpoint.frozen_route_witness_resolved_total)
                    .saturating_add(self.checkpoint.frozen_route_irreducible_total)
                    .saturating_add(self.checkpoint.frozen_route_applicability_abstain_total),
            frozen_future_accepted_total: self.checkpoint.frozen_future_accepted_total,
            frozen_route_accounting_complete: self
                .checkpoint
                .frozen_route_candidates_considered_total
                == self
                    .checkpoint
                    .frozen_route_anti_rejected_total
                    .saturating_add(self.checkpoint.frozen_route_phase_rejected_total)
                    .saturating_add(self.checkpoint.frozen_route_verifier_rejected_total)
                    .saturating_add(self.checkpoint.frozen_future_accepted_total)
                    .saturating_add(self.checkpoint.late_after_freeze_total)
                    .saturating_add(self.checkpoint.future_intent_rejected_total),
            exact_executable_observations_total: self
                .checkpoint
                .exact_executable_observations_total,
            semantic_executable_observations_total: self
                .checkpoint
                .semantic_executable_observations_total,
            teacher_only_observations_total: self.checkpoint.teacher_only_observations_total,
            accounted_executable_total,
            accounted_ambiguous_total,
            accounted_irreducible_total,
            legacy_unclassified_observations_total,
            observation_accounting_complete: self.checkpoint.observations_total
                == accounted_executable_total
                    .saturating_add(accounted_ambiguous_total)
                    .saturating_add(accounted_irreducible_total),
            program_pool_reuse_total: self.checkpoint.program_pool_reuse_total,
            program_pool_receipts_total: self.checkpoint.program_pool_receipts_total,
            renderer_consensus_migrated_examples_total: self
                .checkpoint
                .renderer_consensus_migrated_examples_total,
            legacy_partial_observations_discarded_total: self
                .checkpoint
                .legacy_partial_observations_discarded_total,
            legacy_partial_buckets_discarded_total: self
                .checkpoint
                .legacy_partial_buckets_discarded_total,
            legacy_partial_receipts_discarded_total: self
                .checkpoint
                .legacy_partial_receipts_discarded_total,
            unreplayable_support_discarded_total: self
                .checkpoint
                .unreplayable_support_discarded_total,
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

    pub(super) fn persist_new_observation(
        &mut self,
        evidence_graph_sha256: String,
        durable: bool,
        merge_buckets: bool,
    ) -> Result<(), String> {
        if merge_buckets {
            self.merge_converged_unfrozen_buckets()?;
        }
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

    pub(super) fn merge_converged_unfrozen_buckets(&mut self) -> Result<(), String> {
        let max_receipts = self.checkpoint.config.max_receipts_per_bucket;
        let mut index = 0_usize;
        while index < self.checkpoint.buckets.len() {
            if self.checkpoint.buckets[index]
                .frozen_program_sha256
                .is_some()
                || self.checkpoint.buckets[index].programs.is_empty()
            {
                index = index.saturating_add(1);
                continue;
            }
            loop {
                let merge = ((index + 1)..self.checkpoint.buckets.len()).find_map(|candidate| {
                    let bucket = &self.checkpoint.buckets[candidate];
                    let compatible = bucket.frozen_program_sha256.is_none()
                        && bucket.archetype_id == self.checkpoint.buckets[index].archetype_id
                        && buckets_share_execution_law(&self.checkpoint.buckets[index], bucket);
                    if !compatible {
                        return None;
                    }
                    let programs = self.checkpoint.buckets[index]
                        .programs
                        .iter()
                        .chain(&bucket.programs)
                        .map(|(digest, program)| (digest.clone(), program.clone()))
                        .collect::<BTreeMap<_, _>>();
                    let receipts = self.checkpoint.buckets[index]
                        .support
                        .iter()
                        .chain(&bucket.support)
                        .cloned()
                        .collect::<Vec<_>>();
                    select_program_receipt_cover(
                        &programs,
                        &receipts,
                        crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS,
                    )
                    .map(|selected| (candidate, selected))
                });
                let Some((duplicate, selected_programs)) = merge else {
                    break;
                };
                let other = self.checkpoint.buckets.remove(duplicate);
                let bucket = &mut self.checkpoint.buckets[index];
                bucket.programs.extend(other.programs);
                bucket
                    .programs
                    .retain(|digest, _| selected_programs.contains(digest));
                bucket
                    .common_request_atom_ids
                    .retain(|atom| other.common_request_atom_ids.contains(atom));
                merge_receipts(&mut bucket.support, other.support, max_receipts);
                merge_receipts(&mut bucket.future, other.future, max_receipts);
                for receipt in bucket.support.iter_mut().chain(&mut bucket.future) {
                    receipt
                        .matched_program_sha256
                        .retain(|digest| selected_programs.contains(digest));
                }
                for (digest, example) in other.runtime_examples {
                    bucket.runtime_examples.entry(digest).or_insert(example);
                }
                for (digest, receipt) in other.durable_runtime_parity_receipts {
                    bucket
                        .durable_runtime_parity_receipts
                        .entry(digest)
                        .or_insert(receipt);
                }
                trim_runtime_examples(&mut bucket.runtime_examples, max_receipts);
                let future_refs = bucket
                    .future
                    .iter()
                    .map(|receipt| receipt.evidence_graph_sha256.as_str())
                    .collect::<BTreeSet<_>>();
                bucket
                    .durable_runtime_parity_receipts
                    .retain(|evidence_ref, _| future_refs.contains(evidence_ref.as_str()));
                bucket
                    .rejected_program_sha256
                    .extend(other.rejected_program_sha256);
                bucket
                    .learned_anti_atom_ids
                    .extend(other.learned_anti_atom_ids);
                bucket.wrong_accepts = bucket.wrong_accepts.saturating_add(other.wrong_accepts);
            }
            self.checkpoint.buckets[index].bucket_id = collection_archetype_bucket_id(
                &self.checkpoint.buckets[index].archetype_id,
                self.checkpoint.buckets[index].programs.keys(),
            )?;
            self.normalize_bucket_receipts(index);
            self.freeze_or_split(index)?;
            index = index.saturating_add(1);
        }
        Ok(())
    }

    pub(super) fn deduplicate_exact_unfrozen_buckets(&mut self) -> Result<(), String> {
        let mut keepers = BTreeMap::<String, (usize, String)>::new();
        let mut remove = BTreeSet::<usize>::new();
        for (index, bucket) in self.checkpoint.buckets.iter().enumerate() {
            if bucket.frozen_program_sha256.is_some() {
                continue;
            }
            let fingerprint = canonical_json_sha256(&(
                "nando.collection-unfrozen-proof-state.v1",
                &bucket.programs,
                &bucket.common_request_atom_ids,
                &bucket.support,
                &bucket.future,
                &bucket.runtime_examples,
                &bucket.durable_runtime_parity_receipts,
                &bucket.rejected_program_sha256,
                &bucket.learned_anti_atom_ids,
                bucket.wrong_accepts,
            ))
            .map_err(str::to_owned)?;
            match keepers.get(&fingerprint) {
                Some((keeper_index, keeper_id)) if bucket.bucket_id < *keeper_id => {
                    remove.insert(*keeper_index);
                    keepers.insert(fingerprint, (index, bucket.bucket_id.clone()));
                }
                Some(_) => {
                    remove.insert(index);
                }
                None => {
                    keepers.insert(fingerprint, (index, bucket.bucket_id.clone()));
                }
            }
        }
        for index in remove.into_iter().rev() {
            self.checkpoint.buckets.remove(index);
        }
        Ok(())
    }

    pub(super) fn persist_if(&self, durable: bool) -> Result<(), String> {
        if durable { self.persist() } else { Ok(()) }
    }
}
