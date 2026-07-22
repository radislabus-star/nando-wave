use std::collections::BTreeSet;

use super::{
    F5C_MAX_DISPATCHED_MODES_V3, StructuralDispatchIndexV3, StructuralDispatchReportV3,
    StructuralDispatchVerdictV3,
};
use crate::CanonicalRuntimeRequestV3;

impl StructuralDispatchIndexV3 {
    #[must_use]
    pub fn dispatch(&self, request: &CanonicalRuntimeRequestV3<'_>) -> StructuralDispatchReportV3 {
        let source_types = request
            .view()
            .structural
            .roles
            .iter()
            .map(|role| role.features.value_type)
            .collect::<BTreeSet<_>>();
        let mut matched = BTreeSet::new();
        for source_type in source_types {
            if let Some(indices) = self.source_type_buckets.get(&source_type) {
                matched.extend(indices.iter().copied());
            }
        }
        let matched_mode_count = matched.len();
        if matched_mode_count > F5C_MAX_DISPATCHED_MODES_V3 {
            return StructuralDispatchReportV3 {
                index_sha256: self.index_sha256.clone(),
                mode_indices: Box::new([]),
                matched_mode_count,
                verdict: StructuralDispatchVerdictV3::AbstainDispatchExhausted,
            };
        }
        StructuralDispatchReportV3 {
            index_sha256: self.index_sha256.clone(),
            mode_indices: matched.into_iter().collect::<Vec<_>>().into_boxed_slice(),
            matched_mode_count,
            verdict: StructuralDispatchVerdictV3::Complete,
        }
    }
}
