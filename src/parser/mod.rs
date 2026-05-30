use crate::ast::*;
use crate::lexer::{Lexer, Token, TokenKind, Position, LexError};
use std::cell::RefCell;

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

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedToken { expected, found, position } => {
                write!(f, "Unexpected token at line {}, column {}: expected {}, found {:?}",
                       position.line, position.column, expected, found)
            }
            ParseError::UnexpectedEof { expected, position } => {
                write!(f, "Unexpected end of file at line {}, column {}: expected {}",
                       position.line, position.column, expected)
            }
            ParseError::UnsupportedFeature { feature, position, suggestion } => {
                write!(f, "Unsupported feature '{}' at line {}, column {}: {}",
                       feature, position.line, position.column, suggestion)
            }
            ParseError::LexError(lex_error) => {
                write!(f, "Lexer error: {}", lex_error)
            }
        }
    }
}

impl std::error::Error for ParseError {}

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
        // Phase 11a-α: First parse any imports
        let imports = self.parse_imports()?;

        // Expect high/low keyword
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
                let mut program = self.parse_program(mode, mode_token.position)?;
                program.imports = imports;  // Attach imports to program
                Ok(TopLevel::Program(program))
            }
            TokenKind::Module => {
                let mut module = self.parse_module(mode, mode_token.position)?;
                module.imports = imports;  // Attach imports to module
                Ok(TopLevel::Module(module))
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "'program' or 'module'".to_string(),
                found: kind_token.kind,
                position: kind_token.position,
            }),
        }
    }

    fn parse_imports(&mut self) -> Result<Vec<ImportStatement>, ParseError> {
        let mut imports = Vec::new();

        // Parse zero or more import statements
        while self.check(&TokenKind::Import) {
            imports.push(self.parse_import_statement()?);
        }

        Ok(imports)
    }

    fn parse_import_statement(&mut self) -> Result<ImportStatement, ParseError> {
        let import_pos = self.advance()?.position;  // consume 'import'

        // Expect '{'
        self.expect_token(TokenKind::LeftBrace, "Expected '{' after 'import'")?;

        // Parse imported names
        let mut names = Vec::new();

        // Must have at least one name
        let first_name = self.expect_identifier("Expected identifier after '{'")?;
        names.push(first_name.lexeme);

        // Parse additional names separated by commas
        while self.check(&TokenKind::Comma) {
            self.advance()?; // consume ','
            let name = self.expect_identifier("Expected identifier after ','")?;
            names.push(name.lexeme);
        }

        // Expect '}'
        self.expect_token(TokenKind::RightBrace, "Expected '}' after import list")?;

        // Expect 'from'
        self.expect_token(TokenKind::From, "Expected 'from' after import list")?;

        // Expect string literal path
        let path_token = self.advance()?;
        let path = match path_token.kind {
            TokenKind::StringLit(s) => s,
            _ => return Err(ParseError::UnexpectedToken {
                expected: "string literal".to_string(),
                found: path_token.kind,
                position: path_token.position,
            }),
        };

        Ok(ImportStatement {
            names,
            path,
            position: import_pos,
        })
    }

    fn parse_program(&mut self, mode: Mode, start_pos: Position) -> Result<Program, ParseError> {
        // Parse parameter list
        self.expect_token(TokenKind::LeftParen, "Expected '(' after 'program'")?;
        let params = self.parse_parameter_list()?;
        self.expect_token(TokenKind::RightParen, "Expected ')' after parameter list")?;

        // Parse return type
        self.expect_token(TokenKind::Colon, "Expected ':' after parameter list")?;
        let return_type = self.parse_type()?;

        // Parse body - Phase 6a-fixup: parse statements and nested functions
        let body = self.parse_program_body(mode.clone())?;

        // Expect EOF - Phase 11a-α: check for imports after program block
        if self.check(&TokenKind::Import) {
            return Err(ParseError::UnsupportedFeature {
                feature: "import after program block".to_string(),
                position: self.peek()?.position.clone(),
                suggestion: "'import' statements must appear before the program or module block".to_string(),
            });
        }
        self.expect_token(TokenKind::Eof, "Expected end of file")?;

        Ok(Program {
            mode,
            params,
            return_type,
            body: Some(body),
            imports: vec![],  // Phase 11a-α: empty imports list (parser doesn't handle imports yet)
            position: start_pos,
        })
    }

    fn parse_module(&mut self, mode: Mode, start_pos: Position) -> Result<Module, ParseError> {
        // Parse module body
        self.expect_token(TokenKind::LeftBrace, "Expected '{' after 'module'")?;

        let mut items = Vec::new();
        let mut lets = Vec::new();
        let mut watchers = Vec::new();

        // Parse declarations until we hit the closing brace
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Check for export modifier
            let is_export = if self.check(&TokenKind::Export) {
                self.advance()?; // consume 'export'
                true
            } else {
                false
            };

            // Determine what kind of declaration this is
            if self.check(&TokenKind::Function) {
                // Function declaration
                let mut function = self.parse_function_signature(mode.clone())?;
                function.is_export = is_export;
                items.push(function);
            } else if self.check(&TokenKind::Watcher) {
                // Watcher declaration
                let mut watcher = self.parse_watcher_signature(mode.clone())?;
                watcher.is_export = is_export;
                watchers.push(watcher);
            } else if self.check(&TokenKind::High) || self.check(&TokenKind::Low) {
                // Mode-prefixed declaration - peek ahead to see if it's function or watcher
                match self.peek_ahead(1)?.kind {
                    TokenKind::Function => {
                        let mut function = self.parse_function_signature(mode.clone())?;
                        function.is_export = is_export;
                        items.push(function);
                    }
                    TokenKind::Watcher => {
                        let mut watcher = self.parse_watcher_signature(mode.clone())?;
                        watcher.is_export = is_export;
                        watchers.push(watcher);
                    }
                    _ => {
                        let token = self.peek()?;
                        return Err(ParseError::UnexpectedToken {
                            expected: "'function' or 'watcher' after mode keyword".to_string(),
                            found: token.kind.clone(),
                            position: token.position.clone(),
                        });
                    }
                }
            } else if self.check(&TokenKind::Let) {
                // Let declaration
                if is_export {
                    let let_decl = self.parse_module_let_declaration(true)?;
                    lets.push(let_decl);
                } else {
                    let let_decl = self.parse_module_let_declaration(false)?;
                    lets.push(let_decl);
                }
            } else {
                // Invalid declaration in module body
                let token = self.peek()?;
                return Err(ParseError::UnsupportedFeature {
                    feature: "non-declaration statement in module body".to_string(),
                    position: token.position.clone(),
                    suggestion: "module body may only contain declarations".to_string(),
                });
            }
        }

        self.expect_token(TokenKind::RightBrace, "Expected '}' after module body")?;

        // Phase 11a-α: check for imports after module block
        if self.check(&TokenKind::Import) {
            return Err(ParseError::UnsupportedFeature {
                feature: "import after module block".to_string(),
                position: self.peek()?.position.clone(),
                suggestion: "'import' statements must appear before the program or module block".to_string(),
            });
        }
        self.expect_token(TokenKind::Eof, "Expected end of file")?;

        Ok(Module {
            mode,
            items,
            lets,
            watchers,
            imports: vec![],  // Phase 11a-α: imports are attached from parse_top_level
            position: start_pos,
        })
    }

    fn parse_module_let_declaration(&mut self, is_export: bool) -> Result<LetDecl, ParseError> {
        let start_pos = self.advance()?.position; // consume 'let'

        // Parse the let pattern (identifier or destructuring)
        let pattern = self.parse_let_pattern(&start_pos)?;

        // Optional type annotation
        let pattern = if self.check(&TokenKind::Colon) {
            self.advance()?; // consume ':'
            let ty = self.parse_type()?;
            match pattern {
                LetPattern::Identifier(name, _) => LetPattern::Identifier(name, Some(ty)),
                LetPattern::Tuple(_) => {
                    return Err(ParseError::UnsupportedFeature {
                        feature: "type annotations on tuple destructuring".to_string(),
                        position: start_pos,
                        suggestion: "tuple destructuring with type annotations not yet supported".to_string(),
                    });
                }
            }
        } else {
            pattern
        };

        // Required initializer for module-level lets
        self.expect_token(TokenKind::Equal, "Expected '=' after let declaration in module")?;
        let initializer = Some(self.parse_expression()?);

        Ok(LetDecl {
            pattern,
            initializer,
            is_export,
            position: start_pos,
        })
    }

    fn parse_let_pattern(&mut self, start_pos: &Position) -> Result<LetPattern, ParseError> {
        // Parse the pattern - either identifier or tuple destructuring
        if self.check(&TokenKind::LeftParen) {
            // Tuple destructuring: let (a, b, c) = ...
            self.advance()?; // consume '('

            let mut names = Vec::new();

            // Parse first identifier
            let first_name = self.expect_identifier("Expected variable name in tuple pattern")?;
            names.push(first_name.lexeme);

            // Parse remaining identifiers
            while self.check(&TokenKind::Comma) {
                self.advance()?; // consume ','
                let name = self.expect_identifier("Expected variable name in tuple pattern")?;
                names.push(name.lexeme);
            }

            self.expect_token(TokenKind::RightParen, "Expected ')' after tuple pattern")?;

            if names.len() < 2 {
                return Err(ParseError::UnexpectedToken {
                    expected: "at least 2 elements in tuple pattern".to_string(),
                    found: TokenKind::RightParen,
                    position: start_pos.clone(),
                });
            }

            Ok(LetPattern::Tuple(names))
        } else {
            // Regular identifier: let name = ...
            let name_token = self.expect_identifier("Expected variable name after 'let'")?;
            let name = name_token.lexeme;

            // Optional type annotation
            let ty = if self.check(&TokenKind::Colon) {
                self.advance()?; // consume ':'
                Some(self.parse_type()?)
            } else {
                None
            };

            Ok(LetPattern::Identifier(name, ty))
        }
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

    fn peek_ahead(&self, offset: usize) -> Result<&Token, ParseError> {
        let index = self.current + offset;
        if index >= self.tokens.len() {
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
        Ok(&self.tokens[index])
    }

    fn parse_watcher_signature(&mut self, default_mode: Mode) -> Result<Watcher, ParseError> {
        let start_pos = self.peek()?.position.clone();
        let mut is_export = false;

        // Check for export keyword
        if self.check(&TokenKind::Export) {
            is_export = true;
            self.advance()?;
        }

        // Check for explicit mode
        let watcher_mode = if self.check(&TokenKind::High) || self.check(&TokenKind::Low) {
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

        // Expect 'watcher' keyword
        self.expect_token(TokenKind::Watcher, "Expected 'watcher'")?;

        // Parse watcher name
        let name_token = self.expect_identifier("Expected watcher name")?;
        let name = name_token.lexeme;

        // Parse subscription list
        self.expect_token(TokenKind::LeftParen, "Expected '(' after watcher name")?;
        let subscriptions = self.parse_subscription_list()?;
        self.expect_token(TokenKind::RightParen, "Expected ')' after subscription list")?;

        // No return type for watchers (unlike functions)
        // Parse body
        let body = self.parse_block()?;

        Ok(Watcher {
            name,
            mode: watcher_mode,
            subscriptions,
            body,
            is_export,
            position: start_pos,
        })
    }

    fn parse_subscription_list(&mut self) -> Result<Vec<Subscription>, ParseError> {
        let mut subscriptions = Vec::new();

        // Handle empty subscription list - this is an error for watchers
        if self.check(&TokenKind::RightParen) {
            let pos = self.peek()?.position.clone();
            return Err(ParseError::UnsupportedFeature {
                feature: "empty watcher subscription list".to_string(),
                position: pos,
                suggestion: "provide at least one subscription".to_string(),
            });
        }

        // Parse first subscription
        subscriptions.push(self.parse_subscription()?);

        // Parse additional subscriptions
        while self.check(&TokenKind::Comma) {
            self.advance()?; // consume comma
            subscriptions.push(self.parse_subscription()?);
        }

        // Check for duplicate variable/modifier combinations
        let mut seen = std::collections::HashSet::new();
        for subscription in &subscriptions {
            let key = (&subscription.variable_name, &subscription.modifier);
            if seen.contains(&key) {
                return Err(ParseError::UnexpectedToken {
                    expected: "unique subscription".to_string(),
                    found: TokenKind::Identifier, // The duplicate variable name
                    position: subscription.position.clone(),
                });
            }
            seen.insert(key);
        }

        Ok(subscriptions)
    }

    fn parse_subscription(&mut self) -> Result<Subscription, ParseError> {
        let start_pos = self.peek()?.position.clone();
        let mut modifier = SubscriptionModifier::Changed; // default
        let mut alias = None;

        // Check for modifier: (modifier)variable or (alias=modifier)variable
        if self.check(&TokenKind::LeftParen) {
            self.advance()?; // consume '('

            // Parse the modifier part
            let first_ident = self.expect_identifier("Expected modifier or alias in subscription")?;

            if self.check(&TokenKind::Equal) {
                // This is alias=modifier form
                alias = Some(first_ident.lexeme);
                self.advance()?; // consume '='
                let modifier_ident = self.expect_identifier("Expected modifier after '=' in subscription")?;
                modifier = self.parse_subscription_modifier(&modifier_ident.lexeme, &modifier_ident.position)?;
            } else {
                // This is just modifier form
                modifier = self.parse_subscription_modifier(&first_ident.lexeme, &first_ident.position)?;
            }

            self.expect_token(TokenKind::RightParen, "Expected ')' after subscription modifier")?;
        }

        // Parse variable name
        let var_token = self.expect_identifier("Expected variable name in subscription")?;
        let variable_name = var_token.lexeme;

        Ok(Subscription {
            variable_name,
            modifier,
            alias,
            position: start_pos,
            resolved_var_type: RefCell::new(None),
            resolved_alias_type: RefCell::new(None),
        })
    }

    fn parse_subscription_modifier(&self, name: &str, position: &Position) -> Result<SubscriptionModifier, ParseError> {
        match name {
            "changed" => Ok(SubscriptionModifier::Changed),
            "assigned" => Ok(SubscriptionModifier::Assigned),
            "added" => Ok(SubscriptionModifier::Added),
            "removed" => Ok(SubscriptionModifier::Removed),
            "moved" => Ok(SubscriptionModifier::Moved),
            _ => Err(ParseError::UnsupportedFeature {
                feature: format!("subscription modifier '{}'", name),
                position: position.clone(),
                suggestion: "valid modifiers are: changed, assigned, added, removed, moved".to_string(),
            })
        }
    }

    fn parse_watcher_expression(&mut self, start_pos: Position) -> Result<Expression, ParseError> {
        // Expect 'watcher' keyword (already consumed by caller)

        // Parse subscription list
        self.expect_token(TokenKind::LeftParen, "Expected '(' after 'watcher' in expression")?;
        let subscriptions = self.parse_subscription_list()?;
        self.expect_token(TokenKind::RightParen, "Expected ')' after subscription list in watcher expression")?;

        // Parse body
        let body = self.parse_block()?;

        Ok(Expression::WatcherExpr(WatcherExpr {
            subscriptions,
            body,
            position: start_pos,
            captures: RefCell::new(Vec::new()),
        }))
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        let base_type = if self.check(&TokenKind::LeftBracket) {
            self.parse_array_type()?
        } else if self.check(&TokenKind::LeftParen) {
            // Could be tuple type (T1, T2, ...) or parenthesized type (T)
            self.parse_tuple_or_parenthesized_type()?
        } else if self.check(&TokenKind::Star) {
            // Pointer types not supported in Phase 2a
            let pos = self.peek()?.position.clone();
            return Err(ParseError::UnsupportedFeature {
                feature: "pointer types".to_string(),
                position: pos,
                suggestion: "pointers not yet supported (Phase 12)".to_string(),
            })
        } else if self.check(&TokenKind::Function) {
            self.advance()?; // consume 'function'

            // Check for parameterized function type: function(param_types): return_type
            if self.check(&TokenKind::LeftParen) {
                self.advance()?; // consume '('

                let mut param_types = Vec::new();

                // Parse parameter types (if any)
                if !self.check(&TokenKind::RightParen) {
                    loop {
                        param_types.push(self.parse_type()?);

                        if self.check(&TokenKind::Comma) {
                            self.advance()?; // consume ','
                        } else {
                            break;
                        }
                    }
                }

                self.expect_token(TokenKind::RightParen, "Expected ')' after function parameter types")?;
                self.expect_token(TokenKind::Colon, "Expected ':' after function parameter types")?;

                let return_type = Box::new(self.parse_type()?);

                Type::Function(param_types, return_type)
            } else {
                // Placeholder function type for backward compatibility
                Type::Function(vec![], Box::new(Type::Primitive(PrimitiveType::Nothing)))
            }
        } else if self.check(&TokenKind::Watcher) {
               self.advance()?; // consume 'watcher'
               Type::Watcher
        } else {
            self.parse_primitive_type()?
        };

        // Check for optional syntax: T?
        if self.check(&TokenKind::Question) {
            self.advance()?; // consume '?'
            Ok(Type::Optional(Box::new(base_type)))
        } else {
            Ok(base_type)
        }
    }

    fn parse_tuple_or_parenthesized_type(&mut self) -> Result<Type, ParseError> {
        self.advance()?; // consume '('

        // Parse first type
        let first_type = self.parse_type()?;

        if self.check(&TokenKind::Comma) {
            // This is a tuple type: (T1, T2, ...)
            let mut types = vec![first_type];

            while self.check(&TokenKind::Comma) {
                self.advance()?; // consume ','
                types.push(self.parse_type()?);
            }

            self.expect_token(TokenKind::RightParen, "Expected ')' after tuple type")?;

            if types.len() < 2 {
                return Err(ParseError::UnexpectedToken {
                    expected: "at least 2 elements in tuple type".to_string(),
                    found: TokenKind::RightParen,
                    position: self.current_position(),
                });
            }

            Ok(Type::Tuple(types))
        } else {
            // This is a parenthesized type: (T)
            self.expect_token(TokenKind::RightParen, "Expected ')' after parenthesized type")?;
            Ok(first_type)
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
                    "unknown" => PrimitiveType::Unknown,
                    "time" => PrimitiveType::Time,
                    "duration" => PrimitiveType::Duration,
                    "money" => {
                        // Check for parameterized money type: money<USD>
                        if self.check(&TokenKind::Less) {
                            self.advance()?; // consume '<'

                            // Expect currency identifier
                            let currency_token = self.advance()?;
                            let currency = match &currency_token.kind {
                                TokenKind::USD => "USD".to_string(),
                                TokenKind::EUR => "EUR".to_string(),
                                TokenKind::GBP => "GBP".to_string(),
                                TokenKind::JPY => "JPY".to_string(),
                                TokenKind::CAD => "CAD".to_string(),
                                TokenKind::AUD => "AUD".to_string(),
                                TokenKind::CHF => "CHF".to_string(),
                                TokenKind::CNY => "CNY".to_string(),
                                _ => {
                                    return Err(ParseError::UnexpectedToken {
                                        expected: "currency code".to_string(),
                                        found: currency_token.kind.clone(),
                                        position: currency_token.position,
                                    });
                                }
                            };

                            self.expect_token(TokenKind::Greater, "Expected '>' after currency")?;

                            // Return parameterized money type directly (not wrapped in Primitive)
                            return Ok(Type::MoneyOf(currency));
                        } else {
                            // Regular money type
                            PrimitiveType::Money
                        }
                    },
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
            TokenKind::Unknown => PrimitiveType::Unknown,
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

        let mut items = Vec::new();

        // Skip any leading semicolons
        self.skip_semicolons();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            match &self.peek()?.kind {
                TokenKind::Function => {
                    // Parse nested function definition (default to High mode for now)
                    let function = self.parse_function_signature(Mode::High)?;
                    items.push(BlockItem::Function(function));
                }
                TokenKind::Watcher => {
                    // Parse nested watcher definition (default to High mode for now)
                    let watcher = self.parse_watcher_signature(Mode::High)?;
                    items.push(BlockItem::Watcher(watcher));
                }
                TokenKind::High | TokenKind::Low => {
                    // Peek ahead to see if it's followed by function or watcher
                    match self.peek_ahead(1)?.kind {
                        TokenKind::Function => {
                            let function = self.parse_function_signature(Mode::High)?;
                            items.push(BlockItem::Function(function));
                        }
                        TokenKind::Watcher => {
                            let watcher = self.parse_watcher_signature(Mode::High)?;
                            items.push(BlockItem::Watcher(watcher));
                        }
                        _ => {
                            // Not a function or watcher, treat as statement
                            let statement = self.parse_statement()?;
                            items.push(BlockItem::Statement(statement));
                        }
                    }
                }
                _ => {
                    // Parse statement
                    let statement = self.parse_statement()?;
                    items.push(BlockItem::Statement(statement));
                }
            }
            // Skip any trailing semicolons after each item
            self.skip_semicolons();
        }

        self.expect_token(TokenKind::RightBrace, "Expected '}'")?;

        Ok(Block { items, position })
    }

    fn parse_program_body(&mut self, mode: Mode) -> Result<ProgramBody, ParseError> {
        let start_token = self.expect_token(TokenKind::LeftBrace, "Expected '{'")?;
        let position = start_token.position;

        let mut items = Vec::new();

        // Skip any leading semicolons
        self.skip_semicolons();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            match &self.peek()?.kind {
                TokenKind::Function => {
                    // Parse nested function definition (inherits mode from program/module)
                    let function = self.parse_function_signature(mode.clone())?;
                    items.push(BlockItem::Function(function));
                }
                TokenKind::Watcher => {
                    // Parse nested watcher definition (inherits mode from program/module)
                    let watcher = self.parse_watcher_signature(mode.clone())?;
                    items.push(BlockItem::Watcher(watcher));
                }
                TokenKind::High | TokenKind::Low => {
                    // Peek ahead to see if it's followed by function or watcher
                    match self.peek_ahead(1)?.kind {
                        TokenKind::Function => {
                            let function = self.parse_function_signature(mode.clone())?;
                            items.push(BlockItem::Function(function));
                        }
                        TokenKind::Watcher => {
                            let watcher = self.parse_watcher_signature(mode.clone())?;
                            items.push(BlockItem::Watcher(watcher));
                        }
                        _ => {
                            // Not a function or watcher, treat as statement
                            let statement = self.parse_statement()?;
                            items.push(BlockItem::Statement(statement));
                        }
                    }
                }
                _ => {
                    // Parse statement
                    let statement = self.parse_statement()?;
                    items.push(BlockItem::Statement(statement));
                }
            }
            // Skip any trailing semicolons after each item
            self.skip_semicolons();
        }

        self.expect_token(TokenKind::RightBrace, "Expected '}'")?;

        Ok(ProgramBody { items, position })
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        // Phase 11a-α: Check for export modifier in program body (which is invalid)
        if self.check(&TokenKind::Export) {
            return Err(ParseError::UnsupportedFeature {
                feature: "export in program body".to_string(),
                position: self.peek()?.position.clone(),
                suggestion: "'export' is only valid inside a module body".to_string(),
            });
        }

        match &self.peek()?.kind {
            TokenKind::Let => self.parse_let_statement(),
            TokenKind::Return => self.parse_return_statement(),
            TokenKind::If => self.parse_if_statement(),
            TokenKind::While => self.parse_while_statement(),
            TokenKind::Loop => self.parse_loop_statement(),
            TokenKind::For => self.parse_for_in_statement(),
            TokenKind::Switch => self.parse_switch_statement(),
            TokenKind::Break => self.parse_break_statement(),
            TokenKind::Continue => self.parse_continue_statement(),
            TokenKind::Stealth => self.parse_stealth_statement(),
            _ => {
                // Try to parse assignment or expression statement
                let checkpoint = self.current;

                // Try regular assignment first
                if let Ok(assignment) = self.try_parse_assignment() {
                    return Ok(Statement::Assign(assignment));
                }

                // Reset and try qualified assignment
                self.current = checkpoint;
                if let Ok(qualified_op) = self.try_parse_qualified_assignment() {
                    return Ok(Statement::QualifiedOp(qualified_op));
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

        // Parse the let pattern using the shared method
        let pattern = self.parse_let_pattern(&start_pos)?;

        // Optional initializer
        let initializer = if self.check(&TokenKind::Equal) {
            self.advance()?; // consume '='
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::Let(LetDecl {
            pattern,
            initializer,
            is_export: false,  // Phase 11a-α: let statements in program body are never exported
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
                        items: vec![BlockItem::Statement(Statement::If(if_stmt))],
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

    fn parse_stealth_statement(&mut self) -> Result<Statement, ParseError> {
        let position = self.advance()?.position;  // consume 'stealth'
        let block = self.parse_block()?;
        Ok(Statement::StealthBlock(block, position))
    }

    fn parse_for_in_statement(&mut self) -> Result<Statement, ParseError> {
        let start_pos = self.advance()?.position; // consume 'for'

        self.expect_token(TokenKind::LeftParen, "Expected '(' after 'for'")?;
        self.expect_token(TokenKind::Let, "Expected 'let' in for-in loop")?;

        // Expect tuple destructuring: (key, value)
        self.expect_token(TokenKind::LeftParen, "Expected '(' for tuple destructuring")?;

        let key_name_token = self.expect_identifier("Expected key variable name")?;
        let key_name = key_name_token.lexeme;

        self.expect_token(TokenKind::Comma, "Expected ',' between key and value")?;

        let value_name_token = self.expect_identifier("Expected value variable name")?;
        let value_name = value_name_token.lexeme;

        self.expect_token(TokenKind::RightParen, "Expected ')' after tuple destructuring")?;

        self.expect_token(TokenKind::In, "Expected 'in' after tuple destructuring")?;

        let iterable = self.parse_expression()?;

        self.expect_token(TokenKind::RightParen, "Expected ')' after for-in expression")?;

        let body = self.parse_block()?;

        Ok(Statement::ForIn(ForInStmt {
            key_name,
            value_name,
            iterable,
            body,
            position: start_pos,
        }))
    }

    fn parse_switch_statement(&mut self) -> Result<Statement, ParseError> {
        let start_pos = self.advance()?.position; // consume 'switch'

        self.expect_token(TokenKind::LeftParen, "Expected '(' after 'switch'")?;
        let value = self.parse_expression()?;
        self.expect_token(TokenKind::RightParen, "Expected ')' after switch expression")?;

        self.expect_token(TokenKind::LeftBrace, "Expected '{' to start switch body")?;

        let mut cases = Vec::new();
        let mut default = None;

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            match &self.peek()?.kind {
                TokenKind::Case => {
                    let case_pos = self.advance()?.position; // consume 'case'

                    let pattern = self.parse_literal()?;

                    self.expect_token(TokenKind::Colon, "Expected ':' after case pattern")?;

                    let mut body = Vec::new();
                    while !self.check(&TokenKind::Case) && !self.check(&TokenKind::Default) && !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
                        body.push(self.parse_statement()?);
                    }

                    cases.push(SwitchCase {
                        pattern,
                        body,
                        position: case_pos,
                    });
                }
                TokenKind::Default => {
                    if default.is_some() {
                        return Err(ParseError::UnexpectedToken {
                            expected: "only one default clause".to_string(),
                            found: TokenKind::Default,
                            position: self.peek()?.position.clone(),
                        });
                    }

                    self.advance()?; // consume 'default'
                    self.expect_token(TokenKind::Colon, "Expected ':' after 'default'")?;

                    let mut body = Vec::new();
                    while !self.check(&TokenKind::Case) && !self.check(&TokenKind::Default) && !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
                        body.push(self.parse_statement()?);
                    }

                    default = Some(body);
                }
                _ => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "'case' or 'default'".to_string(),
                        found: self.peek()?.kind.clone(),
                        position: self.peek()?.position.clone(),
                    });
                }
            }
        }

        self.expect_token(TokenKind::RightBrace, "Expected '}' to close switch body")?;

        Ok(Statement::Switch(SwitchStmt {
            value,
            cases,
            default,
            position: start_pos,
        }))
    }

    fn parse_literal(&mut self) -> Result<Literal, ParseError> {
        let token = self.advance()?;
        match token.kind {
            TokenKind::Integer(n) => Ok(Literal::Integer(n)),
            TokenKind::Float(f) => Ok(Literal::Float(f)),
            TokenKind::StringLit(s) => Ok(Literal::String(s)),
            TokenKind::True => Ok(Literal::Bool(true)),
            TokenKind::False => Ok(Literal::Bool(false)),
            _ => Err(ParseError::UnexpectedToken {
                expected: "literal value".to_string(),
                found: token.kind,
                position: token.position,
            })
        }
    }

    fn try_parse_assignment(&mut self) -> Result<AssignStmt, ParseError> {
        let target = self.parse_postfix_expression()?;

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

    fn try_parse_qualified_assignment(&mut self) -> Result<QualifiedOp, ParseError> {
        // Parse left-hand side (qualified assignments expect simple identifiers)
        let lhs = self.parse_primary_expression()?;

        // Check if this looks like a qualified operator (next token should be '(')
        if !self.check(&TokenKind::LeftParen) {
            return Err(ParseError::UnexpectedToken {
                expected: "'(' for qualified assignment".to_string(),
                found: self.peek()?.kind.clone(),
                position: self.peek()?.position.clone(),
            });
        }

        // Parse the qualified operator, but force it to be an assignment
        let position = self.peek()?.position.clone();
        self.advance()?; // consume '('

        // Parse qualifier list
        let qualifiers = self.parse_qualifier_list()?;

        self.expect_token(TokenKind::RightParen, "Expected ')' after qualifier list")?;

        // Parse operator - in assignment context, only = is valid
        let op = match self.advance()?.kind {
            TokenKind::Equal => QualifiedOpKind::Assign,
            found => return Err(ParseError::UnexpectedToken {
                expected: "'=' for qualified assignment".to_string(),
                found,
                position,
            }),
        };

        // Parse right-hand side
        let rhs = self.parse_expression_with_precedence(5)?;

        Ok(QualifiedOp {
            lhs: Box::new(lhs),
            qualifiers,
            op,
            rhs: Box::new(rhs),
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

                let _op_kind = op_token.kind.clone(); // Clone for potential error message
                let position = self.advance()?.position; // consume operator

                // Special handling for 'is' and 'is not' to create IsCheck or ObjectIsCheck nodes
                if matches!(op, BinaryOpKind::Is | BinaryOpKind::IsNot) {
                    // Peek at next token to decide between primitive type check vs object prototype check
                    if !self.is_at_end() {
                        let token = self.peek()?;
                        if matches!(token.kind, TokenKind::Identifier | TokenKind::Nothing | TokenKind::Unknown) {
                            let type_name = token.lexeme.clone(); // Clone to avoid borrow conflict
                            // Check if it's a primitive type name
                            let is_primitive_type = matches!(type_name.as_str(),
                                "i8" | "i16" | "i32" | "i64" | "i128" |
                                "u8" | "u16" | "u32" | "u64" | "u128" |
                                "f32" | "f64" | "bool" | "string" |
                                "usize" | "isize" | "nothing" | "unknown"
                            );

                            if is_primitive_type {
                                // Parse as primitive type check
                                let _type_token = self.advance()?;
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
                                    "unknown" => Type::Primitive(PrimitiveType::Unknown),
                                    _ => unreachable!(),
                                };

                                left = Expression::IsCheck(IsCheck {
                                    expression: Box::new(left),
                                    ty,
                                    negated: matches!(op, BinaryOpKind::IsNot),
                                    position,
                                });
                            } else {
                                // Parse as object prototype check
                                let right = self.parse_expression_with_precedence(prec + 1)?;

                                left = Expression::ObjectIsCheck(ObjectIsCheck {
                                    lhs: Box::new(left),
                                    rhs: Box::new(right),
                                    negated: matches!(op, BinaryOpKind::IsNot),
                                    position,
                                });
                            }
                        } else {
                            // Parse as object prototype check (RHS is not an identifier)
                            let right = self.parse_expression_with_precedence(prec + 1)?;

                            left = Expression::ObjectIsCheck(ObjectIsCheck {
                                lhs: Box::new(left),
                                rhs: Box::new(right),
                                negated: matches!(op, BinaryOpKind::IsNot),
                                position,
                            });
                        }
                    } else {
                        return Err(ParseError::UnexpectedEof {
                            expected: "type name or expression after 'is'".to_string(),
                            position: position.clone(),
                        });
                    }
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
                    // Check if this is a qualified operator or function call
                    if self.is_qualified_operator(self.current + 1) {
                        // Parse as qualified operator
                        expr = self.parse_qualified_operator(expr)?;
                    } else {
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
                }
                TokenKind::Dot => {
                    // Member access or tuple field access
                    let position = self.advance()?.position; // consume '.'

                    // Check if the next token is an integer (tuple field access) or identifier (member access)
                    let next_token = self.peek()?;
                    match &next_token.kind {
                        TokenKind::Integer(index) => {
                            // Tuple field access: expr.0, expr.1, etc.
                            let index_val = *index;
                            let index_pos = next_token.position.clone();
                            self.advance()?; // consume the integer

                            if index_val < 0 {
                                return Err(ParseError::UnexpectedToken {
                                    expected: "non-negative integer".to_string(),
                                    found: TokenKind::Integer(index_val),
                                    position: index_pos,
                                });
                            }

                            expr = Expression::TupleAccess(Box::new(expr), index_val as usize, position);
                        }
                        TokenKind::Identifier => {
                            // Regular member access
                            let member_token = self.expect_identifier("Expected member name after '.'")?;

                            expr = Expression::MemberAccess(MemberAccess {
                                object: Box::new(expr),
                                member: member_token.lexeme,
                                position,
                            });
                        }
                        _ => {
                            return Err(ParseError::UnexpectedToken {
                                expected: "member name or numeric index".to_string(),
                                found: next_token.kind.clone(),
                                position: next_token.position.clone(),
                            });
                        }
                    }
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
                TokenKind::Colon => {
                    // Check if the token after the colon could start a type
                    // If not, this colon isn't for type ascription (e.g., f-string format spec)
                    if let Ok(next_token) = self.peek_ahead(1) {
                        match next_token.kind {
                            // Valid type starters
                            TokenKind::Identifier |
                            TokenKind::LeftBracket |  // [Type]
                            TokenKind::LeftParen |    // (Type, Type)
                            TokenKind::LeftBrace |    // {field: Type}
                            TokenKind::At => {        // @Type
                                let position = self.advance()?.position; // consume ':'
                                let ty = self.parse_type()?;             // reuse existing type parser
                                expr = Expression::TypeAscription(Box::new(expr), ty, position);
                            }
                            // Invalid type starters - don't consume the colon
                            _ => break,
                        }
                    } else {
                        // Can't peek ahead - don't consume the colon
                        break;
                    }
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
            TokenKind::StringLit(s) => Ok(Expression::StringLit(s, token.position)),
            TokenKind::DurationLiteral(nanos, unit) => Ok(Expression::DurationLit(nanos, unit, token.position)),
            TokenKind::MoneyLit(micro_units, currency) => Ok(Expression::MoneyLit(micro_units, currency, token.position)),
            TokenKind::FStringStart => self.parse_f_string(token.position),
            TokenKind::True => Ok(Expression::BoolLit(true, token.position)),
            TokenKind::False => Ok(Expression::BoolLit(false, token.position)),
            TokenKind::Nothing => Ok(Expression::Nothing(token.position)),
            TokenKind::Identifier => {
                Ok(Expression::Ident { name: token.lexeme, refined_type: None, position: token.position })
            }
            TokenKind::This => {
                Ok(Expression::This(token.position))
            }
            TokenKind::LeftParen => {
                // Could be parenthesized expression (expr) or tuple literal (expr1, expr2, ...)
                self.parse_tuple_or_parenthesized_expression(token.position)
            }
            TokenKind::LeftBracket => {
                // Array literal [expr1, expr2, ...]
                self.parse_array_literal(token.position)
            }
            TokenKind::LeftBrace => {
                // Object literal (in expression position)
                self.parse_object_literal(token.position)
            }
            TokenKind::Function => {
                // Function expression
                self.parse_function_expression(token.position)
            }
            TokenKind::Watcher => {
                // Watcher expression
                self.parse_watcher_expression(token.position)
            }
            TokenKind::Match => {
                // Match expression
                self.parse_match_expression(token.position)
            }
            TokenKind::Weak => {
                // Weak reference expression
                let expr = self.parse_unary_expression()?;
                Ok(Expression::WeakRef(Box::new(expr), token.position))
            }
            TokenKind::Unknown => {
                // Unknown constructor: unknown(reason, options: [...])
                self.parse_unknown_constructor(token.position)
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "expression".to_string(),
                found: token.kind,
                position: token.position,
            }),
        }
    }

    fn parse_tuple_or_parenthesized_expression(&mut self, start_pos: Position) -> Result<Expression, ParseError> {
        // Parse the first expression
        let first_expr = self.parse_expression()?;

        if self.check(&TokenKind::Comma) {
            // This is a tuple literal: (expr1, expr2, ...)
            let mut expressions = vec![first_expr];

            while self.check(&TokenKind::Comma) {
                self.advance()?; // consume ','
                expressions.push(self.parse_expression()?);
            }

            self.expect_token(TokenKind::RightParen, "Expected ')' after tuple literal")?;

            if expressions.len() < 2 {
                return Err(ParseError::UnexpectedToken {
                    expected: "at least 2 elements in tuple literal".to_string(),
                    found: TokenKind::RightParen,
                    position: start_pos,
                });
            }

            Ok(Expression::TupleLit(expressions, start_pos))
        } else {
            // This is a parenthesized expression: (expr)
            self.expect_token(TokenKind::RightParen, "Expected ')' after expression")?;
            Ok(first_expr)
        }
    }

    fn parse_array_literal(&mut self, start_pos: Position) -> Result<Expression, ParseError> {
        let mut elements = Vec::new();

        // Check for empty array
        if self.check(&TokenKind::RightBracket) {
            // Empty array - let type checker handle validation
        } else {
            // Parse first element
            elements.push(self.parse_expression()?);

            // Parse remaining elements
            while self.check(&TokenKind::Comma) {
            self.advance()?; // consume ','

            // Allow trailing comma
            if self.check(&TokenKind::RightBracket) {
                break;
            }

            elements.push(self.parse_expression()?);
        }
        }

        self.expect_token(TokenKind::RightBracket, "Expected ']' after array literal")?;

        Ok(Expression::ArrayLit(elements, start_pos))
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

    fn expect_qualifier_name(&mut self, message: &str) -> Result<Token, ParseError> {
        let token = self.advance()?;
        match token.kind {
            TokenKind::Identifier => Ok(token),
            // Allow specific keywords to be used as qualifier names
            TokenKind::Or => Ok(Token {
                kind: TokenKind::Identifier,
                lexeme: "or".to_string(),
                position: token.position,
            }),
            TokenKind::And => Ok(Token {
                kind: TokenKind::Identifier,
                lexeme: "and".to_string(),
                position: token.position,
            }),
            // Add other keywords that can be qualifiers
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

    fn current_position(&self) -> Position {
        if self.is_at_end() {
            if self.tokens.is_empty() {
                Position { line: 1, column: 1 }
            } else {
                self.tokens[self.tokens.len() - 1].position.clone()
            }
        } else {
            self.tokens[self.current].position.clone()
        }
    }

    fn previous(&self) -> Result<&Token, ParseError> {
        if self.current == 0 {
            return Err(ParseError::UnexpectedEof {
                expected: "previous token".to_string(),
                position: Position { line: 1, column: 1 },
            });
        }
        Ok(&self.tokens[self.current - 1])
    }

    fn skip_semicolons(&mut self) {
        while !self.is_at_end() && self.check(&TokenKind::Semicolon) {
            self.current += 1; // consume semicolon
        }
    }

    // Helper to check if a parenthesized expression after `expr (` is a qualified operator
    // Returns true if this should be parsed as a qualified operator (ends with = or !=)
    fn is_qualified_operator(&self, start_pos: usize) -> bool {
        let mut pos = start_pos; // start_pos should be the position AFTER the opening '('
        let mut paren_depth = 1;

        // Find the matching closing parenthesis
        while pos < self.tokens.len() && paren_depth > 0 {
            match self.tokens[pos].kind {
                TokenKind::LeftParen => paren_depth += 1,
                TokenKind::RightParen => paren_depth -= 1,
                _ => {}
            }
            pos += 1;
        }

        // If we didn't find matching paren, treat as function call
        if paren_depth > 0 {
            return false;
        }

        // Check what comes after the closing paren
        if pos < self.tokens.len() {
            matches!(self.tokens[pos].kind, TokenKind::Equal | TokenKind::NotEq)
        } else {
            false
        }
    }

    // Parse a list of qualifier specs: ident or ident: expr
    fn parse_qualifier_list(&mut self) -> Result<Vec<QualifierSpec>, ParseError> {
        let mut qualifiers = Vec::new();

        loop {
            // Parse qualifier name (can be identifier or keyword)
            let name_token = self.expect_qualifier_name("Expected qualifier name")?;
            let name = name_token.lexeme;
            let position = name_token.position;

            // Check for optional argument
            let arg = if self.check(&TokenKind::Colon) {
                self.advance()?; // consume ':'
                Some(self.parse_expression()?)
            } else {
                None
            };

            qualifiers.push(QualifierSpec {
                name,
                arg,
                position,
            });

            // Check for more qualifiers
            if self.check(&TokenKind::Comma) {
                self.advance()?; // consume ','
            } else {
                break;
            }
        }

        Ok(qualifiers)
    }

    // Parse a qualified operator: expr (qualifier-list) op expr
    fn parse_qualified_operator(&mut self, lhs: Expression) -> Result<Expression, ParseError> {
        let position = self.advance()?.position; // consume '('

        // Parse qualifier list
        let qualifiers = self.parse_qualifier_list()?;

        self.expect_token(TokenKind::RightParen, "Expected ')' after qualifier list")?;

        // Parse operator (= or !=)
        // In expression context, = means equality, not assignment
        let op = match self.advance()?.kind {
            TokenKind::Equal => QualifiedOpKind::Eq,
            TokenKind::NotEq => QualifiedOpKind::NotEq,
            found => return Err(ParseError::UnexpectedToken {
                expected: "'=' or '!='".to_string(),
                found,
                position,
            }),
        };

        // Parse right-hand side
        let rhs = self.parse_expression_with_precedence(5)?; // Higher precedence than assignment

        Ok(Expression::QualifiedOp(QualifiedOp {
            lhs: Box::new(lhs),
            qualifiers,
            op,
            rhs: Box::new(rhs),
            position,
        }))
    }

    fn parse_f_string(&mut self, start_pos: Position) -> Result<Expression, ParseError> {
        let mut parts = Vec::new();

        // Parse f-string parts until we hit FStringEnd
        loop {
            let token = self.advance()?;
            match token.kind {
                TokenKind::FStringText(text) => {
                    parts.push(FStringPart::Text(text));
                }
                TokenKind::FStringExprStart => {
                    // Parse expression until FStringExprEnd
                    let (expr, format_spec) = self.parse_f_string_expression()?;
                    parts.push(FStringPart::Expression(expr, format_spec));
                }
                TokenKind::FStringEnd => {
                    // End of f-string
                    break;
                }
                _ => return Err(ParseError::UnexpectedToken {
                    expected: "f-string content or end".to_string(),
                    found: token.kind,
                    position: token.position,
                }),
            }
        }

        Ok(Expression::FString(FString {
            parts,
            position: start_pos,
        }))
    }

    fn parse_f_string_expression(&mut self) -> Result<(Expression, Option<FormatSpec>), ParseError> {
        let expr = self.parse_expression()?;

        // Check for format specifiers (colon after expression)
        let format_spec = if self.check(&TokenKind::Colon) {
            let colon_token = self.advance()?;
            Some(self.parse_format_spec(colon_token.position)?)
        } else {
            None
        };

        // Expect FStringExprEnd
        self.expect_token(TokenKind::FStringExprEnd, "Expected '}' after f-string expression")?;

        Ok((expr, format_spec))
    }

    fn parse_format_spec(&mut self, start_pos: Position) -> Result<FormatSpec, ParseError> {
        // Format spec is parsed as a sequence of tokens up until the closing brace
        // We'll collect all tokens and parse them as a string
        let mut spec_chars = String::new();

        // Collect all characters until FStringExprEnd
        loop {
            let token = self.peek()?;
            match token.kind {
                TokenKind::FStringExprEnd => break,
                _ => {
                    // Consume the token and add its lexeme to the spec string
                    let consumed_token = self.advance()?;
                    spec_chars.push_str(&consumed_token.lexeme);
                }
            }
        }

        // If no spec characters found, error
        if spec_chars.is_empty() {
            return Err(ParseError::UnexpectedToken {
                expected: "format specifier after ':'".to_string(),
                found: TokenKind::FStringExprEnd,
                position: self.peek()?.position.clone(),
            });
        }

        // Parse the format specification
        self.parse_format_spec_string(&spec_chars, start_pos)
    }

    fn parse_format_spec_string(&mut self, spec: &str, position: Position) -> Result<FormatSpec, ParseError> {
        // Parse format spec according to: [fill align] [width] ['.' precision] [type]
        let chars: Vec<char> = spec.chars().collect();
        let mut i = 0;

        let mut fill: Option<char> = None;
        let mut align: Option<Align> = None;
        let mut width: Option<u32> = None;
        let mut precision: Option<u32> = None;
        let mut type_code: Option<char> = None;

        // Parse [fill align]
        if i < chars.len() && i + 1 < chars.len() {
            // Check if the second character is an alignment character
            let possible_align = chars[i + 1];
            if matches!(possible_align, '<' | '>' | '^') {
                fill = Some(chars[i]);
                align = Some(match possible_align {
                    '<' => Align::Left,
                    '>' => Align::Right,
                    '^' => Align::Center,
                    _ => unreachable!(),
                });
                i += 2;
            } else if matches!(chars[i], '<' | '>' | '^') {
                // No fill, just align
                align = Some(match chars[i] {
                    '<' => Align::Left,
                    '>' => Align::Right,
                    '^' => Align::Center,
                    _ => unreachable!(),
                });
                i += 1;
            }
        } else if i < chars.len() && matches!(chars[i], '<' | '>' | '^') {
            // No fill, just align
            align = Some(match chars[i] {
                '<' => Align::Left,
                '>' => Align::Right,
                '^' => Align::Center,
                _ => unreachable!(),
            });
            i += 1;
        }

        // Parse [width] - sequence of digits, possibly with leading zero for padding
        let width_start = i;
        let mut zero_padding = false;

        // Check for zero-padding (leading 0 in width)
        if i < chars.len() && chars[i] == '0' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
            zero_padding = true;
            i += 1; // Skip the leading 0
        }

        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        if width_start < i {
            let width_str: String = if zero_padding {
                chars[width_start + 1..i].iter().collect()  // Skip the leading 0
            } else {
                chars[width_start..i].iter().collect()
            };
            width = Some(width_str.parse().map_err(|_| ParseError::UnexpectedToken {
                expected: "valid width number".to_string(),
                found: TokenKind::FStringExprEnd,
                position: position.clone(),
            })?);

            // Set fill to '0' if zero-padding was specified
            if zero_padding {
                fill = Some('0');
            }
        }

        // Parse ['.' precision]
        if i < chars.len() && chars[i] == '.' {
            i += 1; // skip the dot
            let precision_start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if precision_start < i {
                let precision_str: String = chars[precision_start..i].iter().collect();
                precision = Some(precision_str.parse().map_err(|_| ParseError::UnexpectedToken {
                    expected: "valid precision number".to_string(),
                    found: TokenKind::FStringExprEnd,
                    position: position.clone(),
                })?);
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "precision number after '.'".to_string(),
                    found: TokenKind::FStringExprEnd,
                    position,
                });
            }
        }

        // Parse [type]
        if i < chars.len() {
            let type_char = chars[i];
            if matches!(type_char, 'd' | 'x' | 'X' | 'b' | 'o' | 'e' | 'E' | 'f' | 'g' | 's' | 'c') {
                type_code = Some(type_char);
                i += 1;
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "valid format type (d, x, X, b, o, e, E, f, g, s, c)".to_string(),
                    found: TokenKind::FStringExprEnd,
                    position: position.clone(),
                });
            }
        }

        // Check if there are any remaining characters
        if i < chars.len() {
            return Err(ParseError::UnexpectedToken {
                expected: "end of format specifier".to_string(),
                found: TokenKind::FStringExprEnd,
                position,
            });
        }

        Ok(FormatSpec {
            fill,
            align,
            width,
            precision,
            type_code,
            position,
        })
    }

    fn parse_object_literal(&mut self, start_pos: Position) -> Result<Expression, ParseError> {
        let mut properties = Vec::new();

        // Handle empty object literal: {}
        if self.check(&TokenKind::RightBrace) {
            self.advance()?; // consume '}'
            return Ok(Expression::ObjectLiteral(ObjectLiteral {
                properties,
                position: start_pos,
            }));
        }

        // Parse property list
        loop {
            // Parse property name (must be identifier)
            let name_token = self.expect_token(TokenKind::Identifier, "Expected property name")?;
            let prop_name = name_token.lexeme;

            // Parse ':'
            self.expect_token(TokenKind::Colon, "Expected ':' after property name")?;

            // Parse property value
            let value = self.parse_expression()?;

            properties.push((prop_name, value));

            // Check for ',' or '}'
            let token = self.advance()?;
            match token.kind {
                TokenKind::Comma => {
                    // Check for trailing comma (optional)
                    if self.check(&TokenKind::RightBrace) {
                        self.advance()?; // consume '}'
                        break;
                    }
                    // Continue to next property
                }
                TokenKind::RightBrace => {
                    break;
                }
                _ => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "',' or '}'".to_string(),
                        found: token.kind,
                        position: token.position,
                    });
                }
            }
        }

        Ok(Expression::ObjectLiteral(ObjectLiteral {
            properties,
            position: start_pos,
        }))
    }

    fn parse_function_expression(&mut self, start_pos: Position) -> Result<Expression, ParseError> {
        // Parse parameter list
        self.expect_token(TokenKind::LeftParen, "Expected '(' after 'function'")?;
        let params = self.parse_parameter_list()?;
        self.expect_token(TokenKind::RightParen, "Expected ')' after parameter list")?;

        // Parse return type
        self.expect_token(TokenKind::Colon, "Expected ':' after parameter list")?;
        let return_type = self.parse_type()?;

        // Parse body
        let body = self.parse_block()?;

        Ok(Expression::FunctionExpr(FunctionExpr {
            params,
            return_type,
            body,
            position: start_pos,
            captures: RefCell::new(Vec::new()),  // Initialized by parser, populated by type checker
        }))
    }

    fn parse_match_expression(&mut self, start_pos: Position) -> Result<Expression, ParseError> {
        // Parse the matched expression
        let value = self.parse_expression()?;

        // Expect opening brace
        self.expect_token(TokenKind::LeftBrace, "Expected '{' after match expression")?;

        let mut arms = Vec::new();

        // Parse match arms
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let arm_pos = self.current_position();

            // Parse pattern
            let pattern = self.parse_match_pattern()?;

            // Expect arrow
            self.expect_token(TokenKind::Arrow, "Expected '=>' after match pattern")?;

            // Parse body
            let body = self.parse_match_body()?;

            arms.push(MatchArm {
                pattern,
                body,
                position: arm_pos,
            });

            // Optional comma after arm
            if self.check(&TokenKind::Comma) {
                self.advance()?;
            }
        }

        // Expect closing brace
        self.expect_token(TokenKind::RightBrace, "Expected '}' after match arms")?;

        if arms.is_empty() {
            return Err(ParseError::UnexpectedToken {
                expected: "at least one match arm".to_string(),
                found: TokenKind::RightBrace,
                position: self.previous()?.position.clone(),
            });
        }

        Ok(Expression::Match(MatchExpr {
            value: Box::new(value),
            arms,
            position: start_pos,
        }))
    }

    fn parse_match_pattern(&mut self) -> Result<MatchPattern, ParseError> {
        let token = self.advance()?;

        match token.kind {
            TokenKind::Integer(n) => Ok(MatchPattern::Literal(Literal::Integer(n))),
            TokenKind::Float(f) => Ok(MatchPattern::Literal(Literal::Float(f))),
            TokenKind::StringLit(s) => Ok(MatchPattern::Literal(Literal::String(s))),
            TokenKind::True => Ok(MatchPattern::Literal(Literal::Bool(true))),
            TokenKind::False => Ok(MatchPattern::Literal(Literal::Bool(false))),
            TokenKind::Identifier if token.lexeme == "_" => Ok(MatchPattern::Wildcard),
            _ => Err(ParseError::UnexpectedToken {
                expected: "literal or wildcard pattern".to_string(),
                found: token.kind,
                position: token.position,
            }),
        }
    }

    fn parse_match_body(&mut self) -> Result<MatchBody, ParseError> {
        if self.check(&TokenKind::LeftBrace) {
            // Block body
            let block = self.parse_block()?;
            Ok(MatchBody::Block(block))
        } else {
            // Expression body
            let expr = self.parse_expression()?;
            Ok(MatchBody::Expression(expr))
        }
    }

    fn parse_unknown_constructor(&mut self, start_pos: Position) -> Result<Expression, ParseError> {
        // Parse unknown(reason, options: [...])
        self.expect_token(TokenKind::LeftParen, "Expected '(' after 'unknown'")?;

        // Parse reason (required)
        let reason = self.parse_expression()?;

        // Check for options argument
        let options = if self.check(&TokenKind::Comma) {
            self.advance()?; // consume ','

            // Expect 'options:'
            let ident_token = self.advance()?;
            if let TokenKind::Identifier = ident_token.kind {
                if ident_token.lexeme != "options" {
                    return Err(ParseError::UnexpectedToken {
                        expected: "options".to_string(),
                        found: ident_token.kind,
                        position: ident_token.position,
                    });
                }
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "options".to_string(),
                    found: ident_token.kind,
                    position: ident_token.position,
                });
            }

            self.expect_token(TokenKind::Colon, "Expected ':' after 'options'")?;

            // Parse array expression
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect_token(TokenKind::RightParen, "Expected ')' after unknown constructor arguments")?;

        Ok(Expression::Unknown(UnknownConstruction {
            reason: Box::new(reason),
            options: options.map(Box::new),
            position: start_pos,
        }))
    }
}
