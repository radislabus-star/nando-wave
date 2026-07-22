use nando_operator_kernel::{
    BindingCallLineageV1, BindingCapabilityClassV1, BindingCompletionStateV1,
    BindingRequestRelationV1, BindingSourceEventClassV1, BindingValueTypeV1,
};

use super::ModeToRoleErrorV3;

pub(super) const TOPOLOGY_WORDS_V3: usize = 8;

pub(super) fn digest_words_v3(root: &str) -> Result<[u32; TOPOLOGY_WORDS_V3], ModeToRoleErrorV3> {
    if root.len() != 64 || !root.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ModeToRoleErrorV3::UnsupportedSelector);
    }
    let mut words = [0_u32; TOPOLOGY_WORDS_V3];
    for (slot, word) in words.iter_mut().enumerate() {
        let start = slot.saturating_mul(8);
        *word = u32::from_str_radix(&root[start..start + 8], 16)
            .map_err(|_| ModeToRoleErrorV3::UnsupportedSelector)?;
    }
    Ok(words)
}

pub(super) const fn value_type_tag_v3(value: BindingValueTypeV1) -> u8 {
    match value {
        BindingValueTypeV1::String => 1,
        BindingValueTypeV1::Integer => 2,
        BindingValueTypeV1::Boolean => 3,
        BindingValueTypeV1::Identifier => 4,
    }
}

pub(super) const fn source_event_tag_v3(value: BindingSourceEventClassV1) -> u8 {
    match value {
        BindingSourceEventClassV1::Textual => 1,
        BindingSourceEventClassV1::Structured => 2,
        BindingSourceEventClassV1::Mixed => 3,
        BindingSourceEventClassV1::Scalar => 4,
        BindingSourceEventClassV1::Unknown => 5,
    }
}

pub(super) const fn call_lineage_tag_v3(value: BindingCallLineageV1) -> u8 {
    match value {
        BindingCallLineageV1::SameValueAcrossEvents => 1,
        BindingCallLineageV1::SharedOpaqueAnchor => 2,
        BindingCallLineageV1::Unlinked => 3,
        BindingCallLineageV1::Unknown => 4,
    }
}

pub(super) const fn capability_class_tag_v3(value: BindingCapabilityClassV1) -> u8 {
    match value {
        BindingCapabilityClassV1::None => 1,
        BindingCapabilityClassV1::Single => 2,
        BindingCapabilityClassV1::Multiple => 3,
    }
}

pub(super) const fn completion_state_tag_v3(value: BindingCompletionStateV1) -> u8 {
    match value {
        BindingCompletionStateV1::Unresolved => 1,
        BindingCompletionStateV1::Completed => 2,
        BindingCompletionStateV1::Unknown => 3,
    }
}

pub(super) const fn request_relation_tag_v3(value: BindingRequestRelationV1) -> u8 {
    match value {
        BindingRequestRelationV1::Mentioned => 1,
        BindingRequestRelationV1::NotMentioned => 2,
        BindingRequestRelationV1::RequestAbsent => 3,
    }
}
