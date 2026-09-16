use std::collections::{HashMap, HashSet};

use buraaq_ast::{
    BinOp as AstBinOp, Block, Expr, ForIter, Function, Item, Literal, Method, MethodFn, Program,
    SpawnExpr, Stmt, StringPart, StructFill, UnaryOp,
};
use buraaq_source::Spanned;
use buraaq_types::{TypeInterner, Ty, TyKind};
use thiserror::Error;

use crate::{
    BasicBlock, BinOp, Constant, LocalId, MirFunction, MirLocal, MirModule, MirStruct,
    MirTy, Operand, Rvalue, Statement, Terminator, UnOp,
};

#[derive(Debug, Error)]
pub enum LowerError {
    #[error("lowering error: {0}")]
    Msg(String),
}

pub struct LowerUnit<'a> {
    pub program: &'a Program,
    pub module_name: &'a str,
}

pub fn lower_program(program: &Program) -> Result<MirModule, LowerError> {
    lower_units(&[LowerUnit {
        program,
        module_name: "",
    }])
}

pub fn lower_units(units: &[LowerUnit]) -> Result<MirModule, LowerError> {
    let multi = units.len() > 1;
    let mut ctx = LowerCtx::new();
    for unit in units {
        for item in &unit.program.items {
            match &item.node {
                Item::Struct(s) => ctx.collect_struct(&s.node),
                Item::Enum(e) => {
                    let variants: Vec<(String, Vec<MirTy>)> = e
                        .node
                        .variants
                        .iter()
                        .map(|v| match &v.node {
                            buraaq_ast::EnumVariant::Unit(name) => (name.node.clone(), vec![]),
                            buraaq_ast::EnumVariant::Tuple(name, _) => {
                                (name.node.clone(), vec![])
                            }
                            buraaq_ast::EnumVariant::Struct(name, _) => {
                                (name.node.clone(), vec![])
                            }
                        })
                        .collect();
                    ctx.enum_variants.insert(
                        e.node.name.node.clone(),
                        variants.iter().map(|(n, _)| n.clone()).collect(),
                    );
                    ctx.module.enums.push(crate::MirEnum {
                        name: e.node.name.node.clone(),
                        variants,
                    });
                }
                _ => {}
            }
        }
    }
    for unit in units {
        for item in &unit.program.items {
            match &item.node {
                Item::Function(f) => {
                    let src = f.node.name.node.clone();
                    ctx.fn_names.insert(src.clone());
                    if !f.node.generics.is_empty() {
                        ctx.generic_templates.insert(src.clone(), f.node.clone());
                        ctx.generic_sigs.insert(src, generic_sig(&f.node));
                        continue;
                    }
                    let llvm = if !multi || f.node.pub_ || src == "main" || unit.module_name.is_empty()
                    {
                        src.clone()
                    } else {
                        format!("{}__{src}", unit.module_name.replace('.', "_"))
                    };
                    let params: Vec<MirTy> = f
                        .node
                        .params
                        .iter()
                        .map(|p| wrap_param_ty(ctx.ast_ty(&p.node.ty), p.node.by_ref))
                        .collect();
                    let ret = f
                        .node
                        .ret
                        .as_ref()
                        .map(|t| ctx.ast_ty(t))
                        .unwrap_or(MirTy::Void);
                    ctx.fn_sigs.insert(src.clone(), (params.clone(), ret.clone()));
                    ctx.fn_sigs.insert(llvm.clone(), (params, ret));
                    ctx.name_map.insert(src, llvm);
                }
                Item::Extern(ex) => {
                    for ef in &ex.node.functions {
                        let name = ef.node.name.node.clone();
                        let params: Vec<MirTy> =
                            ef.node.params.iter().map(|p| ctx.ast_ty(&p.node.ty)).collect();
                        let ret = ctx.ast_ty(&ef.node.ret);
                        ctx.fn_names.insert(name.clone());
                        ctx.extern_fns.insert(name.clone());
                        ctx.fn_sigs.insert(name.clone(), (params.clone(), ret.clone()));
                        ctx.module.externs.push(crate::MirExtern {
                            name,
                            params,
                            ret,
                            variadic: ef.node.variadic,
                        });
                    }
                }
                _ => {}
            }
        }
    }
    for (struct_name, method) in &ctx.methods.clone() {
        let name = format!("{struct_name}_{}", method.name.node);
        let mut params = vec![MirTy::Struct {
            name: struct_name.clone(),
        }];
        params.extend(method.params.iter().map(|p| ctx.ast_ty(&p.node.ty)));
        let ret = method
            .ret
            .as_ref()
            .map(|t| ctx.ast_ty(t))
            .unwrap_or(MirTy::Void);
        ctx.fn_names.insert(name.clone());
        ctx.fn_sigs.insert(name, (params, ret));
    }
    for unit in units {
        for item in &unit.program.items {
            if let Item::Function(f) = &item.node {
                if f.node.generics.is_empty() {
                    ctx.lower_function(&f.node)?;
                }
            }
        }
    }
    let methods = ctx.methods.clone();
    for (struct_name, method) in methods {
        ctx.lower_method(&struct_name, &method)?;
    }
    ctx.lower_pending_specializations()?;
    Ok(ctx.module)
}

fn wrap_param_ty(ty: MirTy, by_ref: buraaq_ast::ParamRef) -> MirTy {
    match by_ref {
        buraaq_ast::ParamRef::None => ty,
        buraaq_ast::ParamRef::Ref => MirTy::Ref {
            mut_: false,
            inner: Box::new(ty),
        },
        buraaq_ast::ParamRef::RefMut => MirTy::Ref {
            mut_: true,
            inner: Box::new(ty),
        },
    }
}

fn spec_fn_name(src: &str, tys: &[MirTy]) -> String {
    let suffix: Vec<String> = tys.iter().map(MirTy::mangle).collect();
    format!("{src}__{}", suffix.join("_"))
}

fn named_type_name(ty: &Spanned<buraaq_ast::Type>) -> Option<String> {
    if let buraaq_ast::Type::Named(n) = &ty.node {
        n.node.path.node.segments.last().map(|s| s.node.clone())
    } else {
        None
    }
}

#[derive(Clone, Debug)]
struct GenericSig {
    type_param_count: usize,
    ret_is_param: Option<usize>,
}

fn generic_sig(f: &Function) -> GenericSig {
    let names: Vec<String> = f
        .generics
        .iter()
        .map(|g| g.node.name.node.clone())
        .collect();
    let ret_is_param = f
        .ret
        .as_ref()
        .and_then(named_type_name)
        .and_then(|n| names.iter().position(|p| *p == n));
    GenericSig {
        type_param_count: names.len(),
        ret_is_param,
    }
}

struct LowerCtx {
    module: MirModule,
    struct_defs: HashMap<String, MirStruct>,
    fn_names: HashSet<String>,
    name_map: HashMap<String, String>,
    generic_templates: HashMap<String, Function>,
    generic_sigs: HashMap<String, GenericSig>,
    pending_specs: Vec<(String, Vec<MirTy>)>,
    done_specs: HashSet<String>,
    current_subst: HashMap<String, MirTy>,
    interner: TypeInterner,
    fn_sigs: HashMap<String, (Vec<MirTy>, MirTy)>,
    methods: Vec<(String, MethodFn)>,
    enum_variants: HashMap<String, Vec<String>>,
    extern_fns: HashSet<String>,
}

// ast_ty requires &mut self on LowerCtx

#[derive(Clone, Copy)]
struct LoopFrame {
    /// Continue target (`while` header or `for` step).
    cont: u32,
    exit: u32,
    local_start: u32,
    defer_mark: usize,
}

