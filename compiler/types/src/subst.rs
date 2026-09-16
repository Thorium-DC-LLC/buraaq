use crate::interner::TypeInterner;
use crate::ty::*;

/// Simple type substitution for generic instantiation.
#[derive(Clone, Debug, Default)]
pub struct Subst {
    pairs: Vec<(DefId, Vec<Ty>)>,
}

impl Subst {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, def: DefId, args: Vec<Ty>) {
        self.pairs.push((def, args));
    }

    pub fn apply(&self, ty: Ty, interner: &mut TypeInterner) -> Ty {
        match interner.kind(ty).clone() {
            TyKind::Named { def, args } => {
                for (d, subst_args) in &self.pairs {
                    if *d == def {
                        return interner.intern(TyKind::Named {
                            def,
                            args: subst_args.clone(),
                        });
                    }
                }
                let new_args: Vec<Ty> = args
                    .into_iter()
                    .map(|a| self.apply(a, interner))
                    .collect();
                interner.intern(TyKind::Named { def, args: new_args })
            }
            TyKind::Tuple(ts) => {
                let ts: Vec<Ty> = ts.into_iter().map(|t| self.apply(t, interner)).collect();
                interner.intern(TyKind::Tuple(ts))
            }
            TyKind::Slice(inner) => {
                let inner = self.apply(inner, interner);
                interner.intern(TyKind::Slice(inner))
            }
            TyKind::Array { elem, len } => {
                let elem = self.apply(elem, interner);
                interner.intern(TyKind::Array { elem, len })
            }
            TyKind::Ref { mut_, inner, region } => {
                let inner = self.apply(inner, interner);
                interner.intern(TyKind::Ref {
                    mut_: mut_,
                    inner,
                    region,
                })
            }
            TyKind::Function { params, ret, throws } => {
                let params: Vec<Ty> = params.into_iter().map(|p| self.apply(p, interner)).collect();
                let ret = self.apply(ret, interner);
                let throws = throws.map(|t| self.apply(t, interner));
                interner.intern(TyKind::Function {
                    params,
                    ret,
                    throws,
                })
            }
            other => interner.intern(other),
        }
    }
}
