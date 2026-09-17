/* HTTP/1.1 + optional TLS (OpenSSL loaded at runtime) + PostgreSQL (libpq loaded at runtime). */
#ifndef _POSIX_C_SOURCE
#define _POSIX_C_SOURCE 200809L
#endif
#include "buraaq_std.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#ifdef _WIN32
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
#include <direct.h>
#include <io.h>
#include <sys/stat.h>
#include <process.h>
#define bq_ncasecmp _strnicmp
typedef SOCKET bq_sock;
#define BQ_INVALID INVALID_SOCKET
#define bq_close_sock(s) closesocket(s)
#define bq_last_err() WSAGetLastError()
#else
#include <unistd.h>
#include <errno.h>
#include <fcntl.h>
#include <strings.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/time.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <dlfcn.h>
#include <sys/wait.h>
#define bq_ncasecmp strncasecmp
typedef int bq_sock;
#define BQ_INVALID (-1)
#define bq_close_sock(s) close(s)
#define SOCKET_ERROR (-1)
#define bq_last_err() errno
#endif

static char *dup_str(const char *s) {
    if (!s) s = "";
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

static char *dup_empty(void) { return dup_str(""); }

static void hdr_get(const char *headers, const char *key, char *out, size_t cap) {
    if (!out || cap == 0) return;
    out[0] = 0;
    if (!headers || !key) return;
    size_t klen = strlen(key);
    const char *p = headers;
    while (*p) {
        const char *eol = strstr(p, "\r\n");
        size_t n = eol ? (size_t)(eol - p) : strlen(p);
        if (n > klen + 1 && bq_ncasecmp(p, key, (int)klen) == 0 && p[klen] == ':') {
            const char *v = p + klen + 1;
            while (v < p + n && (*v == ' ' || *v == '\t')) v++;
            size_t vn = (size_t)((p + n) - v);
            if (vn >= cap) vn = cap - 1;
            memcpy(out, v, vn);
            out[vn] = 0;
            return;
        }
        if (!eol) break;
        p = eol + 2;
    }
}

static int ct_eq_str(const char *a, const char *b) {
    if (!a) a = "";
    if (!b) b = "";
    size_t na = strlen(a), nb = strlen(b);
    if (na != nb) return 0;
    unsigned char d = 0;
    for (size_t i = 0; i < na; i++) d |= (unsigned char)a[i] ^ (unsigned char)b[i];
    return d == 0;
}

static void bq_set_recv_timeout(bq_sock fd, int ms) {
#ifdef _WIN32
    DWORD t = (DWORD)ms;
    setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, (const char *)&t, sizeof(t));
    setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, (const char *)&t, sizeof(t));
#else
    struct timeval tv;
    tv.tv_sec = ms / 1000;
    tv.tv_usec = (ms % 1000) * 1000;
    setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));
    setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, &tv, sizeof(tv));
#endif
}

/* ---------- sockets ---------- */

#define BQ_MAX_LISTEN 4
#define BQ_MAX_CONN 64
#define BQ_REQ_MAX (1u << 20)

typedef struct {
    bq_sock fd;
    int https;
} ListenSlot;

typedef struct {
    int in_use;
    int https;
    int peer_loopback;
    bq_sock fd;
    void *ssl;
    char method[16];
    char path[2048];
    char origin[384];
    char apikey[192];
    char *body;
} Conn;

static ListenSlot g_listen[BQ_MAX_LISTEN];
static int g_nlisten = 0;
static Conn g_conn[BQ_MAX_CONN];
static int g_wsa = 0;
static char g_cors_origin[1024];
static char g_api_key[128];
static int g_api_lock = 0;
static int g_public = 0;
static int g_http_port = 8080;
static int g_tls_port = 8443;

static int env_flag(const char *name) {
    const char *v = getenv(name);
    if (!v || !v[0]) return 0;
    return v[0] == '1' || v[0] == 'y' || v[0] == 'Y' || v[0] == 't' || v[0] == 'T';
}

static int env_flag_default(const char *name, int fallback) {
    const char *v = getenv(name);
    if (!v || !v[0]) return fallback;
    if (v[0] == '0' || v[0] == 'n' || v[0] == 'N' || v[0] == 'f' || v[0] == 'F') return 0;
    return env_flag(name) ? 1 : fallback;
}

/* Local by default. BURAAQ_PUBLIC=1 or BURAAQ_BIND=0.0.0.0 for every interface. */
static uint32_t bq_listen_addr(void) {
    const char *bind = getenv("BURAAQ_BIND");
    if (bind && bind[0]) {
        if (strcmp(bind, "0.0.0.0") == 0 || strcmp(bind, "*") == 0) {
            g_public = 1;
            return INADDR_ANY;
        }
        if (strcmp(bind, "127.0.0.1") == 0 || strcmp(bind, "localhost") == 0) {
            g_public = 0;
            return htonl(INADDR_LOOPBACK);
        }
    }
    if (env_flag("BURAAQ_PUBLIC")) {
        g_public = 1;
        return INADDR_ANY;
    }
    g_public = 0;
    return htonl(INADDR_LOOPBACK);
}

static int sock_init(void) {
#ifdef _WIN32
    if (!g_wsa) {
        WSADATA w;
        if (WSAStartup(MAKEWORD(2, 2), &w) != 0) return -1;
        g_wsa = 1;
    }
#endif
    setvbuf(stdout, NULL, _IONBF, 0);
    setvbuf(stderr, NULL, _IONBF, 0);
    return 0;
}

/* ---------- OpenSSL (optional, dynamic) ---------- */

typedef struct {
    int loaded;
    void *libssl;
    void *libcrypto;
    void *(*TLS_server_method)(void);
    void *(*SSL_CTX_new)(const void *);
    int (*SSL_CTX_use_certificate_file)(void *, const char *, int);
    int (*SSL_CTX_use_PrivateKey_file)(void *, const char *, int);
    int (*SSL_CTX_check_private_key)(const void *);
    long (*SSL_CTX_ctrl)(void *, int, long, void *);
    void *(*SSL_new)(void *);
    int (*SSL_set_fd)(void *, int);
    int (*SSL_accept)(void *);
    int (*SSL_read)(void *, void *, int);
    int (*SSL_write)(void *, const void *, int);
    void (*SSL_free)(void *);
    void (*SSL_CTX_free)(void *);
    int (*OPENSSL_init_ssl)(uint64_t, const void *);
    int (*SSL_get_error)(const void *, int);
    unsigned long (*ERR_get_error)(void);
    void *ctx;
} SslApi;

static SslApi g_ssl;

#ifdef _WIN32
static void bq_add_pg_bin(void) {
    if (GetFileAttributesA("C:\\Program Files\\PostgreSQL\\17\\bin\\libpq.dll") != INVALID_FILE_ATTRIBUTES) {
        SetDllDirectoryA("C:\\Program Files\\PostgreSQL\\17\\bin");
    } else if (GetFileAttributesA("C:\\Program Files\\PostgreSQL\\16\\bin\\libpq.dll") != INVALID_FILE_ATTRIBUTES) {
        SetDllDirectoryA("C:\\Program Files\\PostgreSQL\\16\\bin");
    }
}

