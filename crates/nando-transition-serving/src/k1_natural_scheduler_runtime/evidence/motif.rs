use super::*;

const K1_MOTIF_ARCHIVE_SCHEMA_V1: &str = "nando.k1-motif-evidence-archive.v1";
const K1_MOTIF_OVERFLOW_SCHEMA_V1: &str = "nando.k1-motif-overflow-manifest.v1";
const K1_MOTIF_SOURCE_OCCURRENCES_SCHEMA_V1: &str = "nando.k1-motif-source-occurrences.v1";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct MotifCandidateKey {
    capture_generation_root_sha256: String,
    motif_root_sha256: String,
    semantic_novelty_signature_root_sha256: String,
    consequence_type: K1ConsequenceTypeV1,
}

#[derive(Debug)]
struct MotifSupportReservoir {
    key: MotifCandidateKey,
    selected_rows: usize,
    selected_lineages: BTreeSet<String>,
    retained_row_roots_sha256: Vec<String>,
    overflow_occurrences: u64,
    overflow_manifest_root_sha256: String,
}

impl MotifSupportReservoir {
    fn new(key: MotifCandidateKey) -> Result<Self, String> {
        let overflow_manifest_root_sha256 = canonical_json_sha256(&(
            K1_MOTIF_OVERFLOW_SCHEMA_V1,
            "empty",
            key.capture_generation_root_sha256.as_str(),
            key.motif_root_sha256.as_str(),
            key.semantic_novelty_signature_root_sha256.as_str(),
            key.consequence_type,
        ))
        .map_err(str::to_owned)?;
        Ok(Self {
            key,
            selected_rows: 0,
            selected_lineages: BTreeSet::new(),
            retained_row_roots_sha256: Vec::new(),
            overflow_occurrences: 0,
            overflow_manifest_root_sha256,
        })
    }

    fn should_retain(&mut self, lineage_root_sha256: &str) -> bool {
        if self.selected_rows >= K1_MAX_SUPPORT_ROWS_V1 {
            return false;
        }
        let lineage_already_selected = self.selected_lineages.contains(lineage_root_sha256);
        let selected_lineages = u64::try_from(self.selected_lineages.len()).unwrap_or(u64::MAX);
        let reserved_lineage_slots = usize::try_from(
            K1_CANDIDATE_READINESS_MIN_LINEAGES_V1.saturating_sub(selected_lineages),
        )
        .unwrap_or(K1_MAX_SUPPORT_ROWS_V1);
        let existing_lineage_limit = K1_MAX_SUPPORT_ROWS_V1.saturating_sub(reserved_lineage_slots);
        if lineage_already_selected
            && reserved_lineage_slots > 0
            && self.selected_rows >= existing_lineage_limit
        {
            return false;
        }
        self.selected_rows = self.selected_rows.saturating_add(1);
        self.selected_lineages
            .insert(lineage_root_sha256.to_owned());
        true
    }

    fn record_retained(&mut self, row_root_sha256: String) {
        self.retained_row_roots_sha256.push(row_root_sha256);
    }

    fn record_overflow(&mut self, row: &K1NaturalEvidenceRowV1) -> Result<(), String> {
        self.overflow_occurrences = self.overflow_occurrences.saturating_add(1);
        self.overflow_manifest_root_sha256 = canonical_json_sha256(&(
            K1_MOTIF_OVERFLOW_SCHEMA_V1,
            "append",
            self.overflow_manifest_root_sha256.as_str(),
            self.overflow_occurrences,
            row.row_root_sha256.as_str(),
            row.capture_sequence,
            row.lineage_root_sha256.as_str(),
        ))
        .map_err(str::to_owned)?;
        Ok(())
    }

    fn finish(self) -> Result<K1MotifCandidateSupportV1, String> {
        let retained_rows = u64::try_from(self.retained_row_roots_sha256.len())
            .map_err(|_| "k1_motif_retained_count".to_owned())?;
        let retained_manifest_root_sha256 = canonical_json_sha256(&(
            "nando.k1-motif-evidence-manifest.v1",
            self.retained_row_roots_sha256
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        ))
        .map_err(str::to_owned)?;
        K1MotifCandidateSupportV1::seal(
            self.key.capture_generation_root_sha256,
            self.key.motif_root_sha256,
            self.key.semantic_novelty_signature_root_sha256,
            self.key.consequence_type,
            retained_rows,
            retained_manifest_root_sha256,
            self.overflow_occurrences,
            self.overflow_manifest_root_sha256,
        )
        .map_err(str::to_owned)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::k1_natural_scheduler_runtime) struct MotifOccurrenceBinding {
    pub(in crate::k1_natural_scheduler_runtime) row: K1NaturalEvidenceRowV1,
    pub(in crate::k1_natural_scheduler_runtime) ambient_index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::k1_natural_scheduler_runtime) struct ExactMotifOccurrenceV1 {
    pub(in crate::k1_natural_scheduler_runtime) row: K1NaturalEvidenceRowV1,
    pub(in crate::k1_natural_scheduler_runtime) motif: SourceNeutralTopologyMotifV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MotifAmbientBinding {
    arena_binding_index: usize,
    join_root_sha256: String,
    complete_topology_root_sha256: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::k1_natural_scheduler_runtime) struct MotifEvidenceArchive {
    pub(in crate::k1_natural_scheduler_runtime) archive_root_sha256: String,
    ambient: Vec<MotifAmbientBinding>,
    pub(in crate::k1_natural_scheduler_runtime) occurrences: Vec<MotifOccurrenceBinding>,
    pub(in crate::k1_natural_scheduler_runtime) candidate_supports: Vec<K1MotifCandidateSupportV1>,
    pub(in crate::k1_natural_scheduler_runtime) source_dispositions:
        Vec<K1MotifSourceDispositionV1>,
    pub(in crate::k1_natural_scheduler_runtime) disposition: K1MotifDispositionSummaryV1,
}

#[derive(Serialize)]
struct MotifArchiveDigestV1<'a> {
    schema: &'static str,
    ambient_bindings: Vec<(usize, &'a str, &'a str)>,
    occurrence_bindings: Vec<(&'a str, usize)>,
    candidate_support_roots_sha256: Vec<&'a str>,
    source_disposition_roots_sha256: Vec<&'a str>,
    disposition_summary_root_sha256: &'a str,
}

impl MotifEvidenceArchive {
    pub(in crate::k1_natural_scheduler_runtime) fn evidence_rows(
        &self,
    ) -> Vec<K1NaturalEvidenceRowV1> {
        self.occurrences
            .iter()
            .map(|binding| binding.row.clone())
            .collect()
    }

    #[cfg(test)]
    fn ambient_payloads(&self) -> usize {
        self.ambient.len()
    }

    pub(in crate::k1_natural_scheduler_runtime) fn joined_for<'a>(
        &'a self,
        arena: &'a [EvidenceBinding],
        binding: &MotifOccurrenceBinding,
    ) -> Result<&'a BlindThenRevealJoinedTransitionV1, String> {
        let ambient = self
            .ambient
            .get(binding.ambient_index)
            .ok_or_else(|| "k1_motif_archive_ambient_missing".to_owned())?;
        let arena_binding = arena
            .get(ambient.arena_binding_index)
            .ok_or_else(|| "k1_motif_archive_arena_binding_missing".to_owned())?;
        let joined = arena_binding.joined();
        if joined.join_root_sha256 != ambient.join_root_sha256 {
            return Err("k1_motif_archive_arena_root_mismatch".to_owned());
        }
        Ok(joined)
    }

