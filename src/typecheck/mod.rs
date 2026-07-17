use crate::ast::{self, *};
use crate::types::{Type, TypeError};
use crate::lexer::Position;
use crate::qualifiers::{QualifierRegistry, QualifierContext, CodegenStatus};
use std::collections::{HashMap, HashSet};

/// Symbol table entry for a variable
#[derive(Debug, Clone)]
struct Symbol {
    ty: Type,
    position: Position, // Where it was declared
}

/// Scope for lexical scoping
#[derive(Debug)]
struct Scope {
    symbols: HashMap<String, Symbol>,
}

impl Scope {
    fn new() -> Self {
        Self {
            symbols: HashMap::new(),
        }
    }

    fn declare(&mut self, name: String, ty: Type, position: Position) {
        self.symbols.insert(name, Symbol { ty, position });
    }

    fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }
}

/// Type refinement for flow-sensitive typing
#[derive(Debug, Clone)]
struct TypeRefinement {
    variable_name: String,
    refined_type: Type,
}

/// Refinement scope for tracking type narrowing
#[derive(Debug)]
struct RefinementScope {
    refinements: HashMap<String, Type>,
}

impl RefinementScope {
    fn new() -> Self {
        Self {
            refinements: HashMap::new(),
        }
    }

    fn refine(&mut self, name: String, ty: Type) {
        self.refinements.insert(name, ty);
    }

    fn lookup(&self, name: &str) -> Option<&Type> {
        self.refinements.get(name)
    }
}

/// An exported symbol table for one module: name -> type.
/// Function exports use `Type::Function(...)`; let exports use the let's annotated type.
pub type ExportTable = HashMap<String, Type>;

