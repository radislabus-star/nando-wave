use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

pub const EXACT_ATTEMPT_INDEX_SCHEMA_V1: &str = "nando.k1-exact-attempt-index.v1";

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExactAttemptRecordV1 {
    pub opportunity_root_sha256: String,
    pub identifier_result_root_sha256: String,
    pub terminal_diagnostic_root_sha256: String,
    pub candidate_freeze_root_sha256: String,
    pub generation_sequence: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExactAttemptIndexV1 {
    pub schema: String,
    pub index_root_sha256: String,
    pub deterministic_attempts: Vec<ExactAttemptRecordV1>,
    pub legacy_unbound_terminals: u64,
    pub authority_ready: bool,
}

impl ExactAttemptIndexV1 {
    pub fn seal(
        mut deterministic_attempts: Vec<ExactAttemptRecordV1>,
        legacy_unbound_terminals: u64,
    ) -> Result<Self, &'static str> {
        deterministic_attempts.sort();
        let mut index = Self {
            schema: EXACT_ATTEMPT_INDEX_SCHEMA_V1.to_owned(),
            index_root_sha256: String::new(),
            deterministic_attempts,
            legacy_unbound_terminals,
            authority_ready: false,
        };
        index.index_root_sha256 = index.expected_root()?;
        index.validate()?;
        Ok(index)
    }

    pub fn empty(legacy_unbound_terminals: u64) -> Result<Self, &'static str> {
        Self::seal(Vec::new(), legacy_unbound_terminals)
    }

    pub fn contains(&self, opportunity_root_sha256: &str) -> bool {
        self.deterministic_attempts
            .binary_search_by(|record| {
                record
                    .opportunity_root_sha256
                    .as_str()
                    .cmp(opportunity_root_sha256)
            })
            .is_ok()
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != EXACT_ATTEMPT_INDEX_SCHEMA_V1
            || self.authority_ready
            || self.deterministic_attempts.iter().any(|record| {
                record.generation_sequence == 0
                    || [
                        record.opportunity_root_sha256.as_str(),
                        record.identifier_result_root_sha256.as_str(),
                        record.terminal_diagnostic_root_sha256.as_str(),
                        record.candidate_freeze_root_sha256.as_str(),
                    ]
                    .into_iter()
                    .any(|root| !valid_nonzero_sha256(root))
            })
            || !self
                .deterministic_attempts
                .windows(2)
                .all(|pair| pair[0].opportunity_root_sha256 < pair[1].opportunity_root_sha256)
            || self.index_root_sha256 != self.expected_root()?
        {
            return Err("exact_attempt_index_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            EXACT_ATTEMPT_INDEX_SCHEMA_V1,
            &self.deterministic_attempts,
            self.legacy_unbound_terminals,
            false,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(value: u64) -> String {
        format!("{value:064x}")
    }

    #[test]
    fn legacy_terminals_never_create_exact_attempts() {
        let index = ExactAttemptIndexV1::empty(585).expect("empty index");
        assert!(index.deterministic_attempts.is_empty());
        assert_eq!(index.legacy_unbound_terminals, 585);
        assert!(!index.contains(&root(1)));
    }

    #[test]
    fn exact_records_are_canonical_and_unique() {
        let record = |value| ExactAttemptRecordV1 {
            opportunity_root_sha256: root(value),
            identifier_result_root_sha256: root(value + 10),
            terminal_diagnostic_root_sha256: root(value + 20),
            candidate_freeze_root_sha256: root(value + 30),
            generation_sequence: value,
        };
        let index = ExactAttemptIndexV1::seal(vec![record(2), record(1)], 0).expect("index");
        assert!(index.contains(&root(1)));
        assert_eq!(index.deterministic_attempts[0].generation_sequence, 1);
        assert_eq!(
            ExactAttemptIndexV1::seal(vec![record(1), record(1)], 0),
            Err("exact_attempt_index_invalid")
        );
    }
}
