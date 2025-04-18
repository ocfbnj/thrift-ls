//! Thrift Analyzer.

pub mod ast;
pub mod base;
pub mod macros;
pub mod parser;
pub mod scanner;
pub mod symbol;
pub mod token;

use std::{
    collections::{HashMap, HashSet},
    fs, io,
    path::{Path, PathBuf},
    rc::Rc,
};

use ast::{DefinitionNode, FieldNode, FieldTypeNode, FunctionNode, HeaderNode};
use base::{Location, Position};

use crate::analyzer::{
    ast::{DocumentNode, IdentifierNode, Node},
    base::Error,
    parser::Parser,
    symbol::SymbolTable,
};

/// Analyzer for Thrift files.
pub struct Analyzer {
    documents: HashMap<String, Vec<char>>,

    document_nodes: HashMap<String, Rc<DocumentNode>>,
    symbol_tables: HashMap<String, Rc<SymbolTable>>,

    errors: HashMap<String, Vec<Error>>,
    semantic_tokens: HashMap<String, Vec<u32>>,

    pub(crate) wasm_read_file: Option<Box<dyn Fn(String) -> io::Result<String>>>,
}

const KEYWORDS: &[&str] = &[
    "namespace",
    "include",
    "cpp_include",
    "const",
    "typedef",
    "extends",
    "required",
    "optional",
    "oneway",
    "void",
    "bool",
    "byte",
    "i8",
    "i16",
    "i32",
    "i64",
    "struct",
    "enum",
    "union",
    "exception",
    "service",
];

impl Analyzer {
    /// Create a new analyzer.
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            document_nodes: HashMap::new(),
            symbol_tables: HashMap::new(),
            errors: HashMap::new(),
            semantic_tokens: HashMap::new(),
            wasm_read_file: None,
        }
    }

    /// Sync a document.
    pub fn sync_document(&mut self, path: &str, content: &str) {
        self.documents
            .insert(path.to_string(), content.chars().collect());
        self.analyze(path);
    }

    /// Remove a document.
    pub fn remove_document(&mut self, path: &str) {
        self.documents.remove(path);
        self.document_nodes.remove(path);
        self.symbol_tables.remove(path);
        self.errors.remove(path);
        self.semantic_tokens.remove(path);
    }

    /// Get the errors for all files.
    pub fn errors(&self) -> &HashMap<String, Vec<Error>> {
        &self.errors
    }

    /// Get semantic tokens for a specific file.
    pub fn semantic_tokens(&self, path: &str) -> Option<&Vec<u32>> {
        self.semantic_tokens.get(path)
    }

    /// Get the semantic token types.
    pub fn semantic_token_types(&self) -> Vec<String> {
        vec!["type".to_string(), "function".to_string()]
    }

    /// Get the semantic token modifiers.
    pub fn semantic_token_modifiers(&self) -> Vec<String> {
        vec![]
    }

    /// Get the definition at a specific position.
    pub fn definition(&self, path: &str, pos: Position) -> Option<Location> {
        let document_node = self.document_nodes.get(path)?.as_ref();
        let identifier = self.find_identifier(document_node, pos)?;
        let symbol_table = self.symbol_tables.get(path)?;
        let (new_path, def, header) =
            symbol_table.find_definition_of_identifier_type(identifier)?;

        if identifier.position_in_namespace(pos) {
            if let Some(include) = header {
                return Some(Location {
                    path: path.to_string(),
                    range: include.range(),
                });
            }
            return None;
        }

        Some(Location {
            path: new_path,
            range: def.identifier().range(),
        })
    }

    /// Get the types for completion.
    pub fn types_for_completion(&self, path: &str, pos: Position) -> Vec<String> {
        let offset = match self.offset_at_position(path, pos) {
            Some(offset) => offset,
            None => return vec![],
        };
        let document = match self.documents.get(path) {
            Some(document) => document,
            None => return vec![],
        };
        let mut symbol_table = match self.symbol_tables.get(path) {
            Some(symbol_table) => symbol_table.clone(),
            None => return vec![],
        };

        if offset > 0 && document[offset - 1] == '.' {
            let word = match self.idet_prev_offset(path, offset - 1) {
                Some(word) => word,
                None => return vec!["".to_string()],
            };
            let table = match symbol_table.includes().get(&word) {
                Some(table) => table.clone(),
                None => return vec!["".to_string()],
            };
            symbol_table = table;
        }

        return symbol_table.types().keys().cloned().collect();
    }

    /// Get the includes for completion.
    pub fn includes_for_completion(&self, path: &str, _pos: Position) -> Vec<String> {
        let symbol_table = match self.symbol_tables.get(path) {
            Some(symbol_table) => symbol_table,
            None => return vec![],
        };

        symbol_table.includes().keys().cloned().collect()
    }

    /// Get the keywords for completion.
    pub fn keywords_for_completion(&self) -> Vec<String> {
        KEYWORDS.iter().map(|s| s.to_string()).collect()
    }
}

