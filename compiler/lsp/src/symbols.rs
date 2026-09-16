use buraaq_ast::{Block, Expr, Item, Program, Stmt, StructDef, Type};
use buraaq_semantic::{DefKind, DefMap};
use buraaq_source::{SourceFile, Span, Spanned};
use tower_lsp::lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, DocumentSymbol,
    SymbolInformation, SymbolKind, Url,
};

use crate::convert::{document_symbol, file_uri, lsp_range, workspace_symbol};

type ExprNode = buraaq_ast::ExprNode;

pub fn document_symbols(file: &SourceFile, program: &Program) -> Vec<DocumentSymbol> {
    let mut out = Vec::new();
    for item in &program.items {
        match &item.node {
            Item::Function(f) => {
                out.push(document_symbol(
                    &f.node.name.node,
                    SymbolKind::FUNCTION,
                    range_from_span(file, f.span),
                    range_from_span(file, f.node.name.span),
                ));
            }
            Item::Struct(s) => out.push(struct_symbol(file, &s.node, s.span)),
            Item::Enum(e) => {
                out.push(document_symbol(
                    &e.node.name.node,
                    SymbolKind::ENUM,
                    range_from_span(file, e.span),
                    range_from_span(file, e.node.name.span),
                ));
            }
            Item::Trait(t) => {
                out.push(document_symbol(
                    &t.node.name.node,
                    SymbolKind::INTERFACE,
                    range_from_span(file, t.span),
                    range_from_span(file, t.node.name.span),
                ));
            }
            Item::Const(c) => {
                out.push(document_symbol(
                    &c.node.name.node,
                    SymbolKind::CONSTANT,
                    range_from_span(file, c.span),
                    range_from_span(file, c.node.name.span),
                ));
            }
            Item::TypeAlias(t) => {
                out.push(document_symbol(
                    &t.node.name.node,
                    SymbolKind::TYPE_PARAMETER,
                    range_from_span(file, t.span),
                    range_from_span(file, t.node.name.span),
                ));
            }
            _ => {}
        }
    }
    out
}

fn struct_symbol(file: &SourceFile, s: &StructDef, span: Span) -> DocumentSymbol {
    let mut children = Vec::new();
    for field in &s.fields {
        children.push(document_symbol(
            &field.node.name.node,
            SymbolKind::FIELD,
            range_from_span(file, field.span),
            range_from_span(file, field.node.name.span),
        ));
    }
    for method in &s.methods {
        if let buraaq_ast::Method::Function(f) = &method.node {
            children.push(document_symbol(
                &f.node.name.node,
                SymbolKind::METHOD,
                range_from_span(file, method.span),
                range_from_span(file, f.node.name.span),
            ));
        }
    }
    DocumentSymbol {
        name: s.name.node.clone(),
        detail: Some("struct".into()),
        kind: SymbolKind::STRUCT,
        tags: None,
        deprecated: None,
        range: range_from_span(file, span),
        selection_range: range_from_span(file, s.name.span),
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }
}

pub fn workspace_symbols(file: &SourceFile, program: &Program, uri: Url) -> Vec<SymbolInformation> {
    document_symbols(file, program)
        .into_iter()
        .map(|sym| workspace_symbol(&sym.name, sym.kind, uri.clone(), sym.selection_range))
        .collect()
}

pub fn ident_at_offset(program: &Program, offset: u32) -> Option<(String, Span)> {
    let mut best: Option<(String, Span)> = None;
    walk_program(program, &mut |name, span| {
        if offset >= span.start.0 && offset <= span.end.0 {
            best = Some((name.to_string(), span));
        }
    });
    best
}

pub fn definition_of(name: &str, program: &Program, defs: &DefMap) -> Option<Span> {
    if defs.lookup(name).is_some() {
        find_def_span(program, name)
    } else {
        None
    }
}

pub fn references_of(name: &str, program: &Program) -> Vec<Span> {
    let mut spans = Vec::new();
    walk_program(program, &mut |ident, span| {
        if ident == name {
            spans.push(span);
        }
    });
    spans
}

pub fn call_hierarchy_item(
    file: &SourceFile,
    program: &Program,
    offset: u32,
) -> Option<CallHierarchyItem> {
    let (name, span) = ident_at_offset(program, offset)?;
    let def_span = find_def_span(program, &name)?;
    Some(CallHierarchyItem {
        name,
        kind: SymbolKind::FUNCTION,
        tags: None,
        detail: None,
        uri: file_uri(file),
        range: range_from_span(file, def_span),
        selection_range: range_from_span(file, span),
        data: None,
    })
}

