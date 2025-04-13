use std::{any::Any, fmt::Debug, ops::Deref, rc::Rc};

use crate::analyzer::base::Range;

use super::base::Position;

/// A trait for all AST nodes.
pub trait Node: Debug + Any {
    /// Returns the node as a `dyn Any`.
    fn as_any(&self) -> &dyn Any;
    /// Returns the range of the node.
    fn range(&self) -> Range;
    /// Returns the children of the node.
    fn children(&self) -> Vec<&dyn Node>;
}

/// An enum representing all possible header nodes.
#[derive(Debug)]
pub enum HeaderNode {
    Include(IncludeNode),
    CppInclude(CppIncludeNode),
    Namespace(NamespaceNode),
}

impl Deref for HeaderNode {
    type Target = dyn Node;

    fn deref(&self) -> &Self::Target {
        match self {
            HeaderNode::Include(node) => node,
            HeaderNode::CppInclude(node) => node,
            HeaderNode::Namespace(node) => node,
        }
    }
}

/// An enum representing all possible definition nodes.
#[derive(Debug)]
pub enum DefinitionNode {
    Const(ConstNode),
    Typedef(TypedefNode),
    Enum(EnumNode),
    Struct(StructNode),
    Union(UnionNode),
    Exception(ExceptionNode),
    Service(ServiceNode),
}

impl Deref for DefinitionNode {
    type Target = dyn Node;

    fn deref(&self) -> &Self::Target {
        match self {
            DefinitionNode::Const(node) => node,
            DefinitionNode::Typedef(node) => node,
            DefinitionNode::Enum(node) => node,
            DefinitionNode::Struct(node) => node,
            DefinitionNode::Union(node) => node,
            DefinitionNode::Exception(node) => node,
            DefinitionNode::Service(node) => node,
        }
    }
}

impl DefinitionNode {
    pub fn name(&self) -> &str {
        &self.identifier().name
    }

    pub fn identifier(&self) -> &IdentifierNode {
        match self {
            DefinitionNode::Const(node) => &node.identifier,
            DefinitionNode::Typedef(node) => &node.identifier,
            DefinitionNode::Enum(node) => &node.identifier,
            DefinitionNode::Struct(node) => &node.identifier,
            DefinitionNode::Union(node) => &node.identifier,
            DefinitionNode::Exception(node) => &node.identifier,
            DefinitionNode::Service(node) => &node.identifier,
        }
    }
}

#[derive(Debug)]
pub struct DocumentNode {
    pub range: Range,
    pub headers: Vec<Rc<HeaderNode>>,
    pub definitions: Vec<Rc<DefinitionNode>>,
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

#[derive(Debug, Clone)]
pub struct IdentifierNode {
    pub range: Range,
    pub name: String,
}

impl IdentifierNode {
    /// Check if the position is in the namespace.
    pub fn position_in_namespace(&self, pos: Position) -> bool {
        let dot_index = match self.name.find('.') {
            Some(index) => index,
            None => return false,
        };

        self.range.contains(pos)
            && pos.column >= self.range.start.column
            && pos.column <= self.range.start.column + dot_index as u32
    }

    /// Split the identifier by the first dot.
    pub fn split_by_first_dot(&self) -> (Option<IdentifierNode>, IdentifierNode) {
        let dot_index = match self.name.find('.') {
            Some(index) => index,
            None => return (None, self.clone()),
        };

        let namespace = IdentifierNode {
            range: Range {
                start: self.range.start,
                end: Position {
                    line: self.range.start.line,
                    column: self.range.start.column + dot_index as u32,
                },
            },
            name: self.name[..dot_index].to_string(),
        };
        let identifier = IdentifierNode {
            range: Range {
                start: Position {
                    line: self.range.start.line,
                    column: self.range.start.column + dot_index as u32,
                },
                end: self.range.end,
            },
            name: self.name[dot_index + 1..].to_string(),
        };

        (Some(namespace), identifier)
    }
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
    pub field_id: Option<FieldIdNode>,
    pub field_req: Option<String>,
    pub field_type: Box<dyn Node>,
    pub identifier: IdentifierNode,
    pub default_value: Option<ConstValueNode>,
    pub ext: Option<ExtNode>,
}

#[derive(Debug)]
pub struct FieldIdNode {
    pub range: Range,
    pub id: i32,
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
        children.extend(self.headers.iter().map(|h| h.as_ref().deref()));
        children.extend(self.definitions.iter().map(|d| d.as_ref().deref()));
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
        if let Some(field_id) = &self.field_id {
            children.push(field_id as &dyn Node);
        }
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

impl Node for FieldIdNode {
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
