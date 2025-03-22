use crate::{
    ast::{
        BaseTypeNode, ConstNode, ConstValueNode, CppIncludeNode, DocumentNode, IdentifierNode,
        IncludeNode, ListTypeNode, MapTypeNode, NamespaceNode, Node, SetTypeNode, TypedefNode,
    },
    scanner::{Input, Scanner},
    token::{Token, TokenKind},
};

pub struct Parser {
    scanner: Scanner,
}

impl Parser {
    pub fn new(input: impl Input) -> Parser {
        Parser {
            scanner: Scanner::new(input),
        }
    }
}

impl Parser {
    pub fn parse(&mut self) -> DocumentNode {
        let node = DocumentNode {
            headers: self.parse_headers(),
            definitions: self.parse_definitions(),
        };

        node
    }

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

    fn parse_headers(&mut self) -> Vec<Box<dyn Node>> {
        let mut headers: Vec<Box<dyn Node>> = Vec::new();

        loop {
            let next_token = self.peek_next_token();

            match next_token.kind {
                TokenKind::Include => {
                    if let Some(include) = self.parse_include() {
                        headers.push(Box::new(include));
                    } else {
                        todo!()
                    }
                }
                TokenKind::CppInclude => {
                    if let Some(cpp_include) = self.parse_cpp_include() {
                        headers.push(Box::new(cpp_include));
                    } else {
                        todo!()
                    }
                }
                TokenKind::Namespace => {
                    if let Some(namespace) = self.parse_namespace() {
                        headers.push(Box::new(namespace));
                    } else {
                        todo!()
                    }
                }
                _ => break,
            }
        }

        headers
    }

    fn parse_include(&mut self) -> Option<IncludeNode> {
        let state = self.scanner.save_state();
        let token = self.next_token();
        if token.kind != TokenKind::Include {
            self.scanner.restore_state(state);
            return None;
        }

        let token = self.next_token();
        if let TokenKind::Literal(literal) = token.kind {
            return Some(IncludeNode { literal });
        } else {
            self.scanner.restore_state(state);
        }

        None
    }

    fn parse_cpp_include(&mut self) -> Option<CppIncludeNode> {
        let state = self.scanner.save_state();
        let token = self.next_token();
        if token.kind != TokenKind::CppInclude {
            self.scanner.restore_state(state);
            return None;
        }

        let token = self.next_token();
        if let TokenKind::Literal(literal) = token.kind {
            return Some(CppIncludeNode { literal });
        } else {
            self.scanner.restore_state(state);
        }

        None
    }

    fn parse_namespace(&mut self) -> Option<NamespaceNode> {
        let state = self.scanner.save_state();
        let token = self.next_token();
        if token.kind != TokenKind::Namespace {
            self.scanner.restore_state(state);
            return None;
        }

        let token = self.next_token();
        let mut scope = String::new();
        if let TokenKind::NamespaceScope(inner_scope) = token.kind {
            scope = inner_scope;
        } else {
            self.scanner.restore_state(state);
            return None;
        }

        let token = self.next_token();
        if let TokenKind::Identifier(name) = token.kind {
            return Some(NamespaceNode { name, scope });
        } else {
            self.scanner.restore_state(state);
        }

        None
    }

    fn parse_definitions(&mut self) -> Vec<Box<dyn Node>> {
        // Definition ::= Const | Typedef | Enum | Struct | Union | Exception | Service

        let mut definitions: Vec<Box<dyn Node>> = Vec::new();

        loop {
            let next_token = self.peek_next_token();
            match next_token.kind {
                TokenKind::Const => {
                    if let Some(const_node) = self.parse_const() {
                        definitions.push(Box::new(const_node));
                    } else {
                        todo!()
                    }
                }
                TokenKind::Typedef => {
                    if let Some(typedef_node) = self.parse_typedef() {
                        definitions.push(Box::new(typedef_node));
                    } else {
                        todo!()
                    }
                }
                _ => break,
            }
        }

        definitions
    }

