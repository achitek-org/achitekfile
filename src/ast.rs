use crate::sort::{Graph, SortError, sort_graph};
use std::collections::{HashMap, HashSet};
use tree_sitter::{Language, Node, Query, QueryCursor, QueryError, StreamingIterator, Tree};

/// A parsed prompt declaration from an Achitekfile.
///
/// `Prompt` is the semantic representation of a `prompt "name" { ... }`
/// block. It intentionally hides the grammar wrappers used by Tree-sitter
/// (`question_attribute`, `type_attribute`, `validate_block`, and friends) and
/// exposes the information a caller needs to decide what to ask and how to
/// validate the answer.
///
/// The prompt name is the stable identifier used by dependency expressions. A
/// prompt such as `depends_on = database != "none"` references the prompt named
/// `database`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    /// The prompt identifier from `prompt "..."`.
    pub name: String,
    /// The kind of input expected for this prompt.
    pub prompt_type: PromptType,
    /// Optional help text shown to a user alongside the prompt.
    pub help: Option<String>,
    /// The allowed choices for `select` and `multiselect` prompts.
    ///
    /// Non-choice prompt types may leave this empty.
    pub choices: Vec<Value>,
    /// The default answer for the prompt, if one was declared.
    pub default: Option<Value>,
    /// Whether the prompt requires an answer.
    ///
    /// `None` means the Achitekfile omitted the `required` attribute and the
    /// caller should apply its own default policy.
    pub required: Option<bool>,
    /// A dependency expression that controls whether this prompt should be
    /// asked.
    pub depends_on: Option<Dependency>,
    /// Validation rules declared in the nested `validate { ... }` block.
    pub validation: Validation,
}

/// The supported prompt input types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptType {
    /// A single-line string answer.
    String,
    /// A multi-line string answer.
    Paragraph,
    /// A boolean answer.
    Bool,
    /// A single choice from the prompt's `choices`.
    Select,
    /// Zero or more choices from the prompt's `choices`.
    MultiSelect,
}

/// A literal or identifier value parsed from an Achitekfile.
///
/// Values are used by prompt defaults, choice arrays, and dependency
/// comparisons. The parser preserves identifiers separately from strings so
/// callers can distinguish `"database"` from `database`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A double-quoted string literal with supported escape sequences decoded.
    String(String),
    /// A `true` or `false` literal.
    Bool(bool),
    /// A non-negative integer literal.
    Integer(u64),
    /// An unquoted identifier.
    Identifier(String),
    /// An array of values.
    Array(Vec<Value>),
}

/// A dependency expression from a prompt's `depends_on` attribute.
///
/// Dependencies are both executable conditions and graph edges. For ordering,
/// every variant can reveal the prompt names it references:
///
/// - `database`
/// - `database != "none"`
/// - `features.contains("auth")`
/// - `all(database != "none", features.contains("auth"))`
///
/// In all of those examples, the referenced prompt names must appear before the
/// dependent prompt returned by [`AchitekAst::ordered_prompts`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dependency {
    /// A direct dependency on another prompt by name, such as `depends_on = database`.
    Reference(String),
    /// A comparison dependency, such as `database != "none"`.
    Comparison {
        /// The prompt name on the left-hand side of the comparison.
        left: String,
        /// The equality operator used by the comparison.
        operator: ComparisonOperator,
        /// The literal value on the right-hand side of the comparison.
        right: Value,
    },
    /// A `contains` dependency, such as `features.contains("auth")`.
    Contains {
        /// The prompt name whose answer is searched.
        receiver: String,
        /// The value expected to be contained in the receiver's answer.
        argument: Value,
    },
    /// A dependency that requires every nested dependency to be true.
    All(Vec<Dependency>),
    /// A dependency that requires at least one nested dependency to be true.
    Any(Vec<Dependency>),
}

impl Dependency {
    fn references(&self) -> Vec<&str> {
        let mut references = Vec::new();
        self.collect_references(&mut references);
        references
    }

