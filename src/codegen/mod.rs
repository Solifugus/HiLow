use crate::ast::*;
use crate::types::Type;
use crate::typecheck::TypeChecker;
use std::collections::HashMap;

/// Errors that can occur during code generation
#[derive(Debug)]
pub enum CodegenError {
    UnsupportedFeature {
        feature: String,
        phase: String,
    },
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodegenError::UnsupportedFeature { feature, phase } => {
                write!(f, "Unsupported feature '{}' - will be implemented in {}", feature, phase)
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
    /// Function symbols - maps function name to its signature
    functions: HashMap<String, String>,
    /// Variable types - maps variable name to its type
    variable_types: HashMap<String, Type>,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            var_counter: 0,
            functions: HashMap::new(),
            variable_types: HashMap::new(),
        }
    }

    /// Generate C code for the entire program
    pub fn generate(&mut self, top_level: &TopLevel, type_checker: &TypeChecker) -> Result<String, CodegenError> {
        // Add standard C includes
        self.emit_includes();

        match top_level {
            TopLevel::Program(program) => {
                self.generate_program(program, type_checker)?;
            }
            TopLevel::Module(module) => {
                self.generate_module(module, type_checker)?;
            }
        }

        Ok(self.output.clone())
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

        // Generate the main function
        self.output.push_str("int main() {\n");

        if let Some(body) = &program.body {
            self.generate_program_body_statements(body, type_checker)?;
        }

        self.output.push_str("}\n");
        Ok(())
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

        // Generate parameters
        for (i, param) in function.params.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            let c_type = self.hilow_type_to_c(&Type::from_ast_type(&param.ty));
            self.output.push_str(&format!("{} {}", c_type, param.name));
        }

        self.output.push_str(") {\n");

        // Generate function body
        if let Some(body) = &function.body {
            self.generate_block(body, type_checker)?;
        }

        self.output.push_str("}\n\n");
        Ok(())
    }

    fn generate_block(&mut self, block: &Block, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        for statement in &block.statements {
            self.generate_statement(statement, type_checker)?;
        }
        Ok(())
    }

    fn generate_program_body_functions(&mut self, body: &ProgramBody, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // First, track function types for later reference
        for item in &body.items {
            if let BlockItem::Function(function) = item {
                let return_type = Type::from_ast_type(&function.return_type);
                self.variable_types.insert(function.name.clone(), return_type);
            }
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
        // Generate only the statements, not the nested functions
        for item in &body.items {
            if let BlockItem::Statement(statement) = item {
                self.generate_statement(statement, type_checker)?;
            }
        }
        Ok(())
    }

    fn generate_statement(&mut self, statement: &Statement, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        match statement {
            Statement::Let(let_decl) => {
                self.generate_let_statement(let_decl, type_checker)?;
            }
            Statement::Return(return_stmt) => {
                self.generate_return_statement(return_stmt, type_checker)?;
            }
            Statement::ExprStatement(expr) => {
                // Generate the expression and add semicolon
                self.generate_expression(expr, type_checker)?;
                self.output.push_str(";\n");
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
            Statement::Break(_) => {
                self.output.push_str("  break;\n");
            }
            Statement::Continue(_) => {
                self.output.push_str("  continue;\n");
            }
            Statement::Assign(assign_stmt) => {
                self.generate_assign_statement(assign_stmt, type_checker)?;
            }
            Statement::QualifiedOp(qualified_op) => {
                self.generate_qualified_op_statement(qualified_op, type_checker)?;
            }
        }
        Ok(())
    }

    fn generate_let_statement(&mut self, let_decl: &LetDecl, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Determine the type
        let var_type = if let Some(ref ty) = let_decl.ty {
            Type::from_ast_type(ty)
        } else if let Some(ref initializer) = let_decl.initializer {
            // Type inference - need to handle object literals properly
            match initializer {
                Expression::IntLit(value, _) => Type::default_integer_type(*value),
                Expression::FloatLit(_, _) => Type::default_float_type(),
                Expression::StringLit(_, _) => Type::String,
                Expression::BoolLit(_, _) => Type::Bool,
                Expression::ObjectLiteral(obj_lit) => {
                    // Infer object type from literal
                    let mut properties = Vec::new();
                    for (prop_name, prop_expr) in &obj_lit.properties {
                        let prop_type = self.infer_expression_type_for_codegen(prop_expr);
                        properties.push((prop_name.clone(), prop_type));
                    }
                    Type::Object(properties)
                }
                _ => {
                    // For other complex expressions, try to infer
                    self.infer_expression_type_for_codegen(initializer)
                }
            }
        } else {
            return Err(CodegenError::UnsupportedFeature {
                feature: "uninitialized variables".to_string(),
                phase: "Phase 9 (nothing type)".to_string(),
            });
        };

        let c_type = self.hilow_type_to_c(&var_type);
        self.output.push_str(&format!("  {} {}", c_type, let_decl.name));

        if let Some(ref initializer) = let_decl.initializer {
            self.output.push_str(" = ");
            self.generate_expression(initializer, type_checker)?;
        }

        self.output.push_str(";\n");

        // Track the variable type for later reference
        self.variable_types.insert(let_decl.name.clone(), var_type);

        Ok(())
    }

    fn generate_return_statement(&mut self, return_stmt: &ReturnStmt, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        self.output.push_str("  return");
        if let Some(ref value) = return_stmt.value {
            self.output.push_str(" ");
            self.generate_expression(value, type_checker)?;
        }
        self.output.push_str(";\n");
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

    fn generate_while_statement(&mut self, while_stmt: &WhileStmt, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        self.output.push_str("  while (");
        self.generate_condition(&while_stmt.condition, type_checker)?;
        self.output.push_str(") {\n");

        self.generate_block(&while_stmt.body, type_checker)?;

        self.output.push_str("  }\n");
        Ok(())
    }

    fn generate_loop_statement(&mut self, loop_stmt: &LoopStmt, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        self.output.push_str("  while (1) {\n");

        self.generate_block(&loop_stmt.body, type_checker)?;

        self.output.push_str("  }\n");
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

            // Determine the type of the value to call the right setter
            let value_type = self.infer_expression_type_for_codegen(&assign_stmt.value);

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
                _ => {
                    return Err(CodegenError::UnsupportedFeature {
                        feature: format!("assignment of type {} to object property", value_type),
                        phase: "future phases".to_string(),
                    });
                }
            }

            // Generate: object, property name, value
            self.generate_expression(&member_access.object, type_checker)?;
            self.output.push_str(", \"");
            self.output.push_str(&member_access.member);
            self.output.push_str("\", ");
            self.generate_expression(&assign_stmt.value, type_checker)?;
            self.output.push_str(");\n");
        } else {
            // Regular assignment to variables
            self.generate_expression(&assign_stmt.target, type_checker)?;

            let op_str = match assign_stmt.op {
                AssignOpKind::Assign => " = ",
                AssignOpKind::AddAssign => " += ",
                AssignOpKind::SubAssign => " -= ",
                AssignOpKind::MulAssign => " *= ",
                AssignOpKind::DivAssign => " /= ",
                AssignOpKind::ModAssign => " %= ",
            };

            self.output.push_str(op_str);
            self.generate_expression(&assign_stmt.value, type_checker)?;
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
                self.generate_expression(condition, type_checker)?;
            }
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 |
            Type::Isize | Type::Usize | Type::F32 | Type::F64 => {
                // For numeric types, emit (expr != 0) for truthy/falsy check
                self.output.push_str("(");
                self.generate_expression(condition, type_checker)?;
                self.output.push_str(" != 0)");
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

    fn generate_expression(&mut self, expression: &Expression, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        match expression {
            Expression::IntLit(value, _) => {
                self.output.push_str(&value.to_string());
            }
            Expression::FloatLit(value, _) => {
                self.output.push_str(&value.to_string());
            }
            Expression::StringLit(value, _) => {
                // Emit C string literal with UTF-8 support
                self.output.push('"');
                for ch in value.chars() {
                    match ch {
                        '"' => self.output.push_str("\\\""),
                        '\\' => self.output.push_str("\\\\"),
                        '\n' => self.output.push_str("\\n"),
                        '\t' => self.output.push_str("\\t"),
                        '\r' => self.output.push_str("\\r"),
                        c if (c as u32) < 0x20 && c != '\n' && c != '\t' && c != '\r' => {
                            // Escape control characters below 0x20 (except \n, \t, \r already handled)
                            self.output.push_str(&format!("\\x{:02x}", c as u8));
                        }
                        c => {
                            // Emit UTF-8 bytes directly - C99/C11 supports arbitrary bytes in string literals
                            self.output.push(c);
                        }
                    }
                }
                self.output.push('"');
            }
            Expression::FString(fstring) => {
                self.generate_fstring(fstring, type_checker)?;
            }
            Expression::BoolLit(value, _) => {
                self.output.push_str(if *value { "true" } else { "false" });
            }
            Expression::Ident(name, _) => {
                self.output.push_str(name);
            }
            Expression::BinaryOp(binary_op) => {
                self.generate_binary_op(binary_op, type_checker)?;
            }
            Expression::UnaryOp(unary_op) => {
                self.generate_unary_op(unary_op, type_checker)?;
            }
            Expression::Call(call) => {
                self.generate_call(call, type_checker)?;
            }
            Expression::MemberAccess(member_access) => {
                self.generate_member_access(member_access, type_checker)?;
            }
            Expression::IndexAccess(_) => {
                return Err(CodegenError::UnsupportedFeature {
                    feature: "index access".to_string(),
                    phase: "Phase 6 (arrays)".to_string(),
                });
            }
            Expression::IsCheck(is_check) => {
                self.generate_is_check(is_check, type_checker)?;
            }
            Expression::QualifiedOp(qualified_op) => {
                self.generate_qualified_op_expression(qualified_op, type_checker)?;
            }
            Expression::ObjectLiteral(obj_lit) => {
                self.generate_object_literal(obj_lit, type_checker)?;
            }
        }
        Ok(())
    }

    fn generate_binary_op(&mut self, binary_op: &BinaryOp, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        self.output.push_str("(");
        self.generate_expression(&binary_op.lhs, type_checker)?;

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
        self.generate_expression(&binary_op.rhs, type_checker)?;
        self.output.push_str(")");
        Ok(())
    }

    fn generate_unary_op(&mut self, unary_op: &UnaryOp, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        let op_str = match unary_op.op {
            UnaryOpKind::Neg => "-",
            UnaryOpKind::Not => "!",
            UnaryOpKind::BitNot => "~",
        };

        self.output.push_str(op_str);
        self.generate_expression(&unary_op.operand, type_checker)?;
        Ok(())
    }

    fn generate_call(&mut self, call: &Call, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Check if this is the special print() function
        if let Expression::Ident(func_name, _) = call.callee.as_ref() {
            if func_name == "print" {
                return self.generate_print_call(call, type_checker);
            }
        }

        // Regular function call - handle nested function name mangling
        if let Expression::Ident(func_name, _) = call.callee.as_ref() {
            // Use mangled name for nested functions
            let c_func_name = self.mangle_function_name(func_name);
            self.output.push_str(&c_func_name);
        } else {
            self.generate_expression(&call.callee, type_checker)?;
        }
        self.output.push_str("(");

        for (i, arg) in call.args.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            self.generate_expression(arg, type_checker)?;
        }

        self.output.push_str(")");
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
            Type::String => "print_str",
            _ => {
                return Err(CodegenError::UnsupportedFeature {
                    feature: format!("print() for type {}", arg_type),
                    phase: "later phases".to_string(),
                });
            }
        };

        self.output.push_str(runtime_func);
        self.output.push_str("(");
        self.generate_expression(arg, type_checker)?;
        self.output.push_str(")");

        Ok(())
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
            Type::String => "const char*".to_string(),
            Type::Usize => "size_t".to_string(),
            Type::Isize => "ssize_t".to_string(),
            Type::Nothing => "void".to_string(),
            Type::FixedArray(_, _) => "void*".to_string(), // Placeholder for Phase 6
            Type::DynamicArray(_) => "void*".to_string(), // Placeholder for Phase 6
            Type::Object(_) => "HiLowObject*".to_string(),
            Type::Unknown => "void".to_string(),
        }
    }

    fn generate_is_check(&mut self, is_check: &IsCheck, _type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // For primitive types, is checks are done at compile time
        let expr_type = self.infer_expression_type(&is_check.expression);
        let target_type = Type::from_ast_type(&is_check.ty);

        // Compare types at compile time
        let types_match = expr_type == target_type;

        // Apply negation if needed
        let result = if is_check.negated { !types_match } else { types_match };

        // Emit 1 for true, 0 for false
        self.output.push_str(if result { "1" } else { "0" });

        Ok(())
    }

    fn next_var_name(&mut self) -> String {
        let name = format!("_v{}", self.var_counter);
        self.var_counter += 1;
        name
    }

    fn mangle_function_name(&self, name: &str) -> String {
        // Phase 6a-fixup: Simple mangling for nested functions to avoid C keyword conflicts
        format!("hilow_{}", name)
    }

    /// Simple type inference for expressions in Phase 4a
    /// This is a simplified version that doesn't use the full type checker context
    fn infer_expression_type(&self, expr: &Expression) -> Type {
        match expr {
            Expression::IntLit(_, _) => Type::I32, // Default integer type
            Expression::FloatLit(_, _) => Type::F64, // Default float type
            Expression::StringLit(_, _) => Type::String,
            Expression::FString(_) => Type::String,
            Expression::BoolLit(_, _) => Type::Bool,
            Expression::Ident(name, _) => {
                // Look up the variable type from our tracking
                self.variable_types.get(name).cloned().unwrap_or(Type::I32)
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
            Expression::QualifiedOp(qualified_op) => {
                match qualified_op.op {
                    QualifiedOpKind::Assign => self.infer_expression_type(&qualified_op.lhs),
                    QualifiedOpKind::Eq | QualifiedOpKind::NotEq => Type::Bool,
                }
            }
            _ => Type::I32, // Default fallback
        }
    }

    /// Enhanced type inference for codegen that handles object types
    fn infer_expression_type_for_codegen(&self, expr: &Expression) -> Type {
        match expr {
            Expression::IntLit(_, _) => Type::I32,
            Expression::FloatLit(_, _) => Type::F64,
            Expression::StringLit(_, _) => Type::String,
            Expression::FString(_) => Type::String,
            Expression::BoolLit(_, _) => Type::Bool,
            Expression::Ident(name, _) => {
                // Look up the variable type from our tracking
                self.variable_types.get(name).cloned().unwrap_or(Type::Unknown)
            }
            Expression::ObjectLiteral(obj_lit) => {
                let mut properties = Vec::new();
                for (prop_name, prop_expr) in &obj_lit.properties {
                    let prop_type = self.infer_expression_type_for_codegen(prop_expr);
                    properties.push((prop_name.clone(), prop_type));
                }
                Type::Object(properties)
            }
            Expression::MemberAccess(member_access) => {
                let object_type = self.infer_expression_type_for_codegen(&member_access.object);
                match object_type {
                    Type::Object(properties) => {
                        for (prop_name, prop_type) in properties {
                            if prop_name == member_access.member {
                                return prop_type;
                            }
                        }
                        Type::Unknown // Property not found
                    }
                    _ => Type::Unknown
                }
            }
            Expression::BinaryOp(op) => {
                match op.op {
                    BinaryOpKind::Add | BinaryOpKind::Sub | BinaryOpKind::Mul |
                    BinaryOpKind::Div | BinaryOpKind::Mod => {
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
            Expression::QualifiedOp(qualified_op) => {
                match qualified_op.op {
                    QualifiedOpKind::Assign => self.infer_expression_type_for_codegen(&qualified_op.lhs),
                    QualifiedOpKind::Eq | QualifiedOpKind::NotEq => Type::Bool,
                }
            }
            Expression::Call(call) => {
                // For function calls, try to look up the return type
                if let Expression::Ident(func_name, _) = call.callee.as_ref() {
                    if func_name == "print" {
                        return Type::I32; // print() returns i32
                    }
                    // Look up the function's return type in our variable tracking
                    self.variable_types.get(func_name).cloned().unwrap_or(Type::I32)
                } else {
                    Type::I32 // Default for complex call expressions
                }
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
        self.generate_expression(&qualified_op.lhs, type_checker)?;
        self.output.push_str(" = ");
        self.generate_expression(&qualified_op.lhs, type_checker)?;

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
        self.generate_expression(&qualified_op.rhs, type_checker)?;

        Ok(())
    }

    fn generate_fstring(&mut self, fstring: &FString, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Use malloc'd buffer approach as specified in Phase 6b-i requirements
        // Memory leak is acceptable for Phase 6b-i, will be fixed in Phase 8

        // Calculate buffer size estimate (4KB default with some buffer)
        let buffer_size = 4096;

        // Generate: malloc'd buffer with snprintf chain
        self.output.push_str("({ char* __fstring_buf = malloc(");
        self.output.push_str(&buffer_size.to_string());
        self.output.push_str("); __fstring_buf[0] = '\\0'; ");

        // Track position for potential future use

        for part in &fstring.parts {
            match part {
                FStringPart::Text(text) => {
                    if !text.is_empty() {
                        self.output.push_str("strcat(__fstring_buf, ");
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
                        self.output.push_str("\"); ");
                    }
                }
                FStringPart::Expression(expr, format_spec) => {
                    let expr_type = self.infer_expression_type_for_codegen(expr);

                    if let Some(format_spec) = format_spec {
                        // Handle special binary format case
                        if format_spec.type_code == Some('b') {
                            self.output.push_str("{ char* __tmp_buf = hl_format_binary((unsigned long long)");
                            self.generate_expression(expr, type_checker)?;
                            self.output.push_str("); ");

                            // Handle alignment for binary format
                            if format_spec.align == Some(Align::Center) && format_spec.width.is_some() {
                                self.output.push_str(&format!("char* __centered_buf = hl_format_center(__tmp_buf, {}); ", format_spec.width.unwrap()));
                                self.output.push_str("strcat(__fstring_buf, __centered_buf); free(__tmp_buf); free(__centered_buf); } ");
                            } else {
                                // For binary format, we'll implement basic left/right alignment here
                                if let Some(width) = format_spec.width {
                                    if format_spec.align == Some(Align::Left) {
                                        self.output.push_str(&format!("sprintf(__tmp_buf + strlen(__tmp_buf), \"%*s\", {}, \"\"); ", width.saturating_sub(1)));
                                    } else if format_spec.align == Some(Align::Right) || format_spec.align.is_none() {
                                        // Right align or default - pad on left
                                        self.output.push_str("{ char __padded_buf[128]; ");
                                        self.output.push_str(&format!("sprintf(__padded_buf, \"%*s\", {}, __tmp_buf); ", width));
                                        self.output.push_str("strcat(__fstring_buf, __padded_buf); } ");
                                    }
                                } else {
                                    self.output.push_str("strcat(__fstring_buf, __tmp_buf); ");
                                }
                                self.output.push_str("free(__tmp_buf); } ");
                            }
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
                                    self.output.push_str("strcat(__fstring_buf, __centered_buf); free(__centered_buf); } ");
                                } else {
                                    self.output.push_str("strcat(__fstring_buf, __tmp_buf); } ");
                                }
                            } else {
                                // Standard sprintf with possible alignment
                                self.output.push_str("{ char __tmp_buf[64]; sprintf(__tmp_buf, \"");
                                self.output.push_str(&c_format);
                                self.output.push_str("\", ");
                                self.generate_format_expression_with_cast(&expr_type, expr, type_checker)?;
                                self.output.push_str("); strcat(__fstring_buf, __tmp_buf); } ");
                            }
                        }
                    } else {
                        // No format specifier - use default formatting
                        match expr_type {
                            Type::String => {
                                // String: concatenate directly
                                self.output.push_str("strcat(__fstring_buf, ");
                                self.generate_expression(expr, type_checker)?;
                                self.output.push_str("); ");
                            }
                            Type::I8 | Type::I16 | Type::I32 | Type::Isize => {
                                // 32-bit integers
                                self.output.push_str("{ char __tmp_buf[32]; sprintf(__tmp_buf, \"%d\", (int)");
                                self.generate_expression(expr, type_checker)?;
                                self.output.push_str("); strcat(__fstring_buf, __tmp_buf); } ");
                            }
                            Type::I64 => {
                                // 64-bit integers
                                self.output.push_str("{ char __tmp_buf[32]; sprintf(__tmp_buf, \"%lld\", (long long)");
                                self.generate_expression(expr, type_checker)?;
                                self.output.push_str("); strcat(__fstring_buf, __tmp_buf); } ");
                            }
                            Type::U8 | Type::U16 | Type::U32 | Type::Usize => {
                                // 32-bit unsigned integers
                                self.output.push_str("{ char __tmp_buf[32]; sprintf(__tmp_buf, \"%u\", (unsigned int)");
                                self.generate_expression(expr, type_checker)?;
                                self.output.push_str("); strcat(__fstring_buf, __tmp_buf); } ");
                            }
                            Type::U64 => {
                                // 64-bit unsigned integers
                                self.output.push_str("{ char __tmp_buf[32]; sprintf(__tmp_buf, \"%llu\", (unsigned long long)");
                                self.generate_expression(expr, type_checker)?;
                                self.output.push_str("); strcat(__fstring_buf, __tmp_buf); } ");
                            }
                            Type::F32 => {
                                // 32-bit floats
                                self.output.push_str("{ char __tmp_buf[64]; sprintf(__tmp_buf, \"%g\", (double)");
                                self.generate_expression(expr, type_checker)?;
                                self.output.push_str("); strcat(__fstring_buf, __tmp_buf); } ");
                            }
                            Type::F64 => {
                                // 64-bit floats
                                self.output.push_str("{ char __tmp_buf[64]; sprintf(__tmp_buf, \"%g\", ");
                                self.generate_expression(expr, type_checker)?;
                                self.output.push_str("); strcat(__fstring_buf, __tmp_buf); } ");
                            }
                            Type::Bool => {
                                // Boolean: "true" or "false"
                                self.output.push_str("strcat(__fstring_buf, (");
                                self.generate_expression(expr, type_checker)?;
                                self.output.push_str(") ? \"true\" : \"false\"); ");
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

        self.output.push_str("__fstring_buf; })");

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
                self.generate_expression(expr, type_checker)?;
            }
            Type::I64 => {
                self.output.push_str("(long long)");
                self.generate_expression(expr, type_checker)?;
            }
            Type::U8 | Type::U16 | Type::U32 | Type::Usize => {
                self.output.push_str("(unsigned int)");
                self.generate_expression(expr, type_checker)?;
            }
            Type::U64 => {
                self.output.push_str("(unsigned long long)");
                self.generate_expression(expr, type_checker)?;
            }
            Type::F32 => {
                self.output.push_str("(double)");
                self.generate_expression(expr, type_checker)?;
            }
            Type::F64 => {
                self.generate_expression(expr, type_checker)?;
            }
            Type::String => {
                self.generate_expression(expr, type_checker)?;
            }
            Type::Bool => {
                self.output.push_str("(");
                self.generate_expression(expr, type_checker)?;
                self.output.push_str(") ? \"true\" : \"false\"");
            }
            _ => {
                self.generate_expression(expr, type_checker)?;
            }
        }
        Ok(())
    }

    fn generate_object_literal(&mut self, obj_lit: &ObjectLiteral, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Generate object creation: hl_object_new()
        self.output.push_str("({\n");
        self.output.push_str("    HiLowObject* obj = hl_object_new();\n");

        // Generate property assignments
        for (prop_name, prop_expr) in &obj_lit.properties {
            self.output.push_str(&format!("    hl_object_set_"));

            // Determine the type of the property to call the right setter
            let expr_type = self.infer_expression_type_for_codegen(prop_expr);
            match expr_type {
                Type::I32 => {
                    self.output.push_str("i32(obj, \"");
                    self.output.push_str(prop_name);
                    self.output.push_str("\", ");
                    self.generate_expression(prop_expr, type_checker)?;
                    self.output.push_str(");\n");
                }
                Type::I64 => {
                    self.output.push_str("i64(obj, \"");
                    self.output.push_str(prop_name);
                    self.output.push_str("\", ");
                    self.generate_expression(prop_expr, type_checker)?;
                    self.output.push_str(");\n");
                }
                Type::U32 => {
                    self.output.push_str("u32(obj, \"");
                    self.output.push_str(prop_name);
                    self.output.push_str("\", ");
                    self.generate_expression(prop_expr, type_checker)?;
                    self.output.push_str(");\n");
                }
                Type::U64 => {
                    self.output.push_str("u64(obj, \"");
                    self.output.push_str(prop_name);
                    self.output.push_str("\", ");
                    self.generate_expression(prop_expr, type_checker)?;
                    self.output.push_str(");\n");
                }
                Type::F32 => {
                    self.output.push_str("f32(obj, \"");
                    self.output.push_str(prop_name);
                    self.output.push_str("\", ");
                    self.generate_expression(prop_expr, type_checker)?;
                    self.output.push_str(");\n");
                }
                Type::F64 => {
                    self.output.push_str("f64(obj, \"");
                    self.output.push_str(prop_name);
                    self.output.push_str("\", ");
                    self.generate_expression(prop_expr, type_checker)?;
                    self.output.push_str(");\n");
                }
                Type::Bool => {
                    self.output.push_str("bool(obj, \"");
                    self.output.push_str(prop_name);
                    self.output.push_str("\", ");
                    self.generate_expression(prop_expr, type_checker)?;
                    self.output.push_str(");\n");
                }
                Type::String => {
                    self.output.push_str("str(obj, \"");
                    self.output.push_str(prop_name);
                    self.output.push_str("\", ");
                    self.generate_expression(prop_expr, type_checker)?;
                    self.output.push_str(");\n");
                }
                Type::Object(_) => {
                    self.output.push_str("object(obj, \"");
                    self.output.push_str(prop_name);
                    self.output.push_str("\", ");
                    self.generate_expression(prop_expr, type_checker)?;
                    self.output.push_str(");\n");
                }
                _ => {
                    return Err(CodegenError::UnsupportedFeature {
                        feature: format!("object property of type {}", expr_type),
                        phase: "future phases".to_string(),
                    });
                }
            }
        }

        self.output.push_str("    obj;\n");
        self.output.push_str("})");

        Ok(())
    }

    fn generate_member_access(&mut self, member_access: &MemberAccess, type_checker: &TypeChecker) -> Result<(), CodegenError> {
        // Generate property access: hl_object_get_TYPE(obj, "property")

        // Determine the type of the property by looking up the object type and property
        let object_type = self.infer_expression_type_for_codegen(&member_access.object);
        let member_type = match object_type {
            Type::Object(properties) => {
                // Look up the property in the object's type
                let mut found_type = Type::Unknown;
                for (prop_name, prop_type) in properties {
                    if prop_name == member_access.member {
                        found_type = prop_type;
                        break;
                    }
                }
                found_type
            }
            _ => Type::Unknown
        };

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
            _ => {
                return Err(CodegenError::UnsupportedFeature {
                    feature: format!("member access for type {}", member_type),
                    phase: "future phases".to_string(),
                });
            }
        }

        // Generate the object expression
        self.generate_expression(&member_access.object, type_checker)?;

        // Generate the property name as a string literal
        self.output.push_str(", \"");
        self.output.push_str(&member_access.member);
        self.output.push_str("\")");

        Ok(())
    }
}