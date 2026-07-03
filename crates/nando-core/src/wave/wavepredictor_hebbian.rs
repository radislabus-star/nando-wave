use std::collections::{BTreeMap, HashMap};

pub type WavePredictorCenterId = u32;
pub const WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_MAGIC: [u8; 8] = *b"NWRB0001";
pub const WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_HEADER_BYTES: usize = 44;
const WAVE_PREDICTOR_ROLE_BINDING_EDGE_BYTES: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavePredictorHebbianConfig {
    pub eta_pos: i16,
    pub eta_neg: i16,
    pub eta_conflict: i16,
    pub eta_anti: i16,
    pub eta_binding: i16,
    pub state_delta_binding_feature_base: Option<WavePredictorCenterId>,
    pub state_delta_binding_action_base: Option<WavePredictorCenterId>,
    pub state_delta_binding_action_count: WavePredictorCenterId,
    pub state_delta_binding_role_base: Option<WavePredictorCenterId>,
    pub state_delta_binding_role_stride: WavePredictorCenterId,
    pub state_delta_binding_role_count: u8,
    pub state_delta_binding_slot_scoped_action_page_bits: u8,
    pub state_delta_binding_slot_scoped_action_page_mask: u64,
    pub state_delta_binding_slot_scoped_action_source_bits: u8,
    pub weight_limit: i16,
}

