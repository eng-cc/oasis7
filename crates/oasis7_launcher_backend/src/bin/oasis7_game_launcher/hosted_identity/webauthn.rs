//! Version-pinned WebAuthn boundary. Ceremony state is private server data.
//! Cryptographic parsing, RP/origin, UP/UV and counters are checked by webauthn-rs.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::Value;
use std::time::Duration;
use url::Url;
use uuid::Uuid;
use webauthn_rs_core::{WebauthnCore, proto::*};

#[derive(Clone)]
pub struct WebAuthnAdapter {
    core: WebauthnCore,
}

type Result<T> = std::result::Result<T, String>;

impl WebAuthnAdapter {
    pub fn new(rp_id: &str, origins: &[String]) -> Result<Self> {
        if rp_id.is_empty() || origins.is_empty() {
            return Err("webauthn_configuration_invalid".into());
        }
        let parsed = origins
            .iter()
            .map(|origin| {
                let url = Url::parse(origin).map_err(|_| "webauthn_configuration_invalid")?;
                let host = url.host_str().ok_or("webauthn_configuration_invalid")?;
                if url.scheme() != "https"
                    || !url.username().is_empty()
                    || url.password().is_some()
                    || url.query().is_some()
                    || url.fragment().is_some()
                    || url.path() != "/"
                    || !(host == rp_id || host.ends_with(&format!(".{rp_id}")))
                {
                    return Err("webauthn_configuration_invalid");
                }
                Ok(url)
            })
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(str::to_owned)?;
        Ok(Self {
            core: WebauthnCore::new_unsafe_experts_only(
                "Oasis7",
                rp_id,
                parsed,
                Duration::from_secs(120),
                Some(false),
                Some(false),
            ),
        })
    }

    pub fn registration_options(
        &self,
        user_handle: &str,
        display_name: &str,
        existing: &[Value],
    ) -> Result<(Value, Value)> {
        let user = Uuid::parse_str(user_handle).map_err(|_| "invalid_user_handle")?;
        let credentials = existing
            .iter()
            .map(decode::<Credential>)
            .collect::<Result<Vec<_>>>()?;
        // Use core's paired builder rather than editing only browser JSON. High-level
        // passkey registration requests resident credentials only opportunistically.
        let builder = self
            .core
            .new_challenge_register_builder(user.as_bytes(), display_name, display_name)
            .map_err(|_| "registration_options_failed")?
            .require_resident_key(true)
            .user_verification_policy(UserVerificationPolicy::Required)
            .attestation(AttestationConveyancePreference::None)
            .authenticator_attachment(None)
            .credential_algorithms(vec![COSEAlgorithm::ES256, COSEAlgorithm::RS256])
            .exclude_credentials(Some(credentials.into_iter().map(|c| c.cred_id).collect()));
        let (options, state) = self
            .core
            .generate_challenge_register(builder)
            .map_err(|_| "registration_options_failed")?;
        Ok((encode(options)?, encode(state)?))
    }

    pub fn finish_registration(&self, response: &Value, state: &Value) -> Result<Value> {
        reject_embedded_client(response)?;
        let response = decode::<RegisterPublicKeyCredential>(response)?;
        let state = decode::<RegistrationState>(state)?;
        let credential = self
            .core
            .register_credential(&response, &state, None)
            .map_err(|_| "authentication_failed")?;
        if credential.cred_id.as_ref() != response.raw_id.as_ref() {
            return Err("authentication_failed".into());
        }
        encode(credential)
    }

    pub fn authentication_options(&self, credentials: &[Value]) -> Result<(Value, Value)> {
        let credentials = credentials
            .iter()
            .map(decode::<Credential>)
            .collect::<Result<Vec<_>>>()?;
        let builder = self
            .core
            .new_challenge_authenticate_builder(credentials, Some(UserVerificationPolicy::Required))
            .map_err(|_| "authentication_options_failed")?;
        let (options, state) = self
            .core
            .generate_challenge_authenticate(builder)
            .map_err(|_| "authentication_options_failed")?;
        Ok((encode(options)?, encode(state)?))
    }

    /// Identification only: callers must still check ownership and finish verification.
    pub fn identify_authentication(&self, response: &Value) -> Result<(String, String)> {
        reject_embedded_client(response)?;
        let credential = decode::<PublicKeyCredential>(response)?;
        let handle = credential
            .response
            .user_handle
            .as_ref()
            .ok_or("authentication_failed")?;
        let user = Uuid::from_slice(handle.as_ref()).map_err(|_| "authentication_failed")?;
        Ok((
            user.to_string(),
            URL_SAFE_NO_PAD.encode(credential.raw_id.as_ref()),
        ))
    }

    pub fn credential_id(&self, credential: &Value) -> Result<String> {
        let credential = decode::<Credential>(credential)?;
        Ok(URL_SAFE_NO_PAD.encode(credential.cred_id.as_ref()))
    }

    pub fn finish_authentication(
        &self,
        response: &Value,
        state: &Value,
        stored_credential: &Value,
    ) -> Result<Value> {
        reject_embedded_client(response)?;
        let response = decode::<PublicKeyCredential>(response)?;
        let mut state = decode::<AuthenticationState>(state)?;
        let mut credential = decode::<Credential>(stored_credential)?;
        // Discoverable ceremonies have no credential list until the verified store
        // lookup. Scoped reauthentication must not widen its original allowlist.
        let state_json = encode(&state)?;
        let allowed = state_json
            .get("credentials")
            .and_then(Value::as_array)
            .ok_or("invalid_webauthn_data")?;
        if !allowed.is_empty()
            && !allowed.iter().any(|item| {
                decode::<Credential>(item).is_ok_and(|c| c.cred_id == credential.cred_id)
            })
        {
            return Err("authentication_failed".into());
        }
        // Always use the current authoritative credential counter, including scoped
        // ceremonies whose options were issued before another successful assertion.
        state.set_allowed_credentials(vec![credential.clone()]);
        let result = self
            .core
            .authenticate_credential(&response, &state)
            .map_err(|_| "authentication_failed")?;
        if result.cred_id() != &credential.cred_id || !result.user_verified() {
            return Err("authentication_failed".into());
        }
        credential.counter = result.counter();
        credential.backup_state = result.backup_state();
        credential.backup_eligible = result.backup_eligible();
        encode(credential)
    }
}

