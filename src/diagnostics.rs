//! **`achitekfile`** violations represented as structured diagnostics.
//!
//! Diagnostics describe violations found while parsing or analyzing
//! Achitekfile source. They are intended for user-facing tooling such as
//! language servers, command-line validators, formatters, and documentation
//! generators.
//!
//! A diagnostic is different from a fatal Rust error. Invalid Achitekfile source
//! is normal input for editor and validation workflows, so callers should be
//! able to receive a partial analysis result plus every diagnostic that could
//! be discovered.
//!
//! Diagnostic codes are stable identifiers for classes of violations. Message
//! text and help text may improve over time, but released codes should not be
//! reused for different meanings.
//!
//! # Codes
//!
//! Diagnostic codes are distinguished by range:
//!
//! - `ACH0000`-`ACH0999`: syntax and parse diagnostics
//! - `ACH1000`-`ACH1999`: single-file semantic diagnostics
//! - `ACH2000`-`ACH2999`: dependency graph diagnostics
//! - `ACH3000`-`ACH3999`: validation rule diagnostics
//!
//! | violation | kind | severity | code |
//! | --- | --- | --- | --- |
//! | Missing blueprint block | [syntax] | [error] | [ACH0000] |
//! | Multiple blueprint blocks | [syntax] | [error] | [ACH0001] |
//! | Prompt before blueprint | [syntax] | [error] | [ACH0002] |
//! | Unknown top-level item | [syntax] | [error] | [ACH0003] |
//! | Unknown blueprint attribute | [syntax] | [error] | [ACH0004] |
//! | Unknown prompt attribute | [syntax] | [error] | [ACH0005] |
//! | Unknown validate attribute | [syntax] | [error] | [ACH0006] |
//! | Unknown prompt type | [syntax] | [error] | [ACH0007] |
//! | Invalid boolean literal | [syntax] | [error] | [ACH0008] |
//! | Unterminated string | [syntax] | [error] | [ACH0009] |
//! | Invalid escape sequence | [syntax] | [error] | [ACH0010] |
//! | Invalid dependency expression | [syntax] | [error] | [ACH0011] |
//! | Unknown dependency method | [syntax] | [error] | [ACH0012] |
//! | Invalid identifier | [syntax] | [error] | [ACH0013] |
//! | Invalid integer | [syntax] | [error] | [ACH0014] |
//! | Malformed array | [syntax] | [error] | [ACH0015] |
//!
//! ## Code stability
//!
//! Diagnostic codes are part of this crate's public API.
//!
//! - Released codes keep their meaning across compatible releases.
//! - Do not reuse a removed code for a different diagnostic.
//! - Prefer adding a new code when a diagnostic splits into multiple cases.
//! - Message and help text may change over time.
//! - Tests and downstream tools should rely on codes, not exact prose.
//! - Code severity should remain stable unless changing it is intentional and
//!   documented in release notes.
//!
//! [syntax]: DiagnosticKind::Syntax
//! [semantic]: DiagnosticKind::Semantic
//! [dependency]: DiagnosticKind::Dependency
//! [validation]: DiagnosticKind::Validation
//!
//! [error]: Severity::Error
//! [warning]: Severity::Warning
//! [hint]: Severity::Hint
//!
//! [ACH0000]: DiagnosticCode::MissingBlueprintBlock
//! [ACH0001]: DiagnosticCode::MultipleBlueprintBlocks
//! [ACH0002]: DiagnosticCode::PromptBeforeBlueprint
//! [ACH0003]: DiagnosticCode::UnknownTopLevelItem
//! [ACH0004]: DiagnosticCode::UnknownBlueprintAttribute
//! [ACH0005]: DiagnosticCode::UnknownPromptAttribute
//! [ACH0006]: DiagnosticCode::UnknownValidateAttribute
//! [ACH0007]: DiagnosticCode::UnknownPromptType
//! [ACH0008]: DiagnosticCode::InvalidBooleanLiteral
//! [ACH0009]: DiagnosticCode::UnterminatedString
//! [ACH0010]: DiagnosticCode::InvalidEscapeSequence
//! [ACH0011]: DiagnosticCode::InvalidDependencyExpression
//! [ACH0012]: DiagnosticCode::UnknownDependencyMethod
//! [ACH0013]: DiagnosticCode::InvalidIdentifier
//! [ACH0014]: DiagnosticCode::InvalidInteger
//! [ACH0015]: DiagnosticCode::MalformedArray

