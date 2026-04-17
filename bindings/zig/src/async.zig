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
const types = @import("types.zig");

pub const c = types.c;

const Allocator = std.mem.Allocator;
const Error = types.Error;

pub fn Poll(comptime T: type) type {
    return union(enum) {
        pending,
        ready: T,
    };
}

const OpState = enum {
    pending,
    consumed,
    canceled,
};

pub const Runtime = struct {
    allocator_storage: Allocator,
    inner: ?*c.od_zig_runtime,

    pub fn init(gpa: Allocator, options: types.RuntimeOptions) !Runtime {
        const result = c.od_zig_runtime_new(options.toRaw());
        try types.codeToError(result.code);
        return .{
            .allocator_storage = gpa,
            .inner = result.value orelse return error.Internal,
        };
    }

    pub fn allocator(self: *const Runtime) Allocator {
        return self.allocator_storage;
    }

    pub fn deinit(self: *Runtime) void {
        if (self.inner) |ptr| {
            c.od_zig_runtime_free(ptr);
            self.inner = null;
        }
    }

    pub fn lastErrorInfo(self: *Runtime, gpa: Allocator) !?types.ErrorInfo {
        const runtime = self.inner orelse return error.Internal;
        const result = c.od_zig_runtime_last_error_info(runtime);
        try types.codeToError(result.code);
        if (!result.has_value) {
            return null;
        }
        return try types.errorInfoFromOwned(gpa, result.value);
    }

    fn waitForTask(self: *Runtime, task: ?*c.od_zig_task, io: ?std.Io) !void {
        _ = io;
        const task_ptr = task orelse return error.Internal;
        while (true) {
            const poll_result = c.od_zig_task_poll(task_ptr);
            try types.codeToError(poll_result.code);
            if (poll_result.state == c.OD_ZIG_POLL_READY) {
                return;
            }
            try self.waitOnce();
        }
    }

    fn waitOnce(self: *Runtime) !void {
        const runtime = self.inner orelse return error.Internal;
        switch (c.od_zig_runtime_notifier_kind(runtime)) {
            c.OD_ZIG_NOTIFIER_NONE => return,
            c.OD_ZIG_NOTIFIER_PIPE_READ_FD, c.OD_ZIG_NOTIFIER_EVENTFD => {
                const fd = c.od_zig_runtime_notifier_fd(runtime);
                if (fd < 0) {
                    return error.Internal;
                }
                var fds = [_]std.posix.pollfd{.{
                    .fd = fd,
                    .events = std.posix.POLL.IN,
                    .revents = 0,
                }};
                _ = try std.posix.poll(&fds, 10);

                const drain = c.od_zig_runtime_drain_ready_tasks(runtime);
                try types.codeToError(drain.code);
            },
            c.OD_ZIG_NOTIFIER_WIN_HANDLE => return error.Unsupported,
            else => return error.Internal,
        }
    }
};

fn takeUnit(_: *Runtime, task: ?*c.od_zig_task) !void {
    const result = c.od_zig_task_take_unit(task orelse return error.Internal);
    try types.codeToError(result.code);
}

fn takeBool(_: *Runtime, task: ?*c.od_zig_task) !bool {
    const result = c.od_zig_task_take_bool(task orelse return error.Internal);
    try types.codeToError(result.code);
    return result.value;
}

fn takeUsize(_: *Runtime, task: ?*c.od_zig_task) !usize {
    const result = c.od_zig_task_take_usize(task orelse return error.Internal);
    try types.codeToError(result.code);
    return result.value;
}

fn takeU64(_: *Runtime, task: ?*c.od_zig_task) !u64 {
    const result = c.od_zig_task_take_u64(task orelse return error.Internal);
    try types.codeToError(result.code);
    return result.value;
}

fn takeBytes(_: *Runtime, task: ?*c.od_zig_task) !types.Bytes {
    const result = c.od_zig_task_take_bytes(task orelse return error.Internal);
    try types.codeToError(result.code);
    return try types.bytesFromOwned(result.value);
}

fn takeMetadata(runtime: *Runtime, task: ?*c.od_zig_task) !types.Metadata {
    const result = c.od_zig_task_take_metadata(task orelse return error.Internal);
    try types.codeToError(result.code);
    return try types.metadataFromOwned(runtime.allocator(), result.value);
}

fn takeInfo(runtime: *Runtime, task: ?*c.od_zig_task) !types.OperatorInfo {
    const result = c.od_zig_task_take_info(task orelse return error.Internal);
    try types.codeToError(result.code);
    return try types.infoFromOwned(runtime.allocator(), result.value);
}

