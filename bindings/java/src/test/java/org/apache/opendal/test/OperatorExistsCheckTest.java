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
import java.nio.file.Path;
import java.util.Collections;
import org.apache.opendal.AsyncOperator;
import org.apache.opendal.OpenDALException.Code;
import org.apache.opendal.Operator;
import org.apache.opendal.layer.CapabilityOverrideLayer;
import org.apache.opendal.test.condition.OpenDALExceptionCondition;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

public class OperatorExistsCheckTest {
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
    void testExistsPropagatesNonNotFoundErrors() {
        try (AsyncOperator base = newOperator("memory");
                AsyncOperator noStat = base.layer(new CapabilityOverrideLayer("stat=false"));
                Operator op = noStat.blocking()) {
            assertThatThrownBy(() -> op.exists("file")).is(OpenDALExceptionCondition.ofSync(Code.Unsupported));
            assertThatThrownBy(noStat.exists("file")::join).is(OpenDALExceptionCondition.ofAsync(Code.Unsupported));
        }
    }

    private AsyncOperator newOperator(String scheme) {
        return AsyncOperator.of(
                scheme,
                "fs".equals(scheme) ? Collections.singletonMap("root", root.toString()) : Collections.emptyMap());
    }
}
