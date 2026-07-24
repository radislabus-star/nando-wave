//! Support subcenters, active witnesses, and checkpoint structural validation.

use super::*;

pub(super) fn validate_config(config: OnlineCollectionConfig) -> Result<(), String> {
    if config.support_rows == 0
        || config.future_rows == 0
        || config.max_buckets == 0
        || config.max_receipts_per_bucket < config.support_rows.max(config.future_rows)
    {
        return Err("online_collection_invalid_config".to_owned());
    }
    Ok(())
}

pub(super) fn support_program_subcenters(
    bucket: &OnlineCollectionBucket,
    required_support_rows: usize,
    max_receipts_per_bucket: usize,
) -> Result<Vec<OnlineCollectionBucket>, String> {
    let mut ranked = Vec::new();
    for (program_sha256, program) in &bucket.programs {
        if !is_source_neutral_response_program(program) {
            continue;
        }
        let mut support = bucket
            .support
            .iter()
            .filter(|receipt| {
                receipt.verifier_pass
                    && receipt
                        .matched_program_sha256
                        .iter()
                        .any(|matched| matched == program_sha256)
            })
            .cloned()
            .collect::<Vec<_>>();
        if support.len() < required_support_rows {
            continue;
        }
        support.truncate(max_receipts_per_bucket);
        for receipt in &mut support {
            receipt.matched_program_sha256 = vec![program_sha256.clone()];
        }
        let mut common_request_atom_ids = support.first().map_or_else(BTreeSet::new, |receipt| {
            receipt.request_atom_ids.iter().copied().collect()
        });
        for receipt in support.iter().skip(1) {
            common_request_atom_ids
                .retain(|candidate| receipt.request_atom_ids.binary_search(candidate).is_ok());
        }
        let support_ids = support
            .iter()
            .map(|receipt| receipt.evidence_graph_sha256.clone())
            .collect::<BTreeSet<_>>();
        let bucket_id = canonical_json_sha256(&(
            "nando.collection-support-program-subcenter.v1",
            &bucket.archetype_id,
            program_sha256,
        ))
        .map_err(str::to_owned)?;
        let archetype_id = canonical_json_sha256(&(
            "nando.collection-support-program-subcenter-archetype.v1",
            &bucket.archetype_id,
            program_sha256,
        ))
        .map_err(str::to_owned)?;
        let program_bytes = serde_json::to_vec(program).map_or(usize::MAX, |value| value.len());
        ranked.push((
            support.len(),
            program_bytes,
            program_sha256.clone(),
            OnlineCollectionBucket {
                bucket_id,
                archetype_id,
                programs: BTreeMap::from([(program_sha256.clone(), program.clone())]),
                common_request_atom_ids,
                support,
                future: Vec::new(),
                runtime_examples: bucket
                    .runtime_examples
                    .iter()
                    .filter(|(evidence_id, _)| support_ids.contains(*evidence_id))
                    .map(|(evidence_id, example)| (evidence_id.clone(), example.clone()))
                    .collect(),
                durable_adapter_phase_atoms: durable_adapter_phase_subset(
                    bucket,
                    &support_ids,
                    &BTreeSet::from([program_sha256.clone()]),
                ),
                durable_runtime_parity_receipts: BTreeMap::new(),
                adaptive_candidate_freeze: None,
                frozen_program_sha256: None,
                support_watermark_event_time_unix_nanos: None,
                support_manifest_sha256: None,
                rejected_program_sha256: BTreeSet::new(),
                learned_anti_atom_ids: BTreeSet::new(),
                wrong_accepts: 0,
            },
        ));
    }
    ranked.sort_by(
        |(left_rows, left_bytes, left_digest, _), (right_rows, right_bytes, right_digest, _)| {
            right_rows
                .cmp(left_rows)
                .then_with(|| left_bytes.cmp(right_bytes))
                .then_with(|| left_digest.cmp(right_digest))
        },
    );
    Ok(ranked
        .into_iter()
        .map(|(_, _, _, subcenter)| subcenter)
        .collect())
}

