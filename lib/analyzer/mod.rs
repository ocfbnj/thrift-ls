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
};

use crate::{
    analyzer::{
        ast::{
            DocumentNode, ExceptionNode, IdentifierNode, IncludeNode, ListTypeNode, MapTypeNode,
            Node, ServiceNode, SetTypeNode, StructNode, UnionNode,
        },
        base::Error,
        parser::Parser,
        symbol::SymbolTable,
    },
    lsp::{self, Diagnostic},
};

/// Analyzer for Thrift files.
pub struct Analyzer {
    documents: HashMap<PathBuf, Vec<char>>,

    document_nodes: HashMap<PathBuf, DocumentNode>,
    symbol_tables: HashMap<PathBuf, Rc<RefCell<SymbolTable>>>,
    file_dependencies: HashMap<PathBuf, Vec<(PathBuf, IncludeNode)>>,

    diagnostics: HashMap<PathBuf, Vec<Diagnostic>>,
    semantic_tokens: HashMap<PathBuf, Vec<u32>>,
}

impl Analyzer {
    /// Create a new analyzer.
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            document_nodes: HashMap::new(),
            symbol_tables: HashMap::new(),
            file_dependencies: HashMap::new(),
            diagnostics: HashMap::new(),
            semantic_tokens: HashMap::new(),
        }
    }

    /// Sync a document.
    pub fn sync_document(&mut self, path: PathBuf, content: &str) {
        self.documents.insert(path, content.chars().collect());
        self.analyze();
    }

    /// Get the diagnostics for all files.
    pub fn diagnostics(&self) -> &HashMap<PathBuf, Vec<Diagnostic>> {
        &self.diagnostics
    }

    /// Get semantic tokens for a specific file.
    pub fn semantic_tokens(&self, path: &PathBuf) -> Option<&Vec<u32>> {
        self.semantic_tokens.get(path)
    }
}

impl Analyzer {
    /// Analyze all documents.
    fn analyze(&mut self) {
        // clear previous state
        self.document_nodes.clear();
        self.symbol_tables.clear();
        self.file_dependencies.clear();
        self.diagnostics.clear();
        self.semantic_tokens.clear();

        // parse all files
        self.documents
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .iter()
            .for_each(|path| {
                self.parse_file(path, Rc::new(RefCell::new(HashSet::new())), None);
            });

        self.type_checking();
        self.generate_semantic_tokens();

        log::debug!("symbol tables: {:#?}", self.symbol_tables);
    }

    /// Recursively parse AST and build symbol tables for a file.
    fn parse_file(
        &mut self,
        path: &PathBuf,
        visited: Rc<RefCell<HashSet<PathBuf>>>,
        source: Option<(&PathBuf, &IncludeNode)>,
    ) -> bool {
        // check for circular dependencies
        if visited.borrow().contains(path) {
            if let Some((source_path, node)) = source {
                let error = Error {
                    range: node.range(),
                    message: format!("Circular dependency detected: {}", path.display()),
                };

                self.diagnostics
                    .entry(source_path.clone())
                    .or_default()
                    .push(error.into());
            }
            return false;
        }

        // mark file as being processed
        visited.borrow_mut().insert(path.clone());

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
                            message: format!("Failed to read file {}: {}", path.display(), e),
                        };

                        self.diagnostics
                            .entry(source_path.clone())
                            .or_default()
                            .push(error.into());
                    }
                    return false;
                }
            }
        };

        // parse the file
        let (document_node, errors) = Parser::new(content).parse();

        // store parser errors
        self.diagnostics
            .entry(path.clone())
            .or_default()
            .extend(errors.into_iter().map(|e| e.into()));

        // track file dependencies
        let mut dependencies = Vec::new();
        for header in &document_node.headers {
            let header = header.as_ref().as_any();
            if let Some(include) = header.downcast_ref::<IncludeNode>() {
                if let Some(parent) = path.parent() {
                    dependencies.push((parent.join(&include.literal), include));
                }
            }
        }

        // build symbol table
        let symbol_table = Rc::new(RefCell::new(SymbolTable::new_from_ast(&document_node)));

        // recursively parse dependencies
        for (dep_path, include_node) in dependencies.iter() {
            let res = self.parse_file(dep_path, visited.clone(), Some((path, include_node)));
            visited.borrow_mut().remove(dep_path);
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
        self.symbol_tables.insert(path.clone(), symbol_table);
        self.document_nodes.insert(path.clone(), document_node);

        true
    }

    /// Generate semantic tokens for a document.
    fn generate_semantic_tokens(&mut self) {
        let field_type_identifiers = self.find_field_type_identifiers();
        let function_identifiers = self.find_function_identifiers();

        let mut identifiers: HashMap<PathBuf, Vec<(&IdentifierNode, u32)>> = HashMap::new();
        for (path, ids) in field_type_identifiers {
            identifiers
                .entry(path)
                .or_default()
                .extend(ids.into_iter().map(|x| (x, 0)));
        }
        for (path, ids) in function_identifiers {
            identifiers
                .entry(path)
                .or_default()
                .extend(ids.into_iter().map(|x| (x, 1)));
        }

        let mut new_tokens = HashMap::new();
        for (path, ids) in identifiers {
            new_tokens.insert(path, self.convert_identifiers_to_semantic_tokens(ids));
        }

        self.semantic_tokens = new_tokens;
    }

    /// Find all IdentifierNode instances used as field types in the document nodes.
    fn find_field_type_identifiers(&self) -> HashMap<PathBuf, Vec<&IdentifierNode>> {
        let mut result = HashMap::new();

        for (path, document_node) in &self.document_nodes {
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
            // tokenType: 0 for type, 1 for function (as defined in SemanticTokensLegend)
            // tokenModifiers: 0 for no modifiers
            tokens.extend_from_slice(&[delta_line, delta_start, length, token_type, 0]);

            prev_line = line;
            prev_char = char;
        }

        tokens
    }

    /// Find all function identifiers in the document nodes.
    fn find_function_identifiers(&self) -> HashMap<PathBuf, Vec<&IdentifierNode>> {
        let mut result = HashMap::new();

        for (path, document_node) in &self.document_nodes {
            for definition in &document_node.definitions {
                let definition = definition.as_ref().as_any();

                if let Some(service_node) = definition.downcast_ref::<ServiceNode>() {
                    for function in &service_node.functions {
                        result
                            .entry(path.clone())
                            .or_insert_with(Vec::new)
                            .push(&function.identifier);
                    }
                }
            }
        }

        result
    }

    /// type checking
    fn type_checking(&mut self) {
        for (path, document_node) in self.document_nodes.iter() {
            if let Some(symbol_table) = self.symbol_tables.get_mut(path) {
                symbol_table
                    .borrow_mut()
                    .check_document_types(document_node);

                self.diagnostics.entry(path.clone()).or_default().extend(
                    symbol_table
                        .borrow()
                        .errors()
                        .iter()
                        .map(|e| e.clone().into()),
                );
            }
        }
    }
}
