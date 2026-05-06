use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoPos {
    pub x_cm: f64,
    pub y_cm: f64,
    pub z_cm: f64,
}

impl GeoPos {
    pub fn new(x_cm: f64, y_cm: f64, z_cm: f64) -> Self {
        Self { x_cm, y_cm, z_cm }.canonicalized()
    }

    pub fn canonicalized(self) -> Self {
        Self {
            x_cm: canonicalize_space_component_cm(self.x_cm),
            y_cm: canonicalize_space_component_cm(self.y_cm),
            z_cm: canonicalize_space_component_cm(self.z_cm),
        }
    }
}

pub const SPACE_UNIT_CM: i64 = 1;
pub const DEFAULT_CLOUD_WIDTH_KM: i64 = 100;
pub const DEFAULT_CLOUD_DEPTH_KM: i64 = 100;
pub const DEFAULT_CLOUD_HEIGHT_KM: i64 = 10;
pub const DEFAULT_CLOUD_WIDTH_CM: i64 = DEFAULT_CLOUD_WIDTH_KM * 100_000;
pub const DEFAULT_CLOUD_DEPTH_CM: i64 = DEFAULT_CLOUD_DEPTH_KM * 100_000;
pub const DEFAULT_CLOUD_HEIGHT_CM: i64 = DEFAULT_CLOUD_HEIGHT_KM * 100_000;

pub fn canonicalize_space_component_cm(value_cm: f64) -> f64 {
    if !value_cm.is_finite() {
        return value_cm;
    }
    let unit = SPACE_UNIT_CM.max(1) as f64;
    (value_cm / unit).round() * unit
}

pub fn space_distance_m(a: GeoPos, b: GeoPos) -> f64 {
    let dx_m = (a.x_cm - b.x_cm) / 100.0;
    let dy_m = (a.y_cm - b.y_cm) / 100.0;
    let dz_m = (a.z_cm - b.z_cm) / 100.0;
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
    fn geo_pos_new_rounds_to_centimeter_grid() {
        let pos = GeoPos::new(10.49, -2.5, 3.51);

        assert_eq!(pos.x_cm, 10.0);
        assert_eq!(pos.y_cm, -3.0);
        assert_eq!(pos.z_cm, 4.0);
    }
}
