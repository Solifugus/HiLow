use clap::Parser;

pub mod lexer;
pub mod ast;
pub mod parser;

#[derive(Parser)]
#[command(name = "hilowc")]
#[command(about = "HiLow Programming Language Compiler")]
struct Cli {
    /// Input file to compile
    file: String,
}

fn main() {
    let cli = Cli::parse();

    // For now, just acknowledge the file and print stub message
    let _ = cli.file;
    println!("HiLow compiler v0.1 (stub)");
}