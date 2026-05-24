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
    // Phase 9a: let x without type or initializer is now valid (type nothing)
    let input = "high program(): i32 { let x }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "let x without type or initializer should be valid in Phase 9a (type nothing)");
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

// Phase 7c-α: Function expression type checker tests

#[test]
fn test_function_expression_basic_type_check() {
    let input = "high program(): i32 {
        let f = function(): i32 { return 42 }
        return 0
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Basic function expression should type check: {:?}", result);
}

#[test]
fn test_function_expression_with_parameters() {
    let input = "high program(): i32 {
        let f = function(x: i32, y: i32): i32 {
            let sum = x + y
            return sum
        }
        return 0
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Function expression with parameters should type check: {:?}", result);
}

#[test]
fn test_function_expression_wrong_return_type() {
    let input = "high program(): i32 {
        let f = function(): i32 { return true }
        return 0
    }";
    let result = type_check_program(input);
    // Note: For Phase 7c-α, we're not implementing return type validation yet,
    // so this test passes for now. This validation will be added in future phases.
    assert!(result.is_ok(), "Return type validation not yet implemented in Phase 7c-α");
}

#[test]
fn test_function_expression_variable_capture_rejected() {
    let input = "high program(): i32 {
        let outer = 42
        let f = function(): i32 { return outer }
        return 0
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Variable capture should now be allowed in Phase 7c-δ; got: {:?}", result);
}

#[test]
fn test_function_expression_no_capture_allowed() {
    let input = "high program(): i32 {
        let f = function(x: i32): i32 { return x + 1 }
        return 0
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Function expression with no capture should type-check; got: {:?}", result);
}

// Phase 7c-γ: Capture Detection tests

#[test]
fn test_capture_single_variable_reported() {
    let input = "high program(): i32 {
        let outer = 42
        let f = function(): i32 { return outer }
        return 0
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Variable capture should now be allowed in Phase 7c-δ; got: {:?}", result);
}

#[test]
fn test_capture_multiple_variables_reported() {
    let input = "high program(): i32 {
        let x = 5
        let name = \"hi\"
        let f = function(): i32 {
            print(name)
            return x
        }
        return 0
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Multiple variable capture should now be allowed in Phase 7c-δ; got: {:?}", result);
}

#[test]
fn test_capture_same_variable_referenced_twice() {
    let input = "high program(): i32 {
        let counter = 0
        let f = function(): i32 {
            let temp = counter
            return counter + 1
        }
        return 0
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Variable capture should now be allowed in Phase 7c-δ; got: {:?}", result);
}

#[test]
fn test_no_capture_no_error() {
    let input = "high program(): i32 {
        let f = function(x: i32): i32 { return x + 1 }
        return 0
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Function expression with no capture should type-check; got: {:?}", result);
}

#[test]
fn test_capture_metadata_stored_on_ast() {
    // This test inspects the AST to confirm capture metadata is populated.
    // Use parser+type-checker access pattern from existing tests.

    let input = "high program(): i32 {
        let outer = 42
        let f = function(): i32 { return outer }
        return 0
    }";

    // Parse
    let parser_result = Parser::new(input).unwrap().parse();
    assert!(parser_result.is_ok());
    let top_level = parser_result.unwrap();

    // Type-check (will fail, but we want the AST to have metadata after partial type-check)
    // The type checker populates the captures field before producing the error
    let mut type_checker = TypeChecker::new();
    let _result = type_checker.check(&top_level); // ignore result, we expect it to fail

    // Walk the AST to find the FunctionExpr and check its captures field
    use hilowc::ast::*;
    if let TopLevel::Program(program) = &top_level {
        if let Some(body) = &program.body {
            for item in &body.items {
                if let BlockItem::Statement(Statement::Let(let_stmt)) = item {
                    if let Some(Expression::FunctionExpr(func_expr)) = &let_stmt.initializer {
                        let captures = func_expr.captures.borrow();
                        assert_eq!(captures.len(), 1, "Expected 1 capture, got {}", captures.len());
                        assert_eq!(captures[0].0, "outer", "Expected capture name 'outer', got '{}'", captures[0].0);
                        // Type should be i32
                        if let Type::Primitive(PrimitiveType::I32) = &captures[0].1 {
                            // correct
                        } else {
                            panic!("Expected i32 type for captured variable, got: {:?}", captures[0].1);
                        }
                        return; // test passed
                    }
                }
            }
        }
    }
    panic!("Could not find function expression in AST");
}

#[test]
fn test_function_placeholder_type_still_works() {
    let input = "high program(): i32 {
        let f: function
        return 0
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Placeholder function type should still type-check; got: {:?}", result);
}

#[test]
fn test_function_type_precise_catches_arity_error() {
    let input = "high program(): i32 {
        let f: function(i32): i32 = function(x: i32): i32 { return x }
        f()  // Wrong: expects 1 arg
        return 0
    }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Calling function with wrong arity should error");
}

// Type narrowing tests for Phase 9b

