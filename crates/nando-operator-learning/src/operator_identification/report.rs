use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperatorIdentificationMetricsV1 {
    pub observations: usize,
    pub verified_passes: usize,
    pub applicability_negatives: usize,
    pub hard_contradictions: usize,
    pub censored: usize,
    pub zero_gain_observations: usize,
    pub total_information_gain: usize,
    pub semantic_classes_remaining: usize,
    pub surviving_programs: usize,
}
