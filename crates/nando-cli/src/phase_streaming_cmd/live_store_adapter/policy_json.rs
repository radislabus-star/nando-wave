pub(super) fn live_store_json_u32_vec(value: &serde_json::Value, path: &[&str]) -> Vec<u32> {
    super::super::json_at(value, path)
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_u64().and_then(|number| u32::try_from(number).ok()))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn live_store_forbidden_flags_from_json(
    value: &serde_json::Value,
) -> super::super::ForbiddenFlags {
    super::super::ForbiddenFlags {
        target_id_used: super::super::json_bool(value, &["forbidden_flags", "target_id_used"])
            .unwrap_or(true),
        proof_rule_id_authority_used: super::super::json_bool(
            value,
            &["forbidden_flags", "proof_rule_id_authority_used"],
        )
        .unwrap_or(true),
        concrete_x_lookup_used: super::super::json_bool(
            value,
            &["forbidden_flags", "concrete_x_lookup_used"],
        )
        .unwrap_or(true),
        manual_local_out_t_used: super::super::json_bool(
            value,
            &["forbidden_flags", "manual_local_out_t_used"],
        )
        .unwrap_or(true),
        hidden_frame_id_or_bind_x_used: super::super::json_bool(
            value,
            &["forbidden_flags", "hidden_frame_id_or_bind_x_used"],
        )
        .unwrap_or(true),
        legacy_backend_used: super::super::json_bool(
            value,
            &["forbidden_flags", "legacy_backend_used"],
        )
        .unwrap_or(true),
    }
}
