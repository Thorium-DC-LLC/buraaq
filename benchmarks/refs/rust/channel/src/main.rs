use std::sync::mpsc;
use std::time::Instant;

const ITERS: i64 = 200_000;

fn main() {
    let (tx, rx) = mpsc::sync_channel::<i64>(256);
    let t0 = Instant::now();
    for i in 0..ITERS {
        tx.send(i).unwrap();
        rx.recv().unwrap();
    }
    let secs = t0.elapsed().as_secs_f64();
    let per_op_ns = (secs * 1e9) / ITERS as f64;
    println!(
        "BENCH|channel_ping_rust|iters={}|total_sec={:.6}|per_op_ns={:.2}",
        ITERS, secs, per_op_ns
    );
}
