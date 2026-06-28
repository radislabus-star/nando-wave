//! Verified atom extraction into semantic EquationForm.
//!
//! This is deliberately narrow. It is not a general parser and not an LLM. It
//! accepts only role-complete forms or returns WATCH, because a bad extractor
//! would make the semantic-wave operator learn garbage.

use super::{SemanticAtom, SemanticFact, SemanticSchemaKey};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticExtractionStatus {
    Accepted,
    Watch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SemanticExtractedForm {
    Fact(SemanticFact),
    Equation(SemanticEquationForm),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticEquationForm {
    pub subject: Option<SemanticAtom>,
    pub schema: SemanticSchemaKey,
    pub object: Option<SemanticAtom>,
    pub unknown_role: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticExtraction {
    pub status: SemanticExtractionStatus,
    pub form: Option<SemanticExtractedForm>,
    pub extractor_profile: &'static str,
    pub reason_codes: Vec<String>,
    pub missing_fields: Vec<String>,
}

impl SemanticExtraction {
    #[must_use]
    pub fn accepted(form: SemanticExtractedForm, extractor_profile: &'static str) -> Self {
        Self {
            status: SemanticExtractionStatus::Accepted,
            form: Some(form),
            extractor_profile,
            reason_codes: Vec::new(),
            missing_fields: Vec::new(),
        }
    }

    #[must_use]
    pub fn watch(
        extractor_profile: &'static str,
        reason_codes: Vec<String>,
        missing_fields: Vec<String>,
    ) -> Self {
        Self {
            status: SemanticExtractionStatus::Watch,
            form: None,
            extractor_profile,
            reason_codes,
            missing_fields,
        }
    }

    #[must_use]
    pub fn fact(&self) -> Option<&SemanticFact> {
        match self.form.as_ref()? {
            SemanticExtractedForm::Fact(fact) => Some(fact),
            SemanticExtractedForm::Equation(_) => None,
        }
    }

    #[must_use]
    pub fn equation(&self) -> Option<&SemanticEquationForm> {
        match self.form.as_ref()? {
            SemanticExtractedForm::Fact(_) => None,
            SemanticExtractedForm::Equation(equation) => Some(equation),
        }
    }
}

pub struct SemanticAtomExtractor;

impl SemanticAtomExtractor {
    #[must_use]
    pub fn extract(text: &str) -> SemanticExtraction {
        let text = text.trim();
        if text.is_empty() {
            return SemanticExtraction::watch(
                "semantic-atom-extractor-v0",
                vec!["empty_input".to_string()],
                required_fact_fields(),
            );
        }

        if text.contains('|') {
            return extract_pipe_form(text);
        }

        extract_linux_command_provider(text).unwrap_or_else(|| {
            SemanticExtraction::watch(
                "semantic-atom-extractor-v0",
                vec!["no_role_complete_extraction".to_string()],
                required_fact_fields(),
            )
        })
    }
}

#[must_use]
pub fn semantic_label_slot(
    route: &str,
    relation: &str,
    object_role: &str,
    object_label: &str,
) -> u32 {
    let mut state = 0x4154_4F4D_4558_5630u64;
    for part in [route, relation, object_role, object_label] {
        for byte in part.as_bytes() {
            state ^= u64::from(*byte).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            state = splitmix64(state);
        }
    }
    (splitmix64(state) & 0xFFFF_FFFF) as u32
}

fn extract_pipe_form(text: &str) -> SemanticExtraction {
    let mut subject_role = None;
    let mut subject_label = None;
    let mut relation = None;
    let mut object_role = None;
    let mut object_label = None;
    let mut route = None;
    let mut polarity = Some("positive".to_string());
    let mut evidence_kind = None;
    let mut slot = None;

    let parts: Vec<_> = text
        .split('|')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();

    for (index, part) in parts.iter().enumerate() {
        if let Some((key, value)) = part.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "subject" => {
                    let Some((role, label)) = split_role_label(value) else {
                        return SemanticExtraction::watch(
                            "semantic-atom-extractor-v0",
                            vec!["bad_subject_atom".to_string()],
                            vec!["subject_role".to_string(), "subject_label".to_string()],
                        );
                    };
                    subject_role = Some(role);
                    subject_label = Some(label);
                }
                "object" => {
                    let Some((role, label)) = split_role_label(value) else {
                        return SemanticExtraction::watch(
                            "semantic-atom-extractor-v0",
                            vec!["bad_object_atom".to_string()],
                            vec!["object_role".to_string(), "object_label".to_string()],
                        );
                    };
                    object_role = Some(role);
                    object_label = Some(label);
                }
                "subject_role" => subject_role = Some(value.to_string()),
                "subject_label" => subject_label = Some(value.to_string()),
                "relation" => relation = Some(value.to_string()),
                "object_role" => object_role = Some(value.to_string()),
                "object_label" => object_label = Some(value.to_string()),
                "route" => route = Some(value.to_string()),
                "polarity" => polarity = Some(value.to_string()),
                "evidence" | "evidence_kind" => evidence_kind = Some(value.to_string()),
                "slot" => match value.parse::<u32>() {
                    Ok(value) => slot = Some(value),
                    Err(_) => {
                        return SemanticExtraction::watch(
                            "semantic-atom-extractor-v0",
                            vec!["bad_slot".to_string()],
                            vec!["slot_u32".to_string()],
                        );
                    }
                },
                _ => {}
            }
            continue;
        }

        match index {
            0 => {
                if let Some((role, label)) = split_role_label(part) {
                    subject_role = Some(role);
                    subject_label = Some(label);
                }
            }
            1 => relation = Some((*part).to_string()),
            2 => {
                if let Some((role, label)) = split_role_label(part) {
                    object_role = Some(role);
                    object_label = Some(label);
                }
            }
            _ => {}
        }
    }

    build_fact(
        subject_role,
        subject_label,
        relation,
        object_role,
        object_label,
        route,
        polarity,
        evidence_kind,
        slot,
    )
}

fn extract_linux_command_provider(text: &str) -> Option<SemanticExtraction> {
    let tokens = tokenize(text);
    if tokens.is_empty() {
        return None;
    }

    let schema = linux_command_provider_schema();

    if tokens.len() == 5
        && tokens[0] == "which"
        && tokens[1] == "package"
        && tokens[2] == "provides"
        && tokens[3] == "command"
    {
        let command = tokens[4].clone();
        let slot = semantic_label_slot(
            &schema.route,
            &schema.relation,
            &schema.object_role,
            &command,
        );
        let object = SemanticAtom::new("command", route_family(&schema.route), slot, command);
        return Some(SemanticExtraction::accepted(
            SemanticExtractedForm::Equation(SemanticEquationForm {
                subject: None,
                schema,
                object: Some(object),
                unknown_role: Some("package".to_string()),
            }),
            "linux-command-provider-v0",
        ));
    }

    if tokens.len() == 5
        && tokens[0] == "package"
        && tokens[2] == "provides"
        && tokens[3] == "command"
    {
        return Some(linux_provider_fact(&tokens[1], &tokens[4], schema));
    }

    if tokens.len() == 5
        && tokens[1] == "package"
        && tokens[2] == "provides"
        && tokens[3] == "command"
    {
        return Some(linux_provider_fact(&tokens[0], &tokens[4], schema));
    }

    None
}

fn linux_provider_fact(
    package_label: &str,
    command_label: &str,
    schema: SemanticSchemaKey,
) -> SemanticExtraction {
    let slot = semantic_label_slot(
        &schema.route,
        &schema.relation,
        &schema.object_role,
        command_label,
    );
    let family = route_family(&schema.route);
    let subject = SemanticAtom::new("package", family.clone(), slot, package_label);
    let object = SemanticAtom::new("command", family, slot, command_label);
    SemanticExtraction::accepted(
        SemanticExtractedForm::Fact(SemanticFact::new(subject, schema, object)),
        "linux-command-provider-v0",
    )
}

#[allow(clippy::too_many_arguments)]
fn build_fact(
    subject_role: Option<String>,
    subject_label: Option<String>,
    relation: Option<String>,
    object_role: Option<String>,
    object_label: Option<String>,
    route: Option<String>,
    polarity: Option<String>,
    evidence_kind: Option<String>,
    slot: Option<u32>,
) -> SemanticExtraction {
    let missing = missing_fields(
        subject_role.as_deref(),
        subject_label.as_deref(),
        relation.as_deref(),
        object_role.as_deref(),
        object_label.as_deref(),
        route.as_deref(),
        polarity.as_deref(),
        evidence_kind.as_deref(),
    );
    if !missing.is_empty() {
        return SemanticExtraction::watch(
            "semantic-atom-extractor-v0",
            vec!["missing_role_complete_channels".to_string()],
            missing,
        );
    }

    let subject_role = subject_role.expect("checked");
    let subject_label = subject_label.expect("checked");
    let relation = relation.expect("checked");
    let object_role = object_role.expect("checked");
    let object_label = object_label.expect("checked");
    let route = route.expect("checked");
    let polarity = polarity.expect("checked");
    let evidence_kind = evidence_kind.expect("checked");

    if is_generic_relation(&relation) {
        return SemanticExtraction::watch(
            "semantic-atom-extractor-v0",
            vec!["relation_too_generic".to_string()],
            vec!["role_typed_relation".to_string()],
        );
    }

    let slot =
        slot.unwrap_or_else(|| semantic_label_slot(&route, &relation, &object_role, &object_label));
    let family = route_family(&route);
    let subject = SemanticAtom::new(subject_role.clone(), family.clone(), slot, subject_label);
    let object = SemanticAtom::new(object_role.clone(), family, slot, object_label);
    let schema = SemanticSchemaKey::new(
        subject_role,
        relation,
        object_role,
        route,
        polarity,
        evidence_kind,
    );

    SemanticExtraction::accepted(
        SemanticExtractedForm::Fact(SemanticFact::new(subject, schema, object)),
        "semantic-atom-extractor-v0",
    )
}

fn linux_command_provider_schema() -> SemanticSchemaKey {
    SemanticSchemaKey::new(
        "package",
        "provides_command",
        "command",
        "linux.command.provider",
        "positive",
        "package_metadata",
    )
}

fn split_role_label(value: &str) -> Option<(String, String)> {
    let (role, label) = value.split_once(':')?;
    let role = role.trim();
    let label = label.trim();
    if role.is_empty() || label.is_empty() {
        return None;
    }
    Some((role.to_string(), label.to_string()))
}

fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|token| token.trim_matches(|ch: char| matches!(ch, '?' | '.' | ',' | ';' | ':')))
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn missing_fields(
    subject_role: Option<&str>,
    subject_label: Option<&str>,
    relation: Option<&str>,
    object_role: Option<&str>,
    object_label: Option<&str>,
    route: Option<&str>,
    polarity: Option<&str>,
    evidence_kind: Option<&str>,
) -> Vec<String> {
    [
        ("subject_role", subject_role),
        ("subject_label", subject_label),
        ("relation", relation),
        ("object_role", object_role),
        ("object_label", object_label),
        ("route", route),
        ("polarity", polarity),
        ("evidence_kind", evidence_kind),
    ]
    .into_iter()
    .filter_map(|(name, value)| {
        value
            .filter(|value| !value.is_empty())
            .map(|_| ())
            .is_none()
            .then_some(name.to_string())
    })
    .collect()
}

