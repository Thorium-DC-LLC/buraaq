package main

import (
	"fmt"
	"runtime"
	"time"
)

func main() {
	const n = 100_000_000
	t0 := time.Now()
	sum := 0.0
	for i := 1; i <= n; i++ {
		sum += 1.0 / float64(i)
	}
	elapsed := time.Since(t0).Seconds()
	runtime.KeepAlive(sum)
	fmt.Printf("BENCH name=numerical_loop time_sec=%.6f ops=%d ops_per_sec=%.0f\n",
		elapsed, n, float64(n)/elapsed)
	if sum <= 0 {
		panic("bad sum")
	}
}
