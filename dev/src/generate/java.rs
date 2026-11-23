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

use crate::generate::parser::{Config, ConfigType, Services, sorted_services};
use anyhow::Result;
use minijinja::value::ViaDeserialize;
use minijinja::{Environment, context};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use super::capability;
use super::metadata::{self, MetadataKind};

/// Fields present in the legacy 46-parameter public `Capability` constructor.
const LEGACY_46_CAPABILITY_FIELDS: &[&str] = &[
    "stat",
    "stat_with_if_match",
    "stat_with_if_none_match",
    "stat_with_if_version_match",
    "stat_with_if_version_not_match",
    "stat_with_if_modified_since",
    "stat_with_if_unmodified_since",
    "stat_with_version",
    "read",
    "read_with_if_match",
    "read_with_if_none_match",
    "read_with_override_cache_control",
    "read_with_override_content_disposition",
    "read_with_override_content_type",
    "write",
    "write_can_multi",
    "write_can_append",
    "write_with_content_type",
    "write_with_content_disposition",
    "write_with_cache_control",
    "write_with_if_match",
    "write_with_if_none_match",
    "write_with_if_not_exists",
    "write_with_user_metadata",
    "write_multi_max_size",
    "write_multi_min_size",
    "create_dir",
    "delete",
    "copy",
    "rename",
    "list",
    "list_with_limit",
    "list_with_start_after",
    "list_with_recursive",
    "list_with_versions",
    "list_with_deleted",
    "presign",
    "presign_read",
    "presign_stat",
    "presign_write",
    "shared",
    "read_with_version",
    "read_with_if_version_match",
    "read_with_if_version_not_match",
    "read_with_if_modified_since",
    "read_with_if_unmodified_since",
];

/// Fields present in the legacy 10-parameter public `Metadata` constructor.
const LEGACY_10_METADATA_FIELDS: &[&str] = &[
    "mode",
    "content_length",
    "content_disposition",
    "content_md5",
    "content_type",
    "cache_control",
    "etag",
    "last_modified",
    "version",
    "user_metadata",
];

/// Render model handed to the Java capability templates.
#[derive(Serialize)]
struct CapabilityField {
    /// snake_case field name in core `opendal::Capability`.
    name: String,
    /// camelCase field name in Java `org.apache.opendal.Capability`.
    camel: String,
    /// Plain `bool` flag when true, `Option<usize>` limit when false.
    is_bool: bool,
    /// Whether the field is part of the legacy 46-parameter public constructor.
    in_legacy_46: bool,
    /// Single-line Javadoc text from core.
    doc: String,
}

/// Render model handed to the Java metadata templates.
#[derive(Serialize)]
struct MetadataField {
    /// camelCase field name in Java `org.apache.opendal.Metadata`.
    camel: String,
    /// Java field type.
    java_type: String,
    /// JNI type descriptor for the constructor parameter.
    jni_sig: String,
    /// Optional Rust statement binding the converted JNI local before `env.new_object`.
    rs_bind: String,
    /// Rust `JValue` argument passed to `env.new_object`.
    rs_jvalue: String,
    /// Java initializer expression used in the legacy 10-parameter constructor.
    legacy_expr: String,
    /// Single-line Javadoc text from core.
    doc: String,
}

fn enabled_service(srv: &str) -> bool {
    match srv {
        // not enabled in bindings/java/Cargo.toml
        "foundationdb" | "ftp" | "git" | "hdfs" | "rocksdb" | "smb" | "tikv" => false,
        _ => true,
    }
}

pub fn generate(workspace_dir: PathBuf, services: Services) -> Result<()> {
    generate_service_config(&workspace_dir, services)?;
    generate_capability(&workspace_dir)?;
    generate_metadata(&workspace_dir)
}

fn generate_service_config(workspace_dir: &Path, services: Services) -> Result<()> {
    let srvs = sorted_services(services, enabled_service);
    let mut env = Environment::new();
    env.add_template("java", include_str!("java.j2"))?;
    env.add_function("make_field", make_field);
    env.add_function("make_populate_map", make_populate_map);
    env.add_filter("case_java_class_name", case_java_class_name);
    let tmpl = env.get_template("java")?;

    let output =
        workspace_dir.join("bindings/java/src/main/java/org/apache/opendal/ServiceConfig.java");
    let rendered = tmpl.render(context! { srvs => srvs })?;
    fs::write(output, format!("{rendered}\n"))?;
    Ok(())
}

