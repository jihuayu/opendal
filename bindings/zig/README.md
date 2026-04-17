# Apache OpenDAL™ Zig Binding

[![](https://img.shields.io/badge/status-unreleased-red)](https://opendal.apache.org/bindings/zig)

![](https://github.com/apache/opendal/assets/5351546/87bbf6e5-f19e-449a-b368-3e283016c887)

> **Note**: This binding has its own independent version number, which may differ from the Rust core version. When checking for updates or compatibility, always refer to this binding's version rather than the core version.

## Overview

This binding now uses a Zig-native architecture:

- `bindings/zig/src` contains the public Zig API.
- `bindings/zig/native` contains the Zig-specific Rust FFI layer built directly on top of OpenDAL core.
- The Zig package no longer depends on `bindings/c` during normal builds.

The current API surface includes synchronous operations, native async operations, zero-copy `Bytes` reads, and Zig-owned copies for small structured results such as metadata and presigned requests.

## Build

To compile OpenDAL Zig binding from source code, you need:

- [Zig](https://ziglang.org/download) 0.16.0 or higher
- a Rust toolchain matching this repository's `rust-toolchain.toml`

```bash
# build the Zig package and the native Rust layer
zig build

# build and run the Zig test suite
zig build test

# run the bundled examples
zig build example-sync-memory
zig build example-async-memory
```

`zig build test` automatically runs Cargo for `bindings/zig/native` before compiling and running the Zig tests.

## Examples

See the runnable examples under `bindings/zig/examples/`:

- `examples/sync_memory.zig`
- `examples/async_memory.zig`

```zig
const opendal = @import("opendal");

pub fn main() !void {
    var op = try opendal.Operator.initKnown(.memory, &.{});
    defer op.deinit();

    try op.write("hello.txt", "world", .{});

    var bytes = try op.readBytes("hello.txt", .{});
    defer bytes.deinit();

    _ = bytes.slice();
}
```

## License and Trademarks

Licensed under the Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0

Apache OpenDAL, OpenDAL, and Apache are either registered trademarks or trademarks of the Apache Software Foundation.
