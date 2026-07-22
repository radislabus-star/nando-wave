use nando_operator_kernel::{RuntimeProjectionV3, Sha256CommitmentV3};
use serde::{Deserialize, Serialize};

pub const PROVIDER_REQUEST_CAPTURE_RECEIPT_SCHEMA_V3: &str =
    "nando.provider-request-capture-receipt.v3.f8a";
pub const PROVIDER_REQUEST_CAPTURE_RECEIPT_MAX_BYTES_V3: usize = 1_024;

const EVENT_DOMAIN_V3: &[u8] = b"nando.provider-request-capture-event.v3.f8a";
const RECEIPT_DOMAIN_V3: &[u8] = b"nando.provider-request-capture-receipt.v3.f8a";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderRequestCaptureReceiptV3 {
    capture_sequence: u64,
    capture_epoch_root: Sha256CommitmentV3,
    lineage_root_sha256: Sha256CommitmentV3,
    event_root_sha256: Sha256CommitmentV3,
    request_root_sha256: Sha256CommitmentV3,
    projection: RuntimeProjectionV3,
    streaming: bool,
    observed_at_unix_ms: u64,
    receipt_sha256: Sha256CommitmentV3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderRequestCaptureInputV3 {
    pub capture_sequence: u64,
    pub capture_epoch_root: Sha256CommitmentV3,
    pub lineage_root_sha256: Sha256CommitmentV3,
    pub request_root_sha256: Sha256CommitmentV3,
    pub projection: RuntimeProjectionV3,
    pub streaming: bool,
    pub observed_at_unix_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderRequestCaptureErrorV3 {
    InvalidInput,
    InvalidReceipt,
    BudgetExhausted,
    Serialization,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct ProviderRequestCaptureReceiptWireV3(
    String,
    u64,
    Sha256CommitmentV3,
    Sha256CommitmentV3,
    Sha256CommitmentV3,
    Sha256CommitmentV3,
    RuntimeProjectionV3,
    bool,
    u64,
    Sha256CommitmentV3,
);

pub fn seal_provider_request_capture_v3(
    input: ProviderRequestCaptureInputV3,
) -> Result<ProviderRequestCaptureReceiptV3, ProviderRequestCaptureErrorV3> {
    if input.capture_sequence == 0 || input.observed_at_unix_ms == 0 {
        return Err(ProviderRequestCaptureErrorV3::InvalidInput);
    }
    let capture_sequence = input.capture_sequence.to_be_bytes();
    let observed_at = input.observed_at_unix_ms.to_be_bytes();
    let projection = [projection_tag(input.projection)];
    let streaming = [u8::from(input.streaming)];
    let event_root_sha256 = Sha256CommitmentV3::digest_parts(
        EVENT_DOMAIN_V3,
        &[
            &capture_sequence,
            input.capture_epoch_root.as_bytes(),
            input.lineage_root_sha256.as_bytes(),
            input.request_root_sha256.as_bytes(),
            &projection,
            &streaming,
            &observed_at,
        ],
    );
    let receipt_sha256 = receipt_digest(
        input.capture_sequence,
        input.capture_epoch_root,
        input.lineage_root_sha256,
        event_root_sha256,
        input.request_root_sha256,
        input.projection,
        input.streaming,
        input.observed_at_unix_ms,
    );
    let receipt = ProviderRequestCaptureReceiptV3 {
        capture_sequence: input.capture_sequence,
        capture_epoch_root: input.capture_epoch_root,
        lineage_root_sha256: input.lineage_root_sha256,
        event_root_sha256,
        request_root_sha256: input.request_root_sha256,
        projection: input.projection,
        streaming: input.streaming,
        observed_at_unix_ms: input.observed_at_unix_ms,
        receipt_sha256,
    };
    receipt.validate()?;
    Ok(receipt)
}

impl ProviderRequestCaptureReceiptV3 {
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, ProviderRequestCaptureErrorV3> {
        if bytes.len() > PROVIDER_REQUEST_CAPTURE_RECEIPT_MAX_BYTES_V3 {
            return Err(ProviderRequestCaptureErrorV3::BudgetExhausted);
        }
        let wire: ProviderRequestCaptureReceiptWireV3 = serde_cbor::from_slice(bytes)
            .map_err(|_| ProviderRequestCaptureErrorV3::InvalidReceipt)?;
        let receipt = Self::from_wire(wire)?;
        if receipt.canonical_bytes()?.as_ref() != bytes {
            return Err(ProviderRequestCaptureErrorV3::InvalidReceipt);
        }
        Ok(receipt)
    }

    pub fn canonical_bytes(&self) -> Result<Box<[u8]>, ProviderRequestCaptureErrorV3> {
        self.validate()?;
        let bytes = serde_cbor::to_vec(&self.to_wire())
            .map_err(|_| ProviderRequestCaptureErrorV3::Serialization)?;
        if bytes.len() > PROVIDER_REQUEST_CAPTURE_RECEIPT_MAX_BYTES_V3 {
            return Err(ProviderRequestCaptureErrorV3::BudgetExhausted);
        }
        Ok(bytes.into_boxed_slice())
    }

    pub(crate) fn validate(&self) -> Result<(), ProviderRequestCaptureErrorV3> {
        if self.capture_sequence == 0 || self.observed_at_unix_ms == 0 {
            return Err(ProviderRequestCaptureErrorV3::InvalidReceipt);
        }
        let expected_event = event_digest(self);
        let expected_receipt = receipt_digest(
            self.capture_sequence,
            self.capture_epoch_root,
            self.lineage_root_sha256,
            self.event_root_sha256,
            self.request_root_sha256,
            self.projection,
            self.streaming,
            self.observed_at_unix_ms,
        );
        if expected_event != self.event_root_sha256 || expected_receipt != self.receipt_sha256 {
            return Err(ProviderRequestCaptureErrorV3::InvalidReceipt);
        }
        Ok(())
    }

    #[must_use]
    pub const fn capture_sequence(&self) -> u64 {
        self.capture_sequence
    }

    #[must_use]
    pub const fn capture_epoch_root(&self) -> Sha256CommitmentV3 {
        self.capture_epoch_root
    }

    #[must_use]
    pub const fn lineage_root_sha256(&self) -> Sha256CommitmentV3 {
        self.lineage_root_sha256
    }

    #[must_use]
    pub const fn event_root_sha256(&self) -> Sha256CommitmentV3 {
        self.event_root_sha256
    }

    #[must_use]
    pub const fn request_root_sha256(&self) -> Sha256CommitmentV3 {
        self.request_root_sha256
    }

    #[must_use]
    pub const fn projection(&self) -> RuntimeProjectionV3 {
        self.projection
    }

    #[must_use]
    pub const fn streaming(&self) -> bool {
        self.streaming
    }

    #[must_use]
    pub const fn observed_at_unix_ms(&self) -> u64 {
        self.observed_at_unix_ms
    }

    #[must_use]
    pub const fn receipt_sha256(&self) -> Sha256CommitmentV3 {
        self.receipt_sha256
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }

    fn to_wire(&self) -> ProviderRequestCaptureReceiptWireV3 {
        ProviderRequestCaptureReceiptWireV3(
            PROVIDER_REQUEST_CAPTURE_RECEIPT_SCHEMA_V3.to_owned(),
            self.capture_sequence,
            self.capture_epoch_root,
            self.lineage_root_sha256,
            self.event_root_sha256,
            self.request_root_sha256,
            self.projection,
            self.streaming,
            self.observed_at_unix_ms,
            self.receipt_sha256,
        )
    }

    fn from_wire(
        wire: ProviderRequestCaptureReceiptWireV3,
    ) -> Result<Self, ProviderRequestCaptureErrorV3> {
        if wire.0 != PROVIDER_REQUEST_CAPTURE_RECEIPT_SCHEMA_V3 {
            return Err(ProviderRequestCaptureErrorV3::InvalidReceipt);
        }
        let receipt = Self {
            capture_sequence: wire.1,
            capture_epoch_root: wire.2,
            lineage_root_sha256: wire.3,
            event_root_sha256: wire.4,
            request_root_sha256: wire.5,
            projection: wire.6,
            streaming: wire.7,
            observed_at_unix_ms: wire.8,
            receipt_sha256: wire.9,
        };
        receipt.validate()?;
        Ok(receipt)
    }
}

fn event_digest(receipt: &ProviderRequestCaptureReceiptV3) -> Sha256CommitmentV3 {
    let sequence = receipt.capture_sequence.to_be_bytes();
    let observed_at = receipt.observed_at_unix_ms.to_be_bytes();
    let projection = [projection_tag(receipt.projection)];
    let streaming = [u8::from(receipt.streaming)];
    Sha256CommitmentV3::digest_parts(
        EVENT_DOMAIN_V3,
        &[
            &sequence,
            receipt.capture_epoch_root.as_bytes(),
            receipt.lineage_root_sha256.as_bytes(),
            receipt.request_root_sha256.as_bytes(),
            &projection,
            &streaming,
            &observed_at,
        ],
    )
}

#[allow(clippy::too_many_arguments)]
fn receipt_digest(
    capture_sequence: u64,
    capture_epoch_root: Sha256CommitmentV3,
    lineage_root_sha256: Sha256CommitmentV3,
    event_root_sha256: Sha256CommitmentV3,
    request_root_sha256: Sha256CommitmentV3,
    projection: RuntimeProjectionV3,
    streaming: bool,
    observed_at_unix_ms: u64,
) -> Sha256CommitmentV3 {
    let sequence = capture_sequence.to_be_bytes();
    let observed_at = observed_at_unix_ms.to_be_bytes();
    let projection = [projection_tag(projection)];
    let streaming = [u8::from(streaming)];
    Sha256CommitmentV3::digest_parts(
        RECEIPT_DOMAIN_V3,
        &[
            &sequence,
            capture_epoch_root.as_bytes(),
            lineage_root_sha256.as_bytes(),
            event_root_sha256.as_bytes(),
            request_root_sha256.as_bytes(),
            &projection,
            &streaming,
            &observed_at,
        ],
    )
}

const fn projection_tag(projection: RuntimeProjectionV3) -> u8 {
    match projection {
        RuntimeProjectionV3::Responses => 1,
        RuntimeProjectionV3::ChatCompletions => 2,
        RuntimeProjectionV3::TransitionApi => 3,
    }
}

pub(super) fn receipt_to_wire_v3(
    receipt: &ProviderRequestCaptureReceiptV3,
) -> Result<Vec<u8>, ProviderRequestCaptureErrorV3> {
    receipt.canonical_bytes().map(Into::into)
}

pub(super) fn receipt_from_wire_v3(
    bytes: &[u8],
) -> Result<ProviderRequestCaptureReceiptV3, ProviderRequestCaptureErrorV3> {
    ProviderRequestCaptureReceiptV3::from_canonical_bytes(bytes)
}
