use std::path::Path;

use super::state::LiveStorePersistedProductHotQuarantine;

pub(super) fn live_store_load_persisted_product_hot_quarantine(
    report_path: &Path,
) -> Result<LiveStorePersistedProductHotQuarantine, String> {
    if !report_path.exists() {
        return Ok(LiveStorePersistedProductHotQuarantine::default());
    }
    let report = super::super::read_json_value(report_path)?;
    let mut quarantine = LiveStorePersistedProductHotQuarantine::default();
    if let Some(values) = report
        .get("product_hot_score_only_quarantined_profile_ids")
        .and_then(serde_json::Value::as_array)
    {
        for value in values {
            if let Some(profile_id) = value.as_u64().and_then(|id| u32::try_from(id).ok()) {
                quarantine.profile_ids.insert(profile_id);
            }
        }
    }
    if quarantine.profile_ids.is_empty() {
        return Ok(quarantine);
    }
    quarantine.false_accepts = super::super::json_u64(
        &report,
        &["product_hot_score_only_quarantine_false_accepts"],
    )
    .unwrap_or_default() as usize;
    quarantine.reason =
        super::super::json_string(&report, &["product_hot_score_only_quarantine_reason"])
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "persisted_product_hot_shadow_quarantine".to_owned());
    quarantine.trace_id =
        super::super::json_string(&report, &["product_hot_score_only_quarantine_trace_id"])
            .unwrap_or_default();
    quarantine.route_key =
        super::super::json_string(&report, &["product_hot_score_only_quarantine_route_key"])
            .unwrap_or_default();
    quarantine.bucket_key =
        super::super::json_string(&report, &["product_hot_score_only_quarantine_bucket_key"])
            .unwrap_or_default();
    Ok(quarantine)
}
