# RFC: OpenDAL Zig Native Binding 设计（基于 Zig 0.16.0）

- 状态：Draft
- 作者：Codex
- 日期：2026-04-17
- 目标目录：`bindings/zig`

## 1. 摘要

本文重新设计 OpenDAL 的 Zig binding，目标是让它在 **Zig 0.16.0** 的语义下成为一个真正可发布、可维护、可扩展的 Zig Native SDK。

核心结论如下：

- 最低 Zig 版本提升到 **0.16.0**。
- 废弃 `bindings/zig -> bindings/c -> core` 的分层方式。
- 改为 `bindings/zig/src -> bindings/zig/native -> core` 的 Zig Native 结构。
- 公共 API 同时提供：
  - **同步 API**
  - **原生异步 API**
- 大对象默认采用**零拷贝优先**策略，不再强制每次都复制到 Zig allocator。
- 小对象默认采用**拷贝后归 Zig 管理**策略，避免生命周期陷阱。
- 异步设计采用 **“Zig 驱动编排，Rust 执行 I/O”** 的长期方案：
  - Zig 负责 API 形状、等待、取消、组合、与 `std.Io` 对齐。
  - Rust 负责执行 OpenDAL / Tokio 任务、生成完成事件、维护 task state。

更具体地说，本 RFC 不再把 Zig binding 设计成 C binding 的薄封装，也不再把 async 设计成“把 blocking API 扔进线程池就算结束”。本文给出的是一份**完整 async/event 设计**，覆盖：

- 公共 Zig 类型
- 内存所有权模型
- 零拷贝和复制策略
- 异步 operation 对象
- Zig 与 Rust 的职责边界
- 事件通知与完成队列
- 完整 FFI 面
- 渐进式实施顺序

## 2. 背景与问题

当前仓库中的 Zig binding 仍有如下问题：

- `bindings/zig` 通过 `translate-c` 和 `opendal_c` 依赖 `bindings/c`。
- `bindings/zig/build.zig.zon` 仍声明最低版本为 `0.14.0`。
- 现有 Zig API 基本是 C API 的薄封装，不是 Zig-native 设计。
- 现有内存管理语义继承了 C binding 的释放规则，容易在 Zig 侧暴露 dangling pointer、use-after-free、double-free 风险。
- 现有 async 尝试依赖实验性 coroutine 方案，不符合 Zig 0.16.0 的官方方向。

与此同时，Zig 官方在 0.16.0 之后已经把异步与 I/O 的设计中心转向 `std.Io`：

- 通过 `io.async(...)` 创建 `Future(T)`。
- 通过 `future.await(io)` 与 `future.cancel(io)` 管理任务。
- 通过 `std.Io.Group` 管理并发任务集合。
- 通过 `std.Io.Event`、`std.Io.Mutex`、`std.Io.Condition` 提供异步兼容的同步原语。

但这里要澄清一个非常重要的点：

- **“使用 `std.Io` 语义”是 Zig 官方提供的能力**
- **“OpenDAL 如何与这套能力结合”是本 RFC 的设计选择**

也就是说，`std.Io.Event`、`Future.await(io)`、`Group` 的存在是 Zig 官方方向；而 OpenDAL 选择让 async API 与这套语义对齐，是为了长期可维护性和 Zig 用户体验，而不是语言强制要求。

## 3. 设计目标与非目标

### 3.1 目标

- 提供真正 Zig-native 的公共 API。
- 明确 Zig 与 Rust 之间的所有权边界。
- 避免大对象不必要的复制。
- 让公共 API 能自然接入 Zig 0.16.0 的 `std.Io` 语义。
- 提供完整的同步与异步 API。
- 异步 API 支持等待、取消、轮询、完成通知和组合。
- 为长期支持 `Io.Evented` 留出结构空间。

### 3.2 非目标

- V1 不为每个后端提供单独的 typed builder。
- V1 不直接把 Rust allocator 暴露给 Zig。
- V1 不维护与 `bindings/c` 兼容的 ABI。
- V1 不要求 Zig 直接 poll Rust 原生 future 对象。

## 4. 版本基线与兼容策略

### 4.1 最低 Zig 版本

本 RFC 要求将 `bindings/zig` 的最低 Zig 版本从 `0.14.0` 提升到 `0.16.0`。

原因：

- 本文的异步设计依赖 Zig 0.16.0 的 `std.Io`、Future 语义和取消语义。
- 如果继续维持 `0.14.0`，就必须围绕更旧的实验性 async 心智来设计接口，这会让 API 既不稳定也不 Zig-native。

### 4.2 对 `Io.Threaded` 与 `Io.Evented` 的支持策略

长期设计结论如下：

- **第一阶段正式支持**：`std.Io.Threaded`
- **第二阶段结构兼容**：`std.Io.Evented`

原因：

- Zig 0.16.0 的 `Io.Evented` 生态还未完整覆盖网络场景。
- OpenDAL 的核心负载包含文件 I/O 与网络存储访问。
- 因此，第一阶段应优先把异步 API 语义、任务取消、事件通知与内存模型设计对，再逐步落到 evented reactor 的最佳适配。

这里的关键是：

- **API 现在就按最终 async 形状设计**
- **底层驱动可分阶段落地**

## 5. 总体架构

新的结构如下：

```text
bindings/zig/
  build.zig
  build.zig.zon
  src/
    opendal.zig
    async.zig
    types.zig
  native/
    Cargo.toml
    src/lib.rs
    include/opendal_zig.h
  test/
```

### 5.1 `bindings/zig/native`

这是 Zig 专用 Rust FFI 层，直接依赖 `core`。

职责：

- 调用 `opendal` Rust API。
- 提供 Zig 专用、稳定、最小化的 ABI。
- 管理 Rust 侧句柄、task、完成队列、错误消息。
- 把 OpenDAL 的同步/异步能力投影成 Zig 能可靠消费的抽象。

