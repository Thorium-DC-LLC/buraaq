// Buraaq minimal native runtime — linked with all generated executables.
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <inttypes.h>

void buraaq_print_i32(int32_t n) { printf("%" PRId32, n); }
void buraaq_print_i32_ln(int32_t n) { printf("%" PRId32 "\n", n); }

void buraaq_print_i64(int64_t n) { printf("%" PRId64, n); }
void buraaq_print_i64_ln(int64_t n) { printf("%" PRId64 "\n", n); }

void buraaq_print_f64(double n) { printf("%g", n); }
void buraaq_print_f64_ln(double n) { printf("%g\n", n); }

void buraaq_print_bool(int b) { printf("%s", b ? "true" : "false"); }
void buraaq_print_bool_ln(int b) { printf("%s\n", b ? "true" : "false"); }

void buraaq_print_str(const char *s) { fputs(s, stdout); }
void buraaq_print_str_ln(const char *s) { puts(s); }

static char *buraaq_dup(const char *s) {
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

char *buraaq_i32_to_text(int32_t n) {
    char buf[32];
    snprintf(buf, sizeof(buf), "%" PRId32, n);
    return buraaq_dup(buf);
}

char *buraaq_i64_to_text(int64_t n) {
    char buf[32];
    snprintf(buf, sizeof(buf), "%" PRId64, n);
    return buraaq_dup(buf);
}

char *buraaq_f64_to_text(double n) {
    char buf[64];
    snprintf(buf, sizeof(buf), "%g", n);
    return buraaq_dup(buf);
}

char *buraaq_bool_to_text(int b) { return buraaq_dup(b ? "true" : "false"); }

void *buraaq_alloc(int64_t bytes) {
    if (bytes <= 0) return NULL;
    return malloc((size_t)bytes);
}

void buraaq_free(void *p) { free(p); }

#if defined(__clang__) || defined(__GNUC__)
__attribute__((weak))
#endif
void buraaq_rt_set_args(int argc, char **argv) {
    (void)argc;
    (void)argv;
}