fn takePresignedRequest(runtime: *Runtime, task: ?*c.od_zig_task) !types.PresignedRequest {
    const result = c.od_zig_task_take_presigned_request(task orelse return error.Internal);
    try types.codeToError(result.code);
    return try types.presignedRequestFromOwned(runtime.allocator(), result.value);
}

fn takeEntry(runtime: *Runtime, task: ?*c.od_zig_task) !?types.Entry {
    const result = c.od_zig_task_take_entry(task orelse return error.Internal);
    try types.codeToError(result.code);
    if (!result.has_value) {
        return null;
    }
    return try types.entryFromOwned(runtime.allocator(), result.value);
}

pub fn AsyncOp(
    comptime T: type,
    comptime take_result: *const fn (*Runtime, ?*c.od_zig_task) Error!T,
) type {
    return struct {
        runtime: *Runtime,
        task: ?*c.od_zig_task,
        state: OpState = .pending,

        const Self = @This();

        pub fn init(runtime: *Runtime, task: ?*c.od_zig_task) Self {
            return .{ .runtime = runtime, .task = task };
        }

        pub fn poll(self: *Self) !Poll(T) {
            try self.ensurePending();
            const task_ptr = self.task orelse return error.Internal;
            const poll_result = c.od_zig_task_poll(task_ptr);
            try types.codeToError(poll_result.code);
            if (poll_result.state == c.OD_ZIG_POLL_PENDING) {
                return .pending;
            }
            const value = try take_result(self.runtime, task_ptr);
            self.state = .consumed;
            return .{ .ready = value };
        }

        pub fn await(self: *Self, io: std.Io) !T {
            try self.ensurePending();
            try self.runtime.waitForTask(self.task, io);
            const value = try take_result(self.runtime, self.task);
            self.state = .consumed;
            return value;
        }

        pub fn cancel(self: *Self, io: std.Io) !?T {
            _ = io;
            try self.ensurePending();
            const task_ptr = self.task orelse return error.Internal;
            const cancel_result = c.od_zig_task_cancel(task_ptr);
            try types.codeToError(cancel_result.code);
            if (cancel_result.completed) {
                const value = try take_result(self.runtime, task_ptr);
                self.state = .consumed;
                return value;
            }
            self.state = .canceled;
            return null;
        }

        pub fn deinit(self: *Self) void {
            if (self.task) |task_ptr| {
                std.debug.assert(self.state != .pending);
                c.od_zig_task_free(task_ptr);
                self.task = null;
            }
        }

        fn ensurePending(self: *Self) !void {
            return switch (self.state) {
                .pending => {},
                .consumed, .canceled => error.Internal,
            };
        }
    };
}

pub const VoidOp = AsyncOp(void, takeUnit);
pub const CreateDirOp = AsyncOp(void, takeUnit);
pub const DeleteOp = AsyncOp(void, takeUnit);
pub const RenameOp = AsyncOp(void, takeUnit);
pub const CopyOp = AsyncOp(void, takeUnit);
pub const WriteCompletionOp = AsyncOp(void, takeUnit);
pub const ExistsOp = AsyncOp(bool, takeBool);
pub const ReaderReadOp = AsyncOp(usize, takeUsize);
pub const ReaderSeekOp = AsyncOp(u64, takeU64);
pub const WriterWriteOp = AsyncOp(usize, takeUsize);
pub const WriterCloseOp = AsyncOp(void, takeUnit);
pub const StatOp = AsyncOp(types.Metadata, takeMetadata);
pub const InfoOp = AsyncOp(types.OperatorInfo, takeInfo);
pub const PresignOp = AsyncOp(types.PresignedRequest, takePresignedRequest);
pub const ListNextOp = AsyncOp(?types.Entry, takeEntry);

const ReadBaseOp = AsyncOp(types.Bytes, takeBytes);

pub const ReadOp = struct {
    inner: ReadBaseOp,

    pub fn init(runtime: *Runtime, task: ?*c.od_zig_task) ReadOp {
        return .{ .inner = ReadBaseOp.init(runtime, task) };
    }

    pub fn poll(self: *ReadOp) !Poll(types.Bytes) {
        return try self.inner.poll();
    }

    pub fn await(self: *ReadOp, io: std.Io) !types.Bytes {
        return try self.inner.await(io);
    }

    pub fn cancel(self: *ReadOp, io: std.Io) !?types.Bytes {
        return try self.inner.cancel(io);
    }

    pub fn awaitAlloc(self: *ReadOp, io: std.Io, allocator: Allocator) ![]u8 {
        var bytes = try self.await(io);
        defer bytes.deinit();
        return try bytes.cloneAlloc(allocator);
    }

    pub fn deinit(self: *ReadOp) void {
        self.inner.deinit();
    }
};

