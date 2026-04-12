#[cfg(target_arch = "wasm32")]
use ed25519_dalek::{Signer, SigningKey};
#[cfg(not(target_arch = "wasm32"))]
use oasis7::consensus_action_payload::sign_main_token_runtime_action_auth;
#[cfg(not(target_arch = "wasm32"))]
use oasis7::runtime::Action;
#[cfg(target_arch = "wasm32")]
use serde::Serialize;

use super::WebTransferSubmitRequest;

#[cfg(target_arch = "wasm32")]
const MAIN_TOKEN_ACTION_AUTH_PAYLOAD_VERSION: u8 = 1;
#[cfg(target_arch = "wasm32")]
const MAIN_TOKEN_TRANSFER_AUTH_SIGNATURE_V1_PREFIX: &str = "octransferauth:v1:";
const VIEWER_AUTH_PUBLIC_KEY_ENV: &str = "OASIS7_VIEWER_AUTH_PUBLIC_KEY";
const VIEWER_AUTH_PRIVATE_KEY_ENV: &str = "OASIS7_VIEWER_AUTH_PRIVATE_KEY";
#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_CONFIG_PATH: &str = "config.toml";
#[cfg(not(target_arch = "wasm32"))]
const NODE_TABLE_KEY: &str = "node";
#[cfg(not(target_arch = "wasm32"))]
const NODE_PRIVATE_KEY_FIELD: &str = "private_key";
#[cfg(not(target_arch = "wasm32"))]
const NODE_PUBLIC_KEY_FIELD: &str = "public_key";
#[cfg(target_arch = "wasm32")]
const VIEWER_AUTH_BOOTSTRAP_OBJECT: &str = "__OASIS7_VIEWER_AUTH_ENV";

#[derive(Debug, Clone, PartialEq, Eq)]
struct TransferAuthSigner {
    public_key: String,
    private_key: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Serialize)]
struct TransferActionData<'a> {
    from_account_id: &'a str,
    to_account_id: &'a str,
    amount: u64,
    nonce: u64,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "data")]
enum TransferActionEnvelope<'a> {
    TransferMainToken(TransferActionData<'a>),
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Serialize)]
struct MainTokenTransferSigningEnvelope<'a> {
    version: u8,
    operation: &'static str,
    account_id: &'a str,
    public_key: &'a str,
    action: TransferActionEnvelope<'a>,
}

