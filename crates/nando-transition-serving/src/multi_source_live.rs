use nando_operator_admission::OperatorCertificationLedgerV1;
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
    let registry = read_response_registry(registry_path)?;
    Ok(registry
        .packages
        .iter()
        .filter(|package| package.state == ResponsePackageState::Active)
        .filter_map(|package| active_t1_protocol_mode_root_v1(&package.program))
        .collect())
}

pub(crate) fn known_epistemic_protocol_mode_roots(
    registry_path: &Path,
    certification: &OperatorCertificationLedgerV1,
) -> Result<BTreeSet<String>, String> {
    certification
        .validate()
        .map_err(|error| format!("multi_source_certification_registry_invalid:{error}"))?;
    let known_packages = certification
        .latest_entries()
        .into_iter()
        .filter(|entry| entry.epistemic_registry_member && entry.k1_unit_eligible)
        .map(|entry| entry.package_id.as_str())
        .collect::<BTreeSet<_>>();
    let registry = read_response_registry(registry_path)?;
    let mut roots = BTreeSet::new();
    for package_id in known_packages {
        let package = registry
            .packages
            .iter()
            .find(|package| package.package_id == package_id)
            .ok_or_else(|| format!("multi_source_epistemic_package_missing:{package_id}"))?;
        let root = active_t1_protocol_mode_root_v1(&package.program).ok_or_else(|| {
            format!("multi_source_epistemic_protocol_mode_unsupported:{package_id}")
        })?;
        roots.insert(root);
    }
    Ok(roots)
}

fn read_response_registry(registry_path: &Path) -> Result<ResponseRegistry, String> {
    let bytes = match std::fs::read(registry_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ResponseRegistry {
                schema: "nando.response-registry.v6".to_owned(),
                revision: 0,
                packages: Vec::new(),
            });
        }
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
    Ok(registry)
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
    use nando_operator_admission::{
        ExecutionCertificateStatusV1, ExecutionCertificateV1, LawCertificateStatusV1,
        LawCertificateV1, MechanismCertificateStatusV1, MechanismCertificateV1,
        OperatorCertificationEntryV1, OperatorMechanismClassV1,
    };
    use nando_operator_kernel::{
        AtomValueType, ResponseArgument, ResponseProgram, ResponseValueSelector, SemanticRole,
    };
    use nando_response_actor::{ResponsePackage, ResponsePackageOrigin, ResponsePackageProof};

    fn root(value: u64) -> String {
        format!("{value:064x}")
    }

    fn continuation_package(package_id: &str) -> ResponsePackage {
        ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id: package_id.to_owned(),
            origin: ResponsePackageOrigin::LegacyTemplate,
            state: ResponsePackageState::Active,
            program: ResponseProgram::function_call_from_roles(
                "write_stdin",
                ResponseValueSelector::ContinuationHandle {
                    value_type: AtomValueType::Integer,
                },
                vec![ResponseArgument::Role {
                    name: "session_id".to_owned(),
                    role: SemanticRole::ContinuationHandle,
                    value_type: Some(AtomValueType::Integer),
                }],
            ),
            verifier: None,
            routing_predicates: Vec::new(),
            required_routing_atom_ids: Vec::new(),
            phase_centers: vec![1],
            anti_centers: Vec::new(),
            wave_margin_micro: 1,
            learned_wave_route: None,
            crystallized_operator: None,
            proof: ResponsePackageProof {
                support_rows: 32,
                future_rows: 1,
                distinct_sessions: 2,
                distinct_surfaces: 1,
                wrong_accepts: 0,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass: false,
                verifier_schema: "continue_handle_external_evidence.v1".to_owned(),
                adaptive_identification: None,
            },
        }
    }

    fn epistemic_entry(package_id: &str) -> OperatorCertificationEntryV1 {
        let bundle = root(10);
        OperatorCertificationEntryV1::seal(
            &bundle,
            package_id,
            &root(11),
            &root(12),
            ExecutionCertificateV1::seal(
                &bundle,
                package_id,
                ExecutionCertificateStatusV1::Pass,
                vec![root(13)],
                "",
            )
            .expect("execution"),
            LawCertificateV1::seal(
                &bundle,
                package_id,
                LawCertificateStatusV1::Pass,
                vec![root(14)],
                Some(root(15)),
                "",
            )
            .expect("law"),
            MechanismCertificateV1::seal(
                &bundle,
                package_id,
                MechanismCertificateStatusV1::Collecting,
                OperatorMechanismClassV1::Unresolved,
                vec![root(16)],
                "mechanism_collecting",
            )
            .expect("mechanism"),
            0,
        )
        .expect("entry")
    }

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

    #[test]
    fn product_only_package_is_not_an_epistemically_known_protocol_mode() {
        let root_dir =
            std::env::temp_dir().join(format!("nando-k1-product-only-{}", std::process::id()));
        std::fs::create_dir_all(&root_dir).expect("root");
        let registry_path = root_dir.join("registry.json");
        let package = continuation_package("legacy-product-only");
        let expected = active_t1_protocol_mode_root_v1(&package.program).expect("mode root");
        std::fs::write(
            &registry_path,
            serde_json::to_vec(&ResponseRegistry {
                schema: "nando.response-registry.v6".to_owned(),
                revision: 1,
                packages: vec![package],
            })
            .expect("registry"),
        )
        .expect("registry write");

        let empty = OperatorCertificationLedgerV1::empty().expect("empty ledger");
        assert!(
            known_epistemic_protocol_mode_roots(&registry_path, &empty)
                .expect("product-only roots")
                .is_empty()
        );

        let mut epistemic = OperatorCertificationLedgerV1::empty().expect("ledger");
        epistemic
            .append(epistemic_entry("legacy-product-only"))
            .expect("append entry");
        assert_eq!(
            known_epistemic_protocol_mode_roots(&registry_path, &epistemic)
                .expect("epistemic roots"),
            BTreeSet::from([expected])
        );
        std::fs::remove_dir_all(root_dir).expect("cleanup");
    }
}
