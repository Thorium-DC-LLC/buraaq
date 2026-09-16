// Buraaq standard library native runtime — zero-GC, explicit allocation.
#ifndef _POSIX_C_SOURCE
#define _POSIX_C_SOURCE 200809L
#endif
#include "buraaq_std.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <time.h>

#ifdef _WIN32
#include <windows.h>
#include <wininet.h>
#else
#include <pthread.h>
#include <unistd.h>
#include <sys/stat.h>
#include <sys/socket.h>
#include <netdb.h>
#include <arpa/inet.h>
#ifdef BURAAQ_OPENSSL
#include <openssl/err.h>
#include <openssl/ssl.h>
#endif
#endif

static char *dup_str(const char *s) {
    if (!s) return NULL;
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

char *buraaq_file_read(const char *path) {
    if (!path) return NULL;
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    if (fseek(f, 0, SEEK_END) != 0) { fclose(f); return NULL; }
    long sz = ftell(f);
    if (sz < 0) { fclose(f); return NULL; }
    rewind(f);
    char *buf = (char *)malloc((size_t)sz + 1);
    if (!buf) { fclose(f); return NULL; }
    size_t n = fread(buf, 1, (size_t)sz, f);
    fclose(f);
    buf[n] = '\0';
    return buf;
}

int buraaq_file_write(const char *path, const char *contents) {
    if (!path || !contents) return -1;
    FILE *f = fopen(path, "wb");
    if (!f) return -1;
    size_t n = strlen(contents);
    size_t w = fwrite(contents, 1, n, f);
    fclose(f);
    return w == n ? 0 : -1;
}

int buraaq_file_exists(const char *path) {
    if (!path) return 0;
#ifdef _WIN32
    DWORD attr = GetFileAttributesA(path);
    return attr != INVALID_FILE_ATTRIBUTES;
#else
    struct stat st;
    return stat(path, &st) == 0;
#endif
}

char *buraaq_text_concat(const char *a, const char *b) {
    if (!a) a = "";
    if (!b) b = "";
    size_t la = strlen(a), lb = strlen(b);
    char *out = (char *)malloc(la + lb + 1);
    if (!out) return NULL;
    memcpy(out, a, la);
    memcpy(out + la, b, lb + 1);
    return out;
}

int32_t buraaq_text_len(const char *s) { return s ? (int32_t)strlen(s) : 0; }

int32_t buraaq_text_eq(const char *a, const char *b) {
    if (!a) a = "";
    if (!b) b = "";
    return strcmp(a, b);
}

int32_t buraaq_text_byte(const char *s, int32_t i) {
    if (!s || i < 0) return -1;
    size_t n = strlen(s);
    if ((size_t)i >= n) return -1;
    return (int32_t)(unsigned char)s[i];
}

char *buraaq_text_slice(const char *s, int32_t start, int32_t end) {
    if (!s) s = "";
    size_t n = strlen(s);
    if (start < 0) start = 0;
    if (end < start) end = start;
    if ((size_t)start > n) start = (int32_t)n;
    if ((size_t)end > n) end = (int32_t)n;
    size_t len = (size_t)(end - start);
    char *out = (char *)malloc(len + 1);
    if (!out) return NULL;
    memcpy(out, s + start, len);
    out[len] = '\0';
    return out;
}

int32_t buraaq_keepalive_i32(int32_t v) {
    static volatile int32_t sink;
    sink = v;
    return sink;
}

double buraaq_keepalive_f64(double v) {
    static volatile double sink;
    sink = v;
    return sink;
}

void buraaq_bench_report(const char *name, double sec, int32_t n, int32_t checksum) {
    static volatile int32_t sink;
    sink = checksum;
    if (!name) name = "bench";
    if (sec > 0.0) {
        printf("BENCH name=%s time_sec=%.6f ops=%d ops_per_sec=%.0f\n",
               name, sec, n, (double)n / sec);
    } else {
        printf("BENCH name=%s time_sec=%.6f ops=%d ops_per_sec=0\n", name, sec, n);
    }
}

void buraaq_bench_report_f64(const char *name, double sec, int32_t n, double checksum) {
    static volatile double sink;
    sink = checksum;
    buraaq_bench_report(name, sec, n, (int32_t)(checksum * 1000.0));
}

double buraaq_now_sec(void) {
#ifdef _WIN32
    LARGE_INTEGER f, c;
    QueryPerformanceFrequency(&f);
    QueryPerformanceCounter(&c);
    return (double)c.QuadPart / (double)f.QuadPart;
#else
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) return 0.0;
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
#endif
}

