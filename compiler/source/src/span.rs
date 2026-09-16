use std::fmt;

use crate::SourceFile;

/// Byte offset from the start of a source file (UTF-8).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BytePos(pub u32);

impl BytePos {
    pub fn offset(self, delta: u32) -> Self {
        Self(self.0.saturating_add(delta))
    }
}

/// One-based line and column (column in Unicode scalar values).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct LineCol {
    pub line: u32,
    pub column: u32,
}

/// A half-open byte range `[start, end)` into a [`SourceFile`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: BytePos,
    pub end: BytePos,
}

impl Span {
    pub const DUMMY: Self = Self {
        start: BytePos(0),
        end: BytePos(0),
    };

    pub fn new(start: BytePos, end: BytePos) -> Self {
        if start.0 <= end.0 {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }

    pub fn from_range(range: std::ops::Range<u32>) -> Self {
        Self::new(BytePos(range.start), BytePos(range.end))
    }

    pub fn merge(self, other: Span) -> Span {
        Span::new(
            BytePos(self.start.0.min(other.start.0)),
            BytePos(self.end.0.max(other.end.0)),
        )
    }

    pub fn contains(self, pos: BytePos) -> bool {
        self.start.0 <= pos.0 && pos.0 < self.end.0
    }

    pub fn is_empty(self) -> bool {
        self.start.0 >= self.end.0
    }

    pub fn len(self) -> u32 {
        self.end.0.saturating_sub(self.start.0)
    }

    pub fn to_line_col_range(self, file: &SourceFile) -> (LineCol, LineCol) {
        (file.line_col(self.start), file.line_col(self.end))
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start.0, self.end.0)
    }
}

/// Wrapper attaching a span to a value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            node: f(self.node),
            span: self.span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverted_span_does_not_panic() {
        let s = Span::new(BytePos(40), BytePos(10));
        assert!(s.start.0 <= s.end.0);
        assert_eq!(s.start.0, 10);
        assert_eq!(s.end.0, 40);
    }
}
