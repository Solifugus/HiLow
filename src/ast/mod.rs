use crate::lexer::Position;
use std::cell::RefCell;

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
    Object(Vec<(String, Type)>), // structural object type: properties and their types
    Function(Vec<Type>, Box<Type>), // parameter types, return type
    Unknown, // placeholder for unknown types
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
    pub position: Position,
}


#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOpKind {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // Comparison
    Eq,        // ?=
    NotEq,     // !=
    NotLess,   // !<
    NotGreater, // !>
    Less,      // <
    Greater,   // >
    LessEq,    // <=
    GreaterEq, // >=

    // Logical
    And,       // and
    Or,        // or

    // Bitwise
    BitAnd,    // &
    BitOr,     // |
    BitXor,    // ^
    ShiftLeft, // <<
    ShiftRight, // >>

    // Type checking
    Is,        // is
    IsNot,     // is not
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOpKind {
    Neg,    // unary -
    Not,    // not
    BitNot, // ~
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignOpKind {
    Assign,    // =
    AddAssign, // +=
    SubAssign, // -=
    MulAssign, // *=
    DivAssign, // /=
    ModAssign, // %=
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryOp {
    pub lhs: Box<Expression>,
    pub op: BinaryOpKind,
    pub rhs: Box<Expression>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnaryOp {
    pub op: UnaryOpKind,
    pub operand: Box<Expression>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub callee: Box<Expression>,
    pub args: Vec<Expression>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemberAccess {
    pub object: Box<Expression>,
    pub member: String,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexAccess {
    pub object: Box<Expression>,
    pub index: Box<Expression>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IsCheck {
    pub expression: Box<Expression>,
    pub ty: Type,
    pub negated: bool, // true for "is not"
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectIsCheck {
    pub lhs: Box<Expression>,  // child object
    pub rhs: Box<Expression>,  // parent object
    pub negated: bool,         // true for "is not"
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QualifierSpec {
    pub name: String,
    pub arg: Option<Expression>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QualifiedOpKind {
    Assign,
    Eq,
    NotEq,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedOp {
    pub lhs: Box<Expression>,
    pub qualifiers: Vec<QualifierSpec>,
    pub op: QualifiedOpKind,
    pub rhs: Box<Expression>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Align {
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormatSpec {
    pub fill: Option<char>,
    pub align: Option<Align>,
    pub width: Option<u32>,
    pub precision: Option<u32>,
    pub type_code: Option<char>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FStringPart {
    Text(String),
    Expression(Expression, Option<FormatSpec>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FString {
    pub parts: Vec<FStringPart>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectLiteral {
    pub properties: Vec<(String, Expression)>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionExpr {
    pub params: Vec<Parameter>,
    pub return_type: Type,
    pub body: Block,
    pub position: Position,
    pub captures: RefCell<Vec<(String, Type, Position)>>,  // name, ast::Type, first-ref position
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchPattern {
    Literal(Literal),
    Wildcard,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchBody {
    Expression(Expression),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: MatchBody,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchExpr {
    pub value: Box<Expression>,
    pub arms: Vec<MatchArm>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    IntLit(i64, Position),
    FloatLit(f64, Position),
    StringLit(String, Position),
    FString(FString),
    BoolLit(bool, Position),
    Ident(String, Position),
    This(Position),
    BinaryOp(BinaryOp),
    UnaryOp(UnaryOp),
    Call(Call),
    MemberAccess(MemberAccess),
    IndexAccess(IndexAccess),
    IsCheck(IsCheck),
    ObjectIsCheck(ObjectIsCheck),
    QualifiedOp(QualifiedOp),
    ObjectLiteral(ObjectLiteral),
    FunctionExpr(FunctionExpr),
    Match(MatchExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockItem {
    Statement(Statement),
    Function(Function),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProgramBody {
    pub items: Vec<BlockItem>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LetDecl {
    pub name: String,
    pub ty: Option<Type>,
    pub initializer: Option<Expression>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub condition: Expression,
    pub then_block: Block,
    pub else_block: Option<Block>, // For else { ... }
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileStmt {
    pub condition: Expression,
    pub body: Block,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoopStmt {
    pub body: Block,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssignStmt {
    pub target: Expression, // identifier or member access
    pub op: AssignOpKind,
    pub value: Expression,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForInStmt {
    pub key_name: String,     // The key variable name
    pub value_name: String,   // The value variable name
    pub iterable: Expression, // The object being iterated over
    pub body: Block,         // Loop body
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt {
    pub value: Option<Expression>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchStmt {
    pub value: Expression,
    pub cases: Vec<SwitchCase>,
    pub default: Option<Vec<Statement>>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub pattern: Literal,
    pub body: Vec<Statement>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let(LetDecl),
    Return(ReturnStmt),
    If(IfStmt),
    While(WhileStmt),
    Loop(LoopStmt),
    ForIn(ForInStmt),
    Switch(SwitchStmt),
    Break(Position),
    Continue(Position),
    Assign(AssignStmt),
    QualifiedOp(QualifiedOp), // For qualified assignment operations
    ExprStatement(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub mode: Mode,           // Effective mode after inheritance
    pub params: Vec<Parameter>,
    pub return_type: Type,
    pub body: Option<Block>,  // Parsed body (None in Phase 2a, Some in Phase 2b)
    pub is_export: bool,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub mode: Mode,
    pub params: Vec<Parameter>,
    pub return_type: Type,
    pub body: Option<ProgramBody>, // Parsed body with nested functions support
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