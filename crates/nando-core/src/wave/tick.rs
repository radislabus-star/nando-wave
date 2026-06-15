use std::f32::consts::TAU;

use super::{
    CarrierWave, PHASE_SLOTS, STAGE2_ORGAN_CELLS, STAGE2_TOP_K, SpectrumSnapshot, Stage2Organ,
    Stage2Tick, TickTrace, WaveBus, insert_top_index,
};

/// Trace-only result for hot loops that do not need a serialized snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct Stage2TraceTick {
    pub carrier: CarrierWave,
    pub trace: TickTrace,
}

/// Trace result with the full bus spectrum for research readouts.
#[derive(Debug, Clone, PartialEq)]
pub struct Stage2BusTraceTick {
    pub carrier: CarrierWave,
    pub bus: WaveBus,
    pub trace: TickTrace,
}

/// Precomputed byte-to-phase table for hot training loops.
#[derive(Debug, Clone)]
pub struct BytePhaseLut {
    phases: Vec<[f32; PHASE_SLOTS]>,
    phase_sin: Vec<[f32; PHASE_SLOTS]>,
    phase_cos: Vec<[f32; PHASE_SLOTS]>,
}

impl Default for BytePhaseLut {
    fn default() -> Self {
        Self::new()
    }
}

impl BytePhaseLut {
    /// Build all 256 byte phase vectors once.
    #[must_use]
    pub fn new() -> Self {
        Self {
            phases: (0..=u8::MAX).map(encode_byte_phases).collect(),
            phase_sin: (0..=u8::MAX)
                .map(|byte| encode_byte_phases(byte).map(|phase| phase.sin()))
                .collect(),
            phase_cos: (0..=u8::MAX)
                .map(|byte| encode_byte_phases(byte).map(|phase| phase.cos()))
                .collect(),
        }
    }

    /// Borrow the phase vector for one input byte.
    #[must_use]
    pub fn phases(&self, input_byte: u8) -> &[f32; PHASE_SLOTS] {
        &self.phases[input_byte as usize]
    }

    /// Borrow the sine vector for one input byte.
    #[must_use]
    pub fn phase_sin(&self, input_byte: u8) -> &[f32; PHASE_SLOTS] {
        &self.phase_sin[input_byte as usize]
    }

    /// Borrow the cosine vector for one input byte.
    #[must_use]
    pub fn phase_cos(&self, input_byte: u8) -> &[f32; PHASE_SLOTS] {
        &self.phase_cos[input_byte as usize]
    }
}

/// Encode a byte as a deterministic phase vector.
#[must_use]
pub fn encode_byte_phases(input_byte: u8) -> [f32; PHASE_SLOTS] {
    let base = input_byte as f32 / 256.0;
    std::array::from_fn(|slot| {
        let harmonic = (slot % 32 + 1) as f32;
        let octave = (slot / 32) as f32 * 0.03125;
        (TAU * (base * harmonic + octave)).rem_euclid(TAU)
    })
}

/// Run one deterministic Stage 2 wave tick.
#[must_use]
pub fn run_stage2_tick(seed: u64, input_byte: u8) -> Stage2Tick {
    let carrier = CarrierWave::from_seed(seed, input_byte);
    run_stage2_tick_with_carrier(seed, input_byte, carrier, None)
}

/// Run one deterministic Stage 2 trace-only tick.
#[must_use]
pub fn run_stage2_trace_tick(seed: u64, input_byte: u8) -> Stage2TraceTick {
    let carrier = CarrierWave::from_seed(seed, input_byte);
    let organ = Stage2Organ::new(seed);
    run_stage2_trace_with_organ_carrier(&organ, input_byte, carrier, None)
}

/// Run one deterministic Stage 2 wave tick with an optional disabled cell.
#[must_use]
pub fn run_stage2_tick_with_disabled(
    seed: u64,
    input_byte: u8,
    disabled_cell_id: Option<u32>,
) -> Stage2Tick {
    let carrier = CarrierWave::from_seed(seed, input_byte);
    run_stage2_tick_with_carrier(seed, input_byte, carrier, disabled_cell_id)
}

