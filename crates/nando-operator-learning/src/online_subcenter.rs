use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

const MAX_ATOMS_PER_EVENT: usize = 96;
const CANDIDATE_ATOMS_PER_EVENT: usize = 16;
const MIN_RECURRENT_ATOM_ROWS: u32 = 8;
const MAX_TRACKED_PAIRS: usize = 32_768;
const PAIR_VETO_BITS: usize = 1 << 15;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PairVetoBloom {
    bits: Vec<u64>,
}

impl Default for PairVetoBloom {
    fn default() -> Self {
        Self {
            bits: vec![0; PAIR_VETO_BITS / 64],
        }
    }
}

impl PairVetoBloom {
    fn insert(&mut self, pair: (u64, u64)) {
        for bit in pair_veto_bits(pair) {
            self.bits[bit / 64] |= 1_u64 << (bit % 64);
        }
    }

    fn contains(&self, pair: (u64, u64)) -> bool {
        pair_veto_bits(pair)
            .into_iter()
            .all(|bit| self.bits[bit / 64] & (1_u64 << (bit % 64)) != 0)
    }
}

fn pair_veto_bits((left, right): (u64, u64)) -> [usize; 3] {
    let first = mix64(left ^ right.rotate_left(17));
    let second = mix64(right ^ left.rotate_left(31) ^ 0x9e37_79b9_7f4a_7c15);
    let third = mix64(first ^ second.rotate_left(23));
    [first, second, third].map(|hash| hash as usize & (PAIR_VETO_BITS - 1))
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct ActionEvidence {
    rows: u32,
    tokens: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct FeatureEvidence {
    total_rows: u32,
    by_action: BTreeMap<String, ActionEvidence>,
}

impl FeatureEvidence {
    fn observe(&mut self, action: &str, tokens: u64) {
        self.total_rows = self.total_rows.saturating_add(1);
        let action = self.by_action.entry(action.to_owned()).or_default();
        action.rows = action.rows.saturating_add(1);
        action.tokens = action.tokens.saturating_add(tokens);
    }

    fn clean_for(&self, action: &str, minimum_rows: u32) -> Option<(u32, u64)> {
        let action = self.by_action.get(action)?;
        (action.rows >= minimum_rows && action.rows == self.total_rows)
            .then_some((action.rows, action.tokens))
    }

    fn for_action(&self, action: &str, minimum_rows: u32) -> Option<(u32, u64)> {
        let action = self.by_action.get(action)?;
        (action.rows >= minimum_rows).then_some((action.rows, action.tokens))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[doc(hidden)]
pub struct CleanSubcenter {
    pub atom_ids: Vec<u64>,
    pub positive_rows: u32,
    pub positive_tokens: u64,
}

/// One-pass action-neutral split discovery. Each feature records which completed
/// teacher actions co-occurred with it; a feature is clean only when every
/// observed occurrence belongs to one action.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[doc(hidden)]
pub struct OnlineSubcenterDiscovery {
    atoms: BTreeMap<u64, FeatureEvidence>,
    pairs: BTreeMap<u64, BTreeMap<u64, FeatureEvidence>>,
    pair_presence_by_action: BTreeMap<String, PairVetoBloom>,
    pair_count: usize,
    rows_seen: u64,
    truncated_rows: u64,
}

impl OnlineSubcenterDiscovery {
    pub fn observe(&mut self, action: &str, atom_ids: &[u64], tokens: u64) {
        let mut atoms = atom_ids.to_vec();
        atoms.sort_unstable();
        atoms.dedup();
        if atoms.len() > MAX_ATOMS_PER_EVENT {
            atoms.truncate(MAX_ATOMS_PER_EVENT);
            self.truncated_rows = self.truncated_rows.saturating_add(1);
        }
        self.rows_seen = self.rows_seen.saturating_add(1);
        for atom in &atoms {
            self.atoms.entry(*atom).or_default().observe(action, tokens);
        }
        let presence = self
            .pair_presence_by_action
            .entry(action.to_owned())
            .or_default();
        for left_index in 0..atoms.len() {
            for right in &atoms[left_index + 1..] {
                presence.insert((atoms[left_index], *right));
            }
        }
        atoms.retain(|atom| {
            self.atoms
                .get(atom)
                .and_then(|evidence| evidence.by_action.get(action))
                .is_some_and(|evidence| evidence.rows >= MIN_RECURRENT_ATOM_ROWS)
        });
        atoms.truncate(CANDIDATE_ATOMS_PER_EVENT);
        for left_index in 0..atoms.len() {
            for right in &atoms[left_index + 1..] {
                let left = atoms[left_index];
                let exists = self
                    .pairs
                    .get(&left)
                    .is_some_and(|rights| rights.contains_key(right));
                if !exists && self.pair_count >= MAX_TRACKED_PAIRS {
                    continue;
                }
                let evidence = self
                    .pairs
                    .entry(left)
                    .or_default()
                    .entry(*right)
                    .or_insert_with(|| {
                        self.pair_count = self.pair_count.saturating_add(1);
                        FeatureEvidence::default()
                    });
                evidence.observe(action, tokens);
            }
        }
    }

    pub fn clean_subcenters(
        &self,
        action: &str,
        minimum_rows: u32,
        limit: usize,
    ) -> Vec<CleanSubcenter> {
        let mut clean = self
            .atoms
            .iter()
            .filter_map(|(atom, evidence)| {
                evidence
                    .clean_for(action, minimum_rows)
                    .map(|(positive_rows, positive_tokens)| CleanSubcenter {
                        atom_ids: vec![*atom],
                        positive_rows,
                        positive_tokens,
                    })
            })
            .collect::<Vec<_>>();
        for (left, rights) in &self.pairs {
            clean.extend(rights.iter().filter_map(|(right, evidence)| {
                evidence
                    .for_action(action, minimum_rows)
                    .filter(|_| {
                        self.pair_presence_by_action
                            .iter()
                            .filter(|(observed_action, _)| observed_action.as_str() != action)
                            .all(|(_, pairs)| !pairs.contains((*left, *right)))
                    })
                    .map(|(positive_rows, positive_tokens)| CleanSubcenter {
                        atom_ids: vec![*left, *right],
                        positive_rows,
                        positive_tokens,
                    })
            }));
        }
        clean.sort_by(|left, right| {
            right
                .positive_tokens
                .cmp(&left.positive_tokens)
                .then_with(|| right.positive_rows.cmp(&left.positive_rows))
                .then_with(|| left.atom_ids.len().cmp(&right.atom_ids.len()))
                .then_with(|| left.atom_ids.cmp(&right.atom_ids))
        });
        clean.truncate(limit);
        clean
    }

    pub fn pair_count(&self) -> usize {
        self.pair_count
    }

    pub fn rows_seen(&self) -> u64 {
        self.rows_seen
    }

    pub fn bytes_estimate(&self) -> usize {
        self.atoms
            .len()
            .saturating_mul(64)
            .saturating_add(self.pair_count.saturating_mul(80))
            .saturating_add(
                self.pair_presence_by_action
                    .len()
                    .saturating_mul(PAIR_VETO_BITS / 8),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_pair_is_vetoed_by_competing_action_even_outside_candidate_top_k() {
        let mut discovery = OnlineSubcenterDiscovery::default();
        for _ in 0..72 {
            discovery.observe("function:wait", &[100, 101], 10);
        }
        let mut competing = (0_u64..20).collect::<Vec<_>>();
        competing.extend([100, 101]);
        discovery.observe("function:write_stdin", &competing, 10);

        assert!(
            discovery
                .clean_subcenters("function:wait", 64, 256)
                .iter()
                .all(|candidate| candidate.atom_ids != [100, 101])
        );
    }

    #[test]
    fn repeated_action_pair_becomes_clean_subcenter() {
        let mut discovery = OnlineSubcenterDiscovery::default();
        for _ in 0..72 {
            discovery.observe("function:wait", &[100, 101], 10);
        }

        assert!(
            discovery
                .clean_subcenters("function:wait", 64, 256)
                .iter()
                .any(|candidate| candidate.atom_ids == [100, 101])
        );
    }
}
