use std::{cell::RefCell, collections::HashMap, path::Path, rc::Rc};

use crate::analyzer::{
    ast::{DefinitionNode, DocumentNode, FieldTypeNode, HeaderNode, IdentifierNode, Node},
    base::Error,
};

/// Symbol table for a single file.
#[derive(Debug)]
pub struct SymbolTable {
    path: String,
    types: HashMap<String, Rc<DefinitionNode>>,
    include_nodes: HashMap<String, Rc<HeaderNode>>,
    includes: HashMap<String, Rc<SymbolTable>>,
    namespace_to_path: HashMap<String, String>,
    errors: RefCell<Vec<Error>>,
}

impl SymbolTable {
    /// Create a new empty symbol table.
    pub fn new() -> Self {
        Self {
            path: String::new(),
            types: HashMap::new(),
            includes: HashMap::new(),
            include_nodes: HashMap::new(),
            namespace_to_path: HashMap::new(),
            errors: RefCell::new(Vec::new()),
        }
    }

    /// Create a new symbol table from an AST.
    pub fn new_from_ast(path: &str, document: &DocumentNode) -> Self {
        let mut table = Self::new();
        table.path = path.to_string();
        document.definitions.iter().for_each(|definition| {
            table.process_definition(definition);
        });
        table
    }

    /// Add a dependency to the symbol table.
    pub fn add_dependency(
        &mut self,
        path: &str,
        node: Rc<HeaderNode>,
        dependency: Rc<SymbolTable>,
    ) {
        let namespace = Path::new(path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_string();

        if let HeaderNode::Include(_) = node.as_ref() {
            self.include_nodes.insert(namespace.clone(), node);
        }

        self.includes.insert(namespace.clone(), dependency);
        self.namespace_to_path.insert(namespace, path.to_string());
    }

    /// Get the types in the symbol table.
    pub fn types(&self) -> &HashMap<String, Rc<DefinitionNode>> {
        &self.types
    }

    /// Get the includes in the symbol table.
    pub fn includes(&self) -> &HashMap<String, Rc<SymbolTable>> {
        &self.includes
    }

    /// Get the errors.
    pub fn errors(&self) -> Vec<Error> {
        self.errors.borrow().clone()
    }

    /// Check the types of the document.
    pub fn check_document_types(&self, document: &DocumentNode) {
        for definition in &document.definitions {
            match definition.as_ref() {
                DefinitionNode::Const(const_def) => {
                    self.check_field_type(&const_def.field_type);
                }
                DefinitionNode::Struct(struct_def) => {
                    for field in &struct_def.fields {
                        self.check_field_type(&field.field_type);
                    }
                }
                DefinitionNode::Union(union_def) => {
                    for field in &union_def.fields {
                        self.check_field_type(&field.field_type);
                    }
                }
                DefinitionNode::Exception(exception_def) => {
                    for field in &exception_def.fields {
                        self.check_field_type(&field.field_type);
                    }
                }
                DefinitionNode::Service(service_def) => {
                    if let Some(extends) = &service_def.extends {
                        self.check_identifier_type(extends);
                    }

                    for function in &service_def.functions {
                        if let Some(function_type) = &function.function_type {
                            self.check_field_type(function_type);
                        }

                        for field in &function.fields {
                            self.check_field_type(&field.field_type);
                        }

                        if let Some(throws) = &function.throws {
                            for throw in throws {
                                self.check_field_type(&throw.field_type);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Find a definition of an identifier type.
    pub fn find_definition_of_identifier_type(
        &self,
        identifier: &IdentifierNode,
    ) -> Option<(String, Rc<DefinitionNode>, Option<Rc<HeaderNode>>)> {
        // check if the identifier contains a namespace (file name)
        if let (Some(namespace), identifier) = identifier.split_by_first_dot() {
            // look up in included files
            let included_table = self.includes.get(&namespace.name)?;
            let (path, def, _) = included_table.find_definition_of_identifier_type(&identifier)?;

            // get the header node
            let header = self.include_nodes.get(&namespace.name)?.clone();
            return Some((path, def, Some(header)));
        }

        // look up type in current symbol table
        Some((
            self.path.clone(),
            self.types.get(&identifier.name)?.clone(),
            None,
        ))
    }
}

impl SymbolTable {
    fn process_definition(&mut self, definition: &Rc<DefinitionNode>) {
        if self.types.contains_key(definition.name()) {
            self.errors.borrow_mut().push(Error {
                range: definition.range(),
                message: format!("Duplicate definition: {}", definition.name()),
            });
            return;
        }

        self.types
            .insert(definition.name().to_string(), definition.clone());
    }

    fn check_field_type(&self, field_type: &FieldTypeNode) {
        match field_type {
            FieldTypeNode::Identifier(identifier) => {
                self.check_identifier_type(identifier);
            }
            FieldTypeNode::BaseType(_) => {
                // base types are always valid
            }
            FieldTypeNode::MapType(map_type) => {
                self.check_field_type(&*map_type.key_type);
                self.check_field_type(&*map_type.value_type);
            }
            FieldTypeNode::SetType(set_type) => {
                self.check_field_type(&*set_type.type_node);
            }
            FieldTypeNode::ListType(list_type) => {
                self.check_field_type(&*list_type.type_node);
            }
        }
    }

    fn check_identifier_type(&self, identifier: &IdentifierNode) {
        let def = self.find_definition_of_identifier_type(identifier);
        if def.is_none() {
            self.errors.borrow_mut().push(Error {
                range: identifier.range(),
                message: format!("Undefined type: {}", identifier.name),
            });
        }
    }
}
