use crate::ast::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::process::Command;

pub struct CodeGenerator {
    output: String,
    indent_level: usize,
    variables: HashMap<String, String>,
}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {
            output: String::new(),
            indent_level: 0,
            variables: HashMap::new(),
        }
    }

    fn indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }

    fn emit(&mut self, code: &str) {
        self.output.push_str(&self.indent());
        self.output.push_str(code);
        self.output.push('\n');
    }

    fn emit_no_indent(&mut self, code: &str) {
        self.output.push_str(code);
    }

    pub fn generate(&mut self, program: &Program) -> Result<String, String> {
        // Generate C preamble
        self.emit("#include <stdio.h>");
        self.emit("#include <stdlib.h>");
        self.emit("#include <stdint.h>");
        self.emit("#include <stdbool.h>");
        self.emit("#include <string.h>");
        self.emit("");

        // Generate forward declarations
        for stmt in &program.statements {
            if let Statement::FunctionDecl { name, params, return_type, .. } = stmt {
                self.generate_function_declaration(name, params, return_type)?;
            }
        }

        self.emit("");

        // Generate function definitions
        for stmt in &program.statements {
            self.generate_statement(stmt)?;
        }

        Ok(self.output.clone())
    }

    fn generate_function_declaration(
        &mut self,
        name: &str,
        params: &[Parameter],
        return_type: &Option<Type>,
    ) -> Result<(), String> {
        let ret_type = return_type
            .as_ref()
            .map(|t| self.type_to_c(t))
            .unwrap_or_else(|| "void".to_string());

        self.emit_no_indent(&ret_type);
        self.emit_no_indent(" ");
        self.emit_no_indent(name);
        self.emit_no_indent("(");

        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                self.emit_no_indent(", ");
            }
            self.emit_no_indent(&self.type_to_c(&param.param_type));
            self.emit_no_indent(" ");
            self.emit_no_indent(&param.name);
        }

        self.emit_no_indent(")");
        self.output.push_str(";\n");

        Ok(())
    }

    fn generate_statement(&mut self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::FunctionDecl {
                name,
                params,
                return_type,
                body,
            } => {
                let ret_type = return_type
                    .as_ref()
                    .map(|t| self.type_to_c(t))
                    .unwrap_or_else(|| "void".to_string());

                self.emit_no_indent(&ret_type);
                self.emit_no_indent(" ");
                self.emit_no_indent(name);
                self.emit_no_indent("(");

                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        self.emit_no_indent(", ");
                    }
                    let c_type = self.type_to_c(&param.param_type);
                    self.emit_no_indent(&c_type);
                    self.emit_no_indent(" ");
                    self.emit_no_indent(&param.name);

                    // Store variable type
                    self.variables.insert(param.name.clone(), c_type);
                }

                self.emit_no_indent(") {");
                self.output.push('\n');

                self.indent_level += 1;
                self.generate_block(body)?;
                self.indent_level -= 1;

                self.emit("}");
                self.emit("");
            }

            Statement::VariableDecl {
                name,
                var_type,
                initializer,
            } => {
                let c_type = if let Some(t) = var_type {
                    self.type_to_c(t)
                } else {
                    // Try to infer from initializer
                    if let Some(Expression::IntegerLiteral(_)) = initializer {
                        "int32_t".to_string()
                    } else if let Some(Expression::FloatLiteral(_)) = initializer {
                        "double".to_string()
                    } else if let Some(Expression::StringLiteral(_)) = initializer {
                        "char*".to_string()
                    } else if let Some(Expression::BooleanLiteral(_)) = initializer {
                        "bool".to_string()
                    } else {
                        return Err("Cannot infer type for variable".to_string());
                    }
                };

                self.variables.insert(name.clone(), c_type.clone());

                self.emit_no_indent(&self.indent());
                self.emit_no_indent(&c_type);
                self.emit_no_indent(" ");
                self.emit_no_indent(name);

                if let Some(init) = initializer {
                    self.emit_no_indent(" = ");
                    self.generate_expression(init)?;
                }

                self.output.push_str(";\n");
            }

            Statement::Return { value } => {
                self.emit_no_indent(&self.indent());
                self.emit_no_indent("return");

                if let Some(expr) = value {
                    self.emit_no_indent(" ");
                    self.generate_expression(expr)?;
                }

                self.output.push_str(";\n");
            }

            Statement::Expression(expr) => {
                self.emit_no_indent(&self.indent());
                self.generate_expression(expr)?;
                self.output.push_str(";\n");
            }

            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.emit_no_indent(&self.indent());
                self.emit_no_indent("if (");
                self.generate_expression(condition)?;
                self.emit_no_indent(") {\n");

                self.indent_level += 1;
                self.generate_block(then_branch)?;
                self.indent_level -= 1;

                if let Some(else_stmt) = else_branch {
                    self.emit("} else {");
                    self.indent_level += 1;

                    match else_stmt.as_ref() {
                        Statement::Block(block) => self.generate_block(block)?,
                        other => self.generate_statement(other)?,
                    }

                    self.indent_level -= 1;
                    self.emit("}");
                } else {
                    self.emit("}");
                }
            }

            Statement::While { condition, body } => {
                self.emit_no_indent(&self.indent());
                self.emit_no_indent("while (");
                self.generate_expression(condition)?;
                self.emit_no_indent(") {\n");

                self.indent_level += 1;
                self.generate_block(body)?;
                self.indent_level -= 1;

                self.emit("}");
            }

            Statement::For {
                init,
                condition,
                increment,
                body,
            } => {
                self.emit_no_indent(&self.indent());
                self.emit_no_indent("for (");

                if let Some(init_stmt) = init {
                    match init_stmt.as_ref() {
                        Statement::VariableDecl {
                            name,
                            var_type,
                            initializer,
                        } => {
                            let c_type = var_type
                                .as_ref()
                                .map(|t| self.type_to_c(t))
                                .unwrap_or_else(|| "int32_t".to_string());
                            self.emit_no_indent(&c_type);
                            self.emit_no_indent(" ");
                            self.emit_no_indent(name);
                            if let Some(init) = initializer {
                                self.emit_no_indent(" = ");
                                self.generate_expression(init)?;
                            }
                        }
                        _ => return Err("Invalid for loop initializer".to_string()),
                    }
                }

                self.emit_no_indent("; ");

                if let Some(cond) = condition {
                    self.generate_expression(cond)?;
                }

                self.emit_no_indent("; ");

                if let Some(inc) = increment {
                    self.generate_expression(inc)?;
                }

                self.emit_no_indent(") {\n");

                self.indent_level += 1;
                self.generate_block(body)?;
                self.indent_level -= 1;

                self.emit("}");
            }

            Statement::Block(block) => {
                self.emit("{");
                self.indent_level += 1;
                self.generate_block(block)?;
                self.indent_level -= 1;
                self.emit("}");
            }
        }

        Ok(())
    }

    fn generate_block(&mut self, block: &Block) -> Result<(), String> {
        for stmt in &block.statements {
            self.generate_statement(stmt)?;
        }
        Ok(())
    }

    fn generate_expression(&mut self, expr: &Expression) -> Result<(), String> {
        match expr {
            Expression::IntegerLiteral(n) => {
                self.emit_no_indent(&n.to_string());
            }

            Expression::FloatLiteral(f) => {
                self.emit_no_indent(&f.to_string());
            }

            Expression::StringLiteral(s) => {
                self.emit_no_indent(&format!("\"{}\"", s.escape_default()));
            }

            Expression::BooleanLiteral(b) => {
                self.emit_no_indent(if *b { "true" } else { "false" });
            }

            Expression::Identifier(name) => {
                self.emit_no_indent(name);
            }

            Expression::Binary { left, op, right } => {
                self.emit_no_indent("(");
                self.generate_expression(left)?;
                self.emit_no_indent(" ");
                self.emit_no_indent(&self.binary_op_to_c(op));
                self.emit_no_indent(" ");
                self.generate_expression(right)?;
                self.emit_no_indent(")");
            }

            Expression::Unary { op, operand } => {
                self.emit_no_indent(&self.unary_op_to_c(op));
                self.emit_no_indent("(");
                self.generate_expression(operand)?;
                self.emit_no_indent(")");
            }

            Expression::Call { callee, args } => {
                // Special case for print function
                if let Expression::Identifier(name) = callee.as_ref() {
                    if name == "print" {
                        self.emit_no_indent("printf(");
                        for (i, arg) in args.iter().enumerate() {
                            if i > 0 {
                                self.emit_no_indent(", ");
                            }

                            // Generate format string for print
                            if let Expression::StringLiteral(_) = arg {
                                self.emit_no_indent("\"%s\\n\"");
                                self.emit_no_indent(", ");
                                self.generate_expression(arg)?;
                            } else {
                                self.generate_expression(arg)?;
                            }
                        }
                        self.emit_no_indent(")");
                        return Ok(());
                    }
                }

                self.generate_expression(callee)?;
                self.emit_no_indent("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.emit_no_indent(", ");
                    }
                    self.generate_expression(arg)?;
                }
                self.emit_no_indent(")");
            }

            Expression::Assignment { target, value } => {
                self.generate_expression(target)?;
                self.emit_no_indent(" = ");
                self.generate_expression(value)?;
            }
        }

        Ok(())
    }

    fn type_to_c(&self, ty: &Type) -> String {
        match ty {
            Type::I8 => "int8_t".to_string(),
            Type::I16 => "int16_t".to_string(),
            Type::I32 => "int32_t".to_string(),
            Type::I64 => "int64_t".to_string(),
            Type::I128 => "__int128".to_string(),
            Type::U8 => "uint8_t".to_string(),
            Type::U16 => "uint16_t".to_string(),
            Type::U32 => "uint32_t".to_string(),
            Type::U64 => "uint64_t".to_string(),
            Type::U128 => "unsigned __int128".to_string(),
            Type::F32 => "float".to_string(),
            Type::F64 => "double".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "char*".to_string(),
            Type::Nothing => "void*".to_string(),
            Type::Unknown => "void*".to_string(),
            Type::Array { element_type, size } => {
                if let Some(_s) = size {
                    format!("{}*", self.type_to_c(element_type))
                } else {
                    format!("{}*", self.type_to_c(element_type))
                }
            }
            Type::Function { .. } => "void*".to_string(),
        }
    }

    fn binary_op_to_c(&self, op: &BinaryOp) -> String {
        match op {
            BinaryOp::Add => "+".to_string(),
            BinaryOp::Subtract => "-".to_string(),
            BinaryOp::Multiply => "*".to_string(),
            BinaryOp::Divide => "/".to_string(),
            BinaryOp::Modulo => "%".to_string(),
            BinaryOp::Equal | BinaryOp::StrictEqual => "==".to_string(),
            BinaryOp::NotEqual | BinaryOp::StrictNotEqual => "!=".to_string(),
            BinaryOp::Less => "<".to_string(),
            BinaryOp::LessEqual => "<=".to_string(),
            BinaryOp::Greater => ">".to_string(),
            BinaryOp::GreaterEqual => ">=".to_string(),
            BinaryOp::And => "&&".to_string(),
            BinaryOp::Or => "||".to_string(),
            BinaryOp::BitwiseAnd => "&".to_string(),
            BinaryOp::BitwiseOr => "|".to_string(),
            BinaryOp::BitwiseXor => "^".to_string(),
            BinaryOp::ShiftLeft => "<<".to_string(),
            BinaryOp::ShiftRight => ">>".to_string(),
        }
    }

    fn unary_op_to_c(&self, op: &UnaryOp) -> String {
        match op {
            UnaryOp::Negate => "-".to_string(),
            UnaryOp::Not => "!".to_string(),
            UnaryOp::BitwiseNot => "~".to_string(),
        }
    }
}

pub fn compile(program: &Program, output_path: &str, optimization: u8) -> Result<(), String> {
    let mut codegen = CodeGenerator::new();
    let c_code = codegen.generate(program)?;

    // Write C code to temporary file
    let c_file_path = format!("{}.c", output_path);
    let mut c_file = File::create(&c_file_path)
        .map_err(|e| format!("Failed to create C file: {}", e))?;

    c_file
        .write_all(c_code.as_bytes())
        .map_err(|e| format!("Failed to write C file: {}", e))?;

    // Compile C code with GCC
    let opt_flag = format!("-O{}", optimization);
    let output = Command::new("gcc")
        .args(&[
            &c_file_path,
            "-o",
            output_path,
            &opt_flag,
            "-std=c11",
        ])
        .output()
        .map_err(|e| format!("Failed to run GCC: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("GCC compilation failed:\n{}", stderr));
    }

    // Clean up C file
    std::fs::remove_file(&c_file_path)
        .map_err(|e| format!("Failed to remove temporary C file: {}", e))?;

    Ok(())
}