### 5.2 `bindings/zig/src`

这是用户真正会 `@import("opendal")` 的公共 Zig API 层。

职责：

- 封装所有 FFI 细节。
- 组织 Zig 风格的类型和生命周期。
- 为大对象提供零拷贝持有类型。
- 为小对象提供易用的 Zig-owned value。
- 提供同步 API 与原生异步 operation API。

## 6. 公共 Zig API 设计

## 6.1 类型总览

公共 API 至少包含以下核心类型：

- `Runtime`
- `RuntimeOptions`
- `Operator`
- `Option`
- `KnownScheme`
- `Bytes`
- `Reader`
- `Writer`
- `Lister`
- `AsyncReader`
- `AsyncWriter`
- `AsyncLister`
- `EntryView`
- `Entry`
- `MetadataView`
- `Metadata`
- `OperatorInfo`
- `Capability`
- `HeaderView`
- `PresignedRequest`
- `Header`
- `ReadOptions`
- `WriteOptions`
- `StatOptions`
- `DeleteOptions`
- `CopyOptions`
- `ListOptions`
- `PresignOptions`
- `Poll(T)`
- `VoidOp`
- `ReadOp`
- `WriteOp`
- `StatOp`
- `ExistsOp`
- `DeleteOp`
- `CreateDirOp`
- `RenameOp`
- `CopyOp`
- `InfoOp`
- `PresignOp`
- `ListNextOp`
- `ReaderReadOp`
- `ReaderSeekOp`
- `WriterWriteOp`
- `WriterCloseOp`
- `Error`
- `ErrorInfo`

## 6.2 Runtime

如果用户只使用同步 API，可以不显式创建 runtime。

如果用户要使用原生 async API，则必须显式创建 `Runtime`。

```zig
pub const Runtime = struct {
    pub fn init(allocator: std.mem.Allocator, options: RuntimeOptions) !Runtime
    pub fn allocator(self: *const Runtime) std.mem.Allocator
    pub fn deinit(self: *Runtime) void
};

pub const RuntimeOptions = struct {
    worker_threads: ?u16 = null,
    completion_queue_capacity: usize = 1024,
    cancellation_check_interval_ms: u32 = 10,
};
```

`Runtime` 负责：

- Rust async runtime 的宿主
- task registry
- completion queue
- 平台通知器
- Zig wrapper 的 await / cancel 入口
- copy-backed async result 的分配入口

## 6.3 Operator 构造

同步-only Operator：

```zig
pub fn init(scheme: []const u8, options: []const Option) !Operator
pub fn initKnown(scheme: KnownScheme, options: []const Option) !Operator
```

带 async 能力的 Operator：

```zig
pub fn initWithRuntime(
    runtime: *Runtime,
    scheme: []const u8,
    options: []const Option,
) !Operator

pub fn initKnownWithRuntime(
    runtime: *Runtime,
    scheme: KnownScheme,
    options: []const Option,
) !Operator
```

`Option` 仍采用通用 `scheme + key/value` 构造：

```zig
pub const Option = struct {
    key: []const u8,
    value: []const u8,
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
    // ...
};
```

## 6.4 同步 API

### 设计结论

同步 API 不强制接收 `io: std.Io` 参数，保持 Zig 常见同步调用体验。

```zig
pub fn deinit(self: *Operator) void
pub fn check(self: *Operator) !void
pub fn exists(self: *Operator, path: []const u8) !bool
pub fn createDir(self: *Operator, path: []const u8) !void
pub fn delete(self: *Operator, path: []const u8, options: DeleteOptions) !void
pub fn rename(self: *Operator, from: []const u8, to: []const u8) !void
pub fn copy(self: *Operator, from: []const u8, to: []const u8, options: CopyOptions) !void
pub fn write(self: *Operator, path: []const u8, data: []const u8, options: WriteOptions) !void
pub fn reader(self: *Operator, path: []const u8, options: ReadOptions) !Reader
pub fn writer(self: *Operator, path: []const u8, options: WriteOptions) !Writer
pub fn lister(self: *Operator, path: []const u8, options: ListOptions) !Lister
```

### 读取 API

这里不再只提供“复制到 Zig allocator”的方案，而是拆成两层：

#### 零拷贝主 API

```zig
pub fn readBytes(self: *Operator, path: []const u8, options: ReadOptions) !Bytes
```

#### 方便型复制 API

```zig
pub fn readAlloc(
    self: *Operator,
    allocator: std.mem.Allocator,
    path: []const u8,
    options: ReadOptions,
) ![]u8
```

设计原因：

- 是的，如果所有读取结果都先落到 Rust，再复制到 Zig，会产生额外开销。
- 对于大对象读取，这个额外复制通常是不必要的。
- 因此，`readBytes` 才是主 API，`readAlloc` 只是 convenience helper。

### 结构化查询 API

```zig
pub fn statAlloc(
    self: *Operator,
    allocator: std.mem.Allocator,
    path: []const u8,
    options: StatOptions,
) !Metadata
pub fn infoAlloc(self: *Operator, allocator: std.mem.Allocator) !OperatorInfo
pub fn presignReadAlloc(
    self: *Operator,
    allocator: std.mem.Allocator,
    path: []const u8,
    expire_secs: u64,
    options: PresignOptions,
) !PresignedRequest
pub fn presignWriteAlloc(
    self: *Operator,
    allocator: std.mem.Allocator,
    path: []const u8,
    expire_secs: u64,
    options: PresignOptions,
) !PresignedRequest
pub fn presignStatAlloc(
    self: *Operator,
    allocator: std.mem.Allocator,
    path: []const u8,
    expire_secs: u64,
    options: PresignOptions,
) !PresignedRequest
pub fn presignDeleteAlloc(
    self: *Operator,
    allocator: std.mem.Allocator,
    path: []const u8,
    expire_secs: u64,
    options: PresignOptions,
) !PresignedRequest
```

