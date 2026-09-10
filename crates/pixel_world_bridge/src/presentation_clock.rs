//! Presentation-only animation clock and motion preference controls.
use super::{BRIDGE_SHARED, PixelWorldBridge, status_value};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl PixelWorldBridge {
    #[wasm_bindgen]
    pub fn set_reduced_motion(&mut self, reduced_motion: bool) {
        BRIDGE_SHARED.with(|shared| {
            let mut shared = shared.borrow_mut();
            if shared.reduced_motion != reduced_motion {
                shared.reduced_motion = reduced_motion;
                shared.animation_version = shared.animation_version.wrapping_add(1);
            }
        });
    }

    #[wasm_bindgen]
    pub fn tick(&mut self, _animation_ms: f64) -> JsValue {
        if self.mounted {
            BRIDGE_SHARED.with(|shared| {
                let mut shared = shared.borrow_mut();
                shared.animation_version = shared.animation_version.wrapping_add(1);
            });
            status_value("ready")
        } else {
            status_value("detached")
        }
    }
}
