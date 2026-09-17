/* Lumen — native HD UI for Buraaq.
 *
 * Win32 GDI (ClearType, per-monitor DPI). POSIX compiles with a stub that
 * returns a clear error until the X11/Cocoa blit is wired.
 */
#ifndef _POSIX_C_SOURCE
#define _POSIX_C_SOURCE 200809L
#endif
#include "buraaq_std.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <ctype.h>

#ifdef _WIN32
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef UNICODE
#define UNICODE
#endif
#ifndef _UNICODE
#define _UNICODE
#endif
#ifndef _WIN32_WINNT
#define _WIN32_WINNT 0x0A00
#endif
#include <windows.h>
#include <windowsx.h>
#include <wininet.h>
#endif

#define UI_MAX_W 48
#define UI_MAX_ROWS 64
#define UI_VAL 512
#define UI_ID 32

enum {
    UI_HEADING = 1,
    UI_NOTE,
    UI_FIELD,
    UI_BUTTON,
    UI_LIST
};

typedef struct {
    int kind;
    char id[UI_ID];
    char label[256];
    char value[UI_VAL];
    RECT box;
    int hot;
} UiW;

typedef struct {
    char id[32];
    char title[256];
    char body[512];
    char created[64];
} UiRow;

#ifndef _WIN32
typedef struct { long left, top, right, bottom; } RECT;
#endif

static char g_keep_path[260];
static int g_next_id = 1;
static UiW g_w[UI_MAX_W];
static int g_nw;
static char g_title[128] = "Lumen";
static int g_cw = 1280, g_ch = 800;
static int g_focus = -1;
static int g_scroll;
static char g_bind_base[256];
static char g_bind_res[64];
static int g_bound;
static UiRow g_rows[UI_MAX_ROWS];
static int g_nrows;
static char g_status[256] = "Ready";
static int g_live;
static char g_clicked[UI_ID];
static int g_running;

#ifdef _WIN32
static HWND g_hwnd;
static int g_dpi = 96;
static HFONT g_font_title;
static HFONT g_font_body;
static HFONT g_font_small;
static HFONT g_font_btn;

static int px(int logical) { return MulDiv(logical, g_dpi, 96); }

static COLORREF col_bg(void) { return RGB(240, 253, 250); }
static COLORREF col_fg(void) { return RGB(19, 78, 74); }
static COLORREF col_primary(void) { return RGB(13, 148, 136); }
static COLORREF col_accent(void) { return RGB(234, 88, 12); }
static COLORREF col_card(void) { return RGB(255, 255, 255); }
static COLORREF col_border(void) { return RGB(153, 246, 228); }
static COLORREF col_muted(void) { return RGB(71, 85, 105); }
static COLORREF col_danger(void) { return RGB(220, 38, 38); }
static COLORREF col_on_acc(void) { return RGB(0, 0, 0); }

static void utf8_to_wide(const char *s, wchar_t *o, int cap) {
    if (!s) s = "";
    MultiByteToWideChar(CP_UTF8, 0, s, -1, o, cap);
    o[cap - 1] = 0;
}

static void fill_round(HDC hdc, RECT r, COLORREF fill, COLORREF border, int rad) {
    HBRUSH br = CreateSolidBrush(fill);
    HPEN pn = CreatePen(PS_SOLID, px(1), border);
    HGDIOBJ obr = SelectObject(hdc, br);
    HGDIOBJ opn = SelectObject(hdc, pn);
    RoundRect(hdc, r.left, r.top, r.right, r.bottom, rad, rad);
    SelectObject(hdc, obr);
    SelectObject(hdc, opn);
    DeleteObject(br);
    DeleteObject(pn);
}

static void draw_text(HDC hdc, HFONT font, COLORREF c, RECT r, const char *s, UINT fmt) {
    wchar_t w[1024];
    utf8_to_wide(s ? s : "", w, 1024);
    HGDIOBJ old = SelectObject(hdc, font);
    SetBkMode(hdc, TRANSPARENT);
    SetTextColor(hdc, c);
    DrawTextW(hdc, w, -1, &r, fmt | DT_NOPREFIX);
    SelectObject(hdc, old);
}