这里的 `Metadata`、`OperatorInfo`、`PresignedRequest` 采用小对象复制策略。

原因：

- 这些对象体积相对小。
- 字段多为字符串和 header 数组，复制开销远小于用户后续网络 I/O 成本。
- 复制后 Zig 完全掌握生命周期，API 更容易正确使用。
- 同步路径显式接收 `allocator`，避免隐藏分配器。

## 6.5 Bytes

`Bytes` 是大对象的零拷贝持有类型。它本身是一个 Zig-visible、Rust-backed 的拥有型对象。

```zig
pub const Bytes = struct {
    pub fn deinit(self: *Bytes) void
    pub fn len(self: Bytes) usize
    pub fn slice(self: Bytes) []const u8
    pub fn cloneAlloc(self: Bytes, allocator: std.mem.Allocator) ![]u8
};
```

这里的关键点是：

- `Bytes` 不是“借用视图”
- `Bytes` 也不是“必须复制成 Zig slice 才能用”
- 它是 **公开的、显式的、可 deinit 的 Rust-owned buffer handle**

换句话说，`Bytes` 会把“外部所有权”变成一个明确的 Zig 对象，而不是隐藏在 API 背后。

这就是本 RFC 对评论中“这不会导致额外的数据复制吗？”的回答：

- **如果用户选择 `readAlloc`，会复制**
- **如果用户选择 `readBytes`，不会复制**

## 6.6 Reader / Writer / Lister

### Reader

```zig
pub const Reader = struct {
    pub fn deinit(self: *Reader) void
    pub fn read(self: *Reader, buf: []u8) !usize
    pub fn seekTo(self: *Reader, pos: u64) !u64
    pub fn seekBy(self: *Reader, delta: i64) !u64
    pub fn seekFromEnd(self: *Reader, delta: i64) !u64
};
```

### Writer

```zig
pub const Writer = struct {
    pub fn deinit(self: *Writer) void
    pub fn write(self: *Writer, data: []const u8) !usize
    pub fn writeAll(self: *Writer, data: []const u8) !void
    pub fn close(self: *Writer) !void
};
```

### Lister

为减少不必要复制，`Lister` 提供两套接口：

#### 借用型热路径

```zig
pub fn next(self: *Lister) !?EntryView
```

`EntryView` 有效期：

- 直到下一次 `next()`
- 或 `Lister.deinit()`

#### 拥有型方便接口

```zig
pub fn nextAlloc(self: *Lister, allocator: std.mem.Allocator) !?Entry
```

类型定义：

```zig
pub const EntryView = struct {
    pub fn path(self: EntryView) []const u8
    pub fn name(self: EntryView) []const u8
    pub fn metadata(self: EntryView) MetadataView
    pub fn cloneAlloc(self: EntryView, allocator: std.mem.Allocator) !Entry
};

pub const Entry = struct {
    path: []u8,
    metadata: Metadata,

    pub fn deinit(self: *Entry, allocator: std.mem.Allocator) void
    pub fn name(self: Entry) []const u8
};
```

这样设计的原因同样是回应“复制是否必要”：

- **列表热路径默认不复制**
- **用户需要长期持有时再复制**

## 6.7 Capability / Metadata / OperatorInfo / PresignedRequest

### HeaderView

```zig
pub const HeaderView = struct {
    key: []const u8,
    value: []const u8,
};
```

### Capability

`Capability` 需要和 OpenDAL core 的 capability matrix 保持一一对应，而不是只暴露一个模糊的 `supports_xxx` 集合。

```zig
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
};
```

这里的设计要求是：

- Zig 侧字段名尽量与 core 对齐
- 新 capability 字段新增时采用向后兼容追加
- `OperatorInfo.full_capability` 与 `native_capability` 都使用同一类型

### MetadataView

`MetadataView` 是 `Lister.next()` 热路径里的借用型元数据视图。

```zig
pub const MetadataView = struct {
    pub fn mode(self: MetadataView) EntryMode
    pub fn isFile(self: MetadataView) bool
    pub fn isDir(self: MetadataView) bool
    pub fn isDeleted(self: MetadataView) bool
    pub fn contentLength(self: MetadataView) ?u64
    pub fn contentType(self: MetadataView) ?[]const u8
    pub fn contentEncoding(self: MetadataView) ?[]const u8
    pub fn contentDisposition(self: MetadataView) ?[]const u8
    pub fn contentMd5(self: MetadataView) ?[]const u8
    pub fn etag(self: MetadataView) ?[]const u8
    pub fn lastModifiedRfc3339(self: MetadataView) ?[]const u8
    pub fn version(self: MetadataView) ?[]const u8
    pub fn userMetadata(self: MetadataView) []const HeaderView
    pub fn cloneAlloc(self: MetadataView, allocator: std.mem.Allocator) !Metadata
};
```

这样可以避免 `EntryView.metadata()` 在没有 allocator 参数的前提下偷偷分配内存，也让“列表热路径默认不复制”真正成立。

### Metadata

```zig
pub const EntryMode = enum {
    file,
    dir,
    unknown,
};

pub const Header = struct {
    key: []u8,
    value: []u8,
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

    pub fn deinit(self: *Metadata, allocator: std.mem.Allocator) void
};
```

### OperatorInfo

```zig
pub const OperatorInfo = struct {
    scheme: []u8,
    root: []u8,
    name: []u8,
    full_capability: Capability,
    native_capability: Capability,

    pub fn deinit(self: *OperatorInfo, allocator: std.mem.Allocator) void
};
```

### PresignedRequest

```zig
pub const PresignedRequest = struct {
    method: []u8,
    url: []u8,
    headers: []Header,

    pub fn deinit(self: *PresignedRequest, allocator: std.mem.Allocator) void
};
```

## 6.8 原生异步 API

### 总体结论

