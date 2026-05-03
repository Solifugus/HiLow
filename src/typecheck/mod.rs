use crate::ast::*;
use crate::types::{Type, TypeError};
use crate::lexer::Position;
use crate::qualifiers::{QualifierRegistry, QualifierContext, CodegenStatus};
use std::collections::HashMap;

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

/// The type checker for HiLow programs
pub struct TypeChecker {
    scopes: Vec<Scope>,
    errors: Vec<TypeError>,
    loop_depth: usize, // Track nested loop depth for break/continue validation
    qualifier_registry: QualifierRegistry,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::new()], // Start with global scope
            errors: Vec::new(),
            loop_depth: 0,
            qualifier_registry: QualifierRegistry::new(),
        }
    }

    /// Type check a top-level program or module
    pub fn check(&mut self, top_level: &TopLevel) -> Result<(), Vec<TypeError>> {
        match top_level {
            TopLevel::Program(program) => self.check_program(program),
            TopLevel::Module(module) => self.check_module(module),
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
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
            self.check_block(body);
        }
    }

    fn check_module(&mut self, module: &Module) {
        for function in &module.items {
            self.check_function(function);
        }
    }

    fn check_function(&mut self, function: &Function) {
        // Enter function scope
        self.enter_scope();

        // Add parameters to scope
        for param in &function.params {
            let param_type = Type::from_ast_type(&param.ty);
            self.declare_variable(&param.name, param_type, param.position.clone());
        }

        // Check function body
        if let Some(body) = &function.body {
            self.check_block(body);
        }

        // Exit function scope
        self.exit_scope();
    }

    fn check_block(&mut self, block: &Block) {
        // Enter block scope
        self.enter_scope();

        // Check each statement
        for statement in &block.statements {
            self.check_statement(statement);
        }

        // Exit block scope
        self.exit_scope();
    }

    fn check_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Let(let_decl) => self.check_let_statement(let_decl),
            Statement::Return(return_stmt) => self.check_return_statement(return_stmt),
            Statement::If(if_stmt) => self.check_if_statement(if_stmt),
            Statement::While(while_stmt) => self.check_while_statement(while_stmt),
            Statement::Loop(loop_stmt) => self.check_loop_statement(loop_stmt),
            Statement::Break(pos) => {
                if self.loop_depth == 0 {
                    self.add_error(
                        "break is only valid inside a loop".to_string(),
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
            Statement::ExprStatement(expr) => {
                self.check_expression(expr);
            }
        }
    }

    fn check_let_statement(&mut self, let_decl: &LetDecl) {
        let declared_type = let_decl.ty.as_ref().map(|ty| Type::from_ast_type(ty));
        let initializer_type = let_decl.initializer.as_ref().map(|expr| {
            // Special handling for literals when there's a declared type
            if let Some(ref declared) = declared_type {
                self.check_expression_with_expected_type(expr, declared)
            } else {
                self.check_expression(expr)
            }
        });

        let final_type = match (declared_type, initializer_type) {
            (Some(declared), Some(inferred)) => {
                // Check that initializer matches declared type
                if declared != inferred {
                    self.add_error(
                        format!("Type mismatch: declared {} but initializer has type {}",
                                declared, inferred),
                        let_decl.position.clone()
                    );
                    declared // Use declared type for symbol table
                } else {
                    declared
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
                // Error: no type and no initializer
                self.add_error(
                    "Type cannot be inferred without an initializer; either add a type annotation or an initializer".to_string(),
                    let_decl.position.clone()
                );
                Type::Unknown
            }
        };

        // Add to symbol table
        self.declare_variable(&let_decl.name, final_type, let_decl.position.clone());
    }

    fn check_return_statement(&mut self, return_stmt: &ReturnStmt) {
        if let Some(value) = &return_stmt.value {
            self.check_expression(value);
        }
        // TODO: Check that return type matches function return type
        // For now, just type check the expression
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

        // Check then block
        self.check_block(&if_stmt.then_block);

        // Check else block if present
        if let Some(else_block) = &if_stmt.else_block {
            self.check_block(else_block);
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

    fn check_assign_statement(&mut self, assign_stmt: &AssignStmt) {
        let target_type = self.check_expression(&assign_stmt.target);
        let value_type = self.check_expression(&assign_stmt.value);

        // For assignment, types must match exactly (no coercion)
        if target_type != value_type {
            self.add_error(
                format!("Cannot assign {} to {}", value_type, target_type),
                assign_stmt.position.clone()
            );
        }

        // TODO: Check that target is assignable (not a constant, etc.)
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
            Expression::BoolLit(_, _) => Type::Bool,
            Expression::Ident(name, pos) => {
                // Look up variable in symbol table
                self.lookup_variable(name, pos.clone())
            },
            Expression::BinaryOp(binary_op) => self.check_binary_op(binary_op),
            Expression::UnaryOp(unary_op) => self.check_unary_op(unary_op),
            Expression::Call(call) => self.check_call(call),
            Expression::MemberAccess(_) => {
                // TODO: Implement member access type checking
                Type::Unknown
            },
            Expression::IndexAccess(_) => {
                // TODO: Implement index access type checking
                Type::Unknown
            },
            Expression::IsCheck(is_check) => {
                // is/is not always returns bool
                self.check_expression(&is_check.expression); // Type check the expression
                // Note: The type in is_check.ty is already validated during parsing
                Type::Bool
            },
            Expression::QualifiedOp(qualified_op) => self.check_qualified_op_expression(qualified_op),
        }
    }

    fn check_binary_op(&mut self, binary_op: &BinaryOp) -> Type {
        let lhs_type = self.check_expression(&binary_op.lhs);
        let rhs_type = self.check_expression(&binary_op.rhs);

        match binary_op.op {
            // Arithmetic operators: both operands must be same numeric type
            BinaryOpKind::Add | BinaryOpKind::Sub | BinaryOpKind::Mul |
            BinaryOpKind::Div | BinaryOpKind::Mod => {
                if !lhs_type.is_numeric() {
                    self.add_error(
                        format!("Cannot apply arithmetic operator to non-numeric type {}", lhs_type),
                        binary_op.position.clone()
                    );
                    return Type::Unknown;
                }

                if lhs_type != rhs_type {
                    self.add_error(
                        format!("Cannot {} {} and {}; types must match exactly",
                                binary_op.op.operator_name(), lhs_type, rhs_type),
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
                if !lhs_type.is_numeric() || !rhs_type.is_numeric() {
                    self.add_error(
                        format!("Cannot compare non-numeric types {} and {}", lhs_type, rhs_type),
                        binary_op.position.clone()
                    );
                    return Type::Unknown;
                }

                if lhs_type != rhs_type {
                    self.add_error(
                        format!("Cannot compare {} and {}; types must match exactly", lhs_type, rhs_type),
                        binary_op.position.clone()
                    );
                    return Type::Unknown;
                }

                Type::Bool
            },

            // Equality operators: both operands must be same type (any type), result is bool
            BinaryOpKind::Eq | BinaryOpKind::NotEq => {
                if lhs_type != rhs_type {
                    self.add_error(
                        format!("Cannot compare {} and {} for equality; types must match exactly",
                                lhs_type, rhs_type),
                        binary_op.position.clone()
                    );
                    return Type::Unknown;
                }

                Type::Bool
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
                // Logical not: operand must be bool
                if operand_type != Type::Bool {
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
        if let Expression::Ident(func_name, _) = call.callee.as_ref() {
            if func_name == "print" {
                return self.check_print_call(call);
            }
        }

        // Type check the callee
        self.check_expression(&call.callee);

        // Type check all arguments
        for arg in &call.args {
            self.check_expression(arg);
        }

        // TODO: Implement proper function type checking
        // For now, just return unknown
        Type::Unknown
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
            Type::F32 | Type::F64 | Type::Bool | Type::Usize | Type::Isize | Type::String => {
                // These types are printable
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

    // Scope management
    fn enter_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_variable(&mut self, name: &str, ty: Type, position: Position) {
        if let Some(current_scope) = self.scopes.last_mut() {
            current_scope.declare(name.to_string(), ty, position);
        }
    }

    fn lookup_variable(&mut self, name: &str, position: Position) -> Type {
        // Search scopes from innermost to outermost
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
            // Other types not yet implemented for truthy/falsy
            _ => false,
        }
    }

    fn check_qualified_op(&mut self, qualified_op: &QualifiedOp) {
        // Type check both operands
        let lhs_type = self.check_expression(&qualified_op.lhs);
        let rhs_type = self.check_expression(&qualified_op.rhs);

        // Check each qualifier in the list
        for qualifier_spec in &qualified_op.qualifiers {
            // Check if qualifier is defined
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

            // Check context (assignment vs equality)
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

            // Check arguments
            if let Err(err) = self.qualifier_registry.check_args(qualifier_name, qualifier_spec.arg.is_some()) {
                self.add_error(err, qualifier_spec.position.clone());
                continue;
            }

            // Check if qualifier applies to the operand types
            if !applies_to_type(&lhs_type) {
                self.add_error(
                    format!("qualifier '{}' requires compatible types; got {}",
                           qualifier_name, lhs_type),
                    qualifier_spec.position.clone(),
                );
            }

            // Check codegen status
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

    fn add_error(&mut self, message: String, position: Position) {
        self.errors.push(TypeError::new(message, position));
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
            Expression::BoolLit(_, pos) => pos.clone(),
            Expression::Ident(_, pos) => pos.clone(),
            Expression::BinaryOp(op) => op.position.clone(),
            Expression::UnaryOp(op) => op.position.clone(),
            Expression::Call(call) => call.position.clone(),
            Expression::MemberAccess(access) => access.position.clone(),
            Expression::IndexAccess(access) => access.position.clone(),
            Expression::IsCheck(check) => check.position.clone(),
            Expression::QualifiedOp(qualified_op) => qualified_op.position.clone(),
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