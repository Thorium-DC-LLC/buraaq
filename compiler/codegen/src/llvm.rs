use std::collections::{HashMap, HashSet};
use std::fmt::Write as FmtWrite;

use buraaq_mir::{
    BasicBlock, BinOp, BlockId, Constant, MirFunction, MirModule, MirTy, Operand, Rvalue,
    Statement, Terminator, UnOp,
};
use thiserror::Error;

use crate::target::{OptLevel, TargetTriple};

#[derive(Debug, Error)]
pub enum CodegenError {
    #[error("codegen error: {0}")]
    Msg(String),
}

#[derive(Clone, Debug)]
pub struct LlvmEmitOptions {
    pub target: TargetTriple,
    pub opt: OptLevel,
    pub module_name: String,
}

impl Default for LlvmEmitOptions {
    fn default() -> Self {
        Self {
            target: TargetTriple::detect_host(),
            opt: OptLevel::Debug,
            module_name: "buraaq_module".into(),
        }
    }
}

/// Emit LLVM IR text from a MIR module (BIR lowering).
pub fn emit_llvm_ir(module: &MirModule, opts: &LlvmEmitOptions) -> Result<String, CodegenError> {
    let mut emitter = LlvmEmitter::new(module, opts);
    emitter.emit_module()
}

struct LlvmEmitter<'a> {
    module: &'a MirModule,
    opts: &'a LlvmEmitOptions,
    out: String,
    struct_types: HashMap<String, String>,
    string_globals: Vec<String>,
    string_names: HashMap<String, String>,
    tmp_counter: u32,
    next_md: u32,
    loop_md_ids: Vec<u32>,
}

impl<'a> LlvmEmitter<'a> {
    fn new(module: &'a MirModule, opts: &'a LlvmEmitOptions) -> Self {
        Self {
            module,
            opts,
            out: String::new(),
            struct_types: HashMap::new(),
            string_globals: Vec::new(),
            string_names: HashMap::new(),
            tmp_counter: 0,
            next_md: 1,
            loop_md_ids: Vec::new(),
        }
    }

    fn emit_fn_attrs(&self) -> bool {
        !matches!(self.opts.opt, OptLevel::Debug)
    }

