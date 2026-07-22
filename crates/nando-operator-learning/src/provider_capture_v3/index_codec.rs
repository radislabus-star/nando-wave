use nando_operator_kernel::Sha256CommitmentV3;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

use super::index::INDEX_DOMAIN_V3;
use super::{
    PROVIDER_CAPTURE_INDEX_MAX_BYTES_V3, PROVIDER_CAPTURE_INDEX_SCHEMA_V3,
    ProviderCaptureIndexErrorV3, ProviderCaptureSequenceLeaseCommitmentV3,
    ProviderRequestCaptureErrorV3, ProviderRequestCaptureReceiptV3, receipt_from_wire_v3,
    receipt_to_wire_v3,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ProviderCaptureIndexWireV3 {
    schema: String,
    publish_sequence: u64,
    reserved_through_sequence: u64,
    leases: Vec<ProviderCaptureSequenceLeaseCommitmentV3>,
    records: Vec<ByteBuf>,
    index_sha256: Sha256CommitmentV3,
    raw_payloads_persisted: u8,
    execution_authority: bool,
}

#[derive(Serialize)]
struct ProviderCaptureIndexDigestV3<'a> {
    schema: &'static str,
    publish_sequence: u64,
    reserved_through_sequence: u64,
    leases: &'a [ProviderCaptureSequenceLeaseCommitmentV3],
    records: &'a [ByteBuf],
    raw_payloads_persisted: u8,
    execution_authority: bool,
}

pub(super) struct DecodedProviderCaptureIndexV3 {
    pub(super) publish_sequence: u64,
    pub(super) reserved_through_sequence: u64,
    pub(super) leases: Vec<ProviderCaptureSequenceLeaseCommitmentV3>,
    pub(super) records: Vec<ProviderRequestCaptureReceiptV3>,
    pub(super) index_sha256: Sha256CommitmentV3,
}

pub(super) fn decode_index(
    bytes: &[u8],
) -> Result<DecodedProviderCaptureIndexV3, ProviderCaptureIndexErrorV3> {
    if bytes.len() > PROVIDER_CAPTURE_INDEX_MAX_BYTES_V3 {
        return Err(ProviderCaptureIndexErrorV3::BudgetExhausted);
    }
    let wire: ProviderCaptureIndexWireV3 =
        serde_cbor::from_slice(bytes).map_err(|_| ProviderCaptureIndexErrorV3::InvalidIndex)?;
    if wire.schema != PROVIDER_CAPTURE_INDEX_SCHEMA_V3
        || wire.raw_payloads_persisted != 0
        || wire.execution_authority
    {
        return Err(ProviderCaptureIndexErrorV3::InvalidIndex);
    }
    let records = wire
        .records
        .iter()
        .map(|record| {
            receipt_from_wire_v3(record.as_ref())
                .map_err(|_| ProviderCaptureIndexErrorV3::InvalidReceipt)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(DecodedProviderCaptureIndexV3 {
        publish_sequence: wire.publish_sequence,
        reserved_through_sequence: wire.reserved_through_sequence,
        leases: wire.leases,
        records,
        index_sha256: wire.index_sha256,
    })
}

pub(super) fn encode_index(
    publish_sequence: u64,
    reserved_through_sequence: u64,
    leases: &[ProviderCaptureSequenceLeaseCommitmentV3],
    records: &[ByteBuf],
    index_sha256: Sha256CommitmentV3,
) -> Result<Box<[u8]>, ProviderCaptureIndexErrorV3> {
    let bytes = serde_cbor::to_vec(&ProviderCaptureIndexWireV3 {
        schema: PROVIDER_CAPTURE_INDEX_SCHEMA_V3.to_owned(),
        publish_sequence,
        reserved_through_sequence,
        leases: leases.to_vec(),
        records: records.to_vec(),
        index_sha256,
        raw_payloads_persisted: 0,
        execution_authority: false,
    })
    .map_err(|_| ProviderCaptureIndexErrorV3::Serialization)?;
    if bytes.len() > PROVIDER_CAPTURE_INDEX_MAX_BYTES_V3 {
        return Err(ProviderCaptureIndexErrorV3::BudgetExhausted);
    }
    Ok(bytes.into_boxed_slice())
}

pub(super) fn canonical_record_bytes(
    records: &[ProviderRequestCaptureReceiptV3],
) -> Result<Vec<ByteBuf>, ProviderCaptureIndexErrorV3> {
    records
        .iter()
        .map(|record| {
            receipt_to_wire_v3(record)
                .map(ByteBuf::from)
                .map_err(map_receipt_error)
        })
        .collect()
}

pub(super) fn index_digest(
    publish_sequence: u64,
    reserved_through_sequence: u64,
    leases: &[ProviderCaptureSequenceLeaseCommitmentV3],
    records: &[ByteBuf],
) -> Result<Sha256CommitmentV3, ProviderCaptureIndexErrorV3> {
    let bytes = serde_cbor::to_vec(&ProviderCaptureIndexDigestV3 {
        schema: PROVIDER_CAPTURE_INDEX_SCHEMA_V3,
        publish_sequence,
        reserved_through_sequence,
        leases,
        records,
        raw_payloads_persisted: 0,
        execution_authority: false,
    })
    .map_err(|_| ProviderCaptureIndexErrorV3::Serialization)?;
    Ok(Sha256CommitmentV3::digest_parts(INDEX_DOMAIN_V3, &[&bytes]))
}

fn map_receipt_error(error: ProviderRequestCaptureErrorV3) -> ProviderCaptureIndexErrorV3 {
    match error {
        ProviderRequestCaptureErrorV3::BudgetExhausted => {
            ProviderCaptureIndexErrorV3::BudgetExhausted
        }
        ProviderRequestCaptureErrorV3::InvalidInput
        | ProviderRequestCaptureErrorV3::InvalidReceipt => {
            ProviderCaptureIndexErrorV3::InvalidReceipt
        }
        ProviderRequestCaptureErrorV3::Serialization => ProviderCaptureIndexErrorV3::Serialization,
    }
}
