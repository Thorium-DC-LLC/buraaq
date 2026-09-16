#ifndef BURAAQ_RUNTIME_H
#define BURAAQ_RUNTIME_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* --- lifecycle --- */
void buraaq_runtime_init(int worker_threads); /* 0 = auto (num CPUs) */
void buraaq_runtime_shutdown(void);
int buraaq_runtime_worker_count(void);

/* --- OS threads (always 1:1 with platform thread) --- */
typedef struct buraaq_os_thread buraaq_os_thread_t;
typedef void (*buraaq_fn_void_ptr)(void *arg);

buraaq_os_thread_t *buraaq_os_thread_spawn(buraaq_fn_void_ptr fn, void *arg);
int64_t buraaq_os_thread_join(buraaq_os_thread_t *t); /* thread return payload as i64 */

/* --- lightweight tasks (work-stealing pool) --- */
typedef struct buraaq_task buraaq_task_t;

buraaq_task_t *buraaq_task_submit(buraaq_fn_void_ptr fn, void *arg);
int buraaq_task_done(buraaq_task_t *task); /* 1 if finished */
int64_t buraaq_task_join(buraaq_task_t *task);
void buraaq_task_yield(void); /* cooperative yield to executor */

/* --- task groups (structured join) --- */
typedef struct buraaq_task_group buraaq_task_group_t;

buraaq_task_group_t *buraaq_task_group_new(void);
void buraaq_task_group_add(buraaq_task_group_t *g, buraaq_task_t *t);
void buraaq_task_group_wait(buraaq_task_group_t *g);
void buraaq_task_group_free(buraaq_task_group_t *g);

/* --- channels (bounded, int64 payload v1) --- */
typedef struct buraaq_channel buraaq_channel_t;

buraaq_channel_t *buraaq_channel_bounded(int capacity);
int buraaq_channel_send(buraaq_channel_t *ch, int64_t value); /* 0 ok, -1 closed */
int buraaq_channel_recv(buraaq_channel_t *ch, int64_t *out);
void buraaq_channel_close(buraaq_channel_t *ch);
void buraaq_channel_free(buraaq_channel_t *ch);

/* --- atomics --- */
typedef struct buraaq_atomic_int buraaq_atomic_int_t;

buraaq_atomic_int_t *buraaq_atomic_int_new(int64_t v);
void buraaq_atomic_int_free(buraaq_atomic_int_t *a);
int64_t buraaq_atomic_int_load(buraaq_atomic_int_t *a);
void buraaq_atomic_int_store(buraaq_atomic_int_t *a, int64_t v);
int64_t buraaq_atomic_int_fetch_add(buraaq_atomic_int_t *a, int64_t delta);

/* --- cancellation --- */
typedef struct buraaq_cancel_token buraaq_cancel_token_t;

buraaq_cancel_token_t *buraaq_cancel_token_new(void);
int buraaq_cancel_token_is_cancelled(buraaq_cancel_token_t *t);
void buraaq_cancel_token_cancel(buraaq_cancel_token_t *t);
void buraaq_cancel_token_free(buraaq_cancel_token_t *t);

/* --- timeouts (ms) --- */
int buraaq_runtime_sleep_ms(int64_t ms);
int buraaq_runtime_wait_timeout(buraaq_cancel_token_t *token, int64_t ms); /* 0 ok, -1 timeout, -2 cancelled */

/* --- async I/O stubs (v1 — reactor upgrade in v0.8) --- */
typedef struct buraaq_io_request buraaq_io_request_t;
typedef void (*buraaq_io_callback)(void *user, int status, const char *data, size_t len);

buraaq_io_request_t *buraaq_fetch_async(const char *url, buraaq_io_callback cb, void *user);
void buraaq_fetch_blocking(const char *url, char **out_body, size_t *out_len);

#ifdef __cplusplus
}
#endif

#endif
