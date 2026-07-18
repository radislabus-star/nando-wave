use std::collections::{BTreeMap, BTreeSet};

use super::{
    CandidateCubeField, CandidateCubeFieldError, OperatorCircuit, OperatorCircuitError,
    OperatorCircuitRelation, OperatorGrokkingConfig, OperatorRelationCell, PhaseCenterCell,
    TernaryRelationState, VerifiedPartialRelationWave, VerifiedWaveOutcome,
};

pub const CIRCUIT_SYNTHESIS_MAX_FRAGMENTS: usize = 256;
pub const CIRCUIT_SYNTHESIS_MAX_CIRCUITS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RelationFragment {
    pub receipt_id: u64,
    pub surface_id: u64,
    pub session_id: u64,
    pub cell: OperatorRelationCell,
    pub state: TernaryRelationState,
    pub phase: PhaseCenterCell,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RelationFragmentReport {
    pub positive_samples: usize,
    pub emitted_fragments: usize,
    pub unresolved_fragments: usize,
    pub zero_phase_fragments: usize,
    pub ignored_non_positive_waves: usize,
    pub truncated_fragments: usize,
    pub fragments: Box<[RelationFragment]>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CircuitSynthesisBlocker {
    NoPositiveFragments,
    NonCanonicalRoleSpace,
    ZeroAggregatePhase,
    DisconnectedCircuit,
    InvalidCircuit,
    CircuitCapacityReached,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CircuitSynthesisBlockerCount {
    pub blocker: CircuitSynthesisBlocker,
    pub count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CircuitSynthesisConfig {
    pub max_circuits: usize,
    pub max_fragments: usize,
}

impl Default for CircuitSynthesisConfig {
    fn default() -> Self {
        Self {
            max_circuits: CIRCUIT_SYNTHESIS_MAX_CIRCUITS,
            max_fragments: CIRCUIT_SYNTHESIS_MAX_FRAGMENTS,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperatorCircuitSynthesisReport {
    pub fragments: RelationFragmentReport,
    pub structural_cells: usize,
    pub state_assignments: usize,
    pub emitted_circuits: usize,
    pub duplicate_circuits: usize,
    pub blocked_circuits: usize,
    pub truncated_circuits: usize,
    pub blockers: Box<[CircuitSynthesisBlockerCount]>,
    pub circuits: Box<[OperatorCircuit]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CircuitSynthesisError {
    InvalidConfig,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrozenCircuitSetError {
    NoSynthesizedCircuits,
    SupportReceiptReused,
    CandidateField(CandidateCubeFieldError),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrozenSynthesizedCircuitSet {
    source_generation: u64,
    support_receipt_ids: Box<[u64]>,
    circuits: Box<[OperatorCircuit]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrozenFutureCircuitField {
    support_receipt_ids: Box<[u64]>,
    field: CandidateCubeField,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RelationFragmentGenerator;

#[derive(Clone, Copy, Debug, Default)]
pub struct CircuitSynthesizer;

#[derive(Clone, Debug, Default)]
struct AggregateFragment {
    phase_re: f64,
    phase_im: f64,
}

impl RelationFragmentGenerator {
    #[must_use]
    pub fn generate(
        waves: &[VerifiedPartialRelationWave],
        max_fragments: usize,
    ) -> RelationFragmentReport {
        let mut fragments = Vec::with_capacity(max_fragments.min(CIRCUIT_SYNTHESIS_MAX_FRAGMENTS));
        let mut positive_samples = 0_usize;
        let mut unresolved_fragments = 0_usize;
        let mut zero_phase_fragments = 0_usize;
        let mut ignored_non_positive_waves = 0_usize;
        let mut truncated_fragments = 0_usize;

        for wave in waves {
            if wave.outcome != VerifiedWaveOutcome::Positive {
                ignored_non_positive_waves = ignored_non_positive_waves.saturating_add(1);
                continue;
            }
            for sample in &wave.samples {
                positive_samples = positive_samples.saturating_add(1);
                if sample.state == TernaryRelationState::Unresolved {
                    unresolved_fragments = unresolved_fragments.saturating_add(1);
                    continue;
                }
                if sample.phase.re.hypot(sample.phase.im) <= f64::EPSILON {
                    zero_phase_fragments = zero_phase_fragments.saturating_add(1);
                    continue;
                }
                if fragments.len() == max_fragments {
                    truncated_fragments = truncated_fragments.saturating_add(1);
                    continue;
                }
                fragments.push(RelationFragment {
                    receipt_id: wave.receipt_id,
                    surface_id: wave.surface_id,
                    session_id: wave.session_id,
                    cell: sample.cell,
                    state: sample.state,
                    phase: sample.phase,
                });
            }
        }

        RelationFragmentReport {
            positive_samples,
            emitted_fragments: fragments.len(),
            unresolved_fragments,
            zero_phase_fragments,
            ignored_non_positive_waves,
            truncated_fragments,
            fragments: fragments.into_boxed_slice(),
        }
    }
}

impl CircuitSynthesizer {
    pub fn synthesize(
        support_waves: &[VerifiedPartialRelationWave],
        config: CircuitSynthesisConfig,
    ) -> Result<OperatorCircuitSynthesisReport, CircuitSynthesisError> {
        if config.max_circuits == 0
            || config.max_circuits > CIRCUIT_SYNTHESIS_MAX_CIRCUITS
            || config.max_fragments == 0
            || config.max_fragments > CIRCUIT_SYNTHESIS_MAX_FRAGMENTS
        {
            return Err(CircuitSynthesisError::InvalidConfig);
        }

        let fragments = RelationFragmentGenerator::generate(support_waves, config.max_fragments);
        let mut blocker_counts = BTreeMap::<CircuitSynthesisBlocker, usize>::new();
        let mut aggregates = BTreeMap::<
            OperatorRelationCell,
            BTreeMap<TernaryRelationState, AggregateFragment>,
        >::new();
        for fragment in &fragments.fragments {
            let aggregate = aggregates
                .entry(fragment.cell)
                .or_default()
                .entry(fragment.state)
                .or_default();
            aggregate.phase_re += fragment.phase.re;
            aggregate.phase_im += fragment.phase.im;
        }

        if aggregates.is_empty() {
            add_blocker(
                &mut blocker_counts,
                CircuitSynthesisBlocker::NoPositiveFragments,
            );
        }

        let structural_cells = aggregates.len();
        let role_count = canonical_role_count(aggregates.keys().copied());
        if !aggregates.is_empty() && role_count.is_none() {
            add_blocker(
                &mut blocker_counts,
                CircuitSynthesisBlocker::NonCanonicalRoleSpace,
            );
        }

        let mut alternatives = Vec::<Vec<OperatorCircuitRelation>>::new();
        for (cell, states) in aggregates {
            let mut cell_alternatives = Vec::new();
            for (state, aggregate) in states {
                let magnitude = aggregate.phase_re.hypot(aggregate.phase_im);
                if magnitude <= f64::EPSILON {
                    add_blocker(
                        &mut blocker_counts,
                        CircuitSynthesisBlocker::ZeroAggregatePhase,
                    );
                    continue;
                }
                cell_alternatives.push(OperatorCircuitRelation {
                    cell,
                    state,
                    phase_anchor: PhaseCenterCell {
                        re: aggregate.phase_re / magnitude,
                        im: aggregate.phase_im / magnitude,
                    },
                });
            }
            if !cell_alternatives.is_empty() {
                alternatives.push(cell_alternatives);
            }
        }

        let mut assignments = Vec::new();
        enumerate_assignments(
            &alternatives,
            0,
            &mut Vec::new(),
            config.max_circuits.saturating_add(1),
            &mut assignments,
        );
        let state_assignments = assignments.len();
        let mut circuits = Vec::new();
        let mut fingerprints = BTreeSet::new();
        let mut duplicate_circuits = 0_usize;
        let mut blocked_circuits = 0_usize;
        let mut truncated_circuits = 0_usize;

        if let Some(role_count) = role_count {
            for relations in assignments {
                if circuits.len() == config.max_circuits {
                    truncated_circuits = truncated_circuits.saturating_add(1);
                    add_blocker(
                        &mut blocker_counts,
                        CircuitSynthesisBlocker::CircuitCapacityReached,
                    );
                    continue;
                }
                match OperatorCircuit::new(role_count, relations) {
                    Ok(circuit) => {
                        if fingerprints.insert(circuit.fingerprint64()) {
                            circuits.push(circuit);
                        } else {
                            duplicate_circuits = duplicate_circuits.saturating_add(1);
                        }
                    }
                    Err(OperatorCircuitError::DisconnectedCircuit) => {
                        blocked_circuits = blocked_circuits.saturating_add(1);
                        add_blocker(
                            &mut blocker_counts,
                            CircuitSynthesisBlocker::DisconnectedCircuit,
                        );
                    }
                    Err(_) => {
                        blocked_circuits = blocked_circuits.saturating_add(1);
                        add_blocker(&mut blocker_counts, CircuitSynthesisBlocker::InvalidCircuit);
                    }
                }
            }
        }
        circuits.sort_by_key(OperatorCircuit::fingerprint64);

        Ok(OperatorCircuitSynthesisReport {
            fragments,
            structural_cells,
            state_assignments,
            emitted_circuits: circuits.len(),
            duplicate_circuits,
            blocked_circuits,
            truncated_circuits,
            blockers: blocker_counts
                .into_iter()
                .map(|(blocker, count)| CircuitSynthesisBlockerCount { blocker, count })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            circuits: circuits.into_boxed_slice(),
        })
    }
}

impl FrozenSynthesizedCircuitSet {
    pub fn freeze(
        source_generation: u64,
        report: &OperatorCircuitSynthesisReport,
    ) -> Result<Self, FrozenCircuitSetError> {
        if report.circuits.is_empty() {
            return Err(FrozenCircuitSetError::NoSynthesizedCircuits);
        }
        let support_receipt_ids = report
            .fragments
            .fragments
            .iter()
            .map(|fragment| fragment.receipt_id)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Ok(Self {
            source_generation,
            support_receipt_ids,
            circuits: report.circuits.clone(),
        })
    }

    pub fn new_future_field(
        &self,
        config: OperatorGrokkingConfig,
    ) -> Result<FrozenFutureCircuitField, FrozenCircuitSetError> {
        let mut field = CandidateCubeField::new(self.source_generation, config)
            .map_err(FrozenCircuitSetError::CandidateField)?;
        for circuit in &self.circuits {
            field
                .register_circuit(circuit.clone())
                .map_err(FrozenCircuitSetError::CandidateField)?;
        }
        Ok(FrozenFutureCircuitField {
            support_receipt_ids: self.support_receipt_ids.clone(),
            field,
        })
    }

    #[must_use]
    pub const fn source_generation(&self) -> u64 {
        self.source_generation
    }

    #[must_use]
    pub fn support_receipt_ids(&self) -> &[u64] {
        &self.support_receipt_ids
    }

    #[must_use]
    pub fn circuits(&self) -> &[OperatorCircuit] {
        &self.circuits
    }
}

impl FrozenFutureCircuitField {
    pub fn observe(
        &mut self,
        wave: VerifiedPartialRelationWave,
    ) -> Result<(), FrozenCircuitSetError> {
        if self
            .support_receipt_ids
            .binary_search(&wave.receipt_id)
            .is_ok()
        {
            return Err(FrozenCircuitSetError::SupportReceiptReused);
        }
        self.field
            .observe(wave)
            .map_err(FrozenCircuitSetError::CandidateField)
    }

    #[must_use]
    pub fn candidate_field(&self) -> &CandidateCubeField {
        &self.field
    }
}

fn canonical_role_count(cells: impl Iterator<Item = OperatorRelationCell>) -> Option<u8> {
    let roles = cells
        .flat_map(|cell| [cell.source_role, cell.target_role])
        .collect::<BTreeSet<_>>();
    let max_role = roles.last().copied()?;
    let expected = (0..=max_role).collect::<BTreeSet<_>>();
    (roles == expected).then_some(max_role.saturating_add(1))
}

fn enumerate_assignments(
    alternatives: &[Vec<OperatorCircuitRelation>],
    index: usize,
    current: &mut Vec<OperatorCircuitRelation>,
    limit: usize,
    output: &mut Vec<Vec<OperatorCircuitRelation>>,
) {
    if output.len() >= limit {
        return;
    }
    if index == alternatives.len() {
        if !current.is_empty() {
            output.push(current.clone());
        }
        return;
    }
    for relation in &alternatives[index] {
        current.push(*relation);
        enumerate_assignments(alternatives, index + 1, current, limit, output);
        current.pop();
        if output.len() >= limit {
            break;
        }
    }
}

fn add_blocker(
    blockers: &mut BTreeMap<CircuitSynthesisBlocker, usize>,
    blocker: CircuitSynthesisBlocker,
) {
    let count = blockers.entry(blocker).or_default();
    *count = count.saturating_add(1);
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, PI};

    use super::*;
    use crate::wave::{VerifiedRelationSample, VerifiedWaveOutcome};

    fn wave(
        id: u64,
        outcome: VerifiedWaveOutcome,
        cell: OperatorRelationCell,
        state: TernaryRelationState,
        angle: f64,
    ) -> VerifiedPartialRelationWave {
        VerifiedPartialRelationWave::new(
            id,
            id,
            id + 10,
            7,
            outcome,
            vec![VerifiedRelationSample {
                cell,
                state,
                phase: PhaseCenterCell {
                    re: angle.cos(),
                    im: angle.sin(),
                },
            }],
        )
        .expect("valid wave")
    }

    #[test]
    fn only_positive_verified_waves_create_topology_fragments() {
        let cell = OperatorRelationCell {
            plane: 0,
            source_role: 0,
            target_role: 1,
        };
        let report = RelationFragmentGenerator::generate(
            &[
                wave(
                    1,
                    VerifiedWaveOutcome::Positive,
                    cell,
                    TernaryRelationState::Supported,
                    0.0,
                ),
                wave(
                    2,
                    VerifiedWaveOutcome::ApplicabilityNegative,
                    cell,
                    TernaryRelationState::Opposed,
                    PI,
                ),
                wave(
                    3,
                    VerifiedWaveOutcome::HardContradiction,
                    cell,
                    TernaryRelationState::Opposed,
                    PI,
                ),
            ],
            16,
        );

        assert_eq!(report.positive_samples, 1);
        assert_eq!(report.emitted_fragments, 1);
        assert_eq!(report.ignored_non_positive_waves, 2);
        assert_eq!(report.fragments[0].state, TernaryRelationState::Supported);
    }

    #[test]
    fn constructs_a_connected_circuit_absent_from_the_initial_candidate_set() {
        let support = [
            wave(
                1,
                VerifiedWaveOutcome::Positive,
                OperatorRelationCell {
                    plane: 0,
                    source_role: 0,
                    target_role: 1,
                },
                TernaryRelationState::Supported,
                0.0,
            ),
            wave(
                2,
                VerifiedWaveOutcome::Positive,
                OperatorRelationCell {
                    plane: 1,
                    source_role: 1,
                    target_role: 2,
                },
                TernaryRelationState::Supported,
                FRAC_PI_2,
            ),
            wave(
                3,
                VerifiedWaveOutcome::Positive,
                OperatorRelationCell {
                    plane: 2,
                    source_role: 0,
                    target_role: 2,
                },
                TernaryRelationState::Opposed,
                PI,
            ),
        ];

        let report = CircuitSynthesizer::synthesize(&support, CircuitSynthesisConfig::default())
            .expect("bounded synthesis");

        assert_eq!(report.fragments.emitted_fragments, 3);
        assert_eq!(report.structural_cells, 3);
        assert_eq!(report.emitted_circuits, 1);
        assert_eq!(report.circuits[0].relations().len(), 3);
        assert_eq!(report.circuits[0].role_count(), 3);
        assert!(report.blockers.is_empty());
    }
}
