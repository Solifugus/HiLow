use hilowc::parser::Parser;
use hilowc::typecheck::TypeChecker;
use hilowc::codegen::CodeGenerator;

fn main() {
    let input = "high program(): i32 { x.y }";
    let ast = Parser::new(input).unwrap().parse().unwrap();
    let type_checker = TypeChecker::new();

    let mut codegen = CodeGenerator::new();
    let result = codegen.generate(&ast, &type_checker);

    match result {
        Ok(_) => println!("Codegen succeeded (unexpected)"),
        Err(e) => println!("Codegen error: {}", e),
    }
}