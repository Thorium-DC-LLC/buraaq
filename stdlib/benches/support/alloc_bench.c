#include "buraaq_std.h"

#include <stdio.h>
#include <stdlib.h>

#define ITERS 10000

int main(void) {
    size_t allocs = 0;

    for (int i = 0; i < ITERS; i++) {
        char *s = buraaq_text_concat("a", "b");
        if (s) {
            allocs++;
            free(s);
        }
    }

    printf("alloc_bench: %d iterations, %zu heap allocs (1 per concat)\n", ITERS, allocs);
    return allocs == (size_t)ITERS ? 0 : 1;
}
