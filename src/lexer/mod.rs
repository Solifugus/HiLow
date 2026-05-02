use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Integer(i64),
    Float(f64),
    True,
    False,

    // Identifiers
    Identifier,

    // Keywords (41 total as specified)
    And,
    Arena,
    Async,
    Break,
    Case,
    Continue,
    Decreases,
    Default,
    Defer,
    Else,
    Ensures,
    Excluding,
    Export,
    For,
    From,
    Function,
    Heap,
    High,
    If,
    Import,
    In,
    Invariant,
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
    Stealth,
    Switch,
    This,
    Unknown,
    Watch,
    When,
    While,

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

    // Arithmetic operators
    Plus,         // +
    Minus,        // -
    Star,         // *
    Slash,        // /
    Percent,      // %

    // Comparison operators (excluding equality)
    Less,         // <
    Greater,      // >
    LessEqual,    // <=
    GreaterEqual, // >=

    // Assignment operators
    Equal,        // =
    PlusEqual,    // +=
    MinusEqual,   // -=
    StarEqual,    // *=
    SlashEqual,   // /=
    PercentEqual, // %=

    // Bitwise operators
    Ampersand,    // &
    Pipe,         // |
    Caret,        // ^
    Tilde,        // ~
    LeftShift,    // <<
    RightShift,   // >>

    // Range operator
    DotDot,       // ..

    // Special
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LexError {
    UnexpectedCharacter { char: char, position: Position },
    UnterminatedBlockComment { position: Position },
    InvalidNumber { text: String, position: Position },
}

pub struct Lexer {
    input: Vec<char>,
    current: usize,
    line: usize,
    column: usize,
    keywords: HashMap<String, TokenKind>,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        let mut keywords = HashMap::new();

        // Insert all 41 keywords
        keywords.insert("and".to_string(), TokenKind::And);
        keywords.insert("arena".to_string(), TokenKind::Arena);
        keywords.insert("async".to_string(), TokenKind::Async);
        keywords.insert("break".to_string(), TokenKind::Break);
        keywords.insert("case".to_string(), TokenKind::Case);
        keywords.insert("continue".to_string(), TokenKind::Continue);
        keywords.insert("decreases".to_string(), TokenKind::Decreases);
        keywords.insert("default".to_string(), TokenKind::Default);
        keywords.insert("defer".to_string(), TokenKind::Defer);
        keywords.insert("else".to_string(), TokenKind::Else);
        keywords.insert("ensures".to_string(), TokenKind::Ensures);
        keywords.insert("excluding".to_string(), TokenKind::Excluding);
        keywords.insert("export".to_string(), TokenKind::Export);
        keywords.insert("false".to_string(), TokenKind::False);
        keywords.insert("for".to_string(), TokenKind::For);
        keywords.insert("from".to_string(), TokenKind::From);
        keywords.insert("function".to_string(), TokenKind::Function);
        keywords.insert("heap".to_string(), TokenKind::Heap);
        keywords.insert("high".to_string(), TokenKind::High);
        keywords.insert("if".to_string(), TokenKind::If);
        keywords.insert("import".to_string(), TokenKind::Import);
        keywords.insert("in".to_string(), TokenKind::In);
        keywords.insert("invariant".to_string(), TokenKind::Invariant);
        keywords.insert("is".to_string(), TokenKind::Is);
        keywords.insert("let".to_string(), TokenKind::Let);
        keywords.insert("loop".to_string(), TokenKind::Loop);
        keywords.insert("low".to_string(), TokenKind::Low);
        keywords.insert("manual".to_string(), TokenKind::Manual);
        keywords.insert("match".to_string(), TokenKind::Match);
        keywords.insert("module".to_string(), TokenKind::Module);
        keywords.insert("not".to_string(), TokenKind::Not);
        keywords.insert("nothing".to_string(), TokenKind::Nothing);
        keywords.insert("or".to_string(), TokenKind::Or);
        keywords.insert("program".to_string(), TokenKind::Program);
        keywords.insert("requires".to_string(), TokenKind::Requires);
        keywords.insert("return".to_string(), TokenKind::Return);
        keywords.insert("shared".to_string(), TokenKind::Shared);
        keywords.insert("stack".to_string(), TokenKind::Stack);
        keywords.insert("stealth".to_string(), TokenKind::Stealth);
        keywords.insert("switch".to_string(), TokenKind::Switch);
        keywords.insert("this".to_string(), TokenKind::This);
        keywords.insert("true".to_string(), TokenKind::True);
        keywords.insert("unknown".to_string(), TokenKind::Unknown);
        keywords.insert("watch".to_string(), TokenKind::Watch);
        keywords.insert("when".to_string(), TokenKind::When);
        keywords.insert("while".to_string(), TokenKind::While);

