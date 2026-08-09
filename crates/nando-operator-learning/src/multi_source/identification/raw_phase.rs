use std::collections::{BTreeMap, BTreeSet};

use nando_core::wave::{phase_coherence, phase_vector_from_atom_ids};
use nando_operator_kernel::{
    RelationFrame, ResponseProgram, canonical_json_sha256, relation_frame_phase_atom_ids,
    response_program_required_routing_atom_ids, response_program_version_root_sha256,
    valid_nonzero_sha256,
};
use serde::{Deserialize, Serialize};

use super::MULTI_SOURCE_T1_MAX_SUPPORT_BASIS_ROWS;

pub const RAW_PHASE_T1_HYPOTHESIS_ENVELOPE_SCHEMA_V1: &str =
    "nando.raw-phase-t1-hypothesis-envelope.v1";
pub const RAW_PHASE_T1_HYPOTHESIS_GENERATOR_V1: &str =
    "nando.raw-phase-t1.phase-scored-existing-bounded-programs.v1";
const RAW_PHASE_T1_CELLS_V1: usize = 16;
const RAW_PHASE_T1_SCORE_SCALE: f64 = 1_000_000.0;
const RAW_PHASE_T1_MAX_PROGRAMS: usize = 4_096;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawPhaseT1HypothesisScoreV1 {
    pub program_root_sha256: String,
    pub required_phase_atoms_root_sha256: String,
    pub coherence_micro: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawPhaseT1HypothesisEnvelopeV1 {
    pub schema: String,
    pub envelope_root_sha256: String,
    pub frozen_domain_root_sha256: String,
    pub support_watermark: u64,
    pub support_frame_roots_sha256: Vec<String>,
    pub support_lineage_roots_sha256: Vec<String>,
    pub hypothesis_source_artifact_roots_sha256: Vec<String>,
    pub support_phase_root_sha256: String,
    pub candidate_set_root_sha256: String,
    pub candidate_program_roots_sha256: Vec<String>,
    pub scores: Vec<RawPhaseT1HypothesisScoreV1>,
    pub generator_schema: String,
    pub bounded_candidate_set_complete: bool,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Serialize)]
struct RawPhaseT1HypothesisEnvelopeDigestV1<'a> {
    schema: &'static str,
    frozen_domain_root_sha256: &'a str,
    support_watermark: u64,
    support_frame_roots_sha256: &'a [String],
    support_lineage_roots_sha256: &'a [String],
    hypothesis_source_artifact_roots_sha256: &'a [String],
    support_phase_root_sha256: &'a str,
    candidate_set_root_sha256: &'a str,
    candidate_program_roots_sha256: &'a [String],
    scores: &'a [RawPhaseT1HypothesisScoreV1],
    generator_schema: &'static str,
    bounded_candidate_set_complete: bool,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

