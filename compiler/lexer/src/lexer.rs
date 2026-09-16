use buraaq_diagnostics::{Diagnostic, DiagnosticHandler, Help, Label, Suggestion};
use buraaq_source::{BytePos, SourceFile, Span, Spanned};

use crate::keywords::lookup;
use crate::{StringPart, Token, TokenKind};

pub struct Lexer<'a> {
    file: &'a SourceFile,
    diagnostics: Option<&'a dyn DiagnosticHandler>,
    chars: Vec<char>,
    pos: usize,
    offset: u32,
}

impl<'a> Lexer<'a> {
    pub fn new(file: &'a SourceFile) -> Self {
        Self {
            file,
            diagnostics: None,
            chars: file.text.chars().collect(),
            pos: 0,
            offset: 0,
        }
    }

    pub fn with_diagnostics(mut self, handler: &'a dyn DiagnosticHandler) -> Self {
        self.diagnostics = Some(handler);
        self
    }

    pub fn tokenize(mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let done = matches!(token.kind, TokenKind::Eof);
            tokens.push(token);
            if done {
                break;
            }
        }
        tokens
    }

    fn next_token(&mut self) -> Token {
        self.skip_trivia();
        let start = self.offset;
        if self.is_at_end() {
            return self.make_token(TokenKind::Eof, start);
        }

        let c = self.peek();
        let kind = match c {
            '(' => {
                self.advance();
                TokenKind::LParen
            }
            ')' => {
                self.advance();
                TokenKind::RParen
            }
            '{' => {
                self.advance();
                TokenKind::LBrace
            }
            '}' => {
                self.advance();
                TokenKind::RBrace
            }
            '[' => {
                self.advance();
                TokenKind::LBracket
            }
            ']' => {
                self.advance();
                TokenKind::RBracket
            }
            ',' => {
                self.advance();
                TokenKind::Comma
            }
            ';' => {
                self.advance();
                TokenKind::Semicolon
            }
            ':' => {
                self.advance();
                TokenKind::Colon
            }
            '+' => {
                self.advance();
                TokenKind::Plus
            }
            '*' => {
                self.advance();
                TokenKind::Star
            }
            '%' => {
                self.advance();
                TokenKind::Percent
            }
            '^' => {
                self.advance();
                TokenKind::Caret
            }
            '~' => {
                self.advance();
                TokenKind::Tilde
            }
            '@' => {
                self.advance();
                TokenKind::At
            }
            '#' => {
                self.advance();
                TokenKind::Hash
            }
            '-' => {
                self.advance();
                if self.match_char('>') {
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            '.' => self.lex_dot(),
            '=' => {
                self.advance();
                if self.match_char('=') {
                    TokenKind::EqEq
                } else if self.match_char('>') {
                    TokenKind::FatArrow
                } else {
                    TokenKind::Eq
                }
            }
            '!' => {
                self.advance();
                if self.match_char('=') {
                    TokenKind::NotEq
                } else {
                    TokenKind::Bang
                }
            }
            '<' => {
                self.advance();
                if self.match_char('=') {
                    TokenKind::Le
                } else if self.match_char('<') {
                    TokenKind::Shl
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                self.advance();
                if self.match_char('=') {
                    TokenKind::Ge
                } else if self.match_char('>') {
                    TokenKind::Shr
                } else {
                    TokenKind::Gt
                }
            }
            '&' => {
                self.advance();
                if self.match_char('&') {
                    TokenKind::AndAnd
                } else {
                    TokenKind::Amp
                }
            }
            '|' => {
                self.advance();
                if self.match_char('|') {
                    TokenKind::OrOr
                } else {
                    TokenKind::Pipe
                }
            }
            '?' => {
                self.advance();
                if self.match_char('?') {
                    TokenKind::QuestionQuestion
                } else {
                    TokenKind::Question
                }
            }
            '/' => {
                self.advance();
                TokenKind::Slash
            }
            '"' => return self.lex_string(start),
            '\'' => return self.lex_char(start),
            'b' if self.peek_next() == Some('"') => return self.lex_bytes(start),
            c if is_ident_start(c) => self.lex_ident_or_keyword(),
            c if c.is_ascii_digit() => self.lex_number(),
            c => {
                let msg = format!("unexpected character `{c}`");
                self.report_error(start, msg.clone(), "invalid token in source");
                self.advance();
                TokenKind::Invalid(msg)
            }
        };

        self.make_token(kind, start)
    }

    fn lex_dot(&mut self) -> TokenKind {
        self.advance();
        if self.match_char('.') {
            if self.match_char('=') {
                TokenKind::DotDotEq
            } else if self.match_char('.') {
                TokenKind::DotDotDot
            } else {
                TokenKind::DotDot
            }
        } else {
            TokenKind::Dot
        }
    }

    fn lex_ident_or_keyword(&mut self) -> TokenKind {
        let start = self.offset;
        while !self.is_at_end() && is_ident_continue(self.peek()) {
            self.advance();
        }
        let text: String = self.chars[self.byte_index(start)..self.byte_index(self.offset)]
            .iter()
            .collect();
        lookup(&text)
    }

    fn lex_number(&mut self) -> TokenKind {
        let start = self.offset;
        if self.peek() == '0' {
            self.advance();
            match self.peek() {
                'x' | 'X' => {
                    self.advance();
                    return self.lex_radix_int(start, 16, |c| c.is_ascii_hexdigit());
                }
                'b' | 'B' => {
                    self.advance();
                    return self.lex_radix_int(start, 2, |c| c == '0' || c == '1');
                }
                _ => {}
            }
        }

        while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
            self.advance();
        }

        if self.peek() == '.' && self.peek_next().is_some_and(|c| c.is_ascii_digit()) {
            self.advance();
            while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
                self.advance();
            }
            return self.parse_float(start);
        }

        if matches!(self.peek(), 'e' | 'E') {
            self.lex_exponent();
            return self.parse_float(start);
        }

        self.parse_int(start)
    }

    fn lex_exponent(&mut self) {
        if matches!(self.peek(), 'e' | 'E') {
            self.advance();
            if matches!(self.peek(), '+' | '-') {
                self.advance();
            }
            while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
                self.advance();
            }
        }
    }

    fn lex_radix_int(&mut self, start: u32, radix: u32, valid: impl Fn(char) -> bool) -> TokenKind {
        let digit_start = self.offset;
        while !self.is_at_end() && (valid(self.peek()) || self.peek() == '_') {
            self.advance();
        }
        if digit_start == self.offset {
            self.report_error(start, "expected digits after radix prefix", "integer literal is incomplete");
            return TokenKind::Int(0);
        }
        self.parse_int(start)
    }

    fn parse_int(&self, start: u32) -> TokenKind {
        let text: String = self.slice(start, self.offset).chars().filter(|c| *c != '_').collect();
        let value = if start as usize + 1 < self.file.text.len()
            && self.file.text.as_bytes().get(start as usize) == Some(&b'0')
            && matches!(self.file.text.as_bytes().get(start as usize + 1), Some(b'x' | b'X'))
        {
            i128::from_str_radix(text.trim_start_matches("0x").trim_start_matches("0X"), 16)
        } else if start as usize + 1 < self.file.text.len()
            && self.file.text.as_bytes().get(start as usize) == Some(&b'0')
            && matches!(self.file.text.as_bytes().get(start as usize + 1), Some(b'b' | b'B'))
        {
            i128::from_str_radix(text.trim_start_matches("0b").trim_start_matches("0B"), 2)
        } else {
            text.parse::<i128>()
        };

        match value {
            Ok(v) => TokenKind::Int(v),
            Err(_) => {
                TokenKind::Int(0)
            }
        }
    }

    fn parse_float(&self, start: u32) -> TokenKind {
        let text: String = self.slice(start, self.offset).chars().filter(|c| *c != '_').collect();
        text.parse::<f64>().map(TokenKind::Float).unwrap_or(TokenKind::Float(0.0))
    }

    fn lex_char(&mut self, start: u32) -> Token {
        self.advance(); // opening '
        let result = if self.is_at_end() {
            self.report_error(start, "unterminated character literal", "missing closing `'`");
            TokenKind::Char('\0')
        } else if self.peek() == '\\' {
            match self.scan_escape() {
                Some(ch) => TokenKind::Char(ch),
                None => TokenKind::Char('\0'),
            }
        } else if self.peek() == '\'' {
            self.report_error(start, "empty character literal", "expected a character");
            TokenKind::Char('\0')
        } else {
            let ch = self.advance();
            if self.peek() != '\'' {
                self.report_error(start, "unterminated character literal", "missing closing `'`");
            } else {
                self.advance();
            }
            TokenKind::Char(ch)
        };
        self.make_token(result, start)
    }

    fn lex_string(&mut self, start: u32) -> Token {
        self.advance(); // opening "
        let mut parts = Vec::new();
        let mut text_buf = String::new();

        while !self.is_at_end() && self.peek() != '"' {
            if self.peek() == '{' {
                if !text_buf.is_empty() {
                    parts.push(StringPart::Text(std::mem::take(&mut text_buf)));
                }
                let interp_start = self.offset;
                self.advance(); // {
                match self.scan_balanced_expr_from_open() {
                    Some(raw) => {
                        let span = Span::new(BytePos(interp_start), BytePos(self.offset));
                        parts.push(StringPart::InterpRaw(Spanned::new(raw, span)));
                    }
                    None => {
                        self.report_error(
                            interp_start,
                            "unclosed interpolation expression",
                            "missing `}` to close `{...}`",
                        );
                        break;
                    }
                }
            } else if self.peek() == '\\' {
                if let Some(ch) = self.scan_escape_char() {
                    text_buf.push(ch);
                }
            } else if self.peek() == '\n' || self.peek() == '\r' {
                self.report_error(start, "unterminated string literal", "missing closing `\"`");
                break;
            } else {
                text_buf.push(self.advance());
            }
        }

        if !text_buf.is_empty() {
            parts.push(StringPart::Text(text_buf));
        }

        if self.peek() == '"' {
            self.advance();
        } else if !self.is_at_end() {
            self.report_error(start, "unterminated string literal", "missing closing `\"`");
        }

        self.make_token(TokenKind::String(crate::token::StringLit { parts }), start)
    }

    fn lex_bytes(&mut self, start: u32) -> Token {
        self.advance(); // b
        self.advance(); // "
        let mut bytes = Vec::new();
        while !self.is_at_end() && self.peek() != '"' {
            if self.peek() == '\\' {
                self.advance();
                if self.is_at_end() {
                    break;
                }
                match self.peek() {
                    'n' => bytes.push(b'\n'),
                    'r' => bytes.push(b'\r'),
                    't' => bytes.push(b'\t'),
                    '\\' => bytes.push(b'\\'),
                    '0' => bytes.push(0),
                    'x' => {
                        self.advance();
                        let hi = self.hex_digit().unwrap_or(0);
                        let lo = self.hex_digit().unwrap_or(0);
                        bytes.push((hi << 4) | lo);
                    }
                    c => bytes.push(c as u8),
                }
                self.advance();
            } else {
                bytes.push(self.advance() as u8);
            }
        }
        if self.peek() == '"' {
            self.advance();
        }
        self.make_token(TokenKind::Bytes(bytes), start)
    }

    /// Called after consuming `{`; scans until matching `}`.
    fn scan_balanced_expr_from_open(&mut self) -> Option<String> {
        let start = self.offset;
        let mut depth = 1i32;
        while !self.is_at_end() {
            match self.peek() {
                '{' => {
                    depth += 1;
                    self.advance();
                }
                '}' => {
                    depth -= 1;
                    self.advance();
                    if depth == 0 {
                        let inner = self.slice(start, self.offset - 1);
                        return Some(inner.to_owned());
                    }
                }
                '"' => {
                    self.lex_string_inner_quoted();
                }
                '\'' => {
                    self.advance();
                    while !self.is_at_end() && self.peek() != '\'' {
                        if self.peek() == '\\' {
                            self.advance();
                        }
                        self.advance();
                    }
                    if self.peek() == '\'' {
                        self.advance();
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }
        None
    }

    fn lex_string_inner_quoted(&mut self) {
        self.advance();
        while !self.is_at_end() && self.peek() != '"' {
            if self.peek() == '\\' {
                self.advance();
                if !self.is_at_end() {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }
        if self.peek() == '"' {
            self.advance();
        }
    }

    fn scan_escape(&mut self) -> Option<char> {
        self.advance();
        if self.is_at_end() {
            return None;
        }
        let ch = match self.peek() {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '\\' => '\\',
            '\'' => '\'',
            '"' => '"',
            '0' => '\0',
            'x' => {
                self.advance();
                let hi = self.hex_digit()? as u32;
                let lo = self.hex_digit()? as u32;
                char::from_u32((hi << 4) | lo)?
            }
            c => c,
        };
        self.advance();
        Some(ch)
    }

    fn scan_escape_char(&mut self) -> Option<char> {
        // `scan_escape` consumes the leading `\`.
        self.scan_escape()
    }

    fn hex_digit(&mut self) -> Option<u8> {
        if self.is_at_end() {
            return None;
        }
        let v = self.peek().to_digit(16)? as u8;
        self.advance();
        Some(v)
    }

    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                '\n' => {
                    self.advance();
                }
                '#' => {
                    while !self.is_at_end() && self.peek() != '\n' {
                        self.advance();
                    }
                }
                _ if self.starts_with("\"\"\"") => {
                    self.advance();
                    self.advance();
                    self.advance();
                    while !self.is_at_end() && !self.starts_with("\"\"\"") {
                        self.advance();
                    }
                    if self.starts_with("\"\"\"") {
                        self.advance();
                        self.advance();
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        self.file.text[self.offset as usize..].starts_with(s)
    }

    fn slice(&self, start: u32, end: u32) -> &str {
        &self.file.text[start as usize..end as usize]
    }

    fn byte_index(&self, offset: u32) -> usize {
        self.file.text[..offset as usize].chars().count()
    }

    fn make_token(&self, kind: TokenKind, start: u32) -> Token {
        Token {
            kind,
            span: Span::new(BytePos(start), BytePos(self.offset)),
        }
    }

    fn report_error(&self, start: u32, message: impl Into<String>, reason: impl Into<String>) {
        if let Some(handler) = self.diagnostics {
            let span = Span::new(BytePos(start), BytePos(self.offset.max(start + 1)));
            let reason = reason.into();
            handler.emit(
                self.file,
                Diagnostic::error(message)
                    .with_code("E0001")
                    .with_reason(reason.clone())
                    .with_label(Label::primary(span, reason)),
            );
        }
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.chars[self.pos]
        }
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn peek_at(&self, offset: u32) -> char {
        let idx = self.file.text[..offset as usize].chars().count();
        self.chars.get(idx).copied().unwrap_or('\0')
    }

    fn advance(&mut self) -> char {
        if self.is_at_end() {
            return '\0';
        }
        let ch = self.chars[self.pos];
        self.pos += 1;
        self.offset += ch.len_utf8() as u32;
        ch
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.peek() != expected {
            false
        } else {
            self.advance();
            true
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StringPart;
    use buraaq_diagnostics::StandardHandler;
    use std::sync::Arc;

    fn lex(source: &str) -> Vec<TokenKind> {
        let file = SourceFile::new("test.bq", source);
        Lexer::new(&file)
            .tokenize()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn lexes_keywords_and_ident() {
        let kinds = lex("fn main async");
        assert!(matches!(kinds[0], TokenKind::Fn));
        assert!(matches!(kinds[1], TokenKind::Ident(_)));
        assert!(matches!(kinds[2], TokenKind::Async));
    }

    #[test]
    fn lexes_integers() {
        let kinds = lex("42 0xFF 0b1010 1_000");
        assert!(matches!(kinds[0], TokenKind::Int(42)));
        assert!(matches!(kinds[1], TokenKind::Int(255)));
        assert!(matches!(kinds[2], TokenKind::Int(10)));
        assert!(matches!(kinds[3], TokenKind::Int(1000)));
    }

    #[test]
    fn lexes_floats() {
        let kinds = lex("3.14 1e9");
        assert!(matches!(kinds[0], TokenKind::Float(v) if (v - 3.14).abs() < f64::EPSILON));
        assert!(matches!(kinds[1], TokenKind::Float(_)));
    }

    #[test]
    fn lexes_string_with_interpolation() {
        let file = SourceFile::new("test.bq", r#""hi {name}""#);
        let tokens = Lexer::new(&file).tokenize();
        assert!(matches!(&tokens[0].kind, TokenKind::String(s) if !s.parts.is_empty()));
    }

    #[test]
    fn lexes_string_newline_escape() {
        let file = SourceFile::new("test.bq", r#""\n""#);
        let tokens = Lexer::new(&file).tokenize();
        assert!(matches!(
            &tokens[0].kind,
            TokenKind::String(s) if matches!(
                s.parts.as_slice(),
                [StringPart::Text(t)] if t == "\n"
            )
        ));
        assert!(matches!(tokens[1].kind, TokenKind::Eof));
        assert_eq!(lex(r#""line1\nline2""#).len(), 2); // string + Eof
    }

    #[test]
    fn lexes_operators() {
        let kinds = lex("-> => .. ..= == != && || ??");
        assert!(matches!(kinds[0], TokenKind::Arrow));
        assert!(matches!(kinds[1], TokenKind::FatArrow));
        assert!(matches!(kinds[2], TokenKind::DotDot));
        assert!(matches!(kinds[3], TokenKind::DotDotEq));
    }

    #[test]
    fn invalid_char_reports_diagnostic() {
        let file = SourceFile::new("test.bq", "\u{0001}");
        let handler = Arc::new(StandardHandler::new());
        Lexer::new(&file)
            .with_diagnostics(handler.as_ref())
            .tokenize();
        assert_eq!(handler.error_count(), 1);
    }
}