fn decode<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T> {
    serde_json::from_value(value.clone()).map_err(|_| "invalid_webauthn_data".into())
}
fn encode<T: serde::Serialize>(value: T) -> Result<Value> {
    serde_json::to_value(value).map_err(|_| "webauthn_state_failed".into())
}

fn reject_embedded_client(response: &Value) -> Result<()> {
    let id = response
        .get("id")
        .and_then(Value::as_str)
        .ok_or("invalid_webauthn_data")?;
    let raw_id = response
        .get("rawId")
        .and_then(Value::as_str)
        .ok_or("invalid_webauthn_data")?;
    if id.len() > 2048
        || raw_id.len() > 2048
        || response.get("type").and_then(Value::as_str) != Some("public-key")
    {
        return Err("invalid_webauthn_data".into());
    }
    let id_bytes = URL_SAFE_NO_PAD
        .decode(id)
        .map_err(|_| "invalid_webauthn_data")?;
    let raw_bytes = URL_SAFE_NO_PAD
        .decode(raw_id)
        .map_err(|_| "invalid_webauthn_data")?;
    if id_bytes.is_empty() || id_bytes != raw_bytes {
        return Err("invalid_webauthn_data".into());
    }
    let encoded = response
        .pointer("/response/clientDataJSON")
        .and_then(Value::as_str)
        .ok_or("invalid_webauthn_data")?;
    if encoded.len() > 16_384 {
        return Err("invalid_webauthn_data".into());
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| "invalid_webauthn_data")?;
    let client: Value = serde_json::from_slice(&bytes).map_err(|_| "invalid_webauthn_data")?;
    if client
        .get("crossOrigin")
        .is_some_and(|v| v != &Value::Bool(false))
        || client.get("topOrigin").is_some()
    {
        return Err("authentication_failed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn adapter() -> WebAuthnAdapter {
        WebAuthnAdapter::new(
            "bootstrap1.oasis7.tech",
            &["https://bootstrap1.oasis7.tech".into()],
        )
        .unwrap()
    }
    #[test]
    fn registration_requires_resident_uv_and_private_paired_state() {
        let (options, state) = adapter()
            .registration_options("19a66cd1-2195-4cd6-8b61-75bf8623e3cf", "玩家 7K2M", &[])
            .unwrap();
        assert_eq!(
            options.pointer("/publicKey/authenticatorSelection/residentKey"),
            Some(&Value::String("required".into()))
        );
        assert_eq!(
            options.pointer("/publicKey/authenticatorSelection/userVerification"),
            Some(&Value::String("required".into()))
        );
        assert_eq!(state["require_resident_key"], true);
        assert_eq!(state["policy"], "required");
        assert!(
            options
                .pointer("/publicKey/authenticatorSelection/authenticatorAttachment")
                .is_none()
        );
    }
    #[test]
    fn discoverable_has_empty_allowlist_and_requires_uv() {
        let (options, _) = adapter().authentication_options(&[]).unwrap();
        assert_eq!(
            options.pointer("/publicKey/allowCredentials"),
            Some(&serde_json::json!([]))
        );
        assert_eq!(
            options.pointer("/publicKey/userVerification"),
            Some(&Value::String("required".into()))
        );
    }
    #[test]
    fn private_state_roundtrips_and_malformed_registration_cannot_create_key() {
        let adapter = adapter();
        let (_, state) = adapter
            .registration_options("19a66cd1-2195-4cd6-8b61-75bf8623e3cf", "玩家 7K2M", &[])
            .unwrap();
        let restored: RegistrationState = decode(&state).unwrap();
        assert_eq!(encode(restored).unwrap(), state);
        let client = serde_json::json!({"type":"webauthn.create", "challenge":"wrong", "origin":"https://bootstrap1.oasis7.tech"});
        let response = serde_json::json!({"id":"AA", "rawId":"AA", "type":"public-key", "response":{
            "clientDataJSON":URL_SAFE_NO_PAD.encode(serde_json::to_vec(&client).unwrap()),
            "attestationObject":"AA"}, "extensions":{}});
        assert!(adapter.finish_registration(&response, &state).is_err());
    }
    #[test]
    fn rejects_untrusted_origin_configuration_and_embedded_clients() {
        assert!(WebAuthnAdapter::new("oasis7.tech", &["https://evil.example".into()]).is_err());
        assert!(WebAuthnAdapter::new("oasis7.tech", &["http://oasis7.tech".into()]).is_err());
        for client in [
            serde_json::json!({"crossOrigin":true}),
            serde_json::json!({"topOrigin":"https://evil.example"}),
        ] {
            let response = serde_json::json!({"id":"AQ","rawId":"AQ","type":"public-key","response":{"clientDataJSON":URL_SAFE_NO_PAD.encode(serde_json::to_vec(&client).unwrap())}});
            assert_eq!(
                reject_embedded_client(&response),
                Err("authentication_failed".into())
            );
        }
    }
}
