use hilowc::parser::Parser;
use hilowc::ast::*;

// Successful parse tests

#[test]
fn test_minimal_high_program() {
    let input = "high program(): i32 { }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Program(program) => {
            assert_eq!(program.mode, Mode::High);
            assert!(program.params.is_empty());
            assert_eq!(program.return_type, Type::Primitive(PrimitiveType::I32));
        }
        _ => panic!("Expected Program, got Module"),
    }
}

#[test]
fn test_minimal_low_program() {
    let input = "low program(): i32 { }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Program(program) => {
            assert_eq!(program.mode, Mode::Low);
            assert!(program.params.is_empty());
            assert_eq!(program.return_type, Type::Primitive(PrimitiveType::I32));
        }
        _ => panic!("Expected Program, got Module"),
    }
}

#[test]
fn test_program_with_parameters() {
    let input = "high program(args: [string]): i32 { }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Program(program) => {
            assert_eq!(program.mode, Mode::High);
            assert_eq!(program.params.len(), 1);

            let param = &program.params[0];
            assert_eq!(param.name, "args");
            assert_eq!(
                param.ty,
                Type::DynamicArray(Box::new(Type::Primitive(PrimitiveType::String)))
            );

            assert_eq!(program.return_type, Type::Primitive(PrimitiveType::I32));
        }
        _ => panic!("Expected Program, got Module"),
    }
}

#[test]
fn test_high_module_with_function_signatures() {
    let input = "high module { export function add(a: i32, b: i32): i32 { } }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Module(module) => {
            assert_eq!(module.mode, Mode::High);
            assert_eq!(module.items.len(), 1);

            let func = &module.items[0];
            assert_eq!(func.name, "add");
            assert_eq!(func.mode, Mode::High); // Inherited
            assert!(func.is_export);
            assert_eq!(func.params.len(), 2);
            assert_eq!(func.params[0].name, "a");
            assert_eq!(func.params[0].ty, Type::Primitive(PrimitiveType::I32));
            assert_eq!(func.params[1].name, "b");
            assert_eq!(func.params[1].ty, Type::Primitive(PrimitiveType::I32));
            assert_eq!(func.return_type, Type::Primitive(PrimitiveType::I32));
        }
        _ => panic!("Expected Module, got Program"),
    }
}

#[test]
fn test_module_with_multiple_functions() {
    let input = "high module {
        export function add(a: i32, b: i32): i32 { }
        function helper(): bool { }
    }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Module(module) => {
            assert_eq!(module.mode, Mode::High);
            assert_eq!(module.items.len(), 2);

            // First function (exported)
            let func1 = &module.items[0];
            assert_eq!(func1.name, "add");
            assert_eq!(func1.mode, Mode::High);
            assert!(func1.is_export);

            // Second function (not exported)
            let func2 = &module.items[1];
            assert_eq!(func2.name, "helper");
            assert_eq!(func2.mode, Mode::High);
            assert!(!func2.is_export);
            assert_eq!(func2.return_type, Type::Primitive(PrimitiveType::Bool));
        }
        _ => panic!("Expected Module, got Program"),
    }
}

#[test]
fn test_mode_override_at_function_level() {
    // For this test, we need to modify our approach since the program body
    // isn't parsed yet. Let me test this with a module instead.
    let input = "high module {
        function highFunc(x: i32): i32 { }
        low function lowFunc(p: u32): u32 { }
    }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Module(module) => {
            assert_eq!(module.mode, Mode::High);
            assert_eq!(module.items.len(), 2);

            // First function inherits High mode
            let func1 = &module.items[0];
            assert_eq!(func1.name, "highFunc");
            assert_eq!(func1.mode, Mode::High);

            // Second function explicitly Low mode
            let func2 = &module.items[1];
            assert_eq!(func2.name, "lowFunc");
            assert_eq!(func2.mode, Mode::Low);
        }
        _ => panic!("Expected Module, got Program"),
    }
}

#[test]
fn test_fixed_array_type() {
    let input = "high program(buf: [u8; 256]): i32 { }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Program(program) => {
            assert_eq!(program.params.len(), 1);
            let param = &program.params[0];
            assert_eq!(param.name, "buf");
            assert_eq!(
                param.ty,
                Type::FixedArray(Box::new(Type::Primitive(PrimitiveType::U8)), 256)
            );
        }
        _ => panic!("Expected Program, got Module"),
    }
}

