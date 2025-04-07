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
    path::PathBuf,
    rc::Rc,
    sync::{Arc, RwLock},
};

use crate::{
    analyzer::{
        ast::{
            DocumentNode, ExceptionNode, IdentifierNode, IncludeNode, ListTypeNode, MapTypeNode,
            Node, ServiceNode, SetTypeNode, StructNode, UnionNode,
        },
        base::Error,
        parser::Parser,
        scanner::{EmptyInput, FileInput},
        symbol::SymbolTable,
    },
    lsp,
};

/// Analyzer for Thrift files.
pub struct Analyzer {
    parser: Parser,
    documents: HashMap<PathBuf, Vec<char>>,

    document_nodes: HashMap<PathBuf, DocumentNode>,
    symbol_tables: HashMap<PathBuf, Arc<RwLock<SymbolTable>>>,
    errors: HashMap<PathBuf, Vec<Error>>,
    file_dependencies: HashMap<PathBuf, Vec<(PathBuf, IncludeNode)>>,
    semantic_tokens: HashMap<PathBuf, Vec<u32>>,
}

impl Analyzer {
    /// Create a new analyzer.
    pub fn new() -> Self {
        Self {
            parser: Parser::new(EmptyInput),
            documents: HashMap::new(),
            document_nodes: HashMap::new(),
            symbol_tables: HashMap::new(),
            errors: HashMap::new(),
            file_dependencies: HashMap::new(),
            semantic_tokens: HashMap::new(),
        }
    }

    /// Sync a document.
    pub fn sync_document(&mut self, path: PathBuf, content: &str) {
        self.documents.insert(path, content.chars().collect());
    }

    /// Analyze all documents.
    pub fn analyze(&mut self) {
        // clear previous state
        self.document_nodes.clear();
        self.symbol_tables.clear();
        self.errors.clear();
        self.file_dependencies.clear();

        // parse all files
        let paths: Vec<PathBuf> = self.documents.keys().cloned().collect();
        for path in paths {
            let visited_files = Rc::new(RefCell::new(HashSet::new()));
            self.parse_file(&path, visited_files, None);
        }

        // build global symbol tables recursively
        for path in self.symbol_tables.keys() {
            self.build_symbol_table(path, Rc::new(RefCell::new(HashSet::new())));
        }

        // type checking
        for (path, document_node) in &self.document_nodes {
            if let Some(symbol_table) = self.symbol_tables.get_mut(path) {
                symbol_table
                    .write()
                    .unwrap()
                    .check_document_types(document_node);

                self.errors
                    .entry(path.clone())
                    .or_default()
                    .extend(symbol_table.read().unwrap().errors().to_vec());
            }
        }

        // generate semantic tokens
        self.generate_semantic_tokens();
    }

    /// Get the errors.
    pub fn errors(&self) -> &HashMap<PathBuf, Vec<Error>> {
        &self.errors
    }

    /// Get semantic tokens for a specific file.
    pub fn semantic_tokens(&self, path: &PathBuf) -> Option<&Vec<u32>> {
        self.semantic_tokens.get(path)
    }

    /// Recursively parse AST and build symbol tables for a file.
    fn parse_file(
        &mut self,
        path: &PathBuf,
        visited_files: Rc<RefCell<HashSet<PathBuf>>>,
        include_node: Option<IncludeNode>,
    ) {
        // check for circular dependencies
        if visited_files.borrow().contains(path) {
            if let Some(include_node) = include_node {
                let error = Error {
                    range: include_node.range(),
                    message: format!("Circular dependency detected: {}", path.display()),
                };

                let include_path = include_node.range().start.path;
                self.errors.entry(include_path).or_default().push(error);
                return;
            }
        }

        // if file is already parsed, return
        if self.document_nodes.contains_key(path) {
            return;
        }

        // mark file as being processed
        visited_files.borrow_mut().insert(path.clone());

        // read the file
        let content = if let Some(content) = self.documents.get(path) {
            content.clone()
        } else {
            // try to read from local file system
            match fs::read_to_string(path) {
                Ok(content) => content.chars().collect(),
                Err(e) => {
                    if let Some(include_node) = include_node {
                        let error = Error {
                            range: include_node.range(),
                            message: format!("Failed to read file {}: {}", path.display(), e),
                        };

                        let include_path = include_node.range().start.path;
                        self.errors.entry(include_path).or_default().push(error);
                    }
                    return;
                }
            }
        };

        // parse the file
        let file_input = FileInput::new_with_content(path, &content);
        self.parser.reset(file_input);
        let document_node = self.parser.parse();

        // store parser errors
        self.errors
            .entry(path.clone())
            .or_default()
            .extend(self.parser.errors().to_vec());

        // build symbol table
        self.symbol_tables.insert(
            path.clone(),
            Arc::new(RwLock::new(SymbolTable::new_from_ast(&document_node))),
        );

        // track file dependencies
        let mut dependencies = Vec::new();
        for header in &document_node.headers {
            let header = header.as_ref().as_any();
            if let Some(include) = header.downcast_ref::<IncludeNode>() {
                if let Some(parent) = path.parent() {
                    dependencies.push((parent.join(&include.literal), include.clone()));
                }
            }
        }
        self.file_dependencies
            .insert(path.clone(), dependencies.clone());

        // store document
        self.document_nodes.insert(path.clone(), document_node);

        // recursively parse dependencies
        for (dep_path, include_node) in dependencies {
            self.parse_file(&dep_path, visited_files.clone(), Some(include_node));
        }
    }

