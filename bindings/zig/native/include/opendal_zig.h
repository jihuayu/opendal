/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing,
 * software distributed under the License is distributed on an
 * "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
 * KIND, either express or implied.  See the License for the
 * specific language governing permissions and limitations
 * under the License.
 */

#ifndef OPENDAL_ZIG_H
#define OPENDAL_ZIG_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct od_zig_runtime od_zig_runtime;
typedef struct od_zig_operator od_zig_operator;
typedef struct od_zig_reader od_zig_reader;
typedef struct od_zig_writer od_zig_writer;
typedef struct od_zig_lister od_zig_lister;
typedef struct od_zig_async_reader od_zig_async_reader;
typedef struct od_zig_async_writer od_zig_async_writer;
typedef struct od_zig_async_lister od_zig_async_lister;
typedef struct od_zig_bytes od_zig_bytes;
typedef struct od_zig_task od_zig_task;

typedef enum od_zig_error_code {
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
} od_zig_error_code;

typedef enum od_zig_notifier_kind {
  OD_ZIG_NOTIFIER_NONE = 0,
  OD_ZIG_NOTIFIER_EVENTFD = 1,
  OD_ZIG_NOTIFIER_PIPE_READ_FD = 2,
  OD_ZIG_NOTIFIER_WIN_HANDLE = 3,
} od_zig_notifier_kind;

typedef enum od_zig_task_state {
  OD_ZIG_TASK_PENDING = 0,
  OD_ZIG_TASK_READY = 1,
  OD_ZIG_TASK_CONSUMED = 2,
  OD_ZIG_TASK_CANCELED = 3,
  OD_ZIG_TASK_FAILED = 4,
} od_zig_task_state;

typedef enum od_zig_poll_state {
  OD_ZIG_POLL_PENDING = 0,
  OD_ZIG_POLL_READY = 1,
} od_zig_poll_state;

typedef enum od_zig_entry_mode {
  OD_ZIG_ENTRY_MODE_FILE = 0,
  OD_ZIG_ENTRY_MODE_DIR = 1,
  OD_ZIG_ENTRY_MODE_UNKNOWN = 2,
} od_zig_entry_mode;

typedef struct od_zig_slice {
  const uint8_t *ptr;
  uintptr_t len;
} od_zig_slice;

typedef struct od_zig_mut_slice {
  uint8_t *ptr;
  uintptr_t len;
} od_zig_mut_slice;

typedef struct od_zig_option {
  od_zig_slice key;
  od_zig_slice value;
} od_zig_option;

typedef struct od_zig_runtime_options {
  bool has_worker_threads;
  uint16_t worker_threads;
  uintptr_t completion_queue_capacity;
  uint32_t cancellation_check_interval_ms;
} od_zig_runtime_options;

typedef struct od_zig_header {
  od_zig_slice key;
  od_zig_slice value;
} od_zig_header;

typedef od_zig_header od_zig_header_view;

typedef struct od_zig_capability {
  bool stat;
  bool stat_with_if_match;
  bool stat_with_if_none_match;
  bool stat_with_if_modified_since;
  bool stat_with_if_unmodified_since;
  bool stat_with_override_cache_control;
  bool stat_with_override_content_disposition;
  bool stat_with_override_content_type;
  bool stat_with_version;
  bool read;
  bool read_with_if_match;
  bool read_with_if_none_match;
  bool read_with_if_modified_since;
  bool read_with_if_unmodified_since;
  bool read_with_override_cache_control;
  bool read_with_override_content_disposition;
  bool read_with_override_content_type;
  bool read_with_version;
  bool write;
  bool write_can_multi;
  bool write_can_empty;
  bool write_can_append;
  bool write_with_content_type;
  bool write_with_content_disposition;
  bool write_with_content_encoding;
  bool write_with_cache_control;
  bool write_with_if_match;
  bool write_with_if_none_match;
  bool write_with_if_not_exists;
  bool write_with_user_metadata;
  bool has_write_multi_max_size;
  uintptr_t write_multi_max_size;
  bool has_write_multi_min_size;
  uintptr_t write_multi_min_size;
  bool has_write_total_max_size;
  uintptr_t write_total_max_size;
  bool create_dir;
  bool delete;
  bool delete_with_version;
  bool delete_with_recursive;
  bool has_delete_max_size;
  uintptr_t delete_max_size;
  bool copy;
  bool copy_with_if_not_exists;
  bool rename;
  bool list;
  bool list_with_limit;
  bool list_with_start_after;
  bool list_with_recursive;
  bool list_with_versions;
  bool list_with_deleted;
  bool presign;
  bool presign_read;
  bool presign_stat;
  bool presign_write;
  bool presign_delete;
  bool shared;
} od_zig_capability;