pub(super) fn support_law_subcenters(
    bucket: &OnlineCollectionBucket,
    required_support_rows: usize,
    max_receipts_per_bucket: usize,
) -> Result<Vec<OnlineCollectionBucket>, String> {
    let mut law_groups = BTreeMap::<Vec<u8>, BTreeMap<String, ResponseProgram>>::new();
    for (program_sha256, program) in &bucket.programs {
        if !is_privacy_safe_online_response_program(program)
            || !is_learned_bounded_response_program(program)
        {
            continue;
        }
        let law_key = response_law_key(program).map_err(str::to_owned)?;
        law_groups
            .entry(law_key)
            .or_default()
            .entry(program_sha256.clone())
            .or_insert_with(|| program.clone());
    }

    let mut ranked = Vec::new();
    for (law_key, adapters) in law_groups {
        let mut support = Vec::new();
        for receipt in &bucket.support {
            if !receipt.verifier_pass {
                continue;
            }
            // matched_program_sha256 is written only after the program has
            // reproduced the complete teacher response. Requiring the raw
            // example again made a valid proof disappear after restart.
            let matched_program_sha256 = adapters
                .iter()
                .filter(|(program_sha256, _)| {
                    receipt
                        .matched_program_sha256
                        .iter()
                        .any(|digest| digest == *program_sha256)
                })
                .map(|(program_sha256, _)| program_sha256.clone())
                .collect::<Vec<_>>();
            if matched_program_sha256.is_empty() {
                continue;
            }
            let mut canonical_receipt = receipt.clone();
            canonical_receipt.matched_program_sha256 = matched_program_sha256;
            support.push(canonical_receipt);
        }
        if support.len() < required_support_rows {
            continue;
        }
        support.truncate(max_receipts_per_bucket);
        let selected_adapter_digests = support
            .iter()
            .flat_map(|receipt| receipt.matched_program_sha256.iter().cloned())
            .collect::<BTreeSet<_>>();
        let programs = adapters
            .into_iter()
            .filter(|(digest, _)| selected_adapter_digests.contains(digest))
            .collect::<BTreeMap<_, _>>();
        if programs.is_empty() {
            continue;
        }
        let mut common_request_atom_ids = support.first().map_or_else(BTreeSet::new, |receipt| {
            receipt.request_atom_ids.iter().copied().collect()
        });
        for receipt in support.iter().skip(1) {
            common_request_atom_ids
                .retain(|candidate| receipt.request_atom_ids.binary_search(candidate).is_ok());
        }
        let support_ids = support
            .iter()
            .map(|receipt| receipt.evidence_graph_sha256.clone())
            .collect::<BTreeSet<_>>();
        let parent_support_ids = bucket
            .support
            .iter()
            .map(|receipt| receipt.evidence_graph_sha256.clone())
            .collect::<BTreeSet<_>>();
        let selected_program_ids = programs.keys().cloned().collect::<BTreeSet<_>>();
        let parent_program_ids = bucket.programs.keys().cloned().collect::<BTreeSet<_>>();
        if support_ids == parent_support_ids && selected_program_ids == parent_program_ids {
            continue;
        }
        let law_commitment_sha256 = sha256_bytes(&law_key);
        let bucket_id = canonical_json_sha256(&(
            "nando.collection-support-law-subcenter.v1",
            &bucket.archetype_id,
            &law_commitment_sha256,
        ))
        .map_err(str::to_owned)?;
        let archetype_id = canonical_json_sha256(&(
            "nando.collection-support-law-subcenter-archetype.v1",
            &bucket.archetype_id,
            &law_commitment_sha256,
        ))
        .map_err(str::to_owned)?;
        let program_bytes = programs
            .values()
            .map(|program| serde_json::to_vec(program).map_or(usize::MAX, |value| value.len()))
            .sum::<usize>();
        let rank_digest = programs.keys().next().cloned().unwrap_or_default();
        ranked.push((
            support.len(),
            program_bytes,
            rank_digest,
            OnlineCollectionBucket {
                bucket_id,
                archetype_id,
                programs,
                common_request_atom_ids,
                support,
                future: Vec::new(),
                runtime_examples: bucket
                    .runtime_examples
                    .iter()
                    .filter(|(evidence_id, _)| support_ids.contains(*evidence_id))
                    .map(|(evidence_id, example)| (evidence_id.clone(), example.clone()))
                    .collect(),
                durable_adapter_phase_atoms: durable_adapter_phase_subset(
                    bucket,
                    &support_ids,
                    &selected_program_ids,
                ),
                durable_runtime_parity_receipts: BTreeMap::new(),
                adaptive_candidate_freeze: None,
                frozen_program_sha256: None,
                support_watermark_event_time_unix_nanos: None,
                support_manifest_sha256: None,
                rejected_program_sha256: BTreeSet::new(),
                learned_anti_atom_ids: BTreeSet::new(),
                wrong_accepts: 0,
            },
        ));
    }
    ranked.sort_by(
        |(left_rows, left_bytes, left_digest, _), (right_rows, right_bytes, right_digest, _)| {
            right_rows
                .cmp(left_rows)
                .then_with(|| left_bytes.cmp(right_bytes))
                .then_with(|| left_digest.cmp(right_digest))
        },
    );
    Ok(ranked
        .into_iter()
        .map(|(_, _, _, subcenter)| subcenter)
        .collect())
}

