/// Aggregate result for one baseline on the periodic byte task.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaselineResult {
    pub name: &'static str,
    pub correct: usize,
    pub cases: usize,
    pub accuracy: f32,
    pub mean_circular_error: f32,
    pub mean_coherence: f32,
    pub mean_spectral_entropy: f32,
}

impl BaselineResult {
    pub(crate) fn new(name: &'static str, cases: usize) -> Self {
        Self {
            name,
            correct: 0,
            cases,
            accuracy: 0.0,
            mean_circular_error: 0.0,
            mean_coherence: 0.0,
            mean_spectral_entropy: 0.0,
        }
    }

    pub(crate) fn finish(&mut self) {
        let cases = self.cases as f32;
        if cases > 0.0 {
            self.accuracy = self.correct as f32 / cases;
            self.mean_circular_error /= cases;
            self.mean_coherence /= cases;
            self.mean_spectral_entropy /= cases;
        }
    }
}

/// Exact-response result for the first Chat-0 output loop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chat0Result {
    pub name: &'static str,
    pub correct: usize,
    pub cases: usize,
    pub exact_accuracy: f32,
    pub mean_output_len: f32,
}

impl Chat0Result {
    pub(crate) fn new(name: &'static str, cases: usize) -> Self {
        Self {
            name,
            correct: 0,
            cases,
            exact_accuracy: 0.0,
            mean_output_len: 0.0,
        }
    }

    pub(crate) fn score(&mut self, predicted: &'static str, expected: &'static str) {
        self.score_text(predicted, expected);
    }

    pub(crate) fn score_text(&mut self, predicted: &str, expected: &str) {
        if predicted == expected {
            self.correct += 1;
        }
        self.mean_output_len += predicted.len() as f32;
    }

    pub(crate) fn finish(&mut self) {
        let cases = self.cases as f32;
        if cases > 0.0 {
            self.exact_accuracy = self.correct as f32 / cases;
            self.mean_output_len /= cases;
        }
    }
}

pub(crate) fn best_chat0_control<const N: usize>(results: [Chat0Result; N]) -> Chat0Result {
    let mut best = results[0];
    for result in results.into_iter().skip(1) {
        if result.exact_accuracy > best.exact_accuracy {
            best = result;
        }
    }
    best
}

pub(crate) fn format_baseline(result: BaselineResult) -> String {
    format!(
        concat!(
            "{name}.correct: {correct}\n",
            "{name}.cases: {cases}\n",
            "{name}.accuracy: {accuracy:.6}\n",
            "{name}.mean_circular_error: {mean_circular_error:.6}\n",
            "{name}.mean_coherence: {mean_coherence:.6}\n",
            "{name}.mean_spectral_entropy: {mean_spectral_entropy:.6}\n"
        ),
        name = result.name,
        correct = result.correct,
        cases = result.cases,
        accuracy = result.accuracy,
        mean_circular_error = result.mean_circular_error,
        mean_coherence = result.mean_coherence,
        mean_spectral_entropy = result.mean_spectral_entropy
    )
}

pub(crate) fn format_chat0_result(result: Chat0Result) -> String {
    format!(
        concat!(
            "{name}.correct: {correct}\n",
            "{name}.cases: {cases}\n",
            "{name}.exact_accuracy: {exact_accuracy:.6}\n",
            "{name}.mean_output_len: {mean_output_len:.6}\n"
        ),
        name = result.name,
        correct = result.correct,
        cases = result.cases,
        exact_accuracy = result.exact_accuracy,
        mean_output_len = result.mean_output_len
    )
}
