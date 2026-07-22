use clap::Parser;
use std::fs;
use std::process::{Command, exit};
use std::path::Path;
use std::collections::HashMap;
use hilowc::{parser, typecheck, codegen, resolver, ast::TopLevel, lexer};

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
    let abs_input = std::fs::canonicalize(input_path)?;
    let source = fs::read_to_string(&abs_input)?;
    let mut parser_obj = parser::Parser::new(&source)
        .map_err(|e| format!("Lexer error: {}", e))?;
    let entry_ast = parser_obj.parse()
        .map_err(|e| format!("Parse error: {}", e))?;

    let needs_graph = match &entry_ast {
        TopLevel::Program(p) => !p.imports.is_empty(),
        TopLevel::Module(_) => true,
    };

    if needs_graph {
        compile_graph(&abs_input, entry_ast, output_path)
    } else {
        compile_single_file(entry_ast, output_path)
    }
}

fn compile_single_file(mut ast: TopLevel, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
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

    // Write refinements to the AST for codegen to use
    type_checker.write_refinements_to_ast(&mut ast);

    // Generate C code
    let mut codegen = codegen::CodeGenerator::new();
    let c_code = codegen.generate(&ast, &type_checker)
        .map_err(|e| format!("Code generation error: {}", e))?;

    invoke_cc(c_code, output_path, codegen.uses_async())
}

fn compile_graph(abs_entry_path: &Path, entry_ast: TopLevel, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let TopLevel::Module(_) = &entry_ast {
        return Err("entry file is a module, not a program; module-only entries are not supported".into());
    }

    let entry_dir = abs_entry_path.parent().unwrap();
    let entry_with_abs_imports = rewrite_imports_to_absolute(entry_ast, entry_dir)?;

    // Build callback with a cache and seed with the entry file
    let parse_callback = make_parse_callback();

    let entry_abs_str = abs_entry_path.to_string_lossy().to_string();
    let graph = resolver::resolve(&entry_abs_str, parse_callback)
        .map_err(|e| format!("Resolve error: {:?}", e))?;


    let mut type_checker = typecheck::TypeChecker::new();
    type_checker.check_graph(&graph)
        .map_err(|errors| {
            let mut error_msg = String::from("Type checking failed:\n");
            for error in errors {
                error_msg.push_str(&format!("  {}\n", error));
            }
            error_msg
        })?;

    let mut codegen = codegen::CodeGenerator::new();
    let c_code = codegen.generate_graph(&graph, &type_checker, abs_entry_path)?;

    invoke_cc(c_code, output_path, codegen.uses_async())
}

fn invoke_cc(c_code: String, output_path: &str, threaded: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Create a unique temporary directory per process to avoid race conditions
    let pid = std::process::id();
    let temp_dir = format!("/tmp/hilow_{}", pid);
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temporary directory: {}", e))?;

    // Write C code to the temporary directory
    let temp_c_file = format!("{}/main.c", temp_dir);
    fs::write(&temp_c_file, c_code)
        .map_err(|e| format!("Failed to write temporary C file: {}", e))?;

    // Copy runtime files to the temporary directory
    let runtime_h_content = include_str!("runtime/runtime.h");
    let runtime_c_content = include_str!("runtime/runtime.c");

    let temp_runtime_h = format!("{}/runtime.h", temp_dir);
    let temp_runtime_c = format!("{}/runtime.c", temp_dir);

    fs::write(&temp_runtime_h, runtime_h_content)
        .map_err(|e| format!("Failed to write runtime.h: {}", e))?;
    fs::write(&temp_runtime_c, runtime_c_content)
        .map_err(|e| format!("Failed to write runtime.c: {}", e))?;

    // Compile with cc, using the unique temp directory for includes
    let mut cmd = Command::new("cc");
    cmd.arg("-pthread"); // Phase 5a: thread-local statics + (5b) async pthreads
    // Phase 5b: threaded runtime mode — atomic refcounts in BOTH main.c and
    // runtime.c (one cc invocation, one -D). Only for programs that use async;
    // a single-threaded program gets no -D, so the runtime's refcount macros
    // expand to the exact plain ++/-- and behavior is unchanged.
    if threaded {
        cmd.arg("-DHILOW_THREADED");
    }
    let status = cmd
        .arg("-o")
        .arg(output_path)
        .arg(&temp_c_file)
        .arg(&temp_runtime_c)
        .arg(format!("-I{}", temp_dir)) // Include directory for runtime.h
        .status()
        .map_err(|e| format!("Failed to invoke C compiler: {}", e))?;

    if !status.success() {
        return Err("C compilation failed".into());
    }

    // Clean up the temporary directory and all files
    // Note: Using per-process temp directories prevents race conditions when
    // multiple hilowc invocations run concurrently (e.g., during parallel cargo test)
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

fn rewrite_imports_to_absolute(ast: TopLevel, base_dir: &Path) -> Result<TopLevel, Box<dyn std::error::Error>> {
    match ast {
        TopLevel::Program(mut p) => {
            for import in &mut p.imports {
                let relative_path = &import.path;
                let abs_path = base_dir.join(relative_path).with_extension("hl");
                let canonical = std::fs::canonicalize(&abs_path)
                    .map_err(|e| format!("Failed to resolve import '{}': {}", relative_path, e))?;
                import.path = canonical.to_string_lossy().to_string();
            }
            Ok(TopLevel::Program(p))
        },
        TopLevel::Module(mut m) => {
            for import in &mut m.imports {
                let relative_path = &import.path;
                let abs_path = base_dir.join(relative_path).with_extension("hl");
                let canonical = std::fs::canonicalize(&abs_path)
                    .map_err(|e| format!("Failed to resolve import '{}': {}", relative_path, e))?;
                import.path = canonical.to_string_lossy().to_string();
            }
            Ok(TopLevel::Module(m))
        }
    }
}

fn make_parse_callback() -> impl FnMut(&str) -> Result<TopLevel, resolver::ResolverError> {
    let mut cache: HashMap<String, TopLevel> = HashMap::new();

    move |abs_path: &str| {
        if let Some(cached) = cache.get(abs_path) {
            return Ok(cached.clone());
        }

        let source = fs::read_to_string(abs_path)
            .map_err(|_| resolver::ResolverError::ModuleNotFound {
                path: abs_path.to_string(),
                position: crate::lexer::Position { line: 1, column: 1 },
            })?;

        let mut parser_obj = parser::Parser::new(&source)
            .map_err(|_| resolver::ResolverError::ModuleNotFound {
                path: abs_path.to_string(),
                position: crate::lexer::Position { line: 1, column: 1 },
            })?;

        let mut parsed_ast = parser_obj.parse()
            .map_err(|_| resolver::ResolverError::ModuleNotFound {
                path: abs_path.to_string(),
                position: crate::lexer::Position { line: 1, column: 1 },
            })?;

        let abs_path_obj = Path::new(abs_path);
        let file_dir = abs_path_obj.parent().unwrap();
        let rewritten_ast = rewrite_imports_to_absolute(parsed_ast, file_dir)
            .map_err(|_| resolver::ResolverError::ModuleNotFound {
                path: abs_path.to_string(),
                position: crate::lexer::Position { line: 1, column: 1 },
            })?;

        let result = rewritten_ast;

        cache.insert(abs_path.to_string(), result.clone());
        Ok(result)
    }
}