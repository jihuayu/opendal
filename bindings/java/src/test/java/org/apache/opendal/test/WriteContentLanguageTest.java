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
import java.nio.file.Path;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;
import org.apache.opendal.AsyncOperator;
import org.apache.opendal.OpenDALException.Code;
import org.apache.opendal.Operator;
import org.apache.opendal.ServiceConfig;
import org.apache.opendal.WriteOptions;
import org.apache.opendal.layer.CapabilityCheckLayer;
import org.apache.opendal.test.condition.OpenDALExceptionCondition;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

public class WriteContentLanguageTest {

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
}
