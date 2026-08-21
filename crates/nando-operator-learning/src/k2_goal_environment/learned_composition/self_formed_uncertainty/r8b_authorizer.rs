use std::collections::{BTreeMap, BTreeSet};
use std::io::BufReader;
use std::path::Path;

use super::immutable_publication::closed_tree_paths_v2;
use super::immutable_publication::decode_canonical_json_v1;
use super::immutable_publication::open_closed_file_v2;
use super::immutable_publication::read_closed_file_v2;
use super::immutable_publication::read_closed_json_v2;

use super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, require_composition_root_v1, valid_composition_path_v1,
};
use super::{
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2_UNCERTAINTY_R8B_AUTHORIZATION_RECEIPT_SCHEMA_V3,
    K2_UNCERTAINTY_R8B_MAX_LEDGER_BYTES_V3, K2_UNCERTAINTY_R8B_PACKET_MANIFEST_SCHEMA_V3,
    K2UncertaintyR8BAuthorizationReceiptV3, K2UncertaintyR8BAuthorizationRequestV3, K2UncertaintyR8BControlWrapperV3,
    K2UncertaintyR8BDownstreamContractV3, K2UncertaintyR8BEvidenceKindV2 as EvidenceKind,
    K2UncertaintyR8BExecutableManifestV2, K2UncertaintyR8BLedgerSummaryV3,
    K2UncertaintyR8BManifestClassV2 as ManifestClass, K2UncertaintyR8BObjectRoleV3 as ObjectRole,
    K2UncertaintyR8BOracleWrapperV3, K2UncertaintyR8BOutputContractV3, K2UncertaintyR8BPacketDescriptorV3,
    K2UncertaintyR8BPacketManifestV3, K2UncertaintyR8BResourceReceiptV3, decode_self_formed_r8b_evidence_view_v3,
    denied_authority_v1, uncertainty_root_v1, validate_self_formed_r8b_control_wrapper_v3,
    validate_self_formed_r8b_delegated_resource_v3, validate_self_formed_r8b_downstream_contract_v3,
    validate_self_formed_r8b_ledger_stream_attested_v3, validate_self_formed_r8b_oracle_wrapper_v3,
    validate_self_formed_r8b_process_projections_v3,
};
const PACKET_MANIFEST_PATH_V3: &str = "packet-manifest.json";
const PROCESS_LEDGER_PATH_V3: &str = "process-ledger.json";

pub fn validate_self_formed_r8b_packet_manifest_v3(
    manifest: &K2UncertaintyR8BPacketManifestV3,
    ledger: &K2UncertaintyR8BLedgerSummaryV3,
    c08: &K2UncertaintyR8BDownstreamContractV3,
) -> K2CompositionResultV1<()> {
    validate_manifest_shape_v3(manifest)?;
    validate_self_formed_r8b_downstream_contract_v3(c08)?;
    reject(
        manifest.schema != K2_UNCERTAINTY_R8B_PACKET_MANIFEST_SCHEMA_V3
            || manifest.route_id_sha256 != ledger.route_id_sha256
            || manifest.route_id_sha256 != c08.route_id_sha256
            || manifest.c08_projection_root_sha256 != c08.projection_root_sha256
            || manifest.ledger_event_count != ledger.event_count
            || ledger.seal_root_sha256.as_ref() != Some(&manifest.ledger_seal_root_sha256),
        "self_formed_r8b_v3_packet_binding_invalid",
    )?;
    validate_self_formed_r8b_process_projections_v3(ledger, c08)?;
    validate_dual_roots_v3(manifest, ledger)?;
    validate_descriptor_attestations_v3(manifest, ledger)?;
    Ok(())
}

