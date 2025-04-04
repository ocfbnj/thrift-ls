use std::{collections::HashMap, path::PathBuf};

use crate::{
    ast::DocumentNode,
    parser::{ParseError, Parser},
    scanner::{EmptyInput, FileInput},
};

pub struct Analyzer {
    parser: Parser,
    documents: HashMap<PathBuf, Vec<char>>,
    document_nodes: HashMap<PathBuf, DocumentNode>,
    errors: HashMap<PathBuf, Vec<ParseError>>,
}

impl Analyzer {
    pub fn new() -> Self {
        Self {
            parser: Parser::new(EmptyInput),
            documents: HashMap::new(),
            document_nodes: HashMap::new(),
            errors: HashMap::new(),
        }
    }

    pub fn add_document(&mut self, path: PathBuf, content: &str) {
        self.documents.insert(path, content.chars().collect());
    }

    pub fn analyze(&mut self) {
        for (path, content) in self.documents.iter_mut() {
            let file_input = FileInput::new_with_content(path, content);
            self.parser.reset(file_input);
            let document_node = self.parser.parse();
            self.document_nodes.insert(path.clone(), document_node);
            self.errors
                .insert(path.clone(), self.parser.errors().to_vec());
        }
    }

    pub fn errors(&self) -> &HashMap<PathBuf, Vec<ParseError>> {
        &self.errors
    }
}
