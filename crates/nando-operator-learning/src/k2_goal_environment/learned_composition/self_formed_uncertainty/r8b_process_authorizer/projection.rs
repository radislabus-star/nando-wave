use super::*;

pub(super) fn validate_producer_shape_v3(
    request: &K2UncertaintyR8BProducerRequestV3,
) -> K2CompositionResultV1<()> {
    let expected = match request.producer_role.as_str() {
        "S01_CRATE_UNIT" => (0, 3),
        "S02_RESTART" => (16, 1),
        "S03_MODE_MATRIX" | "S04_CLEANUP_NEGATIVE" => (6, 1),
        "S05_AUTHORITY_PUBLICATION" => (2, 1),
        "M24_LINKED_RUNNER" => (151, 4),
        _ => return Err(invalid("self_formed_r8b_v3_producer_role_invalid")),
    };
    let roles = request
        .outputs
        .iter()
        .fold(BTreeMap::new(), |mut counts, row| {
            *counts.entry(row.object_role).or_insert(0_usize) += 1;
            counts
        });
    reject(
        (request.invocation_plan.len(), request.outputs.len()) != expected
            || (request.producer_role == "M24_LINKED_RUNNER"
                && (roles.get(&ObjectRole::DownstreamInvocationContract) != Some(&1)
                    || roles.get(&ObjectRole::Evidence) != Some(&3)
                    || roles.len() != 2))
            || (request.producer_role != "M24_LINKED_RUNNER"
                && roles != BTreeMap::from([(ObjectRole::Evidence, expected.1)])),
        "self_formed_r8b_v3_producer_shape_invalid",
    )
}

pub fn validate_self_formed_r8b_schedule_authority_v3(
    authority: &K2UncertaintyR8BScheduleAuthorityV3,
) -> K2CompositionResultV1<()> {
    for root in std::iter::once(&authority.schedule_grammar_root_sha256)
        .chain(authority.case_ids_sha256.iter())
        .chain(std::iter::once(&authority.authority_root_sha256))
    {
        require_composition_root_v1(root)?;
    }
    let mut canonical = authority.clone();
    canonical.authority_root_sha256.clear();
    reject(
        authority.schema != K2_UNCERTAINTY_R8B_SCHEDULE_AUTHORITY_SCHEMA_V3
            || authority.formula != K2_UNCERTAINTY_R8B_SCHEDULE_FORMULA_V3
            || authority.case_ids_sha256.len() != 16
            || !authority
                .case_ids_sha256
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            || authority.minimum_representatives != 8
            || authority.maximum_representatives != 1_792
            || authority.authority_root_sha256 != uncertainty_root_v1(&canonical)?,
        "self_formed_r8b_schedule_authority_invalid",
    )
}

pub fn validate_self_formed_r8b_downstream_contract_v3(
    contract: &K2UncertaintyR8BDownstreamContractV3,
) -> K2CompositionResultV1<()> {
    require_composition_root_v1(&contract.route_id_sha256)?;
    require_composition_root_v1(&contract.schedule_grammar_root_sha256)?;
    let mut ids = BTreeSet::new();
    for invocation in &contract.invocations {
        validate_invocation_plan_v3(invocation)?;
        reject(
            !downstream_invocation_v3(invocation)
                || !ids.insert(invocation.invocation_id_sha256.as_str()),
            "self_formed_r8b_v3_downstream_row_invalid",
        )?;
    }
    let mut canonical = contract.clone();
    canonical.projection_root_sha256.clear();
    reject(
        contract.schema != K2_UNCERTAINTY_R8B_DOWNSTREAM_CONTRACT_SCHEMA_V3
            || contract.invocations.len() != 149
            || !contract
                .invocations
                .windows(2)
                .all(|pair| pair[0].invocation_id_sha256 < pair[1].invocation_id_sha256)
            || contract.projection_root_sha256 != uncertainty_root_v1(&canonical)?,
        "self_formed_r8b_v3_downstream_contract_invalid",
    )?;
    validate_c08_cardinality_v3(&contract.invocations)?;
    Ok(())
}

