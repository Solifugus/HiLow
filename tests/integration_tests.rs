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

// Phase 5b: Qualified operators integration tests

#[test]
fn test_qualified_assign_integration() {
    let executable = compile_program("tests/programs/qualified_assign.hl")
        .expect("Failed to compile qualified_assign.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run qualified_assign");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = "1\n5\n2";
    assert_eq!(stdout.trim(), expected, "Output should match expected result");

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_bad_qualifier_error_handling() {
    let result = compile_program("tests/programs/bad_qualifier.hl");
    assert!(result.is_err(), "Compilation should fail for unknown qualifier");

    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("qualifier 'nonexistent' is not defined"));
}

// TODO: Re-enable this test once context semantics are clarified
// #[test]
// fn test_wrong_context_error_handling() {
//     let result = compile_program("tests/programs/wrong_context.hl");
//     assert!(result.is_err(), "Compilation should fail for qualifier in wrong context");
//
//     let error_msg = result.unwrap_err();
//     assert!(error_msg.contains("qualifier 'or' applies to assignment only, not equality"));
// }

// Phase 6a: String integration tests

#[test]
fn test_strings_basic_integration() {
    let executable = compile_program("tests/programs/strings_basic.hl")
        .expect("Failed to compile strings_basic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run strings_basic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/strings_basic.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_hello_string_integration() {
    let executable = compile_program("tests/programs/hello_string.hl")
        .expect("Failed to compile hello_string.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run hello_string");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/hello_string.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_escape_chars_integration() {
    let executable = compile_program("tests/programs/escape_chars.hl")
        .expect("Failed to compile escape_chars.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run escape_chars");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/escape_chars.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_raw_path_integration() {
    let executable = compile_program("tests/programs/raw_path.hl")
        .expect("Failed to compile raw_path.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run raw_path");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/raw_path.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_multiline_integration() {
    let executable = compile_program("tests/programs/multiline.hl")
        .expect("Failed to compile multiline.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run multiline");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/multiline.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_utf8_integration() {
    let executable = compile_program("tests/programs/utf8_test.hl")
        .expect("Failed to compile utf8_test.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run utf8_test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/utf8_test.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_nested_function_integration() {
    let executable = compile_program("tests/programs/nested_function.hl")
        .expect("Failed to compile nested_function.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run nested_function");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/nested_function.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_hello_fstring_integration() {
    let executable = compile_program("tests/programs/hello_fstring.hl")
        .expect("Failed to compile hello_fstring.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run hello_fstring program");

    // Verify the program ran successfully
    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/hello_fstring.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_arithmetic_fstring_integration() {
    let executable = compile_program("tests/programs/arithmetic_fstring.hl")
        .expect("Failed to compile arithmetic_fstring.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run arithmetic_fstring program");

    // Verify the program ran successfully
    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/arithmetic_fstring.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_format_spec_basic_integration() {
    // This test verifies that format specifiers work correctly (converted from error test in Phase 6b-ii)
    let executable = compile_program("tests/programs/format_spec_basic.hl")
        .expect("Failed to compile format_spec_basic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run format_spec_basic program");

    // Verify the program ran successfully
    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/format_spec_basic.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

// Whitespace preservation regression tests
#[test]
fn test_fstring_whitespace1_integration() {
    let executable = compile_program("tests/programs/fstring_whitespace1.hl")
        .expect("Failed to compile fstring_whitespace1.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run fstring_whitespace1 program");

    // Verify the program ran successfully
    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/fstring_whitespace1.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_fstring_whitespace2_integration() {
    let executable = compile_program("tests/programs/fstring_whitespace2.hl")
        .expect("Failed to compile fstring_whitespace2.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run fstring_whitespace2 program");

    // Verify the program ran successfully
    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/fstring_whitespace2.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_fstring_whitespace3_integration() {
    let executable = compile_program("tests/programs/fstring_whitespace3.hl")
        .expect("Failed to compile fstring_whitespace3.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run fstring_whitespace3 program");

    // Verify the program ran successfully
    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/fstring_whitespace3.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

// Phase 7a object integration tests

#[test]
fn test_object_basic_integration() {
    let executable = compile_program("tests/programs/object_basic.hl")
        .expect("Failed to compile object_basic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run object_basic program");

    // Verify the program ran successfully
    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/object_basic.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_object_assign_integration() {
    let executable = compile_program("tests/programs/object_assign.hl")
        .expect("Failed to compile object_assign.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run object_assign program");

    // Verify the program ran successfully
    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/object_assign.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_object_mixed_types_integration() {
    let executable = compile_program("tests/programs/object_mixed_types.hl")
        .expect("Failed to compile object_mixed_types.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run object_mixed_types program");

    // Verify the program ran successfully
    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/object_mixed_types.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

// Phase 7b: Prototype delegation integration tests

#[test]
fn test_proto_basic_integration() {
    let executable = compile_program("tests/programs/proto_basic.hl")
        .expect("Failed to compile proto_basic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run proto_basic program");

    // Verify the program ran successfully
    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/proto_basic.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_proto_method_integration() {
    let executable = compile_program("tests/programs/proto_method.hl")
        .expect("Failed to compile proto_method.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run proto_method program");

    // Verify the program ran successfully
    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/proto_method.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_proto_override_integration() {
    let executable = compile_program("tests/programs/proto_override.hl")
        .expect("Failed to compile proto_override.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run proto_override program");

    // Verify the program ran successfully
    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/proto_override.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_proto_chain_integration() {
    let executable = compile_program("tests/programs/proto_chain.hl")
        .expect("Failed to compile proto_chain.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run proto_chain program");

    // Verify the program ran successfully
    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/proto_chain.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_proto_assign_local_integration() {
    let executable = compile_program("tests/programs/proto_assign_local.hl")
        .expect("Failed to compile proto_assign_local.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run proto_assign_local program");

    // Verify the program ran successfully
    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/proto_assign_local.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout, expected);

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_is_object_basic_integration() {
    let executable = compile_program("tests/programs/is_object_basic.hl")
        .expect("Failed to compile is_object_basic.hl");

    let expected_output = fs::read_to_string("tests/expected/is_object_basic.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run is_object_basic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_is_object_self_integration() {
    let executable = compile_program("tests/programs/is_object_self.hl")
        .expect("Failed to compile is_object_self.hl");

    let expected_output = fs::read_to_string("tests/expected/is_object_self.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run is_object_self");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_is_object_chain_integration() {
    let executable = compile_program("tests/programs/is_object_chain.hl")
        .expect("Failed to compile is_object_chain.hl");

    let expected_output = fs::read_to_string("tests/expected/is_object_chain.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run is_object_chain");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_is_object_unrelated_integration() {
    let executable = compile_program("tests/programs/is_object_unrelated.hl")
        .expect("Failed to compile is_object_unrelated.hl");

    let expected_output = fs::read_to_string("tests/expected/is_object_unrelated.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run is_object_unrelated");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

// Phase 7c-α: Function expression integration test for codegen deferral
// NOTE: This test was for Phase 7c-α. Now that Phase 7c-β is implemented, function expressions should work.

// Phase 7c-β: Function expression integration tests

#[test]
fn test_func_expr_basic_integration() {
    let executable = compile_program("tests/programs/phase7c-beta/func_expr_basic.hl")
        .expect("Failed to compile func_expr_basic.hl");

    let expected_output = fs::read_to_string("tests/expected/phase7c-beta/func_expr_basic.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run func_expr_basic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_func_expr_param_integration() {
    let executable = compile_program("tests/programs/phase7c-beta/func_expr_param.hl")
        .expect("Failed to compile func_expr_param.hl");

    let expected_output = fs::read_to_string("tests/expected/phase7c-beta/func_expr_param.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run func_expr_param");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_func_expr_two_params_integration() {
    let executable = compile_program("tests/programs/phase7c-beta/func_expr_two_params.hl")
        .expect("Failed to compile func_expr_two_params.hl");

    let expected_output = fs::read_to_string("tests/expected/phase7c-beta/func_expr_two_params.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run func_expr_two_params");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_func_expr_in_object_integration() {
    let executable = compile_program("tests/programs/phase7c-beta/func_expr_in_object.hl")
        .expect("Failed to compile func_expr_in_object.hl");

    let expected_output = fs::read_to_string("tests/expected/phase7c-beta/func_expr_in_object.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run func_expr_in_object");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_func_expr_capture_now_works_integration() {
    // This test verifies that variable capture now works in Phase 7c-δ
    let result = compile_program("tests/programs/phase7c-beta/func_expr_capture_still_rejected.hl");

    // We now expect compilation to SUCCEED since captures are implemented
    assert!(result.is_ok(), "Function expression with capture should now compile successfully; got: {:?}", result);
}

// Phase 7c-δ integration tests for closures with capture

#[test]
fn test_closure_counter_integration() {
    let executable = compile_program("tests/programs/closure_counter.hl")
        .expect("Failed to compile closure_counter.hl");

    let expected_output = fs::read_to_string("tests/expected/closure_counter.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run closure_counter");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_closure_independent_integration() {
    let executable = compile_program("tests/programs/closure_independent.hl")
        .expect("Failed to compile closure_independent.hl");

    let expected_output = fs::read_to_string("tests/expected/closure_independent.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run closure_independent");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_closure_capture_param_integration() {
    let executable = compile_program("tests/programs/closure_capture_param.hl")
        .expect("Failed to compile closure_capture_param.hl");

    let expected_output = fs::read_to_string("tests/expected/closure_capture_param.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run closure_capture_param");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_closure_string_capture_integration() {
    let executable = compile_program("tests/programs/closure_string_capture.hl")
        .expect("Failed to compile closure_string_capture.hl");

    let expected_output = fs::read_to_string("tests/expected/closure_string_capture.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run closure_string_capture");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_closure_no_capture_still_works_integration() {
    let executable = compile_program("tests/programs/closure_no_capture_still_works.hl")
        .expect("Failed to compile closure_no_capture_still_works.hl");

    let expected_output = fs::read_to_string("tests/expected/closure_no_capture_still_works.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run closure_no_capture_still_works");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

// Phase 7c-ε: Method this binding integration tests

#[test]
fn test_method_this_basic_integration() {
    let expected_output = fs::read_to_string("tests/expected/method_this_basic.txt")
        .expect("Expected output file should exist");

    let executable = compile_program("tests/programs/method_this_basic.hl")
        .expect("Failed to compile method_this_basic");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run method_this_basic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_method_this_with_args_integration() {
    let expected_output = fs::read_to_string("tests/expected/method_this_with_args.txt")
        .expect("Expected output file should exist");

    let executable = compile_program("tests/programs/method_this_with_args.hl")
        .expect("Failed to compile method_this_with_args");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run method_this_with_args");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_method_this_proto_integration() {
    let expected_output = fs::read_to_string("tests/expected/method_this_proto.txt")
        .expect("Expected output file should exist");

    let executable = compile_program("tests/programs/method_this_proto.hl")
        .expect("Failed to compile method_this_proto");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run method_this_proto");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_method_this_modifies_integration() {
    let expected_output = fs::read_to_string("tests/expected/method_this_modifies.txt")
        .expect("Expected output file should exist");

    let executable = compile_program("tests/programs/method_this_modifies.hl")
        .expect("Failed to compile method_this_modifies");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run method_this_modifies");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_method_this_outside_method_error_integration() {
    let result = compile_program("tests/programs/method_this_outside_method_error.hl");

    match result {
        Err(error_msg) => {
            assert!(error_msg.contains("this is only valid inside methods"),
                   "Error message should mention that this is only valid inside methods. Got: {}", error_msg);
        }
        Ok(_) => panic!("Expected compilation to fail for this outside method context"),
    }
}

// Phase 7c-ζ: For-in iteration tests

#[test]
fn test_for_in_basic_integration() {
    let expected_output = fs::read_to_string("tests/expected/for_in_basic.txt")
        .expect("Expected output file should exist");

    let executable = compile_program("tests/programs/for_in_basic.hl")
        .expect("Failed to compile for_in_basic");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run for_in_basic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_for_in_mixed_types_integration() {
    let expected_output = fs::read_to_string("tests/expected/for_in_mixed_types.txt")
        .expect("Expected output file should exist");

    let executable = compile_program("tests/programs/for_in_mixed_types.hl")
        .expect("Failed to compile for_in_mixed_types");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run for_in_mixed_types");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_for_in_count_integration() {
    let expected_output = fs::read_to_string("tests/expected/for_in_count.txt")
        .expect("Expected output file should exist");

    let executable = compile_program("tests/programs/for_in_count.hl")
        .expect("Failed to compile for_in_count");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run for_in_count");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_for_in_empty_integration() {
    let expected_output = fs::read_to_string("tests/expected/for_in_empty.txt")
        .expect("Expected output file should exist");

    let executable = compile_program("tests/programs/for_in_empty.hl")
        .expect("Failed to compile for_in_empty");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run for_in_empty");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_for_in_proto_excluded_integration() {
    let expected_output = fs::read_to_string("tests/expected/for_in_proto_excluded.txt")
        .expect("Expected output file should exist");

    let executable = compile_program("tests/programs/for_in_proto_excluded.hl")
        .expect("Failed to compile for_in_proto_excluded");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run for_in_proto_excluded");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

// Match expression integration tests
#[test]
fn test_match_int_integration() {
    let expected_output = fs::read_to_string("tests/expected/match_int.txt")
        .expect("Failed to read expected output");

    let executable = compile_program("tests/programs/match_int.hl")
        .expect("Failed to compile match_int");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run match_int");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_match_int_default_integration() {
    let expected_output = fs::read_to_string("tests/expected/match_int_default.txt")
        .expect("Failed to read expected output");

    let executable = compile_program("tests/programs/match_int_default.hl")
        .expect("Failed to compile match_int_default");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run match_int_default");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_match_string_integration() {
    let expected_output = fs::read_to_string("tests/expected/match_string.txt")
        .expect("Failed to read expected output");

    let executable = compile_program("tests/programs/match_string.hl")
        .expect("Failed to compile match_string");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run match_string");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_match_bool_integration() {
    let expected_output = fs::read_to_string("tests/expected/match_bool.txt")
        .expect("Failed to read expected output");

    let executable = compile_program("tests/programs/match_bool.hl")
        .expect("Failed to compile match_bool");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run match_bool");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_match_as_expression_integration() {
    let expected_output = fs::read_to_string("tests/expected/match_as_expression.txt")
        .expect("Failed to read expected output");

    let executable = compile_program("tests/programs/match_as_expression.hl")
        .expect("Failed to compile match_as_expression");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run match_as_expression");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_match_block_body_integration() {
    let expected_output = fs::read_to_string("tests/expected/match_block_body.txt")
        .expect("Failed to read expected output");

    let executable = compile_program("tests/programs/match_block_body.hl")
        .expect("Failed to compile match_block_body");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run match_block_body");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_match_non_exhaustive_expression_error_integration() {
    let result = compile_program("tests/programs/match_non_exhaustive_expression_error.hl");

    // This should fail to compile
    assert!(result.is_err(), "Expected compilation to fail for non-exhaustive match expression");

    let error_message = result.unwrap_err();
    assert!(error_message.contains("exhaustive") || error_message.contains("wildcard"),
            "Error should mention exhaustiveness or wildcard requirement, got: {}", error_message);
}

#[test]
fn test_switch_int_basic_integration() {
    let executable = compile_program("tests/programs/switch_int_basic.hl")
        .expect("Failed to compile switch_int_basic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run switch_int_basic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/switch_int_basic.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_switch_int_default_integration() {
    let executable = compile_program("tests/programs/switch_int_default.hl")
        .expect("Failed to compile switch_int_default.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run switch_int_default");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/switch_int_default.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_switch_int_fallthrough_integration() {
    let executable = compile_program("tests/programs/switch_int_fallthrough.hl")
        .expect("Failed to compile switch_int_fallthrough.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run switch_int_fallthrough");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/switch_int_fallthrough.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_switch_string_integration() {
    let executable = compile_program("tests/programs/switch_string.hl")
        .expect("Failed to compile switch_string.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run switch_string");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/switch_string.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_switch_bool_integration() {
    let executable = compile_program("tests/programs/switch_bool.hl")
        .expect("Failed to compile switch_bool.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run switch_bool");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/switch_bool.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_switch_no_default_integration() {
    let executable = compile_program("tests/programs/switch_no_default.hl")
        .expect("Failed to compile switch_no_default.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run switch_no_default");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/switch_no_default.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

// Phase 8a: Scope-Based Memory Cleanup Tests

#[test]
fn test_scope_object_leak_free_integration() {
    let executable = compile_program("tests/programs/scope_object_leak_free.hl")
        .expect("Failed to compile scope_object_leak_free.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run scope_object_leak_free");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no leaks)");
    assert!(stderr.is_empty(), "No stderr output expected (no leak messages)");

    let expected_output = fs::read_to_string("tests/expected/scope_object_leak_free.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_scope_nested_block_integration() {
    let executable = compile_program("tests/programs/scope_nested_block.hl")
        .expect("Failed to compile scope_nested_block.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run scope_nested_block");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no leaks)");
    assert!(stderr.is_empty(), "No stderr output expected (no leak messages)");

    let expected_output = fs::read_to_string("tests/expected/scope_nested_block.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_scope_function_returns_object_integration() {
    let executable = compile_program("tests/programs/scope_function_returns_object.hl")
        .expect("Failed to compile scope_function_returns_object.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run scope_function_returns_object");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no leaks)");
    assert!(stderr.is_empty(), "No stderr output expected (no leak messages)");

    let expected_output = fs::read_to_string("tests/expected/scope_function_returns_object.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_scope_fstring_cleanup_integration() {
    let executable = compile_program("tests/programs/scope_fstring_cleanup.hl")
        .expect("Failed to compile scope_fstring_cleanup.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run scope_fstring_cleanup");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no leaks)");
    assert!(stderr.is_empty(), "No stderr output expected (no leak messages)");

    let expected_output = fs::read_to_string("tests/expected/scope_fstring_cleanup.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_scope_inline_fstring_integration() {
    let executable = compile_program("tests/programs/scope_inline_fstring.hl")
        .expect("Failed to compile scope_inline_fstring.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run scope_inline_fstring");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no leaks)");
    assert!(stderr.is_empty(), "No stderr output expected (no leak messages)");

    let expected_output = fs::read_to_string("tests/expected/scope_inline_fstring.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_scope_multi_object_integration() {
    let executable = compile_program("tests/programs/scope_multi_object.hl")
        .expect("Failed to compile scope_multi_object.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run scope_multi_object");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no leaks)");
    assert!(stderr.is_empty(), "No stderr output expected (no leak messages)");

    let expected_output = fs::read_to_string("tests/expected/scope_multi_object.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_scope_object_in_loop_integration() {
    let executable = compile_program("tests/programs/scope_object_in_loop.hl")
        .expect("Failed to compile scope_object_in_loop.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run scope_object_in_loop");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no leaks)");
    assert!(stderr.is_empty(), "No stderr output expected (no leak messages)");

    let expected_output = fs::read_to_string("tests/expected/scope_object_in_loop.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

// Phase 8a fix: Multi-owner rejection tests

#[test]
fn test_accept_function_in_object_integration() {
    let executable = compile_program("tests/programs/accept_function_in_object.hl")
        .expect("Failed to compile accept_function_in_object.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run function in object program");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no leaks)");
    assert!(stderr.is_empty(), "No stderr output expected (no leak messages)");

    let expected_output = fs::read_to_string("tests/expected/accept_function_in_object.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_accept_escaping_closure_integration() {
    let executable = compile_program("tests/programs/accept_escaping_closure.hl")
        .expect("Failed to compile accept_escaping_closure.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run escaping closure program");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no leaks)");
    assert!(stderr.is_empty(), "No stderr output expected (no leak messages)");

    let expected_output = fs::read_to_string("tests/expected/accept_escaping_closure.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_accept_object_alias_integration() {
    let executable = compile_program("tests/programs/accept_object_alias.hl")
        .expect("Failed to compile accept_object_alias.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run object alias program");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no leaks)");
    assert!(stderr.is_empty(), "No stderr output expected (no leak messages)");

    let expected_output = fs::read_to_string("tests/expected/accept_object_alias.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_accept_local_closure_no_capture_integration() {
    let executable = compile_program("tests/programs/accept_local_closure_no_capture.hl")
        .expect("Failed to compile accept_local_closure_no_capture.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run accept_local_closure_no_capture");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no leaks)");
    assert!(stderr.is_empty(), "No stderr output expected (no leak messages)");

    let expected_output = fs::read_to_string("tests/expected/accept_local_closure_no_capture.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_weak_basic_integration() {
    let executable = compile_program("tests/programs/weak_basic.hl")
        .expect("Failed to compile weak_basic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run weak_basic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/weak_basic.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_weak_breaks_cycle_integration() {
    let executable = compile_program("tests/programs/weak_breaks_cycle.hl")
        .expect("Failed to compile weak_breaks_cycle.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run weak_breaks_cycle");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no leaks)");
    assert!(stderr.is_empty(), "No stderr output expected (no leak messages)");

    let expected_output = fs::read_to_string("tests/expected/weak_breaks_cycle.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

// Phase 9a: Nothing type integration tests

#[test]
fn test_nothing_basic_integration() {
    let executable = compile_program("tests/programs/nothing_basic.hl")
        .expect("Failed to compile nothing_basic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run nothing_basic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/nothing_basic.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_nothing_explicit_integration() {
    let executable = compile_program("tests/programs/nothing_explicit.hl")
        .expect("Failed to compile nothing_explicit.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run nothing_explicit");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/nothing_explicit.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_nothing_missing_property_integration() {
    let executable = compile_program("tests/programs/nothing_missing_property.hl")
        .expect("Failed to compile nothing_missing_property.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run nothing_missing_property");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/nothing_missing_property.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_nothing_falsy_integration() {
    let executable = compile_program("tests/programs/nothing_falsy.hl")
        .expect("Failed to compile nothing_falsy.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run nothing_falsy");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/nothing_falsy.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_nothing_print_integration() {
    let executable = compile_program("tests/programs/nothing_print.hl")
        .expect("Failed to compile nothing_print.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run nothing_print");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/nothing_print.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

// Phase 9b: Unknown type integration tests

#[test]
fn test_unknown_basic_integration() {
    let executable = compile_program("tests/programs/unknown_basic.hl")
        .expect("Failed to compile unknown_basic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run unknown_basic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/unknown_basic.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_unknown_optional_return_integration() {
    let executable = compile_program("tests/programs/unknown_optional_return.hl")
        .expect("Failed to compile unknown_optional_return.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run unknown_optional_return");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/unknown_optional_return.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
#[ignore = "Array literals not yet supported (future phase)"]
fn test_unknown_with_options_integration() {
    let executable = compile_program("tests/programs/unknown_with_options.hl")
        .expect("Failed to compile unknown_with_options.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run unknown_with_options");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/unknown_with_options.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_unknown_narrowing_in_else_integration() {
    let executable = compile_program("tests/programs/unknown_narrowing_in_else.hl")
        .expect("Failed to compile unknown_narrowing_in_else.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run unknown_narrowing_in_else");

    assert_eq!(exit_code, 1, "Program should exit with code 1");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/unknown_narrowing_in_else.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_unknown_print_integration() {
    let executable = compile_program("tests/programs/unknown_print.hl")
        .expect("Failed to compile unknown_print.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run unknown_print");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/unknown_print.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

// Phase 9c: Time type integration tests

#[test]
fn test_time_now_integration() {
    let executable = compile_program("tests/programs/time_now.hl")
        .expect("Failed to compile time_now.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run time_now");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/time_now.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_time_arithmetic_integration() {
    let executable = compile_program("tests/programs/time_arithmetic.hl")
        .expect("Failed to compile time_arithmetic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run time_arithmetic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/time_arithmetic.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_duration_arithmetic_integration() {
    let executable = compile_program("tests/programs/duration_arithmetic.hl")
        .expect("Failed to compile duration_arithmetic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run duration_arithmetic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/duration_arithmetic.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_time_comparison_integration() {
    let executable = compile_program("tests/programs/time_comparison.hl")
        .expect("Failed to compile time_comparison.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run time_comparison");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/time_comparison.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_time_precision_compare_integration() {
    let executable = compile_program("tests/programs/time_precision_compare.hl")
        .expect("Failed to compile time_precision_compare.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run time_precision_compare");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/time_precision_compare.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_time_parse_invalid_integration() {
    let executable = compile_program("tests/programs/time_parse_invalid.hl")
        .expect("Failed to compile time_parse_invalid.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run time_parse_invalid");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/time_parse_invalid.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

// Phase 9d: Money tests

#[test]
fn test_money_basic_integration() {
    let executable = compile_program("tests/programs/money_basic.hl")
        .expect("Failed to compile money_basic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run money_basic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/money_basic.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_money_arithmetic_integration() {
    let executable = compile_program("tests/programs/money_arithmetic.hl")
        .expect("Failed to compile money_arithmetic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run money_arithmetic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/money_arithmetic.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_money_multiplication_integration() {
    let executable = compile_program("tests/programs/money_multiplication.hl")
        .expect("Failed to compile money_multiplication.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run money_multiplication");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/money_multiplication.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_money_currencies_integration() {
    let executable = compile_program("tests/programs/money_currencies.hl")
        .expect("Failed to compile money_currencies.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run money_currencies");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/money_currencies.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_money_comparison_integration() {
    let executable = compile_program("tests/programs/money_comparison.hl")
        .expect("Failed to compile money_comparison.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run money_comparison");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/money_comparison.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_reject_money_mismatch_integration() {
    // This test should fail compilation with a currency mismatch error
    let result = compile_program("tests/programs/reject_money_mismatch.hl");

    assert!(result.is_err(), "reject_money_mismatch.hl should fail to compile");

    let error_message = result.unwrap_err();
    assert!(error_message.contains("Cannot mix"), "Error should mention currency mixing: {}", error_message);
    assert!(error_message.contains("USD") && error_message.contains("EUR"),
            "Error should mention both USD and EUR: {}", error_message);
}

// Phase 9e: Tuple tests

#[test]
fn test_tuple_basic_integration() {
    let executable = compile_program("tests/programs/tuple_basic.hl")
        .expect("Failed to compile tuple_basic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run tuple_basic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/tuple_basic.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_tuple_destructuring_integration() {
    let executable = compile_program("tests/programs/tuple_destructuring.hl")
        .expect("Failed to compile tuple_destructuring.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run tuple_destructuring");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/tuple_destructuring.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_tuple_function_return_integration() {
    let executable = compile_program("tests/programs/tuple_function_return.hl")
        .expect("Failed to compile tuple_function_return.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run tuple_function_return");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/tuple_function_return.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_tuple_print_integration() {
    let executable = compile_program("tests/programs/tuple_print.hl")
        .expect("Failed to compile tuple_print.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run tuple_print");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/tuple_print.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_tuple_heterogeneous_integration() {
    let executable = compile_program("tests/programs/tuple_heterogeneous.hl")
        .expect("Failed to compile tuple_heterogeneous.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run tuple_heterogeneous");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected_output = fs::read_to_string("tests/expected/tuple_heterogeneous.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_reject_tuple_arity_mismatch_integration() {
    // This test should fail compilation with an arity mismatch error
    let result = compile_program("tests/programs/reject_tuple_arity_mismatch.hl");

    assert!(result.is_err(), "reject_tuple_arity_mismatch.hl should fail to compile");

    let error_message = result.unwrap_err();
    // The error should mention tuple arity mismatch or similar
    assert!(error_message.contains("arity") || error_message.contains("mismatch") ||
            error_message.contains("variables") || error_message.contains("element"),
            "Error should mention arity/mismatch issue: {}", error_message);
}

#[test]
fn test_modules_basic_integration() {
    let executable = compile_program("tests/programs/modules/basic/app.hl")
        .expect("Failed to compile modules/basic/app.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run modules/basic test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/modules/basic.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_modules_export_let_integration() {
    let executable = compile_program("tests/programs/modules/export_let/app.hl")
        .expect("Failed to compile modules/export_let/app.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run modules/export_let test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/modules/export_let.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_modules_chain_integration() {
    let executable = compile_program("tests/programs/modules/chain/app.hl")
        .expect("Failed to compile modules/chain/app.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run modules/chain test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/modules/chain.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_modules_diamond_integration() {
    let executable = compile_program("tests/programs/modules/diamond/app.hl")
        .expect("Failed to compile modules/diamond/app.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run modules/diamond test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/modules/diamond.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_modules_two_cycle_integration() {
    let executable = compile_program("tests/programs/modules/two_cycle/app.hl")
        .expect("Failed to compile modules/two_cycle/app.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run modules/two_cycle test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/modules/two_cycle.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_modules_three_cycle_integration() {
    let executable = compile_program("tests/programs/modules/three_cycle/app.hl")
        .expect("Failed to compile modules/three_cycle/app.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run modules/three_cycle test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/modules/three_cycle.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_modules_iseven_isodd_integration() {
    let executable = compile_program("tests/programs/modules/iseven_isodd/app.hl")
        .expect("Failed to compile modules/iseven_isodd/app.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run modules/iseven_isodd test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/modules/iseven_isodd.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_watcher_changed_on_i32_fires_on_change() {
    let executable = compile_program("tests/programs/watcher/changed_on_i32/main.hl")
        .expect("Failed to compile watcher/changed_on_i32/main.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher changed_on_i32 test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/changed_on_i32.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_watcher_assigned_fires_every_assignment() {
    let executable = compile_program("tests/programs/watcher/assigned_fires_every/main.hl")
        .expect("Failed to compile watcher/assigned_fires_every/main.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher assigned_fires_every test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/assigned_fires_every.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_watcher_changed_multiple_subscriptions() {
    let executable = compile_program("tests/programs/watcher/changed_multiple_subscriptions/main.hl")
        .expect("Failed to compile watcher/changed_multiple_subscriptions/main.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher changed_multiple_subscriptions test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/changed_multiple_subscriptions.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_watcher_pause_resume() {
    let executable = compile_program("tests/programs/watcher/pause_resume/main.hl")
        .expect("Failed to compile watcher/pause_resume/main.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher pause_resume test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/pause_resume.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_watcher_end_is_permanent() {
    let executable = compile_program("tests/programs/watcher/end_is_permanent/main.hl")
        .expect("Failed to compile watcher/end_is_permanent/main.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher end_is_permanent test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/end_is_permanent.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_watcher_isactive_query() {
    let executable = compile_program("tests/programs/watcher/isactive_query/main.hl")
        .expect("Failed to compile watcher/isactive_query/main.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher isactive_query test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/isactive_query.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

// Phase 10-θ: Nested declarations in blocks + scope-bounded watcher activation

#[test]
fn test_nested_function_in_block() {
    let executable = compile_program("tests/programs/nested_function_in_block.hl")
        .expect("Failed to compile nested_function_in_block.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run nested_function_in_block");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/nested_function_in_block.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_watcher_in_function_body_scope_bounded() {
    let executable = compile_program("tests/programs/watcher/in_function_body_scope_bounded/main.hl")
        .expect("Failed to compile watcher/in_function_body_scope_bounded/main.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher in_function_body_scope_bounded test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/in_function_body_scope_bounded.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_watcher_in_if_branch_scope_bounded() {
    let executable = compile_program("tests/programs/watcher/in_if_branch_scope_bounded/main.hl")
        .expect("Failed to compile watcher/in_if_branch_scope_bounded/main.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher in_if_branch_scope_bounded test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/in_if_branch_scope_bounded.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_watcher_pre_declaration_assignment_does_not_fire() {
    let executable = compile_program("tests/programs/watcher/pre_declaration_assignment_does_not_fire/main.hl")
        .expect("Failed to compile watcher/pre_declaration_assignment_does_not_fire/main.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher pre_declaration_assignment_does_not_fire test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/pre_declaration_assignment_does_not_fire.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

// Phase 10-δ-α: Heap-allocated watcher tests

#[test]
fn test_watcher_expression_basic() {
    let executable = compile_program("tests/programs/watcher/expression_basic/main.hl")
        .expect("Failed to compile test_heap_watcher_allocation.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run heap watcher allocation test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/expression_basic.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_watcher_expression_methods() {
    let executable = compile_program("tests/programs/watcher/expression_methods/main.hl")
        .expect("Failed to compile test_heap_watcher_methods.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run heap watcher methods test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/expression_methods.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_watcher_expression_scope_release() {
    let executable = compile_program("tests/programs/watcher/expression_scope/main.hl")
        .expect("Failed to compile test_heap_watcher_cleanup.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run heap watcher cleanup test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/expression_scope.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_watcher_expression_fires_on_change() {
    let executable = compile_program("tests/programs/watcher/expression_fires/main.hl")
        .expect("Failed to compile test_watcher_expression_fires_on_change.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher expression fires test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/expression_fires.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_watcher_expression_pause_blocks_firing() {
    let executable = compile_program("tests/programs/watcher/expression_pause/main.hl")
        .expect("Failed to compile test_watcher_expression_pause_blocks_firing.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher expression pause test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/expression_pause.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_watcher_expression_coexists_with_declaration_form() {
    let executable = compile_program("tests/programs/watcher/expression_coexists/main.hl")
        .expect("Failed to compile test_watcher_expression_coexists_with_declaration_form.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher expression coexistence test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/expression_coexists.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_watcher_factory_returns_and_fires() {
    let executable = compile_program("tests/programs/watcher/factory_returns_and_fires/main.hl")
        .expect("Failed to compile watcher factory test");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher factory test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/factory_returns_and_fires.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_watcher_factory_with_methods() {
    let executable = compile_program("tests/programs/watcher/factory_with_methods/main.hl")
        .expect("Failed to compile watcher factory with methods test");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher factory with methods test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/factory_with_methods.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_three_level_shadow_probe() {
    let executable = compile_program("tests/programs/phase10a/three_level_shadow_probe.hl")
        .expect("Failed to compile three level shadow probe test");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run three level shadow probe test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/phase10a/three_level_shadow_probe.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

// Array Phase A integration tests

#[test]
fn test_array_literal_and_index() {
    let executable = compile_program("tests/programs/array/literal_and_index.hl")
        .expect("Failed to compile array literal and index test");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array literal and index test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/literal_and_index.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_bool() {
    let executable = compile_program("tests/programs/array/bool.hl")
        .expect("Failed to compile array bool test");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array bool test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/bool.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_float() {
    let executable = compile_program("tests/programs/array/float.hl")
        .expect("Failed to compile array float test");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array float test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/float.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_index_variable() {
    let executable = compile_program("tests/programs/array/index_variable.hl")
        .expect("Failed to compile array index variable test");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array index variable test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/index_variable.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_scope_cleanup() {
    let executable = compile_program("tests/programs/array/scope_cleanup.hl")
        .expect("Failed to compile array scope cleanup test");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array scope cleanup test");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no memory leaks)");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/scope_cleanup.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_element_type_mismatch_rejected() {
    let result = compile_program("tests/programs/array/type_mismatch.hl");

    // This should fail to compile
    assert!(result.is_err(), "Expected compilation to fail for array element type mismatch");

    let error_message = result.unwrap_err();
    assert!(error_message.contains("array elements must all have the same type"),
            "Error should mention array element type mismatch, got: {}", error_message);
}

// Array Phase B: Mutation tests

#[test]
fn test_array_push() {
    let executable = compile_program("tests/programs/array/push.hl")
        .expect("Failed to compile array push test");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array push test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/push.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_index_assign() {
    let executable = compile_program("tests/programs/array/index_assign.hl")
        .expect("Failed to compile array index assign test");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array index assign test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/index_assign.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_pop() {
    let executable = compile_program("tests/programs/array/pop.hl")
        .expect("Failed to compile array pop test");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array pop test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/pop.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_push_pop_sequence() {
    let executable = compile_program("tests/programs/array/push_pop_sequence.hl")
        .expect("Failed to compile array push-pop sequence test");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array push-pop sequence test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/push_pop_sequence.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_mutation_scope_cleanup() {
    let executable = compile_program("tests/programs/array/mutation_scope_cleanup.hl")
        .expect("Failed to compile array mutation scope cleanup test");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array mutation scope cleanup test");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no memory leaks)");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/mutation_scope_cleanup.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    // Clean up
    let _ = fs::remove_file(&executable);
}


#[test]
fn test_array_return_element_from_scope() {
    // Regression test for the use-after-free where a function returned a value
    // derived from a heap-local array (return local[i]). The scope cleanup was
    // releasing the array before the return expression read from it. Fixed by
    // capturing the return value into a temp before emitting cleanup.
    let executable = compile_program("tests/programs/array/return_element_from_scope/main.hl")
        .expect("Failed to compile return_element_from_scope.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run return_element_from_scope test");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no use-after-free, no leak)");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/return_element_from_scope.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_length_comparison() {
    // usize/.length papercut fix: array length (usize) compared against bare
    // integer literals works for both relational (>, <) and equality (?=),
    // and against an explicitly-typed usize variable. Regression guard so the
    // papercut cannot silently return.
    let executable = compile_program("tests/programs/array/length_comparison/main.hl")
        .expect("Failed to compile length_comparison.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run length_comparison test");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/length_comparison.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

// Phase 10-ε-α: Array watcher integration tests

#[test]
fn test_array_watcher_changed_fires_on_push() {
    let executable = compile_program("tests/programs/watcher/test_array_watcher_changed_fires_on_push.hl")
        .expect("Failed to compile test_array_watcher_changed_fires_on_push.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run test_array_watcher_changed_fires_on_push");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/test_array_watcher_changed_fires_on_push.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}


#[test]
fn test_array_watcher_fires_on_index_assign() {
    let executable = compile_program("tests/programs/watcher/test_array_watcher_fires_on_index_assign.hl")
        .expect("Failed to compile test_array_watcher_fires_on_index_assign.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run test_array_watcher_fires_on_index_assign");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/test_array_watcher_fires_on_index_assign.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_watcher_fires_on_pop() {
    let executable = compile_program("tests/programs/watcher/test_array_watcher_fires_on_pop.hl")
        .expect("Failed to compile test_array_watcher_fires_on_pop.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run test_array_watcher_fires_on_pop");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/test_array_watcher_fires_on_pop.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_watcher_alias_fires() {
    let executable = compile_program("tests/programs/watcher/test_array_watcher_alias_fires.hl")
        .expect("Failed to compile test_array_watcher_alias_fires.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run test_array_watcher_alias_fires");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/test_array_watcher_alias_fires.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_watcher_pause_blocks_firing() {
    let executable = compile_program("tests/programs/watcher/test_array_watcher_pause_blocks_firing.hl")
        .expect("Failed to compile test_array_watcher_pause_blocks_firing.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run test_array_watcher_pause_blocks_firing");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/test_array_watcher_pause_blocks_firing.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

// Phase 10-ε-β: Array watcher delta-passing tests

#[test]
fn test_array_watcher_added_with_alias() {
    let executable = "/tmp/test_array_watcher_added_with_alias";
    let output = Command::new("./target/debug/hilowc")
        .arg("tests/programs/watcher/test_array_watcher_added_with_alias.hl")
        .arg("-o")
        .arg(executable)
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        panic!("Compilation failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let result = Command::new(executable)
        .output()
        .expect("Failed to run test program");

    assert!(result.status.success(), "Program execution failed");

    let stdout = String::from_utf8_lossy(&result.stdout);
    let expected = fs::read_to_string("tests/expected/watcher/test_array_watcher_added_with_alias.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_watcher_removed_with_alias() {
    let executable = "/tmp/test_array_watcher_removed_with_alias";
    let output = Command::new("./target/debug/hilowc")
        .arg("tests/programs/watcher/test_array_watcher_removed_with_alias.hl")
        .arg("-o")
        .arg(executable)
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        panic!("Compilation failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let result = Command::new(executable)
        .output()
        .expect("Failed to run test program");

    assert!(result.status.success(), "Program execution failed");

    let stdout = String::from_utf8_lossy(&result.stdout);
    let expected = fs::read_to_string("tests/expected/watcher/test_array_watcher_removed_with_alias.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_watcher_added_no_alias() {
    let executable = "/tmp/test_array_watcher_added_no_alias";
    let output = Command::new("./target/debug/hilowc")
        .arg("tests/programs/watcher/test_array_watcher_added_no_alias.hl")
        .arg("-o")
        .arg(executable)
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        panic!("Compilation failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let result = Command::new(executable)
        .output()
        .expect("Failed to run test program");

    assert!(result.status.success(), "Program execution failed");

    let stdout = String::from_utf8_lossy(&result.stdout);
    let expected = fs::read_to_string("tests/expected/watcher/test_array_watcher_added_no_alias.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_watcher_added_and_changed_both_fire() {
    let executable = "/tmp/test_array_watcher_added_and_changed_both_fire";
    let output = Command::new("./target/debug/hilowc")
        .arg("tests/programs/watcher/test_array_watcher_added_and_changed_both_fire.hl")
        .arg("-o")
        .arg(executable)
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        panic!("Compilation failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let result = Command::new(executable)
        .output()
        .expect("Failed to run test program");

    assert!(result.status.success(), "Program execution failed");

    let stdout = String::from_utf8_lossy(&result.stdout);
    let expected = fs::read_to_string("tests/expected/watcher/test_array_watcher_added_and_changed_both_fire.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_watcher_added_alias_value() {
    let executable = compile_program("tests/programs/watcher/array_watcher_added_alias_value/main.hl")
        .expect("Failed to compile array_watcher_added_alias_value/main.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_watcher_added_alias_value");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/array_watcher_added_alias_value.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_watcher_removed_alias_value() {
    let executable = compile_program("tests/programs/watcher/array_watcher_removed_alias_value/main.hl")
        .expect("Failed to compile array_watcher_removed_alias_value/main.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_watcher_removed_alias_value");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/watcher/array_watcher_removed_alias_value.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

// Array Phase B-2: .remove() and .insert() integration tests

#[test]
fn test_array_remove_value() {
    let executable = compile_program("tests/programs/array/array_remove_value.hl")
        .expect("Failed to compile array_remove_value.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_remove_value");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/array_remove_value.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_insert_value() {
    let executable = compile_program("tests/programs/array/array_insert_value.hl")
        .expect("Failed to compile array_insert_value.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_insert_value");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/array_insert_value.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_remove_no_watcher() {
    let executable = compile_program("tests/programs/array/array_remove_no_watcher.hl")
        .expect("Failed to compile array_remove_no_watcher.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_remove_no_watcher");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/array_remove_no_watcher.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_insert_at_end() {
    let executable = compile_program("tests/programs/array/array_insert_at_end.hl")
        .expect("Failed to compile array_insert_at_end.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_insert_at_end");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/array_insert_at_end.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_remove_fires_removed_watcher() {
    let executable = compile_program("tests/programs/array/array_remove_fires_removed_watcher.hl")
        .expect("Failed to compile array_remove_fires_removed_watcher.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_remove_fires_removed_watcher");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/array_remove_fires_removed_watcher.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_remove_scope_cleanup() {
    let executable = compile_program("tests/programs/array/array_remove_scope_cleanup.hl")
        .expect("Failed to compile array_remove_scope_cleanup.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_remove_scope_cleanup");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no memory leaks)");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/array_remove_scope_cleanup.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_forin_basic() {
    let executable = compile_program("tests/programs/array/array_forin_basic.hl")
        .expect("Failed to compile array_forin_basic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_forin_basic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/array_forin_basic.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_forin_empty() {
    let executable = compile_program("tests/programs/array/array_forin_empty.hl")
        .expect("Failed to compile array_forin_empty.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_forin_empty");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/array_forin_empty.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_forin_sum() {
    let executable = compile_program("tests/programs/array/array_forin_sum.hl")
        .expect("Failed to compile array_forin_sum.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_forin_sum");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/array_forin_sum.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_forin_index_used() {
    let executable = compile_program("tests/programs/array/array_forin_index_used.hl")
        .expect("Failed to compile array_forin_index_used.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_forin_index_used");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/array_forin_index_used.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_forin_mutation_push() {
    let executable = compile_program("tests/programs/array/array_forin_mutation_push.hl")
        .expect("Failed to compile array_forin_mutation_push.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_forin_mutation_push");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/array_forin_mutation_push.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_forin_nested() {
    let executable = compile_program("tests/programs/array/array_forin_nested.hl")
        .expect("Failed to compile array_forin_nested.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_forin_nested");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array/array_forin_nested.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_objects_basic() {
    let executable = compile_program("tests/programs/array_objects_basic.hl")
        .expect("Failed to compile array_objects_basic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_objects_basic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, indicating no memory leaks");

    let expected = fs::read_to_string("tests/expected/array_objects_basic.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_objects_push() {
    let executable = compile_program("tests/programs/array_objects_push.hl")
        .expect("Failed to compile array_objects_push.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_objects_push");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, indicating no memory leaks");

    let expected = fs::read_to_string("tests/expected/array_objects_push.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_objects_pop() {
    let executable = compile_program("tests/programs/array_objects_pop.hl")
        .expect("Failed to compile array_objects_pop.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_objects_pop");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, indicating no memory leaks");

    let expected = fs::read_to_string("tests/expected/array_objects_pop.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_objects_remove() {
    let executable = compile_program("tests/programs/array_objects_remove.hl")
        .expect("Failed to compile array_objects_remove.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_objects_remove");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, indicating no memory leaks");

    let expected = fs::read_to_string("tests/expected/array_objects_remove.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_objects_forin() {
    let executable = compile_program("tests/programs/array_objects_forin.hl")
        .expect("Failed to compile array_objects_forin.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_objects_forin");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, indicating no memory leaks");

    let expected = fs::read_to_string("tests/expected/array_objects_forin.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_objects_scope_cleanup() {
    let executable = compile_program("tests/programs/array_objects_scope_cleanup.hl")
        .expect("Failed to compile array_objects_scope_cleanup.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_objects_scope_cleanup");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, indicating no memory leaks");

    let expected = fs::read_to_string("tests/expected/array_objects_scope_cleanup.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_objects_pop_use() {
    let executable = compile_program("tests/programs/array_objects_pop_use.hl")
        .expect("Failed to compile array_objects_pop_use.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_objects_pop_use");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, indicating no memory leaks");

    let expected = fs::read_to_string("tests/expected/array_objects_pop_use.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_objects_remove_use() {
    let executable = compile_program("tests/programs/array_objects_remove_use.hl")
        .expect("Failed to compile array_objects_remove_use.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_objects_remove_use");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, indicating no memory leaks");

    let expected = fs::read_to_string("tests/expected/array_objects_remove_use.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_primitives_unchanged() {
    let executable = compile_program("tests/programs/array_primitives_unchanged.hl")
        .expect("Failed to compile array_primitives_unchanged.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_primitives_unchanged");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array_primitives_unchanged.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_nested_basic() {
    let executable = compile_program("tests/programs/array_nested_basic.hl")
        .expect("Failed to compile array_nested_basic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_nested_basic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, indicating no memory leaks");

    let expected = fs::read_to_string("tests/expected/array_nested_basic.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_nested_forin() {
    let executable = compile_program("tests/programs/array_nested_forin.hl")
        .expect("Failed to compile array_nested_forin.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_nested_forin");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, indicating no memory leaks");

    let expected = fs::read_to_string("tests/expected/array_nested_forin.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_nested_push() {
    let executable = compile_program("tests/programs/array_nested_push.hl")
        .expect("Failed to compile array_nested_push.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_nested_push");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, indicating no memory leaks");

    let expected = fs::read_to_string("tests/expected/array_nested_push.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_nested_pop() {
    let executable = compile_program("tests/programs/array_nested_pop.hl")
        .expect("Failed to compile array_nested_pop.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_nested_pop");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, indicating no memory leaks");

    let expected = fs::read_to_string("tests/expected/array_nested_pop.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_nested_scope() {
    let executable = compile_program("tests/programs/array_nested_scope.hl")
        .expect("Failed to compile array_nested_scope.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_nested_scope");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, indicating no memory leaks");

    let expected = fs::read_to_string("tests/expected/array_nested_scope.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

// Phase 10-ε-γ: Array move tests

#[test]
fn test_array_move_basic() {
    let executable = compile_program("tests/programs/array_move_basic.hl")
        .expect("Failed to compile array_move_basic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_move_basic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array_move_basic.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_move_backward() {
    let executable = compile_program("tests/programs/array_move_backward.hl")
        .expect("Failed to compile array_move_backward.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_move_backward");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array_move_backward.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_move_no_watcher() {
    let executable = compile_program("tests/programs/array_move_no_watcher.hl")
        .expect("Failed to compile array_move_no_watcher.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_move_no_watcher");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array_move_no_watcher.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_move_objects_no_leak() {
    let executable = compile_program("tests/programs/array_move_objects_no_leak.hl")
        .expect("Failed to compile array_move_objects_no_leak.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_move_objects_no_leak");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, indicating no memory leaks");

    let expected = fs::read_to_string("tests/expected/array_move_objects_no_leak.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_moved_changed_both_fire() {
    let executable = compile_program("tests/programs/array_moved_changed_both_fire.hl")
        .expect("Failed to compile array_moved_changed_both_fire.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_moved_changed_both_fire");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array_moved_changed_both_fire.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_moved_alias_rejected_on_changed() {
    // This test verifies that alias on changed modifier is still rejected
    let result = compile_program("tests/programs/array_moved_alias_rejected_on_changed.hl");
    assert!(result.is_err(), "Expected compilation to fail for alias on changed modifier");
    let error = result.unwrap_err();
    assert!(error.contains("alias binding is only supported with added/removed/moved modifiers"),
            "Expected alias rejection error, got: {}", error);
}

#[test]
fn test_array_clear_basic() {
    let executable = compile_program("tests/programs/array_clear_basic.hl")
        .expect("Failed to compile array_clear_basic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_clear_basic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array_clear_basic.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_clear_objects_no_leak() {
    let executable = compile_program("tests/programs/array_clear_objects_no_leak.hl")
        .expect("Failed to compile array_clear_objects_no_leak.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_clear_objects_no_leak");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, indicating no memory leaks");

    let expected = fs::read_to_string("tests/expected/array_clear_objects_no_leak.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_clear_empty() {
    let executable = compile_program("tests/programs/array_clear_empty.hl")
        .expect("Failed to compile array_clear_empty.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_clear_empty");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array_clear_empty.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_clear_then_reuse() {
    let executable = compile_program("tests/programs/array_clear_then_reuse.hl")
        .expect("Failed to compile array_clear_then_reuse.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_clear_then_reuse");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array_clear_then_reuse.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_clear_changed_fires_not_removed() {
    let executable = compile_program("tests/programs/array_clear_changed_fires_not_removed.hl")
        .expect("Failed to compile array_clear_changed_fires_not_removed.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_clear_changed_fires_not_removed");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/array_clear_changed_fires_not_removed.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_ascription_empty_array_integration() {
    let executable = compile_program("tests/programs/ascription_empty_array.hl")
        .expect("Failed to compile ascription_empty_array.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run ascription_empty_array");

    assert_eq!(exit_code, 0, "Program should exit 0 (no leak)");
    assert!(stderr.is_empty(), "No stderr (no memory leak) expected");

    let expected = fs::read_to_string("tests/expected/ascription_empty_array.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_ascription_old_form_still_works_integration() {
    let executable = compile_program("tests/programs/ascription_old_form_still_works.hl")
        .expect("Failed to compile ascription_old_form_still_works.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run ascription_old_form_still_works");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/ascription_old_form_still_works.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_ascription_numeric_width_integration() {
    let executable = compile_program("tests/programs/ascription_numeric_width.hl")
        .expect("Failed to compile ascription_numeric_width.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run ascription_numeric_width");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/ascription_numeric_width.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_ascription_redundant_integration() {
    let executable = compile_program("tests/programs/ascription_redundant.hl")
        .expect("Failed to compile ascription_redundant.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run ascription_redundant");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/ascription_redundant.expected.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_stealth_basic_scalar() {
    let executable = compile_program("tests/programs/stealth_basic_scalar.hl")
        .expect("Failed to compile stealth_basic_scalar.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run stealth_basic_scalar");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/stealth_basic_scalar.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_stealth_basic_array() {
    let executable = compile_program("tests/programs/stealth_basic_array.hl")
        .expect("Failed to compile stealth_basic_array.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run stealth_basic_array");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/stealth_basic_array.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_stealth_all_array_ops() {
    let executable = compile_program("tests/programs/stealth_all_array_ops.hl")
        .expect("Failed to compile stealth_all_array_ops.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run stealth_all_array_ops");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/stealth_all_array_ops.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_stealth_dynamic() {
    let executable = compile_program("tests/programs/stealth_dynamic.hl")
        .expect("Failed to compile stealth_dynamic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run stealth_dynamic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/stealth_dynamic.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_stealth_nested() {
    let executable = compile_program("tests/programs/stealth_nested.hl")
        .expect("Failed to compile stealth_nested.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run stealth_nested");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/stealth_nested.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_stealth_after_exit() {
    let executable = compile_program("tests/programs/stealth_after_exit.hl")
        .expect("Failed to compile stealth_after_exit.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run stealth_after_exit");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/stealth_after_exit.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_stealth_leak_check() {
    let executable = compile_program("tests/programs/stealth_leak_check.hl")
        .expect("Failed to compile stealth_leak_check.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run stealth_leak_check");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/stealth_leak_check.txt")
        .expect("Failed to read expected output");

    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_scalar_watcher_capture_read_integration() {
    let executable = compile_program("tests/programs/phase10a/scalar_capture_read.hl")
        .expect("Failed to compile scalar_capture_read.hl");

    let expected_output = fs::read_to_string("tests/expected/phase10a/scalar_capture_read.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run scalar_capture_read");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_scalar_watcher_capture_write_integration() {
    let executable = compile_program("tests/programs/phase10a/scalar_capture_write.hl")
        .expect("Failed to compile scalar_capture_write.hl");

    let expected_output = fs::read_to_string("tests/expected/phase10a/scalar_capture_write.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run scalar_capture_write");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_by_reference_sees_current_integration() {
    let executable = compile_program("tests/programs/phase10a/by_reference_sees_current.hl")
        .expect("Failed to compile by_reference_sees_current.hl");

    let expected_output = fs::read_to_string("tests/expected/phase10a/by_reference_sees_current.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run by_reference_sees_current");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_multiple_captures_integration() {
    let executable = compile_program("tests/programs/phase10a/multiple_captures.hl")
        .expect("Failed to compile multiple_captures.hl");

    let expected_output = fs::read_to_string("tests/expected/phase10a/multiple_captures.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run multiple_captures");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_same_name_caller_callee_integration() {
    let executable = compile_program("tests/programs/phase10a/same_name_caller_callee.hl")
        .expect("Failed to compile same_name_caller_callee.hl");

    let expected_output = fs::read_to_string("tests/expected/phase10a/same_name_caller_callee.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run same_name_caller_callee");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_nested_watchers_integration() {
    let executable = compile_program("tests/programs/phase10a/nested_watchers.hl")
        .expect("Failed to compile nested_watchers.hl");

    let expected_output = fs::read_to_string("tests/expected/phase10a/nested_watchers.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run nested_watchers");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_no_capture_regression_integration() {
    let executable = compile_program("tests/programs/phase10a/no_capture_regression.hl")
        .expect("Failed to compile no_capture_regression.hl");

    let expected_output = fs::read_to_string("tests/expected/phase10a/no_capture_regression.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run no_capture_regression");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_capture_read_integration() {
    let executable = compile_program("tests/programs/phase10a/array_capture_read.hl")
        .expect("Failed to compile array_capture_read.hl");

    let expected_output = fs::read_to_string("tests/expected/phase10a/array_capture_read.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_capture_read");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_capture_write_integration() {
    let executable = compile_program("tests/programs/phase10a/array_capture_write.hl")
        .expect("Failed to compile array_capture_write.hl");

    let expected_output = fs::read_to_string("tests/expected/phase10a/array_capture_write.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_capture_write");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_capture_by_reference_integration() {
    let executable = compile_program("tests/programs/phase10a/array_capture_by_reference.hl")
        .expect("Failed to compile array_capture_by_reference.hl");

    let expected_output = fs::read_to_string("tests/expected/phase10a/array_capture_by_reference.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_capture_by_reference");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_capture_multiple_integration() {
    let executable = compile_program("tests/programs/phase10a/array_capture_multiple.hl")
        .expect("Failed to compile array_capture_multiple.hl");

    let expected_output = fs::read_to_string("tests/expected/phase10a/array_capture_multiple.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_capture_multiple");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_watcher_dies_with_scope_integration() {
    let executable = compile_program("tests/programs/phase10a/array_watcher_dies_with_scope.hl")
        .expect("Failed to compile array_watcher_dies_with_scope.hl");

    let expected_output = fs::read_to_string("tests/expected/phase10a/array_watcher_dies_with_scope.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_watcher_dies_with_scope");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_watcher_multiple_fires_integration() {
    let executable = compile_program("tests/programs/phase10a/array_watcher_multiple_fires.hl")
        .expect("Failed to compile array_watcher_multiple_fires.hl");

    let expected_output = fs::read_to_string("tests/expected/phase10a/array_watcher_multiple_fires.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_watcher_multiple_fires");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_capture_no_leak_integration() {
    let executable = compile_program("tests/programs/phase10a/array_capture_no_leak.hl")
        .expect("Failed to compile array_capture_no_leak.hl");

    let expected_output = fs::read_to_string("tests/expected/phase10a/array_capture_no_leak.expected.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_capture_no_leak");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    // Clean up
    let _ = fs::remove_file(&executable);
}


// Managed Strings Sub-phase 1: String-as-tagged-array tests

#[test]
fn test_string_literal_basic() {
    let executable = compile_program("tests/programs/string_literal_basic.hl")
        .expect("Failed to compile string_literal_basic.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run string_literal_basic");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/string_literal_basic.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_string_bytelength() {
    let executable = compile_program("tests/programs/string_bytelength.hl")
        .expect("Failed to compile string_bytelength.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run string_bytelength");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/string_bytelength.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_string_index_byte() {
    let executable = compile_program("tests/programs/string_index_byte.hl")
        .expect("Failed to compile string_index_byte.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run string_index_byte");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/string_index_byte.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_string_equality() {
    let executable = compile_program("tests/programs/string_equality.hl")
        .expect("Failed to compile string_equality.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run string_equality");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/string_equality.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_string_concat() {
    let executable = compile_program("tests/programs/string_concat.hl")
        .expect("Failed to compile string_concat.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run string_concat");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/string_concat.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_string_reassign() {
    let executable = compile_program("tests/programs/string_reassign.hl")
        .expect("Failed to compile string_reassign.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run string_reassign");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/string_reassign.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_string_scope_lifetime() {
    let executable = compile_program("tests/programs/string_scope_lifetime.hl")
        .expect("Failed to compile string_scope_lifetime.hl");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run string_scope_lifetime");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected");

    let expected = fs::read_to_string("tests/expected/string_scope_lifetime.expected.txt")
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim());

    let _ = fs::remove_file(&executable);
}

// Phase 1.5b: control-transfer temp cleanup pins.
// Each program exercises a leak the emitted alloc/free check converts to
// exit 1 + "MEMORY LEAK" on stderr, so the exit-0/empty-stderr assertions
// pin the fix.

#[test]
fn test_return_in_match_arm_temps_integration() {
    let executable = compile_program("tests/programs/return_in_match_arm_temps.hl")
        .expect("Failed to compile return_in_match_arm_temps.hl");

    let expected_output = fs::read_to_string("tests/expected/return_in_match_arm_temps.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run return_in_match_arm_temps");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no leak)");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_return_in_switch_case_temps_integration() {
    let executable = compile_program("tests/programs/return_in_switch_case_temps.hl")
        .expect("Failed to compile return_in_switch_case_temps.hl");

    let expected_output = fs::read_to_string("tests/expected/return_in_switch_case_temps.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run return_in_switch_case_temps");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no leak)");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_for_in_key_break_integration() {
    let executable = compile_program("tests/programs/for_in_key_break.hl")
        .expect("Failed to compile for_in_key_break.hl");

    let expected_output = fs::read_to_string("tests/expected/for_in_key_break.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run for_in_key_break");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no leak)");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_for_in_key_continue_integration() {
    let executable = compile_program("tests/programs/for_in_key_continue.hl")
        .expect("Failed to compile for_in_key_continue.hl");

    let expected_output = fs::read_to_string("tests/expected/for_in_key_continue.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run for_in_key_continue");

    assert_eq!(exit_code, 0, "Program should exit with code 0 (no leak)");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

// Phase 1.5b: format-specifier fixtures promoted to integration tests.
// These pin the spec'd radix/pad/precision format specifiers
// (hilow-design.md string formatting: {n:x}, {n:b}, {n:08d}, {x:.2f}),
// previously unreferenced fixtures broken by the f-string colon being
// parsed as type ascription.

#[test]
fn test_format_binary_integration() {
    let executable = compile_program("tests/programs/format_binary.hl")
        .expect("Failed to compile format_binary.hl");

    let expected_output = fs::read_to_string("tests/expected/format_binary.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run format_binary");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_format_hex_integration() {
    let executable = compile_program("tests/programs/format_hex.hl")
        .expect("Failed to compile format_hex.hl");

    let expected_output = fs::read_to_string("tests/expected/format_hex.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run format_hex");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_format_comprehensive_integration() {
    let executable = compile_program("tests/programs/format_comprehensive.hl")
        .expect("Failed to compile format_comprehensive.hl");

    let expected_output = fs::read_to_string("tests/expected/format_comprehensive.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run format_comprehensive");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

// Phase 1.5c: object ownership discipline — pinning tests for
// double-release-prone patterns. Every store of a heap reference retains;
// every overwrite, removal, and owner death releases exactly once (weak
// references neither retain nor release). These programs are additionally
// covered by the valgrind gate, which requires them to be memory-clean.

#[test]
fn test_object_two_arrays_integration() {
    let executable = compile_program("tests/programs/object_two_arrays.hl")
        .expect("Failed to compile object_two_arrays.hl");

    let expected_output = fs::read_to_string("tests/expected/object_two_arrays.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run object_two_arrays");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_proto_reassign_integration() {
    let executable = compile_program("tests/programs/proto_reassign.hl")
        .expect("Failed to compile proto_reassign.hl");

    let expected_output = fs::read_to_string("tests/expected/proto_reassign.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run proto_reassign");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_array_element_overwrite_integration() {
    let executable = compile_program("tests/programs/array_element_overwrite.hl")
        .expect("Failed to compile array_element_overwrite.hl");

    let expected_output = fs::read_to_string("tests/expected/array_element_overwrite.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_element_overwrite");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_object_shared_two_fields_integration() {
    let executable = compile_program("tests/programs/object_shared_two_fields.hl")
        .expect("Failed to compile object_shared_two_fields.hl");

    let expected_output = fs::read_to_string("tests/expected/object_shared_two_fields.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run object_shared_two_fields");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_object_field_overwrite_integration() {
    let executable = compile_program("tests/programs/object_field_overwrite.hl")
        .expect("Failed to compile object_field_overwrite.hl");

    let expected_output = fs::read_to_string("tests/expected/object_field_overwrite.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run object_field_overwrite");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

#[test]
fn test_object_string_prop_overwrite_integration() {
    let executable = compile_program("tests/programs/object_string_prop_overwrite.hl")
        .expect("Failed to compile object_string_prop_overwrite.hl");

    let expected_output = fs::read_to_string("tests/expected/object_string_prop_overwrite.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run object_string_prop_overwrite");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

// ============================================================
// Phase 1.5d: gap tests (audit §4.4 items 1, 4, 5, 6, 7, 9 + §3.4 latent-bug
// pins per the §5 item 4 adjudication + weak-after-death 1.5e pin).
// Live tests pin current correct behavior. #[ignore]d tests assert
// adjudicated EXPECTED behavior that current machinery does not deliver;
// each names the bug/phase that flips it live. Their programs are carried
// on the valgrind gate's honesty lists (KNOWN_MEMORY_BUGS /
// REJECTION_FIXTURES in tests/valgrind_gate.rs).
// ============================================================

// §4.4 item 4 (scalar variant) — FLIPPED in Phase 3c from pinning the hole
// to pinning the fix: a decl-form watcher on a program-scope variable inside
// a function with an early-return path. Scope exit RELEASES the watcher
// (heap_owners cleanup, which reaches early returns), so mutations after
// BOTH the early-return call and the normal-exit call must not fire; the
// in-scope mutation on the normal path fires once.
#[test]
fn test_watcher_early_return_scalar_integration() {
    let executable = compile_program("tests/programs/watcher_early_return_scalar.hl")
        .expect("Failed to compile watcher_early_return_scalar.hl");

    let expected_output = fs::read_to_string("tests/expected/watcher_early_return_scalar.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher_early_return_scalar");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

// §4.4 item 4 (array variant): watcher registered on a caller-owned array
// inside a function that returns early. The watcher dies with the function
// scope (1.5b early-return cleanup unregisters + frees the env), so the
// caller's later push must NOT fire it.
#[test]
fn test_watcher_early_return_array_integration() {
    let executable = compile_program("tests/programs/watcher_early_return_array.hl")
        .expect("Failed to compile watcher_early_return_array.hl");

    let expected_output = fs::read_to_string("tests/expected/watcher_early_return_array.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher_early_return_array");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

// §4.4 item 5: shadowing + array watchers. Array subscription is by array
// IDENTITY, so a shadowing `xs` in a nested function must not fire the outer
// watcher; the outer array's own push must.
#[test]
fn test_watcher_shadow_array_integration() {
    let executable = compile_program("tests/programs/watcher_shadow_array.hl")
        .expect("Failed to compile watcher_shadow_array.hl");

    let expected_output = fs::read_to_string("tests/expected/watcher_shadow_array.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher_shadow_array");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

// §4.4 item 6: .insert(i, v) fires CHANGED then ADDED (with the added alias
// carrying the inserted value) — pins the runtime firing order and delta.
#[test]
fn test_array_insert_watcher_fires_integration() {
    let executable = compile_program("tests/programs/array_insert_watcher_fires.hl")
        .expect("Failed to compile array_insert_watcher_fires.hl");

    let expected_output = fs::read_to_string("tests/expected/array_insert_watcher_fires.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_insert_watcher_fires");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

// §4.4 item 7: array-watcher factory — a watcher on a caller-owned array,
// returned from the declaring function (no captures), keeps firing after the
// function exits. Mirrors the scalar factory pin (factory_returns_and_fires).
#[test]
fn test_array_watcher_factory_integration() {
    let executable = compile_program("tests/programs/array_watcher_factory.hl")
        .expect("Failed to compile array_watcher_factory.hl");

    let expected_output = fs::read_to_string("tests/expected/array_watcher_factory.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run array_watcher_factory");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

// §4.4 item 9: wire the existing stealth_return_rejected.hl fixture — return
// inside a stealth block is a compile-time rejection.
#[test]
fn test_stealth_return_rejected() {
    let result = compile_program("tests/programs/stealth_return_rejected.hl");

    assert!(result.is_err(), "Expected compilation to fail for return inside stealth block");

    let error_message = result.unwrap_err();
    assert!(error_message.contains("return inside stealth block"),
            "Error should mention return inside stealth block, got: {}", error_message);
}

// (test_string_watcher_rejected flipped LIVE in Phase 3e-α: string watching
// works via the variable-slot cell — see test_watcher_string_changed_fires
// and companions at the end of this file.)

// §4.4 item 1, adjudicated: asserts DEFERRED (declaring-thread queue) firing
// per docs/cell-redesign-brief.md — a mutation made inside a watcher body is
// queued, so the second watcher fires AFTER the first body completes, and the
// first body's delta alias survives the nested mutation. Current machinery
// fires synchronously (nested) — legitimately different until Phase 5 lands
// the notification queues.
#[test]
#[ignore = "asserts deferred (declaring-thread queue) firing per the redesign brief; current firing is synchronous — flips live in Phase 5 (queues)"]
fn test_watcher_reentrant_deferred_integration() {
    let executable = compile_program("tests/programs/watcher_reentrant_deferred.hl")
        .expect("Failed to compile watcher_reentrant_deferred.hl");

    let expected_output = fs::read_to_string("tests/expected/watcher_reentrant_deferred.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher_reentrant_deferred");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

// audit §3.4(a), fixed in Phase 2c: all mutators fire through hl_cell_notify
// with the one (env, cell, delta) body ABI — the .move 2-arg env-dropping
// casts are gone, so a (moved) watcher with captures reads its real env.
#[test]
fn test_watcher_move_capture_env_integration() {
    let executable = compile_program("tests/programs/watcher_move_capture_env.hl")
        .expect("Failed to compile watcher_move_capture_env.hl");

    let expected_output = fs::read_to_string("tests/expected/watcher_move_capture_env.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher_move_capture_env");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

// audit §3.4(b), fixed in Phase 2a: the watcher dies with its scope — its
// release unsubscribes it from BOTH arrays (watcher-identity subscription
// backrefs), so mutating either array afterwards fires nothing.
#[test]
fn test_watcher_multi_array_capture_unregister_integration() {
    let executable = compile_program("tests/programs/watcher_multi_array_capture_unregister.hl")
        .expect("Failed to compile watcher_multi_array_capture_unregister.hl");

    let expected_output = fs::read_to_string("tests/expected/watcher_multi_array_capture_unregister.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher_multi_array_capture_unregister");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

// audit §3.4(c), fixed in Phase 2a: no-capture watchers are no longer keyed
// on the literal "NULL" — scope exit releases the watcher values, which
// unsubscribe themselves, so mutations after scope death fire nothing.
#[test]
fn test_watcher_null_key_scope_death_integration() {
    let executable = compile_program("tests/programs/watcher_null_key_scope_death.hl")
        .expect("Failed to compile watcher_null_key_scope_death.hl");

    let expected_output = fs::read_to_string("tests/expected/watcher_null_key_scope_death.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher_null_key_scope_death");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

// audit §3.4(d), fixed in Phase 2a: an array-watcher expression registers by
// construction (hl_watcher_new_subscribed — one call, any syntactic
// position), so the call-argument form subscribes like any other; the
// temporary watcher's statement-end release unsubscribes it. Later mutation
// fires nothing, no leak, exit 0.
#[test]
fn test_watcher_temp_env_statement_integration() {
    let executable = compile_program("tests/programs/watcher_temp_env_statement.hl")
        .expect("Failed to compile watcher_temp_env_statement.hl");

    let expected_output = fs::read_to_string("tests/expected/watcher_temp_env_statement.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher_temp_env_statement");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

// Weak-after-death, adjudicated 2026-07-15 (audit §5 item 6), implemented in
// Phase 1.5e: reading a weak property whose referent has died yields unknown
// with reason "weak referent released"; member access on it propagates per
// the spec's unknown rules (hilow-design.md "unknown propagates through
// property access").
#[test]
fn test_weak_after_death_unknown_integration() {
    let executable = compile_program("tests/programs/weak_after_death_unknown.hl")
        .expect("Failed to compile weak_after_death_unknown.hl");

    let expected_output = fs::read_to_string("tests/expected/weak_after_death_unknown.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run weak_after_death_unknown");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

// ============================================================================
// Phase 2b step zero (audit §5 item 7): optional inners without a runtime
// payload kind are rejected at compile time citing Phase 3; the narrow
// optional-return type check closes the mis-kinding path.
// ============================================================================

#[test]
fn test_optional_i64_rejected() {
    let result = compile_program("tests/programs/optional_i64_rejected.hl");
    assert!(result.is_err(), "i64? declaration should be rejected");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("optional type 'i64?' is not supported yet")
            && msg.contains("Phase 3 (scalar boxing)"),
        "diagnostic should cite the payload matrix and Phase 3, got: {}",
        msg
    );
}

#[test]
fn test_optional_bool_let_rejected() {
    let result = compile_program("tests/programs/optional_bool_let_rejected.hl");
    assert!(result.is_err(), "bool? let annotation should be rejected");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("optional type 'bool?' is not supported yet")
            && msg.contains("Phase 3 (scalar boxing)"),
        "diagnostic should cite the payload matrix and Phase 3, got: {}",
        msg
    );
}

#[test]
fn test_optional_return_mismatch_rejected() {
    let result = compile_program("tests/programs/optional_return_mismatch_rejected.hl");
    assert!(result.is_err(), "returning string from i32? should be rejected");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("cannot return string from a function declared to return i32?"),
        "diagnostic should name both types, got: {}",
        msg
    );
}

// Phase 2c: the SECOND §3.4(a) firing site — the from==to no-op branch of
// hl_array_move had its own 2-arg env-dropping cast, uncovered by the 1.5d
// fixture (which exercises only the real-move branch). A capturing (moved)
// watcher must read its env and the (from,to) delta correctly there too.
#[test]
fn test_watcher_move_noop_capture_env_integration() {
    let executable = compile_program("tests/programs/watcher_move_noop_capture_env.hl")
        .expect("Failed to compile watcher_move_noop_capture_env.hl");

    let expected_output = fs::read_to_string("tests/expected/watcher_move_noop_capture_env.txt")
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .expect("Failed to run watcher_move_noop_capture_env");

    assert_eq!(exit_code, 0, "Program should exit with code 0");
    assert!(stderr.is_empty(), "No stderr output expected, got: {}", stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "stdout should match expected output");

    let _ = fs::remove_file(&executable);
}

// ---------------------------------------------------------------------------
// Phase 2d: parent lists + (deep) propagation, arrays only. The (deep)
// surface syntax re-entered the language this phase (audit §5 item 3),
// gated on these nested-container tests.
// ---------------------------------------------------------------------------

fn run_deep_fixture(name: &str) {
    let program = format!("tests/programs/{}.hl", name);
    let expected_path = format!("tests/expected/{}.txt", name);
    let executable = compile_program(&program)
        .unwrap_or_else(|e| panic!("Failed to compile {}: {:?}", name, e));

    let expected_output = fs::read_to_string(&expected_path)
        .expect("Failed to read expected output file");

    let (stdout, stderr, exit_code) = run_program(&executable)
        .unwrap_or_else(|e| panic!("Failed to run {}: {:?}", name, e));

    assert_eq!(exit_code, 0, "{} should exit with code 0", name);
    assert!(stderr.is_empty(), "No stderr output expected from {}, got: {}", name, stderr);
    assert_eq!(stdout.trim(), expected_output.trim(), "{} stdout should match expected output", name);

    let _ = fs::remove_file(&executable);
}

// A (deep) watcher on the mutated array itself fires for every mutation
// (spec: "fires when items.push(x), items[0] = y, etc.").
#[test]
fn test_watcher_deep_direct_fires_integration() {
    run_deep_fixture("watcher_deep_direct_fires");
}

// Innermost mutation fires the middle and outermost deep watchers via the
// multi-hop parent walk, inner-to-outer.
#[test]
fn test_watcher_deep_nested_fires_integration() {
    run_deep_fixture("watcher_deep_nested_fires");
}

// The parent walk only goes UP: mutating a's content fires outer's deep
// watcher but never the sibling b's.
#[test]
fn test_watcher_deep_sibling_no_fire_integration() {
    run_deep_fixture("watcher_deep_sibling_no_fire");
}

// The same child stored twice in one parent (two parent-list entries by the
// duplicate policy) fires the parent's deep watcher exactly ONCE per
// mutation — the walk's epoch stamp collapses revisits. This is the
// realizable stand-in for the self-containing-array cycle test: a truly
// self-containing array is unrepresentable in the type system (no recursive
// types), and this exercises the identical termination mechanism.
#[test]
fn test_watcher_deep_diamond_single_fire_integration() {
    run_deep_fixture("watcher_deep_diamond_single_fire");
}

// w.end() stops deep fires; the deliberately-stale deep_watched bit costs a
// silent walk, never a fire (clearing is deferred — STATUS.md).
#[test]
fn test_watcher_deep_unsubscribe_stops_integration() {
    run_deep_fixture("watcher_deep_unsubscribe_stops");
}

// stealth {} suppresses deep fires via hl_cell_notify's single stealth gate.
#[test]
fn test_watcher_deep_stealth_suppressed_integration() {
    run_deep_fixture("watcher_deep_stealth_suppressed");
}

// A child pushed into an already-deep-watched subtree is marked on entry;
// its later mutations walk up and fire.
#[test]
fn test_watcher_deep_new_child_marked_integration() {
    run_deep_fixture("watcher_deep_new_child_marked");
}

// Phase 2d/2e boundary: (deep) on a non-container is rejected until scalars
// gain cells (Phase 3 boxing). Objects joined the containers in Phase 2e.
#[test]
fn test_watcher_deep_scalar_rejected() {
    let result = compile_program("tests/programs/watcher_deep_scalar_rejected.hl");
    assert!(result.is_err(), "Expected compilation to fail for (deep) on a scalar");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("(deep) modifier requires an array or object type in this phase")
            && msg.contains("Phase 3 (boxing)"),
        "Expected the deep-on-scalar diagnostic citing Phase 3, got: {}",
        msg
    );
}

// ---------------------------------------------------------------------------
// Phase 2e: objects join the cell model — parent lists via property/proto
// stores, deep propagation across array↔object containment in both
// directions, and the minimal object event mapping (existing-property set →
// CHANGED; proto reassignment → CHANGED; ADDED reserved for dynamic property
// addition, which is not yet expressible; no REMOVED per the tombstone
// ruling). run_deep_fixture is reused — same compile/run/compare shape.

// Array-valued object properties (enabling work): store, read through the
// borrow getter, mutate through the property.
#[test]
fn test_object_array_property_basic_integration() {
    run_deep_fixture("object_array_property_basic");
}

// (changed) on an object fires on every property set; the body binds the
// object's post-mutation state.
#[test]
fn test_object_watch_changed_fires_integration() {
    run_deep_fixture("object_watch_changed_fires");
}

// Proto is an ordinary property: reassignment fires CHANGED and delegation
// switches to the new prototype.
#[test]
fn test_object_watch_proto_reassign_fires_integration() {
    run_deep_fixture("object_watch_proto_reassign_fires");
}

// Nested object mutation fires the mutated object's own deep watcher
// (own-list DEEP rule) and then the ancestor's, inner-to-outer.
#[test]
fn test_watcher_deep_object_in_object_integration() {
    run_deep_fixture("watcher_deep_object_in_object");
}

// Deep crosses array-in-object: pushing into an object's array property
// fires the holder's deep watcher.
#[test]
fn test_watcher_deep_array_in_object_integration() {
    run_deep_fixture("watcher_deep_array_in_object");
}

// Deep crosses object-in-array: mutating a contained object's property
// fires the array's deep watcher.
#[test]
fn test_watcher_deep_object_in_array_integration() {
    run_deep_fixture("watcher_deep_object_in_array");
}

// Proto links are containment: mutating a prototype fires deep watchers on
// objects that delegate to it.
#[test]
fn test_watcher_deep_proto_chain_fires_integration() {
    run_deep_fixture("watcher_deep_proto_chain_fires");
}

// The same child under two properties of one parent fires the parent's deep
// watcher exactly once per mutation (epoch revisit-suppression; true object
// cycles are unrepresentable — see STATUS.md Phase 2e entry).
#[test]
fn test_watcher_deep_object_diamond_single_fire_integration() {
    run_deep_fixture("watcher_deep_object_diamond_single_fire");
}

// Sibling isolation: mutating one child fires the shared parent but not a
// deep watcher on the sibling; positive control on the sibling itself.
#[test]
fn test_watcher_deep_object_sibling_no_fire_integration() {
    run_deep_fixture("watcher_deep_object_sibling_no_fire");
}

// stealth {} suppresses object fires (and their deep propagation) via
// hl_cell_notify's single gate.
#[test]
fn test_watcher_deep_object_stealth_suppressed_integration() {
    run_deep_fixture("watcher_deep_object_stealth_suppressed");
}

// Adjudicated weak boundary: a weak property creates no parent link, so
// mutation under a weakly-held child does NOT fire the weak holder's deep
// watcher; a strong holder of the same child does fire (positive control).
#[test]
fn test_watcher_deep_object_weak_no_fire_integration() {
    run_deep_fixture("watcher_deep_object_weak_no_fire");
}

// Property overwrite into a deep-watched object marks the entering child
// (its mutations fire) and unlinks exactly one backref from the replaced
// child (its mutations go silent).
#[test]
fn test_watcher_deep_object_new_child_marked_integration() {
    run_deep_fixture("watcher_deep_object_new_child_marked");
}

// Phase 2e adjudication: (added) on objects is rejected while dynamic
// property addition is unexpressible — a subscription that provably cannot
// fire is a trap, not a feature.
#[test]
fn test_object_watch_added_rejected() {
    let result = compile_program("tests/programs/object_watch_added_rejected.hl");
    assert!(result.is_err(), "Expected compilation to fail for (added) on an object");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("(added) on objects is unscheduled")
            && msg.contains("dynamic property addition is unimplemented"),
        "Expected the added-on-object diagnostic, got: {}",
        msg
    );
}

// No REMOVED event exists for objects (tombstone ruling: removal
// unimplemented, indices append-only).
#[test]
fn test_object_watch_removed_rejected() {
    let result = compile_program("tests/programs/object_watch_removed_rejected.hl");
    assert!(result.is_err(), "Expected compilation to fail for (removed) on an object");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("(removed) on objects is unscheduled")
            && msg.contains("tombstone ruling"),
        "Expected the removed-on-object diagnostic, got: {}",
        msg
    );
}

// (assigned) means rebinding-fires — landed in Phase 3e-α via the
// variable-slot cell.
#[test]
fn test_object_watch_assigned_rejected() {
    // Flipped LIVE in Phase 3e-α: (assigned)obj subscribes the variable's
    // slot cell. Real coverage: test_watcher_object_assigned_fires. This
    // test now asserts the OPPOSITE of its old self — the fixture shape
    // compiles — using the live fixture program.
    let result = compile_program("tests/programs/watcher_object_assigned_fires.hl");
    assert!(result.is_ok(), "(assigned)obj should compile as of Phase 3e-α: {:?}", result.err());
    if let Ok(exe) = result {
        let _ = fs::remove_file(&exe);
    }
}

// The body prologue casts the fired cell to the first subscription's
// container type — unsound across mixed containers, so mixed watchers are
// rejected until a per-subscription typed rebind exists.
#[test]
fn test_watcher_mixed_array_object_rejected() {
    let result = compile_program("tests/programs/watcher_mixed_array_object_rejected.hl");
    assert!(result.is_err(), "Expected compilation to fail for a mixed array+object watcher");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("mixed array and object subscriptions in one watcher"),
        "Expected the mixed-container diagnostic, got: {}",
        msg
    );
}

// ===== Phase 3b: boxed scalars lower to cells (hl_cell_set) =====

/// Helper for Phase 3b run-fixtures: compile tests/programs/<name>.hl, run,
/// assert exit 0 / empty stderr / stdout matches tests/expected/<name>.txt.
fn run_3b_fixture(name: &str) {
    let executable = compile_program(&format!("tests/programs/{}.hl", name))
        .unwrap_or_else(|e| panic!("Failed to compile {}: {}", name, e));
    let (stdout, stderr, exit_code) = run_program(&executable)
        .unwrap_or_else(|e| panic!("Failed to run {}: {}", name, e));
    assert_eq!(exit_code, 0, "{}: program should exit with code 0 (stderr: {})", name, stderr);
    assert!(stderr.is_empty(), "{}: no stderr expected, got: {}", name, stderr);
    let expected = fs::read_to_string(&format!("tests/expected/{}.txt", name))
        .expect("Failed to read expected output");
    assert_eq!(stdout.trim(), expected.trim(), "{}: stdout should match expected output", name);
    let _ = fs::remove_file(&executable);
}

// Fire-order ruling (Phase 3b): on one changing assignment, changed
// subscribers fire before assigned subscribers (the legacy firing block's
// order); a same-value assignment fires assigned only.
#[test]
fn test_watcher_changed_assigned_order() {
    run_3b_fixture("watcher_changed_assigned_order");
}

// Adjudication B: compound assignment to a watched scalar is an assignment —
// read payload, apply operator, hl_cell_set. (changed) fires only when the
// result differs; (assigned) fires every time. The Phase 10-γ rejection and
// the silent expression-form no-fire both died with the firing block.
#[test]
fn test_watcher_compound_assign_fires() {
    run_3b_fixture("watcher_compound_assign_fires");
}

// Adjudication C: decl-form watcher bodies may reference outer non-subscribed
// variables (previously emitted C that did not compile). The capture reads
// the cell at fire time — the second fire sees the updated value.
#[test]
fn test_watcher_decl_capture_fires() {
    run_3b_fixture("watcher_decl_capture_fires");
}

// §5 item 1: escape is SOUND. The previously-rejected direct-return shape —
// a watcher capturing a function-local — escapes and keeps firing; the env's
// retain keeps the boxed cell alive (valgrind-clean via the gate).
#[test]
fn test_watcher_escape_capture_sound() {
    run_3b_fixture("watcher_escape_capture_sound");
}

// §5 item 1: a watcher subscribing a function-LOCAL escapes soundly too —
// nothing can mutate the cell after return, so it never fires again; it is
// merely inert, not dangling (the reachability rule's case, now safe).
#[test]
fn test_watcher_escape_subscribed_local_sound() {
    run_3b_fixture("watcher_escape_subscribed_local_sound");
}

// (Adjudication A's rejection flipped LIVE in Phase 3e-β: decl-form
// watchers on container variables work — (changed)/(assigned) subscribe the
// slot, content modifiers follow rebinding via retargeting. The old fixture's
// shape — (changed)xs + push — lives on as
// watcher_decl_container_changed_slot_only; the sentinel now asserts the
// surface compiles and behaves.)
#[test]
fn test_watcher_decl_container_rejected() {
    // Compiles-now assertion on the live fixture that inherited the old
    // sentinel's shape (the 3e-α precedent for flipped rejections).
    run_3b_fixture("watcher_decl_container_changed_slot_only");
}

// Adjudication E: a boxed tuple-destructured binding rejects rather than
// miscompiles (destructured bindings do not box in 3b).
#[test]
fn test_watcher_destructured_binding_rejected() {
    let result = compile_program("tests/programs/watcher_destructured_binding_rejected.hl");
    assert!(result.is_err(), "Expected compilation to fail for watching a destructured binding");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("tuple-destructured binding"),
        "Expected the destructured-binding diagnostic, got: {}",
        msg
    );
}

// ============================================================
// Phase 3c: runtime watcher lifecycle
// ============================================================

// Phase 3c collision pin: a decl-form watcher literally named `w` fires and
// pause/resume work. Under the 3b hidden-variable scheme this segfaulted —
// `hilow_watcher_{id}_w` (hidden var) collided with the body function name
// `hilow_watcher_{id}_{name}` when name == "w", so the uninitialized local
// was passed as the body pointer. Construction under the user's own name
// (Phase 3c) removes the collision class.
#[test]
fn test_watcher_decl_named_w_fires() {
    run_3b_fixture("watcher_decl_named_w_fires");
}

// Phase 3c adjudication A: a declaration-form watcher name is not a
// first-class value — returning it is rejected at compile time.
#[test]
fn test_watcher_decl_name_return_rejected() {
    let result = compile_program("tests/programs/watcher_decl_name_return_rejected.hl");
    assert!(result.is_err(), "Expected compilation to fail for returning a decl-form watcher name");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("declaration-form watcher 'w' supports method calls only"),
        "Expected the decl-form-name diagnostic, got: {}",
        msg
    );
}

// Phase 3c adjudication A: aliasing a declaration-form watcher name via let
// is rejected at compile time.
#[test]
fn test_watcher_decl_name_alias_rejected() {
    let result = compile_program("tests/programs/watcher_decl_name_alias_rejected.hl");
    assert!(result.is_err(), "Expected compilation to fail for let-aliasing a decl-form watcher name");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("declaration-form watcher 'w' supports method calls only"),
        "Expected the decl-form-name diagnostic, got: {}",
        msg
    );
}

// Phase 3c: module-level watcher declarations are rejected — module
// initialization semantics (when they would construct and start observing)
// are not yet specified. Before this rejection they were parser-accepted,
// skipped by the module-graph typecheck path, and died in codegen with an
// internal error.
#[test]
fn test_module_level_watcher_rejected() {
    let result = compile_program("tests/programs/modules/watcher_in_module/app.hl");
    assert!(result.is_err(), "Expected compilation to fail for a module-level watcher");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("watcher declarations are not supported at module level")
            && msg.contains("module initialization semantics are not yet specified"),
        "Expected the module-level-watcher diagnostic, got: {}",
        msg
    );
}

// ============================================================
// Phase 3d: delete name-keyed subscription
// ============================================================

// Phase 3d: watcher method-call inference keys on the variable's TYPE, not
// the deleted watcher_name_to_id map — so BOTH forms infer isActive as
// bool. Before 3d, expression-form `print(w.isActive())` fell through to
// i32 inference and printed 1/0 while decl-form printed true/false (probed;
// no fixture pinned the split). This pins the unified behavior — a forced
// consequence of the map deletion, disclosed in the approved plan.
#[test]
fn test_watcher_expression_isactive_print() {
    run_3b_fixture("watcher_expression_isactive_print");
}

// ============================================================
// Phase 3e-α: variable-slot cells — (assigned) + string watching
// (adjudications: audit §5 item 10; retargeting lands in 3e-β)
// ============================================================

// Slot (changed) on a string fires iff the new value is UNEQUAL under
// string value equality (adjudication 10a): rebinding to an equal value
// (distinct allocation, same bytes) does not fire.
#[test]
fn test_watcher_string_changed_fires() {
    run_3b_fixture("watcher_string_changed_fires");
}

// (assigned)s fires on every assignment including equal-value; with both
// modifiers subscribed, changed fires before assigned on an unequal
// assignment (the strings analogue of watcher_changed_assigned_order).
#[test]
fn test_watcher_string_assigned_fires() {
    run_3b_fixture("watcher_string_assigned_fires");
}

// Decl-form watcher on a string: slot subscription; pause/resume work
// through the shared heap path.
#[test]
fn test_watcher_string_decl_form_fires() {
    run_3b_fixture("watcher_string_decl_form_fires");
}

// (assigned)obj fires on rebinding and NOT on content mutation (the slot
// never sees property sets).
#[test]
fn test_watcher_object_assigned_fires() {
    run_3b_fixture("watcher_object_assigned_fires");
}

// (assigned)xs coexists with a value subscription on the same variable:
// push fires the (added) value watcher; rebinding fires (assigned) only —
// the value subscription stays on the ORIGINAL container by identity
// (adjudication 10b; content-following retargeting is 3e-β). Also kills
// the 3b-era hole where expression-form (assigned)xs compiled and silently
// never fired.
#[test]
fn test_watcher_array_assigned_fires() {
    run_3b_fixture("watcher_array_assigned_fires");
}

// Mixing a slot-kind subscription ((assigned)) with a container value
// subscription in ONE watcher hits the existing mixed-scalar-container
// gate (slot subscriptions route down the scalar/slot body path).
#[test]
fn test_watcher_mixed_assigned_content_rejected() {
    let result = compile_program("tests/programs/watcher_mixed_assigned_content_rejected.hl");
    assert!(result.is_err(), "Expected compilation to fail for mixed slot/value subscriptions");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("mixed scalar and container subscriptions in one watcher"),
        "Expected the mixed-subscriptions diagnostic, got: {}",
        msg
    );
}

// ============================================================
// Phase 3e-β: decl-form content-following on containers —
// subscription retargeting on rebinding (audit §5 item 10b)
// ============================================================

// THE follow proof: a decl-form (v=added)xs watcher fires on content
// mutation before AND after rebinding (the alias binds the delta through
// the retargeted node); mutating the OLD container after rebinding fires
// nothing (its nodes moved to the new container in §5 item 9 order).
#[test]
fn test_watcher_decl_container_follows() {
    run_3b_fixture("watcher_decl_container_follows");
}

// Decl-form (changed)xs is a SLOT subscription (spec: mutating a value in
// place is not an assignment and never fires a variable subscription):
// push is silent; rebinding to a different array fires once. This fixture
// inherits the flipped sentinel's exact shape.
#[test]
fn test_watcher_decl_container_changed_slot_only() {
    run_3b_fixture("watcher_decl_container_changed_slot_only");
}

// Deep-following: decl-form (deep)xs fires for nested mutations, and after
// rebinding the deep-watched bit propagates into the NEW subtree (retarget
// step 4) while the old subtree goes silent.
#[test]
fn test_watcher_decl_container_deep_follows() {
    run_3b_fixture("watcher_decl_container_deep_follows");
}

// The object retarget path (hl_cell_set_object_ref + hl_object_mark_deep):
// same deep-following shape on an object graph.
#[test]
fn test_watcher_decl_object_deep_follows() {
    run_3b_fixture("watcher_decl_object_deep_follows");
}

// Watcher state lives on the watcher object; retargeting moves NODES, not
// state: pause → rebind (nodes still move) → mutation silent → resume →
// new-container mutation fires, old-container mutation stays silent.
#[test]
fn test_watcher_decl_container_pause_retarget() {
    run_3b_fixture("watcher_decl_container_pause_retarget");
}

// Retarget-during-fire soundness, shape A: a slot-subscribed watcher whose
// body (via a companion watcher on a captured trigger) rebinds its own
// variable — the nested same-slot walk is sound and terminates through the
// body's guard; fires in declaration order.
#[test]
fn test_watcher_decl_assigned_rebind_in_body() {
    run_3b_fixture("watcher_decl_assigned_rebind_in_body");
}

// Retarget-during-fire soundness, shape B: a container-following watcher
// whose firing coincides with a rebinding of the followed variable — the
// retarget unlinks nodes from the very cell mid-walk (collect-then-fire
// keeps the walk sound) and the body's snapshot of the OLD container stays
// valid (the deferred old-release), proven by the post-rebind reads and the
// valgrind gate.
#[test]
fn test_watcher_decl_follow_rebind_in_body() {
    run_3b_fixture("watcher_decl_follow_rebind_in_body");
}
