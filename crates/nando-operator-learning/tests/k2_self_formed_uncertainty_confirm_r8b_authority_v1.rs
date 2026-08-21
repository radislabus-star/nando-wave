use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufReader, Cursor};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use nando_operator_learning::{
    K2_UNCERTAINTY_CLEANUP_RECEIPT_SCHEMA_V1, K2_UNCERTAINTY_DEVELOPMENT_RESULT_RECEIPT_SCHEMA_V1,
    K2_UNCERTAINTY_R8B_AUTHORIZATION_RECEIPT_SCHEMA_V3,
    K2_UNCERTAINTY_R8B_CONTROL_WRAPPER_SCHEMA_V3, K2_UNCERTAINTY_R8B_DOWNSTREAM_CONTRACT_SCHEMA_V3,
    K2_UNCERTAINTY_R8B_ORACLE_WRAPPER_SCHEMA_V3, K2_UNCERTAINTY_R8B_PACKET_MANIFEST_SCHEMA_V3,
    K2_UNCERTAINTY_R8B_PRODUCER_REQUEST_SCHEMA_V3, K2_UNCERTAINTY_R8B_SCHEDULE_AUTHORITY_SCHEMA_V3,
    K2_UNCERTAINTY_R8B_SCHEDULE_FORMULA_V3, K2_UNCERTAINTY_R8B_STATIC_PROJECTION_SCHEMA_V3,
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    K2UncertaintyCleanupReceiptV1, K2UncertaintyConfirmDataMountV1,
    K2UncertaintyConfirmGuestExecutableV1, K2UncertaintyConfirmMountTargetV1,
    K2UncertaintyControlEvaluationRequestV1, K2UncertaintyControlProcessOutcomeV1,
    K2UncertaintyControlScopeV1, K2UncertaintyControlStdoutV1,
    K2UncertaintyDevelopmentResultReceiptV1, K2UncertaintyImmutablePublicationFaultV1,
    K2UncertaintyR8BAuthorizationReceiptV3, K2UncertaintyR8BAuthorizationRequestV3,
    K2UncertaintyR8BCompletionKindV3, K2UncertaintyR8BControlWrapperV3,
    K2UncertaintyR8BDownstreamContractV3, K2UncertaintyR8BEvidenceKindV2,
    K2UncertaintyR8BExecutableIdentityV2, K2UncertaintyR8BExecutableManifestV2,
    K2UncertaintyR8BExpectedOutcomeV3, K2UncertaintyR8BFileAttestationV3,
    K2UncertaintyR8BInputBindingV3, K2UncertaintyR8BInputRoleV3, K2UncertaintyR8BInvocationPlanV3,
    K2UncertaintyR8BLaunchKindV3, K2UncertaintyR8BLedgerSummaryV3, K2UncertaintyR8BLedgerWriterV3,
    K2UncertaintyR8BManagerIdentityV3, K2UncertaintyR8BManifestClassV2,
    K2UncertaintyR8BMeasuredReceiptV2, K2UncertaintyR8BObjectRoleV3,
    K2UncertaintyR8BOracleWrapperV3, K2UncertaintyR8BOutputContractV3,
    K2UncertaintyR8BPacketDescriptorV3, K2UncertaintyR8BPacketManifestV3,
    K2UncertaintyR8BPrivilegedProbeV3, K2UncertaintyR8BProcessEventV3,
    K2UncertaintyR8BProcessLedgerV2, K2UncertaintyR8BProducerRequestV3,
    K2UncertaintyR8BPublicationReceiptV3, K2UncertaintyR8BPublicationRequestV3,
    K2UncertaintyR8BResourceReceiptV3, K2UncertaintyR8BScheduleAuthorityV3,
    K2UncertaintyR8BStaticProjectionV3, K2UncertaintyR8BToolIdentityV3, K2UncertaintyR8BToolRoleV3,
    K2UncertaintyR8BUnitResourceObservationV3, K2UncertaintyR8BValidatedFactV3,
    K2UncertaintyR8BValidatedOutputV3, K2UncertaintyR8BValidatorV3, authorize_self_formed_r8b_v3,
    composition_root_v1, composition_sha256_bytes_v1, composition_sha256_file_v1,
    evaluate_self_formed_controls_v1, expected_self_formed_control_v1,
    immutable_publication_temp_relative_path_v1, publish_immutable_file_v1,
    publish_self_formed_r8b_v3, recover_self_formed_r8b_publication_v3,
    run_self_formed_confirm_sandbox_v1, seal_self_formed_r8b_ledger_header_v3,
    seal_self_formed_r8b_process_event_v3, seal_self_formed_r8b_resource_receipt_v3,
    self_formed_r8b_route_unit_v3, uncertainty_bytes_v1, uncertainty_root_v1,
    validate_self_formed_r8b_control_wrapper_v3, validate_self_formed_r8b_downstream_contract_v3,
    validate_self_formed_r8b_ledger_stream_v3, validate_self_formed_r8b_oracle_wrapper_v3,
    validate_self_formed_r8b_packet_manifest_v3, validate_self_formed_r8b_producer_request_v3,
    validate_self_formed_r8b_resource_receipt_v3, validate_self_formed_r8b_schedule_authority_v3,
};

#[rustfmt::skip]
#[path = "k2_self_formed_uncertainty_confirm_r8b_support/mod.rs"]
mod support;

use support::*;

const S05_SELECTOR_V2: &str = "r8b_v7_s05_authority_aggregate";
static POSITIVE_AUTHORIZATION_V3: OnceLock<K2UncertaintyR8BAuthorizationReceiptV3> = OnceLock::new();

fn fixture_invalid(reason: &'static str) -> K2CompositionErrorV1 {
    K2CompositionErrorV1::Invalid(reason)
}

fn seal_self_formed_r8b_packet_manifest_v3(
    mut value: K2UncertaintyR8BPacketManifestV3,
    ledger: &K2UncertaintyR8BLedgerSummaryV3,
    c08: &K2UncertaintyR8BDownstreamContractV3,
) -> K2CompositionResultV1<K2UncertaintyR8BPacketManifestV3> {
    value.schema = K2_UNCERTAINTY_R8B_PACKET_MANIFEST_SCHEMA_V3.to_owned();
    value
        .members
        .sort_by_key(|row| (row.relative_path.clone(), row.object_role as u8));
    value.manifest_root_sha256.clear();
    value.manifest_root_sha256 = uncertainty_root_v1(&value)?;
    validate_self_formed_r8b_packet_manifest_v3(&value, ledger, c08)?;
    Ok(value)
}

fn seal_self_formed_r8b_control_wrapper_v3(
    census: nando_operator_learning::K2UncertaintyR8BMeasuredReceiptV2,
    mut event_roots: Vec<String>,
) -> K2CompositionResultV1<K2UncertaintyR8BControlWrapperV3> {
    event_roots.sort();
    let mut receipt_roots = census.source_roots_sha256.clone();
    receipt_roots.sort();
    let mut value = K2UncertaintyR8BControlWrapperV3 {
        schema: K2_UNCERTAINTY_R8B_CONTROL_WRAPPER_SCHEMA_V3.to_owned(),
        census,
        completion_event_roots_sha256: event_roots,
        receipt_roots_sha256: receipt_roots,
        receipt_root_sha256: String::new(),
    };
    value.receipt_root_sha256 = uncertainty_root_v1(&value)?;
    validate_self_formed_r8b_control_wrapper_v3(&value)?;
    Ok(value)
}

fn seal_self_formed_r8b_oracle_wrapper_v3(
    batch: nando_operator_learning::K2UncertaintyOracleBaselineBatchReceiptV1,
    mut event_roots: Vec<String>,
) -> K2CompositionResultV1<K2UncertaintyR8BOracleWrapperV3> {
    event_roots.sort();
    let mut receipt_roots = batch
        .case_receipts
        .iter()
        .map(|receipt| receipt.receipt_root_sha256.clone())
        .collect::<Vec<_>>();
    receipt_roots.sort();
    let mut value = K2UncertaintyR8BOracleWrapperV3 {
        schema: K2_UNCERTAINTY_R8B_ORACLE_WRAPPER_SCHEMA_V3.to_owned(),
        batch,
        completion_event_roots_sha256: event_roots,
        receipt_roots_sha256: receipt_roots,
        receipt_root_sha256: String::new(),
    };
    value.receipt_root_sha256 = uncertainty_root_v1(&value)?;
    validate_self_formed_r8b_oracle_wrapper_v3(&value)?;
    Ok(value)
}

#[derive(Clone)]
struct PacketEvidenceV3 {
    kind: K2UncertaintyR8BEvidenceKindV2,
    relative_path: String,
    bytes: Vec<u8>,
    semantic_root_sha256: String,
    receipt_schema: String,
    producer_role: String,
    producer_executable_sha256: String,
    source_roots_sha256: Vec<String>,
}

impl PacketEvidenceV3 {
    fn new<T: serde::Serialize>(
        kind: K2UncertaintyR8BEvidenceKindV2,
        relative_path: &str,
        value: &T,
        semantic_root_sha256: String,
        receipt_schema: &str,
        producer_role: &str,
        producer_executable_sha256: String,
        source_roots_sha256: Vec<String>,
    ) -> Self {
        Self {
            kind,
            relative_path: relative_path.to_owned(),
            bytes: uncertainty_bytes_v1(value).expect("canonical packet evidence bytes"),
            semantic_root_sha256,
            receipt_schema: receipt_schema.to_owned(),
            producer_role: producer_role.to_owned(),
            producer_executable_sha256,
            source_roots_sha256,
        }
    }

    fn descriptor(&self) -> K2UncertaintyR8BPacketDescriptorV3 {
        K2UncertaintyR8BPacketDescriptorV3 {
            relative_path: self.relative_path.clone(),
            object_role: K2UncertaintyR8BObjectRoleV3::Evidence,
            evidence_kind: Some(self.kind),
            byte_len: self.bytes.len() as u64,
            unix_mode: 0o400,
            content_sha256: composition_sha256_bytes_v1(&self.bytes),
            semantic_root_sha256: self.semantic_root_sha256.clone(),
        }
    }

    fn authority_output(&self) -> K2UncertaintyR8BOutputContractV3 {
        K2UncertaintyR8BOutputContractV3 {
            relative_path: self.relative_path.clone(),
            object_role: K2UncertaintyR8BObjectRoleV3::Evidence,
            evidence_kind: Some(self.kind),
            receipt_schema: self.receipt_schema.clone(),
            required_denominator: self.kind.required(),
            required_source_roots_sha256: self.source_roots_sha256.clone(),
            producer_role: self.producer_role.clone(),
            producer_executable_sha256: self.producer_executable_sha256.clone(),
            validator: K2UncertaintyR8BValidatorV3::ConcreteReceipt,
            file_attestation: Some(K2UncertaintyR8BFileAttestationV3 {
                byte_len: self.bytes.len() as u64,
                unix_mode: 0o400,
                content_sha256: composition_sha256_bytes_v1(&self.bytes),
                semantic_root_sha256: self.semantic_root_sha256.clone(),
            }),
        }
    }
}

fn seal_self_formed_r8b_schedule_authority_v3(
    schedule_grammar_root_sha256: String,
    mut case_ids_sha256: Vec<String>,
) -> K2CompositionResultV1<K2UncertaintyR8BScheduleAuthorityV3> {
    case_ids_sha256.sort();
    let mut value = K2UncertaintyR8BScheduleAuthorityV3 {
        schema: K2_UNCERTAINTY_R8B_SCHEDULE_AUTHORITY_SCHEMA_V3.to_owned(),
        formula: K2_UNCERTAINTY_R8B_SCHEDULE_FORMULA_V3.to_owned(),
        schedule_grammar_root_sha256,
        case_ids_sha256,
        minimum_representatives: 8,
        maximum_representatives: 1_792,
        authority_root_sha256: String::new(),
    };
    value.authority_root_sha256 = uncertainty_root_v1(&value)?;
    validate_self_formed_r8b_schedule_authority_v3(&value)?;
    Ok(value)
}

fn seal_self_formed_r8b_producer_request_v3(
    mut value: K2UncertaintyR8BProducerRequestV3,
) -> K2CompositionResultV1<K2UncertaintyR8BProducerRequestV3> {
    value.schema = K2_UNCERTAINTY_R8B_PRODUCER_REQUEST_SCHEMA_V3.to_owned();
    value.request_root_sha256.clear();
    value.request_root_sha256 = uncertainty_root_v1(&value)?;
    validate_self_formed_r8b_producer_request_v3(&value)?;
    Ok(value)
}

fn seal_self_formed_r8b_downstream_contract_v3(
    mut value: K2UncertaintyR8BDownstreamContractV3,
) -> K2CompositionResultV1<K2UncertaintyR8BDownstreamContractV3> {
    value.schema = K2_UNCERTAINTY_R8B_DOWNSTREAM_CONTRACT_SCHEMA_V3.to_owned();
    value
        .invocations
        .sort_by(|left, right| left.invocation_id_sha256.cmp(&right.invocation_id_sha256));
    value.projection_root_sha256.clear();
    value.projection_root_sha256 = uncertainty_root_v1(&value)?;
    validate_self_formed_r8b_downstream_contract_v3(&value)?;
    Ok(value)
}

