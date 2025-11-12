use std::fmt;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub enum FStringPart {
    Text(String),
    Expression(Box<Expression>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    FunctionDecl {
        name: String,
        params: Vec<Parameter>,
        return_type: Option<Type>,
        body: Block,
    },
    VariableDecl {
        name: String,
        var_type: Option<Type>,
        initializer: Option<Expression>,
    },
    Return {
        value: Option<Expression>,
    },
    Expression(Expression),
    If {
        condition: Expression,
        then_branch: Block,
        else_branch: Option<Box<Statement>>,
    },
    While {
        condition: Expression,
        body: Block,
    },
    For {
        init: Option<Box<Statement>>,
        condition: Option<Expression>,
        increment: Option<Expression>,
        body: Block,
    },
    ForIn {
        variable: String,
        iterable: Expression,
        body: Block,
    },
    Break,
    Continue,
    Switch {
        expr: Expression,
        cases: Vec<SwitchCase>,
        default: Option<Block>,
    },
    Defer {
        statement: Box<Statement>,
    },
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub value: Expression,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub param_type: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    F32,
    F64,
    Bool,
    String,
    Nothing,
    Unknown,
    Array {
        element_type: Box<Type>,
        size: Option<usize>,
    },
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    Object,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    IntegerLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    FString {
        parts: Vec<FStringPart>,
    },
    BooleanLiteral(bool),
    Identifier(String),
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expression>,
    },
    Call {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },
    Assignment {
        target: Box<Expression>,
        value: Box<Expression>,
    },
    ArrayLiteral {
        elements: Vec<Expression>,
    },
    Index {
        array: Box<Expression>,
        index: Box<Expression>,
    },
    ObjectLiteral {
        properties: Vec<Property>,
    },
    PropertyAccess {
        object: Box<Expression>,
        property: String,
    },
    MethodCall {
        object: Box<Expression>,
        method: String,
        args: Vec<Expression>,
    },
    FunctionExpression {
        params: Vec<Parameter>,
        return_type: Option<Type>,
        body: Block,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub key: String,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Equal,              // ?=
    StrictEqual,        // ??=
    NotEqual,           // !=
    StrictNotEqual,     // !!=
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    // Logical
    And,
    Or,

    // Bitwise
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    ShiftLeft,
    ShiftRight,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Negate,     // -
    Not,        // not
    BitwiseNot, // ~
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::I8 => write!(f, "i8"),
            Type::I16 => write!(f, "i16"),
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::I128 => write!(f, "i128"),
            Type::U8 => write!(f, "u8"),
            Type::U16 => write!(f, "u16"),
            Type::U32 => write!(f, "u32"),
            Type::U64 => write!(f, "u64"),
            Type::U128 => write!(f, "u128"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            Type::Bool => write!(f, "bool"),
            Type::String => write!(f, "string"),
            Type::Nothing => write!(f, "nothing"),
            Type::Unknown => write!(f, "unknown"),
            Type::Array { element_type, size } => {
                if let Some(s) = size {
                    write!(f, "[{}; {}]", element_type, s)
                } else {
                    write!(f, "[{}]", element_type)
                }
            }
            Type::Function { params, return_type } => {
                write!(f, "function(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, "): {}", return_type)
            }
            Type::Object => write!(f, "object"),
        }
    }
}

