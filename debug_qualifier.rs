use hilowc::parser::Parser;
use hilowc::typecheck::TypeChecker;

fn main() {
    let input = "high program(): i32 { let a = 5; let b = 5; if (a (or)= b) { } }";
    let result = Parser::new(input).unwrap().parse();
    let top_level = result.unwrap();

    let mut type_checker = TypeChecker::new();
    let check_result = type_checker.check(&top_level);

    if let Err(errors) = check_result {
        for (i, error) in errors.iter().enumerate() {
            println!("Error {}: {}", i, error.to_string());
        }
    } else {
        println!("No errors found!");
    }
}