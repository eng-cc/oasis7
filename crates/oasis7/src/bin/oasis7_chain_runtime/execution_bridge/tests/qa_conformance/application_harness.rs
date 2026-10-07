//! Parent-side sandbox result and canonical receipt evidence, separate from application execution.
use super::*;
pub(super) fn validate_output(
    fixture: &Fixture,
    output: &std::process::Output,
    wake: bool,
    drift: bool,
) {
    if !output.status.success() {
        application_wake::report_canonical_resume_failure(fixture);
    }
    assert!(
        output.status.success(),
        "sandbox application failed: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let marker = if drift {
        "PRE2_APPLICATION_RESOURCE_DRIFT_WAKE_REJECTED"
    } else {
        application_wake::required_child_marker(wake)
    };
    assert!(
        stdout.contains(marker),
        "child test filter ran no required proof"
    );
    if !drift {
        let feedback_id = stdout
            .lines()
            .find_map(|line| {
                line.split_once("provider_terminal_feedback_id=")
                    .map(|(_, id)| id.trim())
            })
            .expect("provider feedback evidence missing");
        let world = fixture.driver.lock().unwrap().execution_world.clone();
        let feedback = world
            .runtime_feedback_outbox()
            .unwrap()
            .into_iter()
            .find(|record| record.feedback_id == feedback_id)
            .expect("child terminal feedback must exist in canonical outbox");
        let receipt_id = feedback.payload["runtime_receipt_id"]
            .as_str()
            .filter(|id| !id.is_empty())
            .expect("canonical provider receipt identity missing");
        let lineage = world.read_runtime_receipt_lineage(receipt_id).unwrap();
        world.verify_runtime_receipt_lineage(&lineage).unwrap();
        assert_eq!(lineage.feedback_id, feedback_id);
    } else {
        application_wake::report_canonical_resume_failure(fixture);
        let world = fixture.driver.lock().unwrap().execution_world.clone();
        let results = world.capability_revocation_state();
        assert!(results.world_service_results.values().any(|value| {
            value["rejected"]
                .as_str()
                .is_some_and(|reason| reason.contains("cognition_context_mismatch"))
        }));
        assert!(
            world.cognition_in_flight_wakes().unwrap().len() > 0,
            "rejected Resume consumed canonical wake"
        );
    }
    println!("{stdout}");
}