static int http_do(const char *method, const char *url, const char *body, char *out, int cap) {
    if (out && cap > 0) out[0] = 0;
    if (!url || !url[0]) return 0;
    HINTERNET ses = InternetOpenA("BuraaqLumen/1", INTERNET_OPEN_TYPE_PRECONFIG, NULL, NULL, 0);
    if (!ses) return 0;
    DWORD timeout = 2500;
    InternetSetOptionA(ses, INTERNET_OPTION_CONNECT_TIMEOUT, &timeout, sizeof(timeout));
    InternetSetOptionA(ses, INTERNET_OPTION_RECEIVE_TIMEOUT, &timeout, sizeof(timeout));
    HINTERNET req = InternetOpenUrlA(
        ses, url, "Accept: application/json\r\n", (DWORD)-1,
        INTERNET_FLAG_RELOAD | INTERNET_FLAG_NO_CACHE_WRITE | INTERNET_FLAG_NO_UI, 0);
    /* GET via OpenUrl; mutations use a parsed connect. */
    if (strcmp(method, "GET") == 0) {
        if (!req) {
            InternetCloseHandle(ses);
            return 0;
        }
        int n = 0;
        DWORD got = 0;
        while (n + 1 < cap) {
            if (!InternetReadFile(req, out + n, (DWORD)(cap - 1 - n), &got) || got == 0) break;
            n += (int)got;
        }
        out[n] = 0;
        InternetCloseHandle(req);
        InternetCloseHandle(ses);
        return 1;
    }
    if (req) InternetCloseHandle(req);

    char host[128] = {0};
    char path[256] = "/";
    int port = 80;
    int https = 0;
    const char *p = url;
    if (!strncmp(p, "https://", 8)) {
        https = 1;
        port = 443;
        p += 8;
    } else if (!strncmp(p, "http://", 7)) {
        p += 7;
    } else {
        InternetCloseHandle(ses);
        return 0;
    }
    const char *slash = strchr(p, '/');
    const char *colon = strchr(p, ':');
    if (colon && (!slash || colon < slash)) {
        size_t hl = (size_t)(colon - p);
        if (hl >= sizeof(host)) hl = sizeof(host) - 1;
        memcpy(host, p, hl);
        host[hl] = 0;
        port = atoi(colon + 1);
    } else {
        size_t hl = slash ? (size_t)(slash - p) : strlen(p);
        if (hl >= sizeof(host)) hl = sizeof(host) - 1;
        memcpy(host, p, hl);
        host[hl] = 0;
    }
    if (slash) {
        strncpy(path, slash, sizeof(path) - 1);
    }
    HINTERNET con = InternetConnectA(ses, host, (INTERNET_PORT)port, NULL, NULL,
                                     INTERNET_SERVICE_HTTP, 0, 0);
    if (!con) {
        InternetCloseHandle(ses);
        return 0;
    }
    const char *hdr = "Content-Type: application/json\r\n";
    DWORD flags = INTERNET_FLAG_RELOAD | INTERNET_FLAG_NO_CACHE_WRITE | INTERNET_FLAG_NO_UI;
    if (https) flags |= INTERNET_FLAG_SECURE;
    HINTERNET hr = HttpOpenRequestA(con, method, path, NULL, NULL, NULL, flags, 0);
    if (!hr) {
        InternetCloseHandle(con);
        InternetCloseHandle(ses);
        return 0;
    }
    DWORD blen = body ? (DWORD)strlen(body) : 0;
    BOOL ok = HttpSendRequestA(hr, hdr, (DWORD)strlen(hdr), (LPVOID)body, blen);
    if (ok && out && cap > 0) {
        int n = 0;
        DWORD got = 0;
        while (n + 1 < cap) {
            if (!InternetReadFile(hr, out + n, (DWORD)(cap - 1 - n), &got) || got == 0) break;
            n += (int)got;
        }
        out[n] = 0;
    }
    InternetCloseHandle(hr);
    InternetCloseHandle(con);
    InternetCloseHandle(ses);
    return ok ? 1 : 0;
}

static int json_unescape_copy(const char *s, const char *end, char *dst, int cap) {
    int n = 0;
    while (s < end && n + 1 < cap) {
        if (*s == '\\' && s + 1 < end) {
            s++;
            dst[n++] = *s++;
        } else {
            dst[n++] = *s++;
        }
    }
    dst[n] = 0;
    return n;
}

