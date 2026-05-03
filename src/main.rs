use clap::Parser;
use std::fs;
use std::process::{Command, exit};
use std::path::Path;
use hilowc::{lexer, parser, typecheck, codegen};

#[derive(Parser)]
#[command(name = "hilowc")]
#[command(about = "HiLow Programming Language Compiler")]
struct Cli {
    /// Input file to compile
    file: String,
    /// Output file path
    #[arg(short, long, default_value = "a.out")]
    output: String,
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = compile_program(&cli.file, &cli.output) {
        eprintln!("Error: {}", e);
        exit(1);
    }
}

fn compile_program(input_path: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Read the input file
    let source = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read input file '{}': {}", input_path, e))?;

    // Parse the source
    let mut parser = parser::Parser::new(&source)
        .map_err(|e| format!("Lexer error: {}", e))?;

    let ast = parser.parse()
        .map_err(|e| format!("Parse error: {}", e))?;

    // Type check the AST
    let mut type_checker = typecheck::TypeChecker::new();
    type_checker.check(&ast)
        .map_err(|errors| {
            let mut error_msg = String::from("Type checking failed:\n");
            for error in errors {
                error_msg.push_str(&format!("  {}\n", error));
            }
            error_msg
        })?;

    // Generate C code
    let mut codegen = codegen::CodeGenerator::new();
    let c_code = codegen.generate(&ast, &type_checker)
        .map_err(|e| format!("Code generation error: {}", e))?;

    // Write C code to a temporary file
    let temp_c_file = format!("/tmp/hilow_{}.c", std::process::id());
    fs::write(&temp_c_file, c_code)
        .map_err(|e| format!("Failed to write temporary C file: {}", e))?;

    // Copy runtime files to temporary directory
    let runtime_h_content = include_str!("runtime/runtime.h");
    let runtime_c_content = include_str!("runtime/runtime.c");

    let temp_runtime_h = format!("/tmp/runtime.h");
    let temp_runtime_c = format!("/tmp/hilow_{}_runtime.c", std::process::id());

    fs::write(&temp_runtime_h, runtime_h_content)
        .map_err(|e| format!("Failed to write runtime.h: {}", e))?;
    fs::write(&temp_runtime_c, runtime_c_content)
        .map_err(|e| format!("Failed to write runtime.c: {}", e))?;

    // Compile with cc
    let status = Command::new("cc")
        .arg("-o")
        .arg(output_path)
        .arg(&temp_c_file)
        .arg(&temp_runtime_c)
        .arg("-I/tmp") // Include directory for runtime.h
        .status()
        .map_err(|e| format!("Failed to invoke C compiler: {}", e))?;

    if !status.success() {
        return Err("C compilation failed".into());
    }

    // Clean up temporary files
    let _ = fs::remove_file(&temp_c_file);
    let _ = fs::remove_file(&temp_runtime_h);
    let _ = fs::remove_file(&temp_runtime_c);

    println!("Successfully compiled {} to {}", input_path, output_path);
    Ok(())
}