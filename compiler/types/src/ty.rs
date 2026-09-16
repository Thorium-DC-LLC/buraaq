use std::fmt;

/// Interned type identifier.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ty(pub u32);

impl Ty {
    pub const ERROR: Ty = Ty(0);
    pub const VOID: Ty = Ty(1);
    pub const BOOL: Ty = Ty(2);
    pub const CHAR: Ty = Ty(3);
    pub const I32: Ty = Ty(4);
    pub const I64: Ty = Ty(5);
    pub const U32: Ty = Ty(6);
    pub const U64: Ty = Ty(7);
    pub const F32: Ty = Ty(8);
    pub const F64: Ty = Ty(9);
    pub const TEXT: Ty = Ty(10);
    pub const BYTES: Ty = Ty(11);
    pub const NEVER: Ty = Ty(12);
}

/// Definition id for named types, functions, and locals at module scope.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DefId(pub u32);

/// Inference variable for constraint solving.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InferVar(pub u32);

/// Region id for borrow/lifetime tracking (inferred, never surfaced in syntax).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Region(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum IntKind {
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FloatKind {
    F32,
    F64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TyKind {
    Error,
    Void,
    Bool,
    Char,
    Int(IntKind),
    Float(FloatKind),
    Text,
    Bytes,
    Never,
    /// User-defined or built-in generic type: Option[T], List[T], Point, etc.
    Named {
        def: DefId,
        args: Vec<Ty>,
    },
    Tuple(Vec<Ty>),
    Function {
        params: Vec<Ty>,
        ret: Ty,
        throws: Option<Ty>,
    },
    Slice(Ty),
    Array {
        elem: Ty,
        len: u64,
    },
    Ref {
        mut_: bool,
        inner: Ty,
        region: Region,
    },
    RawPtr {
        mut_: bool,
        inner: Ty,
    },
    Infer(InferVar),
    Union(Vec<Ty>),
}

impl fmt::Display for TyKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TyKind::Error => write!(f, "<error>"),
            TyKind::Void => write!(f, "void"),
            TyKind::Bool => write!(f, "bool"),
            TyKind::Char => write!(f, "char"),
            TyKind::Int(k) => write!(f, "{k:?}"),
            TyKind::Float(k) => write!(f, "{k:?}"),
            TyKind::Text => write!(f, "text"),
            TyKind::Bytes => write!(f, "bytes"),
            TyKind::Never => write!(f, "never"),
            TyKind::Named { def, args } => {
                write!(f, "DefId({}).", def.0)?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "T{a:?}")?;
                }
                Ok(())
            }
            TyKind::Tuple(ts) => {
                write!(f, "(")?;
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "T{t:?}")?;
                }
                write!(f, ")")
            }
            TyKind::Function { params, ret, throws } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "T{p:?}")?;
                }
                write!(f, ") -> T{ret:?}")?;
                if let Some(e) = throws {
                    write!(f, " throws T{e:?}")?;
                }
                Ok(())
            }
            TyKind::Slice(inner) => write!(f, "T{inner:?}[]"),
            TyKind::Array { elem, len } => write!(f, "[T{elem:?}; {len}]"),
            TyKind::Ref { mut_, inner, .. } => {
                if *mut_ {
                    write!(f, "ref mut T{inner:?}")
                } else {
                    write!(f, "ref T{inner:?}")
                }
            }
            TyKind::RawPtr { mut_, inner } => {
                if *mut_ {
                    write!(f, "*mut T{inner:?}")
                } else {
                    write!(f, "*T{inner:?}")
                }
            }
            TyKind::Infer(v) => write!(f, "?{}", v.0),
            TyKind::Union(ts) => {
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "T{t:?}")?;
                }
                Ok(())
            }
        }
    }
}

/// Type capabilities used by ownership and borrow checking.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct TypeFlags {
    pub is_copy: bool,
    pub is_drop: bool,
    pub is_unsafe: bool,
}

impl TypeFlags {
    pub const COPY_PRIMITIVE: Self = Self {
        is_copy: true,
        is_drop: false,
        is_unsafe: false,
    };

    pub const OWNED: Self = Self {
        is_copy: false,
        is_drop: true,
        is_unsafe: false,
    };

    pub const UNSAFE_PTR: Self = Self {
        is_copy: true,
        is_drop: false,
        is_unsafe: true,
    };
}
