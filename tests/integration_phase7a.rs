// test_phase7a_integration deleted 2026-07-23 (Phase 6a opening commit). It was
// an #[ignore]d unit-style codegen-substring test superseded AT BIRTH: commit
// 186bc9c, which added its ignore, also added the end-to-end fixtures
// object_basic/object_assign/object_mixed_types. Its stale ignore reason
// ("get_expression_type needs symbol table context") no longer holds — that
// program compiles and runs today. It failed only because its assertions
// demanded output the source never generated (hl_object_get_str: it never read
// a string property; print_i32/print_str: it never printed). The string-read
// codegen path it purported to cover (hl_object_get_str) is covered
// end-to-end and superiorly by test_object_mixed_types_integration, which
// reads person.name and asserts the program prints "Alice". See STATUS.md
// (documented-ignores ledger) for the sizing.

use hilowc::parser::Parser;
use hilowc::typecheck::TypeChecker;

#[test]
fn test_phase7a_missing_property_returns_nothing() {
    // Test that accessing non-existent properties returns nothing (Phase 9a behavior)
    let input = "high program(): i32 {
        let obj = { x: 10 }
        let val = obj.y  // Returns nothing (no longer an error)
        return 0
    }";

    let parse_result = Parser::new(input).unwrap().parse();
    assert!(parse_result.is_ok());
    let ast = parse_result.unwrap();

    let mut type_checker = TypeChecker::new();
    let typecheck_result = type_checker.check(&ast);

    // Should now succeed because missing properties return nothing
    assert!(typecheck_result.is_ok(), "Should succeed when accessing non-existent property (Phase 9a)");
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
        // In Phase 9a, missing properties have type nothing, so assignment fails with type error
        assert!(error_msg.contains("Cannot assign i32 to nothing"));
    }
}