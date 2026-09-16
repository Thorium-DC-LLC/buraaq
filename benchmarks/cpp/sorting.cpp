#include "../suite/bench_common.h"
#include <algorithm>
#include <cstdint>
#include <vector>

int main() {
    const int n = 500000;
    std::vector<int32_t> v(n);
    uint64_t seed = 1;
    for (int i = 0; i < n; ++i) {
        seed = seed * 6364136223846793005ULL + 1;
        v[i] = (int32_t)(seed >> 33);
    }
    double t0 = bench_now_sec();
    std::sort(v.begin(), v.end());
    double elapsed = bench_now_sec() - t0;
    BENCH_PRINT("sorting", elapsed, n);
    return v[0] > v[n - 1] ? 1 : 0;
}