static void *bq_dl(const char *path) {
    if (strchr(path, '\\') || strchr(path, '/')) {
        return (void *)LoadLibraryExA(path, NULL, LOAD_WITH_ALTERED_SEARCH_PATH);
    }
    return (void *)LoadLibraryA(path);
}
static void *bq_sym(void *h, const char *n) { return (void *)GetProcAddress((HMODULE)h, n); }
#else
static void *bq_dl(const char *path) { return dlopen(path, RTLD_NOW); }
static void *bq_sym(void *h, const char *n) { return dlsym(h, n); }
#endif

static int ssl_load(void) {
    if (g_ssl.loaded) return g_ssl.ctx ? 1 : 0;
    g_ssl.loaded = 1;
#ifdef _WIN32
    bq_add_pg_bin();
#endif
#ifdef _WIN32
    const char *ssl_dlls[] = {
        "libssl-3-x64.dll",
        "C:\\Program Files\\PostgreSQL\\17\\bin\\libssl-3-x64.dll",
        "C:\\Program Files\\PostgreSQL\\16\\bin\\libssl-3-x64.dll",
        NULL
    };
    const char *crypto_dlls[] = {
        "libcrypto-3-x64.dll",
        "C:\\Program Files\\PostgreSQL\\17\\bin\\libcrypto-3-x64.dll",
        "C:\\Program Files\\PostgreSQL\\16\\bin\\libcrypto-3-x64.dll",
        NULL
    };
    for (int i = 0; crypto_dlls[i]; i++) {
        g_ssl.libcrypto = bq_dl(crypto_dlls[i]);
        if (g_ssl.libcrypto) break;
    }
    for (int i = 0; ssl_dlls[i]; i++) {
        g_ssl.libssl = bq_dl(ssl_dlls[i]);
        if (g_ssl.libssl) break;
    }
#else
    g_ssl.libcrypto = bq_dl("libcrypto.so.3");
    if (!g_ssl.libcrypto) g_ssl.libcrypto = bq_dl("libcrypto.so");
    g_ssl.libssl = bq_dl("libssl.so.3");
    if (!g_ssl.libssl) g_ssl.libssl = bq_dl("libssl.so");
#endif
    if (!g_ssl.libssl) return 0;
#define SSL_SYM(name) g_ssl.name = (void *)bq_sym(g_ssl.libssl, #name)
    SSL_SYM(TLS_server_method);
    SSL_SYM(SSL_CTX_new);
    SSL_SYM(SSL_CTX_use_certificate_file);
    SSL_SYM(SSL_CTX_use_PrivateKey_file);
    SSL_SYM(SSL_CTX_check_private_key);
    SSL_SYM(SSL_CTX_ctrl);
    SSL_SYM(SSL_new);
    SSL_SYM(SSL_set_fd);
    SSL_SYM(SSL_accept);
    SSL_SYM(SSL_read);
    SSL_SYM(SSL_write);
    SSL_SYM(SSL_free);
    SSL_SYM(SSL_CTX_free);
    SSL_SYM(OPENSSL_init_ssl);
    SSL_SYM(SSL_get_error);
#undef SSL_SYM
    if (!g_ssl.TLS_server_method || !g_ssl.SSL_CTX_new || !g_ssl.SSL_new) return 0;
    if (g_ssl.OPENSSL_init_ssl) g_ssl.OPENSSL_init_ssl(0, NULL);
    const char *cert = getenv("BURAAQ_TLS_CERT");
    const char *key = getenv("BURAAQ_TLS_KEY");
    if (!cert || !cert[0]) cert = "cert.pem";
    if (!key || !key[0]) key = "key.pem";
    g_ssl.ctx = g_ssl.SSL_CTX_new(g_ssl.TLS_server_method());
    if (!g_ssl.ctx) return 0;
    /* SSL_CTRL_SET_MIN_PROTO_VERSION=123, TLS1_2_VERSION=0x0303 */
    if (g_ssl.SSL_CTX_ctrl) g_ssl.SSL_CTX_ctrl(g_ssl.ctx, 123, 0x0303, NULL);
    if (g_ssl.SSL_CTX_use_certificate_file(g_ssl.ctx, cert, 1 /* SSL_FILETYPE_PEM */) != 1) {
        g_ssl.SSL_CTX_free(g_ssl.ctx);
        g_ssl.ctx = NULL;
        return 0;
    }
    if (g_ssl.SSL_CTX_use_PrivateKey_file(g_ssl.ctx, key, 1) != 1) {
        g_ssl.SSL_CTX_free(g_ssl.ctx);
        g_ssl.ctx = NULL;
        return 0;
    }
    if (g_ssl.SSL_CTX_check_private_key && g_ssl.SSL_CTX_check_private_key(g_ssl.ctx) != 1) {
        g_ssl.SSL_CTX_free(g_ssl.ctx);
        g_ssl.ctx = NULL;
        return 0;
    }
    return 1;
}

static int conn_read(Conn *c, char *buf, int n) {
    if (c->https && c->ssl && g_ssl.SSL_read) return g_ssl.SSL_read(c->ssl, buf, n);
#ifdef _WIN32
    return recv(c->fd, buf, n, 0);
#else
    return (int)recv(c->fd, buf, (size_t)n, 0);
#endif
}

static int conn_write(Conn *c, const char *buf, int n) {
    int sent = 0;
    while (sent < n) {
        int k;
        if (c->https && c->ssl && g_ssl.SSL_write)
            k = g_ssl.SSL_write(c->ssl, buf + sent, n - sent);
        else {
#ifdef _WIN32
            k = send(c->fd, buf + sent, n - sent, 0);
#else
            k = (int)send(c->fd, buf + sent, (size_t)(n - sent), 0);
#endif
        }
        if (k <= 0) return -1;
        sent += k;
    }
    return sent;
}

static Conn *conn_at(int id) {
    if (id < 1 || id > BQ_MAX_CONN) return NULL;
    Conn *c = &g_conn[id - 1];
    return c->in_use ? c : NULL;
}

static void conn_clear(Conn *c) {
    if (!c) return;
    if (c->ssl && g_ssl.SSL_free) g_ssl.SSL_free(c->ssl);
    if (c->fd != BQ_INVALID) bq_close_sock(c->fd);
    free(c->body);
    memset(c, 0, sizeof(*c));
    c->fd = BQ_INVALID;
}

static int parse_request(Conn *c, char *raw, size_t n) {
    c->method[0] = 0;
    c->path[0] = 0;
    c->origin[0] = 0;
    c->apikey[0] = 0;
    free(c->body);
    c->body = dup_empty();
    char *line_end = strstr(raw, "\r\n");
    if (!line_end) return -1;
    *line_end = 0;
    char *sp1 = strchr(raw, ' ');
    if (!sp1) return -1;
    *sp1 = 0;
    strncpy(c->method, raw, sizeof(c->method) - 1);
    char *sp2 = strchr(sp1 + 1, ' ');
    if (!sp2) sp2 = sp1 + 1 + strlen(sp1 + 1);
    *sp2 = 0;
    strncpy(c->path, sp1 + 1, sizeof(c->path) - 1);
    char *q = strchr(c->path, '?');
    if (q) *q = 0;
    char *headers = line_end + 2;
    char auth[192];
    auth[0] = 0;
    hdr_get(headers, "Origin", c->origin, sizeof(c->origin));
    hdr_get(headers, "X-Api-Key", c->apikey, sizeof(c->apikey));
    hdr_get(headers, "Authorization", auth, sizeof(auth));
    if (!c->apikey[0] && bq_ncasecmp(auth, "Bearer ", 7) == 0) {
        strncpy(c->apikey, auth + 7, sizeof(c->apikey) - 1);
    }
    char *sep = strstr(headers, "\r\n\r\n");
    size_t header_end = sep ? (size_t)(sep - raw) + 4 : n;
    int content_len = 0;
    char *cl = strstr(headers, "Content-Length:");
    if (!cl) cl = strstr(headers, "content-length:");
    if (cl) content_len = atoi(cl + 15);
    if (content_len < 0) content_len = 0;
    if ((size_t)content_len > BQ_REQ_MAX) content_len = (int)BQ_REQ_MAX;
    const char *body = sep ? sep + 4 : "";
    size_t have = raw + n > body ? n - (size_t)(body - raw) : 0;
    char *out = (char *)malloc((size_t)content_len + 1);
    if (!out) return -1;
    memset(out, 0, (size_t)content_len + 1);
    if (have > 0) memcpy(out, body, have < (size_t)content_len ? have : (size_t)content_len);
    int got = (int)(have < (size_t)content_len ? have : (size_t)content_len);
    while (got < content_len) {
        int k = conn_read(c, out + got, content_len - got);
        if (k <= 0) break;
        got += k;
    }
    out[content_len] = 0;
    free(c->body);
    c->body = out;
    (void)header_end;
    return 0;
}

