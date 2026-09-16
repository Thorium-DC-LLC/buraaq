use buraaq_ast::{ExprNode, *};
use buraaq_diagnostics::{Diagnostic, DiagnosticHandler, Help, Label, Suggestion};
use buraaq_lexer::{Lexer, Token, TokenKind};
use buraaq_source::{BytePos, SourceFile, Span, Spanned};

use crate::recovery::{is_stmt_sync, is_top_level_sync, is_type_sync};

fn missing_expr(span: Span) -> Spanned<ExprNode> {
    Spanned::new(Box::new(Expr::Missing(span)), span)
}

fn box_type(ty: Spanned<Type>) -> Spanned<TypeNode> {
    Spanned::new(Box::new(ty.node), ty.span)
}

pub struct ParseResult {
    pub program: Program,
    pub had_errors: bool,
}

pub struct Parser<'a> {
    pub(crate) file: &'a SourceFile,
    pub(crate) tokens: Vec<Token>,
    pub(crate) pos: usize,
    pub(crate) diagnostics: &'a dyn DiagnosticHandler,
    /// When false, `name {` is an identifier followed by a block, not a struct literal.
    pub(crate) allow_struct_lit: bool,
}

impl<'a> Parser<'a> {
    pub fn new(file: &'a SourceFile, tokens: Vec<Token>, diagnostics: &'a dyn DiagnosticHandler) -> Self {
        Self {
            file,
            tokens,
            pos: 0,
            diagnostics,
            allow_struct_lit: true,
        }
    }

    pub fn parse(file: &'a SourceFile, diagnostics: &'a dyn DiagnosticHandler) -> ParseResult {
        let tokens = Lexer::new(file).with_diagnostics(diagnostics).tokenize();
        let mut parser = Self::new(file, tokens, diagnostics);
        let program = parser.parse_program();
        ParseResult {
            program,
            had_errors: diagnostics.has_errors(),
        }
    }

    fn parse_program(&mut self) -> Program {
        let start = self.current_span().start;
        let attrs = self.parse_attributes();
        let mut module = None;
        let mut imports = Vec::new();
        let mut items = Vec::new();

        while !self.check(&TokenKind::Eof) {
            if self.check(&TokenKind::Module) {
                module = Some(self.parse_module_decl());
            } else if self.check(&TokenKind::Use) {
                imports.push(self.parse_import_decl());
            } else if is_top_level_sync(&self.peek_kind()) {
                items.push(self.parse_item());
            } else {
                self.error_at_current(
                    "unexpected token at top level",
                    "expected a declaration such as `fn`, `struct`, or `use`",
                );
                self.advance();
            }
        }

        let end = if self.pos > 0 {
            self.tokens[self.pos - 1].span.end
        } else {
            BytePos(self.file.len())
        };

        Program {
            attrs,
            module,
            imports,
            items,
            span: Span::new(start, end),
        }
    }

    fn parse_module_decl(&mut self) -> Spanned<ModuleDecl> {
        let start = self.expect(TokenKind::Module, "expected `module`").span.start;
        let path = self.parse_path();
        self.consume_semi();
        let span = Span::new(start, self.previous().span.end);
        Spanned::new(ModuleDecl { path }, span)
    }

    fn parse_import_decl(&mut self) -> Spanned<ImportDecl> {
        let start = self.expect(TokenKind::Use, "expected `use`").span.start;
        let mut specs = vec![self.parse_import_spec()];
        while self.match_token(TokenKind::Comma) {
            specs.push(self.parse_import_spec());
        }
        self.consume_semi();
        let span = Span::new(start, self.previous().span.end);
        Spanned::new(ImportDecl { specs }, span)
    }

    fn parse_import_spec(&mut self) -> Spanned<ImportSpec> {
        let start = self.current_span().start;
        let path = self.parse_path();
        let spec = if self.match_token(TokenKind::Dot) && self.match_token(TokenKind::LBrace) {
            let mut names = Vec::new();
            loop {
                if self.match_token(TokenKind::Star) {
                    names.push(Spanned::new(ImportName::Glob(self.previous().span), self.previous().span));
                } else {
                    let name = self.expect_ident("expected import name");
                    names.push(Spanned::new(ImportName::Name(name.clone()), name.span));
                }
                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RBrace, "expected `}` after import list");
            ImportSpec::Group { path, names }
        } else if self.match_token(TokenKind::As) {
            let alias = self.expect_ident("expected import alias");
            ImportSpec::Alias { path, alias }
        } else {
            ImportSpec::Single { path }
        };
        let span = Span::new(start, self.previous().span.end);
        Spanned::new(spec, span)
    }

    fn parse_item(&mut self) -> Spanned<Item> {
        let start = self.current_span().start;
        let attrs = self.parse_attributes();
        let pub_ = self.match_token(TokenKind::Pub);

        let item = if self.match_token(TokenKind::Async) {
            self.expect(TokenKind::Fn, "expected `fn` after `async`");
            Item::Function(self.parse_function_rest(start, attrs, pub_, true))
        } else if self.check(&TokenKind::Fn) {
            self.advance();
            Item::Function(self.parse_function_rest(start, attrs, pub_, false))
        } else if self.match_token(TokenKind::Struct) {
            Item::Struct(self.parse_struct(attrs, pub_))
        } else if self.match_token(TokenKind::Enum) {
            Item::Enum(self.parse_enum(attrs, pub_))
        } else if self.match_token(TokenKind::Trait) {
            Item::Trait(self.parse_trait(attrs, pub_))
        } else if self.match_token(TokenKind::Impl) {
            Item::Impl(self.parse_impl(attrs))
        } else if self.match_token(TokenKind::Const) {
            Item::Const(self.parse_const(attrs, pub_))
        } else if self.match_token(TokenKind::Type) {
            Item::TypeAlias(self.parse_type_alias(attrs, pub_))
        } else if self.match_token(TokenKind::Extern) {
            Item::Extern(self.parse_extern(attrs))
        } else if self.match_token(TokenKind::Test) {
            Item::Test(self.parse_test(attrs))
        } else if self.match_token(TokenKind::Bench) {
            Item::Bench(self.parse_bench(attrs))
        } else {
            self.error_at_current("expected item declaration", "try `fn`, `struct`, `enum`, or `use`");
            let span = self.current_span();
            self.synchronize_top_level();
            return Spanned::new(Item::Const(Spanned::new(
                ConstDef {
                    attrs: vec![],
                    pub_: false,
                    name: Spanned::new("<error>".into(), span),
                    ty: None,
                    value: missing_expr(span),
                },
                span,
            )), span);
        };

        let span = Span::new(start, self.previous().span.end);
        Spanned::new(item, span)
    }

