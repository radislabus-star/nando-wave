use serde::{Deserialize, Serialize};

use nando_operator_kernel::valid_nonzero_sha256;

pub const RUNTIME_PACKAGE_REVOCATION_LEDGER_SCHEMA_V1: &str =
    "nando.runtime-package-revocation-ledger.v1";
pub const MAX_RUNTIME_PACKAGE_REVOCATIONS: usize = 4_096;

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePackageRevocationV1 {
    pub package_id: String,
    pub execution_payload_sha256: String,
    pub request_sha256: String,
    pub observed_at_unix: u64,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePackageRevocationLedgerV1 {
    pub schema: String,
    pub revocations: Vec<RuntimePackageRevocationV1>,
}

impl Default for RuntimePackageRevocationLedgerV1 {
    fn default() -> Self {
        Self {
            schema: RUNTIME_PACKAGE_REVOCATION_LEDGER_SCHEMA_V1.to_owned(),
            revocations: Vec::new(),
        }
    }
}

impl RuntimePackageRevocationLedgerV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != RUNTIME_PACKAGE_REVOCATION_LEDGER_SCHEMA_V1 {
            return Err("runtime_package_revocation_schema_invalid");
        }
        if self.revocations.len() > MAX_RUNTIME_PACKAGE_REVOCATIONS {
            return Err("runtime_package_revocation_ledger_full");
        }
        if self
            .revocations
            .iter()
            .any(|revocation| revocation.validate().is_err())
        {
            return Err("runtime_package_revocation_invalid");
        }
        if !self.revocations.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err("runtime_package_revocation_order_invalid");
        }
        if self.revocations.windows(2).any(|pair| {
            pair[0].package_id == pair[1].package_id
                && pair[0].execution_payload_sha256 == pair[1].execution_payload_sha256
        }) {
            return Err("runtime_package_revocation_duplicate_identity");
        }
        Ok(())
    }

    pub fn record(&mut self, revocation: RuntimePackageRevocationV1) -> Result<bool, &'static str> {
        revocation.validate()?;
        if self.revokes(&revocation.package_id, &revocation.execution_payload_sha256) {
            return Ok(false);
        }
        if self.revocations.len() >= MAX_RUNTIME_PACKAGE_REVOCATIONS {
            return Err("runtime_package_revocation_ledger_full");
        }
        let position = self
            .revocations
            .binary_search(&revocation)
            .unwrap_or_else(|position| position);
        self.revocations.insert(position, revocation);
        Ok(true)
    }

    #[must_use]
    pub fn revokes(&self, package_id: &str, execution_payload_sha256: &str) -> bool {
        self.revocations.iter().any(|revocation| {
            revocation.package_id == package_id
                && revocation.execution_payload_sha256 == execution_payload_sha256
        })
    }
}

impl RuntimePackageRevocationV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.package_id.is_empty() || self.package_id.len() > 256 {
            return Err("runtime_package_revocation_package_invalid");
        }
        if !valid_nonzero_sha256(&self.execution_payload_sha256)
            || !valid_nonzero_sha256(&self.request_sha256)
        {
            return Err("runtime_package_revocation_digest_invalid");
        }
        if self.reason.len() > 96
            || !self
                .reason
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err("runtime_package_revocation_reason_invalid");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revocation_is_execution_identity_specific_and_deterministic() {
        let mut ledger = RuntimePackageRevocationLedgerV1::default();
        let revocation = RuntimePackageRevocationV1 {
            package_id: "package-a".to_owned(),
            execution_payload_sha256: "11".repeat(32),
            request_sha256: "22".repeat(32),
            observed_at_unix: 7,
            reason: "user_correction".to_owned(),
        };
        assert_eq!(ledger.record(revocation.clone()), Ok(true));
        assert_eq!(ledger.record(revocation), Ok(false));
        assert!(ledger.revokes("package-a", &"11".repeat(32)));
        assert!(!ledger.revokes("package-a", &"33".repeat(32)));
        assert_eq!(ledger.validate(), Ok(()));
    }
}
