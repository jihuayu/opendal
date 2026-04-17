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

pub const c = @import("opendal_zig_header");

const Allocator = std.mem.Allocator;

pub const Error = error{
    Unexpected,
    Unsupported,
    ConfigInvalid,
    NotFound,
    PermissionDenied,
    IsDirectory,
    NotADirectory,
    AlreadyExists,
    RateLimited,
    IsSameFile,
    ConditionNotMatch,
    RangeNotSatisfied,
    InvalidArgument,
    AsyncRuntimeRequired,
    Canceled,
    OutOfMemory,
    Internal,
};

pub const ErrorCode = enum {
    ok,
    unexpected,
    unsupported,
    config_invalid,
    not_found,
    permission_denied,
    is_directory,
    not_a_directory,
    already_exists,
    rate_limited,
    is_same_file,
    condition_not_match,
    range_not_satisfied,
    invalid_argument,
    async_runtime_required,
    canceled,
    out_of_memory,
    internal,

    pub fn fromRaw(code: c.od_zig_error_code) ErrorCode {
        return switch (code) {
            c.OD_ZIG_OK => .ok,
            c.OD_ZIG_UNEXPECTED => .unexpected,
            c.OD_ZIG_UNSUPPORTED => .unsupported,
            c.OD_ZIG_CONFIG_INVALID => .config_invalid,
            c.OD_ZIG_NOT_FOUND => .not_found,
            c.OD_ZIG_PERMISSION_DENIED => .permission_denied,
            c.OD_ZIG_IS_DIRECTORY => .is_directory,
            c.OD_ZIG_NOT_A_DIRECTORY => .not_a_directory,
            c.OD_ZIG_ALREADY_EXISTS => .already_exists,
            c.OD_ZIG_RATE_LIMITED => .rate_limited,
            c.OD_ZIG_IS_SAME_FILE => .is_same_file,
            c.OD_ZIG_CONDITION_NOT_MATCH => .condition_not_match,
            c.OD_ZIG_RANGE_NOT_SATISFIED => .range_not_satisfied,
            c.OD_ZIG_INVALID_ARGUMENT => .invalid_argument,
            c.OD_ZIG_ASYNC_RUNTIME_REQUIRED => .async_runtime_required,
            c.OD_ZIG_CANCELED => .canceled,
            c.OD_ZIG_OUT_OF_MEMORY => .out_of_memory,
            c.OD_ZIG_INTERNAL => .internal,
            else => .internal,
        };
    }
};

pub fn codeToError(code: c.od_zig_error_code) Error!void {
    switch (code) {
        c.OD_ZIG_OK => {},
        c.OD_ZIG_UNEXPECTED => return error.Unexpected,
        c.OD_ZIG_UNSUPPORTED => return error.Unsupported,
        c.OD_ZIG_CONFIG_INVALID => return error.ConfigInvalid,
        c.OD_ZIG_NOT_FOUND => return error.NotFound,
        c.OD_ZIG_PERMISSION_DENIED => return error.PermissionDenied,
        c.OD_ZIG_IS_DIRECTORY => return error.IsDirectory,
        c.OD_ZIG_NOT_A_DIRECTORY => return error.NotADirectory,
        c.OD_ZIG_ALREADY_EXISTS => return error.AlreadyExists,
        c.OD_ZIG_RATE_LIMITED => return error.RateLimited,
        c.OD_ZIG_IS_SAME_FILE => return error.IsSameFile,
        c.OD_ZIG_CONDITION_NOT_MATCH => return error.ConditionNotMatch,
        c.OD_ZIG_RANGE_NOT_SATISFIED => return error.RangeNotSatisfied,
        c.OD_ZIG_INVALID_ARGUMENT => return error.InvalidArgument,
        c.OD_ZIG_ASYNC_RUNTIME_REQUIRED => return error.AsyncRuntimeRequired,
        c.OD_ZIG_CANCELED => return error.Canceled,
        c.OD_ZIG_OUT_OF_MEMORY => return error.OutOfMemory,
        c.OD_ZIG_INTERNAL => return error.Internal,
        else => return error.Internal,
    }
}

pub fn sliceToRaw(value: []const u8) c.od_zig_slice {
    return .{
        .ptr = if (value.len == 0) null else value.ptr,
        .len = value.len,
    };
}

