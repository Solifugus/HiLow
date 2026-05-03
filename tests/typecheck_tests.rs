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
fn test_non_bool_condition() {
    let input = "high program(): i32 { if (5) { } }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("bool"));
        assert!(errors[0].message.contains("i32"));
    }
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
fn test_while_condition_must_be_bool() {
    let input = "high program(): i32 { while (5) { break } }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("bool"));
    }
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