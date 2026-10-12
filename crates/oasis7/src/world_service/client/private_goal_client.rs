//! Confidential goal transport uses its own no-proxy/no-redirect client.
//! Plaintext loopback requires all local listening processes to be trusted: the
//! service response signature does not authenticate the peer before body delivery.
//! Use authenticated TLS transport for hosts with untrusted local users.
use super::*;
use crate::world_service::{
    private_goal_prepare::SignedGoalPrepareV1, private_goal_read::*,
    private_goal_store::PreparedGoalReceiptV1,
};
impl RemoteWorldServiceClient {
    fn private_connection(&self) -> Result<reqwest::blocking::Client, WorldServiceClientError> {
        let endpoint = reqwest::Url::parse(&self.config.endpoint).map_err(|_| {
            WorldServiceClientError::Configuration("invalid private endpoint".into())
        })?;
        let host = endpoint
            .host_str()
            .unwrap_or_default()
            .trim_start_matches('[')
            .trim_end_matches(']');
        if endpoint.scheme() != "https"
            && !(endpoint.scheme() == "http"
                && host
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback()))
        {
            return Err(WorldServiceClientError::Configuration(
                "private goals require HTTPS or a literal loopback HTTP address".into(),
            ));
        }
        if self.config.trusted_service_public_key.len() != 64
            || !self
                .config
                .trusted_service_public_key
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(WorldServiceClientError::Configuration(
                "noncanonical private service trust".into(),
            ));
        }
        let key: [u8; 32] = hex::decode(&self.config.trusted_service_public_key)
            .map_err(|_| {
                WorldServiceClientError::Configuration("invalid private service trust".into())
            })?
            .try_into()
            .map_err(|_| {
                WorldServiceClientError::Configuration("invalid private service trust".into())
            })?;
        if ed25519_dalek::VerifyingKey::from_bytes(&key)
            .map_err(|_| {
                WorldServiceClientError::Configuration("invalid private service trust".into())
            })?
            .is_weak()
        {
            return Err(WorldServiceClientError::Configuration(
                "weak private service trust".into(),
            ));
        }
        reqwest::blocking::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(self.config.timeout)
            .build()
            .map_err(|_| {
                WorldServiceClientError::Configuration("private transport unavailable".into())
            })
    }
    pub(super) fn private_scope(
        &self,
        world: &WorldIdentity,
        agent: &str,
    ) -> Result<(), WorldServiceClientError> {
        if world != &self.config.expected_world || self.config.scope_id != format!("agent:{agent}")
        {
            return Err(WorldServiceClientError::Configuration(
                "private goal world or agent differs from configuration".into(),
            ));
        }
        Ok(())
    }
    fn private_call<Q: Serialize, R: Serialize + DeserializeOwned>(
        &self,
        path: &str,
        request: &Q,
    ) -> Result<SignedServiceResponse<R>, WorldServiceClientError> {
        self.private_call_bounded(path, request, 64 * 1024)
    }
    pub(super) fn private_call_bounded<Q: Serialize, R: Serialize + DeserializeOwned>(
        &self,
        path: &str,
        request: &Q,
        maximum: usize,
    ) -> Result<SignedServiceResponse<R>, WorldServiceClientError> {
        let http = self.private_connection()?;
        self.query_state.check()?;
        let mut response = http
            .post(format!(
                "{}{}",
                self.config.endpoint.trim_end_matches('/'),
                path
            ))
            .json(request)
            .send()
            .map_err(|_| WorldServiceClientError::Transport("private transport failed".into()))?;
        let status = response.status().as_u16();
        if status != 200 {
            return Err(WorldServiceClientError::Rejected {
                status,
                message: "private goal request rejected".into(),
            });
        }
        let bound = self
            .config
            .max_response_bytes
            .min(maximum.min(2 * 1024 * 1024));
        let mut bytes = zeroize::Zeroizing::new(Vec::new());
        response
            .by_ref()
            .take(bound as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| {
                WorldServiceClientError::Transport("private response read failed".into())
            })?;
        if bytes.len() > bound {
            return Err(WorldServiceClientError::Assurance(
                "private response exceeds bound".into(),
            ));
        }
        // Reject duplicate keys/depth first, then ignored wrapper/commit fields.
        let value: Value = oasis7_agent_api::game::parse_bounded_json(&bytes, bound)
            .map_err(|_| WorldServiceClientError::Assurance("invalid private response".into()))?;
        use serde::de::IntoDeserializer;
        let mut ignored = false;
        let signed = serde_ignored::deserialize(value.into_deserializer(), |_| ignored = true)
            .map_err(|_| WorldServiceClientError::Assurance("invalid private response".into()))?;
        if ignored {
            return Err(WorldServiceClientError::Assurance(
                "unsupported private response field".into(),
            ));
        }
        Ok(signed)
    }
    /// Retain and verify the exact original proof. Does not sign or submit a public intent.
    pub fn prepare_goal(
        &self,
        original: &SignedGoalPrepareV1,
    ) -> Result<PreparedGoalReceiptV1, WorldServiceClientError> {
        original
            .verify()
            .map_err(|_| WorldServiceClientError::Assurance("invalid private goal proof".into()))?;
        self.private_scope(
            &WorldIdentity {
                world_id: original.request.world_id.clone(),
                genesis_digest: original.request.genesis_digest.clone(),
            },
            &original.request.agent_id,
        )?;
        let response = self.private_call(GOAL_PREPARE_PATH, original)?;
        verify_prepare_response(original, &response, &self.config.trusted_service_public_key)
            .map_err(|_| {
                WorldServiceClientError::Assurance("private prepare response not verified".into())
            })?;
        Ok(response.payload)
    }
    /// Owner-authorized full goal subset. This does not provide full TaskSnapshot/Context.
    pub fn read_current_goal(
        &self,
        request: &SignedReadRequest<CurrentGoalRequest>,
    ) -> Result<CurrentGoalResponse, WorldServiceClientError> {
        verify_current_request(request)
            .map_err(|_| WorldServiceClientError::Assurance("invalid current goal proof".into()))?;
        self.private_scope(&request.request.world, &request.request.agent_id)?;
        let response = self.private_call(GOAL_CURRENT_PATH, request)?;
        verify_current_response(request, &response, &self.config.trusted_service_public_key)
            .map_err(|_| {
                WorldServiceClientError::Assurance(
                    "current private goal response not verified".into(),
                )
            })?;
        Ok(response.payload)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_transport_rejects_remote_plaintext_and_names_before_network() {
        let private = hex::encode([7; 32]);
        let key = authority::sign_read_request("fixture", (), &private)
            .unwrap()
            .subject_public_key;
        for endpoint in [
            "http://192.0.2.1",
            "http://localhost",
            "http://127.0.0.1.evil.invalid",
        ] {
            let client = RemoteWorldServiceClient::new(WorldServiceClientConfig {
                endpoint: endpoint.into(),
                trusted_service_public_key: key.clone(),
                expected_world: WorldIdentity {
                    world_id: "w".into(),
                    genesis_digest: "g".into(),
                },
                scope_id: "agent:a".into(),
                read_private_key_hex: private.clone(),
                timeout: Duration::from_millis(50),
                max_response_bytes: 65536,
            })
            .unwrap();
            assert!(matches!(
                client.private_connection(),
                Err(WorldServiceClientError::Configuration(_))
            ));
        }
    }
    fn config(endpoint: String, key: String, bound: usize) -> WorldServiceClientConfig {
        WorldServiceClientConfig {
            endpoint,
            trusted_service_public_key: key,
            expected_world: WorldIdentity {
                world_id: "w".into(),
                genesis_digest: "g".into(),
            },
            scope_id: "agent:a".into(),
            read_private_key_hex: hex::encode([7; 32]),
            timeout: Duration::from_secs(5),
            max_response_bytes: bound,
        }
    }
    #[test]
    fn private_trust_canonical_and_weak_keys_rejected_before_sending() {
        let key = authority::sign_read_request("fixture", (), &hex::encode([7; 32]))
            .unwrap()
            .subject_public_key;
        for bad in [key.to_uppercase(), hex::encode([0; 32])] {
            let client =
                RemoteWorldServiceClient::new(config("http://127.0.0.1:1".into(), bad, 65536))
                    .unwrap();
            assert!(matches!(
                client.private_connection(),
                Err(WorldServiceClientError::Configuration(_))
            ));
        }
    }
    #[test]
    fn private_response_size_cap_even_with_usize_max_configuration() {
        use std::io::Write;
        let key = authority::sign_read_request("fixture", (), &hex::encode([7; 32]))
            .unwrap()
            .subject_public_key;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let client = RemoteWorldServiceClient::new(config(
            format!("http://{}", listener.local_addr().unwrap()),
            key,
            usize::MAX,
        ))
        .unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut b = [0; 1];
                stream.read_exact(&mut b).unwrap();
                request.push(b[0]);
            }
            let header = std::str::from_utf8(&request).unwrap();
            let length = header
                .split("\r\n")
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .unwrap()
                .1
                .trim()
                .parse::<usize>()
                .unwrap();
            let mut body = vec![0; length];
            stream.read_exact(&mut body).unwrap();
            let body = vec![b'x'; 65537];
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        });
        let result = client.private_call::<_, Value>(GOAL_PREPARE_PATH, &());
        assert!(
            matches!(result,Err(WorldServiceClientError::Assurance(ref reason)) if reason=="private response exceeds bound")
        );
        server.join().unwrap();
    }
}