        Self {
            input: input.chars().collect(),
            current: 0,
            line: 1,
            column: 1,
            keywords,
        }
    }

    pub fn tokens(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace_and_comments()?;

            if self.is_at_end() {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    lexeme: String::new(),
                    position: Position {
                        line: self.line,
                        column: self.column,
                    },
                });
                break;
            }

            let token = self.next_token()?;
            tokens.push(token);
        }

        Ok(tokens)
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.input.len()
    }

    fn advance(&mut self) -> char {
        let ch = self.input[self.current];
        self.current += 1;

        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        ch
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.input[self.current]
        }
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.input.len() {
            '\0'
        } else {
            self.input[self.current + 1]
        }
    }

    fn make_token(&self, kind: TokenKind, start_pos: Position, lexeme: String) -> Token {
        Token {
            kind,
            lexeme,
            position: start_pos,
        }
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), LexError> {
        loop {
            if self.is_at_end() {
                break;
            }

            match self.peek() {
                ' ' | '\r' | '\t' | '\n' => {
                    self.advance();
                }
                '/' => {
                    if self.peek_next() == '/' {
                        // Line comment
                        while self.peek() != '\n' && !self.is_at_end() {
                            self.advance();
                        }
                    } else if self.peek_next() == '*' {
                        // Block comment (with nesting)
                        self.skip_block_comment()?;
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn skip_block_comment(&mut self) -> Result<(), LexError> {
        let start_pos = Position {
            line: self.line,
            column: self.column,
        };

        self.advance(); // consume '/'
        self.advance(); // consume '*'

        let mut nesting_level = 1;

        while nesting_level > 0 && !self.is_at_end() {
            if self.peek() == '/' && self.peek_next() == '*' {
                self.advance(); // consume '/'
                self.advance(); // consume '*'
                nesting_level += 1;
            } else if self.peek() == '*' && self.peek_next() == '/' {
                self.advance(); // consume '*'
                self.advance(); // consume '/'
                nesting_level -= 1;
            } else {
                self.advance();
            }
        }

        if nesting_level > 0 {
            return Err(LexError::UnterminatedBlockComment {
                position: start_pos,
            });
        }

        Ok(())
    }

    fn next_token(&mut self) -> Result<Token, LexError> {
        let start_pos = Position {
            line: self.line,
            column: self.column,
        };

        let ch = self.advance();

        match ch {
            // Single-character tokens
            '(' => Ok(self.make_token(TokenKind::LeftParen, start_pos, "(".to_string())),
            ')' => Ok(self.make_token(TokenKind::RightParen, start_pos, ")".to_string())),
            '{' => Ok(self.make_token(TokenKind::LeftBrace, start_pos, "{".to_string())),
            '}' => Ok(self.make_token(TokenKind::RightBrace, start_pos, "}".to_string())),
            '[' => Ok(self.make_token(TokenKind::LeftBracket, start_pos, "[".to_string())),
            ']' => Ok(self.make_token(TokenKind::RightBracket, start_pos, "]".to_string())),
            ',' => Ok(self.make_token(TokenKind::Comma, start_pos, ",".to_string())),
            ';' => Ok(self.make_token(TokenKind::Semicolon, start_pos, ";".to_string())),
            ':' => Ok(self.make_token(TokenKind::Colon, start_pos, ":".to_string())),
            '?' => Ok(self.make_token(TokenKind::Question, start_pos, "?".to_string())),
            '@' => Ok(self.make_token(TokenKind::At, start_pos, "@".to_string())),
            '&' => Ok(self.make_token(TokenKind::Ampersand, start_pos, "&".to_string())),
            '|' => Ok(self.make_token(TokenKind::Pipe, start_pos, "|".to_string())),
            '^' => Ok(self.make_token(TokenKind::Caret, start_pos, "^".to_string())),
            '~' => Ok(self.make_token(TokenKind::Tilde, start_pos, "~".to_string())),

            // Multi-character tokens
            '.' => {
                if self.peek() == '.' {
                    self.advance();
                    Ok(self.make_token(TokenKind::DotDot, start_pos, "..".to_string()))
                } else {
                    Ok(self.make_token(TokenKind::Dot, start_pos, ".".to_string()))
                }
            }
            '+' => {
                if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::PlusEqual, start_pos, "+=".to_string()))
                } else {
                    Ok(self.make_token(TokenKind::Plus, start_pos, "+".to_string()))
                }
            }
            '-' => {
                if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::MinusEqual, start_pos, "-=".to_string()))
                } else {
                    Ok(self.make_token(TokenKind::Minus, start_pos, "-".to_string()))
                }
            }
            '*' => {
                if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::StarEqual, start_pos, "*=".to_string()))
                } else {
                    Ok(self.make_token(TokenKind::Star, start_pos, "*".to_string()))
                }
            }
            '/' => {
                if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::SlashEqual, start_pos, "/=".to_string()))
                } else {
                    Ok(self.make_token(TokenKind::Slash, start_pos, "/".to_string()))
                }
            }
            '%' => {
                if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::PercentEqual, start_pos, "%=".to_string()))
                } else {
                    Ok(self.make_token(TokenKind::Percent, start_pos, "%".to_string()))
                }
            }
            '<' => {
                if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::LessEqual, start_pos, "<=".to_string()))
                } else if self.peek() == '<' {
                    self.advance();
                    Ok(self.make_token(TokenKind::LeftShift, start_pos, "<<".to_string()))
                } else {
                    Ok(self.make_token(TokenKind::Less, start_pos, "<".to_string()))
                }
            }
            '>' => {
                if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::GreaterEqual, start_pos, ">=".to_string()))
                } else if self.peek() == '>' {
                    self.advance();
                    Ok(self.make_token(TokenKind::RightShift, start_pos, ">>".to_string()))
                } else {
                    Ok(self.make_token(TokenKind::Greater, start_pos, ">".to_string()))
                }
            }
            '=' => Ok(self.make_token(TokenKind::Equal, start_pos, "=".to_string())),

            // Numbers
            '0'..='9' => self.number(start_pos),

            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' => self.identifier_or_keyword(start_pos),

            _ => Err(LexError::UnexpectedCharacter {
                char: ch,
                position: start_pos,
            }),
        }
    }

    fn number(&mut self, start_pos: Position) -> Result<Token, LexError> {
        let start = self.current - 1; // Back up to include the first digit

        // Check for hex (0x) or binary (0b)
        if self.input[start] == '0' && !self.is_at_end() {
            match self.peek() {
                'x' | 'X' => return self.hex_number(start_pos),
                'b' | 'B' => return self.binary_number(start_pos),
                _ => {}
            }
        }

        // Decimal number
        self.decimal_number(start_pos)
    }

    fn hex_number(&mut self, start_pos: Position) -> Result<Token, LexError> {
        let start = self.current - 1; // Back up to include the '0'
        self.advance(); // consume 'x'

        // Consume hex digits and underscores
        while !self.is_at_end() {
            let ch = self.peek();
            if ch.is_ascii_hexdigit() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let lexeme: String = self.input[start..self.current].iter().collect();
        let number_part = &lexeme[2..]; // Skip "0x"
        let cleaned = number_part.replace('_', "");

        match i64::from_str_radix(&cleaned, 16) {
            Ok(value) => Ok(self.make_token(TokenKind::Integer(value), start_pos, lexeme)),
            Err(_) => Err(LexError::InvalidNumber {
                text: lexeme,
                position: start_pos,
            }),
        }
    }

    fn binary_number(&mut self, start_pos: Position) -> Result<Token, LexError> {
        let start = self.current - 1; // Back up to include the '0'
        self.advance(); // consume 'b'

        // Consume binary digits and underscores
        while !self.is_at_end() {
            let ch = self.peek();
            if ch == '0' || ch == '1' || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let lexeme: String = self.input[start..self.current].iter().collect();
        let number_part = &lexeme[2..]; // Skip "0b"
        let cleaned = number_part.replace('_', "");

        match i64::from_str_radix(&cleaned, 2) {
            Ok(value) => Ok(self.make_token(TokenKind::Integer(value), start_pos, lexeme)),
            Err(_) => Err(LexError::InvalidNumber {
                text: lexeme,
                position: start_pos,
            }),
        }
    }

    fn decimal_number(&mut self, start_pos: Position) -> Result<Token, LexError> {
        let start = self.current - 1; // Back up to include the first digit

        // Consume digits and underscores
        while !self.is_at_end() {
            let ch = self.peek();
            if ch.is_ascii_digit() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }

        // Check for decimal point
        let mut is_float = false;
        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            is_float = true;
            self.advance(); // consume '.'

            // Consume fractional part
            while !self.is_at_end() {
                let ch = self.peek();
                if ch.is_ascii_digit() || ch == '_' {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Check for exponent
        if self.peek() == 'e' || self.peek() == 'E' {
            is_float = true;
            self.advance(); // consume 'e'

            // Optional +/- for exponent
            if self.peek() == '+' || self.peek() == '-' {
                self.advance();
            }

            // Consume exponent digits
            while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
                self.advance();
            }
        }

        let lexeme: String = self.input[start..self.current].iter().collect();
        let cleaned = lexeme.replace('_', "");

        if is_float {
            match cleaned.parse::<f64>() {
                Ok(value) => Ok(self.make_token(TokenKind::Float(value), start_pos, lexeme)),
                Err(_) => Err(LexError::InvalidNumber {
                    text: lexeme,
                    position: start_pos,
                }),
            }
        } else {
            match cleaned.parse::<i64>() {
                Ok(value) => Ok(self.make_token(TokenKind::Integer(value), start_pos, lexeme)),
                Err(_) => Err(LexError::InvalidNumber {
                    text: lexeme,
                    position: start_pos,
                }),
            }
        }
    }

    fn identifier_or_keyword(&mut self, start_pos: Position) -> Result<Token, LexError> {
        let start = self.current - 1; // Back up to include the first character

        // Consume identifier characters
        while !self.is_at_end() {
            let ch = self.peek();
            if ch.is_alphanumeric() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let lexeme: String = self.input[start..self.current].iter().collect();

        // Check if it's a keyword
        if let Some(keyword_kind) = self.keywords.get(&lexeme) {
            Ok(self.make_token(keyword_kind.clone(), start_pos, lexeme))
        } else {
            Ok(self.make_token(TokenKind::Identifier, start_pos, lexeme))
        }
    }
}