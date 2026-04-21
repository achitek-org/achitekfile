//! `achitek` wraps the [`tree-sitter-achitekfile`] grammar and exposes a
//! small semantic API over the concrete Tree-sitter syntax tree. The parser
//! keeps the original source text and syntax tree together in an [`AchitekAst`],
//! then higher-level methods translate Tree-sitter nodes into Rust data types
//! such as prompts, values, dependencies, and validation rules.
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
//! let ast = achitek::from_str(source)?;
//! let prompts = ast.ordered_prompts()?;
//!
//! assert_eq!(prompts[0].name, "database");
//! assert_eq!(prompts[1].name, "orm");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Syntax tree versus semantic model
//!
//! Tree-sitter produces a concrete syntax tree whose node names mirror the
//! grammar: `file`, `blueprint_block`, `prompt_block`, `question_attribute`,
//! `validate_block`, and so on. Those nodes are excellent for precise parsing,
//! editor integrations, and source locations, but they are lower-level than the
//! data most callers want.
//!
//! [`AchitekAst::fetch_prompts`] walks/query-matches that syntax tree and
//! converts each `prompt_block` into a prompt model. [`AchitekAst::ordered_prompts`]
//! then builds a dependency graph from the parsed prompt dependencies and
//! returns prompts in an order that is suitable for asking questions: every
//! prompt appears after the prompts it references.
//!
//! The dependency graph uses edges in the form `dependency -> dependent`. For
//! example, if `orm` has `depends_on = database != "none"`, the graph contains
//! the edge `database -> orm`.

#![deny(missing_docs)]

mod ast;
mod parser;
pub mod sort;

pub use ast::{
    AchitekAst, AstError, ComparisonOperator, Dependency, Prompt, PromptType, Validation, Value,
};
pub use parser::{ParseError, from_str};
