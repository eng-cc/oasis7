use oasis7_client_api::provider::{
    PROVIDER_PHASE1_ACTION_SET_ALIAS, PROVIDER_WORLD_RESOURCE_DELTA_SCHEMA_V1,
    PROVIDER_WORLD_RESOURCE_MANIFEST_SCHEMA_V1,
};
use oasis7_client_api::{
    ProviderCompatibilityStatus, ProviderHealth, ProviderInfo, evaluate_provider_compatibility,
    provider_phase1_required_capabilities,
};

fn compatible_info() -> ProviderInfo {
    ProviderInfo {
        provider_id: "provider-local".into(),
        name: Some("Local Provider".into()),
        version: Some("0.1.0".into()),
        protocol_version: Some("v1".into()),
        capabilities: provider_phase1_required_capabilities()
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        supported_action_sets: vec![PROVIDER_PHASE1_ACTION_SET_ALIAS.into()],
        chain_resource_manifest_schema_version: Some(
            PROVIDER_WORLD_RESOURCE_MANIFEST_SCHEMA_V1.into(),
        ),
        chain_resource_delta_schema_version: Some(PROVIDER_WORLD_RESOURCE_DELTA_SCHEMA_V1.into()),
    }
}

#[test]
fn compatible_provider_fixture_is_ready() {
    let report = evaluate_provider_compatibility(
        &compatible_info(),
        Some(&ProviderHealth {
            ok: true,
            status: Some("ready".into()),
            uptime_ms: Some(42),
            last_error: None,
            queue_depth: Some(0),
        }),
    );
    assert_eq!(report.status, ProviderCompatibilityStatus::Ready);
    assert_eq!(report.fallback_reason, None);
    assert!(report.missing_capabilities.is_empty());
    assert!(report.missing_supported_actions.is_empty());
    assert!(report.resource_schema_errors.is_empty());
}

#[test]
fn provider_missing_capability_is_incompatible() {
    let mut info = compatible_info();
    info.capabilities = vec!["decision".into()];
    let report = evaluate_provider_compatibility(&info, None);
    assert_eq!(report.status, ProviderCompatibilityStatus::Incompatible);
    assert_eq!(report.missing_capabilities, vec!["feedback"]);
    assert_eq!(
        report.fallback_reason.as_deref(),
        Some("missing_provider_capabilities:feedback")
    );
}

#[test]
fn provider_missing_action_is_incompatible() {
    let mut info = compatible_info();
    info.supported_action_sets = vec!["wait".into(), "move_agent".into()];
    let report = evaluate_provider_compatibility(&info, None);
    assert_eq!(report.status, ProviderCompatibilityStatus::Incompatible);
    assert_eq!(
        report.missing_supported_actions,
        vec![
            "wait_ticks",
            "speak_to_nearby",
            "inspect_target",
            "simple_interact"
        ]
    );
}

#[test]
fn provider_schema_drift_is_incompatible_before_capability_checks() {
    let mut info = compatible_info();
    info.chain_resource_manifest_schema_version = Some("oasis7.world_resource_manifest.v0".into());
    info.chain_resource_delta_schema_version = None;
    let report = evaluate_provider_compatibility(&info, None);
    assert_eq!(report.status, ProviderCompatibilityStatus::Incompatible);
    assert_eq!(
        report.resource_schema_errors,
        vec![
            "world_resource_manifest_schema=oasis7.world_resource_manifest.v0",
            "missing_world_resource_delta_schema"
        ]
    );
}

#[test]
fn unhealthy_provider_is_degraded_with_reason() {
    let report = evaluate_provider_compatibility(
        &compatible_info(),
        Some(&ProviderHealth {
            ok: false,
            status: None,
            uptime_ms: Some(42),
            last_error: None,
            queue_depth: Some(3),
        }),
    );
    assert_eq!(report.status, ProviderCompatibilityStatus::Degraded);
    assert_eq!(
        report.fallback_reason.as_deref(),
        Some("provider_health_unhealthy:not_ok")
    );
}

#[test]
fn provider_fixture_round_trips_wire_json() {
    let info = compatible_info();
    let encoded = serde_json::to_vec(&info).expect("provider info should encode");
    let decoded: ProviderInfo = serde_json::from_slice(&encoded).expect("provider info decodes");
    assert_eq!(decoded, info);
}
