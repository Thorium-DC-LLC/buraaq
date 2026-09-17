/* std.stream — WebSocket framed messages. Call it stream, not websocket. */
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
typedef SOCKET bq_sock;
#define BQ_INVALID INVALID_SOCKET
#define bq_close_sock(s) closesocket(s)
#else
#include <unistd.h>
#include <errno.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <netdb.h>
typedef int bq_sock;
#define BQ_INVALID (-1)
#define bq_close_sock(s) close(s)
#define SOCKET_ERROR (-1)
#endif

#define BQ_STREAM_MAX 32
#define BQ_WS_MAX (1u << 20)
#define BQ_WS_MAGIC "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"

typedef struct {
    int in_use;
    int client;
    bq_sock fd;
} BqStream;

static BqStream g_st[BQ_STREAM_MAX];
static bq_sock g_listen = BQ_INVALID;
static int g_wsa = 0;

static char *dup_str(const char *s) {
    if (!s) s = "";
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

static int sock_init(void) {
#ifdef _WIN32
    if (!g_wsa) {
        WSADATA w;
        if (WSAStartup(MAKEWORD(2, 2), &w) != 0) return -1;
        g_wsa = 1;
    }
#endif
    return 0;
}

static void set_timeout(bq_sock fd, int ms) {
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

static int send_all(bq_sock fd, const void *buf, size_t n) {
    const unsigned char *p = (const unsigned char *)buf;
    size_t off = 0;
    while (off < n) {
#ifdef _WIN32
        int k = send(fd, (const char *)(p + off), (int)(n - off), 0);
#else
        ssize_t k = send(fd, p + off, n - off, 0);
#endif
        if (k <= 0) return -1;
        off += (size_t)k;
    }
    return 0;
}

static int recv_n(bq_sock fd, void *buf, size_t n) {
    unsigned char *p = (unsigned char *)buf;
    size_t off = 0;
    while (off < n) {
#ifdef _WIN32
        int k = recv(fd, (char *)(p + off), (int)(n - off), 0);
#else
        ssize_t k = recv(fd, p + off, n - off, 0);
#endif
        if (k <= 0) return -1;
        off += (size_t)k;
    }
    return 0;
}

/* SHA-1 (FIPS 180-1), enough for the WebSocket accept key. */
static uint32_t rol(uint32_t x, int n) { return (x << n) | (x >> (32 - n)); }

static void sha1(const unsigned char *data, size_t len, unsigned char out[20]) {
    uint32_t h0 = 0x67452301, h1 = 0xEFCDAB89, h2 = 0x98BADCFE, h3 = 0x10325476, h4 = 0xC3D2E1F0;
    size_t nblocks = ((len + 8) / 64) + 1;
    unsigned char *msg = (unsigned char *)calloc(nblocks, 64);
    if (!msg) return;
    memcpy(msg, data, len);
    msg[len] = 0x80;
    uint64_t bits = (uint64_t)len * 8;
    msg[nblocks * 64 - 8] = (unsigned char)(bits >> 56);
    msg[nblocks * 64 - 7] = (unsigned char)(bits >> 48);
    msg[nblocks * 64 - 6] = (unsigned char)(bits >> 40);
    msg[nblocks * 64 - 5] = (unsigned char)(bits >> 32);
    msg[nblocks * 64 - 4] = (unsigned char)(bits >> 24);
    msg[nblocks * 64 - 3] = (unsigned char)(bits >> 16);
    msg[nblocks * 64 - 2] = (unsigned char)(bits >> 8);
    msg[nblocks * 64 - 1] = (unsigned char)bits;
    for (size_t blk = 0; blk < nblocks; blk++) {
        uint32_t w[80];
        for (int i = 0; i < 16; i++) {
            size_t o = blk * 64 + (size_t)i * 4;
            w[i] = ((uint32_t)msg[o] << 24) | ((uint32_t)msg[o + 1] << 16) |
                   ((uint32_t)msg[o + 2] << 8) | (uint32_t)msg[o + 3];
        }
        for (int i = 16; i < 80; i++) w[i] = rol(w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16], 1);
        uint32_t a = h0, b = h1, c = h2, d = h3, e = h4;
        for (int i = 0; i < 80; i++) {
            uint32_t f, k;
            if (i < 20) {
                f = (b & c) | ((~b) & d);
                k = 0x5A827999;
            } else if (i < 40) {
                f = b ^ c ^ d;
                k = 0x6ED9EBA1;
            } else if (i < 60) {
                f = (b & c) | (b & d) | (c & d);
                k = 0x8F1BBCDC;
            } else {
                f = b ^ c ^ d;
                k = 0xCA62C1D6;
            }
            uint32_t temp = rol(a, 5) + f + e + k + w[i];
            e = d;
            d = c;
            c = rol(b, 30);
            b = a;
            a = temp;
        }
        h0 += a;
        h1 += b;
        h2 += c;
        h3 += d;
        h4 += e;
    }
    free(msg);
    uint32_t hs[5] = {h0, h1, h2, h3, h4};
    for (int i = 0; i < 5; i++) {
        out[i * 4] = (unsigned char)(hs[i] >> 24);
        out[i * 4 + 1] = (unsigned char)(hs[i] >> 16);
        out[i * 4 + 2] = (unsigned char)(hs[i] >> 8);
        out[i * 4 + 3] = (unsigned char)hs[i];
    }
}

static void b64(const unsigned char *in, size_t n, char *out) {
    static const char T[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    size_t o = 0;
    for (size_t i = 0; i < n; i += 3) {
        unsigned int v = (unsigned int)in[i] << 16;
        if (i + 1 < n) v |= (unsigned int)in[i + 1] << 8;
        if (i + 2 < n) v |= (unsigned int)in[i + 2];
        out[o++] = T[(v >> 18) & 63];
        out[o++] = T[(v >> 12) & 63];
        out[o++] = (i + 1 < n) ? T[(v >> 6) & 63] : '=';
        out[o++] = (i + 2 < n) ? T[v & 63] : '=';
    }
    out[o] = 0;
}

static void ws_accept_key(const char *key, char *out, size_t cap) {
    char buf[128];
    snprintf(buf, sizeof(buf), "%s%s", key ? key : "", BQ_WS_MAGIC);
    unsigned char dig[20];
    sha1((const unsigned char *)buf, strlen(buf), dig);
    char enc[40];
    b64(dig, 20, enc);
    snprintf(out, cap, "%s", enc);
}

static int32_t stream_slot(bq_sock fd, int client) {
    for (int32_t i = 0; i < BQ_STREAM_MAX; i++) {
        if (g_st[i].in_use) continue;
        g_st[i].in_use = 1;
        g_st[i].client = client;
        g_st[i].fd = fd;
        return i;
    }
    bq_close_sock(fd);
    return -1;
}

static BqStream *st_get(int32_t id) {
    if (id < 0 || id >= BQ_STREAM_MAX || !g_st[id].in_use) return NULL;
    return &g_st[id];
}

static int send_frame(bq_sock fd, int opcode, const unsigned char *data, size_t len, int mask) {
    unsigned char hdr[14];
    size_t hlen = 2;
    hdr[0] = (unsigned char)(0x80 | (opcode & 0x0f));
    if (len < 126) {
        hdr[1] = (unsigned char)((mask ? 0x80 : 0) | len);
    } else if (len < 65536) {
        hdr[1] = (unsigned char)((mask ? 0x80 : 0) | 126);
        hdr[2] = (unsigned char)((len >> 8) & 0xff);
        hdr[3] = (unsigned char)(len & 0xff);
        hlen = 4;
    } else {
        hdr[1] = (unsigned char)((mask ? 0x80 : 0) | 127);
        memset(hdr + 2, 0, 8);
        hdr[6] = (unsigned char)((len >> 24) & 0xff);
        hdr[7] = (unsigned char)((len >> 16) & 0xff);
        hdr[8] = (unsigned char)((len >> 8) & 0xff);
        hdr[9] = (unsigned char)(len & 0xff);
        hlen = 10;
    }
    unsigned char mkey[4] = {0, 0, 0, 0};
    if (mask) {
        mkey[0] = (unsigned char)(rand() & 0xff);
        mkey[1] = (unsigned char)(rand() & 0xff);
        mkey[2] = (unsigned char)(rand() & 0xff);
        mkey[3] = (unsigned char)(rand() & 0xff);
    }
    if (send_all(fd, hdr, hlen) != 0) return -1;
    if (mask && send_all(fd, mkey, 4) != 0) return -1;
    if (len == 0) return 0;
    unsigned char *tmp = (unsigned char *)malloc(len);
    if (!tmp) return -1;
    memcpy(tmp, data, len);
    if (mask) {
        for (size_t i = 0; i < len; i++) tmp[i] ^= mkey[i % 4];
    }
    int rc = send_all(fd, tmp, len);
    free(tmp);
    return rc;
}

static int recv_frame(bq_sock fd, int *opcode, unsigned char **payload, size_t *plen) {
    unsigned char hdr[2];
    if (recv_n(fd, hdr, 2) != 0) return -1;
    *opcode = hdr[0] & 0x0f;
    int masked = (hdr[1] & 0x80) != 0;
    uint64_t len = (uint64_t)(hdr[1] & 0x7f);
    if (len == 126) {
        unsigned char e[2];
        if (recv_n(fd, e, 2) != 0) return -1;
        len = ((uint64_t)e[0] << 8) | (uint64_t)e[1];
    } else if (len == 127) {
        unsigned char e[8];
        if (recv_n(fd, e, 8) != 0) return -1;
        len = 0;
        for (int i = 0; i < 8; i++) len = (len << 8) | e[i];
    }
    if (len > BQ_WS_MAX) return -1;
    unsigned char mkey[4] = {0};
    if (masked && recv_n(fd, mkey, 4) != 0) return -1;
    unsigned char *buf = (unsigned char *)malloc((size_t)len + 1);
    if (!buf) return -1;
    if (len && recv_n(fd, buf, (size_t)len) != 0) {
        free(buf);
        return -1;
    }
    if (masked) {
        for (uint64_t i = 0; i < len; i++) buf[i] ^= mkey[i % 4];
    }
    buf[len] = 0;
    *payload = buf;
    *plen = (size_t)len;
    return 0;
}

static void hdr_get(const char *headers, const char *key, char *out, size_t cap) {
    out[0] = 0;
    if (!headers || !key) return;
    size_t klen = strlen(key);
    const char *p = headers;
    while (*p) {
        const char *eol = strstr(p, "\r\n");
        size_t n = eol ? (size_t)(eol - p) : strlen(p);
        int match = 1;
        if (n <= klen) match = 0;
        else {
            for (size_t i = 0; i < klen; i++) {
                char a = p[i], b = key[i];
                if (a >= 'A' && a <= 'Z') a = (char)(a - 'A' + 'a');
                if (b >= 'A' && b <= 'Z') b = (char)(b - 'A' + 'a');
                if (a != b) {
                    match = 0;
                    break;
                }
            }
            if (match && p[klen] != ':') match = 0;
        }
        if (match) {
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

int32_t buraaq_stream_bind(int32_t port) {
    if (sock_init() != 0) return 0;
    if (g_listen != BQ_INVALID) {
        bq_close_sock(g_listen);
        g_listen = BQ_INVALID;
    }
    bq_sock fd = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (fd == BQ_INVALID) return 0;
    int on = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, (const char *)&on, sizeof(on));
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    /* Loopback unless BURAAQ_PUBLIC=1 (same policy as Keel). */
    {
        const char *pub = getenv("BURAAQ_PUBLIC");
        int any = pub && (pub[0] == '1' || pub[0] == 'y' || pub[0] == 'Y' || pub[0] == 't' || pub[0] == 'T');
        addr.sin_addr.s_addr = htonl(any ? INADDR_ANY : INADDR_LOOPBACK);
    }
    addr.sin_port = htons((unsigned short)port);
    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) == SOCKET_ERROR) {
        bq_close_sock(fd);
        return 0;
    }
    if (listen(fd, 32) == SOCKET_ERROR) {
        bq_close_sock(fd);
        return 0;
    }
    g_listen = fd;
    return 1;
}

static int handshake_server(bq_sock fd) {
    char req[8192];
    size_t n = 0;
    while (n + 1 < sizeof(req)) {
#ifdef _WIN32
        int k = recv(fd, req + n, (int)(sizeof(req) - 1 - n), 0);
#else
        ssize_t k = recv(fd, req + n, sizeof(req) - 1 - n, 0);
#endif
        if (k <= 0) return -1;
        n += (size_t)k;
        req[n] = 0;
        if (strstr(req, "\r\n\r\n")) break;
    }
    char key[128];
    hdr_get(req, "sec-websocket-key", key, sizeof(key));
    if (!key[0]) return -1;
    char accept[64];
    ws_accept_key(key, accept, sizeof(accept));
    char resp[256];
    int m = snprintf(
        resp,
        sizeof(resp),
        "HTTP/1.1 101 Switching Protocols\r\n"
        "Upgrade: websocket\r\n"
        "Connection: Upgrade\r\n"
        "Sec-WebSocket-Accept: %s\r\n"
        "\r\n",
        accept);
    if (m < 0) return -1;
    return send_all(fd, resp, (size_t)m);
}

int32_t buraaq_stream_hail(void) {
    if (g_listen == BQ_INVALID) return -1;
    bq_sock cfd = accept(g_listen, NULL, NULL);
    if (cfd == BQ_INVALID) return -1;
    set_timeout(cfd, 10000);
    if (handshake_server(cfd) != 0) {
        bq_close_sock(cfd);
        return -1;
    }
    return stream_slot(cfd, 0);
}

static int parse_ws_url(const char *url, char *host, size_t hcap, int *port, char *path, size_t pcap) {
    if (!url) return -1;
    const char *p = url;
    if (strncmp(p, "ws://", 5) == 0) p += 5;
    else if (strncmp(p, "http://", 7) == 0) p += 7;
    const char *slash = strchr(p, '/');
    const char *hostend = slash ? slash : p + strlen(p);
    const char *colon = memchr(p, ':', (size_t)(hostend - p));
    size_t hn = (size_t)((colon ? colon : hostend) - p);
    if (hn == 0 || hn >= hcap) return -1;
    memcpy(host, p, hn);
    host[hn] = 0;
    *port = 80;
    if (colon) *port = atoi(colon + 1);
    snprintf(path, pcap, "%s", slash ? slash : "/");
    return 0;
}

int32_t buraaq_stream_wire(const char *url) {
    if (sock_init() != 0) return -1;
    char host[256], path[512];
    int port = 80;
    if (parse_ws_url(url, host, sizeof(host), &port, path, sizeof(path)) != 0) return -1;
    char port_s[16];
    snprintf(port_s, sizeof(port_s), "%d", port);
    struct addrinfo hints, *res = NULL;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    if (getaddrinfo(host, port_s, &hints, &res) != 0 || !res) return -1;
    bq_sock fd = socket(res->ai_family, res->ai_socktype, res->ai_protocol);
    if (fd == BQ_INVALID) {
        freeaddrinfo(res);
        return -1;
    }
    if (connect(fd, res->ai_addr, (int)res->ai_addrlen) == SOCKET_ERROR) {
        bq_close_sock(fd);
        freeaddrinfo(res);
        return -1;
    }
    freeaddrinfo(res);
    set_timeout(fd, 10000);
    const char *key = "dGhlIHNhbXBsZSBub25jZQ==";
    char req[1024];
    int m = snprintf(
        req,
        sizeof(req),
        "GET %s HTTP/1.1\r\n"
        "Host: %s:%d\r\n"
        "Upgrade: websocket\r\n"
        "Connection: Upgrade\r\n"
        "Sec-WebSocket-Key: %s\r\n"
        "Sec-WebSocket-Version: 13\r\n"
        "\r\n",
        path,
        host,
        port,
        key);
    if (m < 0 || send_all(fd, req, (size_t)m) != 0) {
        bq_close_sock(fd);
        return -1;
    }
    char resp[2048];
    size_t n = 0;
    while (n + 1 < sizeof(resp)) {
#ifdef _WIN32
        int k = recv(fd, resp + n, (int)(sizeof(resp) - 1 - n), 0);
#else
        ssize_t k = recv(fd, resp + n, sizeof(resp) - 1 - n, 0);
#endif
        if (k <= 0) {
            bq_close_sock(fd);
            return -1;
        }
        n += (size_t)k;
        resp[n] = 0;
        if (strstr(resp, "\r\n\r\n")) break;
    }
    if (!strstr(resp, "101")) {
        bq_close_sock(fd);
        return -1;
    }
    return stream_slot(fd, 1);
}

int32_t buraaq_stream_say(int32_t id, const char *msg) {
    BqStream *s = st_get(id);
    if (!s) return 0;
    if (!msg) msg = "";
    int rc = send_frame(s->fd, 1, (const unsigned char *)msg, strlen(msg), s->client);
    return rc == 0 ? 1 : 0;
}

char *buraaq_stream_hear(int32_t id) {
    BqStream *s = st_get(id);
    if (!s) return dup_str("");
    for (;;) {
        int opcode = 0;
        unsigned char *payload = NULL;
        size_t plen = 0;
        if (recv_frame(s->fd, &opcode, &payload, &plen) != 0) return dup_str("");
        if (opcode == 8) {
            free(payload);
            return dup_str("");
        }
        if (opcode == 9) {
            send_frame(s->fd, 10, payload, plen, s->client);
            free(payload);
            continue;
        }
        if (opcode == 10) {
            free(payload);
            continue;
        }
        if (opcode == 1 || opcode == 2) {
            char *out = (char *)malloc(plen + 1);
            if (!out) {
                free(payload);
                return dup_str("");
            }
            memcpy(out, payload, plen);
            out[plen] = 0;
            free(payload);
            return out;
        }
        free(payload);
    }
}

void buraaq_stream_hangup(int32_t id) {
    BqStream *s = st_get(id);
    if (!s) return;
    send_frame(s->fd, 8, NULL, 0, s->client);
    bq_close_sock(s->fd);
    s->in_use = 0;
    s->fd = BQ_INVALID;
}

void buraaq_stream_run(void) {
    for (;;) {
        int32_t id = buraaq_stream_hail();
        if (id < 0) continue;
        for (;;) {
            char *msg = buraaq_stream_hear(id);
            if (!msg || !msg[0]) {
                free(msg);
                break;
            }
            buraaq_stream_say(id, msg);
            free(msg);
        }
        buraaq_stream_hangup(id);
    }
}
