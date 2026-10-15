use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct GeoPos {
    pub x_cm: i64,
    pub y_cm: i64,
    pub z_cm: i64,
}

impl GeoPos {
    pub fn new(x_cm: i64, y_cm: i64, z_cm: i64) -> Self {
        Self { x_cm, y_cm, z_cm }
    }
}

impl GeoCoordValue {
    fn into_i64<E: de::Error>(self, field: &str) -> Result<i64, E> {
        match self {
            Self::Integer(value) => Ok(value),
            Self::Float(value) => {
                if !value.is_finite() {
                    return Err(E::custom(format!(
                        "GeoPos field `{field}` must be finite, got {value}"
                    )));
                }
                if value < i64::MIN as f64 || value > i64::MAX as f64 {
                    return Err(E::custom(format!(
                        "GeoPos field `{field}` is out of i64 range: {value}"
                    )));
                }
                if value.trunc() != value {
                    return Err(E::custom(format!(
                        "GeoPos field `{field}` must be an integer centimeter value, got {value}"
                    )));
                }
                Ok(value as i64)
            }
        }
    }
}

impl<'de> Deserialize<'de> for GeoPos {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let repr = GeoPosRepr::deserialize(deserializer)?;
        Ok(Self {
            x_cm: repr.x_cm.into_i64("x_cm")?,
            y_cm: repr.y_cm.into_i64("y_cm")?,
            z_cm: repr.z_cm.into_i64("z_cm")?,
        })
    }
}

#[derive(Deserialize)]
struct GeoPosRepr {
    x_cm: GeoCoordValue,
    y_cm: GeoCoordValue,
    z_cm: GeoCoordValue,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum GeoCoordValue {
    Integer(i64),
    Float(f64),
}
