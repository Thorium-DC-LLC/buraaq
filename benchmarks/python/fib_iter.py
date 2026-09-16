import time

def main() -> None:
    n = 100_000_000
    t0 = time.perf_counter()
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    elapsed = time.perf_counter() - t0
    ops_per_sec = n / elapsed if elapsed > 0 else 0.0
    print(
        f"BENCH name=fib_iter time_sec={elapsed:.6f} "
        f"ops={n} ops_per_sec={ops_per_sec:.0f}"
    )
    if a == 0 and b == 0:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