pub fn seal_raw_phase_t1_hypothesis_envelope_v1(
    frozen_domain_root_sha256: String,
    support_watermark: u64,
    support_frames: &[RelationFrame],
    support_lineage_roots_sha256: Vec<String>,
    hypothesis_source_artifact_roots_sha256: Vec<String>,
    programs: &BTreeMap<String, ResponseProgram>,
) -> Result<RawPhaseT1HypothesisEnvelopeV1, &'static str> {
    if !valid_nonzero_sha256(&frozen_domain_root_sha256)
        || support_watermark == 0
        || support_frames.is_empty()
        || support_frames.len() > MULTI_SOURCE_T1_MAX_SUPPORT_BASIS_ROWS
        || programs.is_empty()
        || programs.len() > RAW_PHASE_T1_MAX_PROGRAMS
    {
        return Err("raw_phase_t1_hypothesis_input_invalid");
    }

    let support_frame_roots_sha256 = sorted_unique_roots(
        support_frames
            .iter()
            .map(|frame| frame.frame_id_sha256.clone()),
        false,
    )?;
    if support_frame_roots_sha256.len() != support_frames.len() {
        return Err("raw_phase_t1_support_frame_reused");
    }
    let support_lineage_roots_sha256 = sorted_unique_roots(support_lineage_roots_sha256, false)?;
    let hypothesis_source_artifact_roots_sha256 =
        sorted_unique_roots(hypothesis_source_artifact_roots_sha256, true)?;

    let mut support_phase_rows = support_frames
        .iter()
        .map(|frame| {
            (
                frame.frame_id_sha256.clone(),
                relation_frame_phase_atom_ids(frame),
            )
        })
        .collect::<Vec<_>>();
    support_phase_rows.sort_by(|left, right| left.0.cmp(&right.0));
    let support_phase_root_sha256 = canonical_json_sha256(&(
        "nando.raw-phase-t1-support-phase.v1",
        RAW_PHASE_T1_CELLS_V1,
        &support_phase_rows,
    ))
    .map_err(|_| "raw_phase_t1_support_phase_root_failed")?;
    let support_vectors = support_phase_rows
        .iter()
        .map(|(_, atoms)| phase_vector_from_atom_ids(atoms.iter().copied(), RAW_PHASE_T1_CELLS_V1))
        .collect::<Vec<_>>();

    let mut scores = Vec::with_capacity(programs.len());
    for (program_root, program) in programs {
        if !valid_nonzero_sha256(program_root)
            || program.validate().is_err()
            || response_program_version_root_sha256(program).as_deref() != Ok(program_root.as_str())
        {
            return Err("raw_phase_t1_candidate_program_invalid");
        }
        let required_atoms = response_program_required_routing_atom_ids(program);
        let required_phase_atoms_root_sha256 =
            canonical_json_sha256(&("nando.raw-phase-t1-required-atoms.v1", &required_atoms))
                .map_err(|_| "raw_phase_t1_required_atoms_root_failed")?;
        let center = phase_vector_from_atom_ids(required_atoms, RAW_PHASE_T1_CELLS_V1);
        let coherence = support_vectors
            .iter()
            .map(|query| phase_coherence(query, &center))
            .sum::<f64>()
            / support_vectors.len() as f64;
        scores.push(RawPhaseT1HypothesisScoreV1 {
            program_root_sha256: program_root.clone(),
            required_phase_atoms_root_sha256,
            coherence_micro: finite_score_micro(coherence),
        });
    }
    scores.sort_by(|left, right| left.program_root_sha256.cmp(&right.program_root_sha256));
    let candidate_program_roots_sha256 = programs.keys().cloned().collect::<Vec<_>>();
    let candidate_set_root_sha256 = canonical_json_sha256(&(
        "nando.raw-phase-t1-candidate-set.v1",
        RAW_PHASE_T1_HYPOTHESIS_GENERATOR_V1,
        &candidate_program_roots_sha256,
        &scores,
    ))
    .map_err(|_| "raw_phase_t1_candidate_set_root_failed")?;

    let mut envelope = RawPhaseT1HypothesisEnvelopeV1 {
        schema: RAW_PHASE_T1_HYPOTHESIS_ENVELOPE_SCHEMA_V1.to_owned(),
        envelope_root_sha256: String::new(),
        frozen_domain_root_sha256,
        support_watermark,
        support_frame_roots_sha256,
        support_lineage_roots_sha256,
        hypothesis_source_artifact_roots_sha256,
        support_phase_root_sha256,
        candidate_set_root_sha256,
        candidate_program_roots_sha256,
        scores,
        generator_schema: RAW_PHASE_T1_HYPOTHESIS_GENERATOR_V1.to_owned(),
        bounded_candidate_set_complete: true,
        authority_ready: false,
        phase_mutation_allowed: false,
    };
    envelope.envelope_root_sha256 = envelope.expected_root()?;
    envelope.validate()?;
    Ok(envelope)
}

impl RawPhaseT1HypothesisEnvelopeV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != RAW_PHASE_T1_HYPOTHESIS_ENVELOPE_SCHEMA_V1
            || self.generator_schema != RAW_PHASE_T1_HYPOTHESIS_GENERATOR_V1
            || !valid_nonzero_sha256(&self.envelope_root_sha256)
            || !valid_nonzero_sha256(&self.frozen_domain_root_sha256)
            || self.support_watermark == 0
            || self.support_frame_roots_sha256.is_empty()
            || self.support_frame_roots_sha256.len() > MULTI_SOURCE_T1_MAX_SUPPORT_BASIS_ROWS
            || self.support_lineage_roots_sha256.is_empty()
            || self.candidate_program_roots_sha256.is_empty()
            || self.candidate_program_roots_sha256.len() > RAW_PHASE_T1_MAX_PROGRAMS
            || self.scores.len() != self.candidate_program_roots_sha256.len()
            || !strict_roots(&self.support_frame_roots_sha256)
            || !strict_roots(&self.support_lineage_roots_sha256)
            || !self.hypothesis_source_artifact_roots_sha256.is_empty()
                && !strict_roots(&self.hypothesis_source_artifact_roots_sha256)
            || !strict_roots(&self.candidate_program_roots_sha256)
            || !valid_nonzero_sha256(&self.support_phase_root_sha256)
            || !valid_nonzero_sha256(&self.candidate_set_root_sha256)
            || self
                .scores
                .iter()
                .zip(&self.candidate_program_roots_sha256)
                .any(|(score, root)| {
                    score.program_root_sha256 != *root
                        || !valid_nonzero_sha256(&score.required_phase_atoms_root_sha256)
                        || !(-1_000_000..=1_000_000).contains(&score.coherence_micro)
                })
            || !self.bounded_candidate_set_complete
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.expected_root()? != self.envelope_root_sha256
        {
            return Err("raw_phase_t1_hypothesis_envelope_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&RawPhaseT1HypothesisEnvelopeDigestV1 {
            schema: RAW_PHASE_T1_HYPOTHESIS_ENVELOPE_SCHEMA_V1,
            frozen_domain_root_sha256: &self.frozen_domain_root_sha256,
            support_watermark: self.support_watermark,
            support_frame_roots_sha256: &self.support_frame_roots_sha256,
            support_lineage_roots_sha256: &self.support_lineage_roots_sha256,
            hypothesis_source_artifact_roots_sha256: &self.hypothesis_source_artifact_roots_sha256,
            support_phase_root_sha256: &self.support_phase_root_sha256,
            candidate_set_root_sha256: &self.candidate_set_root_sha256,
            candidate_program_roots_sha256: &self.candidate_program_roots_sha256,
            scores: &self.scores,
            generator_schema: RAW_PHASE_T1_HYPOTHESIS_GENERATOR_V1,
            bounded_candidate_set_complete: self.bounded_candidate_set_complete,
            authority_ready: false,
            phase_mutation_allowed: false,
        })
        .map_err(|_| "raw_phase_t1_hypothesis_envelope_root_failed")
    }
}

