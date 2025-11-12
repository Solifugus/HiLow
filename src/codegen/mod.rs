use crate::ast::*;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::process::Command;

#[derive(Clone)]
struct LambdaInfo {
    name: String,
    captured_vars: HashSet<String>,
    context_struct: String,
}

pub struct CodeGenerator {
    output: String,
    indent_level: usize,
    variables: HashMap<String, String>,
    lambda_counter: usize,
    lambda_functions: Vec<String>,
    lambda_info: Vec<LambdaInfo>,
    defer_stack: Vec<Vec<Statement>>,
}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {
            output: String::new(),
            indent_level: 0,
            variables: HashMap::new(),
            lambda_counter: 0,
            lambda_functions: Vec::new(),
            lambda_info: Vec::new(),
            defer_stack: vec![Vec::new()],
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
        self.emit("#define _GNU_SOURCE");
        self.emit("#include <stdio.h>");
        self.emit("#include <stdlib.h>");
        self.emit("#include <stdint.h>");
        self.emit("#include <stdbool.h>");
        self.emit("#include <string.h>");
        self.emit("#include <ctype.h>");
        self.emit("#include <math.h>");
        self.emit("");

        // Generate unknown type structure
        self.emit("// Unknown type structure");
        self.emit("typedef struct {");
        self.emit("    char* reason;");
        self.emit("    char** options;");
        self.emit("    int32_t option_count;");
        self.emit("} Unknown;");
        self.emit("");
        self.emit("Unknown* create_unknown(const char* reason) {");
        self.emit("    Unknown* u = malloc(sizeof(Unknown));");
        self.emit("    u->reason = strdup(reason);");
        self.emit("    u->options = NULL;");
        self.emit("    u->option_count = 0;");
        self.emit("    return u;");
        self.emit("}");
        self.emit("");

        // Generate dynamic array structure
        self.emit("// Dynamic array structure");
        self.emit("typedef struct {");
        self.emit("    void* data;");
        self.emit("    int32_t length;");
        self.emit("    int32_t capacity;");
        self.emit("    size_t element_size;");
        self.emit("} DynamicArray;");
        self.emit("");
        self.emit("DynamicArray* array_new(size_t element_size) {");
        self.emit("    DynamicArray* arr = malloc(sizeof(DynamicArray));");
        self.emit("    arr->capacity = 4;");
        self.emit("    arr->length = 0;");
        self.emit("    arr->element_size = element_size;");
        self.emit("    arr->data = malloc(arr->capacity * element_size);");
        self.emit("    return arr;");
        self.emit("}");
        self.emit("");
        self.emit("void array_push_i32(DynamicArray* arr, int32_t item) {");
        self.emit("    if (arr->length >= arr->capacity) {");
        self.emit("        arr->capacity *= 2;");
        self.emit("        arr->data = realloc(arr->data, arr->capacity * arr->element_size);");
        self.emit("    }");
        self.emit("    ((int32_t*)arr->data)[arr->length++] = item;");
        self.emit("}");
        self.emit("");
        self.emit("int32_t array_pop_i32(DynamicArray* arr) {");
        self.emit("    if (arr->length == 0) return 0;");
        self.emit("    return ((int32_t*)arr->data)[--arr->length];");
        self.emit("}");
        self.emit("");
        self.emit("void array_push_string(DynamicArray* arr, char* item) {");
        self.emit("    if (arr->length >= arr->capacity) {");
        self.emit("        arr->capacity *= 2;");
        self.emit("        arr->data = realloc(arr->data, arr->capacity * arr->element_size);");
        self.emit("    }");
        self.emit("    ((char**)arr->data)[arr->length++] = item;");
        self.emit("}");
        self.emit("");
        self.emit("DynamicArray* str_split(const char* str, const char* delim) {");
        self.emit("    DynamicArray* result = array_new(sizeof(char*));");
        self.emit("    char* str_copy = strdup(str);");
        self.emit("    char* token = strtok(str_copy, delim);");
        self.emit("    while (token != NULL) {");
        self.emit("        array_push_string(result, strdup(token));");
        self.emit("        token = strtok(NULL, delim);");
        self.emit("    }");
        self.emit("    free(str_copy);");
        self.emit("    return result;");
        self.emit("}");
        self.emit("");
        self.emit("char* array_join_string(DynamicArray* arr, const char* sep) {");
        self.emit("    if (arr->length == 0) return strdup(\"\");");
        self.emit("    int total_len = 0;");
        self.emit("    for (int i = 0; i < arr->length; i++) {");
        self.emit("        total_len += strlen(((char**)arr->data)[i]);");
        self.emit("    }");
        self.emit("    total_len += strlen(sep) * (arr->length - 1);");
        self.emit("    char* result = malloc(total_len + 1);");
        self.emit("    result[0] = '\\0';");
        self.emit("    for (int i = 0; i < arr->length; i++) {");
        self.emit("        if (i > 0) strcat(result, sep);");
        self.emit("        strcat(result, ((char**)arr->data)[i]);");
        self.emit("    }");
        self.emit("    return result;");
        self.emit("}");
        self.emit("");
        self.emit("void array_reverse_i32(DynamicArray* arr) {");
        self.emit("    int32_t* data = (int32_t*)arr->data;");
        self.emit("    for (int i = 0; i < arr->length / 2; i++) {");
        self.emit("        int32_t temp = data[i];");
        self.emit("        data[i] = data[arr->length - 1 - i];");
        self.emit("        data[arr->length - 1 - i] = temp;");
        self.emit("    }");
        self.emit("}");
        self.emit("");
        self.emit("DynamicArray* array_map_i32(DynamicArray* arr, int32_t(*func)(int32_t, int32_t)) {");
        self.emit("    DynamicArray* result = array_new(sizeof(int32_t));");
        self.emit("    for (int i = 0; i < arr->length; i++) {");
        self.emit("        int32_t val = ((int32_t*)arr->data)[i];");
        self.emit("        int32_t mapped = func(val, 0);");
        self.emit("        array_push_i32(result, mapped);");
        self.emit("    }");
        self.emit("    return result;");
        self.emit("}");
        self.emit("");
        self.emit("DynamicArray* array_filter_i32(DynamicArray* arr, int32_t(*func)(int32_t, int32_t)) {");
        self.emit("    DynamicArray* result = array_new(sizeof(int32_t));");
        self.emit("    for (int i = 0; i < arr->length; i++) {");
        self.emit("        int32_t val = ((int32_t*)arr->data)[i];");
        self.emit("        if (func(val, 0)) {");
        self.emit("            array_push_i32(result, val);");
        self.emit("        }");
        self.emit("    }");
        self.emit("    return result;");
        self.emit("}");
        self.emit("");
        self.emit("int32_t array_reduce_i32(DynamicArray* arr, int32_t(*func)(int32_t, int32_t), int32_t initial) {");
        self.emit("    int32_t result = initial;");
        self.emit("    for (int i = 0; i < arr->length; i++) {");
        self.emit("        int32_t val = ((int32_t*)arr->data)[i];");
        self.emit("        result = func(result, val);");
        self.emit("    }");
        self.emit("    return result;");
        self.emit("}");
        self.emit("");
        self.emit("void array_forEach_i32(DynamicArray* arr, int32_t(*func)(int32_t, int32_t)) {");
        self.emit("    for (int i = 0; i < arr->length; i++) {");
        self.emit("        int32_t val = ((int32_t*)arr->data)[i];");
        self.emit("        func(val, 0);");
        self.emit("    }");
        self.emit("}");
        self.emit("");

        // Generate string helper functions
        self.emit("// String helper functions");
        self.emit("char* str_to_upper(const char* str) {");
        self.emit("    int len = strlen(str);");
        self.emit("    char* result = malloc(len + 1);");
        self.emit("    for (int i = 0; i < len; i++) { result[i] = toupper(str[i]); }");
        self.emit("    result[len] = '\\0';");
        self.emit("    return result;");
        self.emit("}");
        self.emit("");
        self.emit("char* str_to_lower(const char* str) {");
        self.emit("    int len = strlen(str);");
        self.emit("    char* result = malloc(len + 1);");
        self.emit("    for (int i = 0; i < len; i++) { result[i] = tolower(str[i]); }");
        self.emit("    result[len] = '\\0';");
        self.emit("    return result;");
        self.emit("}");
        self.emit("");
        self.emit("char* str_trim(const char* str) {");
        self.emit("    while (*str && isspace(*str)) str++;");
        self.emit("    if (*str == '\\0') return strdup(\"\");");
        self.emit("    const char* end = str + strlen(str) - 1;");
        self.emit("    while (end > str && isspace(*end)) end--;");
        self.emit("    int len = end - str + 1;");
        self.emit("    char* result = malloc(len + 1);");
        self.emit("    strncpy(result, str, len);");
        self.emit("    result[len] = '\\0';");
        self.emit("    return result;");
        self.emit("}");
        self.emit("");
        self.emit("char* str_char_at(const char* str, int32_t index) {");
        self.emit("    if (index < 0 || index >= strlen(str)) return strdup(\"\");");
        self.emit("    char* result = malloc(2);");
        self.emit("    result[0] = str[index];");
        self.emit("    result[1] = '\\0';");
        self.emit("    return result;");
        self.emit("}");
        self.emit("");
        self.emit("char* str_substring(const char* str, int32_t start, int32_t end) {");
        self.emit("    int len = strlen(str);");
        self.emit("    if (start < 0) start = 0;");
        self.emit("    if (end > len) end = len;");
        self.emit("    if (start >= end) return strdup(\"\");");
        self.emit("    int sublen = end - start;");
        self.emit("    char* result = malloc(sublen + 1);");
        self.emit("    strncpy(result, str + start, sublen);");
        self.emit("    result[sublen] = '\\0';");
        self.emit("    return result;");
        self.emit("}");
        self.emit("");
        self.emit("char* str_concat(const char* s1, const char* s2) {");
        self.emit("    int len = strlen(s1) + strlen(s2);");
        self.emit("    char* result = malloc(len + 1);");
        self.emit("    strcpy(result, s1);");
        self.emit("    strcat(result, s2);");
        self.emit("    return result;");
        self.emit("}");
        self.emit("");
        self.emit("char* str_replace(const char* str, const char* from, const char* to) {");
        self.emit("    char* pos = strstr(str, from);");
        self.emit("    if (!pos) return strdup(str);");
        self.emit("    int from_len = strlen(from);");
        self.emit("    int to_len = strlen(to);");
        self.emit("    int prefix_len = pos - str;");
        self.emit("    int suffix_len = strlen(pos + from_len);");
        self.emit("    char* result = malloc(prefix_len + to_len + suffix_len + 1);");
        self.emit("    strncpy(result, str, prefix_len);");
        self.emit("    strcpy(result + prefix_len, to);");
        self.emit("    strcpy(result + prefix_len + to_len, pos + from_len);");
        self.emit("    return result;");
        self.emit("}");
        self.emit("");

        // First pass: Process all statements to collect lambda functions
        // We need to do this to know what lambdas to forward-declare
        let mut temp_gen = CodeGenerator::new();
        for stmt in &program.statements {
            temp_gen.generate_statement(stmt)?;
        }

        // Now emit the collected lambda functions
        for lambda_func in &temp_gen.lambda_functions {
            self.output.push_str(lambda_func);
        }

        // Generate forward declarations for regular functions
        for stmt in &program.statements {
            if let Statement::FunctionDecl { name, params, return_type, is_export: _, .. } = stmt {
                self.generate_function_declaration(name, params, return_type)?;
            }
        }

        self.emit("");

        // Second pass: Generate actual function definitions
        // Copy the lambda_functions and lambda_info from temp_gen
        self.lambda_functions = temp_gen.lambda_functions.clone();
        self.lambda_info = temp_gen.lambda_info.clone();
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
                is_export: _,
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
                is_export: _,
            } => {
                // Special handling for dynamic arrays (no size)
                if let Some(Type::Array { element_type, size: None }) = var_type {
                    let elem_c_type = self.type_to_c(element_type);
                    self.variables.insert(name.clone(), "DynamicArray*".to_string());

                    self.emit_no_indent(&self.indent());
                    self.emit_no_indent("DynamicArray* ");
                    self.emit_no_indent(name);

                    if let Some(init) = initializer {
                        // Use the initializer (e.g., from split())
                        self.emit_no_indent(" = ");
                        self.generate_expression(init)?;
                    } else {
                        // Create new empty array
                        self.emit_no_indent(" = ");
                        self.emit_no_indent(&format!("array_new(sizeof({}))", elem_c_type));
                    }

                    self.output.push_str(";\n");
                } else if let Some(Type::Array { element_type, size: Some(size) }) = var_type {
                    // Fixed-size arrays
                    let elem_c_type = self.type_to_c(element_type);
                    self.variables.insert(name.clone(), format!("{}*", elem_c_type));

                    self.emit_no_indent(&self.indent());
                    self.emit_no_indent(&elem_c_type);
                    self.emit_no_indent(" ");
                    self.emit_no_indent(name);
                    self.emit_no_indent(&format!("[{}]", size));

                    if let Some(init) = initializer {
                        self.emit_no_indent(" = ");
                        self.generate_expression(init)?;
                    }

                    self.output.push_str(";\n");
                } else {
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
                        } else if let Some(Expression::ObjectLiteral { properties }) = initializer {
                            // Generate struct type from object literal
                            let mut struct_def = "struct { ".to_string();
                            for (i, prop) in properties.iter().enumerate() {
                                if i > 0 {
                                    struct_def.push_str("; ");
                                }
                                struct_def.push_str("int32_t ");
                                struct_def.push_str(&prop.key);
                            }
                            struct_def.push_str("; }");
                            struct_def
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
            }

            Statement::Return { value } => {
                // Execute all defers before returning
                // Collect all defers from all scopes
                let mut all_defers = Vec::new();
                for scope in &self.defer_stack {
                    for defer_stmt in scope {
                        all_defers.push(defer_stmt.clone());
                    }
                }

                // Execute defers in reverse order
                for defer_stmt in all_defers.iter().rev() {
                    self.generate_statement(defer_stmt)?;
                }

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
                            is_export: _,
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

            Statement::ForIn { variable, iterable, body } => {
                // For now, we'll generate a C-style for loop that iterates over the array
                // This requires the array to be stored in a variable
                // We'll use a temporary index variable
                let index_var = format!("__idx_{}", variable);

                self.emit_no_indent(&self.indent());
                self.emit_no_indent(&format!("for (int32_t {} = 0; {} < ", index_var, index_var));

                // We need to know the array size - for now we'll use a macro-style approach
                // that requires the array to be in a variable
                self.emit_no_indent("sizeof(");
                self.generate_expression(iterable)?;
                self.emit_no_indent(")/sizeof((");
                self.generate_expression(iterable)?;
                self.emit_no_indent(")[0]); ");

                self.emit_no_indent(&format!("{}++) {{\n", index_var));

                self.indent_level += 1;

                // Declare the loop variable
                self.emit(&format!("int32_t {} = (", variable));
                self.emit_no_indent(&self.indent());
                self.generate_expression(iterable)?;
                self.emit_no_indent(&format!(")[{}];\n", index_var));

                // Generate loop body
                self.generate_block(body)?;

                self.indent_level -= 1;
                self.emit("}");
            }

            Statement::Break => {
                self.emit("break;");
            }

            Statement::Continue => {
                self.emit("continue;");
            }

            Statement::Defer { statement } => {
                // Add to current defer stack
                if let Some(current_scope) = self.defer_stack.last_mut() {
                    current_scope.push(statement.as_ref().clone());
                }
                // Don't emit anything here - defers execute at scope exit
            }

            Statement::Switch { expr, cases, default } => {
                self.emit_no_indent(&self.indent());
                self.emit_no_indent("switch (");
                self.generate_expression(expr)?;
                self.emit_no_indent(") {\n");

                self.indent_level += 1;

                for case in cases {
                    self.emit_no_indent(&self.indent());
                    self.emit_no_indent("case ");
                    self.generate_expression(&case.value)?;
                    self.emit_no_indent(":\n");

                    self.indent_level += 1;
                    self.generate_block(&case.body)?;
                    self.indent_level -= 1;
                }

                if let Some(default_block) = default {
                    self.emit("default:");
                    self.indent_level += 1;
                    self.generate_block(default_block)?;
                    self.indent_level -= 1;
                }

                self.indent_level -= 1;
                self.emit("}");
            }

            Statement::Import { .. } => {
                // Import statements don't generate code (handled at link time)
                // For now, we're single-file, so ignore
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
        // Push new defer scope
        self.defer_stack.push(Vec::new());

        for stmt in &block.statements {
            self.generate_statement(stmt)?;
        }

        // Execute defers in reverse order
        if let Some(defers) = self.defer_stack.pop() {
            for defer_stmt in defers.iter().rev() {
                self.generate_statement(defer_stmt)?;
            }
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

            Expression::FString { parts } => {
                use crate::ast::FStringPart;

                // For f-strings, we'll generate a format string and printf call
                // First pass: build the format string and collect arguments
                let mut format_str = String::new();
                let mut args = Vec::new();

                for part in parts {
                    match part {
                        FStringPart::Text(text) => {
                            // Escape % signs for printf
                            format_str.push_str(&text.replace("%", "%%"));
                        }
                        FStringPart::Expression(expr) => {
                            // Simple type inference for format specifier
                            // This is a simplification - a real implementation would use proper type checking
                            format_str.push_str("%d"); // Default to integer for now
                            args.push(expr);
                        }
                    }
                }

                // Generate the printf-style expression
                // We'll use a compound literal (string expression) in C
                self.emit_no_indent("(");
                self.emit_no_indent(&format!("\"{}\"", format_str));

                // For now, if we have expressions, we'll need to use sprintf or similar
                // Let's generate a simple concatenation for basic cases
                if !args.is_empty() {
                    self.emit_no_indent(", ");
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            self.emit_no_indent(", ");
                        }
                        self.generate_expression(arg)?;
                    }
                }
                self.emit_no_indent(")");
            }

            Expression::BooleanLiteral(b) => {
                self.emit_no_indent(if *b { "true" } else { "false" });
            }

            Expression::NothingLiteral => {
                self.emit_no_indent("NULL");
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
                // Special cases for built-in functions
                if let Expression::Identifier(name) = callee.as_ref() {
                    // String operations
                    if name == "string_length" && args.len() == 1 {
                        self.emit_no_indent("strlen(");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(")");
                        return Ok(());
                    }

                    if name == "string_index_of" && args.len() == 2 {
                        // Generate C code for indexOf using strstr
                        self.emit_no_indent("({char* __p = strstr(");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[1])?;
                        self.emit_no_indent("); __p ? (int32_t)(__p - (char*)(");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(")) : -1; })");
                        return Ok(());
                    }

                    if name == "make_unknown" && args.len() == 1 {
                        self.emit_no_indent("create_unknown(");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(")");
                        return Ok(());
                    }

                    if name == "string_concat" && args.len() == 2 {
                        // Simple concatenation using strcat (unsafe but works for demo)
                        self.emit_no_indent("strcat(");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[1])?;
                        self.emit_no_indent(")");
                        return Ok(());
                    }

                    if name == "string_compare" && args.len() == 2 {
                        self.emit_no_indent("strcmp(");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[1])?;
                        self.emit_no_indent(")");
                        return Ok(());
                    }

                    // Math functions
                    if name == "abs" && args.len() == 1 {
                        self.emit_no_indent("abs(");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(")");
                        return Ok(());
                    }

                    if name == "min" && args.len() == 2 {
                        self.emit_no_indent("(");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(" < ");
                        self.generate_expression(&args[1])?;
                        self.emit_no_indent(" ? ");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(" : ");
                        self.generate_expression(&args[1])?;
                        self.emit_no_indent(")");
                        return Ok(());
                    }

                    if name == "max" && args.len() == 2 {
                        self.emit_no_indent("(");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(" > ");
                        self.generate_expression(&args[1])?;
                        self.emit_no_indent(" ? ");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(" : ");
                        self.generate_expression(&args[1])?;
                        self.emit_no_indent(")");
                        return Ok(());
                    }

                    if name == "pow" && args.len() == 2 {
                        self.emit_no_indent("(int32_t)pow(");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[1])?;
                        self.emit_no_indent(")");
                        return Ok(());
                    }

                    if name == "sqrt" && args.len() == 1 {
                        self.emit_no_indent("(int32_t)sqrt(");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(")");
                        return Ok(());
                    }

                    if name == "print" {
                        self.emit_no_indent("printf(");
                        for (i, arg) in args.iter().enumerate() {
                            if i > 0 {
                                self.emit_no_indent(", ");
                            }

                            // Generate format string for print
                            match arg {
                                Expression::StringLiteral(_) => {
                                    self.emit_no_indent("\"%s\\n\"");
                                    self.emit_no_indent(", ");
                                    self.generate_expression(arg)?;
                                }
                                Expression::FString { parts } => {
                                    use crate::ast::FStringPart;

                                    // Build format string for the f-string
                                    let mut format_str = String::new();
                                    let mut fstring_args = Vec::new();

                                    for part in parts {
                                        match part {
                                            FStringPart::Text(text) => {
                                                format_str.push_str(&text.replace("%", "%%"));
                                            }
                                            FStringPart::Expression(expr) => {
                                                // Try to determine format specifier from expression type
                                                let format_spec = match expr.as_ref() {
                                                    Expression::StringLiteral(_) => "%s",
                                                    Expression::Identifier(name) => {
                                                        // Check if it's a string variable
                                                        if let Some(var_type) = self.variables.get(name) {
                                                            if var_type.contains("char*") {
                                                                "%s"
                                                            } else {
                                                                "%d"
                                                            }
                                                        } else {
                                                            "%d"
                                                        }
                                                    }
                                                    Expression::FloatLiteral(_) => "%f",
                                                    _ => "%d",
                                                };
                                                format_str.push_str(format_spec);
                                                fstring_args.push(expr);
                                            }
                                        }
                                    }

                                    format_str.push_str("\\n");
                                    self.emit_no_indent(&format!("\"{}\"", format_str));

                                    for expr in fstring_args {
                                        self.emit_no_indent(", ");
                                        self.generate_expression(expr)?;
                                    }
                                }
                                _ => {
                                    self.generate_expression(arg)?;
                                }
                            }
                        }
                        self.emit_no_indent(")");
                        return Ok(());
                    }
                }

                // Check if callee is a function pointer variable (void*)
                let is_function_ptr = if let Expression::Identifier(name) = callee.as_ref() {
                    self.variables.get(name)
                        .map(|t| t == "void*")
                        .unwrap_or(false)
                } else {
                    false
                };

                if is_function_ptr {
                    // Cast void* to function pointer and call
                    // For now, assume signature is int32_t(*)(int32_t, int32_t)
                    self.emit_no_indent("((int32_t(*)(");
                    for (i, _arg) in args.iter().enumerate() {
                        if i > 0 {
                            self.emit_no_indent(", ");
                        }
                        self.emit_no_indent("int32_t");
                    }
                    self.emit_no_indent("))");
                    self.generate_expression(callee)?;
                    self.emit_no_indent(")(");
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            self.emit_no_indent(", ");
                        }
                        self.generate_expression(arg)?;
                    }
                    self.emit_no_indent(")");
                } else {
                    // Regular function call
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
            }

            Expression::Assignment { target, value } => {
                self.generate_expression(target)?;
                self.emit_no_indent(" = ");
                self.generate_expression(value)?;
            }

            Expression::ArrayLiteral { elements } => {
                self.emit_no_indent("{");
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        self.emit_no_indent(", ");
                    }
                    self.generate_expression(elem)?;
                }
                self.emit_no_indent("}");
            }

            Expression::Index { array, index } => {
                // Check if it's a dynamic array
                let is_dynamic_array = if let Expression::Identifier(name) = array.as_ref() {
                    self.variables.get(name)
                        .map(|t| t.contains("DynamicArray"))
                        .unwrap_or(false)
                } else {
                    false
                };

                if is_dynamic_array {
                    // Dynamic array indexing: arr->data[index]
                    self.emit_no_indent("((int32_t*)");
                    self.generate_expression(array)?;
                    self.emit_no_indent("->data)[");
                    self.generate_expression(index)?;
                    self.emit_no_indent("]");
                } else {
                    // Regular array indexing
                    self.generate_expression(array)?;
                    self.emit_no_indent("[");
                    self.generate_expression(index)?;
                    self.emit_no_indent("]");
                }
            }

            Expression::ObjectLiteral { properties } => {
                // For objects in C, we'll generate a compound literal with a struct
                self.emit_no_indent("{");
                for (i, prop) in properties.iter().enumerate() {
                    if i > 0 {
                        self.emit_no_indent(", ");
                    }
                    self.emit_no_indent(".");
                    self.emit_no_indent(&prop.key);
                    self.emit_no_indent(" = ");
                    self.generate_expression(&prop.value)?;
                }
                self.emit_no_indent("}");
            }

            Expression::PropertyAccess { object, property } => {
                // Special case for .length
                if property == "length" {
                    // Check if it's an array (DynamicArray*) or string
                    // For now, we'll check the variable type
                    let is_dynamic_array = if let Expression::Identifier(name) = object.as_ref() {
                        self.variables.get(name)
                            .map(|t| t.contains("DynamicArray"))
                            .unwrap_or(false)
                    } else {
                        false
                    };

                    if is_dynamic_array {
                        // Dynamic array length
                        self.generate_expression(object)?;
                        self.emit_no_indent("->length");
                    } else {
                        // String length
                        self.emit_no_indent("strlen(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(")");
                    }
                } else {
                    self.generate_expression(object)?;
                    self.emit_no_indent(".");
                    self.emit_no_indent(property);
                }
            }

            Expression::MethodCall { object, method, args } => {
                // Handle string methods
                match method.as_str() {
                    "indexOf" if args.len() == 1 => {
                        self.emit_no_indent("({char* __p = strstr(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent("); __p ? (int32_t)(__p - (char*)(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(")) : -1; })");
                    }
                    "slice" if args.len() >= 1 && args.len() <= 2 => {
                        self.emit_no_indent("({char* __s = (char*)(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(") + ");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent("; __s; })");
                    }
                    "compare" if args.len() == 1 => {
                        self.emit_no_indent("strcmp(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(")");
                    }
                    "toUpperCase" if args.is_empty() => {
                        self.emit_no_indent("str_to_upper(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(")");
                    }
                    "toLowerCase" if args.is_empty() => {
                        self.emit_no_indent("str_to_lower(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(")");
                    }
                    "trim" if args.is_empty() => {
                        self.emit_no_indent("str_trim(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(")");
                    }
                    "charAt" if args.len() == 1 => {
                        self.emit_no_indent("str_char_at(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(")");
                    }
                    "substring" if args.len() == 2 => {
                        self.emit_no_indent("str_substring(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[1])?;
                        self.emit_no_indent(")");
                    }
                    "concat" if args.len() == 1 => {
                        self.emit_no_indent("str_concat(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(")");
                    }
                    "replace" if args.len() == 2 => {
                        self.emit_no_indent("str_replace(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[1])?;
                        self.emit_no_indent(")");
                    }
                    "split" if args.len() == 1 => {
                        self.emit_no_indent("str_split(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(")");
                    }
                    // Array methods
                    "join" if args.len() == 1 => {
                        self.emit_no_indent("array_join_string(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(")");
                    }
                    "push" if args.len() == 1 => {
                        // For now, assume i32 arrays - needs type system improvement
                        self.emit_no_indent("array_push_i32(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(")");
                    }
                    "pop" if args.is_empty() => {
                        self.emit_no_indent("array_pop_i32(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(")");
                    }
                    "reverse" if args.is_empty() => {
                        self.emit_no_indent("array_reverse_i32(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(")");
                    }
                    "map" if args.len() == 1 => {
                        self.emit_no_indent("array_map_i32(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(")");
                    }
                    "filter" if args.len() == 1 => {
                        self.emit_no_indent("array_filter_i32(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(")");
                    }
                    "reduce" if args.len() == 2 => {
                        self.emit_no_indent("array_reduce_i32(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[1])?;
                        self.emit_no_indent(")");
                    }
                    "forEach" if args.len() == 1 => {
                        self.emit_no_indent("array_forEach_i32(");
                        self.generate_expression(object)?;
                        self.emit_no_indent(", ");
                        self.generate_expression(&args[0])?;
                        self.emit_no_indent(")");
                    }
                    _ => {
                        // Generic method call (for objects)
                        self.generate_expression(object)?;
                        self.emit_no_indent(".");
                        self.emit_no_indent(method);
                        self.emit_no_indent("(");
                        for (i, arg) in args.iter().enumerate() {
                            if i > 0 {
                                self.emit_no_indent(", ");
                            }
                            self.generate_expression(arg)?;
                        }
                        self.emit_no_indent(")");
                    }
                }
            }

            Expression::Match { expr, arms } => {
                use crate::ast::MatchPattern;

                // Generate match as a statement expression with switch
                self.emit_no_indent("({ int32_t __match_result; switch (");
                self.generate_expression(expr)?;
                self.emit_no_indent(") {");

                for (i, arm) in arms.iter().enumerate() {
                    match &arm.pattern {
                        MatchPattern::Literal(lit_expr) => {
                            self.emit_no_indent(" case ");
                            self.generate_expression(lit_expr)?;
                            self.emit_no_indent(": __match_result = ");
                            self.generate_expression(&arm.body)?;
                            self.emit_no_indent("; break;");
                        }
                        MatchPattern::Wildcard => {
                            self.emit_no_indent(" default: __match_result = ");
                            self.generate_expression(&arm.body)?;
                            self.emit_no_indent("; break;");
                        }
                    }
                }

                self.emit_no_indent(" } __match_result; })");
            }

            Expression::FunctionExpression { params, return_type, body } => {
                use std::collections::HashSet;

                // Generate a unique lambda function name
                let lambda_name = format!("__lambda_{}", self.lambda_counter);
                let context_name = format!("__context_{}", self.lambda_counter);
                self.lambda_counter += 1;

                // Detect captured variables
                let mut param_names = HashSet::new();
                for param in params {
                    param_names.insert(param.name.clone());
                }
                let captured_vars = body.find_free_variables(&param_names);

                // Build the function signature
                let ret_type = return_type
                    .as_ref()
                    .map(|t| self.type_to_c(t))
                    .unwrap_or_else(|| "void".to_string());

                let mut func_def = String::new();

                // Generate global variables for captured variables (simplified approach)
                if !captured_vars.is_empty() {
                    for var_name in &captured_vars {
                        func_def.push_str(&format!("int32_t __captured_{};\n", var_name));
                    }
                    func_def.push_str("\n");
                }

                func_def.push_str(&ret_type);
                func_def.push_str(" ");
                func_def.push_str(&lambda_name);
                func_def.push_str("(");

                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        func_def.push_str(", ");
                    }
                    func_def.push_str(&self.type_to_c(&param.param_type));
                    func_def.push_str(" ");
                    func_def.push_str(&param.name);
                }

                func_def.push_str(") {\n");

                // For captured variables, use #define to alias them to globals
                // This way assignments work correctly
                if !captured_vars.is_empty() {
                    for var_name in &captured_vars {
                        func_def.push_str(&format!("#define {} __captured_{}\n", var_name, var_name));
                    }
                }

                // Save current output and indent to generate body
                let saved_output = self.output.clone();
                let saved_indent = self.indent_level;
                self.output = String::new();
                self.indent_level = 1;

                // Generate the function body
                self.generate_block(body)?;

                // Capture generated body
                let body_code = self.output.clone();

                // Restore output and indent
                self.output = saved_output;
                self.indent_level = saved_indent;

                func_def.push_str(&body_code);

                // Undefine the captured variable aliases
                if !captured_vars.is_empty() {
                    for var_name in &captured_vars {
                        func_def.push_str(&format!("#undef {}\n", var_name));
                    }
                }

                func_def.push_str("}\n\n");

                self.lambda_functions.push(func_def);

                // Store lambda info for later use
                self.lambda_info.push(LambdaInfo {
                    name: lambda_name.clone(),
                    captured_vars: captured_vars.clone(),
                    context_struct: context_name.clone(),
                });

                // Set captured variables before emitting the function reference
                if !captured_vars.is_empty() {
                    self.emit_no_indent("({");
                    for var_name in &captured_vars {
                        self.emit_no_indent(&format!("__captured_{} = {}; ", var_name, var_name));
                    }
                    self.emit_no_indent(&format!("{}; ", lambda_name));
                    self.emit_no_indent("})");
                } else {
                    self.emit_no_indent(&lambda_name);
                }
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
            Type::Function { params, return_type } => {
                // Generate proper function pointer typedef
                // For simplicity, we'll use a generic function pointer signature
                // In C: int32_t (*name)(int32_t, int32_t)
                // But for variables, we can't include the name, so we use typedef
                // For now, simplified: assume all function pointers take (i32, i32) -> i32
                "void*".to_string()  // Will cast at call site
            },
            Type::Object => "void*".to_string(),
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
