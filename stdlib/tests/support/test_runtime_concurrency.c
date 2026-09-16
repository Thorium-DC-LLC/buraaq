#include "buraaq_runtime.h"
#include <assert.h>

static int g_ran = 0;

static void inc(void *arg) {
    int *p = (int *)arg;
    (*p)++;
    g_ran++;
}

int main(void) {
    buraaq_runtime_init(2);
    assert(buraaq_runtime_worker_count() >= 1);

    int counter = 0;
    buraaq_task_t *t = buraaq_task_submit(inc, &counter);
    buraaq_task_join(t);
    assert(counter == 1);
    assert(g_ran == 1);

    buraaq_channel_t *ch = buraaq_channel_bounded(4);
    assert(buraaq_channel_send(ch, 42) == 0);
    int64_t v = 0;
    assert(buraaq_channel_recv(ch, &v) == 0);
    assert(v == 42);
    buraaq_channel_free(ch);

    buraaq_cancel_token_t *tok = buraaq_cancel_token_new();
    assert(!buraaq_cancel_token_is_cancelled(tok));
    buraaq_cancel_token_cancel(tok);
    assert(buraaq_cancel_token_is_cancelled(tok));
    buraaq_cancel_token_free(tok);

    buraaq_runtime_shutdown();
    return 0;
}
