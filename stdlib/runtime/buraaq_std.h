#ifndef BURAAQ_STD_H
#define BURAAQ_STD_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* --- fs / path --- */
char *buraaq_file_read(const char *path);
int buraaq_file_write(const char *path, const char *contents);
int buraaq_file_exists(const char *path);

/* --- text / buffer --- */
char *buraaq_text_concat(const char *a, const char *b);
int32_t buraaq_text_len(const char *s);
int32_t buraaq_text_eq(const char *a, const char *b);
int32_t buraaq_text_byte(const char *s, int32_t i);
char *buraaq_text_slice(const char *s, int32_t start, int32_t end);
void buraaq_rt_set_args(int argc, char **argv);
double buraaq_now_sec(void);
int32_t buraaq_keepalive_i32(int32_t v);
double buraaq_keepalive_f64(double v);
void buraaq_bench_report(const char *name, double sec, int32_t n, int32_t checksum);
void buraaq_bench_report_f64(const char *name, double sec, int32_t n, double checksum);

/* --- math --- */
double buraaq_math_sqrt(double x);
double buraaq_math_abs_f64(double x);
int32_t buraaq_math_abs_i32(int32_t x);
double buraaq_math_min_f64(double a, double b);
double buraaq_math_max_f64(double a, double b);
double buraaq_math_sin(double x);
double buraaq_math_cos(double x);
double buraaq_math_tan(double x);
double buraaq_math_asin(double x);
double buraaq_math_acos(double x);
double buraaq_math_atan(double x);
double buraaq_math_atan2(double y, double x);
double buraaq_math_sinh(double x);
double buraaq_math_cosh(double x);
double buraaq_math_tanh(double x);
double buraaq_math_exp(double x);
double buraaq_math_log(double x);
double buraaq_math_log10(double x);
double buraaq_math_log2(double x);
double buraaq_math_pow(double x, double y);
double buraaq_math_hypot(double x, double y);
double buraaq_math_floor(double x);
double buraaq_math_ceil(double x);
double buraaq_math_trunc(double x);
double buraaq_math_round(double x);
double buraaq_math_fmod(double x, double y);
double buraaq_math_copysign(double mag, double sgn);
double buraaq_math_cbrt(double x);
double buraaq_math_pi(void);
double buraaq_math_euler(void);
double buraaq_math_deg(double rad);
double buraaq_math_rad(double deg);
double buraaq_math_clamp(double x, double lo, double hi);
double buraaq_math_lerp(double a, double b, double t);
double buraaq_math_sign(double x);

/* --- grid (numeric arrays) --- */
int32_t buraaq_grid_zeros(int32_t rows, int32_t cols);
int32_t buraaq_grid_ones(int32_t rows, int32_t cols);
int32_t buraaq_grid_fill(int32_t rows, int32_t cols, double v);
int32_t buraaq_grid_eye(int32_t n);
int32_t buraaq_grid_linspace(double a, double b, int32_t n);
int32_t buraaq_grid_row(const char *csv);
int32_t buraaq_grid_nrows(int32_t id);
int32_t buraaq_grid_ncols(int32_t id);
double buraaq_grid_at(int32_t id, int32_t r, int32_t c);
int32_t buraaq_grid_put_at(int32_t id, int32_t r, int32_t c, double v);
int32_t buraaq_grid_plus(int32_t a, int32_t b);
int32_t buraaq_grid_times(int32_t a, int32_t b);
int32_t buraaq_grid_scale(int32_t id, double k);
int32_t buraaq_grid_matmul(int32_t a, int32_t b);
double buraaq_grid_dot(int32_t a, int32_t b);
int32_t buraaq_grid_transpose(int32_t id);
double buraaq_grid_sum(int32_t id);
double buraaq_grid_mean(int32_t id);
double buraaq_grid_least(int32_t id);
double buraaq_grid_most(int32_t id);
double buraaq_grid_norm(int32_t id);
int32_t buraaq_grid_sin_all(int32_t id);
int32_t buraaq_grid_cos_all(int32_t id);
char *buraaq_grid_view(int32_t id);

