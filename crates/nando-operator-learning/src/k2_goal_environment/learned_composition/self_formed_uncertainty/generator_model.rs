use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionLearnedEffectV1,
    K2CompositionResultV1, composition_sha256_bytes_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_ACTIONS_V1, K2_UNCERTAINTY_CONFIRM_CASES_V1,
    K2_UNCERTAINTY_GENERATOR_REQUEST_SCHEMA_V1, K2_UNCERTAINTY_GENERATOR_RESPONSE_SCHEMA_V1,
    K2_UNCERTAINTY_PRIVATE_BATCH_SCHEMA_V1, K2_UNCERTAINTY_PRIVATE_CASE_SCHEMA_V1,
    K2_UNCERTAINTY_PUBLIC_BATCH_SCHEMA_V1, K2_UNCERTAINTY_PUBLIC_CASE_SCHEMA_V1,
    K2UncertaintyDomainVocabularyV1, K2UncertaintySplitV1, K2UncertaintySupportSetV1,
    denied_authority_v1, require_denied_authority_v1, require_exact_len_v1,
    require_sorted_unique_v1, uncertainty_root_v1,
};

pub const K2_UNCERTAINTY_DEVELOPMENT_SEED_COMMITMENT_V1: &str =
    "f8fde5ba0c466bf1d4570daf6326eb54522453dc8e006637906321c764cea138";
pub const K2_UNCERTAINTY_PREREGISTRATION_V2_ROOT_V1: &str =
    "7875d8809b9340774170d2468b07302e17e503712728173b6efb699f9b768a95";
