use super::*;
use oasis7_model_provider::{
    connection::Limits,
    preset::{Protocol, ProviderId, ProviderPreset, RequestLimits},
};
use std::sync::atomic::{AtomicBool, Ordering};

const SECRET: &str = "fixture-key-never-returned";
fn principal(account: &str, session: &str) -> Principal {
    Principal {
        account_id: account.into(),
        session_id: session.into(),
        auth_ref: format!("auth:{session}"),
    }
}
// Construct directly: production new() changes process-wide core-dump limits.
fn fixture() -> (HostedModelClient, Arc<AtomicBool>) {
    let current = Arc::new(AtomicBool::new(true));
    let authority = Arc::clone(&current);
    let limits = Limits::default();
    let manager = ConnectionManager::new(
        Arc::new(move |_: &Principal| authority.load(Ordering::SeqCst)),
        Arc::new(|_: &Principal, _: &BindingTarget| false),
        Arc::new(MonotonicClock::default()),
        limits.clone(),
    )
    .unwrap();
    (
        HostedModelClient {
            config: Config {
                enabled: true,
                public_origin: "https://localhost".into(),
                display_name: "Test executor".into(),
                location: "Local test".into(),
                memory_only_controls_confirmed: true,
                limits,
                executors: vec![],
                presets: vec![ProviderPreset {
                    id: "test".into(),
                    provider: ProviderId::DeepSeek,
                    protocol: Protocol::ChatCompletions,
                    model: "deepseek-chat".into(),
                    routes: vec![],
                    revision: 1,
                    limits: RequestLimits::default(),
                }],
            },
            manager,
            executors: Arc::new(
                executor::Executors::load(&[], Arc::new(Mutex::new(None))).unwrap(),
            ),
            executions: execution::Executions::default(),
        },
        current,
    )
}
fn request(
    host: &HostedModelClient,
    p: &Principal,
    method: &str,
    suffix: &str,
    body: Value,
) -> IdentityReply {
    let reply = host.handle(p, method, &format!("{PREFIX}{suffix}"), body);
    assert!(
        !reply.body.to_string().contains(SECRET),
        "credential appeared in response"
    );
    assert!(reply.cookies.is_empty());
    reply
}
fn create(host: &HostedModelClient, p: &Principal) -> Value {
    let reply = request(
        host,
        p,
        "POST",
        "/model-connections",
        json!({"preset_id":"test","raw_key":SECRET,"consent":true}),
    );
    assert_eq!(reply.status, 200);
    reply.body["connection"].clone()
}
#[test]
fn disabled_model_receiver_does_not_advertise_a_credential_form() {
    let reply = disabled("not_configured");
    assert_eq!(reply.status, 200);
    assert_eq!(reply.body["enabled"], false);
    assert_eq!(reply.body["authenticated"], false);
    assert_eq!(reply.body["executor_available"], false);
    assert_eq!(reply.body["presets"], json!([]));
    assert!(reply.body.get("trusted_executor").is_none());
}
#[test]
fn revoked_authority_cannot_store_a_key_and_create_rejects_transport_overrides() {
    let (host, current) = fixture();
    let p = principal("a", "one");
    current.store(false, Ordering::SeqCst);
    assert_eq!(
        request(
            &host,
            &p,
            "POST",
            "/model-connections",
            json!({"preset_id":"test","raw_key":SECRET,"consent":true})
        )
        .status,
        404
    );
    current.store(true, Ordering::SeqCst);
    for field in ["owner", "base_url", "baseURL", "headers"] {
        let mut body = json!({"preset_id":"test","raw_key":SECRET,"consent":true});
        body[field] = json!({"injected":SECRET});
        assert_eq!(
            request(&host, &p, "POST", "/model-connections", body).status,
            400
        );
    }
    let list = request(&host, &p, "GET", "/model-connections", Value::Null);
    assert_eq!(list.body["connections"], json!([]));
}
#[test]
fn connection_ownership_is_opaque_across_account_and_session() {
    let (host, _) = fixture();
    let p = principal("a", "one");
    let c = create(&host, &p);
    let id = c["id"].as_str().unwrap();
    for other in [principal("b", "two"), principal("a", "two")] {
        // Probe ownership fails before the provider transport can be reached.
        for (method, action) in [
            ("GET", ""),
            ("POST", "/probe"),
            ("POST", "/activate"),
            ("POST", "/stop"),
            ("DELETE", ""),
        ] {
            let reply = request(
                &host,
                &other,
                method,
                &format!("/model-connections/{id}{action}"),
                json!({}),
            );
            assert_eq!(reply.status, 404);
            assert_eq!(reply.body["error_code"], "not_found");
        }
    }
}
#[test]
fn executor_unavailable_and_stop_cas_replay_are_distinct_from_forget() {
    let (host, _) = fixture();
    let p = principal("a", "one");
    assert_eq!(host.capabilities()["executor_available"], false);
    let c = create(&host, &p);
    let path = format!("/model-connections/{}", c["id"].as_str().unwrap());
    let v = c["version"].as_u64().unwrap();
    assert_eq!(
        request(
            &host,
            &p,
            "POST",
            &format!("{path}/activate"),
            json!({"expected_version":v,"consent":true})
        )
        .status,
        409
    );
    let stop = |version| json!({"expected_version":version,"operation_id":"stop-operation-one"});
    assert_eq!(
        request(&host, &p, "POST", &format!("{path}/stop"), stop(v + 1)).body["error_code"],
        "version_conflict"
    );
    let stopped = request(&host, &p, "POST", &format!("{path}/stop"), stop(v));
    assert_eq!(stopped.status, 200);
    let replay = request(&host, &p, "POST", &format!("{path}/stop"), stop(v));
    assert!(
        stopped.body == replay.body,
        "same operation must replay its existing projection"
    );
    assert_eq!(stopped.body["connection"]["run_state"], "stopped");
    assert_eq!(
        request(&host, &p, "DELETE", &path, json!({"expected_version":v})).body["error_code"],
        "version_conflict"
    );
    let version = stopped.body["connection"]["version"].as_u64().unwrap();
    let forgotten = request(
        &host,
        &p,
        "DELETE",
        &path,
        json!({"expected_version":version}),
    );
    assert_eq!(forgotten.status, 200);
    assert_eq!(forgotten.body["connection"]["state"], "revoked");
    assert_eq!(forgotten.body["connection"]["calls_spent"], 0);
}

