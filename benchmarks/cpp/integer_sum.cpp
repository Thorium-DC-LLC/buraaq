#include "../suite/bench_common.h"

int main() {
    volatile int nv = 100000000;
    int n = nv;
    int sum = 0;
    double t0 = bench_now_sec();
    for (int i = 1; i <= n; ++i) {
        sum = sum * 3 + i;
    }
    double elapsed = bench_now_sec() - t0;
    bench_sink_i32(&sum);
    BENCH_PRINT("integer_sum", elapsed, n);
    return 0;
}
