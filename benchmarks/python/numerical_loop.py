import time

def main() -> None:
    n = 100_000_000
    t0 = time.perf_counter()
    total = 0.0
    for i in range(1, n + 1):
        total += 1.0 / i
    elapsed = time.perf_counter() - t0
    ops_per_sec = n / elapsed if elapsed > 0 else 0.0
    print(
        f"BENCH name=numerical_loop time_sec={elapsed:.6f} "
        f"ops={n} ops_per_sec={ops_per_sec:.0f}"
    )
    if total <= 0.0:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
