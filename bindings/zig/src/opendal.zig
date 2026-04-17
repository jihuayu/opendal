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
const async_api = @import("async.zig");

pub const c = types.c;

pub const Error = types.Error;
pub const ErrorCode = types.ErrorCode;
pub const Runtime = async_api.Runtime;
pub const RuntimeOptions = types.RuntimeOptions;
pub const Option = types.Option;
pub const KnownScheme = types.KnownScheme;
pub const HeaderView = types.HeaderView;
pub const Header = types.Header;
pub const Capability = types.Capability;
pub const EntryMode = types.EntryMode;
pub const MetadataView = types.MetadataView;
pub const Metadata = types.Metadata;
pub const EntryView = types.EntryView;
pub const Entry = types.Entry;
pub const OperatorInfo = types.OperatorInfo;
pub const PresignedRequest = types.PresignedRequest;
pub const ErrorInfo = types.ErrorInfo;
pub const Bytes = types.Bytes;
pub const ReadOptions = types.ReadOptions;
pub const WriteOptions = types.WriteOptions;
pub const StatOptions = types.StatOptions;
pub const DeleteOptions = types.DeleteOptions;
pub const CopyOptions = types.CopyOptions;
pub const ListOptions = types.ListOptions;
pub const PresignOptions = types.PresignOptions;

pub const Poll = async_api.Poll;
pub const VoidOp = async_api.VoidOp;
pub const ExistsOp = async_api.ExistsOp;
pub const CreateDirOp = async_api.CreateDirOp;
pub const DeleteOp = async_api.DeleteOp;
pub const RenameOp = async_api.RenameOp;
pub const CopyOp = async_api.CopyOp;
pub const WriteOp = async_api.WriteCompletionOp;
pub const ReadOp = async_api.ReadOp;
pub const StatOp = async_api.StatOp;
pub const InfoOp = async_api.InfoOp;
pub const PresignOp = async_api.PresignOp;
pub const ListNextOp = async_api.ListNextOp;
pub const ReaderReadOp = async_api.ReaderReadOp;
pub const ReaderSeekOp = async_api.ReaderSeekOp;
pub const WriterWriteOp = async_api.WriterWriteOp;
pub const WriterCloseOp = async_api.WriterCloseOp;
pub const AsyncReader = async_api.AsyncReader;
pub const AsyncWriter = async_api.AsyncWriter;
pub const AsyncLister = async_api.AsyncLister;

