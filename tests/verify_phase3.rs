use hilowc::parser::Parser;
use hilowc::typecheck::TypeChecker;
use std::fs;

#[test]
fn test_types1_passes_type_check() {
    let content = fs::read_to_string("tests/programs/phase3/types1.hl")
        .expect("Could not read types1.hl");

    let parse_result = Parser::new(&content).unwrap().parse();
    assert!(parse_result.is_ok(), "types1.hl should parse successfully: {:?}", parse_result);

    let top_level = parse_result.unwrap();
    let mut type_checker = TypeChecker::new();
    let type_result = type_checker.check(&top_level);
    assert!(type_result.is_ok(), "types1.hl should type check successfully: {:?}", type_result);
}

#[test]
fn test_types2_fails_type_check() {
    let content = fs::read_to_string("tests/programs/phase3/types2.hl")
        .expect("Could not read types2.hl");

    let parse_result = Parser::new(&content).unwrap().parse();
    assert!(parse_result.is_ok(), "types2.hl should parse successfully: {:?}", parse_result);

    let top_level = parse_result.unwrap();
    let mut type_checker = TypeChecker::new();
    let type_result = type_checker.check(&top_level);
    assert!(type_result.is_err(), "types2.hl should fail type check (i32 + f64)");

    if let Err(errors) = type_result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("i32"));
        assert!(errors[0].message.contains("f64"));
    }
}

#[test]
fn test_types3_fails_type_check() {
    let content = fs::read_to_string("tests/programs/phase3/types3.hl")
        .expect("Could not read types3.hl");

    let parse_result = Parser::new(&content).unwrap().parse();
    assert!(parse_result.is_ok(), "types3.hl should parse successfully: {:?}", parse_result);

    let top_level = parse_result.unwrap();
    let mut type_checker = TypeChecker::new();
    let type_result = type_checker.check(&top_level);
    assert!(type_result.is_err(), "types3.hl should fail type check (no coercion)");

    if let Err(errors) = type_result {
        assert!(!errors.is_empty());
        // Should fail because bool + i32 is not allowed
        assert!(errors[0].message.contains("bool") || errors[0].message.contains("i32"));
    }
}