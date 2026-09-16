use buraaq_lexer::TokenKind;

/// Tokens that can begin a top-level declaration after an error.
pub fn is_top_level_sync(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Fn
            | TokenKind::Async
            | TokenKind::Struct
            | TokenKind::Enum
            | TokenKind::Trait
            | TokenKind::Impl
            | TokenKind::Const
            | TokenKind::Type
            | TokenKind::Extern
            | TokenKind::Test
            | TokenKind::Bench
            | TokenKind::Use
            | TokenKind::Module
            | TokenKind::Pub
            | TokenKind::Hash
            | TokenKind::Eof
    )
}

/// Tokens that can begin a statement inside a block.
pub fn is_stmt_sync(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Fn
            | TokenKind::Async
            | TokenKind::Struct
            | TokenKind::Enum
            | TokenKind::Trait
            | TokenKind::Impl
            | TokenKind::Const
            | TokenKind::Type
            | TokenKind::Extern
            | TokenKind::If
            | TokenKind::While
            | TokenKind::For
            | TokenKind::Parallel
            | TokenKind::Match
            | TokenKind::Return
            | TokenKind::Break
            | TokenKind::Continue
            | TokenKind::Defer
            | TokenKind::Unsafe
            | TokenKind::Expect
            | TokenKind::Mut
            | TokenKind::Ident(_)
            | TokenKind::SelfKw
            | TokenKind::LBrace
            | TokenKind::RBrace
            | TokenKind::Hash
            | TokenKind::Eof
    ) || kind.is_literal()
}

pub fn is_type_sync(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Ident(_)
            | TokenKind::Void
            | TokenKind::Fn
            | TokenKind::LParen
            | TokenKind::LBracket
    )
}
