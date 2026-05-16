use hilowc::resolver::{resolve, ResolverError};
use hilowc::ast::{Module, ImportStatement, Mode, TopLevel};
use hilowc::lexer::Position;
use std::collections::HashMap;

// Helper function to create a Module with given imports and empty body
fn make_module(imports: Vec<(&str, Vec<&str>)>) -> TopLevel {
    let import_statements: Vec<ImportStatement> = imports
        .into_iter()
        .map(|(path, names)| ImportStatement {
            path: path.to_string(),
            names: names.iter().map(|s| s.to_string()).collect(),
            position: Position { line: 1, column: 1 },
        })
        .collect();

    TopLevel::Module(Module {
        mode: Mode::High,
        imports: import_statements,
        items: Vec::new(), // Empty items for tests
        lets: Vec::new(), // Empty lets for tests
        watchers: Vec::new(), // Empty watchers for tests
        position: Position { line: 1, column: 1 },
    })
}

#[test]
fn test_resolve_single_file_no_imports() {
    let mut files = HashMap::new();
    files.insert("app".to_string(), make_module(vec![]));

    let parse = |path: &str| -> Result<TopLevel, ResolverError> {
        files.get(path).cloned().ok_or_else(|| ResolverError::ModuleNotFound {
            path: path.to_string(),
            position: Position { line: 1, column: 1 },
        })
    };

    let result = resolve("app", parse);
    assert!(result.is_ok());
    let graph = result.unwrap();
    assert_eq!(graph.topo_order, vec!["app"]);
    assert_eq!(graph.imports.len(), 1);
    assert_eq!(graph.imports.get("app").unwrap(), &Vec::<String>::new());
    assert_eq!(graph.files.len(), 1);
}

#[test]
fn test_resolve_linear_chain() {
    let mut files = HashMap::new();
    files.insert("app".to_string(), make_module(vec![("math", vec![])]));
    files.insert("math".to_string(), make_module(vec![("util", vec![])]));
    files.insert("util".to_string(), make_module(vec![]));

    let parse = |path: &str| -> Result<TopLevel, ResolverError> {
        files.get(path).cloned().ok_or_else(|| ResolverError::ModuleNotFound {
            path: path.to_string(),
            position: Position { line: 1, column: 1 },
        })
    };

    let result = resolve("app", parse);
    assert!(result.is_ok());
    let graph = result.unwrap();
    assert_eq!(graph.topo_order, vec!["util", "math", "app"]);
    assert_eq!(graph.files.len(), 3);
}

#[test]
fn test_resolve_diamond() {
    let mut files = HashMap::new();
    files.insert("app".to_string(), make_module(vec![("a", vec![]), ("b", vec![])]));
    files.insert("a".to_string(), make_module(vec![("util", vec![])]));
    files.insert("b".to_string(), make_module(vec![("util", vec![])]));
    files.insert("util".to_string(), make_module(vec![]));

    let parse = |path: &str| -> Result<TopLevel, ResolverError> {
        files.get(path).cloned().ok_or_else(|| ResolverError::ModuleNotFound {
            path: path.to_string(),
            position: Position { line: 1, column: 1 },
        })
    };

    let result = resolve("app", parse);
    assert!(result.is_ok());
    let graph = result.unwrap();
    assert_eq!(graph.topo_order.len(), 4);
    assert_eq!(graph.topo_order[0], "util");
    assert_eq!(graph.topo_order[3], "app");
    // Verify that both 'a' and 'b' come between util and app
    let a_pos = graph.topo_order.iter().position(|x| x == "a").unwrap();
    let b_pos = graph.topo_order.iter().position(|x| x == "b").unwrap();
    assert!(a_pos > 0 && a_pos < 3);
    assert!(b_pos > 0 && b_pos < 3);
    assert_eq!(graph.files.len(), 4);
}

