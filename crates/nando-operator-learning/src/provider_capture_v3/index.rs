use std::collections::BTreeMap;

use nando_operator_kernel::Sha256CommitmentV3;

use super::{
    ProviderCaptureSequenceLeaseCommitmentV3, ProviderRequestCaptureReceiptV3,
    index_codec::{canonical_record_bytes, decode_index, encode_index, index_digest},
    index_validation::{validate_leases, validate_records},
};

pub const PROVIDER_CAPTURE_INDEX_SCHEMA_V3: &str = "nando.provider-capture-index.v3.f8a";
pub const PROVIDER_CAPTURE_INDEX_MAX_RECORDS_V3: usize = 16_384;
pub const PROVIDER_CAPTURE_INDEX_MAX_BYTES_V3: usize = 8 * 1024 * 1024;
pub const PROVIDER_CAPTURE_SEQUENCE_LEASE_SIZE_V3: u64 =
    PROVIDER_CAPTURE_INDEX_MAX_RECORDS_V3 as u64;

pub(super) const INDEX_DOMAIN_V3: &[u8] = b"nando.provider-capture-index.v3.f8a";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderCaptureIndexV3 {
    publish_sequence: u64,
    reserved_through_sequence: u64,
    leases: Vec<ProviderCaptureSequenceLeaseCommitmentV3>,
    records: Vec<ProviderRequestCaptureReceiptV3>,
    index_sha256: Sha256CommitmentV3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderCaptureIndexErrorV3 {
    InvalidIndex,
    InvalidReceipt,
    DuplicateCommitment,
    NonMonotonicPublish,
    SequenceOutsideLease,
    EvidenceRollback,
    BudgetExhausted,
    Serialization,
}

impl ProviderCaptureIndexV3 {
    pub fn empty() -> Result<Self, ProviderCaptureIndexErrorV3> {
        Self::build(0, 0, Vec::new(), Vec::new())
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, ProviderCaptureIndexErrorV3> {
        let decoded = decode_index(bytes)?;
        let index = Self {
            publish_sequence: decoded.publish_sequence,
            reserved_through_sequence: decoded.reserved_through_sequence,
            leases: decoded.leases,
            records: decoded.records,
            index_sha256: decoded.index_sha256,
        };
        index.validate()?;
        if index.canonical_bytes()?.as_ref() != bytes {
            return Err(ProviderCaptureIndexErrorV3::InvalidIndex);
        }
        Ok(index)
    }

    pub fn canonical_bytes(&self) -> Result<Box<[u8]>, ProviderCaptureIndexErrorV3> {
        self.validate()?;
        let records = canonical_record_bytes(&self.records)?;
        encode_index(
            self.publish_sequence,
            self.reserved_through_sequence,
            &self.leases,
            &records,
            self.index_sha256,
        )
    }

    pub fn reserve_next_lease(
        &self,
    ) -> Result<(Self, ProviderCaptureSequenceLeaseCommitmentV3), ProviderCaptureIndexErrorV3> {
        let lease =
            ProviderCaptureSequenceLeaseCommitmentV3::next_after(self.reserved_through_sequence)?;
        let mut leases = self.leases.clone();
        leases.push(lease);
        let next = Self::build(
            self.next_publish_sequence()?,
            lease.last_sequence(),
            leases,
            self.records.clone(),
        )?;
        Ok((next, lease))
    }

    pub fn append_batch(
        &self,
        additions: &[ProviderRequestCaptureReceiptV3],
    ) -> Result<Self, ProviderCaptureIndexErrorV3> {
        if additions.is_empty() {
            return Err(ProviderCaptureIndexErrorV3::InvalidIndex);
        }
        let mut records = self.records.clone();
        records.extend_from_slice(additions);
        records.sort_by_key(ProviderRequestCaptureReceiptV3::capture_sequence);
        Self::build(
            self.next_publish_sequence()?,
            self.reserved_through_sequence,
            self.leases.clone(),
            records,
        )
    }

    pub fn validate_transition_from(
        &self,
        previous: &Self,
    ) -> Result<(), ProviderCaptureIndexErrorV3> {
        if previous.publish_sequence.checked_add(1) != Some(self.publish_sequence) {
            return Err(ProviderCaptureIndexErrorV3::NonMonotonicPublish);
        }
        if self.reserved_through_sequence < previous.reserved_through_sequence
            || !self.leases.starts_with(&previous.leases)
        {
            return Err(ProviderCaptureIndexErrorV3::EvidenceRollback);
        }
        let current = self
            .records
            .iter()
            .map(|record| (record.capture_sequence(), record.receipt_sha256()))
            .collect::<BTreeMap<_, _>>();
        if previous
            .records
            .iter()
            .any(|record| current.get(&record.capture_sequence()) != Some(&record.receipt_sha256()))
            || (self.leases.len() == previous.leases.len()
                && self.records.len() == previous.records.len())
        {
            return Err(ProviderCaptureIndexErrorV3::EvidenceRollback);
        }
        Ok(())
    }

    #[must_use]
    pub fn contains_request_root(&self, root: Sha256CommitmentV3) -> bool {
        self.records
            .iter()
            .any(|record| record.request_root_sha256() == root)
    }

    #[must_use]
    pub fn contains_event_root(&self, root: Sha256CommitmentV3) -> bool {
        self.records
            .iter()
            .any(|record| record.event_root_sha256() == root)
    }

    #[must_use]
    pub fn contains_receipt_root(&self, root: Sha256CommitmentV3) -> bool {
        self.records
            .iter()
            .any(|record| record.receipt_sha256() == root)
    }

    #[must_use]
    pub fn contains_exact(
        &self,
        capture_sequence: u64,
        event_root_sha256: Sha256CommitmentV3,
        request_root_sha256: Sha256CommitmentV3,
        receipt_sha256: Sha256CommitmentV3,
    ) -> bool {
        self.find_exact(
            capture_sequence,
            event_root_sha256,
            request_root_sha256,
            receipt_sha256,
        )
        .is_some()
    }

    #[must_use]
    pub fn find_exact(
        &self,
        capture_sequence: u64,
        event_root_sha256: Sha256CommitmentV3,
        request_root_sha256: Sha256CommitmentV3,
        receipt_sha256: Sha256CommitmentV3,
    ) -> Option<&ProviderRequestCaptureReceiptV3> {
        self.records
            .binary_search_by_key(&capture_sequence, |record| record.capture_sequence())
            .ok()
            .and_then(|index| self.records.get(index))
            .filter(|record| {
                record.event_root_sha256() == event_root_sha256
                    && record.request_root_sha256() == request_root_sha256
                    && record.receipt_sha256() == receipt_sha256
            })
    }

    #[must_use]
    pub const fn publish_sequence(&self) -> u64 {
        self.publish_sequence
    }

    #[must_use]
    pub const fn reserved_through_sequence(&self) -> u64 {
        self.reserved_through_sequence
    }

    #[must_use]
    pub fn leases(&self) -> &[ProviderCaptureSequenceLeaseCommitmentV3] {
        &self.leases
    }

    #[must_use]
    pub fn records(&self) -> &[ProviderRequestCaptureReceiptV3] {
        &self.records
    }

    #[must_use]
    pub const fn index_sha256(&self) -> Sha256CommitmentV3 {
        self.index_sha256
    }

    #[must_use]
    pub const fn raw_payloads_persisted(&self) -> u8 {
        0
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }

    fn build(
        publish_sequence: u64,
        reserved_through_sequence: u64,
        leases: Vec<ProviderCaptureSequenceLeaseCommitmentV3>,
        records: Vec<ProviderRequestCaptureReceiptV3>,
    ) -> Result<Self, ProviderCaptureIndexErrorV3> {
        validate_leases(&leases, reserved_through_sequence)?;
        validate_records(&records, &leases)?;
        let record_bytes = canonical_record_bytes(&records)?;
        let index_sha256 = index_digest(
            publish_sequence,
            reserved_through_sequence,
            &leases,
            &record_bytes,
        )?;
        let index = Self {
            publish_sequence,
            reserved_through_sequence,
            leases,
            records,
            index_sha256,
        };
        index.canonical_bytes_unchecked(&record_bytes)?;
        Ok(index)
    }

    fn validate(&self) -> Result<(), ProviderCaptureIndexErrorV3> {
        validate_leases(&self.leases, self.reserved_through_sequence)?;
        validate_records(&self.records, &self.leases)?;
        let records = canonical_record_bytes(&self.records)?;
        if index_digest(
            self.publish_sequence,
            self.reserved_through_sequence,
            &self.leases,
            &records,
        )? != self.index_sha256
        {
            return Err(ProviderCaptureIndexErrorV3::InvalidIndex);
        }
        Ok(())
    }

    fn canonical_bytes_unchecked(
        &self,
        records: &[serde_bytes::ByteBuf],
    ) -> Result<Box<[u8]>, ProviderCaptureIndexErrorV3> {
        encode_index(
            self.publish_sequence,
            self.reserved_through_sequence,
            &self.leases,
            records,
            self.index_sha256,
        )
    }

    fn next_publish_sequence(&self) -> Result<u64, ProviderCaptureIndexErrorV3> {
        self.publish_sequence
            .checked_add(1)
            .ok_or(ProviderCaptureIndexErrorV3::BudgetExhausted)
    }
}
