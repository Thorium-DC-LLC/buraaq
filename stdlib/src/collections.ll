; Buraaq LLVM IR — module `collections`
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


