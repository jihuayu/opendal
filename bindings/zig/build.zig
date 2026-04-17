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

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const use_llvm = b.option(bool, "use-llvm", "Use LLVM backend (default: true)") orelse true;

    const native_binding = b.addTranslateC(.{
        .optimize = optimize,
        .target = target,
        .link_libc = true,
        .root_source_file = b.path("native/include/opendal_zig.h"),
    });
    const native_header_module = native_binding.createModule();

    const cargo_args = switch (optimize) {
        .Debug => &[_][]const u8{ "cargo", "build", "--manifest-path", "native/Cargo.toml" },
        else => &[_][]const u8{ "cargo", "build", "--release", "--manifest-path", "native/Cargo.toml" },
    };
    const cargo_build = b.addSystemCommand(cargo_args);
    const native_step = b.step("native", "Build the Zig native Rust runtime");
    native_step.dependOn(&cargo_build.step);

    const native_lib_dir = switch (optimize) {
        .Debug => b.path("native/target/debug"),
        else => b.path("native/target/release"),
    };

    const opendal_module = b.addModule("opendal", .{
        .root_source_file = b.path("src/opendal.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    opendal_module.addImport("opendal_zig_header", native_header_module);
    opendal_module.addLibraryPath(native_lib_dir);
    opendal_module.linkSystemLibrary("opendal_zig_native", .{});
    opendal_module.linkSystemLibrary("iconv", .{});
    opendal_module.linkFramework("Security", .{});
    opendal_module.linkFramework("CoreFoundation", .{});

    const lib = b.addLibrary(.{
        .name = "opendal",
        .root_module = opendal_module,
        .use_llvm = use_llvm,
    });
    lib.step.dependOn(&cargo_build.step);
    b.installArtifact(lib);

    const lib_test_module = b.createModule(.{
        .root_source_file = b.path("src/opendal.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    lib_test_module.addImport("opendal_zig_header", native_header_module);
    lib_test_module.addLibraryPath(native_lib_dir);
    lib_test_module.linkSystemLibrary("opendal_zig_native", .{});
    lib_test_module.linkSystemLibrary("iconv", .{});
    lib_test_module.linkFramework("Security", .{});
    lib_test_module.linkFramework("CoreFoundation", .{});

    const lib_test = b.addTest(.{
        .root_module = lib_test_module,
        .use_llvm = use_llvm,
    });
    lib_test.step.dependOn(&cargo_build.step);

    const bdd_test_module = b.createModule(.{
        .root_source_file = b.path("test/bdd.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
        .imports = &.{.{ .name = "opendal", .module = opendal_module }},
    });
    bdd_test_module.addLibraryPath(native_lib_dir);
    bdd_test_module.linkSystemLibrary("opendal_zig_native", .{});
    bdd_test_module.linkSystemLibrary("iconv", .{});
    bdd_test_module.linkFramework("Security", .{});
    bdd_test_module.linkFramework("CoreFoundation", .{});

    const bdd_test = b.addTest(.{
        .name = "bdd_test",
        .root_module = bdd_test_module,
        .use_llvm = use_llvm,
    });
    bdd_test.step.dependOn(&cargo_build.step);

    const run_lib_test = b.addRunArtifact(lib_test);
    const run_bdd_test = b.addRunArtifact(bdd_test);
    const test_step = b.step("test", "Run OpenDAL Zig bindings tests");
    test_step.dependOn(&cargo_build.step);
    test_step.dependOn(&run_lib_test.step);
    test_step.dependOn(&run_bdd_test.step);

    const sync_example_module = b.createModule(.{
        .root_source_file = b.path("examples/sync_memory.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
        .imports = &.{.{ .name = "opendal", .module = opendal_module }},
    });
    sync_example_module.addLibraryPath(native_lib_dir);
    sync_example_module.linkSystemLibrary("opendal_zig_native", .{});
    sync_example_module.linkSystemLibrary("iconv", .{});
    sync_example_module.linkFramework("Security", .{});
    sync_example_module.linkFramework("CoreFoundation", .{});

    const sync_example = b.addExecutable(.{
        .name = "opendal-zig-sync-memory-example",
        .root_module = sync_example_module,
        .use_llvm = use_llvm,
    });
    sync_example.step.dependOn(&cargo_build.step);

    const async_example_module = b.createModule(.{
        .root_source_file = b.path("examples/async_memory.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
        .imports = &.{.{ .name = "opendal", .module = opendal_module }},
    });
    async_example_module.addLibraryPath(native_lib_dir);
    async_example_module.linkSystemLibrary("opendal_zig_native", .{});
    async_example_module.linkSystemLibrary("iconv", .{});
    async_example_module.linkFramework("Security", .{});
    async_example_module.linkFramework("CoreFoundation", .{});

    const async_example = b.addExecutable(.{
        .name = "opendal-zig-async-memory-example",
        .root_module = async_example_module,
        .use_llvm = use_llvm,
    });
    async_example.step.dependOn(&cargo_build.step);

    b.installArtifact(sync_example);
    b.installArtifact(async_example);

    const run_sync_example = b.addRunArtifact(sync_example);
    const run_async_example = b.addRunArtifact(async_example);

    const example_sync_step = b.step("example-sync-memory", "Run the synchronous memory example");
    example_sync_step.dependOn(&cargo_build.step);
    example_sync_step.dependOn(&run_sync_example.step);

    const example_async_step = b.step("example-async-memory", "Run the asynchronous memory example");
    example_async_step.dependOn(&cargo_build.step);
    example_async_step.dependOn(&run_async_example.step);

    const examples_step = b.step("examples", "Build and run Zig examples");
    examples_step.dependOn(example_sync_step);
    examples_step.dependOn(example_async_step);
}