impl Analyzer {
    /// Analyze a document.
    fn analyze(&mut self, path: &str) {
        // clear previous state
        self.document_nodes.remove(path);
        self.symbol_tables.remove(path);
        self.errors.remove(path);
        self.semantic_tokens.remove(path);

        let mut visited = HashSet::new();
        self.parse_document(path, &mut visited, None);
        self.static_check(path);
        self.generate_semantic_tokens(path);
    }

    /// Recursively parse AST and build symbol tables for a file.
    fn parse_document(
        &mut self,
        path: &str,
        visited: &mut HashSet<String>,
        source: Option<(&str, &Rc<HeaderNode>)>,
    ) -> bool {
        // check for circular dependencies
        if visited.contains(path) {
            if let Some((source_path, node)) = source {
                let error = Error {
                    range: node.range(),
                    message: format!("Circular dependency detected: {}", path),
                };

                self.errors
                    .entry(source_path.to_string())
                    .or_default()
                    .push(error);
            }
            return false;
        }

        // mark file as being processed
        visited.insert(path.to_string());

        // if file is already parsed, return
        if self.document_nodes.contains_key(path) {
            return true;
        }

        // read the file
        let content = if let Some(content) = self.documents.get(path) {
            content
        } else {
            // try to read from local file system
            match self.read_file(path) {
                Ok(content) => &content.chars().collect(),
                Err(e) => {
                    if let Some((source_path, node)) = source {
                        let error = Error {
                            range: node.range(),
                            message: format!("Failed to read file {}: {}", path, e),
                        };

                        self.errors
                            .entry(source_path.to_string())
                            .or_default()
                            .push(error);
                    }
                    return false;
                }
            }
        };

        // parse the file
        let (document_node, errors) = Parser::new(content).parse();

        // store parser errors
        self.errors
            .entry(path.to_string())
            .or_default()
            .extend(errors.into_iter().map(|e| e));

        // track file dependencies
        let mut dependencies = Vec::new();
        for header in &document_node.headers {
            if let HeaderNode::Include(include) = header.as_ref() {
                if let Some(parent) = path_parent(path) {
                    dependencies.push((
                        parent.join(&include.literal).to_string_lossy().to_string(),
                        header.clone(),
                    ));
                }
            }
        }

        // build symbol table
        let mut symbol_table = SymbolTable::new_from_ast(path, &document_node);

        // recursively parse dependencies
        for (dep_path, header) in dependencies.iter() {
            let res = self.parse_document(dep_path, visited, Some((path, header)));
            visited.remove(dep_path.as_str());
            if !res {
                continue;
            }

            // add dependency to current symbol table
            if let Some(dep_table) = self.symbol_tables.get(dep_path) {
                symbol_table.add_dependency(dep_path, header.clone(), dep_table.clone());
            }
        }

        // store document
        self.symbol_tables
            .insert(path.to_string(), Rc::new(symbol_table));
        self.document_nodes
            .insert(path.to_string(), Rc::new(document_node));

        true
    }
}

/// Static check
impl Analyzer {
    fn static_check(&mut self, path: &str) {
        let document_node = match self.document_nodes.get(path) {
            Some(document_node) => document_node.clone(),
            None => return,
        };
        let symbol_table = match self.symbol_tables.get_mut(path) {
            Some(symbol_table) => symbol_table.clone(),
            None => return,
        };

        // type check
        symbol_table.check_document_types(document_node.as_ref());
        self.errors
            .entry(path.to_string())
            .or_default()
            .extend(symbol_table.errors().into_iter().map(|e| e));

        // field check
        self.document_check(path, document_node.as_ref());
    }

    fn document_check(&mut self, path: &str, document_node: &DocumentNode) {
        for definition in &document_node.definitions {
            match definition.as_ref() {
                DefinitionNode::Struct(struct_node) => {
                    self.fields_check(path, &struct_node.fields);
                }
                DefinitionNode::Union(union_node) => {
                    self.fields_check(path, &union_node.fields);
                }
                DefinitionNode::Exception(exception_node) => {
                    self.fields_check(path, &exception_node.fields);
                }
                DefinitionNode::Service(service_node) => {
                    self.functions_check(path, &service_node.functions);
                }
                _ => {}
            }
        }
    }