#[test]
fn test_dynamic_array_type() {
    let input = "high program(items: [i32]): bool { }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Program(program) => {
            assert_eq!(program.params.len(), 1);
            let param = &program.params[0];
            assert_eq!(param.name, "items");
            assert_eq!(
                param.ty,
                Type::DynamicArray(Box::new(Type::Primitive(PrimitiveType::I32)))
            );
            assert_eq!(program.return_type, Type::Primitive(PrimitiveType::Bool));
        }
        _ => panic!("Expected Program, got Module"),
    }
}

#[test]
fn test_various_primitive_types() {
    let input = "high module {
        function test1(): u64 { }
        function test2(): f32 { }
        function test3(): bool { }
        function test4(): string { }
        function test5(): usize { }
        function test6(): nothing { }
    }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Module(module) => {
            assert_eq!(module.items.len(), 6);

            assert_eq!(module.items[0].return_type, Type::Primitive(PrimitiveType::U64));
            assert_eq!(module.items[1].return_type, Type::Primitive(PrimitiveType::F32));
            assert_eq!(module.items[2].return_type, Type::Primitive(PrimitiveType::Bool));
            assert_eq!(module.items[3].return_type, Type::Primitive(PrimitiveType::String));
            assert_eq!(module.items[4].return_type, Type::Primitive(PrimitiveType::Usize));
            assert_eq!(module.items[5].return_type, Type::Primitive(PrimitiveType::Nothing));
        }
        _ => panic!("Expected Module, got Program"),
    }
}

// Error case tests

#[test]
fn test_missing_program_keyword() {
    let input = "high (): i32 { }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_err());
    if let Err(error) = result {
        match error {
            hilowc::parser::ParseError::UnexpectedToken { expected, .. } => {
                assert!(expected.contains("'program' or 'module'"));
            }
            _ => panic!("Expected UnexpectedToken error, got {:?}", error),
        }
    }
}

#[test]
fn test_missing_return_type() {
    let input = "high program() { }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_err());
    if let Err(error) = result {
        match error {
            hilowc::parser::ParseError::UnexpectedToken { expected, .. } => {
                assert!(expected.contains("':'"));
            }
            _ => panic!("Expected UnexpectedToken error for missing colon, got {:?}", error),
        }
    }
}

#[test]
fn test_unclosed_parameter_list() {
    let input = "high program(args: [string]: i32 { }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_err());
    if let Err(error) = result {
        match error {
            hilowc::parser::ParseError::UnexpectedToken { expected, .. } => {
                assert!(expected.contains("')'"));
            }
            _ => panic!("Expected UnexpectedToken error for missing paren, got {:?}", error),
        }
    }
}

#[test]
fn test_pointer_type_not_supported() {
    let input = "high module { function p(x: *u8): i32 { } }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_err());
    if let Err(error) = result {
        match error {
            hilowc::parser::ParseError::UnsupportedFeature { feature, suggestion, .. } => {
                assert_eq!(feature, "pointer types");
                assert!(suggestion.contains("Phase 12"));
            }
            _ => panic!("Expected UnsupportedFeature error, got {:?}", error),
        }
    }
}

#[test]
fn test_unknown_type_name() {
    let input = "high program(x: unknowntype): i32 { }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_err());
    if let Err(error) = result {
        match error {
            hilowc::parser::ParseError::UnexpectedToken { expected, .. } => {
                assert!(expected.contains("primitive type name"));
            }
            _ => panic!("Expected UnexpectedToken error for unknown type, got {:?}", error),
        }
    }
}

#[test]
fn test_missing_function_body() {
    let input = "high program(): i32";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_err());
    if let Err(error) = result {
        match error {
            hilowc::parser::ParseError::UnexpectedToken { expected, .. } => {
                assert!(expected.contains("'{'"));
            }
            _ => panic!("Expected UnexpectedToken error for missing brace, got {:?}", error),
        }
    }
}

#[test]
fn test_unclosed_function_body() {
    let input = "high program(): i32 { ";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_err());
    // Phase 2b: Now that we parse statements, the error occurs when trying to parse expressions
    if let Err(error) = result {
        match error {
            hilowc::parser::ParseError::UnexpectedToken { found, .. } => {
                assert_eq!(found, hilowc::lexer::TokenKind::Eof);
            }
            _ => panic!("Expected UnexpectedToken error for EOF, got {:?}", error),
        }
    }
}