这一版 RFC **明确设计原生 async API**，而不是只依赖 `io.async(...)` 把同步方法扔到线程里。

也就是说，之前“V1 不自定义 future”的说法在本 RFC 中被替换为：

- OpenDAL 提供 **typed async operation object**
- 这些对象的方法语义尽量贴近 Zig 0.16.0 的等待/取消模型
- `std.Io` 负责等待与组合
- OpenDAL 负责 task identity、结果提取和取消映射

### 异步 Operator 方法

```zig
pub fn checkAsync(self: *Operator) !VoidOp
pub fn existsAsync(self: *Operator, path: []const u8) !ExistsOp
pub fn createDirAsync(self: *Operator, path: []const u8) !CreateDirOp
pub fn deleteAsync(self: *Operator, path: []const u8, options: DeleteOptions) !DeleteOp
pub fn renameAsync(self: *Operator, from: []const u8, to: []const u8) !RenameOp
pub fn copyAsync(self: *Operator, from: []const u8, to: []const u8, options: CopyOptions) !CopyOp
pub fn writeAsync(self: *Operator, path: []const u8, data: []const u8, options: WriteOptions) !WriteOp
pub fn readAsync(self: *Operator, path: []const u8, options: ReadOptions) !ReadOp
pub fn statAsync(self: *Operator, path: []const u8, options: StatOptions) !StatOp
pub fn infoAsync(self: *Operator) !InfoOp
pub fn presignReadAsync(self: *Operator, path: []const u8, expire_secs: u64, options: PresignOptions) !PresignOp
pub fn presignWriteAsync(self: *Operator, path: []const u8, expire_secs: u64, options: PresignOptions) !PresignOp
pub fn presignStatAsync(self: *Operator, path: []const u8, expire_secs: u64, options: PresignOptions) !PresignOp
pub fn presignDeleteAsync(self: *Operator, path: []const u8, expire_secs: u64, options: PresignOptions) !PresignOp
```

### 异步 Reader / Writer / Lister

```zig
pub fn readerAsync(self: *Operator, path: []const u8, options: ReadOptions) !AsyncReader
pub fn writerAsync(self: *Operator, path: []const u8, options: WriteOptions) !AsyncWriter
pub fn listerAsync(self: *Operator, path: []const u8, options: ListOptions) !AsyncLister
```

```zig
pub const AsyncReader = struct {
    pub fn deinit(self: *AsyncReader) void
    pub fn readAsync(self: *AsyncReader, buf: []u8) !ReaderReadOp
    pub fn seekToAsync(self: *AsyncReader, pos: u64) !ReaderSeekOp
    pub fn seekByAsync(self: *AsyncReader, delta: i64) !ReaderSeekOp
    pub fn seekFromEndAsync(self: *AsyncReader, delta: i64) !ReaderSeekOp
};

pub const AsyncWriter = struct {
    pub fn deinit(self: *AsyncWriter) void
    pub fn writeAsync(self: *AsyncWriter, data: []const u8) !WriterWriteOp
    pub fn closeAsync(self: *AsyncWriter) !WriterCloseOp
};

pub const AsyncLister = struct {
    pub fn deinit(self: *AsyncLister) void
    pub fn nextAsync(self: *AsyncLister) !ListNextOp
};
```

## 6.9 Async operation object 语义

每个 operation object 都是单次使用对象，至少提供以下方法：

```zig
pub fn poll(self: *Op) !Poll(T)
pub fn await(self: *Op, io: std.Io) !T
pub fn cancel(self: *Op, io: std.Io) !?T
pub fn deinit(self: *Op) void
```

其中：

```zig
pub fn Poll(comptime T: type) type {
    return union(enum) {
        pending,
        ready: T,
    };
}
```

语义如下：

- `poll()`：非阻塞查询是否完成
  - 如果返回 `ready: T`，则视为取走结果，operation 进入 `consumed`
- `await(io)`：等待完成并取出结果
- `cancel(io)`：
  - 如果成功在完成前取消，返回 `null`
  - 如果任务已经完成但结果尚未取走，返回 `T`
- `deinit()`：
  - 只释放 operation 本身的本地句柄
  - 若 operation 仍在 pending，必须先调用 `cancel(io)` 或 `await(io)`

之所以把 `cancel(io)` 设计成 `!?T`，就是为了解决你评论里指出的那个现实问题：

- 如果取消时任务已经完成，结果不能丢，也不能泄漏
- 对于 `Bytes` 这类拥有型结果，调用者必须拿到它并决定是否释放

### ReadOp 示例

```zig
pub const ReadOp = struct {
    pub fn poll(self: *ReadOp) !Poll(Bytes)
    pub fn await(self: *ReadOp, io: std.Io) !Bytes
    pub fn cancel(self: *ReadOp, io: std.Io) !?Bytes
    pub fn deinit(self: *ReadOp) void

    pub fn awaitAlloc(
        self: *ReadOp,
        io: std.Io,
        allocator: std.mem.Allocator,
    ) ![]u8
};
```

`awaitAlloc` 是 convenience API，会复制；`await(io)` 返回 `Bytes` 则不会复制。

对于 `StatOp`、`InfoOp`、`PresignOp`、`ListNextOp` 这类返回 Zig-owned value 的 operation：

- `poll()` 在返回 `ready` 时，使用 `Runtime.init(...)` 传入的 allocator 完成 materialize
- `await(io)` / `cancel(io)` 使用 `Runtime.init(...)` 传入的 allocator 完成 materialize
- 调用者可通过 `runtime.allocator()` 取回同一个 allocator 来执行 `deinit` / `free`

## 7. 内存管理模型

## 7.1 总原则

旧稿中的“Rust 持有句柄，Zig 持有公共数据”太粗，会让人误以为**所有结果都必须复制**。这并不准确。

新版规则改为：

