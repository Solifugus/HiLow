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
    StringLit(String),
    DurationLiteral(i64, String), // nanoseconds, original_unit
    True,
    False,

    // F-string tokens
    FStringStart,
    FStringText(String),
    FStringExprStart,
    FStringExprEnd,
    FStringEnd,

    // Identifiers
    Identifier,

    // Keywords (46 total as specified)
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
    Weak,
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

    // Comparison operators
    Less,         // <
    Greater,      // >
    LessEqual,    // <=
    GreaterEqual, // >=

    // Equality operators
    EqStrict,     // ?=
    NotEq,        // !=

    // Negation comparators
    NotLess,      // !<
    NotGreater,   // !>

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

    // Arrow operator
    Arrow,        // =>

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
    InvalidOperator { operator: String, position: Position, suggestion: String },
    UnterminatedString { position: Position },
    InvalidEscapeSequence { sequence: String, position: Position },
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexError::UnexpectedCharacter { char, position } => {
                write!(f, "Unexpected character '{}' at line {}, column {}", char, position.line, position.column)
            }
            LexError::UnterminatedBlockComment { position } => {
                write!(f, "Unterminated block comment at line {}, column {}", position.line, position.column)
            }
            LexError::InvalidNumber { text, position } => {
                write!(f, "Invalid number '{}' at line {}, column {}", text, position.line, position.column)
            }
            LexError::InvalidOperator { operator, position, suggestion } => {
                write!(f, "Invalid operator '{}' at line {}, column {}: {}", operator, position.line, position.column, suggestion)
            }
            LexError::UnterminatedString { position } => {
                write!(f, "Unterminated string at line {}, column {}", position.line, position.column)
            }
            LexError::InvalidEscapeSequence { sequence, position } => {
                write!(f, "Invalid escape sequence '{}' at line {}, column {}", sequence, position.line, position.column)
            }
        }
    }
}

impl std::error::Error for LexError {}

pub struct Lexer {
    input: Vec<char>,
    current: usize,
    line: usize,
    column: usize,
    keywords: HashMap<String, TokenKind>,
    // F-string state
    fstring_state: Option<FStringState>,
}

#[derive(Debug, Clone)]
struct FStringState {
    quote_count: usize,
    is_raw: bool,
    brace_depth: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        let mut keywords = HashMap::new();

        // Insert all 46 keywords
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
        keywords.insert("weak".to_string(), TokenKind::Weak);
        keywords.insert("when".to_string(), TokenKind::When);
        keywords.insert("while".to_string(), TokenKind::While);

