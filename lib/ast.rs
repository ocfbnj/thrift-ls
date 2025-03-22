use core::fmt;

pub trait Node: fmt::Debug {}
impl<T> Node for T where T: fmt::Debug {}

#[derive(Debug)]
pub struct DocumentNode {
    pub(crate) headers: Vec<Box<dyn Node>>,
    pub(crate) definitions: Vec<Box<dyn Node>>,
}

pub struct HeaderNode {}

#[derive(Debug)]
pub struct IncludeNode {
    pub(crate) literal: String,
}

#[derive(Debug)]
pub struct CppIncludeNode {
    pub(crate) literal: String,
}

#[derive(Debug)]
pub struct NamespaceNode {
    pub(crate) name: String,
    pub(crate) scope: String,
}

#[derive(Debug)]
pub struct ConstNode {
    pub(crate) field_type: Box<dyn Node>,
    pub(crate) name: String,
    pub(crate) value: Box<dyn Node>,
}

#[derive(Debug)]
pub struct IdentifierNode {
    pub(crate) name: String,
}

#[derive(Debug)]
pub struct BaseTypeNode {
    pub(crate) name: String,
}

#[derive(Debug)]
pub struct MapTypeNode {
    pub(crate) cpp_type: Option<String>,
    pub(crate) key_type: Box<dyn Node>,
    pub(crate) value_type: Box<dyn Node>,
}

#[derive(Debug)]
pub struct SetTypeNode {
    pub(crate) cpp_type: Option<String>,
    pub(crate) type_node: Box<dyn Node>,
}

#[derive(Debug)]
pub struct ListTypeNode {
    pub(crate) cpp_type: Option<String>,
    pub(crate) type_node: Box<dyn Node>,
}

#[derive(Debug)]
pub struct ConstValueNode {
    pub(crate) value: String,
}

#[derive(Debug)]
pub struct TypedefNode {
    pub(crate) definition_type: Box<dyn Node>,
    pub(crate) name: String,
}

#[derive(Debug)]
pub struct EnumNode {}

#[derive(Debug)]
pub struct StructNode {}

#[derive(Debug)]
pub struct UnionNode {}

#[derive(Debug)]
pub struct ExceptionNode {}

#[derive(Debug)]
pub struct ServiceNode {}
