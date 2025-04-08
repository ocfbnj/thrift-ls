/// Parses a header.
#[macro_export]
macro_rules! parse_header {
    ($self:ident, $headers:ident, $($kind:ident => $parse_fn:ident),* $(,)?) => {
        match $self.peek_next_token().kind {
            $(
                TokenKind::$kind => {
                    if let Some(node) = $self.$parse_fn() {
                        $headers.push(Box::new(node));
                    } else {
                        $self.recover_to_next_line();
                    }
                }
            )*
            _ => break,
        }
    };
}

/// Parses a definition.
#[macro_export]
macro_rules! parse_definition {
    ($self:ident, $definitions:ident, $($kind:ident => $parse_fn:ident),* $(,)?) => {
        let next_token = $self.peek_next_token();

        match next_token.kind {
            $(
                TokenKind::$kind => {
                    if let Some(node) = $self.$parse_fn() {
                        $definitions.push(Rc::new(node));
                    } else {
                        $self.recover_to_next_definition();
                    }
                }
            )*
            TokenKind::Eof => break,
            _ => {
                $self.add_error(
                    format!("Unexpected token: {}", next_token.kind),
                    next_token.range(),
                );
                $self.eat_next_token();
                $self.recover_to_next_definition();
            }
        }
    };
}

/// Extracts the value of a token.
#[macro_export]
macro_rules! extract_token_value {
    ($self:expr, $token:expr, $value_type:ident, $kind:expr) => {
        if let TokenKind::$value_type(value) = $token.kind {
            value
        } else {
            $self.add_error(
                format!("Expected {}, but got {}", $kind, $token.kind),
                $token.range(),
            );
            return None;
        }
    };
}

/// Expects a token kind.
#[macro_export]
macro_rules! expect_token {
    ($self:expr, $kind:ident, $expected_str:expr) => {
        let token = $self.next_token();
        if token.kind != TokenKind::$kind {
            $self.add_error(
                format!("Expected {}, but got {}", $expected_str, token.kind),
                token.range(),
            );
            return None;
        }
    };
}

/// Expects a token.
#[macro_export]
macro_rules! expect {
    ($self:expr, $expected:expr, $expected_str:expr) => {
        let token = $self.next_token();
        if token.kind != $expected {
            $self.add_error(
                format!("Expected {}, but got {}", $expected_str, token.kind),
                token.range(),
            );
            return None;
        }
    };
}

/// Optional list separator.
#[macro_export]
macro_rules! opt_list_separator {
    ($self:expr) => {
        let token = $self.peek_next_token();
        if token.is_line_separator() {
            $self.eat_next_token();
        }
    };
}

/// Breaks if the next token is the expected kind or EOF.
#[macro_export]
macro_rules! break_opt_token_or_eof {
    ($self:expr, $kind:ident) => {
        let next_token = $self.peek_next_token();
        if next_token.kind == TokenKind::$kind {
            $self.eat_next_token();
            break;
        }
        if next_token.is_eof() {
            $self.add_error("Unexpected end of file".to_string(), next_token.range());
            break;
        }
    };
}

/// Implements the DefinitionNode trait for the given types.
#[macro_export]
macro_rules! impl_definition_node {
    ($($t:ty),*) => {
        $(
            impl DefinitionNode for $t {
                fn name(&self) -> &str {
                    &self.identifier.name
                }

                fn as_node(&self) -> &dyn Node {
                    self
                }

                fn identifier(&self) -> &IdentifierNode {
                    &self.identifier
                }
            }
        )*
    };
}