pub const K2_UNCERTAINTY_PREREGISTRATION_V3_ROOT_V1: &str =
    "79b50fb11ed01a246f18882afd5cd27025cbd9d05898585675bd11f3a2c611f8";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyTopologyFamilyV1 {
    U1SingleFour,
    U2DoubleTwo,
    U3SingleFourCost,
    U4DoubleTwoRisk,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyGeneratorRequestV1 {
    pub schema: String,
    pub split: K2UncertaintySplitV1,
    pub seed_bytes: Vec<u8>,
    pub seed_commitment_sha256: String,
    pub preregistration_v2_root_sha256: String,
    pub preregistration_v3_root_sha256: String,
    pub generator_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintyGeneratorRequestV1 {
    pub fn development(
        seed_bytes: Vec<u8>,
        generator_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let seed_commitment_sha256 = composition_sha256_bytes_v1(&seed_bytes);
        let authority = denied_authority_v1();
        let mut request = Self {
            schema: K2_UNCERTAINTY_GENERATOR_REQUEST_SCHEMA_V1.to_owned(),
            split: K2UncertaintySplitV1::Development,
            seed_bytes,
            seed_commitment_sha256,
            preregistration_v2_root_sha256: K2_UNCERTAINTY_PREREGISTRATION_V2_ROOT_V1.to_owned(),
            preregistration_v3_root_sha256: K2_UNCERTAINTY_PREREGISTRATION_V3_ROOT_V1.to_owned(),
            generator_executable_sha256,
            authority,
            request_root_sha256: String::new(),
        };
        request.request_root_sha256 = request.expected_root()?;
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.generator_executable_sha256)?;
        require_denied_authority_v1(&self.authority)?;
        let seed_commitment = composition_sha256_bytes_v1(&self.seed_bytes);
        if self.schema != K2_UNCERTAINTY_GENERATOR_REQUEST_SCHEMA_V1
            || self.split != K2UncertaintySplitV1::Development
            || self.seed_bytes.len() != 32
            || self.seed_commitment_sha256 != seed_commitment
            || self.seed_commitment_sha256 != K2_UNCERTAINTY_DEVELOPMENT_SEED_COMMITMENT_V1
            || self.preregistration_v2_root_sha256 != K2_UNCERTAINTY_PREREGISTRATION_V2_ROOT_V1
            || self.preregistration_v3_root_sha256 != K2_UNCERTAINTY_PREREGISTRATION_V3_ROOT_V1
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_generator_request_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_GENERATOR_REQUEST_SCHEMA_V1,
            self.split,
            &self.seed_bytes,
            &self.seed_commitment_sha256,
            &self.preregistration_v2_root_sha256,
            &self.preregistration_v3_root_sha256,
            &self.generator_executable_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyPrivateMappingEntryV1 {
    pub opaque_action_root_sha256: String,
    pub effect: K2CompositionLearnedEffectV1,
}

impl K2UncertaintyPrivateMappingEntryV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.opaque_action_root_sha256)?;
        self.effect.validate()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyPublicCaseV1 {
    pub schema: String,
    pub vocabulary: K2UncertaintyDomainVocabularyV1,
    pub support: K2UncertaintySupportSetV1,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub public_case_root_sha256: String,
}

impl K2UncertaintyPublicCaseV1 {
    pub fn seal(
        vocabulary: K2UncertaintyDomainVocabularyV1,
        support: K2UncertaintySupportSetV1,
    ) -> K2CompositionResultV1<Self> {
        let authority = denied_authority_v1();
        let mut case = Self {
            schema: K2_UNCERTAINTY_PUBLIC_CASE_SCHEMA_V1.to_owned(),
            vocabulary,
            support,
            authority,
            public_case_root_sha256: String::new(),
        };
        case.public_case_root_sha256 = case.expected_root()?;
        case.validate()?;
        Ok(case)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.vocabulary.validate()?;
        self.support.validate()?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_PUBLIC_CASE_SCHEMA_V1
            || self.vocabulary.split != K2UncertaintySplitV1::Development
            || self.vocabulary.case_id_sha256 != self.support.case_id_sha256
            || self.vocabulary.vocabulary_root_sha256 != self.support.vocabulary_root_sha256
            || self.public_case_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_public_case_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_PUBLIC_CASE_SCHEMA_V1,
            &self.vocabulary,
            &self.support,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyPrivateCaseV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub case_id_sha256: String,
    pub public_case_root_sha256: String,
    pub topology_family: K2UncertaintyTopologyFamilyV1,
    pub matched_pair: u8,
    pub mapping: Vec<K2UncertaintyPrivateMappingEntryV1>,
    pub expected_syntactic_model_count: u64,
    pub private_case_root_sha256: String,
}

impl K2UncertaintyPrivateCaseV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_id_sha256,
            &self.case_id_sha256,
            &self.public_case_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_exact_len_v1(
            self.mapping.len(),
            K2_UNCERTAINTY_ACTIONS_V1,
            "self_formed_private_mapping_count_invalid",
        )?;
        for entry in &self.mapping {
            entry.validate()?;
        }
        require_sorted_unique_v1(&self.mapping, "self_formed_private_mapping_invalid")?;
        let expected = self.expected_root()?;
        if self.schema != K2_UNCERTAINTY_PRIVATE_CASE_SCHEMA_V1
            || self.matched_pair > 1
            || self.expected_syntactic_model_count != 4
            || self.private_case_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_private_case_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.mapping.sort();
        self.private_case_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_PRIVATE_CASE_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.case_id_sha256,
            &self.public_case_root_sha256,
            self.topology_family,
            self.matched_pair,
            &self.mapping,
            self.expected_syntactic_model_count,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyPublicBatchV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub split_commitment_root_sha256: String,
    pub cases: Vec<K2UncertaintyPublicCaseV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub public_batch_root_sha256: String,
}

impl K2UncertaintyPublicBatchV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.experiment_id_sha256)?;
        require_composition_root_v1(&self.split_commitment_root_sha256)?;
        require_exact_len_v1(
            self.cases.len(),
            K2_UNCERTAINTY_CONFIRM_CASES_V1,
            "self_formed_public_batch_case_count_invalid",
        )?;
        let mut case_ids = BTreeSet::new();
        for case in &self.cases {
            case.validate()?;
            if case.vocabulary.experiment_id_sha256 != self.experiment_id_sha256
                || !case_ids.insert(&case.vocabulary.case_id_sha256)
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_public_batch_identity_invalid",
                ));
            }
        }
        require_denied_authority_v1(&self.authority)?;
        let expected = self.expected_root()?;
        if self.schema != K2_UNCERTAINTY_PUBLIC_BATCH_SCHEMA_V1
            || self.split_commitment_root_sha256 != K2_UNCERTAINTY_DEVELOPMENT_SEED_COMMITMENT_V1
            || self.public_batch_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_public_batch_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.authority = denied_authority_v1();
        self.public_batch_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_PUBLIC_BATCH_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.split_commitment_root_sha256,
            &self.cases,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyPrivateBatchV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub public_batch_root_sha256: String,
    pub cases: Vec<K2UncertaintyPrivateCaseV1>,
    pub expected_denominator_commitment_sha256: String,
    pub private_batch_root_sha256: String,
}