/// A user-facing issue found in Achitekfile source.
///
/// Diagnostics carry stable machine-readable metadata that downstream tools can
/// map into their own reporting formats. For example, `achitek-ls` can convert
/// this type into an LSP diagnostic without defining its own Achitekfile
/// diagnostic codes.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Stable identifier for this class of diagnostic.
    code: DiagnosticCode,
    // /// Broad category that produced the diagnostic.
    // kind: DiagnosticKind,
    /// How strongly tooling should surface the diagnostic.
    severity: Severity,
    /// Informational message about the diagnostic.
    message: String,
    /// Help message to assist in remediating the diagnostic.
    help: Option<String>,
    /// The source span where something appears in the achitekfile
    range: TextRange,
}
impl Diagnostic {
    /// Creates a diagnostic from a code and source range.
    pub(crate) fn new(code: DiagnosticCode, range: TextRange) -> Self {
        Self {
            code,
            severity: code.severity(),
            message: code.message().to_owned(),
            help: code.help().map(str::to_owned),
            range,
        }
    }

    /// Creates a diagnostic with custom message text from a code and source
    /// range.
    pub(crate) fn with_message(
        code: DiagnosticCode,
        range: TextRange,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: code.severity(),
            message: message.into(),
            help: code.help().map(str::to_owned),
            range,
        }
    }

    /// Getter for code field
    pub fn code(&self) -> DiagnosticCode {
        self.code
    }
    /// Getter for severity field
    pub fn severity(&self) -> Severity {
        self.severity
    }
    /// Getter for message field
    pub fn message(&self) -> &str {
        &self.message
    }
    /// Getter for help field
    pub fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }
    /// Getter for range field
    pub fn range(&self) -> TextRange {
        self.range
    }
    /// Getter for diagnostic code kind
    pub fn kind(&self) -> DiagnosticKind {
        self.code.kind()
    }
}

/// Broad category for an Achitekfile diagnostic.
///
/// The kind describes which analysis layer produced a diagnostic. It is useful
/// for grouping diagnostics in docs and tests, while [`DiagnosticCode`] remains
/// the stable identifier for a specific violation.
#[derive(Debug, Clone)]
pub enum DiagnosticKind {
    /// A syntax or parse violation in the source text.
    Syntax,
    /// A semantic violation in syntactically valid Achitekfile source.
    Semantic,
    /// A dependency graph violation between prompt declarations.
    Dependency,
    /// A validation rule violation on a prompt declaration.
    Validation,
}

