#include "../suite/bench_common.h"
#include <cstdint>
#include <unordered_map>

int main() {
    const int n = 500000;
    std::unordered_map<int32_t, int32_t> m;
    uint64_t seed = 42;
    double t0 = bench_now_sec();
    for (int i = 0; i < n; ++i) {
        seed = seed * 1103515245ULL + 12345;
        int32_t k = (int32_t)(seed >> 16);
        m[k] = i;
    }
    long long sum = 0;
    for (const auto& kv : m) sum += kv.second;
    double elapsed = bench_now_sec() - t0;
    BENCH_PRINT("hash_map", elapsed, n);
    return sum > 0 ? 0 : 1;
}
