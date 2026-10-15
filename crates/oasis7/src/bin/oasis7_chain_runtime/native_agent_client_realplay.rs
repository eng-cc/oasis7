//! Explicit separate-process production executor probe. No World access or fake provider.
//! The worker runs in this probe process, not inside the fixture node. This tests
//! the production executor code, not node-owned startup or launcher/GUI acceptance.
use super::*;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn prepared() -> PreparedExecutor {
    assert_eq!(
        std::env::var("OASIS7_AGENT_EXECUTION_PROFILE").as_deref(),
        Ok(PROFILE)
    );
    let budget: u32 = std::env::var("OASIS7_AGENT_MAX_MODEL_CALLS")
        .expect("explicit budget required")
        .parse()
        .expect("invalid budget");
    assert!(
        (1..=20).contains(&budget),
        "realplay budget must be within authorized 20 physical calls"
    );
    let path =
        std::env::var("OASIS7_AGENT_DEVICE_CONFIG").expect("absolute device config required");
    assert!(Path::new(&path).is_absolute());
    let client =
        Client::open(Path::new(&path)).unwrap_or_else(|_| panic!("device preflight failed"));
    let remote = client.remote.config();
    let endpoint = reqwest::Url::parse(&remote.endpoint).expect("invalid endpoint");
    let port = endpoint
        .port_or_known_default()
        .expect("endpoint port required");
    PreparedExecutor::from_environment(
        true,
        port,
        &client.world.world_id,
        &remote.trusted_service_public_key,
    )
    .unwrap_or_else(|_| panic!("production executor preflight failed"))
    .expect("native profile required")
}

#[test]
#[ignore = "operator-only existing-config preflight; never starts executor or calls model"]
fn native_existing_world_production_executor_preflight() {
    drop(prepared());
    eprintln!(
        "NATIVE_REALPLAY_PREFLIGHT profile=native_task_industry_v1 credentials=present executor=production inference_started=false"
    );
}

#[test]
#[ignore = "operator explicitly starts actual paid production executor in separate bounded process"]
fn native_existing_world_production_executor_process() {
    assert_eq!(
        std::env::var("OASIS7_NATIVE_REALPLAY_START").as_deref(),
        Ok("explicitly-authorized"),
        "explicit start required"
    );
    // Preserve this absolute deadline, run ID and device configuration on restart.
    // The SDK encrypted reservations count crashed/unknown physical calls as spent.
    let deadline_ms: u128 = std::env::var("OASIS7_NATIVE_REALPLAY_DEADLINE_UNIX_MS")
        .expect("explicit original absolute deadline required")
        .parse()
        .expect("invalid deadline");
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    assert!(
        deadline_ms > now_ms && deadline_ms - now_ms <= 900_000,
        "deadline must be future and within authorized 15 minutes"
    );
    let deadline =
        Instant::now() + Duration::from_millis(u64::try_from(deadline_ms - now_ms).unwrap());
    eprintln!(
        "NATIVE_REALPLAY_PROBE actual_location=separate_probe_process node_owned_startup_verified=false"
    );
    prepared()
        .start()
        .expect("production executor start failed");
    let mut previous = (String::new(), String::new());
    loop {
        let status = snapshot();
        let phase = status["phase"].as_str().unwrap_or("unknown");
        let reason = status["reason"].as_str().unwrap_or("unknown");
        if previous.0 != phase || previous.1 != reason {
            // Only production-owned bounded classifications, never prompts, keys,
            // private contexts, signed requests or provider response text.
            eprintln!(
                "NATIVE_REALPLAY_STATUS phase={phase} reason={reason} reserved={} committed_turns={}",
                status["reserved_model_calls"], status["committed_turns"]
            );
            previous = (phase.into(), reason.into());
        }
        if matches!(phase, "stopped" | "blocked") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "NATIVE_REALPLAY_DEADLINE process must exit; retain original run and reservations"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}
