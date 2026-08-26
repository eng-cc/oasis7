use std::env;
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

pub(super) fn default_gateway_health_url() -> String {
    let config_path = env::var("HOME").ok().map(PathBuf::from).map(|home| {
        let dir = [".open", "claw"].concat();
        let file = ["open", "claw", ".json"].concat();
        home.join(dir).join(file)
    });
    if let Some(config_path) = config_path {
        if let Ok(raw) = fs::read_to_string(config_path) {
            if let Ok(value) = serde_json::from_str::<Value>(raw.as_str()) {
                if let Some(port) = value
                    .get("gateway")
                    .and_then(|gateway| gateway.get("port"))
                    .and_then(Value::as_u64)
                {
                    return format!("http://127.0.0.1:{port}/health");
                }
            }
        }
    }
    "http://127.0.0.1:18789/health".to_string()
}