fn required_fact_fields() -> Vec<String> {
    vec![
        "subject_role".to_string(),
        "subject_label".to_string(),
        "relation".to_string(),
        "object_role".to_string(),
        "object_label".to_string(),
        "route".to_string(),
        "polarity".to_string(),
        "evidence_kind".to_string(),
    ]
}

fn route_family(route: &str) -> String {
    route.replace('.', "-")
}

fn is_generic_relation(relation: &str) -> bool {
    matches!(relation, "is" | "has" | "status" | "active" | "related_to")
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
    use crate::wave::{SemanticCandidate, SemanticWaveMemory};

    #[test]
    fn extracts_role_complete_pipe_fact() {
        let extraction = SemanticAtomExtractor::extract(
            "subject=package:bash | relation=provides_command | object=command:bash | route=linux.command.provider | polarity=positive | evidence=package_metadata",
        );

        assert_eq!(extraction.status, SemanticExtractionStatus::Accepted);
        let fact = extraction.fact().expect("fact");
        assert_eq!(fact.subject.role, "package");
        assert_eq!(fact.subject.label, "bash");
        assert_eq!(fact.object.role, "command");
        assert_eq!(fact.object.label, "bash");
        assert_eq!(fact.schema.route, "linux.command.provider");
    }

    #[test]
    fn extracts_linux_command_provider_question_as_equation_with_unknown_subject() {
        let extraction = SemanticAtomExtractor::extract("which package provides command bash?");

        assert_eq!(extraction.status, SemanticExtractionStatus::Accepted);
        let equation = extraction.equation().expect("equation");
        assert!(equation.subject.is_none());
        assert_eq!(equation.unknown_role.as_deref(), Some("package"));
        assert_eq!(equation.object.as_ref().expect("object").role, "command");
        assert_eq!(equation.schema.relation, "provides_command");
    }

    #[test]
    fn refuses_ambiguous_text_without_role_complete_channels() {
        let extraction = SemanticAtomExtractor::extract("bash provides bash");

        assert_eq!(extraction.status, SemanticExtractionStatus::Watch);
        assert!(extraction.form.is_none());
        assert!(
            extraction
                .reason_codes
                .contains(&"no_role_complete_extraction".to_string())
        );
    }

    #[test]
    fn refuses_generic_relation_even_in_pipe_form() {
        let extraction = SemanticAtomExtractor::extract(
            "subject=layer:water | relation=status | object=state:active | route=physics.material_layer.status | polarity=positive | evidence=local_artifact",
        );

        assert_eq!(extraction.status, SemanticExtractionStatus::Watch);
        assert!(
            extraction
                .reason_codes
                .contains(&"relation_too_generic".to_string())
        );
    }

    #[test]
    fn extracted_facts_feed_semantic_operator_on_heldout_slots() {
        let train: Vec<_> = (0..800)
            .map(|index| {
                SemanticAtomExtractor::extract(&format!(
                    "package pkg{index:04} provides command cmd{index:04}"
                ))
                .fact()
                .expect("fact")
                .clone()
            })
            .collect();
        let heldout = SemanticAtomExtractor::extract("package pkg0800 provides command cmd0800")
            .fact()
            .expect("heldout fact")
            .clone();

        let mut memory = SemanticWaveMemory::new();
        memory.train(train.iter());
        let prediction = memory
            .predict(
                &heldout.query(),
                &[
                    SemanticCandidate::new(heldout.object.clone()),
                    SemanticCandidate::new(SemanticAtom::new(
                        "package",
                        heldout.subject.family.clone(),
                        heldout.subject.slot,
                        "pkg0800",
                    )),
                    SemanticCandidate::new(SemanticAtom::new(
                        "command",
                        heldout.object.family.clone(),
                        semantic_label_slot(
                            &heldout.schema.route,
                            &heldout.schema.relation,
                            &heldout.schema.object_role,
                            "cmd0801",
                        ),
                        "cmd0801",
                    )),
                ],
            )
            .expect("operator");

        assert_eq!(prediction.object_label, "cmd0800");
        assert!(prediction.margin > 0, "prediction={prediction:?}");
    }

    #[test]
    fn question_equation_resolves_unknown_subject_through_trained_wave_operator() {
        let train: Vec<_> = (0..800)
            .map(|index| {
                SemanticAtomExtractor::extract(&format!(
                    "package pkg{index:04} provides command cmd{index:04}"
                ))
                .fact()
                .expect("fact")
                .clone()
            })
            .collect();
        let equation = SemanticAtomExtractor::extract("which package provides command cmd0800?")
            .equation()
            .expect("equation")
            .clone();

        let mut memory = SemanticWaveMemory::new();
        memory.train(train.iter());

        let family = equation.object.as_ref().expect("object").family.clone();
        let candidates = [
            SemanticCandidate::new(SemanticAtom::new(
                "package",
                family.clone(),
                semantic_label_slot(
                    &equation.schema.route,
                    &equation.schema.relation,
                    &equation.schema.object_role,
                    "cmd0800",
                ),
                "pkg0800",
            )),
            SemanticCandidate::new(SemanticAtom::new(
                "package",
                family.clone(),
                semantic_label_slot(
                    &equation.schema.route,
                    &equation.schema.relation,
                    &equation.schema.object_role,
                    "cmd0801",
                ),
                "pkg0801",
            )),
            SemanticCandidate::new(SemanticAtom::new(
                "command",
                family,
                semantic_label_slot(
                    &equation.schema.route,
                    &equation.schema.relation,
                    &equation.schema.object_role,
                    "cmd0800",
                ),
                "cmd0800",
            )),
        ];

        let prediction = memory
            .solve_equation(&equation, &candidates)
            .expect("equation should be solvable by trained operator");

        assert_eq!(prediction.resolved_role, "package");
        assert_eq!(prediction.resolved_label, "pkg0800");
        assert!(prediction.margin > 0, "prediction={prediction:?}");
    }
}
