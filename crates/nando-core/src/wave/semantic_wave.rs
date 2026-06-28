//! L2 semantic-wave relation operator.
//!
//! This is not a text parser. It assumes role-complete atoms already exist and
//! tests whether a relation operator can map a subject atom to the correct
//! object atom on heldout slots while rejecting role/slot/route near-misses.

use std::collections::HashMap;

use super::{SURFACE_WAVE_DIM, SemanticEquationForm};

pub const SEMANTIC_WAVE_DIM: usize = SURFACE_WAVE_DIM;
pub const SEMANTIC_WAVE_BYTES: usize = SEMANTIC_WAVE_DIM * std::mem::size_of::<i16>();
pub const SEMANTIC_OPERATOR_BYTES: usize = SEMANTIC_WAVE_DIM * std::mem::size_of::<i32>();

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SemanticAtom {
    pub label: String,
    pub role: String,
    pub family: String,
    pub slot: u32,
}

impl SemanticAtom {
    #[must_use]
    pub fn new(
        role: impl Into<String>,
        family: impl Into<String>,
        slot: u32,
        label: impl Into<String>,
    ) -> Self {
        Self {
            role: role.into(),
            family: family.into(),
            slot,
            label: label.into(),
        }
    }

    #[must_use]
    pub fn wave(&self) -> SemanticWave4096 {
        let mut wave = SemanticWave4096::zero();
        wave.add_typed("role", self.role.as_bytes(), 2);
        wave.add_typed("family", self.family.as_bytes(), 2);
        wave.add_slot("slot", self.slot, 3);
        wave
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SemanticSchemaKey {
    pub subject_role: String,
    pub relation: String,
    pub object_role: String,
    pub route: String,
    pub polarity: String,
    pub evidence_kind: String,
}

impl SemanticSchemaKey {
    #[must_use]
    pub fn new(
        subject_role: impl Into<String>,
        relation: impl Into<String>,
        object_role: impl Into<String>,
        route: impl Into<String>,
        polarity: impl Into<String>,
        evidence_kind: impl Into<String>,
    ) -> Self {
        Self {
            subject_role: subject_role.into(),
            relation: relation.into(),
            object_role: object_role.into(),
            route: route.into(),
            polarity: polarity.into(),
            evidence_kind: evidence_kind.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticFact {
    pub subject: SemanticAtom,
    pub schema: SemanticSchemaKey,
    pub object: SemanticAtom,
}

impl SemanticFact {
    #[must_use]
    pub fn new(subject: SemanticAtom, schema: SemanticSchemaKey, object: SemanticAtom) -> Self {
        Self {
            subject,
            schema,
            object,
        }
    }

    #[must_use]
    pub fn query(&self) -> SemanticQuery {
        SemanticQuery {
            subject: self.subject.clone(),
            schema: self.schema.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticQuery {
    pub subject: SemanticAtom,
    pub schema: SemanticSchemaKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticCandidate {
    pub atom: SemanticAtom,
}

impl SemanticCandidate {
    #[must_use]
    pub fn new(atom: SemanticAtom) -> Self {
        Self { atom }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticPrediction {
    pub object_label: String,
    pub score: i64,
    pub runner_up_score: i64,
    pub margin: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticEquationPrediction {
    pub resolved_role: String,
    pub resolved_label: String,
    pub score: i64,
    pub runner_up_score: i64,
    pub margin: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticWaveEvalReport {
    pub eval_cases: usize,
    pub correct: usize,
    pub accuracy: f32,
    pub min_margin: i64,
    pub average_margin: f32,
    pub role_swap_rejected: bool,
    pub slot_swap_rejected: bool,
    pub unknown_route_rejected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticWaveGrokkingVerdict {
    Proven,
    Watch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticWaveGrokkingProof {
    pub verdict: SemanticWaveGrokkingVerdict,
    pub train_facts: usize,
    pub heldout_facts: usize,
    pub operator_count: usize,
    pub operator_hot_bytes: usize,
    pub naive_fact_wave_bytes: usize,
    pub compression_pass: bool,
    pub heldout_pass: bool,
    pub role_swap_reject_pass: bool,
    pub slot_swap_reject_pass: bool,
    pub route_swap_reject_pass: bool,
    pub exact_lookup_heldout_hits: usize,
    pub eval: SemanticWaveEvalReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticWave4096 {
    lanes: [i16; SEMANTIC_WAVE_DIM],
}

impl SemanticWave4096 {
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            lanes: [0; SEMANTIC_WAVE_DIM],
        }
    }

    #[must_use]
    pub fn lanes(&self) -> &[i16; SEMANTIC_WAVE_DIM] {
        &self.lanes
    }

    fn add_typed(&mut self, channel: &str, bytes: &[u8], amplitude: i16) {
        for salt in 0..4 {
            let mixed = semantic_mix(channel.as_bytes(), bytes, salt);
            let lane = (mixed % SEMANTIC_WAVE_DIM as u64) as usize;
            let sign = if (mixed >> 63) == 0 { 1 } else { -1 };
            self.lanes[lane] = self.lanes[lane].saturating_add(sign * amplitude);
        }
    }

    fn add_slot(&mut self, channel: &str, slot: u32, amplitude: i16) {
        let bytes = slot.to_le_bytes();
        self.add_typed(channel, &bytes, amplitude);
    }

    fn add_schema(&mut self, schema: &SemanticSchemaKey) {
        self.add_typed("subject_role", schema.subject_role.as_bytes(), 1);
        self.add_typed("relation", schema.relation.as_bytes(), 2);
        self.add_typed("object_role", schema.object_role.as_bytes(), 2);
        self.add_typed("route", schema.route.as_bytes(), 2);
        self.add_typed("polarity", schema.polarity.as_bytes(), 1);
        self.add_typed("evidence_kind", schema.evidence_kind.as_bytes(), 1);
    }
}

impl Default for SemanticWave4096 {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticRelationOperator {
    support: u32,
    delta_sum: Vec<i32>,
}

impl SemanticRelationOperator {
    #[must_use]
    pub fn support(&self) -> u32 {
        self.support
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SemanticWaveMemory {
    operators: HashMap<SemanticSchemaKey, SemanticRelationOperator>,
}

impl SemanticWaveMemory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn train<'a, I>(&mut self, facts: I)
    where
        I: IntoIterator<Item = &'a SemanticFact>,
    {
        for fact in facts {
            self.train_fact(fact);
        }
    }

    #[must_use]
    pub fn operator_count(&self) -> usize {
        self.operators.len()
    }

    #[must_use]
    pub fn hot_operator_bytes(&self) -> usize {
        self.operators.len() * SEMANTIC_OPERATOR_BYTES
    }

    #[must_use]
    pub fn has_operator(&self, schema: &SemanticSchemaKey) -> bool {
        self.operators.contains_key(schema)
    }

    #[must_use]
    pub fn predict(
        &self,
        query: &SemanticQuery,
        candidates: &[SemanticCandidate],
    ) -> Option<SemanticPrediction> {
        let operator = self.operators.get(&query.schema)?;
        if candidates.is_empty() {
            return None;
        }

        let query_wave = query_wave(query);
        let mut best_index = 0;
        let mut best_score = i64::MIN;
        let mut runner_up_score = i64::MIN;

        for (index, candidate) in candidates.iter().enumerate() {
            let score = score_candidate(&query_wave, operator, &candidate.atom.wave());
            if score > best_score {
                runner_up_score = best_score;
                best_score = score;
                best_index = index;
            } else if score > runner_up_score {
                runner_up_score = score;
            }
        }

        if runner_up_score == i64::MIN {
            runner_up_score = best_score;
        }

        Some(SemanticPrediction {
            object_label: candidates[best_index].atom.label.clone(),
            score: best_score,
            runner_up_score,
            margin: best_score - runner_up_score,
        })
    }

    #[must_use]
    pub fn solve_equation(
        &self,
        equation: &SemanticEquationForm,
        candidates: &[SemanticCandidate],
    ) -> Option<SemanticEquationPrediction> {
        let operator = self.operators.get(&equation.schema)?;
        let unknown_role = equation.unknown_role.as_ref()?;

        match (&equation.subject, &equation.object) {
            (Some(subject), None) if unknown_role == &equation.schema.object_role => {
                let prediction = self.predict(
                    &SemanticQuery {
                        subject: subject.clone(),
                        schema: equation.schema.clone(),
                    },
                    candidates,
                )?;
                Some(SemanticEquationPrediction {
                    resolved_role: unknown_role.clone(),
                    resolved_label: prediction.object_label,
                    score: prediction.score,
                    runner_up_score: prediction.runner_up_score,
                    margin: prediction.margin,
                })
            }
            (None, Some(object)) if unknown_role == &equation.schema.subject_role => {
                solve_unknown_subject(&equation.schema, operator, object, unknown_role, candidates)
            }
            _ => None,
        }
    }

    fn train_fact(&mut self, fact: &SemanticFact) {
        let query = query_wave(&fact.query());
        let object = fact.object.wave();
        let operator = self
            .operators
            .entry(fact.schema.clone())
            .or_insert_with(|| SemanticRelationOperator {
                support: 0,
                delta_sum: vec![0; SEMANTIC_WAVE_DIM],
            });
        operator.support += 1;

        for lane in 0..SEMANTIC_WAVE_DIM {
            operator.delta_sum[lane] +=
                i32::from(object.lanes[lane]) - i32::from(query.lanes[lane]);
        }
    }
}

impl SemanticWaveGrokkingProof {
    #[must_use]
    pub fn prove_profile() -> Self {
        let schema = package_command_schema();
        let wrong_route_schema = SemanticSchemaKey::new(
            "package",
            "provides_command",
            "command",
            "linux.service.runtime",
            "positive",
            "package_metadata",
        );
        let train: Vec<_> = (0..8_000)
            .map(|slot| package_command_fact(slot, &schema))
            .collect();
        let heldout: Vec<_> = (8_000..10_000)
            .map(|slot| package_command_fact(slot, &schema))
            .collect();

        let mut memory = SemanticWaveMemory::new();
        memory.train(train.iter());
        let eval = evaluate_profile(&memory, &heldout, &wrong_route_schema);

        let exact_lookup_heldout_hits = heldout
            .iter()
            .filter(|heldout_fact| train.iter().any(|train_fact| train_fact == *heldout_fact))
            .count();
        let operator_hot_bytes = memory.hot_operator_bytes();
        let naive_fact_wave_bytes = (train.len() + heldout.len()) * SEMANTIC_WAVE_BYTES;
        let compression_pass = operator_hot_bytes * 32 < naive_fact_wave_bytes;
        let heldout_pass = eval.accuracy >= 0.99 && eval.min_margin > 0;
        let role_swap_reject_pass = eval.role_swap_rejected;
        let slot_swap_reject_pass = eval.slot_swap_rejected;
        let route_swap_reject_pass = eval.unknown_route_rejected;
        let verdict = if compression_pass
            && heldout_pass
            && role_swap_reject_pass
            && slot_swap_reject_pass
            && route_swap_reject_pass
            && exact_lookup_heldout_hits == 0
        {
            SemanticWaveGrokkingVerdict::Proven
        } else {
            SemanticWaveGrokkingVerdict::Watch
        };

        Self {
            verdict,
            train_facts: train.len(),
            heldout_facts: heldout.len(),
            operator_count: memory.operator_count(),
            operator_hot_bytes,
            naive_fact_wave_bytes,
            compression_pass,
            heldout_pass,
            role_swap_reject_pass,
            slot_swap_reject_pass,
            route_swap_reject_pass,
            exact_lookup_heldout_hits,
            eval,
        }
    }
}

fn evaluate_profile(
    memory: &SemanticWaveMemory,
    heldout: &[SemanticFact],
    wrong_route_schema: &SemanticSchemaKey,
) -> SemanticWaveEvalReport {
    let mut correct = 0;
    let mut margin_sum = 0i64;
    let mut min_margin = i64::MAX;
    let mut role_swap_rejected = true;
    let mut slot_swap_rejected = true;
    let mut unknown_route_rejected = true;

    for fact in heldout {
        let slot = fact.subject.slot;
        let candidates = vec![
            SemanticCandidate::new(fact.object.clone()),
            SemanticCandidate::new(package_atom(slot)),
            SemanticCandidate::new(command_atom(slot + 1)),
        ];
        let prediction = memory
            .predict(&fact.query(), &candidates)
            .expect("profile operator must exist");
        if prediction.object_label == fact.object.label {
            correct += 1;
        }
        min_margin = min_margin.min(prediction.margin);
        margin_sum += prediction.margin;
        slot_swap_rejected &= prediction.object_label != command_atom(slot + 1).label;

        let role_swap_prediction = memory
            .predict(
                &SemanticQuery {
                    subject: command_atom(slot),
                    schema: fact.schema.clone(),
                },
                &[
                    SemanticCandidate::new(package_atom(slot)),
                    SemanticCandidate::new(command_atom(slot)),
                ],
            )
            .expect("profile operator must exist");
        role_swap_rejected &= role_swap_prediction.object_label != package_atom(slot).label;

        let unknown_route_query = SemanticQuery {
            subject: fact.subject.clone(),
            schema: wrong_route_schema.clone(),
        };
        unknown_route_rejected &= memory.predict(&unknown_route_query, &candidates).is_none();
    }

    let eval_cases = heldout.len();
    SemanticWaveEvalReport {
        eval_cases,
        correct,
        accuracy: correct as f32 / eval_cases as f32,
        min_margin,
        average_margin: margin_sum as f32 / eval_cases as f32,
        role_swap_rejected,
        slot_swap_rejected,
        unknown_route_rejected,
    }
}

fn query_wave(query: &SemanticQuery) -> SemanticWave4096 {
    let mut wave = query.subject.wave();
    wave.add_schema(&query.schema);
    wave
}

fn score_candidate(
    query_wave: &SemanticWave4096,
    operator: &SemanticRelationOperator,
    candidate_wave: &SemanticWave4096,
) -> i64 {
    let support = i64::from(operator.support.max(1));
    let mut score = 0i64;
    for lane in 0..SEMANTIC_WAVE_DIM {
        let predicted =
            i64::from(query_wave.lanes[lane]) * support + i64::from(operator.delta_sum[lane]);
        score += predicted * i64::from(candidate_wave.lanes[lane]);
    }
    score
}

fn solve_unknown_subject(
    schema: &SemanticSchemaKey,
    operator: &SemanticRelationOperator,
    object: &SemanticAtom,
    unknown_role: &str,
    candidates: &[SemanticCandidate],
) -> Option<SemanticEquationPrediction> {
    if candidates.is_empty() {
        return None;
    }

    let object_wave = object.wave();
    let mut best_index = 0;
    let mut best_score = i64::MIN;
    let mut runner_up_score = i64::MIN;

    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.atom.role != unknown_role {
            continue;
        }
        let query_wave = query_wave(&SemanticQuery {
            subject: candidate.atom.clone(),
            schema: schema.clone(),
        });
        let score = score_candidate(&query_wave, operator, &object_wave);
        if score > best_score {
            runner_up_score = best_score;
            best_score = score;
            best_index = index;
        } else if score > runner_up_score {
            runner_up_score = score;
        }
    }

    if best_score == i64::MIN {
        return None;
    }
    if runner_up_score == i64::MIN {
        runner_up_score = best_score;
    }

    Some(SemanticEquationPrediction {
        resolved_role: unknown_role.to_string(),
        resolved_label: candidates[best_index].atom.label.clone(),
        score: best_score,
        runner_up_score,
        margin: best_score - runner_up_score,
    })
}

fn package_command_schema() -> SemanticSchemaKey {
    SemanticSchemaKey::new(
        "package",
        "provides_command",
        "command",
        "linux.command.provider",
        "positive",
        "package_metadata",
    )
}

fn package_command_fact(slot: u32, schema: &SemanticSchemaKey) -> SemanticFact {
    SemanticFact::new(package_atom(slot), schema.clone(), command_atom(slot))
}

fn package_atom(slot: u32) -> SemanticAtom {
    SemanticAtom::new(
        "package",
        "linux-command-provider",
        slot,
        format!("pkg_{slot:04}"),
    )
}

fn command_atom(slot: u32) -> SemanticAtom {
    SemanticAtom::new(
        "command",
        "linux-command-provider",
        slot,
        format!("cmd_{slot:04}"),
    )
}

fn semantic_mix(channel: &[u8], payload: &[u8], salt: u64) -> u64 {
    let mut state = 0x5345_4D41_4E54_5731u64 ^ salt.rotate_left(17);
    for byte in channel.iter().chain(payload.iter()) {
        state ^= u64::from(*byte).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        state = splitmix64(state);
    }
    splitmix64(state)
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_atom_hot_wave_uses_role_family_slot_not_label_text() {
        let left = SemanticAtom::new("package", "linux-command-provider", 42, "bash");
        let right = SemanticAtom::new(
            "package",
            "linux-command-provider",
            42,
            "totally-different-label",
        );
        let different_slot = SemanticAtom::new("package", "linux-command-provider", 43, "bash");

        assert_eq!(left.wave(), right.wave());
        assert_ne!(left.wave(), different_slot.wave());
    }

    #[test]
    fn semantic_wave_operator_predicts_heldout_object_and_rejects_near_misses() {
        let proof = SemanticWaveGrokkingProof::prove_profile();

        assert_eq!(proof.verdict, SemanticWaveGrokkingVerdict::Proven);
        assert_eq!(proof.train_facts, 8_000);
        assert_eq!(proof.heldout_facts, 2_000);
        assert_eq!(proof.operator_count, 1);
        assert_eq!(proof.exact_lookup_heldout_hits, 0);
        assert!(proof.compression_pass);
        assert!(proof.heldout_pass);
        assert!(proof.role_swap_reject_pass);
        assert!(proof.slot_swap_reject_pass);
        assert!(proof.route_swap_reject_pass);
        assert!(proof.eval.accuracy >= 0.99, "proof={proof:?}");
        assert!(proof.eval.min_margin > 0, "proof={proof:?}");
    }

    #[test]
    fn untrained_or_wrong_route_has_no_semantic_authority() {
        let schema = package_command_schema();
        let wrong_route_schema = SemanticSchemaKey::new(
            "package",
            "provides_command",
            "command",
            "linux.service.runtime",
            "positive",
            "package_metadata",
        );
        let fact = package_command_fact(7, &schema);
        let memory = SemanticWaveMemory::new();

        assert!(
            memory
                .predict(&fact.query(), &[SemanticCandidate::new(command_atom(7))])
                .is_none()
        );
        assert!(!memory.has_operator(&wrong_route_schema));
    }
}