pub fn optionalSliceToRaw(value: ?[]const u8) c.od_zig_slice {
    return if (value) |slice| sliceToRaw(slice) else .{ .ptr = null, .len = 0 };
}

pub fn mutSliceToRaw(value: []u8) c.od_zig_mut_slice {
    return .{
        .ptr = if (value.len == 0) null else value.ptr,
        .len = value.len,
    };
}

pub fn rawToSlice(value: c.od_zig_slice) []const u8 {
    if (value.ptr == null or value.len == 0) {
        return &.{};
    }
    const ptr: [*]const u8 = @ptrCast(value.ptr);
    return ptr[0..value.len];
}

pub fn rawOptionPtr(value: []const Option) ?*const c.od_zig_option {
    return if (value.len == 0) null else @ptrCast(value.ptr);
}

pub fn rawHeaderPtr(value: []const HeaderView) ?*const c.od_zig_header {
    return if (value.len == 0) null else @ptrCast(value.ptr);
}

fn rawHeaderSlice(ptr: ?*c.od_zig_header, len: usize) []const HeaderView {
    if (ptr == null or len == 0) {
        return &.{};
    }
    const raw_ptr: [*]const HeaderView = @ptrCast(ptr);
    return raw_ptr[0..len];
}

fn cloneOptionalBytes(allocator: Allocator, raw: c.od_zig_slice) !?[]u8 {
    if (raw.ptr == null) {
        return null;
    }
    return try allocator.dupe(u8, rawToSlice(raw));
}

fn cloneHeaderAlloc(allocator: Allocator, view: HeaderView) !Header {
    return .{
        .key = try allocator.dupe(u8, view.keyBytes()),
        .value = try allocator.dupe(u8, view.valueBytes()),
    };
}

fn cloneHeaderSliceAlloc(allocator: Allocator, views: []const HeaderView) ![]Header {
    const headers = try allocator.alloc(Header, views.len);
    errdefer allocator.free(headers);

    var index: usize = 0;
    errdefer {
        for (headers[0..index]) |*header| {
            header.deinit(allocator);
        }
    }

    while (index < views.len) : (index += 1) {
        headers[index] = try cloneHeaderAlloc(allocator, views[index]);
    }

    return headers;
}

pub const Poll = union(enum) {
    pending,
    ready: anyopaque,
};

pub const RuntimeOptions = struct {
    worker_threads: ?u16 = null,
    completion_queue_capacity: usize = 1024,
    cancellation_check_interval_ms: u32 = 10,

    pub fn toRaw(self: RuntimeOptions) c.od_zig_runtime_options {
        return .{
            .has_worker_threads = self.worker_threads != null,
            .worker_threads = self.worker_threads orelse 0,
            .completion_queue_capacity = self.completion_queue_capacity,
            .cancellation_check_interval_ms = self.cancellation_check_interval_ms,
        };
    }
};

pub const Option = extern struct {
    key: c.od_zig_slice = .{ .ptr = null, .len = 0 },
    value: c.od_zig_slice = .{ .ptr = null, .len = 0 },

    pub fn init(key: []const u8, value: []const u8) Option {
        return .{
            .key = sliceToRaw(key),
            .value = sliceToRaw(value),
        };
    }

    pub fn keyBytes(self: Option) []const u8 {
        return rawToSlice(self.key);
    }

    pub fn valueBytes(self: Option) []const u8 {
        return rawToSlice(self.value);
    }
};

pub const KnownScheme = enum {
    memory,
    fs,
    s3,
    gcs,
    azblob,
    azdls,
    oss,
    obs,
    webdav,
    http,

    pub fn bytes(self: KnownScheme) []const u8 {
        return switch (self) {
            .memory => "memory",
            .fs => "fs",
            .s3 => "s3",
            .gcs => "gcs",
            .azblob => "azblob",
            .azdls => "azdls",
            .oss => "oss",
            .obs => "obs",
            .webdav => "webdav",
            .http => "http",
        };
    }
};

pub const HeaderView = extern struct {
    key: c.od_zig_slice = .{ .ptr = null, .len = 0 },
    value: c.od_zig_slice = .{ .ptr = null, .len = 0 },

    pub fn init(key: []const u8, value: []const u8) HeaderView {
        return .{
            .key = sliceToRaw(key),
            .value = sliceToRaw(value),
        };
    }

    pub fn keyBytes(self: HeaderView) []const u8 {
        return rawToSlice(self.key);
    }

    pub fn valueBytes(self: HeaderView) []const u8 {
        return rawToSlice(self.value);
    }
};

