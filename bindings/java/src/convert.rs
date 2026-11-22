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

use crate::Result;
use jni::Env;
use jni::jni_sig;
use jni::jni_str;
use jni::objects::JByteArray;
use jni::objects::JMap;
use jni::objects::JObject;
use jni::objects::JString;
use jni::objects::JValue;
use jni::strings::JNIString;
use jni::sys::jlong;
use opendal::{EntryMode, Error, ErrorKind, Metadata, MetadataBuilder};
use std::collections::HashMap;
use std::ops::Bound;

pub(crate) fn usize_to_jlong(n: Option<usize>) -> jlong {
    // usize is always >= 0, so we can use -1 to identify the empty value.
    n.map_or(-1, |v| v as jlong)
}

pub(crate) fn entry_mode_to_jobject<'a>(env: &mut Env<'a>, mode: EntryMode) -> Result<JObject<'a>> {
    let code = match mode {
        EntryMode::FILE => 0,
        EntryMode::DIR => 1,
        EntryMode::Unknown => 2,
    };
    Ok(env
        .call_static_method(
            jni_str!("org/apache/opendal/Metadata$EntryMode"),
            jni_str!("of"),
            jni_sig!("(I)Lorg/apache/opendal/Metadata$EntryMode;"),
            &[JValue::Int(code)],
        )?
        .l()?)
}

pub(crate) fn option_bool_to_jboolean<'a>(
    env: &mut Env<'a>,
    value: Option<bool>,
) -> Result<JObject<'a>> {
    value.map_or_else(
        || Ok(JObject::null()),
        |v| {
            Ok(env
                .call_static_method(
                    jni_str!("java/lang/Boolean"),
                    jni_str!("valueOf"),
                    jni_sig!("(Z)Ljava/lang/Boolean;"),
                    &[JValue::Bool(v)],
                )?
                .l()?)
        },
    )
}

pub(crate) fn timestamp_to_jinstant<'a>(
    env: &mut Env<'a>,
    ts: Option<opendal::raw::Timestamp>,
) -> Result<JObject<'a>> {
    ts.map_or_else(
        || Ok(JObject::null()),
        |v| {
            Ok(env
                .call_static_method(
                    jni_str!("java/time/Instant"),
                    jni_str!("ofEpochSecond"),
                    jni_sig!("(JJ)Ljava/time/Instant;"),
                    &[
                        JValue::Long(v.into_inner().as_second()),
                        JValue::Long(v.into_inner().subsec_nanosecond() as jlong),
                    ],
                )?
                .l()?)
        },
    )
}

pub(crate) fn jmap_to_hashmap(env: &mut Env, params: &JObject) -> Result<HashMap<String, String>> {
    let map = env.new_cast_local_ref::<JMap>(params)?;
    let mut iter = map.iter(env)?;

    let mut result: HashMap<String, String> = HashMap::new();
    while let Some(entry) = iter.next(env)? {
        let k = entry.key(env)?;
        let v = entry.value(env)?;
        // SAFETY: a `java.util.Map<String, String>` only yields String keys and values.
        let k = unsafe { JString::from_raw(env, k.into_raw()) };
        let v = unsafe { JString::from_raw(env, v.into_raw()) };
        result.insert(k.mutf8_chars(env)?.into(), v.mutf8_chars(env)?.into());
    }

    Ok(result)
}

pub(crate) fn hashmap_to_jmap<'a, K: AsRef<str>, V: AsRef<str>>(
    env: &mut Env<'a>,
    map: impl IntoIterator<Item = (K, V)>,
) -> Result<JObject<'a>> {
    let map_object = env.new_object(jni_str!("java/util/HashMap"), jni_sig!("()V"), &[])?;
    let jmap = env.new_cast_local_ref::<JMap>(&map_object)?;
    for (k, v) in map {
        let key = env.new_string(k.as_ref())?;
        let value = env.new_string(v.as_ref())?;
        jmap.put(env, &key, &value)?;
    }
    Ok(map_object)
}

pub(crate) fn string_to_jstring<'a>(env: &mut Env<'a>, s: Option<&str>) -> Result<JObject<'a>> {
    s.map_or_else(|| Ok(JObject::null()), |v| Ok(env.new_string(v)?.into()))
}

pub(crate) fn read_bool_field(env: &mut Env<'_>, obj: &JObject, field: &str) -> Result<bool> {
    Ok(env
        .get_field(obj, JNIString::new(field), jni_sig!("Z"))?
        .z()?)
}