struct FnBuilder {
    name: String,
    params: Vec<(String, MirTy)>,
    return_ty: MirTy,
    locals: Vec<MirLocal>,
    blocks: Vec<BasicBlock>,
    scopes: Vec<HashMap<String, LocalId>>,
    local_tys: HashMap<LocalId, MirTy>,
    next_local: LocalId,
    next_bb: u32,
    current_bb: BlockId,
    temp: u32,
    is_entry: bool,
    fn_names: HashSet<String>,
    name_map: HashMap<String, String>,
    generic_sigs: HashMap<String, GenericSig>,
    pending_specs: Vec<(String, Vec<MirTy>)>,
    defers: Vec<Spanned<buraaq_ast::ExprNode>>,
    pending_spawns: Vec<(String, Block)>,
    spawn_id: u32,
    fn_sigs: HashMap<String, (Vec<MirTy>, MirTy)>,
    loop_stack: Vec<LoopFrame>,
    enum_variants: HashMap<String, Vec<String>>,
}

type BlockId = u32;

impl LowerCtx {
    fn new() -> Self {
        Self {
            module: MirModule {
                name: "main".into(),
                ..Default::default()
            },
            struct_defs: HashMap::new(),
            fn_names: HashSet::new(),
            name_map: HashMap::new(),
            generic_templates: HashMap::new(),
            generic_sigs: HashMap::new(),
            pending_specs: Vec::new(),
            done_specs: HashSet::new(),
            current_subst: HashMap::new(),
            interner: TypeInterner::new(),
            fn_sigs: HashMap::new(),
            methods: Vec::new(),
            enum_variants: HashMap::new(),
            extern_fns: HashSet::new(),
        }
    }

    fn collect_struct(&mut self, s: &buraaq_ast::StructDef) {
        // mut self: ast_ty needs &mut interner
        let st = MirStruct {
            name: s.name.node.clone(),
            fields: s
                .fields
                .iter()
                .map(|f| (f.node.name.node.clone(), self.ast_ty(&f.node.ty)))
                .collect(),
        };
        self.struct_defs.insert(st.name.clone(), st.clone());
        self.module.structs.push(st);
        for m in &s.methods {
            match &m.node {
                Method::Function(mf) => {
                    self.methods.push((s.name.node.clone(), mf.node.clone()));
                }
                Method::Drop(body) => {
                    self.methods.push((
                        s.name.node.clone(),
                        MethodFn {
                            name: Spanned::new("drop".into(), body.span),
                            receiver: buraaq_ast::Receiver::SelfValue,
                            params: vec![],
                            ret: None,
                            throws: None,
                            body: body.clone(),
                        },
                    ));
                }
            }
        }
    }

    fn ast_ty(&mut self, ty: &Spanned<buraaq_ast::Type>) -> MirTy {
        if let buraaq_ast::Type::Named(n) = &ty.node {
            let name = n
                .node
                .path
                .node
                .segments
                .last()
                .map(|s| s.node.clone())
                .unwrap_or_default();
            if let Some(mapped) = self.current_subst.get(&name) {
                return mapped.clone();
            }
            if self.struct_defs.contains_key(&name) {
                return MirTy::Struct { name };
            }
            if self.enum_variants.contains_key(&name) {
                return MirTy::Enum { name };
            }
            if let Some(k) = buraaq_types::resolve_type_name(&name) {
                let ty = self.interner.intern(k);
                return self.ty_to_mir(ty);
            }
        }
        let t = buraaq_types::ast_type_to_ty(
            ty,
            &mut self.interner,
            &|name| {
                if self.struct_defs.contains_key(name) {
                    Some(buraaq_types::DefId(0))
                } else {
                    None
                }
            },
        );
        self.ty_to_mir(t)
    }

    fn ty_to_mir(&self, ty: Ty) -> MirTy {
        match self.interner.kind(ty) {
            TyKind::Void => MirTy::Void,
            TyKind::Bool => MirTy::Bool,
            TyKind::Int(buraaq_types::IntKind::I64)
            | TyKind::Int(buraaq_types::IntKind::U64) => MirTy::I64,
            TyKind::Int(_) => MirTy::I32,
            TyKind::Float(buraaq_types::FloatKind::F32) => MirTy::F32,
            TyKind::Float(_) => MirTy::F64,
            TyKind::Text => MirTy::Text,
            TyKind::Bytes => MirTy::Bytes,
            TyKind::Named { def, .. } => {
                let name = format!("T{}", def.0);
                MirTy::Struct { name }
            }
            TyKind::Slice(elem) => MirTy::Slice {
                elem: Box::new(self.ty_to_mir(*elem)),
            },
            TyKind::Array { elem, len } => MirTy::Array {
                elem: Box::new(self.ty_to_mir(*elem)),
                len: *len as usize,
            },
            TyKind::Ref { mut_, inner, .. } => MirTy::Ref {
                mut_: *mut_,
                inner: Box::new(self.ty_to_mir(*inner)),
            },
            TyKind::RawPtr { inner, .. } => MirTy::ptr_to(self.ty_to_mir(*inner)),
            TyKind::Function { params, ret, .. } => MirTy::FnPtr {
                params: params.iter().map(|&p| self.ty_to_mir(p)).collect(),
                ret: Box::new(self.ty_to_mir(*ret)),
            },
            _ => MirTy::I32,
        }
    }

    fn lower_function(&mut self, f: &Function) -> Result<(), LowerError> {
        let src_name = f.name.node.clone();
        let name = self
            .name_map
            .get(&src_name)
            .cloned()
            .unwrap_or_else(|| src_name.clone());
        let is_entry = src_name == "main";
        let return_ty = f
            .ret
            .as_ref()
            .map(|t| self.ast_ty(t))
            .unwrap_or(MirTy::Void);

        let mut fb = FnBuilder::new(
            name.clone(),
            return_ty.clone(),
            is_entry,
            &self.fn_names,
            &self.name_map,
            &self.generic_sigs,
            &self.fn_sigs,
            &self.enum_variants,
        );
        for p in &f.params {
            let ty = wrap_param_ty(self.ast_ty(&p.node.ty), p.node.by_ref);
            fb.declare_param(p.node.name.node.clone(), ty);
        }

        fb.start_block();
        fb.lower_block(&f.body.node, &self.struct_defs)?;

        if matches!(fb.current_terminator(), None | Some(Terminator::Unreachable)) {
            fb.flush_defers(&self.struct_defs)?;
            fb.terminate(implicit_return(&return_ty));
        }

        self.pending_specs.extend(std::mem::take(&mut fb.pending_specs));
        let spawns = std::mem::take(&mut fb.pending_spawns);
        self.module.functions.push(fb.finish());
        for (name, body) in spawns {
            self.lower_spawn_fn(&name, &body)?;
        }
        Ok(())
    }

    fn lower_spawn_fn(&mut self, name: &str, body: &Block) -> Result<(), LowerError> {
        let mut fb = FnBuilder::new(
            name.to_string(),
            MirTy::Void,
            false,
            &self.fn_names,
            &self.name_map,
            &self.generic_sigs,
            &self.fn_sigs,
            &self.enum_variants,
        );
        fb.declare_param("_arg".into(), MirTy::ptr_to(MirTy::I8));
        fb.start_block();
        fb.lower_block(body, &self.struct_defs)?;
        if matches!(fb.current_terminator(), None | Some(Terminator::Unreachable)) {
            fb.flush_defers(&self.struct_defs)?;
            fb.terminate(Terminator::Return(None));
        }
        self.pending_specs.extend(std::mem::take(&mut fb.pending_specs));
        let nested = std::mem::take(&mut fb.pending_spawns);
        self.module.functions.push(fb.finish());
        for (n, b) in nested {
            self.lower_spawn_fn(&n, &b)?;
        }
        Ok(())
    }

