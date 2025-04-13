use std::rc::Rc;

use crate::{
    analyzer::{
        ast::{
            BaseTypeNode, ConstNode, ConstValueNode, CppIncludeNode, DefinitionNode, DocumentNode,
            EnumNode, EnumValueNode, ExceptionNode, ExtNode, FieldIdNode, FieldNode, FunctionNode,
            HeaderNode, IdentifierNode, IncludeNode, ListTypeNode, MapTypeNode, NamespaceNode,
            Node, ServiceNode, SetTypeNode, StructNode, TypedefNode, UnionNode,
        },
        base::{Error, Range},
        scanner::Scanner,
        token::{Token, TokenKind},
    },
    break_opt_token_or_eof, expect, expect_token, extract_token_value, opt_list_separator,
    parse_definition, parse_header,
};

/// Parser for a single file.
pub struct Parser<'a> {
    scanner: Scanner<'a>,
    errors: Vec<Error>,
    prev_token: Option<Token>,
}

impl<'a> Parser<'a> {
    /// Create a new parser.
    pub fn new(input: &'a [char]) -> Parser<'a> {
        Parser {
            scanner: Scanner::new(input),
            errors: Vec::new(),
            prev_token: None,
        }
    }

    /// Parse a single file.
    pub fn parse(mut self) -> (DocumentNode, Vec<Error>) {
        let start = self.peek_next_token().range().start;
        let headers = self.parse_headers();
        let definitions = self.parse_definitions();
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        let node = DocumentNode {
            headers,
            definitions,
            range,
        };

        (node, self.errors)
    }
}

impl<'a> Parser<'a> {
    fn next_token(&mut self) -> Token {
        self.skip_comment_tokens();
        let (next_token, err) = self.scanner.scan();
        if let Some(err) = err {
            self.add_error(err.message, err.range);
        }

        self.prev_token = Some(next_token.clone());
        next_token
    }

    fn prev_token(&self) -> Option<Token> {
        self.prev_token.clone()
    }

    fn peek_next_token(&mut self) -> Token {
        self.skip_comment_tokens();

        let state = self.scanner.save_state();
        let (next_token, _) = self.scanner.scan();
        self.scanner.restore_state(state);

        next_token
    }

    fn eat_next_token(&mut self) {
        self.next_token();
    }

    fn skip_comment_tokens(&mut self) {
        loop {
            let state = self.scanner.save_state();
            let (next_token, _) = self.scanner.scan();
            if !next_token.is_comment() {
                self.scanner.restore_state(state);
                break;
            }
        }
    }
}

// parse headers
impl<'a> Parser<'a> {
    fn parse_headers(&mut self) -> Vec<Rc<HeaderNode>> {
        // Headers ::= ( Include | CppInclude | Namespace )*
        let mut headers: Vec<Rc<HeaderNode>> = Vec::new();

        loop {
            parse_header!(
                self,
                headers,
                Include => parse_include,
                CppInclude => parse_cpp_include,
                Namespace => parse_namespace,
            );
        }

        headers
    }

    fn parse_include(&mut self) -> Option<IncludeNode> {
        // Include ::= 'include' Literal

        let start = self.peek_next_token().range().start;
        expect_token!(self, Include, "'include'");
        let token = self.next_token();
        let literal = extract_token_value!(self, token, Literal, "literal");
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(IncludeNode { range, literal })
    }

    fn parse_cpp_include(&mut self) -> Option<CppIncludeNode> {
        // CppInclude ::= 'cpp_include' Literal

        let start = self.peek_next_token().range().start;
        expect_token!(self, CppInclude, "'cpp_include'");
        let token = self.next_token();
        let literal = extract_token_value!(self, token, Literal, "literal");
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(CppIncludeNode { range, literal })
    }