pub(crate) fn read_int64_field(env: &mut Env<'_>, obj: &JObject, field: &str) -> Result<i64> {
    Ok(env
        .get_field(obj, JNIString::new(field), jni_sig!("J"))?
        .j()?)
}

pub(crate) fn read_int_field(env: &mut Env<'_>, obj: &JObject, field: &str) -> Result<i32> {
    Ok(env
        .get_field(obj, JNIString::new(field), jni_sig!("I"))?
        .i()?)
}

pub(crate) fn read_positive_jint_to_usize(
    env: &mut Env<'_>,
    obj: &JObject,
    field: &str,
) -> Result<usize> {
    let value = read_int_field(env, obj, field)?;
    if value <= 0 {
        return Err(Error::new(
            ErrorKind::ConfigInvalid,
            format!("{field} must be positive"),
        )
        .into());
    }
    Ok(usize::try_from(value)
        .map_err(|_| Error::new(ErrorKind::ConfigInvalid, format!("{field} is too large")))?)
}

pub(crate) fn read_non_negative_jint_to_usize(
    env: &mut Env<'_>,
    obj: &JObject,
    field: &str,
) -> Result<usize> {
    let value = read_int_field(env, obj, field)?;
    Ok(usize::try_from(value).map_err(|_| {
        Error::new(
            ErrorKind::ConfigInvalid,
            format!("{field} must be non-negative"),
        )
    })?)
}

pub(crate) fn read_string_field(
    env: &mut Env<'_>,
    obj: &JObject,
    field: &str,
) -> Result<Option<String>> {
    let result = env
        .get_field(obj, JNIString::new(field), jni_sig!("Ljava/lang/String;"))?
        .l()?;
    if result.is_null() {
        Ok(None)
    } else {
        // SAFETY: the field is declared as `java.lang.String`.
        let result = unsafe { JString::from_raw(env, result.into_raw()) };
        Ok(Some(jstring_to_string(env, &result)?))
    }
}

pub(crate) fn read_metadata_condition_field(
    env: &mut Env<'_>,
    obj: &JObject,
    field: &str,
) -> Result<Option<Metadata>> {
    let snapshot = env
        .get_field(
            obj,
            JNIString::new(field),
            jni_sig!("Lorg/apache/opendal/Metadata;"),
        )?
        .l()?;
    if snapshot.is_null() {
        return Ok(None);
    }

    // Conditional operations use only the object identity from the snapshot.
    let mut builder = MetadataBuilder::unknown();
    if let Some(version) = read_string_field(env, &snapshot, "version")? {
        builder.version(version);
    }
    if let Some(etag) = read_string_field(env, &snapshot, "etag")? {
        builder.etag(etag);
    }
    Ok(Some(builder.build()))
}

pub(crate) fn read_map_field(
    env: &mut Env<'_>,
    obj: &JObject,
    field: &str,
) -> Result<Option<HashMap<String, String>>> {
    let result = env
        .get_field(obj, JNIString::new(field), jni_sig!("Ljava/util/Map;"))?
        .l()?;
    if result.is_null() {
        Ok(None)
    } else {
        Ok(Some(jmap_to_hashmap(env, &result)?))
    }
}

pub(crate) fn read_jlong_field_to_usize(
    env: &mut Env<'_>,
    options: &JObject,
    field_name: &str,
) -> Result<Option<usize>> {
    match read_int64_field(env, options, field_name)? {
        -1 => Ok(None),
        value if value > 0 => Ok(Some(usize::try_from(value).map_err(|_| {
            Error::new(
                ErrorKind::ConfigInvalid,
                format!("{field_name} is too large"),
            )
        })?)),
        _ => Err(Error::new(
            ErrorKind::ConfigInvalid,
            format!("{field_name} must be -1 or positive"),
        )
        .into()),
    }
}

pub(crate) fn read_non_negative_jlong_to_usize(
    env: &mut Env<'_>,
    options: &JObject,
    field_name: &str,
) -> Result<Option<usize>> {
    match read_int64_field(env, options, field_name)? {
        -1 => Ok(None),
        value if value >= 0 => Ok(Some(usize::try_from(value).map_err(|_| {
            Error::new(
                ErrorKind::ConfigInvalid,
                format!("{field_name} is too large"),
            )
        })?)),
        _ => Err(Error::new(
            ErrorKind::ConfigInvalid,
            format!("{field_name} must be -1 or non-negative"),
        )
        .into()),
    }
}

