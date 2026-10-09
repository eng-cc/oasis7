use oasis7_client_api::{
    MainTokenTransfer, TRANSFER_TX_TYPE_ASSET_TRANSFER, TRANSFER_TX_VERSION_V2,
    sign_main_token_transfer,
};

use super::WebTransferSubmitRequest;

const VIEWER_AUTH_PUBLIC_KEY_ENV: &str = "OASIS7_VIEWER_AUTH_PUBLIC_KEY";
const VIEWER_AUTH_PRIVATE_KEY_ENV: &str = "OASIS7_VIEWER_AUTH_PRIVATE_KEY";
#[cfg(test)]
pub(crate) static TRANSFER_AUTH_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
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

pub(super) fn build_signed_web_transfer_submit_request(
    from_account_id: &str,
    to_account_id: &str,
    amount: u64,
    nonce: u64,
    chain_id: Option<&str>,
    network_id: Option<&str>,
) -> Result<WebTransferSubmitRequest, String> {
    let signer = resolve_transfer_auth_signer()?;
    let from_account_id = from_account_id.trim().to_string();
    let to_account_id = to_account_id.trim().to_string();
    let chain_id = chain_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let network_id = network_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let (public_key, signature) = sign_transfer_request(
        signer,
        from_account_id.as_str(),
        to_account_id.as_str(),
        amount,
        nonce,
        chain_id.as_deref(),
        network_id.as_deref(),
    )?;
    Ok(WebTransferSubmitRequest {
        from_account_id,
        to_account_id,
        amount,
        nonce,
        asset_id: Some("main_token".to_string()),
        memo: None,
        chain_id,
        network_id,
        tx_version: Some(TRANSFER_TX_VERSION_V2),
        tx_type: Some(TRANSFER_TX_TYPE_ASSET_TRANSFER.to_string()),
        valid_until_unix_ms: None,
        max_fee: None,
        fee_asset_id: None,
        application_payload_hash: None,
        client_request_id: None,
        public_key,
        signature,
    })
}