pub const Header = struct {
    key: []u8,
    value: []u8,

    pub fn deinit(self: *Header, allocator: Allocator) void {
        allocator.free(self.key);
        allocator.free(self.value);
        self.* = undefined;
    }
};

pub const ReadOptions = struct {
    version: ?[]const u8 = null,
    if_match: ?[]const u8 = null,
    if_none_match: ?[]const u8 = null,
    if_modified_since_http_date: ?[]const u8 = null,
    if_unmodified_since_http_date: ?[]const u8 = null,
    override_content_type: ?[]const u8 = null,
    override_cache_control: ?[]const u8 = null,
    override_content_disposition: ?[]const u8 = null,
    offset: ?u64 = null,
    size: ?u64 = null,

    pub fn toRaw(self: ReadOptions) c.od_zig_read_options {
        return .{
            .version = optionalSliceToRaw(self.version),
            .if_match = optionalSliceToRaw(self.if_match),
            .if_none_match = optionalSliceToRaw(self.if_none_match),
            .if_modified_since_http_date = optionalSliceToRaw(self.if_modified_since_http_date),
            .if_unmodified_since_http_date = optionalSliceToRaw(self.if_unmodified_since_http_date),
            .override_content_type = optionalSliceToRaw(self.override_content_type),
            .override_cache_control = optionalSliceToRaw(self.override_cache_control),
            .override_content_disposition = optionalSliceToRaw(self.override_content_disposition),
            .has_offset = self.offset != null,
            .offset = self.offset orelse 0,
            .has_size = self.size != null,
            .size = self.size orelse 0,
        };
    }
};

pub const WriteOptions = struct {
    content_type: ?[]const u8 = null,
    content_disposition: ?[]const u8 = null,
    content_encoding: ?[]const u8 = null,
    cache_control: ?[]const u8 = null,
    if_match: ?[]const u8 = null,
    if_none_match: ?[]const u8 = null,
    if_not_exists: bool = false,
    append: bool = false,
    user_metadata: []const HeaderView = &.{},

    pub fn toRaw(self: WriteOptions) c.od_zig_write_options {
        return .{
            .content_type = optionalSliceToRaw(self.content_type),
            .content_disposition = optionalSliceToRaw(self.content_disposition),
            .content_encoding = optionalSliceToRaw(self.content_encoding),
            .cache_control = optionalSliceToRaw(self.cache_control),
            .if_match = optionalSliceToRaw(self.if_match),
            .if_none_match = optionalSliceToRaw(self.if_none_match),
            .if_not_exists = self.if_not_exists,
            .append = self.append,
            .user_metadata_ptr = rawHeaderPtr(self.user_metadata),
            .user_metadata_len = self.user_metadata.len,
        };
    }
};

pub const StatOptions = struct {
    if_match: ?[]const u8 = null,
    if_none_match: ?[]const u8 = null,
    if_modified_since_http_date: ?[]const u8 = null,
    if_unmodified_since_http_date: ?[]const u8 = null,
    override_content_type: ?[]const u8 = null,
    override_cache_control: ?[]const u8 = null,
    override_content_disposition: ?[]const u8 = null,
    version: ?[]const u8 = null,

    pub fn toRaw(self: StatOptions) c.od_zig_stat_options {
        return .{
            .if_match = optionalSliceToRaw(self.if_match),
            .if_none_match = optionalSliceToRaw(self.if_none_match),
            .if_modified_since_http_date = optionalSliceToRaw(self.if_modified_since_http_date),
            .if_unmodified_since_http_date = optionalSliceToRaw(self.if_unmodified_since_http_date),
            .override_content_type = optionalSliceToRaw(self.override_content_type),
            .override_cache_control = optionalSliceToRaw(self.override_cache_control),
            .override_content_disposition = optionalSliceToRaw(self.override_content_disposition),
            .version = optionalSliceToRaw(self.version),
        };
    }
};

pub const DeleteOptions = struct {
    version: ?[]const u8 = null,
    recursive: bool = false,

    pub fn toRaw(self: DeleteOptions) c.od_zig_delete_options {
        return .{
            .version = optionalSliceToRaw(self.version),
            .recursive = self.recursive,
        };
    }
};

