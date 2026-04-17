// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

const std = @import("std");
const opendal = @import("opendal");

pub fn main() !void {
    const gpa = std.heap.page_allocator;

    var runtime = try opendal.Runtime.init(gpa, .{});
    defer runtime.deinit();

    var io_runtime = std.Io.Threaded.init(gpa, .{});
    defer io_runtime.deinit();
    const io = io_runtime.io();

    var op = try opendal.Operator.initKnownWithRuntime(&runtime, .memory, &.{});
    defer op.deinit();

    var write_op = try op.writeAsync("async.txt", "Hello async Zig", .{});
    defer write_op.deinit();
    try write_op.await(io);

    var read_op = try op.readAsync("async.txt", .{});
    defer read_op.deinit();
    var bytes = try read_op.await(io);
    defer bytes.deinit();

    var info_op = try op.infoAsync();
    defer info_op.deinit();
    var info = try info_op.await(io);
    defer info.deinit(runtime.allocator());

    std.debug.print("async read: {s}\n", .{bytes.slice()});
    std.debug.print("scheme={s} root={s}\n", .{ info.scheme, info.root });

    var lister = try op.listerAsync("", .{});
    defer lister.deinit();

    var next_op = try lister.nextAsync();
    defer next_op.deinit();

    if (try next_op.await(io)) |entry| {
        var owned = entry;
        defer owned.deinit(runtime.allocator());
        std.debug.print("async entry: {s}\n", .{owned.path});
    }
}
