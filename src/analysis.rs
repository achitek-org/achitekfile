//! This module provides the user-facing API for parsing a source
//! achitekfile.
//!
//! The API is forgiving in that the exposed [`AnalysisError`]
//! is reserved for infrastructure failures only; if errors are
//! detected, they are returned as structured diagnostics.

use super::{
    Diagnostic, DiagnosticCode, TextPosition, TextRange,
    model::{
        AchitekFile, Blueprint, ComparisonOperator, Dependency, Prompt, PromptType, Spanned,
        ValidAchitekFile, ValidBlueprint, ValidPrompt, Validation, Value,
    },
    parser::{self, ParseError},
};
use thiserror::Error;
use tree_sitter::{Node, Point, Tree};

/// A forgiving analysis result for Achitekfile source.
pub struct Analysis<'a> {
    source: &'a str,
    file: AchitekFile,
    diagnostics: Vec<Diagnostic>,
}
impl<'a> Analysis<'a> {
    /// Returns the source text analyzed by this result.
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// Returns the recovered Achitekfile model.
    pub fn file(&self) -> &AchitekFile {
        &self.file
    }

    /// Returns diagnostics discovered while analyzing the source.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Returns true when any diagnostic has error severity.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity() == super::Severity::Error)
    }

    /// Converts this forgiving analysis into a validated Achitekfile model.
    ///
    /// This succeeds only when analysis has no error diagnostics and the
    /// recovered model contains the required runtime fields. On failure, the
    /// returned diagnostics describe why the source cannot be treated as a
    /// valid executable Achitekfile.
    pub fn into_valid(self) -> Result<ValidAchitekFile, Vec<Diagnostic>> {
        if self.has_errors() {
            return Err(self.diagnostics);
        }

        validate_file(self.file)
    }
}

/// Errors that prevent Achitekfile analysis from running.
///
/// Normal source violations are returned as diagnostics in [`Analysis`], not as
/// `AnalysisError`.
#[derive(Debug, Error)]
pub enum AnalysisError {
    /// The source could not be parsed into a Tree-sitter tree.
    #[error("failed to parse achitekfile source: {0}")]
    Parse(#[from] ParseError),
}

/// Analyzes Achitekfile source and returns a forgiving analysis result.
///
/// Syntax errors in the source are collected as diagnostics. This function only
/// returns an error when the parser cannot be configured or Tree-sitter does not
/// produce a parse tree.
pub fn analyze(source: &str) -> Result<Analysis<'_>, AnalysisError> {
    let tree = parser::from_str(source)?;
    let diagnostics = syntax_diagnostics(&tree);
    let file = build_file(&tree, source);

    Ok(Analysis {
        source,
        file,
        diagnostics,
    })
}

fn build_file(tree: &Tree, source: &str) -> AchitekFile {
    let root = tree.root_node();
    let mut cursor = root.walk();
    let mut blueprint = Blueprint::default();
    let mut prompts = Vec::new();

    for child in root.named_children(&mut cursor) {
        match child.kind() {
            "blueprint_block" => {
                blueprint = parse_blueprint(child, source);
            }
            "prompt_block" => {
                if let Some(prompt) = parse_prompt(child, source) {
                    prompts.push(prompt);
                }
            }
            _ => {}
        }
    }

    AchitekFile::new(blueprint, prompts)
}

fn validate_file(file: AchitekFile) -> Result<ValidAchitekFile, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let blueprint = validate_blueprint(file.blueprint(), &mut diagnostics);
    let prompts = file
        .prompts()
        .iter()
        .map(validate_prompt)
        .collect::<Vec<_>>();

    if diagnostics.is_empty() {
        Ok(ValidAchitekFile::new(blueprint, prompts))
    } else {
        Err(diagnostics)
    }
}

fn validate_blueprint(blueprint: &Blueprint, diagnostics: &mut Vec<Diagnostic>) -> ValidBlueprint {
    let version = match &blueprint.version {
        Some(version) => version.value.clone(),
        None => {
            diagnostics.push(Diagnostic::with_message(
                DiagnosticCode::MissingBlueprintVersion,
                TextRange::default(),
                "missing required blueprint `version` attribute",
            ));
            String::new()
        }
    };
    let name = match &blueprint.name {
        Some(name) => name.value.clone(),
        None => {
            diagnostics.push(Diagnostic::with_message(
                DiagnosticCode::MissingBlueprintName,
                TextRange::default(),
                "missing required blueprint `name` attribute",
            ));
            String::new()
        }
    };

    ValidBlueprint {
        version,
        name,
        description: blueprint
            .description
            .as_ref()
            .map(|description| description.value.clone()),
        author: blueprint.author.as_ref().map(|author| author.value.clone()),
        min_achitek_version: blueprint
            .min_achitek_version
            .as_ref()
            .map(|version| version.value.clone()),
    }
}

