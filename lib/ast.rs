use std::fmt;

pub trait Node: fmt::Debug {}
impl<T> Node for T where T: fmt::Debug {}

#[derive(Debug)]
pub struct DocumentNode {
    pub headers: Vec<Box<dyn Node>>,
    pub definitions: Vec<Box<dyn Node>>,
}

#[derive(Debug)]
pub struct HeaderNode {}

#[derive(Debug)]
pub struct IncludeNode {
    pub literal: String,
}

#[derive(Debug)]
pub struct CppIncludeNode {
    pub literal: String,
}

#[derive(Debug)]
pub struct NamespaceNode {
    pub name: String,
    pub scope: String,
}

#[derive(Debug)]
pub struct ConstNode {
    pub field_type: Box<dyn Node>,
    pub name: String,
    pub value: Box<dyn Node>,
}

#[derive(Debug)]
pub struct IdentifierNode {
    pub name: String,
}

#[derive(Debug)]
pub struct BaseTypeNode {
    pub name: String,
}

#[derive(Debug)]
pub struct MapTypeNode {
    pub cpp_type: Option<String>,
    pub key_type: Box<dyn Node>,
    pub value_type: Box<dyn Node>,
}

#[derive(Debug)]
pub struct SetTypeNode {
    pub cpp_type: Option<String>,
    pub type_node: Box<dyn Node>,
}

#[derive(Debug)]
pub struct ListTypeNode {
    pub cpp_type: Option<String>,
    pub type_node: Box<dyn Node>,
}

#[derive(Debug)]
pub struct ConstValueNode {
    pub value: String,
}

#[derive(Debug)]
pub struct TypedefNode {
    pub definition_type: Box<dyn Node>,
    pub name: String,
}

#[derive(Debug)]
pub struct EnumNode {
    pub name: String,
    pub values: Vec<EnumValueNode>,
}

#[derive(Debug)]
pub struct EnumValueNode {
    pub name: String,
    pub value: Option<i32>,
}

#[derive(Debug)]
pub struct StructNode {
    pub name: String,
    pub fields: Vec<FieldNode>,
}

#[derive(Debug)]
pub struct FieldNode {
    pub field_id: Option<i32>,
    pub field_req: Option<String>,
    pub field_type: Box<dyn Node>,
    pub name: String,
    pub default_value: Option<ConstValueNode>,
}

#[derive(Debug)]
pub struct UnionNode {
    pub name: String,
    pub fields: Vec<FieldNode>,
}

#[derive(Debug)]
pub struct ExceptionNode {
    pub name: String,
    pub fields: Vec<FieldNode>,
}

#[derive(Debug)]
pub struct ServiceNode {
    pub name: String,
    pub extends: Option<String>,
    pub functions: Vec<FunctionNode>,
}

#[derive(Debug)]
pub struct FunctionNode {
    pub is_oneway: bool,
    pub return_type: Box<dyn Node>,
    pub name: String,
    pub parameters: Vec<FieldNode>,
    pub throws: Option<Vec<FieldNode>>,
}