/// Run one Stage 2 wave tick with an explicit CarrierWave.
///
/// This is used by carrier-control experiments: correct carrier, no carrier,
/// wrong carrier, and corrupted carrier must all exercise the same organism.
#[must_use]
pub fn run_stage2_tick_with_carrier(
    seed: u64,
    input_byte: u8,
    carrier: CarrierWave,
    disabled_cell_id: Option<u32>,
) -> Stage2Tick {
    run_stage2_tick_with_state(
        seed,
        input_byte,
        carrier,
        disabled_cell_id,
        &[0.0; STAGE2_ORGAN_CELLS],
    )
}

/// Run one Stage 2 wave tick through a precomputed organism.
#[must_use]
pub fn run_stage2_tick_with_organ_carrier(
    organ: &Stage2Organ,
    input_byte: u8,
    carrier: CarrierWave,
    disabled_cell_id: Option<u32>,
) -> Stage2Tick {
    run_stage2_tick_with_organ_state(
        organ,
        input_byte,
        carrier,
        disabled_cell_id,
        &[0.0; STAGE2_ORGAN_CELLS],
    )
}

/// Run one full tick through a precomputed organism and byte phase LUT.
#[must_use]
pub fn run_stage2_tick_with_organ_lut_carrier(
    organ: &Stage2Organ,
    lut: &BytePhaseLut,
    input_byte: u8,
    carrier: CarrierWave,
    disabled_cell_id: Option<u32>,
) -> Stage2Tick {
    run_stage2_tick_with_organ_lut_state(
        organ,
        lut,
        input_byte,
        carrier,
        disabled_cell_id,
        &[0.0; STAGE2_ORGAN_CELLS],
    )
}

/// Run one trace-only tick through a precomputed organism.
#[must_use]
pub fn run_stage2_trace_with_organ_carrier(
    organ: &Stage2Organ,
    input_byte: u8,
    carrier: CarrierWave,
    disabled_cell_id: Option<u32>,
) -> Stage2TraceTick {
    run_stage2_trace_with_organ_state(
        organ,
        input_byte,
        carrier,
        disabled_cell_id,
        &[0.0; STAGE2_ORGAN_CELLS],
    )
}

/// Run one tick through a precomputed organism and expose the full bus spectrum.
#[must_use]
pub fn run_stage2_bus_trace_with_organ_carrier(
    organ: &Stage2Organ,
    input_byte: u8,
    carrier: CarrierWave,
    disabled_cell_id: Option<u32>,
) -> Stage2BusTraceTick {
    run_stage2_bus_trace_with_organ_state(
        organ,
        input_byte,
        carrier,
        disabled_cell_id,
        &[0.0; STAGE2_ORGAN_CELLS],
    )
}

/// Run one trace-only tick through a precomputed organism and byte phase LUT.
#[must_use]
pub fn run_stage2_trace_with_organ_lut_carrier(
    organ: &Stage2Organ,
    lut: &BytePhaseLut,
    input_byte: u8,
    carrier: CarrierWave,
    disabled_cell_id: Option<u32>,
) -> Stage2TraceTick {
    run_stage2_trace_with_organ_lut_state(
        organ,
        lut,
        input_byte,
        carrier,
        disabled_cell_id,
        &[0.0; STAGE2_ORGAN_CELLS],
    )
}

pub(crate) fn run_stage2_tick_with_state(
    seed: u64,
    input_byte: u8,
    carrier: CarrierWave,
    disabled_cell_id: Option<u32>,
    cell_coupling: &[f32; STAGE2_ORGAN_CELLS],
) -> Stage2Tick {
    let organ = Stage2Organ::new(seed);
    run_stage2_tick_with_organ_state(&organ, input_byte, carrier, disabled_cell_id, cell_coupling)
}