static int json_field(const char *obj, const char *key, char *dst, int cap) {
    char pat[80];
    snprintf(pat, sizeof(pat), "\"%s\"", key);
    const char *p = strstr(obj, pat);
    if (!p) {
        if (dst && cap) dst[0] = 0;
        return 0;
    }
    p = strchr(p + strlen(pat), ':');
    if (!p) return 0;
    p++;
    while (*p == ' ') p++;
    if (*p == '"') {
        p++;
        const char *e = p;
        while (*e && *e != '"') {
            if (*e == '\\' && e[1]) e += 2;
            else e++;
        }
        json_unescape_copy(p, e, dst, cap);
        return 1;
    }
    const char *e = p;
    while (*e && *e != ',' && *e != '}' && *e != ' ') e++;
    json_unescape_copy(p, e, dst, cap);
    return 1;
}

static void ui_parse_rows(const char *buf) {
    g_nrows = 0;
    g_next_id = 1;
    if (!buf) return;
    const char *p = strstr(buf, "\"rows\"");
    if (!p) return;
    p = strchr(p, '[');
    if (!p) return;
    p++;
    while (*p && g_nrows < UI_MAX_ROWS) {
        const char *obj = strchr(p, '{');
        if (!obj) break;
        const char *end = strchr(obj, '}');
        if (!end) break;
        char chunk[1024];
        size_t n = (size_t)(end - obj + 1);
        if (n >= sizeof(chunk)) n = sizeof(chunk) - 1;
        memcpy(chunk, obj, n);
        chunk[n] = 0;
        UiRow *r = &g_rows[g_nrows];
        memset(r, 0, sizeof(*r));
        json_field(chunk, "id", r->id, sizeof(r->id));
        json_field(chunk, "title", r->title, sizeof(r->title));
        json_field(chunk, "body", r->body, sizeof(r->body));
        json_field(chunk, "created_at", r->created, sizeof(r->created));
        int idn = atoi(r->id);
        if (idn >= g_next_id) g_next_id = idn + 1;
        g_nrows++;
        p = end + 1;
    }
}

static void json_esc(const char *s, char *d, int cap) {
    int n = 0;
    if (!s) s = "";
    while (*s && n + 2 < cap) {
        if (*s == '"' || *s == '\\') {
            d[n++] = '\\';
            d[n++] = *s++;
        } else if (*s == '\n' || *s == '\r') {
            s++;
            if (n + 2 < cap) {
                d[n++] = '\\';
                d[n++] = 'n';
            }
        } else {
            d[n++] = *s++;
        }
    }
    d[n] = 0;
}

static void ui_keep_save(void) {
    if (!g_keep_path[0]) return;
    FILE *f = fopen(g_keep_path, "wb");
    if (!f) return;
    fputs("{\"rows\":[", f);
    for (int i = 0; i < g_nrows; i++) {
        char t[512], b[1024];
        json_esc(g_rows[i].title, t, sizeof(t));
        json_esc(g_rows[i].body, b, sizeof(b));
        fprintf(f, "%s{\"id\":\"%s\",\"title\":\"%s\",\"body\":\"%s\",\"created_at\":\"%s\"}",
                i ? "," : "", g_rows[i].id, t, b, g_rows[i].created);
    }
    fputs("]}", f);
    fclose(f);
}

static void ui_keep_load(void) {
    if (!g_keep_path[0]) return;
    FILE *f = fopen(g_keep_path, "rb");
    if (!f) {
        g_nrows = 0;
        snprintf(g_status, sizeof(g_status), "Desk  %s", g_keep_path);
        g_live = 1;
        return;
    }
    char buf[1 << 16];
    size_t n = fread(buf, 1, sizeof(buf) - 1, f);
    buf[n] = 0;
    fclose(f);
    ui_parse_rows(buf);
    g_live = 1;
    snprintf(g_status, sizeof(g_status), "Desk  %d notes  %s", g_nrows, g_keep_path);
}

