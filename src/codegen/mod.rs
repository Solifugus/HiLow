use crate::ast::*;
use crate::types::Type;
use crate::typecheck::TypeChecker;
use crate::lexer::Position;
use std::collections::{HashMap, HashSet};

/// Types of heap allocations for ownership tracking (Phase 8a)
#[derive(Debug, Clone, PartialEq)]
pub enum HeapType {
    Object,         // HiLowObject*
    Function,       // HiLowFunction*
    FStringBuffer,  // char* from f-string
    Unknown,        // HiLowUnknown*
    Optional,       // T? - may contain unknown or success value
    Watcher,        // HiLowWatcher*
    Array,          // HiLowArray*
    Tuple(Vec<Type>), // Tuple with heap-allocated elements
    Scalar,         // HiLowScalar* — boxed watched scalar (Phase 3b)
}

/// Phase 3b: the kind of cell a watcher env slot holds. Every slot is a
/// retained cell pointer — scalar cells joined containers when boxing landed.
#[derive(Debug, Clone, Copy, PartialEq)]
enum EnvSlot {
    Scalar,
    Array,
    Object,
}

impl EnvSlot {
    fn c_type(self) -> &'static str {
        match self {
            EnvSlot::Scalar => "HiLowScalar*",
            EnvSlot::Array => "HiLowArray*",
            EnvSlot::Object => "HiLowObject*",
        }
    }
    fn retain_fn(self) -> &'static str {
        match self {
            EnvSlot::Scalar => "hl_scalar_retain",
            EnvSlot::Array => "hl_array_retain",
            EnvSlot::Object => "hl_object_retain",
        }
    }
    fn release_fn(self) -> &'static str {
        match self {
            EnvSlot::Scalar => "hl_scalar_release",
            EnvSlot::Array => "hl_array_release",
            EnvSlot::Object => "hl_object_release",
        }
    }
}

/// Loop context for control-transfer cleanup (Phase 1.5b).
/// Pushed while generating a loop body so break/continue can release
/// resources they would otherwise jump past.
#[derive(Debug, Clone)]
struct LoopFrame {
    /// Extra release statements the loop bottom would normally run
    /// (e.g. the object for-in key release), re-emitted before break/continue
    extra_cleanups: Vec<String>,
    /// Length of enclosing_temp_frames at loop-body entry: frames above this
    /// belong to statements inside the loop and must release on break/continue
    temp_frame_base: usize,
}

/// Expression context for temporary tracking (Phase 11a expression-temporary cleanup)
#[derive(Debug, Clone, PartialEq)]
enum ExprContext {
    Owned,      // Expression result will be bound/returned - track in heap_owners
    Temporary,  // Expression result is a temporary - track in temp_owners for statement-end cleanup
}

/// Context for function expression generation to detect escaping closures (Phase 8a)
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionExprContext {
    Normal,           // Regular expression context
    ReturnValue,      // Being returned from function
    ObjectProperty,   // Being stored as object property
    LetInitializer,   // Being assigned to let variable
}

/// Errors that can occur during code generation
#[derive(Debug)]
pub enum CodegenError {
    UnsupportedFeature {
        feature: String,
        phase: String,
    },
    MultiOwnerHeapValue {
        message: String,
    },
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodegenError::UnsupportedFeature { feature, phase } => {
                write!(f, "Unsupported feature '{}' - will be implemented in {}", feature, phase)
            }
            CodegenError::MultiOwnerHeapValue { message } => {
                write!(f, "{}", message)
            }
        }
    }
}

impl std::error::Error for CodegenError {}