// Phase 2b tests

#[test]
fn test_simple_let_statement() {
    let input = "high program(): i32 { let x = 5 }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Program(program) => {
            let body = program.body.expect("Program should have body");
            assert_eq!(body.items.len(), 1);

            match &body.items[0] {
                BlockItem::Statement(Statement::Let(let_decl)) => {
                    assert_eq!(let_decl.name, "x");
                    assert_eq!(let_decl.ty, None);
                    assert!(let_decl.initializer.is_some());

                    match let_decl.initializer.as_ref().unwrap() {
                        Expression::IntLit(5, _) => {},
                        _ => panic!("Expected integer literal 5"),
                    }
                }
                _ => panic!("Expected let statement"),
            }
        }
        _ => panic!("Expected Program"),
    }
}

#[test]
fn test_arithmetic_precedence() {
    let input = "high program(): i32 { let x = 1 + 2 * 3 }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Program(program) => {
            let body = program.body.expect("Program should have body");

            if let BlockItem::Statement(Statement::Let(let_decl)) = &body.items[0] {
                if let Some(Expression::BinaryOp(add_op)) = &let_decl.initializer {
                    // Should be: 1 + (2 * 3)
                    assert_eq!(add_op.op, BinaryOpKind::Add);

                    // Left side should be 1
                    match add_op.lhs.as_ref() {
                        Expression::IntLit(1, _) => {},
                        _ => panic!("Expected left side to be 1"),
                    }

                    // Right side should be 2 * 3
                    match add_op.rhs.as_ref() {
                        Expression::BinaryOp(mul_op) => {
                            assert_eq!(mul_op.op, BinaryOpKind::Mul);
                        }
                        _ => panic!("Expected right side to be multiplication"),
                    }
                } else {
                    panic!("Expected binary operation in let initializer");
                }
            } else {
                panic!("Expected let statement");
            }
        }
        _ => panic!("Expected Program"),
    }
}

#[test]
fn test_if_statement() {
    let input = "high program(): i32 { if (x > 5) { let y = 10 } }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Program(program) => {
            let body = program.body.expect("Program should have body");
            assert_eq!(body.items.len(), 1);

            match &body.items[0] {
                BlockItem::Statement(Statement::If(if_stmt)) => {
                    // Check condition
                    match &if_stmt.condition {
                        Expression::BinaryOp(op) => {
                            assert_eq!(op.op, BinaryOpKind::Greater);
                        }
                        _ => panic!("Expected comparison in if condition"),
                    }

                    // Check then block
                    assert_eq!(if_stmt.then_block.statements.len(), 1);
                    assert!(if_stmt.else_block.is_none());
                }
                _ => panic!("Expected if statement"),
            }
        }
        _ => panic!("Expected Program"),
    }
}

#[test]
fn test_return_statement() {
    let input = "high program(): i32 { return 0 }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Program(program) => {
            let body = program.body.expect("Program should have body");
            assert_eq!(body.items.len(), 1);

            match &body.items[0] {
                BlockItem::Statement(Statement::Return(return_stmt)) => {
                    assert!(return_stmt.value.is_some());
                    match return_stmt.value.as_ref().unwrap() {
                        Expression::IntLit(0, _) => {},
                        _ => panic!("Expected return value 0"),
                    }
                }
                _ => panic!("Expected return statement"),
            }
        }
        _ => panic!("Expected Program"),
    }
}

#[test]
fn test_assignment_not_allowed_in_expression_position() {
    let input = "high program(): i32 { if (x = 5) { } }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_err(), "Assignment should not be allowed in expression position");

    if let Err(error) = result {
        match error {
            hilowc::parser::ParseError::UnexpectedToken { expected, found, .. } => {
                // Should find the equals token where an expression continuation was expected
                assert_eq!(found, hilowc::lexer::TokenKind::Equal);
                assert!(expected.contains(")"));
            }
            _ => panic!("Expected UnexpectedToken error, got {:?}", error),
        }
    }
}

// Phase 5b: Qualified operators parser tests

