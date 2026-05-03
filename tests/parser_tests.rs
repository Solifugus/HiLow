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
    if let Err(error) = result {
        match error {
            hilowc::parser::ParseError::UnexpectedEof { expected, .. } => {
                assert!(expected.contains("'}'"));
            }
            _ => panic!("Expected UnexpectedEof error for unclosed body, got {:?}", error),
        }
    }
}