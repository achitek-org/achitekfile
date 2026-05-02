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
}

/// Severity level for an Achitekfile diagnostic.
///
/// Severity indicates how tools should present a diagnostic. Errors describe
/// invalid source that should prevent normal execution. Warnings describe
/// suspicious but still analyzable source. Hints provide low-priority guidance.
#[derive(Debug, Clone, Copy)]
pub enum Severity {
    /// Invalid source that should prevent normal execution.
    Error,
    /// Suspicious source that can still be analyzed.
    Warning,
    /// Low-priority guidance.
    Hint,
}

/// A zero-based position in Achitekfile source text.
///
/// `line` and `character` are intended for editor and diagnostic reporting.
/// They are independent of LSP types so this crate can be used by non-editor
/// consumers such as CLI tooling.
#[derive(Debug, Clone, Copy)]
pub struct TextPosition {
    /// Zero-based line number.
    pub line: usize,

    /// Zero-based character offset within the line.
    pub character: usize,
}

/// A source range in Achitekfile text.
///
/// The range starts at `start` and ends at `end`. Consumers can use this to
/// highlight diagnostics, symbols, prompt names, attributes, and other source
/// elements.
#[derive(Debug, Clone, Copy)]
pub struct TextRange {
    /// Start position of the range.
    pub start: TextPosition,

    /// End position of the range.
    pub end: TextPosition,
}
