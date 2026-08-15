use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_sha256_bytes_v1, require_composition_root_v1, valid_composition_path_v1,
};
use super::{
    K2_UNCERTAINTY_ACTIONS_V1, K2_UNCERTAINTY_BUDGET_SCHEMA_V1, K2_UNCERTAINTY_CONFIRM_CASES_V1,
    K2_UNCERTAINTY_CONTENT_ATOM_SCHEMA_V1, K2_UNCERTAINTY_CONTENTS_V1,
    K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1, K2_UNCERTAINTY_MATCHED_PAIRS_V1,
    K2_UNCERTAINTY_MAX_BATCH_WALL_MS_V1, K2_UNCERTAINTY_MAX_CASE_WALL_MS_V1,
    K2_UNCERTAINTY_MAX_CONTENT_BYTES_V1, K2_UNCERTAINTY_MAX_COST_UNITS_V1,
    K2_UNCERTAINTY_MAX_MANIFEST_BYTES_V1, K2_UNCERTAINTY_MAX_MANIFEST_ENTRIES_V1,
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2_UNCERTAINTY_MAX_REPRESENTATIVES_V1,
    K2_UNCERTAINTY_MAX_RESIDENT_BYTES_V1, K2_UNCERTAINTY_MAX_RISK_UNITS_V1,
    K2_UNCERTAINTY_MAX_SELECTOR_REQUESTS_V1, K2_UNCERTAINTY_MIN_REPRESENTATIVES_V1,
    K2_UNCERTAINTY_PATH_ATOM_SCHEMA_V1, K2_UNCERTAINTY_PATHS_V1, K2_UNCERTAINTY_RAW_PROBES_V1,
    K2_UNCERTAINTY_SELECTOR_PROBES_V1, K2_UNCERTAINTY_STATE_COUNT_V1,
    K2_UNCERTAINTY_SUPPORT_ROWS_PER_ACTION_V1, K2_UNCERTAINTY_TOPOLOGY_FAMILIES_V1,
    K2_UNCERTAINTY_VOCABULARY_SCHEMA_V1, denied_authority_v1, require_denied_authority_v1,
    require_exact_len_v1, require_sorted_unique_v1, uncertainty_root_v1,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintySplitV1 {
    Development,
    Confirm,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyPathAtomV1 {
    pub schema: String,
    pub ordinal: u8,
    pub path: String,
    pub path_root_sha256: String,
}

impl K2UncertaintyPathAtomV1 {
    pub fn seal(ordinal: u8, path: String) -> K2CompositionResultV1<Self> {
        let path_root_sha256 =
            uncertainty_root_v1(&(K2_UNCERTAINTY_PATH_ATOM_SCHEMA_V1, ordinal, &path))?;
        let atom = Self {
            schema: K2_UNCERTAINTY_PATH_ATOM_SCHEMA_V1.to_owned(),
            ordinal,
            path,
            path_root_sha256,
        };
        atom.validate()?;
        Ok(atom)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        let expected =
            uncertainty_root_v1(&(K2_UNCERTAINTY_PATH_ATOM_SCHEMA_V1, self.ordinal, &self.path))?;
        if self.schema != K2_UNCERTAINTY_PATH_ATOM_SCHEMA_V1
            || usize::from(self.ordinal) >= K2_UNCERTAINTY_PATHS_V1
            || !valid_composition_path_v1(&self.path)
            || expected != self.path_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_path_atom_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyContentAtomV1 {
    pub schema: String,
    pub ordinal: u8,
    pub bytes: Vec<u8>,
    pub byte_len: u64,
    pub bytes_sha256: String,
    pub content_root_sha256: String,
}

impl K2UncertaintyContentAtomV1 {
    pub fn seal(ordinal: u8, bytes: Vec<u8>) -> K2CompositionResultV1<Self> {
        let byte_len = u64::try_from(bytes.len())
            .map_err(|_| K2CompositionErrorV1::Invalid("content_length_overflow"))?;
        let bytes_sha256 = composition_sha256_bytes_v1(&bytes);
        let content_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONTENT_ATOM_SCHEMA_V1,
            ordinal,
            &bytes,
            byte_len,
            &bytes_sha256,
        ))?;
        let atom = Self {
            schema: K2_UNCERTAINTY_CONTENT_ATOM_SCHEMA_V1.to_owned(),
            ordinal,
            bytes,
            byte_len,
            bytes_sha256,
            content_root_sha256,
        };
        atom.validate()?;
        Ok(atom)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        let expected_len = u64::try_from(self.bytes.len())
            .map_err(|_| K2CompositionErrorV1::Invalid("content_length_overflow"))?;
        let expected_sha = composition_sha256_bytes_v1(&self.bytes);
        let expected_root = uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONTENT_ATOM_SCHEMA_V1,
            self.ordinal,
            &self.bytes,
            self.byte_len,
            &self.bytes_sha256,
        ))?;
        if self.schema != K2_UNCERTAINTY_CONTENT_ATOM_SCHEMA_V1
            || usize::from(self.ordinal) >= K2_UNCERTAINTY_CONTENTS_V1
            || self.bytes.is_empty()
            || self.bytes.len() > K2_UNCERTAINTY_MAX_CONTENT_BYTES_V1
            || self.byte_len != expected_len
            || self.bytes_sha256 != expected_sha
            || self.content_root_sha256 != expected_root
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_content_atom_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyBudgetV1 {
    pub schema: String,
    pub action_count: u64,
    pub path_count: u64,
    pub content_count: u64,
    pub support_rows_per_action: u64,
    pub state_count: u64,
    pub raw_probe_count: u64,
    pub minimum_representatives: u64,
    pub maximum_representatives: u64,
    pub predecessor_request_probes: u64,
    pub maximum_selector_requests: u64,
    pub maximum_frontier_page_probes: u64,
    pub maximum_content_bytes: u64,
    pub maximum_manifest_entries: u64,
    pub maximum_manifest_bytes: u64,
    pub maximum_protocol_bytes: u64,
    pub maximum_resident_bytes: u64,
    pub maximum_case_wall_ms: u64,
    pub maximum_batch_wall_ms: u64,
    pub maximum_risk_units: u64,
    pub maximum_cost_units: u64,
    pub confirm_cases: u64,
    pub topology_families: u64,
    pub matched_pairs: u64,
    pub budget_root_sha256: String,
}

impl K2UncertaintyBudgetV1 {
    pub fn frozen_v3() -> K2CompositionResultV1<Self> {
        let mut budget = Self {
            schema: K2_UNCERTAINTY_BUDGET_SCHEMA_V1.to_owned(),
            action_count: K2_UNCERTAINTY_ACTIONS_V1 as u64,
            path_count: K2_UNCERTAINTY_PATHS_V1 as u64,
            content_count: K2_UNCERTAINTY_CONTENTS_V1 as u64,
            support_rows_per_action: K2_UNCERTAINTY_SUPPORT_ROWS_PER_ACTION_V1 as u64,
            state_count: K2_UNCERTAINTY_STATE_COUNT_V1 as u64,
            raw_probe_count: K2_UNCERTAINTY_RAW_PROBES_V1 as u64,
            minimum_representatives: K2_UNCERTAINTY_MIN_REPRESENTATIVES_V1 as u64,
            maximum_representatives: K2_UNCERTAINTY_MAX_REPRESENTATIVES_V1 as u64,
            predecessor_request_probes: K2_UNCERTAINTY_SELECTOR_PROBES_V1 as u64,
            maximum_selector_requests: K2_UNCERTAINTY_MAX_SELECTOR_REQUESTS_V1 as u64,
            maximum_frontier_page_probes: K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1 as u64,
            maximum_content_bytes: K2_UNCERTAINTY_MAX_CONTENT_BYTES_V1 as u64,
            maximum_manifest_entries: K2_UNCERTAINTY_MAX_MANIFEST_ENTRIES_V1 as u64,
            maximum_manifest_bytes: K2_UNCERTAINTY_MAX_MANIFEST_BYTES_V1,
            maximum_protocol_bytes: K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 as u64,
            maximum_resident_bytes: K2_UNCERTAINTY_MAX_RESIDENT_BYTES_V1,
            maximum_case_wall_ms: K2_UNCERTAINTY_MAX_CASE_WALL_MS_V1,
            maximum_batch_wall_ms: K2_UNCERTAINTY_MAX_BATCH_WALL_MS_V1,
            maximum_risk_units: K2_UNCERTAINTY_MAX_RISK_UNITS_V1,
            maximum_cost_units: K2_UNCERTAINTY_MAX_COST_UNITS_V1,
            confirm_cases: K2_UNCERTAINTY_CONFIRM_CASES_V1 as u64,
            topology_families: K2_UNCERTAINTY_TOPOLOGY_FAMILIES_V1 as u64,
            matched_pairs: K2_UNCERTAINTY_MATCHED_PAIRS_V1 as u64,
            budget_root_sha256: String::new(),
        };
        budget.budget_root_sha256 = budget.expected_root()?;
        budget.validate()?;
        Ok(budget)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        if self != &Self::frozen_without_root()?
            || self.budget_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_budget_not_frozen_v3",
            ));
        }
        Ok(())
    }

    fn frozen_without_root() -> K2CompositionResultV1<Self> {
        let mut value = Self::frozen_v3_unchecked();
        value.budget_root_sha256 = value.expected_root()?;
        Ok(value)
    }

    fn frozen_v3_unchecked() -> Self {
        Self {
            schema: K2_UNCERTAINTY_BUDGET_SCHEMA_V1.to_owned(),
            action_count: K2_UNCERTAINTY_ACTIONS_V1 as u64,
            path_count: K2_UNCERTAINTY_PATHS_V1 as u64,
            content_count: K2_UNCERTAINTY_CONTENTS_V1 as u64,
            support_rows_per_action: K2_UNCERTAINTY_SUPPORT_ROWS_PER_ACTION_V1 as u64,
            state_count: K2_UNCERTAINTY_STATE_COUNT_V1 as u64,
            raw_probe_count: K2_UNCERTAINTY_RAW_PROBES_V1 as u64,
            minimum_representatives: K2_UNCERTAINTY_MIN_REPRESENTATIVES_V1 as u64,
            maximum_representatives: K2_UNCERTAINTY_MAX_REPRESENTATIVES_V1 as u64,
            predecessor_request_probes: K2_UNCERTAINTY_SELECTOR_PROBES_V1 as u64,
            maximum_selector_requests: K2_UNCERTAINTY_MAX_SELECTOR_REQUESTS_V1 as u64,
            maximum_frontier_page_probes: K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1 as u64,
            maximum_content_bytes: K2_UNCERTAINTY_MAX_CONTENT_BYTES_V1 as u64,
            maximum_manifest_entries: K2_UNCERTAINTY_MAX_MANIFEST_ENTRIES_V1 as u64,
            maximum_manifest_bytes: K2_UNCERTAINTY_MAX_MANIFEST_BYTES_V1,
            maximum_protocol_bytes: K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 as u64,
            maximum_resident_bytes: K2_UNCERTAINTY_MAX_RESIDENT_BYTES_V1,
            maximum_case_wall_ms: K2_UNCERTAINTY_MAX_CASE_WALL_MS_V1,
            maximum_batch_wall_ms: K2_UNCERTAINTY_MAX_BATCH_WALL_MS_V1,
            maximum_risk_units: K2_UNCERTAINTY_MAX_RISK_UNITS_V1,
            maximum_cost_units: K2_UNCERTAINTY_MAX_COST_UNITS_V1,
            confirm_cases: K2_UNCERTAINTY_CONFIRM_CASES_V1 as u64,
            topology_families: K2_UNCERTAINTY_TOPOLOGY_FAMILIES_V1 as u64,
            matched_pairs: K2_UNCERTAINTY_MATCHED_PAIRS_V1 as u64,
            budget_root_sha256: String::new(),
        }
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_BUDGET_SCHEMA_V1,
            (
                self.action_count,
                self.path_count,
                self.content_count,
                self.support_rows_per_action,
                self.state_count,
                self.raw_probe_count,
                self.minimum_representatives,
                self.maximum_representatives,
            ),
            (
                self.predecessor_request_probes,
                self.maximum_selector_requests,
                self.maximum_frontier_page_probes,
                self.maximum_content_bytes,
                self.maximum_manifest_entries,
                self.maximum_manifest_bytes,
                self.maximum_protocol_bytes,
                self.maximum_resident_bytes,
            ),
            (
                self.maximum_case_wall_ms,
                self.maximum_batch_wall_ms,
                self.maximum_risk_units,
                self.maximum_cost_units,
                self.confirm_cases,
                self.topology_families,
                self.matched_pairs,
            ),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyDomainVocabularyV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub case_id_sha256: String,
    pub split: K2UncertaintySplitV1,
    pub generator_schema_root_sha256: String,
    pub opaque_action_roots_sha256: Vec<String>,
    pub path_atoms: Vec<K2UncertaintyPathAtomV1>,
    pub content_atoms: Vec<K2UncertaintyContentAtomV1>,
    pub budget: K2UncertaintyBudgetV1,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub vocabulary_root_sha256: String,
}

