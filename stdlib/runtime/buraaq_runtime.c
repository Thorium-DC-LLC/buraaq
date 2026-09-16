/* Buraaq concurrency runtime — small, native, zero-GC.
 * Work-stealing executor, OS threads, channels, atomics, cancellation.
 */
#include "buraaq_runtime.h"

#include <stdlib.h>
#include <string.h>
#include <stdio.h>

#ifdef _WIN32
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#else
#include <pthread.h>
#include <unistd.h>
#include <time.h>
#include <errno.h>
#include <sched.h>
#endif

#if defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
#include <stdatomic.h>
#define BQ_ATOMIC int64_t _Atomic
#define bq_atomic_load(p) atomic_load(p)
#define bq_atomic_store(p, v) atomic_store(p, v)
#define bq_atomic_fetch_add(p, d) atomic_fetch_add(p, d)
#else
#define BQ_ATOMIC volatile int64_t
#define bq_atomic_load(p) (*(p))
#define bq_atomic_store(p, v) (*(p) = (v))
#define bq_atomic_fetch_add(p, d) (__sync_fetch_and_add(p, d))
#endif

/* ---------- utilities ---------- */

static int bq_num_cpus(void) {
#ifdef _WIN32
    SYSTEM_INFO si;
    GetSystemInfo(&si);
    return (int)si.dwNumberOfProcessors;
#else
    long n = sysconf(_SC_NPROCESSORS_ONLN);
    return n > 0 ? (int)n : 1;
#endif
}

static void *bq_malloc(size_t n) {
    void *p = malloc(n);
    return p;
}

/* ---------- OS threads ---------- */

struct buraaq_os_thread {
#ifdef _WIN32
    HANDLE handle;
#else
    pthread_t handle;
#endif
    buraaq_fn_void_ptr fn;
    void *arg;
    int64_t result;
};

#ifdef _WIN32
static DWORD WINAPI bq_os_thread_entry(LPVOID arg) {
    buraaq_os_thread_t *t = (buraaq_os_thread_t *)arg;
    t->fn(t->arg);
    return 0;
}
#else
static void *bq_os_thread_entry(void *arg) {
    buraaq_os_thread_t *t = (buraaq_os_thread_t *)arg;
    t->fn(t->arg);
    return NULL;
}
#endif

buraaq_os_thread_t *buraaq_os_thread_spawn(buraaq_fn_void_ptr fn, void *arg) {
    buraaq_os_thread_t *t = (buraaq_os_thread_t *)bq_malloc(sizeof(*t));
    if (!t) return NULL;
    t->fn = fn;
    t->arg = arg;
    t->result = 0;
#ifdef _WIN32
    t->handle = CreateThread(NULL, 0, bq_os_thread_entry, t, 0, NULL);
    if (!t->handle) { free(t); return NULL; }
#else
    if (pthread_create(&t->handle, NULL, bq_os_thread_entry, t) != 0) {
        free(t);
        return NULL;
    }
#endif
    return t;
}

int64_t buraaq_os_thread_join(buraaq_os_thread_t *t) {
    if (!t) return 0;
#ifdef _WIN32
    WaitForSingleObject(t->handle, INFINITE);
    CloseHandle(t->handle);
#else
    pthread_join(t->handle, NULL);
#endif
    int64_t r = t->result;
    free(t);
    return r;
}

/* ---------- work-stealing task executor ---------- */

typedef struct {
    buraaq_fn_void_ptr fn;
    void *arg;
} bq_job_t;

typedef struct {
    bq_job_t *jobs;
    int head;
    int tail;
    int cap;
#ifdef _WIN32
    CRITICAL_SECTION mu;
#else
    pthread_mutex_t mu;
#endif
} bq_deque_t;

typedef struct {
    bq_deque_t *local;
    int id;
#ifdef _WIN32
    HANDLE thread;
    HANDLE wake;
#else
    pthread_t thread;
    pthread_cond_t wake;
    pthread_mutex_t wake_mu;
#endif
} bq_worker_t;