/* --- hold (tables) --- */
int32_t buraaq_hold_new(const char *cols);
int32_t buraaq_hold_stow(int32_t id, const char *row);
int32_t buraaq_hold_rows(int32_t id);
int32_t buraaq_hold_cols(int32_t id);
char *buraaq_hold_pick(int32_t id, const char *col, int32_t row);
int32_t buraaq_hold_keep(int32_t id, const char *col, const char *value);
double buraaq_hold_col_mean(int32_t id, const char *col);
double buraaq_hold_col_sum(int32_t id, const char *col);
int32_t buraaq_hold_order(int32_t id, const char *col);
int32_t buraaq_hold_from_csv(const char *path);
int32_t buraaq_hold_to_csv(int32_t id, const char *path);
char *buraaq_hold_view(int32_t id);

/* --- stream (websockets) --- */
int32_t buraaq_stream_bind(int32_t port);
int32_t buraaq_stream_hail(void);
int32_t buraaq_stream_wire(const char *url);
int32_t buraaq_stream_say(int32_t id, const char *msg);
char *buraaq_stream_hear(int32_t id);
void buraaq_stream_hangup(int32_t id);
void buraaq_stream_run(void);

/* --- time --- */
int64_t buraaq_time_now_ms(void);
void buraaq_time_sleep_ms(int64_t ms);

/* --- os --- */
char *buraaq_os_getenv(const char *name);
int32_t buraaq_os_argc(void);
char *buraaq_os_argv(int32_t index);

/* --- json (minimal) --- */
char *buraaq_json_parse_string_field(const char *json, const char *key);

/* --- http client --- */
char *buraaq_http_get_body(const char *url);

/* --- AI client (OpenAI-compatible local serve) --- */
int32_t buraaq_ai_model(const char *name);
char *buraaq_ai_chat(int32_t handle, const char *prompt);
char *buraaq_ai_embed(int32_t handle, const char *text);
int32_t buraaq_ai_system(int32_t handle, const char *prompt);

/* --- http/https server --- */
int32_t buraaq_http_listen(int32_t port, int32_t https);
int32_t buraaq_http_accept(int32_t listener);
char *buraaq_http_method(int32_t conn);
char *buraaq_http_path(int32_t conn);
char *buraaq_http_body(int32_t conn);
int32_t buraaq_http_path_starts(int32_t conn, const char *prefix);
int32_t buraaq_http_path_id(int32_t conn);
char *buraaq_http_json_field(int32_t conn, const char *key);
int32_t buraaq_http_reply(int32_t conn, int32_t status, const char *ctype, const char *body);
void buraaq_http_close(int32_t conn);

/* --- postgres --- */
int32_t buraaq_pg_connect(const char *conninfo);
int32_t buraaq_pg_ok(void);
char *buraaq_pg_exec(const char *sql);
char *buraaq_pg_quote(const char *s);
void buraaq_pg_close(void);

/* --- production service (TLS + REST + Postgres) --- */
int32_t buraaq_svc_page(const char *path, const char *file);
int32_t buraaq_svc_api(const char *name, const char *fields);
int32_t buraaq_svc_store(const char *url);
int32_t buraaq_svc_key(const char *secret);
int32_t buraaq_svc_origin(const char *allowed);
int32_t buraaq_svc_run(int32_t port);

/* --- Lumen: native HD UI --- */
int32_t buraaq_ui_app(const char *title, int32_t w, int32_t h);
int32_t buraaq_ui_heading(const char *text);
int32_t buraaq_ui_note(const char *text);
int32_t buraaq_ui_field(const char *id, const char *label);
int32_t buraaq_ui_button(const char *id, const char *label);
int32_t buraaq_ui_bind(const char *base_url, const char *resource);
int32_t buraaq_ui_keep(const char *path);
int32_t buraaq_ui_show(void);
char *buraaq_ui_value(const char *id);
int32_t buraaq_ui_clicked(const char *id);

/* --- crypto (stub) --- */
char *buraaq_crypto_sha256_hex(const char *data);

/* --- sync --- */
void *buraaq_mutex_new(void);
void buraaq_mutex_lock(void *m);
void buraaq_mutex_unlock(void *m);
void buraaq_mutex_free(void *m);

/* --- process --- */
int32_t buraaq_process_exit_code(const char *cmd);

#ifdef __cplusplus
}
#endif

#endif