pub fn authorize_self_formed_r8b_v3(
    request: &K2UncertaintyR8BAuthorizationRequestV3,
    packet_root: &Path,
) -> K2CompositionResultV1<K2UncertaintyR8BAuthorizationReceiptV3> {
    request.validate()?;
    let manifest: K2UncertaintyR8BPacketManifestV3 = read_closed_json_v2(
        &packet_root.join(PACKET_MANIFEST_PATH_V3),
        K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1,
        None,
        None,
    )?;
    validate_manifest_shape_v3(&manifest)?;
    reject(
        manifest.route_id_sha256 != request.route_id_sha256
            || manifest.manifest_root_sha256 != request.manifest_root_sha256,
        "self_formed_r8b_v3_manifest_envelope_invalid",
    )?;
    let expected_paths = manifest
        .members
        .iter()
        .map(|row| row.relative_path.clone())
        .chain(std::iter::once(PACKET_MANIFEST_PATH_V3.to_owned()))
        .collect::<BTreeSet<_>>();
    reject(
        expected_paths.len() != 23 || closed_tree_paths_v2(packet_root)? != expected_paths,
        "self_formed_r8b_v3_packet_path_set_invalid",
    )?;

    let ledger_descriptor = descriptor_v3(&manifest, DescriptorKeyV3::Role(ObjectRole::ProcessLedger))?;
    let ledger_file = open_closed_file_v2(
        &packet_root.join(&ledger_descriptor.relative_path),
        K2_UNCERTAINTY_R8B_MAX_LEDGER_BYTES_V3,
        Some(ledger_descriptor.byte_len),
    )?;
    let (ledger, ledger_len, ledger_sha256) =
        validate_self_formed_r8b_ledger_stream_attested_v3(BufReader::new(ledger_file), true)?;
    reject(
        ledger_len != ledger_descriptor.byte_len
            || ledger_sha256 != ledger_descriptor.content_sha256
            || ledger.seal_root_sha256.as_ref() != Some(&ledger_descriptor.semantic_root_sha256),
        "self_formed_r8b_v3_ledger_attestation_invalid",
    )?;

    let c08_descriptor = descriptor_v3(&manifest, DescriptorKeyV3::Role(ObjectRole::DownstreamInvocationContract))?;
    let c08: K2UncertaintyR8BDownstreamContractV3 = read_descriptor_json_v3(packet_root, c08_descriptor)?;
    validate_self_formed_r8b_packet_manifest_v3(&manifest, &ledger, &c08)?;

    let resource_descriptor = descriptor_v3(&manifest, DescriptorKeyV3::Role(ObjectRole::ResourceReceipt))?;
    let resource: K2UncertaintyR8BResourceReceiptV3 = read_descriptor_json_v3(packet_root, resource_descriptor)?;
    validate_self_formed_r8b_delegated_resource_v3(&ledger, &resource)?;
    reject(
        resource.route_id_sha256 != manifest.route_id_sha256
            || resource.receipt_root_sha256 != resource_descriptor.semantic_root_sha256,
        "self_formed_r8b_v3_resource_binding_invalid",
    )?;
    let linked = read_identity_manifest_v3(
        packet_root,
        descriptor_v3(&manifest, DescriptorKeyV3::Kind(EvidenceKind::LinkedManifest))?,
        ManifestClass::Linked,
    )?;
    let suite = read_identity_manifest_v3(
        packet_root,
        descriptor_v3(&manifest, DescriptorKeyV3::Kind(EvidenceKind::SuiteManifest))?,
        ManifestClass::Suite,
    )?;
    let identities = linked
        .identities
        .iter()
        .chain(&suite.identities)
        .map(|identity| (identity.role.as_str(), identity.sha256.as_str()))
        .collect::<BTreeMap<_, _>>();
    reject(
        identities.get("M25_R8B_AUTHORIZER") != Some(&request.authorizer_executable_sha256.as_str()),
        "self_formed_r8b_v3_authorizer_identity_invalid",
    )?;
    for invocation in &ledger.invocations {
        reject(
            identities.get(invocation.target_role.as_str()) != Some(&invocation.target_executable_sha256.as_str())
                || identities.get(invocation.request_owner_role.as_str())
                    != Some(&invocation.request_owner_executable_sha256.as_str()),
            "self_formed_r8b_v3_process_identity_invalid",
        )?;
    }
    for (_, output) in &ledger.authority_outputs {
        reject(
            identities.get(output.producer_role.as_str()) != Some(&output.producer_executable_sha256.as_str()),
            "self_formed_r8b_v3_output_identity_invalid",
        )?;
    }
    for descriptor in manifest.members.iter().filter(|row| {
        row.object_role == ObjectRole::Evidence
            && !matches!(row.evidence_kind, Some(EvidenceKind::LinkedManifest | EvidenceKind::SuiteManifest))
    }) {
        validate_evidence_v3(packet_root, descriptor, &manifest, &ledger, &identities)?;
    }
    let publisher =
        identities.get("M26_R8B_PUBLISHER").ok_or_else(|| invalid("self_formed_r8b_v3_publisher_identity_missing"))?;
    let mut packet_member_roots_sha256 =
        manifest.members.iter().map(|row| row.semantic_root_sha256.clone()).collect::<Vec<_>>();
    packet_member_roots_sha256.sort();
    let mut receipt = K2UncertaintyR8BAuthorizationReceiptV3 {
        schema: K2_UNCERTAINTY_R8B_AUTHORIZATION_RECEIPT_SCHEMA_V3.to_owned(),
        request_root_sha256: request.request_root_sha256.clone(),
        route_id_sha256: manifest.route_id_sha256.clone(),
        manifest_root_sha256: manifest.manifest_root_sha256.clone(),
        c08_projection_root_sha256: manifest.c08_projection_root_sha256.clone(),
        resource_receipt_root_sha256: manifest.resource_receipt_root_sha256.clone(),
        ledger_seal_root_sha256: manifest.ledger_seal_root_sha256.clone(),
        packet_member_roots_sha256,
        publisher_executable_sha256: (*publisher).to_owned(),
        disposition: "R8B_FROZEN".to_owned(),
        authority: denied_authority_v1(),
        receipt_root_sha256: String::new(),
    };
    receipt.receipt_root_sha256 = uncertainty_root_v1(&receipt)?;
    receipt.validate()?;
    Ok(receipt)
}

