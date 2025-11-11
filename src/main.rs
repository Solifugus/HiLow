mod lexer;
mod parser;
mod ast;
mod codegen;

use clap::{Parser as ClapParser, Subcommand};
use std::fs;
use std::path::PathBuf;

#[derive(ClapParser)]
#[command(name = "hilowc")]
#[command(about = "The HiLow programming language compiler", long_about = None)]
struct Cli {
    /// Input file to compile
    input: PathBuf,

    /// Output file
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Print tokens (lexer output)
    #[arg(long)]
    print_tokens: bool,

    /// Print AST (parser output)
    #[arg(long)]
    print_ast: bool,

    /// Optimization level (0-3)
    #[arg(short = 'O', default_value = "0")]
    optimization: u8,
}

fn main() {
    let cli = Cli::parse();

    // Read input file
    let source = match fs::read_to_string(&cli.input) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };

    // Lexer
    let mut lexer = lexer::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(tokens) => {
            if cli.print_tokens {
                println!("=== TOKENS ===");
                for token in &tokens {
                    println!("{}", token);
                }
                println!();
            }
            tokens
        }
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            std::process::exit(1);
        }
    };

    // Parser
    let mut parser = parser::Parser::new(tokens);
    let program = match parser.parse() {
        Ok(program) => {
            if cli.print_ast {
                println!("=== AST ===");
                println!("{:#?}", program);
                println!();
            }
            program
        }
        Err(e) => {
            eprintln!("Parser error: {}", e);
            std::process::exit(1);
        }
    };

    // Determine output path
    let output_path = cli.output.unwrap_or_else(|| {
        let mut path = cli.input.clone();
        path.set_extension("");
        path
    });

    // Code generation
    let result = codegen::compile(&program, output_path.to_str().unwrap(), cli.optimization);

    match result {
        Ok(_) => {
            println!("Compilation successful: {}", output_path.display());
        }
        Err(e) => {
            eprintln!("Code generation error: {}", e);
            std::process::exit(1);
        }
    }
}
