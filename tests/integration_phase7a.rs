use hilowc::parser::Parser;
use hilowc::typecheck::TypeChecker;
use hilowc::codegen::CodeGenerator;

#[test]
#[ignore = "Codegen limitation: get_expression_type needs symbol table context"]
fn test_phase7a_integration() {
    let input = "high program(): i32 {
        let point = { x: 10, y: 20 }
        let x_val = point.x

        point.x = 99
        let new_x = point.x

        let person = {
            name: \"Alice\",
            age: 30,
            active: true
        }

        return 0
    }";

    // Parse
    println!("Parsing...");
    let parse_result = Parser::new(input).unwrap().parse();
    assert!(parse_result.is_ok(), "Parse failed: {:?}", parse_result);
    let ast = parse_result.unwrap();

    // Type check
    println!("Type checking...");
    let mut type_checker = TypeChecker::new();
    let typecheck_result = type_checker.check(&ast);
    assert!(typecheck_result.is_ok(), "Type check failed: {:?}", typecheck_result);

    // Code generation
    println!("Generating code...");
    let mut codegen = CodeGenerator::new();
    let codegen_result = codegen.generate(&ast, &type_checker);
    assert!(codegen_result.is_ok(), "Codegen failed: {:?}", codegen_result);

    let c_code = codegen_result.unwrap();
    println!("Generated C code:\n{}", c_code);

    // Verify the generated code contains expected object operations
    assert!(c_code.contains("hl_object_new()"), "Should create objects with hl_object_new");
    assert!(c_code.contains("hl_object_set_i32"), "Should set integer properties");
    assert!(c_code.contains("hl_object_set_str"), "Should set string properties");
    assert!(c_code.contains("hl_object_set_bool"), "Should set boolean properties");
    assert!(c_code.contains("hl_object_get_i32"), "Should get integer properties");
    assert!(c_code.contains("hl_object_get_str"), "Should get string properties");
    assert!(c_code.contains("print_i32"), "Should print integer values");
    assert!(c_code.contains("print_str"), "Should print string values");
}

#[test]
fn test_phase7a_strict_property_access() {
    // Test that accessing non-existent properties fails in Phase 7a
    let input = "high program(): i32 {
        let obj = { x: 10 }
        let val = obj.y  // Error: y doesn't exist
        return 0
    }";

    let parse_result = Parser::new(input).unwrap().parse();
    assert!(parse_result.is_ok());
    let ast = parse_result.unwrap();

    let mut type_checker = TypeChecker::new();
    let typecheck_result = type_checker.check(&ast);

    // Should fail with property not found error
    assert!(typecheck_result.is_err(), "Should fail when accessing non-existent property");

    if let Err(errors) = typecheck_result {
        assert!(!errors.is_empty());
        let error_msg = &errors[0].message;
        assert!(error_msg.contains("Object does not have property 'y'"));
        assert!(error_msg.contains("Phase 9 will allow runtime property access"));
    }
}

#[test]
fn test_phase7a_strict_property_assignment() {
    // Test that assigning to non-existent properties fails in Phase 7a
    let input = "high program(): i32 {
        let obj = { x: 10 }
        obj.y = 20  // Error: y doesn't exist
        return 0
    }";

    let parse_result = Parser::new(input).unwrap().parse();
    assert!(parse_result.is_ok());
    let ast = parse_result.unwrap();

    let mut type_checker = TypeChecker::new();
    let typecheck_result = type_checker.check(&ast);

    // Should fail with property not found error
    assert!(typecheck_result.is_err(), "Should fail when assigning to non-existent property");

    if let Err(errors) = typecheck_result {
        assert!(!errors.is_empty());
        let error_msg = &errors[0].message;
        assert!(error_msg.contains("Object does not have property 'y'"));
        assert!(error_msg.contains("Phase 9 will allow runtime property access"));
    }
}