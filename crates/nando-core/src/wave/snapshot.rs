use std::fmt;

use super::{
    CarrierWave, PHASE_SLOTS, SNAPSHOT_TOP_SLOTS, SNAPSHOT_V1_BYTES, STAGE2_TOP_K, WaveBus,
    insert_top_slot,
};

const SNAPSHOT_MAGIC: [u8; 4] = *b"NWV1";

/// Trace for the first deterministic wave tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TickTrace {
    pub seed: u64,
    pub input_byte: u8,
    pub cells_scanned: usize,
    pub active_count: usize,
    pub active_cell_ids: [u32; STAGE2_TOP_K],
    pub top_resonance: f32,
    pub coherence: f32,
    pub spectral_entropy: f32,
    pub center_phase: f32,
    pub center_magnitude: f32,
}

/// Compact runtime-state snapshot for Stage 2.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpectrumSnapshot {
    pub version: u16,
    pub seed: u64,
    pub input_byte: u8,
    pub carrier: CarrierWave,
    pub coherence: f32,
    pub spectral_entropy: f32,
    pub center_phase: f32,
    pub center_magnitude: f32,
    pub top_slots: [u16; SNAPSHOT_TOP_SLOTS],
    pub top_phases: [f32; SNAPSHOT_TOP_SLOTS],
    pub top_amplitudes: [f32; SNAPSHOT_TOP_SLOTS],
    pub active_cell_ids: [u32; STAGE2_TOP_K],
}

impl SpectrumSnapshot {
    /// Build a compact snapshot from a finalized bus and trace.
    #[must_use]
    pub fn from_bus(
        seed: u64,
        input_byte: u8,
        carrier: CarrierWave,
        bus: &WaveBus,
        trace: TickTrace,
    ) -> Self {
        let mut top_slots = [0; SNAPSHOT_TOP_SLOTS];
        let mut top_scores = [f32::NEG_INFINITY; SNAPSHOT_TOP_SLOTS];

        for (slot, amplitude) in bus.amplitude_sum.iter().enumerate() {
            insert_top_slot(slot as u16, *amplitude, &mut top_slots, &mut top_scores);
        }

        let mut top_phases = [0.0; SNAPSHOT_TOP_SLOTS];
        let mut top_amplitudes = [0.0; SNAPSHOT_TOP_SLOTS];

        for ((slot, phase), amplitude) in top_slots
            .iter()
            .zip(top_phases.iter_mut())
            .zip(top_amplitudes.iter_mut())
        {
            let slot_index = usize::from(*slot);
            *phase = bus.phase_sum[slot_index];
            *amplitude = bus.amplitude_sum[slot_index];
        }

        Self {
            version: 1,
            seed,
            input_byte,
            carrier,
            coherence: bus.coherence,
            spectral_entropy: bus.spectral_entropy,
            center_phase: bus.center_phase,
            center_magnitude: bus.center_magnitude,
            top_slots,
            top_phases,
            top_amplitudes,
            active_cell_ids: trace.active_cell_ids,
        }
    }

    /// Serialize this snapshot to the stable Stage 2 binary format.
    #[must_use]
    pub fn to_bytes(self) -> [u8; SNAPSHOT_V1_BYTES] {
        let mut writer = SnapshotWriter::default();

        writer.write_bytes(&SNAPSHOT_MAGIC);
        writer.write_u16(self.version);
        writer.write_u16(SNAPSHOT_TOP_SLOTS as u16);
        writer.write_u16(STAGE2_TOP_K as u16);
        writer.write_u16(PHASE_SLOTS as u16);
        writer.write_u64(self.seed);
        writer.write_u8(self.input_byte);
        writer.write_u8(0);
        writer.write_u16(0);
        writer.write_f32(self.carrier.phase);
        writer.write_f32(self.carrier.amplitude);
        writer.write_f32(self.carrier.frequency);
        writer.write_f32(self.carrier.boundary);
        writer.write_f32(self.coherence);
        writer.write_f32(self.spectral_entropy);
        writer.write_f32(self.center_phase);
        writer.write_f32(self.center_magnitude);

        for slot in self.top_slots {
            writer.write_u16(slot);
        }
        for phase in self.top_phases {
            writer.write_f32(phase);
        }
        for amplitude in self.top_amplitudes {
            writer.write_f32(amplitude);
        }
        for cell_id in self.active_cell_ids {
            writer.write_u32(cell_id);
        }

        writer.finish()
    }

    /// Parse a snapshot from the stable Stage 2 binary format.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SnapshotParseError> {
        let mut reader = SnapshotReader::new(bytes)?;
        let magic = reader.read_bytes::<4>()?;
        if magic != SNAPSHOT_MAGIC {
            return Err(SnapshotParseError::BadMagic);
        }

        let version = reader.read_u16()?;
        if version != 1 {
            return Err(SnapshotParseError::UnsupportedVersion(version));
        }

        let top_slots_len = reader.read_u16()?;
        let top_k_len = reader.read_u16()?;
        let phase_slots_len = reader.read_u16()?;

        if top_slots_len != SNAPSHOT_TOP_SLOTS as u16
            || top_k_len != STAGE2_TOP_K as u16
            || phase_slots_len != PHASE_SLOTS as u16
        {
            return Err(SnapshotParseError::BadShape);
        }

