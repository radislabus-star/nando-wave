use super::{
    F5C_MAX_DISPATCHED_MODES_V3, StructuralDispatchIndexV3, StructuralDispatchReportV3,
    StructuralDispatchVerdictV3,
};
use crate::CanonicalRuntimeRequestV3;

impl StructuralDispatchIndexV3 {
    #[must_use]
    pub fn dispatch(&self, request: &CanonicalRuntimeRequestV3<'_>) -> StructuralDispatchReportV3 {
        let Ok((matched, matched_mode_count)) = self.dispatch_bits.matched_mode_indices(
            &request.view().capabilities,
            request
                .view()
                .structural
                .roles
                .iter()
                .map(|role| &role.features),
            F5C_MAX_DISPATCHED_MODES_V3,
        ) else {
            return StructuralDispatchReportV3 {
                index_sha256: self.index_sha256.clone(),
                mode_indices: Box::new([]),
                matched_mode_count: 0,
                verdict: StructuralDispatchVerdictV3::AbstainDispatchExhausted,
            };
        };
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
            mode_indices: matched.into_boxed_slice(),
            matched_mode_count,
            verdict: StructuralDispatchVerdictV3::Complete,
        }
    }
}