// Helper functions for variable analysis
impl Expression {
    pub fn find_free_variables(&self, bound_vars: &HashSet<String>) -> HashSet<String> {
        let mut free = HashSet::new();

        match self {
            Expression::Identifier(name) => {
                if !bound_vars.contains(name) {
                    free.insert(name.clone());
                }
            }
            Expression::Binary { left, right, .. } => {
                free.extend(left.find_free_variables(bound_vars));
                free.extend(right.find_free_variables(bound_vars));
            }
            Expression::Unary { operand, .. } => {
                free.extend(operand.find_free_variables(bound_vars));
            }
            Expression::Call { callee, args } => {
                free.extend(callee.find_free_variables(bound_vars));
                for arg in args {
                    free.extend(arg.find_free_variables(bound_vars));
                }
            }
            Expression::Assignment { target, value } => {
                free.extend(target.find_free_variables(bound_vars));
                free.extend(value.find_free_variables(bound_vars));
            }
            Expression::Index { array, index } => {
                free.extend(array.find_free_variables(bound_vars));
                free.extend(index.find_free_variables(bound_vars));
            }
            Expression::PropertyAccess { object, .. } => {
                free.extend(object.find_free_variables(bound_vars));
            }
            Expression::MethodCall { object, args, .. } => {
                free.extend(object.find_free_variables(bound_vars));
                for arg in args {
                    free.extend(arg.find_free_variables(bound_vars));
                }
            }
            Expression::ArrayLiteral { elements } => {
                for elem in elements {
                    free.extend(elem.find_free_variables(bound_vars));
                }
            }
            Expression::ObjectLiteral { properties } => {
                for prop in properties {
                    free.extend(prop.value.find_free_variables(bound_vars));
                }
            }
            Expression::FString { parts } => {
                for part in parts {
                    if let FStringPart::Expression(expr) = part {
                        free.extend(expr.find_free_variables(bound_vars));
                    }
                }
            }
            Expression::FunctionExpression { params, body, .. } => {
                // Create new bound set including parameters
                let mut new_bound = bound_vars.clone();
                for param in params {
                    new_bound.insert(param.name.clone());
                }
                free.extend(body.find_free_variables(&new_bound));
            }
            _ => {}
        }

        free
    }
}

impl Block {
    pub fn find_free_variables(&self, bound_vars: &HashSet<String>) -> HashSet<String> {
        let mut free = HashSet::new();
        let mut local_bound = bound_vars.clone();

        for stmt in &self.statements {
            free.extend(stmt.find_free_variables(&local_bound));

            // Add variables declared in this block to bound set
            if let Statement::VariableDecl { name, .. } = stmt {
                local_bound.insert(name.clone());
            }
        }

        free
    }
}

impl Statement {
    pub fn find_free_variables(&self, bound_vars: &HashSet<String>) -> HashSet<String> {
        let mut free = HashSet::new();

        match self {
            Statement::VariableDecl { initializer, .. } => {
                if let Some(init) = initializer {
                    free.extend(init.find_free_variables(bound_vars));
                }
            }
            Statement::Return { value } => {
                if let Some(val) = value {
                    free.extend(val.find_free_variables(bound_vars));
                }
            }
            Statement::Expression(expr) => {
                free.extend(expr.find_free_variables(bound_vars));
            }
            Statement::If { condition, then_branch, else_branch } => {
                free.extend(condition.find_free_variables(bound_vars));
                free.extend(then_branch.find_free_variables(bound_vars));
                if let Some(else_stmt) = else_branch {
                    free.extend(else_stmt.find_free_variables(bound_vars));
                }
            }
            Statement::While { condition, body } => {
                free.extend(condition.find_free_variables(bound_vars));
                free.extend(body.find_free_variables(bound_vars));
            }
            Statement::For { init, condition, increment, body } => {
                let mut new_bound = bound_vars.clone();
                if let Some(init_stmt) = init {
                    if let Statement::VariableDecl { name, initializer, .. } = init_stmt.as_ref() {
                        if let Some(init_expr) = initializer {
                            free.extend(init_expr.find_free_variables(&new_bound));
                        }
                        new_bound.insert(name.clone());
                    }
                }
                if let Some(cond) = condition {
                    free.extend(cond.find_free_variables(&new_bound));
                }
                if let Some(inc) = increment {
                    free.extend(inc.find_free_variables(&new_bound));
                }
                free.extend(body.find_free_variables(&new_bound));
            }
            Statement::ForIn { variable, iterable, body } => {
                free.extend(iterable.find_free_variables(bound_vars));
                let mut new_bound = bound_vars.clone();
                new_bound.insert(variable.clone());
                free.extend(body.find_free_variables(&new_bound));
            }
            Statement::Switch { expr, cases, default } => {
                free.extend(expr.find_free_variables(bound_vars));
                for case in cases {
                    free.extend(case.value.find_free_variables(bound_vars));
                    free.extend(case.body.find_free_variables(bound_vars));
                }
                if let Some(def) = default {
                    free.extend(def.find_free_variables(bound_vars));
                }
            }
            Statement::Block(block) => {
                free.extend(block.find_free_variables(bound_vars));
            }
            _ => {}
        }

        free
    }
}
