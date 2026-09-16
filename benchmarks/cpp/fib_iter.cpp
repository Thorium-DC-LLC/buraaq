#include "../suite/bench_common.h"

int main() {
    volatile int nv = 100000000;
    int n = nv;
    int a = 0;
    int b = 1;
    double t0 = bench_now_sec();
    for (int i = 0; i < n; ++i) {
        int c = a + b;
        a = b;
        b = c;
    }
    double elapsed = bench_now_sec() - t0;
    bench_sink_i32(&a);
    bench_sink_i32(&b);
    BENCH_PRINT("fib_iter", elapsed, n);
    return 0;
}
