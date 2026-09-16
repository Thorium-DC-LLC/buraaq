; Buraaq LLVM IR — module `json`
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

%struct_Value = type { i8* }

define %struct_Value @buraaq_fn_parse(i8* %p0) {
  %l0 = alloca i8*, align 8
  %l1 = alloca %struct_Value, align 8
  store i8* %p0, i8** %l0, align 8
bb0:
  %t1 = alloca %struct_Value, align 8
  %t2 = load i8*, i8** %l0, align 8
  %t3 = getelementptr inbounds %struct_Value, %struct_Value* %t1, i32 0, i32 0
  store i8* %t2, i8** %t3, align 8
  %t4 = load %struct_Value, %struct_Value* %t1, align 8
  store %struct_Value %t4, %struct_Value* %l1, align 8
  %t5 = load %struct_Value, %struct_Value* %l1, align 8
  ret %struct_Value %t5
}

define i8* @buraaq_fn_stringify(%struct_Value %p0) {
  %l0 = alloca %struct_Value, align 8
  %l1 = alloca i8*, align 8
  store %struct_Value %p0, %struct_Value* %l0, align 8
bb0:
  %t6 = load %struct_Value, %struct_Value* %l0, align 8
  %t7 = alloca %struct_Value, align 8
  store %struct_Value %t6, %struct_Value* %t7, align 8
  %t8 = getelementptr inbounds %struct_Value, %struct_Value* %t7, i32 0, i32 0
  %t9 = load i8*, i8** %t8, align 8
  store i8* %t9, i8** %l1, align 8
  %t10 = load i8*, i8** %l1, align 8
  ret i8* %t10
}

