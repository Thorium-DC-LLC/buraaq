use std::time::Instant;

fn main() {
    let n = 100_000_000_i64;
    let t0 = Instant::now();
    let mut a = 0_i64;
    let mut b = 1_i64;
    for _ in 0..n {
        let c = a + b;
        a = b;
        b = c;
    }
    let elapsed = t0.elapsed().as_secs_f64();
    std::hint::black_box((a, b));
    println!(
        "BENCH name=fib_iter time_sec={:.6} ops={} ops_per_sec={:.0}",
        elapsed,
        n,
        n as f64 / elapsed
    );
    std::process::exit(if a != 0 || b != 0 { 0 } else { 1 });
}
