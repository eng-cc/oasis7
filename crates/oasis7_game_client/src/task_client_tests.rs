use super::*;
use oasis7_agent_api::game::{DecimalU64, GAME_TASK_CREATE_DOMAIN, GameTaskCreateV1};
use std::{
    io::{Read, Write},
    net::TcpListener,
};
fn body(stream: &mut std::net::TcpStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .unwrap();
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        head.push(byte[0]);
    }
    let head = String::from_utf8(head).unwrap();
    let length: usize = head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().unwrap())
        })
        .unwrap();
    let mut body = vec![0; length];
    stream.read_exact(&mut body).unwrap();
    body
}
#[test]
fn lost_task_submit_is_unknown_and_only_explicit_lookup_recovers_same_signed_request() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = WorldServiceTaskClient::new(crate::tests::config(format!(
        "http://{}",
        listener.local_addr().unwrap()
    )))
    .unwrap();
    let payload = WorldServicePayloadV1::GameTaskCreate(
        sign_read_request(
            GAME_TASK_CREATE_DOMAIN,
            GameTaskCreateV1 {
                api_rev: 1,
                client_operation_id: "create".into(),
                world_id: "world".into(),
                genesis_digest: "genesis".into(),
                branch_id: "main".into(),
                reorg_epoch: DecimalU64(0),
                agent_id: "a".into(),
                owner_binding: "owner".into(),
                owner_generation: DecimalU64(1),
            },
            &hex::encode([7; 32]),
        )
        .unwrap(),
    );
    let correlation = derive_correlation(client.config().expected_world.clone(), &payload).unwrap();
    let expected_payload = payload.clone();
    let expected_correlation = correlation.clone();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let submitted: SubmitIntentRequest<WorldServicePayloadV1> =
            serde_json::from_slice(&body(&mut stream)).unwrap();
        assert_eq!(submitted.signed_payload, expected_payload);
        assert_eq!(submitted.correlation, expected_correlation);
        drop(stream);
        let (mut stream, _) = listener.accept().unwrap();
        let lookup: AuthenticatedLookup = serde_json::from_slice(&body(&mut stream)).unwrap();
        assert_eq!(lookup.original, expected_payload);
        assert_eq!(lookup.request.key, expected_correlation.key);
        let digest = request_digest(LOOKUP_PATH, &lookup).unwrap();
        let response = sign_service_response(
            LOOKUP_PATH,
            digest,
            IntentResponse::<Value> {
                formal_profile: None,
                execution_evidence_scope: None,
                contract_version: 1,
                correlation: expected_correlation,
                outcome: IntentOutcome::Pending,
            },
            &hex::encode([9; 32]),
        )
        .unwrap();
        let bytes = serde_json::to_vec(&response).unwrap();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            bytes.len()
        )
        .unwrap();
        stream.write_all(&bytes).unwrap();
        listener.set_nonblocking(true).unwrap();
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    });
    let response = client
        .submit(SubmitIntentRequest {
            contract_version: 1,
            correlation: correlation.clone(),
            deadline_unix_ms: None,
            signed_payload: payload.clone(),
        })
        .unwrap();
    assert!(
        matches!(response, SubmitObservation::OutcomeUnknown(ref value) if value == &correlation)
    );
    assert!(matches!(
        client
            .lookup(
                LookupIntentRequest {
                    contract_version: 1,
                    key: correlation.key
                },
                payload
            )
            .unwrap()
            .outcome,
        IntentOutcome::Pending
    ));
    server.join().unwrap();
}
