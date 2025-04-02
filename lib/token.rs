use std::fmt;
use std::path::PathBuf;

#[derive(PartialEq, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub location: Location,
}

impl Token {
    // returns true if the token is an EOF token.
    pub fn is_eof(&self) -> bool {
        self.kind == TokenKind::Eof
    }

    // returns true if the token is an invalid token.
    pub fn is_invalid(&self) -> bool {
        match self.kind {
            TokenKind::Invalid(_) => true,
            TokenKind::InvalidString(_) => true,
            _ => false,
        }
    }

    // returns true if the token is a comment.
    pub fn is_comment(&self) -> bool {
        match self.kind {
            TokenKind::Comment(_) => true,
            TokenKind::BlockComment(_) => true,
            _ => false,
        }
    }

    // returns true if the token is a separator.
    pub fn is_line_separator(&self) -> bool {
        match self.kind {
            TokenKind::ListSeparator(_) => true,
            _ => false,
        }
    }

    // returns the range of the token.
    pub fn range(&self) -> Range {
        let mut end = self.location.clone();
        end.column += self.kind.len();
        Range {
            start: self.location.clone(),
            end,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.path.display(), self.line, self.column)
    }
}

#[derive(Debug, Clone)]
pub struct Range {
    pub start: Location,
    pub end: Location,
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start.path == self.end.path && self.start.line == self.end.line {
            write!(
                f,
                "{}:{}:{}-{}",
                self.start.path.display(),
                self.start.line,
                self.start.column,
                self.end.column
            )
        } else {
            write!(
                f,
                "{}:{}:{}-{}:{}:{}",
                self.start.path.display(),
                self.start.line,
                self.start.column,
                self.end.path.display(),
                self.end.line,
                self.end.column
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // keywords
    Include,    // include
    CppInclude, // cpp_include
    Namespace,  // namespace
    Const,      // const
    Typedef,    // typedef
    Enum,       // enum
    Struct,     // struct
    Union,      // union
    Exception,  // exception
    Service,    // service
    Required,   // required
    Optional,   // optional
    Oneway,     // oneway
    Void,       // void
    Throws,     // throws
    Extends,    // extends

    // types
    Map,     // map
    Set,     // set
    List,    // list
    CppType, // cpp_type

    // characters
    Assign,  // =
    Colon,   // :
    Less,    // <
    Greater, // >
    Lparen,  // (
    Rparen,  // )
    Lbrace,  // {
    Rbrace,  // }
    Lbrack,  // [
    Rbrack,  // ]

    // comments
    Comment(String),      // // comment
    BlockComment(String), // /* comment */

    // constants
    IntConstant(String),    // 123
    DoubleConstant(String), // 123.456e-78

    // multi-character tokens
    NamespaceScope(String), // cpp, go, java, etc.
    BaseType(String),       // bool, i16, i32, etc.
    Literal(String),        // literal
    Identifier(String),     // identifier
    ListSeparator(char),    // , | ;

    // invalid tokens
    Invalid(char),
    InvalidString(String),

    // end of file
    Eof,
}

impl TokenKind {
    pub fn len(&self) -> usize {
        match self {
            TokenKind::Include => 7,
            TokenKind::CppInclude => 11,
            TokenKind::Namespace => 9,
            TokenKind::Const => 5,
            TokenKind::Typedef => 7,
            TokenKind::Enum => 4,
            TokenKind::Struct => 6,
            TokenKind::Union => 5,
            TokenKind::Exception => 9,
            TokenKind::Service => 7,
            TokenKind::Required => 8,
            TokenKind::Optional => 8,
            TokenKind::Oneway => 6,
            TokenKind::Void => 4,
            TokenKind::Throws => 6,
            TokenKind::Extends => 7,
            TokenKind::Map => 3,
            TokenKind::Set => 3,
            TokenKind::List => 4,
            TokenKind::CppType => 8,
            TokenKind::Assign => 1,
            TokenKind::Colon => 1,
            TokenKind::Less => 1,
            TokenKind::Greater => 1,
            TokenKind::Lparen => 1,
            TokenKind::Rparen => 1,
            TokenKind::Lbrace => 1,
            TokenKind::Rbrace => 1,
            TokenKind::Lbrack => 1,
            TokenKind::Rbrack => 1,
            TokenKind::Comment(ref s) => s.len() + 2,
            TokenKind::BlockComment(ref s) => s.len() + 4,
            TokenKind::IntConstant(ref s) => s.len(),
            TokenKind::DoubleConstant(ref s) => s.len(),
            TokenKind::NamespaceScope(ref s) => s.len(),
            TokenKind::BaseType(ref s) => s.len(),
            TokenKind::Literal(ref s) => s.len(),
            TokenKind::Identifier(ref s) => s.len(),
            TokenKind::ListSeparator(_) => 1,
            TokenKind::Invalid(_) => 1,
            TokenKind::InvalidString(ref s) => s.len(),
            TokenKind::Eof => 0,
        }
    }
}

impl TokenKind {
    // create token from string.
    pub fn from_string(s: &str) -> Option<TokenKind> {
        let tok = match s {
            "include" => TokenKind::Include,
            "cpp_include" => TokenKind::CppInclude,
            "namespace" => TokenKind::Namespace,
            "const" => TokenKind::Const,
            "typedef" => TokenKind::Typedef,
            "enum" => TokenKind::Enum,
            "struct" => TokenKind::Struct,
            "union" => TokenKind::Union,
            "exception" => TokenKind::Exception,
            "service" => TokenKind::Service,
            "required" => TokenKind::Required,
            "optional" => TokenKind::Optional,
            "oneway" => TokenKind::Oneway,
            "void" => TokenKind::Void,
            "throws" => TokenKind::Throws,
            "extends" => TokenKind::Extends,
            "map" => TokenKind::Map,
            "set" => TokenKind::Set,
            "list" => TokenKind::List,
            "cpp_type" => TokenKind::CppType,

            // namespace scopes
            "c_glib" => TokenKind::NamespaceScope(String::from("c_glib")),
            "cpp" => TokenKind::NamespaceScope(String::from("cpp")),
            "delphi" => TokenKind::NamespaceScope(String::from("delphi")),
            "haxe" => TokenKind::NamespaceScope(String::from("haxe")),
            "go" => TokenKind::NamespaceScope(String::from("go")),
            "java" => TokenKind::NamespaceScope(String::from("java")),
            "js" => TokenKind::NamespaceScope(String::from("js")),
            "lua" => TokenKind::NamespaceScope(String::from("lua")),
            "netstd" => TokenKind::NamespaceScope(String::from("netstd")),
            "perl" => TokenKind::NamespaceScope(String::from("perl")),
            "php" => TokenKind::NamespaceScope(String::from("php")),
            "py" => TokenKind::NamespaceScope(String::from("py")),
            "py.twisted" => TokenKind::NamespaceScope(String::from("py.twisted")),
            "rb" => TokenKind::NamespaceScope(String::from("rb")),
            "st" => TokenKind::NamespaceScope(String::from("st")),
            "xsd" => TokenKind::NamespaceScope(String::from("xsd")),

            // base types
            "bool" => TokenKind::BaseType(String::from("bool")),
            "byte" => TokenKind::BaseType(String::from("byte")),
            "i8" => TokenKind::BaseType(String::from("i8")),
            "i16" => TokenKind::BaseType(String::from("i16")),
            "i32" => TokenKind::BaseType(String::from("i32")),
            "i64" => TokenKind::BaseType(String::from("i64")),
            "double" => TokenKind::BaseType(String::from("double")),
            "string" => TokenKind::BaseType(String::from("string")),
            "binary" => TokenKind::BaseType(String::from("binary")),
            "uuid" => TokenKind::BaseType(String::from("uuid")),

            _ => return None,
        };

        Some(tok)
    }

    // create token from char.
    pub fn from_char(c: char) -> Option<TokenKind> {
        let tok = match c {
            '=' => TokenKind::Assign,
            ':' => TokenKind::Colon,
            '<' => TokenKind::Less,
            '>' => TokenKind::Greater,
            ',' => TokenKind::ListSeparator(','),
            ';' => TokenKind::ListSeparator(';'),
            '(' => TokenKind::Lparen,
            ')' => TokenKind::Rparen,
            '{' => TokenKind::Lbrace,
            '}' => TokenKind::Rbrace,
            '[' => TokenKind::Lbrack,
            ']' => TokenKind::Rbrack,
            '*' => TokenKind::NamespaceScope(String::from("*")),
            _ => return None,
        };

        Some(tok)
    }
}