pub(super) fn build_signed_web_transfer_submit_request(
    from_account_id: &str,
    to_account_id: &str,
    amount: u64,
    nonce: u64,
) -> Result<WebTransferSubmitRequest, String> {
    let signer = resolve_transfer_auth_signer()?;
    let from_account_id = from_account_id.trim().to_string();
    let to_account_id = to_account_id.trim().to_string();
    let (public_key, signature) = sign_transfer_request(
        signer,
        from_account_id.as_str(),
        to_account_id.as_str(),
        amount,
        nonce,
    )?;
    Ok(WebTransferSubmitRequest {
        from_account_id,
        to_account_id,
        amount,
        nonce,
        public_key,
        signature,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn sign_transfer_request(
    signer: TransferAuthSigner,
    from_account_id: &str,
    to_account_id: &str,
    amount: u64,
    nonce: u64,
) -> Result<(String, String), String> {
    let action = Action::TransferMainToken {
        from_account_id: from_account_id.to_string(),
        to_account_id: to_account_id.to_string(),
        amount,
        nonce,
    };
    let proof = sign_main_token_runtime_action_auth(
        &action,
        from_account_id,
        signer.public_key.as_str(),
        signer.private_key.as_str(),
    )
    .map_err(|err| format!("sign main-token transfer request failed: {err}"))?;
    let public_key = proof
        .public_key
        .ok_or_else(|| "signed main-token transfer proof missing public_key".to_string())?;
    let signature = proof
        .signature
        .ok_or_else(|| "signed main-token transfer proof missing signature".to_string())?;
    Ok((public_key, signature))
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_transfer_auth_signer() -> Result<TransferAuthSigner, String> {
    if let Some(signer) = resolve_transfer_auth_signer_from_env()? {
        return Ok(signer);
    }
    resolve_transfer_auth_signer_from_path(std::path::Path::new(DEFAULT_CONFIG_PATH))
        .map_err(|err| format!("transfer signer bootstrap is unavailable: {err}"))
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_transfer_auth_signer_from_env() -> Result<Option<TransferAuthSigner>, String> {
    let public_key = std::env::var(VIEWER_AUTH_PUBLIC_KEY_ENV)
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|value| !value.is_empty());
    let private_key = std::env::var(VIEWER_AUTH_PRIVATE_KEY_ENV)
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|value| !value.is_empty());
    match (public_key, private_key) {
        (Some(public_key), Some(private_key)) => Ok(Some(TransferAuthSigner {
            public_key,
            private_key,
        })),
        (None, None) => Ok(None),
        (Some(_), None) => Err(format!("{VIEWER_AUTH_PRIVATE_KEY_ENV} is not set")),
        (None, Some(_)) => Err(format!("{VIEWER_AUTH_PUBLIC_KEY_ENV} is not set")),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_transfer_auth_signer_from_path(
    path: &std::path::Path,
) -> Result<TransferAuthSigner, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|err| format!("read {} failed: {err}", path.display()))?;
    let value: toml::Value = toml::from_str(content.as_str())
        .map_err(|err| format!("parse {} failed: {err}", path.display()))?;
    let node = value
        .get(NODE_TABLE_KEY)
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("{NODE_TABLE_KEY} table is missing in {}", path.display()))?;
    Ok(TransferAuthSigner {
        public_key: resolve_required_toml_string(node, NODE_PUBLIC_KEY_FIELD, "node.public_key")?,
        private_key: resolve_required_toml_string(
            node,
            NODE_PRIVATE_KEY_FIELD,
            "node.private_key",
        )?,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_required_toml_string(
    table: &toml::value::Table,
    key: &str,
    label: &str,
) -> Result<String, String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("{label} is missing or empty"))
}

#[cfg(target_arch = "wasm32")]
fn resolve_transfer_auth_signer() -> Result<TransferAuthSigner, String> {
    let window = web_sys::window().ok_or_else(|| "window is unavailable".to_string())?;
    let bootstrap = js_sys::Reflect::get(
        window.as_ref(),
        &web_sys::wasm_bindgen::JsValue::from_str(VIEWER_AUTH_BOOTSTRAP_OBJECT),
    )
    .map_err(|_| "viewer auth bootstrap lookup failed".to_string())?;
    if bootstrap.is_null() || bootstrap.is_undefined() {
        return Err("viewer auth bootstrap is unavailable".to_string());
    }
    Ok(TransferAuthSigner {
        public_key: resolve_bootstrap_string(&bootstrap, VIEWER_AUTH_PUBLIC_KEY_ENV)?,
        private_key: resolve_bootstrap_string(&bootstrap, VIEWER_AUTH_PRIVATE_KEY_ENV)?,
    })
}

#[cfg(target_arch = "wasm32")]
fn resolve_bootstrap_string(
    bootstrap: &web_sys::wasm_bindgen::JsValue,
    key: &str,
) -> Result<String, String> {
    let value = js_sys::Reflect::get(bootstrap, &web_sys::wasm_bindgen::JsValue::from_str(key))
        .map_err(|_| format!("{key} lookup failed"))?;
    value
        .as_string()
        .map(|raw| raw.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{key} is missing"))
}

#[cfg(target_arch = "wasm32")]
fn sign_transfer_request(
    signer: TransferAuthSigner,
    from_account_id: &str,
    to_account_id: &str,
    amount: u64,
    nonce: u64,
) -> Result<(String, String), String> {
    let public_key = normalize_hex_array::<32>(
        signer.public_key.as_str(),
        "main token auth signer public key",
    )?;
    let private_key = decode_hex_array::<32>(
        signer.private_key.as_str(),
        "main token auth signer private key",
    )?;
    let signing_key = SigningKey::from_bytes(&private_key);
    let expected_public_key = hex::encode(signing_key.verifying_key().to_bytes());
    if expected_public_key != public_key {
        return Err(format!(
            "main token auth signer public key does not match private key: expected={expected_public_key} actual={public_key}"
        ));
    }
    let expected_account_id = format!("oc:pk:{public_key}");
    if from_account_id.trim() != expected_account_id {
        return Err(format!(
            "main token auth account_id does not match signer public key: expected={expected_account_id} actual={}",
            from_account_id.trim()
        ));
    }
    let payload = build_transfer_signing_payload(
        from_account_id.trim(),
        to_account_id.trim(),
        amount,
        nonce,
        public_key.as_str(),
    )?;
    let signature = signing_key.sign(payload.as_slice());
    Ok((
        public_key,
        format!(
            "{}{}",
            MAIN_TOKEN_TRANSFER_AUTH_SIGNATURE_V1_PREFIX,
            hex::encode(signature.to_bytes())
        ),
    ))
}

#[cfg(target_arch = "wasm32")]
fn build_transfer_signing_payload(
    from_account_id: &str,
    to_account_id: &str,
    amount: u64,
    nonce: u64,
    public_key: &str,
) -> Result<Vec<u8>, String> {
    let envelope = MainTokenTransferSigningEnvelope {
        version: MAIN_TOKEN_ACTION_AUTH_PAYLOAD_VERSION,
        operation: "transfer_main_token",
        account_id: from_account_id,
        public_key,
        action: TransferActionEnvelope::TransferMainToken(TransferActionData {
            from_account_id,
            to_account_id,
            amount,
            nonce,
        }),
    };
    serde_json::to_vec(&envelope)
        .map_err(|err| format!("encode main token auth signing payload failed: {err}"))
}

#[cfg(target_arch = "wasm32")]
fn normalize_hex_array<const N: usize>(raw: &str, label: &str) -> Result<String, String> {
    let bytes = decode_hex_array::<N>(raw, label)?;
    Ok(hex::encode(bytes))
}

#[cfg(target_arch = "wasm32")]
fn decode_hex_array<const N: usize>(raw: &str, label: &str) -> Result<[u8; N], String> {
    let bytes = hex::decode(raw.trim()).map_err(|err| format!("decode {label} failed: {err}"))?;
    if bytes.len() != N {
        return Err(format!(
            "{label} length mismatch: expected {N} bytes, got {}",
            bytes.len()
        ));
    }
    let mut fixed = [0_u8; N];
    fixed.copy_from_slice(bytes.as_slice());
    Ok(fixed)
}

#[cfg(test)]
mod tests {
    use super::{
        build_signed_web_transfer_submit_request, resolve_transfer_auth_signer_from_env,
        resolve_transfer_auth_signer_from_path, VIEWER_AUTH_PRIVATE_KEY_ENV,
        VIEWER_AUTH_PUBLIC_KEY_ENV,
    };
    use oasis7::consensus_action_payload::{
        sign_main_token_runtime_action_auth, verify_main_token_runtime_action_auth,
        MainTokenActionAuthScheme,
    };
    use oasis7::runtime::Action;
    use serde::Serialize;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_signer(seed: u8) -> (String, String) {
        let private_key = [seed; 32];
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key);
        (
            hex::encode(signing_key.verifying_key().to_bytes()),
            hex::encode(private_key),
        )
    }

    fn temp_config_path(label: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        path.push(format!(
            "oasis7_client_launcher_transfer_auth_{label}_{}_{}.toml",
            std::process::id(),
            stamp
        ));
        path
    }

    #[derive(Debug, Serialize)]
    struct WasmTransferActionData<'a> {
        from_account_id: &'a str,
        to_account_id: &'a str,
        amount: u64,
        nonce: u64,
    }

    #[derive(Debug, Serialize)]
    #[serde(tag = "type", content = "data")]
    enum WasmTransferActionEnvelope<'a> {
        TransferMainToken(WasmTransferActionData<'a>),
    }

    #[derive(Debug, Serialize)]
    struct WasmMainTokenTransferSigningEnvelope<'a> {
        version: u8,
        operation: &'static str,
        account_id: &'a str,
        public_key: &'a str,
        action: WasmTransferActionEnvelope<'a>,
    }

    #[derive(Debug, Serialize)]
    struct NativeTransferActionData<'a> {
        from_account_id: &'a str,
        to_account_id: &'a str,
        amount: u64,
        nonce: u64,
    }

    #[derive(Debug, Serialize)]
    #[serde(tag = "type", content = "data")]
    enum NativeTransferActionEnvelope<'a> {
        TransferMainToken(NativeTransferActionData<'a>),
    }

    #[derive(Debug, Serialize)]
    struct NativeMainTokenTransferSigningEnvelope<'a> {
        version: u8,
        operation: &'static str,
        account_id: &'a str,
        public_key: &'a str,
        action: NativeTransferActionEnvelope<'a>,
    }

    #[test]
    fn resolve_transfer_auth_signer_from_env_requires_both_keys() {
        std::env::remove_var(VIEWER_AUTH_PUBLIC_KEY_ENV);
        std::env::set_var(VIEWER_AUTH_PRIVATE_KEY_ENV, "private");
        let err = resolve_transfer_auth_signer_from_env().expect_err("missing public key");
        assert!(err.contains(VIEWER_AUTH_PUBLIC_KEY_ENV));
        std::env::remove_var(VIEWER_AUTH_PRIVATE_KEY_ENV);
    }

    #[test]
    fn resolve_transfer_auth_signer_from_path_reads_node_keys() {
        let config_path = temp_config_path("node_keys");
        fs::write(
            &config_path,
            "[node]\nprivate_key = \"private-key-hex\"\npublic_key = \"public-key-hex\"\n",
        )
        .expect("write config");
        let signer = resolve_transfer_auth_signer_from_path(config_path.as_path()).expect("signer");
        assert_eq!(signer.public_key, "public-key-hex");
        assert_eq!(signer.private_key, "private-key-hex");
        let _ = fs::remove_file(config_path);
    }

    #[test]
    fn build_signed_web_transfer_submit_request_includes_auth_fields() {
        let (public_key, private_key) = test_signer(21);
        let from_account_id = format!("oc:pk:{public_key}");
        let action = Action::TransferMainToken {
            from_account_id: from_account_id.clone(),
            to_account_id: "protocol:treasury".to_string(),
            amount: 7,
            nonce: 3,
        };
        std::env::set_var(VIEWER_AUTH_PUBLIC_KEY_ENV, public_key.as_str());
        std::env::set_var(VIEWER_AUTH_PRIVATE_KEY_ENV, private_key.as_str());
        let request = build_signed_web_transfer_submit_request(
            from_account_id.as_str(),
            "protocol:treasury",
            7,
            3,
        )
        .expect("signed request");
        let verified = verify_main_token_runtime_action_auth(
            &action,
            &oasis7::consensus_action_payload::MainTokenActionAuthProof {
                scheme: MainTokenActionAuthScheme::Ed25519,
                account_id: request.from_account_id.clone(),
                public_key: Some(request.public_key.clone()),
                signature: Some(request.signature.clone()),
                threshold: None,
                participant_signatures: Vec::new(),
            },
        )
        .expect("verify");
        assert_eq!(verified.account_id, from_account_id);
        assert_eq!(verified.signer_public_keys, vec![public_key.clone()]);
        std::env::remove_var(VIEWER_AUTH_PUBLIC_KEY_ENV);
        std::env::remove_var(VIEWER_AUTH_PRIVATE_KEY_ENV);
    }

    #[test]
    fn wasm_transfer_signing_payload_matches_runtime_helper_shape() {
        let (public_key, private_key) = test_signer(23);
        let from_account_id = format!("oc:pk:{public_key}");
        let action = Action::TransferMainToken {
            from_account_id: from_account_id.clone(),
            to_account_id: "protocol:treasury".to_string(),
            amount: 7,
            nonce: 9,
        };
        let proof = sign_main_token_runtime_action_auth(
            &action,
            from_account_id.as_str(),
            public_key.as_str(),
            private_key.as_str(),
        )
        .expect("native proof");

        let native_payload = serde_json::to_vec(&NativeMainTokenTransferSigningEnvelope {
            version: 1,
            operation: "transfer_main_token",
            account_id: from_account_id.as_str(),
            public_key: public_key.as_str(),
            action: NativeTransferActionEnvelope::TransferMainToken(NativeTransferActionData {
                from_account_id: from_account_id.as_str(),
                to_account_id: "protocol:treasury",
                amount: 7,
                nonce: 9,
            }),
        })
        .expect("native payload");

        let wasm_payload = serde_json::to_vec(&WasmMainTokenTransferSigningEnvelope {
            version: 1,
            operation: "transfer_main_token",
            account_id: from_account_id.as_str(),
            public_key: public_key.as_str(),
            action: WasmTransferActionEnvelope::TransferMainToken(WasmTransferActionData {
                from_account_id: from_account_id.as_str(),
                to_account_id: "protocol:treasury",
                amount: 7,
                nonce: 9,
            }),
        })
        .expect("wasm payload");

        assert_eq!(
            String::from_utf8(native_payload.clone()).expect("native utf8"),
            String::from_utf8(wasm_payload.clone()).expect("wasm utf8"),
        );

        let signature = proof.signature.expect("signature");
        let signature_hex = signature
            .strip_prefix("octransferauth:v1:")
            .expect("transfer prefix");
        let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(
            &hex::decode(public_key.as_str())
                .expect("public key hex")
                .try_into()
                .expect("32-byte public key"),
        )
        .expect("verifying key");
        verifying_key
            .verify_strict(
                wasm_payload.as_slice(),
                &ed25519_dalek::Signature::from_bytes(
                    &hex::decode(signature_hex)
                        .expect("signature hex")
                        .try_into()
                        .expect("64-byte signature"),
                ),
            )
            .expect("wasm payload should verify against runtime helper signature");
    }
}