double buraaq_math_sqrt(double x) { return sqrt(x); }
double buraaq_math_abs_f64(double x) { return fabs(x); }
int32_t buraaq_math_abs_i32(int32_t x) { return x < 0 ? -x : x; }
double buraaq_math_min_f64(double a, double b) { return a < b ? a : b; }
double buraaq_math_max_f64(double a, double b) { return a > b ? a : b; }
double buraaq_math_sin(double x) { return sin(x); }
double buraaq_math_cos(double x) { return cos(x); }
double buraaq_math_tan(double x) { return tan(x); }
double buraaq_math_asin(double x) { return asin(x); }
double buraaq_math_acos(double x) { return acos(x); }
double buraaq_math_atan(double x) { return atan(x); }
double buraaq_math_atan2(double y, double x) { return atan2(y, x); }
double buraaq_math_sinh(double x) { return sinh(x); }
double buraaq_math_cosh(double x) { return cosh(x); }
double buraaq_math_tanh(double x) { return tanh(x); }
double buraaq_math_exp(double x) { return exp(x); }
double buraaq_math_log(double x) { return log(x); }
double buraaq_math_log10(double x) { return log10(x); }
double buraaq_math_log2(double x) { return log(x) / log(2.0); }
double buraaq_math_pow(double x, double y) { return pow(x, y); }
double buraaq_math_hypot(double x, double y) { return hypot(x, y); }
double buraaq_math_floor(double x) { return floor(x); }
double buraaq_math_ceil(double x) { return ceil(x); }
double buraaq_math_trunc(double x) { return trunc(x); }
double buraaq_math_round(double x) { return round(x); }
double buraaq_math_fmod(double x, double y) { return fmod(x, y); }
double buraaq_math_copysign(double mag, double sgn) { return copysign(mag, sgn); }
double buraaq_math_cbrt(double x) { return cbrt(x); }
double buraaq_math_pi(void) { return 3.14159265358979323846; }
double buraaq_math_euler(void) { return 2.71828182845904523536; }
double buraaq_math_deg(double rad) { return rad * 180.0 / 3.14159265358979323846; }
double buraaq_math_rad(double deg) { return deg * 3.14159265358979323846 / 180.0; }
double buraaq_math_clamp(double x, double lo, double hi) {
    if (x < lo) return lo;
    if (x > hi) return hi;
    return x;
}
double buraaq_math_lerp(double a, double b, double t) { return a + (b - a) * t; }
double buraaq_math_sign(double x) {
    if (x > 0.0) return 1.0;
    if (x < 0.0) return -1.0;
    return 0.0;
}

int64_t buraaq_time_now_ms(void) {
    return (int64_t)time(NULL) * 1000;
}

void buraaq_time_sleep_ms(int64_t ms) {
    if (ms <= 0) return;
#ifdef _WIN32
    Sleep((DWORD)ms);
#else
    usleep((useconds_t)(ms * 1000));
#endif
}

char *buraaq_os_getenv(const char *name) {
    if (!name) return NULL;
    const char *v = getenv(name);
    return v ? dup_str(v) : NULL;
}

int buraaq_rt_argc = 0;
char **buraaq_rt_argv = NULL;

void buraaq_rt_set_args(int argc, char **argv) {
    buraaq_rt_argc = argc;
    buraaq_rt_argv = argv;
}

int32_t buraaq_os_argc(void) { return (int32_t)buraaq_rt_argc; }
char *buraaq_os_argv(int32_t index) {
    if (index < 0 || index >= buraaq_rt_argc || !buraaq_rt_argv) return NULL;
    return dup_str(buraaq_rt_argv[index]);
}

static char *dup_range(const char *start, const char *end) {
    if (!start || end < start) return NULL;
    size_t n = (size_t)(end - start);
    char *out = (char *)malloc(n + 1);
    if (!out) return NULL;
    memcpy(out, start, n);
    out[n] = '\0';
    return out;
}