typedef struct od_zig_metadata {
  uint8_t mode;
  bool is_file;
  bool is_dir;
  bool is_deleted;
  bool has_content_length;
  uint64_t content_length;
  od_zig_slice content_type;
  od_zig_slice content_encoding;
  od_zig_slice content_disposition;
  od_zig_slice content_md5;
  od_zig_slice etag;
  od_zig_slice last_modified_rfc3339;
  od_zig_slice version;
  od_zig_header *user_metadata_ptr;
  uintptr_t user_metadata_len;
} od_zig_metadata;

typedef od_zig_metadata od_zig_metadata_view;

typedef struct od_zig_entry_view {
  od_zig_slice path;
  od_zig_slice name;
  od_zig_metadata_view metadata;
} od_zig_entry_view;

typedef struct od_zig_entry_owned {
  od_zig_slice path;
  od_zig_slice name;
  od_zig_metadata metadata;
} od_zig_entry_owned;

typedef struct od_zig_operator_info {
  od_zig_slice scheme;
  od_zig_slice root;
  od_zig_slice name;
  od_zig_capability full_capability;
  od_zig_capability native_capability;
} od_zig_operator_info;

typedef struct od_zig_presigned_request {
  od_zig_slice method;
  od_zig_slice url;
  od_zig_header *headers_ptr;
  uintptr_t headers_len;
} od_zig_presigned_request;

typedef struct od_zig_error_info {
  od_zig_error_code code;
  od_zig_slice message;
} od_zig_error_info;

typedef struct od_zig_read_options {
  od_zig_slice version;
  od_zig_slice if_match;
  od_zig_slice if_none_match;
  od_zig_slice if_modified_since_http_date;
  od_zig_slice if_unmodified_since_http_date;
  od_zig_slice override_content_type;
  od_zig_slice override_cache_control;
  od_zig_slice override_content_disposition;
  bool has_offset;
  uint64_t offset;
  bool has_size;
  uint64_t size;
} od_zig_read_options;

typedef struct od_zig_write_options {
  od_zig_slice content_type;
  od_zig_slice content_disposition;
  od_zig_slice content_encoding;
  od_zig_slice cache_control;
  od_zig_slice if_match;
  od_zig_slice if_none_match;
  bool if_not_exists;
  bool append;
  const od_zig_header *user_metadata_ptr;
  uintptr_t user_metadata_len;
} od_zig_write_options;

typedef struct od_zig_stat_options {
  od_zig_slice if_match;
  od_zig_slice if_none_match;
  od_zig_slice if_modified_since_http_date;
  od_zig_slice if_unmodified_since_http_date;
  od_zig_slice override_content_type;
  od_zig_slice override_cache_control;
  od_zig_slice override_content_disposition;
  od_zig_slice version;
} od_zig_stat_options;

typedef struct od_zig_delete_options {
  od_zig_slice version;
  bool recursive;
} od_zig_delete_options;

typedef struct od_zig_copy_options {
  bool if_not_exists;
} od_zig_copy_options;

typedef struct od_zig_list_options {
  bool recursive;
  bool has_limit;
  uintptr_t limit;
  od_zig_slice start_after;
  bool versions;
  bool deleted;
} od_zig_list_options;

typedef struct od_zig_presign_options {
  od_zig_slice version;
  od_zig_slice if_match;
  od_zig_slice if_none_match;
  od_zig_slice if_modified_since_http_date;
  od_zig_slice if_unmodified_since_http_date;
  od_zig_slice override_content_type;
  od_zig_slice override_cache_control;
  od_zig_slice override_content_disposition;
  od_zig_slice content_encoding;
  bool if_not_exists;
} od_zig_presign_options;

typedef struct od_zig_result_runtime {
  od_zig_error_code code;
  od_zig_runtime *value;
} od_zig_result_runtime;

