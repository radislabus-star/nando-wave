use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{canonical_json_bytes, canonical_json_sha256, valid_nonzero_sha256};

pub const PROGRAM_SEMANTIC_CLASS_SCHEMA_V1: &str = "nando.program-semantic-class.v1";
pub const PROGRAM_SEMANTIC_CLASS_MAX_BYTES_V1: usize = 4 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ProgramSemanticClassIdV1(String);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProgramSemanticClassDescriptorV1 {
    schema: String,
    effect_law_id_sha256: String,
    role_schema_root_sha256: String,
    protocol_mode_set_root_sha256: String,
    executable_behavior_root_sha256: String,
    verifier_contract_root_sha256: String,
    class_id: ProgramSemanticClassIdV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramSemanticClassInputV1 {
    pub effect_law_id_sha256: String,
    pub role_schema_root_sha256: String,
    pub protocol_mode_set_root_sha256: String,
    pub executable_behavior_root_sha256: String,
    pub verifier_contract_root_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgramSemanticClassErrorV1 {
    InvalidRoot,
    InvalidDescriptor,
    BudgetExhausted,
    Serialization,
}

impl fmt::Display for ProgramSemanticClassErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRoot => "program semantic class contains an invalid root",
            Self::InvalidDescriptor => "program semantic class descriptor is invalid",
            Self::BudgetExhausted => "program semantic class exceeds its byte budget",
            Self::Serialization => "program semantic class serialization failed",
        })
    }
}

impl std::error::Error for ProgramSemanticClassErrorV1 {}

pub fn seal_program_semantic_class_v1(
    input: ProgramSemanticClassInputV1,
) -> Result<ProgramSemanticClassDescriptorV1, ProgramSemanticClassErrorV1> {
    validate_roots(&input)?;
    let class_id = ProgramSemanticClassIdV1(
        canonical_json_sha256(&(
            PROGRAM_SEMANTIC_CLASS_SCHEMA_V1,
            input.effect_law_id_sha256.as_str(),
            input.role_schema_root_sha256.as_str(),
            input.protocol_mode_set_root_sha256.as_str(),
            input.executable_behavior_root_sha256.as_str(),
            input.verifier_contract_root_sha256.as_str(),
        ))
        .map_err(|_| ProgramSemanticClassErrorV1::Serialization)?,
    );
    let descriptor = ProgramSemanticClassDescriptorV1 {
        schema: PROGRAM_SEMANTIC_CLASS_SCHEMA_V1.to_owned(),
        effect_law_id_sha256: input.effect_law_id_sha256,
        role_schema_root_sha256: input.role_schema_root_sha256,
        protocol_mode_set_root_sha256: input.protocol_mode_set_root_sha256,
        executable_behavior_root_sha256: input.executable_behavior_root_sha256,
        verifier_contract_root_sha256: input.verifier_contract_root_sha256,
        class_id,
    };
    descriptor.validate()?;
    Ok(descriptor)
}

impl ProgramSemanticClassDescriptorV1 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ProgramSemanticClassErrorV1> {
        self.validate()?;
        let bytes =
            canonical_json_bytes(self).map_err(|_| ProgramSemanticClassErrorV1::Serialization)?;
        if bytes.len() > PROGRAM_SEMANTIC_CLASS_MAX_BYTES_V1 {
            return Err(ProgramSemanticClassErrorV1::BudgetExhausted);
        }
        Ok(bytes)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, ProgramSemanticClassErrorV1> {
        if bytes.len() > PROGRAM_SEMANTIC_CLASS_MAX_BYTES_V1 {
            return Err(ProgramSemanticClassErrorV1::BudgetExhausted);
        }
        let descriptor: Self = serde_json::from_slice(bytes)
            .map_err(|_| ProgramSemanticClassErrorV1::InvalidDescriptor)?;
        descriptor.validate()?;
        if descriptor.canonical_bytes()? != bytes {
            return Err(ProgramSemanticClassErrorV1::InvalidDescriptor);
        }
        Ok(descriptor)
    }