    fn collect_references<'a>(&'a self, references: &mut Vec<&'a str>) {
        match self {
            Self::Reference(name) => references.push(name),
            Self::Comparison { left, .. } => references.push(left),
            Self::Contains { receiver, .. } => references.push(receiver),
            Self::All(dependencies) | Self::Any(dependencies) => {
                for dependency in dependencies {
                    dependency.collect_references(references);
                }
            }
        }
    }
}

/// Operators supported by comparison dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
    /// Equality, written as `==`.
    Equal,
    /// Inequality, written as `!=`.
    NotEqual,
}

/// Validation rules for a prompt.
///
/// These fields correspond to attributes inside a `validate { ... }` block.
/// The parser records what the file declares; it does not currently enforce
/// whether a given rule is appropriate for the prompt type.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Validation {
    /// A regular expression that string-like answers must match.
    pub regex: Option<String>,
    /// The minimum accepted string length.
    pub min_length: Option<u64>,
    /// The maximum accepted string length.
    pub max_length: Option<u64>,
    /// The minimum number of selections for a multiselect prompt.
    pub min_selections: Option<u64>,
    /// The maximum number of selections for a multiselect prompt.
    pub max_selections: Option<u64>,
}

/// A parsed Achitekfile backed by its original Tree-sitter tree and source.
///
/// `AchitekAst` is the bridge between concrete syntax and semantic data. It
/// stores:
///
/// - the Tree-sitter [`Language`] used to build queries,
/// - the original source text used to extract node text,
/// - the parsed [`Tree`] itself.
///
/// Most callers should prefer [`crate::analyze`] instead of constructing this
/// wrapper directly.
pub struct AchitekAst<'a> {
    language: Language,
    source: &'a str,
    ast: Tree,
}

impl<'a> AchitekAst<'a> {
    /// Creates an AST wrapper from a parsed Tree-sitter tree.
    ///
    /// The `source` must be the
    /// exact text that produced `ast`; Tree-sitter nodes store byte ranges into
    /// that text, and parsing helpers use those ranges to recover strings,
    /// identifiers, integers, and operators.
    pub fn new(ast: Tree, language: Language, source: &'a str) -> Self {
        Self {
            ast,
            language,
            source,
        }
    }
    /// Returns prompts in dependency order.
    ///
    /// This method first parses all prompt blocks, then converts their dependency
    /// expressions into a graph:
    ///
    /// ```text
    /// dependency prompt -> prompt that depends on it
    /// ```
    ///
    /// For example, this Achitekfile fragment:
    ///
    /// ```text
    /// prompt "orm" {
    ///   type = select
    ///   depends_on = database != "none"
    /// }
    /// ```
    ///
    /// creates the graph edge:
    ///
    /// ```text
    /// database -> orm
    /// ```
    ///
    /// The returned vector is sorted so every referenced prompt appears before
    /// the prompt that references it. Independent prompts retain the order they
    /// had in the source file.
    ///
    /// # Errors
    ///
    /// Returns an error if prompts cannot be parsed, if prompt names are
    /// duplicated, if a dependency references an unknown prompt, or if the
    /// dependency graph contains a cycle.
    pub fn ordered_prompts(&self) -> Result<Vec<Prompt>, AstError> {
        let prompts = self.fetch_prompts()?;
        let prompt_names = prompts
            .iter()
            .map(|prompt| prompt.name.clone())
            .collect::<Vec<_>>();
        let prompt_name_set = prompt_names
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let mut seen_names = HashSet::new();

        for name in &prompt_names {
            if !seen_names.insert(name.as_str()) {
                return Err(AstError::Unknown);
            }
        }

        let mut edges = Vec::new();
        for prompt in &prompts {
            let Some(dependency) = &prompt.depends_on else {
                continue;
            };

            for reference in dependency.references() {
                if !prompt_name_set.contains(reference) {
                    return Err(AstError::Unknown);
                }

                edges.push((reference.to_owned(), prompt.name.clone()));
            }
        }

        let graph = Graph {
            nodes: prompt_names,
            edges,
        };
        let sorted_names = sort_graph(&graph)?;
        let mut prompts_by_name = prompts
            .into_iter()
            .map(|prompt| (prompt.name.clone(), prompt))
            .collect::<HashMap<_, _>>();

        sorted_names
            .into_iter()
            .map(|name| prompts_by_name.remove(&name).ok_or(AstError::Unknown))
            .collect()
    }