- **Zig 持有公共对象的生命周期责任**
- **底层内存既可以由 Zig allocator 持有，也可以由显式 Rust-owned handle 持有**
- **禁止隐藏的跨语言释放责任**
- **允许显式零拷贝对象**

## 7.2 三类内存对象

### A. 借用输入

- `scheme`
- `path`
- `[]const u8`
- `[]const Option`
- 操作选项里的字符串切片

规则：

- Rust 只能在当前调用期间读取
- 需要跨调用保存时，Rust 必须复制

### B. Rust-owned handle

- `Operator`
- `Reader`
- `Writer`
- `Lister`
- `AsyncReader`
- `AsyncWriter`
- `AsyncLister`
- 各类 async operation object
- `Bytes`

规则：

- Zig 通过 `deinit()` 管理生命周期
- 释放动作由 Rust FFI 完成
- Zig 用户不直接调用 Rust free 函数

### C. Zig-owned value

- `Metadata`
- `OperatorInfo`
- `PresignedRequest`
- `Entry`
- `Header[]`
- `[]u8`（来自 `cloneAlloc` / `readAlloc`）

规则：

- 一旦返回给用户，就完全由 Zig allocator 管理

### D. Borrowed view

- `EntryView`
- `MetadataView`
- `HeaderView`

规则：

- 生命周期严格受宿主对象约束
- 不能跨 `next()`、`deinit()` 或结果消费边界保存
- 如需长期持有，必须显式 `cloneAlloc`

## 7.3 哪些地方会复制，哪些地方不会

### 默认不复制

- `readBytes`
- `readAsync().await(io)`
- `Lister.next() -> EntryView`
- `EntryView.metadata() -> MetadataView`

### 默认复制

- `statAlloc`
- `infoAlloc`
- `presign*Alloc`
- `nextAlloc`
- `readAlloc`
- `Bytes.cloneAlloc`

### 设计理由

这是对你评论里两次指出“这不会导致额外的数据复制吗？”的完整回答：

- **大对象：默认零拷贝**
- **小对象：默认复制**
- **热路径：优先 view**
- **长期持有：显式 clone**

也就是说，复制不是默认教条，而是按对象大小和生命周期要求分别处理。

## 7.4 为什么 `Metadata` / `OperatorInfo` / `PresignedRequest` 仍然复制

原因不是 Zig 强制，也不是 OpenDAL 强制，而是一个 API 设计判断：

- 它们体积小
- 字段语义稳定
- 用户常常希望跨调用长期持有
- 把它们做成借用视图会显著提高误用概率

所以这里选择“复制到 Zig-owned value”是出于可用性和正确性，而不是语言限制。

唯一的例外是 `Lister.next()` 热路径中的 `MetadataView`：

- 它是明确的借用视图
- 生命周期和 `EntryView` 绑定
- 不承担长期持有语义

## 7.5 错误与诊断

### 公共 Zig 层

```zig
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
```

### 诊断对象

额外提供可选诊断对象，而不是让所有 API 都强行暴露原始 message：

```zig
pub const ErrorInfo = struct {
    code: ErrorCode,
    message: []u8,

    pub fn deinit(self: *ErrorInfo, allocator: std.mem.Allocator) void
};
```

并提供：

```zig
pub fn lastErrorInfo(self: *Runtime, allocator: std.mem.Allocator) !?ErrorInfo
```

## 8. 异步事件设计

## 8.1 角色划分：这是设计选择，不是 Zig 硬性要求

旧稿里“OpenDAL 负责普通方法，Zig 标准库负责 Future / Event / Mutex / Condition”的说法不够严谨。

更准确地说：

- Zig 提供了官方 async/event 原语
- OpenDAL 可以选择：
  - 完全隐藏自己的 runtime
  - 或把 async API 设计成能与 Zig 官方原语协作

本 RFC 选择后者。

所以这里的角色划分是：

- **Zig 官方提供能力**
- **OpenDAL 选择与这些能力对齐**

而不是“Zig 要求 OpenDAL 必须这样做”。

## 8.2 为什么长期必须让 Zig 驱动异步编排

这是对你评论“这个设计好吗？为什么不要求 zig 驱动？”的直接回答。

如果完全由 Rust 驱动 async，而 Zig 只看到一个黑盒对象，问题会非常明显：

- Zig 用户很难把 OpenDAL 任务和自己的 `std.Io` 任务统一等待
- Zig 用户很难统一做取消、deadline、structured concurrency
- `std.Io.Group`、`Event`、`Condition` 等原语无法自然协作
- 调试和测试时，OpenDAL 会像一个自带独立事件系统的 foreign runtime

因此，长期正确的方向不是“Zig 去 poll Rust future 对象”，而是：

- **Zig 驱动异步编排与等待**
- **Rust 执行真实 I/O**

这个表述很重要，因为它区分了两件事：

- **驱动编排**：谁来决定等待、取消、组合、超时和用户可见语义
- **驱动底层 future 执行**：谁来真正跑 OpenDAL / Tokio 的内部 future

本 RFC 的结论是：

- 编排由 Zig 驱动
- 执行由 Rust 驱动

## 8.3 为什么不让 Zig 直接 poll Rust future

因为这条路从 ABI、生命周期和执行模型上都很差：

- Rust future 没有稳定 ABI
- Rust future 依赖 pinning 和 waker 语义
- Zig 无法直接安全理解 Rust executor 内部状态
- 取消、drop、panic、source error、waker 生命周期都很难稳定跨 FFI

因此，本 RFC 不设计“Zig 直接 poll Rust future 对象”，而是设计：

- Rust 暴露 **稳定 task handle**
- Rust 提供 **完成队列和通知器**
- Zig 根据这些 stable ABI 做 `await(io)` 和 `cancel(io)`

## 8.4 异步总架构

完整 async 架构如下：