    pub(in crate::k1_natural_scheduler_runtime) fn exact_occurrence(
        &self,
        arena: &[EvidenceBinding],
        binding: &MotifOccurrenceBinding,
    ) -> Result<ExactMotifOccurrenceV1, String> {
        let joined = self.joined_for(arena, binding)?;
        let occurrence = exact_motif_occurrences_for_joined(joined)?
            .into_iter()
            .find(|occurrence| occurrence.row.row_root_sha256 == binding.row.row_root_sha256)
            .ok_or_else(|| "k1_motif_archive_exact_occurrence_missing".to_owned())?;
        if occurrence.row != binding.row {
            return Err("k1_motif_archive_exact_occurrence_mismatch".to_owned());
        }
        Ok(occurrence)
    }

    fn expected_root(&self) -> Result<String, String> {
        canonical_json_sha256(&MotifArchiveDigestV1 {
            schema: K1_MOTIF_ARCHIVE_SCHEMA_V1,
            ambient_bindings: self
                .ambient
                .iter()
                .map(|binding| {
                    (
                        binding.arena_binding_index,
                        binding.join_root_sha256.as_str(),
                        binding.complete_topology_root_sha256.as_str(),
                    )
                })
                .collect(),
            occurrence_bindings: self
                .occurrences
                .iter()
                .map(|binding| (binding.row.row_root_sha256.as_str(), binding.ambient_index))
                .collect(),
            candidate_support_roots_sha256: self
                .candidate_supports
                .iter()
                .map(|support| support.support_root_sha256.as_str())
                .collect(),
            source_disposition_roots_sha256: self
                .source_dispositions
                .iter()
                .map(|receipt| receipt.disposition_root_sha256.as_str())
                .collect(),
            disposition_summary_root_sha256: &self.disposition.summary_root_sha256,
        })
        .map_err(str::to_owned)
    }