static int recv_http(Conn *c) {
    char *buf = (char *)malloc(BQ_REQ_MAX + 1);
    if (!buf) return -1;
    size_t n = 0;
    while (n < BQ_REQ_MAX) {
        int k = conn_read(c, buf + n, (int)(BQ_REQ_MAX - n));
        if (k <= 0) {
            if (n == 0) {
                free(buf);
                return -1;
            }
            break;
        }
        n += (size_t)k;
        buf[n] = 0;
        if (strstr(buf, "\r\n\r\n")) break;
    }
    buf[n] = 0;
    int rc = parse_request(c, buf, n);
    free(buf);
    return rc;
}

int32_t buraaq_http_listen(int32_t port, int32_t https) {
    if (sock_init() != 0) return 0;
    if (g_nlisten >= BQ_MAX_LISTEN) return 0;
    if (https && !ssl_load()) {
        fprintf(stderr, "buraaq: HTTPS requested but TLS cert/OpenSSL not available (need cert.pem + key.pem)\n");
        return 0;
    }
    bq_sock fd = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (fd == BQ_INVALID) return 0;
    int on = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, (const char *)&on, sizeof(on));
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = bq_listen_addr();
    addr.sin_port = htons((uint16_t)port);
    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) == SOCKET_ERROR) {
        bq_close_sock(fd);
        return 0;
    }
    if (listen(fd, 128) == SOCKET_ERROR) {
        bq_close_sock(fd);
        return 0;
    }
    g_listen[g_nlisten].fd = fd;
    g_listen[g_nlisten].https = https ? 1 : 0;
    g_nlisten++;
    return (int32_t)g_nlisten;
}

int32_t buraaq_http_accept(int32_t unused_listener) {
    (void)unused_listener;
    if (g_nlisten <= 0) return 0;
    fd_set rfds;
    FD_ZERO(&rfds);
    bq_sock maxfd = 0;
    for (int i = 0; i < g_nlisten; i++) {
        FD_SET(g_listen[i].fd, &rfds);
        if (g_listen[i].fd > maxfd) maxfd = g_listen[i].fd;
    }
    if (select((int)maxfd + 1, &rfds, NULL, NULL, NULL) <= 0) return 0;
    ListenSlot *ls = NULL;
    for (int i = 0; i < g_nlisten; i++) {
        if (FD_ISSET(g_listen[i].fd, &rfds)) {
            ls = &g_listen[i];
            break;
        }
    }
    if (!ls) return 0;
    struct sockaddr_in peer;
    memset(&peer, 0, sizeof(peer));
#ifdef _WIN32
    int plen = (int)sizeof(peer);
#else
    socklen_t plen = sizeof(peer);
#endif
    bq_sock cfd = accept(ls->fd, (struct sockaddr *)&peer, &plen);
    if (cfd == BQ_INVALID) return 0;
    /* Incomplete browser/Next probes must not stall the whole accept loop. */
    bq_set_recv_timeout(cfd, 3000);
    int slot = -1;
    for (int i = 0; i < BQ_MAX_CONN; i++) {
        if (!g_conn[i].in_use) {
            slot = i;
            break;
        }
    }
    if (slot < 0) {
        bq_close_sock(cfd);
        return 0;
    }
    Conn *c = &g_conn[slot];
    memset(c, 0, sizeof(*c));
    c->in_use = 1;
    c->fd = cfd;
    c->https = ls->https;
    c->peer_loopback = peer.sin_addr.s_addr == htonl(INADDR_LOOPBACK) ? 1 : 0;
    c->body = dup_empty();
    if (c->https) {
        if (!ssl_load() || !g_ssl.ctx) {
            conn_clear(c);
            return 0;
        }
        c->ssl = g_ssl.SSL_new(g_ssl.ctx);
        if (!c->ssl) {
            conn_clear(c);
            return 0;
        }
        g_ssl.SSL_set_fd(c->ssl, (int)cfd);
        if (g_ssl.SSL_accept(c->ssl) <= 0) {
            conn_clear(c);
            return 0;
        }
    }
    if (recv_http(c) != 0) {
        conn_clear(c);
        return 0;
    }
    return (int32_t)(slot + 1);
}

char *buraaq_http_method(int32_t id) {
    Conn *c = conn_at(id);
    return dup_str(c ? c->method : "");
}

char *buraaq_http_path(int32_t id) {
    Conn *c = conn_at(id);
    return dup_str(c ? c->path : "");
}

char *buraaq_http_body(int32_t id) {
    Conn *c = conn_at(id);
    return dup_str(c && c->body ? c->body : "");
}

int32_t buraaq_http_path_starts(int32_t id, const char *prefix) {
    Conn *c = conn_at(id);
    if (!c || !prefix) return 0;
    size_t n = strlen(prefix);
    return strncmp(c->path, prefix, n) == 0 ? 1 : 0;
}

int32_t buraaq_http_path_id(int32_t id) {
    Conn *c = conn_at(id);
    if (!c) return -1;
    const char *s = strrchr(c->path, '/');
    if (!s || !s[1]) return -1;
    return atoi(s + 1);
}

char *buraaq_http_json_field(int32_t id, const char *key) {
    Conn *c = conn_at(id);
    return buraaq_json_parse_string_field(c && c->body ? c->body : "", key);
}