impl K2UncertaintyDomainVocabularyV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        experiment_id_sha256: String,
        case_id_sha256: String,
        split: K2UncertaintySplitV1,
        generator_schema_root_sha256: String,
        mut opaque_action_roots_sha256: Vec<String>,
        mut path_atoms: Vec<K2UncertaintyPathAtomV1>,
        mut content_atoms: Vec<K2UncertaintyContentAtomV1>,
    ) -> K2CompositionResultV1<Self> {
        opaque_action_roots_sha256.sort();
        path_atoms.sort();
        content_atoms.sort();
        let budget = K2UncertaintyBudgetV1::frozen_v3()?;
        let authority = denied_authority_v1();
        let mut vocabulary = Self {
            schema: K2_UNCERTAINTY_VOCABULARY_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            case_id_sha256,
            split,
            generator_schema_root_sha256,
            opaque_action_roots_sha256,
            path_atoms,
            content_atoms,
            budget,
            authority,
            vocabulary_root_sha256: String::new(),
        };
        vocabulary.vocabulary_root_sha256 = vocabulary.expected_root()?;
        vocabulary.validate()?;
        Ok(vocabulary)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_id_sha256,
            &self.case_id_sha256,
            &self.generator_schema_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_exact_len_v1(
            self.opaque_action_roots_sha256.len(),
            K2_UNCERTAINTY_ACTIONS_V1,
            "self_formed_action_count_invalid",
        )?;
        require_sorted_unique_v1(
            &self.opaque_action_roots_sha256,
            "self_formed_action_roots_not_unique",
        )?;
        for root in &self.opaque_action_roots_sha256 {
            require_composition_root_v1(root)?;
        }
        require_exact_len_v1(
            self.path_atoms.len(),
            K2_UNCERTAINTY_PATHS_V1,
            "self_formed_path_count_invalid",
        )?;
        require_exact_len_v1(
            self.content_atoms.len(),
            K2_UNCERTAINTY_CONTENTS_V1,
            "self_formed_content_count_invalid",
        )?;
        for (ordinal, atom) in self.path_atoms.iter().enumerate() {
            atom.validate()?;
            if usize::from(atom.ordinal) != ordinal {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_path_ordinal_invalid",
                ));
            }
        }
        for (ordinal, atom) in self.content_atoms.iter().enumerate() {
            atom.validate()?;
            if usize::from(atom.ordinal) != ordinal {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_content_ordinal_invalid",
                ));
            }
        }
        if self
            .content_atoms
            .iter()
            .map(|atom| &atom.bytes_sha256)
            .collect::<BTreeSet<_>>()
            .len()
            != K2_UNCERTAINTY_CONTENTS_V1
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_content_atoms_not_distinct",
            ));
        }
        self.budget.validate()?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_VOCABULARY_SCHEMA_V1
            || self.vocabulary_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_vocabulary_invalid",
            ));
        }
        Ok(())
    }

    pub fn content_by_sha256(&self, root: &str) -> Option<&K2UncertaintyContentAtomV1> {
        self.content_atoms
            .iter()
            .find(|atom| atom.bytes_sha256 == root)
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_VOCABULARY_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.case_id_sha256,
            self.split,
            &self.generator_schema_root_sha256,
            &self.opaque_action_roots_sha256,
            &self.path_atoms,
            &self.content_atoms,
            &self.budget,
            &self.authority,
        ))
    }
}
