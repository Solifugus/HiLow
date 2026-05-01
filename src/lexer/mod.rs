use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub position: Position,
    pub lexeme: String,
}

impl Token {
    pub fn new(kind: TokenKind, position: Position, lexeme: String) -> Self {
        Self {
            kind,
            position,
            lexeme,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Integer(i64),
    Float(f64),
    True,
    False,

    // Identifiers and keywords
    Identifier,

    // Keywords (Phase 1a includes all keywords from the design spec)
    And,
    Arena,
    Async,
    Break,
    Case,
    Continue,
    Default,
    Defer,
    Else,
    Ensures,
    Export,
    For,
    Function,
    Heap,
    High,
    If,
    Import,
    In,
    Is,
    Let,
    Loop,
    Low,
    Manual,
    Match,
    Module,
    Not,
    Nothing,
    Or,
    Program,
    Requires,
    Return,
    Shared,
    Stack,
    Switch,
    This,
    Unknown,
    When,
    While,
    Watch,

    // Single-character punctuation
    LeftParen,    // (
    RightParen,   // )
    LeftBrace,    // {
    RightBrace,   // }
    LeftBracket,  // [
    RightBracket, // ]
    Comma,        // ,
    Semicolon,    // ;
    Colon,        // :
    Dot,          // .
    Question,     // ?
    At,           // @

    // Operators
    Plus,         // +
    Minus,        // -
    Star,         // *
    Slash,        // /
    Percent,      // %
    Less,         // <
    Greater,      // >
    LessEqual,    // <=
    GreaterEqual, // >=
    Equal,        // =
    PlusEqual,    // +=
    MinusEqual,   // -=
    StarEqual,    // *=
    SlashEqual,   // /=
    PercentEqual, // %=
    Ampersand,    // &
    Pipe,         // |
    Caret,        // ^
    Tilde,        // ~
    LeftShift,    // <<
    RightShift,   // >>
    DotDot,       // ..

    // End of file
    Eof,
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub message: String,
    pub position: Position,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Lex error at {}: {}", self.position, self.message)
    }
}

impl std::error::Error for LexError {}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokens(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace();

        let start_pos = Position::new(self.line, self.column);

        if self.is_at_end() {
            return Ok(Token::new(TokenKind::Eof, start_pos, String::new()));
        }

        let ch = self.current_char();

        match ch {
            // Skip comments
            '/' if self.peek() == Some('/') => {
                self.skip_line_comment();
                return self.next_token();
            }
            '/' if self.peek() == Some('*') => {
                self.skip_block_comment()?;
                return self.next_token();
            }

            // Single-character tokens
            '(' => Ok(self.make_token(TokenKind::LeftParen, 1)),
            ')' => Ok(self.make_token(TokenKind::RightParen, 1)),
            '{' => Ok(self.make_token(TokenKind::LeftBrace, 1)),
            '}' => Ok(self.make_token(TokenKind::RightBrace, 1)),
            '[' => Ok(self.make_token(TokenKind::LeftBracket, 1)),
            ']' => Ok(self.make_token(TokenKind::RightBracket, 1)),
            ',' => Ok(self.make_token(TokenKind::Comma, 1)),
            ';' => Ok(self.make_token(TokenKind::Semicolon, 1)),
            ':' => Ok(self.make_token(TokenKind::Colon, 1)),
            '?' => Ok(self.make_token(TokenKind::Question, 1)),
            '@' => Ok(self.make_token(TokenKind::At, 1)),
            '^' => Ok(self.make_token(TokenKind::Caret, 1)),
            '~' => Ok(self.make_token(TokenKind::Tilde, 1)),
            '&' => Ok(self.make_token(TokenKind::Ampersand, 1)),
            '|' => Ok(self.make_token(TokenKind::Pipe, 1)),

            // Two-character operators and single character fallbacks
            '+' => {
                if self.peek() == Some('=') {
                    Ok(self.make_token(TokenKind::PlusEqual, 2))
                } else {
                    Ok(self.make_token(TokenKind::Plus, 1))
                }
            }
            '-' => {
                if self.peek() == Some('=') {
                    Ok(self.make_token(TokenKind::MinusEqual, 2))
                } else {
                    Ok(self.make_token(TokenKind::Minus, 1))
                }
            }
            '*' => {
                if self.peek() == Some('=') {
                    Ok(self.make_token(TokenKind::StarEqual, 2))
                } else {
                    Ok(self.make_token(TokenKind::Star, 1))
                }
            }
            '/' => {
                if self.peek() == Some('=') {
                    Ok(self.make_token(TokenKind::SlashEqual, 2))
                } else {
                    Ok(self.make_token(TokenKind::Slash, 1))
                }
            }
            '%' => {
                if self.peek() == Some('=') {
                    Ok(self.make_token(TokenKind::PercentEqual, 2))
                } else {
                    Ok(self.make_token(TokenKind::Percent, 1))
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    Ok(self.make_token(TokenKind::LessEqual, 2))
                } else if self.peek() == Some('<') {
                    Ok(self.make_token(TokenKind::LeftShift, 2))
                } else {
                    Ok(self.make_token(TokenKind::Less, 1))
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    Ok(self.make_token(TokenKind::GreaterEqual, 2))
                } else if self.peek() == Some('>') {
                    Ok(self.make_token(TokenKind::RightShift, 2))
                } else {
                    Ok(self.make_token(TokenKind::Greater, 1))
                }
            }
            '=' => Ok(self.make_token(TokenKind::Equal, 1)),
            '.' => {
                if self.peek() == Some('.') {
                    Ok(self.make_token(TokenKind::DotDot, 2))
                } else {
                    Ok(self.make_token(TokenKind::Dot, 1))
                }
            }

            // Numbers
            ch if ch.is_ascii_digit() => self.lex_number(),
            '0' if matches!(self.peek(), Some('x') | Some('X') | Some('b') | Some('B')) => {
                self.lex_number()
            }

            // Identifiers and keywords
            ch if ch.is_alphabetic() || ch == '_' => self.lex_identifier_or_keyword(),

            _ => Err(LexError {
                message: format!("Unexpected character: '{}'", ch),
                position: start_pos,
            }),
        }
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.current_char() {
                ' ' | '\r' => {
                    self.advance();
                }
                '\t' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.column = 0; // Will be incremented by advance()
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn skip_line_comment(&mut self) {
        // Skip the //
        self.advance();
        self.advance();

        while !self.is_at_end() && self.current_char() != '\n' {
            self.advance();
        }
    }

    fn skip_block_comment(&mut self) -> Result<(), LexError> {
        // Skip the /*
        let start_pos = Position::new(self.line, self.column);
        self.advance();
        self.advance();

        let mut nesting_level = 1;

        while !self.is_at_end() && nesting_level > 0 {
            let ch = self.current_char();

            if ch == '/' && self.peek() == Some('*') {
                nesting_level += 1;
                self.advance();
                self.advance();
            } else if ch == '*' && self.peek() == Some('/') {
                nesting_level -= 1;
                self.advance();
                self.advance();
            } else {
                if ch == '\n' {
                    self.line += 1;
                    self.column = 0; // Will be incremented by advance()
                }
                self.advance();
            }
        }

        if nesting_level > 0 {
            return Err(LexError {
                message: "Unterminated block comment".to_string(),
                position: start_pos,
            });
        }

        Ok(())
    }

    fn lex_number(&mut self) -> Result<Token, LexError> {
        let start_pos = Position::new(self.line, self.column);
        let start = self.position;

        // Handle hex numbers
        if self.current_char() == '0' && matches!(self.peek(), Some('x') | Some('X')) {
            self.advance(); // 0
            self.advance(); // x

            while !self.is_at_end() && (self.current_char().is_ascii_hexdigit() || self.current_char() == '_') {
                self.advance();
            }

            let lexeme: String = self.input[start..self.position].iter().collect();
            let hex_str = lexeme[2..].replace('_', "");

            if hex_str.is_empty() {
                return Err(LexError {
                    message: "Invalid hex number".to_string(),
                    position: start_pos,
                });
            }

            let value = i64::from_str_radix(&hex_str, 16)
                .map_err(|_| LexError {
                    message: "Invalid hex number".to_string(),
                    position: start_pos,
                })?;

            return Ok(Token::new(TokenKind::Integer(value), start_pos, lexeme));
        }

        // Handle binary numbers
        if self.current_char() == '0' && matches!(self.peek(), Some('b') | Some('B')) {
            self.advance(); // 0
            self.advance(); // b

            while !self.is_at_end() && (matches!(self.current_char(), '0' | '1' | '_')) {
                self.advance();
            }

            let lexeme: String = self.input[start..self.position].iter().collect();
            let bin_str = lexeme[2..].replace('_', "");

            if bin_str.is_empty() {
                return Err(LexError {
                    message: "Invalid binary number".to_string(),
                    position: start_pos,
                });
            }

            let value = i64::from_str_radix(&bin_str, 2)
                .map_err(|_| LexError {
                    message: "Invalid binary number".to_string(),
                    position: start_pos,
                })?;

            return Ok(Token::new(TokenKind::Integer(value), start_pos, lexeme));
        }

        // Handle decimal numbers (int or float)
        while !self.is_at_end() && (self.current_char().is_ascii_digit() || self.current_char() == '_') {
            self.advance();
        }

        // Check for decimal point or scientific notation
        let mut is_float = false;

        if !self.is_at_end() && self.current_char() == '.' && self.peek().map_or(false, |ch| ch.is_ascii_digit()) {
            is_float = true;
            self.advance(); // .
            while !self.is_at_end() && (self.current_char().is_ascii_digit() || self.current_char() == '_') {
                self.advance();
            }
        }

        // Check for scientific notation
        if !self.is_at_end() && matches!(self.current_char(), 'e' | 'E') {
            is_float = true;
            self.advance();

            if !self.is_at_end() && matches!(self.current_char(), '+' | '-') {
                self.advance();
            }

            if self.is_at_end() || !self.current_char().is_ascii_digit() {
                return Err(LexError {
                    message: "Invalid scientific notation".to_string(),
                    position: start_pos,
                });
            }

            while !self.is_at_end() && (self.current_char().is_ascii_digit() || self.current_char() == '_') {
                self.advance();
            }
        }

        let lexeme: String = self.input[start..self.position].iter().collect();
        let number_str = lexeme.replace('_', "");

        if is_float {
            let value = number_str.parse::<f64>()
                .map_err(|_| LexError {
                    message: "Invalid float literal".to_string(),
                    position: start_pos,
                })?;
            Ok(Token::new(TokenKind::Float(value), start_pos, lexeme))
        } else {
            let value = number_str.parse::<i64>()
                .map_err(|_| LexError {
                    message: "Invalid integer literal".to_string(),
                    position: start_pos,
                })?;
            Ok(Token::new(TokenKind::Integer(value), start_pos, lexeme))
        }
    }

    fn lex_identifier_or_keyword(&mut self) -> Result<Token, LexError> {
        let start_pos = Position::new(self.line, self.column);
        let start = self.position;

        while !self.is_at_end() {
            let ch = self.current_char();
            if ch.is_alphanumeric() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let lexeme: String = self.input[start..self.position].iter().collect();
        let kind = self.keyword_or_identifier(&lexeme);

        Ok(Token::new(kind, start_pos, lexeme))
    }

    fn keyword_or_identifier(&self, lexeme: &str) -> TokenKind {
        match lexeme {
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "and" => TokenKind::And,
            "arena" => TokenKind::Arena,
            "async" => TokenKind::Async,
            "break" => TokenKind::Break,
            "case" => TokenKind::Case,
            "continue" => TokenKind::Continue,
            "default" => TokenKind::Default,
            "defer" => TokenKind::Defer,
            "else" => TokenKind::Else,
            "ensures" => TokenKind::Ensures,
            "export" => TokenKind::Export,
            "for" => TokenKind::For,
            "function" => TokenKind::Function,
            "heap" => TokenKind::Heap,
            "high" => TokenKind::High,
            "if" => TokenKind::If,
            "import" => TokenKind::Import,
            "in" => TokenKind::In,
            "is" => TokenKind::Is,
            "let" => TokenKind::Let,
            "loop" => TokenKind::Loop,
            "low" => TokenKind::Low,
            "manual" => TokenKind::Manual,
            "match" => TokenKind::Match,
            "module" => TokenKind::Module,
            "not" => TokenKind::Not,
            "nothing" => TokenKind::Nothing,
            "or" => TokenKind::Or,
            "program" => TokenKind::Program,
            "requires" => TokenKind::Requires,
            "return" => TokenKind::Return,
            "shared" => TokenKind::Shared,
            "stack" => TokenKind::Stack,
            "switch" => TokenKind::Switch,
            "this" => TokenKind::This,
            "unknown" => TokenKind::Unknown,
            "when" => TokenKind::When,
            "while" => TokenKind::While,
            "watch" => TokenKind::Watch,
            _ => TokenKind::Identifier,
        }
    }

    fn make_token(&mut self, kind: TokenKind, length: usize) -> Token {
        let pos = Position::new(self.line, self.column);
        let start = self.position;

        for _ in 0..length {
            self.advance();
        }

        let lexeme: String = self.input[start..self.position].iter().collect();
        Token::new(kind, pos, lexeme)
    }

    fn current_char(&self) -> char {
        self.input[self.position]
    }

    fn peek(&self) -> Option<char> {
        if self.position + 1 >= self.input.len() {
            None
        } else {
            Some(self.input[self.position + 1])
        }
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.column += 1;
            self.position += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }
}