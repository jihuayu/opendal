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

/**
 * Selects a byte range independently of reader configuration.
 * Offsets and lengths are measured in bytes. Invalid values cause an
 * {@link OpenDALException} with code RangeNotSatisfied when the stream is created.
 */
public final class ByteRange {
    private final long offset;
    private final long length;
    private final boolean suffix;

    private ByteRange(long offset, long length, boolean suffix) {
        this.offset = offset;
        this.length = length;
        this.suffix = suffix;
    }

    /**
     * Selects the entire object.
     *
     * @return an unbounded range starting at zero
     */
    public static ByteRange all() {
        return from(0);
    }

    /**
     * Selects bytes from the given offset to the end of the object.
     *
     * @param offset non-negative starting offset
     * @return an unbounded range
     */
    public static ByteRange from(long offset) {
        return new ByteRange(offset, -1, false);
    }

    /**
     * Selects a range by offset and length. A zero length selects an empty range.
     * A length of -1 selects bytes to the end, as with {@link #from(long)}.
     *
     * @param offset non-negative starting offset
     * @param length non-negative byte count, or -1 to read to the end
     * @return the selected range
     */
    public static ByteRange of(long offset, long length) {
        return new ByteRange(offset, length, false);
    }

    /**
     * Selects the last {@code length} bytes of the object. A length larger than
     * the object selects the whole object; zero selects an empty range.
     * Unchunked streams require native or simulated suffix-read support from the service.
     *
     * @param length non-negative suffix length in bytes
     * @return a suffix range
     */
    public static ByteRange suffix(long length) {
        return new ByteRange(0, length, true);
    }

    /**
     * @return the starting offset, or zero for a suffix range
     */
    public long getOffset() {
        return offset;
    }

    /**
     * @return the byte count, or -1 for an unbounded range
     */
    public long getLength() {
        return length;
    }

    /**
     * @return whether the length is measured from the end of the object
     */
    public boolean isSuffix() {
        return suffix;
    }
}
