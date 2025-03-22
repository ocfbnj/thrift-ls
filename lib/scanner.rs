use std::path::{Path, PathBuf};

use crate::token::{self, TokenKind};

pub struct Scanner {
    input: Vec<char>,    // input data
    path: PathBuf,       // input data path
    state: ScannerState, // current state
}

#[derive(Debug)]
pub enum ScanError {
    InvalidChar(char),
    InvalidString(String),
    UnclosedString(String),
    UnclosedBlockComment(String),
}

#[derive(Clone, Copy)]
pub struct ScannerState {
    offset: usize, // next reading offset
    line: usize,   // current line offset
    column: usize, // current column offset
}

pub trait Input {
    fn data(&self) -> Vec<char>;
    fn path(&self) -> PathBuf;
}

pub struct FileInput {
    path: PathBuf,
}

impl FileInput {
    pub fn new(file_name: &Path) -> Self {
        FileInput {
            path: file_name.to_path_buf(),
        }
    }
}

impl Input for FileInput {
    fn data(&self) -> Vec<char> {
        std::fs::read_to_string(&self.path)
            .unwrap()
            .chars()
            .collect()
    }

    fn path(&self) -> PathBuf {
        self.path.clone()
    }
}

impl Scanner {
    // create a new scanner with the given input data.
    pub fn new(input: impl Input) -> Self {
        Scanner {
            input: input.data(),
            path: input.path(),
            state: ScannerState {
                offset: 0,
                line: 1,
                column: 1,
            },
        }
    }

    // scan the next token and return it.
    pub fn scan(&mut self) -> (token::Token, Option<ScanError>) {
        let mut token = None;
        let mut err = None;

        while self.state.offset < self.input.len() && token.is_none() {
            let ch = self.input[self.state.offset];

            match ch {
                '\n' => {
                    self.state.offset += 1;
                    self.state.column = 1;
                    self.state.line += 1;
                }
                ' ' | '\r' => {
                    self.state.offset += 1;
                    self.state.column += 1;
                }
                '/' => {
                    if self.state.offset + 1 >= self.input.len() {
                        let location = token::Location {
                            line: self.state.line,
                            column: self.state.column,
                            path: self.path.clone(),
                        };
                        token = Some(token::Token {
                            kind: token::TokenKind::Invalid(ch),
                            location,
                        });
                        self.state.offset += 1;
                        self.state.column += 1;
                        break;
                    }

                    let start = self.state.offset;
                    let (offset, ok) = self.scan_line_comment();
                    if ok {
                        let location = token::Location {
                            line: self.state.line,
                            column: self.state.column,
                            path: self.path.clone(),
                        };
                        token = Some(token::Token {
                            kind: token::TokenKind::Comment(
                                self.input[start + 2..start + offset]
                                    .iter()
                                    .collect::<String>(),
                            ),
                            location,
                        });
                        self.state.offset += offset;
                        self.state.column = 1;
                        self.state.line += 1;
                        break;
                    }

                    let (offset, line_offset, column_offset, ok) = self.scan_block_comment();
                    let location = token::Location {
                        line: self.state.line,
                        column: self.state.column,
                        path: self.path.clone(),
                    };
                    if ok {
                        token = Some(token::Token {
                            kind: token::TokenKind::BlockComment(
                                self.input[start + 2..start + offset - 2]
                                    .iter()
                                    .collect::<String>(),
                            ),
                            location,
                        })
                    } else {
                        let value = self.input[start..start + offset].iter().collect::<String>();
                        err = Some(ScanError::UnclosedBlockComment(value.clone()));
                        token = Some(token::Token {
                            kind: token::TokenKind::InvalidString(value),
                            location,
                        });
                    }

                    if line_offset > 0 {
                        debug_assert!(column_offset > 0);
                        self.state.column = 0;
                    }
                    self.state.offset += offset;
                    self.state.column += column_offset;
                    self.state.line += line_offset;
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    let start = self.state.offset;
                    let offset = self.scan_identifier();
                    let value = self.input[start..start + offset].iter().collect::<String>();
                    let location = token::Location {
                        line: self.state.line,
                        column: self.state.column,
                        path: self.path.clone(),
                    };

                    if let Some(tok) = TokenKind::from_string(&value) {
                        token = Some(token::Token {
                            kind: tok,
                            location,
                        });
                    } else {
                        token = Some(token::Token {
                            kind: token::TokenKind::Identifier(value),
                            location,
                        });
                    }

                    self.state.offset += offset;
                    self.state.column += offset;
                }
                '\'' | '"' => {
                    let start = self.state.offset;
                    let (offset, line_offset, ok) = self.scan_literal(ch);
                    let value = self.input[start + 1..start + offset - 1]
                        .iter()
                        .collect::<String>();
                    let location = token::Location {
                        line: self.state.line,
                        column: self.state.column,
                        path: self.path.clone(),
                    };

                    if ok {
                        token = Some(token::Token {
                            kind: token::TokenKind::Literal(value),
                            location,
                        });
                    } else {
                        err = Some(ScanError::UnclosedString(value.clone()));
                        token = Some(token::Token {
                            kind: token::TokenKind::InvalidString(value),
                            location,
                        });
                    }

                    self.state.offset += offset;
                    self.state.column += offset;
                    self.state.line += line_offset;
                }
                '+' | '-' | '0'..='9' => {
                    let start = self.state.offset;
                    let mut offset: usize;
                    let mut int_ok: bool;
                    let mut double_ok = false;

                    (offset, int_ok) = self.scan_int_constant();
                    if !int_ok {
                        (offset, double_ok) = self.scan_double_constant();
                    } else {
                        if self.state.offset + offset < self.input.len() {
                            let next_ch = self.input[self.state.offset + offset];
                            if next_ch == '.' || next_ch == 'e' || next_ch == 'E' {
                                (offset, double_ok) = self.scan_double_constant();
                                if double_ok {
                                    int_ok = false;
                                }
                            }
                        }
                    }

                    let value = self.input[start..start + offset].iter().collect::<String>();
                    let location = token::Location {
                        line: self.state.line,
                        column: self.state.column,
                        path: self.path.clone(),
                    };

                    if int_ok {
                        token = Some(token::Token {
                            kind: token::TokenKind::IntConstant(value),
                            location,
                        });
                    } else if double_ok {
                        token = Some(token::Token {
                            kind: token::TokenKind::DoubleConstant(value),
                            location,
                        });
                    } else {
                        token = Some(token::Token {
                            kind: token::TokenKind::InvalidString(value),
                            location,
                        })
                    }

                    self.state.offset += offset;
                    self.state.column += offset;
                }
                '.' => {
                    let start = self.state.offset;
                    let (offset, double_ok) = self.scan_double_constant();
                    let value = self.input[start..start + offset].iter().collect::<String>();
                    let location = token::Location {
                        line: self.state.line,
                        column: self.state.column,
                        path: self.path.clone(),
                    };

                    if !double_ok {
                        token = Some(token::Token {
                            kind: token::TokenKind::InvalidString(value),
                            location,
                        })
                    } else {
                        token = Some(token::Token {
                            kind: token::TokenKind::DoubleConstant(value),
                            location,
                        });
                    }

                    self.state.offset += offset;
                    self.state.column += offset;
                }
                _ => {
                    let location = token::Location {
                        line: self.state.line,
                        column: self.state.column,
                        path: self.path.clone(),
                    };

                    if let Some(tok) = TokenKind::from_char(ch) {
                        token = Some(token::Token {
                            kind: tok,
                            location,
                        });
                    } else {
                        token = Some(token::Token {
                            kind: token::TokenKind::Invalid(ch),
                            location,
                        })
                    }

                    self.state.offset += 1;
                    self.state.column += 1;
                }
            }
        }

        (token.unwrap_or(self.eof()), err)
    }
}

