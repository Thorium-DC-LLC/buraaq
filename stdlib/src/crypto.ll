; Buraaq LLVM IR — module `crypto`
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


define i8* @buraaq_fn_sha256(i8* %p0) {
  %l0 = alloca i8*, align 8
  store i8* %p0, i8** %l0, align 8
bb0:
  ret i8* 0
}

define i8* @buraaq_fn_sha256_bytes(i8* %p0) {
  %l0 = alloca i8*, align 8
  %l1 = alloca i8*, align 8
  %l2 = alloca i32, align 8
  store i8* %p0, i8** %l0, align 8
bb0:
  %t1 = load i8*, i8** %l0, align 8
  %t2 = call i8* @buraaq_i32_to_text(i32 %t1)
  store i8* %t2, i8** %l1, align 8
  %t3 = load i8*, i8** %l1, align 8
  %t4 = call i32 @buraaq_fn_sha256(i32 %t3)
  store i32 %t4, i32* %l2, align 8
  %t5 = load i32, i32* %l2, align 8
  ret i8* %t5
}