    fn lower_pending_specializations(&mut self) -> Result<(), LowerError> {
        while let Some((src, tys)) = self.pending_specs.pop() {
            let llvm = spec_fn_name(&src, &tys);
            if !self.done_specs.insert(llvm.clone()) {
                continue;
            }
            let template = self
                .generic_templates
                .get(&src)
                .cloned()
                .ok_or_else(|| LowerError::Msg(format!("missing generic template `{src}`")))?;
            let mut subst = HashMap::new();
            for (i, g) in template.generics.iter().enumerate() {
                let ty = tys.get(i).cloned().unwrap_or(MirTy::I32);
                subst.insert(g.node.name.node.clone(), ty);
            }
            self.current_subst = subst;
            let saved_name = template.name.node.clone();
            let mut specialized = template;
            specialized.name.node = llvm;
            specialized.generics.clear();
            self.lower_function(&specialized)?;
            self.current_subst.clear();
            let _ = saved_name;
        }
        Ok(())
    }

    fn lower_method(&mut self, struct_name: &str, m: &MethodFn) -> Result<(), LowerError> {
        let name = format!("{struct_name}_{}", m.name.node);
        let return_ty = m
            .ret
            .as_ref()
            .map(|t| self.ast_ty(t))
            .unwrap_or(MirTy::Void);
        let mut fb = FnBuilder::new(
            name,
            return_ty.clone(),
            false,
            &self.fn_names,
            &self.name_map,
            &self.generic_sigs,
            &self.fn_sigs,
            &self.enum_variants,
        );
        fb.declare_param(
            "self".into(),
            MirTy::Struct {
                name: struct_name.into(),
            },
        );
        for p in &m.params {
            fb.declare_param(p.node.name.node.clone(), self.ast_ty(&p.node.ty));
        }
        fb.start_block();
        fb.lower_block(&m.body.node, &self.struct_defs)?;
        if matches!(fb.current_terminator(), None | Some(Terminator::Unreachable)) {
            fb.flush_defers(&self.struct_defs)?;
            fb.terminate(implicit_return(&return_ty));
        }
        self.pending_specs.extend(std::mem::take(&mut fb.pending_specs));
        self.module.functions.push(fb.finish());
        Ok(())
    }
}

impl FnBuilder {
    fn new(
        name: String,
        return_ty: MirTy,
        is_entry: bool,
        fn_names: &HashSet<String>,
        name_map: &HashMap<String, String>,
        generic_sigs: &HashMap<String, GenericSig>,
        fn_sigs: &HashMap<String, (Vec<MirTy>, MirTy)>,
        enum_variants: &HashMap<String, Vec<String>>,
    ) -> Self {
        Self {
            name,
            params: Vec::new(),
            return_ty,
            locals: Vec::new(),
            blocks: Vec::new(),
            scopes: vec![HashMap::new()],
            local_tys: HashMap::new(),
            next_local: 0,
            next_bb: 0,
            current_bb: 0,
            temp: 0,
            is_entry,
            fn_names: fn_names.clone(),
            name_map: name_map.clone(),
            generic_sigs: generic_sigs.clone(),
            pending_specs: Vec::new(),
            defers: Vec::new(),
            pending_spawns: Vec::new(),
            spawn_id: 0,
            fn_sigs: fn_sigs.clone(),
            loop_stack: Vec::new(),
            enum_variants: enum_variants.clone(),
        }
    }

