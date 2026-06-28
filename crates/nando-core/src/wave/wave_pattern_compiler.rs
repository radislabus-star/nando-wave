//! WavePattern compiler between raw surface waves and semantic operator waves.
//!
//! Raw text still has no semantic authority here. The compiler first checks a
//! learned surface pattern by wave similarity and a compact Fourier signature,
//! then proposes an EquationForm. Final authority still belongs to the semantic
//! operator wave.

use std::f32::consts::TAU;

use super::{
    SEMANTIC_WAVE_DIM, SemanticAtom, SemanticCandidate, SemanticEquationForm,
    SemanticEquationPrediction, SemanticExtractionStatus, SemanticSchemaKey, SemanticWaveMemory,
    SurfaceWave4096, semantic_label_slot,
};

pub const SURFACE_FOURIER_BINS: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFourierSignature {
    bins: [(f32, f32); SURFACE_FOURIER_BINS],
}

impl SurfaceFourierSignature {
    #[must_use]
    pub fn from_surface_wave(wave: &SurfaceWave4096) -> Self {
        Self::from_i16_lanes(wave.lanes())
    }

    #[must_use]
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        let mut dot = 0.0;
        let mut left_norm = 0.0;
        let mut right_norm = 0.0;
        for ((left_re, left_im), (right_re, right_im)) in self.bins.iter().zip(other.bins.iter()) {
            dot += left_re * right_re + left_im * right_im;
            left_norm += left_re * left_re + left_im * left_im;
            right_norm += right_re * right_re + right_im * right_im;
        }
        if left_norm == 0.0 || right_norm == 0.0 {
            return 0.0;
        }
        dot / (left_norm.sqrt() * right_norm.sqrt())
    }

    fn from_i16_lanes(lanes: &[i16; SEMANTIC_WAVE_DIM]) -> Self {
        let bins = std::array::from_fn(|bin| {
            let frequency = bin + 1;
            let mut re = 0.0;
            let mut im = 0.0;
            for (index, value) in lanes.iter().enumerate() {
                if *value == 0 {
                    continue;
                }
                let theta = TAU * frequency as f32 * index as f32 / SEMANTIC_WAVE_DIM as f32;
                re += f32::from(*value) * theta.cos();
                im -= f32::from(*value) * theta.sin();
            }
            (re, im)
        });
        Self { bins }
    }

    fn from_i32_lanes(lanes: &[i32]) -> Self {
        let bins = std::array::from_fn(|bin| {
            let frequency = bin + 1;
            let mut re = 0.0;
            let mut im = 0.0;
            for (index, value) in lanes.iter().enumerate() {
                if *value == 0 {
                    continue;
                }
                let theta = TAU * frequency as f32 * index as f32 / SEMANTIC_WAVE_DIM as f32;
                re += *value as f32 * theta.cos();
                im -= *value as f32 * theta.sin();
            }
            (re, im)
        });
        Self { bins }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceWaveCenter {
    lanes: Vec<i32>,
    support: usize,
    fourier: SurfaceFourierSignature,
}

impl SurfaceWaveCenter {
    #[must_use]
    pub fn learn<'a, I>(texts: I) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut lanes = vec![0i32; SEMANTIC_WAVE_DIM];
        let mut support = 0usize;
        for text in texts {
            let wave = SurfaceWave4096::compile(text);
            for (sum, value) in lanes.iter_mut().zip(wave.lanes().iter()) {
                *sum += i32::from(*value);
            }
            support += 1;
        }
        let fourier = SurfaceFourierSignature::from_i32_lanes(&lanes);
        Self {
            lanes,
            support,
            fourier,
        }
    }

    #[must_use]
    pub fn support(&self) -> usize {
        self.support
    }

    #[must_use]
    pub fn surface_similarity(&self, wave: &SurfaceWave4096) -> f32 {
        let mut dot = 0.0;
        let mut left_norm = 0.0;
        let mut right_norm = 0.0;
        for (left, right) in self.lanes.iter().zip(wave.lanes().iter()) {
            dot += *left as f32 * f32::from(*right);
            left_norm += (*left as f32) * (*left as f32);
            right_norm += f32::from(*right) * f32::from(*right);
        }
        if left_norm == 0.0 || right_norm == 0.0 {
            return 0.0;
        }
        dot / (left_norm.sqrt() * right_norm.sqrt())
    }

    #[must_use]
    pub fn fourier_similarity(&self, wave: &SurfaceWave4096) -> f32 {
        let signature = SurfaceFourierSignature::from_surface_wave(wave);
        self.fourier.cosine_similarity(&signature)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WavePatternTemplate {
    pub template_id: String,
    pub schema: SemanticSchemaKey,
    pub unknown_role: String,
    pub object_anchor: String,
    pub center: SurfaceWaveCenter,
    pub min_surface_similarity: f32,
    pub min_fourier_similarity: f32,
}

impl WavePatternTemplate {
    #[must_use]
    pub fn linux_command_provider<'a, I>(commands: I) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        let texts: Vec<_> = commands
            .into_iter()
            .map(|command| format!("which package provides command {command}"))
            .collect();
        Self {
            template_id: "linux-command-provider-query".to_string(),
            schema: SemanticSchemaKey::new(
                "package",
                "provides_command",
                "command",
                "linux.command.provider",
                "positive",
                "package_metadata",
            ),
            unknown_role: "package".to_string(),
            object_anchor: "command".to_string(),
            center: SurfaceWaveCenter::learn(texts.iter().map(String::as_str)),
            min_surface_similarity: 0.58,
            min_fourier_similarity: 0.20,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WavePatternCompiler {
    templates: Vec<WavePatternTemplate>,
}

impl WavePatternCompiler {
    #[must_use]
    pub fn new(templates: Vec<WavePatternTemplate>) -> Self {
        Self { templates }
    }

    #[must_use]
    pub fn compile(&self, text: &str) -> WavePatternCompileReport {
        let text = text.trim();
        if text.is_empty() {
            return WavePatternCompileReport::watch("empty_input");
        }

        let surface_wave = SurfaceWave4096::compile(text);
        let Some((template, selected, runner_up)) = self.select_template(&surface_wave) else {
            return WavePatternCompileReport::watch("no_surface_pattern_template");
        };

        if selected.surface_similarity < template.min_surface_similarity {
            return WavePatternCompileReport::from_selection(
                selected,
                runner_up,
                None,
                None,
                vec!["surface_center_gap_too_weak".to_string()],
            );
        }
        if selected.fourier_similarity < template.min_fourier_similarity {
            return WavePatternCompileReport::from_selection(
                selected,
                runner_up,
                None,
                None,
                vec!["fourier_signature_gap_too_weak".to_string()],
            );
        }

        let Some(object_label) = extract_object_after_anchor(text, &template.object_anchor) else {
            return WavePatternCompileReport::from_selection(
                selected,
                runner_up,
                None,
                None,
                vec!["object_slot_not_found_by_template".to_string()],
            );
        };

        let object_slot = semantic_label_slot(
            &template.schema.route,
            &template.schema.relation,
            &template.schema.object_role,
            &object_label,
        );
        let object = SemanticAtom::new(
            template.schema.object_role.clone(),
            route_family(&template.schema.route),
            object_slot,
            object_label,
        );
        let equation = SemanticEquationForm {
            subject: None,
            schema: template.schema.clone(),
            object: Some(object),
            unknown_role: Some(template.unknown_role.clone()),
        };

        WavePatternCompileReport::from_selection(selected, runner_up, Some(equation), None, vec![])
    }

    #[must_use]
    pub fn compile_and_solve(
        &self,
        text: &str,
        memory: &SemanticWaveMemory,
        candidates: &[SemanticCandidate],
    ) -> WavePatternCompileReport {
        let mut report = self.compile(text);
        let Some(equation) = report.equation.as_ref() else {
            return report;
        };
        let Some(prediction) = memory.solve_equation(equation, candidates) else {
            report.status = SemanticExtractionStatus::Watch;
            report
                .reason_codes
                .push("semantic_operator_no_solution".to_string());
            return report;
        };
        if prediction.margin <= 0 {
            report.status = SemanticExtractionStatus::Watch;
            report
                .reason_codes
                .push("semantic_center_gap_too_weak".to_string());
            report.prediction = Some(prediction);
            return report;
        }
        report.status = SemanticExtractionStatus::Accepted;
        report.semantic_operator_has_authority = true;
        report.prediction = Some(prediction);
        report
    }

    fn select_template(
        &self,
        wave: &SurfaceWave4096,
    ) -> Option<(
        &WavePatternTemplate,
        WavePatternSelection,
        WavePatternSelection,
    )> {
        let mut scored = self
            .templates
            .iter()
            .map(|template| WavePatternSelection {
                template_id: template.template_id.clone(),
                surface_similarity: template.center.surface_similarity(wave),
                fourier_similarity: template.center.fourier_similarity(wave),
            })
            .enumerate()
            .collect::<Vec<_>>();
        scored.sort_by(|(_, left), (_, right)| {
            right
                .combined_score()
                .total_cmp(&left.combined_score())
                .then_with(|| left.template_id.cmp(&right.template_id))
        });

        let (best_index, best) = scored.first()?.clone();
        let runner_up = scored
            .get(1)
            .map(|(_, selection)| selection.clone())
            .unwrap_or_default();
        Some((&self.templates[best_index], best, runner_up))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WavePatternSelection {
    pub template_id: String,
    pub surface_similarity: f32,
    pub fourier_similarity: f32,
}

impl WavePatternSelection {
    fn combined_score(&self) -> f32 {
        self.surface_similarity + self.fourier_similarity
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WavePatternCompileReport {
    pub status: SemanticExtractionStatus,
    pub selected_template_id: Option<String>,
    pub surface_similarity: f32,
    pub surface_runner_up_similarity: f32,
    pub surface_gap: f32,
    pub fourier_similarity: f32,
    pub fourier_runner_up_similarity: f32,
    pub fourier_gap: f32,
    pub equation: Option<SemanticEquationForm>,
    pub prediction: Option<SemanticEquationPrediction>,
    pub reason_codes: Vec<String>,
    pub raw_text_has_authority: bool,
    pub surface_pattern_has_authority: bool,
    pub semantic_operator_has_authority: bool,
}

impl WavePatternCompileReport {
    fn watch(reason: &str) -> Self {
        Self {
            status: SemanticExtractionStatus::Watch,
            selected_template_id: None,
            surface_similarity: 0.0,
            surface_runner_up_similarity: 0.0,
            surface_gap: 0.0,
            fourier_similarity: 0.0,
            fourier_runner_up_similarity: 0.0,
            fourier_gap: 0.0,
            equation: None,
            prediction: None,
            reason_codes: vec![reason.to_string()],
            raw_text_has_authority: false,
            surface_pattern_has_authority: false,
            semantic_operator_has_authority: false,
        }
    }

    fn from_selection(
        selected: WavePatternSelection,
        runner_up: WavePatternSelection,
        equation: Option<SemanticEquationForm>,
        prediction: Option<SemanticEquationPrediction>,
        reason_codes: Vec<String>,
    ) -> Self {
        let surface_pattern_has_authority = equation.is_some() && reason_codes.is_empty();
        let semantic_operator_has_authority = prediction.as_ref().is_some_and(|p| p.margin > 0);
        Self {
            status: if surface_pattern_has_authority {
                SemanticExtractionStatus::Accepted
            } else {
                SemanticExtractionStatus::Watch
            },
            selected_template_id: Some(selected.template_id),
            surface_similarity: selected.surface_similarity,
            surface_runner_up_similarity: runner_up.surface_similarity,
            surface_gap: selected.surface_similarity - runner_up.surface_similarity,
            fourier_similarity: selected.fourier_similarity,
            fourier_runner_up_similarity: runner_up.fourier_similarity,
            fourier_gap: selected.fourier_similarity - runner_up.fourier_similarity,
            equation,
            prediction,
            reason_codes,
            raw_text_has_authority: false,
            surface_pattern_has_authority,
            semantic_operator_has_authority,
        }
    }
}

fn extract_object_after_anchor(text: &str, anchor: &str) -> Option<String> {
    let tokens = normalize_tokens(text);
    tokens
        .windows(2)
        .find(|pair| pair[0] == anchor)
        .map(|pair| pair[1].clone())
}

fn normalize_tokens(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|token| token.trim_matches(|ch: char| matches!(ch, '?' | '.' | ',' | ';' | ':')))
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn route_family(route: &str) -> String {
    route.replace('.', "-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wave::{SemanticAtomExtractor, SemanticFact};

    fn compiler() -> WavePatternCompiler {
        let commands = (0..800)
            .map(|index| format!("cmd{index:04}"))
            .collect::<Vec<_>>();
        WavePatternCompiler::new(vec![WavePatternTemplate::linux_command_provider(
            commands.iter().map(String::as_str),
        )])
    }

    fn facts() -> Vec<SemanticFact> {
        (0..800)
            .map(|index| {
                SemanticAtomExtractor::extract(&format!(
                    "package pkg{index:04} provides command cmd{index:04}"
                ))
                .fact()
                .expect("fact")
                .clone()
            })
            .collect()
    }

    #[test]
    fn wave_pattern_compiler_uses_surface_fourier_then_semantic_operator() {
        let compiler = compiler();
        let mut memory = SemanticWaveMemory::new();
        memory.train(facts().iter());
        let text = "which package provides command cmd0800?";
        let slot = semantic_label_slot(
            "linux.command.provider",
            "provides_command",
            "command",
            "cmd0800",
        );
        let candidates = [
            SemanticCandidate::new(SemanticAtom::new(
                "package",
                "linux-command-provider",
                slot,
                "pkg0800",
            )),
            SemanticCandidate::new(SemanticAtom::new(
                "package",
                "linux-command-provider",
                semantic_label_slot(
                    "linux.command.provider",
                    "provides_command",
                    "command",
                    "cmd0801",
                ),
                "pkg0801",
            )),
            SemanticCandidate::new(SemanticAtom::new(
                "command",
                "linux-command-provider",
                slot,
                "cmd0800",
            )),
        ];

        let report = compiler.compile_and_solve(text, &memory, &candidates);

        assert_eq!(report.status, SemanticExtractionStatus::Accepted);
        assert!(!report.raw_text_has_authority);
        assert!(report.surface_pattern_has_authority);
        assert!(report.semantic_operator_has_authority, "report={report:#?}");
        assert!(report.surface_similarity > 0.58, "report={report:#?}");
        assert!(report.fourier_similarity > 0.20, "report={report:#?}");
        let prediction = report.prediction.expect("prediction");
        assert_eq!(prediction.resolved_role, "package");
        assert_eq!(prediction.resolved_label, "pkg0800");
    }

    #[test]
    fn arbitrary_text_does_not_compile_to_semantic_wave_pattern() {
        let report = compiler().compile("bash provides bash");

        assert_eq!(report.status, SemanticExtractionStatus::Watch);
        assert!(report.equation.is_none());
        assert!(!report.raw_text_has_authority);
        assert!(!report.semantic_operator_has_authority);
    }

    #[test]
    fn surface_match_without_semantic_operator_still_has_no_answer_authority() {
        let compiler = compiler();
        let memory = SemanticWaveMemory::new();
        let report =
            compiler.compile_and_solve("which package provides command cmd0800?", &memory, &[]);

        assert_eq!(report.status, SemanticExtractionStatus::Watch);
        assert!(report.equation.is_some());
        assert!(!report.raw_text_has_authority);
        assert!(report.surface_pattern_has_authority);
        assert!(!report.semantic_operator_has_authority);
        assert!(
            report
                .reason_codes
                .contains(&"semantic_operator_no_solution".to_string())
        );
    }
}
