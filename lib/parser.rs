use crate::{
    ast::{
        BaseTypeNode, ConstNode, ConstValueNode, CppIncludeNode, DocumentNode, EnumNode,
        EnumValueNode, ExceptionNode, FieldNode, FunctionNode, IdentifierNode, IncludeNode,
        ListTypeNode, MapTypeNode, NamespaceNode, Node, ServiceNode, SetTypeNode, StructNode,
        TypedefNode, UnionNode,
    },
    break_opt_token_or_eof, expect, expect_token, extract_token_value, opt_list_separator,
    parse_definition, parse_header,
    scanner::{Input, Scanner},
    token::{Location, Token, TokenKind},
};

#[derive(Debug)]
pub struct ParseError {
    pub location: Location,
    pub message: String,
}

pub struct Parser {
    scanner: Scanner,
    errors: Vec<ParseError>,
}

impl Parser {
    pub fn new(input: impl Input) -> Parser {
        Parser {
            scanner: Scanner::new(input),
            errors: Vec::new(),
        }
    }

    pub fn parse(&mut self) -> DocumentNode {
        let node = DocumentNode {
            headers: self.parse_headers(),
            definitions: self.parse_definitions(),
        };

        node
    }

    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }
}

impl Parser {
    fn next_token(&mut self) -> Token {
        self.skip_comment_tokens();
        let (next_token, _) = self.scanner.scan();
        next_token
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
impl Parser {
    fn parse_headers(&mut self) -> Vec<Box<dyn Node>> {
        // Headers ::= ( Include | CppInclude | Namespace )*
        let mut headers: Vec<Box<dyn Node>> = Vec::new();

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

        expect_token!(self, Include, "'include'");
        let token = self.next_token();
        let literal = extract_token_value!(self, token, Literal, "literal");

        Some(IncludeNode { literal })
    }

    fn parse_cpp_include(&mut self) -> Option<CppIncludeNode> {
        // CppInclude ::= 'cpp_include' Literal

        expect_token!(self, CppInclude, "'cpp_include'");
        let token = self.next_token();
        let literal = extract_token_value!(self, token, Literal, "literal");

        Some(CppIncludeNode { literal })
    }

    fn parse_namespace(&mut self) -> Option<NamespaceNode> {
        // Namespace ::= 'namespace' NamespaceScope Identifier

        expect_token!(self, Namespace, "'namespace'");
        let token = self.next_token();
        let scope = extract_token_value!(self, token, NamespaceScope, "namespace scope");
        let token = self.next_token();
        let name = extract_token_value!(self, token, Identifier, "identifier");

        Some(NamespaceNode { name, scope })
    }
}

// parse definitions
impl Parser {
    fn parse_definitions(&mut self) -> Vec<Box<dyn Node>> {
        // Definitions ::= ( Const | Typedef | Enum | Struct | Union | Exception | Service )*

        let mut definitions: Vec<Box<dyn Node>> = Vec::new();

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

        expect_token!(self, Const, "'const'");
        let field_type = self.parse_field_type()?;
        let token = self.next_token();
        let name = extract_token_value!(self, token, Identifier, "identifier");
        expect_token!(self, Assign, "'='");
        let value = Box::new(self.parse_const_value()?);
        opt_list_separator!(self);

        Some(ConstNode {
            field_type,
            name,
            value,
        })
    }

    fn parse_field_type(&mut self) -> Option<Box<dyn Node>> {
        // FieldType ::= Identifier | DefinitionType

        let next_token = self.peek_next_token();
        match next_token.kind {
            TokenKind::Identifier(identifier) => {
                self.eat_next_token();
                return Some(Box::new(IdentifierNode { name: identifier }));
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
            TokenKind::BaseType(base_type) => {
                self.eat_next_token();
                return Some(Box::new(BaseTypeNode { name: base_type }));
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
                self.add_error_at(
                    format!("Expected map, set, or list, but got {:?}", next_token.kind),
                    next_token.location,
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

        self.eat_next_token();
        let token = self.next_token();
        Some(extract_token_value!(self, token, Identifier, "identifier"))
    }

    fn parse_map_type(&mut self) -> Option<MapTypeNode> {
        // MapType ::= 'map' CppType? '<' FieldType ',' FieldType '>'

        expect_token!(self, Map, "'map'");
        let cpp_type = self.opt_parse_cpp_type();

        expect_token!(self, Less, "'<'");
        let key_type = self.parse_field_type()?;
        expect!(self, TokenKind::ListSeparator(','), "','");
        let value_type = self.parse_field_type()?;
        expect_token!(self, Greater, "'>'");

        Some(MapTypeNode {
            cpp_type,
            key_type,
            value_type,
        })
    }

    fn parse_set_type(&mut self) -> Option<SetTypeNode> {
        // SetType ::= 'set' CppType? '<' FieldType '>'

        expect_token!(self, Set, "'set'");
        let cpp_type = self.opt_parse_cpp_type();

        expect!(self, TokenKind::Less, "'<'");
        let type_node = self.parse_field_type()?;
        expect_token!(self, Greater, "'>'");

        Some(SetTypeNode {
            cpp_type,
            type_node,
        })
    }

    fn parse_list_type(&mut self) -> Option<ListTypeNode> {
        // ListType ::= 'list' CppType? '<' FieldType '>'

        expect_token!(self, List, "'list'");
        let cpp_type = self.opt_parse_cpp_type();

        expect!(self, TokenKind::Less, "'<'");
        let type_node = self.parse_field_type()?;
        expect_token!(self, Greater, "'>'");

        Some(ListTypeNode {
            cpp_type,
            type_node,
        })
    }

    fn parse_const_value(&mut self) -> Option<ConstValueNode> {
        // ConstValue ::= IntConstant | DoubleConstant | Literal | Identifier | ConstList | ConstMap

        let next_token = self.peek_next_token();
        match next_token.kind {
            TokenKind::IntConstant(value)
            | TokenKind::DoubleConstant(value)
            | TokenKind::Literal(value)
            | TokenKind::Identifier(value) => {
                self.eat_next_token();
                Some(ConstValueNode { value })
            }
            TokenKind::Lbrack => self.parse_const_list(),
            TokenKind::Lbrace => self.parse_const_map(),
            _ => {
                self.eat_next_token();
                self.add_error_at(
                    format!("Expected constant value, but got {:?}", next_token.kind),
                    next_token.location,
                );
                None
            }
        }
    }

    fn parse_const_list(&mut self) -> Option<ConstValueNode> {
        // ConstList ::= '[' (ConstValue ListSeparator?)* ']'

        expect_token!(self, Lbrack, "'['");
        let mut values = Vec::new();
        loop {
            break_opt_token_or_eof!(self, Rbrack);
            values.push(self.parse_const_value()?.value);
            opt_list_separator!(self);
        }

        Some(ConstValueNode {
            value: format!("[{}]", values.join(", ")),
        })
    }

    fn parse_const_map(&mut self) -> Option<ConstValueNode> {
        // ConstMap ::= '{' ConstMapValue* '}'

        expect_token!(self, Lbrace, "'{'");
        let mut pairs = Vec::new();
        loop {
            break_opt_token_or_eof!(self, Rbrace);
            pairs.push(self.parse_const_map_value()?);
        }

        Some(ConstValueNode {
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

        expect_token!(self, Typedef, "'typedef'");
        let definition_type = self.parse_definition_type()?;
        let token = self.next_token();
        let name = extract_token_value!(self, token, Identifier, "identifier");

        Some(TypedefNode {
            name,
            definition_type,
        })
    }

    fn parse_enum(&mut self) -> Option<EnumNode> {
        // Enum ::= 'enum' Identifier '{' EnumValue* '}'

        expect_token!(self, Enum, "'enum'");
        let token = self.next_token();
        let name = extract_token_value!(self, token, Identifier, "identifier");
        expect_token!(self, Lbrace, "'{'");

        let mut values = Vec::new();
        loop {
            break_opt_token_or_eof!(self, Rbrace);
            values.push(self.parse_enum_value()?);
        }

        Some(EnumNode { name, values })
    }

    fn parse_enum_value(&mut self) -> Option<EnumValueNode> {
        // EnumValue ::= Identifier ('=' IntConstant)? ListSeparator?

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
                    .unwrap(),
            );
        }

        opt_list_separator!(self);

        Some(EnumValueNode { name, value })
    }

    fn parse_struct(&mut self) -> Option<StructNode> {
        // Struct ::= 'struct' Identifier '{' Field* '}'

        expect_token!(self, Struct, "'struct'");
        let token = self.next_token();
        let name = extract_token_value!(self, token, Identifier, "identifier");
        expect_token!(self, Lbrace, "'{'");

        let mut fields = Vec::new();
        loop {
            break_opt_token_or_eof!(self, Rbrace);
            fields.push(self.parse_field()?);
        }

        Some(StructNode { name, fields })
    }

    fn parse_field(&mut self) -> Option<FieldNode> {
        // Field ::= FieldID? FieldReq? FieldType Identifier ('=' ConstValue)? ListSeparator?
        // FieldID ::= IntConstant ':'
        // FieldReq ::= 'required' | 'optional'

        let mut field_id = None;
        let mut field_req = None;

        let next_token = self.peek_next_token();
        match next_token.kind {
            TokenKind::IntConstant(id) => {
                field_id = Some(id.parse().unwrap());
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
                self.add_error_at(
                    format!("Expected field type, but got {:?}", next_token.kind),
                    next_token.location,
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
        let token = self.next_token();
        let identifier = extract_token_value!(self, token, Identifier, "identifier");

        let mut default_value = None;
        let next_token = self.peek_next_token();
        if next_token.kind == TokenKind::Assign {
            self.eat_next_token();
            default_value = Some(self.parse_const_value()?);
        }

        opt_list_separator!(self);

        Some(FieldNode {
            field_id,
            field_req,
            field_type,
            name: identifier,
            default_value,
        })
    }

    fn parse_union(&mut self) -> Option<UnionNode> {
        // Union ::= 'union' Identifier '{' Field* '}'

        expect_token!(self, Union, "'union'");
        let token = self.next_token();
        let name = extract_token_value!(self, token, Identifier, "identifier");
        expect_token!(self, Lbrace, "'{'");

        let mut fields = Vec::new();
        loop {
            break_opt_token_or_eof!(self, Rbrace);
            fields.push(self.parse_field()?);
        }

        Some(UnionNode { name, fields })
    }

    fn parse_exception(&mut self) -> Option<ExceptionNode> {
        // Exception ::= 'exception' Identifier '{' Field* '}'

        expect_token!(self, Exception, "'exception'");
        let token = self.next_token();
        let name = extract_token_value!(self, token, Identifier, "identifier");
        expect_token!(self, Lbrace, "'{'");

        let mut fields = Vec::new();
        loop {
            break_opt_token_or_eof!(self, Rbrace);
            fields.push(self.parse_field()?);
        }

        Some(ExceptionNode { name, fields })
    }

    fn parse_service(&mut self) -> Option<ServiceNode> {
        // Service ::= 'service' Identifier ( 'extends' Identifier )? '{' Function* '}'

        expect_token!(self, Service, "'service'");
        let token = self.next_token();
        let name = extract_token_value!(self, token, Identifier, "identifier");

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
            functions.push(self.parse_function()?);
        }

        Some(ServiceNode {
            name,
            extends,
            functions,
        })
    }

    fn parse_function(&mut self) -> Option<FunctionNode> {
        // Function ::= 'oneway'? FunctionType Identifier '(' Field* ')' Throws? ListSeparator?

        let mut is_oneway = false;
        let next_token = self.peek_next_token();
        if next_token.kind == TokenKind::Oneway {
            is_oneway = true;
            self.eat_next_token();
        }

        let return_type = self.parse_function_type()?;
        let token = self.next_token();
        let name = extract_token_value!(self, token, Identifier, "identifier");
        expect_token!(self, Lparen, "'('");

        let mut parameters = Vec::new();
        loop {
            break_opt_token_or_eof!(self, Rparen);
            parameters.push(self.parse_field()?);
        }

        let mut throws = None;
        let next_token = self.peek_next_token();
        if next_token.kind == TokenKind::Throws {
            throws = Some(self.parse_throws()?);
        }
        opt_list_separator!(self);

        Some(FunctionNode {
            is_oneway,
            return_type,
            name,
            parameters,
            throws,
        })
    }

    fn parse_function_type(&mut self) -> Option<Box<dyn Node>> {
        // FunctionType ::= FieldType | 'void'
        let next_token = self.peek_next_token();
        if next_token.kind == TokenKind::Void {
            self.eat_next_token();
            return Some(Box::new(BaseTypeNode {
                name: "void".to_string(),
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
}

// error handling
impl Parser {
    fn add_error_at(&mut self, message: String, location: Location) {
        self.errors.push(ParseError { location, message });
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
    use crate::scanner::FileInput;

    use super::*;

    #[test]
    fn parse_success() {
        let work_path = std::env::current_dir().unwrap();
        let file_path = work_path.join(std::path::Path::new("./lib/test_file/ThriftTest.thrift"));
        let mut parser = Parser::new(FileInput::new(&file_path));

        let document = parser.parse();
        println!("Document: {:#?}", document);
        println!("\nErrors:");
        for error in parser.errors() {
            println!("  {}: {}", error.location, error.message);
        }
        assert!(parser.errors().is_empty());
    }

    #[test]
    fn parse_failed() {
        let work_path = std::env::current_dir().unwrap();
        let file_path = work_path.join(std::path::Path::new(
            "./lib/test_file/InvalidThriftTest.thrift",
        ));
        let mut parser = Parser::new(FileInput::new(&file_path));

        let document = parser.parse();
        println!("Document: {:#?}", document);
        println!("\nErrors:");
        for error in parser.errors() {
            println!("  {}: {}", error.location, error.message);
        }
        assert!(!parser.errors().is_empty());
    }
}
