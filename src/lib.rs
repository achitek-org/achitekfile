//! A [Tree-Sitter] backed semantic parser for Achitek
//!
//! [Tree-Sitter]: https://tree-sitter.github.io/tree-sitter/
//!
//! ```
//! let source = r#"
//! blueprint {
//!   version = "1.0.0"
//!   name = "web-app"
//! }
//!
//! prompt "database" {
//!   type = select
//!   choices = ["postgres", "sqlite"]
//!   default = "postgres"
//! }
//!
//! prompt "orm" {
//!   type = select
//!   choices = ["sqlx", "diesel"]
//!   depends_on = database != "sqlite"
//! }
//! "#;
//!
//! let file = achitekfile::analyze(source)?.into_valid().map_err(|diagnostics| {
//!     let message = diagnostics
//!         .into_iter()
//!         .map(|diagnostic| diagnostic.message().to_owned())
//!         .collect::<Vec<_>>()
//!         .join(", ");
//!     std::io::Error::new(std::io::ErrorKind::InvalidData, message)
//! })?;
//!
//! assert_eq!(file.blueprint().name, "web-app");
//! assert_eq!(file.prompts().len(), 2);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! `achitekfile` wraps the [tree-sitter-achitekfile] grammar and exposes a
//! small semantic API over the concrete Tree-sitter syntax tree.
//!
//! [tree-sitter-achitekfile]: https://docs.rs/tree-sitter-achitekfile/0.1.0/tree_sitter_achitekfile/

#![deny(missing_docs)]

mod analysis;
mod diagnostics;
pub mod model;
mod parser;
mod sort;

pub use analysis::{Analysis, AnalysisError, analyze};
pub use diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticKind, Severity, TextPosition, TextRange,
};
pub use parser::{ParseError, parse_tree};