    fn parse_function_rest(
        &mut self,
        start: BytePos,
        attrs: Vec<Attribute>,
        pub_: bool,
        async_: bool,
    ) -> Spanned<Function> {
        let name = self.expect_ident("expected function name");
        let generics = self.parse_generic_params();
        self.expect(TokenKind::LParen, "expected `(` after function name");
        let params = self.parse_param_list();
        self.expect(TokenKind::RParen, "expected `)` after parameters");
        let (ret, throws) = self.parse_fn_sig_tail();
        let body = self.parse_block();
        let span = Span::new(start, body.span.end);
        Spanned::new(
            Function {
                attrs,
                pub_,
                async_,
                name,
                generics,
                params,
                ret,
                throws,
                body,
            },
            span,
        )
    }

    fn parse_fn_sig_tail(&mut self) -> (Option<Spanned<Type>>, Option<Spanned<Type>>) {
        let throws = if self.match_token(TokenKind::Throws) {
            Some(self.parse_type())
        } else {
            None
        };
        let ret = if self.match_token(TokenKind::Arrow) {
            Some(self.parse_type())
        } else {
            None
        };
        (ret, throws)
    }

    fn parse_param_list(&mut self) -> Vec<Spanned<Param>> {
        let mut params = Vec::new();
        if self.check(&TokenKind::RParen) {
            return params;
        }
        loop {
            let by_ref = if self.match_token(TokenKind::Ref) {
                if self.match_token(TokenKind::Mut) {
                    buraaq_ast::ParamRef::RefMut
                } else {
                    buraaq_ast::ParamRef::Ref
                }
            } else {
                buraaq_ast::ParamRef::None
            };
            let name = self.expect_ident("expected parameter name");
            self.expect(TokenKind::Colon, "expected `:` after parameter name");
            let ty = self.parse_type();
            let span = Span::new(name.span.start, ty.span.end);
            params.push(Spanned::new(Param { name, ty, by_ref }, span));
            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }
        params
    }

    fn parse_generic_params(&mut self) -> Vec<Spanned<GenericParam>> {
        let mut params = Vec::new();
        if !self.match_token(TokenKind::LBracket) {
            return params;
        }
        loop {
            let name = self.expect_ident("expected generic parameter name");
            let mut bounds = Vec::new();
            if self.match_token(TokenKind::Colon) {
                bounds.push(self.parse_path());
                while self.match_token(TokenKind::Plus) {
                    bounds.push(self.parse_path());
                }
            }
            let span = Span::new(name.span.start, self.previous().span.end);
            params.push(Spanned::new(GenericParam { name, bounds }, span));
            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RBracket, "expected `]` after generic parameters");
        params
    }

