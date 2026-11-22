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

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;
import com.sun.net.httpserver.HttpServer;
import java.net.InetSocketAddress;
import java.nio.file.Path;
import java.time.Duration;
import java.util.Collections;
import java.util.stream.Stream;
import org.apache.opendal.AsyncOperator;
import org.apache.opendal.DeleteOptions;
import org.apache.opendal.OpenDALException.Code;
import org.apache.opendal.Operator;
import org.apache.opendal.PresignedRequest;
import org.apache.opendal.RenameOptions;
import org.apache.opendal.RestoreOptions;
import org.apache.opendal.ServiceConfig;
import org.apache.opendal.test.condition.OpenDALExceptionCondition;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.MethodSource;
import org.junit.jupiter.params.provider.ValueSource;

public class OperatorOperationsTest {
    @TempDir
    Path root;

    @ParameterizedTest
    @ValueSource(strings = {"memory", "fs"})
    void testExistsAndCheck(String scheme) {
        try (AsyncOperator async = newOperator(scheme);
                Operator op = async.blocking()) {
            op.check();
            async.check().join();

            assertThat(op.exists("missing")).isFalse();
            assertThat(async.exists("missing").join()).isFalse();

            op.write("present", "content");
            assertThat(op.exists("present")).isTrue();
            assertThat(async.exists("present").join()).isTrue();
        }
    }

    @Test
    void testExistsPropagatesNonNotFoundErrors() throws Exception {
        HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.createContext("/", exchange -> {
            try {
                exchange.sendResponseHeaders(403, -1);
            } finally {
                exchange.close();
            }
        });
        server.start();
        String endpoint = "http://127.0.0.1:" + server.getAddress().getPort();

        try (AsyncOperator async = AsyncOperator.of("http", Collections.singletonMap("endpoint", endpoint));
                Operator op = async.blocking()) {
            assertThatThrownBy(() -> op.exists("file")).is(OpenDALExceptionCondition.ofSync(Code.PermissionDenied));
            assertThatThrownBy(async.exists("file")::join).is(OpenDALExceptionCondition.ofAsync(Code.PermissionDenied));
        } finally {
            server.stop(0);
        }
    }

    @Test
    void testRenameWithOptions() {
        try (AsyncOperator async = newOperator("fs");
                Operator op = async.blocking()) {
            op.write("src-1", "a");
            op.write("dst-1", "b");

            RenameOptions ifNotExists =
                    RenameOptions.builder().ifNotExists(true).build();
            Code expectedCode = op.info.capability.renameWithIfNotExists ? Code.ConditionNotMatch : Code.Unsupported;
            assertThatThrownBy(() -> op.rename("src-1", "dst-1", ifNotExists))
                    .is(OpenDALExceptionCondition.ofSync(expectedCode));
            assertThatThrownBy(async.rename("src-1", "dst-1", ifNotExists)::join)
                    .is(OpenDALExceptionCondition.ofAsync(expectedCode));

            if (op.info.capability.renameWithIfNotExists) {
                op.rename("src-1", "dst-new", ifNotExists);
                assertThat(op.exists("src-1")).isFalse();
                assertThat(op.exists("dst-new")).isTrue();
            } else {
                op.rename("src-1", "dst-2", RenameOptions.builder().build());
                assertThat(op.exists("src-1")).isFalse();
                assertThat(op.exists("dst-2")).isTrue();
            }
        }
    }

    @Test
    void testRestoreUnsupportedOnMemory() {
        try (AsyncOperator async = newOperator("memory");
                Operator op = async.blocking()) {
            RestoreOptions options =
                    RestoreOptions.builder().version("v1").ifNotExists(true).build();
            assertThatThrownBy(() -> op.restore("file", options))
                    .is(OpenDALExceptionCondition.ofSync(Code.Unsupported));
            assertThatThrownBy(async.restore("file", options)::join)
                    .is(OpenDALExceptionCondition.ofAsync(Code.Unsupported));
        }
    }

    @Test
    void testRestoreIfNotExistsRequiresVersion() {
        try (AsyncOperator async = AsyncOperator.of("memory", Collections.emptyMap());
                Operator op = async.blocking()) {
            RestoreOptions options = RestoreOptions.builder().ifNotExists(true).build();
            assertThatThrownBy(() -> op.restore("file", options))
                    .is(OpenDALExceptionCondition.ofSync(Code.ConfigInvalid));
            assertThatThrownBy(async.restore("file", options)::join)
                    .is(OpenDALExceptionCondition.ofAsync(Code.ConfigInvalid));
        }
    }

    @Test
    void testPresignDelete() {
        ServiceConfig.S3 s3 = ServiceConfig.S3
                .builder()
                .bucket("opendal-test")
                .region("us-east-1")
                .endpoint("https://s3.us-east-1.amazonaws.com")
                .accessKeyId("opendal-test-access-key")
                .secretAccessKey("opendal-test-secret-key")
                .disableConfigLoad(true)
                .build();
        try (AsyncOperator op = AsyncOperator.of(s3)) {
            PresignedRequest request =
                    op.presignDelete("report.csv", Duration.ofMinutes(10)).join();
            assertThat(request.getMethod()).isEqualTo("DELETE");
            assertThat(request.getUri()).contains("X-Amz-Signature", "X-Amz-Expires=600");

            PresignedRequest versioned = op.presignDelete(
                            "report.csv",
                            Duration.ofMinutes(10),
                            DeleteOptions.builder().version("v1").build())
                    .join();
            assertThat(versioned.getMethod()).isEqualTo("DELETE");
            assertThat(versioned.getUri()).contains("versionId=v1");
        }
    }

    static Stream<Duration> invalidPresignDurations() {
        return Stream.of(
                Duration.ZERO, Duration.ofSeconds(-1), Duration.ofNanos(-1), Duration.ofSeconds(Long.MAX_VALUE));
    }

    @ParameterizedTest
    @MethodSource("invalidPresignDurations")
    void testPresignDeleteRejectsNonPositiveOrOverflowingDuration(Duration invalidDuration) {
        ServiceConfig.S3 s3 = ServiceConfig.S3
                .builder()
                .bucket("opendal-test")
                .region("us-east-1")
                .endpoint("https://s3.us-east-1.amazonaws.com")
                .accessKeyId("opendal-test-access-key")
                .secretAccessKey("opendal-test-secret-key")
                .disableConfigLoad(true)
                .build();
        try (AsyncOperator op = AsyncOperator.of(s3)) {
            assertThatThrownBy(() -> op.presignDelete("report.csv", invalidDuration))
                    .is(OpenDALExceptionCondition.ofSync(Code.ConfigInvalid));
        }
    }

    private AsyncOperator newOperator(String scheme) {
        return AsyncOperator.of(
                scheme,
                "fs".equals(scheme) ? Collections.singletonMap("root", root.toString()) : Collections.emptyMap());
    }
}