/// The type checker for HiLow programs
pub struct TypeChecker {
    scopes: Vec<Scope>,
    refinement_scopes: Vec<RefinementScope>, // Stack of type refinement scopes for narrowing
    persistent_refinements: HashMap<String, Type>, // Post-block refinements that persist until scope exit
    final_refinements: HashMap<String, Type>, // Saved refinements from the final program scope for codegen
    errors: Vec<TypeError>,
    loop_depth: usize, // Track nested loop depth for break/continue validation
    switch_depth: usize, // Track nested switch depth for break validation
    qualifier_registry: QualifierRegistry,
    method_context: Option<Type>, // Track the receiver object type when inside a method
    /// Phase 10-δ-γ: Scope depth at which the currently checking function's body
    /// begins. Used by escape analysis to determine which subscribed variables
    /// are function-local (depth >= this value) vs reachable from the caller
    /// (depth < this value).
    current_function_scope_depth: Option<usize>,
    /// Phase 2b step zero: the declared return type of the function whose body
    /// is being checked. Used by check_return_statement to validate returns
    /// into optional-declared functions (the narrow return-type check; the
    /// general return-type gap remains an open question).
    current_function_return_type: Option<Type>,
    /// Phase 2b: watcher bindings (in the current function) whose initializer
    /// was a WatcherExpr capturing function-frame variables. Returning one is
    /// rejected until Phase 3 boxing makes escape sound. Maps binding name →
    /// name of the offending captured variable.
    capture_unsafe_watchers: HashMap<String, String>,
    /// Cross-module export tables, keyed by module path. Populated during pass 1 of check_graph.
    /// Empty during single-file `check`.
    module_exports: HashMap<String, ExportTable>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::new()], // Start with global scope
            refinement_scopes: Vec::new(), // Start with no refinements
            persistent_refinements: HashMap::new(), // Start with no persistent refinements
            final_refinements: HashMap::new(), // Start with no final refinements
            errors: Vec::new(),
            loop_depth: 0,
            switch_depth: 0,
            qualifier_registry: QualifierRegistry::new(),
            method_context: None,
            current_function_scope_depth: None,
            current_function_return_type: None,
            capture_unsafe_watchers: HashMap::new(),
            module_exports: HashMap::new(),
        }
    }

    /// Phase 2b step zero (audit §5 item 7): optional types whose inner has no
    /// runtime payload kind are rejected at compile time; the full payload
    /// matrix lands in Phase 3 (scalar boxing). Walks nested positions (array
    /// elements, tuple elements, function-type params/return, T??). The
    /// internal Object case is allowed (weak reads produce it; no annotation
    /// can — `object` is not a parseable type).
    fn validate_declared_type(&mut self, ty: &Type, position: &Position) {
        match ty {
            Type::Optional(inner) => {
                match inner.as_ref() {
                    Type::I32 | Type::String | Type::Time | Type::Duration
                    | Type::Money | Type::MoneyOf(_) | Type::Object(_) => {
                        // supported payload kinds
                    }
                    other => {
                        self.add_error(
                            format!(
                                "optional type '{}?' is not supported yet — the full optional \
                                 payload matrix lands in Phase 3 (scalar boxing); supported \
                                 today: i32?, string?, time?, duration?, money?",
                                other
                            ),
                            position.clone(),
                        );
                    }
                }
                self.validate_declared_type(inner, position);
            }
            Type::DynamicArray(elem) | Type::FixedArray(elem, _) => {
                self.validate_declared_type(elem, position);
            }
            Type::Tuple(elems) => {
                for e in elems {
                    self.validate_declared_type(e, position);
                }
            }
            Type::Function(params, ret) => {
                for p in params {
                    self.validate_declared_type(p, position);
                }
                self.validate_declared_type(ret, position);
            }
            _ => {}
        }
    }

    /// Type check a top-level program or module
    pub fn check(&mut self, top_level: &TopLevel) -> Result<(), Vec<TypeError>> {
        match top_level {
            TopLevel::Program(program) => {
                // Phase 11a-α: defensive guard for imports
                if !program.imports.is_empty() {
                    self.errors.push(TypeError::new(
                        "imports not yet implemented in Phase 11a-α",
                        program.imports[0].position.clone(),
                    ));
                    return self.finish_check();
                }
                self.check_program(program)
            }
            TopLevel::Module(_module) => {
                // Phase 11a-α: defensive guard for modules
                self.errors.push(TypeError::new(
                    "modules not yet implemented in Phase 11a-α",
                    _module.position.clone(),
                ));
                return self.finish_check();
            }
        }

        self.finish_check()
    }

    fn finish_check(&mut self) -> Result<(), Vec<TypeError>> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }

    /// Type check a resolved module graph with two-pass approach
    pub fn check_graph(&mut self, graph: &crate::resolver::ResolvedGraph) -> Result<(), Vec<TypeError>> {
        // Clear state for fresh invocation
        self.module_exports.clear();
        self.errors.clear();

        // Pass 1 - signature collection
        for path in &graph.topo_order {
            let parsed = graph.files.get(path).expect("ResolvedGraph should contain all paths");
            let export_table = self.collect_module_exports(path, parsed);
            self.module_exports.insert(path.clone(), export_table);
        }

        // Pass 2 - body checking
        for path in &graph.topo_order {
            let parsed = graph.files.get(path).expect("ResolvedGraph should contain all paths");
            self.check_module_bodies(path, parsed, &graph.imports);
        }

        // Finalize
        self.finish_check()
    }

    /// Returns the per-module export tables built during pass 1 of check_graph.
    /// Codegen consults this for cross-module type information.
    pub fn module_exports(&self) -> &HashMap<String, ExportTable> {
        &self.module_exports
    }

    /// Collect exports from a parsed module into an export table
    fn collect_module_exports(&mut self, path: &str, parsed: &TopLevel) -> ExportTable {
        let mut export_table = HashMap::new();

        match parsed {
            TopLevel::Program(_) => {
                // Programs don't have exports, but we may be checking a single program via this path
                // No exports to collect
            }
            TopLevel::Module(module) => {
                // Process exported functions
                for function in &module.items {
                    if function.is_export {
                        // Check if return type is explicitly annotated
                        // All functions in AST have return_type: Type, but we need to check
                        // what the parser produces for missing return types

                        // For now, assume all functions have explicit return types
                        // (will be validated in testing)

                        // Check if parameters are explicitly annotated
                        // All parameters have ty: Type, so parameter annotation rule is likely vacuous

                        let param_types: Vec<crate::types::Type> = function.params
                            .iter()
                            .map(|p| crate::types::Type::from_ast_type(&p.ty))
                            .collect();
                        let return_type = crate::types::Type::from_ast_type(&function.return_type);

                        export_table.insert(
                            function.name.clone(),
                            crate::types::Type::Function(param_types, Box::new(return_type))
                        );
                    }
                }

                // Process exported lets
                for let_decl in &module.lets {
                    if let_decl.is_export {
                        if let LetPattern::Identifier(name, type_annotation) = &let_decl.pattern {
                            if let Some(type_annotation) = type_annotation {
                                let let_type = crate::types::Type::from_ast_type(type_annotation);

                                // Phase 11b-fixup: reject cross-module function calls in export let initializers
                                if let Some(ref initializer) = let_decl.initializer {
                                    let local_function_names: std::collections::HashSet<String> = module.items
                                        .iter()
                                        .map(|f| f.name.clone())
                                        .collect();
                                    if let Some(callee_name) = self.expression_contains_cross_module_call(initializer, &local_function_names) {
                                        self.errors.push(crate::types::TypeError::new(
                                            format!("exported 'let' initializer '{}' cannot call functions from other modules (called '{}')", name, callee_name),
                                            let_decl.position.clone(),
                                        ));
                                    }
                                }

                                export_table.insert(name.clone(), let_type);
                            } else {
                                // Missing type annotation on exported let
                                self.errors.push(crate::types::TypeError::new(
                                    format!("exported 'let' declaration '{}' requires an explicit type annotation", name),
                                    let_decl.position.clone(),
                                ));
                            }
                        } else {
                            // Tuple destructuring exports not supported yet
                            self.errors.push(crate::types::TypeError::new(
                                "exported tuple destructuring not yet supported",
                                let_decl.position.clone(),
                            ));
                        }
                    }
                }
            }
        }

        export_table
    }

    /// Phase 11b-fixup: Check if an expression contains cross-module function calls
    fn expression_contains_cross_module_call(&self, expr: &Expression, local_function_names: &std::collections::HashSet<String>) -> Option<String> {
        match expr {
            Expression::Call(call) => {
                // Check the callee
                if let Expression::Ident { name, .. } = call.callee.as_ref() {
                    if !local_function_names.contains(name) {
                        return Some(name.clone());
                    }
                }
                // Recurse into the callee (for nested calls like (foo())(x))
                if let Some(name) = self.expression_contains_cross_module_call(&call.callee, local_function_names) {
                    return Some(name);
                }
                // Recurse into args
                for arg in &call.args {
                    if let Some(name) = self.expression_contains_cross_module_call(arg, local_function_names) {
                        return Some(name);
                    }
                }
                None
            }
            Expression::BinaryOp(bop) => {
                if let Some(name) = self.expression_contains_cross_module_call(&bop.lhs, local_function_names) {
                    return Some(name);
                }
                self.expression_contains_cross_module_call(&bop.rhs, local_function_names)
            }
            Expression::UnaryOp(uop) => {
                self.expression_contains_cross_module_call(&uop.operand, local_function_names)
            }
            // Other expression types don't contain calls in ways relevant to this rule.
            // Add cases here if testing reveals gaps.
            _ => None,
        }
    }

    /// Check the bodies of functions and lets in a module
    fn check_module_bodies(&mut self, path: &str, parsed: &TopLevel, all_imports: &HashMap<String, Vec<String>>) {
        // Start a fresh outermost scope for this module
        self.enter_scope();

        // Populate scope with imported names
        if let Some(imported_paths) = all_imports.get(path) {
            // For each import statement in this module, resolve the names
            match parsed {
                TopLevel::Program(program) => {
                    for import_stmt in &program.imports {
                        if imported_paths.contains(&import_stmt.path) {
                            if let Some(export_table) = self.module_exports.get(&import_stmt.path).cloned() {
                                for import_name in &import_stmt.names {
                                    if let Some(imported_type) = export_table.get(import_name) {
                                        self.declare_variable(import_name, imported_type.clone(), import_stmt.position.clone());
                                    } else {
                                        self.errors.push(crate::types::TypeError::new(
                                            format!("'{}' is not exported from '{}'", import_name, import_stmt.path),
                                            import_stmt.position.clone(),
                                        ));
                                    }
                                }
                            } else {
                                // Export table not found - shouldn't happen with valid resolver output
                                self.errors.push(crate::types::TypeError::new(
                                    format!("module '{}' not found in export tables", import_stmt.path),
                                    import_stmt.position.clone(),
                                ));
                            }
                        }
                    }
                }
                TopLevel::Module(module) => {
                    for import_stmt in &module.imports {
                        if imported_paths.contains(&import_stmt.path) {
                            if let Some(export_table) = self.module_exports.get(&import_stmt.path).cloned() {
                                for import_name in &import_stmt.names {
                                    if let Some(imported_type) = export_table.get(import_name) {
                                        self.declare_variable(import_name, imported_type.clone(), import_stmt.position.clone());
                                    } else {
                                        self.errors.push(crate::types::TypeError::new(
                                            format!("'{}' is not exported from '{}'", import_name, import_stmt.path),
                                            import_stmt.position.clone(),
                                        ));
                                    }
                                }
                            } else {
                                // Export table not found - shouldn't happen with valid resolver output
                                self.errors.push(crate::types::TypeError::new(
                                    format!("module '{}' not found in export tables", import_stmt.path),
                                    import_stmt.position.clone(),
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Add module's own declarations to scope and check their bodies
        match parsed {
            TopLevel::Program(program) => {
                // Check program body using existing helper
                if let Some(body) = &program.body {
                    // Add program's functions to scope first
                    for item in &body.items {
                        if let crate::ast::BlockItem::Function(function) = item {
                            let param_types: Vec<crate::types::Type> = function.params
                                .iter()
                                .map(|p| crate::types::Type::from_ast_type(&p.ty))
                                .collect();
                            let return_type = crate::types::Type::from_ast_type(&function.return_type);
                            let func_type = crate::types::Type::Function(param_types, Box::new(return_type));
                            self.declare_variable(&function.name, func_type, function.position.clone());
                        }
                    }

                    // Check function bodies and statements
                    for item in &body.items {
                        match item {
                            crate::ast::BlockItem::Function(function) => {
                                if function.body.is_some() {
                                    self.check_function(function);
                                }
                            }
                            crate::ast::BlockItem::Statement(statement) => {
                                self.check_statement(statement);
                            }
                            crate::ast::BlockItem::Watcher(watcher) => {
                                self.check_watcher(watcher);
                            }
                        }
                    }
                }
            }
            TopLevel::Module(module) => {
                // Add module's functions to scope first
                for function in &module.items {
                    let param_types: Vec<crate::types::Type> = function.params
                        .iter()
                        .map(|p| crate::types::Type::from_ast_type(&p.ty))
                        .collect();
                    let return_type = crate::types::Type::from_ast_type(&function.return_type);
                    let func_type = crate::types::Type::Function(param_types, Box::new(return_type));
                    self.declare_variable(&function.name, func_type, function.position.clone());
                }

                // Check function bodies
                for function in &module.items {
                    if let Some(_func_body) = &function.body {
                        self.check_function(function);
                    }
                }

                // Add module's lets to scope and check initializers
                for let_decl in &module.lets {
                    self.check_let_statement(let_decl);
                }
            }
        }

        // Exit the module's scope
        self.exit_function_scope();
    }

    fn check_program(&mut self, program: &Program) {
        // Check parameter types (they should all be valid)
        for param in &program.params {
            // Parameters must have explicit types in Phase 3
            // No need to check - they're already parsed with types
        }

        // Check return type
        // No need to check - it's already parsed

        // Check body
        if let Some(body) = &program.body {
            self.check_program_body(body);
        }
    }

    fn check_module(&mut self, module: &Module) {
        for function in &module.items {
            self.check_function(function);
        }

        // Check module-level watchers
        for watcher in &module.watchers {
            self.check_watcher(watcher);
        }
    }

    fn check_function(&mut self, function: &Function) {
        // Enter function scope
        self.enter_scope();

        // Phase 10-δ-γ: Track function scope depth for escape analysis
        let saved_depth = self.current_function_scope_depth;
        self.current_function_scope_depth = Some(self.scopes.len() - 1);

        // Phase 2b step zero: validate declared types (optional payload
        // allow-list) and track the declared return type for the narrow
        // optional-return check.
        let declared_return = Type::from_ast_type(&function.return_type);
        self.validate_declared_type(&declared_return, &function.position);
        let saved_return = self.current_function_return_type.take();
        self.current_function_return_type = Some(declared_return);
        let saved_unsafe = std::mem::take(&mut self.capture_unsafe_watchers);

        // Add parameters to scope
        for param in &function.params {
            let param_type = Type::from_ast_type(&param.ty);
            self.validate_declared_type(&param_type, &param.position);
            self.declare_variable(&param.name, param_type, param.position.clone());
        }

        // Check function body
        if let Some(body) = &function.body {
            self.check_block(body);
        }

        // Restore previous function context
        self.current_function_scope_depth = saved_depth;
        self.current_function_return_type = saved_return;
        self.capture_unsafe_watchers = saved_unsafe;

        // Exit function scope
        self.exit_function_scope();
    }

    fn check_block(&mut self, block: &Block) {
        // Enter block scope
        self.enter_scope();

        // Phase 1: register all nested function and watcher names in scope
        // (so they can refer to each other forward and not collide with let bindings)
        for item in &block.items {
            match item {
                BlockItem::Function(function) => {
                    // Add function to symbol table with full function type for call-site argument checking
                    let param_types: Vec<Type> = function.params.iter()
                        .map(|p| Type::from_ast_type(&p.ty))
                        .collect();
                    let return_type = Type::from_ast_type(&function.return_type);
                    let func_type = Type::Function(param_types, Box::new(return_type));
                    self.declare_variable(&function.name, func_type, function.position.clone());
                }
                BlockItem::Watcher(watcher) => {
                    // Register the watcher's name in scope for method calls
                    self.declare_variable(&watcher.name, Type::Watcher, watcher.position.clone());
                }
                BlockItem::Statement(_) => { /* skip */ }
            }
        }

        // Phase 2: check each item in source order
        for item in &block.items {
            match item {
                BlockItem::Statement(s) => self.check_statement(s),
                BlockItem::Function(f) => self.check_function(f),
                BlockItem::Watcher(w) => self.check_watcher(w),
            }
        }

        // Exit block scope
        self.exit_scope();
    }

    fn check_watcher(&mut self, watcher: &Watcher) {
        // Phase 10-γ-fixup: Register the watcher's name in the outer scope for method calls
        self.declare_variable(&watcher.name, Type::Watcher, watcher.position.clone());

        self.enter_scope();

        // Check each subscription, registering body bindings as we go
        for sub in watcher.subscriptions.iter() {
            self.check_subscription_and_bind(sub, &watcher.position);
        }

        // Check the body
        self.check_block(&watcher.body);

        // Validate that the body does not contain `return value;` — only bare `return;` is allowed
        self.check_no_return_with_value(&watcher.body, watcher.position.clone());

        self.exit_function_scope();
    }

    fn check_subscription_and_bind(&mut self, sub: &Subscription, watcher_position: &Position) {
        // 1. Look up the outer variable's type by name
        let outer_type = match self.lookup_variable_type(&sub.variable_name) {
            Some(t) => t,
            None => {
                self.add_error(
                    format!("subscribed variable '{}' is not in scope", sub.variable_name),
                    sub.position.clone()
                );
                return;
            }
        };

        // 2. Reject subscriptions to callable bindings (functions, watchers)
        if matches!(outer_type, Type::Function(_, _) | Type::Watcher) {
            self.add_error(
                format!("cannot subscribe to '{}' because it is a {} (subscriptions are for data values, not callables)",
                    sub.variable_name,
                    match outer_type { Type::Function(_, _) => "function", Type::Watcher => "watcher", _ => "?" }),
                sub.position.clone()
            );
            return;
        }

        // 3. Validate modifier-type compatibility (per the rules above)
        let alias_type = self.validate_subscription_modifier(&sub.modifier, &outer_type, &sub.position);

        // 3.1. Phase 10-ε-β/γ: Validate alias usage - only allowed with added/removed/moved modifiers
        if sub.alias.is_some() && !matches!(sub.modifier, SubscriptionModifier::Added | SubscriptionModifier::Removed | SubscriptionModifier::Moved) {
            self.add_error(
                format!("alias binding is only supported with added/removed/moved modifiers, got {:?}", sub.modifier),
                sub.position.clone()
            );
            return;
        }

        // 3.2. Store resolved types in the subscription for codegen access
        sub.resolved_var_type.borrow_mut().replace(outer_type.to_ast_type());
        if let Some(ref at) = alias_type {
            sub.resolved_alias_type.borrow_mut().replace(at.to_ast_type());
        }

        // 4. Register the body-scope binding for the variable name (always the outer type)
        self.declare_variable(&sub.variable_name, outer_type.clone(), sub.position.clone());

        // 5. If alias present, register it too with the alias type
        if let Some(ref alias_name) = sub.alias {
            if let Some(at) = alias_type {
                self.declare_variable(alias_name, at, sub.position.clone());
            }
        }
    }

    fn validate_subscription_modifier(
        &mut self,
        modifier: &SubscriptionModifier,
        outer_type: &Type,
        position: &Position,
    ) -> Option<Type> {
        use SubscriptionModifier::*;
        match modifier {
            Changed | Assigned => {
                // Compatible with any type; no alias-specific type
                Some(outer_type.clone())
            }
            Added | Removed => {
                // Phase 10-ε-β: Require collection type, alias gets element type
                if let Some(element_type) = self.collection_element_type(outer_type) {
                    Some(element_type)  // alias binds to the element, not the array
                } else {
                    self.add_error(
                        format!("({:?}) modifier requires a collection type, got '{}'",
                            modifier, self.type_name(outer_type)),
                        position.clone()
                    );
                    None
                }
            }
            Moved => {
                // Phase 10-ε-γ: Require ordered collection (array), alias gets (from,to) tuple
                if matches!(outer_type, Type::DynamicArray(_) | Type::FixedArray(_, _)) {
                    Some(Type::Tuple(vec![Type::Usize, Type::Usize]))  // alias binds to (from, to) indices
                } else {
                    self.add_error(
                        format!("(moved) modifier requires an ordered collection type (array), got '{}'",
                            self.type_name(outer_type)),
                        position.clone()
                    );
                    None
                }
            }
            Deep => {
                // Phase 2d: arrays only until other values gain the cell header.
                // The parameter binds the subscribed variable's current full value.
                if matches!(outer_type, Type::DynamicArray(_)) {
                    Some(outer_type.clone())
                } else {
                    self.add_error(
                        format!("(deep) modifier requires an array type in this phase, got '{}' — deep watching of scalars lands with Phase 3 (boxing); objects are unscheduled (see STATUS.md)",
                            self.type_name(outer_type)),
                        position.clone()
                    );
                    None
                }
            }
        }
    }

    fn check_no_return_with_value(&mut self, block: &Block, error_position: Position) {
        for statement in block.statements_iter() {
            self.check_no_return_with_value_statement(statement, &error_position);
        }
    }

    fn check_no_return_with_value_statement(&mut self, statement: &Statement, error_position: &Position) {
        match statement {
            Statement::Return(return_stmt) => {
                if return_stmt.value.is_some() {
                    self.add_error(
                        "watcher body cannot return a value (use bare 'return;' for early exit)".to_string(),
                        return_stmt.position.clone()
                    );
                }
            }
            Statement::If(if_stmt) => {
                self.check_no_return_with_value(&if_stmt.then_block, error_position.clone());
                if let Some(ref else_block) = if_stmt.else_block {
                    self.check_no_return_with_value(else_block, error_position.clone());
                }
            }
            Statement::While(while_stmt) => {
                self.check_no_return_with_value(&while_stmt.body, error_position.clone());
            }
            Statement::Loop(loop_stmt) => {
                self.check_no_return_with_value(&loop_stmt.body, error_position.clone());
            }
            Statement::ForIn(for_stmt) => {
                self.check_no_return_with_value(&for_stmt.body, error_position.clone());
            }
            // Other statement types don't contain nested blocks with returns
            _ => {}
        }
    }

    fn lookup_variable_type(&self, name: &str) -> Option<Type> {
        // First, check for type refinements from innermost to outermost
        for refinement_scope in self.refinement_scopes.iter().rev() {
            if let Some(refined_type) = refinement_scope.lookup(name) {
                return Some(refined_type.clone());
            }
        }

        // Second, check for persistent refinements
        if let Some(refined_type) = self.persistent_refinements.get(name) {
            return Some(refined_type.clone());
        }

        // Then search regular scopes from innermost to outermost
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.lookup(name) {
                return Some(symbol.ty.clone());
            }
        }

        None
    }

    /// Phase 10-δ-γ: Look up a variable and return both its type and the
    /// scope index (depth) at which it was found. Used by escape analysis
    /// to determine reachability of subscribed variables.
    fn lookup_variable_with_depth(&self, name: &str) -> Option<(Type, usize)> {
        // First, check for type refinements from innermost to outermost
        for (depth, refinement_scope) in self.refinement_scopes.iter().enumerate().rev() {
            if let Some(refined_type) = refinement_scope.lookup(name) {
                // For refined types, use the regular scope depth where the variable was declared
                // Find it in regular scopes to get the proper depth
                for (scope_depth, scope) in self.scopes.iter().enumerate().rev() {
                    if scope.lookup(name).is_some() {
                        return Some((refined_type.clone(), scope_depth));
                    }
                }
            }
        }

        // Second, check for persistent refinements
        if let Some(refined_type) = self.persistent_refinements.get(name) {
            // For persistent refinements, find the variable in regular scopes to get depth
            for (scope_depth, scope) in self.scopes.iter().enumerate().rev() {
                if scope.lookup(name).is_some() {
                    return Some((refined_type.clone(), scope_depth));
                }
            }
        }

        // Then search regular scopes from innermost to outermost
        for (depth, scope) in self.scopes.iter().enumerate().rev() {
            if let Some(symbol) = scope.lookup(name) {
                return Some((symbol.ty.clone(), depth));
            }
        }

        None
    }

    fn is_primitive_type(&self, ty: &Type) -> bool {
        matches!(ty,
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 |
            Type::F32 | Type::F64 | Type::Usize | Type::Isize |
            Type::Bool | Type::String | Type::Nothing | Type::UnknownType |
            Type::Time | Type::Duration | Type::Money | Type::MoneyOf(_)
        )
    }

    fn collection_element_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::DynamicArray(element_type) => Some((**element_type).clone()),
            Type::FixedArray(element_type, _) => Some((**element_type).clone()),
            _ => None
        }
    }

    fn type_name(&self, ty: &Type) -> String {
        format!("{}", ty)
    }

    fn check_program_body(&mut self, body: &ProgramBody) {
        // Enter program body scope
        self.enter_scope();

        // Phase 1: Declare all nested functions in the current scope first
        for item in &body.items {
            if let BlockItem::Function(function) = item {
                // For Phase 6a-fixup, nested functions can't access enclosing variables
                // They're treated as top-level functions that happen to be declared nested
                // Add function to symbol table with full function type for call-site argument checking
                let param_types: Vec<Type> = function.params.iter()
                    .map(|p| Type::from_ast_type(&p.ty))
                    .collect();
                let return_type = Type::from_ast_type(&function.return_type);
                let func_type = Type::Function(param_types, Box::new(return_type));
                self.declare_variable(&function.name, func_type, function.position.clone());
            }
        }

        // Phase 2: Check function bodies and statements
        for item in &body.items {
            match item {
                BlockItem::Statement(statement) => self.check_statement(statement),
                BlockItem::Function(function) => self.check_function(function),
                BlockItem::Watcher(watcher) => {
                    self.check_watcher(watcher);
                }
            }
        }

        // Save persistent refinements before exiting program body scope (for codegen)
        self.final_refinements = self.persistent_refinements.clone();

        // Exit program body scope
        self.exit_function_scope();
    }

    fn check_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Let(let_decl) => self.check_let_statement(let_decl),
            Statement::Return(return_stmt) => self.check_return_statement(return_stmt),
            Statement::If(if_stmt) => self.check_if_statement(if_stmt),
            Statement::While(while_stmt) => self.check_while_statement(while_stmt),
            Statement::Loop(loop_stmt) => self.check_loop_statement(loop_stmt),
            Statement::ForIn(for_in_stmt) => self.check_for_in_statement(for_in_stmt),
            Statement::Switch(switch_stmt) => self.check_switch_statement(switch_stmt),
            Statement::Break(pos) => {
                if self.loop_depth == 0 && self.switch_depth == 0 {
                    self.add_error(
                        "break is only valid inside a loop or switch".to_string(),
                        pos.clone()
                    );
                }
            },
            Statement::Continue(pos) => {
                if self.loop_depth == 0 {
                    self.add_error(
                        "continue is only valid inside a loop".to_string(),
                        pos.clone()
                    );
                }
            },
            Statement::Assign(assign_stmt) => self.check_assign_statement(assign_stmt),
            Statement::QualifiedOp(qualified_op) => self.check_qualified_op(qualified_op),
            Statement::StealthBlock(block, _) => self.check_block(block),
            Statement::ExprStatement(expr) => {
                self.check_expression(expr);
            }
        }
    }

    fn check_let_statement(&mut self, let_decl: &LetDecl) {
        match &let_decl.pattern {
            LetPattern::Identifier(name, declared_type_ast) => {
                let declared_type = declared_type_ast.as_ref().map(|ty| Type::from_ast_type(ty));
                // Phase 2b step zero: optional payload allow-list on annotations
                if let Some(ref declared) = declared_type {
                    self.validate_declared_type(declared, &let_decl.position);
                }
                let initializer_type = let_decl.initializer.as_ref().map(|expr| {
                    // Special handling for literals when there's a declared type
                    if let Some(ref declared) = declared_type {
                        self.check_expression_with_expected_type(expr, declared)
                    } else {
                        self.check_expression(expr)
                    }
                });

                // Phase 2b: a watcher binding that captures function-frame
                // variables (locals or params — both stack-resident, captured
                // by address) must not escape its declaring function. Record
                // it so check_return_statement can reject `return <name>`.
                // Sound escape lands in Phase 3 (boxing).
                self.capture_unsafe_watchers.remove(name); // rebinding clears
                if let (Some(Expression::WatcherExpr(watcher_expr)), Some(function_depth)) =
                    (let_decl.initializer.as_ref(), self.current_function_scope_depth)
                {
                    let captures = watcher_expr.captures.borrow();
                    for (cap_name, _ty, _pos) in captures.iter() {
                        if let Some((_, var_depth)) = self.lookup_variable_with_depth(cap_name) {
                            if var_depth >= function_depth {
                                self.capture_unsafe_watchers
                                    .insert(name.clone(), cap_name.clone());
                                break;
                            }
                        }
                    }
                }

                let final_type = match (declared_type, initializer_type) {
                    (Some(declared), Some(inferred)) => {
                        // Check that initializer matches declared type
                        // Special handling for money types: money<USD> is assignable to money
                        let types_compatible = if declared == inferred {
                            true
                        } else if matches!(&declared, Type::Money) && matches!(&inferred, Type::MoneyOf(_)) {
                            true // money<X> can be assigned to money
                        } else {
                            false
                        };

                        if !types_compatible {
                            self.add_error(
                                format!("Type mismatch: declared {} but initializer has type {}",
                                        declared, inferred),
                                let_decl.position.clone()
                            );
                            declared // Use declared type for symbol table
                        } else {
                            // For compatible money assignments, preserve the specific currency
                            if matches!(&declared, Type::Money) && matches!(&inferred, Type::MoneyOf(_)) {
                                inferred // Use specific currency type (money<USD>) instead of generic money
                            } else {
                                declared
                            }
                        }
                    },
                    (Some(declared), None) => {
                        // Just a type declaration, no initializer
                        declared
                    },
                    (None, Some(inferred)) => {
                        // Type inference from initializer
                        inferred
                    },
                    (None, None) => {
                        // Uninitialized let binding has type nothing
                        Type::Nothing
                    }
                };

                // Add to symbol table
                self.declare_variable(name, final_type, let_decl.position.clone());
            },
            LetPattern::Tuple(names) => {
                // Tuple destructuring: let (a, b, c) = tuple_expr
                if let Some(initializer) = &let_decl.initializer {
                    let initializer_type = self.check_expression(initializer);

                    match &initializer_type {
                        Type::Tuple(element_types) => {
                            if element_types.len() != names.len() {
                                self.add_error(
                                    format!("Tuple destructuring arity mismatch: expected {} elements, got {}",
                                            names.len(), element_types.len()),
                                    let_decl.position.clone()
                                );
                                // Declare variables with unknown types as fallback
                                for name in names {
                                    self.declare_variable(name, Type::Unknown, let_decl.position.clone());
                                }
                            } else {
                                // Declare each variable with its corresponding tuple element type
                                for (name, element_type) in names.iter().zip(element_types.iter()) {
                                    self.declare_variable(name, element_type.clone(), let_decl.position.clone());
                                }
                            }
                        },
                        _ => {
                            self.add_error(
                                format!("Cannot destructure non-tuple type {} in tuple pattern",
                                        initializer_type),
                                let_decl.position.clone()
                            );
                            // Declare variables with unknown types as fallback
                            for name in names {
                                self.declare_variable(name, Type::Unknown, let_decl.position.clone());
                            }
                        }
                    }
                } else {
                    self.add_error(
                        "Tuple destructuring requires an initializer".to_string(),
                        let_decl.position.clone()
                    );
                    // Declare variables with unknown types as fallback
                    for name in names {
                        self.declare_variable(name, Type::Unknown, let_decl.position.clone());
                    }
                }
            }
        }
    }

    fn check_return_statement(&mut self, return_stmt: &ReturnStmt) {
        if let Some(value) = &return_stmt.value {
            // Phase 2b: reject returning a watcher binding that captures
            // function-frame variables (its env would hold addresses into the
            // dead frame). Sound escape lands in Phase 3 (boxing).
            if let Expression::Ident { name, .. } = value {
                if let Some(cap_name) = self.capture_unsafe_watchers.get(name) {
                    self.add_error(
                        format!(
                            "watcher '{}' captures '{}', which lives in this function's \
                             frame — a watcher capturing function-local variables cannot \
                             escape its declaring function until Phase 3 (scalar boxing)",
                            name, cap_name
                        ),
                        return_stmt.position.clone(),
                    );
                }
            }

            let value_type = self.check_expression(value);

            // Phase 10-δ-γ: Validate reachability of subscribed variables for
            // escaping watcher expressions. Runs AFTER check_expression so the
            // watcher's captures (populated during checking) are visible to
            // the Phase 2b capture-escape rejection.
            if let Expression::WatcherExpr(watcher_expr) = value {
                self.check_watcher_escape_reachability(watcher_expr, &return_stmt.position);
            }

            // Phase 2b step zero: narrow return-type check for functions
            // declared to return T? — the returned value must be the inner
            // type, the optional itself, or unknown. (The general
            // return-type check remains an open question.)
            if let Some(Type::Optional(inner)) = self.current_function_return_type.clone() {
                let compatible = value_type == *inner
                    || value_type == Type::Optional(inner.clone())
                    || matches!(value_type, Type::UnknownType)
                    || matches!(value_type, Type::Unknown); // error recovery
                if !compatible {
                    self.add_error(
                        format!(
                            "cannot return {} from a function declared to return {}?",
                            value_type, inner
                        ),
                        return_stmt.position.clone(),
                    );
                }
            }
        }
        // TODO (unchanged general gap): check non-optional return types too
    }

    fn check_watcher_escape_reachability(&mut self, watcher_expr: &WatcherExpr, return_position: &Position) {
        let function_depth = match self.current_function_scope_depth {
            Some(d) => d,
            None => {
                // Not inside a function — shouldn't happen for a return statement, but
                // belt-and-suspenders.
                self.errors.push(TypeError::new(
                    "watcher expression in return statement outside function context",
                    return_position.clone()
                ));
                return;
            }
        };

        for sub in &watcher_expr.subscriptions {
            match self.lookup_variable_with_depth(&sub.variable_name) {
                None => {
                    self.errors.push(TypeError::new(
                        &format!("subscribed variable '{}' is not in scope", sub.variable_name),
                        sub.position.clone()
                    ));
                }
                Some((_, var_depth)) => {
                    if var_depth >= function_depth {
                        // Variable is at function-local depth or deeper — not reachable from caller
                        self.errors.push(TypeError::new(
                            &format!(
                                "subscribed variable '{}' is not reachable from the function's caller; \
                                 the watcher cannot escape this scope because '{}' is declared inside the function. \
                                 Move the variable declaration to an enclosing scope to allow the watcher to escape.",
                                sub.variable_name, sub.variable_name
                            ),
                            sub.position.clone()
                        ));
                    }
                }
            }
        }

        // Phase 2b: captured variables are stored in the env by address —
        // function-frame captures (locals AND params) dangle if the watcher
        // escapes. Reject until Phase 3 boxing makes escape sound.
        let captures = watcher_expr.captures.borrow();
        for (cap_name, _ty, _pos) in captures.iter() {
            if let Some((_, var_depth)) = self.lookup_variable_with_depth(cap_name) {
                if var_depth >= function_depth {
                    self.errors.push(TypeError::new(
                        &format!(
                            "watcher captures '{}', which lives in this function's frame — \
                             a watcher capturing function-local variables cannot escape its \
                             declaring function until Phase 3 (scalar boxing)",
                            cap_name
                        ),
                        return_position.clone()
                    ));
                }
            }
        }
    }

    fn check_if_statement(&mut self, if_stmt: &IfStmt) {
        // Phase 4b: Check condition - supports truthy/falsy (bool, integers, floats)
        let condition_type = self.check_expression(&if_stmt.condition);
        if !self.is_condition_type(&condition_type) {
            self.add_error(
                format!("If condition must be bool, integer, or float type, found {}", condition_type),
                if_stmt.condition.position()
            );
        }

        // Phase 9b: Check for type narrowing opportunities
        let is_unknown_check = self.extract_is_unknown_check(&if_stmt.condition);

        match is_unknown_check {
            Some((var_name, original_type)) => {
                // This is a "variable is unknown" check - apply scope-local narrowing

                // Check the then block with refinement: variable is narrowed to UnknownType
                self.enter_refinement_scope();
                self.refine_variable_type(&var_name, Type::UnknownType);
                self.check_block(&if_stmt.then_block);
                let then_block_exits = self.block_always_exits(&if_stmt.then_block);
                self.exit_refinement_scope();

                // Check else block if present with refinement: variable is narrowed to underlying type
                if let Some(else_block) = &if_stmt.else_block {
                    self.enter_refinement_scope();
                    // Extract the underlying type from Optional(T) -> T
                    if let Type::Optional(underlying_type) = original_type.clone() {
                        self.refine_variable_type(&var_name, *underlying_type);
                    }
                    self.check_block(else_block);
                    self.exit_refinement_scope();
                }

                // Post-block narrowing: if then block always exits, variable is narrowed to underlying type
                if then_block_exits {
                    if let Type::Optional(underlying_type) = original_type {
                        self.add_persistent_refinement(&var_name, *underlying_type);
                    }
                }
            }
            None => {
                // No type narrowing - check blocks normally
                self.check_block(&if_stmt.then_block);

                if let Some(else_block) = &if_stmt.else_block {
                    self.check_block(else_block);
                }
            }
        }
    }

    fn check_while_statement(&mut self, while_stmt: &WhileStmt) {
        // Phase 4b: Check condition - supports truthy/falsy (bool, integers, floats)
        let condition_type = self.check_expression(&while_stmt.condition);
        if !self.is_condition_type(&condition_type) {
            self.add_error(
                format!("While condition must be bool, integer, or float type, found {}", condition_type),
                while_stmt.condition.position()
            );
        }

        // Enter loop scope for break/continue validation
        self.loop_depth += 1;

        // Check body
        self.check_block(&while_stmt.body);

        // Exit loop scope
        self.loop_depth -= 1;
    }

    fn check_loop_statement(&mut self, loop_stmt: &LoopStmt) {
        // Enter loop scope for break/continue validation
        self.loop_depth += 1;

        // Check body
        self.check_block(&loop_stmt.body);

        // Exit loop scope
        self.loop_depth -= 1;
    }

    fn check_for_in_statement(&mut self, for_in_stmt: &ForInStmt) {
        // Check that the iterable expression is an object or array type
        let iterable_type = self.check_expression(&for_in_stmt.iterable);

        // Enter loop scope for break/continue validation
        self.loop_depth += 1;

        // Enter a new scope for the loop variables
        self.enter_scope();

        // Declare the loop variables in the new scope based on iterable type
        match iterable_type {
            Type::Object(_) => {
                // Object iteration: key is string, value is ObjectIterValue
                self.declare_variable(
                    &for_in_stmt.key_name,
                    Type::String,
                    for_in_stmt.position.clone()
                );
                self.declare_variable(
                    &for_in_stmt.value_name,
                    Type::ObjectIterValue, // Special type for iteration values
                    for_in_stmt.position.clone()
                );
            }
            Type::DynamicArray(elem_type) => {
                // Array iteration: key is usize index, value is element type
                self.declare_variable(
                    &for_in_stmt.key_name,
                    Type::Usize,
                    for_in_stmt.position.clone()
                );
                self.declare_variable(
                    &for_in_stmt.value_name,
                    *elem_type,
                    for_in_stmt.position.clone()
                );
            }
            _ => {
                self.add_error(
                    format!("for-in requires an object or array; got {}", iterable_type),
                    for_in_stmt.position.clone()
                );
                // Exit scopes before returning
                self.exit_scope();
                self.loop_depth -= 1;
                return;
            }
        }

        // Check the loop body
        self.check_block(&for_in_stmt.body);

        // Exit the loop scope
        self.exit_scope();
        self.loop_depth -= 1;
    }

    fn check_assign_statement(&mut self, assign_stmt: &AssignStmt) {
        // Special handling for index assignment (arr[i] = x) - Array Phase B
        if let Expression::IndexAccess(index_access) = &assign_stmt.target {
            let array_type = self.check_expression(&index_access.object);
            let index_type = self.check_expression(&index_access.index);
            let value_type = self.check_expression(&assign_stmt.value);

            // Verify array is actually an array type
            if let Type::DynamicArray(elem_type) = array_type {
                // Verify index is integer type
                if !matches!(index_type, Type::I32 | Type::I64 | Type::U32 | Type::U64 | Type::Usize) {
                    self.add_error(
                        format!("Array index must be integer type, got {}", index_type),
                        index_access.position.clone()
                    );
                }

                // Verify assigned value matches element type
                if *elem_type != value_type {
                    self.add_error(
                        format!("Cannot assign {} to array element of type {}", value_type, elem_type),
                        assign_stmt.position.clone()
                    );
                }
            } else {
                self.add_error(
                    format!("Cannot index assign to non-array type {}", array_type),
                    assign_stmt.position.clone()
                );
            }
            return;
        }

        // Regular assignment (non-index)
        let target_type = self.check_expression(&assign_stmt.target);
        let value_type = self.check_expression(&assign_stmt.value);

        // For assignment, types must match exactly (no coercion).
        // Phase 1.5e exception: storing `weak x` into a weak slot — the slot
        // reads back as T? but the store takes the referent itself.
        let weak_store_ok = matches!(assign_stmt.value, Expression::WeakRef(_, _))
            && matches!(value_type, Type::Object(_))
            && target_type == Type::Optional(Box::new(value_type.clone()));
        if target_type != value_type && !weak_store_ok {
            self.add_error(
                format!("Cannot assign {} to {}", value_type, target_type),
                assign_stmt.position.clone()
            );
        }

        // TODO: Check that target is assignable (not a constant, etc.)
    }

    fn check_switch_statement(&mut self, switch_stmt: &SwitchStmt) {
        // Check the switch expression
        let switch_expr_type = self.check_expression(&switch_stmt.value);

        // Enter switch context for break validation
        self.switch_depth += 1;

        // Check each case
        for case in &switch_stmt.cases {
            // Check that case pattern type matches switch expression type
            let pattern_type = self.literal_type(&case.pattern);
            if pattern_type != switch_expr_type {
                self.add_error(
                    format!(
                        "Case pattern type {} does not match switch expression type {}",
                        pattern_type, switch_expr_type
                    ),
                    case.position.clone()
                );
            }

            // Check case body statements
            for statement in &case.body {
                self.check_statement(statement);
            }
        }

        // Check default case if present
        if let Some(default_statements) = &switch_stmt.default {
            for statement in default_statements {
                self.check_statement(statement);
            }
        }

        // Exit switch context
        self.switch_depth -= 1;
    }

    fn literal_type(&self, literal: &Literal) -> Type {
        match literal {
            Literal::Integer(_) => Type::I32,
            Literal::Float(_) => Type::F64,
            Literal::String(_) => Type::String,
            Literal::Bool(_) => Type::Bool,
        }
    }

    fn check_expression_with_expected_type(&mut self, expression: &Expression, expected: &Type) -> Type {
        match expression {
            Expression::IntLit(value, _) => {
                // Check if the literal fits in the expected type
                if expected.can_fit_integer(*value) {
                    expected.clone()
                } else {
                    // Fall back to default inference
                    Type::default_integer_type(*value)
                }
            },
            Expression::FloatLit(_, _) => {
                // Float literals can match f32 or f64
                if *expected == Type::F32 || *expected == Type::F64 {
                    expected.clone()
                } else {
                    Type::default_float_type()
                }
            },
            Expression::DurationLit(_, _, _) => {
                // Duration literals always have type duration
                Type::Duration
            },
            Expression::ArrayLit(elements, pos) => {
                // Special handling for empty arrays with expected type
                if elements.is_empty() {
                    match expected {
                        Type::DynamicArray(_) => {
                            expected.clone()
                        },
                        _ => {
                            self.add_error(
                                format!("empty array literal expected to have array type, but expected type is {}", expected),
                                pos.clone()
                            );
                            Type::Unknown
                        }
                    }
                } else {
                    // Non-empty arrays: use regular type checking
                    self.check_expression(expression)
                }
            },
            _ => {
                // For non-literals, use regular type checking
                self.check_expression(expression)
            }
        }
    }

    fn check_expression(&mut self, expression: &Expression) -> Type {
        match expression {
            Expression::IntLit(value, _) => {
                // Apply integer literal inference rules
                Type::default_integer_type(*value)
            },
            Expression::FloatLit(_, _) => {
                // Apply float literal inference rules
                Type::default_float_type()
            },
            Expression::StringLit(_, _) => Type::String,
            Expression::FString(fstring) => self.check_fstring(fstring),
            Expression::BoolLit(_, _) => Type::Bool,
            Expression::DurationLit(_, _, _) => Type::Duration,
            Expression::MoneyLit(_, currency, _) => Type::MoneyOf(currency.clone()),
            Expression::Nothing(_) => Type::Nothing,
            Expression::Ident { name, position, .. } => {
                // Special handling for builtin identifiers
                if name == "time" {
                    // time is a builtin namespace with methods
                    return Type::Object(vec![]); // Empty object type for namespace access
                }

                // Look up variable in symbol table with refinement support
                let declared_type = self.lookup_variable(name, position.clone());

                // Check for refinements (scope-local or persistent)
                let refined = self.get_variable_refinement(name);

                // Return the refined type for type checking
                refined.unwrap_or(declared_type)
            },
            Expression::This(pos) => {
                // this is only valid inside methods
                if let Some(receiver_type) = &self.method_context {
                    receiver_type.clone()
                } else {
                    self.add_error(
                        "this is only valid inside methods".to_string(),
                        pos.clone()
                    );
                    Type::Unknown
                }
            },
            Expression::BinaryOp(binary_op) => self.check_binary_op(binary_op),
            Expression::UnaryOp(unary_op) => self.check_unary_op(unary_op),
            Expression::Call(call) => self.check_call(call),
            Expression::MemberAccess(member_access) => self.check_member_access(member_access),
            Expression::IndexAccess(index_access) => {
                let object_type = self.check_expression(&index_access.object);
                let index_type = self.check_expression(&index_access.index);

                // Validate index is an integer type
                if !matches!(index_type, Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
                                       Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::Usize) {
                    self.add_error(
                        format!("array index must be an integer type, found {}", index_type),
                        index_access.index.position()
                    );
                    return Type::Unknown;
                }

                // Check if object is an array or string
                match object_type {
                    Type::DynamicArray(elem_type) => *elem_type,
                    Type::String => Type::U8, // String indexing returns u8 bytes
                    _ => {
                        self.add_error(
                            format!("index access is only supported on arrays and strings, found {}", object_type),
                            index_access.object.position()
                        );
                        Type::Unknown
                    }
                }
            },
            Expression::IsCheck(is_check) => {
                // is/is not always returns bool
                self.check_expression(&is_check.expression); // Type check the expression
                // Note: The type in is_check.ty is already validated during parsing
                Type::Bool
            },
            Expression::ObjectIsCheck(obj_is_check) => {
                // obj is obj always returns bool
                // Both sides should be objects, but we'll allow any type for flexibility
                self.check_expression(&obj_is_check.lhs);
                self.check_expression(&obj_is_check.rhs);
                Type::Bool
            },
            Expression::QualifiedOp(qualified_op) => self.check_qualified_op_expression(qualified_op),
            Expression::ObjectLiteral(obj_lit) => self.check_object_literal(obj_lit),
            Expression::FunctionExpr(func_expr) => self.check_function_expression(func_expr),
            Expression::Match(match_expr) => self.check_match_expression(match_expr),
            Expression::WeakRef(expr, pos) => {
                // Check that the expression has Object type
                let expr_type = self.check_expression(expr);
                if !matches!(expr_type, Type::Object(_)) {
                    self.add_error(
                        format!("weak can only be applied to object types, found {}", expr_type),
                        pos.clone()
                    );
                    Type::Unknown
                } else {
                    // Return the same type as the expression (Object type)
                    // The weak flag will be tracked at codegen time
                    expr_type
                }
            },
            Expression::Unknown(unknown_construction) => self.check_unknown_construction(unknown_construction),
            Expression::TupleLit(elements, _) => {
                // Type check each element and create tuple type
                let element_types: Vec<Type> = elements.iter()
                    .map(|element| self.check_expression(element))
                    .collect();
                Type::Tuple(element_types)
            },
            Expression::TupleAccess(tuple_expr, index, pos) => {
                // Type check the tuple expression
                let tuple_type = self.check_expression(tuple_expr);

                match &tuple_type {
                    Type::Tuple(element_types) => {
                        // Check bounds
                        if *index >= element_types.len() {
                            self.add_error(
                                format!("Tuple index {} is out of bounds (tuple has {} elements)",
                                        index, element_types.len()),
                                pos.clone()
                            );
                            Type::Unknown
                        } else {
                            element_types[*index].clone()
                        }
                    },
                    _ => {
                        self.add_error(
                            format!("Cannot index non-tuple type {} with .{}",
                                    tuple_type, index),
                            pos.clone()
                        );
                        Type::Unknown
                    }
                }
            },
            Expression::ArrayLit(elements, pos) => {
                if elements.is_empty() {
                    // Empty arrays without type ascription are not supported
                    // If this is reached, it means we have a bare [] without type ascription
                    self.add_error(
                        "empty array literals require type ascription (use []: [ElementType]) or binding annotation (let x: [ElementType] = [])".to_string(),
                        pos.clone()
                    );
                    return Type::Unknown;
                }

                // Infer element type from the first element
                let first_elem_type = self.check_expression(&elements[0]);

                // Verify all elements have the same type
                for (i, element) in elements.iter().enumerate().skip(1) {
                    let elem_type = self.check_expression(element);
                    if elem_type != first_elem_type {
                        self.add_error(
                            format!("array elements must all have the same type; found {} and {}",
                                    first_elem_type, elem_type),
                            element.position()
                        );
                        return Type::Unknown;
                    }
                }

                Type::DynamicArray(Box::new(first_elem_type))
            },
            Expression::TypeAscription(inner, ascribed_ty, pos) => {
                // Type ascription: expr : Type

                // Convert AST type to internal type for comparison
                let ascribed_internal_type = Type::from_ast_type(ascribed_ty);

                // Phase 2b step zero: optional payload allow-list
                self.validate_declared_type(&ascribed_internal_type, pos);

                // Special case: empty array literal with ascription provides the element type
                if let Expression::ArrayLit(elements, _) = inner.as_ref() {
                    if elements.is_empty() {
                        // For empty arrays, the ascription must be a DynamicArray type
                        if let crate::ast::Type::DynamicArray(_) = ascribed_ty {
                            return ascribed_internal_type;
                        } else {
                            self.add_error(
                                format!("empty array literal with type ascription must have array type, found {:?}", ascribed_ty),
                                pos.clone()
                            );
                            return Type::Unknown;
                        }
                    }
                }

                // For all other cases, check the inner expression normally
                let inner_type = self.check_expression(inner);

                // For numeric literals, allow ascription to compatible numeric types
                if let Expression::IntLit(_, _) = inner.as_ref() {
                    if ascribed_internal_type.is_numeric() {
                        return ascribed_internal_type;
                    }
                }

                // For other cases, the types must be compatible
                if inner_type == ascribed_internal_type {
                    // Redundant/matching ascription - allow it
                    ascribed_internal_type
                } else if inner_type == Type::Unknown {
                    // If inner type is unknown (due to error), return ascribed type to avoid cascading errors
                    ascribed_internal_type
                } else {
                    // Type mismatch - error
                    self.add_error(
                        format!("cannot ascribe type {:?} to expression of type {}; type ascription does not perform conversion",
                                ascribed_ty, inner_type),
                        pos.clone()
                    );
                    ascribed_internal_type
                }
            },
            Expression::WatcherExpr(watcher_expr) => {
                self.check_watcher_expression(watcher_expr)
            },
        }
    }

    fn check_binary_op(&mut self, binary_op: &BinaryOp) -> Type {
        let lhs_type = self.check_expression(&binary_op.lhs);
        let rhs_type = self.check_expression(&binary_op.rhs);

        match binary_op.op {
            // Arithmetic operators: handle numeric types and time/duration
            BinaryOpKind::Add => {
                // Special cases for time and duration
                match (&lhs_type, &rhs_type) {
                    (Type::Time, Type::Duration) => Type::Time,        // time + duration → time
                    (Type::Duration, Type::Time) => Type::Time,        // duration + time → time
                    (Type::Duration, Type::Duration) => Type::Duration, // duration + duration → duration
                    (Type::Time, Type::Time) => {
                        self.add_error(
                            "Cannot add time to time; use time - time to get duration".to_string(),
                            binary_op.position.clone()
                        );
                        Type::Unknown
                    }
                    // Money addition: same currency required
                    (Type::Money, Type::Money) => Type::Money, // money + money → money (generic)
                    (Type::MoneyOf(c1), Type::MoneyOf(c2)) => {
                        if c1 == c2 {
                            Type::MoneyOf(c1.clone()) // same currency → result is that currency
                        } else {
                            self.add_error(
                                format!("Cannot mix money<{}> and money<{}> in arithmetic; explicit conversion required", c1, c2),
                                binary_op.position.clone()
                            );
                            Type::Unknown
                        }
                    }
                    (Type::Money, Type::MoneyOf(currency)) | (Type::MoneyOf(currency), Type::Money) => {
                        Type::MoneyOf(currency.clone()) // specific currency takes precedence
                    }
                    // String concatenation
                    (Type::String, Type::String) => Type::String,
                    _ => {
                        // Regular numeric addition
                        if !lhs_type.is_numeric() || !rhs_type.is_numeric() {
                            self.add_error(
                                format!("Cannot add {} and {}; operands must be numeric or time/duration",
                                        lhs_type, rhs_type),
                                binary_op.position.clone()
                            );
                            return Type::Unknown;
                        }

                        if lhs_type != rhs_type {
                            self.add_error(
                                format!("Cannot add {} and {}; types must match exactly", lhs_type, rhs_type),
                                binary_op.position.clone()
                            );
                            return Type::Unknown;
                        }

                        lhs_type
                    }
                }
            },
            BinaryOpKind::Sub => {
                // Special cases for time and duration
                match (&lhs_type, &rhs_type) {
                    (Type::Time, Type::Duration) => Type::Time,         // time - duration → time
                    (Type::Time, Type::Time) => Type::Duration,        // time - time → duration
                    (Type::Duration, Type::Duration) => Type::Duration, // duration - duration → duration
                    (Type::Duration, Type::Time) => {
                        self.add_error(
                            "Cannot subtract time from duration; use time - duration instead".to_string(),
                            binary_op.position.clone()
                        );
                        Type::Unknown
                    }
                    // Money subtraction: same currency required
                    (Type::Money, Type::Money) => Type::Money, // money - money → money (generic)
                    (Type::MoneyOf(c1), Type::MoneyOf(c2)) => {
                        if c1 == c2 {
                            Type::MoneyOf(c1.clone()) // same currency → result is that currency
                        } else {
                            self.add_error(
                                format!("Cannot mix money<{}> and money<{}> in arithmetic; explicit conversion required", c1, c2),
                                binary_op.position.clone()
                            );
                            Type::Unknown
                        }
                    }
                    (Type::Money, Type::MoneyOf(currency)) | (Type::MoneyOf(currency), Type::Money) => {
                        Type::MoneyOf(currency.clone()) // specific currency takes precedence
                    }
                    _ => {
                        // Regular numeric subtraction
                        if !lhs_type.is_numeric() || !rhs_type.is_numeric() {
                            self.add_error(
                                format!("Cannot subtract {} and {}; operands must be numeric or time/duration",
                                        lhs_type, rhs_type),
                                binary_op.position.clone()
                            );
                            return Type::Unknown;
                        }

                        if lhs_type != rhs_type {
                            self.add_error(
                                format!("Cannot subtract {} and {}; types must match exactly", lhs_type, rhs_type),
                                binary_op.position.clone()
                            );
                            return Type::Unknown;
                        }

                        lhs_type
                    }
                }
            },
            BinaryOpKind::Mul => {
                // Handle money multiplication: money * scalar
                match (&lhs_type, &rhs_type) {
                    (Type::Money, rhs_t) => {
                        if rhs_t.is_numeric() {
                            Type::Money // money * scalar → money
                        } else {
                            self.add_error(
                                format!("Cannot multiply money by non-numeric type"),
                                binary_op.position.clone()
                            );
                            Type::Unknown
                        }
                    }
                    (lhs_t, Type::Money) => {
                        if lhs_t.is_numeric() {
                            Type::Money // scalar * money → money
                        } else {
                            self.add_error(
                                format!("Cannot multiply money by non-numeric type"),
                                binary_op.position.clone()
                            );
                            Type::Unknown
                        }
                    }
                    (Type::MoneyOf(currency), rhs_t) => {
                        if rhs_t.is_numeric() {
                            Type::MoneyOf(currency.clone()) // money<USD> * scalar → money<USD>
                        } else {
                            self.add_error(
                                format!("Cannot multiply money by non-numeric type"),
                                binary_op.position.clone()
                            );
                            Type::Unknown
                        }
                    }
                    (lhs_t, Type::MoneyOf(currency)) => {
                        if lhs_t.is_numeric() {
                            Type::MoneyOf(currency.clone()) // scalar * money<USD> → money<USD>
                        } else {
                            self.add_error(
                                format!("Cannot multiply money by non-numeric type"),
                                binary_op.position.clone()
                            );
                            Type::Unknown
                        }
                    }
                    _ => {
                        // Regular numeric multiplication
                        if !lhs_type.is_numeric() || !rhs_type.is_numeric() {
                            self.add_error(
                                format!("Cannot multiply {} and {}; operands must be numeric", lhs_type, rhs_type),
                                binary_op.position.clone()
                            );
                            return Type::Unknown;
                        }

                        if lhs_type != rhs_type {
                            self.add_error(
                                format!("Cannot multiply {} and {}; types must match exactly", lhs_type, rhs_type),
                                binary_op.position.clone()
                            );
                            return Type::Unknown;
                        }

                        lhs_type
                    }
                }
            },
            BinaryOpKind::Div => {
                // Handle money division: money / scalar or money / money
                match (&lhs_type, &rhs_type) {
                    // money / scalar → money
                    (Type::Money, rhs) => {
                        if rhs.is_numeric() {
                            Type::Money
                        } else {
                            self.add_error(
                                format!("Cannot divide money by non-numeric type"),
                                binary_op.position.clone()
                            );
                            Type::Unknown
                        }
                    }
                    (Type::MoneyOf(currency), rhs) => {
                        if rhs.is_numeric() {
                            Type::MoneyOf(currency.clone())
                        } else {
                            self.add_error(
                                format!("Cannot divide money by non-numeric type"),
                                binary_op.position.clone()
                            );
                            Type::Unknown
                        }
                    }
                    // money / money (same currency) → f64 ratio
                    (Type::Money, Type::Money) => Type::F64,
                    (Type::MoneyOf(c1), Type::MoneyOf(c2)) => {
                        if c1 == c2 {
                            Type::F64 // ratio
                        } else {
                            self.add_error(
                                format!("Cannot divide money<{}> by money<{}>; currencies must match", c1, c2),
                                binary_op.position.clone()
                            );
                            Type::Unknown
                        }
                    }
                    (Type::Money, Type::MoneyOf(_)) | (Type::MoneyOf(_), Type::Money) => {
                        Type::F64 // ratio (mixed generic/specific money)
                    }
                    _ => {
                        // Regular numeric division
                        if !lhs_type.is_numeric() || !rhs_type.is_numeric() {
                            self.add_error(
                                format!("Cannot divide {} and {}; operands must be numeric", lhs_type, rhs_type),
                                binary_op.position.clone()
                            );
                            return Type::Unknown;
                        }

                        if lhs_type != rhs_type {
                            self.add_error(
                                format!("Cannot divide {} and {}; types must match exactly", lhs_type, rhs_type),
                                binary_op.position.clone()
                            );
                            return Type::Unknown;
                        }

                        lhs_type
                    }
                }
            },
            BinaryOpKind::Mod => {
                // Mod not supported for money
                if matches!(lhs_type, Type::Money | Type::MoneyOf(_)) || matches!(rhs_type, Type::Money | Type::MoneyOf(_)) {
                    self.add_error(
                        "Modulo operator not supported for money types".to_string(),
                        binary_op.position.clone()
                    );
                    return Type::Unknown;
                }

                // Regular numeric mod
                if !lhs_type.is_numeric() || !rhs_type.is_numeric() {
                    self.add_error(
                        format!("Cannot apply mod to non-numeric types {} and {}", lhs_type, rhs_type),
                        binary_op.position.clone()
                    );
                    return Type::Unknown;
                }

                if lhs_type != rhs_type {
                    self.add_error(
                        format!("Cannot mod {} and {}; types must match exactly", lhs_type, rhs_type),
                        binary_op.position.clone()
                    );
                    return Type::Unknown;
                }

                lhs_type
            },

            // Comparison operators: both operands must be same numeric type, result is bool
            BinaryOpKind::Less | BinaryOpKind::Greater |
            BinaryOpKind::LessEq | BinaryOpKind::GreaterEq |
            BinaryOpKind::NotLess | BinaryOpKind::NotGreater => {
                // Allow time-time, duration-duration, money-money, and optional comparisons
                if (lhs_type == Type::Time && rhs_type == Type::Time) ||
                   (lhs_type == Type::Duration && rhs_type == Type::Duration) ||
                   (matches!(&lhs_type, Type::Optional(inner) if **inner == Type::Time) &&
                    matches!(&rhs_type, Type::Optional(inner) if **inner == Type::Time)) ||
                   (matches!(&lhs_type, Type::Optional(inner) if **inner == Type::Duration) &&
                    matches!(&rhs_type, Type::Optional(inner) if **inner == Type::Duration)) {
                    Type::Bool
                } else {
                    // Check for money comparisons
                    match (&lhs_type, &rhs_type) {
                        (Type::Money, Type::Money) => Type::Bool,
                        (Type::MoneyOf(c1), Type::MoneyOf(c2)) => {
                            if c1 == c2 {
                                Type::Bool
                            } else {
                                self.add_error(
                                    format!("Cannot compare money<{}> and money<{}>; currencies must match", c1, c2),
                                    binary_op.position.clone()
                                );
                                Type::Unknown
                            }
                        }
                        (Type::Money, Type::MoneyOf(_)) | (Type::MoneyOf(_), Type::Money) => {
                            Type::Bool // mixed money comparisons are allowed
                        }
                        _ => {
                            // Regular numeric comparisons
                            if lhs_type.is_numeric() && rhs_type.is_numeric() {
                                if lhs_type != rhs_type {
                                    // usize/.length papercut fix: a bare integer literal adapts
                                    // to usize when compared against a usize value (e.g.
                                    // `arr.length > 2`). Mirrors existing bare-literal inference;
                                    // NOT general coercion — a typed i32 variable vs usize still
                                    // requires an exact match (use `let i: usize = 0` for counters).
                                    let lhs_is_int_literal = matches!(&*binary_op.lhs, Expression::IntLit(_, _));
                                    let rhs_is_int_literal = matches!(&*binary_op.rhs, Expression::IntLit(_, _));
                                    let usize_literal_ok =
                                        (lhs_type == Type::Usize && rhs_is_int_literal) ||
                                        (rhs_type == Type::Usize && lhs_is_int_literal);
                                    if !usize_literal_ok {
                                        self.add_error(
                                            format!("Cannot compare {} and {}; types must match exactly", lhs_type, rhs_type),
                                            binary_op.position.clone()
                                        );
                                        return Type::Unknown;
                                    }
                                }

                                Type::Bool
                            } else {
                                self.add_error(
                                    format!("Cannot compare {} and {}; types must be numeric, time, duration, or money",
                                            lhs_type, rhs_type),
                                    binary_op.position.clone()
                                );
                                Type::Unknown
                            }
                        }
                    }
                }
            },

            // Equality operators: both operands must be same type (any type), result is bool
            BinaryOpKind::Eq | BinaryOpKind::NotEq => {
                // Special handling for money types
                match (&lhs_type, &rhs_type) {
                    // Same type money comparisons
                    (Type::Money, Type::Money) => Type::Bool,
                    (Type::MoneyOf(c1), Type::MoneyOf(c2)) => {
                        if c1 == c2 {
                            Type::Bool
                        } else {
                            self.add_error(
                                format!("Cannot compare money<{}> and money<{}> for equality; currencies must match", c1, c2),
                                binary_op.position.clone()
                            );
                            Type::Unknown
                        }
                    }
                    // Mixed money and money<X>
                    (Type::Money, Type::MoneyOf(_)) | (Type::MoneyOf(_), Type::Money) => Type::Bool,
                    _ => {
                        // Regular equality check
                        if lhs_type != rhs_type {
                            // usize/.length papercut fix (equality): a bare integer literal
                            // adapts to usize when compared against a usize (e.g. `arr.length ?= 3`).
                            // Mirrors the relational-comparison fix; NOT general coercion.
                            let lhs_is_int_literal = matches!(&*binary_op.lhs, Expression::IntLit(_, _));
                            let rhs_is_int_literal = matches!(&*binary_op.rhs, Expression::IntLit(_, _));
                            let usize_literal_ok =
                                (lhs_type == Type::Usize && rhs_is_int_literal) ||
                                (rhs_type == Type::Usize && lhs_is_int_literal);
                            if !usize_literal_ok {
                                self.add_error(
                                    format!("Cannot compare {} and {} for equality; types must match exactly",
                                            lhs_type, rhs_type),
                                    binary_op.position.clone()
                                );
                                return Type::Unknown;
                            }
                        }

                        Type::Bool
                    }
                }
            },

            // Logical operators: both operands must be bool
            BinaryOpKind::And | BinaryOpKind::Or => {
                if lhs_type != Type::Bool {
                    self.add_error(
                        format!("Left operand of logical operator must be bool, found {}", lhs_type),
                        binary_op.position.clone()
                    );
                }

                if rhs_type != Type::Bool {
                    self.add_error(
                        format!("Right operand of logical operator must be bool, found {}", rhs_type),
                        binary_op.position.clone()
                    );
                }

                Type::Bool
            },

            // Bitwise operators: both operands must be same integer type
            BinaryOpKind::BitAnd | BinaryOpKind::BitOr | BinaryOpKind::BitXor |
            BinaryOpKind::ShiftLeft | BinaryOpKind::ShiftRight => {
                if !lhs_type.is_integer() {
                    self.add_error(
                        format!("Cannot apply bitwise operator to non-integer type {}", lhs_type),
                        binary_op.position.clone()
                    );
                    return Type::Unknown;
                }

                if lhs_type != rhs_type {
                    self.add_error(
                        format!("Cannot apply bitwise operator to {} and {}; types must match exactly",
                                lhs_type, rhs_type),
                        binary_op.position.clone()
                    );
                    return Type::Unknown;
                }

                lhs_type
            },

            // Type checking operators
            BinaryOpKind::Is | BinaryOpKind::IsNot => {
                // is/is not always returns bool
                Type::Bool
            },
        }
    }

    fn check_unary_op(&mut self, unary_op: &UnaryOp) -> Type {
        let operand_type = self.check_expression(&unary_op.operand);

        match unary_op.op {
            UnaryOpKind::Neg => {
                // Negation: operand must be numeric
                if !operand_type.is_numeric() {
                    self.add_error(
                        format!("Cannot negate non-numeric type {}", operand_type),
                        unary_op.position.clone()
                    );
                    Type::Unknown
                } else {
                    operand_type
                }
            },
            UnaryOpKind::Not => {
                // Logical not: operand must be bool or nothing
                if operand_type != Type::Bool && operand_type != Type::Nothing {
                    self.add_error(
                        format!("Cannot apply 'not' to non-bool type {}", operand_type),
                        unary_op.position.clone()
                    );
                    Type::Unknown
                } else {
                    Type::Bool
                }
            },
            UnaryOpKind::BitNot => {
                // Bitwise not: operand must be integer
                if !operand_type.is_integer() {
                    self.add_error(
                        format!("Cannot apply bitwise not to non-integer type {}", operand_type),
                        unary_op.position.clone()
                    );
                    Type::Unknown
                } else {
                    operand_type
                }
            },
        }
    }

    fn check_call(&mut self, call: &Call) -> Type {
        // Check if this is the special print() function
        if let Expression::Ident { name: func_name, .. } = call.callee.as_ref() {
            if func_name == "print" {
                return self.check_print_call(call);
            }
        }

        // Phase 10-γ-fixup: Check for watcher method calls
        if let Expression::MemberAccess(member_access) = call.callee.as_ref() {
            let object_type = self.check_expression(&member_access.object);
            if matches!(object_type, Type::Watcher) {
                return self.check_watcher_method_call(call, member_access);
            }
        }

        // Type check the callee
        let callee_type = self.check_expression(&call.callee);

        // Phase 7c-β: Handle function value calls
        if let Type::Function(param_types, return_type) = &callee_type {
            // Validate argument count
            if call.args.len() != param_types.len() {
                self.add_error(
                    format!("Function call expects {} arguments, got {}", param_types.len(), call.args.len()),
                    call.position.clone()
                );
                return Type::Unknown;
            }

            // Validate argument types match parameter types
            for (arg_index, (arg, expected_param_type)) in call.args.iter().zip(param_types.iter()).enumerate() {
                let arg_type = self.check_expression_with_expected_type(arg, expected_param_type);

                // Check compatibility using same logic as let-statement
                let types_compatible = if *expected_param_type == arg_type {
                    true
                } else if matches!(expected_param_type, Type::Money) && matches!(arg_type, Type::MoneyOf(_)) {
                    true  // money<X> can be passed where money is expected
                } else {
                    false
                };

                if !types_compatible {
                    self.add_error(
                        format!("Type mismatch in argument {}: expected {} but got {}",
                                arg_index + 1,
                                expected_param_type,
                                arg_type),
                        call.position.clone()
                    );
                }
            }

            return *return_type.clone();
        }

        // For non-function callees, type check arguments without expected types
        for arg in &call.args {
            self.check_expression(arg);
        }

        // Phase 6a-fixup: For nested functions, return the function's return type
        // For now, we use a simple approach: if callee is a function identifier,
        // return the type we stored (which is the return type)
        if let Expression::Ident { name: func_name, .. } = call.callee.as_ref() {
            // Look up the function in symbol table
            let func_type = self.lookup_variable(func_name, call.position.clone());
            if func_type != Type::Unknown {
                return func_type;
            }
        }

        // For other cases, return the callee type or unknown
        if callee_type != Type::Unknown {
            callee_type
        } else {
            Type::Unknown
        }
    }

    /// Special handling for print() built-in function
    /// Phase 4a-only: print() is treated as a magic function known to both type checker and codegen.
    /// This will be replaced with proper module imports in later phases.
    fn check_print_call(&mut self, call: &Call) -> Type {
        if call.args.len() != 1 {
            self.add_error(
                "print() function expects exactly one argument".to_string(),
                call.position.clone()
            );
            return Type::Unknown;
        }

        let arg = &call.args[0];
        let arg_type = self.check_expression(arg);

        // Check if the argument type is printable
        match arg_type {
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 |
            Type::F32 | Type::F64 | Type::Bool | Type::Usize | Type::Isize | Type::String |
            Type::Time | Type::Duration | Type::Money | Type::MoneyOf(_) |
            Type::Nothing | Type::UnknownType | Type::Tuple(_) => {
                // These types are printable
            }
            // Phase 1.5e: T? prints via runtime dispatch (print_optional_*):
            // the value when known, the unknown otherwise. Phase 2b step zero
            // trimmed the list to the inners that are both constructible
            // (payload allow-list) and printable.
            Type::Optional(ref inner) if matches!(inner.as_ref(),
                Type::I32 | Type::String) => {
                // Printable via print_optional_*
            }
            _ => {
                self.add_error(
                    format!("Cannot print value of type {}", arg_type),
                    call.position.clone()
                );
                return Type::Unknown;
            }
        }

        // print() returns i32 for now (will be nothing in Phase 9)
        Type::I32
    }

    /// Phase 10-γ-fixup: Special handling for watcher method calls
    fn check_watcher_method_call(&mut self, call: &Call, member_access: &MemberAccess) -> Type {
        // Validate that the method is one of the four valid watcher methods
        match member_access.member.as_str() {
            "pause" | "resume" | "end" => {
                // These methods return nothing and take no arguments
                if !call.args.is_empty() {
                    self.add_error(
                        format!("watcher method '{}' takes no arguments, got {}", member_access.member, call.args.len()),
                        call.position.clone()
                    );
                }
                Type::Nothing
            }
            "isActive" => {
                // This method returns bool and takes no arguments
                if !call.args.is_empty() {
                    self.add_error(
                        format!("watcher method 'isActive' takes no arguments, got {}", call.args.len()),
                        call.position.clone()
                    );
                }
                Type::Bool
            }
            _ => {
                self.add_error(
                    format!("watcher has no method '{}'; valid methods are 'pause', 'resume', 'end', 'isActive'", member_access.member),
                    call.position.clone()
                );
                Type::Unknown
            }
        }
    }

    // Scope management
    fn enter_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn exit_function_scope(&mut self) {
        self.scopes.pop();
        // Clear persistent refinements when exiting a function scope
        self.clear_persistent_refinements();
    }

    // Refinement scope management for type narrowing
    fn enter_refinement_scope(&mut self) {
        self.refinement_scopes.push(RefinementScope::new());
    }

    fn exit_refinement_scope(&mut self) {
        self.refinement_scopes.pop();
    }

    fn refine_variable_type(&mut self, name: &str, ty: Type) {
        if let Some(current_refinement_scope) = self.refinement_scopes.last_mut() {
            current_refinement_scope.refine(name.to_string(), ty);
        }
    }

    fn add_persistent_refinement(&mut self, name: &str, ty: Type) {
        self.persistent_refinements.insert(name.to_string(), ty);
    }

    fn clear_persistent_refinements(&mut self) {
        self.persistent_refinements.clear();
    }

    fn get_variable_refinement(&self, name: &str) -> Option<Type> {
        // First check current refinement scope (scope-local narrowing)
        if let Some(current_scope) = self.refinement_scopes.last() {
            if let Some(refined_type) = current_scope.lookup(name) {
                return Some(refined_type.clone());
            }
        }

        // Then check persistent refinements (post-block narrowing)
        if let Some(refined_type) = self.persistent_refinements.get(name) {
            return Some(refined_type.clone());
        }

        // Finally check final refinements (saved from program scope for codegen)
        self.final_refinements.get(name).cloned()
    }

    /// Write refinements to the AST after type checking is complete
    /// This allows codegen to read the refined types from the AST
    pub fn write_refinements_to_ast(&self, top_level: &mut TopLevel) {
        match top_level {
            TopLevel::Program(program) => {
                if let Some(ref mut body) = program.body {
                    for item in &mut body.items {
                        match item {
                            BlockItem::Statement(stmt) => {
                                self.write_refinements_to_statement(stmt);
                            }
                            BlockItem::Function(func) => {
                                if let Some(ref mut func_body) = func.body {
                                    self.write_refinements_to_block(func_body);
                                }
                            }
                            BlockItem::Watcher(_) => {
                                // Watchers are not yet implemented in Phase 10-α
                                // Skip refinement processing
                            }
                        }
                    }
                }
            }
            TopLevel::Module(_) => {
                // Module refinement writing not implemented yet
            }
        }
    }

    fn write_refinements_to_block(&self, block: &mut Block) {
        for item in &mut block.items {
            if let BlockItem::Statement(stmt) = item {
                self.write_refinements_to_statement(stmt);
            }
        }
    }

    fn write_refinements_to_statement(&self, stmt: &mut Statement) {
        match stmt {
            Statement::Let(let_decl) => {
                if let Some(ref mut init) = let_decl.initializer {
                    self.write_refinements_to_expression(init);
                }
            }
            Statement::Return(return_stmt) => {
                if let Some(ref mut value) = return_stmt.value {
                    self.write_refinements_to_expression(value);
                }
            }
            Statement::If(if_stmt) => {
                self.write_refinements_to_expression(&mut if_stmt.condition);
                self.write_refinements_to_block(&mut if_stmt.then_block);
                if let Some(ref mut else_block) = if_stmt.else_block {
                    self.write_refinements_to_block(else_block);
                }
            }
            Statement::While(while_stmt) => {
                self.write_refinements_to_expression(&mut while_stmt.condition);
                self.write_refinements_to_block(&mut while_stmt.body);
            }
            Statement::Loop(loop_stmt) => {
                self.write_refinements_to_block(&mut loop_stmt.body);
            }
            Statement::ForIn(for_in_stmt) => {
                self.write_refinements_to_expression(&mut for_in_stmt.iterable);
                self.write_refinements_to_block(&mut for_in_stmt.body);
            }
            Statement::Switch(switch_stmt) => {
                self.write_refinements_to_expression(&mut switch_stmt.value);
                for case in &mut switch_stmt.cases {
                    for case_stmt in &mut case.body {
                        self.write_refinements_to_statement(case_stmt);
                    }
                }
                if let Some(ref mut default_stmts) = switch_stmt.default {
                    for default_stmt in default_stmts {
                        self.write_refinements_to_statement(default_stmt);
                    }
                }
            }
            Statement::Assign(assign_stmt) => {
                self.write_refinements_to_expression(&mut assign_stmt.target);
                self.write_refinements_to_expression(&mut assign_stmt.value);
            }
            Statement::QualifiedOp(qualified_op) => {
                self.write_refinements_to_expression(&mut qualified_op.lhs);
                self.write_refinements_to_expression(&mut qualified_op.rhs);
            }
            Statement::StealthBlock(block, _) => {
                self.write_refinements_to_block(block);
            }
            Statement::ExprStatement(expr) => {
                self.write_refinements_to_expression(expr);
            }
            Statement::Break(_) | Statement::Continue(_) => {
                // No expressions to refine
            }
        }
    }

    fn write_refinements_to_expression(&self, expr: &mut Expression) {
        match expr {
            Expression::Ident { name, refined_type, .. } => {
                // Check if this variable has a current refinement
                if let Some(refined_ty) = self.get_variable_refinement(name) {
                    let declared_type = self.lookup_variable_without_error(name);
                    if refined_ty != declared_type {
                        *refined_type = Some(refined_ty.to_ast_type());
                    }
                }
            }
            Expression::BinaryOp(binary_op) => {
                self.write_refinements_to_expression(&mut binary_op.lhs);
                self.write_refinements_to_expression(&mut binary_op.rhs);
            }
            Expression::UnaryOp(unary_op) => {
                self.write_refinements_to_expression(&mut unary_op.operand);
            }
            Expression::Call(call) => {
                self.write_refinements_to_expression(&mut call.callee);
                for arg in &mut call.args {
                    self.write_refinements_to_expression(arg);
                }
            }
            Expression::MemberAccess(member) => {
                self.write_refinements_to_expression(&mut member.object);
            }
            Expression::IndexAccess(index) => {
                self.write_refinements_to_expression(&mut index.object);
                self.write_refinements_to_expression(&mut index.index);
            }
            Expression::IsCheck(is_check) => {
                self.write_refinements_to_expression(&mut is_check.expression);
            }
            Expression::ObjectIsCheck(obj_check) => {
                self.write_refinements_to_expression(&mut obj_check.lhs);
                self.write_refinements_to_expression(&mut obj_check.rhs);
            }
            Expression::QualifiedOp(qualified_op) => {
                self.write_refinements_to_expression(&mut qualified_op.lhs);
                self.write_refinements_to_expression(&mut qualified_op.rhs);
            }
            Expression::ObjectLiteral(obj_lit) => {
                for (_, prop_expr) in &mut obj_lit.properties {
                    self.write_refinements_to_expression(prop_expr);
                }
            }
            Expression::FunctionExpr(func_expr) => {
                self.write_refinements_to_block(&mut func_expr.body);
            }
            Expression::Match(match_expr) => {
                self.write_refinements_to_expression(&mut match_expr.value);
                for arm in &mut match_expr.arms {
                    match &mut arm.body {
                        MatchBody::Expression(expr) => {
                            self.write_refinements_to_expression(expr);
                        }
                        MatchBody::Block(block) => {
                            self.write_refinements_to_block(block);
                        }
                    }
                }
            }
            Expression::WeakRef(inner, _) => {
                self.write_refinements_to_expression(inner);
            }
            Expression::Unknown(unknown) => {
                self.write_refinements_to_expression(&mut unknown.reason);
                if let Some(ref mut options) = unknown.options {
                    self.write_refinements_to_expression(options);
                }
            }
            Expression::FString(fstring) => {
                for part in &mut fstring.parts {
                    if let FStringPart::Expression(expr, _) = part {
                        self.write_refinements_to_expression(expr);
                    }
                }
            }
            Expression::TupleLit(elements, _) => {
                for element in elements {
                    self.write_refinements_to_expression(element);
                }
            }
            Expression::TupleAccess(tuple_expr, _, _) => {
                self.write_refinements_to_expression(tuple_expr);
            }
            Expression::ArrayLit(elements, _) => {
                for element in elements {
                    self.write_refinements_to_expression(element);
                }
            }
            Expression::TypeAscription(inner, _, _) => {
                self.write_refinements_to_expression(inner);
            }
            Expression::WatcherExpr(watcher_expr) => {
                self.write_refinements_to_block(&mut watcher_expr.body);
            }
            // Literals don't contain variables to refine
            Expression::IntLit(_, _) | Expression::FloatLit(_, _) | Expression::DurationLit(_, _, _) |
            Expression::MoneyLit(_, _, _) | Expression::StringLit(_, _) | Expression::BoolLit(_, _) |
            Expression::This(_) | Expression::Nothing(_) => {
                // No variables to refine
            }
        }
    }

    fn declare_variable(&mut self, name: &str, ty: Type, position: Position) {
        if let Some(current_scope) = self.scopes.last_mut() {
            current_scope.declare(name.to_string(), ty, position);
        }
    }

    fn lookup_variable(&mut self, name: &str, position: Position) -> Type {
        // First, check for type refinements from innermost to outermost
        for refinement_scope in self.refinement_scopes.iter().rev() {
            if let Some(refined_type) = refinement_scope.lookup(name) {
                return refined_type.clone();
            }
        }

        // Second, check for persistent refinements
        if let Some(refined_type) = self.persistent_refinements.get(name) {
            return refined_type.clone();
        }

        // Then search regular scopes from innermost to outermost
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.lookup(name) {
                return symbol.ty.clone();
            }
        }

        // Variable not found
        self.add_error(
            format!("Undefined variable '{}'", name),
            position
        );
        Type::Unknown
    }

    /// Phase 4b: Check if a type is valid for truthy/falsy conditions
    fn is_condition_type(&self, ty: &Type) -> bool {
        match ty {
            // Bool is always valid
            Type::Bool => true,
            // All integer types are valid (truthy if non-zero)
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 |
            Type::Isize | Type::Usize => true,
            // All float types are valid (truthy if non-zero)
            Type::F32 | Type::F64 => true,
            // Nothing is always falsy
            Type::Nothing => true,
            // Other types not yet implemented for truthy/falsy
            _ => false,
        }
    }

    /// Check if an expression is an "is unknown" check and return the variable name and original type if so
    fn extract_is_unknown_check(&mut self, expr: &Expression) -> Option<(String, Type)> {
        if let Expression::IsCheck(is_check) = expr {
            if !is_check.negated && matches!(is_check.ty, ast::Type::Primitive(ast::PrimitiveType::Unknown)) {
                // This is a "variable is unknown" check
                if let Expression::Ident { name: var_name, .. } = is_check.expression.as_ref() {
                    // Get the original type of the variable
                    let original_type = self.lookup_variable_without_error(var_name);
                    return Some((var_name.clone(), original_type));
                }
            }
        }
        None
    }

    /// Helper to look up a variable without generating an error
    fn lookup_variable_without_error(&self, name: &str) -> Type {
        // First, check for type refinements from innermost to outermost
        for refinement_scope in self.refinement_scopes.iter().rev() {
            if let Some(refined_type) = refinement_scope.lookup(name) {
                return refined_type.clone();
            }
        }

        // Second, check for persistent refinements
        if let Some(refined_type) = self.persistent_refinements.get(name) {
            return refined_type.clone();
        }

        // Then search regular scopes from innermost to outermost
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.lookup(name) {
                return symbol.ty.clone();
            }
        }

        // Variable not found - return Unknown without error
        Type::Unknown
    }

    /// Check if a block always exits (ends with return, break, or continue)
    fn block_always_exits(&self, block: &Block) -> bool {
        // Find the last statement (skipping nested function/watcher declarations)
        if let Some(last_stmt) = block.statements_iter().last() {
            matches!(last_stmt,
                Statement::Return(_) |
                Statement::Break(_) |
                Statement::Continue(_)
            )
        } else {
            false
        }
    }

    fn check_qualified_op(&mut self, qualified_op: &QualifiedOp) {
        // Type check both operands
        let lhs_type = self.check_expression(&qualified_op.lhs);
        let rhs_type = self.check_expression(&qualified_op.rhs);

        // Check each qualifier in the list
        for qualifier_spec in &qualified_op.qualifiers {
            // 1. Check if qualifier is defined
            let qualifier_name = &qualifier_spec.name;
            let qualifier_info = self.qualifier_registry.get_qualifier(qualifier_name);

            if qualifier_info.is_none() {
                self.add_error(
                    format!("qualifier '{}' is not defined", qualifier_name),
                    qualifier_spec.position.clone(),
                );
                continue;
            }

            let qualifier_info = qualifier_info.unwrap();

            // Extract the information we need before making mutable borrows
            let contexts = qualifier_info.contexts.clone();
            let applies_to_type = qualifier_info.applies_to_type;
            let codegen_status = qualifier_info.codegen_status.clone();

            // 2. Check context (assignment vs equality) - BEFORE type checking
            let is_assignment = matches!(qualified_op.op, QualifiedOpKind::Assign);
            if !self.qualifier_registry.is_valid_in_context(qualifier_name, is_assignment) {
                let context_name = if is_assignment { "assignment" } else { "equality" };
                let allowed_context = match contexts {
                    QualifierContext::Assignment => "assignment only",
                    QualifierContext::Equality => "equality only",
                    QualifierContext::Both => unreachable!(),
                };
                self.add_error(
                    format!("qualifier '{}' applies to {}, not {}",
                           qualifier_name, allowed_context, context_name),
                    qualifier_spec.position.clone(),
                );
                continue;
            }

            // 3. Check arguments
            if let Err(err) = self.qualifier_registry.check_args(qualifier_name, qualifier_spec.arg.is_some()) {
                self.add_error(err, qualifier_spec.position.clone());
                continue;
            }

            // 4. Check if qualifier applies to the operand types
            if !applies_to_type(&lhs_type) {
                self.add_error(
                    format!("qualifier '{}' requires compatible types; got {}",
                           qualifier_name, lhs_type),
                    qualifier_spec.position.clone(),
                );
            }

            // 5. Check codegen status
            match &codegen_status {
                CodegenStatus::NotYetImplemented(phase) => {
                    self.add_error(
                        format!("qualifier '{}' for type {} is implemented in {}",
                               qualifier_name, lhs_type, phase),
                        qualifier_spec.position.clone(),
                    );
                }
                CodegenStatus::NotYetForType(phase) => {
                    self.add_error(
                        format!("qualifier '{}' for type {} is implemented in {}",
                               qualifier_name, lhs_type, phase),
                        qualifier_spec.position.clone(),
                    );
                }
                CodegenStatus::Implemented => {
                    // Good to go!
                }
            }
        }

        // Type checking for the operation itself
        match qualified_op.op {
            QualifiedOpKind::Assign => {
                // For assignment, both types must match
                if lhs_type != rhs_type {
                    self.add_error(
                        format!("Cannot assign {} to {}; types must match exactly",
                               rhs_type, lhs_type),
                        qualified_op.position.clone(),
                    );
                }
            }
            QualifiedOpKind::Eq | QualifiedOpKind::NotEq => {
                // For equality, both types must match and result is bool
                if lhs_type != rhs_type {
                    self.add_error(
                        format!("Cannot compare {} and {} for equality; types must match exactly",
                               lhs_type, rhs_type),
                        qualified_op.position.clone(),
                    );
                }
            }
        }
    }

    fn check_qualified_op_expression(&mut self, qualified_op: &QualifiedOp) -> Type {
        // Perform the same checks as the statement version
        self.check_qualified_op(qualified_op);

        // Return the appropriate type
        match qualified_op.op {
            QualifiedOpKind::Assign => {
                // Assignment returns the assigned value type
                self.check_expression(&qualified_op.lhs)
            }
            QualifiedOpKind::Eq | QualifiedOpKind::NotEq => {
                // Equality returns bool
                Type::Bool
            }
        }
    }

    fn check_fstring(&mut self, fstring: &FString) -> Type {
        // Check each part of the f-string
        for part in &fstring.parts {
            match part {
                FStringPart::Text(_) => {
                    // Text parts are always valid
                }
                FStringPart::Expression(expr, format_spec) => {
                    let expr_type = self.check_expression(expr);

                    // Check if the expression type can be interpolated
                    match expr_type {
                        Type::String |
                        Type::Bool |
                        Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
                        Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 |
                        Type::Isize | Type::Usize |
                        Type::F32 | Type::F64 |
                        Type::Nothing |
                        Type::UnknownType |
                        Type::Optional(_) |
                        Type::ObjectIterValue |
                        Type::Tuple(_) => {
                            // These types can be interpolated
                            // ObjectIterValue gets runtime dispatch like print()
                        }
                        _ => {
                            self.add_error(
                                format!("value of type {:?} cannot be interpolated in f-strings", expr_type),
                                expr.position()
                            );
                        }
                    }

                    // Check format specifier compatibility if present
                    if let Some(format_spec) = format_spec {
                        self.check_format_spec_compatibility(&expr_type, format_spec);
                    }
                }
            }
        }

        // F-strings always have type string
        Type::String
    }

    fn check_format_spec_compatibility(&mut self, expr_type: &Type, format_spec: &FormatSpec) {
        // Check type code compatibility
        if let Some(type_code) = format_spec.type_code {
            match type_code {
                'd' | 'x' | 'X' | 'b' | 'o' => {
                    // Integer format codes
                    match expr_type {
                        Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
                        Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 |
                        Type::Isize | Type::Usize => {
                            // OK
                        }
                        _ => {
                            self.add_error(
                                format!("format type '{}' requires an integer, got {:?}", type_code, expr_type),
                                format_spec.position.clone()
                            );
                        }
                    }
                }
                'e' | 'E' | 'f' | 'g' => {
                    // Float format codes
                    match expr_type {
                        Type::F32 | Type::F64 => {
                            // OK
                        }
                        _ => {
                            self.add_error(
                                format!("format type '{}' requires a float, got {:?}", type_code, expr_type),
                                format_spec.position.clone()
                            );
                        }
                    }
                }
                's' => {
                    // String format code
                    match expr_type {
                        Type::String => {
                            // OK
                        }
                        _ => {
                            self.add_error(
                                format!("format type '{}' incompatible with {:?}", type_code, expr_type),
                                format_spec.position.clone()
                            );
                        }
                    }
                }
                'c' => {
                    // Character format code
                    match expr_type {
                        Type::U8 | Type::I8 => {
                            // OK
                        }
                        _ => {
                            self.add_error(
                                format!("format type '{}' requires u8 or i8, got {:?}", type_code, expr_type),
                                format_spec.position.clone()
                            );
                        }
                    }
                }
                _ => {
                    self.add_error(
                        format!("unknown format type '{}'", type_code),
                        format_spec.position.clone()
                    );
                }
            }
        }

        // Check precision validity
        if let Some(_precision) = format_spec.precision {
            if let Some(type_code) = format_spec.type_code {
                match type_code {
                    'e' | 'E' | 'f' | 'g' => {
                        // Precision is valid with float type codes
                    }
                    'd' | 'x' | 'X' | 'b' | 'o' | 's' | 'c' => {
                        self.add_error(
                            format!("precision is not valid with format type '{}'", type_code),
                            format_spec.position.clone()
                        );
                    }
                    _ => {}
                }
            } else {
                // No type code but has precision - only valid for floats
                match expr_type {
                    Type::F32 | Type::F64 => {
                        // OK - implicit float formatting
                    }
                    _ => {
                        self.add_error(
                            "precision is only valid with float types".to_string(),
                            format_spec.position.clone()
                        );
                    }
                }
            }
        }
    }

    fn add_error(&mut self, message: String, position: Position) {
        self.errors.push(TypeError::new(message, position));
    }

    fn check_object_literal(&mut self, obj_lit: &ObjectLiteral) -> Type {
        let mut properties = Vec::new();

        // First pass: collect non-function properties
        for (prop_name, prop_expr) in &obj_lit.properties {
            match prop_expr {
                Expression::FunctionExpr(_) => {
                    // Skip function expressions for now - will handle in second pass
                }
                _ => {
                    let prop_type = self.check_expression(prop_expr);
                    // Phase 1.5e: a weak slot reads back as referent-or-unknown
                    let prop_type = if matches!(prop_expr, Expression::WeakRef(_, _))
                        && matches!(prop_type, Type::Object(_)) {
                        Type::Optional(Box::new(prop_type))
                    } else {
                        prop_type
                    };
                    properties.push((prop_name.clone(), prop_type));
                }
            }
        }

        // Create the preliminary object type for method context
        let object_type = Type::Object(properties.clone());

        // Second pass: check function expressions with method context
        for (prop_name, prop_expr) in &obj_lit.properties {
            if let Expression::FunctionExpr(func_expr) = prop_expr {
                // Set method context for this function expression
                let old_context = self.method_context.clone();
                self.method_context = Some(object_type.clone());

                let prop_type = self.check_function_expression(func_expr);
                properties.push((prop_name.clone(), prop_type));

                // Restore previous context
                self.method_context = old_context;
            }
        }

        Type::Object(properties)
    }

    /// Walk the prototype chain to find a property type (Phase 7b)
    fn find_property_in_chain(&self, object_type: &Type, property_name: &str, depth: usize) -> Option<Type> {
        const MAX_PROTO_DEPTH: usize = 100;

        if depth >= MAX_PROTO_DEPTH {
            return None; // Cycle detected or too deep
        }

        match object_type {
            Type::Object(ref properties) => {
                // First, look for the property directly on this object
                for (prop_name, prop_type) in properties {
                    if prop_name == property_name {
                        return Some(prop_type.clone());
                    }
                }

                // Property not found on this object - check prototype
                for (prop_name, prop_type) in properties {
                    if prop_name == "proto" {
                        // Found prototype property, recurse into it
                        return self.find_property_in_chain(prop_type, property_name, depth + 1);
                    }
                }

                // No prototype property found
                None
            },
            _ => None // Not an object type
        }
    }

    fn check_member_access(&mut self, member_access: &MemberAccess) -> Type {
        // Special handling for builtin types like `time`
        if let Expression::Ident { name, .. } = member_access.object.as_ref() {
            if name == "time" {
                // time is a builtin type with methods
                match member_access.member.as_str() {
                    "now" => return Type::Function(vec![], Box::new(Type::Time)), // time.now() -> time
                    "parse" => return Type::Function(vec![Type::String], Box::new(Type::Optional(Box::new(Type::Time)))), // time.parse(string) -> time?
                    _ => {
                        self.add_error(
                            format!("time builtin does not have a method named '{}'", member_access.member),
                            member_access.position.clone()
                        );
                        return Type::Unknown;
                    }
                }
            }
        }

        let object_type = self.check_expression(&member_access.object);

        match object_type {
            Type::Object(_) => {
                // Use prototype chain lookup (Phase 7b)
                if let Some(prop_type) = self.find_property_in_chain(&object_type, &member_access.member, 0) {
                    prop_type
                } else {
                    // Property not found anywhere in the chain - return nothing
                    Type::Nothing
                }
            },
            Type::Unknown => Type::Unknown, // Error recovery
            Type::UnknownType => {
                // Unknown types have known properties: reason and options
                match member_access.member.as_str() {
                    "reason" => Type::String,
                    "options" => Type::DynamicArray(Box::new(Type::String)),
                    _ => Type::Nothing, // Unknown properties return nothing (Phase 9a behavior)
                }
            },
            // Phase 1.5e: a weak property read is object-or-unknown. Member
            // access on it propagates the possibly-unknown state: a property
            // of type T reads as T?. A property that is itself a weak slot
            // (already T?) stays T? — unknown propagates as the same unknown,
            // it does not nest. If the referent's chain has no such property,
            // .reason/.options address the unknown state itself.
            Type::Optional(ref inner) if matches!(inner.as_ref(), Type::Object(_)) => {
                if let Some(prop_type) = self.find_property_in_chain(inner, &member_access.member, 0) {
                    if matches!(prop_type, Type::Optional(_)) {
                        prop_type
                    } else {
                        Type::Optional(Box::new(prop_type))
                    }
                } else {
                    match member_access.member.as_str() {
                        "reason" => Type::String,
                        "options" => Type::DynamicArray(Box::new(Type::String)),
                        _ => Type::Nothing,
                    }
                }
            },
            Type::DynamicArray(elem_type) => {
                // Arrays have .length property and mutation methods (Array Phase B)
                match member_access.member.as_str() {
                    "length" => Type::Usize,
                    "push" => {
                        // .push(x) where x: T -> Nothing (push returns nothing)
                        Type::Function(vec![*elem_type.clone()], Box::new(Type::Nothing))
                    },
                    "pop" => {
                        // .pop() -> T (returns element of array type)
                        Type::Function(vec![], elem_type.clone())
                    },
                    "remove" => {
                        // .remove(index) where index: integer -> T (returns removed element)
                        Type::Function(vec![Type::Usize], elem_type.clone())
                    },
                    "insert" => {
                        // .insert(index, elem) where index: integer, elem: T -> Nothing
                        Type::Function(vec![Type::Usize, *elem_type.clone()], Box::new(Type::Nothing))
                    },
                    "move" => {
                        // .move(from, to) where from: usize, to: usize -> Nothing
                        Type::Function(vec![Type::Usize, Type::Usize], Box::new(Type::Nothing))
                    },
                    "clear" => {
                        // .clear() -> Nothing (no args)
                        Type::Function(vec![], Box::new(Type::Nothing))
                    },
                    _ => {
                        self.add_error(
                            format!("Arrays do not have a property named '{}'", member_access.member),
                            member_access.position.clone()
                        );
                        Type::Unknown
                    }
                }
            },
            Type::String => {
                // Strings have .bytelength property
                match member_access.member.as_str() {
                    "bytelength" => Type::Usize,
                    _ => {
                        self.add_error(
                            format!("Strings do not have a property named '{}'", member_access.member),
                            member_access.position.clone()
                        );
                        Type::Unknown
                    }
                }
            },
            _ => {
                self.add_error(
                    format!("Cannot access property '{}' on non-object type {}", member_access.member, object_type),
                    member_access.position.clone()
                );
                Type::Unknown
            }
        }
    }

    /// Public method for codegen to get expression types
    pub fn get_expression_type(&self, expression: &Expression) -> Type {
        // Create a temporary type checker to evaluate the expression type
        // This is a simplified version that doesn't do error reporting
        match expression {
            Expression::IntLit(value, _) => Type::default_integer_type(*value),
            Expression::FloatLit(_, _) => Type::default_float_type(),
            Expression::StringLit(_, _) => Type::String,
            Expression::DurationLit(_, _, _) => Type::Duration,
            Expression::MoneyLit(_, currency, _) => Type::MoneyOf(currency.clone()),
            Expression::BoolLit(_, _) => Type::Bool,
            Expression::Ident { name, .. } => {
                // Look up in symbol table
                for scope in self.scopes.iter().rev() {
                    if let Some(symbol) = scope.lookup(name) {
                        return symbol.ty.clone();
                    }
                }
                Type::Unknown // Variable not found
            }
            Expression::MemberAccess(member_access) => {
                let object_type = self.get_expression_type(&member_access.object);
                match object_type {
                    Type::Object(_) => {
                        // Use prototype chain lookup (Phase 7b)
                        self.find_property_in_chain(&object_type, &member_access.member, 0)
                            .unwrap_or(Type::Unknown)
                    }
                    _ => Type::Unknown
                }
            }
            Expression::ObjectLiteral(obj_lit) => {
                let mut properties = Vec::new();
                for (prop_name, prop_expr) in &obj_lit.properties {
                    let prop_type = self.get_expression_type(prop_expr);
                    properties.push((prop_name.clone(), prop_type));
                }
                Type::Object(properties)
            }
            // Add other expression types as needed
            _ => Type::Unknown
        }
    }

    fn check_function_expression(&mut self, func_expr: &FunctionExpr) -> Type {
        // Record the scope depth before entering the function - variables from outer scopes are captures
        let outer_scope_depth = self.scopes.len();

        // Create a new scope for the function body
        self.enter_scope();

        // Phase 10-δ-γ: Track function scope depth for escape analysis
        let saved_depth = self.current_function_scope_depth;
        self.current_function_scope_depth = Some(self.scopes.len() - 1);

        // Phase 2b step zero: declared-type validation + return-type context
        let declared_return = Type::from_ast_type(&func_expr.return_type);
        self.validate_declared_type(&declared_return, &func_expr.position);
        let saved_return = self.current_function_return_type.take();
        self.current_function_return_type = Some(declared_return);
        let saved_unsafe = std::mem::take(&mut self.capture_unsafe_watchers);

        // Add parameters to the scope
        let mut param_types = Vec::new();
        for param in &func_expr.params {
            let param_type = Type::from_ast_type(&param.ty);
            self.validate_declared_type(&param_type, &param.position);
            param_types.push(param_type.clone());
            self.declare_variable(&param.name, param_type, param.position.clone());
        }

        // Phase 7c-γ: Collect capture metadata before checking for errors
        let mut captures = Vec::new();
        for statement in func_expr.body.statements_iter() {
            self.collect_captures_in_statement(statement, outer_scope_depth, &mut captures);
        }

        // Convert from types::Type to ast::Type for storage in AST
        let ast_captures: Vec<(String, ast::Type, Position)> = captures.iter()
            .map(|(name, ty, pos)| (name.clone(), ty.to_ast_type(), pos.clone()))
            .collect();

        // Store captures in the AST node (using RefCell for interior mutability)
        func_expr.captures.borrow_mut().clone_from(&ast_captures);

        // Phase 7c-δ: Captures are now supported!

        // Type-check the function body
        for statement in func_expr.body.statements_iter() {
            self.check_statement(statement);
        }

        // Optional-declared returns are validated per-return-statement via
        // current_function_return_type (Phase 2b step zero); the general
        // return-type check remains an open question.
        let return_type = Type::from_ast_type(&func_expr.return_type);

        // Restore previous function context
        self.current_function_scope_depth = saved_depth;
        self.current_function_return_type = saved_return;
        self.capture_unsafe_watchers = saved_unsafe;

        self.exit_function_scope();

        // Return the function type
        Type::Function(param_types, Box::new(return_type))
    }

    fn check_watcher_expression(&mut self, watcher_expr: &WatcherExpr) -> Type {
        // Record the scope depth before entering the watcher - variables from outer scopes are captures
        let outer_scope_depth = self.scopes.len();

        self.enter_scope();

        for sub in &watcher_expr.subscriptions {
            self.check_subscription_and_bind(sub, &watcher_expr.position);
        }

        // Phase 10a: Collect capture metadata before checking for errors
        let mut captures = Vec::new();
        for statement in watcher_expr.body.statements_iter() {
            self.collect_captures_in_statement(statement, outer_scope_depth, &mut captures);
        }

        // Convert from types::Type to ast::Type for storage in AST
        let ast_captures: Vec<(String, ast::Type, Position)> = captures.iter()
            .map(|(name, ty, pos)| (name.clone(), ty.to_ast_type(), pos.clone()))
            .collect();

        // Store captures in the AST node (using RefCell for interior mutability)
        watcher_expr.captures.borrow_mut().clone_from(&ast_captures);

        self.check_block(&watcher_expr.body);
        self.check_no_return_with_value(&watcher_expr.body, watcher_expr.position.clone());

        self.exit_function_scope();

        Type::Watcher
    }

    fn check_for_captures_in_statement(&mut self, statement: &Statement, outer_scope_depth: usize) {
        match statement {
            Statement::Let(let_stmt) => {
                if let Some(initializer) = &let_stmt.initializer {
                    self.check_for_captures_in_expression(initializer, outer_scope_depth);
                }
            }
            Statement::Return(return_stmt) => {
                if let Some(expr) = &return_stmt.value {
                    self.check_for_captures_in_expression(expr, outer_scope_depth);
                }
            }
            Statement::If(if_stmt) => {
                self.check_for_captures_in_expression(&if_stmt.condition, outer_scope_depth);
                for stmt in if_stmt.then_block.statements_iter() {
                    self.check_for_captures_in_statement(stmt, outer_scope_depth);
                }
                if let Some(else_block) = &if_stmt.else_block {
                    for stmt in else_block.statements_iter() {
                        self.check_for_captures_in_statement(stmt, outer_scope_depth);
                    }
                }
            }
            Statement::While(while_stmt) => {
                self.check_for_captures_in_expression(&while_stmt.condition, outer_scope_depth);
                for stmt in while_stmt.body.statements_iter() {
                    self.check_for_captures_in_statement(stmt, outer_scope_depth);
                }
            }
            Statement::Loop(loop_stmt) => {
                for stmt in loop_stmt.body.statements_iter() {
                    self.check_for_captures_in_statement(stmt, outer_scope_depth);
                }
            }
            Statement::ForIn(for_in_stmt) => {
                self.check_for_captures_in_expression(&for_in_stmt.iterable, outer_scope_depth);
                for stmt in for_in_stmt.body.statements_iter() {
                    self.check_for_captures_in_statement(stmt, outer_scope_depth);
                }
            }
            Statement::Assign(assign_stmt) => {
                self.check_for_captures_in_expression(&assign_stmt.target, outer_scope_depth);
                self.check_for_captures_in_expression(&assign_stmt.value, outer_scope_depth);
            }
            Statement::ExprStatement(expr) => {
                self.check_for_captures_in_expression(expr, outer_scope_depth);
            }
            Statement::QualifiedOp(qualified_op) => {
                self.check_for_captures_in_expression(&qualified_op.lhs, outer_scope_depth);
                self.check_for_captures_in_expression(&qualified_op.rhs, outer_scope_depth);
            }
            Statement::Switch(switch_stmt) => {
                self.check_for_captures_in_expression(&switch_stmt.value, outer_scope_depth);
                for case in &switch_stmt.cases {
                    for stmt in &case.body {
                        self.check_for_captures_in_statement(stmt, outer_scope_depth);
                    }
                }
                if let Some(default_statements) = &switch_stmt.default {
                    for stmt in default_statements {
                        self.check_for_captures_in_statement(stmt, outer_scope_depth);
                    }
                }
            }
            Statement::StealthBlock(block, _) => {
                for stmt in block.statements_iter() {
                    self.check_for_captures_in_statement(stmt, outer_scope_depth);
                }
            }
            Statement::Break(_) | Statement::Continue(_) => {
                // No expressions to check
            }
        }
    }

    fn check_for_captures_in_expression(&mut self, expression: &Expression, outer_scope_depth: usize) {
        match expression {
            Expression::Ident { name, position, .. } => {
                // Check if this identifier refers to a variable from an outer scope
                if self.is_variable_capture(name, outer_scope_depth) {
                    self.add_error(
                        "function expressions cannot capture variables (Phase 7c-γ will add capture detection, Phase 7c-δ will implement closures)".to_string(),
                        position.clone()
                    );
                }
            }
            Expression::BinaryOp(binary_op) => {
                self.check_for_captures_in_expression(&binary_op.lhs, outer_scope_depth);
                self.check_for_captures_in_expression(&binary_op.rhs, outer_scope_depth);
            }
            Expression::UnaryOp(unary_op) => {
                self.check_for_captures_in_expression(&unary_op.operand, outer_scope_depth);
            }
            Expression::Call(call) => {
                self.check_for_captures_in_expression(&call.callee, outer_scope_depth);
                for arg in &call.args {
                    self.check_for_captures_in_expression(arg, outer_scope_depth);
                }
            }
            Expression::MemberAccess(member_access) => {
                self.check_for_captures_in_expression(&member_access.object, outer_scope_depth);
            }
            Expression::IndexAccess(index_access) => {
                self.check_for_captures_in_expression(&index_access.object, outer_scope_depth);
                self.check_for_captures_in_expression(&index_access.index, outer_scope_depth);
            }
            Expression::IsCheck(is_check) => {
                self.check_for_captures_in_expression(&is_check.expression, outer_scope_depth);
            }
            Expression::ObjectIsCheck(obj_is_check) => {
                self.check_for_captures_in_expression(&obj_is_check.lhs, outer_scope_depth);
                self.check_for_captures_in_expression(&obj_is_check.rhs, outer_scope_depth);
            }
            Expression::QualifiedOp(qualified_op) => {
                self.check_for_captures_in_expression(&qualified_op.lhs, outer_scope_depth);
                self.check_for_captures_in_expression(&qualified_op.rhs, outer_scope_depth);
            }
            Expression::ObjectLiteral(obj_lit) => {
                for (_, prop_expr) in &obj_lit.properties {
                    self.check_for_captures_in_expression(prop_expr, outer_scope_depth);
                }
            }
            Expression::FunctionExpr(func_expr) => {
                // Nested function expressions get their own capture check
                self.check_function_expression(func_expr);
            }
            Expression::Match(match_expr) => {
                self.check_for_captures_in_expression(&match_expr.value, outer_scope_depth);
                for arm in &match_expr.arms {
                    match &arm.body {
                        MatchBody::Expression(expr) => {
                            self.check_for_captures_in_expression(expr, outer_scope_depth);
                        }
                        MatchBody::Block(block) => {
                            for stmt in block.statements_iter() {
                                self.check_for_captures_in_statement(stmt, outer_scope_depth);
                            }
                        }
                    }
                }
            }
            Expression::WeakRef(expr, _) => {
                // Check for captures in the inner expression
                self.check_for_captures_in_expression(expr, outer_scope_depth);
            }
            Expression::Unknown(unknown_construction) => {
                // Check for captures in reason and options expressions
                self.check_for_captures_in_expression(&unknown_construction.reason, outer_scope_depth);
                if let Some(ref options) = unknown_construction.options {
                    self.check_for_captures_in_expression(options, outer_scope_depth);
                }
            }
            Expression::TupleLit(elements, _) => {
                for element in elements {
                    self.check_for_captures_in_expression(element, outer_scope_depth);
                }
            }
            Expression::TupleAccess(tuple_expr, _, _) => {
                self.check_for_captures_in_expression(tuple_expr, outer_scope_depth);
            }
            Expression::ArrayLit(elements, _) => {
                for element in elements {
                    self.check_for_captures_in_expression(element, outer_scope_depth);
                }
            }
            Expression::TypeAscription(inner, _, _) => {
                self.check_for_captures_in_expression(inner, outer_scope_depth);
            }
            Expression::WatcherExpr(watcher_expr) => {
                // Phase 10a: Watcher expressions get their own capture check
                self.check_watcher_expression(watcher_expr);
            }
            // Literal expressions don't contain variable references
            Expression::IntLit(_, _) | Expression::FloatLit(_, _) | Expression::DurationLit(_, _, _) |
            Expression::MoneyLit(_, _, _) | Expression::StringLit(_, _) |
            Expression::FString(_) | Expression::BoolLit(_, _) | Expression::Nothing(_) | Expression::This(_) => {
                // No variables to capture
            }
        }
    }

    fn is_variable_capture(&self, name: &str, outer_scope_depth: usize) -> bool {
        // Check if the variable exists in an outer scope (before the function scope)
        for (scope_index, scope) in self.scopes.iter().enumerate() {
            if scope_index < outer_scope_depth {
                if scope.lookup(name).is_some() {
                    return true; // Variable found in outer scope - this is a capture
                }
            }
        }
        false
    }

    // Phase 7c-γ: Capture collection methods (collect metadata instead of immediate error)
    fn collect_captures_in_statement(&mut self, statement: &Statement, outer_scope_depth: usize, captures: &mut Vec<(String, Type, Position)>) {
        match statement {
            Statement::Let(let_stmt) => {
                if let Some(initializer) = &let_stmt.initializer {
                    self.collect_captures_in_expression(initializer, outer_scope_depth, captures);
                }
            }
            Statement::Return(return_stmt) => {
                if let Some(expr) = &return_stmt.value {
                    self.collect_captures_in_expression(expr, outer_scope_depth, captures);
                }
            }
            Statement::If(if_stmt) => {
                self.collect_captures_in_expression(&if_stmt.condition, outer_scope_depth, captures);
                for stmt in if_stmt.then_block.statements_iter() {
                    self.collect_captures_in_statement(stmt, outer_scope_depth, captures);
                }
                if let Some(else_block) = &if_stmt.else_block {
                    for stmt in else_block.statements_iter() {
                        self.collect_captures_in_statement(stmt, outer_scope_depth, captures);
                    }
                }
            }
            Statement::While(while_stmt) => {
                self.collect_captures_in_expression(&while_stmt.condition, outer_scope_depth, captures);
                for stmt in while_stmt.body.statements_iter() {
                    self.collect_captures_in_statement(stmt, outer_scope_depth, captures);
                }
            }
            Statement::Loop(loop_stmt) => {
                for stmt in loop_stmt.body.statements_iter() {
                    self.collect_captures_in_statement(stmt, outer_scope_depth, captures);
                }
            }
            Statement::ForIn(for_in_stmt) => {
                self.collect_captures_in_expression(&for_in_stmt.iterable, outer_scope_depth, captures);
                for stmt in for_in_stmt.body.statements_iter() {
                    self.collect_captures_in_statement(stmt, outer_scope_depth, captures);
                }
            }
            Statement::Assign(assign_stmt) => {
                self.collect_captures_in_expression(&assign_stmt.target, outer_scope_depth, captures);
                self.collect_captures_in_expression(&assign_stmt.value, outer_scope_depth, captures);
            }
            Statement::ExprStatement(expr) => {
                self.collect_captures_in_expression(expr, outer_scope_depth, captures);
            }
            Statement::QualifiedOp(qualified_op) => {
                self.collect_captures_in_expression(&qualified_op.lhs, outer_scope_depth, captures);
                self.collect_captures_in_expression(&qualified_op.rhs, outer_scope_depth, captures);
            }
            Statement::Switch(switch_stmt) => {
                self.collect_captures_in_expression(&switch_stmt.value, outer_scope_depth, captures);
                for case in &switch_stmt.cases {
                    for stmt in &case.body {
                        self.collect_captures_in_statement(stmt, outer_scope_depth, captures);
                    }
                }
                if let Some(default_statements) = &switch_stmt.default {
                    for stmt in default_statements {
                        self.collect_captures_in_statement(stmt, outer_scope_depth, captures);
                    }
                }
            }
            Statement::StealthBlock(block, _) => {
                for stmt in block.statements_iter() {
                    self.collect_captures_in_statement(stmt, outer_scope_depth, captures);
                }
            }
            Statement::Break(_) | Statement::Continue(_) => {
                // No expressions to check
            }
        }
    }

    fn collect_captures_in_expression(&mut self, expression: &Expression, outer_scope_depth: usize, captures: &mut Vec<(String, Type, Position)>) {
        match expression {
            Expression::Ident { name, position, .. } => {
                // Check if this identifier refers to a variable from an outer scope
                if let Some((var_type, _)) = self.find_variable_in_outer_scope(name, outer_scope_depth) {
                    // Check if we already recorded this capture (by name)
                    if !captures.iter().any(|(capture_name, _, _)| capture_name == name) {
                        captures.push((name.clone(), var_type, position.clone()));
                    }
                }
            }
            Expression::BinaryOp(binary_op) => {
                self.collect_captures_in_expression(&binary_op.lhs, outer_scope_depth, captures);
                self.collect_captures_in_expression(&binary_op.rhs, outer_scope_depth, captures);
            }
            Expression::UnaryOp(unary_op) => {
                self.collect_captures_in_expression(&unary_op.operand, outer_scope_depth, captures);
            }
            Expression::Call(call) => {
                self.collect_captures_in_expression(&call.callee, outer_scope_depth, captures);
                for arg in &call.args {
                    self.collect_captures_in_expression(arg, outer_scope_depth, captures);
                }
            }
            Expression::MemberAccess(member_access) => {
                self.collect_captures_in_expression(&member_access.object, outer_scope_depth, captures);
            }
            Expression::IndexAccess(index_access) => {
                self.collect_captures_in_expression(&index_access.object, outer_scope_depth, captures);
                self.collect_captures_in_expression(&index_access.index, outer_scope_depth, captures);
            }
            Expression::IsCheck(is_check) => {
                self.collect_captures_in_expression(&is_check.expression, outer_scope_depth, captures);
            }
            Expression::ObjectIsCheck(obj_is_check) => {
                self.collect_captures_in_expression(&obj_is_check.lhs, outer_scope_depth, captures);
                self.collect_captures_in_expression(&obj_is_check.rhs, outer_scope_depth, captures);
            }
            Expression::QualifiedOp(qualified_op) => {
                self.collect_captures_in_expression(&qualified_op.lhs, outer_scope_depth, captures);
                self.collect_captures_in_expression(&qualified_op.rhs, outer_scope_depth, captures);
            }
            Expression::ObjectLiteral(obj_lit) => {
                for (_, prop_expr) in &obj_lit.properties {
                    self.collect_captures_in_expression(prop_expr, outer_scope_depth, captures);
                }
            }
            Expression::FunctionExpr(func_expr) => {
                // Nested function expressions get their own capture check
                self.check_function_expression(func_expr);
            }
            Expression::Match(match_expr) => {
                self.collect_captures_in_expression(&match_expr.value, outer_scope_depth, captures);
                for arm in &match_expr.arms {
                    match &arm.body {
                        MatchBody::Expression(expr) => {
                            self.collect_captures_in_expression(expr, outer_scope_depth, captures);
                        }
                        MatchBody::Block(block) => {
                            for stmt in block.statements_iter() {
                                self.collect_captures_in_statement(stmt, outer_scope_depth, captures);
                            }
                        }
                    }
                }
            }
            Expression::WeakRef(expr, _) => {
                // Collect captures in the inner expression
                self.collect_captures_in_expression(expr, outer_scope_depth, captures);
            }
            Expression::Unknown(unknown_construction) => {
                // Collect captures from the reason and options expressions
                self.collect_captures_in_expression(&unknown_construction.reason, outer_scope_depth, captures);
                if let Some(ref options) = unknown_construction.options {
                    self.collect_captures_in_expression(options, outer_scope_depth, captures);
                }
            }
            Expression::TupleLit(elements, _) => {
                for element in elements {
                    self.collect_captures_in_expression(element, outer_scope_depth, captures);
                }
            }
            Expression::TupleAccess(tuple_expr, _, _) => {
                self.collect_captures_in_expression(tuple_expr, outer_scope_depth, captures);
            }
            Expression::ArrayLit(elements, _) => {
                for element in elements {
                    self.collect_captures_in_expression(element, outer_scope_depth, captures);
                }
            }
            Expression::TypeAscription(inner, _, _) => {
                self.collect_captures_in_expression(inner, outer_scope_depth, captures);
            }
            Expression::WatcherExpr(watcher_expr) => {
                // Phase 10a: Watcher expressions get their own capture check
                self.check_watcher_expression(watcher_expr);
            }
            // Literal expressions don't contain variable references
            Expression::IntLit(_, _) | Expression::FloatLit(_, _) | Expression::DurationLit(_, _, _) |
            Expression::MoneyLit(_, _, _) | Expression::StringLit(_, _) |
            Expression::FString(_) | Expression::BoolLit(_, _) | Expression::Nothing(_) | Expression::This(_) => {
                // No variables to capture
            }
        }
    }

    fn find_variable_in_outer_scope(&self, name: &str, outer_scope_depth: usize) -> Option<(Type, Position)> {
        // Look for the variable in outer scopes (before the function scope)
        for (scope_index, scope) in self.scopes.iter().enumerate() {
            if scope_index < outer_scope_depth {
                if let Some(symbol) = scope.lookup(name) {
                    return Some((symbol.ty.clone(), symbol.position.clone()));
                }
            }
        }
        None
    }

    fn check_match_expression(&mut self, match_expr: &MatchExpr) -> Type {
        // Check the matched expression
        let matched_type = self.check_expression(&match_expr.value);

        // Check all arms
        let mut arm_types = Vec::new();
        let mut has_wildcard = false;

        for arm in &match_expr.arms {
            // Check pattern compatibility with matched expression type
            match &arm.pattern {
                MatchPattern::Literal(literal) => {
                    let pattern_type = self.get_literal_type(literal);
                    if pattern_type != matched_type {
                        self.add_error(
                            format!("Match pattern type {} does not match expression type {}",
                                pattern_type, matched_type),
                            arm.position.clone()
                        );
                    }
                }
                MatchPattern::Wildcard => {
                    has_wildcard = true;
                }
            }

            // Check body and collect its type
            let body_type = match &arm.body {
                MatchBody::Expression(expr) => self.check_expression(expr),
                MatchBody::Block(block) => {
                    // For blocks in match arms, we consider the type to be Nothing
                    // unless it's used as an expression (then it needs a return type)
                    self.enter_scope();
                    self.check_block(block);
                    self.exit_scope();
                    Type::Nothing  // Blocks don't have return values in this context
                }
            };

            arm_types.push(body_type);
        }

        // For match as expression: check that all arms have same type and exhaustiveness
        // For match as statement: no type checking needed
        // We can't distinguish context here, so we'll be permissive for now

        // Check type consistency for expression context
        if !arm_types.is_empty() {
            let first_type = &arm_types[0];
            for (i, arm_type) in arm_types.iter().enumerate().skip(1) {
                if arm_type != first_type && *arm_type != Type::Nothing && *first_type != Type::Nothing {
                    self.add_error(
                        format!("Match arm {} has type {} but first arm has type {}",
                            i + 1, arm_type, first_type),
                        match_expr.arms[i].position.clone()
                    );
                }
            }

            // For exhaustiveness check on expressions with non-Nothing types
            if !has_wildcard && *first_type != Type::Nothing {
                // For now, only require wildcard for non-boolean types
                // (boolean can be exhaustive with just true/false)
                if matched_type != Type::Bool {
                    self.add_error(
                        "Match expression must be exhaustive; add a wildcard pattern (_) or cover all values".to_string(),
                        match_expr.position.clone()
                    );
                }
            }

            first_type.clone()
        } else {
            Type::Nothing
        }
    }

    fn get_literal_type(&self, literal: &Literal) -> Type {
        match literal {
            Literal::Integer(value) => Type::default_integer_type(*value),
            Literal::Float(_) => Type::default_float_type(),
            Literal::String(_) => Type::String,
            Literal::Bool(_) => Type::Bool,
        }
    }

    fn check_unknown_construction(&mut self, unknown_construction: &UnknownConstruction) -> Type {
        // Check that reason is a string expression
        let reason_type = self.check_expression(&unknown_construction.reason);
        if reason_type != Type::String {
            self.add_error(
                format!("unknown reason must be a string, found {}", reason_type),
                unknown_construction.reason.position()
            );
        }

        // Check options argument if present
        if let Some(ref options_expr) = unknown_construction.options {
            let options_type = self.check_expression(options_expr);
            // For now, just check that it's an array - we'll be more specific about [string] in the future
            match options_type {
                Type::DynamicArray(element_type) => {
                    if *element_type != Type::String {
                        self.add_error(
                            format!("unknown options must be an array of strings, found [{}]", element_type),
                            options_expr.position()
                        );
                    }
                },
                _ => {
                    self.add_error(
                        format!("unknown options must be an array of strings, found {}", options_type),
                        options_expr.position()
                    );
                }
            }
        }

        // Return unknown type
        Type::UnknownType
    }
}

