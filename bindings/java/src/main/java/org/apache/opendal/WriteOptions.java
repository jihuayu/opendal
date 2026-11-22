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

package org.apache.opendal;

import java.time.Duration;
import java.util.Map;
import lombok.Builder;

@Builder
public class WriteOptions {

    /**
     * Sets the Content-Type header for the object.
     * Requires capability: writeWithContentType
     */
    public final String contentType;

    /**
     * Sets the Content-Disposition header for the object
     * Requires capability: writeWithContentDisposition
     */
    public final String contentDisposition;

    /**
     * Sets the Cache-Control header for the object
     * Requires capability: writeWithCacheControl
     */
    public final String cacheControl;

    /**
     * Sets the Content-Encoding header for the object
     */
    public final String contentEncoding;

    /**
     * Sets the If-Match header for conditional writes
     * Requires capability: writeWithIfMatch
     */
    public final String ifMatch;

    /**
     * Sets the If-None-Match header for conditional writes
     * Requires capability: writeWithIfNoneMatch
     */
    public final String ifNoneMatch;

    /**
     * Writes only when the file at the write path has this exact version.
     * Requires capability: writeWithIfVersionMatch
     */
    public final String ifVersionMatch;

    /**
     * Writes only when the file at the write path does not have this version.
     * Requires capability: writeWithIfVersionNotMatch
     */
    public final String ifVersionNotMatch;

    /**
     * Writes only when the current file at the write path still has the identity recorded in
     * metadata previously returned by OpenDAL for the same path.
     * OpenDAL derives a version match when the service supports version conditions and a version
     * is present; otherwise it uses a strong (non-weak) ETag when available.
     * Fails with {@code ConfigInvalid} when the metadata contains neither a version nor a non-weak
     * ETag, {@code ConditionNotMatch} when the file changed, is missing, or is combined with a
     * conflicting {@link #ifMatch} or {@link #ifVersionMatch} value, and {@code Unsupported} when
     * the service does not support the derived primitive condition.
     */
    public final Metadata ifNotChanged;

    /**
     * Sets custom metadata for the file.
     * Requires capability: writeWithUserMetadata
     */
    public final Map<String, String> userMetadata;

    /**
     * Sets how long the new file lives after the write commits.
     * Requires capability: writeWithExpires
     */
    public final Duration expires;

    /**
     * Enables append mode for writing.
     * When true, data will be appended to the end of existing file.
     * Requires capability: writeCanAppend
     */
    public final boolean append;

    /**
     * Write only if the file does not exist.
     * Operation will fail if the file at the designated path already exists.
     * Requires capability: writeWithIfNotExists
     */
    public final boolean ifNotExists;

    /**
     * Sets concurrent write operations for this writer.
     */
    @Builder.Default
    public final int concurrent = 1;

    /**
     * Sets chunk size for buffered writes.
     */
    @Builder.Default
    public final long chunk = -1L;
}