char *buraaq_json_parse_string_field(const char *json, const char *key) {
    if (!json || !key || !key[0]) return NULL;
    char quoted[128];
    char bare[128];
    snprintf(quoted, sizeof(quoted), "\"%s\"", key);
    snprintf(bare, sizeof(bare), "%s:", key);
    const char *p = strstr(json, quoted);
    if (p) {
        p = strchr(p + strlen(quoted), ':');
        if (!p) return NULL;
    } else {
        p = strstr(json, bare);
        if (!p) return NULL;
        if (p != json) {
            char prev = p[-1];
            if (prev != '{' && prev != ',' && prev != ' ' && prev != '\n' && prev != '\r' && prev != '\t') {
                return NULL;
            }
        }
    }
    while (*p && (*p == ':' || *p == ' ' || *p == '\t')) p++;
    if (*p == '"') {
        p++;
        const char *end = p;
        while (*end && *end != '"') {
            if (*end == '\\' && end[1]) end += 2;
            else end++;
        }
        if (*end != '"') return NULL;
        return dup_range(p, end);
    }
    const char *end = p;
    while (*end && *end != ',' && *end != '}' && *end != ']' && *end != '\n' && *end != '\r') {
        end++;
    }
    while (end > p && (end[-1] == ' ' || end[-1] == '\t')) end--;
    if (end <= p) return NULL;
    return dup_range(p, end);
}

#ifdef _WIN32
static char *http_get_wininet(const char *url) {
    HINTERNET ses = InternetOpenA("buraaq/1.0", INTERNET_OPEN_TYPE_PRECONFIG, NULL, NULL, 0);
    if (!ses) return NULL;
    HINTERNET req = InternetOpenUrlA(ses, url, NULL, 0, INTERNET_FLAG_NO_UI | INTERNET_FLAG_RELOAD, 0);
    if (!req) {
        InternetCloseHandle(ses);
        return NULL;
    }
    size_t cap = 4096, len = 0;
    char *buf = (char *)malloc(cap);
    if (!buf) {
        InternetCloseHandle(req);
        InternetCloseHandle(ses);
        return NULL;
    }
    DWORD n = 0;
    while (InternetReadFile(req, buf + len, (DWORD)(cap - len - 1), &n) && n > 0) {
        len += n;
        if (len + 1 >= cap) {
            cap *= 2;
            char *nb = (char *)realloc(buf, cap);
            if (!nb) {
                free(buf);
                InternetCloseHandle(req);
                InternetCloseHandle(ses);
                return NULL;
            }
            buf = nb;
        }
    }
    buf[len] = '\0';
    InternetCloseHandle(req);
    InternetCloseHandle(ses);
    return buf;
}
#else
static char *http_strip_headers(char *buf) {
    if (!buf) return NULL;
    char *body = strstr(buf, "\r\n\r\n");
    if (body) {
        body += 4;
        char *out = dup_str(body);
        free(buf);
        return out;
    }
    return buf;
}

#ifdef BURAAQ_OPENSSL
static char *http_tls_exchange(int fd, const char *host, const char *req) {
    SSL_library_init();
    SSL_load_error_strings();
    const SSL_METHOD *method = TLS_client_method();
    if (!method) {
        close(fd);
        return NULL;
    }
    SSL_CTX *ctx = SSL_CTX_new(method);
    if (!ctx) {
        close(fd);
        return NULL;
    }
    SSL_CTX_set_default_verify_paths(ctx);
    SSL *ssl = SSL_new(ctx);
    if (!ssl) {
        SSL_CTX_free(ctx);
        close(fd);
        return NULL;
    }
    SSL_set_fd(ssl, fd);
    SSL_set_tlsext_host_name(ssl, host);
    if (SSL_connect(ssl) != 1) {
        SSL_free(ssl);
        SSL_CTX_free(ctx);
        close(fd);
        return NULL;
    }
    if (SSL_write(ssl, req, (int)strlen(req)) <= 0) {
        SSL_shutdown(ssl);
        SSL_free(ssl);
        SSL_CTX_free(ctx);
        close(fd);
        return NULL;
    }
    size_t cap = 4096, len = 0;
    char *buf = (char *)malloc(cap);
    if (!buf) {
        SSL_shutdown(ssl);
        SSL_free(ssl);
        SSL_CTX_free(ctx);
        close(fd);
        return NULL;
    }
    for (;;) {
        int n = SSL_read(ssl, buf + len, (int)(cap - len - 1));
        if (n <= 0) break;
        len += (size_t)n;
        if (len + 1 >= cap) {
            cap *= 2;
            char *nb = (char *)realloc(buf, cap);
            if (!nb) {
                free(buf);
                SSL_shutdown(ssl);
                SSL_free(ssl);
                SSL_CTX_free(ctx);
                close(fd);
                return NULL;
            }
            buf = nb;
        }
    }
    buf[len] = '\0';
    SSL_shutdown(ssl);
    SSL_free(ssl);
    SSL_CTX_free(ctx);
    close(fd);
    return http_strip_headers(buf);
}
#endif