    fn parse_namespace(&mut self) -> Option<NamespaceNode> {
        // Namespace ::= 'namespace' NamespaceScope Identifier Ext?

        let start = self.peek_next_token().range().start;
        expect_token!(self, Namespace, "'namespace'");
        let token = self.next_token();
        let scope = extract_token_value!(self, token, NamespaceScope, "namespace scope");
        let identifier = self.parse_identifier()?;
        let ext = self.opt_parse_ext();
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(NamespaceNode {
            range,
            scope,
            identifier,
            ext,
        })
    }

    fn parse_identifier(&mut self) -> Option<IdentifierNode> {
        // Identifier ::= ( Letter | '_' ) ( Letter | Digit | '.' | '_' )*

        let token = self.next_token();
        let range = token.range();
        if let TokenKind::Identifier(name) = token.kind {
            return Some(IdentifierNode { range, name });
        }

        let name = token.kind.to_string();
        if !TokenKind::from_string(&name).is_some() {
            self.add_error(format!("Invalid identifier: {}", name), token.range());
            return None;
        }

        Some(IdentifierNode { range, name })
    }
}

// parse definitions
impl<'a> Parser<'a> {
    fn parse_definitions(&mut self) -> Vec<Rc<DefinitionNode>> {
        // Definitions ::= ( Const | Typedef | Enum | Struct | Union | Exception | Service )*

        let mut definitions: Vec<Rc<DefinitionNode>> = Vec::new();

        loop {
            parse_definition!(
                self,
                definitions,
                Const => parse_const,
                Typedef => parse_typedef,
                Enum => parse_enum,
                Struct => parse_struct,
                Union => parse_union,
                Exception => parse_exception,
                Service => parse_service,
            );
        }

        definitions
    }

    fn parse_const(&mut self) -> Option<ConstNode> {
        // Const ::= 'const' FieldType Identifier '=' ConstValue ListSeparator?

        let start = self.peek_next_token().range().start;
        expect_token!(self, Const, "'const'");
        let field_type = self.parse_field_type()?;
        let identifier = self.parse_identifier()?;
        expect_token!(self, Assign, "'='");
        let value = Box::new(self.parse_const_value()?);
        opt_list_separator!(self);
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(ConstNode {
            range,
            field_type,
            identifier,
            value,
        })
    }

    fn parse_field_type(&mut self) -> Option<Box<dyn Node>> {
        // FieldType ::= Identifier | DefinitionType

        let next_token = self.peek_next_token();
        match next_token.kind {
            TokenKind::Identifier(ref identifier) => {
                self.eat_next_token();
                return Some(Box::new(IdentifierNode {
                    range: next_token.range(),
                    name: identifier.clone(),
                }));
            }
            _ => {
                return self.parse_definition_type();
            }
        }
    }

    fn parse_definition_type(&mut self) -> Option<Box<dyn Node>> {
        // DefinitionType ::= BaseType | ContainerType

        let next_token = self.peek_next_token();
        match next_token.kind {
            TokenKind::BaseType(ref base_type) => {
                self.eat_next_token();
                return Some(Box::new(BaseTypeNode {
                    range: next_token.range(),
                    name: base_type.clone(),
                }));
            }
            _ => {
                return self.parse_container_type();
            }
        }
    }

    fn parse_container_type(&mut self) -> Option<Box<dyn Node>> {
        // ContainerType ::= MapType | SetType | ListType

        let next_token = self.peek_next_token();
        match next_token.kind {
            TokenKind::Map => self.parse_map_type().map(|x| Box::new(x) as Box<dyn Node>),
            TokenKind::Set => self.parse_set_type().map(|x| Box::new(x) as Box<dyn Node>),
            TokenKind::List => self.parse_list_type().map(|x| Box::new(x) as Box<dyn Node>),
            _ => {
                self.add_error(
                    format!("Expected map, set, or list, but got {}", next_token.kind),
                    next_token.range(),
                );
                None
            }
        }
    }

    fn opt_parse_cpp_type(&mut self) -> Option<String> {
        // CppType ::= 'cpp_type' Identifier

        if self.peek_next_token().kind != TokenKind::CppType {
            return None;
        }
        expect_token!(self, CppType, "'cpp_type'");

        let token = self.next_token();
        Some(extract_token_value!(self, token, Identifier, "identifier"))
    }

