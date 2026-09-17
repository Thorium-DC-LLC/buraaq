/* Buraaq AI — std.ai client (HTTP to local OpenAI-compatible serve). */
#include "buraaq_std.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <wininet.h>
#pragma comment(lib, "wininet.lib")
#else
#include <unistd.h>
#include <stdint.h>
#include <sys/socket.h>
#include <netdb.h>
#include <arpa/inet.h>
#endif

#define AI_MAX_MODELS 32
#define AI_NAME_MAX 256
#define AI_SYS_MAX 4096

typedef struct {
    char name[AI_NAME_MAX];
    char system[AI_SYS_MAX];
    int used;
} AiModel;

static AiModel g_models[AI_MAX_MODELS];
static int g_next = 1;

static char *dup_str(const char *s) {
    if (!s) s = "";
    size_t n = strlen(s);
    char *o = (char *)malloc(n + 1);
    if (!o) return NULL;
    memcpy(o, s, n + 1);
    return o;
}

static const char *ai_base_url(void) {
    const char *e = getenv("BURAAQ_AI_BASE_URL");
    if (e && e[0]) return e;
    return "http://127.0.0.1:8000";
}

static const char *ai_api_key(void) {
    const char *e = getenv("BURAAQ_AI_KEY");
    return (e && e[0]) ? e : NULL;
}

/* Very small JSON string escape into out (must be large enough). */
static void json_escape(const char *in, char *out, size_t out_cap) {
    size_t j = 0;
    if (!in) in = "";
    for (size_t i = 0; in[i] && j + 2 < out_cap; i++) {
        unsigned char c = (unsigned char)in[i];
        if (c == '"' || c == '\\') {
            if (j + 3 >= out_cap) break;
            out[j++] = '\\';
            out[j++] = (char)c;
        } else if (c == '\n') {
            if (j + 3 >= out_cap) break;
            out[j++] = '\\';
            out[j++] = 'n';
        } else if (c < 0x20) {
            continue;
        } else {
            out[j++] = (char)c;
        }
    }
    out[j] = '\0';
}

/* Extract choices[0].message.content with a naive scan. */
static char *extract_content(const char *json) {
    if (!json) return dup_str("");
    const char *key = "\"content\"";
    const char *p = strstr(json, key);
    if (!p) return dup_str(json);
    p = strchr(p + strlen(key), '"');
    if (!p) return dup_str("");
    p++;
    const char *end = p;
    while (*end) {
        if (*end == '\\' && end[1]) {
            end += 2;
            continue;
        }
        if (*end == '"') break;
        end++;
    }
    size_t n = (size_t)(end - p);
    char *out = (char *)malloc(n + 1);
    if (!out) return dup_str("");
    size_t j = 0;
    for (size_t i = 0; i < n; i++) {
        if (p[i] == '\\' && i + 1 < n) {
            char e = p[i + 1];
            if (e == 'n') {
                out[j++] = '\n';
                i++;
            } else if (e == '"' || e == '\\') {
                out[j++] = e;
                i++;
            } else {
                out[j++] = p[i];
            }
        } else {
            out[j++] = p[i];
        }
    }
    out[j] = '\0';
    return out;
}

