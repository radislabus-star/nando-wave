use std::path::{Path, PathBuf};

use super::selected_split_nwpc_provider_export_attestation::{
    EXTERNAL_PROVIDER_BILLING_SOURCE_KIND, PROVIDER_EXPORT_ATTESTATION_SCHEMA_VERSION,
    PROVIDER_EXPORT_ATTESTATION_TEMPLATE_SCHEMA_VERSION, provider_export_attestation_path,
    provider_export_fingerprint64, review_provider_export_attestation,
};
use super::write_json_file;

const DEFAULT_TARGETED_AGGREGATE_PROVIDER_EXPORT_ATTESTATION_CONTRACT_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-aggregate-provider-export-attestation-contract-v1.report.json";

pub(crate) fn run_phase_stream_online_miner_targeted_aggregate_provider_export_attestation_contract_v1<
    I,
>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_TARGETED_AGGREGATE_PROVIDER_EXPORT_ATTESTATION_CONTRACT_REPORT)
    });
    let provider_export_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "provider export path is required".to_owned())?;
    let template_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| provider_export_attestation_template_path(&provider_export_path));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let provider_export_fingerprint64 = provider_export_fingerprint64(&provider_export_path)?;
    let provider_export_line_count = provider_export_line_count(&provider_export_path)?;
    let required_attestation_path = provider_export_attestation_path(&provider_export_path);
    let existing_attestation_review =
        review_provider_export_attestation(&provider_export_path, provider_export_fingerprint64)?;

    let template = serde_json::json!({
        "schema_version": PROVIDER_EXPORT_ATTESTATION_TEMPLATE_SCHEMA_VERSION,
        "safe_to_use_as_evidence": false,
        "targeted_aggregate_billing_rows_expected": 677,
        "copy_required_attestation_to": required_attestation_path,
        "required_attestation": {
            "schema_version": PROVIDER_EXPORT_ATTESTATION_SCHEMA_VERSION,
            "source_kind": EXTERNAL_PROVIDER_BILLING_SOURCE_KIND,
            "provider": "replace-with-external-provider-name",
            "billing_source": "replace-with-external-provider-billing-export-name",
            "provider_export_fingerprint64": provider_export_fingerprint64,
            "captured_at": "replace-with-external-export-capture-time",
            "boundary": "attests the adjacent provider export file came from an external provider billing/usage export; does not by itself prove Nando savings"
        },
        "boundary": "template only: placeholders intentionally fail targeted aggregate provider-export attestation validation until replaced with real external provider provenance"
    });
    write_json_file(&template_path, &template)?;

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_targeted_aggregate_provider_export_attestation_contract_v1",
        "provider_export_path": provider_export_path,
        "provider_export_line_count": provider_export_line_count,
        "provider_export_fingerprint64": provider_export_fingerprint64,
        "required_attestation_path": required_attestation_path,
        "template_path": template_path,
        "existing_attestation_review": existing_attestation_review,
        "targeted_aggregate_billing_rows_expected": 677,
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "serving_registry_mutated": false,
        "product_runtime_changed": false,
        "serving_runtime_changed": false,
        "market_money_claim_allowed": false,
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "manual_class_list_used": false,
            "manual_threshold_selection_used": false,
            "local_accept_without_verifier_used": false
        },
        "verdict": "PHASE_STREAM_ONLINE_MINER_TARGETED_AGGREGATE_PROVIDER_EXPORT_ATTESTATION_CONTRACT_V1_READY",
        "boundary": "writes an attestation template/contract for a real external provider export for targeted aggregate billing evidence; does not create valid evidence, normalize money, enable local_accept, promote, serve, or use legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!(
        "phase_stream_online_miner_targeted_aggregate_provider_export_attestation_contract_v1:"
    );
    println!("  report_path: {}", report_path.display());
    println!("  template_path: {}", template_path.display());
    println!(
        "  required_attestation_path: {}",
        required_attestation_path.display()
    );
    println!("  provider_export_fingerprint64: {provider_export_fingerprint64}");
    println!("  market_money_claim_allowed: false");
    println!(
        "  verdict: PHASE_STREAM_ONLINE_MINER_TARGETED_AGGREGATE_PROVIDER_EXPORT_ATTESTATION_CONTRACT_V1_READY"
    );
    Ok(())
}

fn provider_export_attestation_template_path(provider_export_path: &Path) -> PathBuf {
    PathBuf::from(format!(
        "{}.attestation.template.json",
        provider_export_path.display()
    ))
}

fn provider_export_line_count(provider_export_path: &Path) -> Result<usize, String> {
    let text = std::fs::read_to_string(provider_export_path).map_err(|error| {
        format!(
            "failed to read provider export '{}': {error}",
            provider_export_path.display()
        )
    })?;
    Ok(text.lines().filter(|line| !line.trim().is_empty()).count())
}
