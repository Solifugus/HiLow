use hilowc::parser::Parser;
use hilowc::typecheck::TypeChecker;

// Helper function to parse and type check a program
fn type_check_program(input: &str) -> Result<(), Vec<hilowc::types::TypeError>> {
    let result = Parser::new(input).unwrap().parse();
    assert!(result.is_ok(), "Parse failed: {:?}", result);
    let top_level = result.unwrap();

    let mut type_checker = TypeChecker::new();
    type_checker.check(&top_level)
}

// Successful type checks

#[test]
fn test_let_with_explicit_type_and_matching_initializer() {
    let input = "high program(): i32 { let x: i32 = 42 }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check, got: {:?}", result);
}

#[test]
fn test_let_with_type_inference() {
    let input = "high program(): i32 { let x = 42 }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check, got: {:?}", result);
}

#[test]
fn test_let_with_float_inference() {
    let input = "high program(): i32 { let x = 3.14 }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check, got: {:?}", result);
}

#[test]
fn test_let_with_small_integer_in_u8() {
    let input = "high program(): i32 { let x: u8 = 42 }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check, got: {:?}", result);
}

#[test]
fn test_arithmetic_with_same_types() {
    let input = "high program(): i32 {
        let x: i32 = 5
        let y = x + 1
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check, got: {:?}", result);
}

#[test]
fn test_bool_condition() {
    let input = "high program(): i32 {
        let cond = true
        if (cond) { }
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check, got: {:?}", result);
}

#[test]
fn test_comparison_returns_bool() {
    let input = "high program(): i32 { if (5 < 10) { } }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check, got: {:?}", result);
}

#[test]
fn test_equality_check_same_types() {
    let input = "high program(): i32 {
        let x = 5
        if (x ?= 5) { }
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check, got: {:?}", result);
}

// Type errors

#[test]
fn test_let_type_mismatch() {
    let input = "high program(): i32 { let x: i32 = true }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("bool"));
        assert!(errors[0].message.contains("i32"));
    }
}

#[test]
fn test_integer_too_large_for_u8() {
    let input = "high program(): i32 { let x: u8 = 300 }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        // The error should be about type mismatch since 300 infers to i32
        assert!(errors[0].message.contains("i32"));
        assert!(errors[0].message.contains("u8"));
    }
}

#[test]
fn test_arithmetic_type_mismatch() {
    let input = "high program(): i32 {
        let x = 5
        x + 3.14
    }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("i32"));
        assert!(errors[0].message.contains("f64"));
    }
}

#[test]
fn test_string_plus_integer() {
    let input = "high program(): i32 { true + 2 }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("bool"));
    }
}

#[test]
fn test_string_equals_integer() {
    let input = "high program(): i32 { 5 ?= true }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("i32"));
        assert!(errors[0].message.contains("bool"));
    }
}

#[test]
fn test_numeric_condition_allowed() {
    // Phase 4b: numeric conditions are allowed (truthy/falsy)
    let input = "high program(): i32 { if (5) { } }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Numeric conditions should be allowed in Phase 4b");
}

#[test]
fn test_bool_plus_integer() {
    let input = "high program(): i32 { true + 5 }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("bool"));
    }
}

#[test]
fn test_undefined_variable() {
    let input = "high program(): i32 { return y }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("Undefined variable 'y'"));
    }
}

#[test]
fn test_let_with_no_type_and_no_initializer() {
    // This should produce a parse error, not a type error
    // But let's check that we handle it gracefully
    let input = "high program(): i32 { let x }";
    let parse_result = Parser::new(input).unwrap().parse();
    // This should fail during parsing, not type checking
    assert!(parse_result.is_err(), "Expected parse error for let with no type and no initializer");
}

// Tests for various arithmetic combinations

#[test]
fn test_i32_plus_i64_error() {
    let input = "high program(): i32 {
        let x: i32 = 5
        let y: i64 = 10
        x + y
    }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("i32"));
        assert!(errors[0].message.contains("i64"));
    }
}

