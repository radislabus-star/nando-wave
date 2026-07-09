use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_CLEAN_SUBSET_MANIFEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-clean-subset-manifest-v1.report.json";
const DEFAULT_CLEAN_SUBSET_SELECTOR_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-clean-subset-selector-v1.report.json";
const DEFAULT_FUTURE_TAIL_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-future-tail-replay-v1.report.json";

pub(crate) fn run_phase_stream_online_miner_portfolio_clean_subset_manifest_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CLEAN_SUBSET_MANIFEST_REPORT));
    let clean_selector_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CLEAN_SUBSET_SELECTOR_REPORT));
    let future_tail_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_FUTURE_TAIL_REPORT));

    let future_tail = read_json_value(&future_tail_report_path)?;
    let source_selector_report_path = PathBuf::from(
        json_string(&future_tail, &["selector_report_path"]).ok_or_else(|| {
            format!(
                "future-tail report '{}' missing selector_report_path",
                future_tail_report_path.display()
            )
        })?,
    );
    let source_selector = read_json_value(&source_selector_report_path)?;

    let clean_bucket_rows = future_tail
        .get("bucket_reports")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| json_bool(row, &["future_clean_candidate"]).unwrap_or(false))
        .filter(|row| json_usize(row, &["future_false_accepts"]).unwrap_or(usize::MAX) == 0)
        .filter(|row| json_usize(row, &["future_unique_accepts_over_exact_cache"]).unwrap_or(0) > 0)
        .cloned()
        .collect::<Vec<_>>();

    let clean_bucket_keys = clean_bucket_rows
        .iter()
        .filter_map(|row| json_string(row, &["bucket_key"]))
        .collect::<BTreeSet<_>>();
    let rejected_bucket_rows = future_tail
        .get("bucket_reports")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| {
            !clean_bucket_keys.contains(&json_string(row, &["bucket_key"]).unwrap_or_default())
        })
        .cloned()
        .collect::<Vec<_>>();

    let source_selected_buckets = source_selector
        .get("selected_buckets")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!(
                "source selector '{}' missing selected_buckets",
                source_selector_report_path.display()
            )
        })?;
    let filtered_selected_buckets = source_selected_buckets
        .iter()
        .filter(|row| {
            json_string(row, &["bucket_key"])
                .map(|bucket_key| clean_bucket_keys.contains(&bucket_key))
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();

    let missing_selector_bucket_keys = clean_bucket_keys
        .iter()
        .filter(|bucket_key| {
            !filtered_selected_buckets.iter().any(|row| {
                json_string(row, &["bucket_key"]).as_deref() == Some(bucket_key.as_str())
            })
        })
        .cloned()
        .collect::<Vec<_>>();

    let clean_unique_accepts = json_usize(
        &future_tail,
        &["future_clean_unique_accepts_over_exact_cache"],
    )
    .unwrap_or_else(|| {
        clean_bucket_rows
            .iter()
            .map(|row| json_usize(row, &["future_unique_accepts_over_exact_cache"]).unwrap_or(0))
            .sum()
    });
    let clean_tokens_saved = json_usize(&future_tail, &["future_clean_tokens_saved"])
        .unwrap_or_else(|| {
            clean_bucket_rows
                .iter()
                .map(|row| json_usize(row, &["future_tokens_saved"]).unwrap_or(0))
                .sum()
        });
    let clean_cost_saved_microusd = json_u64(&future_tail, &["future_clean_cost_saved_microusd"])
        .unwrap_or_else(|| {
            clean_bucket_rows
                .iter()
                .map(|row| json_u64(row, &["future_cost_saved_microusd"]).unwrap_or(0))
                .sum()
        });
    let full_future_false_accepts =
        json_usize(&future_tail, &["future_false_accepts"]).unwrap_or(usize::MAX);
    let full_portfolio_runtime_passed =
        json_bool(&future_tail, &["future_runtime_replay_passed"]).unwrap_or(false);
    let hot_margin_parity_mismatches =
        json_usize(&future_tail, &["hot_margin_parity_mismatches"]).unwrap_or(usize::MAX);
    let hot_decision_parity_mismatches =
        json_usize(&future_tail, &["hot_decision_parity_mismatches"]).unwrap_or(usize::MAX);
    let missing_package_rows =
        json_usize(&future_tail, &["missing_package_rows"]).unwrap_or(usize::MAX);

    let clean_subset_shadow_ready = !clean_bucket_keys.is_empty()
        && filtered_selected_buckets.len() == clean_bucket_keys.len()
        && missing_selector_bucket_keys.is_empty()
        && clean_unique_accepts > 0
        && hot_margin_parity_mismatches == 0
        && hot_decision_parity_mismatches == 0
        && missing_package_rows == 0;
    let clean_selector_written = clean_subset_shadow_ready;
    let verdict = if clean_subset_shadow_ready {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_CLEAN_SUBSET_MANIFEST_V1_READY"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_CLEAN_SUBSET_MANIFEST_V1_BLOCKED"
    };

    let mut clean_selector = source_selector
        .as_object()
        .cloned()
        .ok_or_else(|| "source selector report is not a JSON object".to_owned())?;
    clean_selector.insert(
        "report_kind".to_owned(),
        serde_json::json!("phase_stream_online_miner_portfolio_clean_subset_selector_v1"),
    );
    clean_selector.insert(
        "source_selector_report_path".to_owned(),
        serde_json::json!(source_selector_report_path),
    );
    clean_selector.insert(
        "source_future_tail_report_path".to_owned(),
        serde_json::json!(future_tail_report_path),
    );
    clean_selector.insert(
        "selected_buckets".to_owned(),
        serde_json::json!(filtered_selected_buckets),
    );
    clean_selector.insert(
        "selected_bucket_count".to_owned(),
        serde_json::json!(clean_bucket_keys.len()),
    );
    clean_selector.insert(
        "clean_subset_source".to_owned(),
        serde_json::json!("future_tail_zero_false_accept_subcenters"),
    );
    clean_selector.insert(
        "clean_subset_shadow_ready".to_owned(),
        serde_json::json!(clean_subset_shadow_ready),
    );
    clean_selector.insert("local_accept_enabled".to_owned(), serde_json::json!(false));
    clean_selector.insert("auto_promote_enabled".to_owned(), serde_json::json!(false));
    clean_selector.insert(
        "product_promotion_allowed".to_owned(),
        serde_json::json!(false),
    );
    clean_selector.insert(
        "market_money_claim_allowed".to_owned(),
        serde_json::json!(false),
    );
    write_json_file(&clean_selector_report_path, &Value::Object(clean_selector))?;

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_portfolio_clean_subset_manifest_v1",
        "future_tail_report_path": future_tail_report_path,
        "source_selector_report_path": source_selector_report_path,
        "clean_selector_report_path": clean_selector_report_path,
        "full_portfolio_runtime_passed": full_portfolio_runtime_passed,
        "full_future_false_accepts": full_future_false_accepts,
        "full_portfolio_blocked_by_false_accepts": full_future_false_accepts > 0,
        "hot_margin_parity_mismatches": hot_margin_parity_mismatches,
        "hot_decision_parity_mismatches": hot_decision_parity_mismatches,
        "missing_package_rows": missing_package_rows,
        "clean_subset_shadow_ready": clean_subset_shadow_ready,
        "clean_selector_written": clean_selector_written,
        "clean_bucket_count": clean_bucket_keys.len(),
        "clean_bucket_keys": clean_bucket_keys,
        "clean_unique_accepts_over_exact_cache": clean_unique_accepts,
        "clean_tokens_saved": clean_tokens_saved,
        "clean_cost_saved_microusd": clean_cost_saved_microusd,
        "rejected_bucket_count": rejected_bucket_rows.len(),
        "rejected_bucket_reports": rejected_bucket_rows,
        "missing_selector_bucket_keys": missing_selector_bucket_keys,
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "local_accept_without_verifier_used": false
        },
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "serving_registry_mutated": false,
        "product_runtime_changed": false,
        "market_money_claim_allowed": false,
        "product_promotion_allowed": false,
        "verdict": verdict,
        "boundary": "cold clean-subset manifest only: filters future-tail zero-false subcenters into a selector-compatible report; does not mine, compile, promote, serve, enable local_accept, claim market money, or use legacy nwrb"
    });
    write_json_file(&report_path, &report)?;

    println!("phase_stream_online_miner_portfolio_clean_subset_manifest_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  clean_selector_report_path: {}",
        clean_selector_report_path.display()
    );
    println!("  clean_bucket_count: {}", clean_bucket_keys.len());
    println!("  clean_unique_accepts_over_exact_cache: {clean_unique_accepts}");
    println!("  clean_tokens_saved: {clean_tokens_saved}");
    println!("  full_future_false_accepts: {full_future_false_accepts}");
    println!("  clean_subset_shadow_ready: {clean_subset_shadow_ready}");
    println!("  verdict: {verdict}");
    Ok(())
}

fn read_json_value(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse JSON '{}': {error}", path.display()))
}

fn write_json_file(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("failed to serialize '{}': {error}", path.display()))?;
    std::fs::write(path, bytes)
        .map_err(|error| format!("failed to write '{}': {error}", path.display()))
}

fn json_string(value: &Value, path: &[&str]) -> Option<String> {
    json_at(value, path)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn json_bool(value: &Value, path: &[&str]) -> Option<bool> {
    json_at(value, path).and_then(Value::as_bool)
}

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    json_at(value, path)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn json_u64(value: &Value, path: &[&str]) -> Option<u64> {
    json_at(value, path).and_then(Value::as_u64)
}

fn json_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}