impl K2UncertaintyPrivateBatchV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.experiment_id_sha256)?;
        require_composition_root_v1(&self.public_batch_root_sha256)?;
        require_exact_len_v1(
            self.cases.len(),
            K2_UNCERTAINTY_CONFIRM_CASES_V1,
            "self_formed_private_batch_case_count_invalid",
        )?;
        let mut ids = BTreeSet::new();
        for case in &self.cases {
            case.validate()?;
            if case.experiment_id_sha256 != self.experiment_id_sha256
                || !ids.insert(&case.case_id_sha256)
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_private_batch_identity_invalid",
                ));
            }
        }
        let denominator = uncertainty_root_v1(&(
            "nando.k2-self-formed-private-denominator.v1",
            &self.experiment_id_sha256,
            &self.public_batch_root_sha256,
            self.cases
                .iter()
                .map(|case| {
                    (
                        &case.case_id_sha256,
                        case.topology_family,
                        case.expected_syntactic_model_count,
                    )
                })
                .collect::<Vec<_>>(),
        ))?;
        let expected = self.expected_root()?;
        if self.schema != K2_UNCERTAINTY_PRIVATE_BATCH_SCHEMA_V1
            || self.expected_denominator_commitment_sha256 != denominator
            || self.private_batch_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_private_batch_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.expected_denominator_commitment_sha256 = uncertainty_root_v1(&(
            "nando.k2-self-formed-private-denominator.v1",
            &self.experiment_id_sha256,
            &self.public_batch_root_sha256,
            self.cases
                .iter()
                .map(|case| {
                    (
                        &case.case_id_sha256,
                        case.topology_family,
                        case.expected_syntactic_model_count,
                    )
                })
                .collect::<Vec<_>>(),
        ))?;
        self.private_batch_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_PRIVATE_BATCH_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.public_batch_root_sha256,
            &self.cases,
            &self.expected_denominator_commitment_sha256,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyGeneratorResponseV1 {
    pub schema: String,
    pub generator_request_root_sha256: String,
    pub public: K2UncertaintyPublicBatchV1,
    pub private: K2UncertaintyPrivateBatchV1,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub response_root_sha256: String,
}

impl K2UncertaintyGeneratorResponseV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.generator_request_root_sha256)?;
        self.public.validate()?;
        self.private.validate()?;
        require_denied_authority_v1(&self.authority)?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_GENERATOR_RESPONSE_SCHEMA_V1,
            &self.generator_request_root_sha256,
            &self.public,
            &self.private,
            &self.authority,
        ))?;
        if self.schema != K2_UNCERTAINTY_GENERATOR_RESPONSE_SCHEMA_V1
            || self.public.experiment_id_sha256 != self.private.experiment_id_sha256
            || self.public.public_batch_root_sha256 != self.private.public_batch_root_sha256
            || self.response_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_generator_response_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.authority = denied_authority_v1();
        self.response_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_GENERATOR_RESPONSE_SCHEMA_V1,
            &self.generator_request_root_sha256,
            &self.public,
            &self.private,
            &self.authority,
        ))?;
        self.validate()
    }
}
