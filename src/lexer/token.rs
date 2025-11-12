use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Function,
    Let,
    If,
    Else,
    While,
    For,
    In,
    Return,
    Export,
    Import,
    From,
    Stack,
    Heap,
    Defer,
    Watch,
    Async,
    Shared,
    Match,
    Switch,
    Case,
    Default,
    Break,
    Continue,
    And,
    Or,
    Not,
    Nothing,
    Unknown,
    Requires,
    Ensures,
    When,

    // Literals
    IntegerLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BooleanLiteral(bool),

    // Identifiers
    Identifier(String),

    // Operators
    Plus,           // +
    Minus,          // -
    Star,           // *
    Slash,          // /
    Percent,        // %

    // Assignment
    Equal,          // =
    PlusEqual,      // +=
    MinusEqual,     // -=
    StarEqual,      // *=
    SlashEqual,     // /=
    PercentEqual,   // %=

    // Comparison
    EqualQuestion,      // ?=  (with coercion)
    EqualQuestionDouble, // ??= (strict)
    BangEqual,          // !=  (with coercion)
    BangEqualDouble,    // !!= (strict)
    Less,               // <
    Greater,            // >
    LessEqual,          // <=
    GreaterEqual,       // >=

    // Bitwise
    Ampersand,      // &
    Pipe,           // |
    Caret,          // ^
    Tilde,          // ~
    ShiftLeft,      // <<
    ShiftRight,     // >>

    // Delimiters
    LeftParen,      // (
    RightParen,     // )
    LeftBrace,      // {
    RightBrace,     // }
    LeftBracket,    // [
    RightBracket,   // ]
    Semicolon,      // ;
    Colon,          // :
    Comma,          // ,
    Dot,            // .
    DotDot,         // ..
    Arrow,          // =>

    // Special
    Eof,
    Error(String),
}

impl Token {
    pub fn new(kind: TokenKind, lexeme: String, line: usize, column: usize) -> Self {
        Token { kind, lexeme, line, column }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} '{}' at {}:{}", self.kind, self.lexeme, self.line, self.column)
    }
}

pub fn keyword(s: &str) -> Option<TokenKind> {
    match s {
        "function" => Some(TokenKind::Function),
        "let" => Some(TokenKind::Let),
        "if" => Some(TokenKind::If),
        "else" => Some(TokenKind::Else),
        "while" => Some(TokenKind::While),
        "for" => Some(TokenKind::For),
        "in" => Some(TokenKind::In),
        "return" => Some(TokenKind::Return),
        "export" => Some(TokenKind::Export),
        "import" => Some(TokenKind::Import),
        "from" => Some(TokenKind::From),
        "stack" => Some(TokenKind::Stack),
        "heap" => Some(TokenKind::Heap),
        "defer" => Some(TokenKind::Defer),
        "watch" => Some(TokenKind::Watch),
        "async" => Some(TokenKind::Async),
        "shared" => Some(TokenKind::Shared),
        "match" => Some(TokenKind::Match),
        "switch" => Some(TokenKind::Switch),
        "case" => Some(TokenKind::Case),
        "default" => Some(TokenKind::Default),
        "break" => Some(TokenKind::Break),
        "continue" => Some(TokenKind::Continue),
        "and" => Some(TokenKind::And),
        "or" => Some(TokenKind::Or),
        "not" => Some(TokenKind::Not),
        "nothing" => Some(TokenKind::Nothing),
        "unknown" => Some(TokenKind::Unknown),
        "requires" => Some(TokenKind::Requires),
        "ensures" => Some(TokenKind::Ensures),
        "when" => Some(TokenKind::When),
        "true" => Some(TokenKind::BooleanLiteral(true)),
        "false" => Some(TokenKind::BooleanLiteral(false)),
        _ => None,
    }
}
