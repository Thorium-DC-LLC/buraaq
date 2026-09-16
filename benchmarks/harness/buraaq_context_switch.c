#include "../../stdlib/runtime/buraaq_runtime.h"
#include "bench_common.h"

#define ITERS 100000

typedef struct { buraaq_channel_t *ch; int id; } ping_ctx_t;

static void ping_pong(void *arg) {
    ping_ctx_t *ctx = (ping_ctx_t *)arg;
    for (int i = 0; i < ITERS; i++) {
        int64_t v;
        if (ctx->id == 0) {
            buraaq_channel_send(ctx->ch, 1);
            buraaq_channel_recv(ctx->ch, &v);
        } else {
            buraaq_channel_recv(ctx->ch, &v);
            buraaq_channel_send(ctx->ch, 2);
        }
    }
}

int main(void) {
    buraaq_runtime_init(2);
    buraaq_channel_t *ch = buraaq_channel_bounded(1);
    ping_ctx_t a = { ch, 0 };
    ping_ctx_t b = { ch, 1 };
    double t0 = bench_now_sec();
    buraaq_task_t *t0h = buraaq_task_submit(ping_pong, &a);
    buraaq_task_t *t1h = buraaq_task_submit(ping_pong, &b);
    buraaq_task_join(t0h);
    buraaq_task_join(t1h);
    double t1 = bench_now_sec();
    bench_print("context_switch_buraaq", ITERS * 2, t1 - t0);
    buraaq_channel_free(ch);
    buraaq_runtime_shutdown();
    return 0;
}
