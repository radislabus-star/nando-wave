use std::collections::BTreeSet;

use super::source_events::{
    LIVE_STORE_MAX_BUCKET_REFINEMENT_DEPTH, live_store_bucket_selector,
    live_store_false_accept_split_atom_refinement_blocker, live_store_forbidden_atom,
    live_store_hash_id,
};

pub(super) fn live_store_auto_subcenter_atoms_from_safe_atoms(
    safe_atoms: &[String],
) -> Vec<String> {
    let bucket_atoms = safe_atoms
        .iter()
        .filter(|atom| live_store_bucket_selector(atom, LIVE_STORE_MAX_BUCKET_REFINEMENT_DEPTH))
        .filter(|atom| live_store_auto_subcenter_atom_blocker(atom) == "none")
        .cloned()
        .collect::<Vec<_>>();
    let pair_partner_atoms = safe_atoms
        .iter()
        .filter(|atom| live_store_bucket_selector(atom, LIVE_STORE_MAX_BUCKET_REFINEMENT_DEPTH))
        .filter(|atom| live_store_auto_subcenter_pair_partner(atom))
        .cloned()
        .collect::<Vec<_>>();
    let mut atoms = Vec::new();
    let mut seen = BTreeSet::<String>::new();
    for hidden_atom in live_store_hidden_state_atoms_from_safe_atoms(safe_atoms) {
        live_store_push_subcenter_atom(&mut atoms, &mut seen, hidden_atom);
    }
    for state_atom in bucket_atoms
        .iter()
        .filter(|atom| atom.starts_with("state_exit_code_band:"))
    {
        let state_family = live_store_auto_subcenter_pair_family_atom(state_atom);
        for partner in &pair_partner_atoms {
            live_store_push_subcenter_atom(
                &mut atoms,
                &mut seen,
                format!("pair:{state_family}|{partner}"),
            );
        }
        let command_partners = pair_partner_atoms
            .iter()
            .filter(|atom| live_store_auto_subcenter_command_partner(atom))
            .collect::<Vec<_>>();
        let shape_partners = pair_partner_atoms
            .iter()
            .filter(|atom| live_store_auto_subcenter_shape_partner(atom))
            .collect::<Vec<_>>();
        for command in &command_partners {
            for shape in &shape_partners {
                live_store_push_subcenter_atom(
                    &mut atoms,
                    &mut seen,
                    format!("combo:{state_family}|{command}|{shape}"),
                );
            }
        }
    }
    for atom in bucket_atoms {
        live_store_push_subcenter_atom(&mut atoms, &mut seen, atom);
    }
    atoms.truncate(super::DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_AUTO_SUBCENTER_ATOMS);
    atoms
}

#[cfg(test)]
pub(super) fn live_store_quarantine_recovery_subcenter_atoms(
    route_key: &str,
    parent_atoms: &[String],
    split_atoms: &[String],
    quarantined_profile_ids: &BTreeSet<u32>,
    limit: usize,
) -> Vec<String> {
    live_store_quarantine_recovery_subcenter_atoms_for_parent_ids(
        route_key,
        parent_atoms,
        &[],
        split_atoms,
        quarantined_profile_ids,
        limit,
    )
}

pub(super) fn live_store_quarantine_recovery_subcenter_atoms_for_parent_ids(
    route_key: &str,
    parent_atoms: &[String],
    explicit_parent_bucket_ids: &[u32],
    split_atoms: &[String],
    recovery_profile_ids: &BTreeSet<u32>,
    limit: usize,
) -> Vec<String> {
    if limit == 0 || recovery_profile_ids.is_empty() {
        return Vec::new();
    }
    let mut atoms = Vec::new();
    let mut seen = BTreeSet::<String>::new();
    for parent_atom in parent_atoms {
        let parent_bucket_key = live_store_auto_subcenter_bucket_key(route_key, parent_atom);
        let parent_bucket_id =
            live_store_hash_id(["live_store_bucket", parent_bucket_key.as_str()]);
        if !recovery_profile_ids.contains(&parent_bucket_id) {
            continue;
        }
        let recovery_prefix = if parent_atom.starts_with("hidden_state:") {
            "hidden_state:quarantine_recovery"
        } else {
            "quarantine_recovery"
        };
        live_store_push_recovery_split_atoms(
            &mut atoms,
            &mut seen,
            parent_bucket_id,
            recovery_prefix,
            split_atoms,
            Some(parent_atom),
            limit,
        );
        if atoms.len() >= limit {
            return atoms;
        }
    }
    for parent_bucket_id in explicit_parent_bucket_ids {
        if !recovery_profile_ids.contains(parent_bucket_id) {
            continue;
        }
        live_store_push_recovery_split_atoms(
            &mut atoms,
            &mut seen,
            *parent_bucket_id,
            "quarantine_recovery",
            split_atoms,
            None,
            limit,
        );
        if atoms.len() >= limit {
            return atoms;
        }
    }
    atoms
}

