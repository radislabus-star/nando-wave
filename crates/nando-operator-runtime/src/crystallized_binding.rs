use std::collections::{BTreeMap, BTreeSet};

use nando_core::wave::{
    Commitment256, LocalRelationFragment, OperatorCircuit, OperatorPage32, RoleGraph,
    RuntimeRoleBinder, SearchCompletion, StructuralRoleSignature, SurfaceFragmentBundle,
    TernaryRelationState, TransformOp8, phase_vector_from_atoms,
};
use nando_operator_kernel::{
    AtomValueType, CollectionOutputRenderer, CollectionProgramStep, CollectionScalarType,
    ResponseOperation, ResponseProgram, ResponseRenderSegment, ResponseValueSelector,
    TRANSFORM_FLAG_CANONICAL_JSON, TRANSFORM_OPCODE_COUNT_COLLECTION,
    TRANSFORM_OPCODE_FILTER_REQUEST_VALUE, TRANSFORM_OPCODE_PROJECT_STATUS,
    TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR, TRANSFORM_ROLE_NONE, TRANSFORM_STATUS_ZERO_IS_TRUE,
    TRANSFORM_VALUE_BOOLEAN, TRANSFORM_VALUE_COLLECTION, TRANSFORM_VALUE_IDENTIFIER,
    TRANSFORM_VALUE_INTEGER, TRANSFORM_VALUE_STRING, canonical_json_sha256,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::is_source_neutral_request_selector;
use crate::{
    ObservedRoleCandidate, ObservedSourceClass, ResponseExecutionStatus,
    execute_operator_page_with_actor, execute_response_unverified, observed_request_ordinal_roles,
    provider_payload_view, selector_candidates,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeBindingError {
    InvalidActor,
    UnsupportedTransformProgram,
    BindingExhausted,
    RelationMismatch,
    MissingAnchor,
    OperandArityMismatch,
    OperandTypeMismatch,
    AmbiguousAction,
    ActorDidNotExecute,
    ActorVmMismatch,
    VmRejected,
    DigestFailure,
    ExpectedActionMissing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidatedRuntimeBindingError<E> {
    Runtime(RuntimeBindingError),
    Validation(E),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeRoleAnchor {
    pub local_role: u8,
    pub selector: ResponseValueSelector,
    pub json_path_sha256: Option<Commitment256>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSurfaceEvidence {
    pub bundle: nando_core::wave::SurfaceFragmentBundle,
    pub request_text: String,
    pub provider_payload: Value,
    pub anchors: Box<[RuntimeRoleAnchor]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundRoleEnvironment {
    surface_sha256: Commitment256,
    local_to_canonical: Box<[u8]>,
    mapping_sha256: Commitment256,
    action_equivalence_sha256: Commitment256,
    phase_fit_fixed: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundRuntimeOperator {
    environment: BoundRoleEnvironment,
    actor: ResponseProgram,
    vm_page: Option<OperatorPage32>,
    bound_selectors: Box<[ResponseValueSelector]>,
    request_text: String,
    provider_payload: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReboundActorCandidate {
    response: String,
    actor_sha256: String,
    actor: ResponseProgram,
}

#[derive(Clone, Copy, Debug)]
pub struct RuntimeOperatorSpec<'a> {
    role_graph: &'a RoleGraph,
    relation_program: &'a OperatorCircuit,
    transform_program: &'a [TransformOp8],
    actor_template: &'a ResponseProgram,
    page: Option<&'a OperatorPage32>,
}

impl<'a> RuntimeOperatorSpec<'a> {
    #[must_use]
    pub const fn new(
        role_graph: &'a RoleGraph,
        relation_program: &'a OperatorCircuit,
        transform_program: &'a [TransformOp8],
        actor_template: &'a ResponseProgram,
        page: Option<&'a OperatorPage32>,
    ) -> Self {
        Self {
            role_graph,
            relation_program,
            transform_program,
            actor_template,
            page,
        }
    }
}

impl ReboundActorCandidate {
    #[must_use]
    pub fn response(&self) -> &str {
        &self.response
    }

    #[must_use]
    pub fn actor_sha256(&self) -> &str {
        &self.actor_sha256
    }

    #[must_use]
    pub const fn actor(&self) -> &ResponseProgram {
        &self.actor
    }
}

pub fn bind_pre_action_with_validator<E, F>(
    operator: RuntimeOperatorSpec<'_>,
    request_text: &str,
    provider_payload: &Value,
    validator: F,
) -> Result<BoundRuntimeOperator, ValidatedRuntimeBindingError<E>>
where
    E: Clone,
    F: Fn(&BoundRuntimeOperator, &str) -> Result<(), E>,
{
    let RuntimeOperatorSpec {
        role_graph,
        relation_program,
        transform_program,
        actor_template,
        page,
    } = operator;
    let transforms = ordered_role_transforms(transform_program)
        .map_err(ValidatedRuntimeBindingError::Runtime)?;
    let payload = serde_json::to_vec(provider_payload).map_err(|_| {
        ValidatedRuntimeBindingError::Runtime(RuntimeBindingError::RelationMismatch)
    })?;
    let lineage_sha256 = digest_parts(
        b"nando.runtime-operator-lineage.v1",
        &[request_text.as_bytes(), &payload],
    );
    let surface_sha256 = digest_parts(
        b"nando.observed-runtime-surface.v1",
        &[request_text.as_bytes(), &payload],
    );
    let provider_view = provider_payload_view(request_text, provider_payload)
        .map_err(|_| ValidatedRuntimeBindingError::Runtime(RuntimeBindingError::MissingAnchor))?;
    let provider_payload = provider_view.as_ref();
    if let Some(filter) = transforms
        .iter()
        .find(|transform| transform.opcode == TRANSFORM_OPCODE_FILTER_REQUEST_VALUE)
    {
        let predicate_type = transform_value_type(filter.parameter & 0x00ff)
            .map_err(ValidatedRuntimeBindingError::Runtime)?;
        let mut actions = BTreeMap::<String, BoundRuntimeOperator>::new();
        let mut first_blocker = None;
        let mut deepest_blocker = None;
        let evidence = filter_runtime_evidence_candidates(
            request_text,
            provider_payload,
            lineage_sha256,
            surface_sha256,
            predicate_type,
        )
        .map_err(ValidatedRuntimeBindingError::Runtime)?;
        for evidence in evidence {
            let bound = match bind_operator_components(
                role_graph,
                relation_program,
                transform_program,
                actor_template,
                evidence,
            ) {
                Ok(bound) => match page {
                    Some(page) => bound.with_vm_page(page.clone()),
                    None => bound,
                },
                Err(error) => {
                    let wrapped = ValidatedRuntimeBindingError::Runtime(error);
                    first_blocker.get_or_insert(wrapped.clone());
                    if error != RuntimeBindingError::RelationMismatch {
                        deepest_blocker.get_or_insert(wrapped);
                    }
                    continue;
                }
            };
            let response = match bound.execute_unverified() {
                Ok(response) => response,
                Err(error) => {
                    let wrapped = ValidatedRuntimeBindingError::Runtime(error);
                    first_blocker.get_or_insert(wrapped.clone());
                    deepest_blocker.get_or_insert(wrapped);
                    continue;
                }
            };
            if let Err(error) = validator(&bound, &response) {
                let wrapped = ValidatedRuntimeBindingError::Validation(error);
                first_blocker.get_or_insert(wrapped.clone());
                deepest_blocker.get_or_insert(wrapped);
                continue;
            }
            actions.entry(response).or_insert(bound);
        }
        return match actions.len() {
            0 => Err(deepest_blocker.or(first_blocker).unwrap_or(
                ValidatedRuntimeBindingError::Runtime(RuntimeBindingError::MissingAnchor),
            )),
            1 => Ok(actions.into_values().next().expect("one action class")),
            _ => Err(ValidatedRuntimeBindingError::Runtime(
                RuntimeBindingError::AmbiguousAction,
            )),
        };
    }
    if transforms.len() == 1 {
        let expected_type = transform_value_type(transforms[0].parameter & 0x00ff)
            .map_err(ValidatedRuntimeBindingError::Runtime)?;
        let mut actions = BTreeMap::<String, BoundRuntimeOperator>::new();
        for selector in runtime_selector_candidates(
            provider_payload,
            expected_type,
            actor_primary_selector(actor_template),
        )
        .filter(|selector| selector_value_type(selector) == Some(expected_type))
        {
            let Ok(evidence) = observed_scalar_runtime_surface(
                request_text,
                provider_payload,
                selector,
                lineage_sha256,
                surface_sha256,
            ) else {
                continue;
            };
            let Ok(bound) = bind_operator_components(
                role_graph,
                relation_program,
                transform_program,
                actor_template,
                evidence,
            ) else {
                continue;
            };
            let bound = match page {
                Some(page) => bound.with_vm_page(page.clone()),
                None => bound,
            };
            let Ok(response) = bound.execute_unverified() else {
                continue;
            };
            if validator(&bound, &response).is_err() {
                continue;
            }
            actions.entry(response).or_insert(bound);
        }
        return match actions.len() {
            0 => Err(ValidatedRuntimeBindingError::Runtime(
                RuntimeBindingError::MissingAnchor,
            )),
            1 => Ok(actions.into_values().next().expect("one action class")),
            _ => Err(ValidatedRuntimeBindingError::Runtime(
                RuntimeBindingError::AmbiguousAction,
            )),
        };
    }
    let observed = observed_request_ordinal_roles(request_text, provider_payload)
        .map_err(|_| ValidatedRuntimeBindingError::Runtime(RuntimeBindingError::MissingAnchor))?;
    if observed.len() != transforms.len() {
        return Err(ValidatedRuntimeBindingError::Runtime(
            RuntimeBindingError::MissingAnchor,
        ));
    }
    let evidence = observed_multi_role_runtime_surface(
        request_text,
        provider_payload,
        &observed,
        lineage_sha256,
        surface_sha256,
    )
    .map_err(ValidatedRuntimeBindingError::Runtime)?;
    let bound = bind_operator_components(
        role_graph,
        relation_program,
        transform_program,
        actor_template,
        evidence,
    )
    .map_err(ValidatedRuntimeBindingError::Runtime)?;
    let bound = match page {
        Some(page) => bound.with_vm_page(page.clone()),
        None => bound,
    };
    let response = bound
        .execute_unverified()
        .map_err(ValidatedRuntimeBindingError::Runtime)?;
    validator(&bound, &response).map_err(ValidatedRuntimeBindingError::Validation)?;
    Ok(bound)
}

pub fn independently_rebound_actor_candidates(
    role_graph: &RoleGraph,
    relation_program: &OperatorCircuit,
    transform_program: &[TransformOp8],
    actor_template: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Result<Vec<ReboundActorCandidate>, RuntimeBindingError> {
    let transforms = ordered_role_transforms(transform_program)?;
    if let Some(filter) = transforms
        .iter()
        .find(|transform| transform.opcode == TRANSFORM_OPCODE_FILTER_REQUEST_VALUE)
    {
        let payload = serde_json::to_vec(provider_payload)
            .map_err(|_| RuntimeBindingError::RelationMismatch)?;
        let predicate_type = transform_value_type(filter.parameter & 0x00ff)?;
        let evidence = filter_runtime_evidence_candidates(
            request_text,
            provider_payload,
            digest_parts(
                b"nando.independent-filter-lineage.v1",
                &[request_text.as_bytes(), &payload],
            ),
            digest_parts(
                b"nando.independent-filter-surface.v1",
                &[request_text.as_bytes(), &payload],
            ),
            predicate_type,
        )?;
        let mut programs = BTreeMap::<String, ResponseProgram>::new();
        for surface in evidence {
            let report = RuntimeRoleBinder::bind(
                role_graph,
                relation_program,
                &surface.bundle,
                nando_core::wave::OPERATOR_BLUEPRINT_MAX_ALIGNMENTS,
            );
            if !matches!(report.completion(), SearchCompletion::Complete { .. }) {
                return Err(RuntimeBindingError::BindingExhausted);
            }
            for mapping in report.mappings() {
                let selectors = selectors_for_mapping(&transforms, mapping, &surface)?;
                let program = instantiate_bound_actor(actor_template, &transforms, &selectors)?;
                if execute_response_unverified(&program, request_text, provider_payload)
                    .response
                    .as_deref()
                    == Some(expected_response)
                {
                    programs.entry(actor_digest(&program)?).or_insert(program);
                }
            }
        }
        return candidates_for_expected(programs, expected_response);
    }

    let mut response_classes = BTreeMap::<String, BTreeMap<String, ResponseProgram>>::new();
    if transforms.len() == 1 {
        let expected_type = transform_value_type(transforms[0].parameter & 0x00ff)?;
        for selector in runtime_selector_candidates(
            provider_payload,
            expected_type,
            actor_primary_selector(actor_template),
        )
        .filter(|selector| selector_value_type(selector) == Some(expected_type))
        {
            let program = instantiate_bound_actor(
                actor_template,
                &transforms,
                std::slice::from_ref(&selector),
            )?;
            let Some(response) =
                execute_response_unverified(&program, request_text, provider_payload).response
            else {
                continue;
            };
            response_classes
                .entry(response)
                .or_default()
                .entry(actor_digest(&program)?)
                .or_insert(program);
        }
    } else {
        let observed = observed_request_ordinal_roles(request_text, provider_payload)
            .map_err(|_| RuntimeBindingError::MissingAnchor)?;
        if observed.len() != transforms.len() {
            return Err(RuntimeBindingError::MissingAnchor);
        }
        let payload = serde_json::to_vec(provider_payload)
            .map_err(|_| RuntimeBindingError::RelationMismatch)?;
        let evidence = observed_multi_role_runtime_surface(
            request_text,
            provider_payload,
            &observed,
            digest_parts(
                b"nando.independent-verifier-lineage.v1",
                &[request_text.as_bytes(), &payload],
            ),
            digest_parts(
                b"nando.independent-verifier-surface.v1",
                &[request_text.as_bytes(), &payload],
            ),
        )?;
        let report = RuntimeRoleBinder::bind(
            role_graph,
            relation_program,
            &evidence.bundle,
            nando_core::wave::OPERATOR_BLUEPRINT_MAX_ALIGNMENTS,
        );
        if !matches!(report.completion(), SearchCompletion::Complete { .. })
            || report.mappings().is_empty()
        {
            return Err(RuntimeBindingError::RelationMismatch);
        }
        for mapping in report.mappings() {
            let selectors = selectors_for_mapping(&transforms, mapping, &evidence)?;
            let program = instantiate_bound_actor(actor_template, &transforms, &selectors)?;
            let Some(response) =
                execute_response_unverified(&program, request_text, provider_payload).response
            else {
                continue;
            };
            response_classes
                .entry(response)
                .or_default()
                .entry(actor_digest(&program)?)
                .or_insert(program);
        }
    }
    if response_classes.len() != 1 {
        return Err(RuntimeBindingError::AmbiguousAction);
    }
    let (response, programs) = response_classes
        .into_iter()
        .next()
        .ok_or(RuntimeBindingError::ExpectedActionMissing)?;
    if response != expected_response {
        return Err(RuntimeBindingError::ExpectedActionMissing);
    }
    candidates_for_expected(programs, &response)
}

fn selectors_for_mapping(
    transforms: &[TransformOp8],
    mapping: &nando_core::wave::RuntimeRoleMapping,
    evidence: &RuntimeSurfaceEvidence,
) -> Result<Vec<ResponseValueSelector>, RuntimeBindingError> {
    ordered_transform_operand_roles(transforms)
        .into_iter()
        .map(|source_role| {
            let local_role = mapping
                .local_role_for(source_role)
                .ok_or(RuntimeBindingError::MissingAnchor)?;
            evidence
                .anchors
                .iter()
                .find(|anchor| anchor.local_role == local_role)
                .map(|anchor| anchor.selector.clone())
                .ok_or(RuntimeBindingError::MissingAnchor)
        })
        .collect()
}

fn candidates_for_expected(
    programs: BTreeMap<String, ResponseProgram>,
    expected_response: &str,
) -> Result<Vec<ReboundActorCandidate>, RuntimeBindingError> {
    if programs.is_empty() {
        return Err(RuntimeBindingError::ExpectedActionMissing);
    }
    Ok(programs
        .into_iter()
        .map(|(actor_sha256, actor)| ReboundActorCandidate {
            response: expected_response.to_owned(),
            actor_sha256,
            actor,
        })
        .collect())
}

impl BoundRuntimeOperator {
    pub fn execute_unverified(&self) -> Result<String, RuntimeBindingError> {
        let execution =
            execute_response_unverified(&self.actor, &self.request_text, &self.provider_payload);
        if execution.status != ResponseExecutionStatus::Executed {
            return Err(RuntimeBindingError::ActorDidNotExecute);
        }
        let reference_response = execution
            .response
            .ok_or(RuntimeBindingError::ActorDidNotExecute)?;
        let response = match &self.vm_page {
            Some(page) => execute_operator_page_with_actor(
                page,
                &self.bound_selectors,
                &self.request_text,
                &self.provider_payload,
                &self.actor,
            )
            .map_err(|_| RuntimeBindingError::VmRejected)?,
            None => reference_response.clone(),
        };
        if response != reference_response {
            return Err(RuntimeBindingError::ActorVmMismatch);
        }
        Ok(response)
    }

    #[must_use]
    pub fn with_vm_page(mut self, page: OperatorPage32) -> Self {
        self.vm_page = Some(page);
        self
    }

    #[must_use]
    pub const fn environment(&self) -> &BoundRoleEnvironment {
        &self.environment
    }

    #[must_use]
    pub const fn actor(&self) -> &ResponseProgram {
        &self.actor
    }

    #[must_use]
    pub fn bound_selectors(&self) -> &[ResponseValueSelector] {
        &self.bound_selectors
    }

    #[must_use]
    pub fn request_text(&self) -> &str {
        &self.request_text
    }

    #[must_use]
    pub const fn provider_payload(&self) -> &Value {
        &self.provider_payload
    }
}

impl BoundRoleEnvironment {
    #[must_use]
    pub const fn surface_sha256(&self) -> &Commitment256 {
        &self.surface_sha256
    }

    #[must_use]
    pub fn local_to_canonical(&self) -> &[u8] {
        &self.local_to_canonical
    }

    #[must_use]
    pub const fn mapping_sha256(&self) -> &Commitment256 {
        &self.mapping_sha256
    }

    #[must_use]
    pub const fn action_equivalence_sha256(&self) -> &Commitment256 {
        &self.action_equivalence_sha256
    }

    #[must_use]
    pub const fn phase_fit_fixed(&self) -> i64 {
        self.phase_fit_fixed
    }
}

#[doc(hidden)]
pub fn observed_scalar_runtime_surface(
    request_text: &str,
    provider_payload: &Value,
    selector: ResponseValueSelector,
    lineage_sha256: Commitment256,
    surface_sha256: Commitment256,
) -> Result<RuntimeSurfaceEvidence, RuntimeBindingError> {
    if request_text.trim().is_empty() && is_source_neutral_request_selector(&selector) {
        return Err(RuntimeBindingError::MissingAnchor);
    }
    let value_type = selector_value_type(&selector).ok_or(RuntimeBindingError::MissingAnchor)?;
    let context = 0_u8;
    let source = 1_u8;
    let roles = vec![
        StructuralRoleSignature::new(5, 1, 0, 1, vec![0]),
        runtime_role_signature_for_selector(&selector, 0),
    ];
    let phase_atoms = [
        format!("scalar_type:{}", runtime_value_type_tag(value_type)),
        "cardinality:unique".to_owned(),
    ];
    let phase = phase_vector_from_atoms(phase_atoms.iter().map(String::as_str), 1)[0];
    let bundle = SurfaceFragmentBundle::new(
        lineage_sha256,
        surface_sha256,
        roles,
        vec![LocalRelationFragment {
            plane: 0,
            source_local_role: context,
            target_local_role: source,
            state: TernaryRelationState::Supported,
            phase_anchor: phase,
        }],
        Vec::new(),
    )
    .map_err(|_| RuntimeBindingError::RelationMismatch)?;
    Ok(RuntimeSurfaceEvidence {
        bundle,
        request_text: request_text.to_owned(),
        provider_payload: provider_payload.clone(),
        anchors: vec![RuntimeRoleAnchor {
            local_role: source,
            selector,
            json_path_sha256: None,
        }]
        .into_boxed_slice(),
    })
}

#[doc(hidden)]
pub fn observed_multi_role_runtime_surface(
    request_text: &str,
    provider_payload: &Value,
    observed_roles: &[ObservedRoleCandidate],
    lineage_sha256: Commitment256,
    surface_sha256: Commitment256,
) -> Result<RuntimeSurfaceEvidence, RuntimeBindingError> {
    if observed_roles.len() < 2 || observed_roles.len() > 16 {
        return Err(RuntimeBindingError::RelationMismatch);
    }
    let role_count = observed_roles.len().saturating_add(1);
    let context = 0_u8;
    let planes = (0..observed_roles.len())
        .map(|index| u8::try_from(index).map_err(|_| RuntimeBindingError::RelationMismatch))
        .collect::<Result<Vec<_>, _>>()?;
    let mut roles = vec![StructuralRoleSignature::new(0, 0, 0, 0, Vec::new()); role_count];
    roles[usize::from(context)] = StructuralRoleSignature::new(5, 1, 0, 1, planes.clone());
    let mut relations = Vec::with_capacity(observed_roles.len());
    let mut anchors = Vec::with_capacity(observed_roles.len());
    for (index, observed) in observed_roles.iter().enumerate() {
        let source = u8::try_from(index + 1).map_err(|_| RuntimeBindingError::RelationMismatch)?;
        let plane = u8::try_from(index).map_err(|_| RuntimeBindingError::RelationMismatch)?;
        let value_type = observed.value_type;
        roles[usize::from(source)] =
            runtime_multi_role_signature_for_selector(&observed.selector, plane);
        let phase_atom = format!(
            "scalar_role:{index}:type:{}",
            runtime_value_type_tag(value_type)
        );
        let phase = phase_vector_from_atoms([phase_atom.as_str()], 1)[0];
        relations.push(LocalRelationFragment {
            plane,
            source_local_role: context,
            target_local_role: source,
            state: TernaryRelationState::Supported,
            phase_anchor: phase,
        });
        anchors.push(RuntimeRoleAnchor {
            local_role: source,
            selector: observed.selector.clone(),
            json_path_sha256: Some(observed.json_path_sha256),
        });
    }
    let bundle =
        SurfaceFragmentBundle::new(lineage_sha256, surface_sha256, roles, relations, Vec::new())
            .map_err(|_| RuntimeBindingError::RelationMismatch)?;
    Ok(RuntimeSurfaceEvidence {
        bundle,
        request_text: request_text.to_owned(),
        provider_payload: provider_payload.clone(),
        anchors: anchors.into_boxed_slice(),
    })
}

#[doc(hidden)]
pub fn filter_runtime_evidence_candidates(
    request_text: &str,
    provider_payload: &Value,
    lineage_sha256: Commitment256,
    surface_sha256: Commitment256,
    predicate_type: AtomValueType,
) -> Result<Vec<RuntimeSurfaceEvidence>, RuntimeBindingError> {
    if request_text.trim().is_empty() {
        return Err(RuntimeBindingError::MissingAnchor);
    }
    let collection_selector = ResponseValueSelector::UniqueScalar {
        value_type: AtomValueType::Collection,
    };
    let mut evidence = Vec::new();
    for (index, predicate_selector) in selector_candidates(provider_payload)
        .into_iter()
        .filter(|selector| selector_value_type(selector) == Some(predicate_type))
        .filter(is_source_neutral_request_selector)
        .take(64)
        .enumerate()
    {
        let selectors = [collection_selector.clone(), predicate_selector];
        let observed = selectors
            .into_iter()
            .enumerate()
            .map(|(role_index, selector)| ObservedRoleCandidate {
                value_type: selector_value_type(&selector).unwrap_or(AtomValueType::String),
                selector,
                request_position: u16::try_from(role_index).unwrap_or(u16::MAX),
                json_path_sha256: digest_parts(
                    b"nando.runtime-filter-role.v1",
                    &[
                        &(index as u64).to_le_bytes(),
                        &(role_index as u64).to_le_bytes(),
                    ],
                ),
                source_class: ObservedSourceClass::ImmediateToolJson,
            })
            .collect::<Vec<_>>();
        if let Ok(candidate) = observed_multi_role_runtime_surface(
            request_text,
            provider_payload,
            &observed,
            lineage_sha256,
            surface_sha256,
        ) {
            evidence.push(candidate);
        }
    }
    if evidence.is_empty() {
        return Err(RuntimeBindingError::MissingAnchor);
    }
    Ok(evidence)
}

#[doc(hidden)]
#[must_use]
pub fn runtime_role_signature_for_selector(
    selector: &ResponseValueSelector,
    plane: u8,
) -> StructuralRoleSignature {
    let value_type = selector_value_type(selector).unwrap_or(AtomValueType::String);
    let (temporal_position, source_mask) = match selector {
        ResponseValueSelector::RequestLastToken | ResponseValueSelector::RequestUniqueLiteral => {
            (0, 0x0100)
        }
        _ => (1, 0x0200),
    };
    StructuralRoleSignature::new(
        runtime_value_type_tag(value_type),
        1,
        temporal_position,
        2 | source_mask,
        vec![plane],
    )
}

#[doc(hidden)]
#[must_use]
pub fn runtime_multi_role_signature_for_selector(
    selector: &ResponseValueSelector,
    plane: u8,
) -> StructuralRoleSignature {
    let value_type = selector_value_type(selector).unwrap_or(AtomValueType::String);
    let temporal_position = match selector {
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal { ordinal, .. } => {
            u8::try_from(ordinal.saturating_add(1)).unwrap_or(u8::MAX)
        }
        ResponseValueSelector::RequestLastToken | ResponseValueSelector::RequestUniqueLiteral => 0,
        _ => 1,
    };
    StructuralRoleSignature::new(
        runtime_value_type_tag(value_type),
        1,
        temporal_position,
        2 | if temporal_position == 0 {
            0x0100
        } else {
            0x0200
        },
        vec![plane],
    )
}

const fn runtime_value_type_tag(value_type: AtomValueType) -> u8 {
    match value_type {
        AtomValueType::String => 1,
        AtomValueType::Integer => 2,
        AtomValueType::Boolean => 3,
        AtomValueType::Identifier => 4,
        AtomValueType::Collection => 5,
    }
}

pub fn bind_operator_components(
    role_graph: &RoleGraph,
    relation_program: &OperatorCircuit,
    transform_program: &[TransformOp8],
    actor_template: &ResponseProgram,
    evidence: RuntimeSurfaceEvidence,
) -> Result<BoundRuntimeOperator, RuntimeBindingError> {
    let report = RuntimeRoleBinder::bind(
        role_graph,
        relation_program,
        &evidence.bundle,
        nando_core::wave::OPERATOR_BLUEPRINT_MAX_ALIGNMENTS,
    );
    if !matches!(report.completion(), SearchCompletion::Complete { .. }) {
        return Err(RuntimeBindingError::BindingExhausted);
    }
    if report.mappings().is_empty() {
        return Err(RuntimeBindingError::RelationMismatch);
    }
    let ordered_transforms = ordered_role_transforms(transform_program)?;
    let mut actions = BTreeMap::<Commitment256, Vec<_>>::new();
    for mapping in report.mappings() {
        let selectors = ordered_transform_operand_roles(&ordered_transforms)
            .into_iter()
            .map(|source_role| {
                let source_local_role = mapping
                    .local_role_for(source_role)
                    .ok_or(RuntimeBindingError::MissingAnchor)?;
                evidence
                    .anchors
                    .iter()
                    .find(|anchor| anchor.local_role == source_local_role)
                    .map(|anchor| anchor.selector.clone())
                    .ok_or(RuntimeBindingError::MissingAnchor)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let actor = instantiate_bound_actor(actor_template, &ordered_transforms, &selectors)?;
        let execution =
            execute_response_unverified(&actor, &evidence.request_text, &evidence.provider_payload);
        let response = execution
            .response
            .ok_or(RuntimeBindingError::ActorDidNotExecute)?;
        let action_sha256 = digest_parts(
            b"nando.bound-action.v1",
            &[
                response.as_bytes(),
                &ordered_transforms
                    .iter()
                    .map(|transform| transform.output)
                    .collect::<Vec<_>>(),
            ],
        );
        actions
            .entry(action_sha256)
            .or_default()
            .push((mapping, actor, selectors));
    }
    if actions.len() != 1 {
        return Err(RuntimeBindingError::AmbiguousAction);
    }
    let (action_equivalence_sha256, mut equivalent) =
        actions.into_iter().next().expect("one action class");
    equivalent.sort_by(|left, right| {
        left.0
            .local_to_canonical()
            .cmp(right.0.local_to_canonical())
    });
    let (mapping, actor, bound_selectors) = equivalent.remove(0);
    let mut mapping_commitment = mapping.local_to_canonical().to_vec();
    let mut anchors = evidence.anchors.iter().collect::<Vec<_>>();
    anchors.sort_by_key(|anchor| anchor.local_role);
    for anchor in anchors {
        mapping_commitment.push(anchor.local_role);
        match anchor.json_path_sha256 {
            Some(path_sha256) => {
                mapping_commitment.push(1);
                mapping_commitment.extend_from_slice(&path_sha256);
            }
            None => {
                mapping_commitment.push(0);
                mapping_commitment.extend_from_slice(&[0; 32]);
            }
        }
    }
    let mapping_sha256 = digest_parts(b"nando.bound-role-mapping.v2", &[&mapping_commitment]);
    Ok(BoundRuntimeOperator {
        environment: BoundRoleEnvironment {
            surface_sha256: *evidence.bundle.surface_sha256(),
            local_to_canonical: mapping.local_to_canonical().into(),
            mapping_sha256,
            action_equivalence_sha256,
            phase_fit_fixed: mapping.phase_fit_fixed(),
        },
        actor,
        vm_page: None,
        bound_selectors: bound_selectors.into_boxed_slice(),
        request_text: evidence.request_text,
        provider_payload: evidence.provider_payload,
    })
}

#[doc(hidden)]
pub fn ordered_role_transforms(
    transform_program: &[TransformOp8],
) -> Result<Vec<TransformOp8>, RuntimeBindingError> {
    if transform_program.is_empty() || transform_program.len() > 16 {
        return Err(RuntimeBindingError::UnsupportedTransformProgram);
    }
    let mut ordered = transform_program.to_vec();
    ordered.sort_by_key(|transform| transform.parameter >> 8);
    let mut outputs = BTreeMap::new();
    for (index, transform) in ordered.iter().enumerate() {
        if outputs.insert(transform.output, index).is_some() {
            return Err(RuntimeBindingError::UnsupportedTransformProgram);
        }
    }
    for (index, transform) in ordered.iter().enumerate() {
        if transform.output == transform.source_a || usize::from(transform.parameter >> 8) != index
        {
            return Err(RuntimeBindingError::UnsupportedTransformProgram);
        }
        if transform.source_b != TRANSFORM_ROLE_NONE
            && (transform.source_b == transform.output || transform.source_b == transform.source_a)
        {
            return Err(RuntimeBindingError::UnsupportedTransformProgram);
        }
        for source in [transform.source_a, transform.source_b] {
            if source != TRANSFORM_ROLE_NONE
                && outputs
                    .get(&source)
                    .is_some_and(|producer| *producer >= index)
            {
                return Err(RuntimeBindingError::UnsupportedTransformProgram);
            }
        }
        validate_typed_transform(*transform)?;
    }
    Ok(ordered)
}

#[doc(hidden)]
#[must_use]
pub fn ordered_transform_operand_roles(transforms: &[TransformOp8]) -> Vec<u8> {
    let produced = transforms
        .iter()
        .map(|transform| transform.output)
        .collect::<BTreeSet<_>>();
    let mut external = Vec::new();
    for transform in transforms {
        for source in [transform.source_a, transform.source_b] {
            if source != TRANSFORM_ROLE_NONE
                && !produced.contains(&source)
                && !external.contains(&source)
            {
                external.push(source);
            }
        }
    }
    external
}

#[doc(hidden)]
pub fn instantiate_bound_actor(
    template: &ResponseProgram,
    transforms: &[TransformOp8],
    selectors: &[ResponseValueSelector],
) -> Result<ResponseProgram, RuntimeBindingError> {
    let produced = transforms
        .iter()
        .map(|transform| transform.output)
        .collect::<BTreeSet<_>>();
    let mut expected_by_role = BTreeMap::new();
    for transform in transforms {
        let operand_roles = [transform.source_a, transform.source_b];
        let operand_types = transform_operand_types(transform)?;
        for (role, value_type) in operand_roles.into_iter().zip(operand_types) {
            if role == TRANSFORM_ROLE_NONE || produced.contains(&role) {
                continue;
            }
            if expected_by_role
                .insert(role, value_type)
                .is_some_and(|known| known != value_type)
            {
                return Err(RuntimeBindingError::OperandTypeMismatch);
            }
        }
    }
    let expected_types = ordered_transform_operand_roles(transforms)
        .into_iter()
        .filter_map(|role| expected_by_role.get(&role).copied())
        .collect::<Vec<_>>();
    if expected_types.len() != selectors.len() {
        return Err(RuntimeBindingError::OperandArityMismatch);
    }
    for (expected_type, selector) in expected_types.iter().zip(selectors) {
        if selector_value_type(selector) != Some(*expected_type) {
            return Err(RuntimeBindingError::OperandTypeMismatch);
        }
    }
    let mut actor = template.clone();
    bind_program_selectors(&mut actor, selectors)?;
    actor
        .validate()
        .map_err(|_| RuntimeBindingError::InvalidActor)?;
    Ok(actor)
}

fn bind_program_selectors(
    program: &mut ResponseProgram,
    selectors: &[ResponseValueSelector],
) -> Result<(), RuntimeBindingError> {
    match &mut program.operation {
        ResponseOperation::FunctionCallFromRoles { selector, .. }
        | ResponseOperation::CustomToolCallFromRoles { selector, .. } => {
            let [primary] = selectors else {
                return Err(RuntimeBindingError::MissingAnchor);
            };
            *selector = primary.clone();
        }
        ResponseOperation::ProjectSelectedValue {
            selector, renderer, ..
        } => {
            let Some(primary) = selectors.first() else {
                return Err(RuntimeBindingError::MissingAnchor);
            };
            *selector = primary.clone();
            let mut next = 1_usize;
            if let CollectionOutputRenderer::RenderSequence { segments } = renderer {
                for segment in segments {
                    if let ResponseRenderSegment::Selected { selector, .. } = segment {
                        *selector = selectors
                            .get(next)
                            .cloned()
                            .ok_or(RuntimeBindingError::MissingAnchor)?;
                        next = next.saturating_add(1);
                    }
                }
            }
            if next != selectors.len() {
                return Err(RuntimeBindingError::UnsupportedTransformProgram);
            }
        }
        ResponseOperation::ComposeCollection { steps, .. } => {
            let Some(collection_selector) = selectors.first() else {
                return Err(RuntimeBindingError::MissingAnchor);
            };
            let count = matches!(
                steps.as_slice(),
                [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::Count
                ]
            );
            let filter_type = match steps.as_slice() {
                [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
                        value_type, ..
                    },
                ]
                | [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
                        value_type, ..
                    },
                    CollectionProgramStep::Count,
                ] => Some(collection_atom_type(*value_type)),
                _ => None,
            };
            if selector_value_type(collection_selector) != Some(AtomValueType::Collection)
                || !((count && selectors.len() == 1)
                    || (selectors.len() == 2 && filter_type == selector_value_type(&selectors[1])))
            {
                return Err(RuntimeBindingError::UnsupportedTransformProgram);
            }
            if filter_type.is_some() {
                let [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
                        selector, ..
                    },
                    tail @ ..,
                ] = steps.as_mut_slice()
                else {
                    return Err(RuntimeBindingError::UnsupportedTransformProgram);
                };
                if !matches!(tail, [] | [CollectionProgramStep::Count]) {
                    return Err(RuntimeBindingError::UnsupportedTransformProgram);
                }
                *selector = selectors[1].clone();
            }
        }
        ResponseOperation::ProjectStatus { selector, .. } => {
            let [primary] = selectors else {
                return Err(RuntimeBindingError::MissingAnchor);
            };
            if selector_value_type(primary) != Some(AtomValueType::Integer) {
                return Err(RuntimeBindingError::MissingAnchor);
            }
            *selector = primary.clone();
        }
        ResponseOperation::UniqueConsensus { variants, .. } => {
            if variants.is_empty() {
                return Err(RuntimeBindingError::UnsupportedTransformProgram);
            }
            for variant in variants {
                bind_program_selectors(&mut variant.program, selectors)?;
            }
        }
        _ => return Err(RuntimeBindingError::UnsupportedTransformProgram),
    }
    Ok(())
}

#[doc(hidden)]
#[must_use]
pub const fn selector_value_type(selector: &ResponseValueSelector) -> Option<AtomValueType> {
    match selector {
        ResponseValueSelector::ContinuationHandle { value_type }
        | ResponseValueSelector::UniqueScalar { value_type }
        | ResponseValueSelector::UniqueTurnScalar { value_type }
        | ResponseValueSelector::ContentLinePrefix { value_type, .. }
        | ResponseValueSelector::JsonField { value_type, .. }
        | ResponseValueSelector::JsonScalarOrdinal { value_type, .. }
        | ResponseValueSelector::UniqueTurnJsonField { value_type, .. }
        | ResponseValueSelector::UniqueActiveTurnJsonField { value_type, .. }
        | ResponseValueSelector::RequestReferencedJsonField { value_type }
        | ResponseValueSelector::RequestReferencedJsonFieldOrdinal { value_type, .. }
        | ResponseValueSelector::TurnOutputLine { value_type, .. }
        | ResponseValueSelector::TurnOutputScalarOrdinal { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputLine { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputScalarOrdinal { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputScalarFromEnd { value_type, .. } => {
            Some(*value_type)
        }
        ResponseValueSelector::CommandOutputBody
        | ResponseValueSelector::RequestLastToken
        | ResponseValueSelector::RequestUniqueLiteral => Some(AtomValueType::String),
    }
}

#[doc(hidden)]
pub fn runtime_selector_candidates<'a>(
    provider_payload: &'a Value,
    expected_type: AtomValueType,
    preferred: Option<&'a ResponseValueSelector>,
) -> impl Iterator<Item = ResponseValueSelector> + 'a {
    let mut candidates = preferred.cloned().into_iter().collect::<Vec<_>>();
    candidates.extend(if expected_type == AtomValueType::Collection {
        vec![ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::Collection,
        }]
    } else {
        crate::selector_candidates(provider_payload)
    });
    candidates.sort();
    candidates.dedup();
    candidates.into_iter()
}

#[doc(hidden)]
#[must_use]
pub fn actor_primary_selector(program: &ResponseProgram) -> Option<&ResponseValueSelector> {
    match &program.operation {
        ResponseOperation::FunctionCallFromRoles { selector, .. }
        | ResponseOperation::CustomToolCallFromRoles { selector, .. }
        | ResponseOperation::ProjectSelectedValue { selector, .. }
        | ResponseOperation::ProjectStatus { selector, .. } => Some(selector),
        _ => None,
    }
}

#[doc(hidden)]
pub fn validate_typed_transform(transform: TransformOp8) -> Result<(), RuntimeBindingError> {
    if transform.output == transform.source_a {
        return Err(RuntimeBindingError::UnsupportedTransformProgram);
    }
    let value_type = transform_value_type(transform.parameter & 0x00ff)?;
    match transform.opcode {
        TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR
            if value_type != AtomValueType::Collection
                && transform.source_b == TRANSFORM_ROLE_NONE => {}
        TRANSFORM_OPCODE_COUNT_COLLECTION
            if value_type == AtomValueType::Collection
                && transform.source_b == TRANSFORM_ROLE_NONE
                && transform.flags == 0 => {}
        TRANSFORM_OPCODE_PROJECT_STATUS
            if value_type == AtomValueType::Integer
                && transform.source_b == TRANSFORM_ROLE_NONE
                && transform.flags <= TRANSFORM_STATUS_ZERO_IS_TRUE => {}
        TRANSFORM_OPCODE_FILTER_REQUEST_VALUE
            if matches!(
                value_type,
                AtomValueType::String | AtomValueType::Integer | AtomValueType::Boolean
            ) && transform.source_b != TRANSFORM_ROLE_NONE
                && transform.source_b != transform.output
                && transform.source_b != transform.source_a
                && transform.flags == TRANSFORM_FLAG_CANONICAL_JSON => {}
        _ => return Err(RuntimeBindingError::UnsupportedTransformProgram),
    }
    Ok(())
}

#[doc(hidden)]
pub fn transform_operand_types(
    transform: &TransformOp8,
) -> Result<Vec<AtomValueType>, RuntimeBindingError> {
    let value_type = transform_value_type(transform.parameter & 0x00ff)?;
    if transform.opcode == TRANSFORM_OPCODE_FILTER_REQUEST_VALUE {
        Ok(vec![AtomValueType::Collection, value_type])
    } else {
        Ok(vec![value_type])
    }
}

#[doc(hidden)]
pub fn transform_value_type(parameter: u16) -> Result<AtomValueType, RuntimeBindingError> {
    Ok(match parameter {
        TRANSFORM_VALUE_STRING => AtomValueType::String,
        TRANSFORM_VALUE_INTEGER => AtomValueType::Integer,
        TRANSFORM_VALUE_BOOLEAN => AtomValueType::Boolean,
        TRANSFORM_VALUE_IDENTIFIER => AtomValueType::Identifier,
        TRANSFORM_VALUE_COLLECTION => AtomValueType::Collection,
        _ => return Err(RuntimeBindingError::UnsupportedTransformProgram),
    })
}

const fn collection_atom_type(value_type: CollectionScalarType) -> AtomValueType {
    match value_type {
        CollectionScalarType::String => AtomValueType::String,
        CollectionScalarType::Integer => AtomValueType::Integer,
        CollectionScalarType::Boolean => AtomValueType::Boolean,
    }
}

#[doc(hidden)]
pub fn digest_parts(domain: &[u8], parts: &[&[u8]]) -> Commitment256 {
    let mut digest = Sha256::new();
    digest.update(domain);
    for part in parts {
        digest.update((part.len() as u64).to_le_bytes());
        digest.update(part);
    }
    digest.finalize().into()
}

#[doc(hidden)]
pub fn actor_digest(program: &ResponseProgram) -> Result<String, RuntimeBindingError> {
    canonical_json_sha256(program).map_err(|_| RuntimeBindingError::DigestFailure)
}
