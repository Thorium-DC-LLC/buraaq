#ifndef BENCH_COMMON_H
#define BENCH_COMMON_H

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#ifdef _WIN32
#include <windows.h>
static inline double bench_now_sec(void) {
    LARGE_INTEGER freq, counter;
    QueryPerformanceFrequency(&freq);
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

static inline void bench_print(const char *name, int64_t iters, double secs) {
    double per_op_ns = (secs * 1e9) / (double)iters;
    printf("BENCH|%s|iters=%lld|total_sec=%.6f|per_op_ns=%.2f\n",
           name, (long long)iters, secs, per_op_ns);
}

#endif