pub const CopyOptions = struct {
    if_not_exists: bool = false,

    pub fn toRaw(self: CopyOptions) c.od_zig_copy_options {
        return .{ .if_not_exists = self.if_not_exists };
    }
};

pub const ListOptions = struct {
    recursive: bool = false,
    limit: ?usize = null,
    start_after: ?[]const u8 = null,
    versions: bool = false,
    deleted: bool = false,

    pub fn toRaw(self: ListOptions) c.od_zig_list_options {
        return .{
            .recursive = self.recursive,
            .has_limit = self.limit != null,
            .limit = self.limit orelse 0,
            .start_after = optionalSliceToRaw(self.start_after),
            .versions = self.versions,
            .deleted = self.deleted,
        };
    }
};

pub const PresignOptions = struct {
    version: ?[]const u8 = null,
    if_match: ?[]const u8 = null,
    if_none_match: ?[]const u8 = null,
    if_modified_since_http_date: ?[]const u8 = null,
    if_unmodified_since_http_date: ?[]const u8 = null,
    override_content_type: ?[]const u8 = null,
    override_cache_control: ?[]const u8 = null,
    override_content_disposition: ?[]const u8 = null,
    content_encoding: ?[]const u8 = null,
    if_not_exists: bool = false,

    pub fn toRaw(self: PresignOptions) c.od_zig_presign_options {
        return .{
            .version = optionalSliceToRaw(self.version),
            .if_match = optionalSliceToRaw(self.if_match),
            .if_none_match = optionalSliceToRaw(self.if_none_match),
            .if_modified_since_http_date = optionalSliceToRaw(self.if_modified_since_http_date),
            .if_unmodified_since_http_date = optionalSliceToRaw(self.if_unmodified_since_http_date),
            .override_content_type = optionalSliceToRaw(self.override_content_type),
            .override_cache_control = optionalSliceToRaw(self.override_cache_control),
            .override_content_disposition = optionalSliceToRaw(self.override_content_disposition),
            .content_encoding = optionalSliceToRaw(self.content_encoding),
            .if_not_exists = self.if_not_exists,
        };
    }
};

pub const EntryMode = enum {
    file,
    dir,
    unknown,

    fn fromRaw(mode: u8) EntryMode {
        return switch (mode) {
            @as(u8, @intCast(c.OD_ZIG_ENTRY_MODE_FILE)) => .file,
            @as(u8, @intCast(c.OD_ZIG_ENTRY_MODE_DIR)) => .dir,
            else => .unknown,
        };
    }
};