pub(crate) fn run_stage2_tick_with_organ_state(
    organ: &Stage2Organ,
    input_byte: u8,
    carrier: CarrierWave,
    disabled_cell_id: Option<u32>,
    cell_coupling: &[f32; STAGE2_ORGAN_CELLS],
) -> Stage2Tick {
    let (bus, trace) =
        build_bus_and_trace(organ, input_byte, carrier, disabled_cell_id, cell_coupling);
    let snapshot = SpectrumSnapshot::from_bus(organ.seed, input_byte, carrier, &bus, trace);

    Stage2Tick {
        carrier,
        trace,
        snapshot,
    }
}

pub(crate) fn run_stage2_tick_with_organ_lut_state(
    organ: &Stage2Organ,
    lut: &BytePhaseLut,
    input_byte: u8,
    carrier: CarrierWave,
    disabled_cell_id: Option<u32>,
    cell_coupling: &[f32; STAGE2_ORGAN_CELLS],
) -> Stage2Tick {
    let (bus, trace) = build_bus_and_trace_with_phase_trig(
        organ,
        lut.phase_sin(input_byte),
        lut.phase_cos(input_byte),
        input_byte,
        carrier,
        disabled_cell_id,
        cell_coupling,
    );
    let snapshot = SpectrumSnapshot::from_bus(organ.seed, input_byte, carrier, &bus, trace);

    Stage2Tick {
        carrier,
        trace,
        snapshot,
    }
}

pub(crate) fn run_stage2_trace_with_organ_state(
    organ: &Stage2Organ,
    input_byte: u8,
    carrier: CarrierWave,
    disabled_cell_id: Option<u32>,
    cell_coupling: &[f32; STAGE2_ORGAN_CELLS],
) -> Stage2TraceTick {
    let (_, trace) =
        build_bus_and_trace(organ, input_byte, carrier, disabled_cell_id, cell_coupling);

    Stage2TraceTick { carrier, trace }
}

pub(crate) fn run_stage2_bus_trace_with_organ_state(
    organ: &Stage2Organ,
    input_byte: u8,
    carrier: CarrierWave,
    disabled_cell_id: Option<u32>,
    cell_coupling: &[f32; STAGE2_ORGAN_CELLS],
) -> Stage2BusTraceTick {
    let (bus, trace) =
        build_bus_and_trace(organ, input_byte, carrier, disabled_cell_id, cell_coupling);

    Stage2BusTraceTick {
        carrier,
        bus,
        trace,
    }
}

pub(crate) fn run_stage2_trace_with_organ_lut_state(
    organ: &Stage2Organ,
    lut: &BytePhaseLut,
    input_byte: u8,
    carrier: CarrierWave,
    disabled_cell_id: Option<u32>,
    cell_coupling: &[f32; STAGE2_ORGAN_CELLS],
) -> Stage2TraceTick {
    let (_, trace) = build_bus_and_trace_with_phase_trig(
        organ,
        lut.phase_sin(input_byte),
        lut.phase_cos(input_byte),
        input_byte,
        carrier,
        disabled_cell_id,
        cell_coupling,
    );

    Stage2TraceTick { carrier, trace }
}

fn build_bus_and_trace(
    organ: &Stage2Organ,
    input_byte: u8,
    carrier: CarrierWave,
    disabled_cell_id: Option<u32>,
    cell_coupling: &[f32; STAGE2_ORGAN_CELLS],
) -> (WaveBus, TickTrace) {
    let input_phases = encode_byte_phases(input_byte);
    build_bus_and_trace_with_phases(
        organ,
        &input_phases,
        input_byte,
        carrier,
        disabled_cell_id,
        cell_coupling,
    )
}

fn build_bus_and_trace_with_phases(
    organ: &Stage2Organ,
    input_phases: &[f32; PHASE_SLOTS],
    input_byte: u8,
    carrier: CarrierWave,
    disabled_cell_id: Option<u32>,
    cell_coupling: &[f32; STAGE2_ORGAN_CELLS],
) -> (WaveBus, TickTrace) {
    let mut input_sin = [0.0; PHASE_SLOTS];
    let mut input_cos = [0.0; PHASE_SLOTS];
    for ((sin_value, cos_value), input_phase) in input_sin
        .iter_mut()
        .zip(input_cos.iter_mut())
        .zip(input_phases.iter())
    {
        *sin_value = input_phase.sin();
        *cos_value = input_phase.cos();
    }

    build_bus_and_trace_with_phase_trig(
        organ,
        &input_sin,
        &input_cos,
        input_byte,
        carrier,
        disabled_cell_id,
        cell_coupling,
    )
}