fn sorted_unique_roots(
    roots: impl IntoIterator<Item = String>,
    allow_empty: bool,
) -> Result<Vec<String>, &'static str> {
    let roots = roots.into_iter().collect::<BTreeSet<_>>();
    if (!allow_empty && roots.is_empty()) || roots.iter().any(|root| !valid_nonzero_sha256(root)) {
        return Err("raw_phase_t1_provenance_root_invalid");
    }
    Ok(roots.into_iter().collect())
}

fn strict_roots(roots: &[String]) -> bool {
    roots.iter().all(|root| valid_nonzero_sha256(root))
        && roots.windows(2).all(|pair| pair[0] < pair[1])
}

fn finite_score_micro(value: f64) -> i64 {
    if !value.is_finite() {
        return i64::MIN;
    }
    (value.clamp(-1.0, 1.0) * RAW_PHASE_T1_SCORE_SCALE).round() as i64
}

#[cfg(test)]
mod tests {
    use nando_operator_kernel::{
        AtomSource, AtomValueType, RELATION_FRAME_SCHEMA, RelationAtom, ResponseValueSelector,
        ValueProjectionFormat,
    };

    use super::*;

    fn root(value: u64) -> String {
        format!("{value:064x}")
    }

    fn frame(id: u64) -> RelationFrame {
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: root(id),
            event_id_sha256: root(id + 100),
            client_intent_id_sha256: root(id + 200),
            session_id_sha256: root(id + 300),
            observed_at_unix_nanos: id,
            estimated_input_tokens: 1,
            extractor_version: "raw-phase-test".to_owned(),
            verifier_label: Some(true),
            atoms: vec![
                RelationAtom::TypedSlot {
                    slot_id: 1,
                    value_type: AtomValueType::Integer,
                    source: AtomSource::Observation,
                    value_sha256: root(id + 400),
                },
                RelationAtom::UniqueSlot { slot_id: 1 },
                RelationAtom::ObservationSelector {
                    slot_id: 1,
                    selector: ResponseValueSelector::UniqueScalar {
                        value_type: AtomValueType::Integer,
                    },
                },
                RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
            ],
            evidence_ref_sha256: root(id + 500),
        }
    }

    #[test]
    fn envelope_is_hypothesis_only_and_order_independent() {
        let program = ResponseProgram::project_selected_value(
            ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::Integer,
            },
            ValueProjectionFormat::PlainText,
            "completed",
        );
        let program_root = response_program_version_root_sha256(&program).expect("program root");
        let programs = BTreeMap::from([(program_root, program)]);
        let left = seal_raw_phase_t1_hypothesis_envelope_v1(
            root(900),
            2,
            &[frame(1), frame(2)],
            vec![root(701), root(702)],
            Vec::new(),
            &programs,
        )
        .expect("raw phase envelope");
        let right = seal_raw_phase_t1_hypothesis_envelope_v1(
            root(900),
            2,
            &[frame(2), frame(1)],
            vec![root(702), root(701)],
            Vec::new(),
            &programs,
        )
        .expect("reordered raw phase envelope");

        assert_eq!(left, right);
        assert!(!left.authority_ready);
        assert!(!left.phase_mutation_allowed);
        assert!(left.bounded_candidate_set_complete);
        assert_eq!(left.scores.len(), 1);

        let mut authority_tampered = left.clone();
        authority_tampered.authority_ready = true;
        assert_eq!(
            authority_tampered.validate(),
            Err("raw_phase_t1_hypothesis_envelope_invalid")
        );
        let mut phase_tampered = left;
        phase_tampered.phase_mutation_allowed = true;
        assert_eq!(
            phase_tampered.validate(),
            Err("raw_phase_t1_hypothesis_envelope_invalid")
        );
    }
}
