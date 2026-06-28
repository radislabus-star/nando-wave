use super::{
    CELL32_BYTES, SYMBOL_CELL_DENSE2K_BYTES, SYMBOL_CELL_DENSE2K_INTERFERENCE_SLOTS,
    SYMBOL_CELL_DENSE2K_MODES, SYMBOL_CELL_DENSE2K_TRANSITIONS, SYMBOL_CELL8_BYTES,
    SYMBOL_CELL8_CALIBRATION_STATS_BYTES, SYMBOL_CELL8_INTERFERENCE_SLOTS, SYMBOL_CELL8_MODES,
    SYMBOL_CELL8_PROJECTION_LANES, SYMBOL_CELL8_SCRATCH_BYTES, SYMBOL_CELL8_TRANSITIONS,
    SYMBOL_CELL32_CALIBRATION_STATS_BYTES, SYMBOL_CELL32_INTERFERENCE_SLOTS, SYMBOL_CELL32_MODES,
    SYMBOL_CELL32_PROJECTION_BYTES, SYMBOL_CELL32_SCRATCH_BYTES, SYMBOL_CELL32_TRANSITIONS,
    SYMBOL_CLIQUE_CLASS_BYTES, unit_noise,
};

const PROJECTION_ENTRIES: usize = SYMBOL_CELL32_PROJECTION_BYTES / 16;
const ACTIVE_EXCITATION_MODES: usize = 8;
const CELL8_ACTIVE_MODES: usize = 4;
const CELL8_ACTIVE_SLOT_CAPACITY: usize = 8;
const SYMBOL_CELL_MAGIC: [u8; 8] = *b"NANDA32\0";
const SYMBOL_CELL8_MAGIC: [u8; 8] = *b"NANDA8K\0";
const SYMBOL_CLIQUE_CLASS_MAGIC: [u8; 8] = *b"NANDACLQ";
const SYMBOL_DENSE2K_MAGIC: [u8; 8] = *b"NANDA2K\0";
const SYMBOL_CELL_VERSION: u16 = 1;
const SYMBOL_CELL_SCHEMA: u16 = 1;
const CALIBRATION_STATS_RESERVED_BYTES: usize = SYMBOL_CELL32_CALIBRATION_STATS_BYTES - 28;
const CELL8_CALIBRATION_RESERVED_BYTES: usize =
    SYMBOL_CELL8_CALIBRATION_STATS_BYTES - 60 - 1 - CELL8_ACTIVE_SLOT_CAPACITY;
const SYMBOL_CLIQUE_CLASS_PROJECTION_BYTES: usize = 1_024;
const SYMBOL_CLIQUE_CLASS_CALIBRATION_BYTES: usize = 512;
const SYMBOL_CLIQUE_CLASS_ROLE_BYTES: usize = 256;
const SYMBOL_CLIQUE_CLASS_RESERVED_BYTES: usize = SYMBOL_CLIQUE_CLASS_BYTES
    - 64
    - SYMBOL_CLIQUE_CLASS_PROJECTION_BYTES
    - SYMBOL_CLIQUE_CLASS_CALIBRATION_BYTES
    - SYMBOL_CLIQUE_CLASS_ROLE_BYTES;
const SYMBOL_DENSE2K_MODE_BYTES: usize = SYMBOL_CELL_DENSE2K_MODES * 4;
const SYMBOL_DENSE2K_TRANSITION_BYTES: usize = SYMBOL_CELL_DENSE2K_TRANSITIONS * 4;
const SYMBOL_DENSE2K_INTERFERENCE_BYTES: usize = SYMBOL_CELL_DENSE2K_INTERFERENCE_SLOTS * 4;
const SYMBOL_DENSE2K_LOCAL_BYTES: usize = 32;
const SYMBOL_DENSE2K_RESERVED_BYTES: usize = SYMBOL_CELL_DENSE2K_BYTES
    - SYMBOL_DENSE2K_MODE_BYTES
    - SYMBOL_DENSE2K_TRANSITION_BYTES
    - SYMBOL_DENSE2K_INTERFERENCE_BYTES
    - SYMBOL_DENSE2K_LOCAL_BYTES;

/// Fixed header for a 32 KB Unicode symbol wave cell.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymbolHeader {
    pub magic: [u8; 8],
    pub id: u32,
    pub version: u16,
    pub schema: u16,
    pub role: u8,
    pub flags: u8,
    pub checksum: u32,
    reserved: [u8; 232],
}

impl SymbolHeader {
    #[must_use]
    pub fn new(id: u32, role: u8) -> Self {
        Self {
            magic: SYMBOL_CELL_MAGIC,
            id,
            version: SYMBOL_CELL_VERSION,
            schema: SYMBOL_CELL_SCHEMA,
            role,
            flags: 0,
            checksum: id.rotate_left(13) ^ u32::from(role) ^ 0x4E41_4E44,
            reserved: [0; 232],
        }
    }
}

/// One 16 byte Unicode projection lane.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProjectionEntry16 {
    pub utf8_len: u8,
    pub byte_mix: [u8; 4],
    pub lane: u16,
    pub frequency_hint: u16,
    pub amplitude: i8,
    pub phase: i8,
    pub damping: u8,
    pub role: u8,
    reserved: [u8; 2],
}

/// Compact 8 byte mode. This is the main payload of `SymbolCell32`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mode8 {
    pub frequency_id: u16,
    pub sin_weight: i8,
    pub cos_weight: i8,
    pub amplitude: i8,
    pub phase: i8,
    pub damping: u8,
    pub role: u8,
}

/// Compact link from a previous mode into a current mode.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Transition8 {
    pub previous_mode: u16,
    pub current_mode: u16,
    pub coupling: i8,
    pub phase_shift: i8,
    pub damping: u8,
    pub role: u8,
}

/// Accumulated interference slot.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Interference8 {
    pub real: i16,
    pub imag: i16,
    pub energy: u16,
    pub coherence: u16,
}

/// Bit-packed 32 bit mode for dense runtime cells.
///
/// Layout:
/// bits 0..10   frequency id
/// bits 11..15  signed sin weight, biased by 16
/// bits 16..20  signed cos weight, biased by 16
/// bits 21..25  signed amplitude, biased by 16
/// bits 26..30  signed phase, biased by 16
/// bit 31       damping/high role flag selected by the clique class
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PackedMode32(pub u32);

/// Bit-packed transition for dense runtime cells.
///
/// Layout:
/// bits 0..9    previous mode
/// bits 10..19  current mode
/// bits 20..25  signed coupling, biased by 32
/// bits 26..30  signed phase shift, biased by 16
/// bit 31       damping/high role flag selected by the clique class
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PackedTransition32(pub u32);

