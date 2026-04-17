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

const opendal = @import("opendal");
const std = @import("std");
const testing = std.testing;

fn initThreadedIo() std.Io.Threaded {
    return std.Io.Threaded.init(testing.allocator, .{});
}

test "sync memory operator" {
    var op = try opendal.Operator.initKnown(.memory, &.{});
    defer op.deinit();

    try op.check();
    try op.write("dir/file.txt", "hello", .{});
    try testing.expect(try op.exists("dir/file.txt"));

    var bytes = try op.readBytes("dir/file.txt", .{});
    defer bytes.deinit();
    try testing.expectEqualStrings("hello", bytes.slice());

    var meta = try op.statAlloc(testing.allocator, "dir/file.txt", .{});
    defer meta.deinit(testing.allocator);
    try testing.expect(meta.is_file);
    try testing.expectEqual(@as(?u64, 5), meta.content_length);

    var info = try op.infoAlloc(testing.allocator);
    defer info.deinit(testing.allocator);
    try testing.expectEqualStrings("memory", info.scheme);

    var lister = try op.lister("dir/", .{});
    defer lister.deinit();

    const entry = (try lister.next()).?;
    try testing.expectEqualStrings("dir/file.txt", entry.path());
    try testing.expectEqualStrings("file.txt", entry.name());
}

test "sync reader writer and owned entry lifecycle" {
    var op = try opendal.Operator.initKnown(.memory, &.{});
    defer op.deinit();

    var writer = try op.writer("notes/hello.txt", .{});
    defer writer.deinit();
    try writer.writeAll("abcdef");
    try writer.close();

    var reader = try op.reader("notes/hello.txt", .{});
    defer reader.deinit();

    var buf: [3]u8 = undefined;
    const first_read = try reader.read(buf[0..]);
    try testing.expectEqual(@as(usize, 3), first_read);
    try testing.expectEqualStrings("abc", buf[0..first_read]);

    try testing.expectEqual(@as(u64, 2), try reader.seekTo(2));
    const second_read = try reader.read(buf[0..]);
    try testing.expectEqual(@as(usize, 3), second_read);
    try testing.expectEqualStrings("cde", buf[0..second_read]);

    var lister = try op.lister("notes/", .{});
    defer lister.deinit();

    const view = (try lister.next()).?;
    try testing.expectEqualStrings("notes/hello.txt", view.path());
    try testing.expectEqualStrings("hello.txt", view.name());

    const metadata_view = view.metadata();
    try testing.expect(metadata_view.isFile());
    try testing.expectEqual(@as(?u64, 6), metadata_view.contentLength());

    var owned = try view.cloneAlloc(testing.allocator);
    defer owned.deinit(testing.allocator);
    try testing.expectEqualStrings("notes/hello.txt", owned.path);
    try testing.expectEqualStrings("hello.txt", owned.name());
    try testing.expect(owned.metadata.is_file);

    var lister_owned = try op.lister("notes/", .{});
    defer lister_owned.deinit();
    var owned_from_next = (try lister_owned.nextAlloc(testing.allocator)).?;
    defer owned_from_next.deinit(testing.allocator);
    try testing.expectEqualStrings("notes/hello.txt", owned_from_next.path);
}

test "async APIs require runtime" {
    var op = try opendal.Operator.initKnown(.memory, &.{});
    defer op.deinit();

    try testing.expectError(error.AsyncRuntimeRequired, op.readAsync("missing.txt", .{}));
    try testing.expectError(error.AsyncRuntimeRequired, op.infoAsync());
    try testing.expectError(error.AsyncRuntimeRequired, op.writerAsync("missing.txt", .{}));
}

test "async memory operator" {
    var runtime = try opendal.Runtime.init(testing.allocator, .{});
    defer runtime.deinit();

    var io_runtime = initThreadedIo();
    defer io_runtime.deinit();
    const io = io_runtime.io();

    var op = try opendal.Operator.initKnownWithRuntime(&runtime, .memory, &.{});
    defer op.deinit();

    var write_op = try op.writeAsync("async.txt", "world", .{});
    defer write_op.deinit();
    try write_op.await(io);

    var read_op = try op.readAsync("async.txt", .{});
    defer read_op.deinit();
    var bytes = try read_op.await(io);
    defer bytes.deinit();
    try testing.expectEqualStrings("world", bytes.slice());

    var reader = try op.readerAsync("async.txt", .{});
    defer reader.deinit();
    var buf: [5]u8 = undefined;
    var read_chunk = try reader.readAsync(buf[0..]);
    defer read_chunk.deinit();
    const read_len = try read_chunk.await(io);
    try testing.expectEqual(@as(usize, 5), read_len);
    try testing.expectEqualStrings("world", buf[0..read_len]);

    var lister = try op.listerAsync("", .{});
    defer lister.deinit();
    var next_op = try lister.nextAsync();
    defer next_op.deinit();
    var entry = (try next_op.await(io)).?;
    defer entry.deinit(runtime.allocator());
    try testing.expectEqualStrings("async.txt", entry.path);
}

