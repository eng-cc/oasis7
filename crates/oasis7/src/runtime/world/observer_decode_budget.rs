//! Allocation-free structural preflight before typed serde decoding.
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::fmt;

use super::{ObserverLoadError, ObserverReadLimits, WorldEvent};

struct Budget {
    remaining: usize,
    container_limit: usize,
    string_limit: usize,
    depth: usize,
}
impl Budget {
    fn new(limits: ObserverReadLimits) -> Self {
        // A journal event is one of the largest elements stored in observer
        // containers. Use its in-memory width before any Vec capacity hint is
        // honored; byte/string payloads have their separate length bound.
        let width = std::mem::size_of::<WorldEvent>().max(256);
        Self {
            remaining: limits.max_elements,
            container_limit: limits
                .max_elements
                .min(limits.max_single_allocation_bytes / width),
            string_limit: limits.max_single_allocation_bytes,
            depth: 0,
        }
    }
    fn spend<E: serde::de::Error>(&mut self) -> Result<(), E> {
        self.remaining = self
            .remaining
            .checked_sub(1)
            .ok_or_else(|| E::custom("ResourceLimited: serde element budget"))?;
        Ok(())
    }
    fn enter<E: serde::de::Error>(&mut self, hint: Option<usize>) -> Result<(), E> {
        if self.depth >= 64 || hint.is_some_and(|n| n > self.container_limit) {
            return Err(E::custom("ResourceLimited: serde container/depth budget"));
        }
        self.depth += 1;
        Ok(())
    }
}
#[derive(Clone, Copy)]
enum Schema {
    Generic,
    SnapshotRoot,
    ModuleArtifacts,
}
struct Check<'a>(&'a mut Budget, Schema);
impl<'de> DeserializeSeed<'de> for Check<'_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<(), D::Error> {
        self.0.spend()?;
        match self.1 {
            Schema::Generic => decoder.deserialize_any(self),
            Schema::SnapshotRoot | Schema::ModuleArtifacts => decoder.deserialize_map(self),
        }
    }
}
impl<'de> Visitor<'de> for Check<'_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a budgeted persisted value")
    }
    fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_none<E: serde::de::Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_some<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<(), D::Error> {
        self.deserialize(decoder)
    }
    fn visit_newtype_struct<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<(), D::Error> {
        self.deserialize(decoder)
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<(), E> {
        if value.len() > self.0.string_limit {
            Err(E::custom("ResourceLimited: serde string budget"))
        } else {
            Ok(())
        }
    }
    fn visit_borrowed_str<E: serde::de::Error>(self, value: &'de str) -> Result<(), E> {
        self.visit_str(value)
    }
    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<(), E> {
        self.visit_str(&value)
    }
    fn visit_bytes<E: serde::de::Error>(self, value: &[u8]) -> Result<(), E> {
        if value.len() > self.0.string_limit {
            Err(E::custom("ResourceLimited: serde byte budget"))
        } else {
            Ok(())
        }
    }
    fn visit_borrowed_bytes<E: serde::de::Error>(self, value: &'de [u8]) -> Result<(), E> {
        self.visit_bytes(value)
    }
    fn visit_byte_buf<E: serde::de::Error>(self, value: Vec<u8>) -> Result<(), E> {
        self.visit_bytes(&value)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        self.0.enter(seq.size_hint())?;
        let mut length = 0usize;
        while seq
            .next_element_seed(Check(self.0, Schema::Generic))?
            .is_some()
        {
            length += 1;
            if length > self.0.container_limit {
                return Err(serde::de::Error::custom(
                    "ResourceLimited: serde sequence budget",
                ));
            }
        }
        self.0.depth -= 1;
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        self.0.enter(map.size_hint())?;
        let mut length = 0usize;
        loop {
            let artifact_field = if matches!(self.1, Schema::SnapshotRoot) {
                match map.next_key_seed(SnapshotKey(self.0))? {
                    Some(value) => value,
                    None => break,
                }
            } else {
                if map.next_key_seed(Check(self.0, Schema::Generic))?.is_none() {
                    break;
                }
                false
            };
            length += 1;
            if length > self.0.container_limit {
                return Err(serde::de::Error::custom(
                    "ResourceLimited: serde map budget",
                ));
            }
            if matches!(self.1, Schema::ModuleArtifacts) {
                map.next_value_seed(BytePayload(self.0))?;
            } else {
                let schema = if artifact_field {
                    Schema::ModuleArtifacts
                } else {
                    Schema::Generic
                };
                map.next_value_seed(Check(self.0, schema))?;
            }
        }
        self.0.depth -= 1;
        Ok(())
    }
}
// Only a direct Snapshot root key selects the artifact-byte schema. Nested
// maps, journal events, and strings named the same never inherit this state.
struct SnapshotKey<'a>(&'a mut Budget);
impl<'de> DeserializeSeed<'de> for SnapshotKey<'_> {
    type Value = bool;
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<bool, D::Error> {
        self.0.spend()?;
        decoder.deserialize_str(self)
    }
}
impl<'de> Visitor<'de> for SnapshotKey<'_> {
    type Value = bool;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Snapshot field")
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<bool, E> {
        if value.len() > self.0.string_limit {
            return Err(E::custom("ResourceLimited: serde string budget"));
        }
        Ok(value == "module_artifact_bytes")
    }
}
struct BytePayload<'a>(&'a mut Budget);
impl<'de> DeserializeSeed<'de> for BytePayload<'_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<(), D::Error> {
        self.0.spend()?;
        decoder.deserialize_any(self)
    }
}
struct ByteElement<'a>(&'a mut Budget);
impl<'de> DeserializeSeed<'de> for ByteElement<'_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<(), D::Error> {
        self.0.spend()?;
        <u8 as serde::Deserialize>::deserialize(decoder).map(|_| ())
    }
}
impl<'de> Visitor<'de> for BytePayload<'_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an artifact u8 sequence")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        let limit = self.0.string_limit.min(self.0.remaining);
        // Byte arrays allocate one byte per entry; preserve pre-allocation
        // hint rejection without applying WorldEvent's unrelated width.
        if seq.size_hint().is_some_and(|n| n > limit) {
            return Err(serde::de::Error::custom(
                "ResourceLimited: artifact byte capacity",
            ));
        }
        self.0.enter(None)?;
        let mut length = 0usize;
        while seq.next_element_seed(ByteElement(self.0))?.is_some() {
            length += 1;
            if length > limit {
                return Err(serde::de::Error::custom(
                    "ResourceLimited: artifact byte capacity",
                ));
            }
        }
        self.0.depth -= 1;
        Ok(())
    }
    fn visit_bytes<E: serde::de::Error>(self, bytes: &[u8]) -> Result<(), E> {
        if bytes.len() > self.0.string_limit {
            return Err(E::custom("ResourceLimited: artifact byte capacity"));
        }
        self.0.remaining = self
            .0
            .remaining
            .checked_sub(bytes.len())
            .ok_or_else(|| E::custom("ResourceLimited: serde element budget"))?;
        Ok(())
    }
    fn visit_borrowed_bytes<E: serde::de::Error>(self, bytes: &'de [u8]) -> Result<(), E> {
        self.visit_bytes(bytes)
    }
    fn visit_byte_buf<E: serde::de::Error>(self, bytes: Vec<u8>) -> Result<(), E> {
        self.visit_bytes(&bytes)
    }
}
fn failure(error: impl fmt::Display) -> ObserverLoadError {
    let message = error.to_string();
    if message.contains("ResourceLimited") {
        ObserverLoadError::ResourceLimited
    } else {
        ObserverLoadError::IntegrityFailure(message)
    }
}
pub(super) fn json(bytes: &[u8], limits: ObserverReadLimits) -> Result<(), ObserverLoadError> {
    let mut budget = Budget::new(limits);
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    Check(&mut budget, Schema::Generic)
        .deserialize(&mut decoder)
        .map_err(failure)?;
    decoder.end().map_err(failure)
}
pub(super) fn cbor(bytes: &[u8], limits: ObserverReadLimits) -> Result<(), ObserverLoadError> {
    let mut budget = Budget::new(limits);
    let mut decoder = serde_cbor::Deserializer::from_slice(bytes);
    Check(&mut budget, Schema::Generic)
        .deserialize(&mut decoder)
        .map_err(failure)?;
    decoder.end().map_err(failure)
}

pub(super) fn snapshot_cbor(
    bytes: &[u8],
    limits: ObserverReadLimits,
) -> Result<(), ObserverLoadError> {
    let mut budget = Budget::new(limits);
    let mut decoder = serde_cbor::Deserializer::from_slice(bytes);
    Check(&mut budget, Schema::SnapshotRoot)
        .deserialize(&mut decoder)
        .map_err(failure)?;
    decoder.end().map_err(failure)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_small_inputs_with_excessive_typed_element_capacity() {
        let limits = ObserverReadLimits {
            max_elements: 3,
            ..ObserverReadLimits::default()
        };
        assert!(matches!(
            json(b"[0,0,0,0]", limits),
            Err(ObserverLoadError::ResourceLimited)
        ));
        // Definite CBOR array declaring an enormous number of elements. The
        // preflight rejects its size hint before typed Vec can reserve it.
        assert!(matches!(
            cbor(
                &[0x9b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
                limits
            ),
            Err(ObserverLoadError::ResourceLimited)
        ));
        assert!(json(b"[0,1]", limits).is_ok());
    }
}

#[cfg(test)]
#[path = "observer_decode_budget_tests.rs"]
mod snapshot_tests;
