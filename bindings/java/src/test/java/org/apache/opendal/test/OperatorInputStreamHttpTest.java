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
import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.net.InetSocketAddress;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Random;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import org.apache.commons.io.IOUtils;
import org.apache.opendal.ByteRange;
import org.apache.opendal.OpenDALException;
import org.apache.opendal.Operator;
import org.apache.opendal.OperatorInputStream;
import org.apache.opendal.ReadOptions;
import org.apache.opendal.ReaderOptions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

class OperatorInputStreamHttpTest {
    @ParameterizedTest
    @ValueSource(ints = {1, 4})
    void testConcurrentAloneKeepsUnchunkedStreaming(int concurrent) throws Exception {
        try (final RangeServer server = new RangeServer();
                final Operator op = server.operator()) {
            final ReaderOptions options =
                    ReaderOptions.builder().concurrent(concurrent).build();
            try (final OperatorInputStream in = op.createInputStream("data.bin", ByteRange.all(), options)) {
                assertThat(IOUtils.toByteArray(in)).isEqualTo(server.content);
            }
            assertThat(server.ranges).containsExactly("all");
            assertThat(server.heads.get()).isZero();
        }
    }

    @ParameterizedTest
    @ValueSource(ints = {1024, 4096})
    void testChunkChangesRequestsFromPublicJavaEntryPoint(int chunk) throws Exception {
        try (final RangeServer server = new RangeServer();
                final Operator op = server.operator()) {
            final int offset = 3;
            final int end = server.content.length - 7;
            final ReaderOptions options = ReaderOptions.builder()
                    .chunk(chunk)
                    .concurrent(3)
                    .prefetch(1)
                    .build();
            try (final OperatorInputStream in =
                    op.createInputStream("data.bin", ByteRange.of(offset, end - offset), options)) {
                assertThat(IOUtils.toByteArray(in)).isEqualTo(Arrays.copyOfRange(server.content, offset, end));
                assertThat(in.read()).isEqualTo(-1);
            }
            final List<String> expected = new ArrayList<>();
            for (int start = offset; start < end; start += chunk) {
                expected.add("bytes=" + start + "-" + (Math.min(start + chunk, end) - 1));
            }
            // Only core schedules requests; the server records the Range headers it receives.
            assertThat(server.ranges).containsExactlyInAnyOrderElementsOf(expected);
        }
    }

    @ParameterizedTest
    @ValueSource(booleans = {false, true})
    void testContentLengthHintAvoidsStatForOpenEndedRange(boolean withHint) throws Exception {
        try (final RangeServer server = new RangeServer();
                final Operator op = server.operator()) {
            final ReaderOptions options = ReaderOptions.builder()
                    .chunk(1024)
                    .contentLengthHint(withHint ? server.content.length : -1)
                    .build();
            try (final OperatorInputStream in = op.createInputStream("data.bin", ByteRange.from(3), options)) {
                assertThat(IOUtils.toByteArray(in))
                        .isEqualTo(Arrays.copyOfRange(server.content, 3, server.content.length));
            }
            assertThat(server.heads.get()).isEqualTo(withHint ? 0 : 1);
        }
    }

    @Test
    void testConcurrencyAndPausedConsumer() throws Exception {
        final int concurrent = 3;
        final int prefetch = 1;
        final ExecutorService consumer = Executors.newSingleThreadExecutor();
        try (final RangeServer server = new RangeServer();
                final Operator op = server.operator()) {
            server.arrived = new CountDownLatch(concurrent);
            server.release = new CountDownLatch(1);
            server.requestLimit = concurrent;
            final ReaderOptions options = ReaderOptions.builder()
                    .chunk(1024)
                    .concurrent(concurrent)
                    .prefetch(prefetch)
                    .build();
            final CountDownLatch byteRead = new CountDownLatch(1);
            final CountDownLatch finishConsumer = new CountDownLatch(1);
            final Future<Integer> firstByte = consumer.submit(() -> {
                // The consuming thread also owns close, including failure and timeout paths.
                try (final OperatorInputStream in =
                        op.createInputStream("data.bin", ByteRange.of(3, server.content.length - 10), options)) {
                    final int value = in.read();
                    byteRead.countDown();
                    if (!finishConsumer.await(15, TimeUnit.SECONDS)) {
                        throw new IOException("Timed out waiting for the paused consumer");
                    }
                    return value;
                }
            });
            try {
                assertThat(server.arrived.await(10, TimeUnit.SECONDS)).isTrue();
                // All configured tasks are blocked at the server, so another request exceeds the limit.
                assertThat(server.exceeded.await(250, TimeUnit.MILLISECONDS)).isFalse();
                server.requestLimit = concurrent + prefetch + 1;
                server.release.countDown();
                assertThat(byteRead.await(10, TimeUnit.SECONDS)).isTrue();
                assertThat(server.peak.get()).isEqualTo(concurrent);
                // The consumer holds one chunk and stops reading; core must not schedule the whole object.
                assertThat(server.exceeded.await(250, TimeUnit.MILLISECONDS)).isFalse();
                assertThat(server.ranges.size()).isBetween(concurrent, server.requestLimit);
            } finally {
                server.release.countDown();
                finishConsumer.countDown();
                assertThat(firstByte.get(15, TimeUnit.SECONDS)).isEqualTo(server.content[3] & 0xff);
            }
        } finally {
            consumer.shutdownNow();
        }
    }

