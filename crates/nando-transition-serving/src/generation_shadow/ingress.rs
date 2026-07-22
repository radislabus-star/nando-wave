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
        match GenerationShadowRequestV3::from_provider_capture(
            ingress.capture_receipt,
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
