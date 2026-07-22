use std::time::Instant;

use nando_operator_kernel::{RuntimeProjectionV3, sha256_bytes};

use super::{
    TrafficShadowGenerationV3, TrafficShadowInputV3, TrafficShadowReceiptV3, TrafficShadowSourceV3,
    TrafficShadowVerdictV3,
};

pub(super) struct TrafficShadowReceiptBuilderV3 {
    started: Instant,
    window_row_sha256: String,
    request_sha256: String,
    generation_sequence: u64,
    generation_root_sha256: String,
    index_sha256: String,
    projection: Option<RuntimeProjectionV3>,
    streaming: Option<bool>,
    source: TrafficShadowSourceV3,
    extraction_receipt_sha256: Option<String>,
    phase_report_sha256: Option<String>,
    operator_shadow_receipt_sha256: Option<String>,
}

impl TrafficShadowReceiptBuilderV3 {
    pub(super) fn new(
        generation: &TrafficShadowGenerationV3,
        input: &TrafficShadowInputV3<'_>,
    ) -> Self {
        Self {
            started: Instant::now(),
            window_row_sha256: input.window_row_sha256().to_owned(),
            request_sha256: input.request_sha256().to_owned(),
            generation_sequence: generation.sequence(),
            generation_root_sha256: generation.generation_root_sha256().to_owned(),
            index_sha256: generation.index_sha256().to_owned(),
            projection: input.projection(),
            streaming: input.streaming(),
            source: input.source(),
            extraction_receipt_sha256: None,
            phase_report_sha256: None,
            operator_shadow_receipt_sha256: None,
        }
    }

    pub(super) fn set_extraction_receipt(&mut self, value: &str) {
        self.extraction_receipt_sha256 = Some(value.to_owned());
    }

    pub(super) fn set_phase_report(&mut self, value: &str) {
        self.phase_report_sha256 = Some(value.to_owned());
    }

    pub(super) fn set_operator_shadow_receipt(&mut self, value: &str) {
        self.operator_shadow_receipt_sha256 = Some(value.to_owned());
    }

    pub(super) fn finish(self, verdict: TrafficShadowVerdictV3) -> TrafficShadowReceiptV3 {
        let receipt_sha256 = receipt_digest(&self, verdict);
        let elapsed_nanos = u64::try_from(self.started.elapsed().as_nanos()).unwrap_or(u64::MAX);
        TrafficShadowReceiptV3 {
            receipt_sha256,
            window_row_sha256: self.window_row_sha256,
            request_sha256: self.request_sha256,
            generation_sequence: self.generation_sequence,
            generation_root_sha256: self.generation_root_sha256,
            index_sha256: self.index_sha256,
            projection: self.projection,
            streaming: self.streaming,
            source: self.source,
            verdict,
            extraction_receipt_sha256: self.extraction_receipt_sha256,
            phase_report_sha256: self.phase_report_sha256,
            operator_shadow_receipt_sha256: self.operator_shadow_receipt_sha256,
            elapsed_nanos,
            raw_payloads_persisted: 0,
            local_accepts: 0,
            execution_authority: false,
        }
    }
}

impl TrafficShadowReceiptV3 {
    #[must_use]
    pub fn receipt_sha256(&self) -> &str {
        &self.receipt_sha256
    }

    #[must_use]
    pub fn window_row_sha256(&self) -> &str {
        &self.window_row_sha256
    }

    #[must_use]
    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    #[must_use]
    pub const fn generation_sequence(&self) -> u64 {
        self.generation_sequence
    }

    #[must_use]
    pub fn generation_root_sha256(&self) -> &str {
        &self.generation_root_sha256
    }

    #[must_use]
    pub fn index_sha256(&self) -> &str {
        &self.index_sha256
    }

    #[must_use]
    pub const fn projection(&self) -> Option<RuntimeProjectionV3> {
        self.projection
    }

    #[must_use]
    pub const fn streaming(&self) -> Option<bool> {
        self.streaming
    }

    #[must_use]
    pub const fn source(&self) -> TrafficShadowSourceV3 {
        self.source
    }

    #[must_use]
    pub const fn verdict(&self) -> TrafficShadowVerdictV3 {
        self.verdict
    }

    #[must_use]
    pub fn extraction_receipt_sha256(&self) -> Option<&str> {
        self.extraction_receipt_sha256.as_deref()
    }

    #[must_use]
    pub fn phase_report_sha256(&self) -> Option<&str> {
        self.phase_report_sha256.as_deref()
    }

    #[must_use]
    pub fn operator_shadow_receipt_sha256(&self) -> Option<&str> {
        self.operator_shadow_receipt_sha256.as_deref()
    }

    #[must_use]
    pub const fn elapsed_nanos(&self) -> u64 {
        self.elapsed_nanos
    }

    #[must_use]
    pub const fn raw_payloads_persisted(&self) -> u8 {
        self.raw_payloads_persisted
    }

    #[must_use]
    pub const fn local_accepts(&self) -> u8 {
        self.local_accepts
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        self.execution_authority
    }
}

fn receipt_digest(
    builder: &TrafficShadowReceiptBuilderV3,
    verdict: TrafficShadowVerdictV3,
) -> String {
    let mut bytes = b"nando.traffic-shadow-receipt.v3".to_vec();
    push(&mut bytes, &builder.window_row_sha256);
    push(&mut bytes, &builder.request_sha256);
    bytes.extend_from_slice(&builder.generation_sequence.to_le_bytes());
    push(&mut bytes, &builder.generation_root_sha256);
    push(&mut bytes, &builder.index_sha256);
    bytes.push(projection_tag(builder.projection));
    bytes.push(streaming_tag(builder.streaming));
    bytes.push(builder.source as u8);
    bytes.push(verdict as u8);
    push_optional(&mut bytes, builder.extraction_receipt_sha256.as_deref());
    push_optional(&mut bytes, builder.phase_report_sha256.as_deref());
    push_optional(
        &mut bytes,
        builder.operator_shadow_receipt_sha256.as_deref(),
    );
    sha256_bytes(&bytes)
}

fn push(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_optional(bytes: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            bytes.push(1);
            push(bytes, value);
        }
        None => bytes.push(0),
    }
}

const fn projection_tag(projection: Option<RuntimeProjectionV3>) -> u8 {
    match projection {
        None => 0,
        Some(RuntimeProjectionV3::Responses) => 1,
        Some(RuntimeProjectionV3::ChatCompletions) => 2,
        Some(RuntimeProjectionV3::TransitionApi) => 3,
    }
}

const fn streaming_tag(streaming: Option<bool>) -> u8 {
    match streaming {
        None => 0,
        Some(false) => 1,
        Some(true) => 2,
    }
}