pub const Capability = struct {
    stat: bool,
    stat_with_if_match: bool,
    stat_with_if_none_match: bool,
    stat_with_if_modified_since: bool,
    stat_with_if_unmodified_since: bool,
    stat_with_override_cache_control: bool,
    stat_with_override_content_disposition: bool,
    stat_with_override_content_type: bool,
    stat_with_version: bool,
    read: bool,
    read_with_if_match: bool,
    read_with_if_none_match: bool,
    read_with_if_modified_since: bool,
    read_with_if_unmodified_since: bool,
    read_with_override_cache_control: bool,
    read_with_override_content_disposition: bool,
    read_with_override_content_type: bool,
    read_with_version: bool,
    write: bool,
    write_can_multi: bool,
    write_can_empty: bool,
    write_can_append: bool,
    write_with_content_type: bool,
    write_with_content_disposition: bool,
    write_with_content_encoding: bool,
    write_with_cache_control: bool,
    write_with_if_match: bool,
    write_with_if_none_match: bool,
    write_with_if_not_exists: bool,
    write_with_user_metadata: bool,
    write_multi_max_size: ?usize,
    write_multi_min_size: ?usize,
    write_total_max_size: ?usize,
    create_dir: bool,
    delete: bool,
    delete_with_version: bool,
    delete_with_recursive: bool,
    delete_max_size: ?usize,
    copy: bool,
    copy_with_if_not_exists: bool,
    rename: bool,
    list: bool,
    list_with_limit: bool,
    list_with_start_after: bool,
    list_with_recursive: bool,
    list_with_versions: bool,
    list_with_deleted: bool,
    presign: bool,
    presign_read: bool,
    presign_stat: bool,
    presign_write: bool,
    presign_delete: bool,
    shared: bool,

    pub fn fromRaw(raw: c.od_zig_capability) Capability {
        return .{
            .stat = raw.stat,
            .stat_with_if_match = raw.stat_with_if_match,
            .stat_with_if_none_match = raw.stat_with_if_none_match,
            .stat_with_if_modified_since = raw.stat_with_if_modified_since,
            .stat_with_if_unmodified_since = raw.stat_with_if_unmodified_since,
            .stat_with_override_cache_control = raw.stat_with_override_cache_control,
            .stat_with_override_content_disposition = raw.stat_with_override_content_disposition,
            .stat_with_override_content_type = raw.stat_with_override_content_type,
            .stat_with_version = raw.stat_with_version,
            .read = raw.read,
            .read_with_if_match = raw.read_with_if_match,
            .read_with_if_none_match = raw.read_with_if_none_match,
            .read_with_if_modified_since = raw.read_with_if_modified_since,
            .read_with_if_unmodified_since = raw.read_with_if_unmodified_since,
            .read_with_override_cache_control = raw.read_with_override_cache_control,
            .read_with_override_content_disposition = raw.read_with_override_content_disposition,
            .read_with_override_content_type = raw.read_with_override_content_type,
            .read_with_version = raw.read_with_version,
            .write = raw.write,
            .write_can_multi = raw.write_can_multi,
            .write_can_empty = raw.write_can_empty,
            .write_can_append = raw.write_can_append,
            .write_with_content_type = raw.write_with_content_type,
            .write_with_content_disposition = raw.write_with_content_disposition,
            .write_with_content_encoding = raw.write_with_content_encoding,
            .write_with_cache_control = raw.write_with_cache_control,
            .write_with_if_match = raw.write_with_if_match,
            .write_with_if_none_match = raw.write_with_if_none_match,
            .write_with_if_not_exists = raw.write_with_if_not_exists,
            .write_with_user_metadata = raw.write_with_user_metadata,
            .write_multi_max_size = if (raw.has_write_multi_max_size) raw.write_multi_max_size else null,
            .write_multi_min_size = if (raw.has_write_multi_min_size) raw.write_multi_min_size else null,
            .write_total_max_size = if (raw.has_write_total_max_size) raw.write_total_max_size else null,
            .create_dir = raw.create_dir,
            .delete = raw.delete,
            .delete_with_version = raw.delete_with_version,
            .delete_with_recursive = raw.delete_with_recursive,
            .delete_max_size = if (raw.has_delete_max_size) raw.delete_max_size else null,
            .copy = raw.copy,
            .copy_with_if_not_exists = raw.copy_with_if_not_exists,
            .rename = raw.rename,
            .list = raw.list,
            .list_with_limit = raw.list_with_limit,
            .list_with_start_after = raw.list_with_start_after,
            .list_with_recursive = raw.list_with_recursive,
            .list_with_versions = raw.list_with_versions,
            .list_with_deleted = raw.list_with_deleted,
            .presign = raw.presign,
            .presign_read = raw.presign_read,
            .presign_stat = raw.presign_stat,
            .presign_write = raw.presign_write,
            .presign_delete = raw.presign_delete,
            .shared = raw.shared,
        };
    }
};

pub const MetadataView = struct {
    raw: c.od_zig_metadata_view,

    pub fn entryMode(self: MetadataView) EntryMode {
        return EntryMode.fromRaw(self.raw.mode);
    }

    pub fn isFile(self: MetadataView) bool {
        return self.raw.is_file;
    }

    pub fn isDir(self: MetadataView) bool {
        return self.raw.is_dir;
    }

    pub fn isDeleted(self: MetadataView) bool {
        return self.raw.is_deleted;
    }

    pub fn contentLength(self: MetadataView) ?u64 {
        return if (self.raw.has_content_length) self.raw.content_length else null;
    }

    pub fn contentType(self: MetadataView) ?[]const u8 {
        return if (self.raw.content_type.ptr == null) null else rawToSlice(self.raw.content_type);
    }

    pub fn contentEncoding(self: MetadataView) ?[]const u8 {
        return if (self.raw.content_encoding.ptr == null) null else rawToSlice(self.raw.content_encoding);
    }

    pub fn contentDisposition(self: MetadataView) ?[]const u8 {
        return if (self.raw.content_disposition.ptr == null) null else rawToSlice(self.raw.content_disposition);
    }

    pub fn contentMd5(self: MetadataView) ?[]const u8 {
        return if (self.raw.content_md5.ptr == null) null else rawToSlice(self.raw.content_md5);
    }

    pub fn etag(self: MetadataView) ?[]const u8 {
        return if (self.raw.etag.ptr == null) null else rawToSlice(self.raw.etag);
    }

    pub fn lastModifiedRfc3339(self: MetadataView) ?[]const u8 {
        return if (self.raw.last_modified_rfc3339.ptr == null) null else rawToSlice(self.raw.last_modified_rfc3339);
    }

    pub fn version(self: MetadataView) ?[]const u8 {
        return if (self.raw.version.ptr == null) null else rawToSlice(self.raw.version);
    }

    pub fn userMetadata(self: MetadataView) []const HeaderView {
        return rawHeaderSlice(self.raw.user_metadata_ptr, self.raw.user_metadata_len);
    }

    pub fn cloneAlloc(self: MetadataView, allocator: Allocator) !Metadata {
        return cloneMetadataAlloc(allocator, self.raw);
    }
};

