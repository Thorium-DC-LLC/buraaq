//! Buraaq Mid-level IR (MIR).
//!
//! Three-address CFG with explicit operations for codegen and future GFA passes.
//! Decoupled from any particular backend (LLVM, Cranelift, WASM, etc.).

mod drop;
mod gfa;
mod lower;
mod opt;
mod ty;
mod verify;
mod visit;

pub use drop::insert_drops;
pub use gfa::{check_module as check_gfa, GfaError};
pub use lower::{lower_program, lower_units, LowerError, LowerUnit};
pub use opt::{optimize, OptConfig, OptStats};
pub use ty::MirTy;
pub use verify::{verify, VerifyError};
pub use visit::MirVisitor;

pub type LocalId = u32;
pub type BlockId = u32;
pub type FuncId = u32;

/// A compiled module: types, globals, and functions.
#[derive(Clone, Debug, Default)]
pub struct MirModule {
    pub name: String,
    pub structs: Vec<MirStruct>,
    pub enums: Vec<MirEnum>,
    pub globals: Vec<MirGlobal>,
    pub functions: Vec<MirFunction>,
    pub externs: Vec<MirExtern>,
}

#[derive(Clone, Debug)]
pub struct MirExtern {
    pub name: String,
    pub params: Vec<MirTy>,
    pub ret: MirTy,
    pub variadic: bool,
}

#[derive(Clone, Debug)]
pub struct MirStruct {
    pub name: String,
    pub fields: Vec<(String, MirTy)>,
}

#[derive(Clone, Debug)]
pub struct MirEnum {
    pub name: String,
    pub variants: Vec<(String, Vec<MirTy>)>,
}

#[derive(Clone, Debug)]
pub struct MirGlobal {
    pub name: String,
    pub ty: MirTy,
    pub init: Constant,
}

#[derive(Clone, Debug)]
pub struct MirFunction {
    pub name: String,
    pub params: Vec<(String, MirTy)>,
    pub return_ty: MirTy,
    pub locals: Vec<MirLocal>,
    pub blocks: Vec<BasicBlock>,
    pub is_entry: bool,
}

#[derive(Clone, Debug)]
pub struct MirLocal {
    pub name: String,
    pub ty: MirTy,
    pub mutable: bool,
}

#[derive(Clone, Debug)]
pub struct BasicBlock {
    pub id: BlockId,
    pub stmts: Vec<Statement>,
    pub terminator: Terminator,
}

#[derive(Clone, Debug)]
pub enum Statement {
    /// `%dest = rvalue`
    Assign { dest: LocalId, rvalue: Rvalue },
    /// `store value, ty* ptr`
    Store {
        ptr: Operand,
        value: Operand,
        ty: MirTy,
    },
    /// Stack slot for aggregate / named local
    StorageLive { local: LocalId },
    StorageDead { local: LocalId },
}

#[derive(Clone, Debug)]
pub enum Rvalue {
    Use(Operand),
    Literal(Constant),
    Binary {
        op: BinOp,
        left: Operand,
        right: Operand,
    },
    Unary {
        op: UnOp,
        operand: Operand,
    },
    Call {
        func: String,
        args: Vec<Operand>,
        ret_ty: MirTy,
    },
    Aggregate {
        ty: MirTy,
        fields: Vec<Operand>,
    },
    Field {
        base: Operand,
        field_index: u32,
        ty: MirTy,
    },
    Index {
        base: Operand,
        index: Operand,
        elem_ty: MirTy,
    },
    Cast {
        operand: Operand,
        from: MirTy,
        to: MirTy,
    },
    HeapAlloc {
        elem_ty: MirTy,
        count: Operand,
    },
    Load {
        ptr: Operand,
        ty: MirTy,
    },
    AddrOf {
        local: LocalId,
        ty: MirTy,
    },
}

#[derive(Clone, Debug)]
pub enum Terminator {
    Return(Option<Operand>),
    Goto(BlockId),
    If {
        cond: Operand,
        then_bb: BlockId,
        else_bb: BlockId,
    },
    Switch {
        discr: Operand,
        arms: Vec<(i64, BlockId)>,
        otherwise: BlockId,
    },
    Unreachable,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Clone, Debug)]
pub enum Operand {
    Local(LocalId),
    Constant(Constant),
}

#[derive(Clone, Debug)]
pub enum Constant {
    Void,
    Bool(bool),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Str(String),
    NullPtr(MirTy),
    FnAddr(String),
}

impl MirModule {
    pub fn entry_function(&self) -> Option<&MirFunction> {
        self.functions.iter().find(|f| f.is_entry)
    }
}
