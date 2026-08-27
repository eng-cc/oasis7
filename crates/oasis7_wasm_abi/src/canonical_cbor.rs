use crate::ModuleCommandValidationError;
use serde::Serialize;

/// Encode a serializable value using the canonical map ordering required by
/// the module ABI.
///
/// A CBOR map's deterministic order sorts encoded keys by length and then by
/// their encoded bytes. Ciborium provides the maintained serde-compatible
/// value model and canonical key comparator; normalize every nested map before
/// serializing so the ABI does not depend on a struct declaration or map
/// iteration order.
pub fn encode_canonical_cbor<T: Serialize>(
    value: &T,
) -> Result<Vec<u8>, ModuleCommandValidationError> {
    let mut value = ciborium::Value::serialized(value)
        .map_err(|error| ModuleCommandValidationError::CanonicalEncoding(error.to_string()))?;
    normalize_canonical_cbor_value(&mut value);
    let mut encoded = Vec::new();
    ciborium::ser::into_writer(&value, &mut encoded)
        .map_err(|error| ModuleCommandValidationError::CanonicalEncoding(error.to_string()))
        .map(|_| encoded)
}

fn normalize_canonical_cbor_value(value: &mut ciborium::Value) {
    match value {
        ciborium::Value::Array(values) => {
            for value in values {
                normalize_canonical_cbor_value(value);
            }
        }
        ciborium::Value::Map(entries) => {
            for (key, value) in entries.iter_mut() {
                normalize_canonical_cbor_value(key);
                normalize_canonical_cbor_value(value);
            }
            entries.sort_by(|(left, _), (right, _)| {
                ciborium::value::CanonicalValue::from(left.clone())
                    .cmp(&ciborium::value::CanonicalValue::from(right.clone()))
            });
        }
        ciborium::Value::Tag(_, value) => normalize_canonical_cbor_value(value),
        ciborium::Value::Integer(_)
        | ciborium::Value::Bytes(_)
        | ciborium::Value::Float(_)
        | ciborium::Value::Text(_)
        | ciborium::Value::Bool(_)
        | ciborium::Value::Null => {}
        _ => {}
    }
}
