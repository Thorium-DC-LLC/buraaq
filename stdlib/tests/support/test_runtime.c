#include "buraaq_std.h"

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#include <windows.h>
static DWORD WINAPI echo_once(void *arg) {
    (void)arg;
    int32_t id = buraaq_stream_hail();
    if (id < 0) return 1;
    char *msg = buraaq_stream_hear(id);
    if (msg) {
        buraaq_stream_say(id, msg);
        free(msg);
    }
    buraaq_stream_hangup(id);
    return 0;
}
#else
#include <pthread.h>
#include <unistd.h>
static void *echo_once_posix(void *arg) {
    (void)arg;
    int32_t id = buraaq_stream_hail();
    if (id >= 0) {
        char *msg = buraaq_stream_hear(id);
        if (msg) {
            buraaq_stream_say(id, msg);
            free(msg);
        }
        buraaq_stream_hangup(id);
    }
    return NULL;
}
#endif

int main(void) {
    assert(buraaq_file_exists("buraaq_stdlib_missing_file_xyz") == 0);

    char *hello = buraaq_text_concat("Hello", " Buraaq");
    assert(hello != NULL);
    assert(strcmp(hello, "Hello Buraaq") == 0);
    free(hello);

    assert(buraaq_text_len("abc") == 3);
    assert(buraaq_math_min_f64(2.0, 5.0) == 2.0);
    assert(buraaq_math_max_f64(2.0, 5.0) == 5.0);

    char *field = buraaq_json_parse_string_field("{\"name\":\"Ada\"}", "name");
    assert(field != NULL);
    assert(strcmp(field, "Ada") == 0);
    free(field);

    char *body = buraaq_http_get_body("https://example.com");
    assert(body != NULL);
    free(body);

    char *hash = buraaq_crypto_sha256_hex("test");
    assert(hash != NULL);
    assert(strlen(hash) == 64);
    free(hash);

    assert(buraaq_math_sin(0.0) == 0.0);
    assert(buraaq_math_cos(0.0) == 1.0);
    assert(buraaq_math_clamp(5.0, 0.0, 1.0) == 1.0);
    assert(buraaq_math_sign(-2.0) == -1.0);

    int32_t g = buraaq_grid_zeros(2, 2);
    assert(g >= 0);
    assert(buraaq_grid_put_at(g, 0, 0, 2.0) == 1);
    assert(buraaq_grid_put_at(g, 1, 1, 3.0) == 1);
    int32_t ident = buraaq_grid_eye(2);
    int32_t prod = buraaq_grid_matmul(g, ident);
    assert(buraaq_grid_at(prod, 0, 0) == 2.0);
    assert(buraaq_grid_at(prod, 1, 1) == 3.0);
    int32_t v = buraaq_grid_row("1,2,3");
    int32_t w = buraaq_grid_row("4,5,6");
    assert(buraaq_grid_dot(v, w) == 32.0);
    assert(buraaq_grid_sum(v) == 6.0);

    int32_t h = buraaq_hold_new("name,cents");
    assert(h >= 0);
    assert(buraaq_hold_stow(h, "checking,100") == 1);
    assert(buraaq_hold_stow(h, "savings,300") == 1);
    assert(buraaq_hold_rows(h) == 2);
    assert(buraaq_hold_col_mean(h, "cents") == 200.0);
    int32_t kept = buraaq_hold_keep(h, "name", "checking");
    assert(buraaq_hold_rows(kept) == 1);
    char *cell = buraaq_hold_pick(kept, "name", 0);
    assert(strcmp(cell, "checking") == 0);
    free(cell);

#ifdef _WIN32
    {
        char csvpath[MAX_PATH];
        GetTempPathA(MAX_PATH, csvpath);
        strcat(csvpath, "bq_hold.csv");
        assert(buraaq_hold_to_csv(h, csvpath) == 1);
        int32_t loaded = buraaq_hold_from_csv(csvpath);
        assert(buraaq_hold_rows(loaded) == 2);
        DeleteFileA(csvpath);
    }
#else
    {
        const char *csvpath = "/tmp/bq_hold.csv";
        assert(buraaq_hold_to_csv(h, csvpath) == 1);
        int32_t loaded = buraaq_hold_from_csv(csvpath);
        assert(buraaq_hold_rows(loaded) == 2);
        remove(csvpath);
    }
#endif

#ifdef _WIN32
    assert(buraaq_stream_bind(18742) == 1);
    HANDLE th = CreateThread(NULL, 0, echo_once, NULL, 0, NULL);
    assert(th != NULL);
    Sleep(50);
    int32_t sid = buraaq_stream_wire("ws://127.0.0.1:18742/");
    assert(sid >= 0);
    assert(buraaq_stream_say(sid, "buraaq") == 1);
    char *got = buraaq_stream_hear(sid);
    assert(got != NULL);
    assert(strcmp(got, "buraaq") == 0);
    free(got);
    buraaq_stream_hangup(sid);
    WaitForSingleObject(th, 4000);
    CloseHandle(th);
#else
    assert(buraaq_stream_bind(18742) == 1);
    pthread_t th;
    assert(pthread_create(&th, NULL, echo_once_posix, NULL) == 0);
    usleep(50000);
    int32_t sid = buraaq_stream_wire("ws://127.0.0.1:18742/");
    assert(sid >= 0);
    assert(buraaq_stream_say(sid, "buraaq") == 1);
    char *got = buraaq_stream_hear(sid);
    assert(got != NULL);
    assert(strcmp(got, "buraaq") == 0);
    free(got);
    buraaq_stream_hangup(sid);
    pthread_join(th, NULL);
#endif

    void *m = buraaq_mutex_new();
    assert(m != NULL);
    buraaq_mutex_lock(m);
    buraaq_mutex_unlock(m);
    buraaq_mutex_free(m);

    return 0;
}
