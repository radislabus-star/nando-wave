use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{CegisWinner, RelationFrame, TeacherPoolSnapshot};

pub const FROZEN_PARTITION_VERSION: u32 = 15;
const MAX_EXACT_PARTITION_SESSIONS: usize = 16;

#[must_use]
pub(crate) fn support_partition_complete(support_rows: usize, policy: RolloverPolicy) -> bool {
    support_rows >= policy.support_rows
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolloverPolicy {
    pub support_rows: usize,
    pub future_rows: usize,
    pub minimum_future_sessions: usize,
    pub minimum_surfaces: usize,
    pub successor_future_rows: usize,
    pub successor_future_sessions: usize,
    pub reserved_newest_sessions: usize,
}

impl Default for RolloverPolicy {
    fn default() -> Self {
        Self {
            support_rows: 32,
            future_rows: 32,
            minimum_future_sessions: 3,
            minimum_surfaces: 2,
            successor_future_rows: 64,
            successor_future_sessions: 6,
            reserved_newest_sessions: 3,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FrozenGeneration {
    #[serde(default)]
    pub partition_version: u32,
    pub generation_id_sha256: String,
    pub generation: u64,
    pub teacher_signature_sha256: String,
    pub cohort_id_sha256: String,
    pub support: Vec<RelationFrame>,
    pub future: Vec<RelationFrame>,
    pub negatives: Vec<RelationFrame>,
    pub support_watermark_unix_nanos: u64,
    pub support_sessions: usize,
    pub future_sessions: usize,
    pub surfaces: usize,
    pub wrong_future_rows: usize,
    pub blocker: Option<String>,
}

fn frozen_generation_id(
    partition_version: u32,
    cohort_id_sha256: &str,
    generation: u64,
    support: &[RelationFrame],
) -> String {
    crate::sha256_bytes(
        &serde_json::to_vec(&(
            "nando.frozen-generation.v11",
            partition_version,
            cohort_id_sha256,
            generation,
            support
                .iter()
                .map(|frame| frame.frame_id_sha256.as_str())
                .collect::<Vec<_>>(),
        ))
        .unwrap_or_default(),
    )
}

#[must_use]
pub fn freeze_generation(
    winner: &CegisWinner,
    pool: &TeacherPoolSnapshot,
    policy: RolloverPolicy,
    generation: u64,
    support_eligible_ids: &BTreeSet<String>,
    future_eligible_ids: &BTreeSet<String>,
    preferred_generation_support_ids: &BTreeSet<String>,
) -> FrozenGeneration {
    let mut matching = pool
        .positives
        .iter()
        .filter(|frame| {
            crate::synthesis::program_is_consistent(&winner.program, frame)
                && crate::cegis::winner_routes_frame(winner, frame)
        })
        .cloned()
        .collect::<Vec<_>>();
    matching.sort_by(frame_event_order);
    matching.dedup_by(|left, right| left.frame_id_sha256 == right.frame_id_sha256);
    let mut synthesis_support_ids = winner
        .support_frame_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    // Immutable replay/generation support is older and has stronger partition
    // provenance than a newly selected CEGIS support row. Prefer it so a
    // regroup does not move the watermark forward and consume live future.
    synthesis_support_ids.extend(preferred_generation_support_ids.iter().cloned());

    let (support, mut future) = initial_session_partition(
        &matching,
        policy,
        support_eligible_ids,
        future_eligible_ids,
        winner.repair_watermark_unix_nanos,
        &synthesis_support_ids,
    );
    let support_sessions = support
        .iter()
        .map(|frame| frame.session_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let support_session_count = support_sessions.len();
    let watermark = support
        .iter()
        .map(|frame| frame.observed_at_unix_nanos)
        .max()
        .unwrap_or(0);
    let future_watermark = watermark.max(winner.repair_watermark_unix_nanos);
    future.retain(|frame| frame.observed_at_unix_nanos > future_watermark);
    future.sort_by(frame_event_order);
    future.truncate(policy.future_rows);
    let future_sessions = future
        .iter()
        .map(|frame| frame.session_id_sha256.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let surfaces = support
        .iter()
        .chain(future.iter())
        .filter_map(crate::relation_frame_structural_family_id)
        .collect::<BTreeSet<_>>()
        .len();
    let wrong_future_rows = future
        .iter()
        .filter(|frame| {
            frame.verifier_label != Some(true)
                || !crate::synthesis::program_is_consistent(&winner.program, frame)
        })
        .count();
    let blocker = if support.len() < policy.support_rows {
        Some(format!("support_rows_below_{}", policy.support_rows))
    } else if future.len() < policy.future_rows {
        Some(format!("future_rows_below_{}", policy.future_rows))
    } else if future_sessions < policy.minimum_future_sessions {
        Some(format!(
            "future_sessions_below_{}",
            policy.minimum_future_sessions
        ))
    } else if surfaces < policy.minimum_surfaces {
        Some(format!("surfaces_below_{}", policy.minimum_surfaces))
    } else if wrong_future_rows != 0 {
        Some("future_wrong_accepts_nonzero".to_owned())
    } else {
        None
    };
    let generation_id_sha256 = frozen_generation_id(
        FROZEN_PARTITION_VERSION,
        &winner.cohort_id_sha256,
        generation,
        &support,
    );
    FrozenGeneration {
        partition_version: FROZEN_PARTITION_VERSION,
        generation_id_sha256,
        generation,
        teacher_signature_sha256: winner.teacher_signature_sha256.clone(),
        cohort_id_sha256: winner.cohort_id_sha256.clone(),
        support,
        future,
        negatives: pool.negatives.clone(),
        support_watermark_unix_nanos: watermark,
        support_sessions: support_session_count,
        future_sessions,
        surfaces,
        wrong_future_rows,
        blocker,
    }
}

#[must_use]
pub fn refresh_frozen_generation(
    current: &FrozenGeneration,
    winner: &CegisWinner,
    pool: &TeacherPoolSnapshot,
    policy: RolloverPolicy,
    future_eligible_ids: &BTreeSet<String>,
) -> FrozenGeneration {
    let support = current.support.clone();
    let support_ids = support
        .iter()
        .map(|frame| frame.frame_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let support_sessions = support
        .iter()
        .map(|frame| frame.session_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let support_intents = support
        .iter()
        .map(|frame| frame.client_intent_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let support_events = support
        .iter()
        .map(|frame| frame.event_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let mut future = current
        .future
        .iter()
        .filter(|frame| future_eligible_ids.contains(&frame.frame_id_sha256))
        .filter(|frame| !winner.support_frame_ids.contains(&frame.frame_id_sha256))
        .cloned()
        .map(|frame| (frame.frame_id_sha256.clone(), frame))
        .collect::<BTreeMap<_, _>>();
    let future_watermark = current
        .support_watermark_unix_nanos
        .max(winner.repair_watermark_unix_nanos);
    for frame in &pool.positives {
        if frame.observed_at_unix_nanos <= future_watermark
            || !future_eligible_ids.contains(&frame.frame_id_sha256)
            || support_ids.contains(frame.frame_id_sha256.as_str())
            || support_sessions.contains(frame.session_id_sha256.as_str())
            || support_intents.contains(frame.client_intent_id_sha256.as_str())
            || support_events.contains(frame.event_id_sha256.as_str())
            || winner.support_frame_ids.contains(&frame.frame_id_sha256)
            || !crate::synthesis::program_is_consistent(&winner.program, frame)
            || !crate::cegis::winner_routes_frame(winner, frame)
        {
            continue;
        }
        future
            .entry(frame.frame_id_sha256.clone())
            .or_insert_with(|| frame.clone());
    }
    let mut future = future.into_values().collect::<Vec<_>>();
    future.sort_by(frame_event_order);
    // A frozen generation owns one immutable proof window. Later receipts may
    // seed a successor candidate, but cannot rewrite or enlarge this 32/32
    // support/future contract after it has been satisfied.
    future.truncate(policy.future_rows);
    if !support_partition_complete(support.len(), policy) {
        future.clear();
    }
    let support_session_count = support_sessions.len();
    let future_sessions = future
        .iter()
        .map(|frame| frame.session_id_sha256.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let surfaces = support
        .iter()
        .chain(future.iter())
        .filter_map(crate::relation_frame_structural_family_id)
        .collect::<BTreeSet<_>>()
        .len();
    let wrong_future_rows = future
        .iter()
        .filter(|frame| {
            frame.verifier_label != Some(true)
                || !crate::synthesis::program_is_consistent(&winner.program, frame)
        })
        .count();
    let blocker = if support.len() < policy.support_rows {
        Some(format!("support_rows_below_{}", policy.support_rows))
    } else if future.len() < policy.future_rows {
        Some(format!("future_rows_below_{}", policy.future_rows))
    } else if future_sessions < policy.minimum_future_sessions {
        Some(format!(
            "future_sessions_below_{}",
            policy.minimum_future_sessions
        ))
    } else if surfaces < policy.minimum_surfaces {
        Some(format!("surfaces_below_{}", policy.minimum_surfaces))
    } else if wrong_future_rows != 0 {
        Some("future_wrong_accepts_nonzero".to_owned())
    } else {
        None
    };
    let generation_id_sha256 = frozen_generation_id(
        current.partition_version,
        &winner.cohort_id_sha256,
        current.generation,
        &support,
    );
    FrozenGeneration {
        partition_version: current.partition_version,
        generation_id_sha256,
        generation: current.generation,
        teacher_signature_sha256: current.teacher_signature_sha256.clone(),
        cohort_id_sha256: current.cohort_id_sha256.clone(),
        support,
        future,
        negatives: pool.negatives.clone(),
        support_watermark_unix_nanos: current.support_watermark_unix_nanos,
        support_sessions: support_session_count,
        future_sessions,
        surfaces,
        wrong_future_rows,
        blocker,
    }
}

#[must_use]
pub fn successor_generation(
    current: &FrozenGeneration,
    policy: RolloverPolicy,
) -> Option<FrozenGeneration> {
    let mut sessions = BTreeMap::<String, u64>::new();
    for frame in &current.future {
        sessions
            .entry(frame.session_id_sha256.clone())
            .and_modify(|latest| *latest = (*latest).max(frame.observed_at_unix_nanos))
            .or_insert(frame.observed_at_unix_nanos);
    }
    if current.future.len() < policy.successor_future_rows
        || sessions.len() < policy.successor_future_sessions
        || current.wrong_future_rows != 0
    {
        return None;
    }
    let mut ordered_sessions = sessions.into_iter().collect::<Vec<_>>();
    ordered_sessions.sort_by_key(|(_, event_time)| *event_time);
    let reserve_from = ordered_sessions
        .len()
        .saturating_sub(policy.reserved_newest_sessions);
    let reserved = ordered_sessions[reserve_from..]
        .iter()
        .map(|(session, _)| session.as_str())
        .collect::<BTreeSet<_>>();
    let support_candidates = current
        .future
        .iter()
        .filter(|frame| !reserved.contains(frame.session_id_sha256.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let support = select_diverse_support_rows(&support_candidates, policy.support_rows);
    if support.len() < policy.support_rows {
        return None;
    }
    let support_sessions = support
        .iter()
        .map(|frame| frame.session_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let support_session_count = support_sessions.len();
    let support_intents = support
        .iter()
        .map(|frame| frame.client_intent_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let watermark = support
        .iter()
        .map(|frame| frame.observed_at_unix_nanos)
        .max()
        .unwrap_or(0);
    let future = current
        .future
        .iter()
        .filter(|frame| {
            reserved.contains(frame.session_id_sha256.as_str())
                && !support_intents.contains(frame.client_intent_id_sha256.as_str())
                && frame.observed_at_unix_nanos > watermark
        })
        .cloned()
        .collect::<Vec<_>>();
    let future_sessions = future
        .iter()
        .map(|frame| frame.session_id_sha256.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let surfaces = support
        .iter()
        .chain(future.iter())
        .filter_map(crate::relation_frame_structural_family_id)
        .collect::<BTreeSet<_>>()
        .len();
    let blocker = if future.len() < policy.future_rows {
        Some(format!("future_rows_below_{}", policy.future_rows))
    } else if future_sessions < policy.minimum_future_sessions {
        Some(format!(
            "future_sessions_below_{}",
            policy.minimum_future_sessions
        ))
    } else if surfaces < policy.minimum_surfaces {
        Some(format!("surfaces_below_{}", policy.minimum_surfaces))
    } else {
        None
    };
    let generation = current.generation.saturating_add(1);
    let generation_id_sha256 = frozen_generation_id(
        current.partition_version,
        &current.cohort_id_sha256,
        generation,
        &support,
    );
    Some(FrozenGeneration {
        partition_version: current.partition_version,
        generation_id_sha256,
        generation,
        teacher_signature_sha256: current.teacher_signature_sha256.clone(),
        cohort_id_sha256: current.cohort_id_sha256.clone(),
        support,
        future,
        negatives: current.negatives.clone(),
        support_watermark_unix_nanos: watermark,
        support_sessions: support_session_count,
        future_sessions,
        surfaces,
        wrong_future_rows: 0,
        blocker,
    })
}

#[must_use]
pub fn generation_monotonically_improves(
    previous: &FrozenGeneration,
    next: &FrozenGeneration,
) -> bool {
    next.generation == previous.generation.saturating_add(1)
        && next.teacher_signature_sha256 == previous.teacher_signature_sha256
        && next.cohort_id_sha256 == previous.cohort_id_sha256
        && next.wrong_future_rows == 0
        && next.support.len() >= previous.support.len().min(32)
        && next.future_sessions >= previous.future_sessions.min(3)
}

fn initial_session_partition(
    matching: &[RelationFrame],
    policy: RolloverPolicy,
    support_eligible_ids: &BTreeSet<String>,
    future_eligible_ids: &BTreeSet<String>,
    minimum_future_watermark: u64,
    preferred_support_ids: &BTreeSet<String>,
) -> (Vec<RelationFrame>, Vec<RelationFrame>) {
    let mut parity_eligible = matching
        .iter()
        .filter(|frame| support_eligible_ids.contains(&frame.frame_id_sha256))
        .cloned()
        .collect::<Vec<_>>();
    parity_eligible.sort_by(frame_event_order);
    let sessions = parity_eligible
        .iter()
        .map(|frame| frame.session_id_sha256.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let fallback = select_support_sessions(
        &parity_eligible,
        policy.support_rows,
        policy.minimum_future_sessions,
    );
    let support_session_candidates = if sessions.len() <= MAX_EXACT_PARTITION_SESSIONS {
        (1_u64..(1_u64 << sessions.len()))
            .map(|mask| {
                sessions
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| mask & (1_u64 << index) != 0)
                    .map(|(_, session)| session.clone())
                    .collect::<BTreeSet<_>>()
            })
            .collect::<Vec<_>>()
    } else {
        vec![fallback]
    };
    support_session_candidates
        .into_iter()
        .filter_map(|support_sessions| {
            partition_for_support_sessions(
                &parity_eligible,
                &support_sessions,
                policy,
                future_eligible_ids,
                minimum_future_watermark,
                preferred_support_ids,
            )
        })
        .max_by(|left, right| compare_partitions(left, right, policy, preferred_support_ids))
        .unwrap_or_else(|| {
            // Incomplete verified support is still generation-owned evidence.
            // Returning an empty partition here strands 1..31 valid receipts in
            // the bounded signature reservoir, where later traffic may evict
            // them before the generation can ever reach the admission threshold.
            // Historical rows remain support-only; frozen future still starts
            // strictly after the completed support watermark.
            (
                select_diverse_support_rows(&parity_eligible, policy.support_rows),
                Vec::new(),
            )
        })
}

fn partition_for_support_sessions(
    parity_eligible: &[RelationFrame],
    support_session_ids: &BTreeSet<String>,
    policy: RolloverPolicy,
    future_eligible_ids: &BTreeSet<String>,
    minimum_future_watermark: u64,
    preferred_support_ids: &BTreeSet<String>,
) -> Option<(Vec<RelationFrame>, Vec<RelationFrame>)> {
    let mut support_candidates = parity_eligible
        .iter()
        .filter(|frame| support_session_ids.contains(&frame.session_id_sha256))
        .cloned()
        .collect::<Vec<_>>();
    support_candidates.sort_by(|left, right| {
        let left_preferred = preferred_support_ids.contains(&left.frame_id_sha256);
        let right_preferred = preferred_support_ids.contains(&right.frame_id_sha256);
        right_preferred
            .cmp(&left_preferred)
            .then_with(|| frame_event_order(left, right))
    });
    let support = support_candidates
        .into_iter()
        .take(policy.support_rows)
        .collect::<Vec<_>>();
    if support.len() < policy.support_rows {
        return None;
    }
    let support_ids = support
        .iter()
        .map(|frame| frame.frame_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let support_sessions = support
        .iter()
        .map(|frame| frame.session_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let support_intents = support
        .iter()
        .map(|frame| frame.client_intent_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let support_events = support
        .iter()
        .map(|frame| frame.event_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let watermark = support
        .iter()
        .map(|frame| frame.observed_at_unix_nanos)
        .max()
        .unwrap_or(0)
        .max(minimum_future_watermark);
    let future = parity_eligible
        .iter()
        .filter(|frame| {
            frame.observed_at_unix_nanos > watermark
                && future_eligible_ids.contains(&frame.frame_id_sha256)
                && !preferred_support_ids.contains(&frame.frame_id_sha256)
                && !support_ids.contains(frame.frame_id_sha256.as_str())
                && !support_sessions.contains(frame.session_id_sha256.as_str())
                && !support_intents.contains(frame.client_intent_id_sha256.as_str())
                && !support_events.contains(frame.event_id_sha256.as_str())
        })
        .take(policy.future_rows.saturating_mul(4))
        .cloned()
        .collect::<Vec<_>>();
    Some((support, future))
}

fn compare_partitions(
    left: &(Vec<RelationFrame>, Vec<RelationFrame>),
    right: &(Vec<RelationFrame>, Vec<RelationFrame>),
    policy: RolloverPolicy,
    preferred_support_ids: &BTreeSet<String>,
) -> std::cmp::Ordering {
    let score = |partition: &(Vec<RelationFrame>, Vec<RelationFrame>)| {
        let future_sessions = partition
            .1
            .iter()
            .map(|frame| frame.session_id_sha256.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        let support_sessions = partition
            .0
            .iter()
            .map(|frame| frame.session_id_sha256.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        let support_watermark = partition
            .0
            .iter()
            .map(|frame| frame.observed_at_unix_nanos)
            .max()
            .unwrap_or(u64::MAX);
        let preferred_support_rows = partition
            .0
            .iter()
            .filter(|frame| preferred_support_ids.contains(&frame.frame_id_sha256))
            .count();
        (
            partition.1.len() >= policy.future_rows
                && future_sessions >= policy.minimum_future_sessions,
            preferred_support_rows,
            partition.1.len(),
            future_sessions,
            std::cmp::Reverse(support_sessions),
            std::cmp::Reverse(support_watermark),
        )
    };
    score(left).cmp(&score(right))
}

pub(crate) fn select_support_sessions(
    frames: &[RelationFrame],
    support_rows: usize,
    _minimum_future_sessions: usize,
) -> BTreeSet<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for frame in frames {
        *counts.entry(frame.session_id_sha256.clone()).or_default() += 1;
    }
    if counts.is_empty() {
        return BTreeSet::new();
    }

    // Use the smallest sufficient support-session set. Every unused session
    // remains eligible for the independent future partition.
    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    for limit in 1..=ranked.len() {
        let selected = ranked
            .iter()
            .take(limit)
            .map(|(session, _)| session.clone())
            .collect::<BTreeSet<_>>();
        let selected_rows = ranked
            .iter()
            .take(limit)
            .map(|(_, rows)| *rows)
            .sum::<usize>();
        if selected_rows >= support_rows {
            return selected;
        }
    }
    ranked.into_iter().map(|(session, _)| session).collect()
}

pub(crate) fn select_diverse_support_rows(
    frames: &[RelationFrame],
    support_rows: usize,
) -> Vec<RelationFrame> {
    if support_rows == 0 || frames.is_empty() {
        return Vec::new();
    }
    let mut groups = BTreeMap::<Vec<u64>, Vec<&RelationFrame>>::new();
    for frame in frames {
        groups
            .entry(crate::relation_frame_online_routing_atom_ids(frame))
            .or_default()
            .push(frame);
    }
    let mut groups = groups
        .into_iter()
        .map(|(signature, mut rows)| {
            rows.sort_by(|left, right| frame_event_order(left, right));
            let tokens = rows
                .iter()
                .map(|frame| frame.estimated_input_tokens)
                .sum::<u64>();
            (signature, rows, tokens)
        })
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        right
            .1
            .len()
            .cmp(&left.1.len())
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| frame_event_order(left.1[0], right.1[0]))
            .then_with(|| left.0.cmp(&right.0))
    });

    let mut selected = Vec::with_capacity(support_rows.min(frames.len()));
    let mut selected_ids = BTreeSet::<String>::new();
    let mut used_sessions = vec![BTreeSet::<String>::new(); groups.len()];
    let mut session_counts = BTreeMap::<String, usize>::new();
    loop {
        let mut added = false;
        for (group_index, (_, rows, _)) in groups.iter().enumerate() {
            if selected.len() >= support_rows {
                break;
            }
            let Some(frame) = rows
                .iter()
                .copied()
                .filter(|frame| {
                    !selected_ids.contains(&frame.frame_id_sha256)
                        && !used_sessions[group_index].contains(&frame.session_id_sha256)
                })
                .min_by(|left, right| {
                    session_counts
                        .get(&left.session_id_sha256)
                        .copied()
                        .unwrap_or(0)
                        .cmp(
                            &session_counts
                                .get(&right.session_id_sha256)
                                .copied()
                                .unwrap_or(0),
                        )
                        .then_with(|| frame_event_order(left, right))
                })
            else {
                continue;
            };
            selected_ids.insert(frame.frame_id_sha256.clone());
            used_sessions[group_index].insert(frame.session_id_sha256.clone());
            session_counts
                .entry(frame.session_id_sha256.clone())
                .and_modify(|count| *count = count.saturating_add(1))
                .or_insert(1);
            selected.push(frame.clone());
            added = true;
        }
        if !added || selected.len() >= support_rows {
            break;
        }
    }

    if selected.len() < support_rows {
        let mut remaining = frames
            .iter()
            .filter(|frame| !selected_ids.contains(&frame.frame_id_sha256))
            .collect::<Vec<_>>();
        remaining.sort_by(|left, right| frame_event_order(left, right));
        for frame in remaining.into_iter().take(support_rows - selected.len()) {
            selected.push(frame.clone());
        }
    }
    selected.sort_by(frame_event_order);
    selected
}

#[cfg(test)]
fn distinct_session_count(frames: &[RelationFrame]) -> usize {
    frames
        .iter()
        .map(|frame| frame.session_id_sha256.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

fn frame_event_order(left: &RelationFrame, right: &RelationFrame) -> std::cmp::Ordering {
    left.observed_at_unix_nanos
        .cmp(&right.observed_at_unix_nanos)
        .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn future_requires_complete_frozen_support() {
        let policy = RolloverPolicy::default();
        assert!(!support_partition_complete(31, policy));
        assert!(support_partition_complete(32, policy));
    }

    #[test]
    fn incomplete_verified_support_remains_generation_owned() {
        let matching = (0..13)
            .map(|index| RelationFrame {
                schema: crate::RELATION_FRAME_SCHEMA.to_owned(),
                frame_id_sha256: format!("{index:064x}"),
                event_id_sha256: format!("{:064x}", index + 1_000),
                client_intent_id_sha256: format!("{:064x}", index + 2_000),
                session_id_sha256: format!("{:064x}", index + 3_000),
                observed_at_unix_nanos: u64::try_from(index + 1).unwrap_or(u64::MAX),
                estimated_input_tokens: 1,
                extractor_version: "partial-support-test".to_owned(),
                verifier_label: Some(true),
                atoms: Vec::new(),
                evidence_ref_sha256: format!("{:064x}", index + 4_000),
            })
            .collect::<Vec<_>>();
        let eligible = matching
            .iter()
            .map(|frame| frame.frame_id_sha256.clone())
            .collect::<BTreeSet<_>>();
        let (support, future) = initial_session_partition(
            &matching,
            RolloverPolicy::default(),
            &eligible,
            &eligible,
            0,
            &BTreeSet::new(),
        );
        assert_eq!(support.len(), 13);
        assert!(future.is_empty());
    }

    #[test]
    fn generation_identity_commits_support_but_not_future_growth() {
        let support = (0..32)
            .map(|index| RelationFrame {
                schema: crate::RELATION_FRAME_SCHEMA.to_owned(),
                frame_id_sha256: format!("{index:064x}"),
                event_id_sha256: String::new(),
                client_intent_id_sha256: String::new(),
                session_id_sha256: String::new(),
                observed_at_unix_nanos: u64::try_from(index).unwrap_or_default(),
                estimated_input_tokens: 1,
                extractor_version: "identity-test".to_owned(),
                verifier_label: Some(true),
                atoms: Vec::new(),
                evidence_ref_sha256: String::new(),
            })
            .collect::<Vec<_>>();
        let before_future = frozen_generation_id(14, "cohort", 2, &support);
        let after_future_growth = frozen_generation_id(14, "cohort", 2, &support);
        assert_eq!(before_future, after_future_growth);

        let mut changed_support = support;
        changed_support[31].frame_id_sha256 = "f".repeat(64);
        assert_ne!(
            before_future,
            frozen_generation_id(14, "cohort", 2, &changed_support)
        );
    }

    #[test]
    fn initial_partition_reserves_three_independent_sessions_for_future() {
        let mut matching = Vec::new();
        for row in 0..50 {
            for session in 0..5 {
                let index = row * 5 + session;
                let frame_id_sha256 = format!("{index:064x}");
                matching.push(RelationFrame {
                    schema: crate::RELATION_FRAME_SCHEMA.to_owned(),
                    frame_id_sha256,
                    event_id_sha256: format!("{:064x}", index + 1_000),
                    client_intent_id_sha256: format!("{:064x}", index + 2_000),
                    session_id_sha256: format!("{session:064x}"),
                    observed_at_unix_nanos: u64::try_from(index + 1).unwrap_or(u64::MAX),
                    estimated_input_tokens: 1,
                    extractor_version: "rollover-test".to_owned(),
                    verifier_label: Some(true),
                    atoms: vec![crate::RelationAtom::RequestPhaseAtom {
                        atom_id: u64::try_from(row + 1).unwrap_or(u64::MAX),
                    }],
                    evidence_ref_sha256: format!("{:064x}", index + 3_000),
                });
            }
        }
        let support_sessions = select_support_sessions(&matching, 32, 3);
        let support_candidates = matching
            .iter()
            .filter(|frame| support_sessions.contains(frame.session_id_sha256.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        let selected_support = select_diverse_support_rows(&support_candidates, 32);
        assert_eq!(
            selected_support
                .iter()
                .map(crate::relation_frame_online_routing_atom_ids)
                .collect::<BTreeSet<_>>()
                .len(),
            32
        );
        let future_eligible_ids = matching
            .iter()
            .map(|frame| frame.frame_id_sha256.clone())
            .collect::<BTreeSet<_>>();

        let (support, future) = initial_session_partition(
            &matching,
            RolloverPolicy::default(),
            &future_eligible_ids,
            &future_eligible_ids,
            0,
            &BTreeSet::new(),
        );

        assert_eq!(support.len(), 32);
        assert!(future.len() >= 32);
        assert!(distinct_session_count(&future) >= 3);
        let expected_support_sessions = (0..1)
            .map(|session| format!("{session:064x}"))
            .collect::<BTreeSet<_>>();
        assert_eq!(support_sessions, expected_support_sessions);
        let expected_future_sessions = (2..5)
            .map(|session| format!("{session:064x}"))
            .collect::<BTreeSet<_>>();
        let actual_future_sessions = future
            .iter()
            .map(|frame| frame.session_id_sha256.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual_future_sessions, expected_future_sessions);
        assert!(future.iter().all(|frame| {
            !support
                .iter()
                .any(|support_frame| support_frame.frame_id_sha256 == frame.frame_id_sha256)
        }));

        let preferred_support_ids = selected_support
            .iter()
            .map(|frame| frame.frame_id_sha256.clone())
            .collect::<BTreeSet<_>>();
        let (preferred_support, preferred_future) = initial_session_partition(
            &matching,
            RolloverPolicy::default(),
            &future_eligible_ids,
            &future_eligible_ids,
            0,
            &preferred_support_ids,
        );
        assert_eq!(preferred_support.len(), 32);
        assert!(preferred_future.len() >= 32);
        assert!(
            preferred_support
                .iter()
                .all(|frame| { preferred_support_ids.contains(&frame.frame_id_sha256) })
        );
        assert!(
            preferred_future
                .iter()
                .all(|frame| { !preferred_support_ids.contains(&frame.frame_id_sha256) })
        );

        let (_, repaired_future) = initial_session_partition(
            &matching,
            RolloverPolicy::default(),
            &future_eligible_ids,
            &future_eligible_ids,
            200,
            &BTreeSet::new(),
        );
        assert!(
            repaired_future
                .iter()
                .all(|frame| frame.observed_at_unix_nanos > 200)
        );

        let parity_eligible_ids = matching
            .iter()
            .filter(|frame| frame.session_id_sha256 == format!("{:064x}", 4))
            .map(|frame| frame.frame_id_sha256.clone())
            .collect::<BTreeSet<_>>();
        let (parity_support, parity_future) = initial_session_partition(
            &matching,
            RolloverPolicy::default(),
            &parity_eligible_ids,
            &parity_eligible_ids,
            0,
            &BTreeSet::new(),
        );
        assert_eq!(parity_support.len(), RolloverPolicy::default().support_rows);
        assert!(parity_future.is_empty());
        assert!(
            parity_support
                .iter()
                .all(|frame| parity_eligible_ids.contains(&frame.frame_id_sha256))
        );
    }
}
