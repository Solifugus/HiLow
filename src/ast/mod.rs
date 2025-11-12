use std::fmt;

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
