/* std.hold — named columns of cargo. Not a spreadsheet product; a table you can stow. */
#include "buraaq_std.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define BQ_HOLD_MAX 64
#define BQ_HOLD_COLS 48
#define BQ_HOLD_START 16

typedef struct {
    int in_use;
    int32_t ncols;
    int32_t nrows;
    int32_t cap;
    char *names[BQ_HOLD_COLS];
    char **cells;
} BqHold;

static BqHold g_holds[BQ_HOLD_MAX];

static char *dup_str(const char *s) {
    if (!s) s = "";
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

static BqHold *hold_get(int32_t id) {
    if (id < 0 || id >= BQ_HOLD_MAX || !g_holds[id].in_use) return NULL;
    return &g_holds[id];
}

static int col_index(const BqHold *h, const char *name) {
    if (!name) return -1;
    for (int32_t i = 0; i < h->ncols; i++) {
        if (h->names[i] && strcmp(h->names[i], name) == 0) return (int)i;
    }
    return -1;
}

static int parse_csv_fields(const char *line, char **out, int32_t max) {
    int32_t n = 0;
    const char *p = line ? line : "";
    while (*p && n < max) {
        while (*p == ' ') p++;
        char buf[4096];
        size_t b = 0;
        if (*p == '"') {
            p++;
            while (*p && b + 1 < sizeof(buf)) {
                if (*p == '"' && p[1] == '"') {
                    buf[b++] = '"';
                    p += 2;
                    continue;
                }
                if (*p == '"') {
                    p++;
                    break;
                }
                buf[b++] = *p++;
            }
        } else {
            while (*p && *p != ',' && *p != '\n' && *p != '\r' && b + 1 < sizeof(buf)) {
                buf[b++] = *p++;
            }
        }
        buf[b] = 0;
        out[n++] = dup_str(buf);
        if (*p == ',') p++;
        if (*p == '\r') p++;
        if (*p == '\n') break;
    }
    return n;
}

static int32_t hold_slot(void) {
    for (int32_t i = 0; i < BQ_HOLD_MAX; i++) {
        if (!g_holds[i].in_use) return i;
    }
    return -1;
}

static int hold_grow(BqHold *h) {
    int32_t next = h->cap == 0 ? BQ_HOLD_START : h->cap * 2;
    char **cells = (char **)realloc(h->cells, (size_t)next * (size_t)h->ncols * sizeof(char *));
    if (!cells) return 0;
    size_t old = (size_t)h->cap * (size_t)h->ncols;
    size_t neu = (size_t)next * (size_t)h->ncols;
    for (size_t i = old; i < neu; i++) cells[i] = NULL;
    h->cells = cells;
    h->cap = next;
    return 1;
}

static int32_t hold_from_names(char **names, int32_t ncols) {
    int32_t id = hold_slot();
    if (id < 0 || ncols <= 0 || ncols > BQ_HOLD_COLS) return -1;
    BqHold *h = &g_holds[id];
    memset(h, 0, sizeof(*h));
    h->in_use = 1;
    h->ncols = ncols;
    for (int32_t i = 0; i < ncols; i++) h->names[i] = names[i];
    if (!hold_grow(h)) {
        h->in_use = 0;
        return -1;
    }
    return id;
}

int32_t buraaq_hold_new(const char *cols) {
    char *parts[BQ_HOLD_COLS];
    int32_t n = parse_csv_fields(cols, parts, BQ_HOLD_COLS);
    if (n <= 0) return -1;
    int32_t id = hold_from_names(parts, n);
    if (id < 0) {
        for (int32_t i = 0; i < n; i++) free(parts[i]);
    }
    return id;
}

int32_t buraaq_hold_stow(int32_t id, const char *row) {
    BqHold *h = hold_get(id);
    if (!h) return 0;
    if (h->nrows >= h->cap && !hold_grow(h)) return 0;
    char *parts[BQ_HOLD_COLS];
    int32_t n = parse_csv_fields(row, parts, BQ_HOLD_COLS);
    for (int32_t c = 0; c < h->ncols; c++) {
        char *cell = c < n && parts[c] ? parts[c] : dup_str("");
        h->cells[(size_t)h->nrows * (size_t)h->ncols + (size_t)c] = cell;
    }
    for (int32_t c = h->ncols; c < n; c++) free(parts[c]);
    h->nrows++;
    return 1;
}

int32_t buraaq_hold_rows(int32_t id) {
    BqHold *h = hold_get(id);
    return h ? h->nrows : 0;
}

int32_t buraaq_hold_cols(int32_t id) {
    BqHold *h = hold_get(id);
    return h ? h->ncols : 0;
}

char *buraaq_hold_pick(int32_t id, const char *col, int32_t row) {
    BqHold *h = hold_get(id);
    int c = h ? col_index(h, col) : -1;
    if (!h || c < 0 || row < 0 || row >= h->nrows) return dup_str("");
    const char *v = h->cells[(size_t)row * (size_t)h->ncols + (size_t)c];
    return dup_str(v ? v : "");
}

int32_t buraaq_hold_keep(int32_t id, const char *col, const char *value) {
    BqHold *h = hold_get(id);
    int c = h ? col_index(h, col) : -1;
    if (!h || c < 0) return -1;
    char *names[BQ_HOLD_COLS];
    for (int32_t i = 0; i < h->ncols; i++) names[i] = dup_str(h->names[i]);
    int32_t nid = hold_from_names(names, h->ncols);
    if (nid < 0) {
        for (int32_t i = 0; i < h->ncols; i++) free(names[i]);
        return -1;
    }
    const char *want = value ? value : "";
    for (int32_t r = 0; r < h->nrows; r++) {
        const char *v = h->cells[(size_t)r * (size_t)h->ncols + (size_t)c];
        if (v && strcmp(v, want) == 0) {
            size_t cap = (size_t)h->ncols * 64 + 8;
            char *line = (char *)malloc(cap);
            if (!line) continue;
            line[0] = 0;
            size_t n = 0;
            for (int32_t i = 0; i < h->ncols; i++) {
                const char *cell = h->cells[(size_t)r * (size_t)h->ncols + (size_t)i];
                int w = snprintf(line + n, cap - n, "%s%s", i ? "," : "", cell ? cell : "");
                if (w < 0) break;
                n += (size_t)w;
            }
            buraaq_hold_stow(nid, line);
            free(line);
        }
    }
    return nid;
}

static double col_fold(int32_t id, const char *col, int mean) {
    BqHold *h = hold_get(id);
    int c = h ? col_index(h, col) : -1;
    if (!h || c < 0 || h->nrows == 0) return 0.0;
    double s = 0.0;
    int32_t n = 0;
    for (int32_t r = 0; r < h->nrows; r++) {
        const char *v = h->cells[(size_t)r * (size_t)h->ncols + (size_t)c];
        if (!v || !v[0]) continue;
        s += strtod(v, NULL);
        n++;
    }
    if (mean) return n ? s / (double)n : 0.0;
    return s;
}

double buraaq_hold_col_mean(int32_t id, const char *col) { return col_fold(id, col, 1); }
double buraaq_hold_col_sum(int32_t id, const char *col) { return col_fold(id, col, 0); }

int32_t buraaq_hold_order(int32_t id, const char *col) {
    BqHold *h = hold_get(id);
    int c = h ? col_index(h, col) : -1;
    if (!h || c < 0) return 0;
    for (int32_t i = 0; i + 1 < h->nrows; i++) {
        for (int32_t j = 0; j + 1 < h->nrows - i; j++) {
            const char *a = h->cells[(size_t)j * (size_t)h->ncols + (size_t)c];
            const char *b = h->cells[(size_t)(j + 1) * (size_t)h->ncols + (size_t)c];
            if (!a) a = "";
            if (!b) b = "";
            char *ea = NULL, *eb = NULL;
            double da = strtod(a, &ea), db = strtod(b, &eb);
            int numeric = ea != a && eb != b && *ea == 0 && *eb == 0;
            int swap = numeric ? (da > db) : (strcmp(a, b) > 0);
            if (!swap) continue;
            for (int32_t k = 0; k < h->ncols; k++) {
                size_t ia = (size_t)j * (size_t)h->ncols + (size_t)k;
                size_t ib = (size_t)(j + 1) * (size_t)h->ncols + (size_t)k;
                char *tmp = h->cells[ia];
                h->cells[ia] = h->cells[ib];
                h->cells[ib] = tmp;
            }
        }
    }
    return 1;
}

int32_t buraaq_hold_from_csv(const char *path) {
    char *raw = buraaq_file_read(path);
    if (!raw) return -1;
    char *line = raw;
    int32_t id = -1;
    while (line && *line) {
        char *nl = strpbrk(line, "\r\n");
        if (nl) {
            char save = *nl;
            *nl = 0;
            if (id < 0) id = buraaq_hold_new(line);
            else if (line[0]) buraaq_hold_stow(id, line);
            *nl = save;
            line = nl + 1;
            if (save == '\r' && *line == '\n') line++;
        } else {
            if (id < 0) id = buraaq_hold_new(line);
            else if (line[0]) buraaq_hold_stow(id, line);
            break;
        }
    }
    free(raw);
    return id;
}

static void csv_write_field(FILE *f, const char *s) {
    if (!s) s = "";
    int quote = 0;
    for (const char *p = s; *p; p++) {
        if (*p == ',' || *p == '"' || *p == '\n' || *p == '\r') {
            quote = 1;
            break;
        }
    }
    if (!quote) {
        fputs(s, f);
        return;
    }
    fputc('"', f);
    for (const char *p = s; *p; p++) {
        if (*p == '"') fputc('"', f);
        fputc(*p, f);
    }
    fputc('"', f);
}

int32_t buraaq_hold_to_csv(int32_t id, const char *path) {
    BqHold *h = hold_get(id);
    if (!h || !path) return 0;
    FILE *f = fopen(path, "wb");
    if (!f) return 0;
    for (int32_t c = 0; c < h->ncols; c++) {
        if (c) fputc(',', f);
        csv_write_field(f, h->names[c]);
    }
    fputc('\n', f);
    for (int32_t r = 0; r < h->nrows; r++) {
        for (int32_t c = 0; c < h->ncols; c++) {
            if (c) fputc(',', f);
            csv_write_field(f, h->cells[(size_t)r * (size_t)h->ncols + (size_t)c]);
        }
        fputc('\n', f);
    }
    fclose(f);
    return 1;
}

char *buraaq_hold_view(int32_t id) {
    BqHold *h = hold_get(id);
    if (!h) return dup_str("");
    size_t cap = 256;
    char *out = (char *)malloc(cap);
    if (!out) return NULL;
    size_t n = 0;
    out[0] = 0;
    for (int32_t r = -1; r < h->nrows; r++) {
        for (int32_t c = 0; c < h->ncols; c++) {
            const char *cell = r < 0 ? h->names[c]
                                     : h->cells[(size_t)r * (size_t)h->ncols + (size_t)c];
            size_t need = n + (cell ? strlen(cell) : 0) + 4;
            if (need >= cap) {
                cap *= 2;
                if (cap < need) cap = need + 64;
                char *p = (char *)realloc(out, cap);
                if (!p) {
                    free(out);
                    return NULL;
                }
                out = p;
            }
            int w = snprintf(out + n, cap - n, "%s%s", c ? "\t" : "", cell ? cell : "");
            if (w < 0) break;
            n += (size_t)w;
        }
        if (r + 1 < h->nrows || r == -1) {
            if (n + 2 >= cap) {
                cap *= 2;
                char *p = (char *)realloc(out, cap);
                if (!p) {
                    free(out);
                    return NULL;
                }
                out = p;
            }
            out[n++] = '\n';
            out[n] = 0;
        }
    }
    return out;
}
