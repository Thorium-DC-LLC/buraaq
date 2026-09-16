use buraaq_ast::*;
use buraaq_lexer::TokenKind;
use buraaq_source::{Span, Spanned};

use crate::parser::Parser;

fn ex(e: Expr, span: Span) -> Spanned<ExprNode> {
    Spanned::new(Box::new(e), span)
}

impl Parser<'_> {
    pub(crate) fn parse_expr(&mut self) -> Spanned<ExprNode> {
        let start = self.current_span().start;
        let expr = self.parse_assign();
        ex(expr, Span::new(start, self.previous().span.end))
    }

    /// Parses an expression when the next `{` begins a block, not a struct literal.
    pub(crate) fn parse_expr_before_block(&mut self) -> Spanned<ExprNode> {
        let prev = self.allow_struct_lit;
        self.allow_struct_lit = false;
        let expr = self.parse_expr();
        self.allow_struct_lit = prev;
        expr
    }

    pub(crate) fn is_expr_start(&self) -> bool {
        matches!(
            self.peek_kind(),
            TokenKind::Ident(_)
                | TokenKind::Int(_)
                | TokenKind::Float(_)
                | TokenKind::True
                | TokenKind::False
                | TokenKind::None
                | TokenKind::Some
                | TokenKind::Ok
                | TokenKind::Err
                | TokenKind::Char(_)
                | TokenKind::String(_)
                | TokenKind::Bytes(_)
                | TokenKind::LParen
                | TokenKind::LBrace
                | TokenKind::If
                | TokenKind::Match
                | TokenKind::Async
                | TokenKind::Spawn
                | TokenKind::New
                | TokenKind::Unsafe
                | TokenKind::SelfKw
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Bang
                | TokenKind::Not
                | TokenKind::Ref
                | TokenKind::Give
                | TokenKind::Copy
                | TokenKind::Await
                | TokenKind::LBracket
        )
    }

    fn parse_assign(&mut self) -> Expr {
        let left = self.parse_coalesce();
        if self.match_token(TokenKind::Eq) {
            let span = expr_span(&left).merge(self.previous().span);
            let value = if self.is_expr_start() {
                self.parse_assign()
            } else {
                self.error_with_help(
                    self.current_span(),
                    "expected a value after `=`",
                    "a value is required here",
                    "add a value on the right-hand side",
                    "user.name = \"Asim\"".into(),
                );
                Expr::Missing(self.current_span())
            };
            return Expr::Assign(Spanned::new(
                AssignExpr {
                    target: ex(left, span),
                    value: ex(value, self.previous().span),
                },
                span,
            ));
        }
        left
    }

    fn parse_coalesce(&mut self) -> Expr {
        let mut left = self.parse_or();
        while self.match_token(TokenKind::QuestionQuestion) {
            let right = self.parse_or();
            let span = expr_span(&left).merge(expr_span(&right));
            left = Expr::Coalesce(Spanned::new(
                CoalesceExpr {
                    left: ex(left, span),
                    right: ex(right, self.previous().span),
                },
                span,
            ));
        }
        left
    }

    fn parse_or(&mut self) -> Expr {
        let mut left = self.parse_and();
        while self.match_token(TokenKind::OrOr) {
            let right = self.parse_and();
            let span = expr_span(&left).merge(expr_span(&right));
            left = Expr::Binary(Spanned::new(
                BinaryExpr {
                    left: ex(left, span),
                    op: Spanned::new(BinOp::Or, self.previous().span),
                    right: ex(right, self.previous().span),
                },
                span,
            ));
        }
        left
    }

    fn parse_and(&mut self) -> Expr {
        let mut left = self.parse_equality();
        while self.match_token(TokenKind::AndAnd) {
            let right = self.parse_equality();
            let span = expr_span(&left).merge(expr_span(&right));
            left = Expr::Binary(Spanned::new(
                BinaryExpr {
                    left: ex(left, span),
                    op: Spanned::new(BinOp::And, self.previous().span),
                    right: ex(right, self.previous().span),
                },
                span,
            ));
        }
        left
    }

    fn parse_equality(&mut self) -> Expr {
        let mut left = self.parse_relational();
        while matches!(self.peek_kind(), TokenKind::EqEq | TokenKind::NotEq) {
            let op = match self.advance().kind {
                TokenKind::EqEq => BinOp::Eq,
                _ => BinOp::NotEq,
            };
            let right = self.parse_relational();
            let span = expr_span(&left).merge(expr_span(&right));
            left = Expr::Binary(Spanned::new(
                BinaryExpr {
                    left: ex(left, span),
                    op: Spanned::new(op, self.previous().span),
                    right: ex(right, self.previous().span),
                },
                span,
            ));
        }
        left
    }

    fn parse_relational(&mut self) -> Expr {
        let mut left = self.parse_bitwise_or();
        while matches!(
            self.peek_kind(),
            TokenKind::Lt | TokenKind::Le | TokenKind::Gt | TokenKind::Ge
        ) {
            let op = match self.advance().kind {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Le => BinOp::Le,
                TokenKind::Gt => BinOp::Gt,
                _ => BinOp::Ge,
            };
            let right = self.parse_bitwise_or();
            let span = expr_span(&left).merge(expr_span(&right));
            left = Expr::Binary(Spanned::new(
                BinaryExpr {
                    left: ex(left, span),
                    op: Spanned::new(op, self.previous().span),
                    right: ex(right, self.previous().span),
                },
                span,
            ));
        }
        left
    }

    fn parse_bitwise_or(&mut self) -> Expr {
        let mut left = self.parse_bitwise_xor();
        while self.match_token(TokenKind::Pipe) {
            let right = self.parse_bitwise_xor();
            let span = expr_span(&left).merge(expr_span(&right));
            left = Expr::Binary(Spanned::new(
                BinaryExpr {
                    left: ex(left, span),
                    op: Spanned::new(BinOp::BitOr, self.previous().span),
                    right: ex(right, self.previous().span),
                },
                span,
            ));
        }
        left
    }

    fn parse_bitwise_xor(&mut self) -> Expr {
        let mut left = self.parse_bitwise_and();
        while self.match_token(TokenKind::Caret) {
            let right = self.parse_bitwise_and();
            let span = expr_span(&left).merge(expr_span(&right));
            left = Expr::Binary(Spanned::new(
                BinaryExpr {
                    left: ex(left, span),
                    op: Spanned::new(BinOp::BitXor, self.previous().span),
                    right: ex(right, self.previous().span),
                },
                span,
            ));
        }
        left
    }

    fn parse_bitwise_and(&mut self) -> Expr {
        let mut left = self.parse_shift();
        while self.match_token(TokenKind::Amp) {
            let right = self.parse_shift();
            let span = expr_span(&left).merge(expr_span(&right));
            left = Expr::Binary(Spanned::new(
                BinaryExpr {
                    left: ex(left, span),
                    op: Spanned::new(BinOp::BitAnd, self.previous().span),
                    right: ex(right, self.previous().span),
                },
                span,
            ));
        }
        left
    }

    fn parse_shift(&mut self) -> Expr {
        let mut left = self.parse_additive();
        while matches!(self.peek_kind(), TokenKind::Shl | TokenKind::Shr) {
            let op = match self.advance().kind {
                TokenKind::Shl => BinOp::Shl,
                _ => BinOp::Shr,
            };
            let right = self.parse_additive();
            let span = expr_span(&left).merge(expr_span(&right));
            left = Expr::Binary(Spanned::new(
                BinaryExpr {
                    left: ex(left, span),
                    op: Spanned::new(op, self.previous().span),
                    right: ex(right, self.previous().span),
                },
                span,
            ));
        }
        left
    }

    fn parse_additive(&mut self) -> Expr {
        let mut left = self.parse_multiplicative();
        while matches!(self.peek_kind(), TokenKind::Plus | TokenKind::Minus) {
            let op = match self.advance().kind {
                TokenKind::Plus => BinOp::Add,
                _ => BinOp::Sub,
            };
            let right = self.parse_multiplicative();
            let span = expr_span(&left).merge(expr_span(&right));
            left = Expr::Binary(Spanned::new(
                BinaryExpr {
                    left: ex(left, span),
                    op: Spanned::new(op, self.previous().span),
                    right: ex(right, self.previous().span),
                },
                span,
            ));
        }
        left
    }

    fn parse_multiplicative(&mut self) -> Expr {
        let mut left = self.parse_unary();
        while matches!(
            self.peek_kind(),
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent
        ) {
            let op = match self.advance().kind {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                _ => BinOp::Mod,
            };
            let right = self.parse_unary();
            let span = expr_span(&left).merge(expr_span(&right));
            left = Expr::Binary(Spanned::new(
                BinaryExpr {
                    left: ex(left, span),
                    op: Spanned::new(op, self.previous().span),
                    right: ex(right, self.previous().span),
                },
                span,
            ));
        }
        left
    }

    fn parse_unary(&mut self) -> Expr {
        let start = self.current_span();
        if self.match_token(TokenKind::Minus) {
            let inner = self.parse_unary();
            return Expr::Unary(Spanned::new(
                UnaryExpr {
                    op: Spanned::new(UnaryOp::Neg, self.previous().span),
                    expr: ex(inner, self.previous().span),
                },
                Span::new(start.start, self.previous().span.end),
            ));
        }
        if self.match_token(TokenKind::Star) {
            let inner = self.parse_unary();
            return Expr::Unary(Spanned::new(
                UnaryExpr {
                    op: Spanned::new(UnaryOp::Deref, self.previous().span),
                    expr: ex(inner, self.previous().span),
                },
                Span::new(start.start, self.previous().span.end),
            ));
        }
        if self.match_token(TokenKind::Bang) || self.match_token(TokenKind::Not) {
            let inner = self.parse_unary();
            return Expr::Unary(Spanned::new(
                UnaryExpr {
                    op: Spanned::new(UnaryOp::Not, self.previous().span),
                    expr: ex(inner, self.previous().span),
                },
                Span::new(start.start, self.previous().span.end),
            ));
        }
        if self.match_token(TokenKind::Ref) {
            let op = if self.match_token(TokenKind::Mut) {
                UnaryOp::RefMut
            } else {
                UnaryOp::Ref
            };
            let inner = self.parse_unary();
            return Expr::Unary(Spanned::new(
                UnaryExpr {
                    op: Spanned::new(op, self.previous().span),
                    expr: ex(inner, self.previous().span),
                },
                Span::new(start.start, self.previous().span.end),
            ));
        }
        if self.match_token(TokenKind::Give) {
            let inner = self.parse_unary();
            return Expr::Unary(Spanned::new(
                UnaryExpr {
                    op: Spanned::new(UnaryOp::Give, self.previous().span),
                    expr: ex(inner, self.previous().span),
                },
                Span::new(start.start, self.previous().span.end),
            ));
        }
        if self.match_token(TokenKind::Copy) {
            let inner = self.parse_unary();
            return Expr::Unary(Spanned::new(
                UnaryExpr {
                    op: Spanned::new(UnaryOp::Copy, self.previous().span),
                    expr: ex(inner, self.previous().span),
                },
                Span::new(start.start, self.previous().span.end),
            ));
        }
        if self.match_token(TokenKind::Await) {
            let inner = self.parse_unary();
            return Expr::Await(Spanned::new(
                Box::new(inner),
                Span::new(start.start, self.previous().span.end),
            ));
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Expr {
        let mut expr = self.parse_primary();
        loop {
            match self.peek_kind() {
                TokenKind::Dot => {
                    self.advance();
                    let field = self.expect_ident("expected field or method name");
                    if self.check(&TokenKind::LParen) {
                        let args = self.parse_call_args();
                        let span = expr_span(&expr).merge(self.previous().span);
                        expr = Expr::MethodCall(Spanned::new(
                            MethodCallExpr {
                                receiver: ex(expr, span),
                                method: field,
                                args,
                            },
                            span,
                        ));
                    } else {
                        let span = expr_span(&expr).merge(field.span);
                        expr = Expr::Field(Spanned::new(
                            FieldExpr {
                                base: ex(expr, span),
                                field,
                            },
                            span,
                        ));
                    }
                }
                TokenKind::LParen => {
                    let args = self.parse_call_args();
                    let span = expr_span(&expr).merge(self.previous().span);
                    expr = Expr::Call(Spanned::new(
                        CallExpr {
                            callee: ex(expr, span),
                            args,
                        },
                        span,
                    ));
                }
                TokenKind::LBracket => {
                    self.advance();
                    if self.match_token(TokenKind::RBracket) {
                        let span = expr_span(&expr).merge(self.previous().span);
                        expr = Expr::Unary(Spanned::new(
                            UnaryExpr {
                                op: Spanned::new(UnaryOp::Deref, self.previous().span),
                                expr: ex(expr, span),
                            },
                            span,
                        ));
                    } else {
                        let index = self.parse_expr();
                        self.expect(TokenKind::RBracket, "expected `]` after index");
                        let span = expr_span(&expr).merge(self.previous().span);
                        expr = Expr::Index(Spanned::new(
                            IndexExpr {
                                base: ex(expr, span),
                                index,
                            },
                            span,
                        ));
                    }
                }
                TokenKind::Question => {
                    self.advance();
                    let span = expr_span(&expr).merge(self.previous().span);
                    expr = Expr::Try(Spanned::new(Box::new(expr), span));
                }
                TokenKind::Bang => {
                    self.advance();
                    let span = expr_span(&expr).merge(self.previous().span);
                    expr = Expr::Unwrap(Spanned::new(Box::new(expr), span));
                }
                _ => break,
            }
        }
        expr
    }

    fn parse_primary(&mut self) -> Expr {
        let start = self.current_span();
        if self.is_literal_token() {
            return Expr::Literal(self.parse_literal());
        }
        if self.match_token(TokenKind::SelfKw) {
            return Expr::Self_(self.previous().span);
        }
        if let TokenKind::Ident(name) = self.peek_kind() {
            let tok = self.advance();
            let name_sp = Spanned::new(name, tok.span);
            if self.check(&TokenKind::LParen)
                || (self.allow_struct_lit && self.check(&TokenKind::LBrace))
            {
                return self.finish_struct_expr(name_sp);
            }
            return Expr::Ident(name_sp);
        }
        if self.match_token(TokenKind::LParen) {
            let inner = self.parse_expr();
            self.expect(TokenKind::RParen, "expected `)` after expression");
            return Expr::Paren(Spanned::new(
                inner.node,
                Span::new(start.start, self.previous().span.end),
            ));
        }
        if self.match_token(TokenKind::LBrace) {
            let block = self.parse_block_inner(start.start);
            return Expr::Block(block);
        }
        if self.match_token(TokenKind::If) {
            let cond = self.parse_expr();
            let then_block = self.parse_block();
            self.expect(TokenKind::Else, "expected `else` in if expression");
            let else_block = self.parse_block();
            let span = Span::new(start.start, else_block.span.end);
            return Expr::If(Spanned::new(
                IfExpr {
                    cond,
                    then_block,
                    else_block,
                },
                span,
            ));
        }
        if self.match_token(TokenKind::Match) {
            let scrutinee = self.parse_expr_before_block();
            let arms = self.parse_match_arms();
            let span = Span::new(start.start, self.previous().span.end);
            return Expr::Match(Spanned::new(MatchExpr { scrutinee, arms }, span));
        }
        if self.match_token(TokenKind::Async) {
            let body = self.parse_block();
            return Expr::Async(body);
        }
        if self.match_token(TokenKind::Spawn) {
            if self.check(&TokenKind::LBrace) {
                let body = self.parse_block();
                let span = body.span;
                return Expr::Spawn(Spanned::new(SpawnExpr::Block(body), span));
            }
            let call = self.parse_expr();
            return Expr::Spawn(Spanned::new(SpawnExpr::Call(call.clone()), call.span));
        }
        if self.match_token(TokenKind::Unsafe) {
            let body = self.parse_block();
            return Expr::Unsafe(body);
        }
        if self.match_token(TokenKind::New) {
            let ty = self.parse_type();
            let init = if self.check(&TokenKind::LParen) || self.check(&TokenKind::LBrace) {
                Some(self.parse_new_init())
            } else {
                None
            };
            let span = Span::new(start.start, self.previous().span.end);
            return Expr::New(Spanned::new(NewExpr { ty, init }, span));
        }
        if self.match_token(TokenKind::LBracket) {
            let mut elems = Vec::new();
            if !self.check(&TokenKind::RBracket) {
                loop {
                    elems.push(self.parse_expr());
                    if !self.match_token(TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.expect(TokenKind::RBracket, "expected `]` after array literal");
            return Expr::Array(Spanned::new(elems, Span::new(start.start, self.previous().span.end)));
        }
        self.error_at_current("expected expression", "found an unexpected token here");
        Expr::Missing(start)
    }

    fn parse_new_init(&mut self) -> StructInitTail {
        if self.match_token(TokenKind::LParen) {
            let mut args = Vec::new();
            if !self.check(&TokenKind::RParen) {
                loop {
                    args.push(self.parse_expr());
                    if !self.match_token(TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.expect(TokenKind::RParen, "expected `)` after constructor args");
            StructInitTail::Tuple(args)
        } else {
            self.expect(TokenKind::LBrace, "expected `{{` after type in `new`");
            let mut fields = Vec::new();
            while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
                let fname = self.expect_ident("expected field name");
                self.expect(TokenKind::Colon, "expected `:` in constructor");
                let value = self.parse_expr();
                fields.push(Spanned::new(
                    FieldInit {
                        name: fname.clone(),
                        value: value.clone(),
                    },
                    Span::new(fname.span.start, value.span.end),
                ));
                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RBrace, "expected `}` after constructor");
            StructInitTail::Named(fields)
        }
    }

    fn finish_struct_expr(&mut self, name: Spanned<String>) -> Expr {
        let start = name.span.start;
        let path = Spanned::new(
            Path {
                segments: vec![name.clone()],
                span: name.span,
            },
            name.span,
        );
        if self.match_token(TokenKind::LParen) {
            let mut args = Vec::new();
            if !self.check(&TokenKind::RParen) {
                loop {
                    args.push(self.parse_expr());
                    if !self.match_token(TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.expect(TokenKind::RParen, "expected `)` after tuple struct literal");
            let span = Span::new(start, self.previous().span.end);
            return Expr::Struct(Spanned::new(
                StructExpr {
                    path,
                    fields: vec![],
                    fill: StructFill::Tuple(args),
                },
                span,
            ));
        }
        self.expect(TokenKind::LBrace, "expected `{{` or `(` after struct name");
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let fname = self.expect_ident("expected field name");
            self.expect(TokenKind::Colon, "expected `:` in struct literal");
            let value = self.parse_expr();
            fields.push(Spanned::new(
                FieldInit {
                    name: fname.clone(),
                    value: value.clone(),
                },
                Span::new(fname.span.start, value.span.end),
            ));
            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RBrace, "expected `}` after struct literal");
        let span = Span::new(start, self.previous().span.end);
        Expr::Struct(Spanned::new(
            StructExpr {
                path,
                fields,
                fill: StructFill::Named,
            },
            span,
        ))
    }

    fn parse_call_args(&mut self) -> Vec<Spanned<ExprNode>> {
        self.expect(TokenKind::LParen, "expected `(`");
        let mut args = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                args.push(self.parse_expr());
                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen, "expected `)` after arguments");
        args
    }

    fn parse_block_inner(&mut self, start: buraaq_source::BytePos) -> Spanned<Block> {
        let mut stmts = Vec::new();
        let mut tail = None;
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            if self.is_block_tail_expr() {
                let mark = self.mark();
                let expr = self.parse_expr();
                if self.check(&TokenKind::RBrace) {
                    tail = Some(expr);
                    break;
                }
                self.rewind(mark);
            }
            stmts.push(self.parse_stmt());
        }
        let end = self.expect(TokenKind::RBrace, "expected `}` to close block").span.end;
        Spanned::new(
            Block {
                stmts,
                tail,
                span: Span::new(start, end),
            },
            Span::new(start, end),
        )
    }

    fn is_literal_token(&self) -> bool {
        matches!(
            self.peek_kind(),
            TokenKind::Int(_)
                | TokenKind::Float(_)
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Char(_)
                | TokenKind::String(_)
                | TokenKind::Bytes(_)
                | TokenKind::None
                | TokenKind::Some
                | TokenKind::Ok
                | TokenKind::Err
        )
    }
}

fn expr_span(expr: &Expr) -> Span {
    match expr {
        Expr::Literal(s) => s.span,
        Expr::Ident(s) => s.span,
        Expr::Self_(s) => *s,
        Expr::Binary(s) => s.span,
        Expr::Unary(s) => s.span,
        Expr::Assign(s) => s.span,
        Expr::Call(s) => s.span,
        Expr::MethodCall(s) => s.span,
        Expr::Field(s) => s.span,
        Expr::Index(s) => s.span,
        Expr::Paren(s) => s.span,
        Expr::Block(s) => s.span,
        Expr::If(s) => s.span,
        Expr::Match(s) => s.span,
        Expr::Struct(s) => s.span,
        Expr::Array(s) => s.span,
        Expr::New(s) => s.span,
        Expr::Async(s) => s.span,
        Expr::Spawn(s) => s.span,
        Expr::Unsafe(s) => s.span,
        Expr::Await(s) => s.span,
        Expr::Try(s) => s.span,
        Expr::Unwrap(s) => s.span,
        Expr::Coalesce(s) => s.span,
        Expr::Missing(s) => *s,
    }
}
