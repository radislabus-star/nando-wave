use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{ResponseOperation, ResponseProgram};

use crate::multi_source::{BlindThenRevealJoinedTransitionV1, NaturalT1ProgramArtifactV1};

#[derive(Clone)]
pub(super) struct ExternalProgramEvidence {
    pub(super) artifact_roots_sha256: Vec<String>,
    pub(super) programs: BTreeMap<String, ResponseProgram>,
    pub(super) verified_program_roots_sha256: BTreeSet<String>,
}

type CandidateArtifactIndex = BTreeMap<(String, String), Vec<NaturalT1ProgramArtifactV1>>;

pub(super) fn index_candidate_artifacts(
    artifacts: &[NaturalT1ProgramArtifactV1],
) -> Result<CandidateArtifactIndex, &'static str> {
    let mut index = CandidateArtifactIndex::new();
    for artifact in artifacts {
        artifact.validate()?;
        index
            .entry((
                artifact.turn_intent_id_sha256.clone(),
                artifact.session_id_sha256.clone(),
            ))
            .or_default()
            .push(artifact.clone());
    }
    for candidates in index.values_mut() {
        candidates
            .sort_by(|left, right| left.artifact_root_sha256.cmp(&right.artifact_root_sha256));
    }
    Ok(index)
}

pub(super) fn collection_candidate_programs(
    joined: &BlindThenRevealJoinedTransitionV1,
    artifacts: &CandidateArtifactIndex,
) -> Result<ExternalProgramEvidence, &'static str> {
    let candidates = artifacts
        .get(&(
            joined.turn_intent_id_sha256.clone(),
            joined.session_id_sha256.clone(),
        ))
        .ok_or("natural_collection_candidate_artifact_missing")?;
    let mut artifact_roots_sha256 = Vec::new();
    let mut programs = BTreeMap::new();
    let mut verified_program_roots_sha256 = BTreeSet::new();
    for artifact in candidates {
        let mut contributed = false;
        for root in &artifact.verified_program_roots_sha256 {
            let Some(program) = artifact.programs.get(root) else {
                return Err("natural_collection_candidate_program_missing");
            };
            if !matches!(
                program.operation,
                ResponseOperation::ComposeCollection { .. }
            ) {
                continue;
            }
            programs.insert(root.clone(), program.clone());
            verified_program_roots_sha256.insert(root.clone());
            contributed = true;
        }
        if contributed {
            artifact_roots_sha256.push(artifact.artifact_root_sha256.clone());
        }
    }
    if programs.is_empty() {
        return Err("natural_collection_candidate_generation_empty");
    }
    Ok(ExternalProgramEvidence {
        artifact_roots_sha256,
        programs,
        verified_program_roots_sha256,
    })
}
