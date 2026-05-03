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

        // Parse body - Phase 2b: parse actual statements
        let body = self.parse_block()?;

        // Expect EOF
        self.expect_token(TokenKind::Eof, "Expected end of file")?;

        Ok(Program {
            mode,
            params,
            return_type,
            body: Some(body),
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

        // Parse body - Phase 2b: parse actual statements
        let body = self.parse_block()?;

        Ok(Function {
            name,
            mode: function_mode,
            params,
            return_type,
            body: Some(body),
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


    // Phase 2b: Statement and Expression Parsing

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let start_token = self.expect_token(TokenKind::LeftBrace, "Expected '{'")?;
        let position = start_token.position;

        let mut statements = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        self.expect_token(TokenKind::RightBrace, "Expected '}'")?;

        Ok(Block { statements, position })
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match &self.peek()?.kind {
            TokenKind::Let => self.parse_let_statement(),
            TokenKind::Return => self.parse_return_statement(),
            TokenKind::If => self.parse_if_statement(),
            TokenKind::While => self.parse_while_statement(),
            TokenKind::Loop => self.parse_loop_statement(),
            TokenKind::Break => self.parse_break_statement(),
            TokenKind::Continue => self.parse_continue_statement(),
            _ => {
                // Try to parse assignment or expression statement
                let checkpoint = self.current;

                // Try assignment first
                if let Ok(assignment) = self.try_parse_assignment() {
                    return Ok(Statement::Assign(assignment));
                }

                // Reset and parse as expression statement
                self.current = checkpoint;
                let expr = self.parse_expression()?;
                Ok(Statement::ExprStatement(expr))
            }
        }
    }

    fn parse_let_statement(&mut self) -> Result<Statement, ParseError> {
        let start_pos = self.advance()?.position; // consume 'let'

        let name_token = self.expect_identifier("Expected variable name after 'let'")?;
        let name = name_token.lexeme;

        // Optional type annotation
        let ty = if self.check(&TokenKind::Colon) {
            self.advance()?; // consume ':'
            Some(self.parse_type()?)
        } else {
            None
        };

        // Optional initializer
        let initializer = if self.check(&TokenKind::Equal) {
            self.advance()?; // consume '='
            Some(self.parse_expression()?)
        } else {
            None
        };

        // Require at least a type or an initializer
        if ty.is_none() && initializer.is_none() {
            return Err(ParseError::UnexpectedToken {
                expected: "type annotation (': type') or initializer ('= value')".to_string(),
                found: self.peek()?.kind.clone(),
                position: start_pos,
            });
        }

        Ok(Statement::Let(LetDecl {
            name,
            ty,
            initializer,
            position: start_pos,
        }))
    }

    fn parse_return_statement(&mut self) -> Result<Statement, ParseError> {
        let start_pos = self.advance()?.position; // consume 'return'

        // Optional return value
        let value = if self.check(&TokenKind::RightBrace) ||
                       self.check(&TokenKind::Semicolon) ||
                       self.is_at_end() {
            None
        } else {
            Some(self.parse_expression()?)
        };

        Ok(Statement::Return(ReturnStmt {
            value,
            position: start_pos,
        }))
    }

    fn parse_if_statement(&mut self) -> Result<Statement, ParseError> {
        let start_pos = self.advance()?.position; // consume 'if'

        self.expect_token(TokenKind::LeftParen, "Expected '(' after 'if'")?;
        let condition = self.parse_expression()?;
        self.expect_token(TokenKind::RightParen, "Expected ')' after if condition")?;

        let then_block = self.parse_block()?;

        let else_block = if self.check(&TokenKind::Else) {
            self.advance()?; // consume 'else'

            if self.check(&TokenKind::If) {
                // else if - we'll need to handle this by wrapping the if statement in a block
                let if_stmt = self.parse_if_statement()?;
                if let Statement::If(if_stmt) = if_stmt {
                    Some(Block {
                        statements: vec![Statement::If(if_stmt)],
                        position: start_pos.clone(),
                    })
                } else {
                    unreachable!()
                }
            } else {
                // else block
                Some(self.parse_block()?)
            }
        } else {
            None
        };

        Ok(Statement::If(IfStmt {
            condition,
            then_block,
            else_block,
            position: start_pos,
        }))
    }

    fn parse_while_statement(&mut self) -> Result<Statement, ParseError> {
        let start_pos = self.advance()?.position; // consume 'while'

        self.expect_token(TokenKind::LeftParen, "Expected '(' after 'while'")?;
        let condition = self.parse_expression()?;
        self.expect_token(TokenKind::RightParen, "Expected ')' after while condition")?;

        let body = self.parse_block()?;

        Ok(Statement::While(WhileStmt {
            condition,
            body,
            position: start_pos,
        }))
    }

    fn parse_loop_statement(&mut self) -> Result<Statement, ParseError> {
        let start_pos = self.advance()?.position; // consume 'loop'
        let body = self.parse_block()?;

        Ok(Statement::Loop(LoopStmt {
            body,
            position: start_pos,
        }))
    }

    fn parse_break_statement(&mut self) -> Result<Statement, ParseError> {
        let pos = self.advance()?.position; // consume 'break'
        Ok(Statement::Break(pos))
    }

    fn parse_continue_statement(&mut self) -> Result<Statement, ParseError> {
        let pos = self.advance()?.position; // consume 'continue'
        Ok(Statement::Continue(pos))
    }

    fn try_parse_assignment(&mut self) -> Result<AssignStmt, ParseError> {
        let target = self.parse_primary_expression()?;

        let op_token = self.peek()?;
        let op = match &op_token.kind {
            TokenKind::Equal => AssignOpKind::Assign,
            TokenKind::PlusEqual => AssignOpKind::AddAssign,
            TokenKind::MinusEqual => AssignOpKind::SubAssign,
            TokenKind::StarEqual => AssignOpKind::MulAssign,
            TokenKind::SlashEqual => AssignOpKind::DivAssign,
            TokenKind::PercentEqual => AssignOpKind::ModAssign,
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "assignment operator".to_string(),
                    found: op_token.kind.clone(),
                    position: op_token.position.clone(),
                });
            }
        };

        let position = self.advance()?.position; // consume assignment operator
        let value = self.parse_expression()?;

        Ok(AssignStmt {
            target,
            op,
            value,
            position,
        })
    }

    // Expression parsing with Pratt parser for operator precedence

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_expression_with_precedence(0)
    }

    fn parse_expression_with_precedence(&mut self, min_prec: u8) -> Result<Expression, ParseError> {
        let mut left = self.parse_unary_expression()?;

        while !self.is_at_end() {
            let op_token = self.peek()?;

            if let Some((op, prec, _)) = self.get_binary_operator_info(&op_token.kind) {
                if prec < min_prec {
                    break;
                }

                let op_kind = op_token.kind.clone(); // Clone for potential error message
                let position = self.advance()?.position; // consume operator

                // Special handling for 'is' and 'is not' to create IsCheck nodes
                if matches!(op, BinaryOpKind::Is | BinaryOpKind::IsNot) {
                    // Parse the type name
                    let type_token = self.expect_identifier("Expected type name after 'is'")?;
                    let type_name = &type_token.lexeme;

                    // Convert type name to Type
                    let ty = match type_name.as_str() {
                        "i8" => Type::Primitive(PrimitiveType::I8),
                        "i16" => Type::Primitive(PrimitiveType::I16),
                        "i32" => Type::Primitive(PrimitiveType::I32),
                        "i64" => Type::Primitive(PrimitiveType::I64),
                        "i128" => Type::Primitive(PrimitiveType::I128),
                        "u8" => Type::Primitive(PrimitiveType::U8),
                        "u16" => Type::Primitive(PrimitiveType::U16),
                        "u32" => Type::Primitive(PrimitiveType::U32),
                        "u64" => Type::Primitive(PrimitiveType::U64),
                        "u128" => Type::Primitive(PrimitiveType::U128),
                        "f32" => Type::Primitive(PrimitiveType::F32),
                        "f64" => Type::Primitive(PrimitiveType::F64),
                        "bool" => Type::Primitive(PrimitiveType::Bool),
                        "string" => Type::Primitive(PrimitiveType::String),
                        "usize" => Type::Primitive(PrimitiveType::Usize),
                        "isize" => Type::Primitive(PrimitiveType::Isize),
                        "nothing" => Type::Primitive(PrimitiveType::Nothing),
                        _ => {
                            return Err(ParseError::UnexpectedToken {
                                expected: "valid type name".to_string(),
                                found: op_kind,
                                position: type_token.position,
                            });
                        }
                    };

                    left = Expression::IsCheck(IsCheck {
                        expression: Box::new(left),
                        ty,
                        negated: matches!(op, BinaryOpKind::IsNot),
                        position,
                    });
                } else {
                    let right = self.parse_expression_with_precedence(prec + 1)?;

                    left = Expression::BinaryOp(BinaryOp {
                        lhs: Box::new(left),
                        op,
                        rhs: Box::new(right),
                        position,
                    });
                }
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_unary_expression(&mut self) -> Result<Expression, ParseError> {
        let token = self.peek()?;

        match &token.kind {
            TokenKind::Minus => {
                let position = self.advance()?.position;
                let operand = self.parse_unary_expression()?;
                Ok(Expression::UnaryOp(UnaryOp {
                    op: UnaryOpKind::Neg,
                    operand: Box::new(operand),
                    position,
                }))
            }
            TokenKind::Not => {
                let position = self.advance()?.position;
                let operand = self.parse_unary_expression()?;
                Ok(Expression::UnaryOp(UnaryOp {
                    op: UnaryOpKind::Not,
                    operand: Box::new(operand),
                    position,
                }))
            }
            TokenKind::Tilde => {
                let position = self.advance()?.position;
                let operand = self.parse_unary_expression()?;
                Ok(Expression::UnaryOp(UnaryOp {
                    op: UnaryOpKind::BitNot,
                    operand: Box::new(operand),
                    position,
                }))
            }
            _ => self.parse_postfix_expression(),
        }
    }

    fn parse_postfix_expression(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_primary_expression()?;

        loop {
            match &self.peek()?.kind {
                TokenKind::LeftParen => {
                    // Function call
                    let position = self.advance()?.position; // consume '('
                    let mut args = Vec::new();

                    if !self.check(&TokenKind::RightParen) {
                        args.push(self.parse_expression()?);

                        while self.check(&TokenKind::Comma) {
                            self.advance()?; // consume ','
                            args.push(self.parse_expression()?);
                        }
                    }

                    self.expect_token(TokenKind::RightParen, "Expected ')' after function arguments")?;

                    expr = Expression::Call(Call {
                        callee: Box::new(expr),
                        args,
                        position,
                    });
                }
                TokenKind::Dot => {
                    // Member access
                    let position = self.advance()?.position; // consume '.'
                    let member_token = self.expect_identifier("Expected member name after '.'")?;

                    expr = Expression::MemberAccess(MemberAccess {
                        object: Box::new(expr),
                        member: member_token.lexeme,
                        position,
                    });
                }
                TokenKind::LeftBracket => {
                    // Index access
                    let position = self.advance()?.position; // consume '['
                    let index = self.parse_expression()?;
                    self.expect_token(TokenKind::RightBracket, "Expected ']' after array index")?;

                    expr = Expression::IndexAccess(IndexAccess {
                        object: Box::new(expr),
                        index: Box::new(index),
                        position,
                    });
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary_expression(&mut self) -> Result<Expression, ParseError> {
        let token = self.advance()?;

        match token.kind {
            TokenKind::Integer(n) => Ok(Expression::IntLit(n, token.position)),
            TokenKind::Float(f) => Ok(Expression::FloatLit(f, token.position)),
            TokenKind::True => Ok(Expression::BoolLit(true, token.position)),
            TokenKind::False => Ok(Expression::BoolLit(false, token.position)),
            TokenKind::Identifier => Ok(Expression::Ident(token.lexeme, token.position)),
            TokenKind::LeftParen => {
                // Parenthesized expression
                let expr = self.parse_expression()?;
                self.expect_token(TokenKind::RightParen, "Expected ')' after expression")?;
                Ok(expr)
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "expression".to_string(),
                found: token.kind,
                position: token.position,
            }),
        }
    }

    fn get_binary_operator_info(&self, token_kind: &TokenKind) -> Option<(BinaryOpKind, u8, bool)> {
        // Returns (operator, precedence, is_right_associative)
        // Precedence levels (higher number = higher precedence):
        match token_kind {
            // Level 1: or
            TokenKind::Or => Some((BinaryOpKind::Or, 1, false)),

            // Level 2: and
            TokenKind::And => Some((BinaryOpKind::And, 2, false)),

            // Level 4: comparisons (note: 'not' is unary, handled elsewhere)
            TokenKind::EqStrict => Some((BinaryOpKind::Eq, 4, false)),
            TokenKind::NotEq => Some((BinaryOpKind::NotEq, 4, false)),
            TokenKind::NotLess => Some((BinaryOpKind::NotLess, 4, false)),
            TokenKind::NotGreater => Some((BinaryOpKind::NotGreater, 4, false)),
            TokenKind::Less => Some((BinaryOpKind::Less, 4, false)),
            TokenKind::Greater => Some((BinaryOpKind::Greater, 4, false)),
            TokenKind::LessEqual => Some((BinaryOpKind::LessEq, 4, false)),
            TokenKind::GreaterEqual => Some((BinaryOpKind::GreaterEq, 4, false)),
            TokenKind::Is => Some((BinaryOpKind::Is, 4, false)),

            // Level 5: bitwise or
            TokenKind::Pipe => Some((BinaryOpKind::BitOr, 5, false)),

            // Level 6: bitwise xor
            TokenKind::Caret => Some((BinaryOpKind::BitXor, 6, false)),

            // Level 7: bitwise and
            TokenKind::Ampersand => Some((BinaryOpKind::BitAnd, 7, false)),

            // Level 8: shifts
            TokenKind::LeftShift => Some((BinaryOpKind::ShiftLeft, 8, false)),
            TokenKind::RightShift => Some((BinaryOpKind::ShiftRight, 8, false)),

            // Level 9: addition, subtraction
            TokenKind::Plus => Some((BinaryOpKind::Add, 9, false)),
            TokenKind::Minus => Some((BinaryOpKind::Sub, 9, false)),

            // Level 10: multiplication, division, modulo
            TokenKind::Star => Some((BinaryOpKind::Mul, 10, false)),
            TokenKind::Slash => Some((BinaryOpKind::Div, 10, false)),
            TokenKind::Percent => Some((BinaryOpKind::Mod, 10, false)),

            // Levels 11-12 are unary and postfix, handled separately

            _ => None,
        }
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