#[test]
fn no_deployment_config_does_not_construct_a_key_receiver() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .env_clear()
        .args([
            "--ignored",
            "--exact",
            "hosted_model_client::tests::unconfigured_receiver_child",
            "--nocapture",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "isolated default-off configuration failed"
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "child filter must execute its assertion"
    );
}
#[test]
#[ignore = "executed by no_deployment_config_does_not_construct_a_key_receiver in a clean environment"]
fn unconfigured_receiver_child() {
    assert!(Config::from_env().unwrap().is_none());
    let identity = Arc::new(Mutex::new(None));
    assert!(HostedModelClient::from_env(&identity).unwrap().is_none());
}

#[test]
fn capability_presets_use_the_web_provider_contract_without_acronym_splitting() {
    let (host, _) = fixture();
    let capabilities = host.capabilities();
    assert_eq!(capabilities["presets"][0]["provider_id"], "deepseek");
    assert_eq!(
        serde_json::to_value(oasis7_model_provider::preset::ProviderId::OpenRouter).unwrap(),
        "openrouter"
    );
}

#[test]
fn operator_device_configuration_is_optional_but_requires_absolute_private_device_reference() {
    let (mut host, _) = fixture();
    assert!(host.config.validate().is_ok());
    host.config.executors.push(config::ExecutorConfig {
        account_id: "a".into(),
        device_config_path: "relative-device.json".into(),
    });
    assert!(host.config.validate().is_err());
    host.config.executors[0].device_config_path =
        std::env::temp_dir().join("missing-byok-sdk-device.json");
    assert!(host.config.validate().is_ok());
    // A browser key/account label cannot bootstrap a missing SDK signer/config.
    assert!(executor::Executors::load(&host.config.executors, Arc::new(Mutex::new(None))).is_err());
}
#[test]
fn activate_accepts_only_target_reference_and_explicit_displayed_limits() {
    let base = json!({"expected_version":1,"consent":true,"target_ref":{"agent_id":"agent"},"limits":Limits::default()});
    assert!(serde_json::from_value::<execution::Activate>(base.clone()).is_ok());
    for field in [
        "account_id",
        "owner_binding",
        "executor_id",
        "device_config_path",
        "world_id",
        "raw_key",
    ] {
        let mut body = base.clone();
        body["target_ref"][field] = json!("injected");
        assert!(
            serde_json::from_value::<execution::Activate>(body).is_err(),
            "{field}"
        );
    }
}

#[test]
fn explicit_stop_hides_finished_failure_and_late_old_generation() {
    let (host, _) = fixture();
    let p = principal("a", "one");
    let connection = create(&host, &p);
    let id = connection["id"].as_str().unwrap();
    let generation = connection["generation"].as_u64().unwrap();
    let stale = json!({"phase":"finished","reason":"model_failed","model_failure":"output_limit","generation":generation});
    host.executions.set(id, stale.clone());
    let path = format!("/model-connections/{id}");
    let stopped = request(
        &host,
        &p,
        "POST",
        &format!("{path}/stop"),
        json!({"expected_version":connection["version"],"operation_id":"stop-after-failure"}),
    );
    assert_eq!(stopped.status, 200);
    assert_eq!(stopped.body["connection"]["execution"]["phase"], "stopped");
    assert!(stopped.body["connection"]["execution"]["model_failure"].is_null());
    host.executions.set(id, stale);
    let read = request(&host, &p, "GET", &path, json!({}));
    assert_eq!(read.body["connection"]["execution"]["phase"], "stopped");
    let later = host
        .manager
        .stop(
            &p,
            id,
            stopped.body["connection"]["version"].as_u64().unwrap(),
        )
        .unwrap();
    let finished = json!({"phase":"finished","reason":"model_failed","model_failure":"output_limit","generation":later.generation});
    host.executions.set(id, finished.clone());
    let replay = request(
        &host,
        &p,
        "POST",
        &format!("{path}/stop"),
        json!({"expected_version":connection["version"],"operation_id":"stop-after-failure"}),
    );
    assert_eq!(replay.status, 200);
    assert_eq!(replay.body["connection"]["execution"], finished);
}