static char *http_get_posix(const char *url) {
    const char *p = url;
    int https = 0;
    if (strncmp(p, "https://", 8) == 0) {
        https = 1;
        p += 8;
    } else if (strncmp(p, "http://", 7) == 0) {
        p += 7;
    } else {
        return NULL;
    }
    char host[256];
    int port = https ? 443 : 80;
    const char *path = strchr(p, '/');
    size_t host_len = path ? (size_t)(path - p) : strlen(p);
    const char *colon = memchr(p, ':', host_len);
    if (colon) {
        host_len = (size_t)(colon - p);
        port = atoi(colon + 1);
    }
    if (host_len >= sizeof(host)) return NULL;
    memcpy(host, p, host_len);
    host[host_len] = '\0';
    if (!path) path = "/";

#ifndef BURAAQ_OPENSSL
    if (https) return NULL;
#endif

    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_family = AF_UNSPEC;
    char port_s[16];
    snprintf(port_s, sizeof(port_s), "%d", port);
    struct addrinfo *res = NULL;
    if (getaddrinfo(host, port_s, &hints, &res) != 0 || !res) return NULL;
    int fd = socket(res->ai_family, res->ai_socktype, res->ai_protocol);
    if (fd < 0) {
        freeaddrinfo(res);
        return NULL;
    }
    if (connect(fd, res->ai_addr, (socklen_t)res->ai_addrlen) != 0) {
        close(fd);
        freeaddrinfo(res);
        return NULL;
    }
    freeaddrinfo(res);
    char req[1024];
    snprintf(req, sizeof(req), "GET %s HTTP/1.0\r\nHost: %s\r\nUser-Agent: buraaq/1.0\r\n\r\n", path, host);
#ifdef BURAAQ_OPENSSL
    if (https) {
        return http_tls_exchange(fd, host, req);
    }
#endif
    if (send(fd, req, strlen(req), 0) < 0) {
        close(fd);
        return NULL;
    }
    size_t cap = 4096, len = 0;
    char *buf = (char *)malloc(cap);
    if (!buf) {
        close(fd);
        return NULL;
    }
    for (;;) {
        ssize_t n = recv(fd, buf + len, cap - len - 1, 0);
        if (n <= 0) break;
        len += (size_t)n;
        if (len + 1 >= cap) {
            cap *= 2;
            char *nb = (char *)realloc(buf, cap);
            if (!nb) {
                free(buf);
                close(fd);
                return NULL;
            }
            buf = nb;
        }
    }
    close(fd);
    buf[len] = '\0';
    return http_strip_headers(buf);
}
#endif

char *buraaq_http_get_body(const char *url) {
    if (!url) return NULL;
    if (strncmp(url, "file://", 7) == 0) {
        const char *path = url + 7;
#ifdef _WIN32
        if (path[0] == '/') path++;
#endif
        return buraaq_file_read(path);
    }
#ifdef _WIN32
    char *body = http_get_wininet(url);
#else
    char *body = http_get_posix(url);
#endif
    if (body) return body;
    return dup_str("{\"status\":0,\"body\":\"http get failed\"}");
}

static uint32_t rotr32(uint32_t x, int n) { return (x >> n) | (x << (32 - n)); }