pub const AsyncReader = struct {
    runtime: *Runtime,
    inner: ?*c.od_zig_async_reader,

    pub fn deinit(self: *AsyncReader) void {
        if (self.inner) |ptr| {
            c.od_zig_async_reader_free(ptr);
            self.inner = null;
        }
    }

    pub fn readAsync(self: *AsyncReader, buf: []u8) !ReaderReadOp {
        const result = c.od_zig_reader_read_start(self.inner orelse return error.Internal, types.mutSliceToRaw(buf));
        try types.codeToError(result.code);
        return ReaderReadOp.init(self.runtime, result.value);
    }

    pub fn seekToAsync(self: *AsyncReader, pos: u64) !ReaderSeekOp {
        const result = c.od_zig_reader_seek_to_start(self.inner orelse return error.Internal, pos);
        try types.codeToError(result.code);
        return ReaderSeekOp.init(self.runtime, result.value);
    }

    pub fn seekByAsync(self: *AsyncReader, delta: i64) !ReaderSeekOp {
        const result = c.od_zig_reader_seek_by_start(self.inner orelse return error.Internal, delta);
        try types.codeToError(result.code);
        return ReaderSeekOp.init(self.runtime, result.value);
    }

    pub fn seekFromEndAsync(self: *AsyncReader, delta: i64) !ReaderSeekOp {
        const result = c.od_zig_reader_seek_from_end_start(self.inner orelse return error.Internal, delta);
        try types.codeToError(result.code);
        return ReaderSeekOp.init(self.runtime, result.value);
    }
};

pub const AsyncWriter = struct {
    runtime: *Runtime,
    inner: ?*c.od_zig_async_writer,

    pub fn deinit(self: *AsyncWriter) void {
        if (self.inner) |ptr| {
            c.od_zig_async_writer_free(ptr);
            self.inner = null;
        }
    }

    pub fn writeAsync(self: *AsyncWriter, data: []const u8) !WriterWriteOp {
        const result = c.od_zig_writer_write_start(self.inner orelse return error.Internal, types.sliceToRaw(data));
        try types.codeToError(result.code);
        return WriterWriteOp.init(self.runtime, result.value);
    }

    pub fn closeAsync(self: *AsyncWriter) !WriterCloseOp {
        const result = c.od_zig_writer_close_start(self.inner orelse return error.Internal);
        try types.codeToError(result.code);
        return WriterCloseOp.init(self.runtime, result.value);
    }
};

pub const AsyncLister = struct {
    runtime: *Runtime,
    inner: ?*c.od_zig_async_lister,

    pub fn deinit(self: *AsyncLister) void {
        if (self.inner) |ptr| {
            c.od_zig_async_lister_free(ptr);
            self.inner = null;
        }
    }

    pub fn nextAsync(self: *AsyncLister) !ListNextOp {
        const result = c.od_zig_lister_next_owned_start(self.inner orelse return error.Internal);
        try types.codeToError(result.code);
        return ListNextOp.init(self.runtime, result.value);
    }
};

pub fn readerFromTask(runtime: *Runtime, task: ?*c.od_zig_task) !AsyncReader {
    const task_ptr = task orelse return error.Internal;
    defer c.od_zig_task_free(task_ptr);
    try runtime.waitForTask(task_ptr, null);
    const result = c.od_zig_task_take_reader(task_ptr);
    try types.codeToError(result.code);
    return .{
        .runtime = runtime,
        .inner = result.value orelse return error.Internal,
    };
}

pub fn writerFromTask(runtime: *Runtime, task: ?*c.od_zig_task) !AsyncWriter {
    const task_ptr = task orelse return error.Internal;
    defer c.od_zig_task_free(task_ptr);
    try runtime.waitForTask(task_ptr, null);
    const result = c.od_zig_task_take_writer(task_ptr);
    try types.codeToError(result.code);
    return .{
        .runtime = runtime,
        .inner = result.value orelse return error.Internal,
    };
}

pub fn listerFromTask(runtime: *Runtime, task: ?*c.od_zig_task) !AsyncLister {
    const task_ptr = task orelse return error.Internal;
    defer c.od_zig_task_free(task_ptr);
    try runtime.waitForTask(task_ptr, null);
    const result = c.od_zig_task_take_lister(task_ptr);
    try types.codeToError(result.code);
    return .{
        .runtime = runtime,
        .inner = result.value orelse return error.Internal,
    };
}