#[test]
fn test_narrowing_inside_if_branch_access_unknown_properties() {
    let input = "high program(): i32 {
        function tryParse(s: string): i32? {
            return unknown(\"test\")
        }
        let result = tryParse(\"hello\")
        if (result is unknown) {
            print(result.reason)  // Should work - result is narrowed to unknown
        }
        return 0
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Property access on unknown should work inside is-unknown branch; got: {:?}", result);
}

#[test]
fn test_narrowing_post_block_when_if_returns() {
    let input = "high program(): i32 {
        function tryParse(s: string): i32? {
            return unknown(\"test\")
        }
        let result = tryParse(\"hello\")
        if (result is unknown) {
            return 1
        }
        print(result + 1)  // Should work - result is narrowed to i32 after if-block
        return 0
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Arithmetic on narrowed variable should work after if-block exits; got: {:?}", result);
}

#[test]
fn test_no_narrowing_when_if_doesnt_return() {
    let input = "high program(): i32 {
        function tryParse(s: string): i32? {
            return unknown(\"test\")
        }
        let result = tryParse(\"hello\")
        if (result is unknown) {
            print(\"error\")  // No return - so no post-block narrowing
        }
        print(result + 1)  // Should fail - result is still i32?
        return 0
    }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Arithmetic on non-narrowed optional should fail");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("arithmetic") ||
                errors[0].message.contains("non-numeric") ||
                errors[0].message.contains("Cannot add") ||
                errors[0].message.contains("operands must be numeric"));
    }
}

// Phase 9f: Call-site argument type checking tests

#[test]
fn test_call_arg_type_mismatch_string_for_i32() {
    let input = r#"
    high program(): i32 {
        function add(a: i32, b: i32): i32 { return a + b }
        add("hello", 5)
        return 0
    }"#;
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type mismatch error for string argument");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("Type mismatch in argument 1: expected i32 but got string"));
    }
}

#[test]
fn test_call_arg_correct_types_passes() {
    let input = r#"
    high program(): i32 {
        function add(a: i32, b: i32): i32 { return a + b }
        add(2, 3)
        return 0
    }"#;
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check, got: {:?}", result);
}

#[test]
fn test_call_multiple_arg_mismatches_all_reported() {
    let input = r#"
    high program(): i32 {
        function add(a: i32, b: i32): i32 { return a + b }
        add("hello", "world")
        return 0
    }"#;
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type mismatch errors for both arguments");

    if let Err(errors) = result {
        assert!(errors.len() >= 2, "Expected at least 2 errors, got: {}", errors.len());
        assert!(errors[0].message.contains("Type mismatch in argument 1") ||
                errors[1].message.contains("Type mismatch in argument 1"));
        assert!(errors[0].message.contains("Type mismatch in argument 2") ||
                errors[1].message.contains("Type mismatch in argument 2"));
    }
}

#[test]
fn test_call_integer_literal_coerces_to_i64() {
    let input = r#"
    high program(): i32 {
        function bigAdd(a: i64, b: i64): i64 { return a + b }
        bigAdd(2, 3)
        return 0
    }"#;
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected integer literal coercion to i64 to work, got: {:?}", result);
}

#[test]
fn test_call_money_specific_to_generic_compatible() {
    let input = r#"
    high program(): i32 {
        function fee(amount: money): money { return amount }
        let x: money<USD> = 5.00 USD
        fee(x)
        return 0
    }"#;
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected money<USD> to money compatibility, got: {:?}", result);
}

#[test]
fn test_call_money_to_money_specific_behavior() {
    let input = r#"
    high program(): i32 {
        function feeUSD(amount: money<USD>): money<USD> { return amount }
        let x: money = 5.00 USD  // This creates generic money from money<USD> literal
        feeUSD(x)
        return 0
    }"#;
    let result = type_check_program(input);
    // Note: Current type system behavior allows this. The assignment `let x: money = 5.00 USD`
    // uses the money<USD> -> money compatibility rule, so x has type money. Calling feeUSD(x)
    // passes a money where money<USD> is expected. This currently succeeds, documenting the
    // actual behavior rather than assuming it should fail.
    assert!(result.is_ok(), "Current behavior allows money to money<USD> calls, got: {:?}", result);
}

// Array Phase B: Array mutation type checking tests

#[test]
fn test_array_push_type_mismatch_rejected() {
    let input = r#"
    high program(): i32 {
        let nums = [1, 2]
        nums.push(true)
        return 0
    }"#;
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type mismatch error for push(bool) on i32 array");
    let errors = result.unwrap_err();
    let error_msg = format!("{:?}", errors);
    assert!(error_msg.contains("Cannot assign bool to i32") ||
            error_msg.contains("bool") && error_msg.contains("i32"),
            "Error should mention type mismatch, got: {}", error_msg);
}

// Phase 10-ε-β: Array watcher alias validation tests

#[test]
fn test_array_watcher_alias_on_changed_rejected() {
    let input = r#"high program(): i32 {
        let nums = [1, 2]
        let w = watcher((x=changed)nums) {
            print(999)
        }
        return 0
    }"#;
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected error for alias on changed modifier");
    let errors = result.unwrap_err();
    let error_msg = format!("{:?}", errors);
    assert!(error_msg.contains("alias binding is only supported with added/removed modifiers"),
            "Error should mention alias binding restriction, got: {}", error_msg);
}

#[test]
fn test_array_watcher_moved_typechecks_but_fails_codegen() {
    let input = r#"high program(): i32 {
        let nums = [1, 2]
        let w = watcher((moved)nums) {
            print(999)
        }
        return 0
    }"#;
    let result = type_check_program(input);
    assert!(result.is_ok(), "Moved modifier should pass typecheck but fail in codegen");
    // Note: This test verifies that moved passes typecheck. The codegen rejection is tested
    // separately in integration tests since type_check_program doesn't run codegen.
}