int32_t buraaq_http_reply(int32_t id, int32_t status, const char *ctype, const char *body) {
    Conn *c = conn_at(id);
    if (!c) return -1;
    if (!ctype) ctype = "text/plain; charset=utf-8";
    if (!body) body = "";
    const char *reason = "OK";
    if (status == 201) reason = "Created";
    else if (status == 204) reason = "No Content";
    else if (status == 400) reason = "Bad Request";
    else if (status == 401) reason = "Unauthorized";
    else if (status == 403) reason = "Forbidden";
    else if (status == 404) reason = "Not Found";
    else if (status == 405) reason = "Method Not Allowed";
    else if (status == 413) reason = "Payload Too Large";
    else if (status == 500) reason = "Internal Server Error";
    const char *acao = "*";
    char acao_buf[384];
    if (g_cors_origin[0] && strcmp(g_cors_origin, "*") != 0) {
        acao = NULL;
        const char *list = g_cors_origin;
        const char *orig = c->origin;
        if (orig[0]) {
            const char *p = list;
            while (*p) {
                while (*p == ' ' || *p == ',') p++;
                const char *start = p;
                while (*p && *p != ',') p++;
                size_t n = (size_t)(p - start);
                while (n > 0 && (start[n - 1] == ' ' || start[n - 1] == '\t')) n--;
                if (n == strlen(orig) && strncmp(start, orig, n) == 0) {
                    strncpy(acao_buf, orig, sizeof(acao_buf) - 1);
                    acao_buf[sizeof(acao_buf) - 1] = 0;
                    acao = acao_buf;
                    break;
                }
                if (*p == ',') p++;
            }
        }
    } else if (c->origin[0] && g_cors_origin[0] == 0) {
        acao = NULL;
    }
    char hdr[1024];
    int blen = (int)strlen(body);
    int n;
    if (acao) {
        n = snprintf(
            hdr,
            sizeof(hdr),
            "HTTP/1.1 %d %s\r\n"
            "Content-Type: %s\r\n"
            "Content-Length: %d\r\n"
            "Connection: close\r\n"
            "Access-Control-Allow-Origin: %s\r\n"
            "Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\n"
            "Access-Control-Allow-Headers: Content-Type, Authorization, X-Api-Key\r\n"
            "Access-Control-Max-Age: 600\r\n"
            "Vary: Origin\r\n"
            "\r\n",
            status,
            reason,
            ctype,
            blen,
            acao
        );
    } else {
        n = snprintf(
            hdr,
            sizeof(hdr),
            "HTTP/1.1 %d %s\r\n"
            "Content-Type: %s\r\n"
            "Content-Length: %d\r\n"
            "Connection: close\r\n"
            "Vary: Origin\r\n"
            "\r\n",
            status,
            reason,
            ctype,
            blen
        );
    }
    if (n < 0) return -1;
    if (conn_write(c, hdr, n) < 0) return -1;
    if (blen > 0 && conn_write(c, body, blen) < 0) return -1;
    return 0;
}

void buraaq_http_close(int32_t id) {
    Conn *c = conn_at(id);
    if (c) conn_clear(c);
}

/* ---------- PostgreSQL via libpq ---------- */

typedef struct {
    int loaded;
    void *lib;
    void *(*PQconnectdb)(const char *);
    int (*PQstatus)(void *);
    char *(*PQerrorMessage)(void *);
    void *(*PQexec)(void *, const char *);
    int (*PQresultStatus)(void *);
    int (*PQntuples)(void *);
    int (*PQnfields)(void *);
    char *(*PQfname)(void *, int);
    char *(*PQgetvalue)(void *, int, int);
    void (*PQclear)(void *);
    void (*PQfinish)(void *);
    char *(*PQescapeLiteral)(void *, const char *, size_t);
    void (*PQfreemem)(void *);
} PqApi;

static PqApi g_pq;
static void *g_pg = NULL;
static char g_pg_err[256];
static char g_pg_last[2048];

static void pg_set_err(const char *msg) {
    snprintf(g_pg_err, sizeof(g_pg_err), "%s", msg && msg[0] ? msg : "not connected");
    g_pg_err[sizeof(g_pg_err) - 1] = 0;
}

static int pq_load(void) {
    if (g_pq.loaded) return g_pq.PQconnectdb != NULL && g_pq.PQexec != NULL;
    g_pq.loaded = 1;
#ifdef _WIN32
    bq_add_pg_bin();
#endif
#ifdef _WIN32
    const char *dlls[] = {
        "libpq.dll",
        "C:\\Program Files\\PostgreSQL\\17\\bin\\libpq.dll",
        "C:\\Program Files\\PostgreSQL\\16\\bin\\libpq.dll",
        NULL
    };
    for (int i = 0; dlls[i]; i++) {
        g_pq.lib = bq_dl(dlls[i]);
        if (g_pq.lib) break;
    }
#else
    const char *sos[] = {
        "libpq.so.5",
        "libpq.so",
        "/lib/x86_64-linux-gnu/libpq.so.5",
        "/usr/lib/x86_64-linux-gnu/libpq.so.5",
        "/usr/lib/libpq.so.5",
        NULL
    };
    for (int i = 0; sos[i]; i++) {
        g_pq.lib = bq_dl(sos[i]);
        if (g_pq.lib) break;
    }
#endif
    if (!g_pq.lib) return 0;
#define PQ_SYM(name) g_pq.name = (void *)bq_sym(g_pq.lib, #name)
    PQ_SYM(PQconnectdb);
    PQ_SYM(PQstatus);
    PQ_SYM(PQerrorMessage);
    PQ_SYM(PQexec);
    PQ_SYM(PQresultStatus);
    PQ_SYM(PQntuples);
    PQ_SYM(PQnfields);
    PQ_SYM(PQfname);
    PQ_SYM(PQgetvalue);
    PQ_SYM(PQclear);
    PQ_SYM(PQfinish);
    PQ_SYM(PQescapeLiteral);
    PQ_SYM(PQfreemem);
#undef PQ_SYM
    return g_pq.PQconnectdb != NULL && g_pq.PQexec != NULL && g_pq.PQstatus != NULL;
}

static void json_append(char **buf, size_t *len, size_t *cap, const char *s) {
    size_t n = strlen(s);
    if (*len + n + 1 > *cap) {
        size_t nc = (*cap + n + 256) * 2;
        char *nb = (char *)realloc(*buf, nc);
        if (!nb) return;
        *buf = nb;
        *cap = nc;
    }
    memcpy(*buf + *len, s, n + 1);
    *len += n;
}

static void json_escape_append(char **buf, size_t *len, size_t *cap, const char *s) {
    if (!s) s = "";
    json_append(buf, len, cap, "\"");
    for (; *s; s++) {
        char tmp[8];
        unsigned char ch = (unsigned char)*s;
        if (ch == '"' || ch == '\\') {
            tmp[0] = '\\';
            tmp[1] = (char)ch;
            tmp[2] = 0;
            json_append(buf, len, cap, tmp);
        } else if (ch == '\n') json_append(buf, len, cap, "\\n");
        else if (ch == '\r') json_append(buf, len, cap, "\\r");
        else if (ch == '\t') json_append(buf, len, cap, "\\t");
        else if (ch < 0x20) {
            snprintf(tmp, sizeof(tmp), "\\u%04x", ch);
            json_append(buf, len, cap, tmp);
        } else {
            tmp[0] = (char)ch;
            tmp[1] = 0;
            json_append(buf, len, cap, tmp);
        }
    }
    json_append(buf, len, cap, "\"");
}

static void pg_putenv(const char *k, const char *v) {
    if (!k || !k[0] || !v) return;
#ifdef _WIN32
    _putenv_s(k, v);
#else
    setenv(k, v, 0);
#endif
}

static void pg_load_env_file(const char *path) {
    FILE *f = fopen(path, "r");
    if (!f) return;
    char line[2048];
    while (fgets(line, sizeof(line), f)) {
        char *s = line;
        while (*s == ' ' || *s == '\t') s++;
        if (*s == 0 || *s == '#' || *s == '\n' || *s == '\r') continue;
        char *eq = strchr(s, '=');
        if (!eq) continue;
        *eq = 0;
        char *k = s;
        char *v = eq + 1;
        size_t kn = strlen(k);
        while (kn && (k[kn - 1] == ' ' || k[kn - 1] == '\t')) k[--kn] = 0;
        while (*v == ' ' || *v == '\t') v++;
        size_t vn = strlen(v);
        while (vn && (v[vn - 1] == '\n' || v[vn - 1] == '\r' || v[vn - 1] == ' ' || v[vn - 1] == '\t')) {
            v[--vn] = 0;
        }
        if ((vn >= 2) && ((v[0] == '"' && v[vn - 1] == '"') || (v[0] == '\'' && v[vn - 1] == '\''))) {
            v[vn - 1] = 0;
            v++;
        }
        if (!k[0] || !v[0]) continue;
        pg_putenv(k, v);
    }
    fclose(f);
}

