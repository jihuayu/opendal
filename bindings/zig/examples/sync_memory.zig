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

    var op = try opendal.Operator.initKnown(.memory, &.{});
    defer op.deinit();

    try op.write("notes/hello.txt", "Hello from Zig", .{});

    var bytes = try op.readBytes("notes/hello.txt", .{});
    defer bytes.deinit();

    const copied = try bytes.cloneAlloc(gpa);
    defer gpa.free(copied);

    var metadata = try op.statAlloc(gpa, "notes/hello.txt", .{});
    defer metadata.deinit(gpa);

    std.debug.print("sync read: {s}\n", .{bytes.slice()});
    std.debug.print("sync copied bytes: {s}\n", .{copied});
    std.debug.print("content length: {?}\n", .{metadata.content_length});

    var lister = try op.lister("notes/", .{});
    defer lister.deinit();

    while (try lister.next()) |entry| {
        std.debug.print("entry path={s} name={s}\n", .{ entry.path(), entry.name() });
    }
}