fn generate_capability(workspace_dir: &Path) -> Result<()> {
    let source = workspace_dir.join("core/core/src/types/capability.rs");
    let fields: Vec<CapabilityField> = capability::parse(&source)?
        .into_iter()
        .map(|f| CapabilityField {
            camel: case_java_field_name(&f.name),
            in_legacy_46: LEGACY_46_CAPABILITY_FIELDS.contains(&f.name.as_str()),
            name: f.name,
            is_bool: f.is_bool,
            doc: f.doc,
        })
        .collect();
    let jni_sig: String = fields
        .iter()
        .map(|f| if f.is_bool { 'Z' } else { 'J' })
        .collect();

    let mut env = Environment::new();
    env.add_template("capability_java", include_str!("java/capability_java.j2"))?;
    env.add_template("capability_rs", include_str!("java/capability_rs.j2"))?;

    let outputs = [
        (
            "capability_java",
            "bindings/java/src/main/java/org/apache/opendal/Capability.java",
        ),
        ("capability_rs", "bindings/java/src/capability.rs"),
    ];
    for (template, relative) in outputs {
        let tmpl = env.get_template(template)?;
        let rendered = tmpl.render(context! { fields => fields, jni_sig => jni_sig })?;
        fs::write(workspace_dir.join(relative), format!("{rendered}\n"))?;
    }

    Ok(())
}

fn generate_metadata(workspace_dir: &Path) -> Result<()> {
    let source = workspace_dir.join("core/core/src/types/metadata.rs");
    let fields: Vec<MetadataField> = metadata::parse(&source)?
        .into_iter()
        .map(metadata_field)
        .collect();

    let jni_sig: String = fields.iter().map(|f| f.jni_sig.as_str()).collect();

    let mut env = Environment::new();
    env.add_template("metadata_java", include_str!("java/metadata_java.j2"))?;
    env.add_template("metadata_rs", include_str!("java/metadata_rs.j2"))?;

    let outputs = [
        (
            "metadata_java",
            "bindings/java/src/main/java/org/apache/opendal/Metadata.java",
        ),
        ("metadata_rs", "bindings/java/src/metadata.rs"),
    ];
    for (template, relative) in outputs {
        let tmpl = env.get_template(template)?;
        let rendered = tmpl.render(context! { fields => fields, jni_sig => jni_sig })?;
        fs::write(workspace_dir.join(relative), format!("{rendered}\n"))?;
    }

    Ok(())
}

fn metadata_camel(name: &str) -> String {
    match name {
        "is_current" => "current".to_string(),
        "is_deleted" => "deleted".to_string(),
        _ => case_java_field_name(name),
    }
}

fn metadata_field(f: metadata::MetadataField) -> MetadataField {
    let name = &f.name;
    let camel = metadata_camel(name);
    let (java_type, jni_sig, rs_bind, rs_jvalue, default_expr) = match f.kind {
        MetadataKind::Mode => (
            "EntryMode",
            "Lorg/apache/opendal/Metadata$EntryMode;",
            format!("let {name} = convert::entry_mode_to_jobject(env, metadata.{name}())?;"),
            format!("JValue::Object(&{name})"),
            "EntryMode.UNKNOWN",
        ),
        MetadataKind::Bool => (
            "boolean",
            "Z",
            String::new(),
            format!("JValue::Bool(metadata.{name}())"),
            "false",
        ),
        MetadataKind::U64 => (
            "long",
            "J",
            String::new(),
            format!("JValue::Long(metadata.{name}() as jlong)"),
            "0L",
        ),
        MetadataKind::OptionBool => (
            "Boolean",
            "Ljava/lang/Boolean;",
            format!("let {name} = convert::option_bool_to_jboolean(env, metadata.{name}())?;"),
            format!("JValue::Object(&{name})"),
            "null",
        ),
        MetadataKind::OptionStr => (
            "String",
            "Ljava/lang/String;",
            format!("let {name} = convert::string_to_jstring(env, metadata.{name}())?;"),
            format!("JValue::Object(&{name})"),
            "null",
        ),
        MetadataKind::OptionTimestamp => (
            "Instant",
            "Ljava/time/Instant;",
            format!("let {name} = convert::timestamp_to_jinstant(env, metadata.{name}())?;"),
            format!("JValue::Object(&{name})"),
            "null",
        ),
        MetadataKind::OptionUserMetadata => (
            "Map<String, String>",
            "Ljava/util/Map;",
            format!(
                "let {name} =\n        convert::hashmap_to_jmap(env, metadata.{name}().into_iter().flatten())?;"
            ),
            format!("JValue::Object(&{name})"),
            "Collections.emptyMap()",
        ),
    };

    let legacy_expr = if name == "mode" {
        "EntryMode.of(mode)".to_string()
    } else if LEGACY_10_METADATA_FIELDS.contains(&name.as_str()) {
        camel.clone()
    } else {
        default_expr.to_string()
    };

    MetadataField {
        java_type: java_type.to_string(),
        jni_sig: jni_sig.to_string(),
        rs_bind,
        rs_jvalue,
        legacy_expr,
        camel,
        doc: f.doc,
    }
}

