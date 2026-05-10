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
