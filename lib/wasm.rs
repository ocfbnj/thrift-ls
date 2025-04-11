//! WASM bindings for the analyzer.
//!
//! Build: `wasm-pack build --target nodejs`

use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

use crate::analyzer;

#[wasm_bindgen]
pub struct Analyzer {
    analyzer: analyzer::Analyzer,
}

#[wasm_bindgen]
impl Analyzer {
    pub fn new() -> Analyzer {
        Analyzer {
            analyzer: analyzer::Analyzer::new(),
        }
    }

    pub fn sync_document(&mut self, path: &str, content: &str) {
        self.analyzer.sync_document(path, content);
    }

    pub fn remove_document(&mut self, path: &str) {
        self.analyzer.remove_document(path);
    }

    pub fn errors(&self) -> JsValue {
        let errors = self.analyzer.errors();
        to_value(errors).unwrap()
    }

    pub fn semantic_tokens(&self, path: &str) -> Option<Vec<u32>> {
        self.analyzer.semantic_tokens(path).cloned()
    }

    pub fn semantic_token_types(&self) -> Vec<String> {
        self.analyzer.semantic_token_types()
    }

    pub fn semantic_token_modifiers(&self) -> Vec<String> {
        self.analyzer.semantic_token_modifiers()
    }

    pub fn definition(&self, path: &str, line: u32, column: u32) -> JsValue {
        let pos = analyzer::base::Position { line, column };

        match self.analyzer.definition(path, pos) {
            Some(loc) => to_value(&loc).unwrap(),
            None => JsValue::null(),
        }
    }
}
