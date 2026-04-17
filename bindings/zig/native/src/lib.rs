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

#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use std::collections::HashMap;
use std::collections::VecDeque;
use std::io::SeekFrom;
use std::os::fd::AsRawFd;
use std::os::fd::FromRawFd;
use std::os::fd::OwnedFd;
use std::ptr;
use std::panic::AssertUnwindSafe;
use std::slice;
use std::str;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;

use futures::AsyncReadExt;
use futures::AsyncSeekExt;
use futures::FutureExt;
use futures::TryStreamExt;
use opendal::Capability;
use opendal::Entry;
use opendal::EntryMode;
use opendal::Error;
use opendal::ErrorKind;
use opendal::FuturesAsyncReader;
use opendal::Lister;
use opendal::Metadata;
use opendal::Operator;
use opendal::OperatorInfo;
use opendal::Reader;
use opendal::Writer;
use opendal::options;
use opendal::raw::PresignedRequest;
use opendal::raw::Timestamp;
use tokio::runtime::Builder as TokioRuntimeBuilder;
use tokio::runtime::Handle as TokioHandle;
use tokio::runtime::Runtime as TokioRuntime;
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinHandle;

type TaskOutcome = Result<TaskValue, StoredError>;

#[repr(C)]
#[derive(Clone, Copy)]
pub enum od_zig_error_code {
    OD_ZIG_OK = 0,
    OD_ZIG_UNEXPECTED = 1,
    OD_ZIG_UNSUPPORTED = 2,
    OD_ZIG_CONFIG_INVALID = 3,
    OD_ZIG_NOT_FOUND = 4,
    OD_ZIG_PERMISSION_DENIED = 5,
    OD_ZIG_IS_DIRECTORY = 6,
    OD_ZIG_NOT_A_DIRECTORY = 7,
    OD_ZIG_ALREADY_EXISTS = 8,
    OD_ZIG_RATE_LIMITED = 9,
    OD_ZIG_IS_SAME_FILE = 10,
    OD_ZIG_CONDITION_NOT_MATCH = 11,
    OD_ZIG_RANGE_NOT_SATISFIED = 12,
    OD_ZIG_INVALID_ARGUMENT = 13,
    OD_ZIG_ASYNC_RUNTIME_REQUIRED = 14,
    OD_ZIG_CANCELED = 15,
    OD_ZIG_OUT_OF_MEMORY = 16,
    OD_ZIG_INTERNAL = 17,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum od_zig_notifier_kind {
    OD_ZIG_NOTIFIER_NONE = 0,
    OD_ZIG_NOTIFIER_EVENTFD = 1,
    OD_ZIG_NOTIFIER_PIPE_READ_FD = 2,
    OD_ZIG_NOTIFIER_WIN_HANDLE = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum od_zig_task_state {
    OD_ZIG_TASK_PENDING = 0,
    OD_ZIG_TASK_READY = 1,
    OD_ZIG_TASK_CONSUMED = 2,
    OD_ZIG_TASK_CANCELED = 3,
    OD_ZIG_TASK_FAILED = 4,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum od_zig_poll_state {
    OD_ZIG_POLL_PENDING = 0,
    OD_ZIG_POLL_READY = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum od_zig_entry_mode {
    OD_ZIG_ENTRY_MODE_FILE = 0,
    OD_ZIG_ENTRY_MODE_DIR = 1,
    OD_ZIG_ENTRY_MODE_UNKNOWN = 2,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct od_zig_slice {
    pub ptr: *const u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct od_zig_mut_slice {
    pub ptr: *mut u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct od_zig_option {
    pub key: od_zig_slice,
    pub value: od_zig_slice,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct od_zig_runtime_options {
    pub has_worker_threads: bool,
    pub worker_threads: u16,
    pub completion_queue_capacity: usize,
    pub cancellation_check_interval_ms: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct od_zig_header {
    pub key: od_zig_slice,
    pub value: od_zig_slice,
}

pub type od_zig_header_view = od_zig_header;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct od_zig_capability {
    pub stat: bool,
    pub stat_with_if_match: bool,
    pub stat_with_if_none_match: bool,
    pub stat_with_if_modified_since: bool,
    pub stat_with_if_unmodified_since: bool,
    pub stat_with_override_cache_control: bool,
    pub stat_with_override_content_disposition: bool,
    pub stat_with_override_content_type: bool,
    pub stat_with_version: bool,
    pub read: bool,
    pub read_with_if_match: bool,
    pub read_with_if_none_match: bool,
    pub read_with_if_modified_since: bool,
    pub read_with_if_unmodified_since: bool,
    pub read_with_override_cache_control: bool,
    pub read_with_override_content_disposition: bool,
    pub read_with_override_content_type: bool,
    pub read_with_version: bool,
    pub write: bool,
    pub write_can_multi: bool,
    pub write_can_empty: bool,
    pub write_can_append: bool,
    pub write_with_content_type: bool,
    pub write_with_content_disposition: bool,
    pub write_with_content_encoding: bool,
    pub write_with_cache_control: bool,
    pub write_with_if_match: bool,
    pub write_with_if_none_match: bool,
    pub write_with_if_not_exists: bool,
    pub write_with_user_metadata: bool,
    pub has_write_multi_max_size: bool,
    pub write_multi_max_size: usize,
    pub has_write_multi_min_size: bool,
    pub write_multi_min_size: usize,
    pub has_write_total_max_size: bool,
    pub write_total_max_size: usize,
    pub create_dir: bool,
    pub delete: bool,
    pub delete_with_version: bool,
    pub delete_with_recursive: bool,
    pub has_delete_max_size: bool,
    pub delete_max_size: usize,
    pub copy: bool,
    pub copy_with_if_not_exists: bool,
    pub rename: bool,
    pub list: bool,
    pub list_with_limit: bool,
    pub list_with_start_after: bool,
    pub list_with_recursive: bool,
    pub list_with_versions: bool,
    pub list_with_deleted: bool,
    pub presign: bool,
    pub presign_read: bool,
    pub presign_stat: bool,
    pub presign_write: bool,
    pub presign_delete: bool,
    pub shared: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct od_zig_metadata_view {
    pub mode: u8,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_deleted: bool,
    pub has_content_length: bool,
    pub content_length: u64,
    pub content_type: od_zig_slice,
    pub content_encoding: od_zig_slice,
    pub content_disposition: od_zig_slice,
    pub content_md5: od_zig_slice,
    pub etag: od_zig_slice,
    pub last_modified_rfc3339: od_zig_slice,
    pub version: od_zig_slice,
    pub user_metadata_ptr: *mut od_zig_header,
    pub user_metadata_len: usize,
}

#[repr(C)]
pub struct od_zig_metadata {
    pub mode: u8,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_deleted: bool,
    pub has_content_length: bool,
    pub content_length: u64,
    pub content_type: od_zig_slice,
    pub content_encoding: od_zig_slice,
    pub content_disposition: od_zig_slice,
    pub content_md5: od_zig_slice,
    pub etag: od_zig_slice,
    pub last_modified_rfc3339: od_zig_slice,
    pub version: od_zig_slice,
    pub user_metadata_ptr: *mut od_zig_header,
    pub user_metadata_len: usize,
    _content_type_storage: Option<Box<[u8]>>,
    _content_encoding_storage: Option<Box<[u8]>>,
    _content_disposition_storage: Option<Box<[u8]>>,
    _content_md5_storage: Option<Box<[u8]>>,
    _etag_storage: Option<Box<[u8]>>,
    _last_modified_storage: Option<Box<[u8]>>,
    _version_storage: Option<Box<[u8]>>,
    _header_fields: Box<[HeaderStorage]>,
    _header_views: Box<[od_zig_header]>,
}

impl Default for od_zig_metadata {
    fn default() -> Self {
        Self {
            mode: od_zig_entry_mode::OD_ZIG_ENTRY_MODE_UNKNOWN as u8,
            is_file: false,
            is_dir: false,
            is_deleted: false,
            has_content_length: false,
            content_length: 0,
            content_type: od_zig_slice::default(),
            content_encoding: od_zig_slice::default(),
            content_disposition: od_zig_slice::default(),
            content_md5: od_zig_slice::default(),
            etag: od_zig_slice::default(),
            last_modified_rfc3339: od_zig_slice::default(),
            version: od_zig_slice::default(),
            user_metadata_ptr: ptr::null_mut(),
            user_metadata_len: 0,
            _content_type_storage: None,
            _content_encoding_storage: None,
            _content_disposition_storage: None,
            _content_md5_storage: None,
            _etag_storage: None,
            _last_modified_storage: None,
            _version_storage: None,
            _header_fields: Box::default(),
            _header_views: Box::default(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct od_zig_entry_view {
    pub path: od_zig_slice,
    pub name: od_zig_slice,
    pub metadata: od_zig_metadata_view,
}

#[repr(C)]
pub struct od_zig_entry_owned {
    pub path: od_zig_slice,
    pub name: od_zig_slice,
    pub metadata: od_zig_metadata,
    _path_storage: Box<[u8]>,
    _name_storage: Box<[u8]>,
}

#[repr(C)]
pub struct od_zig_operator_info {
    pub scheme: od_zig_slice,
    pub root: od_zig_slice,
    pub name: od_zig_slice,
    pub full_capability: od_zig_capability,
    pub native_capability: od_zig_capability,
    _scheme_storage: Box<[u8]>,
    _root_storage: Box<[u8]>,
    _name_storage: Box<[u8]>,
}

#[repr(C)]
pub struct od_zig_presigned_request {
    pub method: od_zig_slice,
    pub url: od_zig_slice,
    pub headers_ptr: *mut od_zig_header,
    pub headers_len: usize,
    _method_storage: Box<[u8]>,
    _url_storage: Box<[u8]>,
    _header_fields: Box<[HeaderStorage]>,
    _header_views: Box<[od_zig_header]>,
}

#[repr(C)]
pub struct od_zig_error_info {
    pub code: od_zig_error_code,
    pub message: od_zig_slice,
    _message_storage: Box<[u8]>,
}

unsafe impl Send for od_zig_header {}
unsafe impl Sync for od_zig_header {}
unsafe impl Send for od_zig_metadata {}
unsafe impl Sync for od_zig_metadata {}
unsafe impl Send for od_zig_entry_owned {}
unsafe impl Sync for od_zig_entry_owned {}
unsafe impl Send for od_zig_operator_info {}
unsafe impl Sync for od_zig_operator_info {}
unsafe impl Send for od_zig_presigned_request {}
unsafe impl Sync for od_zig_presigned_request {}
unsafe impl Send for od_zig_error_info {}
unsafe impl Sync for od_zig_error_info {}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct od_zig_read_options {
    pub version: od_zig_slice,
    pub if_match: od_zig_slice,
    pub if_none_match: od_zig_slice,
    pub if_modified_since_http_date: od_zig_slice,
    pub if_unmodified_since_http_date: od_zig_slice,
    pub override_content_type: od_zig_slice,
    pub override_cache_control: od_zig_slice,
    pub override_content_disposition: od_zig_slice,
    pub has_offset: bool,
    pub offset: u64,
    pub has_size: bool,
    pub size: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct od_zig_write_options {
    pub content_type: od_zig_slice,
    pub content_disposition: od_zig_slice,
    pub content_encoding: od_zig_slice,
    pub cache_control: od_zig_slice,
    pub if_match: od_zig_slice,
    pub if_none_match: od_zig_slice,
    pub if_not_exists: bool,
    pub append: bool,
    pub user_metadata_ptr: *const od_zig_header,
    pub user_metadata_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct od_zig_stat_options {
    pub if_match: od_zig_slice,
    pub if_none_match: od_zig_slice,
    pub if_modified_since_http_date: od_zig_slice,
    pub if_unmodified_since_http_date: od_zig_slice,
    pub override_content_type: od_zig_slice,
    pub override_cache_control: od_zig_slice,
    pub override_content_disposition: od_zig_slice,
    pub version: od_zig_slice,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct od_zig_delete_options {
    pub version: od_zig_slice,
    pub recursive: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct od_zig_copy_options {
    pub if_not_exists: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct od_zig_list_options {
    pub recursive: bool,
    pub has_limit: bool,
    pub limit: usize,
    pub start_after: od_zig_slice,
    pub versions: bool,
    pub deleted: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct od_zig_presign_options {
    pub version: od_zig_slice,
    pub if_match: od_zig_slice,
    pub if_none_match: od_zig_slice,
    pub if_modified_since_http_date: od_zig_slice,
    pub if_unmodified_since_http_date: od_zig_slice,
    pub override_content_type: od_zig_slice,
    pub override_cache_control: od_zig_slice,
    pub override_content_disposition: od_zig_slice,
    pub content_encoding: od_zig_slice,
    pub if_not_exists: bool,
}

macro_rules! pointer_result {
    ($name:ident, $ty:ty) => {
        #[repr(C)]
        pub struct $name {
            pub code: od_zig_error_code,
            pub value: *mut $ty,
        }
    };
}

pointer_result!(od_zig_result_runtime, od_zig_runtime);
pointer_result!(od_zig_result_operator, od_zig_operator);
pointer_result!(od_zig_result_reader, od_zig_reader);
pointer_result!(od_zig_result_writer, od_zig_writer);
pointer_result!(od_zig_result_lister, od_zig_lister);
pointer_result!(od_zig_result_async_reader, od_zig_async_reader);
pointer_result!(od_zig_result_async_writer, od_zig_async_writer);
pointer_result!(od_zig_result_async_lister, od_zig_async_lister);
pointer_result!(od_zig_result_bytes, od_zig_bytes);
pointer_result!(od_zig_result_metadata, od_zig_metadata);
pointer_result!(od_zig_result_info, od_zig_operator_info);
pointer_result!(od_zig_result_presigned_request, od_zig_presigned_request);
pointer_result!(od_zig_result_task, od_zig_task);

#[repr(C)]
pub struct od_zig_result_error_info_optional {
    pub code: od_zig_error_code,
    pub has_value: bool,
    pub value: *mut od_zig_error_info,
}

#[repr(C)]
pub struct od_zig_result_bool {
    pub code: od_zig_error_code,
    pub value: bool,
}

#[repr(C)]
pub struct od_zig_result_unit {
    pub code: od_zig_error_code,
}

#[repr(C)]
pub struct od_zig_result_usize {
    pub code: od_zig_error_code,
    pub value: usize,
}

#[repr(C)]
pub struct od_zig_result_u64 {
    pub code: od_zig_error_code,
    pub value: u64,
}

#[repr(C)]
pub struct od_zig_result_optional_u64 {
    pub code: od_zig_error_code,
    pub has_value: bool,
    pub value: u64,
}

#[repr(C)]
pub struct od_zig_result_entry_view {
    pub code: od_zig_error_code,
    pub has_value: bool,
    pub value: od_zig_entry_view,
}

#[repr(C)]
pub struct od_zig_result_entry_owned {
    pub code: od_zig_error_code,
    pub has_value: bool,
    pub value: *mut od_zig_entry_owned,
}

#[repr(C)]
pub struct od_zig_task_poll_result {
    pub code: od_zig_error_code,
    pub state: od_zig_poll_state,
}

#[repr(C)]
pub struct od_zig_task_cancel_result {
    pub code: od_zig_error_code,
    pub completed: bool,
}

#[repr(C)]
pub struct od_zig_task_state_result {
    pub code: od_zig_error_code,
    pub state: od_zig_task_state,
}

#[derive(Default)]
struct HeaderStorage {
    _key: Box<[u8]>,
    _value: Box<[u8]>,
}

struct StoredError {
    code: od_zig_error_code,
    message: String,
}

impl StoredError {
    fn new(code: od_zig_error_code, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::new(od_zig_error_code::OD_ZIG_INTERNAL, message)
    }

    fn canceled() -> Self {
        Self::new(od_zig_error_code::OD_ZIG_CANCELED, "task canceled")
    }
}

enum TaskCompletion {
    Pending,
    Canceling,
    Ready(TaskOutcome),
    Canceled,
    Consumed,
}

struct TaskInner {
    state: Mutex<TaskCompletion>,
    join: Mutex<Option<JoinHandle<()>>>,
}

struct NotifierPipe {
    read_fd: OwnedFd,
    write_fd: OwnedFd,
}

impl NotifierPipe {
    #[cfg(unix)]
    fn new() -> Result<Self, StoredError> {
        let mut fds = [0; 2];
        let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
        if rc != 0 {
            return Err(StoredError::new(
                od_zig_error_code::OD_ZIG_INTERNAL,
                format!("create notifier pipe failed: {}", std::io::Error::last_os_error()),
            ));
        }

        for fd in fds {
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if flags >= 0 {
                unsafe {
                    libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
                }
            }
        }

        Ok(Self {
            read_fd: unsafe { OwnedFd::from_raw_fd(fds[0]) },
            write_fd: unsafe { OwnedFd::from_raw_fd(fds[1]) },
        })
    }

    fn signal(&self) {
        let buf = [1u8; 1];
        let _ = unsafe {
            libc::write(
                self.write_fd.as_raw_fd(),
                buf.as_ptr().cast(),
                buf.len(),
            )
        };
    }

    fn drain(&self) -> usize {
        let mut total = 0usize;
        let mut buf = [0u8; 256];
        loop {
            let read = unsafe {
                libc::read(
                    self.read_fd.as_raw_fd(),
                    buf.as_mut_ptr().cast(),
                    buf.len(),
                )
            };
            if read > 0 {
                total += read as usize;
                continue;
            }
            break;
        }
        total
    }

    fn fd(&self) -> i32 {
        self.read_fd.as_raw_fd()
    }
}

struct RuntimeShared {
    _runtime: TokioRuntime,
    handle: TokioHandle,
    tasks: Mutex<HashMap<u64, Arc<TaskInner>>>,
    ready_tasks: Mutex<VecDeque<u64>>,
    next_task_id: AtomicU64,
    notifier: Option<NotifierPipe>,
    last_error: Mutex<Option<(od_zig_error_code, String)>>,
}

impl RuntimeShared {
    fn new(options: od_zig_runtime_options) -> Result<Self, StoredError> {
        let mut builder = TokioRuntimeBuilder::new_multi_thread();
        builder.enable_all();
        if options.has_worker_threads {
            builder.worker_threads(options.worker_threads.max(1).into());
        }

        let runtime = builder.build().map_err(|err| {
            StoredError::new(
                od_zig_error_code::OD_ZIG_INTERNAL,
                format!("build tokio runtime failed: {err}"),
            )
        })?;
        let handle = runtime.handle().clone();

        Ok(Self {
            _runtime: runtime,
            handle,
            tasks: Mutex::new(HashMap::new()),
            ready_tasks: Mutex::new(VecDeque::with_capacity(
                options.completion_queue_capacity.max(1),
            )),
            next_task_id: AtomicU64::new(1),
            notifier: Some(NotifierPipe::new()?),
            last_error: Mutex::new(None),
        })
    }

    fn push_ready(&self, task_id: u64) {
        self.ready_tasks.lock().expect("poisoned").push_back(task_id);
        if let Some(notifier) = &self.notifier {
            notifier.signal();
        }
    }

    fn take_task(&self, task_id: u64) -> Option<Arc<TaskInner>> {
        self.tasks.lock().expect("poisoned").get(&task_id).cloned()
    }

    fn record_error(&self, err: &StoredError) {
        *self.last_error.lock().expect("poisoned") = Some((err.code, err.message.clone()));
    }

    fn clear_error(&self) {
        *self.last_error.lock().expect("poisoned") = None;
    }

    fn map_and_record<T>(&self, result: Result<T, StoredError>) -> Result<T, StoredError> {
        match result {
            Ok(value) => {
                self.clear_error();
                Ok(value)
            }
            Err(err) => {
                self.record_error(&err);
                Err(err)
            }
        }
    }
}

pub struct od_zig_runtime {
    inner: Arc<RuntimeShared>,
}

pub struct od_zig_operator {
    op: Operator,
    runtime: Arc<RuntimeShared>,
    async_enabled: bool,
}

pub struct od_zig_bytes {
    bytes: bytes::Bytes,
}

struct ReaderHandleInner {
    runtime: Arc<RuntimeShared>,
    inner: Arc<AsyncMutex<FuturesAsyncReader>>,
}

pub struct od_zig_reader {
    inner: ReaderHandleInner,
}

pub struct od_zig_async_reader {
    inner: ReaderHandleInner,
}

struct WriterHandleInner {
    runtime: Arc<RuntimeShared>,
    inner: Arc<AsyncMutex<Writer>>,
}

pub struct od_zig_writer {
    inner: WriterHandleInner,
}

pub struct od_zig_async_writer {
    inner: WriterHandleInner,
}

struct SyncListerInner {
    runtime: Arc<RuntimeShared>,
    inner: Arc<AsyncMutex<Lister>>,
    last_entry: Mutex<Option<Box<od_zig_entry_owned>>>,
}

pub struct od_zig_lister {
    inner: SyncListerInner,
}

struct AsyncListerInner {
    runtime: Arc<RuntimeShared>,
    inner: Arc<AsyncMutex<Lister>>,
}

pub struct od_zig_async_lister {
    inner: AsyncListerInner,
}

pub struct od_zig_task {
    runtime: Arc<RuntimeShared>,
    task_id: u64,
}

enum TaskValue {
    Unit,
    Bool(bool),
    Usize(usize),
    U64(u64),
    Bytes(Box<od_zig_bytes>),
    Metadata(Box<od_zig_metadata>),
    Info(Box<od_zig_operator_info>),
    PresignedRequest(Box<od_zig_presigned_request>),
    Entry(Option<Box<od_zig_entry_owned>>),
    Reader(Box<od_zig_async_reader>),
    Writer(Box<od_zig_async_writer>),
    Lister(Box<od_zig_async_lister>),
}

fn registry_init() {
    static REGISTRY_INIT: OnceLock<()> = OnceLock::new();
    REGISTRY_INIT.get_or_init(|| {
        opendal::init_default_registry();
    });
}

fn error_code_from_kind(kind: ErrorKind) -> od_zig_error_code {
    match kind {
        ErrorKind::Unexpected => od_zig_error_code::OD_ZIG_UNEXPECTED,
        ErrorKind::Unsupported => od_zig_error_code::OD_ZIG_UNSUPPORTED,
        ErrorKind::ConfigInvalid => od_zig_error_code::OD_ZIG_CONFIG_INVALID,
        ErrorKind::NotFound => od_zig_error_code::OD_ZIG_NOT_FOUND,
        ErrorKind::PermissionDenied => od_zig_error_code::OD_ZIG_PERMISSION_DENIED,
        ErrorKind::IsADirectory => od_zig_error_code::OD_ZIG_IS_DIRECTORY,
        ErrorKind::NotADirectory => od_zig_error_code::OD_ZIG_NOT_A_DIRECTORY,
        ErrorKind::AlreadyExists => od_zig_error_code::OD_ZIG_ALREADY_EXISTS,
        ErrorKind::RateLimited => od_zig_error_code::OD_ZIG_RATE_LIMITED,
        ErrorKind::IsSameFile => od_zig_error_code::OD_ZIG_IS_SAME_FILE,
        ErrorKind::ConditionNotMatch => od_zig_error_code::OD_ZIG_CONDITION_NOT_MATCH,
        ErrorKind::RangeNotSatisfied => od_zig_error_code::OD_ZIG_RANGE_NOT_SATISFIED,
        _ => od_zig_error_code::OD_ZIG_INTERNAL,
    }
}

fn stored_from_opendal_error(err: Error) -> StoredError {
    StoredError::new(error_code_from_kind(err.kind()), err.to_string())
}

fn stored_from_io_error(err: std::io::Error) -> StoredError {
    StoredError::new(od_zig_error_code::OD_ZIG_UNEXPECTED, err.to_string())
}

unsafe fn slice_from_ffi(input: od_zig_slice) -> &'static [u8] {
    if input.ptr.is_null() {
        &[]
    } else {
        unsafe { slice::from_raw_parts(input.ptr, input.len) }
    }
}

unsafe fn slice_from_ffi_mut(input: od_zig_mut_slice) -> &'static mut [u8] {
    if input.ptr.is_null() {
        &mut []
    } else {
        unsafe { slice::from_raw_parts_mut(input.ptr, input.len) }
    }
}

fn clone_bytes(input: od_zig_slice) -> Vec<u8> {
    unsafe { slice_from_ffi(input) }.to_vec()
}

fn clone_string(input: od_zig_slice) -> Result<String, StoredError> {
    let bytes = unsafe { slice_from_ffi(input) };
    str::from_utf8(bytes)
        .map(|value| value.to_string())
        .map_err(|err| {
            StoredError::new(
                od_zig_error_code::OD_ZIG_INVALID_ARGUMENT,
                format!("invalid utf-8 input: {err}"),
            )
        })
}

fn clone_optional_string(input: od_zig_slice) -> Result<Option<String>, StoredError> {
    if input.ptr.is_null() {
        Ok(None)
    } else {
        clone_string(input).map(Some)
    }
}

fn bytes_box(bytes: impl Into<Vec<u8>>) -> Box<[u8]> {
    bytes.into().into_boxed_slice()
}

fn od_slice_from_box(value: &Option<Box<[u8]>>) -> od_zig_slice {
    value
        .as_ref()
        .map(|data| od_zig_slice {
            ptr: data.as_ptr(),
            len: data.len(),
        })
        .unwrap_or_default()
}

fn od_slice_from_box_ref(value: &Box<[u8]>) -> od_zig_slice {
    od_zig_slice {
        ptr: value.as_ptr(),
        len: value.len(),
    }
}

fn convert_entry_mode(mode: EntryMode) -> u8 {
    match mode {
        EntryMode::FILE => od_zig_entry_mode::OD_ZIG_ENTRY_MODE_FILE as u8,
        EntryMode::DIR => od_zig_entry_mode::OD_ZIG_ENTRY_MODE_DIR as u8,
        EntryMode::Unknown => od_zig_entry_mode::OD_ZIG_ENTRY_MODE_UNKNOWN as u8,
    }
}

fn build_headers(data: Option<&HashMap<String, String>>) -> (Box<[HeaderStorage]>, Box<[od_zig_header]>) {
    let Some(data) = data else {
        return (Box::default(), Box::default());
    };

    let mut fields = Vec::with_capacity(data.len());
    let mut views = Vec::with_capacity(data.len());
    for (key, value) in data {
        let key = bytes_box(key.clone().into_bytes());
        let value = bytes_box(value.clone().into_bytes());
        let view = od_zig_header {
            key: od_slice_from_box_ref(&key),
            value: od_slice_from_box_ref(&value),
        };
        fields.push(HeaderStorage {
            _key: key,
            _value: value,
        });
        views.push(view);
    }
    (fields.into_boxed_slice(), views.into_boxed_slice())
}

fn build_headers_from_http(headers: &http::HeaderMap) -> (Box<[HeaderStorage]>, Box<[od_zig_header]>) {
    let mut fields = Vec::with_capacity(headers.len());
    let mut views = Vec::with_capacity(headers.len());
    for (key, value) in headers {
        let key_box = bytes_box(key.as_str().as_bytes().to_vec());
        let value_box = bytes_box(value.as_bytes().to_vec());
        let view = od_zig_header {
            key: od_slice_from_box_ref(&key_box),
            value: od_slice_from_box_ref(&value_box),
        };
        fields.push(HeaderStorage {
            _key: key_box,
            _value: value_box,
        });
        views.push(view);
    }
    (fields.into_boxed_slice(), views.into_boxed_slice())
}

fn build_capability(cap: Capability) -> od_zig_capability {
    od_zig_capability {
        stat: cap.stat,
        stat_with_if_match: cap.stat_with_if_match,
        stat_with_if_none_match: cap.stat_with_if_none_match,
        stat_with_if_modified_since: cap.stat_with_if_modified_since,
        stat_with_if_unmodified_since: cap.stat_with_if_unmodified_since,
        stat_with_override_cache_control: cap.stat_with_override_cache_control,
        stat_with_override_content_disposition: cap.stat_with_override_content_disposition,
        stat_with_override_content_type: cap.stat_with_override_content_type,
        stat_with_version: cap.stat_with_version,
        read: cap.read,
        read_with_if_match: cap.read_with_if_match,
        read_with_if_none_match: cap.read_with_if_none_match,
        read_with_if_modified_since: cap.read_with_if_modified_since,
        read_with_if_unmodified_since: cap.read_with_if_unmodified_since,
        read_with_override_cache_control: cap.read_with_override_cache_control,
        read_with_override_content_disposition: cap.read_with_override_content_disposition,
        read_with_override_content_type: cap.read_with_override_content_type,
        read_with_version: cap.read_with_version,
        write: cap.write,
        write_can_multi: cap.write_can_multi,
        write_can_empty: cap.write_can_empty,
        write_can_append: cap.write_can_append,
        write_with_content_type: cap.write_with_content_type,
        write_with_content_disposition: cap.write_with_content_disposition,
        write_with_content_encoding: cap.write_with_content_encoding,
        write_with_cache_control: cap.write_with_cache_control,
        write_with_if_match: cap.write_with_if_match,
        write_with_if_none_match: cap.write_with_if_none_match,
        write_with_if_not_exists: cap.write_with_if_not_exists,
        write_with_user_metadata: cap.write_with_user_metadata,
        has_write_multi_max_size: cap.write_multi_max_size.is_some(),
        write_multi_max_size: cap.write_multi_max_size.unwrap_or_default(),
        has_write_multi_min_size: cap.write_multi_min_size.is_some(),
        write_multi_min_size: cap.write_multi_min_size.unwrap_or_default(),
        has_write_total_max_size: cap.write_total_max_size.is_some(),
        write_total_max_size: cap.write_total_max_size.unwrap_or_default(),
        create_dir: cap.create_dir,
        delete: cap.delete,
        delete_with_version: cap.delete_with_version,
        delete_with_recursive: cap.delete_with_recursive,
        has_delete_max_size: cap.delete_max_size.is_some(),
        delete_max_size: cap.delete_max_size.unwrap_or_default(),
        copy: cap.copy,
        copy_with_if_not_exists: cap.copy_with_if_not_exists,
        rename: cap.rename,
        list: cap.list,
        list_with_limit: cap.list_with_limit,
        list_with_start_after: cap.list_with_start_after,
        list_with_recursive: cap.list_with_recursive,
        list_with_versions: cap.list_with_versions,
        list_with_deleted: cap.list_with_deleted,
        presign: cap.presign,
        presign_read: cap.presign_read,
        presign_stat: cap.presign_stat,
        presign_write: cap.presign_write,
        presign_delete: cap.presign_delete,
        shared: cap.shared,
    }
}

fn build_metadata_view(meta: &od_zig_metadata) -> od_zig_metadata_view {
    od_zig_metadata_view {
        mode: meta.mode,
        is_file: meta.is_file,
        is_dir: meta.is_dir,
        is_deleted: meta.is_deleted,
        has_content_length: meta.has_content_length,
        content_length: meta.content_length,
        content_type: meta.content_type,
        content_encoding: meta.content_encoding,
        content_disposition: meta.content_disposition,
        content_md5: meta.content_md5,
        etag: meta.etag,
        last_modified_rfc3339: meta.last_modified_rfc3339,
        version: meta.version,
        user_metadata_ptr: meta.user_metadata_ptr,
        user_metadata_len: meta.user_metadata_len,
    }
}

fn build_metadata(meta: Metadata) -> Box<od_zig_metadata> {
    let mut owned = Box::new(od_zig_metadata::default());

    owned.mode = convert_entry_mode(meta.mode());
    owned.is_file = meta.is_file();
    owned.is_dir = meta.is_dir();
    owned.is_deleted = meta.is_deleted();

    let content_length = meta.content_length();
    owned.has_content_length = content_length != 0 || meta.content_length() != 0;
    if owned.has_content_length {
        owned.content_length = content_length;
    }

    owned._content_type_storage = meta.content_type().map(|value| bytes_box(value.as_bytes().to_vec()));
    owned._content_encoding_storage = meta.content_encoding().map(|value| bytes_box(value.as_bytes().to_vec()));
    owned._content_disposition_storage = meta
        .content_disposition()
        .map(|value| bytes_box(value.as_bytes().to_vec()));
    owned._content_md5_storage = meta.content_md5().map(|value| bytes_box(value.as_bytes().to_vec()));
    owned._etag_storage = meta.etag().map(|value| bytes_box(value.as_bytes().to_vec()));
    owned._last_modified_storage = meta
        .last_modified()
        .map(|value| bytes_box(value.to_string().into_bytes()));
    owned._version_storage = meta.version().map(|value| bytes_box(value.as_bytes().to_vec()));

    owned.content_type = od_slice_from_box(&owned._content_type_storage);
    owned.content_encoding = od_slice_from_box(&owned._content_encoding_storage);
    owned.content_disposition = od_slice_from_box(&owned._content_disposition_storage);
    owned.content_md5 = od_slice_from_box(&owned._content_md5_storage);
    owned.etag = od_slice_from_box(&owned._etag_storage);
    owned.last_modified_rfc3339 = od_slice_from_box(&owned._last_modified_storage);
    owned.version = od_slice_from_box(&owned._version_storage);

    let (header_fields, mut header_views) = build_headers(meta.user_metadata());
    owned.user_metadata_ptr = if header_views.is_empty() {
        ptr::null_mut()
    } else {
        header_views.as_mut_ptr()
    };
    owned.user_metadata_len = header_views.len();
    owned._header_fields = header_fields;
    owned._header_views = header_views;
    owned
}

fn build_entry(entry: Entry) -> Box<od_zig_entry_owned> {
    let name = entry.name().to_string();
    let (path, metadata) = entry.into_parts();
    let path_storage = bytes_box(path.clone().into_bytes());
    let name_storage = bytes_box(name.into_bytes());
    let metadata = *build_metadata(metadata);

    Box::new(od_zig_entry_owned {
        path: od_slice_from_box_ref(&path_storage),
        name: od_slice_from_box_ref(&name_storage),
        metadata,
        _path_storage: path_storage,
        _name_storage: name_storage,
    })
}

fn build_info(info: OperatorInfo) -> Box<od_zig_operator_info> {
    let scheme_storage = bytes_box(info.scheme().as_bytes().to_vec());
    let root_storage = bytes_box(info.root().into_bytes());
    let name_storage = bytes_box(info.name().into_bytes());

    Box::new(od_zig_operator_info {
        scheme: od_slice_from_box_ref(&scheme_storage),
        root: od_slice_from_box_ref(&root_storage),
        name: od_slice_from_box_ref(&name_storage),
        full_capability: build_capability(info.full_capability()),
        native_capability: build_capability(info.native_capability()),
        _scheme_storage: scheme_storage,
        _root_storage: root_storage,
        _name_storage: name_storage,
    })
}

fn build_presigned_request(req: PresignedRequest) -> Box<od_zig_presigned_request> {
    let method_storage = bytes_box(req.method().as_str().as_bytes().to_vec());
    let url_storage = bytes_box(req.uri().to_string().into_bytes());
    let (header_fields, mut header_views) = build_headers_from_http(req.header());

    Box::new(od_zig_presigned_request {
        method: od_slice_from_box_ref(&method_storage),
        url: od_slice_from_box_ref(&url_storage),
        headers_ptr: if header_views.is_empty() {
            ptr::null_mut()
        } else {
            header_views.as_mut_ptr()
        },
        headers_len: header_views.len(),
        _method_storage: method_storage,
        _url_storage: url_storage,
        _header_fields: header_fields,
        _header_views: header_views,
    })
}

fn build_error_info(code: od_zig_error_code, message: String) -> Box<od_zig_error_info> {
    let message_storage = bytes_box(message.into_bytes());
    Box::new(od_zig_error_info {
        code,
        message: od_slice_from_box_ref(&message_storage),
        _message_storage: message_storage,
    })
}

fn build_bytes(buffer: opendal::Buffer) -> Box<od_zig_bytes> {
    let bytes = if buffer.clone().count() <= 1 {
        buffer.current()
    } else {
        buffer.to_bytes()
    };

    Box::new(od_zig_bytes { bytes })
}

fn bool_result(code: od_zig_error_code, value: bool) -> od_zig_result_bool {
    od_zig_result_bool { code, value }
}

fn unit_ok() -> od_zig_result_unit {
    od_zig_result_unit {
        code: od_zig_error_code::OD_ZIG_OK,
    }
}

fn usize_result(code: od_zig_error_code, value: usize) -> od_zig_result_usize {
    od_zig_result_usize { code, value }
}

fn u64_result(code: od_zig_error_code, value: u64) -> od_zig_result_u64 {
    od_zig_result_u64 { code, value }
}

fn empty_runtime_options() -> od_zig_runtime_options {
    od_zig_runtime_options {
        has_worker_threads: false,
        worker_threads: 0,
        completion_queue_capacity: 1024,
        cancellation_check_interval_ms: 10,
    }
}

fn create_operator(
    runtime: Arc<RuntimeShared>,
    scheme: od_zig_slice,
    options_ptr: *const od_zig_option,
    options_len: usize,
    async_enabled: bool,
) -> Result<Box<od_zig_operator>, StoredError> {
    registry_init();
    let scheme = clone_string(scheme)?;
    let options = if options_ptr.is_null() || options_len == 0 {
        Vec::new()
    } else {
        let values = unsafe { slice::from_raw_parts(options_ptr, options_len) };
        let mut out = Vec::with_capacity(values.len());
        for option in values {
            out.push((clone_string(option.key)?, clone_string(option.value)?));
        }
        out
    };

    let op = Operator::via_iter(scheme, options).map_err(stored_from_opendal_error)?;
    Ok(Box::new(od_zig_operator {
        op,
        runtime,
        async_enabled,
    }))
}

fn parse_timestamp(value: od_zig_slice) -> Result<Option<Timestamp>, StoredError> {
    let Some(value) = clone_optional_string(value)? else {
        return Ok(None);
    };

    Timestamp::parse_rfc2822(&value)
        .map(Some)
        .map_err(|err| {
            StoredError::new(
                od_zig_error_code::OD_ZIG_INVALID_ARGUMENT,
                format!("invalid http date '{value}': {err}"),
            )
        })
}

fn convert_read_options(input: od_zig_read_options) -> Result<options::ReadOptions, StoredError> {
    let mut out = options::ReadOptions::default();
    out.version = clone_optional_string(input.version)?;
    out.if_match = clone_optional_string(input.if_match)?;
    out.if_none_match = clone_optional_string(input.if_none_match)?;
    out.if_modified_since = parse_timestamp(input.if_modified_since_http_date)?;
    out.if_unmodified_since = parse_timestamp(input.if_unmodified_since_http_date)?;
    out.override_content_type = clone_optional_string(input.override_content_type)?;
    out.override_cache_control = clone_optional_string(input.override_cache_control)?;
    out.override_content_disposition = clone_optional_string(input.override_content_disposition)?;

    if input.has_offset || input.has_size {
        let start = input.offset;
        let end = if input.has_size {
            start.checked_add(input.size).ok_or_else(|| {
                StoredError::new(
                    od_zig_error_code::OD_ZIG_INVALID_ARGUMENT,
                    "read range overflows u64",
                )
            })?
        } else {
            u64::MAX
        };

        out.range = if input.has_size {
            (start..end).into()
        } else {
            (start..).into()
        };
    }

    Ok(out)
}

fn convert_reader_options(input: od_zig_read_options) -> Result<options::ReaderOptions, StoredError> {
    Ok(options::ReaderOptions {
        version: clone_optional_string(input.version)?,
        if_match: clone_optional_string(input.if_match)?,
        if_none_match: clone_optional_string(input.if_none_match)?,
        if_modified_since: parse_timestamp(input.if_modified_since_http_date)?,
        if_unmodified_since: parse_timestamp(input.if_unmodified_since_http_date)?,
        ..Default::default()
    })
}

fn convert_headers_input(
    ptr: *const od_zig_header,
    len: usize,
) -> Result<Option<HashMap<String, String>>, StoredError> {
    if ptr.is_null() || len == 0 {
        return Ok(None);
    }

    let headers = unsafe { slice::from_raw_parts(ptr, len) };
    let mut out = HashMap::with_capacity(len);
    for header in headers {
        out.insert(clone_string(header.key)?, clone_string(header.value)?);
    }
    Ok(Some(out))
}

fn convert_write_options(input: od_zig_write_options) -> Result<options::WriteOptions, StoredError> {
    Ok(options::WriteOptions {
        append: input.append,
        cache_control: clone_optional_string(input.cache_control)?,
        content_type: clone_optional_string(input.content_type)?,
        content_disposition: clone_optional_string(input.content_disposition)?,
        content_encoding: clone_optional_string(input.content_encoding)?,
        user_metadata: convert_headers_input(input.user_metadata_ptr, input.user_metadata_len)?,
        if_match: clone_optional_string(input.if_match)?,
        if_none_match: clone_optional_string(input.if_none_match)?,
        if_not_exists: input.if_not_exists,
        ..Default::default()
    })
}

fn convert_stat_options(input: od_zig_stat_options) -> Result<options::StatOptions, StoredError> {
    Ok(options::StatOptions {
        version: clone_optional_string(input.version)?,
        if_match: clone_optional_string(input.if_match)?,
        if_none_match: clone_optional_string(input.if_none_match)?,
        if_modified_since: parse_timestamp(input.if_modified_since_http_date)?,
        if_unmodified_since: parse_timestamp(input.if_unmodified_since_http_date)?,
        override_content_type: clone_optional_string(input.override_content_type)?,
        override_cache_control: clone_optional_string(input.override_cache_control)?,
        override_content_disposition: clone_optional_string(input.override_content_disposition)?,
    })
}

fn convert_delete_options(
    input: od_zig_delete_options,
) -> Result<options::DeleteOptions, StoredError> {
    Ok(options::DeleteOptions {
        version: clone_optional_string(input.version)?,
        recursive: input.recursive,
    })
}

fn convert_list_options(input: od_zig_list_options) -> Result<options::ListOptions, StoredError> {
    Ok(options::ListOptions {
        limit: input.has_limit.then_some(input.limit),
        start_after: clone_optional_string(input.start_after)?,
        recursive: input.recursive,
        versions: input.versions,
        deleted: input.deleted,
    })
}

fn convert_presign_read_options(
    input: od_zig_presign_options,
) -> Result<options::ReadOptions, StoredError> {
    Ok(options::ReadOptions {
        version: clone_optional_string(input.version)?,
        if_match: clone_optional_string(input.if_match)?,
        if_none_match: clone_optional_string(input.if_none_match)?,
        if_modified_since: parse_timestamp(input.if_modified_since_http_date)?,
        if_unmodified_since: parse_timestamp(input.if_unmodified_since_http_date)?,
        override_content_type: clone_optional_string(input.override_content_type)?,
        override_cache_control: clone_optional_string(input.override_cache_control)?,
        override_content_disposition: clone_optional_string(input.override_content_disposition)?,
        ..Default::default()
    })
}

fn convert_presign_write_options(
    input: od_zig_presign_options,
) -> Result<options::WriteOptions, StoredError> {
    Ok(options::WriteOptions {
        content_encoding: clone_optional_string(input.content_encoding)?,
        if_match: clone_optional_string(input.if_match)?,
        if_none_match: clone_optional_string(input.if_none_match)?,
        if_not_exists: input.if_not_exists,
        content_type: clone_optional_string(input.override_content_type)?,
        cache_control: clone_optional_string(input.override_cache_control)?,
        content_disposition: clone_optional_string(input.override_content_disposition)?,
        ..Default::default()
    })
}

fn convert_presign_stat_options(
    input: od_zig_presign_options,
) -> Result<options::StatOptions, StoredError> {
    Ok(options::StatOptions {
        version: clone_optional_string(input.version)?,
        if_match: clone_optional_string(input.if_match)?,
        if_none_match: clone_optional_string(input.if_none_match)?,
        if_modified_since: parse_timestamp(input.if_modified_since_http_date)?,
        if_unmodified_since: parse_timestamp(input.if_unmodified_since_http_date)?,
        override_content_type: clone_optional_string(input.override_content_type)?,
        override_cache_control: clone_optional_string(input.override_cache_control)?,
        override_content_disposition: clone_optional_string(input.override_content_disposition)?,
    })
}

fn convert_presign_delete_options(
    input: od_zig_presign_options,
) -> Result<options::DeleteOptions, StoredError> {
    Ok(options::DeleteOptions {
        version: clone_optional_string(input.version)?,
        recursive: false,
    })
}

fn operator_runtime(operator: *mut od_zig_operator) -> Result<&'static mut od_zig_operator, StoredError> {
    unsafe { operator.as_mut() }.ok_or_else(|| {
        StoredError::new(
            od_zig_error_code::OD_ZIG_INVALID_ARGUMENT,
            "operator pointer is null",
        )
    })
}

fn spawn_task<F>(runtime: Arc<RuntimeShared>, future: F) -> *mut od_zig_task
where
    F: std::future::Future<Output = TaskOutcome> + Send + 'static,
{
    let task_id = runtime.next_task_id.fetch_add(1, Ordering::Relaxed);
    let inner = Arc::new(TaskInner {
        state: Mutex::new(TaskCompletion::Pending),
        join: Mutex::new(None),
    });
    runtime
        .tasks
        .lock()
        .expect("poisoned")
        .insert(task_id, inner.clone());

    let runtime_clone = runtime.clone();
    let inner_clone = inner.clone();
    let join = runtime.handle.spawn(async move {
        let outcome = match AssertUnwindSafe(future).catch_unwind().await {
            Ok(outcome) => outcome,
            Err(payload) => {
                let message = if let Some(message) = payload.downcast_ref::<&str>() {
                    format!("async task panicked: {message}")
                } else if let Some(message) = payload.downcast_ref::<String>() {
                    format!("async task panicked: {message}")
                } else {
                    "async task panicked".to_string()
                };
                Err(StoredError::new(od_zig_error_code::OD_ZIG_INTERNAL, message))
            }
        };
        let should_signal = {
            let mut state = inner_clone.state.lock().expect("poisoned");
            match &*state {
                TaskCompletion::Pending | TaskCompletion::Canceling => {
                    *state = TaskCompletion::Ready(outcome);
                    true
                }
                TaskCompletion::Canceled | TaskCompletion::Consumed | TaskCompletion::Ready(_) => false,
            }
        };

        if should_signal {
            runtime_clone.push_ready(task_id);
        }
    });
    *inner.join.lock().expect("poisoned") = Some(join);

    Box::into_raw(Box::new(od_zig_task { runtime, task_id }))
}

fn async_runtime_for_operator(
    operator: &od_zig_operator,
) -> Result<Arc<RuntimeShared>, StoredError> {
    if operator.async_enabled {
        Ok(operator.runtime.clone())
    } else {
        Err(StoredError::new(
            od_zig_error_code::OD_ZIG_ASYNC_RUNTIME_REQUIRED,
            "async runtime is required for async APIs",
        ))
    }
}

fn task_inner(task: *mut od_zig_task) -> Result<(&'static mut od_zig_task, Arc<TaskInner>), StoredError> {
    let task_ref = unsafe { task.as_mut() }.ok_or_else(|| {
        StoredError::new(
            od_zig_error_code::OD_ZIG_INVALID_ARGUMENT,
            "task pointer is null",
        )
    })?;
    let inner = task_ref.runtime.take_task(task_ref.task_id).ok_or_else(|| {
        StoredError::internal("task not found in runtime registry")
    })?;
    Ok((task_ref, inner))
}

fn take_task_value<R>(
    task: *mut od_zig_task,
    extractor: impl FnOnce(TaskValue) -> Result<R, TaskValue>,
) -> Result<R, StoredError> {
    let (_, inner) = task_inner(task)?;
    let mut state = inner.state.lock().expect("poisoned");
    let current = std::mem::replace(&mut *state, TaskCompletion::Consumed);
    match current {
        TaskCompletion::Ready(Ok(value)) => extractor(value).map_err(|value| {
            *state = TaskCompletion::Ready(Ok(value));
            StoredError::internal("task result kind mismatch")
        }),
        TaskCompletion::Ready(Err(err)) => Err(err),
        TaskCompletion::Canceled => {
            *state = TaskCompletion::Canceled;
            Err(StoredError::canceled())
        }
        other => {
            *state = other;
            Err(StoredError::internal("task is not ready"))
        }
    }
}

fn task_error_code_inner(task: *mut od_zig_task) -> od_zig_error_code {
    let Ok((_, inner)) = task_inner(task) else {
        return od_zig_error_code::OD_ZIG_INTERNAL;
    };
    let state = inner.state.lock().expect("poisoned");
    match &*state {
        TaskCompletion::Ready(Err(err)) => err.code,
        TaskCompletion::Canceled => od_zig_error_code::OD_ZIG_CANCELED,
        _ => od_zig_error_code::OD_ZIG_OK,
    }
}

async fn reader_to_futures(reader: Reader) -> Result<FuturesAsyncReader, StoredError> {
    reader
        .into_futures_async_read(..)
        .await
        .map_err(stored_from_opendal_error)
}

async fn into_async_reader_handle(runtime: Arc<RuntimeShared>, reader: Reader) -> Result<Box<od_zig_async_reader>, StoredError> {
    let futures_reader = reader_to_futures(reader).await?;
    Ok(Box::new(od_zig_async_reader {
        inner: ReaderHandleInner {
            runtime,
            inner: Arc::new(AsyncMutex::new(futures_reader)),
        },
    }))
}

async fn into_sync_reader_handle(runtime: Arc<RuntimeShared>, reader: Reader) -> Result<Box<od_zig_reader>, StoredError> {
    let futures_reader = reader_to_futures(reader).await?;
    Ok(Box::new(od_zig_reader {
        inner: ReaderHandleInner {
            runtime,
            inner: Arc::new(AsyncMutex::new(futures_reader)),
        },
    }))
}

fn into_sync_writer_handle(runtime: Arc<RuntimeShared>, writer: Writer) -> Box<od_zig_writer> {
    Box::new(od_zig_writer {
        inner: WriterHandleInner {
            runtime,
            inner: Arc::new(AsyncMutex::new(writer)),
        },
    })
}

fn into_async_writer_handle(runtime: Arc<RuntimeShared>, writer: Writer) -> Box<od_zig_async_writer> {
    Box::new(od_zig_async_writer {
        inner: WriterHandleInner {
            runtime,
            inner: Arc::new(AsyncMutex::new(writer)),
        },
    })
}

fn into_sync_lister_handle(runtime: Arc<RuntimeShared>, lister: Lister) -> Box<od_zig_lister> {
    Box::new(od_zig_lister {
        inner: SyncListerInner {
            runtime,
            inner: Arc::new(AsyncMutex::new(lister)),
            last_entry: Mutex::new(None),
        },
    })
}

fn into_async_lister_handle(runtime: Arc<RuntimeShared>, lister: Lister) -> Box<od_zig_async_lister> {
    Box::new(od_zig_async_lister {
        inner: AsyncListerInner {
            runtime,
            inner: Arc::new(AsyncMutex::new(lister)),
        },
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_runtime_new(options: od_zig_runtime_options) -> od_zig_result_runtime {
    match RuntimeShared::new(options) {
        Ok(runtime) => od_zig_result_runtime {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(Box::new(od_zig_runtime {
                inner: Arc::new(runtime),
            })),
        },
        Err(err) => od_zig_result_runtime {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_runtime_free(runtime: *mut od_zig_runtime) {
    if runtime.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(runtime));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_runtime_notifier_kind(
    runtime: *const od_zig_runtime,
) -> od_zig_notifier_kind {
    let Some(runtime) = (unsafe { runtime.as_ref() }) else {
        return od_zig_notifier_kind::OD_ZIG_NOTIFIER_NONE;
    };

    if runtime.inner.notifier.is_some() {
        od_zig_notifier_kind::OD_ZIG_NOTIFIER_PIPE_READ_FD
    } else {
        od_zig_notifier_kind::OD_ZIG_NOTIFIER_NONE
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_runtime_notifier_fd(runtime: *const od_zig_runtime) -> i32 {
    let Some(runtime) = (unsafe { runtime.as_ref() }) else {
        return -1;
    };

    runtime
        .inner
        .notifier
        .as_ref()
        .map(NotifierPipe::fd)
        .unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_runtime_notifier_handle(_runtime: *const od_zig_runtime) -> usize {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_runtime_drain_ready_tasks(
    runtime: *mut od_zig_runtime,
) -> od_zig_result_usize {
    let Some(runtime) = (unsafe { runtime.as_mut() }) else {
        return usize_result(od_zig_error_code::OD_ZIG_INVALID_ARGUMENT, 0);
    };

    let drained = runtime
        .inner
        .notifier
        .as_ref()
        .map(NotifierPipe::drain)
        .unwrap_or(0);
    usize_result(od_zig_error_code::OD_ZIG_OK, drained)
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_runtime_pop_ready_task(
    runtime: *mut od_zig_runtime,
) -> od_zig_result_optional_u64 {
    let Some(runtime) = (unsafe { runtime.as_mut() }) else {
        return od_zig_result_optional_u64 {
            code: od_zig_error_code::OD_ZIG_INVALID_ARGUMENT,
            has_value: false,
            value: 0,
        };
    };

    let popped = runtime.inner.ready_tasks.lock().expect("poisoned").pop_front();
    od_zig_result_optional_u64 {
        code: od_zig_error_code::OD_ZIG_OK,
        has_value: popped.is_some(),
        value: popped.unwrap_or_default(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_runtime_ack_ready_task(
    _runtime: *mut od_zig_runtime,
    _task_id: u64,
) -> od_zig_result_unit {
    unit_ok()
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_runtime_last_error_info(
    runtime: *mut od_zig_runtime,
) -> od_zig_result_error_info_optional {
    let Some(runtime) = (unsafe { runtime.as_mut() }) else {
        return od_zig_result_error_info_optional {
            code: od_zig_error_code::OD_ZIG_INVALID_ARGUMENT,
            has_value: false,
            value: ptr::null_mut(),
        };
    };

    let value = runtime.inner.last_error.lock().expect("poisoned").clone();
    match value {
        Some((code, message)) => od_zig_result_error_info_optional {
            code: od_zig_error_code::OD_ZIG_OK,
            has_value: true,
            value: Box::into_raw(build_error_info(code, message)),
        },
        None => od_zig_result_error_info_optional {
            code: od_zig_error_code::OD_ZIG_OK,
            has_value: false,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_new(
    scheme: od_zig_slice,
    options_ptr: *const od_zig_option,
    options_len: usize,
) -> od_zig_result_operator {
    let runtime = match RuntimeShared::new(empty_runtime_options()) {
        Ok(runtime) => Arc::new(runtime),
        Err(err) => {
            return od_zig_result_operator {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    match create_operator(runtime, scheme, options_ptr, options_len, false) {
        Ok(operator) => od_zig_result_operator {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(operator),
        },
        Err(err) => od_zig_result_operator {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_new_with_runtime(
    runtime: *mut od_zig_runtime,
    scheme: od_zig_slice,
    options_ptr: *const od_zig_option,
    options_len: usize,
) -> od_zig_result_operator {
    let Some(runtime) = (unsafe { runtime.as_mut() }) else {
        return od_zig_result_operator {
            code: od_zig_error_code::OD_ZIG_INVALID_ARGUMENT,
            value: ptr::null_mut(),
        };
    };

    match create_operator(runtime.inner.clone(), scheme, options_ptr, options_len, true) {
        Ok(operator) => od_zig_result_operator {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(operator),
        },
        Err(err) => od_zig_result_operator {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_free(operator: *mut od_zig_operator) {
    if operator.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(operator));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_check(operator: *mut od_zig_operator) -> od_zig_result_unit {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return od_zig_result_unit { code: err.code },
    };
    let runtime = operator.runtime.clone();
    let op = operator.op.clone();
    match runtime.map_and_record(runtime.handle.block_on(async move {
        op.check().await.map_err(stored_from_opendal_error)
    })) {
        Ok(_) => unit_ok(),
        Err(err) => od_zig_result_unit { code: err.code },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_exists(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
) -> od_zig_result_bool {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return bool_result(err.code, false),
    };
    let runtime = operator.runtime.clone();
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(path) => path,
        Err(err) => return bool_result(err.code, false),
    };
    match runtime.map_and_record(runtime.handle.block_on(async move {
        op.exists(&path).await.map_err(stored_from_opendal_error)
    })) {
        Ok(value) => bool_result(od_zig_error_code::OD_ZIG_OK, value),
        Err(err) => bool_result(err.code, false),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_create_dir(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
) -> od_zig_result_unit {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return od_zig_result_unit { code: err.code },
    };
    let runtime = operator.runtime.clone();
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(path) => path,
        Err(err) => return od_zig_result_unit { code: err.code },
    };
    match runtime.map_and_record(runtime.handle.block_on(async move {
        op.create_dir(&path).await.map_err(stored_from_opendal_error)
    })) {
        Ok(_) => unit_ok(),
        Err(err) => od_zig_result_unit { code: err.code },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_delete(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    options: od_zig_delete_options,
) -> od_zig_result_unit {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return od_zig_result_unit { code: err.code },
    };
    let runtime = operator.runtime.clone();
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(path) => path,
        Err(err) => return od_zig_result_unit { code: err.code },
    };
    let options = match convert_delete_options(options) {
        Ok(options) => options,
        Err(err) => return od_zig_result_unit { code: err.code },
    };
    match runtime.map_and_record(runtime.handle.block_on(async move {
        op.delete_options(&path, options).await.map_err(stored_from_opendal_error)
    })) {
        Ok(_) => unit_ok(),
        Err(err) => od_zig_result_unit { code: err.code },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_rename(
    operator: *mut od_zig_operator,
    from: od_zig_slice,
    to: od_zig_slice,
) -> od_zig_result_unit {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return od_zig_result_unit { code: err.code },
    };
    let runtime = operator.runtime.clone();
    let op = operator.op.clone();
    let from = match clone_string(from) {
        Ok(value) => value,
        Err(err) => return od_zig_result_unit { code: err.code },
    };
    let to = match clone_string(to) {
        Ok(value) => value,
        Err(err) => return od_zig_result_unit { code: err.code },
    };
    match runtime.map_and_record(runtime.handle.block_on(async move {
        op.rename(&from, &to).await.map_err(stored_from_opendal_error)
    })) {
        Ok(_) => unit_ok(),
        Err(err) => od_zig_result_unit { code: err.code },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_copy(
    operator: *mut od_zig_operator,
    from: od_zig_slice,
    to: od_zig_slice,
    options: od_zig_copy_options,
) -> od_zig_result_unit {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return od_zig_result_unit { code: err.code },
    };
    let runtime = operator.runtime.clone();
    let op = operator.op.clone();
    let from = match clone_string(from) {
        Ok(value) => value,
        Err(err) => return od_zig_result_unit { code: err.code },
    };
    let to = match clone_string(to) {
        Ok(value) => value,
        Err(err) => return od_zig_result_unit { code: err.code },
    };
    match runtime.map_and_record(runtime.handle.block_on(async move {
        op.copy_with(&from, &to)
            .if_not_exists(options.if_not_exists)
            .await
            .map_err(stored_from_opendal_error)
    })) {
        Ok(_) => unit_ok(),
        Err(err) => od_zig_result_unit { code: err.code },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_write(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    data: od_zig_slice,
    options: od_zig_write_options,
) -> od_zig_result_unit {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return od_zig_result_unit { code: err.code },
    };
    let runtime = operator.runtime.clone();
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => return od_zig_result_unit { code: err.code },
    };
    let options = match convert_write_options(options) {
        Ok(value) => value,
        Err(err) => return od_zig_result_unit { code: err.code },
    };
    let data = clone_bytes(data);
    match runtime.map_and_record(runtime.handle.block_on(async move {
        op.write_options(&path, data, options)
            .await
            .map(|_| ())
            .map_err(stored_from_opendal_error)
    })) {
        Ok(_) => unit_ok(),
        Err(err) => od_zig_result_unit { code: err.code },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_read_bytes(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    options: od_zig_read_options,
) -> od_zig_result_bytes {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => {
            return od_zig_result_bytes {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    let runtime = operator.runtime.clone();
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_bytes {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    let options = match convert_read_options(options) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_bytes {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    match runtime.map_and_record(runtime.handle.block_on(async move {
        op.read_options(&path, options)
            .await
            .map(build_bytes)
            .map_err(stored_from_opendal_error)
    })) {
        Ok(bytes) => od_zig_result_bytes {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(bytes),
        },
        Err(err) => od_zig_result_bytes {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_stat(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    options: od_zig_stat_options,
) -> od_zig_result_metadata {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => {
            return od_zig_result_metadata {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    let runtime = operator.runtime.clone();
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_metadata {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    let options = match convert_stat_options(options) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_metadata {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    match runtime.map_and_record(runtime.handle.block_on(async move {
        op.stat_options(&path, options)
            .await
            .map(build_metadata)
            .map_err(stored_from_opendal_error)
    })) {
        Ok(metadata) => od_zig_result_metadata {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(metadata),
        },
        Err(err) => od_zig_result_metadata {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_info_get(operator: *mut od_zig_operator) -> od_zig_result_info {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => {
            return od_zig_result_info {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    operator.runtime.clear_error();
    od_zig_result_info {
        code: od_zig_error_code::OD_ZIG_OK,
        value: Box::into_raw(build_info(operator.op.info())),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_reader_open(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    options: od_zig_read_options,
) -> od_zig_result_reader {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => {
            return od_zig_result_reader {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    let runtime = operator.runtime.clone();
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_reader {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    let options = match convert_reader_options(options) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_reader {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    let runtime_for_handle = runtime.clone();
    match runtime.map_and_record(runtime.handle.block_on(async move {
        let reader = op.reader_options(&path, options).await.map_err(stored_from_opendal_error)?;
        into_sync_reader_handle(runtime_for_handle, reader).await
    })) {
        Ok(reader) => od_zig_result_reader {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(reader),
        },
        Err(err) => od_zig_result_reader {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_writer_open(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    options: od_zig_write_options,
) -> od_zig_result_writer {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => {
            return od_zig_result_writer {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    let runtime = operator.runtime.clone();
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_writer {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    let options = match convert_write_options(options) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_writer {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    let runtime_for_handle = runtime.clone();
    match runtime.map_and_record(runtime.handle.block_on(async move {
        let writer = op.writer_options(&path, options).await.map_err(stored_from_opendal_error)?;
        Ok(into_sync_writer_handle(runtime_for_handle, writer))
    })) {
        Ok(writer) => od_zig_result_writer {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(writer),
        },
        Err(err) => od_zig_result_writer {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_lister_open(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    options: od_zig_list_options,
) -> od_zig_result_lister {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => {
            return od_zig_result_lister {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    let runtime = operator.runtime.clone();
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_lister {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    let options = match convert_list_options(options) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_lister {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    let runtime_for_handle = runtime.clone();
    match runtime.map_and_record(runtime.handle.block_on(async move {
        let lister = op.lister_options(&path, options).await.map_err(stored_from_opendal_error)?;
        Ok(into_sync_lister_handle(runtime_for_handle, lister))
    })) {
        Ok(lister) => od_zig_result_lister {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(lister),
        },
        Err(err) => od_zig_result_lister {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

fn presign_common_read(
    operator: &od_zig_operator,
    path: od_zig_slice,
    expire_secs: u64,
    options: od_zig_presign_options,
) -> od_zig_result_presigned_request {
    let runtime = operator.runtime.clone();
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_presigned_request {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    let options = match convert_presign_read_options(options) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_presigned_request {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    match runtime.map_and_record(runtime.handle.block_on(async move {
        op.presign_read_options(&path, Duration::from_secs(expire_secs), options)
            .await
            .map(build_presigned_request)
            .map_err(stored_from_opendal_error)
    })) {
        Ok(request) => od_zig_result_presigned_request {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(request),
        },
        Err(err) => od_zig_result_presigned_request {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

fn presign_common_write(
    operator: &od_zig_operator,
    path: od_zig_slice,
    expire_secs: u64,
    options: od_zig_presign_options,
) -> od_zig_result_presigned_request {
    let runtime = operator.runtime.clone();
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_presigned_request {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    let options = match convert_presign_write_options(options) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_presigned_request {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    match runtime.map_and_record(runtime.handle.block_on(async move {
        op.presign_write_options(&path, Duration::from_secs(expire_secs), options)
            .await
            .map(build_presigned_request)
            .map_err(stored_from_opendal_error)
    })) {
        Ok(request) => od_zig_result_presigned_request {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(request),
        },
        Err(err) => od_zig_result_presigned_request {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

fn presign_common_stat(
    operator: &od_zig_operator,
    path: od_zig_slice,
    expire_secs: u64,
    options: od_zig_presign_options,
) -> od_zig_result_presigned_request {
    let runtime = operator.runtime.clone();
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_presigned_request {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    let options = match convert_presign_stat_options(options) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_presigned_request {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    match runtime.map_and_record(runtime.handle.block_on(async move {
        op.presign_stat_options(&path, Duration::from_secs(expire_secs), options)
            .await
            .map(build_presigned_request)
            .map_err(stored_from_opendal_error)
    })) {
        Ok(request) => od_zig_result_presigned_request {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(request),
        },
        Err(err) => od_zig_result_presigned_request {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

fn presign_common_delete(
    operator: &od_zig_operator,
    path: od_zig_slice,
    expire_secs: u64,
    options: od_zig_presign_options,
) -> od_zig_result_presigned_request {
    let runtime = operator.runtime.clone();
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_presigned_request {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    let options = match convert_presign_delete_options(options) {
        Ok(value) => value,
        Err(err) => {
            return od_zig_result_presigned_request {
                code: err.code,
                value: ptr::null_mut(),
            };
        }
    };
    match runtime.map_and_record(runtime.handle.block_on(async move {
        op.presign_delete_options(&path, Duration::from_secs(expire_secs), options)
            .await
            .map(build_presigned_request)
            .map_err(stored_from_opendal_error)
    })) {
        Ok(request) => od_zig_result_presigned_request {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(request),
        },
        Err(err) => od_zig_result_presigned_request {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_presign_read(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    expire_secs: u64,
    options: od_zig_presign_options,
) -> od_zig_result_presigned_request {
    match operator_runtime(operator) {
        Ok(operator) => presign_common_read(operator, path, expire_secs, options),
        Err(err) => od_zig_result_presigned_request {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_presign_write(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    expire_secs: u64,
    options: od_zig_presign_options,
) -> od_zig_result_presigned_request {
    match operator_runtime(operator) {
        Ok(operator) => presign_common_write(operator, path, expire_secs, options),
        Err(err) => od_zig_result_presigned_request {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_presign_stat(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    expire_secs: u64,
    options: od_zig_presign_options,
) -> od_zig_result_presigned_request {
    match operator_runtime(operator) {
        Ok(operator) => presign_common_stat(operator, path, expire_secs, options),
        Err(err) => od_zig_result_presigned_request {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_presign_delete(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    expire_secs: u64,
    options: od_zig_presign_options,
) -> od_zig_result_presigned_request {
    match operator_runtime(operator) {
        Ok(operator) => presign_common_delete(operator, path, expire_secs, options),
        Err(err) => od_zig_result_presigned_request {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_reader_read(
    reader: *mut od_zig_reader,
    buf: od_zig_mut_slice,
) -> od_zig_result_usize {
    let Some(reader) = (unsafe { reader.as_mut() }) else {
        return usize_result(od_zig_error_code::OD_ZIG_INVALID_ARGUMENT, 0);
    };
    let runtime = reader.inner.runtime.clone();
    let inner = reader.inner.inner.clone();
    let buf = unsafe { slice_from_ffi_mut(buf) };
    match runtime.map_and_record(runtime.handle.block_on(async move {
        let mut guard = inner.lock().await;
        guard.read(buf).await.map_err(stored_from_io_error)
    })) {
        Ok(size) => usize_result(od_zig_error_code::OD_ZIG_OK, size),
        Err(err) => usize_result(err.code, 0),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_reader_seek_to(
    reader: *mut od_zig_reader,
    pos: u64,
) -> od_zig_result_u64 {
    let Some(reader) = (unsafe { reader.as_mut() }) else {
        return u64_result(od_zig_error_code::OD_ZIG_INVALID_ARGUMENT, 0);
    };
    let runtime = reader.inner.runtime.clone();
    let inner = reader.inner.inner.clone();
    match runtime.map_and_record(runtime.handle.block_on(async move {
        let mut guard = inner.lock().await;
        guard.seek(SeekFrom::Start(pos)).await.map_err(stored_from_io_error)
    })) {
        Ok(position) => u64_result(od_zig_error_code::OD_ZIG_OK, position),
        Err(err) => u64_result(err.code, 0),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_reader_seek_by(
    reader: *mut od_zig_reader,
    delta: i64,
) -> od_zig_result_u64 {
    let Some(reader) = (unsafe { reader.as_mut() }) else {
        return u64_result(od_zig_error_code::OD_ZIG_INVALID_ARGUMENT, 0);
    };
    let runtime = reader.inner.runtime.clone();
    let inner = reader.inner.inner.clone();
    match runtime.map_and_record(runtime.handle.block_on(async move {
        let mut guard = inner.lock().await;
        guard.seek(SeekFrom::Current(delta)).await.map_err(stored_from_io_error)
    })) {
        Ok(position) => u64_result(od_zig_error_code::OD_ZIG_OK, position),
        Err(err) => u64_result(err.code, 0),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_reader_seek_from_end(
    reader: *mut od_zig_reader,
    delta: i64,
) -> od_zig_result_u64 {
    let Some(reader) = (unsafe { reader.as_mut() }) else {
        return u64_result(od_zig_error_code::OD_ZIG_INVALID_ARGUMENT, 0);
    };
    let runtime = reader.inner.runtime.clone();
    let inner = reader.inner.inner.clone();
    match runtime.map_and_record(runtime.handle.block_on(async move {
        let mut guard = inner.lock().await;
        guard.seek(SeekFrom::End(delta)).await.map_err(stored_from_io_error)
    })) {
        Ok(position) => u64_result(od_zig_error_code::OD_ZIG_OK, position),
        Err(err) => u64_result(err.code, 0),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_reader_free(reader: *mut od_zig_reader) {
    if reader.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(reader));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_writer_write(
    writer: *mut od_zig_writer,
    data: od_zig_slice,
) -> od_zig_result_usize {
    let Some(writer) = (unsafe { writer.as_mut() }) else {
        return usize_result(od_zig_error_code::OD_ZIG_INVALID_ARGUMENT, 0);
    };
    let runtime = writer.inner.runtime.clone();
    let inner = writer.inner.inner.clone();
    let data_vec = clone_bytes(data);
    let len = data_vec.len();
    match runtime.map_and_record(runtime.handle.block_on(async move {
        let mut guard = inner.lock().await;
        guard.write(data_vec).await.map(|_| len).map_err(stored_from_opendal_error)
    })) {
        Ok(size) => usize_result(od_zig_error_code::OD_ZIG_OK, size),
        Err(err) => usize_result(err.code, 0),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_writer_close(writer: *mut od_zig_writer) -> od_zig_result_unit {
    let Some(writer) = (unsafe { writer.as_mut() }) else {
        return od_zig_result_unit {
            code: od_zig_error_code::OD_ZIG_INVALID_ARGUMENT,
        };
    };
    let runtime = writer.inner.runtime.clone();
    let inner = writer.inner.inner.clone();
    match runtime.map_and_record(runtime.handle.block_on(async move {
        let mut guard = inner.lock().await;
        guard.close().await.map(|_| ()).map_err(stored_from_opendal_error)
    })) {
        Ok(_) => unit_ok(),
        Err(err) => od_zig_result_unit { code: err.code },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_writer_free(writer: *mut od_zig_writer) {
    if writer.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(writer));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_lister_next_view(
    lister: *mut od_zig_lister,
) -> od_zig_result_entry_view {
    let Some(lister) = (unsafe { lister.as_mut() }) else {
        return od_zig_result_entry_view {
            code: od_zig_error_code::OD_ZIG_INVALID_ARGUMENT,
            has_value: false,
            value: od_zig_entry_view::default(),
        };
    };

    let runtime = lister.inner.runtime.clone();
    let inner = lister.inner.inner.clone();
    let result = runtime.map_and_record(runtime.handle.block_on(async move {
        let mut guard = inner.lock().await;
        guard.try_next().await.map_err(stored_from_opendal_error)
    }));

    match result {
        Ok(Some(entry)) => {
            let owned = build_entry(entry);
            let view = od_zig_entry_view {
                path: owned.path,
                name: owned.name,
                metadata: build_metadata_view(&owned.metadata),
            };
            *lister.inner.last_entry.lock().expect("poisoned") = Some(owned);
            od_zig_result_entry_view {
                code: od_zig_error_code::OD_ZIG_OK,
                has_value: true,
                value: view,
            }
        }
        Ok(None) => od_zig_result_entry_view {
            code: od_zig_error_code::OD_ZIG_OK,
            has_value: false,
            value: od_zig_entry_view::default(),
        },
        Err(err) => od_zig_result_entry_view {
            code: err.code,
            has_value: false,
            value: od_zig_entry_view::default(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_lister_next_owned(
    lister: *mut od_zig_lister,
) -> od_zig_result_entry_owned {
    let Some(lister) = (unsafe { lister.as_mut() }) else {
        return od_zig_result_entry_owned {
            code: od_zig_error_code::OD_ZIG_INVALID_ARGUMENT,
            has_value: false,
            value: ptr::null_mut(),
        };
    };

    let runtime = lister.inner.runtime.clone();
    let inner = lister.inner.inner.clone();
    match runtime.map_and_record(runtime.handle.block_on(async move {
        let mut guard = inner.lock().await;
        guard.try_next().await.map_err(stored_from_opendal_error)
    })) {
        Ok(Some(entry)) => od_zig_result_entry_owned {
            code: od_zig_error_code::OD_ZIG_OK,
            has_value: true,
            value: Box::into_raw(build_entry(entry)),
        },
        Ok(None) => od_zig_result_entry_owned {
            code: od_zig_error_code::OD_ZIG_OK,
            has_value: false,
            value: ptr::null_mut(),
        },
        Err(err) => od_zig_result_entry_owned {
            code: err.code,
            has_value: false,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_lister_free(lister: *mut od_zig_lister) {
    if lister.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(lister));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_bytes_len(bytes: *const od_zig_bytes) -> usize {
    unsafe { bytes.as_ref() }.map(|value| value.bytes.len()).unwrap_or_default()
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_bytes_ptr(bytes: *const od_zig_bytes) -> *const u8 {
    unsafe { bytes.as_ref() }
        .map(|value| value.bytes.as_ptr())
        .unwrap_or(ptr::null())
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_bytes_free(bytes: *mut od_zig_bytes) {
    if bytes.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(bytes));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_metadata_free(metadata: *mut od_zig_metadata) {
    if metadata.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(metadata));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_info_free(info: *mut od_zig_operator_info) {
    if info.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(info));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_presigned_request_free(request: *mut od_zig_presigned_request) {
    if request.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(request));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_entry_owned_free(entry: *mut od_zig_entry_owned) {
    if entry.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(entry));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_error_info_free(error_info: *mut od_zig_error_info) {
    if error_info.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(error_info));
    }
}

fn task_result_error(code: od_zig_error_code) -> od_zig_result_task {
    od_zig_result_task {
        code,
        value: ptr::null_mut(),
    }
}

macro_rules! async_operator_start {
    ($name:ident, $body:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(operator: *mut od_zig_operator) -> od_zig_result_task {
            let operator = match operator_runtime(operator) {
                Ok(operator) => operator,
                Err(err) => return task_result_error(err.code),
            };
            let runtime = match async_runtime_for_operator(operator) {
                Ok(runtime) => runtime,
                Err(err) => {
                    operator.runtime.record_error(&err);
                    return task_result_error(err.code);
                }
            };
            let op = operator.op.clone();
            od_zig_result_task {
                code: od_zig_error_code::OD_ZIG_OK,
                value: spawn_task(runtime, $body(op)),
            }
        }
    };
}

async_operator_start!(od_zig_operator_check_start, |op: Operator| async move {
    op.check()
        .await
        .map(|_| TaskValue::Unit)
        .map_err(stored_from_opendal_error)
});

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_exists_start(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
) -> od_zig_result_task {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return task_result_error(err.code),
    };
    let runtime = match async_runtime_for_operator(operator) {
        Ok(runtime) => runtime,
        Err(err) => {
            operator.runtime.record_error(&err);
            return task_result_error(err.code);
        }
    };
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            op.exists(&path)
                .await
                .map(TaskValue::Bool)
                .map_err(stored_from_opendal_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_create_dir_start(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
) -> od_zig_result_task {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return task_result_error(err.code),
    };
    let runtime = match async_runtime_for_operator(operator) {
        Ok(runtime) => runtime,
        Err(err) => {
            operator.runtime.record_error(&err);
            return task_result_error(err.code);
        }
    };
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            op.create_dir(&path)
                .await
                .map(|_| TaskValue::Unit)
                .map_err(stored_from_opendal_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_delete_start(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    options: od_zig_delete_options,
) -> od_zig_result_task {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return task_result_error(err.code),
    };
    let runtime = match async_runtime_for_operator(operator) {
        Ok(runtime) => runtime,
        Err(err) => {
            operator.runtime.record_error(&err);
            return task_result_error(err.code);
        }
    };
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    let options = match convert_delete_options(options) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            op.delete_options(&path, options)
                .await
                .map(|_| TaskValue::Unit)
                .map_err(stored_from_opendal_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_rename_start(
    operator: *mut od_zig_operator,
    from: od_zig_slice,
    to: od_zig_slice,
) -> od_zig_result_task {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return task_result_error(err.code),
    };
    let runtime = match async_runtime_for_operator(operator) {
        Ok(runtime) => runtime,
        Err(err) => {
            operator.runtime.record_error(&err);
            return task_result_error(err.code);
        }
    };
    let op = operator.op.clone();
    let from = match clone_string(from) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    let to = match clone_string(to) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            op.rename(&from, &to)
                .await
                .map(|_| TaskValue::Unit)
                .map_err(stored_from_opendal_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_copy_start(
    operator: *mut od_zig_operator,
    from: od_zig_slice,
    to: od_zig_slice,
    options: od_zig_copy_options,
) -> od_zig_result_task {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return task_result_error(err.code),
    };
    let runtime = match async_runtime_for_operator(operator) {
        Ok(runtime) => runtime,
        Err(err) => {
            operator.runtime.record_error(&err);
            return task_result_error(err.code);
        }
    };
    let op = operator.op.clone();
    let from = match clone_string(from) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    let to = match clone_string(to) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            op.copy_with(&from, &to)
                .if_not_exists(options.if_not_exists)
                .await
                .map(|_| TaskValue::Unit)
                .map_err(stored_from_opendal_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_write_start(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    data: od_zig_slice,
    options: od_zig_write_options,
) -> od_zig_result_task {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return task_result_error(err.code),
    };
    let runtime = match async_runtime_for_operator(operator) {
        Ok(runtime) => runtime,
        Err(err) => {
            operator.runtime.record_error(&err);
            return task_result_error(err.code);
        }
    };
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    let data_vec = clone_bytes(data);
    let options = match convert_write_options(options) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            op.write_options(&path, data_vec, options)
                .await
                .map(|_| TaskValue::Unit)
                .map_err(stored_from_opendal_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_read_start(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    options: od_zig_read_options,
) -> od_zig_result_task {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return task_result_error(err.code),
    };
    let runtime = match async_runtime_for_operator(operator) {
        Ok(runtime) => runtime,
        Err(err) => {
            operator.runtime.record_error(&err);
            return task_result_error(err.code);
        }
    };
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    let options = match convert_read_options(options) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            op.read_options(&path, options)
                .await
                .map(build_bytes)
                .map(TaskValue::Bytes)
                .map_err(stored_from_opendal_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_stat_start(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    options: od_zig_stat_options,
) -> od_zig_result_task {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return task_result_error(err.code),
    };
    let runtime = match async_runtime_for_operator(operator) {
        Ok(runtime) => runtime,
        Err(err) => {
            operator.runtime.record_error(&err);
            return task_result_error(err.code);
        }
    };
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    let options = match convert_stat_options(options) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            op.stat_options(&path, options)
                .await
                .map(build_metadata)
                .map(TaskValue::Metadata)
                .map_err(stored_from_opendal_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_info_start(
    operator: *mut od_zig_operator,
) -> od_zig_result_task {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return task_result_error(err.code),
    };
    let runtime = match async_runtime_for_operator(operator) {
        Ok(runtime) => runtime,
        Err(err) => {
            operator.runtime.record_error(&err);
            return task_result_error(err.code);
        }
    };
    let info = operator.op.info();
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move { Ok(TaskValue::Info(build_info(info))) }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_presign_read_start(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    expire_secs: u64,
    options: od_zig_presign_options,
) -> od_zig_result_task {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return task_result_error(err.code),
    };
    let runtime = match async_runtime_for_operator(operator) {
        Ok(runtime) => runtime,
        Err(err) => {
            operator.runtime.record_error(&err);
            return task_result_error(err.code);
        }
    };
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    let options = match convert_presign_read_options(options) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            op.presign_read_options(&path, Duration::from_secs(expire_secs), options)
                .await
                .map(build_presigned_request)
                .map(TaskValue::PresignedRequest)
                .map_err(stored_from_opendal_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_presign_write_start(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    expire_secs: u64,
    options: od_zig_presign_options,
) -> od_zig_result_task {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return task_result_error(err.code),
    };
    let runtime = match async_runtime_for_operator(operator) {
        Ok(runtime) => runtime,
        Err(err) => {
            operator.runtime.record_error(&err);
            return task_result_error(err.code);
        }
    };
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    let options = match convert_presign_write_options(options) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            op.presign_write_options(&path, Duration::from_secs(expire_secs), options)
                .await
                .map(build_presigned_request)
                .map(TaskValue::PresignedRequest)
                .map_err(stored_from_opendal_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_presign_stat_start(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    expire_secs: u64,
    options: od_zig_presign_options,
) -> od_zig_result_task {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return task_result_error(err.code),
    };
    let runtime = match async_runtime_for_operator(operator) {
        Ok(runtime) => runtime,
        Err(err) => {
            operator.runtime.record_error(&err);
            return task_result_error(err.code);
        }
    };
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    let options = match convert_presign_stat_options(options) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            op.presign_stat_options(&path, Duration::from_secs(expire_secs), options)
                .await
                .map(build_presigned_request)
                .map(TaskValue::PresignedRequest)
                .map_err(stored_from_opendal_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_presign_delete_start(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    expire_secs: u64,
    options: od_zig_presign_options,
) -> od_zig_result_task {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return task_result_error(err.code),
    };
    let runtime = match async_runtime_for_operator(operator) {
        Ok(runtime) => runtime,
        Err(err) => {
            operator.runtime.record_error(&err);
            return task_result_error(err.code);
        }
    };
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    let options = match convert_presign_delete_options(options) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            op.presign_delete_options(&path, Duration::from_secs(expire_secs), options)
                .await
                .map(build_presigned_request)
                .map(TaskValue::PresignedRequest)
                .map_err(stored_from_opendal_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_reader_open_start(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    options: od_zig_read_options,
) -> od_zig_result_task {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return task_result_error(err.code),
    };
    let runtime = match async_runtime_for_operator(operator) {
        Ok(runtime) => runtime,
        Err(err) => {
            operator.runtime.record_error(&err);
            return task_result_error(err.code);
        }
    };
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    let options = match convert_reader_options(options) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    let runtime_clone = runtime.clone();
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            let reader = op.reader_options(&path, options).await.map_err(stored_from_opendal_error)?;
            let reader = into_async_reader_handle(runtime_clone, reader).await?;
            Ok(TaskValue::Reader(reader))
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_writer_open_start(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    options: od_zig_write_options,
) -> od_zig_result_task {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return task_result_error(err.code),
    };
    let runtime = match async_runtime_for_operator(operator) {
        Ok(runtime) => runtime,
        Err(err) => {
            operator.runtime.record_error(&err);
            return task_result_error(err.code);
        }
    };
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    let options = match convert_write_options(options) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    let runtime_clone = runtime.clone();
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            let writer = op.writer_options(&path, options).await.map_err(stored_from_opendal_error)?;
            Ok(TaskValue::Writer(into_async_writer_handle(runtime_clone, writer)))
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_operator_lister_open_start(
    operator: *mut od_zig_operator,
    path: od_zig_slice,
    options: od_zig_list_options,
) -> od_zig_result_task {
    let operator = match operator_runtime(operator) {
        Ok(operator) => operator,
        Err(err) => return task_result_error(err.code),
    };
    let runtime = match async_runtime_for_operator(operator) {
        Ok(runtime) => runtime,
        Err(err) => {
            operator.runtime.record_error(&err);
            return task_result_error(err.code);
        }
    };
    let op = operator.op.clone();
    let path = match clone_string(path) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    let options = match convert_list_options(options) {
        Ok(value) => value,
        Err(err) => return task_result_error(err.code),
    };
    let runtime_clone = runtime.clone();
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            let lister = op.lister_options(&path, options).await.map_err(stored_from_opendal_error)?;
            Ok(TaskValue::Lister(into_async_lister_handle(runtime_clone, lister)))
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_reader_read_start(
    reader: *mut od_zig_async_reader,
    buf: od_zig_mut_slice,
) -> od_zig_result_task {
    let Some(reader) = (unsafe { reader.as_mut() }) else {
        return task_result_error(od_zig_error_code::OD_ZIG_INVALID_ARGUMENT);
    };
    let runtime = reader.inner.runtime.clone();
    let inner = reader.inner.inner.clone();
    let ptr = buf.ptr as usize;
    let len = buf.len;
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            let mut guard = inner.lock().await;
            let buffer = unsafe { slice::from_raw_parts_mut(ptr as *mut u8, len) };
            guard.read(buffer)
                .await
                .map(TaskValue::Usize)
                .map_err(stored_from_io_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_reader_seek_to_start(
    reader: *mut od_zig_async_reader,
    pos: u64,
) -> od_zig_result_task {
    let Some(reader) = (unsafe { reader.as_mut() }) else {
        return task_result_error(od_zig_error_code::OD_ZIG_INVALID_ARGUMENT);
    };
    let runtime = reader.inner.runtime.clone();
    let inner = reader.inner.inner.clone();
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            let mut guard = inner.lock().await;
            guard.seek(SeekFrom::Start(pos))
                .await
                .map(TaskValue::U64)
                .map_err(stored_from_io_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_reader_seek_by_start(
    reader: *mut od_zig_async_reader,
    delta: i64,
) -> od_zig_result_task {
    let Some(reader) = (unsafe { reader.as_mut() }) else {
        return task_result_error(od_zig_error_code::OD_ZIG_INVALID_ARGUMENT);
    };
    let runtime = reader.inner.runtime.clone();
    let inner = reader.inner.inner.clone();
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            let mut guard = inner.lock().await;
            guard.seek(SeekFrom::Current(delta))
                .await
                .map(TaskValue::U64)
                .map_err(stored_from_io_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_reader_seek_from_end_start(
    reader: *mut od_zig_async_reader,
    delta: i64,
) -> od_zig_result_task {
    let Some(reader) = (unsafe { reader.as_mut() }) else {
        return task_result_error(od_zig_error_code::OD_ZIG_INVALID_ARGUMENT);
    };
    let runtime = reader.inner.runtime.clone();
    let inner = reader.inner.inner.clone();
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            let mut guard = inner.lock().await;
            guard.seek(SeekFrom::End(delta))
                .await
                .map(TaskValue::U64)
                .map_err(stored_from_io_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_writer_write_start(
    writer: *mut od_zig_async_writer,
    data: od_zig_slice,
) -> od_zig_result_task {
    let Some(writer) = (unsafe { writer.as_mut() }) else {
        return task_result_error(od_zig_error_code::OD_ZIG_INVALID_ARGUMENT);
    };
    let runtime = writer.inner.runtime.clone();
    let inner = writer.inner.inner.clone();
    let data_vec = clone_bytes(data);
    let len = data_vec.len();
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            let mut guard = inner.lock().await;
            guard.write(data_vec)
                .await
                .map(|_| TaskValue::Usize(len))
                .map_err(stored_from_opendal_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_writer_close_start(
    writer: *mut od_zig_async_writer,
) -> od_zig_result_task {
    let Some(writer) = (unsafe { writer.as_mut() }) else {
        return task_result_error(od_zig_error_code::OD_ZIG_INVALID_ARGUMENT);
    };
    let runtime = writer.inner.runtime.clone();
    let inner = writer.inner.inner.clone();
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            let mut guard = inner.lock().await;
            guard.close()
                .await
                .map(|_| TaskValue::Unit)
                .map_err(stored_from_opendal_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_lister_next_owned_start(
    lister: *mut od_zig_async_lister,
) -> od_zig_result_task {
    let Some(lister) = (unsafe { lister.as_mut() }) else {
        return task_result_error(od_zig_error_code::OD_ZIG_INVALID_ARGUMENT);
    };
    let runtime = lister.inner.runtime.clone();
    let inner = lister.inner.inner.clone();
    od_zig_result_task {
        code: od_zig_error_code::OD_ZIG_OK,
        value: spawn_task(runtime, async move {
            let mut guard = inner.lock().await;
            guard.try_next()
                .await
                .map(|value| TaskValue::Entry(value.map(build_entry)))
                .map_err(stored_from_opendal_error)
        }),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_async_reader_free(reader: *mut od_zig_async_reader) {
    if reader.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(reader));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_async_writer_free(writer: *mut od_zig_async_writer) {
    if writer.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(writer));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_async_lister_free(lister: *mut od_zig_async_lister) {
    if lister.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(lister));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_task_poll(task: *mut od_zig_task) -> od_zig_task_poll_result {
    let Ok((_, inner)) = task_inner(task) else {
        return od_zig_task_poll_result {
            code: od_zig_error_code::OD_ZIG_INVALID_ARGUMENT,
            state: od_zig_poll_state::OD_ZIG_POLL_PENDING,
        };
    };
    let state = inner.state.lock().expect("poisoned");
    let poll_state = match &*state {
        TaskCompletion::Pending | TaskCompletion::Canceling => od_zig_poll_state::OD_ZIG_POLL_PENDING,
        TaskCompletion::Ready(_) | TaskCompletion::Canceled | TaskCompletion::Consumed => {
            od_zig_poll_state::OD_ZIG_POLL_READY
        }
    };
    od_zig_task_poll_result {
        code: od_zig_error_code::OD_ZIG_OK,
        state: poll_state,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_task_cancel(task: *mut od_zig_task) -> od_zig_task_cancel_result {
    let Ok((task_ref, inner)) = task_inner(task) else {
        return od_zig_task_cancel_result {
            code: od_zig_error_code::OD_ZIG_INVALID_ARGUMENT,
            completed: false,
        };
    };

    {
        let mut state = inner.state.lock().expect("poisoned");
        match &*state {
            TaskCompletion::Ready(_) => {
                return od_zig_task_cancel_result {
                    code: od_zig_error_code::OD_ZIG_OK,
                    completed: true,
                };
            }
            TaskCompletion::Canceled => {
                return od_zig_task_cancel_result {
                    code: od_zig_error_code::OD_ZIG_OK,
                    completed: false,
                };
            }
            TaskCompletion::Consumed => {
                return od_zig_task_cancel_result {
                    code: od_zig_error_code::OD_ZIG_OK,
                    completed: false,
                };
            }
            TaskCompletion::Pending => {
                *state = TaskCompletion::Canceling;
            }
            TaskCompletion::Canceling => {}
        }
    }

    let join = inner.join.lock().expect("poisoned").take();
    if let Some(join) = join {
        join.abort();
        let _ = task_ref.runtime.handle.block_on(async { join.await });
    }

    let mut state = inner.state.lock().expect("poisoned");
    match &*state {
        TaskCompletion::Ready(_) => od_zig_task_cancel_result {
            code: od_zig_error_code::OD_ZIG_OK,
            completed: true,
        },
        TaskCompletion::Canceled => od_zig_task_cancel_result {
            code: od_zig_error_code::OD_ZIG_OK,
            completed: false,
        },
        TaskCompletion::Pending | TaskCompletion::Canceling => {
            *state = TaskCompletion::Canceled;
            od_zig_task_cancel_result {
                code: od_zig_error_code::OD_ZIG_OK,
                completed: false,
            }
        }
        TaskCompletion::Consumed => od_zig_task_cancel_result {
            code: od_zig_error_code::OD_ZIG_OK,
            completed: false,
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_task_get_state(task: *mut od_zig_task) -> od_zig_task_state_result {
    let Ok((_, inner)) = task_inner(task) else {
        return od_zig_task_state_result {
            code: od_zig_error_code::OD_ZIG_INVALID_ARGUMENT,
            state: od_zig_task_state::OD_ZIG_TASK_FAILED,
        };
    };
    let state = inner.state.lock().expect("poisoned");
    let value = match &*state {
        TaskCompletion::Pending | TaskCompletion::Canceling => od_zig_task_state::OD_ZIG_TASK_PENDING,
        TaskCompletion::Ready(Ok(_)) => od_zig_task_state::OD_ZIG_TASK_READY,
        TaskCompletion::Ready(Err(_)) => od_zig_task_state::OD_ZIG_TASK_FAILED,
        TaskCompletion::Canceled => od_zig_task_state::OD_ZIG_TASK_CANCELED,
        TaskCompletion::Consumed => od_zig_task_state::OD_ZIG_TASK_CONSUMED,
    };
    od_zig_task_state_result {
        code: od_zig_error_code::OD_ZIG_OK,
        state: value,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_task_error_code(task: *mut od_zig_task) -> od_zig_error_code {
    task_error_code_inner(task)
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_task_take_unit(task: *mut od_zig_task) -> od_zig_result_unit {
    match take_task_value(task, |value| match value {
        TaskValue::Unit => Ok(()),
        other => Err(other),
    }) {
        Ok(_) => unit_ok(),
        Err(err) => od_zig_result_unit { code: err.code },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_task_take_bool(task: *mut od_zig_task) -> od_zig_result_bool {
    match take_task_value(task, |value| match value {
        TaskValue::Bool(value) => Ok(value),
        other => Err(other),
    }) {
        Ok(value) => bool_result(od_zig_error_code::OD_ZIG_OK, value),
        Err(err) => bool_result(err.code, false),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_task_take_usize(task: *mut od_zig_task) -> od_zig_result_usize {
    match take_task_value(task, |value| match value {
        TaskValue::Usize(value) => Ok(value),
        other => Err(other),
    }) {
        Ok(value) => usize_result(od_zig_error_code::OD_ZIG_OK, value),
        Err(err) => usize_result(err.code, 0),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_task_take_u64(task: *mut od_zig_task) -> od_zig_result_u64 {
    match take_task_value(task, |value| match value {
        TaskValue::U64(value) => Ok(value),
        other => Err(other),
    }) {
        Ok(value) => u64_result(od_zig_error_code::OD_ZIG_OK, value),
        Err(err) => u64_result(err.code, 0),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_task_take_bytes(task: *mut od_zig_task) -> od_zig_result_bytes {
    match take_task_value(task, |value| match value {
        TaskValue::Bytes(value) => Ok(value),
        other => Err(other),
    }) {
        Ok(value) => od_zig_result_bytes {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(value),
        },
        Err(err) => od_zig_result_bytes {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_task_take_metadata(
    task: *mut od_zig_task,
) -> od_zig_result_metadata {
    match take_task_value(task, |value| match value {
        TaskValue::Metadata(value) => Ok(value),
        other => Err(other),
    }) {
        Ok(value) => od_zig_result_metadata {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(value),
        },
        Err(err) => od_zig_result_metadata {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_task_take_info(task: *mut od_zig_task) -> od_zig_result_info {
    match take_task_value(task, |value| match value {
        TaskValue::Info(value) => Ok(value),
        other => Err(other),
    }) {
        Ok(value) => od_zig_result_info {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(value),
        },
        Err(err) => od_zig_result_info {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_task_take_presigned_request(
    task: *mut od_zig_task,
) -> od_zig_result_presigned_request {
    match take_task_value(task, |value| match value {
        TaskValue::PresignedRequest(value) => Ok(value),
        other => Err(other),
    }) {
        Ok(value) => od_zig_result_presigned_request {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(value),
        },
        Err(err) => od_zig_result_presigned_request {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_task_take_entry(
    task: *mut od_zig_task,
) -> od_zig_result_entry_owned {
    match take_task_value(task, |value| match value {
        TaskValue::Entry(value) => Ok(value),
        other => Err(other),
    }) {
        Ok(Some(value)) => od_zig_result_entry_owned {
            code: od_zig_error_code::OD_ZIG_OK,
            has_value: true,
            value: Box::into_raw(value),
        },
        Ok(None) => od_zig_result_entry_owned {
            code: od_zig_error_code::OD_ZIG_OK,
            has_value: false,
            value: ptr::null_mut(),
        },
        Err(err) => od_zig_result_entry_owned {
            code: err.code,
            has_value: false,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_task_take_reader(
    task: *mut od_zig_task,
) -> od_zig_result_async_reader {
    match take_task_value(task, |value| match value {
        TaskValue::Reader(value) => Ok(value),
        other => Err(other),
    }) {
        Ok(value) => od_zig_result_async_reader {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(value),
        },
        Err(err) => od_zig_result_async_reader {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_task_take_writer(
    task: *mut od_zig_task,
) -> od_zig_result_async_writer {
    match take_task_value(task, |value| match value {
        TaskValue::Writer(value) => Ok(value),
        other => Err(other),
    }) {
        Ok(value) => od_zig_result_async_writer {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(value),
        },
        Err(err) => od_zig_result_async_writer {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_task_take_lister(
    task: *mut od_zig_task,
) -> od_zig_result_async_lister {
    match take_task_value(task, |value| match value {
        TaskValue::Lister(value) => Ok(value),
        other => Err(other),
    }) {
        Ok(value) => od_zig_result_async_lister {
            code: od_zig_error_code::OD_ZIG_OK,
            value: Box::into_raw(value),
        },
        Err(err) => od_zig_result_async_lister {
            code: err.code,
            value: ptr::null_mut(),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn od_zig_task_free(task: *mut od_zig_task) {
    let Some(task) = (unsafe { task.as_mut() }) else {
        return;
    };

    if let Some(inner) = task.runtime.tasks.lock().expect("poisoned").remove(&task.task_id) {
        if let Some(join) = inner.join.lock().expect("poisoned").take() {
            join.abort();
        }
    }

    unsafe {
        drop(Box::from_raw(task));
    }
}