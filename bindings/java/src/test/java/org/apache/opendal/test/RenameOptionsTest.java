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
import org.apache.opendal.RenameOptions;
import org.apache.opendal.test.condition.OpenDALExceptionCondition;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

public class RenameOptionsTest {
    @TempDir
    Path root;

    @Test
    void testRenameWithOptions() {
        try (AsyncOperator async = AsyncOperator.of("fs", Collections.singletonMap("root", root.toString()));
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

            RenameOptions targetOptions = op.info.capability.renameWithIfNotExists
                    ? ifNotExists
                    : RenameOptions.builder().build();
            op.rename("src-1", "dst-new", targetOptions);
            assertThat(op.read("dst-new")).isEqualTo("a".getBytes());
        }
    }
}
