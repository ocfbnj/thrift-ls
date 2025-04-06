use std::{any::Any, fmt::Debug};

use crate::{analyzer::base::Range, impl_definition_node, impl_node};

/// A trait for all AST nodes.
pub trait Node: Debug + Send + Sync + Any {
    /// Returns the node as a `dyn Any`.
    fn as_any(&self) -> &dyn Any;
    /// Clones the node.
    fn clone_box(&self) -> Box<dyn Node>;
    /// Returns the range of the node.
    fn range(&self) -> Range;
}

impl Clone for Box<dyn Node> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

/// A trait for all definition nodes.
pub trait DefinitionNode: Node + Send + Sync {
    /// Returns the name of the definition.
    fn name(&self) -> &str;
    /// Clones the definition node.
    fn clone_definition_box(&self) -> Box<dyn DefinitionNode>;
}

impl Clone for Box<dyn DefinitionNode> {
    fn clone(&self) -> Self {
        (**self).clone_definition_box()
    }
}

#[derive(Debug, Clone)]
pub struct DocumentNode {
    pub headers: Vec<Box<dyn Node>>,
    pub definitions: Vec<Box<dyn DefinitionNode>>,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct HeaderNode {
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct IncludeNode {
    pub literal: String,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct CppIncludeNode {
    pub literal: String,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct NamespaceNode {
    pub name: String,
    pub scope: String,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct ConstNode {
    pub field_type: Box<dyn Node>,
    pub name: String,
    pub value: Box<dyn Node>,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct IdentifierNode {
    pub name: String,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct BaseTypeNode {
    pub name: String,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct MapTypeNode {
    pub cpp_type: Option<String>,
    pub key_type: Box<dyn Node>,
    pub value_type: Box<dyn Node>,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct SetTypeNode {
    pub cpp_type: Option<String>,
    pub type_node: Box<dyn Node>,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct ListTypeNode {
    pub cpp_type: Option<String>,
    pub type_node: Box<dyn Node>,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct ConstValueNode {
    pub value: String,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct TypedefNode {
    pub definition_type: Box<dyn Node>,
    pub name: String,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct EnumNode {
    pub name: String,
    pub values: Vec<EnumValueNode>,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct EnumValueNode {
    pub name: String,
    pub value: Option<i32>,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct StructNode {
    pub name: String,
    pub fields: Vec<FieldNode>,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct FieldNode {
    pub field_id: Option<i32>,
    pub field_req: Option<String>,
    pub field_type: Box<dyn Node>,
    pub name: String,
    pub default_value: Option<ConstValueNode>,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct UnionNode {
    pub name: String,
    pub fields: Vec<FieldNode>,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct ExceptionNode {
    pub name: String,
    pub fields: Vec<FieldNode>,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct ServiceNode {
    pub name: String,
    pub extends: Option<String>,
    pub functions: Vec<FunctionNode>,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct FunctionNode {
    pub is_oneway: bool,
    pub return_type: Box<dyn Node>,
    pub name: String,
    pub parameters: Vec<FieldNode>,
    pub throws: Option<Vec<FieldNode>>,
    pub range: Range,
}

impl_node!(
    DocumentNode,
    HeaderNode,
    IncludeNode,
    CppIncludeNode,
    NamespaceNode,
    ConstNode,
    IdentifierNode,
    BaseTypeNode,
    MapTypeNode,
    SetTypeNode,
    ListTypeNode,
    ConstValueNode,
    TypedefNode,
    EnumNode,
    EnumValueNode,
    StructNode,
    FieldNode,
    UnionNode,
    ExceptionNode,
    ServiceNode,
    FunctionNode
);

impl_definition_node!(
    ConstNode,
    TypedefNode,
    EnumNode,
    StructNode,
    UnionNode,
    ExceptionNode,
    ServiceNode
);