fn seal_self_formed_r8b_static_projection_v3(
    requests: &[K2UncertaintyR8BProducerRequestV3],
    parent_launches: &[K2UncertaintyR8BInvocationPlanV3],
) -> K2CompositionResultV1<K2UncertaintyR8BStaticProjectionV3> {
    let by_role = requests
        .iter()
        .map(|row| (row.producer_role.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    if by_role.len() != 6 || parent_launches.len() != 6 {
        return Err(fixture_invalid("self_formed_r8b_v3_static_fixture_invalid"));
    }
    let mut invocations = parent_launches.to_vec();
    let mut roots = BTreeMap::new();
    for launch in parent_launches {
        let request = by_role
            .get(launch.target_role.as_str())
            .ok_or_else(|| fixture_invalid("self_formed_r8b_v3_static_fixture_target_invalid"))?;
        roots.insert(
            launch.invocation_id_sha256.clone(),
            request.request_root_sha256.clone(),
        );
    }
    for request in requests {
        validate_self_formed_r8b_producer_request_v3(request)?;
        invocations.extend(
            request
                .invocation_plan
                .iter()
                .filter(|row| {
                    request.producer_role != "M24_LINKED_RUNNER"
                        || (row.request_owner_role == "M24_LINKED_RUNNER"
                            && matches!(
                                row.target_role.as_str(),
                                "M01_DEVELOPMENT_OWNER" | "M10_PUBLIC_COORDINATOR"
                            ))
                })
                .cloned(),
        );
    }
    let m01 = by_role["M24_LINKED_RUNNER"]
        .invocation_plan
        .iter()
        .find(|row| row.target_role == "M01_DEVELOPMENT_OWNER")
        .ok_or_else(|| fixture_invalid("self_formed_r8b_v3_static_fixture_m01_missing"))?;
    let nested = by_role["S02_RESTART"]
        .invocation_plan
        .iter()
        .find(|row| {
            row.request_owner_role == "M01_DEVELOPMENT_OWNER" && row.target_role == "M02_GENERATOR"
        })
        .ok_or_else(|| fixture_invalid("self_formed_r8b_v3_static_fixture_m02_missing"))?;
    let mut nested = nested.clone();
    nested.invocation_id_sha256 = uncertainty_root_v1(&(
        "nando.k2-self-formed-r8b-m01-m02-invocation.v3",
        &m01.invocation_id_sha256,
        &nested.target_executable_sha256,
    ))?;
    nested.parent_invocation_id_sha256 = Some(m01.invocation_id_sha256.clone());
    nested.request_owner_executable_sha256 = m01.target_executable_sha256.clone();
    nested.case_id_sha256 = None;
    nested.probe_ordinal = None;
    invocations.push(nested);
    invocations.sort_by(|left, right| left.invocation_id_sha256.cmp(&right.invocation_id_sha256));
    let mut value = K2UncertaintyR8BStaticProjectionV3 {
        schema: K2_UNCERTAINTY_R8B_STATIC_PROJECTION_SCHEMA_V3.to_owned(),
        route_id_sha256: requests[0].route_id_sha256.clone(),
        schedule_grammar_root_sha256: requests[0].schedule_grammar_root_sha256.clone(),
        invocations,
        producer_request_roots_sha256: roots,
        projection_root_sha256: String::new(),
    };
    value.projection_root_sha256 = uncertainty_root_v1(&value)?;
    Ok(value)
}

#[test]
#[ignore = "requires explicit R8B V7 execution authorization"]
fn r8b_v7_s05_authority_aggregate() {
    begin_suite_request_from_stdin_v2("S05_AUTHORITY_PUBLICATION", S05_SELECTOR_V2);
    r8b_publisher_rejects_no_clobber_and_symlink_boundaries();
    publish_suite_measurements_v2(vec![SuiteMeasurementV2 {
        relative_path: "suites/s05/aggregate-publication-faults.json",
        kind: nando_operator_learning::K2UncertaintyR8BEvidenceKindV2::AggregatePublicationFaults,
        source_roots_sha256: vec![root_v1("s05-existing"), root_v1("s05-symlink")],
        observed: 2,
        metrics: std::collections::BTreeMap::new(),
    }]);
}

#[test]
fn r8b_authorizer_requires_a_closed_actual_packet() {
    let environment = TestEnvironmentV1::new("authority-closed-packet");
    let packet = environment.private_child("packet");
    write_new_read_only_v2(
        &packet.join("aggregate-manifest.json"),
        b"{\"not\":\"a canonical R8B manifest\"}",
    );
    freeze_directory_tree_v2(&packet);
    let authorizer = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-r8b-authorizer"));
    let authorizer_sha = composition_sha256_file_v1(&authorizer).expect("M25 SHA-256");
    let request = K2UncertaintyR8BAuthorizationRequestV3::seal(
        root_v1("closed-packet-route"),
        root_v1("closed-packet-manifest"),
        authorizer_sha.clone(),
    )
    .expect("M25 request");
    let mount = [K2UncertaintyConfirmDataMountV1 {
        host_path: &packet,
        target: K2UncertaintyConfirmMountTargetV1::AggregateEvidence,
        writable: false,
    }];
    assert!(
        run_self_formed_confirm_sandbox_v1(
            K2UncertaintyConfirmGuestExecutableV1::R8BAggregateAuthorizer,
            &authorizer,
            &authorizer_sha,
            &mount,
            &uncertainty_bytes_v1(&request).expect("M25 request bytes"),
            60,
        )
        .is_err()
    );
    assert!(!packet.join("R8B_RECEIPT_V3.json").exists());
}

#[test]
fn r8b_v8_m25_authorizes_one_complete_closed_packet() {
    let receipt = authorize_positive_packet_v3();
    receipt
        .validate()
        .expect("positive M25 V3 authorization receipt");
    assert_eq!(receipt.packet_member_roots_sha256.len(), 22);
    assert_eq!(receipt.disposition, "R8B_FROZEN");
    assert_eq!(
        receipt.authority,
        K2CompositionAuthorityBoundaryV1::denied()
    );
}

#[test]
fn r8b_v8_m24_completion_fits_frozen_event_budget() {
    let mut outputs = producer_request_v3("M24_LINKED_RUNNER", Vec::new()).outputs;
    for output in &mut outputs {
        output.required_source_roots_sha256 = match output.evidence_kind {
            Some(K2UncertaintyR8BEvidenceKindV2::LinkedRoute) => roots_v3("linked-source", 3),
            Some(K2UncertaintyR8BEvidenceKindV2::OracleCases) => Vec::new(),
            Some(K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes) => {
                roots_v3("control-source", 4)
            }
            _ => vec![root_v1("schedule-grammar")],
        };
        output.file_attestation = Some(K2UncertaintyR8BFileAttestationV3 {
            byte_len: 100_000,
            unix_mode: 0o400,
            content_sha256: root_v1(&format!("content-{}", output.relative_path)),
            semantic_root_sha256: root_v1(&format!("semantic-{}", output.relative_path)),
        });
    }
    let receipt_schema = "nando.fixture-m24-receipt.v3";
    let semantic_root_sha256 = root_v1("positive-m24-receipt");
    let stdout = uncertainty_bytes_v1(&(receipt_schema, &semantic_root_sha256)).unwrap();
    let stdout_sha256 = composition_sha256_bytes_v1(&stdout);
    let mut invocation = invocation_v3(11_005, "P01", "M24_LINKED_RUNNER");
    invocation.launch_kind = K2UncertaintyR8BLaunchKindV3::UserSystemd;
    invocation.tool_chain = vec![K2UncertaintyR8BToolIdentityV3 {
        role: K2UncertaintyR8BToolRoleV3::SystemdRun,
        canonical_path: "/usr/bin/systemd-run".to_owned(),
        sha256: root_v1("positive-systemd-run"),
    }];
    let event = K2UncertaintyR8BProcessEventV3 {
        schema: "nando.k2-self-formed-r8b-process-event.v3".to_owned(),
        sequence: 695,
        previous_event_root_sha256: root_v1("previous-event"),
        route_id_sha256: root_v1("v8-positive-route"),
        invocation: invocation.clone(),
        request_root_sha256: root_v1("positive-m24-request"),
        stdin_sha256: root_v1("positive-m24-stdin"),
        started_event_root_sha256: Some(root_v1("positive-m24-start")),
        completion: Some(K2UncertaintyR8BCompletionKindV3::AuthoritySuccess),
        exit_code: Some(0),
        stdout_byte_len: Some(stdout.len() as u64),
        stdout_sha256: Some(stdout_sha256.clone()),
        stderr_byte_len: Some(0),
        stderr_sha256: Some(composition_sha256_bytes_v1(&[])),
        validated_output: Some(K2UncertaintyR8BValidatedOutputV3 {
            stdout_byte_len: stdout.len() as u64,
            stdout_sha256,
            receipt_schema: receipt_schema.to_owned(),
            semantic_root_sha256,
            validator: invocation.validator,
            validator_executable_sha256: invocation.target_executable_sha256,
            fact: K2UncertaintyR8BValidatedFactV3::None,
            authority_outputs: outputs,
        }),
        monotonic_ns: 696,
        event_root_sha256: root_v1("event-root"),
    };
    let bytes = uncertainty_bytes_v1(&event).unwrap();
    assert!(bytes.len() <= 4_096, "M24 completion bytes: {}", bytes.len());
}

#[test]
fn r8b_process_ledger_rejects_started_without_finished() {
    let environment = TestEnvironmentV1::new("authority-ledger");
    let binaries = LinkedBinariesV2::from_cargo();
    let route = root_v1("incomplete-ledger-route");
    let binary = binaries.get("M02_GENERATOR");
    let mut durable = DurableProcessLedgerV2::create(
        &environment.root.join("ledger"),
        route.clone(),
        binaries.get("M24_LINKED_RUNNER"),
        &[binary],
    );
    let started = durable.start(
        "C02",
        None,
        None,
        binary,
        root_v1("generator-request"),
        root_v1("generator-stdin"),
    );
    assert!(K2UncertaintyR8BProcessLedgerV2::seal(route, vec![started]).is_err());
}

#[test]
fn r8b_publisher_writes_exact_authorization_bytes_once() {
    let environment = TestEnvironmentV1::new("authority-publisher");
    let publication = environment.private_child("publication");
    let authorization = authorize_positive_packet_v3();
    let request = K2UncertaintyR8BPublicationRequestV3::seal(
        publication.to_string_lossy().into_owned(),
        authorization.clone(),
    )
    .expect("M26 request");
    let receipt: K2UncertaintyR8BPublicationReceiptV3 =
        publish_self_formed_r8b_v3(&request).expect("M26 exact-byte publication");
    receipt.validate().expect("M26 receipt");
    let final_path = publication.join("R8B_RECEIPT_V3.json");
    assert_eq!(
        fs::read(&final_path).expect("published authorization bytes"),
        uncertainty_bytes_v1(&authorization).expect("expected authorization bytes")
    );
    assert_eq!(
        fs::metadata(final_path)
            .expect("published authorization metadata")
            .permissions()
            .mode()
            & 0o7777,
        0o400
    );
}

#[test]
fn r8b_publisher_rejects_no_clobber_and_symlink_boundaries() {
    for boundary in ["existing", "symlink"] {
        let environment = TestEnvironmentV1::new(boundary);
        let publication = environment.private_child("publication");
        let final_path = publication.join("R8B_RECEIPT_V3.json");
        if boundary == "existing" {
            write_new_read_only_v2(&final_path, b"existing authority bytes");
        } else {
            let outside = environment.root.join("outside.json");
            fs::write(&outside, b"outside").expect("outside bytes");
            std::os::unix::fs::symlink(&outside, &final_path).expect("publication symlink");
        }
        let request = K2UncertaintyR8BPublicationRequestV3::seal(
            publication.to_string_lossy().into_owned(),
            component_authorization_v3(&root_v1("component-publisher")),
        )
        .expect("M26 boundary request");
        assert!(publish_self_formed_r8b_v3(&request).is_err());
        assert!(fs::symlink_metadata(final_path).is_ok());
        if boundary == "symlink" {
            assert_eq!(fs::read(environment.root.join("outside.json")).unwrap(), b"outside");
        }
    }
}

#[test]
fn r8b_v8_m26_recovers_only_linked_no_clobber_temp() {
    let environment = TestEnvironmentV1::new("authority-publisher-recovery");
    let publication = environment.private_child("publication");
    let authorization = component_authorization_v3(&root_v1("component-publisher"));
    let request = K2UncertaintyR8BPublicationRequestV3::seal(
        publication.to_string_lossy().into_owned(),
        authorization.clone(),
    )
    .expect("M26 recovery request");
    let bytes = uncertainty_bytes_v1(&authorization).expect("M25 V3 bytes");
    assert!(
        publish_immutable_file_v1(
            &publication,
            "R8B_RECEIPT_V3.json",
            &bytes,
            0o400,
            0,
            K2UncertaintyImmutablePublicationFaultV1::AfterPublish(0),
        )
        .is_err()
    );
    let temporary = immutable_publication_temp_relative_path_v1("R8B_RECEIPT_V3.json", 0)
        .expect("M26 temporary path");
    assert!(publication.join(&temporary).exists());
    let receipt = recover_self_formed_r8b_publication_v3(&request)
        .expect("M26 linked no-clobber recovery");
    receipt.validate().expect("M26 recovery receipt");
    let final_path = publication.join("R8B_RECEIPT_V3.json");
    assert_eq!(fs::read(&final_path).unwrap(), bytes);
    assert_eq!(fs::metadata(&final_path).unwrap().nlink(), 1);
    assert!(!publication.join(temporary).exists());
    assert!(recover_self_formed_r8b_publication_v3(&request).is_err());
}

#[test]
fn r8b_v8_n17_rejects_false_direct_m24_child_ownership() {
    let mut contract = delegated_contract_v3();
    contract.child_owner = DelegatedChildOwnerV3::M24Direct;
    assert!(validate_delegated_launch_v3(&contract).is_err());
}

#[test]
fn r8b_v8_n18_rejects_unit_and_property_drift() {
    let mut contract = delegated_contract_v3();
    contract.unit = "foreign.service".to_owned();
    assert!(validate_delegated_launch_v3(&contract).is_err());
    let mut contract = delegated_contract_v3();
    contract
        .normalized_argv
        .push("--property=MemoryMax=infinity".to_owned());
    assert!(validate_delegated_launch_v3(&contract).is_err());
}

#[test]
fn r8b_v8_n19_and_n32_reject_input_inventory_drift() {
    let mut request = producer_request_v3("S01_CRATE_UNIT", Vec::new());
    request.inputs.pop();
    assert!(seal_self_formed_r8b_producer_request_v3(request).is_err());
    let mut request = producer_request_v3("S01_CRATE_UNIT", Vec::new());
    request.inputs.push(request.inputs[0].clone());
    assert!(seal_self_formed_r8b_producer_request_v3(request).is_err());
}

#[test]
fn r8b_v8_n20_rejects_output_schema_denominator_substitution() {
    let mut request = producer_request_v3("S01_CRATE_UNIT", Vec::new());
    request.outputs[0].required_denominator = Some(1);
    assert!(seal_self_formed_r8b_producer_request_v3(request).is_err());
}

#[test]
fn r8b_v8_n21_rejects_child_role_substitution() {
    let mut request = producer_request_v3("S03_MODE_MATRIX", simple_plan_v3(6));
    request.invocation_plan[0].target_role = "M99_FOREIGN".to_owned();
    assert!(seal_self_formed_r8b_producer_request_v3(request).is_err());
}

#[test]
fn r8b_v8_n22_rejects_expected_failure_with_authority_output() {
    let invocation = invocation_v3(0, "C09", "M17_CONTROL_EVALUATOR");
    let mut event = terminal_event_v3(
        invocation,
        K2UncertaintyR8BCompletionKindV3::DiagnosticExpectedFailure,
    );
    event.validated_output = Some(validated_output_v3("M17_CONTROL_EVALUATOR"));
    assert!(seal_self_formed_r8b_process_event_v3(event).is_err());
}

#[test]
fn r8b_v8_n23_rejects_generic_schema_root_shape() {
    let bytes = br#"{"schema":"nando.fake.v1","receipt_root_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#;
    assert!(
        nando_operator_learning::uncertainty_decode_v1::<
            nando_operator_learning::K2UncertaintyR8BMeasuredReceiptV2,
        >(bytes)
        .is_err()
    );
}

#[test]
fn r8b_v8_n24_rejects_unpaired_nested_m02() {
    let mut plan = s02_plan_v3();
    plan[15].parent_invocation_id_sha256 = Some(root_v1("foreign-parent"));
    assert!(
        seal_self_formed_r8b_producer_request_v3(producer_request_v3("S02_RESTART", plan)).is_err()
    );
}

#[test]
fn r8b_v8_n25_rejects_tool_chain_omission() {
    let mut plan = simple_plan_v3(6);
    plan[0].launch_kind = K2UncertaintyR8BLaunchKindV3::BwrapPrlimitMediated;
    assert!(
        seal_self_formed_r8b_producer_request_v3(producer_request_v3("S03_MODE_MATRIX", plan))
            .is_err()
    );
}

#[test]
fn r8b_v8_n26_rejects_incomplete_ledger() {
    let header = seal_self_formed_r8b_ledger_header_v3(
        root_v1("route"),
        root_v1("projection"),
        schedule_authority_v3(),
    )
    .unwrap();
    let mut bytes = uncertainty_bytes_v1(&header).unwrap();
    bytes.push(b'\n');
    assert!(
        validate_self_formed_r8b_ledger_stream_v3(BufReader::new(Cursor::new(bytes)), true)
            .is_err()
    );
}

#[test]
fn r8b_v8_n27_rejects_oversized_stream_line() {
    let mut bytes = vec![b'x'; 4_097];
    bytes.push(b'\n');
    assert!(
        validate_self_formed_r8b_ledger_stream_v3(BufReader::new(Cursor::new(bytes)), false)
            .is_err()
    );
}

#[test]
fn r8b_v8_n28_rejects_packet_kind_twenty_or_embedded_ledger() {
    let (mut manifest, ledger, c08) = packet_fixture_v3();
    manifest.members.pop();
    assert!(seal_self_formed_r8b_packet_manifest_v3(manifest, &ledger, &c08).is_err());
}

#[test]
fn r8b_v8_n29_rejects_m16_subset_and_root_swap() {
    let (mut manifest, ledger, c08) = packet_fixture_v3();
    manifest.m16_completion_event_roots_sha256.pop();
    assert!(seal_self_formed_r8b_packet_manifest_v3(manifest, &ledger, &c08).is_err());
    let (mut manifest, ledger, c08) = packet_fixture_v3();
    manifest.m16_completion_event_roots_sha256 = manifest.m16_receipt_roots_sha256.clone();
    assert!(seal_self_formed_r8b_packet_manifest_v3(manifest, &ledger, &c08).is_err());
}

#[test]
fn r8b_v8_n30_rejects_m17_duplicate_and_root_swap() {
    let (mut manifest, ledger, c08) = packet_fixture_v3();
    manifest.m17_receipt_roots_sha256[1] = manifest.m17_receipt_roots_sha256[0].clone();
    assert!(seal_self_formed_r8b_packet_manifest_v3(manifest, &ledger, &c08).is_err());
}

#[test]
fn r8b_v8_m17_wrapper_binds_exact_event_and_receipt_domains() {
    let receipts = roots_v3("m17-wrapper-receipt", 4);
    let census = nando_operator_learning::K2UncertaintyR8BMeasuredReceiptV2::seal(
        K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes,
        root_v1("m17-wrapper-route"),
        receipts.clone(),
        4,
        BTreeMap::new(),
        root_v1("m17-wrapper-producer"),
    )
    .unwrap();
    let wrapper =
        seal_self_formed_r8b_control_wrapper_v3(census, roots_v3("m17-wrapper-event", 4)).unwrap();
    validate_self_formed_r8b_control_wrapper_v3(&wrapper).unwrap();

    let mut swapped = wrapper.clone();
    swapped.completion_event_roots_sha256 = receipts;
    swapped.receipt_root_sha256.clear();
    swapped.receipt_root_sha256 = composition_root_v1(&swapped).unwrap();
    assert!(validate_self_formed_r8b_control_wrapper_v3(&swapped).is_err());
}

#[test]
fn r8b_v8_n31_rejects_writer_projection_crossing() {
    let (manifest, mut ledger, c08) = packet_fixture_v3();
    ledger.invocations[0].stage = "C08".to_owned();
    assert!(validate_self_formed_r8b_packet_manifest_v3(&manifest, &ledger, &c08).is_err());
}

#[test]
fn r8b_v8_projection_partitions_are_owner_target_disjoint() {
    let (manifest, mut ledger, c08) = packet_fixture_v3();
    ledger
        .invocations
        .iter_mut()
        .find(|row| {
            row.request_owner_role == "M10_PUBLIC_COORDINATOR"
                && row.target_role == "M09_CLOSURE_VERIFIER"
        })
        .unwrap()
        .stage = "C09".to_owned();
    validate_self_formed_r8b_packet_manifest_v3(&manifest, &ledger, &c08).unwrap();
    ledger
        .invocations
        .iter_mut()
        .find(|row| row.target_role == "M09_CLOSURE_VERIFIER")
        .unwrap()
        .request_owner_role = "M24_LINKED_RUNNER".to_owned();
    assert!(validate_self_formed_r8b_packet_manifest_v3(&manifest, &ledger, &c08).is_err());

    let (manifest, mut ledger, c08) = packet_fixture_v3();
    ledger
        .invocations
        .iter_mut()
        .find(|row| row.target_role == "M11_PRIVATE_RESOLVER")
        .unwrap()
        .request_owner_role = "M10_PUBLIC_COORDINATOR".to_owned();
    assert!(validate_self_formed_r8b_packet_manifest_v3(&manifest, &ledger, &c08).is_err());

    let (manifest, mut ledger, c08) = packet_fixture_v3();
    ledger
        .invocations
        .iter_mut()
        .find(|row| row.request_owner_role == "S03_MODE_MATRIX")
        .unwrap()
        .request_owner_role = "M10_PUBLIC_COORDINATOR".to_owned();
    assert!(validate_self_formed_r8b_packet_manifest_v3(&manifest, &ledger, &c08).is_err());
}

#[test]
fn r8b_v8_m10_formula_rejects_fact_schedule_mismatch() {
    let (manifest, mut ledger, c08) = packet_fixture_v3();
    *ledger.representative_counts.values_mut().next().unwrap() = 16;
    assert!(validate_self_formed_r8b_packet_manifest_v3(&manifest, &ledger, &c08).is_err());
}

#[test]
fn r8b_v8_n33_and_n34_reject_late_metrics_and_broad_stop() {
    let mut resources = resource_receipt_v3();
    resources.unit.metrics_frozen_while_loaded = false;
    assert!(seal_self_formed_r8b_resource_receipt_v3(resources).is_err());
    let mut resources = resource_receipt_v3();
    resources.unit.stop_target = "user.slice".to_owned();
    assert!(seal_self_formed_r8b_resource_receipt_v3(resources).is_err());
}

#[test]
fn r8b_v8_n35_requires_rustix_signal_transport() {
    let source = include_str!("k2_self_formed_uncertainty_confirm_r8b_restart_v1.rs");
    assert!(source.contains("rustix::process::kill_process"));
    assert!(!source.contains("Command::new(\"/bin/kill\")"));
}

#[test]
fn r8b_v8_n36_and_n41_reject_c08_omission_or_scope_drift() {
    let (mut manifest, ledger, c08) = packet_fixture_v3();
    manifest.members.retain(|row| {
        row.object_role != K2UncertaintyR8BObjectRoleV3::DownstreamInvocationContract
    });
    assert!(seal_self_formed_r8b_packet_manifest_v3(manifest, &ledger, &c08).is_err());
    let mut invalid = c08;
    invalid.invocations[0].stage = "C08".to_owned();
    invalid.projection_root_sha256.clear();
    assert!(seal_self_formed_r8b_downstream_contract_v3(invalid).is_err());
}

#[test]
fn r8b_v8_n37_and_n42_reject_manager_identity_drift() {
    let mut resources = resource_receipt_v3();
    resources.manager_post.start_ticks += 1;
    assert!(seal_self_formed_r8b_resource_receipt_v3(resources).is_err());
    let mut resources = resource_receipt_v3();
    resources.probe_pre.stdout_byte_len = 0;
    assert!(seal_self_formed_r8b_resource_receipt_v3(resources).is_err());
}

#[test]
fn r8b_v8_n38_and_n43_reject_systemd_or_probe_command_drift() {
    let mut contract = delegated_contract_v3();
    contract.normalized_argv.insert(1, "--wait".to_owned());
    assert!(validate_delegated_launch_v3(&contract).is_err());
    let mut resources = resource_receipt_v3();
    resources.probe_post.argv.push("--help".to_owned());
    assert!(seal_self_formed_r8b_resource_receipt_v3(resources).is_err());
}

#[test]
fn r8b_v8_resource_receipt_seals_two_channel_custody() {
    let resources = seal_self_formed_r8b_resource_receipt_v3(resource_receipt_v3()).unwrap();
    validate_self_formed_r8b_resource_receipt_v3(&resources).unwrap();
    assert_eq!(
        resources.authority,
        K2CompositionAuthorityBoundaryV1::denied()
    );
}

#[test]
fn r8b_v8_n39_rejects_packet_overwrite_during_ledger_freeze() {
    let environment = TestEnvironmentV1::new("v8-ledger-no-replace");
    let staging = environment.private_child("staging");
    let packet = environment.private_child("packet");
    fs::write(packet.join("process-ledger.json"), b"occupied").unwrap();
    let writer = K2UncertaintyR8BLedgerWriterV3::create(
        &staging,
        root_v1("route"),
        root_v1("projection"),
        schedule_authority_v3(),
    )
    .unwrap();
    let destination = packet.join("process-ledger.json");
    assert!(writer.freeze(&destination).is_err());
    let open = staging.join("process-ledger.open.jsonl");
    assert_eq!(
        fs::metadata(&open).unwrap().permissions().mode() & 0o7777,
        0o600
    );
    fs::remove_file(&destination).unwrap();
    writer.freeze(&destination).unwrap();
    assert!(!open.exists());
    assert_eq!(
        fs::metadata(&destination).unwrap().permissions().mode() & 0o7777,
        0o400
    );
}

#[test]
fn r8b_v8_ledger_freeze_recovers_rename_before_chmod() {
    let environment = TestEnvironmentV1::new("v8-ledger-post-rename-recovery");
    let staging = environment.private_child("staging");
    let packet = environment.private_child("packet");
    let destination = packet.join("process-ledger.json");
    let writer = K2UncertaintyR8BLedgerWriterV3::create(
        &staging,
        root_v1("route"),
        root_v1("projection"),
        schedule_authority_v3(),
    )
    .unwrap();
    let expected = writer.freeze(&destination).unwrap();
    fs::set_permissions(&destination, fs::Permissions::from_mode(0o600)).unwrap();
    let recovered = writer.freeze(&destination).unwrap();
    assert_eq!(recovered, expected);
    assert_eq!(
        fs::metadata(&destination).unwrap().permissions().mode() & 0o7777,
        0o400
    );
}

#[test]
fn r8b_v8_n40_rejects_oversized_path_before_event() {
    let mut request = producer_request_v3("S01_CRATE_UNIT", Vec::new());
    request.inputs[0].canonical_path = format!("/{}", "x".repeat(241));
    assert!(seal_self_formed_r8b_producer_request_v3(request).is_err());
}

fn producer_request_v3(
    role: &str,
    mut invocation_plan: Vec<K2UncertaintyR8BInvocationPlanV3>,
) -> K2UncertaintyR8BProducerRequestV3 {
    let producer = root_v1(&format!("producer-{role}"));
    if role != "S02_RESTART" {
        for (index, row) in invocation_plan.iter_mut().enumerate() {
            row.request_owner_role = role.to_owned();
            row.request_owner_executable_sha256 = producer.clone();
            if role != "M24_LINKED_RUNNER" {
                row.invocation_id_sha256 = root_v1(&format!("invocation-{role}-{index}"));
            }
        }
    }
    let inputs = [
        (K2UncertaintyR8BInputRoleV3::DevelopmentSeed, 0o400),
        (K2UncertaintyR8BInputRoleV3::FixtureTree, 0o500),
        (K2UncertaintyR8BInputRoleV3::LinkedManifest, 0o400),
        (K2UncertaintyR8BInputRoleV3::SuiteManifest, 0o400),
        (K2UncertaintyR8BInputRoleV3::ProcessLedger, 0o600),
        (K2UncertaintyR8BInputRoleV3::ExclusiveOutput, 0o700),
    ]
    .into_iter()
    .enumerate()
    .map(
        |(index, (input_role, unix_mode))| K2UncertaintyR8BInputBindingV3 {
            role: input_role,
            canonical_path: format!("/tmp/r8b-v8-input-{role}-{index}"),
            unix_mode,
            byte_len: 1,
            content_sha256: root_v1(&format!("input-content-{role}-{index}")),
            semantic_root_sha256: root_v1(&format!("input-semantic-{role}-{index}")),
        },
    )
    .collect();
    let kind = match role {
        "S02_RESTART" => K2UncertaintyR8BEvidenceKindV2::ProcessRestart,
        "S03_MODE_MATRIX" => K2UncertaintyR8BEvidenceKindV2::ModeMatrix,
        "S04_CLEANUP_NEGATIVE" => K2UncertaintyR8BEvidenceKindV2::CleanupInterruption,
        "S05_AUTHORITY_PUBLICATION" => K2UncertaintyR8BEvidenceKindV2::AggregatePublicationFaults,
        _ => K2UncertaintyR8BEvidenceKindV2::ConfirmCanonicalBytes,
    };
    let outputs = if role == "S01_CRATE_UNIT" {
        [
            (
                K2UncertaintyR8BEvidenceKindV2::ConfirmCanonicalBytes,
                "suites/s01/confirm-canonical.json",
            ),
            (
                K2UncertaintyR8BEvidenceKindV2::DevelopmentKnownAnswers,
                "suites/s01/development-known-answers.json",
            ),
            (
                K2UncertaintyR8BEvidenceKindV2::ImmutablePublication,
                "suites/s01/immutable-publication.json",
            ),
        ]
        .into_iter()
        .map(|(kind, path)| output_contract_v3(role, &producer, kind, path))
        .collect()
    } else if role == "M24_LINKED_RUNNER" {
        let mut outputs = [
            (
                K2UncertaintyR8BEvidenceKindV2::LinkedRoute,
                "linked/route.json",
            ),
            (
                K2UncertaintyR8BEvidenceKindV2::OracleCases,
                "linked/oracle-batch.json",
            ),
            (
                K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes,
                "linked/control-scopes.json",
            ),
        ]
        .into_iter()
        .map(|(kind, path)| output_contract_v3(role, &producer, kind, path))
        .collect::<Vec<_>>();
        for output in &mut outputs {
            match output.evidence_kind {
                Some(K2UncertaintyR8BEvidenceKindV2::OracleCases) => {
                    output.receipt_schema = K2_UNCERTAINTY_R8B_ORACLE_WRAPPER_SCHEMA_V3.to_owned();
                    output.required_source_roots_sha256 = roots_v3("m16-request-source", 16);
                }
                Some(K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes) => {
                    output.receipt_schema = K2_UNCERTAINTY_R8B_CONTROL_WRAPPER_SCHEMA_V3.to_owned();
                    output.required_source_roots_sha256 = roots_v3("m17-request-source", 4);
                }
                _ => {}
            }
        }
        outputs.push(K2UncertaintyR8BOutputContractV3 {
            relative_path: "downstream-invocations.json".to_owned(),
            object_role: K2UncertaintyR8BObjectRoleV3::DownstreamInvocationContract,
            evidence_kind: None,
            receipt_schema: "nando.k2-self-formed-r8b-downstream-contract.v3".to_owned(),
            required_denominator: None,
            required_source_roots_sha256: vec![root_v1("schedule-grammar")],
            producer_role: role.to_owned(),
            producer_executable_sha256: producer.clone(),
            validator: K2UncertaintyR8BValidatorV3::DownstreamInvocationContract,
            file_attestation: None,
        });
        outputs
    } else {
        let path = match role {
            "S02_RESTART" => "suites/s02/process-restart.json",
            "S03_MODE_MATRIX" => "suites/s03/mode-matrix.json",
            "S04_CLEANUP_NEGATIVE" => "suites/s04/cleanup-interruption.json",
            "S05_AUTHORITY_PUBLICATION" => "suites/s05/aggregate-publication-faults.json",
            _ => "suite-output.json",
        };
        vec![output_contract_v3(role, &producer, kind, path)]
    };
    K2UncertaintyR8BProducerRequestV3 {
        schema: String::new(),
        route_id_sha256: root_v1("v8-route"),
        producer_role: role.to_owned(),
        producer_executable_sha256: producer.clone(),
        test_selector: "r8b_v8_contract_test".to_owned(),
        inputs,
        outputs,
        invocation_plan,
        schedule_grammar_root_sha256: root_v1("schedule-grammar"),
        request_root_sha256: String::new(),
    }
}

fn output_contract_v3(
    role: &str,
    executable: &str,
    kind: K2UncertaintyR8BEvidenceKindV2,
    path: &str,
) -> K2UncertaintyR8BOutputContractV3 {
    K2UncertaintyR8BOutputContractV3 {
        relative_path: path.to_owned(),
        object_role: K2UncertaintyR8BObjectRoleV3::Evidence,
        evidence_kind: Some(kind),
        receipt_schema: kind.expected_schema().to_owned(),
        required_denominator: kind.required(),
        required_source_roots_sha256: vec![root_v1(&format!("source-{path}"))],
        producer_role: role.to_owned(),
        producer_executable_sha256: executable.to_owned(),
        validator: K2UncertaintyR8BValidatorV3::ConcreteReceipt,
        file_attestation: None,
    }
}

fn invocation_v3(index: usize, stage: &str, target: &str) -> K2UncertaintyR8BInvocationPlanV3 {
    K2UncertaintyR8BInvocationPlanV3 {
        invocation_id_sha256: root_v1(&format!("invocation-{stage}-{target}-{index:05}")),
        parent_invocation_id_sha256: None,
        request_owner_role: "M24_LINKED_RUNNER".to_owned(),
        request_owner_executable_sha256: root_v1("m24-executable"),
        target_role: target.to_owned(),
        target_executable_sha256: root_v1(&format!("target-{target}")),
        launch_kind: K2UncertaintyR8BLaunchKindV3::Direct,
        tool_chain: Vec::new(),
        stage: stage.to_owned(),
        case_id_sha256: None,
        probe_ordinal: None,
        expected_outcome: K2UncertaintyR8BExpectedOutcomeV3::AuthoritySuccess,
        validator: K2UncertaintyR8BValidatorV3::ConcreteReceipt,
    }
}

fn simple_plan_v3(count: usize) -> Vec<K2UncertaintyR8BInvocationPlanV3> {
    (0..count)
        .map(|index| invocation_v3(index, "C03", "M17_CONTROL_EVALUATOR"))
        .collect()
}

fn s02_plan_v3() -> Vec<K2UncertaintyR8BInvocationPlanV3> {
    let mut plan = Vec::new();
    for index in 0..10 {
        let mut row = invocation_v3(index, "C02", "M01_DEVELOPMENT_OWNER");
        row.request_owner_role = "S02_RESTART".to_owned();
        row.request_owner_executable_sha256 = root_v1("producer-S02_RESTART");
        plan.push(row);
    }
    for index in 10..13 {
        let mut row = invocation_v3(index, "C02", "M02_GENERATOR");
        row.request_owner_role = "S02_RESTART".to_owned();
        row.request_owner_executable_sha256 = root_v1("producer-S02_RESTART");
        plan.push(row);
    }
    for index in 13..16 {
        let mut row = invocation_v3(index, "C02", "M02_GENERATOR");
        row.request_owner_role = "M01_DEVELOPMENT_OWNER".to_owned();
        row.request_owner_executable_sha256 = root_v1("target-M01_DEVELOPMENT_OWNER");
        row.parent_invocation_id_sha256 = Some(plan[index - 13].invocation_id_sha256.clone());
        plan.push(row);
    }
    plan
}

fn terminal_event_v3(
    mut invocation: K2UncertaintyR8BInvocationPlanV3,
    completion: K2UncertaintyR8BCompletionKindV3,
) -> K2UncertaintyR8BProcessEventV3 {
    invocation.expected_outcome = K2UncertaintyR8BExpectedOutcomeV3::DiagnosticExpectedFailure;
    K2UncertaintyR8BProcessEventV3 {
        schema: String::new(),
        sequence: 1,
        previous_event_root_sha256: root_v1("previous"),
        route_id_sha256: root_v1("route"),
        invocation,
        request_root_sha256: root_v1("request"),
        stdin_sha256: root_v1("stdin"),
        started_event_root_sha256: Some(root_v1("started")),
        completion: Some(completion),
        exit_code: Some(1),
        stdout_byte_len: Some(1),
        stdout_sha256: Some(root_v1("stdout")),
        stderr_byte_len: Some(1),
        stderr_sha256: Some(root_v1("stderr")),
        validated_output: None,
        monotonic_ns: 2,
        event_root_sha256: String::new(),
    }
}

fn validated_output_v3(role: &str) -> K2UncertaintyR8BValidatedOutputV3 {
    let mut authority_output = output_contract_v3(
        role,
        &root_v1(&format!("target-{role}")),
        K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes,
        "controls.json",
    );
    authority_output.file_attestation = Some(K2UncertaintyR8BFileAttestationV3 {
        byte_len: 1,
        unix_mode: 0o400,
        content_sha256: root_v1("controls-content"),
        semantic_root_sha256: root_v1("controls-semantic"),
    });
    K2UncertaintyR8BValidatedOutputV3 {
        stdout_byte_len: 1,
        stdout_sha256: root_v1("stdout"),
        receipt_schema: "nando.test.v1".to_owned(),
        semantic_root_sha256: root_v1("receipt"),
        validator: K2UncertaintyR8BValidatorV3::ConcreteReceipt,
        validator_executable_sha256: root_v1("validator"),
        fact: K2UncertaintyR8BValidatedFactV3::None,
        authority_outputs: vec![authority_output],
    }
}

fn packet_fixture_v3() -> (
    K2UncertaintyR8BPacketManifestV3,
    K2UncertaintyR8BLedgerSummaryV3,
    K2UncertaintyR8BDownstreamContractV3,
) {
    #[rustfmt::skip]
    let counts = [
        ("M11_PRIVATE_RESOLVER", 24), ("M12_SAFETY", 24),
        ("M13_WORKER", 24), ("M14_OBSERVER", 24),
        ("M15_FINAL_VERIFIER", 16), ("M16_ORACLE", 16),
        ("M19_FRESH_CONTROL_CASE", 12), ("M17_CONTROL_EVALUATOR", 4),
        ("M18_TERMINAL_EVALUATOR", 1), ("M20_CLEANUP_AUTHORIZER", 1),
        ("M21_CLEANUP_OWNER", 1), ("M22_CLEANUP_VERIFIER", 1),
        ("M23_DEVELOPMENT_RESULT_PUBLISHER", 1),
    ];
    let invocations = counts
        .into_iter()
        .flat_map(|(role, count)| std::iter::repeat_n(role, count))
        .enumerate()
        .map(|(index, role)| invocation_v3(index, "C09", role))
        .collect();
    let c08 = seal_self_formed_r8b_downstream_contract_v3(K2UncertaintyR8BDownstreamContractV3 {
        schema: String::new(),
        route_id_sha256: root_v1("v8-route"),
        schedule_grammar_root_sha256: root_v1("schedule-grammar"),
        invocations,
        projection_root_sha256: String::new(),
    })
    .unwrap();
    let (projection, dynamic, representative_counts) = projection_fixture_v3(&c08, None);
    let m16_events = roots_v3("m16-event", 16);
    let m16_receipts = roots_v3("m16-receipt", 16);
    let m17_events = roots_v3("m17-event", 4);
    let m17_receipts = roots_v3("m17-receipt", 4);
    let mut members = K2UncertaintyR8BEvidenceKindV2::ALL
        .into_iter()
        .map(|kind| K2UncertaintyR8BPacketDescriptorV3 {
            relative_path: format!("evidence/{kind:?}.json").to_ascii_lowercase(),
            object_role: K2UncertaintyR8BObjectRoleV3::Evidence,
            evidence_kind: Some(kind),
            byte_len: 1,
            unix_mode: 0o400,
            content_sha256: root_v1(&format!("content-{kind:?}")),
            semantic_root_sha256: root_v1(&format!("semantic-{kind:?}")),
        })
        .collect::<Vec<_>>();
    members.extend([
        packet_descriptor_v3(
            "downstream-invocations.json",
            K2UncertaintyR8BObjectRoleV3::DownstreamInvocationContract,
            c08.projection_root_sha256.clone(),
        ),
        packet_descriptor_v3(
            "resource-receipt.json",
            K2UncertaintyR8BObjectRoleV3::ResourceReceipt,
            root_v1("resource"),
        ),
        packet_descriptor_v3(
            "process-ledger.json",
            K2UncertaintyR8BObjectRoleV3::ProcessLedger,
            root_v1("ledger-seal"),
        ),
    ]);
    let mut authority_outputs = Vec::new();
    for descriptor in members.iter().filter(|row| {
        row.object_role == K2UncertaintyR8BObjectRoleV3::DownstreamInvocationContract
            || row.evidence_kind.is_some_and(|kind| {
                !matches!(
                    kind,
                    K2UncertaintyR8BEvidenceKindV2::LinkedManifest
                        | K2UncertaintyR8BEvidenceKindV2::SuiteManifest
                        | K2UncertaintyR8BEvidenceKindV2::ProductionSurvival
                )
            })
    }) {
        let mut output = if let Some(kind) = descriptor.evidence_kind {
            output_contract_v3(
                "M24_LINKED_RUNNER",
                &root_v1("m24-executable"),
                kind,
                &descriptor.relative_path,
            )
        } else {
            K2UncertaintyR8BOutputContractV3 {
                relative_path: descriptor.relative_path.clone(),
                object_role: descriptor.object_role,
                evidence_kind: None,
                receipt_schema: "nando.k2-self-formed-r8b-downstream-contract.v3".to_owned(),
                required_denominator: None,
                required_source_roots_sha256: vec![root_v1("schedule-grammar")],
                producer_role: "M24_LINKED_RUNNER".to_owned(),
                producer_executable_sha256: root_v1("m24-executable"),
                validator: K2UncertaintyR8BValidatorV3::DownstreamInvocationContract,
                file_attestation: None,
            }
        };
        output.file_attestation = Some(K2UncertaintyR8BFileAttestationV3 {
            byte_len: descriptor.byte_len,
            unix_mode: descriptor.unix_mode,
            content_sha256: descriptor.content_sha256.clone(),
            semantic_root_sha256: descriptor.semantic_root_sha256.clone(),
        });
        authority_outputs.push((
            root_v1(&format!("event-{}", descriptor.relative_path)),
            output,
        ));
    }
    let mut invocations = projection.invocations.clone();
    invocations.extend(dynamic);
    invocations.extend(c08.invocations.clone());
    let mut request_roots_sha256 = invocations
        .iter()
        .map(|row| {
            (
                row.invocation_id_sha256.clone(),
                root_v1(&format!("request-{}", row.invocation_id_sha256)),
            )
        })
        .collect::<BTreeMap<_, _>>();
    request_roots_sha256.extend(projection.producer_request_roots_sha256.clone());
    let ledger = K2UncertaintyR8BLedgerSummaryV3 {
        route_id_sha256: root_v1("v8-route"),
        expected_projection_root_sha256: projection.projection_root_sha256,
        schedule_authority: schedule_authority_v3(),
        event_count: (invocations.len() * 2) as u64,
        final_event_root_sha256: root_v1("final-event"),
        seal_root_sha256: Some(root_v1("ledger-seal")),
        invocations,
        request_roots_sha256,
        representative_counts,
        authority_outputs,
        open_invocations: 0,
        fail_stopped: false,
        m16_event_roots_sha256: m16_events.iter().cloned().collect(),
        m16_receipt_roots_sha256: m16_receipts.iter().cloned().collect(),
        m17_event_roots_sha256: m17_events.iter().cloned().collect(),
        m17_receipt_roots_sha256: m17_receipts.iter().cloned().collect(),
    };
    let manifest = seal_self_formed_r8b_packet_manifest_v3(
        K2UncertaintyR8BPacketManifestV3 {
            schema: String::new(),
            route_id_sha256: ledger.route_id_sha256.clone(),
            c08_projection_root_sha256: c08.projection_root_sha256.clone(),
            resource_receipt_root_sha256: root_v1("resource"),
            ledger_seal_root_sha256: root_v1("ledger-seal"),
            ledger_event_count: ledger.event_count,
            m16_completion_event_roots_sha256: m16_events,
            m16_receipt_roots_sha256: m16_receipts,
            m17_completion_event_roots_sha256: m17_events,
            m17_receipt_roots_sha256: m17_receipts,
            members,
            manifest_root_sha256: String::new(),
        },
        &ledger,
        &c08,
    )
    .unwrap();
    (manifest, ledger, c08)
}

fn schedule_authority_v3() -> nando_operator_learning::K2UncertaintyR8BScheduleAuthorityV3 {
    seal_self_formed_r8b_schedule_authority_v3(
        root_v1("schedule-grammar"),
        (0..16)
            .map(|index| root_v1(&format!("schedule-case-{index}")))
            .collect(),
    )
    .unwrap()
}

fn projection_fixture_v3(
    c08: &K2UncertaintyR8BDownstreamContractV3,
    identities: Option<&BTreeMap<String, String>>,
) -> (
    nando_operator_learning::K2UncertaintyR8BStaticProjectionV3,
    Vec<K2UncertaintyR8BInvocationPlanV3>,
    BTreeMap<String, u64>,
) {
    let mut requests = [
        ("S01_CRATE_UNIT", Vec::new()),
        ("S02_RESTART", s02_plan_v3()),
        ("S03_MODE_MATRIX", simple_plan_v3(6)),
        ("S04_CLEANUP_NEGATIVE", simple_plan_v3(6)),
        ("S05_AUTHORITY_PUBLICATION", simple_plan_v3(2)),
    ]
    .into_iter()
    .map(|(role, plan)| {
        let mut request = producer_request_v3(role, plan);
        if let Some(identities) = identities {
            request.route_id_sha256 = c08.route_id_sha256.clone();
            bind_producer_request_identities_v3(&mut request, identities);
        }
        seal_self_formed_r8b_producer_request_v3(request).unwrap()
    })
    .collect::<Vec<_>>();
    let mut linked_plan = vec![
        invocation_v3(10_000, "C01", "M01_DEVELOPMENT_OWNER"),
        invocation_v3(10_001, "C06", "M10_PUBLIC_COORDINATOR"),
    ];
    linked_plan.extend(c08.invocations.clone());
    let mut linked_request = producer_request_v3("M24_LINKED_RUNNER", linked_plan);
    if let Some(identities) = identities {
        linked_request.route_id_sha256 = c08.route_id_sha256.clone();
        bind_producer_request_identities_v3(&mut linked_request, identities);
    }
    requests.push(seal_self_formed_r8b_producer_request_v3(linked_request).unwrap());
    let parent_launches = requests
        .iter()
        .enumerate()
        .map(|(index, request)| {
            let mut row = invocation_v3(11_000 + index, "P01", &request.producer_role);
            row.target_executable_sha256 = request.producer_executable_sha256.clone();
            if request.producer_role == "M24_LINKED_RUNNER" && identities.is_some() {
                row.launch_kind = K2UncertaintyR8BLaunchKindV3::UserSystemd;
                row.tool_chain = vec![K2UncertaintyR8BToolIdentityV3 {
                    role: K2UncertaintyR8BToolRoleV3::SystemdRun,
                    canonical_path: "/usr/bin/systemd-run".to_owned(),
                    sha256: root_v1("positive-systemd-run"),
                }];
            }
            if let Some(identities) = identities {
                bind_invocation_identities_v3(&mut row, identities);
            }
            row
        })
        .collect::<Vec<_>>();
    let projection =
        seal_self_formed_r8b_static_projection_v3(&requests, &parent_launches).unwrap();
    let authority = schedule_authority_v3();
    let representative_counts = authority
        .case_ids_sha256
        .iter()
        .cloned()
        .map(|case| (case, 8))
        .collect::<BTreeMap<_, _>>();
    let mut ordinal = 20_000;
    let mut dynamic = Vec::new();
    for case in &authority.case_ids_sha256 {
        for (role, count) in [
            ("M03_LEARNER", 1),
            ("M04_PROBE", 1),
            ("M05_SELECTOR", 1),
            ("M06_BASELINE", 4),
            ("M07_SELECTION_PREVERIFIER", 1),
            ("M08_CLOSURE_PLANNER", 1),
            ("M09_CLOSURE_VERIFIER", 1),
        ] {
            for _ in 0..count {
                let mut row = invocation_v3(ordinal, "C03", role);
                row.request_owner_role = "M10_PUBLIC_COORDINATOR".to_owned();
                row.request_owner_executable_sha256 = root_v1("target-M10_PUBLIC_COORDINATOR");
                row.case_id_sha256 = Some(case.clone());
                if role == "M04_PROBE" {
                    row.validator = K2UncertaintyR8BValidatorV3::RepresentativeCount;
                }
                if let Some(identities) = identities {
                    bind_invocation_identities_v3(&mut row, identities);
                }
                dynamic.push(row);
                ordinal += 1;
            }
        }
    }
    (projection, dynamic, representative_counts)
}

fn bind_producer_request_identities_v3(
    request: &mut K2UncertaintyR8BProducerRequestV3,
    identities: &BTreeMap<String, String>,
) {
    request.producer_executable_sha256 = identities[request.producer_role.as_str()].clone();
    for output in &mut request.outputs {
        output.producer_executable_sha256 = request.producer_executable_sha256.clone();
    }
    for invocation in &mut request.invocation_plan {
        bind_invocation_identities_v3(invocation, identities);
    }
}

fn bind_invocation_identities_v3(
    invocation: &mut K2UncertaintyR8BInvocationPlanV3,
    identities: &BTreeMap<String, String>,
) {
    invocation.request_owner_executable_sha256 =
        identities[invocation.request_owner_role.as_str()].clone();
    invocation.target_executable_sha256 = identities[invocation.target_role.as_str()].clone();
}

fn packet_descriptor_v3(
    path: &str,
    object_role: K2UncertaintyR8BObjectRoleV3,
    semantic_root_sha256: String,
) -> K2UncertaintyR8BPacketDescriptorV3 {
    K2UncertaintyR8BPacketDescriptorV3 {
        relative_path: path.to_owned(),
        object_role,
        evidence_kind: None,
        byte_len: 1,
        unix_mode: 0o400,
        content_sha256: root_v1(&format!("content-{path}")),
        semantic_root_sha256,
    }
}

fn roots_v3(label: &str, count: usize) -> Vec<String> {
    let mut roots = (0..count)
        .map(|index| root_v1(&format!("{label}-{index:02}")))
        .collect::<Vec<_>>();
    roots.sort();
    roots
}

fn delegated_contract_v3() -> DelegatedLaunchContractV3 {
    let route = root_v1("delegated-route");
    let mut contract = DelegatedLaunchContractV3 {
        route_id_sha256: route.clone(),
        unit: route_unit_v3(&route),
        request_owner_role: "M24_LINKED_RUNNER".to_owned(),
        child_owner: DelegatedChildOwnerV3::UserSystemdManager,
        systemd_run_sha256: root_v1("systemd-run"),
        child_executable: PathBuf::from("/tmp/r8b-child"),
        child_executable_sha256: root_v1("child"),
        credential_path: PathBuf::from("/tmp/r8b-credential"),
        stdout_path: PathBuf::from("/tmp/r8b-stdout"),
        stderr_path: PathBuf::from("/tmp/r8b-stderr"),
        selector: "r8b_v8_m24_linked_child".to_owned(),
        normalized_argv: Vec::new(),
    };
    contract.normalized_argv = delegated_launch_argv_v3(&contract);
    contract
}

fn resource_receipt_v3() -> K2UncertaintyR8BResourceReceiptV3 {
    let contract = delegated_contract_v3();
    let pid = 42;
    let image = root_v1("manager-image");
    let manager = K2UncertaintyR8BManagerIdentityV3 {
        bus_peer_pid: pid,
        bus_unique_name: ":1.42".to_owned(),
        pidfd_alive: true,
        boot_id: "00000000-0000-4000-8000-000000000001".to_owned(),
        start_ticks: 1,
        uid: 1000,
        command: vec!["/usr/lib/systemd/systemd".to_owned(), "--user".to_owned()],
        cgroup: "/user.slice/user-1000.slice/user@1000.service/init.scope".to_owned(),
        user_unit: "user@1000.service".to_owned(),
        invocation_id: "manager-invocation".to_owned(),
        main_pid: pid,
        exec_start: "/usr/lib/systemd/systemd".to_owned(),
        fragment_path: "/usr/lib/systemd/system/user@.service".to_owned(),
        control_group: "/user.slice/user-1000.slice/user@1000.service".to_owned(),
        version: "systemd 259 (259.5-0ubuntu3.4)".to_owned(),
    };
    let mut stdout = format!("{image} */proc/{pid}/exe").into_bytes();
    stdout.push(0);
    let probe = K2UncertaintyR8BPrivilegedProbeV3 {
        sudo_sha256: root_v1("sudo"),
        sha256sum_sha256: root_v1("sha256sum"),
        argv: privileged_probe_argv_v3(pid),
        exit_code: 0,
        stdout_byte_len: stdout.len() as u64,
        stdout_sha256: composition_sha256_bytes_v1(&stdout),
        stderr_byte_len: 0,
        stderr_sha256: composition_sha256_bytes_v1(&[]),
        live_image_sha256: image.clone(),
        started_monotonic_ns: 1,
        finished_monotonic_ns: 2,
    };
    let mut post_probe = probe.clone();
    post_probe.started_monotonic_ns = 5;
    post_probe.finished_monotonic_ns = 6;
    let unit = self_formed_r8b_route_unit_v3(&contract.route_id_sha256).unwrap();
    K2UncertaintyR8BResourceReceiptV3 {
        schema: String::new(),
        route_id_sha256: contract.route_id_sha256,
        delegated_launch_request_root_sha256: root_v1("delegated-launch-request"),
        normalized_systemd_run_argv: contract.normalized_argv,
        pinned_systemd_sha256: image,
        manager_pre: manager.clone(),
        manager_post: manager,
        probe_pre: probe,
        probe_post: post_probe,
        unit: K2UncertaintyR8BUnitResourceObservationV3 {
            unit: unit.clone(),
            invocation_id: "child-invocation".to_owned(),
            main_pid: 43,
            exec_main_code: "exited".to_owned(),
            exec_main_status: 0,
            active_state: "active".to_owned(),
            sub_state: "exited".to_owned(),
            metrics_frozen_while_loaded: true,
            memory_peak: 1,
            memory_swap_peak: 0,
            oom_policy: "stop".to_owned(),
            oom_kills: 0,
            tasks_current: 0,
            route_started_monotonic_ns: 3,
            route_finished_monotonic_ns: 4,
            stop_target: unit,
            stop_exit_code: 0,
            inactive_after_stop: true,
            descendants_after_stop: 0,
        },
        sudo_frontends: 2,
        sha256sum_descendants: 2,
        external_network_calls: 0,
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    }
}

fn authorize_positive_packet_v3() -> K2UncertaintyR8BAuthorizationReceiptV3 {
    POSITIVE_AUTHORIZATION_V3
        .get_or_init(build_positive_packet_authorization_v3)
        .clone()
}

fn build_positive_packet_authorization_v3() -> K2UncertaintyR8BAuthorizationReceiptV3 {
    let environment = TestEnvironmentV1::new("v8-positive-packet");
    let packet = environment.private_child("packet");
    let staging = environment.private_child("ledger-staging");
    let (linked_manifest, suite_manifest, identities) = positive_identity_manifests_v3();
    let route = root_v1("v8-positive-route");

    let mut c08_rows = downstream_rows_v3();
    for row in &mut c08_rows {
        bind_invocation_identities_v3(row, &identities);
    }
    let c08 = seal_self_formed_r8b_downstream_contract_v3(K2UncertaintyR8BDownstreamContractV3 {
        schema: String::new(),
        route_id_sha256: route.clone(),
        schedule_grammar_root_sha256: root_v1("schedule-grammar"),
        invocations: c08_rows,
        projection_root_sha256: String::new(),
    })
    .expect("positive C08 contract");
    let (projection, dynamic, representative_counts) =
        projection_fixture_v3(&c08, Some(&identities));
    assert_eq!(projection.route_id_sha256, route);

    let m24_parent = projection
        .invocations
        .iter()
        .find(|row| {
            row.target_role == "M24_LINKED_RUNNER"
                && projection
                    .producer_request_roots_sha256
                    .contains_key(&row.invocation_id_sha256)
        })
        .expect("positive M24 parent invocation");
    let m24_request_root =
        projection.producer_request_roots_sha256[&m24_parent.invocation_id_sha256].clone();
    let resource = positive_resource_receipt_v3(&route, &m24_request_root);
    let (oracle_batch, _, _) = r7j_terminal_evidence_v1();
    let experiment = oracle_batch.experiment_id_sha256.clone();
    let freeze = root_v1("positive-control-freeze");

    let control_specs = [
        (
            K2UncertaintyR8BEvidenceKindV2::LegacyControls,
            K2UncertaintyControlScopeV1::SuccessorStaticLegacy,
            "linked/legacy-controls.json",
        ),
        (
            K2UncertaintyR8BEvidenceKindV2::V3Controls,
            K2UncertaintyControlScopeV1::SuccessorStaticV3,
            "linked/v3-controls.json",
        ),
        (
            K2UncertaintyR8BEvidenceKindV2::V4Controls,
            K2UncertaintyControlScopeV1::SuccessorStaticV4,
            "linked/v4-controls.json",
        ),
        (
            K2UncertaintyR8BEvidenceKindV2::FreshControlCases,
            K2UncertaintyControlScopeV1::DevelopmentRehearsalV5,
            "linked/fresh-controls.json",
        ),
    ];
    let mut controls = Vec::new();
    let mut evidence = Vec::new();
    for (kind, scope, path) in control_specs {
        let control = pure_control_receipt_v3(
            scope,
            &experiment,
            (scope == K2UncertaintyControlScopeV1::DevelopmentRehearsalV5)
                .then_some(freeze.clone()),
            &identities["M17_CONTROL_EVALUATOR"],
        );
        evidence.push(PacketEvidenceV3::new(
            kind,
            path,
            &control,
            control.receipt_root_sha256.clone(),
            &control.schema,
            "M17_CONTROL_EVALUATOR",
            identities["M17_CONTROL_EVALUATOR"].clone(),
            Vec::new(),
        ));
        controls.push(control);
    }
    let mut control_roots = controls
        .iter()
        .map(|row| row.receipt_root_sha256.clone())
        .collect::<Vec<_>>();
    control_roots.sort();
    let control_census = K2UncertaintyR8BMeasuredReceiptV2::seal(
        K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes,
        route.clone(),
        control_roots.clone(),
        4,
        BTreeMap::new(),
        identities["M24_LINKED_RUNNER"].clone(),
    )
    .expect("positive M17 control census");

    let cleanup = positive_cleanup_receipt_v3(&identities["M22_CLEANUP_VERIFIER"]);
    let development = positive_development_receipt_v3(
        &identities["M23_DEVELOPMENT_RESULT_PUBLISHER"],
        &cleanup.receipt_root_sha256,
        &cleanup.terminal_receipt_root_sha256,
    );
    let linked_route = K2UncertaintyR8BMeasuredReceiptV2::seal(
        K2UncertaintyR8BEvidenceKindV2::LinkedRoute,
        route.clone(),
        vec![
            oracle_batch.receipt_root_sha256.clone(),
            cleanup.receipt_root_sha256.clone(),
            development.receipt_root_sha256.clone(),
        ],
        1,
        BTreeMap::from([("ordinary_fixture".to_owned(), 1)]),
        identities["M24_LINKED_RUNNER"].clone(),
    )
    .expect("positive linked route receipt");
    evidence.push(PacketEvidenceV3::new(
        K2UncertaintyR8BEvidenceKindV2::LinkedRoute,
        "linked/route.json",
        &linked_route,
        linked_route.receipt_root_sha256.clone(),
        &linked_route.schema,
        "M24_LINKED_RUNNER",
        identities["M24_LINKED_RUNNER"].clone(),
        linked_route.source_roots_sha256.clone(),
    ));
    evidence.push(PacketEvidenceV3::new(
        K2UncertaintyR8BEvidenceKindV2::CleanupTransaction,
        "linked/cleanup.json",
        &cleanup,
        cleanup.receipt_root_sha256.clone(),
        &cleanup.schema,
        "M22_CLEANUP_VERIFIER",
        identities["M22_CLEANUP_VERIFIER"].clone(),
        Vec::new(),
    ));
    evidence.push(PacketEvidenceV3::new(
        K2UncertaintyR8BEvidenceKindV2::DevelopmentResult,
        "linked/development-result.json",
        &development,
        development.receipt_root_sha256.clone(),
        &development.schema,
        "M23_DEVELOPMENT_RESULT_PUBLISHER",
        identities["M23_DEVELOPMENT_RESULT_PUBLISHER"].clone(),
        Vec::new(),
    ));

    for (kind, path, role) in [
        (
            K2UncertaintyR8BEvidenceKindV2::ConfirmCanonicalBytes,
            "suites/s01/confirm-canonical.json",
            "S01_CRATE_UNIT",
        ),
        (
            K2UncertaintyR8BEvidenceKindV2::DevelopmentKnownAnswers,
            "suites/s01/development-known-answers.json",
            "S01_CRATE_UNIT",
        ),
        (
            K2UncertaintyR8BEvidenceKindV2::ImmutablePublication,
            "suites/s01/immutable-publication.json",
            "S01_CRATE_UNIT",
        ),
        (
            K2UncertaintyR8BEvidenceKindV2::ProcessRestart,
            "suites/s02/process-restart.json",
            "S02_RESTART",
        ),
        (
            K2UncertaintyR8BEvidenceKindV2::ModeMatrix,
            "suites/s03/mode-matrix.json",
            "S03_MODE_MATRIX",
        ),
        (
            K2UncertaintyR8BEvidenceKindV2::CleanupInterruption,
            "suites/s04/cleanup-interruption.json",
            "S04_CLEANUP_NEGATIVE",
        ),
        (
            K2UncertaintyR8BEvidenceKindV2::AggregatePublicationFaults,
            "suites/s05/aggregate-publication-faults.json",
            "S05_AUTHORITY_PUBLICATION",
        ),
    ] {
        evidence.push(positive_measured_evidence_v3(
            kind,
            path,
            role,
            &route,
            &identities[role],
        ));
    }
    evidence.push(PacketEvidenceV3::new(
        K2UncertaintyR8BEvidenceKindV2::LinkedManifest,
        "manifests/linked-executables.json",
        &linked_manifest,
        linked_manifest.manifest_root_sha256.clone(),
        &linked_manifest.schema,
        "M24_LINKED_RUNNER",
        identities["M24_LINKED_RUNNER"].clone(),
        Vec::new(),
    ));
    evidence.push(PacketEvidenceV3::new(
        K2UncertaintyR8BEvidenceKindV2::SuiteManifest,
        "manifests/suite-executables.json",
        &suite_manifest,
        suite_manifest.manifest_root_sha256.clone(),
        &suite_manifest.schema,
        "M24_LINKED_RUNNER",
        identities["M24_LINKED_RUNNER"].clone(),
        Vec::new(),
    ));
    evidence.push(positive_measured_evidence_v3(
        K2UncertaintyR8BEvidenceKindV2::ProductionSurvival,
        "production/survival.json",
        "M24_LINKED_RUNNER",
        &route,
        &identities["M24_LINKED_RUNNER"],
    ));

    for row in &evidence {
        write_packet_evidence_v3(&packet, row);
    }
    let c08_bytes = uncertainty_bytes_v1(&c08).expect("positive C08 bytes");
    write_packet_file_v3(&packet, "downstream-invocations.json", &c08_bytes);
    let resource_bytes = uncertainty_bytes_v1(&resource).expect("positive resource bytes");
    write_packet_file_v3(&packet, "resource-receipt.json", &resource_bytes);

    let writer = K2UncertaintyR8BLedgerWriterV3::create(
        &staging,
        route.clone(),
        projection.projection_root_sha256.clone(),
        schedule_authority_v3(),
    )
    .expect("positive ledger writer");
    let parent_ids = projection
        .producer_request_roots_sha256
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut clock = 1_u64;
    let mut m24_started = None;
    for invocation in projection
        .invocations
        .iter()
        .filter(|row| parent_ids.contains(&row.invocation_id_sha256))
    {
        let request_root =
            projection.producer_request_roots_sha256[&invocation.invocation_id_sha256].clone();
        if invocation.target_role == "M24_LINKED_RUNNER" {
            m24_started = Some(append_requested_v3(
                &writer,
                invocation.clone(),
                request_root,
                &mut clock,
            ));
            continue;
        }
        let kinds = match invocation.target_role.as_str() {
            "S01_CRATE_UNIT" => vec![
                K2UncertaintyR8BEvidenceKindV2::ConfirmCanonicalBytes,
                K2UncertaintyR8BEvidenceKindV2::DevelopmentKnownAnswers,
                K2UncertaintyR8BEvidenceKindV2::ImmutablePublication,
            ],
            "S02_RESTART" => vec![K2UncertaintyR8BEvidenceKindV2::ProcessRestart],
            "S03_MODE_MATRIX" => vec![K2UncertaintyR8BEvidenceKindV2::ModeMatrix],
            "S04_CLEANUP_NEGATIVE" => vec![K2UncertaintyR8BEvidenceKindV2::CleanupInterruption],
            "S05_AUTHORITY_PUBLICATION" => {
                vec![K2UncertaintyR8BEvidenceKindV2::AggregatePublicationFaults]
            }
            _ => panic!("unexpected positive producer"),
        };
        let outputs = kinds
            .into_iter()
            .map(|kind| evidence_by_kind_v3(&evidence, kind).authority_output())
            .collect();
        append_success_v3(
            &writer,
            invocation.clone(),
            request_root,
            root_v1(&format!("producer-receipt-{}", invocation.target_role)),
            "nando.fixture-producer-receipt.v3",
            K2UncertaintyR8BValidatedFactV3::None,
            outputs,
            &mut clock,
        );
    }
    for invocation in projection
        .invocations
        .iter()
        .filter(|row| !parent_ids.contains(&row.invocation_id_sha256))
    {
        append_success_v3(
            &writer,
            invocation.clone(),
            root_v1(&format!("request-{}", invocation.invocation_id_sha256)),
            root_v1(&format!("receipt-{}", invocation.invocation_id_sha256)),
            "nando.fixture-static-receipt.v3",
            K2UncertaintyR8BValidatedFactV3::None,
            Vec::new(),
            &mut clock,
        );
    }
    for invocation in dynamic {
        let fact = if invocation.target_role == "M04_PROBE" {
            K2UncertaintyR8BValidatedFactV3::RepresentativeCount {
                count: representative_counts[invocation
                    .case_id_sha256
                    .as_ref()
                    .expect("M04 positive case")],
            }
        } else {
            K2UncertaintyR8BValidatedFactV3::None
        };
        append_success_v3(
            &writer,
            invocation.clone(),
            root_v1(&format!("request-{}", invocation.invocation_id_sha256)),
            root_v1(&format!("receipt-{}", invocation.invocation_id_sha256)),
            "nando.fixture-dynamic-receipt.v3",
            fact,
            Vec::new(),
            &mut clock,
        );
    }

    let mut oracle_index = 0_usize;
    let mut control_index = 0_usize;
    let mut oracle_events = Vec::new();
    let mut control_events = Vec::new();
    for invocation in &c08.invocations {
        let mut semantic_root = root_v1(&format!("receipt-{}", invocation.invocation_id_sha256));
        let mut schema = "nando.fixture-downstream-receipt.v3";
        let mut outputs = Vec::new();
        match invocation.target_role.as_str() {
            "M16_ORACLE" => {
                let receipt = &oracle_batch.case_receipts[oracle_index];
                semantic_root = receipt.receipt_root_sha256.clone();
                schema = &receipt.schema;
                oracle_index += 1;
            }
            "M17_CONTROL_EVALUATOR" => {
                let receipt = &controls[control_index];
                semantic_root = receipt.receipt_root_sha256.clone();
                schema = &receipt.schema;
                outputs.push(
                    evidence_by_kind_v3(&evidence, control_specs[control_index].0)
                        .authority_output(),
                );
                control_index += 1;
            }
            "M22_CLEANUP_VERIFIER" => {
                semantic_root = cleanup.receipt_root_sha256.clone();
                schema = &cleanup.schema;
                outputs.push(
                    evidence_by_kind_v3(
                        &evidence,
                        K2UncertaintyR8BEvidenceKindV2::CleanupTransaction,
                    )
                    .authority_output(),
                );
            }
            "M23_DEVELOPMENT_RESULT_PUBLISHER" => {
                semantic_root = development.receipt_root_sha256.clone();
                schema = &development.schema;
                outputs.push(
                    evidence_by_kind_v3(
                        &evidence,
                        K2UncertaintyR8BEvidenceKindV2::DevelopmentResult,
                    )
                    .authority_output(),
                );
            }
            _ => {}
        }
        let completed = append_success_v3(
            &writer,
            invocation.clone(),
            root_v1(&format!("request-{}", invocation.invocation_id_sha256)),
            semantic_root,
            schema,
            K2UncertaintyR8BValidatedFactV3::None,
            outputs,
            &mut clock,
        );
        match invocation.target_role.as_str() {
            "M16_ORACLE" => oracle_events.push(completed.event_root_sha256),
            "M17_CONTROL_EVALUATOR" => control_events.push(completed.event_root_sha256),
            _ => {}
        }
    }
    assert_eq!((oracle_index, control_index), (16, 4));

    let oracle_wrapper = seal_self_formed_r8b_oracle_wrapper_v3(oracle_batch, oracle_events)
        .expect("positive M16 wrapper");
    let control_wrapper = seal_self_formed_r8b_control_wrapper_v3(control_census, control_events)
        .expect("positive M17 wrapper");
    evidence.push(PacketEvidenceV3::new(
        K2UncertaintyR8BEvidenceKindV2::OracleCases,
        "linked/oracle-batch.json",
        &oracle_wrapper,
        oracle_wrapper.receipt_root_sha256.clone(),
        &oracle_wrapper.schema,
        "M24_LINKED_RUNNER",
        identities["M24_LINKED_RUNNER"].clone(),
        Vec::new(),
    ));
    evidence.push(PacketEvidenceV3::new(
        K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes,
        "linked/control-scopes.json",
        &control_wrapper,
        control_wrapper.receipt_root_sha256.clone(),
        &control_wrapper.schema,
        "M24_LINKED_RUNNER",
        identities["M24_LINKED_RUNNER"].clone(),
        control_roots,
    ));
    for kind in [
        K2UncertaintyR8BEvidenceKindV2::OracleCases,
        K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes,
    ] {
        write_packet_evidence_v3(&packet, evidence_by_kind_v3(&evidence, kind));
    }
    let mut m24_outputs = vec![downstream_output_v3(
        &c08,
        &c08_bytes,
        &identities["M24_LINKED_RUNNER"],
    )];
    m24_outputs.extend(
        [
            K2UncertaintyR8BEvidenceKindV2::LinkedRoute,
            K2UncertaintyR8BEvidenceKindV2::OracleCases,
            K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes,
        ]
        .into_iter()
        .map(|kind| evidence_by_kind_v3(&evidence, kind).authority_output()),
    );
    append_completion_v3(
        &writer,
        &m24_started.expect("positive M24 start"),
        root_v1("positive-m24-receipt"),
        "nando.fixture-m24-receipt.v3",
        K2UncertaintyR8BValidatedFactV3::None,
        m24_outputs,
        &mut clock,
    );

    let ledger = writer
        .freeze(&packet.join("process-ledger.json"))
        .expect("freeze positive ledger");
    assert_eq!(ledger.event_count, 696);
    assert_eq!(
        [
            ledger.m16_event_roots_sha256.len(),
            ledger.m16_receipt_roots_sha256.len(),
            ledger.m17_event_roots_sha256.len(),
            ledger.m17_receipt_roots_sha256.len(),
        ],
        [16, 16, 4, 4],
        "positive dual-root census",
    );
    let ledger_bytes = fs::read(packet.join("process-ledger.json")).expect("positive ledger bytes");
    let mut members = evidence
        .iter()
        .map(PacketEvidenceV3::descriptor)
        .collect::<Vec<_>>();
    members.extend([
        closed_descriptor_v3(
            "downstream-invocations.json",
            K2UncertaintyR8BObjectRoleV3::DownstreamInvocationContract,
            &c08_bytes,
            c08.projection_root_sha256.clone(),
        ),
        closed_descriptor_v3(
            "resource-receipt.json",
            K2UncertaintyR8BObjectRoleV3::ResourceReceipt,
            &resource_bytes,
            resource.receipt_root_sha256.clone(),
        ),
        closed_descriptor_v3(
            "process-ledger.json",
            K2UncertaintyR8BObjectRoleV3::ProcessLedger,
            &ledger_bytes,
            ledger
                .seal_root_sha256
                .clone()
                .expect("positive ledger seal"),
        ),
    ]);
    let manifest = seal_self_formed_r8b_packet_manifest_v3(
        K2UncertaintyR8BPacketManifestV3 {
            schema: String::new(),
            route_id_sha256: route.clone(),
            c08_projection_root_sha256: c08.projection_root_sha256.clone(),
            resource_receipt_root_sha256: resource.receipt_root_sha256.clone(),
            ledger_seal_root_sha256: ledger
                .seal_root_sha256
                .clone()
                .expect("positive ledger seal"),
            ledger_event_count: ledger.event_count,
            m16_completion_event_roots_sha256: ledger
                .m16_event_roots_sha256
                .iter()
                .cloned()
                .collect(),
            m16_receipt_roots_sha256: ledger.m16_receipt_roots_sha256.iter().cloned().collect(),
            m17_completion_event_roots_sha256: ledger
                .m17_event_roots_sha256
                .iter()
                .cloned()
                .collect(),
            m17_receipt_roots_sha256: ledger.m17_receipt_roots_sha256.iter().cloned().collect(),
            members,
            manifest_root_sha256: String::new(),
        },
        &ledger,
        &c08,
    )
    .expect("seal positive packet manifest");
    write_packet_file_v3(
        &packet,
        "packet-manifest.json",
        &uncertainty_bytes_v1(&manifest).expect("positive manifest bytes"),
    );
    freeze_directory_tree_v2(&packet);

    let request = K2UncertaintyR8BAuthorizationRequestV3::seal(
        route,
        manifest.manifest_root_sha256,
        identities["M25_R8B_AUTHORIZER"].clone(),
    )
    .expect("positive M25 request");
    let receipt =
        authorize_self_formed_r8b_v3(&request, &packet).expect("positive M25 authorization");
    assert_eq!(
        receipt.publisher_executable_sha256,
        identities["M26_R8B_PUBLISHER"]
    );
    receipt
}

fn positive_identity_manifests_v3() -> (
    K2UncertaintyR8BExecutableManifestV2,
    K2UncertaintyR8BExecutableManifestV2,
    BTreeMap<String, String>,
) {
    let linked = LinkedBinariesV2::from_cargo().manifest();
    let suite = K2UncertaintyR8BExecutableManifestV2::seal(
        K2UncertaintyR8BManifestClassV2::Suite,
        [
            "S01_CRATE_UNIT",
            "S02_RESTART",
            "S03_MODE_MATRIX",
            "S04_CLEANUP_NEGATIVE",
            "S05_AUTHORITY_PUBLICATION",
        ]
        .into_iter()
        .enumerate()
        .map(|(index, role)| K2UncertaintyR8BExecutableIdentityV2 {
            role: role.to_owned(),
            canonical_path: format!("/fixture/suite/{index:02}-{role}"),
            byte_len: index as u64 + 1,
            unix_mode: 0o555,
            sha256: root_v1(&format!("positive-suite-{role}")),
        })
        .collect(),
    )
    .expect("positive suite manifest");
    let identities = linked
        .identities
        .iter()
        .chain(&suite.identities)
        .map(|identity| (identity.role.clone(), identity.sha256.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(identities.len(), 31);
    (linked, suite, identities)
}

fn downstream_rows_v3() -> Vec<K2UncertaintyR8BInvocationPlanV3> {
    #[rustfmt::skip]
    let counts = [
        ("M11_PRIVATE_RESOLVER", 24), ("M12_SAFETY", 24),
        ("M13_WORKER", 24), ("M14_OBSERVER", 24),
        ("M15_FINAL_VERIFIER", 16), ("M16_ORACLE", 16),
        ("M19_FRESH_CONTROL_CASE", 12), ("M17_CONTROL_EVALUATOR", 4),
        ("M18_TERMINAL_EVALUATOR", 1), ("M20_CLEANUP_AUTHORIZER", 1),
        ("M21_CLEANUP_OWNER", 1), ("M22_CLEANUP_VERIFIER", 1),
        ("M23_DEVELOPMENT_RESULT_PUBLISHER", 1),
    ];
    counts
        .into_iter()
        .flat_map(|(role, count)| std::iter::repeat_n(role, count))
        .enumerate()
        .map(|(index, role)| invocation_v3(index, "C09", role))
        .collect()
}

fn pure_control_receipt_v3(
    scope: K2UncertaintyControlScopeV1,
    experiment: &str,
    freeze: Option<String>,
    evaluator_sha256: &str,
) -> nando_operator_learning::K2UncertaintyControlEvaluationReceiptV1 {
    let outcomes = (0..scope.expected_count())
        .map(|ordinal| {
            let (control_id, disposition) =
                expected_self_formed_control_v1(scope, ordinal).unwrap();
            let stdout = uncertainty_bytes_v1(&K2UncertaintyControlStdoutV1 {
                control_id: control_id.clone(),
                disposition: disposition.clone(),
            })
            .unwrap();
            K2UncertaintyControlProcessOutcomeV1::seal(
                scope,
                control_id,
                experiment.to_owned(),
                freeze.clone(),
                None,
                root_v1(&format!("positive-control-runner-{scope:?}")),
                root_v1(&format!("positive-control-test-{scope:?}-{ordinal}")),
                root_v1(&format!("positive-control-request-{scope:?}-{ordinal}")),
                true,
                0,
                stdout,
                composition_sha256_bytes_v1(&[]),
                false,
                false,
                disposition,
                root_v1(&format!("positive-control-source-{scope:?}-{ordinal}")),
                root_v1(&format!("positive-control-log-{scope:?}-{ordinal}")),
            )
            .unwrap()
        })
        .collect();
    let request = K2UncertaintyControlEvaluationRequestV1::seal(
        scope,
        experiment.to_owned(),
        freeze,
        None,
        outcomes,
        evaluator_sha256.to_owned(),
    )
    .expect("positive pure control request");
    evaluate_self_formed_controls_v1(&request).expect("positive pure control receipt")
}

fn positive_cleanup_receipt_v3(verifier_sha256: &str) -> K2UncertaintyCleanupReceiptV1 {
    let mut value = K2UncertaintyCleanupReceiptV1 {
        schema: K2_UNCERTAINTY_CLEANUP_RECEIPT_SCHEMA_V1.to_owned(),
        request_root_sha256: root_v1("positive-cleanup-request"),
        before_manifest_root_sha256: root_v1("positive-cleanup-before"),
        terminal_receipt_root_sha256: root_v1("positive-terminal-receipt"),
        after_census_root_sha256: root_v1("positive-cleanup-after"),
        control_manifest_root_sha256: root_v1("positive-cleanup-control"),
        owner_receipt_root_sha256: root_v1("positive-cleanup-owner"),
        retained_paths: 1,
        deleted_paths: 1,
        unexpected_residue: 0,
        cleanup_frozen: true,
        verifier_executable_sha256: verifier_sha256.to_owned(),
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    };
    value.receipt_root_sha256 = uncertainty_root_v1(&(
        K2_UNCERTAINTY_CLEANUP_RECEIPT_SCHEMA_V1,
        &value.request_root_sha256,
        &value.before_manifest_root_sha256,
        &value.terminal_receipt_root_sha256,
        &value.after_census_root_sha256,
        &value.control_manifest_root_sha256,
        &value.owner_receipt_root_sha256,
        value.retained_paths,
        value.deleted_paths,
        value.unexpected_residue,
        value.cleanup_frozen,
        &value.verifier_executable_sha256,
        &value.authority,
    ))
    .unwrap();
    value.validate().expect("positive cleanup receipt");
    value
}

fn positive_development_receipt_v3(
    publisher_sha256: &str,
    cleanup_root_sha256: &str,
    terminal_root_sha256: &str,
) -> K2UncertaintyDevelopmentResultReceiptV1 {
    let mut value = K2UncertaintyDevelopmentResultReceiptV1 {
        schema: K2_UNCERTAINTY_DEVELOPMENT_RESULT_RECEIPT_SCHEMA_V1.to_owned(),
        request_root_sha256: root_v1("positive-development-request"),
        terminal_receipt_root_sha256: terminal_root_sha256.to_owned(),
        cleanup_receipt_root_sha256: cleanup_root_sha256.to_owned(),
        disposition: "DEVELOPMENT_REHEARSAL_COMPLETE".to_owned(),
        publisher_executable_sha256: publisher_sha256.to_owned(),
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    };
    value.receipt_root_sha256 = uncertainty_root_v1(&(
        K2_UNCERTAINTY_DEVELOPMENT_RESULT_RECEIPT_SCHEMA_V1,
        &value.request_root_sha256,
        &value.terminal_receipt_root_sha256,
        &value.cleanup_receipt_root_sha256,
        &value.disposition,
        &value.publisher_executable_sha256,
        &value.authority,
    ))
    .unwrap();
    value
        .validate()
        .expect("positive Development result receipt");
    value
}

fn positive_measured_evidence_v3(
    kind: K2UncertaintyR8BEvidenceKindV2,
    path: &str,
    role: &str,
    route: &str,
    producer_sha256: &str,
) -> PacketEvidenceV3 {
    let sources = vec![root_v1(&format!("positive-source-{kind:?}"))];
    let receipt = K2UncertaintyR8BMeasuredReceiptV2::seal(
        kind,
        route.to_owned(),
        sources.clone(),
        kind.required().unwrap_or(1),
        BTreeMap::new(),
        producer_sha256.to_owned(),
    )
    .expect("positive measured evidence");
    PacketEvidenceV3::new(
        kind,
        path,
        &receipt,
        receipt.receipt_root_sha256.clone(),
        &receipt.schema,
        role,
        producer_sha256.to_owned(),
        sources,
    )
}

fn positive_resource_receipt_v3(
    route: &str,
    delegated_request_root: &str,
) -> K2UncertaintyR8BResourceReceiptV3 {
    let mut value = resource_receipt_v3();
    let unit = self_formed_r8b_route_unit_v3(route).expect("positive resource unit");
    value.route_id_sha256 = route.to_owned();
    value.delegated_launch_request_root_sha256 = delegated_request_root.to_owned();
    value.normalized_systemd_run_argv[4] = format!("--unit={unit}");
    value.unit.unit = unit.clone();
    value.unit.stop_target = unit;
    seal_self_formed_r8b_resource_receipt_v3(value).expect("positive resource receipt")
}

fn evidence_by_kind_v3(
    evidence: &[PacketEvidenceV3],
    kind: K2UncertaintyR8BEvidenceKindV2,
) -> &PacketEvidenceV3 {
    evidence
        .iter()
        .find(|row| row.kind == kind)
        .expect("positive evidence kind")
}

fn write_packet_evidence_v3(root: &Path, evidence: &PacketEvidenceV3) {
    write_packet_file_v3(root, &evidence.relative_path, &evidence.bytes);
}

fn write_packet_file_v3(root: &Path, relative_path: &str, bytes: &[u8]) {
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create positive packet parent");
    }
    write_new_read_only_v2(&path, bytes);
}

fn closed_descriptor_v3(
    relative_path: &str,
    object_role: K2UncertaintyR8BObjectRoleV3,
    bytes: &[u8],
    semantic_root_sha256: String,
) -> K2UncertaintyR8BPacketDescriptorV3 {
    K2UncertaintyR8BPacketDescriptorV3 {
        relative_path: relative_path.to_owned(),
        object_role,
        evidence_kind: None,
        byte_len: bytes.len() as u64,
        unix_mode: 0o400,
        content_sha256: composition_sha256_bytes_v1(bytes),
        semantic_root_sha256,
    }
}

fn downstream_output_v3(
    c08: &K2UncertaintyR8BDownstreamContractV3,
    bytes: &[u8],
    producer_sha256: &str,
) -> K2UncertaintyR8BOutputContractV3 {
    K2UncertaintyR8BOutputContractV3 {
        relative_path: "downstream-invocations.json".to_owned(),
        object_role: K2UncertaintyR8BObjectRoleV3::DownstreamInvocationContract,
        evidence_kind: None,
        receipt_schema: K2_UNCERTAINTY_R8B_DOWNSTREAM_CONTRACT_SCHEMA_V3.to_owned(),
        required_denominator: None,
        required_source_roots_sha256: vec![c08.schedule_grammar_root_sha256.clone()],
        producer_role: "M24_LINKED_RUNNER".to_owned(),
        producer_executable_sha256: producer_sha256.to_owned(),
        validator: K2UncertaintyR8BValidatorV3::DownstreamInvocationContract,
        file_attestation: Some(K2UncertaintyR8BFileAttestationV3 {
            byte_len: bytes.len() as u64,
            unix_mode: 0o400,
            content_sha256: composition_sha256_bytes_v1(bytes),
            semantic_root_sha256: c08.projection_root_sha256.clone(),
        }),
    }
}

fn append_requested_v3(
    writer: &K2UncertaintyR8BLedgerWriterV3,
    invocation: K2UncertaintyR8BInvocationPlanV3,
    request_root_sha256: String,
    clock: &mut u64,
) -> K2UncertaintyR8BProcessEventV3 {
    let monotonic_ns = *clock;
    *clock += 1;
    writer
        .append(K2UncertaintyR8BProcessEventV3 {
            schema: String::new(),
            sequence: 0,
            previous_event_root_sha256: root_v1("pending-event"),
            route_id_sha256: root_v1("pending-route"),
            stdin_sha256: root_v1(&format!("stdin-{}", invocation.invocation_id_sha256)),
            invocation,
            request_root_sha256,
            started_event_root_sha256: None,
            completion: None,
            exit_code: None,
            stdout_byte_len: None,
            stdout_sha256: None,
            stderr_byte_len: None,
            stderr_sha256: None,
            validated_output: None,
            monotonic_ns,
            event_root_sha256: String::new(),
        })
        .expect("append positive invocation request")
}

fn append_success_v3(
    writer: &K2UncertaintyR8BLedgerWriterV3,
    invocation: K2UncertaintyR8BInvocationPlanV3,
    request_root_sha256: String,
    semantic_root_sha256: String,
    receipt_schema: &str,
    fact: K2UncertaintyR8BValidatedFactV3,
    authority_outputs: Vec<K2UncertaintyR8BOutputContractV3>,
    clock: &mut u64,
) -> K2UncertaintyR8BProcessEventV3 {
    let started = append_requested_v3(writer, invocation, request_root_sha256, clock);
    append_completion_v3(
        writer,
        &started,
        semantic_root_sha256,
        receipt_schema,
        fact,
        authority_outputs,
        clock,
    )
}

fn append_completion_v3(
    writer: &K2UncertaintyR8BLedgerWriterV3,
    started: &K2UncertaintyR8BProcessEventV3,
    semantic_root_sha256: String,
    receipt_schema: &str,
    fact: K2UncertaintyR8BValidatedFactV3,
    authority_outputs: Vec<K2UncertaintyR8BOutputContractV3>,
    clock: &mut u64,
) -> K2UncertaintyR8BProcessEventV3 {
    let stdout = uncertainty_bytes_v1(&(receipt_schema, &semantic_root_sha256))
        .expect("positive stdout bytes");
    let stdout_sha256 = composition_sha256_bytes_v1(&stdout);
    let monotonic_ns = *clock;
    *clock += 1;
    writer
        .append(K2UncertaintyR8BProcessEventV3 {
            schema: String::new(),
            sequence: 0,
            previous_event_root_sha256: root_v1("pending-event"),
            route_id_sha256: root_v1("pending-route"),
            invocation: started.invocation.clone(),
            request_root_sha256: started.request_root_sha256.clone(),
            stdin_sha256: started.stdin_sha256.clone(),
            started_event_root_sha256: Some(started.event_root_sha256.clone()),
            completion: Some(K2UncertaintyR8BCompletionKindV3::AuthoritySuccess),
            exit_code: Some(0),
            stdout_byte_len: Some(stdout.len() as u64),
            stdout_sha256: Some(stdout_sha256.clone()),
            stderr_byte_len: Some(0),
            stderr_sha256: Some(composition_sha256_bytes_v1(&[])),
            validated_output: Some(K2UncertaintyR8BValidatedOutputV3 {
                stdout_byte_len: stdout.len() as u64,
                stdout_sha256,
                receipt_schema: receipt_schema.to_owned(),
                semantic_root_sha256,
                validator: started.invocation.validator,
                validator_executable_sha256: started.invocation.target_executable_sha256.clone(),
                fact,
                authority_outputs,
            }),
            monotonic_ns,
            event_root_sha256: String::new(),
        })
        .expect("append positive invocation completion")
}

fn component_authorization_v3(publisher_sha: &str) -> K2UncertaintyR8BAuthorizationReceiptV3 {
    let mut value = K2UncertaintyR8BAuthorizationReceiptV3 {
        schema: K2_UNCERTAINTY_R8B_AUTHORIZATION_RECEIPT_SCHEMA_V3.to_owned(),
        request_root_sha256: root_v1("component-request"),
        route_id_sha256: root_v1("component-route"),
        manifest_root_sha256: root_v1("component-manifest"),
        c08_projection_root_sha256: root_v1("component-c08"),
        resource_receipt_root_sha256: root_v1("component-resource"),
        ledger_seal_root_sha256: root_v1("component-ledger-seal"),
        packet_member_roots_sha256: roots_v3("component-member", 22),
        publisher_executable_sha256: publisher_sha.to_owned(),
        disposition: "R8B_FROZEN".to_owned(),
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    };
    value.receipt_root_sha256 = uncertainty_root_v1(&value).expect("component auth root");
    value.validate().expect("component authorization bytes");
    value
}
