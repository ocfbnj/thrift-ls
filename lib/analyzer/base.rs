//! Base types for the analyzer.

/// Represents a location in a document.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Position {
    /// Line number in a document (one-based).
    pub line: u32,
    /// Column offset on a line in a document (one-based).
    pub column: u32,
}

/// Represents a range in a document.
#[derive(Debug, Clone)]
pub struct Range {
    /// Start position of the range.
    pub start: Position,
    /// End position of the range.
    pub end: Position,
}

/// Represents a error in the document.
#[derive(Debug, Clone)]
pub struct Error {
    pub range: Range,
    pub message: String,
}
