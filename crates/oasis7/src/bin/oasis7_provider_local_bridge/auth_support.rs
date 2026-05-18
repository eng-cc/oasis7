use super::*;
use std::time::SystemTime;

#[derive(Debug, Clone, Default)]
pub(super) struct NewapiBridgeStateCache {
    state_path: Option<String>,
    modified_at: Option<SystemTime>,
    payload: Option<Value>,
}

pub(super) fn load_cached_newapi_bridge_state(
    cache: &Arc<Mutex<NewapiBridgeStateCache>>,
) -> Option<Value> {
    let state_path = env::var("OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())?;
    let metadata = fs::metadata(state_path.as_str()).ok()?;
    let modified_at = metadata.modified().ok()?;
    let mut cache = cache.lock().expect("newapi_bridge_state_cache lock");
    if cache.state_path.as_deref() == Some(state_path.as_str())
        && cache.modified_at == Some(modified_at)
    {
        return cache.payload.clone();
    }
    let raw = fs::read_to_string(state_path.as_str()).ok()?;
    let payload = serde_json::from_str::<Value>(raw.as_str()).ok()?;
    cache.state_path = Some(state_path);
    cache.modified_at = Some(modified_at);
    cache.payload = Some(payload.clone());
    Some(payload)
}

pub(super) fn parse_newapi_bridge_bearer_selector(
    normalized: &str,
) -> Option<(Option<&str>, Option<&str>)> {
    if let Some((prefix, value)) = normalized.split_once(':') {
        let value = value.trim();
        return match prefix {
            "newapi_user_ref" if !value.is_empty() => Some((Some(value), None)),
            "bridge_user_id" if !value.is_empty() => Some((None, Some(value))),
            _ => None,
        };
    }
    if normalized.len() < MIN_BRIDGE_AUTH_TOKEN_LEN {
        return None;
    }
    Some((Some(normalized), Some(normalized)))
}