/// Bit-packed interference slot for dense runtime cells.
///
/// Layout:
/// bits 0..7    real, biased by 128
/// bits 8..15   imag, biased by 128
/// bits 16..23  energy
/// bits 24..31  coherence
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PackedInterference32(pub u32);

/// Calibration thresholds and trust counters for one symbol cell.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalibrationStats {
    pub seen: u32,
    pub accepted: u32,
    pub reverted: u32,
    pub false_positive: u32,
    pub excite: i16,
    pub accept: i16,
    pub veto: i16,
    pub decay: u16,
    pub temperature: i16,
    pub reserved: [u8; CALIBRATION_STATS_RESERVED_BYTES],
}

impl CalibrationStats {
    #[must_use]
    pub fn new() -> Self {
        Self {
            seen: 0,
            accepted: 0,
            reverted: 0,
            false_positive: 0,
            excite: 24,
            accept: 96,
            veto: -96,
            decay: 991,
            temperature: 32,
            reserved: [0; CALIBRATION_STATS_RESERVED_BYTES],
        }
    }
}

impl Default for CalibrationStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixed 128 byte header for a cache-resident 8 KB symbol cell.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymbolCell8Header {
    pub magic: [u8; 8],
    pub id: u32,
    pub version: u16,
    pub schema: u16,
    pub role: u8,
    pub flags: u8,
    pub checksum: u32,
    reserved: [u8; 104],
}

impl SymbolCell8Header {
    #[must_use]
    pub fn new(id: u32, role: u8) -> Self {
        Self {
            magic: SYMBOL_CELL8_MAGIC,
            id,
            version: SYMBOL_CELL_VERSION,
            schema: SYMBOL_CELL_SCHEMA,
            role,
            flags: 0,
            checksum: id.rotate_left(7) ^ u32::from(role) ^ 0x4E38_4B21,
            reserved: [0; 104],
        }
    }
}

/// Calibration, vigilance, trust, and tiny peak history for `SymbolCell8`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymbolCell8Calibration {
    pub seen: u32,
    pub accepted: u32,
    pub supported: u32,
    pub reflected: u32,
    pub vetoed: u32,
    pub false_positive: u32,
    pub spurious: u32,
    pub limit_cycle: u32,
    pub excite: u16,
    pub accept: u16,
    pub veto: u16,
    pub decay: u16,
    pub temperature: u16,
    pub vigilance: u16,
    pub coherence_min: u16,
    pub margin_min: u16,
    pub reflection_min: u16,
    pub second_order_min: u16,
    pub last_peak: u16,
    pub previous_peak: u16,
    pub last_energy: u16,
    pub last_second_order: u16,
    pub active_slot_count: u8,
    pub active_slots: [u8; CELL8_ACTIVE_SLOT_CAPACITY],
    reserved: [u8; CELL8_CALIBRATION_RESERVED_BYTES],
}

impl SymbolCell8Calibration {
    #[must_use]
    pub fn new() -> Self {
        Self {
            seen: 0,
            accepted: 0,
            supported: 0,
            reflected: 0,
            vetoed: 0,
            false_positive: 0,
            spurious: 0,
            limit_cycle: 0,
            excite: 18,
            accept: 42,
            veto: 220,
            decay: 896,
            temperature: 24,
            vigilance: 48,
            coherence_min: 90,
            margin_min: 2,
            reflection_min: 32,
            second_order_min: 28,
            last_peak: u16::MAX,
            previous_peak: u16::MAX,
            last_energy: 0,
            last_second_order: 0,
            active_slot_count: 0,
            active_slots: [0; CELL8_ACTIVE_SLOT_CAPACITY],
            reserved: [0; CELL8_CALIBRATION_RESERVED_BYTES],
        }
    }
}

