#include "../suite/bench_common.h"
#include <string>
#include <vector>

int main() {
    const int lines = 200000;
    std::vector<std::string> buf;
    buf.reserve(lines);
    double t0 = bench_now_sec();
    size_t total = 0;
    for (int i = 0; i < lines; ++i) {
        std::string s = "line-" + std::to_string(i) + "-payload";
        total += s.size();
        if (i % 3 == 0) buf.push_back(std::move(s));
    }
    double elapsed = bench_now_sec() - t0;
    BENCH_PRINT("string_processing", elapsed, lines);
    return total > 0 ? 0 : 1;
}
