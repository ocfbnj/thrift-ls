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

use crate::analyzer::{
    ast::{DocumentNode, IncludeNode, Node},
    base::Error,
    parser::Parser,
    scanner::{EmptyInput, FileInput},
    symbol::SymbolTable,
};

/// Analyzer for Thrift files.
pub struct Analyzer {
    parser: Parser,
    documents: HashMap<PathBuf, Vec<char>>,

    document_nodes: HashMap<PathBuf, DocumentNode>,
    symbol_tables: HashMap<PathBuf, Arc<RwLock<SymbolTable>>>,
    errors: HashMap<PathBuf, Vec<Error>>,
    file_dependencies: HashMap<PathBuf, Vec<(PathBuf, IncludeNode)>>,
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
    }

    /// Get the errors.
    pub fn errors(&self) -> &HashMap<PathBuf, Vec<Error>> {
        &self.errors
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
}
