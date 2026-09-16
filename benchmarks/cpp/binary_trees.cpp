// Binary tree creation + traversal (classic allocation-heavy microbench).
#include "../suite/bench_common.h"
#include <cstdlib>

struct Node {
    int value;
    Node* left;
    Node* right;
};

static Node* make_tree(int depth, int value) {
    if (depth <= 0) return nullptr;
    Node* n = new Node{value, nullptr, nullptr};
    n->left = make_tree(depth - 1, value * 2);
    n->right = make_tree(depth - 1, value * 2 + 1);
    return n;
}

static long long sum_tree(Node* n) {
    if (!n) return 0;
    return n->value + sum_tree(n->left) + sum_tree(n->right);
}

static void free_tree(Node* n) {
    if (!n) return;
    free_tree(n->left);
    free_tree(n->right);
    delete n;
}

int main() {
    const int depth = 14;
    const int iters = 20;
    long long checksum = 0;
    double t0 = bench_now_sec();
    for (int i = 0; i < iters; ++i) {
        Node* root = make_tree(depth, i + 1);
        checksum += sum_tree(root);
        free_tree(root);
    }
    double elapsed = bench_now_sec() - t0;
    BENCH_PRINT("binary_trees", elapsed, (long long)iters);
    if (checksum == 0) return 1; /* prevent DCE of work */
    return 0;
}
