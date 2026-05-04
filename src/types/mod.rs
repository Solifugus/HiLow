use crate::ast;
use crate::lexer::Position;

/// Type representation for the HiLow type system
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Integer types
    I8, I16, I32, I64, I128,
    U8, U16, U32, U64, U128,

    /// Floating point types
    F32, F64,

    /// Other primitive types
    Bool,
    String,
    Usize, Isize,

    /// Special types
    Nothing,

    /// Array types
    FixedArray(Box<Type>, usize),
    DynamicArray(Box<Type>),

    /// Object type (structural)
    Object(Vec<(String, Type)>), // properties and their types

    /// Function type
    Function(Vec<Type>, Box<Type>), // parameter types, return type

    /// Unknown type (for error recovery)
    Unknown,
}

impl Type {
    /// Check if this is a numeric type
    pub fn is_numeric(&self) -> bool {
        matches!(self,
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 |
            Type::F32 | Type::F64 | Type::Usize | Type::Isize
        )
    }

    /// Check if this is an integer type
    pub fn is_integer(&self) -> bool {
        matches!(self,
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 |
            Type::Usize | Type::Isize
        )
    }

    /// Check if this is a floating point type
    pub fn is_float(&self) -> bool {
        matches!(self, Type::F32 | Type::F64)
    }

    /// Check if this type can fit a given integer literal value
    pub fn can_fit_integer(&self, value: i64) -> bool {
        match self {
            Type::I8 => value >= i8::MIN as i64 && value <= i8::MAX as i64,
            Type::I16 => value >= i16::MIN as i64 && value <= i16::MAX as i64,
            Type::I32 => value >= i32::MIN as i64 && value <= i32::MAX as i64,
            Type::I64 => true, // i64 can fit any i64 value
            Type::I128 => true, // i128 can fit any i64 value
            Type::U8 => value >= 0 && value <= u8::MAX as i64,
            Type::U16 => value >= 0 && value <= u16::MAX as i64,
            Type::U32 => value >= 0 && value <= u32::MAX as i64,
            Type::U64 => value >= 0, // u64 can fit any non-negative i64 value
            Type::U128 => value >= 0, // u128 can fit any non-negative i64 value
            Type::Usize => value >= 0, // assume usize is at least as big as i64
            Type::Isize => true, // assume isize is at least as big as i64
            _ => false,
        }
    }

    /// Get the default type for an integer literal
    pub fn default_integer_type(value: i64) -> Type {
        if value >= i32::MIN as i64 && value <= i32::MAX as i64 {
            Type::I32
        } else {
            Type::I64
        }
    }

    /// Get the default type for a float literal
    pub fn default_float_type() -> Type {
        Type::F64
    }

    /// Convert from AST Type to type system Type
    pub fn from_ast_type(ast_type: &ast::Type) -> Type {
        match ast_type {
            ast::Type::Primitive(prim) => match prim {
                ast::PrimitiveType::I8 => Type::I8,
                ast::PrimitiveType::I16 => Type::I16,
                ast::PrimitiveType::I32 => Type::I32,
                ast::PrimitiveType::I64 => Type::I64,
                ast::PrimitiveType::I128 => Type::I128,
                ast::PrimitiveType::U8 => Type::U8,
                ast::PrimitiveType::U16 => Type::U16,
                ast::PrimitiveType::U32 => Type::U32,
                ast::PrimitiveType::U64 => Type::U64,
                ast::PrimitiveType::U128 => Type::U128,
                ast::PrimitiveType::F32 => Type::F32,
                ast::PrimitiveType::F64 => Type::F64,
                ast::PrimitiveType::Bool => Type::Bool,
                ast::PrimitiveType::String => Type::String,
                ast::PrimitiveType::Usize => Type::Usize,
                ast::PrimitiveType::Isize => Type::Isize,
                ast::PrimitiveType::Nothing => Type::Nothing,
            },
            ast::Type::FixedArray(element_type, size) => {
                Type::FixedArray(Box::new(Type::from_ast_type(element_type)), *size)
            },
            ast::Type::DynamicArray(element_type) => {
                Type::DynamicArray(Box::new(Type::from_ast_type(element_type)))
            },
            ast::Type::Object(properties) => {
                Type::Object(
                    properties
                        .iter()
                        .map(|(name, ty)| (name.clone(), Type::from_ast_type(ty)))
                        .collect()
                )
            },
            ast::Type::Function(param_types, return_type) => {
                Type::Function(
                    param_types.iter().map(|t| Type::from_ast_type(t)).collect(),
                    Box::new(Type::from_ast_type(return_type))
                )
            },
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
            Type::Usize => write!(f, "usize"),
            Type::Isize => write!(f, "isize"),
            Type::Nothing => write!(f, "nothing"),
            Type::FixedArray(elem, size) => write!(f, "[{}; {}]", elem, size),
            Type::DynamicArray(elem) => write!(f, "[{}]", elem),
            Type::Object(properties) => {
                write!(f, "{{")?;
                for (i, (name, ty)) in properties.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", name, ty)?;
                }
                write!(f, "}}")
            },
            Type::Function(param_types, return_type) => {
                write!(f, "function(")?;
                for (i, param_type) in param_types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param_type)?;
                }
                write!(f, "): {}", return_type)
            },
            Type::Unknown => write!(f, "<unknown>"),
        }
    }
}

/// Represents a type error during type checking
#[derive(Debug, Clone, PartialEq)]
pub struct TypeError {
    pub message: String,
    pub position: Position,
}

impl TypeError {
    pub fn new(message: impl Into<String>, position: Position) -> Self {
        TypeError {
            message: message.into(),
            position,
        }
    }
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Type error at line {}, column {}: {}",
               self.position.line, self.position.column, self.message)
    }
}

impl std::error::Error for TypeError {}