fn build_bus_and_trace_with_phase_trig(
    organ: &Stage2Organ,
    input_sin: &[f32; PHASE_SLOTS],
    input_cos: &[f32; PHASE_SLOTS],
    input_byte: u8,
    carrier: CarrierWave,
    disabled_cell_id: Option<u32>,
    cell_coupling: &[f32; STAGE2_ORGAN_CELLS],
) -> (WaveBus, TickTrace) {
    let (bus, active_scores, cells_scanned, active_count, active_cell_ids) = build_bus_and_top(
        organ,
        input_sin,
        input_cos,
        carrier,
        disabled_cell_id,
        cell_coupling,
    );

    let trace = TickTrace {
        seed: organ.seed,
        input_byte,
        cells_scanned,
        active_count,
        active_cell_ids,
        top_resonance: active_scores[0],
        coherence: bus.coherence,
        spectral_entropy: bus.spectral_entropy,
        center_phase: bus.center_phase,
        center_magnitude: bus.center_magnitude,
    };

    (bus, trace)
}

fn build_bus_and_top(
    organ: &Stage2Organ,
    input_sin: &[f32; PHASE_SLOTS],
    input_cos: &[f32; PHASE_SLOTS],
    carrier: CarrierWave,
    disabled_cell_id: Option<u32>,
    cell_coupling: &[f32; STAGE2_ORGAN_CELLS],
) -> (
    WaveBus,
    [f32; STAGE2_TOP_K],
    usize,
    usize,
    [u32; STAGE2_TOP_K],
) {
    let mut active_indices = [0; STAGE2_TOP_K];
    let mut active_scores = [f32::NEG_INFINITY; STAGE2_TOP_K];
    let mut cells_scanned = 0;
    let mut input_carrier_sin = [0.0; PHASE_SLOTS];
    let mut input_carrier_cos = [0.0; PHASE_SLOTS];
    let carrier_sin = carrier.phase.sin();
    let carrier_cos = carrier.phase.cos();

    for (((carrier_sin_slot, carrier_cos_slot), input_sin), input_cos) in input_carrier_sin
        .iter_mut()
        .zip(input_carrier_cos.iter_mut())
        .zip(input_sin.iter())
        .zip(input_cos.iter())
    {
        *carrier_sin_slot = input_sin.mul_add(carrier_cos, *input_cos * carrier_sin);
        *carrier_cos_slot = input_cos.mul_add(carrier_cos, -(*input_sin * carrier_sin));
    }

    for (index, cell) in organ.cells().iter().enumerate() {
        if disabled_cell_id == Some(cell.id) {
            continue;
        }
        cells_scanned += 1;
        let resonance = cell.resonance_score_with_carrier_trig(
            &input_carrier_sin,
            &input_carrier_cos,
            carrier.envelope(),
        );
        let coupling = cell_coupling
            .get(cell.id as usize)
            .copied()
            .unwrap_or_default();
        let coupled_score = resonance.abs() * (1.0 + coupling * 0.04);
        insert_top_index(
            index,
            coupled_score,
            &mut active_indices,
            &mut active_scores,
        );
    }

    let mut bus = WaveBus::default();
    let mut active_cell_ids = [0; STAGE2_TOP_K];
    let active_count = active_scores
        .iter()
        .filter(|score| **score > f32::NEG_INFINITY)
        .count();

    for (position, index) in active_indices
        .iter()
        .copied()
        .take(active_count)
        .enumerate()
    {
        let cell = &organ.cells()[index];
        active_cell_ids[position] = cell.id;
        bus.add_cell(cell, carrier);
    }

    bus.finish_metrics();

    (
        bus,
        active_scores,
        cells_scanned,
        active_count,
        active_cell_ids,
    )
}
