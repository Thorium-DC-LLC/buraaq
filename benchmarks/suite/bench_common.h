#ifndef BURAAQ_BENCH_COMMON_H
#define BURAAQ_BENCH_COMMON_H

#include <stdint.h>
#include <stdio.h>

#if defined(_WIN32)
#include <windows.h>
static inline double bench_now_sec(void) {
    static LARGE_INTEGER freq;
    LARGE_INTEGER counter;
    if (!freq.QuadPart) QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&counter);
    return (double)counter.QuadPart / (double)freq.QuadPart;
}
#else
#include <time.h>
static inline double bench_now_sec(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
}
#endif

#define BENCH_PRINT(name, secs, ops) \
    do { \
        double _s = (secs); \
        double _ops = (double)(ops); \
        printf("BENCH name=%s time_sec=%.6f ops=%.0f ops_per_sec=%.0f\n", \
               (name), _s, _ops, _s > 0.0 ? _ops / _s : 0.0); \
    } while (0)

/* Prevent LLVM/clang from constant-folding the whole benchmark away at -O2+. */
static inline void bench_sink_i32(volatile int32_t *p) { (void)*p; }
static inline void bench_sink_i64(volatile int64_t *p) { (void)*p; }
static inline void bench_sink_f64(volatile double *p) { (void)*p; }

#endif