fn validate_manifest_shape_v3(manifest: &K2UncertaintyR8BPacketManifestV3) -> K2CompositionResultV1<()> {
    for root in [
        &manifest.route_id_sha256,
        &manifest.c08_projection_root_sha256,
        &manifest.resource_receipt_root_sha256,
        &manifest.ledger_seal_root_sha256,
    ] {
        require_composition_root_v1(root)?;
    }
    let mut paths = BTreeSet::new();
    let mut kinds = BTreeSet::new();
    let mut roles = [0_usize; 4];
    for descriptor in &manifest.members {
        validate_packet_descriptor_v3(descriptor)?;
        roles[descriptor.object_role as usize] += 1;
        reject(
            !paths.insert(&descriptor.relative_path)
                || descriptor.evidence_kind.is_some_and(|kind| !kinds.insert(kind)),
            "self_formed_r8b_v3_packet_member_duplicate",
        )?;
    }
    let mut canonical = manifest.clone();
    canonical.manifest_root_sha256.clear();
    reject(
        manifest.schema != K2_UNCERTAINTY_R8B_PACKET_MANIFEST_SCHEMA_V3
            || manifest.members.len() != 22
            || roles != [19, 1, 1, 1]
            || kinds != EvidenceKind::ALL.into_iter().collect()
            || !manifest.members.windows(2).all(|pair| {
                (&pair[0].relative_path, pair[0].object_role as u8)
                    < (&pair[1].relative_path, pair[1].object_role as u8)
            })
            || manifest.manifest_root_sha256 != uncertainty_root_v1(&canonical)?,
        "self_formed_r8b_v3_manifest_shape_invalid",
    )?;
    for (role, root, path) in [
        (ObjectRole::DownstreamInvocationContract, manifest.c08_projection_root_sha256.as_str(), None),
        (ObjectRole::ResourceReceipt, manifest.resource_receipt_root_sha256.as_str(), None),
        (ObjectRole::ProcessLedger, manifest.ledger_seal_root_sha256.as_str(), Some(PROCESS_LEDGER_PATH_V3)),
    ] {
        require_special_descriptor_v3(manifest, role, root, path)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum DescriptorKeyV3 {
    Role(ObjectRole),
    Kind(EvidenceKind),
}

fn descriptor_v3(
    manifest: &K2UncertaintyR8BPacketManifestV3,
    key: DescriptorKeyV3,
) -> K2CompositionResultV1<&K2UncertaintyR8BPacketDescriptorV3> {
    let mut rows = manifest.members.iter().filter(|row| match key {
        DescriptorKeyV3::Role(role) => row.object_role == role,
        DescriptorKeyV3::Kind(kind) => row.evidence_kind == Some(kind),
    });
    let row = rows.next().ok_or_else(|| invalid("self_formed_r8b_v3_descriptor_missing"))?;
    reject(rows.next().is_some(), "self_formed_r8b_v3_descriptor_duplicate")?;
    Ok(row)
}

fn read_descriptor_json_v3<T>(root: &Path, descriptor: &K2UncertaintyR8BPacketDescriptorV3) -> K2CompositionResultV1<T>
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    read_closed_json_v2(
        &root.join(&descriptor.relative_path),
        K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1,
        Some(descriptor.byte_len),
        Some(&descriptor.content_sha256),
    )
}

fn read_identity_manifest_v3(
    root: &Path,
    descriptor: &K2UncertaintyR8BPacketDescriptorV3,
    class: ManifestClass,
) -> K2CompositionResultV1<K2UncertaintyR8BExecutableManifestV2> {
    let value: K2UncertaintyR8BExecutableManifestV2 = read_descriptor_json_v3(root, descriptor)?;
    value.validate()?;
    reject(
        value.class != class || value.manifest_root_sha256 != descriptor.semantic_root_sha256,
        "self_formed_r8b_v3_identity_manifest_invalid",
    )?;
    Ok(value)
}

fn validate_evidence_v3(
    packet_root: &Path,
    descriptor: &K2UncertaintyR8BPacketDescriptorV3,
    manifest: &K2UncertaintyR8BPacketManifestV3,
    ledger: &K2UncertaintyR8BLedgerSummaryV3,
    identities: &BTreeMap<&str, &str>,
) -> K2CompositionResultV1<()> {
    let kind = descriptor.evidence_kind.ok_or_else(|| invalid("self_formed_r8b_v3_evidence_kind_missing"))?;
    let bytes = read_closed_file_v2(
        &packet_root.join(&descriptor.relative_path),
        K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1,
        Some(descriptor.byte_len),
        Some(&descriptor.content_sha256),
    )?;
    let mut sources = None;
    let (schema, semantic_root, observed, producer) = match kind {
        EvidenceKind::OracleCases => {
            let value: K2UncertaintyR8BOracleWrapperV3 = decode_canonical_json_v1(&bytes)?;
            validate_self_formed_r8b_oracle_wrapper_v3(&value)?;
            reject(
                value.completion_event_roots_sha256 != manifest.m16_completion_event_roots_sha256
                    || value.receipt_roots_sha256 != manifest.m16_receipt_roots_sha256,
                "self_formed_r8b_v3_m16_wrapper_roots_invalid",
            )?;
            (value.schema, value.receipt_root_sha256, 16, None)
        }
        EvidenceKind::FrozenControlScopes => {
            let value: K2UncertaintyR8BControlWrapperV3 = decode_canonical_json_v1(&bytes)?;
            validate_self_formed_r8b_control_wrapper_v3(&value)?;
            reject(
                value.completion_event_roots_sha256 != manifest.m17_completion_event_roots_sha256
                    || value.receipt_roots_sha256 != manifest.m17_receipt_roots_sha256,
                "self_formed_r8b_v3_m17_wrapper_roots_invalid",
            )?;
            sources = Some(value.census.source_roots_sha256);
            (value.schema, value.receipt_root_sha256, 4, Some(value.census.producer_executable_sha256))
        }
        _ => {
            let (view_schema, view_root, view_observed, view_producer, view_sources) =
                decode_self_formed_r8b_evidence_view_v3(kind, &bytes, &manifest.route_id_sha256)?;
            sources = view_sources;
            (view_schema, view_root, view_observed, view_producer)
        }
    };
    reject(
        semantic_root != descriptor.semantic_root_sha256
            || kind.required().is_some_and(|required| observed != required),
        "self_formed_r8b_v3_evidence_semantics_invalid",
    )?;
    if kind == EvidenceKind::ProductionSurvival {
        reject(
            producer.as_deref() != identities.get("M24_LINKED_RUNNER").copied(),
            "self_formed_r8b_v3_parent_evidence_identity_invalid",
        )?;
        return Ok(());
    }
    let output = unique_output_contract_v3(ledger, &descriptor.relative_path)?;
    reject(
        output.object_role != descriptor.object_role
            || output.evidence_kind != descriptor.evidence_kind
            || output.receipt_schema != schema
            || output.required_denominator.is_some_and(|required| required != observed)
            || producer.as_deref().is_some_and(|actual| actual != output.producer_executable_sha256)
            || sources.as_ref().is_some_and(|actual| *actual != output.required_source_roots_sha256),
        "self_formed_r8b_v3_output_semantics_invalid",
    )
}

fn unique_output_contract_v3<'a>(
    ledger: &'a K2UncertaintyR8BLedgerSummaryV3,
    relative_path: &str,
) -> K2CompositionResultV1<&'a K2UncertaintyR8BOutputContractV3> {
    let mut rows = ledger.authority_outputs.iter().filter(|(_, output)| output.relative_path == relative_path);
    let (_, output) = rows.next().ok_or_else(|| invalid("self_formed_r8b_v3_output_contract_missing"))?;
    reject(rows.next().is_some(), "self_formed_r8b_v3_output_contract_duplicate")?;
    Ok(output)
}