pub fn outgoing_calls(
    file: &SourceFile,
    program: &Program,
    item: &CallHierarchyItem,
) -> Vec<CallHierarchyOutgoingCall> {
    let mut out = Vec::new();
    if let Some(body) = function_body_by_name(program, &item.name) {
        walk_exprs_in_block(body, &mut |expr| {
            if let Expr::Call(call) = expr.node.as_ref() {
                if let Expr::Ident(callee) = call.node.callee.node.as_ref() {
                    if let Some(span) = find_def_span(program, &callee.node) {
                        out.push(CallHierarchyOutgoingCall {
                            to: CallHierarchyItem {
                                name: callee.node.clone(),
                                kind: SymbolKind::FUNCTION,
                                tags: None,
                                detail: None,
                                uri: file_uri(file),
                                range: range_from_span(file, span),
                                selection_range: range_from_span(file, call.node.callee.span),
                                data: None,
                            },
                            from_ranges: vec![range_from_span(file, call.span)],
                        });
                    }
                }
            }
        });
    }
    out
}

pub fn incoming_calls(
    file: &SourceFile,
    program: &Program,
    item: &CallHierarchyItem,
) -> Vec<CallHierarchyIncomingCall> {
    let mut out = Vec::new();
    for top in &program.items {
        let Item::Function(item_fn) = &top.node else {
            continue;
        };
        walk_exprs_in_block(&item_fn.node.body, &mut |expr| {
            if let Expr::Call(call) = expr.node.as_ref() {
                if let Expr::Ident(callee) = call.node.callee.node.as_ref() {
                    if callee.node == item.name {
                        out.push(CallHierarchyIncomingCall {
                            from: CallHierarchyItem {
                                name: item_fn.node.name.node.clone(),
                                kind: SymbolKind::FUNCTION,
                                tags: None,
                                detail: None,
                                uri: file_uri(file),
                                range: range_from_span(file, item_fn.span),
                                selection_range: range_from_span(file, item_fn.node.name.span),
                                data: None,
                            },
                            from_ranges: vec![range_from_span(file, call.span)],
                        });
                    }
                }
            }
        });
    }
    out
}

pub fn hover_for_name(name: &str, defs: &DefMap) -> Option<(String, String)> {
    let entry = defs.lookup(name)?;
    let kind = match entry.kind {
        DefKind::Function | DefKind::ImplMethod | DefKind::ExternFn => "function",
        DefKind::Struct => "struct",
        DefKind::Enum => "enum",
        DefKind::Trait => "trait",
        DefKind::TypeAlias => "type alias",
        DefKind::Const => "const",
        DefKind::Builtin => "builtin",
    };
    Some((name.to_string(), format!("{kind} `{name}`")))
}

pub fn completion_items(defs: &DefMap) -> Vec<String> {
    let mut names: Vec<String> = defs.entries().iter().map(|e| e.name.clone()).collect();
    names.sort();
    names.dedup();
    names
}

pub fn keywords() -> &'static [&'static str] {
    &[
        "async", "await", "break", "const", "continue", "copy", "defer", "drop", "else", "elif",
        "enum", "extern", "false", "fn", "for", "give", "if", "impl", "in", "match", "module",
        "mut", "parallel", "pub", "raise", "ref", "return", "self", "spawn", "struct", "throws",
        "trait", "true", "type", "unsafe", "use", "void", "while", "as", "test", "bench", "expect",
    ]
}

pub fn signature_for_call(program: &Program, callee: &str) -> Option<(String, Vec<String>)> {
    for item in &program.items {
        if let Item::Function(f) = &item.node {
            if f.node.name.node == callee {
                let params: Vec<String> = f
                    .node
                    .params
                    .iter()
                    .map(|p| format!("{}: {}", p.node.name.node, type_display(&p.node.ty.node)))
                    .collect();
                return Some((callee.to_string(), params));
            }
        }
    }
    None
}