typedef struct {
    bq_worker_t *workers;
    int nworkers;
    BQ_ATOMIC shutdown;
    BQ_ATOMIC submitted;
    BQ_ATOMIC completed;
#ifdef _WIN32
    CRITICAL_SECTION global_mu;
#else
    pthread_mutex_t global_mu;
#endif
} bq_executor_t;

static bq_executor_t g_exec;
static int g_exec_ready = 0;

static void bq_deque_init(bq_deque_t *d, int cap) {
    d->jobs = (bq_job_t *)bq_malloc((size_t)cap * sizeof(bq_job_t));
    d->head = d->tail = 0;
    d->cap = cap;
#ifdef _WIN32
    InitializeCriticalSection(&d->mu);
#else
    pthread_mutex_init(&d->mu, NULL);
#endif
}

static void bq_deque_free(bq_deque_t *d) {
    free(d->jobs);
#ifdef _WIN32
    DeleteCriticalSection(&d->mu);
#else
    pthread_mutex_destroy(&d->mu);
#endif
}

static int bq_deque_push_bottom(bq_deque_t *d, bq_job_t job) {
#ifdef _WIN32
    EnterCriticalSection(&d->mu);
#else
    pthread_mutex_lock(&d->mu);
#endif
    int next = (d->tail + 1) % d->cap;
    if (next == d->head) {
#ifdef _WIN32
        LeaveCriticalSection(&d->mu);
#else
        pthread_mutex_unlock(&d->mu);
#endif
        return 0;
    }
    d->jobs[d->tail] = job;
    d->tail = next;
#ifdef _WIN32
    LeaveCriticalSection(&d->mu);
#else
    pthread_mutex_unlock(&d->mu);
#endif
    return 1;
}

static int bq_deque_pop_bottom(bq_deque_t *d, bq_job_t *out) {
#ifdef _WIN32
    EnterCriticalSection(&d->mu);
#else
    pthread_mutex_lock(&d->mu);
#endif
    if (d->head == d->tail) {
#ifdef _WIN32
        LeaveCriticalSection(&d->mu);
#else
        pthread_mutex_unlock(&d->mu);
#endif
        return 0;
    }
    d->tail = (d->tail - 1 + d->cap) % d->cap;
    *out = d->jobs[d->tail];
#ifdef _WIN32
    LeaveCriticalSection(&d->mu);
#else
    pthread_mutex_unlock(&d->mu);
#endif
    return 1;
}

static int bq_deque_steal_top(bq_deque_t *d, bq_job_t *out) {
#ifdef _WIN32
    EnterCriticalSection(&d->mu);
#else
    pthread_mutex_lock(&d->mu);
#endif
    if (d->head == d->tail) {
#ifdef _WIN32
        LeaveCriticalSection(&d->mu);
#else
        pthread_mutex_unlock(&d->mu);
#endif
        return 0;
    }
    *out = d->jobs[d->head];
    d->head = (d->head + 1) % d->cap;
#ifdef _WIN32
    LeaveCriticalSection(&d->mu);
#else
    pthread_mutex_unlock(&d->mu);
#endif
    return 1;
}

struct buraaq_task {
    BQ_ATOMIC done;
    int64_t result;
    bq_job_t job;
    int owner_worker; /* -1 = external */
};

static void bq_worker_wake_all(void) {
    for (int i = 0; i < g_exec.nworkers; i++) {
#ifdef _WIN32
        SetEvent(g_exec.workers[i].wake);
#else
        pthread_mutex_lock(&g_exec.workers[i].wake_mu);
        pthread_cond_signal(&g_exec.workers[i].wake);
        pthread_mutex_unlock(&g_exec.workers[i].wake_mu);
#endif
    }
}

