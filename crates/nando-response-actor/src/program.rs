use serde::{Deserialize, Serialize};

use crate::{ResponseValueSelector, SemanticRole};

pub const MAX_PROJECT_STATUS_CODE: u64 = 1_000_000;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum ResponseArgument {
    Role {
        name: String,
        role: SemanticRole,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value_type: Option<crate::AtomValueType>,
    },
    Integer {
        name: String,
        value: u64,
    },
    String {
        name: String,
        value: String,
    },
    Boolean {
        name: String,
        value: bool,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CustomToolResultProjection {
    OutputField {
        output_field: String,
    },
    OutputAndContinuation {
        output_field: String,
        continuation_field: String,
        continuation_prefix: String,
    },
    JsonStringifyResult,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueProjectionFormat {
    PlainText,
    CanonicalJson,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CollectionOutputRenderer {
    #[default]
    Direct,
    RenderTemplate {
        prefix: String,
        suffix: String,
    },
    RenderSequence {
        segments: Vec<ResponseRenderSegment>,
    },
}

impl CollectionOutputRenderer {
    #[must_use]
    pub const fn is_direct(&self) -> bool {
        matches!(self, Self::Direct)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResponseRenderSegment {
    Static {
        text: String,
    },
    Primary,
    Selected {
        selector: ResponseValueSelector,
        format: ValueProjectionFormat,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ResponseScalarLiteral {
    String(String),
    Integer(i64),
    Boolean(bool),
    Null,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionScalarType {
    String,
    Integer,
    Boolean,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionAggregateOperation {
    Sum,
    Min,
    Max,
}

impl ResponseScalarLiteral {
    #[must_use]
    pub fn as_json(&self) -> serde_json::Value {
        match self {
            Self::String(value) => serde_json::Value::String(value.clone()),
            Self::Integer(value) => serde_json::Value::from(*value),
            Self::Boolean(value) => serde_json::Value::Bool(*value),
            Self::Null => serde_json::Value::Null,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum CollectionProgramStep {
    SelectOnlyArrayField,
    SelectField {
        field: String,
    },
    FilterUniqueFieldEquals {
        value: ResponseScalarLiteral,
    },
    FilterUniqueFieldEqualsRequestValue {
        value_type: CollectionScalarType,
    },
    FilterFieldEquals {
        field: String,
        value: ResponseScalarLiteral,
    },
    ProjectField {
        field: String,
    },
    ProjectUniqueFieldByType {
        value_type: CollectionScalarType,
    },
    ProjectOnlyNonFilterField,
    AggregateUniqueIntegerField {
        operation: CollectionAggregateOperation,
    },
    Count,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatusMapping {
    ZeroIsSuccess,
    ZeroIsPass,
    ZeroIsOk,
    ZeroIsTrue,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatusValue {
    Success,
    Failure,
}

impl ProjectStatusValue {
    #[must_use]
    pub const fn canonical_text(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ResponseOperation {
    FunctionCallFromRoles {
        function_name: String,
        selector: ResponseValueSelector,
        arguments: Vec<ResponseArgument>,
    },
    CustomToolCallFromRoles {
        custom_tool_name: String,
        inner_tool_name: String,
        selector: ResponseValueSelector,
        arguments: Vec<ResponseArgument>,
        projection: CustomToolResultProjection,
    },
    ProjectSelectedValue {
        selector: ResponseValueSelector,
        format: ValueProjectionFormat,
        #[serde(default, skip_serializing_if = "CollectionOutputRenderer::is_direct")]
        renderer: CollectionOutputRenderer,
        completion_state: String,
    },
    ProjectStatus {
        selector: ResponseValueSelector,
        mapping: ProjectStatusMapping,
        completion_state: String,
    },
    ComposeCollection {
        steps: Vec<CollectionProgramStep>,
        format: ValueProjectionFormat,
        #[serde(default, skip_serializing_if = "CollectionOutputRenderer::is_direct")]
        renderer: CollectionOutputRenderer,
        completion_state: String,
        max_items: usize,
    },
    CopyAfterPrefix {
        prefixes: Vec<String>,
        trim: bool,
        allow_multiline: bool,
    },
    TestResultSummary {
        required_intent_phrases: Vec<String>,
        forbidden_intent_terms: Vec<String>,
    },
    WaitOnYieldedCell {
        function_name: String,
        yield_time_ms: u64,
        max_tokens: u64,
    },
    WaitOnAnyYieldedCell {
        function_name: String,
        yield_time_ms: u64,
        max_tokens: u64,
    },
    WaitOnYieldedSurfaces {
        surfaces: Vec<String>,
        function_name: String,
        yield_time_ms: u64,
        max_tokens: u64,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResponseProgram {
    pub schema: String,
    pub operation: ResponseOperation,
    pub max_output_bytes: usize,
}

impl ResponseProgram {
    #[must_use]
    pub fn function_call_from_roles(
        function_name: impl Into<String>,
        selector: ResponseValueSelector,
        arguments: Vec<ResponseArgument>,
    ) -> Self {
        Self {
            schema: "nando.response-program.v1".to_owned(),
            operation: ResponseOperation::FunctionCallFromRoles {
                function_name: function_name.into(),
                selector,
                arguments,
            },
            max_output_bytes: 256,
        }
    }

    #[must_use]
    pub fn custom_tool_call_from_roles(
        custom_tool_name: impl Into<String>,
        inner_tool_name: impl Into<String>,
        selector: ResponseValueSelector,
        arguments: Vec<ResponseArgument>,
        projection: CustomToolResultProjection,
    ) -> Self {
        Self {
            schema: "nando.response-program.v1".to_owned(),
            operation: ResponseOperation::CustomToolCallFromRoles {
                custom_tool_name: custom_tool_name.into(),
                inner_tool_name: inner_tool_name.into(),
                selector,
                arguments,
                projection,
            },
            max_output_bytes: 4_096,
        }
    }

    #[must_use]
    pub fn project_selected_value(
        selector: ResponseValueSelector,
        format: ValueProjectionFormat,
        completion_state: impl Into<String>,
    ) -> Self {
        Self {
            schema: "nando.response-program.v1".to_owned(),
            operation: ResponseOperation::ProjectSelectedValue {
                selector,
                format,
                renderer: CollectionOutputRenderer::Direct,
                completion_state: completion_state.into(),
            },
            max_output_bytes: 4_096,
        }
    }

    #[must_use]
    pub fn with_value_renderer(mut self, renderer: CollectionOutputRenderer) -> Self {
        if let ResponseOperation::ProjectSelectedValue {
            renderer: current, ..
        } = &mut self.operation
        {
            *current = renderer;
        }
        self
    }

    #[must_use]
    pub fn project_status(
        selector: ResponseValueSelector,
        mapping: ProjectStatusMapping,
        completion_state: impl Into<String>,
    ) -> Self {
        Self {
            schema: "nando.response-program.v1".to_owned(),
            operation: ResponseOperation::ProjectStatus {
                selector,
                mapping,
                completion_state: completion_state.into(),
            },
            max_output_bytes: ProjectStatusValue::Failure.canonical_text().len(),
        }
    }

    #[must_use]
    pub fn compose_collection(
        steps: Vec<CollectionProgramStep>,
        format: ValueProjectionFormat,
        completion_state: impl Into<String>,
    ) -> Self {
        Self {
            schema: "nando.response-program.v1".to_owned(),
            operation: ResponseOperation::ComposeCollection {
                steps,
                format,
                renderer: CollectionOutputRenderer::Direct,
                completion_state: completion_state.into(),
                max_items: 1_024,
            },
            max_output_bytes: 16_384,
        }
    }

    #[must_use]
    pub fn with_collection_renderer(mut self, renderer: CollectionOutputRenderer) -> Self {
        if let ResponseOperation::ComposeCollection {
            renderer: current, ..
        } = &mut self.operation
        {
            *current = renderer;
        }
        self
    }

    #[must_use]
    pub fn copy_after_prefix<I, S>(prefixes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            schema: "nando.response-program.v1".to_owned(),
            operation: ResponseOperation::CopyAfterPrefix {
                prefixes: prefixes.into_iter().map(Into::into).collect(),
                trim: true,
                allow_multiline: false,
            },
            max_output_bytes: 4096,
        }
    }

    #[must_use]
    pub fn test_result_summary<I, S, J, T>(required: I, forbidden: J) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
        J: IntoIterator<Item = T>,
        T: Into<String>,
    {
        Self {
            schema: "nando.response-program.v1".to_owned(),
            operation: ResponseOperation::TestResultSummary {
                required_intent_phrases: required.into_iter().map(Into::into).collect(),
                forbidden_intent_terms: forbidden.into_iter().map(Into::into).collect(),
            },
            max_output_bytes: 256,
        }
    }

    #[must_use]
    pub fn wait_on_yielded_cell() -> Self {
        Self {
            schema: "nando.response-program.v1".to_owned(),
            operation: ResponseOperation::WaitOnYieldedCell {
                function_name: "wait".to_owned(),
                yield_time_ms: 1_000,
                max_tokens: 5_000,
            },
            max_output_bytes: 256,
        }
    }

    #[must_use]
    pub fn wait_on_any_yielded_cell() -> Self {
        Self {
            schema: "nando.response-program.v1".to_owned(),
            operation: ResponseOperation::WaitOnAnyYieldedCell {
                function_name: "wait".to_owned(),
                yield_time_ms: 1_000,
                max_tokens: 5_000,
            },
            max_output_bytes: 256,
        }
    }

    #[must_use]
    pub fn wait_on_yielded_surfaces<I, S>(surfaces: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            schema: "nando.response-program.v1".to_owned(),
            operation: ResponseOperation::WaitOnYieldedSurfaces {
                surfaces: surfaces.into_iter().map(Into::into).collect(),
                function_name: "wait".to_owned(),
                yield_time_ms: 1_000,
                max_tokens: 5_000,
            },
            max_output_bytes: 256,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != "nando.response-program.v1" {
            return Err("unsupported_program_schema");
        }
        if self.max_output_bytes == 0 || self.max_output_bytes > 65_536 {
            return Err("invalid_output_budget");
        }
        match &self.operation {
            ResponseOperation::FunctionCallFromRoles {
                function_name,
                selector,
                arguments,
            } => {
                validate_identifier(function_name, "invalid_function_name")?;
                validate_selector(selector)?;
                validate_arguments(arguments)
            }
            ResponseOperation::CustomToolCallFromRoles {
                custom_tool_name,
                inner_tool_name,
                selector,
                arguments,
                projection,
            } => {
                validate_identifier(custom_tool_name, "invalid_custom_tool_name")?;
                validate_identifier(inner_tool_name, "invalid_inner_tool_name")?;
                validate_selector(selector)?;
                validate_arguments(arguments)?;
                if let CustomToolResultProjection::OutputAndContinuation {
                    output_field,
                    continuation_field,
                    continuation_prefix,
                } = projection
                {
                    validate_identifier(output_field, "invalid_projection_output")?;
                    validate_identifier(continuation_field, "invalid_projection_continuation")?;
                    if continuation_prefix.is_empty()
                        || continuation_prefix.len() > 128
                        || continuation_prefix
                            .bytes()
                            .any(|byte| byte.is_ascii_control())
                    {
                        return Err("invalid_projection_prefix");
                    }
                }
                Ok(())
            }
            ResponseOperation::ProjectSelectedValue {
                selector,
                renderer,
                completion_state,
                ..
            } => {
                validate_selector(selector)?;
                if !matches!(completion_state.as_str(), "pending" | "completed") {
                    return Err("invalid_projection_completion_state");
                }
                match renderer {
                    CollectionOutputRenderer::Direct => {}
                    CollectionOutputRenderer::RenderTemplate { prefix, suffix } => {
                        if !safe_collection_renderer(prefix, suffix) {
                            return Err("invalid_value_renderer");
                        }
                    }
                    CollectionOutputRenderer::RenderSequence { segments } => {
                        validate_render_sequence(segments)?;
                    }
                }
                Ok(())
            }
            ResponseOperation::ProjectStatus {
                selector,
                completion_state,
                ..
            } => {
                validate_selector(selector)?;
                if selector_value_type(selector) != crate::AtomValueType::Integer {
                    return Err("status_selector_must_be_integer");
                }
                if !matches!(completion_state.as_str(), "pending" | "completed") {
                    return Err("invalid_status_completion_state");
                }
                Ok(())
            }
            ResponseOperation::ComposeCollection {
                steps,
                renderer,
                completion_state,
                max_items,
                ..
            } => {
                if steps.is_empty() || steps.len() > 8 {
                    return Err("invalid_collection_program_length");
                }
                if *max_items == 0 || *max_items > 4_096 {
                    return Err("invalid_collection_item_budget");
                }
                if !matches!(completion_state.as_str(), "pending" | "completed") {
                    return Err("invalid_collection_completion_state");
                }
                match renderer {
                    CollectionOutputRenderer::Direct => {}
                    CollectionOutputRenderer::RenderTemplate { prefix, suffix } => {
                        if !safe_collection_renderer(prefix, suffix) {
                            return Err("invalid_collection_renderer");
                        }
                    }
                    CollectionOutputRenderer::RenderSequence { segments } => {
                        validate_render_sequence(segments)?;
                    }
                }
                for step in steps {
                    match step {
                        CollectionProgramStep::SelectField { field }
                        | CollectionProgramStep::ProjectField { field }
                        | CollectionProgramStep::FilterFieldEquals { field, .. } => {
                            validate_identifier(field, "invalid_collection_field")?;
                        }
                        CollectionProgramStep::SelectOnlyArrayField
                        | CollectionProgramStep::FilterUniqueFieldEquals { .. }
                        | CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { .. }
                        | CollectionProgramStep::ProjectUniqueFieldByType { .. }
                        | CollectionProgramStep::ProjectOnlyNonFilterField
                        | CollectionProgramStep::AggregateUniqueIntegerField { .. }
                        | CollectionProgramStep::Count => {}
                    }
                }
                Ok(())
            }
            ResponseOperation::CopyAfterPrefix { prefixes, .. } => {
                validate_phrases(prefixes, "empty_prefixes")
            }
            ResponseOperation::TestResultSummary {
                required_intent_phrases,
                forbidden_intent_terms,
            } => {
                validate_phrases(required_intent_phrases, "empty_required_intents")?;
                if forbidden_intent_terms.iter().any(String::is_empty) {
                    return Err("empty_forbidden_intent");
                }
                Ok(())
            }
            ResponseOperation::WaitOnYieldedCell {
                function_name,
                yield_time_ms,
                max_tokens,
            }
            | ResponseOperation::WaitOnAnyYieldedCell {
                function_name,
                yield_time_ms,
                max_tokens,
            } => {
                if function_name != "wait"
                    || !(250..=30_000).contains(yield_time_ms)
                    || *max_tokens == 0
                    || *max_tokens > 20_000
                {
                    return Err("invalid_wait_program");
                }
                Ok(())
            }
            ResponseOperation::WaitOnYieldedSurfaces {
                surfaces,
                function_name,
                yield_time_ms,
                max_tokens,
            } => {
                validate_phrases(surfaces, "empty_wait_surfaces")?;
                if function_name != "wait"
                    || !(250..=30_000).contains(yield_time_ms)
                    || *max_tokens == 0
                    || *max_tokens > 20_000
                {
                    return Err("invalid_wait_program");
                }
                Ok(())
            }
        }
    }
}

fn validate_render_sequence(segments: &[ResponseRenderSegment]) -> Result<(), &'static str> {
    if !(2..=32).contains(&segments.len()) {
        return Err("invalid_render_sequence_length");
    }
    let primary_count = segments
        .iter()
        .filter(|segment| matches!(segment, ResponseRenderSegment::Primary))
        .count();
    let selected_count = segments
        .iter()
        .filter(|segment| matches!(segment, ResponseRenderSegment::Selected { .. }))
        .count();
    if primary_count == 0 || primary_count > 8 || primary_count.saturating_add(selected_count) < 2 {
        return Err("invalid_render_sequence_dynamic_segments");
    }
    let mut static_text = String::new();
    let mut previous_static = false;
    for segment in segments {
        match segment {
            ResponseRenderSegment::Static { text } => {
                if text.is_empty() || previous_static {
                    return Err("invalid_render_sequence_static_segment");
                }
                static_text.push_str(text);
                previous_static = true;
            }
            ResponseRenderSegment::Primary => previous_static = false,
            ResponseRenderSegment::Selected {
                selector,
                format: _,
            } => {
                validate_selector(selector)?;
                if selector_value_type(selector) == crate::AtomValueType::Collection {
                    return Err("render_sequence_selector_non_scalar");
                }
                previous_static = false;
            }
        }
    }
    if !safe_collection_renderer(&static_text, "") {
        return Err("unsafe_render_sequence_static_text");
    }
    Ok(())
}

fn safe_collection_renderer(prefix: &str, suffix: &str) -> bool {
    if prefix.len().saturating_add(suffix.len()) > 512 {
        return false;
    }
    let combined = format!("{prefix}{suffix}");
    if combined
        .chars()
        .any(|character| character.is_control() && character != '\n')
    {
        return false;
    }
    if !renderer_static_grammar_allowed(&combined) {
        return false;
    }
    let lower = combined.to_ascii_lowercase();
    if [
        "authorization",
        "bearer ",
        "credential",
        "password",
        "passwd",
        "secret",
        "api_key",
        "api-key",
        "apikey",
        "private_key",
        "private-key",
        "privatekey",
        "cookie",
        "token",
    ]
    .iter()
    .any(|term| lower.contains(term))
        || ["http://", "https://", "www."]
            .iter()
            .any(|term| lower.contains(term))
        || ["/home/", "/etc/", "/var/", "/opt/", "/root/", "/tmp/"]
            .iter()
            .any(|term| lower.contains(term))
        || contains_email_like(&combined)
        || contains_windows_path(&combined)
        || contains_high_entropy_run(&combined)
    {
        return false;
    }
    true
}

fn renderer_static_grammar_allowed(value: &str) -> bool {
    let mut word = String::new();
    for character in value.chars().chain(std::iter::once(' ')) {
        if character.is_alphabetic() {
            word.extend(character.to_lowercase());
            continue;
        }
        if !word.is_empty() {
            if !renderer_word_allowed(&word) {
                return false;
            }
            word.clear();
        }
        if !character.is_whitespace()
            && !matches!(
                character,
                '.' | ','
                    | ':'
                    | ';'
                    | '!'
                    | '?'
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | '\''
                    | '"'
                    | '-'
                    | '/'
                    | '_'
                    | '*'
                    | '`'
                    | '#'
                    | '+'
                    | '|'
                    | '<'
                    | '>'
                    | '%'
            )
        {
            return false;
        }
    }
    true
}

fn renderer_word_allowed(word: &str) -> bool {
    matches!(
        word,
        "selected"
            | "select"
            | "value"
            | "values"
            | "result"
            | "results"
            | "count"
            | "total"
            | "status"
            | "item"
            | "items"
            | "row"
            | "rows"
            | "record"
            | "records"
            | "entry"
            | "entries"
            | "matching"
            | "matched"
            | "filtered"
            | "found"
            | "output"
            | "data"
            | "is"
            | "are"
            | "was"
            | "were"
            | "success"
            | "failure"
            | "passed"
            | "failed"
            | "true"
            | "false"
            | "none"
            | "empty"
            | "выбрано"
            | "выбранные"
            | "значение"
            | "значения"
            | "результат"
            | "результаты"
            | "количество"
            | "всего"
            | "статус"
            | "элемент"
            | "элементы"
            | "строка"
            | "строки"
            | "запись"
            | "записи"
            | "найдено"
            | "найденные"
            | "отфильтровано"
            | "успешно"
            | "ошибка"
            | "да"
            | "нет"
            | "пусто"
            | "данные"
            | "на"
            | "не"
            | "это"
            | "его"
            | "уже"
            | "был"
            | "есть"
            | "то"
            | "только"
            | "проверка"
            | "готово"
            | "заблокирован"
            | "подтвердила"
            | "должен"
            | "обновлять"
            | "автоматически"
            | "обновил"
            | "остался"
            | "тронул"
            | "apt"
            | "chrome"
            | "hold"
    )
}

fn contains_email_like(value: &str) -> bool {
    value.split_whitespace().any(|word| {
        let word = word.trim_matches(|character: char| {
            !character.is_alphanumeric()
                && character != '@'
                && character != '.'
                && character != '_'
                && character != '-'
                && character != '+'
        });
        word.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty() && domain.contains('.') && !domain.ends_with('.')
        })
    })
}

fn contains_windows_path(value: &str) -> bool {
    value.as_bytes().windows(3).any(|window| {
        window[0].is_ascii_alphabetic() && window[1] == b':' && matches!(window[2], b'\\' | b'/')
    }) || value.contains("\\\\")
}

fn contains_high_entropy_run(value: &str) -> bool {
    value
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || matches!(character, '+' | '/' | '_' | '-'))
        })
        .any(|run| {
            if run.len() < 24 {
                return false;
            }
            let has_lower = run.bytes().any(|byte| byte.is_ascii_lowercase());
            let has_upper = run.bytes().any(|byte| byte.is_ascii_uppercase());
            let has_digit = run.bytes().any(|byte| byte.is_ascii_digit());
            let long_hex = run.len() >= 32 && run.bytes().all(|byte| byte.is_ascii_hexdigit());
            long_hex || (has_lower && has_upper && has_digit)
        })
}

const fn selector_value_type(selector: &ResponseValueSelector) -> crate::AtomValueType {
    match selector {
        ResponseValueSelector::UniqueScalar { value_type }
        | ResponseValueSelector::UniqueTurnScalar { value_type }
        | ResponseValueSelector::ContentLinePrefix { value_type, .. }
        | ResponseValueSelector::JsonField { value_type, .. }
        | ResponseValueSelector::JsonScalarOrdinal { value_type, .. }
        | ResponseValueSelector::UniqueTurnJsonField { value_type, .. }
        | ResponseValueSelector::UniqueActiveTurnJsonField { value_type, .. }
        | ResponseValueSelector::TurnOutputLine { value_type, .. } => *value_type,
        ResponseValueSelector::CommandOutputBody
        | ResponseValueSelector::RequestLastToken
        | ResponseValueSelector::RequestUniqueLiteral => crate::AtomValueType::String,
    }
}

fn validate_arguments(arguments: &[ResponseArgument]) -> Result<(), &'static str> {
    if arguments.is_empty() || arguments.len() > 16 {
        return Err("invalid_function_arguments");
    }
    let mut names = std::collections::BTreeSet::new();
    let mut role_count = 0_usize;
    for argument in arguments {
        let name = match argument {
            ResponseArgument::Role { name, .. }
            | ResponseArgument::Integer { name, .. }
            | ResponseArgument::String { name, .. }
            | ResponseArgument::Boolean { name, .. } => name,
        };
        validate_identifier(name, "invalid_argument_name")?;
        if !names.insert(name) {
            return Err("duplicate_argument_name");
        }
        match argument {
            ResponseArgument::Role { .. } => role_count += 1,
            ResponseArgument::String { value, .. }
                if value.len() > 1_024 || value.bytes().any(|byte| byte == 0) =>
            {
                return Err("invalid_string_argument");
            }
            _ => {}
        }
    }
    if role_count != 1 {
        return Err("invalid_role_argument_count");
    }
    Ok(())
}

fn validate_selector(selector: &ResponseValueSelector) -> Result<(), &'static str> {
    match selector {
        ResponseValueSelector::UniqueScalar { .. }
        | ResponseValueSelector::UniqueTurnScalar { .. } => Ok(()),
        ResponseValueSelector::JsonScalarOrdinal { ordinal, .. } => {
            if *ordinal < 64 {
                Ok(())
            } else {
                Err("invalid_selector_scalar_ordinal")
            }
        }
        ResponseValueSelector::ContentLinePrefix { prefix, .. } => {
            if prefix.is_empty()
                || prefix.len() > 128
                || prefix.bytes().any(|byte| byte.is_ascii_control())
                || selector_text_looks_private(prefix)
            {
                Err("invalid_selector_prefix")
            } else {
                Ok(())
            }
        }
        ResponseValueSelector::JsonField { field, .. }
        | ResponseValueSelector::UniqueTurnJsonField { field, .. }
        | ResponseValueSelector::UniqueActiveTurnJsonField { field, .. } => {
            validate_identifier(field, "invalid_selector_field")?;
            if field.len() > 64 || selector_text_looks_private(field) {
                Err("invalid_selector_field")
            } else {
                Ok(())
            }
        }
        ResponseValueSelector::TurnOutputLine {
            output_ordinal,
            line_index,
            value_type,
        } => {
            if *output_ordinal == 0
                || *output_ordinal > 64
                || *line_index > 255
                || *value_type != crate::AtomValueType::String
            {
                Err("invalid_turn_output_line_selector")
            } else {
                Ok(())
            }
        }
        ResponseValueSelector::CommandOutputBody
        | ResponseValueSelector::RequestLastToken
        | ResponseValueSelector::RequestUniqueLiteral => Ok(()),
    }
}

fn selector_text_looks_private(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let compact = lower
        .bytes()
        .filter(|byte| !matches!(byte, b'_' | b' ' | b'\t'))
        .collect::<Vec<_>>();
    lower.contains("://")
        || lower.contains('@')
        || lower.contains("/home/")
        || lower.contains("/root/")
        || lower.contains("\\")
        || [
            "auth",
            "cookie",
            "credential",
            "passwd",
            "password",
            "secret",
            "token",
        ]
        .iter()
        .any(|private| lower.contains(private))
        || compact
            .windows(b"apikey".len())
            .any(|window| window == b"apikey")
        || compact
            .windows(b"privatekey".len())
            .any(|window| window == b"privatekey")
        || has_high_entropy_run(value)
}

fn has_high_entropy_run(value: &str) -> bool {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|part| {
            part.len() >= 32
                && part.bytes().any(|byte| byte.is_ascii_digit())
                && part.bytes().any(|byte| byte.is_ascii_alphabetic())
        })
}

fn validate_identifier(value: &str, reason: &'static str) -> Result<(), &'static str> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(reason);
    }
    Ok(())
}

fn validate_phrases(values: &[String], empty_reason: &'static str) -> Result<(), &'static str> {
    if values.is_empty() {
        return Err(empty_reason);
    }
    if values.iter().any(String::is_empty) {
        return Err("empty_program_phrase");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AtomValueType;

    fn rendered_collection(prefix: &str, suffix: &str) -> ResponseProgram {
        ResponseProgram::compose_collection(
            vec![
                CollectionProgramStep::SelectOnlyArrayField,
                CollectionProgramStep::Count,
            ],
            ValueProjectionFormat::PlainText,
            "completed",
        )
        .with_collection_renderer(CollectionOutputRenderer::RenderTemplate {
            prefix: prefix.to_owned(),
            suffix: suffix.to_owned(),
        })
    }

    #[test]
    fn collection_renderer_validator_rejects_private_and_unbounded_static_text() {
        assert_eq!(
            rendered_collection("Selected values: ", ".").validate(),
            Ok(())
        );
        assert_eq!(rendered_collection("Результат: ", ".").validate(), Ok(()));
        for (prefix, suffix) in [
            ("Authorization: Bearer AbC123", ""),
            ("email=user@example.com ", ""),
            ("source=/home/ubu/private.json ", ""),
            ("source=C:\\private\\data.json ", ""),
            ("key=AbCdEfGhIjKlMnOpQrStUv123456 ", ""),
            ("digest=0123456789abcdef0123456789abcdef ", ""),
            ("line one\nline two ", ""),
            ("Клиент Иван Иванов: ", ""),
            ("Телефон +7 999 123-45-67: ", ""),
            ("Адрес Невский проспект: ", ""),
            ("Client Acme Corporation: ", ""),
        ] {
            assert_eq!(
                rendered_collection(prefix, suffix).validate(),
                Err("invalid_collection_renderer"),
                "{prefix}{suffix}"
            );
        }
        assert_eq!(
            rendered_collection(&"x".repeat(513), "").validate(),
            Err("invalid_collection_renderer")
        );
    }

    #[test]
    fn selected_value_projection_has_a_bounded_typed_contract() {
        let program = ResponseProgram::project_selected_value(
            ResponseValueSelector::JsonField {
                field: "status".to_owned(),
                value_type: AtomValueType::Identifier,
            },
            ValueProjectionFormat::PlainText,
            "completed",
        );
        assert_eq!(program.max_output_bytes, 4_096);
        assert_eq!(program.validate(), Ok(()));
    }

    #[test]
    fn selected_value_projection_rejects_unknown_completion_state() {
        let program = ResponseProgram::project_selected_value(
            ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::Integer,
            },
            ValueProjectionFormat::CanonicalJson,
            "guessed",
        );
        assert_eq!(
            program.validate(),
            Err("invalid_projection_completion_state")
        );
    }

    #[test]
    fn selectors_reject_private_or_high_entropy_material() {
        for selector in [
            ResponseValueSelector::JsonField {
                field: "customer@example_com".to_owned(),
                value_type: AtomValueType::String,
            },
            ResponseValueSelector::ContentLinePrefix {
                prefix: "https://private.example/value=".to_owned(),
                value_type: AtomValueType::String,
            },
            ResponseValueSelector::ContentLinePrefix {
                prefix: "TOKEN_0123456789abcdef0123456789abcdef=".to_owned(),
                value_type: AtomValueType::String,
            },
        ] {
            let program = ResponseProgram::project_selected_value(
                selector,
                ValueProjectionFormat::PlainText,
                "completed",
            );
            assert!(program.validate().is_err());
        }
    }

    #[test]
    fn selector_privacy_matches_canonical_structural_names_without_blocking_session_id() {
        for field in [
            "password",
            "passwd_hash",
            "client_secret",
            "access_token",
            "api_key",
            "apikey",
            "private_key_id",
            "privatekey",
            "authorization",
            "cookie_jar",
            "credential_id",
        ] {
            let program = ResponseProgram::project_status(
                ResponseValueSelector::JsonField {
                    field: field.to_owned(),
                    value_type: AtomValueType::Integer,
                },
                ProjectStatusMapping::ZeroIsSuccess,
                "completed",
            );
            assert_eq!(program.validate(), Err("invalid_selector_field"), "{field}");
        }
        for prefix in ["API KEY=", "PRIVATE KEY=", "TOKEN="] {
            let program = ResponseProgram::project_status(
                ResponseValueSelector::ContentLinePrefix {
                    prefix: prefix.to_owned(),
                    value_type: AtomValueType::Integer,
                },
                ProjectStatusMapping::ZeroIsSuccess,
                "completed",
            );
            assert_eq!(
                program.validate(),
                Err("invalid_selector_prefix"),
                "{prefix}"
            );
        }
        let benign = ResponseProgram::project_status(
            ResponseValueSelector::JsonField {
                field: "session_id".to_owned(),
                value_type: AtomValueType::Integer,
            },
            ProjectStatusMapping::ZeroIsSuccess,
            "completed",
        );
        assert_eq!(benign.validate(), Ok(()));
    }

    #[test]
    fn project_status_has_a_bounded_integer_only_contract() {
        let program = ResponseProgram::project_status(
            ResponseValueSelector::JsonField {
                field: "exit_code".to_owned(),
                value_type: AtomValueType::Integer,
            },
            ProjectStatusMapping::ZeroIsSuccess,
            "completed",
        );
        assert_eq!(program.max_output_bytes, "failure".len());
        assert_eq!(program.validate(), Ok(()));
        assert_eq!(ProjectStatusValue::Success.canonical_text(), "success");
        assert_eq!(ProjectStatusValue::Failure.canonical_text(), "failure");
        assert_eq!(MAX_PROJECT_STATUS_CODE, 1_000_000);
    }

    #[test]
    fn project_status_rejects_non_integer_selectors_and_unknown_completion() {
        for value_type in [
            AtomValueType::String,
            AtomValueType::Boolean,
            AtomValueType::Identifier,
            AtomValueType::Collection,
        ] {
            let program = ResponseProgram::project_status(
                ResponseValueSelector::UniqueScalar { value_type },
                ProjectStatusMapping::ZeroIsSuccess,
                "completed",
            );
            assert_eq!(program.validate(), Err("status_selector_must_be_integer"));
        }
        let invalid_completion = ResponseProgram::project_status(
            ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::Integer,
            },
            ProjectStatusMapping::ZeroIsSuccess,
            "unknown",
        );
        assert_eq!(
            invalid_completion.validate(),
            Err("invalid_status_completion_state")
        );
    }

    #[test]
    fn project_status_keeps_selector_privacy_validation() {
        let program = ResponseProgram::project_status(
            ResponseValueSelector::ContentLinePrefix {
                prefix: "https://private.example/exit_code=".to_owned(),
                value_type: AtomValueType::Integer,
            },
            ProjectStatusMapping::ZeroIsSuccess,
            "completed",
        );
        assert_eq!(program.validate(), Err("invalid_selector_prefix"));
    }
}