    fn fn_attr_suffix(&self) -> &'static str {
        if self.emit_fn_attrs() {
            " #0"
        } else {
            ""
        }
    }

    fn emit_module(mut self) -> Result<String, CodegenError> {
        writeln!(self.out, "; Buraaq LLVM IR — module `{}`", self.opts.module_name).unwrap();
        writeln!(
            self.out,
            "target datalayout = \"{}\"",
            self.opts.target.llvm_datalayout()
        )
        .unwrap();
        writeln!(self.out, "target triple = \"{}\"", self.opts.target.llvm_triple()).unwrap();
        self.emit_runtime_decls();
        self.emit_extern_decls();
        self.emit_struct_types();
        self.emit_string_globals_prescan()?;
        for g in &self.string_globals {
            writeln!(self.out, "{g}").unwrap();
        }
        for func in &self.module.functions {
            self.emit_function(func)?;
        }
        if let Some(entry) = self.module.functions.iter().find(|f| f.is_entry) {
            self.emit_c_main(entry)?;
        }
        self.emit_module_metadata();
        Ok(self.out)
    }

    fn emit_module_metadata(&mut self) {
        if self.emit_fn_attrs() {
            writeln!(
                self.out,
                "attributes #0 = {{ mustprogress nounwind uwtable }}"
            )
            .unwrap();
        }
        if self.loop_md_ids.is_empty() {
            return;
        }
        writeln!(self.out, "!0 = !{{!\"llvm.loop.mustprogress\"}}").unwrap();
        for id in &self.loop_md_ids {
            writeln!(self.out, "!{id} = distinct !{{!{id}, !0}}").unwrap();
        }
    }

    fn fresh_loop_md(&mut self) -> u32 {
        let id = self.next_md;
        self.next_md += 1;
        self.loop_md_ids.push(id);
        id
    }

    fn emit_runtime_decls(&mut self) {
        let decls = [
            "declare void @buraaq_print_i32(i32)",
            "declare void @buraaq_print_i32_ln(i32)",
            "declare void @buraaq_print_i64(i64)",
            "declare void @buraaq_print_i64_ln(i64)",
            "declare void @buraaq_print_f64(double)",
            "declare void @buraaq_print_f64_ln(double)",
            "declare void @buraaq_print_bool(i1)",
            "declare void @buraaq_print_bool_ln(i1)",
            "declare void @buraaq_print_str(i8*)",
            "declare void @buraaq_print_str_ln(i8*)",
            "declare i8* @buraaq_i32_to_text(i32)",
            "declare i8* @buraaq_i64_to_text(i64)",
            "declare i8* @buraaq_f64_to_text(double)",
            "declare i8* @buraaq_bool_to_text(i1)",
            "declare i8* @buraaq_alloc(i64)",
            "declare void @buraaq_free(i8*)",
            "declare i8* @buraaq_task_submit(i8*, i8*)",
            "declare i64 @buraaq_task_join(i8*)",
            "declare void @buraaq_task_yield()",
            "declare void @buraaq_runtime_init(i32)",
            "declare void @buraaq_runtime_shutdown()",
            "declare void @buraaq_led_on()",
            "declare void @buraaq_led_off()",
            "declare void @buraaq_led_toggle()",
            "declare void @buraaq_led_wait_ms(i32)",
            "declare i8* @buraaq_file_read(i8*)",
            "declare i32 @buraaq_file_write(i8*, i8*)",
            "declare i32 @buraaq_file_exists(i8*)",
            "declare i8* @buraaq_text_concat(i8*, i8*)",
            "declare i32 @buraaq_text_len(i8*)",
            "declare double @buraaq_math_sqrt(double)",
            "declare double @buraaq_math_abs_f64(double)",
            "declare i32 @buraaq_math_abs_i32(i32)",
            "declare double @buraaq_math_min_f64(double, double)",
            "declare double @buraaq_math_max_f64(double, double)",
            "declare double @buraaq_math_sin(double)",
            "declare double @buraaq_math_cos(double)",
            "declare double @buraaq_math_tan(double)",
            "declare double @buraaq_math_asin(double)",
            "declare double @buraaq_math_acos(double)",
            "declare double @buraaq_math_atan(double)",
            "declare double @buraaq_math_atan2(double, double)",
            "declare double @buraaq_math_sinh(double)",
            "declare double @buraaq_math_cosh(double)",
            "declare double @buraaq_math_tanh(double)",
            "declare double @buraaq_math_exp(double)",
            "declare double @buraaq_math_log(double)",
            "declare double @buraaq_math_log10(double)",
            "declare double @buraaq_math_log2(double)",
            "declare double @buraaq_math_pow(double, double)",
            "declare double @buraaq_math_hypot(double, double)",
            "declare double @buraaq_math_floor(double)",
            "declare double @buraaq_math_ceil(double)",
            "declare double @buraaq_math_trunc(double)",
            "declare double @buraaq_math_round(double)",
            "declare double @buraaq_math_fmod(double, double)",
            "declare double @buraaq_math_copysign(double, double)",
            "declare double @buraaq_math_cbrt(double)",
            "declare double @buraaq_math_pi()",
            "declare double @buraaq_math_euler()",
            "declare double @buraaq_math_deg(double)",
            "declare double @buraaq_math_rad(double)",
            "declare double @buraaq_math_clamp(double, double, double)",
            "declare double @buraaq_math_lerp(double, double, double)",
            "declare double @buraaq_math_sign(double)",
            "declare i32 @buraaq_grid_zeros(i32, i32)",
            "declare i32 @buraaq_grid_ones(i32, i32)",
            "declare i32 @buraaq_grid_fill(i32, i32, double)",
            "declare i32 @buraaq_grid_eye(i32)",
            "declare i32 @buraaq_grid_linspace(double, double, i32)",
            "declare i32 @buraaq_grid_row(i8*)",
            "declare i32 @buraaq_grid_nrows(i32)",
            "declare i32 @buraaq_grid_ncols(i32)",
            "declare double @buraaq_grid_at(i32, i32, i32)",
            "declare i32 @buraaq_grid_put_at(i32, i32, i32, double)",
            "declare i32 @buraaq_grid_plus(i32, i32)",
            "declare i32 @buraaq_grid_times(i32, i32)",
            "declare i32 @buraaq_grid_scale(i32, double)",
            "declare i32 @buraaq_grid_matmul(i32, i32)",
            "declare double @buraaq_grid_dot(i32, i32)",
            "declare i32 @buraaq_grid_transpose(i32)",
            "declare double @buraaq_grid_sum(i32)",
            "declare double @buraaq_grid_mean(i32)",
            "declare double @buraaq_grid_least(i32)",
            "declare double @buraaq_grid_most(i32)",
            "declare double @buraaq_grid_norm(i32)",
            "declare i32 @buraaq_grid_sin_all(i32)",
            "declare i32 @buraaq_grid_cos_all(i32)",
            "declare i8* @buraaq_grid_view(i32)",
            "declare i32 @buraaq_hold_new(i8*)",
            "declare i32 @buraaq_hold_stow(i32, i8*)",
            "declare i32 @buraaq_hold_rows(i32)",
            "declare i32 @buraaq_hold_cols(i32)",
            "declare i8* @buraaq_hold_pick(i32, i8*, i32)",
            "declare i32 @buraaq_hold_keep(i32, i8*, i8*)",
            "declare double @buraaq_hold_col_mean(i32, i8*)",
            "declare double @buraaq_hold_col_sum(i32, i8*)",
            "declare i32 @buraaq_hold_order(i32, i8*)",
            "declare i32 @buraaq_hold_from_csv(i8*)",
            "declare i32 @buraaq_hold_to_csv(i32, i8*)",
            "declare i8* @buraaq_hold_view(i32)",
            "declare i32 @buraaq_stream_bind(i32)",
            "declare i32 @buraaq_stream_hail()",
            "declare i32 @buraaq_stream_wire(i8*)",
            "declare i32 @buraaq_stream_say(i32, i8*)",
            "declare i8* @buraaq_stream_hear(i32)",
            "declare void @buraaq_stream_hangup(i32)",
            "declare void @buraaq_stream_run()",
            "declare i64 @buraaq_time_now_ms()",
            "declare void @buraaq_time_sleep_ms(i64)",
            "declare i8* @buraaq_os_getenv(i8*)",
            "declare i32 @buraaq_os_argc()",
            "declare i8* @buraaq_os_argv(i32)",
            "declare i8* @buraaq_json_parse_string_field(i8*, i8*)",
            "declare i8* @buraaq_http_get_body(i8*)",
            "declare i32 @buraaq_http_listen(i32, i32)",
            "declare i32 @buraaq_http_accept(i32)",
            "declare i8* @buraaq_http_method(i32)",
            "declare i8* @buraaq_http_path(i32)",
            "declare i8* @buraaq_http_body(i32)",
            "declare i32 @buraaq_http_path_starts(i32, i8*)",
            "declare i32 @buraaq_http_path_id(i32)",
            "declare i8* @buraaq_http_json_field(i32, i8*)",
            "declare i32 @buraaq_http_reply(i32, i32, i8*, i8*)",
            "declare void @buraaq_http_close(i32)",
            "declare i32 @buraaq_pg_connect(i8*)",
            "declare i32 @buraaq_pg_ok()",
            "declare i8* @buraaq_pg_exec(i8*)",
            "declare i8* @buraaq_pg_quote(i8*)",
            "declare void @buraaq_pg_close()",
            "declare i32 @buraaq_svc_page(i8*, i8*)",
            "declare i32 @buraaq_svc_api(i8*, i8*)",
            "declare i32 @buraaq_svc_store(i8*)",
            "declare i32 @buraaq_svc_key(i8*)",
            "declare i32 @buraaq_svc_origin(i8*)",
            "declare i32 @buraaq_svc_run(i32)",
            "declare i32 @buraaq_ui_app(i8*, i32, i32)",
            "declare i32 @buraaq_ui_heading(i8*)",
            "declare i32 @buraaq_ui_note(i8*)",
            "declare i32 @buraaq_ui_field(i8*, i8*)",
            "declare i32 @buraaq_ui_button(i8*, i8*)",
            "declare i32 @buraaq_ui_bind(i8*, i8*)",
            "declare i32 @buraaq_ui_keep(i8*)",
            "declare i32 @buraaq_ui_show()",
            "declare i8* @buraaq_ui_value(i8*)",
            "declare i32 @buraaq_ui_clicked(i8*)",
            "declare i32 @buraaq_flow_desk(i8*, i32, i32)",
            "declare i32 @buraaq_flow_show()",
            "declare i8* @buraaq_crypto_sha256_hex(i8*)",
            "declare i8* @buraaq_mutex_new()",
            "declare void @buraaq_mutex_lock(i8*)",
            "declare void @buraaq_mutex_unlock(i8*)",
            "declare void @buraaq_mutex_free(i8*)",
            "declare i32 @buraaq_process_exit_code(i8*)",
            "declare i32 @buraaq_text_eq(i8*, i8*)",
            "declare i32 @buraaq_text_byte(i8*, i32)",
            "declare double @buraaq_now_sec()",
            "declare i32 @buraaq_keepalive_i32(i32)",
            "declare double @buraaq_keepalive_f64(double)",
            "declare void @buraaq_bench_report(i8*, double, i32, i32)",
            "declare void @buraaq_bench_report_f64(i8*, double, i32, double)",
            "declare i8* @buraaq_text_slice(i8*, i32, i32)",
            "declare void @buraaq_rt_set_args(i32, i8**)",
        ];
        for d in decls {
            writeln!(self.out, "{d}").unwrap();
        }
        writeln!(self.out).unwrap();
    }

    fn emit_extern_decls(&mut self) {
        for ex in &self.module.externs {
            if ex.name.starts_with("buraaq_") {
                continue;
            }
            let ret = self.mir_ty_to_llvm(&ex.ret);
            let mut params = String::new();
            for (i, ty) in ex.params.iter().enumerate() {
                if i > 0 {
                    params.push_str(", ");
                }
                params.push_str(&self.mir_ty_to_llvm(ty));
            }
            if ex.variadic {
                if !params.is_empty() {
                    params.push_str(", ");
                }
                params.push_str("...");
            }
            writeln!(self.out, "declare {ret} @{name}({params})", name = ex.name).unwrap();
        }
        if !self.module.externs.is_empty() {
            writeln!(self.out).unwrap();
        }
    }

    fn emit_struct_types(&mut self) {
        for st in &self.module.structs {
            let llvm_name = self.struct_llvm_name(&st.name);
            let mut fields = String::new();
            for (_, ty) in &st.fields {
                if !fields.is_empty() {
                    fields.push_str(", ");
                }
                fields.push_str(&self.mir_ty_to_llvm(ty));
            }
            writeln!(
                self.out,
                "%{llvm_name} = type {{ {fields} }}"
            )
            .unwrap();
            self.struct_types.insert(st.name.clone(), llvm_name);
        }
        writeln!(self.out).unwrap();
    }

    fn emit_string_globals_prescan(&mut self) -> Result<(), CodegenError> {
        for func in &self.module.functions {
            for bb in &func.blocks {
                for stmt in &bb.stmts {
                    if let Statement::Assign { rvalue, .. } = stmt {
                        self.prescan_rvalue_strings(rvalue)?;
                    }
                }
                self.prescan_term_strings(&bb.terminator)?;
            }
        }
        Ok(())
    }

    fn prescan_rvalue_strings(&mut self, rv: &Rvalue) -> Result<(), CodegenError> {
        match rv {
            Rvalue::Literal(Constant::Str(s)) => {
                self.intern_string(s);
            }
            Rvalue::Use(Operand::Constant(Constant::Str(s))) => {
                self.intern_string(s);
            }
            Rvalue::Binary { left, right, .. } => {
                self.prescan_operand_strings(left)?;
                self.prescan_operand_strings(right)?;
            }
            Rvalue::Unary { operand, .. } => self.prescan_operand_strings(operand)?,
            Rvalue::Call { args, .. } => {
                for a in args {
                    self.prescan_operand_strings(a)?;
                }
            }
            Rvalue::Aggregate { fields, .. } => {
                for f in fields {
                    self.prescan_operand_strings(f)?;
                }
            }
            Rvalue::Field { base, .. } => self.prescan_operand_strings(base)?,
            Rvalue::Index { base, index, .. } => {
                self.prescan_operand_strings(base)?;
                self.prescan_operand_strings(index)?;
            }
            Rvalue::Cast { operand, .. } => self.prescan_operand_strings(operand)?,
            Rvalue::HeapAlloc { count, .. } => self.prescan_operand_strings(count)?,
            Rvalue::Load { ptr, .. } => self.prescan_operand_strings(ptr)?,
            Rvalue::Use(op) => {
                self.prescan_operand_strings(op)?;
            }
            Rvalue::Literal(Constant::Str(s)) => {
                self.intern_string(s);
            }
            Rvalue::Literal(_) | Rvalue::AddrOf { .. } => {}
        }
        Ok(())
    }

    fn prescan_term_strings(&mut self, term: &Terminator) -> Result<(), CodegenError> {
        match term {
            Terminator::Return(Some(op)) => self.prescan_operand_strings(op)?,
            Terminator::If { cond, .. } => self.prescan_operand_strings(cond)?,
            Terminator::Switch { discr, .. } => self.prescan_operand_strings(discr)?,
            _ => {}
        }
        Ok(())
    }

    fn prescan_operand_strings(&mut self, op: &Operand) -> Result<(), CodegenError> {
        if let Operand::Constant(Constant::Str(s)) = op {
            self.intern_string(s);
        }
        Ok(())
    }

    fn intern_string(&mut self, s: &str) -> String {
        if let Some(name) = self.string_names.get(s) {
            return name.clone();
        }
        let escaped = llvm_c_string_body(s);
        let byte_len = s.len() + 1;
        let name = format!("@.buraaq.str.{}", self.string_names.len());
        let global = format!(
            "{name} = private unnamed_addr constant [{byte_len} x i8] c\"{escaped}\\00\""
        );
        self.string_names.insert(s.to_string(), name.clone());
        self.string_globals.push(global);
        name
    }

    fn emit_c_main(&mut self, entry: &MirFunction) -> Result<(), CodegenError> {
        writeln!(
            self.out,
            "define i32 @main(i32 %argc, i8** %argv){} {{",
            self.fn_attr_suffix()
        )
        .unwrap();
        writeln!(self.out, "entry:").unwrap();
        writeln!(self.out, "  call void @buraaq_rt_set_args(i32 %argc, i8** %argv)").unwrap();
        let mangled = Self::mangle(&entry.name);
        if entry.return_ty == MirTy::Void {
            writeln!(self.out, "  call void @{mangled}()").unwrap();
            writeln!(self.out, "  ret i32 0").unwrap();
        } else {
            let tmp = self.fresh_tmp();
            let ret_ty = self.mir_ty_to_llvm(&entry.return_ty);
            writeln!(self.out, "  {tmp} = call {ret_ty} @{mangled}()").unwrap();
            if entry.return_ty == MirTy::I32 {
                writeln!(self.out, "  ret i32 {tmp}").unwrap();
            } else {
                writeln!(self.out, "  ret i32 0").unwrap();
            }
        }
        writeln!(self.out, "}}").unwrap();
        Ok(())
    }

    fn emit_function(&mut self, func: &MirFunction) -> Result<(), CodegenError> {
        let ret_ty = self.mir_ty_to_llvm(&func.return_ty);
        let mut params = String::new();
        for (i, (_, ty)) in func.params.iter().enumerate() {
            if i > 0 {
                params.push_str(", ");
            }
            params.push_str(&format!("{} %p{i}", self.mir_ty_to_llvm(ty)));
        }
        let mangled = Self::mangle(&func.name);
        let attrs = self.fn_attr_suffix();
        writeln!(
            self.out,
            "define internal {ret_ty} @{mangled}({params}){attrs} {{"
        )
        .unwrap();
        self.emit_function_body(func)?;
        writeln!(self.out, "}}").unwrap();
        writeln!(self.out).unwrap();
        Ok(())
    }

    fn emit_function_body(&mut self, func: &MirFunction) -> Result<String, CodegenError> {
        let mut ctx = LocalCtx::default();
        for (i, local) in func.locals.iter().enumerate() {
            ctx.types.insert(i as u32, local.ty.clone());
            if local.ty == MirTy::Void {
                continue;
            }
            let ty = self.mir_ty_to_llvm(&local.ty);
            writeln!(self.out, "  %l{i} = alloca {ty}, align 8").unwrap();
            ctx.slots.insert(i as u32, format!("%l{i}"));
        }
        for (i, (_, ty)) in func.params.iter().enumerate() {
            let llvm_ty = self.mir_ty_to_llvm(ty);
            writeln!(
                self.out,
                "  store {llvm_ty} %p{i}, {llvm_ty}* %l{i}, align 8"
            )
            .unwrap();
        }
        if let Some(first) = func.blocks.first() {
            writeln!(self.out, "  br label %bb{}", first.id).unwrap();
        }

        let loop_headers = loop_header_ids(func);
        let loop_latches = loop_latch_ids(func, &loop_headers);
        let mut last_val = "0".to_string();
        for bb in &func.blocks {
            ctx.cache.clear();
            writeln!(self.out, "bb{}:", bb.id).unwrap();
            for stmt in &bb.stmts {
                last_val = self.emit_stmt(stmt, &mut ctx)?;
            }
            last_val =
                self.emit_term(&bb.terminator, &mut ctx, &func.return_ty, bb.id, &loop_latches)?;
        }
        Ok(last_val)
    }

    fn emit_stmt(&mut self, stmt: &Statement, ctx: &mut LocalCtx) -> Result<String, CodegenError> {
        match stmt {
            Statement::Assign { dest, rvalue } => {
                let val = self.emit_rvalue(rvalue, ctx)?;
                let dest_ty = ctx
                    .types
                    .get(dest)
                    .cloned()
                    .unwrap_or_else(|| self.rvalue_ty(rvalue, ctx));
                if dest_ty == MirTy::Void || !ctx.slots.contains_key(dest) {
                    return Ok(val);
                }
                ctx.cache.insert(*dest, val.clone());
                let ty = self.mir_ty_to_llvm(&dest_ty);
                let dest_slot = ctx.slots.get(dest).ok_or_else(|| {
                    CodegenError::Msg(format!("unknown local {dest}"))
                })?;
                writeln!(self.out, "  store {ty} {val}, {ty}* {dest_slot}, align 8").unwrap();
                Ok(val)
            }
            Statement::Store { ptr, value, ty } => {
                let p = self.emit_operand(ptr, ctx)?;
                let v = self.emit_operand(value, ctx)?;
                let ft = self.mir_ty_to_llvm(ty);
                writeln!(self.out, "  store {ft} {v}, {ft}* {p}, align 8").unwrap();
                Ok(v)
            }
            Statement::StorageLive { .. } | Statement::StorageDead { .. } => Ok("0".into()),
        }
    }

    fn emit_term(
        &mut self,
        term: &Terminator,
        ctx: &mut LocalCtx,
        ret_ty: &MirTy,
        current_bb: BlockId,
        loop_latches: &HashSet<BlockId>,
    ) -> Result<String, CodegenError> {
        match term {
            Terminator::Return(v) => {
                if *ret_ty == MirTy::Void {
                    writeln!(self.out, "  ret void").unwrap();
                } else if let Some(op) = v {
                    let mut val = self.emit_operand(op, ctx)?;
                    let ty = self.mir_ty_to_llvm(ret_ty);
                    if val == "0"
                        && matches!(
                            ret_ty,
                            MirTy::Text | MirTy::Bytes | MirTy::Ptr(_) | MirTy::Ref { .. }
                        )
                    {
                        val = "null".into();
                    }
                    writeln!(self.out, "  ret {ty} {val}").unwrap();
                    return Ok(val);
                } else {
                    writeln!(self.out, "  ret i32 0").unwrap();
                }
                Ok("0".into())
            }
            Terminator::Goto(bb) => {
                if loop_latches.contains(&current_bb) {
                    let md = self.fresh_loop_md();
                    writeln!(self.out, "  br label %bb{bb}, !llvm.loop !{md}").unwrap();
                } else {
                    writeln!(self.out, "  br label %bb{bb}").unwrap();
                }
                Ok("0".into())
            }
            Terminator::If { cond, then_bb, else_bb } => {
                let c = self.emit_operand(cond, ctx)?;
                writeln!(
                    self.out,
                    "  br i1 {c}, label %bb{then_bb}, label %bb{else_bb}"
                )
                .unwrap();
                Ok("0".into())
            }
            Terminator::Switch { discr, arms, otherwise } => {
                let d = self.emit_operand(discr, ctx)?;
                let mut cases = String::new();
                for (val, bb) in arms {
                    cases.push_str(&format!("i32 {val}, label %bb{bb} "));
                }
                writeln!(
                    self.out,
                    "  switch i32 {d}, label %bb{otherwise} [ {cases}]"
                )
                .unwrap();
                Ok("0".into())
            }
            Terminator::Unreachable => {
                writeln!(self.out, "  unreachable").unwrap();
                Ok("0".into())
            }
        }
    }

    fn emit_rvalue(&mut self, rv: &Rvalue, ctx: &mut LocalCtx) -> Result<String, CodegenError> {
        match rv {
            Rvalue::Use(op) => self.emit_operand(op, ctx),
            Rvalue::Literal(c) => self.emit_constant(c),
            Rvalue::Binary { op, left, right } => {
                let lty = self.operand_ty(left, ctx);
                let rty = self.operand_ty(right, ctx);
                if matches!(lty, MirTy::Text) || matches!(rty, MirTy::Text) {
                    let l = self.emit_operand(left, ctx)?;
                    let r = self.emit_operand(right, ctx)?;
                    let t = self.fresh_tmp();
                    writeln!(self.out, "  {t} = call i32 @buraaq_text_eq(i8* {l}, i8* {r})").unwrap();
                    let bit = self.fresh_tmp();
                    match op {
                        BinOp::Eq => {
                            writeln!(self.out, "  {bit} = icmp eq i32 {t}, 0").unwrap();
                        }
                        _ => {
                            writeln!(self.out, "  {bit} = icmp ne i32 {t}, 0").unwrap();
                        }
                    }
                    return Ok(bit);
                }
                let floaty = matches!(lty, MirTy::F64 | MirTy::F32)
                    || matches!(rty, MirTy::F64 | MirTy::F32);
                let mut l = self.emit_operand(left, ctx)?;
                let mut r = self.emit_operand(right, ctx)?;
                if floaty {
                    if !matches!(lty, MirTy::F64 | MirTy::F32) {
                        let c = self.fresh_tmp();
                        writeln!(self.out, "  {c} = sitofp i32 {l} to double").unwrap();
                        l = c;
                    }
                    if !matches!(rty, MirTy::F64 | MirTy::F32) {
                        let c = self.fresh_tmp();
                        writeln!(self.out, "  {c} = sitofp i32 {r} to double").unwrap();
                        r = c;
                    }
                    let tmp = self.fresh_tmp();
                    match op {
                        BinOp::Add => writeln!(self.out, "  {tmp} = fadd double {l}, {r}").unwrap(),
                        BinOp::Sub => writeln!(self.out, "  {tmp} = fsub double {l}, {r}").unwrap(),
                        BinOp::Mul => writeln!(self.out, "  {tmp} = fmul double {l}, {r}").unwrap(),
                        BinOp::Div => writeln!(self.out, "  {tmp} = fdiv double {l}, {r}").unwrap(),
                        BinOp::Eq => {
                            writeln!(self.out, "  {tmp} = fcmp oeq double {l}, {r}").unwrap();
                        }
                        BinOp::NotEq => {
                            writeln!(self.out, "  {tmp} = fcmp one double {l}, {r}").unwrap();
                        }
                        BinOp::Lt => {
                            writeln!(self.out, "  {tmp} = fcmp olt double {l}, {r}").unwrap();
                        }
                        BinOp::Le => {
                            writeln!(self.out, "  {tmp} = fcmp ole double {l}, {r}").unwrap();
                        }
                        BinOp::Gt => {
                            writeln!(self.out, "  {tmp} = fcmp ogt double {l}, {r}").unwrap();
                        }
                        BinOp::Ge => {
                            writeln!(self.out, "  {tmp} = fcmp oge double {l}, {r}").unwrap();
                        }
                        _ => writeln!(self.out, "  {tmp} = fadd double {l}, {r}").unwrap(),
                    }
                    return Ok(tmp);
                }
                let tmp = self.fresh_tmp();
                let add_nuw = matches!(
                    (left, right),
                    (
                        Operand::Local(_),
                        Operand::Constant(Constant::I32(k))
                    ) if (1..256).contains(k)
                ) || matches!(
                    (left, right),
                    (
                        Operand::Constant(Constant::I32(k)),
                        Operand::Local(_)
                    ) if (1..256).contains(k)
                );
                let inst = match op {
                    BinOp::Add if add_nuw => "add nuw nsw",
                    BinOp::Add => "add nsw",
                    BinOp::Sub => "sub nsw",
                    BinOp::Mul => "mul",
                    BinOp::Div => "sdiv",
                    BinOp::Mod => "srem",
                    BinOp::Eq => {
                        writeln!(self.out, "  {tmp} = icmp eq i32 {l}, {r}").unwrap();
                        return Ok(tmp);
                    }
                    BinOp::NotEq => {
                        writeln!(self.out, "  {tmp} = icmp ne i32 {l}, {r}").unwrap();
                        return Ok(tmp);
                    }
                    BinOp::Lt => {
                        writeln!(self.out, "  {tmp} = icmp slt i32 {l}, {r}").unwrap();
                        return Ok(tmp);
                    }
                    BinOp::Le => {
                        writeln!(self.out, "  {tmp} = icmp sle i32 {l}, {r}").unwrap();
                        return Ok(tmp);
                    }
                    BinOp::Gt => {
                        writeln!(self.out, "  {tmp} = icmp sgt i32 {l}, {r}").unwrap();
                        return Ok(tmp);
                    }
                    BinOp::Ge => {
                        writeln!(self.out, "  {tmp} = icmp sge i32 {l}, {r}").unwrap();
                        return Ok(tmp);
                    }
                    BinOp::And => {
                        let lb = self.as_i1(l, &lty);
                        let rb = self.as_i1(r, &rty);
                        writeln!(self.out, "  {tmp} = and i1 {lb}, {rb}").unwrap();
                        return Ok(tmp);
                    }
                    BinOp::Or => {
                        let lb = self.as_i1(l, &lty);
                        let rb = self.as_i1(r, &rty);
                        writeln!(self.out, "  {tmp} = or i1 {lb}, {rb}").unwrap();
                        return Ok(tmp);
                    }
                };
                let l = self.as_i32(l, &lty);
                let r = self.as_i32(r, &rty);
                writeln!(self.out, "  {tmp} = {inst} i32 {l}, {r}").unwrap();
                Ok(tmp)
            }
            Rvalue::Unary { op, operand } => {
                let o = self.emit_operand(operand, ctx)?;
                let tmp = self.fresh_tmp();
                match op {
                    UnOp::Neg if matches!(self.operand_ty(operand, ctx), MirTy::F64 | MirTy::F32) => {
                        writeln!(self.out, "  {tmp} = fneg double {o}").unwrap();
                    }
                    UnOp::Neg => writeln!(self.out, "  {tmp} = sub nsw i32 0, {o}").unwrap(),
                    UnOp::Not => writeln!(self.out, "  {tmp} = xor i1 {o}, true").unwrap(),
                }
                Ok(tmp)
            }
            Rvalue::Call { func, args, ret_ty } => {
                if self.module.functions.iter().any(|f| f.name == *func) {
                    let mangled = Self::mangle(func);
                    let callee = self
                        .module
                        .functions
                        .iter()
                        .find(|f| f.name == *func)
                        .unwrap();
                    let mut arg_str = String::new();
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 {
                            arg_str.push_str(", ");
                        }
                        let val = self.emit_operand(a, ctx)?;
                        let ty = callee
                            .params
                            .get(i)
                            .map(|(_, t)| self.mir_ty_to_llvm(t))
                            .unwrap_or_else(|| "i32".into());
                        arg_str.push_str(&format!("{ty} {val}"));
                    }
                    let ret = self.mir_ty_to_llvm(ret_ty);
                    if *ret_ty == MirTy::Void {
                        writeln!(self.out, "  call void @{mangled}({arg_str})").unwrap();
                        Ok("0".into())
                    } else {
                        let tmp = self.fresh_tmp();
                        writeln!(
                            self.out,
                            "  {tmp} = call {ret} @{mangled}({arg_str})"
                        )
                        .unwrap();
                        Ok(tmp)
                    }
                } else {
                    let mut arg_str = String::new();
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 {
                            arg_str.push_str(", ");
                        }
                        let val = self.emit_operand(a, ctx)?;
                        let ty = match a {
                            Operand::Constant(Constant::Str(_))
                            | Operand::Constant(Constant::FnAddr(_))
                            | Operand::Constant(Constant::NullPtr(_)) => "i8*".to_string(),
                            Operand::Constant(Constant::F64(_))
                            | Operand::Constant(Constant::F32(_)) => "double".to_string(),
                            Operand::Constant(Constant::I64(_)) => "i64".to_string(),
                            Operand::Constant(Constant::Bool(_)) => "i1".to_string(),
                            Operand::Local(id) => match ctx.types.get(id) {
                                Some(MirTy::Text)
                                | Some(MirTy::Bytes)
                                | Some(MirTy::Ptr(_)) => "i8*".to_string(),
                                Some(MirTy::Ref { inner, .. }) => {
                                    format!("{}*", self.mir_ty_to_llvm(inner))
                                }
                                Some(MirTy::I64) => "i64".to_string(),
                                Some(MirTy::F64) | Some(MirTy::F32) => "double".to_string(),
                                Some(MirTy::Bool) => "i1".to_string(),
                                _ => "i32".to_string(),
                            },
                            _ => "i32".to_string(),
                        };
                        arg_str.push_str(&format!("{ty} {val}"));
                    }
                    let tmp = self.fresh_tmp();
                    if *ret_ty == MirTy::Void {
                        writeln!(self.out, "  call void @{func}({arg_str})").unwrap();
                        Ok("0".into())
                    } else {
                        let ret = self.mir_ty_to_llvm(ret_ty);
                        writeln!(self.out, "  {tmp} = call {ret} @{func}({arg_str})").unwrap();
                        Ok(tmp)
                    }
                }
            }
            Rvalue::Aggregate { ty, fields } => {
                if let MirTy::Array { elem, len } = ty {
                    let et = self.mir_ty_to_llvm(elem);
                    let arr_ty = format!("[{len} x {et}]");
                    let tmp = self.fresh_tmp();
                    writeln!(self.out, "  {tmp} = alloca {arr_ty}, align 8").unwrap();
                    for (i, f) in fields.iter().enumerate() {
                        let val = self.emit_operand(f, ctx)?;
                        let fp = self.fresh_tmp();
                        writeln!(
                            self.out,
                            "  {fp} = getelementptr inbounds {arr_ty}, {arr_ty}* {tmp}, i32 0, i32 {i}"
                        )
                        .unwrap();
                        writeln!(self.out, "  store {et} {val}, {et}* {fp}, align 8").unwrap();
                    }
                    let loaded = self.fresh_tmp();
                    writeln!(
                        self.out,
                        "  {loaded} = load {arr_ty}, {arr_ty}* {tmp}, align 8"
                    )
                    .unwrap();
                    return Ok(loaded);
                }
                let struct_name = match ty {
                    MirTy::Struct { name } => self.struct_llvm_name(name),
                    _ => return Err(CodegenError::Msg("aggregate on non-struct".into())),
                };
                let tmp = self.fresh_tmp();
                writeln!(self.out, "  {tmp} = alloca %{struct_name}, align 8").unwrap();
                for (i, f) in fields.iter().enumerate() {
                    let val = self.emit_operand(f, ctx)?;
                    let field_ty = self.field_ty(ty, i);
                    let ft = self.mir_ty_to_llvm(&field_ty);
                    let fp = self.fresh_tmp();
                    writeln!(
                        self.out,
                        "  {fp} = getelementptr inbounds %{struct_name}, %{struct_name}* {tmp}, i32 0, i32 {i}"
                    )
                    .unwrap();
                    writeln!(self.out, "  store {ft} {val}, {ft}* {fp}, align 8").unwrap();
                }
                let loaded = self.fresh_tmp();
                writeln!(
                    self.out,
                    "  {loaded} = load %{struct_name}, %{struct_name}* {tmp}, align 8"
                )
                .unwrap();
                Ok(loaded)
            }
            Rvalue::Field { base, field_index, ty } => {
                let struct_name = match base {
                    Operand::Local(id) => match ctx.types.get(id) {
                        Some(MirTy::Struct { name }) => self.struct_llvm_name(name),
                        _ => return Err(CodegenError::Msg("field base must be struct local".into())),
                    },
                    _ => return Err(CodegenError::Msg("field base must be local".into())),
                };
                let b = self.emit_operand(base, ctx)?;
                let ft = self.mir_ty_to_llvm(ty);
                let base_ptr = self.fresh_tmp();
                writeln!(
                    self.out,
                    "  {base_ptr} = alloca %{struct_name}, align 8"
                )
                .unwrap();
                writeln!(
                    self.out,
                    "  store %{struct_name} {b}, %{struct_name}* {base_ptr}, align 8"
                )
                .unwrap();
                let tmp = self.fresh_tmp();
                writeln!(
                    self.out,
                    "  {tmp} = getelementptr inbounds %{struct_name}, %{struct_name}* {base_ptr}, i32 0, i32 {field_index}"
                )
                .unwrap();
                let val = self.fresh_tmp();
                writeln!(self.out, "  {val} = load {ft}, {ft}* {tmp}, align 8").unwrap();
                Ok(val)
            }
            Rvalue::HeapAlloc { elem_ty, count } => {
                let c = self.emit_operand(count, ctx)?;
                let size = self.type_size(elem_ty);
                let tmp = self.fresh_tmp();
                writeln!(self.out, "  {tmp} = mul i64 {c}, {size}").unwrap();
                let ptr = self.fresh_tmp();
                writeln!(self.out, "  {ptr} = call i8* @buraaq_alloc(i64 {tmp})").unwrap();
                Ok(ptr)
            }
            Rvalue::Load { ptr, ty } => {
                let p = self.emit_operand(ptr, ctx)?;
                let ft = self.mir_ty_to_llvm(ty);
                let tmp = self.fresh_tmp();
                writeln!(self.out, "  {tmp} = load {ft}, {ft}* {p}, align 8").unwrap();
                Ok(tmp)
            }
            Rvalue::AddrOf { local, ty } => {
                let slot = ctx.slots.get(local).ok_or_else(|| {
                    CodegenError::Msg(format!("unknown local {local}"))
                })?;
                let ft = self.mir_ty_to_llvm(ty);
                let tmp = self.fresh_tmp();
                writeln!(self.out, "  {tmp} = bitcast {ft}* {slot} to {ft}*").unwrap();
                Ok(tmp)
            }
            Rvalue::Index { base, index, elem_ty } => {
                let idx = self.emit_operand(index, ctx)?;
                let ft = self.mir_ty_to_llvm(elem_ty);
                if let Operand::Local(id) = base {
                    if let Some(MirTy::Array { elem, len }) = ctx.types.get(id) {
                        let slot = ctx.slots.get(id).ok_or_else(|| {
                            CodegenError::Msg(format!("unknown local {id}"))
                        })?;
                        let et = self.mir_ty_to_llvm(elem);
                        let arr_ty = format!("[{len} x {et}]");
                        let tmp = self.fresh_tmp();
                        writeln!(
                            self.out,
                            "  {tmp} = getelementptr inbounds {arr_ty}, {arr_ty}* {slot}, i32 0, i32 {idx}"
                        )
                        .unwrap();
                        let val = self.fresh_tmp();
                        writeln!(self.out, "  {val} = load {et}, {et}* {tmp}, align 8").unwrap();
                        return Ok(val);
                    }
                }
                let b = self.emit_operand(base, ctx)?;
                let tmp = self.fresh_tmp();
                writeln!(
                    self.out,
                    "  {tmp} = getelementptr inbounds {ft}, {ft}* {b}, i32 {idx}"
                )
                .unwrap();
                let val = self.fresh_tmp();
                writeln!(self.out, "  {val} = load {ft}, {ft}* {tmp}, align 8").unwrap();
                Ok(val)
            }
            Rvalue::Cast { operand, to, .. } => {
                let o = self.emit_operand(operand, ctx)?;
                let tmp = self.fresh_tmp();
                let to_ty = self.mir_ty_to_llvm(to);
                writeln!(self.out, "  {tmp} = bitcast i32 {o} to {to_ty}").unwrap();
                Ok(tmp)
            }
        }
    }

    fn emit_operand(&mut self, op: &Operand, ctx: &mut LocalCtx) -> Result<String, CodegenError> {
        match op {
            Operand::Local(id) => {
                if ctx.types.get(id) == Some(&MirTy::Void) || !ctx.slots.contains_key(id) {
                    return Ok("0".into());
                }
                if let Some(hit) = ctx.cache.get(id) {
                    return Ok(hit.clone());
                }
                let slot = ctx.slots.get(id).ok_or_else(|| {
                    CodegenError::Msg(format!("unknown local {id}"))
                })?;
                let ty = ctx.types.get(id).cloned().unwrap_or(MirTy::I32);
                let llvm_ty = self.mir_ty_to_llvm(&ty);
                let tmp = self.fresh_tmp();
                writeln!(
                    self.out,
                    "  {tmp} = load {llvm_ty}, {llvm_ty}* {slot}, align 8"
                )
                .unwrap();
                ctx.cache.insert(*id, tmp.clone());
                Ok(tmp)
            }
            Operand::Constant(c) => self.emit_constant(c),
        }
    }

    fn emit_constant(&mut self, c: &Constant) -> Result<String, CodegenError> {
        Ok(match c {
            Constant::Void => "0".into(),
            Constant::Bool(b) => if *b { "1" } else { "0" }.into(),
            Constant::I32(n) => n.to_string(),
            Constant::I64(n) => n.to_string(),
            Constant::F32(n) => llvm_float_const(*n as f64),
            Constant::F64(n) => llvm_float_const(*n),
            Constant::Str(s) => {
                let global = self.intern_string(s);
                let tmp = self.fresh_tmp();
                writeln!(
                    self.out,
                    "  {tmp} = getelementptr inbounds [{len} x i8], [{len} x i8]* {global}, i64 0, i64 0",
                    len = s.len() + 1
                )
                .unwrap();
                tmp
            }
            Constant::NullPtr(_) => "null".into(),
            Constant::FnAddr(name) => {
                let tmp = self.fresh_tmp();
                let mangled = Self::mangle(name);
                writeln!(
                    self.out,
                    "  {tmp} = bitcast void (i8*)* @{mangled} to i8*"
                )
                .unwrap();
                tmp
            }
        })
    }

    fn rvalue_ty(&self, rv: &Rvalue, ctx: &LocalCtx) -> MirTy {
        match rv {
            Rvalue::Call { ret_ty, .. } => ret_ty.clone(),
            Rvalue::Aggregate { ty, .. } => ty.clone(),
            Rvalue::Field { ty, .. } => ty.clone(),
            Rvalue::Use(Operand::Local(id)) => ctx.types.get(id).cloned().unwrap_or(MirTy::I32),
            Rvalue::Use(Operand::Constant(Constant::Str(_))) | Rvalue::Literal(Constant::Str(_)) => {
                MirTy::Text
            }
            Rvalue::Use(Operand::Constant(Constant::Bool(_))) | Rvalue::Literal(Constant::Bool(_)) => {
                MirTy::Bool
            }
            Rvalue::Use(Operand::Constant(Constant::I64(_))) | Rvalue::Literal(Constant::I64(_)) => {
                MirTy::I64
            }
            Rvalue::Use(Operand::Constant(Constant::F64(_))) | Rvalue::Literal(Constant::F64(_)) => {
                MirTy::F64
            }
            Rvalue::Binary {
                op: BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::And | BinOp::Or,
                ..
            } => MirTy::Bool,
            _ => MirTy::I32,
        }
    }

    fn field_ty(&self, agg: &MirTy, index: usize) -> MirTy {
        if let MirTy::Struct { name } = agg {
            if let Some(st) = self.module.structs.iter().find(|s| s.name == *name) {
                if let Some((_, ty)) = st.fields.get(index) {
                    return ty.clone();
                }
            }
        }
        MirTy::I32
    }

    fn as_i1(&mut self, val: String, ty: &MirTy) -> String {
        if matches!(ty, MirTy::Bool) {
            return val;
        }
        let t = self.fresh_tmp();
        writeln!(self.out, "  {t} = icmp ne i32 {val}, 0").unwrap();
        t
    }

    fn as_i32(&mut self, val: String, ty: &MirTy) -> String {
        if matches!(ty, MirTy::Bool) {
            let t = self.fresh_tmp();
            writeln!(self.out, "  {t} = zext i1 {val} to i32").unwrap();
            return t;
        }
        val
    }

    fn operand_ty(&self, op: &Operand, ctx: &LocalCtx) -> MirTy {
        match op {
            Operand::Local(id) => ctx.types.get(id).cloned().unwrap_or(MirTy::I32),
            Operand::Constant(Constant::F64(_)) | Operand::Constant(Constant::F32(_)) => MirTy::F64,
            Operand::Constant(Constant::Str(_)) => MirTy::Text,
            Operand::Constant(Constant::I64(_)) => MirTy::I64,
            Operand::Constant(Constant::Bool(_)) => MirTy::Bool,
            Operand::Constant(Constant::NullPtr(_)) => MirTy::ptr_to(MirTy::I8),
            _ => MirTy::I32,
        }
    }

    fn type_size(&self, ty: &MirTy) -> u64 {
        match ty {
            MirTy::I8 => 1,
            MirTy::I32 => 4,
            MirTy::I64 => 8,
            MirTy::F64 => 8,
            MirTy::Ptr(_) | MirTy::Ref { .. } => 8,
            MirTy::Struct { name } => {
                if let Some(st) = self.module.structs.iter().find(|s| s.name == *name) {
                    st.fields.iter().map(|(_, t)| self.type_size(t)).sum()
                } else {
                    8
                }
            }
            _ => 8,
        }
    }

    fn mir_ty_to_llvm(&self, ty: &MirTy) -> String {
        match ty {
            MirTy::Void => "void".into(),
            MirTy::Bool => "i1".into(),
            MirTy::I8 => "i8".into(),
            MirTy::I32 => "i32".into(),
            MirTy::I64 => "i64".into(),
            MirTy::F32 => "float".into(),
            MirTy::F64 => "double".into(),
            MirTy::Text | MirTy::Bytes => "i8*".into(),
            MirTy::Ptr(inner) => format!("{}*", self.mir_ty_to_llvm(inner)),
            MirTy::Ref { inner, .. } => format!("{}*", self.mir_ty_to_llvm(inner)),
            MirTy::Struct { name } => format!("%{}", self.struct_llvm_name(name)),
            MirTy::Enum { .. } => "i32".into(),
            MirTy::Array { elem, len } => {
                format!("[{} x {}]", len, self.mir_ty_to_llvm(elem))
            }
            MirTy::Slice { elem } => format!("{}*", self.mir_ty_to_llvm(elem)),
            MirTy::FnPtr { ret, .. } => format!("{} ()*", self.mir_ty_to_llvm(ret)),
        }
    }

    fn struct_llvm_name(&self, name: &str) -> String {
        self.struct_types
            .get(name)
            .cloned()
            .unwrap_or_else(|| format!("struct_{name}"))
    }

    fn mangle(name: &str) -> String {
        format!("buraaq_fn_{name}")
    }

    fn fresh_tmp(&mut self) -> String {
        self.tmp_counter += 1;
        format!("%t{}", self.tmp_counter)
    }
}

