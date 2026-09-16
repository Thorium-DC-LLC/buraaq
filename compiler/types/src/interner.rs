use std::collections::HashMap;

use crate::ty::*;

/// Arena-backed type interner.
#[derive(Debug, Default)]
pub struct TypeInterner {
    kinds: Vec<TyKind>,
    flags: Vec<TypeFlags>,
    infer: Vec<Option<Ty>>,
    next_infer: u32,
    cache: HashMap<TyKind, Ty>,
}

impl TypeInterner {
    pub fn new() -> Self {
        let mut t = Self::default();
        t.intern_kind(TyKind::Error, TypeFlags::OWNED);
        t.intern_kind(TyKind::Void, TypeFlags::COPY_PRIMITIVE);
        t.intern_kind(TyKind::Bool, TypeFlags::COPY_PRIMITIVE);
        t.intern_kind(TyKind::Char, TypeFlags::COPY_PRIMITIVE);
        t.intern_kind(TyKind::Int(IntKind::I32), TypeFlags::COPY_PRIMITIVE);
        t.intern_kind(TyKind::Int(IntKind::I64), TypeFlags::COPY_PRIMITIVE);
        t.intern_kind(TyKind::Int(IntKind::U32), TypeFlags::COPY_PRIMITIVE);
        t.intern_kind(TyKind::Int(IntKind::U64), TypeFlags::COPY_PRIMITIVE);
        t.intern_kind(TyKind::Float(FloatKind::F32), TypeFlags::COPY_PRIMITIVE);
        t.intern_kind(TyKind::Float(FloatKind::F64), TypeFlags::COPY_PRIMITIVE);
        t.intern_kind(TyKind::Text, TypeFlags::COPY_PRIMITIVE);
        t.intern_kind(TyKind::Bytes, TypeFlags::OWNED);
        t.intern_kind(TyKind::Never, TypeFlags::COPY_PRIMITIVE);
        t
    }

    pub fn intern(&mut self, kind: TyKind) -> Ty {
        if let Some(id) = self.cache.get(&kind) {
            return *id;
        }
        let flags = default_flags(&kind);
        self.intern_kind(kind, flags)
    }

    fn intern_kind(&mut self, kind: TyKind, flags: TypeFlags) -> Ty {
        let id = Ty(self.kinds.len() as u32);
        self.cache.insert(kind.clone(), id);
        self.kinds.push(kind);
        self.flags.push(flags);
        id
    }

    pub fn fresh_infer(&mut self) -> Ty {
        let var = InferVar(self.next_infer);
        self.next_infer += 1;
        self.infer.push(None);
        self.intern(TyKind::Infer(var))
    }

    pub fn kind(&self, ty: Ty) -> &TyKind {
        &self.kinds[ty.0 as usize]
    }

    pub fn flags(&self, ty: Ty) -> TypeFlags {
        self.flags[ty.0 as usize]
    }

    pub fn is_copy(&self, ty: Ty) -> bool {
        self.resolve(ty)
            .map(|t| self.flags(t).is_copy)
            .unwrap_or(true)
    }

    pub fn resolve(&self, ty: Ty) -> Option<Ty> {
        match self.kind(ty) {
            TyKind::Infer(v) => self.infer.get(v.0 as usize)?.map(|t| self.resolve(t).unwrap_or(t)),
            _ => Some(ty),
        }
    }

    pub fn unify_infer(&mut self, var: InferVar, ty: Ty) -> bool {
        if let Some(idx) = self.infer.get_mut(var.0 as usize) {
            if let Some(existing) = *idx {
                return existing == ty;
            }
            *idx = Some(ty);
            true
        } else {
            false
        }
    }

    pub fn infer_var(&self, ty: Ty) -> Option<InferVar> {
        match self.kind(ty) {
            TyKind::Infer(v) => Some(*v),
            _ => None,
        }
    }
}

fn default_flags(kind: &TyKind) -> TypeFlags {
    match kind {
        TyKind::Error | TyKind::Infer(_) => TypeFlags::COPY_PRIMITIVE,
        TyKind::Void | TyKind::Bool | TyKind::Char | TyKind::Never => TypeFlags::COPY_PRIMITIVE,
        TyKind::Int(_) | TyKind::Float(_) => TypeFlags::COPY_PRIMITIVE,
        TyKind::Text => TypeFlags::COPY_PRIMITIVE,
        TyKind::Bytes => TypeFlags::OWNED,
        TyKind::RawPtr { .. } => TypeFlags::UNSAFE_PTR,
        TyKind::Ref { .. } => TypeFlags {
            is_copy: true,
            is_drop: false,
            is_unsafe: false,
        },
        _ => TypeFlags::OWNED,
    }
}

/// Read-only view over an interner.
pub struct TypeTable<'a> {
    pub interner: &'a TypeInterner,
}

impl<'a> TypeTable<'a> {
    pub fn new(interner: &'a TypeInterner) -> Self {
        Self { interner }
    }

    pub fn kind(&self, ty: Ty) -> &TyKind {
        self.interner.kind(ty)
    }

    pub fn is_copy(&self, ty: Ty) -> bool {
        self.interner.is_copy(ty)
    }
}
