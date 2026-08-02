use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    ResponseProgram, canonical_json_sha256, response_program_version_root_sha256,
    valid_nonzero_sha256,
};
use serde::{Deserialize, Serialize};

use crate::CaptureTransitionBinding;

pub const NATURAL_T1_PROGRAM_ARTIFACT_SCHEMA_V1: &str = "nando.natural-t1-program-artifact.v1";
pub const NATURAL_T1_PROGRAM_ARTIFACT_MAX_PROGRAMS_V1: usize = 4_096;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NaturalT1ProgramArtifactV1 {
    pub schema: String,
    pub artifact_root_sha256: String,
    pub turn_intent_id_sha256: String,
    pub session_id_sha256: String,
    pub capture_binding: CaptureTransitionBinding,
    pub programs: BTreeMap<String, ResponseProgram>,
    pub verified_program_roots_sha256: Vec<String>,
}

impl NaturalT1ProgramArtifactV1 {
    pub fn seal(
        turn_intent_id_sha256: String,
        session_id_sha256: String,
        capture_binding: CaptureTransitionBinding,
        programs: BTreeMap<String, ResponseProgram>,
        verified_program_roots_sha256: Vec<String>,
    ) -> Result<Self, &'static str> {
        let mut artifact = Self {
            schema: NATURAL_T1_PROGRAM_ARTIFACT_SCHEMA_V1.to_owned(),
            artifact_root_sha256: String::new(),
            turn_intent_id_sha256,
            session_id_sha256,
            capture_binding,
            programs,
            verified_program_roots_sha256,
        };
        artifact.normalize();
        artifact.validate_members()?;
        artifact.artifact_root_sha256 = artifact.expected_root()?;
        Ok(artifact)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        self.validate_members()?;
        if self.artifact_root_sha256 != self.expected_root()? {
            return Err("natural_t1_program_artifact_root_mismatch");
        }
        Ok(())
    }

    fn normalize(&mut self) {
        self.verified_program_roots_sha256.sort();
        self.verified_program_roots_sha256.dedup();
    }

    fn validate_members(&self) -> Result<(), &'static str> {
        if self.schema != NATURAL_T1_PROGRAM_ARTIFACT_SCHEMA_V1
            || self.programs.is_empty()
            || self.programs.len() > NATURAL_T1_PROGRAM_ARTIFACT_MAX_PROGRAMS_V1
            || self.verified_program_roots_sha256.is_empty()
            || [
                self.turn_intent_id_sha256.as_str(),
                self.session_id_sha256.as_str(),
                self.capture_binding.frame_id_sha256.as_str(),
                self.capture_binding.record_sha256.as_str(),
            ]
            .into_iter()
            .any(|root| !valid_nonzero_sha256(root))
        {
            return Err("natural_t1_program_artifact_invalid");
        }
        self.capture_binding.verify_digest()?;
        let verified = self
            .verified_program_roots_sha256
            .iter()
            .collect::<BTreeSet<_>>();
        if verified.len() != self.verified_program_roots_sha256.len()
            || verified
                .iter()
                .any(|root| !self.programs.contains_key(*root))
            || self.programs.iter().any(|(root, program)| {
                program.validate().is_err()
                    || response_program_version_root_sha256(program).as_deref() != Ok(root.as_str())
            })
        {
            return Err("natural_t1_program_artifact_program_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            NATURAL_T1_PROGRAM_ARTIFACT_SCHEMA_V1,
            self.turn_intent_id_sha256.as_str(),
            self.session_id_sha256.as_str(),
            &self.capture_binding,
            &self.programs,
            &self.verified_program_roots_sha256,
        ))
        .map_err(|_| "natural_t1_program_artifact_digest_failed")
    }
}
