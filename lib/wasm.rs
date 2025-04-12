//! WASM bindings for the analyzer.
//!
//! Build: `wasm-pack build --target nodejs`

use std::io;

use js_sys::Function;
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
        to_value(errors).unwrap_or_default()
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
            Some(loc) => to_value(&loc).unwrap_or_default(),
            None => JsValue::null(),
        }
    }

    pub fn types_for_completion(&self, path: &str, line: u32, column: u32) -> JsValue {
        let pos = analyzer::base::Position { line, column };
        let completions = self.analyzer.types_for_completion(path, pos);
        to_value(&completions).unwrap_or_default()
    }

    pub fn includes_for_completion(&self, path: &str, line: u32, column: u32) -> JsValue {
        let pos = analyzer::base::Position { line, column };
        let completions = self.analyzer.includes_for_completion(path, pos);
        to_value(&completions).unwrap_or_default()
    }

    pub fn set_wasm_read_file(&mut self, read_file: Function) {
        self.analyzer.wasm_read_file = Some(Box::new(move |path: String| -> io::Result<String> {
            let args = js_sys::Array::new();
            args.push(&path.into());
            let result = read_file.apply(&JsValue::null(), &args).unwrap_or_default();
            let content =
                js_sys::Reflect::get(&result, &JsValue::from_str("content")).unwrap_or_default();
            let error = js_sys::Reflect::get(&result, &JsValue::from_str("error"))
                .unwrap_or_default()
                .as_string()
                .unwrap_or_default();
            if error.len() > 0 {
                return Err(io::Error::new(io::ErrorKind::Other, error));
            }

            let result = content.as_string().unwrap_or_default();
            Ok(result)
        }));
    }
}
