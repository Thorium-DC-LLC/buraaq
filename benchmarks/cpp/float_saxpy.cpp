#include "../suite/bench_common.h"

int main() {
    volatile int nv = 10000000;
    int n = nv;
    double y = 0.0;
    double x = 1.0;
    const double a = 1.0000001;
    double t0 = bench_now_sec();
    for (int i = 0; i < n; ++i) {
        y = a * x + y;
        x = x + 1.0;
    }
    double elapsed = bench_now_sec() - t0;
    bench_sink_f64(&y);
    BENCH_PRINT("float_saxpy", elapsed, n);
    return 0;
}