fn sign_transfer_request(
    signer: TransferAuthSigner,
    from_account_id: &str,
    to_account_id: &str,
    amount: u64,
    nonce: u64,
    chain_id: Option<&str>,
    network_id: Option<&str>,
) -> Result<(String, String), String> {
    let mut transfer = MainTokenTransfer::new(from_account_id, to_account_id, amount, nonce);
    transfer.chain_id = chain_id.map(ToOwned::to_owned);
    transfer.network_id = network_id.map(ToOwned::to_owned);
    let proof = sign_main_token_transfer(
        &transfer,
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

#[cfg(test)]
mod tests {
    use super::{
        VIEWER_AUTH_PRIVATE_KEY_ENV, VIEWER_AUTH_PUBLIC_KEY_ENV,
        build_signed_web_transfer_submit_request, resolve_transfer_auth_signer_from_env,
        resolve_transfer_auth_signer_from_path,
    };
    use oasis7_client_api::{
        MAIN_TOKEN_ACTION_AUTH_PAYLOAD_VERSION, MAIN_TOKEN_TRANSFER_AUTH_SIGNATURE_V2_PREFIX,
        MainTokenActionAuthProof, MainTokenActionAuthScheme, MainTokenTransfer,
        build_main_token_transfer_signing_payload, sign_main_token_transfer,
        verify_main_token_transfer_signature,
    };
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
        #[serde(skip_serializing_if = "Option::is_none")]
        asset_id: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        memo: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        chain_id: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        network_id: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tx_version: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tx_type: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        valid_until_unix_ms: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_fee: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        fee_asset_id: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        application_payload_hash: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        client_request_id: Option<&'a str>,
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
        #[serde(skip_serializing_if = "Option::is_none")]
        asset_id: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        memo: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        chain_id: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        network_id: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tx_version: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tx_type: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        valid_until_unix_ms: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_fee: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        fee_asset_id: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        application_payload_hash: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        client_request_id: Option<&'a str>,
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
        let _guard = super::TRANSFER_AUTH_ENV_LOCK.lock().expect("env lock");
        // SAFETY: This test/setup code mutates process environment in a controlled scope.
        unsafe {
            std::env::remove_var(VIEWER_AUTH_PUBLIC_KEY_ENV);
        }
        // SAFETY: This test/setup code mutates process environment in a controlled scope.
        unsafe {
            std::env::set_var(VIEWER_AUTH_PRIVATE_KEY_ENV, "private");
        }
        let err = resolve_transfer_auth_signer_from_env().expect_err("missing public key");
        assert!(err.contains(VIEWER_AUTH_PUBLIC_KEY_ENV));
        // SAFETY: This test/setup code mutates process environment in a controlled scope.
        unsafe {
            std::env::remove_var(VIEWER_AUTH_PRIVATE_KEY_ENV);
        }
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
        let _guard = super::TRANSFER_AUTH_ENV_LOCK.lock().expect("env lock");
        let (public_key, private_key) = test_signer(21);
        let from_account_id = format!("oc:pk:{public_key}");
        let transfer = MainTokenTransfer::new(from_account_id.clone(), "protocol:treasury", 7, 3);
        // SAFETY: This test/setup code mutates process environment in a controlled scope.
        unsafe {
            std::env::set_var(VIEWER_AUTH_PUBLIC_KEY_ENV, public_key.as_str());
        }
        // SAFETY: This test/setup code mutates process environment in a controlled scope.
        unsafe {
            std::env::set_var(VIEWER_AUTH_PRIVATE_KEY_ENV, private_key.as_str());
        }
        let request = build_signed_web_transfer_submit_request(
            from_account_id.as_str(),
            "protocol:treasury",
            7,
            3,
            None,
            None,
        )
        .expect("signed request");
        let verified = verify_main_token_transfer_signature(
            &transfer,
            &MainTokenActionAuthProof {
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
        // SAFETY: This test/setup code mutates process environment in a controlled scope.
        unsafe {
            std::env::remove_var(VIEWER_AUTH_PUBLIC_KEY_ENV);
        }
        // SAFETY: This test/setup code mutates process environment in a controlled scope.
        unsafe {
            std::env::remove_var(VIEWER_AUTH_PRIVATE_KEY_ENV);
        }
    }

    #[test]
    fn client_api_signing_payload_matches_native_and_web_wire_bytes() {
        let (public_key, private_key) = test_signer(23);
        let from_account_id = format!("oc:pk:{public_key}");
        let transfer = MainTokenTransfer::new(from_account_id.clone(), "protocol:treasury", 7, 9);
        let proof = sign_main_token_transfer(
            &transfer,
            from_account_id.as_str(),
            public_key.as_str(),
            private_key.as_str(),
        )
        .expect("native proof");

        let native_payload = serde_json::to_vec(&NativeMainTokenTransferSigningEnvelope {
            version: MAIN_TOKEN_ACTION_AUTH_PAYLOAD_VERSION,
            operation: "transfer_main_token",
            account_id: from_account_id.as_str(),
            public_key: public_key.as_str(),
            action: NativeTransferActionEnvelope::TransferMainToken(NativeTransferActionData {
                from_account_id: from_account_id.as_str(),
                to_account_id: "protocol:treasury",
                amount: 7,
                nonce: 9,
                asset_id: Some("main_token"),
                memo: None,
                chain_id: None,
                network_id: None,
                tx_version: Some(2),
                tx_type: Some("asset_transfer"),
                valid_until_unix_ms: None,
                max_fee: None,
                fee_asset_id: None,
                application_payload_hash: None,
                client_request_id: None,
            }),
        })
        .expect("native payload");

        let wasm_payload = serde_json::to_vec(&WasmMainTokenTransferSigningEnvelope {
            version: MAIN_TOKEN_ACTION_AUTH_PAYLOAD_VERSION,
            operation: "transfer_main_token",
            account_id: from_account_id.as_str(),
            public_key: public_key.as_str(),
            action: WasmTransferActionEnvelope::TransferMainToken(WasmTransferActionData {
                from_account_id: from_account_id.as_str(),
                to_account_id: "protocol:treasury",
                amount: 7,
                nonce: 9,
                asset_id: Some("main_token"),
                memo: None,
                chain_id: None,
                network_id: None,
                tx_version: Some(2),
                tx_type: Some("asset_transfer"),
                valid_until_unix_ms: None,
                max_fee: None,
                fee_asset_id: None,
                application_payload_hash: None,
                client_request_id: None,
            }),
        })
        .expect("wasm payload");

        assert_eq!(
            String::from_utf8(native_payload.clone()).expect("native utf8"),
            String::from_utf8(wasm_payload.clone()).expect("wasm utf8"),
        );

        let api_payload = build_main_token_transfer_signing_payload(
            &transfer,
            from_account_id.as_str(),
            public_key.as_str(),
        )
        .expect("client API canonical payload");
        assert_eq!(api_payload, native_payload);
        assert_eq!(api_payload, wasm_payload);

        let verified = verify_main_token_transfer_signature(&transfer, &proof)
            .expect("client API signed proof verifies");
        assert_eq!(verified.account_id, from_account_id);
        assert_eq!(verified.signer_public_keys, vec![public_key.clone()]);

        let signature = proof.signature.expect("signature");
        let signature_hex = signature
            .strip_prefix(MAIN_TOKEN_TRANSFER_AUTH_SIGNATURE_V2_PREFIX)
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
