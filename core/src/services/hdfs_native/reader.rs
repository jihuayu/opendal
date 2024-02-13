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

use std::io::SeekFrom;
use std::task::Context;
use std::task::Poll;

use bytes::Bytes;
use futures::FutureExt;
use hdfs_native::file::FileReader;

use crate::raw::oio::Read;
use crate::*;

use super::error::parse_hdfs_error;

pub struct HdfsNativeReader {
    f: FileReader,
}

impl HdfsNativeReader {
    pub fn new(f: FileReader) -> Self {
        HdfsNativeReader { f }
    }
}

impl Read for HdfsNativeReader {
    fn poll_read(&mut self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>> {
       Box::pin(self.f.read_buf(buf)).poll_unpin(cx).map_err(parse_hdfs_error)
    }

    fn poll_seek(&mut self, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<Result<u64>> {
        let (_, _) = (cx, pos);

        Poll::Ready(Err(Error::new(
            ErrorKind::Unsupported,
            "HdfsNativeReader doesn't support seeking",
        )))
    }

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        let _ = cx;

        Poll::Ready(Some(Err(Error::new(
            ErrorKind::Unsupported,
            "HdfsNativeReader doesn't support iterating",
        ))))
    }
}
