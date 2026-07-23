use std::collections::BTreeMap;

use nando_operator_kernel::{ProgramSemanticClassDescriptorV1, ProgramSemanticClassIdV1};
use serde::{Deserialize, Serialize};

use crate::InternedProgram;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticProgramClassV1 {
    descriptor: ProgramSemanticClassDescriptorV1,
    member_program_sha256: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticProgramQuotientV1 {
    classes: Vec<SemanticProgramClassV1>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticQuotientErrorV1 {
    MissingDescriptor,
    InvalidDescriptor,
    ConflictingDescriptor,
}

pub fn build_semantic_program_quotient_v1(
    survivors: &[InternedProgram],
    descriptors: &BTreeMap<String, ProgramSemanticClassDescriptorV1>,
) -> Result<SemanticProgramQuotientV1, SemanticQuotientErrorV1> {
    let mut classes = BTreeMap::<ProgramSemanticClassIdV1, SemanticProgramClassV1>::new();
    for survivor in survivors {
        let descriptor = descriptors
            .get(&survivor.digest_sha256)
            .ok_or(SemanticQuotientErrorV1::MissingDescriptor)?;
        descriptor
            .validate()
            .map_err(|_| SemanticQuotientErrorV1::InvalidDescriptor)?;
        let class_id = descriptor.class_id().clone();
        let class = classes
            .entry(class_id)
            .or_insert_with(|| SemanticProgramClassV1 {
                descriptor: descriptor.clone(),
                member_program_sha256: Vec::new(),
            });
        if class.descriptor != *descriptor {
            return Err(SemanticQuotientErrorV1::ConflictingDescriptor);
        }
        class
            .member_program_sha256
            .push(survivor.digest_sha256.clone());
    }
    for class in classes.values_mut() {
        class.member_program_sha256.sort();
        class.member_program_sha256.dedup();
    }
    Ok(SemanticProgramQuotientV1 {
        classes: classes.into_values().collect(),
    })
}

impl SemanticProgramClassV1 {
    #[must_use]
    pub const fn descriptor(&self) -> &ProgramSemanticClassDescriptorV1 {
        &self.descriptor
    }

    #[must_use]
    pub fn class_id(&self) -> &ProgramSemanticClassIdV1 {
        self.descriptor.class_id()
    }

    #[must_use]
    pub fn member_program_sha256(&self) -> &[String] {
        &self.member_program_sha256
    }
}

impl SemanticProgramQuotientV1 {
    #[must_use]
    pub fn classes(&self) -> &[SemanticProgramClassV1] {
        &self.classes
    }

    #[must_use]
    pub fn class_count(&self) -> usize {
        self.classes.len()
    }
}
