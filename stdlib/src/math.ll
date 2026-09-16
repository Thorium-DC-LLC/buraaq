; Buraaq LLVM IR — module `math`
target triple = "x86_64-pc-windows-msvc"
declare void @buraaq_print_i32(i32)
declare void @buraaq_print_i32_ln(i32)
declare void @buraaq_print_i64(i64)
declare void @buraaq_print_i64_ln(i64)
declare void @buraaq_print_f64(double)
declare void @buraaq_print_f64_ln(double)
declare void @buraaq_print_bool(i1)
declare void @buraaq_print_bool_ln(i1)
declare void @buraaq_print_str(i8*)
declare void @buraaq_print_str_ln(i8*)
declare i8* @buraaq_i32_to_text(i32)
declare i8* @buraaq_i64_to_text(i64)
declare i8* @buraaq_f64_to_text(double)
declare i8* @buraaq_bool_to_text(i1)
declare i8* @buraaq_alloc(i64)
declare void @buraaq_free(i8*)


define double @buraaq_fn_sqrt(double %p0) {
  %l0 = alloca double, align 8
  store double %p0, double* %l0, align 8
bb0:
  ret double 0
}

define double @buraaq_fn_abs(double %p0) {
  %l0 = alloca double, align 8
  store double %p0, double* %l0, align 8
bb0:
  ret double 0
}

define i32 @buraaq_fn_abs_int(i32 %p0) {
  %l0 = alloca i32, align 8
  store i32 %p0, i32* %l0, align 8
bb0:
  ret i32 0
}

define double @buraaq_fn_min(double %p0, double %p1) {
  %l0 = alloca double, align 8
  %l1 = alloca double, align 8
  store double %p0, double* %l0, align 8
  store double %p1, double* %l1, align 8
bb0:
  ret double 0
}

define double @buraaq_fn_max(double %p0, double %p1) {
  %l0 = alloca double, align 8
  %l1 = alloca double, align 8
  store double %p0, double* %l0, align 8
  store double %p1, double* %l1, align 8
bb0:
  ret double 0
}

