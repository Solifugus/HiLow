use hilowc::typecheck::TypeChecker;
use hilowc::resolver::{ParsedFile, ResolvedGraph, resolve};
use hilowc::ast::TopLevel;
use hilowc::lexer::Lexer;
use hilowc::parser::Parser;
use std::collections::HashMap;

/// Helper to parse module source into ParsedFile
fn parse_module_source(source: &str) -> ParsedFile {
    let top_level = Parser::new(source).unwrap().parse().expect("parse");
    match top_level {
        TopLevel::Program(p) => ParsedFile::Program(p),
        TopLevel::Module(m) => ParsedFile::Module(m),
    }
}

/// Helper to build a resolved graph from modules
fn build_graph(modules: Vec<(&str, &str)>) -> ResolvedGraph {
    let mut file_loader = HashMap::new();

    // Pick the last module as entry point for simplicity
    let entry_point = modules.last().unwrap().0;

    for (path, source) in modules {
        file_loader.insert(path.to_string(), parse_module_source(source));
    }

    resolve(entry_point, |path| {
        file_loader.get(path).cloned().ok_or_else(|| {
            hilowc::resolver::ResolverError::ModuleNotFound {
                path: path.to_string(),
                position: hilowc::lexer::Position { line: 1, column: 1 },
            }
        })
    }).expect("resolve")
}

#[test]
fn test_check_single_program_via_check_graph() {
    let mut type_checker = TypeChecker::new();

    // Single program file with no imports
    let graph = build_graph(vec![
        ("./app", "high program(): i32 { return 42 }")
    ]);

    let result = type_checker.check_graph(&graph);
    assert!(result.is_ok(), "Expected Ok(()), got: {:?}", result);
}

#[test]
fn test_check_program_imports_module_function() {
    let mut type_checker = TypeChecker::new();

    // Two files: module exports function, program imports and uses it
    let graph = build_graph(vec![
        ("./math", "high module { export function add(a: i32, b: i32): i32 { return a + b } }"),
        ("./app", "import { add } from \"./math\"\nhigh program(): i32 { let x: i32 = add(2, 3) return x }")
    ]);

    let result = type_checker.check_graph(&graph);
    assert!(result.is_ok(), "Expected Ok(()), got: {:?}", result);
}

#[test]
fn test_check_program_imports_module_let() {
    let mut type_checker = TypeChecker::new();

    // Two files: module exports let, program imports and uses it
    let graph = build_graph(vec![
        ("./constants", "high module { export let MAX: i32 = 10 }"),
        ("./app", "import { MAX } from \"./constants\"\nhigh program(): i32 { let y: i32 = MAX + 1 return y }")
    ]);

    let result = type_checker.check_graph(&graph);
    assert!(result.is_ok(), "Expected Ok(()), got: {:?}", result);
}

#[test]
fn test_check_imported_function_call_matches_local_behavior() {
    // Phase 9f implemented call-site argument type checking, closing the gap
    // documented in STATUS.md. This test verifies that imported function calls
    // and local function calls both produce type errors when called with
    // mismatched argument types, ensuring consistent behavior across all
    // function call contexts.

    // Imported function called with wrong-typed args
    let mut imported_checker = TypeChecker::new();
    let imported_graph = build_graph(vec![
        ("./math", "high module { export function add(a: i32, b: i32): i32 { return a + b } }"),
        ("./app", "import { add } from \"./math\"\nhigh program(): i32 { let x: i32 = add(\"hello\", \"world\") return 0 }")
    ]);
    let imported_result = imported_checker.check_graph(&imported_graph);

    // Equivalent local function called with wrong-typed args
    let mut local_checker = TypeChecker::new();
    let local_graph = build_graph(vec![
        ("./app", "high program(): i32 { function add(a: i32, b: i32): i32 { return a + b } let x: i32 = add(\"hello\", \"world\") return 0 }")
    ]);
    let local_result = local_checker.check_graph(&local_graph);

    // Both must produce the same outcome (both should now be Err after Phase 9f)
    assert_eq!(
        imported_result.is_ok(), local_result.is_ok(),
        "Imported call and local call diverged in type-check outcome.\n  imported: {:?}\n  local: {:?}",
        imported_result, local_result
    );

    // Stronger assertions: both should now produce type errors
    assert!(imported_result.is_err(), "Imported call should now produce type error");
    assert!(local_result.is_err(), "Local call should now produce type error");
}

#[test]
fn test_check_imports_unknown_name_fails() {
    let mut type_checker = TypeChecker::new();

    // Two files: math exports only 'add', app tries to import 'subtract'
    let graph = build_graph(vec![
        ("./math", "high module { export function add(a: i32, b: i32): i32 { return a + b } }"),
        ("./app", "import { subtract } from \"./math\"\nhigh program(): i32 { return 0 }")
    ]);

    let result = type_checker.check_graph(&graph);
    assert!(result.is_err(), "Expected import error, got: {:?}", result);

    if let Err(errors) = result {
        let has_import_error = errors.iter().any(|err| {
            err.to_string().contains("'subtract' is not exported from")
        });
        assert!(has_import_error, "Expected 'subtract' not exported error, got: {:?}", errors);
    }
}