/// C code generator
pub struct CodeGenerator {
    /// Output C code
    output: String,
    /// Variable counter for generating unique names
    var_counter: usize,
    /// Function counter for generating unique function names (Phase 7c-β)
    function_counter: usize,
    /// Generated function definitions (Phase 7c-β)
    generated_functions: String,
    /// Optional variables declared in main program (for cleanup fix)
    main_program_optionals: Vec<String>,
    /// Function symbols - maps function name to its return type
    functions: HashMap<String, Type>,
    /// Variable types - maps variable name to its type
    variable_types: HashMap<String, Type>,
    /// Generated environment struct definitions (Phase 7c-δ)
    environment_structs: String,
    /// Variables that have been hoisted to environment (Phase 7c-δ)
    /// Maps variable name to (env_var_name, env_struct_type)
    hoisted_variables: HashMap<String, (String, String)>,
    /// Current environment variable name (if any)
    current_env_var: Option<String>,
    /// Env vars whose struct owns retained heap fields, mapped to the
    /// generated destructor function that releases them (env dies with the
    /// function value via hl_function_free)
    env_dtors: HashMap<String, String>,
    /// Method receiver type when generating method bodies
    method_receiver_type: Option<Type>,
    /// Whether we're currently inside a string switch (no break statements allowed)
    in_string_switch: bool,
    /// Whether we're currently inside an integer/bool switch case body:
    /// break there targets the C switch, not an enclosing loop (Phase 1.5b)
    in_c_switch: bool,
    /// Temp-owner frames of enclosing statements (Phase 1.5b): generate_statement
    /// pushes the outer statement's temp_owners here instead of a local, so
    /// return/break/continue can release temps whose statement-end cleanup
    /// they jump past. Phase 4a: the dead scope_depth component was dropped —
    /// a temp's release point is its owning statement's boundary, not a scope.
    enclosing_temp_frames: Vec<HashMap<String, HeapType>>,
    /// Stack of enclosing loops for break/continue cleanup (Phase 1.5b)
    loop_frames: Vec<LoopFrame>,
    /// Current iteration value variable name for for-in loops
    current_iter_value_name: Option<String>,
    /// Phase 8a: Ownership tracking for heap allocations
    /// Maps variable name to (heap_type, scope_depth) where heap_type is the type of heap allocation
    heap_owners: HashMap<String, (HeapType, usize)>,
    /// Phase 11a expression-temporary cleanup: Temporary heap values for statement-end cleanup.
    /// The per-statement release list — maps temp variable name to its heap type.
    /// Phase 4a: this is the ONE temporary-lifetime mechanism (the store-site
    /// release path, needs_site_release_after_store, was folded in and deleted);
    /// the dead scope_depth component was dropped.
    temp_owners: HashMap<String, HeapType>,
    /// Phase 11a expression-temporary cleanup: Counter for generating unique temporary variable names
    temp_counter: usize,
    /// Phase 11a expression-temporary cleanup: Pending C declarations to hoist to statement scope
    pending_statement_decls: Vec<String>,
    /// Phase 8a: Current scope depth for ownership tracking
    scope_depth: usize,
    /// Phase 8a: Variables that have had ownership transferred (don't free these)
    transferred_vars: HashSet<String>,
    /// Phase 8a: Whether we're currently generating the main program (for special return handling)
    in_main_program: bool,
    /// Phase 8a: Context for function expression generation (to detect escaping closures)
    function_expr_context: FunctionExprContext,
    /// Current function's return type for optional wrapping (Phase 9b)
    current_function_return_type: Option<Type>,
    /// Phase 9e: Generated tuple struct definitions
    tuple_struct_definitions: String,
    /// Phase 9e: Track which tuple types have been generated (to avoid duplicates)
    generated_tuple_types: HashSet<Vec<Type>>,
    /// Phase 11a-δ-α: Current module name mapping for cross-module calls
    current_name_map: Option<HashMap<String, String>>,
    /// Phase 11b: Forward declarations for exported functions and lets
    forward_declarations: String,
    /// Phase 11b-fixup: Track whether main() has explicitly returned (to avoid duplicated epilogue)
    main_explicitly_returned: bool,
    /// Phase 10-γ: emitted C function bodies for watchers, concatenated into
    /// final output between function definitions and main().
    watcher_bodies: String,
    /// Phase 10-γ: Counter for generating unique watcher IDs
    watcher_counter: usize,
    /// Phase 3d: program-body watcher ids, allocated in body-item order by
    /// generate_program_body_functions and consumed by position in
    /// generate_program_body_statements (the two run as separate passes).
    program_watcher_ids: Vec<usize>,
    /// Phase 3b: boxing analysis for the TopLevel currently being generated
    /// (set at the top of generate/generate_graph; per-module for graphs).
    boxing: Option<crate::typecheck::boxing::BoxingAnalysis>,
    /// Phase 3b: (name, scope_depth, boxed) for scalar bindings, innermost
    /// last; exit_scope pops its depth's entries. Read/assign sites consult
    /// the LAST entry for a name — a raw inner shadow of a boxed outer
    /// binding correctly reads raw.
    boxed_bindings: Vec<(String, usize, bool)>,
    /// Phase 3b: env-hoisted captures that are boxed scalar cells — reads
    /// emit hl_scalar_get_i32(env->x), writes emit hl_cell_set_i32(env->x, v).
    boxed_hoisted: HashSet<String>,
    /// Phase 3b: file-scope `static HiLowScalar*` declarations for boxed
    /// PROGRAM-scope variables — nested named functions subscribe/capture
    /// them by cell identity (emitted right after the includes).
    boxed_scalar_statics: String,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            var_counter: 0,
            function_counter: 0,
            generated_functions: String::new(),
            functions: HashMap::new(),
            variable_types: HashMap::new(),
            environment_structs: String::new(),
            hoisted_variables: HashMap::new(),
            current_env_var: None,
            method_receiver_type: None,
            in_string_switch: false,
            current_iter_value_name: None,
            in_c_switch: false,
            enclosing_temp_frames: Vec::new(),
            loop_frames: Vec::new(),
            env_dtors: HashMap::new(),
            heap_owners: HashMap::new(),
            temp_owners: HashMap::new(),
            temp_counter: 0,
            pending_statement_decls: Vec::new(),
            scope_depth: 0,
            transferred_vars: HashSet::new(),
            in_main_program: false,
            function_expr_context: FunctionExprContext::Normal,
            current_function_return_type: None,
            main_program_optionals: Vec::new(),
            tuple_struct_definitions: String::new(),
            generated_tuple_types: HashSet::new(),
            current_name_map: None,
            forward_declarations: String::new(),
            main_explicitly_returned: false,
            watcher_bodies: String::new(),
            watcher_counter: 0,
            program_watcher_ids: Vec::new(),
            boxing: None,
            boxed_bindings: Vec::new(),
            boxed_hoisted: HashSet::new(),
            boxed_scalar_statics: String::new(),
        }
    }

    /// Phase 3b: does the 3a analysis box the declaration at (name, pos)?
    fn is_boxed_decl(&self, name: &str, pos: &crate::lexer::Position) -> bool {
        self.boxing.as_ref().map_or(false, |b| b.is_boxed(name, pos))
    }

    /// Phase 3e-α: does this declaration need a SLOT cell (a subscription
    /// requires the variable itself be a HiLowScalar — (assigned) anywhere,
    /// or any subscription on a string)?
    fn needs_slot_decl(&self, name: &str, pos: &crate::lexer::Position) -> bool {
        self.boxing.as_ref().map_or(false, |b| b.needs_slot(name, pos))
    }

    /// Phase 3e-α: the slot payload accessor names for a slot-boxed
    /// variable of the given HiLow type: (constructor, getter, setter).
    fn slot_fns_for(ty: &Type) -> Option<(&'static str, &'static str, &'static str)> {
        match ty {
            Type::I32 => Some(("hl_scalar_new_i32", "hl_scalar_get_i32", "hl_cell_set_i32")),
            Type::String => Some(("hl_scalar_new_str", "hl_scalar_get_str", "hl_cell_set_str")),
            Type::DynamicArray(_) => Some(("hl_scalar_new_array_ref", "hl_scalar_get_array_ref", "hl_cell_set_array_ref")),
            Type::Object(_) => Some(("hl_scalar_new_object_ref", "hl_scalar_get_object_ref", "hl_cell_set_object_ref")),
            _ => None,
        }
    }

    /// Phase 3b: is the CURRENT binding of `name` a boxed HiLowScalar* cell?
    fn current_binding_boxed(&self, name: &str) -> bool {
        self.boxed_bindings
            .iter()
            .rev()
            .find(|(n, _, _)| n == name)
            .map_or(false, |(_, _, boxed)| *boxed)
    }

    /// Phase 3b: record the boxedness of a fresh binding at the current
    /// scope depth (raw bindings are recorded too, so an inner raw shadow
    /// masks an outer boxed one).
    fn push_boxed_binding(&mut self, name: &str, boxed: bool) {
        self.boxed_bindings
            .push((name.to_string(), self.scope_depth, boxed));
    }

    /// Phase 3b: the C lvalue of a cell-valued variable at the current
    /// emission site — through the enclosing watcher env when hoisted,
    /// otherwise the (possibly static-hoisted) C name.
    fn env_slot_rvalue(&self, name: &str) -> String {
        if let Some((env_var, _)) = self.hoisted_variables.get(name) {
            format!("{}->{}", env_var, name)
        } else {
            self.mangle_variable_name(name)
        }
    }

    /// Phase 3b: emit a watcher env struct + its cell-releasing destructor
    /// into environment_structs; returns the destructor's name. Every env
    /// slot OWNS a retain on its cell (escape soundness — audit §5 item 1);
    /// the dtor releases them, the runtime frees the env itself.
    fn emit_watcher_env_struct(&mut self, env_struct_name: &str, fields: &[(String, EnvSlot)]) -> String {
        self.environment_structs.push_str(&format!("typedef struct {} {{\n", env_struct_name));
        for (name, slot) in fields {
            self.environment_structs.push_str(&format!("    {} {};\n", slot.c_type(), name));
        }
        self.environment_structs.push_str(&format!("}} {};\n\n", env_struct_name));

        let dtor_name = format!("{}_dtor", env_struct_name);
        self.environment_structs.push_str(&format!("static void {}(void* raw) {{\n", dtor_name));
        self.environment_structs.push_str(&format!("    {}* e = ({}*)raw;\n", env_struct_name, env_struct_name));
        for (name, slot) in fields {
            self.environment_structs.push_str(&format!("    {}(e->{});\n", slot.release_fn(), name));
        }
        self.environment_structs.push_str("}\n\n");
        dtor_name
    }

    /// Phase 3b: build the env allocation + packing declaration (hoisted to
    /// statement scope by the caller). Each slot retains its cell.
    fn watcher_env_pack_decl(&self, env_struct_name: &str, env_var: &str, fields: &[(String, EnvSlot)]) -> String {
        let mut decl = format!(
            "{}* {} = malloc(sizeof({})); hl_alloc_count++;",
            env_struct_name, env_var, env_struct_name
        );
        for (name, slot) in fields {
            let rvalue = self.env_slot_rvalue(name);
            decl.push_str(&format!(
                " {}->{} = {}; {}({}->{});",
                env_var, name, rvalue,
                slot.retain_fn(), env_var, name
            ));
        }
        decl
    }

    /// Generate C code for the entire program
    pub fn generate(&mut self, top_level: &TopLevel, type_checker: &TypeChecker) -> Result<String, CodegenError> {
        // Phase 3b: run the boxing analysis (Phase 3a) — the lowering below
        // consults it at every scalar declaration, read, and assignment.
        self.boxing = Some(crate::typecheck::boxing::analyze(top_level));

        // Build the final output in the correct order:
        // 1. Includes
        // 2. Environment struct definitions (from closures)
        // 3. Generated functions (from function expressions)
        // 4. Main program code

        let mut final_output = String::new();

        // Add standard C includes first
        final_output.push_str("#include <stdint.h>\n");
        final_output.push_str("#include <stdbool.h>\n");
        final_output.push_str("#include \"runtime.h\"\n");
        final_output.push_str("\n");

        match top_level {
            TopLevel::Program(program) => {
                // Phase 11a-α: defensive guard for imports
                if !program.imports.is_empty() {
                    return Err(CodegenError::UnsupportedFeature {
                        feature: "imports".to_string(),
                        phase: "not yet implemented in Phase 11a-α".to_string(),
                    });
                }
                self.generate_program(program, type_checker)?;
            }
            TopLevel::Module(_module) => {
                // Phase 11a-α: defensive guard for modules
                return Err(CodegenError::UnsupportedFeature {
                    feature: "modules".to_string(),
                    phase: "not yet implemented in Phase 11a-α".to_string(),
                });
            }
        }

        // Phase 3b: boxed program-scope scalar cells (file-scope statics,
        // referenced by nested functions and watcher envs below)
        final_output.push_str(&self.boxed_scalar_statics);

        // Add environment struct definitions (Phase 7c-δ)
        final_output.push_str(&self.environment_structs);

        // Add tuple struct definitions (Phase 9e)
        final_output.push_str(&self.tuple_struct_definitions);

        // Add generated functions (from function expressions)
        final_output.push_str(&self.generated_functions);

        // Add watcher bodies (Phase 10-γ)
        final_output.push_str(&self.watcher_bodies);

        // Add main program code
        final_output.push_str(&self.output);

        Ok(final_output)
    }

    /// Generate C code for a resolved graph (Phase 11a-δ-α)
    pub fn generate_graph(
        &mut self,
        graph: &crate::resolver::ResolvedGraph,
        type_checker: &TypeChecker,
        entry_abs_path: &std::path::Path,
    ) -> Result<String, CodegenError> {
        let mut final_output = String::new();

        // Add standard C includes first
        final_output.push_str("#include <stdint.h>\n");
        final_output.push_str("#include <stdbool.h>\n");
        final_output.push_str("#include \"runtime.h\"\n");
        final_output.push_str("\n");

        let entry_dir = entry_abs_path.parent().unwrap();

        // Process modules in topological order (dependencies first)
        for abs_path in &graph.topo_order {
            let parsed_file = graph.files.get(abs_path).unwrap();

            // Build per-module name map and populate variable types for imports
            let name_map = self.build_name_map_for_module(abs_path, parsed_file, &graph, entry_dir);

            // For each import in this module, look up types from typecheck's module_exports
            for import in parsed_file.imports() {
                if let Some(export_table) = type_checker.module_exports().get(&import.path) {
                    for imported_name in &import.names {
                        if let Some(ty) = export_table.get(imported_name) {
                            self.variable_types.insert(imported_name.clone(), ty.clone());
                        }
                    }
                }
            }

            self.current_name_map = Some(name_map);

            // Phase 3b: per-module boxing analysis (positions are
            // module-local, so the analysis must be too).
            self.boxing = Some(crate::typecheck::boxing::analyze(parsed_file));

            match parsed_file {
                TopLevel::Module(module) => {
                    // Phase 3c: module-level watchers are rejected by
                    // typecheck (initialization semantics unspecified), so
                    // none can reach codegen.
                    debug_assert!(module.watchers.is_empty());

                    // Generate forward declarations and exported functions with mangled names
                    for func in &module.items {
                        if func.is_export {
                            let mangled_name = self.mangle_module_function_name(abs_path, &func.name, entry_dir);

                            // Add forward declaration
                            let return_c_type = self.hilow_type_to_c(&Type::from_ast_type(&func.return_type));
                            let mut forward_decl = format!("{} {}(", return_c_type, mangled_name);

                            for (i, param) in func.params.iter().enumerate() {
                                if i > 0 {
                                    forward_decl.push_str(", ");
                                }
                                let param_c_type = self.hilow_type_to_c(&Type::from_ast_type(&param.ty));
                                forward_decl.push_str(&format!("{} {}", param_c_type, param.name));
                            }

                            forward_decl.push_str(");\n");
                            self.forward_declarations.push_str(&forward_decl);

                            self.generate_module_function(func, &mangled_name, type_checker)?;
                        }
                    }

                    // Generate forward declarations and exported lets with mangled names
                    for let_decl in &module.lets {
                        if let_decl.is_export {
                            if let LetPattern::Identifier(var_name, type_annotation) = &let_decl.pattern {
                                let mangled_name = self.mangle_module_let_name(abs_path, var_name, entry_dir);

                                // Add extern declaration for the let
                                if let Some(annotation) = type_annotation {
                                    let let_c_type = self.hilow_type_to_c(&Type::from_ast_type(annotation));
                                    self.forward_declarations.push_str(&format!("extern {} {};\n", let_c_type, mangled_name));
                                }

                                self.generate_module_let(let_decl, &mangled_name, type_checker)?;
                            }
                        }
                    }
                }
                TopLevel::Program(program) => {
                    // This should be the entry program - process last
                    if abs_path == &entry_abs_path.to_string_lossy().to_string() {
                        // Generate the main program
                        self.generate_main_function(program, type_checker)?;
                    }
                }
            }
        }

        self.current_name_map = None;

        // Build the final output
        // Add forward declarations for exported functions and lets (Phase 11b)
        final_output.push_str(&self.forward_declarations);
        final_output.push_str("\n");

        // Phase 3b: boxed program-scope scalar cells (file-scope statics)
        final_output.push_str(&self.boxed_scalar_statics);

        // Add tuple struct definitions (Phase 9e)
        final_output.push_str(&self.tuple_struct_definitions);

        // Add environment struct definitions (from closures)
        final_output.push_str(&self.environment_structs);

        // Add generated functions (from function expressions and modules)
        final_output.push_str(&self.generated_functions);

        // Add watcher bodies (Phase 10-γ)
        final_output.push_str(&self.watcher_bodies);

        // Add main program code
        final_output.push_str(&self.output);

        Ok(final_output)
    }

    /// Build name mapping for a module during graph codegen (Phase 11a-δ-α)
    fn build_name_map_for_module(
        &self,
        module_path: &str,
        parsed_file: &TopLevel,
        graph: &crate::resolver::ResolvedGraph,
        entry_dir: &std::path::Path,
    ) -> HashMap<String, String> {
        let mut name_map = HashMap::new();

        // Add this module's own declarations (exported and private)
        match parsed_file {
            TopLevel::Module(module) => {
                // Add exported functions
                for func in &module.items {
                    if func.is_export {
                        let mangled_name = self.mangle_module_function_name(module_path, &func.name, entry_dir);
                        name_map.insert(func.name.clone(), mangled_name);
                    }
                }

                // Add exported lets
                for let_decl in &module.lets {
                    if let_decl.is_export {
                        if let LetPattern::Identifier(var_name, _) = &let_decl.pattern {
                            let mangled_name = self.mangle_module_let_name(module_path, var_name, entry_dir);
                            name_map.insert(var_name.clone(), mangled_name);
                        }
                    }
                }
            }
            TopLevel::Program(_) => {
                // Programs don't have exportable declarations in the same way
            }
        }

        // Add imports from other modules
        for import in parsed_file.imports() {
            let imported_module_path = &import.path;
            if let Some(imported_file) = graph.files.get(imported_module_path) {
                if let TopLevel::Module(imported_module) = imported_file {
                    for imported_name in &import.names {
                        // Check functions
                        for func in &imported_module.items {
                            if func.is_export && func.name == *imported_name {
                                let mangled_name = self.mangle_module_function_name(imported_module_path, &func.name, entry_dir);
                                name_map.insert(imported_name.clone(), mangled_name);
                            }
                        }

                        // Check lets
                        for let_decl in &imported_module.lets {
                            if let_decl.is_export {
                                if let LetPattern::Identifier(var_name, _) = &let_decl.pattern {
                                    if var_name == imported_name {
                                        let mangled_name = self.mangle_module_let_name(imported_module_path, var_name, entry_dir);
                                        name_map.insert(imported_name.clone(), mangled_name);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        name_map
    }

    /// Mangle function name for module (Phase 11a-δ-α)
    fn mangle_module_function_name(&self, abs_path: &str, func_name: &str, entry_dir: &std::path::Path) -> String {
        let rel_path = self.get_relative_path(abs_path, entry_dir);
        format!("{}__{}",
            rel_path.replace("/", "_").replace(".hl", ""),
            func_name
        )
    }

    /// Mangle let name for module (Phase 11a-δ-α)
    fn mangle_module_let_name(&self, abs_path: &str, let_name: &str, entry_dir: &std::path::Path) -> String {
        let rel_path = self.get_relative_path(abs_path, entry_dir);
        format!("{}__{}",
            rel_path.replace("/", "_").replace(".hl", ""),
            let_name
        )
    }

    /// Get relative path from absolute path given entry directory
    fn get_relative_path(&self, abs_path: &str, entry_dir: &std::path::Path) -> String {
        let abs_path_obj = std::path::Path::new(abs_path);
        abs_path_obj.strip_prefix(entry_dir)
            .unwrap_or(abs_path_obj)
            .to_string_lossy()
            .to_string()
    }

    /// Generate a module function with mangled name (Phase 11a-δ-α)
    fn generate_module_function(&mut self, func: &Function, mangled_name: &str, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Generate function signature
        let return_type = self.hilow_type_to_c(&Type::from_ast_type(&func.return_type));
        self.generated_functions.push_str(&return_type);
        self.generated_functions.push_str(" ");
        self.generated_functions.push_str(mangled_name);
        self.generated_functions.push_str("(");

        // Add parameters
        for (i, param) in func.params.iter().enumerate() {
            if i > 0 {
                self.generated_functions.push_str(", ");
            }
            self.generated_functions.push_str(&self.hilow_type_to_c(&Type::from_ast_type(&param.ty)));
            self.generated_functions.push_str(" ");
            self.generated_functions.push_str(&param.name);
        }

        self.generated_functions.push_str(") {\n");

        // Store current output and switch to generated_functions
        let saved_output = self.output.clone();
        self.output.clear();

        // Generate function body
        if let Some(ref body) = func.body {
            self.generate_block_with_parameter_context(body, &func.params, type_checker)?;
        }

        // Move generated body to generated_functions
        self.generated_functions.push_str(&self.output);
        self.generated_functions.push_str("}\n\n");

        // Restore output
        self.output = saved_output;

        Ok(())
    }

    /// Generate a module let with mangled name (Phase 11a-δ-α)
    fn generate_module_let(&mut self, let_decl: &LetDecl, mangled_name: &str, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        if let Some(ref initializer) = let_decl.initializer {
            // For now, assume simple name pattern (not tuple destructuring)
            if let LetPattern::Identifier(var_name, type_annotation) = &let_decl.pattern {
                // Use type annotation if provided, otherwise infer from initializer
                let hilow_type = if let Some(ref annotation) = type_annotation {
                    Type::from_ast_type(annotation)
                } else {
                    // Infer type from the literal if it's a simple literal
                    match initializer {
                        Expression::IntLit(_, _) => crate::types::Type::I32,
                        Expression::StringLit(_, _) => crate::types::Type::String,
                        Expression::BoolLit(_, _) => crate::types::Type::Bool,
                        _ => crate::types::Type::I32, // Default fallback
                    }
                };

                let c_type = self.hilow_type_to_c(&hilow_type);
                self.generated_functions.push_str(&c_type);
                self.generated_functions.push_str(" ");
                self.generated_functions.push_str(mangled_name);
                self.generated_functions.push_str(" = ");

                // Store current output and switch to generated_functions
                let saved_output = self.output.clone();
                self.output.clear();

                // Generate initializer
                self.generate_expression(initializer, type_checker, ExprContext::Temporary)?;

                // Move generated initializer to generated_functions
                self.generated_functions.push_str(&self.output);
                self.generated_functions.push_str(";\n");

                // Restore output
                self.output = saved_output;
            }
        }

        Ok(())
    }

    /// Generate main function from program (Phase 11a-δ-α)
    fn generate_main_function(&mut self, program: &Program, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        self.emit_main_function(program, type_checker)
    }


    fn emit_includes(&mut self) {
        self.output.push_str("#include <stdint.h>\n");
        self.output.push_str("#include <stdbool.h>\n");
        self.output.push_str("#include \"runtime.h\"\n");
        self.output.push_str("\n");
    }

    fn generate_program(&mut self, program: &Program, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Phase 6a-fixup: Generate nested functions first, then main function
        if let Some(body) = &program.body {
            self.generate_program_body_functions(body, type_checker)?;
        }

        // Generate the main function using consolidated helper
        self.emit_main_function(program, type_checker)
    }

    fn generate_module(&mut self, module: &Module, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Generate each function in the module
        for function in &module.items {
            self.generate_function(function, type_checker)?;
        }
        Ok(())
    }

    fn generate_function(&mut self, function: &Function, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Convert return type to C
        let c_return_type = self.hilow_type_to_c(&Type::from_ast_type(&function.return_type));

        // Generate function signature with mangled name to avoid C keyword conflicts
        let c_func_name = self.mangle_function_name(&function.name);
        self.output.push_str(&format!("{} {}(", c_return_type, c_func_name));

        // Generate parameters and track their types. Phase 3b: a parameter
        // the boxing analysis marks arrives raw and is boxed in the
        // prologue — the name rebinds to the cell for the whole body.
        let boxed_param_base = self.boxed_bindings.len();
        let mut boxed_params: Vec<String> = Vec::new();
        for (i, param) in function.params.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            let param_type = Type::from_ast_type(&param.ty);
            // Phase 3e-α: a parameter that needs a SLOT cell (a reference-
            // typed (assigned)/string subscription targets it) has no
            // boxing prologue yet — reject cleanly rather than silently
            // subscribing an event the value cell never emits.
            if self.needs_slot_decl(&param.name, &param.position)
                && !matches!(param_type, Type::I32)
            {
                return Err(CodegenError::UnsupportedFeature {
                    feature: format!(
                        "subscribing parameter '{}' of type {:?} through a variable slot",
                        param.name, param_type
                    ),
                    phase: "a future phase — reference-typed parameter slots (Phase 3e-α boxes locals and program-scope lets only)".to_string(),
                });
            }
            let boxed = self.is_boxed_decl(&param.name, &param.position)
                && matches!(param_type, Type::I32);
            let c_type = self.hilow_type_to_c(&param_type);
            if boxed {
                self.output.push_str(&format!("{} {}__raw", c_type, param.name));
                boxed_params.push(param.name.clone());
            } else {
                self.output.push_str(&format!("{} {}", c_type, param.name));
            }

            // Track parameter types for capture analysis
            self.variable_types.insert(param.name.clone(), param_type);
            self.push_boxed_binding(&param.name, boxed);
        }

        self.output.push_str(") {\n");

        for name in &boxed_params {
            let c_var = self.mangle_variable_name(name);
            self.output.push_str(&format!("  HiLowScalar* {} = hl_scalar_new_i32({}__raw);\n", c_var, name));
        }

        // Set current function return type for optional handling
        let return_type = Type::from_ast_type(&function.return_type);
        self.current_function_return_type = Some(return_type);

        // Phase 10-δ-γ-fixup: a function's ownership transfers are local to it.
        // transferred_vars is keyed by name only, so without this save/restore a
        // function returning a heap variable (e.g. `return w`) marks the name "w"
        // transferred, and a same-named variable in the caller would then be wrongly
        // skipped during cleanup, leaking memory. Save and restore around the body.
        let saved_transferred = std::mem::take(&mut self.transferred_vars);

        // Heap ownership is also local to a function. Without this save/restore, a nested
        // function's heap-owned locals (e.g. `let xs = ...; return xs`) pollute the caller's
        // heap_owners map, so the caller's scope-cleanup tries to release the callee's locals
        // (generating `hl_array_release(xs);` in main where xs is undeclared → C compile error).
        // Same family as the transferred_vars fix above.
        let saved_heap_owners = std::mem::take(&mut self.heap_owners);

        // Phase 3b: the prologue-boxed parameter cells belong to the body
        // scope (entered by generate_block_with_parameter_context below) and
        // release on every exit path like any boxed local.
        for name in &boxed_params {
            self.heap_owners.insert(name.clone(), (HeapType::Scalar, self.scope_depth + 1));
        }

        // Phase 1.5b: control-transfer context is per-C-function — a return/
        // break inside this body must not see the enclosing function's
        // statement temps or loops
        let saved_temp_frames = std::mem::take(&mut self.enclosing_temp_frames);
        let saved_loop_frames = std::mem::take(&mut self.loop_frames);
        let saved_temp_owners = std::mem::take(&mut self.temp_owners);
        let saved_pending_decls = std::mem::take(&mut self.pending_statement_decls);
        let saved_in_c_switch = self.in_c_switch;
        let saved_in_string_switch = self.in_string_switch;
        self.in_c_switch = false;
        self.in_string_switch = false;

        // Phase 3d: the watcher-subscriber shadow masking that lived here is
        // gone — subscription is by cell identity, so shadowing is trivially
        // correct with no name bookkeeping.

        // Generate function body
        if let Some(body) = &function.body {
            self.generate_block_with_parameter_context(body, &function.params, type_checker)?;
        }

        // Restore caller's ownership-transfer state
        self.transferred_vars = saved_transferred;
        // Restore caller's heap-ownership state
        self.heap_owners = saved_heap_owners;
        self.enclosing_temp_frames = saved_temp_frames;
        self.loop_frames = saved_loop_frames;
        self.temp_owners = saved_temp_owners;
        self.pending_statement_decls = saved_pending_decls;
        self.in_c_switch = saved_in_c_switch;
        self.in_string_switch = saved_in_string_switch;

        // Phase 3b: drop this function's parameter binding records
        self.boxed_bindings.truncate(boxed_param_base);

        // Clear function return type context
        self.current_function_return_type = None;

        self.output.push_str("}\n\n");
        Ok(())
    }

    fn generate_block(&mut self, block: &Block, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Phase 8a: Enter new scope for ownership tracking
        self.enter_scope();

        // Set up environment for captured locals (Phase 7c-δ)
        self.setup_environment_for_block(block)?;

        // Phase 1: register nested function signatures (for forward calls within block)
        for item in &block.items {
            if let BlockItem::Function(f) = item {
                // Register f's signature in self.functions
                let return_type = Type::from_ast_type(&f.return_type);
                self.functions.insert(f.name.clone(), return_type);
            }
        }

        // Phase 2: emit nested function bodies (as top-level C functions)
        for item in &block.items {
            if let BlockItem::Function(f) = item {
                self.generate_function(f, type_checker)?;
            }
        }

        // Phase 3: allocate watcher ids up front, in item order (Phase 3d:
        // ids are carried by position, not by name — pass 4 consumes them in
        // the same order this loop allocates them)
        let mut block_watcher_ids: Vec<usize> = Vec::new();
        for item in &block.items {
            if let BlockItem::Watcher(_) = item {
                block_watcher_ids.push(self.watcher_counter);
                self.watcher_counter += 1;
            }
        }

        // Phase 4: emit statements and watcher constructions in source order
        let mut next_watcher_idx = 0;
        for item in &block.items {
            match item {
                BlockItem::Statement(s) => {
                    self.generate_statement(s, type_checker)?;
                }
                BlockItem::Watcher(w) => {
                    // Phase 3b: body into watcher_bodies, then cell
                    // subscription at the declaration site
                    let id = block_watcher_ids[next_watcher_idx];
                    next_watcher_idx += 1;
                    let (_func_name, env_fields, env_dtor) = self.generate_watcher(w, id, type_checker)?;
                    self.emit_decl_watcher_construction(w, id, &env_fields, env_dtor.as_deref());
                }
                BlockItem::Function(_) => {
                    // Function already emitted in Phase 2
                }
            }
        }

        // Phase 3c: no scope-exit deactivation emission — the scope-owned
        // watcher reference is released by the heap_owners cleanup (which
        // also reaches early returns).

        // Phase 8a: Exit scope and emit cleanup
        self.exit_scope();
        Ok(())
    }

    fn generate_block_with_parameter_context(&mut self, block: &Block, params: &[Parameter], type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Phase 8a: Enter new scope for ownership tracking
        self.enter_scope();

        // Set up environment for captured locals (Phase 7c-δ)
        self.setup_environment_for_block_with_params(block, params)?;

        // Phase 1: register nested function signatures (for forward calls within block)
        for item in &block.items {
            if let BlockItem::Function(f) = item {
                // Register f's signature in self.functions
                let return_type = Type::from_ast_type(&f.return_type);
                self.functions.insert(f.name.clone(), return_type);
            }
        }

        // Phase 2: emit nested function bodies (as top-level C functions)
        for item in &block.items {
            if let BlockItem::Function(f) = item {
                self.generate_function(f, type_checker)?;
            }
        }

        // Phase 3: allocate watcher ids up front, in item order (Phase 3d:
        // ids are carried by position, not by name — pass 4 consumes them in
        // the same order this loop allocates them)
        let mut block_watcher_ids: Vec<usize> = Vec::new();
        for item in &block.items {
            if let BlockItem::Watcher(_) = item {
                block_watcher_ids.push(self.watcher_counter);
                self.watcher_counter += 1;
            }
        }

        // Phase 4: emit statements and watcher constructions in source order
        let mut next_watcher_idx = 0;
        for item in &block.items {
            match item {
                BlockItem::Statement(s) => {
                    self.generate_statement(s, type_checker)?;
                }
                BlockItem::Watcher(w) => {
                    // Phase 3b: body into watcher_bodies, then cell
                    // subscription at the declaration site
                    let id = block_watcher_ids[next_watcher_idx];
                    next_watcher_idx += 1;
                    let (_func_name, env_fields, env_dtor) = self.generate_watcher(w, id, type_checker)?;
                    self.emit_decl_watcher_construction(w, id, &env_fields, env_dtor.as_deref());
                }
                BlockItem::Function(_) => {
                    // Function already emitted in Phase 2
                }
            }
        }

        // Phase 3c: no scope-exit deactivation emission — the scope-owned
        // watcher reference is released by the heap_owners cleanup (which
        // also reaches early returns).

        // Phase 8a: Exit scope and emit cleanup
        self.exit_scope();
        Ok(())
    }
    fn generate_program_body_functions(&mut self, body: &ProgramBody, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // First, track function signatures for later reference
        let mut deferred_watchers = Vec::new();
        for item in &body.items {
            if let BlockItem::Function(function) = item {
                let return_type = Type::from_ast_type(&function.return_type);
                // Store function return types in functions map instead of variable_types
                // to distinguish named functions from function value variables
                self.functions.insert(function.name.clone(), return_type);
            } else if let BlockItem::Watcher(watcher) = item {
                // Phase 10-γ: Defer watcher generation until after variables are processed
                deferred_watchers.push(watcher);
            }
            // BlockItem::Statement handled in the second pass; no action here
        }

        // Process variable declarations first to populate variable_types
        // (nested functions and watcher bodies generated before the main
        // statements reference them). Phase 3b: pre-record boxedness too, so
        // a nested function subscribing a program-scope scalar resolves it
        // as the (static-hoisted) cell.
        for item in &body.items {
            if let BlockItem::Statement(Statement::Let(let_decl)) = item {
                // Add variable type to variable_types so watchers can reference it
                match &let_decl.pattern {
                    LetPattern::Identifier(name, Some(ty)) => {
                        let hilow_type = Type::from_ast_type(ty);
                        self.variable_types.insert(name.clone(), hilow_type);
                        let boxed = self.is_boxed_decl(name, &let_decl.position);
                        self.push_boxed_binding(name, boxed);
                    }
                    LetPattern::Identifier(name, None) => {
                        // Type inference case - get from type checker
                        if let Some(init) = &let_decl.initializer {
                            let mut inferred_type = type_checker.get_expression_type(init);
                            // Phase 3e-β: the typechecker's expression map
                            // does not record literal container types. A
                            // slot-boxed variable must carry its real type
                            // here — nested functions key the slot setter on
                            // it (hl_cell_set_array_ref vs _object_ref).
                            if matches!(inferred_type, Type::Unknown)
                                && self.needs_slot_decl(name, &let_decl.position)
                            {
                                inferred_type = self.infer_expression_type_for_codegen(init);
                            }
                            self.variable_types.insert(name.clone(), inferred_type);
                        }
                        let boxed = self.is_boxed_decl(name, &let_decl.position);
                        self.push_boxed_binding(name, boxed);
                    }
                    _ => {} // Tuple patterns handled elsewhere
                }
            }
        }

        // Phase 3d: allocate program-body watcher ids here, BEFORE nested
        // functions generate (so program watchers keep lower ids than
        // nested-function watchers, as before). Ids are carried by position
        // in program_watcher_ids; generate_program_body_statements consumes
        // them in the same body-item order. Bodies and constructions are
        // emitted at the declaration site so captures see earlier lets and
        // pre-declaration assignments cannot fire.
        self.program_watcher_ids.clear();
        for _watcher in deferred_watchers {
            self.program_watcher_ids.push(self.watcher_counter);
            self.watcher_counter += 1;
        }

        // Generate nested functions as top-level C functions
        for item in &body.items {
            if let BlockItem::Function(function) = item {
                self.generate_function(function, type_checker)?;
            }
        }

        Ok(())
    }

    fn generate_program_body_statements(&mut self, body: &ProgramBody, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Create a synthetic block from the program body statements for capture analysis
        let statements: Vec<Statement> = body.items.iter()
            .filter_map(|item| {
                if let BlockItem::Statement(stmt) = item {
                    Some(stmt.clone())
                } else {
                    None
                }
            })
            .collect();
        let synthetic_block = Block {
            items: statements.into_iter().map(|s| BlockItem::Statement(s)).collect(),
            position: Position { line: 0, column: 0 }
        };

        // Set up environment for captured locals (Phase 7c-δ)
        self.setup_environment_for_block(&synthetic_block)?;

        // Generate statements and watcher constructions in source order
        // (Phase 3b: decl-form watchers subscribe at their declaration site)
        let mut next_watcher_idx = 0;
        for item in &body.items {
            match item {
                BlockItem::Statement(statement) => {
                    self.generate_statement(statement, type_checker)?;
                }
                BlockItem::Watcher(w) => {
                    // Phase 3d: ids carried by position from the
                    // generate_program_body_functions pre-pass.
                    let id = self.program_watcher_ids[next_watcher_idx];
                    next_watcher_idx += 1;
                    let (_func_name, env_fields, env_dtor) = self.generate_watcher(w, id, type_checker)?;
                    self.emit_decl_watcher_construction(w, id, &env_fields, env_dtor.as_deref());
                }
                BlockItem::Function(_) => {}
            }
        }
        Ok(())
    }

    fn generate_statement(&mut self, statement: &Statement, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Phase 11a expression-temporary cleanup: Buffer the statement body, then emit declarations first
        let saved_output = std::mem::take(&mut self.output);

        // Re-entrancy: nested generate_statement calls (function-expression bodies,
        // match block arms, switch case bodies) must not flush the enclosing
        // statement's pending decls or release its temps. The outer temps go
        // onto enclosing_temp_frames (not a local) so return/break/continue
        // can release them when jumping past the outer statement's cleanup.
        let saved_pending_decls = std::mem::take(&mut self.pending_statement_decls);
        self.enclosing_temp_frames.push(std::mem::take(&mut self.temp_owners));

        match statement {
            Statement::Let(let_decl) => {
                self.generate_let_statement(let_decl, type_checker)?;
            }
            Statement::Return(return_stmt) => {
                self.generate_return_statement(return_stmt, type_checker)?;
            }
            Statement::ExprStatement(expr) => {
                // pop/remove move the element's reference out of the array;
                // a discarded result must be released here (Phase 1.5c: every
                // removal releases exactly once)
                let discard_release: Option<(&str, &str)> = match expr {
                    Expression::Call(call) => {
                        if let Expression::MemberAccess(member_access) = call.callee.as_ref() {
                            let object_type = self.infer_expression_type_for_codegen(&member_access.object);
                            if let Type::DynamicArray(elem_type) = object_type {
                                if matches!(member_access.member.as_str(), "pop" | "remove") {
                                    match *elem_type {
                                        Type::Object(_) => Some(("HiLowObject*", "hl_object_release")),
                                        Type::DynamicArray(_) => Some(("HiLowArray*", "hl_array_release")),
                                        _ => None,
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some((c_type, release_fn)) = discard_release {
                    let temp_var = format!("temp_{}", self.var_counter);
                    self.var_counter += 1;
                    self.output.push_str(&format!("  {{ {} {} = ", c_type, temp_var));
                    self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(&format!("; {}({}); }}\n", release_fn, temp_var));
                } else {
                    // Normal expression statement
                    self.output.push_str("  ");
                    self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(";\n");
                }
            }
            Statement::If(if_stmt) => {
                self.generate_if_statement(if_stmt, type_checker)?;
            }
            Statement::While(while_stmt) => {
                self.generate_while_statement(while_stmt, type_checker)?;
            }
            Statement::Loop(loop_stmt) => {
                self.generate_loop_statement(loop_stmt, type_checker)?;
            }
            Statement::ForIn(for_in_stmt) => {
                self.generate_for_in_statement(for_in_stmt, type_checker)?;
            }
            Statement::Switch(switch_stmt) => {
                self.generate_switch_statement(switch_stmt, type_checker)?;
            }
            Statement::Break(_) => {
                if self.in_string_switch {
                    // String switches lower to an if/else chain: HiLow's
                    // case-break is a no-op there. Emitting scope cleanup here
                    // released enclosing-scope owners mid-flow and scope exit
                    // released them again (double release — caught by the
                    // valgrind gate on switch_string).
                } else if self.in_c_switch {
                    // C-switch break exits only the switch; execution
                    // continues after it, so no scope cleanup
                    self.output.push_str("  break;\n");
                } else {
                    // Phase 1.5b: a loop-break jumps past the cleanups of the
                    // statements it exits (enclosing temps since loop entry)
                    // and past the loop-bottom extra cleanups (e.g. the
                    // for-in key).
                    if let Some(frame) = self.loop_frames.last().cloned() {
                        // +1 skips the frame holding the loop statement's own
                        // temps — break lands after the loop, where the loop
                        // statement's end-of-statement cleanup still runs
                        self.emit_enclosing_temp_releases(frame.temp_frame_base + 1);
                        for cleanup in &frame.extra_cleanups {
                            self.output.push_str(&format!("  {}\n", cleanup));
                        }
                    }

                    // Phase 8a: release loop-body-scoped heap owners the
                    // break jumps past
                    self.emit_early_return_cleanup(self.scope_depth);

                    self.output.push_str("  break;\n");
                }
            }
            Statement::Continue(_) => {
                // Phase 1.5b: continue always targets the innermost loop —
                // release enclosing temps since loop entry and re-emit the
                // loop-bottom cleanups it skips (recreated next iteration)
                if let Some(frame) = self.loop_frames.last().cloned() {
                    // +1 skips the loop statement's own temps frame — the loop
                    // is still running and releases them at its statement end
                    self.emit_enclosing_temp_releases(frame.temp_frame_base + 1);
                    for cleanup in &frame.extra_cleanups {
                        self.output.push_str(&format!("  {}\n", cleanup));
                    }
                }

                // Phase 8a: Emit cleanup before continue
                self.emit_early_return_cleanup(self.scope_depth);

                self.output.push_str("  continue;\n");
            }
            Statement::Assign(assign_stmt) => {
                self.generate_assign_statement(assign_stmt, type_checker)?;
            }
            Statement::QualifiedOp(qualified_op) => {
                self.generate_qualified_op_statement(qualified_op, type_checker)?;
            }
            Statement::StealthBlock(block, position) => {
                self.generate_stealth_block(block, position, type_checker)?;
            }
        }

        // Phase 11a expression-temporary cleanup: Emit in correct order - declarations first, then statement body, then cleanup
        let statement_body = std::mem::take(&mut self.output);
        self.output = saved_output;

        // Emit pending declarations first (temp variables need to be declared before use)
        for decl in &self.pending_statement_decls {
            self.output.push_str("  ");
            self.output.push_str(decl);
            self.output.push_str("\n");
        }
        self.pending_statement_decls.clear();

        // Emit the statement body that uses the temps
        self.output.push_str(&statement_body);

        // Emit cleanup for all temps used in this statement
        self.emit_temp_cleanup();

        // Restore the enclosing statement's pending decls and temps
        self.pending_statement_decls = saved_pending_decls;
        self.temp_owners = self.enclosing_temp_frames.pop()
            .expect("enclosing_temp_frames underflow: unbalanced generate_statement");

        Ok(())
    }

    fn generate_let_statement(&mut self, let_decl: &LetDecl, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        match &let_decl.pattern {
            LetPattern::Identifier(name, ty) => {
                self.generate_identifier_let_statement(name, ty.as_ref(), &let_decl.initializer, &let_decl.position, type_checker)
            },
            LetPattern::Tuple(names) => {
                // Phase 3b: watching a destructuring binding is unscheduled —
                // reject rather than miscompile (adjudication E).
                for name in names {
                    if self.is_boxed_decl(name, &let_decl.position) {
                        return Err(CodegenError::UnsupportedFeature {
                            feature: format!("watching or capturing the tuple-destructured binding '{}'", name),
                            phase: "a future phase — destructured bindings do not box yet (Phase 3b boxes plain let/param declarations)".to_string(),
                        });
                    }
                }
                self.generate_tuple_let_statement(names, &let_decl.initializer, type_checker)
            }
        }
    }

    fn generate_identifier_let_statement(&mut self, name: &str, ty: Option<&crate::ast::Type>, initializer: &Option<Expression>, position: &crate::lexer::Position, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Determine the type
        let var_type = if let Some(ty) = ty {
            Type::from_ast_type(ty)
        } else if let Some(ref initializer) = initializer {
            // Type inference - need to handle object literals properly
            match initializer {
                Expression::IntLit(value, _) => Type::default_integer_type(*value),
                Expression::FloatLit(_, _) => Type::default_float_type(),
                Expression::StringLit(_, _) => Type::String,
                Expression::DurationLit(_, _, _) => Type::Duration,
                Expression::MoneyLit(_, currency, _) => Type::MoneyOf(currency.clone()),
                Expression::BoolLit(_, _) => Type::Bool,
                Expression::ObjectLiteral(obj_lit) => {
                    // Infer object type from literal
                    let mut properties = Vec::new();
                    for (prop_name, prop_expr) in &obj_lit.properties {
                        let prop_type = self.infer_expression_type_for_codegen(prop_expr);
                        // Phase 1.5e: a weak slot reads back as referent-or-unknown
                        let prop_type = if matches!(prop_expr, Expression::WeakRef(_, _))
                            && matches!(prop_type, Type::Object(_)) {
                            Type::Optional(Box::new(prop_type))
                        } else {
                            prop_type
                        };
                        properties.push((prop_name.clone(), prop_type));
                    }
                    Type::Object(properties)
                }
                Expression::Call(_call_expr) => {
                    // Function call: trust the inferred return type. Functions declared
                    // to return `watcher` now infer correctly as Type::Watcher (the parser
                    // supports the annotation and self.functions records the real return
                    // type), so the Phase 10-δ-γ "assume watcher" heuristic is no longer
                    // needed and has been removed (it wrongly forced Type::Watcher for any
                    // Nothing-returning function call in a let initializer).
                    self.infer_expression_type_for_codegen(initializer)
                }
                _ => {
                    // For other complex expressions, try to infer
                    self.infer_expression_type_for_codegen(initializer)
                }
            }
        } else {
            // Uninitialized variable has type nothing
            Type::Nothing
        };

        // Phase 3b: a declaration the boxing analysis marks lowers to a
        // cell. Containers and heap values are subsumed (they already ARE
        // cells); scalar kinds without a payload yet reject cleanly.
        if self.is_boxed_decl(name, position) {
            match &var_type {
                Type::I32 => {
                    if self.hoisted_variables.contains_key(name) {
                        return Err(CodegenError::UnsupportedFeature {
                            feature: format!("'{}' is both watched and captured by a function closure", name),
                            phase: "a future phase — closure envs do not hold scalar cells yet (Phase 3b)".to_string(),
                        });
                    }
                    let c_var_name = self.mangle_variable_name(name);
                    if self.in_main_program {
                        // Program-scope cells are file-scope statics so
                        // nested functions can subscribe and capture them
                        // (cell identity is what makes this sound).
                        self.boxed_scalar_statics.push_str(&format!("static HiLowScalar* {} = NULL;\n", c_var_name));
                        self.output.push_str(&format!("  {} = hl_scalar_new_i32(", c_var_name));
                    } else {
                        self.output.push_str(&format!("  HiLowScalar* {} = hl_scalar_new_i32(", c_var_name));
                    }
                    if let Some(ref initializer) = initializer {
                        let old_context = self.function_expr_context.clone();
                        self.function_expr_context = FunctionExprContext::LetInitializer;
                        self.generate_expression(initializer, type_checker, ExprContext::Owned)?;
                        self.function_expr_context = old_context;
                    } else {
                        self.output.push_str("0");
                    }
                    self.output.push_str(");\n");
                    self.variable_types.insert(name.to_string(), Type::I32);
                    self.track_heap_owner(name, HeapType::Scalar);
                    self.push_boxed_binding(name, true);
                    return Ok(());
                }
                Type::String | Type::DynamicArray(_) | Type::Object(_)
                    if self.needs_slot_decl(name, position) =>
                {
                    // Phase 3e-α: a subscription requires this variable be a
                    // SLOT cell — a HiLowScalar with a retained reference
                    // payload. The constructor ADOPTS a +1; borrowed
                    // initializers retain first via hl_array_ref/hl_object_ref.
                    if self.hoisted_variables.contains_key(name) {
                        return Err(CodegenError::UnsupportedFeature {
                            feature: format!("'{}' is both watched and captured by a function closure", name),
                            phase: "a future phase — closure envs do not hold scalar cells yet (Phase 3b)".to_string(),
                        });
                    }
                    let (ctor, _, _) = Self::slot_fns_for(&var_type).unwrap();
                    let ref_fn = if matches!(var_type, Type::Object(_)) { "hl_object_ref" } else { "hl_array_ref" };
                    let c_var_name = self.mangle_variable_name(name);
                    if self.in_main_program {
                        self.boxed_scalar_statics.push_str(&format!("static HiLowScalar* {} = NULL;\n", c_var_name));
                        self.output.push_str(&format!("  {} = {}(", c_var_name, ctor));
                    } else {
                        self.output.push_str(&format!("  HiLowScalar* {} = {}(", c_var_name, ctor));
                    }
                    let initializer = initializer.as_ref().ok_or_else(|| CodegenError::UnsupportedFeature {
                        feature: format!("uninitialized slot-watched variable '{}'", name),
                        phase: "Phase 3e-α — slot payloads are never NULL by construction".to_string(),
                    })?;
                    let borrowed = Self::expr_is_borrowed_ref(initializer);
                    if borrowed {
                        self.output.push_str(&format!("{}(", ref_fn));
                    }
                    let old_context = self.function_expr_context.clone();
                    self.function_expr_context = FunctionExprContext::LetInitializer;
                    self.generate_expression(initializer, type_checker, ExprContext::Owned)?;
                    self.function_expr_context = old_context;
                    if borrowed {
                        self.output.push_str(")");
                    }
                    self.output.push_str(");\n");
                    self.variable_types.insert(name.to_string(), var_type.clone());
                    self.track_heap_owner(name, HeapType::Scalar);
                    self.push_boxed_binding(name, true);
                    return Ok(());
                }
                Type::DynamicArray(_) | Type::Object(_) | Type::String | Type::Watcher
                | Type::Function(_, _) | Type::Optional(_) | Type::Unknown | Type::UnknownType
                | Type::Tuple(_) | Type::Nothing => {
                    // Subsumed: fall through to the normal lowering.
                }
                other => {
                    return Err(CodegenError::UnsupportedFeature {
                        feature: format!("watching or capturing '{}' of scalar type {:?}", name, other),
                        phase: "a future phase — boxed scalar payload kinds land as programs need them (i32 today; audit §5 item 7)".to_string(),
                    });
                }
            }
        }

        // Check if this variable is hoisted to an environment
        if let Some((env_var, _env_struct)) = self.hoisted_variables.get(name) {
            // Variable is hoisted - generate environment field assignment
            self.output.push_str(&format!("  {}->", env_var));
            self.output.push_str(name);

            if let Some(ref initializer) = initializer {
                self.output.push_str(" = ");
                // Phase 8a: Set context for escaping closure detection
                let old_context = self.function_expr_context.clone();
                self.function_expr_context = FunctionExprContext::LetInitializer;
                self.generate_expression(initializer, type_checker, ExprContext::Owned)?;
                self.function_expr_context = old_context;
            } else {
                // Uninitialized variable gets nothing value
                self.output.push_str(" = &the_nothing");
            }

            self.output.push_str(";\n");
        } else {
            // Normal variable declaration
            let c_type = self.hilow_type_to_c(&var_type);
            let c_var_name = self.mangle_variable_name(name);
            self.output.push_str(&format!("  {} {}", c_type, c_var_name));

            if let Some(ref initializer) = initializer {
                self.output.push_str(" = ");
                // Phase 8a: Set context for escaping closure detection
                let old_context = self.function_expr_context.clone();
                self.function_expr_context = FunctionExprContext::LetInitializer;
                self.generate_expression(initializer, type_checker, ExprContext::Owned)?;
                self.function_expr_context = old_context;
            } else {
                // Uninitialized variable gets nothing value
                self.output.push_str(" = &the_nothing");
            }

            self.output.push_str(";\n");
        }

        // Track the variable type for later reference
        self.variable_types.insert(name.to_string(), var_type.clone());
        // Phase 3b: record the (raw) binding so an inner raw shadow masks an
        // outer boxed one at read/assign sites
        self.push_boxed_binding(name, false);

        // Phase 8a: Track heap ownership if initializer creates heap allocation
        if let Some(ref initializer) = initializer {
            match initializer {
                Expression::ObjectLiteral(_) => {
                    self.track_heap_owner(name, HeapType::Object);
                }
                Expression::FunctionExpr(_) => {
                    self.track_heap_owner(name, HeapType::Function);
                }
                Expression::FString(_) => {
                    self.track_heap_owner(name, HeapType::Array);
                }
                Expression::Unknown(_) => {
                    self.track_heap_owner(name, HeapType::Unknown);
                }
                Expression::ArrayLit(_, _) => {
                    self.track_heap_owner(name, HeapType::Array);
                }
                Expression::StringLit(_, _) => {
                    self.track_heap_owner(name, HeapType::Array); // String is HiLowArray<u8>
                }
                Expression::TupleLit(elements, _) => {
                    // Check if tuple contains heap-allocated elements
                    let mut element_types = Vec::new();
                    let mut has_heap_elements = false;
                    for element in elements {
                        let element_type = self.infer_expression_type_for_codegen(element);
                        element_types.push(element_type.clone());
                        match element_type {
                            Type::String | Type::Object(_) | Type::Function(_, _) | Type::DynamicArray(_) |
                            Type::Optional(_) | Type::UnknownType => {
                                has_heap_elements = true;
                            }
                            _ => {}
                        }
                    }
                    if has_heap_elements {
                        self.track_heap_owner(name, HeapType::Tuple(element_types));
                    }
                }
                Expression::BinaryOp(binary_op) => {
                    // Track heap ownership for binary operations that return heap types
                    let result_type = self.infer_expression_type_for_codegen(initializer);
                    match result_type {
                        Type::String => {
                            self.track_heap_owner(name, HeapType::Array); // String concatenation returns HiLowArray<u8>
                        }
                        Type::DynamicArray(_) => {
                            self.track_heap_owner(name, HeapType::Array);
                        }
                        _ => {
                            // Non-heap return types don't need tracking
                        }
                    }
                }
                Expression::Match(_) => {
                    // Match-as-expression: the taken arm's value is generated in
                    // Owned context, so the binding owns any heap result
                    let result_type = self.infer_expression_type_for_codegen(initializer);
                    match result_type {
                        Type::String => {
                            self.track_heap_owner(name, HeapType::Array); // String is HiLowArray<u8>
                        }
                        Type::DynamicArray(_) => {
                            self.track_heap_owner(name, HeapType::Array);
                        }
                        Type::Object(_) => {
                            self.track_heap_owner(name, HeapType::Object);
                        }
                        Type::Function(_, _) => {
                            self.track_heap_owner(name, HeapType::Function);
                        }
                        _ => {
                            // Non-heap result types don't need tracking
                        }
                    }
                }
                Expression::Call(_) => {
                    // Track heap ownership for function calls that return heap types
                    let result_type = self.infer_expression_type_for_codegen(initializer);
                    match result_type {
                        Type::String => {
                            self.track_heap_owner(name, HeapType::Array); // String is HiLowArray<u8>
                        }
                        Type::DynamicArray(_) => {
                            self.track_heap_owner(name, HeapType::Array);
                        }
                        Type::Object(_) => {
                            self.track_heap_owner(name, HeapType::Object);
                        }
                        Type::Function(_, _) => {
                            self.track_heap_owner(name, HeapType::Function);
                        }
                        Type::Watcher => {
                            self.track_heap_owner(name, HeapType::Watcher);
                        }
                        Type::Optional(_) => {
                            self.track_heap_owner(name, HeapType::Optional);
                        }
                        Type::Unknown => {
                            self.track_heap_owner(name, HeapType::Unknown);
                        }
                        _ => {
                            // Non-heap return types don't need tracking
                        }
                    }
                }
                Expression::WatcherExpr(_) => {
                    // Phase 2a/3b: EVERY watcher registers by construction
                    // inside the expression itself (hl_watcher_new_subscribed)
                    // — the let just owns the value. The scalar name-keyed
                    // side-channel died with the firing block.
                    self.track_heap_owner(name, HeapType::Watcher);
                }
                Expression::Call(call_expr) => {
                    // Check if this is a function call that returns a heap value
                    let return_type = if let Expression::Ident { name: func_name, .. } = call_expr.callee.as_ref() {
                        // Direct function call
                        self.functions.get(func_name).cloned()
                    } else if let Expression::MemberAccess(member_access) = call_expr.callee.as_ref() {
                        // Member function call - check for time builtins
                        if let Expression::Ident { name, .. } = member_access.object.as_ref() {
                            if name == "time" {
                                match member_access.member.as_str() {
                                    "now" => Some(Type::Time), // time.now() -> time (value type, no heap)
                                    "parse" => Some(Type::Optional(Box::new(Type::Time))), // time.parse() -> time? (heap)
                                    _ => None
                                }
                            } else {
                                // Check for array method calls (pop, remove)
                                let object_type = self.infer_expression_type_for_codegen(&member_access.object);
                                if let Type::DynamicArray(elem_type) = object_type {
                                    match member_access.member.as_str() {
                                        "pop" | "remove" => Some(*elem_type), // Return element type
                                        _ => None,
                                    }
                                } else {
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some(return_type) = return_type {
                        match return_type {
                            Type::Object(_) => {
                                self.track_heap_owner(name, HeapType::Object);
                            }
                            Type::Function(_, _) => {
                                self.track_heap_owner(name, HeapType::Function);
                            }
                            Type::Optional(_) => {
                                // Optional types need conditional cleanup based on runtime value
                                self.track_heap_owner(name, HeapType::Optional);
                                // Track Optional variables in main program for backup cleanup
                                if self.in_main_program {
                                    self.main_program_optionals.push(name.to_string());
                                }
                            }
                            Type::UnknownType => {
                                // Direct unknown values also need cleanup
                                self.track_heap_owner(name, HeapType::Unknown);
                            }
                            Type::Watcher => {
                                // Phase 10-δ-γ: Watcher return values need heap tracking
                                self.track_heap_owner(name, HeapType::Watcher);
                            }
                            Type::DynamicArray(_) => {
                                // Array Phase A: Array return values need heap tracking
                                self.track_heap_owner(name, HeapType::Array);
                            }
                            _ => {} // Non-heap return types (like Type::Time)
                        }
                    }
                }
                Expression::Ident { name: var_name, .. } => {
                    // Phase 8b: Handle heap value aliasing with refcounting
                    if let Some((heap_type, _)) = self.heap_owners.get(var_name).cloned() {
                        // Generate retain call after the assignment
                        match heap_type {
                            HeapType::Object => {
                                let c_var_name = self.mangle_variable_name(name);
                                self.output.push_str(&format!(";\n  hl_object_retain({});", c_var_name));
                            }
                            HeapType::Function => {
                                let c_var_name = self.mangle_variable_name(name);
                                self.output.push_str(&format!(";\n  hl_function_retain({});", c_var_name));
                            }
                            HeapType::Unknown => {
                                let c_var_name = self.mangle_variable_name(name);
                                self.output.push_str(&format!(";\n  hl_unknown_retain({});", c_var_name));
                            }
                            HeapType::Array => {
                                let c_var_name = self.mangle_variable_name(name);
                                self.output.push_str(&format!(";\n  hl_array_retain({});", c_var_name));
                            }
                            HeapType::Scalar => {
                                // Phase 3e-β: reading a slot-boxed variable
                                // yields its PAYLOAD (type-keyed getter, a
                                // borrow) — the binding owns a payload
                                // reference, never the slot. Retain and
                                // track by payload kind; i32 payloads copy
                                // by value (no tracking).
                                let c_var_name = self.mangle_variable_name(name);
                                match self.variable_types.get(var_name) {
                                    Some(Type::DynamicArray(_)) | Some(Type::String) => {
                                        self.output.push_str(&format!(";\n  hl_array_retain({});", c_var_name));
                                        self.track_heap_owner(name, HeapType::Array);
                                    }
                                    Some(Type::Object(_)) => {
                                        self.output.push_str(&format!(";\n  hl_object_retain({});", c_var_name));
                                        self.track_heap_owner(name, HeapType::Object);
                                    }
                                    _ => {}
                                }
                                return Ok(());
                            }
                            _ => {} // Other heap types don't have specific retain functions yet
                        }

                        // Track the new variable as a heap owner
                        self.track_heap_owner(name, heap_type);
                    }
                }
                Expression::MemberAccess(_) | Expression::This(_) => {
                    // Phase 1.5c ownership axiom: a let-binding stores a heap
                    // reference, so it must own one. Object/function member
                    // reads are borrows (retain them); string member reads
                    // arrive +1 from hl_object_get_str (track only).
                    // Phase 1.5e: weak reads / weak member propagation arrive
                    // +1 as HiLowOptional* (track only).
                    let result_type = self.infer_expression_type_for_codegen(initializer);
                    let c_var_name = self.mangle_variable_name(name);
                    match result_type {
                        Type::Object(_) => {
                            self.output.push_str(&format!("  hl_object_retain({});\n", c_var_name));
                            self.track_heap_owner(name, HeapType::Object);
                        }
                        Type::Function(_, _) => {
                            self.output.push_str(&format!("  hl_function_retain({});\n", c_var_name));
                            self.track_heap_owner(name, HeapType::Function);
                        }
                        Type::DynamicArray(_) => {
                            // Phase 2e: array member reads are borrows
                            // (hl_object_get_array), mirroring objects
                            self.output.push_str(&format!("  hl_array_retain({});\n", c_var_name));
                            self.track_heap_owner(name, HeapType::Array);
                        }
                        Type::String => {
                            self.track_heap_owner(name, HeapType::Array);
                        }
                        Type::Optional(_) => {
                            self.track_heap_owner(name, HeapType::Optional);
                            if self.in_main_program {
                                self.main_program_optionals.push(name.to_string());
                            }
                        }
                        _ => {}
                    }
                }
                Expression::IndexAccess(_) => {
                    // Phase 1.5c: binding an array element (hl_array_get
                    // returns a borrow) — retain so the binding owns its
                    // reference even if the array is mutated or dies first
                    let result_type = self.infer_expression_type_for_codegen(initializer);
                    let c_var_name = self.mangle_variable_name(name);
                    match result_type {
                        Type::Object(_) => {
                            self.output.push_str(&format!("  hl_object_retain({});\n", c_var_name));
                            self.track_heap_owner(name, HeapType::Object);
                        }
                        Type::DynamicArray(_) => {
                            self.output.push_str(&format!("  hl_array_retain({});\n", c_var_name));
                            self.track_heap_owner(name, HeapType::Array);
                        }
                        _ => {}
                    }
                }
                Expression::TypeAscription(_, ascribed_type, _) => {
                    // Track heap ownership based on the ascribed type
                    match ascribed_type {
                        crate::ast::Type::DynamicArray(_) => {
                            self.track_heap_owner(name, HeapType::Array);
                        }
                        crate::ast::Type::Object(_) => {
                            self.track_heap_owner(name, HeapType::Object);
                        }
                        crate::ast::Type::Function(_, _) => {
                            self.track_heap_owner(name, HeapType::Function);
                        }
                        crate::ast::Type::Optional(_) => {
                            self.track_heap_owner(name, HeapType::Optional);
                            if self.in_main_program {
                                self.main_program_optionals.push(name.to_string());
                            }
                        }
                        crate::ast::Type::Watcher => {
                            self.track_heap_owner(name, HeapType::Watcher);
                        }
                        _ => {} // Primitive types don't need heap tracking
                    }
                }
                _ => {} // Non-heap-allocating expressions
            }
        }

        Ok(())
    }

    fn generate_tuple_let_statement(&mut self, names: &[String], initializer: &Option<Expression>, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        if let Some(init) = initializer {
            // Generate unique temporary variable for the tuple value
            let temp_var = format!("__dest_{}", self.var_counter);
            self.var_counter += 1;

            // Infer the tuple type from the initializer
            let tuple_type = self.infer_expression_type_for_codegen(init);
            if let Type::Tuple(element_types) = tuple_type {
                // Ensure the tuple struct exists
                self.ensure_tuple_struct(&element_types);
                let struct_name = self.get_tuple_type_name(&element_types);

                // Generate temporary variable assignment
                self.output.push_str(&format!("  {} {} = ", struct_name, temp_var));
                self.generate_expression(init, type_checker, ExprContext::Temporary)?;
                self.output.push_str(";\n");

                // Extract each component into the destructured variables
                for (i, name) in names.iter().enumerate() {
                    if i < element_types.len() {
                        let element_type = &element_types[i];
                        let c_var_name = self.mangle_variable_name(name);
                        let c_type = self.hilow_type_to_c(element_type);

                        self.output.push_str(&format!("  {} {} = {}._{};\n",
                            c_type, c_var_name, temp_var, i));

                        // Track the variable type
                        self.variable_types.insert(name.clone(), element_type.clone());
                    } else {
                        return Err(CodegenError::UnsupportedFeature {
                            feature: format!("Tuple arity mismatch: {} variables for {}-element tuple",
                                names.len(), element_types.len()),
                            phase: "Phase 9e".to_string(),
                        });
                    }
                }
            } else {
                return Err(CodegenError::UnsupportedFeature {
                    feature: "Tuple destructuring of non-tuple type".to_string(),
                    phase: "Phase 9e".to_string(),
                });
            }
        } else {
            return Err(CodegenError::UnsupportedFeature {
                feature: "Tuple destructuring without initializer".to_string(),
                phase: "Phase 9e".to_string(),
            });
        }
        Ok(())
    }

    fn generate_return_statement(&mut self, return_stmt: &ReturnStmt, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Phase 8a: Handle ownership transfer for returned heap values.
        // Phase 3b: NOT for boxed scalars — `return x` returns a COPY of the
        // payload (hl_scalar_get_i32), so the cell still releases with its
        // scope; transferring it would leak the cell.
        if let Some(ref value) = return_stmt.value {
            // Check if we're returning a variable that owns a heap value
            if let Expression::Ident { name: var_name, .. } = value {
                if self.heap_owners.contains_key(var_name)
                    && !matches!(self.heap_owners.get(var_name), Some((HeapType::Scalar, _)))
                {
                    // Transfer ownership - don't free this variable
                    self.transfer_ownership(var_name);
                }
            }
        }

        if self.in_main_program {
            // In main program: set return_value, emit cleanup, leak check, and actual return
            self.output.push_str("  return_value = ");
            if let Some(ref value) = return_stmt.value {
                // Phase 8a: Set context for escaping closure detection
                let old_context = self.function_expr_context.clone();
                self.function_expr_context = FunctionExprContext::ReturnValue;
                self.generate_expression(value, type_checker, ExprContext::Owned)?;
                self.function_expr_context = old_context;
            } else {
                self.output.push_str("0");
            }
            self.output.push_str(";\n");

            // Release this statement's expression temporaries before the leak
            // check — the statement-level cleanup would land after `return`
            // (dead code). return_value already holds the result. Enclosing
            // statements' temps (e.g. a surrounding match/switch subject or
            // arm literals) release too: return skips their statement-end
            // cleanup permanently.
            self.emit_temp_cleanup();
            self.emit_enclosing_temp_releases(0);

            // Phase 9b fix: Emit cleanup for all scopes before returning
            for scope in (1..=self.scope_depth).rev() {
                self.emit_early_return_cleanup(scope);
            }

            // Phase 9b fix: Emit memory leak check and actual return
            self.emit_leak_check_and_return();

            // Phase 11b-fixup: Track that main has explicitly returned
            self.main_explicitly_returned = true;
        } else {
            // In regular function: evaluate the return value into a temp BEFORE
            // emitting scope cleanup. Previously cleanup ran first, which freed
            // heap-owned locals (e.g. arrays) before the return expression read
            // from them (use-after-free for `return local[i]`). This matches the
            // main-program branch ordering: value first, then cleanup, then return.
            if let Some(ref value) = return_stmt.value {
                // Type the temp from the VALUE being returned, not from
                // current_function_return_type (which is stale/wrong inside nested
                // functions and closures — it reflects the enclosing function).
                // For optional-wrapped returns the temp must be the wrapper type.
                let value_type_for_temp = self.infer_expression_type_for_codegen(value);
                // Phase 2b step zero: the wrap set matches the optional
                // payload allow-list exactly (unsupported inners are rejected
                // at typecheck; the constructor dispatch below errors rather
                // than mis-kind if one slips through).
                let wrap_for_temp = if let Some(ref return_type) = self.current_function_return_type {
                    if let Type::Optional(_) = return_type {
                        matches!(value_type_for_temp, Type::I32 | Type::String | Type::UnknownType |
                                             Type::Time | Type::Duration | Type::Money | Type::MoneyOf(_))
                    } else { false }
                } else { false };
                let ret_c_type = if wrap_for_temp {
                    "HiLowOptional*".to_string()
                } else {
                    self.hilow_type_to_c(&value_type_for_temp)
                };
                let ret_temp = format!("__ret_{}", self.var_counter);
                self.var_counter += 1;

                let need_optional_wrap = wrap_for_temp;

                // Phase 1.5c ownership axiom: a function's return value is
                // always +1 to the caller. Owner idents were transferred
                // above; a BORROWED object/function value (non-owner ident,
                // member read, element read) is retained here — the scope
                // cleanup below may release its owner before `return`.
                let ref_wrap = match &value_type_for_temp {
                    Type::Object(_) | Type::Function(_, _) => {
                        let transferred_ident = matches!(value,
                            Expression::Ident { name, .. } if self.heap_owners.contains_key(name));
                        if !transferred_ident && Self::expr_is_borrowed_ref(value) {
                            Some(if matches!(value_type_for_temp, Type::Object(_)) {
                                "hl_object_ref"
                            } else {
                                "hl_function_ref"
                            })
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                self.output.push_str(&format!("  {} {} = ", ret_c_type, ret_temp));
                if need_optional_wrap {
                    // Phase 2b step zero: explicit constructor per payload
                    // kind. No catch-all — a value type outside the allow-list
                    // is a hard error, never a mis-kinded HL_OPT_I32.
                    match &value_type_for_temp {
                        Type::I32 => self.output.push_str("hl_optional_new_i32("),
                        Type::String => self.output.push_str("hl_optional_new_string("),
                        Type::UnknownType => self.output.push_str("hl_optional_new_unknown("),
                        Type::Time => self.output.push_str("hl_optional_new_time("),
                        Type::Duration => self.output.push_str("hl_optional_new_duration("),
                        Type::Money | Type::MoneyOf(_) => self.output.push_str("hl_optional_new_money("),
                        other => {
                            return Err(CodegenError::UnsupportedFeature {
                                feature: format!("optional wrap for return value of type {}", other),
                                phase: "Phase 3 (scalar boxing builds the optional payload matrix)".to_string(),
                            });
                        }
                    }
                }
                if let Some(ref_fn) = ref_wrap {
                    self.output.push_str(&format!("{}(", ref_fn));
                }
                let old_context = self.function_expr_context.clone();
                self.function_expr_context = FunctionExprContext::ReturnValue;
                self.generate_expression(value, type_checker, ExprContext::Owned)?;
                self.function_expr_context = old_context;
                if ref_wrap.is_some() {
                    self.output.push_str(")");
                }
                if need_optional_wrap {
                    self.output.push_str(")");
                }
                self.output.push_str(";\n");

                // Release this statement's expression temporaries before
                // `return` — the statement-level cleanup would land after it
                // (dead code). __ret_N already holds the result. Enclosing
                // statements' temps release too: return skips their
                // statement-end cleanup permanently.
                self.emit_temp_cleanup();
                self.emit_enclosing_temp_releases(0);

                for scope in (1..=self.scope_depth).rev() {
                    self.emit_early_return_cleanup(scope);
                }

                self.output.push_str(&format!("  return {};\n", ret_temp));
            } else {
                // Bare return still skips enclosing statements' temp cleanup
                self.emit_temp_cleanup();
                self.emit_enclosing_temp_releases(0);
                for scope in (1..=self.scope_depth).rev() {
                    self.emit_early_return_cleanup(scope);
                }
                self.output.push_str("  return;\n");
            }
        }
        Ok(())
    }

    fn generate_if_statement(&mut self, if_stmt: &IfStmt, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        self.output.push_str("  if (");
        self.generate_condition(&if_stmt.condition, type_checker)?;
        self.output.push_str(") {\n");

        self.generate_block(&if_stmt.then_block, type_checker)?;

        self.output.push_str("  }");

        if let Some(else_block) = &if_stmt.else_block {
            self.output.push_str(" else {\n");
            self.generate_block(else_block, type_checker)?;
            self.output.push_str("  }");
        }

        self.output.push_str("\n");
        Ok(())
    }

    /// Set up loop context for break/continue cleanup while generating a loop
    /// body (Phase 1.5b): push a LoopFrame and clear the switch flags — a
    /// break inside a loop inside a switch case targets the loop, matching
    /// the C semantics of the emitted code.
    fn enter_loop_body(&mut self, extra_cleanups: Vec<String>) -> (bool, bool) {
        self.loop_frames.push(LoopFrame {
            extra_cleanups,
            temp_frame_base: self.enclosing_temp_frames.len(),
        });
        let saved = (self.in_string_switch, self.in_c_switch);
        self.in_string_switch = false;
        self.in_c_switch = false;
        saved
    }

    fn exit_loop_body(&mut self, saved_switch_flags: (bool, bool)) {
        self.in_string_switch = saved_switch_flags.0;
        self.in_c_switch = saved_switch_flags.1;
        self.loop_frames.pop();
    }

    fn generate_while_statement(&mut self, while_stmt: &WhileStmt, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        self.output.push_str("  while (");
        self.generate_condition(&while_stmt.condition, type_checker)?;
        self.output.push_str(") {\n");

        let saved_flags = self.enter_loop_body(Vec::new());
        let result = self.generate_block(&while_stmt.body, type_checker);
        self.exit_loop_body(saved_flags);
        result?;

        self.output.push_str("  }\n");
        Ok(())
    }

    fn generate_loop_statement(&mut self, loop_stmt: &LoopStmt, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        self.output.push_str("  while (1) {\n");

        let saved_flags = self.enter_loop_body(Vec::new());
        let result = self.generate_block(&loop_stmt.body, type_checker);
        self.exit_loop_body(saved_flags);
        result?;

        self.output.push_str("  }\n");
        Ok(())
    }

    fn generate_for_in_statement(&mut self, for_in_stmt: &ForInStmt, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Determine if we're iterating over an object or array
        let iterable_type = self.infer_expression_type_for_codegen(&for_in_stmt.iterable);

        self.output.push_str("  {\n");

        match iterable_type {
            Type::Object(_) => {
                // Generate runtime iteration over object properties
                // for (let (key, value) in obj) { body }
                // becomes:
                // {
                //     HiLowObject* __iter_obj = obj;
                //     size_t __iter_count = hl_object_property_count(__iter_obj);
                //     for (size_t __iter_i = 0; __iter_i < __iter_count; __iter_i++) {
                //         const char* key = hl_object_property_key_at(__iter_obj, __iter_i);
                //         int __v_type = hl_object_property_type_at(__iter_obj, __iter_i);
                //         // Body with value dispatch based on __v_type
                //     }
                // }

                // Generate the iterable object
                self.output.push_str("    HiLowObject* __iter_obj = ");
                self.generate_expression(&for_in_stmt.iterable, type_checker, ExprContext::Temporary)?;
                self.output.push_str(";\n");

                // Get property count
                self.output.push_str("    size_t __iter_count = hl_object_property_count(__iter_obj);\n");

                // Generate the iteration loop
                self.output.push_str("    for (size_t __iter_i = 0; __iter_i < __iter_count; __iter_i++) {\n");

                // Get key and type for current iteration. The key is a managed
                // string wrapping the object's internal char* key, released at
                // the bottom of each iteration.
                let key_c_name = self.mangle_variable_name(&for_in_stmt.key_name);
                self.output.push_str(&format!("      HiLowArray* {} = hl_string_from_cstr(hl_object_property_key_at(__iter_obj, __iter_i));\n", key_c_name));
                self.output.push_str("      int __v_type = hl_object_property_type_at(__iter_obj, __iter_i);\n");

                // Store the value variable name and type for runtime dispatch in the loop body
                let old_iter_value_name = self.current_iter_value_name.clone();
                self.current_iter_value_name = Some(for_in_stmt.value_name.clone());

                // Update variable types to include the for-in variables
                self.variable_types.insert(for_in_stmt.key_name.clone(), Type::String);
                self.variable_types.insert(for_in_stmt.value_name.clone(), Type::ObjectIterValue);

                // Track the key so a return inside the body releases it
                self.track_heap_owner(&for_in_stmt.key_name, HeapType::Array);

                // Generate loop body; break/continue must release the key they
                // jump past (the loop-bottom release below)
                let saved_flags = self.enter_loop_body(vec![format!("hl_array_release({});", key_c_name)]);
                let body_result = self.generate_block(&for_in_stmt.body, type_checker);
                self.exit_loop_body(saved_flags);
                body_result?;

                // Restore previous state; release the key at iteration end
                self.heap_owners.remove(&for_in_stmt.key_name);
                self.output.push_str(&format!("      hl_array_release({});\n", key_c_name));
                self.current_iter_value_name = old_iter_value_name;
                self.variable_types.remove(&for_in_stmt.key_name);
                self.variable_types.remove(&for_in_stmt.value_name);
            }
            Type::DynamicArray(elem_type) => {
                // Generate array iteration with live length re-read each iteration
                // for (let (i, x) in arr) { body }
                // becomes:
                // {
                //     HiLowArray* __iter_arr = arr;
                //     for (size_t i = 0; i < hl_array_len(__iter_arr); i++) {
                //         ELEM_C_TYPE x = *(ELEM_C_TYPE*)hl_array_get(__iter_arr, i);
                //         body
                //     }
                // }

                let elem_c_type = self.hilow_type_to_c(&elem_type);

                // Store the iterable array in a temporary to avoid re-evaluating complex expressions
                self.output.push_str("    HiLowArray* __iter_arr = ");
                self.generate_expression(&for_in_stmt.iterable, type_checker, ExprContext::Temporary)?;
                self.output.push_str(";\n");

                // Generate the iteration loop with live length re-read (allows mutation during iteration)
                self.output.push_str(&format!("    for (size_t {} = 0; {} < hl_array_len(__iter_arr); {}++) {{\n",
                    for_in_stmt.key_name, for_in_stmt.key_name, for_in_stmt.key_name));

                // Get the element value for current iteration
                self.output.push_str(&format!("      {} {} = *({}*)hl_array_get(__iter_arr, {});\n",
                    elem_c_type, for_in_stmt.value_name, elem_c_type, for_in_stmt.key_name));

                // Phase 1.5c ownership axiom: the element binding owns a
                // reference for the iteration — retain at bind, release at
                // iteration end; break/continue release via the loop frame,
                // return via heap_owners.
                let value_c_name = self.mangle_variable_name(&for_in_stmt.value_name);
                let elem_cleanup = match elem_type.as_ref() {
                    Type::Object(_) => {
                        self.output.push_str(&format!("      hl_object_retain({});\n", value_c_name));
                        self.track_heap_owner(&for_in_stmt.value_name, HeapType::Object);
                        Some(format!("hl_object_release({});", value_c_name))
                    }
                    Type::DynamicArray(_) => {
                        self.output.push_str(&format!("      hl_array_retain({});\n", value_c_name));
                        self.track_heap_owner(&for_in_stmt.value_name, HeapType::Array);
                        Some(format!("hl_array_release({});", value_c_name))
                    }
                    _ => None,
                };

                // Update variable types to include the for-in variables
                self.variable_types.insert(for_in_stmt.key_name.clone(), Type::Usize);
                self.variable_types.insert(for_in_stmt.value_name.clone(), *elem_type);

                // Generate loop body
                let saved_flags = self.enter_loop_body(elem_cleanup.clone().into_iter().collect());
                let body_result = self.generate_block(&for_in_stmt.body, type_checker);
                self.exit_loop_body(saved_flags);
                body_result?;

                // Release the element binding at iteration end
                if let Some(cleanup) = &elem_cleanup {
                    self.heap_owners.remove(&for_in_stmt.value_name);
                    self.output.push_str(&format!("      {}\n", cleanup));
                }

                // Restore variable types
                self.variable_types.remove(&for_in_stmt.key_name);
                self.variable_types.remove(&for_in_stmt.value_name);
            }
            _ => {
                return Err(CodegenError::UnsupportedFeature {
                    feature: format!("for-in over {} type", iterable_type),
                    phase: "Array Phase D".to_string()
                });
            }
        }

        self.output.push_str("    }\n");
        self.output.push_str("  }\n");

        Ok(())
    }

    fn generate_switch_statement(&mut self, switch_stmt: &SwitchStmt, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Determine the type of the switch expression
        let switch_type = self.infer_expression_type_for_codegen(&switch_stmt.value);

        match switch_type {
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 |
            Type::Bool => {
                // For integers and booleans, emit a C switch statement with fallthrough
                self.output.push_str("  switch (");
                self.generate_expression(&switch_stmt.value, type_checker, ExprContext::Temporary)?;
                self.output.push_str(") {\n");

                // Break inside these case bodies targets the C switch, not an
                // enclosing loop — suppress loop-frame cleanup (Phase 1.5b)
                let old_in_c_switch = self.in_c_switch;
                self.in_c_switch = true;

                // Generate cases
                for case in &switch_stmt.cases {
                    self.output.push_str("    case ");
                    match &case.pattern {
                        Literal::Integer(n) => self.output.push_str(&n.to_string()),
                        Literal::Bool(b) => self.output.push_str(if *b { "1" } else { "0" }),
                        _ => unreachable!("Type checker should prevent non-matching patterns"),
                    }
                    self.output.push_str(":\n");

                    // Generate case body
                    for statement in &case.body {
                        self.generate_statement(statement, type_checker)?;
                    }
                }

                // Generate default case if present
                if let Some(default_statements) = &switch_stmt.default {
                    self.output.push_str("    default:\n");
                    for statement in default_statements {
                        self.generate_statement(statement, type_checker)?;
                    }
                }

                self.in_c_switch = old_in_c_switch;

                self.output.push_str("  }\n");
            }
            Type::String => {
                // For strings, emit if/else chain with strcmp (no fallthrough support)
                let temp_var = format!("__sw_val_{}", self.var_counter);
                self.var_counter += 1;

                self.output.push_str("  {\n");
                self.output.push_str(&format!("    HiLowArray* {} = ", temp_var));
                self.generate_expression(&switch_stmt.value, type_checker, ExprContext::Temporary)?;
                self.output.push_str(";\n");

                let mut first_case = true;

                // Set string switch context to suppress break statements
                let old_in_string_switch = self.in_string_switch;
                self.in_string_switch = true;

                for case in &switch_stmt.cases {
                    if !first_case {
                        self.output.push_str("    } else ");
                    } else {
                        self.output.push_str("    ");
                    }

                    self.output.push_str("if (hl_string_eq_cstr(");
                    self.output.push_str(&temp_var);
                    self.output.push_str(", ");

                    match &case.pattern {
                        Literal::String(s) => {
                            self.output.push_str("\"");
                            self.output.push_str(&Self::escape_c_string(s));
                            self.output.push_str("\"");
                        }
                        _ => unreachable!("Type checker should prevent non-matching patterns"),
                    }

                    self.output.push_str(")) {\n");

                    // Generate case body
                    for statement in &case.body {
                        self.generate_statement(statement, type_checker)?;
                    }

                    first_case = false;
                }

                // Generate default case if present
                if let Some(default_statements) = &switch_stmt.default {
                    if !first_case {
                        self.output.push_str("    } else {\n");
                    } else {
                        // No cases, just default
                        self.output.push_str("    {\n");
                    }
                    for statement in default_statements {
                        self.generate_statement(statement, type_checker)?;
                    }
                }

                // Restore string switch context
                self.in_string_switch = old_in_string_switch;

                if !first_case || switch_stmt.default.is_some() {
                    self.output.push_str("    }\n");
                }
                self.output.push_str("  }\n");
            }
            _ => {
                return Err(CodegenError::UnsupportedFeature {
                    feature: format!("switch on type {}", switch_type),
                    phase: "future phases".to_string(),
                });
            }
        }

        Ok(())
    }

    fn generate_assign_statement(&mut self, assign_stmt: &AssignStmt, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        self.output.push_str("  ");

        // Check if we're assigning to a member access (property)
        if let Expression::MemberAccess(member_access) = &assign_stmt.target {
            // For member access assignment, generate a setter call
            if assign_stmt.op != AssignOpKind::Assign {
                return Err(CodegenError::UnsupportedFeature {
                    feature: "compound assignment to object properties".to_string(),
                    phase: "future phases".to_string(),
                });
            }

            // Check if this is a weak reference assignment (Phase 8c)
            let is_weak_assignment = matches!(assign_stmt.value, Expression::WeakRef(_, _));

            // Determine the type of the value to call the right setter
            let value_type = self.infer_expression_type_for_codegen(&assign_stmt.value);

            // Phase 8b: Object property assignment with heap values now supported via refcounting
            // Phase 8c: Weak reference assignments handled specially

            // Phase 1.5c ownership axiom: property stores retain in the
            // runtime (set_property); weak stores route to
            // hl_object_set_object_weak (no retain, slot nulled on target
            // death). The site disposes of untracked fresh +1 values; borrows
            // and tracked temps are released by their own owners.
            if is_weak_assignment && matches!(value_type, Type::Object(_)) {
                let inner_expr = match &assign_stmt.value {
                    Expression::WeakRef(inner, _) => inner.as_ref(),
                    _ => unreachable!("is_weak_assignment checked above"),
                };
                // Phase 4a: weak store (no retain); the value's +1 is disposed
                // by the one temp mechanism — see the object-literal note at the
                // object-construction property-set site.
                self.output.push_str("hl_object_set_object_weak(");
                self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;
                self.output.push_str(&format!(", \"{}\", ", member_access.member));
                self.generate_expression(inner_expr, type_checker, ExprContext::Temporary)?;
                self.output.push_str(");\n");
            } else {
                match value_type {
                    Type::I32 => self.output.push_str("hl_object_set_i32("),
                    Type::I64 => self.output.push_str("hl_object_set_i64("),
                    Type::U32 => self.output.push_str("hl_object_set_u32("),
                    Type::U64 => self.output.push_str("hl_object_set_u64("),
                    Type::F32 => self.output.push_str("hl_object_set_f32("),
                    Type::F64 => self.output.push_str("hl_object_set_f64("),
                    Type::Bool => self.output.push_str("hl_object_set_bool("),
                    Type::String => self.output.push_str("hl_object_set_str("),
                    Type::Object(_) => self.output.push_str("hl_object_set_object("),
                    Type::Function(_, _) => self.output.push_str("hl_object_set_function("),
                    Type::DynamicArray(_) => self.output.push_str("hl_object_set_array("),
                    _ => {
                        return Err(CodegenError::UnsupportedFeature {
                            feature: format!("assignment of type {} to object property", value_type),
                            phase: "future phases".to_string(),
                        });
                    }
                }

                // Generate: object, property name, value. The value is
                // generated in Temporary context so fresh heap productions
                // are statement-tracked temps (released at statement end,
                // balancing the store's retain); borrowed locals emit bare.
                self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", \"");
                self.output.push_str(&member_access.member);
                self.output.push_str("\", ");
                self.generate_expression(&assign_stmt.value, type_checker, ExprContext::Temporary)?;
                self.output.push_str(");\n");
            }
        } else if let Expression::IndexAccess(index_access) = &assign_stmt.target {
            // Array Phase B: Index assignment (arr[i] = x) - route through hl_array_set
            if assign_stmt.op != AssignOpKind::Assign {
                return Err(CodegenError::UnsupportedFeature {
                    feature: "compound assignment to array elements".to_string(),
                    phase: "future phases".to_string(),
                });
            }

            // Verify this is actually array type to route correctly
            let array_type = self.infer_expression_type_for_codegen(&index_access.object);
            if !matches!(array_type, Type::DynamicArray(_)) {
                return Err(CodegenError::UnsupportedFeature {
                    feature: "index assignment on non-array types".to_string(),
                    phase: "future phases".to_string(),
                });
            }

            // Generate temp variable for the assigned value (need lvalue for address)
            let temp_var = format!("temp_{}", self.var_counter);
            self.var_counter += 1;

            // Determine element type for temp declaration
            if let Type::DynamicArray(elem_type) = &array_type {
                let elem_c_type = self.hilow_type_to_c(elem_type);
                // Phase 1.5c: refcount-managed elements (objects, nested
                // arrays) are generated in Temporary context — hl_array_set
                // retains on store, so borrowed locals need no site release
                // and tracked temps are released at statement end. Only
                // untracked fresh productions are disposed of here.
                let managed = matches!(**elem_type, Type::Object(_) | Type::DynamicArray(_));
                let value_ctx = if managed { ExprContext::Temporary } else { ExprContext::Owned };
                self.output.push_str(&format!("  {} {} = ", elem_c_type, temp_var));
                self.generate_expression(&assign_stmt.value, type_checker, value_ctx)?;
                self.output.push_str(";\n");
                self.output.push_str("  hl_array_set(");
                self.generate_expression(&index_access.object, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&index_access.index, type_checker, ExprContext::Temporary)?;
                self.output.push_str(&format!(", &{});\n", temp_var));
                // Phase 4a: hl_array_set retained on store; the value's own +1
                // is disposed by the one temp mechanism (fresh productions mint
                // statement-scoped temps; borrows are scope-owned), no site
                // release here.
            }
        } else {
            // Regular assignment to variables

            // Phase 3b: assignment to a BOXED scalar lowers to the
            // hl_cell_set family — store + equality check + notify (changed
            // fires only on inequality, assigned on every set; stealth
            // suppresses both). Compound assignment is an assignment: it
            // reads the payload, applies the operator, and sets — the old
            // Phase 10-γ rejection and the firing block died here.
            if let Expression::Ident { name: var_name, .. } = &assign_stmt.target {
                let boxed_capture = self.boxed_hoisted.contains(var_name);
                if boxed_capture || self.current_binding_boxed(var_name) {
                    let cell = self.env_slot_rvalue(var_name);
                    let var_ty = self.variable_types.get(var_name).cloned().unwrap_or(Type::I32);
                    // Phase 3e-α: reference-payload slots. The set ADOPTS a
                    // +1 rhs; borrowed rhs values retain first (self-
                    // assignment safe: old arrives at +2).
                    if !matches!(var_ty, Type::I32) {
                        let (_, _, setter) = Self::slot_fns_for(&var_ty).ok_or_else(|| {
                            CodegenError::UnsupportedFeature {
                                feature: format!("slot assignment to '{}' of type {:?}", var_name, var_ty),
                                phase: "Phase 3e — slot payload kinds land as programs need them".to_string(),
                            }
                        })?;
                        if assign_stmt.op != AssignOpKind::Assign {
                            return Err(CodegenError::UnsupportedFeature {
                                feature: format!("compound assignment to slot-watched variable '{}' of type {:?}", var_name, var_ty),
                                phase: "Phase 3e — plain assignment only on reference slots".to_string(),
                            });
                        }
                        let ref_fn = if matches!(var_ty, Type::Object(_)) { "hl_object_ref" } else { "hl_array_ref" };
                        let borrowed = Self::expr_is_borrowed_ref(&assign_stmt.value);
                        self.output.push_str(&format!("  {}({}, ", setter, cell));
                        if borrowed {
                            self.output.push_str(&format!("{}(", ref_fn));
                        }
                        self.generate_expression(&assign_stmt.value, type_checker, ExprContext::Owned)?;
                        if borrowed {
                            self.output.push_str(")");
                        }
                        self.output.push_str(");\n");
                        return Ok(());
                    }
                    let compound_op = match assign_stmt.op {
                        AssignOpKind::Assign => None,
                        AssignOpKind::AddAssign => Some("+"),
                        AssignOpKind::SubAssign => Some("-"),
                        AssignOpKind::MulAssign => Some("*"),
                        AssignOpKind::DivAssign => Some("/"),
                        AssignOpKind::ModAssign => Some("%"),
                    };
                    self.output.push_str(&format!("  hl_cell_set_i32({}, ", cell));
                    if let Some(op) = compound_op {
                        self.output.push_str(&format!("hl_scalar_get_i32({}) {} (", cell, op));
                    }
                    self.generate_expression(&assign_stmt.value, type_checker, ExprContext::Owned)?;
                    if compound_op.is_some() {
                        self.output.push_str(")");
                    }
                    self.output.push_str(");\n");
                    return Ok(());
                }
            }

            // Normal assignment (no watchers)
            // Phase 11a: Release old value for heap-owned variables on reassignment
            if assign_stmt.op == AssignOpKind::Assign {
                if let Expression::Ident { name: var_name, .. } = &assign_stmt.target {
                    if let Some((heap_type, _)) = self.heap_owners.get(var_name).cloned() {
                        let c_var_name = self.mangle_variable_name(var_name);

                        // Phase 1.5c ownership axiom: assigning a BORROWED
                        // reference must retain it — and the retain happens
                        // BEFORE the old value's release so `x = x` survives.
                        // (String member reads are +1 from hl_object_get_str,
                        // not borrows, so Array only treats idents/element
                        // reads as borrowed.)
                        let rhs_borrowed = match &heap_type {
                            HeapType::Object | HeapType::Function => {
                                Self::expr_is_borrowed_ref(&assign_stmt.value)
                            }
                            HeapType::Array => {
                                matches!(&assign_stmt.value, Expression::Ident { .. })
                                    || matches!(&assign_stmt.value, Expression::IndexAccess(_))
                            }
                            _ => false,
                        };
                        if rhs_borrowed {
                            let (c_type, retain_fn, release_fn) = match &heap_type {
                                HeapType::Object => ("HiLowObject*", "hl_object_retain", "hl_object_release"),
                                HeapType::Function => ("HiLowFunction*", "hl_function_retain", "hl_function_release"),
                                HeapType::Array => ("HiLowArray*", "hl_array_retain", "hl_array_release"),
                                _ => unreachable!("rhs_borrowed only true for the types above"),
                            };
                            let temp_var = format!("temp_{}", self.var_counter);
                            self.var_counter += 1;
                            self.output.push_str(&format!("  {{ {} {} = ", c_type, temp_var));
                            self.generate_expression(&assign_stmt.value, type_checker, ExprContext::Temporary)?;
                            self.output.push_str(&format!(";\n    {}({});\n", retain_fn, temp_var));
                            self.output.push_str(&format!("    {}({});\n", release_fn, c_var_name));
                            self.output.push_str(&format!("    {} = {}; }}\n", c_var_name, temp_var));
                            return Ok(());
                        }

                        // Release old value before assignment, mirroring array reassignment pattern
                        match &heap_type {
                            HeapType::Array => {
                                self.output.push_str(&format!("  hl_array_release({});\n", c_var_name));
                            },
                            HeapType::Object => {
                                self.output.push_str(&format!("  hl_object_release({});\n", c_var_name));
                            },
                            HeapType::Function => {
                                self.output.push_str(&format!("  hl_function_release({});\n", c_var_name));
                            },
                            HeapType::Unknown => {
                                self.output.push_str(&format!("  hl_unknown_release({});\n", c_var_name));
                            },
                            HeapType::Watcher => {
                                self.output.push_str(&format!("  hl_watcher_release({});\n", c_var_name));
                            },
                            HeapType::Optional => {
                                self.output.push_str(&format!("  hl_optional_release({});\n", c_var_name));
                            },
                            HeapType::Tuple(element_types) => {
                                // Release heap-allocated elements in the tuple
                                for (i, element_type) in element_types.iter().enumerate() {
                                    match element_type {
                                        Type::String | Type::DynamicArray(_) => {
                                            self.output.push_str(&format!("  hl_array_release({}._{});\n", c_var_name, i));
                                        },
                                        Type::Object(_) => {
                                            self.output.push_str(&format!("  hl_object_release({}._{});\n", c_var_name, i));
                                        },
                                        Type::Function(_, _) => {
                                            self.output.push_str(&format!("  hl_function_release({}._{});\n", c_var_name, i));
                                        },
                                        Type::Unknown => {
                                            self.output.push_str(&format!("  hl_unknown_release({}._{});\n", c_var_name, i));
                                        },
                                        Type::Optional(_) => {
                                            self.output.push_str(&format!("  hl_optional_release({}._{});\n", c_var_name, i));
                                        },
                                        Type::UnknownType => {
                                            self.output.push_str(&format!("  hl_unknown_release({}._{});\n", c_var_name, i));
                                        },
                                        _ => {
                                            // Primitive types don't need release
                                        }
                                    }
                                }
                            },
                            _ => {
                                // Environment uses free() - handled in scope cleanup
                            }
                        }
                    }
                }
            }

            self.generate_expression(&assign_stmt.target, type_checker, ExprContext::Temporary)?;

            let op_str = match assign_stmt.op {
                AssignOpKind::Assign => " = ",
                AssignOpKind::AddAssign => " += ",
                AssignOpKind::SubAssign => " -= ",
                AssignOpKind::MulAssign => " *= ",
                AssignOpKind::DivAssign => " /= ",
                AssignOpKind::ModAssign => " %= ",
            };

            self.output.push_str(op_str);
            self.generate_expression(&assign_stmt.value, type_checker, ExprContext::Owned)?;
            self.output.push_str(";\n");
        }
        Ok(())
    }

    /// Phase 4b: Generate condition expressions with truthy/falsy dispatch
    fn generate_condition(&mut self, condition: &Expression, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Determine the type of the condition
        let condition_type = self.infer_expression_type_for_codegen(condition);

        match condition_type {
            Type::Bool => {
                // For bool, just generate the expression directly
                self.generate_expression(condition, type_checker, ExprContext::Temporary)?;
            }
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 |
            Type::Isize | Type::Usize | Type::F32 | Type::F64 => {
                // For numeric types, emit (expr != 0) for truthy/falsy check
                self.output.push_str("(");
                self.generate_expression(condition, type_checker, ExprContext::Temporary)?;
                self.output.push_str(" != 0)");
            }
            Type::Nothing => {
                // Nothing is always falsy, emit false
                self.output.push_str("false");
            }
            _ => {
                // This should be caught by the type checker, but handle gracefully
                return Err(CodegenError::UnsupportedFeature {
                    feature: format!("conditions with type {}", condition_type),
                    phase: "Phase 4b (truthy/falsy)".to_string(),
                });
            }
        }
        Ok(())
    }

    fn generate_expression(&mut self, expression: &Expression, type_checker: &TypeChecker, context: ExprContext) -> Result<(), CodegenError> {
        // Phase 4a: fresh heap productions (object/array/f-string/function-expr
        // literals and object/function-typed match) in Temporary position mint
        // a statement-scoped tracked temp, released at statement end — the ONE
        // temporary-lifetime mechanism. This subsumes the old store-site
        // release path (needs_site_release_after_store, now deleted) and closes
        // the non-store leak (a fresh literal in a bare statement or as a
        // borrowing call argument). Owned context (let/return) still binds
        // directly into heap_owners. The construction is captured by
        // recursing in Owned context (which emits the raw +1 production and
        // never re-enters this arm); nested fresh productions inside it mint
        // their own temps, hoisted before this one (pending_statement_decls is
        // an ordered Vec) — the string-concat precedent generalized.
        if context == ExprContext::Temporary {
            if let Some((heap_type, c_type)) = self.fresh_production_temp_kind(expression) {
                let temp_name = self.next_temp_name();
                let saved = std::mem::take(&mut self.output);
                self.generate_expression(expression, type_checker, ExprContext::Owned)?;
                let construction = std::mem::replace(&mut self.output, saved);
                self.pending_statement_decls
                    .push(format!("{} {} = {};", c_type, temp_name, construction));
                self.temp_owners.insert(temp_name.clone(), heap_type);
                self.output.push_str(&temp_name);
                return Ok(());
            }
        }
        match expression {
            Expression::IntLit(value, _) => {
                self.output.push_str(&value.to_string());
            }
            Expression::FloatLit(value, _) => {
                self.output.push_str(&value.to_string());
            }
            Expression::StringLit(value, _) => {
                // String as HiLowArray<u8> - create array with UTF-8 bytes
                let utf8_bytes = value.as_bytes();
                let byte_count = utf8_bytes.len();

                let str_var_name = if context == ExprContext::Temporary {
                    // Use tracked temporary name
                    let temp_name = self.next_temp_name();
                    self.temp_owners.insert(temp_name.clone(), HeapType::Array);
                    temp_name
                } else {
                    // Use untracked name for owned context
                    "__str".to_string()
                };

                if matches!(context, ExprContext::Temporary) {
                    // Hoist temp declaration to statement scope for cleanup visibility
                    let mut decl = format!("HiLowArray* {} = hl_array_new(sizeof(uint8_t), {}, NULL, NULL);", str_var_name, byte_count);

                    // Push each UTF-8 byte
                    for (i, &byte_val) in utf8_bytes.iter().enumerate() {
                        decl.push_str(&format!(" {{ uint8_t __b{} = {}; hl_array_push({}, &__b{}); }}", i, byte_val, str_var_name, i));
                    }

                    self.pending_statement_decls.push(decl);

                    // Emit just the reference
                    self.output.push_str(&str_var_name);
                } else {
                    // Use GCC statement-expression for inline string construction (owned context)
                    self.output.push_str(&format!("({{ HiLowArray* {} = hl_array_new(sizeof(uint8_t), {}, NULL, NULL);\n", str_var_name, byte_count));

                    // Push each UTF-8 byte
                    for (i, &byte_val) in utf8_bytes.iter().enumerate() {
                        self.output.push_str(&format!("     uint8_t __b{} = {}; hl_array_push({}, &__b{});\n", i, byte_val, str_var_name, i));
                    }

                    self.output.push_str(&format!("     {}; }})", str_var_name));
                }
            }
            Expression::DurationLit(nanos, _, _) => {
                // Emit duration as struct initializer
                self.output.push_str(&format!("((HiLowDuration){{ {} }})", nanos));
            }
            Expression::MoneyLit(micro_units, currency, _) => {
                // Convert currency string to enum value
                let currency_enum = match currency.as_str() {
                    "USD" => "HL_CURRENCY_USD",
                    "EUR" => "HL_CURRENCY_EUR",
                    "GBP" => "HL_CURRENCY_GBP",
                    "JPY" => "HL_CURRENCY_JPY",
                    "CAD" => "HL_CURRENCY_CAD",
                    "AUD" => "HL_CURRENCY_AUD",
                    "CHF" => "HL_CURRENCY_CHF",
                    "CNY" => "HL_CURRENCY_CNY",
                    _ => "HL_CURRENCY_USD", // fallback
                };
                // Emit money as struct initializer
                self.output.push_str(&format!("((HiLowMoney){{ {}, {} }})", micro_units, currency_enum));
            }
            Expression::FString(fstring) => {
                self.generate_fstring(fstring, type_checker)?;
            }
            Expression::BoolLit(value, _) => {
                self.output.push_str(if *value { "true" } else { "false" });
            }
            Expression::Ident { name, refined_type, .. } => {
                // Phase 11a-δ-α: Check if this is a cross-module reference first
                if let Some(mangled_name) = self.current_name_map.as_ref().and_then(|m| m.get(name)).cloned() {
                    self.output.push_str(&mangled_name);
                    return Ok(());
                }

                // Phase 3b/3e: a boxed cell capture in a watcher env — read
                // the current payload through the retained cell (getter
                // keyed by the variable's type; refs BORROW).
                if self.boxed_hoisted.contains(name) {
                    if let Some((env_var, _)) = self.hoisted_variables.get(name) {
                        let ty = self.variable_types.get(name).cloned().unwrap_or(Type::I32);
                        let getter = Self::slot_fns_for(&ty).map(|(_, g, _)| g).unwrap_or("hl_scalar_get_i32");
                        self.output.push_str(&format!("{}({}->{})", getter, env_var, name));
                        return Ok(());
                    }
                }

                // Check if this variable is hoisted to an environment
                if let Some((env_var, env_struct)) = self.hoisted_variables.get(name) {
                    // Variable is hoisted - use environment access
                    let is_array_watcher_env = env_struct.contains("array_watcher_env");

                    if is_array_watcher_env {
                        // Container captures (arrays; objects as of Phase 2e)
                        // are stored directly; legacy scalars by pointer
                        let is_array_type = self.variable_types.get(name)
                            .map(|ty| matches!(ty, Type::DynamicArray(_) | Type::Object(_)))
                            .unwrap_or(false);

                        if is_array_type {
                            // Containers are stored directly, no dereferencing needed
                            self.output.push_str(&format!("{}->", env_var));
                        } else {
                            // Scalars are stored as pointers, need dereferencing
                            self.output.push_str("(*");
                            self.output.push_str(&format!("{}->", env_var));
                        }
                    } else {
                        self.output.push_str(&format!("{}->", env_var));
                    }

                    if let Some(ref refined) = refined_type {
                        // If the variable is narrowed, emit unwrap for the refined type
                        let types_refined = Type::from_ast_type(refined);
                        self.emit_refined_variable_access(name, &types_refined)?;
                    } else {
                        self.output.push_str(name);
                    }

                    if is_array_watcher_env && !self.variable_types.get(name)
                        .map(|ty| matches!(ty, Type::DynamicArray(_) | Type::Object(_)))
                        .unwrap_or(false) {
                        // Close the dereference paren for scalars
                        self.output.push_str(")");
                    }
                } else if self.current_binding_boxed(name) {
                    // Phase 3b/3e: boxed cell local (or static-hoisted
                    // program cell) — deref the payload (getter keyed by
                    // type; refs BORROW)
                    let ty = self.variable_types.get(name).cloned().unwrap_or(Type::I32);
                    let getter = Self::slot_fns_for(&ty).map(|(_, g, _)| g).unwrap_or("hl_scalar_get_i32");
                    self.output.push_str(&format!("{}({})", getter, self.mangle_variable_name(name)));
                } else {
                    // Normal variable reference
                    if let Some(ref refined) = refined_type {
                        // If the variable is narrowed, emit unwrap for the refined type
                        let types_refined = Type::from_ast_type(refined);
                        self.emit_refined_variable_access(name, &types_refined)?;
                    } else {
                        let c_var_name = self.mangle_variable_name(name);
                        self.output.push_str(&c_var_name);
                    }
                }
            }
            Expression::This(_) => {
                // For now, emit this_obj directly
                // TODO: This should only be valid in method contexts
                self.output.push_str("this_obj");
            }
            Expression::BinaryOp(binary_op) => {
                self.generate_binary_op(binary_op, type_checker, context)?;
            }
            Expression::UnaryOp(unary_op) => {
                self.generate_unary_op(unary_op, type_checker)?;
            }
            Expression::Call(call) => {
                self.generate_call(call, type_checker, context)?;
            }
            Expression::MemberAccess(member_access) => {
                self.generate_member_access(member_access, type_checker, context)?;
            }
            Expression::IndexAccess(index_access) => {
                // Infer element type from the array or string type
                let array_type = self.infer_expression_type_for_codegen(&index_access.object);
                let elem_type = match array_type {
                    Type::DynamicArray(inner) => *inner,
                    Type::String => Type::U8, // String indexing returns u8 bytes
                    _ => {
                        return Err(CodegenError::UnsupportedFeature {
                            feature: "index access on non-array types".to_string(),
                            phase: "only arrays and strings supported".to_string(),
                        });
                    }
                };

                let elem_c_type = self.hilow_type_to_c(&elem_type);

                // Generate (*(ELEM_C_TYPE*)hl_array_get(arr_expr, index_expr))
                self.output.push_str(&format!("(*({}*)hl_array_get(", elem_c_type));
                self.generate_expression(&index_access.object, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&index_access.index, type_checker, ExprContext::Temporary)?;
                self.output.push_str("))");
            }
            Expression::IsCheck(is_check) => {
                self.generate_is_check(is_check, type_checker)?;
            }
            Expression::ObjectIsCheck(obj_is_check) => {
                self.generate_object_is_check(obj_is_check, type_checker)?;
            }
            Expression::QualifiedOp(qualified_op) => {
                self.generate_qualified_op_expression(qualified_op, type_checker)?;
            }
            Expression::ObjectLiteral(obj_lit) => {
                self.generate_object_literal(obj_lit, type_checker)?;
            }
            Expression::FunctionExpr(func_expr) => {
                self.generate_function_expression(func_expr, type_checker)?;
            }
            Expression::Match(match_expr) => {
                self.generate_match_expression(match_expr, type_checker, context)?;
            }
            Expression::WeakRef(expr, _) => {
                // For Phase 8c, weak references generate the same code as the underlying expression
                // The weak behavior is handled at the assignment level
                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
            }
            Expression::Nothing(_) => {
                // Emit reference to the global nothing singleton
                self.output.push_str("&the_nothing");
            }
            Expression::Unknown(unknown_construction) => {
                self.generate_unknown_constructor(unknown_construction, type_checker)?;
            }
            Expression::TupleLit(elements, _) => {
                // Infer element types and generate struct initializer
                let mut element_types = Vec::new();
                for element in elements {
                    let element_type = self.infer_expression_type_for_codegen(element);
                    element_types.push(element_type);
                }

                // Ensure the tuple struct exists
                self.ensure_tuple_struct(&element_types);
                let struct_name = self.get_tuple_type_name(&element_types);

                // Generate struct initializer
                self.output.push_str(&format!("(({}) {{ ", struct_name));
                for (i, element) in elements.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    // Use Owned context for heap-allocated elements since they become owned by the tuple
                    let element_type = self.infer_expression_type_for_codegen(element);
                    let element_context = match element_type {
                        Type::String | Type::Object(_) | Type::Function(_, _) | Type::DynamicArray(_) |
                        Type::Optional(_) | Type::UnknownType => ExprContext::Owned,
                        _ => ExprContext::Temporary,
                    };
                    // Phase 1.5c: the tuple owns each heap element, so a
                    // BORROWED element must be retained (string member reads
                    // are already +1 from hl_object_get_str, so String only
                    // treats idents as borrowed)
                    let ref_wrap = match &element_type {
                        Type::Object(_) if Self::expr_is_borrowed_ref(element) => Some("hl_object_ref"),
                        Type::Function(_, _) if Self::expr_is_borrowed_ref(element) => Some("hl_function_ref"),
                        Type::String if matches!(element, Expression::Ident { .. }) => Some("hl_array_ref"),
                        Type::DynamicArray(_)
                            if matches!(element, Expression::Ident { .. })
                                || matches!(element, Expression::IndexAccess(_)) =>
                        {
                            Some("hl_array_ref")
                        }
                        _ => None,
                    };
                    if let Some(ref_fn) = ref_wrap {
                        self.output.push_str(&format!("{}(", ref_fn));
                    }
                    self.generate_expression(element, type_checker, element_context)?;
                    if ref_wrap.is_some() {
                        self.output.push_str(")");
                    }
                }
                self.output.push_str(" })");
            }
            Expression::TupleAccess(tuple_expr, index, _) => {
                // Generate struct field access
                self.generate_expression(tuple_expr, type_checker, ExprContext::Temporary)?;
                self.output.push_str(&format!("._{}", index));
            }
            Expression::WatcherExpr(watcher_expr) => {
                // Phase 3b: every subscription is a cell now. Watchable:
                // i32 scalars (the corpus payload kind), arrays, objects.
                // Other scalar kinds get a payload as programs need them.
                for subscription in &watcher_expr.subscriptions {
                    let var_name = &subscription.variable_name;
                    if let Some(var_type) = subscription.resolved_var_type.borrow().as_ref() {
                        // Phase 3e-α: strings are watchable — the variable
                        // lowers to a slot cell (rebinding-watch).
                        if !self.is_ast_type_watchable_in_phase_10g(var_type)
                            && !matches!(var_type, crate::ast::Type::Object(_))
                            && !matches!(var_type, crate::ast::Type::Primitive(crate::ast::PrimitiveType::String))
                        {
                            return Err(CodegenError::UnsupportedFeature {
                                feature: format!("watching a scalar of type {:?}", var_type),
                                phase: "a future phase — boxed scalar payload kinds land as programs need them (i32 today; audit §5 item 7)".to_string(),
                            });
                        }
                    } else {
                        return Err(CodegenError::UnsupportedFeature {
                            feature: format!("subscription to '{}' with no resolved type", var_name),
                            phase: "internal error - type checker should have populated this".to_string(),
                        });
                    }
                }

                // Validate modifiers
                for subscription in &watcher_expr.subscriptions {
                    let is_array = subscription.resolved_var_type
                        .borrow()
                        .as_ref()
                        .map(|ty| matches!(ty, crate::ast::Type::DynamicArray(_)))
                        .unwrap_or(false);
                    let is_object = subscription.resolved_var_type
                        .borrow()
                        .as_ref()
                        .map(|ty| matches!(ty, crate::ast::Type::Object(_)))
                        .unwrap_or(false);

                    match subscription.modifier {
                        SubscriptionModifier::Changed | SubscriptionModifier::Assigned => {
                            // Changed: all watchable types. Assigned on
                            // objects is rejected at typecheck (Phase 2e).
                        }
                        SubscriptionModifier::Added | SubscriptionModifier::Removed => {
                            // Phase 10-ε-β: arrays only (objects rejected at
                            // typecheck — Phase 2e)
                            if !is_array {
                                return Err(CodegenError::UnsupportedFeature {
                                    feature: format!("watcher modifier {:?} on non-array type", subscription.modifier),
                                    phase: "added/removed watching only applies to arrays".to_string(),
                                });
                            }
                        }
                        SubscriptionModifier::Moved => {
                            // Phase 10-ε-γ: Moved now supported for arrays
                            if !is_array {
                                return Err(CodegenError::UnsupportedFeature {
                                    feature: format!("watcher modifier {:?} on non-array type", subscription.modifier),
                                    phase: "moved watching only applies to arrays".to_string(),
                                });
                            }
                        }
                        SubscriptionModifier::Deep => {
                            // Phase 2d arrays; Phase 2e objects
                            if !is_array && !is_object {
                                return Err(CodegenError::UnsupportedFeature {
                                    feature: format!("watcher modifier {:?} on non-container type", subscription.modifier),
                                    phase: "deep watching applies to arrays and objects until scalars gain cells (Phase 3)".to_string(),
                                });
                            }
                        }
                    }
                }

                // Classify subscriptions: container VALUE cells (arrays
                // Phase 2a, objects Phase 2e) vs slot/scalar cells. Phase
                // 3e-α: a SLOT-KIND subscription — (assigned) on anything,
                // or any subscription on a string — routes down the
                // scalar/slot path regardless of the variable's type
                // (audit §5 item 10b: (assigned) subscribes the slot).
                let mut has_arrays = false;
                let mut has_objects = false;
                let mut has_scalars = false;
                for subscription in &watcher_expr.subscriptions {
                    if let Some(ast_var_type) = subscription.resolved_var_type.borrow().as_ref() {
                        let slot_kind = matches!(subscription.modifier, SubscriptionModifier::Assigned)
                            || matches!(ast_var_type, crate::ast::Type::Primitive(crate::ast::PrimitiveType::String));
                        if slot_kind {
                            has_scalars = true;
                        } else if matches!(ast_var_type, crate::ast::Type::DynamicArray(_)) {
                            has_arrays = true;
                        } else if matches!(ast_var_type, crate::ast::Type::Object(_)) {
                            has_objects = true;
                        } else {
                            has_scalars = true;
                        }
                    }
                }
                let has_cells = has_arrays || has_objects;

                if has_cells && has_scalars {
                    return Err(CodegenError::UnsupportedFeature {
                        feature: "mixed scalar and container subscriptions in one watcher".to_string(),
                        phase: "Phase 10-ε-α supports only pure container or pure scalar watchers".to_string(),
                    });
                }

                // Phase 2e: the body prologue casts the fired cell to the
                // FIRST subscription's container type, which is unsound when
                // a watcher mixes arrays and objects — rejected until a typed
                // rebind per subscription exists.
                if has_arrays && has_objects {
                    return Err(CodegenError::UnsupportedFeature {
                        feature: "mixed array and object subscriptions in one watcher".to_string(),
                        phase: "a future phase (needs a per-subscription typed rebind; Phase 2e watchers are single-container-kind)".to_string(),
                    });
                }

                // Generate unique body function name
                let body_fn_name = format!("hilow_watcher_expr_{}_body", self.watcher_counter);
                // Captured now: generate_block below can bump the counter for
                // nested watcher expressions, so `watcher_counter - 1` is not
                // safe to recompute after body generation.
                let watcher_index = self.watcher_counter;
                self.watcher_counter += 1;

                // One firing ABI for EVERY watcher (Phase 2c containers,
                // Phase 3b scalars): env-first, cell second, value delta
                // third. Container bodies rebind the watched name from the
                // fired cell; scalar bodies snapshot subscribed values from
                // the env's retained cells.
                self.watcher_bodies.push_str(&format!("void {}(", body_fn_name));
                self.watcher_bodies.push_str("void* env, HiLowCell* hilow_cell, const HiLowDelta* delta");
                self.watcher_bodies.push_str(") {\n");

                // Phase 2c: rebind the watched array name from the cell, then
                // for added/removed/moved with alias, bind from the value
                // delta (copied at body entry, so aliases survive nested
                // mutation exactly as before).
                if has_cells {
                    let first_subscription = &watcher_expr.subscriptions[0];
                    // Phase 2e: cast to the first subscription's container
                    // type (mixed array+object watchers are rejected above,
                    // so the cast is uniform across all subscriptions)
                    let container_c_type = if has_objects { "HiLowObject*" } else { "HiLowArray*" };
                    self.watcher_bodies.push_str(&format!(
                        "    {} {} = ({})hilow_cell;\n",
                        container_c_type, first_subscription.variable_name, container_c_type
                    ));
                    for subscription in &watcher_expr.subscriptions {
                        if matches!(subscription.modifier, SubscriptionModifier::Added | SubscriptionModifier::Removed) {
                            if let Some(ref alias_name) = subscription.alias {
                                if let Some(alias_type) = subscription.resolved_alias_type.borrow().as_ref() {
                                    let c_elem_type = self.ast_type_to_c(alias_type);
                                    self.watcher_bodies.push_str(&format!(
                                        "    {} {} = *({} *)delta->payload;\n",
                                        c_elem_type, alias_name, c_elem_type
                                    ));
                                }
                            }
                        } else if matches!(subscription.modifier, SubscriptionModifier::Moved) {
                            if let Some(ref alias_name) = subscription.alias {
                                // Moved alias is typed as Tuple(Usize, Usize); built from delta->from/to
                                self.watcher_bodies.push_str(&format!(
                                    "    HiLowMovedDelta {} = {{ ._0 = delta->from, ._1 = delta->to }};\n",
                                    alias_name
                                ));
                            }
                        }
                    }
                }

                // Phase 3b: compute the env field list BEFORE entering the
                // body context (boxedness of captured names is a property of
                // the ENCLOSING scope). Containers keep the legacy capture
                // list (its phantom subscribed-container entries are the
                // functional rebind mechanism for multi-subscription
                // watchers); scalars use the shadow-correct 3a analysis.
                let env_struct_name = format!("hilow_array_watcher_env_{}", watcher_index);
                let mut env_fields: Vec<(String, EnvSlot)> = Vec::new();
                if has_cells {
                    for (var_name, ast_type, _pos) in watcher_expr.captures.borrow().iter() {
                        let slot = match ast_type {
                            // Phase 3e-α: slot-boxed variables are
                            // HiLowScalar* whatever their HiLow type.
                            _ if self.current_binding_boxed(var_name) => Some(EnvSlot::Scalar),
                            crate::ast::Type::DynamicArray(_) => Some(EnvSlot::Array),
                            crate::ast::Type::Object(_) => Some(EnvSlot::Object),
                            _ => {
                                if self.current_binding_boxed(var_name)
                                    || self.boxed_hoisted.contains(var_name)
                                {
                                    Some(EnvSlot::Scalar)
                                } else {
                                    // A legacy-scan phantom (the 2e finding:
                                    // body-local shadows recorded as captures)
                                    // — the body never reads the outer
                                    // binding, so no slot.
                                    None
                                }
                            }
                        };
                        if let Some(slot) = slot {
                            env_fields.push((var_name.clone(), slot));
                        }
                    }
                } else {
                    // Subscribed cells first (deduplicated), then captures.
                    for subscription in &watcher_expr.subscriptions {
                        let var_name = &subscription.variable_name;
                        if !env_fields.iter().any(|(n, _)| n == var_name) {
                            env_fields.push((var_name.clone(), EnvSlot::Scalar));
                        }
                    }
                    let analysis_captures: Vec<String> = self
                        .boxing
                        .as_ref()
                        .map(|b| b.captures_for(&watcher_expr.position).to_vec())
                        .unwrap_or_default();
                    for var_name in &analysis_captures {
                        if env_fields.iter().any(|(n, _)| n == var_name) {
                            continue;
                        }
                        let var_type = self.variable_types.get(var_name).cloned();
                        // Phase 3e-α: a slot-boxed variable is a HiLowScalar*
                        // whatever its HiLow type — the representation decides
                        // the env slot kind.
                        let slot = if self.current_binding_boxed(var_name) {
                            EnvSlot::Scalar
                        } else {
                            match var_type {
                                Some(Type::DynamicArray(_)) | Some(Type::String) => EnvSlot::Array,
                                Some(Type::Object(_)) => EnvSlot::Object,
                                Some(Type::I32) => EnvSlot::Scalar,
                                other => {
                                    return Err(CodegenError::UnsupportedFeature {
                                        feature: format!(
                                            "watcher capture of '{}' with type {:?}",
                                            var_name, other
                                        ),
                                        phase: "a future phase — boxed scalar payload kinds land as programs need them (i32 today)".to_string(),
                                    });
                                }
                            }
                        };
                        env_fields.push((var_name.clone(), slot));
                    }
                }
                let env_dtor_name = if env_fields.is_empty() {
                    None
                } else {
                    Some(self.emit_watcher_env_struct(&env_struct_name, &env_fields))
                };

                // Generate the watcher body
                let saved_output = self.output.clone();
                self.output.clear();

                // Save current variable_types and add watcher parameters to scope
                let old_variable_types = self.variable_types.clone();

                // Save hoisted variables state (before any modification)
                let old_hoisted_variables = self.hoisted_variables.clone();
                let old_current_env_var = self.current_env_var.clone();
                let old_boxed_hoisted = self.boxed_hoisted.clone();
                let boxed_mask_base = self.boxed_bindings.len();

                if has_cells {
                    // Container watcher: add the container parameter to scope
                    // Parameter is always the variable name; alias (if any) binds to delta element
                    let first_subscription = &watcher_expr.subscriptions[0];
                    let param_name = &first_subscription.variable_name;
                    if let Some(ast_var_type) = first_subscription.resolved_var_type.borrow().as_ref() {
                        let types_var_type = Type::from_ast_type(ast_var_type);
                        self.variable_types.insert(param_name.clone(), types_var_type);
                    }

                    // Also register aliases for added/removed/moved subscriptions (mirrors cast-emission loop above)
                    for subscription in &watcher_expr.subscriptions {
                        if matches!(subscription.modifier, SubscriptionModifier::Added | SubscriptionModifier::Removed) {
                            if let Some(ref alias_name) = subscription.alias {
                                if let Some(alias_type) = subscription.resolved_alias_type.borrow().as_ref() {
                                    let types_alias_type = Type::from_ast_type(alias_type);
                                    self.variable_types.insert(alias_name.clone(), types_alias_type);
                                }
                            }
                        } else if matches!(subscription.modifier, SubscriptionModifier::Moved) {
                            if let Some(ref alias_name) = subscription.alias {
                                // Moved alias is typed as Tuple(Usize, Usize) for .0/.1 access
                                let tuple_type = Type::Tuple(vec![Type::Usize, Type::Usize]);
                                self.variable_types.insert(alias_name.clone(), tuple_type);
                            }
                        }
                    }

                    if !env_fields.is_empty() {
                        // Set up hoisted variables for captured variables (using env_cast)
                        self.hoisted_variables.clear();
                        self.current_env_var = Some("env_cast".to_string());

                        for (var_name, slot) in &env_fields {
                            self.hoisted_variables.insert(var_name.clone(), ("env_cast".to_string(), env_struct_name.clone()));
                            match slot {
                                EnvSlot::Scalar => {
                                    self.boxed_hoisted.insert(var_name.clone());
                                    // Phase 3e-α: keep the variable's real
                                    // type (getter keyed on it); i32 only as
                                    // the untyped fallback.
                                    if !self.variable_types.contains_key(var_name) {
                                        self.variable_types.insert(var_name.clone(), Type::I32);
                                    }
                                }
                                _ => {
                                    // Container types were recorded by the
                                    // legacy captures list; keep them.
                                    if let Some((_, ast_type, _)) = watcher_expr.captures.borrow().iter().find(|(n, _, _)| n == var_name) {
                                        self.variable_types.insert(var_name.clone(), Type::from_ast_type(ast_type));
                                    }
                                }
                            }
                        }

                        // Emit env cast at the beginning of the body generation (before body statements)
                        self.output.push_str(&format!("  {}* env_cast = ({}*)env;\n",
                            env_struct_name, env_struct_name));
                    }
                } else {
                    // Phase 3b scalar watcher: every subscribed/captured cell
                    // lives in the env. Subscribed names bind SNAPSHOT locals
                    // read from the cells at fire time (assignment to them in
                    // the body writes the snapshot, not the cell — the
                    // pre-queue no-re-entrancy semantics, unchanged);
                    // captures go through the env as cells.
                    self.output.push_str(&format!("  {}* env_cast = ({}*)env;\n",
                        env_struct_name, env_struct_name));
                    self.output.push_str("  (void)hilow_cell; (void)delta;\n");

                    for subscription in &watcher_expr.subscriptions {
                        let var_name = &subscription.variable_name;
                        let param_name = subscription.alias.as_ref().unwrap_or(var_name);
                        // Phase 3e-α: snapshot type follows the subscribed
                        // variable's type (refs bind a BORROW of the payload
                        // at fire time).
                        let sub_ty = subscription
                            .resolved_var_type
                            .borrow()
                            .as_ref()
                            .map(Type::from_ast_type)
                            .unwrap_or(Type::I32);
                        let (c_ty, getter) = match &sub_ty {
                            Type::String => ("HiLowArray*", "hl_scalar_get_str"),
                            Type::DynamicArray(_) => ("HiLowArray*", "hl_scalar_get_array_ref"),
                            Type::Object(_) => ("HiLowObject*", "hl_scalar_get_object_ref"),
                            _ => ("int32_t", "hl_scalar_get_i32"),
                        };
                        self.output.push_str(&format!(
                            "  {} {} = {}(env_cast->{});\n",
                            c_ty, param_name, getter, var_name
                        ));
                        self.variable_types.insert(param_name.clone(), sub_ty);
                        // Mask any outer boxed binding of this name: inside
                        // the body it is a raw snapshot local.
                        self.push_boxed_binding(param_name, false);
                    }

                    self.hoisted_variables.clear();
                    self.current_env_var = Some("env_cast".to_string());
                    for (var_name, slot) in &env_fields {
                        if watcher_expr.subscriptions.iter().any(|s| &s.variable_name == var_name) {
                            continue; // subscribed names bind snapshots, not env reads
                        }
                        self.hoisted_variables.insert(var_name.clone(), ("env_cast".to_string(), env_struct_name.clone()));
                        match slot {
                            EnvSlot::Scalar => {
                                self.boxed_hoisted.insert(var_name.clone());
                                // Phase 3e-α: keep the variable's real type —
                                // the env read keys its getter on it. i32
                                // only as the fallback for untyped names.
                                if !self.variable_types.contains_key(var_name) {
                                    self.variable_types.insert(var_name.clone(), Type::I32);
                                }
                            }
                            EnvSlot::Array => {
                                if !self.variable_types.contains_key(var_name) {
                                    self.variable_types.insert(var_name.clone(), Type::DynamicArray(Box::new(Type::I32)));
                                }
                            }
                            EnvSlot::Object => {}
                        }
                    }
                }

                self.generate_block(&watcher_expr.body, type_checker)?;

                // Restore enclosing-scope state
                self.variable_types = old_variable_types;
                self.hoisted_variables = old_hoisted_variables;
                self.current_env_var = old_current_env_var;
                self.boxed_hoisted = old_boxed_hoisted;
                self.boxed_bindings.truncate(boxed_mask_base);

                self.watcher_bodies.push_str(&self.output);
                self.watcher_bodies.push_str("}\n\n");

                // Restore output and emit the watcher-value creation
                self.output = saved_output;

                // Phase 2a/3b: registration by construction, ONE path for
                // every watcher kind. The expression value is ONE call that
                // creates the watcher AND subscribes it to every cell — any
                // syntactic position registers. The env owns a retain on
                // every cell it holds; its generated dtor releases them on
                // the watcher's final release (escape soundness, §5 item 1).
                let env_var_name = if env_fields.is_empty() {
                    "NULL".to_string()
                } else {
                    let env_var = format!("__watcher_env_{}", watcher_index);
                    let decl = self.watcher_env_pack_decl(&env_struct_name, &env_var, &env_fields);
                    self.pending_statement_decls.push(decl);
                    env_var
                };
                let dtor_arg = env_dtor_name.as_deref().unwrap_or("NULL").to_string();

                // Build the construction call.
                let mut call = format!("hl_watcher_new_subscribed((void*){}, {}, {}", body_fn_name, env_var_name, dtor_arg);
                let mut n_subs = 0;
                let mut sub_args = String::new();
                for subscription in &watcher_expr.subscriptions {
                    let c_modifier = match subscription.modifier {
                        // Phase 3b: (assigned) is the scalar every-assignment
                        // event; (changed) fires only on inequality.
                        SubscriptionModifier::Changed => "HL_ARR_CHANGED",
                        SubscriptionModifier::Assigned => "HL_SCALAR_ASSIGNED",
                        SubscriptionModifier::Added => "HL_ARR_ADDED",
                        SubscriptionModifier::Removed => "HL_ARR_REMOVED",
                        SubscriptionModifier::Moved => "HL_ARR_MOVED",
                        SubscriptionModifier::Deep => "HL_ARR_DEEP",
                    };
                    let mut cell_var = self.env_slot_rvalue(&subscription.variable_name);
                    // Phase 3e-α: a VALUE-kind subscription on a slot-boxed
                    // variable subscribes the CURRENT value's cell (identity
                    // at construction, per audit §5 item 10b) — deref the
                    // slot payload. Slot-kind subscriptions ((assigned),
                    // strings) subscribe the slot itself.
                    let sub_is_slot_kind = matches!(subscription.modifier, SubscriptionModifier::Assigned)
                        || matches!(
                            subscription.resolved_var_type.borrow().as_ref(),
                            Some(crate::ast::Type::Primitive(crate::ast::PrimitiveType::String))
                        );
                    if !sub_is_slot_kind && self.current_binding_boxed(&subscription.variable_name) {
                        // Only CONTAINER value subscriptions deref — a boxed
                        // scalar's slot IS its value cell.
                        let getter = match subscription.resolved_var_type.borrow().as_ref() {
                            Some(crate::ast::Type::Object(_)) => Some("hl_scalar_get_object_ref"),
                            Some(crate::ast::Type::DynamicArray(_)) => Some("hl_scalar_get_array_ref"),
                            _ => None,
                        };
                        if let Some(getter) = getter {
                            cell_var = format!("{}({})", getter, cell_var);
                        }
                    }
                    if matches!(subscription.modifier, SubscriptionModifier::Deep) {
                        // Phase 2d/2e: a (deep) subscription marks the
                        // whole subtree deep-watched so nested mutations
                        // walk up. The mark call is container-typed.
                        let mark_fn = if has_objects { "hl_object_mark_deep" } else { "hl_array_mark_deep" };
                        self.pending_statement_decls.push(format!("{}({});", mark_fn, cell_var));
                    }
                    sub_args.push_str(&format!(", &{}->cell, {}", cell_var, c_modifier));
                    n_subs += 1;
                }
                call.push_str(&format!(", {}{})", n_subs, sub_args));

                if context == ExprContext::Temporary {
                    // The fresh +1 watcher must be released at statement
                    // end (which unsubscribes it) — hoist as a tracked
                    // temp, mirroring the optional-member pattern.
                    let temp_name = self.next_temp_name();
                    self.temp_owners.insert(temp_name.clone(), HeapType::Watcher);
                    self.pending_statement_decls.push(format!("HiLowWatcher* {} = {};", temp_name, call));
                    self.output.push_str(&temp_name);
                } else {
                    self.output.push_str(&call);
                }
            }
            Expression::ArrayLit(elements, _) => {
                // Infer element type from first element
                let elem_type = if !elements.is_empty() {
                    self.infer_expression_type_for_codegen(&elements[0])
                } else {
                    // For empty arrays, if type checking passed, it means there was type context.
                    // Use a reasonable default element type.
                    Type::I32
                };

                let elem_c_type = self.hilow_type_to_c(&elem_type);
                let elem_size = format!("sizeof({})", elem_c_type);
                let initial_capacity = elements.len();

                // Determine retain/release function pointers based on element type
                let (retain_fn, release_fn) = match &elem_type {
                    Type::Object(_) => ("(void(*)(void*))hl_object_retain", "(void(*)(void*))hl_object_release"),
                    Type::DynamicArray(_) => ("(void(*)(void*))hl_array_retain", "(void(*)(void*))hl_array_release"),
                    _ => ("NULL", "NULL"), // Primitive types
                };

                // Use GCC statement-expression for inline array construction
                self.output.push_str(&format!("({{ HiLowArray* __arr = hl_array_new({}, {}, {}, {});\n", elem_size, initial_capacity, retain_fn, release_fn));

                // Push each element
                for (i, element) in elements.iter().enumerate() {
                    self.output.push_str(&format!("     {} __e{} = ", elem_c_type, i));
                    self.generate_expression(element, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(&format!("; hl_array_push(__arr, &__e{});\n", i));
                }

                // Phase 4a: hl_array_push retained each element for the array.
                // The element's own +1 is disposed by the ONE temp mechanism —
                // fresh-production elements (object/array literals, etc.) mint
                // statement-scoped temps released at statement end; borrowed
                // locals belong to their scope cleanup; tracked temps release
                // at statement end. No site release here.

                self.output.push_str("     __arr; })");
            }
            Expression::TypeAscription(inner, ascribed_ty, _) => {
                // Type ascription: expr : Type
                // Handle special cases, otherwise just generate the inner expression

                // Special case: empty array literal with ascription
                if let Expression::ArrayLit(elements, _) = inner.as_ref() {
                    if elements.is_empty() {
                        if let crate::ast::Type::DynamicArray(elem_ast_type) = ascribed_ty {
                            let elem_type = Type::from_ast_type(elem_ast_type);
                            let elem_c_type = self.hilow_type_to_c(&elem_type);
                            let elem_size = format!("sizeof({})", elem_c_type);
                            let initial_capacity = 4; // Small initial capacity for empty arrays

                            // Determine retain/release function pointers based on element type
                            let (retain_fn, release_fn) = match &elem_type {
                                Type::Object(_) => ("(void(*)(void*))hl_object_retain", "(void(*)(void*))hl_object_release"),
                                Type::DynamicArray(_) => ("(void(*)(void*))hl_array_retain", "(void(*)(void*))hl_array_release"),
                                _ => ("NULL", "NULL"), // Primitive types
                            };

                            self.output.push_str(&format!("hl_array_new({}, {}, {}, {})",
                                                         elem_size, initial_capacity, retain_fn, release_fn));
                            return Ok(());
                        }
                    }
                    // Non-empty array: generate normally
                    self.generate_expression(inner, type_checker, ExprContext::Temporary)?;
                    return Ok(());
                }

                // For numeric literals with ascription, emit with the ascribed type
                if let Expression::IntLit(value, _) = inner.as_ref() {
                    if let crate::ast::Type::Primitive(ref prim) = ascribed_ty {
                        match prim {
                            crate::ast::PrimitiveType::I64 => self.output.push_str(&format!("((int64_t){})", value)),
                            crate::ast::PrimitiveType::U64 => self.output.push_str(&format!("((uint64_t){})", value)),
                            _ => {
                                // Default case - just emit the value
                                self.output.push_str(&value.to_string());
                            }
                        }
                        return Ok(());
                    }
                }

                // Default case: just generate the inner expression (ascription is compile-time only)
                self.generate_expression(inner, type_checker, ExprContext::Temporary)?;
            }
        }
        Ok(())
    }

    fn generate_binary_op(&mut self, binary_op: &BinaryOp, type_checker: &TypeChecker, context: ExprContext) -> Result<(), CodegenError> {
        // Check for time/duration arithmetic that needs runtime function calls
        let lhs_type = self.infer_expression_type_for_codegen(&binary_op.lhs);
        let rhs_type = self.infer_expression_type_for_codegen(&binary_op.rhs);

        // Handle time/duration special cases
        match (&lhs_type, &rhs_type, &binary_op.op) {
            // Time arithmetic
            (Type::Time, Type::Duration, BinaryOpKind::Add) => {
                self.output.push_str("hl_time_add_duration(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Duration, Type::Time, BinaryOpKind::Add) => {
                self.output.push_str("hl_time_add_duration(");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Time, Type::Duration, BinaryOpKind::Sub) => {
                self.output.push_str("hl_time_sub_duration(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Time, Type::Time, BinaryOpKind::Sub) => {
                self.output.push_str("hl_time_sub_time(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Duration, Type::Duration, BinaryOpKind::Add) => {
                self.output.push_str("hl_duration_add(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }

            // Time comparisons
            (Type::Time, Type::Time, BinaryOpKind::Eq) => {
                self.output.push_str("hl_time_eq(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Time, Type::Time, BinaryOpKind::NotEq) => {
                self.output.push_str("hl_time_ne(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Time, Type::Time, BinaryOpKind::Less) => {
                self.output.push_str("hl_time_lt(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Time, Type::Time, BinaryOpKind::LessEq) => {
                self.output.push_str("hl_time_le(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Time, Type::Time, BinaryOpKind::Greater) => {
                self.output.push_str("hl_time_gt(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Time, Type::Time, BinaryOpKind::GreaterEq) => {
                self.output.push_str("hl_time_ge(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Time, Type::Time, BinaryOpKind::NotLess) => {
                self.output.push_str("hl_time_ge(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Time, Type::Time, BinaryOpKind::NotGreater) => {
                self.output.push_str("hl_time_le(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }

            // Duration comparisons
            (Type::Duration, Type::Duration, BinaryOpKind::Eq) => {
                self.output.push_str("hl_duration_eq(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Duration, Type::Duration, BinaryOpKind::NotEq) => {
                self.output.push_str("hl_duration_ne(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Duration, Type::Duration, BinaryOpKind::Less) => {
                self.output.push_str("hl_duration_lt(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Duration, Type::Duration, BinaryOpKind::LessEq) => {
                self.output.push_str("hl_duration_le(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Duration, Type::Duration, BinaryOpKind::Greater) => {
                self.output.push_str("hl_duration_gt(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Duration, Type::Duration, BinaryOpKind::GreaterEq) => {
                self.output.push_str("hl_duration_ge(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Duration, Type::Duration, BinaryOpKind::NotLess) => {
                self.output.push_str("hl_duration_ge(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Duration, Type::Duration, BinaryOpKind::NotGreater) => {
                self.output.push_str("hl_duration_le(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }

            // Money arithmetic
            (Type::Money, Type::Money, BinaryOpKind::Add) |
            (Type::MoneyOf(_), Type::MoneyOf(_), BinaryOpKind::Add) |
            (Type::Money, Type::MoneyOf(_), BinaryOpKind::Add) |
            (Type::MoneyOf(_), Type::Money, BinaryOpKind::Add) => {
                self.output.push_str("hl_money_add(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Money, Type::Money, BinaryOpKind::Sub) |
            (Type::MoneyOf(_), Type::MoneyOf(_), BinaryOpKind::Sub) |
            (Type::Money, Type::MoneyOf(_), BinaryOpKind::Sub) |
            (Type::MoneyOf(_), Type::Money, BinaryOpKind::Sub) => {
                self.output.push_str("hl_money_sub(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            // Money * scalar (need separate patterns to check is_numeric properly)
            (Type::Money, rhs_t, BinaryOpKind::Mul) if rhs_t.is_numeric() => {
                self.output.push_str("hl_money_mul_scalar(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::MoneyOf(_), rhs_t, BinaryOpKind::Mul) if rhs_t.is_numeric() => {
                self.output.push_str("hl_money_mul_scalar(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            // scalar * Money
            (lhs_t, Type::Money, BinaryOpKind::Mul) if lhs_t.is_numeric() => {
                self.output.push_str("hl_money_mul_scalar(");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (lhs_t, Type::MoneyOf(_), BinaryOpKind::Mul) if lhs_t.is_numeric() => {
                self.output.push_str("hl_money_mul_scalar(");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            // Money / scalar
            (Type::Money, rhs_t, BinaryOpKind::Div) if rhs_t.is_numeric() => {
                self.output.push_str("hl_money_div_scalar(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::MoneyOf(_), rhs_t, BinaryOpKind::Div) if rhs_t.is_numeric() => {
                self.output.push_str("hl_money_div_scalar(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            // Money / Money (same currency) → f64 ratio
            (Type::Money, Type::Money, BinaryOpKind::Div) |
            (Type::MoneyOf(_), Type::MoneyOf(_), BinaryOpKind::Div) |
            (Type::Money, Type::MoneyOf(_), BinaryOpKind::Div) |
            (Type::MoneyOf(_), Type::Money, BinaryOpKind::Div) => {
                self.output.push_str("hl_money_div_money(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }

            // Money comparisons
            (Type::Money, Type::Money, BinaryOpKind::Eq) |
            (Type::MoneyOf(_), Type::MoneyOf(_), BinaryOpKind::Eq) |
            (Type::Money, Type::MoneyOf(_), BinaryOpKind::Eq) |
            (Type::MoneyOf(_), Type::Money, BinaryOpKind::Eq) => {
                self.output.push_str("hl_money_eq(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Money, Type::Money, BinaryOpKind::NotEq) |
            (Type::MoneyOf(_), Type::MoneyOf(_), BinaryOpKind::NotEq) |
            (Type::Money, Type::MoneyOf(_), BinaryOpKind::NotEq) |
            (Type::MoneyOf(_), Type::Money, BinaryOpKind::NotEq) => {
                self.output.push_str("hl_money_ne(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Money, Type::Money, BinaryOpKind::Less) |
            (Type::MoneyOf(_), Type::MoneyOf(_), BinaryOpKind::Less) |
            (Type::Money, Type::MoneyOf(_), BinaryOpKind::Less) |
            (Type::MoneyOf(_), Type::Money, BinaryOpKind::Less) => {
                self.output.push_str("hl_money_lt(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Money, Type::Money, BinaryOpKind::LessEq) |
            (Type::MoneyOf(_), Type::MoneyOf(_), BinaryOpKind::LessEq) |
            (Type::Money, Type::MoneyOf(_), BinaryOpKind::LessEq) |
            (Type::MoneyOf(_), Type::Money, BinaryOpKind::LessEq) => {
                self.output.push_str("hl_money_le(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Money, Type::Money, BinaryOpKind::Greater) |
            (Type::MoneyOf(_), Type::MoneyOf(_), BinaryOpKind::Greater) |
            (Type::Money, Type::MoneyOf(_), BinaryOpKind::Greater) |
            (Type::MoneyOf(_), Type::Money, BinaryOpKind::Greater) => {
                self.output.push_str("hl_money_gt(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Money, Type::Money, BinaryOpKind::GreaterEq) |
            (Type::MoneyOf(_), Type::MoneyOf(_), BinaryOpKind::GreaterEq) |
            (Type::Money, Type::MoneyOf(_), BinaryOpKind::GreaterEq) |
            (Type::MoneyOf(_), Type::Money, BinaryOpKind::GreaterEq) => {
                self.output.push_str("hl_money_ge(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Money, Type::Money, BinaryOpKind::NotLess) |
            (Type::MoneyOf(_), Type::MoneyOf(_), BinaryOpKind::NotLess) |
            (Type::Money, Type::MoneyOf(_), BinaryOpKind::NotLess) |
            (Type::MoneyOf(_), Type::Money, BinaryOpKind::NotLess) => {
                self.output.push_str("hl_money_ge(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::Money, Type::Money, BinaryOpKind::NotGreater) |
            (Type::MoneyOf(_), Type::MoneyOf(_), BinaryOpKind::NotGreater) |
            (Type::Money, Type::MoneyOf(_), BinaryOpKind::NotGreater) |
            (Type::MoneyOf(_), Type::Money, BinaryOpKind::NotGreater) => {
                self.output.push_str("hl_money_le(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }

            // String equality comparisons
            (Type::String, Type::String, BinaryOpKind::Eq) => {
                self.output.push_str("hl_string_eq(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            (Type::String, Type::String, BinaryOpKind::NotEq) => {
                self.output.push_str("hl_string_ne(");
                self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(", ");
                self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }

            // String concatenation
            (Type::String, Type::String, BinaryOpKind::Add) => {
                if context == ExprContext::Temporary {
                    // Hoist temp declaration to statement scope for cleanup visibility
                    let temp_name = self.next_temp_name();

                    // Build the declaration with the hl_string_concat call
                    let mut decl = format!("HiLowArray* {} = hl_string_concat(", temp_name);

                    // Temporarily capture the arguments
                    let mut lhs_output = String::new();
                    let mut rhs_output = String::new();

                    let saved_output = std::mem::take(&mut self.output);

                    // Generate LHS
                    self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                    lhs_output = std::mem::take(&mut self.output);

                    // Generate RHS
                    self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                    rhs_output = std::mem::take(&mut self.output);

                    // Restore output
                    self.output = saved_output;

                    // Complete the declaration
                    decl.push_str(&lhs_output);
                    decl.push_str(", ");
                    decl.push_str(&rhs_output);
                    decl.push_str(");");

                    self.pending_statement_decls.push(decl);

                    // Track for statement-end cleanup
                    self.temp_owners.insert(temp_name.clone(), HeapType::Array);

                    // Emit just the reference
                    self.output.push_str(&temp_name);
                } else {
                    // Owned context - emit directly
                    self.output.push_str("hl_string_concat(");
                    self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(", ");
                    self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(")");
                }
                return Ok(());
            }

            _ => {
                // Fall through to regular binary operation
            }
        }

        // Regular binary operation
        self.output.push_str("(");
        self.generate_expression(&binary_op.lhs, type_checker, ExprContext::Temporary)?;

        let op_str = match binary_op.op {
            BinaryOpKind::Add => " + ",
            BinaryOpKind::Sub => " - ",
            BinaryOpKind::Mul => " * ",
            BinaryOpKind::Div => " / ",
            BinaryOpKind::Mod => " % ",
            BinaryOpKind::Less => " < ",
            BinaryOpKind::Greater => " > ",
            BinaryOpKind::LessEq => " <= ",
            BinaryOpKind::GreaterEq => " >= ",
            BinaryOpKind::Eq => " == ",
            BinaryOpKind::NotEq => " != ",
            BinaryOpKind::NotLess => " >= ",
            BinaryOpKind::NotGreater => " <= ",
            BinaryOpKind::And => " && ",
            BinaryOpKind::Or => " || ",
            BinaryOpKind::BitAnd => " & ",
            BinaryOpKind::BitOr => " | ",
            BinaryOpKind::BitXor => " ^ ",
            BinaryOpKind::ShiftLeft => " << ",
            BinaryOpKind::ShiftRight => " >> ",
            _ => {
                return Err(CodegenError::UnsupportedFeature {
                    feature: format!("binary operator {:?}", binary_op.op),
                    phase: "later phases".to_string(),
                });
            }
        };

        self.output.push_str(op_str);
        self.generate_expression(&binary_op.rhs, type_checker, ExprContext::Temporary)?;
        self.output.push_str(")");
        Ok(())
    }

    fn generate_unary_op(&mut self, unary_op: &UnaryOp, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        match unary_op.op {
            UnaryOpKind::Not => {
                // Special handling for not operator with nothing type
                let operand_type = self.infer_expression_type_for_codegen(&unary_op.operand);
                if matches!(operand_type, Type::Nothing) {
                    // not nothing should be true (since nothing is falsy)
                    self.output.push_str("true");
                } else {
                    // Regular not operator
                    self.output.push_str("!");
                    self.generate_expression(&unary_op.operand, type_checker, ExprContext::Temporary)?;
                }
            }
            UnaryOpKind::Neg => {
                self.output.push_str("-");
                self.generate_expression(&unary_op.operand, type_checker, ExprContext::Temporary)?;
            }
            UnaryOpKind::BitNot => {
                self.output.push_str("~");
                self.generate_expression(&unary_op.operand, type_checker, ExprContext::Temporary)?;
            }
        }
        Ok(())
    }

    fn generate_call(&mut self, call: &Call, type_checker: &TypeChecker, context: ExprContext) -> Result<(), CodegenError> {
        // Check if this is the special print() function
        if let Expression::Ident { name: func_name, .. } = call.callee.as_ref() {
            if func_name == "print" {
                return self.generate_print_call(call, type_checker);
            }
        }

        // Phase 11a-δ-α: cross-module names resolve to plain C functions
        if let Expression::Ident { name: func_name, .. } = call.callee.as_ref() {
            if let Some(mangled_name) = self.current_name_map.as_ref().and_then(|m| m.get(func_name)).cloned() {
                // Emit plain function call to mangled C name
                self.output.push_str(&mangled_name);
                self.output.push_str("(");
                for (i, arg) in call.args.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.generate_expression(arg, type_checker, ExprContext::Temporary)?;
                }
                self.output.push_str(")");
                return Ok(());
            }
        }

        // Check if callee is a function value (stored in a variable)
        if let Expression::Ident { name: var_name, .. } = call.callee.as_ref() {
            if let Some(var_type) = self.variable_types.get(var_name).cloned() {
                if let Type::Function(param_types, return_type) = var_type {
                    // This is a function value call - emit function pointer dispatch
                    return self.generate_function_value_call(call, &param_types, &return_type, type_checker);
                }
            }
        }

        // Check if callee is a member access returning a function (obj.fnProp)
        if let Expression::MemberAccess(member_access) = call.callee.as_ref() {
            let object_type = self.infer_expression_type_for_codegen(&member_access.object);
            if let Type::Object(_) = object_type {
                // Object method calls
                return self.generate_member_function_call(call, member_access, type_checker, context);
            } else if let Type::DynamicArray(elem_type) = object_type {
                // Array Phase B: Array method calls (.push, .pop)
                match member_access.member.as_str() {
                    "push" => {
                        // arr.push(x) -> hl_array_push(arr, &temp)
                        if call.args.len() != 1 {
                            return Err(CodegenError::UnsupportedFeature {
                                feature: "array.push() with wrong argument count".to_string(),
                                phase: "Array Phase B".to_string(),
                            });
                        }

                        // Generate temp variable for the argument (need lvalue for address)
                        let temp_var = format!("temp_{}", self.var_counter);
                        self.var_counter += 1;
                        let elem_c_type = self.hilow_type_to_c(&elem_type);

                        self.output.push_str("{\n");
                        self.output.push_str(&format!("    {} {} = ", elem_c_type, temp_var));
                        self.generate_expression(&call.args[0], type_checker, ExprContext::Temporary)?;
                        self.output.push_str(";\n");
                        self.output.push_str("    hl_array_push(");
                        self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;
                        self.output.push_str(&format!(", &{});\n", temp_var));
                        // Phase 4a: hl_array_push retained for the array; the
                        // argument's own +1 is disposed by the one temp
                        // mechanism (fresh productions mint statement-scoped
                        // temps; borrows are scope-owned), no site release.

                        self.output.push_str("}");
                        return Ok(());
                    }
                    "pop" => {
                        // arr.pop() -> (*(T*)hl_array_pop(arr))
                        if !call.args.is_empty() {
                            return Err(CodegenError::UnsupportedFeature {
                                feature: "array.pop() with arguments".to_string(),
                                phase: "Array Phase B".to_string(),
                            });
                        }

                        let elem_c_type = self.hilow_type_to_c(&elem_type);
                        self.output.push_str(&format!("(*({}*)hl_array_pop(", elem_c_type));
                        self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;
                        self.output.push_str("))");
                        return Ok(());
                    }
                    "remove" => {
                        // arr.remove(index) -> (hl_array_remove(arr, index, &tmp), tmp)
                        // Phase 2c: the removed element lands in a hoisted
                        // caller-owned temp, not the old static buffer.
                        if call.args.len() != 1 {
                            return Err(CodegenError::UnsupportedFeature {
                                feature: "array.remove() with wrong argument count".to_string(),
                                phase: "Array Phase B-2".to_string(),
                            });
                        }

                        let elem_c_type = self.hilow_type_to_c(&elem_type);
                        let removed_tmp = format!("hilow_removed_{}", self.temp_counter);
                        self.temp_counter += 1;
                        self.pending_statement_decls.push(format!("{} {};", elem_c_type, removed_tmp));
                        self.output.push_str("(hl_array_remove(");
                        self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;
                        self.output.push_str(", ");
                        self.generate_expression(&call.args[0], type_checker, ExprContext::Temporary)?;
                        self.output.push_str(&format!(", &{}), {})", removed_tmp, removed_tmp));
                        return Ok(());
                    }
                    "insert" => {
                        // arr.insert(index, elem) -> hl_array_insert(arr, index, &temp)
                        if call.args.len() != 2 {
                            return Err(CodegenError::UnsupportedFeature {
                                feature: "array.insert() with wrong argument count".to_string(),
                                phase: "Array Phase B-2".to_string(),
                            });
                        }

                        // Generate temp variable for the element (need lvalue for address)
                        let temp_var = format!("temp_{}", self.var_counter);
                        self.var_counter += 1;
                        let elem_c_type = self.hilow_type_to_c(&elem_type);

                        self.output.push_str("{\n");
                        self.output.push_str(&format!("    {} {} = ", elem_c_type, temp_var));
                        self.generate_expression(&call.args[1], type_checker, ExprContext::Temporary)?;
                        self.output.push_str(";\n");
                        self.output.push_str("    hl_array_insert(");
                        self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;
                        self.output.push_str(", ");
                        self.generate_expression(&call.args[0], type_checker, ExprContext::Temporary)?;
                        self.output.push_str(&format!(", &{});\n", temp_var));
                        // Phase 4a: hl_array_insert retained for the array; the
                        // element's own +1 is disposed by the one temp
                        // mechanism, no site release.

                        self.output.push_str("}");
                        return Ok(());
                    }
                    "move" => {
                        // arr.move(from, to) -> hl_array_move(arr, from, to)
                        if call.args.len() != 2 {
                            return Err(CodegenError::UnsupportedFeature {
                                feature: "array.move() with wrong argument count".to_string(),
                                phase: "Phase 10-ε-γ".to_string(),
                            });
                        }

                        self.output.push_str("hl_array_move(");
                        self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;
                        self.output.push_str(", ");
                        self.generate_expression(&call.args[0], type_checker, ExprContext::Temporary)?;  // from
                        self.output.push_str(", ");
                        self.generate_expression(&call.args[1], type_checker, ExprContext::Temporary)?;  // to
                        self.output.push_str(")");
                        return Ok(());
                    }
                    "clear" => {
                        // arr.clear() -> hl_array_clear(arr)
                        if !call.args.is_empty() {
                            return Err(CodegenError::UnsupportedFeature {
                                feature: "array.clear() with arguments".to_string(),
                                phase: "Array .clear()".to_string(),
                            });
                        }

                        self.output.push_str("hl_array_clear(");
                        self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;
                        self.output.push_str(")");
                        return Ok(());
                    }
                    _ => {
                        return Err(CodegenError::UnsupportedFeature {
                            feature: format!("unsupported array method '{}'", member_access.member),
                            phase: "Array Phase B".to_string(),
                        });
                    }
                }
            } else if let Expression::Ident { name, .. } = member_access.object.as_ref() {
                if name == "time" {
                    // Built-in time method calls
                    return self.generate_member_function_call(call, member_access, type_checker, context);
                }

                // Phase 3c: watcher method calls — both forms dispatch through
                // the runtime object (decl-form names are HiLowWatcher* vars).
                if let Some(Type::Watcher) = self.variable_types.get(name) {
                    match member_access.member.as_str() {
                        "pause" => {
                            self.output.push_str(&format!("hl_watcher_pause({})", name));
                            return Ok(());
                        }
                        "resume" => {
                            self.output.push_str(&format!("hl_watcher_resume({})", name));
                            return Ok(());
                        }
                        "end" => {
                            self.output.push_str(&format!("hl_watcher_end({})", name));
                            return Ok(());
                        }
                        "isActive" => {
                            self.output.push_str(&format!("hl_watcher_is_active({})", name));
                            return Ok(());
                        }
                        _ => unreachable!("type checker should have caught invalid watcher method"),
                    }
                }
            }
        }

        // Check if this function call returns a heap value and needs temp tracking
        let return_type = if let Expression::Ident { name: func_name, .. } = call.callee.as_ref() {
            self.functions.get(func_name).cloned()
        } else {
            None
        };

        let needs_temp_tracking = context == ExprContext::Temporary &&
            return_type.as_ref().map_or(false, |t| matches!(t, Type::String | Type::Object(_) |
                Type::Function(_, _) | Type::Optional(_) | Type::DynamicArray(_) | Type::UnknownType));

        if needs_temp_tracking {
            // Create temp variable for heap-returning function call
            let temp_name = self.next_temp_name();
            let heap_type = match return_type.as_ref().unwrap() {
                Type::String | Type::DynamicArray(_) => HeapType::Array,
                Type::Object(_) => HeapType::Object,
                Type::Function(_, _) => HeapType::Function,
                Type::Optional(_) => HeapType::Optional,
                Type::UnknownType => HeapType::Unknown,
                _ => unreachable!("checked above"),
            };

            // Register temp for cleanup
            self.temp_owners.insert(temp_name.clone(), heap_type);

            // Get C type for the temp declaration
            let temp_c_type = self.hilow_type_to_c(return_type.as_ref().unwrap());

            // Build declaration with function call
            let mut decl = format!("{} {} = ", temp_c_type, temp_name);

            // Temporarily capture function name and args
            let saved_output = std::mem::take(&mut self.output);

            // Generate function name/expression
            if let Expression::Ident { name: func_name, .. } = call.callee.as_ref() {
                let c_func_name = self.mangle_function_name(func_name);
                self.output.push_str(&c_func_name);
            } else {
                self.generate_expression(&call.callee, type_checker, ExprContext::Temporary)?;
            }
            self.output.push_str("(");

            for (i, arg) in call.args.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                self.generate_expression(arg, type_checker, ExprContext::Temporary)?;
            }
            self.output.push_str(")");

            let call_expr = std::mem::take(&mut self.output);
            decl.push_str(&call_expr);
            decl.push_str(";");

            // Hoist declaration to statement scope
            self.pending_statement_decls.push(decl);

            // Restore output and emit temp reference
            self.output = saved_output;
            self.output.push_str(&temp_name);
        } else {
            // Direct emission for non-heap types or owned context
            if let Expression::Ident { name: func_name, .. } = call.callee.as_ref() {
                // Use mangled name for nested functions
                let c_func_name = self.mangle_function_name(func_name);
                self.output.push_str(&c_func_name);
            } else {
                self.generate_expression(&call.callee, type_checker, ExprContext::Temporary)?;
            }
            self.output.push_str("(");

            for (i, arg) in call.args.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                self.generate_expression(arg, type_checker, ExprContext::Temporary)?;
            }

            self.output.push_str(")");
        }

        Ok(())
    }

    /// Special handling for print() built-in function
    /// Phase 4a-only: print() is treated as a magic function known to both type checker and codegen.
    /// This will be replaced with proper module imports in later phases.
    fn generate_print_call(&mut self, call: &Call, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        if call.args.len() != 1 {
            return Err(CodegenError::UnsupportedFeature {
                feature: "print() with != 1 argument".to_string(),
                phase: "Phase 6 (proper print implementation)".to_string(),
            });
        }

        let arg = &call.args[0];

        // Determine the type of the argument to call the right runtime function
        // Use enhanced type inference that handles object properties
        let arg_type = self.infer_expression_type_for_codegen(arg);

        let runtime_func = match arg_type {
            Type::I8 | Type::I16 | Type::I32 | Type::Isize => "print_i32",
            Type::I64 => "print_i64",
            Type::I128 => "print_i64", // Fall back to i64 for now
            Type::U8 | Type::U16 | Type::U32 | Type::Usize => "print_u32",
            Type::U64 | Type::U128 => "print_u64",
            Type::F32 => "print_f32",
            Type::F64 => "print_f64",
            Type::Bool => "print_bool",
            Type::String => "print_string",
            Type::Time => "print_time",
            Type::Duration => "print_duration",
            Type::Money => "print_money",
            Type::MoneyOf(_) => "print_money",
            Type::Nothing => {
                // Special case: print_nothing() takes no arguments
                self.output.push_str("print_nothing()");
                return Ok(());
            }
            Type::UnknownType => "print_unknown",
            Type::Optional(inner_type) => {
                // Generate runtime dispatch for optional type
                let inner_print_func = match inner_type.as_ref() {
                    Type::I8 | Type::I16 | Type::I32 | Type::Isize => "print_i32",
                    Type::I64 => "print_i64",
                    Type::U8 | Type::U16 | Type::U32 | Type::Usize => "print_u32",
                    Type::U64 => "print_u64",
                    Type::F32 => "print_f32",
                    Type::F64 => "print_f64",
                    Type::Bool => "print_bool",
                    Type::String => "print_string",
                    _ => {
                        return Err(CodegenError::UnsupportedFeature {
                            feature: format!("print() for optional type {}", inner_type),
                            phase: "later phases".to_string(),
                        });
                    }
                };

                self.output.push_str(&format!("print_optional_{}(",
                    match inner_type.as_ref() {
                        Type::I8 | Type::I16 | Type::I32 | Type::Isize => "i32",
                        Type::I64 => "i64",
                        Type::U8 | Type::U16 | Type::U32 | Type::Usize => "u32",
                        Type::U64 => "u64",
                        Type::F32 => "f32",
                        Type::F64 => "f64",
                        Type::Bool => "bool",
                        Type::String => "string",
                        _ => unreachable!()
                    }));
                self.generate_expression(arg, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            Type::Tuple(element_types) => {
                // Generate per-tuple-type print function call
                self.ensure_tuple_print_function(&element_types);
                let print_func_name = self.get_tuple_print_function_name(&element_types);
                self.output.push_str(&print_func_name);
                self.output.push_str("(");
                self.generate_expression(arg, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
                return Ok(());
            }
            Type::ObjectIterValue => {
                // Runtime dispatch based on type tag
                return self.generate_print_call_for_iter_value(arg, type_checker);
            }
            _ => {
                return Err(CodegenError::UnsupportedFeature {
                    feature: format!("print() for type {}", arg_type),
                    phase: "later phases".to_string(),
                });
            }
        };

        // Phase 4a: f-strings (like all fresh productions) mint a
        // statement-scoped temp in Temporary context, released at statement
        // end — no special inline cleanup here (that would double-free).
        self.output.push_str(runtime_func);
        self.output.push_str("(");

        // Special case: Usize needs to be cast to uint32_t for print_u32
        if matches!(arg_type, Type::Usize) {
            self.output.push_str("(uint32_t)");
        }

        self.generate_expression(arg, type_checker, ExprContext::Temporary)?;
        self.output.push_str(")");

        Ok(())
    }

    /// Generate print call for iteration value with runtime type dispatch
    fn generate_print_call_for_iter_value(&mut self, arg: &Expression, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // For iteration values, generate runtime dispatch based on __v_type
        if let Expression::Ident { name: var_name, .. } = arg {
            if Some(var_name.clone()) == self.current_iter_value_name {
                // This is the iteration value - generate runtime dispatch
                self.output.push_str("{\n");
                self.output.push_str("    switch (__v_type) {\n");
                self.output.push_str("      case TYPE_I32:\n");
                self.output.push_str(&format!("        print_i32(hl_object_property_value_i32_at(__iter_obj, __iter_i));\n"));
                self.output.push_str("        break;\n");
                self.output.push_str("      case TYPE_I64:\n");
                self.output.push_str(&format!("        print_i64(hl_object_property_value_i64_at(__iter_obj, __iter_i));\n"));
                self.output.push_str("        break;\n");
                self.output.push_str("      case TYPE_U32:\n");
                self.output.push_str(&format!("        print_u32(hl_object_property_value_u32_at(__iter_obj, __iter_i));\n"));
                self.output.push_str("        break;\n");
                self.output.push_str("      case TYPE_U64:\n");
                self.output.push_str(&format!("        print_u64(hl_object_property_value_u64_at(__iter_obj, __iter_i));\n"));
                self.output.push_str("        break;\n");
                self.output.push_str("      case TYPE_F32:\n");
                self.output.push_str(&format!("        print_f32(hl_object_property_value_f32_at(__iter_obj, __iter_i));\n"));
                self.output.push_str("        break;\n");
                self.output.push_str("      case TYPE_F64:\n");
                self.output.push_str(&format!("        print_f64(hl_object_property_value_f64_at(__iter_obj, __iter_i));\n"));
                self.output.push_str("        break;\n");
                self.output.push_str("      case TYPE_BOOL:\n");
                self.output.push_str(&format!("        print_bool(hl_object_property_value_bool_at(__iter_obj, __iter_i));\n"));
                self.output.push_str("        break;\n");
                self.output.push_str("      case TYPE_STR: {\n");
                // Accessor retains on return; release after printing
                self.output.push_str("        HiLowArray* __pv = hl_object_property_value_str_at(__iter_obj, __iter_i);\n");
                self.output.push_str("        print_string(__pv);\n");
                self.output.push_str("        hl_array_release(__pv);\n");
                self.output.push_str("        break;\n");
                self.output.push_str("      }\n");
                self.output.push_str("      default:\n");
                self.output.push_str("        printf(\"<unknown value>\\n\");\n");
                self.output.push_str("        break;\n");
                self.output.push_str("    }\n");
                self.output.push_str("  }");
                return Ok(());
            }
        }

        // Fall back to error for non-iteration ObjectIterValue expressions
        Err(CodegenError::UnsupportedFeature {
            feature: "print() for polymorphic value outside for-in loop".to_string(),
            phase: "Phase 7c-ζ".to_string(),
        })
    }

    /// Generate f-string interpolation for iteration value with runtime type dispatch
    fn generate_fstring_interpolation_for_iter_value(&mut self, arg: &Expression, _type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // For iteration values, generate runtime dispatch based on __v_type
        if let Expression::Ident { name: var_name, .. } = arg {
            if Some(var_name.clone()) == self.current_iter_value_name {
                // This is the iteration value - generate runtime dispatch
                self.output.push_str("{ switch (__v_type) {\n");

                self.output.push_str("      case TYPE_I32: {\n");
                self.output.push_str("        char __tmp_buf[32];\n");
                self.output.push_str("        sprintf(__tmp_buf, \"%d\", hl_object_property_value_i32_at(__iter_obj, __iter_i));\n");
                self.output.push_str("        hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf));\n");
                self.output.push_str("        break;\n");
                self.output.push_str("      }\n");

                self.output.push_str("      case TYPE_I64: {\n");
                self.output.push_str("        char __tmp_buf[32];\n");
                self.output.push_str("        sprintf(__tmp_buf, \"%ld\", hl_object_property_value_i64_at(__iter_obj, __iter_i));\n");
                self.output.push_str("        hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf));\n");
                self.output.push_str("        break;\n");
                self.output.push_str("      }\n");

                self.output.push_str("      case TYPE_U32: {\n");
                self.output.push_str("        char __tmp_buf[32];\n");
                self.output.push_str("        sprintf(__tmp_buf, \"%u\", hl_object_property_value_u32_at(__iter_obj, __iter_i));\n");
                self.output.push_str("        hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf));\n");
                self.output.push_str("        break;\n");
                self.output.push_str("      }\n");

                self.output.push_str("      case TYPE_U64: {\n");
                self.output.push_str("        char __tmp_buf[32];\n");
                self.output.push_str("        sprintf(__tmp_buf, \"%lu\", hl_object_property_value_u64_at(__iter_obj, __iter_i));\n");
                self.output.push_str("        hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf));\n");
                self.output.push_str("        break;\n");
                self.output.push_str("      }\n");

                self.output.push_str("      case TYPE_F32: {\n");
                self.output.push_str("        char __tmp_buf[32];\n");
                self.output.push_str("        sprintf(__tmp_buf, \"%g\", hl_object_property_value_f32_at(__iter_obj, __iter_i));\n");
                self.output.push_str("        hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf));\n");
                self.output.push_str("        break;\n");
                self.output.push_str("      }\n");

                self.output.push_str("      case TYPE_F64: {\n");
                self.output.push_str("        char __tmp_buf[32];\n");
                self.output.push_str("        sprintf(__tmp_buf, \"%g\", hl_object_property_value_f64_at(__iter_obj, __iter_i));\n");
                self.output.push_str("        hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf));\n");
                self.output.push_str("        break;\n");
                self.output.push_str("      }\n");

                self.output.push_str("      case TYPE_BOOL: {\n");
                self.output.push_str("        const char* __bool_str = hl_object_property_value_bool_at(__iter_obj, __iter_i) ? \"true\" : \"false\";\n");
                self.output.push_str("        hl_array_append_bytes(__fstring_arr, (const uint8_t*)__bool_str, strlen(__bool_str));\n");
                self.output.push_str("        break;\n");
                self.output.push_str("      }\n");

                self.output.push_str("      case TYPE_STR: {\n");
                // Accessor retains on return; release after appending
                self.output.push_str("        HiLowArray* __str_val = hl_object_property_value_str_at(__iter_obj, __iter_i);\n");
                self.output.push_str("        hl_array_append_bytes(__fstring_arr, (const uint8_t*)__str_val->data, __str_val->length);\n");
                self.output.push_str("        hl_array_release(__str_val);\n");
                self.output.push_str("        break;\n");
                self.output.push_str("      }\n");

                self.output.push_str("      default:\n");
                self.output.push_str("        hl_array_append_bytes(__fstring_arr, (const uint8_t*)\"<unknown value>\", 15);\n");
                self.output.push_str("        break;\n");
                self.output.push_str("    } } ");
                return Ok(());
            }
        }

        // Fall back to error for non-iteration ObjectIterValue expressions
        Err(CodegenError::UnsupportedFeature {
            feature: "f-string interpolation for polymorphic value outside for-in loop".to_string(),
            phase: "Phase 7c-ζ".to_string(),
        })
    }

    /// Convert a HiLow type to a C type string
    fn hilow_type_to_c(&self, hilow_type: &Type) -> String {
        match hilow_type {
            Type::I8 => "int8_t".to_string(),
            Type::I16 => "int16_t".to_string(),
            Type::I32 => "int32_t".to_string(),
            Type::I64 => "int64_t".to_string(),
            Type::I128 => "int64_t".to_string(), // Fall back to 64-bit for now
            Type::U8 => "uint8_t".to_string(),
            Type::U16 => "uint16_t".to_string(),
            Type::U32 => "uint32_t".to_string(),
            Type::U64 => "uint64_t".to_string(),
            Type::U128 => "uint64_t".to_string(), // Fall back to 64-bit for now
            Type::F32 => "float".to_string(),
            Type::F64 => "double".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "HiLowArray*".to_string(), // String as HiLowArray<u8>
            Type::Usize => "size_t".to_string(),
            Type::Isize => "ssize_t".to_string(),
            Type::Nothing => "void*".to_string(),
            Type::Time => "HiLowTime".to_string(),
            Type::Duration => "HiLowDuration".to_string(),
            Type::Money => "HiLowMoney".to_string(),
            Type::MoneyOf(_) => "HiLowMoney".to_string(),
            Type::FixedArray(_, _) => "void*".to_string(), // Placeholder for Phase 6
            Type::DynamicArray(_) => "HiLowArray*".to_string(), // Array Phase A
            Type::Object(_) => "HiLowObject*".to_string(),
            Type::Function(_, _) => "HiLowFunction*".to_string(), // Function value type (Phase 7c-β)
            Type::Watcher => "HiLowWatcher*".to_string(), // Heap-allocated watcher value (Phase 10-δ-α)
            Type::Tuple(element_types) => self.get_tuple_type_name(element_types),
            Type::ObjectIterValue => "void*".to_string(), // Runtime-dispatched iteration value
            Type::Unknown => "void".to_string(),
            Type::UnknownType => "HiLowUnknown*".to_string(), // Unknown type with reason and options
            Type::Optional(_) => "HiLowOptional*".to_string(), // T? types with wrapper struct (Phase 9b fix 3a)
        }
    }

    fn ast_type_to_c(&self, hilow_type: &crate::ast::Type) -> String {
        use crate::ast::{Type as AstType, PrimitiveType};
        match hilow_type {
            AstType::Primitive(PrimitiveType::I8) => "int8_t".to_string(),
            AstType::Primitive(PrimitiveType::I16) => "int16_t".to_string(),
            AstType::Primitive(PrimitiveType::I32) => "int32_t".to_string(),
            AstType::Primitive(PrimitiveType::I64) => "int64_t".to_string(),
            AstType::Primitive(PrimitiveType::I128) => "int64_t".to_string(), // Fall back to 64-bit
            AstType::Primitive(PrimitiveType::U8) => "uint8_t".to_string(),
            AstType::Primitive(PrimitiveType::U16) => "uint16_t".to_string(),
            AstType::Primitive(PrimitiveType::U32) => "uint32_t".to_string(),
            AstType::Primitive(PrimitiveType::U64) => "uint64_t".to_string(),
            AstType::Primitive(PrimitiveType::U128) => "uint64_t".to_string(), // Fall back to 64-bit
            AstType::Primitive(PrimitiveType::F32) => "float".to_string(),
            AstType::Primitive(PrimitiveType::F64) => "double".to_string(),
            AstType::Primitive(PrimitiveType::Bool) => "bool".to_string(),
            AstType::Primitive(PrimitiveType::String) => "const char*".to_string(),
            AstType::Primitive(PrimitiveType::Usize) => "size_t".to_string(),
            AstType::Primitive(PrimitiveType::Isize) => "ssize_t".to_string(),
            AstType::Primitive(PrimitiveType::Nothing) => "void*".to_string(),
            AstType::Primitive(PrimitiveType::Time) => "HiLowTime".to_string(),
            AstType::Primitive(PrimitiveType::Duration) => "HiLowDuration".to_string(),
            AstType::Primitive(PrimitiveType::Money) => "HiLowMoney".to_string(),
            AstType::Primitive(PrimitiveType::Unknown) => "HiLowUnknown*".to_string(),
            AstType::MoneyOf(_) => "HiLowMoney".to_string(),
            AstType::FixedArray(_, _) => "void*".to_string(), // Placeholder
            AstType::DynamicArray(_) => "void*".to_string(), // Placeholder
            AstType::Object(_) => "void*".to_string(), // Placeholder
            AstType::Function(_, _) => "void*".to_string(), // Placeholder
            AstType::Unknown => "void".to_string(),
            AstType::Optional(_) => "HiLowOptional*".to_string(),
            AstType::Tuple(_) => "void*".to_string(), // Placeholder
            AstType::Watcher => panic!("Phase 10-δ-γ-fixup: ast::Type::Watcher should not reach codegen yet"),
        }
    }

    /// Generate the C type name for a tuple type (e.g., "HiLowTuple_i32_string")
    fn get_tuple_type_name(&self, element_types: &[Type]) -> String {
        let mut name = "HiLowTuple".to_string();
        for element_type in element_types {
            name.push_str("_");
            name.push_str(&self.mangle_type_name(element_type));
        }
        name
    }

    /// Mangle a type name for use in C identifiers
    fn mangle_type_name(&self, ty: &Type) -> String {
        match ty {
            Type::I8 => "i8".to_string(),
            Type::I16 => "i16".to_string(),
            Type::I32 => "i32".to_string(),
            Type::I64 => "i64".to_string(),
            Type::I128 => "i128".to_string(),
            Type::U8 => "u8".to_string(),
            Type::U16 => "u16".to_string(),
            Type::U32 => "u32".to_string(),
            Type::U64 => "u64".to_string(),
            Type::U128 => "u128".to_string(),
            Type::F32 => "f32".to_string(),
            Type::F64 => "f64".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "string".to_string(),
            Type::Usize => "usize".to_string(),
            Type::Isize => "isize".to_string(),
            Type::Nothing => "nothing".to_string(),
            Type::Time => "time".to_string(),
            Type::Duration => "duration".to_string(),
            Type::Money => "money".to_string(),
            Type::MoneyOf(currency) => format!("money_{}", currency),
            Type::Object(_) => "object".to_string(),
            Type::Function(_, _) => "function".to_string(),
            Type::Tuple(element_types) => self.get_tuple_type_name(element_types),
            Type::Optional(inner) => format!("opt_{}", self.mangle_type_name(inner)),
            _ => "unknown".to_string(),
        }
    }

    /// Ensure a tuple struct definition exists for the given element types
    fn ensure_tuple_struct(&mut self, element_types: &[Type]) {
        // Check if we've already generated this tuple type
        if self.generated_tuple_types.contains(element_types) {
            return;
        }

        // Generate the struct definition
        let struct_name = self.get_tuple_type_name(element_types);
        self.tuple_struct_definitions.push_str(&format!("typedef struct {} {{\n", struct_name));

        for (i, element_type) in element_types.iter().enumerate() {
            let c_type = self.hilow_type_to_c(element_type);
            self.tuple_struct_definitions.push_str(&format!("    {} _{};\n", c_type, i));
        }

        self.tuple_struct_definitions.push_str(&format!("}} {};\n\n", struct_name));

        // Mark this tuple type as generated
        self.generated_tuple_types.insert(element_types.to_vec());
    }

    /// Get the print function name for a tuple type
    fn get_tuple_print_function_name(&self, element_types: &[Type]) -> String {
        format!("print_tuple_{}", self.mangle_tuple_type_for_func_name(element_types))
    }

    /// Mangle tuple type for function name (simpler than struct name)
    fn mangle_tuple_type_for_func_name(&self, element_types: &[Type]) -> String {
        element_types.iter()
            .map(|t| self.mangle_type_name(t))
            .collect::<Vec<_>>()
            .join("_")
    }

    /// Ensure a print function exists for the given tuple type
    fn ensure_tuple_print_function(&mut self, element_types: &[Type]) {
        let func_name = self.get_tuple_print_function_name(element_types);

        // Check if we've already generated this print function
        if self.generated_functions.contains(&func_name) {
            return;
        }

        // Ensure the struct exists first
        self.ensure_tuple_struct(element_types);
        let struct_name = self.get_tuple_type_name(element_types);

        // Generate the print function
        self.generated_functions.push_str(&format!("void {}({} t) {{\n", func_name, struct_name));
        self.generated_functions.push_str("    printf(\"(\");\n");

        for (i, element_type) in element_types.iter().enumerate() {
            if i > 0 {
                self.generated_functions.push_str("    printf(\", \");\n");
            }

            // Generate element-specific print call
            match element_type {
                Type::I8 | Type::I16 | Type::I32 | Type::Isize => {
                    self.generated_functions.push_str(&format!("    printf(\"%d\", t._{});\n", i));
                }
                Type::I64 => {
                    self.generated_functions.push_str(&format!("    printf(\"%ld\", t._{});\n", i));
                }
                Type::U8 | Type::U16 | Type::U32 | Type::Usize => {
                    self.generated_functions.push_str(&format!("    printf(\"%u\", t._{});\n", i));
                }
                Type::U64 => {
                    self.generated_functions.push_str(&format!("    printf(\"%lu\", t._{});\n", i));
                }
                Type::F32 => {
                    self.generated_functions.push_str(&format!("    printf(\"%g\", t._{});\n", i));
                }
                Type::F64 => {
                    self.generated_functions.push_str(&format!("    printf(\"%g\", t._{});\n", i));
                }
                Type::Bool => {
                    self.generated_functions.push_str(&format!("    printf(t._{}? \"true\" : \"false\");\n", i));
                }
                Type::String => {
                    self.generated_functions.push_str(&format!("    printf(\"%s\", t._{});\n", i));
                }
                _ => {
                    // For other types, use a placeholder
                    self.generated_functions.push_str(&format!("    printf(\"<{}>\");\n", element_type));
                }
            }
        }

        self.generated_functions.push_str("    printf(\")\\n\");\n");
        self.generated_functions.push_str("}\n\n");
    }

    fn generate_is_check(&mut self, is_check: &IsCheck, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        let target_type = Type::from_ast_type(&is_check.ty);

        // Special case: is nothing should be a runtime pointer comparison
        if matches!(target_type, Type::Nothing) {
            if is_check.negated {
                self.output.push_str("(");
                self.generate_expression(&is_check.expression, type_checker, ExprContext::Temporary)?;
                self.output.push_str(" != &the_nothing)");
            } else {
                self.output.push_str("(");
                self.generate_expression(&is_check.expression, type_checker, ExprContext::Temporary)?;
                self.output.push_str(" == &the_nothing)");
            }
            return Ok(());
        }

        // Special case: is unknown on optional types should be a runtime check
        if matches!(target_type, Type::UnknownType) {
            let expr_type = self.infer_expression_type_without_refinements(&is_check.expression);
            if matches!(expr_type, Type::Optional(_)) {
                // Runtime check: is the value an unknown value?
                if is_check.negated {
                    self.output.push_str("(!hl_is_unknown(");
                    self.generate_expression_without_refinements(&is_check.expression, type_checker)?;
                    self.output.push_str("))");
                } else {
                    self.output.push_str("(hl_is_unknown(");
                    self.generate_expression_without_refinements(&is_check.expression, type_checker)?;
                    self.output.push_str("))");
                }
                return Ok(());
            }
        }

        // For other primitive types, is checks are done at compile time
        let expr_type = self.infer_expression_type_for_codegen(&is_check.expression);

        // Compare types at compile time
        let types_match = expr_type == target_type;

        // Apply negation if needed
        let result = if is_check.negated { !types_match } else { types_match };

        // Emit 1 for true, 0 for false
        self.output.push_str(if result { "1" } else { "0" });

        Ok(())
    }

    fn generate_object_is_check(&mut self, obj_is_check: &ObjectIsCheck, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Check if either side is a nothing literal
        let lhs_is_nothing = matches!(obj_is_check.lhs.as_ref(), Expression::Nothing(_));
        let rhs_is_nothing = matches!(obj_is_check.rhs.as_ref(), Expression::Nothing(_));

        if lhs_is_nothing || rhs_is_nothing {
            // Handle is nothing as pointer comparison
            if obj_is_check.negated {
                self.output.push_str("(");
                self.generate_expression(&obj_is_check.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(" != ");
                self.generate_expression(&obj_is_check.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
            } else {
                self.output.push_str("(");
                self.generate_expression(&obj_is_check.lhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(" == ");
                self.generate_expression(&obj_is_check.rhs, type_checker, ExprContext::Temporary)?;
                self.output.push_str(")");
            }
            return Ok(());
        }

        // Generate: hl_object_is(lhs, rhs) for obj is obj
        if obj_is_check.negated {
            self.output.push_str("!");
        }
        self.output.push_str("hl_object_is(");
        self.generate_expression(&obj_is_check.lhs, type_checker, ExprContext::Temporary)?;
        self.output.push_str(", ");
        self.generate_expression(&obj_is_check.rhs, type_checker, ExprContext::Temporary)?;
        self.output.push_str(")");
        Ok(())
    }

    fn next_var_name(&mut self) -> String {
        let name = format!("_v{}", self.var_counter);
        self.var_counter += 1;
        name
    }

    /// Generate unique temporary variable name for expression-temporary cleanup
    fn next_temp_name(&mut self) -> String {
        let name = format!("__tmp_{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }


    fn mangle_function_name(&self, name: &str) -> String {
        // Phase 6a-fixup: Simple mangling for nested functions to avoid C keyword conflicts
        format!("hilow_{}", name)
    }

    fn mangle_variable_name(&self, name: &str) -> String {
        // Phase 7c-β: Simple mangling for variable names to avoid C keyword conflicts
        match name {
            // Common C keywords and types that might conflict
            "auto" | "break" | "case" | "char" | "const" | "continue" | "default" | "do" |
            "double" | "else" | "enum" | "extern" | "float" | "for" | "goto" | "if" |
            "int" | "long" | "register" | "return" | "short" | "signed" | "sizeof" | "static" |
            "struct" | "switch" | "typedef" | "union" | "unsigned" | "void" | "volatile" | "while" |
            // Additional C99/C11 keywords
            "inline" | "restrict" | "_Bool" | "_Complex" | "_Imaginary" |
            // Common types we use
            "int32_t" | "int64_t" | "uint32_t" | "uint64_t" | "bool" | "size_t" => {
                format!("hl_{}", name)
            }
            _ => name.to_string()
        }
    }

    /// Simple type inference for expressions in Phase 4a
    /// This is a simplified version that doesn't use the full type checker context
    fn infer_expression_type(&self, expr: &Expression) -> Type {
        match expr {
            Expression::IntLit(_, _) => Type::I32, // Default integer type
            Expression::FloatLit(_, _) => Type::F64, // Default float type
            Expression::StringLit(_, _) => Type::String,
            Expression::DurationLit(_, _, _) => Type::Duration,
            Expression::MoneyLit(_, currency, _) => Type::MoneyOf(currency.clone()),
            Expression::FString(_) => Type::String,
            Expression::BoolLit(_, _) => Type::Bool,
            Expression::Ident { name, .. } => {
                // Look up the variable type from our tracking
                self.variable_types.get(name).cloned().unwrap_or(Type::I32)
            }
            Expression::This(_) => {
                // Return the receiver object type in method context
                self.method_receiver_type.clone().unwrap_or(Type::Unknown)
            }
            Expression::BinaryOp(op) => {
                // Infer based on the operator
                match op.op {
                    BinaryOpKind::Add | BinaryOpKind::Sub | BinaryOpKind::Mul |
                    BinaryOpKind::Div | BinaryOpKind::Mod => {
                        // Arithmetic operations - check operands
                        let lhs_type = self.infer_expression_type(&op.lhs);
                        let rhs_type = self.infer_expression_type(&op.rhs);

                        // If either operand is float, result is float
                        if matches!(lhs_type, Type::F32 | Type::F64) ||
                           matches!(rhs_type, Type::F32 | Type::F64) {
                            Type::F64
                        } else {
                            Type::I32
                        }
                    }
                    BinaryOpKind::Less | BinaryOpKind::Greater | BinaryOpKind::LessEq |
                    BinaryOpKind::GreaterEq | BinaryOpKind::Eq | BinaryOpKind::NotEq |
                    BinaryOpKind::And | BinaryOpKind::Or => Type::Bool,
                    _ => Type::I32, // Default for other ops
                }
            }
            Expression::UnaryOp(op) => {
                match op.op {
                    UnaryOpKind::Not => Type::Bool,
                    _ => self.infer_expression_type(&op.operand),
                }
            }
            Expression::IsCheck(_) => Type::Bool,
            Expression::ObjectIsCheck(_) => Type::Bool,
            Expression::QualifiedOp(qualified_op) => {
                match qualified_op.op {
                    QualifiedOpKind::Assign => self.infer_expression_type(&qualified_op.lhs),
                    QualifiedOpKind::Eq | QualifiedOpKind::NotEq => Type::Bool,
                }
            }
            _ => Type::I32, // Default fallback
        }
    }

    /// Walk the prototype chain to find a property type (Phase 7b) - codegen version
    fn find_property_type_in_chain(&self, object_type: &Type, property_name: &str, depth: usize) -> Option<Type> {
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
                        return self.find_property_type_in_chain(prop_type, property_name, depth + 1);
                    }
                }

                // No prototype property found
                None
            },
            _ => None // Not an object type
        }
    }

    /// Enhanced type inference for codegen that handles object types
    fn infer_expression_type_for_codegen(&self, expr: &Expression) -> Type {
        match expr {
            Expression::IntLit(_, _) => Type::I32,
            Expression::FloatLit(_, _) => Type::F64,
            Expression::StringLit(_, _) => Type::String,
            Expression::DurationLit(_, _, _) => Type::Duration,
            Expression::MoneyLit(_, currency, _) => Type::MoneyOf(currency.clone()),
            Expression::FString(_) => Type::String,
            Expression::BoolLit(_, _) => Type::Bool,
            Expression::Ident { name, refined_type, .. } => {
                // Check for refined type first (from type narrowing)
                if let Some(ref refined) = refined_type {
                    Type::from_ast_type(refined)
                } else {
                    // Look up the variable type from our tracking
                    self.variable_types.get(name).cloned().unwrap_or(Type::Unknown)
                }
            }
            Expression::This(_) => {
                // Return the receiver object type in method context
                self.method_receiver_type.clone().unwrap_or(Type::Unknown)
            }
            Expression::ObjectLiteral(obj_lit) => {
                let mut properties = Vec::new();
                for (prop_name, prop_expr) in &obj_lit.properties {
                    let prop_type = self.infer_expression_type_for_codegen(prop_expr);
                    // Phase 1.5e: a weak slot reads back as referent-or-unknown
                    let prop_type = if matches!(prop_expr, Expression::WeakRef(_, _))
                        && matches!(prop_type, Type::Object(_)) {
                        Type::Optional(Box::new(prop_type))
                    } else {
                        prop_type
                    };
                    properties.push((prop_name.clone(), prop_type));
                }
                Type::Object(properties)
            }
            Expression::MemberAccess(member_access) => {
                let object_type = self.infer_expression_type_for_codegen(&member_access.object);
                match object_type {
                    Type::Object(_) => {
                        // Use prototype chain lookup (Phase 7b)
                        self.find_property_type_in_chain(&object_type, &member_access.member, 0)
                            .unwrap_or(Type::Unknown)
                    }
                    Type::UnknownType => {
                        // Unknown types have known properties: reason and options
                        match member_access.member.as_str() {
                            "reason" => Type::String,
                            "options" => Type::DynamicArray(Box::new(Type::String)),
                            _ => Type::Nothing, // Unknown properties return nothing
                        }
                    }
                    Type::Optional(ref inner) => {
                        // Phase 1.5e: member access through a weak read
                        // (object-or-unknown) propagates — property T reads
                        // as T?; an already-optional (nested weak) property
                        // stays as-is. Chain lookup wins over the
                        // .reason/.options fallback, which addresses the
                        // unknown state itself (mirrors treat_as_unknown in
                        // generate_member_access).
                        if let Type::Object(_) = inner.as_ref() {
                            if let Some(prop_type) = self.find_property_type_in_chain(inner, &member_access.member, 0) {
                                return if matches!(prop_type, Type::Optional(_)) {
                                    prop_type
                                } else {
                                    Type::Optional(Box::new(prop_type))
                                };
                            }
                        }
                        match member_access.member.as_str() {
                            "reason" => Type::String,
                            "options" => Type::DynamicArray(Box::new(Type::String)),
                            _ => Type::Unknown,
                        }
                    }
                    Type::DynamicArray(_) => {
                        // Arrays have .length property
                        match member_access.member.as_str() {
                            "length" => Type::Usize,
                            _ => Type::Unknown,
                        }
                    }
                    Type::String => {
                        // Strings have .bytelength property
                        match member_access.member.as_str() {
                            "bytelength" => Type::Usize,
                            _ => Type::Unknown,
                        }
                    }
                    _ => Type::Unknown
                }
            }
            Expression::BinaryOp(op) => {
                match op.op {
                    BinaryOpKind::Add => {
                        let lhs_type = self.infer_expression_type_for_codegen(&op.lhs);
                        let rhs_type = self.infer_expression_type_for_codegen(&op.rhs);

                        // Handle time/duration arithmetic first
                        match (&lhs_type, &rhs_type) {
                            (Type::Time, Type::Duration) => Type::Time,        // time + duration → time
                            (Type::Duration, Type::Time) => Type::Time,        // duration + time → time
                            (Type::Duration, Type::Duration) => Type::Duration, // duration + duration → duration
                            // String concatenation
                            (Type::String, Type::String) => Type::String,
                            // Money + Money → Money (currency from left operand, type checker ensures same currency)
                            (Type::MoneyOf(currency), Type::MoneyOf(_)) => Type::MoneyOf(currency.clone()),
                            // Generic money + specific currency → specific currency
                            (Type::Money, Type::MoneyOf(currency)) => Type::MoneyOf(currency.clone()),
                            (Type::MoneyOf(currency), Type::Money) => Type::MoneyOf(currency.clone()),
                            // Generic money + generic money → generic money
                            (Type::Money, Type::Money) => Type::Money,
                            _ => {
                                // Regular numeric addition
                                if matches!(lhs_type, Type::F32 | Type::F64) ||
                                   matches!(rhs_type, Type::F32 | Type::F64) {
                                    Type::F64
                                } else {
                                    Type::I32
                                }
                            }
                        }
                    }
                    BinaryOpKind::Sub => {
                        let lhs_type = self.infer_expression_type_for_codegen(&op.lhs);
                        let rhs_type = self.infer_expression_type_for_codegen(&op.rhs);

                        // Handle time/duration arithmetic first
                        match (&lhs_type, &rhs_type) {
                            (Type::Time, Type::Duration) => Type::Time,         // time - duration → time
                            (Type::Time, Type::Time) => Type::Duration,        // time - time → duration
                            (Type::Duration, Type::Duration) => Type::Duration, // duration - duration → duration
                            // Money - Money → Money (currency from left operand, type checker ensures same currency)
                            (Type::MoneyOf(currency), Type::MoneyOf(_)) => Type::MoneyOf(currency.clone()),
                            // Generic money + specific currency → specific currency
                            (Type::Money, Type::MoneyOf(currency)) => Type::MoneyOf(currency.clone()),
                            (Type::MoneyOf(currency), Type::Money) => Type::MoneyOf(currency.clone()),
                            // Generic money + generic money → generic money
                            (Type::Money, Type::Money) => Type::Money,
                            _ => {
                                // Regular numeric subtraction
                                if matches!(lhs_type, Type::F32 | Type::F64) ||
                                   matches!(rhs_type, Type::F32 | Type::F64) {
                                    Type::F64
                                } else {
                                    Type::I32
                                }
                            }
                        }
                    }
                    BinaryOpKind::Mul => {
                        let lhs_type = self.infer_expression_type_for_codegen(&op.lhs);
                        let rhs_type = self.infer_expression_type_for_codegen(&op.rhs);

                        // Handle money multiplication
                        match (&lhs_type, &rhs_type) {
                            // Money * Numeric → Money (currency from left)
                            (Type::MoneyOf(currency), _) if rhs_type.is_numeric() => Type::MoneyOf(currency.clone()),
                            (Type::Money, _) if rhs_type.is_numeric() => Type::Money,
                            // Numeric * Money → Money (currency from right)
                            (_, Type::MoneyOf(currency)) if lhs_type.is_numeric() => Type::MoneyOf(currency.clone()),
                            (_, Type::Money) if lhs_type.is_numeric() => Type::Money,
                            _ => {
                                // Regular numeric multiplication
                                if matches!(lhs_type, Type::F32 | Type::F64) ||
                                   matches!(rhs_type, Type::F32 | Type::F64) {
                                    Type::F64
                                } else {
                                    Type::I32
                                }
                            }
                        }
                    }
                    BinaryOpKind::Div => {
                        let lhs_type = self.infer_expression_type_for_codegen(&op.lhs);
                        let rhs_type = self.infer_expression_type_for_codegen(&op.rhs);

                        // Handle money division
                        match (&lhs_type, &rhs_type) {
                            // Money / Money → F64 (ratio)
                            (Type::MoneyOf(_), Type::MoneyOf(_)) => Type::F64,
                            (Type::Money, Type::Money) => Type::F64,
                            (Type::Money, Type::MoneyOf(_)) => Type::F64,
                            (Type::MoneyOf(_), Type::Money) => Type::F64,
                            // Money / Numeric → Money (currency from left)
                            (Type::MoneyOf(currency), _) if rhs_type.is_numeric() => Type::MoneyOf(currency.clone()),
                            (Type::Money, _) if rhs_type.is_numeric() => Type::Money,
                            _ => {
                                // Regular numeric division
                                if matches!(lhs_type, Type::F32 | Type::F64) ||
                                   matches!(rhs_type, Type::F32 | Type::F64) {
                                    Type::F64
                                } else {
                                    Type::I32
                                }
                            }
                        }
                    }
                    BinaryOpKind::Mod => {
                        let lhs_type = self.infer_expression_type_for_codegen(&op.lhs);
                        let rhs_type = self.infer_expression_type_for_codegen(&op.rhs);
                        if matches!(lhs_type, Type::F32 | Type::F64) ||
                           matches!(rhs_type, Type::F32 | Type::F64) {
                            Type::F64
                        } else {
                            Type::I32
                        }
                    }
                    BinaryOpKind::Less | BinaryOpKind::Greater | BinaryOpKind::LessEq |
                    BinaryOpKind::GreaterEq | BinaryOpKind::Eq | BinaryOpKind::NotEq |
                    BinaryOpKind::And | BinaryOpKind::Or => Type::Bool,
                    _ => Type::I32
                }
            }
            Expression::UnaryOp(op) => {
                match op.op {
                    UnaryOpKind::Not => Type::Bool,
                    _ => self.infer_expression_type_for_codegen(&op.operand),
                }
            }
            Expression::IsCheck(_) => Type::Bool,
            Expression::ObjectIsCheck(_) => Type::Bool,
            Expression::QualifiedOp(qualified_op) => {
                match qualified_op.op {
                    QualifiedOpKind::Assign => self.infer_expression_type_for_codegen(&qualified_op.lhs),
                    QualifiedOpKind::Eq | QualifiedOpKind::NotEq => Type::Bool,
                }
            }
            Expression::Call(call) => {
                // For function calls, try to look up the return type
                if let Expression::Ident { name: func_name, .. } = call.callee.as_ref() {
                    if func_name == "print" {
                        return Type::I32; // print() returns i32
                    }
                    // Check if it's a named function first
                    if let Some(return_type) = self.functions.get(func_name) {
                        // For named functions, return their declared return type
                        return_type.clone()
                    } else if let Some(var_type) = self.variable_types.get(func_name) {
                        if let Type::Function(_, return_type) = var_type {
                            // For function values, return the return type
                            *return_type.clone()
                        } else {
                            // For other variable types, return the variable type
                            var_type.clone()
                        }
                    } else {
                        Type::I32
                    }
                } else if let Expression::MemberAccess(member_access) = call.callee.as_ref() {
                    // Handle member function calls like time.parse()
                    if let Expression::Ident { name, .. } = member_access.object.as_ref() {
                        if name == "time" {
                            match member_access.member.as_str() {
                                "parse" => Type::Optional(Box::new(Type::Time)),
                                "now" => Type::Time,
                                _ => Type::I32
                            }
                        } else if matches!(self.variable_types.get(name), Some(Type::Watcher)) {
                            // Phase 3d: watcher method calls infer by the
                            // variable's TYPE (both forms), matching the
                            // dispatch arm in generate_function_call.
                            match member_access.member.as_str() {
                                "pause" | "resume" | "end" => Type::Nothing,
                                "isActive" => Type::Bool,
                                _ => Type::I32
                            }
                        } else {
                            // Check if this is a method call on an array (pop/remove)
                            let object_type = self.infer_expression_type_for_codegen(&member_access.object);
                            if let Type::DynamicArray(elem_type) = object_type {
                                match member_access.member.as_str() {
                                    "pop" | "remove" => *elem_type, // Return element type
                                    _ => Type::I32,
                                }
                            } else {
                                Type::I32
                            }
                        }
                    } else {
                        Type::I32
                    }
                } else {
                    Type::I32 // Default for complex call expressions
                }
            }
            Expression::FunctionExpr(func_expr) => {
                // For function expressions, return the function type
                let param_types: Vec<Type> = func_expr.params.iter()
                    .map(|p| Type::from_ast_type(&p.ty))
                    .collect();
                let return_type = Type::from_ast_type(&func_expr.return_type);
                Type::Function(param_types, Box::new(return_type))
            }
            Expression::Match(match_expr) => {
                // For match expressions used in statement context, return Nothing
                // For match expressions used as values, return the arm type
                // Since we can't distinguish context here, check if all arms are statements
                let all_void_expressions = match_expr.arms.iter().all(|arm| {
                    match &arm.body {
                        MatchBody::Expression(expr) => {
                            // Check if this is a statement-like expression (e.g., print calls)
                            matches!(expr, Expression::Call(_))
                        }
                        MatchBody::Block(_) => true, // Blocks don't return values
                    }
                });

                if all_void_expressions {
                    Type::Nothing
                } else if let Some(first_arm) = match_expr.arms.first() {
                    match &first_arm.body {
                        MatchBody::Expression(expr) => self.infer_expression_type_for_codegen(expr),
                        MatchBody::Block(_) => Type::Nothing,
                    }
                } else {
                    Type::Nothing
                }
            }
            Expression::WeakRef(expr, _) => {
                // Weak references have the same type as the inner expression
                self.infer_expression_type_for_codegen(expr)
            }
            Expression::Nothing(_) => Type::Nothing,
            Expression::Unknown(_) => Type::UnknownType,
            Expression::TupleLit(elements, _) => {
                // Infer element types for tuple literal
                let mut element_types = Vec::new();
                for element in elements {
                    let element_type = self.infer_expression_type_for_codegen(element);
                    element_types.push(element_type);
                }
                Type::Tuple(element_types)
            }
            Expression::TupleAccess(tuple_expr, index, _) => {
                // Infer type from tuple access
                let tuple_type = self.infer_expression_type_for_codegen(tuple_expr);
                if let Type::Tuple(element_types) = tuple_type {
                    element_types.get(*index).cloned().unwrap_or(Type::Unknown)
                } else {
                    Type::Unknown
                }
            }
            Expression::ArrayLit(elements, _) => {
                // Infer array type from first element
                if !elements.is_empty() {
                    let elem_type = self.infer_expression_type_for_codegen(&elements[0]);
                    Type::DynamicArray(Box::new(elem_type))
                } else {
                    Type::Unknown
                }
            }
            Expression::IndexAccess(index_access) => {
                // Infer element type from array or string type
                let array_type = self.infer_expression_type_for_codegen(&index_access.object);
                match array_type {
                    Type::DynamicArray(elem_type) => *elem_type,
                    Type::String => Type::U8, // String indexing returns u8 bytes
                    _ => Type::Unknown,
                }
            }
            Expression::WatcherExpr(_) => Type::Watcher,
            Expression::TypeAscription(_, ascribed_ty, _) => {
                // For type ascription, return the ascribed type
                Type::from_ast_type(ascribed_ty)
            }
            _ => Type::Unknown
        }
    }

    fn generate_qualified_op_statement(&mut self, qualified_op: &QualifiedOp, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        self.output.push_str("  ");
        self.generate_qualified_op_expression(qualified_op, type_checker)?;
        self.output.push_str(";\n");
        Ok(())
    }

    fn generate_qualified_op_expression(&mut self, qualified_op: &QualifiedOp, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        match qualified_op.op {
            QualifiedOpKind::Assign => {
                // Generate qualified assignment: x (qualifier)= y  ->  x = x op y
                self.generate_qualified_assignment(qualified_op, type_checker)?;
            }
            QualifiedOpKind::Eq | QualifiedOpKind::NotEq => {
                return Err(CodegenError::UnsupportedFeature {
                    feature: "qualified equality operators".to_string(),
                    phase: "Phase 6c (strings) and Phase 9 (time/money)".to_string(),
                });
            }
        }
        Ok(())
    }

    fn generate_qualified_assignment(&mut self, qualified_op: &QualifiedOp, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // For now, only handle single qualifiers (no comma-separated lists)
        if qualified_op.qualifiers.len() != 1 {
            return Err(CodegenError::UnsupportedFeature {
                feature: "multiple qualifiers in assignment".to_string(),
                phase: "future phases when multiple qualifiers are implemented".to_string(),
            });
        }

        let qualifier = &qualified_op.qualifiers[0];

        // Generate: lhs = lhs op rhs
        self.generate_expression(&qualified_op.lhs, type_checker, ExprContext::Temporary)?;
        self.output.push_str(" = ");
        self.generate_expression(&qualified_op.lhs, type_checker, ExprContext::Temporary)?;

        // Map qualifier to C operator
        let c_operator = match qualifier.name.as_str() {
            "or" => " || ",
            "and" => " && ",
            "bitor" => " | ",
            "bitand" => " & ",
            "bitxor" => " ^ ",
            _ => {
                return Err(CodegenError::UnsupportedFeature {
                    feature: format!("qualifier '{}'", qualifier.name),
                    phase: "a future phase".to_string(),
                });
            }
        };

        self.output.push_str(c_operator);
        self.generate_expression(&qualified_op.rhs, type_checker, ExprContext::Temporary)?;

        Ok(())
    }

    fn generate_stealth_block(&mut self, block: &Block, _position: &Position, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Check for early returns inside the stealth block and reject them
        for item in &block.items {
            if let BlockItem::Statement(Statement::Return(_)) = item {
                return Err(CodegenError::UnsupportedFeature {
                    feature: "return inside stealth block".to_string(),
                    phase: "future phase (not Phase 10a)".to_string(),
                });
            }
        }

        // Emit stealth depth increment
        self.output.push_str("  hl_stealth_depth++;\n");

        // Enter a normal scope for heap cleanup
        self.enter_scope();

        // Generate the block body
        for item in &block.items {
            if let BlockItem::Statement(statement) = item {
                self.generate_statement(statement, type_checker)?;
            }
        }

        // Exit the scope (runs heap cleanup)
        self.exit_scope();

        // Emit stealth depth decrement
        self.output.push_str("  hl_stealth_depth--;\n");

        Ok(())
    }

    fn generate_fstring(&mut self, fstring: &FString, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Create a HiLowArray<u8> for the f-string result
        // This replaces the old malloc'd char* approach with proper HiLowArray
        self.output.push_str("({ HiLowArray* __fstring_arr = hl_array_new(sizeof(uint8_t), 0, NULL, NULL); ");

        for part in &fstring.parts {
            match part {
                FStringPart::Text(text) => {
                    if !text.is_empty() {
                        // Convert text to bytes and append
                        self.output.push_str("{ const char* __text = ");
                        // Emit C string literal for text part
                        self.output.push('"');
                        for ch in text.chars() {
                            match ch {
                                '"' => self.output.push_str("\\\""),
                                '\\' => self.output.push_str("\\\\"),
                                '\n' => self.output.push_str("\\n"),
                                '\t' => self.output.push_str("\\t"),
                                '\r' => self.output.push_str("\\r"),
                                c if (c as u32) < 0x20 && c != '\n' && c != '\t' && c != '\r' => {
                                    self.output.push_str(&format!("\\x{:02x}", c as u8));
                                }
                                c => {
                                    self.output.push(c);
                                }
                            }
                        }
                        self.output.push_str("\"; hl_array_append_bytes(__fstring_arr, (const uint8_t*)__text, strlen(__text)); } ");
                    }
                }
                FStringPart::Expression(expr, format_spec) => {
                    let expr_type = self.infer_expression_type_for_codegen(expr);

                    if let Some(format_spec) = format_spec {
                        // Handle special binary format case.
                        // hl_format_binary/hl_format_center return owned
                        // managed strings (HiLowArray*) — release after use.
                        if format_spec.type_code == Some('b') {
                            self.output.push_str("{ HiLowArray* __bin = hl_format_binary((unsigned long long)");
                            self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                            self.output.push_str("); ");

                            // Handle alignment for binary format
                            if format_spec.align == Some(Align::Center) && format_spec.width.is_some() {
                                self.output.push_str(&format!("HiLowArray* __centered = hl_format_center(__bin, {}); ", format_spec.width.unwrap()));
                                self.output.push_str("hl_array_append_bytes(__fstring_arr, (const uint8_t*)__centered->data, __centered->length); hl_array_release(__centered); ");
                            } else if let Some(width) = format_spec.width {
                                let pad_flag = if format_spec.align == Some(Align::Left) { "-" } else { "" };
                                self.output.push_str("{ char __padded_buf[128]; ");
                                self.output.push_str(&format!(
                                    "snprintf(__padded_buf, sizeof(__padded_buf), \"%{}*.*s\", {}, (int)__bin->length, (const char*)__bin->data); ",
                                    pad_flag, width
                                ));
                                self.output.push_str("hl_array_append_bytes(__fstring_arr, (const uint8_t*)__padded_buf, strlen(__padded_buf)); } ");
                            } else {
                                self.output.push_str("hl_array_append_bytes(__fstring_arr, (const uint8_t*)__bin->data, __bin->length); ");
                            }
                            self.output.push_str("hl_array_release(__bin); } ");
                        } else {
                            // Generate format string based on format spec
                            let c_format = self.generate_c_format_string(&expr_type, format_spec)?;

                            // Handle alignment if specified
                            if format_spec.align == Some(Align::Center) {
                                // Center alignment requires special handling - use runtime helper
                                self.output.push_str("{ char __tmp_buf[64]; sprintf(__tmp_buf, \"");
                                self.output.push_str(&c_format);
                                self.output.push_str("\", ");
                                self.generate_format_expression_with_cast(&expr_type, expr, type_checker)?;
                                self.output.push_str("); ");
                                if let Some(width) = format_spec.width {
                                    self.output.push_str(&format!("char* __centered_buf = hl_format_center(__tmp_buf, {}); ", width));
                                    self.output.push_str("hl_array_append_bytes(__fstring_arr, (const uint8_t*)__centered_buf, strlen(__centered_buf)); free(__centered_buf); } ");
                                } else {
                                    self.output.push_str("hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); } ");
                                }
                            } else {
                                // Standard sprintf with possible alignment
                                self.output.push_str("{ char __tmp_buf[64]; sprintf(__tmp_buf, \"");
                                self.output.push_str(&c_format);
                                self.output.push_str("\", ");
                                self.generate_format_expression_with_cast(&expr_type, expr, type_checker)?;
                                self.output.push_str("); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); } ");
                            }
                        }
                    } else {
                        // No format specifier - use default formatting
                        match expr_type {
                            Type::String => {
                                // String: concatenate directly (String is HiLowArray<u8>)
                                self.output.push_str("{ HiLowArray* __str_expr = ");
                                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                self.output.push_str("; hl_array_append_bytes(__fstring_arr, (const uint8_t*)__str_expr->data, __str_expr->length); } ");
                            }
                            Type::I8 | Type::I16 | Type::I32 | Type::Isize => {
                                // 32-bit integers
                                self.output.push_str("{ char __tmp_buf[32]; sprintf(__tmp_buf, \"%d\", (int)");
                                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                self.output.push_str("); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); } ");
                            }
                            Type::I64 => {
                                // 64-bit integers
                                self.output.push_str("{ char __tmp_buf[32]; sprintf(__tmp_buf, \"%lld\", (long long)");
                                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                self.output.push_str("); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); } ");
                            }
                            Type::U8 | Type::U16 | Type::U32 | Type::Usize => {
                                // 32-bit unsigned integers
                                self.output.push_str("{ char __tmp_buf[32]; sprintf(__tmp_buf, \"%u\", (unsigned int)");
                                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                self.output.push_str("); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); } ");
                            }
                            Type::U64 => {
                                // 64-bit unsigned integers
                                self.output.push_str("{ char __tmp_buf[32]; sprintf(__tmp_buf, \"%llu\", (unsigned long long)");
                                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                self.output.push_str("); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); } ");
                            }
                            Type::F32 => {
                                // 32-bit floats
                                self.output.push_str("{ char __tmp_buf[64]; sprintf(__tmp_buf, \"%g\", (double)");
                                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                self.output.push_str("); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); } ");
                            }
                            Type::F64 => {
                                // 64-bit floats
                                self.output.push_str("{ char __tmp_buf[64]; sprintf(__tmp_buf, \"%g\", ");
                                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                self.output.push_str("); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); } ");
                            }
                            Type::Bool => {
                                // Boolean: "true" or "false"
                                self.output.push_str("{ const char* __bool_str = (");
                                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                self.output.push_str(") ? \"true\" : \"false\"; hl_array_append_bytes(__fstring_arr, (const uint8_t*)__bool_str, strlen(__bool_str)); } ");
                            }
                            Type::Nothing => {
                                // Nothing: just emit "nothing"
                                self.output.push_str("hl_array_append_bytes(__fstring_arr, (const uint8_t*)\"nothing\", 7); ");
                            }
                            Type::UnknownType => {
                                // Unknown: emit "unknown: " + reason
                                self.output.push_str("{ hl_array_append_bytes(__fstring_arr, (const uint8_t*)\"unknown: \", 9); HiLowArray* __reason = hl_unknown_get_reason(");
                                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                self.output.push_str("); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__reason->data, __reason->length); hl_array_release(__reason); } ");
                            }
                            Type::Time => {
                                // Time: format using hl_time_format helper
                                self.output.push_str("{ const char* __tmp_str = hl_time_format(");
                                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                self.output.push_str("); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_str, strlen(__tmp_str)); free((char*)__tmp_str); } ");
                            }
                            Type::Duration => {
                                // Duration: format using hl_duration_format helper
                                self.output.push_str("{ const char* __tmp_str = hl_duration_format(");
                                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                self.output.push_str("); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_str, strlen(__tmp_str)); free((char*)__tmp_str); } ");
                            }
                            Type::Unknown => {
                                // Route through normal expression codegen for property access, etc.
                                self.output.push_str("{ const char* __tmp_str = ");
                                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                self.output.push_str("; hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_str, strlen(__tmp_str)); } ");
                            }
                            Type::Optional(inner_type) => {
                                // Optional: runtime dispatch between unknown and inner type
                                self.output.push_str("{ if (hl_is_unknown(");
                                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                self.output.push_str(")) { hl_array_append_bytes(__fstring_arr, (const uint8_t*)\"unknown: \", 9); HiLowArray* __reason = hl_unknown_get_reason(hl_optional_unwrap_unknown(");
                                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                self.output.push_str(")); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__reason->data, __reason->length); hl_array_release(__reason); } else { ");

                                match inner_type.as_ref() {
                                    Type::I8 | Type::I16 | Type::I32 | Type::Isize => {
                                        self.output.push_str("char __tmp_buf[32]; sprintf(__tmp_buf, \"%d\", hl_optional_unwrap_i32(");
                                        self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                        self.output.push_str(")); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); ");
                                    }
                                    Type::I64 => {
                                        self.output.push_str("char __tmp_buf[32]; sprintf(__tmp_buf, \"%lld\", hl_optional_unwrap_i64(");
                                        self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                        self.output.push_str(")); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); ");
                                    }
                                    Type::U8 | Type::U16 | Type::U32 | Type::Usize => {
                                        self.output.push_str("char __tmp_buf[32]; sprintf(__tmp_buf, \"%u\", hl_optional_unwrap_u32(");
                                        self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                        self.output.push_str(")); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); ");
                                    }
                                    Type::U64 => {
                                        self.output.push_str("char __tmp_buf[32]; sprintf(__tmp_buf, \"%llu\", hl_optional_unwrap_u64(");
                                        self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                        self.output.push_str(")); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); ");
                                    }
                                    Type::F32 => {
                                        self.output.push_str("char __tmp_buf[64]; sprintf(__tmp_buf, \"%g\", hl_optional_unwrap_f32(");
                                        self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                        self.output.push_str(")); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); ");
                                    }
                                    Type::F64 => {
                                        self.output.push_str("char __tmp_buf[64]; sprintf(__tmp_buf, \"%g\", hl_optional_unwrap_f64(");
                                        self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                        self.output.push_str(")); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); ");
                                    }
                                    Type::Bool => {
                                        self.output.push_str("const char* __bool_str = hl_optional_unwrap_bool(");
                                        self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                        self.output.push_str(") ? \"true\" : \"false\"; hl_array_append_bytes(__fstring_arr, (const uint8_t*)__bool_str, strlen(__bool_str)); ");
                                    }
                                    Type::String => {
                                        self.output.push_str("const char* __opt_str = hl_optional_unwrap_string(");
                                        self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                        self.output.push_str("); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__opt_str, strlen(__opt_str)); ");
                                    }
                                    Type::Time => {
                                        // Use print_time functionality for time formatting
                                        self.output.push_str("{ HiLowTime __time_tmp = hl_optional_unwrap_time(");
                                        self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                        self.output.push_str("); char __tmp_buf[64]; struct tm *tm = gmtime(&(time_t){__time_tmp.nanos_since_epoch / 1000000000}); strftime(__tmp_buf, 64, \"%Y-%m-%dT%H:%M:%S\", tm); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); }");
                                    }
                                    Type::Duration => {
                                        // Use print_duration functionality for duration formatting
                                        self.output.push_str("{ HiLowDuration __dur_tmp = hl_optional_unwrap_duration(");
                                        self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                        self.output.push_str("); char __tmp_buf[64]; int64_t nanos = __dur_tmp.nanos; if (nanos == 0) { hl_array_append_bytes(__fstring_arr, (const uint8_t*)\"0s\", 2); } else { sprintf(__tmp_buf, \"%lldns\", nanos); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); } }");
                                    }
                                    _ => {
                                        return Err(CodegenError::UnsupportedFeature {
                                            feature: format!("f-string interpolation for optional type {}", inner_type),
                                            phase: "later phases".to_string(),
                                        });
                                    }
                                }

                                self.output.push_str("} } ");
                            }
                            Type::Tuple(element_types) => {
                                // Use tuple print function to format the tuple
                                self.ensure_tuple_print_function(&element_types);
                                // Generate inline tuple formatting like: (1, 2, 3)
                                self.output.push_str("{ hl_array_append_bytes(__fstring_arr, (const uint8_t*)\"(\", 1); ");
                                for (i, element_type) in element_types.iter().enumerate() {
                                    if i > 0 {
                                        self.output.push_str("hl_array_append_bytes(__fstring_arr, (const uint8_t*)\", \", 2); ");
                                    }

                                    // Generate element-specific formatting
                                    match element_type {
                                        Type::I8 | Type::I16 | Type::I32 | Type::Isize => {
                                            self.output.push_str("{ char __tmp_buf[32]; sprintf(__tmp_buf, \"%d\", ");
                                            self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                            self.output.push_str(&format!("._{}); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); }}", i));
                                        }
                                        Type::I64 => {
                                            self.output.push_str("{ char __tmp_buf[32]; sprintf(__tmp_buf, \"%ld\", ");
                                            self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                            self.output.push_str(&format!("._{}); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); }}", i));
                                        }
                                        Type::U8 | Type::U16 | Type::U32 | Type::Usize => {
                                            self.output.push_str("{ char __tmp_buf[32]; sprintf(__tmp_buf, \"%u\", ");
                                            self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                            self.output.push_str(&format!("._{}); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); }}", i));
                                        }
                                        Type::U64 => {
                                            self.output.push_str("{ char __tmp_buf[32]; sprintf(__tmp_buf, \"%lu\", ");
                                            self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                            self.output.push_str(&format!("._{}); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); }}", i));
                                        }
                                        Type::F32 => {
                                            self.output.push_str("{ char __tmp_buf[64]; sprintf(__tmp_buf, \"%g\", ");
                                            self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                            self.output.push_str(&format!("._{}); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); }}", i));
                                        }
                                        Type::F64 => {
                                            self.output.push_str("{ char __tmp_buf[64]; sprintf(__tmp_buf, \"%g\", ");
                                            self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                            self.output.push_str(&format!("._{}); hl_array_append_bytes(__fstring_arr, (const uint8_t*)__tmp_buf, strlen(__tmp_buf)); }}", i));
                                        }
                                        Type::Bool => {
                                            self.output.push_str("{ const char* __bool_str = ");
                                            self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                            self.output.push_str(&format!("._{} ? \"true\" : \"false\"; hl_array_append_bytes(__fstring_arr, (const uint8_t*)__bool_str, strlen(__bool_str)); }}", i));
                                        }
                                        Type::String => {
                                            self.output.push_str("{ HiLowArray* __str_expr = ");
                                            self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                                            self.output.push_str(&format!("._{}; hl_array_append_bytes(__fstring_arr, (const uint8_t*)__str_expr->data, __str_expr->length); }}", i));
                                        }
                                        _ => {
                                            self.output.push_str("hl_array_append_bytes(__fstring_arr, (const uint8_t*)\"<unknown>\", 9); ");
                                        }
                                    }
                                }
                                self.output.push_str("hl_array_append_bytes(__fstring_arr, (const uint8_t*)\")\", 1); } ");
                            }
                            Type::ObjectIterValue => {
                                // Runtime dispatch for iteration value
                                self.generate_fstring_interpolation_for_iter_value(expr, type_checker)?;
                            }
                            _ => {
                                return Err(CodegenError::UnsupportedFeature {
                                    feature: format!("f-string interpolation of type {:?}", expr_type),
                                    phase: "Phase 6b-i".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        self.output.push_str("__fstring_arr; })");

        Ok(())
    }

    fn generate_c_format_string(&self, expr_type: &Type, format_spec: &FormatSpec) -> Result<String, CodegenError> {
        let mut c_format = String::from("%");

        // Add alignment and width flags
        if let Some(align) = &format_spec.align {
            match align {
                Align::Left => c_format.push('-'),
                Align::Right => {
                    // Right alignment is default in printf, no flag needed unless there's zero-padding
                }
                Align::Center => {
                    // Center alignment handled separately in caller
                }
            }
        }

        // Add fill character for zero-padding
        if format_spec.fill == Some('0') && format_spec.width.is_some() {
            c_format.push('0');
        } else if format_spec.fill.is_some() && format_spec.fill != Some(' ') {
            return Err(CodegenError::UnsupportedFeature {
                feature: "custom fill characters other than '0' and ' '".to_string(),
                phase: "Phase 6b-ii".to_string(),
            });
        }

        // Add width
        if let Some(width) = format_spec.width {
            if format_spec.align != Some(Align::Center) {
                c_format.push_str(&width.to_string());
            }
        }

        // Add precision
        if let Some(precision) = format_spec.precision {
            c_format.push('.');
            c_format.push_str(&precision.to_string());
        }

        // Add type specifier
        if let Some(type_code) = format_spec.type_code {
            match type_code {
                'd' => c_format.push('d'),
                'x' => c_format.push('x'),
                'X' => c_format.push('X'),
                'o' => c_format.push('o'),
                'b' => {
                    // Binary format uses our custom runtime helper
                    c_format.push('s');  // We'll format as string using hl_format_binary
                }
                'e' => c_format.push('e'),
                'E' => c_format.push('E'),
                'f' => c_format.push('f'),
                'g' => c_format.push('g'),
                's' => c_format.push('s'),
                'c' => c_format.push('c'),
                _ => {
                    return Err(CodegenError::UnsupportedFeature {
                        feature: format!("format type '{}'", type_code),
                        phase: "Phase 6b-ii".to_string(),
                    });
                }
            }
        } else {
            // Default format based on type
            match expr_type {
                Type::I8 | Type::I16 | Type::I32 | Type::Isize => c_format.push('d'),
                Type::I64 => c_format.push_str("lld"),
                Type::U8 | Type::U16 | Type::U32 | Type::Usize => c_format.push('u'),
                Type::U64 => c_format.push_str("llu"),
                Type::F32 | Type::F64 => c_format.push('g'),
                Type::String => c_format.push('s'),
                Type::Bool => c_format.push('s'), // We'll handle bool conversion separately
                _ => {
                    return Err(CodegenError::UnsupportedFeature {
                        feature: format!("default formatting for type {:?}", expr_type),
                        phase: "Phase 6b-ii".to_string(),
                    });
                }
            }
        }

        Ok(c_format)
    }

    fn generate_format_expression_with_cast(&mut self, expr_type: &Type, expr: &Expression, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Generate appropriate casting based on expression type and format requirements
        match expr_type {
            Type::I8 | Type::I16 | Type::I32 | Type::Isize => {
                self.output.push_str("(int)");
                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
            }
            Type::I64 => {
                self.output.push_str("(long long)");
                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
            }
            Type::U8 | Type::U16 | Type::U32 | Type::Usize => {
                self.output.push_str("(unsigned int)");
                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
            }
            Type::U64 => {
                self.output.push_str("(unsigned long long)");
                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
            }
            Type::F32 => {
                self.output.push_str("(double)");
                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
            }
            Type::F64 => {
                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
            }
            Type::String => {
                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
            }
            Type::Bool => {
                self.output.push_str("(");
                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                self.output.push_str(") ? \"true\" : \"false\"");
            }
            _ => {
                self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
            }
        }
        Ok(())
    }

    fn generate_object_literal(&mut self, obj_lit: &ObjectLiteral, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Phase 8b: Object properties with heap values now supported via refcounting

        // First, determine the complete object type for method context
        let mut object_properties = Vec::new();
        for (prop_name, prop_expr) in &obj_lit.properties {
            let prop_type = self.infer_expression_type_for_codegen(prop_expr);
            object_properties.push((prop_name.clone(), prop_type));
        }
        let object_type = Type::Object(object_properties);

        // Generate object creation: hl_object_new()
        self.output.push_str("({\n");
        self.output.push_str("    HiLowObject* obj = hl_object_new();\n");

        // Generate property assignments
        for (prop_name, prop_expr) in &obj_lit.properties {
            // Set method receiver context for function expressions
            let old_receiver_type = if matches!(prop_expr, Expression::FunctionExpr(_)) {
                let old = self.method_receiver_type.clone();
                self.method_receiver_type = Some(object_type.clone());
                old
            } else {
                None
            };

            // Determine the type of the property to call the right setter
            let expr_type = self.infer_expression_type_for_codegen(prop_expr);

            // Phase 1.5c: weak and object/function properties have their own
            // ownership shapes; primitives and strings share the plain-setter
            // emission below.
            if let Expression::WeakRef(inner_expr, _) = prop_expr {
                if matches!(expr_type, Type::Object(_)) {
                    // Weak property store: no retain, no release; the slot is
                    // nulled when the target dies. Phase 4a: a fresh-production
                    // value mints a statement-scoped temp released at
                    // statement end (the one mechanism); if nothing else owns
                    // it, it dies then and the weak slot nulls — before any
                    // later statement could read it, matching prior behavior.
                    self.output.push_str(&format!("    hl_object_set_object_weak(obj, \"{}\", ", prop_name));
                    self.generate_expression(inner_expr, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(");\n");
                    if matches!(prop_expr, Expression::FunctionExpr(_)) {
                        self.method_receiver_type = old_receiver_type.clone();
                    }
                    continue;
                }
            }
            match &expr_type {
                Type::Object(_) => {
                    // The store retains (Phase 1.5c axiom). Phase 4a: the
                    // value's own +1 is disposed by the one temp mechanism —
                    // fresh productions mint statement-scoped temps, borrows
                    // belong to their scope, tracked temps release at statement
                    // end; no site release.
                    self.output.push_str(&format!("    hl_object_set_object(obj, \"{}\", ", prop_name));
                    self.generate_expression(prop_expr, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(");\n");
                    if matches!(prop_expr, Expression::FunctionExpr(_)) {
                        self.method_receiver_type = old_receiver_type.clone();
                    }
                    continue;
                }
                Type::Function(_, _) => {
                    self.output.push_str(&format!("    hl_object_set_function(obj, \"{}\", ", prop_name));
                    self.generate_expression(prop_expr, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(");\n");
                    if matches!(prop_expr, Expression::FunctionExpr(_)) {
                        self.method_receiver_type = old_receiver_type.clone();
                    }
                    continue;
                }
                Type::DynamicArray(_) => {
                    // Phase 2e: array-valued properties — same ownership shape
                    // as objects (store retains). Phase 4a: no site release.
                    self.output.push_str(&format!("    hl_object_set_array(obj, \"{}\", ", prop_name));
                    self.generate_expression(prop_expr, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(");\n");
                    continue;
                }
                _ => {}
            }

            self.output.push_str(&format!("    hl_object_set_"));

            match expr_type {
                Type::I32 => {
                    self.output.push_str("i32(obj, \"");
                    self.output.push_str(prop_name);
                    self.output.push_str("\", ");
                    self.generate_expression(prop_expr, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(");\n");
                }
                Type::I64 => {
                    self.output.push_str("i64(obj, \"");
                    self.output.push_str(prop_name);
                    self.output.push_str("\", ");
                    self.generate_expression(prop_expr, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(");\n");
                }
                Type::U32 => {
                    self.output.push_str("u32(obj, \"");
                    self.output.push_str(prop_name);
                    self.output.push_str("\", ");
                    self.generate_expression(prop_expr, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(");\n");
                }
                Type::U64 => {
                    self.output.push_str("u64(obj, \"");
                    self.output.push_str(prop_name);
                    self.output.push_str("\", ");
                    self.generate_expression(prop_expr, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(");\n");
                }
                Type::F32 => {
                    self.output.push_str("f32(obj, \"");
                    self.output.push_str(prop_name);
                    self.output.push_str("\", ");
                    self.generate_expression(prop_expr, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(");\n");
                }
                Type::F64 => {
                    self.output.push_str("f64(obj, \"");
                    self.output.push_str(prop_name);
                    self.output.push_str("\", ");
                    self.generate_expression(prop_expr, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(");\n");
                }
                Type::Bool => {
                    self.output.push_str("bool(obj, \"");
                    self.output.push_str(prop_name);
                    self.output.push_str("\", ");
                    self.generate_expression(prop_expr, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(");\n");
                }
                Type::String => {
                    self.output.push_str("str(obj, \"");
                    self.output.push_str(prop_name);
                    self.output.push_str("\", ");
                    self.generate_expression(prop_expr, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(");\n");
                }
                _ => {
                    return Err(CodegenError::UnsupportedFeature {
                        feature: format!("object property of type {}", expr_type),
                        phase: "future phases".to_string(),
                    });
                }
            }

            // Restore old receiver type if it was changed
            if matches!(prop_expr, Expression::FunctionExpr(_)) {
                self.method_receiver_type = old_receiver_type;
            }
        }

        self.output.push_str("    obj;\n");
        self.output.push_str("})");

        Ok(())
    }

    fn generate_member_access(&mut self, member_access: &MemberAccess, type_checker: &TypeChecker, context: ExprContext) -> Result<(), CodegenError> {
        // Generate property access: hl_object_get_TYPE(obj, "property")

        // Determine the type of the property using prototype chain lookup (Phase 7b)
        let object_type = self.infer_expression_type_for_codegen(&member_access.object);
        let member_type = match object_type {
            Type::Object(_) => {
                // Use prototype chain lookup
                self.find_property_type_in_chain(&object_type, &member_access.member, 0)
                    .unwrap_or(Type::Nothing) // Missing properties return nothing
            }
            Type::UnknownType => {
                // Unknown types have known properties: reason and options
                match member_access.member.as_str() {
                    "reason" => Type::String,
                    "options" => Type::DynamicArray(Box::new(Type::String)),
                    _ => Type::Nothing, // Unknown properties return nothing
                }
            }
            Type::DynamicArray(_) => {
                // Arrays have .length property
                match member_access.member.as_str() {
                    "length" => Type::Usize,
                    _ => Type::Nothing,
                }
            }
            Type::Optional(ref inner) => {
                // Phase 1.5e: member access through a weak read propagates
                // (property T reads as T?, nested weak stays T?). Chain
                // lookup wins over the .reason/.options narrowed-unknown
                // fallback.
                let chain_hit = if let Type::Object(_) = inner.as_ref() {
                    self.find_property_type_in_chain(inner, &member_access.member, 0)
                } else {
                    None
                };
                if let Some(prop_type) = chain_hit {
                    if matches!(prop_type, Type::Optional(_)) {
                        prop_type
                    } else {
                        Type::Optional(Box::new(prop_type))
                    }
                } else if matches!(member_access.member.as_str(), "reason" | "options") {
                    // This is likely a narrowed optional accessing unknown properties
                    match member_access.member.as_str() {
                        "reason" => Type::String,
                        "options" => Type::DynamicArray(Box::new(Type::String)),
                        _ => Type::Nothing,
                    }
                } else {
                    Type::Unknown
                }
            }
            Type::Unknown => {
                // General inference failure
                Type::Unknown
            }
            _ => Type::Unknown
        };

        // Special case: array properties and methods
        if let Type::DynamicArray(elem_type) = &object_type {
            match member_access.member.as_str() {
                "length" => {
                    self.output.push_str("hl_array_len(");
                    self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;
                    self.output.push_str(")");
                    return Ok(());
                }
                "push" | "pop" | "remove" | "insert" => {
                    // Array Phase B/B-2: These are method references, not direct properties
                    // For now, we don't support storing array methods as function values
                    return Err(CodegenError::UnsupportedFeature {
                        feature: format!("array method '{}' as first-class value", member_access.member),
                        phase: "future phases".to_string(),
                    });
                }
                _ => {
                    return Err(CodegenError::UnsupportedFeature {
                        feature: format!("unknown array property '{}'", member_access.member),
                        phase: "Array Phase B".to_string(),
                    });
                }
            }
        }

        // Special case: string properties
        if let Type::String = &object_type {
            match member_access.member.as_str() {
                "bytelength" => {
                    self.output.push_str("((uint32_t)hl_array_len(");
                    self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;
                    self.output.push_str("))");
                    return Ok(());
                }
                _ => {
                    return Err(CodegenError::UnsupportedFeature {
                        feature: format!("unknown string property '{}'", member_access.member),
                        phase: "Managed Strings Sub-phase 1".to_string(),
                    });
                }
            }
        }

        // Phase 1.5e: reading a weak slot — the property's shape type is T?
        // on a plain object. hl_object_get_weak returns a fresh +1 optional:
        // the referent (retained) while alive, unknown "weak referent
        // released" after its death.
        if matches!(object_type, Type::Object(_)) && matches!(member_type, Type::Optional(_)) {
            return self.emit_optional_member_call(
                "hl_object_get_weak",
                &member_access.object,
                &member_access.member,
                type_checker,
                context,
            );
        }

        // Phase 1.5e: member access through a weak read (object-or-unknown
        // optional) — unknown propagates, a live referent's property is
        // wrapped in a fresh optional. The helper is picked by the property's
        // static type; the chain-hit requirement keeps .reason/.options on
        // narrowed unknowns on their existing path below.
        if let Type::Optional(ref inner) = object_type {
            if matches!(inner.as_ref(), Type::Object(_))
                && self.find_property_type_in_chain(inner, &member_access.member, 0).is_some()
            {
                let helper = match &member_type {
                    Type::Optional(prop) => match prop.as_ref() {
                        Type::I32 => "hl_optional_member_i32",
                        Type::String => "hl_optional_member_str",
                        Type::Object(_) => "hl_optional_member_object",
                        other => {
                            return Err(CodegenError::UnsupportedFeature {
                                feature: format!(
                                    "member access through a weak reference for property type {}",
                                    other
                                ),
                                phase: "future phases".to_string(),
                            });
                        }
                    },
                    other => {
                        return Err(CodegenError::UnsupportedFeature {
                            feature: format!(
                                "member access through a weak reference for property type {}",
                                other
                            ),
                            phase: "future phases".to_string(),
                        });
                    }
                };
                return self.emit_optional_member_call(
                    helper,
                    &member_access.object,
                    &member_access.member,
                    type_checker,
                    context,
                );
            }
        }

        // Special case: property access on unknown types or narrowed unknown values
        let treat_as_unknown = matches!(object_type, Type::UnknownType) ||
            (matches!(object_type, Type::Optional(_)) &&
             matches!(member_access.member.as_str(), "reason" | "options"));

        if treat_as_unknown {
            match member_access.member.as_str() {
                "reason" => {
                    // hl_unknown_get_reason returns a fresh managed string the
                    // caller owns. Build the call, then hoist it as a tracked
                    // temp in Temporary context (mirrors hl_object_get_str).
                    let saved_output = std::mem::take(&mut self.output);
                    self.output.push_str("hl_unknown_get_reason(");
                    // If this is a narrowed optional, unwrap the unknown first
                    if matches!(object_type, Type::Optional(_)) {
                        self.output.push_str("hl_optional_unwrap_unknown(");
                        self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;
                        self.output.push_str(")");
                    } else {
                        self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;
                    }
                    self.output.push_str(")");
                    let call_expr = std::mem::take(&mut self.output);
                    self.output = saved_output;

                    if context == ExprContext::Temporary {
                        let temp_name = self.next_temp_name();
                        self.temp_owners.insert(temp_name.clone(), HeapType::Array);
                        self.pending_statement_decls.push(format!("HiLowArray* {} = {};", temp_name, call_expr));
                        self.output.push_str(&temp_name);
                    } else {
                        self.output.push_str(&call_expr);
                    }
                    return Ok(());
                }
                "options" => {
                    self.output.push_str("hl_unknown_get_options(");
                    // If this is a narrowed optional, unwrap the unknown first
                    if matches!(object_type, Type::Optional(_)) {
                        self.output.push_str("hl_optional_unwrap_unknown(");
                        self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;
                        self.output.push_str(")");
                    } else {
                        self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;
                    }
                    self.output.push_str(")");
                    return Ok(());
                }
                _ => {
                    // Unknown properties return nothing
                    self.output.push_str("&the_nothing");
                    return Ok(());
                }
            }
        }

        // Phase 1.5c: only STRING member reads need temp tracking —
        // hl_object_get_str retains-on-return, so the +1 must be released at
        // statement end. Object/function member reads are pure BORROWS
        // (hl_object_get_object / hl_object_get_function do not retain);
        // tracking them released references the property still owns.
        let needs_temp_tracking = context == ExprContext::Temporary &&
            matches!(member_type, Type::String);

        if needs_temp_tracking {
            // Create temp variable for heap-returning property access
            let temp_name = self.next_temp_name();
            let heap_type = HeapType::Array;

            // Register temp for cleanup
            self.temp_owners.insert(temp_name.clone(), heap_type);

            let getter_func = "hl_object_get_str";
            let temp_type = "HiLowArray*";

            // Create declaration
            let mut decl = format!("{} {} = {}(", temp_type, temp_name, getter_func);

            // Temporarily capture the object and property expressions
            let saved_output = std::mem::take(&mut self.output);

            // Generate object
            self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;
            let object_expr = std::mem::take(&mut self.output);

            // Complete declaration
            decl.push_str(&object_expr);
            decl.push_str(&format!(", \"{}\");", member_access.member));

            // Hoist declaration to statement scope
            self.pending_statement_decls.push(decl);

            // Restore output and emit just the temp reference
            self.output = saved_output;
            self.output.push_str(&temp_name);
        } else {
            // Direct emission for non-heap types or owned context
            match member_type {
                Type::I32 => self.output.push_str("hl_object_get_i32("),
                Type::I64 => self.output.push_str("hl_object_get_i64("),
                Type::U32 => self.output.push_str("hl_object_get_u32("),
                Type::U64 => self.output.push_str("hl_object_get_u64("),
                Type::F32 => self.output.push_str("hl_object_get_f32("),
                Type::F64 => self.output.push_str("hl_object_get_f64("),
                Type::Bool => self.output.push_str("hl_object_get_bool("),
                Type::String => self.output.push_str("hl_object_get_str("),
                Type::Object(_) => self.output.push_str("hl_object_get_object("),
                Type::Function(_, _) => self.output.push_str("hl_object_get_function("),
                Type::DynamicArray(_) => self.output.push_str("hl_object_get_array("),  // Phase 2e: borrow
                Type::Nothing => {
                    // Property doesn't exist, return the nothing singleton
                    self.output.push_str("&the_nothing");
                    return Ok(()); // Don't emit object or property name
                }
                _ => {
                    return Err(CodegenError::UnsupportedFeature {
                        feature: format!("member access for type {}", member_type),
                        phase: "future phases".to_string(),
                    });
                }
            }

            // Generate the object expression
            self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;

            // Generate the property name as a string literal
            self.output.push_str(", \"");
            self.output.push_str(&member_access.member);
            self.output.push_str("\")");
        }

        Ok(())
    }

    /// Phase 1.5e: emit `<helper>(<object expr>, "<member>")` for the
    /// optional-returning weak-read helpers (hl_object_get_weak,
    /// hl_optional_member_*). They all return a fresh +1 HiLowOptional the
    /// caller owns: in Temporary context the call is hoisted as a tracked
    /// temp released at statement end (mirroring the .reason path); in Owned
    /// context the binding site takes ownership.
    fn emit_optional_member_call(
        &mut self,
        helper: &str,
        object_expr: &Expression,
        member: &str,
        type_checker: &TypeChecker,
        context: ExprContext,
    ) -> Result<(), CodegenError> {
        let saved_output = std::mem::take(&mut self.output);
        self.output.push_str(helper);
        self.output.push_str("(");
        self.generate_expression(object_expr, type_checker, ExprContext::Temporary)?;
        self.output.push_str(", \"");
        self.output.push_str(member);
        self.output.push_str("\")");
        let call_expr = std::mem::take(&mut self.output);
        self.output = saved_output;

        if context == ExprContext::Temporary {
            let temp_name = self.next_temp_name();
            self.temp_owners.insert(temp_name.clone(), HeapType::Optional);
            self.pending_statement_decls.push(format!("HiLowOptional* {} = {};", temp_name, call_expr));
            self.output.push_str(&temp_name);
        } else {
            self.output.push_str(&call_expr);
        }
        Ok(())
    }

    fn generate_function_expression(&mut self, func_expr: &FunctionExpr, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Generate unique function name
        let func_name = format!("hilow_anon_{}", self.function_counter);
        self.function_counter += 1;

        // Check if this function has captures and generate environment struct if needed
        let captures = func_expr.captures.borrow();
        let has_captures = !captures.is_empty();

        // Phase 8b: Escaping closures with captures now supported via refcounting

        let env_struct_name = if has_captures {
            let struct_name = format!("hilow_anon_{}_env", self.function_counter - 1);
            self.generate_environment_struct(func_expr, &struct_name);
            Some(struct_name)
        } else {
            None
        };
        drop(captures); // Release the borrow

        // Determine return type
        let return_type = Type::from_ast_type(&func_expr.return_type);
        let c_return_type = self.hilow_type_to_c(&return_type);

        // Generate function signature
        self.generated_functions.push_str(&format!("{} {}(", c_return_type, func_name));

        // ALL function expressions take a void* env parameter as the first argument (Phase 7c-δ)
        self.generated_functions.push_str("void* env");

        // For method expressions (when method_receiver_type is set), add this_obj parameter
        if self.method_receiver_type.is_some() {
            self.generated_functions.push_str(", HiLowObject* this_obj");
        }

        // Generate user-defined parameters
        for param in func_expr.params.iter() {
            self.generated_functions.push_str(", ");
            let param_type = Type::from_ast_type(&param.ty);
            let c_param_type = self.hilow_type_to_c(&param_type);
            self.generated_functions.push_str(&format!("{} {}", c_param_type, param.name));
        }

        self.generated_functions.push_str(") {\n");

        // Store current output and switch to function body generation
        let main_output = self.output.clone();
        self.output.clear();

        // Save current environment state and set up closure environment context
        let old_variable_types = self.variable_types.clone();
        let old_hoisted_variables = self.hoisted_variables.clone();
        let old_current_env_var = self.current_env_var.clone();

        // Phase 1.5b: control-transfer context is per-C-function — the anon
        // body must not see the enclosing function's statement temps or loops
        let saved_temp_frames = std::mem::take(&mut self.enclosing_temp_frames);
        let saved_loop_frames = std::mem::take(&mut self.loop_frames);
        let saved_temp_owners = std::mem::take(&mut self.temp_owners);
        let saved_pending_decls = std::mem::take(&mut self.pending_statement_decls);
        let saved_in_c_switch = self.in_c_switch;
        let saved_in_string_switch = self.in_string_switch;
        self.in_c_switch = false;
        self.in_string_switch = false;

        // Set up environment for captured variables within the closure
        if has_captures {
            if let Some(env_struct_name) = &env_struct_name {
                // Cast the env parameter to the correct struct type
                let env_var_name = "env_cast";
                self.output.push_str(&format!("  {}* {} = ({}*)env;\n",
                    env_struct_name, env_var_name, env_struct_name));

                // Set up hoisted variables mapping for the closure
                self.hoisted_variables.clear();
                self.current_env_var = Some(env_var_name.to_string());

                // Map captured variables to use the cast environment
                let captures = func_expr.captures.borrow();
                for (var_name, ast_type, _pos) in captures.iter() {
                    self.hoisted_variables.insert(var_name.clone(), (env_var_name.to_string(), env_struct_name.clone()));

                    // Add captured variable types to variable_types for type-directed dispatch (Fix for Bug 2)
                    let hilow_type = Type::from_ast_type(ast_type);
                    self.variable_types.insert(var_name.clone(), hilow_type);
                }
            }
        } else {
            // Clear environment context for non-capturing function
            self.hoisted_variables.clear();
            self.current_env_var = None;
        }

        // Create new scope for function parameters in variable_types
        for param in &func_expr.params {
            let param_type = Type::from_ast_type(&param.ty);
            self.variable_types.insert(param.name.clone(), param_type);
        }

        // Generate function body (not in main program context)
        let old_in_main_program = self.in_main_program;
        self.in_main_program = false;
        for stmt in func_expr.body.statements_iter() {
            self.generate_statement(stmt, type_checker)?;
        }
        self.in_main_program = old_in_main_program;

        // Restore environment state
        self.variable_types = old_variable_types;
        self.hoisted_variables = old_hoisted_variables;
        self.current_env_var = old_current_env_var;
        self.enclosing_temp_frames = saved_temp_frames;
        self.loop_frames = saved_loop_frames;
        self.temp_owners = saved_temp_owners;
        self.pending_statement_decls = saved_pending_decls;
        self.in_c_switch = saved_in_c_switch;
        self.in_string_switch = saved_in_string_switch;

        // Move function body to generated_functions and restore main output
        self.generated_functions.push_str(&self.output);
        self.generated_functions.push_str("}\n\n");
        self.output = main_output;

        // Generate function value creation
        if has_captures {
            // Use hl_function_new_with_env when there are captures
            if let Some(env_var) = &self.current_env_var {
                if let Some(dtor_name) = self.env_dtors.get(env_var) {
                    // Env owns retained heap fields — attach the destructor so
                    // hl_function_free releases them before freeing the env
                    self.output.push_str(&format!("hl_function_new_with_env_dtor((void*){}, {}, {})", func_name, env_var, dtor_name));
                } else {
                    self.output.push_str(&format!("hl_function_new_with_env((void*){}, {})", func_name, env_var));
                }
            } else {
                // Fallback to regular function (shouldn't happen if captures exist)
                self.output.push_str(&format!("hl_function_new((void*){})", func_name));
            }
        } else {
            // Non-capturing function expression - use hl_function_new with NULL env
            self.output.push_str(&format!("hl_function_new((void*){})", func_name));
        }

        Ok(())
    }

    fn generate_function_value_call(
        &mut self,
        call: &Call,
        param_types: &[Type],
        return_type: &Type,
        type_checker: &TypeChecker
    ) -> Result<(), CodegenError> {
        // Generate function pointer call: ((return_type(*)(void*, param_types))(fn_value->fn_ptr))(fn_value->env, args)
        // ALL function expressions take void* env as first parameter (Phase 7c-δ)

        let c_return_type = self.hilow_type_to_c(return_type);
        self.output.push_str(&format!("(({}(*)(", c_return_type));

        // Always include void* env as first parameter
        self.output.push_str("void*");

        // Generate user-defined parameter types
        for param_type in param_types.iter() {
            self.output.push_str(", ");
            let c_param_type = self.hilow_type_to_c(param_type);
            self.output.push_str(&c_param_type);
        }

        self.output.push_str("))(((HiLowFunction*)");
        self.generate_expression(&call.callee, type_checker, ExprContext::Temporary)?;
        self.output.push_str(")->fn_ptr))(((HiLowFunction*)");

        // Generate the callee expression again to get the environment
        self.generate_expression(&call.callee, type_checker, ExprContext::Temporary)?;
        self.output.push_str(")->env");

        // Generate user arguments
        for arg in call.args.iter() {
            self.output.push_str(", ");
            self.generate_expression(arg, type_checker, ExprContext::Temporary)?;
        }

        self.output.push_str(")");
        Ok(())
    }

    fn generate_time_builtin_call(
        &mut self,
        call: &Call,
        member_access: &MemberAccess,
        _type_checker: &TypeChecker,
        context: ExprContext
    ) -> Result<(), CodegenError> {
        match member_access.member.as_str() {
            "now" => {
                if !call.args.is_empty() {
                    return Err(CodegenError::UnsupportedFeature {
                        feature: "time.now() with arguments".to_string(),
                        phase: "Phase 9c".to_string(),
                    });
                }
                self.output.push_str("hl_time_now()");
                Ok(())
            }
            "parse" => {
                if call.args.len() != 1 {
                    return Err(CodegenError::UnsupportedFeature {
                        feature: "time.parse() with incorrect number of arguments".to_string(),
                        phase: "Phase 9c".to_string(),
                    });
                }

                if context == ExprContext::Temporary {
                    // time.parse() returns optional, which is heap-allocated - track as temp
                    let temp_name = self.next_temp_name();
                    self.temp_owners.insert(temp_name.clone(), HeapType::Optional);

                    // Build declaration with hl_time_parse call
                    let mut decl = format!("HiLowOptional* {} = hl_time_parse(", temp_name);

                    // Temporarily capture the argument
                    let saved_output = std::mem::take(&mut self.output);
                    self.generate_expression(&call.args[0], _type_checker, ExprContext::Temporary)?;
                    let arg_output = std::mem::take(&mut self.output);

                    // Complete declaration
                    decl.push_str(&arg_output);
                    decl.push_str(");");

                    // Hoist declaration to statement scope
                    self.pending_statement_decls.push(decl);

                    // Restore output and emit temp reference
                    self.output = saved_output;
                    self.output.push_str(&temp_name);
                } else {
                    // Direct emission for owned context
                    self.output.push_str("hl_time_parse(");
                    self.generate_expression(&call.args[0], _type_checker, ExprContext::Temporary)?;
                    self.output.push_str(")");
                }
                Ok(())
            }
            _ => Err(CodegenError::UnsupportedFeature {
                feature: format!("time builtin method '{}'", member_access.member),
                phase: "Phase 9c".to_string(),
            })
        }
    }

    fn generate_member_function_call(
        &mut self,
        call: &Call,
        member_access: &MemberAccess,
        type_checker: &TypeChecker,
        context: ExprContext
    ) -> Result<(), CodegenError> {
        // Special handling for time builtin methods
        if let Expression::Ident { name, .. } = member_access.object.as_ref() {
            if name == "time" {
                return self.generate_time_builtin_call(call, member_access, type_checker, context);
            }
        }

        // For obj.fnProp() calls, retrieve the function and call it
        // This is a simplified approach - we assume the property is a function value

        // Generate: ((return_type(*)(void*, args))(hl_object_get_function(obj, "prop")->fn_ptr))(obj.prop->env, args)
        // ALL function expressions take void* env as first parameter (Phase 7c-δ)
        self.output.push_str("((int32_t(*)(");

        // Always include void* env as first parameter
        self.output.push_str("void*");

        // For method calls, include HiLowObject* this_obj as second parameter
        self.output.push_str(", HiLowObject*");

        // Assume all user parameters are int32_t for now (this is a limitation that can be improved)
        for _i in 0..call.args.len() {
            self.output.push_str(", int32_t");
        }

        self.output.push_str("))(hl_object_get_function(");
        self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;
        self.output.push_str(&format!(", \"{}\")->fn_ptr))(", member_access.member));

        // Pass the environment as first argument
        self.output.push_str("hl_object_get_function(");
        self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;
        self.output.push_str(&format!(", \"{}\")->env", member_access.member));

        // Pass the receiver object as this_obj (second argument)
        self.output.push_str(", ");
        self.generate_expression(&member_access.object, type_checker, ExprContext::Temporary)?;

        // Generate user arguments
        for arg in call.args.iter() {
            self.output.push_str(", ");
            self.generate_expression(arg, type_checker, ExprContext::Temporary)?;
        }

        self.output.push_str(")");
        Ok(())
    }

    /// Set up environment allocation for a block that has captured locals
    /// Returns whether an environment was needed
    fn setup_environment_for_block(&mut self, block: &Block) -> Result<bool, CodegenError> {
        let captured_locals = self.analyze_captured_locals_in_block(block);

        if captured_locals.is_empty() {
            return Ok(false);
        }

        // Generate unique environment variable name
        let env_var = format!("env_{}", self.var_counter);
        self.var_counter += 1;

        // Generate environment struct type name
        let env_struct_name = format!("hilow_env_{}", self.var_counter);

        // Generate the struct definition
        self.environment_structs.push_str(&format!("typedef struct {} {{\n", env_struct_name));
        for (var_name, var_type) in &captured_locals {
            let c_type = self.hilow_type_to_c(var_type);
            self.environment_structs.push_str(&format!("    {} {};\n", c_type, var_name));
        }
        self.environment_structs.push_str(&format!("}} {};\n\n", env_struct_name));

        // Allocate the environment
        self.output.push_str(&format!("  {}* {} = malloc(sizeof({}));\n",
                                     env_struct_name, env_var, env_struct_name));
        self.output.push_str("  hl_alloc_count++;\n");

        // Track hoisted variables and current environment
        for (var_name, _var_type) in captured_locals {
            self.hoisted_variables.insert(var_name.clone(), (env_var.clone(), env_struct_name.clone()));
        }
        self.current_env_var = Some(env_var);

        Ok(true)
    }

    /// Set up environment allocation for a block that has captured locals, with parameter copying
    /// Returns whether an environment was needed
    fn setup_environment_for_block_with_params(&mut self, block: &Block, params: &[Parameter]) -> Result<bool, CodegenError> {
        let captured_locals = self.analyze_captured_locals_in_block(block);

        if captured_locals.is_empty() {
            return Ok(false);
        }

        // Generate unique environment variable name
        let env_var = format!("env_{}", self.var_counter);
        self.var_counter += 1;

        // Generate environment struct type name
        let env_struct_name = format!("hilow_env_{}", self.var_counter);

        // Generate the struct definition
        self.environment_structs.push_str(&format!("typedef struct {} {{\n", env_struct_name));
        for (var_name, var_type) in &captured_locals {
            let c_type = self.hilow_type_to_c(var_type);
            self.environment_structs.push_str(&format!("    {} {};\n", c_type, var_name));
        }
        self.environment_structs.push_str(&format!("}} {};\n\n", env_struct_name));

        // Allocate the environment
        self.output.push_str(&format!("  {}* {} = malloc(sizeof({}));\n",
                                     env_struct_name, env_var, env_struct_name));
        self.output.push_str("  hl_alloc_count++;\n");

        // Copy captured parameters to environment (Fix for Bug 1).
        // String params are retained: the caller releases its argument at
        // statement end, so the env must own its own reference. The matching
        // release lives in a generated env destructor run by hl_function_free.
        let mut retained_string_fields: Vec<String> = Vec::new();
        for (var_name, var_type) in &captured_locals {
            // Check if this captured variable is a function parameter
            if params.iter().any(|p| p.name == *var_name) {
                self.output.push_str(&format!("  {}->{} = {};\n", env_var, var_name, var_name));
                if matches!(var_type, Type::String) {
                    self.output.push_str(&format!("  hl_array_retain({}->{});\n", env_var, var_name));
                    retained_string_fields.push(var_name.clone());
                }
            }
        }

        // Emit a destructor releasing exactly the retained fields
        if !retained_string_fields.is_empty() {
            let dtor_name = format!("{}_dtor", env_struct_name);
            self.environment_structs.push_str(&format!("void {}(void* raw) {{\n", dtor_name));
            self.environment_structs.push_str(&format!("    {}* e = ({}*)raw;\n", env_struct_name, env_struct_name));
            for field in &retained_string_fields {
                self.environment_structs.push_str(&format!("    hl_array_release(e->{});\n", field));
            }
            self.environment_structs.push_str("}\n\n");
            self.env_dtors.insert(env_var.clone(), dtor_name);
        }

        // Track hoisted variables and current environment
        for (var_name, _var_type) in captured_locals {
            self.hoisted_variables.insert(var_name.clone(), (env_var.clone(), env_struct_name.clone()));
        }
        self.current_env_var = Some(env_var);

        Ok(true)
    }

    /// Generate environment struct type for a function expression with captures
    /// Returns the struct name to use for this environment
    fn generate_environment_struct(&mut self, func_expr: &FunctionExpr, struct_name: &str) -> String {
        let captures = func_expr.captures.borrow();

        if captures.is_empty() {
            return String::new(); // No environment needed
        }

        // Generate struct definition
        self.environment_structs.push_str(&format!("typedef struct {} {{\n", struct_name));

        for (var_name, ast_type, _pos) in captures.iter() {
            let hilow_type = Type::from_ast_type(ast_type);
            let c_type = self.hilow_type_to_c(&hilow_type);
            self.environment_structs.push_str(&format!("    {} {};\n", c_type, var_name));
        }

        self.environment_structs.push_str(&format!("}} {};\n\n", struct_name));

        struct_name.to_string()
    }

    /// Analyze which local variables are captured by function expressions within a function/program body
    /// Returns a map from variable name to its type for variables that need to be hoisted to environment
    fn analyze_captured_locals_in_block(&self, block: &Block) -> HashMap<String, Type> {
        let mut captured_locals = HashMap::new();

        // Find all function expressions in this block and collect their captures
        let statements_vec: Vec<&Statement> = block.statements_iter().collect();
        for stmt in statements_vec {
            self.collect_captures_from_statement(stmt, &mut captured_locals);
        }

        captured_locals
    }

    /// Recursively collect captures from all function expressions in statements
    fn collect_captures_from_statements(&self, statements: &[Statement], captured_locals: &mut HashMap<String, Type>) {
        for stmt in statements {
            self.collect_captures_from_statement(stmt, captured_locals);
        }
    }

    fn collect_captures_from_statement(&self, stmt: &Statement, captured_locals: &mut HashMap<String, Type>) {
        match stmt {
            Statement::Let(let_stmt) => {
                if let Some(init) = &let_stmt.initializer {
                    self.collect_captures_from_expression(init, captured_locals);
                }
            }
            Statement::Return(return_stmt) => {
                if let Some(expr) = &return_stmt.value {
                    self.collect_captures_from_expression(expr, captured_locals);
                }
            }
            Statement::If(if_stmt) => {
                self.collect_captures_from_expression(&if_stmt.condition, captured_locals);
                for stmt in if_stmt.then_block.statements_iter() {
                    self.collect_captures_from_statement(stmt, captured_locals);
                }
                if let Some(else_block) = &if_stmt.else_block {
                    for stmt in else_block.statements_iter() {
                        self.collect_captures_from_statement(stmt, captured_locals);
                    }
                }
            }
            Statement::While(while_stmt) => {
                self.collect_captures_from_expression(&while_stmt.condition, captured_locals);
                for stmt in while_stmt.body.statements_iter() {
                    self.collect_captures_from_statement(stmt, captured_locals);
                }
            }
            Statement::Loop(loop_stmt) => {
                for stmt in loop_stmt.body.statements_iter() {
                    self.collect_captures_from_statement(stmt, captured_locals);
                }
            }
            Statement::Assign(assign_stmt) => {
                self.collect_captures_from_expression(&assign_stmt.value, captured_locals);
            }
            Statement::ExprStatement(expr) => {
                self.collect_captures_from_expression(expr, captured_locals);
            }
            Statement::QualifiedOp(qualified_op) => {
                self.collect_captures_from_expression(&qualified_op.lhs, captured_locals);
                self.collect_captures_from_expression(&qualified_op.rhs, captured_locals);
            }
            _ => {} // Break, Continue don't contain expressions
        }
    }

    /// Recursively collect captures from function expressions in an expression
    fn collect_captures_from_expression(&self, expr: &Expression, captured_locals: &mut HashMap<String, Type>) {
        match expr {
            Expression::FunctionExpr(func_expr) => {
                // This is a function expression - check its captures
                let captures = func_expr.captures.borrow();
                for (var_name, ast_type, _pos) in captures.iter() {
                    let hilow_type = Type::from_ast_type(ast_type);
                    captured_locals.insert(var_name.clone(), hilow_type);
                }

                // Also recursively check for nested function expressions in the body
                for stmt in func_expr.body.statements_iter() {
                    self.collect_captures_from_statement(stmt, captured_locals);
                }
            }
            Expression::BinaryOp(binary_op) => {
                self.collect_captures_from_expression(&binary_op.lhs, captured_locals);
                self.collect_captures_from_expression(&binary_op.rhs, captured_locals);
            }
            Expression::UnaryOp(unary_op) => {
                self.collect_captures_from_expression(&unary_op.operand, captured_locals);
            }
            Expression::Call(call) => {
                self.collect_captures_from_expression(&call.callee, captured_locals);
                for arg in &call.args {
                    self.collect_captures_from_expression(arg, captured_locals);
                }
            }
            Expression::MemberAccess(member_access) => {
                self.collect_captures_from_expression(&member_access.object, captured_locals);
            }
            Expression::IndexAccess(index_access) => {
                self.collect_captures_from_expression(&index_access.object, captured_locals);
                self.collect_captures_from_expression(&index_access.index, captured_locals);
            }
            Expression::ObjectLiteral(object_literal) => {
                for prop in &object_literal.properties {
                    self.collect_captures_from_expression(&prop.1, captured_locals);
                }
            }
            Expression::FString(fstring) => {
                for part in &fstring.parts {
                    if let FStringPart::Expression(expr, _format_spec) = part {
                        self.collect_captures_from_expression(expr, captured_locals);
                    }
                }
            }
            Expression::QualifiedOp(qualified_op) => {
                self.collect_captures_from_expression(&qualified_op.lhs, captured_locals);
                self.collect_captures_from_expression(&qualified_op.rhs, captured_locals);
            }
            Expression::Match(match_expr) => {
                self.collect_captures_from_expression(&match_expr.value, captured_locals);
                for arm in &match_expr.arms {
                    match &arm.body {
                        MatchBody::Expression(expr) => {
                            self.collect_captures_from_expression(expr, captured_locals);
                        }
                        MatchBody::Block(block) => {
                            for stmt in block.statements_iter() {
                                self.collect_captures_from_statement(stmt, captured_locals);
                            }
                        }
                    }
                }
            }
            // Literals and identifiers don't contain function expressions
            _ => {}
        }
    }

    fn generate_match_expression(&mut self, match_expr: &MatchExpr, type_checker: &TypeChecker, context: ExprContext) -> Result<(), CodegenError> {
        // Generate C code for match expression
        // Strategy: use if-else chain with a temporary variable for the matched value

        // Get type of matched expression
        let matched_type = self.infer_expression_type_for_codegen(&match_expr.value);
        let c_type = self.hilow_type_to_c(&matched_type);

        // Check if we need to produce a result (expression context)
        let result_type = self.infer_expression_type_for_codegen(&Expression::Match(match_expr.clone()));
        let need_result = result_type != Type::Nothing && self.has_expression_body(match_expr);

        if need_result {
            // Expression context: use compound statement
            self.output.push_str("({\n");
        } else {
            // Statement context: direct if-else
            self.output.push_str("{\n");
        }

        // Emit temp variable declaration and assignment
        self.output.push_str(&format!("    {} __match_val = ", c_type));
        self.generate_expression(&match_expr.value, type_checker, ExprContext::Temporary)?;
        self.output.push_str(";\n");

        // For match-as-expression, also declare result variable
        if need_result {
            let result_c_type = self.hilow_type_to_c(&result_type);
            self.output.push_str(&format!("    {} __match_result;\n", result_c_type));
        }

        // Generate if-else chain for arms
        let mut first_arm = true;
        for arm in &match_expr.arms {
            if first_arm {
                self.output.push_str("    if (");
                first_arm = false;
            } else {
                self.output.push_str(" else if (");
            }

            // Generate condition for pattern
            match &arm.pattern {
                MatchPattern::Literal(literal) => {
                    self.generate_pattern_condition(&matched_type, literal)?;
                }
                MatchPattern::Wildcard => {
                    // Wildcard matches everything - use 1 for true
                    self.output.push_str("1");
                }
            }

            self.output.push_str(") {\n");

            // Generate body
            match &arm.body {
                MatchBody::Expression(expr) => {
                    if need_result {
                        // The arm value IS the match result — generate it in the
                        // incoming context. Owned (let/return): ownership passes
                        // to the binding; Temporary: statement-end temp cleanup.
                        //
                        // Phase 1.5c: an object/function-typed match expression
                        // must always yield a +1 reference the consumer
                        // disposes of. Wrap arm values that don't already
                        // produce an untracked fresh +1: in Owned context,
                        // borrows; in Temporary context, everything except
                        // fresh literals (tracked temps are released at
                        // statement end, so the wrap keeps the result alive).
                        let ref_wrap = match &result_type {
                            Type::Object(_) | Type::Function(_, _) => {
                                let needs_wrap = if context == ExprContext::Owned {
                                    Self::expr_is_borrowed_ref(expr)
                                } else {
                                    !matches!(expr, Expression::ObjectLiteral(_) | Expression::FunctionExpr(_))
                                };
                                if needs_wrap {
                                    Some(if matches!(result_type, Type::Object(_)) {
                                        "hl_object_ref"
                                    } else {
                                        "hl_function_ref"
                                    })
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        };
                        self.output.push_str("        __match_result = ");
                        if let Some(ref_fn) = ref_wrap {
                            self.output.push_str(&format!("{}(", ref_fn));
                        }
                        self.generate_expression(expr, type_checker, context.clone())?;
                        if ref_wrap.is_some() {
                            self.output.push_str(")");
                        }
                        self.output.push_str(";\n");
                    } else {
                        self.output.push_str("        ");
                        self.generate_expression(expr, type_checker, ExprContext::Temporary)?;
                        self.output.push_str(";\n");
                    }
                }
                MatchBody::Block(block) => {
                    // Generate block statements with proper indentation
                    for stmt in block.statements_iter() {
                        self.output.push_str("        ");
                        self.generate_statement(stmt, type_checker)?;
                    }
                }
            }

            self.output.push_str("    }");
        }

        // Close the if-else chain
        self.output.push_str("\n");

        // Return result for expression context
        if need_result {
            self.output.push_str("    __match_result;\n");
            self.output.push_str("})");
        } else {
            self.output.push_str("}");
        }

        Ok(())
    }

    fn has_expression_body(&self, match_expr: &MatchExpr) -> bool {
        // Check if any arm has an expression body (not just blocks)
        match_expr.arms.iter().any(|arm| matches!(arm.body, MatchBody::Expression(_)))
    }

    fn generate_pattern_condition(&mut self, matched_type: &Type, literal: &Literal) -> Result<(), CodegenError> {
        // Generate condition to check if __match_val equals the literal
        match literal {
            Literal::Integer(n) => {
                self.output.push_str(&format!("__match_val == {}", n));
            }
            Literal::Float(f) => {
                self.output.push_str(&format!("__match_val == {}", f));
            }
            Literal::String(s) => {
                self.output.push_str(&format!("hl_string_eq_cstr(__match_val, \"{}\")", Self::escape_c_string(s)));
            }
            Literal::Bool(b) => {
                let value = if *b { "1" } else { "0" };
                self.output.push_str(&format!("__match_val == {}", value));
            }
        }
        Ok(())
    }

    /// Escape a string for use in a C string literal
    fn escape_c_string(s: &str) -> String {
        let mut result = String::new();
        for ch in s.chars() {
            match ch {
                '"' => result.push_str("\\\""),
                '\\' => result.push_str("\\\\"),
                '\n' => result.push_str("\\n"),
                '\t' => result.push_str("\\t"),
                '\r' => result.push_str("\\r"),
                c if (c as u32) < 0x20 && c != '\n' && c != '\t' && c != '\r' => {
                    // Escape control characters below 0x20 (except \n, \t, \r already handled)
                    result.push_str(&format!("\\x{:02x}", c as u8));
                }
                c => {
                    // Emit UTF-8 bytes directly - C99/C11 supports arbitrary bytes in string literals
                    result.push(c);
                }
            }
        }
        result
    }

    fn generate_unknown_constructor(&mut self, unknown_construction: &UnknownConstruction, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // For now, we'll only support the simple case: unknown("reason")
        // Options support is deferred for simplicity
        if unknown_construction.options.is_some() {
            return Err(CodegenError::UnsupportedFeature {
                feature: "unknown constructor with options".to_string(),
                phase: "Phase 9b - options support deferred".to_string(),
            });
        }

        // Generate: hl_unknown_new(reason) - pass HiLowArray directly
        self.output.push_str("hl_unknown_new(");
        self.generate_expression(&unknown_construction.reason, type_checker, ExprContext::Temporary)?;
        self.output.push(')');

        Ok(())
    }

    // Phase 8a: Ownership tracking methods

    /// Phase 1.5c ownership axiom: does this expression evaluate to a BORROWED
    /// heap reference (owned by a named local, a container, or the receiver),
    /// as opposed to producing a reference of its own? Borrowed values are
    /// never released by store sites and need a retain when bound or returned.
    fn expr_is_borrowed_ref(expr: &Expression) -> bool {
        match expr {
            Expression::Ident { .. } | Expression::This(_) => true,
            // hl_object_get_object / hl_object_get_function return borrows.
            // (String member access is NOT borrowed: hl_object_get_str
            // retains-on-return.)
            Expression::MemberAccess(_) => true,
            // hl_array_get returns a pointer into the array — a borrow
            Expression::IndexAccess(_) => true,
            Expression::TypeAscription(inner, _, _) => Self::expr_is_borrowed_ref(inner),
            Expression::WeakRef(inner, _) => Self::expr_is_borrowed_ref(inner),
            _ => false,
        }
    }


    /// Phase 4a: if this expression is a fresh heap production that must be
    /// minted as a statement-scoped temp in Temporary context, return its
    /// (HeapType, C type). None for borrows and for the forms that already
    /// mint their own temps in their generate_expression arm (StringLit,
    /// string-concat, heap-returning calls, string/optional member reads).
    /// This is the exact set the deleted needs_site_release_after_store named:
    /// object/function/f-string/array literals and object/function-typed match.
    fn fresh_production_temp_kind(&self, expr: &Expression) -> Option<(HeapType, &'static str)> {
        match expr {
            Expression::ObjectLiteral(_) => Some((HeapType::Object, "HiLowObject*")),
            Expression::ArrayLit(_, _) => Some((HeapType::Array, "HiLowArray*")),
            Expression::FString(_) => Some((HeapType::Array, "HiLowArray*")),
            Expression::FunctionExpr(_) => Some((HeapType::Function, "HiLowFunction*")),
            Expression::Match(_) => match self.infer_expression_type_for_codegen(expr) {
                Type::Object(_) => Some((HeapType::Object, "HiLowObject*")),
                Type::Function(_, _) => Some((HeapType::Function, "HiLowFunction*")),
                _ => None,
            },
            Expression::TypeAscription(inner, _, _) => self.fresh_production_temp_kind(inner),
            _ => None,
        }
    }

    /// Record that a variable owns a heap allocation
    fn track_heap_owner(&mut self, var_name: &str, heap_type: HeapType) {
        self.heap_owners.insert(var_name.to_string(), (heap_type, self.scope_depth));
    }

    /// Mark a variable as having transferred ownership (don't free it)
    fn transfer_ownership(&mut self, var_name: &str) {
        self.transferred_vars.insert(var_name.to_string());
    }

    /// Enter a new scope (increase scope depth)
    fn enter_scope(&mut self) {
        self.scope_depth += 1;
    }

    /// Exit current scope and emit cleanup for variables declared in this scope
    fn exit_scope(&mut self) {
        self.emit_scope_cleanup(self.scope_depth);
        // Phase 3b: drop this scope's boxed-binding records (stack order —
        // entries are pushed in declaration order, deepest last)
        while matches!(self.boxed_bindings.last(), Some((_, d, _)) if *d == self.scope_depth) {
            self.boxed_bindings.pop();
        }
        self.scope_depth = self.scope_depth.saturating_sub(1);
    }

    /// Emit release calls for all heap owners at the specified scope depth
    fn emit_scope_cleanup(&mut self, target_scope: usize) {
        // Collect variables that need to be released (declared at target_scope, not transferred)
        let mut vars_to_release: Vec<String> = Vec::new();

        for (var_name, (heap_type, scope_depth)) in &self.heap_owners {
            if *scope_depth == target_scope && !self.transferred_vars.contains(var_name) {
                vars_to_release.push(var_name.clone());
            }
        }

        // Phase 3a: sort for byte-stable generated C (see emit_early_return_cleanup)
        vars_to_release.sort();

        // Emit release calls in reverse order (LIFO scope cleanup)
        for var_name in vars_to_release.iter().rev() {
            if let Some((heap_type, _)) = self.heap_owners.get(var_name) {
                let c_var_name = self.mangle_variable_name(var_name);
                match heap_type {
                    HeapType::Object => {
                        self.output.push_str(&format!("    hl_object_release({});\n", c_var_name));
                    }
                    HeapType::Function => {
                        self.output.push_str(&format!("    hl_function_release({});\n", c_var_name));
                    }
                    HeapType::FStringBuffer => {
                        self.output.push_str(&format!("    free({}); hl_free_count++;\n", var_name));
                    }
                    HeapType::Unknown => {
                        self.output.push_str(&format!("    hl_unknown_release({});\n", c_var_name));
                    }
                    HeapType::Optional => {
                        // Release optional wrapper struct - handles inner unknown release automatically
                        self.output.push_str(&format!("    hl_optional_release({});\n", c_var_name));
                    }
                    HeapType::Watcher => {
                        // Phase 2b: releases the watcher's owned env too
                        self.output.push_str(&format!("    hl_watcher_release({});\n", c_var_name));
                    }
                    HeapType::Array => {
                        self.output.push_str(&format!("    hl_array_release({});\n", c_var_name));
                    }
                    HeapType::Scalar => {
                        // Phase 3b: the scope's reference to a boxed scalar
                        // cell; an escaped watcher's env retain keeps it alive
                        self.output.push_str(&format!("    hl_scalar_release({});\n", c_var_name));
                    }
                    HeapType::Tuple(element_types) => {
                        // Release heap-allocated elements in the tuple
                        for (i, element_type) in element_types.iter().enumerate() {
                            match element_type {
                                Type::String | Type::DynamicArray(_) => {
                                    self.output.push_str(&format!("    hl_array_release({}._{});\n", c_var_name, i));
                                },
                                Type::Object(_) => {
                                    self.output.push_str(&format!("    hl_object_release({}._{});\n", c_var_name, i));
                                },
                                Type::Function(_, _) => {
                                    self.output.push_str(&format!("    hl_function_release({}._{});\n", c_var_name, i));
                                },
                                Type::Unknown => {
                                    self.output.push_str(&format!("    hl_unknown_release({}._{});\n", c_var_name, i));
                                },
                                Type::Optional(_) => {
                                    self.output.push_str(&format!("    hl_optional_release({}._{});\n", c_var_name, i));
                                },
                                Type::UnknownType => {
                                    self.output.push_str(&format!("    hl_unknown_release({}._{});\n", c_var_name, i));
                                },
                                _ => {
                                    // Primitive types don't need release
                                }
                            }
                        }
                    }
                }
            }
        }

        // Remove released variables from tracking
        for var_name in &vars_to_release {
            self.heap_owners.remove(var_name);
        }
    }

    /// Emit the release call for one tracked temporary (Phase 1.5b extraction:
    /// shared by statement-end cleanup and control-transfer frame walks)
    fn emit_temp_release(&mut self, temp_name: &str, heap_type: &HeapType) {
        match heap_type {
            HeapType::Object => {
                self.output.push_str(&format!("  hl_object_release({});\n", temp_name));
            }
            HeapType::Function => {
                self.output.push_str(&format!("  hl_function_release({});\n", temp_name));
            }
            HeapType::FStringBuffer => {
                self.output.push_str(&format!("  free({}); hl_free_count++;\n", temp_name));
            }
            HeapType::Unknown => {
                self.output.push_str(&format!("  hl_unknown_release({});\n", temp_name));
            }
            HeapType::Optional => {
                self.output.push_str(&format!("  hl_optional_release({});\n", temp_name));
            }
            HeapType::Watcher => {
                self.output.push_str(&format!("  hl_watcher_release({});\n", temp_name));
            }
            HeapType::Array => {
                self.output.push_str(&format!("  hl_array_release({});\n", temp_name));
            }
            HeapType::Scalar => {
                self.output.push_str(&format!("  hl_scalar_release({});\n", temp_name));
            }
            HeapType::Tuple(_) => {
                // Tuples are stack-allocated and shouldn't appear in temp_owners
                // This case should not occur, but added for exhaustive matching
            }
        }
    }

    /// Emit release calls for statement-end temporary cleanup (Phase 11a expression-temporary)
    fn emit_temp_cleanup(&mut self) {
        // Collect all temporary variables for release
        let mut temps_to_release: Vec<(String, HeapType)> = Vec::new();

        for (temp_name, heap_type) in &self.temp_owners {
            temps_to_release.push((temp_name.clone(), heap_type.clone()));
        }

        // Phase 3a: sort for byte-stable generated C (temp_owners is a HashMap)
        temps_to_release.sort_by(|a, b| a.0.cmp(&b.0));

        // Emit release calls in reverse order (LIFO cleanup)
        for (temp_name, heap_type) in temps_to_release.iter().rev() {
            self.emit_temp_release(temp_name, heap_type);
        }

        // Clear the temporary tracking after cleanup
        self.temp_owners.clear();
    }

    /// Emit release calls for enclosing statements' temps whose statement-end
    /// cleanup a control transfer jumps past (Phase 1.5b). Frames are read,
    /// not cleared: the not-taken path still reaches the normal cleanup.
    /// `from_frame` selects how far out to unwind: 0 for return (all frames),
    /// a LoopFrame's temp_frame_base for break/continue.
    fn emit_enclosing_temp_releases(&mut self, from_frame: usize) {
        let from_frame = from_frame.min(self.enclosing_temp_frames.len());
        // Phase 3a: frames are ordered (a Vec), but each frame is a saved
        // temp_owners HashMap — sort within each frame for byte-stable
        // generated C.
        let releases: Vec<(String, HeapType)> = self.enclosing_temp_frames[from_frame..]
            .iter()
            .rev()
            .flat_map(|frame| {
                let mut entries: Vec<(String, HeapType)> = frame
                    .iter()
                    .map(|(name, heap_type)| (name.clone(), heap_type.clone()))
                    .collect();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                entries
            })
            .collect();
        for (temp_name, heap_type) in &releases {
            self.emit_temp_release(temp_name, heap_type);
        }
    }

    /// Emit release calls for early returns without modifying heap_owners
    /// This preserves heap_owners for function-end cleanup
    fn emit_early_return_cleanup(&mut self, target_scope: usize) {
        // Collect variables that need to be released (declared at target_scope, not transferred)
        let mut vars_to_release: Vec<String> = Vec::new();

        for (var_name, (heap_type, scope_depth)) in &self.heap_owners {
            if *scope_depth == target_scope && !self.transferred_vars.contains(var_name) {
                vars_to_release.push(var_name.clone());
            }
        }

        // Phase 3a: heap_owners is a HashMap, so collection order is
        // nondeterministic per process — sort by name so generated C is
        // byte-stable across compiler runs. Release order within one cleanup
        // block is semantically order-independent (2a symmetric unlink).
        vars_to_release.sort();

        // Emit release calls in reverse order (LIFO scope cleanup)
        for var_name in vars_to_release.iter().rev() {
            if let Some((heap_type, _)) = self.heap_owners.get(var_name) {
                let c_var_name = self.mangle_variable_name(var_name);
                match heap_type {
                    HeapType::Object => {
                        self.output.push_str(&format!("    hl_object_release({});\n", c_var_name));
                    }
                    HeapType::Function => {
                        self.output.push_str(&format!("    hl_function_release({});\n", c_var_name));
                    }
                    HeapType::FStringBuffer => {
                        self.output.push_str(&format!("    free({}); hl_free_count++;\n", var_name));
                    }
                    HeapType::Unknown => {
                        self.output.push_str(&format!("    hl_unknown_release({});\n", c_var_name));
                    }
                    HeapType::Optional => {
                        // Release optional wrapper struct - handles inner unknown release automatically
                        self.output.push_str(&format!("    hl_optional_release({});\n", c_var_name));
                    }
                    HeapType::Watcher => {
                        // Phase 2b: releases the watcher's owned env too
                        self.output.push_str(&format!("    hl_watcher_release({});\n", c_var_name));
                    }
                    HeapType::Array => {
                        self.output.push_str(&format!("    hl_array_release({});\n", c_var_name));
                    }
                    HeapType::Scalar => {
                        // Phase 3b: boxed scalar cell (early-return path)
                        self.output.push_str(&format!("    hl_scalar_release({});\n", c_var_name));
                    }
                    HeapType::Tuple(element_types) => {
                        // Release heap-allocated elements in the tuple
                        for (i, element_type) in element_types.iter().enumerate() {
                            match element_type {
                                Type::String | Type::DynamicArray(_) => {
                                    self.output.push_str(&format!("    hl_array_release({}._{});\n", c_var_name, i));
                                },
                                Type::Object(_) => {
                                    self.output.push_str(&format!("    hl_object_release({}._{});\n", c_var_name, i));
                                },
                                Type::Function(_, _) => {
                                    self.output.push_str(&format!("    hl_function_release({}._{});\n", c_var_name, i));
                                },
                                Type::Unknown => {
                                    self.output.push_str(&format!("    hl_unknown_release({}._{});\n", c_var_name, i));
                                },
                                Type::Optional(_) => {
                                    self.output.push_str(&format!("    hl_optional_release({}._{});\n", c_var_name, i));
                                },
                                Type::UnknownType => {
                                    self.output.push_str(&format!("    hl_unknown_release({}._{});\n", c_var_name, i));
                                },
                                _ => {
                                    // Primitive types don't need release
                                }
                            }
                        }
                    }
                }
            }
        }

        // Do NOT modify heap_owners - function-end cleanup needs the same list
    }

    fn emit_leak_check_and_return(&mut self) {
        // Phase 9b: Helper for early returns in main - emit leak check and return
        self.output.push_str("    // Memory leak check (Phase 8a)\n");
        self.output.push_str("    if (hl_alloc_count != hl_free_count) {\n");
        self.output.push_str("        fprintf(stderr, \"MEMORY LEAK: allocated %d, freed %d (diff=%d)\\n\",\n");
        self.output.push_str("                hl_alloc_count, hl_free_count, hl_alloc_count - hl_free_count);\n");
        self.output.push_str("        return 1;\n");
        self.output.push_str("    }\n");
        self.output.push_str("    return return_value;\n");
    }

    /// Phase 11a-ε: Consolidated main() function emission helper
    fn emit_main_function(&mut self, program: &Program, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Phase 11b-fixup: Reset flag for each invocation
        self.main_explicitly_returned = false;

        // Emit main function header
        self.output.push_str("int main() {\n");
        self.output.push_str("  int return_value = 0;\n");

        if let Some(body) = &program.body {
            // Mark that we're in the main program
            self.in_main_program = true;
            self.scope_depth = 1; // Main program starts at scope 1

            // Phase 3c: no pre-activation — watchers are constructed active
            // at their declaration sites in statement order.

            // Generate program body statements
            self.generate_program_body_statements(body, type_checker)?;

            self.in_main_program = false;

            // Phase 9c fix: Final cleanup for any remaining Optional variables
            for var_name in &self.main_program_optionals.clone() {
                if self.heap_owners.contains_key(var_name) {
                    let c_var_name = self.mangle_variable_name(var_name);
                    self.output.push_str(&format!("    // Phase 9c fix: Final cleanup for {}\n", var_name));
                    self.output.push_str(&format!("    hl_optional_release({});\n", c_var_name));
                }
            }

            // Phase 8a: Emit cleanup for all heap-owned variables in main scope
            self.emit_scope_cleanup(1);
        }

        // Phase 8a: Emit memory leak check and return
        if !self.main_explicitly_returned {
            self.emit_leak_check_and_return();
        }
        self.output.push_str("}\n");

        Ok(())
    }


    /// Generate an expression without applying type refinements
    /// Used for contexts like 'is unknown' checks where we need the original variable
    fn generate_expression_without_refinements(&mut self, expression: &Expression, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        match expression {
            Expression::Ident { name, .. } => {
                // Generate the identifier without applying any refinements
                let c_var_name = self.mangle_variable_name(name);
                self.output.push_str(&c_var_name);
            }
            _ => {
                // For non-identifier expressions, use normal generation
                self.generate_expression(expression, type_checker, ExprContext::Temporary)?;
            }
        }
        Ok(())
    }

    /// Get the type of an expression without applying type refinements
    /// Used for contexts like 'is unknown' checks where we need the original type
    fn infer_expression_type_without_refinements(&self, expr: &Expression) -> Type {
        match expr {
            Expression::Ident { name, .. } => {
                // Look up the variable type from our tracking, ignoring refined_type
                self.variable_types.get(name).cloned().unwrap_or(Type::Unknown)
            }
            _ => {
                // For non-identifier expressions, use normal type inference
                self.infer_expression_type_for_codegen(expr)
            }
        }
    }

    /// Emit code to access a variable that has been narrowed through type refinement
    fn emit_refined_variable_access(&mut self, var_name: &str, refined_type: &Type) -> Result<(), CodegenError> {
        let c_var_name = self.mangle_variable_name(var_name);

        // Get the variable's declared type to determine how to unwrap
        if let Some(declared_type) = self.variable_types.get(var_name) {
            match (declared_type, refined_type) {
                // T? narrowed to T - emit unwrap helper
                (Type::Optional(inner_declared), refined) if **inner_declared == *refined => {
                    match refined {
                        Type::I32 => {
                            self.output.push_str(&format!("hl_optional_unwrap_i32({})", c_var_name));
                        }
                        Type::String => {
                            self.output.push_str(&format!("hl_optional_unwrap_string({})", c_var_name));
                        }
                        Type::Time => {
                            self.output.push_str(&format!("hl_optional_unwrap_time({})", c_var_name));
                        }
                        Type::Duration => {
                            self.output.push_str(&format!("hl_optional_unwrap_duration({})", c_var_name));
                        }
                        Type::Money | Type::MoneyOf(_) => {
                            self.output.push_str(&format!("hl_optional_unwrap_money({})", c_var_name));
                        }
                        other => {
                            // Phase 2b step zero: no raw-variable fallback —
                            // that emitted a HiLowOptional* where the narrowed
                            // C type was expected (latent type mismatch).
                            // Unsupported inners are rejected at typecheck;
                            // this is defense-in-depth.
                            return Err(CodegenError::UnsupportedFeature {
                                feature: format!("narrowed access to a {}? value", other),
                                phase: "Phase 3 (scalar boxing builds the optional payload matrix)".to_string(),
                            });
                        }
                    }
                }
                // T? narrowed to unknown - emit unknown access
                (Type::Optional(_), Type::Unknown) => {
                    // The variable is known to be unknown, so unwrap the unknown value
                    self.output.push_str(&format!("hl_optional_unwrap_unknown({})", c_var_name));
                }
                _ => {
                    // No unwrapping needed - use the variable directly
                    self.output.push_str(&c_var_name);
                }
            }
        } else {
            // Fallback: use the variable name directly
            self.output.push_str(&c_var_name);
        }
        Ok(())
    }

    /// Phase 10-γ: Check if a type is allowed for watching (numeric/bool/array types)
    fn is_type_watchable_in_phase_10g(&self, ty: &Type) -> bool {
        matches!(ty,
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 |
            Type::F32 | Type::F64 | Type::Bool | Type::DynamicArray(_)
        )
    }

    /// Phase 3b: watchable types are the boxed-payload kinds the corpus
    /// needs (i32 only — audit §5 item 7, no speculative matrix) plus the
    /// container cells. Further scalar kinds land as programs need them.
    fn is_ast_type_watchable_in_phase_10g(&self, ty: &crate::ast::Type) -> bool {
        use crate::ast::{Type as AstType, PrimitiveType};
        matches!(ty,
            AstType::Primitive(PrimitiveType::I32) | AstType::DynamicArray(_)
        )
    }

    /// Phase 3b: generate a decl-form watcher — env-ABI body dispatched
    /// through the subscribed cells, gated on the legacy statics (they die
    /// in 3c). Emits statics, helpers, and the body into watcher_bodies;
    /// returns (body fn name, env fields, env dtor name) for the caller to
    /// emit the construction at the declaration site.
    fn generate_watcher(
        &mut self,
        watcher: &Watcher,
        watcher_id: usize,
        type_checker: &TypeChecker,
    ) -> Result<(String, Vec<(String, EnvSlot)>, Option<String>), CodegenError> {
        // Validate subscribed types. Phase 3e-β: containers watch fully —
        // (changed)/(assigned) are slot subscriptions (rebinding-watch), the
        // content modifiers subscribe the current value and FOLLOW rebinding
        // via retargeting (audit §5 item 10b).
        for subscription in &watcher.subscriptions {
            let var_name = &subscription.variable_name;
            if let Some(var_type) = subscription.resolved_var_type.borrow().as_ref() {
                match var_type {
                    crate::ast::Type::Primitive(crate::ast::PrimitiveType::I32) => {}
                    // Phase 3e-α: strings watch via the variable-slot cell —
                    // rebinding-watch, both modifiers.
                    crate::ast::Type::Primitive(crate::ast::PrimitiveType::String) => {}
                    crate::ast::Type::DynamicArray(_) | crate::ast::Type::Object(_) => {}
                    other => {
                        return Err(CodegenError::UnsupportedFeature {
                            feature: format!("watching a scalar of type {:?}", other),
                            phase: "a future phase — boxed scalar payload kinds land as programs need them (i32 today; audit §5 item 7)".to_string(),
                        });
                    }
                }
            } else {
                return Err(CodegenError::UnsupportedFeature {
                    feature: format!("subscription to '{}' with no resolved type", var_name),
                    phase: "internal error - type checker should have populated this".to_string(),
                });
            }
        }

        // Validate modifier/type pairing. Content modifiers are legal on
        // container-typed subscriptions only (typecheck already enforces
        // this; defense-in-depth for the scalar kinds).
        for subscription in &watcher.subscriptions {
            match subscription.modifier {
                SubscriptionModifier::Changed | SubscriptionModifier::Assigned => {
                    // Slot subscriptions — every watchable type.
                }
                SubscriptionModifier::Added |
                SubscriptionModifier::Removed | SubscriptionModifier::Moved |
                SubscriptionModifier::Deep => {
                    let is_container = matches!(
                        subscription.resolved_var_type.borrow().as_ref(),
                        Some(crate::ast::Type::DynamicArray(_)) | Some(crate::ast::Type::Object(_))
                    );
                    if !is_container {
                        return Err(CodegenError::UnsupportedFeature {
                            feature: format!("watcher modifier {:?} on a non-container variable", subscription.modifier),
                            phase: "content modifiers subscribe container values".to_string(),
                        });
                    }
                }
            }
        }

        let func_name = format!("hilow_watcher_{}_{}", watcher_id, watcher.name);
        let env_struct_name = format!("hilow_array_watcher_env_{}", watcher_id);

        // Env fields: the subscribed cells (deduplicated), then the
        // shadow-correct capture list from the 3a analysis.
        let mut env_fields: Vec<(String, EnvSlot)> = Vec::new();
        for subscription in &watcher.subscriptions {
            let var_name = &subscription.variable_name;
            if !env_fields.iter().any(|(n, _)| n == var_name) {
                env_fields.push((var_name.clone(), EnvSlot::Scalar));
            }
        }
        let analysis_captures: Vec<String> = self
            .boxing
            .as_ref()
            .map(|b| b.captures_for(&watcher.position).to_vec())
            .unwrap_or_default();
        for var_name in &analysis_captures {
            if env_fields.iter().any(|(n, _)| n == var_name) {
                continue;
            }
            let var_type = self.variable_types.get(var_name).cloned();
            // Phase 3e-α: slot-boxed variables are HiLowScalar* whatever
            // their HiLow type.
            let slot = if self.current_binding_boxed(var_name) {
                EnvSlot::Scalar
            } else {
                match var_type {
                    Some(Type::DynamicArray(_)) | Some(Type::String) => EnvSlot::Array,
                    Some(Type::Object(_)) => EnvSlot::Object,
                    Some(Type::I32) => EnvSlot::Scalar,
                    other => {
                        return Err(CodegenError::UnsupportedFeature {
                            feature: format!("watcher capture of '{}' with type {:?}", var_name, other),
                            phase: "a future phase — boxed scalar payload kinds land as programs need them (i32 today)".to_string(),
                        });
                    }
                }
            };
            env_fields.push((var_name.clone(), slot));
        }
        let env_dtor_name = Some(self.emit_watcher_env_struct(&env_struct_name, &env_fields));


        // Body: one firing ABI (env, cell, delta). Active/ended gating lives
        // on the runtime watcher object — hl_cell_notify checks it per node
        // (Phase 3c; the per-watcher statics are gone).
        self.watcher_bodies.push_str(&format!(
            "void {}(void* env, HiLowCell* hilow_cell, const HiLowDelta* delta) {{\n",
            func_name
        ));

        // Generate the watcher body
        let saved_output = self.output.clone();
        self.output.clear();

        let old_variable_types = self.variable_types.clone();
        let old_hoisted_variables = self.hoisted_variables.clone();
        let old_current_env_var = self.current_env_var.clone();
        let old_boxed_hoisted = self.boxed_hoisted.clone();
        let boxed_mask_base = self.boxed_bindings.len();

        self.output.push_str(&format!("  {}* env_cast = ({}*)env;\n", env_struct_name, env_struct_name));
        self.output.push_str("  (void)hilow_cell; (void)delta;\n");

        // Subscribed names bind snapshot locals read at fire time (Phase
        // 3e-α: snapshot type follows the variable's type; refs BORROW).
        // Uniform across both firing cells (Phase 3e-β): a content fire
        // binds the fired container == the current payload, and a slot fire
        // binds the new payload because retargeting completes before the
        // slot fires. Deduplicated — one snapshot per variable however many
        // subscriptions name it. Aliases on (added)/(removed)/(moved) bind
        // the delta payload (the expression-form prologue's shape).
        let mut bound_snapshots: Vec<String> = Vec::new();
        for subscription in &watcher.subscriptions {
            let var_name = &subscription.variable_name;
            let sub_ty = subscription
                .resolved_var_type
                .borrow()
                .as_ref()
                .map(Type::from_ast_type)
                .unwrap_or(Type::I32);
            if !bound_snapshots.contains(var_name) {
                bound_snapshots.push(var_name.clone());
                let (c_ty, getter) = match &sub_ty {
                    Type::String => ("HiLowArray*", "hl_scalar_get_str"),
                    Type::DynamicArray(_) => ("HiLowArray*", "hl_scalar_get_array_ref"),
                    Type::Object(_) => ("HiLowObject*", "hl_scalar_get_object_ref"),
                    _ => ("int32_t", "hl_scalar_get_i32"),
                };
                self.output.push_str(&format!(
                    "  {} {} = {}(env_cast->{});\n",
                    c_ty, var_name, getter, var_name
                ));
                self.variable_types.insert(var_name.clone(), sub_ty);
                self.push_boxed_binding(var_name, false);
            }
            if let Some(ref alias_name) = subscription.alias {
                match subscription.modifier {
                    SubscriptionModifier::Added | SubscriptionModifier::Removed => {
                        if let Some(alias_type) = subscription.resolved_alias_type.borrow().as_ref() {
                            let c_elem_type = self.ast_type_to_c(alias_type);
                            self.output.push_str(&format!(
                                "  {} {} = *({} *)delta->payload;\n",
                                c_elem_type, alias_name, c_elem_type
                            ));
                            self.variable_types
                                .insert(alias_name.clone(), Type::from_ast_type(alias_type));
                            self.push_boxed_binding(alias_name, false);
                        }
                    }
                    SubscriptionModifier::Moved => {
                        self.output.push_str(&format!(
                            "  HiLowMovedDelta {} = {{ ._0 = delta->from, ._1 = delta->to }};\n",
                            alias_name
                        ));
                        self.variable_types.insert(
                            alias_name.clone(),
                            Type::Tuple(vec![Type::Usize, Type::Usize]),
                        );
                        self.push_boxed_binding(alias_name, false);
                    }
                    _ => {} // typecheck rejects aliases on other modifiers
                }
            }
        }

        // Captures go through the env as cells.
        self.hoisted_variables.clear();
        self.current_env_var = Some("env_cast".to_string());
        for (var_name, slot) in &env_fields {
            if watcher.subscriptions.iter().any(|s| &s.variable_name == var_name) {
                continue;
            }
            self.hoisted_variables.insert(var_name.clone(), ("env_cast".to_string(), env_struct_name.clone()));
            if matches!(slot, EnvSlot::Scalar) {
                self.boxed_hoisted.insert(var_name.clone());
                // Phase 3e-α: keep the real type (getter keyed on it).
                if !self.variable_types.contains_key(var_name) {
                    self.variable_types.insert(var_name.clone(), Type::I32);
                }
            }
        }

        self.generate_block(&watcher.body, type_checker)?;

        // Restore enclosing-scope state
        self.variable_types = old_variable_types;
        self.hoisted_variables = old_hoisted_variables;
        self.current_env_var = old_current_env_var;
        self.boxed_hoisted = old_boxed_hoisted;
        self.boxed_bindings.truncate(boxed_mask_base);

        self.watcher_bodies.push_str(&self.output);
        self.watcher_bodies.push_str("}\n\n");

        self.output = saved_output;

        Ok((func_name, env_fields, env_dtor_name))
    }

    /// Phase 3b/3c: emit a decl-form watcher's construction at its
    /// declaration site, bound to the user's own name so the four methods
    /// dispatch through the runtime object (hl_watcher_pause etc.).
    /// Pre-declaration assignments cannot fire — the cell has no subscriber
    /// until this runs. The watcher is born active (hl_watcher_new).
    fn emit_decl_watcher_construction(
        &mut self,
        watcher: &Watcher,
        watcher_id: usize,
        env_fields: &[(String, EnvSlot)],
        env_dtor_name: Option<&str>,
    ) {
        let env_struct_name = format!("hilow_array_watcher_env_{}", watcher_id);
        let env_var = format!("__watcher_env_{}", watcher_id);
        let func_name = format!("hilow_watcher_{}_{}", watcher_id, watcher.name);

        let env_arg = if env_fields.is_empty() {
            "NULL".to_string()
        } else {
            let decl = self.watcher_env_pack_decl(&env_struct_name, &env_var, env_fields);
            self.output.push_str(&format!("  {}\n", decl));
            env_var
        };

        // Phase 3e-β: (changed)/(assigned) subscribe the SLOT (rebinding-
        // watch, audit §5 item 10a); content modifiers subscribe the CURRENT
        // value's cell (payload deref) and follow rebinding — each followed
        // variable gets ONE HL_SLOT_FOLLOW marker node on its slot so
        // hl_slot_retarget can find the watcher's nodes on rebinding.
        // Phase 3e-γ: each content subscription is ATTRIBUTED to its slot's
        // cell (the origin) so retargeting moves only the rebinding slot's
        // nodes — two followed variables holding the same container stay
        // independent. Watchers with followed variables emit the origins
        // (triples) constructor; everything else keeps the pairs form
        // byte-identically.
        let mut subs: Vec<(String, &str, Option<String>)> = Vec::new();
        let mut followed_vars: Vec<String> = Vec::new();
        for subscription in &watcher.subscriptions {
            let slot_var = self.env_slot_rvalue(&subscription.variable_name);
            let (c_modifier, is_content) = match subscription.modifier {
                SubscriptionModifier::Assigned => ("HL_SCALAR_ASSIGNED", false),
                SubscriptionModifier::Changed => ("HL_ARR_CHANGED", false),
                SubscriptionModifier::Added => ("HL_ARR_ADDED", true),
                SubscriptionModifier::Removed => ("HL_ARR_REMOVED", true),
                SubscriptionModifier::Moved => ("HL_ARR_MOVED", true),
                SubscriptionModifier::Deep => ("HL_ARR_DEEP", true),
            };
            if is_content {
                let getter = match subscription.resolved_var_type.borrow().as_ref() {
                    Some(crate::ast::Type::Object(_)) => "hl_scalar_get_object_ref",
                    _ => "hl_scalar_get_array_ref",
                };
                if !followed_vars.contains(&subscription.variable_name) {
                    followed_vars.push(subscription.variable_name.clone());
                }
                if matches!(subscription.modifier, SubscriptionModifier::Deep) {
                    // (deep) marks the current subtree at construction (the
                    // expression-form precedent); retargeting re-marks the
                    // new subtree on rebinding.
                    let mark_fn = match subscription.resolved_var_type.borrow().as_ref() {
                        Some(crate::ast::Type::Object(_)) => "hl_object_mark_deep",
                        _ => "hl_array_mark_deep",
                    };
                    self.output
                        .push_str(&format!("  {}({}({}));\n", mark_fn, getter, slot_var));
                }
                subs.push((
                    format!("&{}({})->cell", getter, slot_var),
                    c_modifier,
                    Some(format!("&{}->cell", slot_var)),
                ));
            } else {
                subs.push((format!("&{}->cell", slot_var), c_modifier, None));
            }
        }
        for var_name in &followed_vars {
            let slot_var = self.env_slot_rvalue(var_name);
            subs.push((format!("&{}->cell", slot_var), "HL_SLOT_FOLLOW", None));
        }
        let ctor = if followed_vars.is_empty() {
            "hl_watcher_new_subscribed"
        } else {
            "hl_watcher_new_subscribed_origins"
        };
        let mut call = format!(
            "{}((void*){}, {}, {}, {}",
            ctor,
            func_name,
            env_arg,
            env_dtor_name.unwrap_or("NULL"),
            subs.len()
        );
        for (cell_expr, c_modifier, origin) in &subs {
            call.push_str(&format!(", {}, {}", cell_expr, c_modifier));
            if !followed_vars.is_empty() {
                call.push_str(&format!(
                    ", {}",
                    origin.as_deref().unwrap_or("NULL")
                ));
            }
        }
        call.push(')');

        self.output.push_str(&format!("  HiLowWatcher* {} = {};\n", watcher.name, call));
        self.variable_types.insert(watcher.name.clone(), Type::Watcher);
        self.track_heap_owner(&watcher.name, HeapType::Watcher);
    }

}