#ifdef _WIN32
static char *http_post_json(const char *url, const char *body, const char *api_key) {
    URL_COMPONENTSA uc;
    char host[256], path[1024];
    memset(&uc, 0, sizeof(uc));
    uc.dwStructSize = sizeof(uc);
    uc.lpszHostName = host;
    uc.dwHostNameLength = sizeof(host);
    uc.lpszUrlPath = path;
    uc.dwUrlPathLength = sizeof(path);
    if (!InternetCrackUrlA(url, 0, 0, &uc)) return NULL;

    HINTERNET ses = InternetOpenA("buraaq-ai/1.0", INTERNET_OPEN_TYPE_PRECONFIG, NULL, NULL, 0);
    if (!ses) return NULL;
    INTERNET_PORT port = uc.nPort ? uc.nPort : (uc.nScheme == INTERNET_SCHEME_HTTPS ? 443 : 80);
    HINTERNET conn = InternetConnectA(ses, host, port, NULL, NULL, INTERNET_SERVICE_HTTP, 0, 0);
    if (!conn) {
        InternetCloseHandle(ses);
        return NULL;
    }
    const char *verb = "POST";
    DWORD flags = INTERNET_FLAG_RELOAD | INTERNET_FLAG_NO_CACHE_WRITE;
    if (uc.nScheme == INTERNET_SCHEME_HTTPS) flags |= INTERNET_FLAG_SECURE;
    HINTERNET req = HttpOpenRequestA(conn, verb, path, NULL, NULL, NULL, flags, 0);
    if (!req) {
        InternetCloseHandle(conn);
        InternetCloseHandle(ses);
        return NULL;
    }
    char hdr[512];
    if (api_key) {
        snprintf(hdr, sizeof(hdr),
                 "Content-Type: application/json\r\nAuthorization: Bearer %s\r\n", api_key);
    } else {
        snprintf(hdr, sizeof(hdr), "Content-Type: application/json\r\n");
    }
    BOOL ok = HttpSendRequestA(req, hdr, (DWORD)strlen(hdr), (LPVOID)body, (DWORD)strlen(body));
    if (!ok) {
        InternetCloseHandle(req);
        InternetCloseHandle(conn);
        InternetCloseHandle(ses);
        return NULL;
    }
    size_t cap = 8192, len = 0;
    char *buf = (char *)malloc(cap);
    if (!buf) {
        InternetCloseHandle(req);
        InternetCloseHandle(conn);
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
                InternetCloseHandle(conn);
                InternetCloseHandle(ses);
                return NULL;
            }
            buf = nb;
        }
    }
    buf[len] = '\0';
    InternetCloseHandle(req);
    InternetCloseHandle(conn);
    InternetCloseHandle(ses);
    return buf;
}
#else
static char *http_post_json(const char *url, const char *body, const char *api_key) {
    /* Only http://host:port/path for Phase 1. */
    if (strncmp(url, "http://", 7) != 0) return NULL;
    const char *rest = url + 7;
    char host[256];
    char path[1024];
    int port = 80;
    const char *slash = strchr(rest, '/');
    const char *colon = strchr(rest, ':');
    if (colon && (!slash || colon < slash)) {
        size_t hn = (size_t)(colon - rest);
        if (hn >= sizeof(host)) hn = sizeof(host) - 1;
        memcpy(host, rest, hn);
        host[hn] = '\0';
        port = atoi(colon + 1);
        if (slash) {
            strncpy(path, slash, sizeof(path) - 1);
            path[sizeof(path) - 1] = '\0';
        } else {
            strcpy(path, "/");
        }
    } else if (slash) {
        size_t hn = (size_t)(slash - rest);
        if (hn >= sizeof(host)) hn = sizeof(host) - 1;
        memcpy(host, rest, hn);
        host[hn] = '\0';
        strncpy(path, slash, sizeof(path) - 1);
        path[sizeof(path) - 1] = '\0';
    } else {
        strncpy(host, rest, sizeof(host) - 1);
        host[sizeof(host) - 1] = '\0';
        strcpy(path, "/");
    }

    struct hostent *he = gethostbyname(host);
    if (!he) return NULL;
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return NULL;
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)port);
    memcpy(&addr.sin_addr, he->h_addr_list[0], (size_t)he->h_length);
    if (connect(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        close(fd);
        return NULL;
    }
    char header[2048];
    int hl;
    if (api_key) {
        hl = snprintf(header, sizeof(header),
                      "POST %s HTTP/1.0\r\nHost: %s\r\nContent-Type: application/json\r\n"
                      "Authorization: Bearer %s\r\nContent-Length: %zu\r\nConnection: close\r\n\r\n",
                      path, host, api_key, strlen(body));
    } else {
        hl = snprintf(header, sizeof(header),
                      "POST %s HTTP/1.0\r\nHost: %s\r\nContent-Type: application/json\r\n"
                      "Content-Length: %zu\r\nConnection: close\r\n\r\n",
                      path, host, strlen(body));
    }
    if (hl < 0 || (size_t)hl >= sizeof(header)) {
        close(fd);
        return NULL;
    }
    if (send(fd, header, (size_t)hl, 0) < 0 || send(fd, body, strlen(body), 0) < 0) {
        close(fd);
        return NULL;
    }
    size_t cap = 8192, len = 0;
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
    char *bodyp = strstr(buf, "\r\n\r\n");
    if (bodyp) {
        bodyp += 4;
        char *out = dup_str(bodyp);
        free(buf);
        return out;
    }
    return buf;
}
#endif

