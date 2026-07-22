use nando_operator_kernel::Sha256CommitmentV3;
use serde::{Deserialize, Serialize};

use super::{PROVIDER_CAPTURE_SEQUENCE_LEASE_SIZE_V3, ProviderCaptureIndexErrorV3};

const LEASE_DOMAIN_V3: &[u8] = b"nando.provider-capture-sequence-lease.v3.f8a";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderCaptureSequenceLeaseCommitmentV3(u64, u64, Sha256CommitmentV3);

impl ProviderCaptureSequenceLeaseCommitmentV3 {
    pub(super) fn next_after(
        reserved_through_sequence: u64,
    ) -> Result<Self, ProviderCaptureIndexErrorV3> {
        let first_sequence = reserved_through_sequence
            .checked_add(1)
            .ok_or(ProviderCaptureIndexErrorV3::BudgetExhausted)?;
        let last_sequence = first_sequence
            .checked_add(PROVIDER_CAPTURE_SEQUENCE_LEASE_SIZE_V3 - 1)
            .ok_or(ProviderCaptureIndexErrorV3::BudgetExhausted)?;
        Ok(Self(
            first_sequence,
            last_sequence,
            epoch_root(first_sequence, last_sequence),
        ))
    }

    pub(super) fn validate(self) -> Result<(), ProviderCaptureIndexErrorV3> {
        if self.0 == 0
            || self
                .1
                .checked_sub(self.0)
                .and_then(|distance| distance.checked_add(1))
                != Some(PROVIDER_CAPTURE_SEQUENCE_LEASE_SIZE_V3)
            || self.2 != epoch_root(self.0, self.1)
        {
            return Err(ProviderCaptureIndexErrorV3::InvalidIndex);
        }
        Ok(())
    }

    #[must_use]
    pub const fn first_sequence(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn last_sequence(self) -> u64 {
        self.1
    }

    #[must_use]
    pub const fn epoch_root_sha256(self) -> Sha256CommitmentV3 {
        self.2
    }
}

fn epoch_root(first_sequence: u64, last_sequence: u64) -> Sha256CommitmentV3 {
    Sha256CommitmentV3::digest_parts(
        LEASE_DOMAIN_V3,
        &[&first_sequence.to_be_bytes(), &last_sequence.to_be_bytes()],
    )
}