typedef struct od_zig_result_operator {
  od_zig_error_code code;
  od_zig_operator *value;
} od_zig_result_operator;

typedef struct od_zig_result_reader {
  od_zig_error_code code;
  od_zig_reader *value;
} od_zig_result_reader;

typedef struct od_zig_result_writer {
  od_zig_error_code code;
  od_zig_writer *value;
} od_zig_result_writer;

typedef struct od_zig_result_lister {
  od_zig_error_code code;
  od_zig_lister *value;
} od_zig_result_lister;

typedef struct od_zig_result_async_reader {
  od_zig_error_code code;
  od_zig_async_reader *value;
} od_zig_result_async_reader;

typedef struct od_zig_result_async_writer {
  od_zig_error_code code;
  od_zig_async_writer *value;
} od_zig_result_async_writer;

typedef struct od_zig_result_async_lister {
  od_zig_error_code code;
  od_zig_async_lister *value;
} od_zig_result_async_lister;

typedef struct od_zig_result_bytes {
  od_zig_error_code code;
  od_zig_bytes *value;
} od_zig_result_bytes;

typedef struct od_zig_result_metadata {
  od_zig_error_code code;
  od_zig_metadata *value;
} od_zig_result_metadata;

typedef struct od_zig_result_info {
  od_zig_error_code code;
  od_zig_operator_info *value;
} od_zig_result_info;

typedef struct od_zig_result_presigned_request {
  od_zig_error_code code;
  od_zig_presigned_request *value;
} od_zig_result_presigned_request;

typedef struct od_zig_result_entry_view {
  od_zig_error_code code;
  bool has_value;
  od_zig_entry_view value;
} od_zig_result_entry_view;

typedef struct od_zig_result_entry_owned {
  od_zig_error_code code;
  bool has_value;
  od_zig_entry_owned *value;
} od_zig_result_entry_owned;

typedef struct od_zig_result_task {
  od_zig_error_code code;
  od_zig_task *value;
} od_zig_result_task;

typedef struct od_zig_result_error_info_optional {
  od_zig_error_code code;
  bool has_value;
  od_zig_error_info *value;
} od_zig_result_error_info_optional;

typedef struct od_zig_result_bool {
  od_zig_error_code code;
  bool value;
} od_zig_result_bool;

typedef struct od_zig_result_unit {
  od_zig_error_code code;
} od_zig_result_unit;

typedef struct od_zig_result_usize {
  od_zig_error_code code;
  uintptr_t value;
} od_zig_result_usize;

typedef struct od_zig_result_u64 {
  od_zig_error_code code;
  uint64_t value;
} od_zig_result_u64;

typedef struct od_zig_result_optional_u64 {
  od_zig_error_code code;
  bool has_value;
  uint64_t value;
} od_zig_result_optional_u64;

typedef struct od_zig_task_poll_result {
  od_zig_error_code code;
  od_zig_poll_state state;
} od_zig_task_poll_result;

typedef struct od_zig_task_cancel_result {
  od_zig_error_code code;
  bool completed;
} od_zig_task_cancel_result;

typedef struct od_zig_task_state_result {
  od_zig_error_code code;
  od_zig_task_state state;
} od_zig_task_state_result;

od_zig_result_runtime od_zig_runtime_new(od_zig_runtime_options options);
void od_zig_runtime_free(od_zig_runtime *runtime);
od_zig_notifier_kind od_zig_runtime_notifier_kind(const od_zig_runtime *runtime);
int32_t od_zig_runtime_notifier_fd(const od_zig_runtime *runtime);
uintptr_t od_zig_runtime_notifier_handle(const od_zig_runtime *runtime);
od_zig_result_usize od_zig_runtime_drain_ready_tasks(od_zig_runtime *runtime);
od_zig_result_optional_u64 od_zig_runtime_pop_ready_task(od_zig_runtime *runtime);
od_zig_result_unit od_zig_runtime_ack_ready_task(od_zig_runtime *runtime, uint64_t task_id);
od_zig_result_error_info_optional od_zig_runtime_last_error_info(od_zig_runtime *runtime);

