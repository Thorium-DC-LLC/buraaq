use buraaq_ownership::LocalId;
use buraaq_types::Ty;

/// Lexical scope stack mapping names to locals or defs.
#[derive(Debug, Default, Clone)]
pub struct ScopeStack {
    scopes: Vec<ScopeFrame>,
}

#[derive(Debug, Default, Clone)]
struct ScopeFrame {
    bindings: std::collections::HashMap<String, Binding>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Binding {
    Local(LocalId),
    Param(LocalId),
}

impl ScopeStack {
    pub fn new() -> Self {
        Self {
            scopes: vec![ScopeFrame::default()],
        }
    }

    pub fn push(&mut self) {
        self.scopes.push(ScopeFrame::default());
    }

    pub fn pop(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn declare(&mut self, name: String, binding: Binding) {
        if let Some(frame) = self.scopes.last_mut() {
            frame.bindings.insert(name, binding);
        }
    }

    pub fn lookup(&self, name: &str) -> Option<Binding> {
        for frame in self.scopes.iter().rev() {
            if let Some(b) = frame.bindings.get(name) {
                return Some(*b);
            }
        }
        None
    }

    pub fn depth(&self) -> usize {
        self.scopes.len()
    }
}

/// Typed expression result from checking.
#[derive(Copy, Clone, Debug)]
pub struct TypedExpr {
    pub ty: Ty,
}
