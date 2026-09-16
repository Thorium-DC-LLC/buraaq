; Buraaq LLVM IR — module `text`
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

%struct_String = type { i8* }

define %struct_String @buraaq_fn_from_literal(i8* %p0) {
  %l0 = alloca i8*, align 8
  %l1 = alloca %struct_String, align 8
  store i8* %p0, i8** %l0, align 8
bb0:
  %t1 = alloca %struct_String, align 8
  %t2 = load i8*, i8** %l0, align 8
  %t3 = getelementptr inbounds %struct_String, %struct_String* %t1, i32 0, i32 0
  store i8* %t2, i8** %t3, align 8
  %t4 = load %struct_String, %struct_String* %t1, align 8
  store %struct_String %t4, %struct_String* %l1, align 8
  %t5 = load %struct_String, %struct_String* %l1, align 8
  ret %struct_String %t5
}

define i32 @buraaq_fn_len(i8* %p0) {
  %l0 = alloca i8*, align 8
  store i8* %p0, i8** %l0, align 8
bb0:
  ret i32 0
}

define i8* @buraaq_fn_concat(i8* %p0, i8* %p1) {
  %l0 = alloca i8*, align 8
  %l1 = alloca i8*, align 8
  store i8* %p0, i8** %l0, align 8
  store i8* %p1, i8** %l1, align 8
bb0:
  ret i8* 0
}

