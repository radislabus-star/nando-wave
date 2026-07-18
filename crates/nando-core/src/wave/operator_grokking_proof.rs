use std::collections::BTreeMap;

use super::CoherentOperatorCandidate;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OperatorGrokkingAblation {
    FullPhase,
    NoPhase,
    ShuffledResidual,
    MagnitudeOnly,
    MatchedRandomCenter,
    RestoredPhase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperatorGrokkingProofStage {
    CoherentCandidate,
    FutureVerified,
    CausallyVerified,
    ExactAuthorityCleaned,
    ProvenGrokking,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OperatorGrokkingAblationReceipt {
    pub circuit_formed: bool,
    pub transferred: bool,
    pub circuit_fingerprint64: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperatorGrokkingProofTracker {
    circuit_fingerprint64: u64,
    future_rows: usize,
    future_wrong_accepts: usize,
    runtime_parity_mismatches: usize,
    exact_authority_removed: bool,
    decisions_identical_after_cleanup: bool,
    ablations: BTreeMap<OperatorGrokkingAblation, OperatorGrokkingAblationReceipt>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProvenOperatorGrokking {
    pub circuit_fingerprint64: u64,
    pub generation: u64,
    pub future_rows: usize,
}

impl OperatorGrokkingProofTracker {
    #[must_use]
    pub fn new(candidate: &CoherentOperatorCandidate) -> Self {
        Self {
            circuit_fingerprint64: candidate.circuit.fingerprint64(),
            future_rows: 0,
            future_wrong_accepts: 0,
            runtime_parity_mismatches: 0,
            exact_authority_removed: false,
            decisions_identical_after_cleanup: false,
            ablations: BTreeMap::new(),
        }
    }

    pub fn record_future(
        &mut self,
        rows: usize,
        wrong_accepts: usize,
        runtime_parity_mismatches: usize,
    ) {
        self.future_rows = rows;
        self.future_wrong_accepts = wrong_accepts;
        self.runtime_parity_mismatches = runtime_parity_mismatches;
    }

    pub fn record_ablation(
        &mut self,
        kind: OperatorGrokkingAblation,
        receipt: OperatorGrokkingAblationReceipt,
    ) {
        self.ablations.insert(kind, receipt);
    }

    pub fn record_exact_authority_cleanup(&mut self, removed: bool, decisions_identical: bool) {
        self.exact_authority_removed = removed;
        self.decisions_identical_after_cleanup = decisions_identical;
    }

    #[must_use]
    pub fn stage(&self) -> OperatorGrokkingProofStage {
        if self.proof_complete() {
            return OperatorGrokkingProofStage::ProvenGrokking;
        }
        if self.future_passes()
            && self.causal_controls_pass()
            && self.exact_authority_removed
            && self.decisions_identical_after_cleanup
        {
            return OperatorGrokkingProofStage::ExactAuthorityCleaned;
        }
        if self.future_passes() && self.causal_controls_pass() {
            return OperatorGrokkingProofStage::CausallyVerified;
        }
        if self.future_passes() {
            return OperatorGrokkingProofStage::FutureVerified;
        }
        OperatorGrokkingProofStage::CoherentCandidate
    }

    #[must_use]
    pub fn prove(&self, candidate: &CoherentOperatorCandidate) -> Option<ProvenOperatorGrokking> {
        if !self.proof_complete() || candidate.circuit.fingerprint64() != self.circuit_fingerprint64
        {
            return None;
        }
        Some(ProvenOperatorGrokking {
            circuit_fingerprint64: self.circuit_fingerprint64,
            generation: candidate.candidate_generation,
            future_rows: self.future_rows,
        })
    }

    fn future_passes(&self) -> bool {
        self.future_rows > 0
            && self.future_wrong_accepts == 0
            && self.runtime_parity_mismatches == 0
    }

    fn causal_controls_pass(&self) -> bool {
        let expected = self.circuit_fingerprint64;
        let full = self.ablations.get(&OperatorGrokkingAblation::FullPhase);
        let restored = self.ablations.get(&OperatorGrokkingAblation::RestoredPhase);
        let destructive_controls = [
            OperatorGrokkingAblation::NoPhase,
            OperatorGrokkingAblation::ShuffledResidual,
            OperatorGrokkingAblation::MagnitudeOnly,
            OperatorGrokkingAblation::MatchedRandomCenter,
        ];

        full.is_some_and(|receipt| {
            receipt.circuit_formed
                && receipt.transferred
                && receipt.circuit_fingerprint64 == Some(expected)
        }) && restored.is_some_and(|receipt| {
            receipt.circuit_formed
                && receipt.transferred
                && receipt.circuit_fingerprint64 == Some(expected)
        }) && destructive_controls.iter().all(|kind| {
            self.ablations
                .get(kind)
                .is_some_and(|receipt| !receipt.circuit_formed && !receipt.transferred)
        })
    }

    fn proof_complete(&self) -> bool {
        self.future_passes()
            && self.causal_controls_pass()
            && self.exact_authority_removed
            && self.decisions_identical_after_cleanup
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, PI};

    use super::*;
    use crate::wave::{
        OperatorCircuit, OperatorCircuitRelation, OperatorRelationCell, PhaseCenterCell,
        TernaryRelationState,
    };

    fn candidate() -> CoherentOperatorCandidate {
        let phase = |angle: f64| PhaseCenterCell {
            re: angle.cos(),
            im: angle.sin(),
        };
        let relation = |plane, source_role, target_role, angle| OperatorCircuitRelation {
            cell: OperatorRelationCell {
                plane,
                source_role,
                target_role,
            },
            state: TernaryRelationState::Supported,
            phase_anchor: phase(angle),
        };
        CoherentOperatorCandidate {
            source_generation: 7,
            candidate_generation: 8,
            circuit: OperatorCircuit::new(
                3,
                vec![
                    relation(0, 0, 1, 0.0),
                    relation(0, 1, 2, FRAC_PI_2),
                    relation(1, 0, 2, PI),
                ],
            )
            .expect("connected circuit"),
            coherence: 1.0,
            margin_over_runner_up: 0.5,
            independent_surfaces: 3,
            independent_sessions: 3,
            receipt_ids: vec![1, 2, 3].into_boxed_slice(),
        }
    }

    #[test]
    fn coherent_candidate_is_not_proven_until_future_ablation_and_cleanup_pass() {
        let candidate = candidate();
        let fingerprint = candidate.circuit.fingerprint64();
        let mut proof = OperatorGrokkingProofTracker::new(&candidate);
        assert_eq!(proof.stage(), OperatorGrokkingProofStage::CoherentCandidate);
        assert!(proof.prove(&candidate).is_none());

        proof.record_future(32, 0, 0);
        assert_eq!(proof.stage(), OperatorGrokkingProofStage::FutureVerified);
        assert!(proof.prove(&candidate).is_none());

        for kind in [
            OperatorGrokkingAblation::NoPhase,
            OperatorGrokkingAblation::ShuffledResidual,
            OperatorGrokkingAblation::MagnitudeOnly,
            OperatorGrokkingAblation::MatchedRandomCenter,
        ] {
            proof.record_ablation(kind, OperatorGrokkingAblationReceipt::default());
        }
        let passing = OperatorGrokkingAblationReceipt {
            circuit_formed: true,
            transferred: true,
            circuit_fingerprint64: Some(fingerprint),
        };
        proof.record_ablation(OperatorGrokkingAblation::FullPhase, passing);
        proof.record_ablation(OperatorGrokkingAblation::RestoredPhase, passing);
        assert_eq!(proof.stage(), OperatorGrokkingProofStage::CausallyVerified);
        assert!(proof.prove(&candidate).is_none());

        proof.record_exact_authority_cleanup(true, true);
        assert_eq!(proof.stage(), OperatorGrokkingProofStage::ProvenGrokking);
        let receipt = proof.prove(&candidate).expect("all proof gates pass");
        assert_eq!(receipt.circuit_fingerprint64, fingerprint);
        assert_eq!(receipt.generation, 8);
        assert_eq!(receipt.future_rows, 32);
    }

    #[test]
    fn a_control_that_forms_any_circuit_blocks_the_grokking_claim() {
        let candidate = candidate();
        let fingerprint = candidate.circuit.fingerprint64();
        let mut proof = OperatorGrokkingProofTracker::new(&candidate);
        proof.record_future(32, 0, 0);
        let passing = OperatorGrokkingAblationReceipt {
            circuit_formed: true,
            transferred: true,
            circuit_fingerprint64: Some(fingerprint),
        };
        proof.record_ablation(OperatorGrokkingAblation::FullPhase, passing);
        proof.record_ablation(OperatorGrokkingAblation::RestoredPhase, passing);
        for kind in [
            OperatorGrokkingAblation::NoPhase,
            OperatorGrokkingAblation::ShuffledResidual,
            OperatorGrokkingAblation::MagnitudeOnly,
            OperatorGrokkingAblation::MatchedRandomCenter,
        ] {
            proof.record_ablation(kind, OperatorGrokkingAblationReceipt::default());
        }
        proof.record_ablation(OperatorGrokkingAblation::MagnitudeOnly, passing);
        proof.record_exact_authority_cleanup(true, true);

        assert_ne!(proof.stage(), OperatorGrokkingProofStage::ProvenGrokking);
        assert!(proof.prove(&candidate).is_none());
    }
}
