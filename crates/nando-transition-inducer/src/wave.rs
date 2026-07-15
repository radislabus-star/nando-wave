use nando_core::{
    PhaseCenterCell, add_phase_vector, phase_center_from_sum, phase_coherence,
    phase_margin_to_micro, phase_vector_from_atom_ids, phase_vector_from_atoms,
};
use serde::{Deserialize, Serialize};

const ROUTING_CELL_SCALE: f64 = 16_384.0;
const MAX_ANTI_CENTERS: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaveAblation {
    Full,
    NoPhase,
    ShuffledPhase,
    MagnitudeOnly,
    WithoutAntiCenter,
    LegacySingleAntiCenter,
    SkeletonOnly,
    RandomCenter,
    RandomRanking,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PortablePhaseCell {
    pub re_q14: i16,
    pub im_q14: i16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PortablePhaseCenter {
    pub cells: Vec<PortablePhaseCell>,
    pub support: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PortableRoutingSignature {
    pub schema: String,
    pub cells: usize,
    pub positive_centers: Vec<PortablePhaseCenter>,
    pub anti_center: Vec<PortablePhaseCell>,
    #[serde(default)]
    pub anti_centers: Vec<PortablePhaseCenter>,
    pub positive_examples: usize,
    pub negative_examples: usize,
}

impl PortableRoutingSignature {
    #[must_use]
    pub fn score_atoms(&self, atoms: &[String]) -> i64 {
        let vector = phase_vector_from_atoms(atoms.iter().map(String::as_str), self.cells);
        let positive = self
            .positive_centers
            .iter()
            .map(|center| phase_coherence(&vector, &decode_cells(&center.cells)))
            .fold(f64::NEG_INFINITY, f64::max);
        if !positive.is_finite() {
            return i64::MIN;
        }
        let negative = if self.negative_examples == 0 {
            0.0
        } else if !self.anti_centers.is_empty() {
            self.anti_centers
                .iter()
                .map(|center| phase_coherence(&vector, &decode_cells(&center.cells)))
                .fold(f64::NEG_INFINITY, f64::max)
        } else {
            phase_coherence(&vector, &decode_cells(&self.anti_center))
        };
        phase_margin_to_micro(positive - negative).unwrap_or(i64::MIN)
    }

    #[must_use]
    pub fn score_atom_ids(&self, atom_ids: &[u64]) -> i64 {
        let vector = phase_vector_from_atom_ids(atom_ids.iter().copied(), self.cells);
        self.score_vector(&vector)
    }

    fn score_vector(&self, vector: &[PhaseCenterCell]) -> i64 {
        let positive = self
            .positive_centers
            .iter()
            .map(|center| phase_coherence(vector, &decode_cells(&center.cells)))
            .fold(f64::NEG_INFINITY, f64::max);
        if !positive.is_finite() {
            return i64::MIN;
        }
        let negative = if self.negative_examples == 0 {
            0.0
        } else if !self.anti_centers.is_empty() {
            self.anti_centers
                .iter()
                .map(|center| phase_coherence(vector, &decode_cells(&center.cells)))
                .fold(f64::NEG_INFINITY, f64::max)
        } else {
            phase_coherence(vector, &decode_cells(&self.anti_center))
        };
        phase_margin_to_micro(positive - negative).unwrap_or(i64::MIN)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WaveTrainingExample {
    pub atoms: Vec<String>,
    pub valid: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WaveIdTrainingExample {
    pub atom_ids: Vec<u64>,
    pub valid: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WaveContributionMetrics {
    pub positive_examples: usize,
    pub negative_examples: usize,
    pub negative_center_count: usize,
    pub center_count: usize,
    pub center_split_count: usize,
    pub positive_center_mean_support_milli: usize,
    pub positive_center_max_support: usize,
    pub negative_center_mean_support_milli: usize,
    pub negative_center_max_support: usize,
    pub wave_memory_bytes: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct RelationCenter {
    sum: Vec<PhaseCenterCell>,
    center: Vec<PhaseCenterCell>,
    support: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RelationWaveMemory {
    cells: usize,
    split_threshold: f64,
    positive_centers: Vec<RelationCenter>,
    negative_centers: Vec<RelationCenter>,
    negative_sum: Vec<PhaseCenterCell>,
    negative_center: Vec<PhaseCenterCell>,
    positive_examples: usize,
    negative_examples: usize,
    center_split_count: usize,
}

impl RelationWaveMemory {
    #[must_use]
    pub fn train(examples: &[WaveTrainingExample], cells: usize, split_threshold: f64) -> Self {
        let mut memory = Self::empty(cells, split_threshold);
        for example in examples.iter().filter(|example| example.valid) {
            let vector = phase_vector_from_atoms(example.atoms.iter().map(String::as_str), cells);
            memory.add_positive_vector(&vector);
        }
        for example in examples.iter().filter(|example| !example.valid) {
            let vector = phase_vector_from_atoms(example.atoms.iter().map(String::as_str), cells);
            memory.add_negative_observation(&vector);
        }
        memory.finish_training();
        memory
    }

    #[must_use]
    pub fn train_ids(
        examples: &[WaveIdTrainingExample],
        cells: usize,
        split_threshold: f64,
    ) -> Self {
        let mut memory = Self::empty(cells, split_threshold);
        for example in examples.iter().filter(|example| example.valid) {
            let vector = phase_vector_from_atom_ids(example.atom_ids.iter().copied(), cells);
            memory.add_positive_vector(&vector);
        }
        for example in examples.iter().filter(|example| !example.valid) {
            let vector = phase_vector_from_atom_ids(example.atom_ids.iter().copied(), cells);
            memory.add_negative_observation(&vector);
        }
        memory.finish_training();
        memory
    }

    fn empty(cells: usize, split_threshold: f64) -> Self {
        Self {
            cells,
            split_threshold,
            positive_centers: Vec::new(),
            negative_centers: Vec::new(),
            negative_sum: vec![PhaseCenterCell::default(); cells],
            negative_center: vec![PhaseCenterCell::default(); cells],
            positive_examples: 0,
            negative_examples: 0,
            center_split_count: 0,
        }
    }

    fn add_negative_observation(&mut self, vector: &[PhaseCenterCell]) {
        add_phase_vector(&mut self.negative_sum, vector, 1.0);
        self.add_negative_vector(vector);
    }

    fn finish_training(&mut self) {
        self.negative_center = phase_center_from_sum(&self.negative_sum);
    }

    #[must_use]
    pub fn cells(&self) -> usize {
        self.cells
    }

    #[must_use]
    pub fn center_count(&self) -> usize {
        self.positive_centers.len()
    }

    #[must_use]
    pub fn score_atoms(&self, atoms: &[String]) -> i64 {
        let vector = phase_vector_from_atoms(atoms.iter().map(String::as_str), self.cells);
        self.score_vector(&vector)
    }

    #[must_use]
    pub fn score_atom_ids(&self, atom_ids: &[u64]) -> i64 {
        let vector = phase_vector_from_atom_ids(atom_ids.iter().copied(), self.cells);
        self.score_vector(&vector)
    }

    #[must_use]
    pub(crate) fn score_atoms_with_ablation(
        &self,
        atoms: &[String],
        ablation: WaveAblation,
    ) -> i64 {
        let selected_atoms = if ablation == WaveAblation::SkeletonOnly {
            &atoms[..atoms.len().min(2)]
        } else {
            atoms
        };
        let mut vector =
            phase_vector_from_atoms(selected_atoms.iter().map(String::as_str), self.cells);
        match ablation {
            WaveAblation::Full | WaveAblation::SkeletonOnly => self.score_vector(&vector),
            WaveAblation::NoPhase => 0,
            WaveAblation::ShuffledPhase => {
                if vector.len() > 1 {
                    let shift = 5 % vector.len();
                    vector.rotate_left(shift.max(1));
                }
                self.score_vector(&vector)
            }
            WaveAblation::MagnitudeOnly => self.score_magnitude_only(&vector),
            WaveAblation::WithoutAntiCenter => self.score_without_anti_center(&vector),
            WaveAblation::LegacySingleAntiCenter => self.score_with_legacy_anti_center(&vector),
            WaveAblation::RandomCenter => self.score_random_center(&vector),
            WaveAblation::RandomRanking => 0,
        }
    }

    #[must_use]
    pub fn score_vector(&self, vector: &[PhaseCenterCell]) -> i64 {
        let positive = self
            .positive_centers
            .iter()
            .map(|cluster| phase_coherence(vector, &cluster.center))
            .fold(f64::NEG_INFINITY, f64::max);
        if !positive.is_finite() {
            return i64::MIN;
        }
        let negative = if self.negative_examples == 0 {
            0.0
        } else if !self.negative_centers.is_empty() {
            self.negative_centers
                .iter()
                .map(|cluster| phase_coherence(vector, &cluster.center))
                .fold(f64::NEG_INFINITY, f64::max)
        } else {
            phase_coherence(vector, &self.negative_center)
        };
        phase_margin_to_micro(positive - negative).unwrap_or(i64::MIN)
    }

    pub fn consolidate(&mut self, atoms: &[String]) {
        let vector = phase_vector_from_atoms(atoms.iter().map(String::as_str), self.cells);
        self.add_positive_vector(&vector);
    }

    #[must_use]
    pub fn metrics(&self) -> WaveContributionMetrics {
        WaveContributionMetrics {
            positive_examples: self.positive_examples,
            negative_examples: self.negative_examples,
            negative_center_count: self.negative_centers.len(),
            center_count: self.positive_centers.len(),
            center_split_count: self.center_split_count,
            positive_center_mean_support_milli: mean_support_milli(&self.positive_centers),
            positive_center_max_support: max_support(&self.positive_centers),
            negative_center_mean_support_milli: mean_support_milli(&self.negative_centers),
            negative_center_max_support: max_support(&self.negative_centers),
            wave_memory_bytes: self.bytes_estimate(),
        }
    }

    #[must_use]
    pub fn portable_signature(&self) -> PortableRoutingSignature {
        PortableRoutingSignature {
            schema: "nando.portable-relation-wave-routing.v1".to_owned(),
            cells: self.cells,
            positive_centers: self
                .positive_centers
                .iter()
                .map(|center| PortablePhaseCenter {
                    cells: encode_cells(&center.center),
                    support: center.support,
                })
                .collect(),
            anti_center: encode_cells(&self.negative_center),
            anti_centers: self
                .negative_centers
                .iter()
                .map(|center| PortablePhaseCenter {
                    cells: encode_cells(&center.center),
                    support: center.support,
                })
                .collect(),
            positive_examples: self.positive_examples,
            negative_examples: self.negative_examples,
        }
    }

    #[must_use]
    pub fn bytes_estimate(&self) -> usize {
        let cell_bytes = std::mem::size_of::<PhaseCenterCell>();
        (self.positive_centers.len() * 2 + self.negative_centers.len() * 2 + 2)
            * self.cells
            * cell_bytes
            + (self.positive_centers.len() + self.negative_centers.len())
                * std::mem::size_of::<RelationCenter>()
    }

    fn add_positive_vector(&mut self, vector: &[PhaseCenterCell]) {
        self.positive_examples += 1;
        let best = self
            .positive_centers
            .iter()
            .enumerate()
            .map(|(index, cluster)| (index, phase_coherence(vector, &cluster.center)))
            .max_by(|left, right| left.1.total_cmp(&right.1));
        if let Some((index, coherence)) = best
            && coherence >= self.split_threshold
        {
            let cluster = &mut self.positive_centers[index];
            add_phase_vector(&mut cluster.sum, vector, 1.0);
            cluster.center = phase_center_from_sum(&cluster.sum);
            cluster.support += 1;
            return;
        }
        if !self.positive_centers.is_empty() {
            self.center_split_count += 1;
        }
        self.positive_centers.push(RelationCenter {
            sum: vector.to_vec(),
            center: phase_center_from_sum(vector),
            support: 1,
        });
    }

    fn add_negative_vector(&mut self, vector: &[PhaseCenterCell]) {
        self.negative_examples += 1;
        let best = self
            .negative_centers
            .iter()
            .enumerate()
            .map(|(index, cluster)| (index, phase_coherence(vector, &cluster.center)))
            .max_by(|left, right| left.1.total_cmp(&right.1));
        if let Some((index, coherence)) = best
            && (coherence >= self.split_threshold
                || self.negative_centers.len() >= MAX_ANTI_CENTERS)
        {
            let cluster = &mut self.negative_centers[index];
            add_phase_vector(&mut cluster.sum, vector, 1.0);
            cluster.center = phase_center_from_sum(&cluster.sum);
            cluster.support += 1;
            return;
        }
        self.negative_centers.push(RelationCenter {
            sum: vector.to_vec(),
            center: phase_center_from_sum(vector),
            support: 1,
        });
    }

    fn score_without_anti_center(&self, vector: &[PhaseCenterCell]) -> i64 {
        let positive = self
            .positive_centers
            .iter()
            .map(|cluster| phase_coherence(vector, &cluster.center))
            .fold(f64::NEG_INFINITY, f64::max);
        phase_margin_to_micro(positive).unwrap_or(i64::MIN)
    }

    fn score_with_legacy_anti_center(&self, vector: &[PhaseCenterCell]) -> i64 {
        let positive = self
            .positive_centers
            .iter()
            .map(|cluster| phase_coherence(vector, &cluster.center))
            .fold(f64::NEG_INFINITY, f64::max);
        if !positive.is_finite() {
            return i64::MIN;
        }
        let negative = phase_coherence(vector, &self.negative_center);
        phase_margin_to_micro(positive - negative).unwrap_or(i64::MIN)
    }

    fn score_magnitude_only(&self, vector: &[PhaseCenterCell]) -> i64 {
        let positive = self
            .positive_centers
            .iter()
            .map(|cluster| magnitude_coherence(vector, &cluster.center))
            .fold(f64::NEG_INFINITY, f64::max);
        if !positive.is_finite() {
            return i64::MIN;
        }
        let negative = if self.negative_examples == 0 {
            0.0
        } else if !self.negative_centers.is_empty() {
            self.negative_centers
                .iter()
                .map(|cluster| magnitude_coherence(vector, &cluster.center))
                .fold(f64::NEG_INFINITY, f64::max)
        } else {
            magnitude_coherence(vector, &self.negative_center)
        };
        phase_margin_to_micro(positive - negative).unwrap_or(i64::MIN)
    }

    fn score_random_center(&self, vector: &[PhaseCenterCell]) -> i64 {
        let center = phase_vector_from_atoms(["ablation:unrelated_random_center"], self.cells);
        phase_margin_to_micro(phase_coherence(vector, &center)).unwrap_or(i64::MIN)
    }
}

fn magnitude_coherence(vector: &[PhaseCenterCell], center: &[PhaseCenterCell]) -> f64 {
    if vector.is_empty() || center.is_empty() {
        return 0.0;
    }
    let mut score = 0.0;
    let mut active = 0usize;
    for (value, center) in vector.iter().zip(center.iter()) {
        score += value.re.hypot(value.im) * center.re.hypot(center.im);
        active += 1;
    }
    score / active.max(1) as f64
}

fn mean_support_milli(centers: &[RelationCenter]) -> usize {
    if centers.is_empty() {
        return 0;
    }
    centers
        .iter()
        .map(|center| center.support)
        .sum::<usize>()
        .saturating_mul(1000)
        / centers.len()
}

fn max_support(centers: &[RelationCenter]) -> usize {
    centers
        .iter()
        .map(|center| center.support)
        .max()
        .unwrap_or(0)
}

fn encode_cells(cells: &[PhaseCenterCell]) -> Vec<PortablePhaseCell> {
    cells
        .iter()
        .map(|cell| PortablePhaseCell {
            re_q14: quantize_cell(cell.re),
            im_q14: quantize_cell(cell.im),
        })
        .collect()
}

fn decode_cells(cells: &[PortablePhaseCell]) -> Vec<PhaseCenterCell> {
    cells
        .iter()
        .map(|cell| PhaseCenterCell {
            re: f64::from(cell.re_q14) / ROUTING_CELL_SCALE,
            im: f64::from(cell.im_q14) / ROUTING_CELL_SCALE,
        })
        .collect()
}

fn quantize_cell(value: f64) -> i16 {
    (value * ROUTING_CELL_SCALE)
        .round()
        .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
}
