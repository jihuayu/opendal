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

import static org.apache.opendal.test.behavior.BehaviorTestBase.generateBytes;
import static org.assertj.core.api.Assertions.assertThat;
import static org.junit.jupiter.api.Assertions.assertTrue;
import com.sun.net.httpserver.HttpServer;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.time.Instant;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import org.apache.opendal.AsyncOperator;
import org.apache.opendal.Entry;
import org.apache.opendal.ListOptions;
import org.apache.opendal.Metadata;
import org.apache.opendal.Metadata.EntryMode;
import org.apache.opendal.Operator;
import org.apache.opendal.ServiceConfig;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

public class MetadataTest {
    @TempDir
    private static Path tempDir;

    @Test
    public void testAsyncMetadata() {
        final ServiceConfig.Fs fs =
                ServiceConfig.Fs.builder().root(tempDir.toString()).build();

        try (final AsyncOperator op = AsyncOperator.of(fs)) {
            final String dir = UUID.randomUUID() + "/";
            op.createDir(dir).join();
            final Metadata dirMetadata = op.stat(dir).join();
            assertTrue(dirMetadata.isDir());

            final String path = UUID.randomUUID().toString();
            final byte[] content = generateBytes();
            op.write(path, content).join();

            final Metadata metadata = op.stat(path).join();
            assertTrue(metadata.isFile());
            assertThat(metadata.contentLength).isEqualTo(content.length);
            assertThat(metadata.lastModified).isNotNull();
            assertThat(metadata.cacheControl).isNull();
            assertThat(metadata.contentDisposition).isNull();
            assertThat(metadata.contentMd5).isNull();
            assertThat(metadata.contentType).isNull();
            assertThat(metadata.contentEncoding).isNull();
            assertThat(metadata.contentLanguage).isNull();
            assertThat(metadata.etag).isNull();
            assertThat(metadata.version).isNull();
            assertThat(metadata.userMetadata).isEmpty();
            assertThat(metadata.current).isNull();
            assertThat(metadata.deleted).isFalse();

            op.delete(dir).join();
            op.delete(path).join();
        }
    }

    @Test
    public void testBlockingMetadata() {
        final ServiceConfig.Fs fs =
                ServiceConfig.Fs.builder().root(tempDir.toString()).build();

        try (final Operator op = Operator.of(fs)) {
            final String dir = UUID.randomUUID() + "/";
            op.createDir(dir);
            final Metadata dirMetadata = op.stat(dir);
            assertTrue(dirMetadata.isDir());

            final String path = UUID.randomUUID().toString();
            final byte[] content = generateBytes();
            op.write(path, content);

            final Metadata metadata = op.stat(path);
            assertTrue(metadata.isFile());
            assertThat(metadata.contentLength).isEqualTo(content.length);
            assertThat(metadata.lastModified).isNotNull();
            assertThat(metadata.cacheControl).isNull();
            assertThat(metadata.contentDisposition).isNull();
            assertThat(metadata.contentMd5).isNull();
            assertThat(metadata.contentType).isNull();
            assertThat(metadata.contentEncoding).isNull();
            assertThat(metadata.contentLanguage).isNull();
            assertThat(metadata.etag).isNull();
            assertThat(metadata.version).isNull();
            assertThat(metadata.userMetadata).isEmpty();
            assertThat(metadata.current).isNull();
            assertThat(metadata.deleted).isFalse();

            op.delete(dir);
            op.delete(path);
        }
    }