    /// Extracts all prompt blocks from the syntax tree.
    ///
    /// This method uses a Tree-sitter query to find every `prompt_block` with a
    /// `name` field, then parses each block into a [`Prompt`]. The returned
    /// prompts are in source order. Use [`Self::ordered_prompts`] when prompt
    /// dependencies should affect the order.
    ///
    /// # Errors
    ///
    /// Returns an error if the query cannot be compiled or if any prompt block
    /// contains malformed or unsupported syntax.
    fn fetch_prompts(&self) -> Result<Vec<Prompt>, AstError> {
        let root = self.ast.root_node();
        let query = Query::new(
            &self.language,
            r#"
            (prompt_block
              name: (string_literal) @prompt.name) @prompt.block
            "#,
        )?;
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, self.source.as_bytes());
        let prompt_block_capture = query
            .capture_names()
            .iter()
            .position(|name| *name == "prompt.block")
            .ok_or(AstError::Unknown)? as u32;

        let mut prompts = Vec::new();
        while let Some(query_match) = matches.next() {
            for capture in query_match.captures {
                if capture.index == prompt_block_capture {
                    prompts.push(self.parse_prompt(capture.node)?);
                }
            }
        }

        Ok(prompts)
    }

    fn parse_prompt(&self, node: Node<'_>) -> Result<Prompt, AstError> {
        let name = node
            .child_by_field_name("name")
            .ok_or(AstError::Unknown)
            .and_then(|name| self.parse_string_literal(name))?;

        let mut prompt_type = None;
        let mut help = None;
        let mut choices = Vec::new();
        let mut default = None;
        let mut required = None;
        let mut depends_on = None;
        let mut validation = Validation::default();

        for child in named_children(node) {
            match child.kind() {
                "question_attribute" => {
                    let attribute = named_children(child).next().ok_or(AstError::Unknown)?;
                    let value = attribute
                        .child_by_field_name("value")
                        .ok_or(AstError::Unknown)?;

                    match attribute.kind() {
                        "type_attribute" => prompt_type = Some(self.parse_prompt_type(value)?),
                        "help_attribute" => help = Some(self.parse_string_literal(value)?),
                        "choices_attribute" => choices = self.parse_array(value)?,
                        "default_attribute" => default = Some(self.parse_value(value)?),
                        "required_attribute" => required = Some(self.parse_bool(value)?),
                        "depends_on_attribute" => depends_on = Some(self.parse_dependency(value)?),
                        _ => return Err(AstError::Unknown),
                    }
                }
                "validate_block" => self.parse_validation(child, &mut validation)?,
                "string_literal" => {}
                _ => {}
            }
        }

        Ok(Prompt {
            name,
            prompt_type: prompt_type.ok_or(AstError::Unknown)?,
            help,
            choices,
            default,
            required,
            depends_on,
            validation,
        })
    }

    fn parse_validation(
        &self,
        node: Node<'_>,
        validation: &mut Validation,
    ) -> Result<(), AstError> {
        for item in named_children(node).filter(|node| node.kind() == "validate_attribute") {
            let attribute = named_children(item).next().ok_or(AstError::Unknown)?;
            let value = attribute
                .child_by_field_name("value")
                .ok_or(AstError::Unknown)?;

            match attribute.kind() {
                "regex_attribute" => validation.regex = Some(self.parse_string_literal(value)?),
                "min_length_attribute" => validation.min_length = Some(self.parse_integer(value)?),
                "max_length_attribute" => validation.max_length = Some(self.parse_integer(value)?),
                "min_selections_attribute" => {
                    validation.min_selections = Some(self.parse_integer(value)?)
                }
                "max_selections_attribute" => {
                    validation.max_selections = Some(self.parse_integer(value)?)
                }
                _ => return Err(AstError::Unknown),
            }
        }

        Ok(())
    }

    fn parse_prompt_type(&self, node: Node<'_>) -> Result<PromptType, AstError> {
        match self.text(node) {
            "string" => Ok(PromptType::String),
            "paragraph" => Ok(PromptType::Paragraph),
            "bool" => Ok(PromptType::Bool),
            "select" => Ok(PromptType::Select),
            "multiselect" => Ok(PromptType::MultiSelect),
            _ => Err(AstError::Unknown),
        }
    }

    fn parse_dependency(&self, node: Node<'_>) -> Result<Dependency, AstError> {
        let inner = if node.kind() == "dependency_expr" {
            named_children(node).next().unwrap_or(node)
        } else {
            node
        };

        match inner.kind() {
            "simple_dependency" => {
                let reference = inner
                    .child_by_field_name("reference")
                    .ok_or(AstError::Unknown)?;
                Ok(Dependency::Reference(self.text(reference).to_owned()))
            }
            "comparison_dependency" => {
                let left = inner.child_by_field_name("left").ok_or(AstError::Unknown)?;
                let right = inner
                    .child_by_field_name("right")
                    .ok_or(AstError::Unknown)?;
                Ok(Dependency::Comparison {
                    left: self.text(left).to_owned(),
                    operator: self.parse_comparison_operator(inner)?,
                    right: self.parse_value(right)?,
                })
            }
            "method_call_dependency" => {
                let receiver = inner
                    .child_by_field_name("receiver")
                    .ok_or(AstError::Unknown)?;
                let argument = inner
                    .child_by_field_name("argument")
                    .ok_or(AstError::Unknown)?;
                Ok(Dependency::Contains {
                    receiver: self.text(receiver).to_owned(),
                    argument: self.parse_value(argument)?,
                })
            }
            "combinator_dependency" => {
                let name = inner.child_by_field_name("name").ok_or(AstError::Unknown)?;
                let arguments = inner
                    .child_by_field_name("arguments")
                    .ok_or(AstError::Unknown)?;
                let dependencies = named_children(arguments)
                    .filter(|node| node.kind() == "dependency_expr")
                    .map(|node| self.parse_dependency(node))
                    .collect::<Result<Vec<_>, _>>()?;

                match self.text(name) {
                    "all" => Ok(Dependency::All(dependencies)),
                    "any" => Ok(Dependency::Any(dependencies)),
                    _ => Err(AstError::Unknown),
                }
            }
            _ => Err(AstError::Unknown),
        }
    }

    fn parse_comparison_operator(&self, node: Node<'_>) -> Result<ComparisonOperator, AstError> {
        for index in 0..node.child_count() as u32 {
            let Some(child) = node.child(index) else {
                continue;
            };
            match self.text(child) {
                "==" => return Ok(ComparisonOperator::Equal),
                "!=" => return Ok(ComparisonOperator::NotEqual),
                _ => {}
            }
        }

        Err(AstError::Unknown)
    }

    fn parse_value(&self, node: Node<'_>) -> Result<Value, AstError> {
        let inner = if node.kind() == "value" || node.kind() == "literal_value" {
            named_children(node).next().unwrap_or(node)
        } else {
            node
        };

        match inner.kind() {
            "string_literal" => self.parse_string_literal(inner).map(Value::String),
            "boolean" => self.parse_bool(inner).map(Value::Bool),
            "integer" => self.parse_integer(inner).map(Value::Integer),
            "identifier" => Ok(Value::Identifier(self.text(inner).to_owned())),
            "array" => self.parse_array(inner).map(Value::Array),
            _ => Err(AstError::Unknown),
        }
    }

    fn parse_array(&self, node: Node<'_>) -> Result<Vec<Value>, AstError> {
        let Some(value_list) = named_children(node).find(|node| node.kind() == "value_list") else {
            return Ok(Vec::new());
        };

        named_children(value_list)
            .filter(|node| node.kind() == "value")
            .map(|node| self.parse_value(node))
            .collect()
    }

    fn parse_string_literal(&self, node: Node<'_>) -> Result<String, AstError> {
        let text = self.text(node);
        let without_open = text.strip_prefix('"').ok_or(AstError::Unknown)?;
        let inner = without_open.strip_suffix('"').ok_or(AstError::Unknown)?;

        let mut parsed = String::new();
        let mut chars = inner.chars();
        while let Some(ch) = chars.next() {
            if ch != '\\' {
                parsed.push(ch);
                continue;
            }

            match chars.next() {
                Some('n') => parsed.push('\n'),
                Some('t') => parsed.push('\t'),
                Some('r') => parsed.push('\r'),
                Some('"') => parsed.push('"'),
                Some('\\') => parsed.push('\\'),
                Some(_) | None => return Err(AstError::Unknown),
            }
        }

        Ok(parsed)
    }

    fn parse_bool(&self, node: Node<'_>) -> Result<bool, AstError> {
        match self.text(node) {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(AstError::Unknown),
        }
    }

    fn parse_integer(&self, node: Node<'_>) -> Result<u64, AstError> {
        self.text(node).parse().map_err(|_| AstError::Unknown)
    }

    fn text(&self, node: Node<'_>) -> &'a str {
        node.utf8_text(self.source.as_bytes())
            .expect("tree-sitter node byte ranges should be valid utf-8 slices")
    }
}