    fn parse_map_type(&mut self) -> Option<MapTypeNode> {
        // MapType ::= 'map' CppType? '<' FieldType ',' FieldType '>'

        let start = self.peek_next_token().range().start;
        expect_token!(self, Map, "'map'");
        let cpp_type = self.opt_parse_cpp_type();

        expect_token!(self, Less, "'<'");
        let key_type = self.parse_field_type()?;
        expect!(self, TokenKind::ListSeparator(','), "','");
        let value_type = self.parse_field_type()?;
        expect_token!(self, Greater, "'>'");
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(MapTypeNode {
            range,
            cpp_type,
            key_type,
            value_type,
        })
    }

    fn parse_set_type(&mut self) -> Option<SetTypeNode> {
        // SetType ::= 'set' CppType? '<' FieldType '>'

        let start = self.peek_next_token().range().start;
        expect_token!(self, Set, "'set'");
        let cpp_type = self.opt_parse_cpp_type();

        expect!(self, TokenKind::Less, "'<'");
        let type_node = self.parse_field_type()?;
        expect_token!(self, Greater, "'>'");
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(SetTypeNode {
            range,
            cpp_type,
            type_node,
        })
    }

    fn parse_list_type(&mut self) -> Option<ListTypeNode> {
        // ListType ::= 'list' CppType? '<' FieldType '>'

        let start = self.peek_next_token().range().start;
        expect_token!(self, List, "'list'");
        let cpp_type = self.opt_parse_cpp_type();

        expect!(self, TokenKind::Less, "'<'");
        let type_node = self.parse_field_type()?;
        expect_token!(self, Greater, "'>'");
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(ListTypeNode {
            range,
            cpp_type,
            type_node,
        })
    }

    fn parse_const_value(&mut self) -> Option<ConstValueNode> {
        // ConstValue ::= IntConstant | DoubleConstant | Literal | Identifier | ConstList | ConstMap

        let next_token = self.peek_next_token();
        match &next_token.kind {
            TokenKind::IntConstant(value)
            | TokenKind::DoubleConstant(value)
            | TokenKind::Literal(value)
            | TokenKind::Identifier(value) => {
                self.eat_next_token();
                Some(ConstValueNode {
                    range: next_token.range(),
                    value: value.clone(),
                })
            }
            TokenKind::Lbrack => self.parse_const_list(),
            TokenKind::Lbrace => self.parse_const_map(),
            _ => {
                self.eat_next_token();
                self.add_error(
                    format!("Expected constant value, but got {}", next_token.kind),
                    next_token.range(),
                );
                None
            }
        }
    }

    fn parse_const_list(&mut self) -> Option<ConstValueNode> {
        // ConstList ::= '[' (ConstValue ListSeparator?)* ']'

        let start = self.peek_next_token().range().start;
        expect_token!(self, Lbrack, "'['");
        let mut values = Vec::new();
        loop {
            break_opt_token_or_eof!(self, Rbrack);
            values.push(self.parse_const_value()?.value);
            opt_list_separator!(self);
        }
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(ConstValueNode {
            range,
            value: format!("[{}]", values.join(", ")),
        })
    }