fn terminator_succs(term: &Terminator) -> Vec<BlockId> {
    match term {
        Terminator::Goto(bb) => vec![*bb],
        Terminator::If {
            then_bb, else_bb, ..
        } => vec![*then_bb, *else_bb],
        Terminator::Switch {
            arms, otherwise, ..
        } => {
            let mut s: Vec<BlockId> = arms.iter().map(|(_, bb)| *bb).collect();
            s.push(*otherwise);
            s
        }
        Terminator::Return(_) | Terminator::Unreachable => Vec::new(),
    }
}

/// While/for headers are `If` blocks that a successor can reach again.
fn loop_header_ids(func: &MirFunction) -> HashSet<BlockId> {
    let by_id: HashMap<BlockId, &BasicBlock> = func.blocks.iter().map(|b| (b.id, b)).collect();
    let mut headers = HashSet::new();
    for bb in &func.blocks {
        if !matches!(bb.terminator, Terminator::If { .. }) {
            continue;
        }
        if succ_can_reach_header(&by_id, bb.id) {
            headers.insert(bb.id);
        }
    }
    headers
}

fn loop_latch_ids(func: &MirFunction, headers: &HashSet<BlockId>) -> HashSet<BlockId> {
    let by_id: HashMap<BlockId, &BasicBlock> = func.blocks.iter().map(|b| (b.id, b)).collect();
    let mut latches = HashSet::new();
    for bb in &func.blocks {
        if let Terminator::Goto(dest) = &bb.terminator {
            if headers.contains(dest) && reachable_from(&by_id, *dest, bb.id) {
                latches.insert(bb.id);
            }
        }
    }
    latches
}