    @Test
    void testCloseBeforeFirstReadMakesNoRequests() throws Exception {
        try (final RangeServer server = new RangeServer();
                final Operator op = server.operator()) {
            final ReaderOptions options = ReaderOptions.builder().chunk(1024).concurrent(3).build();
            final OperatorInputStream in = op.createInputStream("data.bin", ByteRange.of(3, 2048), options);
            in.close();
            in.close();
            assertThat(server.ranges).isEmpty();
            assertThat(server.heads.get()).isZero();
        }
    }

    @Test
    void testReadFailureDoesNotBecomeEof() throws Exception {
        try (final RangeServer server = new RangeServer();
                final Operator op = server.operator()) {
            server.failFrom = 1024;
            final ReaderOptions options = ReaderOptions.builder().chunk(1024).concurrent(1).build();
            try (final OperatorInputStream in =
                    op.createInputStream("data.bin", ByteRange.of(0, 4096), options)) {
                assertThat(in.read()).isEqualTo(server.content[0] & 0xff);
                assertThatThrownBy(() -> IOUtils.toByteArray(in)).isInstanceOf(OpenDALException.class);
            }
            assertThat(server.ranges).contains("bytes=1024-2047");
        }
    }

    @Test
    void testOutOfBoundsStillFailsForLegacyAndNewRanges() throws Exception {
        try (final RangeServer server = new RangeServer();
                final Operator op = server.operator()) {
            final long offset = server.content.length + 1L;
            try (final OperatorInputStream in = op.createInputStream(
                    "data.bin", ReadOptions.builder().offset(offset).length(1).build())) {
                assertThatThrownBy(() -> in.read()).isInstanceOf(OpenDALException.class);
            }
            try (final OperatorInputStream in = op.createInputStream("data.bin", ByteRange.of(offset, 1))) {
                assertThatThrownBy(() -> in.read()).isInstanceOf(OpenDALException.class);
            }
            assertThat(server.ranges)
                    .containsExactly("bytes=" + offset + "-" + offset, "bytes=" + offset + "-" + offset);
        }
    }

    private static final class RangeServer implements AutoCloseable {
        private final byte[] content = new byte[64 * 1024 + 13];
        private final List<String> ranges = new CopyOnWriteArrayList<>();
        private final AtomicInteger heads = new AtomicInteger();
        private final AtomicInteger active = new AtomicInteger();
        private final AtomicInteger peak = new AtomicInteger();
        private final CountDownLatch exceeded = new CountDownLatch(1);
        private final ExecutorService requests = Executors.newCachedThreadPool();
        private final HttpServer server;
        private volatile CountDownLatch arrived = new CountDownLatch(0);
        private volatile CountDownLatch release = new CountDownLatch(0);
        private volatile int requestLimit = Integer.MAX_VALUE;
        private volatile int failFrom = Integer.MAX_VALUE;

        private RangeServer() throws IOException {
            new Random(8252).nextBytes(content);
            server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
            server.createContext("/data.bin", this::handle);
            server.setExecutor(requests);
            server.start();
        }

        private Operator operator() {
            return Operator.of(ServiceConfig.Http.builder()
                    .endpoint("http://127.0.0.1:" + server.getAddress().getPort())
                    .build());
        }

        private void handle(HttpExchange exchange) throws IOException {
            try {
                exchange.getResponseHeaders().set("Accept-Ranges", "bytes");
                if ("HEAD".equals(exchange.getRequestMethod())) {
                    heads.incrementAndGet();
                    exchange.getResponseHeaders().set("Content-Length", Integer.toString(content.length));
                    exchange.sendResponseHeaders(200, -1);
                    return;
                }
                final String range = exchange.getRequestHeaders().getFirst("Range");
                ranges.add(range == null ? "all" : range);
                if (ranges.size() > requestLimit) {
                    exceeded.countDown();
                }
                final int overlap = active.incrementAndGet();
                peak.accumulateAndGet(overlap, Math::max);
                arrived.countDown();
                try {
                    if (!release.await(10, TimeUnit.SECONDS)) {
                        throw new IOException("Timed out waiting to release range requests");
                    }
                } catch (InterruptedException e) {
                    Thread.currentThread().interrupt();
                    throw new IOException(e);
                } finally {
                    // Stop counting before the client can finish this request and schedule another task.
                    active.decrementAndGet();
                }
                int start = 0;
                int end = content.length;
                if (range != null) {
                    final String[] bounds = range.substring("bytes=".length()).split("-", -1);
                    start = Integer.parseInt(bounds[0]);
                    if (!bounds[1].isEmpty()) {
                        end = Math.min(Integer.parseInt(bounds[1]) + 1, end);
                    }
                }
                if (start >= content.length) {
                    exchange.getResponseHeaders().set("Content-Range", "bytes */" + content.length);
                    exchange.sendResponseHeaders(416, -1);
                    return;
                }
                if (start >= failFrom) {
                    exchange.sendResponseHeaders(500, -1);
                    return;
                }
                if (range != null) {
                    exchange.getResponseHeaders()
                            .set("Content-Range", "bytes " + start + "-" + (end - 1) + "/" + content.length);
                }
                exchange.sendResponseHeaders(range == null ? 200 : 206, end - start);
                exchange.getResponseBody().write(content, start, end - start);
            } finally {
                exchange.close();
            }
        }

        @Override
        public void close() {
            release.countDown();
            server.stop(0);
            requests.shutdownNow();
        }
    }
}
