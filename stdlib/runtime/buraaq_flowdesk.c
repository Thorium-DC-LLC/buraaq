/* Flowdesk — borderless native Windows app for Buraaq.
 *
 * Chrome: custom title bar (drag, maximize, close) — no WS_OVERLAPPED frame.
 * Views: Settings (API URL) and Flow (Vein → API → context).
 *
 * Vein: reads on-screen text via Windows UI Automation accessibility tree.
 * No bitmaps, no DXGI capture, no OCR — the same class of API screen readers use.
 */
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
#define COBJMACROS
#include <ole2.h>
#include <UIAutomation.h>
#endif

#define FD_TITLEBAR 40
#define FD_NAV 48
#define FD_VAL 4096
#define FD_CTX 8192
#define FD_API 512

enum { VIEW_FLOW = 0, VIEW_SETTINGS = 1 };

static char g_title[128] = "Flowdesk";
static int g_cw = 1100, g_ch = 720;
static int g_view = VIEW_FLOW;
static char g_api[FD_API] = "http://127.0.0.1:8080/api/context";
static char g_vein[FD_VAL];
static char g_context[FD_CTX] = "Click another app (Notepad, browser…), then Read Vein.";
static char g_status[256] = "Vein tracks the last focused app — clicking Read will not steal that target.";
static int g_running;
static int g_api_focus;
static int g_maximized;

#ifndef _WIN32
int32_t buraaq_flow_desk(const char *title, int32_t w, int32_t h) {
    (void)title;
    (void)w;
    (void)h;
    return 1;
}
int32_t buraaq_flow_show(void) {
    fprintf(stderr, "flowdesk: native borderless UI is Windows-only today\n");
    return 0;
}
#else

static HWND g_hwnd;
static HWND g_vein_target; /* last foreground window that is not Flowdesk */
static char g_vein_target_title[256];
static int g_dpi = 96;
static HFONT g_font_title;
static HFONT g_font_body;
static HFONT g_font_small;
static HFONT g_font_btn;
static RECT g_rc_close, g_rc_max, g_rc_flow, g_rc_settings, g_rc_read, g_rc_send, g_rc_api;
static int g_hot; /* 1 close 2 max 3 flow 4 settings 5 read 6 send */

static int px(int logical) { return MulDiv(logical, g_dpi, 96); }

static COLORREF col_bg(void) { return RGB(15, 23, 42); }
static COLORREF col_panel(void) { return RGB(30, 41, 59); }
static COLORREF col_chrome(void) { return RGB(2, 44, 46); }
static COLORREF col_fg(void) { return RGB(240, 253, 250); }
static COLORREF col_muted(void) { return RGB(148, 163, 184); }
static COLORREF col_primary(void) { return RGB(13, 148, 136); }
static COLORREF col_accent(void) { return RGB(234, 88, 12); }
static COLORREF col_danger(void) { return RGB(220, 38, 38); }
static COLORREF col_border(void) { return RGB(45, 212, 191); }

static void utf8_to_wide(const char *s, wchar_t *o, int cap) {
    if (!s) s = "";
    MultiByteToWideChar(CP_UTF8, 0, s, -1, o, cap);
    o[cap - 1] = 0;
}

static void wide_to_utf8(const wchar_t *s, char *o, int cap) {
    if (!s) {
        if (cap > 0) o[0] = 0;
        return;
    }
    WideCharToMultiByte(CP_UTF8, 0, s, -1, o, cap, NULL, NULL);
    if (cap > 0) o[cap - 1] = 0;
}

static void ensure_fonts(void) {
    if (g_font_title) return;
    g_dpi = (int)GetDpiForWindow(g_hwnd);
    if (g_dpi <= 0) g_dpi = 96;
    g_font_title = CreateFontW(px(22), 0, 0, 0, FW_SEMIBOLD, 0, 0, 0, DEFAULT_CHARSET,
                               OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY,
                               DEFAULT_PITCH, L"Segoe UI");
    g_font_body = CreateFontW(px(15), 0, 0, 0, FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET,
                              OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY,
                              DEFAULT_PITCH, L"Segoe UI");
    g_font_small = CreateFontW(px(13), 0, 0, 0, FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET,
                               OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY,
                               DEFAULT_PITCH, L"Segoe UI");
    g_font_btn = CreateFontW(px(14), 0, 0, 0, FW_SEMIBOLD, 0, 0, 0, DEFAULT_CHARSET,
                             OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY,
                             DEFAULT_PITCH, L"Segoe UI");
}

