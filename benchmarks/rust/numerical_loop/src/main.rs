use std::time::Instant;

fn main() {
    let n = 100_000_000_i64;
    let t0 = Instant::now();
    let mut sum = 0.0_f64;
    for i in 1..=n {
        sum += 1.0 / i as f64;
    }
    let elapsed = t0.elapsed().as_secs_f64();
    std::hint::black_box(sum);
    println!(
        "BENCH name=numerical_loop time_sec={:.6} ops={} ops_per_sec={:.0}",
        elapsed,
        n,
        n as f64 / elapsed
    );
    std::process::exit(if sum > 0.0 { 0 } else { 1 });
}
