use std::collections::BTreeSet;

use super::super::immutable_publication::decode_canonical_json_v1;
use super::super::{
    K2UncertaintyCleanupReceiptV1, K2UncertaintyControlEvaluationReceiptV1,
    K2UncertaintyDevelopmentResultReceiptV1, K2UncertaintyOracleBaselineBatchReceiptV1,
    K2UncertaintyR8BControlWrapperV3, K2UncertaintyR8BOracleWrapperV3,
};
use super::*;

pub type K2UncertaintyR8BEvidenceViewV3 =
    (String, String, u64, Option<String>, Option<Vec<String>>);

pub fn decode_self_formed_r8b_evidence_view_v3(
    kind: K2UncertaintyR8BEvidenceKindV2,
    bytes: &[u8],
    route_id_sha256: &str,
) -> K2CompositionResultV1<K2UncertaintyR8BEvidenceViewV3> {
    let (schema, semantic_root_sha256, observed, producer_executable_sha256, source_roots_sha256) =
        match kind {
            K2UncertaintyR8BEvidenceKindV2::LegacyControls
            | K2UncertaintyR8BEvidenceKindV2::V3Controls
            | K2UncertaintyR8BEvidenceKindV2::V4Controls
            | K2UncertaintyR8BEvidenceKindV2::FreshControlCases => {
                let value: K2UncertaintyControlEvaluationReceiptV1 =
                    decode_canonical_json_v1(bytes)?;
                value.validate()?;
                (
                    value.schema,
                    value.receipt_root_sha256,
                    value.passed,
                    Some(value.evaluator_executable_sha256),
                    None,
                )
            }
            K2UncertaintyR8BEvidenceKindV2::CleanupTransaction => {
                let value: K2UncertaintyCleanupReceiptV1 = decode_canonical_json_v1(bytes)?;
                value.validate()?;
                (value.schema, value.receipt_root_sha256, 1, None, None)
            }
            K2UncertaintyR8BEvidenceKindV2::DevelopmentResult => {
                let value: K2UncertaintyDevelopmentResultReceiptV1 =
                    decode_canonical_json_v1(bytes)?;
                value.validate()?;
                (value.schema, value.receipt_root_sha256, 1, None, None)
            }
            K2UncertaintyR8BEvidenceKindV2::OracleCases
            | K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes
            | K2UncertaintyR8BEvidenceKindV2::LinkedManifest
            | K2UncertaintyR8BEvidenceKindV2::SuiteManifest => {
                return Err(invalid("self_formed_r8b_v3_special_evidence_redecoded"));
            }
            _ => {
                let value: K2UncertaintyR8BMeasuredReceiptV2 = decode_canonical_json_v1(bytes)?;
                value.validate()?;
                if value.kind != kind || value.route_id_sha256 != route_id_sha256 {
                    return Err(invalid("self_formed_r8b_v3_measured_evidence_invalid"));
                }
                (
                    value.schema,
                    value.receipt_root_sha256,
                    value.observed,
                    Some(value.producer_executable_sha256),
                    Some(value.source_roots_sha256),
                )
            }
        };
    Ok((
        schema,
        semantic_root_sha256,
        observed,
        producer_executable_sha256,
        source_roots_sha256,
    ))
}

pub fn validate_self_formed_r8b_oracle_wrapper_v3(
    value: &K2UncertaintyR8BOracleWrapperV3,
) -> K2CompositionResultV1<()> {
    value.batch.validate()?;
    validate_wrapper_roots_v3(
        &value.completion_event_roots_sha256,
        &value.receipt_roots_sha256,
        16,
    )?;
    let expected = value
        .batch
        .case_receipts
        .iter()
        .map(|row| row.receipt_root_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let mut canonical = value.clone();
    canonical.receipt_root_sha256.clear();
    if value.schema != K2_UNCERTAINTY_R8B_ORACLE_WRAPPER_SCHEMA_V3
        || value
            .receipt_roots_sha256
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != expected
        || value.receipt_root_sha256 != uncertainty_root_v1(&canonical)?
    {
        return Err(invalid("self_formed_r8b_oracle_wrapper_invalid"));
    }
    Ok(())
}

pub fn seal_self_formed_r8b_oracle_wrapper_v3(
    batch: K2UncertaintyOracleBaselineBatchReceiptV1,
    completion_event_roots_sha256: Vec<String>,
    receipt_roots_sha256: Vec<String>,
) -> K2CompositionResultV1<K2UncertaintyR8BOracleWrapperV3> {
    let mut value = K2UncertaintyR8BOracleWrapperV3 {
        schema: K2_UNCERTAINTY_R8B_ORACLE_WRAPPER_SCHEMA_V3.to_owned(),
        batch,
        completion_event_roots_sha256,
        receipt_roots_sha256,
        receipt_root_sha256: String::new(),
    };
    value.receipt_root_sha256 = uncertainty_root_v1(&value)?;
    validate_self_formed_r8b_oracle_wrapper_v3(&value)?;
    Ok(value)
}

pub fn validate_self_formed_r8b_control_wrapper_v3(
    value: &K2UncertaintyR8BControlWrapperV3,
) -> K2CompositionResultV1<()> {
    value.census.validate()?;
    validate_wrapper_roots_v3(
        &value.completion_event_roots_sha256,
        &value.receipt_roots_sha256,
        4,
    )?;
    let mut canonical = value.clone();
    canonical.receipt_root_sha256.clear();
    if value.schema != K2_UNCERTAINTY_R8B_CONTROL_WRAPPER_SCHEMA_V3
        || value.census.kind != K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes
        || value.census.source_roots_sha256 != value.receipt_roots_sha256
        || value.receipt_root_sha256 != uncertainty_root_v1(&canonical)?
    {
        return Err(invalid("self_formed_r8b_control_wrapper_invalid"));
    }
    Ok(())
}

pub fn seal_self_formed_r8b_control_wrapper_v3(
    census: K2UncertaintyR8BMeasuredReceiptV2,
    completion_event_roots_sha256: Vec<String>,
    receipt_roots_sha256: Vec<String>,
) -> K2CompositionResultV1<K2UncertaintyR8BControlWrapperV3> {
    let mut value = K2UncertaintyR8BControlWrapperV3 {
        schema: K2_UNCERTAINTY_R8B_CONTROL_WRAPPER_SCHEMA_V3.to_owned(),
        census,
        completion_event_roots_sha256,
        receipt_roots_sha256,
        receipt_root_sha256: String::new(),
    };
    value.receipt_root_sha256 = uncertainty_root_v1(&value)?;
    validate_self_formed_r8b_control_wrapper_v3(&value)?;
    Ok(value)
}

fn validate_wrapper_roots_v3(
    events: &[String],
    receipts: &[String],
    required: usize,
) -> K2CompositionResultV1<()> {
    for roots in [events, receipts] {
        roots
            .iter()
            .try_for_each(|root| require_composition_root_v1(root))?;
        if roots.len() != required || roots.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(invalid("self_formed_r8b_root_vector_invalid"));
        }
    }
    if !events
        .iter()
        .collect::<BTreeSet<_>>()
        .is_disjoint(&receipts.iter().collect())
    {
        return Err(invalid("self_formed_r8b_dual_root_domain_invalid"));
    }
    Ok(())
}