impl Scanner {
    pub fn save_state(&self) -> ScannerState {
        self.state
    }

    pub fn restore_state(&mut self, state: ScannerState) {
        self.state = state;
    }
}

impl Scanner {
    fn eof(&self) -> token::Token {
        token::Token {
            kind: token::TokenKind::Eof,
            location: token::Location {
                line: self.state.line,
                column: self.state.column,
                path: self.path.clone(),
            },
        }
    }

    // scan the next identifier and return the end offset.
    fn scan_identifier(&mut self) -> usize {
        let mut offset = 1;
        while self.state.offset + offset < self.input.len() {
            let ch = self.input[self.state.offset + offset];

            match ch {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '.' => offset += 1,
                _ => break,
            }
        }

        offset
    }

    // scan the next literal and return the end offset and line offset.
    fn scan_literal(&mut self, delimiter: char) -> (usize, usize, bool) {
        let mut offset = 1;
        let mut line_offset = 0;

        while self.state.offset + offset < self.input.len() {
            let ch = self.input[self.state.offset + offset];
            offset += 1;

            if ch == delimiter {
                return (offset, line_offset, true);
            }
            if ch == '\n' {
                line_offset += 1;
            }
        }

        (offset, line_offset, false)
    }

    // scan the next integer constant and return the end offset.
    fn scan_int_constant(&mut self) -> (usize, bool) {
        match self.input[self.state.offset] {
            '0'..='9' | '+' | '-' => (),
            _ => return (0, false),
        }

        let mut offset = 0;
        while self.state.offset + offset < self.input.len() {
            let ch = self.input[self.state.offset + offset];

            // only allow + or - at the beginning
            if offset > 0 && (ch == '+' || ch == '-') {
                break;
            }

            match ch {
                '0'..='9' | '+' | '-' => offset += 1,
                _ => break,
            }
        }

        if offset > 1 {
            (offset, true)
        } else {
            let ch = self.input[self.state.offset];
            (offset, ch != '+' && ch != '-')
        }
    }