/// Stable identifiers for Achitekfile diagnostics.
///
/// Codes are part of the public diagnostic contract for downstream tools. Once
/// released, a code should keep the same meaning. Prefer adding a new code over
/// reusing or renumbering an existing one.
#[derive(Debug, Clone, Copy)]
pub enum DiagnosticCode {
    /// `ACH0000`: the file does not contain the required `blueprint` block.
    MissingBlueprintBlock,
    /// `ACH0001`: the file contains more than one `blueprint` block.
    MultipleBlueprintBlocks,
    /// `ACH0002`: a `prompt` block appears before the required `blueprint` block.
    PromptBeforeBlueprint,
    /// `ACH0003`: an unsupported item appears at the top level of the file.
    UnknownTopLevelItem,
    /// `ACH0004`: a `blueprint` block contains an unsupported attribute.
    UnknownBlueprintAttribute,
    /// `ACH0005`: a `prompt` block contains an unsupported attribute.
    UnknownPromptAttribute,
    /// `ACH0006`: a `validate` block contains an unsupported attribute.
    UnknownValidateAttribute,
    /// `ACH0007`: a `type` attribute uses an unsupported prompt type.
    UnknownPromptType,
    /// `ACH0008`: a boolean value is not `true` or `false`.
    InvalidBooleanLiteral,
    /// `ACH0009`: a string literal is missing its closing quote.
    UnterminatedString,
    /// `ACH0010`: a string literal contains an unsupported escape sequence.
    InvalidEscapeSequence,
    /// `ACH0011`: a `depends_on` attribute contains an invalid dependency expression.
    InvalidDependencyExpression,
    /// `ACH0012`: a dependency method call uses an unsupported method name.
    UnknownDependencyMethod,
    /// `ACH0013`: an identifier does not match Achitekfile identifier syntax.
    InvalidIdentifier,
    /// `ACH0014`: an integer literal does not match Achitekfile integer syntax.
    InvalidInteger,
    /// `ACH0015`: an array literal is malformed.
    MalformedArray,
}
impl DiagnosticCode {
    /// Returns the broad diagnostic category for this code.
    pub fn kind(&self) -> DiagnosticKind {
        match self {
            Self::MissingBlueprintBlock => DiagnosticKind::Syntax,
            Self::MultipleBlueprintBlocks => DiagnosticKind::Syntax,
            Self::PromptBeforeBlueprint => DiagnosticKind::Syntax,
            Self::UnknownTopLevelItem => DiagnosticKind::Syntax,
            Self::UnknownBlueprintAttribute => DiagnosticKind::Syntax,
            Self::UnknownPromptAttribute => DiagnosticKind::Syntax,
            Self::UnknownValidateAttribute => DiagnosticKind::Syntax,
            Self::UnknownPromptType => DiagnosticKind::Syntax,
            Self::InvalidBooleanLiteral => DiagnosticKind::Syntax,
            Self::UnterminatedString => DiagnosticKind::Syntax,
            Self::InvalidEscapeSequence => DiagnosticKind::Syntax,
            Self::InvalidDependencyExpression => DiagnosticKind::Syntax,
            Self::UnknownDependencyMethod => DiagnosticKind::Syntax,
            Self::InvalidIdentifier => DiagnosticKind::Syntax,
            Self::InvalidInteger => DiagnosticKind::Syntax,
            Self::MalformedArray => DiagnosticKind::Syntax,
        }
    }
    /// Returns the severity of the diagnostic code
    pub fn severity(&self) -> Severity {
        match self {
            Self::MissingBlueprintBlock => Severity::Error,
            Self::MultipleBlueprintBlocks => Severity::Error,
            Self::PromptBeforeBlueprint => Severity::Error,
            Self::UnknownTopLevelItem => Severity::Error,
            Self::UnknownBlueprintAttribute => Severity::Error,
            Self::UnknownPromptAttribute => Severity::Error,
            Self::UnknownValidateAttribute => Severity::Error,
            Self::UnknownPromptType => Severity::Error,
            Self::InvalidBooleanLiteral => Severity::Error,
            Self::UnterminatedString => Severity::Error,
            Self::InvalidEscapeSequence => Severity::Error,
            Self::InvalidDependencyExpression => Severity::Error,
            Self::UnknownDependencyMethod => Severity::Error,
            Self::InvalidIdentifier => Severity::Error,
            Self::InvalidInteger => Severity::Error,
            Self::MalformedArray => Severity::Error,
        }
    }
    /// Returns the stable machine-readable code
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MissingBlueprintBlock => "ACH0000",
            Self::MultipleBlueprintBlocks => "ACH0001",
            Self::PromptBeforeBlueprint => "ACH0002",
            Self::UnknownTopLevelItem => "ACH0003",
            Self::UnknownBlueprintAttribute => "ACH0004",
            Self::UnknownPromptAttribute => "ACH0005",
            Self::UnknownValidateAttribute => "ACH0006",
            Self::UnknownPromptType => "ACH0007",
            Self::InvalidBooleanLiteral => "ACH0008",
            Self::UnterminatedString => "ACH0009",
            Self::InvalidEscapeSequence => "ACH0010",
            Self::InvalidDependencyExpression => "ACH0011",
            Self::UnknownDependencyMethod => "ACH0012",
            Self::InvalidIdentifier => "ACH0013",
            Self::InvalidInteger => "ACH0014",
            Self::MalformedArray => "ACH0015",
        }
    }

    /// Returns the default message for this diagnostic code.
    pub fn message(&self) -> &'static str {
        match self {
            Self::MissingBlueprintBlock => "missing blueprint block",
            Self::MultipleBlueprintBlocks => "multiple blueprint blocks",
            Self::PromptBeforeBlueprint => "prompt block appears before blueprint block",
            Self::UnknownTopLevelItem => "unknown top-level item",
            Self::UnknownBlueprintAttribute => "unknown blueprint attribute",
            Self::UnknownPromptAttribute => "unknown prompt attribute",
            Self::UnknownValidateAttribute => "unknown validate attribute",
            Self::UnknownPromptType => "unknown prompt type",
            Self::InvalidBooleanLiteral => "invalid boolean literal",
            Self::UnterminatedString => "unterminated string literal",
            Self::InvalidEscapeSequence => "invalid escape sequence",
            Self::InvalidDependencyExpression => "invalid dependency expression",
            Self::UnknownDependencyMethod => "unknown dependency method",
            Self::InvalidIdentifier => "invalid identifier",
            Self::InvalidInteger => "invalid integer literal",
            Self::MalformedArray => "malformed array literal",
        }
    }

    /// Returns default help text for this diagnostic code.
    pub fn help(&self) -> Option<&'static str> {
        match self {
            Self::MissingBlueprintBlock => Some("Start the file with a `blueprint { ... }` block."),
            Self::MultipleBlueprintBlocks => {
                Some("Keep exactly one `blueprint` block in each Achitekfile.")
            }
            Self::PromptBeforeBlueprint => {
                Some("Move the `blueprint` block before all `prompt` blocks.")
            }
            Self::UnknownTopLevelItem => {
                Some("Only `blueprint` and `prompt` blocks are valid at the top level.")
            }
            Self::UnknownBlueprintAttribute => Some(
                "Use one of `version`, `name`, `description`, `author`, or `min_achitek_version`.",
            ),
            Self::UnknownPromptAttribute => Some(
                "Use one of `type`, `help`, `choices`, `default`, `required`, `depends_on`, or `validate`.",
            ),
            Self::UnknownValidateAttribute => Some(
                "Use one of `regex`, `min_length`, `max_length`, `min_selections`, or `max_selections`.",
            ),
            Self::UnknownPromptType => {
                Some("Use one of `string`, `paragraph`, `bool`, `select`, or `multiselect`.")
            }
            Self::InvalidBooleanLiteral => Some("Use `true` or `false`."),
            Self::UnterminatedString => Some("Close the string with `\"`."),
            Self::InvalidEscapeSequence => {
                Some("Supported escapes are `\\n`, `\\t`, `\\r`, `\\\"`, and `\\\\`.")
            }
            Self::InvalidDependencyExpression => Some(
                "Use a prompt reference, comparison, `contains(...)`, `all(...)`, or `any(...)`.",
            ),
            Self::UnknownDependencyMethod => Some("The only supported method is `contains`."),
            Self::InvalidIdentifier => Some(
                "Identifiers must start with a letter and contain only letters, digits, or `_`.",
            ),
            Self::InvalidInteger => Some("Use a non-negative integer such as `1` or `42`."),
            Self::MalformedArray => Some("Use `[value, value]` with comma-separated values."),
        }
    }
}

