; Buraaq LLVM IR — module `time`
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

%struct_Duration = type { i32 }

define i32 @buraaq_fn_now_ms() {
bb0:
  ret i32 0
}

define void @buraaq_fn_sleep(%struct_Duration %p0) {
  %l0 = alloca %struct_Duration, align 8
  store %struct_Duration %p0, %struct_Duration* %l0, align 8
bb0:
  ret void
}

define %struct_Duration @buraaq_fn_ms(i32 %p0) {
  %l0 = alloca i32, align 8
  %l1 = alloca %struct_Duration, align 8
  store i32 %p0, i32* %l0, align 8
bb0:
  %t1 = alloca %struct_Duration, align 8
  %t2 = load i32, i32* %l0, align 8
  %t3 = getelementptr inbounds %struct_Duration, %struct_Duration* %t1, i32 0, i32 0
  store i32 %t2, i32* %t3, align 8
  %t4 = load %struct_Duration, %struct_Duration* %t1, align 8
  store %struct_Duration %t4, %struct_Duration* %l1, align 8
  %t5 = load %struct_Duration, %struct_Duration* %l1, align 8
  ret %struct_Duration %t5
}

define %struct_Duration @buraaq_fn_sec(i32 %p0) {
  %l0 = alloca i32, align 8
  %l1 = alloca i32, align 8
  %l2 = alloca %struct_Duration, align 8
  store i32 %p0, i32* %l0, align 8
bb0:
  %t6 = load i32, i32* %l0, align 8
  %t7 = mul i32 %t6, 1000
  store i32 %t7, i32* %l1, align 8
  %t8 = alloca %struct_Duration, align 8
  %t9 = load i32, i32* %l1, align 8
  %t10 = getelementptr inbounds %struct_Duration, %struct_Duration* %t8, i32 0, i32 0
  store i32 %t9, i32* %t10, align 8
  %t11 = load %struct_Duration, %struct_Duration* %t8, align 8
  store %struct_Duration %t11, %struct_Duration* %l2, align 8
  %t12 = load %struct_Duration, %struct_Duration* %l2, align 8
  ret %struct_Duration %t12
}

