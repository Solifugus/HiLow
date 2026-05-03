use crate::lexer::Position;

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    High,
    Low,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveType {
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
    Usize,
    Isize,
    Nothing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Primitive(PrimitiveType),
    FixedArray(Box<Type>, usize),
    DynamicArray(Box<Type>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
    pub position: Position,
}

/// A function body placeholder that stores source positions for later parsing
#[derive(Debug, Clone, PartialEq)]
pub struct BodyPlaceholder {
    pub start_position: Position,
    pub end_position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub mode: Mode,           // Effective mode after inheritance
    pub params: Vec<Parameter>,
    pub return_type: Type,
    pub body: BodyPlaceholder,
    pub is_export: bool,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub mode: Mode,
    pub params: Vec<Parameter>,
    pub return_type: Type,
    pub body: BodyPlaceholder,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub mode: Mode,
    pub items: Vec<Function>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TopLevel {
    Program(Program),
    Module(Module),
}