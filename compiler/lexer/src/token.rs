use buraaq_source::{Span, Spanned};

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    // Keywords
    Async,
    Await,
    Break,
    Const,
    Continue,
    Copy,
    Defer,
    Drop,
    Else,
    Elif,
    Enum,
    Err,
    Extern,
    False,
    Fn,
    For,
    Give,
    If,
    Impl,
    In,
    Match,
    Module,
    Mut,
    New,
    None,
    Not,
    Ok,
    Parallel,
    Pub,
    Raise,
    Ref,
    Return,
    SelfKw,
    Some,
    Spawn,
    Struct,
    Throws,
    Trait,
    True,
    Type,
    Unsafe,
    Use,
    Void,
    While,
    As,
    Test,
    Bench,
    Expect,

    // Literals
    Ident(String),
    Int(i128),
    Float(f64),
    Char(char),
    String(StringLit),
    Bytes(Vec<u8>),

    // Operators and punctuation
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Bang,
    Question,
    QuestionQuestion,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Shl,
    Shr,
    Dot,
    DotDot,
    DotDotEq,
    DotDotDot,
    Arrow,
    FatArrow,
    Comma,
    Colon,
    Semicolon,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Hash,
    At,

    // Structural
    Eof,
    Invalid(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct StringLit {
    pub parts: Vec<StringPart>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StringPart {
    Text(String),
    /// Raw source slice for `${ ... }` — re-parsed by parser.
    InterpRaw(Spanned<String>),
}

impl TokenKind {
    pub fn golden_name(&self) -> &'static str {
        match self {
            TokenKind::Async => "Async",
            TokenKind::Await => "Await",
            TokenKind::Break => "Break",
            TokenKind::Const => "Const",
            TokenKind::Continue => "Continue",
            TokenKind::Copy => "Copy",
            TokenKind::Defer => "Defer",
            TokenKind::Drop => "Drop",
            TokenKind::Else => "Else",
            TokenKind::Elif => "Elif",
            TokenKind::Enum => "Enum",
            TokenKind::Err => "Err",
            TokenKind::Extern => "Extern",
            TokenKind::False => "False",
            TokenKind::Fn => "Fn",
            TokenKind::For => "For",
            TokenKind::Give => "Give",
            TokenKind::If => "If",
            TokenKind::Impl => "Impl",
            TokenKind::In => "In",
            TokenKind::Match => "Match",
            TokenKind::Module => "Module",
            TokenKind::Mut => "Mut",
            TokenKind::New => "New",
            TokenKind::None => "None",
            TokenKind::Not => "Not",
            TokenKind::Ok => "Ok",
            TokenKind::Parallel => "Parallel",
            TokenKind::Pub => "Pub",
            TokenKind::Raise => "Raise",
            TokenKind::Ref => "Ref",
            TokenKind::Return => "Return",
            TokenKind::SelfKw => "SelfKw",
            TokenKind::Some => "Some",
            TokenKind::Spawn => "Spawn",
            TokenKind::Struct => "Struct",
            TokenKind::Throws => "Throws",
            TokenKind::Trait => "Trait",
            TokenKind::True => "True",
            TokenKind::Type => "Type",
            TokenKind::Unsafe => "Unsafe",
            TokenKind::Use => "Use",
            TokenKind::Void => "Void",
            TokenKind::While => "While",
            TokenKind::As => "As",
            TokenKind::Test => "Test",
            TokenKind::Bench => "Bench",
            TokenKind::Expect => "Expect",
            TokenKind::Ident(_) => "Ident",
            TokenKind::Int(_) => "Int",
            TokenKind::Float(_) => "Float",
            TokenKind::Char(_) => "Char",
            TokenKind::String(_) => "String",
            TokenKind::Bytes(_) => "Bytes",
            TokenKind::Plus => "Plus",
            TokenKind::Minus => "Minus",
            TokenKind::Star => "Star",
            TokenKind::Slash => "Slash",
            TokenKind::Percent => "Percent",
            TokenKind::Eq => "Eq",
            TokenKind::EqEq => "EqEq",
            TokenKind::NotEq => "NotEq",
            TokenKind::Lt => "Lt",
            TokenKind::Le => "Le",
            TokenKind::Gt => "Gt",
            TokenKind::Ge => "Ge",
            TokenKind::AndAnd => "AndAnd",
            TokenKind::OrOr => "OrOr",
            TokenKind::Bang => "Bang",
            TokenKind::Question => "Question",
            TokenKind::QuestionQuestion => "QuestionQuestion",
            TokenKind::Amp => "Amp",
            TokenKind::Pipe => "Pipe",
            TokenKind::Caret => "Caret",
            TokenKind::Tilde => "Tilde",
            TokenKind::Shl => "Shl",
            TokenKind::Shr => "Shr",
            TokenKind::Dot => "Dot",
            TokenKind::DotDot => "DotDot",
            TokenKind::DotDotEq => "DotDotEq",
            TokenKind::DotDotDot => "DotDotDot",
            TokenKind::Arrow => "Arrow",
            TokenKind::FatArrow => "FatArrow",
            TokenKind::Comma => "Comma",
            TokenKind::Colon => "Colon",
            TokenKind::Semicolon => "Semicolon",
            TokenKind::LParen => "LParen",
            TokenKind::RParen => "RParen",
            TokenKind::LBracket => "LBracket",
            TokenKind::RBracket => "RBracket",
            TokenKind::LBrace => "LBrace",
            TokenKind::RBrace => "RBrace",
            TokenKind::Hash => "Hash",
            TokenKind::At => "At",
            TokenKind::Eof => "Eof",
            TokenKind::Invalid(_) => "Invalid",
        }
    }

    pub fn is_keyword(&self) -> bool {
        !matches!(self, TokenKind::Ident(_) | TokenKind::Eof | TokenKind::Invalid(_))
            && !self.is_literal()
            && !self.is_punct()
    }

    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            TokenKind::Int(_)
                | TokenKind::Float(_)
                | TokenKind::Char(_)
                | TokenKind::String(_)
                | TokenKind::Bytes(_)
                | TokenKind::True
                | TokenKind::False
                | TokenKind::None
        )
    }

    pub fn is_punct(&self) -> bool {
        matches!(
            self,
            TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent
                | TokenKind::Eq
                | TokenKind::EqEq
                | TokenKind::NotEq
                | TokenKind::Lt
                | TokenKind::Le
                | TokenKind::Gt
                | TokenKind::Ge
                | TokenKind::AndAnd
                | TokenKind::OrOr
                | TokenKind::Bang
                | TokenKind::Question
                | TokenKind::QuestionQuestion
                | TokenKind::Amp
                | TokenKind::Pipe
                | TokenKind::Caret
                | TokenKind::Shl
                | TokenKind::Shr
                | TokenKind::Dot
                | TokenKind::DotDot
                | TokenKind::DotDotEq
                | TokenKind::Arrow
                | TokenKind::FatArrow
                | TokenKind::Comma
                | TokenKind::Colon
                | TokenKind::Semicolon
                | TokenKind::LParen
                | TokenKind::RParen
                | TokenKind::LBracket
                | TokenKind::RBracket
                | TokenKind::LBrace
                | TokenKind::RBrace
                | TokenKind::Hash
                | TokenKind::At
        )
    }
}
