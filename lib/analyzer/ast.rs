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
    pub range: Range,
    pub headers: Vec<Box<dyn Node>>,
    pub definitions: Vec<Box<dyn DefinitionNode>>,
}

#[derive(Debug, Clone)]
pub struct IncludeNode {
    pub range: Range,
    pub literal: String,
}

#[derive(Debug, Clone)]
pub struct CppIncludeNode {
    pub range: Range,
    pub literal: String,
}

#[derive(Debug, Clone)]
pub struct NamespaceNode {
    pub range: Range,
    pub scope: String,
    pub identifier: IdentifierNode,
    pub ext: Option<ExtNode>,
}

#[derive(Debug, Clone)]
pub struct IdentifierNode {
    pub range: Range,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ConstNode {
    pub range: Range,
    pub field_type: Box<dyn Node>,
    pub identifier: IdentifierNode,
    pub value: Box<dyn Node>,
}

#[derive(Debug, Clone)]
pub struct BaseTypeNode {
    pub range: Range,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct MapTypeNode {
    pub range: Range,
    pub cpp_type: Option<String>,
    pub key_type: Box<dyn Node>,
    pub value_type: Box<dyn Node>,
}

#[derive(Debug, Clone)]
pub struct SetTypeNode {
    pub range: Range,
    pub cpp_type: Option<String>,
    pub type_node: Box<dyn Node>,
}

#[derive(Debug, Clone)]
pub struct ListTypeNode {
    pub range: Range,
    pub cpp_type: Option<String>,
    pub type_node: Box<dyn Node>,
}

#[derive(Debug, Clone)]
pub struct ConstValueNode {
    pub range: Range,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct TypedefNode {
    pub range: Range,
    pub definition_type: Box<dyn Node>,
    pub identifier: IdentifierNode,
}

#[derive(Debug, Clone)]
pub struct EnumNode {
    pub range: Range,
    pub identifier: IdentifierNode,
    pub values: Vec<EnumValueNode>,
}

#[derive(Debug, Clone)]
pub struct EnumValueNode {
    pub range: Range,
    pub name: String,
    pub value: Option<i32>,
    pub ext: Option<ExtNode>,
}

#[derive(Debug, Clone)]
pub struct StructNode {
    pub range: Range,
    pub identifier: IdentifierNode,
    pub fields: Vec<FieldNode>,
    pub ext: Option<ExtNode>,
}

#[derive(Debug, Clone)]
pub struct FieldNode {
    pub range: Range,
    pub field_id: Option<i32>,
    pub field_req: Option<String>,
    pub field_type: Box<dyn Node>,
    pub identifier: IdentifierNode,
    pub default_value: Option<ConstValueNode>,
    pub ext: Option<ExtNode>,
}

#[derive(Debug, Clone)]
pub struct UnionNode {
    pub range: Range,
    pub identifier: IdentifierNode,
    pub fields: Vec<FieldNode>,
}

#[derive(Debug, Clone)]
pub struct ExceptionNode {
    pub range: Range,
    pub identifier: IdentifierNode,
    pub fields: Vec<FieldNode>,
}

#[derive(Debug, Clone)]
pub struct ServiceNode {
    pub range: Range,
    pub identifier: IdentifierNode,
    pub extends: Option<String>,
    pub functions: Vec<FunctionNode>,
}

#[derive(Debug, Clone)]
pub struct FunctionNode {
    pub range: Range,
    pub is_oneway: bool,
    pub function_type: Box<dyn Node>,
    pub identifier: IdentifierNode,
    pub fields: Vec<FieldNode>,
    pub throws: Option<Vec<FieldNode>>,
    pub ext: Option<ExtNode>,
}

#[derive(Debug, Clone)]
pub struct ExtNode {
    pub range: Range,
    pub kv_pairs: Vec<(String, String)>,
}

impl_node!(
    DocumentNode,
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
    FunctionNode,
    ExtNode
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
