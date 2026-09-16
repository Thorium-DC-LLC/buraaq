package main

import (
	"fmt"
	"time"
)

const iters = 200000

func main() {
	ch := make(chan int64, 256)
	start := time.Now()
	for i := int64(0); i < iters; i++ {
		ch <- i
		<-ch
	}
	secs := time.Since(start).Seconds()
	perOp := (secs * 1e9) / float64(iters)
	fmt.Printf("BENCH|channel_ping_go|iters=%d|total_sec=%.6f|per_op_ns=%.2f\n", iters, secs, perOp)
}