    fn fields_check(&mut self, path: &str, fields: &[FieldNode]) {
        let mut field_ids = HashSet::new();
        let mut field_identifiers = HashSet::new();

        for field in fields {
            if let Some(field_id) = &field.field_id {
                if field_ids.contains(&field_id.id) {
                    let error = Error {
                        range: field_id.range.clone(),
                        message: format!("Duplicate field ID: {}", field_id.id),
                    };
                    self.errors.entry(path.to_string()).or_default().push(error);
                } else {
                    field_ids.insert(field_id.id);
                }
            }

            let identifier_name = &field.identifier.name;
            if field_identifiers.contains(identifier_name) {
                let error = Error {
                    range: field.identifier.range.clone(),
                    message: format!("Duplicate field identifier: {}", identifier_name),
                };
                self.errors.entry(path.to_string()).or_default().push(error);
            } else {
                field_identifiers.insert(identifier_name.clone());
            }
        }
    }

    fn functions_check(&mut self, path: &str, functions: &[FunctionNode]) {
        let mut function_identifiers = HashSet::new();

        for function in functions {
            self.fields_check(path, &function.fields);

            let identifier_name = &function.identifier.name;
            if function_identifiers.contains(identifier_name) {
                let error = Error {
                    range: function.identifier.range.clone(),
                    message: format!("Duplicate function identifier: {}", identifier_name),
                };
                self.errors.entry(path.to_string()).or_default().push(error);
            } else {
                function_identifiers.insert(identifier_name.clone());
            }
        }
    }
}

/// Semantic tokens
impl Analyzer {
    /// Generate semantic tokens for a document.
    fn generate_semantic_tokens(&mut self, path: &str) {
        let field_type_identifiers = self.find_field_type_identifiers(path);
        let function_identifiers = self.find_function_identifiers(path);

        let mut identifiers: Vec<(&IdentifierNode, u32)> = Vec::new();
        for id in field_type_identifiers {
            identifiers.push((id, 0));
        }
        for id in function_identifiers {
            identifiers.push((id, 1));
        }

        let new_tokens = self.convert_identifiers_to_semantic_tokens(identifiers);
        self.semantic_tokens.insert(path.to_string(), new_tokens);
    }

    /// Find all IdentifierNode instances used as field types in the document nodes.
    fn find_field_type_identifiers(&self, path: &str) -> Vec<&IdentifierNode> {
        let mut result = Vec::new();

        if let Some(document_node) = self.document_nodes.get(path) {
            for definition in &document_node.definitions {
                match definition.as_ref() {
                    DefinitionNode::Const(const_node) => {
                        result.extend(self.collect_field_type_identifiers(&const_node.field_type));
                    }
                    DefinitionNode::Typedef(typedef_node) => {
                        result.extend(
                            self.collect_field_type_identifiers(&typedef_node.definition_type),
                        );
                        result.push(&typedef_node.identifier);
                    }
                    DefinitionNode::Struct(struct_node) => {
                        for field in &struct_node.fields {
                            result.extend(self.collect_field_type_identifiers(&field.field_type));
                        }
                    }
                    DefinitionNode::Union(union_node) => {
                        for field in &union_node.fields {
                            result.extend(self.collect_field_type_identifiers(&field.field_type));
                        }
                    }
                    DefinitionNode::Exception(exception_node) => {
                        for field in &exception_node.fields {
                            result.extend(self.collect_field_type_identifiers(&field.field_type));
                        }
                    }
                    DefinitionNode::Service(service_node) => {
                        if let Some(extends) = &service_node.extends {
                            result.push(extends);
                        }

                        for function in &service_node.functions {
                            if let Some(function_type) = &function.function_type {
                                result.extend(self.collect_field_type_identifiers(function_type));
                            }
                            for field in &function.fields {
                                result
                                    .extend(self.collect_field_type_identifiers(&field.field_type));
                            }
                            if let Some(throws) = &function.throws {
                                for throw in throws {
                                    result.extend(
                                        self.collect_field_type_identifiers(&throw.field_type),
                                    );
                                }
                            }
                        }
                    }

                    _ => {}
                }
            }
        }

        result
    }

