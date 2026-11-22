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
import com.sun.net.httpserver.Headers;
import com.sun.net.httpserver.HttpServer;
import java.net.InetSocketAddress;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.time.Duration;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.stream.Stream;
import org.apache.opendal.AsyncOperator;
import org.apache.opendal.Metadata;
import org.apache.opendal.OpenDALException.Code;
import org.apache.opendal.Operator;
import org.apache.opendal.ServiceConfig;
import org.apache.opendal.WriteOptions;
import org.apache.opendal.layer.CapabilityCheckLayer;
import org.apache.opendal.test.condition.OpenDALExceptionCondition;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

public class WriteOptionsTest {

    static Stream<Arguments> unsupportedOptions() {
        return Stream.of(
                Arguments.of(
                        "ifVersionMatch",
                        WriteOptions.builder().ifVersionMatch("v1").build()),
                Arguments.of(
                        "ifVersionNotMatch",
                        WriteOptions.builder().ifVersionNotMatch("v1").build()),
                Arguments.of("ifNotChanged with an ETag", writeIfNotChanged("\"etag\"", null)),
                Arguments.of("ifNotChanged with a version", writeIfNotChanged(null, "v1")),
                Arguments.of(
                        "expires",
                        WriteOptions.builder().expires(Duration.ofSeconds(60)).build()));
    }

    private static WriteOptions writeIfNotChanged(String etag, String version) {
        Metadata snapshot = new Metadata(0, 7, null, null, null, null, etag, null, version);
        return WriteOptions.builder().ifNotChanged(snapshot).build();
    }

    @ParameterizedTest(name = "{0}")
    @MethodSource("unsupportedOptions")
    void testUnsupportedWriteOption(String name, WriteOptions options) {
        try (AsyncOperator async = AsyncOperator.of("memory", Collections.emptyMap());
                Operator op = async.blocking()) {
            assertThatThrownBy(() -> op.write("file", "content", options))
                    .is(OpenDALExceptionCondition.ofSync(Code.Unsupported));
            assertThatThrownBy(async.write("file", "content", options)::join)
                    .is(OpenDALExceptionCondition.ofAsync(Code.Unsupported));
        }
    }

