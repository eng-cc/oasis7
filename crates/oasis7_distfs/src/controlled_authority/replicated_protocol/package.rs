use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) const MAX_RECORD_BYTES: usize = 8 * 1024 * 1024;
pub(super) const MAX_OBJECTS: usize = 256;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArtifactRole {
    Input,
    Result,
    Snapshot,
    Journal,
    NonceIndex,
    EffectOutbox,
    ExecutionManifest,
    Wasm,
    Rules,
}
const ROLES: [ArtifactRole; 9] = [
    ArtifactRole::Input,
    ArtifactRole::Result,
    ArtifactRole::Snapshot,
    ArtifactRole::Journal,
    ArtifactRole::NonceIndex,
    ArtifactRole::EffectOutbox,
    ArtifactRole::ExecutionManifest,
    ArtifactRole::Wasm,
    ArtifactRole::Rules,
];
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactObject {
    pub content_hash: String,
    #[serde(deserialize_with = "decode_bytes")]
    pub bytes: Vec<u8>,
    /// Only these explicitly declared references are checked. The protocol does
    /// not infer opaque JSON/WASM references or validate execution semantics.
    #[serde(deserialize_with = "decode_references")]
    pub references: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClosedRecord {
    pub payload_digest: String,
    pub before_state_root: String,
    pub after_state_root: String,
    pub roots: BTreeMap<ArtifactRole, String>,
    /// Canonical ordering by content hash; full bytes travel to both endpoints.
    #[serde(deserialize_with = "decode_objects")]
    pub objects: Vec<ArtifactObject>,
}
pub(super) fn bounded_vec<'de, D, T, const N: usize>(decoder: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Bounded<T, const N: usize>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const N: usize> serde::de::Visitor<'de> for Bounded<T, N> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("bounded typed sequence")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<T>, A::Error> {
            if seq.size_hint().is_some_and(|n| n > N) {
                return Err(serde::de::Error::custom("typed sequence capacity limit"));
            }
            let mut result = Vec::new();
            while let Some(value) = seq.next_element()? {
                if result.len() == N {
                    return Err(serde::de::Error::custom("typed sequence count limit"));
                }
                result.push(value);
            }
            Ok(result)
        }
    }
    decoder.deserialize_seq(Bounded::<T, N>(std::marker::PhantomData))
}
fn decode_bytes<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    bounded_vec::<D, u8, MAX_RECORD_BYTES>(d)
}
fn decode_references<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    bounded_vec::<D, String, MAX_OBJECTS>(d)
}
fn decode_objects<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<ArtifactObject>, D::Error> {
    bounded_vec::<D, ArtifactObject, MAX_OBJECTS>(d)
}
impl ClosedRecord {
    /// Caller must supply an authorized, correctly executed record and declare
    /// every recovery dependency. This validates typed declared closure only,
    /// not player authorization, WASM behavior, result correctness or real fsync.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        crypto::hash_hex(&self.payload_digest)?;
        crypto::hash_hex(&self.before_state_root)?;
        crypto::hash_hex(&self.after_state_root)?;
        if self.roots.len() != ROLES.len()
            || ROLES.iter().any(|r| !self.roots.contains_key(r))
            || self.objects.is_empty()
            || self.objects.len() > MAX_OBJECTS
        {
            return Err(invalid(
                "record requires all nine roles and bounded objects",
            ));
        }
        let mut total = 0usize;
        let mut objects = BTreeMap::new();
        let mut previous: Option<&str> = None;
        for object in &self.objects {
            crypto::hash_hex(&object.content_hash)?;
            if previous.is_some_and(|p| p >= object.content_hash.as_str()) {
                return Err(invalid("objects must be strictly hash ordered"));
            }
            previous = Some(&object.content_hash);
            total = total
                .checked_add(object.bytes.len())
                .ok_or_else(|| invalid("byte overflow"))?;
            if total > MAX_RECORD_BYTES || object.references.len() > MAX_OBJECTS {
                return Err(invalid("record byte/reference limit"));
            }
            if blake3::hash(&object.bytes).to_hex().as_str() != object.content_hash {
                return Err(invalid("artifact byte hash mismatch"));
            }
            let mut last: Option<&str> = None;
            for reference in &object.references {
                crypto::hash_hex(reference)?;
                if last.is_some_and(|p| p >= reference.as_str()) {
                    return Err(invalid("references must be strictly ordered"));
                }
                last = Some(reference);
            }
            objects.insert(object.content_hash.as_str(), object);
        }
        if self.roots[&ArtifactRole::Input] != self.payload_digest {
            return Err(invalid("payload digest must identify complete input bytes"));
        }
        let mut reached = BTreeSet::new();
        let mut queue = self.roots.values().map(String::as_str).collect::<Vec<_>>();
        while let Some(hash) = queue.pop() {
            if !reached.insert(hash) {
                continue;
            }
            let object = objects
                .get(hash)
                .ok_or_else(|| invalid("declared artifact missing"))?;
            queue.extend(object.references.iter().map(String::as_str));
        }
        if reached.len() != objects.len() {
            return Err(invalid("unreachable undeclared artifact"));
        }
        Ok(())
    }
    pub(super) fn byte_len(&self) -> usize {
        self.objects.iter().map(|o| o.bytes.len()).sum()
    }
}