    fn lower_if_chain(
        &mut self,
        cond: &Spanned<buraaq_ast::ExprNode>,
        then_block: &Block,
        elifs: &[Spanned<buraaq_ast::ElifBranch>],
        else_block: Option<&Block>,
        merge_bb: u32,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(), LowerError> {
        let (cond_op, _) = self.lower_expr(cond, structs)?;
        let then_bb = self.new_block();
        let else_bb = self.new_block();
        self.terminate(Terminator::If {
            cond: cond_op,
            then_bb,
            else_bb,
        });

        self.current_bb = then_bb;
        self.lower_block(then_block, structs)?;
        if !self.block_terminated() {
            self.terminate(Terminator::Goto(merge_bb));
        }

        self.current_bb = else_bb;
        if let Some((first, rest)) = elifs.split_first() {
            self.lower_if_chain(
                &first.node.cond,
                &first.node.block.node,
                rest,
                else_block,
                merge_bb,
                structs,
            )?;
        } else if let Some(el) = else_block {
            self.lower_block(el, structs)?;
            if !self.block_terminated() {
                self.terminate(Terminator::Goto(merge_bb));
            }
        } else if !self.block_terminated() {
            self.terminate(Terminator::Goto(merge_bb));
        }
        Ok(())
    }

    fn emit_scope_deads(&mut self, start: u32) {
        for id in (start..self.next_local).rev() {
            let name = &self.locals[id as usize].name;
            if name.starts_with("__tmp") {
                continue;
            }
            self.emit(Statement::StorageDead { local: id });
        }
    }

    fn remap_fn(&self, src: &str) -> String {
        self.name_map
            .get(src)
            .cloned()
            .unwrap_or_else(|| src.to_string())
    }

    fn flush_defers(
        &mut self,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(), LowerError> {
        self.flush_defers_since(0, structs)
    }

    fn flush_defers_since(
        &mut self,
        mark: usize,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(), LowerError> {
        let extra: Vec<_> = self.defers.get(mark..).unwrap_or(&[]).to_vec();
        for e in extra.into_iter().rev() {
            self.lower_expr_as_stmt(&e, structs)?;
        }
        Ok(())
    }

    fn close_loop_defers(
        &mut self,
        mark: usize,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(), LowerError> {
        self.flush_defers_since(mark, structs)?;
        self.defers.truncate(mark);
        Ok(())
    }

    fn lower_spawn(
        &mut self,
        s: &Spanned<SpawnExpr>,
        _structs: &HashMap<String, MirStruct>,
    ) -> Result<(Operand, MirTy), LowerError> {
        let name = format!("__spawn_{}", self.spawn_id);
        self.spawn_id += 1;
        match &s.node {
            SpawnExpr::Block(b) => self.pending_spawns.push((name.clone(), b.node.clone())),
            SpawnExpr::Call(c) => {
                let block = Block {
                    stmts: vec![Spanned::new(Stmt::Expr(c.clone()), c.span)],
                    tail: None,
                    span: c.span,
                };
                self.pending_spawns.push((name.clone(), block));
            }
        }
        let dest = self.assign_rvalue(
            Rvalue::Call {
                func: "buraaq_task_submit".into(),
                args: vec![
                    Operand::Constant(Constant::FnAddr(name)),
                    Operand::Constant(Constant::NullPtr(MirTy::ptr_to(MirTy::I8))),
                ],
                ret_ty: MirTy::ptr_to(MirTy::I8),
            },
            MirTy::ptr_to(MirTy::I8),
        );
        Ok((Operand::Local(dest), MirTy::ptr_to(MirTy::I8)))
    }

    fn instantiate_generic(&mut self, src: &str, arg_tys: &[MirTy]) -> (String, MirTy) {
        if let Some(sig) = self.generic_sigs.get(src).cloned() {
            let mut tys: Vec<MirTy> = arg_tys.iter().take(sig.type_param_count).cloned().collect();
            while tys.len() < sig.type_param_count {
                tys.push(tys.last().cloned().unwrap_or(MirTy::I32));
            }
            let name = spec_fn_name(src, &tys);
            let ret = sig
                .ret_is_param
                .and_then(|i| tys.get(i).cloned())
                .or_else(|| tys.first().cloned())
                .unwrap_or(MirTy::I32);
            self.pending_specs.push((src.to_string(), tys));
            (name, ret)
        } else {
            (self.remap_fn(src), MirTy::I32)
        }
    }

    fn finish(self) -> MirFunction {
        MirFunction {
            name: self.name,
            params: self.params,
            return_ty: self.return_ty,
            locals: self.locals,
            blocks: self.blocks,
            is_entry: self.is_entry,
        }
    }

    fn fresh_local(&mut self, name: String, ty: MirTy, mutable: bool) -> LocalId {
        if !name.starts_with("__tmp") {
            if let Some(existing) = self.lookup(&name) {
                return existing;
            }
        }
        let id = self.next_local;
        self.next_local += 1;
        self.local_tys.insert(id, ty.clone());
        self.locals.push(MirLocal {
            name: name.clone(),
            ty,
            mutable,
        });
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, id);
        }
        id
    }

    fn declare_param(&mut self, name: String, ty: MirTy) {
        let id = self.fresh_local(name.clone(), ty.clone(), false);
        self.params.push((name, ty));
        let _ = id;
    }

    fn lookup(&self, name: &str) -> Option<LocalId> {
        for scope in self.scopes.iter().rev() {
            if let Some(id) = scope.get(name) {
                return Some(*id);
            }
        }
        None
    }

    fn temp_local(&mut self, ty: MirTy) -> LocalId {
        let n = self.temp;
        self.temp += 1;
        self.fresh_local(format!("__tmp{n}"), ty, false)
    }

    fn start_block(&mut self) {
        let id = self.next_bb;
        self.next_bb += 1;
        self.current_bb = id;
        self.blocks.push(BasicBlock {
            id,
            stmts: Vec::new(),
            terminator: Terminator::Unreachable,
        });
    }

    fn new_block(&mut self) -> BlockId {
        let id = self.next_bb;
        self.next_bb += 1;
        self.blocks.push(BasicBlock {
            id,
            stmts: Vec::new(),
            terminator: Terminator::Unreachable,
        });
        id
    }

    fn emit(&mut self, stmt: Statement) {
        self.blocks[self.current_bb as usize].stmts.push(stmt);
    }

    fn terminate(&mut self, term: Terminator) {
        self.blocks[self.current_bb as usize].terminator = term;
    }

    fn current_terminator(&self) -> Option<Terminator> {
        self.blocks.get(self.current_bb as usize).map(|b| b.terminator.clone())
    }

    fn assign_rvalue(&mut self, rv: Rvalue, ty: MirTy) -> LocalId {
        let dest = self.temp_local(ty);
        self.emit(Statement::Assign { dest, rvalue: rv });
        dest
    }

    fn lower_expr_into(
        &mut self,
        dest: LocalId,
        expr: &Spanned<buraaq_ast::ExprNode>,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(), LowerError> {
        match expr.node.as_ref() {
            Expr::Binary(b) => {
                let (left, lty) = self.lower_expr(&b.node.left, structs)?;
                let (right, _) = self.lower_expr(&b.node.right, structs)?;
                let op = map_binop(b.node.op.node);
                let _ty = match op {
                    BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
                    | BinOp::And | BinOp::Or => MirTy::Bool,
                    _ => lty,
                };
                self.emit(Statement::Assign {
                    dest,
                    rvalue: Rvalue::Binary { op, left, right },
                });
                Ok(())
            }
            Expr::Unary(u) if matches!(u.node.op.node, UnaryOp::Neg | UnaryOp::Not) => {
                let (operand, _) = self.lower_expr(&u.node.expr, structs)?;
                let op = match u.node.op.node {
                    UnaryOp::Neg => UnOp::Neg,
                    _ => UnOp::Not,
                };
                self.emit(Statement::Assign {
                    dest,
                    rvalue: Rvalue::Unary { op, operand },
                });
                Ok(())
            }
            _ => {
                let (init, _) = self.lower_expr(expr, structs)?;
                self.emit(Statement::Assign {
                    dest,
                    rvalue: Rvalue::Use(init),
                });
                Ok(())
            }
        }
    }

    fn lower_stmts(
        &mut self,
        block: &Block,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(), LowerError> {
        for stmt in &block.stmts {
            self.lower_stmt(stmt, structs)?;
        }
        Ok(())
    }

    fn lower_block(
        &mut self,
        block: &Block,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(), LowerError> {
        self.lower_stmts(block, structs)?;
        if let Some(tail) = &block.tail {
            let (op, ty) = self.lower_expr(tail, structs)?;
            if self.return_ty == MirTy::Void {
                let _ = (op, ty);
            } else {
                self.terminate(Terminator::Return(Some(op)));
            }
        }
        Ok(())
    }

    fn lower_stmt(
        &mut self,
        stmt: &Spanned<Stmt>,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(), LowerError> {
        match &stmt.node {
            Stmt::VarDecl(v) => {
                let name = v.node.name.node.clone();
                if let Some(existing) = self.lookup(&name) {
                    self.lower_expr_into(existing, &v.node.init, structs)?;
                } else {
                    let (init, ty) = self.lower_expr(&v.node.init, structs)?;
                    let id = self.fresh_local(name, ty, v.node.mutable);
                    self.emit(Statement::Assign {
                        dest: id,
                        rvalue: Rvalue::Use(init),
                    });
                }
            }
            Stmt::Expr(e) => {
                self.lower_expr_as_stmt(e, structs)?;
            }
            Stmt::If(i) => {
                let merge_bb = self.new_block();
                self.lower_if_chain(
                    &i.node.cond,
                    &i.node.then_block.node,
                    &i.node.elifs,
                    i.node.else_block.as_ref().map(|e| &e.node),
                    merge_bb,
                    structs,
                )?;
                self.current_bb = merge_bb;
            }
            Stmt::While(w) => {
                let header = self.new_block();
                let body_bb = self.new_block();
                let exit_bb = self.new_block();
                self.terminate(Terminator::Goto(header));

                self.current_bb = header;
                let (cond, _) = self.lower_expr(&w.node.cond, structs)?;
                self.terminate(Terminator::If {
                    cond,
                    then_bb: body_bb,
                    else_bb: exit_bb,
                });

                let defer_mark = self.defers.len();
                self.loop_stack.push(LoopFrame {
                    cont: header,
                    exit: exit_bb,
                    local_start: self.next_local,
                    defer_mark,
                });
                self.current_bb = body_bb;
                self.lower_block(&w.node.body.node, structs)?;
                if !self.block_terminated() {
                    self.close_loop_defers(defer_mark, structs)?;
                    self.terminate(Terminator::Goto(header));
                }
                self.loop_stack.pop();

                self.current_bb = exit_bb;
            }
            Stmt::For(f) => {
                if f.node.parallel {
                    return Err(LowerError::Msg("`parallel for` not yet implemented".into()));
                }
                match &f.node.iter {
                    ForIter::Range { start, end, inclusive } => {
                        let var_name = f.node.var.node.clone();
                        let (start_op, start_ty) = self.lower_expr(start, structs)?;
                        let loop_id = self.fresh_local(var_name, start_ty, true);
                        self.emit(Statement::Assign {
                            dest: loop_id,
                            rvalue: Rvalue::Use(start_op),
                        });
                        let (end_op, _) = self.lower_expr(end, structs)?;

                        let header = self.new_block();
                        let body_bb = self.new_block();
                        let step_bb = self.new_block();
                        let exit_bb = self.new_block();
                        self.terminate(Terminator::Goto(header));

                        self.current_bb = header;
                        let cond = self.assign_rvalue(
                            Rvalue::Binary {
                                op: if *inclusive { BinOp::Le } else { BinOp::Lt },
                                left: Operand::Local(loop_id),
                                right: end_op,
                            },
                            MirTy::Bool,
                        );
                        self.terminate(Terminator::If {
                            cond: Operand::Local(cond),
                            then_bb: body_bb,
                            else_bb: exit_bb,
                        });

                        let defer_mark = self.defers.len();
                        self.loop_stack.push(LoopFrame {
                            cont: step_bb,
                            exit: exit_bb,
                            local_start: self.next_local,
                            defer_mark,
                        });
                        self.current_bb = body_bb;
                        self.lower_block(&f.node.body.node, structs)?;
                        if !self.block_terminated() {
                            self.close_loop_defers(defer_mark, structs)?;
                            self.terminate(Terminator::Goto(step_bb));
                        }
                        self.loop_stack.pop();

                        self.current_bb = step_bb;
                        let next = self.assign_rvalue(
                            Rvalue::Binary {
                                op: BinOp::Add,
                                left: Operand::Local(loop_id),
                                right: Operand::Constant(Constant::I32(1)),
                            },
                            MirTy::I32,
                        );
                        self.emit(Statement::Assign {
                            dest: loop_id,
                            rvalue: Rvalue::Use(Operand::Local(next)),
                        });
                        self.terminate(Terminator::Goto(header));

                        self.current_bb = exit_bb;
                    }
                    ForIter::In(_) => {
                        return Err(LowerError::Msg("`for x in collection` not yet implemented".into()));
                    }
                }
            }
            Stmt::Break(_) => {
                let frame = self
                    .loop_stack
                    .last()
                    .cloned()
                    .ok_or_else(|| LowerError::Msg("`break` outside of a loop".into()))?;
                self.close_loop_defers(frame.defer_mark, structs)?;
                self.emit_scope_deads(frame.local_start);
                self.terminate(Terminator::Goto(frame.exit));
            }
            Stmt::Continue(_) => {
                let frame = self
                    .loop_stack
                    .last()
                    .cloned()
                    .ok_or_else(|| LowerError::Msg("`continue` outside of a loop".into()))?;
                self.close_loop_defers(frame.defer_mark, structs)?;
                self.emit_scope_deads(frame.local_start);
                self.terminate(Terminator::Goto(frame.cont));
            }
            Stmt::Return(r) => {
                let op = r
                    .node
                    .value
                    .as_ref()
                    .map(|v| self.lower_expr(v, structs).map(|(o, _)| o))
                    .transpose()?;
                self.flush_defers(structs)?;
                self.terminate(Terminator::Return(op));
            }
            Stmt::Defer(e) => {
                self.defers.push(e.clone());
            }
            Stmt::Unsafe(b) => {
                self.lower_stmts(&b.node, structs)?;
                if let Some(tail) = &b.node.tail {
                    self.lower_expr(tail, structs)?;
                }
            }
            Stmt::Match(m) => {
                self.lower_match(&m.node.scrutinee, &m.node.arms, structs)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn block_terminated(&self) -> bool {
        !matches!(
            self.blocks[self.current_bb as usize].terminator,
            Terminator::Unreachable
        )
    }

    fn lower_expr_as_stmt(
        &mut self,
        expr: &Spanned<buraaq_ast::ExprNode>,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(), LowerError> {
        match expr.node.as_ref() {
            Expr::Spawn(s) => {
                self.lower_spawn(s, structs)?;
            }
            _ => {
                self.lower_expr(expr, structs)?;
            }
        }
        Ok(())
    }

    fn lower_expr(
        &mut self,
        expr: &Spanned<buraaq_ast::ExprNode>,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(Operand, MirTy), LowerError> {
        match expr.node.as_ref() {
            Expr::Literal(l) => self.lower_literal(&l.node, structs),
            Expr::Ident(name) => {
                let id = self
                    .lookup(&name.node)
                    .ok_or_else(|| LowerError::Msg(format!("unknown name `{}`", name.node)))?;
                let ty = self.local_tys[&id].clone();
                Ok((Operand::Local(id), ty))
            }
            Expr::Self_(_) => {
                let id = self
                    .lookup("self")
                    .ok_or_else(|| LowerError::Msg("unknown `self`".into()))?;
                let ty = self.local_tys[&id].clone();
                Ok((Operand::Local(id), ty))
            }
            Expr::Binary(b) => {
                let (left, lty) = self.lower_expr(&b.node.left, structs)?;
                let (right, _) = self.lower_expr(&b.node.right, structs)?;
                let op = map_binop(b.node.op.node);
                let ty = match op {
                    BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
                    | BinOp::And | BinOp::Or => MirTy::Bool,
                    _ => lty.clone(),
                };
                let dest = self.assign_rvalue(
                    Rvalue::Binary {
                        op,
                        left,
                        right,
                    },
                    ty.clone(),
                );
                Ok((Operand::Local(dest), ty))
            }
            Expr::Unary(u) => {
                let (operand, ty) = self.lower_expr(&u.node.expr, structs)?;
                if matches!(u.node.op.node, UnaryOp::Deref) {
                    let elem = match &ty {
                        MirTy::Ptr(inner) | MirTy::Ref { inner, .. } => (**inner).clone(),
                        _ => MirTy::I32,
                    };
                    let dest = self.assign_rvalue(
                        Rvalue::Load {
                            ptr: operand,
                            ty: elem.clone(),
                        },
                        elem.clone(),
                    );
                    return Ok((Operand::Local(dest), elem));
                }
                if matches!(u.node.op.node, UnaryOp::Ref | UnaryOp::RefMut) {
                    if let Expr::Ident(name) = u.node.expr.node.as_ref() {
                        if let Some(id) = self.lookup(&name.node) {
                            let inner = self.local_tys[&id].clone();
                            let dest = self.assign_rvalue(
                                Rvalue::AddrOf {
                                    local: id,
                                    ty: inner.clone(),
                                },
                                MirTy::Ref {
                                    mut_: matches!(u.node.op.node, UnaryOp::RefMut),
                                    inner: Box::new(inner),
                                },
                            );
                            let ty = self.local_tys[&dest].clone();
                            return Ok((Operand::Local(dest), ty));
                        }
                    }
                    return Err(LowerError::Msg("`ref` requires a local variable".into()));
                }
                let op = match u.node.op.node {
                    UnaryOp::Neg => UnOp::Neg,
                    UnaryOp::Not => UnOp::Not,
                    _ => {
                        return Err(LowerError::Msg(
                            "unsupported unary operator in codegen".into(),
                        ))
                    }
                };
                let dest = self.assign_rvalue(Rvalue::Unary { op, operand }, ty.clone());
                Ok((Operand::Local(dest), ty))
            }
            Expr::Call(c) => self.lower_call(&c.node.callee, &c.node.args, structs),
            Expr::Struct(s) => self.lower_struct_expr(s, structs),
            Expr::MethodCall(m) => self.lower_method_call(m, structs),
            Expr::Array(a) => self.lower_array(&a.node, structs),
            Expr::Match(m) => self.lower_match_expr(&m.node, structs),
            Expr::Field(f) => {
                if let Expr::Ident(name) = f.node.base.node.as_ref() {
                    if let Some(vars) = self.enum_variants.get(&name.node) {
                        if let Some(i) = vars.iter().position(|v| v == &f.node.field.node) {
                            return Ok((
                                Operand::Constant(Constant::I32(i as i32)),
                                MirTy::Enum {
                                    name: name.node.clone(),
                                },
                            ));
                        }
                    }
                }
                let (base, _) = self.lower_expr(&f.node.base, structs)?;
                let (field_index, field_ty) =
                    self.resolve_field(&f.node.base, &f.node.field.node, structs)?;
                let ty = field_ty.clone();
                let dest = self.assign_rvalue(
                    Rvalue::Field {
                        base,
                        field_index,
                        ty: field_ty,
                    },
                    ty.clone(),
                );
                Ok((Operand::Local(dest), ty))
            }
            Expr::Index(i) => {
                let (base, _) = self.lower_expr(&i.node.base, structs)?;
                let (index, _) = self.lower_expr(&i.node.index, structs)?;
                let elem_ty = MirTy::I32; // simplified
                let ty = elem_ty.clone();
                let dest = self.assign_rvalue(
                    Rvalue::Index {
                        base,
                        index,
                        elem_ty,
                    },
                    ty.clone(),
                );
                Ok((Operand::Local(dest), ty))
            }
            Expr::Paren(p) => self.lower_expr(p, structs),
            Expr::Block(b) => {
                self.lower_stmts(&b.node, structs)?;
                if let Some(tail) = &b.node.tail {
                    self.lower_expr(tail, structs)
                } else {
                    Ok((Operand::Constant(Constant::Void), MirTy::Void))
                }
            }
            Expr::If(i) => self.lower_if_expr(i, structs),
            Expr::Spawn(s) => self.lower_spawn(s, structs),
            Expr::Unsafe(b) => {
                self.lower_stmts(&b.node, structs)?;
                if let Some(tail) = &b.node.tail {
                    self.lower_expr(tail, structs)
                } else {
                    Ok((Operand::Constant(Constant::Void), MirTy::Void))
                }
            }
            Expr::Assign(a) => {
                let (val, val_ty) = self.lower_expr(&a.node.value, structs)?;
                if let Expr::Unary(u) = a.node.target.node.as_ref() {
                    if matches!(u.node.op.node, UnaryOp::Deref) {
                        let (ptr, _) = self.lower_expr(&u.node.expr, structs)?;
                        self.emit(Statement::Store {
                            ptr,
                            value: val.clone(),
                            ty: val_ty.clone(),
                        });
                        return Ok((val, val_ty));
                    }
                }
                if let Expr::Ident(name) = a.node.target.node.as_ref() {
                    if let Some(id) = self.lookup(&name.node) {
                        self.emit(Statement::Assign {
                            dest: id,
                            rvalue: Rvalue::Use(val.clone()),
                        });
                        let ty = self.local_tys[&id].clone();
                        return Ok((val, ty));
                    }
                }
                Err(LowerError::Msg("unsupported assignment target".into()))
            }
            _ => Err(LowerError::Msg(format!(
                "unsupported expression in codegen: {:?}",
                expr.node
            ))),
        }
    }

    fn lower_literal(
        &mut self,
        lit: &Literal,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(Operand, MirTy), LowerError> {
        match lit {
            Literal::Int(n) => Ok((Operand::Constant(Constant::I32(n.node as i32)), MirTy::I32)),
            Literal::Float(n) => Ok((Operand::Constant(Constant::F64(n.node)), MirTy::F64)),
            Literal::Bool(b) => Ok((Operand::Constant(Constant::Bool(b.node)), MirTy::Bool)),
            Literal::String(s) => self.lower_string_parts(&s.node.parts, structs),
            Literal::Char(c) => Ok((Operand::Constant(Constant::I32(c.node as i32)), MirTy::I32)),
            _ => Ok((Operand::Constant(Constant::I32(0)), MirTy::I32)),
        }
    }

    fn lower_string_parts(
        &mut self,
        parts: &[StringPart],
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(Operand, MirTy), LowerError> {
        let has_interp = parts.iter().any(|p| matches!(p, StringPart::Interp(_)));
        if !has_interp {
            let text: String = parts
                .iter()
                .filter_map(|p| match p {
                    StringPart::Text(t) => Some(t.as_str()),
                    StringPart::Interp(_) => None,
                })
                .collect();
            return Ok((Operand::Constant(Constant::Str(text)), MirTy::Text));
        }

        let mut acc: Option<Operand> = None;
        for part in parts {
            let piece = match part {
                StringPart::Text(t) => Operand::Constant(Constant::Str(t.clone())),
                StringPart::Interp(expr) => {
                    let (op, ty) = self.lower_expr(expr, structs)?;
                    self.coerce_to_text(op, ty)
                }
            };
            acc = Some(match acc {
                None => piece,
                Some(left) => Operand::Local(self.assign_rvalue(
                    Rvalue::Call {
                        func: "buraaq_text_concat".into(),
                        args: vec![left, piece],
                        ret_ty: MirTy::Text,
                    },
                    MirTy::Text,
                )),
            });
        }
        Ok((
            acc.unwrap_or_else(|| Operand::Constant(Constant::Str(String::new()))),
            MirTy::Text,
        ))
    }

    fn coerce_to_text(&mut self, op: Operand, ty: MirTy) -> Operand {
        if matches!(ty, MirTy::Text | MirTy::Bytes) {
            return op;
        }
        let runtime_fn = match ty {
            MirTy::I32 => "buraaq_i32_to_text",
            MirTy::I64 => "buraaq_i64_to_text",
            MirTy::F64 | MirTy::F32 => "buraaq_f64_to_text",
            MirTy::Bool => "buraaq_bool_to_text",
            _ => "buraaq_i32_to_text",
        };
        Operand::Local(self.assign_rvalue(
            Rvalue::Call {
                func: runtime_fn.to_string(),
                args: vec![op],
                ret_ty: MirTy::Text,
            },
            MirTy::Text,
        ))
    }

    fn lower_call(
        &mut self,
        callee: &Spanned<buraaq_ast::ExprNode>,
        args: &[Spanned<buraaq_ast::ExprNode>],
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(Operand, MirTy), LowerError> {
        let src_name = match callee.node.as_ref() {
            Expr::Ident(name) => name.node.clone(),
            Expr::Field(f) => f.node.field.node.clone(),
            _ => return Err(LowerError::Msg("indirect calls not yet supported".into())),
        };
        let mut arg_ops = Vec::new();
        let mut arg_tys = Vec::new();
        for a in args {
            let (op, ty) = self.lower_expr(a, structs)?;
            arg_ops.push(op);
            arg_tys.push(ty);
        }
        let (func_name, ret_ty) = if self.generic_sigs.contains_key(&src_name) {
            self.instantiate_generic(&src_name, &arg_tys)
        } else if let Some((_, ret)) = self.fn_sigs.get(&src_name) {
            (self.remap_fn(&src_name), ret.clone())
        } else {
            let mapped = self.remap_fn(&src_name);
            (mapped, MirTy::I32)
        };
        let dest = self.assign_rvalue(
            Rvalue::Call {
                func: func_name,
                args: arg_ops,
                ret_ty: ret_ty.clone(),
            },
            ret_ty.clone(),
        );
        Ok((Operand::Local(dest), ret_ty))
    }

    fn lower_struct_expr(
        &mut self,
        s: &Spanned<buraaq_ast::StructExpr>,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(Operand, MirTy), LowerError> {
        let struct_name = s
            .node
            .path
            .node
            .segments
            .last()
            .map(|seg| seg.node.clone())
            .unwrap_or_default();

        if matches!(
            struct_name.as_str(),
            "print" | "println" | "print_int" | "print_float" | "print_bool"
        ) {
            return self.lower_print_builtin(s, structs);
        }

        if self.fn_names.contains(&struct_name) && !structs.contains_key(&struct_name) {
            let mut arg_ops = Vec::new();
            let mut arg_tys = Vec::new();
            if let StructFill::Tuple(args) = &s.node.fill {
                for a in args {
                    let (op, ty) = self.lower_expr(a, structs)?;
                    arg_ops.push(op);
                    arg_tys.push(ty);
                }
            }
            let (func, ret_ty) = if self.generic_sigs.contains_key(&struct_name) {
                self.instantiate_generic(&struct_name, &arg_tys)
            } else if let Some((_, ret)) = self.fn_sigs.get(&struct_name) {
                (self.remap_fn(&struct_name), ret.clone())
            } else {
                (self.remap_fn(&struct_name), MirTy::I32)
            };
            let dest = self.assign_rvalue(
                Rvalue::Call {
                    func,
                    args: arg_ops,
                    ret_ty: ret_ty.clone(),
                },
                ret_ty.clone(),
            );
            return Ok((Operand::Local(dest), ret_ty));
        }

        let st = structs
            .get(&struct_name)
            .ok_or_else(|| LowerError::Msg(format!("unknown struct `{struct_name}`")))?;
        let ty = MirTy::Struct {
            name: struct_name.clone(),
        };

        let mut fields = Vec::new();
        if let StructFill::Tuple(args) = &s.node.fill {
            for a in args {
                let (op, _) = self.lower_expr(a, structs)?;
                fields.push(op);
            }
        } else {
            for (fname, _) in &st.fields {
                let init = s
                    .node
                    .fields
                    .iter()
                    .find(|f| f.node.name.node == *fname)
                    .ok_or_else(|| {
                        LowerError::Msg(format!(
                            "missing field `{fname}` in struct `{struct_name}`"
                        ))
                    })?;
                let (op, _) = self.lower_expr(&init.node.value, structs)?;
                fields.push(op);
            }
        }

        let dest = self.assign_rvalue(Rvalue::Aggregate { ty: ty.clone(), fields }, ty.clone());
        Ok((Operand::Local(dest), ty))
    }

    fn lower_print_builtin(
        &mut self,
        s: &Spanned<buraaq_ast::StructExpr>,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(Operand, MirTy), LowerError> {
        let name = s
            .node
            .path
            .node
            .segments
            .last()
            .map(|seg| seg.node.clone())
            .unwrap_or_else(|| "print".into());

        let args: Vec<_> = if let StructFill::Tuple(a) = &s.node.fill {
            a.iter().collect()
        } else {
            vec![]
        };

        let trailing_nl = name == "println"
            || name == "print_int"
            || name == "print_float"
            || name == "print_bool";

        for (i, a) in args.iter().enumerate() {
            if i > 0 {
                self.emit_runtime_call(
                    "buraaq_print_str",
                    vec![Operand::Constant(Constant::Str(" ".into()))],
                );
            }
            let (op, ty) = self.lower_expr(a, structs)?;
            let runtime_fn = match name.as_str() {
                "print_int" => "buraaq_print_i32",
                "print_float" => "buraaq_print_f64",
                "print_bool" => "buraaq_print_bool",
                _ => match ty {
                    MirTy::Text => "buraaq_print_str",
                    MirTy::I32 => "buraaq_print_i32",
                    MirTy::I64 => "buraaq_print_i64",
                    MirTy::Bool => "buraaq_print_bool",
                    MirTy::F64 | MirTy::F32 => "buraaq_print_f64",
                    _ => "buraaq_print_i32",
                },
            };
            self.emit_runtime_call(runtime_fn, vec![op]);
        }
        if trailing_nl {
            self.emit_runtime_call(
                "buraaq_print_str_ln",
                vec![Operand::Constant(Constant::Str(String::new()))],
            );
        }
        Ok((Operand::Constant(Constant::Void), MirTy::Void))
    }

    fn emit_runtime_call(&mut self, func: &str, args: Vec<Operand>) {
        self.assign_rvalue(
            Rvalue::Call {
                func: func.to_string(),
                args,
                ret_ty: MirTy::Void,
            },
            MirTy::Void,
        );
    }

    fn lower_method_call(
        &mut self,
        m: &Spanned<buraaq_ast::MethodCallExpr>,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(Operand, MirTy), LowerError> {
        let (receiver, recv_ty) = self.lower_expr(&m.node.receiver, structs)?;
        let method = &m.node.method.node;

        if method == "to_c" {
            return Ok((receiver, recv_ty));
        }
        if method == "len" {
            if let MirTy::Array { len, .. } = recv_ty {
                return Ok((
                    Operand::Constant(Constant::I32(len as i32)),
                    MirTy::I32,
                ));
            }
        }
        if method == "wait" || method == "join" {
            let dest = self.assign_rvalue(
                Rvalue::Call {
                    func: "buraaq_task_join".into(),
                    args: vec![receiver],
                    ret_ty: MirTy::I64,
                },
                MirTy::I64,
            );
            return Ok((Operand::Local(dest), MirTy::I64));
        }
        if method == "to_text" {
            let text = self.coerce_to_text(receiver, recv_ty);
            return Ok((text, MirTy::Text));
        }

        if let MirTy::Struct { name } = &recv_ty {
            let fname = format!("{name}_{method}");
            if self.fn_names.contains(&fname) || self.fn_sigs.contains_key(&fname) {
                let mut args = vec![receiver];
                for a in &m.node.args {
                    let (op, _) = self.lower_expr(a, structs)?;
                    args.push(op);
                }
                let ret_ty = self
                    .fn_sigs
                    .get(&fname)
                    .map(|(_, r)| r.clone())
                    .unwrap_or(MirTy::I32);
                let dest = self.assign_rvalue(
                    Rvalue::Call {
                        func: fname,
                        args,
                        ret_ty: ret_ty.clone(),
                    },
                    ret_ty.clone(),
                );
                return Ok((Operand::Local(dest), ret_ty));
            }
        }

        Err(LowerError::Msg(format!("unsupported method `{method}`")))
    }

    fn lower_if_expr(
        &mut self,
        i: &Spanned<buraaq_ast::IfExpr>,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(Operand, MirTy), LowerError> {
        let (cond, _) = self.lower_expr(&i.node.cond, structs)?;
        let then_bb = self.new_block();
        let else_bb = self.new_block();
        let merge_bb = self.new_block();
        self.terminate(Terminator::If {
            cond,
            then_bb,
            else_bb,
        });

        self.current_bb = then_bb;
        let (then_val, then_ty) = if let Some(t) = &i.node.then_block.node.tail {
            self.lower_expr(t, structs)?
        } else {
            (Operand::Constant(Constant::I32(0)), MirTy::I32)
        };
        if !self.block_terminated() {
            self.terminate(Terminator::Goto(merge_bb));
        }

        self.current_bb = else_bb;
        let (else_val, _else_ty) = if let Some(t) = &i.node.else_block.node.tail {
            self.lower_expr(t, structs)?
        } else {
            (Operand::Constant(Constant::I32(0)), MirTy::I32)
        };
        if !self.block_terminated() {
            self.terminate(Terminator::Goto(merge_bb));
        }

        let result_ty = then_ty.clone();
        let dest = self.temp_local(result_ty.clone());
        self.current_bb = merge_bb;
        self.emit(Statement::Assign {
            dest,
            rvalue: Rvalue::Use(then_val),
        });
        let _ = else_val;
        Ok((Operand::Local(dest), result_ty))
    }

    fn lower_array(
        &mut self,
        elems: &[Spanned<buraaq_ast::ExprNode>],
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(Operand, MirTy), LowerError> {
        let mut fields = Vec::new();
        let mut elem_ty = MirTy::I32;
        for e in elems {
            let (op, ty) = self.lower_expr(e, structs)?;
            elem_ty = ty;
            fields.push(op);
        }
        let ty = MirTy::Array {
            elem: Box::new(elem_ty),
            len: elems.len(),
        };
        let dest = self.assign_rvalue(Rvalue::Aggregate { ty: ty.clone(), fields }, ty.clone());
        Ok((Operand::Local(dest), ty))
    }

    fn lower_match(
        &mut self,
        scrutinee: &Spanned<buraaq_ast::ExprNode>,
        arms: &[Spanned<buraaq_ast::MatchArm>],
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(), LowerError> {
        let _ = self.lower_match_inner(scrutinee, arms, structs)?;
        Ok(())
    }

    fn lower_match_expr(
        &mut self,
        m: &buraaq_ast::MatchExpr,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(Operand, MirTy), LowerError> {
        self.lower_match_inner(&m.scrutinee, &m.arms, structs)
    }

    fn lower_match_inner(
        &mut self,
        scrutinee: &Spanned<buraaq_ast::ExprNode>,
        arms: &[Spanned<buraaq_ast::MatchArm>],
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(Operand, MirTy), LowerError> {
        let (discr, discr_ty) = self.lower_expr(scrutinee, structs)?;
        let merge_bb = self.new_block();
        let otherwise_bb = self.new_block();
        let mut switch_arms = Vec::new();
        let mut lowered_arms = Vec::new();
        let mut wild = None;
        for arm in arms {
            if self.pattern_is_wild(&arm.node.pattern.node) {
                wild = Some(arm);
                continue;
            }
            let bb = self.new_block();
            if let Some(disc) = self.pattern_disc(&arm.node.pattern.node, &discr_ty) {
                switch_arms.push((disc as i64, bb));
            } else {
                switch_arms.push((i64::MIN, bb));
            }
            lowered_arms.push((bb, arm));
        }
        self.terminate(Terminator::Switch {
            discr: discr.clone(),
            arms: switch_arms
                .into_iter()
                .filter(|(v, _)| *v != i64::MIN)
                .collect(),
            otherwise: otherwise_bb,
        });

        let dest_ty = match &self.return_ty {
            MirTy::Void => MirTy::Text,
            other => other.clone(),
        };
        let dest = self.temp_local(dest_ty.clone());
        let mut result_ty = dest_ty;
        for (bb, arm) in lowered_arms {
            self.current_bb = bb;
            self.bind_arm_pattern(&arm.node.pattern.node, discr.clone())?;
            let (val, ty) = self.lower_arm_body(&arm.node.body, structs)?;
            result_ty = ty.clone();
            self.local_tys.insert(dest, ty.clone());
            if !self.block_terminated() {
                self.emit(Statement::Assign {
                    dest,
                    rvalue: Rvalue::Use(val),
                });
                self.terminate(Terminator::Goto(merge_bb));
            }
        }
        self.current_bb = otherwise_bb;
        if let Some(arm) = wild {
            self.bind_arm_pattern(&arm.node.pattern.node, discr)?;
            let (val, ty) = self.lower_arm_body(&arm.node.body, structs)?;
            result_ty = ty;
            if !self.block_terminated() {
                self.emit(Statement::Assign {
                    dest,
                    rvalue: Rvalue::Use(val),
                });
                self.terminate(Terminator::Goto(merge_bb));
            }
        } else if !self.block_terminated() {
            self.terminate(Terminator::Unreachable);
        }
        self.current_bb = merge_bb;
        Ok((Operand::Local(dest), result_ty))
    }

    fn pattern_is_wild(&self, pat: &buraaq_ast::Pattern) -> bool {
        matches!(pat, buraaq_ast::Pattern::Wild(_))
    }

    fn pattern_disc(&self, pat: &buraaq_ast::Pattern, ty: &MirTy) -> Option<i32> {
        let variant = match pat {
            buraaq_ast::Pattern::Path(p) => p.node.variant.node.as_str(),
            _ => return None,
        };
        if let MirTy::Enum { name } = ty {
            return self
                .enum_variants
                .get(name)
                .and_then(|vs| vs.iter().position(|v| v == variant).map(|i| i as i32));
        }
        for vs in self.enum_variants.values() {
            if let Some(i) = vs.iter().position(|v| v == variant) {
                return Some(i as i32);
            }
        }
        None
    }

    fn bind_arm_pattern(
        &mut self,
        pat: &buraaq_ast::Pattern,
        scrut: Operand,
    ) -> Result<(), LowerError> {
        match pat {
            buraaq_ast::Pattern::Ident(name) => {
                let id = self.fresh_local(name.node.clone(), MirTy::I32, false);
                self.emit(Statement::Assign {
                    dest: id,
                    rvalue: Rvalue::Use(scrut),
                });
            }
            buraaq_ast::Pattern::Path(p) => {
                if let buraaq_ast::PathPatternKind::Tuple(pats) = &p.node.kind {
                    for sub in pats {
                        self.bind_arm_pattern(&sub.node, scrut.clone())?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn lower_arm_body(
        &mut self,
        body: &buraaq_ast::MatchArmBody,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(Operand, MirTy), LowerError> {
        match body {
            buraaq_ast::MatchArmBody::Expr(e) => self.lower_expr(e, structs),
            buraaq_ast::MatchArmBody::Block(b) => {
                self.lower_stmts(&b.node, structs)?;
                if let Some(t) = &b.node.tail {
                    self.lower_expr(t, structs)
                } else {
                    Ok((Operand::Constant(Constant::Void), MirTy::Void))
                }
            }
        }
    }

    fn resolve_field(
        &self,
        base: &Spanned<buraaq_ast::ExprNode>,
        field: &str,
        structs: &HashMap<String, MirStruct>,
    ) -> Result<(u32, MirTy), LowerError> {
        let key = match base.node.as_ref() {
            Expr::Ident(name) => name.node.as_str(),
            Expr::Self_(_) => "self",
            _ => return Err(LowerError::Msg("field base must be local struct".into())),
        };
        let id = self
            .lookup(key)
            .ok_or_else(|| LowerError::Msg(format!("unknown `{key}` in field access")))?;
        let struct_name = match &self.local_tys[&id] {
            MirTy::Struct { name } => name.clone(),
            other => {
                return Err(LowerError::Msg(format!(
                    "field access on non-struct type `{other}`"
                )))
            }
        };
        let st = structs
            .get(&struct_name)
            .ok_or_else(|| LowerError::Msg(format!("unknown struct `{struct_name}`")))?;
        for (i, (fname, fty)) in st.fields.iter().enumerate() {
            if fname == field {
                return Ok((i as u32, fty.clone()));
            }
        }
        Err(LowerError::Msg(format!(
            "struct `{struct_name}` has no field `{field}`"
        )))
    }
}

fn implicit_return(ty: &MirTy) -> Terminator {
    match ty {
        MirTy::Void => Terminator::Return(None),
        MirTy::Text | MirTy::Bytes | MirTy::Ptr(_) | MirTy::Ref { .. } => {
            Terminator::Return(Some(Operand::Constant(Constant::NullPtr(ty.clone()))))
        }
        MirTy::Bool => Terminator::Return(Some(Operand::Constant(Constant::Bool(false)))),
        MirTy::I64 => Terminator::Return(Some(Operand::Constant(Constant::I64(0)))),
        MirTy::F64 => Terminator::Return(Some(Operand::Constant(Constant::F64(0.0)))),
        _ => Terminator::Return(Some(Operand::Constant(Constant::I32(0)))),
    }
}

fn map_binop(op: AstBinOp) -> BinOp {
    match op {
        AstBinOp::Add => BinOp::Add,
        AstBinOp::Sub => BinOp::Sub,
        AstBinOp::Mul => BinOp::Mul,
        AstBinOp::Div => BinOp::Div,
        AstBinOp::Mod => BinOp::Mod,
        AstBinOp::Eq => BinOp::Eq,
        AstBinOp::NotEq => BinOp::NotEq,
        AstBinOp::Lt => BinOp::Lt,
        AstBinOp::Le => BinOp::Le,
        AstBinOp::Gt => BinOp::Gt,
        AstBinOp::Ge => BinOp::Ge,
        AstBinOp::And => BinOp::And,
        AstBinOp::Or => BinOp::Or,
        _ => BinOp::Add,
    }
}
