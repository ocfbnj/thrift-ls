use crate::analyzer::{
    base::{Error, Position},
    token::{Token, TokenKind},
};

/// Represents a Thrift scanner.
pub struct Scanner<'a> {
    input: &'a [char],   // input data
    state: ScannerState, // current state
}

/// Represents a Thrift scanner state.
#[derive(Clone, Copy)]
pub struct ScannerState {
    offset: usize, // next reading offset
    line: usize,   // current line offset
    column: usize, // current column offset
}

impl Into<Position> for ScannerState {
    fn into(self) -> Position {
        Position {
            line: self.line as u32,
            column: self.column as u32,
        }
    }
}

impl<'a> Scanner<'a> {
    /// Creates a new scanner with the given input data.
    pub fn new(input: &'a [char]) -> Self {
        Scanner {
            input,
            state: ScannerState {
                offset: 0,
                line: 1,
                column: 1,
            },
        }
    }

    /// Scans the next token and returns it.
    pub fn scan(&mut self) -> (Token, Option<Error>) {
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
                '\r' => {
                    self.state.offset += 1;
                    self.state.column = 1;
                    self.state.line += 1;

                    if self.state.offset < self.input.len() && self.input[self.state.offset] == '\n'
                    {
                        self.state.offset += 1;
                    }
                }
                ' ' | '\t' => {
                    self.state.offset += 1;
                    self.state.column += 1;
                }
                '/' => {
                    if self.state.offset + 1 >= self.input.len() {
                        token = Some(Token {
                            kind: TokenKind::Invalid(ch),
                            position: self.state.into(),
                        });
                        self.state.offset += 1;
                        self.state.column += 1;
                        break;
                    }

                    let start = self.state.offset;
                    let (offset, ok) = self.scan_line_comment();
                    if ok {
                        token = Some(Token {
                            kind: TokenKind::Comment(
                                self.input[start + 2..start + offset]
                                    .iter()
                                    .collect::<String>(),
                            ),
                            position: self.state.into(),
                        });
                        self.state.offset += offset;
                        self.state.column = 1;
                        self.state.line += 1;
                        break;
                    }

                    let (offset, line_offset, column_offset, ok) = self.scan_block_comment();
                    let position = self.state.into();
                    if ok {
                        token = Some(Token {
                            kind: TokenKind::BlockComment(
                                self.input[start + 2..start + offset - 2]
                                    .iter()
                                    .collect::<String>(),
                            ),
                            position,
                        })
                    } else {
                        let value = self.input[start..start + offset].iter().collect::<String>();
                        let tk = Token {
                            kind: TokenKind::InvalidString(value.clone()),
                            position,
                        };
                        err = Some(Error {
                            range: tk.range(),
                            message: format!("Unclosed block comment: {}", value),
                        });
                        token = Some(tk);
                    }

                    if line_offset > 0 {
                        debug_assert!(column_offset > 0);
                        self.state.column = 0;
                    }
                    self.state.offset += offset;
                    self.state.column += column_offset;
                    self.state.line += line_offset;
                }
                '#' => {
                    let start = self.state.offset;
                    let offset = self.scan_pound_comment();
                    let value = self.input[start..start + offset].iter().collect::<String>();
                    let position = self.state.into();

                    token = Some(Token {
                        kind: TokenKind::PoundComment(value),
                        position,
                    });

                    self.state.offset += offset;
                    self.state.column = 1;
                    self.state.line += 1;
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    let start = self.state.offset;
                    let offset = self.scan_identifier();
                    let value = self.input[start..start + offset].iter().collect::<String>();
                    let position = self.state.into();

                    if let Some(tok) = TokenKind::from_string(&value) {
                        token = Some(Token {
                            kind: tok,
                            position,
                        });
                    } else {
                        token = Some(Token {
                            kind: TokenKind::Identifier(value),
                            position,
                        });
                    }

                    self.state.offset += offset;
                    self.state.column += offset;
                }
                '\'' | '"' => {
                    let start = self.state.offset;
                    let (offset, line_offset, column_offset, ok) = self.scan_literal(ch);
                    let value = self.input[start + 1..start + offset - 1]
                        .iter()
                        .collect::<String>();
                    let position = self.state.into();

                    if ok {
                        token = Some(Token {
                            kind: TokenKind::Literal(value),
                            position,
                        });
                    } else {
                        let tk = Token {
                            kind: TokenKind::InvalidString(value.clone()),
                            position,
                        };
                        err = Some(Error {
                            range: tk.range(),
                            message: format!("Unclosed string: {}", value),
                        });
                        token = Some(tk);
                    }

                    if line_offset > 0 {
                        debug_assert!(column_offset > 0);
                        self.state.column = 0;
                    }
                    self.state.offset += offset;
                    self.state.column += column_offset;
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
                    let position = self.state.into();

                    if int_ok {
                        token = Some(Token {
                            kind: TokenKind::IntConstant(value),
                            position,
                        });
                    } else if double_ok {
                        token = Some(Token {
                            kind: TokenKind::DoubleConstant(value),
                            position,
                        });
                    } else {
                        token = Some(Token {
                            kind: TokenKind::InvalidString(value),
                            position,
                        })
                    }

                    self.state.offset += offset;
                    self.state.column += offset;
                }
                '.' => {
                    let start = self.state.offset;
                    let (offset, double_ok) = self.scan_double_constant();
                    let value = self.input[start..start + offset].iter().collect::<String>();
                    let position = self.state.into();

                    if !double_ok {
                        token = Some(Token {
                            kind: TokenKind::InvalidString(value),
                            position,
                        })
                    } else {
                        token = Some(Token {
                            kind: TokenKind::DoubleConstant(value),
                            position,
                        });
                    }

                    self.state.offset += offset;
                    self.state.column += offset;
                }
                _ => {
                    let position = self.state.into();

                    if let Some(tok) = TokenKind::from_char(ch) {
                        token = Some(Token {
                            kind: tok,
                            position,
                        });
                    } else {
                        token = Some(Token {
                            kind: TokenKind::Invalid(ch),
                            position,
                        })
                    }

                    self.state.offset += 1;
                    self.state.column += 1;
                }
            }
        }

        (token.unwrap_or(self.eof()), err)
    }

    /// Skips to the next line.
    pub fn skip_to_next_line(&mut self) {
        while self.state.offset < self.input.len() {
            let ch = self.input[self.state.offset] as char;
            self.state.offset += 1;

            if ch == '\n' {
                self.state.line += 1;
                self.state.column = 1;
                break;
            } else if ch == '\r' {
                if self.state.offset < self.input.len()
                    && self.input[self.state.offset] as char == '\n'
                {
                    self.state.offset += 1;
                }
                self.state.line += 1;
                self.state.column = 1;
                break;
            }
        }
    }
}

