use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::*;
use crate::{sha256_bytes, teacher_semantic_law_signature};

// No label-free fixture currently proves a V1-only protocol distinction. Until
// one exists, every V3 merge across legacy classes remains WATCH.
const PROTOCOL_ONLY_MERGE_FIXTURE_PROVEN_V3: bool = false;

#[derive(Clone, Debug)]
pub struct EffectLawDualClassificationRowV3 {
    transition: TeacherTransition,
    observation: SealedEffectObservationV3,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectLawDualClassificationRowStatusV3 {
    TrustedClassified,
    TrustedLegacyUnknown,
    TrustedV3Censored,
    TrustedDualCensored,
    TrustFailure,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectLawDualClassificationReasonV3 {
    ProtocolFacet,
    EffectInvariant,
    TypedConstants,
    CompletionStatusRenderer,
    TemporalCardinality,
    PreservedFrame,
    TrustFailure,
    IndependenceFailure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectLawDualClassificationVerdictV3 {
    Pass,
    Watch,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectLawDualClassificationRowReportV3 {
    pub row_sha256: String,
    pub transition_sha256: String,
    pub observation_sha256: String,
    pub status: EffectLawDualClassificationRowStatusV3,
    pub legacy_v1_signature_sha256: Option<String>,
    pub effect_law_id_v3: Option<String>,
    pub reason: Option<EffectLawDualClassificationReasonV3>,
    pub protocol_facet_root_sha256: Option<String>,
    pub physical_program_id_sha256: Option<String>,
    pub independent_surface_root_sha256: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectLawDualClassificationMapV3 {
    pub source_sha256: String,
    pub target_sha256: Vec<String>,
    pub rows: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectLawDualIndependenceReportV3 {
    pub effect_law_id_v3: String,
    pub observations: usize,
    pub episode_lineages: usize,
    pub independent_surfaces: usize,
    pub physical_programs: usize,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectLawDualClassificationDiscrepancyDirectionV3 {
    V1PairToOneV3Merge,
    OneV1ToV3PairSplit,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectLawDualClassificationDiscrepancyWitnessV3 {
    pub direction: EffectLawDualClassificationDiscrepancyDirectionV3,
    pub left_class_sha256: String,
    pub right_class_sha256: String,
    pub shared_class_sha256: String,
    pub left_effect_facet_root_sha256: String,
    pub right_effect_facet_root_sha256: String,
    pub left_protocol_facet_set_root_sha256: String,
    pub right_protocol_facet_set_root_sha256: String,
    pub effect_facets_identical: bool,
    pub protocol_facets_distinct: bool,
    pub supporting_fixture_proven: bool,
    pub reasons: Vec<EffectLawDualClassificationReasonV3>,
    pub explained: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectLawDualClassificationDiscrepancyV3 {
    pub legacy_v1_signatures_sha256: Vec<String>,
    pub effect_law_ids_v3: Vec<String>,
    pub pairwise_witnesses: Vec<EffectLawDualClassificationDiscrepancyWitnessV3>,
    pub rows: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectLawDualClassificationReportV3 {
    pub schema: String,
    pub verdict: EffectLawDualClassificationVerdictV3,
    pub denominator: usize,
    pub accounted_rows: usize,
    pub trusted_rows: usize,
    pub legacy_v1_attempted_rows: usize,
    pub v3_attempted_rows: usize,
    pub legacy_v1_classified_rows: usize,
    pub v3_classified_rows: usize,
    pub legacy_v1_route_root_sha256: String,
    pub row_accounting: Vec<EffectLawDualClassificationRowReportV3>,
    pub legacy_v1_cohort_to_v3_laws: Vec<EffectLawDualClassificationMapV3>,
    pub v3_law_to_legacy_v1_cohorts: Vec<EffectLawDualClassificationMapV3>,
    pub v3_law_to_physical_programs: Vec<EffectLawDualClassificationMapV3>,
    pub v3_law_to_independent_surfaces: Vec<EffectLawDualClassificationMapV3>,
    pub expected_surface_only_merges: Vec<EffectLawDualClassificationDiscrepancyV3>,
    pub effect_significant_splits: Vec<EffectLawDualClassificationDiscrepancyV3>,
    pub unknown_censored_rows: Vec<EffectLawDualClassificationRowReportV3>,
    pub trust_failures: Vec<EffectLawDualClassificationRowReportV3>,
    pub three_dimensional_independence: Vec<EffectLawDualIndependenceReportV3>,
    pub pairwise_discrepancies_expected: usize,
    pub pairwise_discrepancies_accounted: usize,
    pub unexplained_merges: Vec<EffectLawDualClassificationDiscrepancyV3>,
    pub unexplained_splits: Vec<EffectLawDualClassificationDiscrepancyV3>,
    pub execution_authority: bool,
}

pub struct EffectLawDualClassifierV3<'a> {
    trusted_evidence: &'a TrustedEffectEvidenceSetV3,
    dictionary: &'a EffectLawDictionaryV3,
    hypothesis: &'a EffectQuotientHypothesisV3,
}

struct PendingRow {
    input_index: usize,
    row_sha256: String,
    transition_sha256: String,
    observation_sha256: String,
    legacy_v1_signature_sha256: Option<String>,
    effect_law_id_v3: Option<String>,
    reason: Option<EffectLawDualClassificationReasonV3>,
    facets: Option<LawFacets>,
}

#[derive(Clone)]
struct LawFacets {
    grouping_key_sha256: String,
    protocol_facet_root_sha256: String,
    physical_program_id_sha256: String,
    independent_surface_root_sha256: String,
    episode_lineage_sha256: String,
    effect_invariant_root_sha256: String,
    typed_constants_root_sha256: String,
    completion_status_renderer_root_sha256: String,
    temporal_cardinality_root_sha256: String,
    preserved_frame_root_sha256: String,
    effect_facet_root_sha256: String,
}

#[derive(Clone)]
struct ClassifiedLawFacets {
    legacy_v1_signature_sha256: String,
    facets: LawFacets,
}

impl EffectLawDualClassificationRowV3 {
    #[must_use]
    pub fn new(transition: TeacherTransition, observation: SealedEffectObservationV3) -> Self {
        Self {
            transition,
            observation,
        }
    }
}

impl<'a> EffectLawDualClassifierV3<'a> {
    #[must_use]
    pub fn new(
        trusted_evidence: &'a TrustedEffectEvidenceSetV3,
        dictionary: &'a EffectLawDictionaryV3,
        hypothesis: &'a EffectQuotientHypothesisV3,
    ) -> Self {
        Self {
            trusted_evidence,
            dictionary,
            hypothesis,
        }
    }

    pub fn classify(
        &self,
        rows: &[EffectLawDualClassificationRowV3],
    ) -> Result<EffectLawDualClassificationReportV3, EffectLawV3Error> {
        if rows.is_empty() || rows.len() > MAX_OBSERVATIONS_V3 {
            return Err(EffectLawV3Error::OverBudget);
        }
        let denominator = rows.len();
        let mut seen_transitions = BTreeSet::new();
        let mut seen_observations = BTreeSet::new();
        let mut row_accounting = Vec::with_capacity(rows.len());
        let mut pending = Vec::new();

        for (input_index, row) in rows.iter().enumerate() {
            let transition_sha256 = evidence::sha256_serialized(&row.transition)?;
            let observation_sha256 = row.observation.observation_sha256.clone();
            if !seen_transitions.insert(transition_sha256.clone())
                || !seen_observations.insert(observation_sha256.clone())
            {
                return Err(EffectLawV3Error::InvalidCandidate);
            }
            let row_sha256 = evidence::sha256_serialized(&(
                "nando.effect-law-dual-classification-row.v1-v3",
                transition_sha256.as_str(),
                observation_sha256.as_str(),
            ))?;
            if validate_trusted_row(&row.transition, &row.observation, self.trusted_evidence)
                .is_err()
            {
                row_accounting.push(EffectLawDualClassificationRowReportV3 {
                    row_sha256,
                    transition_sha256,
                    observation_sha256,
                    status: EffectLawDualClassificationRowStatusV3::TrustFailure,
                    legacy_v1_signature_sha256: None,
                    effect_law_id_v3: None,
                    reason: Some(EffectLawDualClassificationReasonV3::TrustFailure),
                    protocol_facet_root_sha256: None,
                    physical_program_id_sha256: None,
                    independent_surface_root_sha256: None,
                });
                continue;
            }

            let legacy_v1_signature_sha256 =
                teacher_semantic_law_signature(&row.transition.as_training_relation_frame());
            let (facets, reason) = match canonical::observation_classification_facets(
                &row.observation,
                self.dictionary,
                self.hypothesis,
            ) {
                Ok(canonical) => {
                    let effect_invariant_root_sha256 = evidence::sha256_serialized(&(
                        canonical.law.topology_nodes(),
                        canonical.law.topology_edges(),
                        canonical.law.effect_invariant_root_sha256(),
                    ))?;
                    let effect_facet_root_sha256 = evidence::sha256_serialized(&(
                        effect_invariant_root_sha256.as_str(),
                        canonical.typed_constants_root_sha256.as_str(),
                        canonical.completion_status_renderer_root_sha256.as_str(),
                        canonical.temporal_cardinality_root_sha256.as_str(),
                        canonical.law.preserved_frame_root_sha256(),
                    ))?;
                    (
                        Some(LawFacets {
                            grouping_key_sha256: canonical.grouping_key_sha256,
                            protocol_facet_root_sha256: row
                                .observation
                                .protocol_facet
                                .root_sha256
                                .clone(),
                            physical_program_id_sha256: row.observation.physical_program_id.clone(),
                            independent_surface_root_sha256: row
                                .observation
                                .surface_root_sha256
                                .clone(),
                            episode_lineage_sha256: row.observation.episode_lineage_sha256.clone(),
                            effect_invariant_root_sha256,
                            typed_constants_root_sha256: canonical.typed_constants_root_sha256,
                            completion_status_renderer_root_sha256: canonical
                                .completion_status_renderer_root_sha256,
                            temporal_cardinality_root_sha256: canonical
                                .temporal_cardinality_root_sha256,
                            preserved_frame_root_sha256: canonical
                                .law
                                .preserved_frame_root_sha256()
                                .to_owned(),
                            effect_facet_root_sha256,
                        }),
                        None,
                    )
                }
                Err(error) => (None, Some(censor_reason(error))),
            };
            pending.push(PendingRow {
                input_index,
                row_sha256,
                transition_sha256,
                observation_sha256,
                legacy_v1_signature_sha256,
                effect_law_id_v3: None,
                reason,
                facets,
            });
        }

        let mut groups = BTreeMap::<String, Vec<usize>>::new();
        for (pending_index, item) in pending.iter().enumerate() {
            if let Some(facets) = &item.facets {
                groups
                    .entry(facets.grouping_key_sha256.clone())
                    .or_default()
                    .push(pending_index);
            }
        }
        for pending_indices in groups.into_values() {
            let observations = pending_indices
                .iter()
                .map(|index| rows[pending[*index].input_index].observation.clone())
                .collect::<Vec<_>>();
            match canonical::search_quotient(&observations, self.dictionary, self.hypothesis) {
                Ok(report) => {
                    if let Some(candidate) = report.candidate {
                        let law_id = candidate.law.effect_law_id()?.as_str().to_owned();
                        for index in pending_indices {
                            pending[index].effect_law_id_v3 = Some(law_id.clone());
                        }
                    } else {
                        for index in pending_indices {
                            pending[index].reason =
                                Some(EffectLawDualClassificationReasonV3::EffectInvariant);
                        }
                    }
                }
                Err(error) => {
                    let reason = censor_reason(error);
                    for index in pending_indices {
                        pending[index].reason = Some(reason);
                    }
                }
            }
        }

        for item in pending {
            let status = match (
                item.legacy_v1_signature_sha256.is_some(),
                item.effect_law_id_v3.is_some(),
            ) {
                (true, true) => EffectLawDualClassificationRowStatusV3::TrustedClassified,
                (false, true) => EffectLawDualClassificationRowStatusV3::TrustedLegacyUnknown,
                (true, false) => EffectLawDualClassificationRowStatusV3::TrustedV3Censored,
                (false, false) => EffectLawDualClassificationRowStatusV3::TrustedDualCensored,
            };
            row_accounting.push(EffectLawDualClassificationRowReportV3 {
                row_sha256: item.row_sha256,
                transition_sha256: item.transition_sha256,
                observation_sha256: item.observation_sha256,
                status,
                legacy_v1_signature_sha256: item.legacy_v1_signature_sha256,
                effect_law_id_v3: item.effect_law_id_v3,
                reason: item.reason,
                protocol_facet_root_sha256: item
                    .facets
                    .as_ref()
                    .map(|facets| facets.protocol_facet_root_sha256.clone()),
                physical_program_id_sha256: item
                    .facets
                    .as_ref()
                    .map(|facets| facets.physical_program_id_sha256.clone()),
                independent_surface_root_sha256: item
                    .facets
                    .as_ref()
                    .map(|facets| facets.independent_surface_root_sha256.clone()),
            });
        }
        row_accounting.sort_by(|left, right| left.row_sha256.cmp(&right.row_sha256));

        build_report(denominator, row_accounting, rows, self)
    }
}

impl EffectLawDualClassificationReportV3 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EffectLawV3Error> {
        evidence::canonical_bytes(self)
    }

    pub fn report_sha256(&self) -> Result<String, EffectLawV3Error> {
        Ok(sha256_bytes(&self.canonical_bytes()?))
    }
}

fn build_report(
    denominator: usize,
    row_accounting: Vec<EffectLawDualClassificationRowReportV3>,
    rows: &[EffectLawDualClassificationRowV3],
    classifier: &EffectLawDualClassifierV3<'_>,
) -> Result<EffectLawDualClassificationReportV3, EffectLawV3Error> {
    let trusted_rows = row_accounting
        .iter()
        .filter(|row| row.status != EffectLawDualClassificationRowStatusV3::TrustFailure)
        .count();
    let legacy_v1_classified_rows = row_accounting
        .iter()
        .filter(|row| row.legacy_v1_signature_sha256.is_some())
        .count();
    let v3_classified_rows = row_accounting
        .iter()
        .filter(|row| row.effect_law_id_v3.is_some())
        .count();
    let legacy_route = row_accounting
        .iter()
        .filter(|row| row.status != EffectLawDualClassificationRowStatusV3::TrustFailure)
        .map(|row| {
            (
                row.row_sha256.as_str(),
                row.legacy_v1_signature_sha256.as_deref(),
            )
        })
        .collect::<Vec<_>>();
    let legacy_v1_route_root_sha256 = evidence::sha256_serialized(&legacy_route)?;

    let legacy_to_v3 = grouped_map(
        &row_accounting,
        |row| row.legacy_v1_signature_sha256.as_deref(),
        |row| row.effect_law_id_v3.as_deref(),
    );
    let v3_to_legacy = grouped_map(
        &row_accounting,
        |row| row.effect_law_id_v3.as_deref(),
        |row| row.legacy_v1_signature_sha256.as_deref(),
    );
    let v3_to_programs = grouped_map(
        &row_accounting,
        |row| row.effect_law_id_v3.as_deref(),
        |row| row.physical_program_id_sha256.as_deref(),
    );
    let v3_to_surfaces = grouped_map(
        &row_accounting,
        |row| row.effect_law_id_v3.as_deref(),
        |row| row.independent_surface_root_sha256.as_deref(),
    );

    let facets_by_law = classified_law_facets_by_id(rows, classifier, &row_accounting)?;
    let mut expected_surface_only_merges = Vec::new();
    let mut unexplained_merges = Vec::new();
    for map in &v3_to_legacy {
        if map.target_sha256.len() < 2 {
            continue;
        }
        let witnesses =
            merge_pairwise_witnesses(&map.source_sha256, &map.target_sha256, &facets_by_law)?;
        let fully_explained = witnesses.len() == pair_count(map.target_sha256.len())
            && witnesses.iter().all(|witness| witness.explained);
        let discrepancy = EffectLawDualClassificationDiscrepancyV3 {
            legacy_v1_signatures_sha256: map.target_sha256.clone(),
            effect_law_ids_v3: vec![map.source_sha256.clone()],
            pairwise_witnesses: witnesses,
            rows: map.rows,
        };
        if fully_explained {
            expected_surface_only_merges.push(discrepancy);
        } else {
            unexplained_merges.push(discrepancy);
        }
    }

    let mut effect_significant_splits = Vec::new();
    let mut unexplained_splits = Vec::new();
    for map in &legacy_to_v3 {
        if map.target_sha256.len() < 2 {
            continue;
        }
        let witnesses =
            split_pairwise_witnesses(&map.source_sha256, &map.target_sha256, &facets_by_law)?;
        let fully_explained = witnesses.len() == pair_count(map.target_sha256.len())
            && witnesses.iter().all(|witness| witness.explained);
        let discrepancy = EffectLawDualClassificationDiscrepancyV3 {
            legacy_v1_signatures_sha256: vec![map.source_sha256.clone()],
            effect_law_ids_v3: map.target_sha256.clone(),
            pairwise_witnesses: witnesses,
            rows: map.rows,
        };
        if fully_explained {
            effect_significant_splits.push(discrepancy);
        } else {
            unexplained_splits.push(discrepancy);
        }
    }

    let unknown_censored_rows = row_accounting
        .iter()
        .filter(|row| {
            matches!(
                row.status,
                EffectLawDualClassificationRowStatusV3::TrustedLegacyUnknown
                    | EffectLawDualClassificationRowStatusV3::TrustedV3Censored
                    | EffectLawDualClassificationRowStatusV3::TrustedDualCensored
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    let trust_failures = row_accounting
        .iter()
        .filter(|row| row.status == EffectLawDualClassificationRowStatusV3::TrustFailure)
        .cloned()
        .collect::<Vec<_>>();
    let mut three_dimensional_independence = facets_by_law
        .iter()
        .map(|(law_id, facets)| EffectLawDualIndependenceReportV3 {
            effect_law_id_v3: law_id.clone(),
            observations: facets.len(),
            episode_lineages: facets
                .iter()
                .map(|item| item.facets.episode_lineage_sha256.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            independent_surfaces: facets
                .iter()
                .map(|item| item.facets.independent_surface_root_sha256.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            physical_programs: facets
                .iter()
                .map(|item| item.facets.physical_program_id_sha256.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
        })
        .collect::<Vec<_>>();
    three_dimensional_independence
        .sort_by(|left, right| left.effect_law_id_v3.cmp(&right.effect_law_id_v3));
    let pairwise_discrepancies_expected = legacy_to_v3
        .iter()
        .chain(&v3_to_legacy)
        .map(|map| pair_count(map.target_sha256.len()))
        .sum();
    let pairwise_discrepancies_accounted = expected_surface_only_merges
        .iter()
        .chain(&effect_significant_splits)
        .chain(&unexplained_merges)
        .chain(&unexplained_splits)
        .map(|discrepancy| discrepancy.pairwise_witnesses.len())
        .sum();
    let verdict = if unknown_censored_rows.is_empty()
        && trust_failures.is_empty()
        && unexplained_merges.is_empty()
        && unexplained_splits.is_empty()
        && pairwise_discrepancies_accounted == pairwise_discrepancies_expected
        && legacy_v1_classified_rows == trusted_rows
        && v3_classified_rows == trusted_rows
    {
        EffectLawDualClassificationVerdictV3::Pass
    } else {
        EffectLawDualClassificationVerdictV3::Watch
    };

    Ok(EffectLawDualClassificationReportV3 {
        schema: EFFECT_LAW_DUAL_CLASSIFICATION_REPORT_SCHEMA_V3.to_owned(),
        verdict,
        denominator,
        accounted_rows: row_accounting.len(),
        trusted_rows,
        legacy_v1_attempted_rows: trusted_rows,
        v3_attempted_rows: trusted_rows,
        legacy_v1_classified_rows,
        v3_classified_rows,
        legacy_v1_route_root_sha256,
        row_accounting,
        legacy_v1_cohort_to_v3_laws: legacy_to_v3,
        v3_law_to_legacy_v1_cohorts: v3_to_legacy,
        v3_law_to_physical_programs: v3_to_programs,
        v3_law_to_independent_surfaces: v3_to_surfaces,
        expected_surface_only_merges,
        effect_significant_splits,
        unknown_censored_rows,
        trust_failures,
        three_dimensional_independence,
        pairwise_discrepancies_expected,
        pairwise_discrepancies_accounted,
        unexplained_merges,
        unexplained_splits,
        execution_authority: false,
    })
}

fn grouped_map<'a>(
    rows: &'a [EffectLawDualClassificationRowReportV3],
    source: impl Fn(&'a EffectLawDualClassificationRowReportV3) -> Option<&'a str>,
    target: impl Fn(&'a EffectLawDualClassificationRowReportV3) -> Option<&'a str>,
) -> Vec<EffectLawDualClassificationMapV3> {
    let mut grouped = BTreeMap::<String, (BTreeSet<String>, usize)>::new();
    for row in rows {
        let Some(source) = source(row) else {
            continue;
        };
        let entry = grouped.entry(source.to_owned()).or_default();
        entry.1 += 1;
        if let Some(target) = target(row) {
            entry.0.insert(target.to_owned());
        }
    }
    grouped
        .into_iter()
        .map(
            |(source_sha256, (target_sha256, rows))| EffectLawDualClassificationMapV3 {
                source_sha256,
                target_sha256: target_sha256.into_iter().collect(),
                rows,
            },
        )
        .collect()
}

fn classified_law_facets_by_id(
    rows: &[EffectLawDualClassificationRowV3],
    classifier: &EffectLawDualClassifierV3<'_>,
    accounting: &[EffectLawDualClassificationRowReportV3],
) -> Result<BTreeMap<String, Vec<ClassifiedLawFacets>>, EffectLawV3Error> {
    let classification_by_observation = accounting
        .iter()
        .filter_map(|row| {
            Some((
                row.observation_sha256.clone(),
                (
                    row.effect_law_id_v3.clone()?,
                    row.legacy_v1_signature_sha256.clone()?,
                ),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let mut output = BTreeMap::<String, Vec<ClassifiedLawFacets>>::new();
    for row in rows {
        let Some((law_id, legacy_v1_signature_sha256)) =
            classification_by_observation.get(row.observation.observation_sha256.as_str())
        else {
            continue;
        };
        let canonical = canonical::observation_classification_facets(
            &row.observation,
            classifier.dictionary,
            classifier.hypothesis,
        )?;
        let effect_invariant_root_sha256 = evidence::sha256_serialized(&(
            canonical.law.topology_nodes(),
            canonical.law.topology_edges(),
            canonical.law.effect_invariant_root_sha256(),
        ))?;
        let effect_facet_root_sha256 = evidence::sha256_serialized(&(
            effect_invariant_root_sha256.as_str(),
            canonical.typed_constants_root_sha256.as_str(),
            canonical.completion_status_renderer_root_sha256.as_str(),
            canonical.temporal_cardinality_root_sha256.as_str(),
            canonical.law.preserved_frame_root_sha256(),
        ))?;
        output
            .entry(law_id.clone())
            .or_default()
            .push(ClassifiedLawFacets {
                legacy_v1_signature_sha256: legacy_v1_signature_sha256.clone(),
                facets: LawFacets {
                    grouping_key_sha256: canonical.grouping_key_sha256,
                    protocol_facet_root_sha256: row.observation.protocol_facet.root_sha256.clone(),
                    physical_program_id_sha256: row.observation.physical_program_id.clone(),
                    independent_surface_root_sha256: row.observation.surface_root_sha256.clone(),
                    episode_lineage_sha256: row.observation.episode_lineage_sha256.clone(),
                    effect_invariant_root_sha256,
                    typed_constants_root_sha256: canonical.typed_constants_root_sha256,
                    completion_status_renderer_root_sha256: canonical
                        .completion_status_renderer_root_sha256,
                    temporal_cardinality_root_sha256: canonical.temporal_cardinality_root_sha256,
                    preserved_frame_root_sha256: canonical
                        .law
                        .preserved_frame_root_sha256()
                        .to_owned(),
                    effect_facet_root_sha256,
                },
            });
    }
    for items in output.values_mut() {
        items.sort_by(|left, right| {
            left.legacy_v1_signature_sha256
                .cmp(&right.legacy_v1_signature_sha256)
                .then_with(|| {
                    left.facets
                        .grouping_key_sha256
                        .cmp(&right.facets.grouping_key_sha256)
                })
                .then_with(|| {
                    left.facets
                        .episode_lineage_sha256
                        .cmp(&right.facets.episode_lineage_sha256)
                })
        });
    }
    Ok(output)
}

struct FacetClassSummary<'a> {
    representative: Option<&'a LawFacets>,
    effect_facet_root_sha256: String,
    protocol_facet_set_root_sha256: String,
    protocol_facet_roots: BTreeSet<&'a str>,
    effect_facets_uniform: bool,
}

fn summarize_facet_class<'a>(
    items: impl Iterator<Item = &'a ClassifiedLawFacets>,
) -> Result<FacetClassSummary<'a>, EffectLawV3Error> {
    let items = items.collect::<Vec<_>>();
    let effect_roots = items
        .iter()
        .map(|item| item.facets.effect_facet_root_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let protocol_facet_roots = items
        .iter()
        .map(|item| item.facets.protocol_facet_root_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let effect_facets_uniform = effect_roots.len() == 1;
    let effect_facet_root_sha256 = if effect_facets_uniform {
        effect_roots.first().copied().unwrap_or_default().to_owned()
    } else {
        evidence::sha256_serialized(&effect_roots)?
    };
    let protocol_facet_set_root_sha256 = evidence::sha256_serialized(&protocol_facet_roots)?;
    Ok(FacetClassSummary {
        representative: items.first().map(|item| &item.facets),
        effect_facet_root_sha256,
        protocol_facet_set_root_sha256,
        protocol_facet_roots,
        effect_facets_uniform,
    })
}

fn merge_pairwise_witnesses(
    effect_law_id_v3: &str,
    legacy_signatures: &[String],
    facets_by_law: &BTreeMap<String, Vec<ClassifiedLawFacets>>,
) -> Result<Vec<EffectLawDualClassificationDiscrepancyWitnessV3>, EffectLawV3Error> {
    let items = facets_by_law
        .get(effect_law_id_v3)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut witnesses = Vec::with_capacity(pair_count(legacy_signatures.len()));
    for left_index in 0..legacy_signatures.len() {
        for right_index in (left_index + 1)..legacy_signatures.len() {
            let left_signature = &legacy_signatures[left_index];
            let right_signature = &legacy_signatures[right_index];
            let left = summarize_facet_class(
                items
                    .iter()
                    .filter(|item| item.legacy_v1_signature_sha256 == *left_signature),
            )?;
            let right = summarize_facet_class(
                items
                    .iter()
                    .filter(|item| item.legacy_v1_signature_sha256 == *right_signature),
            )?;
            let effect_facets_identical = left.effect_facets_uniform
                && right.effect_facets_uniform
                && left.effect_facet_root_sha256 == right.effect_facet_root_sha256;
            let protocol_facets_distinct = !left.protocol_facet_roots.is_empty()
                && !right.protocol_facet_roots.is_empty()
                && left.protocol_facet_roots != right.protocol_facet_roots;
            let protocol_only_candidate = effect_facets_identical && protocol_facets_distinct;
            let explained = protocol_only_candidate && PROTOCOL_ONLY_MERGE_FIXTURE_PROVEN_V3;
            witnesses.push(EffectLawDualClassificationDiscrepancyWitnessV3 {
                direction: EffectLawDualClassificationDiscrepancyDirectionV3::V1PairToOneV3Merge,
                left_class_sha256: left_signature.clone(),
                right_class_sha256: right_signature.clone(),
                shared_class_sha256: effect_law_id_v3.to_owned(),
                left_effect_facet_root_sha256: left.effect_facet_root_sha256,
                right_effect_facet_root_sha256: right.effect_facet_root_sha256,
                left_protocol_facet_set_root_sha256: left.protocol_facet_set_root_sha256,
                right_protocol_facet_set_root_sha256: right.protocol_facet_set_root_sha256,
                effect_facets_identical,
                protocol_facets_distinct,
                supporting_fixture_proven: PROTOCOL_ONLY_MERGE_FIXTURE_PROVEN_V3,
                reasons: protocol_only_candidate
                    .then_some(EffectLawDualClassificationReasonV3::ProtocolFacet)
                    .into_iter()
                    .collect(),
                explained,
            });
        }
    }
    Ok(witnesses)
}

fn split_pairwise_witnesses(
    legacy_signature: &str,
    effect_law_ids: &[String],
    facets_by_law: &BTreeMap<String, Vec<ClassifiedLawFacets>>,
) -> Result<Vec<EffectLawDualClassificationDiscrepancyWitnessV3>, EffectLawV3Error> {
    let mut witnesses = Vec::with_capacity(pair_count(effect_law_ids.len()));
    for left_index in 0..effect_law_ids.len() {
        for right_index in (left_index + 1)..effect_law_ids.len() {
            let left_law = &effect_law_ids[left_index];
            let right_law = &effect_law_ids[right_index];
            let left = summarize_facet_class(
                facets_by_law
                    .get(left_law)
                    .into_iter()
                    .flat_map(|items| items.iter()),
            )?;
            let right = summarize_facet_class(
                facets_by_law
                    .get(right_law)
                    .into_iter()
                    .flat_map(|items| items.iter()),
            )?;
            let reasons = if left.effect_facets_uniform && right.effect_facets_uniform {
                split_pair_reasons(left.representative, right.representative)
            } else {
                Vec::new()
            };
            let effect_facets_identical = left.effect_facets_uniform
                && right.effect_facets_uniform
                && left.effect_facet_root_sha256 == right.effect_facet_root_sha256;
            let protocol_facets_distinct = left.protocol_facet_roots != right.protocol_facet_roots;
            let explained = !reasons.is_empty() && !effect_facets_identical;
            witnesses.push(EffectLawDualClassificationDiscrepancyWitnessV3 {
                direction: EffectLawDualClassificationDiscrepancyDirectionV3::OneV1ToV3PairSplit,
                left_class_sha256: left_law.clone(),
                right_class_sha256: right_law.clone(),
                shared_class_sha256: legacy_signature.to_owned(),
                left_effect_facet_root_sha256: left.effect_facet_root_sha256,
                right_effect_facet_root_sha256: right.effect_facet_root_sha256,
                left_protocol_facet_set_root_sha256: left.protocol_facet_set_root_sha256,
                right_protocol_facet_set_root_sha256: right.protocol_facet_set_root_sha256,
                effect_facets_identical,
                protocol_facets_distinct,
                supporting_fixture_proven: true,
                reasons,
                explained,
            });
        }
    }
    Ok(witnesses)
}

fn split_pair_reasons(
    left: Option<&LawFacets>,
    right: Option<&LawFacets>,
) -> Vec<EffectLawDualClassificationReasonV3> {
    let (Some(left), Some(right)) = (left, right) else {
        return Vec::new();
    };
    let mut reasons = BTreeSet::new();
    if left.typed_constants_root_sha256 != right.typed_constants_root_sha256 {
        reasons.insert(EffectLawDualClassificationReasonV3::TypedConstants);
    }
    if left.completion_status_renderer_root_sha256 != right.completion_status_renderer_root_sha256 {
        reasons.insert(EffectLawDualClassificationReasonV3::CompletionStatusRenderer);
    }
    if left.temporal_cardinality_root_sha256 != right.temporal_cardinality_root_sha256 {
        reasons.insert(EffectLawDualClassificationReasonV3::TemporalCardinality);
    }
    if left.preserved_frame_root_sha256 != right.preserved_frame_root_sha256 {
        reasons.insert(EffectLawDualClassificationReasonV3::PreservedFrame);
    }
    if reasons.is_empty() && left.effect_invariant_root_sha256 != right.effect_invariant_root_sha256
    {
        reasons.insert(EffectLawDualClassificationReasonV3::EffectInvariant);
    }
    reasons.into_iter().collect()
}

const fn pair_count(classes: usize) -> usize {
    classes.saturating_mul(classes.saturating_sub(1)) / 2
}

fn validate_trusted_row(
    transition: &TeacherTransition,
    observation: &SealedEffectObservationV3,
    trusted: &TrustedEffectEvidenceSetV3,
) -> Result<(), EffectLawV3Error> {
    evidence::validate_sealed_observation(observation)?;
    let transition_sha256 = evidence::sha256_serialized(transition)?;
    let parity_case = transition
        .runtime_parity_case
        .as_ref()
        .ok_or(EffectLawV3Error::InvalidTrustRoot)?;
    let capture_receipt = parity_case
        .capture_receipt
        .as_ref()
        .ok_or(EffectLawV3Error::InvalidTrustRoot)?;
    trusted
        .capture_index
        .verify_receipt(capture_receipt)
        .map_err(|_| EffectLawV3Error::InvalidTrustRoot)?;
    let (entry, parity, observed) = trust::entry(trusted, &observation.evidence_ref_sha256)?;
    if transition_sha256 != observation.transition_sha256
        || transition_sha256 != entry.transition_sha256
        || transition.before.frame_id_sha256 != observation.evidence_ref_sha256
        || parity_case.evidence_ref_sha256 != observation.evidence_ref_sha256
        || transition.before.session_id_sha256 != observation.episode_lineage_sha256
        || entry.episode_lineage_sha256 != observation.episode_lineage_sha256
        || entry.surface_root_sha256 != observation.surface_root_sha256
        || entry.physical_program_id != observation.physical_program_id
        || capture_receipt.records_root_sha256 != observation.capture_receipt_root_sha256
        || entry.capture_receipt_root_sha256 != observation.capture_receipt_root_sha256
        || entry.parity_receipt_root_sha256 != observation.parity_receipt_root_sha256
        || parity.receipt_sha256 != observation.parity_receipt_root_sha256
        || parity.verifier_sha256 != observation.verifier_root_sha256
        || observed.receipt_sha256 != observation.observed_state_root_sha256
        || trusted.resolver_root_sha256 != observation.resolver_root_sha256
        || trusted.trust_manifest_root_sha256 != observation.trust_manifest_root_sha256
        || trusted.delta_verifier_root_sha256 != observation.delta_verifier_root_sha256
    {
        return Err(EffectLawV3Error::InvalidTrustRoot);
    }
    Ok(())
}

fn censor_reason(error: EffectLawV3Error) -> EffectLawDualClassificationReasonV3 {
    match error {
        EffectLawV3Error::InvalidTrustRoot
        | EffectLawV3Error::InvalidCandidate
        | EffectLawV3Error::InvalidCaptureReceipt
        | EffectLawV3Error::InvalidParityReceipt
        | EffectLawV3Error::InvalidVerifierReceipt => {
            EffectLawDualClassificationReasonV3::TrustFailure
        }
        EffectLawV3Error::InsufficientIndependentEvidence => {
            EffectLawDualClassificationReasonV3::IndependenceFailure
        }
        EffectLawV3Error::EffectDeltaDisagreement
        | EffectLawV3Error::IncompleteEffectDelta
        | EffectLawV3Error::InvalidDictionary
        | EffectLawV3Error::NoInvariantQuotient
        | EffectLawV3Error::AmbiguousActionEquivalence
        | EffectLawV3Error::OverBudget
        | EffectLawV3Error::InvalidRestartBundle
        | EffectLawV3Error::Serialization => EffectLawDualClassificationReasonV3::EffectInvariant,
    }
}

#[cfg(test)]
#[path = "../effect_law_dual_classifier_v3_tests.rs"]
mod tests;