```text
Zig ReadOp.await(io)
  -> Zig Runtime.wait(task_id, io)
      -> wait on notifier through std.Io
      -> drain completion queue
      -> mark task ready in Zig-side task table
  -> ffi.task_take_bytes(task)
  -> return Bytes

Rust Runtime
  -> spawn opendal async future
  -> when future completes:
      1. store result in task slot
      2. push task_id into completion queue
      3. signal notifier
```

### 关键对象

- `Runtime`
- `TaskHandle`
- `CompletionQueue`
- `Notifier`
- `AsyncOp(T)`

## 8.5 Runtime 内部组件

`Runtime` 至少包含：

- Rust async executor
- task table
- completion queue
- platform notifier
- cancellation registry
- Zig 侧 ready map / task state cache

### Task 状态机

每个 task 必须处于以下状态之一：

- `pending`
- `ready`
- `consumed`
- `canceled`
- `failed`

状态转换：

- `pending -> ready`
- `pending -> canceled`
- `ready -> consumed`
- `pending -> failed`
- `failed -> consumed`

## 8.6 通知器设计

为了让 Zig 的 `await(io)` 真正接入 Zig 官方异步等待语义，Rust runtime 需要向 Zig 暴露一个**可等待的通知器**。

建议实现：

- Linux / Android：`eventfd`
- macOS / BSD：nonblocking pipe
- Windows：manual-reset event handle

公共抽象：

```zig
pub const NotifierKind = enum {
    eventfd,
    pipe_read_fd,
    win_handle,
};
```

Rust FFI 提供：

- notifier 类型
- 对应 fd / handle
- drain completions

Zig wrapper 用它接入 `std.Io` 的等待能力。

## 8.7 Async operation object 结构

所有 async operation object 共享一套通用内部结构：

```zig
const OpState = enum {
    pending,
    ready,
    consumed,
    canceled,
    failed,
};

pub fn AsyncOp(comptime T: type) type {
    return struct {
        runtime: *Runtime,
        task: *opaque {},
        state: OpState,

        pub fn poll(self: *@This()) !Poll(T)
        pub fn await(self: *@This(), io: std.Io) !T
        pub fn cancel(self: *@This(), io: std.Io) !?T
        pub fn deinit(self: *@This()) void
    };
}
```

具体操作通过 type alias 或小 wrapper 暴露：

```zig
pub const VoidOp = AsyncOp(void);
pub const CreateDirOp = AsyncOp(void);
pub const DeleteOp = AsyncOp(void);
pub const RenameOp = AsyncOp(void);
pub const CopyOp = AsyncOp(void);
pub const WriteOp = AsyncOp(void);
pub const ReadOp = AsyncOp(Bytes);
pub const StatOp = AsyncOp(Metadata);
pub const InfoOp = AsyncOp(OperatorInfo);
pub const ExistsOp = AsyncOp(bool);
pub const PresignOp = AsyncOp(PresignedRequest);
pub const ListNextOp = AsyncOp(?Entry);
pub const ReaderReadOp = AsyncOp(usize);
pub const ReaderSeekOp = AsyncOp(u64);
pub const WriterWriteOp = AsyncOp(usize);
pub const WriterCloseOp = AsyncOp(void);
```

## 8.8 为什么仍然需要 `std.Io.Event` / `Mutex` / `Condition`

这是对你评论“这个角色是 zig 要求的还是 opendal 要求的”的补充。

答案是：

- **不是 Zig 强制要求**
- **是 OpenDAL 为了和 Zig 官方 async 语义稳定对齐而主动选用**

这些原语在本设计中的角色是：

- Zig wrapper 内部同步
- 多等待者协调
- 测试中的 race-free 协调
- 在未来接入 `Io.Evented` 时，避免继续依赖 `std.Thread.*`

它们不是核心 public API，但应该作为内部首选实现原语。

## 8.9 为什么这个设计比“纯 Rust 驱动黑盒 async”更好

综合比较如下：

### 纯 Rust 驱动黑盒 async

优点：

- 实现可能更快起步

缺点：

- Zig 用户无法把 OpenDAL async 和自己程序的 async 统一建模
- 取消和 deadline 语义不透明
- 难以和 `std.Io` 组合
- 长期像“嵌入了另一个 runtime”

### Zig 驱动编排 + Rust 执行 I/O

优点：

- 公共语义统一
- `await(io)` / `cancel(io)` / group-friendly
- 更容易测试
- 更容易渐进支持 `Io.Evented`

缺点：

- 实现更复杂
- 需要完成队列和通知器设计

本 RFC 选择第二条路线，因为它是长期正确方案。

## 9. Rust Native FFI 设计

## 9.1 放置位置

Rust FFI 层放在：

```text
bindings/zig/native/
```

而不是：

- `core/`
- `bindings/c/`

原因：

- 这是 Zig 专用 ABI，不应污染 core。
- 它不再是通用 C binding 的一部分。
- Zig 的内存和 async 语义不应该反向绑架通用 C API。

## 9.2 输入 ABI

FFI 使用显式长度的 slice-like ABI，不使用 nul-terminated C string。

```c
typedef struct {
  const uint8_t *ptr;
  uintptr_t len;
} od_zig_slice;

typedef struct {
  od_zig_slice key;
  od_zig_slice value;
} od_zig_option;
```

理由：

- 避免误把 Zig slice 当作 C string
- 消除 NUL 结尾约束
- 保持 ABI 对 Zig 自然

## 9.3 输出 ABI 分类

FFI 输出分四类：

### A. opaque handle

- operator
- reader
- writer
- lister
- bytes
- runtime
- task

### B. Rust-owned temporary allocated value

- metadata
- info
- presigned request
- entry owned
- error info

### C. borrowed view

- entry view
- metadata view
- header view
- borrowed path/name slice

为了避免 borrowed object 也变成一堆 getter-handle，本 RFC 要求这些 view 尽量以 **FFI-safe snapshot struct** 的形式一次性返回给 Zig wrapper，再由 Zig 包装成 `EntryView` / `MetadataView`。