static void fill_rect(HDC hdc, RECT r, COLORREF c) {
    HBRUSH br = CreateSolidBrush(c);
    FillRect(hdc, &r, br);
    DeleteObject(br);
}

static void frame_rect(HDC hdc, RECT r, COLORREF c) {
    HPEN pn = CreatePen(PS_SOLID, px(1), c);
    HGDIOBJ old = SelectObject(hdc, pn);
    HGDIOBJ obr = SelectObject(hdc, GetStockObject(NULL_BRUSH));
    Rectangle(hdc, r.left, r.top, r.right, r.bottom);
    SelectObject(hdc, obr);
    SelectObject(hdc, old);
    DeleteObject(pn);
}

static void draw_text(HDC hdc, HFONT font, COLORREF c, RECT r, const char *s, UINT fmt) {
    wchar_t w[4096];
    utf8_to_wide(s ? s : "", w, 4096);
    HGDIOBJ old = SelectObject(hdc, font);
    SetBkMode(hdc, TRANSPARENT);
    SetTextColor(hdc, c);
    DrawTextW(hdc, w, -1, &r, fmt | DT_NOPREFIX);
    SelectObject(hdc, old);
}

static void draw_btn(HDC hdc, RECT r, const char *label, int hot, int primary) {
    COLORREF fill = primary ? col_primary() : col_panel();
    if (hot) fill = primary ? RGB(15, 118, 110) : RGB(51, 65, 85);
    fill_rect(hdc, r, fill);
    frame_rect(hdc, r, col_border());
    draw_text(hdc, g_font_btn, col_fg(), r, label, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
}

static int hit(RECT r, int x, int y) {
    return x >= r.left && x < r.right && y >= r.top && y < r.bottom;
}

static void layout(int w, int h) {
    int tb = px(FD_TITLEBAR);
    int nav = px(FD_NAV);
    int pad = px(16);
    int bw = px(36);
    g_rc_close.left = w - bw - px(8);
    g_rc_close.top = px(4);
    g_rc_close.right = w - px(8);
    g_rc_close.bottom = tb - px(4);
    g_rc_max = g_rc_close;
    g_rc_max.left -= bw + px(4);
    g_rc_max.right = g_rc_close.left - px(4);

    g_rc_flow.left = pad;
    g_rc_flow.top = tb + px(8);
    g_rc_flow.right = pad + px(100);
    g_rc_flow.bottom = tb + nav - px(8);
    g_rc_settings = g_rc_flow;
    g_rc_settings.left = g_rc_flow.right + px(8);
    g_rc_settings.right = g_rc_settings.left + px(110);

    g_rc_read.left = pad;
    g_rc_read.top = h - px(56);
    g_rc_read.right = pad + px(140);
    g_rc_read.bottom = h - px(16);
    g_rc_send = g_rc_read;
    g_rc_send.left = g_rc_read.right + px(10);
    g_rc_send.right = g_rc_send.left + px(160);

    g_rc_api.left = pad;
    g_rc_api.top = tb + nav + px(72);
    g_rc_api.right = w - pad;
    g_rc_api.bottom = g_rc_api.top + px(36);
}

/* --- Vein: UI Automation + Win32 control text (no pixels / OCR) --- */

static void vein_track_foreground(void) {
    HWND fg = GetForegroundWindow();
    if (!fg || !IsWindow(fg)) return;
    if (fg == g_hwnd) return;
    /* Ignore our owned popups if any */
    HWND root = GetAncestor(fg, GA_ROOT);
    if (root == g_hwnd) return;
    g_vein_target = fg;
    wchar_t wt[200];
    if (GetWindowTextW(fg, wt, 200) > 0)
        wide_to_utf8(wt, g_vein_target_title, sizeof(g_vein_target_title));
    else
        snprintf(g_vein_target_title, sizeof(g_vein_target_title), "(untitled)");
}

static void vein_append(char *dst, int cap, const wchar_t *piece) {
    if (!piece || !piece[0]) return;
    /* skip pure whitespace */
    int any = 0;
    for (const wchar_t *p = piece; *p; p++) {
        if (*p > 32) {
            any = 1;
            break;
        }
    }
    if (!any) return;
    char utf[2048];
    wide_to_utf8(piece, utf, sizeof(utf));
    if (!utf[0]) return;
    /* de-dupe exact last line */
    size_t have = strlen(dst);
    if (have > 0) {
        const char *last = dst;
        const char *nl = strrchr(dst, '\n');
        if (nl) last = nl + 1;
        if (strcmp(last, utf) == 0) return;
    }
    if (have > 0 && have + 2 < (size_t)cap) {
        dst[have++] = '\n';
        dst[have] = 0;
    }
    size_t add = strlen(utf);
    if (have + add >= (size_t)cap) add = (size_t)cap - have - 1;
    if (add > 0) {
        memcpy(dst + have, utf, add);
        dst[have + add] = 0;
    }
}

typedef struct {
    char *dst;
    int cap;
} VeinGather;

static BOOL CALLBACK vein_enum_child(HWND child, LPARAM lp) {
    VeinGather *g = (VeinGather *)lp;
    if (!IsWindowVisible(child)) return TRUE;
    wchar_t buf[2048];
    buf[0] = 0;
    /* WM_GETTEXT works for Edit, RichEdit, static, buttons, etc. */
    LRESULT n = SendMessageW(child, WM_GETTEXT, (WPARAM)2047, (LPARAM)buf);
    if (n > 0) vein_append(g->dst, g->cap, buf);
    if ((int)strlen(g->dst) > g->cap - 200) return FALSE;
    return TRUE;
}

static void vein_win32_tree(HWND hwnd, char *dst, int cap) {
    if (!hwnd || !IsWindow(hwnd)) return;
    wchar_t title[512];
    if (GetWindowTextW(hwnd, title, 512) > 0) vein_append(dst, cap, title);
    VeinGather g = {dst, cap};
    EnumChildWindows(hwnd, vein_enum_child, (LPARAM)&g);
}

static void vein_from_element(IUIAutomation *auto_, IUIAutomationElement *el, char *dst, int cap, int depth) {
    if (!el || depth > 8) return;
    BSTR name = NULL;
    if (SUCCEEDED(IUIAutomationElement_get_CurrentName(el, &name)) && name) {
        if (SysStringLen(name) > 0) vein_append(dst, cap, name);
        SysFreeString(name);
    }
    IUIAutomationValuePattern *val = NULL;
    if (SUCCEEDED(IUIAutomationElement_GetCurrentPatternAs(el, UIA_ValuePatternId,
                                                            &IID_IUIAutomationValuePattern, (void **)&val)) &&
        val) {
        BSTR v = NULL;
        if (SUCCEEDED(IUIAutomationValuePattern_get_CurrentValue(val, &v)) && v) {
            if (SysStringLen(v) > 0) vein_append(dst, cap, v);
            SysFreeString(v);
        }
        IUIAutomationValuePattern_Release(val);
    }
    IUIAutomationTextPattern *tp = NULL;
    if (SUCCEEDED(IUIAutomationElement_GetCurrentPatternAs(el, UIA_TextPatternId,
                                                            &IID_IUIAutomationTextPattern, (void **)&tp)) &&
        tp) {
        IUIAutomationTextRange *doc = NULL;
        if (SUCCEEDED(IUIAutomationTextPattern_get_DocumentRange(tp, &doc)) && doc) {
            BSTR t = NULL;
            if (SUCCEEDED(IUIAutomationTextRange_GetText(doc, 8000, &t)) && t) {
                if (SysStringLen(t) > 0) vein_append(dst, cap, t);
                SysFreeString(t);
            }
            IUIAutomationTextRange_Release(doc);
        }
        IUIAutomationTextPattern_Release(tp);
    }

    if (depth >= 6) return;
    IUIAutomationCondition *TrueCond = NULL;
    IUIAutomationElementArray *kids = NULL;
    if (SUCCEEDED(IUIAutomation_CreateTrueCondition(auto_, &TrueCond)) && TrueCond) {
        IUIAutomationElement_FindAll(el, TreeScope_Children, TrueCond, &kids);
        IUIAutomationCondition_Release(TrueCond);
    }
    if (!kids) return;
    int n = 0;
    IUIAutomationElementArray_get_Length(kids, &n);
    if (n > 60) n = 60;
    for (int i = 0; i < n; i++) {
        IUIAutomationElement *child = NULL;
        if (SUCCEEDED(IUIAutomationElementArray_GetElement(kids, i, &child)) && child) {
            vein_from_element(auto_, child, dst, cap, depth + 1);
            IUIAutomationElement_Release(child);
        }
        if ((int)strlen(dst) > cap - 200) break;
    }
    IUIAutomationElementArray_Release(kids);
}

static int vein_uia_hwnd(HWND hwnd, char *dst, int cap) {
    if (!hwnd || !IsWindow(hwnd)) return 0;
    HRESULT hr = CoInitializeEx(NULL, COINIT_APARTMENTTHREADED);
    int need_uninit = SUCCEEDED(hr);
    if (FAILED(hr) && hr != RPC_E_CHANGED_MODE) return 0;

    IUIAutomation *auto_ = NULL;
    hr = CoCreateInstance(&CLSID_CUIAutomation, NULL, CLSCTX_INPROC_SERVER, &IID_IUIAutomation,
                          (void **)&auto_);
    if (FAILED(hr) || !auto_) {
        if (need_uninit) CoUninitialize();
        return 0;
    }

    IUIAutomationElement *root = NULL;
    hr = IUIAutomation_ElementFromHandle(auto_, hwnd, &root);
    if (SUCCEEDED(hr) && root) {
        vein_from_element(auto_, root, dst, cap, 0);
        IUIAutomationElement_Release(root);
    }
    IUIAutomation_Release(auto_);
    if (need_uninit) CoUninitialize();
    return dst[0] != 0;
}

static int vein_read_target(char *dst, int cap) {
    dst[0] = 0;
    vein_track_foreground(); /* refresh if user somehow still on other app */
    HWND target = g_vein_target;
    if (!target || !IsWindow(target) || target == g_hwnd) {
        /* Last resort: window under cursor (user can hover the other app) */
        POINT pt;
        if (GetCursorPos(&pt)) {
            HWND under = WindowFromPoint(pt);
            if (under) {
                under = GetAncestor(under, GA_ROOT);
                if (under && under != g_hwnd) target = under;
            }
        }
    }
    if (!target || !IsWindow(target) || target == g_hwnd) return 0;

    wchar_t wt[200];
    if (GetWindowTextW(target, wt, 200) > 0)
        wide_to_utf8(wt, g_vein_target_title, sizeof(g_vein_target_title));

    /* 1) Classic Win32 control text (Notepad, many dialogs) */
    vein_win32_tree(target, dst, cap);
    /* 2) UI Automation tree (modern apps, browsers, WPF, WinUI) */
    vein_uia_hwnd(target, dst, cap);

    return dst[0] != 0;
}

static void do_read_vein(void) {
    if (!vein_read_target(g_vein, sizeof(g_vein))) {
        snprintf(g_status, sizeof(g_status),
                 "No text yet. Click Notepad/browser first (status will show Target), then Read Vein.");
        g_vein[0] = 0;
        return;
    }
    snprintf(g_status, sizeof(g_status), "Vein ← %s (%d chars, no screenshot).",
             g_vein_target_title[0] ? g_vein_target_title : "window", (int)strlen(g_vein));
}

/* --- HTTP POST JSON to settings API --- */

static int http_post_json(const char *url, const char *json, char *out, int cap) {
    if (out && cap > 0) out[0] = 0;
    if (!url || !url[0] || !json) return 0;
    HINTERNET ses = InternetOpenA("BuraaqFlowdesk/1", INTERNET_OPEN_TYPE_PRECONFIG, NULL, NULL, 0);
    if (!ses) return 0;
    DWORD timeout = 8000;
    InternetSetOptionA(ses, INTERNET_OPTION_CONNECT_TIMEOUT, &timeout, sizeof(timeout));
    InternetSetOptionA(ses, INTERNET_OPTION_RECEIVE_TIMEOUT, &timeout, sizeof(timeout));

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
    if (slash) strncpy(path, slash, sizeof(path) - 1);

    HINTERNET con = InternetConnectA(ses, host, (INTERNET_PORT)port, NULL, NULL, INTERNET_SERVICE_HTTP, 0, 0);
    if (!con) {
        InternetCloseHandle(ses);
        return 0;
    }
    DWORD flags = INTERNET_FLAG_RELOAD | INTERNET_FLAG_NO_CACHE_WRITE | INTERNET_FLAG_NO_UI;
    if (https) flags |= INTERNET_FLAG_SECURE;
    HINTERNET req = HttpOpenRequestA(con, "POST", path, NULL, NULL, NULL, flags, 0);
    if (!req) {
        InternetCloseHandle(con);
        InternetCloseHandle(ses);
        return 0;
    }
    const char *hdrs = "Content-Type: application/json\r\nAccept: application/json\r\n";
    int ok = HttpSendRequestA(req, hdrs, (DWORD)strlen(hdrs), (LPVOID)json, (DWORD)strlen(json));
    if (ok && out && cap > 0) {
        int n = 0;
        DWORD got = 0;
        while (n + 1 < cap) {
            if (!InternetReadFile(req, out + n, (DWORD)(cap - 1 - n), &got) || got == 0) break;
            n += (int)got;
        }
        out[n] = 0;
    }
    InternetCloseHandle(req);
    InternetCloseHandle(con);
    InternetCloseHandle(ses);
    return ok;
}

static void json_escape(const char *in, char *out, int cap) {
    int j = 0;
    for (int i = 0; in && in[i] && j + 2 < cap; i++) {
        unsigned char c = (unsigned char)in[i];
        if (c == '"' || c == '\\') {
            if (j + 3 >= cap) break;
            out[j++] = '\\';
            out[j++] = (char)c;
        } else if (c < 0x20) {
            if (c == '\n') {
                if (j + 3 >= cap) break;
                out[j++] = '\\';
                out[j++] = 'n';
            } else if (c == '\r') {
                continue;
            } else if (c == '\t') {
                if (j + 3 >= cap) break;
                out[j++] = '\\';
                out[j++] = 't';
            }
        } else {
            out[j++] = (char)c;
        }
    }
    out[j] = 0;
}

static void extract_context_field(const char *json, char *out, int cap) {
    out[0] = 0;
    if (!json) return;
    const char *keys[] = {"\"context\"", "\"result\"", "\"message\"", "\"body\"", NULL};
    for (int k = 0; keys[k]; k++) {
        const char *p = strstr(json, keys[k]);
        if (!p) continue;
        p = strchr(p + strlen(keys[k]), '"');
        if (!p) continue;
        p++;
        int j = 0;
        while (*p && *p != '"' && j + 1 < cap) {
            if (*p == '\\' && p[1]) {
                p++;
                if (*p == 'n') out[j++] = '\n';
                else out[j++] = *p;
                p++;
                continue;
            }
            out[j++] = *p++;
        }
        out[j] = 0;
        if (out[0]) return;
    }
    /* raw body fallback */
    strncpy(out, json, (size_t)cap - 1);
    out[cap - 1] = 0;
}

static void local_context_fallback(const char *text, char *out, int cap) {
    /* Offline demo: structured context without a server */
    snprintf(out, (size_t)cap,
             "Local Vein context (API unreachable)\n"
             "— Source: UI Automation accessibility tree\n"
             "— Capture: none  OCR: none\n"
             "— Chars: %d\n"
             "— Preview: %.240s%s",
             text ? (int)strlen(text) : 0, text ? text : "",
             text && strlen(text) > 240 ? "…" : "");
}

static void do_send_api(void) {
    if (!g_vein[0]) {
        strncpy(g_status, "Read Vein first, then send.", sizeof(g_status) - 1);
        return;
    }
    char esc[FD_VAL * 2];
    json_escape(g_vein, esc, sizeof(esc));
    char body[FD_VAL * 2 + 64];
    snprintf(body, sizeof(body), "{\"text\": \"%s\"}", esc);
    char resp[FD_CTX];
    strncpy(g_status, "Calling API…", sizeof(g_status) - 1);
    if (http_post_json(g_api, body, resp, sizeof(resp)) && resp[0]) {
        extract_context_field(resp, g_context, sizeof(g_context));
        strncpy(g_status, "API responded with context.", sizeof(g_status) - 1);
    } else {
        local_context_fallback(g_vein, g_context, sizeof(g_context));
        strncpy(g_status, "API unreachable — showing local Vein context.", sizeof(g_status) - 1);
    }
}

static void paint(HDC hdc, int w, int h) {
    ensure_fonts();
    layout(w, h);
    RECT full = {0, 0, w, h};
    fill_rect(hdc, full, col_bg());

    RECT chrome = {0, 0, w, px(FD_TITLEBAR)};
    fill_rect(hdc, chrome, col_chrome());
    RECT title_r = {px(16), 0, w - px(100), px(FD_TITLEBAR)};
    draw_text(hdc, g_font_title, col_fg(), title_r, g_title, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    draw_btn(hdc, g_rc_max, g_maximized ? "❐" : "□", g_hot == 2, 0);
    draw_btn(hdc, g_rc_close, "✕", g_hot == 1, 0);

    draw_btn(hdc, g_rc_flow, "Flow", g_hot == 3 || g_view == VIEW_FLOW, g_view == VIEW_FLOW);
    draw_btn(hdc, g_rc_settings, "Settings", g_hot == 4 || g_view == VIEW_SETTINGS,
             g_view == VIEW_SETTINGS);

    int top = px(FD_TITLEBAR) + px(FD_NAV);
    RECT body = {px(16), top, w - px(16), h - px(72)};
    fill_rect(hdc, body, col_panel());
    frame_rect(hdc, body, RGB(51, 65, 85));

    if (g_view == VIEW_SETTINGS) {
        RECT lab = {body.left + px(16), body.top + px(20), body.right - px(16), body.top + px(48)};
        draw_text(hdc, g_font_body, col_fg(), lab, "Context API URL (POST JSON {\"text\":…})",
                  DT_LEFT | DT_TOP);
        fill_rect(hdc, g_rc_api, RGB(15, 23, 42));
        frame_rect(hdc, g_rc_api, g_api_focus ? col_primary() : RGB(71, 85, 105));
        RECT ar = g_rc_api;
        ar.left += px(10);
        ar.right -= px(10);
        draw_text(hdc, g_font_body, col_fg(), ar, g_api, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS);

        RECT note = {body.left + px(16), g_rc_api.bottom + px(20), body.right - px(16), body.bottom - px(16)};
        draw_text(hdc, g_font_small, col_muted(), note,
                  "Vein never grabs pixels. It walks the Windows UI Automation tree "
                  "(Name, Value, Text patterns) of the focused app — the native path "
                  "assistive tech uses. Point this URL at your Keel/context service.",
                  DT_LEFT | DT_TOP | DT_WORDBREAK);
    } else {
        RECT lab1 = {body.left + px(16), body.top + px(12), body.right - px(16), body.top + px(36)};
        char labbuf[320];
        if (g_vein_target && IsWindow(g_vein_target) && g_vein_target_title[0])
            snprintf(labbuf, sizeof(labbuf), "Vein target: %s", g_vein_target_title);
        else
            snprintf(labbuf, sizeof(labbuf), "Vein target: (click Notepad/browser first)");
        draw_text(hdc, g_font_small, col_muted(), lab1, labbuf, DT_LEFT | DT_TOP);
        RECT vein_box = {body.left + px(16), body.top + px(36), body.right - px(16),
                         body.top + (body.bottom - body.top) / 2 - px(8)};
        fill_rect(hdc, vein_box, RGB(15, 23, 42));
        frame_rect(hdc, vein_box, RGB(71, 85, 105));
        RECT vr = vein_box;
        vr.left += px(10);
        vr.top += px(8);
        vr.right -= px(10);
        vr.bottom -= px(8);
        draw_text(hdc, g_font_small, col_fg(), vr, g_vein[0] ? g_vein : "(empty — press Read Vein)",
                  DT_LEFT | DT_TOP | DT_WORDBREAK);

        RECT lab2 = {body.left + px(16), vein_box.bottom + px(8), body.right - px(16),
                     vein_box.bottom + px(28)};
        draw_text(hdc, g_font_small, col_muted(), lab2, "API context", DT_LEFT | DT_TOP);
        RECT ctx_box = {body.left + px(16), lab2.bottom + px(4), body.right - px(16), body.bottom - px(12)};
        fill_rect(hdc, ctx_box, RGB(15, 23, 42));
        frame_rect(hdc, ctx_box, col_border());
        RECT cr = ctx_box;
        cr.left += px(10);
        cr.top += px(8);
        cr.right -= px(10);
        cr.bottom -= px(8);
        draw_text(hdc, g_font_small, col_fg(), cr, g_context, DT_LEFT | DT_TOP | DT_WORDBREAK);

        draw_btn(hdc, g_rc_read, "Read Vein", g_hot == 5, 1);
        draw_btn(hdc, g_rc_send, "Send → API", g_hot == 6, 0);
    }

    RECT st = {px(16), h - px(28), w - px(16), h - px(8)};
    draw_text(hdc, g_font_small, col_muted(), st, g_status, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS);
}

static void update_hot(int x, int y) {
    int n = 0;
    if (hit(g_rc_close, x, y)) n = 1;
    else if (hit(g_rc_max, x, y)) n = 2;
    else if (hit(g_rc_flow, x, y)) n = 3;
    else if (hit(g_rc_settings, x, y)) n = 4;
    else if (g_view == VIEW_FLOW && hit(g_rc_read, x, y)) n = 5;
    else if (g_view == VIEW_FLOW && hit(g_rc_send, x, y)) n = 6;
    if (n != g_hot) {
        g_hot = n;
        InvalidateRect(g_hwnd, NULL, FALSE);
    }
}

static void on_click(int x, int y) {
    if (hit(g_rc_close, x, y)) {
        DestroyWindow(g_hwnd);
        return;
    }
    if (hit(g_rc_max, x, y)) {
        if (g_maximized) {
            ShowWindow(g_hwnd, SW_RESTORE);
            g_maximized = 0;
        } else {
            ShowWindow(g_hwnd, SW_MAXIMIZE);
            g_maximized = 1;
        }
        return;
    }
    if (hit(g_rc_flow, x, y)) {
        g_view = VIEW_FLOW;
        g_api_focus = 0;
        InvalidateRect(g_hwnd, NULL, FALSE);
        return;
    }
    if (hit(g_rc_settings, x, y)) {
        g_view = VIEW_SETTINGS;
        g_api_focus = 1;
        InvalidateRect(g_hwnd, NULL, FALSE);
        return;
    }
    if (g_view == VIEW_SETTINGS && hit(g_rc_api, x, y)) {
        g_api_focus = 1;
        InvalidateRect(g_hwnd, NULL, FALSE);
        return;
    }
    g_api_focus = 0;
    if (g_view == VIEW_FLOW && hit(g_rc_read, x, y)) {
        do_read_vein();
        InvalidateRect(g_hwnd, NULL, FALSE);
        return;
    }
    if (g_view == VIEW_FLOW && hit(g_rc_send, x, y)) {
        do_send_api();
        InvalidateRect(g_hwnd, NULL, FALSE);
        return;
    }
}

static void on_char(wchar_t ch) {
    if (g_view != VIEW_SETTINGS || !g_api_focus) return;
    if (ch == 8) { /* backspace */
        size_t n = strlen(g_api);
        if (n) g_api[n - 1] = 0;
        InvalidateRect(g_hwnd, NULL, FALSE);
        return;
    }
    if (ch < 32) return;
    char utf[8];
    wchar_t ws[2] = {ch, 0};
    wide_to_utf8(ws, utf, sizeof(utf));
    size_t n = strlen(g_api);
    size_t add = strlen(utf);
    if (n + add < sizeof(g_api)) {
        memcpy(g_api + n, utf, add + 1);
        InvalidateRect(g_hwnd, NULL, FALSE);
    }
}

static LRESULT CALLBACK fd_wnd(HWND hwnd, UINT msg, WPARAM wparam, LPARAM lparam) {
    switch (msg) {
    case WM_CREATE:
        g_hwnd = hwnd;
        SetTimer(hwnd, 1, 200, NULL);
        return 0;
    case WM_TIMER:
        if (wparam == 1) {
            char prev[256];
            memcpy(prev, g_vein_target_title, sizeof(prev));
            vein_track_foreground();
            if (g_view == VIEW_FLOW && strcmp(prev, g_vein_target_title) != 0)
                InvalidateRect(hwnd, NULL, FALSE);
        }
        return 0;
    case WM_NCCALCSIZE:
        if (wparam) return 0; /* client = full window (borderless) */
        break;
    case WM_NCHITTEST: {
        POINT pt = {GET_X_LPARAM(lparam), GET_Y_LPARAM(lparam)};
        ScreenToClient(hwnd, &pt);
        RECT rc;
        GetClientRect(hwnd, &rc);
        layout(rc.right, rc.bottom);
        if (hit(g_rc_close, pt.x, pt.y) || hit(g_rc_max, pt.x, pt.y)) return HTCLIENT;
        if (pt.y < px(FD_TITLEBAR)) return HTCAPTION; /* drag */
        return HTCLIENT;
    }
    case WM_DPICHANGED: {
        g_dpi = HIWORD(wparam);
        if (g_font_title) {
            DeleteObject(g_font_title);
            DeleteObject(g_font_body);
            DeleteObject(g_font_small);
            DeleteObject(g_font_btn);
            g_font_title = g_font_body = g_font_small = g_font_btn = NULL;
        }
        RECT *s = (RECT *)lparam;
        SetWindowPos(hwnd, NULL, s->left, s->top, s->right - s->left, s->bottom - s->top,
                     SWP_NOZORDER | SWP_NOACTIVATE);
        return 0;
    }
    case WM_PAINT: {
        PAINTSTRUCT ps;
        HDC hdc = BeginPaint(hwnd, &ps);
        RECT rc;
        GetClientRect(hwnd, &rc);
        HDC mem = CreateCompatibleDC(hdc);
        HBITMAP bmp = CreateCompatibleBitmap(hdc, rc.right, rc.bottom);
        HGDIOBJ old = SelectObject(mem, bmp);
        paint(mem, rc.right, rc.bottom);
        BitBlt(hdc, 0, 0, rc.right, rc.bottom, mem, 0, 0, SRCCOPY);
        SelectObject(mem, old);
        DeleteObject(bmp);
        DeleteDC(mem);
        EndPaint(hwnd, &ps);
        return 0;
    }
    case WM_MOUSEMOVE:
        update_hot(GET_X_LPARAM(lparam), GET_Y_LPARAM(lparam));
        return 0;
    case WM_LBUTTONDOWN:
        SetCapture(hwnd);
        on_click(GET_X_LPARAM(lparam), GET_Y_LPARAM(lparam));
        return 0;
    case WM_LBUTTONUP:
        ReleaseCapture();
        return 0;
    case WM_CHAR:
        on_char((wchar_t)wparam);
        return 0;
    case WM_KEYDOWN:
        if (wparam == VK_ESCAPE) DestroyWindow(hwnd);
        return 0;
    case WM_DESTROY:
        KillTimer(hwnd, 1);
        g_running = 0;
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProcW(hwnd, msg, wparam, lparam);
}

static int fd_open(void) {
    HINSTANCE inst = GetModuleHandleW(NULL);
    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    WNDCLASSW wc;
    memset(&wc, 0, sizeof(wc));
    wc.lpfnWndProc = fd_wnd;
    wc.hInstance = inst;
    wc.hCursor = LoadCursor(NULL, IDC_ARROW);
    wc.hbrBackground = (HBRUSH)GetStockObject(BLACK_BRUSH);
    wc.lpszClassName = L"BuraaqFlowdesk";
    RegisterClassW(&wc);
    wchar_t wt[160];
    utf8_to_wide(g_title, wt, 160);
    int sw = GetSystemMetrics(SM_CXSCREEN);
    int sh = GetSystemMetrics(SM_CYSCREEN);
    HWND hwnd = CreateWindowExW(
        WS_EX_APPWINDOW, L"BuraaqFlowdesk", wt,
        WS_POPUP | WS_THICKFRAME | WS_MINIMIZEBOX | WS_VISIBLE,
        (sw - g_cw) / 2, (sh - g_ch) / 2, g_cw, g_ch, NULL, NULL, inst, NULL);
    if (!hwnd) return 0;
    g_hwnd = hwnd;
    ShowWindow(hwnd, SW_SHOW);
    UpdateWindow(hwnd);
    return 1;
}

static int32_t fd_loop(void) {
    if (!fd_open()) {
        fprintf(stderr, "flowdesk: could not create window\n");
        return 0;
    }
    g_running = 1;
    MSG msg;
    while (g_running && GetMessageW(&msg, NULL, 0, 0) > 0) {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
    return 1;
}

int32_t buraaq_flow_desk(const char *title, int32_t w, int32_t h) {
    if (title && title[0]) strncpy(g_title, title, sizeof(g_title) - 1);
    if (w > 640) g_cw = w;
    if (h > 480) g_ch = h;
    return 1;
}

int32_t buraaq_flow_show(void) { return fd_loop(); }

#endif /* _WIN32 */
