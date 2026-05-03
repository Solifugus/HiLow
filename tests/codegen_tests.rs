use hilowc::codegen::CodeGenerator;
use hilowc::typecheck::TypeChecker;
use hilowc::parser::Parser;

#[test]
fn test_simple_let_statement_codegen() {
    let input = "high program(): i32 { let x = 42 }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should include the required headers
    assert!(c_code.contains("#include <stdint.h>"));
    assert!(c_code.contains("#include <stdbool.h>"));
    assert!(c_code.contains("#include \"runtime.h\""));

    // Should generate a main function
    assert!(c_code.contains("int main()"));

    // Should generate the let statement as int32_t
    assert!(c_code.contains("int32_t x = 42"));
}

#[test]
fn test_arithmetic_expression_codegen() {
    let input = "high program(): i32 { let x = 1 + 2 * 3 }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should generate arithmetic with proper parentheses
    assert!(c_code.contains("int32_t x = (1 + (2 * 3))"));
}

#[test]
fn test_return_statement_codegen() {
    let input = "high program(): i32 { return 42 }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should generate return statement
    assert!(c_code.contains("return 42"));
}

#[test]
fn test_print_call_codegen() {
    let input = "high program(): i32 { print(42) }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should generate call to print_i32
    assert!(c_code.contains("print_i32(42)"));
}

#[test]
fn test_bool_literal_codegen() {
    let input = "high program(): i32 {
        let flag = true
        print(flag)
    }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should generate bool variable and print_bool call
    assert!(c_code.contains("bool flag = true"));
    assert!(c_code.contains("print_bool(flag)"));
}

#[test]
fn test_float_literal_codegen() {
    let input = "high program(): i32 {
        let pi = 3.14
        print(pi)
    }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should generate double variable and print_f64 call
    assert!(c_code.contains("double pi = 3.14"));
    assert!(c_code.contains("print_f64(pi)"));
}

#[test]
fn test_unsupported_feature_error() {
    let input = "high program(): i32 { x.y }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("member access"));
    assert!(error.to_string().contains("Phase 7"));
}

// Phase 4b codegen tests

#[test]
fn test_if_statement_codegen() {
    let input = "high program(): i32 { if (true) { let x = 1 } }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should generate if statement with proper C syntax
    assert!(c_code.contains("if (true)"));
    assert!(c_code.contains("int32_t x = 1"));
}

#[test]
fn test_if_else_statement_codegen() {
    let input = "high program(): i32 { if (false) { let x = 1 } else { let y = 2 } }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should generate if-else statement
    assert!(c_code.contains("if (false)"));
    assert!(c_code.contains("} else {"));
    assert!(c_code.contains("int32_t x = 1"));
    assert!(c_code.contains("int32_t y = 2"));
}

#[test]
fn test_while_loop_codegen() {
    let input = "high program(): i32 { while (true) { break } }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should generate while loop
    assert!(c_code.contains("while (true)"));
    assert!(c_code.contains("break;"));
}

#[test]
fn test_infinite_loop_codegen() {
    let input = "high program(): i32 { loop { break } }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should generate while(1) for infinite loop
    assert!(c_code.contains("while (1)"));
    assert!(c_code.contains("break;"));
}

#[test]
fn test_continue_statement_codegen() {
    let input = "high program(): i32 { while (true) { continue } }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should generate continue statement
    assert!(c_code.contains("continue;"));
}

#[test]
fn test_compound_assignment_codegen() {
    let input = "high program(): i32 {\n  let x = 5\n  x += 3\n}";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should generate compound assignment
    assert!(c_code.contains("x += 3;"));
}

#[test]
fn test_truthy_numeric_condition_codegen() {
    let input = "high program(): i32 { if (42) { } }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should generate truthy check (x != 0) for numeric condition
    assert!(c_code.contains("if ((42 != 0))"));
}

#[test]
fn test_truthy_variable_condition_codegen() {
    let input = "high program(): i32 { let x = 5\n  if (x) { } }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should generate truthy check for variable condition
    assert!(c_code.contains("if ((x != 0))"));
}

#[test]
fn test_bool_condition_no_truthy_check() {
    let input = "high program(): i32 { let flag = true\n  if (flag) { } }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should NOT add truthy check for bool conditions
    assert!(c_code.contains("if (flag)"));
    assert!(!c_code.contains("flag != 0"));
}

// Phase 5a codegen tests

#[test]
fn test_equality_operators_codegen() {
    let input = "high program(): i32 { let a = 5\n  let b = 6\n  if (a ?= b) { print(1) }\n  if (a != b) { print(2) }\n  return 0 }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should generate equality operators as C == and !=
    assert!(c_code.contains("(a == b)"));
    assert!(c_code.contains("(a != b)"));
}

#[test]
fn test_negation_comparators_codegen() {
    let input = "high program(): i32 { let x = 10\n  let y = 20\n  if (x !< y) { print(1) }\n  if (x !> y) { print(2) }\n  return 0 }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should generate negation comparators as C >= and <=
    assert!(c_code.contains("(x >= y)"));
    assert!(c_code.contains("(x <= y)"));
}

#[test]
fn test_is_check_true_codegen() {
    let input = "high program(): i32 { let x = 42\n  if (x is i32) { print(1) }\n  return 0 }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should generate compile-time true check as literal 1
    assert!(c_code.contains("if (1)"));
}

#[test]
fn test_is_check_false_codegen() {
    let input = "high program(): i32 { let x = 42\n  if (x is f64) { print(1) }\n  return 0 }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    assert!(result.is_ok());
    let c_code = result.unwrap();

    // Should generate compile-time false check as literal 0
    assert!(c_code.contains("if (0)"));
}