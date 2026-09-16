use std::time::Instant;

const ITERS: i64 = 50_000;

fn main() {
    let t0 = Instant::now();
    for _ in 0..ITERS {
        let h = std::thread::spawn(|| {});
        h.join().unwrap();
    }
    let secs = t0.elapsed().as_secs_f64();
    let per_op_ns = (secs * 1e9) / ITERS as f64;
    println!(
        "BENCH|task_spawn_rust|iters={}|total_sec={:.6}|per_op_ns={:.2}",
        ITERS, secs, per_op_ns
    );
}
