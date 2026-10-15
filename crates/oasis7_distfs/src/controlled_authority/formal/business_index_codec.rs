//! Bounded decoding before allocation of delta sequences. History size is never
//! a limit; only one transition's entries and object byte strings are bounded.
use super::*;
fn bounded_seq<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>, const MAX: usize>(
    decoder: D,
) -> Result<Vec<T>, D::Error> {
    struct Visitor<T, const MAX: usize>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const MAX: usize> serde::de::Visitor<'de> for Visitor<T, MAX> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("bounded business delta sequence")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Vec<T>, A::Error> {
            if sequence.size_hint().is_some_and(|size| size > MAX) {
                return Err(serde::de::Error::custom("business delta entry bound"));
            }
            let mut values = Vec::new();
            while let Some(value) = sequence.next_element()? {
                if values.len() == MAX {
                    return Err(serde::de::Error::custom("business delta entry bound"));
                }
                values.push(value);
            }
            Ok(values)
        }
    }
    decoder.deserialize_seq(Visitor::<T, MAX>(std::marker::PhantomData))
}
pub(super) fn decode_updates<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Vec<BusinessIndexLeafV1>, D::Error> {
    bounded_seq::<D, BusinessIndexLeafV1, BUSINESS_INDEX_MAX_UPDATES>(decoder)
}
pub(super) fn decode_writes<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Vec<BusinessIndexNodeWriteV1>, D::Error> {
    bounded_seq::<
        D,
        BusinessIndexNodeWriteV1,
        { BUSINESS_INDEX_MAX_UPDATES * (BUSINESS_INDEX_DEPTH + 1) },
    >(decoder)
}
pub(super) fn encode_bytes<S: serde::Serializer>(
    value: &[u8],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_bytes(value)
}
pub(super) fn decode_value<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Vec<u8>, D::Error> {
    bounded_bytes::<D, BUSINESS_INDEX_MAX_LEAF_BYTES>(decoder)
}
pub(super) fn decode_node<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Vec<u8>, D::Error> {
    bounded_bytes::<D, { BUSINESS_INDEX_MAX_LEAF_BYTES + 128 }>(decoder)
}
fn bounded_bytes<'de, D: serde::Deserializer<'de>, const MAX: usize>(
    decoder: D,
) -> Result<Vec<u8>, D::Error> {
    struct Visitor<const MAX: usize>;
    impl<'de, const MAX: usize> serde::de::Visitor<'de> for Visitor<MAX> {
        type Value = Vec<u8>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("bounded business node bytes")
        }
        fn visit_bytes<E: serde::de::Error>(self, value: &[u8]) -> Result<Vec<u8>, E> {
            if value.len() > MAX {
                return Err(E::custom("business node byte bound"));
            }
            Ok(value.to_vec())
        }
        fn visit_byte_buf<E: serde::de::Error>(self, value: Vec<u8>) -> Result<Vec<u8>, E> {
            if value.len() > MAX {
                return Err(E::custom("business node byte bound"));
            }
            Ok(value)
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Vec<u8>, A::Error> {
            if sequence.size_hint().is_some_and(|size| size > MAX) {
                return Err(serde::de::Error::custom("business node byte bound"));
            }
            let mut values = Vec::new();
            while let Some(value) = sequence.next_element()? {
                if values.len() == MAX {
                    return Err(serde::de::Error::custom("business node byte bound"));
                }
                values.push(value);
            }
            Ok(values)
        }
    }
    decoder.deserialize_byte_buf(Visitor::<MAX>)
}

pub(super) fn decode_path_nodes<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Vec<BusinessIndexNodeWriteV1>, D::Error> {
    struct PathVisitor;
    impl<'de> serde::de::Visitor<'de> for PathVisitor {
        type Value = Vec<BusinessIndexNodeWriteV1>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("bounded business node path")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            if seq
                .size_hint()
                .is_some_and(|size| size > BUSINESS_INDEX_DEPTH + 1)
            {
                return Err(serde::de::Error::custom("business index path node bound"));
            }
            let mut nodes = Vec::new();
            let mut total = 0usize;
            while let Some(node) = seq.next_element::<BusinessIndexNodeWriteV1>()? {
                if nodes.len() == BUSINESS_INDEX_DEPTH + 1 {
                    return Err(serde::de::Error::custom("business index path node bound"));
                }
                total = total
                    .checked_add(node.bytes.len())
                    .ok_or_else(|| serde::de::Error::custom("business index path byte overflow"))?;
                if total > BUSINESS_INDEX_MAX_DELTA_BYTES {
                    return Err(serde::de::Error::custom("business index path byte bound"));
                }
                nodes.push(node);
            }
            Ok(nodes)
        }
    }
    decoder.deserialize_seq(PathVisitor)
}
