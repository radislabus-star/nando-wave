use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::PhaseCenterCell;

pub const OPERATOR_CIRCUIT_MAX_ROLES: usize = 32;
pub const OPERATOR_CIRCUIT_MAX_RELATIONS: usize = 256;
pub const OPERATOR_WAVE_MAX_SAMPLES: usize = 256;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(i8)]
pub enum TernaryRelationState {
    Opposed = -1,
    Unresolved = 0,
    Supported = 1,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OperatorRelationCell {
    pub plane: u8,
    pub source_role: u8,
    pub target_role: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OperatorCircuitRelation {
    pub cell: OperatorRelationCell,
    pub state: TernaryRelationState,
    pub phase_anchor: PhaseCenterCell,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperatorCircuit {
    role_count: u8,
    relations: Box<[OperatorCircuitRelation]>,
    fingerprint64: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifiedWaveOutcome {
    Positive,
    ApplicabilityNegative,
    HardContradiction,
    CensoredUnknown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VerifiedRelationSample {
    pub cell: OperatorRelationCell,
    pub state: TernaryRelationState,
    pub phase: PhaseCenterCell,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VerifiedPartialRelationWave {
    pub receipt_id: u64,
    pub surface_id: u64,
    pub session_id: u64,
    pub generation: u64,
    pub outcome: VerifiedWaveOutcome,
    pub samples: Box<[VerifiedRelationSample]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperatorCircuitError {
    EmptyCircuit,
    TooManyRoles,
    TooManyRelations,
    InvalidRole,
    SelfRelation,
    DuplicateRelation,
    UnresolvedCircuitRelation,
    DisconnectedCircuit,
    EmptyWave,
    TooManySamples,
    DuplicateSample,
}

impl OperatorCircuit {
    pub fn new(
        role_count: u8,
        mut relations: Vec<OperatorCircuitRelation>,
    ) -> Result<Self, OperatorCircuitError> {
        if role_count == 0 || relations.is_empty() {
            return Err(OperatorCircuitError::EmptyCircuit);
        }
        if usize::from(role_count) > OPERATOR_CIRCUIT_MAX_ROLES {
            return Err(OperatorCircuitError::TooManyRoles);
        }
        if relations.len() > OPERATOR_CIRCUIT_MAX_RELATIONS {
            return Err(OperatorCircuitError::TooManyRelations);
        }
        relations.sort_by_key(|relation| relation.cell);

        let mut previous = None;
        for relation in &relations {
            if relation.cell.source_role >= role_count || relation.cell.target_role >= role_count {
                return Err(OperatorCircuitError::InvalidRole);
            }
            if relation.cell.source_role == relation.cell.target_role {
                return Err(OperatorCircuitError::SelfRelation);
            }
            if relation.state == TernaryRelationState::Unresolved {
                return Err(OperatorCircuitError::UnresolvedCircuitRelation);
            }
            if previous == Some(relation.cell) {
                return Err(OperatorCircuitError::DuplicateRelation);
            }
            previous = Some(relation.cell);
        }
        if !relations_are_connected(role_count, &relations) {
            return Err(OperatorCircuitError::DisconnectedCircuit);
        }

        let fingerprint64 = circuit_fingerprint64(role_count, &relations);
        Ok(Self {
            role_count,
            relations: relations.into_boxed_slice(),
            fingerprint64,
        })
    }

    #[must_use]
    pub const fn role_count(&self) -> u8 {
        self.role_count
    }

    #[must_use]
    pub fn relations(&self) -> &[OperatorCircuitRelation] {
        &self.relations
    }

    #[must_use]
    pub const fn fingerprint64(&self) -> u64 {
        self.fingerprint64
    }

    #[must_use]
    pub fn relation(&self, cell: OperatorRelationCell) -> Option<&OperatorCircuitRelation> {
        self.relations
            .binary_search_by_key(&cell, |relation| relation.cell)
            .ok()
            .map(|index| &self.relations[index])
    }
}

impl VerifiedPartialRelationWave {
    pub fn new(
        receipt_id: u64,
        surface_id: u64,
        session_id: u64,
        generation: u64,
        outcome: VerifiedWaveOutcome,
        mut samples: Vec<VerifiedRelationSample>,
    ) -> Result<Self, OperatorCircuitError> {
        if samples.is_empty() {
            return Err(OperatorCircuitError::EmptyWave);
        }
        if samples.len() > OPERATOR_WAVE_MAX_SAMPLES {
            return Err(OperatorCircuitError::TooManySamples);
        }
        samples.sort_by_key(|sample| sample.cell);
        if samples.windows(2).any(|pair| pair[0].cell == pair[1].cell) {
            return Err(OperatorCircuitError::DuplicateSample);
        }
        Ok(Self {
            receipt_id,
            surface_id,
            session_id,
            generation,
            outcome,
            samples: samples.into_boxed_slice(),
        })
    }
}

fn relations_are_connected(role_count: u8, relations: &[OperatorCircuitRelation]) -> bool {
    let mut adjacency = BTreeMap::<u8, BTreeSet<u8>>::new();
    let mut active_roles = BTreeSet::new();
    for relation in relations {
        active_roles.insert(relation.cell.source_role);
        active_roles.insert(relation.cell.target_role);
        adjacency
            .entry(relation.cell.source_role)
            .or_default()
            .insert(relation.cell.target_role);
        adjacency
            .entry(relation.cell.target_role)
            .or_default()
            .insert(relation.cell.source_role);
    }
    if active_roles.len() != usize::from(role_count) {
        return false;
    }

    let Some(first) = active_roles.first().copied() else {
        return false;
    };
    let mut seen = BTreeSet::from([first]);
    let mut queue = VecDeque::from([first]);
    while let Some(role) = queue.pop_front() {
        if let Some(neighbors) = adjacency.get(&role) {
            for neighbor in neighbors {
                if seen.insert(*neighbor) {
                    queue.push_back(*neighbor);
                }
            }
        }
    }
    seen == active_roles
}

fn circuit_fingerprint64(role_count: u8, relations: &[OperatorCircuitRelation]) -> u64 {
    // Stable FNV-1a keeps the pure core dependency-free. It is an identity key,
    // not a cryptographic proof hash; cold proof packages bind a stronger hash.
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    hash = fnv_byte(hash, role_count);
    for relation in relations {
        hash = fnv_byte(hash, relation.cell.plane);
        hash = fnv_byte(hash, relation.cell.source_role);
        hash = fnv_byte(hash, relation.cell.target_role);
        hash = fnv_byte(hash, relation.state as i8 as u8);
        hash = fnv_bytes(hash, &relation.phase_anchor.re.to_bits().to_le_bytes());
        hash = fnv_bytes(hash, &relation.phase_anchor.im.to_bits().to_le_bytes());
    }
    hash
}

fn fnv_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash = fnv_byte(hash, *byte);
    }
    hash
}

const fn fnv_byte(hash: u64, byte: u8) -> u64 {
    (hash ^ byte as u64).wrapping_mul(0x0000_0100_0000_01b3)
}