    pub(in crate::k1_natural_scheduler_runtime) fn validate(
        &self,
        arena: &[EvidenceBinding],
    ) -> Result<(), String> {
        self.disposition.validate().map_err(str::to_owned)?;
        if self
            .candidate_supports
            .iter()
            .any(|support| support.validate().is_err())
            || self
                .source_dispositions
                .iter()
                .any(|receipt| receipt.validate().is_err())
        {
            return Err("k1_motif_archive_receipt_invalid".to_owned());
        }
        let mut used_ambient = BTreeSet::new();
        let mut retained_by_candidate =
            BTreeMap::<MotifCandidateKey, Vec<&K1NaturalEvidenceRowV1>>::new();
        let mut retained_by_source = BTreeMap::<&str, u64>::new();
        let mut occurrence_roots = BTreeSet::new();
        for binding in &self.occurrences {
            binding.row.validate().map_err(str::to_owned)?;
            if !occurrence_roots.insert(binding.row.row_root_sha256.as_str()) {
                return Err("k1_motif_archive_occurrence_reused".to_owned());
            }
            let joined = self.joined_for(arena, binding)?;
            let ambient = &self.ambient[binding.ambient_index];
            let complete_topology_root_sha256 =
                canonical_json_sha256(&joined.topology).map_err(str::to_owned)?;
            if binding.row.evidence_root_sha256 != joined.join_root_sha256
                || binding.row.complete_topology_root_sha256 != complete_topology_root_sha256
                || ambient.complete_topology_root_sha256 != complete_topology_root_sha256
            {
                return Err("k1_motif_archive_ambient_binding_invalid".to_owned());
            }
            used_ambient.insert(binding.ambient_index);
            retained_by_candidate
                .entry(MotifCandidateKey {
                    capture_generation_root_sha256: binding
                        .row
                        .capture_generation_root_sha256
                        .clone(),
                    motif_root_sha256: binding.row.motif_root_sha256.clone(),
                    semantic_novelty_signature_root_sha256: binding
                        .row
                        .semantic_novelty_signature_root_sha256
                        .clone(),
                    consequence_type: binding.row.consequence_type,
                })
                .or_default()
                .push(&binding.row);
            *retained_by_source
                .entry(binding.row.evidence_root_sha256.as_str())
                .or_default() += 1;
        }
        let ambient_roots = self
            .ambient
            .iter()
            .map(|binding| binding.join_root_sha256.as_str())
            .collect::<BTreeSet<_>>();
        if ambient_roots.len() != self.ambient.len() {
            return Err("k1_motif_archive_ambient_reused".to_owned());
        }

        let mut support_by_candidate = BTreeMap::new();
        for support in &self.candidate_supports {
            let key = MotifCandidateKey {
                capture_generation_root_sha256: support.capture_generation_root_sha256.clone(),
                motif_root_sha256: support.motif_root_sha256.clone(),
                semantic_novelty_signature_root_sha256: support
                    .semantic_novelty_signature_root_sha256
                    .clone(),
                consequence_type: support.consequence_type,
            };
            if support_by_candidate.insert(key, support).is_some() {
                return Err("k1_motif_archive_support_reused".to_owned());
            }
        }
        for (key, mut rows) in retained_by_candidate {
            rows.sort_by(|left, right| {
                left.capture_sequence
                    .cmp(&right.capture_sequence)
                    .then_with(|| left.row_root_sha256.cmp(&right.row_root_sha256))
            });
            let support = support_by_candidate
                .remove(&key)
                .ok_or_else(|| "k1_motif_archive_support_missing".to_owned())?;
            let retained_manifest_root_sha256 = canonical_json_sha256(&(
                "nando.k1-motif-evidence-manifest.v1",
                rows.iter()
                    .map(|row| row.row_root_sha256.as_str())
                    .collect::<Vec<_>>(),
            ))
            .map_err(str::to_owned)?;
            if support.retained_rows != u64::try_from(rows.len()).unwrap_or(u64::MAX)
                || support.retained_manifest_root_sha256 != retained_manifest_root_sha256
            {
                return Err("k1_motif_archive_support_incoherent".to_owned());
            }
        }
        if !support_by_candidate.is_empty() {
            return Err("k1_motif_archive_support_orphaned".to_owned());
        }

        let source_dispositions_ordered = self.source_dispositions.windows(2).all(|pair| {
            (
                pair[0].capture_sequence,
                pair[0].evidence_root_sha256.as_str(),
            ) < (
                pair[1].capture_sequence,
                pair[1].evidence_root_sha256.as_str(),
            )
        });
        let source_evidence_roots = self
            .source_dispositions
            .iter()
            .map(|receipt| receipt.evidence_root_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let retained_from_sources = self
            .source_dispositions
            .iter()
            .map(|receipt| receipt.retained_occurrences)
            .try_fold(0u64, u64::checked_add)
            .ok_or_else(|| "k1_motif_archive_source_count".to_owned())?;
        let overflow_from_sources = self
            .source_dispositions
            .iter()
            .map(|receipt| receipt.overflow_occurrences)
            .try_fold(0u64, u64::checked_add)
            .ok_or_else(|| "k1_motif_archive_source_count".to_owned())?;
        if !source_dispositions_ordered
            || source_evidence_roots.len() != self.source_dispositions.len()
            || self.source_dispositions.iter().any(|receipt| {
                retained_by_source
                    .get(receipt.evidence_root_sha256.as_str())
                    .copied()
                    .unwrap_or(0)
                    != receipt.retained_occurrences
            })
        {
            return Err("k1_motif_archive_source_disposition_incoherent".to_owned());
        }
        let support_overflow_occurrences = self
            .candidate_supports
            .iter()
            .map(|support| support.overflow_occurrences)
            .try_fold(0u64, u64::checked_add)
            .ok_or_else(|| "k1_motif_archive_support_count".to_owned())?;
        let support_manifest_root_sha256 = canonical_json_sha256(&(
            "nando.k1-motif-candidate-support-manifest.v1",
            self.candidate_supports
                .iter()
                .map(|support| support.support_root_sha256.as_str())
                .collect::<BTreeSet<_>>(),
        ))
        .map_err(str::to_owned)?;
        let class_count = |class| {
            u64::try_from(
                self.source_dispositions
                    .iter()
                    .filter(|receipt| receipt.class == class)
                    .count(),
            )
            .unwrap_or(u64::MAX)
        };
        let class_manifest = |label: &str, class| {
            canonical_json_sha256(&(
                label,
                self.source_dispositions
                    .iter()
                    .filter(|receipt| receipt.class == class)
                    .map(|receipt| receipt.disposition_root_sha256.as_str())
                    .collect::<Vec<_>>(),
            ))
            .map_err(str::to_owned)
        };
        let source_disposition_manifest_root_sha256 = canonical_json_sha256(&(
            "nando.k1-motif-source-disposition-manifest.v1",
            self.source_dispositions
                .iter()
                .map(|receipt| receipt.disposition_root_sha256.as_str())
                .collect::<Vec<_>>(),
        ))
        .map_err(str::to_owned)?;
        let motif_source_rows = class_count(K1MotifSourceDispositionClassV1::MotifRetained)
            .saturating_add(class_count(
                K1MotifSourceDispositionClassV1::MotifSupportOverflow,
            ));
        let summaries_match = self.disposition.motif_source_rows == motif_source_rows
            && self.disposition.retained_motif_occurrences == retained_from_sources
            && self.disposition.support_overflow_occurrences == overflow_from_sources
            && overflow_from_sources == support_overflow_occurrences
            && self.disposition.support_overflow_manifest_root_sha256
                == support_manifest_root_sha256
            && self.disposition.budget_censored_rows
                == class_count(K1MotifSourceDispositionClassV1::CensoredMotifEnumerationBudget)
            && self.disposition.budget_censored_manifest_root_sha256
                == class_manifest(
                    "nando.k1-motif-budget-disposition-manifest.v1",
                    K1MotifSourceDispositionClassV1::CensoredMotifEnumerationBudget,
                )?
            && self.disposition.empty_or_incomplete_rows
                == class_count(K1MotifSourceDispositionClassV1::CensoredEmptyOrIncompleteTopology)
            && self.disposition.empty_or_incomplete_manifest_root_sha256
                == class_manifest(
                    "nando.k1-motif-empty-disposition-manifest.v1",
                    K1MotifSourceDispositionClassV1::CensoredEmptyOrIncompleteTopology,
                )?
            && self.disposition.invalid_embedding_rows
                == class_count(K1MotifSourceDispositionClassV1::CensoredInvalidEmbedding)
            && self.disposition.invalid_embedding_manifest_root_sha256
                == class_manifest(
                    "nando.k1-motif-invalid-disposition-manifest.v1",
                    K1MotifSourceDispositionClassV1::CensoredInvalidEmbedding,
                )?
            && self.disposition.fixture_or_controlled_excluded_rows
                == class_count(K1MotifSourceDispositionClassV1::FixtureOrControlledExcluded)
            && self.disposition.fixture_or_controlled_manifest_root_sha256
                == class_manifest(
                    "nando.k1-motif-fixture-disposition-manifest.v1",
                    K1MotifSourceDispositionClassV1::FixtureOrControlledExcluded,
                )?
            && self.disposition.safety_veto_rows
                == class_count(K1MotifSourceDispositionClassV1::SafetyVeto)
            && self.disposition.safety_veto_manifest_root_sha256
                == class_manifest(
                    "nando.k1-motif-safety-disposition-manifest.v1",
                    K1MotifSourceDispositionClassV1::SafetyVeto,
                )?
            && self.disposition.source_disposition_manifest_root_sha256
                == source_disposition_manifest_root_sha256;
        if !summaries_match {
            return Err("k1_motif_archive_summary_incoherent".to_owned());
        }
        if used_ambient.len() != self.ambient.len()
            || self.disposition.retained_motif_occurrences
                != u64::try_from(self.occurrences.len()).unwrap_or(u64::MAX)
            || self.disposition.scanned_source_rows
                != u64::try_from(self.source_dispositions.len()).unwrap_or(u64::MAX)
            || self.archive_root_sha256 != self.expected_root()?
        {
            return Err("k1_motif_archive_invalid".to_owned());
        }
        Ok(())
    }
}

pub(in crate::k1_natural_scheduler_runtime) fn exact_motif_occurrences_for_joined(
    joined: &BlindThenRevealJoinedTransitionV1,
) -> Result<Vec<ExactMotifOccurrenceV1>, String> {
    if !capture_generation_v2(joined)
        || joined.topology.validate().is_err()
        || !matches!(
            joined.topology.extraction_status,
            MultiSourceExtractionStatusV1::Complete
        )
    {
        return Err("k1_motif_exact_source_invalid".to_owned());
    }
    let complete_topology_root_sha256 =
        canonical_json_sha256(&joined.topology).map_err(str::to_owned)?;
    let motifs = source_neutral_topology_motifs_v1(&joined.topology).map_err(str::to_owned)?;
    if motifs.is_empty()
        || motifs.iter().any(|motif| {
            motif.validate().is_err()
                || motif.embeddings.iter().any(|embedding| {
                    embedding.ambient_topology_root_sha256 != complete_topology_root_sha256
                })
        })
    {
        return Err("k1_motif_exact_embedding_invalid".to_owned());
    }
    let factorized = factor_multi_source_row_v1(joined);
    let consequence_type = consequence_type(joined, factorized.completed_effect);
    let semantic_novelty_signature_root_sha256 =
        canonical_json_sha256(&(K1_SEMANTIC_NOVELTY_SCHEMA_V1, consequence_type))
            .map_err(str::to_owned)?;
    motifs
        .into_iter()
        .map(|motif| {
            let row = K1NaturalEvidenceRowV1::seal_motif_v4(
                joined.join_root_sha256.clone(),
                joined.capture_generation_root_sha256.clone(),
                complete_topology_root_sha256.clone(),
                &motif,
                semantic_novelty_signature_root_sha256.clone(),
                joined.session_lineage_sha256.clone(),
                consequence_type,
                joined.capture_sequence,
                joined.capture_sequence,
                joined.input_tokens,
                true,
                joined.accepted,
                false,
            )
            .map_err(str::to_owned)?;
            Ok(ExactMotifOccurrenceV1 { row, motif })
        })
        .collect()
}

pub(in crate::k1_natural_scheduler_runtime) struct MotifEvidenceAccumulator {
    last_order_key: Option<(u64, String)>,
    seen_evidence_roots_sha256: BTreeSet<String>,
    reservoirs: BTreeMap<MotifCandidateKey, MotifSupportReservoir>,
    ambient: Vec<MotifAmbientBinding>,
    occurrences: Vec<MotifOccurrenceBinding>,
    source_dispositions: Vec<K1MotifSourceDispositionV1>,
}

impl MotifEvidenceAccumulator {
    pub(in crate::k1_natural_scheduler_runtime) fn new() -> Self {
        Self {
            last_order_key: None,
            seen_evidence_roots_sha256: BTreeSet::new(),
            reservoirs: BTreeMap::new(),
            ambient: Vec::new(),
            occurrences: Vec::new(),
            source_dispositions: Vec::new(),
        }
    }

