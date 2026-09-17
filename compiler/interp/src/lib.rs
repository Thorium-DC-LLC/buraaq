//! Opt-in MIR interpreter for `buraaq script`.
//!
//! Same front half as AOT (parse → MIR). No GC: values are owned and dropped
//! with the frame. Unsupported MIR ops fail with a clear error so users switch
//! to `buraaq run` for full native builds.

use std::collections::HashMap;
use std::io::{self, Write};

use buraaq_mir::{
    BinOp, Constant, MirFunction, MirModule, MirTy, Operand, Rvalue, Statement, Terminator, UnOp,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InterpError {
    #[error("{0}")]
    Msg(String),
    #[error("script mode does not support `{0}` yet — use `buraaq run` for full native")]
    Unsupported(&'static str),
}

#[derive(Clone, Debug)]
enum Value {
    Void,
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(String),
    Struct(Vec<Value>),
}

impl Value {
    fn as_bool(&self) -> Result<bool, InterpError> {
        match self {
            Value::Bool(b) => Ok(*b),
            Value::I64(n) => Ok(*n != 0),
            _ => Err(InterpError::Msg("expected bool condition".into())),
        }
    }

    fn as_i64(&self) -> Result<i64, InterpError> {
        match self {
            Value::I64(n) => Ok(*n),
            Value::Bool(b) => Ok(if *b { 1 } else { 0 }),
            Value::F64(f) => Ok(*f as i64),
            _ => Err(InterpError::Msg("expected integer".into())),
        }
    }

    fn as_f64(&self) -> Result<f64, InterpError> {
        match self {
            Value::F64(f) => Ok(*f),
            Value::I64(n) => Ok(*n as f64),
            _ => Err(InterpError::Msg("expected float".into())),
        }
    }

    fn as_str(&self) -> Result<&str, InterpError> {
        match self {
            Value::Str(s) => Ok(s),
            _ => Err(InterpError::Msg("expected text".into())),
        }
    }
}

/// Interpret a lowered MIR module, calling the entry `main` (or first entry fn).
pub fn interpret_module(module: &MirModule) -> Result<i32, InterpError> {
    let entry = module
        .entry_function()
        .or_else(|| module.functions.iter().find(|f| f.name == "main"))
        .ok_or_else(|| InterpError::Msg("no main entry function".into()))?;
    let mut vm = Vm {
        module,
        fns: module
            .functions
            .iter()
            .map(|f| (f.name.clone(), f))
            .collect(),
    };
    match vm.call_fn(entry, vec![])? {
        Value::I64(n) => Ok(n as i32),
        Value::Void => Ok(0),
        Value::Bool(b) => Ok(if b { 0 } else { 1 }),
        _ => Ok(0),
    }
}

struct Vm<'a> {
    module: &'a MirModule,
    fns: HashMap<String, &'a MirFunction>,
}

impl<'a> Vm<'a> {
    fn call_fn(&mut self, func: &MirFunction, args: Vec<Value>) -> Result<Value, InterpError> {
        let mut locals: Vec<Value> = (0..func.locals.len()).map(|_| Value::Void).collect();
        for (i, arg) in args.into_iter().enumerate() {
            if i < locals.len() {
                locals[i] = arg;
            }
        }
        let mut bb = 0u32;
        let mut steps = 0u64;
        const MAX_STEPS: u64 = 50_000_000;
        loop {
            steps += 1;
            if steps > MAX_STEPS {
                return Err(InterpError::Msg(
                    "script exceeded step limit (infinite loop?)".into(),
                ));
            }
            let block = func
                .blocks
                .iter()
                .find(|b| b.id == bb)
                .ok_or_else(|| InterpError::Msg(format!("bad block {bb}")))?;
            for stmt in &block.stmts {
                self.exec_stmt(stmt, &mut locals)?;
            }
            match &block.terminator {
                Terminator::Return(None) => return Ok(Value::Void),
                Terminator::Return(Some(op)) => return self.operand(op, &locals),
                Terminator::Goto(t) => bb = *t,
                Terminator::If {
                    cond,
                    then_bb,
                    else_bb,
                } => {
                    bb = if self.operand(cond, &locals)?.as_bool()? {
                        *then_bb
                    } else {
                        *else_bb
                    };
                }
                Terminator::Switch {
                    discr,
                    arms,
                    otherwise,
                } => {
                    let d = self.operand(discr, &locals)?.as_i64()?;
                    bb = arms
                        .iter()
                        .find(|(v, _)| *v == d)
                        .map(|(_, t)| *t)
                        .unwrap_or(*otherwise);
                }
                Terminator::Unreachable => {
                    return Err(InterpError::Msg("reached unreachable".into()));
                }
            }
        }
    }

    fn exec_stmt(&mut self, stmt: &Statement, locals: &mut [Value]) -> Result<(), InterpError> {
        match stmt {
            Statement::Assign { dest, rvalue } => {
                let v = self.eval_rvalue(rvalue, locals)?;
                let i = *dest as usize;
                if i >= locals.len() {
                    return Err(InterpError::Msg(format!("bad local {dest}")));
                }
                locals[i] = v;
                Ok(())
            }
            Statement::Store { .. } => Err(InterpError::Unsupported("store/pointers")),
            Statement::StorageLive { .. } | Statement::StorageDead { .. } => Ok(()),
        }
    }

    fn eval_rvalue(&mut self, rv: &Rvalue, locals: &[Value]) -> Result<Value, InterpError> {
        match rv {
            Rvalue::Use(op) => self.operand(op, locals),
            Rvalue::Literal(c) => Ok(const_to_value(c)),
            Rvalue::Binary { op, left, right } => {
                let l = self.operand(left, locals)?;
                let r = self.operand(right, locals)?;
                eval_bin(*op, l, r)
            }
            Rvalue::Unary { op, operand } => {
                let v = self.operand(operand, locals)?;
                eval_un(*op, v)
            }
            Rvalue::Call { func, args, .. } => {
                let argv: Result<Vec<_>, _> =
                    args.iter().map(|a| self.operand(a, locals)).collect();
                let argv = argv?;
                self.call_named(func, argv)
            }
            Rvalue::Aggregate { fields, .. } => {
                let mut out = Vec::with_capacity(fields.len());
                for f in fields {
                    out.push(self.operand(f, locals)?);
                }
                Ok(Value::Struct(out))
            }
            Rvalue::Field {
                base,
                field_index,
                ..
            } => match self.operand(base, locals)? {
                Value::Struct(fields) => fields
                    .get(*field_index as usize)
                    .cloned()
                    .ok_or_else(|| InterpError::Msg("bad field index".into())),
                _ => Err(InterpError::Unsupported("field on non-struct")),
            },
            Rvalue::Cast { operand, to, .. } => {
                let v = self.operand(operand, locals)?;
                cast_value(v, to)
            }
            Rvalue::Index { .. } => Err(InterpError::Unsupported("indexing")),
            Rvalue::HeapAlloc { .. } => Err(InterpError::Unsupported("heap alloc")),
            Rvalue::Load { .. } | Rvalue::AddrOf { .. } => {
                Err(InterpError::Unsupported("pointers"))
            }
        }
    }

    fn call_named(&mut self, name: &str, args: Vec<Value>) -> Result<Value, InterpError> {
        if let Some(ret) = try_builtin(name, &args)? {
            return Ok(ret);
        }
        // Try bare name and common mangling.
        let candidates = [name.to_string(), strip_fn_prefix(name)];
        for cand in &candidates {
            if let Some(f) = self.fns.get(cand).copied() {
                return self.call_fn(f, args);
            }
        }
        // Module-qualified std wrappers often keep source names after lower.
        if let Some(f) = self.module.functions.iter().find(|f| f.name == name) {
            return self.call_fn(f, args);
        }
        Err(InterpError::Msg(format!(
            "unknown function `{name}` in script mode"
        )))
    }

    fn operand(&self, op: &Operand, locals: &[Value]) -> Result<Value, InterpError> {
        match op {
            Operand::Local(id) => locals
                .get(*id as usize)
                .cloned()
                .ok_or_else(|| InterpError::Msg(format!("bad local {id}"))),
            Operand::Constant(c) => Ok(const_to_value(c)),
        }
    }
}

fn strip_fn_prefix(name: &str) -> String {
    name.strip_prefix("buraaq_fn_")
        .unwrap_or(name)
        .to_string()
}

fn const_to_value(c: &Constant) -> Value {
    match c {
        Constant::Void => Value::Void,
        Constant::Bool(b) => Value::Bool(*b),
        Constant::I32(n) => Value::I64(*n as i64),
        Constant::I64(n) => Value::I64(*n),
        Constant::F32(f) => Value::F64(*f as f64),
        Constant::F64(f) => Value::F64(*f),
        Constant::Str(s) => Value::Str(s.clone()),
        Constant::NullPtr(_) => Value::I64(0),
        Constant::FnAddr(s) => Value::Str(s.clone()),
    }
}

fn cast_value(v: Value, to: &MirTy) -> Result<Value, InterpError> {
    match to {
        MirTy::Bool => Ok(Value::Bool(v.as_bool()?)),
        MirTy::I8 | MirTy::I32 | MirTy::I64 => Ok(Value::I64(v.as_i64()?)),
        MirTy::F32 | MirTy::F64 => Ok(Value::F64(v.as_f64()?)),
        MirTy::Text => Ok(Value::Str(v.as_str()?.to_string())),
        MirTy::Void => Ok(Value::Void),
        _ => Ok(v),
    }
}

fn eval_bin(op: BinOp, left: Value, right: Value) -> Result<Value, InterpError> {
    if matches!(
        op,
        BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
    ) {
        // Prefer float if either side is float.
        if matches!(left, Value::F64(_)) || matches!(right, Value::F64(_)) {
            let l = left.as_f64()?;
            let r = right.as_f64()?;
            let b = match op {
                BinOp::Eq => l == r,
                BinOp::NotEq => l != r,
                BinOp::Lt => l < r,
                BinOp::Le => l <= r,
                BinOp::Gt => l > r,
                BinOp::Ge => l >= r,
                _ => unreachable!(),
            };
            return Ok(Value::Bool(b));
        }
        if matches!(left, Value::Str(_)) || matches!(right, Value::Str(_)) {
            let l = left.as_str()?;
            let r = right.as_str()?;
            let b = match op {
                BinOp::Eq => l == r,
                BinOp::NotEq => l != r,
                BinOp::Lt => l < r,
                BinOp::Le => l <= r,
                BinOp::Gt => l > r,
                BinOp::Ge => l >= r,
                _ => unreachable!(),
            };
            return Ok(Value::Bool(b));
        }
        let l = left.as_i64()?;
        let r = right.as_i64()?;
        let b = match op {
            BinOp::Eq => l == r,
            BinOp::NotEq => l != r,
            BinOp::Lt => l < r,
            BinOp::Le => l <= r,
            BinOp::Gt => l > r,
            BinOp::Ge => l >= r,
            _ => unreachable!(),
        };
        return Ok(Value::Bool(b));
    }
    if matches!(op, BinOp::And | BinOp::Or) {
        let l = left.as_bool()?;
        let r = right.as_bool()?;
        return Ok(Value::Bool(match op {
            BinOp::And => l && r,
            BinOp::Or => l || r,
            _ => unreachable!(),
        }));
    }
    if matches!(left, Value::F64(_)) || matches!(right, Value::F64(_)) {
        let l = left.as_f64()?;
        let r = right.as_f64()?;
        let v = match op {
            BinOp::Add => l + r,
            BinOp::Sub => l - r,
            BinOp::Mul => l * r,
            BinOp::Div => l / r,
            BinOp::Mod => l % r,
            _ => return Err(InterpError::Unsupported("float binop")),
        };
        return Ok(Value::F64(v));
    }
    if matches!(left, Value::Str(_)) || matches!(right, Value::Str(_)) {
        if op == BinOp::Add {
            return Ok(Value::Str(format!(
                "{}{}",
                left.as_str().unwrap_or(""),
                right.as_str().unwrap_or("")
            )));
        }
    }
    let l = left.as_i64()?;
    let r = right.as_i64()?;
    let v = match op {
        BinOp::Add => l + r,
        BinOp::Sub => l - r,
        BinOp::Mul => l * r,
        BinOp::Div => {
            if r == 0 {
                return Err(InterpError::Msg("division by zero".into()));
            }
            l / r
        }
        BinOp::Mod => {
            if r == 0 {
                return Err(InterpError::Msg("modulo by zero".into()));
            }
            l % r
        }
        _ => return Err(InterpError::Unsupported("binop")),
    };
    Ok(Value::I64(v))
}

fn eval_un(op: UnOp, v: Value) -> Result<Value, InterpError> {
    match op {
        UnOp::Neg => match v {
            Value::F64(f) => Ok(Value::F64(-f)),
            _ => Ok(Value::I64(-v.as_i64()?)),
        },
        UnOp::Not => Ok(Value::Bool(!v.as_bool()?)),
    }
}

fn try_builtin(name: &str, args: &[Value]) -> Result<Option<Value>, InterpError> {
    let mut out = io::stdout();
    match name {
        "buraaq_print_str" | "print" => {
            write!(out, "{}", args.first().map(|a| a.as_str()).transpose()?.unwrap_or(""))
                .ok();
            out.flush().ok();
            Ok(Some(Value::Void))
        }
        "buraaq_print_str_ln" | "println" => {
            writeln!(
                out,
                "{}",
                args.first().map(|a| a.as_str()).transpose()?.unwrap_or("")
            )
            .ok();
            Ok(Some(Value::Void))
        }
        "buraaq_print_i32" | "buraaq_print_i64" => {
            write!(out, "{}", args.first().map(|a| a.as_i64()).transpose()?.unwrap_or(0)).ok();
            out.flush().ok();
            Ok(Some(Value::Void))
        }
        "buraaq_print_i32_ln" | "buraaq_print_i64_ln" | "print_int" => {
            writeln!(
                out,
                "{}",
                args.first().map(|a| a.as_i64()).transpose()?.unwrap_or(0)
            )
            .ok();
            Ok(Some(Value::Void))
        }
        "buraaq_print_f64" => {
            write!(out, "{}", args.first().map(|a| a.as_f64()).transpose()?.unwrap_or(0.0)).ok();
            out.flush().ok();
            Ok(Some(Value::Void))
        }
        "buraaq_print_f64_ln" | "print_float" => {
            writeln!(
                out,
                "{}",
                args.first().map(|a| a.as_f64()).transpose()?.unwrap_or(0.0)
            )
            .ok();
            Ok(Some(Value::Void))
        }
        "buraaq_print_bool" => {
            write!(
                out,
                "{}",
                if args.first().map(|a| a.as_bool()).transpose()?.unwrap_or(false) {
                    "true"
                } else {
                    "false"
                }
            )
            .ok();
            out.flush().ok();
            Ok(Some(Value::Void))
        }
        "buraaq_print_bool_ln" | "print_bool" => {
            writeln!(
                out,
                "{}",
                if args.first().map(|a| a.as_bool()).transpose()?.unwrap_or(false) {
                    "true"
                } else {
                    "false"
                }
            )
            .ok();
            Ok(Some(Value::Void))
        }
        "buraaq_text_concat" => {
            let a = args.first().map(|v| v.as_str()).transpose()?.unwrap_or("");
            let b = args.get(1).map(|v| v.as_str()).transpose()?.unwrap_or("");
            Ok(Some(Value::Str(format!("{a}{b}"))))
        }
        "buraaq_text_len" => {
            let a = args.first().map(|v| v.as_str()).transpose()?.unwrap_or("");
            Ok(Some(Value::I64(a.len() as i64)))
        }
        "buraaq_i32_to_text" | "buraaq_i64_to_text" => {
            Ok(Some(Value::Str(
                args.first().map(|a| a.as_i64()).transpose()?.unwrap_or(0).to_string(),
            )))
        }
        "buraaq_f64_to_text" => Ok(Some(Value::Str(
            args.first()
                .map(|a| a.as_f64())
                .transpose()?
                .unwrap_or(0.0)
                .to_string(),
        ))),
        "buraaq_bool_to_text" => Ok(Some(Value::Str(
            if args.first().map(|a| a.as_bool()).transpose()?.unwrap_or(false) {
                "true".into()
            } else {
                "false".into()
            },
        ))),
        "buraaq_rt_set_args" | "buraaq_runtime_init" | "buraaq_runtime_shutdown" => {
            Ok(Some(Value::Void))
        }
        "buraaq_time_sleep_ms" | "buraaq_led_wait_ms" => {
            let ms = args.first().map(|a| a.as_i64()).transpose()?.unwrap_or(0);
            if ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(ms as u64));
            }
            Ok(Some(Value::Void))
        }
        "buraaq_led_on" => {
            println!("LED on");
            Ok(Some(Value::Void))
        }
        "buraaq_led_off" => {
            println!("LED off");
            Ok(Some(Value::Void))
        }
        "buraaq_led_toggle" => {
            println!("LED toggle");
            Ok(Some(Value::Void))
        }
        _ => Ok(None),
    }
}