    // scan the next double constant and return the end offset.
    fn scan_double_constant(&mut self) -> (usize, bool) {
        match self.input[self.state.offset] {
            '0'..='9' | '+' | '-' | '.' | 'e' | 'E' => (),
            _ => return (0, false),
        }

        enum State {
            ParsePlusMinus,
            ParseFirstDigits,
            ParseDot,
            ParseSecondDigits,
            ParseE,
            PraseIntConstant,
        }

        let mut state = State::ParsePlusMinus;
        let mut offset = 0;

        while self.state.offset + offset < self.input.len() {
            let ch = self.input[self.state.offset + offset];

            match state {
                State::ParsePlusMinus => {
                    if ch == '+' || ch == '-' {
                        offset += 1;
                    }
                    state = State::ParseFirstDigits;
                }
                State::ParseFirstDigits => match ch {
                    '0'..='9' => {
                        offset += 1;
                    }
                    _ => {
                        state = State::ParseDot;
                    }
                },
                State::ParseDot => {
                    if ch == '.' {
                        offset += 1;
                    }
                    state = State::ParseSecondDigits;
                }
                State::ParseSecondDigits => match ch {
                    '0'..='9' => {
                        offset += 1;
                    }
                    _ => {
                        state = State::ParseE;
                    }
                },
                State::ParseE => {
                    if ch == 'e' || ch == 'E' {
                        offset += 1;
                    }
                    state = State::PraseIntConstant;
                }
                State::PraseIntConstant => {
                    let cur_state = self.save_state();
                    self.state.offset += offset;
                    let (int_offset, ok) = self.scan_int_constant();
                    self.restore_state(cur_state);

                    if ok {
                        offset += int_offset;
                    }
                    break;
                }
            }
        }

        let mut has_digit = false;
        for i in 0..offset {
            let ch = self.input[self.state.offset + i];
            if ch >= '0' && ch <= '9' {
                has_digit = true;
                break;
            }
        }

        (offset, has_digit)
    }

    // scan the next line comment and return the end offset.
    fn scan_line_comment(&mut self) -> (usize, bool) {
        let mut offset = 1;
        if self.state.offset + offset >= self.input.len()
            || self.input[self.state.offset + offset] != '/'
        {
            return (offset, false);
        }

        offset += 1;
        while self.state.offset + offset < self.input.len() {
            let ch = self.input[self.state.offset + offset];
            offset += 1;
            if ch == '\n' {
                break;
            }
        }

        (offset, true)
    }

    // scan the next block comment and return the end offset.
    fn scan_block_comment(&mut self) -> (usize, usize, usize, bool) {
        let mut offset = 1;
        let mut line_offset = 0;
        let mut column_offset = 1;
        if self.state.offset + offset >= self.input.len()
            || self.input[self.state.offset + offset] != '*'
        {
            return (offset, line_offset, column_offset, false);
        }
        offset += 1;
        column_offset += 1;

        while self.state.offset + offset < self.input.len() {
            let ch = self.input[self.state.offset + offset];
            offset += 1;
            column_offset += 1;

            if ch == '\n' {
                line_offset += 1;
                column_offset = 1;
            }

            if self.state.offset + offset >= self.input.len() {
                return (offset, line_offset, column_offset, false);
            }

            // sanc delimiter
            let next_ch = self.input[self.state.offset + offset];
            if ch == '*' && next_ch == '/' {
                offset += 1;
                column_offset += 1;
                return (offset, line_offset, column_offset, true);
            }

            // scan nested block comments
            if ch == '/' && next_ch == '*' {
                let state = self.save_state();
                self.state.offset += offset - 1;
                let (nested_offset, nested_line_offset, nested_column_offset, ok) =
                    self.scan_block_comment();
                self.restore_state(state);
                offset += nested_offset - 1;
                line_offset += nested_line_offset;
                column_offset += nested_column_offset;
                if !ok {
                    return (offset, line_offset, column_offset, false);
                }
            }
        }

        (offset, line_offset, column_offset, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan() {
        let work_path = std::env::current_dir().unwrap();
        let file_path = work_path.join(std::path::Path::new("./lib/test_file/user.thrift"));
        let mut scanner = Scanner::new(FileInput::new(&file_path));

        loop {
            let (token, err) = scanner.scan();
            println!("{:?}", token);
            if token.is_eof() {
                break;
            }

            if token.is_invalid() {
                println!("invalid token: {:?}, err: {:?}", token, err)
            }
        }
    }
}
