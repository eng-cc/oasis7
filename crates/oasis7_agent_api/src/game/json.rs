use std::fmt;

use serde::de::{self, DeserializeOwned, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

use super::{MAX_JSON_DEPTH, MAX_REQUEST_BYTES};

/// Parse raw request bytes before any conversion to `Value` can discard duplicate
/// keys. The depth and byte budgets are fixed protocol limits, not caller input.
pub fn parse_request<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, serde_json::Error> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(de::Error::custom("Game API request exceeds byte limit"));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = JsonSeed(0).deserialize(&mut deserializer)?;
    deserializer.end()?;
    serde_json::from_value(value)
}

struct JsonSeed(usize);

impl<'de> DeserializeSeed<'de> for JsonSeed {
    type Value = Value;
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
        d.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for JsonSeed {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("bounded JSON without duplicate object keys")
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Value, E> {
        Ok(v.into())
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Value, E> {
        Ok(v.into())
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Value, E> {
        Ok(v.into())
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Value, E> {
        serde_json::Number::from_f64(v)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Value, E> {
        Ok(v.into())
    }
    fn visit_string<E: de::Error>(self, v: String) -> Result<Value, E> {
        Ok(v.into())
    }
    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
        if self.0 >= MAX_JSON_DEPTH {
            return Err(de::Error::custom("JSON depth exceeded"));
        }
        let mut values = Vec::new();
        while let Some(value) = a.next_element_seed(JsonSeed(self.0 + 1))? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
        if self.0 >= MAX_JSON_DEPTH {
            return Err(de::Error::custom("JSON depth exceeded"));
        }
        let mut values = Map::new();
        while let Some(key) = a.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom("duplicate JSON key"));
            }
            values.insert(key, a.next_value_seed(JsonSeed(self.0 + 1))?);
        }
        Ok(Value::Object(values))
    }
}