impl Default for SymbolCell8Calibration {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeakOutcome {
    Accepted = 0,
    Supported = 1,
    Reflected = 2,
    Vetoed = 3,
    Spurious = 4,
    LimitCycle = 5,
    Unstable = 6,
    NoPeak = 7,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StablePeakScore {
    pub energy: u16,
    pub separation: u16,
    pub coherence: u16,
    pub persistence: u16,
    pub transition_support: u16,
    pub second_order: u16,
    pub vigilance: u16,
    pub veto: u16,
    pub stable_score: u16,
}

/// Compact hot-path wave advice. It is exactly 8 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SymbolCell8Advice {
    pub peak_slot: u16,
    pub energy: u16,
    pub coherence: u16,
    pub phase: i8,
    pub role: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymbolCell8Tick {
    pub projection: SymbolProjection,
    pub peak_slot: u16,
    pub second_peak_slot: u16,
    pub score: StablePeakScore,
    pub outcome: PeakOutcome,
    pub advice: Option<SymbolCell8Advice>,
    pub reflection: Option<SymbolCell8Advice>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymbolProjection {
    pub codepoint: u32,
    pub lane: u16,
    pub utf8_len: u8,
    pub bytes: [u8; 4],
    pub amplitude: i8,
    pub phase: i8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SymbolExcitation {
    pub projection: SymbolProjection,
    pub peak_slot: u16,
    pub peak_energy: u16,
    pub coherence: f32,
    pub accepted: bool,
}

/// A fixed 32 KB NANDA symbol cell.
///
/// Layout:
/// header 256 B, Unicode projection 4096 B, 2048 compact modes,
/// 512 transitions, 512 interference slots, calibration/stats, scratch.
#[repr(C)]
#[derive(Clone)]
pub struct SymbolCell32 {
    pub header: SymbolHeader,
    pub projection: [ProjectionEntry16; PROJECTION_ENTRIES],
    pub modes: [Mode8; SYMBOL_CELL32_MODES],
    pub transitions: [Transition8; SYMBOL_CELL32_TRANSITIONS],
    pub interference: [Interference8; SYMBOL_CELL32_INTERFERENCE_SLOTS],
    pub calibration: CalibrationStats,
    scratch: [u8; SYMBOL_CELL32_SCRATCH_BYTES],
}

/// A fixed 8 KB NANDA symbol cell.
///
/// Layout:
/// header 128 B, 64 projection lanes, 512 compact modes, 128 transitions,
/// 128 interference slots, calibration/stats, scratch.
#[repr(C)]
#[derive(Clone)]
pub struct SymbolCell8 {
    pub header: SymbolCell8Header,
    pub projection: [ProjectionEntry16; SYMBOL_CELL8_PROJECTION_LANES],
    pub modes: [Mode8; SYMBOL_CELL8_MODES],
    pub transitions: [Transition8; SYMBOL_CELL8_TRANSITIONS],
    pub interference: [Interference8; SYMBOL_CELL8_INTERFERENCE_SLOTS],
    pub calibration: SymbolCell8Calibration,
    scratch: [u8; SYMBOL_CELL8_SCRATCH_BYTES],
}

/// Shared class metadata for a dense symbol clique.
///
/// This is the part that must not be repeated in every dense cell: schema,
/// projection lanes, calibration defaults, and role layout live once per clique.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymbolCliqueClass {
    pub magic: [u8; 8],
    pub version: u16,
    pub schema: u16,
    pub flags: u32,
    pub cell_bytes: u16,
    pub mode_count: u16,
    pub transition_count: u16,
    pub interference_slots: u16,
    pub projection_lanes: u16,
    pub role_count: u16,
    pub checksum: u32,
    reserved_header: [u8; 32],
    pub projection: [ProjectionEntry16; SYMBOL_CLIQUE_CLASS_PROJECTION_BYTES / 16],
    pub calibration: [u8; SYMBOL_CLIQUE_CLASS_CALIBRATION_BYTES],
    pub role_layout: [u8; SYMBOL_CLIQUE_CLASS_ROLE_BYTES],
    reserved: [u8; SYMBOL_CLIQUE_CLASS_RESERVED_BYTES],
}

/// Dense 2 KB runtime cell. It has no repeated schema, projection table, large
/// calibration block, or scratch padding; those are owned by `SymbolCliqueClass`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymbolCellDense2K {
    pub modes: [PackedMode32; SYMBOL_CELL_DENSE2K_MODES],
    pub transitions: [PackedTransition32; SYMBOL_CELL_DENSE2K_TRANSITIONS],
    pub interference: [PackedInterference32; SYMBOL_CELL_DENSE2K_INTERFERENCE_SLOTS],
    pub local: [u8; SYMBOL_DENSE2K_LOCAL_BYTES],
    reserved: [u8; SYMBOL_DENSE2K_RESERVED_BYTES],
}

impl SymbolCliqueClass {
    #[must_use]
    pub fn new() -> Self {
        Self {
            magic: SYMBOL_CLIQUE_CLASS_MAGIC,
            version: SYMBOL_CELL_VERSION,
            schema: SYMBOL_CELL_SCHEMA,
            flags: 0,
            cell_bytes: SYMBOL_CELL_DENSE2K_BYTES as u16,
            mode_count: SYMBOL_CELL_DENSE2K_MODES as u16,
            transition_count: SYMBOL_CELL_DENSE2K_TRANSITIONS as u16,
            interference_slots: SYMBOL_CELL_DENSE2K_INTERFERENCE_SLOTS as u16,
            projection_lanes: (SYMBOL_CLIQUE_CLASS_PROJECTION_BYTES / 16) as u16,
            role_count: 6,
            checksum: 0x4E44_324B,
            reserved_header: [0; 32],
            projection: [ProjectionEntry16::default(); SYMBOL_CLIQUE_CLASS_PROJECTION_BYTES / 16],
            calibration: [0; SYMBOL_CLIQUE_CLASS_CALIBRATION_BYTES],
            role_layout: [0; SYMBOL_CLIQUE_CLASS_ROLE_BYTES],
            reserved: [0; SYMBOL_CLIQUE_CLASS_RESERVED_BYTES],
        }
    }
}

impl Default for SymbolCliqueClass {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolCellDense2K {
    #[must_use]
    pub fn new() -> Self {
        let mut local = [0; SYMBOL_DENSE2K_LOCAL_BYTES];
        local[0..8].copy_from_slice(&SYMBOL_DENSE2K_MAGIC);
        Self {
            modes: [PackedMode32::default(); SYMBOL_CELL_DENSE2K_MODES],
            transitions: [PackedTransition32::default(); SYMBOL_CELL_DENSE2K_TRANSITIONS],
            interference: [PackedInterference32::default(); SYMBOL_CELL_DENSE2K_INTERFERENCE_SLOTS],
            local,
            reserved: [0; SYMBOL_DENSE2K_RESERVED_BYTES],
        }
    }
}

impl Default for SymbolCellDense2K {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolCell8 {
    #[must_use]
    pub fn new(id: u32, role: u8, seed: u64) -> Self {
        let mut projection = [ProjectionEntry16::default(); SYMBOL_CELL8_PROJECTION_LANES];
        for (index, entry) in projection.iter_mut().enumerate() {
            let unit_a = unit_noise(seed, id as u64, index as u64, 0x384B_5052);
            let unit_b = unit_noise(seed, id as u64, index as u64, 0x384B_4C4E);
            entry.utf8_len = (index % 4 + 1) as u8;
            entry.byte_mix = [
                index as u8,
                (index.rotate_left(2) & 0xFF) as u8,
                (index.rotate_left(5) & 0xFF) as u8,
                (index ^ 0x3C) as u8,
            ];
            entry.lane = index as u16;
            entry.frequency_hint = ((unit_a * u16::MAX as f32) as u16) & 0x01FF;
            entry.amplitude = signed_unit_i8(unit_b);
            entry.phase = signed_unit_i8(unit_a);
            entry.damping = 224 + (index % 32) as u8;
            entry.role = role;
        }

        let mut modes = [Mode8::default(); SYMBOL_CELL8_MODES];
        for (index, mode) in modes.iter_mut().enumerate() {
            let unit_a = unit_noise(seed, id as u64, index as u64, 0x384B_4D4F);
            let unit_b = unit_noise(seed, id as u64, index as u64, 0x384B_5349);
            let unit_c = unit_noise(seed, id as u64, index as u64, 0x384B_434F);
            mode.frequency_id = (index as u16).wrapping_mul(13) & 0x01FF;
            mode.sin_weight = signed_unit_i8(unit_b);
            mode.cos_weight = signed_unit_i8(unit_c);
            mode.amplitude = signed_unit_i8(unit_a).saturating_abs().max(1);
            mode.phase = signed_unit_i8(unit_a);
            mode.damping = 224 + (index % 32) as u8;
            mode.role = role;
        }

        let mut transitions = [Transition8::default(); SYMBOL_CELL8_TRANSITIONS];
        for (index, transition) in transitions.iter_mut().enumerate() {
            let unit_a = unit_noise(seed, id as u64, index as u64, 0x384B_5452);
            let unit_b = unit_noise(seed, id as u64, index as u64, 0x384B_5048);
            transition.previous_mode = ((index * 5) % SYMBOL_CELL8_MODES) as u16;
            transition.current_mode = ((index * 5 + 1) % SYMBOL_CELL8_MODES) as u16;
            transition.coupling = signed_unit_i8(unit_a);
            transition.phase_shift = signed_unit_i8(unit_b);
            transition.damping = 224 + (index % 32) as u8;
            transition.role = role;
        }

        Self {
            header: SymbolCell8Header::new(id, role),
            projection,
            modes,
            transitions,
            interference: [Interference8::default(); SYMBOL_CELL8_INTERFERENCE_SLOTS],
            calibration: SymbolCell8Calibration::new(),
            scratch: [0; SYMBOL_CELL8_SCRATCH_BYTES],
        }
    }