static void ui_refresh_bind(void) {
    if (!g_bound) {
        ui_keep_load();
        return;
    }
    char url[320];
    char buf[1 << 16];
    snprintf(url, sizeof(url), "%s/api/health", g_bind_base);
    g_live = 0;
    if (http_do("GET", url, NULL, buf, sizeof(buf)) && strstr(buf, "\"ok\":true")) {
        g_live = 1;
        snprintf(g_status, sizeof(g_status), "API live  %s", g_bind_base);
    } else {
        snprintf(g_status, sizeof(g_status), "API down  %s", g_bind_base);
    }
    snprintf(url, sizeof(url), "%s/api/%s", g_bind_base, g_bind_res);
    if (!http_do("GET", url, NULL, buf, sizeof(buf))) {
        g_nrows = 0;
        return;
    }
    ui_parse_rows(buf);
}

static UiW *ui_find(const char *id) {
    if (!id) return NULL;
    for (int i = 0; i < g_nw; i++) {
        if (strcmp(g_w[i].id, id) == 0) return &g_w[i];
    }
    return NULL;
}

static void ui_post_create(void) {
    UiW *t = ui_find("title");
    UiW *b = ui_find("body");
    const char *title = t ? t->value : "";
    const char *body = b ? b->value : "";
    if (!title[0]) {
        snprintf(g_status, sizeof(g_status), "Title is required.");
        return;
    }
    if (g_bound) {
        char te[512], be[1024];
        char payload[1600];
        json_esc(title, te, sizeof(te));
        json_esc(body, be, sizeof(be));
        snprintf(payload, sizeof(payload), "{\"title\":\"%s\",\"body\":\"%s\"}", te, be);
        char url[320];
        char resp[4096];
        snprintf(url, sizeof(url), "%s/api/%s", g_bind_base, g_bind_res);
        if (http_do("POST", url, payload, resp, sizeof(resp))) {
            snprintf(g_status, sizeof(g_status), "Created.");
            if (t) t->value[0] = 0;
            if (b) b->value[0] = 0;
            ui_refresh_bind();
        } else {
            snprintf(g_status, sizeof(g_status), "Create failed.");
        }
        return;
    }
    if (g_nrows >= UI_MAX_ROWS) {
        snprintf(g_status, sizeof(g_status), "Desk is full.");
        return;
    }
    UiRow *r = &g_rows[g_nrows++];
    memset(r, 0, sizeof(*r));
    snprintf(r->id, sizeof(r->id), "%d", g_next_id++);
    strncpy(r->title, title, sizeof(r->title) - 1);
    strncpy(r->body, body, sizeof(r->body) - 1);
    {
        SYSTEMTIME st;
        GetLocalTime(&st);
        snprintf(r->created, sizeof(r->created), "%04d-%02d-%02d %02d:%02d",
                 (int)st.wYear, (int)st.wMonth, (int)st.wDay, (int)st.wHour, (int)st.wMinute);
    }
    if (t) t->value[0] = 0;
    if (b) b->value[0] = 0;
    ui_keep_save();
    snprintf(g_status, sizeof(g_status), "Saved  %d notes", g_nrows);
    g_live = 1;
}

static void ui_delete_id(const char *id) {
    if (!id || !id[0]) return;
    if (g_bound) {
        char url[320];
        char resp[1024];
        snprintf(url, sizeof(url), "%s/api/%s/%s", g_bind_base, g_bind_res, id);
        http_do("DELETE", url, "", resp, sizeof(resp));
        ui_refresh_bind();
        return;
    }
    int w = 0;
    for (int i = 0; i < g_nrows; i++) {
        if (strcmp(g_rows[i].id, id) == 0) continue;
        if (w != i) g_rows[w] = g_rows[i];
        w++;
    }
    g_nrows = w;
    ui_keep_save();
    snprintf(g_status, sizeof(g_status), "Saved  %d notes", g_nrows);
}

static void ui_layout(int w, int h) {
    int pad = px(48);
    int gap = px(16);
    int cw = w - pad * 2;
    if (cw > px(720)) cw = px(720);
    int x = (w - cw) / 2;
    int y = pad;
    for (int i = 0; i < g_nw; i++) {
        UiW *e = &g_w[i];
        int ht = px(40);
        if (e->kind == UI_HEADING) ht = px(48);
        else if (e->kind == UI_NOTE) ht = px(28);
        else if (e->kind == UI_FIELD) ht = px(72);
        else if (e->kind == UI_BUTTON) ht = px(44);
        else if (e->kind == UI_LIST) ht = h - y - pad;
        if (ht < px(24)) ht = px(24);
        e->box.left = x;
        e->box.top = y;
        e->box.right = x + cw;
        if (e->kind == UI_BUTTON) e->box.right = x + px(168);
        e->box.bottom = y + ht;
        y += ht + gap;
    }
}

