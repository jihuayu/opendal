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

package org.apache.opendal.test;

import static org.assertj.core.api.Assertions.assertThatThrownBy;
import java.time.Duration;
import java.util.Collections;
import java.util.stream.Stream;
import org.apache.opendal.AsyncOperator;
import org.apache.opendal.OpenDALException.Code;
import org.apache.opendal.Operator;
import org.apache.opendal.WriteOptions;
import org.apache.opendal.test.condition.OpenDALExceptionCondition;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.MethodSource;

public class WriteExpiresTest {

    @Test
    void testUnsupportedExpiresIsRejected() {
        WriteOptions options =
                WriteOptions.builder().expires(Duration.ofSeconds(60)).build();
        try (AsyncOperator async = AsyncOperator.of("memory", Collections.emptyMap());
                Operator op = async.blocking()) {
            assertThatThrownBy(() -> op.write("file", "content", options))
                    .is(OpenDALExceptionCondition.ofSync(Code.Unsupported));
            assertThatThrownBy(async.write("file", "content", options)::join)
                    .is(OpenDALExceptionCondition.ofAsync(Code.Unsupported));
        }
    }

    static Stream<Duration> invalidExpires() {
        return Stream.of(Duration.ZERO, Duration.ofSeconds(-1), Duration.ofNanos(-1));
    }

    @ParameterizedTest
    @MethodSource("invalidExpires")
    void testNonPositiveExpiresIsRejected(Duration expires) {
        WriteOptions options = WriteOptions.builder().expires(expires).build();
        try (AsyncOperator async = AsyncOperator.of("memory", Collections.emptyMap());
                Operator op = async.blocking()) {
            assertThatThrownBy(() -> op.write("file", "content", options))
                    .is(OpenDALExceptionCondition.ofSync(Code.ConfigInvalid));
            assertThatThrownBy(() -> async.write("file", "content", options))
                    .is(OpenDALExceptionCondition.ofSync(Code.ConfigInvalid));
        }
    }
}