impl<'a> Scanner<'a> {
    /// Saves the current state.
    pub fn save_state(&self) -> ScannerState {
        self.state
    }

    /// Restores the state.
    pub fn restore_state(&mut self, state: ScannerState) {
        self.state = state;
    }
}

impl<'a> Scanner<'a> {
    fn eof(&self) -> Token {
        Token {
            kind: TokenKind::Eof,
            position: Position {
                line: self.state.line as u32,
                column: self.state.column as u32,
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
    fn scan_literal(&mut self, delimiter: char) -> (usize, usize, usize, bool) {
        let mut offset = 1;
        let mut line_offset = 0;
        let mut column_offset = 1;
        let mut prev_ch = delimiter;

        while self.state.offset + offset < self.input.len() {
            let ch = self.input[self.state.offset + offset];
            offset += 1;
            column_offset += 1;

            if ch == delimiter && prev_ch != '\\' {
                return (offset, line_offset, column_offset, true);
            }
            if ch == '\n' {
                line_offset += 1;
                column_offset = 1;
            } else if ch == '\r' {
                if self.state.offset + offset < self.input.len()
                    && self.input[self.state.offset + offset] as char == '\n'
                {
                    offset += 1;
                }
                line_offset += 1;
                column_offset = 1;
            }

            prev_ch = ch;
        }

        (offset, line_offset, column_offset, false)
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
            } else if ch == '\r' {
                if self.state.offset + offset < self.input.len()
                    && self.input[self.state.offset + offset] as char == '\n'
                {
                    offset += 1;
                }
                line_offset += 1;
                column_offset = 1;
            }

            if self.state.offset + offset >= self.input.len() {
                return (offset, line_offset, column_offset, false);
            }

            // scan delimiter
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

    // scan the next pound comment and return the end offset.
    fn scan_pound_comment(&mut self) -> usize {
        let mut offset = 1;

        while self.state.offset + offset < self.input.len() {
            let ch = self.input[self.state.offset + offset];
            offset += 1;
            if ch == '\n' {
                break;
            } else if ch == '\r' {
                if self.state.offset + offset < self.input.len()
                    && self.input[self.state.offset + offset] as char == '\n'
                {
                    offset += 1;
                }
                break;
            }
        }

        offset
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::Path};

    use super::*;

    #[test]
    fn test_scan() {
        let work_path = env::current_dir().unwrap();
        let file_path = work_path.join(Path::new("./lib/analyzer/test_file/ThriftTest.thrift"));
        let content = fs::read_to_string(&file_path)
            .unwrap()
            .chars()
            .collect::<Vec<_>>();
        let mut scanner = Scanner::new(&content);

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