    pub(in crate::k1_natural_scheduler_runtime) fn resume(
        archive: MotifEvidenceArchive,
        arena: &[EvidenceBinding],
    ) -> Result<Self, String> {
        archive.validate(arena)?;
        let mut retained_by_candidate =
            BTreeMap::<MotifCandidateKey, (Vec<String>, BTreeSet<String>)>::new();
        for binding in &archive.occurrences {
            let state = retained_by_candidate
                .entry(MotifCandidateKey {
                    capture_generation_root_sha256: binding
                        .row
                        .capture_generation_root_sha256
                        .clone(),
                    motif_root_sha256: binding.row.motif_root_sha256.clone(),
                    semantic_novelty_signature_root_sha256: binding
                        .row
                        .semantic_novelty_signature_root_sha256
                        .clone(),
                    consequence_type: binding.row.consequence_type,
                })
                .or_default();
            state.0.push(binding.row.row_root_sha256.clone());
            state.1.insert(binding.row.lineage_root_sha256.clone());
        }
        let mut reservoirs = BTreeMap::new();
        for support in &archive.candidate_supports {
            let key = MotifCandidateKey {
                capture_generation_root_sha256: support.capture_generation_root_sha256.clone(),
                motif_root_sha256: support.motif_root_sha256.clone(),
                semantic_novelty_signature_root_sha256: support
                    .semantic_novelty_signature_root_sha256
                    .clone(),
                consequence_type: support.consequence_type,
            };
            let (retained_row_roots_sha256, selected_lineages) = retained_by_candidate
                .remove(&key)
                .ok_or_else(|| "k1_motif_resume_support_orphaned".to_owned())?;
            let selected_rows = retained_row_roots_sha256.len();
            if support.retained_rows != u64::try_from(selected_rows).unwrap_or(u64::MAX) {
                return Err("k1_motif_resume_support_count".to_owned());
            }
            reservoirs.insert(
                key.clone(),
                MotifSupportReservoir {
                    key,
                    selected_rows,
                    selected_lineages,
                    retained_row_roots_sha256,
                    overflow_occurrences: support.overflow_occurrences,
                    overflow_manifest_root_sha256: support.overflow_manifest_root_sha256.clone(),
                },
            );
        }
        if !retained_by_candidate.is_empty() {
            return Err("k1_motif_resume_support_missing".to_owned());
        }
        let last_order_key = archive.source_dispositions.last().map(|receipt| {
            (
                receipt.capture_sequence,
                receipt.evidence_root_sha256.clone(),
            )
        });
        let seen_evidence_roots_sha256 = archive
            .source_dispositions
            .iter()
            .map(|receipt| receipt.evidence_root_sha256.clone())
            .collect();
        Ok(Self {
            last_order_key,
            seen_evidence_roots_sha256,
            reservoirs,
            ambient: archive.ambient,
            occurrences: archive.occurrences,
            source_dispositions: archive.source_dispositions,
        })
    }

    pub(in crate::k1_natural_scheduler_runtime) fn push_natural(
        &mut self,
        arena_binding_index: usize,
        joined: &BlindThenRevealJoinedTransitionV1,
    ) -> Result<(), String> {
        self.push(
            arena_binding_index,
            joined,
            K1NaturalEvidenceClassV1::NaturalLive,
            false,
        )
    }