pub const Operator = struct {
    runtime: ?*Runtime = null,
    inner: ?*c.od_zig_operator,

    pub fn init(scheme: []const u8, options: []const Option) !Operator {
        const result = c.od_zig_operator_new(types.sliceToRaw(scheme), types.rawOptionPtr(options), options.len);
        try types.codeToError(result.code);
        return .{ .inner = result.value orelse return error.Internal };
    }

    pub fn initKnown(scheme: KnownScheme, options: []const Option) !Operator {
        return init(scheme.bytes(), options);
    }

    pub fn initWithRuntime(runtime: *Runtime, scheme: []const u8, options: []const Option) !Operator {
        const result = c.od_zig_operator_new_with_runtime(
            runtime.inner orelse return error.Internal,
            types.sliceToRaw(scheme),
            types.rawOptionPtr(options),
            options.len,
        );
        try types.codeToError(result.code);
        return .{
            .runtime = runtime,
            .inner = result.value orelse return error.Internal,
        };
    }

    pub fn initKnownWithRuntime(runtime: *Runtime, scheme: KnownScheme, options: []const Option) !Operator {
        return initWithRuntime(runtime, scheme.bytes(), options);
    }

    pub fn deinit(self: *Operator) void {
        if (self.inner) |ptr| {
            c.od_zig_operator_free(ptr);
            self.inner = null;
        }
    }

    pub fn check(self: *Operator) !void {
        const result = c.od_zig_operator_check(self.operatorPtr());
        try types.codeToError(result.code);
    }

    pub fn exists(self: *Operator, path: []const u8) !bool {
        const result = c.od_zig_operator_exists(self.operatorPtr(), types.sliceToRaw(path));
        try types.codeToError(result.code);
        return result.value;
    }

    pub fn createDir(self: *Operator, path: []const u8) !void {
        const result = c.od_zig_operator_create_dir(self.operatorPtr(), types.sliceToRaw(path));
        try types.codeToError(result.code);
    }

    pub fn delete(self: *Operator, path: []const u8, options: DeleteOptions) !void {
        const result = c.od_zig_operator_delete(self.operatorPtr(), types.sliceToRaw(path), options.toRaw());
        try types.codeToError(result.code);
    }

    pub fn rename(self: *Operator, from: []const u8, to: []const u8) !void {
        const result = c.od_zig_operator_rename(self.operatorPtr(), types.sliceToRaw(from), types.sliceToRaw(to));
        try types.codeToError(result.code);
    }

    pub fn copy(self: *Operator, from: []const u8, to: []const u8, options: CopyOptions) !void {
        const result = c.od_zig_operator_copy(self.operatorPtr(), types.sliceToRaw(from), types.sliceToRaw(to), options.toRaw());
        try types.codeToError(result.code);
    }

    pub fn write(self: *Operator, path: []const u8, data: []const u8, options: WriteOptions) !void {
        const result = c.od_zig_operator_write(self.operatorPtr(), types.sliceToRaw(path), types.sliceToRaw(data), options.toRaw());
        try types.codeToError(result.code);
    }

    pub fn readBytes(self: *Operator, path: []const u8, options: ReadOptions) !Bytes {
        const result = c.od_zig_operator_read_bytes(self.operatorPtr(), types.sliceToRaw(path), options.toRaw());
        try types.codeToError(result.code);
        return try types.bytesFromOwned(result.value);
    }

    pub fn readAlloc(self: *Operator, allocator: std.mem.Allocator, path: []const u8, options: ReadOptions) ![]u8 {
        var bytes = try self.readBytes(path, options);
        defer bytes.deinit();
        return try bytes.cloneAlloc(allocator);
    }

    pub fn statAlloc(self: *Operator, allocator: std.mem.Allocator, path: []const u8, options: StatOptions) !Metadata {
        const result = c.od_zig_operator_stat(self.operatorPtr(), types.sliceToRaw(path), options.toRaw());
        try types.codeToError(result.code);
        return try types.metadataFromOwned(allocator, result.value);
    }

    pub fn infoAlloc(self: *Operator, allocator: std.mem.Allocator) !OperatorInfo {
        const result = c.od_zig_operator_info_get(self.operatorPtr());
        try types.codeToError(result.code);
        return try types.infoFromOwned(allocator, result.value);
    }

    pub fn presignReadAlloc(
        self: *Operator,
        allocator: std.mem.Allocator,
        path: []const u8,
        expire_secs: u64,
        options: PresignOptions,
    ) !PresignedRequest {
        const result = c.od_zig_operator_presign_read(self.operatorPtr(), types.sliceToRaw(path), expire_secs, options.toRaw());
        try types.codeToError(result.code);
        return try types.presignedRequestFromOwned(allocator, result.value);
    }

    pub fn presignWriteAlloc(
        self: *Operator,
        allocator: std.mem.Allocator,
        path: []const u8,
        expire_secs: u64,
        options: PresignOptions,
    ) !PresignedRequest {
        const result = c.od_zig_operator_presign_write(self.operatorPtr(), types.sliceToRaw(path), expire_secs, options.toRaw());
        try types.codeToError(result.code);
        return try types.presignedRequestFromOwned(allocator, result.value);
    }

    pub fn presignStatAlloc(
        self: *Operator,
        allocator: std.mem.Allocator,
        path: []const u8,
        expire_secs: u64,
        options: PresignOptions,
    ) !PresignedRequest {
        const result = c.od_zig_operator_presign_stat(self.operatorPtr(), types.sliceToRaw(path), expire_secs, options.toRaw());
        try types.codeToError(result.code);
        return try types.presignedRequestFromOwned(allocator, result.value);
    }

    pub fn presignDeleteAlloc(
        self: *Operator,
        allocator: std.mem.Allocator,
        path: []const u8,
        expire_secs: u64,
        options: PresignOptions,
    ) !PresignedRequest {
        const result = c.od_zig_operator_presign_delete(self.operatorPtr(), types.sliceToRaw(path), expire_secs, options.toRaw());
        try types.codeToError(result.code);
        return try types.presignedRequestFromOwned(allocator, result.value);
    }

    pub fn reader(self: *Operator, path: []const u8, options: ReadOptions) !Reader {
        const result = c.od_zig_operator_reader_open(self.operatorPtr(), types.sliceToRaw(path), options.toRaw());
        try types.codeToError(result.code);
        return .{ .inner = result.value orelse return error.Internal };
    }

    pub fn writer(self: *Operator, path: []const u8, options: WriteOptions) !Writer {
        const result = c.od_zig_operator_writer_open(self.operatorPtr(), types.sliceToRaw(path), options.toRaw());
        try types.codeToError(result.code);
        return .{ .inner = result.value orelse return error.Internal };
    }

    pub fn lister(self: *Operator, path: []const u8, options: ListOptions) !Lister {
        const result = c.od_zig_operator_lister_open(self.operatorPtr(), types.sliceToRaw(path), options.toRaw());
        try types.codeToError(result.code);
        return .{ .inner = result.value orelse return error.Internal };
    }

    pub fn checkAsync(self: *Operator) !VoidOp {
        const runtime = try self.runtimePtr();
        const result = c.od_zig_operator_check_start(self.operatorPtr());
        try types.codeToError(result.code);
        return VoidOp.init(runtime, result.value);
    }

    pub fn existsAsync(self: *Operator, path: []const u8) !ExistsOp {
        const runtime = try self.runtimePtr();
        const result = c.od_zig_operator_exists_start(self.operatorPtr(), types.sliceToRaw(path));
        try types.codeToError(result.code);
        return ExistsOp.init(runtime, result.value);
    }

    pub fn createDirAsync(self: *Operator, path: []const u8) !CreateDirOp {
        const runtime = try self.runtimePtr();
        const result = c.od_zig_operator_create_dir_start(self.operatorPtr(), types.sliceToRaw(path));
        try types.codeToError(result.code);
        return CreateDirOp.init(runtime, result.value);
    }

    pub fn deleteAsync(self: *Operator, path: []const u8, options: DeleteOptions) !DeleteOp {
        const runtime = try self.runtimePtr();
        const result = c.od_zig_operator_delete_start(self.operatorPtr(), types.sliceToRaw(path), options.toRaw());
        try types.codeToError(result.code);
        return DeleteOp.init(runtime, result.value);
    }

    pub fn renameAsync(self: *Operator, from: []const u8, to: []const u8) !RenameOp {
        const runtime = try self.runtimePtr();
        const result = c.od_zig_operator_rename_start(self.operatorPtr(), types.sliceToRaw(from), types.sliceToRaw(to));
        try types.codeToError(result.code);
        return RenameOp.init(runtime, result.value);
    }

    pub fn copyAsync(self: *Operator, from: []const u8, to: []const u8, options: CopyOptions) !CopyOp {
        const runtime = try self.runtimePtr();
        const result = c.od_zig_operator_copy_start(self.operatorPtr(), types.sliceToRaw(from), types.sliceToRaw(to), options.toRaw());
        try types.codeToError(result.code);
        return CopyOp.init(runtime, result.value);
    }

    pub fn writeAsync(self: *Operator, path: []const u8, data: []const u8, options: WriteOptions) !WriteOp {
        const runtime = try self.runtimePtr();
        const result = c.od_zig_operator_write_start(self.operatorPtr(), types.sliceToRaw(path), types.sliceToRaw(data), options.toRaw());
        try types.codeToError(result.code);
        return WriteOp.init(runtime, result.value);
    }

    pub fn readAsync(self: *Operator, path: []const u8, options: ReadOptions) !ReadOp {
        const runtime = try self.runtimePtr();
        const result = c.od_zig_operator_read_start(self.operatorPtr(), types.sliceToRaw(path), options.toRaw());
        try types.codeToError(result.code);
        return ReadOp.init(runtime, result.value);
    }

    pub fn statAsync(self: *Operator, path: []const u8, options: StatOptions) !StatOp {
        const runtime = try self.runtimePtr();
        const result = c.od_zig_operator_stat_start(self.operatorPtr(), types.sliceToRaw(path), options.toRaw());
        try types.codeToError(result.code);
        return StatOp.init(runtime, result.value);
    }

    pub fn infoAsync(self: *Operator) !InfoOp {
        const runtime = try self.runtimePtr();
        const result = c.od_zig_operator_info_start(self.operatorPtr());
        try types.codeToError(result.code);
        return InfoOp.init(runtime, result.value);
    }

    pub fn presignReadAsync(self: *Operator, path: []const u8, expire_secs: u64, options: PresignOptions) !PresignOp {
        const runtime = try self.runtimePtr();
        const result = c.od_zig_operator_presign_read_start(self.operatorPtr(), types.sliceToRaw(path), expire_secs, options.toRaw());
        try types.codeToError(result.code);
        return PresignOp.init(runtime, result.value);
    }

    pub fn presignWriteAsync(self: *Operator, path: []const u8, expire_secs: u64, options: PresignOptions) !PresignOp {
        const runtime = try self.runtimePtr();
        const result = c.od_zig_operator_presign_write_start(self.operatorPtr(), types.sliceToRaw(path), expire_secs, options.toRaw());
        try types.codeToError(result.code);
        return PresignOp.init(runtime, result.value);
    }

    pub fn presignStatAsync(self: *Operator, path: []const u8, expire_secs: u64, options: PresignOptions) !PresignOp {
        const runtime = try self.runtimePtr();
        const result = c.od_zig_operator_presign_stat_start(self.operatorPtr(), types.sliceToRaw(path), expire_secs, options.toRaw());
        try types.codeToError(result.code);
        return PresignOp.init(runtime, result.value);
    }

    pub fn presignDeleteAsync(self: *Operator, path: []const u8, expire_secs: u64, options: PresignOptions) !PresignOp {
        const runtime = try self.runtimePtr();
        const result = c.od_zig_operator_presign_delete_start(self.operatorPtr(), types.sliceToRaw(path), expire_secs, options.toRaw());
        try types.codeToError(result.code);
        return PresignOp.init(runtime, result.value);
    }

    pub fn readerAsync(self: *Operator, path: []const u8, options: ReadOptions) !AsyncReader {
        const runtime = try self.runtimePtr();
        const result = c.od_zig_operator_reader_open_start(self.operatorPtr(), types.sliceToRaw(path), options.toRaw());
        try types.codeToError(result.code);
        return try async_api.readerFromTask(runtime, result.value);
    }

    pub fn writerAsync(self: *Operator, path: []const u8, options: WriteOptions) !AsyncWriter {
        const runtime = try self.runtimePtr();
        const result = c.od_zig_operator_writer_open_start(self.operatorPtr(), types.sliceToRaw(path), options.toRaw());
        try types.codeToError(result.code);
        return try async_api.writerFromTask(runtime, result.value);
    }

    pub fn listerAsync(self: *Operator, path: []const u8, options: ListOptions) !AsyncLister {
        const runtime = try self.runtimePtr();
        const result = c.od_zig_operator_lister_open_start(self.operatorPtr(), types.sliceToRaw(path), options.toRaw());
        try types.codeToError(result.code);
        return try async_api.listerFromTask(runtime, result.value);
    }

    fn operatorPtr(self: *Operator) *c.od_zig_operator {
        return self.inner orelse @panic("operator already deinitialized");
    }

    fn runtimePtr(self: *Operator) !*Runtime {
        return self.runtime orelse error.AsyncRuntimeRequired;
    }
};