pub fn validate_self_formed_r8b_process_projections_v3(
    ledger: &K2UncertaintyR8BLedgerSummaryV3,
    c08: &K2UncertaintyR8BDownstreamContractV3,
) -> K2CompositionResultV1<()> {
    validate_self_formed_r8b_schedule_authority_v3(&ledger.schedule_authority)?;
    let static_rows = ledger
        .invocations
        .iter()
        .filter(|row| static_invocation_v3(row))
        .cloned()
        .collect::<Vec<_>>();
    let roots = static_rows
        .iter()
        .filter(|row| PRODUCER_ROLES_V3.contains(&row.target_role.as_str()))
        .map(|row| {
            ledger
                .request_roots_sha256
                .get(&row.invocation_id_sha256)
                .cloned()
                .map(|root| (row.invocation_id_sha256.clone(), root))
        })
        .collect::<Option<BTreeMap<_, _>>>()
        .ok_or_else(|| invalid("self_formed_r8b_v3_static_request_root_missing"))?;
    let observed = seal_static_projection_v3(
        ledger.route_id_sha256.clone(),
        ledger
            .schedule_authority
            .schedule_grammar_root_sha256
            .clone(),
        static_rows,
        roots,
    )?;
    reject(
        observed.projection_root_sha256 != ledger.expected_projection_root_sha256
            || c08.schedule_grammar_root_sha256
                != ledger.schedule_authority.schedule_grammar_root_sha256
            || ledger.invocations.iter().any(|row| {
                [
                    static_invocation_v3(row),
                    dynamic_invocation_v3(row),
                    downstream_invocation_v3(row),
                ]
                .into_iter()
                .filter(|value| *value)
                .count()
                    != 1
            }),
        "self_formed_r8b_v3_projection_partition_invalid",
    )?;
    validate_m10_projection_v3(ledger)?;
    validate_downstream_projection_v3(ledger, c08)
}

fn validate_m10_projection_v3(
    ledger: &K2UncertaintyR8BLedgerSummaryV3,
) -> K2CompositionResultV1<()> {
    let cases = ledger
        .schedule_authority
        .case_ids_sha256
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    reject(
        ledger
            .representative_counts
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
            != cases,
        "self_formed_r8b_v3_m10_fact_set_invalid",
    )?;
    let mut expected = BTreeMap::new();
    for (case, representatives) in &ledger.representative_counts {
        reject(
            !(ledger.schedule_authority.minimum_representatives
                ..=ledger.schedule_authority.maximum_representatives)
                .contains(representatives),
            "self_formed_r8b_v3_m10_fact_range_invalid",
        )?;
        let t = representatives.saturating_sub(8).div_ceil(7) + 1;
        for (role, count) in [
            ("M03_LEARNER", 1),
            ("M04_PROBE", 1),
            ("M05_SELECTOR", t),
            ("M06_BASELINE", 1 + 3 * t),
            ("M07_SELECTION_PREVERIFIER", 1),
            ("M08_CLOSURE_PLANNER", 1),
            ("M09_CLOSURE_VERIFIER", 1),
        ] {
            expected.insert((case.clone(), role.to_owned()), count);
        }
    }
    let mut observed = BTreeMap::<(String, String), BTreeSet<u64>>::new();
    for row in ledger
        .invocations
        .iter()
        .filter(|row| dynamic_invocation_v3(row))
    {
        let case = row
            .case_id_sha256
            .clone()
            .ok_or_else(|| invalid("self_formed_r8b_v3_m10_case_missing"))?;
        let (stage, validator) = match row.target_role.as_str() {
            "M03_LEARNER" => ("C03", Validator::ConcreteReceipt),
            "M04_PROBE" => ("C04", Validator::RepresentativeCount),
            "M05_SELECTOR" => ("C05", Validator::ConcreteReceipt),
            "M06_BASELINE" => ("C06", Validator::ConcreteReceipt),
            "M07_SELECTION_PREVERIFIER" => ("C07", Validator::ConcreteReceipt),
            "M08_CLOSURE_PLANNER" => ("C08", Validator::ConcreteReceipt),
            "M09_CLOSURE_VERIFIER" => ("C09", Validator::ConcreteReceipt),
            _ => return Err(invalid("self_formed_r8b_v3_m10_role_invalid")),
        };
        reject(
            row.stage != stage
                || row.launch_kind != LaunchKind::BwrapPrlimitMediated
                || row.expected_outcome != ExpectedOutcome::AuthoritySuccess
                || row.expected_exit_predicate.is_some()
                || row.validator != validator
                || row.parent_invocation_id_sha256.is_some()
                || row.probe_ordinal.is_none(),
            "self_formed_r8b_v3_m10_route_invalid",
        )?;
        reject(
            !observed
                .entry((case, row.target_role.clone()))
                .or_default()
                .insert(row.probe_ordinal.unwrap_or_default()),
            "self_formed_r8b_v3_m10_ordinal_duplicate",
        )?;
    }
    reject(
        observed.len() != expected.len()
            || expected.iter().any(|(key, count)| {
                observed.get(key).is_none_or(|ordinals| {
                    ordinals.len() as u64 != *count || ordinals.iter().copied().ne(0..*count)
                })
            }),
        "self_formed_r8b_v3_m10_projection_mismatch",
    )
}

