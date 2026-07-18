use std::collections::{BTreeMap, BTreeSet};

use super::{
    OperatorCircuit, OperatorRelationCell, TernaryRelationState, VerifiedPartialRelationWave,
    VerifiedWaveOutcome,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OperatorGrokkingConfig {
    pub max_circuits: usize,
    pub max_waves: usize,
    pub min_independent_surfaces: usize,
    pub min_independent_sessions: usize,
    pub min_relation_planes: usize,
    pub coherence_floor: f64,
    pub coherence_margin: f64,
}

impl Default for OperatorGrokkingConfig {
    fn default() -> Self {
        Self {
            max_circuits: 64,
            max_waves: 256,
            min_independent_surfaces: 3,
            min_independent_sessions: 3,
            min_relation_planes: 2,
            coherence_floor: 0.90,
            coherence_margin: 0.10,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateCubeFieldError {
    InvalidConfig,
    CircuitCapacityReached,
    WaveCapacityReached,
    WrongGeneration,
    DuplicateCircuit,
    DuplicateReceipt,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateCubeField {
    generation: u64,
    config: OperatorGrokkingConfig,
    circuits: Vec<OperatorCircuit>,
    waves: Vec<VerifiedPartialRelationWave>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperatorCircuitStage {
    Memorizing,
    CompetingCircuits,
    CoherentCandidate,
    Inconsistent,
    Censored,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperatorCircuitScore {
    pub circuit_fingerprint64: u64,
    pub coherence: f64,
    pub evidence_coverage: f64,
    pub independent_surfaces: usize,
    pub independent_sessions: usize,
    pub relation_planes: usize,
    pub one_surface_contains_complete_circuit: bool,
    pub hard_contradictions: usize,
    pub eligible: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoherentOperatorCandidate {
    pub source_generation: u64,
    pub candidate_generation: u64,
    pub circuit: OperatorCircuit,
    pub coherence: f64,
    pub margin_over_runner_up: f64,
    pub independent_surfaces: usize,
    pub independent_sessions: usize,
    pub receipt_ids: Box<[u64]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperatorConsolidationReport {
    pub stage: OperatorCircuitStage,
    pub semantic_waves: usize,
    pub censored_waves: usize,
    pub scores: Box<[OperatorCircuitScore]>,
    pub candidate: Option<CoherentOperatorCandidate>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct OperatorGrokkingConsolidator;

impl CandidateCubeField {
    pub fn new(
        generation: u64,
        config: OperatorGrokkingConfig,
    ) -> Result<Self, CandidateCubeFieldError> {
        if config.max_circuits == 0
            || config.max_waves == 0
            || config.min_independent_surfaces < 2
            || config.min_independent_sessions == 0
            || config.min_relation_planes == 0
            || !(0.0..=1.0).contains(&config.coherence_floor)
            || !(0.0..=1.0).contains(&config.coherence_margin)
        {
            return Err(CandidateCubeFieldError::InvalidConfig);
        }
        Ok(Self {
            generation,
            config,
            circuits: Vec::with_capacity(config.max_circuits),
            waves: Vec::with_capacity(config.max_waves),
        })
    }

    pub fn register_circuit(
        &mut self,
        circuit: OperatorCircuit,
    ) -> Result<(), CandidateCubeFieldError> {
        if self
            .circuits
            .iter()
            .any(|current| current.fingerprint64() == circuit.fingerprint64())
        {
            return Err(CandidateCubeFieldError::DuplicateCircuit);
        }
        if self.circuits.len() == self.config.max_circuits {
            return Err(CandidateCubeFieldError::CircuitCapacityReached);
        }
        self.circuits.push(circuit);
        self.circuits.sort_by_key(OperatorCircuit::fingerprint64);
        Ok(())
    }

    pub fn observe(
        &mut self,
        wave: VerifiedPartialRelationWave,
    ) -> Result<(), CandidateCubeFieldError> {
        if wave.generation != self.generation {
            return Err(CandidateCubeFieldError::WrongGeneration);
        }
        if self
            .waves
            .iter()
            .any(|current| current.receipt_id == wave.receipt_id)
        {
            return Err(CandidateCubeFieldError::DuplicateReceipt);
        }
        if self.waves.len() == self.config.max_waves {
            return Err(CandidateCubeFieldError::WaveCapacityReached);
        }
        self.waves.push(wave);
        Ok(())
    }

    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    #[must_use]
    pub fn circuits(&self) -> &[OperatorCircuit] {
        &self.circuits
    }

    #[must_use]
    pub fn waves(&self) -> &[VerifiedPartialRelationWave] {
        &self.waves
    }
}

impl OperatorGrokkingConsolidator {
    #[must_use]
    pub fn consolidate(field: &CandidateCubeField) -> OperatorConsolidationReport {
        let semantic_waves = field
            .waves
            .iter()
            .filter(|wave| wave.outcome != VerifiedWaveOutcome::CensoredUnknown)
            .count();
        let censored_waves = field.waves.len().saturating_sub(semantic_waves);
        if semantic_waves == 0 {
            return OperatorConsolidationReport {
                stage: OperatorCircuitStage::Censored,
                semantic_waves,
                censored_waves,
                scores: Vec::new().into_boxed_slice(),
                candidate: None,
            };
        }

        let mut scored = field
            .circuits
            .iter()
            .map(|circuit| score_circuit(circuit, &field.waves, field.config))
            .collect::<Vec<_>>();
        scored.sort_by(|left, right| {
            right
                .coherence
                .total_cmp(&left.coherence)
                .then_with(|| right.evidence_coverage.total_cmp(&left.evidence_coverage))
                .then_with(|| left.circuit_fingerprint64.cmp(&right.circuit_fingerprint64))
        });

        let best = scored.first();
        let global_surfaces = field
            .waves
            .iter()
            .filter(|wave| wave.outcome != VerifiedWaveOutcome::CensoredUnknown)
            .map(|wave| wave.surface_id)
            .collect::<BTreeSet<_>>()
            .len();
        let global_sessions = field
            .waves
            .iter()
            .filter(|wave| wave.outcome != VerifiedWaveOutcome::CensoredUnknown)
            .map(|wave| wave.session_id)
            .collect::<BTreeSet<_>>()
            .len();

        let evidence_ready = global_surfaces >= field.config.min_independent_surfaces
            && global_sessions >= field.config.min_independent_sessions;
        let eligible = scored
            .iter()
            .filter(|score| score.eligible)
            .collect::<Vec<_>>();

        let candidate = eligible.first().and_then(|winner| {
            let runner_up = scored
                .iter()
                .find(|score| score.circuit_fingerprint64 != winner.circuit_fingerprint64)
                .map_or(0.0, |score| score.coherence);
            let margin = winner.coherence - runner_up;
            if evidence_ready
                && winner.coherence >= field.config.coherence_floor
                && margin >= field.config.coherence_margin
            {
                let circuit = field
                    .circuits
                    .iter()
                    .find(|circuit| circuit.fingerprint64() == winner.circuit_fingerprint64)?
                    .clone();
                let receipt_ids = supporting_receipts(&circuit, &field.waves);
                Some(CoherentOperatorCandidate {
                    source_generation: field.generation,
                    candidate_generation: field.generation.saturating_add(1),
                    circuit,
                    coherence: winner.coherence,
                    margin_over_runner_up: margin,
                    independent_surfaces: winner.independent_surfaces,
                    independent_sessions: winner.independent_sessions,
                    receipt_ids: receipt_ids.into_boxed_slice(),
                })
            } else {
                None
            }
        });

        let stage = if candidate.is_some() {
            OperatorCircuitStage::CoherentCandidate
        } else if !evidence_ready {
            OperatorCircuitStage::Memorizing
        } else if best.is_none() || scored.iter().all(|score| score.hard_contradictions > 0) {
            OperatorCircuitStage::Inconsistent
        } else {
            OperatorCircuitStage::CompetingCircuits
        };

        OperatorConsolidationReport {
            stage,
            semantic_waves,
            censored_waves,
            scores: scored.into_boxed_slice(),
            candidate,
        }
    }
}

fn score_circuit(
    circuit: &OperatorCircuit,
    waves: &[VerifiedPartialRelationWave],
    config: OperatorGrokkingConfig,
) -> OperatorCircuitScore {
    let mut covered = BTreeSet::<OperatorRelationCell>::new();
    let mut surfaces = BTreeSet::new();
    let mut sessions = BTreeSet::new();
    let mut planes = BTreeSet::new();
    let mut surface_coverage = BTreeMap::<u64, BTreeSet<OperatorRelationCell>>::new();
    let mut resultant_re = 0.0;
    let mut resultant_im = 0.0;
    let mut resultant_weight = 0.0;
    let mut hard_contradictions = 0_usize;

    for wave in waves {
        if wave.outcome == VerifiedWaveOutcome::CensoredUnknown {
            continue;
        }
        let mut wave_matches = 0_usize;
        let mut wave_conflicts = 0_usize;
        for sample in &wave.samples {
            let Some(relation) = circuit.relation(sample.cell) else {
                continue;
            };
            if sample.state == TernaryRelationState::Unresolved {
                continue;
            }
            if sample.state != relation.state {
                wave_conflicts = wave_conflicts.saturating_add(1);
                continue;
            }

            wave_matches = wave_matches.saturating_add(1);
            covered.insert(sample.cell);
            planes.insert(sample.cell.plane);
            surface_coverage
                .entry(wave.surface_id)
                .or_default()
                .insert(sample.cell);

            // Rotate each observed phase by the conjugate circuit anchor. A
            // coherent whole circuit makes these residuals point together.
            let aligned_re = sample.phase.re * relation.phase_anchor.re
                + sample.phase.im * relation.phase_anchor.im;
            let aligned_im = sample.phase.im * relation.phase_anchor.re
                - sample.phase.re * relation.phase_anchor.im;
            let weight = aligned_re.hypot(aligned_im);
            if weight > f64::EPSILON {
                resultant_re += aligned_re;
                resultant_im += aligned_im;
                resultant_weight += weight;
            }
        }

        if wave_matches > 0 {
            surfaces.insert(wave.surface_id);
            sessions.insert(wave.session_id);
        }
        if wave.outcome == VerifiedWaveOutcome::HardContradiction
            && wave_matches > 0
            && wave_conflicts == 0
        {
            hard_contradictions = hard_contradictions.saturating_add(1);
        }
    }

    let coherence = if resultant_weight > f64::EPSILON {
        resultant_re.hypot(resultant_im) / resultant_weight
    } else {
        0.0
    };
    let evidence_coverage = covered.len() as f64 / circuit.relations().len() as f64;
    let one_surface_contains_complete_circuit = surface_coverage
        .values()
        .any(|cells| cells.len() == circuit.relations().len());
    let eligible = evidence_coverage == 1.0
        && surfaces.len() >= config.min_independent_surfaces
        && sessions.len() >= config.min_independent_sessions
        && planes.len() >= config.min_relation_planes
        && !one_surface_contains_complete_circuit
        && hard_contradictions == 0;

    OperatorCircuitScore {
        circuit_fingerprint64: circuit.fingerprint64(),
        coherence,
        evidence_coverage,
        independent_surfaces: surfaces.len(),
        independent_sessions: sessions.len(),
        relation_planes: planes.len(),
        one_surface_contains_complete_circuit,
        hard_contradictions,
        eligible,
    }
}

fn supporting_receipts(
    circuit: &OperatorCircuit,
    waves: &[VerifiedPartialRelationWave],
) -> Vec<u64> {
    let mut receipts = waves
        .iter()
        .filter(|wave| wave.outcome != VerifiedWaveOutcome::CensoredUnknown)
        .filter(|wave| {
            wave.samples.iter().any(|sample| {
                circuit
                    .relation(sample.cell)
                    .is_some_and(|relation| relation.state == sample.state)
            })
        })
        .map(|wave| wave.receipt_id)
        .collect::<Vec<_>>();
    receipts.sort_unstable();
    receipts.dedup();
    receipts
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, PI};

    use super::*;
    use crate::wave::{OperatorCircuitRelation, PhaseCenterCell, VerifiedRelationSample};

    fn phase(angle: f64) -> PhaseCenterCell {
        PhaseCenterCell {
            re: angle.cos(),
            im: angle.sin(),
        }
    }

    fn cell(plane: u8, source_role: u8, target_role: u8) -> OperatorRelationCell {
        OperatorRelationCell {
            plane,
            source_role,
            target_role,
        }
    }

    fn circuit(anchors: [f64; 3]) -> OperatorCircuit {
        OperatorCircuit::new(
            3,
            vec![
                OperatorCircuitRelation {
                    cell: cell(0, 0, 1),
                    state: TernaryRelationState::Supported,
                    phase_anchor: phase(anchors[0]),
                },
                OperatorCircuitRelation {
                    cell: cell(0, 1, 2),
                    state: TernaryRelationState::Supported,
                    phase_anchor: phase(anchors[1]),
                },
                OperatorCircuitRelation {
                    cell: cell(1, 0, 2),
                    state: TernaryRelationState::Supported,
                    phase_anchor: phase(anchors[2]),
                },
            ],
        )
        .expect("test circuit is connected")
    }

    fn wave(
        receipt_id: u64,
        surface_id: u64,
        sample_cell: OperatorRelationCell,
        sample_phase: f64,
    ) -> VerifiedPartialRelationWave {
        VerifiedPartialRelationWave::new(
            receipt_id,
            surface_id,
            surface_id,
            7,
            VerifiedWaveOutcome::Positive,
            vec![VerifiedRelationSample {
                cell: sample_cell,
                state: TernaryRelationState::Supported,
                phase: phase(sample_phase),
            }],
        )
        .expect("test wave is valid")
    }

    #[test]
    fn whole_circuit_forms_only_after_three_partial_surfaces_phase_lock() {
        let correct = circuit([0.0, FRAC_PI_2, PI]);
        let competitor = circuit([0.0, 0.0, 0.0]);
        let correct_hash = correct.fingerprint64();
        let mut field =
            CandidateCubeField::new(7, OperatorGrokkingConfig::default()).expect("valid config");
        field.register_circuit(correct).expect("correct circuit");
        field
            .register_circuit(competitor)
            .expect("competing circuit");

        field
            .observe(wave(1, 101, cell(0, 0, 1), 0.0))
            .expect("surface A");
        let first = OperatorGrokkingConsolidator::consolidate(&field);
        assert_eq!(first.stage, OperatorCircuitStage::Memorizing);
        assert!(first.candidate.is_none());

        field
            .observe(wave(2, 202, cell(0, 1, 2), FRAC_PI_2))
            .expect("surface B");
        let second = OperatorGrokkingConsolidator::consolidate(&field);
        assert_eq!(second.stage, OperatorCircuitStage::Memorizing);
        assert!(second.candidate.is_none());

        field
            .observe(wave(3, 303, cell(1, 0, 2), PI))
            .expect("surface C");
        let third = OperatorGrokkingConsolidator::consolidate(&field);
        assert_eq!(third.stage, OperatorCircuitStage::CoherentCandidate);
        let candidate = third.candidate.expect("whole circuit must form");
        assert_eq!(candidate.circuit.fingerprint64(), correct_hash);
        assert!(candidate.coherence > 0.999);
        assert!(candidate.margin_over_runner_up > 0.60);
        assert_eq!(candidate.independent_surfaces, 3);
        assert_eq!(candidate.receipt_ids.as_ref(), &[1, 2, 3]);
        assert!(
            third
                .scores
                .iter()
                .all(|score| !score.one_surface_contains_complete_circuit)
        );
    }

    #[test]
    fn magnitude_only_residuals_do_not_form_the_phase_locked_circuit() {
        let mut field =
            CandidateCubeField::new(7, OperatorGrokkingConfig::default()).expect("valid config");
        field
            .register_circuit(circuit([0.0, FRAC_PI_2, PI]))
            .expect("correct circuit");
        for (receipt, surface, relation_cell) in [
            (1, 101, cell(0, 0, 1)),
            (2, 202, cell(0, 1, 2)),
            (3, 303, cell(1, 0, 2)),
        ] {
            field
                .observe(wave(receipt, surface, relation_cell, 0.0))
                .expect("magnitude-only wave");
        }

        let report = OperatorGrokkingConsolidator::consolidate(&field);
        assert_eq!(report.stage, OperatorCircuitStage::CompetingCircuits);
        assert!(report.candidate.is_none());
        assert!(report.scores[0].coherence < 0.40);
    }

    #[test]
    fn censored_evidence_never_changes_semantic_state() {
        let mut field =
            CandidateCubeField::new(7, OperatorGrokkingConfig::default()).expect("valid config");
        field
            .register_circuit(circuit([0.0, FRAC_PI_2, PI]))
            .expect("correct circuit");
        let mut censored = wave(1, 101, cell(0, 0, 1), 0.0);
        censored.outcome = VerifiedWaveOutcome::CensoredUnknown;
        field.observe(censored).expect("censored receipt");

        let report = OperatorGrokkingConsolidator::consolidate(&field);
        assert_eq!(report.stage, OperatorCircuitStage::Censored);
        assert_eq!(report.semantic_waves, 0);
        assert_eq!(report.censored_waves, 1);
    }
}