static void pg_load_host_env(void) {
    const char *have = getenv("BURAAQ_DATABASE_URL");
    if (have && have[0]) return;
    const char *home = getenv("HOME");
#ifdef _WIN32
    if (!home || !home[0]) home = getenv("USERPROFILE");
#endif
    char path[1024];
    if (home && home[0]) {
        snprintf(path, sizeof(path), "%s/.buraaq/dock/env", home);
        pg_load_env_file(path);
    }
    pg_load_env_file("forge.env");
}

static void pg_tune_conninfo(const char *in, char *out, size_t n) {
    if (!in) in = "";
    snprintf(out, n, "%s", in);
    int neon = strstr(out, "neon.tech") || strstr(out, "neon.build");
    int uri = strstr(out, "://") != NULL;
    if (neon && !strstr(out, "sslmode=")) {
        if (uri) strncat(out, strchr(out, '?') ? "&sslmode=require" : "?sslmode=require", n - strlen(out) - 1);
        else strncat(out, " sslmode=require", n - strlen(out) - 1);
    }
    if (!strstr(out, "channel_binding") && getenv("BURAAQ_PG_CHANNEL_BINDING") &&
        strstr(getenv("BURAAQ_PG_CHANNEL_BINDING"), "disable")) {
        if (uri) strncat(out, strchr(out, '?') ? "&channel_binding=disable" : "?channel_binding=disable", n - strlen(out) - 1);
        else strncat(out, " channel_binding=disable", n - strlen(out) - 1);
    }
}

static int pg_alive(void) {
    return g_pg && g_pq.PQstatus && g_pq.PQstatus(g_pg) == 0;
}

static void pg_close(void) {
    if (g_pg && g_pq.PQfinish) g_pq.PQfinish(g_pg);
    g_pg = NULL;
}

int32_t buraaq_pg_connect(const char *conninfo) {
    pg_load_host_env();
    if (!pq_load()) {
#ifdef _WIN32
        fprintf(stderr, "buraaq postgres: libpq.dll not found\n");
        pg_set_err("libpq not found");
#else
        fprintf(stderr, "buraaq postgres: libpq.so not found (install libpq5)\n");
        pg_set_err("libpq not found");
#endif
        return 0;
    }
    const char *cands[8];
    int n = 0;
    int have_explicit = 0;
    if (conninfo && conninfo[0]) {
        cands[n++] = conninfo;
        have_explicit = 1;
    }
    const char *e1 = getenv("BURAAQ_DATABASE_URL");
    const char *e2 = getenv("DATABASE_URL");
    if (e1 && e1[0]) {
        cands[n++] = e1;
        have_explicit = 1;
    }
    if (e2 && e2[0]) {
        cands[n++] = e2;
        have_explicit = 1;
    }
    static char sspi[512];
    static char local[512];
    if (!have_explicit) {
#ifdef _WIN32
        const char *user = getenv("USERNAME");
        if (user && user[0]) {
            snprintf(sspi, sizeof(sspi), "host=localhost port=5432 dbname=buraaq_play user=%s", user);
            cands[n++] = sspi;
        }
        snprintf(local, sizeof(local), "host=127.0.0.1 port=5432 dbname=buraaq_play user=postgres");
        cands[n++] = local;
#else
        const char *want_local = getenv("BURAAQ_PG_LOCAL");
        if (want_local && want_local[0] == '1') {
            snprintf(local, sizeof(local), "host=127.0.0.1 port=5432 dbname=buraaq_play user=postgres");
            cands[n++] = local;
        }
#endif
    }
    if (n == 0) {
        pg_set_err("BURAAQ_DATABASE_URL unset");
        fprintf(stderr, "buraaq postgres: BURAAQ_DATABASE_URL unset\n");
        return 0;
    }
    static char tuned[2048];
    for (int i = 0; i < n; i++) {
        pg_close();
        pg_tune_conninfo(cands[i], tuned, sizeof(tuned));
        g_pg = g_pq.PQconnectdb(tuned);
        if (pg_alive()) {
            snprintf(g_pg_last, sizeof(g_pg_last), "%s", tuned);
            pg_set_err("");
            return 1;
        }
        if (g_pg) {
            const char *em = g_pq.PQerrorMessage ? g_pq.PQerrorMessage(g_pg) : "";
            fprintf(stderr, "buraaq postgres: connection failed\n");
            pg_set_err(em && em[0] ? "connection failed" : "connection failed");
            pg_close();
        }
    }
    if (!g_pg_err[0]) pg_set_err("connection failed");
    return 0;
}

int32_t buraaq_pg_ok(void) { return pg_alive() ? 1 : 0; }

char *buraaq_pg_quote(const char *s) {
    if (!pq_load() || !g_pg) return dup_str("''");
    if (!s) s = "";
    char *q = g_pq.PQescapeLiteral(g_pg, s, strlen(s));
    if (!q) return dup_str("''");
    char *out = dup_str(q);
    g_pq.PQfreemem(q);
    return out;
}

char *buraaq_pg_exec(const char *sql) {
    size_t len = 0, cap = 256;
    char *buf = (char *)malloc(cap);
    if (!buf) return dup_empty();
    buf[0] = 0;
    if (!pq_load()) {
        json_append(&buf, &len, &cap, "{\"ok\":false,\"error\":\"libpq not found\",\"rows\":[]}");
        return buf;
    }
    if (!pg_alive()) {
        buraaq_pg_connect(g_pg_last);
    }
    if (!pg_alive()) {
        json_append(&buf, &len, &cap, "{\"ok\":false,\"error\":");
        json_escape_append(&buf, &len, &cap, g_pg_err[0] ? g_pg_err : "not connected");
        json_append(&buf, &len, &cap, ",\"rows\":[]}");
        return buf;
    }
    if (!sql) sql = "";
    void *res = g_pq.PQexec(g_pg, sql);
    if (!res || !pg_alive()) {
        if (res && g_pq.PQclear) g_pq.PQclear(res);
        buraaq_pg_connect(g_pg_last);
        if (!pg_alive()) {
            json_append(&buf, &len, &cap, "{\"ok\":false,\"error\":");
            json_escape_append(&buf, &len, &cap, g_pg_err[0] ? g_pg_err : "connection lost");
            json_append(&buf, &len, &cap, ",\"rows\":[]}");
            return buf;
        }
        res = g_pq.PQexec(g_pg, sql);
    }
    if (!res) {
        json_append(&buf, &len, &cap, "{\"ok\":false,\"error\":\"exec failed\",\"rows\":[]}");
        return buf;
    }
    int st = g_pq.PQresultStatus(res);
    /* PGRES_COMMAND_OK=1, PGRES_TUPLES_OK=2 */
    if (st != 1 && st != 2) {
        json_append(&buf, &len, &cap, "{\"ok\":false,\"error\":");
        json_escape_append(&buf, &len, &cap, g_pq.PQerrorMessage(g_pg));
        json_append(&buf, &len, &cap, ",\"rows\":[]}");
        g_pq.PQclear(res);
        return buf;
    }
    json_append(&buf, &len, &cap, "{\"ok\":true,\"error\":\"\",\"rows\":[");
    int rows = g_pq.PQntuples(res);
    int cols = g_pq.PQnfields(res);
    for (int r = 0; r < rows; r++) {
        if (r) json_append(&buf, &len, &cap, ",");
        json_append(&buf, &len, &cap, "{");
        for (int c = 0; c < cols; c++) {
            if (c) json_append(&buf, &len, &cap, ",");
            json_escape_append(&buf, &len, &cap, g_pq.PQfname(res, c));
            json_append(&buf, &len, &cap, ":");
            json_escape_append(&buf, &len, &cap, g_pq.PQgetvalue(res, r, c));
        }
        json_append(&buf, &len, &cap, "}");
    }
    json_append(&buf, &len, &cap, "]}");
    g_pq.PQclear(res);
    return buf;
}

