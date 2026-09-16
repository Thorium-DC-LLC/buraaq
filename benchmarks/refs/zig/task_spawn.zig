const std = @import("std");

const ITERS: i64 = 50_000;

pub fn main() !void {
    var t0 = try std.time.Instant.now();
    var i: i64 = 0;
    while (i < ITERS) : (i += 1) {
        const t = try std.Thread.spawn(.{}, struct {
            fn run() void {}
        }.run);
        t.join();
    }
    const t1 = try std.time.Instant.now();
    const secs = @as(f64, @floatFromInt(t1.since(t0))) / 1e9;
    const per_op_ns = (secs * 1e9) / @as(f64, @floatFromInt(ITERS));
    std.debug.print("BENCH|task_spawn_zig|iters={}|total_sec={d:.6}|per_op_ns={d:.2}\n", .{ ITERS, secs, per_op_ns });
}