    #[must_use]
    pub fn project_symbol(&self, symbol: char) -> SymbolProjection {
        let codepoint = symbol as u32;
        let mut bytes = [0; 4];
        let utf8_len = symbol.encode_utf8(&mut bytes).len() as u8;
        let lane =
            projection_lane_for_entries(codepoint, bytes, utf8_len, SYMBOL_CELL8_PROJECTION_LANES);
        let entry = self.projection[lane as usize];

        SymbolProjection {
            codepoint,
            lane,
            utf8_len,
            bytes,
            amplitude: entry.amplitude,
            phase: entry.phase,
        }
    }

    pub fn tick_symbol(&mut self, symbol: char) -> SymbolCell8Tick {
        self.tick_symbol_with_context(symbol, None, 0, &[])
    }

    pub fn tick_symbol_with_context(
        &mut self,
        symbol: char,
        previous_peak: Option<u16>,
        carrier_phase: i8,
        incoming: &[SymbolCell8Advice],
    ) -> SymbolCell8Tick {
        self.calibration.seen = self.calibration.seen.saturating_add(1);
        self.decay_active_interference();

        let projection = self.project_symbol(symbol);
        let previous_peak = previous_peak.or_else(|| {
            (self.calibration.last_peak != u16::MAX).then_some(self.calibration.last_peak)
        });
        let mut transition_support = 0u16;
        let base_slot = cell8_base_slot(projection, previous_peak, carrier_phase);

        for offset in 0..CELL8_ACTIVE_MODES {
            let mode_index = cell8_mode_index(projection, previous_peak, carrier_phase, offset);
            let mode = self.modes[mode_index];
            let transition = self.transition_for(previous_peak, mode_index, offset);
            let slot_index = (base_slot + offset % 2) % SYMBOL_CELL8_INTERFERENCE_SLOTS;
            self.activate_slot(slot_index);
            let slot = &mut self.interference[slot_index];
            transition_support =
                transition_support.saturating_add(transition_support_score(transition));
            apply_mode_to_slot_cell8(slot, mode, transition, projection, carrier_phase);
        }

        transition_support = transition_support
            .saturating_add(previous_peak.map_or(24, |_| 80))
            .min(255);

        let (peak_slot, second_peak_slot) = self.top_two_active_slots();
        let peak = self.interference[peak_slot];
        let second = self.interference[second_peak_slot];
        let separation = peak.energy.saturating_sub(second.energy);
        let persistence = self.persistence_score(peak_slot as u16);
        let second_order = second_order_score(peak, projection, carrier_phase);
        let vigilance_score = 255u16.saturating_sub(self.calibration.vigilance / 2);
        let veto = veto_score(peak, separation, self.calibration);
        let stable_score = peak
            .energy
            .min(separation.saturating_mul(16))
            .min(peak.coherence)
            .min(persistence)
            .min(transition_support)
            .min(second_order)
            .min(vigilance_score);
        let score = StablePeakScore {
            energy: peak.energy,
            separation,
            coherence: peak.coherence,
            persistence,
            transition_support,
            second_order,
            vigilance: vigilance_score,
            veto,
            stable_score,
        };

        let reflection_energy = incoming_mismatch_energy(incoming, carrier_phase, self.header.role);
        let limit_cycle = self.is_limit_cycle(peak_slot as u16);
        let outcome = classify_peak(score, self.calibration, limit_cycle, reflection_energy);
        let advice = advice_for_outcome(
            outcome,
            peak_slot as u16,
            peak,
            projection,
            self.header.role,
        );
        let reflection =
            (reflection_energy >= self.calibration.reflection_min).then_some(SymbolCell8Advice {
                peak_slot: peak_slot as u16,
                energy: reflection_energy,
                coherence: 0,
                phase: projection.phase.wrapping_neg(),
                role: self.header.role,
            });

        self.record_outcome(outcome);
        self.update_peak_history(peak_slot as u16, peak.energy, second_order);

        SymbolCell8Tick {
            projection,
            peak_slot: peak_slot as u16,
            second_peak_slot: second_peak_slot as u16,
            score,
            outcome,
            advice,
            reflection,
        }
    }

    fn transition_for(
        &self,
        previous_peak: Option<u16>,
        mode_index: usize,
        offset: usize,
    ) -> Transition8 {
        let previous = usize::from(previous_peak.unwrap_or(0));
        let index = previous
            .wrapping_mul(17)
            .wrapping_add(mode_index)
            .wrapping_add(offset * 7)
            % SYMBOL_CELL8_TRANSITIONS;
        self.transitions[index]
    }

    fn decay_active_interference(&mut self) {
        let decay = u32::from(self.calibration.decay);
        let count = self.active_slot_count();
        let mut write = 0usize;

        for read in 0..count {
            let slot_index = usize::from(self.calibration.active_slots[read]);
            if slot_index >= SYMBOL_CELL8_INTERFERENCE_SLOTS {
                continue;
            }

            decay_interference_slot(&mut self.interference[slot_index], decay);
            if self.interference[slot_index].energy > 0 {
                let slot = slot_index as u8;
                if !self.calibration.active_slots[..write].contains(&slot) {
                    self.calibration.active_slots[write] = slot;
                    write += 1;
                }
            }
        }

        for slot in &mut self.calibration.active_slots[write..count] {
            *slot = 0;
        }
        self.calibration.active_slot_count = write as u8;
    }

    fn activate_slot(&mut self, slot_index: usize) {
        debug_assert!(slot_index < SYMBOL_CELL8_INTERFERENCE_SLOTS);
        let slot = slot_index as u8;
        let count = self.active_slot_count();

        if self.calibration.active_slots[..count].contains(&slot) {
            return;
        }

        if count < CELL8_ACTIVE_SLOT_CAPACITY {
            self.calibration.active_slots[count] = slot;
            self.calibration.active_slot_count = (count + 1) as u8;
            return;
        }

        let weakest_position = self.weakest_active_slot_position();
        let evicted_slot = usize::from(self.calibration.active_slots[weakest_position]);
        if evicted_slot != slot_index {
            self.interference[evicted_slot] = Interference8::default();
        }
        self.calibration.active_slots[weakest_position] = slot;
    }

    fn weakest_active_slot_position(&self) -> usize {
        let count = self.active_slot_count();
        let mut weakest_position = 0usize;
        let mut weakest_energy = u16::MAX;

        for position in 0..count {
            let slot_index = usize::from(self.calibration.active_slots[position]);
            let energy = self.interference[slot_index].energy;
            if energy < weakest_energy {
                weakest_energy = energy;
                weakest_position = position;
            }
        }

        weakest_position
    }