test "async poll, alloc, stat, info and cancel" {
    var runtime = try opendal.Runtime.init(testing.allocator, .{});
    defer runtime.deinit();

    var io_runtime = initThreadedIo();
    defer io_runtime.deinit();
    const io = io_runtime.io();

    var op = try opendal.Operator.initKnownWithRuntime(&runtime, .memory, &.{});
    defer op.deinit();

    try op.write("poll.txt", "zig-native", .{});

    var read_op = try op.readAsync("poll.txt", .{});
    defer read_op.deinit();

    switch (try read_op.poll()) {
        .pending => {
            var bytes = try read_op.await(io);
            defer bytes.deinit();
            try testing.expectEqualStrings("zig-native", bytes.slice());
        },
        .ready => |bytes| {
            var ready_bytes = bytes;
            defer ready_bytes.deinit();
            try testing.expectEqualStrings("zig-native", ready_bytes.slice());
        },
    }

    var alloc_op = try op.readAsync("poll.txt", .{});
    defer alloc_op.deinit();
    const copied = try alloc_op.awaitAlloc(io, testing.allocator);
    defer testing.allocator.free(copied);
    try testing.expectEqualStrings("zig-native", copied);

    var stat_op = try op.statAsync("poll.txt", .{});
    defer stat_op.deinit();
    var stat = try stat_op.await(io);
    defer stat.deinit(runtime.allocator());
    try testing.expect(stat.is_file);
    try testing.expectEqual(@as(?u64, 10), stat.content_length);

    var info_op = try op.infoAsync();
    defer info_op.deinit();
    var info = try info_op.await(io);
    defer info.deinit(runtime.allocator());
    try testing.expectEqualStrings("memory", info.scheme);
    try testing.expect(info.full_capability.read);

    var cancel_op = try op.readAsync("poll.txt", .{});
    defer cancel_op.deinit();
    if (try cancel_op.cancel(io)) |bytes| {
        var canceled_bytes = bytes;
        defer canceled_bytes.deinit();
        try testing.expectEqualStrings("zig-native", canceled_bytes.slice());
    }
}

test "async reader writer and lister" {
    var runtime = try opendal.Runtime.init(testing.allocator, .{});
    defer runtime.deinit();

    var io_runtime = initThreadedIo();
    defer io_runtime.deinit();
    const io = io_runtime.io();

    var op = try opendal.Operator.initKnownWithRuntime(&runtime, .memory, &.{});
    defer op.deinit();

    var writer = try op.writerAsync("stream.txt", .{});
    defer writer.deinit();

    var first_write = try writer.writeAsync("stream-");
    defer first_write.deinit();
    try testing.expectEqual(@as(usize, 7), try first_write.await(io));

    var second_write = try writer.writeAsync("data");
    defer second_write.deinit();
    try testing.expectEqual(@as(usize, 4), try second_write.await(io));

    var close_op = try writer.closeAsync();
    defer close_op.deinit();
    try close_op.await(io);

    var reader = try op.readerAsync("stream.txt", .{});
    defer reader.deinit();

    var seek_op = try reader.seekToAsync(7);
    defer seek_op.deinit();
    try testing.expectEqual(@as(u64, 7), try seek_op.await(io));

    var buf: [4]u8 = undefined;
    var read_op = try reader.readAsync(buf[0..]);
    defer read_op.deinit();
    try testing.expectEqual(@as(usize, 4), try read_op.await(io));
    try testing.expectEqualStrings("data", buf[0..]);

    var lister = try op.listerAsync("", .{});
    defer lister.deinit();

    var next_op = try lister.nextAsync();
    defer next_op.deinit();
    var entry = (try next_op.await(io)).?;
    defer entry.deinit(runtime.allocator());
    try testing.expectEqualStrings("stream.txt", entry.path);
}

test "module loads" {
    _ = opendal;
}