fn validate_packet_descriptor_v3(descriptor: &K2UncertaintyR8BPacketDescriptorV3) -> K2CompositionResultV1<()> {
    require_composition_root_v1(&descriptor.content_sha256)?;
    require_composition_root_v1(&descriptor.semantic_root_sha256)?;
    let evidence = descriptor.object_role == ObjectRole::Evidence;
    reject(
        !valid_composition_path_v1(&descriptor.relative_path)
            || descriptor.relative_path == PACKET_MANIFEST_PATH_V3
            || descriptor.relative_path.len() > 240
            || descriptor.byte_len == 0
            || descriptor.unix_mode != 0o400
            || evidence != descriptor.evidence_kind.is_some(),
        "self_formed_r8b_v3_packet_descriptor_invalid",
    )
}

fn require_special_descriptor_v3(
    manifest: &K2UncertaintyR8BPacketManifestV3,
    role: ObjectRole,
    semantic_root: &str,
    path: Option<&str>,
) -> K2CompositionResultV1<()> {
    let matches = manifest.members.iter().filter(|row| {
        row.object_role == role
            && row.semantic_root_sha256 == semantic_root
            && path.is_none_or(|expected| row.relative_path == expected)
    });
    reject(matches.count() != 1, "self_formed_r8b_v3_packet_special_descriptor_invalid")
}