    fn top_two_active_slots(&self) -> (usize, usize) {
        let count = self.active_slot_count();
        let mut first: Option<usize> = None;
        let mut second: Option<usize> = None;

        for position in 0..count {
            let slot_index = usize::from(self.calibration.active_slots[position]);
            if slot_index >= SYMBOL_CELL8_INTERFERENCE_SLOTS {
                continue;
            }

            match first {
                None => first = Some(slot_index),
                Some(first_index)
                    if self.interference[slot_index].energy
                        > self.interference[first_index].energy =>
                {
                    second = first;
                    first = Some(slot_index);
                }
                _ => match second {
                    None if Some(slot_index) != first => {
                        second = Some(slot_index);
                    }
                    Some(second_index)
                        if Some(slot_index) != first
                            && self.interference[slot_index].energy
                                > self.interference[second_index].energy =>
                    {
                        second = Some(slot_index);
                    }
                    _ => {}
                },
            }
        }

        let first = first.unwrap_or(0);
        let second = second.unwrap_or(if first == 0 { 1 } else { 0 });
        (first, second)
    }

    fn active_slot_count(&self) -> usize {
        usize::from(
            self.calibration
                .active_slot_count
                .min(CELL8_ACTIVE_SLOT_CAPACITY as u8),
        )
    }

    fn persistence_score(&self, peak_slot: u16) -> u16 {
        if self.calibration.last_peak == peak_slot {
            255
        } else if compatible_peak(self.calibration.last_peak, peak_slot) {
            192
        } else if self.calibration.previous_peak == peak_slot {
            96
        } else {
            12
        }
    }

    fn is_limit_cycle(&self, peak_slot: u16) -> bool {
        self.calibration.previous_peak == peak_slot
            && self.calibration.last_peak != u16::MAX
            && self.calibration.last_peak != peak_slot
    }

    fn record_outcome(&mut self, outcome: PeakOutcome) {
        match outcome {
            PeakOutcome::Accepted => {
                self.calibration.accepted = self.calibration.accepted.saturating_add(1);
                self.calibration.vigilance = self.calibration.vigilance.saturating_sub(1);
            }
            PeakOutcome::Supported => {
                self.calibration.supported = self.calibration.supported.saturating_add(1);
            }
            PeakOutcome::Reflected => {
                self.calibration.reflected = self.calibration.reflected.saturating_add(1);
                self.calibration.vigilance = self.calibration.vigilance.saturating_add(2).min(255);
            }
            PeakOutcome::Vetoed => {
                self.calibration.vetoed = self.calibration.vetoed.saturating_add(1);
                self.calibration.vigilance = self.calibration.vigilance.saturating_add(3).min(255);
            }
            PeakOutcome::Spurious => {
                self.calibration.spurious = self.calibration.spurious.saturating_add(1);
                self.calibration.false_positive = self.calibration.false_positive.saturating_add(1);
                self.calibration.vigilance = self.calibration.vigilance.saturating_add(4).min(255);
            }
            PeakOutcome::LimitCycle => {
                self.calibration.limit_cycle = self.calibration.limit_cycle.saturating_add(1);
                self.calibration.vigilance = self.calibration.vigilance.saturating_add(4).min(255);
            }
            PeakOutcome::Unstable | PeakOutcome::NoPeak => {}
        }
    }

    fn update_peak_history(&mut self, peak_slot: u16, energy: u16, second_order: u16) {
        self.calibration.previous_peak = self.calibration.last_peak;
        self.calibration.last_peak = peak_slot;
        self.calibration.last_energy = energy;
        self.calibration.last_second_order = second_order;
    }
}

impl SymbolCell32 {
    #[must_use]
    pub fn new(id: u32, role: u8, seed: u64) -> Self {
        let mut projection = [ProjectionEntry16::default(); PROJECTION_ENTRIES];
        for (index, entry) in projection.iter_mut().enumerate() {
            let unit_a = unit_noise(seed, id as u64, index as u64, 0x5052_4F4A);
            let unit_b = unit_noise(seed, id as u64, index as u64, 0x4C41_4E45);
            entry.utf8_len = (index % 4 + 1) as u8;
            entry.byte_mix = [
                index as u8,
                (index.rotate_left(1) & 0xFF) as u8,
                (index.rotate_left(3) & 0xFF) as u8,
                (index ^ 0xA5) as u8,
            ];
            entry.lane = index as u16;
            entry.frequency_hint = ((unit_a * u16::MAX as f32) as u16) & 0x07FF;
            entry.amplitude = signed_unit_i8(unit_b);
            entry.phase = signed_unit_i8(unit_a);
            entry.damping = 240 + (index % 16) as u8;
            entry.role = role;
        }

        let mut modes = [Mode8::default(); SYMBOL_CELL32_MODES];
        for (index, mode) in modes.iter_mut().enumerate() {
            let unit_a = unit_noise(seed, id as u64, index as u64, 0x4D4F_4445);
            let unit_b = unit_noise(seed, id as u64, index as u64, 0x5349_4E43);
            let unit_c = unit_noise(seed, id as u64, index as u64, 0x434F_5343);
            mode.frequency_id = (index as u16).wrapping_mul(17) & 0x07FF;
            mode.sin_weight = signed_unit_i8(unit_b);
            mode.cos_weight = signed_unit_i8(unit_c);
            mode.amplitude = signed_unit_i8(unit_a).saturating_abs().max(1);
            mode.phase = signed_unit_i8(unit_a);
            mode.damping = 232 + (index % 24) as u8;
            mode.role = role;
        }

        let mut transitions = [Transition8::default(); SYMBOL_CELL32_TRANSITIONS];
        for (index, transition) in transitions.iter_mut().enumerate() {
            let previous_mode = (index * 3) % SYMBOL_CELL32_MODES;
            let current_mode = (index * 3 + 1) % SYMBOL_CELL32_MODES;
            let unit = unit_noise(seed, id as u64, index as u64, 0x5452_414E);
            transition.previous_mode = previous_mode as u16;
            transition.current_mode = current_mode as u16;
            transition.coupling = signed_unit_i8(unit);
            transition.phase_shift =
                signed_unit_i8(unit_noise(seed, id as u64, index as u64, 0x5048_5345));
            transition.damping = 236 + (index % 20) as u8;
            transition.role = role;
        }

        Self {
            header: SymbolHeader::new(id, role),
            projection,
            modes,
            transitions,
            interference: [Interference8::default(); SYMBOL_CELL32_INTERFERENCE_SLOTS],
            calibration: CalibrationStats::new(),
            scratch: [0; SYMBOL_CELL32_SCRATCH_BYTES],
        }
    }

