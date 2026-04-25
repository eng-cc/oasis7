use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

pub(super) const VIEWER_PLAYER_ID: &str = "viewer-player";
pub(super) const VIEWER_PLAYER_ID_ENV: &str = "OASIS7_VIEWER_PLAYER_ID";
pub(super) const VIEWER_AUTH_PUBLIC_KEY_ENV: &str = "OASIS7_VIEWER_AUTH_PUBLIC_KEY";
pub(super) const VIEWER_AUTH_PRIVATE_KEY_ENV: &str = "OASIS7_VIEWER_AUTH_PRIVATE_KEY";

#[cfg(target_arch = "wasm32")]
const VIEWER_AUTH_BOOTSTRAP_OBJECT: &str = "__OASIS7_VIEWER_AUTH_ENV";
#[cfg(not(target_arch = "wasm32"))]
const NODE_CONFIG_FILE_NAME: &str = "config.toml";
#[cfg(not(target_arch = "wasm32"))]
const NODE_TABLE_KEY: &str = "node";
#[cfg(not(target_arch = "wasm32"))]
const NODE_PRIVATE_KEY_FIELD: &str = "private_key";
#[cfg(not(target_arch = "wasm32"))]
const NODE_PUBLIC_KEY_FIELD: &str = "public_key";

static VIEWER_AUTH_NONCE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ViewerAuthSigner {
    pub(super) player_id: String,
    pub(super) public_key: String,
    pub(super) private_key: String,
}

fn resolve_env_trimmed<F>(get_env: &F, key: &str) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    get_env(key)
        .map(|raw| raw.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn resolve_viewer_player_id_from<F>(get_env: &F) -> Result<String, String>
where
    F: Fn(&str) -> Option<String>,
{
    match resolve_env_trimmed(get_env, VIEWER_PLAYER_ID_ENV) {
        Some(value) => Ok(value),
        None => Ok(VIEWER_PLAYER_ID.to_string()),
    }
}

pub(super) fn resolve_viewer_auth_signer_from<F>(get_env: F) -> Result<ViewerAuthSigner, String>
where
    F: Fn(&str) -> Option<String>,
{
    let player_id = resolve_viewer_player_id_from(&get_env)?;
    let public_key = resolve_env_trimmed(&get_env, VIEWER_AUTH_PUBLIC_KEY_ENV)
        .ok_or_else(|| format!("{VIEWER_AUTH_PUBLIC_KEY_ENV} is not set"))?;
    let private_key = resolve_env_trimmed(&get_env, VIEWER_AUTH_PRIVATE_KEY_ENV)
        .ok_or_else(|| format!("{VIEWER_AUTH_PRIVATE_KEY_ENV} is not set"))?;
    Ok(ViewerAuthSigner {
        player_id,
        public_key,
        private_key,
    })
}

pub(super) fn resolve_viewer_auth_signer() -> Result<ViewerAuthSigner, String> {
    let runtime_result = resolve_viewer_auth_signer_from(runtime_auth_value);
    match runtime_result {
        Ok(signer) => Ok(signer),
        Err(runtime_err) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                match resolve_viewer_auth_signer_from_node_config(std::path::Path::new(
                    NODE_CONFIG_FILE_NAME,
                )) {
                    Ok(signer) => return Ok(signer),
                    Err(config_err) => {
                        return Err(format!(
                            "{runtime_err}; fallback {NODE_CONFIG_FILE_NAME} failed: {config_err}"
                        ))
                    }
                }
            }
            #[cfg(target_arch = "wasm32")]
            {
                Err(runtime_err)
            }
        }
    }
}

fn runtime_auth_value(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    if let Some(value) = resolve_wasm_viewer_auth_value(key) {
        return Some(value);
    }
    std::env::var(key).ok()
}