### D. POD result struct

- bool / usize / u64 / error code / poll state

## 9.4 完整同步 FFI 面

这是对你评论“请考虑长远，把所有需要的都列一下”的直接响应。

同步 FFI 完整列举如下：

### Runtime

- `od_zig_runtime_new`
- `od_zig_runtime_free`
- `od_zig_runtime_notifier_kind`
- `od_zig_runtime_notifier_fd`
- `od_zig_runtime_notifier_handle`
- `od_zig_runtime_drain`
- `od_zig_runtime_last_error_info`

### Operator

- `od_zig_operator_new`
- `od_zig_operator_new_with_runtime`
- `od_zig_operator_free`
- `od_zig_operator_check`
- `od_zig_operator_exists`
- `od_zig_operator_create_dir`
- `od_zig_operator_delete`
- `od_zig_operator_rename`
- `od_zig_operator_copy`
- `od_zig_operator_write`
- `od_zig_operator_read_bytes`
- `od_zig_operator_stat`
- `od_zig_operator_info`
- `od_zig_operator_reader_open`
- `od_zig_operator_writer_open`
- `od_zig_operator_lister_open`
- `od_zig_operator_presign_read`
- `od_zig_operator_presign_write`
- `od_zig_operator_presign_stat`
- `od_zig_operator_presign_delete`

### Reader

- `od_zig_reader_read`
- `od_zig_reader_seek_to`
- `od_zig_reader_seek_by`
- `od_zig_reader_seek_from_end`
- `od_zig_reader_free`

### Writer

- `od_zig_writer_write`
- `od_zig_writer_close`
- `od_zig_writer_free`

### Lister

- `od_zig_lister_next_view`
- `od_zig_lister_next_owned`
- `od_zig_lister_free`

### Bytes

- `od_zig_bytes_len`
- `od_zig_bytes_ptr`
- `od_zig_bytes_free`

### Allocated values

- `od_zig_metadata_free`
- `od_zig_info_free`
- `od_zig_presigned_request_free`
- `od_zig_entry_owned_free`
- `od_zig_error_info_free`

## 9.5 完整异步 FFI 面

异步 FFI 必须完整列出，而不是只写一个“以后再加 task ABI”。

### Async task start

- `od_zig_operator_check_start`
- `od_zig_operator_exists_start`
- `od_zig_operator_create_dir_start`
- `od_zig_operator_delete_start`
- `od_zig_operator_rename_start`
- `od_zig_operator_copy_start`
- `od_zig_operator_write_start`
- `od_zig_operator_read_start`
- `od_zig_operator_stat_start`
- `od_zig_operator_info_start`
- `od_zig_operator_presign_read_start`
- `od_zig_operator_presign_write_start`
- `od_zig_operator_presign_stat_start`
- `od_zig_operator_presign_delete_start`
- `od_zig_operator_reader_open_start`
- `od_zig_operator_writer_open_start`
- `od_zig_operator_lister_open_start`

### Async reader / writer / lister task start

- `od_zig_reader_read_start`
- `od_zig_reader_seek_to_start`
- `od_zig_reader_seek_by_start`
- `od_zig_reader_seek_from_end_start`
- `od_zig_writer_write_start`
- `od_zig_writer_close_start`
- `od_zig_lister_next_owned_start`

### Task control

- `od_zig_task_poll`
- `od_zig_task_cancel`
- `od_zig_task_state`
- `od_zig_task_error_code`
- `od_zig_task_take_bool`
- `od_zig_task_take_unit`
- `od_zig_task_take_usize`
- `od_zig_task_take_u64`
- `od_zig_task_take_bytes`
- `od_zig_task_take_metadata`
- `od_zig_task_take_info`
- `od_zig_task_take_presigned_request`
- `od_zig_task_take_entry`
- `od_zig_task_take_reader`
- `od_zig_task_take_writer`
- `od_zig_task_take_lister`
- `od_zig_task_free`

### Completion queue

- `od_zig_runtime_drain_ready_tasks`
- `od_zig_runtime_pop_ready_task`
- `od_zig_runtime_ack_ready_task`

## 9.6 为什么 FFI 层不能只提供“普通同步调用”

这是对你评论“这个版本我就需要你设计一下，全面设计”的直接回应。

结论是：

- 只提供同步调用是不够的
- 如果 async 是正式目标，FFI 层必须在第一版设计里完整考虑 runtime、task、completion、cancel 和 notifier

所以本 RFC 把这些一次性列全，即使实现顺序可以分阶段。

## 10. 操作选项类型

### ReadOptions

```zig
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
};
```

### WriteOptions

```zig
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
};
```

### StatOptions

```zig
pub const StatOptions = struct {
    if_match: ?[]const u8 = null,
    if_none_match: ?[]const u8 = null,
    if_modified_since_http_date: ?[]const u8 = null,
    if_unmodified_since_http_date: ?[]const u8 = null,
    override_content_type: ?[]const u8 = null,
    override_cache_control: ?[]const u8 = null,
    override_content_disposition: ?[]const u8 = null,
    version: ?[]const u8 = null,
};
```

### DeleteOptions

```zig
pub const DeleteOptions = struct {
    version: ?[]const u8 = null,
    recursive: bool = false,
    max_size: ?u64 = null,
};
```

### CopyOptions

```zig
pub const CopyOptions = struct {
    if_not_exists: bool = false,
};
```

### ListOptions

```zig
pub const ListOptions = struct {
    recursive: bool = false,
    limit: ?usize = null,
    start_after: ?[]const u8 = null,
    versions: bool = false,
    deleted: bool = false,
};
```

### PresignOptions

```zig
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
};
```

### 设计约束

- 所有 option struct 都是 typed struct，不退回无类型 map。
- option field 名字尽量与 OpenDAL core capability 和 option 语义对齐。
- `check`、`info`、`createDir`、`rename` 在本版本不引入额外 options。

