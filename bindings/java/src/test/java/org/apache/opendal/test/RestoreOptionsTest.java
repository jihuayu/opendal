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
import java.util.Collections;
import org.apache.opendal.AsyncOperator;
import org.apache.opendal.OpenDALException.Code;
import org.apache.opendal.Operator;
import org.apache.opendal.RestoreOptions;
import org.apache.opendal.test.condition.OpenDALExceptionCondition;
import org.junit.jupiter.api.Test;

public class RestoreOptionsTest {

    @Test
    void testRestoreUnsupportedOnMemory() {
        try (AsyncOperator async = AsyncOperator.of("memory", Collections.emptyMap());
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
}
