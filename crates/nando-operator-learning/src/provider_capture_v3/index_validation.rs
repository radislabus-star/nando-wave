use std::collections::BTreeSet;

use super::{
    PROVIDER_CAPTURE_INDEX_MAX_RECORDS_V3, ProviderCaptureIndexErrorV3,
    ProviderCaptureSequenceLeaseCommitmentV3, ProviderRequestCaptureReceiptV3,
};

pub(super) fn validate_leases(
    leases: &[ProviderCaptureSequenceLeaseCommitmentV3],
    reserved_through_sequence: u64,
) -> Result<(), ProviderCaptureIndexErrorV3> {
    if leases.is_empty() {
        return (reserved_through_sequence == 0)
            .then_some(())
            .ok_or(ProviderCaptureIndexErrorV3::InvalidIndex);
    }
    for lease in leases {
        lease.validate()?;
    }
    if leases[0].first_sequence() != 1
        || leases
            .windows(2)
            .any(|pair| pair[0].last_sequence().checked_add(1) != Some(pair[1].first_sequence()))
        || leases.last().map(|lease| lease.last_sequence()) != Some(reserved_through_sequence)
    {
        return Err(ProviderCaptureIndexErrorV3::InvalidIndex);
    }
    Ok(())
}

pub(super) fn validate_records(
    records: &[ProviderRequestCaptureReceiptV3],
    leases: &[ProviderCaptureSequenceLeaseCommitmentV3],
) -> Result<(), ProviderCaptureIndexErrorV3> {
    if records.len() > PROVIDER_CAPTURE_INDEX_MAX_RECORDS_V3 {
        return Err(ProviderCaptureIndexErrorV3::BudgetExhausted);
    }
    let mut sequences = BTreeSet::new();
    let mut events = BTreeSet::new();
    let mut requests = BTreeSet::new();
    let mut receipts = BTreeSet::new();
    for record in records {
        record
            .validate()
            .map_err(|_| ProviderCaptureIndexErrorV3::InvalidReceipt)?;
        let lease_index =
            leases.partition_point(|lease| lease.last_sequence() < record.capture_sequence());
        let Some(lease) = leases.get(lease_index) else {
            return Err(ProviderCaptureIndexErrorV3::SequenceOutsideLease);
        };
        if record.capture_sequence() < lease.first_sequence()
            || record.capture_epoch_root() != lease.epoch_root_sha256()
        {
            return Err(ProviderCaptureIndexErrorV3::SequenceOutsideLease);
        }
        if !sequences.insert(record.capture_sequence())
            || !events.insert(record.event_root_sha256())
            || !requests.insert(record.request_root_sha256())
            || !receipts.insert(record.receipt_sha256())
        {
            return Err(ProviderCaptureIndexErrorV3::DuplicateCommitment);
        }
    }
    if records
        .windows(2)
        .any(|pair| pair[0].capture_sequence() >= pair[1].capture_sequence())
    {
        return Err(ProviderCaptureIndexErrorV3::InvalidIndex);
    }
    Ok(())
}
