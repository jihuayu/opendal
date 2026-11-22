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
use anyhow::{Result, bail};
use minijinja::value::ViaDeserialize;
use minijinja::{Environment, context};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use super::capability;
use super::core_options::{self, OptionField, OptionKind, OptionsStruct};
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

/// The core option structs the Java binding exposes, with the snake_case
/// name used in the generated `make_<fn_name>_options` JNI converter.
const EXPOSED_OPTIONS: &[(&str, &str)] = &[
    ("DeleteOptions", "delete"),
    ("ListOptions", "list"),
    ("ReadOptions", "read"),
    ("ReaderOptions", "reader"),
    ("StatOptions", "stat"),
    ("WriteOptions", "write"),
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

/// Render model for one public `*Options` Java class and its JNI parser.
#[derive(Serialize)]
struct OptionsClass {
    /// The class name, identical to the core struct name, e.g. `ReadOptions`.
    name: String,
    /// snake_case operation name for `make_{fn_name}_options`, e.g. `read`.
    fn_name: String,
    /// Class Javadoc summary line.
    summary: String,
    /// Ordered Java imports before `lombok.*`.
    imports: Vec<String>,
    /// The public fields in core field order.
    properties: Vec<OptionProperty>,
    /// Field initializers in `opendal::options::{name} { ... }`.
    rs_fields: Vec<OptionRsField>,
}

/// One public field of a Java `*Options` class.
#[derive(Serialize)]
struct OptionProperty {
    /// camelCase field name in Java.
    camel: String,
    /// The Java field type, e.g. `String`, `boolean`, `int`, `long`, `Instant`.
    java_type: String,
    /// Default initializer after ` = ` when `@Builder.Default` is needed, empty otherwise.
    default_value: String,
    /// Field Javadoc summary line.
    summary: String,
    /// `@see` targets of the capabilities the core doc refers to.
    see_also: Vec<String>,
}

/// One field initialization in `opendal::options::{name} { ... }`.
#[derive(Serialize)]
struct OptionRsField {
    /// snake_case field name in core.
    name: String,
    /// Rust expression producing the core field value from `env` and `options`.
    expr: String,
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
    generate_metadata(&workspace_dir)?;
    generate_options(&workspace_dir)
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

fn generate_options(workspace_dir: &Path) -> Result<()> {
    let source = workspace_dir.join("core/core/src/types/options.rs");
    let structs = core_options::parse(&source)?;

    let mut classes = Vec::with_capacity(EXPOSED_OPTIONS.len());
    for (name, fn_name) in EXPOSED_OPTIONS {
        let Some(options) = structs.iter().find(|s| s.name == *name) else {
            bail!("options struct {name} not found in {}", source.display());
        };
        classes.push(options_class(options, fn_name));
    }

    let mut env = Environment::new();
    env.add_template("options_rs", include_str!("java/options_rs.j2"))?;
    env.add_template("options_java", include_str!("java/options_java.j2"))?;

    let rendered = env
        .get_template("options_rs")?
        .render(context! { classes => classes })?;
    fs::write(
        workspace_dir.join("bindings/java/src/options.rs"),
        format!("{rendered}\n"),
    )?;

    let tmpl = env.get_template("options_java")?;
    for class in &classes {
        let rendered = tmpl.render(context! { class => class })?;
        let output = workspace_dir.join(format!(
            "bindings/java/src/main/java/org/apache/opendal/{}.java",
            class.name
        ));
        fs::write(output, format!("{rendered}\n"))?;
    }

    Ok(())
}

fn options_class(options: &OptionsStruct, fn_name: &str) -> OptionsClass {
    let ctx = DocContext {
        fields: &options.fields,
    };

    let mut imports = Vec::new();
    let mut properties = Vec::new();
    let mut rs_fields = Vec::new();

    for f in &options.fields {
        let key = f.name.as_str();
        let camel = case_java_field_name(key);
        let property = |java_type: &str, default_value: &str| OptionProperty {
            camel: camel.clone(),
            java_type: java_type.to_string(),
            default_value: default_value.to_string(),
            summary: ctx.summary(&f.doc, Some(key)),
            see_also: capability_refs(&f.doc),
        };
        let call_3 = |func: &str| {
            let single = format!("{func}(env, options, \"{camel}\")?");
            if 8 + key.len() + 2 + single.len() + 1 > 100 {
                format!(
                    "{func}(\n            env,\n            options,\n            \"{camel}\",\n        )?"
                )
            } else {
                single
            }
        };

        match f.kind {
            OptionKind::Bool => {
                properties.push(property("boolean", ""));
                rs_fields.push(OptionRsField {
                    name: key.to_string(),
                    expr: call_3("convert::read_bool_field"),
                });
            }
            OptionKind::Usize => {
                let (default_value, func) = if key == "prefetch" {
                    ("0", "convert::read_non_negative_jint_to_usize")
                } else {
                    ("1", "convert::read_positive_jint_to_usize")
                };
                properties.push(property("int", default_value));
                rs_fields.push(OptionRsField {
                    name: key.to_string(),
                    expr: call_3(func),
                });
            }
            OptionKind::OptionUsize => {
                let func = if key == "gap" {
                    "convert::read_non_negative_jlong_to_usize"
                } else {
                    "convert::read_jlong_field_to_usize"
                };
                properties.push(property("long", "-1L"));
                rs_fields.push(OptionRsField {
                    name: key.to_string(),
                    expr: call_3(func),
                });
            }
            OptionKind::OptionU64 => {
                properties.push(property("long", "-1L"));
                rs_fields.push(OptionRsField {
                    name: key.to_string(),
                    expr: call_3("convert::read_non_negative_jlong_to_u64"),
                });
            }
            OptionKind::OptionString => {
                properties.push(property("String", ""));
                rs_fields.push(OptionRsField {
                    name: key.to_string(),
                    expr: call_3("convert::read_string_field"),
                });
            }
            OptionKind::OptionTimestamp => {
                add_import(&mut imports, "java.time.Instant");
                properties.push(property("Instant", ""));
                rs_fields.push(OptionRsField {
                    name: key.to_string(),
                    expr: call_3("convert::read_instant_field_to_timestamp"),
                });
            }
            OptionKind::OptionDuration => {
                add_import(&mut imports, "java.time.Duration");
                properties.push(property("Duration", ""));
                rs_fields.push(OptionRsField {
                    name: key.to_string(),
                    expr: call_3("convert::read_duration_field"),
                });
            }
            OptionKind::OptionStringMap => {
                add_import(&mut imports, "java.util.Map");
                properties.push(property("Map<String, String>", ""));
                rs_fields.push(OptionRsField {
                    name: key.to_string(),
                    expr: call_3("convert::read_map_field"),
                });
            }
            OptionKind::OptionMetadata => {
                properties.push(property("Metadata", ""));
                rs_fields.push(OptionRsField {
                    name: key.to_string(),
                    expr: call_3("convert::read_metadata_condition_field"),
                });
            }
            OptionKind::BytesRange => {
                let range = |camel: &str, default_value: &str, summary: &str| OptionProperty {
                    camel: camel.to_string(),
                    java_type: "long".to_string(),
                    default_value: default_value.to_string(),
                    summary: summary.to_string(),
                    see_also: Vec::new(),
                };
                properties.push(range(
                    "offset",
                    "",
                    "The byte offset to start reading from. Must be non-negative; defaults to zero.",
                ));
                properties.push(range(
                    "length",
                    "-1L",
                    "The number of bytes to read from {@link #offset}. The default of {@code -1L} reads to the end.",
                ));
                rs_fields.push(OptionRsField {
                    name: key.to_string(),
                    expr: "convert::read_range_fields(env, options)?".to_string(),
                });
            }
        }
    }

    imports.sort();
    let summary = match ctx.summary(&options.doc, None) {
        s if s.is_empty() => format!("Options for {fn_name} operations."),
        s => s,
    };

    OptionsClass {
        name: options.name.clone(),
        fn_name: fn_name.to_string(),
        summary,
        imports,
        properties,
        rs_fields,
    }
}

fn add_import(imports: &mut Vec<String>, imp: &str) {
    if !imports.iter().any(|existing| existing == imp) {
        imports.push(imp.to_string());
    }
}

/// Renders rustdoc of one options struct as Javadoc text.
struct DocContext<'a> {
    /// Fields of the struct, so code spans naming sibling fields become `{@link #<field>}`.
    fields: &'a [OptionField],
}

impl DocContext<'_> {
    /// The first paragraph of a rustdoc comment as a single line of Javadoc text.
    fn summary(&self, lines: &[String], current_field: Option<&str>) -> String {
        let paragraph = lines
            .iter()
            .map(|line| line.trim())
            .take_while(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        self.inline(&paragraph, current_field)
    }

    /// Translate inline rustdoc markup: intra-doc links, Markdown links, and
    /// code spans. Everything else is HTML-escaped text.
    fn inline(&self, text: &str, current_field: Option<&str>) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(pos) = rest.find(['[', '`']) {
            out.push_str(&html_escape(&rest[..pos]));
            rest = &rest[pos..];

            if let Some(code) = rest.strip_prefix('`') {
                match code.find('`') {
                    Some(end) => {
                        out.push_str(&self.code(&code[..end], current_field));
                        rest = &code[end + 1..];
                    }
                    None => {
                        out.push('`');
                        rest = code;
                    }
                }
                continue;
            }

            let Some(close) = rest.find(']') else {
                out.push('[');
                rest = &rest[1..];
                continue;
            };
            let label = &rest[1..close];
            let after = &rest[close + 1..];
            if let Some(target) = after.strip_prefix('(')
                && let Some(end) = target.find(')')
            {
                let url = &target[..end];
                if url.starts_with("http://") || url.starts_with("https://") {
                    out.push_str(&format!(
                        "<a href=\"{}\">{}</a>",
                        html_escape(url),
                        self.inline(label, current_field)
                    ));
                } else {
                    out.push_str(&self.path(url, current_field));
                }
                rest = &target[end + 1..];
            } else if let Some(target) = after.strip_prefix('[')
                && let Some(end) = target.find(']')
            {
                out.push_str(&self.inline(label, current_field));
                rest = &target[end + 1..];
            } else if let Some(path) = label.strip_prefix('`').and_then(|l| l.strip_suffix('`')) {
                out.push_str(&self.path(path, current_field));
                rest = after;
            } else {
                out.push_str(&self.inline(label, current_field));
                rest = after;
            }
        }
        out.push_str(&html_escape(rest));
        out
    }

    /// An intra-doc link target, e.g. `crate::Capability::read_with_version`.
    fn path(&self, path: &str, current_field: Option<&str>) -> String {
        let path = path.strip_prefix("crate::").unwrap_or(path);
        let segments: Vec<&str> = path.split("::").collect();
        match segments.as_slice() {
            ["Capability", field] => {
                format!("{{@link Capability#{}}}", case_java_field_name(field))
            }
            ["ErrorKind", variant] => format!("{{@link OpenDALException.Code#{variant}}}"),
            ["Metadata", field] => {
                format!("{{@link Metadata#{}}}", metadata_camel(field))
            }
            ["Self", field] => self.code(field, current_field),
            [ty] if EXPOSED_OPTIONS.iter().any(|(name, _)| name == ty) => {
                format!("{{@link {ty}}}")
            }
            [ty, field] if EXPOSED_OPTIONS.iter().any(|(name, _)| name == ty) => {
                format!("{{@link {ty}#{}}}", case_java_field_name(field))
            }
            _ => format!("{{@code {path}}}"),
        }
    }

    /// A code span: a sibling field becomes a `{@link #<field>}` reference.
    fn code(&self, code: &str, current_field: Option<&str>) -> String {
        if current_field != Some(code)
            && self
                .fields
                .iter()
                .any(|f| f.name == code && f.kind != OptionKind::BytesRange)
        {
            format!("{{@link #{}}}", case_java_field_name(code))
        } else {
            format!("{{@code {code}}}")
        }
    }
}

/// The `@see` targets of every `Capability` field a rustdoc comment links to,
/// in order of first mention.
fn capability_refs(lines: &[String]) -> Vec<String> {
    const MARKER: &str = "crate::Capability::";

    let mut refs = Vec::new();
    for line in lines {
        let mut rest = line.as_str();
        while let Some(pos) = rest.find(MARKER) {
            rest = &rest[pos + MARKER.len()..];
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            let cref = format!("Capability#{}", case_java_field_name(&rest[..end]));
            if !refs.contains(&cref) {
                refs.push(cref);
            }
        }
    }
    refs
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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

    #[test]
    fn test_exposed_options_exist_in_core() {
        let path = crate::workspace_dir().join("core/core/src/types/options.rs");
        let structs = core_options::parse(&path).expect("core options must parse");

        for (name, fn_name) in EXPOSED_OPTIONS {
            let options = structs
                .iter()
                .find(|s| s.name == *name)
                .unwrap_or_else(|| panic!("exposed options struct `{name}` is missing from core"));
            let class = options_class(options, fn_name);
            assert!(!class.properties.is_empty());
            assert_eq!(class.rs_fields.len(), options.fields.len());
        }
    }
}
