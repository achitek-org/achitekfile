//! This module provides the user-facing API for parsing a source
//! achitekfile.
//!
//! The API is forgiving in that the exposed [`AnalysisError`]
//! is reserved for infrastructure failures only; if errors are
//! detected, they are returned as structured diagnostics.

mod build_model;
mod diagnostics;
mod syntax;
mod validate;

use self::{build_model::build_file, diagnostics::collect_diagnostics, validate::validate_file};
use super::{
    Diagnostic,
    model::{AchitekFile, ValidAchitekFile},
    parser::{self, ParseError},
};
use thiserror::Error;

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
    let file = build_file(&tree, source);
    let diagnostics = collect_diagnostics(&tree, source, &file);

    Ok(Analysis {
        source,
        file,
        diagnostics,
    })
}
