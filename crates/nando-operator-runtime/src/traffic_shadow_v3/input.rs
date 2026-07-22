use nando_operator_kernel::RuntimeProjectionV3;
use serde_json::Value;

use super::{TrafficShadowInputErrorV3, TrafficShadowSourceV3};

pub struct TrafficShadowInputV3<'a> {
    window_row_sha256: &'a str,
    request_sha256: &'a str,
    projection: Option<RuntimeProjectionV3>,
    streaming: Option<bool>,
    source: TrafficShadowSourceV3,
    request_text: Option<&'a str>,
    provider_payload: Option<&'a Value>,
}

impl<'a> TrafficShadowInputV3<'a> {
    pub fn replayable(
        window_row_sha256: &'a str,
        request_sha256: &'a str,
        projection: RuntimeProjectionV3,
        streaming: bool,
        source: TrafficShadowSourceV3,
        request_text: &'a str,
        provider_payload: &'a Value,
    ) -> Result<Self, TrafficShadowInputErrorV3> {
        Self::build(
            window_row_sha256,
            request_sha256,
            Some(projection),
            Some(streaming),
            source,
            Some(request_text),
            Some(provider_payload),
        )
    }

    pub fn metadata_only(
        window_row_sha256: &'a str,
        request_sha256: &'a str,
        projection: Option<RuntimeProjectionV3>,
        streaming: Option<bool>,
        source: TrafficShadowSourceV3,
    ) -> Result<Self, TrafficShadowInputErrorV3> {
        Self::build(
            window_row_sha256,
            request_sha256,
            projection,
            streaming,
            source,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build(
        window_row_sha256: &'a str,
        request_sha256: &'a str,
        projection: Option<RuntimeProjectionV3>,
        streaming: Option<bool>,
        source: TrafficShadowSourceV3,
        request_text: Option<&'a str>,
        provider_payload: Option<&'a Value>,
    ) -> Result<Self, TrafficShadowInputErrorV3> {
        if !valid_sha256(window_row_sha256) || !valid_sha256(request_sha256) {
            return Err(TrafficShadowInputErrorV3::InvalidDigest);
        }
        if request_text.is_some() != provider_payload.is_some() {
            return Err(TrafficShadowInputErrorV3::PartialRuntimePayload);
        }
        Ok(Self {
            window_row_sha256,
            request_sha256,
            projection,
            streaming,
            source,
            request_text,
            provider_payload,
        })
    }

    pub(super) const fn window_row_sha256(&self) -> &str {
        self.window_row_sha256
    }

    pub(super) const fn request_sha256(&self) -> &str {
        self.request_sha256
    }

    pub(super) const fn projection(&self) -> Option<RuntimeProjectionV3> {
        self.projection
    }

    pub(super) const fn streaming(&self) -> Option<bool> {
        self.streaming
    }

    pub(super) const fn source(&self) -> TrafficShadowSourceV3 {
        self.source
    }

    pub(super) const fn runtime_payload(&self) -> Option<(&str, &Value)> {
        match (self.request_text, self.provider_payload) {
            (Some(request_text), Some(provider_payload)) => Some((request_text, provider_payload)),
            _ => None,
        }
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
