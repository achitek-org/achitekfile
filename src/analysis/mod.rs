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
///
/// # Examples
///
/// ```
/// let source = r#"
/// blueprint {
///   version = "1.0.0"
///   name = "web-app"
/// }
///
/// prompt "project_name" {
///   type = string
/// }
/// "#;
///
/// let analysis = achitekfile::analyze(source)?;
///
/// assert!(!analysis.has_errors());
/// assert_eq!(analysis.file().prompts().len(), 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct Analysis<'a> {
    source: &'a str,
    file: AchitekFile,
    diagnostics: Vec<Diagnostic>,
}
impl<'a> Analysis<'a> {
    /// Returns the source text analyzed by this result.
    ///
    /// See [`analyze`] for a complete example.
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// Returns the recovered Achitekfile model.
    ///
    /// See [`analyze`] for a complete example.
    pub fn file(&self) -> &AchitekFile {
        &self.file
    }

    /// Returns diagnostics discovered while analyzing the source.
    ///
    /// See [`analyze`] for a complete example.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Returns true when any diagnostic has error severity.
    ///
    /// See [`analyze`] for a complete example.
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
    ///
    /// # Examples
    ///
    /// ```
    /// let source = r#"
    /// blueprint {
    ///   version = "1.0.0"
    ///   name = "web-app"
    /// }
    ///
    /// prompt "database" {
    ///   type = select
    ///   choices = ["postgres", "sqlite"]
    /// }
    /// "#;
    ///
    /// let file = achitekfile::analyze(source)?.into_valid().map_err(|diagnostics| {
    ///     let message = diagnostics
    ///         .into_iter()
    ///         .map(|diagnostic| diagnostic.message().to_owned())
    ///         .collect::<Vec<_>>()
    ///         .join(", ");
    ///     std::io::Error::new(std::io::ErrorKind::InvalidData, message)
    /// })?;
    ///
    /// assert_eq!(file.blueprint().name, "web-app");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn into_valid(self) -> Result<ValidAchitekFile, Vec<Diagnostic>> {
        if self.has_errors() {
            return Err(self.diagnostics);
        }

        Ok(validate_file(self.file))
    }
}

/// Errors that prevent Achitekfile analysis from running.
///
/// Normal source violations are returned as diagnostics in [`Analysis`], not as
/// `AnalysisError`.
///
/// See [`analyze`] for an example of the distinction between fatal analysis
/// errors and recoverable Achitekfile diagnostics.
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
///
/// # Examples
///
/// ```
/// let source = r#"
/// blueprint {
///   version = "1.0.0"
///   name = "web-app"
/// }
///
/// prompt "project_name" {
///   help = "Project name"
/// }
/// "#;
///
/// let analysis = achitekfile::analyze(source)?;
/// let messages = analysis
///     .diagnostics()
///     .iter()
///     .map(|diagnostic| diagnostic.message())
///     .collect::<Vec<_>>();
///
/// assert!(analysis.has_errors());
/// assert!(messages.contains(&"missing prompt type"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn analyze(source: &str) -> Result<Analysis<'_>, AnalysisError> {
    let tree = parser::parse_tree(source)?;
    let file = build_file(&tree, source);
    let diagnostics = collect_diagnostics(&tree, source, &file);

    Ok(Analysis {
        source,
        file,
        diagnostics,
    })
}