    #[must_use]
    pub fn project_symbol(&self, symbol: char) -> SymbolProjection {
        let codepoint = symbol as u32;
        let mut bytes = [0; 4];
        let utf8_len = symbol.encode_utf8(&mut bytes).len() as u8;
        let lane = projection_lane(codepoint, bytes, utf8_len);
        let entry = self.projection[lane as usize % PROJECTION_ENTRIES];

        SymbolProjection {
            codepoint,
            lane,
            utf8_len,
            bytes,
            amplitude: entry.amplitude,
            phase: entry.phase,
        }
    }

    pub fn excite_symbol(
        &mut self,
        symbol: char,
        previous_symbol: Option<char>,
    ) -> SymbolExcitation {
        let projection = self.project_symbol(symbol);
        let previous_projection = previous_symbol.map(|previous| self.project_symbol(previous));
        self.calibration.seen = self.calibration.seen.saturating_add(1);

        let mut peak_slot = 0usize;
        let mut peak_energy = 0u16;

        for offset in 0..ACTIVE_EXCITATION_MODES {
            let mode_index = excitation_mode_index(projection, offset);
            let mode = self.modes[mode_index];
            let transition = previous_projection
                .map(|previous| self.transition_for(previous, projection, offset))
                .unwrap_or_default();
            let slot_index = (usize::from(projection.lane) + mode_index + offset)
                % SYMBOL_CELL32_INTERFERENCE_SLOTS;
            let slot = &mut self.interference[slot_index];
            apply_mode_to_slot(slot, mode, transition, projection);

            if slot.energy > peak_energy {
                peak_energy = slot.energy;
                peak_slot = slot_index;
            }
        }

        let coherence = f32::from(self.interference[peak_slot].coherence) / f32::from(u16::MAX);
        let accepted = peak_energy >= self.calibration.accept as u16
            && i16::from_ne_bytes(peak_energy.to_ne_bytes()) > self.calibration.veto;
        if accepted {
            self.calibration.accepted = self.calibration.accepted.saturating_add(1);
        }

        SymbolExcitation {
            projection,
            peak_slot: peak_slot as u16,
            peak_energy,
            coherence,
            accepted,
        }
    }