fn live_store_push_recovery_split_atoms(
    atoms: &mut Vec<String>,
    seen: &mut BTreeSet<String>,
    parent_bucket_id: u32,
    recovery_prefix: &str,
    split_atoms: &[String],
    excluded_parent_atom: Option<&String>,
    limit: usize,
) {
    let safe_split_atoms = split_atoms
        .iter()
        .filter(|atom| Some(*atom) != excluded_parent_atom)
        .filter(|atom| live_store_auto_subcenter_atom_blocker(atom) == "none")
        .collect::<BTreeSet<_>>();
    let mut safe_split_atoms = safe_split_atoms.into_iter().collect::<Vec<_>>();
    safe_split_atoms.sort_by(|left, right| {
        live_store_quarantine_recovery_split_atom_score(right)
            .cmp(&live_store_quarantine_recovery_split_atom_score(left))
            .then_with(|| left.cmp(right))
    });
    let single_limit = limit.saturating_add(2) / 3;
    for split_atom in safe_split_atoms.iter().take(single_limit) {
        let atom = format!("{recovery_prefix}:parent={parent_bucket_id}:split={split_atom}");
        live_store_push_recovery_atom(atoms, seen, atom, limit);
        if atoms.len() >= limit {
            return;
        }
    }
    for left_index in 0..safe_split_atoms.len() {
        for right_index in left_index + 1..safe_split_atoms.len() {
            let left_atom = safe_split_atoms[left_index];
            let right_atom = safe_split_atoms[right_index];
            let atom = format!(
                "{recovery_prefix}:parent={parent_bucket_id}:split_pair={left_atom}+{right_atom}"
            );
            live_store_push_recovery_atom(atoms, seen, atom, limit);
            if atoms.len() >= limit {
                return;
            }
        }
    }
    let composite_split_atoms = safe_split_atoms
        .iter()
        .take(limit.max(3).min(12))
        .copied()
        .collect::<Vec<_>>();
    for left_index in 0..composite_split_atoms.len() {
        for middle_index in left_index + 1..composite_split_atoms.len() {
            for right_index in middle_index + 1..composite_split_atoms.len() {
                let left_atom = composite_split_atoms[left_index];
                let middle_atom = composite_split_atoms[middle_index];
                let right_atom = composite_split_atoms[right_index];
                let atom = format!(
                    "{recovery_prefix}:parent={parent_bucket_id}:split_triple={left_atom}+{middle_atom}+{right_atom}"
                );
                live_store_push_recovery_atom(atoms, seen, atom, limit);
                if atoms.len() >= limit {
                    return;
                }
            }
        }
    }
}

fn live_store_push_recovery_atom(
    atoms: &mut Vec<String>,
    seen: &mut BTreeSet<String>,
    atom: String,
    limit: usize,
) {
    if atoms.len() < limit && seen.insert(atom.clone()) {
        atoms.push(atom);
    }
}

fn live_store_quarantine_recovery_split_atom_score(atom: &str) -> u16 {
    if atom.starts_with("hidden_state:") {
        600
    } else if atom.starts_with("combo:") {
        500
    } else if atom.starts_with("pair:") {
        400
    } else if atom.starts_with("tool_check_kind:") {
        350
    } else if atom.starts_with("request_command_kind:")
        || atom.starts_with("tool_command_kind:")
        || atom.starts_with("state_exit_code_band:")
    {
        300
    } else if atom.starts_with("shadow_active_fringe_len_band:")
        || atom.starts_with("shadow_slot_count_band:")
    {
        200
    } else {
        100
    }
}