/// Severity level for an Achitekfile diagnostic.
///
/// Severity indicates how tools should present a diagnostic. Errors describe
/// invalid source that should prevent normal execution. Warnings describe
/// suspicious but still analyzable source. Hints provide low-priority guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Invalid source that should prevent normal execution.
    Error,
    /// Suspicious source that can still be analyzed.
    Warning,
    /// Low-priority guidance.
    Hint,
}

/// A zero-based byte position in Achitekfile source text.
///
/// `line` and `byte` use Tree-sitter's native coordinate system: the line is
/// zero-based and `byte` is the zero-based UTF-8 byte offset from the beginning
/// of that line.
///
/// This type is independent of LSP positions. Language-server consumers should
/// convert `byte` into the negotiated LSP position encoding before publishing
/// diagnostics or other ranges to an editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextPosition {
    /// Zero-based line number.
    pub line: usize,

    /// Zero-based UTF-8 byte offset within the line.
    pub byte: usize,
}

/// A byte range in Achitekfile source text.
///
/// The range starts at `start` and ends at `end`, both expressed as zero-based
/// line plus UTF-8 byte offset positions. Consumers can use this to highlight
/// diagnostics, symbols, prompt names, attributes, and other source elements
/// after converting into their presentation protocol's expected position
/// encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextRange {
    /// Start position of the range.
    pub start: TextPosition,

    /// End position of the range.
    pub end: TextPosition,
}