od_zig_result_operator od_zig_operator_new(od_zig_slice scheme, const od_zig_option *options_ptr, uintptr_t options_len);
od_zig_result_operator od_zig_operator_new_with_runtime(od_zig_runtime *runtime, od_zig_slice scheme, const od_zig_option *options_ptr, uintptr_t options_len);
void od_zig_operator_free(od_zig_operator *operator);
od_zig_result_unit od_zig_operator_check(od_zig_operator *operator);
od_zig_result_bool od_zig_operator_exists(od_zig_operator *operator, od_zig_slice path);
od_zig_result_unit od_zig_operator_create_dir(od_zig_operator *operator, od_zig_slice path);
od_zig_result_unit od_zig_operator_delete(od_zig_operator *operator, od_zig_slice path, od_zig_delete_options options);
od_zig_result_unit od_zig_operator_rename(od_zig_operator *operator, od_zig_slice from, od_zig_slice to);
od_zig_result_unit od_zig_operator_copy(od_zig_operator *operator, od_zig_slice from, od_zig_slice to, od_zig_copy_options options);
od_zig_result_unit od_zig_operator_write(od_zig_operator *operator, od_zig_slice path, od_zig_slice data, od_zig_write_options options);
od_zig_result_bytes od_zig_operator_read_bytes(od_zig_operator *operator, od_zig_slice path, od_zig_read_options options);
od_zig_result_metadata od_zig_operator_stat(od_zig_operator *operator, od_zig_slice path, od_zig_stat_options options);
od_zig_result_info od_zig_operator_info_get(od_zig_operator *operator);
od_zig_result_reader od_zig_operator_reader_open(od_zig_operator *operator, od_zig_slice path, od_zig_read_options options);
od_zig_result_writer od_zig_operator_writer_open(od_zig_operator *operator, od_zig_slice path, od_zig_write_options options);
od_zig_result_lister od_zig_operator_lister_open(od_zig_operator *operator, od_zig_slice path, od_zig_list_options options);
od_zig_result_presigned_request od_zig_operator_presign_read(od_zig_operator *operator, od_zig_slice path, uint64_t expire_secs, od_zig_presign_options options);
od_zig_result_presigned_request od_zig_operator_presign_write(od_zig_operator *operator, od_zig_slice path, uint64_t expire_secs, od_zig_presign_options options);
od_zig_result_presigned_request od_zig_operator_presign_stat(od_zig_operator *operator, od_zig_slice path, uint64_t expire_secs, od_zig_presign_options options);
od_zig_result_presigned_request od_zig_operator_presign_delete(od_zig_operator *operator, od_zig_slice path, uint64_t expire_secs, od_zig_presign_options options);

od_zig_result_usize od_zig_reader_read(od_zig_reader *reader, od_zig_mut_slice buf);
od_zig_result_u64 od_zig_reader_seek_to(od_zig_reader *reader, uint64_t pos);
od_zig_result_u64 od_zig_reader_seek_by(od_zig_reader *reader, int64_t delta);
od_zig_result_u64 od_zig_reader_seek_from_end(od_zig_reader *reader, int64_t delta);
void od_zig_reader_free(od_zig_reader *reader);

od_zig_result_usize od_zig_writer_write(od_zig_writer *writer, od_zig_slice data);
od_zig_result_unit od_zig_writer_close(od_zig_writer *writer);
void od_zig_writer_free(od_zig_writer *writer);

od_zig_result_entry_view od_zig_lister_next_view(od_zig_lister *lister);
od_zig_result_entry_owned od_zig_lister_next_owned(od_zig_lister *lister);
void od_zig_lister_free(od_zig_lister *lister);

uintptr_t od_zig_bytes_len(const od_zig_bytes *bytes);
const uint8_t *od_zig_bytes_ptr(const od_zig_bytes *bytes);
void od_zig_bytes_free(od_zig_bytes *bytes);

void od_zig_metadata_free(od_zig_metadata *metadata);
void od_zig_info_free(od_zig_operator_info *info);
void od_zig_presigned_request_free(od_zig_presigned_request *request);
void od_zig_entry_owned_free(od_zig_entry_owned *entry);
void od_zig_error_info_free(od_zig_error_info *error_info);