fn case_java_class_name(s: &str) -> String {
    heck::AsUpperCamelCase(s).to_string()
}

fn case_java_field_name(s: &str) -> String {
    heck::AsLowerCamelCase(s).to_string()
}

fn make_field(field: ViaDeserialize<Config>) -> Result<String, minijinja::Error> {
    use std::fmt::Write;

    let mut result = String::new();
    let w = &mut result;

    // write comment
    let html = markdown::to_html(&field.comments);
    writeln!(w, "/**")?;
    for line in html.lines() {
        writeln!(w, " * {line}")?;
    }
    if let Some(deprecated) = &field.deprecated {
        writeln!(w, " *")?;
        writeln!(w, " * @deprecated {}", deprecated.note)?;
    }
    writeln!(w, " */")?;

    // write field definition
    let field_type = if field.optional {
        match field.value {
            ConfigType::Bool => "Boolean",
            ConfigType::String => "String",
            ConfigType::Duration => "Duration",
            ConfigType::Usize | ConfigType::U64 | ConfigType::I64 => "Long",
            ConfigType::U32 | ConfigType::U16 => "Integer",
            ConfigType::Vec => "List<String>",
            ConfigType::HashMap => "Map<String, String>",
        }
    } else {
        match field.value {
            ConfigType::Bool => "boolean",
            ConfigType::String => "@NonNull String",
            ConfigType::Duration => "@NonNull Duration",
            ConfigType::Usize | ConfigType::U64 | ConfigType::I64 => "long",
            ConfigType::U32 | ConfigType::U16 => "int",
            ConfigType::Vec => "@NonNull List<String>",
            ConfigType::HashMap => "@NonNull Map<String, String>",
        }
    };

    writeln!(
        w,
        "public final {} {};",
        field_type,
        case_java_field_name(&field.name)
    )?;
    Ok(result)
}

fn make_populate_map(field: ViaDeserialize<Config>) -> Result<String, minijinja::Error> {
    use std::fmt::Write;

    let mut result = String::new();
    let w = &mut result;

    let name = case_java_field_name(&field.name);
    let value = match field.value {
        ConfigType::Usize
        | ConfigType::U64
        | ConfigType::I64
        | ConfigType::Bool
        | ConfigType::U32
        | ConfigType::U16 => Some(format!("String.valueOf({name})")),
        ConfigType::String => Some(name.clone()),
        ConfigType::Duration => Some(format!("{name}.toString()")),
        ConfigType::Vec => Some(format!("String.join(\",\", {name})")),
        ConfigType::HashMap => None,
    };

    let indent = if field.optional { "    " } else { "" };
    if field.optional {
        writeln!(w, "if ({name} != null) {{")?;
    }
    if let Some(value) = value {
        let populate = format!("map.put(\"{}\", {value});", field.name);
        // java.j2 adds 12 columns; keep calls within the formatter's 120-column layout.
        if 12 + indent.len() + populate.len() > 120 {
            writeln!(w, "{indent}map.put(")?;
            writeln!(w, "{indent}        \"{}\",", field.name)?;
            writeln!(w, "{indent}        {value});")?;
        } else {
            writeln!(w, "{indent}{populate}")?;
        }
    } else {
        writeln!(w, "{indent}map.putAll({name});")?;
    }
    if field.optional {
        writeln!(w, "}}")?;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_46_capability_fields_exist_in_core() {
        let path = crate::workspace_dir().join("core/core/src/types/capability.rs");
        let fields = capability::parse(&path).expect("core capability must parse");

        assert_eq!(LEGACY_46_CAPABILITY_FIELDS.len(), 46);
        for &legacy in LEGACY_46_CAPABILITY_FIELDS {
            assert!(
                fields.iter().any(|f| f.name == legacy),
                "legacy capability field `{legacy}` is missing from core Capability"
            );
        }
    }

    #[test]
    fn test_legacy_10_metadata_fields_exist_in_core() {
        let path = crate::workspace_dir().join("core/core/src/types/metadata.rs");
        let fields = metadata::parse(&path).expect("core metadata must parse");

        assert_eq!(LEGACY_10_METADATA_FIELDS.len(), 10);
        for &legacy in LEGACY_10_METADATA_FIELDS {
            assert!(
                fields.iter().any(|f| f.name == legacy),
                "legacy metadata field `{legacy}` is missing from core Metadata"
            );
        }
    }
}