pub(crate) fn read_non_negative_jlong_to_u64(
    env: &mut Env<'_>,
    options: &JObject,
    field_name: &str,
) -> Result<Option<u64>> {
    match read_int64_field(env, options, field_name)? {
        -1 => Ok(None),
        value => Ok(Some(u64::try_from(value).map_err(|_| {
            Error::new(
                ErrorKind::ConfigInvalid,
                format!("{field_name} must be -1 or non-negative"),
            )
        })?)),
    }
}

pub(crate) fn read_instant_field_to_timestamp(
    env: &mut Env<'_>,
    obj: &JObject,
    field: &str,
) -> Result<Option<opendal::raw::Timestamp>> {
    let result = env
        .get_field(obj, JNIString::new(field), jni_sig!("Ljava/time/Instant;"))?
        .l()?;
    if result.is_null() {
        return Ok(None);
    }

    let epoch_second = env
        .call_method(&result, jni_str!("getEpochSecond"), jni_sig!("()J"), &[])?
        .j()?;
    let nano = env
        .call_method(&result, jni_str!("getNano"), jni_sig!("()I"), &[])?
        .i()?;
    match opendal::raw::Timestamp::new(epoch_second, nano) {
        Ok(ts) => Ok(Some(ts)),
        Err(err) => Err(Error::new(
            ErrorKind::Unexpected,
            format!("invalid timestamp: seconds={epoch_second}, nanos={nano}"),
        )
        .set_source(err)
        .into()),
    }
}

pub(crate) fn read_duration_field(
    env: &mut Env<'_>,
    obj: &JObject,
    field: &str,
) -> Result<Option<opendal::raw::Duration>> {
    let result = env
        .get_field(obj, JNIString::new(field), jni_sig!("Ljava/time/Duration;"))?
        .l()?;
    if result.is_null() {
        return Ok(None);
    }

    let seconds = env
        .call_method(&result, jni_str!("getSeconds"), jni_sig!("()J"), &[])?
        .j()?;
    let nanos = env
        .call_method(&result, jni_str!("getNano"), jni_sig!("()I"), &[])?
        .i()?;
    if seconds < 0 || (seconds == 0 && nanos <= 0) {
        return Err(Error::new(
            ErrorKind::ConfigInvalid,
            format!("{field} must be positive: seconds={seconds}, nanos={nanos}"),
        )
        .into());
    }

    let secs = u64::try_from(seconds).map_err(|err| {
        Error::new(
            ErrorKind::ConfigInvalid,
            format!("invalid {field} seconds: {seconds}"),
        )
        .set_source(err)
    })?;
    let subsec_nanos = u32::try_from(nanos).map_err(|err| {
        Error::new(
            ErrorKind::ConfigInvalid,
            format!("invalid {field} nanos: {nanos}"),
        )
        .set_source(err)
    })?;
    Ok(Some(opendal::raw::Duration::new(secs, subsec_nanos)))
}

pub(crate) fn read_range_fields(env: &mut Env<'_>, obj: &JObject) -> Result<opendal::BytesRange> {
    let offset = read_int64_field(env, obj, "offset")?;
    let length = read_int64_field(env, obj, "length")?;
    Ok(offset_length_to_range(offset, length)?.into())
}

pub(crate) fn offset_length_to_range(offset: i64, length: i64) -> Result<(Bound<u64>, Bound<u64>)> {
    let offset = u64::try_from(offset)
        .map_err(|_| Error::new(ErrorKind::RangeNotSatisfied, "offset must be non-negative"))?;

    match length {
        -1 => Ok((Bound::Included(offset), Bound::Unbounded)),
        _ => match u64::try_from(length) {
            Ok(length) => match offset.checked_add(length) {
                Some(end) => Ok((Bound::Included(offset), Bound::Excluded(end))),
                None => Err(Error::new(
                    ErrorKind::RangeNotSatisfied,
                    "offset + length causes overflow",
                )
                .into()),
            },
            Err(_) => {
                Err(Error::new(ErrorKind::RangeNotSatisfied, "length must be non-negative").into())
            }
        },
    }
}

pub(crate) fn bytes_to_jbytearray<'a>(
    env: &mut Env<'a>,
    bytes: impl AsRef<[u8]>,
) -> Result<JByteArray<'a>> {
    let bytes = bytes.as_ref();
    let res = env.byte_array_from_slice(bytes)?;
    Ok(res)
}

pub(crate) fn jstring_to_string(env: &mut Env, s: &JString) -> Result<String> {
    Ok(s.mutf8_chars(env)?.into())
}
