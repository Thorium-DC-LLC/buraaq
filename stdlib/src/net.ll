; Buraaq LLVM IR — module `net`
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

%struct_TcpListener = type { i32 }
%struct_TcpStream = type { i8*, i32 }
%struct_UdpSocket = type { i32 }

define %struct_TcpListener @buraaq_fn_listen(i32 %p0) {
  %l0 = alloca i32, align 8
  %l1 = alloca %struct_TcpListener, align 8
  store i32 %p0, i32* %l0, align 8
bb0:
  %t1 = alloca %struct_TcpListener, align 8
  %t2 = load i32, i32* %l0, align 8
  %t3 = getelementptr inbounds %struct_TcpListener, %struct_TcpListener* %t1, i32 0, i32 0
  store i32 %t2, i32* %t3, align 8
  %t4 = load %struct_TcpListener, %struct_TcpListener* %t1, align 8
  store %struct_TcpListener %t4, %struct_TcpListener* %l1, align 8
  %t5 = load %struct_TcpListener, %struct_TcpListener* %l1, align 8
  ret %struct_TcpListener %t5
}

define %struct_TcpStream @buraaq_fn_connect(i8* %p0, i32 %p1) {
  %l0 = alloca i8*, align 8
  %l1 = alloca i32, align 8
  %l2 = alloca %struct_TcpStream, align 8
  store i8* %p0, i8** %l0, align 8
  store i32 %p1, i32* %l1, align 8
bb0:
  %t6 = alloca %struct_TcpStream, align 8
  %t7 = load i8*, i8** %l0, align 8
  %t8 = getelementptr inbounds %struct_TcpStream, %struct_TcpStream* %t6, i32 0, i32 0
  store i8* %t7, i8** %t8, align 8
  %t9 = load i32, i32* %l1, align 8
  %t10 = getelementptr inbounds %struct_TcpStream, %struct_TcpStream* %t6, i32 0, i32 1
  store i32 %t9, i32* %t10, align 8
  %t11 = load %struct_TcpStream, %struct_TcpStream* %t6, align 8
  store %struct_TcpStream %t11, %struct_TcpStream* %l2, align 8
  %t12 = load %struct_TcpStream, %struct_TcpStream* %l2, align 8
  ret %struct_TcpStream %t12
}

define %struct_UdpSocket @buraaq_fn_udp_bind(i32 %p0) {
  %l0 = alloca i32, align 8
  %l1 = alloca %struct_UdpSocket, align 8
  store i32 %p0, i32* %l0, align 8
bb0:
  %t13 = alloca %struct_UdpSocket, align 8
  %t14 = load i32, i32* %l0, align 8
  %t15 = getelementptr inbounds %struct_UdpSocket, %struct_UdpSocket* %t13, i32 0, i32 0
  store i32 %t14, i32* %t15, align 8
  %t16 = load %struct_UdpSocket, %struct_UdpSocket* %t13, align 8
  store %struct_UdpSocket %t16, %struct_UdpSocket* %l1, align 8
  %t17 = load %struct_UdpSocket, %struct_UdpSocket* %l1, align 8
  ret %struct_UdpSocket %t17
}

