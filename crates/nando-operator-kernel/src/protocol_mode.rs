use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    BindingCompletionStateV1, BindingPredicateV1, BindingValueTypeV1, canonical_json_sha256,
};

pub const PROTOCOL_MODE_SET_SCHEMA_V2: &str = "nando.protocol-mode-set.v2.f4r2";
pub const MAX_SELECTOR_PREDICATES_V2: usize = 3;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingProtocolCompileVerdictV2 {
    ProtocolModeSet,
    Abstain,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolModeCompilerBudgetV2 {
    pub max_candidates: usize,
    pub max_surviving_modes: usize,
}

impl Default for ProtocolModeCompilerBudgetV2 {
    fn default() -> Self {
        Self {
            max_candidates: 512,
            max_surviving_modes: 32,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolRoleCardinalityV2 {
    OneActionClass,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolSourceRoleV2 {
    pub role_id: u16,
    pub value_type: BindingValueTypeV1,
    pub cardinality: ProtocolRoleCardinalityV2,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolSourceRoleSchemaV2 {
    pub roles: Vec<ProtocolSourceRoleV2>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolSelectorProgramV2 {
    pub predicates: Vec<BindingPredicateV1>,
    pub max_action_classes: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolValueContractV2 {
    pub observed: BindingValueTypeV1,
    pub emitted: BindingValueTypeV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolCapabilityContractV2 {
    pub protocol_facet_root_sha256: String,
    pub physical_program_ids_sha256: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolArgumentRoleV2 {
    pub argument_ordinal: u16,
    pub source_role_id: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolArgumentRoleSchemaV2 {
    pub roles: Vec<ProtocolArgumentRoleV2>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolConstantContractV2 {
    pub semantic_constants_sha256: Vec<String>,
    pub protocol_noop_constants_sha256: Vec<String>,
    pub execution_budget_roots_sha256: Vec<String>,
    pub transport_default_roots_sha256: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolStructuralGuardV2 {
    pub relation_identity_sha256: String,
    pub effect_invariant_root_sha256: String,
    pub selector_program_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolTemporalCardinalityContractV2 {
    pub completion_states: Vec<BindingCompletionStateV1>,
    pub temporal_distances: Vec<u16>,
    pub event_candidate_cardinalities: Vec<u16>,
    pub require_unique_action_class: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolModeProgramV2 {
    pub source_role_schema: ProtocolSourceRoleSchemaV2,
    pub selector_program: ProtocolSelectorProgramV2,
    pub value_contract: ProtocolValueContractV2,
    pub capability_contract: ProtocolCapabilityContractV2,
    pub argument_role_schema: ProtocolArgumentRoleSchemaV2,
    pub constant_contract: ProtocolConstantContractV2,
    pub structural_guard: ProtocolStructuralGuardV2,
    pub temporal_cardinality_contract: ProtocolTemporalCardinalityContractV2,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundedProtocolModeCandidateV2 {
    pub candidate_id_sha256: String,
    pub effect_law_id_sha256: String,
    pub relation_identity_sha256: String,
    pub protocol_facet_root_sha256: String,
    pub effect_invariant_root_sha256: String,
    pub source_role_schema_root_sha256: String,
    pub selector_program_root_sha256: String,
    pub observed_emitted_types_root_sha256: String,
    pub capability_protocol_root_sha256: String,
    pub argument_role_schema_root_sha256: String,
    pub constant_contract_root_sha256: String,
    pub structural_guard_root_sha256: String,
    pub temporal_cardinality_contract_root_sha256: String,
    pub action_class_root_sha256: String,
    pub program: ProtocolModeProgramV2,
    pub covers_positive_rows_sha256: Vec<String>,
    pub accepts_negative_rows_sha256: Vec<String>,
    pub wrong_action_rows_sha256: Vec<String>,
    pub verify_failed_rows_sha256: Vec<String>,
    pub search_exhausted: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolModeV2 {
    pub mode_id_sha256: String,
    pub effect_law_id_sha256: String,
    pub relation_identity_sha256: String,
    pub protocol_facet_root_sha256: String,
    pub effect_invariant_root_sha256: String,
    pub source_role_schema_root_sha256: String,
    pub selector_program_root_sha256: String,
    pub observed_emitted_types_root_sha256: String,
    pub capability_protocol_root_sha256: String,
    pub argument_role_schema_root_sha256: String,
    pub constant_contract_root_sha256: String,
    pub structural_guard_root_sha256: String,
    pub temporal_cardinality_contract_root_sha256: String,
    pub action_class_root_sha256: String,
    pub program: ProtocolModeProgramV2,
    pub covered_positive_rows_sha256: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolModeSetV2 {
    pub schema: String,
    pub mode_set_sha256: String,
    pub verdict: BindingProtocolCompileVerdictV2,
    pub binding_capability_root_sha256: String,
    pub effect_law_id_sha256: String,
    pub relation_identity_sha256: String,
    pub modes: Vec<ProtocolModeV2>,
    pub positive_rows: usize,
    pub positive_rows_covered: usize,
    pub wrong_actions: usize,
    pub verify_failed: usize,
    pub negative_accepts: usize,
    pub search_exhausted: bool,
    pub action_equivalence_classes: usize,
    pub all_surviving_covers_action_equivalent: bool,
    pub production_admissible: bool,
    pub execution_authority: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingProtocolCompilerErrorV2 {
    InvalidDigest,
    InvalidBudget,
    InvalidCandidate,
    InvalidGraphView,
    InvalidModeSet,
    Serialization,
}

impl ProtocolModeSetV2 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, BindingProtocolCompilerErrorV2> {
        pretty_json_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, BindingProtocolCompilerErrorV2> {
        let set: Self = serde_json::from_slice(bytes)
            .map_err(|_| BindingProtocolCompilerErrorV2::InvalidModeSet)?;
        if set.canonical_bytes()? != bytes {
            return Err(BindingProtocolCompilerErrorV2::InvalidModeSet);
        }
        validate_protocol_mode_set_v2(&set)?;
        Ok(set)
    }
}

pub fn validate_protocol_mode_budget_v2(
    budget: ProtocolModeCompilerBudgetV2,
) -> Result<(), BindingProtocolCompilerErrorV2> {
    if budget.max_candidates == 0
        || budget.max_candidates > 4096
        || budget.max_surviving_modes == 0
        || budget.max_surviving_modes > 512
    {
        return Err(BindingProtocolCompilerErrorV2::InvalidBudget);
    }
    Ok(())
}

pub fn validate_protocol_mode_program_v2(
    program: &ProtocolModeProgramV2,
) -> Result<(), BindingProtocolCompilerErrorV2> {
    if program.source_role_schema.roles.len() != 1
        || program.source_role_schema.roles[0].role_id != 0
        || program.source_role_schema.roles[0].cardinality
            != ProtocolRoleCardinalityV2::OneActionClass
        || program.selector_program.max_action_classes != 1
        || program.selector_program.predicates.len() > MAX_SELECTOR_PREDICATES_V2
        || program.argument_role_schema.roles
            != vec![ProtocolArgumentRoleV2 {
                argument_ordinal: 0,
                source_role_id: 0,
            }]
        || program.value_contract.observed != program.source_role_schema.roles[0].value_type
        || program.value_contract.emitted != program.source_role_schema.roles[0].value_type
        || program.structural_guard.selector_program_root_sha256
            != derived_mode_root_v2("selector-program", &program.selector_program)?
        || !is_protocol_mode_sha256(&program.structural_guard.relation_identity_sha256)
        || !is_protocol_mode_sha256(&program.structural_guard.effect_invariant_root_sha256)
        || !is_protocol_mode_sha256(&program.capability_contract.protocol_facet_root_sha256)
        || program
            .capability_contract
            .physical_program_ids_sha256
            .is_empty()
        || program
            .capability_contract
            .physical_program_ids_sha256
            .iter()
            .any(|root| !is_protocol_mode_sha256(root))
        || constant_roots_v2(&program.constant_contract).any(|root| !is_protocol_mode_sha256(root))
    {
        return Err(BindingProtocolCompilerErrorV2::InvalidCandidate);
    }
    let mut predicates = program.selector_program.predicates.clone();
    predicates.sort();
    predicates.dedup();
    let mut physical_programs = program
        .capability_contract
        .physical_program_ids_sha256
        .clone();
    physical_programs.sort();
    physical_programs.dedup();
    if predicates != program.selector_program.predicates
        || physical_programs != program.capability_contract.physical_program_ids_sha256
    {
        return Err(BindingProtocolCompilerErrorV2::InvalidCandidate);
    }
    Ok(())
}

pub fn validate_protocol_mode_candidate_v2(
    candidate: &BoundedProtocolModeCandidateV2,
) -> Result<(), BindingProtocolCompilerErrorV2> {
    validate_protocol_mode_program_v2(&candidate.program)?;
    let roots = [
        candidate.candidate_id_sha256.as_str(),
        candidate.effect_law_id_sha256.as_str(),
        candidate.relation_identity_sha256.as_str(),
        candidate.protocol_facet_root_sha256.as_str(),
        candidate.effect_invariant_root_sha256.as_str(),
        candidate.source_role_schema_root_sha256.as_str(),
        candidate.selector_program_root_sha256.as_str(),
        candidate.observed_emitted_types_root_sha256.as_str(),
        candidate.capability_protocol_root_sha256.as_str(),
        candidate.argument_role_schema_root_sha256.as_str(),
        candidate.constant_contract_root_sha256.as_str(),
        candidate.structural_guard_root_sha256.as_str(),
        candidate.temporal_cardinality_contract_root_sha256.as_str(),
        candidate.action_class_root_sha256.as_str(),
    ];
    if roots.into_iter().any(|root| !is_protocol_mode_sha256(root))
        || candidate.source_role_schema_root_sha256
            != derived_mode_root_v2("source-role-schema", &candidate.program.source_role_schema)?
        || candidate.selector_program_root_sha256
            != derived_mode_root_v2("selector-program", &candidate.program.selector_program)?
        || candidate.observed_emitted_types_root_sha256
            != derived_mode_root_v2("observed-emitted-types", &candidate.program.value_contract)?
        || candidate.capability_protocol_root_sha256
            != derived_mode_root_v2(
                "capability-protocol",
                &candidate.program.capability_contract,
            )?
        || candidate.argument_role_schema_root_sha256
            != derived_mode_root_v2(
                "argument-role-schema",
                &candidate.program.argument_role_schema,
            )?
        || candidate.constant_contract_root_sha256
            != derived_mode_root_v2("constant-contract", &candidate.program.constant_contract)?
        || candidate.structural_guard_root_sha256
            != derived_mode_root_v2("structural-guard", &candidate.program.structural_guard)?
        || candidate.temporal_cardinality_contract_root_sha256
            != derived_mode_root_v2(
                "temporal-cardinality",
                &candidate.program.temporal_cardinality_contract,
            )?
        || candidate.protocol_facet_root_sha256
            != candidate
                .program
                .capability_contract
                .protocol_facet_root_sha256
        || candidate.relation_identity_sha256
            != candidate.program.structural_guard.relation_identity_sha256
        || candidate.effect_invariant_root_sha256
            != candidate
                .program
                .structural_guard
                .effect_invariant_root_sha256
        || candidate
            .covers_positive_rows_sha256
            .iter()
            .chain(candidate.accepts_negative_rows_sha256.iter())
            .chain(candidate.wrong_action_rows_sha256.iter())
            .chain(candidate.verify_failed_rows_sha256.iter())
            .any(|root| !is_protocol_mode_sha256(root))
    {
        return Err(BindingProtocolCompilerErrorV2::InvalidDigest);
    }
    Ok(())
}

pub fn protocol_mode_from_candidate_v2(
    candidate: &BoundedProtocolModeCandidateV2,
    covered: BTreeSet<String>,
) -> Result<ProtocolModeV2, BindingProtocolCompilerErrorV2> {
    let covered_positive_rows_sha256 = covered.into_iter().collect::<Vec<_>>();
    let mut mode = ProtocolModeV2 {
        mode_id_sha256: String::new(),
        effect_law_id_sha256: candidate.effect_law_id_sha256.clone(),
        relation_identity_sha256: candidate.relation_identity_sha256.clone(),
        protocol_facet_root_sha256: candidate.protocol_facet_root_sha256.clone(),
        effect_invariant_root_sha256: candidate.effect_invariant_root_sha256.clone(),
        source_role_schema_root_sha256: candidate.source_role_schema_root_sha256.clone(),
        selector_program_root_sha256: candidate.selector_program_root_sha256.clone(),
        observed_emitted_types_root_sha256: candidate.observed_emitted_types_root_sha256.clone(),
        capability_protocol_root_sha256: candidate.capability_protocol_root_sha256.clone(),
        argument_role_schema_root_sha256: candidate.argument_role_schema_root_sha256.clone(),
        constant_contract_root_sha256: candidate.constant_contract_root_sha256.clone(),
        structural_guard_root_sha256: candidate.structural_guard_root_sha256.clone(),
        temporal_cardinality_contract_root_sha256: candidate
            .temporal_cardinality_contract_root_sha256
            .clone(),
        action_class_root_sha256: candidate.action_class_root_sha256.clone(),
        program: candidate.program.clone(),
        covered_positive_rows_sha256,
    };
    mode.mode_id_sha256 = protocol_mode_digest_v2(&mode)?;
    Ok(mode)
}

pub fn protocol_mode_digest_v2(
    mode: &ProtocolModeV2,
) -> Result<String, BindingProtocolCompilerErrorV2> {
    protocol_mode_json_sha256(&(
        mode.effect_law_id_sha256.as_str(),
        mode.relation_identity_sha256.as_str(),
        mode.protocol_facet_root_sha256.as_str(),
        mode.effect_invariant_root_sha256.as_str(),
        mode.source_role_schema_root_sha256.as_str(),
        mode.selector_program_root_sha256.as_str(),
        mode.observed_emitted_types_root_sha256.as_str(),
        mode.capability_protocol_root_sha256.as_str(),
        mode.argument_role_schema_root_sha256.as_str(),
        mode.constant_contract_root_sha256.as_str(),
        mode.structural_guard_root_sha256.as_str(),
        mode.temporal_cardinality_contract_root_sha256.as_str(),
        mode.action_class_root_sha256.as_str(),
        &mode.program,
        &mode.covered_positive_rows_sha256,
    ))
}

pub fn validate_protocol_mode_set_v2(
    set: &ProtocolModeSetV2,
) -> Result<(), BindingProtocolCompilerErrorV2> {
    let mode_ids = set
        .modes
        .iter()
        .map(|mode| mode.mode_id_sha256.as_str())
        .collect::<Vec<_>>();
    let mode_ids_are_sorted = mode_ids.windows(2).all(|pair| pair[0] < pair[1]);
    if set.schema != PROTOCOL_MODE_SET_SCHEMA_V2
        || !is_protocol_mode_sha256(&set.mode_set_sha256)
        || !is_protocol_mode_sha256(&set.binding_capability_root_sha256)
        || !is_protocol_mode_sha256(&set.effect_law_id_sha256)
        || !is_protocol_mode_sha256(&set.relation_identity_sha256)
        || set.execution_authority
        || !mode_ids_are_sorted
        || set.positive_rows_covered > set.positive_rows
        || set.mode_set_sha256 != protocol_mode_set_digest_v2(set)?
        || (set.verdict == BindingProtocolCompileVerdictV2::Abstain && !set.modes.is_empty())
        || (set.verdict == BindingProtocolCompileVerdictV2::ProtocolModeSet
            && (set.modes.is_empty()
                || set.search_exhausted
                || set.action_equivalence_classes != 1
                || set.wrong_actions != 0
                || set.verify_failed != 0
                || set.negative_accepts != 0
                || set.positive_rows == 0
                || set.positive_rows_covered != set.positive_rows
                || !set.all_surviving_covers_action_equivalent))
    {
        return Err(BindingProtocolCompilerErrorV2::InvalidModeSet);
    }
    let mut mode_ids = BTreeSet::new();
    let mut covered_rows = BTreeSet::new();
    let mut action_classes = BTreeSet::new();
    for mode in &set.modes {
        validate_protocol_mode_program_v2(&mode.program)?;
        let candidate = BoundedProtocolModeCandidateV2 {
            candidate_id_sha256: mode.mode_id_sha256.clone(),
            effect_law_id_sha256: mode.effect_law_id_sha256.clone(),
            relation_identity_sha256: mode.relation_identity_sha256.clone(),
            protocol_facet_root_sha256: mode.protocol_facet_root_sha256.clone(),
            effect_invariant_root_sha256: mode.effect_invariant_root_sha256.clone(),
            source_role_schema_root_sha256: mode.source_role_schema_root_sha256.clone(),
            selector_program_root_sha256: mode.selector_program_root_sha256.clone(),
            observed_emitted_types_root_sha256: mode.observed_emitted_types_root_sha256.clone(),
            capability_protocol_root_sha256: mode.capability_protocol_root_sha256.clone(),
            argument_role_schema_root_sha256: mode.argument_role_schema_root_sha256.clone(),
            constant_contract_root_sha256: mode.constant_contract_root_sha256.clone(),
            structural_guard_root_sha256: mode.structural_guard_root_sha256.clone(),
            temporal_cardinality_contract_root_sha256: mode
                .temporal_cardinality_contract_root_sha256
                .clone(),
            action_class_root_sha256: mode.action_class_root_sha256.clone(),
            program: mode.program.clone(),
            covers_positive_rows_sha256: Vec::new(),
            accepts_negative_rows_sha256: Vec::new(),
            wrong_action_rows_sha256: Vec::new(),
            verify_failed_rows_sha256: Vec::new(),
            search_exhausted: false,
        };
        validate_protocol_mode_candidate_v2(&candidate)?;
        if mode.effect_law_id_sha256 != set.effect_law_id_sha256
            || mode.relation_identity_sha256 != set.relation_identity_sha256
            || mode.mode_id_sha256 != protocol_mode_digest_v2(mode)?
            || !mode_ids.insert(mode.mode_id_sha256.clone())
            || mode.covered_positive_rows_sha256.is_empty()
            || mode
                .covered_positive_rows_sha256
                .iter()
                .any(|root| !is_protocol_mode_sha256(root))
        {
            return Err(BindingProtocolCompilerErrorV2::InvalidModeSet);
        }
        if !mode
            .covered_positive_rows_sha256
            .windows(2)
            .all(|pair| pair[0] < pair[1])
            || mode
                .covered_positive_rows_sha256
                .iter()
                .any(|row| !covered_rows.insert(row.clone()))
        {
            return Err(BindingProtocolCompilerErrorV2::InvalidModeSet);
        }
        action_classes.insert(mode.action_class_root_sha256.clone());
    }
    if !set.modes.is_empty()
        && (covered_rows.len() != set.positive_rows_covered
            || action_classes.len() != set.action_equivalence_classes)
    {
        return Err(BindingProtocolCompilerErrorV2::InvalidModeSet);
    }
    Ok(())
}

pub fn protocol_mode_set_digest_v2(
    set: &ProtocolModeSetV2,
) -> Result<String, BindingProtocolCompilerErrorV2> {
    protocol_mode_json_sha256(&(
        set.schema.as_str(),
        set.verdict,
        set.binding_capability_root_sha256.as_str(),
        set.effect_law_id_sha256.as_str(),
        set.relation_identity_sha256.as_str(),
        &set.modes,
        set.positive_rows,
        set.positive_rows_covered,
        set.wrong_actions,
        set.verify_failed,
        set.negative_accepts,
        set.search_exhausted,
        set.action_equivalence_classes,
        set.all_surviving_covers_action_equivalent,
        set.production_admissible,
        set.execution_authority,
    ))
}

pub fn derived_mode_root_v2<T: Serialize>(
    label: &str,
    material: &T,
) -> Result<String, BindingProtocolCompilerErrorV2> {
    protocol_mode_json_sha256(&(PROTOCOL_MODE_SET_SCHEMA_V2, label, material))
}

pub fn protocol_mode_json_sha256<T: Serialize>(
    value: &T,
) -> Result<String, BindingProtocolCompilerErrorV2> {
    canonical_json_sha256(value).map_err(|_| BindingProtocolCompilerErrorV2::Serialization)
}

#[must_use]
pub fn is_protocol_mode_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn pretty_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, BindingProtocolCompilerErrorV2> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|_| BindingProtocolCompilerErrorV2::Serialization)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn constant_roots_v2(contract: &ProtocolConstantContractV2) -> impl Iterator<Item = &String> {
    contract
        .semantic_constants_sha256
        .iter()
        .chain(contract.protocol_noop_constants_sha256.iter())
        .chain(contract.execution_budget_roots_sha256.iter())
        .chain(contract.transport_default_roots_sha256.iter())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    fn candidate() -> BoundedProtocolModeCandidateV2 {
        let selector_program = ProtocolSelectorProgramV2 {
            predicates: Vec::new(),
            max_action_classes: 1,
        };
        let program = ProtocolModeProgramV2 {
            source_role_schema: ProtocolSourceRoleSchemaV2 {
                roles: vec![ProtocolSourceRoleV2 {
                    role_id: 0,
                    value_type: BindingValueTypeV1::Identifier,
                    cardinality: ProtocolRoleCardinalityV2::OneActionClass,
                }],
            },
            selector_program: selector_program.clone(),
            value_contract: ProtocolValueContractV2 {
                observed: BindingValueTypeV1::Identifier,
                emitted: BindingValueTypeV1::Identifier,
            },
            capability_contract: ProtocolCapabilityContractV2 {
                protocol_facet_root_sha256: digest('a'),
                physical_program_ids_sha256: vec![digest('b')],
            },
            argument_role_schema: ProtocolArgumentRoleSchemaV2 {
                roles: vec![ProtocolArgumentRoleV2 {
                    argument_ordinal: 0,
                    source_role_id: 0,
                }],
            },
            constant_contract: ProtocolConstantContractV2 {
                semantic_constants_sha256: Vec::new(),
                protocol_noop_constants_sha256: Vec::new(),
                execution_budget_roots_sha256: Vec::new(),
                transport_default_roots_sha256: Vec::new(),
            },
            structural_guard: ProtocolStructuralGuardV2 {
                relation_identity_sha256: digest('c'),
                effect_invariant_root_sha256: digest('d'),
                selector_program_root_sha256: derived_mode_root_v2(
                    "selector-program",
                    &selector_program,
                )
                .expect("selector digest"),
            },
            temporal_cardinality_contract: ProtocolTemporalCardinalityContractV2 {
                completion_states: Vec::new(),
                temporal_distances: Vec::new(),
                event_candidate_cardinalities: Vec::new(),
                require_unique_action_class: true,
            },
        };
        BoundedProtocolModeCandidateV2 {
            candidate_id_sha256: digest('e'),
            effect_law_id_sha256: digest('f'),
            relation_identity_sha256: digest('c'),
            protocol_facet_root_sha256: digest('a'),
            effect_invariant_root_sha256: digest('d'),
            source_role_schema_root_sha256: derived_mode_root_v2(
                "source-role-schema",
                &program.source_role_schema,
            )
            .expect("role digest"),
            selector_program_root_sha256: derived_mode_root_v2(
                "selector-program",
                &program.selector_program,
            )
            .expect("selector digest"),
            observed_emitted_types_root_sha256: derived_mode_root_v2(
                "observed-emitted-types",
                &program.value_contract,
            )
            .expect("value digest"),
            capability_protocol_root_sha256: derived_mode_root_v2(
                "capability-protocol",
                &program.capability_contract,
            )
            .expect("capability digest"),
            argument_role_schema_root_sha256: derived_mode_root_v2(
                "argument-role-schema",
                &program.argument_role_schema,
            )
            .expect("argument digest"),
            constant_contract_root_sha256: derived_mode_root_v2(
                "constant-contract",
                &program.constant_contract,
            )
            .expect("constant digest"),
            structural_guard_root_sha256: derived_mode_root_v2(
                "structural-guard",
                &program.structural_guard,
            )
            .expect("guard digest"),
            temporal_cardinality_contract_root_sha256: derived_mode_root_v2(
                "temporal-cardinality",
                &program.temporal_cardinality_contract,
            )
            .expect("temporal digest"),
            action_class_root_sha256: digest('1'),
            program,
            covers_positive_rows_sha256: Vec::new(),
            accepts_negative_rows_sha256: Vec::new(),
            wrong_action_rows_sha256: Vec::new(),
            verify_failed_rows_sha256: Vec::new(),
            search_exhausted: false,
        }
    }

    #[test]
    fn protocol_mode_set_roundtrips_canonical_bytes() {
        let candidate = candidate();
        validate_protocol_mode_candidate_v2(&candidate).expect("valid candidate");
        let mode = protocol_mode_from_candidate_v2(&candidate, BTreeSet::from([digest('2')]))
            .expect("mode");
        let mut set = ProtocolModeSetV2 {
            schema: PROTOCOL_MODE_SET_SCHEMA_V2.to_owned(),
            mode_set_sha256: String::new(),
            verdict: BindingProtocolCompileVerdictV2::ProtocolModeSet,
            binding_capability_root_sha256: digest('3'),
            effect_law_id_sha256: candidate.effect_law_id_sha256,
            relation_identity_sha256: candidate.relation_identity_sha256,
            modes: vec![mode],
            positive_rows: 1,
            positive_rows_covered: 1,
            wrong_actions: 0,
            verify_failed: 0,
            negative_accepts: 0,
            search_exhausted: false,
            action_equivalence_classes: 1,
            all_surviving_covers_action_equivalent: true,
            production_admissible: false,
            execution_authority: false,
        };
        set.mode_set_sha256 = protocol_mode_set_digest_v2(&set).expect("set digest");
        let bytes = set.canonical_bytes().expect("canonical bytes");

        assert_eq!(ProtocolModeSetV2::from_canonical_bytes(&bytes), Ok(set));
    }

    #[test]
    fn protocol_mode_validation_stays_fail_closed() {
        let mut candidate = candidate();
        candidate.program.selector_program.max_action_classes = 2;
        assert_eq!(
            validate_protocol_mode_candidate_v2(&candidate),
            Err(BindingProtocolCompilerErrorV2::InvalidCandidate)
        );
        assert_eq!(
            validate_protocol_mode_budget_v2(ProtocolModeCompilerBudgetV2 {
                max_candidates: 0,
                max_surviving_modes: 1,
            }),
            Err(BindingProtocolCompilerErrorV2::InvalidBudget)
        );
    }
}
