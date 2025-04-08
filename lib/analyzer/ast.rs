use std::{any::Any, fmt::Debug, rc::Rc};

use crate::{analyzer::base::Range, impl_definition_node};

/// A trait for all AST nodes.
pub trait Node: Debug + Any {
    /// Returns the node as a `dyn Any`.
    fn as_any(&self) -> &dyn Any;
    /// Returns the range of the node.
    fn range(&self) -> Range;
    /// Returns the children of the node.
    fn children(&self) -> Vec<&dyn Node>;
}

/// A trait for all definition nodes.
pub trait DefinitionNode: Node {
    /// Returns the name of the definition.
    fn name(&self) -> &str;
    /// Returns the node as a `dyn Node`.
    fn as_node(&self) -> &dyn Node;
    /// Returns the identifier of the definition.
    fn identifier(&self) -> &IdentifierNode;
}

#[derive(Debug)]
pub struct DocumentNode {
    pub range: Range,
    pub headers: Vec<Box<dyn Node>>,
    pub definitions: Vec<Rc<dyn DefinitionNode>>,
}

#[derive(Debug)]
pub struct IncludeNode {
    pub range: Range,
    pub literal: String,
}

#[derive(Debug)]
pub struct CppIncludeNode {
    pub range: Range,
    pub literal: String,
}

#[derive(Debug)]
pub struct NamespaceNode {
    pub range: Range,
    pub scope: String,
    pub identifier: IdentifierNode,
    pub ext: Option<ExtNode>,
}

#[derive(Debug)]
pub struct IdentifierNode {
    pub range: Range,
    pub name: String,
}

#[derive(Debug)]
pub struct ConstNode {
    pub range: Range,
    pub field_type: Box<dyn Node>,
    pub identifier: IdentifierNode,
    pub value: Box<dyn Node>,
}

#[derive(Debug)]
pub struct BaseTypeNode {
    pub range: Range,
    pub name: String,
}

#[derive(Debug)]
pub struct MapTypeNode {
    pub range: Range,
    pub cpp_type: Option<String>,
    pub key_type: Box<dyn Node>,
    pub value_type: Box<dyn Node>,
}

#[derive(Debug)]
pub struct SetTypeNode {
    pub range: Range,
    pub cpp_type: Option<String>,
    pub type_node: Box<dyn Node>,
}

#[derive(Debug)]
pub struct ListTypeNode {
    pub range: Range,
    pub cpp_type: Option<String>,
    pub type_node: Box<dyn Node>,
}

#[derive(Debug)]
pub struct ConstValueNode {
    pub range: Range,
    pub value: String,
}

#[derive(Debug)]
pub struct TypedefNode {
    pub range: Range,
    pub definition_type: Box<dyn Node>,
    pub identifier: IdentifierNode,
}

#[derive(Debug)]
pub struct EnumNode {
    pub range: Range,
    pub identifier: IdentifierNode,
    pub values: Vec<EnumValueNode>,
}

#[derive(Debug)]
pub struct EnumValueNode {
    pub range: Range,
    pub name: String,
    pub value: Option<i32>,
    pub ext: Option<ExtNode>,
}

#[derive(Debug)]
pub struct StructNode {
    pub range: Range,
    pub identifier: IdentifierNode,
    pub fields: Vec<FieldNode>,
    pub ext: Option<ExtNode>,
}

#[derive(Debug)]
pub struct FieldNode {
    pub range: Range,
    pub field_id: Option<i32>,
    pub field_req: Option<String>,
    pub field_type: Box<dyn Node>,
    pub identifier: IdentifierNode,
    pub default_value: Option<ConstValueNode>,
    pub ext: Option<ExtNode>,
}

#[derive(Debug)]
pub struct UnionNode {
    pub range: Range,
    pub identifier: IdentifierNode,
    pub fields: Vec<FieldNode>,
}

#[derive(Debug)]
pub struct ExceptionNode {
    pub range: Range,
    pub identifier: IdentifierNode,
    pub fields: Vec<FieldNode>,
}

#[derive(Debug)]
pub struct ServiceNode {
    pub range: Range,
    pub identifier: IdentifierNode,
    pub extends: Option<String>,
    pub functions: Vec<FunctionNode>,
}

