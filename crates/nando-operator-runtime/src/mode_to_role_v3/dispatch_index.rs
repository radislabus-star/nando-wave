use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    BindingValueTypeV1, RuntimeCapabilityDescriptorV3, RuntimeCapabilityKindV3,
    StructuralCandidateFeaturesV3,
};

use super::constraint::{
    CompiledConstraintKindV3, CompiledConstraintV3, DISPATCH_DIMENSIONS_V3, observed_constraints_v3,
};
use super::{CompiledProtocolModeV3, ModeToRoleErrorV3};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CapabilityShapeV3 {
    kind: RuntimeCapabilityKindV3,
    required_arity: usize,
    argument_types: Box<[BindingValueTypeV1]>,
}

#[derive(Clone, Debug)]
struct ModeBitSetV3 {
    words: Box<[u64]>,
}

#[derive(Clone, Debug)]
pub(super) struct StructuralDispatchBitIndexV3 {
    mode_count: usize,
    capability_masks: BTreeMap<CapabilityShapeV3, ModeBitSetV3>,
    wildcard_masks: BTreeMap<(CompiledConstraintKindV3, u8), ModeBitSetV3>,
    exact_masks: BTreeMap<CompiledConstraintV3, ModeBitSetV3>,
}

impl StructuralDispatchBitIndexV3 {
    pub(super) fn build(modes: &[CompiledProtocolModeV3]) -> Self {
        let word_count = modes.len().div_ceil(u64::BITS as usize);
        let mut index = Self {
            mode_count: modes.len(),
            capability_masks: BTreeMap::new(),
            wildcard_masks: DISPATCH_DIMENSIONS_V3
                .into_iter()
                .map(|dimension| (dimension, ModeBitSetV3::zero(word_count)))
                .collect(),
            exact_masks: BTreeMap::new(),
        };
        for (mode_index, mode) in modes.iter().enumerate() {
            index
                .capability_masks
                .entry(CapabilityShapeV3::from_mode(mode))
                .or_insert_with(|| ModeBitSetV3::zero(word_count))
                .insert(mode_index);
            for dimension in DISPATCH_DIMENSIONS_V3 {
                if let Some(constraint) = mode
                    .constraints
                    .iter()
                    .find(|constraint| (constraint.kind, constraint.slot) == dimension)
                {
                    index
                        .exact_masks
                        .entry(*constraint)
                        .or_insert_with(|| ModeBitSetV3::zero(word_count))
                        .insert(mode_index);
                } else {
                    index
                        .wildcard_masks
                        .get_mut(&dimension)
                        .expect("dispatch dimensions are initialized together")
                        .insert(mode_index);
                }
            }
        }
        index
    }

    pub(super) fn matched_mode_indices<'a>(
        &self,
        capabilities: &[RuntimeCapabilityDescriptorV3],
        candidates: impl Iterator<Item = &'a StructuralCandidateFeaturesV3>,
        max_modes: usize,
    ) -> Result<(Vec<usize>, usize), ModeToRoleErrorV3> {
        let word_count = self.mode_count.div_ceil(u64::BITS as usize);
        let mut capability_eligible = ModeBitSetV3::zero(word_count);
        for shape in capabilities
            .iter()
            .map(CapabilityShapeV3::from_runtime)
            .collect::<BTreeSet<_>>()
        {
            if let Some(mask) = self.capability_masks.get(&shape) {
                capability_eligible.union_assign(mask);
            }
        }

        let mut matched = ModeBitSetV3::zero(word_count);
        let mut active = ModeBitSetV3::zero(word_count);
        for features in candidates.collect::<BTreeSet<_>>() {
            active.copy_from(&capability_eligible);
            for observed in observed_constraints_v3(features)? {
                let wildcard = &self.wildcard_masks[&(observed.kind, observed.slot)];
                active.intersect_union_assign(wildcard, self.exact_masks.get(&observed));
                if active.is_empty() {
                    break;
                }
            }
            matched.union_assign(&active);
        }
        let count = matched.count();
        let indices = if count > max_modes {
            Vec::new()
        } else {
            matched.indices(self.mode_count)
        };
        Ok((indices, count))
    }
}

impl CapabilityShapeV3 {
    fn from_mode(mode: &CompiledProtocolModeV3) -> Self {
        Self {
            kind: mode.runtime_capability_kind(),
            required_arity: mode.capability_argument_types.len(),
            argument_types: mode.capability_argument_types.clone(),
        }
    }

    fn from_runtime(capability: &RuntimeCapabilityDescriptorV3) -> Self {
        Self {
            kind: capability.kind,
            required_arity: usize::from(capability.required_arity),
            argument_types: capability.argument_types.clone().into_boxed_slice(),
        }
    }
}

impl ModeBitSetV3 {
    fn zero(word_count: usize) -> Self {
        Self {
            words: vec![0; word_count].into_boxed_slice(),
        }
    }

    fn insert(&mut self, index: usize) {
        self.words[index / u64::BITS as usize] |= 1_u64 << (index % u64::BITS as usize);
    }

    fn union_assign(&mut self, other: &Self) {
        for (target, source) in self.words.iter_mut().zip(other.words.iter()) {
            *target |= source;
        }
    }

    fn copy_from(&mut self, other: &Self) {
        self.words.copy_from_slice(&other.words);
    }

    fn intersect_union_assign(&mut self, wildcard: &Self, exact: Option<&Self>) {
        for index in 0..self.words.len() {
            let exact_word = exact.map_or(0, |mask| mask.words[index]);
            self.words[index] &= wildcard.words[index] | exact_word;
        }
    }

    fn is_empty(&self) -> bool {
        self.words.iter().all(|word| *word == 0)
    }

    fn count(&self) -> usize {
        self.words
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    fn indices(&self, mode_count: usize) -> Vec<usize> {
        (0..mode_count)
            .filter(|index| {
                self.words[index / u64::BITS as usize] & (1_u64 << (index % u64::BITS as usize))
                    != 0
            })
            .collect()
    }
}
