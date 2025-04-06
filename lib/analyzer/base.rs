use std::{fmt, path::PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
}

impl Default for Location {
    fn default() -> Self {
        Location {
            path: PathBuf::default(),
            line: 1,
            column: 1,
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.path.display(), self.line, self.column)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Range {
    pub start: Location,
    pub end: Location,
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start.path == self.end.path && self.start.line == self.end.line {
            write!(
                f,
                "{}:{}:{}-{}",
                self.start.path.display(),
                self.start.line,
                self.start.column,
                self.end.column
            )
        } else {
            write!(
                f,
                "{}:{}:{}-{}:{}:{}",
                self.start.path.display(),
                self.start.line,
                self.start.column,
                self.end.path.display(),
                self.end.line,
                self.end.column
            )
        }
    }
}

#[derive(Debug, Clone)]
pub struct Error {
    pub range: Range,
    pub message: String,
}
