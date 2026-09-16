use std::fmt;

/// Backend-neutral type representation for MIR operands.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MirTy {
    Void,
    Bool,
    I8,
    I32,
    I64,
    F32,
    F64,
    Text,
    Bytes,
    Ptr(Box<MirTy>),
    Ref {
        mut_: bool,
        inner: Box<MirTy>,
    },
    Struct {
        name: String,
    },
    Enum {
        name: String,
    },
    Array {
        elem: Box<MirTy>,
        len: usize,
    },
    Slice {
        elem: Box<MirTy>,
    },
    FnPtr {
        params: Vec<MirTy>,
        ret: Box<MirTy>,
    },
}

impl MirTy {
    pub fn is_integer(&self) -> bool {
        matches!(self, MirTy::I8 | MirTy::I32 | MirTy::I64)
    }

    pub fn is_float(&self) -> bool {
        matches!(self, MirTy::F32 | MirTy::F64)
    }

    pub fn ptr_to(inner: MirTy) -> Self {
        MirTy::Ptr(Box::new(inner))
    }

    /// Stable suffix used for monomorphized symbol names.
    pub fn mangle(&self) -> String {
        match self {
            MirTy::Void => "void".into(),
            MirTy::Bool => "bool".into(),
            MirTy::I8 => "i8".into(),
            MirTy::I32 => "i32".into(),
            MirTy::I64 => "i64".into(),
            MirTy::F32 => "f32".into(),
            MirTy::F64 => "f64".into(),
            MirTy::Text => "text".into(),
            MirTy::Bytes => "bytes".into(),
            MirTy::Ptr(t) => format!("p{}", t.mangle()),
            MirTy::Ref { mut_, inner } => {
                format!("{}{}", if *mut_ { "rm" } else { "r" }, inner.mangle())
            }
            MirTy::Struct { name } | MirTy::Enum { name } => name.clone(),
            MirTy::Array { elem, len } => format!("a{len}{}", elem.mangle()),
            MirTy::Slice { elem } => format!("s{}", elem.mangle()),
            MirTy::FnPtr { params, ret } => {
                let ps: Vec<String> = params.iter().map(MirTy::mangle).collect();
                format!("fn{}_{}", ps.join(""), ret.mangle())
            }
        }
    }
}

impl fmt::Display for MirTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MirTy::Void => write!(f, "void"),
            MirTy::Bool => write!(f, "bool"),
            MirTy::I8 => write!(f, "i8"),
            MirTy::I32 => write!(f, "i32"),
            MirTy::I64 => write!(f, "i64"),
            MirTy::F32 => write!(f, "f32"),
            MirTy::F64 => write!(f, "f64"),
            MirTy::Text => write!(f, "text"),
            MirTy::Bytes => write!(f, "bytes"),
            MirTy::Ptr(t) => write!(f, "ptr<{t}>"),
            MirTy::Ref { mut_, inner } => {
                if *mut_ {
                    write!(f, "ref mut {inner}")
                } else {
                    write!(f, "ref {inner}")
                }
            }
            MirTy::Struct { name } => write!(f, "{name}"),
            MirTy::Enum { name } => write!(f, "enum {name}"),
            MirTy::Array { elem, len } => write!(f, "[{elem}; {len}]"),
            MirTy::Slice { elem } => write!(f, "{elem}[]"),
            MirTy::FnPtr { params, ret } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, ") -> {ret}")
            }
        }
    }
}
