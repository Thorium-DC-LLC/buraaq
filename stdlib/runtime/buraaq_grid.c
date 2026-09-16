/* std.grid — numeric arrays. Complexity stays here; source stays zeros/at/dot. */
#include "buraaq_std.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define BQ_GRID_MAX 128
#define BQ_GRID_CELLS 1000000

typedef struct {
    int in_use;
    int32_t rows;
    int32_t cols;
    double *data;
} BqGrid;

static BqGrid g_grids[BQ_GRID_MAX];

static int64_t grid_n(const BqGrid *g) {
    return (int64_t)g->rows * (int64_t)g->cols;
}

static BqGrid *grid_get(int32_t id) {
    if (id < 0 || id >= BQ_GRID_MAX || !g_grids[id].in_use) return NULL;
    return &g_grids[id];
}

static int32_t grid_new(int32_t rows, int32_t cols, double fill) {
    if (rows <= 0 || cols <= 0) return -1;
    int64_t n = (int64_t)rows * (int64_t)cols;
    if (n > BQ_GRID_CELLS) return -1;
    for (int32_t i = 0; i < BQ_GRID_MAX; i++) {
        if (g_grids[i].in_use) continue;
        double *d = (double *)malloc((size_t)n * sizeof(double));
        if (!d) return -1;
        for (int64_t k = 0; k < n; k++) d[k] = fill;
        g_grids[i].in_use = 1;
        g_grids[i].rows = rows;
        g_grids[i].cols = cols;
        g_grids[i].data = d;
        return i;
    }
    return -1;
}

static int32_t grid_copy_shape(const BqGrid *src) {
    return grid_new(src->rows, src->cols, 0.0);
}

int32_t buraaq_grid_zeros(int32_t rows, int32_t cols) { return grid_new(rows, cols, 0.0); }
int32_t buraaq_grid_ones(int32_t rows, int32_t cols) { return grid_new(rows, cols, 1.0); }
int32_t buraaq_grid_fill(int32_t rows, int32_t cols, double v) { return grid_new(rows, cols, v); }

int32_t buraaq_grid_eye(int32_t n) {
    int32_t id = grid_new(n, n, 0.0);
    BqGrid *g = grid_get(id);
    if (!g) return -1;
    for (int32_t i = 0; i < n; i++) g->data[(size_t)i * (size_t)n + (size_t)i] = 1.0;
    return id;
}

int32_t buraaq_grid_linspace(double a, double b, int32_t n) {
    if (n <= 0) return -1;
    int32_t id = grid_new(1, n, 0.0);
    BqGrid *g = grid_get(id);
    if (!g) return -1;
    if (n == 1) {
        g->data[0] = a;
        return id;
    }
    double step = (b - a) / (double)(n - 1);
    for (int32_t i = 0; i < n; i++) g->data[i] = a + step * (double)i;
    return id;
}

int32_t buraaq_grid_row(const char *csv) {
    if (!csv) csv = "";
    int32_t count = 1;
    for (const char *p = csv; *p; p++) {
        if (*p == ',') count++;
    }
    int32_t id = grid_new(1, count, 0.0);
    BqGrid *g = grid_get(id);
    if (!g) return -1;
    int32_t i = 0;
    const char *p = csv;
    while (i < count) {
        char *end = NULL;
        g->data[i++] = strtod(p, &end);
        if (!end || *end == 0) break;
        if (*end == ',') p = end + 1;
        else p = end;
    }
    return id;
}

int32_t buraaq_grid_nrows(int32_t id) {
    BqGrid *g = grid_get(id);
    return g ? g->rows : 0;
}

int32_t buraaq_grid_ncols(int32_t id) {
    BqGrid *g = grid_get(id);
    return g ? g->cols : 0;
}

double buraaq_grid_at(int32_t id, int32_t r, int32_t c) {
    BqGrid *g = grid_get(id);
    if (!g || r < 0 || c < 0 || r >= g->rows || c >= g->cols) return 0.0;
    return g->data[(size_t)r * (size_t)g->cols + (size_t)c];
}

int32_t buraaq_grid_put_at(int32_t id, int32_t r, int32_t c, double v) {
    BqGrid *g = grid_get(id);
    if (!g || r < 0 || c < 0 || r >= g->rows || c >= g->cols) return 0;
    g->data[(size_t)r * (size_t)g->cols + (size_t)c] = v;
    return 1;
}

int32_t buraaq_grid_plus(int32_t a, int32_t b) {
    BqGrid *ga = grid_get(a), *gb = grid_get(b);
    if (!ga || !gb || ga->rows != gb->rows || ga->cols != gb->cols) return -1;
    int32_t id = grid_copy_shape(ga);
    BqGrid *out = grid_get(id);
    if (!out) return -1;
    int64_t n = grid_n(ga);
    for (int64_t i = 0; i < n; i++) out->data[i] = ga->data[i] + gb->data[i];
    return id;
}

int32_t buraaq_grid_times(int32_t a, int32_t b) {
    BqGrid *ga = grid_get(a), *gb = grid_get(b);
    if (!ga || !gb || ga->rows != gb->rows || ga->cols != gb->cols) return -1;
    int32_t id = grid_copy_shape(ga);
    BqGrid *out = grid_get(id);
    if (!out) return -1;
    int64_t n = grid_n(ga);
    for (int64_t i = 0; i < n; i++) out->data[i] = ga->data[i] * gb->data[i];
    return id;
}

