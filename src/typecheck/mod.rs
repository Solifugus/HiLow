use crate::ast::{self, *};
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
    method_context: Option<Type>, // Track the receiver object type when inside a method
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::new()], // Start with global scope
            errors: Vec::new(),
            loop_depth: 0,
            qualifier_registry: QualifierRegistry::new(),
            method_context: None,
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
            self.check_program_body(body);
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

    fn check_program_body(&mut self, body: &ProgramBody) {
        // Enter program body scope
        self.enter_scope();

        // Phase 1: Declare all nested functions in the current scope first
        for item in &body.items {
            if let BlockItem::Function(function) = item {
                // For Phase 6a-fixup, nested functions can't access enclosing variables
                // They're treated as top-level functions that happen to be declared nested
                // Add function to symbol table - for simplicity, use return type as the function type
                let func_type = Type::from_ast_type(&function.return_type);
                self.declare_variable(&function.name, func_type, function.position.clone());
            }
        }

        // Phase 2: Check function bodies and statements
        for item in &body.items {
            match item {
                BlockItem::Statement(statement) => self.check_statement(statement),
                BlockItem::Function(function) => self.check_function(function),
            }
        }

        // Exit program body scope
        self.exit_scope();
    }

    fn check_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Let(let_decl) => self.check_let_statement(let_decl),
            Statement::Return(return_stmt) => self.check_return_statement(return_stmt),
            Statement::If(if_stmt) => self.check_if_statement(if_stmt),
            Statement::While(while_stmt) => self.check_while_statement(while_stmt),
            Statement::Loop(loop_stmt) => self.check_loop_statement(loop_stmt),
            Statement::ForIn(for_in_stmt) => self.check_for_in_statement(for_in_stmt),
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

    fn check_for_in_statement(&mut self, for_in_stmt: &ForInStmt) {
        // Check that the iterable expression is an object type
        let iterable_type = self.check_expression(&for_in_stmt.iterable);
        match iterable_type {
            Type::Object(_) => {
                // Valid - we can iterate over objects
            }
            _ => {
                self.add_error(
                    format!("for-in requires an object; got {}", iterable_type),
                    for_in_stmt.position.clone()
                );
                return;
            }
        }

        // Enter loop scope for break/continue validation
        self.loop_depth += 1;

        // Enter a new scope for the loop variables
        self.enter_scope();

        // Declare the loop variables in the new scope
        // Key is always string type
        self.declare_variable(
            &for_in_stmt.key_name,
            Type::String,
            for_in_stmt.position.clone()
        );

        // Value is a polymorphic type that allows runtime dispatch
        // For Phase 7c-ζ, we'll use a special type that only allows certain operations
        self.declare_variable(
            &for_in_stmt.value_name,
            Type::ObjectIterValue, // Special type for iteration values
            for_in_stmt.position.clone()
        );

        // Check the loop body
        self.check_block(&for_in_stmt.body);

        // Exit the loop scope
        self.exit_scope();
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
            Expression::FString(fstring) => self.check_fstring(fstring),
            Expression::BoolLit(_, _) => Type::Bool,
            Expression::Ident(name, pos) => {
                // Look up variable in symbol table
                self.lookup_variable(name, pos.clone())
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
        let callee_type = self.check_expression(&call.callee);

        // Type check all arguments
        for arg in &call.args {
            self.check_expression(arg);
        }

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

            // TODO: Validate argument types match parameter types
            // For now, just return the return type
            return *return_type.clone();
        }

        // Phase 6a-fixup: For nested functions, return the function's return type
        // For now, we use a simple approach: if callee is a function identifier,
        // return the type we stored (which is the return type)
        if let Expression::Ident(func_name, _) = call.callee.as_ref() {
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
                        Type::ObjectIterValue => {
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
        let object_type = self.check_expression(&member_access.object);

        match object_type {
            Type::Object(_) => {
                // Use prototype chain lookup (Phase 7b)
                if let Some(prop_type) = self.find_property_in_chain(&object_type, &member_access.member, 0) {
                    prop_type
                } else {
                    // Property not found anywhere in the chain
                    self.add_error(
                        format!("Object does not have property '{}' (Phase 9 will allow runtime property access)", member_access.member),
                        member_access.position.clone()
                    );
                    Type::Unknown
                }
            },
            Type::Unknown => Type::Unknown, // Error recovery
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
            Expression::BoolLit(_, _) => Type::Bool,
            Expression::Ident(name, _) => {
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

        // Add parameters to the scope
        let mut param_types = Vec::new();
        for param in &func_expr.params {
            let param_type = Type::from_ast_type(&param.ty);
            param_types.push(param_type.clone());
            self.declare_variable(&param.name, param_type, param.position.clone());
        }

        // Phase 7c-γ: Collect capture metadata before checking for errors
        let mut captures = Vec::new();
        for statement in &func_expr.body.statements {
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
        for statement in &func_expr.body.statements {
            self.check_statement(statement);
        }

        // TODO: Validate return statements match the declared return type
        // For now, just convert the return type from AST to type system
        let return_type = Type::from_ast_type(&func_expr.return_type);

        self.exit_scope();

        // Return the function type
        Type::Function(param_types, Box::new(return_type))
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
                for stmt in &if_stmt.then_block.statements {
                    self.check_for_captures_in_statement(stmt, outer_scope_depth);
                }
                if let Some(else_block) = &if_stmt.else_block {
                    for stmt in &else_block.statements {
                        self.check_for_captures_in_statement(stmt, outer_scope_depth);
                    }
                }
            }
            Statement::While(while_stmt) => {
                self.check_for_captures_in_expression(&while_stmt.condition, outer_scope_depth);
                for stmt in &while_stmt.body.statements {
                    self.check_for_captures_in_statement(stmt, outer_scope_depth);
                }
            }
            Statement::Loop(loop_stmt) => {
                for stmt in &loop_stmt.body.statements {
                    self.check_for_captures_in_statement(stmt, outer_scope_depth);
                }
            }
            Statement::ForIn(for_in_stmt) => {
                self.check_for_captures_in_expression(&for_in_stmt.iterable, outer_scope_depth);
                for stmt in &for_in_stmt.body.statements {
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
            Statement::Break(_) | Statement::Continue(_) => {
                // No expressions to check
            }
        }
    }

    fn check_for_captures_in_expression(&mut self, expression: &Expression, outer_scope_depth: usize) {
        match expression {
            Expression::Ident(name, pos) => {
                // Check if this identifier refers to a variable from an outer scope
                if self.is_variable_capture(name, outer_scope_depth) {
                    self.add_error(
                        "function expressions cannot capture variables (Phase 7c-γ will add capture detection, Phase 7c-δ will implement closures)".to_string(),
                        pos.clone()
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
                            for stmt in &block.statements {
                                self.check_for_captures_in_statement(stmt, outer_scope_depth);
                            }
                        }
                    }
                }
            }
            // Literal expressions don't contain variable references
            Expression::IntLit(_, _) | Expression::FloatLit(_, _) | Expression::StringLit(_, _) |
            Expression::FString(_) | Expression::BoolLit(_, _) | Expression::This(_) => {
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
                for stmt in &if_stmt.then_block.statements {
                    self.collect_captures_in_statement(stmt, outer_scope_depth, captures);
                }
                if let Some(else_block) = &if_stmt.else_block {
                    for stmt in &else_block.statements {
                        self.collect_captures_in_statement(stmt, outer_scope_depth, captures);
                    }
                }
            }
            Statement::While(while_stmt) => {
                self.collect_captures_in_expression(&while_stmt.condition, outer_scope_depth, captures);
                for stmt in &while_stmt.body.statements {
                    self.collect_captures_in_statement(stmt, outer_scope_depth, captures);
                }
            }
            Statement::Loop(loop_stmt) => {
                for stmt in &loop_stmt.body.statements {
                    self.collect_captures_in_statement(stmt, outer_scope_depth, captures);
                }
            }
            Statement::ForIn(for_in_stmt) => {
                self.collect_captures_in_expression(&for_in_stmt.iterable, outer_scope_depth, captures);
                for stmt in &for_in_stmt.body.statements {
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
            Statement::Break(_) | Statement::Continue(_) => {
                // No expressions to check
            }
        }
    }

    fn collect_captures_in_expression(&mut self, expression: &Expression, outer_scope_depth: usize, captures: &mut Vec<(String, Type, Position)>) {
        match expression {
            Expression::Ident(name, pos) => {
                // Check if this identifier refers to a variable from an outer scope
                if let Some((var_type, _)) = self.find_variable_in_outer_scope(name, outer_scope_depth) {
                    // Check if we already recorded this capture (by name)
                    if !captures.iter().any(|(capture_name, _, _)| capture_name == name) {
                        captures.push((name.clone(), var_type, pos.clone()));
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
                            for stmt in &block.statements {
                                self.collect_captures_in_statement(stmt, outer_scope_depth, captures);
                            }
                        }
                    }
                }
            }
            // Literal expressions don't contain variable references
            Expression::IntLit(_, _) | Expression::FloatLit(_, _) | Expression::StringLit(_, _) |
            Expression::FString(_) | Expression::BoolLit(_, _) | Expression::This(_) => {
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
            Expression::FString(fstring) => fstring.position.clone(),
            Expression::BoolLit(_, pos) => pos.clone(),
            Expression::Ident(_, pos) => pos.clone(),
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