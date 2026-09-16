#include "../../stdlib/runtime/buraaq_std.h"
#include "../../stdlib/runtime/buraaq_runtime.h"
#include "bench_common.h"

#define ITERS 500000

int main(void) {
    void *m = buraaq_mutex_new();
    buraaq_atomic_int_t *counter = buraaq_atomic_int_new(0);
    double t0 = bench_now_sec();
    for (int i = 0; i < ITERS; i++) {
        buraaq_mutex_lock(m);
        buraaq_atomic_int_store(counter, buraaq_atomic_int_load(counter) + 1);
        buraaq_mutex_unlock(m);
    }
    double t1 = bench_now_sec();
    bench_print("mutex_contention_buraaq", ITERS, t1 - t0);
    buraaq_mutex_free(m);
    buraaq_atomic_int_free(counter);
    return 0;
}
