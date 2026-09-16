package main

import (
	"fmt"
	"runtime"
	"time"
)

func main() {
	const n = 100_000_000
	t0 := time.Now()
	a, b := 0, 1
	for i := 0; i < n; i++ {
		a, b = b, a+b
	}
	elapsed := time.Since(t0).Seconds()
	runtime.KeepAlive(a)
	runtime.KeepAlive(b)
	fmt.Printf("BENCH name=fib_iter time_sec=%.6f ops=%d ops_per_sec=%.0f\n",
		elapsed, n, float64(n)/elapsed)
	if a == 0 && b == 0 {
		panic("bad fib")
	}
}