    @Test
    void testMetadataWithoutStrongIdentityIsRejected() {
        try (AsyncOperator async = AsyncOperator.of("memory", Collections.emptyMap());
                Operator op = async.blocking()) {
            op.write("file", "content");
            WriteOptions withoutIdentity =
                    WriteOptions.builder().ifNotChanged(op.stat("file")).build();

            assertThatThrownBy(() -> op.write("file", "updated", withoutIdentity))
                    .is(OpenDALExceptionCondition.ofSync(Code.ConfigInvalid));
            assertThatThrownBy(async.write("file", "updated", withoutIdentity)::join)
                    .is(OpenDALExceptionCondition.ofAsync(Code.ConfigInvalid));

            WriteOptions weakEtagOnly = writeIfNotChanged("W/\"weak\"", null);
            assertThatThrownBy(() -> op.write("file", "updated", weakEtagOnly))
                    .is(OpenDALExceptionCondition.ofSync(Code.ConfigInvalid));
            assertThatThrownBy(async.write("file", "updated", weakEtagOnly)::join)
                    .is(OpenDALExceptionCondition.ofAsync(Code.ConfigInvalid));
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

    @Test
    void testContentLanguageReachesCoreAndS3Request(@TempDir Path root) throws Exception {
        WriteOptions options =
                WriteOptions.builder().contentLanguage("fr-CA, en").build();
        try (AsyncOperator base = AsyncOperator.of("fs", Collections.singletonMap("root", root.toString()));
                AsyncOperator checked = base.layer(new CapabilityCheckLayer());
                Operator op = checked.blocking()) {
            assertThatThrownBy(() -> op.write("file", "content", options))
                    .is(OpenDALExceptionCondition.ofSync(Code.Unsupported));
            assertThatThrownBy(checked.write("file", "content", options)::join)
                    .is(OpenDALExceptionCondition.ofAsync(Code.Unsupported));
        }

        List<Headers> requests = new CopyOnWriteArrayList<>();
        HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.createContext("/", exchange -> {
            try {
                requests.add(exchange.getRequestHeaders());
                exchange.getRequestBody().close();
                exchange.getResponseHeaders().set("ETag", "\"etag-1\"");
                exchange.sendResponseHeaders(200, -1);
            } finally {
                exchange.close();
            }
        });
        server.start();
        String endpoint = "http://127.0.0.1:" + server.getAddress().getPort();

        try (AsyncOperator async = AsyncOperator.of(ServiceConfig.S3
                        .builder()
                        .bucket("bucket")
                        .region("us-east-1")
                        .endpoint(endpoint)
                        .skipSignature(true)
                        .build());
                Operator op = async.blocking()) {
            op.write("file", "content", options);
            async.write("file", "content", options).join();
        } finally {
            server.stop(0);
        }

        assertThat(requests).hasSize(2);
        for (Headers headers : requests) {
            assertThat(headers.getFirst("Content-Language")).isEqualTo("fr-CA, en");
        }
    }

    @Test
    void testConditionalEtagAndIfNotChangedWrites() throws Exception {
        List<Headers> requests = new CopyOnWriteArrayList<>();
        HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.createContext("/", exchange -> {
            try {
                requests.add(exchange.getRequestHeaders());
                exchange.getRequestBody().close();
                if ("\"stale\"".equals(exchange.getRequestHeaders().getFirst("If-Match"))) {
                    exchange.sendResponseHeaders(412, -1);
                    return;
                }
                exchange.getResponseHeaders().set("ETag", "\"etag-1\"");
                exchange.sendResponseHeaders(200, -1);
            } finally {
                exchange.close();
            }
        });
        server.start();
        String endpoint = "http://127.0.0.1:" + server.getAddress().getPort();

        try (AsyncOperator async = AsyncOperator.of(ServiceConfig.S3
                        .builder()
                        .bucket("bucket")
                        .region("us-east-1")
                        .endpoint(endpoint)
                        .skipSignature(true)
                        .build());
                Operator op = async.blocking()) {
            WriteOptions match = writeIfNotChanged("\"etag-1\"", null);
            op.write("file", "content", match);
            async.write("file", "content", match).join();

            WriteOptions stale = writeIfNotChanged("\"stale\"", null);
            assertThatThrownBy(() -> op.write("file", "content", stale))
                    .is(OpenDALExceptionCondition.ofSync(Code.ConditionNotMatch));
            assertThatThrownBy(async.write("file", "content", stale)::join)
                    .is(OpenDALExceptionCondition.ofAsync(Code.ConditionNotMatch));

            Metadata snapshot = new Metadata(0, 7, null, null, null, null, "\"etag-1\"", null, null);
            WriteOptions conflicting = WriteOptions.builder()
                    .ifNotChanged(snapshot)
                    .ifMatch("\"etag-2\"")
                    .build();
            assertThatThrownBy(() -> op.write("file", "content", conflicting))
                    .is(OpenDALExceptionCondition.ofSync(Code.ConditionNotMatch));
            assertThatThrownBy(async.write("file", "content", conflicting)::join)
                    .is(OpenDALExceptionCondition.ofAsync(Code.ConditionNotMatch));
        } finally {
            server.stop(0);
        }

        assertThat(requests)
                .extracting(headers -> headers.getFirst("If-Match"))
                .containsExactly("\"etag-1\"", "\"etag-1\"", "\"stale\"", "\"stale\"");
    }

    @Test
    void testConditionalVersionAndIfNotChangedWrites() throws Exception {
        List<URI> initiatedUris = new CopyOnWriteArrayList<>();
        HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.createContext("/", exchange -> {
            try {
                exchange.getRequestBody().close();
                URI uri = exchange.getRequestURI();
                if ("POST".equals(exchange.getRequestMethod())) {
                    initiatedUris.add(uri);
                    if (uri.getQuery() != null && uri.getQuery().contains("ifGenerationMatch=99")) {
                        exchange.sendResponseHeaders(412, -1);
                        return;
                    }
                    String location = "http://127.0.0.1:" + server.getAddress().getPort() + "/upload-session";
                    exchange.getResponseHeaders().set("Location", location);
                    exchange.sendResponseHeaders(200, -1);
                    return;
                }
                byte[] body = "{\"size\":\"7\",\"generation\":\"19\"}".getBytes(StandardCharsets.UTF_8);
                exchange.getResponseHeaders().set("Content-Type", "application/json");
                exchange.sendResponseHeaders(200, body.length);
                exchange.getResponseBody().write(body);
            } finally {
                exchange.close();
            }
        });
        server.start();
        String endpoint = "http://127.0.0.1:" + server.getAddress().getPort();

        try (AsyncOperator async = AsyncOperator.of(ServiceConfig.Gcs.builder()
                        .bucket("bucket")
                        .endpoint(endpoint)
                        .skipSignature(true)
                        .build());
                Operator op = async.blocking()) {
            Metadata snapshot = new Metadata(0, 7, null, null, null, null, "\"etag-1\"", null, "17");
            WriteOptions options = WriteOptions.builder()
                    .ifNotChanged(snapshot)
                    .ifVersionNotMatch("18")
                    .build();
            op.write("file", "content", options);
            async.write("file", "content", options).join();

            WriteOptions stale = WriteOptions.builder().ifVersionMatch("99").build();
            assertThatThrownBy(() -> op.write("file", "content", stale))
                    .is(OpenDALExceptionCondition.ofSync(Code.ConditionNotMatch));
            assertThatThrownBy(async.write("file", "content", stale)::join)
                    .is(OpenDALExceptionCondition.ofAsync(Code.ConditionNotMatch));

            WriteOptions conflicting = WriteOptions.builder()
                    .ifNotChanged(snapshot)
                    .ifVersionMatch("18")
                    .build();
            assertThatThrownBy(() -> op.write("file", "content", conflicting))
                    .is(OpenDALExceptionCondition.ofSync(Code.ConditionNotMatch));
            assertThatThrownBy(async.write("file", "content", conflicting)::join)
                    .is(OpenDALExceptionCondition.ofAsync(Code.ConditionNotMatch));
        } finally {
            server.stop(0);
        }

        assertThat(initiatedUris).hasSize(4);
        assertThat(initiatedUris.get(0).getQuery()).contains("ifGenerationMatch=17", "ifGenerationNotMatch=18");
        assertThat(initiatedUris.get(1).getQuery()).contains("ifGenerationMatch=17", "ifGenerationNotMatch=18");
        assertThat(initiatedUris.get(2).getQuery()).contains("ifGenerationMatch=99");
        assertThat(initiatedUris.get(3).getQuery()).contains("ifGenerationMatch=99");
    }
}
