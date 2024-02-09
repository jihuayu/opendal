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

use std::collections::HashMap;
use std::fmt::Debug;

use async_trait::async_trait;
use serde::Deserialize;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;

use crate::raw::adapters::kv;
use crate::raw::*;
use crate::*;

/// Config for surreal services support.
#[derive(Default, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct SurrealdbConfig {
    root: Option<String>,

    url: Option<String>,

    auth_type: Option<String>,
    username: Option<String>,
    password: Option<String>,
    token: Option<String>,

    namespace: Option<String>,
    database: Option<String>,
    table: Option<String>,
    key_field: Option<String>,
    value_field: Option<String>,
}

impl Debug for SurrealdbConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("SurrealdbConfig");

        d.field("root", &self.root)
            .field("url", &self.url)
            .field("username", &self.username)
            .field("password", &self.password)
            .field("database", &self.database)
            .field("namespace", &self.namespace)
            .field("table", &self.table)
            .field("key_field", &self.key_field)
            .field("value_field", &self.value_field)
            .finish()
    }
}

#[doc = include_str!("docs.md")]
#[derive(Default)]
pub struct SurrealdbBuilder {
    config: SurrealdbConfig,
}

impl Debug for SurrealdbBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("SurrealdbBuilder");

        d.field("config", &self.config).finish()
    }
}

impl SurrealdbBuilder {
    /// set the working directory, all operations will be performed under it.
    ///
    /// default: "/"
    pub fn root(&mut self, root: &str) -> &mut Self {
        if !root.is_empty() {
            self.config.root = Some(root.to_string());
        }
        self
    }

    /// Set the table name of the surreal service to read/write.
    pub fn table(&mut self, table: &str) -> &mut Self {
        if !table.is_empty() {
            self.config.table = Some(table.to_string());
        }
        self
    }

    /// Set the key field name of the surreal service to read/write.
    ///
    /// Default to `key` if not specified.
    pub fn key_field(&mut self, key_field: &str) -> &mut Self {
        if !key_field.is_empty() {
            self.config.key_field = Some(key_field.to_string());
        }
        self
    }

    /// Set the value field name of the surreal service to read/write.
    ///
    /// Default to `value` if not specified.
    pub fn value_field(&mut self, value_field: &str) -> &mut Self {
        if !value_field.is_empty() {
            self.config.value_field = Some(value_field.to_string());
        }
        self
    }

    pub fn url(&mut self, url: &str) -> &mut Self {
        if !url.is_empty() {
            self.config.url = Some(url.to_string());
        }
        self
    }

    pub fn username(&mut self, username: &str) -> &mut Self {
        if !username.is_empty() {
            self.config.username = Some(username.to_string());
        }
        self
    }

    pub fn password(&mut self, password: &str) -> &mut Self {
        if !password.is_empty() {
            self.config.password = Some(password.to_string());
        }
        self
    }

    pub fn namespace(&mut self, namespace: &str) -> &mut Self {
        if !namespace.is_empty() {
            self.config.namespace = Some(namespace.to_string());
        }
        self
    }

    pub fn database(&mut self, database: &str) -> &mut Self {
        if !database.is_empty() {
            self.config.database = Some(database.to_string());
        }
        self
    }
}

impl Builder for SurrealdbBuilder {
    const SCHEME: Scheme = Scheme::Surreal;
    type Accessor = SurrealBackend;

    fn from_map(map: HashMap<String, String>) -> Self {
        let config = SurrealdbConfig::deserialize(ConfigDeserializer::new(map))
            .expect("config deserialize must succeed");

        SurrealdbBuilder { config }
    }

    fn build(&mut self) -> Result<Self::Accessor> {
        let url = match self.config.url.clone() {
            Some(v) => v,
            None => {
                return Err(Error::new(ErrorKind::ConfigInvalid, "url is empty")
                    .with_context("service", Scheme::Surreal))
            }
        };

        let table = match self.config.table.clone() {
            Some(v) => v,
            None => {
                return Err(Error::new(ErrorKind::ConfigInvalid, "table is empty")
                    .with_context("service", Scheme::Surreal))
            }
        };
        let key_field = match self.config.key_field.clone() {
            Some(v) => v,
            None => "key".to_string(),
        };
        let value_field = match self.config.value_field.clone() {
            Some(v) => v,
            None => "value".to_string(),
        };
        let root = normalize_root(
            self.config
                .root
                .clone()
                .unwrap_or_else(|| "/".to_string())
                .as_str(),
        );
        let x = create_db();
        let db = futures::executor::block_on(x);
        if db.is_err() {
            return Err(Error::new(ErrorKind::ConfigInvalid, "table is empty")
                .with_context("service", Scheme::Surreal));
        }

        Ok(SurrealBackend::new(Adapter {
            db: db.unwrap(),
            table,
            key_field,
            value_field,
        })
        .with_root(&root))
    }
}

async fn create_db() -> surrealdb::Result<Surreal<Client>> {
    let db = Surreal::new::<Ws>("127.0.0.1:8000").await?;
    db.signin(Root {
        username: "root",
        password: "root",
    })
    .await?;
    db.use_ns("test").use_db("test").await?;
    return Ok(db);
}

/// Backend for Surreal service
pub type SurrealBackend = kv::Backend<Adapter>;

#[derive(Clone)]
pub struct Adapter {
    db: Surreal<Client>,

    table: String,
    key_field: String,
    value_field: String,
}

impl Debug for Adapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Adapter")
            .field("connection_pool", &self.db)
            .field("table", &self.table)
            .field("key_field", &self.key_field)
            .field("value_field", &self.value_field)
            .finish()
    }
}

#[async_trait]
impl kv::Adapter for Adapter {
    fn metadata(&self) -> kv::Metadata {
        kv::Metadata::new(
            Scheme::Surreal,
            &self.table,
            Capability {
                read: true,
                write: true,
                ..Default::default()
            },
        )
    }

    async fn get(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let res: Option<Vec<u8>> = self
            .db
            .select((&self.table, path))
            .await
            .map_err(parse_surreal_error)?;
        Ok(res)
    }

    async fn set(&self, path: &str, value: &[u8]) -> Result<()> {
        let _: Option<Vec<u8>> = self
            .db
            .create((&self.table, path))
            .content(value)
            .await
            .map_err(parse_surreal_error)?;
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let _: Option<Vec<u8>> = self
            .db
            .delete((&self.table, path))
            .await
            .map_err(parse_surreal_error)?;
        Ok(())
    }
}

fn parse_surreal_error(err: surrealdb::Error) -> Error {
    Error::new(ErrorKind::Unexpected, "unhandled error from surreal").set_source(err)
}