        Self {
            input: input.chars().collect(),
            current: 0,
            line: 1,
            column: 1,
            keywords,
            fstring_state: None,
        }
    }

    pub fn tokens(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        loop {
            // Only skip whitespace if we're not in f-string text mode
            // (preserve whitespace in f-string text segments)
            if let Some(state) = &self.fstring_state {
                if state.brace_depth == 0 {
                    // We're in f-string text mode - don't skip whitespace
                } else {
                    // We're in f-string expression mode - skip whitespace normally
                    self.skip_whitespace_and_comments()?;
                }
            } else {
                // Not in f-string mode - skip whitespace normally
                self.skip_whitespace_and_comments()?;
            }

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

        // Check if we're in f-string mode
        if let Some(state) = &self.fstring_state.clone() {
            let ch = self.peek();

            // If we're in an expression (brace_depth > 0), handle normally
            if state.brace_depth > 0 {
                // Let normal tokenization handle the content
                // Update brace depth tracking
                if ch == '{' {
                    if let Some(ref mut state) = &mut self.fstring_state {
                        state.brace_depth += 1;
                    }
                } else if ch == '}' {
                    if let Some(ref mut state) = &mut self.fstring_state {
                        state.brace_depth -= 1;
                        if state.brace_depth == 0 {
                            // This closes the expression
                            self.advance(); // consume '}'
                            return Ok(self.make_token(TokenKind::FStringExprEnd, start_pos, "}".to_string()));
                        }
                    }
                }
                // Continue with normal tokenization for expression content
            } else {
                // We're in f-string text mode (brace_depth == 0)
                return self.f_string_content(start_pos);
            }
        }

        let ch = self.advance();

        match ch {
            // Single-character tokens
            '(' => Ok(self.make_token(TokenKind::LeftParen, start_pos, "(".to_string())),
            ')' => Ok(self.make_token(TokenKind::RightParen, start_pos, ")".to_string())),
            '{' => {
                Ok(self.make_token(TokenKind::LeftBrace, start_pos, "{".to_string()))
            }
            '}' => {
                Ok(self.make_token(TokenKind::RightBrace, start_pos, "}".to_string()))
            }
            '[' => Ok(self.make_token(TokenKind::LeftBracket, start_pos, "[".to_string())),
            ']' => Ok(self.make_token(TokenKind::RightBracket, start_pos, "]".to_string())),
            ',' => Ok(self.make_token(TokenKind::Comma, start_pos, ",".to_string())),
            ';' => Ok(self.make_token(TokenKind::Semicolon, start_pos, ";".to_string())),
            ':' => Ok(self.make_token(TokenKind::Colon, start_pos, ":".to_string())),
            '?' => {
                if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::EqStrict, start_pos, "?=".to_string()))
                } else {
                    Ok(self.make_token(TokenKind::Question, start_pos, "?".to_string()))
                }
            }
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
            '=' => {
                if self.peek() == '=' {
                    Err(LexError::InvalidOperator {
                        operator: "==".to_string(),
                        position: start_pos,
                        suggestion: "'==' is not a valid operator in HiLow; use '?=' for equality or '=' for assignment".to_string(),
                    })
                } else if self.peek() == '>' {
                    self.advance();
                    Ok(self.make_token(TokenKind::Arrow, start_pos, "=>".to_string()))
                } else {
                    Ok(self.make_token(TokenKind::Equal, start_pos, "=".to_string()))
                }
            }
            '!' => {
                match self.peek() {
                    '=' => {
                        self.advance();
                        Ok(self.make_token(TokenKind::NotEq, start_pos, "!=".to_string()))
                    }
                    '<' => {
                        self.advance(); // consume '<'
                        if self.peek() == '=' {
                            Err(LexError::InvalidOperator {
                                operator: "!<=".to_string(),
                                position: start_pos,
                                suggestion: "'!<=' is redundant; use '>' instead".to_string(),
                            })
                        } else {
                            Ok(self.make_token(TokenKind::NotLess, start_pos, "!<".to_string()))
                        }
                    }
                    '>' => {
                        self.advance(); // consume '>'
                        if self.peek() == '=' {
                            Err(LexError::InvalidOperator {
                                operator: "!>=".to_string(),
                                position: start_pos,
                                suggestion: "'!>=' is redundant; use '<' instead".to_string(),
                            })
                        } else {
                            Ok(self.make_token(TokenKind::NotGreater, start_pos, "!>".to_string()))
                        }
                    }
                    _ => {
                        Err(LexError::InvalidOperator {
                            operator: "!".to_string(),
                            position: start_pos,
                            suggestion: "'!' is not a valid operator in HiLow; use the 'not' keyword for logical negation".to_string(),
                        })
                    }
                }
            }

            // Numbers
            '0'..='9' => self.number(start_pos),

            // String literals
            '"' => self.string_literal(start_pos, false),

            // Identifiers and keywords (including raw strings and f-strings)
            'a'..='z' | 'A'..='Z' | '_' => {
                // Check for string prefixes
                if ch == 'r' && self.peek() == '"' {
                    // Raw string: r"..."
                    self.advance(); // consume '"'
                    self.string_literal(start_pos, true)
                } else if ch == 'f' && self.peek() == '"' {
                    // F-string: f"..."
                    self.advance(); // consume '"'
                    self.f_string(start_pos, false)
                } else if ch == 'r' && self.peek() == 'f' {
                    // Check for raw f-string: rf"..."
                    let lookahead = self.current + 1;
                    if lookahead < self.input.len() && self.input[lookahead] == '"' {
                        self.advance(); // consume 'f'
                        self.advance(); // consume '"'
                        self.f_string(start_pos, true)
                    } else {
                        self.identifier_or_keyword(start_pos)
                    }
                } else if ch == 'f' && self.peek() == 'r' {
                    // Check for raw f-string: fr"..."
                    let lookahead = self.current + 1;
                    if lookahead < self.input.len() && self.input[lookahead] == '"' {
                        self.advance(); // consume 'r'
                        self.advance(); // consume '"'
                        self.f_string(start_pos, true)
                    } else {
                        self.identifier_or_keyword(start_pos)
                    }
                } else {
                    self.identifier_or_keyword(start_pos)
                }
            }

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

        // Get the numeric part first (without suffix)
        let numeric_end = self.current;
        let numeric_lexeme: String = self.input[start..numeric_end].iter().collect();
        let numeric_cleaned = numeric_lexeme.replace('_', "");

        // Check for duration suffix immediately following the number (no whitespace)
        let duration_suffix = self.try_duration_suffix();

        // If we found a duration suffix, create a DurationLiteral token
        if let Some(suffix) = duration_suffix {
            let full_lexeme: String = self.input[start..self.current].iter().collect();

            let numeric_part = if is_float {
                match numeric_cleaned.parse::<f64>() {
                    Ok(value) => value,
                    Err(_) => return Err(LexError::InvalidNumber {
                        text: full_lexeme,
                        position: start_pos,
                    }),
                }
            } else {
                match numeric_cleaned.parse::<i64>() {
                    Ok(value) => value as f64,
                    Err(_) => return Err(LexError::InvalidNumber {
                        text: full_lexeme,
                        position: start_pos,
                    }),
                }
            };

            let nanos = self.convert_to_nanoseconds(numeric_part, &suffix);
            return Ok(self.make_token(TokenKind::DurationLiteral(nanos, suffix), start_pos, full_lexeme));
        }

        // Regular number (not duration)
        let lexeme = numeric_lexeme;
        let cleaned = numeric_cleaned;

        // Regular number (not duration)
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

    fn string_literal(&mut self, start_pos: Position, is_raw: bool) -> Result<Token, LexError> {
        // Count the opening quotes
        let mut quote_count = 1; // We already consumed the first quote
        while !self.is_at_end() && self.peek() == '"' {
            self.advance();
            quote_count += 1;
        }

        let mut content = String::new();
        let mut consecutive_quotes = 0;

        // Read content until we find N consecutive quotes
        while !self.is_at_end() {
            let ch = self.advance();

            if ch == '"' {
                consecutive_quotes += 1;
                if consecutive_quotes == quote_count {
                    // Found the closing quote sequence
                    break;
                }
                // Don't add quotes to content yet - we might find more
            } else {
                // Reset quote counter and add any accumulated quotes
                if consecutive_quotes > 0 {
                    for _ in 0..consecutive_quotes {
                        content.push('"');
                    }
                    consecutive_quotes = 0;
                }

                if is_raw {
                    // Raw strings: no escape processing
                    content.push(ch);
                } else {
                    // Regular strings: process escape sequences
                    if ch == '\\' {
                        if self.is_at_end() {
                            return Err(LexError::UnterminatedString { position: start_pos });
                        }
                        let escaped_char = self.advance();
                        match escaped_char {
                            'n' => content.push('\n'),
                            't' => content.push('\t'),
                            'r' => content.push('\r'),
                            '\\' => content.push('\\'),
                            '"' => content.push('"'),
                            'x' => {
                                // \x followed by exactly 2 hex digits
                                let hex_chars = self.read_hex_digits(2)?;
                                if hex_chars.len() != 2 {
                                    return Err(LexError::InvalidEscapeSequence {
                                        sequence: format!("\\x{}", hex_chars),
                                        position: start_pos,
                                    });
                                }
                                if let Ok(byte_val) = u8::from_str_radix(&hex_chars, 16) {
                                    content.push(byte_val as char);
                                } else {
                                    return Err(LexError::InvalidEscapeSequence {
                                        sequence: format!("\\x{}", hex_chars),
                                        position: start_pos,
                                    });
                                }
                            }
                            'u' => {
                                // \u{...} with 1-6 hex digits
                                if self.peek() != '{' {
                                    return Err(LexError::InvalidEscapeSequence {
                                        sequence: format!("\\u{}", escaped_char),
                                        position: start_pos,
                                    });
                                }
                                self.advance(); // consume '{'
                                let hex_chars = self.read_until_char('}')?;
                                if hex_chars.len() == 0 || hex_chars.len() > 6 {
                                    return Err(LexError::InvalidEscapeSequence {
                                        sequence: format!("\\u{{{}}}", hex_chars),
                                        position: start_pos,
                                    });
                                }
                                if let Ok(codepoint) = u32::from_str_radix(&hex_chars, 16) {
                                    if let Some(unicode_char) = std::char::from_u32(codepoint) {
                                        content.push(unicode_char);
                                    } else {
                                        return Err(LexError::InvalidEscapeSequence {
                                            sequence: format!("\\u{{{}}}", hex_chars),
                                            position: start_pos,
                                        });
                                    }
                                } else {
                                    return Err(LexError::InvalidEscapeSequence {
                                        sequence: format!("\\u{{{}}}", hex_chars),
                                        position: start_pos,
                                    });
                                }
                                self.advance(); // consume '}'
                            }
                            _ => {
                                return Err(LexError::InvalidEscapeSequence {
                                    sequence: format!("\\{}", escaped_char),
                                    position: start_pos,
                                });
                            }
                        }
                    } else {
                        content.push(ch);
                    }
                }
            }
        }

        if consecutive_quotes != quote_count {
            return Err(LexError::UnterminatedString { position: start_pos });
        }

        // Create lexeme for debugging (include the quotes)
        let prefix = if is_raw { "r" } else { "" };
        let quotes = "\"".repeat(quote_count);
        let lexeme = format!("{}{}{}{}", prefix, quotes, content, quotes);

        Ok(self.make_token(TokenKind::StringLit(content), start_pos, lexeme))
    }

    fn read_hex_digits(&mut self, max_digits: usize) -> Result<String, LexError> {
        let mut hex_chars = String::new();
        for _ in 0..max_digits {
            if self.is_at_end() || !self.peek().is_ascii_hexdigit() {
                break;
            }
            hex_chars.push(self.advance());
        }
        Ok(hex_chars)
    }

    fn read_until_char(&mut self, delimiter: char) -> Result<String, LexError> {
        let mut chars = String::new();
        while !self.is_at_end() && self.peek() != delimiter {
            let ch = self.advance();
            if ch.is_ascii_hexdigit() {
                chars.push(ch);
            } else {
                return Ok(chars); // Return what we have so far if non-hex found
            }
        }
        Ok(chars)
    }

    fn f_string(&mut self, start_pos: Position, is_raw: bool) -> Result<Token, LexError> {
        // Count the opening quotes
        let mut quote_count = 1; // We already consumed the first quote
        while !self.is_at_end() && self.peek() == '"' {
            self.advance();
            quote_count += 1;
        }

        // Initialize f-string state
        self.fstring_state = Some(FStringState {
            quote_count,
            is_raw,
            brace_depth: 0,
        });

        // Create lexeme for debugging
        let prefix = if is_raw { "rf" } else { "f" };
        let quotes = "\"".repeat(quote_count);
        let lexeme = format!("{}{}", prefix, quotes);

        Ok(self.make_token(TokenKind::FStringStart, start_pos, lexeme))
    }

    fn f_string_content(&mut self, start_pos: Position) -> Result<Token, LexError> {
        let state = self.fstring_state.as_ref().unwrap();
        let quote_count = state.quote_count;
        let is_raw = state.is_raw;

        // First check if we're immediately at the end of the f-string
        if self.peek() == '"' {
            // Count quotes to see if this is the end
            let mut quotes_found = 0;
            let mut temp_pos = self.current;
            while temp_pos < self.input.len() && self.input[temp_pos] == '"' {
                quotes_found += 1;
                temp_pos += 1;
            }

            if quotes_found >= quote_count {
                // This is the end - consume the quotes and end the f-string
                for _ in 0..quote_count {
                    self.advance();
                }
                self.fstring_state = None;
                let lexeme = "\"".repeat(quote_count);
                return Ok(self.make_token(TokenKind::FStringEnd, start_pos, lexeme));
            }
        }

        let mut content = String::new();

        // Read content until we find a delimiter (closing quotes, {, or })
        while !self.is_at_end() {
            let ch = self.peek();

            if ch == '"' {
                // Check if this is the closing quote sequence
                let mut quotes_found = 0;
                let mut temp_pos = self.current;
                while temp_pos < self.input.len() && self.input[temp_pos] == '"' {
                    quotes_found += 1;
                    temp_pos += 1;
                }

                if quotes_found >= quote_count {
                    // This is the end - return any accumulated text first
                    if !content.is_empty() {
                        return Ok(self.make_token(TokenKind::FStringText(content.clone()), start_pos, content));
                    } else {
                        // No content, end the f-string
                        for _ in 0..quote_count {
                            self.advance();
                        }
                        self.fstring_state = None;
                        let lexeme = "\"".repeat(quote_count);
                        return Ok(self.make_token(TokenKind::FStringEnd, start_pos, lexeme));
                    }
                } else {
                    // Not enough quotes - treat as literal content
                    for _ in 0..quotes_found {
                        self.advance();
                        content.push('"');
                    }
                    continue;
                }
            }

            if ch == '{' {
                // Start of expression or {{ escape
                if self.peek_next() == '{' {
                    // {{ escape - literal {
                    self.advance(); // consume first {
                    self.advance(); // consume second {
                    content.push('{');
                    continue;
                } else {
                    // Start of expression
                    if !content.is_empty() {
                        // Return accumulated text first
                        return Ok(self.make_token(TokenKind::FStringText(content.clone()), start_pos, content));
                    } else {
                        // Start expression immediately
                        self.advance(); // consume {
                        // Update brace depth to indicate we're in an expression
                        if let Some(ref mut state) = &mut self.fstring_state {
                            state.brace_depth = 1;
                        }
                        return Ok(self.make_token(TokenKind::FStringExprStart, start_pos, "{".to_string()));
                    }
                }
            }

            if ch == '}' {
                // }} escape or error
                if self.peek_next() == '}' {
                    // }} escape - literal }
                    self.advance(); // consume first }
                    self.advance(); // consume second }
                    content.push('}');
                    continue;
                } else {
                    // Unexpected single } in f-string text
                    return Err(LexError::UnexpectedCharacter { char: ch, position: start_pos });
                }
            }

            // Regular character - add to content
            self.advance();
            if is_raw {
                content.push(ch);
            } else {
                // Handle escape sequences (simplified for now)
                if ch == '\\' && !self.is_at_end() {
                    let escaped = self.advance();
                    match escaped {
                        'n' => content.push('\n'),
                        't' => content.push('\t'),
                        'r' => content.push('\r'),
                        '\\' => content.push('\\'),
                        '"' => content.push('"'),
                        '{' => content.push('{'),
                        '}' => content.push('}'),
                        _ => {
                            content.push('\\');
                            content.push(escaped);
                        }
                    }
                } else {
                    content.push(ch);
                }
            }
        }

        Err(LexError::UnterminatedString { position: start_pos })
    }

    // Try to parse a duration suffix immediately following a number (no whitespace)
    fn try_duration_suffix(&mut self) -> Option<String> {
        let saved_pos = self.current;

        // Check for duration suffixes in priority order (longer first)
        let suffixes = ["ns", "us", "ms", "s", "m", "h", "d"];

        for suffix in &suffixes {
            if self.matches_suffix(suffix) {
                // Consume the suffix
                for _ in 0..suffix.len() {
                    self.advance();
                }
                return Some(suffix.to_string());
            }
        }

        // No suffix found, restore position
        self.current = saved_pos;
        None
    }

    fn matches_suffix(&self, suffix: &str) -> bool {
        let suffix_chars: Vec<char> = suffix.chars().collect();
        if self.current + suffix_chars.len() > self.input.len() {
            return false;
        }

        for (i, &expected) in suffix_chars.iter().enumerate() {
            if self.input[self.current + i] != expected {
                return false;
            }
        }

        // Make sure there's no identifier character following (would mean it's not a suffix)
        let next_pos = self.current + suffix_chars.len();
        if next_pos < self.input.len() {
            let next_char = self.input[next_pos];
            if next_char.is_ascii_alphanumeric() || next_char == '_' {
                return false; // This is part of a larger identifier
            }
        }

        true
    }

    fn convert_to_nanoseconds(&self, value: f64, suffix: &str) -> i64 {
        let nanos = match suffix {
            "ns" => value,
            "us" => value * 1_000.0,
            "ms" => value * 1_000_000.0,
            "s" => value * 1_000_000_000.0,
            "m" => value * 60.0 * 1_000_000_000.0,
            "h" => value * 60.0 * 60.0 * 1_000_000_000.0,
            "d" => value * 24.0 * 60.0 * 60.0 * 1_000_000_000.0,
            _ => value, // fallback, shouldn't happen
        };
        nanos as i64
    }
}