pub const Metadata = struct {
    mode: EntryMode,
    is_file: bool,
    is_dir: bool,
    is_deleted: bool,
    content_length: ?u64,
    content_type: ?[]u8,
    content_encoding: ?[]u8,
    content_disposition: ?[]u8,
    content_md5: ?[]u8,
    etag: ?[]u8,
    last_modified_rfc3339: ?[]u8,
    version: ?[]u8,
    user_metadata: []Header,

    pub fn deinit(self: *Metadata, allocator: Allocator) void {
        if (self.content_type) |value| allocator.free(value);
        if (self.content_encoding) |value| allocator.free(value);
        if (self.content_disposition) |value| allocator.free(value);
        if (self.content_md5) |value| allocator.free(value);
        if (self.etag) |value| allocator.free(value);
        if (self.last_modified_rfc3339) |value| allocator.free(value);
        if (self.version) |value| allocator.free(value);
        for (self.user_metadata) |*header| {
            header.deinit(allocator);
        }
        allocator.free(self.user_metadata);
        self.* = undefined;
    }
};

fn cloneMetadataAlloc(allocator: Allocator, raw: c.od_zig_metadata_view) !Metadata {
    return .{
        .mode = EntryMode.fromRaw(raw.mode),
        .is_file = raw.is_file,
        .is_dir = raw.is_dir,
        .is_deleted = raw.is_deleted,
        .content_length = if (raw.has_content_length) raw.content_length else null,
        .content_type = try cloneOptionalBytes(allocator, raw.content_type),
        .content_encoding = try cloneOptionalBytes(allocator, raw.content_encoding),
        .content_disposition = try cloneOptionalBytes(allocator, raw.content_disposition),
        .content_md5 = try cloneOptionalBytes(allocator, raw.content_md5),
        .etag = try cloneOptionalBytes(allocator, raw.etag),
        .last_modified_rfc3339 = try cloneOptionalBytes(allocator, raw.last_modified_rfc3339),
        .version = try cloneOptionalBytes(allocator, raw.version),
        .user_metadata = try cloneHeaderSliceAlloc(allocator, rawHeaderSlice(raw.user_metadata_ptr, raw.user_metadata_len)),
    };
}

pub fn metadataFromOwned(allocator: Allocator, raw: ?*c.od_zig_metadata) !Metadata {
    const ptr = raw orelse return error.Internal;
    defer c.od_zig_metadata_free(ptr);
    return cloneMetadataAlloc(allocator, ptr.*);
}

pub const EntryView = struct {
    raw: c.od_zig_entry_view,

    pub fn path(self: EntryView) []const u8 {
        return rawToSlice(self.raw.path);
    }

    pub fn name(self: EntryView) []const u8 {
        return rawToSlice(self.raw.name);
    }

    pub fn metadata(self: EntryView) MetadataView {
        return .{ .raw = self.raw.metadata };
    }

    pub fn cloneAlloc(self: EntryView, allocator: Allocator) !Entry {
        return .{
            .path = try allocator.dupe(u8, self.path()),
            .name_storage = try allocator.dupe(u8, self.name()),
            .metadata = try self.metadata().cloneAlloc(allocator),
        };
    }
};

pub const Entry = struct {
    path: []u8,
    name_storage: []u8,
    metadata: Metadata,

    pub fn deinit(self: *Entry, allocator: Allocator) void {
        allocator.free(self.path);
        allocator.free(self.name_storage);
        self.metadata.deinit(allocator);
        self.* = undefined;
    }

    pub fn name(self: Entry) []const u8 {
        return self.name_storage;
    }
};