fn type_display(ty: &Type) -> String {
    match ty {
        Type::Named(n) => n
            .node
            .path
            .node
            .segments
            .iter()
            .map(|s| s.node.as_str())
            .collect::<Vec<_>>()
            .join("::"),
        Type::Function(f) => {
            let params: Vec<_> = f.node.params.iter().map(|t| type_display(&t.node)).collect();
            format!("fn({}) -> {}", params.join(", "), type_display(&f.node.ret.node))
        }
        Type::Tuple(ts) => format!(
            "({})",
            ts.node
                .iter()
                .map(|t| type_display(&t.node))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Type::Void(_) => "void".into(),
        Type::Infer(_) => "_".into(),
        Type::Slice(s) => format!("[]{}", type_display(&s.node)),
        Type::Array(a) => format!(
            "[{}; {}]",
            type_display(&a.node.elem.node),
            a.node.len.node
        ),
        Type::Union(u) => u
            .node
            .iter()
            .map(|t| type_display(&t.node))
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

fn range_from_span(file: &SourceFile, span: Span) -> tower_lsp::lsp_types::Range {
    lsp_range(file, span.start.0, span.end.0)
}

fn find_def_span(program: &Program, name: &str) -> Option<Span> {
    for item in &program.items {
        match &item.node {
            Item::Function(f) if f.node.name.node == name => return Some(f.node.name.span),
            Item::Struct(s) => {
                if s.node.name.node == name {
                    return Some(s.node.name.span);
                }
                for m in &s.node.methods {
                    if let buraaq_ast::Method::Function(f) = &m.node {
                        if f.node.name.node == name {
                            return Some(f.node.name.span);
                        }
                    }
                }
            }
            Item::Enum(e) if e.node.name.node == name => return Some(e.node.name.span),
            Item::Trait(t) if t.node.name.node == name => return Some(t.node.name.span),
            Item::Const(c) if c.node.name.node == name => return Some(c.node.name.span),
            Item::TypeAlias(t) if t.node.name.node == name => return Some(t.node.name.span),
            Item::Impl(i) => {
                for m in &i.node.methods {
                    if let buraaq_ast::Method::Function(f) = &m.node {
                        if f.node.name.node == name {
                            return Some(f.node.name.span);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn function_body_by_name<'a>(program: &'a Program, name: &str) -> Option<&'a Spanned<Block>> {
    for item in &program.items {
        if let Item::Function(f) = &item.node {
            if f.node.name.node == name {
                return Some(&f.node.body);
            }
        }
    }
    None
}

fn walk_program(program: &Program, f: &mut dyn FnMut(&str, Span)) {
    for item in &program.items {
        match &item.node {
            Item::Function(func) => walk_block_idents(&func.node.body, f),
            Item::Struct(s) => {
                for m in &s.node.methods {
                    if let buraaq_ast::Method::Function(func) = &m.node {
                        walk_block_idents(&func.node.body, f);
                    }
                }
            }
            Item::Impl(i) => {
                for m in &i.node.methods {
                    if let buraaq_ast::Method::Function(func) = &m.node {
                        walk_block_idents(&func.node.body, f);
                    }
                }
            }
            _ => {}
        }
    }
}

fn walk_block_idents(block: &Spanned<Block>, f: &mut dyn FnMut(&str, Span)) {
    for stmt in &block.node.stmts {
        walk_stmt_idents(stmt, f);
    }
    if let Some(tail) = &block.node.tail {
        walk_expr_idents(tail, f);
    }
}

fn walk_stmt_idents(stmt: &Spanned<Stmt>, f: &mut dyn FnMut(&str, Span)) {
    match &stmt.node {
        Stmt::VarDecl(v) => {
            f(&v.node.name.node, v.node.name.span);
            walk_expr_idents(&v.node.init, f);
        }
        Stmt::Expr(e) => walk_expr_idents(e, f),
        Stmt::Return(r) => {
            if let Some(v) = &r.node.value {
                walk_expr_idents(v, f);
            }
        }
        Stmt::If(i) => {
            walk_expr_idents(&i.node.cond, f);
            walk_block_idents(&i.node.then_block, f);
            for elif in &i.node.elifs {
                walk_expr_idents(&elif.node.cond, f);
                walk_block_idents(&elif.node.block, f);
            }
            if let Some(el) = &i.node.else_block {
                walk_block_idents(el, f);
            }
        }
        Stmt::While(w) => {
            walk_expr_idents(&w.node.cond, f);
            walk_block_idents(&w.node.body, f);
        }
        Stmt::For(fr) => walk_block_idents(&fr.node.body, f),
        Stmt::Match(m) => {
            walk_expr_idents(&m.node.scrutinee, f);
            for arm in &m.node.arms {
                match &arm.node.body {
                    buraaq_ast::MatchArmBody::Expr(e) => walk_expr_idents(e, f),
                    buraaq_ast::MatchArmBody::Block(b) => walk_block_idents(b, f),
                }
            }
        }
        Stmt::Unsafe(b) => walk_block_idents(b, f),
        Stmt::Defer(e) => walk_expr_idents(e, f),
        Stmt::Expect(e) => walk_expr_idents(&e.node.expr, f),
        _ => {}
    }
}

fn walk_expr_idents(expr: &Spanned<ExprNode>, f: &mut dyn FnMut(&str, Span)) {
    if let Expr::Ident(name) = expr.node.as_ref() {
        f(&name.node, name.span);
    }
    walk_expr_children(expr, &mut |child| walk_expr_idents(child, f));
}

fn walk_exprs_in_block(block: &Spanned<Block>, f: &mut dyn FnMut(&Spanned<ExprNode>)) {
    for stmt in &block.node.stmts {
        walk_stmt_exprs(stmt, f);
    }
    if let Some(tail) = &block.node.tail {
        walk_expr(tail, f);
    }
}

fn walk_stmt_exprs(stmt: &Spanned<Stmt>, f: &mut dyn FnMut(&Spanned<ExprNode>)) {
    match &stmt.node {
        Stmt::VarDecl(v) => walk_expr(&v.node.init, f),
        Stmt::Expr(e) => walk_expr(e, f),
        Stmt::Return(r) => {
            if let Some(v) = &r.node.value {
                walk_expr(v, f);
            }
        }
        Stmt::If(i) => {
            walk_expr(&i.node.cond, f);
            walk_exprs_in_block(&i.node.then_block, f);
            for elif in &i.node.elifs {
                walk_expr(&elif.node.cond, f);
                walk_exprs_in_block(&elif.node.block, f);
            }
            if let Some(el) = &i.node.else_block {
                walk_exprs_in_block(el, f);
            }
        }
        Stmt::While(w) => {
            walk_expr(&w.node.cond, f);
            walk_exprs_in_block(&w.node.body, f);
        }
        Stmt::For(fr) => walk_exprs_in_block(&fr.node.body, f),
        Stmt::Match(m) => {
            walk_expr(&m.node.scrutinee, f);
            for arm in &m.node.arms {
                match &arm.node.body {
                    buraaq_ast::MatchArmBody::Expr(e) => walk_expr(e, f),
                    buraaq_ast::MatchArmBody::Block(b) => walk_exprs_in_block(b, f),
                }
            }
        }
        Stmt::Unsafe(b) => walk_exprs_in_block(b, f),
        Stmt::Defer(e) => walk_expr(e, f),
        Stmt::Expect(e) => walk_expr(&e.node.expr, f),
        _ => {}
    }
}

fn walk_expr(expr: &Spanned<ExprNode>, f: &mut dyn FnMut(&Spanned<ExprNode>)) {
    f(expr);
    walk_expr_children(expr, &mut |child| walk_expr(child, f));
}

fn walk_expr_children(expr: &Spanned<ExprNode>, f: &mut dyn FnMut(&Spanned<ExprNode>)) {
    match expr.node.as_ref() {
        Expr::Call(c) => {
            f(&c.node.callee);
            for a in &c.node.args {
                f(a);
            }
        }
        Expr::MethodCall(m) => {
            f(&m.node.receiver);
            for a in &m.node.args {
                f(a);
            }
        }
        Expr::Binary(b) => {
            f(&b.node.left);
            f(&b.node.right);
        }
        Expr::Unary(u) => f(&u.node.expr),
        Expr::Assign(a) => {
            f(&a.node.target);
            f(&a.node.value);
        }
        Expr::Field(fld) => f(&fld.node.base),
        Expr::Index(idx) => {
            f(&idx.node.base);
            f(&idx.node.index);
        }
        Expr::Paren(p) => f(p),
        Expr::Block(b) => walk_exprs_in_block(b, f),
        Expr::If(i) => {
            f(&i.node.cond);
            walk_exprs_in_block(&i.node.then_block, f);
            walk_exprs_in_block(&i.node.else_block, f);
        }
        Expr::Match(m) => {
            f(&m.node.scrutinee);
            for arm in &m.node.arms {
                match &arm.node.body {
                    buraaq_ast::MatchArmBody::Expr(e) => f(e),
                    buraaq_ast::MatchArmBody::Block(b) => walk_exprs_in_block(b, f),
                }
            }
        }
        Expr::Await(a) => f(a),
        Expr::Try(t) => f(t),
        _ => {}
    }
}