pub const Reader = struct {
    inner: ?*c.od_zig_reader,

    pub fn deinit(self: *Reader) void {
        if (self.inner) |ptr| {
            c.od_zig_reader_free(ptr);
            self.inner = null;
        }
    }

    pub fn read(self: *Reader, buf: []u8) !usize {
        const result = c.od_zig_reader_read(self.readerPtr(), types.mutSliceToRaw(buf));
        try types.codeToError(result.code);
        return result.value;
    }

    pub fn seekTo(self: *Reader, pos: u64) !u64 {
        const result = c.od_zig_reader_seek_to(self.readerPtr(), pos);
        try types.codeToError(result.code);
        return result.value;
    }

    pub fn seekBy(self: *Reader, delta: i64) !u64 {
        const result = c.od_zig_reader_seek_by(self.readerPtr(), delta);
        try types.codeToError(result.code);
        return result.value;
    }

    pub fn seekFromEnd(self: *Reader, delta: i64) !u64 {
        const result = c.od_zig_reader_seek_from_end(self.readerPtr(), delta);
        try types.codeToError(result.code);
        return result.value;
    }

    fn readerPtr(self: *Reader) *c.od_zig_reader {
        return self.inner orelse @panic("reader already deinitialized");
    }
};

pub const Writer = struct {
    inner: ?*c.od_zig_writer,

    pub fn deinit(self: *Writer) void {
        if (self.inner) |ptr| {
            c.od_zig_writer_free(ptr);
            self.inner = null;
        }
    }

    pub fn write(self: *Writer, data: []const u8) !usize {
        const result = c.od_zig_writer_write(self.writerPtr(), types.sliceToRaw(data));
        try types.codeToError(result.code);
        return result.value;
    }

    pub fn writeAll(self: *Writer, data: []const u8) !void {
        var written: usize = 0;
        while (written < data.len) {
            const step = try self.write(data[written..]);
            if (step == 0) {
                return error.Unexpected;
            }
            written += step;
        }
    }

    pub fn close(self: *Writer) !void {
        const result = c.od_zig_writer_close(self.writerPtr());
        try types.codeToError(result.code);
    }

    fn writerPtr(self: *Writer) *c.od_zig_writer {
        return self.inner orelse @panic("writer already deinitialized");
    }
};

pub const Lister = struct {
    inner: ?*c.od_zig_lister,

    pub fn deinit(self: *Lister) void {
        if (self.inner) |ptr| {
            c.od_zig_lister_free(ptr);
            self.inner = null;
        }
    }

    pub fn next(self: *Lister) !?EntryView {
        const result = c.od_zig_lister_next_view(self.listerPtr());
        try types.codeToError(result.code);
        if (!result.has_value) {
            return null;
        }
        return EntryView{ .raw = result.value };
    }

    pub fn nextAlloc(self: *Lister, allocator: std.mem.Allocator) !?Entry {
        const result = c.od_zig_lister_next_owned(self.listerPtr());
        try types.codeToError(result.code);
        if (!result.has_value) {
            return null;
        }
        return try types.entryFromOwned(allocator, result.value);
    }

    fn listerPtr(self: *Lister) *c.od_zig_lister {
        return self.inner orelse @panic("lister already deinitialized");
    }
};