    fn parse_const_map(&mut self) -> Option<ConstValueNode> {
        // ConstMap ::= '{' ConstMapValue* '}'

        let start = self.peek_next_token().range().start;
        expect_token!(self, Lbrace, "'{'");
        let mut pairs = Vec::new();
        loop {
            break_opt_token_or_eof!(self, Rbrace);
            pairs.push(self.parse_const_map_value()?);
        }
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(ConstValueNode {
            range,
            value: format!(
                "{{{}}}",
                pairs
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        })
    }

    fn parse_const_map_value(&mut self) -> Option<(String, String)> {
        // ConstMapValue ::= ConstValue ':' ConstValue ListSeparator?

        let key = self.parse_const_value()?;
        expect_token!(self, Colon, "':'");
        let value = self.parse_const_value()?;
        opt_list_separator!(self);

        Some((key.value, value.value))
    }

    fn parse_typedef(&mut self) -> Option<TypedefNode> {
        // Typedef ::= 'typedef' DefinitionType Identifier

        let start = self.peek_next_token().range().start;
        expect_token!(self, Typedef, "'typedef'");
        let definition_type = self.parse_definition_type()?;
        let identifier = self.parse_identifier()?;
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(TypedefNode {
            range,
            definition_type,
            identifier,
        })
    }

    fn parse_enum(&mut self) -> Option<EnumNode> {
        // Enum ::= 'enum' Identifier '{' EnumValue* '}'

        let start = self.peek_next_token().range().start;
        expect_token!(self, Enum, "'enum'");
        let identifier = self.parse_identifier()?;
        expect_token!(self, Lbrace, "'{'");

        let mut values = Vec::new();
        loop {
            break_opt_token_or_eof!(self, Rbrace);
            if let Some(value) = self.parse_enum_value() {
                values.push(value);
            } else {
                self.recover_to_next_line();
            }
        }
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(EnumNode {
            range,
            identifier,
            values,
        })
    }

    fn parse_enum_value(&mut self) -> Option<EnumValueNode> {
        // EnumValue ::= Identifier ('=' IntConstant)? Ext? ListSeparator?

        let start = self.peek_next_token().range().start;
        let token = self.next_token();
        let name = extract_token_value!(self, token, Identifier, "identifier");

        let mut value = None;
        let next_token = self.peek_next_token();
        if next_token.kind == TokenKind::Assign {
            self.eat_next_token();
            let token = self.next_token();
            value = Some(
                extract_token_value!(self, token, IntConstant, "integer constant")
                    .parse::<i32>()
                    .unwrap_or_default(),
            );
        }

        let ext = self.opt_parse_ext();
        opt_list_separator!(self);
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(EnumValueNode {
            range,
            name,
            value,
            ext,
        })
    }

    fn parse_struct(&mut self) -> Option<StructNode> {
        // Struct ::= 'struct' Identifier '{' Field* '}' Ext?

        let start = self.peek_next_token().range().start;
        expect_token!(self, Struct, "'struct'");
        let identifier = self.parse_identifier()?;
        expect_token!(self, Lbrace, "'{'");

        let mut fields = Vec::new();
        loop {
            break_opt_token_or_eof!(self, Rbrace);
            if let Some(field) = self.parse_field() {
                fields.push(field);
            } else {
                self.recover_to_next_line();
            }
        }
        let ext = self.opt_parse_ext();
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(StructNode {
            range,
            identifier,
            fields,
            ext,
        })
    }

    fn parse_field(&mut self) -> Option<FieldNode> {
        // Field ::= FieldID? FieldReq? FieldType Identifier ('=' ConstValue)? Ext? ListSeparator?

        let start = self.peek_next_token().range().start;
        let mut field_id = None;
        let mut field_req = None;

        let next_token = self.peek_next_token();
        match next_token.kind {
            TokenKind::IntConstant(ref id) => {
                field_id = Some(FieldIdNode {
                    range: next_token.range(),
                    id: id.parse().unwrap_or_default(),
                });
                self.eat_next_token();
                expect_token!(self, Colon, "':'");
            }
            TokenKind::Required | TokenKind::Optional => {
                field_req = Some(match next_token.kind {
                    TokenKind::Required => "required".to_string(),
                    TokenKind::Optional => "optional".to_string(),
                    _ => unreachable!(),
                });
                self.eat_next_token();
            }
            _ => {}
        }

        let next_token = self.peek_next_token();
        if let TokenKind::Required | TokenKind::Optional = next_token.kind {
            if !field_req.is_none() {
                self.add_error(
                    format!("Expected field type, but got {}", next_token.kind),
                    next_token.range(),
                );
                return None;
            }
            field_req = Some(match next_token.kind {
                TokenKind::Required => "required".to_string(),
                TokenKind::Optional => "optional".to_string(),
                _ => unreachable!(),
            });
            self.eat_next_token();
        }

        let field_type = self.parse_field_type()?;
        let identifier = self.parse_identifier()?;

        let mut default_value = None;
        let next_token = self.peek_next_token();
        if next_token.kind == TokenKind::Assign {
            self.eat_next_token();
            default_value = Some(self.parse_const_value()?);
        }

        let ext = self.opt_parse_ext();
        opt_list_separator!(self);
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(FieldNode {
            range,
            field_id,
            field_req,
            field_type,
            identifier,
            default_value,
            ext,
        })
    }

    fn parse_union(&mut self) -> Option<UnionNode> {
        // Union ::= 'union' Identifier '{' Field* '}'

        let start = self.peek_next_token().range().start;
        expect_token!(self, Union, "'union'");
        let identifier = self.parse_identifier()?;
        expect_token!(self, Lbrace, "'{'");

        let mut fields = Vec::new();
        loop {
            break_opt_token_or_eof!(self, Rbrace);
            if let Some(field) = self.parse_field() {
                fields.push(field);
            } else {
                self.recover_to_next_line();
            }
        }
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(UnionNode {
            range,
            identifier,
            fields,
        })
    }

    fn parse_exception(&mut self) -> Option<ExceptionNode> {
        // Exception ::= 'exception' Identifier '{' Field* '}'

        let start = self.peek_next_token().range().start;
        expect_token!(self, Exception, "'exception'");
        let identifier = self.parse_identifier()?;
        expect_token!(self, Lbrace, "'{'");

        let mut fields = Vec::new();
        loop {
            break_opt_token_or_eof!(self, Rbrace);
            if let Some(field) = self.parse_field() {
                fields.push(field);
            } else {
                self.recover_to_next_line();
            }
        }
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(ExceptionNode {
            range,
            identifier,
            fields,
        })
    }

    fn parse_service(&mut self) -> Option<ServiceNode> {
        // Service ::= 'service' Identifier ( 'extends' Identifier )? '{' Function* '}'

        let start = self.peek_next_token().range().start;
        expect_token!(self, Service, "'service'");
        let identifier = self.parse_identifier()?;

        let mut extends = None;
        let next_token = self.peek_next_token();
        if next_token.kind == TokenKind::Extends {
            self.eat_next_token();
            let extends_token = self.next_token();
            extends = Some(extract_token_value!(
                self,
                extends_token,
                Identifier,
                "identifier"
            ));
        }

        expect_token!(self, Lbrace, "'{'");
        let mut functions = Vec::new();
        loop {
            break_opt_token_or_eof!(self, Rbrace);
            if let Some(function) = self.parse_function() {
                functions.push(function);
            } else {
                self.recover_to_next_line();
            }
        }
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(ServiceNode {
            range,
            identifier,
            extends,
            functions,
        })
    }

    fn parse_function(&mut self) -> Option<FunctionNode> {
        // Function ::= 'oneway'? FunctionType Identifier '(' Field* ')' Throws? Ext? ListSeparator?

        let start = self.peek_next_token().range().start;
        let mut is_oneway = false;
        let next_token = self.peek_next_token();
        if next_token.kind == TokenKind::Oneway {
            is_oneway = true;
            self.eat_next_token();
        }

        let return_type = self.parse_function_type()?;
        let identifier = self.parse_identifier()?;
        expect_token!(self, Lparen, "'('");

        let mut fields = Vec::new();
        loop {
            break_opt_token_or_eof!(self, Rparen);
            fields.push(self.parse_field()?);
        }

        let mut throws = None;
        let next_token = self.peek_next_token();
        if next_token.kind == TokenKind::Throws {
            throws = Some(self.parse_throws()?);
        }
        let ext = self.opt_parse_ext();
        opt_list_separator!(self);
        let end = self.prev_token().unwrap_or_default().range().end;

        let range = Range { start, end };
        Some(FunctionNode {
            range,
            is_oneway,
            function_type: return_type,
            identifier,
            fields,
            throws,
            ext,
        })
    }

    fn parse_function_type(&mut self) -> Option<Box<dyn Node>> {
        // FunctionType ::= FieldType | 'void'

        let next_token = self.peek_next_token();
        if next_token.kind == TokenKind::Void {
            self.eat_next_token();
            return Some(Box::new(BaseTypeNode {
                name: "void".to_string(),
                range: next_token.range(),
            }));
        }
        self.parse_field_type()
    }

    fn parse_throws(&mut self) -> Option<Vec<FieldNode>> {
        // Throws ::= 'throws' '(' Field* ')'

        expect_token!(self, Throws, "'throws'");
        expect_token!(self, Lparen, "'('");

        let mut fields = Vec::new();
        loop {
            break_opt_token_or_eof!(self, Rparen);
            fields.push(self.parse_field()?);
        }

        Some(fields)
    }

    fn opt_parse_ext(&mut self) -> Option<ExtNode> {
        // Ext ::= '(' KeyValuePair* ')'

        let start = self.peek_next_token().range().start;
        if self.peek_next_token().kind != TokenKind::Lparen {
            return None;
        }
        expect_token!(self, Lparen, "'('");

        let mut kv_pairs = Vec::new();
        loop {
            break_opt_token_or_eof!(self, Rparen);
            kv_pairs.push(self.parse_key_value_pair()?);
        }

        let end = self.prev_token().unwrap_or_default().range().end;
        let range = Range { start, end };
        Some(ExtNode { kv_pairs, range })
    }

    fn parse_key_value_pair(&mut self) -> Option<(String, String)> {
        // KeyValuePair ::= Identifier '=' Literal ListSeparator?

        let token = self.next_token();
        let key = extract_token_value!(self, token, Identifier, "identifier");
        expect_token!(self, Assign, "'='");
        let token = self.next_token();
        let value = extract_token_value!(self, token, Literal, "literal");
        opt_list_separator!(self);

        Some((key, value))
    }
}

// error handling
impl<'a> Parser<'a> {
    fn add_error(&mut self, message: String, range: Range) {
        self.errors.push(Error { range, message });
    }