    /// Collect all IdentifierNode instances used as field types in the document nodes.
    fn collect_field_type_identifiers<'a>(
        &'a self,
        field_type: &'a FieldTypeNode,
    ) -> Vec<&'a IdentifierNode> {
        match field_type {
            FieldTypeNode::Identifier(identifier) => vec![identifier],
            FieldTypeNode::BaseType(_) => vec![],
            FieldTypeNode::MapType(map_type) => {
                let mut result = self.collect_field_type_identifiers(&map_type.key_type);
                result.extend(self.collect_field_type_identifiers(&map_type.value_type));
                result
            }
            FieldTypeNode::SetType(set_type) => {
                self.collect_field_type_identifiers(&set_type.type_node)
            }
            FieldTypeNode::ListType(list_type) => {
                self.collect_field_type_identifiers(&list_type.type_node)
            }
        }
    }

    /// Convert a vector of IdentifierNode references to semantic tokens.
    fn convert_identifiers_to_semantic_tokens(
        &self,
        mut identifiers: Vec<(&IdentifierNode, u32)>,
    ) -> Vec<u32> {
        identifiers.sort_by_key(|(identifier, _)| identifier.range());

        let mut tokens = Vec::new();
        let mut prev_line = 0;
        let mut prev_char = 0;

        for (identifier, token_type) in identifiers {
            let range = identifier.range();

            // convert to 0-based line and column
            let line = range.start.line - 1 as u32;
            let char = range.start.column - 1 as u32;
            let length = identifier.name.len() as u32;

            // deltaLine: line number relative to the previous token
            let delta_line = line - prev_line;
            // deltaStart: start character relative to the previous token
            let delta_start = if delta_line == 0 {
                char - prev_char
            } else {
                char
            };
            // length: length of the token
            // tokenType: 0 for type, 1 for function (as defined in SemanticTokensLegend)
            // tokenModifiers: 0 for no modifiers
            tokens.extend_from_slice(&[delta_line, delta_start, length, token_type, 0]);

            prev_line = line;
            prev_char = char;
        }

        tokens
    }

    /// Find all function identifiers in the document nodes.
    fn find_function_identifiers(&self, path: &str) -> Vec<&IdentifierNode> {
        let mut result = Vec::new();

        if let Some(document_node) = self.document_nodes.get(path) {
            for definition in &document_node.definitions {
                match definition.as_ref() {
                    DefinitionNode::Service(service_node) => {
                        for function in &service_node.functions {
                            result.push(&function.identifier);
                        }
                    }
                    _ => {}
                }
            }
        }

        result
    }
}

/// Definition
impl Analyzer {
    /// Find an identifier at a specific position.
    fn find_identifier<'a>(&self, node: &'a dyn Node, pos: Position) -> Option<&'a IdentifierNode> {
        if !node.range().contains(pos) {
            return None;
        }

        if let Some(identifier) = node.as_any().downcast_ref::<IdentifierNode>() {
            return Some(identifier);
        }

        for child in node.children() {
            if let Some(identifier) = self.find_identifier(child, pos) {
                return Some(identifier);
            }
        }

        None
    }
}

/// Completion
impl Analyzer {
    /// Get the offset at a specific position.
    fn offset_at_position(&self, path: &str, pos: Position) -> Option<usize> {
        let document = self.documents.get(path)?;
        let mut offset = 0;
        let mut cur_pos = Position { line: 1, column: 1 };

        while offset < document.len() {
            if cur_pos >= pos {
                break;
            }

            if document[offset] == '\n' {
                offset += 1;
                cur_pos.line += 1;
                cur_pos.column = 1;
            } else if document[offset] == '\r' {
                offset += 1;
                cur_pos.line += 1;
                cur_pos.column = 1;
                if offset < document.len() && document[offset] == '\n' {
                    offset += 1;
                }
            } else {
                offset += 1;
                cur_pos.column += 1;
            }
        }

        if cur_pos == pos {
            Some(offset)
        } else {
            None
        }
    }

    /// Get the identifier at the previous offset. no consider the '.'.
    fn idet_prev_offset(&self, path: &str, offset: usize) -> Option<String> {
        let document = self.documents.get(path)?;

        Some(
            document[..offset]
                .iter()
                .rev()
                .take_while(|&&c| c.is_ascii_alphanumeric() || c == '_')
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect(),
        )
    }
}

impl Analyzer {
    fn read_file(&self, path: &str) -> io::Result<String> {
        if let Some(read_file) = &self.wasm_read_file {
            read_file(path.to_string())
        } else {
            fs::read_to_string(path)
        }
    }
}

/// Returns the parent path of a given path.
///
/// Build with WASM target on windows, `Path::new(path).parent()` always return `""`.
/// So we need to implement our own path_parent function.
fn path_parent(path: &str) -> Option<PathBuf> {
    let parent = Path::new(path).parent();
    if let Some(p) = parent {
        if p.to_string_lossy().len() > 0 {
            return Some(p.to_path_buf());
        }
    }

    if let Some(p) = path.rfind("\\") {
        return Some(PathBuf::from(&path[..p]));
    }

    parent.map(|p| p.to_path_buf())
}