#[test]
fn test_simple_qualified_assignment() {
    let input = "high program(): i32 { flags (bitor)= 4 }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Program(program) => {
            if let Some(body) = program.body {
                assert_eq!(body.items.len(), 1);
                match &body.items[0] {
                    BlockItem::Statement(Statement::QualifiedOp(qualified_op)) => {
                        assert_eq!(qualified_op.qualifiers.len(), 1);
                        assert_eq!(qualified_op.qualifiers[0].name, "bitor");
                        assert!(qualified_op.qualifiers[0].arg.is_none());
                        assert_eq!(qualified_op.op, QualifiedOpKind::Assign);
                        match qualified_op.rhs.as_ref() {
                            Expression::IntLit(4, _) => {},
                            _ => panic!("Expected integer literal 4")
                        }
                    }
                    _ => panic!("Expected qualified operator statement")
                }
            }
        }
        _ => panic!("Expected Program")
    }
}

#[test]
fn test_qualified_assignment_with_argument() {
    let input = "high program(): i32 { x (within: 0.01)= y }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Program(program) => {
            if let Some(body) = program.body {
                assert_eq!(body.items.len(), 1);
                match &body.items[0] {
                    BlockItem::Statement(Statement::QualifiedOp(qualified_op)) => {
                        assert_eq!(qualified_op.qualifiers.len(), 1);
                        assert_eq!(qualified_op.qualifiers[0].name, "within");
                        assert!(qualified_op.qualifiers[0].arg.is_some());
                        match &qualified_op.qualifiers[0].arg {
                            Some(Expression::FloatLit(f, _)) => {
                                assert_eq!(*f, 0.01);
                            }
                            _ => panic!("Expected float literal 0.01")
                        }
                        assert_eq!(qualified_op.op, QualifiedOpKind::Assign);
                    }
                    _ => panic!("Expected qualified operator statement")
                }
            }
        }
        _ => panic!("Expected Program")
    }
}

#[test]
fn test_qualified_equality_multiple_qualifiers() {
    let input = "high program(): i32 { if (s1 (caseless, trimmed)= s2) { } }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Program(program) => {
            if let Some(body) = program.body {
                assert_eq!(body.items.len(), 1);
                match &body.items[0] {
                    BlockItem::Statement(Statement::If(if_stmt)) => {
                        match &if_stmt.condition {
                            Expression::QualifiedOp(qualified_op) => {
                                assert_eq!(qualified_op.qualifiers.len(), 2);
                                assert_eq!(qualified_op.qualifiers[0].name, "caseless");
                                assert_eq!(qualified_op.qualifiers[1].name, "trimmed");
                                assert!(qualified_op.qualifiers[0].arg.is_none());
                                assert!(qualified_op.qualifiers[1].arg.is_none());
                                assert_eq!(qualified_op.op, QualifiedOpKind::Eq);
                            }
                            _ => panic!("Expected qualified operator in if condition")
                        }
                    }
                    _ => panic!("Expected if statement")
                }
            }
        }
        _ => panic!("Expected Program")
    }
}

#[test]
fn test_or_qualified_assignment() {
    let input = "high program(): i32 { x (or)= y }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Program(program) => {
            if let Some(body) = program.body {
                assert_eq!(body.items.len(), 1);
                match &body.items[0] {
                    BlockItem::Statement(Statement::QualifiedOp(qualified_op)) => {
                        assert_eq!(qualified_op.qualifiers.len(), 1);
                        assert_eq!(qualified_op.qualifiers[0].name, "or");
                        assert!(qualified_op.qualifiers[0].arg.is_none());
                        assert_eq!(qualified_op.op, QualifiedOpKind::Assign);
                    }
                    _ => panic!("Expected qualified operator statement")
                }
            }
        }
        _ => panic!("Expected Program")
    }
}

#[test]
fn test_function_call_vs_qualified_operator_disambiguation() {
    // This should parse as a function call, not qualified operator
    let input = "high program(): i32 { x() }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Program(program) => {
            if let Some(body) = program.body {
                assert_eq!(body.items.len(), 1);
                match &body.items[0] {
                    BlockItem::Statement(Statement::ExprStatement(Expression::Call(call))) => {
                        match call.callee.as_ref() {
                            Expression::Ident(name, _) => {
                                assert_eq!(name, "x");
                            }
                            _ => panic!("Expected identifier 'x'")
                        }
                        assert!(call.args.is_empty());
                    }
                    _ => panic!("Expected function call")
                }
            }
        }
        _ => panic!("Expected Program")
    }
}