    pub(in crate::k1_natural_scheduler_runtime) fn push(
        &mut self,
        arena_binding_index: usize,
        joined: &BlindThenRevealJoinedTransitionV1,
        evidence_class: K1NaturalEvidenceClassV1,
        safety_veto: bool,
    ) -> Result<(), String> {
        let order_key = (joined.capture_sequence, joined.join_root_sha256.clone());
        if self
            .last_order_key
            .as_ref()
            .is_some_and(|previous| previous >= &order_key)
        {
            return Err("k1_motif_evidence_out_of_order".to_owned());
        }
        if !self
            .seen_evidence_roots_sha256
            .insert(joined.join_root_sha256.clone())
        {
            return Err("k1_motif_evidence_reused".to_owned());
        }
        self.last_order_key = Some(order_key);
        let complete_topology_root_sha256 =
            canonical_json_sha256(&joined.topology).map_err(str::to_owned)?;

        match evidence_class {
            K1NaturalEvidenceClassV1::Controlled
            | K1NaturalEvidenceClassV1::GeneratedMs5
            | K1NaturalEvidenceClassV1::GeneratedMs6 => {
                return self.record_empty_disposition(
                    joined,
                    complete_topology_root_sha256,
                    K1MotifSourceDispositionClassV1::FixtureOrControlledExcluded,
                );
            }
            K1NaturalEvidenceClassV1::Unknown => {
                return self.record_empty_disposition(
                    joined,
                    complete_topology_root_sha256,
                    K1MotifSourceDispositionClassV1::SafetyVeto,
                );
            }
            K1NaturalEvidenceClassV1::NaturalLive => {}
        }
        if safety_veto || !capture_generation_v2(joined) {
            return self.record_empty_disposition(
                joined,
                complete_topology_root_sha256,
                K1MotifSourceDispositionClassV1::SafetyVeto,
            );
        }
        if joined.topology.validate().is_err() {
            return self.record_empty_disposition(
                joined,
                complete_topology_root_sha256,
                K1MotifSourceDispositionClassV1::CensoredInvalidEmbedding,
            );
        }
        let motifs = match source_neutral_topology_motifs_v1(&joined.topology) {
            Ok(motifs) if motifs.is_empty() => {
                return self.record_empty_disposition(
                    joined,
                    complete_topology_root_sha256,
                    K1MotifSourceDispositionClassV1::CensoredEmptyOrIncompleteTopology,
                );
            }
            Ok(motifs) => motifs,
            Err("source_neutral_topology_motif_budget") => {
                return self.record_empty_disposition(
                    joined,
                    complete_topology_root_sha256,
                    K1MotifSourceDispositionClassV1::CensoredMotifEnumerationBudget,
                );
            }
            Err("source_neutral_topology_motif_censored") => {
                return self.record_empty_disposition(
                    joined,
                    complete_topology_root_sha256,
                    K1MotifSourceDispositionClassV1::CensoredEmptyOrIncompleteTopology,
                );
            }
            Err(_) => {
                return self.record_empty_disposition(
                    joined,
                    complete_topology_root_sha256,
                    K1MotifSourceDispositionClassV1::CensoredInvalidEmbedding,
                );
            }
        };
        if motifs.iter().any(|motif| {
            motif.validate().is_err()
                || motif.embeddings.iter().any(|embedding| {
                    embedding.ambient_topology_root_sha256 != complete_topology_root_sha256
                })
        }) {
            return self.record_empty_disposition(
                joined,
                complete_topology_root_sha256,
                K1MotifSourceDispositionClassV1::CensoredInvalidEmbedding,
            );
        }

        let factorized = factor_multi_source_row_v1(joined);
        let consequence_type = consequence_type(joined, factorized.completed_effect);
        let semantic_novelty_signature_root_sha256 =
            canonical_json_sha256(&(K1_SEMANTIC_NOVELTY_SCHEMA_V1, consequence_type))
                .map_err(str::to_owned)?;
        let mut retained_rows = Vec::new();
        let mut source_occurrences = Vec::with_capacity(motifs.len());
        let mut overflow_occurrences = 0u64;
        for motif in motifs {
            let row = K1NaturalEvidenceRowV1::seal_motif_v4(
                joined.join_root_sha256.clone(),
                joined.capture_generation_root_sha256.clone(),
                complete_topology_root_sha256.clone(),
                &motif,
                semantic_novelty_signature_root_sha256.clone(),
                joined.session_lineage_sha256.clone(),
                consequence_type,
                joined.capture_sequence,
                joined.capture_sequence,
                joined.input_tokens,
                true,
                joined.accepted,
                false,
            )
            .map_err(str::to_owned)?;
            let key = MotifCandidateKey {
                capture_generation_root_sha256: joined.capture_generation_root_sha256.clone(),
                motif_root_sha256: motif.motif_root_sha256.clone(),
                semantic_novelty_signature_root_sha256: semantic_novelty_signature_root_sha256
                    .clone(),
                consequence_type,
            };
            if !self.reservoirs.contains_key(&key) {
                self.reservoirs
                    .insert(key.clone(), MotifSupportReservoir::new(key.clone())?);
            }
            let reservoir = self
                .reservoirs
                .get_mut(&key)
                .ok_or_else(|| "k1_motif_reservoir_missing".to_owned())?;
            if reservoir.should_retain(&joined.session_lineage_sha256) {
                reservoir.record_retained(row.row_root_sha256.clone());
                source_occurrences.push((
                    motif.motif_root_sha256,
                    row.row_root_sha256.clone(),
                    "retained",
                ));
                retained_rows.push(row);
            } else {
                reservoir.record_overflow(&row)?;
                overflow_occurrences = overflow_occurrences.saturating_add(1);
                source_occurrences.push((motif.motif_root_sha256, row.row_root_sha256, "overflow"));
            }
        }
        let retained_occurrences =
            u64::try_from(retained_rows.len()).map_err(|_| "k1_motif_retained_count".to_owned())?;
        let enumerated_occurrences = u64::try_from(source_occurrences.len())
            .map_err(|_| "k1_motif_enumerated_count".to_owned())?;
        let occurrence_manifest_root_sha256 = canonical_json_sha256(&(
            K1_MOTIF_SOURCE_OCCURRENCES_SCHEMA_V1,
            joined.join_root_sha256.as_str(),
            source_occurrences,
        ))
        .map_err(str::to_owned)?;
        let class = if retained_occurrences > 0 {
            K1MotifSourceDispositionClassV1::MotifRetained
        } else {
            K1MotifSourceDispositionClassV1::MotifSupportOverflow
        };
        let receipt = K1MotifSourceDispositionV1::seal(
            joined.join_root_sha256.clone(),
            complete_topology_root_sha256,
            joined.capture_sequence,
            class,
            enumerated_occurrences,
            retained_occurrences,
            overflow_occurrences,
            occurrence_manifest_root_sha256,
        )
        .map_err(str::to_owned)?;
        if !retained_rows.is_empty() {
            let ambient_index = self.ambient.len();
            self.ambient.push(MotifAmbientBinding {
                arena_binding_index,
                join_root_sha256: joined.join_root_sha256.clone(),
                complete_topology_root_sha256: receipt.complete_topology_root_sha256.clone(),
            });
            self.occurrences.extend(
                retained_rows
                    .into_iter()
                    .map(|row| MotifOccurrenceBinding { row, ambient_index }),
            );
        }
        self.source_dispositions.push(receipt);
        Ok(())
    }

    fn record_empty_disposition(
        &mut self,
        joined: &BlindThenRevealJoinedTransitionV1,
        complete_topology_root_sha256: String,
        class: K1MotifSourceDispositionClassV1,
    ) -> Result<(), String> {
        let occurrence_manifest_root_sha256 = canonical_json_sha256(&(
            K1_MOTIF_SOURCE_OCCURRENCES_SCHEMA_V1,
            joined.join_root_sha256.as_str(),
            Vec::<String>::new(),
        ))
        .map_err(str::to_owned)?;
        let receipt = K1MotifSourceDispositionV1::seal(
            joined.join_root_sha256.clone(),
            complete_topology_root_sha256,
            joined.capture_sequence,
            class,
            0,
            0,
            0,
            occurrence_manifest_root_sha256,
        )
        .map_err(str::to_owned)?;
        self.source_dispositions.push(receipt);
        Ok(())
    }

