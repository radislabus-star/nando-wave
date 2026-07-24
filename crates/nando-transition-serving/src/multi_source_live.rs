use nando_operator_learning::multi_source::{
    LiveMultiSourceDiscoverySnapshotV2, RequestStructureAuditSnapshotV1,
    build_live_multi_source_discovery_snapshot_v2,
};
use nando_operator_learning::opportunity::OpportunityIntentAuditRowV1;
use nando_response_actor::RelationFrame;

pub(crate) const LIVE_MULTI_SOURCE_SNAPSHOT_MAX_BYTES: usize = 1024 * 1024;

pub(crate) fn build_snapshot(
    opportunities: Vec<OpportunityIntentAuditRowV1>,
    requests: RequestStructureAuditSnapshotV1,
    frames: Vec<RelationFrame>,
) -> Result<LiveMultiSourceDiscoverySnapshotV2, String> {
    let snapshot = build_live_multi_source_discovery_snapshot_v2(opportunities, requests, frames);
    if !snapshot.validate() {
        return Err("live_multi_source_snapshot_invalid".to_owned());
    }
    let bytes = serde_json::to_vec(&snapshot)
        .map_err(|error| format!("live_multi_source_snapshot_encode:{error}"))?;
    if bytes.len() > LIVE_MULTI_SOURCE_SNAPSHOT_MAX_BYTES {
        return Err("live_multi_source_snapshot_budget".to_owned());
    }
    Ok(snapshot)
}
