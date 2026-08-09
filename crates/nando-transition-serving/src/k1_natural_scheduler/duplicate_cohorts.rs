use std::collections::BTreeSet;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};

use super::*;

const DUPLICATE_PROTOCOL_BLOCKER: &str = "all_supported_t1_protocol_modes_already_active";
const COHORT_IDENTITY_SCHEMA: &str = "nando.k1-natural-cohort-identity.v2";
const ACTIVE_PROTOCOL_MODE_SET_SCHEMA: &str = "nando.k1-active-protocol-mode-set.v1";
const LEGACY_DISCOVERY_BASIS_SCHEMA: &str = "nando.k1-legacy-unversioned-discovery-basis.v1";

pub(crate) fn active_protocol_mode_set_root(
    active_protocol_mode_roots_sha256: &BTreeSet<String>,
) -> Result<String, String> {
    canonical_json_sha256(&(
        ACTIVE_PROTOCOL_MODE_SET_SCHEMA,
        active_protocol_mode_roots_sha256,
    ))
    .map_err(str::to_owned)
}

pub(super) fn duplicate_candidate_exclusions(
    ledger: &K1SchedulerLedgerV1,
    catalog: &K1NaturalCohortCatalogV1,
    active_protocol_mode_set_root_sha256: &str,
    current_discovery_basis_root_sha256: &str,
) -> Result<BTreeSet<String>, String> {
    ledger.validate().map_err(str::to_owned)?;
    catalog.validate().map_err(str::to_owned)?;
    if !valid_nonzero_sha256(current_discovery_basis_root_sha256) {
        return Err("k1_duplicate_cohort_discovery_basis_invalid".to_owned());
    }

    let mut active_freeze = None;
    let mut duplicate_identities = BTreeSet::new();
    for event in &ledger.events {
        match &event.payload {
            K1SchedulerEventPayloadV1::CandidateFreeze(freeze) => {
                active_freeze = Some(freeze);
            }
            K1SchedulerEventPayloadV1::TerminalVerdict(verdict) => {
                let freeze = active_freeze
                    .take()
                    .ok_or_else(|| "k1_duplicate_cohort_candidate_missing".to_owned())?;
                if verdict.blocker == DUPLICATE_PROTOCOL_BLOCKER
                    && verdict
                        .evidence_roots_sha256
                        .iter()
                        .any(|root| root == active_protocol_mode_set_root_sha256)
                {
                    duplicate_identities.insert(freeze_identity_root(freeze)?);
                }
            }
            _ => {}
        }
    }

    let mut exclusions = BTreeSet::new();
    for candidate in &catalog.candidates {
        if duplicate_identities.contains(&candidate_identity_root(
            candidate,
            current_discovery_basis_root_sha256,
        )?) {
            exclusions.insert(candidate.candidate_root_sha256.clone());
        }
    }
    Ok(exclusions)
}

fn freeze_identity_root(freeze: &K1NaturalCandidateFreezeV1) -> Result<String, String> {
    freeze.validate().map_err(str::to_owned)?;
    cohort_identity_root(
        &freeze.candidate_structural_root_sha256,
        &freeze.source_neutral_topology_root_sha256,
        &freeze.semantic_novelty_signature_root_sha256,
        freeze.consequence_type,
        &freeze.generator_schema,
        &freeze_discovery_basis_root(freeze)?,
    )
}

fn candidate_identity_root(
    candidate: &K1NaturalCohortCandidateV1,
    discovery_basis_root_sha256: &str,
) -> Result<String, String> {
    candidate.validate().map_err(str::to_owned)?;
    cohort_identity_root(
        &candidate.candidate_structural_root_sha256,
        &candidate.source_neutral_topology_root_sha256,
        &candidate.semantic_novelty_signature_root_sha256,
        candidate.consequence_type,
        &candidate.generator_schema,
        discovery_basis_root_sha256,
    )
}

fn freeze_discovery_basis_root(freeze: &K1NaturalCandidateFreezeV1) -> Result<String, String> {
    if freeze.discovery_basis_root_sha256.is_empty() {
        return canonical_json_sha256(&(LEGACY_DISCOVERY_BASIS_SCHEMA, freeze.schema.as_str()))
            .map_err(str::to_owned);
    }
    Ok(freeze.discovery_basis_root_sha256.clone())
}

fn cohort_identity_root(
    candidate_structural_root_sha256: &str,
    source_neutral_topology_root_sha256: &str,
    semantic_novelty_signature_root_sha256: &str,
    consequence_type: K1ConsequenceTypeV1,
    generator_schema: &str,
    discovery_basis_root_sha256: &str,
) -> Result<String, String> {
    canonical_json_sha256(&(
        COHORT_IDENTITY_SCHEMA,
        candidate_structural_root_sha256,
        source_neutral_topology_root_sha256,
        semantic_novelty_signature_root_sha256,
        consequence_type,
        generator_schema,
        discovery_basis_root_sha256,
    ))
    .map_err(str::to_owned)
}
