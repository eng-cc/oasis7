use crate::simulator::WorldEventKind;

use super::ViewerEventKind;

pub fn viewer_event_kind_matches(filter: &ViewerEventKind, kind: &WorldEventKind) -> bool {
    match (filter, kind) {
        (ViewerEventKind::LocationRegistered, WorldEventKind::LocationRegistered { .. }) => true,
        (ViewerEventKind::AgentRegistered, WorldEventKind::AgentRegistered { .. }) => true,
        (ViewerEventKind::AgentMoved, WorldEventKind::AgentMoved { .. }) => true,
        (ViewerEventKind::AgentSpoke, WorldEventKind::AgentSpoke { .. }) => true,
        (ViewerEventKind::TargetInspected, WorldEventKind::TargetInspected { .. }) => true,
        (
            ViewerEventKind::SimpleInteractionPerformed,
            WorldEventKind::SimpleInteractionPerformed { .. },
        ) => true,
        (ViewerEventKind::ResourceTransferred, WorldEventKind::ResourceTransferred { .. }) => true,
        (ViewerEventKind::RadiationHarvested, WorldEventKind::RadiationHarvested { .. }) => true,
        (ViewerEventKind::ActionRejected, WorldEventKind::ActionRejected { .. }) => true,
        (ViewerEventKind::Power, WorldEventKind::Power(_)) => true,
        (ViewerEventKind::PromptUpdated, WorldEventKind::AgentPromptUpdated { .. }) => true,
        (ViewerEventKind::RuntimeEvent, WorldEventKind::RuntimeEvent { .. }) => true,
        _ => false,
    }
}
