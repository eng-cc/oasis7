//! One budget-permitted physical HTTPS send. No redirects, proxies, fallback or retry.
use crate::{
    connection::{Limits, RequestPermit},
    preset::ProviderPreset,
    protocol::{Conversation, ProviderFailure, ProviderResponse, decode_response, encode_request},
};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue};
use std::{
    io::Read,
    net::{IpAddr, SocketAddr, ToSocketAddrs},
    sync::mpsc,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;
#[derive(Default)]
pub struct ModelTransport;
impl ModelTransport {
    /// Caller must finish the permit on every return path, retaining unknown
    /// usage reservations. This method never marks a capability probe ready.
    pub fn send(
        &self,
        permit: &RequestPermit,
        conversation: &Conversation,
    ) -> Result<ProviderResponse, ProviderFailure> {
        self.send_with_timeout(permit, conversation, permit.preset().limits.timeout_ms)
    }
    pub fn send_with_timeout(
        &self,
        permit: &RequestPermit,
        conversation: &Conversation,
        timeout_ms: u64,
    ) -> Result<ProviderResponse, ProviderFailure> {
        if timeout_ms == 0 {
            return Err(ProviderFailure::InputLimit);
        }
        let preset = permit.preset();
        let body = validate_input(preset, permit.limits(), conversation)?;
        if permit.reserved_tokens()
            < conservative_tokens(body.len(), conversation.max_output_tokens)?
        {
            return Err(ProviderFailure::InputLimit);
        }
        let deadline =
            Instant::now() + Duration::from_millis(preset.limits.timeout_ms.min(timeout_ms));
        let addresses = resolve_public(preset, deadline)?;
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or(ProviderFailure::TransportFailed)?;
        let client = reqwest::blocking::Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(remaining)
            .connect_timeout(remaining.min(Duration::from_secs(10)))
            .resolve_to_addrs(preset.host(), &addresses)
            .build()
            .map_err(|_| ProviderFailure::TransportFailed)?;
        permit
            .before_send()
            .map_err(|_| ProviderFailure::SessionExpired)?;
        permit
            .with_key(|key| {
                if key.is_empty() || contains_secret_json(&body, key) {
                    return Err(ProviderFailure::InputLimit);
                }
                let mut authorization = Zeroizing::new(Vec::with_capacity(key.len() + 7));
                authorization.extend_from_slice(b"Bearer ");
                authorization.extend_from_slice(key);
                let mut header = HeaderValue::from_bytes(&authorization)
                    .map_err(|_| ProviderFailure::AuthenticationFailed)?;
                header.set_sensitive(true);
                let mut response = client
                    .post(format!("https://{}{}", preset.host(), preset.path()))
                    .header(AUTHORIZATION, header)
                    .header(CONTENT_TYPE, "application/json")
                    .body(body.to_vec())
                    .send()
                    .map_err(|_| ProviderFailure::TransportFailed)?;
                match response.status().as_u16() {
                    200 => {}
                    401 | 403 => return Err(ProviderFailure::AuthenticationFailed),
                    402 => return Err(ProviderFailure::QuotaUnavailable),
                    429 => return Err(ProviderFailure::RateLimited),
                    404 => return Err(ProviderFailure::ModelUnavailable),
                    _ => return Err(ProviderFailure::TransportFailed),
                }
                let content_type = response
                    .headers()
                    .get(CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or_default()
                    .to_owned();
                let mut bytes = Zeroizing::new(Vec::new());
                response
                    .by_ref()
                    .take(preset.limits.max_response_bytes.saturating_add(1) as u64)
                    .read_to_end(&mut bytes)
                    .map_err(|_| ProviderFailure::TransportFailed)?;
                if bytes.len() > preset.limits.max_response_bytes {
                    return Err(ProviderFailure::ResponseLimit);
                }
                if contains_secret_json(&bytes, key) {
                    return Err(ProviderFailure::InvalidResponse);
                }
                decode_response(preset, &bytes, &content_type, &conversation.tools)
            })
            .map_err(|_| ProviderFailure::SessionExpired)?
    }
}
/// Pure admission shared by preflight and transport. Retains the full history.
pub(crate) fn validate_input(
    preset: &ProviderPreset,
    limits: &Limits,
    conversation: &Conversation,
) -> Result<Zeroizing<Vec<u8>>, ProviderFailure> {
    let body = Zeroizing::new(encode_request(preset, conversation)?);
    let required = conservative_tokens(body.len(), conversation.max_output_tokens)?;
    if required
        > limits
            .max_input_tokens
            .saturating_add(u64::from(conversation.max_output_tokens))
        || u64::from(conversation.max_output_tokens) > limits.max_output_tokens
    {
        return Err(ProviderFailure::InputLimit);
    }
    Ok(body)
}
/// Each encoded UTF-8 byte is reserved as one token, plus a conservative fixed
/// envelope allowance and the full output ceiling. No price/cost claim follows.
pub fn conservative_tokens(
    input_bytes: usize,
    max_output_tokens: u32,
) -> Result<u64, ProviderFailure> {
    u64::try_from(input_bytes)
        .ok()
        .and_then(|n| n.checked_add(256))
        .and_then(|n| n.checked_add(u64::from(max_output_tokens)))
        .ok_or(ProviderFailure::InputLimit)
}
fn contains_secret_json(bytes: &[u8], key: &[u8]) -> bool {
    if key.is_empty() {
        return true;
    }
    if bytes.windows(key.len()).any(|v| v == key) {
        return true;
    }
    fn visit(v: &serde_json::Value, key: &[u8]) -> bool {
        match v {
            serde_json::Value::String(s) => s.as_bytes().windows(key.len()).any(|v| v == key),
            serde_json::Value::Array(a) => a.iter().any(|v| visit(v, key)),
            serde_json::Value::Object(m) => m
                .iter()
                .any(|(k, v)| k.as_bytes().windows(key.len()).any(|v| v == key) || visit(v, key)),
            _ => false,
        }
    }
    serde_json::from_slice(bytes)
        .ok()
        .is_some_and(|v| visit(&v, key))
}
fn resolve_public(
    preset: &ProviderPreset,
    deadline: Instant,
) -> Result<Vec<SocketAddr>, ProviderFailure> {
    let host = preset.host();
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("byok-dns".into())
        .spawn(move || {
            let result = (host, 443)
                .to_socket_addrs()
                .map(|v| v.take(65).collect::<Vec<_>>());
            let _ = tx.send(result);
        })
        .map_err(|_| ProviderFailure::TransportFailed)?;
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .ok_or(ProviderFailure::TransportFailed)?;
    let addresses = rx
        .recv_timeout(remaining.min(Duration::from_secs(5)))
        .map_err(|_| ProviderFailure::TransportFailed)?
        .map_err(|_| ProviderFailure::TransportFailed)?;
    if addresses.is_empty()
        || addresses.len() > 64
        || addresses
            .iter()
            .any(|v| v.port() != 443 || !is_public(v.ip()))
    {
        return Err(ProviderFailure::TransportFailed);
    }
    Ok(addresses)
}
fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            let [a, b, c, _] = v.octets();
            let reserved = matches!(a, 0 | 10 | 127 | 224..=255)
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192
                    && ((b == 0 && (c == 0 || c == 2)) || (b == 88 && c == 99) || b == 168))
                || (a == 198 && ((18..=19).contains(&b) || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113);
            !reserved
        }
        IpAddr::V6(v) => {
            let s = v.segments();
            (s[0] & 0xe000) == 0x2000
                && !(s[0] == 0x2001 && (s[1] <= 0x01ff || s[1] == 0x0db8))
                && s[0] != 0x2002
                && !(s[0] == 0x3fff && (s[1] & 0xf000) == 0)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_addresses_fail_closed_for_private_mapped_documentation_and_tunnels() {
        for address in [
            "127.0.0.1",
            "10.0.0.1",
            "169.254.169.254",
            "100.64.0.1",
            "192.0.2.1",
            "198.19.1.1",
            "224.1.1.1",
            "::1",
            "::ffff:8.8.8.8",
            "64:ff9b::a00:1",
            "2001:db8::1",
            "2002:7f00:1::",
            "fc00::1",
            "fe80::1",
        ] {
            assert!(!is_public(address.parse().unwrap()), "{address}");
        }
        for address in ["8.8.8.8", "1.1.1.1", "2606:4700:4700::1111"] {
            assert!(is_public(address.parse().unwrap()));
        }
    }
    #[test]
    fn key_canary_is_blocked_in_encoded_fields_and_names() {
        for body in [
            br#"{"value":"secret-canary"}"#.as_slice(),
            br#"{"value":"secret-\u0063anary"}"#,
            br#"{"secret-canary":"x"}"#,
        ] {
            assert!(contains_secret_json(body, b"secret-canary"));
        }
        assert!(!contains_secret_json(br#"{"ok":true}"#, b"secret-canary"));
    }
}
