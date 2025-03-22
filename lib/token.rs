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

    // return true if the token is a separator.
    pub fn is_line_separator(&self) -> bool {
        match self.kind {
            TokenKind::ListSeparator(_) => true,
            _ => false,
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct Location {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
}

#[derive(PartialEq, Debug)]
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