fn validate_root_vector_v3(roots: &[String], required: usize) -> K2CompositionResultV1<()> {
    for root in roots {
        require_composition_root_v1(root)?;
    }
    reject(
        roots.len() != required || roots.windows(2).any(|pair| pair[0] >= pair[1]),
        "self_formed_r8b_root_vector_invalid",
    )
}

fn validate_dual_roots_v3(
    manifest: &K2UncertaintyR8BPacketManifestV3,
    ledger: &K2UncertaintyR8BLedgerSummaryV3,
) -> K2CompositionResultV1<()> {
    #[rustfmt::skip]
    let groups = [
        (&manifest.m16_completion_event_roots_sha256, &ledger.m16_event_roots_sha256,
         &manifest.m16_receipt_roots_sha256, &ledger.m16_receipt_roots_sha256, 16),
        (&manifest.m17_completion_event_roots_sha256, &ledger.m17_event_roots_sha256,
         &manifest.m17_receipt_roots_sha256, &ledger.m17_receipt_roots_sha256, 4),
    ];
    for (events, observed_events, receipts, observed_receipts, required) in groups {
        for (claimed, observed) in [(events, observed_events), (receipts, observed_receipts)] {
            validate_root_vector_v3(claimed, required)?;
            reject(
                claimed.iter().cloned().collect::<BTreeSet<_>>() != *observed,
                "self_formed_r8b_v3_dual_root_set_invalid",
            )?;
        }
        reject(
            !events.iter().collect::<BTreeSet<_>>().is_disjoint(&receipts.iter().collect()),
            "self_formed_r8b_dual_root_domain_invalid",
        )?;
    }
    Ok(())
}

fn validate_descriptor_attestations_v3(
    manifest: &K2UncertaintyR8BPacketManifestV3,
    ledger: &K2UncertaintyR8BLedgerSummaryV3,
) -> K2CompositionResultV1<()> {
    for descriptor in manifest.members.iter().filter(|row| {
        row.object_role == ObjectRole::DownstreamInvocationContract
            || row.evidence_kind.is_some_and(|kind| {
                !matches!(
                    kind,
                    EvidenceKind::LinkedManifest | EvidenceKind::SuiteManifest | EvidenceKind::ProductionSurvival
                )
            })
    }) {
        let output = unique_output_contract_v3(ledger, &descriptor.relative_path)
            .map_err(|_| invalid("self_formed_r8b_v3_packet_descriptor_unattested"))?;
        let attestation = output
            .file_attestation
            .as_ref()
            .ok_or_else(|| invalid("self_formed_r8b_v3_packet_descriptor_attestation_missing"))?;
        reject(
            output.object_role != descriptor.object_role
                || output.evidence_kind != descriptor.evidence_kind
                || (attestation.byte_len, attestation.unix_mode) != (descriptor.byte_len, descriptor.unix_mode)
                || attestation.content_sha256 != descriptor.content_sha256
                || attestation.semantic_root_sha256 != descriptor.semantic_root_sha256,
            "self_formed_r8b_v3_packet_descriptor_attestation_invalid",
        )?;
    }
    Ok(())
}

fn invalid(reason: &'static str) -> K2CompositionErrorV1 {
    K2CompositionErrorV1::Invalid(reason)
}

fn reject(condition: bool, reason: &'static str) -> K2CompositionResultV1<()> {
    (!condition).then_some(()).ok_or_else(|| invalid(reason))
}