int32_t buraaq_grid_scale(int32_t id, double k) {
    BqGrid *g = grid_get(id);
    if (!g) return -1;
    int32_t out_id = grid_copy_shape(g);
    BqGrid *out = grid_get(out_id);
    if (!out) return -1;
    int64_t n = grid_n(g);
    for (int64_t i = 0; i < n; i++) out->data[i] = g->data[i] * k;
    return out_id;
}

int32_t buraaq_grid_matmul(int32_t a, int32_t b) {
    BqGrid *ga = grid_get(a), *gb = grid_get(b);
    if (!ga || !gb || ga->cols != gb->rows) return -1;
    int32_t id = grid_new(ga->rows, gb->cols, 0.0);
    BqGrid *out = grid_get(id);
    if (!out) return -1;
    for (int32_t i = 0; i < ga->rows; i++) {
        for (int32_t k = 0; k < ga->cols; k++) {
            double aik = ga->data[(size_t)i * (size_t)ga->cols + (size_t)k];
            for (int32_t j = 0; j < gb->cols; j++) {
                out->data[(size_t)i * (size_t)out->cols + (size_t)j] +=
                    aik * gb->data[(size_t)k * (size_t)gb->cols + (size_t)j];
            }
        }
    }
    return id;
}

double buraaq_grid_dot(int32_t a, int32_t b) {
    BqGrid *ga = grid_get(a), *gb = grid_get(b);
    if (!ga || !gb) return 0.0;
    int64_t na = grid_n(ga), nb = grid_n(gb);
    if (na != nb) return 0.0;
    double s = 0.0;
    for (int64_t i = 0; i < na; i++) s += ga->data[i] * gb->data[i];
    return s;
}

int32_t buraaq_grid_transpose(int32_t id) {
    BqGrid *g = grid_get(id);
    if (!g) return -1;
    int32_t out_id = grid_new(g->cols, g->rows, 0.0);
    BqGrid *out = grid_get(out_id);
    if (!out) return -1;
    for (int32_t i = 0; i < g->rows; i++) {
        for (int32_t j = 0; j < g->cols; j++) {
            out->data[(size_t)j * (size_t)out->cols + (size_t)i] =
                g->data[(size_t)i * (size_t)g->cols + (size_t)j];
        }
    }
    return out_id;
}

double buraaq_grid_sum(int32_t id) {
    BqGrid *g = grid_get(id);
    if (!g) return 0.0;
    double s = 0.0;
    int64_t n = grid_n(g);
    for (int64_t i = 0; i < n; i++) s += g->data[i];
    return s;
}

double buraaq_grid_mean(int32_t id) {
    BqGrid *g = grid_get(id);
    if (!g) return 0.0;
    int64_t n = grid_n(g);
    if (n <= 0) return 0.0;
    return buraaq_grid_sum(id) / (double)n;
}

double buraaq_grid_least(int32_t id) {
    BqGrid *g = grid_get(id);
    if (!g) return 0.0;
    int64_t n = grid_n(g);
    if (n <= 0) return 0.0;
    double m = g->data[0];
    for (int64_t i = 1; i < n; i++) if (g->data[i] < m) m = g->data[i];
    return m;
}

double buraaq_grid_most(int32_t id) {
    BqGrid *g = grid_get(id);
    if (!g) return 0.0;
    int64_t n = grid_n(g);
    if (n <= 0) return 0.0;
    double m = g->data[0];
    for (int64_t i = 1; i < n; i++) if (g->data[i] > m) m = g->data[i];
    return m;
}

double buraaq_grid_norm(int32_t id) {
    return sqrt(buraaq_grid_dot(id, id));
}

static int32_t grid_map(int32_t id, double (*fn)(double)) {
    BqGrid *g = grid_get(id);
    if (!g) return -1;
    int32_t out_id = grid_copy_shape(g);
    BqGrid *out = grid_get(out_id);
    if (!out) return -1;
    int64_t n = grid_n(g);
    for (int64_t i = 0; i < n; i++) out->data[i] = fn(g->data[i]);
    return out_id;
}

int32_t buraaq_grid_sin_all(int32_t id) { return grid_map(id, sin); }
int32_t buraaq_grid_cos_all(int32_t id) { return grid_map(id, cos); }

char *buraaq_grid_view(int32_t id) {
    BqGrid *g = grid_get(id);
    if (!g) {
        char *e = (char *)malloc(1);
        if (e) e[0] = 0;
        return e;
    }
    size_t cap = (size_t)grid_n(g) * 24 + (size_t)g->rows * 2 + 8;
    char *out = (char *)malloc(cap);
    if (!out) return NULL;
    size_t n = 0;
    out[0] = 0;
    for (int32_t i = 0; i < g->rows; i++) {
        for (int32_t j = 0; j < g->cols; j++) {
            int w = snprintf(
                out + n,
                cap - n,
                "%s%g",
                j ? " " : "",
                g->data[(size_t)i * (size_t)g->cols + (size_t)j]);
            if (w < 0) break;
            n += (size_t)w;
            if (n + 1 >= cap) break;
        }
        if (i + 1 < g->rows && n + 2 < cap) {
            out[n++] = '\n';
            out[n] = 0;
        }
    }
    return out;
}