## 11. 示例

## 11.1 同步零拷贝读取

```zig
var op = try opendal.Operator.init("memory", &.{});
defer op.deinit();

try op.write("hello.txt", "world", .{});

var bytes = try op.readBytes("hello.txt", .{});
defer bytes.deinit();

std.debug.print("{s}\n", .{bytes.slice()});
```

## 11.2 同步复制读取

```zig
const data = try op.readAlloc(allocator, "hello.txt", .{});
defer allocator.free(data);
```

## 11.3 原生异步读取

```zig
var threaded: std.Io.Threaded = .{};
defer threaded.deinit();

const io = threaded.io();

var rt = try opendal.Runtime.init(allocator, .{});
defer rt.deinit();

var op = try opendal.Operator.initWithRuntime(&rt, "memory", &.{});
defer op.deinit();

try op.write("a.txt", "hello", .{});

var read_op = try op.readAsync("a.txt", .{});
defer read_op.deinit();

var bytes = try read_op.await(io);
defer bytes.deinit();
```

## 11.4 取消

```zig
var read_op = try op.readAsync("big.bin", .{});
defer read_op.deinit();

if (try read_op.cancel(io)) |bytes| {
    defer bytes.deinit();
    // task 已经完成但结果还没被消费
} else {
    // 成功在完成前取消
}
```

## 11.5 列表热路径零拷贝

```zig
var lister = try op.lister("prefix/", .{});
defer lister.deinit();

while (try lister.next()) |entry_view| {
    const meta = entry_view.metadata();
    std.debug.print("{s} file={any}\n", .{ entry_view.path(), meta.isFile() });
}
```

## 12. 测试与验收标准

### 12.1 同步测试

- `Operator.init` / `initKnown`
- `check`
- `exists`
- `createDir`
- `delete`
- `rename`
- `copy`
- `write` + `readBytes`
- `write` + `readAlloc`
- `reader`
- `writer`
- `lister`
- `Bytes.slice` / `Bytes.cloneAlloc` / `Bytes.deinit`
- `statAlloc`
- `infoAlloc`
- `presign*Alloc`
- `reader.read` / `reader.seek*`
- `writer.writeAll` / `writer.close`
- `lister.next`
- `lister.nextAlloc`
- `EntryView.metadata().cloneAlloc`

### 12.2 异步测试

- `checkAsync.await`
- `existsAsync.await`
- `writeAsync.await`
- `readAsync.await`
- `readAsync.cancel`
- `statAsync.await`
- `infoAsync.await`
- `readerAsync.readAsync.await`
- `writerAsync.writeAsync.await`
- `listerAsync.nextAsync.await`
- `poll -> pending -> ready` 状态转换
- `poll -> ready` 后进入 consumed
- `cancel` 时返回已完成结果的路径

### 12.3 内存测试

- 大对象读取零拷贝路径不发生额外 clone
- `readAlloc` 路径确实复制且释放正确
- `EntryView` 生命周期只到下一次 `next()`
- `MetadataView` 生命周期与 `EntryView` 一致
- copy-backed async result 使用 `Runtime` allocator materialize 且可正确释放
- `Metadata` / `PresignedRequest` / `OperatorInfo` 的 owned 内存可独立释放
- `Bytes.deinit`、task `deinit`、handle `deinit` 全部无泄漏

### 12.4 验收标准

- 用户不需要调用任何 Rust free API。
- 大对象可以零拷贝读取。
- 小对象默认安全复制。
- async API 支持 `poll`、`await(io)`、`cancel(io)`。
- async 任务通过完成队列与通知器接入 Zig async 语义。
- 同步和异步 API 都是正式设计的一部分，而不是一个临时过渡层。

## 13. 实施顺序

推荐按以下顺序落地：

1. 新建 `bindings/zig/native` Rust crate。
2. 去掉对 `bindings/c` 的依赖。
3. 实现同步 handle 与零拷贝 `Bytes`。
4. 实现同步 `Reader` / `Writer` / `Lister`。
5. 实现小对象 owned copy 路径。
6. 实现 `Runtime`、task、completion queue、notifier。
7. 实现 `ReadOp` / `StatOp` / `ExistsOp` 等基础 async op。
8. 实现 async `Reader` / `Writer` / `Lister`。
9. 升级最低 Zig 版本到 `0.16.0`。
10. 删除旧的 coroutine 试验代码与文档。

## 14. 结论

本 RFC 的最终结论是：

- OpenDAL Zig binding 应以 Zig 0.16.0 为基线重建。
- 公共 API 应同时提供同步与原生异步两层能力。
- 大对象默认零拷贝，小对象默认复制。
- “Zig 驱动编排，Rust 执行 I/O” 是长期正确的 async 结构。
- `std.Io.Event` / `Mutex` / `Condition` 的引入是 OpenDAL 主动对齐 Zig 官方 async 语义的设计选择，不是语言硬性要求。
- FFI 第一版设计必须一次性把 runtime / task / completion / cancel / notifier 列全，而不是只留一句“以后再加”。

这版设计比旧稿更完整，也更直接回应了几个关键问题：

- 复制是不是必要：不是，应该分对象类别处理。
- 角色划分是谁要求的：是 OpenDAL 的设计选择，不是 Zig 强制。
- 为什么不让 Zig 直接 poll Rust future：因为 ABI 和执行模型不合适。
- 为什么必须让 Zig 驱动编排：因为用户最终要在 Zig 世界里等待、取消、组合和测试这些任务。

## 15. 参考资料

- Zig 官方下载页：[https://ziglang.org/download/](https://ziglang.org/download/)
- Zig 0.16.0 Release Notes：[https://ziglang.org/download/0.16.0/release-notes.html](https://ziglang.org/download/0.16.0/release-notes.html)
- 当前仓库 Zig 版本声明：[bindings/zig/build.zig.zon](../bindings/zig/build.zig.zon)
