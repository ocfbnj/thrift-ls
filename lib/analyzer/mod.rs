//! Thrift Analyzer.

pub mod ast;
pub mod base;
pub mod macros;
pub mod parser;
pub mod scanner;
pub mod symbol;
pub mod token;

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    rc::Rc,
};

use base::{Location, Position};

use crate::analyzer::{
    ast::{
        DocumentNode, ExceptionNode, IdentifierNode, IncludeNode, ListTypeNode, MapTypeNode, Node,
        ServiceNode, SetTypeNode, StructNode, UnionNode,
    },
    base::Error,
    parser::Parser,
    symbol::SymbolTable,
};

/// Analyzer for Thrift files.
pub struct Analyzer {
    documents: HashMap<String, Vec<char>>,

    document_nodes: HashMap<String, DocumentNode>,
    symbol_tables: HashMap<String, Rc<RefCell<SymbolTable>>>,

    errors: HashMap<String, Vec<Error>>,
    semantic_tokens: HashMap<String, Vec<u32>>,
}

impl Analyzer {
    /// Create a new analyzer.
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            document_nodes: HashMap::new(),
            symbol_tables: HashMap::new(),
            errors: HashMap::new(),
            semantic_tokens: HashMap::new(),
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
        let document_node = self.document_nodes.get(path)?;
        let identifier = self.find_identifier(document_node, pos)?;
        let symbol_table = self.symbol_tables.get(path)?;
        symbol_table
            .borrow()
            .find_definition_of_identifier_type(identifier)
            .map(|(path, definition)| Location {
                path,
                range: definition.identifier().range(),
            })
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

        self.parse_document(path, Rc::new(RefCell::new(HashSet::new())), None);
        self.type_checking(path);
        self.generate_semantic_tokens(path);
    }

    /// Recursively parse AST and build symbol tables for a file.
    fn parse_document(
        &mut self,
        path: &str,
        visited: Rc<RefCell<HashSet<String>>>,
        source: Option<(&str, &IncludeNode)>,
    ) -> bool {
        // check for circular dependencies
        if visited.borrow().contains(path) {
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
        visited.borrow_mut().insert(path.to_string());

        // if file is already parsed, return
        if self.document_nodes.contains_key(path) {
            return true;
        }

        // read the file
        let content = if let Some(content) = self.documents.get(path) {
            content
        } else {
            // try to read from local file system
            match fs::read_to_string(path) {
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
            let header = header.as_ref().as_any();
            if let Some(include) = header.downcast_ref::<IncludeNode>() {
                if let Some(parent) = Path::new(path).parent() {
                    dependencies.push((
                        parent.join(&include.literal).to_string_lossy().to_string(),
                        include,
                    ));
                }
            }
        }

        // build symbol table
        let symbol_table = Rc::new(RefCell::new(SymbolTable::new_from_ast(
            path,
            &document_node,
        )));

        // recursively parse dependencies
        for (dep_path, include_node) in dependencies.iter() {
            let res = self.parse_document(dep_path, visited.clone(), Some((path, include_node)));
            visited.borrow_mut().remove(dep_path.as_str());
            if !res {
                continue;
            }

            // add dependency to current symbol table
            if let Some(dep_table) = self.symbol_tables.get(dep_path) {
                symbol_table
                    .borrow_mut()
                    .add_dependency(dep_path, dep_table.clone());
            }
        }

        // store document
        self.symbol_tables.insert(path.to_string(), symbol_table);
        self.document_nodes.insert(path.to_string(), document_node);

        true
    }
}

/// Type checking
impl Analyzer {
    fn type_checking(&mut self, path: &str) {
        if let Some(document_node) = self.document_nodes.get(path) {
            if let Some(symbol_table) = self.symbol_tables.get_mut(path) {
                symbol_table
                    .borrow_mut()
                    .check_document_types(document_node);

                self.errors
                    .entry(path.to_string())
                    .or_default()
                    .extend(symbol_table.borrow().errors().iter().map(|e| e.clone()));
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
                let definition = definition.as_ref().as_any();

                if let Some(struct_node) = definition.downcast_ref::<StructNode>() {
                    for field in &struct_node.fields {
                        self.collect_field_type_identifiers(&field.field_type, &mut result, path);
                    }
                } else if let Some(union_node) = definition.downcast_ref::<UnionNode>() {
                    for field in &union_node.fields {
                        self.collect_field_type_identifiers(&field.field_type, &mut result, path);
                    }
                } else if let Some(exception_node) = definition.downcast_ref::<ExceptionNode>() {
                    for field in &exception_node.fields {
                        self.collect_field_type_identifiers(&field.field_type, &mut result, path);
                    }
                } else if let Some(service_node) = definition.downcast_ref::<ServiceNode>() {
                    for function in &service_node.functions {
                        self.collect_field_type_identifiers(
                            &function.function_type,
                            &mut result,
                            path,
                        );
                        for field in &function.fields {
                            self.collect_field_type_identifiers(
                                &field.field_type,
                                &mut result,
                                path,
                            );
                        }
                        if let Some(throws) = &function.throws {
                            for throw in throws {
                                self.collect_field_type_identifiers(
                                    &throw.field_type,
                                    &mut result,
                                    path,
                                );
                            }
                        }
                    }
                }
            }
        }

        result
    }

    /// Collect all IdentifierNode instances used as field types in the document nodes.
    fn collect_field_type_identifiers<'a>(
        &'a self,
        field_type: &'a Box<dyn Node>,
        result: &mut Vec<&'a IdentifierNode>,
        path: &str,
    ) {
        let field_type = field_type.as_ref().as_any();
        if let Some(identifier) = field_type.downcast_ref::<IdentifierNode>() {
            result.push(identifier);
        } else if let Some(list_type) = field_type.downcast_ref::<ListTypeNode>() {
            self.collect_field_type_identifiers(&list_type.type_node, result, path);
        } else if let Some(set_type) = field_type.downcast_ref::<SetTypeNode>() {
            self.collect_field_type_identifiers(&set_type.type_node, result, path);
        } else if let Some(map_type) = field_type.downcast_ref::<MapTypeNode>() {
            self.collect_field_type_identifiers(&map_type.key_type, result, path);
            self.collect_field_type_identifiers(&map_type.value_type, result, path);
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
                let definition = definition.as_ref().as_any();

                if let Some(service_node) = definition.downcast_ref::<ServiceNode>() {
                    for function in &service_node.functions {
                        result.push(&function.identifier);
                    }
                }
            }
        }

        result
    }
}

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