#[test]
fn test_while_numeric_condition_allowed() {
    // Phase 4b: numeric conditions are allowed (truthy/falsy)
    let input = "high program(): i32 { while (5) { break } }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Numeric conditions should be allowed in Phase 4b");
}

#[test]
fn test_logical_operators_need_bool() {
    let input = "high program(): i32 { 5 and 6 }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("bool"));
    }
}

#[test]
fn test_bitwise_operators_need_integers() {
    let input = "high program(): i32 { 5.5 & 6.6 }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("integer"));
    }
}

#[test]
fn test_negation_of_bool() {
    let input = "high program(): i32 { -true }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("bool"));
    }
}

#[test]
fn test_logical_not_of_integer() {
    let input = "high program(): i32 { not 5 }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("bool"));
    }
}

#[test]
fn test_is_check_returns_bool() {
    let input = "high program(): i32 { if (5 is i32) { return 0 } return 1 }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check, got: {:?}", result);
}

// Phase 4b tests

#[test]
fn test_break_outside_loop() {
    let input = "high program(): i32 { break }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("break is only valid inside a loop"));
    }
}

#[test]
fn test_continue_outside_loop() {
    let input = "high program(): i32 { continue }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("continue is only valid inside a loop"));
    }
}

#[test]
fn test_break_inside_while_loop() {
    let input = "high program(): i32 { while (true) { break } }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Break should be allowed inside while loop");
}

#[test]
fn test_continue_inside_loop() {
    let input = "high program(): i32 { loop { continue } }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Continue should be allowed inside loop");
}

#[test]
fn test_float_condition() {
    let input = "high program(): i32 { if (3.14) { } }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Float conditions should be allowed in Phase 4b");
}

// Note: String condition test omitted because string literals not implemented until Phase 6

// Phase 5b: Qualified operators type checker tests

#[test]
fn test_bitor_assignment_with_integer() {
    let input = "high program(): i32 { let flags = 0\n  flags (bitor)= 4 }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "bitor qualifier should work with integer types");
}

#[test]
fn test_bitor_assignment_with_bool_error() {
    let input = "high program(): i32 { let flag = true; flag (bitor)= false }";
    let result = type_check_program(input);
    assert!(result.is_err(), "bitor qualifier should not work with bool types");

    if let Err(errors) = result {
        let error_message = errors[0].to_string();
        assert!(error_message.contains("bitor"));
        assert!(error_message.contains("requires compatible types"));
    }
}

#[test]
fn test_or_assignment_with_bool() {
    let input = "high program(): i32 { let ready = false; ready (or)= true }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "or qualifier should work with bool types");
}

#[test]
fn test_or_assignment_with_integer_error() {
    let input = "high program(): i32 { let x = 0; x (or)= 1 }";
    let result = type_check_program(input);
    assert!(result.is_err(), "or qualifier should not work with integer types");

    if let Err(errors) = result {
        let error_message = errors[0].to_string();
        assert!(error_message.contains("or"));
        assert!(error_message.contains("requires compatible types"));
    }
}

#[test]
fn test_unknown_qualifier_error() {
    let input = "high program(): i32 { let x = 0; x (nonexistent)= 5 }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Unknown qualifiers should be rejected");

    if let Err(errors) = result {
        let error_message = errors[0].to_string();
        assert!(error_message.contains("nonexistent"));
        assert!(error_message.contains("is not defined"));
    }
}

#[test]
fn test_qualifier_with_wrong_arguments() {
    let input = "high program(): i32 { let x = 0; x (or: 5)= 1 }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Qualifiers with wrong arguments should be rejected");

    if let Err(errors) = result {
        let error_message = errors[0].to_string();
        assert!(error_message.contains("or"));
        assert!(error_message.contains("takes no arguments"));
    }
}

#[test]
fn test_qualifier_in_wrong_context() {
    let input = "high program(): i32 { let a = 5; let b = 5; if (a (or)= b) { } }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Assignment qualifiers in equality context should be rejected");

    if let Err(errors) = result {
        let error_message = errors[0].to_string();
        assert!(error_message.contains("or"));
        assert!(error_message.contains("assignment only"));
        assert!(error_message.contains("not equality"));
    }
}