fn validate_prompt(prompt: &Spanned<Prompt>) -> ValidPrompt {
    let prompt = &prompt.value;

    ValidPrompt {
        name: prompt.name.clone(),
        prompt_type: prompt.prompt_type,
        help: prompt.help.clone(),
        choices: prompt.choices.clone(),
        default: prompt.default.clone(),
        required: prompt.required.unwrap_or(false),
        depends_on: prompt.depends_on.clone(),
        validation: prompt.validation.clone(),
    }
}

fn named_children(node: Node<'_>) -> std::vec::IntoIter<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .collect::<Vec<_>>()
        .into_iter()
}

fn parse_blueprint(node: Node<'_>, source: &str) -> Blueprint {
    let mut blueprint = Blueprint::default();
    for child in named_children(node) {
        if child.kind() != "blueprint_attribute" {
            continue;
        }

        let Some(key_node) = child.child_by_field_name("key") else {
            continue;
        };

        let Some(value_node) = child.child_by_field_name("value") else {
            continue;
        };

        let key = text(key_node, source);
        let Some(value) = parse_string_literal(value_node, source) else {
            continue;
        };
        let spanned = Spanned {
            value,
            range: text_range_for_node(child),
        };

        match key {
            "version" => blueprint.version = Some(spanned),
            "name" => blueprint.name = Some(spanned),
            "description" => blueprint.description = Some(spanned),
            "author" => blueprint.author = Some(spanned),
            "min_achitek_version" => blueprint.min_achitek_version = Some(spanned),
            _ => {}
        }
    }

    blueprint
}

fn parse_prompt(node: Node<'_>, source: &str) -> Option<Spanned<Prompt>> {
    let name_node = node.child_by_field_name("name")?;
    let name = parse_string_literal(name_node, source)?;
    let mut choices: Vec<Value> = Vec::new();
    let mut prompt_type = None;
    let mut help = None;
    let mut default = None;
    let mut required = None;
    let mut depends_on = None;

    for child in named_children(node) {
        if child.kind() != "question_attribute" {
            continue;
        }
        let Some(attribute) = named_children(child).next() else {
            continue;
        };
        let Some(value_node) = attribute.child_by_field_name("value") else {
            continue;
        };

        match attribute.kind() {
            "type_attribute" => prompt_type = parse_prompt_type(value_node, source),
            "help_attribute" => help = parse_string_literal(value_node, source),
            "choices_attribute" => choices = parse_array(value_node, source),
            "default_attribute" => default = parse_value(value_node, source),
            "required_attribute" => required = parse_bool(value_node, source),
            "depends_on_attribute" => depends_on = parse_dependency(value_node, source),
            _ => {}
        }
    }

    Some(Spanned {
        value: Prompt {
            name,
            prompt_type: prompt_type?,
            help,
            choices,
            default,
            required,
            depends_on,
            validation: Validation::default(),
        },
        range: text_range_for_node(node),
    })
}

fn parse_prompt_type(node: Node<'_>, source: &str) -> Option<PromptType> {
    match text(node, source) {
        "string" => Some(PromptType::String),
        "paragraph" => Some(PromptType::Paragraph),
        "bool" => Some(PromptType::Bool),
        "select" => Some(PromptType::Select),
        "multiselect" => Some(PromptType::MultiSelect),
        _ => None,
    }
}

fn text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes())
        .expect("tree-sitter node byte ranges should be valid utf-8 slices")
}

fn parse_string_literal(node: Node<'_>, source: &str) -> Option<String> {
    let text = text(node, source);
    let without_open = text.strip_prefix('"')?;
    let inner = without_open.strip_suffix('"')?;

    let mut parsed = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            parsed.push(ch);
            continue;
        }

        match chars.next()? {
            'n' => parsed.push('\n'),
            't' => parsed.push('\t'),
            'r' => parsed.push('\r'),
            '"' => parsed.push('"'),
            '\\' => parsed.push('\\'),
            _ => return None,
        }
    }

    Some(parsed)
}

fn parse_array(node: Node<'_>, source: &str) -> Vec<Value> {
    let Some(value_list) = named_children(node).find(|node| node.kind() == "value_list") else {
        return Vec::new();
    };

    named_children(value_list)
        .filter(|node| node.kind() == "value")
        .filter_map(|node| parse_value(node, source))
        .collect()
}

fn parse_value(node: Node<'_>, source: &str) -> Option<Value> {
    let inner = if node.kind() == "value" || node.kind() == "literal_value" {
        named_children(node).next()?
    } else {
        node
    };

    match inner.kind() {
        "string_literal" => parse_string_literal(inner, source).map(Value::String),
        "boolean" => match text(inner, source) {
            "true" => Some(Value::Bool(true)),
            "false" => Some(Value::Bool(false)),
            _ => None,
        },
        "integer" => text(inner, source).parse::<u64>().ok().map(Value::Integer),
        "identifier" => Some(Value::Identifier(text(inner, source).to_owned())),
        "array" => Some(Value::Array(parse_array(inner, source))),
        _ => None,
    }
}

