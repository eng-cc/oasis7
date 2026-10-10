//! Functional completion barriers share one bounded isolated-child budget.
//! This budget prevents hangs; it is not an end-to-end latency promise.
use super::*;
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const CHILD_BUDGET: Duration = Duration::from_secs(60);
const DEADLINE_ENV: &str = "PRE2_CONFORMANCE_COMPLETION_DEADLINE_MS";

pub(super) fn child_deadline() -> Instant {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let remaining = std::env::var(DEADLINE_ENV)
        .ok()
        .map(|value| value.parse::<u128>().expect("parent completion deadline"))
        .map(|deadline| {
            Duration::from_millis(deadline.saturating_sub(now).min(u64::MAX as u128) as u64)
        })
        .unwrap_or(CHILD_BUDGET);
    Instant::now() + remaining.min(CHILD_BUDGET)
}

pub(super) fn run_child(command: &mut Command) -> Output {
    let deadline = Instant::now() + CHILD_BUDGET;
    let unix_deadline = SystemTime::now().duration_since(UNIX_EPOCH).unwrap() + CHILD_BUDGET;
    command.env(DEADLINE_ENV, unix_deadline.as_millis().to_string());
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().expect("isolated completion child");
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    // Drain concurrently: waiting on full output pipes would deadlock.
    let out = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let err = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child
                .kill()
                .expect("kill owned isolated child after hard deadline");
            let status = child.wait().unwrap();
            eprintln!(
                "conformance_parent_hard_deadline_exceeded owned_child={} budget_ms={}",
                child.id(),
                CHILD_BUDGET.as_millis()
            );
            break status;
        }
        thread::sleep(Duration::from_millis(10));
    };
    Output {
        status,
        stdout: out.join().unwrap(),
        stderr: err.join().unwrap(),
    }
}

pub(super) fn has_failure(summary: &serde_json::Value) -> bool {
    ["agent_service_pump_error", "hosted_service_memory_failure"]
        .iter()
        .any(|key| !summary[*key].is_null())
        || summary["terminal_states"]["agent-a"]["status"]
            .as_str()
            .is_some_and(|status| status != "committed")
}

pub(super) struct CompletionWatch {
    deadline: Instant,
    started: Instant,
    previous: serde_json::Value,
}
impl CompletionWatch {
    pub(super) fn new() -> Self {
        Self {
            deadline: child_deadline(),
            started: Instant::now(),
            previous: serde_json::Value::Null,
        }
    }
    pub(super) fn finished(&mut self, summary: &serde_json::Value, complete: bool) -> bool {
        let progress = serde_json::json!({
            "phase": summary["hosted_service_phase"], "inflight": summary["hosted_service_inflight"],
            "terminal": summary["terminal_states"], "pending_intents": summary["pending_intent_count"],
            "pending_actions": summary["pending_action_count"]
        });
        if progress != self.previous {
            println!(
                "conformance_completion_progress elapsed_ms={} original_phase={progress}",
                self.started.elapsed().as_millis()
            );
            self.previous = progress;
        }
        complete || has_failure(summary) || Instant::now() >= self.deadline
    }
}

pub(super) fn canonical_complete(summary: &serde_json::Value) -> bool {
    let terminal = &summary["terminal_states"]["agent-a"];
    terminal["status"] == "committed"
        && terminal["request_digest"]
            .as_str()
            .is_some_and(|digest| !digest.is_empty())
        && summary["pending_intent_count"] == 0
        && summary["pending_action_count"] == 0
}

pub(super) fn save_completion(summary: &serde_json::Value) {
    let root = std::path::PathBuf::from(std::env::var("PRE2_METADATA_DIR").unwrap());
    fs::write(
        root.join("canonical-completion.json"),
        serde_json::to_vec(summary).unwrap(),
    )
    .unwrap();
}

pub(super) fn verify_canonical_completion(fixture: &Fixture, root: &std::path::Path) {
    let summary: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("canonical-completion.json")).unwrap()).unwrap();
    assert!(
        canonical_complete(&summary),
        "child canonical completion barrier missing"
    );
    let digest = summary["terminal_states"]["agent-a"]["request_digest"]
        .as_str()
        .unwrap();
    let world = fixture.driver.lock().unwrap().execution_world.clone();
    let results = world.capability_revocation_state().world_service_results.values()
        .filter_map(|value| serde_json::from_value::<wire::CanonicalIntentResultV1>(value.clone()).ok())
        .filter(|result| matches!(&result.request.signed_payload, WorldServicePayloadV1::Cognition(signed) if signed.request.request.request_digest == digest)).collect::<Vec<_>>();
    assert_eq!(
        results.len(),
        1,
        "one canonical Cognition result for original request"
    );
    let result = &results[0];
    assert!(result.rejected.is_none());
    let response = fixture
        .client
        .lookup(
            LookupIntentRequest {
                contract_version: 1,
                key: result.request.correlation.key.clone(),
            },
            result.request.signed_payload.clone(),
        )
        .unwrap();
    let IntentOutcome::Committed { receipt, .. } = response.outcome else {
        panic!("original canonical Lookup receipt missing");
    };
    assert_eq!(receipt["commit_record"]["request_digest"], digest);
    let mut config = fixture.client.config().clone();
    config.scope_id = "agent:agent-a".into();
    let client = RemoteWorldServiceClient::new(config).unwrap();
    let mut request = fixture.view(None);
    request.scope_id = "agent:agent-a".into();
    let view = client.read_view(request).unwrap();
    assert_eq!(
        view.projection()
            .cognition_leases
            .iter()
            .filter(|lease| lease.request_digest == digest
                && lease.status == oasis7::runtime::CognitionLeaseStatusV1::Settled)
            .count(),
        1,
        "one settled original lease"
    );
    println!(
        "conformance_original_completion request_digest={digest} canonical_result_count=1 settled_lease_count=1 original_correlation={}",
        serde_json::to_string(&result.request.correlation).unwrap()
    );
}
