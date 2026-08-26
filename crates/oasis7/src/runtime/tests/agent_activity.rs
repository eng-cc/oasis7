use super::super::*;
use super::pos;
use crate::models::AgentState;

#[test]
fn agent_cell_activity_defaults_to_missing_for_legacy_json() {
    let cell = AgentCell::new(AgentState::new("agent-legacy", pos(0, 0)), 7);
    assert!(cell.activity.is_none());

    let mut value = serde_json::to_value(&cell).expect("encode agent cell");
    value
        .as_object_mut()
        .expect("agent cell object")
        .remove("activity");
    let restored: AgentCell = serde_json::from_value(value).expect("decode legacy agent cell");

    assert!(restored.activity.is_none());
}

#[test]
fn agent_registered_initializes_idle_activity_at_event_time() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-1".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");

    let event = world.journal().events.last().expect("registration event");
    let cell = world
        .state()
        .agents
        .get("agent-1")
        .expect("registered cell");
    let activity: &AgentActivityV1 = cell.activity.as_ref().expect("registered activity");
    assert_eq!(activity.status, AgentActivityStatus::Idle);
    assert_eq!(activity.updated_at, event.time);
}

#[test]
fn snapshot_roundtrip_preserves_agent_activity() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-1".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");

    let snapshot = world.snapshot();
    let restored = Snapshot::from_json(&snapshot.to_json().expect("encode snapshot"))
        .expect("decode snapshot");

    assert_eq!(
        restored.state.agents["agent-1"].activity,
        world.state().agents["agent-1"].activity
    );
}

#[test]
fn missing_activity_is_not_synthesized_as_idle() {
    let cell = AgentCell::new(AgentState::new("agent-missing", pos(0, 0)), 11);
    assert_eq!(cell.activity, None);

    let encoded = serde_json::to_string(&cell).expect("encode missing activity");
    let restored: AgentCell = serde_json::from_str(&encoded).expect("decode missing activity");
    assert_eq!(restored.activity, None);
}