fn parse_bool(node: Node<'_>, source: &str) -> Option<bool> {
    match text(node, source) {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_dependency(node: Node<'_>, source: &str) -> Option<Dependency> {
    let inner = if node.kind() == "dependency_expr" {
        named_children(node).next()?
    } else {
        node
    };

    match inner.kind() {
        "simple_dependency" => {
            let reference = inner.child_by_field_name("reference")?;
            Some(Dependency::Reference(text(reference, source).to_owned()))
        }
        "comparison_dependency" => {
            let left = inner.child_by_field_name("left")?;
            let right = inner.child_by_field_name("right")?;
            Some(Dependency::Comparison {
                left: text(left, source).to_owned(),
                operator: parse_comparison_operator(inner, source)?,
                right: parse_value(right, source)?,
            })
        }
        "method_call_dependency" => {
            let receiver = inner.child_by_field_name("receiver")?;
            let method = inner.child_by_field_name("method")?;
            let argument = inner.child_by_field_name("argument")?;

            if text(method, source) != "contains" {
                return None;
            }

            Some(Dependency::Contains {
                receiver: text(receiver, source).to_owned(),
                argument: parse_value(argument, source)?,
            })
        }
        "combinator_dependency" => {
            let name = inner.child_by_field_name("name")?;
            let arguments = inner.child_by_field_name("arguments")?;
            let dependencies = named_children(arguments)
                .filter(|node| node.kind() == "dependency_expr")
                .filter_map(|node| parse_dependency(node, source))
                .collect::<Vec<_>>();

            match text(name, source) {
                "all" => Some(Dependency::All(dependencies)),
                "any" => Some(Dependency::Any(dependencies)),
                _ => None,
            }
        }
        _ => None,
    }
}

fn parse_comparison_operator(node: Node<'_>, source: &str) -> Option<ComparisonOperator> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match text(child, source) {
            "==" => return Some(ComparisonOperator::Equal),
            "!=" => return Some(ComparisonOperator::NotEqual),
            _ => {}
        }
    }

    None
}

fn syntax_diagnostics(tree: &Tree) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    collect_file_shape_diagnostics(tree.root_node(), &mut diagnostics);
    collect_syntax_diagnostics(tree.root_node(), &mut diagnostics);
    diagnostics
}

fn collect_file_shape_diagnostics(root: Node<'_>, diagnostics: &mut Vec<Diagnostic>) {
    let mut cursor = root.walk();
    let has_blueprint = root
        .named_children(&mut cursor)
        .any(|node| node.kind() == "blueprint_block");

    if !has_blueprint {
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::MissingBlueprintBlock,
            text_range_for_node(root),
        ));
    }
}

fn collect_syntax_diagnostics(node: Node<'_>, diagnostics: &mut Vec<Diagnostic>) {
    if node.is_missing() {
        diagnostics.push(Diagnostic::new(
            missing_node_code(node),
            text_range_for_node(node),
        ));
        return;
    }

    if node.is_error() {
        diagnostics.push(Diagnostic::new(
            error_node_code(node),
            text_range_for_node(node),
        ));
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_syntax_diagnostics(child, diagnostics);
    }
}

fn missing_node_code(node: Node<'_>) -> DiagnosticCode {
    match node.parent().map(|parent| parent.kind()) {
        Some("array" | "value_list") => return DiagnosticCode::MalformedArray,
        Some("depends_on_attribute" | "dependency_expr") => {
            return DiagnosticCode::InvalidDependencyExpression;
        }
        Some("string_literal") => return DiagnosticCode::UnterminatedString,
        _ => {}
    }

    match node.kind() {
        "blueprint_block" => DiagnosticCode::MissingBlueprintBlock,
        "array" | "value_list" => DiagnosticCode::MalformedArray,
        "dependency_expr" => DiagnosticCode::InvalidDependencyExpression,
        "string_literal" => DiagnosticCode::UnterminatedString,
        "identifier" => DiagnosticCode::InvalidIdentifier,
        "integer" => DiagnosticCode::InvalidInteger,
        _ => DiagnosticCode::UnknownTopLevelItem,
    }
}

fn error_node_code(node: Node<'_>) -> DiagnosticCode {
    match node.parent().map(|parent| parent.kind()) {
        Some("array" | "value_list") => DiagnosticCode::MalformedArray,
        Some("depends_on_attribute" | "dependency_expr") => {
            DiagnosticCode::InvalidDependencyExpression
        }
        Some("blueprint_block" | "blueprint_attribute") => {
            DiagnosticCode::UnknownBlueprintAttribute
        }
        Some("prompt_block" | "question_attribute") => DiagnosticCode::UnknownPromptAttribute,
        Some("validate_block" | "validate_attribute") => DiagnosticCode::UnknownValidateAttribute,
        _ => DiagnosticCode::UnknownTopLevelItem,
    }
}

fn text_range_for_node(node: Node<'_>) -> TextRange {
    TextRange {
        start: text_position_for_point(node.start_position()),
        end: text_position_for_point(node.end_position()),
    }
}

fn text_position_for_point(point: Point) -> TextPosition {
    TextPosition {
        line: point.row,
        byte: point.column,
    }
}