#[test]
fn test_resolve_self_import_fails() {
    let mut files = HashMap::new();
    files.insert("self".to_string(), make_module(vec![("self", vec![])]));

    let parse = |path: &str| -> Result<TopLevel, ResolverError> {
        files.get(path).cloned().ok_or_else(|| ResolverError::ModuleNotFound {
            path: path.to_string(),
            position: Position { line: 1, column: 1 },
        })
    };

    let result = resolve("self", parse);
    assert!(result.is_err());
    match result.unwrap_err() {
        ResolverError::SelfImport { path, .. } => {
            assert_eq!(path, "self");
        }
        _ => panic!("Expected SelfImport error"),
    }
}

#[test]
fn test_resolve_two_cycle() {
    let mut files = HashMap::new();
    files.insert("even".to_string(), make_module(vec![("odd", vec![])]));
    files.insert("odd".to_string(), make_module(vec![("even", vec![])]));

    let parse = |path: &str| -> Result<TopLevel, ResolverError> {
        files.get(path).cloned().ok_or_else(|| ResolverError::ModuleNotFound {
            path: path.to_string(),
            position: Position { line: 1, column: 1 },
        })
    };

    let result = resolve("even", parse);
    assert!(result.is_ok());
    let graph = result.unwrap();
    assert_eq!(graph.topo_order.len(), 2);
    assert!(graph.topo_order.contains(&"even".to_string()));
    assert!(graph.topo_order.contains(&"odd".to_string()));
    assert_eq!(graph.files.len(), 2);
}

#[test]
fn test_resolve_three_cycle() {
    let mut files = HashMap::new();
    files.insert("a".to_string(), make_module(vec![("b", vec![])]));
    files.insert("b".to_string(), make_module(vec![("c", vec![])]));
    files.insert("c".to_string(), make_module(vec![("a", vec![])]));

    let parse = |path: &str| -> Result<TopLevel, ResolverError> {
        files.get(path).cloned().ok_or_else(|| ResolverError::ModuleNotFound {
            path: path.to_string(),
            position: Position { line: 1, column: 1 },
        })
    };

    let result = resolve("a", parse);
    assert!(result.is_ok());
    let graph = result.unwrap();
    assert_eq!(graph.topo_order.len(), 3);
    assert!(graph.topo_order.contains(&"a".to_string()));
    assert!(graph.topo_order.contains(&"b".to_string()));
    assert!(graph.topo_order.contains(&"c".to_string()));
    assert_eq!(graph.files.len(), 3);
}

#[test]
fn test_resolve_missing_module_fails() {
    let mut files = HashMap::new();
    files.insert("app".to_string(), make_module(vec![("missing", vec![])]));

    let parse = |path: &str| -> Result<TopLevel, ResolverError> {
        files.get(path).cloned().ok_or_else(|| ResolverError::ModuleNotFound {
            path: path.to_string(),
            position: Position { line: 1, column: 1 },
        })
    };

    let result = resolve("app", parse);
    assert!(result.is_err());
    match result.unwrap_err() {
        ResolverError::ModuleNotFound { path, .. } => {
            assert_eq!(path, "missing");
        }
        _ => panic!("Expected ModuleNotFound error"),
    }
}

#[test]
fn test_resolve_deep_chain() {
    let mut files = HashMap::new();
    files.insert("app".to_string(), make_module(vec![("a", vec![])]));
    files.insert("a".to_string(), make_module(vec![("b", vec![])]));
    files.insert("b".to_string(), make_module(vec![("c", vec![])]));
    files.insert("c".to_string(), make_module(vec![("d", vec![])]));
    files.insert("d".to_string(), make_module(vec![]));

    let parse = |path: &str| -> Result<TopLevel, ResolverError> {
        files.get(path).cloned().ok_or_else(|| ResolverError::ModuleNotFound {
            path: path.to_string(),
            position: Position { line: 1, column: 1 },
        })
    };

    let result = resolve("app", parse);
    assert!(result.is_ok());
    let graph = result.unwrap();
    assert_eq!(graph.topo_order, vec!["d", "c", "b", "a", "app"]);
    assert_eq!(graph.files.len(), 5);
}