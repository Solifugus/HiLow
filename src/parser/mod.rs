use crate::ast::*;
use crate::lexer::token::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        match &self.peek().kind {
            TokenKind::Function => self.parse_function_decl(),
            TokenKind::Let => self.parse_variable_decl(),
            TokenKind::Return => self.parse_return(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Break => {
                self.advance();
                self.consume_semicolon()?;
                Ok(Statement::Break)
            }
            TokenKind::Continue => {
                self.advance();
                self.consume_semicolon()?;
                Ok(Statement::Continue)
            }
            TokenKind::Switch => self.parse_switch(),
            TokenKind::LeftBrace => self.parse_block_statement(),
            _ => {
                let expr = self.parse_expression()?;
                self.consume_semicolon()?;
                Ok(Statement::Expression(expr))
            }
        }
    }

    fn parse_function_decl(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::Function)?;

        let name = self.expect_identifier()?;

        self.expect(TokenKind::LeftParen)?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            loop {
                let param_name = self.expect_identifier()?;
                self.expect(TokenKind::Colon)?;
                let param_type = self.parse_type()?;

                params.push(Parameter {
                    name: param_name,
                    param_type,
                });

                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.expect(TokenKind::RightParen)?;

        let return_type = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(Statement::FunctionDecl {
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_variable_decl(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::Let)?;

        let name = self.expect_identifier()?;

        let var_type = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let initializer = if self.match_token(&TokenKind::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume_semicolon()?;

        Ok(Statement::VariableDecl {
            name,
            var_type,
            initializer,
        })
    }

    fn parse_return(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::Return)?;

        let value = if !self.check(&TokenKind::Semicolon) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume_semicolon()?;

        Ok(Statement::Return { value })
    }

    fn parse_if(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::If)?;
        self.expect(TokenKind::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;

        let then_branch = self.parse_block()?;

        let else_branch = if self.match_token(&TokenKind::Else) {
            if self.check(&TokenKind::If) {
                Some(Box::new(self.parse_if()?))
            } else {
                Some(Box::new(Statement::Block(self.parse_block()?)))
            }
        } else {
            None
        };

        Ok(Statement::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parse_while(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::While)?;
        self.expect(TokenKind::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;

        let body = self.parse_block()?;

        Ok(Statement::While { condition, body })
    }

    fn parse_for(&mut self) -> Result<Statement, String> {
        self.expect(TokenKind::For)?;
        self.expect(TokenKind::LeftParen)?;

        // Check if this is a for-in loop
        // Save position in case we need to backtrack
        let checkpoint = self.current;

        // Try to parse as for-in loop: for (item in array)
        if let Ok(var_name) = self.expect_identifier() {
            if self.match_token(&TokenKind::In) {
                // This is a for-in loop
                let iterable = self.parse_expression()?;
                self.expect(TokenKind::RightParen)?;
                let body = self.parse_block()?;

                return Ok(Statement::ForIn {
                    variable: var_name,
                    iterable,
                    body,
                });
            }
        }

        // Not a for-in loop, restore position and parse as C-style for loop
        self.current = checkpoint;

        let init = if self.check(&TokenKind::Semicolon) {
            None
        } else if self.check(&TokenKind::Let) {
            Some(Box::new(self.parse_variable_decl()?))
        } else {
            let expr = self.parse_expression()?;
            self.consume_semicolon()?;
            Some(Box::new(Statement::Expression(expr)))
        };

        let condition = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.consume_semicolon()?;

        let increment = if self.check(&TokenKind::RightParen) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.expect(TokenKind::RightParen)?;

        let body = self.parse_block()?;

        Ok(Statement::For {
            init,
            condition,
            increment,
            body,
        })
    }

    fn parse_switch(&mut self) -> Result<Statement, String> {
        use crate::ast::SwitchCase;

        self.expect(TokenKind::Switch)?;
        self.expect(TokenKind::LeftParen)?;
        let expr = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;
        self.expect(TokenKind::LeftBrace)?;

        let mut cases = Vec::new();
        let mut default = None;

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            if self.match_token(&TokenKind::Case) {
                let value = self.parse_expression()?;
                self.expect(TokenKind::Colon)?;

                let mut case_statements = Vec::new();
                while !self.check(&TokenKind::Case)
                    && !self.check(&TokenKind::Default)
                    && !self.check(&TokenKind::RightBrace) {
                    case_statements.push(self.parse_statement()?);
                }

                cases.push(SwitchCase {
                    value,
                    body: Block {
                        statements: case_statements,
                    },
                });
            } else if self.match_token(&TokenKind::Default) {
                self.expect(TokenKind::Colon)?;

                let mut default_statements = Vec::new();
                while !self.check(&TokenKind::Case)
                    && !self.check(&TokenKind::Default)
                    && !self.check(&TokenKind::RightBrace) {
                    default_statements.push(self.parse_statement()?);
                }

                default = Some(Block {
                    statements: default_statements,
                });
            } else {
                return Err(format!("Expected 'case' or 'default' in switch statement, got {:?}", self.peek().kind));
            }
        }

        self.expect(TokenKind::RightBrace)?;

        Ok(Statement::Switch {
            expr,
            cases,
            default,
        })
    }

    fn parse_block_statement(&mut self) -> Result<Statement, String> {
        Ok(Statement::Block(self.parse_block()?))
    }

    fn parse_block(&mut self) -> Result<Block, String> {
        self.expect(TokenKind::LeftBrace)?;

        let mut statements = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        self.expect(TokenKind::RightBrace)?;

        Ok(Block { statements })
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        let token = self.advance();

        match &token.kind {
            TokenKind::Identifier(name) => match name.as_str() {
                "i8" => Ok(Type::I8),
                "i16" => Ok(Type::I16),
                "i32" => Ok(Type::I32),
                "i64" => Ok(Type::I64),
                "i128" => Ok(Type::I128),
                "u8" => Ok(Type::U8),
                "u16" => Ok(Type::U16),
                "u32" => Ok(Type::U32),
                "u64" => Ok(Type::U64),
                "u128" => Ok(Type::U128),
                "f32" => Ok(Type::F32),
                "f64" => Ok(Type::F64),
                "bool" => Ok(Type::Bool),
                "string" => Ok(Type::String),
                _ => Err(format!("Unknown type: {}", name)),
            },
            TokenKind::Nothing => Ok(Type::Nothing),
            TokenKind::Unknown => Ok(Type::Unknown),
            TokenKind::LeftBracket => {
                let element_type = Box::new(self.parse_type()?);

                if self.match_token(&TokenKind::Semicolon) {
                    let size_token = self.advance();
                    if let TokenKind::IntegerLiteral(size) = size_token.kind {
                        self.expect(TokenKind::RightBracket)?;
                        Ok(Type::Array {
                            element_type,
                            size: Some(size as usize),
                        })
                    } else {
                        Err(format!("Expected array size, got {:?}", size_token.kind))
                    }
                } else {
                    self.expect(TokenKind::RightBracket)?;
                    Ok(Type::Array {
                        element_type,
                        size: None,
                    })
                }
            }
            _ => Err(format!("Expected type, got {:?}", token.kind)),
        }
    }

    fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expression, String> {
        let expr = self.parse_or()?;

        if self.match_token(&TokenKind::Equal) {
            let value = self.parse_assignment()?;
            return Ok(Expression::Assignment {
                target: Box::new(expr),
                value: Box::new(value),
            });
        }

        // Handle compound assignments: +=, -=, *=, /=, %=
        let op = if self.match_token(&TokenKind::PlusEqual) {
            Some(BinaryOp::Add)
        } else if self.match_token(&TokenKind::MinusEqual) {
            Some(BinaryOp::Subtract)
        } else if self.match_token(&TokenKind::StarEqual) {
            Some(BinaryOp::Multiply)
        } else if self.match_token(&TokenKind::SlashEqual) {
            Some(BinaryOp::Divide)
        } else if self.match_token(&TokenKind::PercentEqual) {
            Some(BinaryOp::Modulo)
        } else {
            None
        };

        if let Some(binary_op) = op {
            let value = self.parse_assignment()?;
            // Transform `a += b` into `a = a + b`
            return Ok(Expression::Assignment {
                target: Box::new(expr.clone()),
                value: Box::new(Expression::Binary {
                    left: Box::new(expr),
                    op: binary_op,
                    right: Box::new(value),
                }),
            });
        }

        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_and()?;

        while self.match_token(&TokenKind::Or) {
            let right = self.parse_and()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_equality()?;

        while self.match_token(&TokenKind::And) {
            let right = self.parse_equality()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_comparison()?;

        loop {
            let op = if self.match_token(&TokenKind::EqualQuestionDouble) {
                BinaryOp::StrictEqual
            } else if self.match_token(&TokenKind::EqualQuestion) {
                BinaryOp::Equal
            } else if self.match_token(&TokenKind::BangEqualDouble) {
                BinaryOp::StrictNotEqual
            } else if self.match_token(&TokenKind::BangEqual) {
                BinaryOp::NotEqual
            } else {
                break;
            };

            let right = self.parse_comparison()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_bitwise_or()?;

        loop {
            let op = if self.match_token(&TokenKind::Less) {
                BinaryOp::Less
            } else if self.match_token(&TokenKind::LessEqual) {
                BinaryOp::LessEqual
            } else if self.match_token(&TokenKind::Greater) {
                BinaryOp::Greater
            } else if self.match_token(&TokenKind::GreaterEqual) {
                BinaryOp::GreaterEqual
            } else {
                break;
            };

            let right = self.parse_bitwise_or()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_bitwise_xor()?;

        while self.match_token(&TokenKind::Pipe) {
            let right = self.parse_bitwise_xor()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::BitwiseOr,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_bitwise_and()?;

        while self.match_token(&TokenKind::Caret) {
            let right = self.parse_bitwise_and()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::BitwiseXor,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_shift()?;

        while self.match_token(&TokenKind::Ampersand) {
            let right = self.parse_shift()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::BitwiseAnd,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_term()?;

        loop {
            let op = if self.match_token(&TokenKind::ShiftLeft) {
                BinaryOp::ShiftLeft
            } else if self.match_token(&TokenKind::ShiftRight) {
                BinaryOp::ShiftRight
            } else {
                break;
            };

            let right = self.parse_term()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_factor()?;

        loop {
            let op = if self.match_token(&TokenKind::Plus) {
                BinaryOp::Add
            } else if self.match_token(&TokenKind::Minus) {
                BinaryOp::Subtract
            } else {
                break;
            };

            let right = self.parse_factor()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_unary()?;

        loop {
            let op = if self.match_token(&TokenKind::Star) {
                BinaryOp::Multiply
            } else if self.match_token(&TokenKind::Slash) {
                BinaryOp::Divide
            } else if self.match_token(&TokenKind::Percent) {
                BinaryOp::Modulo
            } else {
                break;
            };

            let right = self.parse_unary()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, String> {
        if self.match_token(&TokenKind::Minus) {
            let operand = self.parse_unary()?;
            return Ok(Expression::Unary {
                op: UnaryOp::Negate,
                operand: Box::new(operand),
            });
        }

        if self.match_token(&TokenKind::Not) {
            let operand = self.parse_unary()?;
            return Ok(Expression::Unary {
                op: UnaryOp::Not,
                operand: Box::new(operand),
            });
        }

        if self.match_token(&TokenKind::Tilde) {
            let operand = self.parse_unary()?;
            return Ok(Expression::Unary {
                op: UnaryOp::BitwiseNot,
                operand: Box::new(operand),
            });
        }

        self.parse_call()
    }

    fn parse_call(&mut self) -> Result<Expression, String> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_token(&TokenKind::LeftParen) {
                let mut args = Vec::new();

                if !self.check(&TokenKind::RightParen) {
                    loop {
                        args.push(self.parse_expression()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                }

                self.expect(TokenKind::RightParen)?;

                expr = Expression::Call {
                    callee: Box::new(expr),
                    args,
                };
            } else if self.match_token(&TokenKind::LeftBracket) {
                let index = self.parse_expression()?;
                self.expect(TokenKind::RightBracket)?;

                expr = Expression::Index {
                    array: Box::new(expr),
                    index: Box::new(index),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression, String> {
        let token = self.peek();

        match &token.kind {
            TokenKind::IntegerLiteral(_) | TokenKind::FloatLiteral(_)
            | TokenKind::StringLiteral(_) | TokenKind::BooleanLiteral(_)
            | TokenKind::Identifier(_) => {
                let token = self.advance();
                match token.kind {
                    TokenKind::IntegerLiteral(n) => Ok(Expression::IntegerLiteral(n)),
                    TokenKind::FloatLiteral(f) => Ok(Expression::FloatLiteral(f)),
                    TokenKind::StringLiteral(s) => Ok(Expression::StringLiteral(s)),
                    TokenKind::BooleanLiteral(b) => Ok(Expression::BooleanLiteral(b)),
                    TokenKind::Identifier(name) => Ok(Expression::Identifier(name)),
                    _ => unreachable!(),
                }
            }
            TokenKind::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(TokenKind::RightParen)?;
                Ok(expr)
            }
            TokenKind::LeftBracket => {
                self.advance();
                let mut elements = Vec::new();

                if !self.check(&TokenKind::RightBracket) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                }

                self.expect(TokenKind::RightBracket)?;
                Ok(Expression::ArrayLiteral { elements })
            }
            _ => {
                let t = self.advance();
                Err(format!("Unexpected token: {:?}", t.kind))
            }
        }
    }

    // Helper methods

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.tokens[self.current - 1].clone()
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            return false;
        }
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, String> {
        if self.check(&kind) {
            Ok(self.advance())
        } else {
            Err(format!(
                "Expected {:?}, got {:?} at {}:{}",
                kind,
                self.peek().kind,
                self.peek().line,
                self.peek().column
            ))
        }
    }

    fn expect_identifier(&mut self) -> Result<String, String> {
        let token = self.advance();
        if let TokenKind::Identifier(name) = token.kind {
            Ok(name)
        } else {
            Err(format!(
                "Expected identifier, got {:?} at {}:{}",
                token.kind, token.line, token.column
            ))
        }
    }

    fn consume_semicolon(&mut self) -> Result<(), String> {
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn test_parse_function() {
        let mut lexer = Lexer::new("function main(): i32 { return 0; }");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        assert_eq!(program.statements.len(), 1);
        assert!(matches!(program.statements[0], Statement::FunctionDecl { .. }));
    }

    #[test]
    fn test_parse_variable_decl() {
        let mut lexer = Lexer::new("let x: i32 = 42;");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        assert_eq!(program.statements.len(), 1);
        assert!(matches!(program.statements[0], Statement::VariableDecl { .. }));
    }

    #[test]
    fn test_parse_expressions() {
        let mut lexer = Lexer::new("let result = 2 + 3 * 4;");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let _program = parser.parse().unwrap();
    }
}
