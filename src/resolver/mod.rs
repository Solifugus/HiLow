use crate::ast::{Program, Module, ImportStatement, TopLevel};
use crate::lexer::Position;
use std::collections::HashMap;


#[derive(Debug, Clone, PartialEq)]
pub enum ResolverError {
    SelfImport {
        path: String,
        position: Position,
    },
    Cycle {
        cycle: Vec<String>,  // ordered, first and last paths are the same to make the cycle obvious
        position: Position,  // position of the import that closed the cycle
    },
    ModuleNotFound {
        path: String,
        position: Position,  // position of the import that referenced the missing module
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedGraph {
    /// Module paths in topological order: leaves (no outgoing imports) first,
    /// entry point last.
    pub topo_order: Vec<String>,

    /// For each module path, the paths it imports from.
    /// imports[&path] is the set of paths `path` imports.
    pub imports: HashMap<String, Vec<String>>,

    /// The parsed file for each module path. Owned here so the caller can
    /// iterate and type-check / codegen later without re-parsing.
    pub files: HashMap<String, TopLevel>,
}

pub fn resolve<F>(entry_path: &str, mut parse: F) -> Result<ResolvedGraph, ResolverError>
where
    F: FnMut(&str) -> Result<TopLevel, ResolverError>,
{
    let mut files = HashMap::new();
    let mut imports = HashMap::new();
    let mut visited = std::collections::HashSet::new();

    // DFS to load all files and build dependency graph
    fn dfs_load<F>(
        path: &str,
        parse: &mut F,
        files: &mut HashMap<String, TopLevel>,
        imports: &mut HashMap<String, Vec<String>>,
        visited: &mut std::collections::HashSet<String>,
        current_path_stack: &mut Vec<String>,
    ) -> Result<(), ResolverError>
    where
        F: FnMut(&str) -> Result<TopLevel, ResolverError>,
    {
        if visited.contains(path) {
            return Ok(()); // Already processed
        }

        // Check for cycle (including self-import)
        if current_path_stack.iter().any(|p| p == path) {
            return Ok(()); // This is a cycle, will be detected later in topological sort
        }

        current_path_stack.push(path.to_string());

        // Parse the file
        let file = parse(path)?;
        let import_statements = file.imports();

        // Check for self-import first
        for import in import_statements {
            if import.path == path {
                return Err(ResolverError::SelfImport {
                    path: path.to_string(),
                    position: import.position.clone(),
                });
            }
        }

        // Extract import paths
        let import_paths: Vec<String> = import_statements
            .iter()
            .map(|import| import.path.clone())
            .collect();

        // Store the file and its imports
        files.insert(path.to_string(), file);
        imports.insert(path.to_string(), import_paths.clone());
        visited.insert(path.to_string());

        // Recursively process imports
        for import_path in import_paths {
            dfs_load(
                &import_path,
                parse,
                files,
                imports,
                visited,
                current_path_stack,
            )?;
        }

        current_path_stack.pop();
        Ok(())
    }

    // Start DFS from entry point
    let mut current_path_stack = Vec::new();
    dfs_load(
        entry_path,
        &mut parse,
        &mut files,
        &mut imports,
        &mut visited,
        &mut current_path_stack,
    )?;

    // Topological sort using Kahn's algorithm to detect cycles
    let mut in_degree = HashMap::new();
    let mut adj_list = HashMap::new();

    // Initialize data structures
    for path in files.keys() {
        in_degree.insert(path.clone(), 0);
        adj_list.insert(path.clone(), Vec::new());
    }

    // Build adjacency list and in-degree count
    for (path, import_list) in &imports {
        for imported_path in import_list {
            adj_list.get_mut(imported_path).unwrap().push(path.clone());
            *in_degree.get_mut(path).unwrap() += 1;
        }
    }

    // Kahn's algorithm
    let mut queue = std::collections::VecDeque::new();
    let mut topo_order = Vec::new();

    // Add all nodes with in-degree 0 to queue
    for (path, &degree) in &in_degree {
        if degree == 0 {
            queue.push_back(path.clone());
        }
    }

    while let Some(node) = queue.pop_front() {
        topo_order.push(node.clone());

        // For each node that depends on current node
        for dependent in &adj_list[&node] {
            let degree = in_degree.get_mut(dependent).unwrap();
            *degree -= 1;
            if *degree == 0 {
                queue.push_back(dependent.clone());
            }
        }
    }

    // Check for cycles
    if topo_order.len() != files.len() {
        // There's a cycle - find it
        let remaining: Vec<String> = files
            .keys()
            .filter(|path| !topo_order.contains(path))
            .cloned()
            .collect();

        // Find a cycle by doing DFS from one of the remaining nodes
        if let Some(start) = remaining.first() {
            let mut cycle_path = Vec::new();
            let mut cycle_visited = std::collections::HashSet::new();

            fn find_cycle_dfs(
                node: &str,
                imports: &HashMap<String, Vec<String>>,
                path: &mut Vec<String>,
                visited: &mut std::collections::HashSet<String>,
            ) -> Option<Vec<String>> {
                if path.contains(&node.to_string()) {
                    // Found cycle - extract it
                    let cycle_start = path.iter().position(|x| x == node).unwrap();
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(node.to_string()); // Close the cycle
                    return Some(cycle);
                }

                if visited.contains(node) {
                    return None;
                }

                visited.insert(node.to_string());
                path.push(node.to_string());

                if let Some(deps) = imports.get(node) {
                    for dep in deps {
                        if let Some(cycle) = find_cycle_dfs(dep, imports, path, visited) {
                            return Some(cycle);
                        }
                    }
                }

                path.pop();
                None
            }

            let cycle = find_cycle_dfs(start, &imports, &mut cycle_path, &mut cycle_visited)
                .unwrap_or_else(|| vec![start.clone(), start.clone()]);

            // Find the position of the import that closed the cycle
            let cycle_closer = cycle.get(cycle.len() - 2).unwrap_or(start);
            let cycle_target = cycle.last().unwrap();
            let position = files[cycle_closer]
                .imports()
                .iter()
                .find(|import| import.path == *cycle_target)
                .map(|import| import.position.clone())
                .unwrap_or(Position { line: 1, column: 1 });

            return Err(ResolverError::Cycle {
                cycle,
                position,
            });
        }
    }

    Ok(ResolvedGraph {
        topo_order,
        imports,
        files,
    })
}