pub(super) fn maximal_decidable_support_subcenter(
    bucket: &OnlineCollectionBucket,
    required_support_rows: usize,
    max_receipts_per_bucket: usize,
) -> Result<Option<OnlineCollectionBucket>, String> {
    if bucket.support.len() < required_support_rows || bucket.wrong_accepts > 0 {
        return Ok(None);
    }
    match support_consensus_candidate(bucket)? {
        SupportConsensusCandidate::Ready(_) => return Ok(None),
        SupportConsensusCandidate::Blocked(
            "support_phase_adapter_unproven"
            | "support_layout_adapter_unproven"
            | "support_consensus_variant_budget_exceeded",
        ) => {}
        SupportConsensusCandidate::Blocked(_) => return Ok(None),
    }

    let mut by_layout = BTreeMap::<String, Vec<OnlineCollectionReceipt>>::new();
    for receipt in &bucket.support {
        by_layout
            .entry(receipt.layout_sha256.clone())
            .or_default()
            .push(receipt.clone());
    }
    let mut layout_groups = by_layout.into_iter().collect::<Vec<_>>();
    layout_groups.sort_by(|(left_layout, left), (right_layout, right)| {
        right
            .len()
            .cmp(&left.len())
            .then_with(|| left_layout.cmp(right_layout))
    });

    let mut selected_support = Vec::new();
    for (_, layout_support) in layout_groups {
        let common_adapter_exists = bucket.programs.keys().any(|digest| {
            layout_support.iter().all(|receipt| {
                receipt.verifier_pass && receipt.matched_program_sha256.contains(digest)
            })
        });
        if common_adapter_exists {
            selected_support.extend(layout_support);
        }
    }
    selected_support.sort_by(|left, right| {
        left.event_time_unix_nanos
            .cmp(&right.event_time_unix_nanos)
            .then_with(|| left.evidence_graph_sha256.cmp(&right.evidence_graph_sha256))
    });
    if selected_support.len() > max_receipts_per_bucket {
        selected_support.drain(
            ..selected_support
                .len()
                .saturating_sub(max_receipts_per_bucket),
        );
    }
    if selected_support.len() < required_support_rows {
        return Ok(None);
    }

    let selected_layouts = selected_support
        .iter()
        .map(|receipt| receipt.layout_sha256.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut child = support_subset_bucket(bucket, selected_support);
    child.bucket_id = canonical_json_sha256(&(
        "nando.collection-maximal-decidable-subcenter.v1",
        &bucket.archetype_id,
        &selected_layouts,
    ))
    .map_err(str::to_owned)?;
    child.archetype_id = canonical_json_sha256(&(
        "nando.collection-maximal-decidable-subcenter-archetype.v1",
        &bucket.archetype_id,
        &selected_layouts,
    ))
    .map_err(str::to_owned)?;
    if !matches!(
        support_consensus_candidate(&child)?,
        SupportConsensusCandidate::Ready(_)
    ) {
        return Ok(None);
    }
    Ok(Some(child))
}

pub(super) fn support_subset_bucket(
    bucket: &OnlineCollectionBucket,
    support: Vec<OnlineCollectionReceipt>,
) -> OnlineCollectionBucket {
    let support_ids = support
        .iter()
        .map(|receipt| receipt.evidence_graph_sha256.clone())
        .collect::<BTreeSet<_>>();
    let mut common_request_atom_ids = support.first().map_or_else(BTreeSet::new, |receipt| {
        receipt.request_atom_ids.iter().copied().collect()
    });
    for receipt in support.iter().skip(1) {
        common_request_atom_ids
            .retain(|candidate| receipt.request_atom_ids.binary_search(candidate).is_ok());
    }
    OnlineCollectionBucket {
        bucket_id: bucket.bucket_id.clone(),
        archetype_id: bucket.archetype_id.clone(),
        programs: bucket.programs.clone(),
        common_request_atom_ids,
        support,
        future: Vec::new(),
        runtime_examples: bucket
            .runtime_examples
            .iter()
            .filter(|(evidence_id, _)| support_ids.contains(*evidence_id))
            .map(|(evidence_id, example)| (evidence_id.clone(), example.clone()))
            .collect(),
        durable_adapter_phase_atoms: durable_adapter_phase_subset(
            bucket,
            &support_ids,
            &bucket.programs.keys().cloned().collect(),
        ),
        durable_runtime_parity_receipts: BTreeMap::new(),
        adaptive_candidate_freeze: None,
        frozen_program_sha256: None,
        support_watermark_event_time_unix_nanos: None,
        support_manifest_sha256: None,
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::new(),
        wrong_accepts: 0,
    }
}

pub(super) fn clean_pre_action_program_subcenter(
    bucket: &OnlineCollectionBucket,
    required_support_rows: usize,
    max_receipts_per_bucket: usize,
) -> Result<Option<OnlineCollectionBucket>, String> {
    let rows = bucket
        .support
        .iter()
        .filter_map(|receipt| {
            bucket
                .runtime_examples
                .get(&receipt.evidence_graph_sha256)
                .map(|example| {
                    let mut atoms = request_atoms_for_example(example).unwrap_or_default();
                    atoms.extend(response_pre_action_context_atom_ids(
                        &example.provider_payload,
                    ));
                    (receipt, atoms)
                })
        })
        .collect::<Vec<_>>();
    if rows.len() < required_support_rows {
        return Ok(None);
    }

    let mut best = None::<(u64, usize, Vec<u64>, String, Vec<usize>)>;
    for digest in bucket.programs.keys() {
        let positive_indices = rows
            .iter()
            .enumerate()
            .filter(|(_, (receipt, _))| {
                receipt.verifier_pass && receipt.matched_program_sha256.contains(digest)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if positive_indices.len() < required_support_rows {
            continue;
        }
        let mut frequencies = BTreeMap::<u64, usize>::new();
        for index in &positive_indices {
            for atom in &rows[*index].1 {
                *frequencies.entry(*atom).or_default() += 1;
            }
        }
        let mut atoms = frequencies
            .into_iter()
            .filter(|(_, count)| *count >= required_support_rows)
            .collect::<Vec<_>>();
        atoms.sort_by(|(left_atom, left), (right_atom, right)| {
            right.cmp(left).then_with(|| left_atom.cmp(right_atom))
        });
        atoms.truncate(32);
        let atoms = atoms.into_iter().map(|(atom, _)| atom).collect::<Vec<_>>();

        let mut evaluate = |required_atoms: &[u64]| {
            let selected = rows
                .iter()
                .enumerate()
                .filter(|(_, (_, row_atoms))| {
                    required_atoms.iter().all(|atom| row_atoms.contains(atom))
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if selected.len() < required_support_rows
                || selected.iter().any(|index| {
                    let receipt = rows[*index].0;
                    !receipt.verifier_pass || !receipt.matched_program_sha256.contains(digest)
                })
            {
                return;
            }
            let tokens = selected.iter().fold(0_u64, |total, index| {
                total.saturating_add(rows[*index].0.estimated_input_tokens)
            });
            let candidate = (
                tokens,
                selected.len(),
                required_atoms.to_vec(),
                digest.clone(),
                selected,
            );
            let replace = best.as_ref().is_none_or(|current| {
                candidate.0 > current.0
                    || (candidate.0 == current.0 && candidate.1 > current.1)
                    || (candidate.0 == current.0
                        && candidate.1 == current.1
                        && candidate.2.len() < current.2.len())
                    || (candidate.0 == current.0
                        && candidate.1 == current.1
                        && candidate.2.len() == current.2.len()
                        && (&candidate.2, &candidate.3) < (&current.2, &current.3))
            });
            if replace {
                best = Some(candidate);
            }
        };
        for left in 0..atoms.len() {
            evaluate(&atoms[left..=left]);
            for right in left + 1..atoms.len() {
                evaluate(&[atoms[left], atoms[right]]);
                for third in right + 1..atoms.len() {
                    evaluate(&[atoms[left], atoms[right], atoms[third]]);
                }
            }
        }
    }

    let Some((_, _, required_atoms, digest, mut selected)) = best else {
        return Ok(None);
    };
    selected.sort_by_key(|index| {
        (
            rows[*index].0.event_time_unix_nanos,
            rows[*index].0.evidence_graph_sha256.as_str(),
        )
    });
    if selected.len() > max_receipts_per_bucket {
        selected.drain(..selected.len().saturating_sub(max_receipts_per_bucket));
    }
    let mut support = selected
        .into_iter()
        .map(|index| {
            let mut receipt = rows[index].0.clone();
            receipt.matched_program_sha256 = vec![digest.clone()];
            receipt
        })
        .collect::<Vec<_>>();
    support.sort_by(|left, right| {
        left.event_time_unix_nanos
            .cmp(&right.event_time_unix_nanos)
            .then_with(|| left.evidence_graph_sha256.cmp(&right.evidence_graph_sha256))
    });
    let mut child = support_subset_bucket(bucket, support);
    child.programs.retain(|candidate, _| candidate == &digest);
    child.common_request_atom_ids = required_atoms.iter().copied().collect();
    child.bucket_id = canonical_json_sha256(&(
        "nando.collection-clean-pre-action-subcenter.v1",
        &bucket.archetype_id,
        &digest,
        &required_atoms,
    ))
    .map_err(str::to_owned)?;
    child.archetype_id = canonical_json_sha256(&(
        "nando.collection-clean-pre-action-subcenter-archetype.v1",
        &bucket.archetype_id,
        &digest,
        &required_atoms,
    ))
    .map_err(str::to_owned)?;
    if !matches!(
        support_consensus_candidate(&child)?,
        SupportConsensusCandidate::Ready(_)
    ) {
        return Err("online_collection_pre_action_subcenter_not_ready".to_owned());
    }
    Ok(Some(child))
}

pub(super) enum ActiveWitnessDecision {
    Successor {
        bucket: Box<OnlineCollectionBucket>,
        resolved: bool,
    },
    Pending,
    Irreducible,
}

pub(super) fn active_witness_decision(
    bucket: &OnlineCollectionBucket,
    program_sha256: &str,
    observation: &OnlineCollectionObservation,
    max_receipts: usize,
) -> Result<ActiveWitnessDecision, String> {
    let Some(program) = bucket.programs.get(program_sha256) else {
        return Err("online_collection_witness_program_missing".to_owned());
    };
    let crate::ResponseOperation::UniqueConsensus { variants, .. } = &program.operation else {
        return Ok(ActiveWitnessDecision::Irreducible);
    };
    if variants.len() < 2 {
        return Ok(ActiveWitnessDecision::Irreducible);
    }
    let next_round = bucket
        .support
        .iter()
        .filter_map(|receipt| receipt.witness_round)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    if next_round > MAX_ACTIVE_WITNESS_ROUNDS {
        return Ok(ActiveWitnessDecision::Irreducible);
    }

    let mut candidates = BTreeMap::new();
    for variant in variants {
        let digest = canonical_json_sha256(&variant.program).map_err(str::to_owned)?;
        candidates
            .entry(digest)
            .or_insert_with(|| variant.program.clone());
    }
    if candidates.len() < 2 {
        return Ok(ActiveWitnessDecision::Irreducible);
    }
    let survivors = candidates
        .iter()
        .filter(|(_, candidate)| {
            independently_verified_authority_response(candidate, &observation.example)
                .is_some_and(|response| response == observation.example.expected_response)
        })
        .map(|(digest, candidate)| (digest.clone(), candidate.clone()))
        .collect::<BTreeMap<_, _>>();
    if survivors.is_empty() {
        return Ok(ActiveWitnessDecision::Irreducible);
    }
    if survivors.len() == candidates.len() {
        return Ok(ActiveWitnessDecision::Pending);
    }

    let candidate_digests = candidates.keys().cloned().collect::<Vec<_>>();
    let survivor_digests = survivors.keys().cloned().collect::<BTreeSet<_>>();
    let class_commitment_sha256 = canonical_json_sha256(&(
        "nando.collection-active-witness-class.v1",
        &bucket.bucket_id,
        program_sha256,
        &bucket.support_manifest_sha256,
        &candidate_digests,
    ))
    .map_err(str::to_owned)?;
    let mut support = bucket
        .support
        .iter()
        .filter_map(|receipt| {
            let mut receipt = receipt.clone();
            receipt
                .matched_program_sha256
                .retain(|digest| survivor_digests.contains(digest));
            (!receipt.matched_program_sha256.is_empty()).then_some(receipt)
        })
        .collect::<Vec<_>>();
    let mut witness = receipt_with_program_atoms(observation, true, &survivors)?;
    witness.witness_class_commitment_sha256 = Some(class_commitment_sha256.clone());
    witness.witness_round = Some(next_round);
    witness.witness_candidates_before = Some(candidates.len());
    witness.witness_candidates_after = Some(survivors.len());
    push_bounded(&mut support, witness, max_receipts);

    let mut common_request_atom_ids = support.first().map_or_else(BTreeSet::new, |receipt| {
        receipt.request_atom_ids.iter().copied().collect()
    });
    for receipt in support.iter().skip(1) {
        common_request_atom_ids.retain(|atom| receipt.request_atom_ids.binary_search(atom).is_ok());
    }
    let successor_id = canonical_json_sha256(&(
        "nando.collection-active-witness-successor.v1",
        &bucket.bucket_id,
        &class_commitment_sha256,
        &observation.evidence_graph_sha256,
        survivors.keys().collect::<Vec<_>>(),
    ))
    .map_err(str::to_owned)?;
    let successor_archetype_id = canonical_json_sha256(&(
        "nando.collection-active-witness-successor-archetype.v1",
        &bucket.archetype_id,
        &class_commitment_sha256,
    ))
    .map_err(str::to_owned)?;
    let support_ids = support
        .iter()
        .map(|receipt| receipt.evidence_graph_sha256.clone())
        .collect::<BTreeSet<_>>();
    let survivor_ids = survivors.keys().cloned().collect::<BTreeSet<_>>();
    let mut successor = OnlineCollectionBucket {
        bucket_id: successor_id,
        archetype_id: successor_archetype_id,
        programs: survivors,
        common_request_atom_ids,
        support,
        future: Vec::new(),
        runtime_examples: BTreeMap::from([(
            observation.evidence_graph_sha256.clone(),
            observation.example.clone(),
        )]),
        durable_adapter_phase_atoms: durable_adapter_phase_subset(
            bucket,
            &support_ids,
            &survivor_ids,
        ),
        durable_runtime_parity_receipts: BTreeMap::new(),
        adaptive_candidate_freeze: None,
        frozen_program_sha256: None,
        support_watermark_event_time_unix_nanos: None,
        support_manifest_sha256: None,
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::new(),
        wrong_accepts: 0,
    };
    refresh_durable_adapter_phase_atoms(&mut successor);
    Ok(ActiveWitnessDecision::Successor {
        resolved: successor.programs.len() == 1,
        bucket: Box::new(successor),
    })
}

pub(super) fn revoke_frozen_bucket(bucket: &mut OnlineCollectionBucket, program_sha256: &str) {
    let rejected = bucket_adapter_digests(bucket);
    bucket.adaptive_candidate_freeze = None;
    bucket.frozen_program_sha256 = None;
    bucket.support_watermark_event_time_unix_nanos = None;
    bucket.support_manifest_sha256 = None;
    bucket.programs.clear();
    bucket.rejected_program_sha256.extend(rejected);
    bucket
        .rejected_program_sha256
        .insert(program_sha256.to_owned());
}

pub(super) fn counterexample_subcenters(
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
            partition_sha256.clone(),
        ))
        .map_err(str::to_owned)?;
        let archetype_id = canonical_json_sha256(&(
            "nando.collection-cegis-subcenter-archetype.v1",
            &bucket.archetype_id,
            program_sha256,
            atom,
            &partition_sha256,
        ))
        .map_err(str::to_owned)?;
        let support_ids = support
            .iter()
            .map(|receipt| receipt.evidence_graph_sha256.clone())
            .collect::<BTreeSet<_>>();
        output.push(OnlineCollectionBucket {
            bucket_id,
            archetype_id,
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
            durable_adapter_phase_atoms: durable_adapter_phase_subset(
                bucket,
                &support_ids,
                &BTreeSet::from([program_sha256.to_owned()]),
            ),
            durable_runtime_parity_receipts: BTreeMap::new(),
            adaptive_candidate_freeze: None,
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

pub(super) fn validate_checkpoint(
    checkpoint: &OnlineCollectionCheckpoint,
    config: OnlineCollectionConfig,
) -> Result<(), String> {
    if checkpoint.schema != ONLINE_COLLECTION_SCHEMA_V3
        || checkpoint.pooling_strategy_version != ONLINE_COLLECTION_POOLING_STRATEGY_V37
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
        if let Some(reason) = invalid_collection_bucket_reason(bucket) {
            return Err(format!(
                "online_collection_checkpoint_program_invalid:{}:{reason}",
                bucket.bucket_id
            ));
        }
    }
    Ok(())
}

pub(super) fn migrate_collection_active_witness_pools(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<(), String> {
    let mut discarded_support = 0_u64;
    for bucket in &mut checkpoint.buckets {
        let known_digests = bucket_adapter_digests(bucket);
        for digest in bucket
            .support
            .iter()
            .chain(bucket.future.iter())
            .flat_map(|receipt| receipt.matched_program_sha256.iter())
            .filter(|digest| !known_digests.contains(*digest))
        {
            bucket.rejected_program_sha256.insert(digest.clone());
        }

        let invalid_programs = bucket
            .programs
            .iter()
            .filter_map(|(digest, program)| {
                let valid = canonical_json_sha256(program).ok().as_ref() == Some(digest)
                    && program.validate().is_ok()
                    && is_privacy_safe_online_response_program(program);
                (!valid).then_some(digest.clone())
            })
            .collect::<Vec<_>>();
        for digest in invalid_programs {
            bucket.programs.remove(&digest);
            bucket.rejected_program_sha256.insert(digest);
        }

        let support_before = bucket.support.len();
        bucket.support.retain(valid_witness_receipt_metadata);
        discarded_support = discarded_support
            .saturating_add(support_before.saturating_sub(bucket.support.len()) as u64);
        bucket.future.retain(valid_witness_receipt_metadata);

        if bucket.archetype_id.is_empty() {
            bucket.archetype_id = bucket
                .programs
                .values()
                .next()
                .map(response_program_archetype_id)
                .transpose()?
                .unwrap_or_else(|| format!("rejected: {}", bucket.bucket_id));
        }

        let frozen_valid = bucket.frozen_program_sha256.as_ref().is_some_and(|digest| {
            bucket.programs.contains_key(digest)
                && bucket.support_watermark_event_time_unix_nanos.is_some()
                && bucket.support.iter().all(|receipt| {
                    receipt.event_time_unix_nanos.is_some_and(|event_time| {
                        bucket
                            .support_watermark_event_time_unix_nanos
                            .is_some_and(|watermark| event_time <= watermark)
                    })
                })
                && collection_support_manifest_digest(bucket).ok().as_ref()
                    == bucket.support_manifest_sha256.as_ref()
        });
        if bucket.frozen_program_sha256.is_some() && !frozen_valid {
            bucket.future.clear();
            bucket.durable_runtime_parity_receipts.clear();
            bucket.adaptive_candidate_freeze = None;
            bucket.frozen_program_sha256 = None;
            bucket.support_watermark_event_time_unix_nanos = None;
            bucket.support_manifest_sha256 = None;
        } else if bucket.frozen_program_sha256.is_none() {
            bucket.adaptive_candidate_freeze = None;
            bucket.support_manifest_sha256 = None;
        }
    }
    checkpoint.unreplayable_support_discarded_total = checkpoint
        .unreplayable_support_discarded_total
        .saturating_add(discarded_support);
    Ok(())
}

pub(super) fn migrate_collection_exact_receipts(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<(), String> {
    let mut discarded_support = 0_u64;
    for bucket in &mut checkpoint.buckets {
        let previous_programs = std::mem::take(&mut bucket.programs);
        let mut exact_programs = BTreeMap::new();
        for example in bucket
            .runtime_examples
            .values()
            .take(MAX_EXACT_RECEIPT_MIGRATION_SEEDS_PER_BUCKET)
        {
            let Ok(space) = enumerate_source_neutral_response_programs(example) else {
                continue;
            };
            for program in space.programs {
                if independently_verified_authority_response(&program, example).as_deref()
                    != Some(example.expected_response.as_str())
                    || !is_privacy_safe_online_response_program(&program)
                    || response_program_archetype_id(&program)? != bucket.archetype_id
                {
                    continue;
                }
                let digest = canonical_json_sha256(&program).map_err(str::to_owned)?;
                exact_programs.entry(digest).or_insert(program);
            }
        }
        if exact_programs.is_empty() {
            bucket.programs = previous_programs;
        } else {
            bucket.rejected_program_sha256.extend(
                previous_programs
                    .keys()
                    .filter(|digest| !exact_programs.contains_key(*digest))
                    .cloned(),
            );
            bucket.programs = bounded_program_map(
                exact_programs,
                crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS,
            );
        }

        let previous_support = std::mem::take(&mut bucket.support);
        for old_receipt in previous_support {
            let Some(example) = bucket
                .runtime_examples
                .get(&old_receipt.evidence_graph_sha256)
            else {
                discarded_support = discarded_support.saturating_add(1);
                continue;
            };
            let observation = OnlineCollectionObservation {
                evidence_graph_sha256: old_receipt.evidence_graph_sha256.clone(),
                client_intent_id_sha256: old_receipt.client_intent_id_sha256.clone(),
                session_id_sha256: old_receipt.session_id_sha256.clone(),
                event_time_unix_nanos: old_receipt.event_time_unix_nanos,
                estimated_input_tokens: old_receipt.estimated_input_tokens,
                example: example.clone(),
            };
            let mut rebuilt = receipt_with_program_atoms(&observation, true, &bucket.programs)?;
            if rebuilt.matched_program_sha256.is_empty() {
                discarded_support = discarded_support.saturating_add(1);
                continue;
            }
            rebuilt.witness_class_commitment_sha256 = old_receipt.witness_class_commitment_sha256;
            rebuilt.witness_round = old_receipt.witness_round;
            rebuilt.witness_candidates_before = old_receipt.witness_candidates_before;
            rebuilt.witness_candidates_after = old_receipt.witness_candidates_after;
            bucket.support.push(rebuilt);
        }
        bucket.common_request_atom_ids = bucket
            .support
            .first()
            .map_or_else(BTreeSet::new, |receipt| {
                receipt.request_atom_ids.iter().copied().collect()
            });
        for receipt in bucket.support.iter().skip(1) {
            bucket
                .common_request_atom_ids
                .retain(|atom| receipt.request_atom_ids.binary_search(atom).is_ok());
        }
        bucket.future.clear();
        bucket.durable_runtime_parity_receipts.clear();
        bucket.adaptive_candidate_freeze = None;
        bucket.frozen_program_sha256 = None;
        bucket.support_watermark_event_time_unix_nanos = None;
        bucket.support_manifest_sha256 = None;
        bucket.learned_anti_atom_ids.clear();
        bucket.wrong_accepts = 0;
    }
    checkpoint.unreplayable_support_discarded_total = checkpoint
        .unreplayable_support_discarded_total
        .saturating_add(discarded_support);
    Ok(())
}

pub(super) fn migrate_collection_relational_role_programs(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<(), String> {
    let required_support_rows = checkpoint.config.support_rows;
    let mut authority_cache = BTreeMap::<(String, String), bool>::new();
    for bucket in &mut checkpoint.buckets {
        if bucket.frozen_program_sha256.is_some() || bucket.runtime_examples.is_empty() {
            continue;
        }
        let law_keys = bucket
            .programs
            .values()
            .filter_map(|program| response_law_key(program).ok())
            .collect::<BTreeSet<_>>();
        let mut relational = BTreeMap::<String, ResponseProgram>::new();
        let mut candidates = Vec::new();
        for program in bucket.programs.values() {
            collect_relational_role_programs(program, &mut candidates);
        }
        for canonical in candidates {
            if !is_privacy_safe_online_response_program(&canonical)
                || response_law_key(&canonical)
                    .ok()
                    .is_none_or(|law| !law_keys.contains(&law))
            {
                continue;
            }
            let digest = canonical_json_sha256(&canonical).map_err(str::to_owned)?;
            let support = bucket
                .support
                .iter()
                .filter(|receipt| {
                    let Some(example) = bucket.runtime_examples.get(&receipt.evidence_graph_sha256)
                    else {
                        return false;
                    };
                    *authority_cache
                        .entry((receipt.evidence_graph_sha256.clone(), digest.clone()))
                        .or_insert_with(|| {
                            response_program_authority_matches_example(&canonical, example)
                        })
                })
                .count();
            if support >= required_support_rows {
                relational.entry(digest).or_insert(canonical);
            }
        }
        if relational.is_empty() {
            continue;
        }
        for (digest, program) in &relational {
            bucket.programs.insert(digest.clone(), program.clone());
        }
        for receipt in &mut bucket.support {
            let Some(example) = bucket.runtime_examples.get(&receipt.evidence_graph_sha256) else {
                continue;
            };
            for (digest, program) in &relational {
                if *authority_cache
                    .entry((receipt.evidence_graph_sha256.clone(), digest.clone()))
                    .or_insert_with(|| response_program_authority_matches_example(program, example))
                {
                    receipt.matched_program_sha256.push(digest.clone());
                }
            }
            receipt.matched_program_sha256.sort();
            receipt.matched_program_sha256.dedup();
        }
        while bucket.programs.len() > crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS {
            let relational_digests = relational.keys().collect::<BTreeSet<_>>();
            let Some(evicted) = bucket
                .programs
                .keys()
                .filter(|digest| !relational_digests.contains(digest))
                .map(|digest| {
                    let support = bucket
                        .support
                        .iter()
                        .filter(|receipt| receipt.matched_program_sha256.contains(digest))
                        .count();
                    (support, digest.clone())
                })
                .min()
                .map(|(_, digest)| digest)
            else {
                break;
            };
            bucket.programs.remove(&evicted);
            bucket.rejected_program_sha256.insert(evicted);
        }
    }
    Ok(())
}

pub(super) fn collect_relational_role_programs(
    program: &ResponseProgram,
    output: &mut Vec<ResponseProgram>,
) {
    match &program.operation {
        crate::ResponseOperation::ProjectSelectedValue {
            selector,
            format,
            completion_state,
            ..
        } => {
            let value_type = online_selector_value_type(selector);
            output.push(ResponseProgram::project_selected_value(
                crate::ResponseValueSelector::RequestReferencedJsonField { value_type },
                *format,
                completion_state.clone(),
            ));
            for reverse_ordinal in 0..4 {
                output.push(ResponseProgram::project_selected_value(
                    crate::ResponseValueSelector::LatestTurnOutputScalarFromEnd {
                        reverse_ordinal,
                        value_type,
                    },
                    *format,
                    completion_state.clone(),
                ));
            }
        }
        crate::ResponseOperation::ProjectStatus {
            mapping,
            completion_state,
            ..
        } => {
            output.push(ResponseProgram::project_status(
                crate::ResponseValueSelector::RequestReferencedJsonField {
                    value_type: crate::AtomValueType::Integer,
                },
                *mapping,
                completion_state.clone(),
            ));
            for reverse_ordinal in 0..4 {
                output.push(ResponseProgram::project_status(
                    crate::ResponseValueSelector::LatestTurnOutputScalarFromEnd {
                        reverse_ordinal,
                        value_type: crate::AtomValueType::Integer,
                    },
                    *mapping,
                    completion_state.clone(),
                ));
            }
        }
        crate::ResponseOperation::UniqueConsensus { variants, .. } => {
            for variant in variants {
                collect_relational_role_programs(&variant.program, output);
            }
        }
        _ => {}
    }
}

pub(super) const fn online_selector_value_type(
    selector: &crate::ResponseValueSelector,
) -> crate::AtomValueType {
    match selector {
        crate::ResponseValueSelector::ContinuationHandle { value_type }
        | crate::ResponseValueSelector::UniqueScalar { value_type }
        | crate::ResponseValueSelector::UniqueTurnScalar { value_type }
        | crate::ResponseValueSelector::ContentLinePrefix { value_type, .. }
        | crate::ResponseValueSelector::JsonField { value_type, .. }
        | crate::ResponseValueSelector::JsonScalarOrdinal { value_type, .. }
        | crate::ResponseValueSelector::UniqueTurnJsonField { value_type, .. }
        | crate::ResponseValueSelector::UniqueActiveTurnJsonField { value_type, .. }
        | crate::ResponseValueSelector::RequestReferencedJsonField { value_type }
        | crate::ResponseValueSelector::RequestReferencedJsonFieldOrdinal { value_type, .. }
        | crate::ResponseValueSelector::TurnOutputLine { value_type, .. }
        | crate::ResponseValueSelector::TurnOutputScalarOrdinal { value_type, .. }
        | crate::ResponseValueSelector::LatestTurnOutputLine { value_type, .. }
        | crate::ResponseValueSelector::LatestTurnOutputScalarOrdinal { value_type, .. } => {
            *value_type
        }
        crate::ResponseValueSelector::LatestTurnOutputScalarFromEnd { value_type, .. } => {
            *value_type
        }
        crate::ResponseValueSelector::CommandOutputBody
        | crate::ResponseValueSelector::RequestLastToken
        | crate::ResponseValueSelector::RequestUniqueLiteral => crate::AtomValueType::String,
    }
}

pub(super) fn invalid_collection_bucket_reason(bucket: &OnlineCollectionBucket) -> Option<String> {
    if bucket.programs.is_empty()
        && (bucket.rejected_program_sha256.is_empty() || bucket.frozen_program_sha256.is_some())
    {
        return Some("empty_program_pool_without_rejected_history".to_owned());
    }
    if bucket.archetype_id.is_empty() {
        return Some("empty_archetype_id".to_owned());
    }
    if let Some(frozen_digest) = &bucket.frozen_program_sha256 {
        if !bucket.programs.contains_key(frozen_digest) {
            return Some(format!("frozen_program_missing:{frozen_digest}"));
        }
        let Some(watermark) = bucket.support_watermark_event_time_unix_nanos else {
            return Some("frozen_support_watermark_missing".to_owned());
        };
        if bucket.support.iter().any(|receipt| {
            receipt
                .event_time_unix_nanos
                .is_none_or(|event_time| event_time > watermark)
        }) {
            return Some("frozen_support_after_watermark".to_owned());
        }
        if collection_support_manifest_digest(bucket).ok().as_ref()
            != bucket.support_manifest_sha256.as_ref()
        {
            return Some("frozen_support_manifest_mismatch".to_owned());
        }
        if let Some(freeze) = &bucket.adaptive_candidate_freeze
            && (freeze.validate().is_err()
                || nando_operator_kernel::response_program_version_root_sha256(
                    bucket.programs.get(frozen_digest)?,
                )
                .ok()
                .as_deref()
                    != Some(freeze.canonical_program_root_sha256())
                || identify_collection_bucket(bucket)
                    .ok()
                    .flatten()
                    .map(|identification| identification.freeze)
                    .as_ref()
                    != Some(freeze))
        {
            return Some("adaptive_candidate_freeze_invalid".to_owned());
        }
    } else if bucket.support_manifest_sha256.is_some() {
        return Some("unfrozen_bucket_has_support_manifest".to_owned());
    } else if bucket.adaptive_candidate_freeze.is_some() {
        return Some("unfrozen_bucket_has_adaptive_freeze".to_owned());
    }
    for (digest, program) in &bucket.programs {
        if canonical_json_sha256(program).ok().as_ref() != Some(digest) {
            return Some(format!("program_digest_mismatch:{digest}"));
        }
        if let Err(reason) = program.validate() {
            return Some(format!("program_contract_invalid:{digest}:{reason}"));
        }
        if !is_privacy_safe_online_response_program(program) {
            return Some(format!("program_privacy_invalid:{digest}"));
        }
    }
    let adapter_digests = bucket_adapter_digests(bucket);
    for (kind, receipts) in [("support", &bucket.support), ("future", &bucket.future)] {
        for receipt in receipts {
            if receipt.matched_program_sha256.is_empty() {
                return Some(format!("{kind}_receipt_programs_empty"));
            }
            if !valid_witness_receipt_metadata(receipt) {
                return Some(format!("{kind}_receipt_witness_metadata_invalid"));
            }
            if let Some(digest) = receipt
                .matched_program_sha256
                .iter()
                .find(|digest| !adapter_digests.contains(*digest))
            {
                return Some(format!("{kind}_receipt_program_unknown:{digest}"));
            }
        }
    }
    None
}

pub(super) fn bucket_adapter_digests(bucket: &OnlineCollectionBucket) -> BTreeSet<String> {
    let mut digests = bucket.programs.keys().cloned().collect::<BTreeSet<_>>();
    digests.extend(bucket.rejected_program_sha256.iter().cloned());
    for program in bucket.programs.values() {
        if let crate::ResponseOperation::UniqueConsensus { variants, .. } = &program.operation {
            for variant in variants {
                if let Ok(digest) = canonical_json_sha256(&variant.program) {
                    digests.insert(digest);
                }
            }
        }
    }
    digests
}