impl Default for WavePredictorHebbianConfig {
    fn default() -> Self {
        Self {
            eta_pos: 4,
            eta_neg: 3,
            eta_conflict: 2,
            eta_anti: 6,
            eta_binding: 0,
            state_delta_binding_feature_base: None,
            state_delta_binding_action_base: None,
            state_delta_binding_action_count: 0,
            state_delta_binding_role_base: None,
            state_delta_binding_role_stride: 0,
            state_delta_binding_role_count: 0,
            state_delta_binding_slot_scoped_action_page_bits: 0,
            state_delta_binding_slot_scoped_action_page_mask: 0,
            state_delta_binding_slot_scoped_action_source_bits: 0,
            weight_limit: 512,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavePredictorActiveCenter {
    pub center_id: WavePredictorCenterId,
    pub strength: i16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WavePredictorConvergenceError {
    pub active_fringe: Vec<WavePredictorActiveCenter>,
    pub target_center: WavePredictorCenterId,
    pub nearest_wrong_center: WavePredictorCenterId,
    pub target_gap: i32,
    pub margin_required: i32,
    pub trap_accepted: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WavePredictorHebbianEdge {
    pub source_center: WavePredictorCenterId,
    pub target_center: WavePredictorCenterId,
    pub compatibility: i16,
    pub conflict: i16,
    pub anti_wave: i16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WavePredictorFlatRoleBindingEdge {
    pub action_center: WavePredictorCenterId,
    pub output_slot_id: u8,
    pub slot_id: u8,
    pub sign_key: u8,
    pub weight: i16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WavePredictorRoleBindingRuntimeEdgeSample {
    pub action_center: WavePredictorCenterId,
    pub output_slot_id: u8,
    pub slot_id: u8,
    pub sign_key: u8,
    pub weight: i16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WavePredictorFlatRoleBindingTable {
    action_base: WavePredictorCenterId,
    action_count: WavePredictorCenterId,
    role_base: WavePredictorCenterId,
    role_stride: WavePredictorCenterId,
    slot_scoped_action_page_bits: u8,
    slot_scoped_action_page_mask: u64,
    slot_scoped_action_source_bits: u8,
    edges: Vec<WavePredictorFlatRoleBindingEdge>,
    action_offsets: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavePredictorRoleBindingPackageInfo {
    pub magic: [u8; WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_MAGIC.len()],
    pub action_base: WavePredictorCenterId,
    pub action_count: WavePredictorCenterId,
    pub role_base: WavePredictorCenterId,
    pub role_stride: WavePredictorCenterId,
    pub slot_scoped_action_page_bits: u8,
    pub slot_scoped_action_page_mask: u64,
    pub slot_scoped_action_source_bits: u8,
    pub edge_count: usize,
    pub serialized_len: usize,
    pub payload_bytes: usize,
    pub fingerprint64: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WavePredictorRoleBindingPackageError {
    EmptyRuntime,
    RuntimePackageTooLarge,
    InvalidRuntimePackage,
    InvalidPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RoleBindingPackageLayout {
    action_base: WavePredictorCenterId,
    action_count: WavePredictorCenterId,
    role_base: WavePredictorCenterId,
    role_stride: WavePredictorCenterId,
    slot_scoped_action_page_bits: u8,
    slot_scoped_action_page_mask: u64,
    slot_scoped_action_source_bits: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavePredictorRoleBindingOffloadPolicy {
    pub local_margin_threshold: i32,
}

impl WavePredictorRoleBindingOffloadPolicy {
    pub fn new(local_margin_threshold: i32) -> Result<Self, WavePredictorRoleBindingPackageError> {
        if local_margin_threshold <= 0 {
            return Err(WavePredictorRoleBindingPackageError::InvalidPolicy);
        }
        Ok(Self {
            local_margin_threshold,
        })
    }

    pub fn default_conservative() -> Self {
        Self {
            local_margin_threshold: 1,
        }
    }
}

impl Default for WavePredictorRoleBindingOffloadPolicy {
    fn default() -> Self {
        Self::default_conservative()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WavePredictorRoleBindingOffloadAction {
    LocalOperator,
    FallbackToLlm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavePredictorRoleBindingDecision {
    pub action: WavePredictorRoleBindingOffloadAction,
    pub margin: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavePredictorRoleBindingOffloadSummary {
    pub calls: usize,
    pub local_operator_calls: usize,
    pub fallback_to_llm_calls: usize,
    pub false_local_accepts: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavePredictorRoleBindingEvalTask<'a> {
    pub target_lane_id: u16,
    pub target_signed_strength: i16,
    pub wrong_lane_id: u16,
    pub wrong_signed_strength: i16,
    pub active_fringe: &'a [WavePredictorActiveCenter],
    pub binding_output_slot: Option<u8>,
    pub expect_local_operator: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WavePredictorRoleBindingPreparedFringe {
    active_actions: Vec<(WavePredictorCenterId, i16)>,
    slot_actions: HashMap<u8, Vec<(WavePredictorCenterId, i16)>>,
    role_strengths: HashMap<(u8, WavePredictorCenterId), i16>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct WavePredictorPackedRoleBindingGroup {
    action_center: WavePredictorCenterId,
    output_slot_id: u8,
    sign_key: u8,
    start: usize,
    end: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct WavePredictorPackedRoleBindingEdge {
    slot_id: u8,
    weight: i16,
}

type WavePredictorRoleBindingEdgeIndexKey = (WavePredictorCenterId, u8, u8);
type WavePredictorRoleBindingEdgeIndex =
    HashMap<WavePredictorRoleBindingEdgeIndexKey, Vec<(u8, i16)>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WavePredictorRoleBindingOffloadRuntime {
    table: Option<WavePredictorFlatRoleBindingTable>,
    edge_index: Option<WavePredictorRoleBindingEdgeIndex>,
    packed_groups: Vec<WavePredictorPackedRoleBindingGroup>,
    packed_group_offsets: Vec<usize>,
    packed_edges: Vec<WavePredictorPackedRoleBindingEdge>,
    package_info: WavePredictorRoleBindingPackageInfo,
    policy: WavePredictorRoleBindingOffloadPolicy,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WavePredictorHebbianUpdateReport {
    pub touched_edges: usize,
    pub attraction_updates: usize,
    pub repulsion_updates: usize,
    pub conflict_updates: usize,
    pub anti_wave_updates: usize,
    pub target_gap_before: i32,
    pub target_gap_after: i32,
    pub margin_required: i32,
    pub margin_fixed: bool,
    pub base_mass_drift_detected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WavePredictorHebbianField {
    base_mass: Vec<i16>,
    edges: BTreeMap<(WavePredictorCenterId, WavePredictorCenterId), WavePredictorHebbianEdge>,
    state_delta_edges: HashMap<(WavePredictorCenterId, u16), i16>,
    state_delta_role_binding_edges: HashMap<(WavePredictorCenterId, u8, u8, u8), i16>,
    state_delta_binding_positive: i16,
    state_delta_binding_negative: i16,
    config: WavePredictorHebbianConfig,
}

impl WavePredictorHebbianField {
    pub fn new(center_count: usize, config: WavePredictorHebbianConfig) -> Self {
        Self {
            base_mass: vec![0; center_count],
            edges: BTreeMap::new(),
            state_delta_edges: HashMap::new(),
            state_delta_role_binding_edges: HashMap::new(),
            state_delta_binding_positive: 0,
            state_delta_binding_negative: 0,
            config,
        }
    }

    pub fn config(&self) -> WavePredictorHebbianConfig {
        self.config
    }

    pub fn set_base_mass(&mut self, center_id: WavePredictorCenterId, mass: i16) {
        if let Some(slot) = self.base_mass.get_mut(center_id as usize) {
            *slot = mass;
        }
    }

    pub fn base_mass(&self, center_id: WavePredictorCenterId) -> Option<i16> {
        self.base_mass.get(center_id as usize).copied()
    }

    pub fn edge(
        &self,
        source_center: WavePredictorCenterId,
        target_center: WavePredictorCenterId,
    ) -> Option<WavePredictorHebbianEdge> {
        self.edges.get(&(source_center, target_center)).copied()
    }

    pub fn insert_edge(&mut self, edge: WavePredictorHebbianEdge) {
        self.edges
            .insert((edge.source_center, edge.target_center), edge);
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn state_delta_edge(
        &self,
        source_center: WavePredictorCenterId,
        lane_id: u16,
    ) -> Option<i16> {
        self.state_delta_edges
            .get(&(source_center, lane_id))
            .copied()
    }

    pub fn state_delta_edge_count(&self) -> usize {
        self.state_delta_edges.len()
    }

    pub fn state_delta_role_binding_edge_count(&self) -> usize {
        self.state_delta_role_binding_edges.len()
    }

    pub fn state_delta_role_binding_nonzero_report(&self) -> (usize, i16) {
        let mut nonzero = 0usize;
        let mut max_abs = 0i16;
        for weight in self.state_delta_role_binding_edges.values() {
            if *weight == 0 {
                continue;
            }
            nonzero += 1;
            max_abs = max_abs.max(weight.abs());
        }
        (nonzero, max_abs)
    }

    pub fn compile_flat_role_binding_table(&self) -> WavePredictorFlatRoleBindingTable {
        let action_base = self.config.state_delta_binding_action_base.unwrap_or(0);
        let action_count = self.config.state_delta_binding_action_count;
        let role_base = self.config.state_delta_binding_role_base.unwrap_or(0);
        let role_stride = self.config.state_delta_binding_role_stride;
        let mut edges: Vec<_> = self
            .state_delta_role_binding_edges
            .iter()
            .filter_map(
                |((action_center, output_slot_id, slot_id, sign_key), weight)| {
                    if *weight == 0 {
                        return None;
                    }
                    Some(WavePredictorFlatRoleBindingEdge {
                        action_center: *action_center,
                        output_slot_id: *output_slot_id,
                        slot_id: *slot_id,
                        sign_key: *sign_key,
                        weight: *weight,
                    })
                },
            )
            .collect();
        edges.sort_by_key(|edge| {
            (
                edge.action_center,
                edge.output_slot_id,
                edge.slot_id,
                edge.sign_key,
            )
        });

        let mut action_offsets = vec![0usize; action_count as usize + 1];
        for edge in &edges {
            if edge.action_center < action_base {
                continue;
            }
            let action_index = (edge.action_center - action_base) as usize;
            if action_index < action_count as usize {
                action_offsets[action_index + 1] += 1;
            }
        }
        for index in 1..action_offsets.len() {
            action_offsets[index] += action_offsets[index - 1];
        }

        WavePredictorFlatRoleBindingTable {
            action_base,
            action_count,
            role_base,
            role_stride,
            slot_scoped_action_page_bits: self
                .config
                .state_delta_binding_slot_scoped_action_page_bits,
            slot_scoped_action_page_mask: self
                .config
                .state_delta_binding_slot_scoped_action_page_mask,
            slot_scoped_action_source_bits: self
                .config
                .state_delta_binding_slot_scoped_action_source_bits,
            edges,
            action_offsets,
        }
    }

    pub fn state_delta_binding_weights(&self) -> (i16, i16) {
        (
            self.state_delta_binding_positive,
            self.state_delta_binding_negative,
        )
    }

    pub fn score_state_delta_lane(
        &self,
        lane_id: u16,
        active_fringe: &[WavePredictorActiveCenter],
    ) -> i32 {
        active_fringe
            .iter()
            .map(|active| {
                let Some(weight) = self.state_delta_edge(active.center_id, lane_id) else {
                    return 0;
                };
                i32::from(active.strength) * i32::from(weight)
            })
            .sum()
    }

    pub fn score_state_delta_binding_alignment(
        &self,
        lane_id: u16,
        signed_strength: i16,
        active_fringe: &[WavePredictorActiveCenter],
        binding_output_slot: Option<u8>,
    ) -> i32 {
        let self_transfer_score = if let Some(active_strength) =
            self.state_delta_binding_active_strength(lane_id, active_fringe)
        {
            let weight = if signed_strength < 0 {
                self.state_delta_binding_negative
            } else {
                self.state_delta_binding_positive
            };
            i32::from(active_strength.abs()) * i32::from(weight)
        } else {
            0
        };

        self_transfer_score
            + self.score_state_delta_role_binding_alignment(
                lane_id,
                signed_strength,
                active_fringe,
                binding_output_slot,
            )
    }

    pub fn adjust_state_delta_edge(
        &mut self,
        source_center: WavePredictorCenterId,
        lane_id: u16,
        delta: i32,
    ) -> bool {
        if delta == 0 {
            return false;
        }

        let limit = self.config.weight_limit;
        let edge = self
            .state_delta_edges
            .entry((source_center, lane_id))
            .or_insert(0);
        let before = *edge;
        *edge = clamp_i16(i32::from(*edge) + delta, limit);
        *edge != before
    }

    pub fn adjust_state_delta_binding(&mut self, signed_strength: i16, delta: i32) -> bool {
        if delta == 0 {
            return false;
        }

        let limit = self.config.weight_limit;
        let slot = if signed_strength < 0 {
            &mut self.state_delta_binding_negative
        } else {
            &mut self.state_delta_binding_positive
        };
        let before = *slot;
        *slot = clamp_non_negative_i16(i32::from(*slot) + delta, limit);
        *slot != before
    }

    pub fn state_delta_binding_active_strength(
        &self,
        lane_id: u16,
        active_fringe: &[WavePredictorActiveCenter],
    ) -> Option<i16> {
        let base = self.config.state_delta_binding_feature_base?;
        let center_id = base.checked_add(WavePredictorCenterId::from(lane_id))?;
        active_fringe
            .iter()
            .find(|active| active.center_id == center_id && active.strength != 0)
            .map(|active| active.strength)
    }

    pub fn adjust_state_delta_role_binding(
        &mut self,
        lane_id: u16,
        signed_strength: i16,
        active_fringe: &[WavePredictorActiveCenter],
        binding_output_slot: Option<u8>,
        delta: i32,
    ) -> usize {
        if delta == 0 {
            return 0;
        }

        let actions = self.active_binding_actions(active_fringe);
        let roles = self.active_binding_roles_for_lane(lane_id, active_fringe);
        if actions.is_empty() || roles.is_empty() {
            return 0;
        }

        let sign_key = binding_sign_key(signed_strength);
        let output_slot_id = binding_output_slot.unwrap_or(0);
        let limit = self.config.weight_limit;
        let mut changed = 0usize;
        for (action_center, action_strength) in actions {
            if !self
                .config
                .slot_scoped_action_allows_output_slot(action_center, output_slot_id)
            {
                continue;
            }
            for (slot_id, role_strength) in &roles {
                if !self
                    .config
                    .slot_scoped_action_allows_role_slot(action_center, *slot_id)
                {
                    continue;
                }
                let edge = self
                    .state_delta_role_binding_edges
                    .entry((action_center, output_slot_id, *slot_id, sign_key))
                    .or_insert(0);
                let before = *edge;
                *edge = clamp_i16(
                    i32::from(*edge)
                        + delta * i32::from(action_strength.abs()) * i32::from(role_strength.abs()),
                    limit,
                );
                changed += usize::from(*edge != before);
            }
        }
        changed
    }

    fn score_state_delta_role_binding_alignment(
        &self,
        lane_id: u16,
        signed_strength: i16,
        active_fringe: &[WavePredictorActiveCenter],
        binding_output_slot: Option<u8>,
    ) -> i32 {
        let actions = self.active_binding_actions(active_fringe);
        let roles = self.active_binding_roles_for_lane(lane_id, active_fringe);
        if actions.is_empty() || roles.is_empty() {
            return 0;
        }

        let sign_key = binding_sign_key(signed_strength);
        let output_slot_id = binding_output_slot.unwrap_or(0);
        let mut score = 0i32;
        for (action_center, action_strength) in actions {
            if !self
                .config
                .slot_scoped_action_allows_output_slot(action_center, output_slot_id)
            {
                continue;
            }
            for (slot_id, role_strength) in &roles {
                if !self
                    .config
                    .slot_scoped_action_allows_role_slot(action_center, *slot_id)
                {
                    continue;
                }
                let Some(weight) = self.state_delta_role_binding_edges.get(&(
                    action_center,
                    output_slot_id,
                    *slot_id,
                    sign_key,
                )) else {
                    continue;
                };
                score += i32::from(action_strength.abs())
                    * i32::from(role_strength.abs())
                    * i32::from(*weight);
            }
        }
        score
    }

    fn active_binding_actions(
        &self,
        active_fringe: &[WavePredictorActiveCenter],
    ) -> Vec<(WavePredictorCenterId, i16)> {
        let Some(base) = self.config.state_delta_binding_action_base else {
            return Vec::new();
        };
        let end = base.saturating_add(self.config.state_delta_binding_action_count);
        active_fringe
            .iter()
            .filter(|active| {
                active.strength != 0 && active.center_id >= base && active.center_id < end
            })
            .map(|active| (active.center_id, active.strength))
            .collect()
    }

    fn active_binding_roles_for_lane(
        &self,
        lane_id: u16,
        active_fringe: &[WavePredictorActiveCenter],
    ) -> Vec<(u8, i16)> {
        let Some(base) = self.config.state_delta_binding_role_base else {
            return Vec::new();
        };
        let stride = self.config.state_delta_binding_role_stride;
        if stride == 0 {
            return Vec::new();
        }

        let lane = WavePredictorCenterId::from(lane_id);
        let projected_lane = if lane >= stride { lane % stride } else { lane };
        let mut roles = Vec::new();
        for slot_id in 0..self.config.state_delta_binding_role_count {
            let Some(slot_base) =
                base.checked_add(WavePredictorCenterId::from(slot_id).saturating_mul(stride))
            else {
                continue;
            };
            let Some(center_id) = slot_base.checked_add(projected_lane) else {
                continue;
            };
            if let Some(active) = active_fringe
                .iter()
                .find(|active| active.center_id == center_id && active.strength != 0)
            {
                roles.push((slot_id, active.strength));
            }
        }
        roles
    }

    pub fn score_center(
        &self,
        center_id: WavePredictorCenterId,
        active_fringe: &[WavePredictorActiveCenter],
    ) -> i32 {
        active_fringe
            .iter()
            .map(|active| {
                let Some(edge) = self.edge(active.center_id, center_id) else {
                    return 0;
                };
                i32::from(active.strength)
                    * (i32::from(edge.compatibility)
                        - i32::from(edge.conflict)
                        - i32::from(edge.anti_wave))
            })
            .sum()
    }

    pub fn target_gap(&self, error: &WavePredictorConvergenceError) -> i32 {
        self.score_center(error.target_center, &error.active_fringe)
            - self.score_center(error.nearest_wrong_center, &error.active_fringe)
    }

    pub fn apply_sparse_contrastive_error(
        &mut self,
        error: &WavePredictorConvergenceError,
    ) -> WavePredictorHebbianUpdateReport {
        let base_mass_before = self.base_mass.clone();
        let gap_before = error.target_gap;
        let mut report = WavePredictorHebbianUpdateReport {
            target_gap_before: gap_before,
            margin_required: error.margin_required,
            ..WavePredictorHebbianUpdateReport::default()
        };

        if gap_before < error.margin_required {
            for active in &error.active_fringe {
                let strength = active.strength.max(0);
                if strength == 0 {
                    continue;
                }

                let eta_pos = self.config.eta_pos;
                let eta_neg = self.config.eta_neg;
                let eta_conflict = self.config.eta_conflict;
                let limit = self.config.weight_limit;

                {
                    let target_edge = self.edge_mut(active.center_id, error.target_center);
                    target_edge.compatibility = clamp_i16(
                        i32::from(target_edge.compatibility)
                            + i32::from(eta_pos) * i32::from(strength),
                        limit,
                    );
                    report.touched_edges += 1;
                    report.attraction_updates += 1;
                }

                {
                    let wrong_edge = self.edge_mut(active.center_id, error.nearest_wrong_center);
                    wrong_edge.compatibility = clamp_i16(
                        i32::from(wrong_edge.compatibility)
                            - i32::from(eta_neg) * i32::from(strength),
                        limit,
                    );
                    wrong_edge.conflict = clamp_non_negative_i16(
                        i32::from(wrong_edge.conflict)
                            + i32::from(eta_conflict) * i32::from(strength),
                        limit,
                    );
                    report.touched_edges += 1;
                    report.repulsion_updates += 1;
                    report.conflict_updates += 1;
                }
            }
        }

        if error.trap_accepted {
            for active in &error.active_fringe {
                let strength = active.strength.max(0);
                if strength == 0 {
                    continue;
                }
                let eta_anti = self.config.eta_anti;
                let limit = self.config.weight_limit;
                let wrong_edge = self.edge_mut(active.center_id, error.nearest_wrong_center);
                wrong_edge.anti_wave = clamp_non_negative_i16(
                    i32::from(wrong_edge.anti_wave) + i32::from(eta_anti) * i32::from(strength),
                    limit,
                );
                report.touched_edges += 1;
                report.anti_wave_updates += 1;
            }
        }

        report.target_gap_after = self.target_gap(error);
        report.margin_fixed = report.target_gap_after >= error.margin_required;
        report.base_mass_drift_detected = self.base_mass != base_mass_before;
        report
    }

    fn edge_mut(
        &mut self,
        source_center: WavePredictorCenterId,
        target_center: WavePredictorCenterId,
    ) -> &mut WavePredictorHebbianEdge {
        self.edges
            .entry((source_center, target_center))
            .or_insert(WavePredictorHebbianEdge {
                source_center,
                target_center,
                ..WavePredictorHebbianEdge::default()
            })
    }
}

impl WavePredictorHebbianConfig {
    fn slot_scoped_action_allows_output_slot(
        &self,
        action_center: WavePredictorCenterId,
        output_slot_id: u8,
    ) -> bool {
        slot_scoped_action_allows_output_slot(
            action_center,
            output_slot_id,
            self.state_delta_binding_slot_scoped_action_page_bits,
            self.state_delta_binding_slot_scoped_action_page_mask,
            self.state_delta_binding_slot_scoped_action_source_bits,
        )
    }

    fn slot_scoped_action_allows_role_slot(
        &self,
        action_center: WavePredictorCenterId,
        role_slot_id: u8,
    ) -> bool {
        slot_scoped_action_allows_role_slot(
            action_center,
            role_slot_id,
            self.state_delta_binding_slot_scoped_action_page_bits,
            self.state_delta_binding_slot_scoped_action_page_mask,
            self.state_delta_binding_slot_scoped_action_source_bits,
        )
    }
}

fn slot_scoped_action_allows_output_slot(
    action_center: WavePredictorCenterId,
    output_slot_id: u8,
    page_bits: u8,
    page_mask: u64,
    source_bits: u8,
) -> bool {
    if page_mask == 0 {
        return true;
    }
    if page_bits == 0 || page_bits >= WavePredictorCenterId::BITS as u8 {
        return true;
    }
    if source_bits >= page_bits {
        return true;
    }

    let page = action_center >> page_bits;
    if page >= u64::BITS {
        return true;
    }
    if (page_mask & (1_u64 << page)) == 0 {
        return true;
    }

    let lane_mask = (1_u32 << page_bits) - 1;
    let lane = action_center & lane_mask;
    ((lane >> source_bits) as u8) == output_slot_id
}

fn slot_scoped_action_allows_role_slot(
    action_center: WavePredictorCenterId,
    role_slot_id: u8,
    page_bits: u8,
    page_mask: u64,
    source_bits: u8,
) -> bool {
    if page_mask == 0 {
        return true;
    }
    if page_bits == 0 || page_bits >= WavePredictorCenterId::BITS as u8 {
        return true;
    }
    if source_bits == 0 || source_bits >= page_bits {
        return true;
    }

    let page = action_center >> page_bits;
    if page >= u64::BITS {
        return true;
    }
    if (page_mask & (1_u64 << page)) == 0 {
        return true;
    }

    let lane_mask = (1_u32 << page_bits) - 1;
    let source_mask = (1_u32 << source_bits) - 1;
    let lane = action_center & lane_mask;
    ((lane & source_mask) as u8) == role_slot_id
}

fn clamp_i16(value: i32, limit: i16) -> i16 {
    value.clamp(-i32::from(limit), i32::from(limit)) as i16
}

fn clamp_non_negative_i16(value: i32, limit: i16) -> i16 {
    value.clamp(0, i32::from(limit)) as i16
}

fn binding_sign_key(signed_strength: i16) -> u8 {
    u8::from(signed_strength < 0)
}

impl WavePredictorFlatRoleBindingTable {
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn edges(&self) -> &[WavePredictorFlatRoleBindingEdge] {
        &self.edges
    }

    pub fn byte_size_estimate(&self) -> usize {
        self.edges.len() * std::mem::size_of::<WavePredictorFlatRoleBindingEdge>()
            + self.action_offsets.len() * std::mem::size_of::<usize>()
    }

    pub fn serialized_len(&self) -> usize {
        role_binding_package_len(self.edges.len()).unwrap_or(usize::MAX)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, WavePredictorRoleBindingPackageError> {
        let edge_count = u32::try_from(self.edges.len())
            .map_err(|_| WavePredictorRoleBindingPackageError::RuntimePackageTooLarge)?;
        let serialized_len = role_binding_package_len(self.edges.len())
            .ok_or(WavePredictorRoleBindingPackageError::RuntimePackageTooLarge)?;
        let mut bytes = Vec::with_capacity(serialized_len);
        bytes.extend_from_slice(&WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_MAGIC);
        bytes.extend_from_slice(&self.action_base.to_le_bytes());
        bytes.extend_from_slice(&self.action_count.to_le_bytes());
        bytes.extend_from_slice(&self.role_base.to_le_bytes());
        bytes.extend_from_slice(&self.role_stride.to_le_bytes());
        bytes.extend_from_slice(&u32::from(self.slot_scoped_action_page_bits).to_le_bytes());
        bytes.extend_from_slice(&self.slot_scoped_action_page_mask.to_le_bytes());
        bytes.extend_from_slice(&u32::from(self.slot_scoped_action_source_bits).to_le_bytes());
        bytes.extend_from_slice(&edge_count.to_le_bytes());
        for edge in &self.edges {
            bytes.extend_from_slice(&edge.action_center.to_le_bytes());
            bytes.push(edge.output_slot_id);
            bytes.push(edge.slot_id);
            bytes.push(edge.sign_key);
            bytes.push(0);
            bytes.extend_from_slice(&edge.weight.to_le_bytes());
            bytes.extend_from_slice(&0_u16.to_le_bytes());
        }
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WavePredictorRoleBindingPackageError> {
        let info = Self::inspect_bytes(bytes)?;
        let mut edges = Vec::with_capacity(info.edge_count);
        let mut offset = WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_HEADER_BYTES;
        for _ in 0..info.edge_count {
            let action_center = read_role_binding_u32_le(bytes, offset)?;
            let output_slot_id = *bytes
                .get(offset + 4)
                .ok_or(WavePredictorRoleBindingPackageError::InvalidRuntimePackage)?;
            let slot_id = *bytes
                .get(offset + 5)
                .ok_or(WavePredictorRoleBindingPackageError::InvalidRuntimePackage)?;
            let sign_key = *bytes
                .get(offset + 6)
                .ok_or(WavePredictorRoleBindingPackageError::InvalidRuntimePackage)?;
            let weight = read_role_binding_i16_le(bytes, offset + 8)?;
            edges.push(WavePredictorFlatRoleBindingEdge {
                action_center,
                output_slot_id,
                slot_id,
                sign_key,
                weight,
            });
            offset += WAVE_PREDICTOR_ROLE_BINDING_EDGE_BYTES;
        }
        Ok(role_binding_table_from_parts(
            RoleBindingPackageLayout {
                action_base: info.action_base,
                action_count: info.action_count,
                role_base: info.role_base,
                role_stride: info.role_stride,
                slot_scoped_action_page_bits: info.slot_scoped_action_page_bits,
                slot_scoped_action_page_mask: info.slot_scoped_action_page_mask,
                slot_scoped_action_source_bits: info.slot_scoped_action_source_bits,
            },
            edges,
        ))
    }

    pub fn inspect_bytes(
        bytes: &[u8],
    ) -> Result<WavePredictorRoleBindingPackageInfo, WavePredictorRoleBindingPackageError> {
        if bytes.len() < WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_HEADER_BYTES {
            return Err(WavePredictorRoleBindingPackageError::InvalidRuntimePackage);
        }
        if bytes[..WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_MAGIC.len()]
            != WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_MAGIC
        {
            return Err(WavePredictorRoleBindingPackageError::InvalidRuntimePackage);
        }
        let action_base = read_role_binding_u32_le(bytes, 8)?;
        let action_count = read_role_binding_u32_le(bytes, 12)?;
        let role_base = read_role_binding_u32_le(bytes, 16)?;
        let role_stride = read_role_binding_u32_le(bytes, 20)?;
        let slot_scoped_action_page_bits = u8::try_from(read_role_binding_u32_le(bytes, 24)?)
            .map_err(|_| WavePredictorRoleBindingPackageError::InvalidRuntimePackage)?;
        let slot_scoped_action_page_mask = read_role_binding_u64_le(bytes, 28)?;
        let slot_scoped_action_source_bits = u8::try_from(read_role_binding_u32_le(bytes, 36)?)
            .map_err(|_| WavePredictorRoleBindingPackageError::InvalidRuntimePackage)?;
        let edge_count = read_role_binding_u32_le(bytes, 40)? as usize;
        if action_count == 0 || role_stride == 0 || edge_count == 0 {
            return Err(WavePredictorRoleBindingPackageError::EmptyRuntime);
        }
        let serialized_len = role_binding_package_len(edge_count)
            .ok_or(WavePredictorRoleBindingPackageError::RuntimePackageTooLarge)?;
        if bytes.len() != serialized_len {
            return Err(WavePredictorRoleBindingPackageError::InvalidRuntimePackage);
        }
        Ok(WavePredictorRoleBindingPackageInfo {
            magic: WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_MAGIC,
            action_base,
            action_count,
            role_base,
            role_stride,
            slot_scoped_action_page_bits,
            slot_scoped_action_page_mask,
            slot_scoped_action_source_bits,
            edge_count,
            serialized_len,
            payload_bytes: serialized_len - WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_HEADER_BYTES,
            fingerprint64: role_binding_package_fingerprint64(bytes),
        })
    }

    pub fn score_alignment(
        &self,
        lane_id: u16,
        signed_strength: i16,
        active_fringe: &[WavePredictorActiveCenter],
        binding_output_slot: Option<u8>,
    ) -> i32 {
        if self.action_count == 0 || self.role_stride == 0 {
            return 0;
        }

        let sign_key = binding_sign_key(signed_strength);
        let output_slot_id = binding_output_slot.unwrap_or(0);
        let lane = WavePredictorCenterId::from(lane_id);
        let projected_lane = if lane >= self.role_stride {
            lane % self.role_stride
        } else {
            lane
        };
        let mut score = 0i32;
        for active in active_fringe {
            if active.center_id < self.action_base {
                continue;
            }
            if !slot_scoped_action_allows_output_slot(
                active.center_id,
                output_slot_id,
                self.slot_scoped_action_page_bits,
                self.slot_scoped_action_page_mask,
                self.slot_scoped_action_source_bits,
            ) {
                continue;
            }
            let action_index = (active.center_id - self.action_base) as usize;
            if action_index >= self.action_count as usize {
                continue;
            }
            let start = self.action_offsets[action_index];
            let end = self.action_offsets[action_index + 1];
            for edge in &self.edges[start..end] {
                if edge.output_slot_id != output_slot_id || edge.sign_key != sign_key {
                    continue;
                }
                if !slot_scoped_action_allows_role_slot(
                    active.center_id,
                    edge.slot_id,
                    self.slot_scoped_action_page_bits,
                    self.slot_scoped_action_page_mask,
                    self.slot_scoped_action_source_bits,
                ) {
                    continue;
                }
                let Some(role_center) = self
                    .role_base
                    .checked_add(
                        WavePredictorCenterId::from(edge.slot_id).saturating_mul(self.role_stride),
                    )
                    .and_then(|slot_base| slot_base.checked_add(projected_lane))
                else {
                    continue;
                };
                let Some(role_active) = active_fringe.iter().find(|candidate| {
                    candidate.center_id == role_center && candidate.strength != 0
                }) else {
                    continue;
                };
                score += i32::from(active.strength.abs())
                    * i32::from(role_active.strength.abs())
                    * i32::from(edge.weight);
            }
        }
        score
    }
}

impl WavePredictorRoleBindingOffloadRuntime {
    pub fn inspect_package_bytes(
        bytes: &[u8],
    ) -> Result<WavePredictorRoleBindingPackageInfo, WavePredictorRoleBindingPackageError> {
        WavePredictorFlatRoleBindingTable::inspect_bytes(bytes)
    }

    pub fn from_package_bytes(
        bytes: &[u8],
        policy: WavePredictorRoleBindingOffloadPolicy,
    ) -> Result<Self, WavePredictorRoleBindingPackageError> {
        Self::from_package_bytes_with_reference(bytes, policy)
    }

    pub fn from_package_bytes_with_reference(
        bytes: &[u8],
        policy: WavePredictorRoleBindingOffloadPolicy,
    ) -> Result<Self, WavePredictorRoleBindingPackageError> {
        if policy.local_margin_threshold <= 0 {
            return Err(WavePredictorRoleBindingPackageError::InvalidPolicy);
        }
        let package_info = WavePredictorFlatRoleBindingTable::inspect_bytes(bytes)?;
        let table = WavePredictorFlatRoleBindingTable::from_bytes(bytes)?;
        let edge_index = role_binding_edge_index(table.edges());
        let (packed_groups, packed_group_offsets, packed_edges) =
            role_binding_packed_groups(table.edges(), table.action_base, table.action_count);
        Ok(Self {
            table: Some(table),
            edge_index: Some(edge_index),
            packed_groups,
            packed_group_offsets,
            packed_edges,
            package_info,
            policy,
        })
    }

    pub fn from_package_bytes_serving_packed_only(
        bytes: &[u8],
        policy: WavePredictorRoleBindingOffloadPolicy,
    ) -> Result<Self, WavePredictorRoleBindingPackageError> {
        if policy.local_margin_threshold <= 0 {
            return Err(WavePredictorRoleBindingPackageError::InvalidPolicy);
        }
        let package_info = WavePredictorFlatRoleBindingTable::inspect_bytes(bytes)?;
        let table = WavePredictorFlatRoleBindingTable::from_bytes(bytes)?;
        let (packed_groups, packed_group_offsets, packed_edges) =
            role_binding_packed_groups(table.edges(), table.action_base, table.action_count);
        Ok(Self {
            table: None,
            edge_index: None,
            packed_groups,
            packed_group_offsets,
            packed_edges,
            package_info,
            policy,
        })
    }

    pub fn package_info(&self) -> WavePredictorRoleBindingPackageInfo {
        self.package_info
    }

    pub fn policy(&self) -> WavePredictorRoleBindingOffloadPolicy {
        self.policy
    }

    pub fn edge_count(&self) -> usize {
        self.package_info.edge_count
    }

    pub fn bytes_estimate(&self) -> usize {
        self.table
            .as_ref()
            .map(WavePredictorFlatRoleBindingTable::byte_size_estimate)
            .unwrap_or(0)
            + self.packed_groups.len() * std::mem::size_of::<WavePredictorPackedRoleBindingGroup>()
            + self.packed_group_offsets.len() * std::mem::size_of::<usize>()
            + self.packed_edges.len() * std::mem::size_of::<WavePredictorPackedRoleBindingEdge>()
    }

    pub fn table(&self) -> &WavePredictorFlatRoleBindingTable {
        self.table
            .as_ref()
            .expect("reference role-binding table is unavailable in serving packed-only runtime")
    }

    pub fn has_reference_table(&self) -> bool {
        self.table.is_some()
    }

    pub fn sample_positive_edge(&self) -> Option<WavePredictorRoleBindingRuntimeEdgeSample> {
        for group in &self.packed_groups {
            for edge in &self.packed_edges[group.start..group.end] {
                if edge.weight <= 0 {
                    continue;
                }
                return Some(WavePredictorRoleBindingRuntimeEdgeSample {
                    action_center: group.action_center,
                    output_slot_id: group.output_slot_id,
                    slot_id: edge.slot_id,
                    sign_key: group.sign_key,
                    weight: edge.weight,
                });
            }
        }
        None
    }

    pub fn prepare_active_fringe(
        &self,
        active_fringe: &[WavePredictorActiveCenter],
    ) -> WavePredictorRoleBindingPreparedFringe {
        self.prepare_active_fringe_from_iter(
            active_fringe
                .iter()
                .map(|active| (active.center_id, active.strength)),
        )
    }

    pub fn prepare_active_fringe_from_iter<I>(
        &self,
        active_fringe: I,
    ) -> WavePredictorRoleBindingPreparedFringe
    where
        I: IntoIterator<Item = (WavePredictorCenterId, i16)>,
    {
        let mut active_actions = Vec::new();
        let mut slot_actions: HashMap<u8, Vec<(WavePredictorCenterId, i16)>> = HashMap::new();
        let mut role_strengths = HashMap::new();
        let action_end = self
            .package_info
            .action_base
            .saturating_add(self.package_info.action_count);

        for (center_id, strength) in active_fringe {
            if strength == 0 {
                continue;
            }
            let strength = strength.abs();
            if center_id >= self.package_info.action_base && center_id < action_end {
                if let Some(output_slot) = self.slot_scoped_output_slot(center_id) {
                    slot_actions
                        .entry(output_slot)
                        .or_default()
                        .push((center_id, strength));
                } else {
                    active_actions.push((center_id, strength));
                }
                continue;
            }
            if self.package_info.role_stride == 0 || center_id < self.package_info.role_base {
                continue;
            }
            let role_offset = center_id - self.package_info.role_base;
            let slot_id = role_offset / self.package_info.role_stride;
            let lane = role_offset % self.package_info.role_stride;
            if let Ok(slot_id) = u8::try_from(slot_id) {
                role_strengths
                    .entry((slot_id, lane))
                    .and_modify(|existing_strength: &mut i16| {
                        *existing_strength = (*existing_strength).max(strength);
                    })
                    .or_insert(strength);
            }
        }

        WavePredictorRoleBindingPreparedFringe {
            active_actions,
            slot_actions,
            role_strengths,
        }
    }

    pub fn score_alignment(
        &self,
        lane_id: u16,
        signed_strength: i16,
        active_fringe: &[WavePredictorActiveCenter],
        binding_output_slot: Option<u8>,
    ) -> i32 {
        let prepared = self.prepare_active_fringe(active_fringe);
        self.score_alignment_prepared(&prepared, lane_id, signed_strength, binding_output_slot)
    }

    pub fn score_alignment_prepared(
        &self,
        prepared: &WavePredictorRoleBindingPreparedFringe,
        lane_id: u16,
        signed_strength: i16,
        binding_output_slot: Option<u8>,
    ) -> i32 {
        self.score_alignment_prepared_packed(
            prepared,
            lane_id,
            signed_strength,
            binding_output_slot,
        )
    }

    pub fn score_alignment_prepared_reference(
        &self,
        prepared: &WavePredictorRoleBindingPreparedFringe,
        lane_id: u16,
        signed_strength: i16,
        binding_output_slot: Option<u8>,
    ) -> i32 {
        if self.package_info.action_count == 0 || self.package_info.role_stride == 0 {
            return 0;
        }
        let edge_index = self
            .edge_index
            .as_ref()
            .expect("reference edge index is unavailable in serving packed-only runtime");
        let output_slot_id = binding_output_slot.unwrap_or(0);
        let sign_key = binding_sign_key(signed_strength);
        let lane = WavePredictorCenterId::from(lane_id);
        let projected_lane = if lane >= self.package_info.role_stride {
            lane % self.package_info.role_stride
        } else {
            lane
        };
        let mut score = 0i32;
        let global_actions = prepared.active_actions.iter();
        let slot_actions = prepared
            .slot_actions
            .get(&output_slot_id)
            .into_iter()
            .flatten();
        for (action_center, action_strength) in global_actions.chain(slot_actions) {
            let Some(edges) = edge_index.get(&(*action_center, output_slot_id, sign_key)) else {
                continue;
            };
            for (slot_id, weight) in edges {
                let Some(role_strength) = prepared.role_strengths.get(&(*slot_id, projected_lane))
                else {
                    continue;
                };
                score +=
                    i32::from(*action_strength) * i32::from(*role_strength) * i32::from(*weight);
            }
        }
        score
    }

    fn score_alignment_prepared_packed(
        &self,
        prepared: &WavePredictorRoleBindingPreparedFringe,
        lane_id: u16,
        signed_strength: i16,
        binding_output_slot: Option<u8>,
    ) -> i32 {
        if self.package_info.action_count == 0 || self.package_info.role_stride == 0 {
            return 0;
        }
        let output_slot_id = binding_output_slot.unwrap_or(0);
        let sign_key = binding_sign_key(signed_strength);
        let lane = WavePredictorCenterId::from(lane_id);
        let projected_lane = if lane >= self.package_info.role_stride {
            lane % self.package_info.role_stride
        } else {
            lane
        };
        let mut score = 0i32;
        let global_actions = prepared.active_actions.iter();
        let slot_actions = prepared
            .slot_actions
            .get(&output_slot_id)
            .into_iter()
            .flatten();
        for (action_center, action_strength) in global_actions.chain(slot_actions) {
            if *action_center < self.package_info.action_base {
                continue;
            }
            let action_index = (*action_center - self.package_info.action_base) as usize;
            if action_index + 1 >= self.packed_group_offsets.len() {
                continue;
            }
            let group_start = self.packed_group_offsets[action_index];
            let group_end = self.packed_group_offsets[action_index + 1];
            for group in &self.packed_groups[group_start..group_end] {
                if group.output_slot_id != output_slot_id || group.sign_key != sign_key {
                    continue;
                }
                for edge in &self.packed_edges[group.start..group.end] {
                    let Some(role_strength) =
                        prepared.role_strengths.get(&(edge.slot_id, projected_lane))
                    else {
                        continue;
                    };
                    score += i32::from(*action_strength)
                        * i32::from(*role_strength)
                        * i32::from(edge.weight);
                }
            }
        }
        score
    }

    pub fn score_task(&self, task: &WavePredictorRoleBindingEvalTask<'_>) -> i32 {
        let prepared = self.prepare_active_fringe(task.active_fringe);
        let target_score = self.score_alignment_prepared(
            &prepared,
            task.target_lane_id,
            task.target_signed_strength,
            task.binding_output_slot,
        );
        let wrong_score = self.score_alignment_prepared(
            &prepared,
            task.wrong_lane_id,
            task.wrong_signed_strength,
            task.binding_output_slot,
        );
        target_score - wrong_score
    }

    pub fn decide_task(
        &self,
        task: &WavePredictorRoleBindingEvalTask<'_>,
    ) -> WavePredictorRoleBindingDecision {
        let margin = self.score_task(task);
        let action = if margin >= self.policy.local_margin_threshold {
            WavePredictorRoleBindingOffloadAction::LocalOperator
        } else {
            WavePredictorRoleBindingOffloadAction::FallbackToLlm
        };
        WavePredictorRoleBindingDecision { action, margin }
    }

    pub fn offload_summary_into(
        &self,
        tasks: &[WavePredictorRoleBindingEvalTask<'_>],
        decisions: &mut Vec<WavePredictorRoleBindingDecision>,
        margins: &mut Vec<i32>,
    ) -> Result<WavePredictorRoleBindingOffloadSummary, WavePredictorRoleBindingPackageError> {
        if self.policy.local_margin_threshold <= 0 {
            return Err(WavePredictorRoleBindingPackageError::InvalidPolicy);
        }

        decisions.clear();
        margins.clear();
        let mut summary = WavePredictorRoleBindingOffloadSummary {
            calls: tasks.len(),
            local_operator_calls: 0,
            fallback_to_llm_calls: 0,
            false_local_accepts: 0,
        };

        for task in tasks {
            let decision = self.decide_task(task);
            summary.local_operator_calls += usize::from(
                decision.action == WavePredictorRoleBindingOffloadAction::LocalOperator,
            );
            summary.fallback_to_llm_calls += usize::from(
                decision.action == WavePredictorRoleBindingOffloadAction::FallbackToLlm,
            );
            summary.false_local_accepts += usize::from(
                decision.action == WavePredictorRoleBindingOffloadAction::LocalOperator
                    && !task.expect_local_operator,
            );
            margins.push(decision.margin);
            decisions.push(decision);
        }
        Ok(summary)
    }

    fn slot_scoped_output_slot(&self, center_id: WavePredictorCenterId) -> Option<u8> {
        if self.package_info.slot_scoped_action_page_bits == 0
            || self.package_info.slot_scoped_action_source_bits == 0
        {
            return None;
        }
        let page = center_id >> u32::from(self.package_info.slot_scoped_action_page_bits);
        if page >= u64::BITS
            || (self.package_info.slot_scoped_action_page_mask & (1_u64 << page)) == 0
        {
            return None;
        }
        let lane_mask = (1_u32 << u32::from(self.package_info.slot_scoped_action_page_bits)) - 1;
        let lane = center_id & lane_mask;
        u8::try_from(lane >> u32::from(self.package_info.slot_scoped_action_source_bits)).ok()
    }
}

fn role_binding_table_from_parts(
    layout: RoleBindingPackageLayout,
    mut edges: Vec<WavePredictorFlatRoleBindingEdge>,
) -> WavePredictorFlatRoleBindingTable {
    edges.sort_by_key(|edge| {
        (
            edge.action_center,
            edge.output_slot_id,
            edge.slot_id,
            edge.sign_key,
        )
    });
    let mut action_offsets = vec![0usize; layout.action_count as usize + 1];
    for edge in &edges {
        if edge.action_center < layout.action_base {
            continue;
        }
        let action_index = (edge.action_center - layout.action_base) as usize;
        if action_index < layout.action_count as usize {
            action_offsets[action_index + 1] += 1;
        }
    }
    for index in 1..action_offsets.len() {
        action_offsets[index] += action_offsets[index - 1];
    }
    WavePredictorFlatRoleBindingTable {
        action_base: layout.action_base,
        action_count: layout.action_count,
        role_base: layout.role_base,
        role_stride: layout.role_stride,
        slot_scoped_action_page_bits: layout.slot_scoped_action_page_bits,
        slot_scoped_action_page_mask: layout.slot_scoped_action_page_mask,
        slot_scoped_action_source_bits: layout.slot_scoped_action_source_bits,
        edges,
        action_offsets,
    }
}

fn role_binding_edge_index(
    edges: &[WavePredictorFlatRoleBindingEdge],
) -> WavePredictorRoleBindingEdgeIndex {
    let mut grouped: BTreeMap<(WavePredictorCenterId, u8, u8), BTreeMap<u8, i16>> = BTreeMap::new();
    for edge in edges {
        grouped
            .entry((edge.action_center, edge.output_slot_id, edge.sign_key))
            .or_default()
            .entry(edge.slot_id)
            .and_modify(|weight| {
                *weight = clamp_i16(i32::from(*weight) + i32::from(edge.weight), i16::MAX);
            })
            .or_insert(edge.weight);
    }
    grouped
        .into_iter()
        .map(|(key, by_slot)| (key, by_slot.into_iter().collect()))
        .collect()
}

fn role_binding_packed_groups(
    edges: &[WavePredictorFlatRoleBindingEdge],
    action_base: WavePredictorCenterId,
    action_count: WavePredictorCenterId,
) -> (
    Vec<WavePredictorPackedRoleBindingGroup>,
    Vec<usize>,
    Vec<WavePredictorPackedRoleBindingEdge>,
) {
    let mut grouped: BTreeMap<(WavePredictorCenterId, u8, u8), BTreeMap<u8, i16>> = BTreeMap::new();
    for edge in edges {
        grouped
            .entry((edge.action_center, edge.output_slot_id, edge.sign_key))
            .or_default()
            .entry(edge.slot_id)
            .and_modify(|weight| {
                *weight = clamp_i16(i32::from(*weight) + i32::from(edge.weight), i16::MAX);
            })
            .or_insert(edge.weight);
    }

    let mut groups = Vec::with_capacity(grouped.len());
    let mut packed_edges = Vec::new();
    for ((action_center, output_slot_id, sign_key), by_slot) in grouped {
        let start = packed_edges.len();
        packed_edges.extend(
            by_slot
                .into_iter()
                .map(|(slot_id, weight)| WavePredictorPackedRoleBindingEdge { slot_id, weight }),
        );
        let end = packed_edges.len();
        groups.push(WavePredictorPackedRoleBindingGroup {
            action_center,
            output_slot_id,
            sign_key,
            start,
            end,
        });
    }
    let mut group_offsets = vec![0usize; action_count as usize + 1];
    for group in &groups {
        if group.action_center < action_base {
            continue;
        }
        let action_index = (group.action_center - action_base) as usize;
        if action_index < action_count as usize {
            group_offsets[action_index + 1] += 1;
        }
    }
    for index in 1..group_offsets.len() {
        group_offsets[index] += group_offsets[index - 1];
    }
    (groups, group_offsets, packed_edges)
}

fn role_binding_package_len(edge_count: usize) -> Option<usize> {
    edge_count
        .checked_mul(WAVE_PREDICTOR_ROLE_BINDING_EDGE_BYTES)?
        .checked_add(WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_HEADER_BYTES)
}

fn read_role_binding_u32_le(
    bytes: &[u8],
    offset: usize,
) -> Result<u32, WavePredictorRoleBindingPackageError> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or(WavePredictorRoleBindingPackageError::InvalidRuntimePackage)?;
    Ok(u32::from_le_bytes(slice.try_into().map_err(|_| {
        WavePredictorRoleBindingPackageError::InvalidRuntimePackage
    })?))
}

fn read_role_binding_i16_le(
    bytes: &[u8],
    offset: usize,
) -> Result<i16, WavePredictorRoleBindingPackageError> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or(WavePredictorRoleBindingPackageError::InvalidRuntimePackage)?;
    Ok(i16::from_le_bytes(slice.try_into().map_err(|_| {
        WavePredictorRoleBindingPackageError::InvalidRuntimePackage
    })?))
}

fn read_role_binding_u64_le(
    bytes: &[u8],
    offset: usize,
) -> Result<u64, WavePredictorRoleBindingPackageError> {
    let slice = bytes
        .get(offset..offset + 8)
        .ok_or(WavePredictorRoleBindingPackageError::InvalidRuntimePackage)?;
    Ok(u64::from_le_bytes(slice.try_into().map_err(|_| {
        WavePredictorRoleBindingPackageError::InvalidRuntimePackage
    })?))
}

fn role_binding_package_fingerprint64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_contrastive_error_update_repairs_weak_margin_locally() {
        let mut field = WavePredictorHebbianField::new(6, WavePredictorHebbianConfig::default());
        for center in 0..6 {
            field.set_base_mass(center, 100 + center as i16);
        }
        field.insert_edge(WavePredictorHebbianEdge {
            source_center: 0,
            target_center: 3,
            compatibility: 24,
            ..WavePredictorHebbianEdge::default()
        });
        field.insert_edge(WavePredictorHebbianEdge {
            source_center: 5,
            target_center: 4,
            compatibility: 99,
            ..WavePredictorHebbianEdge::default()
        });

        let error = WavePredictorConvergenceError {
            active_fringe: vec![
                WavePredictorActiveCenter {
                    center_id: 0,
                    strength: 12,
                },
                WavePredictorActiveCenter {
                    center_id: 1,
                    strength: 8,
                },
            ],
            target_center: 2,
            nearest_wrong_center: 3,
            target_gap: -288,
            margin_required: 120,
            trap_accepted: false,
        };

        let untouched_before = field.edge(5, 4);
        let report = field.apply_sparse_contrastive_error(&error);

        assert!(
            report.target_gap_after > report.target_gap_before,
            "report={report:#?}"
        );
        assert!(report.margin_fixed, "report={report:#?}");
        assert_eq!(report.attraction_updates, 2);
        assert_eq!(report.repulsion_updates, 2);
        assert_eq!(report.conflict_updates, 2);
        assert_eq!(report.anti_wave_updates, 0);
        assert!(!report.base_mass_drift_detected, "report={report:#?}");
        assert_eq!(field.base_mass(2), Some(102));
        assert_eq!(field.edge(5, 4), untouched_before);
        assert!(
            field
                .edge(0, 2)
                .expect("target edge should be created")
                .compatibility
                > 0
        );
        let wrong_edge = field
            .edge(0, 3)
            .expect("wrong edge should be updated for repulsion");
        assert!(wrong_edge.compatibility < 24);
        assert!(wrong_edge.conflict > 0);
    }

    #[test]
    fn accepted_trap_reinforces_only_wrong_anti_wave() {
        let mut field = WavePredictorHebbianField::new(4, WavePredictorHebbianConfig::default());
        let error = WavePredictorConvergenceError {
            active_fringe: vec![WavePredictorActiveCenter {
                center_id: 0,
                strength: 5,
            }],
            target_center: 1,
            nearest_wrong_center: 2,
            target_gap: 200,
            margin_required: 0,
            trap_accepted: true,
        };

        let report = field.apply_sparse_contrastive_error(&error);

        assert_eq!(report.attraction_updates, 0);
        assert_eq!(report.repulsion_updates, 0);
        assert_eq!(report.conflict_updates, 0);
        assert_eq!(report.anti_wave_updates, 1);
        assert_eq!(field.edge(0, 1), None);
        assert!(
            field
                .edge(0, 2)
                .expect("wrong edge should receive anti-wave")
                .anti_wave
                > 0
        );
        assert!(!report.base_mass_drift_detected, "report={report:#?}");
    }

    #[test]
    fn slot_scoped_operator_pair_actions_only_vote_for_their_output_slot() {
        let page_bits = 12;
        let role_stride = 1 << page_bits;
        let scoped_page = 17_u32;
        let config = WavePredictorHebbianConfig {
            eta_binding: 2,
            state_delta_binding_action_base: Some(16 << page_bits),
            state_delta_binding_action_count: role_stride * 2,
            state_delta_binding_role_base: Some(0),
            state_delta_binding_role_stride: role_stride,
            state_delta_binding_role_count: 2,
            state_delta_binding_slot_scoped_action_page_bits: page_bits as u8,
            state_delta_binding_slot_scoped_action_page_mask: 1_u64 << scoped_page,
            state_delta_binding_slot_scoped_action_source_bits: 4,
            ..WavePredictorHebbianConfig::default()
        };
        let mut field = WavePredictorHebbianField::new(1 << 18, config);
        let lane_id = 42_u16;
        let scoped_action = (scoped_page << page_bits) | (1 << 4);
        let active_fringe = vec![
            WavePredictorActiveCenter {
                center_id: scoped_action,
                strength: 1,
            },
            WavePredictorActiveCenter {
                center_id: u32::from(lane_id),
                strength: 1,
            },
        ];

        assert_eq!(
            field.adjust_state_delta_role_binding(lane_id, 1, &active_fringe, Some(2), 10),
            0
        );
        assert_eq!(
            field.score_state_delta_binding_alignment(lane_id, 1, &active_fringe, Some(2)),
            0
        );
        assert_eq!(
            field.adjust_state_delta_role_binding(lane_id, 1, &active_fringe, Some(1), 10),
            1
        );
        assert_eq!(
            field.score_state_delta_binding_alignment(lane_id, 1, &active_fringe, Some(1)),
            10
        );

        let flat = field.compile_flat_role_binding_table();
        assert_eq!(flat.score_alignment(lane_id, 1, &active_fringe, Some(2)), 0);
        assert_eq!(
            flat.score_alignment(lane_id, 1, &active_fringe, Some(1)),
            10
        );
    }

    #[test]
    fn settled_non_trap_case_does_not_change_weights_or_mass() {
        let mut field = WavePredictorHebbianField::new(3, WavePredictorHebbianConfig::default());
        field.set_base_mass(1, 42);
        field.insert_edge(WavePredictorHebbianEdge {
            source_center: 0,
            target_center: 1,
            compatibility: 64,
            ..WavePredictorHebbianEdge::default()
        });
        let before = field.clone();
        let error = WavePredictorConvergenceError {
            active_fringe: vec![WavePredictorActiveCenter {
                center_id: 0,
                strength: 4,
            }],
            target_center: 1,
            nearest_wrong_center: 2,
            target_gap: 256,
            margin_required: 120,
            trap_accepted: false,
        };

        let report = field.apply_sparse_contrastive_error(&error);

        assert_eq!(report.touched_edges, 0);
        assert_eq!(field, before);
        assert!(!report.base_mass_drift_detected, "report={report:#?}");
    }
}