#[cfg(target_arch = "wasm32")]
fn resolve_wasm_viewer_auth_value(key: &str) -> Option<String> {
    let window = web_sys::window()?;
    let store = js_sys::Reflect::get(
        window.as_ref(),
        &JsValue::from_str(VIEWER_AUTH_BOOTSTRAP_OBJECT),
    )
    .ok()?;
    if store.is_null() || store.is_undefined() {
        return None;
    }
    js_sys::Reflect::get(&store, &JsValue::from_str(key))
        .ok()?
        .as_string()
        .map(|raw| raw.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn resolve_viewer_auth_signer_from_node_config(
    path: &std::path::Path,
) -> Result<ViewerAuthSigner, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|err| format!("read {} failed: {err}", path.display()))?;
    let value: toml::Value = toml::from_str(content.as_str())
        .map_err(|err| format!("parse {} failed: {err}", path.display()))?;
    let table = value
        .as_table()
        .ok_or_else(|| format!("{} root must be a table", path.display()))?;
    let node = table
        .get(NODE_TABLE_KEY)
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("{NODE_TABLE_KEY} table is missing in {}", path.display()))?;
    let private_key =
        resolve_required_toml_string(node, NODE_PRIVATE_KEY_FIELD, "node.private_key")?;
    let public_key = resolve_required_toml_string(node, NODE_PUBLIC_KEY_FIELD, "node.public_key")?;
    let player_id = resolve_viewer_player_id_from(&runtime_auth_value)?;
    Ok(ViewerAuthSigner {
        player_id,
        public_key,
        private_key,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_required_toml_string(
    table: &toml::value::Table,
    key: &str,
    label: &str,
) -> Result<String, String> {
    let value = table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{label} is missing or empty"))?;
    Ok(value.to_string())
}

pub(super) fn next_viewer_auth_nonce() -> Result<u64, String> {
    let nonce = VIEWER_AUTH_NONCE_COUNTER.fetch_add(1, Ordering::SeqCst);
    if nonce == 0 {
        return Err("viewer auth nonce exhausted".to_string());
    }
    Ok(nonce)
}

pub(super) fn attach_prompt_control_apply_auth(
    request: &mut oasis7::viewer::PromptControlApplyRequest,
    signer: &ViewerAuthSigner,
    nonce: u64,
    intent: oasis7::viewer::PromptControlAuthIntent,
) -> Result<(), String> {
    request.player_id = signer.player_id.clone();
    request.updated_by = Some(signer.player_id.clone());
    request.public_key = Some(signer.public_key.clone());
    request.auth = None;
    let proof = oasis7::viewer::sign_prompt_control_apply_auth_proof(
        intent,
        request,
        nonce,
        signer.public_key.as_str(),
        signer.private_key.as_str(),
    )?;
    request.auth = Some(proof);
    Ok(())
}

pub(super) fn attach_agent_chat_auth(
    request: &mut oasis7::viewer::AgentChatRequest,
    signer: &ViewerAuthSigner,
    nonce: u64,
) -> Result<(), String> {
    if request.intent_seq.is_none() {
        request.intent_seq = Some(nonce);
    }
    request.player_id = Some(signer.player_id.clone());
    request.public_key = Some(signer.public_key.clone());
    request.auth = None;
    let proof = oasis7::viewer::sign_agent_chat_auth_proof(
        request,
        nonce,
        signer.public_key.as_str(),
        signer.private_key.as_str(),
    )?;
    request.auth = Some(proof);
    Ok(())
}

pub(super) fn attach_gameplay_action_auth(
    request: &mut oasis7::viewer::GameplayActionRequest,
    signer: &ViewerAuthSigner,
    nonce: u64,
) -> Result<(), String> {
    request.player_id = signer.player_id.clone();
    request.public_key = Some(signer.public_key.clone());
    request.auth = None;
    let proof = oasis7::viewer::sign_gameplay_action_auth_proof(
        request,
        nonce,
        signer.public_key.as_str(),
        signer.private_key.as_str(),
    )?;
    request.auth = Some(proof);
    Ok(())
}

pub(super) fn sign_prompt_control_apply_request(
    request: &mut oasis7::viewer::PromptControlApplyRequest,
    intent: oasis7::viewer::PromptControlAuthIntent,
) -> Result<(), String> {
    let signer = resolve_viewer_auth_signer()?;
    let nonce = next_viewer_auth_nonce()?;
    attach_prompt_control_apply_auth(request, &signer, nonce, intent)
}

pub(super) fn sign_agent_chat_request(
    request: &mut oasis7::viewer::AgentChatRequest,
) -> Result<(), String> {
    let signer = resolve_viewer_auth_signer()?;
    let nonce = next_viewer_auth_nonce()?;
    attach_agent_chat_auth(request, &signer, nonce)
}

pub(super) fn build_signed_gameplay_action_request(
    action_id: &str,
    target_agent_id: &str,
    actor_agent_id: Option<&str>,
) -> Result<oasis7::viewer::GameplayActionRequest, String> {
    let signer = resolve_viewer_auth_signer()?;
    let nonce = next_viewer_auth_nonce()?;
    let mut request = oasis7::viewer::GameplayActionRequest {
        action_id: action_id.to_string(),
        target_agent_id: target_agent_id.to_string(),
        actor_agent_id: actor_agent_id.map(ToOwned::to_owned),
        player_id: signer.player_id.clone(),
        public_key: None,
        auth: None,
    };
    attach_gameplay_action_auth(&mut request, &signer, nonce)?;
    Ok(request)
}

pub(super) fn build_session_register_request(
    requested_agent_id: Option<String>,
) -> Result<oasis7::viewer::AuthoritativeSessionRegisterRequest, String> {
    let signer = resolve_viewer_auth_signer()?;
    let nonce = next_viewer_auth_nonce()?;
    let mut request = oasis7::viewer::AuthoritativeSessionRegisterRequest {
        player_id: signer.player_id.clone(),
        public_key: Some(signer.public_key.clone()),
        auth: None,
        requested_agent_id,
        force_rebind: false,
    };
    let proof = oasis7::viewer::sign_session_register_auth_proof(
        &request,
        nonce,
        signer.public_key.as_str(),
        signer.private_key.as_str(),
    )?;
    request.auth = Some(proof);
    Ok(request)
}

pub(super) fn sync_viewer_auth_nonce_from_state(state: &ViewerState) {
    let Ok(player_id) = resolve_viewer_player_id_from(&runtime_auth_value) else {
        return;
    };
    let Some(snapshot) = state.snapshot.as_ref() else {
        return;
    };
    let Some(last_nonce) = snapshot
        .model
        .player_auth_last_nonce
        .get(player_id.as_str())
    else {
        return;
    };

    let desired = last_nonce.saturating_add(1).max(1);
    let mut current = VIEWER_AUTH_NONCE_COUNTER.load(Ordering::SeqCst);
    while current < desired {
        match VIEWER_AUTH_NONCE_COUNTER.compare_exchange(
            current,
            desired,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => break,
            Err(next) => current = next,
        }
    }
}

#[cfg(test)]
pub(super) fn viewer_auth_nonce_for_tests() -> u64 {
    VIEWER_AUTH_NONCE_COUNTER.load(Ordering::SeqCst)
}
