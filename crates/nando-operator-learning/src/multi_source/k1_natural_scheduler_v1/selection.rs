use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};

use super::model::{
    K1_NATURAL_CANDIDATE_MAX_ROWS_V1, K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V1,
    K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V2, K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V3,
    K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V4, K1_NATURAL_COHORT_CATALOG_SCHEMA_V1,
    K1_NATURAL_COHORT_CATALOG_SCHEMA_V2, K1_NATURAL_EVIDENCE_ROW_SCHEMA_V3,
    K1_NATURAL_EVIDENCE_ROW_SCHEMA_V4, K1CandidateReadinessV1, K1CandidateScoreV1,
    K1ConsequenceTypeV1, K1DeficitSnapshotV1, K1MotifCandidateSupportV1,
    K1MotifDispositionSummaryV1, K1NaturalCandidateQueueRowV1, K1NaturalCandidateQueueV1,
    K1NaturalCohortCandidateV1, K1NaturalCohortCatalogV1, K1NaturalEvidenceClassV1,
    K1NaturalEvidenceRowV1,
};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CohortKey {
    capture_generation_root_sha256: String,
    candidate_structural_root_sha256: String,
    source_neutral_topology_root_sha256: String,
    semantic_novelty_signature_root_sha256: String,
    consequence_type: K1ConsequenceTypeV1,
}

pub fn build_k1_natural_cohort_catalog_v1(
    rows: &[K1NaturalEvidenceRowV1],
    evidence_epoch_root_sha256: String,
    fixture_exclusion_root_sha256: String,
    generator_schema: String,
) -> Result<K1NaturalCohortCatalogV1, &'static str> {
    if !valid_nonzero_sha256(&evidence_epoch_root_sha256)
        || !valid_nonzero_sha256(&fixture_exclusion_root_sha256)
        || generator_schema.is_empty()
    {
        return Err("k1_natural_catalog_binding_invalid");
    }
    let mut cohorts = BTreeMap::<CohortKey, Vec<&K1NaturalEvidenceRowV1>>::new();
    let mut natural_rows = 0u64;
    let mut controlled_rows_excluded = 0u64;
    let mut generated_fixture_rows_excluded = 0u64;
    let mut unknown_rows_excluded = 0u64;
    let mut safety_veto_rows_excluded = 0u64;
    let mut seen_rows = BTreeSet::new();
    for row in rows {
        row.validate()?;
        if !seen_rows.insert(row.row_root_sha256.as_str()) {
            return Err("k1_natural_catalog_evidence_reused");
        }
        if row.schema != K1_NATURAL_EVIDENCE_ROW_SCHEMA_V3 || row.safety_veto {
            safety_veto_rows_excluded = safety_veto_rows_excluded.saturating_add(1);
            continue;
        }
        match row.evidence_class {
            K1NaturalEvidenceClassV1::NaturalLive => {
                natural_rows = natural_rows.saturating_add(1);
            }
            K1NaturalEvidenceClassV1::Controlled => {
                controlled_rows_excluded = controlled_rows_excluded.saturating_add(1);
                continue;
            }
            K1NaturalEvidenceClassV1::GeneratedMs5 | K1NaturalEvidenceClassV1::GeneratedMs6 => {
                generated_fixture_rows_excluded = generated_fixture_rows_excluded.saturating_add(1);
                continue;
            }
            K1NaturalEvidenceClassV1::Unknown => {
                unknown_rows_excluded = unknown_rows_excluded.saturating_add(1);
                continue;
            }
        }
        cohorts
            .entry(CohortKey {
                capture_generation_root_sha256: row.capture_generation_root_sha256.clone(),
                candidate_structural_root_sha256: row.candidate_structural_root_sha256.clone(),
                source_neutral_topology_root_sha256: row
                    .source_neutral_topology_root_sha256
                    .clone(),
                semantic_novelty_signature_root_sha256: row
                    .semantic_novelty_signature_root_sha256
                    .clone(),
                consequence_type: row.consequence_type,
            })
            .or_default()
            .push(row);
    }

    let mut candidates = cohorts
        .into_iter()
        .map(|(key, rows)| build_candidate(key, rows, &generator_schema))
        .collect::<Result<Vec<_>, _>>()?;
    candidates.sort_by(|left, right| left.candidate_root_sha256.cmp(&right.candidate_root_sha256));
    let mut catalog = K1NaturalCohortCatalogV1 {
        schema: K1_NATURAL_COHORT_CATALOG_SCHEMA_V1.to_owned(),
        catalog_root_sha256: String::new(),
        evidence_epoch_root_sha256,
        fixture_exclusion_root_sha256,
        scanned_rows: u64::try_from(rows.len()).map_err(|_| "k1_natural_catalog_count")?,
        natural_rows,
        controlled_rows_excluded,
        generated_fixture_rows_excluded,
        unknown_rows_excluded,
        safety_veto_rows_excluded,
        candidates,
        authority_ready: false,
        motif_retained_occurrences: 0,
        motif_disposition: None,
    };
    catalog.catalog_root_sha256 = catalog.expected_root()?;
    catalog.validate()?;
    Ok(catalog)
}

