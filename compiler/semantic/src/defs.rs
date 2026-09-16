use buraaq_ast::{Function, ImplDef, Item, Program, StructDef, TraitDef, TypeAlias};
use buraaq_types::DefId;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DefKind {
    Function,
    Struct,
    Enum,
    Trait,
    TypeAlias,
    Const,
    ExternFn,
    ImplMethod,
    Builtin,
}

#[derive(Clone, Debug)]
pub struct DefMapEntry {
    pub name: String,
    pub kind: DefKind,
    pub def: DefId,
    pub imported: bool,
}

/// Top-level definitions collected from a program.
#[derive(Clone, Debug, Default)]
pub struct DefMap {
    entries: Vec<DefMapEntry>,
    by_name: std::collections::HashMap<String, DefId>,
}

impl DefMap {
    pub fn collect(program: &Program) -> Self {
        let mut map = Self::default();
        for name in [
            "print",
            "println",
            "print_int",
            "print_float",
            "print_bool",
            "Some",
            "None",
            "Ok",
            "Err",
        ] {
            map.insert_name(name, DefKind::Builtin);
        }
        for item in &program.items {
            match &item.node {
                Item::Function(f) => {
                    map.insert_fn(&f.node);
                }
                Item::Struct(s) => {
                    map.insert_struct(&s.node);
                }
                Item::Enum(e) => {
                    map.insert_name(&e.node.name.node, DefKind::Enum);
                }
                Item::Trait(t) => {
                    map.insert_trait(&t.node);
                }
                Item::TypeAlias(t) => {
                    map.insert_alias(&t.node);
                }
                Item::Const(c) => {
                    map.insert_name(&c.node.name.node, DefKind::Const);
                }
                Item::Impl(i) => {
                    map.insert_impl(&i.node);
                }
                Item::Extern(e) => {
                    for f in &e.node.functions {
                        map.insert_name(&f.node.name.node, DefKind::ExternFn);
                    }
                }
                Item::Test(_) | Item::Bench(_) => {}
            }
        }
        map
    }

    fn insert_name(&mut self, name: &str, kind: DefKind) -> DefId {
        let id = DefId(self.entries.len() as u32);
        self.entries.push(DefMapEntry {
            name: name.to_string(),
            kind,
            def: id,
            imported: false,
        });
        self.by_name.insert(name.to_string(), id);
        id
    }

    fn insert_fn(&mut self, f: &Function) {
        self.insert_name(&f.name.node, DefKind::Function);
    }

    fn insert_struct(&mut self, s: &StructDef) {
        self.insert_name(&s.name.node, DefKind::Struct);
    }

    fn insert_trait(&mut self, t: &TraitDef) {
        self.insert_name(&t.name.node, DefKind::Trait);
    }

    fn insert_alias(&mut self, t: &TypeAlias) {
        self.insert_name(&t.name.node, DefKind::TypeAlias);
    }

    fn insert_impl(&mut self, i: &ImplDef) {
        for m in &i.methods {
            if let buraaq_ast::Method::Function(f) = &m.node {
                self.insert_name(&f.node.name.node, DefKind::ImplMethod);
            }
        }
    }

    pub fn resolve(&self, name: &str) -> Option<DefId> {
        self.by_name.get(name).copied()
    }

    pub fn lookup(&self, name: &str) -> Option<&DefMapEntry> {
        self.resolve(name).map(|id| self.entry(id))
    }

    pub fn entries(&self) -> &[DefMapEntry] {
        &self.entries
    }

    pub fn entry(&self, id: DefId) -> &DefMapEntry {
        &self.entries[id.0 as usize]
    }

    pub fn insert_imported(&mut self, name: &str, kind: DefKind) {
        if self.by_name.contains_key(name) {
            return;
        }
        let id = DefId(self.entries.len() as u32);
        self.entries.push(DefMapEntry {
            name: name.to_string(),
            kind,
            def: id,
            imported: true,
        });
        self.by_name.insert(name.to_string(), id);
    }
}
