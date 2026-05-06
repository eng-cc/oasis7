use crate::geometry::GeoPos;
use serde::{Deserialize, Serialize};

use super::world_model::SpaceConfig;

pub const CHUNK_SIZE_X_CM: i64 = 2_000_000;
pub const CHUNK_SIZE_Y_CM: i64 = 2_000_000;
pub const CHUNK_SIZE_Z_CM: i64 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChunkBounds {
    pub min: GeoPos,
    pub max: GeoPos,
}

pub fn chunk_grid_dims(space: &SpaceConfig) -> (i32, i32, i32) {
    (
        ceil_div_i64(space.width_cm.max(0), CHUNK_SIZE_X_CM) as i32,
        ceil_div_i64(space.depth_cm.max(0), CHUNK_SIZE_Y_CM) as i32,
        ceil_div_i64(space.height_cm.max(0), CHUNK_SIZE_Z_CM) as i32,
    )
}

pub fn chunk_coords(space: &SpaceConfig) -> Vec<ChunkCoord> {
    let (gx, gy, gz) = chunk_grid_dims(space);
    let mut coords = Vec::new();
    for x in 0..gx {
        for y in 0..gy {
            for z in 0..gz {
                coords.push(ChunkCoord { x, y, z });
            }
        }
    }
    coords
}

pub fn chunk_coord_of(pos: GeoPos, space: &SpaceConfig) -> Option<ChunkCoord> {
    if !space.contains(pos) {
        return None;
    }
    let (gx, gy, gz) = chunk_grid_dims(space);
    if gx <= 0 || gy <= 0 || gz <= 0 {
        return None;
    }

    let x = clamp_chunk_index((pos.x_cm / CHUNK_SIZE_X_CM) as i32, gx);
    let y = clamp_chunk_index((pos.y_cm / CHUNK_SIZE_Y_CM) as i32, gy);
    let z = clamp_chunk_index((pos.z_cm / CHUNK_SIZE_Z_CM) as i32, gz);
    Some(ChunkCoord { x, y, z })
}

pub fn chunk_bounds(coord: ChunkCoord, space: &SpaceConfig) -> Option<ChunkBounds> {
    let (gx, gy, gz) = chunk_grid_dims(space);
    if coord.x < 0 || coord.x >= gx || coord.y < 0 || coord.y >= gy || coord.z < 0 || coord.z >= gz
    {
        return None;
    }

    let min_x = coord.x as i64 * CHUNK_SIZE_X_CM;
    let min_y = coord.y as i64 * CHUNK_SIZE_Y_CM;
    let min_z = coord.z as i64 * CHUNK_SIZE_Z_CM;

    let max_x = ((coord.x as i64 + 1) * CHUNK_SIZE_X_CM).min(space.width_cm);
    let max_y = ((coord.y as i64 + 1) * CHUNK_SIZE_Y_CM).min(space.depth_cm);
    let max_z = ((coord.z as i64 + 1) * CHUNK_SIZE_Z_CM).min(space.height_cm);

    Some(ChunkBounds {
        min: GeoPos::new(min_x, min_y, min_z),
        max: GeoPos::new(max_x, max_y, max_z),
    })
}

pub fn chunk_seed(world_seed: u64, coord: ChunkCoord) -> u64 {
    let mut x = world_seed ^ 0x9E37_79B9_7F4A_7C15;
    x ^= zigzag_i32(coord.x).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = splitmix64(x);
    x ^= zigzag_i32(coord.y).wrapping_mul(0x94D0_49BB_1331_11EB);
    x = splitmix64(x);
    x ^= zigzag_i32(coord.z).wrapping_mul(0xD6E8_FEB8_6659_FD93);
    splitmix64(x)
}

fn ceil_div_i64(value: i64, divisor: i64) -> i64 {
    if value <= 0 {
        return 0;
    }
    (value + divisor - 1) / divisor
}

fn clamp_chunk_index(index: i32, grid: i32) -> i32 {
    if grid <= 0 {
        return 0;
    }
    if index < 0 {
        0
    } else if index >= grid {
        grid - 1
    } else {
        index
    }
}

fn zigzag_i32(value: i32) -> u64 {
    let v = value as i64;
    ((v << 1) ^ (v >> 63)) as u64
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}
