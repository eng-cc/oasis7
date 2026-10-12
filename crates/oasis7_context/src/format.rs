use crate::{
    error::{ContextError, Result},
    model::*,
};
use serde::{
    Deserialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
struct StrictValue(Value);
impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = StrictValue;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("JSON")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(v.into()))
            }
            fn visit_f64<E: de::Error>(self, _: f64) -> std::result::Result<Self::Value, E> {
                Err(E::custom("floats forbidden"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(v.into()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(v.into()))
            }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut v = Vec::new();
                while let Some(x) = a.next_element::<StrictValue>()? {
                    v.push(x.0)
                }
                Ok(StrictValue(Value::Array(v)))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut m = serde_json::Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if m.contains_key(&k) {
                        return Err(de::Error::custom("duplicate key"));
                    }
                    m.insert(k, a.next_value::<StrictValue>()?.0);
                }
                Ok(StrictValue(Value::Object(m)))
            }
        }
        d.deserialize_any(V)
    }
}
pub fn strict_json<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    if bytes.len() > MAX_METADATA_BYTES {
        return Err(ContextError::new("LIMIT_EXCEEDED", "JSON exceeds limit"));
    }
    strict_json_bounded(bytes, MAX_METADATA_BYTES)
}
pub fn strict_json_bounded<T: serde::de::DeserializeOwned>(bytes: &[u8], max: usize) -> Result<T> {
    if bytes.len() > max {
        return Err(ContextError::new("LIMIT_EXCEEDED", "JSON exceeds limit"));
    }
    let value: StrictValue = serde_json::from_slice(bytes)
        .map_err(|_| ContextError::new("INVALID_FORMAT", "invalid JSON"))?;
    fn depth(v: &Value, n: usize) -> bool {
        n <= 16
            && match v {
                Value::Object(m) => m.values().all(|x| depth(x, n + 1)),
                Value::Array(a) => a.iter().all(|x| depth(x, n + 1)),
                _ => true,
            }
    }
    if !depth(&value.0, 0) {
        return Err(ContextError::new(
            "INVALID_FORMAT",
            "JSON nesting exceeds limit",
        ));
    }
    serde_json::from_value(value.0)
        .map_err(|_| ContextError::new("INVALID_SCHEMA", "invalid JSON fields"))
}
pub fn revision(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}
pub fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
pub fn parse(bytes: &[u8]) -> Result<Entry> {
    if bytes.len() > MAX_ENTRY_BYTES {
        return Err(ContextError::new("LIMIT_EXCEEDED", "entry exceeds limit"));
    }
    let raw = std::str::from_utf8(bytes)
        .map_err(|_| ContextError::new("INVALID_FORMAT", "entry is not UTF-8"))?;
    if raw.starts_with('\u{feff}') || raw.contains('\0') {
        return Err(ContextError::new(
            "INVALID_FORMAT",
            "invalid entry encoding",
        ));
    }
    let crlf = raw.contains("\r\n");
    if raw.contains('\r') && (!crlf || raw.replace("\r\n", "").contains('\r'))
        || crlf && raw.replace("\r\n", "").contains('\n')
    {
        return Err(ContextError::new("INVALID_FORMAT", "mixed line endings"));
    }
    let normalized = raw.replace("\r\n", "\n");
    if !normalized.starts_with("---\n") {
        return Err(ContextError::new("INVALID_FORMAT", "missing front matter"));
    }
    let tail = &normalized[4..];
    let end = tail
        .find("\n---\n")
        .ok_or_else(|| ContextError::new("INVALID_FORMAT", "missing front matter delimiter"))?;
    let metadata: Metadata = strict_json(&tail.as_bytes()[..end])?;
    metadata.validate()?;
    Ok(Entry {
        metadata,
        body: tail[end + 5..].into(),
    })
}
pub fn encode(entry: &Entry) -> Result<Vec<u8>> {
    entry.metadata.validate()?;
    let meta = serde_json::to_string_pretty(&entry.metadata)
        .map_err(|_| ContextError::new("INVALID_FORMAT", "metadata serialization failed"))?;
    if meta.len() > MAX_METADATA_BYTES {
        return Err(ContextError::new(
            "LIMIT_EXCEEDED",
            "metadata exceeds limit",
        ));
    }
    let bytes = format!("---\n{meta}\n---\n{}", entry.body.replace("\r\n", "\n")).into_bytes();
    if bytes.len() > MAX_ENTRY_BYTES {
        return Err(ContextError::new("LIMIT_EXCEEDED", "entry exceeds limit"));
    }
    Ok(bytes)
}
pub fn canonical_json(v: &Value) -> Vec<u8> {
    fn normalized(v: &Value) -> Value {
        match v {
            Value::Object(m) => {
                let mut keys: Vec<_> = m.keys().collect();
                keys.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
                let mut n = serde_json::Map::new();
                for k in keys {
                    n.insert(k.clone(), normalized(&m[k]));
                }
                Value::Object(n)
            }
            Value::Array(a) => Value::Array(a.iter().map(normalized).collect()),
            _ => v.clone(),
        }
    } // Explicit writer prevents serde_json preserve_order feature from changing the contract.
    fn write(v: &Value, out: &mut String) {
        match v {
            Value::Object(m) => {
                out.push('{');
                let mut keys: Vec<_> = m.keys().collect();
                keys.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
                for (i, k) in keys.iter().enumerate() {
                    if i > 0 {
                        out.push(',')
                    }
                    out.push_str(&serde_json::to_string(k).unwrap());
                    out.push(':');
                    write(&m[*k], out)
                }
                out.push('}')
            }
            Value::Array(a) => {
                out.push('[');
                for (i, x) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',')
                    }
                    write(x, out)
                }
                out.push(']')
            }
            _ => out.push_str(&serde_json::to_string(v).unwrap()),
        }
    }
    let mut s = String::new();
    write(&normalized(v), &mut s);
    s.into_bytes()
}
pub fn subject_sha256(e: &Entry) -> String {
    let mut m = serde_json::to_value(&e.metadata).unwrap();
    for k in ["updated_at", "updated_by", "verification"] {
        m.as_object_mut().unwrap().remove(k);
    }
    digest(&canonical_json(
        &serde_json::json!({"domain":"oasis7.context_subject.v1","metadata":m,"body_lf":e.body.replace("\r\n","\n")}),
    ))
}
pub fn content_equal_for_save(a: &Entry, b: &Entry) -> bool {
    let mut a = a.clone();
    let mut b = b.clone();
    a.metadata.updated_at.clear();
    a.metadata.updated_by.clear();
    b.metadata.updated_at.clear();
    b.metadata.updated_by.clear();
    a.body = a.body.replace("\r\n", "\n");
    b.body = b.body.replace("\r\n", "\n");
    a == b
}
pub fn verification_state(e: &Entry, max_age_days: u32) -> VerificationState {
    let Some(v) = &e.metadata.verification else {
        return VerificationState::Unverified;
    };
    if subject_sha256(e) != v.subject_sha256 {
        return VerificationState::ContentChanged;
    }
    let Ok(t) = validate_timestamp(&v.observed_at) else {
        return VerificationState::ContentChanged;
    };
    let now = chrono::Utc::now();
    if t > now {
        VerificationState::FutureDated
    } else if now.signed_duration_since(t) > chrono::Duration::days(max_age_days.into()) {
        VerificationState::Expired
    } else {
        VerificationState::Current
    }
}