char *buraaq_crypto_sha256_hex(const char *data) {
    static const uint32_t K[64] = {
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2
    };
    uint32_t h[8] = {
        0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19
    };
    const uint8_t *msg = (const uint8_t *)(data ? data : "");
    uint64_t bitlen = (uint64_t)strlen((const char *)msg) * 8;
    size_t len = (size_t)(bitlen / 8);
    size_t pad = ((len + 9 + 63) / 64) * 64;
    uint8_t *buf = (uint8_t *)calloc(pad, 1);
    if (!buf) return NULL;
    memcpy(buf, msg, len);
    buf[len] = 0x80;
    for (int i = 0; i < 8; i++) buf[pad - 1 - i] = (uint8_t)(bitlen >> (8 * i));
    for (size_t off = 0; off < pad; off += 64) {
        uint32_t w[64];
        for (int i = 0; i < 16; i++) {
            w[i] = ((uint32_t)buf[off + i * 4] << 24) | ((uint32_t)buf[off + i * 4 + 1] << 16)
                | ((uint32_t)buf[off + i * 4 + 2] << 8) | (uint32_t)buf[off + i * 4 + 3];
        }
        for (int i = 16; i < 64; i++) {
            uint32_t s0 = rotr32(w[i - 15], 7) ^ rotr32(w[i - 15], 18) ^ (w[i - 15] >> 3);
            uint32_t s1 = rotr32(w[i - 2], 17) ^ rotr32(w[i - 2], 19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16] + s0 + w[i - 7] + s1;
        }
        uint32_t a = h[0], b = h[1], c = h[2], d = h[3], e = h[4], f = h[5], g = h[6], hh = h[7];
        for (int i = 0; i < 64; i++) {
            uint32_t S1 = rotr32(e, 6) ^ rotr32(e, 11) ^ rotr32(e, 25);
            uint32_t ch = (e & f) ^ ((~e) & g);
            uint32_t t1 = hh + S1 + ch + K[i] + w[i];
            uint32_t S0 = rotr32(a, 2) ^ rotr32(a, 13) ^ rotr32(a, 22);
            uint32_t maj = (a & b) ^ (a & c) ^ (b & c);
            uint32_t t2 = S0 + maj;
            hh = g; g = f; f = e; e = d + t1; d = c; c = b; b = a; a = t1 + t2;
        }
        h[0] += a; h[1] += b; h[2] += c; h[3] += d; h[4] += e; h[5] += f; h[6] += g; h[7] += hh;
    }
    free(buf);
    char *out = (char *)malloc(65);
    if (!out) return NULL;
    for (int i = 0; i < 8; i++) sprintf(out + i * 8, "%08x", h[i]);
    out[64] = '\0';
    return out;
}

void *buraaq_mutex_new(void) {
#ifdef _WIN32
    CRITICAL_SECTION *cs = (CRITICAL_SECTION *)malloc(sizeof(CRITICAL_SECTION));
    if (!cs) return NULL;
    InitializeCriticalSection(cs);
    return cs;
#else
    pthread_mutex_t *m = (pthread_mutex_t *)malloc(sizeof(pthread_mutex_t));
    if (!m) return NULL;
    pthread_mutex_init(m, NULL);
    return m;
#endif
}

void buraaq_mutex_lock(void *m) {
    if (!m) return;
#ifdef _WIN32
    EnterCriticalSection((CRITICAL_SECTION *)m);
#else
    pthread_mutex_lock((pthread_mutex_t *)m);
#endif
}

void buraaq_mutex_unlock(void *m) {
    if (!m) return;
#ifdef _WIN32
    LeaveCriticalSection((CRITICAL_SECTION *)m);
#else
    pthread_mutex_unlock((pthread_mutex_t *)m);
#endif
}

void buraaq_mutex_free(void *m) {
    if (!m) return;
#ifdef _WIN32
    DeleteCriticalSection((CRITICAL_SECTION *)m);
#else
    pthread_mutex_destroy((pthread_mutex_t *)m);
#endif
    free(m);
}

int32_t buraaq_process_exit_code(const char *cmd) {
    if (!cmd) return -1;
    return (int32_t)system(cmd);
}