#[derive(Debug)]
pub struct FunctionNode {
    pub range: Range,
    pub is_oneway: bool,
    pub function_type: Box<dyn Node>,
    pub identifier: IdentifierNode,
    pub fields: Vec<FieldNode>,
    pub throws: Option<Vec<FieldNode>>,
    pub ext: Option<ExtNode>,
}

#[derive(Debug)]
pub struct ExtNode {
    pub range: Range,
    pub kv_pairs: Vec<(String, String)>,
}

impl Node for DocumentNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        let mut children = Vec::new();
        children.extend(self.headers.iter().map(|h| h.as_ref()));
        children.extend(self.definitions.iter().map(|d| d.as_node()));
        children
    }
}

impl Node for IncludeNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        Vec::new()
    }
}

impl Node for CppIncludeNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        Vec::new()
    }
}

impl Node for NamespaceNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        let mut children = Vec::new();
        children.push(&self.identifier as &dyn Node);
        if let Some(ext) = &self.ext {
            children.push(ext as &dyn Node);
        }
        children
    }
}

impl Node for IdentifierNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        Vec::new()
    }
}

impl Node for ConstNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        vec![
            self.field_type.as_ref(),
            &self.identifier as &dyn Node,
            self.value.as_ref(),
        ]
    }
}

impl Node for BaseTypeNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        Vec::new()
    }
}

impl Node for MapTypeNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        vec![self.key_type.as_ref(), self.value_type.as_ref()]
    }
}

impl Node for SetTypeNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        vec![self.type_node.as_ref()]
    }
}

impl Node for ListTypeNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        vec![self.type_node.as_ref()]
    }
}

impl Node for ConstValueNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        Vec::new()
    }
}

impl Node for TypedefNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        vec![self.definition_type.as_ref(), &self.identifier as &dyn Node]
    }
}

impl Node for EnumNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        let mut children = Vec::new();
        children.push(&self.identifier as &dyn Node);
        children.extend(self.values.iter().map(|v| v as &dyn Node));
        children
    }
}

impl Node for EnumValueNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        let mut children = Vec::new();
        if let Some(ext) = &self.ext {
            children.push(ext as &dyn Node);
        }
        children
    }
}

impl Node for StructNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        let mut children = Vec::new();
        children.push(&self.identifier as &dyn Node);
        children.extend(self.fields.iter().map(|f| f as &dyn Node));
        if let Some(ext) = &self.ext {
            children.push(ext as &dyn Node);
        }
        children
    }
}

impl Node for FieldNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        let mut children = Vec::new();
        children.push(self.field_type.as_ref());
        children.push(&self.identifier as &dyn Node);
        if let Some(default_value) = &self.default_value {
            children.push(default_value as &dyn Node);
        }
        if let Some(ext) = &self.ext {
            children.push(ext as &dyn Node);
        }
        children
    }
}

impl Node for UnionNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        let mut children = Vec::new();
        children.push(&self.identifier as &dyn Node);
        children.extend(self.fields.iter().map(|f| f as &dyn Node));
        children
    }
}

impl Node for ExceptionNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        let mut children = Vec::new();
        children.push(&self.identifier as &dyn Node);
        children.extend(self.fields.iter().map(|f| f as &dyn Node));
        children
    }
}

impl Node for ServiceNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        let mut children = Vec::new();
        children.push(&self.identifier as &dyn Node);
        children.extend(self.functions.iter().map(|f| f as &dyn Node));
        children
    }
}

impl Node for FunctionNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        let mut children = Vec::new();
        children.push(self.function_type.as_ref());
        children.push(&self.identifier as &dyn Node);
        children.extend(self.fields.iter().map(|f| f as &dyn Node));
        if let Some(throws) = &self.throws {
            children.extend(throws.iter().map(|f| f as &dyn Node));
        }
        if let Some(ext) = &self.ext {
            children.push(ext as &dyn Node);
        }
        children
    }
}

impl Node for ExtNode {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn range(&self) -> Range {
        self.range.clone()
    }

    fn children(&self) -> Vec<&dyn Node> {
        Vec::new()
    }
}

impl_definition_node!(
    ConstNode,
    TypedefNode,
    EnumNode,
    StructNode,
    UnionNode,
    ExceptionNode,
    ServiceNode
);