fn live_store_push_subcenter_atom(
    atoms: &mut Vec<String>,
    seen: &mut BTreeSet<String>,
    atom: String,
) {
    if seen.insert(atom.clone()) {
        atoms.push(atom);
    }
}

pub(super) fn live_store_auto_subcenter_atom_blocker(atom: &str) -> &'static str {
    let base_blocker = live_store_false_accept_split_atom_refinement_blocker(atom);
    if base_blocker != "none" {
        return base_blocker;
    }
    if atom.starts_with("action_family:") || atom.starts_with("domain_family:") {
        return "family_atom_too_broad_for_single_subcenter";
    }
    if atom.starts_with("tool_command_shell_family:") {
        return "shell_family_too_broad_for_single_subcenter";
    }
    "none"
}

fn live_store_auto_subcenter_pair_family_atom(atom: &str) -> &'static str {
    if atom.starts_with("state_exit_code_band:") {
        "state_exit_code_band:*"
    } else {
        "unknown_family:*"
    }
}

fn live_store_auto_subcenter_pair_partner(atom: &str) -> bool {
    live_store_auto_subcenter_command_partner(atom) || live_store_auto_subcenter_shape_partner(atom)
}

fn live_store_auto_subcenter_command_partner(atom: &str) -> bool {
    (atom.starts_with("request_command_kind:") && atom != "request_command_kind:other")
        || (atom.starts_with("tool_command_kind:") && atom != "tool_command_kind:other")
        || atom.starts_with("tool_check_kind:")
}

fn live_store_auto_subcenter_shape_partner(atom: &str) -> bool {
    atom.starts_with("shadow_active_fringe_len_band:")
        || atom.starts_with("shadow_slot_count_band:")
}

pub(super) fn live_store_auto_subcenter_bucket_key(route_key: &str, atom: &str) -> String {
    if atom.starts_with("state_exit_code_band:") {
        return format!("{route_key}::auto_subcenter_family:state_exit_code_band");
    }
    if atom.starts_with("hidden_state:") {
        return format!("{route_key}::hidden_state:{atom}");
    }
    if atom.starts_with("pair:") {
        return format!("{route_key}::auto_subcenter_pair:{atom}");
    }
    if atom.starts_with("combo:") {
        return format!("{route_key}::auto_subcenter_combo:{atom}");
    }
    format!("{route_key}::auto_subcenter:{atom}")
}

pub(super) fn live_store_auto_subcenter_bucket_ids(
    route_key: &str,
    primary_bucket_id: u32,
    atoms: &[String],
) -> Vec<u32> {
    let mut bucket_ids = atoms
        .iter()
        .map(|atom| live_store_auto_subcenter_bucket_key(route_key, atom))
        .map(|bucket_key| live_store_hash_id(["live_store_bucket", bucket_key.as_str()]))
        .filter(|&bucket_id| bucket_id != primary_bucket_id)
        .collect::<Vec<_>>();
    bucket_ids.sort_unstable();
    bucket_ids.dedup();
    bucket_ids
}

