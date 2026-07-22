use nando_operator_kernel::canonical_json_sha256;

use super::{
    GenerationShadowIngressV3, GenerationShadowRequestErrorV3, GenerationShadowRequestV3,
    GenerationShadowRuntimeV3, GenerationShadowSubmitVerdictV3,
};

impl GenerationShadowRuntimeV3 {
    pub(crate) fn observe_provider_request(
        &self,
        ingress: GenerationShadowIngressV3<'_>,
    ) -> GenerationShadowSubmitVerdictV3 {
        if !self.enabled() {
            return GenerationShadowSubmitVerdictV3::CensoredDisabled;
        }
        let window_row_sha256 = match canonical_json_sha256(&(
            "nando.f7e-incoming-shadow-row.v3",
            ingress.request_sha256,
            ingress.request_ordinal,
        )) {
            Ok(root) => root,
            Err(_) => {
                return self
                    .observe_censored(GenerationShadowSubmitVerdictV3::CensoredInvalidRequest);
            }
        };
        // The HTTP capture owner computed request_sha256 from these exact
        // Bytes. F6 repeats that check independently after queue handoff.
        match GenerationShadowRequestV3::from_capture_owner(
            window_row_sha256,
            ingress.request_sha256.to_owned(),
            ingress.projection,
            ingress.streaming,
            ingress.request_text.to_owned(),
            ingress.provider_payload_bytes,
        ) {
            Ok(request) => self.try_submit(request),
            Err(GenerationShadowRequestErrorV3::BudgetExhausted) => {
                self.observe_censored(GenerationShadowSubmitVerdictV3::CensoredBudget)
            }
            Err(
                GenerationShadowRequestErrorV3::InvalidCommitment
                | GenerationShadowRequestErrorV3::RequestDigestMismatch,
            ) => self.observe_censored(GenerationShadowSubmitVerdictV3::CensoredInvalidRequest),
        }
    }
}