#[test]
fn test_function_call_with_arg() {
    // This should parse as a function call, not qualified operator
    let input = "high program(): i32 { foo(arg) }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok());
    let top_level = result.unwrap();

    match top_level {
        TopLevel::Program(program) => {
            if let Some(body) = program.body {
                assert_eq!(body.items.len(), 1);
                match &body.items[0] {
                    BlockItem::Statement(Statement::ExprStatement(Expression::Call(call))) => {
                        match call.callee.as_ref() {
                            Expression::Ident(name, _) => {
                                assert_eq!(name, "foo");
                            }
                            _ => panic!("Expected identifier 'foo'")
                        }
                        assert_eq!(call.args.len(), 1);
                    }
                    _ => panic!("Expected function call")
                }
            }
        }
        _ => panic!("Expected Program")
    }
}

// Phase 6a-fixup: Nested function tests

#[test]
fn test_nested_function_parsing() {
    let input = "high program(): i32 {
        function double(x: i32): i32 {
            return x * 2
        }
        print(double(21))
        return 0
    }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok(), "Nested function should parse successfully");

    let top_level = result.unwrap();
    match top_level {
        TopLevel::Program(program) => {
            let body = program.body.expect("Program should have body");
            assert_eq!(body.items.len(), 3); // function, print statement, and return statement

            // First item should be a function
            match &body.items[0] {
                BlockItem::Function(func) => {
                    assert_eq!(func.name, "double");
                }
                _ => panic!("Expected function as first item"),
            }

            // Second item should be a statement
            match &body.items[1] {
                BlockItem::Statement(_) => {
                    // Expected
                }
                _ => panic!("Expected statement as second item"),
            }
        }
        _ => panic!("Expected Program"),
    }
}

// Optional semicolon tests (per spec: JavaScript-style optional semicolons)

#[test]
fn test_single_line_semicolon_separated() {
    let input = "high program(): i32 { let x = 1; let y = 2; let z = 3; return 0 }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok(), "Single-line semicolon-separated statements should parse");

    let top_level = result.unwrap();
    match top_level {
        TopLevel::Program(program) => {
            let body = program.body.expect("Program should have body");
            assert_eq!(body.items.len(), 4); // three let statements and one return
        }
        _ => panic!("Expected Program"),
    }
}

#[test]
fn test_trailing_semicolon() {
    let input = "high program(): i32 { let x = 1; return x }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok(), "Trailing semicolon should be accepted");

    let top_level = result.unwrap();
    match top_level {
        TopLevel::Program(program) => {
            let body = program.body.expect("Program should have body");
            assert_eq!(body.items.len(), 2); // one let and one return statement
        }
        _ => panic!("Expected Program"),
    }
}

#[test]
fn test_multiple_semicolons() {
    let input = "high program(): i32 { let x = 1;;; return x }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok(), "Multiple semicolons should be accepted as no-ops");

    let top_level = result.unwrap();
    match top_level {
        TopLevel::Program(program) => {
            let body = program.body.expect("Program should have body");
            assert_eq!(body.items.len(), 2); // one let and one return statement (semicolons are ignored)
        }
        _ => panic!("Expected Program"),
    }
}

#[test]
fn test_mixed_newlines_and_semicolons() {
    let input = "high program(): i32 {
        let x = 1;
        let y = 2
        let z = 3;
        return x + y + z
    }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok(), "Mixed newlines and semicolons should parse");

    let top_level = result.unwrap();
    match top_level {
        TopLevel::Program(program) => {
            let body = program.body.expect("Program should have body");
            assert_eq!(body.items.len(), 4); // three let statements and one return
        }
        _ => panic!("Expected Program"),
    }
}

#[test]
fn test_leading_semicolon() {
    let input = "high program(): i32 { ; let x = 1; return x }";
    let result = Parser::new(input).unwrap().parse();

    assert!(result.is_ok(), "Leading semicolon should be accepted");

    let top_level = result.unwrap();
    match top_level {
        TopLevel::Program(program) => {
            let body = program.body.expect("Program should have body");
            assert_eq!(body.items.len(), 2); // one let and one return statement (leading semicolon ignored)
        }
        _ => panic!("Expected Program"),
    }
}
