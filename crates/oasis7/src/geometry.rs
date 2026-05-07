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

pub const SPACE_UNIT_CM: i64 = 1;
pub const DEFAULT_CLOUD_WIDTH_KM: i64 = 100;
pub const DEFAULT_CLOUD_DEPTH_KM: i64 = 100;
pub const DEFAULT_CLOUD_HEIGHT_KM: i64 = 10;
pub const DEFAULT_CLOUD_WIDTH_CM: i64 = DEFAULT_CLOUD_WIDTH_KM * 100_000;
pub const DEFAULT_CLOUD_DEPTH_CM: i64 = DEFAULT_CLOUD_DEPTH_KM * 100_000;
pub const DEFAULT_CLOUD_HEIGHT_CM: i64 = DEFAULT_CLOUD_HEIGHT_KM * 100_000;

pub fn space_distance_m(a: GeoPos, b: GeoPos) -> f64 {
    let dx_m = (a.x_cm - b.x_cm) as f64 / 100.0;
    let dy_m = (a.y_cm - b.y_cm) as f64 / 100.0;
    let dz_m = (a.z_cm - b.z_cm) as f64 / 100.0;
    ((dx_m * dx_m) + (dy_m * dy_m) + (dz_m * dz_m)).sqrt()
}

pub fn space_distance_cm(a: GeoPos, b: GeoPos) -> i64 {
    let distance_m = space_distance_m(a, b);
    (distance_m * 100.0).round().max(0.0) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geo_pos_new_keeps_integer_centimeter_coordinates() {
        let pos = GeoPos::new(10, -3, 4);

        assert_eq!(pos.x_cm, 10);
        assert_eq!(pos.y_cm, -3);
        assert_eq!(pos.z_cm, 4);
    }

    #[test]
    fn geo_pos_deserializes_legacy_float_centimeter_coordinates() {
        let decoded: GeoPos = serde_json::from_str(r#"{"x_cm":10.0,"y_cm":-3.0,"z_cm":4.0}"#)
            .expect("decode legacy float geopo");

        assert_eq!(decoded, GeoPos::new(10, -3, 4));
    }

    #[test]
    fn geo_pos_rejects_non_integral_float_coordinates() {
        let err = serde_json::from_str::<GeoPos>(r#"{"x_cm":10.5,"y_cm":-3.0,"z_cm":4.0}"#)
            .expect_err("non-integral coordinates should fail");

        assert!(err.to_string().contains("integer centimeter value"));
    }
}
