package main

import (
	"fmt"
	"sync"
	"time"
)

const iters = 50000

func main() {
	start := time.Now()
	for i := 0; i < iters; i++ {
		var wg sync.WaitGroup
		wg.Add(1)
		go func() {
			defer wg.Done()
		}()
		wg.Wait()
	}
	secs := time.Since(start).Seconds()
	perOp := (secs * 1e9) / float64(iters)
	fmt.Printf("BENCH|task_spawn_go|iters=%d|total_sec=%.6f|per_op_ns=%.2f\n", iters, secs, perOp)
}
