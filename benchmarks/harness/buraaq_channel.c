#include "../../stdlib/runtime/buraaq_runtime.h"
#include "bench_common.h"

#define ITERS 200000

static void noop(void *arg) { (void)arg; }

int main(void) {
    buraaq_runtime_init(4);
    buraaq_channel_t *ch = buraaq_channel_bounded(256);
    double t0 = bench_now_sec();
    for (int i = 0; i < ITERS; i++) {
        buraaq_channel_send(ch, (int64_t)i);
        int64_t v;
        buraaq_channel_recv(ch, &v);
    }
    double t1 = bench_now_sec();
    bench_print("channel_ping_buraaq", ITERS, t1 - t0);
    buraaq_channel_free(ch);
    buraaq_runtime_shutdown();
    return 0;
}
