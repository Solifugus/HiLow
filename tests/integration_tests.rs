use std::process::Command;
use std::fs;
use std::time;

/// Helper function to compile a HiLow program and return the path to the executable
fn compile_program(source_path: &str) -> Result<String, String> {
    let output_path = format!("/tmp/test_program_{}_{}", std::process::id(),
                              time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap().as_nanos());

    let output = Command::new("./target/debug/hilowc")
        .arg(source_path)
        .arg("-o")
        .arg(&output_path)
        .output()
        .map_err(|e| format!("Failed to run compiler: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Compilation failed: {}", stderr));
    }

    Ok(output_path)
}

/// Helper function to run a compiled program and return (stdout, stderr, exit_code)
fn run_program(executable_path: &str) -> Result<(String, String, i32), String> {
    let output = Command::new(executable_path)
        .output()
        .map_err(|e| format!("Failed to run program: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok((stdout, stderr, exit_code))
}

#[test]
fn test_hello_int_integration() {
    let executable = compile_program("tests/programs/hello_int.hl")
        .expect("Failed to compile hello_int.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run hello_int");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/hello_int.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_arithmetic_integration() {
    let executable = compile_program("tests/programs/arithmetic.hl")
        .expect("Failed to compile arithmetic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run arithmetic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/arithmetic.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_function_call_integration() {
    let executable = compile_program("tests/programs/function_call.hl")
        .expect("Failed to compile function_call.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run function_call");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/function_call.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_return_value_zero_integration() {
    let executable = compile_program("tests/programs/return_value_zero.hl")
        .expect("Failed to compile return_value_zero.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run return_value_zero");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert!(stdout.trim().is_empty(), "No stdout output expected");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_return_value_nonzero_integration() {
    let executable = compile_program("tests/programs/return_value_nonzero.hl")
        .expect("Failed to compile return_value_nonzero.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run return_value_nonzero");

    assert_eq!(exit_code, 5, "Program should exit with code 5");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert!(stdout.trim().is_empty(), "No stdout output expected");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_compilation_error_handling() {
    // Test a program with a type error
    let temp_file = "/tmp/bad_program.hl";
    fs::write(temp_file, "high program(): i32 { let x = 5 + true }")
        .expect("Failed to write test file");

    let result = compile_program(temp_file);
    assert!(result.is_err(), "Compilation should fail for type error");

    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("Type checking failed") || error_msg.contains("Type error"));

    // Clean up
    let _ = fs::remove_file(temp_file);
}

#[test]
fn test_syntax_error_handling() {
    // Test a program with a syntax error
    let temp_file = "/tmp/syntax_error.hl";
    fs::write(temp_file, "high program(): i32 { let x = }")
        .expect("Failed to write test file");

    let result = compile_program(temp_file);
    assert!(result.is_err(), "Compilation should fail for syntax error");

    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("Parse error") || error_msg.contains("Unexpected"));

    // Clean up
    let _ = fs::remove_file(temp_file);
}

// Phase 4b integration tests

#[test]
fn test_counter_integration() {
    let executable = compile_program("tests/programs/counter.hl")
        .expect("Failed to compile counter.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run counter");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/counter.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_fizzbuzz_numeric_integration() {
    let executable = compile_program("tests/programs/fizzbuzz_numeric.hl")
        .expect("Failed to compile fizzbuzz_numeric.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run fizzbuzz_numeric");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/fizzbuzz_numeric.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_early_exit_integration() {
    let executable = compile_program("tests/programs/early_exit.hl")
        .expect("Failed to compile early_exit.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run early_exit");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/early_exit.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_continue_skip_integration() {
    let executable = compile_program("tests/programs/continue_skip.hl")
        .expect("Failed to compile continue_skip.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run continue_skip");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/continue_skip.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_nested_loops_integration() {
    let executable = compile_program("tests/programs/nested_loops.hl")
        .expect("Failed to compile nested_loops.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run nested_loops");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/nested_loops.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_truthy_integration() {
    let executable = compile_program("tests/programs/truthy.hl")
        .expect("Failed to compile truthy.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run truthy");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/truthy.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_compound_assign_integration() {
    let executable = compile_program("tests/programs/compound_assign.hl")
        .expect("Failed to compile compound_assign.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run compound_assign");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/compound_assign.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

// Phase 5a integration tests

#[test]
fn test_equality_integration() {
    let executable = compile_program("tests/programs/equality.hl")
        .expect("Failed to compile equality.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run equality");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/equality.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_negation_compare_integration() {
    let executable = compile_program("tests/programs/negation_compare.hl")
        .expect("Failed to compile negation_compare.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run negation_compare");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/negation_compare.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_type_mismatch_error_handling() {
    let result = compile_program("tests/programs/type_mismatch.hl");
    assert!(result.is_err(), "Compilation should fail for type mismatch");

    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("Type checking failed") || error_msg.contains("Cannot compare i32 and f64"));
}

#[test]
fn test_bad_equals_error_handling() {
    let result = compile_program("tests/programs/bad_equals.hl");
    assert!(result.is_err(), "Compilation should fail for == operator");

    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("Invalid operator '==' at"));
}
