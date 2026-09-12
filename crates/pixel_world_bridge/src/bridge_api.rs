use super::*;

#[wasm_bindgen]
impl PixelWorldBridge {
    #[wasm_bindgen(constructor)]
    pub fn new(on_event: Function, on_fatal: Function) -> Self {
        Self {
            mounted: false,
            on_event,
            on_fatal,
        }
    }

    #[wasm_bindgen]
    pub fn mount(&mut self, canvas: HtmlCanvasElement, initial_render_state: JsValue) -> JsValue {
        let parsed_state = match parse_render_state(initial_render_state) {
            Ok(state) => state,
            Err(error) => return emit_fatal_payload(&error.as_string().unwrap_or_default()),
        };
        let canvas_id = if canvas.id().is_empty() {
            let generated = "pixel-world-embedded-runtime-canvas".to_string();
            canvas.set_id(&generated);
            generated
        } else {
            canvas.id()
        };
        let canvas_selector = format!("#{canvas_id}");

        let mount_result = BRIDGE_SHARED.with(|shared| {
            let mut shared = shared.borrow_mut();
            if let Some(existing_selector) = &shared.canvas_selector
                && existing_selector != &canvas_selector
            {
                return Err(format!(
                    "bevy runtime already bound to {existing_selector}, cannot rebind to {canvas_selector}"
                ));
            }
            shared.canvas_selector = Some(canvas_selector.clone());
            shared.render_state = Some(parsed_state);
            shared.render_version += 1;
            shared.mounted = true;
            shared.on_event = Some(self.on_event.clone());
            shared.on_fatal = Some(self.on_fatal.clone());
            let should_boot = !shared.booted;
            if should_boot {
                shared.booted = true;
            }
            Ok(should_boot)
        });

        let should_boot = match mount_result {
            Ok(should_boot) => should_boot,
            Err(message) => return emit_fatal_payload(&message),
        };

        self.mounted = true;

        if should_boot {
            boot_bevy_app(canvas_selector);
        }

        let _ = emit_event_value(&json!({ "type": "canvas_ready" }));
        let _ = emit_camera_state(&CameraState::default());
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn update(&mut self, next_render_state: JsValue) -> JsValue {
        if !self.mounted {
            return status_value("detached");
        }
        let parsed_state = match parse_render_state(next_render_state) {
            Ok(state) => state,
            Err(error) => return emit_fatal_payload(&error.as_string().unwrap_or_default()),
        };
        BRIDGE_SHARED.with(|shared| {
            let mut shared = shared.borrow_mut();
            shared.render_state = Some(parsed_state);
            shared.render_version += 1;
        });
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn hotspot_test_hit_targets(&self, contract: String) -> JsValue {
        if !self.mounted || contract != HOTSPOT_TEST_READBACK_CONTRACT {
            return JsValue::NULL;
        }
        BRIDGE_SHARED.with(|shared| {
            js_value_from_serializable(&shared.borrow().hotspot_test_targets)
                .unwrap_or(JsValue::NULL)
        })
    }

    #[wasm_bindgen]
    pub fn location_test_hit_targets(&self, contract: String) -> JsValue {
        if !self.mounted || contract != LOCATION_TEST_READBACK_CONTRACT {
            return JsValue::NULL;
        }
        BRIDGE_SHARED.with(|shared| {
            js_value_from_serializable(&shared.borrow().location_test_targets)
                .unwrap_or(JsValue::NULL)
        })
    }

    #[wasm_bindgen]
    pub fn pointer_down(&mut self, x: f64, y: f64, pointer_id: i32) -> JsValue {
        push_input_event(InputEvent::PointerDown { x, y, pointer_id });
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn pointer_move(&mut self, x: f64, y: f64, is_leave: bool, pointer_id: i32) -> JsValue {
        push_input_event(InputEvent::PointerMove {
            x,
            y,
            is_leave,
            pointer_id,
        });
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn pointer_up(&mut self, pointer_id: i32) -> JsValue {
        push_input_event(InputEvent::PointerUp { pointer_id });
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn wheel(&mut self, delta_y: f64) -> JsValue {
        push_input_event(InputEvent::Wheel { delta_y });
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn click(&mut self, x: f64, y: f64) -> JsValue {
        push_input_event(InputEvent::Click { x, y });
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn unmount(&mut self) -> JsValue {
        self.mounted = false;
        BRIDGE_SHARED.with(|shared| {
            let mut shared = shared.borrow_mut();
            shared.mounted = false;
            shared.render_state = None;
            shared.render_version += 1;
            shared.input_events.clear();
            shared.hotspot_test_targets.clear();
            shared.location_test_targets.clear();
        });
        status_value("detached")
    }
}