    /// Recursively build symbol tables for a file and its dependencies.
    fn build_symbol_table(&self, path: &PathBuf, visited: Rc<RefCell<HashSet<PathBuf>>>) -> bool {
        // check for circular dependencies
        if visited.borrow().contains(path) {
            return false;
        }

        // mark file as being processed
        visited.borrow_mut().insert(path.clone());

        // get current symbol table
        let symbol_table = if let Some(table) = self.symbol_tables.get(path) {
            table
        } else {
            return true;
        };

        // process dependencies
        if let Some(dependencies) = self.file_dependencies.get(path) {
            for (dep_path, _) in dependencies {
                // recursively build dependency's symbol table
                if !self.build_symbol_table(dep_path, visited.clone()) {
                    continue;
                }

                // add dependency to current symbol table
                if let Some(dep_table) = self.symbol_tables.get(dep_path) {
                    symbol_table
                        .write()
                        .unwrap()
                        .add_dependency(dep_path.clone(), dep_table.clone());
                }
            }
        }

        true
    }

    /// Generate semantic tokens for a document.
    fn generate_semantic_tokens(&mut self) {
        let identifier_field_types: Vec<(PathBuf, Vec<&IdentifierNode>)> = self
            .find_identifier_field_types()
            .into_iter()
            .map(|(path, identifiers)| (path.clone(), identifiers))
            .collect();

        let mut new_tokens = HashMap::new();
        for (path, identifiers) in identifier_field_types {
            let tokens = self.convert_identifiers_to_semantic_tokens(identifiers);
            new_tokens.insert(path, tokens);
        }
        self.semantic_tokens = new_tokens;
    }

    /// Find all IdentifierNode instances used as field types in the document nodes.
    fn find_identifier_field_types(&self) -> HashMap<PathBuf, Vec<&IdentifierNode>> {
        let mut result = HashMap::new();

        for (path, document_node) in &self.document_nodes {
            for definition in &document_node.definitions {
                let definition = definition.as_ref().as_any();

                if let Some(struct_node) = definition.downcast_ref::<StructNode>() {
                    for field in &struct_node.fields {
                        self.collect_identifier_field_types(&field.field_type, &mut result, path);
                    }
                } else if let Some(union_node) = definition.downcast_ref::<UnionNode>() {
                    for field in &union_node.fields {
                        self.collect_identifier_field_types(&field.field_type, &mut result, path);
                    }
                } else if let Some(exception_node) = definition.downcast_ref::<ExceptionNode>() {
                    for field in &exception_node.fields {
                        self.collect_identifier_field_types(&field.field_type, &mut result, path);
                    }
                } else if let Some(service_node) = definition.downcast_ref::<ServiceNode>() {
                    for function in &service_node.functions {
                        self.collect_identifier_field_types(
                            &function.return_type,
                            &mut result,
                            path,
                        );
                        for param in &function.parameters {
                            self.collect_identifier_field_types(
                                &param.field_type,
                                &mut result,
                                path,
                            );
                        }
                        if let Some(throws) = &function.throws {
                            for throw in throws {
                                self.collect_identifier_field_types(
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
    fn collect_identifier_field_types<'a>(
        &'a self,
        field_type: &'a Box<dyn Node>,
        result: &mut HashMap<PathBuf, Vec<&'a IdentifierNode>>,
        path: &PathBuf,
    ) {
        let field_type = field_type.as_ref().as_any();
        if let Some(identifier) = field_type.downcast_ref::<IdentifierNode>() {
            result
                .entry(path.clone())
                .or_insert_with(Vec::new)
                .push(identifier);
        } else if let Some(list_type) = field_type.downcast_ref::<ListTypeNode>() {
            self.collect_identifier_field_types(&list_type.type_node, result, path);
        } else if let Some(set_type) = field_type.downcast_ref::<SetTypeNode>() {
            self.collect_identifier_field_types(&set_type.type_node, result, path);
        } else if let Some(map_type) = field_type.downcast_ref::<MapTypeNode>() {
            self.collect_identifier_field_types(&map_type.key_type, result, path);
            self.collect_identifier_field_types(&map_type.value_type, result, path);
        }
    }

    /// Convert a vector of IdentifierNode references to semantic tokens.
    fn convert_identifiers_to_semantic_tokens(
        &self,
        identifiers: Vec<&IdentifierNode>,
    ) -> Vec<u32> {
        let mut tokens = Vec::new();
        let mut prev_line = 0;
        let mut prev_char = 0;

        for identifier in identifiers {
            let range = lsp::Range::from(identifier.range());

            let line = range.start.line as u32;
            let char = range.start.character as u32;
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
            // tokenType: 0 for type (as defined in SemanticTokensLegend)
            // tokenModifiers: 0 for no modifiers
            tokens.extend_from_slice(&[delta_line, delta_start, length, 0, 0]);

            prev_line = line;
            prev_char = char;
        }

        tokens
    }
}
