//! A [Tree-Sitter] backed semantic parser for Achitek
//!
//! [Tree-Sitter]: https://tree-sitter.github.io/tree-sitter/
//!
//! ```
//! let source = r#"
//!     blueprint {
//!         version = "1.0.0"
//!         name = "web-app"
//!     }
//!
//!     prompt "database" {
//!         type = select
//!         choices = ["postgres", "sqlite"]
//!         default = "postgres"
//!     }
//!
//!     prompt "orm" {
//!         type = select
//!         choices = ["sqlx", "diesel"]
//!         depends_on = database != "sqlite"
//!     }
//! "#;
//!
//! ```
//!
//! `achitekfile` wraps the [tree-sitter-achitekfile] grammar and exposes a
//! small semantic API over the concrete Tree-sitter syntax tree.
//!
//! [tree-sitter-achitekfile]: https://docs.rs/tree-sitter-achitekfile/0.1.0/tree_sitter_achitekfile/

#![deny(missing_docs)]

mod ast;
mod diagnostics;
mod parser;
mod sort;

pub use ast::{
    AchitekAst, AstError, ComparisonOperator, Dependency, Prompt, PromptType, Validation, Value,
};
pub use diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticKind, Severity, TextPosition, TextRange,
};
pub use parser::{ParseError, from_str};