fn reachable_from(
    by_id: &HashMap<BlockId, &BasicBlock>,
    start: BlockId,
    target: BlockId,
) -> bool {
    if start == target {
        return true;
    }
    let mut stack = vec![start];
    let mut seen = HashSet::new();
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        if id == target {
            return true;
        }
        if let Some(bb) = by_id.get(&id) {
            stack.extend(terminator_succs(&bb.terminator));
        }
    }
    false
}

fn succ_can_reach_header(by_id: &HashMap<BlockId, &BasicBlock>, header: BlockId) -> bool {
    let Some(start) = by_id.get(&header) else {
        return false;
    };
    let mut stack = terminator_succs(&start.terminator);
    let mut seen = HashSet::new();
    while let Some(id) = stack.pop() {
        if id == header {
            return true;
        }
        if !seen.insert(id) {
            continue;
        }
        if let Some(bb) = by_id.get(&id) {
            stack.extend(terminator_succs(&bb.terminator));
        }
    }
    false
}

/// Escape decoded string bytes for an LLVM `c"..."` literal (not Rust `escape_default`).
fn llvm_c_string_body(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\0A"),
            '\r' => out.push_str("\\0D"),
            '\t' => out.push_str("\\09"),
            '\0' => out.push_str("\\00"),
            c if c.is_ascii() && c >= ' ' && c <= '~' => out.push(c),
            c => {
                for b in c.encode_utf8(&mut [0; 4]).bytes() {
                    out.push_str(&format!("\\{:02X}", b));
                }
            }
        }
    }
    out
}

/// LLVM rejects `1e0` (no decimal point). Always emit a decimal form.
fn llvm_float_const(n: f64) -> String {
    if !n.is_finite() {
        return format!("0x{:016X}", n.to_bits());
    }
    format!("{n:.16e}")
}

#[derive(Default)]
struct LocalCtx {
    slots: HashMap<u32, String>,
    types: HashMap<u32, MirTy>,
    cache: HashMap<u32, String>,
}