    pub(in crate::k1_natural_scheduler_runtime) fn finish(
        self,
        arena: &[EvidenceBinding],
    ) -> Result<MotifEvidenceArchive, String> {
        let candidate_supports = self
            .reservoirs
            .into_values()
            .map(MotifSupportReservoir::finish)
            .collect::<Result<Vec<_>, _>>()?;
        let support_overflow_occurrences = candidate_supports
            .iter()
            .map(|support| support.overflow_occurrences)
            .try_fold(0u64, u64::checked_add)
            .ok_or_else(|| "k1_motif_overflow_count".to_owned())?;
        let support_overflow_manifest_root_sha256 = canonical_json_sha256(&(
            "nando.k1-motif-candidate-support-manifest.v1",
            candidate_supports
                .iter()
                .map(|support| support.support_root_sha256.as_str())
                .collect::<BTreeSet<_>>(),
        ))
        .map_err(str::to_owned)?;
        let class_count = |class| {
            u64::try_from(
                self.source_dispositions
                    .iter()
                    .filter(|receipt| receipt.class == class)
                    .count(),
            )
            .unwrap_or(u64::MAX)
        };
        let class_manifest = |label: &str, class| {
            canonical_json_sha256(&(
                label,
                self.source_dispositions
                    .iter()
                    .filter(|receipt| receipt.class == class)
                    .map(|receipt| receipt.disposition_root_sha256.as_str())
                    .collect::<Vec<_>>(),
            ))
            .map_err(str::to_owned)
        };
        let motif_source_rows = self
            .source_dispositions
            .iter()
            .filter(|receipt| {
                matches!(
                    receipt.class,
                    K1MotifSourceDispositionClassV1::MotifRetained
                        | K1MotifSourceDispositionClassV1::MotifSupportOverflow
                )
            })
            .count();
        let fixture_rows =
            class_count(K1MotifSourceDispositionClassV1::FixtureOrControlledExcluded);
        let safety_rows = class_count(K1MotifSourceDispositionClassV1::SafetyVeto);
        let budget_rows =
            class_count(K1MotifSourceDispositionClassV1::CensoredMotifEnumerationBudget);
        let empty_rows =
            class_count(K1MotifSourceDispositionClassV1::CensoredEmptyOrIncompleteTopology);
        let invalid_rows = class_count(K1MotifSourceDispositionClassV1::CensoredInvalidEmbedding);
        let source_disposition_manifest_root_sha256 = canonical_json_sha256(&(
            "nando.k1-motif-source-disposition-manifest.v1",
            self.source_dispositions
                .iter()
                .map(|receipt| receipt.disposition_root_sha256.as_str())
                .collect::<Vec<_>>(),
        ))
        .map_err(str::to_owned)?;
        let disposition = K1MotifDispositionSummaryV1::seal(
            source_neutral_topology_motif_config_root_v1().map_err(str::to_owned)?,
            u64::try_from(self.source_dispositions.len())
                .map_err(|_| "k1_motif_source_count".to_owned())?,
            u64::try_from(motif_source_rows).map_err(|_| "k1_motif_source_count".to_owned())?,
            u64::try_from(self.occurrences.len())
                .map_err(|_| "k1_motif_retained_count".to_owned())?,
            support_overflow_occurrences,
            support_overflow_manifest_root_sha256,
            budget_rows,
            class_manifest(
                "nando.k1-motif-budget-disposition-manifest.v1",
                K1MotifSourceDispositionClassV1::CensoredMotifEnumerationBudget,
            )?,
            empty_rows,
            class_manifest(
                "nando.k1-motif-empty-disposition-manifest.v1",
                K1MotifSourceDispositionClassV1::CensoredEmptyOrIncompleteTopology,
            )?,
            invalid_rows,
            class_manifest(
                "nando.k1-motif-invalid-disposition-manifest.v1",
                K1MotifSourceDispositionClassV1::CensoredInvalidEmbedding,
            )?,
            fixture_rows,
            class_manifest(
                "nando.k1-motif-fixture-disposition-manifest.v1",
                K1MotifSourceDispositionClassV1::FixtureOrControlledExcluded,
            )?,
            safety_rows,
            class_manifest(
                "nando.k1-motif-safety-disposition-manifest.v1",
                K1MotifSourceDispositionClassV1::SafetyVeto,
            )?,
            source_disposition_manifest_root_sha256,
        )
        .map_err(str::to_owned)?;
        let mut archive = MotifEvidenceArchive {
            archive_root_sha256: String::new(),
            ambient: self.ambient,
            occurrences: self.occurrences,
            candidate_supports,
            source_dispositions: self.source_dispositions,
            disposition,
        };
        archive.archive_root_sha256 = archive.expected_root()?;
        archive.validate(arena)?;
        Ok(archive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nando_operator_kernel::{
        MultiSourceCardinalityClassV1, MultiSourceRelationEdgeV1, MultiSourceRelationKindV1,
        MultiSourceRoleNodeV1, MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1,
        PreActionMultiSourceTopologyV1, sha256_bytes,
    };
    use nando_operator_learning::multi_source::{
        CompletedEffectAtomV1, K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6, K1CandidateScoreV1,
        K1GenerationBudgetV1,
    };

    fn root(value: u64) -> String {
        format!("{value:064x}")
    }

    fn role(
        local_role_id: u16,
        sequence: u64,
    ) -> (MultiSourceRoleNodeV1, MultiSourceRoleWitnessV1) {
        (
            MultiSourceRoleNodeV1 {
                local_role_id,
                source_ordinal: local_role_id,
                value_ordinal: 0,
                type_class: MultiSourceTypeClassV1::Number,
                container_class: MultiSourceContainerClassV1::Scalar,
                cardinality_class: MultiSourceCardinalityClassV1::One,
                temporal_class: MultiSourceTemporalClassV1::Latest,
                depth_bucket: 1,
                structural_flags: 1,
            },
            MultiSourceRoleWitnessV1 {
                local_role_id,
                value_sha256: root(20_000 + sequence * 100 + u64::from(local_role_id)),
                request_reference_ordinal: None,
                request_reference_ordinal_candidates: Vec::new(),
            },
        )
    }

    fn joined(sequence: u64, lineage: u64) -> BlindThenRevealJoinedTransitionV1 {
        let extractor = sha256_bytes(b"nando.multi-source-extractor.v4");
        let config = sha256_bytes(b"nando.multi-source-extractor-config.v4");
        let generation = canonical_json_sha256(&(
            nando_operator_learning::multi_source::MULTI_SOURCE_CAPTURE_GENERATION_SCHEMA_V2,
            extractor.as_str(),
            config.as_str(),
        ))
        .expect("generation");
        let (role, witness) = role(0, sequence);
        BlindThenRevealJoinedTransitionV1 {
            schema: nando_operator_learning::multi_source::BLIND_THEN_REVEAL_JOIN_SCHEMA_V2
                .to_owned(),
            join_root_sha256: root(100 + sequence),
            capture_sequence: sequence,
            turn_intent_id_sha256: root(200 + sequence),
            request_event_id_sha256: root(300 + sequence),
            action_event_id_sha256: root(400 + sequence),
            session_lineage_sha256: root(lineage),
            session_id_sha256: root(500 + sequence),
            topology_commitment_root_sha256: root(600 + sequence),
            extractor_root_sha256: extractor,
            extractor_config_root_sha256: config,
            capture_generation_root_sha256: generation,
            pre_action_record_root_sha256: root(700 + sequence),
            completed_frame_root_sha256: root(800 + sequence),
            physical_action_root_sha256: root(900 + sequence),
            semantic_action_root_sha256: root(1_000 + sequence),
            effect_atoms: vec![CompletedEffectAtomV1::ValueProjection],
            verifier_receipt_root_sha256: root(1_100 + sequence),
            input_tokens: 10 + sequence,
            captured_at_unix_ms: 1_000 + sequence,
            completed_at_unix_nanos: (1_001 + sequence) * 1_000_000,
            accepted: true,
            topology: PreActionMultiSourceTopologyV1 {
                extraction_status: MultiSourceExtractionStatusV1::Complete,
                grounded_output_count: 1,
                output_part_count: 1,
                roles: vec![role],
                role_witnesses: vec![witness],
                relations: Vec::new(),
            },
        }
    }

    fn chain_joined(sequence: u64, role_count: u16) -> BlindThenRevealJoinedTransitionV1 {
        let mut joined = joined(sequence, sequence);
        let roles = (0..role_count)
            .map(|local_role_id| role(local_role_id, sequence))
            .collect::<Vec<_>>();
        joined.topology.roles = roles.iter().map(|pair| pair.0.clone()).collect();
        joined.topology.role_witnesses = roles.into_iter().map(|pair| pair.1).collect();
        joined.topology.relations = (0..role_count.saturating_sub(1))
            .map(|source_role_id| MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::Precedes,
                source_role_id,
                target_role_id: source_role_id + 1,
            })
            .collect();
        joined
    }

    fn arena_binding(joined: BlindThenRevealJoinedTransitionV1) -> EvidenceBinding {
        let completed_frame_root_sha256 = joined.completed_frame_root_sha256.clone();
        let topology_commitment_root_sha256 = joined.topology_commitment_root_sha256.clone();
        let row = K1NaturalEvidenceRowV1::seal(
            joined.join_root_sha256.clone(),
            joined.capture_generation_root_sha256.clone(),
            root(30_001),
            root(30_002),
            root(30_003),
            joined.session_lineage_sha256.clone(),
            K1ConsequenceTypeV1::Scalar,
            K1NaturalEvidenceClassV1::NaturalLive,
            joined.capture_sequence,
            joined.capture_sequence,
            joined.input_tokens,
            true,
            joined.accepted,
            true,
        )
        .expect("arena row");
        EvidenceBinding {
            row,
            joined: Some(Box::new(joined)),
            completed_frame_root_sha256,
            topology_commitment_root_sha256,
        }
    }

    fn build_archive(
        observations: Vec<(
            BlindThenRevealJoinedTransitionV1,
            K1NaturalEvidenceClassV1,
            bool,
        )>,
    ) -> (Vec<EvidenceBinding>, MotifEvidenceArchive) {
        let arena = observations
            .iter()
            .map(|observation| arena_binding(observation.0.clone()))
            .collect::<Vec<_>>();
        let mut accumulator = MotifEvidenceAccumulator::new();
        for (index, (_, evidence_class, safety_veto)) in observations.into_iter().enumerate() {
            accumulator
                .push(index, arena[index].joined(), evidence_class, safety_veto)
                .expect("motif observation");
        }
        let archive = accumulator.finish(&arena).expect("archive");
        (arena, archive)
    }

    fn build_natural_archive(
        rows: Vec<BlindThenRevealJoinedTransitionV1>,
    ) -> (Vec<EvidenceBinding>, MotifEvidenceArchive) {
        build_archive(
            rows.into_iter()
                .map(|row| (row, K1NaturalEvidenceClassV1::NaturalLive, false))
                .collect(),
        )
    }

    fn freeze_for_support(
        archive: &MotifEvidenceArchive,
        support_watermark: u64,
    ) -> K1NaturalCandidateFreezeV1 {
        let support = archive
            .candidate_supports
            .first()
            .expect("candidate support");
        K1NaturalCandidateFreezeV1 {
            schema: K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6.to_owned(),
            freeze_root_sha256: root(90_000),
            generation_sequence: 1,
            catalog_root_sha256: root(90_001),
            k1_deficit_snapshot_root_sha256: root(90_002),
            epistemic_registry_revision: 1,
            epistemic_registry_root_sha256: root(90_003),
            fixture_exclusion_root_sha256: root(90_004),
            candidate_root_sha256: root(90_005),
            capture_generation_root_sha256: support.capture_generation_root_sha256.clone(),
            candidate_structural_root_sha256: support.motif_root_sha256.clone(),
            source_neutral_topology_root_sha256: support.motif_root_sha256.clone(),
            semantic_novelty_signature_root_sha256: support
                .semantic_novelty_signature_root_sha256
                .clone(),
            consequence_type: support.consequence_type,
            evidence_manifest_root_sha256: support.retained_manifest_root_sha256.clone(),
            generator_schema: MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V4.to_owned(),
            discovery_basis_root_sha256: natural_t1_discovery_basis_root_v4()
                .expect("discovery basis"),
            readiness_receipt_root_sha256: root(90_006),
            scoring_tuple: K1CandidateScoreV1 {
                total_k1_gain: 1,
                law_gain: 1,
                semantic_gain: 0,
                topology_gain: 0,
                readiness_rank: 0,
                bounded_discovery_cost_units: 1,
                expected_verified_input_tokens: 1,
                stable_hash_sha256: root(90_007),
            },
            scheduler_schema: K1_SCHEDULER_SCHEMA_V2.to_owned(),
            budget: K1GenerationBudgetV1 {
                maximum_support_rows: K1_MAX_SUPPORT_ROWS_V1 as u64,
                maximum_probe_rounds: 1,
                maximum_probe_cost_units: 1,
                maximum_generation_seconds: 1,
            },
            support_watermark,
            contract_watermark: support_watermark,
            future_min_sequence: support_watermark + 1,
            selected_at_unix: 1,
            authority_ready: false,
            phase_mutation_allowed: false,
            motif_disposition_summary_root_sha256: archive.disposition.summary_root_sha256.clone(),
            motif_enumeration_config_root_sha256: archive
                .disposition
                .enumeration_config_root_sha256
                .clone(),
            complete_topology_manifest_root_sha256: root(90_008),
            motif_embedding_manifest_root_sha256: root(90_009),
            motif_support_overflow_occurrences: support.overflow_occurrences,
            motif_support_overflow_manifest_root_sha256: support
                .overflow_manifest_root_sha256
                .clone(),
        }
    }

    #[test]
    fn one_ambient_payload_is_shared_by_all_retained_motif_occurrences() {
        let (arena, archive) = build_natural_archive(vec![chain_joined(1, 3)]);

        assert_eq!(archive.ambient_payloads(), 1);
        assert!(archive.occurrences.len() > 1);
        assert!(
            archive
                .occurrences
                .iter()
                .all(|binding| binding.ambient_index == 0)
        );
        assert!(archive.occurrences.iter().all(|binding| {
            archive
                .joined_for(&arena, binding)
                .expect("joined payload")
                .join_root_sha256
                == binding.row.evidence_root_sha256
        }));
    }

    #[test]
    fn support_reservoir_is_bounded_and_preserves_independent_lineage_slots() {
        let (_, archive) = build_natural_archive(
            (1..=72)
                .map(|sequence| {
                    let lineage = if sequence <= 70 { 1 } else { 2 };
                    joined(sequence, lineage)
                })
                .collect(),
        );
        let rows = archive.evidence_rows();
        let catalog = build_k1_motif_cohort_catalog_v1(
            &rows,
            &archive.candidate_supports,
            root(80_000),
            fixture_exclusion_root().expect("fixture root"),
            "nando.operator-blind-version-space-generator.v4".to_owned(),
            archive.disposition.clone(),
        )
        .expect("catalog");

        assert_eq!(archive.occurrences.len(), K1_MAX_SUPPORT_ROWS_V1);
        assert_eq!(archive.ambient_payloads(), K1_MAX_SUPPORT_ROWS_V1);
        assert_eq!(archive.candidate_supports.len(), 1);
        assert_eq!(archive.candidate_supports[0].overflow_occurrences, 8);
        assert_eq!(catalog.candidates[0].independent_lineages, 2);
        assert_eq!(catalog.motif_retained_occurrences, 64);
        assert_eq!(
            catalog
                .motif_disposition
                .as_ref()
                .expect("disposition")
                .support_overflow_occurrences,
            8
        );
    }

    #[test]
    fn overflow_changes_support_receipt_without_mutating_retained_evidence() {
        let build = |maximum: u64| {
            build_natural_archive(
                (1..=maximum)
                    .map(|sequence| {
                        let lineage = if sequence <= 62 { 1 } else { 2 };
                        joined(sequence, lineage)
                    })
                    .collect(),
            )
            .1
        };
        let baseline = build(64);
        let overflowed = build(65);

        assert_eq!(baseline.evidence_rows(), overflowed.evidence_rows());
        assert_eq!(
            baseline.candidate_supports[0].retained_manifest_root_sha256,
            overflowed.candidate_supports[0].retained_manifest_root_sha256
        );
        assert_eq!(baseline.candidate_supports[0].overflow_occurrences, 0);
        assert_eq!(overflowed.candidate_supports[0].overflow_occurrences, 1);
        assert_ne!(
            baseline.candidate_supports[0].support_root_sha256,
            overflowed.candidate_supports[0].support_root_sha256
        );
    }

    #[test]
    fn post_freeze_overflow_is_future_only_and_never_backfills_support() {
        let (arena, archive) = build_natural_archive(
            (1..=65)
                .map(|sequence| {
                    let lineage = if sequence <= 63 { 1 } else { 2 };
                    joined(sequence, lineage)
                })
                .collect(),
        );
        let freeze = freeze_for_support(&archive, 64);
        let empty = BTreeSet::new();
        let (support, support_motifs, support_rows) =
            super::super::identification::frozen_motif_identification_evidence(
                &arena,
                Some(&archive),
                &freeze,
                &empty,
                &empty,
            )
            .expect("frozen support replay");

        assert_eq!(archive.occurrences.len(), K1_MAX_SUPPORT_ROWS_V1);
        assert_eq!(archive.candidate_supports[0].overflow_occurrences, 1);
        assert_eq!(support.len(), K1_MAX_SUPPORT_ROWS_V1);
        assert_eq!(support_motifs.len(), support.len());
        assert_eq!(support_rows.len(), support.len());
        assert!(support.iter().all(|row| row.capture_sequence <= 64));

        let future_root = arena[64].join_root_sha256().to_owned();
        let trials = BTreeSet::from([future_root]);
        let (with_future, motifs, row_roots) =
            super::super::identification::frozen_motif_identification_evidence(
                &arena,
                Some(&archive),
                &freeze,
                &empty,
                &trials,
            )
            .expect("future replay");
        assert_eq!(with_future.len(), K1_MAX_SUPPORT_ROWS_V1 + 1);
        assert_eq!(motifs.len(), with_future.len());
        assert_eq!(row_roots.len(), with_future.len());
        assert_eq!(with_future.last().map(|row| row.capture_sequence), Some(65));
    }

    #[test]
    fn source_and_occurrence_denominators_and_censors_are_disjoint() {
        let mut empty = joined(4, 4);
        empty.topology.roles.clear();
        empty.topology.role_witnesses.clear();

        let mut invalid = joined(5, 5);
        invalid
            .topology
            .roles
            .push(invalid.topology.roles[0].clone());
        invalid
            .topology
            .role_witnesses
            .push(invalid.topology.role_witnesses[0].clone());

        let mut budget = chain_joined(6, 11);
        budget.topology.relations = (0..11)
            .flat_map(|source_role_id| {
                (source_role_id + 1..11).map(move |target_role_id| MultiSourceRelationEdgeV1 {
                    relation: MultiSourceRelationKindV1::Precedes,
                    source_role_id,
                    target_role_id,
                })
            })
            .collect();
        let (_, archive) = build_archive(vec![
            (joined(1, 1), K1NaturalEvidenceClassV1::NaturalLive, false),
            (joined(2, 2), K1NaturalEvidenceClassV1::Controlled, false),
            (joined(3, 3), K1NaturalEvidenceClassV1::NaturalLive, true),
            (empty, K1NaturalEvidenceClassV1::NaturalLive, false),
            (invalid, K1NaturalEvidenceClassV1::NaturalLive, false),
            (budget, K1NaturalEvidenceClassV1::NaturalLive, false),
        ]);
        assert_eq!(archive.disposition.scanned_source_rows, 6);
        assert_eq!(archive.disposition.motif_source_rows, 1);
        assert_eq!(archive.disposition.retained_motif_occurrences, 1);
        assert_eq!(archive.disposition.budget_censored_rows, 1);
        assert_eq!(archive.disposition.empty_or_incomplete_rows, 1);
        assert_eq!(archive.disposition.invalid_embedding_rows, 1);
        assert_eq!(archive.disposition.fixture_or_controlled_excluded_rows, 1);
        assert_eq!(archive.disposition.safety_veto_rows, 1);
        assert_eq!(archive.source_dispositions.len(), 6);
    }

    #[test]
    fn resumed_delta_matches_one_pass_archive_exactly() {
        let arena = (1..=72)
            .map(|sequence| {
                let lineage = if sequence <= 70 { 1 } else { 2 };
                arena_binding(joined(sequence, lineage))
            })
            .collect::<Vec<_>>();
        let mut one_pass = MotifEvidenceAccumulator::new();
        for (index, binding) in arena.iter().enumerate() {
            one_pass
                .push_natural(index, binding.joined())
                .expect("one-pass observation");
        }
        let one_pass = one_pass.finish(&arena).expect("one-pass archive");

        let mut first = MotifEvidenceAccumulator::new();
        for (index, binding) in arena.iter().take(32).enumerate() {
            first
                .push_natural(index, binding.joined())
                .expect("first observation");
        }
        let first = first.finish(&arena).expect("first archive");
        let mut resumed = MotifEvidenceAccumulator::resume(first, &arena).expect("resume");
        for (index, binding) in arena.iter().enumerate().skip(32) {
            resumed
                .push_natural(index, binding.joined())
                .expect("delta observation");
        }
        let resumed = resumed.finish(&arena).expect("resumed archive");

        assert_eq!(resumed, one_pass);
    }

    #[test]
    fn duplicate_and_out_of_order_source_rows_fail_closed() {
        let duplicate_arena = [arena_binding(joined(1, 1)), arena_binding(joined(1, 1))];
        let mut duplicate = MotifEvidenceAccumulator::new();
        duplicate
            .push_natural(0, duplicate_arena[0].joined())
            .expect("first source");
        assert_eq!(
            duplicate
                .push_natural(1, duplicate_arena[1].joined())
                .expect_err("duplicate must fail"),
            "k1_motif_evidence_out_of_order"
        );

        let out_of_order_arena = [arena_binding(joined(2, 2)), arena_binding(joined(1, 1))];
        let mut out_of_order = MotifEvidenceAccumulator::new();
        out_of_order
            .push_natural(0, out_of_order_arena[0].joined())
            .expect("newer source");
        assert_eq!(
            out_of_order
                .push_natural(1, out_of_order_arena[1].joined())
                .expect_err("older source must fail"),
            "k1_motif_evidence_out_of_order"
        );
    }
}