pub fn build_k1_motif_cohort_catalog_v1(
    rows: &[K1NaturalEvidenceRowV1],
    candidate_supports: &[K1MotifCandidateSupportV1],
    evidence_epoch_root_sha256: String,
    fixture_exclusion_root_sha256: String,
    generator_schema: String,
    disposition: K1MotifDispositionSummaryV1,
) -> Result<K1NaturalCohortCatalogV1, &'static str> {
    if !valid_nonzero_sha256(&evidence_epoch_root_sha256)
        || !valid_nonzero_sha256(&fixture_exclusion_root_sha256)
        || generator_schema.is_empty()
    {
        return Err("k1_motif_catalog_binding_invalid");
    }
    disposition.validate()?;
    let mut cohorts = BTreeMap::<CohortKey, Vec<&K1NaturalEvidenceRowV1>>::new();
    let mut seen_rows = BTreeSet::new();
    for row in rows {
        row.validate()?;
        if !seen_rows.insert(row.row_root_sha256.as_str()) {
            return Err("k1_motif_catalog_evidence_reused");
        }
        if row.schema != K1_NATURAL_EVIDENCE_ROW_SCHEMA_V4
            || row.safety_veto
            || row.evidence_class != K1NaturalEvidenceClassV1::NaturalLive
        {
            return Err("k1_motif_catalog_retained_evidence_invalid");
        }
        cohorts
            .entry(CohortKey {
                capture_generation_root_sha256: row.capture_generation_root_sha256.clone(),
                candidate_structural_root_sha256: row.candidate_structural_root_sha256.clone(),
                source_neutral_topology_root_sha256: row
                    .source_neutral_topology_root_sha256
                    .clone(),
                semantic_novelty_signature_root_sha256: row
                    .semantic_novelty_signature_root_sha256
                    .clone(),
                consequence_type: row.consequence_type,
            })
            .or_default()
            .push(row);
    }
    let mut support_by_key = BTreeMap::new();
    for support in candidate_supports {
        support.validate()?;
        let key = CohortKey {
            capture_generation_root_sha256: support.capture_generation_root_sha256.clone(),
            candidate_structural_root_sha256: support.motif_root_sha256.clone(),
            source_neutral_topology_root_sha256: support.motif_root_sha256.clone(),
            semantic_novelty_signature_root_sha256: support
                .semantic_novelty_signature_root_sha256
                .clone(),
            consequence_type: support.consequence_type,
        };
        if support_by_key.insert(key, support).is_some() {
            return Err("k1_motif_catalog_support_reused");
        }
    }
    let mut candidates = cohorts
        .into_iter()
        .map(|(key, rows)| {
            let support = support_by_key
                .remove(&key)
                .ok_or("k1_motif_catalog_support_missing")?;
            build_motif_candidate(key, rows, support, &generator_schema)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !support_by_key.is_empty() {
        return Err("k1_motif_catalog_support_orphaned");
    }
    candidates.sort_by(|left, right| left.candidate_root_sha256.cmp(&right.candidate_root_sha256));
    let motif_retained_occurrences =
        u64::try_from(rows.len()).map_err(|_| "k1_motif_catalog_count")?;
    let support_overflow_occurrences = candidates
        .iter()
        .map(|candidate| candidate.motif_support_overflow_occurrences)
        .try_fold(0u64, u64::checked_add)
        .ok_or("k1_motif_catalog_count")?;
    let support_overflow_manifest_root_sha256 = canonical_json_sha256(&(
        "nando.k1-motif-candidate-support-manifest.v1",
        candidate_supports
            .iter()
            .map(|support| support.support_root_sha256.as_str())
            .collect::<BTreeSet<_>>(),
    ))?;
    if disposition.retained_motif_occurrences != motif_retained_occurrences
        || disposition.support_overflow_occurrences != support_overflow_occurrences
        || disposition.support_overflow_manifest_root_sha256
            != support_overflow_manifest_root_sha256
    {
        return Err("k1_motif_catalog_disposition_partition_invalid");
    }
    let mut catalog = K1NaturalCohortCatalogV1 {
        schema: K1_NATURAL_COHORT_CATALOG_SCHEMA_V2.to_owned(),
        catalog_root_sha256: String::new(),
        evidence_epoch_root_sha256,
        fixture_exclusion_root_sha256,
        scanned_rows: disposition.scanned_source_rows,
        natural_rows: disposition.motif_source_rows,
        controlled_rows_excluded: disposition.fixture_or_controlled_excluded_rows,
        generated_fixture_rows_excluded: 0,
        unknown_rows_excluded: 0,
        safety_veto_rows_excluded: disposition.safety_veto_rows,
        candidates,
        authority_ready: false,
        motif_retained_occurrences,
        motif_disposition: Some(disposition),
    };
    catalog.catalog_root_sha256 = catalog.expected_root()?;
    catalog.validate()?;
    Ok(catalog)
}

pub fn build_k1_natural_candidate_queue_v1(
    catalog: &K1NaturalCohortCatalogV1,
    deficit: &K1DeficitSnapshotV1,
    contract_watermark: u64,
) -> Result<K1NaturalCandidateQueueV1, &'static str> {
    build_k1_natural_candidate_queue_with_exclusions_v1(
        catalog,
        deficit,
        &BTreeSet::new(),
        contract_watermark,
    )
}

