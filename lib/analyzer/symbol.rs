use std::{cell::RefCell, collections::HashMap, path::PathBuf, rc::Rc};

use crate::analyzer::{
    ast::{
        BaseTypeNode, DocumentNode, ExceptionNode, IdentifierNode, ListTypeNode, MapTypeNode, Node,
        ServiceNode, SetTypeNode, StructNode, UnionNode,
    },
    base::Error,
};

use super::ast::DefinitionNode;

/// Symbol table for a single file.
#[derive(Debug)]
pub struct SymbolTable {
    types: HashMap<String, Rc<dyn DefinitionNode>>,
    includes: HashMap<String, Rc<RefCell<SymbolTable>>>,
    path: PathBuf,
    namespace_to_path: HashMap<String, PathBuf>,
    errors: Vec<Error>,
}

impl SymbolTable {
    /// Create a new empty symbol table.
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
            includes: HashMap::new(),
            path: PathBuf::new(),
            namespace_to_path: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// Create a new symbol table from an AST.
    pub fn new_from_ast(path: PathBuf, document: &DocumentNode) -> Self {
        let mut table = Self::new();
        table.path = path;
        document.definitions.iter().for_each(|definition| {
            table.process_definition(definition);
        });
        table
    }

    /// Add a dependency to the symbol table.
    pub fn add_dependency(&mut self, path: &PathBuf, dependency: Rc<RefCell<SymbolTable>>) {
        let namespace = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_string();

        self.includes.insert(namespace.clone(), dependency);
        self.namespace_to_path.insert(namespace, path.clone());
    }

    /// Get the errors.
    pub fn errors(&self) -> &[Error] {
        &self.errors
    }

    /// Check the types of the document.
    pub fn check_document_types(&mut self, document: &DocumentNode) {
        for definition in &document.definitions {
            let definition = definition.as_ref();
            let definition = definition.as_any();

            if let Some(struct_def) = definition.downcast_ref::<StructNode>() {
                for field in &struct_def.fields {
                    self.check_field_type(&*field.field_type);
                }
            } else if let Some(union_def) = definition.downcast_ref::<UnionNode>() {
                for field in &union_def.fields {
                    self.check_field_type(&*field.field_type);
                }
            } else if let Some(exception_def) = definition.downcast_ref::<ExceptionNode>() {
                for field in &exception_def.fields {
                    self.check_field_type(&*field.field_type);
                }
            } else if let Some(service_def) = definition.downcast_ref::<ServiceNode>() {
                for function in &service_def.functions {
                    self.check_field_type(&*function.function_type);

                    for field in &function.fields {
                        self.check_field_type(&*field.field_type);
                    }

                    if let Some(throws) = &function.throws {
                        for throw in throws {
                            self.check_field_type(&*throw.field_type);
                        }
                    }
                }
            }
        }
    }

    /// Find a definition of an identifier type.
    pub fn find_definition_of_identifier_type(
        &self,
        path: &PathBuf,
        identifier: &IdentifierNode,
    ) -> Option<(PathBuf, Rc<dyn DefinitionNode>)> {
        // check if the identifier contains a namespace (file name)
        if let Some((namespace, type_name)) = identifier.name.split_once('.') {
            // look up in included files
            let included_table = self.includes.get(namespace)?;
            // create a new identifier with just the type name
            let type_identifier = IdentifierNode {
                range: identifier.range(),
                name: type_name.to_string(),
            };
            // get the path of the included file
            let path = self.namespace_to_path.get(namespace)?;

            return included_table
                .borrow()
                .find_definition_of_identifier_type(&path, &type_identifier);
        }

        // look up type in current symbol table
        Some((path.clone(), self.types.get(&identifier.name)?.clone()))
    }
}

impl SymbolTable {
    fn process_definition(&mut self, definition: &Rc<dyn DefinitionNode>) {
        if self.types.contains_key(definition.name()) {
            self.errors.push(Error {
                range: definition.range(),
                message: format!("Duplicate definition with name: {}", definition.name()),
            });
            return;
        }

        self.types
            .insert(definition.name().to_string(), definition.clone());
    }

    fn check_field_type(&mut self, field_type: &dyn Node) {
        if let Some(_) = field_type.as_any().downcast_ref::<BaseTypeNode>() {
            // base types are always valid
        } else if let Some(identifier) = field_type.as_any().downcast_ref::<IdentifierNode>() {
            if self
                .find_definition_of_identifier_type(&self.path, identifier)
                .is_none()
            {
                self.errors.push(Error {
                    range: identifier.range(),
                    message: format!("Undefined type: {}", identifier.name),
                });
            }
        } else if let Some(list_type) = field_type.as_any().downcast_ref::<ListTypeNode>() {
            self.check_field_type(&*list_type.type_node);
        } else if let Some(set_type) = field_type.as_any().downcast_ref::<SetTypeNode>() {
            self.check_field_type(&*set_type.type_node);
        } else if let Some(map_type) = field_type.as_any().downcast_ref::<MapTypeNode>() {
            self.check_field_type(&*map_type.key_type);
            self.check_field_type(&*map_type.value_type);
        } else {
            self.errors.push(Error {
                range: field_type.range(),
                message: "Invalid field type".to_string(),
            });
        }
    }
}
