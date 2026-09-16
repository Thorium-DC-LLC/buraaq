const std = @import("std");

pub fn main() !void {
    const n: i64 = 100_000_000;
    var timer = try std.time.Timer.start();
    var sum: f64 = 0;
    var i: i64 = 1;
    while (i <= n) : (i += 1) {
        sum += 1.0 / @as(f64, @floatFromInt(i));
    }
    const elapsed = @as(f64, @floatFromInt(timer.read())) / 1e9;
    std.debug.print("BENCH name=numerical_loop time_sec={d:.6} ops={d} ops_per_sec={d:.0}\n", .{
        elapsed, n, @as(f64, @floatFromInt(n)) / elapsed,
    });
    if (sum <= 0) std.process.exit(1);
}
