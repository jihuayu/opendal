// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use std::task::Context;
use std::task::Poll;

use futures::FutureExt;
use hdfs_native::file::FileWriter;

use crate::raw::oio;
use crate::raw::oio::WriteBuf;
use crate::*;

use super::error::parse_hdfs_error;

pub struct HdfsNativeWriter {
    f: FileWriter,
}

impl HdfsNativeWriter {
    pub fn new(f: FileWriter) -> Self {
        HdfsNativeWriter { f }
    }
}

impl oio::Write for HdfsNativeWriter {
    fn poll_write(&mut self, cx: &mut Context<'_>, bs: &dyn WriteBuf) -> Poll<Result<usize>> {
        println!("HdfsNativeWriter::poll_write{}", bs.remaining());
        Box::pin(self.f.write(bs.bytes(bs.remaining())))
            .poll_unpin(cx)
            .map_err(parse_hdfs_error)
    }

    fn poll_close(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        println!("HdfsNativeWriter::poll_close");
        Box::pin(self.f.close())
            .poll_unpin(cx)
            .map_err(parse_hdfs_error)
    }

    fn poll_abort(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Err(Error::new(
            ErrorKind::Unsupported,
            "HdfsNativeWriter doesn't support abort",
        )))
    }
}
