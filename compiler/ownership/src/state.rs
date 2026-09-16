use buraaq_source::Span;
use buraaq_types::Ty;

/// Local binding identifier within a function body.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);

/// A memory place that can be owned or borrowed.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Place {
    pub local: LocalId,
    pub projection: u32,
}

impl Place {
    pub fn root(local: LocalId) -> Self {
        Self { local, projection: 0 }
    }
}

/// Definite-initialization state for a local binding.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InitState {
    /// Declared but not yet assigned.
    Uninitialized,
    /// Fully initialized and usable.
    Valid,
    /// Moved out; must not be used unless reinitialized.
    Moved,
    /// Partially moved (e.g. field extracted from struct).
    PartiallyMoved,
}

#[derive(Clone, Debug)]
pub struct LocalState {
    pub name: String,
    pub ty: Ty,
    pub mutable: bool,
    pub init: InitState,
    /// Where ownership was transferred away (for rich diagnostics).
    pub moved_at: Option<Span>,
    pub move_note: Option<String>,
}

impl LocalState {
    pub fn new(name: String, ty: Ty, mutable: bool) -> Self {
        Self {
            name,
            ty,
            mutable,
            init: InitState::Uninitialized,
            moved_at: None,
            move_note: None,
        }
    }

    pub fn is_usable(&self) -> bool {
        matches!(self.init, InitState::Valid | InitState::PartiallyMoved)
    }
}
