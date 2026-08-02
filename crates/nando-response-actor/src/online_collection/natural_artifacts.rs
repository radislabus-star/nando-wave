use super::*;

const MAX_NATURAL_T1_ARTIFACTS: usize = 512;

pub(super) fn natural_t1_program_artifacts(
    checkpoint: &OnlineCollectionCheckpoint,
) -> Result<Vec<nando_operator_learning::multi_source::NaturalT1ProgramArtifactV1>, String> {
    let mut artifacts = Vec::new();
    for bucket in &checkpoint.buckets {
        for receipt in bucket.support.iter().chain(&bucket.future) {
            if !receipt.verifier_pass {
                continue;
            }
            let Some(binding) = &receipt.capture_binding else {
                continue;
            };
            binding.verify_digest().map_err(str::to_owned)?;
            let verified = receipt
                .matched_program_sha256
                .iter()
                .chain(&receipt.verified_semantic_program_sha256)
                .filter(|root| bucket.programs.contains_key(*root))
                .cloned()
                .collect::<BTreeSet<_>>();
            if verified.is_empty() {
                continue;
            }
            let programs = bucket
                .programs
                .iter()
                .filter(|(root, program)| {
                    verified.contains(*root)
                        && is_source_neutral_response_program(program)
                        && is_learned_bounded_response_program(program)
                        && is_privacy_safe_online_response_program(program)
                })
                .map(|(root, program)| (root.clone(), program.clone()))
                .collect::<BTreeMap<_, _>>();
            let verified = verified
                .into_iter()
                .filter(|root| programs.contains_key(root))
                .collect::<BTreeSet<_>>();
            if verified.is_empty() {
                continue;
            }
            artifacts.push(
                nando_operator_learning::multi_source::NaturalT1ProgramArtifactV1::seal(
                    receipt.client_intent_id_sha256.clone(),
                    receipt.session_id_sha256.clone(),
                    binding.clone(),
                    programs,
                    verified.into_iter().collect(),
                )
                .map_err(str::to_owned)?,
            );
            if artifacts.len() > MAX_NATURAL_T1_ARTIFACTS {
                return Err("natural_t1_program_artifact_budget_exhausted".to_owned());
            }
        }
    }
    artifacts.sort_by(|left, right| left.artifact_root_sha256.cmp(&right.artifact_root_sha256));
    artifacts.dedup_by(|left, right| left.artifact_root_sha256 == right.artifact_root_sha256);
    Ok(artifacts)
}
