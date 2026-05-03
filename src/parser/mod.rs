use crate::ast::*;
use crate::lexer::{Lexer, Token, TokenKind, Position, LexError};

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedToken {
        expected: String,
        found: TokenKind,
        position: Position,
    },
    UnexpectedEof {
        expected: String,
        position: Position,
    },
    UnsupportedFeature {
        feature: String,
        position: Position,
        suggestion: String,
    },
    LexError(LexError),
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(input: &str) -> Result<Self, ParseError> {
        let tokens = Lexer::new(input).tokens().map_err(ParseError::LexError)?;
        Ok(Parser { tokens, current: 0 })
    }

    pub fn parse(mut self) -> Result<TopLevel, ParseError> {
        self.parse_top_level()
    }

    fn parse_top_level(&mut self) -> Result<TopLevel, ParseError> {
        // Expect high/low keyword first
        let mode_token = self.expect_mode_keyword()?;
        let mode = match mode_token.kind {
            TokenKind::High => Mode::High,
            TokenKind::Low => Mode::Low,
            _ => unreachable!(),
        };

        // Then expect program or module
        let kind_token = self.advance()?;
        match kind_token.kind {
            TokenKind::Program => {
                let program = self.parse_program(mode, mode_token.position)?;
                Ok(TopLevel::Program(program))
            }
            TokenKind::Module => {
                let module = self.parse_module(mode, mode_token.position)?;
                Ok(TopLevel::Module(module))
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "'program' or 'module'".to_string(),
                found: kind_token.kind,
                position: kind_token.position,
            }),
        }
    }

    fn parse_program(&mut self, mode: Mode, start_pos: Position) -> Result<Program, ParseError> {
        // Parse parameter list
        self.expect_token(TokenKind::LeftParen, "Expected '(' after 'program'")?;
        let params = self.parse_parameter_list()?;
        self.expect_token(TokenKind::RightParen, "Expected ')' after parameter list")?;

        // Parse return type
        self.expect_token(TokenKind::Colon, "Expected ':' after parameter list")?;
        let return_type = self.parse_type()?;

        // Parse body (skip with brace counting)
        let body = self.parse_body_placeholder()?;

        // Expect EOF
        self.expect_token(TokenKind::Eof, "Expected end of file")?;

        Ok(Program {
            mode,
            params,
            return_type,
            body,
            position: start_pos,
        })
    }

    fn parse_module(&mut self, mode: Mode, start_pos: Position) -> Result<Module, ParseError> {
        // Parse module body
        self.expect_token(TokenKind::LeftBrace, "Expected '{' after 'module'")?;

        let mut items = Vec::new();

        // Parse functions until we hit the closing brace
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let function = self.parse_function_signature(mode.clone())?;
            items.push(function);
        }

        self.expect_token(TokenKind::RightBrace, "Expected '}' after module body")?;
        self.expect_token(TokenKind::Eof, "Expected end of file")?;

        Ok(Module {
            mode,
            items,
            position: start_pos,
        })
    }

    fn parse_function_signature(&mut self, default_mode: Mode) -> Result<Function, ParseError> {
        let start_pos = self.peek()?.position.clone();
        let mut is_export = false;

        // Check for export keyword
        if self.check(&TokenKind::Export) {
            is_export = true;
            self.advance()?;
        }

        // Check for explicit mode
        let function_mode = if self.check(&TokenKind::High) || self.check(&TokenKind::Low) {
            let mode_token = self.advance()?;
            match mode_token.kind {
                TokenKind::High => Mode::High,
                TokenKind::Low => Mode::Low,
                _ => unreachable!(),
            }
        } else {
            // Inherit from enclosing scope
            default_mode
        };

        // Expect 'function' keyword
        self.expect_token(TokenKind::Function, "Expected 'function'")?;

        // Parse function name
        let name_token = self.expect_identifier("Expected function name")?;
        let name = name_token.lexeme;

        // Parse parameter list
        self.expect_token(TokenKind::LeftParen, "Expected '(' after function name")?;
        let params = self.parse_parameter_list()?;
        self.expect_token(TokenKind::RightParen, "Expected ')' after parameter list")?;

        // Parse return type
        self.expect_token(TokenKind::Colon, "Expected ':' after parameter list")?;
        let return_type = self.parse_type()?;

        // Parse body (skip with brace counting)
        let body = self.parse_body_placeholder()?;

        Ok(Function {
            name,
            mode: function_mode,
            params,
            return_type,
            body,
            is_export,
            position: start_pos,
        })
    }

    fn parse_parameter_list(&mut self) -> Result<Vec<Parameter>, ParseError> {
        let mut params = Vec::new();

        // Handle empty parameter list
        if self.check(&TokenKind::RightParen) {
            return Ok(params);
        }

        // Parse first parameter
        params.push(self.parse_parameter()?);

        // Parse additional parameters
        while self.check(&TokenKind::Comma) {
            self.advance()?; // consume comma
            params.push(self.parse_parameter()?);
        }

        Ok(params)
    }

    fn parse_parameter(&mut self) -> Result<Parameter, ParseError> {
        let name_token = self.expect_identifier("Expected parameter name")?;
        let position = name_token.position;
        let name = name_token.lexeme;

        self.expect_token(TokenKind::Colon, &format!("Expected ':' after parameter name '{}'", name))?;
        let ty = self.parse_type()?;

        Ok(Parameter { name, ty, position })
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        if self.check(&TokenKind::LeftBracket) {
            self.parse_array_type()
        } else if self.check(&TokenKind::Star) {
            // Pointer types not supported in Phase 2a
            let pos = self.peek()?.position.clone();
            Err(ParseError::UnsupportedFeature {
                feature: "pointer types".to_string(),
                position: pos,
                suggestion: "pointers not yet supported (Phase 12)".to_string(),
            })
        } else {
            self.parse_primitive_type()
        }
    }

    fn parse_array_type(&mut self) -> Result<Type, ParseError> {
        self.expect_token(TokenKind::LeftBracket, "Expected '['")?;

        let element_type = Box::new(self.parse_type()?);

        if self.check(&TokenKind::Semicolon) {
            // Fixed array: [T; N]
            self.advance()?; // consume semicolon

            let size_token = self.advance()?;
            let size = match size_token.kind {
                TokenKind::Integer(n) if n >= 0 => n as usize,
                TokenKind::Integer(_) => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "positive integer".to_string(),
                        found: size_token.kind,
                        position: size_token.position,
                    });
                }
                _ => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "array size (integer)".to_string(),
                        found: size_token.kind,
                        position: size_token.position,
                    });
                }
            };

            self.expect_token(TokenKind::RightBracket, "Expected ']' after array size")?;
            Ok(Type::FixedArray(element_type, size))
        } else {
            // Dynamic array: [T]
            self.expect_token(TokenKind::RightBracket, "Expected ']' after array element type")?;
            Ok(Type::DynamicArray(element_type))
        }
    }

    fn parse_primitive_type(&mut self) -> Result<Type, ParseError> {
        let token = self.advance()?;

        let primitive = match &token.kind {
            TokenKind::Identifier => {
                match token.lexeme.as_str() {
                    "i8" => PrimitiveType::I8,
                    "i16" => PrimitiveType::I16,
                    "i32" => PrimitiveType::I32,
                    "i64" => PrimitiveType::I64,
                    "i128" => PrimitiveType::I128,
                    "u8" => PrimitiveType::U8,
                    "u16" => PrimitiveType::U16,
                    "u32" => PrimitiveType::U32,
                    "u64" => PrimitiveType::U64,
                    "u128" => PrimitiveType::U128,
                    "f32" => PrimitiveType::F32,
                    "f64" => PrimitiveType::F64,
                    "bool" => PrimitiveType::Bool,
                    "string" => PrimitiveType::String,
                    "usize" => PrimitiveType::Usize,
                    "isize" => PrimitiveType::Isize,
                    _ => {
                        return Err(ParseError::UnexpectedToken {
                            expected: "primitive type name".to_string(),
                            found: token.kind.clone(),
                            position: token.position,
                        });
                    }
                }
            }
            TokenKind::Nothing => PrimitiveType::Nothing,
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "primitive type name".to_string(),
                    found: token.kind,
                    position: token.position,
                });
            }
        };

        Ok(Type::Primitive(primitive))
    }

    fn parse_body_placeholder(&mut self) -> Result<BodyPlaceholder, ParseError> {
        let start_token = self.expect_token(TokenKind::LeftBrace, "Expected '{' for function body")?;
        let start_position = start_token.position;

        // Skip body with brace counting
        let mut brace_count = 1;
        let mut end_position = start_position.clone();

        while brace_count > 0 && !self.is_at_end() {
            let token = self.advance()?;
            end_position = token.position;

            match token.kind {
                TokenKind::LeftBrace => brace_count += 1,
                TokenKind::RightBrace => brace_count -= 1,
                _ => {}
            }
        }

        if brace_count > 0 {
            return Err(ParseError::UnexpectedEof {
                expected: "'}' to close function body".to_string(),
                position: end_position,
            });
        }

        Ok(BodyPlaceholder {
            start_position,
            end_position,
        })
    }

    // Utility methods

    fn expect_mode_keyword(&mut self) -> Result<Token, ParseError> {
        let token = self.advance()?;
        match token.kind {
            TokenKind::High | TokenKind::Low => Ok(token),
            _ => Err(ParseError::UnexpectedToken {
                expected: "'high' or 'low'".to_string(),
                found: token.kind,
                position: token.position,
            }),
        }
    }

    fn expect_token(&mut self, expected: TokenKind, message: &str) -> Result<Token, ParseError> {
        let token = self.advance()?;
        if std::mem::discriminant(&token.kind) == std::mem::discriminant(&expected) {
            Ok(token)
        } else {
            Err(ParseError::UnexpectedToken {
                expected: message.to_string(),
                found: token.kind,
                position: token.position,
            })
        }
    }

    fn expect_identifier(&mut self, message: &str) -> Result<Token, ParseError> {
        let token = self.advance()?;
        match token.kind {
            TokenKind::Identifier => Ok(token),
            _ => Err(ParseError::UnexpectedToken {
                expected: message.to_string(),
                found: token.kind,
                position: token.position,
            }),
        }
    }

    fn advance(&mut self) -> Result<Token, ParseError> {
        if self.is_at_end() {
            let last_pos = if self.tokens.is_empty() {
                Position { line: 1, column: 1 }
            } else {
                self.tokens[self.tokens.len() - 1].position.clone()
            };
            return Err(ParseError::UnexpectedEof {
                expected: "token".to_string(),
                position: last_pos,
            });
        }

        let token = &self.tokens[self.current];
        self.current += 1;
        Ok(token.clone())
    }

    fn peek(&self) -> Result<&Token, ParseError> {
        if self.is_at_end() {
            let last_pos = if self.tokens.is_empty() {
                Position { line: 1, column: 1 }
            } else {
                self.tokens[self.tokens.len() - 1].position.clone()
            };
            return Err(ParseError::UnexpectedEof {
                expected: "token".to_string(),
                position: last_pos,
            });
        }
        Ok(&self.tokens[self.current])
    }

    fn check(&self, token_kind: &TokenKind) -> bool {
        if self.is_at_end() {
            return false;
        }
        std::mem::discriminant(&self.tokens[self.current].kind) == std::mem::discriminant(token_kind)
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }
}