static void ui_paint(HDC hdc, int w, int h) {
    RECT all = {0, 0, w, h};
    HBRUSH bg = CreateSolidBrush(col_bg());
    FillRect(hdc, &all, bg);
    DeleteObject(bg);

    RECT badge = {px(48), px(20), w - px(48), px(44)};
    draw_text(hdc, g_font_small, g_live ? col_primary() : col_danger(), badge, g_status,
              DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    for (int i = 0; i < g_nw; i++) {
        UiW *e = &g_w[i];
        RECT r = e->box;
        if (e->kind == UI_HEADING) {
            draw_text(hdc, g_font_title, col_fg(), r, e->label, DT_LEFT | DT_BOTTOM | DT_SINGLELINE);
        } else if (e->kind == UI_NOTE) {
            draw_text(hdc, g_font_body, col_muted(), r, e->label, DT_LEFT | DT_TOP | DT_WORDBREAK);
        } else if (e->kind == UI_FIELD) {
            RECT lab = r;
            lab.bottom = r.top + px(22);
            draw_text(hdc, g_font_small, col_fg(), lab, e->label, DT_LEFT | DT_BOTTOM | DT_SINGLELINE);
            RECT box = r;
            box.top = r.top + px(26);
            fill_round(hdc, box, col_card(), e->hot || g_focus == i ? col_primary() : col_border(), px(10));
            RECT tr = box;
            tr.left += px(12);
            tr.right -= px(12);
            tr.top += px(6);
            const char *show = e->value[0] ? e->value : "";
            draw_text(hdc, g_font_body, show[0] ? col_fg() : col_muted(), tr, show,
                      DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS);
            if (g_focus == i) {
                SIZE sz = {0};
                wchar_t ww[UI_VAL];
                utf8_to_wide(e->value, ww, UI_VAL);
                HGDIOBJ old = SelectObject(hdc, g_font_body);
                GetTextExtentPoint32W(hdc, ww, (int)wcslen(ww), &sz);
                SelectObject(hdc, old);
                int cx = tr.left + sz.cx + px(1);
                int cy1 = box.top + px(10);
                int cy2 = box.bottom - px(10);
                HPEN pn = CreatePen(PS_SOLID, px(1), col_primary());
                HGDIOBJ opn = SelectObject(hdc, pn);
                MoveToEx(hdc, cx, cy1, NULL);
                LineTo(hdc, cx, cy2);
                SelectObject(hdc, opn);
                DeleteObject(pn);
            }
        } else if (e->kind == UI_BUTTON) {
            COLORREF fill = e->hot ? RGB(249, 115, 22) : col_accent();
            fill_round(hdc, r, fill, fill, px(10));
            draw_text(hdc, g_font_btn, col_on_acc(), r, e->label,
                      DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        } else if (e->kind == UI_LIST) {
            fill_round(hdc, r, col_card(), col_border(), px(16));
            RECT clip = r;
            clip.left += px(16);
            clip.right -= px(16);
            clip.top += px(16);
            clip.bottom -= px(16);
            HRGN rg = CreateRectRgn(clip.left, clip.top, clip.right, clip.bottom);
            SelectClipRgn(hdc, rg);
            int y = clip.top - g_scroll;
            if (g_nrows == 0) {
                draw_text(hdc, g_font_body, col_muted(), clip,
                          g_bound ? "No jobs yet. Create one." : "No notes yet. Write one.",
                          DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            }
            for (int k = 0; k < g_nrows; k++) {
                RECT card = {clip.left, y, clip.right, y + px(108)};
                if (card.bottom > clip.top && card.top < clip.bottom) {
                    fill_round(hdc, card, col_bg(), col_border(), px(12));
                    RECT t = card;
                    t.left += px(16);
                    t.right -= px(96);
                    t.top += px(12);
                    t.bottom = t.top + px(28);
                    draw_text(hdc, g_font_btn, col_fg(), t, g_rows[k].title,
                              DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS);
                    t.top = t.bottom;
                    t.bottom += px(36);
                    draw_text(hdc, g_font_small, col_muted(), t, g_rows[k].body,
                              DT_LEFT | DT_TOP | DT_WORDBREAK | DT_END_ELLIPSIS);
                    RECT del = {card.right - px(84), card.top + px(16), card.right - px(16),
                                card.top + px(48)};
                    fill_round(hdc, del, col_card(), col_danger(), px(8));
                    draw_text(hdc, g_font_small, col_danger(), del, "Delete",
                              DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                }
                y += px(120);
            }
            SelectClipRgn(hdc, NULL);
            DeleteObject(rg);
        }
    }
}

static int hit_list_delete(int mx, int my) {
    for (int i = 0; i < g_nw; i++) {
        if (g_w[i].kind != UI_LIST) continue;
        RECT r = g_w[i].box;
        int pad = px(16);
        int y = r.top + pad - g_scroll;
        for (int k = 0; k < g_nrows; k++) {
            RECT del = {r.right - pad - px(84), y + px(16), r.right - pad - px(16), y + px(48)};
            POINT pt = {mx, my};
            if (PtInRect(&del, pt)) return k;
            y += px(120);
        }
    }
    return -1;
}

static void ui_click(int mx, int my) {
    POINT pt = {mx, my};
    int del = hit_list_delete(mx, my);
    if (del >= 0) {
        ui_delete_id(g_rows[del].id);
        return;
    }
    g_focus = -1;
    for (int i = 0; i < g_nw; i++) {
        if (!PtInRect(&g_w[i].box, pt)) continue;
        if (g_w[i].kind == UI_FIELD) {
            g_focus = i;
            return;
        }
        if (g_w[i].kind == UI_BUTTON) {
            strncpy(g_clicked, g_w[i].id, UI_ID - 1);
            if (strcmp(g_w[i].id, "create") == 0 || strcmp(g_w[i].id, "save") == 0) {
                ui_post_create();
            }
            return;
        }
    }
}

static void ui_char(wchar_t ch) {
    if (g_focus < 0 || g_focus >= g_nw) return;
    UiW *e = &g_w[g_focus];
    if (e->kind != UI_FIELD) return;
    if (ch == 8) {
        size_t n = strlen(e->value);
        if (n) e->value[n - 1] = 0;
        return;
    }
    if (ch < 32) return;
    char u8[8];
    int n = WideCharToMultiByte(CP_UTF8, 0, &ch, 1, u8, sizeof(u8), NULL, NULL);
    size_t have = strlen(e->value);
    if (n > 0 && have + (size_t)n < sizeof(e->value) - 1) {
        memcpy(e->value + have, u8, (size_t)n);
        e->value[have + (size_t)n] = 0;
    }
}

static void ui_hot(int mx, int my) {
    POINT pt = {mx, my};
    for (int i = 0; i < g_nw; i++) {
        g_w[i].hot = PtInRect(&g_w[i].box, pt) ? 1 : 0;
    }
}

static void ui_fonts(HWND hwnd) {
    if (g_font_title) DeleteObject(g_font_title);
    if (g_font_body) DeleteObject(g_font_body);
    if (g_font_small) DeleteObject(g_font_small);
    if (g_font_btn) DeleteObject(g_font_btn);
    LOGFONTW lf;
    memset(&lf, 0, sizeof(lf));
    lf.lfHeight = -px(32);
    lf.lfWeight = FW_SEMIBOLD;
    lf.lfQuality = CLEARTYPE_QUALITY;
    wcscpy(lf.lfFaceName, L"Segoe UI");
    g_font_title = CreateFontIndirectW(&lf);
    lf.lfHeight = -px(15);
    lf.lfWeight = FW_NORMAL;
    g_font_body = CreateFontIndirectW(&lf);
    lf.lfHeight = -px(13);
    lf.lfWeight = FW_MEDIUM;
    g_font_small = CreateFontIndirectW(&lf);
    lf.lfHeight = -px(14);
    lf.lfWeight = FW_SEMIBOLD;
    g_font_btn = CreateFontIndirectW(&lf);
    (void)hwnd;
}

static LRESULT CALLBACK ui_wnd(HWND hwnd, UINT msg, WPARAM wparam, LPARAM lparam) {
    switch (msg) {
    case WM_CREATE:
        g_dpi = (int)GetDpiForWindow(hwnd);
        if (g_dpi < 96) g_dpi = 96;
        ui_fonts(hwnd);
        return 0;
    case WM_DPICHANGED: {
        g_dpi = HIWORD(wparam);
        ui_fonts(hwnd);
        RECT *nr = (RECT *)lparam;
        SetWindowPos(hwnd, NULL, nr->left, nr->top, nr->right - nr->left, nr->bottom - nr->top,
                     SWP_NOZORDER | SWP_NOACTIVATE);
        InvalidateRect(hwnd, NULL, FALSE);
        return 0;
    }
    case WM_SIZE: {
        RECT rc;
        GetClientRect(hwnd, &rc);
        ui_layout(rc.right, rc.bottom);
        InvalidateRect(hwnd, NULL, FALSE);
        return 0;
    }
    case WM_ERASEBKGND:
        return 1;
    case WM_PAINT: {
        PAINTSTRUCT ps;
        HDC hdc = BeginPaint(hwnd, &ps);
        RECT rc;
        GetClientRect(hwnd, &rc);
        HDC mem = CreateCompatibleDC(hdc);
        HBITMAP bmp = CreateCompatibleBitmap(hdc, rc.right, rc.bottom);
        HGDIOBJ old = SelectObject(mem, bmp);
        ui_paint(mem, rc.right, rc.bottom);
        BitBlt(hdc, 0, 0, rc.right, rc.bottom, mem, 0, 0, SRCCOPY);
        SelectObject(mem, old);
        DeleteObject(bmp);
        DeleteDC(mem);
        EndPaint(hwnd, &ps);
        return 0;
    }
    case WM_MOUSEMOVE:
        ui_hot(GET_X_LPARAM(lparam), GET_Y_LPARAM(lparam));
        InvalidateRect(hwnd, NULL, FALSE);
        return 0;
    case WM_LBUTTONDOWN:
        SetCapture(hwnd);
        ui_click(GET_X_LPARAM(lparam), GET_Y_LPARAM(lparam));
        InvalidateRect(hwnd, NULL, FALSE);
        return 0;
    case WM_LBUTTONUP:
        ReleaseCapture();
        return 0;
    case WM_MOUSEWHEEL: {
        int delta = GET_WHEEL_DELTA_WPARAM(wparam);
        g_scroll -= (delta / WHEEL_DELTA) * px(48);
        if (g_scroll < 0) g_scroll = 0;
        InvalidateRect(hwnd, NULL, FALSE);
        return 0;
    }
    case WM_CHAR:
        ui_char((wchar_t)wparam);
        InvalidateRect(hwnd, NULL, FALSE);
        return 0;
    case WM_KEYDOWN:
        if (wparam == VK_TAB) {
            int start = g_focus;
            for (int n = 0; n < g_nw; n++) {
                int i = (start + 1 + n) % g_nw;
                if (g_w[i].kind == UI_FIELD) {
                    g_focus = i;
                    break;
                }
            }
            InvalidateRect(hwnd, NULL, FALSE);
        } else if (wparam == VK_RETURN && g_bound) {
            ui_post_create();
            InvalidateRect(hwnd, NULL, FALSE);
        } else if (wparam == VK_F5) {
            ui_refresh_bind();
            InvalidateRect(hwnd, NULL, FALSE);
        }
        return 0;
    case WM_DESTROY:
        g_running = 0;
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProcW(hwnd, msg, wparam, lparam);
}

static int ui_open_window(void) {
    HINSTANCE inst = GetModuleHandleW(NULL);
    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    WNDCLASSW wc;
    memset(&wc, 0, sizeof(wc));
    wc.lpfnWndProc = ui_wnd;
    wc.hInstance = inst;
    wc.hCursor = LoadCursor(NULL, IDC_ARROW);
    wc.hbrBackground = (HBRUSH)(COLOR_WINDOW + 1);
    wc.lpszClassName = L"BuraaqLumen";
    RegisterClassW(&wc);
    wchar_t wt[160];
    utf8_to_wide(g_title, wt, 160);
    int sw = GetSystemMetrics(SM_CXSCREEN);
    int sh = GetSystemMetrics(SM_CYSCREEN);
    int ww = g_cw, hh = g_ch;
    HWND hwnd = CreateWindowExW(
        WS_EX_APPWINDOW, L"BuraaqLumen", wt, WS_OVERLAPPEDWINDOW | WS_VISIBLE,
        (sw - ww) / 2, (sh - hh) / 2, ww, hh, NULL, NULL, inst, NULL);
    if (!hwnd) return 0;
    g_hwnd = hwnd;
    for (int i = 0; i < g_nw; i++) {
        if (g_w[i].kind == UI_FIELD) {
            g_focus = i;
            break;
        }
    }
    ShowWindow(hwnd, SW_SHOW);
    UpdateWindow(hwnd);
    return 1;
}

static int32_t ui_loop(void) {
    if (!ui_open_window()) {
        fprintf(stderr, "lumen: could not create window\n");
        return 0;
    }
    ui_refresh_bind();
    g_running = 1;
    MSG msg;
    while (g_running && GetMessageW(&msg, NULL, 0, 0) > 0) {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
    return 1;
}
#else
static int32_t ui_loop(void) {
    fprintf(stderr, "lumen: native HD window is on Windows today; Linux/macOS blit is next\n");
    return 0;
}
static void ui_refresh_bind(void) {}
#endif

static int32_t ui_add(int kind, const char *id, const char *label) {
    if (g_nw >= UI_MAX_W) return 0;
    UiW *e = &g_w[g_nw++];
    memset(e, 0, sizeof(*e));
    e->kind = kind;
    if (id) strncpy(e->id, id, UI_ID - 1);
    if (label) strncpy(e->label, label, sizeof(e->label) - 1);
    return 1;
}

int32_t buraaq_ui_app(const char *title, int32_t w, int32_t h) {
    if (title && title[0]) strncpy(g_title, title, sizeof(g_title) - 1);
    if (w > 400) g_cw = w;
    if (h > 300) g_ch = h;
    return 1;
}

int32_t buraaq_ui_heading(const char *text) { return ui_add(UI_HEADING, "", text ? text : ""); }
int32_t buraaq_ui_note(const char *text) { return ui_add(UI_NOTE, "", text ? text : ""); }
int32_t buraaq_ui_field(const char *id, const char *label) {
    return ui_add(UI_FIELD, id ? id : "field", label ? label : "");
}
int32_t buraaq_ui_button(const char *id, const char *label) {
    return ui_add(UI_BUTTON, id ? id : "ok", label ? label : "OK");
}
int32_t buraaq_ui_bind(const char *base_url, const char *resource) {
    g_bound = 1;
    g_bind_base[0] = 0;
    g_bind_res[0] = 0;
    if (base_url && base_url[0]) {
        strncpy(g_bind_base, base_url, sizeof(g_bind_base) - 1);
        size_t n = strlen(g_bind_base);
        while (n && g_bind_base[n - 1] == '/') {
            g_bind_base[--n] = 0;
        }
    } else {
        strncpy(g_bind_base, "https://127.0.0.1:8443", sizeof(g_bind_base) - 1);
    }
    if (resource) strncpy(g_bind_res, resource, sizeof(g_bind_res) - 1);
    ui_add(UI_LIST, "list", resource ? resource : "items");
    return 1;
}

int32_t buraaq_ui_keep(const char *path) {
    g_keep_path[0] = 0;
    if (path && path[0]) strncpy(g_keep_path, path, sizeof(g_keep_path) - 1);
    int has_list = 0;
    for (int i = 0; i < g_nw; i++) {
        if (g_w[i].kind == UI_LIST) has_list = 1;
    }
    if (!has_list) ui_add(UI_LIST, "list", "notes");
    return 1;
}

int32_t buraaq_ui_show(void) {
    int has_list = 0;
    for (int i = 0; i < g_nw; i++) {
        if (g_w[i].kind == UI_LIST) has_list = 1;
    }
    if (!has_list) ui_add(UI_LIST, "list", "notes");
    if (!g_bound && !g_keep_path[0]) strncpy(g_keep_path, "desk.json", sizeof(g_keep_path) - 1);
    return ui_loop();
}

char *buraaq_ui_value(const char *id) {
#ifdef _WIN32
    UiW *e = ui_find(id);
    const char *s = e ? e->value : "";
#else
    const char *s = "";
    (void)id;
#endif
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

int32_t buraaq_ui_clicked(const char *id) {
    if (!id) return 0;
    return strcmp(g_clicked, id) == 0 ? 1 : 0;
}