fn validate_c08_cardinality_v3(
    plan: &[K2UncertaintyR8BInvocationPlanV3],
) -> K2CompositionResultV1<()> {
    #[rustfmt::skip]
    let expected = [
        ("M11_PRIVATE_RESOLVER", 24), ("M12_SAFETY", 24),
        ("M13_WORKER", 24), ("M14_OBSERVER", 24),
        ("M15_FINAL_VERIFIER", 16), ("M16_ORACLE", 16),
        ("M19_FRESH_CONTROL_CASE", 12), ("M17_CONTROL_EVALUATOR", 4),
        ("M18_TERMINAL_EVALUATOR", 1), ("M20_CLEANUP_AUTHORIZER", 1),
        ("M21_CLEANUP_OWNER", 1), ("M22_CLEANUP_VERIFIER", 1),
        ("M23_DEVELOPMENT_RESULT_PUBLISHER", 1),
    ].into_iter().collect::<BTreeMap<_, _>>();
    let mut observed = BTreeMap::new();
    let mut scheduled = BTreeMap::<&str, BTreeSet<(&str, Option<u64>)>>::new();
    for row in plan {
        let (stage, launch, case, probe) = match row.target_role.as_str() {
            "M11_PRIVATE_RESOLVER" | "M12_SAFETY" | "M13_WORKER" | "M14_OBSERVER" => {
                ("C09", LaunchKind::BwrapPrlimitMediated, true, true)
            }
            "M15_FINAL_VERIFIER" => ("C09", LaunchKind::BwrapPrlimitMediated, true, false),
            "M16_ORACLE" => ("C10", LaunchKind::BwrapPrlimitMediated, true, false),
            "M19_FRESH_CONTROL_CASE" => ("C11", LaunchKind::BwrapPrlimitMediated, true, false),
            "M17_CONTROL_EVALUATOR" => ("C12", LaunchKind::Direct, false, false),
            "M18_TERMINAL_EVALUATOR" => ("C14", LaunchKind::Direct, false, false),
            "M20_CLEANUP_AUTHORIZER" => ("C16", LaunchKind::BwrapPrlimitMediated, false, false),
            "M21_CLEANUP_OWNER" => ("C17", LaunchKind::BwrapPrlimitMediated, false, false),
            "M22_CLEANUP_VERIFIER" => ("C18", LaunchKind::BwrapPrlimitMediated, false, false),
            "M23_DEVELOPMENT_RESULT_PUBLISHER" => {
                ("C20", LaunchKind::BwrapPrlimitMediated, false, false)
            }
            _ => return Err(invalid("self_formed_r8b_v3_c08_role_invalid")),
        };
        reject(
            row.request_owner_role != "M24_LINKED_RUNNER"
                || row.stage != stage
                || row.launch_kind != launch
                || row.expected_outcome != ExpectedOutcome::AuthoritySuccess
                || row.expected_exit_predicate.is_some()
                || row.validator != Validator::ConcreteReceipt
                || row.case_id_sha256.is_some() != case
                || row.probe_ordinal.is_some() != probe
                || row.parent_invocation_id_sha256.is_some(),
            "self_formed_r8b_v3_c08_route_invalid",
        )?;
        if let Some(case_id) = row.case_id_sha256.as_deref() {
            reject(
                !scheduled
                    .entry(row.target_role.as_str())
                    .or_default()
                    .insert((case_id, row.probe_ordinal)),
                "self_formed_r8b_v3_c08_schedule_duplicate",
            )?;
        }
        *observed.entry(row.target_role.as_str()).or_insert(0_usize) += 1;
    }
    reject(
        observed != expected
            || scheduled["M11_PRIVATE_RESOLVER"] != scheduled["M12_SAFETY"]
            || scheduled["M11_PRIVATE_RESOLVER"] != scheduled["M13_WORKER"]
            || scheduled["M11_PRIVATE_RESOLVER"] != scheduled["M14_OBSERVER"]
            || scheduled["M15_FINAL_VERIFIER"] != scheduled["M16_ORACLE"],
        "self_formed_r8b_v3_c08_cardinality_invalid",
    )
}