// Helper trait to get position from expressions
trait HasPosition {
    fn position(&self) -> Position;
}

impl HasPosition for Expression {
    fn position(&self) -> Position {
        match self {
            Expression::IntLit(_, pos) => pos.clone(),
            Expression::FloatLit(_, pos) => pos.clone(),
            Expression::StringLit(_, pos) => pos.clone(),
            Expression::DurationLit(_, _, pos) => pos.clone(),
            Expression::MoneyLit(_, _, pos) => pos.clone(),
            Expression::FString(fstring) => fstring.position.clone(),
            Expression::BoolLit(_, pos) => pos.clone(),
            Expression::Ident { position, .. } => position.clone(),
            Expression::This(pos) => pos.clone(),
            Expression::BinaryOp(op) => op.position.clone(),
            Expression::UnaryOp(op) => op.position.clone(),
            Expression::Call(call) => call.position.clone(),
            Expression::MemberAccess(access) => access.position.clone(),
            Expression::IndexAccess(access) => access.position.clone(),
            Expression::IsCheck(check) => check.position.clone(),
            Expression::ObjectIsCheck(obj_is_check) => obj_is_check.position.clone(),
            Expression::QualifiedOp(qualified_op) => qualified_op.position.clone(),
            Expression::ObjectLiteral(obj_lit) => obj_lit.position.clone(),
            Expression::FunctionExpr(func_expr) => func_expr.position.clone(),
            Expression::Match(match_expr) => match_expr.position.clone(),
            Expression::WeakRef(_, pos) => pos.clone(),
            Expression::Nothing(pos) => pos.clone(),
            Expression::Unknown(unknown_construction) => unknown_construction.position.clone(),
            Expression::TupleLit(_, pos) => pos.clone(),
            Expression::TupleAccess(_, _, pos) => pos.clone(),
            Expression::ArrayLit(_, pos) => pos.clone(),
            Expression::TypeAscription(_, _, pos) => pos.clone(),
            Expression::WatcherExpr(watcher_expr) => watcher_expr.position.clone(),
        }
    }

}

// Helper trait for binary operation names
trait OperatorName {
    fn operator_name(&self) -> &'static str;
}

impl OperatorName for BinaryOpKind {
    fn operator_name(&self) -> &'static str {
        match self {
            BinaryOpKind::Add => "add",
            BinaryOpKind::Sub => "subtract",
            BinaryOpKind::Mul => "multiply",
            BinaryOpKind::Div => "divide",
            BinaryOpKind::Mod => "mod",
            _ => "operate on",
        }
    }
}