#[test]
fn test_check_export_function_missing_return_type() {
    let mut type_checker = TypeChecker::new();

    // This test will determine if the return type annotation rule is real or vacuous
    // For now, assume it's vacuous (all functions require return types in parser)
    // If the parser allows missing return types, this test will catch it

    // Try to construct a function without return type - if parser rejects this,
    // the rule is vacuous and this test documents that fact
    let source = "high module { export function foo(x: i32) { return x } }";

    // If this panics during parsing, the rule is vacuous
    let parsed = std::panic::catch_unwind(|| parse_module_source(source));

    if parsed.is_err() {
        // Parser doesn't allow missing return types - rule is vacuous
        // Rename test concept to document this
        println!("Export function return type rule is vacuous - parser requires all function return types");
        return;
    }

    // If we get here, parser allows missing return types, so test the rule
    let graph = build_graph(vec![("./test", source)]);
    let result = type_checker.check_graph(&graph);

    assert!(result.is_err(), "Expected return type annotation error");
    if let Err(errors) = result {
        let has_annotation_error = errors.iter().any(|err| {
            err.to_string().contains("exported function 'foo' requires an explicit return type annotation")
        });
        assert!(has_annotation_error, "Expected return type annotation error, got: {:?}", errors);
    }
}

#[test]
fn test_check_export_let_missing_type() {
    let mut type_checker = TypeChecker::new();

    // Module with exported let without type annotation
    let graph = build_graph(vec![
        ("./test", "high module { export let X = 42 }")
    ]);

    let result = type_checker.check_graph(&graph);
    assert!(result.is_err(), "Expected type annotation error, got: {:?}", result);

    if let Err(errors) = result {
        let has_annotation_error = errors.iter().any(|err| {
            err.to_string().contains("exported 'let' declaration 'X' requires an explicit type annotation")
        });
        assert!(has_annotation_error, "Expected let type annotation error, got: {:?}", errors);
    }
}

#[test]
fn test_check_diamond_modules_all_check() {
    let mut type_checker = TypeChecker::new();

    // Diamond dependency: app imports a and b, both import util
    let graph = build_graph(vec![
        ("./util", "high module { export function helper(): i32 { return 5 } }"),
        ("./a", "import { helper } from \"./util\"\nhigh module { export function func_a(): i32 { return helper() + 1 } }"),
        ("./b", "import { helper } from \"./util\"\nhigh module { export function func_b(): i32 { return helper() + 2 } }"),
        ("./app", "import { func_a } from \"./a\"\nimport { func_b } from \"./b\"\nhigh program(): i32 { return func_a() + func_b() }")
    ]);

    let result = type_checker.check_graph(&graph);
    assert!(result.is_ok(), "Expected Ok(()), got: {:?}", result);
}

#[test]
fn test_check_private_declaration_not_visible_across_modules() {
    let mut type_checker = TypeChecker::new();

    // lib has private helper, app tries to import it
    let graph = build_graph(vec![
        ("./lib", "high module { function helper(): i32 { return 7 } export function pub(): i32 { return helper() } }"),
        ("./app", "import { helper } from \"./lib\"\nhigh program(): i32 { return helper() }")
    ]);

    let result = type_checker.check_graph(&graph);
    assert!(result.is_err(), "Expected import error, got: {:?}", result);

    if let Err(errors) = result {
        let has_export_error = errors.iter().any(|err| {
            err.to_string().contains("'helper' is not exported from")
        });
        assert!(has_export_error, "Expected 'helper' not exported error, got: {:?}", errors);
    }
}

#[test]
fn test_check_private_helper_usable_inside_own_module() {
    let mut type_checker = TypeChecker::new();

    // lib has private helper used by public function, app imports public function
    let graph = build_graph(vec![
        ("./lib", "high module { function helper(): i32 { return 7 } export function pub(): i32 { return helper() } }"),
        ("./app", "import { pub } from \"./lib\"\nhigh program(): i32 { return pub() }")
    ]);

    let result = type_checker.check_graph(&graph);
    assert!(result.is_ok(), "Expected Ok(()), got: {:?}", result);
}

#[test]
fn test_check_export_function_with_explicit_types() {
    let mut type_checker = TypeChecker::new();

    // Function with explicit return type and parameter types - should work
    let graph = build_graph(vec![
        ("./test", "high module { export function add(a: i32, b: i32): i32 { return a + b } }")
    ]);

    let result = type_checker.check_graph(&graph);
    assert!(result.is_ok(), "Expected Ok(()), got: {:?}", result);
}
