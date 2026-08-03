use super::*;
use nando_operator_kernel::AtomValueType;

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
            // Export bounded hypotheses. The online learner does not decide
            // which program is scientifically consistent.
            let programs = bucket
                .programs
                .iter()
                .filter(|(root, program)| artifact_program(root, program))
                .map(|(root, program)| (root.clone(), program.clone()))
                .collect::<BTreeMap<_, _>>();
            if programs.is_empty() {
                continue;
            }
            let hypothesis_roots = programs.keys().cloned().collect();
            let predicted_typed_consequence_roots_sha256 = checkpoint
                .buckets
                .iter()
                .find_map(|candidate| {
                    candidate
                        .runtime_examples
                        .get(&receipt.evidence_graph_sha256)
                })
                .map(|example| predicted_consequences(&programs, example))
                .transpose()?
                .unwrap_or_default();
            artifacts.push(
                nando_operator_learning::multi_source::NaturalT1ProgramArtifactV1::seal_with_predictions(
                    receipt.client_intent_id_sha256.clone(),
                    receipt.session_id_sha256.clone(),
                    binding.clone(),
                    programs,
                    hypothesis_roots,
                    predicted_typed_consequence_roots_sha256,
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

fn predicted_consequences(
    programs: &BTreeMap<String, ResponseProgram>,
    example: &nando_operator_learning::CollectionSynthesisExample,
) -> Result<BTreeMap<String, String>, String> {
    programs
        .iter()
        .filter_map(|(root, program)| {
            let execution = execute_response(program, "", &example.provider_payload);
            let response = execution.response?;
            Some((root, response))
        })
        .map(|(root, response)| {
            let value_type = match serde_json::from_str::<serde_json::Value>(&response) {
                Ok(serde_json::Value::Bool(_)) => AtomValueType::Boolean,
                Ok(serde_json::Value::Number(_)) => AtomValueType::Integer,
                Ok(serde_json::Value::Array(_) | serde_json::Value::Object(_)) => {
                    AtomValueType::Collection
                }
                _ => AtomValueType::String,
            };
            let value_root = nando_operator_kernel::sha256_bytes(response.as_bytes());
            let consequence = nando_operator_learning::multi_source::typed_consequence_root_v1(
                value_type,
                &value_root,
            )
            .map_err(str::to_owned)?;
            Ok((root.clone(), consequence))
        })
        .collect()
}

fn artifact_program(root: &str, program: &ResponseProgram) -> bool {
    program.validate().is_ok()
        && nando_operator_kernel::response_program_version_root_sha256(program).as_deref()
            == Ok(root)
        && is_source_neutral_response_program(program)
        && is_learned_bounded_response_program(program)
        && is_privacy_safe_online_response_program(program)
}

#[cfg(test)]
mod tests {
    use nando_operator_kernel::{
        CollectionProgramStep, ValueProjectionFormat, response_program_version_root_sha256,
    };

    use super::*;

    #[test]
    fn artifact_filter_rejects_invalid_or_rebound_checkpoint_programs() {
        let program = ResponseProgram::compose_collection(
            vec![CollectionProgramStep::Count],
            ValueProjectionFormat::PlainText,
            "completed",
        );
        let root = response_program_version_root_sha256(&program).expect("program root");
        assert!(artifact_program(&root, &program));
        assert!(!artifact_program(&"0".repeat(64), &program));

        let mut invalid = program;
        invalid.schema = "legacy-invalid".to_owned();
        assert!(!artifact_program(&root, &invalid));
    }
}
