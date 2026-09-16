/* Task creation benchmark — Buraaq runtime */
#include "../../stdlib/runtime/buraaq_runtime.h"
#include "bench_common.h"

#define ITERS 50000

static void noop(void *arg) { (void)arg; }

int main(void) {
    buraaq_runtime_init(0);
    double t0 = bench_now_sec();
    for (int i = 0; i < ITERS; i++) {
        buraaq_task_t *t = buraaq_task_submit(noop, NULL);
        buraaq_task_join(t);
    }
    double t1 = bench_now_sec();
    bench_print("task_spawn_buraaq", ITERS, t1 - t0);
    buraaq_runtime_shutdown();
    return 0;
}
