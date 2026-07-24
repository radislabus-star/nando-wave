use nando_operator_learning::multi_source::{
    LiveMultiSourceDiscoverySnapshotV3, RequestStructureAuditSnapshotV1,
    active_t1_protocol_mode_root_v1,
    build_live_multi_source_discovery_snapshot_with_active_protocols_v3,
};
use nando_operator_learning::opportunity::OpportunityIntentAuditRowV1;
use nando_operator_learning::write_atomic_cbor;
use nando_response_actor::{RelationFrame, ResponsePackageState, ResponseRegistry};
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) const LIVE_MULTI_SOURCE_SNAPSHOT_MAX_BYTES: usize = 1024 * 1024;

pub(crate) fn build_snapshot(
    opportunities: Vec<OpportunityIntentAuditRowV1>,
    requests: RequestStructureAuditSnapshotV1,
    frames: Vec<RelationFrame>,
    active_protocol_mode_roots_sha256: &BTreeSet<String>,
) -> Result<LiveMultiSourceDiscoverySnapshotV3, String> {
    let snapshot = build_live_multi_source_discovery_snapshot_with_active_protocols_v3(
        opportunities,
        requests,
        frames,
        active_protocol_mode_roots_sha256,
    );
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

pub(crate) fn active_protocol_mode_roots(registry_path: &Path) -> Result<BTreeSet<String>, String> {
    let bytes = match std::fs::read(registry_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(error) => {
            return Err(format!(
                "multi_source_active_registry_read:{}:{error}",
                registry_path.display()
            ));
        }
    };
    let registry = serde_json::from_slice::<ResponseRegistry>(&bytes)
        .map_err(|error| format!("multi_source_active_registry_decode:{error}"))?;
    registry
        .validate()
        .map_err(|error| format!("multi_source_active_registry_invalid:{error}"))?;
    Ok(registry
        .packages
        .iter()
        .filter(|package| package.state == ResponsePackageState::Active)
        .filter_map(|package| active_t1_protocol_mode_root_v1(&package.program))
        .collect())
}

pub(crate) fn write_snapshot(
    path: &Path,
    snapshot: &LiveMultiSourceDiscoverySnapshotV3,
) -> Result<(), String> {
    if !snapshot.validate() {
        return Err("live_multi_source_snapshot_invalid".to_owned());
    }
    let bytes = serde_cbor::to_vec(snapshot)
        .map_err(|error| format!("live_multi_source_snapshot_encode:{error}"))?;
    if bytes.len() > LIVE_MULTI_SOURCE_SNAPSHOT_MAX_BYTES {
        return Err("live_multi_source_snapshot_budget".to_owned());
    }
    write_atomic_cbor(path, snapshot)
}

pub(crate) fn read_snapshot(path: &Path) -> Result<LiveMultiSourceDiscoverySnapshotV3, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("live_multi_source_snapshot_read:{}:{error}", path.display()))?;
    if bytes.len() > LIVE_MULTI_SOURCE_SNAPSHOT_MAX_BYTES {
        return Err("live_multi_source_snapshot_budget".to_owned());
    }
    let snapshot = serde_cbor::from_slice::<LiveMultiSourceDiscoverySnapshotV3>(&bytes)
        .map_err(|error| format!("live_multi_source_snapshot_decode:{error}"))?;
    if !snapshot.validate() {
        return Err("live_multi_source_snapshot_invalid".to_owned());
    }
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_requests() -> RequestStructureAuditSnapshotV1 {
        RequestStructureAuditSnapshotV1 {
            rows: Vec::new(),
            topologies: Vec::new(),
            evictions: 0,
            stored_turns: 0,
            stored_topologies: 0,
            provider_bound_by_construction: true,
            pre_action_context_persisted: true,
        }
    }

    #[test]
    fn cross_process_snapshot_roundtrip_preserves_root_and_authority_boundary() {
        let root = std::env::temp_dir().join(format!(
            "nando-multi-source-snapshot-{}",
            std::process::id()
        ));
        let path = root.join("snapshot.cbor");
        let snapshot = build_snapshot(Vec::new(), empty_requests(), Vec::new(), &BTreeSet::new())
            .expect("snapshot");

        write_snapshot(&path, &snapshot).expect("write");
        let restored = read_snapshot(&path).expect("read");

        assert_eq!(restored, snapshot);
        assert!(!restored.authority_ready);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn cross_process_snapshot_rejects_forged_root() {
        let root = std::env::temp_dir().join(format!(
            "nando-multi-source-snapshot-forged-{}",
            std::process::id()
        ));
        let path = root.join("snapshot.cbor");
        let mut snapshot =
            build_snapshot(Vec::new(), empty_requests(), Vec::new(), &BTreeSet::new())
                .expect("snapshot");
        snapshot.snapshot_root_sha256 = "0".repeat(64);
        std::fs::create_dir_all(&root).expect("root");
        std::fs::write(&path, serde_cbor::to_vec(&snapshot).expect("encode")).expect("write");

        assert_eq!(
            read_snapshot(&path).expect_err("forged root rejected"),
            "live_multi_source_snapshot_invalid"
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
