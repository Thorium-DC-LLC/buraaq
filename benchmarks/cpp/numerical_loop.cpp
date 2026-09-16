#include "../suite/bench_common.h"

int main() {
    volatile int nv = 100000000;
    int n = nv;
    double sum = 0.0;
    double t0 = bench_now_sec();
    for (int i = 1; i <= n; ++i) {
        sum += 1.0 / (double)i;
    }
    double elapsed = bench_now_sec() - t0;
    bench_sink_f64(&sum);
    BENCH_PRINT("numerical_loop", elapsed, n);
    return sum > 0.0 ? 0 : 1;
}