/// Errors that can occur while reading semantic data from an [`AchitekAst`].
#[derive(Debug, thiserror::Error)]
pub enum AstError {
    /// A syntax-tree shape or value could not be converted into the semantic
    /// model.
    #[error("something happened")]
    Unknown,
    /// Prompt dependencies form a cycle, so no valid prompt order exists.
    #[error("cycle detected in prompt dependencies")]
    CycleDetected,
}

impl From<QueryError> for AstError {
    fn from(_value: QueryError) -> Self {
        Self::Unknown
    }
}

impl From<SortError<String>> for AstError {
    fn from(value: SortError<String>) -> Self {
        match value {
            SortError::CycleDetected(_) => Self::CycleDetected,
        }
    }
}

fn named_children(node: Node<'_>) -> std::vec::IntoIter<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .collect::<Vec<_>>()
        .into_iter()
}

#[cfg(test)]
mod test {
    use super::{Dependency, PromptType};
    use crate::{AchitekAst, from_str};
    use indoc::indoc;
    use tree_sitter::Language;

    fn ast_from_str(source: &str) -> AchitekAst<'_> {
        let tree = from_str(source).expect("Expected to create parse tree");
        let language: Language = tree_sitter_achitekfile::LANGUAGE.into();

        AchitekAst::new(tree, language, source)
    }

    #[test]
    fn test_query_all_prompts() {
        let source = indoc! {r#"
            blueprint {
              version = "1.0.0"
              name = "web-app"
            }

            prompt "is_workspace" {
              type = bool
              help = "Is this a rust workspace"
            }

            prompt "is_workspace_optimized" {
              type = bool
              help = "Is this a rust optimized workspace"
              depends_on = is_workspace
            }
        "#};

        let ast = ast_from_str(source);

        let prompts = ast.fetch_prompts().expect("Expected to fetch prompts");

        assert_eq!(prompts.len(), 2);
        assert_eq!(prompts[0].name, "is_workspace");
        assert_eq!(prompts[0].prompt_type, PromptType::Bool);
        assert_eq!(prompts[0].help.as_deref(), Some("Is this a rust workspace"));
        assert_eq!(prompts[0].depends_on, None);

        assert_eq!(prompts[1].name, "is_workspace_optimized");
        assert_eq!(prompts[1].prompt_type, PromptType::Bool);
        assert_eq!(
            prompts[1].help.as_deref(),
            Some("Is this a rust optimized workspace")
        );
        assert_eq!(
            prompts[1].depends_on,
            Some(Dependency::Reference("is_workspace".to_owned()))
        );
    }

    #[test]
    fn test_ordered_prompts_sorts_dependencies_first() {
        let source = indoc! {r#"
            blueprint {
              version = "1.0.0"
              name = "web-app"
            }

            prompt "app_name" {
              type = string
              depends_on = is_workspace_optimized
            }

            prompt "is_workspace_optimized" {
              type = bool
              depends_on = is_workspace
            }

            prompt "is_workspace" {
              type = bool
            }
        "#};

        let ast = ast_from_str(source);
        let prompts = ast
            .ordered_prompts()
            .expect("Expected to order prompts by dependency");
        let prompt_names = prompts
            .iter()
            .map(|prompt| prompt.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            prompt_names,
            vec!["is_workspace", "is_workspace_optimized", "app_name"]
        );
    }
}