fn live_store_hidden_state_atoms_from_safe_atoms(safe_atoms: &[String]) -> Vec<String> {
    const BASIS_LIMIT: usize = 6;
    const MAX_HIDDEN_STATE_ATOMS: usize = 12;

    let request_basis = live_store_hidden_state_basis(safe_atoms, BASIS_LIMIT, |atom| {
        atom.starts_with("request_command_kind:")
            || atom.starts_with("request_route_family:")
            || atom.starts_with("domain_family:")
    });
    let state_basis = live_store_hidden_state_basis(safe_atoms, BASIS_LIMIT, |atom| {
        atom.starts_with("state_") || atom.starts_with("shadow_")
    });
    let tool_basis = live_store_hidden_state_basis(safe_atoms, BASIS_LIMIT, |atom| {
        atom.starts_with("tool_command_kind:")
            || atom.starts_with("tool_check_kind:")
            || atom.starts_with("tool_command_shell_family:")
    });

    let mut candidates = Vec::<(u128, String)>::new();
    let mut seen = BTreeSet::<String>::new();
    live_store_push_hidden_state_cross_layer_candidates(
        &mut candidates,
        &mut seen,
        "request_state",
        &request_basis,
        &state_basis,
    );
    live_store_push_hidden_state_cross_layer_candidates(
        &mut candidates,
        &mut seen,
        "state_tool",
        &state_basis,
        &tool_basis,
    );
    live_store_push_hidden_state_cross_layer_candidates(
        &mut candidates,
        &mut seen,
        "request_tool",
        &request_basis,
        &tool_basis,
    );
    'outer: for request_atom in &request_basis {
        for state_atom in &state_basis {
            for tool_atom in &tool_basis {
                if candidates.len() >= MAX_HIDDEN_STATE_ATOMS {
                    break 'outer;
                }
                live_store_push_hidden_state_candidate(
                    &mut candidates,
                    &mut seen,
                    "request_state_tool",
                    &[request_atom, state_atom, tool_atom],
                );
            }
        }
    }
    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    candidates
        .into_iter()
        .map(|(_, atom)| atom)
        .take(MAX_HIDDEN_STATE_ATOMS)
        .collect()
}

fn live_store_hidden_state_basis(
    safe_atoms: &[String],
    limit: usize,
    predicate: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut atoms = safe_atoms
        .iter()
        .filter(|atom| predicate(atom))
        .filter(|atom| live_store_hidden_state_source_atom_allowed(atom))
        .cloned()
        .collect::<Vec<_>>();
    atoms.sort_by(|left, right| {
        live_store_hidden_state_source_atom_score(right)
            .cmp(&live_store_hidden_state_source_atom_score(left))
            .then_with(|| left.cmp(right))
    });
    atoms.dedup();
    atoms.truncate(limit);
    atoms
}

fn live_store_push_hidden_state_cross_layer_candidates(
    candidates: &mut Vec<(u128, String)>,
    seen: &mut BTreeSet<String>,
    transition_kind: &str,
    left_basis: &[String],
    right_basis: &[String],
) {
    for left in left_basis {
        for right in right_basis {
            live_store_push_hidden_state_candidate(
                candidates,
                seen,
                transition_kind,
                &[left, right],
            );
        }
    }
}

fn live_store_push_hidden_state_candidate(
    candidates: &mut Vec<(u128, String)>,
    seen: &mut BTreeSet<String>,
    transition_kind: &str,
    parts: &[&String],
) {
    let Some(atom) = live_store_hidden_state_atom(transition_kind, parts) else {
        return;
    };
    if !seen.insert(atom.clone()) {
        return;
    }
    let score = parts
        .iter()
        .map(|part| live_store_hidden_state_source_atom_score(part))
        .fold(0u128, u128::saturating_add)
        .saturating_mul(128)
        .saturating_add(
            super::super::stable_fingerprint(["live_store_hidden_state", &atom]) as u128,
        );
    candidates.push((score, atom));
}

fn live_store_hidden_state_atom(transition_kind: &str, parts: &[&String]) -> Option<String> {
    if parts.len() < 2
        || !transition_kind
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch == '_')
        || !parts
            .iter()
            .all(|part| live_store_hidden_state_source_atom_allowed(part))
    {
        return None;
    }
    let compact_parts = parts
        .iter()
        .map(|part| live_store_hidden_state_part(part))
        .collect::<Option<Vec<_>>>()?;
    let atom = format!("hidden_state:{transition_kind}:{}", compact_parts.join("+"));
    live_store_hidden_state_atom_allowed(&atom).then_some(atom)
}

fn live_store_hidden_state_part(atom: &str) -> Option<String> {
    let (family, _) = atom.split_once(':')?;
    if family.is_empty()
        || !family
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch == '_' || ch.is_ascii_digit())
    {
        return None;
    }
    let fingerprint =
        super::super::stable_fingerprint(["live_store_hidden_state_part", atom]) & 0xffff;
    Some(format!("{family}_{fingerprint:04x}"))
}

