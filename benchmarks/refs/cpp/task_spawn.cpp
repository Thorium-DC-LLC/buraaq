#include <chrono>
#include <cstdio>
#include <thread>

static const int ITERS = 50000;

int main() {
    auto t0 = std::chrono::steady_clock::now();
    for (int i = 0; i < ITERS; i++) {
        std::thread([] {}).join();
    }
    auto t1 = std::chrono::steady_clock::now();
    double secs = std::chrono::duration<double>(t1 - t0).count();
    double per_op_ns = (secs * 1e9) / ITERS;
    std::printf("BENCH|task_spawn_cpp|iters=%d|total_sec=%.6f|per_op_ns=%.2f\n", ITERS, secs, per_op_ns);
    return 0;
}
