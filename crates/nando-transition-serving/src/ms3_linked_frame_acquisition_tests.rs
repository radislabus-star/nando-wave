use super::*;

#[test]
fn contract_and_terminal_failure_restart_byte_identically() {
    let root = std::env::temp_dir().join(format!(
        "nando-ms3-linked-frame-acquisition-{}",
        std::process::id()
    ));
    let topology_root = root.join("topologies");
    let topology_archive =
        MultiSourceTopologyArchive::open(&topology_root).expect("topology archive");
    let mut runtime = Ms3LinkedFrameAcquisitionRuntime::open(&root, &topology_archive, 100, 4, 60)
        .expect("runtime");
    let contract = runtime.contract().clone();
    let collecting = runtime
        .evaluate(100, Vec::new(), Vec::new(), Vec::new())
        .expect("collecting");
    assert!(!collecting.is_terminal());
    assert_eq!(runtime.frozen_evaluated_topology_rows(), None);
    let failed = runtime
        .evaluate(160, Vec::new(), Vec::new(), Vec::new())
        .expect("failed");
    assert!(failed.is_terminal());
    assert_eq!(runtime.frozen_evaluated_topology_rows(), Some(0));
    drop(runtime);

    let mut restarted =
        Ms3LinkedFrameAcquisitionRuntime::open(&root, &topology_archive, 999, 1, 60)
            .expect("restart");
    assert_eq!(restarted.contract(), &contract);
    assert_eq!(restarted.frozen_evaluated_topology_rows(), Some(0));
    assert_eq!(
        restarted
            .evaluate(999, Vec::new(), Vec::new(), Vec::new())
            .expect("restored report"),
        failed
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn changed_topology_prefix_rejects_existing_contract() {
    let root = std::env::temp_dir().join(format!(
        "nando-ms3-linked-frame-prefix-{}",
        std::process::id()
    ));
    let topology_root = root.join("topologies");
    let topology_archive =
        MultiSourceTopologyArchive::open(&topology_root).expect("topology archive");
    Ms3LinkedFrameAcquisitionRuntime::open(&root, &topology_archive, 100, 4, 60).expect("runtime");
    let contract_path = root.join(CONTRACT_FILE);
    let mut contract: Ms3LinkedFrameAcquisitionContractV1 =
        serde_cbor::from_slice(&std::fs::read(&contract_path).expect("contract bytes"))
            .expect("contract");
    contract.topology_prefix_root_sha256 = "11".repeat(32);
    contract.contract_root_sha256 = contract.expected_root();
    std::fs::write(
        &contract_path,
        serde_cbor::to_vec(&contract).expect("encode"),
    )
    .expect("tamper");

    assert_eq!(
        Ms3LinkedFrameAcquisitionRuntime::open(&root, &topology_archive, 100, 4, 60)
            .err()
            .expect("reject"),
        "ms3_acquisition_contract_invalid"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