    fn parse_struct(&mut self, attrs: Vec<Attribute>, pub_: bool) -> Spanned<StructDef> {
        let start = self.previous().span.start;
        let name = self.expect_ident("expected struct name");
        let generics = self.parse_generic_params();
        self.expect(TokenKind::LBrace, "expected `{` after struct name");
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            if self.match_token(TokenKind::Drop) {
                let body = self.parse_block();
                let span = body.span;
                methods.push(Spanned::new(Method::Drop(body), span));
            } else if self.check(&TokenKind::Fn) {
                methods.push(self.parse_method());
            } else {
                let before = self.pos;
                let fname = self.expect_ident("expected field name");
                self.expect(TokenKind::Colon, "expected `:` after field name");
                let ty = self.parse_type();
                self.consume_optional_semi();
                self.match_token(TokenKind::Comma);
                let span = Span::new(fname.span.start, ty.span.end);
                fields.push(Spanned::new(StructField { name: fname, ty }, span));
                if self.pos == before {
                    self.advance();
                }
            }
        }
        self.expect(TokenKind::RBrace, "expected `}` to close struct");
        let span = Span::new(start, self.previous().span.end);
        Spanned::new(
            StructDef {
                attrs,
                pub_,
                name,
                generics,
                fields,
                methods,
            },
            span,
        )
    }

    fn parse_enum(&mut self, attrs: Vec<Attribute>, pub_: bool) -> Spanned<EnumDef> {
        let start = self.previous().span.start;
        let name = self.expect_ident("expected enum name");
        let generics = self.parse_generic_params();
        self.expect(TokenKind::LBrace, "expected `{` after enum name");
        let mut variants = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            variants.push(self.parse_enum_variant());
            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }
        self.match_token(TokenKind::Comma);
        self.expect(TokenKind::RBrace, "expected `}` to close enum");
        let span = Span::new(start, self.previous().span.end);
        Spanned::new(
            EnumDef {
                attrs,
                pub_,
                name,
                generics,
                variants,
            },
            span,
        )
    }

    fn parse_enum_variant(&mut self) -> Spanned<EnumVariant> {
        let start = self.current_span().start;
        let name = self.expect_ident("expected variant name");
        let variant = if self.match_token(TokenKind::LParen) {
            let mut types = Vec::new();
            if !self.check(&TokenKind::RParen) {
                loop {
                    types.push(self.parse_type());
                    if !self.match_token(TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.expect(TokenKind::RParen, "expected `)` after variant types");
            EnumVariant::Tuple(name.clone(), types)
        } else if self.match_token(TokenKind::LBrace) {
            let mut fields = Vec::new();
            while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
                let fname = self.expect_ident("expected field name");
                self.expect(TokenKind::Colon, "expected `:` after field name");
                let ty = self.parse_type();
                let fspan = Span::new(fname.span.start, ty.span.end);
                fields.push(Spanned::new(StructField { name: fname, ty }, fspan));
                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RBrace, "expected `}` after variant fields");
            EnumVariant::Struct(name.clone(), fields)
        } else {
            EnumVariant::Unit(name.clone())
        };
        let span = Span::new(start, self.previous().span.end);
        Spanned::new(variant, span)
    }

    fn parse_trait(&mut self, attrs: Vec<Attribute>, pub_: bool) -> Spanned<TraitDef> {
        let start = self.previous().span.start;
        let name = self.expect_ident("expected trait name");
        let generics = self.parse_generic_params();
        self.expect(TokenKind::LBrace, "expected `{` after trait name");
        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            if !self.check(&TokenKind::Fn) {
                self.error_at_current("expected `fn` in trait", "trait bodies contain method signatures");
                self.advance();
                continue;
            }
            methods.push(self.parse_trait_method());
        }
        self.expect(TokenKind::RBrace, "expected `}` to close trait");
        let span = Span::new(start, self.previous().span.end);
        Spanned::new(
            TraitDef {
                attrs,
                pub_,
                name,
                generics,
                methods,
            },
            span,
        )
    }

    fn parse_trait_method(&mut self) -> Spanned<TraitMethod> {
        let start = self.current_span().start;
        self.expect(TokenKind::Fn, "expected `fn` in trait");
        let mname = self.expect_ident("expected method name");
        let generics = self.parse_generic_params();
        self.expect(TokenKind::LParen, "expected `(` after method name");
        let (receiver, params) = self.parse_method_params();
        self.expect(TokenKind::RParen, "expected `)` after parameters");
        let ret = if self.match_token(TokenKind::Arrow) {
            Some(self.parse_type())
        } else {
            None
        };
        self.consume_semi();
        let span = Span::new(start, self.previous().span.end);
        Spanned::new(
            TraitMethod {
                name: mname,
                generics,
                receiver,
                params,
                ret,
            },
            span,
        )
    }

    fn parse_impl(&mut self, attrs: Vec<Attribute>) -> Spanned<ImplDef> {
        let start = self.previous().span.start;
        let generics = self.parse_generic_params();
        let trait_ = self.parse_path();
        self.expect(TokenKind::For, "expected `for` in impl");
        let ty = self.parse_type();
        self.expect(TokenKind::LBrace, "expected `{` after impl type");
        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            if !self.check(&TokenKind::Fn) && !self.check(&TokenKind::Drop) {
                self.error_at_current("expected method in impl block", "try `fn` or `drop`");
                self.advance();
                continue;
            }
            methods.push(self.parse_method());
        }
        self.expect(TokenKind::RBrace, "expected `}` to close impl");
        let span = Span::new(start, self.previous().span.end);
        Spanned::new(
            ImplDef {
                attrs,
                generics,
                trait_,
                ty,
                methods,
            },
            span,
        )
    }

    fn parse_method(&mut self) -> Spanned<Method> {
        let start = self.current_span().start;
        if self.match_token(TokenKind::Drop) {
            let body = self.parse_block();
            let span = Span::new(start, body.span.end);
            return Spanned::new(Method::Drop(body), span);
        }
        self.expect(TokenKind::Fn, "expected `fn` or `drop`");
        let name = self.expect_ident("expected method name");
        self.expect(TokenKind::LParen, "expected `(` after method name");
        let (receiver, params) = self.parse_method_params();
        self.expect(TokenKind::RParen, "expected `)` after method parameters");
        let (ret, throws) = self.parse_fn_sig_tail();
        let body = self.parse_block();
        let span = Span::new(start, body.span.end);
        Spanned::new(
            Method::Function(Spanned::new(
                MethodFn {
                    name,
                    receiver,
                    params,
                    ret,
                    throws,
                    body,
                },
                span,
            )),
            span,
        )
    }

    fn parse_method_params(&mut self) -> (Receiver, Vec<Spanned<Param>>) {
        if self.match_token(TokenKind::Mut) && self.check(&TokenKind::SelfKw) {
            self.advance();
            let mut params = Vec::new();
            if self.match_token(TokenKind::Comma) {
                params = self.parse_param_list();
            }
            return (Receiver::SelfMut, params);
        }
        if self.match_token(TokenKind::SelfKw) {
            let mut params = Vec::new();
            if self.match_token(TokenKind::Comma) {
                params = self.parse_param_list();
            }
            return (Receiver::SelfValue, params);
        }
        if self.match_token(TokenKind::Ref) {
            if self.match_token(TokenKind::Mut) && self.check(&TokenKind::SelfKw) {
                self.advance();
                let mut params = Vec::new();
                if self.match_token(TokenKind::Comma) {
                    params = self.parse_param_list();
                }
                return (Receiver::SelfRefMut, params);
            }
            if self.match_token(TokenKind::SelfKw) {
                let mut params = Vec::new();
                if self.match_token(TokenKind::Comma) {
                    params = self.parse_param_list();
                }
                return (Receiver::SelfRef, params);
            }
        }
        (Receiver::None, self.parse_param_list())
    }

    fn parse_const(&mut self, attrs: Vec<Attribute>, pub_: bool) -> Spanned<ConstDef> {
        let start = self.previous().span.start;
        let name = self.expect_ident("expected constant name");
        let ty = if self.match_token(TokenKind::Colon) {
            Some(self.parse_type())
        } else {
            None
        };
        self.expect(TokenKind::Eq, "expected `=` after constant name");
        let value = self.parse_expr();
        self.consume_semi();
        let span = Span::new(start, self.previous().span.end);
        Spanned::new(
            ConstDef {
                attrs,
                pub_,
                name,
                ty,
                value,
            },
            span,
        )
    }

    fn parse_type_alias(&mut self, attrs: Vec<Attribute>, pub_: bool) -> Spanned<TypeAlias> {
        let start = self.previous().span.start;
        let name = self.expect_ident("expected type alias name");
        let generics = self.parse_generic_params();
        self.expect(TokenKind::Eq, "expected `=` in type alias");
        let ty = self.parse_type();
        self.consume_semi();
        let span = Span::new(start, self.previous().span.end);
        Spanned::new(
            TypeAlias {
                attrs,
                pub_,
                name,
                generics,
                ty,
            },
            span,
        )
    }

    fn parse_extern(&mut self, attrs: Vec<Attribute>) -> Spanned<ExternBlock> {
        let start = self.previous().span.start;
        let abi = self.expect_ident("expected ABI name after `extern`");
        if abi.node != "c" {
            self.error(
                abi.span,
                "only `extern c` is supported",
                "Buraaq currently supports C ABI extern blocks only",
            );
        }
        self.expect(TokenKind::LBrace, "expected `{` after `extern c`");
        let mut functions = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            self.expect(TokenKind::Fn, "expected `fn` in extern block");
            let name = self.expect_ident("expected extern function name");
            self.expect(TokenKind::LParen, "expected `(` after extern function name");
            let mut params = Vec::new();
            let mut variadic = false;
            if !self.check(&TokenKind::RParen) {
                loop {
                    if self.match_token(TokenKind::DotDotDot) {
                        variadic = true;
                        break;
                    }
                    let pname = self.expect_ident("expected parameter name");
                    self.expect(TokenKind::Colon, "expected `:` after parameter name");
                    let ty = self.parse_type();
                    let pspan = Span::new(pname.span.start, ty.span.end);
                    params.push(Spanned::new(ExternParam { name: pname, ty }, pspan));
                    if !self.match_token(TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.expect(TokenKind::RParen, "expected `)` after extern parameters");
            self.expect(TokenKind::Arrow, "expected `->` after extern parameters");
            let ret = self.parse_type();
            self.consume_semi();
            let span = Span::new(name.span.start, self.previous().span.end);
            functions.push(Spanned::new(
                ExternFn {
                    name,
                    params,
                    ret,
                    variadic,
                },
                span,
            ));
        }
        self.expect(TokenKind::RBrace, "expected `}` to close extern block");
        let span = Span::new(start, self.previous().span.end);
        Spanned::new(ExternBlock { attrs, functions }, span)
    }

    fn parse_test(&mut self, _attrs: Vec<Attribute>) -> Spanned<TestDef> {
        let start = self.previous().span.start;
        let name = self.parse_string_literal_name("test");
        let body = self.parse_block();
        let span = Span::new(start, body.span.end);
        Spanned::new(TestDef { name, body }, span)
    }

    fn parse_bench(&mut self, _attrs: Vec<Attribute>) -> Spanned<BenchDef> {
        let start = self.previous().span.start;
        let name = self.parse_string_literal_name("bench");
        let body = self.parse_block();
        let span = Span::new(start, body.span.end);
        Spanned::new(BenchDef { name, body }, span)
    }

    fn parse_string_literal_name(&mut self, ctx: &str) -> Spanned<String> {
        match self.peek_kind() {
            TokenKind::String(lit) => {
                let tok = self.advance();
                let name = lit
                    .parts
                    .iter()
                    .filter_map(|p| match p {
                        buraaq_lexer::StringPart::Text(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .collect::<String>();
                Spanned::new(name, tok.span)
            }
            _ => {
                self.error_at_current(
                    &format!("expected string literal name for `{ctx}`"),
                    "try `test \"name\" {{ ... }}`",
                );
                Spanned::new("<error>".into(), self.current_span())
            }
        }
    }

    pub(crate) fn mark(&self) -> usize {
        self.pos
    }

    pub(crate) fn rewind(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub(crate) fn parse_block(&mut self) -> Spanned<Block> {
        let start = self.expect(TokenKind::LBrace, "expected `{` to start block").span.start;
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

    pub(crate) fn is_block_tail_expr(&mut self) -> bool {
        if !matches!(
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
                | TokenKind::String(_)
                | TokenKind::LParen
                | TokenKind::LBrace
                | TokenKind::SelfKw
                | TokenKind::Unsafe
                | TokenKind::Match
        ) {
            return false;
        }
        // Heuristic: if next tokens look like `ident =`, it's a var decl not tail expr.
        if let TokenKind::Ident(_) = self.peek_kind() {
            let next = self.peek_at(1).map(|t| t.kind.clone());
            if matches!(next, Some(TokenKind::Eq)) {
                return false;
            }
            if matches!(next, Some(TokenKind::Colon)) {
                return false;
            }
        }
        if self.check(&TokenKind::Mut) {
            return false;
        }
        true
    }

    pub(crate) fn parse_stmt(&mut self) -> Spanned<Stmt> {
        let start = self.current_span().start;
        let stmt = if self.check(&TokenKind::Fn)
            || self.check(&TokenKind::Async)
            || self.check(&TokenKind::Struct)
            || self.check(&TokenKind::Enum)
            || self.check(&TokenKind::Trait)
            || self.check(&TokenKind::Impl)
            || self.check(&TokenKind::Const)
            || self.check(&TokenKind::Type)
            || self.check(&TokenKind::Extern)
        {
            Stmt::Item(self.parse_item())
        } else if self.match_token(TokenKind::If) {
            Stmt::If(self.parse_if_stmt())
        } else if self.match_token(TokenKind::While) {
            let cond = self.parse_expr_before_block();
            let body = self.parse_block();
            let span = Span::new(start, body.span.end);
            Stmt::While(Spanned::new(WhileStmt { cond, body }, span))
        } else if self.match_token(TokenKind::Parallel) {
            self.expect(TokenKind::For, "expected `for` after `parallel`");
            Stmt::For(self.parse_for_stmt(true, start))
        } else if self.match_token(TokenKind::For) {
            Stmt::For(self.parse_for_stmt(false, start))
        } else if self.match_token(TokenKind::Match) {
            Stmt::Match(self.parse_match_stmt(start))
        } else if self.match_token(TokenKind::Return) {
            let value = if self.is_expr_start() { Some(self.parse_expr()) } else { None };
            self.consume_semi();
            let span = Span::new(start, self.previous().span.end);
            Stmt::Return(Spanned::new(ReturnStmt { value }, span))
        } else if self.match_token(TokenKind::Break) {
            let value = if self.is_expr_start() { Some(self.parse_expr()) } else { None };
            self.consume_semi();
            let span = Span::new(start, self.previous().span.end);
            Stmt::Break(Spanned::new(BreakStmt { value }, span))
        } else if self.match_token(TokenKind::Continue) {
            self.consume_semi();
            Stmt::Continue(Span::new(start, self.previous().span.end))
        } else if self.match_token(TokenKind::Defer) {
            let expr = self.parse_expr();
            self.consume_semi();
            let span = Span::new(start, self.previous().span.end);
            Stmt::Defer(expr)
        } else if self.match_token(TokenKind::Unsafe) {
            let body = self.parse_block();
            let span = Span::new(start, body.span.end);
            let bspan = body.span;
            Stmt::Unsafe(Spanned::new(body.node, bspan))
        } else if self.match_token(TokenKind::Expect) {
            let expr = self.parse_expr();
            self.consume_semi();
            let span = Span::new(start, self.previous().span.end);
            Stmt::Expect(Spanned::new(buraaq_ast::ExpectStmt { expr }, span))
        } else if self.check(&TokenKind::Mut) || self.is_var_decl_start() {
            Stmt::VarDecl(self.parse_var_decl())
        } else if self.is_expr_start() {
            let expr = self.parse_expr();
            self.consume_semi();
            Stmt::Expr(expr)
        } else {
            self.error_at_current("expected statement", "try `x = ...`, `if`, `for`, or `return`");
            self.synchronize_stmt();
            Stmt::Empty(self.current_span())
        };
        let end = self.previous().span.end;
        Spanned::new(stmt, Span::new(start, end))
    }

    fn parse_if_stmt(&mut self) -> Spanned<IfStmt> {
        let start = self.previous().span.start;
        let cond = self.parse_expr_before_block();
        let then_block = self.parse_block();
        let mut elifs = Vec::new();
        while self.match_token(TokenKind::Elif) {
            let ec = self.parse_expr_before_block();
            let eb = self.parse_block();
            let span = Span::new(ec.span.start, eb.span.end);
            elifs.push(Spanned::new(ElifBranch { cond: ec, block: eb }, span));
        }
        let else_block = if self.match_token(TokenKind::Else) {
            Some(self.parse_block())
        } else {
            None
        };
        let end = else_block
            .as_ref()
            .map(|b| b.span.end)
            .unwrap_or(then_block.span.end);
        Spanned::new(
            IfStmt {
                cond,
                then_block,
                elifs,
                else_block,
            },
            Span::new(start, end),
        )
    }

    fn parse_for_stmt(&mut self, parallel: bool, start: BytePos) -> Spanned<ForStmt> {
        let var = self.expect_ident("expected loop variable");
        self.expect(TokenKind::In, "expected `in` in for-loop");
        let iter = if self.is_expr_start() {
            let expr = self.parse_expr_before_block();
            if self.match_token(TokenKind::DotDotEq) {
                let end = self.parse_expr_before_block();
                ForIter::Range {
                    start: expr,
                    end,
                    inclusive: true,
                }
            } else if self.match_token(TokenKind::DotDot) {
                let end = self.parse_expr_before_block();
                ForIter::Range {
                    start: expr,
                    end,
                    inclusive: false,
                }
            } else {
                ForIter::In(expr)
            }
        } else {
            self.error_at_current("expected iterator expression", "provide a range like `0..10` or collection");
            ForIter::In(missing_expr(self.current_span()))
        };
        let body = self.parse_block();
        let end = body.span.end;
        Spanned::new(
            ForStmt {
                parallel,
                var,
                iter,
                body,
            },
            Span::new(start, end),
        )
    }

    fn parse_match_stmt(&mut self, start: BytePos) -> Spanned<MatchStmt> {
        let scrutinee = self.parse_expr_before_block();
        let arms = self.parse_match_arms();
        Spanned::new(
            MatchStmt { scrutinee, arms },
            Span::new(start, self.previous().span.end),
        )
    }

    pub(crate) fn parse_match_arms(&mut self) -> Vec<Spanned<MatchArm>> {
        self.expect(TokenKind::LBrace, "expected `{` after match scrutinee");
        let mut arms = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let pattern = self.parse_pattern();
            self.expect(TokenKind::FatArrow, "expected `=>` after match pattern");
            let body = if self.check(&TokenKind::LBrace) {
                MatchArmBody::Block(self.parse_block())
            } else {
                MatchArmBody::Expr(self.parse_expr())
            };
            let span = Span::new(pattern.span.start, self.previous().span.end);
            arms.push(Spanned::new(MatchArm { pattern, body }, span));
            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }
        self.match_token(TokenKind::Comma);
        self.expect(TokenKind::RBrace, "expected `}` to close match");
        arms
    }

    fn parse_var_decl(&mut self) -> Spanned<VarDecl> {
        let start = self.current_span().start;
        let mutable = self.match_token(TokenKind::Mut);
        let name = self.expect_ident("expected variable name");
        let ty = if self.match_token(TokenKind::Colon) {
            Some(self.parse_type())
        } else {
            None
        };
        let name_text = self.file.slice(name.span).trim().to_owned();
        if !self.match_token(TokenKind::Eq) {
            self.error_with_help(
                name.span,
                "expected `=` in variable declaration",
                "a binding needs `=` and an initializer",
                "add a value on the right-hand side",
                format!("{name_text} = \"Asim\""),
            );
            let missing = missing_expr(self.current_span());
            self.consume_semi();
            return Spanned::new(
                VarDecl {
                    mutable,
                    name,
                    ty,
                    init: missing,
                },
                Span::new(start, self.previous().span.end),
            );
        }
        let init = if self.is_expr_start() {
            self.parse_expr()
        } else {
            self.error_with_help(
                self.current_span(),
                "expected a value after `=`",
                "a variable binding needs an initializer",
                "add a value on the right-hand side",
                format!("{name_text} = \"Asim\""),
            );
            missing_expr(self.current_span())
        };
        self.consume_semi();
        Spanned::new(
            VarDecl {
                mutable,
                name,
                ty,
                init,
            },
            Span::new(start, self.previous().span.end),
        )
    }

    fn is_var_decl_start(&mut self) -> bool {
        if !matches!(self.peek_kind(), TokenKind::Ident(_)) {
            return false;
        }
        matches!(
            self.peek_at(1).map(|t| &t.kind),
            Some(TokenKind::Colon) | Some(TokenKind::Eq)
        )
    }

    fn parse_pattern(&mut self) -> Spanned<Pattern> {
        let start = self.current_span().start;
        let pat =         if matches!(self.peek_kind(), TokenKind::Ident(ref s) if s == "_") {
            self.advance();
            Pattern::Wild(self.previous().span)
        } else if self.match_token(TokenKind::Dot) {
            let variant = self.expect_ident("expected variant name after `.`");
            Pattern::Path(Spanned::new(
                PathPattern {
                    qual: None,
                    variant: variant.clone(),
                    kind: PathPatternKind::Unit,
                },
                Span::new(start, variant.span.end),
            ))
        } else if self.check(&TokenKind::LParen) {
            self.advance();
            let mut pats = Vec::new();
            if !self.check(&TokenKind::RParen) {
                loop {
                    pats.push(self.parse_pattern());
                    if !self.match_token(TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.expect(TokenKind::RParen, "expected `)` after tuple pattern");
            Pattern::Tuple(Spanned::new(pats, Span::new(start, self.previous().span.end)))
        } else if let TokenKind::Ident(first) = self.peek_kind().clone() {
            if self.peek_at(1).is_some_and(|t| matches!(t.kind, TokenKind::Dot)) {
                self.advance();
                self.advance();
                let variant = self.expect_ident("expected variant name");
                let kind = if self.match_token(TokenKind::LParen) {
                    let mut pats = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        loop {
                            pats.push(self.parse_pattern());
                            if !self.match_token(TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(TokenKind::RParen, "expected `)` after pattern tuple");
                    PathPatternKind::Tuple(pats)
                } else if self.match_token(TokenKind::LBrace) {
                    let mut fields = Vec::new();
                    while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
                        let fname = self.expect_ident("expected field name");
                        let sub = if self.match_token(TokenKind::Colon) {
                            Some(self.parse_pattern())
                        } else {
                            None
                        };
                        fields.push(Spanned::new(FieldPattern { name: fname, pattern: sub }, self.previous().span));
                        if !self.match_token(TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(TokenKind::RBrace, "expected `}` after struct pattern");
                    PathPatternKind::Struct(fields)
                } else {
                    PathPatternKind::Unit
                };
                Pattern::Path(Spanned::new(
                    PathPattern {
                        qual: Some(Spanned::new(first, Span::new(start, start))),
                        variant,
                        kind,
                    },
                    Span::new(start, self.previous().span.end),
                ))
            } else if self
                .peek_at(1)
                .is_some_and(|t| matches!(t.kind, TokenKind::LParen))
            {
                let variant = self.advance();
                let variant_name = if let TokenKind::Ident(name) = variant.kind {
                    Spanned::new(name, variant.span)
                } else {
                    Spanned::new("<error>".into(), variant.span)
                };
                self.expect(TokenKind::LParen, "expected `(` after constructor name");
                let mut pats = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        pats.push(self.parse_pattern());
                        if !self.match_token(TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RParen, "expected `)` after constructor pattern");
                Pattern::Path(Spanned::new(
                    PathPattern {
                        qual: None,
                        variant: variant_name,
                        kind: PathPatternKind::Tuple(pats),
                    },
                    Span::new(start, self.previous().span.end),
                ))
            } else {
                self.advance();
                Pattern::Ident(Spanned::new(first, Span::new(start, self.previous().span.end)))
            }
        } else if matches!(
            self.peek_kind(),
            TokenKind::Some | TokenKind::None | TokenKind::Ok | TokenKind::Err
        ) {
            let kw = self.advance();
            let (name, kw_span) = match kw.kind {
                TokenKind::Some => ("Some".to_string(), kw.span),
                TokenKind::None => ("None".to_string(), kw.span),
                TokenKind::Ok => ("Ok".to_string(), kw.span),
                TokenKind::Err => ("Err".to_string(), kw.span),
                _ => unreachable!(),
            };
            let kind = if self.match_token(TokenKind::LParen) {
                let mut pats = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        pats.push(self.parse_pattern());
                        if !self.match_token(TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RParen, "expected `)` after pattern tuple");
                PathPatternKind::Tuple(pats)
            } else {
                PathPatternKind::Unit
            };
            Pattern::Path(Spanned::new(
                PathPattern {
                    qual: None,
                    variant: Spanned::new(name, kw_span),
                    kind,
                },
                Span::new(start, self.previous().span.end),
            ))
        } else if self.peek_kind().is_literal() {
            Pattern::Literal(self.parse_literal())
        } else {
            self.error_at_current("expected pattern", "try `_`, a name, or `.Variant`");
            Pattern::Missing(self.current_span())
        };
        let end = self.previous().span.end.max(start);
        Spanned::new(pat, Span::new(start, end))
    }

    pub(crate) fn parse_type(&mut self) -> Spanned<Type> {
        self.parse_type_inner(0)
    }

    fn parse_type_inner(&mut self, min_union: u8) -> Spanned<Type> {
        let start = self.current_span().start;
        let mut ty = self.parse_type_primary();
        loop {
            if self.match_token(TokenKind::LBracket) {
                if self.match_token(TokenKind::RBracket) {
                    let span = Span::new(ty.span.start, self.previous().span.end);
                    let inner = ty.node;
                    ty = Spanned::new(
                        Type::Slice(Spanned::new(Box::new(inner), span)),
                        span,
                    );
                } else if self.check(&TokenKind::RBracket) {
                    self.error_at_current("expected type inside `[]`", "slice syntax is `T[]`");
                    break;
                } else {
                    let first = self.parse_type_inner(0);
                    if self.match_token(TokenKind::Semicolon) {
                        let elem = box_type(first);
                        let len = if let TokenKind::Int(n) = self.peek_kind() {
                            let tok = self.advance();
                            Spanned::new(n as u128, tok.span)
                        } else {
                            self.error_at_current("expected array length", "fixed array size must be an integer");
                            Spanned::new(0, self.current_span())
                        };
                        self.expect(TokenKind::RBracket, "expected `]` after array length");
                        let span = Span::new(ty.span.start, self.previous().span.end);
                        ty = Spanned::new(
                            Type::Array(Spanned::new(ArrayType { elem, len }, span)),
                            span,
                        );
                    } else {
                        // Generic args List[T]
                        let mut args = vec![first];
                        while self.match_token(TokenKind::Comma) {
                            args.push(self.parse_type_inner(0));
                        }
                        self.expect(TokenKind::RBracket, "expected `]` after generic type args");
                        if let Type::Named(n) = ty.node {
                            let nspan = n.span;
                            let span = Span::new(nspan.start, self.previous().span.end);
                            let generics = args.into_iter().map(|t| box_type(t)).collect();
                            ty = Spanned::new(
                                Type::Named(Spanned::new(
                                    NamedType {
                                        path: n.node.path,
                                        generics,
                                    },
                                    span,
                                )),
                                span,
                            );
                        }
                    }
                }
            } else if self.match_token(TokenKind::Pipe) && min_union == 0 {
                let mut parts = vec![ty];
                parts.push(self.parse_type_inner(1));
                while self.match_token(TokenKind::Pipe) {
                    parts.push(self.parse_type_inner(1));
                }
                let span = Span::new(start, self.previous().span.end);
                return Spanned::new(Type::Union(Spanned::new(parts, span)), span);
            } else {
                break;
            }
        }
        ty
    }

    fn parse_type_primary(&mut self) -> Spanned<Type> {
        let start = self.current_span().start;
        if self.match_token(TokenKind::Void) {
            return Spanned::new(Type::Void(self.previous().span), self.previous().span);
        }
        if self.match_token(TokenKind::Fn) {
            self.expect(TokenKind::LParen, "expected `(` in function type");
            let mut params = Vec::new();
            if !self.check(&TokenKind::RParen) {
                loop {
                    params.push(self.parse_type());
                    if !self.match_token(TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.expect(TokenKind::RParen, "expected `)` in function type");
            self.expect(TokenKind::Arrow, "expected `->` in function type");
            let ret = box_type(self.parse_type());
            let span = Span::new(start, ret.span.end);
            return Spanned::new(
                Type::Function(Spanned::new(
                    FunctionType {
                        params: params.into_iter().map(|t| box_type(t)).collect(),
                        ret,
                    },
                    span,
                )),
                span,
            );
        }
        if self.match_token(TokenKind::LParen) {
            let mut types = vec![self.parse_type()];
            while self.match_token(TokenKind::Comma) {
                types.push(self.parse_type());
            }
            self.expect(TokenKind::RParen, "expected `)` in tuple type");
            let span = Span::new(start, self.previous().span.end);
            return Spanned::new(Type::Tuple(Spanned::new(types, span)), span);
        }
        if is_type_sync(&self.peek_kind()) {
            let path = self.parse_path();
            let span = path.span;
            return Spanned::new(
                Type::Named(Spanned::new(NamedType { path, generics: vec![] }, span)),
                span,
            );
        }
        self.error_at_current("expected type", "try `int`, `text`, or a type name");
        let span = self.current_span();
        Spanned::new(
            Type::Named(Spanned::new(
                NamedType {
                    path: Spanned::new(Path { segments: vec![], span }, span),
                    generics: vec![],
                },
                span,
            )),
            span,
        )
    }

    fn path_segment_from_token(kind: &TokenKind) -> Option<String> {
        match kind {
            TokenKind::Ident(name) => Some(name.clone()),
            // Module paths may use keywords (`std.async`, etc.)
            TokenKind::Async => Some("async".into()),
            TokenKind::Type => Some("type".into()),
            _ => None,
        }
    }

    fn parse_path_segment(&mut self) -> Spanned<String> {
        if let Some(name) = Self::path_segment_from_token(&self.peek_kind()) {
            let tok = self.advance();
            return Spanned::new(name, tok.span);
        }
        self.error_at_current("expected path segment", "try an identifier");
        Spanned::new("<error>".into(), self.current_span())
    }

    fn parse_path(&mut self) -> Spanned<Path> {
        let start = self.current_span().start;
        let first = self.parse_path_segment();
        let mut segments = vec![first];
        while self.check(&TokenKind::Dot) {
            if self
                .peek_at(1)
                .is_some_and(|t| matches!(t.kind, TokenKind::LBrace))
            {
                break;
            }
            self.advance();
            segments.push(self.parse_path_segment());
        }
        let span = Span::new(start, self.previous().span.end);
        Spanned::new(Path { segments, span }, span)
    }

    fn parse_attributes(&mut self) -> Vec<Attribute> {
        let mut attrs = Vec::new();
        while self.check(&TokenKind::Hash) {
            attrs.push(self.parse_attribute());
        }
        attrs
    }

    fn parse_attribute(&mut self) -> Attribute {
        let start = self.expect(TokenKind::Hash, "expected `#`").span.start;
        self.expect(TokenKind::LBracket, "expected `[` after `#`");
        let name = self.expect_ident("expected attribute name");
        let mut args = Vec::new();
        while self.match_token(TokenKind::Comma) || self.match_token(TokenKind::LParen) {
            if self.previous().kind == TokenKind::LParen {
                if !self.check(&TokenKind::RParen) {
                    loop {
                        args.push(self.parse_attr_arg());
                        if !self.match_token(TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RParen, "expected `)` after attribute args");
                break;
            } else {
                args.push(self.parse_attr_arg());
            }
        }
        self.expect(TokenKind::RBracket, "expected `]` after attribute");
        let span = Span::new(start, self.previous().span.end);
        Attribute {
            name,
            args,
            span,
        }
    }

    fn parse_attr_arg(&mut self) -> AttrArg {
        if self.peek_kind().is_literal() {
            AttrArg::Literal(self.parse_literal())
        } else {
            AttrArg::Ident(self.expect_ident("expected attribute argument"))
        }
    }

    pub(crate) fn parse_literal(&mut self) -> Spanned<Literal> {
        // used by expr and pattern parsing
        let tok = self.advance();
        let span = tok.span;
        let lit = match tok.kind {
            TokenKind::Int(v) => Literal::Int(Spanned::new(v, span)),
            TokenKind::Float(v) => Literal::Float(Spanned::new(v, span)),
            TokenKind::True => Literal::Bool(Spanned::new(true, span)),
            TokenKind::False => Literal::Bool(Spanned::new(false, span)),
            TokenKind::Char(c) => Literal::Char(Spanned::new(c, span)),
            TokenKind::String(s) => Literal::String(Spanned::new(self.lower_string(s), span)),
            TokenKind::Bytes(b) => Literal::Bytes(Spanned::new(b, span)),
            TokenKind::None => Literal::None(span),
            TokenKind::Some => {
                self.expect(TokenKind::LParen, "expected `(` after `some`");
                let inner = self.parse_expr();
                self.expect(TokenKind::RParen, "expected `)` after `some` argument");
                Literal::Some(inner)
            }
            TokenKind::Ok => {
                self.expect(TokenKind::LParen, "expected `(` after `ok`");
                let inner = self.parse_expr();
                self.expect(TokenKind::RParen, "expected `)` after `ok` argument");
                Literal::Ok(inner)
            }
            TokenKind::Err => {
                self.expect(TokenKind::LParen, "expected `()` after `err`");
                let inner = self.parse_expr();
                self.expect(TokenKind::RParen, "expected `)` after `err` argument");
                Literal::Err(inner)
            }
            _ => {
                self.error(span, "expected literal", "invalid literal token");
                Literal::None(span)
            }
        };
        Spanned::new(lit, span)
    }

    fn lower_string(&mut self, s: buraaq_lexer::StringLit) -> StringParts {
        use buraaq_lexer::StringPart as LPart;
        let mut parts = Vec::new();
        for part in s.parts {
            match part {
                LPart::Text(t) => parts.push(StringPart::Text(t)),
                LPart::InterpRaw(raw) => {
                    let sub_file = SourceFile::new("interp.bq", raw.node.clone());
                    let sub_tokens = Lexer::new(&sub_file).tokenize();
                    let mut sub_parser = Parser::new(&sub_file, sub_tokens, self.diagnostics);
                    let expr = sub_parser.parse_expr();
                    parts.push(StringPart::Interp(expr));
                }
            }
        }
        StringParts { parts }
    }

    // --- Token helpers ---

    pub(crate) fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&self.tokens[self.tokens.len() - 1])
    }

    pub(crate) fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset)
    }

    pub(crate) fn peek_kind(&self) -> TokenKind {
        self.peek().kind.clone()
    }

    pub(crate) fn check(&self, kind: &TokenKind) -> bool {
        &self.peek().kind == kind
    }

    pub(crate) fn match_token(&mut self, kind: TokenKind) -> bool {
        if self.check(&kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(crate) fn expect(&mut self, kind: TokenKind, message: &str) -> Token {
        if self.check(&kind) {
            self.advance()
        } else {
            self.error_at_current(message, "unexpected token here");
            Token {
                kind: kind_placeholder(&kind),
                span: self.current_span(),
            }
        }
    }

    pub(crate) fn expect_ident(&mut self, message: &str) -> Spanned<String> {
        match self.peek_kind() {
            TokenKind::Ident(name) => {
                let tok = self.advance();
                Spanned::new(name, tok.span)
            }
            TokenKind::Ok => {
                let tok = self.advance();
                Spanned::new("Ok".into(), tok.span)
            }
            TokenKind::Err => {
                let tok = self.advance();
                Spanned::new("Err".into(), tok.span)
            }
            TokenKind::Some => {
                let tok = self.advance();
                Spanned::new("Some".into(), tok.span)
            }
            TokenKind::None => {
                let tok = self.advance();
                Spanned::new("None".into(), tok.span)
            }
            _ => {
                self.error_at_current(message, "expected an identifier");
                Spanned::new("<error>".into(), self.current_span())
            }
        }
    }

    pub(crate) fn advance(&mut self) -> Token {
        if !self.check(&TokenKind::Eof) {
            self.pos += 1;
        }
        self.previous().clone()
    }

    pub(crate) fn previous(&self) -> &Token {
        &self.tokens[self.pos.saturating_sub(1)]
    }

    pub(crate) fn current_span(&self) -> Span {
        self.peek().span
    }

    fn consume_semi(&mut self) {
        if !self.match_token(TokenKind::Semicolon) {
            // optional semicolons — newline serves as separator
        }
    }

    fn consume_optional_semi(&mut self) {
        self.match_token(TokenKind::Semicolon);
    }

    fn synchronize_top_level(&mut self) {
        while !self.check(&TokenKind::Eof) {
            if is_top_level_sync(&self.peek_kind()) {
                return;
            }
            self.advance();
        }
    }

    fn synchronize_stmt(&mut self) {
        while !self.check(&TokenKind::Eof) {
            if self.check(&TokenKind::RBrace) || is_stmt_sync(&self.peek_kind()) {
                return;
            }
            self.advance();
        }
    }

    pub(crate) fn error_at_current(&mut self, message: &str, label: &str) {
        self.error(self.current_span(), message, label);
    }

    pub(crate) fn error(&self, span: Span, message: &str, label: &str) {
        self.diagnostics.emit(
            self.file,
            Diagnostic::error(message)
                .with_code("E0100")
                .with_label(Label::primary(span, label)),
        );
    }

    pub(crate) fn error_with_help(
        &self,
        span: Span,
        message: &str,
        reason: &str,
        help_msg: &str,
        suggestion: String,
    ) {
        self.diagnostics.emit(
            self.file,
            Diagnostic::error(message)
                .with_code("E0102")
                .with_reason(reason)
                .with_label(Label::primary(span, "a value is required here"))
                .with_help(Help {
                    message: help_msg.into(),
                    suggestion: Some(Suggestion {
                        replacement: suggestion,
                        span: None,
                    }),
                }),
        );
    }
}

fn kind_placeholder(kind: &TokenKind) -> TokenKind {
    match kind {
        TokenKind::Ident(_) => TokenKind::Ident("<error>".into()),
        TokenKind::Int(_) => TokenKind::Int(0),
        TokenKind::RParen => TokenKind::RParen,
        TokenKind::RBrace => TokenKind::RBrace,
        TokenKind::RBracket => TokenKind::RBracket,
        TokenKind::Semicolon => TokenKind::Semicolon,
        _ => TokenKind::Ident("<error>".into()),
    }
}
