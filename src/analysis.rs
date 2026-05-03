//! This module provides the user-facing API for parsing a source
//! achitekfile.
//!
//! The API is forgiving in that the exposed [`AnalysisError`]
//! is reserved for infrastructure failures only; if errors are
//! detected, they are returned as structured diagnostics.

use super::{
    Diagnostic, DiagnosticCode, TextPosition, TextRange,
    parser::{self, ParseError},
};
use thiserror::Error;
use tree_sitter::{Node, Point, Tree};

/// A forgiving analysis result for Achitekfile source.
pub struct Analysis<'a> {
    source: &'a str,
    diagnostics: Vec<Diagnostic>,
}
impl<'a> Analysis<'a> {
    /// Returns the source text analyzed by this result.
    pub fn source(&self) -> &'a str {
        self.source
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

    Ok(Analysis {
        source,
        diagnostics,
    })
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