pub fn entryFromOwned(allocator: Allocator, raw: ?*c.od_zig_entry_owned) !Entry {
    const ptr = raw orelse return error.Internal;
    defer c.od_zig_entry_owned_free(ptr);

    return .{
        .path = try allocator.dupe(u8, rawToSlice(ptr.path)),
        .name_storage = try allocator.dupe(u8, rawToSlice(ptr.name)),
        .metadata = try cloneMetadataAlloc(allocator, ptr.metadata),
    };
}

pub const OperatorInfo = struct {
    scheme: []u8,
    root: []u8,
    name: []u8,
    full_capability: Capability,
    native_capability: Capability,

    pub fn deinit(self: *OperatorInfo, allocator: Allocator) void {
        allocator.free(self.scheme);
        allocator.free(self.root);
        allocator.free(self.name);
        self.* = undefined;
    }
};

pub fn infoFromOwned(allocator: Allocator, raw: ?*c.od_zig_operator_info) !OperatorInfo {
    const ptr = raw orelse return error.Internal;
    defer c.od_zig_info_free(ptr);

    return .{
        .scheme = try allocator.dupe(u8, rawToSlice(ptr.scheme)),
        .root = try allocator.dupe(u8, rawToSlice(ptr.root)),
        .name = try allocator.dupe(u8, rawToSlice(ptr.name)),
        .full_capability = Capability.fromRaw(ptr.full_capability),
        .native_capability = Capability.fromRaw(ptr.native_capability),
    };
}

pub const PresignedRequest = struct {
    method: []u8,
    url: []u8,
    headers: []Header,

    pub fn deinit(self: *PresignedRequest, allocator: Allocator) void {
        allocator.free(self.method);
        allocator.free(self.url);
        for (self.headers) |*header| {
            header.deinit(allocator);
        }
        allocator.free(self.headers);
        self.* = undefined;
    }
};

pub fn presignedRequestFromOwned(
    allocator: Allocator,
    raw: ?*c.od_zig_presigned_request,
) !PresignedRequest {
    const ptr = raw orelse return error.Internal;
    defer c.od_zig_presigned_request_free(ptr);

    return .{
        .method = try allocator.dupe(u8, rawToSlice(ptr.method)),
        .url = try allocator.dupe(u8, rawToSlice(ptr.url)),
        .headers = try cloneHeaderSliceAlloc(allocator, rawHeaderSlice(ptr.headers_ptr, ptr.headers_len)),
    };
}

pub const ErrorInfo = struct {
    code: ErrorCode,
    message: []u8,

    pub fn deinit(self: *ErrorInfo, allocator: Allocator) void {
        allocator.free(self.message);
        self.* = undefined;
    }
};

pub fn errorInfoFromOwned(allocator: Allocator, raw: ?*c.od_zig_error_info) !ErrorInfo {
    const ptr = raw orelse return error.Internal;
    defer c.od_zig_error_info_free(ptr);

    return .{
        .code = ErrorCode.fromRaw(ptr.code),
        .message = try allocator.dupe(u8, rawToSlice(ptr.message)),
    };
}

pub const Bytes = struct {
    inner: ?*c.od_zig_bytes,

    pub fn deinit(self: *Bytes) void {
        if (self.inner) |ptr| {
            c.od_zig_bytes_free(ptr);
            self.inner = null;
        }
    }

    pub fn len(self: Bytes) usize {
        const ptr = self.inner orelse return 0;
        return c.od_zig_bytes_len(ptr);
    }

    pub fn slice(self: Bytes) []const u8 {
        const ptr = self.inner orelse return &.{};
        const byte_len = c.od_zig_bytes_len(ptr);
        const raw_ptr = c.od_zig_bytes_ptr(ptr);
        if (raw_ptr == null or byte_len == 0) {
            return &.{};
        }
        const bytes_ptr: [*]const u8 = @ptrCast(raw_ptr);
        return bytes_ptr[0..byte_len];
    }

    pub fn cloneAlloc(self: Bytes, allocator: Allocator) ![]u8 {
        return try allocator.dupe(u8, self.slice());
    }
};

pub fn bytesFromOwned(raw: ?*c.od_zig_bytes) !Bytes {
    return .{ .inner = raw orelse return error.Internal };
}