fn live_store_hidden_state_source_atom_allowed(atom: &str) -> bool {
    !live_store_forbidden_atom(atom)
        && !atom.starts_with("route_hint:")
        && !atom.starts_with("route_key:")
        && !atom.starts_with("state_source:")
        && !atom.starts_with("state_session_bucket:")
        && !atom.starts_with("state_cwd_kind:")
        && !atom.starts_with("request_route_family:")
        && !atom.starts_with("profile_id:")
        && atom.chars().all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, ':' | '_' | '-')
        })
}

fn live_store_hidden_state_atom_allowed(atom: &str) -> bool {
    atom.starts_with("hidden_state:")
        && !atom.contains("route_hint")
        && !atom.contains("route_key")
        && !atom.contains("profile_id")
        && !atom.contains("proof_rule_id")
        && !atom.contains("target_id")
        && atom.chars().all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, ':' | '_' | '+')
        })
}

fn live_store_hidden_state_source_atom_score(atom: &str) -> u128 {
    if atom.starts_with("state_exit_code_band:") {
        10_000
    } else if atom.starts_with("tool_command_kind:") || atom.starts_with("request_command_kind:") {
        8_000
    } else if atom.starts_with("tool_check_kind:") {
        7_000
    } else if atom.starts_with("shadow_active_fringe_len_band:")
        || atom.starts_with("shadow_slot_count_band:")
    {
        5_000
    } else if atom.starts_with("domain_family:") {
        2_000
    } else {
        1_000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarantine_recovery_subcenter_atoms_split_only_quarantined_parent_with_safe_atoms() {
        let route_key = "route:test_output_parse";
        let parent_atom =
            "hidden_state:request_state:request_command_kind:cargo|state_exit_code_band:nonzero"
                .to_owned();
        let parent_bucket_key = live_store_auto_subcenter_bucket_key(route_key, &parent_atom);
        let parent_bucket_id =
            live_store_hash_id(["live_store_bucket", parent_bucket_key.as_str()]);
        let mut quarantined = BTreeSet::new();
        quarantined.insert(parent_bucket_id);

        let recovery_atoms = live_store_quarantine_recovery_subcenter_atoms(
            route_key,
            &[parent_atom],
            &[
                "request_command_kind:cargo".to_owned(),
                "action_family:tool".to_owned(),
                "state_session_bucket:abc".to_owned(),
            ],
            &quarantined,
            8,
        );

        assert_eq!(recovery_atoms.len(), 1);
        assert!(recovery_atoms[0].starts_with("hidden_state:quarantine_recovery:"));
        assert!(recovery_atoms[0].contains("split=request_command_kind:cargo"));
    }

    #[test]
    fn quarantine_recovery_subcenter_atoms_can_split_explicit_primary_parent_bucket() {
        let route_key = "route:test_output_parse";
        let primary_bucket_id = live_store_hash_id([
            "live_store_bucket",
            "route:test_output_parse::request_command_kind:cargo",
        ]);
        let mut recovery_ids = BTreeSet::new();
        recovery_ids.insert(primary_bucket_id);

        let recovery_atoms = live_store_quarantine_recovery_subcenter_atoms_for_parent_ids(
            route_key,
            &[],
            &[primary_bucket_id],
            &[
                "request_command_kind:cargo".to_owned(),
                "tool_check_kind:test".to_owned(),
                "state_exit_code_band:zero".to_owned(),
                "action_family:tool".to_owned(),
            ],
            &recovery_ids,
            8,
        );

        assert!(!recovery_atoms.is_empty());
        assert!(
            recovery_atoms
                .iter()
                .all(|atom| atom.starts_with("quarantine_recovery:"))
        );
        assert!(recovery_atoms.iter().any(|atom| {
            atom.contains(&format!("parent={primary_bucket_id}"))
                && atom.contains("split=request_command_kind:cargo")
        }));
        assert!(
            recovery_atoms
                .iter()
                .all(|atom| !atom.contains("action_family:tool"))
        );
    }

    #[test]
    fn quarantine_recovery_subcenter_atoms_adds_bounded_pair_splits() {
        let route_key = "route:test_output_parse";
        let parent_atom =
            "hidden_state:request_state:request_command_kind:cargo|state_exit_code_band:nonzero"
                .to_owned();
        let parent_bucket_key = live_store_auto_subcenter_bucket_key(route_key, &parent_atom);
        let parent_bucket_id =
            live_store_hash_id(["live_store_bucket", parent_bucket_key.as_str()]);
        let mut quarantined = BTreeSet::new();
        quarantined.insert(parent_bucket_id);

        let recovery_atoms = live_store_quarantine_recovery_subcenter_atoms(
            route_key,
            &[parent_atom],
            &[
                "request_command_kind:cargo".to_owned(),
                "tool_check_kind:test".to_owned(),
                "action_family:tool".to_owned(),
            ],
            &quarantined,
            8,
        );

        assert_eq!(recovery_atoms.len(), 3);
        assert!(recovery_atoms.iter().any(|atom| {
            atom.contains("split_pair=request_command_kind:cargo+tool_check_kind:test")
                || atom.contains("split_pair=tool_check_kind:test+request_command_kind:cargo")
        }));
        assert!(
            recovery_atoms
                .iter()
                .all(|atom| !atom.contains("action_family:tool"))
        );
    }

    #[test]
    fn quarantine_recovery_subcenter_atoms_can_split_by_peer_hidden_state() {
        let route_key = "route:test_output_parse";
        let parent_atom =
            "hidden_state:request_state:request_command_kind:cargo+state_exit_code_band:nonzero"
                .to_owned();
        let peer_hidden_atom =
            "hidden_state:state_tool:state_exit_code_band:nonzero+tool_check_kind:test".to_owned();
        let parent_bucket_key = live_store_auto_subcenter_bucket_key(route_key, &parent_atom);
        let parent_bucket_id =
            live_store_hash_id(["live_store_bucket", parent_bucket_key.as_str()]);
        let mut quarantined = BTreeSet::new();
        quarantined.insert(parent_bucket_id);

        let recovery_atoms = live_store_quarantine_recovery_subcenter_atoms(
            route_key,
            &[parent_atom.clone(), peer_hidden_atom.clone()],
            &[
                peer_hidden_atom.clone(),
                "request_command_kind:cargo".to_owned(),
                "action_family:tool".to_owned(),
            ],
            &quarantined,
            8,
        );

        assert!(recovery_atoms.iter().any(|atom| {
            atom.contains("hidden_state:quarantine_recovery:")
                && atom.contains(&format!("split={peer_hidden_atom}"))
        }));
        assert!(
            recovery_atoms
                .iter()
                .all(|atom| !atom.contains("action_family:tool"))
        );
    }

    #[test]
    fn quarantine_recovery_subcenter_atoms_adds_bounded_triple_splits() {
        let route_key = "route:test_output_parse";
        let parent_atom =
            "hidden_state:request_state:request_command_kind:cargo|state_exit_code_band:nonzero"
                .to_owned();
        let parent_bucket_key = live_store_auto_subcenter_bucket_key(route_key, &parent_atom);
        let parent_bucket_id =
            live_store_hash_id(["live_store_bucket", parent_bucket_key.as_str()]);
        let mut quarantined = BTreeSet::new();
        quarantined.insert(parent_bucket_id);

        let recovery_atoms = live_store_quarantine_recovery_subcenter_atoms(
            route_key,
            &[parent_atom],
            &[
                "request_command_kind:cargo".to_owned(),
                "tool_check_kind:test".to_owned(),
                "state_exit_code_band:nonzero".to_owned(),
                "shadow_slot_count_band:many".to_owned(),
                "action_family:tool".to_owned(),
            ],
            &quarantined,
            12,
        );

        assert!(recovery_atoms.iter().any(|atom| {
            atom.contains("split_triple=")
                && atom.contains("request_command_kind:cargo")
                && atom.contains("tool_check_kind:test")
                && atom.contains("state_exit_code_band:nonzero")
        }));
        assert!(recovery_atoms.len() <= 12);
        assert!(
            recovery_atoms
                .iter()
                .all(|atom| !atom.contains("action_family:tool"))
        );
    }
}
