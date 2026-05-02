use crate::AchitekAst;
use thiserror::Error;
use tree_sitter::Parser;

/// Parses Achitekfile source text into an [`AchitekAst`].
///
/// This function configures a Tree-sitter parser with the Achitekfile grammar,
/// parses the supplied source, and returns an AST wrapper that keeps the source
/// text available for later node-to-text conversion.
///
/// The returned AST can be used directly for low-level Tree-sitter access or
/// for higher-level semantic operations:
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
/// let ast = achitekfile::from_str(source)?;
/// let prompts = ast.ordered_prompts()?;
///
/// assert_eq!(prompts[0].name, "project_name");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Returns [`ParseError::Language`] if the parser cannot be configured with the
/// Achitek grammar, or [`ParseError::ParseCancelled`] if Tree-sitter does not
/// produce a tree.
pub fn from_str(source: &str) -> Result<AchitekAst<'_>, ParseError> {
    let mut parser = Parser::new();
    let language = tree_sitter_achitekfile::LANGUAGE.into();
    parser.set_language(&language)?;
    let ast = parser
        .parse(source, None)
        .ok_or(ParseError::ParseCancelled)?;

    Ok(AchitekAst::new(ast, language, source))
}

/// Errors that can occur while parsing source text into an [`AchitekAst`].
#[derive(Debug, Error)]
pub enum ParseError {
    /// The Achitek grammar could not be installed into the parser.
    #[error("failed to configure the Achitek parser: {0}")]
    Language(#[from] tree_sitter::LanguageError),
    /// Parsing was interrupted before Tree-sitter produced a tree.
    #[error("tree-sitter did not produce a parse tree")]
    ParseCancelled,
}
