use super::LauncherUxState;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) const UX_STATE_PATH: &str = ".oasis7_launcher_ux_state.json";
#[cfg(target_arch = "wasm32")]
const UX_STATE_STORAGE_KEY: &str = "oasis7_launcher_ux_state_v1";

pub(crate) fn load_launcher_ux_state() -> LauncherUxState {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let content = std::fs::read_to_string(UX_STATE_PATH);
        let Ok(content) = content else {
            return LauncherUxState::default();
        };
        return serde_json::from_str::<LauncherUxState>(content.as_str())
            .unwrap_or_else(|_| LauncherUxState::default());
    }

    #[cfg(target_arch = "wasm32")]
    {
        let Some(window) = web_sys::window() else {
            return LauncherUxState::default();
        };
        let Ok(Some(storage)) = window.local_storage() else {
            return LauncherUxState::default();
        };
        let content = storage.get_item(UX_STATE_STORAGE_KEY);
        let Ok(Some(content)) = content else {
            return LauncherUxState::default();
        };
        return serde_json::from_str::<LauncherUxState>(content.as_str())
            .unwrap_or_else(|_| LauncherUxState::default());
    }
}

#[cfg(not(test))]
pub(crate) fn save_launcher_ux_state(state: &LauncherUxState) -> Result<(), String> {
    let content = serde_json::to_string(state)
        .map_err(|err| format!("serialize launcher ux state failed: {err}"))?;

    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::write(UX_STATE_PATH, content.as_bytes())
            .map_err(|err| format!("write launcher ux state failed: {err}"))
    }

    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window().ok_or_else(|| "missing browser window".to_string())?;
        let storage = window
            .local_storage()
            .map_err(|err| format!("query localStorage failed: {err:?}"))?
            .ok_or_else(|| "localStorage unavailable".to_string())?;
        storage
            .set_item(UX_STATE_STORAGE_KEY, content.as_str())
            .map_err(|err| format!("persist launcher ux state failed: {err:?}"))
    }
}

pub(super) fn current_unix_ms() -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        use web_time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        return i64::try_from(now.as_millis()).unwrap_or(i64::MAX);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        i64::try_from(now.as_millis()).unwrap_or(i64::MAX)
    }
}