fn validate_downstream_projection_v3(
    ledger: &K2UncertaintyR8BLedgerSummaryV3,
    c08: &K2UncertaintyR8BDownstreamContractV3,
) -> K2CompositionResultV1<()> {
    let mut observed = ledger
        .invocations
        .iter()
        .filter(|row| downstream_invocation_v3(row))
        .cloned()
        .collect::<Vec<_>>();
    observed.sort_by(|left, right| left.invocation_id_sha256.cmp(&right.invocation_id_sha256));
    reject(
        observed != c08.invocations,
        "self_formed_r8b_v3_c08_projection_mismatch",
    )
}

fn seal_static_projection_v3(
    route_id_sha256: String,
    schedule_grammar_root_sha256: String,
    mut invocations: Vec<K2UncertaintyR8BInvocationPlanV3>,
    producer_request_roots_sha256: BTreeMap<String, String>,
) -> K2CompositionResultV1<K2UncertaintyR8BStaticProjectionV3> {
    require_composition_root_v1(&route_id_sha256)?;
    require_composition_root_v1(&schedule_grammar_root_sha256)?;
    invocations.sort_by(|left, right| left.invocation_id_sha256.cmp(&right.invocation_id_sha256));
    for row in &invocations {
        validate_invocation_plan_v3(row)?;
    }
    for root in producer_request_roots_sha256.values() {
        require_composition_root_v1(root)?;
    }
    let parent_launches = invocations
        .iter()
        .filter(|row| producer_request_roots_sha256.contains_key(&row.invocation_id_sha256))
        .collect::<Vec<_>>();
    let parent_targets = parent_launches
        .iter()
        .map(|row| row.target_role.as_str())
        .collect::<BTreeSet<_>>();
    reject(
        invocations.len() != 39
            || producer_request_roots_sha256.len() != 6
            || parent_targets != PRODUCER_ROLES_V3.into_iter().collect()
            || parent_launches.iter().any(|row| {
                row.request_owner_role != "M24_LINKED_RUNNER"
                    || row.stage != "P01"
                    || row.parent_invocation_id_sha256.is_some()
                    || row.case_id_sha256.is_some()
                    || row.probe_ordinal.is_some()
                    || row.expected_outcome != ExpectedOutcome::AuthoritySuccess
                    || row.expected_exit_predicate.is_some()
                    || row.validator != Validator::ConcreteReceipt
                    || if row.target_role == "M24_LINKED_RUNNER" {
                        row.launch_kind != LaunchKind::UserSystemd
                    } else {
                        row.launch_kind != LaunchKind::Direct
                    }
            })
            || invocations
                .windows(2)
                .any(|pair| pair[0].invocation_id_sha256 >= pair[1].invocation_id_sha256)
            || producer_request_roots_sha256.keys().any(|id| {
                !invocations
                    .iter()
                    .any(|row| &row.invocation_id_sha256 == id)
            }),
        "self_formed_r8b_v3_static_projection_invalid",
    )?;
    let mut projection = K2UncertaintyR8BStaticProjectionV3 {
        schema: K2_UNCERTAINTY_R8B_STATIC_PROJECTION_SCHEMA_V3.to_owned(),
        route_id_sha256,
        schedule_grammar_root_sha256,
        invocations,
        producer_request_roots_sha256,
        projection_root_sha256: String::new(),
    };
    projection.projection_root_sha256 = uncertainty_root_v1(&projection)?;
    Ok(projection)
}

fn dynamic_invocation_v3(row: &K2UncertaintyR8BInvocationPlanV3) -> bool {
    row.request_owner_role == "M10_PUBLIC_COORDINATOR"
        && DYNAMIC_ROLES_V3.contains(&row.target_role.as_str())
}

fn downstream_invocation_v3(row: &K2UncertaintyR8BInvocationPlanV3) -> bool {
    row.request_owner_role == "M24_LINKED_RUNNER"
        && DOWNSTREAM_ROLES_V3.contains(&row.target_role.as_str())
        && row
            .stage
            .strip_prefix('C')
            .and_then(|value| value.parse::<u8>().ok())
            .is_some_and(|value| (9..=20).contains(&value))
}

fn static_invocation_v3(row: &K2UncertaintyR8BInvocationPlanV3) -> bool {
    PRODUCER_ROLES_V3.contains(&row.target_role.as_str())
        || PRODUCER_ROLES_V3[..5].contains(&row.request_owner_role.as_str())
        || (row.request_owner_role == "M01_DEVELOPMENT_OWNER" && row.target_role == "M02_GENERATOR")
        || (row.request_owner_role == "M24_LINKED_RUNNER"
            && matches!(
                row.target_role.as_str(),
                "M01_DEVELOPMENT_OWNER" | "M10_PUBLIC_COORDINATOR"
            ))
}