    fn recover_to_next_definition(&mut self) {
        loop {
            let next_token = self.peek_next_token();
            if next_token.is_eof() {
                return;
            }

            match next_token.kind {
                TokenKind::Const
                | TokenKind::Typedef
                | TokenKind::Enum
                | TokenKind::Struct
                | TokenKind::Union
                | TokenKind::Exception
                | TokenKind::Service => {
                    break;
                }
                _ => self.eat_next_token(),
            }
        }
    }

    fn recover_to_next_line(&mut self) {
        self.scanner.skip_to_next_line();
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

    #[test]
    fn parse_success() {
        let work_path = std::env::current_dir().unwrap();
        let file_path = work_path.join(Path::new("./lib/analyzer/test_file/ThriftTest.thrift"));
        let content = fs::read_to_string(&file_path)
            .unwrap()
            .chars()
            .collect::<Vec<_>>();

        let (document, errors) = Parser::new(&content).parse();
        println!("Document: {:#?}", document);
        println!("\nErrors:");
        for error in errors.iter() {
            println!("  {:?}: {}", error.range, error.message);
        }
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_failed() {
        let work_path = std::env::current_dir().unwrap();
        let file_path = work_path.join(Path::new(
            "./lib/analyzer/test_file/InvalidThriftTest.thrift",
        ));
        let content = fs::read_to_string(&file_path)
            .unwrap()
            .chars()
            .collect::<Vec<_>>();

        let (document, errors) = Parser::new(&content).parse();
        println!("Document: {:#?}", document);
        println!("\nErrors:");
        for error in errors.iter() {
            println!("  {:?}: {}", error.range, error.message);
        }
        assert!(!errors.is_empty());
    }
}
