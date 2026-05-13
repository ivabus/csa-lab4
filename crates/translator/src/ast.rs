#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I32,
    U32,
    I8,
    U8,
    Cstr,
    Bool,
}

#[derive(Debug, Clone)]
pub struct Path {
    pub segments: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    ModDecl(ModDecl),
    UseDecl(UseDecl),
    ConstDecl(ConstDecl),
    FnDecl(FnDecl),
}

#[derive(Debug, Clone)]
pub struct ModDecl {
    pub vis: bool,
    pub name: String,
    pub body: Option<Vec<Item>>,
}

#[derive(Debug, Clone)]
pub struct UseDecl {
    pub vis: bool,
    pub tree: UseTree,
}

#[derive(Debug, Clone)]
pub enum UseTree {
    Path(Path),
    Glob,
    Group(Vec<UseTree>),
    Nested(Path, Box<UseTree>),
}

#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub vis: bool,
    pub name: String,
    pub ty: Type,
    pub value: Expression,
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub vis: bool,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    LetDecl(LetDecl),
    IfStmt(IfStmt),
    WhileStmt(WhileStmt),
    Break,
    Continue,
    Return(Option<Expression>),
    Assign(AssignStmt),
    Expr(Expression),
}

#[derive(Debug, Clone)]
pub struct LetDecl {
    pub is_mut: bool,
    pub name: String,
    pub ty: Option<Type>,
    pub value: Expression,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Expression,
    pub then_block: Block,
    pub else_branch: Option<ElseBranch>,
}

#[derive(Debug, Clone)]
pub enum ElseBranch {
    Block(Block),
    If(Box<IfStmt>),
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub condition: Expression,
    pub block: Block,
}

#[derive(Debug, Clone)]
pub struct AssignStmt {
    pub target: AssignTarget,
    pub op: AssignOp,
    pub value: Expression,
}

#[derive(Debug, Clone)]
pub enum AssignTarget {
    Path(Path),
    Deref(Expression),
    Index(Path, Expression),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    AddEq,
    SubEq,
    MulEq,
    DivEq,
    ModEq,
    BitAndEq,
    BitOrEq,
    BitXorEq,
    ShlEq,
    ShrEq,
}

impl AssignOp {
    pub fn to_opcode(&self) -> Option<shared::Opcode> {
        match self {
            AssignOp::Assign => None,
            AssignOp::AddEq => Some(shared::Opcode::Add),
            AssignOp::SubEq => Some(shared::Opcode::Sub),
            AssignOp::MulEq => Some(shared::Opcode::Mul),
            AssignOp::DivEq => Some(shared::Opcode::Div),
            AssignOp::ModEq => Some(shared::Opcode::Mod),
            AssignOp::BitAndEq => Some(shared::Opcode::And),
            AssignOp::BitOrEq => Some(shared::Opcode::Or),
            AssignOp::BitXorEq => Some(shared::Opcode::Xor),
            AssignOp::ShlEq => Some(shared::Opcode::Ls),
            AssignOp::ShrEq => Some(shared::Opcode::Rs),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expression {
    Binary(BinaryOp, Box<Expression>, Box<Expression>),
    Unary(UnaryOp, Box<Expression>),
    Literal(Literal),
    Path(Path),
    Call(Path, Vec<Expression>),
    Index(Path, Box<Expression>),
    Reserve(Type, usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    LogicAnd,
    LogicOr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
    Deref,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i32),
    Uint(u32),
    Char(u8),
    String(String),
    Bool(bool),
}