    fn parse_const(&mut self) -> Option<ConstNode> {
        // Const ::= 'const' FieldType Identifier '=' ConstValue ListSeparator?

        let state = self.scanner.save_state();
        let token = self.next_token();
        if token.kind != TokenKind::Const {
            self.scanner.restore_state(state);
            return None;
        }

        let field_type = self.parse_field_type();
        if field_type.is_none() {
            self.scanner.restore_state(state);
            return None;
        }

        let token = self.next_token();
        let mut identifier = String::new();
        if let TokenKind::Identifier(name) = token.kind {
            identifier = name;
        } else {
            self.scanner.restore_state(state);
            return None;
        }

        let token = self.next_token();
        if token.kind != TokenKind::Assign {
            self.scanner.restore_state(state);
            return None;
        }

        let const_value = self.parse_const_value();

        let token = self.peek_next_token();
        if token.is_line_separator() {
            self.eat_next_token();
        }

        Some(ConstNode {
            field_type: field_type.unwrap(),
            name: identifier,
            value: const_value
                .map(|x| -> Box<dyn Node> { Box::new(x) })
                .unwrap(),
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
            TokenKind::Map => {
                return self.parse_map_type().map(|x| Box::new(x) as Box<dyn Node>);
            }
            TokenKind::Set => {
                return self.parse_set_type().map(|x| Box::new(x) as Box<dyn Node>);
            }
            TokenKind::List => {
                return self.parse_list_type().map(|x| Box::new(x) as Box<dyn Node>);
            }
            _ => {
                todo!()
            }
        }
    }

    fn parse_map_type(&mut self) -> Option<MapTypeNode> {
        // MapType ::= 'map' '<' FieldType ',' FieldType '>'

        let state = self.scanner.save_state();
        let token = self.next_token();
        if token.kind != TokenKind::Map {
            self.scanner.restore_state(state);
            return None;
        }

        let mut token = self.next_token();
        let mut cpp_type = None;
        if token.kind == TokenKind::CppType {
            let inner_token = self.next_token();
            if let TokenKind::Identifier(identifier) = inner_token.kind {
                cpp_type = Some(identifier);
                token = self.next_token();
            } else {
                todo!()
            }
        }

        if token.kind != TokenKind::Less {
            self.scanner.restore_state(state);
            return None;
        }

        let key_type = self.parse_field_type();

        let token = self.next_token();
        if token.kind != TokenKind::ListSeparator(',') {
            self.scanner.restore_state(state);
            return None;
        }

        let value_type = self.parse_field_type();

        let token = self.next_token();
        if token.kind != TokenKind::Greater {
            self.scanner.restore_state(state);
            return None;
        }

        Some(MapTypeNode {
            cpp_type,
            key_type: key_type.unwrap(),
            value_type: value_type.unwrap(),
        })
    }

    fn parse_set_type(&mut self) -> Option<SetTypeNode> {
        // SetType ::='set' '<' FieldType '>'

        let state = self.scanner.save_state();
        let token = self.next_token();
        if token.kind != TokenKind::Set {
            self.scanner.restore_state(state);
            return None;
        }

        let mut token = self.next_token();
        let mut cpp_type = None;
        if token.kind == TokenKind::CppType {
            let inner_token = self.next_token();
            if let TokenKind::Identifier(identifier) = inner_token.kind {
                cpp_type = Some(identifier);
                token = self.next_token();
            } else {
                todo!()
            }
        }

        if token.kind != TokenKind::Less {
            self.scanner.restore_state(state);
            return None;
        }

        let type_node = self.parse_field_type();

        let token = self.next_token();
        if token.kind != TokenKind::Greater {
            self.scanner.restore_state(state);
            return None;
        }

        Some(SetTypeNode {
            cpp_type,
            type_node: type_node.unwrap(),
        })
    }

    fn parse_list_type(&mut self) -> Option<ListTypeNode> {
        // ListType ::='list' '<' FieldType '>'

        let state = self.scanner.save_state();
        let token = self.next_token();
        if token.kind != TokenKind::List {
            self.scanner.restore_state(state);
            return None;
        }

        let mut token = self.next_token();
        let mut cpp_type = None;
        if token.kind == TokenKind::CppType {
            let inner_token = self.next_token();
            if let TokenKind::Identifier(identifier) = inner_token.kind {
                cpp_type = Some(identifier);
                token = self.next_token();
            } else {
                todo!()
            }
        }

        if token.kind != TokenKind::Less {
            self.scanner.restore_state(state);
            return None;
        }

        let type_node = self.parse_field_type();

        let token = self.next_token();
        if token.kind != TokenKind::Greater {
            self.scanner.restore_state(state);
            return None;
        }

        Some(ListTypeNode {
            cpp_type,
            type_node: type_node.unwrap(),
        })
    }

    fn parse_const_value(&mut self) -> Option<ConstValueNode> {
        let next_token = self.next_token();
        match next_token.kind {
            TokenKind::IntConstant(value) => Some(ConstValueNode { value }),
            TokenKind::DoubleConstant(value) => Some(ConstValueNode { value }),
            _ => {
                todo!()
            }
        }
    }

    fn parse_typedef(&mut self) -> Option<TypedefNode> {
        // Typedef ::= 'typedef' DefinitionType Identifier

        let state = self.scanner.save_state();
        let token = self.next_token();
        if token.kind != TokenKind::Typedef {
            self.scanner.restore_state(state);
            return None;
        }

        let definition_type = self.parse_definition_type();
        if definition_type.is_none() {
            self.scanner.restore_state(state);
            return None;
        }

        let token = self.next_token();
        let mut identifier = String::new();
        if let TokenKind::Identifier(name) = token.kind {
            identifier = name;
        } else {
            self.scanner.restore_state(state);
            return None;
        }

        Some(TypedefNode {
            name: identifier,
            definition_type: definition_type.unwrap(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::scanner::FileInput;

    use super::*;

    #[test]
    fn test_scan() {
        let work_path = std::env::current_dir().unwrap();
        let file_path = work_path.join(std::path::Path::new("./lib/test_file/user.thrift"));
        let mut parser = Parser::new(FileInput::new(&file_path));

        let document = parser.parse();
        println!("{:#?}", document);
    }
}
