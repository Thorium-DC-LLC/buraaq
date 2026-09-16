#include "../suite/bench_common.h"

int main() {
    volatile int nv = 10000;
    int n = nv;
    int sum = 0;
    double t0 = bench_now_sec();
    for (int i = 0; i < n; ++i) {
        for (int j = 0; j < n; ++j) {
            sum = sum * 3 + 1;
        }
    }
    double elapsed = bench_now_sec() - t0;
    bench_sink_i32(&sum);
    BENCH_PRINT("nested_loop", elapsed, (double)n * (double)n);
    return 0;
}
