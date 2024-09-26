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
use hdfs_native::client::ListStatusIterator;

use crate::raw::oio;
use crate::raw::oio::Entry;
use crate::*;

use self::raw::parse_datetime_from_from_timestamp;

use super::error::parse_hdfs_error;

pub struct HdfsNativeLister {
    iter: ListStatusIterator,
}

impl HdfsNativeLister {
    pub fn new(iter: ListStatusIterator) -> Self {
        HdfsNativeLister { iter }
    }
}

impl oio::List for HdfsNativeLister {
    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Result<Option<Entry>>> {
        println!("HdfsNativeLister::poll_next");
        let item = self.iter.next();
        let ret = async move {
            if let Some(status_result) = item.await {
                let status = status_result.map_err(parse_hdfs_error)?;
                println!("ok");
                let mode = if status.isdir {
                    EntryMode::DIR
                } else {
                    EntryMode::FILE
                };
    
                let mut metadata = Metadata::new(mode);
                metadata
                    .set_last_modified(parse_datetime_from_from_timestamp(
                        status.modification_time as i64,
                    )?)
                    .set_content_length(status.length as u64);
                let path = status.path;
                if status.isdir {
                    // Make sure we are returning the correct path.
                   return Ok(Some(oio::Entry::new(&format!("{path}/"), Metadata::new(EntryMode::DIR))));
                } else {
                    return Ok(Some(Entry::new(&path, metadata)));
                };
            }
            println!("none");
            Ok(None)
        };
        Box::pin(ret).poll_unpin(cx)
    }
}
