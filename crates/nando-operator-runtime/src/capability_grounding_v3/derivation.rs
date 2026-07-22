use nando_operator_kernel::{
    BoundProtocolActionInputV3, BoundProtocolArgumentInputV3, RuntimeCapabilityDescriptorV3,
    build_bound_protocol_action_v3, canonical_json_sha256,
};

use super::{ActionDerivationVerdictV3, MappingActionAttemptV3};
use crate::{
    CanonicalRuntimeRequestV3, CompiledProtocolModeV3, RuntimeCapabilityBindingV3,
    RuntimeStructuralMappingV3,
};

pub(super) struct DerivedActionV3 {
    pub attempt: MappingActionAttemptV3,
    pub action: Option<nando_operator_kernel::BoundProtocolActionV3>,
}

pub(super) fn mapping_digest_v3(
    mode: &CompiledProtocolModeV3,
    mapping: &RuntimeStructuralMappingV3,
) -> Result<String, ()> {
    canonical_json_sha256(&(
        "nando.f5d.runtime-structural-mapping.v3",
        mode.mode_id_sha256(),
        mapping.runtime_source_role_id(),
        mapping.local_to_canonical(),
        mapping.phase_fit_fixed(),
    ))
    .map_err(|_| ())
}

pub(super) fn capability_matches_mode_v3(
    mode: &CompiledProtocolModeV3,
    descriptor: &RuntimeCapabilityDescriptorV3,
) -> bool {
    descriptor.kind == mode.runtime_capability_kind()
        && descriptor.argument_types == mode.capability_argument_types()
        && usize::from(descriptor.required_arity) == mode.capability_arguments().len()
}

