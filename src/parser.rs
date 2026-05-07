use thiserror::Error;
use tree_sitter::{Language, Parser, Tree};

/// Parses Achitekfile source text into a Tree-sitter [`Tree`].
///
/// This is a low level API that you generally shouldn't need to use.
///
/// This function configures a Tree-sitter parser with the Achitekfile grammar,
/// parses the supplied source, and returns the raw Tree-sitter parse tree.
///
/// Prefer [`crate::analyze`] unless you specifically need low-level
/// Tree-sitter access.
///
/// ```
/// let source = r#"
/// blueprint {
///   version = "1.0.0"
///   name = "example"
/// }
///
/// prompt "project_name" {
///   type = string
///   help = "Project name"
/// }
/// "#;
///
/// let tree = achitekfile::parse_tree(source)?;
///
/// assert_eq!(tree.root_node().kind(), "file");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Returns [`ParseError::Language`] if the parser cannot be configured with the
/// Achitek grammar, or [`ParseError::ParseCancelled`] if Tree-sitter does not
/// produce a tree.
pub fn parse_tree(source: &str) -> Result<Tree, ParseError> {
    let mut parser = Parser::new();
    let language: Language = tree_sitter_achitekfile::LANGUAGE.into();
    parser.set_language(&language)?;
    let ast: Tree = parser
        .parse(source, None)
        .ok_or(ParseError::ParseCancelled)?;

    Ok(ast)
}

/// Errors that can occur while parsing source text into a Tree-sitter [`Tree`].
///
/// See [`parse_tree`] for an example of handling parser setup and Tree-sitter
/// parse failures with `?`.
#[derive(Debug, Error)]
pub enum ParseError {
    /// The Achitek grammar could not be installed into the parser.
    #[error("failed to configure the Achitek parser: {0}")]
    Language(#[from] tree_sitter::LanguageError),
    /// Parsing was interrupted before Tree-sitter produced a tree.
    #[error("tree-sitter did not produce a parse tree")]
    ParseCancelled,
}