void buraaq_pg_close(void) {
    if (g_pg && g_pq.PQfinish) g_pq.PQfinish(g_pg);
    g_pg = NULL;
}

/* ---------- std.service: TLS APIs, pages, REST tables ---------- */

#define BQ_SVC_MAX_PAGE 32
#define BQ_SVC_MAX_API 16
#define BQ_SVC_MAX_FIELD 8

typedef struct {
    char path[256];
    char file[512];
} SvcPage;

typedef struct {
    char name[64];
    char fields[BQ_SVC_MAX_FIELD][64];
    int nfields;
} SvcApi;

static SvcPage g_pages[BQ_SVC_MAX_PAGE];
static int g_npages = 0;
static SvcApi g_apis[BQ_SVC_MAX_API];
static int g_napis = 0;
static char g_store_url[1024];
static int g_store_set = 0;

static int path_is_safe_file(const char *s) {
    if (!s || !s[0]) return 0;
    if (s[0] == '/' || s[0] == '\\') return 0;
#ifdef _WIN32
    if (((s[0] >= 'A' && s[0] <= 'Z') || (s[0] >= 'a' && s[0] <= 'z')) && s[1] == ':') return 0;
#endif
    if (strstr(s, "..") != NULL) return 0;
    return 1;
}

static int env_int(const char *name, int fallback) {
    const char *v = getenv(name);
    if (!v || !v[0]) return fallback;
    int n = atoi(v);
    return n > 0 && n < 65536 ? n : fallback;
}

static int svc_random_key(char *out, size_t cap) {
    unsigned char raw[16];
    memset(raw, 0, sizeof(raw));
    int got = 0;
#ifdef _WIN32
    typedef unsigned char (WINAPI *RtlGenRandomFn)(void *, unsigned long);
    HMODULE adv = LoadLibraryA("advapi32.dll");
    if (adv) {
        RtlGenRandomFn fn = (RtlGenRandomFn)GetProcAddress(adv, "SystemFunction036");
        if (fn && fn(raw, (ULONG)sizeof(raw))) got = 1;
        FreeLibrary(adv);
    }
#else
    FILE *ur = fopen("/dev/urandom", "rb");
    if (ur) {
        size_t n = fread(raw, 1, sizeof(raw), ur);
        fclose(ur);
        if (n == sizeof(raw)) got = 1;
    }
#endif
    if (!got) {
        out[0] = 0;
        return 0;
    }
    {
        int allz = 1;
        size_t i;
        for (i = 0; i < sizeof(raw); i++) if (raw[i]) allz = 0;
        if (allz) {
            out[0] = 0;
            return 0;
        }
    }
    static const char *hexd = "0123456789abcdef";
    size_t i;
    for (i = 0; i < sizeof(raw) && (i * 2 + 1) < cap; i++) {
        out[i * 2] = hexd[(raw[i] >> 4) & 0xf];
        out[i * 2 + 1] = hexd[raw[i] & 0xf];
    }
    out[i * 2] = 0;
    return 1;
}

static void svc_ensure_api_key(void) {
    if (g_api_key[0]) return;
    const char *env = getenv("BURAAQ_API_KEY");
    if (env && env[0]) {
        strncpy(g_api_key, env, sizeof(g_api_key) - 1);
        return;
    }
#ifdef _WIN32
    _mkdir(".buraaq");
#else
    mkdir(".buraaq", 0700);
#endif
    FILE *f = fopen(".buraaq/api.key", "rb");
    if (f) {
        size_t n = fread(g_api_key, 1, sizeof(g_api_key) - 1, f);
        fclose(f);
        while (n > 0 && (g_api_key[n - 1] == '\n' || g_api_key[n - 1] == '\r' || g_api_key[n - 1] == ' ')) {
            g_api_key[--n] = 0;
        }
        if (g_api_key[0]) return;
    }
    if (!svc_random_key(g_api_key, sizeof(g_api_key))) {
        fprintf(stderr, "buraaq service: cannot generate API key (RNG failed)\n");
        g_api_key[0] = 0;
        return;
    }
    f = fopen(".buraaq/api.key", "wb");
    if (f) {
        fputs(g_api_key, f);
        fputc('\n', f);
        fclose(f);
#ifdef _WIN32
        _chmod(".buraaq/api.key", _S_IREAD | _S_IWRITE);
#else
        chmod(".buraaq/api.key", 0600);
#endif
    }
}

static int svc_ensure_tls_files(const char *cert, const char *key) {
    FILE *fc = fopen(cert, "rb");
    FILE *fk = fopen(key, "rb");
    if (fc && fk) {
        fclose(fc);
        fclose(fk);
        return 1;
    }
    if (fc) fclose(fc);
    if (fk) fclose(fk);
    const char *openssl = "openssl";
#ifdef _WIN32
    if (GetFileAttributesA("C:\\Program Files\\Git\\usr\\bin\\openssl.exe") != INVALID_FILE_ATTRIBUTES) {
        openssl = "C:\\Program Files\\Git\\usr\\bin\\openssl.exe";
    }
#endif
    if (!path_is_safe_file(cert) || !path_is_safe_file(key)) return 0;
#ifdef _WIN32
    {
        const char *args[] = {
            openssl, "req", "-x509", "-newkey", "rsa:2048",
            "-keyout", key, "-out", cert, "-days", "365", "-nodes",
            "-subj", "/CN=localhost", NULL
        };
        if (_spawnvp(_P_WAIT, openssl, args) != 0) return 0;
    }
#else
    {
        pid_t pid = fork();
        if (pid == 0) {
            execlp("openssl", "openssl", "req", "-x509", "-newkey", "rsa:2048",
                   "-keyout", key, "-out", cert, "-days", "365", "-nodes",
                   "-subj", "/CN=localhost", (char *)NULL);
            _exit(127);
        }
        if (pid < 0) return 0;
        int st = 0;
        if (waitpid(pid, &st, 0) < 0 || !WIFEXITED(st) || WEXITSTATUS(st) != 0) return 0;
    }
#endif
    fc = fopen(cert, "rb");
    fk = fopen(key, "rb");
    if (fc && fk) {
        fclose(fc);
        fclose(fk);
        fprintf(stdout, "generated %s and %s\n", cert, key);
        return 1;
    }
    if (fc) fclose(fc);
    if (fk) fclose(fk);
    return 0;
}

