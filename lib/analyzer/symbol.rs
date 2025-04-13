use std::{cell::RefCell, collections::HashMap, path::Path, rc::Rc};

use crate::analyzer::{
    ast::{
        BaseTypeNode, DefinitionNode, DocumentNode, IdentifierNode, ListTypeNode, MapTypeNode,
        Node, SetTypeNode,
    },
    base::Error,
};

use super::ast::HeaderNode;

/// Symbol table for a single file.
#[derive(Debug)]
pub struct SymbolTable {
    path: String,
    types: HashMap<String, Rc<DefinitionNode>>,
    include_nodes: HashMap<String, Rc<HeaderNode>>,
    includes: HashMap<String, Rc<RefCell<SymbolTable>>>,
    namespace_to_path: HashMap<String, String>,
    errors: Vec<Error>,
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
            errors: Vec::new(),
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
        dependency: Rc<RefCell<SymbolTable>>,
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
    pub fn includes(&self) -> &HashMap<String, Rc<RefCell<SymbolTable>>> {
        &self.includes
    }

    /// Get the errors.
    pub fn errors(&self) -> &[Error] {
        &self.errors
    }

    /// Check the types of the document.
    pub fn check_document_types(&mut self, document: &DocumentNode) {
        for definition in &document.definitions {
            match definition.as_ref() {
                DefinitionNode::Struct(struct_def) => {
                    for field in &struct_def.fields {
                        self.check_field_type(&*field.field_type);
                    }
                }
                DefinitionNode::Union(union_def) => {
                    for field in &union_def.fields {
                        self.check_field_type(&*field.field_type);
                    }
                }
                DefinitionNode::Exception(exception_def) => {
                    for field in &exception_def.fields {
                        self.check_field_type(&*field.field_type);
                    }
                }
                DefinitionNode::Service(service_def) => {
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
            let included_table = self.includes.get(&namespace.name)?.borrow();
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
        // skip const definitions
        if let DefinitionNode::Const(_) = definition.as_ref() {
            return;
        }

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
                .find_definition_of_identifier_type(identifier)
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