    @Test
    public void testContentHeadersAndVersionedListingMetadata() throws Exception {
        String listXml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
                + "<ListVersionsResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">"
                + "<Name>bucket</Name>"
                + "<Prefix>dir/</Prefix>"
                + "<IsTruncated>false</IsTruncated>"
                + "<Version>"
                + "<Key>dir/file</Key>"
                + "<VersionId>v2</VersionId>"
                + "<IsLatest>true</IsLatest>"
                + "<LastModified>2024-01-02T00:00:00Z</LastModified>"
                + "<ETag>&quot;etag-v2&quot;</ETag>"
                + "<Size>5</Size>"
                + "</Version>"
                + "<Version>"
                + "<Key>dir/file</Key>"
                + "<VersionId>v1</VersionId>"
                + "<IsLatest>false</IsLatest>"
                + "<LastModified>2024-01-01T00:00:00Z</LastModified>"
                + "<ETag>&quot;etag-v1&quot;</ETag>"
                + "<Size>3</Size>"
                + "</Version>"
                + "<DeleteMarker>"
                + "<Key>dir/deleted</Key>"
                + "<VersionId>dm-2</VersionId>"
                + "<IsLatest>true</IsLatest>"
                + "<LastModified>2024-01-03T00:00:00Z</LastModified>"
                + "</DeleteMarker>"
                + "<DeleteMarker>"
                + "<Key>dir/deleted</Key>"
                + "<VersionId>dm-1</VersionId>"
                + "<IsLatest>false</IsLatest>"
                + "<LastModified>2024-01-01T00:00:00Z</LastModified>"
                + "</DeleteMarker>"
                + "</ListVersionsResult>";

        HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.createContext("/", exchange -> {
            try {
                exchange.getRequestBody().close();
                if ("HEAD".equals(exchange.getRequestMethod())) {
                    exchange.getResponseHeaders().set("Content-Length", "5");
                    exchange.getResponseHeaders().set("Content-Encoding", "zstd");
                    exchange.getResponseHeaders().set("Content-Language", "en-US, zh-CN");
                    exchange.getResponseHeaders().set("ETag", "\"etag-v2\"");
                    exchange.getResponseHeaders().set("x-amz-version-id", "v2");
                    exchange.sendResponseHeaders(200, -1);
                    return;
                }
                byte[] body = listXml.getBytes(StandardCharsets.UTF_8);
                exchange.getResponseHeaders().set("Content-Type", "application/xml");
                exchange.sendResponseHeaders(200, body.length);
                exchange.getResponseBody().write(body);
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
                        .enableVersioning(true)
                        .skipSignature(true)
                        .build());
                Operator op = async.blocking()) {
            Metadata statMeta = op.stat("dir/file");
            assertThat(statMeta.getContentEncoding()).isEqualTo("zstd");
            assertThat(statMeta.getContentLanguage()).isEqualTo("en-US, zh-CN");
            assertThat(statMeta.getVersion()).isEqualTo("v2");
            assertThat(statMeta.getCurrent()).isNull();
            assertThat(statMeta.isDeleted()).isFalse();

            ListOptions options =
                    ListOptions.builder().versions(true).deleted(true).build();
            List<Entry> entries = op.list("dir/", options);
            assertThat(entries).hasSize(4);

            Metadata currentVersion = entries.get(0).getMetadata();
            assertThat(entries.get(0).getPath()).isEqualTo("dir/file");
            assertThat(currentVersion.getVersion()).isEqualTo("v2");
            assertThat(currentVersion.getCurrent()).isEqualTo(Boolean.TRUE);
            assertThat(currentVersion.isDeleted()).isFalse();

            Metadata historicalVersion = entries.get(1).getMetadata();
            assertThat(entries.get(1).getPath()).isEqualTo("dir/file");
            assertThat(historicalVersion.getVersion()).isEqualTo("v1");
            assertThat(historicalVersion.getCurrent()).isEqualTo(Boolean.FALSE);
            assertThat(historicalVersion.isDeleted()).isFalse();

            Metadata currentDeleteMarker = entries.get(2).getMetadata();
            assertThat(entries.get(2).getPath()).isEqualTo("dir/deleted");
            assertThat(currentDeleteMarker.getVersion()).isEqualTo("dm-2");
            assertThat(currentDeleteMarker.getCurrent()).isEqualTo(Boolean.TRUE);
            assertThat(currentDeleteMarker.isDeleted()).isTrue();

            Metadata historicalDeleteMarker = entries.get(3).getMetadata();
            assertThat(entries.get(3).getPath()).isEqualTo("dir/deleted");
            assertThat(historicalDeleteMarker.getVersion()).isEqualTo("dm-1");
            assertThat(historicalDeleteMarker.getCurrent()).isEqualTo(Boolean.FALSE);
            assertThat(historicalDeleteMarker.isDeleted()).isTrue();
        } finally {
            server.stop(0);
        }
    }

    @Test
    @SuppressWarnings("deprecation")
    public void testBuilderAndLegacyConstructors() {
        Metadata defaultMeta = Metadata.builder().build();
        assertThat(defaultMeta.getMode()).isEqualTo(EntryMode.UNKNOWN);
        assertThat(defaultMeta.getUserMetadata()).isEmpty();
        assertThat(defaultMeta.getCurrent()).isNull();
        assertThat(defaultMeta.isDeleted()).isFalse();

        Instant now = Instant.ofEpochSecond(1700000000L, 123456789L);
        Map<String, String> userMeta = Collections.singletonMap("author", "opendal");

        Metadata legacy9 =
                new Metadata(0, 42L, "inline", "md5-hash", "text/plain", "max-age=60", "\"etag-1\"", now, "v1");
        assertThat(legacy9.isFile()).isTrue();
        assertThat(legacy9.getContentLength()).isEqualTo(42L);
        assertThat(legacy9.getContentDisposition()).isEqualTo("inline");
        assertThat(legacy9.getContentMd5()).isEqualTo("md5-hash");
        assertThat(legacy9.getContentType()).isEqualTo("text/plain");
        assertThat(legacy9.getCacheControl()).isEqualTo("max-age=60");
        assertThat(legacy9.getEtag()).isEqualTo("\"etag-1\"");
        assertThat(legacy9.getLastModified()).isEqualTo(now);
        assertThat(legacy9.getVersion()).isEqualTo("v1");
        assertThat(legacy9.getUserMetadata()).isEmpty();
        assertThat(legacy9.getCurrent()).isNull();
        assertThat(legacy9.isDeleted()).isFalse();
        assertThat(legacy9.getContentEncoding()).isNull();
        assertThat(legacy9.getContentLanguage()).isNull();

        Metadata legacy10 = new Metadata(1, 0L, null, null, null, null, "\"etag-2\"", now, "v2", userMeta);
        assertThat(legacy10.isDir()).isTrue();
        assertThat(legacy10.getUserMetadata()).isEqualTo(userMeta);

        Metadata modified = legacy10.toBuilder()
                .current(Boolean.TRUE)
                .deleted(true)
                .contentEncoding("gzip")
                .contentLanguage("en-US")
                .build();
        assertThat(modified.isDir()).isTrue();
        assertThat(modified.getCurrent()).isEqualTo(Boolean.TRUE);
        assertThat(modified.isDeleted()).isTrue();
        assertThat(modified.getContentEncoding()).isEqualTo("gzip");
        assertThat(modified.getContentLanguage()).isEqualTo("en-US");
        assertThat(modified.getEtag()).isEqualTo("\"etag-2\"");
        assertThat(modified.getUserMetadata()).isEqualTo(userMeta);
    }
}