static int svc_self_origin(const char *origin) {
    if (!origin || !origin[0]) return 0;
    char a[64], b[64], c[64], d[64];
    snprintf(a, sizeof(a), "http://127.0.0.1:%d", g_http_port);
    snprintf(b, sizeof(b), "https://127.0.0.1:%d", g_tls_port);
    snprintf(c, sizeof(c), "http://localhost:%d", g_http_port);
    snprintf(d, sizeof(d), "https://localhost:%d", g_tls_port);
    return strcmp(origin, a) == 0 || strcmp(origin, b) == 0 || strcmp(origin, c) == 0
        || strcmp(origin, d) == 0;
}

static int svc_write_allowed(Conn *c) {
    if (!g_api_key[0]) return 1;
    if (g_api_lock) return ct_eq_str(c->apikey, g_api_key);
    /* Same-origin pages on loopback may write without a header. Origin is never auth from the network. */
    if (c->peer_loopback && svc_self_origin(c->origin)) return 1;
    return ct_eq_str(c->apikey, g_api_key);
}

static int svc_ident(const char *s) {
    if (!s || !*s) return 0;
    unsigned char c = (unsigned char)*s;
    if (!(c == '_' || (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z'))) return 0;
    for (s++; *s; s++) {
        c = (unsigned char)*s;
        if (!(c == '_' || (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9'))) return 0;
    }
    return 1;
}

static const char *svc_ctype(const char *path) {
    const char *dot = strrchr(path ? path : "", '.');
    if (!dot) return "application/octet-stream";
    if (strcmp(dot, ".html") == 0 || strcmp(dot, ".htm") == 0) return "text/html; charset=utf-8";
    if (strcmp(dot, ".css") == 0) return "text/css; charset=utf-8";
    if (strcmp(dot, ".js") == 0) return "text/javascript; charset=utf-8";
    if (strcmp(dot, ".json") == 0) return "application/json";
    if (strcmp(dot, ".svg") == 0) return "image/svg+xml";
    if (strcmp(dot, ".txt") == 0) return "text/plain; charset=utf-8";
    return "application/octet-stream";
}

static void svc_sql_fields(const SvcApi *a, char *out, size_t cap, int for_select) {
    out[0] = 0;
    if (for_select) strncat(out, "id::text AS id", cap - 1);
    for (int i = 0; i < a->nfields; i++) {
        if (out[0]) strncat(out, ", ", cap - strlen(out) - 1);
        strncat(out, a->fields[i], cap - strlen(out) - 1);
    }
    if (for_select) strncat(out, ", created_at::text AS created_at", cap - strlen(out) - 1);
}

static void svc_schema(const SvcApi *a) {
    char sql[2048];
    snprintf(sql, sizeof(sql), "CREATE TABLE IF NOT EXISTS %s (id SERIAL PRIMARY KEY", a->name);
    for (int i = 0; i < a->nfields; i++) {
        strncat(sql, ", ", sizeof(sql) - strlen(sql) - 1);
        strncat(sql, a->fields[i], sizeof(sql) - strlen(sql) - 1);
        strncat(sql, " TEXT NOT NULL DEFAULT ''", sizeof(sql) - strlen(sql) - 1);
    }
    strncat(sql, ", created_at TIMESTAMPTZ NOT NULL DEFAULT now())", sizeof(sql) - strlen(sql) - 1);
    char *res = buraaq_pg_exec(sql);
    free(res);
}

static int svc_match_item(const char *path, const char *name, int *id) {
    char prefix[96];
    snprintf(prefix, sizeof(prefix), "/api/%s", name);
    if (strcmp(path, prefix) == 0) {
        *id = -1;
        return 0;
    }
    size_t n = strlen(prefix);
    if (strncmp(path, prefix, n) != 0 || path[n] != '/' || !path[n + 1]) return -1;
    const char *rest = path + n + 1;
    if (*rest < '0' || *rest > '9') return -1;
    *id = atoi(rest);
    while (*rest >= '0' && *rest <= '9') rest++;
    return *rest == 0 ? 1 : -1;
}

static void svc_reply_sql(int32_t conn, int status, const char *sql) {
    char *j = buraaq_pg_exec(sql);
    buraaq_http_reply(conn, status, "application/json", j ? j : "{\"ok\":false,\"rows\":[]}");
    free(j);
}

static void svc_collection(int32_t conn, const SvcApi *a, const char *method) {
    char cols[512];
    svc_sql_fields(a, cols, sizeof(cols), 1);
    if (strcmp(method, "GET") == 0) {
        char sql[1024];
        snprintf(sql, sizeof(sql), "SELECT %s FROM %s ORDER BY id", cols, a->name);
        svc_reply_sql(conn, 200, sql);
        return;
    }
    if (strcmp(method, "POST") == 0) {
        char sql[4096];
        snprintf(sql, sizeof(sql), "INSERT INTO %s (", a->name);
        for (int i = 0; i < a->nfields; i++) {
            if (i) strncat(sql, ", ", sizeof(sql) - strlen(sql) - 1);
            strncat(sql, a->fields[i], sizeof(sql) - strlen(sql) - 1);
        }
        strncat(sql, ") VALUES (", sizeof(sql) - strlen(sql) - 1);
        for (int i = 0; i < a->nfields; i++) {
            if (i) strncat(sql, ", ", sizeof(sql) - strlen(sql) - 1);
            char *raw = buraaq_http_json_field(conn, a->fields[i]);
            char *q = buraaq_pg_quote(raw ? raw : "");
            strncat(sql, q ? q : "''", sizeof(sql) - strlen(sql) - 1);
            free(raw);
            free(q);
        }
        strncat(sql, ") RETURNING ", sizeof(sql) - strlen(sql) - 1);
        strncat(sql, cols, sizeof(sql) - strlen(sql) - 1);
        svc_reply_sql(conn, 201, sql);
        return;
    }
    buraaq_http_reply(conn, 405, "text/plain; charset=utf-8", "method not allowed");
}

static void svc_item(int32_t conn, const SvcApi *a, const char *method, int id) {
    char cols[512];
    svc_sql_fields(a, cols, sizeof(cols), 1);
    if (strcmp(method, "GET") == 0) {
        char sql[1024];
        snprintf(sql, sizeof(sql), "SELECT %s FROM %s WHERE id = %d", cols, a->name, id);
        svc_reply_sql(conn, 200, sql);
        return;
    }
    if (strcmp(method, "PUT") == 0) {
        char sql[4096];
        snprintf(sql, sizeof(sql), "UPDATE %s SET ", a->name);
        for (int i = 0; i < a->nfields; i++) {
            if (i) strncat(sql, ", ", sizeof(sql) - strlen(sql) - 1);
            strncat(sql, a->fields[i], sizeof(sql) - strlen(sql) - 1);
            strncat(sql, " = ", sizeof(sql) - strlen(sql) - 1);
            char *raw = buraaq_http_json_field(conn, a->fields[i]);
            char *q = buraaq_pg_quote(raw ? raw : "");
            strncat(sql, q ? q : "''", sizeof(sql) - strlen(sql) - 1);
            free(raw);
            free(q);
        }
        char where[64];
        snprintf(where, sizeof(where), " WHERE id = %d RETURNING ", id);
        strncat(sql, where, sizeof(sql) - strlen(sql) - 1);
        strncat(sql, cols, sizeof(sql) - strlen(sql) - 1);
        svc_reply_sql(conn, 200, sql);
        return;
    }
    if (strcmp(method, "DELETE") == 0) {
        char sql[256];
        snprintf(sql, sizeof(sql), "DELETE FROM %s WHERE id = %d RETURNING id::text AS id", a->name, id);
        svc_reply_sql(conn, 200, sql);
        return;
    }
    buraaq_http_reply(conn, 405, "text/plain; charset=utf-8", "method not allowed");
}

static void svc_handle(int32_t conn) {
    Conn *c = conn_at(conn);
    char *method = buraaq_http_method(conn);
    char *path = buraaq_http_path(conn);
    if (!method) method = dup_empty();
    if (!path) path = dup_empty();
    if (strcmp(method, "OPTIONS") == 0) {
        buraaq_http_reply(conn, 204, "text/plain; charset=utf-8", "");
        free(method);
        free(path);
        return;
    }
    if (strcmp(path, "/api/health") == 0) {
        svc_reply_sql(conn, 200, "SELECT 1::text AS ok");
        free(method);
        free(path);
        return;
    }
    int is_write = strcmp(method, "POST") == 0 || strcmp(method, "PUT") == 0
        || strcmp(method, "DELETE") == 0;
    int is_api = strncmp(path, "/api/", 5) == 0;
    if (is_api && (is_write || g_api_lock) && c && !svc_write_allowed(c)) {
        buraaq_http_reply(
            conn,
            401,
            "application/json",
            "{\"ok\":false,\"error\":\"unauthorized\",\"rows\":[]}"
        );
        free(method);
        free(path);
        return;
    }
    for (int i = 0; i < g_npages; i++) {
        if (strcmp(path, g_pages[i].path) == 0) {
            char *html = buraaq_file_read(g_pages[i].file);
            if (!html) {
                buraaq_http_reply(conn, 404, "text/plain; charset=utf-8", "not found");
            } else {
                buraaq_http_reply(conn, 200, svc_ctype(g_pages[i].file), html);
                free(html);
            }
            free(method);
            free(path);
            return;
        }
    }
    for (int i = 0; i < g_napis; i++) {
        int id = -1;
        int kind = svc_match_item(path, g_apis[i].name, &id);
        if (kind == 0) {
            svc_collection(conn, &g_apis[i], method);
            free(method);
            free(path);
            return;
        }
        if (kind == 1) {
            svc_item(conn, &g_apis[i], method, id);
            free(method);
            free(path);
            return;
        }
    }
    buraaq_http_reply(conn, 404, "text/plain; charset=utf-8", "not found");
    free(method);
    free(path);
}

int32_t buraaq_svc_page(const char *path, const char *file) {
    if (!path || !file || g_npages >= BQ_SVC_MAX_PAGE) return 0;
    if (path_is_safe_file(file) == 0 || strstr(path ? path : "", "..") != NULL) return 0;
    SvcPage *p = &g_pages[g_npages++];
    memset(p, 0, sizeof(*p));
    strncpy(p->path, path, sizeof(p->path) - 1);
    strncpy(p->file, file, sizeof(p->file) - 1);
    return 1;
}

int32_t buraaq_svc_api(const char *name, const char *fields) {
    if (!name || !svc_ident(name) || g_napis >= BQ_SVC_MAX_API) return 0;
    SvcApi *a = &g_apis[g_napis];
    memset(a, 0, sizeof(*a));
    strncpy(a->name, name, sizeof(a->name) - 1);
    const char *s = fields ? fields : "";
    while (*s) {
        while (*s == ' ' || *s == ',') s++;
        if (!*s) break;
        char buf[64];
        int n = 0;
        while (*s && *s != ',' && *s != ' ' && n < 63) buf[n++] = *s++;
        buf[n] = 0;
        if (!svc_ident(buf) || a->nfields >= BQ_SVC_MAX_FIELD) {
            return 0;
        }
        memcpy(a->fields[a->nfields], buf, (size_t)n + 1);
        a->nfields++;
        while (*s == ' ') s++;
        if (*s == ',') s++;
    }
    if (a->nfields <= 0) return 0;
    g_napis++;
    return 1;
}

int32_t buraaq_svc_store(const char *url) {
    g_store_set = 1;
    g_store_url[0] = 0;
    if (url) strncpy(g_store_url, url, sizeof(g_store_url) - 1);
    return 1;
}

int32_t buraaq_svc_key(const char *secret) {
    g_api_key[0] = 0;
    if (secret) strncpy(g_api_key, secret, sizeof(g_api_key) - 1);
    return 1;
}

int32_t buraaq_svc_origin(const char *allowed) {
    g_cors_origin[0] = 0;
    if (allowed) strncpy(g_cors_origin, allowed, sizeof(g_cors_origin) - 1);
    return 1;
}

int32_t buraaq_svc_run(int32_t port) {
    const char *info = g_store_set ? g_store_url : "";
    if (g_napis > 0) {
        if (!buraaq_pg_connect(info)) {
            fprintf(stderr, "buraaq service: postgres not connected (set BURAAQ_DATABASE_URL)\n");
        } else {
            for (int i = 0; i < g_napis; i++) svc_schema(&g_apis[i]);
            fprintf(stdout, "postgres connected\n");
        }
    }
    const char *cors_env = getenv("BURAAQ_CORS_ORIGIN");
    if (cors_env && cors_env[0] && !g_cors_origin[0]) {
        strncpy(g_cors_origin, cors_env, sizeof(g_cors_origin) - 1);
    }
    (void)bq_listen_addr();
    g_api_lock = g_public ? 1 : 0;
    g_api_lock = env_flag_default("BURAAQ_API_LOCK", g_api_lock);
    svc_ensure_api_key();
    if (g_api_key[0]) {
        fprintf(stdout, "api key file .buraaq/api.key\n");
        fprintf(stdout, "X-Api-Key required for curl writes (loopback pages are same-origin)\n");
        if (g_api_lock) fprintf(stdout, "API lock on (reads need the key too)\n");
    }
    const char *cert = getenv("BURAAQ_TLS_CERT");
    const char *keyp = getenv("BURAAQ_TLS_KEY");
    if (!cert || !cert[0]) cert = "cert.pem";
    if (!keyp || !keyp[0]) keyp = "key.pem";
    svc_ensure_tls_files(cert, keyp);
    g_tls_port = port > 0 ? port : env_int("BURAAQ_TLS_PORT", 8443);
    g_http_port = g_tls_port == 443 ? 80 : env_int("BURAAQ_HTTP_PORT", 8080);
    int https_ok = buraaq_http_listen(g_tls_port, 1);
    int want_http = !g_public || env_flag("BURAAQ_HTTP") || !https_ok;
    int http_ok = want_http ? buraaq_http_listen(g_http_port, 0) : 0;
    const char *shown = g_public ? "0.0.0.0" : "127.0.0.1";
    if (https_ok) fprintf(stdout, "https://%s:%d\n", shown, g_tls_port);
    else fprintf(stderr, "buraaq service: TLS not bound on %d (need cert.pem + key.pem)\n", g_tls_port);
    if (http_ok) fprintf(stdout, "http://%s:%d\n", shown, g_http_port);
    else if (want_http) fprintf(stderr, "buraaq service: HTTP not bound on %d\n", g_http_port);
    if (!https_ok && !http_ok) return 0;
    fprintf(stdout, "service ready\n");
    for (;;) {
        int32_t conn = buraaq_http_accept(0);
        if (conn) {
            svc_handle(conn);
            buraaq_http_close(conn);
        }
    }
}
