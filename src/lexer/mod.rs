pub mod token;

use token::{Token, TokenKind, keyword};

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            let is_eof = matches!(token.kind, TokenKind::Eof);
            tokens.push(token);

            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token, String> {
        self.skip_whitespace_and_comments();

        let start_line = self.line;
        let start_column = self.column;

        if self.is_at_end() {
            return Ok(Token::new(TokenKind::Eof, String::new(), start_line, start_column));
        }

        let ch = self.current();

        // String literals (including f-strings and raw strings)
        if ch == '"' || self.peek_string("f\"") || self.peek_string("r\"") || self.peek_string("rf\"") {
            return self.read_string(start_line, start_column);
        }

        // Identifiers and keywords
        if ch.is_alphabetic() || ch == '_' {
            return Ok(self.read_identifier(start_line, start_column));
        }

        // Numbers
        if ch.is_numeric() {
            return self.read_number(start_line, start_column);
        }

        // Operators and delimiters
        let token_kind = match ch {
            '(' => {
                self.advance();
                TokenKind::LeftParen
            }
            ')' => {
                self.advance();
                TokenKind::RightParen
            }
            '{' => {
                self.advance();
                TokenKind::LeftBrace
            }
            '}' => {
                self.advance();
                TokenKind::RightBrace
            }
            '[' => {
                self.advance();
                TokenKind::LeftBracket
            }
            ']' => {
                self.advance();
                TokenKind::RightBracket
            }
            ';' => {
                self.advance();
                TokenKind::Semicolon
            }
            ':' => {
                self.advance();
                TokenKind::Colon
            }
            ',' => {
                self.advance();
                TokenKind::Comma
            }
            '.' => {
                self.advance();
                if self.current() == '.' {
                    self.advance();
                    TokenKind::DotDot
                } else {
                    TokenKind::Dot
                }
            }
            '+' => {
                self.advance();
                if self.current() == '=' {
                    self.advance();
                    TokenKind::PlusEqual
                } else {
                    TokenKind::Plus
                }
            }
            '-' => {
                self.advance();
                if self.current() == '=' {
                    self.advance();
                    TokenKind::MinusEqual
                } else {
                    TokenKind::Minus
                }
            }
            '*' => {
                self.advance();
                if self.current() == '=' {
                    self.advance();
                    TokenKind::StarEqual
                } else {
                    TokenKind::Star
                }
            }
            '/' => {
                self.advance();
                if self.current() == '=' {
                    self.advance();
                    TokenKind::SlashEqual
                } else {
                    TokenKind::Slash
                }
            }
            '%' => {
                self.advance();
                if self.current() == '=' {
                    self.advance();
                    TokenKind::PercentEqual
                } else {
                    TokenKind::Percent
                }
            }
            '=' => {
                self.advance();
                if self.current() == '>' {
                    self.advance();
                    TokenKind::Arrow
                } else {
                    TokenKind::Equal
                }
            }
            '?' => {
                self.advance();
                if self.current() == '=' {
                    self.advance();
                    TokenKind::EqualQuestion
                } else if self.peek_string("?=") {
                    self.advance();
                    self.advance();
                    TokenKind::EqualQuestionDouble
                } else {
                    return Err(format!("Unexpected character '?' at {}:{}", start_line, start_column));
                }
            }
            '!' => {
                self.advance();
                if self.current() == '=' {
                    self.advance();
                    TokenKind::BangEqual
                } else if self.peek_string("!=") {
                    self.advance();
                    self.advance();
                    TokenKind::BangEqualDouble
                } else {
                    return Err(format!("Unexpected character '!' at {}:{}", start_line, start_column));
                }
            }
            '<' => {
                self.advance();
                if self.current() == '=' {
                    self.advance();
                    TokenKind::LessEqual
                } else if self.current() == '<' {
                    self.advance();
                    TokenKind::ShiftLeft
                } else {
                    TokenKind::Less
                }
            }
            '>' => {
                self.advance();
                if self.current() == '=' {
                    self.advance();
                    TokenKind::GreaterEqual
                } else if self.current() == '>' {
                    self.advance();
                    TokenKind::ShiftRight
                } else {
                    TokenKind::Greater
                }
            }
            '&' => {
                self.advance();
                TokenKind::Ampersand
            }
            '|' => {
                self.advance();
                TokenKind::Pipe
            }
            '^' => {
                self.advance();
                TokenKind::Caret
            }
            '~' => {
                self.advance();
                TokenKind::Tilde
            }
            _ => {
                return Err(format!("Unexpected character '{}' at {}:{}", ch, start_line, start_column));
            }
        };

        let lexeme = ch.to_string();
        Ok(Token::new(token_kind, lexeme, start_line, start_column))
    }

    fn read_identifier(&mut self, start_line: usize, start_column: usize) -> Token {
        let start = self.position;

        while !self.is_at_end() && (self.current().is_alphanumeric() || self.current() == '_') {
            self.advance();
        }

        let lexeme: String = self.input[start..self.position].iter().collect();

        let kind = if let Some(keyword_kind) = keyword(&lexeme) {
            keyword_kind
        } else {
            TokenKind::Identifier(lexeme.clone())
        };

        Token::new(kind, lexeme, start_line, start_column)
    }

    fn read_number(&mut self, start_line: usize, start_column: usize) -> Result<Token, String> {
        let start = self.position;

        while !self.is_at_end() && self.current().is_numeric() {
            self.advance();
        }

        // Check for decimal point
        let is_float = !self.is_at_end() && self.current() == '.' &&
                       self.peek_ahead(1).map_or(false, |c| c.is_numeric());

        if is_float {
            self.advance(); // consume '.'
            while !self.is_at_end() && self.current().is_numeric() {
                self.advance();
            }
        }

        // Check for scientific notation (e or E)
        if !self.is_at_end() && (self.current() == 'e' || self.current() == 'E') {
            self.advance(); // consume 'e' or 'E'

            // Optional + or - sign
            if !self.is_at_end() && (self.current() == '+' || self.current() == '-') {
                self.advance();
            }

            // Exponent digits
            if !self.is_at_end() && self.current().is_numeric() {
                while !self.is_at_end() && self.current().is_numeric() {
                    self.advance();
                }
            } else {
                return Err(format!("Invalid scientific notation at {}:{}", start_line, start_column));
            }
        }

        // Determine if this is a float
        let lexeme: String = self.input[start..self.position].iter().collect();
        if is_float || lexeme.contains('e') || lexeme.contains('E') {
            let value = lexeme.parse::<f64>()
                .map_err(|_| format!("Invalid float literal '{}' at {}:{}", lexeme, start_line, start_column))?;

            Ok(Token::new(TokenKind::FloatLiteral(value), lexeme, start_line, start_column))
        } else {
            let lexeme: String = self.input[start..self.position].iter().collect();
            let value = lexeme.parse::<i64>()
                .map_err(|_| format!("Invalid integer literal '{}' at {}:{}", lexeme, start_line, start_column))?;

            Ok(Token::new(TokenKind::IntegerLiteral(value), lexeme, start_line, start_column))
        }
    }

    fn read_string(&mut self, start_line: usize, start_column: usize) -> Result<Token, String> {
        use crate::lexer::token::FStringPart;

        // Check for prefix (f, r, rf)
        let is_fstring = self.peek_string("f\"") || self.peek_string("rf\"");
        let is_raw = self.peek_string("r\"") || self.peek_string("rf\"");

        // Skip prefix
        if self.current() == 'r' {
            self.advance();
        }
        if self.current() == 'f' {
            self.advance();
        }

        // Count opening quotes
        let quote_count = self.count_quotes();
        if quote_count == 0 {
            return Err(format!("Expected string opening quotes at {}:{}", start_line, start_column));
        }

        // Skip opening quotes
        for _ in 0..quote_count {
            self.advance();
        }

        if is_fstring {
            // Parse f-string with interpolation
            let mut parts = Vec::new();
            let mut current_text = String::new();

            while !self.is_at_end() {
                // Check if we have closing quotes
                if self.peek_quotes_ahead() == quote_count {
                    // Add any remaining text
                    if !current_text.is_empty() {
                        parts.push(FStringPart::Text(current_text.clone()));
                    }

                    // Consume closing quotes
                    for _ in 0..quote_count {
                        self.advance();
                    }

                    let lexeme = format!("f\"...\"");
                    return Ok(Token::new(TokenKind::FStringLiteral(parts), lexeme, start_line, start_column));
                }

                let ch = self.current();

                if ch == '{' {
                    // Save current text part
                    if !current_text.is_empty() {
                        parts.push(FStringPart::Text(current_text.clone()));
                        current_text.clear();
                    }

                    // Skip the '{'
                    self.advance();

                    // Read the expression until we find '}'
                    let mut expr = String::new();
                    let mut brace_depth = 1;

                    while !self.is_at_end() && brace_depth > 0 {
                        let expr_ch = self.current();
                        if expr_ch == '{' {
                            brace_depth += 1;
                        } else if expr_ch == '}' {
                            brace_depth -= 1;
                            if brace_depth == 0 {
                                break;
                            }
                        }
                        expr.push(expr_ch);
                        self.advance();
                    }

                    if self.is_at_end() {
                        return Err(format!("Unterminated expression in f-string at {}:{}", start_line, start_column));
                    }

                    // Skip the closing '}'
                    self.advance();

                    parts.push(FStringPart::Expression(expr.trim().to_string()));
                } else {
                    if ch == '\n' {
                        self.line += 1;
                        self.column = 0;
                    }
                    current_text.push(ch);
                    self.advance();
                }
            }

            Err(format!("Unterminated f-string at {}:{}", start_line, start_column))
        } else {
            // Regular string
            let mut value = String::new();

            // Read until we find matching closing quotes
            while !self.is_at_end() {
                // Check if we have closing quotes
                if self.peek_quotes_ahead() == quote_count {
                    // Consume closing quotes
                    for _ in 0..quote_count {
                        self.advance();
                    }

                    let lexeme = format!("\"{}\"", value);
                    let token_kind = if is_raw {
                        TokenKind::RawStringLiteral(value)
                    } else {
                        TokenKind::StringLiteral(value)
                    };
                    return Ok(Token::new(token_kind, lexeme, start_line, start_column));
                }

                let ch = self.current();

                if ch == '\n' {
                    self.line += 1;
                    self.column = 0;
                }

                value.push(ch);
                self.advance();
            }

            Err(format!("Unterminated string at {}:{}", start_line, start_column))
        }
    }

    fn count_quotes(&self) -> usize {
        let mut count = 0;
        let mut pos = self.position;

        while pos < self.input.len() && self.input[pos] == '"' {
            count += 1;
            pos += 1;
        }

        count
    }

    fn peek_quotes_ahead(&self) -> usize {
        let mut count = 0;
        let mut pos = self.position;

        while pos < self.input.len() && self.input[pos] == '"' {
            count += 1;
            pos += 1;
        }

        count
    }

    fn skip_whitespace_and_comments(&mut self) {
        while !self.is_at_end() {
            let ch = self.current();

            if ch.is_whitespace() {
                if ch == '\n' {
                    self.line += 1;
                    self.column = 0;
                }
                self.advance();
            } else if ch == '/' && self.peek_ahead(1) == Some('/') {
                // Single-line comment
                while !self.is_at_end() && self.current() != '\n' {
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn current(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.input[self.position]
        }
    }

    fn peek_ahead(&self, offset: usize) -> Option<char> {
        let pos = self.position + offset;
        if pos < self.input.len() {
            Some(self.input[pos])
        } else {
            None
        }
    }

    fn peek_string(&self, s: &str) -> bool {
        let chars: Vec<char> = s.chars().collect();
        for (i, &ch) in chars.iter().enumerate() {
            if self.position + i >= self.input.len() || self.input[self.position + i] != ch {
                return false;
            }
        }
        true
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.position += 1;
            self.column += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new("function let if else return");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 6); // 5 keywords + EOF
        assert!(matches!(tokens[0].kind, TokenKind::Function));
        assert!(matches!(tokens[1].kind, TokenKind::Let));
        assert!(matches!(tokens[2].kind, TokenKind::If));
        assert!(matches!(tokens[3].kind, TokenKind::Else));
        assert!(matches!(tokens[4].kind, TokenKind::Return));
    }

    #[test]
    fn test_identifiers() {
        let mut lexer = Lexer::new("foo bar_baz x123");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 4); // 3 identifiers + EOF
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(_)));
        assert!(matches!(tokens[2].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("42 3.14 0");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 4); // 3 numbers + EOF
        assert!(matches!(tokens[0].kind, TokenKind::IntegerLiteral(42)));
        assert!(matches!(tokens[1].kind, TokenKind::FloatLiteral(_)));
        assert!(matches!(tokens[2].kind, TokenKind::IntegerLiteral(0)));
    }

    #[test]
    fn test_strings() {
        let mut lexer = Lexer::new(r#""hello" "world""#);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 3); // 2 strings + EOF
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "hello");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_operators() {
        let mut lexer = Lexer::new("+ - * / ?= ??= != !!=");
        let tokens = lexer.tokenize().unwrap();

        assert!(matches!(tokens[0].kind, TokenKind::Plus));
        assert!(matches!(tokens[1].kind, TokenKind::Minus));
        assert!(matches!(tokens[2].kind, TokenKind::Star));
        assert!(matches!(tokens[3].kind, TokenKind::Slash));
        assert!(matches!(tokens[4].kind, TokenKind::EqualQuestion));
        assert!(matches!(tokens[5].kind, TokenKind::EqualQuestionDouble));
        assert!(matches!(tokens[6].kind, TokenKind::BangEqual));
        assert!(matches!(tokens[7].kind, TokenKind::BangEqualDouble));
    }

    #[test]
    fn test_comments() {
        let mut lexer = Lexer::new("let x // comment\nlet y");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 5); // let, x, let, y, EOF
    }
}
