use super::CELL32_BYTES;

/// Cache profile for the ThinkPad T480 / i7-8650U target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheProfile {
    pub cores: usize,
    pub l1d_bytes_per_core: usize,
    pub l2_bytes_per_core: usize,
    pub l3_bytes_shared: usize,
}

/// Cell role counts for the first L3-sized organism plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Organ128Plan {
    pub fast_cells: usize,
    pub mid_cells: usize,
    pub guard_cells: usize,
    pub carrier_cells: usize,
    pub memory_cells: usize,
}

/// Runtime hot-window limits used to keep the CPU cache hierarchy sane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotWindowPlan {
    pub l1_active_cells_total: usize,
    pub l2_hot_cells_total: usize,
    pub l3_warm_cells_target: usize,
    pub l3_warm_cells_max: usize,
    pub ram_cold_cells_min: usize,
}

/// Full cache-aware planning summary for Nando Wave on the T480 class CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheAwareOrganPlan {
    pub profile: CacheProfile,
    pub organ128: Organ128Plan,
    pub hot_window: HotWindowPlan,
    pub organ128_bytes: usize,
    pub l3_target_bytes: usize,
    pub l3_max_bytes: usize,
}

impl CacheProfile {
    /// Intel i7-8650U as reported by `lscpu` on the target T480.
    #[must_use]
    pub const fn t480_i7_8650u() -> Self {
        Self {
            cores: 4,
            l1d_bytes_per_core: 32 * 1024,
            l2_bytes_per_core: 256 * 1024,
            l3_bytes_shared: 8 * 1024 * 1024,
        }
    }
}

impl Organ128Plan {
    #[must_use]
    pub const fn cell_count(self) -> usize {
        self.fast_cells + self.mid_cells + self.guard_cells + self.carrier_cells + self.memory_cells
    }

    #[must_use]
    pub const fn byte_size(self) -> usize {
        self.cell_count() * CELL32_BYTES
    }
}

impl CacheAwareOrganPlan {
    /// First serious organism plan: 128 warm Cell32 packets in shared L3.
    #[must_use]
    pub const fn t480_organ128() -> Self {
        let profile = CacheProfile::t480_i7_8650u();
        let organ128 = Organ128Plan {
            fast_cells: 64,
            mid_cells: 32,
            guard_cells: 16,
            carrier_cells: 8,
            memory_cells: 8,
        };
        let hot_window = HotWindowPlan {
            l1_active_cells_total: profile.cores,
            l2_hot_cells_total: profile.cores * 8,
            l3_warm_cells_target: 128,
            l3_warm_cells_max: 256,
            ram_cold_cells_min: 1024,
        };

        Self {
            profile,
            organ128,
            hot_window,
            organ128_bytes: organ128.byte_size(),
            l3_target_bytes: hot_window.l3_warm_cells_target * CELL32_BYTES,
            l3_max_bytes: hot_window.l3_warm_cells_max * CELL32_BYTES,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wave::{PLANNED_ORGAN128_BYTES, PLANNED_ORGAN128_CELLS};

    #[test]
    fn t480_organ128_plan_matches_cache_budget() {
        let plan = CacheAwareOrganPlan::t480_organ128();

        assert_eq!(plan.profile.cores, 4);
        assert_eq!(plan.profile.l1d_bytes_per_core, CELL32_BYTES);
        assert_eq!(plan.profile.l2_bytes_per_core / CELL32_BYTES, 8);
        assert_eq!(plan.profile.l3_bytes_shared / CELL32_BYTES, 256);
        assert_eq!(plan.organ128.cell_count(), PLANNED_ORGAN128_CELLS);
        assert_eq!(plan.organ128.byte_size(), PLANNED_ORGAN128_BYTES);
        assert_eq!(plan.organ128.fast_cells, 64);
        assert_eq!(plan.organ128.mid_cells, 32);
        assert_eq!(plan.organ128.guard_cells, 16);
        assert_eq!(plan.organ128.carrier_cells, 8);
        assert_eq!(plan.organ128.memory_cells, 8);
        assert_eq!(plan.hot_window.l1_active_cells_total, 4);
        assert_eq!(plan.hot_window.l2_hot_cells_total, 32);
        assert_eq!(plan.hot_window.l3_warm_cells_target, 128);
        assert_eq!(plan.hot_window.l3_warm_cells_max, 256);
    }
}