int32_t buraaq_ai_model(const char *name) {
    if (!name || !name[0] || strstr(name, "..") != NULL) return 0;
    if (g_next >= AI_MAX_MODELS) return 0;
    int id = g_next++;
    AiModel *m = &g_models[id];
    memset(m, 0, sizeof(*m));
    strncpy(m->name, name, AI_NAME_MAX - 1);
    m->used = 1;
    return id;
}

int32_t buraaq_ai_system(int32_t handle, const char *prompt) {
    if (handle <= 0 || handle >= AI_MAX_MODELS || !g_models[handle].used) return -1;
    if (!prompt) prompt = "";
    strncpy(g_models[handle].system, prompt, AI_SYS_MAX - 1);
    return 0;
}

char *buraaq_ai_chat(int32_t handle, const char *prompt) {
    if (handle <= 0 || handle >= AI_MAX_MODELS || !g_models[handle].used) {
        return dup_str("error: invalid ai model handle");
    }
    if (!prompt) prompt = "";
    AiModel *m = &g_models[handle];
    char esc_user[8192];
    char esc_sys[8192];
    char esc_model[512];
    json_escape(prompt, esc_user, sizeof(esc_user));
    json_escape(m->system, esc_sys, sizeof(esc_sys));
    json_escape(m->name, esc_model, sizeof(esc_model));

    char *body = NULL;
    if (m->system[0]) {
        size_t need = strlen(esc_user) + strlen(esc_sys) + strlen(esc_model) + 256;
        body = (char *)malloc(need);
        if (!body) return dup_str("error: oom");
        snprintf(body, need,
                 "{\"model\":\"%s\",\"messages\":[{\"role\":\"system\",\"content\":\"%s\"},"
                 "{\"role\":\"user\",\"content\":\"%s\"}],\"max_tokens\":1024}",
                 esc_model, esc_sys, esc_user);
    } else {
        size_t need = strlen(esc_user) + strlen(esc_model) + 200;
        body = (char *)malloc(need);
        if (!body) return dup_str("error: oom");
        snprintf(body, need,
                 "{\"model\":\"%s\",\"messages\":[{\"role\":\"user\",\"content\":\"%s\"}],"
                 "\"max_tokens\":1024}",
                 esc_model, esc_user);
    }

    char url[512];
    snprintf(url, sizeof(url), "%s/v1/chat/completions", ai_base_url());
    char *resp = http_post_json(url, body, ai_api_key());
    free(body);
    if (!resp) {
        return dup_str(
            "error: AI serve unreachable. Run: buraaq ai serve MODEL  (or set BURAAQ_AI_BASE_URL)");
    }
    char *content = extract_content(resp);
    free(resp);
    return content;
}

char *buraaq_ai_embed(int32_t handle, const char *text) {
    (void)handle;
    (void)text;
    return dup_str("[]"); /* Phase 1: embeddings via serve may 501 */
}