    pub fn validate(&self) -> Result<(), ProgramSemanticClassErrorV1> {
        if self.schema != PROGRAM_SEMANTIC_CLASS_SCHEMA_V1 {
            return Err(ProgramSemanticClassErrorV1::InvalidDescriptor);
        }
        let input = ProgramSemanticClassInputV1 {
            effect_law_id_sha256: self.effect_law_id_sha256.clone(),
            role_schema_root_sha256: self.role_schema_root_sha256.clone(),
            protocol_mode_set_root_sha256: self.protocol_mode_set_root_sha256.clone(),
            executable_behavior_root_sha256: self.executable_behavior_root_sha256.clone(),
            verifier_contract_root_sha256: self.verifier_contract_root_sha256.clone(),
        };
        validate_roots(&input)?;
        let expected = canonical_json_sha256(&(
            PROGRAM_SEMANTIC_CLASS_SCHEMA_V1,
            input.effect_law_id_sha256.as_str(),
            input.role_schema_root_sha256.as_str(),
            input.protocol_mode_set_root_sha256.as_str(),
            input.executable_behavior_root_sha256.as_str(),
            input.verifier_contract_root_sha256.as_str(),
        ))
        .map_err(|_| ProgramSemanticClassErrorV1::Serialization)?;
        if self.class_id.as_str() != expected {
            return Err(ProgramSemanticClassErrorV1::InvalidDescriptor);
        }
        Ok(())
    }

    #[must_use]
    pub const fn class_id(&self) -> &ProgramSemanticClassIdV1 {
        &self.class_id
    }

    #[must_use]
    pub fn effect_law_id_sha256(&self) -> &str {
        &self.effect_law_id_sha256
    }

    #[must_use]
    pub fn role_schema_root_sha256(&self) -> &str {
        &self.role_schema_root_sha256
    }

    #[must_use]
    pub fn protocol_mode_set_root_sha256(&self) -> &str {
        &self.protocol_mode_set_root_sha256
    }

    #[must_use]
    pub fn executable_behavior_root_sha256(&self) -> &str {
        &self.executable_behavior_root_sha256
    }

    #[must_use]
    pub fn verifier_contract_root_sha256(&self) -> &str {
        &self.verifier_contract_root_sha256
    }
}

impl ProgramSemanticClassIdV1 {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn validate_roots(input: &ProgramSemanticClassInputV1) -> Result<(), ProgramSemanticClassErrorV1> {
    [
        input.effect_law_id_sha256.as_str(),
        input.role_schema_root_sha256.as_str(),
        input.protocol_mode_set_root_sha256.as_str(),
        input.executable_behavior_root_sha256.as_str(),
        input.verifier_contract_root_sha256.as_str(),
    ]
    .into_iter()
    .all(valid_nonzero_sha256)
    .then_some(())
    .ok_or(ProgramSemanticClassErrorV1::InvalidRoot)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(label: &str) -> String {
        canonical_json_sha256(&label).expect("root")
    }

    fn input(label: &str) -> ProgramSemanticClassInputV1 {
        ProgramSemanticClassInputV1 {
            effect_law_id_sha256: root(&format!("{label}:effect")),
            role_schema_root_sha256: root(&format!("{label}:roles")),
            protocol_mode_set_root_sha256: root(&format!("{label}:modes")),
            executable_behavior_root_sha256: root(&format!("{label}:behavior")),
            verifier_contract_root_sha256: root(&format!("{label}:verifier")),
        }
    }

    #[test]
    fn descriptor_roundtrips_and_rejects_tamper() {
        let descriptor = seal_program_semantic_class_v1(input("wait")).expect("descriptor");
        let bytes = descriptor.canonical_bytes().expect("bytes");
        assert_eq!(
            ProgramSemanticClassDescriptorV1::from_canonical_bytes(&bytes)
                .expect("restored")
                .canonical_bytes()
                .expect("restored bytes"),
            bytes
        );

        let mut tampered = bytes;
        let index = tampered.len() / 2;
        tampered[index] ^= 1;
        assert!(ProgramSemanticClassDescriptorV1::from_canonical_bytes(&tampered).is_err());
    }

    #[test]
    fn protocol_mode_set_is_part_of_executable_semantic_identity() {
        let left = seal_program_semantic_class_v1(input("wait")).expect("left");
        let mut changed = input("wait");
        changed.protocol_mode_set_root_sha256 = root("broader-mode-set");
        let right = seal_program_semantic_class_v1(changed).expect("right");
        assert_ne!(left.class_id(), right.class_id());
    }
}
