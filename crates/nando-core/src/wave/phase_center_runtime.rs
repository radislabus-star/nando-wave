//! Flat CPU runtime for phase-center operator scoring.
//!
//! This module intentionally contains no corpus loading, no lookup table of
//! answers, and no training loop. It scores a candidate transition against
//! precompiled positive/negative phase centers.

pub const PHASE_CENTER_RUNTIME_PACKAGE_MAGIC: [u8; 8] = *b"NWPCF001";
pub const PHASE_CENTER_RUNTIME_PACKAGE_FINGERPRINT_PERSONAL: [u8; 8] = *b"nwpcpkg1";
pub const PHASE_CENTER_RUNTIME_PACKAGE_HEADER_BYTES: usize = 16;
pub const PHASE_CENTER_DEFAULT_OFFLOAD_MARGIN_THRESHOLD_MICRO: i64 = 300_000;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PhaseCenterCell {
    pub re: f64,
    pub im: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterFlatRecord {
    pub positive_center: Box<[PhaseCenterCell]>,
    pub negative_center: Box<[PhaseCenterCell]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterFlatRuntime {
    cells: usize,
    records: Box<[PhaseCenterFlatRecord]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterOffloadRuntime {
    runtime: PhaseCenterFlatRuntime,
    policy: PhaseCenterOffloadPolicy,
    package_info: PhaseCenterRuntimePackageInfo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterRuntimePackageInfo {
    pub magic: [u8; PHASE_CENTER_RUNTIME_PACKAGE_MAGIC.len()],
    pub cells: usize,
    pub record_count: usize,
    pub serialized_len: usize,
    pub payload_bytes: usize,
    pub fingerprint64: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterCompiler {
    cells: usize,
    positive_sums: Vec<Vec<PhaseCenterCell>>,
    negative_sums: Vec<Vec<PhaseCenterCell>>,
    positive_counts: Vec<usize>,
    negative_counts: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterEvalTask {
    pub center_index: usize,
    pub correct_vec: Box<[PhaseCenterCell]>,
    pub wrong_vec: Box<[PhaseCenterCell]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseCenterOffloadAction {
    LocalOperator,
    FallbackToLlm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterOffloadPolicy {
    pub margin_threshold_micro: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterOffloadDecision {
    pub action: PhaseCenterOffloadAction,
    pub margin_micro: i64,
    pub margin_threshold_micro: i64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhaseCenterOffloadSummary {
    pub calls: usize,
    pub local_operator_calls: usize,
    pub fallback_to_llm_calls: usize,
    pub offload_rate_milli: usize,
    pub local_accuracy_milli: usize,
    pub false_local_accepts: usize,
    pub median_margin_micro: i64,
    pub p10_margin_micro: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseCenterRuntimeError {
    EmptyRuntime,
    RecordWidthMismatch,
    CenterIndexOutOfBounds,
    ProgramIndexOutOfBounds,
    IncompleteProgram,
    VectorWidthMismatch,
    RuntimePackageTooLarge,
    InvalidRuntimePackage,
    InvalidOffloadThreshold,
    InvalidMargin,
}

impl PhaseCenterOffloadPolicy {
    pub fn new(margin_threshold_micro: i64) -> Result<Self, PhaseCenterRuntimeError> {
        if margin_threshold_micro <= 0 {
            return Err(PhaseCenterRuntimeError::InvalidOffloadThreshold);
        }
        Ok(Self {
            margin_threshold_micro,
        })
    }

    pub fn default_conservative() -> Self {
        Self {
            margin_threshold_micro: PHASE_CENTER_DEFAULT_OFFLOAD_MARGIN_THRESHOLD_MICRO,
        }
    }

    pub fn decide_margin(
        self,
        margin: f64,
    ) -> Result<PhaseCenterOffloadDecision, PhaseCenterRuntimeError> {
        let margin_micro = phase_margin_to_micro(margin)?;
        let action = if margin_micro >= self.margin_threshold_micro {
            PhaseCenterOffloadAction::LocalOperator
        } else {
            PhaseCenterOffloadAction::FallbackToLlm
        };
        Ok(PhaseCenterOffloadDecision {
            action,
            margin_micro,
            margin_threshold_micro: self.margin_threshold_micro,
        })
    }
}

impl Default for PhaseCenterOffloadPolicy {
    fn default() -> Self {
        Self::default_conservative()
    }
}

impl PhaseCenterOffloadRuntime {
    pub fn inspect_package_bytes(
        bytes: &[u8],
    ) -> Result<PhaseCenterRuntimePackageInfo, PhaseCenterRuntimeError> {
        PhaseCenterFlatRuntime::inspect_bytes(bytes)
    }

    pub fn from_package_bytes(
        bytes: &[u8],
        policy: PhaseCenterOffloadPolicy,
    ) -> Result<Self, PhaseCenterRuntimeError> {
        let package_info = Self::inspect_package_bytes(bytes)?;
        let runtime = PhaseCenterFlatRuntime::from_bytes(bytes)?;
        Ok(Self {
            runtime,
            policy,
            package_info,
        })
    }

    #[must_use]
    pub const fn policy(&self) -> PhaseCenterOffloadPolicy {
        self.policy
    }

    #[must_use]
    pub const fn package_info(&self) -> PhaseCenterRuntimePackageInfo {
        self.package_info
    }

    #[must_use]
    pub const fn runtime(&self) -> &PhaseCenterFlatRuntime {
        &self.runtime
    }

    #[must_use]
    pub const fn cells(&self) -> usize {
        self.runtime.cells()
    }

    #[must_use]
    pub fn record_count(&self) -> usize {
        self.runtime.record_count()
    }

    #[must_use]
    pub fn bytes_estimate(&self) -> usize {
        self.runtime.bytes_estimate()
    }

    pub fn offload_decision(
        &self,
        task: &PhaseCenterEvalTask,
    ) -> Result<PhaseCenterOffloadDecision, PhaseCenterRuntimeError> {
        self.runtime.offload_decision(task, self.policy)
    }

    pub fn offload_decisions_into<'a, I>(
        &self,
        tasks: I,
        out: &mut Vec<PhaseCenterOffloadDecision>,
    ) -> Result<(), PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = &'a PhaseCenterEvalTask>,
    {
        self.runtime.offload_decisions_into(tasks, self.policy, out)
    }

    pub fn offload_summary_into<'a, I>(
        &self,
        tasks: I,
        decision_scratch: &mut Vec<PhaseCenterOffloadDecision>,
        margin_scratch: &mut Vec<i64>,
    ) -> Result<PhaseCenterOffloadSummary, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = &'a PhaseCenterEvalTask>,
    {
        self.runtime
            .offload_summary_into(tasks, self.policy, decision_scratch, margin_scratch)
    }

    pub fn offload_summary_for_into<'a, I>(
        &self,
        tasks: I,
        decision_scratch: &mut Vec<PhaseCenterOffloadDecision>,
        margin_scratch: &mut Vec<i64>,
    ) -> Result<PhaseCenterOffloadSummary, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = (usize, &'a [PhaseCenterCell], &'a [PhaseCenterCell])>,
    {
        self.runtime
            .offload_summary_for_into(tasks, self.policy, decision_scratch, margin_scratch)
    }
}

impl PhaseCenterOffloadDecision {
    #[must_use]
    pub const fn is_local_operator(self) -> bool {
        matches!(self.action, PhaseCenterOffloadAction::LocalOperator)
    }

    #[must_use]
    pub const fn is_fallback_to_llm(self) -> bool {
        matches!(self.action, PhaseCenterOffloadAction::FallbackToLlm)
    }

    #[must_use]
    pub const fn is_false_local_accept(self) -> bool {
        self.is_local_operator() && self.margin_micro <= 0
    }
}

impl PhaseCenterOffloadSummary {
    #[must_use]
    pub fn from_decisions<I>(decisions: I) -> Self
    where
        I: IntoIterator<Item = PhaseCenterOffloadDecision>,
    {
        let decisions = decisions.into_iter().collect::<Vec<_>>();
        Self::from_decision_slice(&decisions)
    }

    #[must_use]
    pub fn from_repeated_decisions<I>(decisions: I, calls: usize) -> Self
    where
        I: IntoIterator<Item = PhaseCenterOffloadDecision>,
    {
        let decisions = decisions.into_iter().collect::<Vec<_>>();
        Self::from_repeated_decision_slice(&decisions, calls)
    }

    #[must_use]
    pub fn from_decision_slice(decisions: &[PhaseCenterOffloadDecision]) -> Self {
        let mut margin_scratch = Vec::new();
        Self::from_decision_slice_into(decisions, &mut margin_scratch)
    }

    #[must_use]
    pub fn from_decision_slice_into(
        decisions: &[PhaseCenterOffloadDecision],
        margin_scratch: &mut Vec<i64>,
    ) -> Self {
        Self::from_repeated_decision_slice_into(decisions, decisions.len(), margin_scratch)
    }

    #[must_use]
    pub fn from_repeated_decision_slice(
        decisions: &[PhaseCenterOffloadDecision],
        calls: usize,
    ) -> Self {
        let mut margin_scratch = Vec::new();
        Self::from_repeated_decision_slice_into(decisions, calls, &mut margin_scratch)
    }

    #[must_use]
    pub fn from_repeated_decision_slice_into(
        decisions: &[PhaseCenterOffloadDecision],
        calls: usize,
        margin_scratch: &mut Vec<i64>,
    ) -> Self {
        Self::from_repeated_decision_fn_into(
            decisions.len(),
            calls,
            |index| decisions[index],
            margin_scratch,
        )
    }

    #[must_use]
    pub fn from_repeated_decision_fn_into<F>(
        decision_count: usize,
        calls: usize,
        decision_at: F,
        margin_scratch: &mut Vec<i64>,
    ) -> Self
    where
        F: Fn(usize) -> PhaseCenterOffloadDecision,
    {
        summarize_repeated_offload_decisions_into(
            decision_count,
            calls,
            decision_at,
            margin_scratch,
        )
    }
}

impl PhaseCenterCompiler {
    pub fn new(cells: usize, program_count: usize) -> Result<Self, PhaseCenterRuntimeError> {
        if cells == 0 || program_count == 0 {
            return Err(PhaseCenterRuntimeError::EmptyRuntime);
        }
        Ok(Self {
            cells,
            positive_sums: vec![vec![PhaseCenterCell::default(); cells]; program_count],
            negative_sums: vec![vec![PhaseCenterCell::default(); cells]; program_count],
            positive_counts: vec![0; program_count],
            negative_counts: vec![0; program_count],
        })
    }

    #[must_use]
    pub const fn cells(&self) -> usize {
        self.cells
    }

    #[must_use]
    pub fn program_count(&self) -> usize {
        self.positive_sums.len()
    }

    pub fn add_positive_atoms<'a, I>(
        &mut self,
        program_index: usize,
        atoms: I,
    ) -> Result<(), PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let vector = phase_vector_from_atoms(atoms, self.cells);
        self.add_positive_vector(program_index, &vector)
    }

    pub fn add_negative_atoms<'a, I>(
        &mut self,
        program_index: usize,
        atoms: I,
    ) -> Result<(), PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let vector = phase_vector_from_atoms(atoms, self.cells);
        self.add_negative_vector(program_index, &vector)
    }

    pub fn add_positive_vector(
        &mut self,
        program_index: usize,
        vector: &[PhaseCenterCell],
    ) -> Result<(), PhaseCenterRuntimeError> {
        self.add_vector(program_index, vector, true)
    }

    pub fn add_negative_vector(
        &mut self,
        program_index: usize,
        vector: &[PhaseCenterCell],
    ) -> Result<(), PhaseCenterRuntimeError> {
        self.add_vector(program_index, vector, false)
    }

    fn add_vector(
        &mut self,
        program_index: usize,
        vector: &[PhaseCenterCell],
        is_positive: bool,
    ) -> Result<(), PhaseCenterRuntimeError> {
        if vector.len() != self.cells {
            return Err(PhaseCenterRuntimeError::VectorWidthMismatch);
        }
        let Some(sum) = (if is_positive {
            self.positive_sums.get_mut(program_index)
        } else {
            self.negative_sums.get_mut(program_index)
        }) else {
            return Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds);
        };
        add_phase_vector(sum, vector, 1.0);
        if is_positive {
            self.positive_counts[program_index] += 1;
        } else {
            self.negative_counts[program_index] += 1;
        }
        Ok(())
    }

    pub fn compile(self) -> Result<PhaseCenterFlatRuntime, PhaseCenterRuntimeError> {
        if self
            .positive_counts
            .iter()
            .zip(self.negative_counts.iter())
            .any(|(positive, negative)| *positive == 0 || *negative == 0)
        {
            return Err(PhaseCenterRuntimeError::IncompleteProgram);
        }

        let records = self
            .positive_sums
            .into_iter()
            .zip(self.negative_sums)
            .map(|(positive_sum, negative_sum)| PhaseCenterFlatRecord {
                positive_center: phase_center_from_sum(&positive_sum).into_boxed_slice(),
                negative_center: phase_center_from_sum(&negative_sum).into_boxed_slice(),
            })
            .collect::<Vec<_>>();
        PhaseCenterFlatRuntime::new(self.cells, records)
    }
}

impl PhaseCenterFlatRuntime {
    pub fn new(
        cells: usize,
        records: Vec<PhaseCenterFlatRecord>,
    ) -> Result<Self, PhaseCenterRuntimeError> {
        if cells == 0 || records.is_empty() {
            return Err(PhaseCenterRuntimeError::EmptyRuntime);
        }
        if records.iter().any(|record| {
            record.positive_center.len() != cells || record.negative_center.len() != cells
        }) {
            return Err(PhaseCenterRuntimeError::RecordWidthMismatch);
        }
        Ok(Self {
            cells,
            records: records.into_boxed_slice(),
        })
    }

    #[must_use]
    pub const fn cells(&self) -> usize {
        self.cells
    }

    #[must_use]
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    #[must_use]
    pub fn bytes_estimate(&self) -> usize {
        self.records.len() * 2 * self.cells * std::mem::size_of::<PhaseCenterCell>()
            + self.records.len() * std::mem::size_of::<PhaseCenterFlatRecord>()
    }

    #[must_use]
    pub fn serialized_len(&self) -> usize {
        runtime_package_len(self.cells, self.records.len()).unwrap_or(usize::MAX)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, PhaseCenterRuntimeError> {
        let cells = u32::try_from(self.cells)
            .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        let records = u32::try_from(self.records.len())
            .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        let serialized_len = runtime_package_len(self.cells, self.records.len())
            .ok_or(PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        let mut bytes = Vec::with_capacity(serialized_len);
        bytes.extend_from_slice(&PHASE_CENTER_RUNTIME_PACKAGE_MAGIC);
        bytes.extend_from_slice(&cells.to_le_bytes());
        bytes.extend_from_slice(&records.to_le_bytes());
        for record in self.records.iter() {
            write_phase_center_cells(&mut bytes, &record.positive_center);
            write_phase_center_cells(&mut bytes, &record.negative_center);
        }
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PhaseCenterRuntimeError> {
        let info = Self::inspect_bytes(bytes)?;

        let mut offset = PHASE_CENTER_RUNTIME_PACKAGE_HEADER_BYTES;
        let mut records = Vec::with_capacity(info.record_count);
        for _ in 0..info.record_count {
            let positive_center = read_phase_center_cells(bytes, &mut offset, info.cells)?;
            let negative_center = read_phase_center_cells(bytes, &mut offset, info.cells)?;
            records.push(PhaseCenterFlatRecord {
                positive_center,
                negative_center,
            });
        }
        PhaseCenterFlatRuntime::new(info.cells, records)
    }

    pub fn inspect_bytes(
        bytes: &[u8],
    ) -> Result<PhaseCenterRuntimePackageInfo, PhaseCenterRuntimeError> {
        if bytes.len() < PHASE_CENTER_RUNTIME_PACKAGE_HEADER_BYTES {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        if bytes[..PHASE_CENTER_RUNTIME_PACKAGE_MAGIC.len()] != PHASE_CENTER_RUNTIME_PACKAGE_MAGIC {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        let cells = read_u32_le(bytes, 8)? as usize;
        let record_count = read_u32_le(bytes, 12)? as usize;
        if cells == 0 || record_count == 0 {
            return Err(PhaseCenterRuntimeError::EmptyRuntime);
        }
        let serialized_len = runtime_package_len(cells, record_count)
            .ok_or(PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        if bytes.len() != serialized_len {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        Ok(PhaseCenterRuntimePackageInfo {
            magic: PHASE_CENTER_RUNTIME_PACKAGE_MAGIC,
            cells,
            record_count,
            serialized_len,
            payload_bytes: serialized_len - PHASE_CENTER_RUNTIME_PACKAGE_HEADER_BYTES,
            fingerprint64: runtime_package_fingerprint64(bytes),
        })
    }

    pub fn margin(&self, task: &PhaseCenterEvalTask) -> Result<f64, PhaseCenterRuntimeError> {
        self.margin_for(task.center_index, &task.correct_vec, &task.wrong_vec)
    }

    pub fn offload_decision(
        &self,
        task: &PhaseCenterEvalTask,
        policy: PhaseCenterOffloadPolicy,
    ) -> Result<PhaseCenterOffloadDecision, PhaseCenterRuntimeError> {
        let margin = self.margin(task)?;
        policy.decide_margin(margin)
    }

    pub fn offload_decisions<'a, I>(
        &self,
        tasks: I,
        policy: PhaseCenterOffloadPolicy,
    ) -> Result<Vec<PhaseCenterOffloadDecision>, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = &'a PhaseCenterEvalTask>,
    {
        let mut out = Vec::new();
        self.offload_decisions_into(tasks, policy, &mut out)?;
        Ok(out)
    }

    pub fn offload_decisions_into<'a, I>(
        &self,
        tasks: I,
        policy: PhaseCenterOffloadPolicy,
        out: &mut Vec<PhaseCenterOffloadDecision>,
    ) -> Result<(), PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = &'a PhaseCenterEvalTask>,
    {
        out.clear();
        for task in tasks {
            out.push(self.offload_decision(task, policy)?);
        }
        Ok(())
    }

    pub fn offload_summary_into<'a, I>(
        &self,
        tasks: I,
        policy: PhaseCenterOffloadPolicy,
        decision_scratch: &mut Vec<PhaseCenterOffloadDecision>,
        margin_scratch: &mut Vec<i64>,
    ) -> Result<PhaseCenterOffloadSummary, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = &'a PhaseCenterEvalTask>,
    {
        self.offload_decisions_into(tasks, policy, decision_scratch)?;
        Ok(PhaseCenterOffloadSummary::from_decision_slice_into(
            decision_scratch,
            margin_scratch,
        ))
    }

    pub fn margin_for(
        &self,
        center_index: usize,
        correct_vec: &[PhaseCenterCell],
        wrong_vec: &[PhaseCenterCell],
    ) -> Result<f64, PhaseCenterRuntimeError> {
        if correct_vec.len() != self.cells || wrong_vec.len() != self.cells {
            return Err(PhaseCenterRuntimeError::VectorWidthMismatch);
        }
        let Some(record) = self.records.get(center_index) else {
            return Err(PhaseCenterRuntimeError::CenterIndexOutOfBounds);
        };
        Ok(phase_margin_from_centers(
            correct_vec,
            wrong_vec,
            &record.positive_center,
            &record.negative_center,
        ))
    }

    pub fn offload_decision_for(
        &self,
        center_index: usize,
        correct_vec: &[PhaseCenterCell],
        wrong_vec: &[PhaseCenterCell],
        policy: PhaseCenterOffloadPolicy,
    ) -> Result<PhaseCenterOffloadDecision, PhaseCenterRuntimeError> {
        let margin = self.margin_for(center_index, correct_vec, wrong_vec)?;
        policy.decide_margin(margin)
    }

    pub fn offload_decisions_for<'a, I>(
        &self,
        tasks: I,
        policy: PhaseCenterOffloadPolicy,
    ) -> Result<Vec<PhaseCenterOffloadDecision>, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = (usize, &'a [PhaseCenterCell], &'a [PhaseCenterCell])>,
    {
        let mut out = Vec::new();
        self.offload_decisions_for_into(tasks, policy, &mut out)?;
        Ok(out)
    }

    pub fn offload_decisions_for_into<'a, I>(
        &self,
        tasks: I,
        policy: PhaseCenterOffloadPolicy,
        out: &mut Vec<PhaseCenterOffloadDecision>,
    ) -> Result<(), PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = (usize, &'a [PhaseCenterCell], &'a [PhaseCenterCell])>,
    {
        out.clear();
        for (center_index, correct_vec, wrong_vec) in tasks {
            out.push(self.offload_decision_for(center_index, correct_vec, wrong_vec, policy)?);
        }
        Ok(())
    }

    pub fn offload_summary_for_into<'a, I>(
        &self,
        tasks: I,
        policy: PhaseCenterOffloadPolicy,
        decision_scratch: &mut Vec<PhaseCenterOffloadDecision>,
        margin_scratch: &mut Vec<i64>,
    ) -> Result<PhaseCenterOffloadSummary, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = (usize, &'a [PhaseCenterCell], &'a [PhaseCenterCell])>,
    {
        self.offload_decisions_for_into(tasks, policy, decision_scratch)?;
        Ok(PhaseCenterOffloadSummary::from_decision_slice_into(
            decision_scratch,
            margin_scratch,
        ))
    }
}

#[must_use]
pub fn phase_margin_from_centers(
    correct_vec: &[PhaseCenterCell],
    wrong_vec: &[PhaseCenterCell],
    positive_center: &[PhaseCenterCell],
    negative_center: &[PhaseCenterCell],
) -> f64 {
    if correct_vec.len() == wrong_vec.len()
        && correct_vec.len() == positive_center.len()
        && correct_vec.len() == negative_center.len()
    {
        if correct_vec.is_empty() {
            return 0.0;
        }
        let mut score = 0.0f64;
        for (((correct, wrong), positive), negative) in correct_vec
            .iter()
            .zip(wrong_vec.iter())
            .zip(positive_center.iter())
            .zip(negative_center.iter())
        {
            let vector_delta_re = correct.re - wrong.re;
            let vector_delta_im = correct.im - wrong.im;
            let center_delta_re = positive.re - negative.re;
            let center_delta_im = positive.im - negative.im;
            score += vector_delta_re * center_delta_re + vector_delta_im * center_delta_im;
        }
        return score / correct_vec.len() as f64;
    }

    let correct_pos = phase_coherence(correct_vec, positive_center);
    let wrong_pos = phase_coherence(wrong_vec, positive_center);
    let correct_neg = phase_coherence(correct_vec, negative_center);
    let wrong_neg = phase_coherence(wrong_vec, negative_center);
    (correct_pos - correct_neg) - (wrong_pos - wrong_neg)
}

pub fn phase_margin_to_micro(margin: f64) -> Result<i64, PhaseCenterRuntimeError> {
    if !margin.is_finite() {
        return Err(PhaseCenterRuntimeError::InvalidMargin);
    }
    let scaled = (margin * 1_000_000.0).round();
    if scaled < i64::MIN as f64 || scaled > i64::MAX as f64 {
        return Err(PhaseCenterRuntimeError::InvalidMargin);
    }
    Ok(scaled as i64)
}

#[must_use]
fn summarize_repeated_offload_decisions_into<F>(
    decision_count: usize,
    calls: usize,
    decision_at: F,
    margin_scratch: &mut Vec<i64>,
) -> PhaseCenterOffloadSummary
where
    F: Fn(usize) -> PhaseCenterOffloadDecision,
{
    margin_scratch.clear();
    if decision_count == 0 || calls == 0 {
        return PhaseCenterOffloadSummary::default();
    }

    if margin_scratch.capacity() < calls {
        margin_scratch.reserve(calls);
    }
    let mut local_operator_calls = 0usize;
    let mut fallback_to_llm_calls = 0usize;
    let mut false_local_accepts = 0usize;
    for call_index in 0..calls {
        let decision = decision_at(call_index % decision_count);
        margin_scratch.push(decision.margin_micro);
        if decision.is_local_operator() {
            local_operator_calls += 1;
            if decision.is_false_local_accept() {
                false_local_accepts += 1;
            }
        } else {
            fallback_to_llm_calls += 1;
        }
    }
    margin_scratch.sort_unstable();
    let local_correct = local_operator_calls.saturating_sub(false_local_accepts);
    PhaseCenterOffloadSummary {
        calls,
        local_operator_calls,
        fallback_to_llm_calls,
        offload_rate_milli: phase_center_milli_ratio(local_operator_calls, calls),
        local_accuracy_milli: phase_center_milli_ratio(local_correct, local_operator_calls),
        false_local_accepts,
        median_margin_micro: phase_center_percentile_i64(margin_scratch, 50),
        p10_margin_micro: phase_center_percentile_i64(margin_scratch, 10),
    }
}

#[must_use]
fn phase_center_milli_ratio(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    (numerator * 1000 + denominator / 2) / denominator
}

#[must_use]
fn phase_center_percentile_i64(sorted: &[i64], percentile: usize) -> i64 {
    if sorted.is_empty() {
        return 0;
    }
    let index = (sorted.len() - 1) * percentile / 100;
    sorted[index]
}

#[must_use]
pub fn phase_coherence(vector: &[PhaseCenterCell], center: &[PhaseCenterCell]) -> f64 {
    if vector.is_empty() || center.is_empty() {
        return 0.0;
    }
    let mut active = 0usize;
    let mut score = 0.0f64;
    for (value, center) in vector.iter().zip(center.iter()) {
        active += 1;
        score += value.re * center.re + value.im * center.im;
    }
    if active == 0 {
        0.0
    } else {
        score / active as f64
    }
}

#[must_use]
pub fn phase_center_from_sum(values: &[PhaseCenterCell]) -> Vec<PhaseCenterCell> {
    values
        .iter()
        .map(|value| phase_circular_unit(*value))
        .collect()
}

pub fn add_phase_vector(target: &mut [PhaseCenterCell], source: &[PhaseCenterCell], sign: f64) {
    for (target_cell, source_cell) in target.iter_mut().zip(source.iter()) {
        target_cell.re += sign * source_cell.re;
        target_cell.im += sign * source_cell.im;
    }
}

#[must_use]
pub fn phase_vector_from_atoms<'a, I>(atoms: I, cells: usize) -> Vec<PhaseCenterCell>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut sums = vec![PhaseCenterCell::default(); cells];
    for atom in atoms {
        for (cell, sum) in sums.iter_mut().enumerate() {
            let phase = stable_phase_cell(atom, cell);
            sum.re += phase.re;
            sum.im += phase.im;
        }
    }
    phase_center_from_sum(&sums)
}

#[must_use]
pub fn stable_phase_cell(atom: &str, cell: usize) -> PhaseCenterCell {
    let input = format!("{cell}\0{atom}");
    let hash = blake2b8_personalized(input.as_bytes(), b"nwphase");
    let angle = (hash as f64 / (u64::MAX as f64 + 1.0)) * std::f64::consts::TAU;
    PhaseCenterCell {
        re: angle.cos(),
        im: angle.sin(),
    }
}

#[must_use]
pub fn phase_circular_unit(value: PhaseCenterCell) -> PhaseCenterCell {
    let magnitude = (value.re * value.re + value.im * value.im).sqrt();
    if magnitude == 0.0 {
        PhaseCenterCell::default()
    } else {
        PhaseCenterCell {
            re: value.re / magnitude,
            im: value.im / magnitude,
        }
    }
}

fn blake2b8_personalized(input: &[u8], personal: &[u8]) -> u64 {
    const IV: [u64; 8] = [
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d1,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
    ];
    let mut h = IV;
    h[0] ^= 0x01010008;
    h[6] ^= le_u64_padded(personal, 0);
    h[7] ^= le_u64_padded(personal, 8);

    let mut offset = 0usize;
    while offset < input.len() || (input.is_empty() && offset == 0) {
        let remaining = input.len().saturating_sub(offset);
        let block_len = remaining.min(128);
        let mut block = [0u8; 128];
        if block_len > 0 {
            block[..block_len].copy_from_slice(&input[offset..offset + block_len]);
        }
        offset += block_len;
        let is_last = offset >= input.len();
        blake2b_compress(&mut h, &block, offset as u128, is_last);
        if is_last {
            break;
        }
    }
    h[0]
}

fn le_u64_padded(bytes: &[u8], start: usize) -> u64 {
    let mut out = [0u8; 8];
    for (dst, src) in out.iter_mut().zip(bytes.iter().skip(start).take(8)) {
        *dst = *src;
    }
    u64::from_le_bytes(out)
}

fn blake2b_compress(h: &mut [u64; 8], block: &[u8; 128], counter: u128, is_last: bool) {
    const IV: [u64; 8] = [
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d1,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
    ];
    const SIGMA: [[usize; 16]; 12] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
        [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
        [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
        [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
        [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
        [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
        [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
        [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
        [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    ];

    let mut m = [0u64; 16];
    for (index, chunk) in block.chunks_exact(8).enumerate() {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(chunk);
        m[index] = u64::from_le_bytes(bytes);
    }

    let mut v = [0u64; 16];
    v[..8].copy_from_slice(h);
    v[8..].copy_from_slice(&IV);
    v[12] ^= counter as u64;
    v[13] ^= (counter >> 64) as u64;
    if is_last {
        v[14] = !v[14];
    }

    for schedule in SIGMA {
        blake2b_g(&mut v, 0, 4, 8, 12, m[schedule[0]], m[schedule[1]]);
        blake2b_g(&mut v, 1, 5, 9, 13, m[schedule[2]], m[schedule[3]]);
        blake2b_g(&mut v, 2, 6, 10, 14, m[schedule[4]], m[schedule[5]]);
        blake2b_g(&mut v, 3, 7, 11, 15, m[schedule[6]], m[schedule[7]]);
        blake2b_g(&mut v, 0, 5, 10, 15, m[schedule[8]], m[schedule[9]]);
        blake2b_g(&mut v, 1, 6, 11, 12, m[schedule[10]], m[schedule[11]]);
        blake2b_g(&mut v, 2, 7, 8, 13, m[schedule[12]], m[schedule[13]]);
        blake2b_g(&mut v, 3, 4, 9, 14, m[schedule[14]], m[schedule[15]]);
    }

    for index in 0..8 {
        h[index] ^= v[index] ^ v[index + 8];
    }
}

fn blake2b_g(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
    v[d] = (v[d] ^ v[a]).rotate_right(32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(24);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(63);
}

fn runtime_package_len(cells: usize, records: usize) -> Option<usize> {
    records
        .checked_mul(2)?
        .checked_mul(cells)?
        .checked_mul(2)?
        .checked_mul(std::mem::size_of::<f64>())?
        .checked_add(PHASE_CENTER_RUNTIME_PACKAGE_HEADER_BYTES)
}

fn runtime_package_fingerprint64(bytes: &[u8]) -> u64 {
    blake2b8_personalized(bytes, &PHASE_CENTER_RUNTIME_PACKAGE_FINGERPRINT_PERSONAL)
}

fn write_phase_center_cells(bytes: &mut Vec<u8>, cells: &[PhaseCenterCell]) {
    for cell in cells {
        bytes.extend_from_slice(&cell.re.to_le_bytes());
        bytes.extend_from_slice(&cell.im.to_le_bytes());
    }
}

fn read_phase_center_cells(
    bytes: &[u8],
    offset: &mut usize,
    cells: usize,
) -> Result<Box<[PhaseCenterCell]>, PhaseCenterRuntimeError> {
    let mut out = Vec::with_capacity(cells);
    for _ in 0..cells {
        let re = read_f64_le(bytes, *offset)?;
        *offset += std::mem::size_of::<f64>();
        let im = read_f64_le(bytes, *offset)?;
        *offset += std::mem::size_of::<f64>();
        out.push(PhaseCenterCell { re, im });
    }
    Ok(out.into_boxed_slice())
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, PhaseCenterRuntimeError> {
    let chunk = bytes
        .get(offset..offset + std::mem::size_of::<u32>())
        .ok_or(PhaseCenterRuntimeError::InvalidRuntimePackage)?;
    let mut out = [0u8; 4];
    out.copy_from_slice(chunk);
    Ok(u32::from_le_bytes(out))
}

fn read_f64_le(bytes: &[u8], offset: usize) -> Result<f64, PhaseCenterRuntimeError> {
    let chunk = bytes
        .get(offset..offset + std::mem::size_of::<f64>())
        .ok_or(PhaseCenterRuntimeError::InvalidRuntimePackage)?;
    let mut out = [0u8; 8];
    out.copy_from_slice(chunk);
    Ok(f64::from_le_bytes(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_hash_is_unit_and_deterministic() {
        let a = stable_phase_cell("rel:o0:s1", 7);
        let b = stable_phase_cell("rel:o0:s1", 7);
        let magnitude = (a.re * a.re + a.im * a.im).sqrt();
        assert_eq!(a, b);
        assert!((magnitude - 1.0).abs() < 1e-12);
    }

    #[test]
    fn runtime_scores_correct_transition_above_wrong() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let task = PhaseCenterEvalTask {
            center_index: 0,
            correct_vec: positive.into_boxed_slice(),
            wrong_vec: negative.into_boxed_slice(),
        };
        assert!(runtime.margin(&task).expect("valid task") > 0.0);
    }

    #[test]
    fn offload_policy_rejects_invalid_threshold() {
        assert_eq!(
            PhaseCenterOffloadPolicy::new(0),
            Err(PhaseCenterRuntimeError::InvalidOffloadThreshold)
        );
        assert_eq!(
            PhaseCenterOffloadPolicy::new(-1),
            Err(PhaseCenterRuntimeError::InvalidOffloadThreshold)
        );
    }

    #[test]
    fn offload_policy_routes_by_margin_micro_threshold() {
        let policy = PhaseCenterOffloadPolicy::new(300_000).expect("valid threshold");
        let local = policy.decide_margin(0.3004).expect("finite margin");
        let fallback = policy.decide_margin(0.2994).expect("finite margin");
        assert_eq!(local.margin_micro, 300_400);
        assert_eq!(local.action, PhaseCenterOffloadAction::LocalOperator);
        assert!(local.is_local_operator());
        assert_eq!(fallback.margin_micro, 299_400);
        assert_eq!(fallback.action, PhaseCenterOffloadAction::FallbackToLlm);
        assert!(fallback.is_fallback_to_llm());
    }

    #[test]
    fn offload_policy_rejects_nonfinite_margin() {
        let policy = PhaseCenterOffloadPolicy::default_conservative();
        assert_eq!(
            policy.decide_margin(f64::NAN),
            Err(PhaseCenterRuntimeError::InvalidMargin)
        );
        assert_eq!(
            phase_margin_to_micro(f64::INFINITY),
            Err(PhaseCenterRuntimeError::InvalidMargin)
        );
    }

    #[test]
    fn runtime_offload_decision_uses_packaged_margin() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        let task = PhaseCenterEvalTask {
            center_index: 0,
            correct_vec: positive.into_boxed_slice(),
            wrong_vec: negative.into_boxed_slice(),
        };
        let decision = runtime
            .offload_decision(&task, policy)
            .expect("valid offload decision");
        assert!(decision.is_local_operator());
        assert!(decision.margin_micro > 0);
    }

    #[test]
    fn runtime_offload_decisions_batch_matches_per_task_decisions() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let tasks = vec![
            PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: positive.clone().into_boxed_slice(),
                wrong_vec: negative.clone().into_boxed_slice(),
            },
            PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: negative.clone().into_boxed_slice(),
                wrong_vec: positive.clone().into_boxed_slice(),
            },
        ];
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        let batch = runtime
            .offload_decisions(&tasks, policy)
            .expect("valid batch decisions");
        let per_task = tasks
            .iter()
            .map(|task| {
                runtime
                    .offload_decision(task, policy)
                    .expect("valid per-task decision")
            })
            .collect::<Vec<_>>();
        assert_eq!(batch, per_task);
        assert!(batch[0].is_local_operator());
        assert!(batch[1].is_fallback_to_llm());
    }

    #[test]
    fn runtime_offload_decisions_into_reuses_caller_buffer() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let tasks = vec![
            PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: positive.clone().into_boxed_slice(),
                wrong_vec: negative.clone().into_boxed_slice(),
            },
            PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: negative.clone().into_boxed_slice(),
                wrong_vec: positive.clone().into_boxed_slice(),
            },
        ];
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        let expected = runtime
            .offload_decisions(&tasks, policy)
            .expect("valid batch decisions");
        let mut out = Vec::with_capacity(8);
        let original_capacity = out.capacity();
        runtime
            .offload_decisions_into(&tasks, policy, &mut out)
            .expect("valid reused-buffer batch decisions");
        assert_eq!(out, expected);
        assert_eq!(out.capacity(), original_capacity);

        runtime
            .offload_decisions_into(tasks.iter().take(1), policy, &mut out)
            .expect("valid shorter reused-buffer batch decisions");
        assert_eq!(out.len(), 1);
        assert_eq!(out.capacity(), original_capacity);
    }

    #[test]
    fn runtime_offload_decisions_for_batch_reports_first_error() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let valid_task = (0, positive.as_slice(), negative.as_slice());
        let invalid_width = (0, positive[..7].as_ref(), negative.as_slice());
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        assert_eq!(
            runtime.offload_decisions_for([valid_task, invalid_width], policy),
            Err(PhaseCenterRuntimeError::VectorWidthMismatch)
        );
    }

    #[test]
    fn runtime_offload_decisions_for_into_reuses_buffer_and_reports_error() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        let mut out = Vec::with_capacity(4);
        let original_capacity = out.capacity();
        runtime
            .offload_decisions_for_into(
                [(0, positive.as_slice(), negative.as_slice())],
                policy,
                &mut out,
            )
            .expect("valid raw-slice batch decisions");
        assert_eq!(out.len(), 1);
        assert_eq!(out.capacity(), original_capacity);
        assert!(out[0].is_local_operator());

        assert_eq!(
            runtime.offload_decisions_for_into(
                [(0, positive[..7].as_ref(), negative.as_slice())],
                policy,
                &mut out,
            ),
            Err(PhaseCenterRuntimeError::VectorWidthMismatch)
        );
        assert!(out.is_empty());
        assert_eq!(out.capacity(), original_capacity);
    }

    #[test]
    fn runtime_offload_summary_into_reuses_caller_buffers() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let tasks = vec![
            PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: positive.clone().into_boxed_slice(),
                wrong_vec: negative.clone().into_boxed_slice(),
            },
            PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: negative.clone().into_boxed_slice(),
                wrong_vec: positive.clone().into_boxed_slice(),
            },
        ];
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        let expected_decisions = runtime
            .offload_decisions(&tasks, policy)
            .expect("valid batch decisions");
        let expected_summary = PhaseCenterOffloadSummary::from_decision_slice(&expected_decisions);
        let mut decision_scratch = Vec::with_capacity(8);
        let mut margin_scratch = Vec::with_capacity(8);
        let decision_capacity = decision_scratch.capacity();
        let margin_capacity = margin_scratch.capacity();

        let summary = runtime
            .offload_summary_into(&tasks, policy, &mut decision_scratch, &mut margin_scratch)
            .expect("valid summary");

        assert_eq!(decision_scratch, expected_decisions);
        assert_eq!(summary, expected_summary);
        assert_eq!(decision_scratch.capacity(), decision_capacity);
        assert_eq!(margin_scratch.capacity(), margin_capacity);
    }

    #[test]
    fn runtime_offload_summary_for_into_reuses_caller_buffers() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        let mut decision_scratch = Vec::with_capacity(4);
        let mut margin_scratch = Vec::with_capacity(4);
        let decision_capacity = decision_scratch.capacity();
        let margin_capacity = margin_scratch.capacity();
        let summary = runtime
            .offload_summary_for_into(
                [
                    (0, positive.as_slice(), negative.as_slice()),
                    (0, negative.as_slice(), positive.as_slice()),
                ],
                policy,
                &mut decision_scratch,
                &mut margin_scratch,
            )
            .expect("valid raw-slice summary");

        assert_eq!(summary.calls, 2);
        assert_eq!(summary.local_operator_calls, 1);
        assert_eq!(summary.fallback_to_llm_calls, 1);
        assert_eq!(decision_scratch.capacity(), decision_capacity);
        assert_eq!(margin_scratch.capacity(), margin_capacity);
    }

    #[test]
    fn offload_runtime_from_package_bytes_reuses_caller_buffers() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let bytes = runtime.to_bytes().expect("runtime serializes");
        let package_info =
            PhaseCenterOffloadRuntime::inspect_package_bytes(&bytes).expect("sdk inspects");
        assert_eq!(
            package_info,
            PhaseCenterFlatRuntime::inspect_bytes(&bytes).expect("runtime inspects")
        );
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        let offload_runtime =
            PhaseCenterOffloadRuntime::from_package_bytes(&bytes, policy).expect("sdk loads");
        assert_eq!(offload_runtime.package_info(), package_info);
        assert_eq!(offload_runtime.policy(), policy);
        assert_eq!(offload_runtime.cells(), 8);
        assert_eq!(offload_runtime.record_count(), 1);
        assert_eq!(offload_runtime.bytes_estimate(), runtime.bytes_estimate());
        assert_eq!(
            offload_runtime.runtime().record_count(),
            runtime.record_count()
        );

        let tasks = vec![
            PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: positive.clone().into_boxed_slice(),
                wrong_vec: negative.clone().into_boxed_slice(),
            },
            PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: negative.clone().into_boxed_slice(),
                wrong_vec: positive.clone().into_boxed_slice(),
            },
        ];
        let expected_summary = runtime
            .offload_summary_into(
                &tasks,
                policy,
                &mut Vec::with_capacity(2),
                &mut Vec::with_capacity(2),
            )
            .expect("runtime summary");
        let mut decision_scratch = Vec::with_capacity(4);
        let mut margin_scratch = Vec::with_capacity(4);
        let decision_capacity = decision_scratch.capacity();
        let margin_capacity = margin_scratch.capacity();
        let summary = offload_runtime
            .offload_summary_into(&tasks, &mut decision_scratch, &mut margin_scratch)
            .expect("sdk summary");
        assert_eq!(summary, expected_summary);
        assert_eq!(decision_scratch.capacity(), decision_capacity);
        assert_eq!(margin_scratch.capacity(), margin_capacity);
    }

    #[test]
    fn offload_runtime_rejects_bad_package_bytes() {
        let policy = PhaseCenterOffloadPolicy::default_conservative();
        assert_eq!(
            PhaseCenterOffloadRuntime::inspect_package_bytes(b"bad"),
            Err(PhaseCenterRuntimeError::InvalidRuntimePackage)
        );
        assert_eq!(
            PhaseCenterOffloadRuntime::from_package_bytes(b"bad", policy),
            Err(PhaseCenterRuntimeError::InvalidRuntimePackage)
        );
    }

    #[test]
    fn offload_summary_counts_unique_decisions_and_false_local_accepts() {
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        let local = policy.decide_margin(0.5).expect("finite margin");
        let fallback = policy.decide_margin(-0.1).expect("finite margin");
        let false_local = PhaseCenterOffloadDecision {
            action: PhaseCenterOffloadAction::LocalOperator,
            margin_micro: 0,
            margin_threshold_micro: 1,
        };
        let summary = PhaseCenterOffloadSummary::from_decisions([local, fallback, false_local]);
        assert_eq!(
            summary,
            PhaseCenterOffloadSummary {
                calls: 3,
                local_operator_calls: 2,
                fallback_to_llm_calls: 1,
                offload_rate_milli: 667,
                local_accuracy_milli: 500,
                false_local_accepts: 1,
                median_margin_micro: 0,
                p10_margin_micro: -100_000,
            }
        );
    }

    #[test]
    fn offload_summary_repeats_decision_ring_for_simulated_calls() {
        let policy = PhaseCenterOffloadPolicy::new(300_000).expect("valid policy");
        let local = policy.decide_margin(0.4).expect("finite margin");
        let fallback = policy.decide_margin(0.2).expect("finite margin");
        let summary = PhaseCenterOffloadSummary::from_repeated_decisions([local, fallback], 5);
        assert_eq!(summary.calls, 5);
        assert_eq!(summary.local_operator_calls, 3);
        assert_eq!(summary.fallback_to_llm_calls, 2);
        assert_eq!(summary.offload_rate_milli, 600);
        assert_eq!(summary.local_accuracy_milli, 1000);
        assert_eq!(summary.false_local_accepts, 0);
        assert_eq!(summary.median_margin_micro, 400_000);
        assert_eq!(summary.p10_margin_micro, 200_000);
    }

    #[test]
    fn offload_summary_into_reuses_caller_margin_scratch() {
        let policy = PhaseCenterOffloadPolicy::new(300_000).expect("valid policy");
        let local = policy.decide_margin(0.4).expect("finite margin");
        let fallback = policy.decide_margin(0.2).expect("finite margin");
        let decisions = [local, fallback];
        let mut margin_scratch = Vec::with_capacity(8);
        let original_capacity = margin_scratch.capacity();
        let unique =
            PhaseCenterOffloadSummary::from_decision_slice_into(&decisions, &mut margin_scratch);
        assert_eq!(unique.calls, 2);
        assert_eq!(unique.local_operator_calls, 1);
        assert_eq!(unique.fallback_to_llm_calls, 1);
        assert_eq!(margin_scratch, [200_000, 400_000]);
        assert_eq!(margin_scratch.capacity(), original_capacity);

        let repeated = PhaseCenterOffloadSummary::from_repeated_decision_fn_into(
            decisions.len(),
            5,
            |index| decisions[index],
            &mut margin_scratch,
        );
        assert_eq!(repeated.calls, 5);
        assert_eq!(repeated.local_operator_calls, 3);
        assert_eq!(repeated.fallback_to_llm_calls, 2);
        assert_eq!(repeated.median_margin_micro, 400_000);
        assert_eq!(
            margin_scratch,
            [200_000, 200_000, 400_000, 400_000, 400_000]
        );
        assert_eq!(margin_scratch.capacity(), original_capacity);
    }

    #[test]
    fn compiler_builds_runtime_from_relation_atoms() {
        let mut compiler = PhaseCenterCompiler::new(8, 1).expect("valid compiler");
        compiler
            .add_positive_atoms(0, ["class:order", "rel:o0:s1", "out:o0", "src:s1"])
            .expect("positive atoms accepted");
        compiler
            .add_negative_atoms(0, ["class:order", "rel:o0:s0", "out:o0", "src:s0"])
            .expect("negative atoms accepted");
        let runtime = compiler.compile().expect("complete compiler");
        let correct = phase_vector_from_atoms(["class:order", "rel:o0:s1", "out:o0", "src:s1"], 8);
        let wrong = phase_vector_from_atoms(["class:order", "rel:o0:s0", "out:o0", "src:s0"], 8);
        assert_eq!(runtime.record_count(), 1);
        assert!(
            runtime
                .margin_for(0, &correct, &wrong)
                .expect("valid compiled runtime")
                > 0.0
        );
    }

    #[test]
    fn compiler_rejects_incomplete_programs() {
        let mut compiler = PhaseCenterCompiler::new(8, 1).expect("valid compiler");
        compiler
            .add_positive_atoms(0, ["class:order", "rel:o0:s1"])
            .expect("positive atoms accepted");
        assert_eq!(
            compiler.compile(),
            Err(PhaseCenterRuntimeError::IncompleteProgram)
        );
    }

    #[test]
    fn runtime_package_roundtrip_preserves_margin() {
        let mut compiler = PhaseCenterCompiler::new(8, 1).expect("valid compiler");
        compiler
            .add_positive_atoms(0, ["class:order", "rel:o0:s1", "out:o0", "src:s1"])
            .expect("positive atoms accepted");
        compiler
            .add_negative_atoms(0, ["class:order", "rel:o0:s0", "out:o0", "src:s0"])
            .expect("negative atoms accepted");
        let runtime = compiler.compile().expect("complete compiler");
        let bytes = runtime.to_bytes().expect("runtime serializes");
        let loaded = PhaseCenterFlatRuntime::from_bytes(&bytes).expect("runtime loads");
        let correct = phase_vector_from_atoms(["class:order", "rel:o0:s1", "out:o0", "src:s1"], 8);
        let wrong = phase_vector_from_atoms(["class:order", "rel:o0:s0", "out:o0", "src:s0"], 8);
        assert_eq!(bytes.len(), runtime.serialized_len());
        assert_eq!(loaded.cells(), runtime.cells());
        assert_eq!(loaded.record_count(), runtime.record_count());
        assert_eq!(
            loaded.margin_for(0, &correct, &wrong),
            runtime.margin_for(0, &correct, &wrong)
        );
    }

    #[test]
    fn runtime_package_inspect_reports_header_without_loading_scores() {
        let mut compiler = PhaseCenterCompiler::new(8, 2).expect("valid compiler");
        compiler
            .add_positive_atoms(0, ["class:order", "rel:o0:s1"])
            .expect("positive atoms accepted");
        compiler
            .add_negative_atoms(0, ["class:order", "rel:o0:s0"])
            .expect("negative atoms accepted");
        compiler
            .add_positive_atoms(1, ["class:edit", "rel:o1:s2"])
            .expect("positive atoms accepted");
        compiler
            .add_negative_atoms(1, ["class:edit", "rel:o1:s1"])
            .expect("negative atoms accepted");
        let runtime = compiler.compile().expect("complete compiler");
        let bytes = runtime.to_bytes().expect("runtime serializes");
        let info = PhaseCenterFlatRuntime::inspect_bytes(&bytes).expect("runtime inspects");
        let repeat_info = PhaseCenterFlatRuntime::inspect_bytes(&bytes).expect("runtime inspects");
        let mut mutated_bytes = bytes.clone();
        let last = mutated_bytes.last_mut().expect("package has payload");
        *last ^= 0x01;
        let mutated_info =
            PhaseCenterFlatRuntime::inspect_bytes(&mutated_bytes).expect("runtime inspects");
        assert_eq!(
            info,
            PhaseCenterRuntimePackageInfo {
                magic: PHASE_CENTER_RUNTIME_PACKAGE_MAGIC,
                cells: 8,
                record_count: 2,
                serialized_len: bytes.len(),
                payload_bytes: bytes.len() - PHASE_CENTER_RUNTIME_PACKAGE_HEADER_BYTES,
                fingerprint64: runtime_package_fingerprint64(&bytes),
            }
        );
        assert_ne!(info.fingerprint64, 0);
        assert_eq!(info.fingerprint64, repeat_info.fingerprint64);
        assert_ne!(info.fingerprint64, mutated_info.fingerprint64);
    }

    #[test]
    fn runtime_package_rejects_bad_magic() {
        let mut compiler = PhaseCenterCompiler::new(8, 1).expect("valid compiler");
        compiler
            .add_positive_atoms(0, ["class:order", "rel:o0:s1"])
            .expect("positive atoms accepted");
        compiler
            .add_negative_atoms(0, ["class:order", "rel:o0:s0"])
            .expect("negative atoms accepted");
        let runtime = compiler.compile().expect("complete compiler");
        let mut bytes = runtime.to_bytes().expect("runtime serializes");
        bytes[0] = b'X';
        assert_eq!(
            PhaseCenterFlatRuntime::from_bytes(&bytes),
            Err(PhaseCenterRuntimeError::InvalidRuntimePackage)
        );
    }
}