    #[must_use]
    fn transition_for(
        &self,
        previous: SymbolProjection,
        current: SymbolProjection,
        offset: usize,
    ) -> Transition8 {
        let index = (usize::from(previous.lane)
            .wrapping_mul(31)
            .wrapping_add(usize::from(current.lane))
            .wrapping_add(offset))
            % SYMBOL_CELL32_TRANSITIONS;
        self.transitions[index]
    }
}

fn apply_mode_to_slot(
    slot: &mut Interference8,
    mode: Mode8,
    transition: Transition8,
    projection: SymbolProjection,
) {
    let amplitude = i16::from(mode.amplitude.max(1));
    let transition_gain = i16::from(transition.coupling) / 8;
    let phase =
        i16::from(mode.phase) + i16::from(projection.phase) + i16::from(transition.phase_shift);
    let real = (i16::from(mode.cos_weight) + transition_gain) * amplitude / 64;
    let imag = (i16::from(mode.sin_weight) + phase / 16) * amplitude / 64;

    slot.real = decayed_i16(slot.real, real);
    slot.imag = decayed_i16(slot.imag, imag);

    let energy =
        u16::try_from(i32::from(slot.real).abs() + i32::from(slot.imag).abs()).unwrap_or(u16::MAX);
    slot.energy = slot.energy.saturating_add(energy / 2).max(energy);
    slot.coherence = slot
        .coherence
        .saturating_add((u16::from(projection.utf8_len) + u16::from(mode.damping)) / 4);
}

fn apply_mode_to_slot_cell8(
    slot: &mut Interference8,
    mode: Mode8,
    transition: Transition8,
    projection: SymbolProjection,
    carrier_phase: i8,
) {
    let amplitude = i16::from(mode.amplitude.saturating_abs())
        .max(8)
        .saturating_add(i16::from(projection.amplitude.saturating_abs()) / 8);
    let transition_gain = i16::from(transition.coupling) / 4;
    let phase = i16::from(mode.phase)
        + i16::from(projection.phase)
        + i16::from(transition.phase_shift)
        + i16::from(carrier_phase);
    let real = (i16::from(mode.cos_weight) + transition_gain + i16::from(carrier_phase) / 8)
        * amplitude
        / 48;
    let imag = (i16::from(mode.sin_weight) + phase / 12) * amplitude / 48;

    slot.real = decayed_i16(slot.real, real);
    slot.imag = decayed_i16(slot.imag, imag);

    let energy =
        u16::try_from(i32::from(slot.real).abs() + i32::from(slot.imag).abs()).unwrap_or(u16::MAX);
    slot.energy = slot.energy.saturating_add(energy / 2).max(energy);
    let transition_boost = u16::try_from(i16::from(transition.coupling).max(0)).unwrap_or(0) / 4;
    slot.coherence = slot
        .coherence
        .saturating_add(u16::from(mode.damping) / 3 + transition_boost + 8)
        .min(255);
}

fn decayed_i16(current: i16, delta: i16) -> i16 {
    let next = i32::from(current) * 7 / 8 + i32::from(delta);
    next.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

fn decay_interference_slot(slot: &mut Interference8, decay: u32) {
    slot.real = ((i32::from(slot.real) * i32::try_from(decay).unwrap_or(0)) / 1024)
        .clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
    slot.imag = ((i32::from(slot.imag) * i32::try_from(decay).unwrap_or(0)) / 1024)
        .clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
    slot.energy =
        u16::try_from(i32::from(slot.real).abs() + i32::from(slot.imag).abs()).unwrap_or(u16::MAX);
    slot.coherence = ((u32::from(slot.coherence) * decay) / 1024) as u16;
}

fn excitation_mode_index(projection: SymbolProjection, offset: usize) -> usize {
    (usize::from(projection.lane) * 8
        + usize::from(projection.utf8_len)
        + usize::from(projection.bytes[offset % projection.utf8_len as usize])
        + offset * 17)
        % SYMBOL_CELL32_MODES
}

fn cell8_base_slot(
    projection: SymbolProjection,
    previous_peak: Option<u16>,
    carrier_phase: i8,
) -> usize {
    let previous = usize::from(previous_peak.unwrap_or(0));
    let carrier = usize::from(carrier_phase.unsigned_abs());
    (usize::from(projection.lane) * 2 + previous % 2 + carrier % 17)
        % SYMBOL_CELL8_INTERFERENCE_SLOTS
}

fn cell8_mode_index(
    projection: SymbolProjection,
    previous_peak: Option<u16>,
    carrier_phase: i8,
    offset: usize,
) -> usize {
    let previous = usize::from(previous_peak.unwrap_or(0));
    let byte = usize::from(projection.bytes[offset % projection.utf8_len as usize]);
    (usize::from(projection.lane) * 8
        + usize::from(projection.utf8_len)
        + byte
        + previous * 3
        + usize::from(carrier_phase.unsigned_abs())
        + offset * 17)
        % SYMBOL_CELL8_MODES
}

fn transition_support_score(transition: Transition8) -> u16 {
    let coupling = u16::try_from(i16::from(transition.coupling).max(0)).unwrap_or(0);
    (coupling / 2 + u16::from(transition.damping) / 8).min(96)
}

fn second_order_score(peak: Interference8, projection: SymbolProjection, carrier_phase: i8) -> u16 {
    let real = i32::from(peak.real);
    let imag = i32::from(peak.imag);
    let doubled_real = real * real - imag * imag;
    let doubled_imag = 2 * real * imag;
    let harmonic =
        u16::try_from((doubled_real.abs() + doubled_imag.abs()) / 64).unwrap_or(u16::MAX);
    let phase_agreement =
        128u16.saturating_sub(phase_distance_i8(projection.phase, carrier_phase).min(128));
    harmonic
        .saturating_add(peak.energy / 2)
        .saturating_add(peak.coherence / 4)
        .saturating_add(phase_agreement / 4)
        .min(255)
}

fn veto_score(peak: Interference8, separation: u16, calibration: SymbolCell8Calibration) -> u16 {
    if peak.energy >= calibration.accept && peak.coherence < calibration.coherence_min {
        240
    } else if peak.energy >= calibration.accept && separation < calibration.margin_min {
        224
    } else {
        0
    }
}

fn incoming_mismatch_energy(
    incoming: &[SymbolCell8Advice],
    carrier_phase: i8,
    local_role: u8,
) -> u16 {
    incoming
        .iter()
        .map(|message| {
            let phase = phase_distance_i8(message.phase, carrier_phase);
            let role = if message.role == local_role { 0 } else { 32 };
            ((u32::from(message.energy) * u32::from(phase)) / 128)
                .saturating_add(role)
                .min(u32::from(u16::MAX)) as u16
        })
        .max()
        .unwrap_or(0)
}

fn classify_peak(
    score: StablePeakScore,
    calibration: SymbolCell8Calibration,
    limit_cycle: bool,
    reflection_energy: u16,
) -> PeakOutcome {
    if score.energy < calibration.excite {
        return if reflection_energy >= calibration.reflection_min {
            PeakOutcome::Reflected
        } else {
            PeakOutcome::NoPeak
        };
    }

    if limit_cycle && score.energy >= calibration.accept {
        return PeakOutcome::LimitCycle;
    }

    if score.energy >= calibration.accept
        && (score.coherence < calibration.coherence_min
            || score.separation < calibration.margin_min
            || score.second_order < calibration.second_order_min)
    {
        return PeakOutcome::Spurious;
    }

    if score.veto >= calibration.veto {
        return PeakOutcome::Vetoed;
    }

    if score.stable_score >= calibration.accept
        && score.separation >= calibration.margin_min
        && score.coherence >= calibration.coherence_min
        && score.second_order >= calibration.second_order_min
    {
        return PeakOutcome::Accepted;
    }

    if reflection_energy >= calibration.reflection_min {
        PeakOutcome::Reflected
    } else if score.energy >= calibration.excite {
        PeakOutcome::Supported
    } else {
        PeakOutcome::Unstable
    }
}

fn advice_for_outcome(
    outcome: PeakOutcome,
    peak_slot: u16,
    peak: Interference8,
    projection: SymbolProjection,
    role: u8,
) -> Option<SymbolCell8Advice> {
    matches!(outcome, PeakOutcome::Accepted | PeakOutcome::Supported).then_some(SymbolCell8Advice {
        peak_slot,
        energy: peak.energy,
        coherence: peak.coherence,
        phase: projection.phase,
        role,
    })
}

fn phase_distance_i8(a: i8, b: i8) -> u16 {
    let direct = (i16::from(a) - i16::from(b)).unsigned_abs();
    direct.min(256u16.saturating_sub(direct))
}

fn compatible_peak(previous: u16, current: u16) -> bool {
    previous != u16::MAX && previous.abs_diff(current) <= 1
}

fn projection_lane(codepoint: u32, bytes: [u8; 4], utf8_len: u8) -> u16 {
    projection_lane_for_entries(codepoint, bytes, utf8_len, PROJECTION_ENTRIES)
}

fn projection_lane_for_entries(
    codepoint: u32,
    bytes: [u8; 4],
    utf8_len: u8,
    entries: usize,
) -> u16 {
    let mut hash = codepoint ^ (u32::from(utf8_len) << 24);
    for byte in bytes {
        hash = hash.rotate_left(5) ^ u32::from(byte);
        hash = hash.wrapping_mul(0x045D_9F3B);
    }
    (hash as usize % entries) as u16
}

fn signed_unit_i8(unit: f32) -> i8 {
    let scaled = (unit.mul_add(255.0, -128.0)).round();
    scaled.clamp(f32::from(i8::MIN), f32::from(i8::MAX)) as i8
}

const _: [(); CELL32_BYTES] = [(); std::mem::size_of::<SymbolCell32>()];
const _: [(); 256] = [(); std::mem::size_of::<SymbolHeader>()];
const _: [(); 16] = [(); std::mem::size_of::<ProjectionEntry16>()];
const _: [(); 8] = [(); std::mem::size_of::<Mode8>()];
const _: [(); 8] = [(); std::mem::size_of::<Transition8>()];
const _: [(); 8] = [(); std::mem::size_of::<Interference8>()];
const _: [(); 8] = [(); std::mem::size_of::<SymbolCell8Advice>()];
const _: [(); 128] = [(); std::mem::size_of::<SymbolCell8Header>()];
const _: [(); SYMBOL_CELL8_BYTES] = [(); std::mem::size_of::<SymbolCell8>()];
const _: [(); SYMBOL_CELL8_CALIBRATION_STATS_BYTES] =
    [(); std::mem::size_of::<SymbolCell8Calibration>()];
const _: [(); SYMBOL_CELL32_CALIBRATION_STATS_BYTES] =
    [(); std::mem::size_of::<CalibrationStats>()];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_cell_layout_matches_architecture() {
        assert_eq!(std::mem::size_of::<SymbolCell32>(), CELL32_BYTES);
        assert_eq!(std::mem::size_of::<SymbolCell8>(), SYMBOL_CELL8_BYTES);
        assert_eq!(std::mem::size_of::<SymbolHeader>(), 256);
        assert_eq!(std::mem::size_of::<SymbolCell8Header>(), 128);
        assert_eq!(
            std::mem::size_of::<ProjectionEntry16>() * PROJECTION_ENTRIES,
            4_096
        );
        assert_eq!(
            std::mem::size_of::<ProjectionEntry16>() * SYMBOL_CELL8_PROJECTION_LANES,
            1_024
        );
        assert_eq!(std::mem::size_of::<Mode8>() * SYMBOL_CELL32_MODES, 16_384);
        assert_eq!(std::mem::size_of::<Mode8>() * SYMBOL_CELL8_MODES, 4_096);
        assert_eq!(
            std::mem::size_of::<Transition8>() * SYMBOL_CELL32_TRANSITIONS,
            4_096
        );
        assert_eq!(
            std::mem::size_of::<Transition8>() * SYMBOL_CELL8_TRANSITIONS,
            1_024
        );
        assert_eq!(
            std::mem::size_of::<Interference8>() * SYMBOL_CELL32_INTERFERENCE_SLOTS,
            4_096
        );
        assert_eq!(
            std::mem::size_of::<Interference8>() * SYMBOL_CELL8_INTERFERENCE_SLOTS,
            1_024
        );
        assert_eq!(
            std::mem::size_of::<CalibrationStats>(),
            SYMBOL_CELL32_CALIBRATION_STATS_BYTES
        );
        assert_eq!(
            std::mem::size_of::<SymbolCell8Calibration>(),
            SYMBOL_CELL8_CALIBRATION_STATS_BYTES
        );
        assert_eq!(std::mem::size_of::<SymbolCell8Advice>(), 8);
    }

    #[test]
    fn symbol_excitation_updates_interference_and_stats() {
        let mut cell = SymbolCell32::new(7, 1, 42);

        let first = cell.excite_symbol('Н', None);
        let second = cell.excite_symbol('А', Some('Н'));

        assert_eq!(cell.calibration.seen, 2);
        assert!(first.peak_energy > 0);
        assert!(second.peak_energy > 0);
        assert_ne!(first.projection.lane, second.projection.lane);
    }

    #[test]
    fn symbol_cell8_tick_is_deterministic_for_fresh_cells() {
        let mut first = SymbolCell8::new(3, 1, 99);
        let mut second = SymbolCell8::new(3, 1, 99);

        let first_tick = first.tick_symbol('A');
        let second_tick = second.tick_symbol('A');

        assert_eq!(first_tick.peak_slot, second_tick.peak_slot);
        assert_eq!(first_tick.outcome, second_tick.outcome);
        assert_eq!(first_tick.score, second_tick.score);
    }

    #[test]
    fn symbol_cell8_requires_persistence_before_accepting() {
        let mut cell = SymbolCell8::new(5, 2, 17);

        let first = cell.tick_symbol('N');
        let second = cell.tick_symbol('N');

        assert_ne!(first.outcome, PeakOutcome::Accepted);
        assert_eq!(second.outcome, PeakOutcome::Accepted);
        assert!(second.score.persistence > first.score.persistence);
        assert_eq!(cell.calibration.accepted, 1);
    }

    #[test]
    fn symbol_cell8_projection_ablation_does_not_erase_wave_support() {
        let mut intact = SymbolCell8::new(6, 2, 19);
        let intact_first = intact.tick_symbol('A');
        let intact_second = intact.tick_symbol('A');

        let mut ablated = SymbolCell8::new(6, 2, 19);
        let lane = ablated.project_symbol('A').lane as usize;
        ablated.projection[lane] = ProjectionEntry16::default();
        let ablated_first = ablated.tick_symbol('A');
        let ablated_second = ablated.tick_symbol('A');

        assert!(intact_first.score.energy > 0);
        assert!(intact_second.score.energy > 0);
        assert!(ablated_first.score.energy > 0);
        assert!(ablated_second.score.energy > 0);
        assert_ne!(ablated_second.outcome, PeakOutcome::NoPeak);
        assert!(ablated_second.peak_slot.abs_diff(intact_second.peak_slot) <= 1);
    }

    #[test]
    fn symbol_cell8_keeps_active_slot_set_bounded() {
        let mut cell = SymbolCell8::new(7, 2, 23);
        let symbols = ['N', 'A', 'D', 'W', '0', '1', 'x', ' ', 'Z', 'Q'];

        for index in 0..128 {
            let _ = cell.tick_symbol(symbols[index % symbols.len()]);
            assert!(usize::from(cell.calibration.active_slot_count) <= CELL8_ACTIVE_SLOT_CAPACITY);
        }
    }

    #[test]
    fn symbol_cell8_classifies_high_energy_low_coherence_as_spurious() {
        let mut cell = SymbolCell8::new(8, 2, 31);
        cell.calibration.coherence_min = 255;
        cell.calibration.accept = 1;

        let tick = cell.tick_symbol('S');

        assert_eq!(tick.outcome, PeakOutcome::Spurious);
        assert_eq!(cell.calibration.spurious, 1);
        assert_eq!(cell.calibration.false_positive, 1);
        assert!(cell.calibration.vigilance > 48);
    }

    #[test]
    fn symbol_cell8_accepted_repetition_relaxes_vigilance() {
        let mut cell = SymbolCell8::new(9, 2, 41);
        cell.calibration.vigilance = 80;

        let _ = cell.tick_symbol('V');
        let accepted = cell.tick_symbol('V');

        assert_eq!(accepted.outcome, PeakOutcome::Accepted);
        assert!(cell.calibration.vigilance < 80);
    }

    #[test]
    fn symbol_cell8_detects_limit_cycle_history() {
        let mut probe = SymbolCell8::new(13, 3, 55);
        let expected = probe.tick_symbol_with_context('L', Some(0), 0, &[]);

        let mut cell = SymbolCell8::new(13, 3, 55);
        cell.calibration.accept = 1;
        cell.calibration.previous_peak = expected.peak_slot;
        cell.calibration.last_peak =
            (expected.peak_slot + 1) % SYMBOL_CELL8_INTERFERENCE_SLOTS as u16;

        let tick = cell.tick_symbol_with_context('L', Some(0), 0, &[]);

        assert_eq!(tick.peak_slot, expected.peak_slot);
        assert_eq!(tick.outcome, PeakOutcome::LimitCycle);
        assert_eq!(cell.calibration.limit_cycle, 1);
    }

    #[test]
    fn symbol_cell8_emits_bounded_reflection_for_mismatch() {
        let mut cell = SymbolCell8::new(21, 1, 77);
        let incoming = [SymbolCell8Advice {
            peak_slot: 7,
            energy: 240,
            coherence: 10,
            phase: 127,
            role: 9,
        }];

        let tick = cell.tick_symbol_with_context('R', None, -120, &incoming);

        assert_eq!(tick.outcome, PeakOutcome::Reflected);
        assert!(tick.reflection.is_some());
        assert_eq!(cell.calibration.reflected, 1);
    }
}
