use hilowc::parser::Parser;
use std::fs;

#[test]
fn test_arith_program_parses() {
    let content = fs::read_to_string("tests/programs/phase2b/arith.hl")
        .expect("Could not read arith.hl");

    let result = Parser::new(&content).unwrap().parse();
    assert!(result.is_ok(), "arith.hl should parse successfully: {:?}", result);
}

#[test]
fn test_control_program_parses() {
    let content = fs::read_to_string("tests/programs/phase2b/control.hl")
        .expect("Could not read control.hl");

    let result = Parser::new(&content).unwrap().parse();
    assert!(result.is_ok(), "control.hl should parse successfully: {:?}", result);
}

#[test]
fn test_equality_program_parses() {
    let content = fs::read_to_string("tests/programs/phase2b/equality.hl")
        .expect("Could not read equality.hl");

    let result = Parser::new(&content).unwrap().parse();
    assert!(result.is_ok(), "equality.hl should parse successfully: {:?}", result);
}