static void bq_run_job(bq_job_t *job) {
    if (job->fn) job->fn(job->arg);
}

#ifdef _WIN32
static DWORD WINAPI bq_worker_main(LPVOID arg) {
#else
static void *bq_worker_main(void *arg) {
#endif
    bq_worker_t *w = (bq_worker_t *)arg;
    for (;;) {
        bq_job_t job;
        if (bq_deque_pop_bottom(w->local, &job)) {
            bq_run_job(&job);
            bq_atomic_fetch_add(&g_exec.completed, 1);
            continue;
        }
        int stole = 0;
        for (int i = 0; i < g_exec.nworkers; i++) {
            if (i == w->id) continue;
            if (bq_deque_steal_top(g_exec.workers[i].local, &job)) {
                bq_run_job(&job);
                bq_atomic_fetch_add(&g_exec.completed, 1);
                stole = 1;
                break;
            }
        }
        if (stole) continue;
        if (bq_atomic_load(&g_exec.shutdown)) break;
#ifdef _WIN32
        WaitForSingleObject(w->wake, 10);
#else
        struct timespec ts;
        clock_gettime(CLOCK_REALTIME, &ts);
        ts.tv_nsec += 2 * 1000 * 1000;
        if (ts.tv_nsec >= 1000000000) { ts.tv_sec++; ts.tv_nsec -= 1000000000; }
        pthread_mutex_lock(&w->wake_mu);
        pthread_cond_timedwait(&w->wake, &w->wake_mu, &ts);
        pthread_mutex_unlock(&w->wake_mu);
#endif
    }
#ifdef _WIN32
    return 0;
#else
    return NULL;
#endif
}

void buraaq_runtime_init(int worker_threads) {
    if (g_exec_ready) return;
    if (worker_threads <= 0) worker_threads = bq_num_cpus();
    if (worker_threads < 1) worker_threads = 1;
    g_exec.nworkers = worker_threads;
    g_exec.workers = (bq_worker_t *)bq_malloc((size_t)worker_threads * sizeof(bq_worker_t));
    bq_atomic_store(&g_exec.shutdown, 0);
    bq_atomic_store(&g_exec.submitted, 0);
    bq_atomic_store(&g_exec.completed, 0);
#ifdef _WIN32
    InitializeCriticalSection(&g_exec.global_mu);
#else
    pthread_mutex_init(&g_exec.global_mu, NULL);
#endif
    for (int i = 0; i < worker_threads; i++) {
        g_exec.workers[i].id = i;
        g_exec.workers[i].local = (bq_deque_t *)bq_malloc(sizeof(bq_deque_t));
        bq_deque_init(g_exec.workers[i].local, 4096);
#ifdef _WIN32
        g_exec.workers[i].wake = CreateEvent(NULL, FALSE, FALSE, NULL);
        g_exec.workers[i].thread = CreateThread(NULL, 0, bq_worker_main, &g_exec.workers[i], 0, NULL);
#else
        pthread_mutex_init(&g_exec.workers[i].wake_mu, NULL);
        pthread_cond_init(&g_exec.workers[i].wake, NULL);
        pthread_create(&g_exec.workers[i].thread, NULL, bq_worker_main, &g_exec.workers[i]);
#endif
    }
    g_exec_ready = 1;
}

void buraaq_runtime_shutdown(void) {
    if (!g_exec_ready) return;
    bq_atomic_store(&g_exec.shutdown, 1);
    bq_worker_wake_all();
    for (int i = 0; i < g_exec.nworkers; i++) {
#ifdef _WIN32
        WaitForSingleObject(g_exec.workers[i].thread, INFINITE);
        CloseHandle(g_exec.workers[i].thread);
        CloseHandle(g_exec.workers[i].wake);
#else
        pthread_join(g_exec.workers[i].thread, NULL);
        pthread_cond_destroy(&g_exec.workers[i].wake);
        pthread_mutex_destroy(&g_exec.workers[i].wake_mu);
#endif
        bq_deque_free(g_exec.workers[i].local);
        free(g_exec.workers[i].local);
    }
    free(g_exec.workers);
#ifdef _WIN32
    DeleteCriticalSection(&g_exec.global_mu);
#else
    pthread_mutex_destroy(&g_exec.global_mu);
#endif
    g_exec_ready = 0;
}

int buraaq_runtime_worker_count(void) {
    return g_exec_ready ? g_exec.nworkers : 0;
}

typedef struct {
    buraaq_task_t *task;
} bq_task_trampoline_ctx;

static void bq_task_trampoline(void *arg) {
    bq_task_trampoline_ctx *ctx = (bq_task_trampoline_ctx *)arg;
    buraaq_task_t *t = ctx->task;
    bq_run_job(&t->job);
    bq_atomic_store(&t->done, 1);
    free(ctx);
}

buraaq_task_t *buraaq_task_submit(buraaq_fn_void_ptr fn, void *arg) {
    if (!g_exec_ready) buraaq_runtime_init(0);
    buraaq_task_t *t = (buraaq_task_t *)bq_malloc(sizeof(*t));
    if (!t) return NULL;
    bq_atomic_store(&t->done, 0);
    t->result = 0;
    t->job.fn = fn;
    t->job.arg = arg;
    t->owner_worker = -1;

    bq_task_trampoline_ctx *ctx = (bq_task_trampoline_ctx *)bq_malloc(sizeof(*ctx));
    ctx->task = t;
    bq_job_t wrapper = { bq_task_trampoline, ctx };
    int wid = (int)(bq_atomic_fetch_add(&g_exec.submitted, 1) % (int64_t)g_exec.nworkers);
    if (!bq_deque_push_bottom(g_exec.workers[wid].local, wrapper)) {
        /* queue full — run inline (backpressure) */
        bq_task_trampoline(ctx);
    } else {
        bq_worker_wake_all();
    }
    return t;
}

int buraaq_task_done(buraaq_task_t *task) {
    return task && bq_atomic_load(&task->done);
}

int64_t buraaq_task_join(buraaq_task_t *task) {
    if (!task) return 0;
    while (!bq_atomic_load(&task->done)) {
        buraaq_task_yield();
    }
    int64_t r = task->result;
    free(task);
    return r;
}

void buraaq_task_yield(void) {
#ifdef _WIN32
    Sleep(0);
#else
    sched_yield();
#endif
}

/* ---------- task groups ---------- */

struct buraaq_task_group {
    buraaq_task_t **tasks;
    int len;
    int cap;
};

buraaq_task_group_t *buraaq_task_group_new(void) {
    buraaq_task_group_t *g = (buraaq_task_group_t *)bq_malloc(sizeof(*g));
    g->cap = 16;
    g->len = 0;
    g->tasks = (buraaq_task_t **)bq_malloc((size_t)g->cap * sizeof(buraaq_task_t *));
    return g;
}

void buraaq_task_group_add(buraaq_task_group_t *g, buraaq_task_t *t) {
    if (!g || !t) return;
    if (g->len >= g->cap) {
        g->cap *= 2;
        g->tasks = (buraaq_task_t **)realloc(g->tasks, (size_t)g->cap * sizeof(buraaq_task_t *));
    }
    g->tasks[g->len++] = t;
}

void buraaq_task_group_wait(buraaq_task_group_t *g) {
    if (!g) return;
    for (int i = 0; i < g->len; i++) {
        buraaq_task_join(g->tasks[i]);
    }
    g->len = 0;
}

void buraaq_task_group_free(buraaq_task_group_t *g) {
    if (!g) return;
    free(g->tasks);
    free(g);
}

/* ---------- channels ---------- */

struct buraaq_channel {
    int64_t *buf;
    int cap;
    int head;
    int tail;
    int count;
    int closed;
#ifdef _WIN32
    CRITICAL_SECTION mu;
    CONDITION_VARIABLE not_empty;
    CONDITION_VARIABLE not_full;
#else
    pthread_mutex_t mu;
    pthread_cond_t not_empty;
    pthread_cond_t not_full;
#endif
};

buraaq_channel_t *buraaq_channel_bounded(int capacity) {
    if (capacity < 1) capacity = 1;
    buraaq_channel_t *ch = (buraaq_channel_t *)bq_malloc(sizeof(*ch));
    ch->buf = (int64_t *)bq_malloc((size_t)capacity * sizeof(int64_t));
    ch->cap = capacity;
    ch->head = ch->tail = ch->count = 0;
    ch->closed = 0;
#ifdef _WIN32
    InitializeCriticalSection(&ch->mu);
    InitializeConditionVariable(&ch->not_empty);
    InitializeConditionVariable(&ch->not_full);
#else
    pthread_mutex_init(&ch->mu, NULL);
    pthread_cond_init(&ch->not_empty, NULL);
    pthread_cond_init(&ch->not_full, NULL);
#endif
    return ch;
}

static void bq_ch_lock(buraaq_channel_t *ch) {
#ifdef _WIN32
    EnterCriticalSection(&ch->mu);
#else
    pthread_mutex_lock(&ch->mu);
#endif
}

static void bq_ch_unlock(buraaq_channel_t *ch) {
#ifdef _WIN32
    LeaveCriticalSection(&ch->mu);
#else
    pthread_mutex_unlock(&ch->mu);
#endif
}

int buraaq_channel_send(buraaq_channel_t *ch, int64_t value) {
    if (!ch) return -1;
    bq_ch_lock(ch);
    while (ch->count >= ch->cap && !ch->closed) {
#ifdef _WIN32
        SleepConditionVariableCS(&ch->not_full, &ch->mu, INFINITE);
#else
        pthread_cond_wait(&ch->not_full, &ch->mu);
#endif
    }
    if (ch->closed) { bq_ch_unlock(ch); return -1; }
    ch->buf[ch->tail] = value;
    ch->tail = (ch->tail + 1) % ch->cap;
    ch->count++;
#ifdef _WIN32
    WakeConditionVariable(&ch->not_empty);
#else
    pthread_cond_signal(&ch->not_empty);
#endif
    bq_ch_unlock(ch);
    return 0;
}

int buraaq_channel_recv(buraaq_channel_t *ch, int64_t *out) {
    if (!ch || !out) return -1;
    bq_ch_lock(ch);
    while (ch->count == 0 && !ch->closed) {
#ifdef _WIN32
        SleepConditionVariableCS(&ch->not_empty, &ch->mu, INFINITE);
#else
        pthread_cond_wait(&ch->not_empty, &ch->mu);
#endif
    }
    if (ch->count == 0 && ch->closed) { bq_ch_unlock(ch); return -1; }
    *out = ch->buf[ch->head];
    ch->head = (ch->head + 1) % ch->cap;
    ch->count--;
#ifdef _WIN32
    WakeConditionVariable(&ch->not_full);
#else
    pthread_cond_signal(&ch->not_full);
#endif
    bq_ch_unlock(ch);
    return 0;
}

void buraaq_channel_close(buraaq_channel_t *ch) {
    if (!ch) return;
    bq_ch_lock(ch);
    ch->closed = 1;
#ifdef _WIN32
    WakeAllConditionVariable(&ch->not_empty);
    WakeAllConditionVariable(&ch->not_full);
#else
    pthread_cond_broadcast(&ch->not_empty);
    pthread_cond_broadcast(&ch->not_full);
#endif
    bq_ch_unlock(ch);
}

void buraaq_channel_free(buraaq_channel_t *ch) {
    if (!ch) return;
    free(ch->buf);
#ifdef _WIN32
    DeleteCriticalSection(&ch->mu);
#else
    pthread_mutex_destroy(&ch->mu);
    pthread_cond_destroy(&ch->not_empty);
    pthread_cond_destroy(&ch->not_full);
#endif
    free(ch);
}

/* ---------- atomics ---------- */

struct buraaq_atomic_int {
    BQ_ATOMIC value;
};

buraaq_atomic_int_t *buraaq_atomic_int_new(int64_t v) {
    buraaq_atomic_int_t *a = (buraaq_atomic_int_t *)bq_malloc(sizeof(*a));
    bq_atomic_store(&a->value, v);
    return a;
}

void buraaq_atomic_int_free(buraaq_atomic_int_t *a) { free(a); }

int64_t buraaq_atomic_int_load(buraaq_atomic_int_t *a) {
    return a ? bq_atomic_load(&a->value) : 0;
}

void buraaq_atomic_int_store(buraaq_atomic_int_t *a, int64_t v) {
    if (a) bq_atomic_store(&a->value, v);
}

int64_t buraaq_atomic_int_fetch_add(buraaq_atomic_int_t *a, int64_t delta) {
    return a ? bq_atomic_fetch_add(&a->value, delta) : 0;
}

/* ---------- cancellation ---------- */

struct buraaq_cancel_token {
    BQ_ATOMIC cancelled;
};

buraaq_cancel_token_t *buraaq_cancel_token_new(void) {
    buraaq_cancel_token_t *t = (buraaq_cancel_token_t *)bq_malloc(sizeof(*t));
    bq_atomic_store(&t->cancelled, 0);
    return t;
}

int buraaq_cancel_token_is_cancelled(buraaq_cancel_token_t *t) {
    return t && bq_atomic_load(&t->cancelled);
}

void buraaq_cancel_token_cancel(buraaq_cancel_token_t *t) {
    if (t) bq_atomic_store(&t->cancelled, 1);
}

void buraaq_cancel_token_free(buraaq_cancel_token_t *t) { free(t); }

int buraaq_runtime_sleep_ms(int64_t ms) {
    if (ms <= 0) return 0;
#ifdef _WIN32
    Sleep((DWORD)ms);
#else
    struct timespec ts;
    ts.tv_sec = ms / 1000;
    ts.tv_nsec = (ms % 1000) * 1000000L;
    nanosleep(&ts, NULL);
#endif
    return 0;
}

int buraaq_runtime_wait_timeout(buraaq_cancel_token_t *token, int64_t ms) {
    int64_t elapsed = 0;
    const int64_t step = 5;
    while (elapsed < ms) {
        if (token && buraaq_cancel_token_is_cancelled(token)) return -2;
        buraaq_runtime_sleep_ms(step);
        elapsed += step;
    }
    if (token && buraaq_cancel_token_is_cancelled(token)) return -2;
    return -1;
}

/* ---------- async I/O (v1 stub) ---------- */

struct buraaq_io_request {
    char *url;
};

void buraaq_fetch_blocking(const char *url, char **out_body, size_t *out_len) {
    static const char stub[] = "{\"status\":200}";
    size_t n = sizeof(stub) - 1;
    char *p = (char *)bq_malloc(n + 1);
    memcpy(p, stub, n + 1);
    *out_body = p;
    *out_len = n;
    (void)url;
}

static char *bq_strdup(const char *s) {
    if (!s) return NULL;
    size_t n = strlen(s) + 1;
    char *p = (char *)bq_malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

buraaq_io_request_t *buraaq_fetch_async(const char *url, buraaq_io_callback cb, void *user) {
    buraaq_io_request_t *req = (buraaq_io_request_t *)bq_malloc(sizeof(*req));
    req->url = bq_strdup(url);
    if (cb) {
        char *body = NULL;
        size_t len = 0;
        buraaq_fetch_blocking(url, &body, &len);
        cb(user, 0, body, len);
        free(body);
    }
    return req;
}