od_zig_result_task od_zig_operator_check_start(od_zig_operator *operator);
od_zig_result_task od_zig_operator_exists_start(od_zig_operator *operator, od_zig_slice path);
od_zig_result_task od_zig_operator_create_dir_start(od_zig_operator *operator, od_zig_slice path);
od_zig_result_task od_zig_operator_delete_start(od_zig_operator *operator, od_zig_slice path, od_zig_delete_options options);
od_zig_result_task od_zig_operator_rename_start(od_zig_operator *operator, od_zig_slice from, od_zig_slice to);
od_zig_result_task od_zig_operator_copy_start(od_zig_operator *operator, od_zig_slice from, od_zig_slice to, od_zig_copy_options options);
od_zig_result_task od_zig_operator_write_start(od_zig_operator *operator, od_zig_slice path, od_zig_slice data, od_zig_write_options options);
od_zig_result_task od_zig_operator_read_start(od_zig_operator *operator, od_zig_slice path, od_zig_read_options options);
od_zig_result_task od_zig_operator_stat_start(od_zig_operator *operator, od_zig_slice path, od_zig_stat_options options);
od_zig_result_task od_zig_operator_info_start(od_zig_operator *operator);
od_zig_result_task od_zig_operator_presign_read_start(od_zig_operator *operator, od_zig_slice path, uint64_t expire_secs, od_zig_presign_options options);
od_zig_result_task od_zig_operator_presign_write_start(od_zig_operator *operator, od_zig_slice path, uint64_t expire_secs, od_zig_presign_options options);
od_zig_result_task od_zig_operator_presign_stat_start(od_zig_operator *operator, od_zig_slice path, uint64_t expire_secs, od_zig_presign_options options);
od_zig_result_task od_zig_operator_presign_delete_start(od_zig_operator *operator, od_zig_slice path, uint64_t expire_secs, od_zig_presign_options options);
od_zig_result_task od_zig_operator_reader_open_start(od_zig_operator *operator, od_zig_slice path, od_zig_read_options options);
od_zig_result_task od_zig_operator_writer_open_start(od_zig_operator *operator, od_zig_slice path, od_zig_write_options options);
od_zig_result_task od_zig_operator_lister_open_start(od_zig_operator *operator, od_zig_slice path, od_zig_list_options options);

od_zig_result_task od_zig_reader_read_start(od_zig_async_reader *reader, od_zig_mut_slice buf);
od_zig_result_task od_zig_reader_seek_to_start(od_zig_async_reader *reader, uint64_t pos);
od_zig_result_task od_zig_reader_seek_by_start(od_zig_async_reader *reader, int64_t delta);
od_zig_result_task od_zig_reader_seek_from_end_start(od_zig_async_reader *reader, int64_t delta);
od_zig_result_task od_zig_writer_write_start(od_zig_async_writer *writer, od_zig_slice data);
od_zig_result_task od_zig_writer_close_start(od_zig_async_writer *writer);
od_zig_result_task od_zig_lister_next_owned_start(od_zig_async_lister *lister);

void od_zig_async_reader_free(od_zig_async_reader *reader);
void od_zig_async_writer_free(od_zig_async_writer *writer);
void od_zig_async_lister_free(od_zig_async_lister *lister);

od_zig_task_poll_result od_zig_task_poll(od_zig_task *task);
od_zig_task_cancel_result od_zig_task_cancel(od_zig_task *task);
od_zig_task_state_result od_zig_task_get_state(od_zig_task *task);
od_zig_error_code od_zig_task_error_code(od_zig_task *task);
od_zig_result_unit od_zig_task_take_unit(od_zig_task *task);
od_zig_result_bool od_zig_task_take_bool(od_zig_task *task);
od_zig_result_usize od_zig_task_take_usize(od_zig_task *task);
od_zig_result_u64 od_zig_task_take_u64(od_zig_task *task);
od_zig_result_bytes od_zig_task_take_bytes(od_zig_task *task);
od_zig_result_metadata od_zig_task_take_metadata(od_zig_task *task);
od_zig_result_info od_zig_task_take_info(od_zig_task *task);
od_zig_result_presigned_request od_zig_task_take_presigned_request(od_zig_task *task);
od_zig_result_entry_owned od_zig_task_take_entry(od_zig_task *task);
od_zig_result_async_reader od_zig_task_take_reader(od_zig_task *task);
od_zig_result_async_writer od_zig_task_take_writer(od_zig_task *task);
od_zig_result_async_lister od_zig_task_take_lister(od_zig_task *task);
void od_zig_task_free(od_zig_task *task);

#ifdef __cplusplus
}
#endif

#endif