        let seed = reader.read_u64()?;
        let input_byte = reader.read_u8()?;
        let _reserved_a = reader.read_u8()?;
        let _reserved_b = reader.read_u16()?;
        let carrier = CarrierWave {
            phase: reader.read_f32()?,
            amplitude: reader.read_f32()?,
            frequency: reader.read_f32()?,
            boundary: reader.read_f32()?,
        };
        let coherence = reader.read_f32()?;
        let spectral_entropy = reader.read_f32()?;
        let center_phase = reader.read_f32()?;
        let center_magnitude = reader.read_f32()?;

        let mut top_slots = [0; SNAPSHOT_TOP_SLOTS];
        for slot in &mut top_slots {
            *slot = reader.read_u16()?;
        }

        let mut top_phases = [0.0; SNAPSHOT_TOP_SLOTS];
        for phase in &mut top_phases {
            *phase = reader.read_f32()?;
        }

        let mut top_amplitudes = [0.0; SNAPSHOT_TOP_SLOTS];
        for amplitude in &mut top_amplitudes {
            *amplitude = reader.read_f32()?;
        }

        let mut active_cell_ids = [0; STAGE2_TOP_K];
        for cell_id in &mut active_cell_ids {
            *cell_id = reader.read_u32()?;
        }

        reader.finish()?;

        Ok(Self {
            version,
            seed,
            input_byte,
            carrier,
            coherence,
            spectral_entropy,
            center_phase,
            center_magnitude,
            top_slots,
            top_phases,
            top_amplitudes,
            active_cell_ids,
        })
    }
}

/// Snapshot parse errors for the stable Stage 2 format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotParseError {
    BadLength { actual: usize },
    BadMagic,
    UnsupportedVersion(u16),
    BadShape,
    Truncated,
    TrailingBytes,
}

impl fmt::Display for SnapshotParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadLength { actual } => {
                write!(
                    formatter,
                    "snapshot length must be {SNAPSHOT_V1_BYTES} bytes, got {actual}"
                )
            }
            Self::BadMagic => formatter.write_str("snapshot magic is not NWV1"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported snapshot version {version}")
            }
            Self::BadShape => {
                formatter.write_str("snapshot shape does not match Stage 2 constants")
            }
            Self::Truncated => formatter.write_str("snapshot is truncated"),
            Self::TrailingBytes => formatter.write_str("snapshot has trailing bytes"),
        }
    }
}

impl std::error::Error for SnapshotParseError {}

/// Combined result for the deterministic Stage 2 tick.
#[derive(Debug, Clone, PartialEq)]
pub struct Stage2Tick {
    pub carrier: CarrierWave,
    pub trace: TickTrace,
    pub snapshot: SpectrumSnapshot,
}

struct SnapshotWriter {
    bytes: [u8; SNAPSHOT_V1_BYTES],
    offset: usize,
}

impl Default for SnapshotWriter {
    fn default() -> Self {
        Self {
            bytes: [0; SNAPSHOT_V1_BYTES],
            offset: 0,
        }
    }
}

impl SnapshotWriter {
    fn write_bytes<const N: usize>(&mut self, bytes: &[u8; N]) {
        self.bytes[self.offset..self.offset + N].copy_from_slice(bytes);
        self.offset += N;
    }

    fn write_u8(&mut self, value: u8) {
        self.bytes[self.offset] = value;
        self.offset += 1;
    }

    fn write_u16(&mut self, value: u16) {
        self.write_bytes(&value.to_le_bytes());
    }

    fn write_u32(&mut self, value: u32) {
        self.write_bytes(&value.to_le_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.write_bytes(&value.to_le_bytes());
    }

    fn write_f32(&mut self, value: f32) {
        self.write_bytes(&value.to_le_bytes());
    }

    fn finish(self) -> [u8; SNAPSHOT_V1_BYTES] {
        debug_assert_eq!(self.offset, SNAPSHOT_V1_BYTES);
        self.bytes
    }
}

struct SnapshotReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> SnapshotReader<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self, SnapshotParseError> {
        if bytes.len() != SNAPSHOT_V1_BYTES {
            return Err(SnapshotParseError::BadLength {
                actual: bytes.len(),
            });
        }

        Ok(Self { bytes, offset: 0 })
    }

    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N], SnapshotParseError> {
        if self.offset + N > self.bytes.len() {
            return Err(SnapshotParseError::Truncated);
        }

        let mut output = [0; N];
        output.copy_from_slice(&self.bytes[self.offset..self.offset + N]);
        self.offset += N;
        Ok(output)
    }

    fn read_u8(&mut self) -> Result<u8, SnapshotParseError> {
        if self.offset + 1 > self.bytes.len() {
            return Err(SnapshotParseError::Truncated);
        }
        let value = self.bytes[self.offset];
        self.offset += 1;
        Ok(value)
    }

    fn read_u16(&mut self) -> Result<u16, SnapshotParseError> {
        Ok(u16::from_le_bytes(self.read_bytes()?))
    }

    fn read_u32(&mut self) -> Result<u32, SnapshotParseError> {
        Ok(u32::from_le_bytes(self.read_bytes()?))
    }

    fn read_u64(&mut self) -> Result<u64, SnapshotParseError> {
        Ok(u64::from_le_bytes(self.read_bytes()?))
    }

    fn read_f32(&mut self) -> Result<f32, SnapshotParseError> {
        Ok(f32::from_le_bytes(self.read_bytes()?))
    }

    fn finish(self) -> Result<(), SnapshotParseError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(SnapshotParseError::TrailingBytes)
        }
    }
}