pub fn build_k1_natural_candidate_queue_with_exclusions_v1(
    catalog: &K1NaturalCohortCatalogV1,
    deficit: &K1DeficitSnapshotV1,
    excluded_candidate_roots_sha256: &BTreeSet<String>,
    contract_watermark: u64,
) -> Result<K1NaturalCandidateQueueV1, &'static str> {
    catalog.validate()?;
    deficit.validate()?;
    if excluded_candidate_roots_sha256
        .iter()
        .any(|root| !valid_nonzero_sha256(root))
    {
        return Err("k1_natural_candidate_exclusion_invalid");
    }
    let known_topologies = deficit
        .eligible_role_topology_roots_sha256
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let known_consequences = deficit
        .known_consequence_types
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let catalog_candidates =
        u64::try_from(catalog.candidates.len()).map_err(|_| "k1_natural_candidate_count")?;
    let completed_candidates_excluded = u64::try_from(
        catalog
            .candidates
            .iter()
            .filter(|candidate| {
                excluded_candidate_roots_sha256.contains(&candidate.candidate_root_sha256)
            })
            .count(),
    )
    .map_err(|_| "k1_natural_candidate_count")?;
    let mut rows = catalog
        .candidates
        .iter()
        .filter(|candidate| {
            !excluded_candidate_roots_sha256.contains(&candidate.candidate_root_sha256)
        })
        .map(|candidate| {
            let law_gain = u8::from(deficit.law_deficit > 0);
            let semantic_gain = u8::from(
                deficit.semantic_deficit > 0
                    && !known_consequences.contains(&candidate.consequence_type),
            );
            let topology_gain = u8::from(
                deficit.topology_deficit > 0
                    && !known_topologies
                        .contains(candidate.source_neutral_topology_root_sha256.as_str()),
            );
            let freeze_ready = candidate.readiness.freeze_ready_at(
                candidate.evidence_rows,
                candidate.first_capture_sequence,
                candidate.last_capture_sequence,
                contract_watermark,
            )?;
            let score = K1CandidateScoreV1 {
                total_k1_gain: law_gain
                    .saturating_add(semantic_gain)
                    .saturating_add(topology_gain),
                law_gain,
                semantic_gain,
                topology_gain,
                readiness_rank: u8::from(freeze_ready),
                bounded_discovery_cost_units: candidate.bounded_discovery_cost_units,
                expected_verified_input_tokens: candidate.expected_verified_input_tokens,
                stable_hash_sha256: candidate.candidate_root_sha256.clone(),
            };
            score.validate()?;
            Ok(K1NaturalCandidateQueueRowV1 {
                candidate_root_sha256: candidate.candidate_root_sha256.clone(),
                readiness_receipt_root_sha256: candidate
                    .readiness
                    .readiness_receipt_root_sha256
                    .clone(),
                score,
            })
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    let queue_schema = if catalog.schema == K1_NATURAL_COHORT_CATALOG_SCHEMA_V2 {
        K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V2
    } else {
        K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V1
    };
    rows.sort_by(|left, right| rank_candidates(left, right, queue_schema));
    let scored_candidates = u64::try_from(rows.len()).map_err(|_| "k1_natural_candidate_count")?;
    let first_ready = rows
        .iter()
        .find(|row| row.score.readiness_rank == 1)
        .cloned();
    rows.truncate(K1_NATURAL_CANDIDATE_MAX_ROWS_V1);
    let mut readiness_rescue_included = false;
    if !rows.iter().any(|row| row.score.readiness_rank == 1)
        && let Some(first_ready) = first_ready
        && rows.len() == K1_NATURAL_CANDIDATE_MAX_ROWS_V1
    {
        rows.pop();
        rows.push(first_ready);
        rows.sort_by(|left, right| rank_candidates(left, right, queue_schema));
        readiness_rescue_included = true;
    }
    let retained_candidates =
        u64::try_from(rows.len()).map_err(|_| "k1_natural_candidate_count")?;
    let capacity_excluded_candidates = scored_candidates
        .checked_sub(retained_candidates)
        .ok_or("k1_natural_candidate_count")?;
    let mut queue = K1NaturalCandidateQueueV1 {
        schema: queue_schema.to_owned(),
        queue_root_sha256: String::new(),
        catalog_root_sha256: catalog.catalog_root_sha256.clone(),
        k1_deficit_snapshot_root_sha256: deficit.snapshot_root_sha256.clone(),
        fixture_exclusion_root_sha256: catalog.fixture_exclusion_root_sha256.clone(),
        catalog_candidates,
        completed_candidates_excluded,
        scored_candidates,
        capacity_excluded_candidates,
        readiness_rescue_included,
        rows,
        authority_ready: false,
    };
    queue.queue_root_sha256 = queue.expected_root()?;
    queue.validate()?;
    Ok(queue)
}

fn build_candidate(
    key: CohortKey,
    mut rows: Vec<&K1NaturalEvidenceRowV1>,
    generator_schema: &str,
) -> Result<K1NaturalCohortCandidateV1, &'static str> {
    rows.sort_by(|left, right| {
        left.capture_sequence
            .cmp(&right.capture_sequence)
            .then_with(|| left.row_root_sha256.cmp(&right.row_root_sha256))
    });
    let evidence_roots = rows
        .iter()
        .map(|row| row.row_root_sha256.as_str())
        .collect::<Vec<_>>();
    let evidence_manifest_root_sha256 =
        canonical_json_sha256(&("nando.k1-natural-evidence-manifest.v1", &evidence_roots))?;
    let settled_rows = u64::try_from(rows.iter().filter(|row| row.settled).count())
        .map_err(|_| "k1_natural_candidate_count")?;
    let verified_rows = u64::try_from(rows.iter().filter(|row| row.verified).count())
        .map_err(|_| "k1_natural_candidate_count")?;
    let independent_lineages = u64::try_from(
        rows.iter()
            .filter(|row| row.settled)
            .map(|row| row.lineage_root_sha256.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
    )
    .map_err(|_| "k1_natural_candidate_count")?;
    let expected_verified_input_tokens = rows
        .iter()
        .filter(|row| row.verified)
        .map(|row| row.input_tokens)
        .sum();
    let evidence_rows = u64::try_from(rows.len()).map_err(|_| "k1_natural_candidate_count")?;
    let readiness =
        K1CandidateReadinessV1::seal(settled_rows, verified_rows, independent_lineages)?;
    let first_capture_sequence = rows
        .first()
        .map(|row| row.capture_sequence)
        .ok_or("k1_natural_candidate_empty")?;
    let last_capture_sequence = rows
        .last()
        .map(|row| row.capture_sequence)
        .ok_or("k1_natural_candidate_empty")?;
    let mut candidate = K1NaturalCohortCandidateV1 {
        schema: K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V3.to_owned(),
        candidate_root_sha256: String::new(),
        capture_generation_root_sha256: key.capture_generation_root_sha256,
        candidate_structural_root_sha256: key.candidate_structural_root_sha256,
        source_neutral_topology_root_sha256: key.source_neutral_topology_root_sha256,
        semantic_novelty_signature_root_sha256: key.semantic_novelty_signature_root_sha256,
        consequence_type: key.consequence_type,
        evidence_manifest_root_sha256,
        evidence_rows,
        settled_rows,
        verified_rows,
        independent_lineages,
        expected_verified_input_tokens,
        bounded_discovery_cost_units: evidence_rows.max(1),
        first_capture_sequence,
        last_capture_sequence,
        generator_schema: generator_schema.to_owned(),
        readiness,
        authority_ready: false,
        phase_mutation_allowed: false,
        complete_topology_manifest_root_sha256: String::new(),
        motif_embedding_manifest_root_sha256: String::new(),
        motif_support_overflow_occurrences: 0,
        motif_support_overflow_manifest_root_sha256: String::new(),
    };
    candidate.candidate_root_sha256 = candidate.expected_root()?;
    candidate.validate()?;
    Ok(candidate)
}

fn build_motif_candidate(
    key: CohortKey,
    mut rows: Vec<&K1NaturalEvidenceRowV1>,
    support: &K1MotifCandidateSupportV1,
    generator_schema: &str,
) -> Result<K1NaturalCohortCandidateV1, &'static str> {
    rows.sort_by(|left, right| {
        left.capture_sequence
            .cmp(&right.capture_sequence)
            .then_with(|| left.row_root_sha256.cmp(&right.row_root_sha256))
    });
    let evidence_rows = u64::try_from(rows.len()).map_err(|_| "k1_motif_candidate_count")?;
    let evidence_manifest_root_sha256 = canonical_json_sha256(&(
        "nando.k1-motif-evidence-manifest.v1",
        rows.iter()
            .map(|row| row.row_root_sha256.as_str())
            .collect::<Vec<_>>(),
    ))?;
    let complete_topology_manifest_root_sha256 = canonical_json_sha256(&(
        "nando.k1-motif-complete-topology-manifest.v1",
        rows.iter()
            .map(|row| row.complete_topology_root_sha256.as_str())
            .collect::<Vec<_>>(),
    ))?;
    let motif_embedding_manifest_root_sha256 = canonical_json_sha256(&(
        "nando.k1-motif-candidate-embedding-manifest.v1",
        rows.iter()
            .map(|row| row.motif_embedding_manifest_root_sha256.as_str())
            .collect::<Vec<_>>(),
    ))?;
    if support.capture_generation_root_sha256 != key.capture_generation_root_sha256
        || support.motif_root_sha256 != key.candidate_structural_root_sha256
        || support.semantic_novelty_signature_root_sha256
            != key.semantic_novelty_signature_root_sha256
        || support.consequence_type != key.consequence_type
        || support.retained_rows != evidence_rows
        || support.retained_manifest_root_sha256 != evidence_manifest_root_sha256
    {
        return Err("k1_motif_candidate_support_incoherent");
    }
    let settled_rows = u64::try_from(rows.iter().filter(|row| row.settled).count())
        .map_err(|_| "k1_motif_candidate_count")?;
    let verified_rows = u64::try_from(rows.iter().filter(|row| row.verified).count())
        .map_err(|_| "k1_motif_candidate_count")?;
    let independent_lineages = u64::try_from(
        rows.iter()
            .filter(|row| row.settled)
            .map(|row| row.lineage_root_sha256.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
    )
    .map_err(|_| "k1_motif_candidate_count")?;
    let expected_verified_input_tokens = rows
        .iter()
        .filter(|row| row.verified)
        .map(|row| row.input_tokens)
        .sum();
    let readiness =
        K1CandidateReadinessV1::seal(settled_rows, verified_rows, independent_lineages)?;
    let first_capture_sequence = rows
        .first()
        .map(|row| row.capture_sequence)
        .ok_or("k1_motif_candidate_empty")?;
    let last_capture_sequence = rows
        .last()
        .map(|row| row.capture_sequence)
        .ok_or("k1_motif_candidate_empty")?;
    let mut candidate = K1NaturalCohortCandidateV1 {
        schema: K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V4.to_owned(),
        candidate_root_sha256: String::new(),
        capture_generation_root_sha256: key.capture_generation_root_sha256,
        candidate_structural_root_sha256: key.candidate_structural_root_sha256,
        source_neutral_topology_root_sha256: key.source_neutral_topology_root_sha256,
        semantic_novelty_signature_root_sha256: key.semantic_novelty_signature_root_sha256,
        consequence_type: key.consequence_type,
        evidence_manifest_root_sha256,
        evidence_rows,
        settled_rows,
        verified_rows,
        independent_lineages,
        expected_verified_input_tokens,
        bounded_discovery_cost_units: evidence_rows
            .saturating_add(support.overflow_occurrences)
            .max(1),
        first_capture_sequence,
        last_capture_sequence,
        generator_schema: generator_schema.to_owned(),
        readiness,
        authority_ready: false,
        phase_mutation_allowed: false,
        complete_topology_manifest_root_sha256,
        motif_embedding_manifest_root_sha256,
        motif_support_overflow_occurrences: support.overflow_occurrences,
        motif_support_overflow_manifest_root_sha256: support.overflow_manifest_root_sha256.clone(),
    };
    candidate.candidate_root_sha256 = candidate.expected_root()?;
    candidate.validate()?;
    Ok(candidate)
}

fn rank_candidates(
    left: &K1NaturalCandidateQueueRowV1,
    right: &K1NaturalCandidateQueueRowV1,
    queue_schema: &str,
) -> Ordering {
    let order = right
        .score
        .total_k1_gain
        .cmp(&left.score.total_k1_gain)
        .then_with(|| right.score.readiness_rank.cmp(&left.score.readiness_rank));
    let order = if queue_schema == K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V2 {
        order
            .then_with(|| {
                left.score
                    .bounded_discovery_cost_units
                    .cmp(&right.score.bounded_discovery_cost_units)
            })
            .then_with(|| {
                right
                    .score
                    .expected_verified_input_tokens
                    .cmp(&left.score.expected_verified_input_tokens)
            })
    } else {
        order
            .then_with(|| {
                right
                    .score
                    .expected_verified_input_tokens
                    .cmp(&left.score.expected_verified_input_tokens)
            })
            .then_with(|| {
                left.score
                    .bounded_discovery_cost_units
                    .cmp(&right.score.bounded_discovery_cost_units)
            })
    };
    order.then_with(|| {
        left.score
            .stable_hash_sha256
            .cmp(&right.score.stable_hash_sha256)
    })
}