pub(super) fn derive_action_v3(
    index_sha256: &str,
    request: &CanonicalRuntimeRequestV3<'_>,
    mode: &CompiledProtocolModeV3,
    mapping: &RuntimeStructuralMappingV3,
    mapping_sha256: &str,
    capability: &RuntimeCapabilityBindingV3<'_>,
) -> DerivedActionV3 {
    if capability.argument_topology_ambiguous {
        return blocked_attempt(
            mode,
            mapping,
            mapping_sha256,
            Some(capability.capability_id),
            ActionDerivationVerdictV3::AmbiguousCapabilityTopology,
        );
    }
    if capability.arguments.len() != mode.capability_arguments().len() {
        return blocked_attempt(
            mode,
            mapping,
            mapping_sha256,
            Some(capability.capability_id),
            ActionDerivationVerdictV3::InvalidAction,
        );
    }

    let mut arguments = Vec::with_capacity(mode.capability_arguments().len());
    for expected in mode.capability_arguments() {
        let Some(physical) = capability
            .arguments
            .get(usize::from(expected.argument_ordinal()))
        else {
            return blocked_attempt(
                mode,
                mapping,
                mapping_sha256,
                Some(capability.capability_id),
                ActionDerivationVerdictV3::InvalidAction,
            );
        };
        if physical.argument_ordinal != expected.argument_ordinal()
            || physical.value_type != expected.value_type()
            || !physical.required
        {
            return blocked_attempt(
                mode,
                mapping,
                mapping_sha256,
                Some(capability.capability_id),
                ActionDerivationVerdictV3::InvalidAction,
            );
        }
        let Some(runtime_role_id) = runtime_role_for_source_v3(mapping, expected.source_role_id())
        else {
            return blocked_attempt(
                mode,
                mapping,
                mapping_sha256,
                Some(capability.capability_id),
                ActionDerivationVerdictV3::UnsupportedSourceRole,
            );
        };
        let Some(binding) = request.role_value(runtime_role_id) else {
            return blocked_attempt(
                mode,
                mapping,
                mapping_sha256,
                Some(capability.capability_id),
                ActionDerivationVerdictV3::MissingRoleValue,
            );
        };
        if binding.values().len() != 1 {
            return blocked_attempt(
                mode,
                mapping,
                mapping_sha256,
                Some(capability.capability_id),
                if binding.values().is_empty() {
                    ActionDerivationVerdictV3::MissingRoleValue
                } else {
                    ActionDerivationVerdictV3::AmbiguousRoleValue
                },
            );
        }
        let value = binding.values()[0].clone();
        if mode.source_role_type(expected.source_role_id()) != Some(value.value_type())
            || value.value_type() != expected.value_type()
        {
            return blocked_attempt(
                mode,
                mapping,
                mapping_sha256,
                Some(capability.capability_id),
                ActionDerivationVerdictV3::InvalidAction,
            );
        }
        arguments.push(BoundProtocolArgumentInputV3 {
            argument_ordinal: expected.argument_ordinal(),
            source_role_id: expected.source_role_id(),
            physical_name: physical.physical_name.to_owned(),
            value,
        });
    }

    let action = build_bound_protocol_action_v3(BoundProtocolActionInputV3 {
        index_sha256: index_sha256.to_owned(),
        artifact_root_sha256: mode.artifact_root_sha256().to_owned(),
        mode_id_sha256: mode.mode_id_sha256().to_owned(),
        executable_mode_root_sha256: mode.executable_mode_root_sha256().to_owned(),
        payload_root_sha256: mode.payload_root_sha256().to_owned(),
        effect_law_id_sha256: mode.effect_law_id_sha256().to_owned(),
        action_class_root_sha256: mode.action_class_root_sha256().to_owned(),
        request_view_sha256: request.view().request_view_sha256.clone(),
        mapping_sha256: mapping_sha256.to_owned(),
        capability_id: capability.capability_id,
        capability_kind: capability.kind,
        physical_symbol: capability.physical_symbol.to_owned(),
        arguments,
    });
    let Ok(action) = action else {
        return blocked_attempt(
            mode,
            mapping,
            mapping_sha256,
            Some(capability.capability_id),
            ActionDerivationVerdictV3::InvalidAction,
        );
    };
    let attempt = MappingActionAttemptV3 {
        mode_id_sha256: mode.mode_id_sha256().to_owned(),
        mapping_sha256: mapping_sha256.to_owned(),
        runtime_source_role_id: mapping.runtime_source_role_id(),
        phase_fit_fixed: mapping.phase_fit_fixed(),
        capability_id: Some(capability.capability_id),
        verdict: ActionDerivationVerdictV3::Bound,
        semantic_action_sha256: Some(action.semantic_action_sha256().to_owned()),
        physical_action_sha256: Some(action.physical_action_sha256().to_owned()),
    };
    DerivedActionV3 {
        attempt,
        action: Some(action),
    }
}

pub(super) fn missing_capability_attempt_v3(
    mode: &CompiledProtocolModeV3,
    mapping: &RuntimeStructuralMappingV3,
    mapping_sha256: &str,
) -> MappingActionAttemptV3 {
    blocked_attempt(
        mode,
        mapping,
        mapping_sha256,
        None,
        ActionDerivationVerdictV3::MissingCapability,
    )
    .attempt
}

fn blocked_attempt(
    mode: &CompiledProtocolModeV3,
    mapping: &RuntimeStructuralMappingV3,
    mapping_sha256: &str,
    capability_id: Option<u16>,
    verdict: ActionDerivationVerdictV3,
) -> DerivedActionV3 {
    DerivedActionV3 {
        attempt: MappingActionAttemptV3 {
            mode_id_sha256: mode.mode_id_sha256().to_owned(),
            mapping_sha256: mapping_sha256.to_owned(),
            runtime_source_role_id: mapping.runtime_source_role_id(),
            phase_fit_fixed: mapping.phase_fit_fixed(),
            capability_id,
            verdict,
            semantic_action_sha256: None,
            physical_action_sha256: None,
        },
        action: None,
    }
}

const fn runtime_role_for_source_v3(
    mapping: &RuntimeStructuralMappingV3,
    source_role_id: u16,
) -> Option<u16> {
    // F5-C currently proves the scalar role at source role 0. The action IR is
    // multi-role; unsupported higher roles fail closed until the graph compiler
    // emits their explicit runtime assignments.
    if source_role_id == 0 {
        Some(mapping.runtime_source_role_id())
    } else {
        None
    }
}
